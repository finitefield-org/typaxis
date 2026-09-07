fn production_footnote_two_long_definitions() -> serde_json::Value {
    let mut value = production_footnote_break_fixture();
    let mut second = value["document"]["footnotes"][0].clone();
    second["footnote_id"] = "note-b".into();
    value["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let children = value["document"]["blocks"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap();
    let mut reference = children.last().unwrap().clone();
    reference["footnote_id"] = "note-b".into();
    children.push(reference);
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_footnote_required_region_starts_every_pending_definition_before_extras() {
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let value = production_footnote_two_long_definitions();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let state = search
            .require_body(&base, 0..flow.body_items().len())
            .unwrap();
        let first = &flow.definition_items(0).unwrap()[0];
        let second = &flow.definition_items(1).unwrap()[0];
        let gap = first
            .space_after()
            .checked_add(second.space_before())
            .unwrap();
        let height = first.consumed_height().unwrap();
        let minimum = height
            .checked_add(gap)
            .unwrap()
            .checked_add(second.consumed_height().unwrap())
            .unwrap();
        let greedy = search.select_region(&state, minimum).unwrap().unwrap();
        assert_eq!(greedy.fragments().len(), 1);
        let required = search
            .select_required_region(&state, minimum)
            .unwrap()
            .unwrap();
        required.verify(&state).unwrap();
        assert_eq!(required.fragments().len(), 2);
        assert_eq!(required.used_height(), minimum);
        for (index, fragment) in required.fragments().iter().enumerate() {
            assert_eq!(fragment.fragment().definition_index(), index);
            assert_eq!(fragment.fragment().items().len(), 1);
            assert!(fragment.fragment().marker().is_some());
            assert!(
                fragment
                    .fragment()
                    .candidates()
                    .iter()
                    .all(|candidate| candidate.used_height()
                        <= fragment.fragment().available_height())
            );
        }
        assert_eq!(required.next_state().pending_definitions(), [0, 1]);
        assert!(search
            .select_required_region(
                &state,
                minimum.checked_sub(Length::from_raw(1).unwrap()).unwrap()
            )
            .unwrap()
            .is_none());
        let extra = minimum
            .checked_add(height)
            .unwrap()
            .checked_add(height)
            .unwrap();
        let expanded = search
            .select_required_region(&state, extra)
            .unwrap()
            .unwrap();
        assert_eq!(expanded.fragments()[0].fragment().items().len(), 3);
        assert_eq!(expanded.fragments()[1].fragment().items().len(), 1);
        assert_eq!(expanded.used_height(), extra);
        let continuation = search
            .select_required_region(required.next_state(), minimum)
            .unwrap()
            .unwrap();
        assert!(continuation
            .fragments()
            .iter()
            .all(|f| f.fragment().marker().is_none()));
    });
}

fn production_footnote_joint_geometry_fixture() -> serde_json::Value {
    let mut value = production_footnote_flow_fixture();
    let paragraph = value["document"]["footnotes"][1]["blocks"][0].clone();
    value["document"]["footnotes"][0]["blocks"] = serde_json::json!([paragraph]);
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(paragraph);
    production_body_renumber(&mut value["document"], &mut 0);
    let mut body_first = 0;
    let mut reservation = 0;
    with_production_footnote_prepared(&value, &config(), |flow, _| {
        assert_eq!(flow.body_items().len(), 2);
        body_first = flow.body_items()[0].consumed_height().unwrap().raw();
        let first = &flow.definition_items(0).unwrap()[0];
        let second = &flow.definition_items(1).unwrap()[0];
        reservation = first.consumed_height().unwrap().raw()
            + first.space_after().raw()
            + second.space_before().raw()
            + second.consumed_height().unwrap().raw()
            + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW;
    });
    let master = &mut value["page_masters"]["masters"][0];
    master["body"]["height"] = (body_first + reservation).into();
    master["footnote"]["height"] = reservation.into();
    master["footnote"]["y"] = (master["body"]["y"].as_i64().unwrap() + body_first).into();
    value
}

