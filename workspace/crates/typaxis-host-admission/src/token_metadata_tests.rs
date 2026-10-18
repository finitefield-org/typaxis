use super::*;
struct Trace<'a> {
    host: &'a HostAdmissionSession,
    calls: Vec<HostReadRequest>,
    refuse: bool,
    reenter: bool,
}
impl HostReadBudget for Trace<'_> {
    type Error = HostReadStage;
    const OBSERVE_OPEN: bool = true;
    const OBSERVE_OPEN_IDENTITIES: bool = true;
    fn charge(&mut self, r: HostReadRequest) -> Result<(), Self::Error> {
        // A token can be issued here: callbacks must run outside the mutex.
        let _ = self.host.read_ledger().token().unwrap();
        self.calls.push(r);
        if r.stage == HostReadStage::OpenIdentityLookup {
            if self.reenter {
                self.reenter = false;
                let _ = self
                    .host
                    .roots()
                    .register_candidates(std::iter::once(&PortablePath::new("other.bin").unwrap()))
                    .unwrap();
            }
            if self.refuse {
                return Err(r.stage);
            }
        }
        Ok(())
    }
}
#[test]
fn token_metadata_matches_snapshot_through_hard_links_and_repeated_opens() {
    let tree = TempTree::new("token-metadata-aliases");
    fs::write(tree.path().join("a.bin"), b"a").unwrap();
    fs::write(tree.path().join("b.bin"), b"b").unwrap();
    fs::hard_link(tree.path().join("a.bin"), tree.path().join("alias.bin")).unwrap();
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    let names = ["a.bin", "alias.bin", "b.bin"].map(|s| PortablePath::new(s).unwrap());
    let registered = host.roots().register_candidates(&names).unwrap();
    let stale = host.read_ledger().token().unwrap();
    assert_eq!(stale.stored_opened_identity_count(), 0);
    for (step, (index, expected)) in [(0, 1), (1, 1), (2, 2), (0, 2), (1, 2)]
        .into_iter()
        .enumerate()
    {
        let mut trace = Trace {
            host: &host,
            calls: Vec::new(),
            refuse: false,
            reenter: false,
        };
        let _opened = host
            .roots()
            .open_registered_with_budget(&registered[index], &mut trace)
            .unwrap();
        let token = host.read_ledger().token().unwrap();
        let snapshot = host.ledger.snapshot().unwrap();
        assert_eq!(token.stored_opened_identity_count(), expected);
        assert_eq!(expected, snapshot.opened_identities.len());
        assert_eq!(
            token.stored_candidate_identity_count(),
            snapshot.candidate_identities.len()
        );
        assert_eq!(token.generation, snapshot.generation);
        let charged = trace
            .calls
            .iter()
            .filter(|v| v.stage == HostReadStage::OpenIdentityLookup)
            .collect::<Vec<_>>();
        assert_eq!(charged.len(), usize::from(step < 3));
        for r in charged {
            assert_eq!(r.work, host_open_identity_work_bound(3).unwrap());
            assert_eq!(r.buffer_bytes, 0);
        }
    }
    assert_eq!(
        stale.revalidate(),
        Err(HostAdmissionError::ReadIdentityMismatch)
    );
    let current = host.ledger.token().unwrap();
    assert!(current
        .revalidate_write_target(&HostPath::new(tree.path().join("alias.bin")).unwrap())
        .unwrap());
    assert!(!current
        .revalidate_write_target(&HostPath::new(tree.path().join("new.bin")).unwrap())
        .unwrap());
}
#[test]
fn identity_lookup_refusal_preserves_generation_and_retry() {
    let tree = TempTree::new("token-metadata-refusal");
    fs::write(tree.path().join("a.bin"), b"a").unwrap();
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    let names = [PortablePath::new("a.bin").unwrap()];
    let registered = host.roots().register_candidates(&names).unwrap();
    let before = host.ledger.token().unwrap();
    let mut trace = Trace {
        host: &host,
        calls: Vec::new(),
        refuse: true,
        reenter: false,
    };
    let HostReadError::Budget(f) = host
        .roots()
        .open_registered_with_budget(&registered[0], &mut trace)
        .unwrap_err()
    else {
        panic!("wrong cause")
    };
    assert_eq!(f.error, HostReadStage::OpenIdentityLookup);
    assert_eq!(f.request.work, host_open_identity_work_bound(1).unwrap());
    let after = host.ledger.token().unwrap();
    assert_eq!(before.generation, after.generation);
    assert_eq!(after.stored_opened_identity_count(), 0);
    before.revalidate().unwrap();
    trace.refuse = false;
    host.roots()
        .open_registered_with_budget(&registered[0], &mut trace)
        .unwrap();
    assert_eq!(
        host.ledger.token().unwrap().stored_opened_identity_count(),
        1
    );
    assert_eq!(
        before.revalidate(),
        Err(HostAdmissionError::ReadIdentityMismatch)
    );
}
#[test]
fn identity_lookup_reentrant_mutation_fails_before_scanning_unpaid_state() {
    let tree = TempTree::new("token-metadata-reentry");
    fs::write(tree.path().join("a.bin"), b"a").unwrap();
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    let names = [PortablePath::new("a.bin").unwrap()];
    let registered = host.roots().register_candidates(&names).unwrap();
    let mut trace = Trace {
        host: &host,
        calls: Vec::new(),
        refuse: false,
        reenter: true,
    };
    assert!(matches!(
        host.roots()
            .open_registered_with_budget(&registered[0], &mut trace),
        Err(HostReadError::Native(
            HostAdmissionError::ReadIdentityMismatch
        ))
    ));
    assert_eq!(
        host.ledger.token().unwrap().stored_opened_identity_count(),
        0
    );
    host.roots()
        .open_registered_with_budget(&registered[0], &mut trace)
        .unwrap();
    assert_eq!(
        host.ledger.token().unwrap().stored_opened_identity_count(),
        1
    );
}

