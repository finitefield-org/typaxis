use super::*;

#[test]
fn book_v2_column_converged_pdf_unplaced_reference_to_unplaced_anchor_keeps_actual_none_target() {
    let root = Root::new();
    let caps = caps();
    let mut authored = data(true);
    let mut unplaced_heading = authored["document"]["blocks"][0].clone();
    unplaced_heading["anchor_id"] = "unplaced-page-target".into();
    let blocks = authored["document"]["footnotes"][1]["blocks"]
        .as_array_mut()
        .unwrap();
    blocks[0]["children"][0]["target"] = "unplaced-page-target".into();
    blocks.insert(0, unplaced_heading);
    let mut next = 0;
    renumber(&mut authored["document"], &mut next);
    for master in authored["page_masters"]["masters"].as_array_mut().unwrap() {
        for role in ["header_content", "footer_content"] {
            renumber(&mut master[role], &mut next);
        }
    }
    let owner = NodeId::new(
        authored["document"]["footnotes"][1]["blocks"][1]["children"][0]["node_id"]
            .as_u64()
            .unwrap()
            .try_into()
            .unwrap(),
    );
    let target_owner = NodeId::new(
        authored["document"]["footnotes"][1]["blocks"][0]["node_id"]
            .as_u64()
            .unwrap()
            .try_into()
            .unwrap(),
    );
    let prepared = input(&root, &authored, &caps);
    let mut budget = BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
    let mut attempts = Vec::new();
    let mut called = 0;
    let delivered = with_converged_column_pdf_observed(
        &prepared,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, observed| {
            called += 1;
            assert!(bundle.pdf().page_reference_labels_match());
            let actual = bundle
                .pdf()
                .page_references()
                .iter()
                .find(|r| r.owner() == owner)
                .unwrap();
            assert_eq!(actual.candidate_page(), 1);
            assert_eq!(actual.target_page(), None);
            assert!(!actual.is_placed());
            let destination = bundle
                .navigation()
                .destinations()
                .iter()
                .find(|d| d.owner() == target_owner)
                .expect("authentic unplaced anchor identity");
            assert_eq!(destination.position(), None);
            assert!(matches!(
                destination.kind(),
                typaxis_pdf::book_v2::BookV2DestinationKind::Anchor("unplaced-page-target")
            ));
            assert!(!bundle
                .pdf()
                .bytes()
                .windows(b"unplaced-page-target".len())
                .any(|window| window == b"unplaced-page-target"));
            observed
        },
        |attempt| attempts.push(Attempt::from_actual(attempt)),
    )
    .unwrap();
    assert_eq!(called, 1);
    assert_eq!(delivered, budget.observation());
    assert_eq!(attempts.len(), 2);
    for attempt in &attempts {
        assert_eq!(attempt.values.len(), 5);
        assert_eq!(attempt.values.iter().find(|r| r.0 == owner).unwrap().1, 1);
        assert_eq!(
            *attempt.references.iter().find(|r| r.0 == owner).unwrap(),
            (owner, 1, None, false)
        );
    }
    assert_eq!(attempts[0].decision, ColumnLabelDecision::Retry);
    assert_eq!(attempts[1].decision, ColumnLabelDecision::Ready);
    assert_eq!(attempts[0].display, attempts[1].display);
    assert_eq!(attempts[0].pdf, attempts[1].pdf);
    assert_eq!(attempts[0].bytes, attempts[1].bytes);
    eprintln!("Column actual unplaced-target None convergence: owner={owner:?}; attempts=2; references={:?}; after={:?}", attempts[1].references, quartet(delivered));
}