#[test]
fn production_footnote_joint_candidate_accounts_for_separator_and_body_collision() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionFootnoteDemandStatus as Status,
    };
    let mut value = production_footnote_joint_geometry_fixture();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        assert!(search
            .evaluate_body_candidate(&base, 0..2)
            .unwrap()
            .is_none());
        let fits = search
            .evaluate_body_candidate(&base, 0..1)
            .unwrap()
            .unwrap();
        fits.verify(&base).unwrap();
        assert_eq!(fits.body_range(), 0..1);
        assert_eq!(
            fits.body_height(),
            flow.body_items()[0].consumed_height().unwrap()
        );
        let bounds = fits.footnote_bounds().unwrap();
        assert_eq!(bounds, flow.footnote_region().unwrap());
        assert_eq!(
            bounds.height().get().raw(),
            fits.footnotes().unwrap().used_height().raw()
                + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW
        );
        assert_eq!(fits.next_state().status(0), Some(Status::Complete));
        assert_eq!(base.status(0), Some(Status::Unreferenced));
        let alternate = search.begin().unwrap();
        assert!(fits.verify(&alternate).is_err());
    });
    // Move the declared region beside the body. Shared y no longer collides.
    let body_right = value["page_masters"]["masters"][0]["body"]["x"]
        .as_i64()
        .unwrap()
        + value["page_masters"]["masters"][0]["body"]["width"]
            .as_i64()
            .unwrap();
    value["page_masters"]["masters"][0]["footnote"]["x"] = body_right.into();
    value["page_masters"]["masters"][0]["footnote"]["width"] = 3_000_000.into();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let fits = search
            .evaluate_body_candidate(&base, 0..2)
            .unwrap()
            .unwrap();
        assert_eq!(fits.footnote_bounds().unwrap().x().raw(), body_right);
    });
    // The declared maximum can also be entirely above the body.
    let master = &mut value["page_masters"]["masters"][0];
    master["footnote"]["x"] = master["body"]["x"].clone();
    master["footnote"]["y"] = 0.into();
    let body_y = master["footnote"]["height"].as_i64().unwrap() + 65_536;
    master["body"]["y"] = body_y.into();
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        let fits = search
            .evaluate_body_candidate(&base, 0..2)
            .unwrap()
            .unwrap();
        let bounds = fits.footnote_bounds().unwrap();
        assert!(bounds.y().raw() + bounds.height().get().raw() <= body_y);
    });
}

#[test]
fn production_footnote_joint_candidate_rejects_keep_cuts_and_unstarted_references() {
    use typaxis_pagination::prepare_production_footnote_demand_search;
    let mut value = production_footnote_joint_geometry_fixture();
    value["page_masters"]["masters"][0]["body"]["height"] = 10_000_000.into();
    value["page_masters"]["masters"][0]["footnote"]["y"] = 13_000_000.into();
    production_body_set_style(&mut value, "paragraph", "keep_with_next", true.into());
    with_production_footnote_prepared(&value, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        assert!(search
            .evaluate_body_candidate(&base, 0..1)
            .unwrap()
            .is_none());
        assert!(search
            .evaluate_body_candidate(&base, 1..2)
            .unwrap()
            .is_none());
        assert!(search
            .evaluate_body_candidate(&base, 0..2)
            .unwrap()
            .is_some());
    });
    let mut forced = production_footnote_two_long_definitions();
    let span = forced["document"]["footnotes"][0]["span"].clone();
    forced["document"]["footnotes"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(
            0,
            serde_json::json!({"kind":"page_break","node_id":0,"span":span,"classes":[]}),
        );
    production_body_renumber(&mut forced["document"], &mut 0);
    with_production_footnote_prepared(&forced, &config(), |flow, limits| {
        let mut search = prepare_production_footnote_demand_search(flow, limits, 100_000).unwrap();
        let base = search.begin().unwrap();
        assert!(search
            .evaluate_body_candidate(&base, 0..flow.body_items().len())
            .unwrap()
            .is_none());
    });
}

#[test]
fn production_footnote_joint_candidate_preserves_cumulative_record_and_work_budgets() {
    use typaxis_pagination::{
        prepare_production_footnote_demand_search, ProductionBodyPaginationErrorKind as E,
    };
    let value = production_footnote_two_long_definitions();
    let mut records = 0;
    let mut work = 0;
    for (record_delta, work_delta) in [
        (None, None),
        (Some(0), None),
        (Some(1), None),
        (None, Some(0)),
        (None, Some(1)),
    ] {
        let cfg = record_delta.map_or_else(config, |d| {
            config_with_limits(ResourceLimits {
                max_fragments: records - d,
                ..ResourceLimits::default()
            })
        });
        with_production_footnote_prepared(&value, &cfg, |flow, limits| {
            let mut search = prepare_production_footnote_demand_search(
                flow,
                limits,
                work_delta.map_or(100_000, |d| work - d),
            )
            .unwrap();
            let base = search.begin().unwrap();
            let result = search.evaluate_body_candidate(&base, 0..flow.body_items().len());
            if record_delta == Some(1) {
                assert_eq!(result.err().unwrap().kind, E::FragmentLimit);
            } else if work_delta == Some(1) {
                assert_eq!(result.err().unwrap().kind, E::FootnoteSearchLimit);
            } else {
                let selected = result.unwrap().unwrap();
                assert_eq!(selected.footnotes().unwrap().fragments().len(), 2);
                records = search.record_charge();
                work = search.work_steps();
                if record_delta == Some(0) {
                    assert_eq!(
                        search
                            .evaluate_body_candidate(&base, 0..flow.body_items().len())
                            .err()
                            .unwrap()
                            .kind,
                        E::FragmentLimit
                    );
                }
                if work_delta == Some(0) {
                    assert_eq!(
                        search
                            .evaluate_body_candidate(&base, 0..flow.body_items().len())
                            .err()
                            .unwrap()
                            .kind,
                        E::FootnoteSearchLimit
                    );
                }
            }
        });
    }
}
