use super::*;
fn many_candidates(host: &HostAdmissionSession, count: usize) {
    let names = (0..count)
        .map(|i| PortablePath::new(format!("missing-{i:04}.bin")).unwrap())
        .collect::<Vec<_>>();
    host.roots().register_candidates(&names).unwrap();
}
#[test]
fn valid_views_are_bounded_and_callbacks_can_read_the_same_ledger() {
    let tree = TempTree::new("validation-view-bound");
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    many_candidates(&host, 257);
    let token = host.ledger.token().unwrap();
    assert_eq!(host.ledger.snapshot().unwrap().roots.len(), 1);
    let mut batches = Vec::new();
    token
        .revalidate_with_observer_for_test(|opened, len| {
            assert!(len <= 64);
            assert_eq!(host.ledger.token().unwrap().candidate_attempt_count(), 257);
            batches.push((opened, len));
        })
        .unwrap();
    let expected = [64, 64, 64, 64, 1];
    assert_eq!(
        batches,
        expected
            .map(|n| (false, n))
            .into_iter()
            .chain(expected.map(|n| (true, n)))
            .collect::<Vec<_>>()
    );
}
#[test]
fn generation_changes_between_candidate_or_opened_batches_fail_closed() {
    for phase in [false, true] {
        let tree = TempTree::new("validation-view-generation");
        let host =
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
        many_candidates(&host, 257);
        let token = host.ledger.token().unwrap();
        let mut seen = 0;
        let result = token.revalidate_with_observer_for_test(|opened, _| {
            if opened == phase {
                seen += 1;
                if seen == 2 {
                    host.roots()
                        .register_candidates(std::iter::once(
                            &PortablePath::new("new-candidate.bin").unwrap(),
                        ))
                        .unwrap();
                }
            }
        });
        assert_eq!(result, Err(HostAdmissionError::ReadIdentityMismatch));
        assert_eq!(seen, 2);
        host.ledger.token().unwrap().revalidate().unwrap();
    }
}
#[test]
fn all_candidate_checks_precede_opened_terminal_errors_across_batches() {
    use std::os::unix::fs::symlink;
    let tree = TempTree::new("validation-view-order");
    fs::write(tree.path().join("a-open.bin"), b"a").unwrap();
    fs::write(tree.path().join("outside.bin"), b"outside").unwrap();
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    many_candidates(&host, 130);
    host.roots()
        .register_candidates(std::iter::once(
            &PortablePath::new("z-missing/leaf.bin").unwrap(),
        ))
        .unwrap();
    let opened = host
        .roots()
        .open(&PortablePath::new("a-open.bin").unwrap())
        .unwrap();
    drop(opened);
    let token = host.ledger.token().unwrap();
    fs::remove_file(tree.path().join("a-open.bin")).unwrap();
    symlink(
        tree.path().join("outside.bin"),
        tree.path().join("a-open.bin"),
    )
    .unwrap();
    fs::create_dir(tree.path().join("z-missing")).unwrap();
    assert_eq!(
        token.revalidate(),
        Err(HostAdmissionError::ReadIdentityMismatch)
    );
    fs::remove_dir(tree.path().join("z-missing")).unwrap();
    assert_eq!(token.revalidate(), Err(HostAdmissionError::UnsafeCandidate));
}
#[test]
fn target_membership_preserves_hard_link_candidate_and_unopened_alias_guards() {
    let tree = TempTree::new("validation-view-targets");
    fs::write(tree.path().join("a.bin"), b"a").unwrap();
    fs::hard_link(tree.path().join("a.bin"), tree.path().join("alias.bin")).unwrap();
    let host = HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
    many_candidates(&host, 130);
    let opened = host
        .roots()
        .open(&PortablePath::new("a.bin").unwrap())
        .unwrap();
    drop(opened);
    let token = host.ledger.token().unwrap();
    for name in ["a.bin", "alias.bin", "missing-0129.bin"] {
        assert!(token
            .revalidate_write_target(&HostPath::new(tree.path().join(name)).unwrap())
            .unwrap());
    }
    assert!(!token
        .revalidate_write_target(&HostPath::new(tree.path().join("new-output.pdf")).unwrap())
        .unwrap());
}
