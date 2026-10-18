use super::*;

#[derive(Default)]
struct Trace<'a> {
    requests: Vec<HostReadRequest>,
    refuse: Option<usize>,
    callback: Option<&'a mut dyn FnMut(HostReadRequest)>,
}
impl HostReadBudget for Trace<'_> {
    type Error = usize;
    const OBSERVE_LEDGER: bool = true;
    fn charge(&mut self, request: HostReadRequest) -> Result<(), Self::Error> {
        let index = self.requests.len();
        self.requests.push(request);
        if self.refuse == Some(index) {
            return Err(index);
        }
        if let Some(callback) = &mut self.callback {
            callback(request);
        }
        Ok(())
    }
}

fn candidates(host: &HostAdmissionSession) {
    let mut paths = (0..137)
        .map(|i| PortablePath::new(format!("missing-{i:04}.bin")).unwrap())
        .collect::<Vec<_>>();
    paths.push(
        PortablePath::new(format!(
            "alias/{}/leaf.bin",
            "x".repeat(50) + &"/x".repeat(100)
        ))
        .unwrap(),
    );
    paths.push(PortablePath::new("alias/missing.bin").unwrap());
    host.roots().register_candidates(&paths).unwrap();
    drop(
        host.roots()
            .open(&PortablePath::new("a.bin").unwrap())
            .unwrap(),
    );
}

fn fixture() -> (TempTree, HostAdmissionSession) {
    let tree = TempTree::new("validation-budget");
    fs::write(tree.path().join("a.bin"), b"input").unwrap();
    fs::create_dir(tree.path().join("destination")).unwrap();
    std::os::unix::fs::symlink("destination", tree.path().join("alias")).unwrap();
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    candidates(&host);
    (tree, host)
}

#[test]
fn observed_ledger_refusals_preserve_every_prefix_generation_and_retry() {
    let (_tree, host) = fixture();
    let token = host.ledger.token().unwrap();
    let generation = host.ledger.snapshot().unwrap().generation;
    let mut baseline = Trace::default();
    token.revalidate_with_budget(&mut baseline).unwrap();
    for stage in [
        HostReadStage::LedgerView,
        HostReadStage::LedgerCandidatePath,
        HostReadStage::LedgerCandidateWalk,
        HostReadStage::LedgerAliasPath,
        HostReadStage::LedgerAliasProbe,
        HostReadStage::LedgerAliasCanonicalize,
        HostReadStage::LedgerOpenedPath,
        HostReadStage::LedgerOpenedWalk,
        HostReadStage::LedgerFinish,
    ] {
        assert!(
            baseline.requests.iter().any(|r| r.stage == stage),
            "{stage:?}"
        );
    }
    assert!(baseline
        .requests
        .iter()
        .any(|r| r.stage == HostReadStage::LedgerCandidatePath && r.buffer_bytes > 0));
    assert!(baseline
        .requests
        .iter()
        .any(|r| r.stage == HostReadStage::LedgerAliasPath && r.buffer_bytes > 0));
    let opened = baseline
        .requests
        .iter()
        .position(|r| r.stage == HostReadStage::LedgerOpenedPath)
        .unwrap();
    assert!(!baseline.requests[opened..]
        .iter()
        .any(|r| r.stage == HostReadStage::LedgerCandidatePath));
    for cut in 0..baseline.requests.len() {
        let mut refused = Trace {
            refuse: Some(cut),
            ..Trace::default()
        };
        let HostReadError::Budget(failure) =
            token.revalidate_with_budget(&mut refused).unwrap_err()
        else {
            panic!("wrong failure at {cut}");
        };
        assert_eq!(failure.error, cut);
        assert_eq!(failure.request, baseline.requests[cut]);
        assert_eq!(refused.requests, baseline.requests[..=cut]);
        assert_eq!(host.ledger.snapshot().unwrap().generation, generation);
        let mut retry = Trace::default();
        token.revalidate_with_budget(&mut retry).unwrap();
        assert_eq!(retry.requests, baseline.requests);
    }
}

#[test]
fn stale_ledger_rejects_before_any_validation_callback() {
    let (_tree, host) = fixture();
    let stale = host.ledger.token().unwrap();
    host.roots()
        .register_candidates([&PortablePath::new("later.bin").unwrap()])
        .unwrap();
    let mut trace = Trace {
        refuse: Some(0),
        ..Trace::default()
    };
    assert!(matches!(
        stale.revalidate_with_budget(&mut trace),
        Err(HostReadError::Native(
            HostAdmissionError::ReadIdentityMismatch
        ))
    ));
    assert!(trace.requests.is_empty());
}

