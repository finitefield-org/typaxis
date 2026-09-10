use super::*;
use typaxis_pagination::book_v2::BookV2PreparedBodyFlow;

fn with_joint_flow(
    mut data: Value,
    mut check: impl FnMut(&BookV2PreparedBodyFlow<'_, '_, '_, '_>, &M4EffectiveResourceLimits),
) {
    let rect = &data["page_masters"]["masters"][0]["body"];
    let n = |key: &str| Length::from_raw(rect[key].as_i64().unwrap()).unwrap();
    let body = Rect::new(
        n("x"),
        n("y"),
        PositiveLength::new(n("width")).unwrap(),
        PositiveLength::new(n("height")).unwrap(),
    );
    renumber(&mut data["document"], &mut 0);
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body,
        1_000_000,
        |stable| {
            let measured =
                prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                    .unwrap();
            check(&measured, &limits);
        },
    )
    .unwrap();
}
fn joint_data() -> Value {
    let mut data = input_data();
    data["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .truncate(3);
    let plain = data["document"]["footnotes"][1]["blocks"][0].clone();
    data["document"]["footnotes"][0]["blocks"] = json!([plain]);
    data["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(plain);
    data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"joint-spacing","selector":"paragraph","source_order":3,"extends":null,"declarations":[
        {"name":"space_before","important":false,"value":{"kind":"length","value":65_536}},
        {"name":"space_after","important":false,"value":{"kind":"length","value":65_536}}
    ]}));
    let mut first_height = 0;
    let mut reservation = 0;
    with_joint_flow(data.clone(), |flow, _| {
        assert_eq!(flow.body_items().len(), 2);
        first_height = flow.body_items()[0].consumed_height().unwrap().raw();
        let first = &flow.definition_items(0).unwrap()[0];
        let second = &flow.definition_items(1).unwrap()[0];
        reservation = first.consumed_height().unwrap().raw()
            + first.space_after().raw()
            + second.space_before().raw()
            + second.consumed_height().unwrap().raw()
            + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW;
    });
    let master = &mut data["page_masters"]["masters"][0];
    master["body"]["height"] = (first_height + reservation).into();
    master["footnote"]["height"] = reservation.into();
    master["footnote"]["y"] = (master["body"]["y"].as_i64().unwrap() + first_height).into();
    data
}
#[test]
fn book_v2_joint_fit_reserves_separator_and_rejects_body_overlap_with_exact_budgets() {
    with_joint_flow(joint_data(), |flow, limits| {
        let run = |prior, work| -> Result<(u64, u64), ProductionBodyPaginationError> {
            let mut search = prepare_book_v2_footnote_demand_search(flow, limits, work, prior)?;
            let base = search.begin()?;
            assert!(search.evaluate_body_candidate(&base, 0..2)?.is_none());
            let fit = search.evaluate_body_candidate(&base, 0..1)?.unwrap();
            fit.verify(&base)?;
            assert_eq!(fit.body_range(), 0..1);
            assert_eq!(
                fit.body_height(),
                flow.body_items()[0].consumed_height().unwrap()
            );
            let bounds = fit.footnote_bounds().unwrap();
            let frames = flow.lines().frames().unwrap();
            assert_eq!(bounds, frames.footnote_region().unwrap());
            assert_eq!(
                bounds.height().get().raw(),
                fit.footnotes().unwrap().used_height().raw()
                    + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW
            );
            assert_eq!(
                bounds.y(),
                frames.body().y().checked_add(fit.body_height()).unwrap()
            );
            assert_eq!(fit.footnotes().unwrap().fragments().len(), 2);
            assert!(fit.next_state().pending_definitions().is_empty());
            assert_eq!(base.status(0), Some(Status::Unreferenced));
            let sibling = search.begin()?;
            assert!(matches!(fit.verify(&sibling),Err(e) if e.kind==Error::ReceiptMismatch));
            let repeated = search
                .evaluate_body_candidate(fit.next_state(), 0..2)?
                .unwrap();
            assert!(repeated.footnotes().is_none());
            assert!(repeated.footnote_bounds().is_none());
            let demanded = search.require_body(&base, 0..1)?;
            let notes_only = search.evaluate_body_candidate(&demanded, 0..0)?.unwrap();
            assert_eq!(notes_only.body_height(), Length::ZERO);
            assert!(notes_only.next_state().pending_definitions().is_empty());
            assert!(
                matches!(search.evaluate_body_candidate(&base,0..3),Err(e) if e.kind==Error::ReceiptMismatch)
            );
            Ok((search.record_charge(), search.work_steps()))
        };
        let (records, work) = run(0, 1_000_000).unwrap();
        let exact_prior = limits.base().get().max_fragments - (records - flow.record_charge());
        assert_eq!(
            run(exact_prior, work).unwrap(),
            (limits.base().get().max_fragments, work)
        );
        assert!(matches!(run(exact_prior+1,work),Err(e) if e.kind==Error::FragmentLimit));
        assert!(matches!(run(0,work-1),Err(e) if e.kind==Error::FootnoteSearchLimit));
        let mut first = prepare_book_v2_footnote_demand_search(flow, limits, 1_000_000, 0).unwrap();
        let mut other = prepare_book_v2_footnote_demand_search(flow, limits, 1_000_000, 0).unwrap();
        let foreign = other.begin().unwrap();
        assert!(
            matches!(first.evaluate_body_candidate(&foreign,0..1),Err(e) if e.kind==Error::ReceiptMismatch)
        );
    });
}
#[test]
fn book_v2_joint_fit_accepts_nonoverlapping_regions_and_honors_keep_and_forced_body_cuts() {
    let mut data = joint_data();
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 42_000_000.into();
    master["trim"]["width"] = 42_000_000.into();
    let right = master["body"]["x"].as_i64().unwrap() + master["body"]["width"].as_i64().unwrap();
    master["footnote"]["x"] = right.into();
    master["footnote"]["width"] = 10_000_000.into();
    with_joint_flow(data.clone(), |flow, limits| {
        let mut search =
            prepare_book_v2_footnote_demand_search(flow, limits, 1_000_000, 0).unwrap();
        let state = search.begin().unwrap();
        let fit = search
            .evaluate_body_candidate(&state, 0..2)
            .unwrap()
            .unwrap();
        assert_eq!(fit.footnote_bounds().unwrap().x().raw(), right);
    });
    let master = &mut data["page_masters"]["masters"][0];
    master["footnote"]["x"] = master["body"]["x"].clone();
    master["footnote"]["y"] = 0.into();
    let body_y = master["footnote"]["height"].as_i64().unwrap() + 65_536;
    master["body"]["y"] = body_y.into();
    with_joint_flow(data.clone(), |flow, limits| {
        let mut search =
            prepare_book_v2_footnote_demand_search(flow, limits, 1_000_000, 0).unwrap();
        let state = search.begin().unwrap();
        let fit = search
            .evaluate_body_candidate(&state, 0..2)
            .unwrap()
            .unwrap();
        let bounds = fit.footnote_bounds().unwrap();
        assert!(bounds.y().raw() + bounds.height().get().raw() <= body_y);
    });
    let mut kept = data.clone();
    kept["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"joint-keep","selector":"paragraph","source_order":4,"extends":null,"declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    with_joint_flow(kept, |flow, limits| {
        let mut search =
            prepare_book_v2_footnote_demand_search(flow, limits, 1_000_000, 0).unwrap();
        let state = search.begin().unwrap();
        assert!(search
            .evaluate_body_candidate(&state, 0..1)
            .unwrap()
            .is_none());
        assert!(search
            .evaluate_body_candidate(&state, 1..2)
            .unwrap()
            .is_none());
        assert!(search
            .evaluate_body_candidate(&state, 0..2)
            .unwrap()
            .is_some());
    });
    let span = data["document"]["blocks"][0]["span"].clone();
    data["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(
            1,
            json!({"kind":"page_break","node_id":0,"classes":[],"span":span}),
        );
    with_joint_flow(data, |flow, limits| {
        let mut search =
            prepare_book_v2_footnote_demand_search(flow, limits, 1_000_000, 0).unwrap();
        let state = search.begin().unwrap();
        assert_eq!(flow.body_items().len(), 3);
        assert!(search
            .evaluate_body_candidate(&state, 0..3)
            .unwrap()
            .is_none());
        assert!(search
            .evaluate_body_candidate(&state, 0..1)
            .unwrap()
            .is_some());
        assert!(search
            .evaluate_body_candidate(&state, 2..3)
            .unwrap()
            .is_some());
    });
}
#[test]
fn book_v2_joint_fit_starts_new_definition_references_in_the_same_region() {
    let mut data = joint_data();
    data["document"]["footnotes"][0]["blocks"] =
        input_data()["document"]["footnotes"][0]["blocks"].clone();
    // Preserve roomy geometry while deciding whether both references actually
    // occur in the source body candidate.
    data["page_masters"]["masters"][0]["footnote"]["y"] = 15_000_000.into();
    with_joint_flow(data.clone(), |flow, limits| {
        let mut search =
            prepare_book_v2_footnote_demand_search(flow, limits, 1_000_000, 0).unwrap();
        let base = search.begin().unwrap();
        assert!(search
            .evaluate_body_candidate(&base, 0..1)
            .unwrap()
            .is_some());
    });
    data["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .truncate(2);
    with_joint_flow(data, |flow, limits| {
        let mut search =
            prepare_book_v2_footnote_demand_search(flow, limits, 1_000_000, 0).unwrap();
        let base = search.begin().unwrap();
        let fit = search
            .evaluate_body_candidate(&base, 0..1)
            .unwrap()
            .unwrap();
        assert_eq!(fit.footnotes().unwrap().fragments().len(), 2);
        assert_eq!(fit.next_state().status(0), Some(Status::Complete));
        assert_eq!(fit.next_state().status(1), Some(Status::Complete));
        assert_eq!(base.status(0), Some(Status::Unreferenced));
        assert_eq!(base.status(1), Some(Status::Unreferenced));
    });
}

#[test]
fn book_v2_joint_dependency_backtracking_discards_unselected_references_and_charges_retries() {
    let mut data = joint_data();
    data["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .truncate(2);
    let plain = data["document"]["footnotes"][1]["blocks"][0].clone();
    let nested = input_data()["document"]["footnotes"][0]["blocks"][0].clone();
    data["document"]["footnotes"][0]["blocks"] = json!([plain, nested]);
    // Derive a region that can hold A in full, or A's continuation plus B,
    // but cannot hold all three paragraphs simultaneously.
    let mut capacity = 0;
    with_joint_flow(data.clone(), |flow, _| {
        let a = flow.definition_items(0).unwrap();
        let b = flow.definition_items(1).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(b.len(), 1);
        let h = |i: &typaxis_pagination::ProductionBodyFlowItem| i.consumed_height().unwrap().raw();
        let a_height = h(&a[0]) + a[0].space_after().raw() + a[1].space_before().raw() + h(&a[1]);
        let tail_height =
            h(&a[1]) + a[1].space_after().raw() + b[0].space_before().raw() + h(&b[0]);
        capacity = a_height.max(tail_height);
        assert!(
            capacity < a_height + a[1].space_after().raw() + b[0].space_before().raw() + h(&b[0])
        );
    });
    let master = &mut data["page_masters"]["masters"][0];
    master["footnote"]["y"] = 15_000_000.into();
    master["footnote"]["height"] = (capacity + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
    with_joint_flow(data, |flow, limits| {
        let run = |prior, work| -> Result<(u64, u64), ProductionBodyPaginationError> {
            let mut search = prepare_book_v2_footnote_demand_search(flow, limits, work, prior)?;
            let base = search.begin()?;
            let demanded = search.require_body(&base, 0..1)?;
            let greedy = search
                .select_required_region(&demanded, Length::from_raw(capacity).unwrap())?
                .unwrap();
            assert_eq!(greedy.fragments().len(), 1);
            assert_eq!(greedy.fragments()[0].fragment().items().unwrap().len(), 2);
            assert!(matches!(
                greedy.next_state().status(1),
                Some(Status::Pending)
            ));
            let fit = search.evaluate_body_candidate(&base, 0..1)?.unwrap();
            fit.verify(&base)?;
            let region = fit.footnotes().unwrap();
            assert_eq!(region.fragments().len(), 1);
            assert_eq!(region.fragments()[0].fragment().items().unwrap().len(), 1);
            assert!(matches!(fit.next_state().status(0), Some(Status::Pending)));
            assert_eq!(fit.next_state().status(1), Some(Status::Unreferenced));
            assert_eq!(fit.next_state().first_reference(1), None);
            assert_eq!(base.status(0), Some(Status::Unreferenced));
            let second = search
                .evaluate_body_candidate(fit.next_state(), 0..0)?
                .unwrap();
            assert_eq!(second.footnotes().unwrap().fragments().len(), 2);
            assert_eq!(second.next_state().status(0), Some(Status::Complete));
            assert_eq!(second.next_state().status(1), Some(Status::Complete));
            let reference = flow
                .references()
                .iter()
                .find(|r| r.source().source_definition() == Some(0))
                .unwrap()
                .source()
                .owner();
            assert_eq!(second.next_state().first_reference(1), Some(reference));
            Ok((search.record_charge(), search.work_steps()))
        };
        let (records, work) = run(0, 1_000_000).unwrap();
        let prior = limits.base().get().max_fragments - (records - flow.record_charge());
        assert_eq!(
            run(prior, work).unwrap(),
            (limits.base().get().max_fragments, work)
        );
        assert!(matches!(run(prior + 1, work), Err(e) if e.kind == Error::FragmentLimit));
        assert!(matches!(run(0, work - 1), Err(e) if e.kind == Error::FootnoteSearchLimit));
        let mut search =
            prepare_book_v2_footnote_demand_search(flow, limits, 1_000_000, 0).unwrap();
        let base = search.begin().unwrap();
        let first = search
            .evaluate_body_candidate(&base, 0..1)
            .unwrap()
            .unwrap();
        let before = (search.record_charge(), search.work_steps());
        let again = search
            .evaluate_body_candidate(&base, 0..1)
            .unwrap()
            .unwrap();
        assert_eq!(first.next_state().status(0), again.next_state().status(0));
        assert_eq!(first.next_state().status(1), again.next_state().status(1));
        assert!(search.record_charge() > before.0);
        assert!(search.work_steps() > before.1);
    });
}

#[test]
fn book_v2_joint_dependency_rejects_one_unit_short_without_retaining_partial_demands() {
    let mut data = joint_data();
    data["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .truncate(2);
    data["document"]["footnotes"][0]["blocks"] =
        input_data()["document"]["footnotes"][0]["blocks"].clone();
    let mut required = 0;
    with_joint_flow(data.clone(), |flow, _| {
        let a = &flow.definition_items(0).unwrap()[0];
        let b = &flow.definition_items(1).unwrap()[0];
        required = a.consumed_height().unwrap().raw()
            + a.space_after().raw()
            + b.space_before().raw()
            + b.consumed_height().unwrap().raw()
            + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW;
    });
    for short in [false, true] {
        let mut candidate = data.clone();
        candidate["page_masters"]["masters"][0]["footnote"]["y"] = 15_000_000.into();
        candidate["page_masters"]["masters"][0]["footnote"]["height"] =
            (required - i64::from(short)).into();
        with_joint_flow(candidate, |flow, limits| {
            let mut search =
                prepare_book_v2_footnote_demand_search(flow, limits, 1_000_000, 0).unwrap();
            let base = search.begin().unwrap();
            let fit = search.evaluate_body_candidate(&base, 0..1).unwrap();
            assert_eq!(fit.is_none(), short);
            if let Some(fit) = fit {
                assert_eq!(fit.footnotes().unwrap().fragments().len(), 2);
                assert_eq!(fit.next_state().status(0), Some(Status::Complete));
                assert_eq!(fit.next_state().status(1), Some(Status::Complete));
                assert_eq!(
                    fit.footnote_bounds().unwrap().height().get().raw(),
                    required
                );
            }
            assert_eq!(base.status(0), Some(Status::Unreferenced));
            assert_eq!(base.status(1), Some(Status::Unreferenced));
            assert_eq!(base.first_reference(1), None);
            if short {
                let before = (search.record_charge(), search.work_steps());
                assert!(search
                    .evaluate_body_candidate(&base, 0..1)
                    .unwrap()
                    .is_none());
                assert!(search.record_charge() > before.0);
                assert!(search.work_steps() > before.1);
            }
        });
    }
}
