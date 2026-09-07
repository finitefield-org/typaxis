#[test]
fn production_footnote_demand_deduplicates_and_keeps_branch_and_owner_identity() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
        ProductionFootnoteDemandStatus as Status,
    };
    let mut value = production_footnote_flow_fixture();
    let children = value["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    let n = children.len();
    children[n - 2]["footnote_id"] = "second".into();
    children[n - 1]["footnote_id"] = "first".into();
    children.push(children[n - 2].clone());
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let range = 0..flow.body_items().len();
        let state = search.require_body(&base, range.clone()).unwrap();
        assert_eq!(base.status(0), Some(Status::Unreferenced));
        assert!(base.pending_definitions().is_empty());
        assert_eq!(state.pending_definitions(), [1, 0]);
        assert_eq!(
            state.first_reference(1),
            Some(flow.references()[0].source().owner())
        );
        assert_eq!(state.status(2), None);
        let duplicate = search.require_body(&state, range.clone()).unwrap();
        assert_eq!(duplicate.pending_definitions(), [1, 0]);
        let maximum = flow.footnote_region().unwrap().height().get();
        let selected = search.evaluate_next(&state, maximum).unwrap().unwrap();
        assert_eq!(selected.fragment().definition_index(), 1);
        assert_eq!(
            search.advance(&duplicate, &selected).err().unwrap().kind,
            E::ReceiptMismatch
        );
        let after = search.advance(&state, &selected).unwrap();
        assert_eq!(after.status(1), Some(Status::Complete));
        assert_eq!(after.pending_definitions(), [0]);
        assert_eq!(
            search.advance(&after, &selected).err().unwrap().kind,
            E::ReceiptMismatch
        );
        let records = search.record_charge();
        let other_branch = search.advance(&state, &selected).unwrap();
        assert_eq!(
            other_branch.pending_definitions(),
            after.pending_definitions()
        );
        assert!(search.record_charge() > records);
        let mut other = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        assert_eq!(
            other
                .require_body(&state, range.clone())
                .err()
                .unwrap()
                .kind,
            E::ReceiptMismatch
        );
        assert_eq!(
            other.evaluate_next(&state, maximum).err().unwrap().kind,
            E::ReceiptMismatch
        );
        let other_state = other.begin().unwrap();
        assert_eq!(
            search
                .evaluate_next(&other_state, maximum)
                .err()
                .unwrap()
                .kind,
            E::ReceiptMismatch
        );
        for invalid in [2..1, 0..flow.body_items().len() + 1] {
            assert_eq!(
                search.require_body(&state, invalid).err().unwrap().kind,
                E::ReceiptMismatch
            );
        }
        let mut remaining = after;
        while !remaining.pending_definitions().is_empty() {
            let selected = search.evaluate_next(&remaining, maximum).unwrap().unwrap();
            remaining = search.advance(&remaining, &selected).unwrap();
        }
        let final_state = search.require_body(&remaining, range).unwrap();
        assert!(final_state.pending_definitions().is_empty());
        assert_eq!(final_state.status(0), Some(Status::Complete));
        assert_eq!(final_state.status(1), Some(Status::Complete));
        assert_eq!(final_state.first_reference(1), state.first_reference(1));
        assert!(search
            .evaluate_next(&final_state, maximum)
            .unwrap()
            .is_none());
    });
}