#[test]
fn ledger_callbacks_can_read_or_mutate_without_a_mutex_deadlock() {
    for stage in [
        HostReadStage::LedgerView,
        HostReadStage::LedgerCandidateWalk,
        HostReadStage::LedgerAliasProbe,
        HostReadStage::LedgerAliasCanonicalize,
        HostReadStage::LedgerOpenedWalk,
        HostReadStage::LedgerFinish,
    ] {
        let (_tree, host) = fixture();
        let token = host.ledger.token().unwrap();
        let mut changed = false;
        let mut callback = |request: HostReadRequest| {
            assert_eq!(
                host.ledger.token().unwrap().stored_opened_identity_count(),
                1
            );
            if request.stage == stage && !changed {
                host.roots()
                    .register_candidates([&PortablePath::new("later.bin").unwrap()])
                    .unwrap();
                changed = true;
            }
        };
        let mut trace = Trace {
            callback: Some(&mut callback),
            ..Trace::default()
        };
        assert!(
            matches!(
                token.revalidate_with_budget(&mut trace),
                Err(HostReadError::Native(
                    HostAdmissionError::ReadIdentityMismatch
                ))
            ),
            "{stage:?}"
        );
        assert!(changed, "{stage:?}");
        host.ledger
            .token()
            .unwrap()
            .revalidate_with_budget(&mut Trace::default())
            .unwrap();
    }
}

#[test]
fn ledger_observation_keeps_candidate_errors_before_opened_errors() {
    let (tree, host) = fixture();
    let path = PortablePath::new("z-missing/leaf.bin").unwrap();
    host.roots().register_candidates([&path]).unwrap();
    let token = host.ledger.token().unwrap();
    fs::remove_file(tree.path().join("a.bin")).unwrap();
    std::os::unix::fs::symlink("destination", tree.path().join("a.bin")).unwrap();
    fs::create_dir(tree.path().join("z-missing")).unwrap();
    assert_eq!(
        token.revalidate(),
        Err(HostAdmissionError::ReadIdentityMismatch)
    );
    let mut trace = Trace::default();
    assert!(matches!(
        token.revalidate_with_budget(&mut trace),
        Err(HostReadError::Native(
            HostAdmissionError::ReadIdentityMismatch
        ))
    ));
    assert!(!trace
        .requests
        .iter()
        .any(|r| r.stage == HostReadStage::LedgerOpenedPath));
    fs::remove_dir(tree.path().join("z-missing")).unwrap();
    assert_eq!(token.revalidate(), Err(HostAdmissionError::UnsafeCandidate));
    assert!(matches!(
        token.revalidate_with_budget(&mut Trace::default()),
        Err(HostReadError::Native(HostAdmissionError::UnsafeCandidate))
    ));
}

#[test]
fn ordinary_observers_keep_their_existing_callback_sequence() {
    struct Existing;
    impl HostReadBudget for Existing {
        type Error = std::convert::Infallible;
        fn charge(&mut self, _: HostReadRequest) -> Result<(), Self::Error> {
            panic!("ledger observation was not opted into");
        }
    }
    struct Unobserved;
    impl HostReadBudget for Unobserved {
        type Error = std::convert::Infallible;
        const OBSERVED: bool = false;
        const OBSERVE_LEDGER: bool = true;
        fn charge(&mut self, _: HostReadRequest) -> Result<(), Self::Error> {
            panic!("unobserved ledger called a hook");
        }
    }
    let (_tree, host) = fixture();
    let token = host.ledger.token().unwrap();
    token.revalidate_with_budget(&mut Existing).unwrap();
    token.revalidate_with_budget(&mut Unobserved).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn observed_ledger_keeps_unlinked_descriptor_alias_canonicalization_errors() {
    use std::os::fd::AsRawFd;
    let tree = TempTree::new("validation-budget-magic-link");
    let directory = tree.path().join("destination");
    fs::create_dir(&directory).unwrap();
    let handle = File::open(&directory).unwrap();
    let descriptor = format!("/proc/self/fd/{}", handle.as_raw_fd());
    if !Path::new(&descriptor).exists() {
        return;
    }
    std::os::unix::fs::symlink(descriptor, tree.path().join("alias")).unwrap();
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    host.roots()
        .register_candidates([&PortablePath::new("alias/leaf.bin").unwrap()])
        .unwrap();
    let token = host.ledger.token().unwrap();
    fs::remove_dir(directory).unwrap();
    assert_eq!(token.revalidate(), Err(HostAdmissionError::Read));
    let mut trace = Trace::default();
    assert!(matches!(
        token.revalidate_with_budget(&mut trace),
        Err(HostReadError::Native(HostAdmissionError::Read))
    ));
    assert_eq!(
        trace.requests.last().unwrap().stage,
        HostReadStage::LedgerAliasCanonicalize
    );
}
