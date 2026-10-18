use super::*;
#[derive(Default)]
struct Trace {
    requests: Vec<HostReadRequest>,
    stop: Option<usize>,
    ledger: Option<HostReadIdentityLedger>,
}
impl HostReadBudget for Trace {
    type Error = usize;
    const OBSERVE_OPEN: bool = true;
    fn charge(&mut self, r: HostReadRequest) -> Result<(), usize> {
        if let Some(ledger) = &self.ledger {
            ledger.token().unwrap();
        } // Re-entry must not deadlock.
        let at = self.requests.len();
        if self.stop == Some(at) {
            return Err(at);
        }
        self.requests.push(r);
        Ok(())
    }
}
fn uri_of_length(mut n: usize) -> String {
    let mut s = String::new();
    while n > 200 {
        s.push_str(&"d".repeat(180));
        s.push('/');
        n -= 181;
    }
    s.push_str(&"f".repeat(n));
    s
}
#[test]
fn observed_registered_opens_reserve_before_mutation_and_allow_reentry() {
    let tree = TempTree::new("observed-open");
    let mut cuts = 0;
    for n in [1, 63, 64, 255, 256, 257, 1842] {
        let uri = PortablePath::new(uri_of_length(n)).unwrap();
        let path = tree.path().join(uri.as_str());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"stable").unwrap();
        let make =
            || HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
        let host = make();
        let registered = host.roots().register_candidates([&uri]).unwrap();
        let token = host.read_ledger().token().unwrap();
        let mut full = Trace {
            ledger: Some(host.ledger.clone()),
            ..Trace::default()
        };
        let opened = host
            .roots()
            .open_registered_with_budget(&registered[0], &mut full)
            .unwrap();
        assert_eq!(opened.observed_exact_length(), 6);
        assert_eq!(
            full.requests.iter().map(|r| r.stage).collect::<Vec<_>>(),
            [
                HostReadStage::OpenPath,
                HostReadStage::OpenRoot,
                HostReadStage::OpenLock,
                HostReadStage::OpenLedger,
                HostReadStage::OpenOwner
            ]
        );
        assert_eq!(
            full.requests[0].buffer_bytes,
            if n < 256 { 0 } else { n as u64 + 1 }
        );
        assert_eq!(full.requests[4].buffer_bytes, 16);
        assert!(matches!(
            token.revalidate(),
            Err(HostAdmissionError::ReadIdentityMismatch)
        ));
        let current = host.read_ledger().token().unwrap();
        let another = host
            .roots()
            .open_registered_with_budget(&registered[0], &mut Trace::default())
            .unwrap();
        current.revalidate().unwrap();
        assert_ne!(opened.read_identity(), another.read_identity());
        assert_eq!(current.stored_opened_identity_count(), 1);
        for stop in 0..full.requests.len() {
            let host = make();
            let registered = host.roots().register_candidates([&uri]).unwrap();
            let token = host.read_ledger().token().unwrap();
            let mut short = Trace {
                stop: Some(stop),
                ledger: Some(host.ledger.clone()),
                ..Trace::default()
            };
            assert!(
                matches!(host.roots().open_registered_with_budget(&registered[0],&mut short),Err(HostReadError::Budget(f)) if f.error==stop && f.request==full.requests[stop])
            );
            assert_eq!(short.requests, full.requests[..stop]);
            token.revalidate().unwrap();
            assert_eq!(
                host.read_ledger()
                    .token()
                    .unwrap()
                    .stored_opened_identity_count(),
                0
            );
            short.stop = None;
            let opened = host
                .roots()
                .open_registered_with_budget(&registered[0], &mut short)
                .unwrap();
            assert_eq!(&short.requests[stop..], full.requests);
            let id = opened.read_identity().clone();
            let permit = host.roots().issue_bounded_read_permit(opened).unwrap();
            let bytes = host
                .roots()
                .accept_receipt(&id, host.roots().read_bounded(permit).unwrap())
                .unwrap();
            assert_eq!(bytes.bytes(), b"stable");
            cuts += 1;
        }
    }
    assert_eq!(cuts, 35);
    println!("host observed open URI cases=7 callback refusals=35 reentrant_ledger=true");
}
#[test]
fn registered_slots_preserve_alias_counts_generations_and_growth() {
    let tree = TempTree::new("opened-slots");
    fs::write(tree.path().join("a"), b"same").unwrap();
    fs::hard_link(tree.path().join("a"), tree.path().join("b")).unwrap();
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    let a = PortablePath::new("a").unwrap();
    let b = PortablePath::new("b").unwrap();
    let first = host.roots().register_candidates([&a]).unwrap();
    let opened = host.roots().open_registered(&first[0]).unwrap();
    let old = host.read_ledger().token().unwrap();
    let other = host.roots().register_candidates([&b]).unwrap();
    assert!(matches!(
        old.revalidate(),
        Err(HostAdmissionError::ReadIdentityMismatch)
    ));
    host.roots()
        .open_registered_with_budget(&other[0], &mut Trace::default())
        .unwrap();
    let snapshot = host.ledger.snapshot().unwrap();
    assert_eq!(snapshot.candidates.len(), 2);
    assert_eq!(snapshot.opened_by_candidate.len(), 2);
    assert_eq!(snapshot.opened_identities.len(), 1);
    assert_eq!(
        host.read_ledger()
            .token()
            .unwrap()
            .stored_opened_identity_count(),
        1
    );
    let current = host.read_ledger().token().unwrap();
    host.roots()
        .open_registered_with_budget(&first[0], &mut Trace::default())
        .unwrap();
    current.revalidate().unwrap();
    assert_eq!(opened.observed_exact_length(), 4);
}
#[test]
fn observed_open_preserves_native_failures_and_rejects_foreign_tokens_first() {
    use std::os::unix::fs::symlink;
    let a = TempTree::new("open-failures");
    let b = TempTree::new("open-foreign");
    fs::write(a.path().join("file"), b"ok").unwrap();
    symlink("file", a.path().join("link")).unwrap();
    fs::create_dir(a.path().join("dir")).unwrap();
    let host = HostAdmissionSession::new(&context(a.path(), &[]), &effective_config()).unwrap();
    let foreign = HostAdmissionSession::new(&context(b.path(), &[]), &effective_config()).unwrap();
    for name in ["missing", "link", "dir"] {
        let uri = PortablePath::new(name).unwrap();
        let registered = host.roots().register_candidates([&uri]).unwrap();
        let native = host.roots().open_registered(&registered[0]).err().unwrap();
        assert!(
            matches!(host.roots().open_registered_with_budget(&registered[0],&mut Trace::default()),Err(HostReadError::Native(e)) if e==native)
        );
        let mut trace = Trace::default();
        assert!(matches!(
            foreign
                .roots()
                .open_registered_with_budget(&registered[0], &mut trace),
            Err(HostReadError::Native(HostAdmissionError::SessionMismatch))
        ));
        assert!(trace.requests.is_empty());
    }
}