#[test]
fn production_footnote_demand_continuations_consume_only_the_selected_branch() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionFootnoteDemandStatus as Status,
    };
    let mut value = production_footnote_break_fixture();
    let span = value["document"]["footnotes"][0]["span"].clone();
    value["document"]["footnotes"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(
            0,
            serde_json::json!({"kind":"page_break","node_id":0,"span":span,"classes":[]}),
        );
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let state = search
            .require_body(&base, 0..flow.body_items().len())
            .unwrap();
        let leading = search.evaluate_next(&state, Length::ZERO).unwrap().unwrap();
        assert_eq!(leading.fragment().consumed_range(), 0..1);
        assert!(leading.fragment().items().is_empty());
        assert!(leading.fragment().marker().is_none());
        let state = search.advance(&state, &leading).unwrap();
        let records = search.record_charge();
        assert!(search
            .evaluate_next(&state, Length::ZERO)
            .unwrap()
            .is_none());
        assert!(search.record_charge() > records);
        let height = flow.definition_items(0).unwrap()[1]
            .consumed_height()
            .unwrap();
        let capacity = height.checked_add(height).unwrap();
        let dropped = search.evaluate_next(&state, capacity).unwrap().unwrap();
        let expected = dropped.fragment().consumed_range();
        assert!(dropped.fragment().marker().is_some());
        drop(dropped);
        let retry = search.evaluate_next(&state, capacity).unwrap().unwrap();
        assert_eq!(retry.fragment().consumed_range(), expected);
        let mut end = expected.end;
        let mut current = search.advance(&state, &retry).unwrap();
        assert_eq!(state.status(0), Some(Status::Pending));
        while !current.pending_definitions().is_empty() {
            let selected = search.evaluate_next(&current, capacity).unwrap().unwrap();
            assert_eq!(selected.fragment().consumed_range().start, end);
            assert!(selected.fragment().marker().is_none());
            end = selected.fragment().consumed_range().end;
            current = search.advance(&current, &selected).unwrap();
        }
        assert_eq!(end, flow.definition_items(0).unwrap().len());
        assert_eq!(current.status(0), Some(Status::Complete));
    });
}

#[test]
fn production_footnote_demand_nested_cycle_does_not_requeue_seen_definitions() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionFootnoteDemandStatus as Status,
    };
    let mut value = production_footnote_nested_reference_fixture();
    value["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .pop();
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_untagged_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let mut state = search
            .require_body(&base, 0..flow.body_items().len())
            .unwrap();
        assert_eq!(state.pending_definitions(), [0]);
        let first_nested_owner = flow
            .references()
            .iter()
            .find(|r| r.source().source_definition() == Some(0))
            .unwrap()
            .source()
            .owner();
        let mut transitions = 0;
        while !state.pending_definitions().is_empty() {
            assert!(
                transitions < 20,
                "cycle must terminate after actual finite content"
            );
            let selected = search
                .evaluate_next(&state, flow.footnote_region().unwrap().height().get())
                .unwrap()
                .unwrap();
            state = search.advance(&state, &selected).unwrap();
            assert!(state.pending_definitions().len() <= 2);
            transitions += 1;
        }
        assert_eq!(state.status(0), Some(Status::Complete));
        assert_eq!(state.status(1), Some(Status::Complete));
        assert_eq!(state.first_reference(1), Some(first_nested_owner));
        assert!(transitions > 2); // Actual forced breaks/continuations were consumed.
    });
}

#[test]
fn production_footnote_demand_snapshots_and_attempts_share_exact_budgets() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_footnote_break_fixture();
    let mut required_records = 0;
    let mut required_work = 0;
    for (record_delta, work_delta) in [
        (None, None),
        (Some(0), None),
        (Some(1), None),
        (None, Some(0)),
        (None, Some(1)),
    ] {
        let cfg = record_delta.map_or_else(config, |delta| {
            config_with_limits(ResourceLimits {
                max_fragments: required_records - delta,
                ..ResourceLimits::default()
            })
        });
        with_production_footnote_prepared(&value, &cfg, |flow, limits| {
            let work_limit = work_delta.map_or(100_000, |delta| required_work - delta);
            let mut search =
                prepare_production_footnote_demand_search(flow, limits, work_limit).unwrap();
            let capacity = flow.definition_items(0).unwrap()[0]
                .consumed_height()
                .unwrap();
            let result = (|| {
                let base = search.begin()?;
                let demanded = search.require_body(&base, 0..flow.body_items().len())?;
                let selected = search.select_region(&demanded, capacity)?.unwrap();
                search.require_body(selected.next_state(), 0..flow.body_items().len())
            })();
            if record_delta == Some(1) {
                assert_eq!(result.err().unwrap().kind, E::FragmentLimit);
            } else if work_delta == Some(1) {
                assert_eq!(result.err().unwrap().kind, E::FootnoteSearchLimit);
            } else {
                let state = result.unwrap();
                assert_eq!(state.pending_definitions(), [0]);
                required_records = search.record_charge();
                required_work = search.work_steps();
                if record_delta == Some(0) {
                    assert_eq!(search.begin().err().unwrap().kind, E::FragmentLimit);
                }
                if work_delta == Some(0) {
                    assert_eq!(search.begin().err().unwrap().kind, E::FootnoteSearchLimit);
                }
            }
        });
    }
}

