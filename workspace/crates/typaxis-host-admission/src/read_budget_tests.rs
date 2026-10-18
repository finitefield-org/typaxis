use super::*;

#[derive(Default)]
struct Trace {
    accepted: Vec<HostReadRequest>,
    stop: Option<usize>,
}
impl HostReadBudget for Trace {
    type Error = usize;
    fn charge(&mut self, request: HostReadRequest) -> Result<(), usize> {
        let index = self.accepted.len();
        if self.stop == Some(index) {
            return Err(index);
        }
        self.accepted.push(request);
        Ok(())
    }
}

#[test]
fn observed_reads_match_stable_bytes_and_stop_before_every_request() {
    let tree = TempTree::new("observed-read");
    let config = effective_config();
    for length in [0usize, 1, 55, 56, 63, 64, 65, 8191, 8192, 8193, 20000] {
        let bytes: Vec<_> = (0..length).map(|i| (i % 251) as u8).collect();
        fs::write(tree.path().join("payload.bin"), &bytes).unwrap();
        let host = HostAdmissionSession::new(&context(tree.path(), &[]), &config).unwrap();
        let roots = host.roots();
        let uri = PortablePath::new("payload.bin").unwrap();
        let open = || {
            roots
                .issue_bounded_read_permit(roots.open(&uri).unwrap())
                .unwrap()
        };
        let permit = open();
        let identity = permit.opened.read_identity().clone();
        let mut full = Trace::default();
        let stable = roots
            .accept_receipt(
                &identity,
                roots.read_bounded_with_budget(permit, &mut full).unwrap(),
            )
            .unwrap();
        assert_eq!(stable.bytes(), bytes);
        assert_eq!(stable.sha256(), sha256(&bytes));
        let mut expected = vec![HostReadRequest {
            stage: HostReadStage::Buffer,
            file_offset: 0,
            buffer_bytes: length as u64,
            work: 8256,
        }];
        for offset in (0..length).step_by(8192) {
            expected.push(HostReadRequest {
                stage: HostReadStage::Chunk,
                file_offset: offset as u64,
                buffer_bytes: 0,
                work: (length - offset).min(8192) as u64 + 64,
            });
        }
        expected.push(HostReadRequest {
            stage: HostReadStage::DigestFinish,
            file_offset: length as u64,
            buffer_bytes: 0,
            work: 192,
        });
        assert_eq!(full.accepted, expected);
        for stop in 0..full.accepted.len() {
            let mut short = Trace {
                stop: Some(stop),
                ..Trace::default()
            };
            let failure = roots
                .read_bounded_with_budget(open(), &mut short)
                .unwrap_err();
            let HostReadError::Budget(failure) = failure else {
                panic!("wrong error")
            };
            assert_eq!(failure.error, stop);
            assert_eq!(failure.request, full.accepted[stop]);
            assert_eq!(short.accepted, full.accepted[..stop]);
            // A new authentic open can retry with the retained paid prefix.
            short.stop = None;
            roots.read_bounded_with_budget(open(), &mut short).unwrap();
            assert_eq!(&short.accepted[stop..], full.accepted);
        }
    }
}

#[test]
fn observed_read_preserves_native_mutation_errors_and_paid_prefix() {
    let tree = TempTree::new("observed-mutation");
    let path = tree.path().join("payload.bin");
    let bytes = vec![0x41; 20000];
    for mutation in 0..3 {
        fs::write(&path, &bytes).unwrap();
        let host =
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
        let roots = host.roots();
        let permit = roots
            .issue_bounded_read_permit(
                roots
                    .open(&PortablePath::new("payload.bin").unwrap())
                    .unwrap(),
            )
            .unwrap();
        let mut trace = Trace::default();
        let error = read_bounded_counted(
            permit,
            || match mutation {
                0 => File::options()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_len(1)
                    .unwrap(),
                1 => File::options()
                    .append(true)
                    .open(&path)
                    .unwrap()
                    .write_all(b"growth")
                    .unwrap(),
                2 => {
                    let other = tree.path().join("replacement.bin");
                    fs::write(&other, &bytes).unwrap();
                    fs::rename(other, &path).unwrap();
                }
                _ => unreachable!(),
            },
            &mut trace,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            HostReadError::Native(HostAdmissionError::LengthMismatch)
        ));
        assert_eq!(trace.accepted[0].stage, HostReadStage::Buffer);
        assert!(trace
            .accepted
            .iter()
            .any(|r| r.stage == HostReadStage::Chunk));
        assert!(!trace
            .accepted
            .iter()
            .any(|r| r.stage == HostReadStage::DigestFinish));
    }
}

#[test]
fn observed_read_rejects_a_foreign_root_before_charging() {
    let a = TempTree::new("observed-root-a");
    let b = TempTree::new("observed-root-b");
    fs::write(a.path().join("payload.bin"), b"payload").unwrap();
    let config = effective_config();
    let first = HostAdmissionSession::new(&context(a.path(), &[]), &config).unwrap();
    let second = HostAdmissionSession::new(&context(b.path(), &[]), &config).unwrap();
    let permit = first
        .roots()
        .issue_bounded_read_permit(
            first
                .roots()
                .open(&PortablePath::new("payload.bin").unwrap())
                .unwrap(),
        )
        .unwrap();
    let mut trace = Trace::default();
    assert!(matches!(
        second.roots().read_bounded_with_budget(permit, &mut trace),
        Err(HostReadError::Native(HostAdmissionError::SessionMismatch))
    ));
    assert!(trace.accepted.is_empty());
}

#[test]
fn streaming_sha_keeps_partial_blocks_across_short_updates() {
    for length in [0usize, 1, 55, 56, 63, 64, 65, 127, 128, 129, 257, 8193] {
        let bytes: Vec<_> = (0..length).map(|i| (i % 251) as u8).collect();
        for chunk in [1, 2, 3, 7, 31, 63, 64, 65, 257, 8192] {
            let mut hasher = StreamingSha256::new();
            for part in bytes.chunks(chunk) {
                hasher.update(part);
            }
            assert_eq!(
                hasher.finalize(),
                sha256(&bytes),
                "length={length}, chunk={chunk}"
            );
        }
    }
}
