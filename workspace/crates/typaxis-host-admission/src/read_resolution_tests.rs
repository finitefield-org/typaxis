use super::*;
#[derive(Default)]
struct Trace {
    accepted: Vec<HostReadRequest>,
    stop: Option<usize>,
}
impl HostReadBudget for Trace {
    type Error = usize;
    const OBSERVE_RESOLUTION: bool = true;
    fn charge(&mut self, r: HostReadRequest) -> Result<(), usize> {
        let at = self.accepted.len();
        if self.stop == Some(at) {
            return Err(at);
        }
        self.accepted.push(r);
        Ok(())
    }
}
fn uri_of_length(mut length: usize) -> String {
    let mut text = String::new();
    while length > 200 {
        text.push_str(&"d".repeat(180));
        text.push('/');
        length -= 181;
    }
    text.push_str(&"f".repeat(length));
    text
}
#[test]
fn resolution_permits_cover_stack_heap_and_every_refusal() {
    let tree = TempTree::new("observed-resolution");
    let config = effective_config();
    let mut refusals = 0;
    for length in [1, 63, 64, 255, 256, 257, 1842] {
        let uri = PortablePath::new(uri_of_length(length)).unwrap();
        let path = tree.path().join(uri.as_str());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        for size in [0, 1, 8193] {
            let bytes = vec![0x51; size];
            fs::write(&path, &bytes).unwrap();
            let host = HostAdmissionSession::new(&context(tree.path(), &[]), &config).unwrap();
            let roots = host.roots();
            let open = || {
                roots
                    .issue_bounded_read_permit(roots.open(&uri).unwrap())
                    .unwrap()
            };
            let mut full = Trace::default();
            let permit = open();
            let identity = permit.opened.read_identity().clone();
            let stable = roots
                .accept_receipt(
                    &identity,
                    roots.read_bounded_with_budget(permit, &mut full).unwrap(),
                )
                .unwrap();
            assert_eq!(stable.bytes(), bytes);
            assert_eq!(stable.sha256(), sha256(&bytes));
            let native_permit = open();
            let native_identity = native_permit.opened.read_identity().clone();
            let native = roots
                .accept_receipt(&native_identity, roots.read_bounded(native_permit).unwrap())
                .unwrap();
            assert_eq!(native.bytes(), stable.bytes());
            let at = 1 + size.div_ceil(8192);
            assert_eq!(
                full.accepted[at],
                HostReadRequest {
                    stage: HostReadStage::RevalidatePath,
                    file_offset: 0,
                    buffer_bytes: if length < 256 { 0 } else { length as u64 + 1 },
                    work: 16 * length as u64 + 64 + if length < 256 { 256 } else { 0 }
                }
            );
            assert_eq!(
                full.accepted[at + 1],
                HostReadRequest {
                    stage: HostReadStage::RevalidateRoot,
                    file_offset: 0,
                    buffer_bytes: 0,
                    work: 16 * length as u64 + 128 * uri.as_str().split('/').count() as u64 + 64
                }
            );
            assert_eq!(full.accepted[at + 2].stage, HostReadStage::DigestFinish);
            for stop in 0..full.accepted.len() {
                let mut short = Trace {
                    stop: Some(stop),
                    ..Trace::default()
                };
                assert!(
                    matches!(roots.read_bounded_with_budget(open(),&mut short),Err(HostReadError::Budget(f)) if f.error==stop && f.request==full.accepted[stop])
                );
                assert_eq!(short.accepted, full.accepted[..stop]);
                short.stop = None;
                roots.read_bounded_with_budget(open(), &mut short).unwrap();
                assert_eq!(&short.accepted[stop..], full.accepted);
                refusals += 1;
            }
        }
    }
    assert_eq!(refusals, 105);
    println!("host revalidation URI cases=7 byte lengths=3 callback refusals={refusals}");
}
#[test]
fn prepared_component_walker_matches_native_errors_and_restores_path() {
    use std::os::unix::fs::symlink;
    let tree = TempTree::new("prepared-component-walker");
    fs::create_dir(tree.path().join("dir")).unwrap();
    fs::write(tree.path().join("dir/file.bin"), b"stable").unwrap();
    symlink("dir", tree.path().join("linked-dir")).unwrap();
    symlink("dir/file.bin", tree.path().join("linked-file")).unwrap();
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    let root = &host.host_roots[0];
    let long = uri_of_length(1842);
    let long_path = tree.path().join(&long);
    fs::create_dir_all(long_path.parent().unwrap()).unwrap();
    fs::write(long_path, b"long").unwrap();
    for text in [
        "dir/file.bin",
        "dir/missing",
        "missing/file",
        "linked-dir/file.bin",
        "linked-file",
        "dir",
        "dir/file.bin/leaf",
        long.as_str(),
    ] {
        let uri = PortablePath::new(text).unwrap();
        let native = root.open_candidate(&uri).map(|v| v.map(|f| f.identity()));
        let mut path = platform::PreparedReadPath::new(&uri).unwrap();
        for _ in 0..2 {
            assert_eq!(
                path.open_component_walker(root)
                    .map(|v| v.map(|f| f.identity())),
                native,
                "{text}"
            );
            assert_eq!(
                path.open(root).map(|v| v.map(|f| f.identity())),
                native,
                "{text}"
            );
        }
    }
}
#[test]
fn observed_revalidation_rechecks_every_root_and_rejects_new_aliases() {
    let a = TempTree::new("resolution-root-a");
    let b = TempTree::new("resolution-root-b");
    let uri = PortablePath::new("payload.bin").unwrap();
    fs::write(a.path().join(uri.as_str()), b"original").unwrap();
    let host =
        HostAdmissionSession::new(&context(a.path(), &[b.path()]), &effective_config()).unwrap();
    assert_eq!(host.host_roots.len(), 2);
    let open = || {
        host.roots()
            .issue_bounded_read_permit(host.roots().open(&uri).unwrap())
            .unwrap()
    };
    let mut trace = Trace::default();
    let error = read_bounded_counted(
        open(),
        || fs::write(b.path().join(uri.as_str()), b"shadow").unwrap(),
        &mut trace,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        HostReadError::Native(HostAdmissionError::LengthMismatch)
    ));
    assert_eq!(
        trace
            .accepted
            .iter()
            .filter(|r| r.stage == HostReadStage::RevalidateRoot)
            .count(),
        2
    );
    assert!(!trace
        .accepted
        .iter()
        .any(|r| r.stage == HostReadStage::DigestFinish));
}
