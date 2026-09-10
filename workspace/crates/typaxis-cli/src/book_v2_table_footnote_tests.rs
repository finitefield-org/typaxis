use super::*;
use typaxis_pagination::book_v2::prepare_book_v2_table_footnote_search;
use typaxis_pagination::ProductionFootnoteDemandStatus as Status;

#[test]
fn book_v2_table_and_footnotes_share_retries_header_references_and_continuation_budgets() {
    let root = Root::new();
    let limits = limits();
    let mut data = table_data();
    let span = data["document"]["footnotes"][0]["span"].clone();
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    data["document"]["footnotes"] = json!([
        {"node_id":0,"span":span,"footnote_id":"note","blocks":vec![paragraph.clone();8]},
        {"node_id":0,"span":span,"footnote_id":"later","blocks":[paragraph]}
    ]);
    let table = &mut data["document"]["blocks"][0]["blocks"][0];
    table["head"][0]["cells"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"note"}));
    table["body"][2]["cells"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"later"}));
    renumber(&mut data["document"], &mut 0);
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings =
        typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,rect(500_000,500_000,10_000_000,20_000_000),1_000_000,|stable| {
        let measured = prepare_book_v2_table_measurements(prepare_book_v2_body_flow(stable.lines(),None,stable.footnotes(),&limits,0).unwrap(),&limits).unwrap();
        assert_eq!(measured.flow().references().len(),3);
        let table = &measured.tables()[0];
        let capacity = table.rows()[0].height().checked_add(table.rows()[1].height()).unwrap();
        let run = |prior,work| -> Result<(u64,u64,Vec<[u8;32]>),ProductionBodyPaginationError> {
            let mut search = prepare_book_v2_table_footnote_search(&measured,0,&limits,work,prior)?;
            let current = search.begin()?;
            assert_eq!(current.demand().status(0),Some(Status::Unreferenced));
            assert!(search.evaluate(&current,Length::ZERO)?.is_none());
            // Both referenced definitions cannot be started in this region.
            assert!(search.evaluate(&current,table.height())?.is_none());
            assert!(current.table_cursor().is_initial());
            assert!(current.demand().pending_definitions().is_empty());
            let mut fingerprints = Vec::new();
            let mut markers = [0,0];
            let mut references = Vec::new();
            let mut note_only = 0;
            let mut attempts = 0;
            let mut selected = None;
            loop {
                let state = selected.as_ref().map_or(&current,|s: &typaxis_pagination::book_v2::BookV2TableFootnoteSelection<'_,'_,'_,'_,'_>| s.next_state());
                if state.is_complete() { assert!(search.evaluate(state,capacity)?.is_none()); break; }
                attempts += 1;
                assert!(attempts < 32);
                let full = search.evaluate(state,capacity)?;
                let next = match full { Some(next) => next, None => search.evaluate(state,Length::ZERO)?.expect("pending notes advance without claiming another table cut") };
                next.verify(state)?;
                if let Some(fragment) = next.table() {
                    assert_eq!(fragment.repeats_header(),!fingerprints.is_empty());
                    assert!(fragment.after().offset() > state.table_cursor().offset());
                    for range in fragment.semantic_leaf_ranges() {
                        for reference in measured.flow().references_in_items(None,range) { references.push(reference.source().owner()); }
                    }
                    fingerprints.push(fragment.fingerprint());
                } else {
                    note_only += 1;
                    assert_eq!(next.next_state().table_cursor().offset(),state.table_cursor().offset());
                    assert_eq!(next.next_state().table_cursor().next_row(),state.table_cursor().next_row());
                }
                if let Some(region) = next.footnotes() {
                    for fragment in region.fragments() {
                        markers[fragment.fragment().definition_index()] += usize::from(fragment.fragment().marker().is_some());
                    }
                    assert_eq!(next.footnote_bounds().unwrap().height().get().raw(),region.used_height().raw()+typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW);
                }
                selected = Some(next);
            }
            assert!(fingerprints.len() >= 3);
            assert!(note_only > 0);
            assert_eq!(markers,[1,1]);
            assert_eq!(references.len(),2);
            assert_ne!(references[0],references[1]);
            assert_eq!(current.demand().status(0),Some(Status::Unreferenced));
            assert_eq!(current.demand().status(1),Some(Status::Unreferenced));
            let sibling = search.begin()?;
            assert!(matches!(selected.as_ref().unwrap().verify(&sibling),Err(e) if e.kind==Error::ReceiptMismatch));
            assert!(matches!(search.evaluate(&current,Length::from_raw(-1).unwrap()),Err(e) if e.kind==Error::InvalidTableCapacity));
            assert!(matches!(search.evaluate(&current,Length::from_raw(20_000_001).unwrap()),Err(e) if e.kind==Error::InvalidTableCapacity));
            // Keep the source branch alive across every discarded capacity.
            assert!(current.table_cursor().is_initial());
            Ok((search.record_charge(),search.work_charge(),fingerprints))
        };
        let (records,work,fingerprints) = run(0,1_000_000).unwrap();
        let exact_prior = limits.base().get().max_fragments-(records-measured.record_charge());
        assert_eq!(run(exact_prior,work).unwrap(),(limits.base().get().max_fragments,work,fingerprints));
        assert!(matches!(run(exact_prior+1,work),Err(e) if e.kind==Error::FragmentLimit));
        assert!(matches!(run(0,work-1),Err(e) if e.kind==Error::FootnoteSearchLimit));
        let mut probe = prepare_book_v2_table_footnote_search(&measured,0,&limits,1_000_000,0).unwrap();
        let _ = probe.begin().unwrap();
        let maximum = probe.work_charge()+1;
        let mut exhausted = prepare_book_v2_table_footnote_search(&measured,0,&limits,maximum,0).unwrap();
        let unchanged = exhausted.begin().unwrap();
        let before = exhausted.record_charge();
        assert!(matches!(exhausted.evaluate(&unchanged,capacity),Err(e) if e.kind==Error::TableSearchLimit));
        assert_eq!(exhausted.work_charge(),maximum);
        assert!(exhausted.record_charge()>before);
        let after_failure = exhausted.record_charge();
        assert!(matches!(exhausted.evaluate(&unchanged,capacity),Err(e) if e.kind==Error::TableSearchLimit));
        assert_eq!(exhausted.work_charge(),maximum);
        assert!(exhausted.record_charge()>after_failure);
        assert!(unchanged.table_cursor().is_initial());
        assert!(unchanged.demand().pending_definitions().is_empty());
        assert!(matches!(exhausted.begin(),Err(e) if e.kind==Error::FootnoteSearchLimit));
        let mut first = prepare_book_v2_table_footnote_search(&measured,0,&limits,1_000_000,0).unwrap();
        let mut other = prepare_book_v2_table_footnote_search(&measured,0,&limits,1_000_000,0).unwrap();
        let foreign = other.begin().unwrap();
        assert!(matches!(first.evaluate(&foreign,capacity),Err(e) if e.kind==Error::ReceiptMismatch));
    }).unwrap();
}