#[test]
fn production_footnote_demand_region_reserves_actual_definition_spacing() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
        ProductionFootnoteDemandStatus as Status,
    };
    let mut value = production_footnote_flow_fixture();
    value["document"]["footnotes"][0]["blocks"] =
        value["document"]["footnotes"][1]["blocks"].clone();
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let state = search
            .require_body(&base, 0..flow.body_items().len())
            .unwrap();
        let first = &flow.definition_items(0).unwrap()[0];
        let second = &flow.definition_items(1).unwrap()[0];
        let first_height = first.consumed_height().unwrap();
        let gap = first
            .space_after()
            .checked_add(second.space_before())
            .unwrap();
        assert!(gap > Length::ZERO);
        let offset = first_height.checked_add(gap).unwrap();
        let height = offset
            .checked_add(second.consumed_height().unwrap())
            .unwrap();
        let selected = search.select_region(&state, height).unwrap().unwrap();
        selected.verify(&state).unwrap();
        assert!(selected.verify(&base).is_err());
        assert_eq!(selected.fragments().len(), 2);
        assert_eq!(selected.fragments()[0].offset(), Length::ZERO);
        assert_eq!(selected.fragments()[1].offset(), offset);
        assert_eq!(selected.used_height(), height);
        assert_eq!(selected.available_height(), height);
        assert!(selected.next_state().pending_definitions().is_empty());
        assert_eq!(selected.next_state().status(1), Some(Status::Complete));
        assert_eq!(state.pending_definitions(), [0, 1]);
        let short = search
            .select_region(
                &state,
                height.checked_sub(Length::from_raw(1).unwrap()).unwrap(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(short.fragments().len(), 1);
        assert_eq!(short.used_height(), first_height);
        assert_eq!(short.next_state().pending_definitions(), [1]);
        assert!(search
            .select_region(
                &state,
                first_height
                    .checked_sub(Length::from_raw(1).unwrap())
                    .unwrap()
            )
            .unwrap()
            .is_none());
        assert!(search
            .select_region(selected.next_state(), height)
            .unwrap()
            .is_none());
        for invalid in [
            Length::from_raw(-1).unwrap(),
            flow.footnote_region()
                .unwrap()
                .height()
                .get()
                .checked_add(Length::from_raw(1).unwrap())
                .unwrap(),
        ] {
            assert_eq!(
                search.select_region(&state, invalid).err().unwrap().kind,
                E::InvalidFootnoteCapacity
            );
        }
    });
}

#[test]
fn production_footnote_demand_region_preserves_forced_boundaries_and_continuations() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionFootnoteDemandStatus as Status,
    };
    let mut value = production_footnote_break_fixture();
    let paragraph = value["document"]["footnotes"][0]["blocks"][0].clone();
    let forced =
        serde_json::json!({"kind":"page_break","node_id":0,"span":paragraph["span"],"classes":[]});
    value["document"]["footnotes"][0]["blocks"] = serde_json::json!([forced, paragraph, forced]);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let state = search
            .require_body(&base, 0..flow.body_items().len())
            .unwrap();
        let leading = search.select_region(&state, Length::ZERO).unwrap().unwrap();
        assert_eq!(leading.fragments().len(), 1);
        assert_eq!(leading.used_height(), Length::ZERO);
        assert!(leading.forced_break_owner().is_some());
        let mut regions = vec![leading];
        let capacity = Length::from_raw(2 * 917_504).unwrap();
        let mut consumed = 1;
        let mut markers = 0;
        loop {
            let previous = regions.last().unwrap();
            let next = search
                .select_region(previous.next_state(), capacity)
                .unwrap()
                .unwrap();
            next.verify(previous.next_state()).unwrap();
            assert_eq!(next.fragments().len(), 1);
            let fragment = next.fragments()[0].fragment();
            assert_eq!(fragment.consumed_range().start, consumed);
            consumed = fragment.consumed_range().end;
            markers += usize::from(fragment.marker().is_some());
            let complete = next.next_state().pending_definitions().is_empty();
            if complete {
                assert!(next.forced_break_owner().is_some());
                assert_eq!(next.next_state().status(0), Some(Status::Complete));
            }
            regions.push(next);
            if complete {
                break;
            }
        }
        assert_eq!(markers, 1);
        assert_eq!(consumed, flow.definition_items(0).unwrap().len());
        assert_eq!(state.status(0), Some(Status::Pending));
    });
}