#[test]
fn opened_slots_remain_stable_for_8192_reverse_opens_and_second_root() {
    let a = TempTree::new("slot-scale-a");
    let b = TempTree::new("slot-scale-b");
    let paths = (0..8192)
        .map(|i| PortablePath::new(format!("file-{i:04}")).unwrap())
        .collect::<Vec<_>>();
    for uri in &paths {
        fs::write(b.path().join(uri.as_str()), b"data").unwrap();
    }
    let host =
        HostAdmissionSession::new(&context(a.path(), &[b.path()]), &effective_config()).unwrap();
    assert_eq!(host.host_roots.len(), 2);
    let registered = host.roots().register_candidates(paths.iter()).unwrap();
    let mut trace = Trace::default();
    for i in (0..8192).rev() {
        let opened = host
            .roots()
            .open_registered_with_budget(&registered[i], &mut trace)
            .unwrap();
        assert_eq!(opened.observed_exact_length(), 4);
    }
    let snapshot = host.ledger.snapshot().unwrap();
    assert_eq!(snapshot.opened_by_candidate.len(), 8192);
    assert_eq!(snapshot.opened_identities.len(), 8192);
    assert!(snapshot
        .opened_by_candidate
        .keys()
        .all(|v| v.root == host.host_roots[1].identity()));
    let token = host.read_ledger().token().unwrap();
    host.roots()
        .open_registered_with_budget(&registered[0], &mut Trace::default())
        .unwrap();
    token.revalidate().unwrap();
    assert_eq!(trace.requests.len(), 8192 * 6);
    println!("host opened slots scale=8192 roots=2 reverse=true allocations_at_open_ledger=0");
}