#[test]
fn every_index_preparation_refusal_preserves_facts_and_retry() {
    struct Stop<'a> {
        inner: Trace<'a>,
        stage: Option<HostReadStage>,
    }
    impl HostReadBudget for Stop<'_> {
        type Error = HostReadStage;
        const OBSERVE_OPEN: bool = true;
        const OBSERVE_OPEN_IDENTITIES: bool = true;
        fn charge(&mut self, r: HostReadRequest) -> Result<(), Self::Error> {
            self.inner.charge(r)?;
            if self.stage == Some(r.stage) {
                Err(r.stage)
            } else {
                Ok(())
            }
        }
    }
    for stage in [
        HostReadStage::OpenIdentityStorage,
        HostReadStage::OpenIdentityMigrate,
        HostReadStage::OpenIdentityLookup,
    ] {
        let tree = TempTree::new("index-every-cut");
        fs::write(tree.path().join("a.bin"), b"a").unwrap();
        let host =
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
        let names = [PortablePath::new("a.bin").unwrap()];
        let registered = host.roots().register_candidates(&names).unwrap();
        let before = host.ledger.token().unwrap();
        let mut trace = Stop {
            inner: Trace {
                host: &host,
                calls: Vec::new(),
                refuse: false,
                reenter: false,
            },
            stage: Some(stage),
        };
        let HostReadError::Budget(failure) = host
            .roots()
            .open_registered_with_budget(&registered[0], &mut trace)
            .unwrap_err()
        else {
            panic!("wrong failure")
        };
        assert_eq!(failure.request.stage, stage);
        assert_eq!(failure.error, stage);
        let after = host.ledger.token().unwrap();
        assert_eq!(before.generation, after.generation);
        assert_eq!(after.stored_opened_identity_count(), 0);
        before.revalidate().unwrap();
        trace.stage = None;
        host.roots()
            .open_registered_with_budget(&registered[0], &mut trace)
            .unwrap();
        assert_eq!(
            host.ledger.token().unwrap().stored_opened_identity_count(),
            1
        );
    }
}
#[test]
fn opened_index_expands_after_later_registration_without_losing_aliases() {
    let tree = TempTree::new("index-grow");
    for name in ["a.bin", "b.bin"] {
        fs::write(tree.path().join(name), name.as_bytes()).unwrap();
    }
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    for (number, name) in ["a.bin", "b.bin"].into_iter().enumerate() {
        let names = [PortablePath::new(name).unwrap()];
        let registered = host.roots().register_candidates(&names).unwrap();
        let mut trace = Trace {
            host: &host,
            calls: Vec::new(),
            refuse: false,
            reenter: false,
        };
        host.roots()
            .open_registered_with_budget(&registered[0], &mut trace)
            .unwrap();
        let token = host.ledger.token().unwrap();
        assert_eq!(token.stored_opened_identity_count(), number + 1);
        let storage = trace
            .calls
            .iter()
            .find(|r| r.stage == HostReadStage::OpenIdentityStorage)
            .unwrap();
        assert_eq!(
            storage.buffer_bytes,
            ((number + 1) * HOST_OPEN_IDENTITY_NODE_BYTES) as u64
        );
        let migration = trace
            .calls
            .iter()
            .find(|r| r.stage == HostReadStage::OpenIdentityMigrate)
            .unwrap();
        if number == 1 {
            assert_eq!(migration.work, HOST_OPEN_IDENTITY_NODE_BYTES as u64 + 64);
        }
    }
    assert_eq!(host.ledger.snapshot().unwrap().opened_identities.len(), 2);
}
