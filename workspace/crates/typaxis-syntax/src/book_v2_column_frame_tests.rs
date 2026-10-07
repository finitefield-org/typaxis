use super::*;
use typaxis_core::{Length, Rect};
use typaxis_document_package::WireColumnBalance;
use BookV2PageMasterError as E;

fn input() -> Value {
    let mut data = root(FIXTURE);
    let original = data["page_masters"]["masters"][0].clone();
    let masters: Vec<_> = [
        ("a-main", 17, 23, 303, 190, 3, 11, "none"),
        ("b-first", 31, 47, 206, 210, 2, 10, "last_page"),
        ("c-odd", 37, 53, 323, 180, 4, 7, "none"),
        ("d-even", 41, 59, 421, 220, 1, 0, "none"),
        ("e-named", 43, 61, 407, 250, 3, 13, "last_page"),
        ("z-unreachable", 0, 0, 1, 1, 65535, 100, "none"),
    ]
    .into_iter()
    .map(|(id, x, y, w, h, count, gap, balance)| {
        let mut master = original.clone();
        master["master_id"] = id.into();
        master["width"] = 1000.into();
        master["height"] = 1000.into();
        master["trim"] = json!({"x":0,"y":0,"width":1000,"height":1000});
        master["body"] = json!({"x":x,"y":y,"width":w,"height":h});
        master["footnote"] = json!({"x":57,"y":400,"width":390,"height":55});
        master["column_layout"] = if count == 1 {
            Value::Null
        } else {
            json!({"count":count,"gap":gap,"fill":"sequential","balance":balance})
        };
        master
    })
    .collect();
    data["page_masters"]["masters"] = json!(masters);
    data["page_masters"]["default_master_id"] = "a-main".into();
    data["page_masters"]["selection_rules"] = json!([
        {"master_id":"b-first","parity":"any","first":true,"named_page":null,"source_order":0},
        {"master_id":"c-odd","parity":"even","first":null,"named_page":null,"source_order":1},
        {"master_id":"d-even","parity":"odd","first":false,"named_page":null,"source_order":2},
        {"master_id":"e-named","parity":"any","first":null,"named_page":"appendix","source_order":3}
    ]);
    data["style_sheet"]["rules"].as_array_mut().unwrap().extend([
        json!({"style_id":"paragraph-text","selector":"paragraph","source_order":2,"extends":null,"declarations":[
            {"name":"font_family","important":false,"value":{"kind":"font_family_list","families":["Body"]}},
            {"name":"font_size","important":false,"value":{"kind":"length","value":786432}},
            {"name":"line_height","important":false,"value":{"kind":"length","value":1048576}}
        ]}),
        json!({"style_id":"appendix-scope","selector":"semantic_container.nested","source_order":3,"extends":null,
            "declarations":[{"name":"page","important":false,"value":{"kind":"string","value":"appendix"}}]})
    ]);
    data
}
fn styled(data: &Value, bound: &ValidatedResourceLimits) -> StyledBookV2Body {
    style_book_v2_body(prepare_book_v2_body(decode(data, bound), bound).unwrap()).unwrap()
}
fn rectangle(r: Rect) -> [i64; 4] {
    [
        r.x().raw(),
        r.y().raw(),
        r.width().get().raw(),
        r.height().get().raw(),
    ]
}

#[test]
fn column_plan_binds_original_names_first_parity_and_full_note_region() {
    let mut data = input();
    let mut paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    paragraph["classes"] = json!(["column-note"]);
    data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({
        "style_id":"column-note","selector":"paragraph.column-note","source_order":4,"extends":null,
        "declarations":[{"name":"page","important":false,"value":{"kind":"string","value":"appendix"}}]
    }));
    data["document"]["footnotes"] = json!([{"footnote_id":"note","node_id":0,
        "span":paragraph["span"],"blocks":[paragraph]}]);
    renumber(&mut data["document"], &mut 0);
    let bound = limits();
    let body = styled(&data, &bound);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
    assert!(plan.has_source_names());
    plan.verify(&body).unwrap();
    assert!(std::ptr::eq(plan.source(), &body));
    let name = plan.source_name_index(NodeId::new(4)).unwrap();
    assert_eq!(plan.name(name), Some("appendix"));
    assert_eq!(plan.source_name_index(NodeId::new(11)), Some(name));
    for (page, expected) in [
        (0, vec![[31, 47, 98, 210], [139, 47, 98, 210]]),
        (
            1,
            vec![
                [37, 53, 75, 180],
                [119, 53, 75, 180],
                [201, 53, 75, 180],
                [283, 53, 77, 180],
            ],
        ),
        (2, vec![[41, 59, 421, 220]]),
        (
            3,
            vec![
                [37, 53, 75, 180],
                [119, 53, 75, 180],
                [201, 53, 75, 180],
                [283, 53, 77, 180],
            ],
        ),
    ] {
        let frames = plan.page(page).unwrap();
        assert!(std::ptr::eq(frames.source(), &body));
        assert_eq!(frames.page_index(), page);
        assert_eq!(frames.named_page_index(), None);
        assert_eq!(
            (0..frames.column_count())
                .map(|i| rectangle(frames.column(i).unwrap()))
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(rectangle(frames.footnote().unwrap()), [57, 400, 390, 55]);
        assert_eq!(frames.column(frames.column_count()), Err(E::Identity));
    }
    for page in [0, 1, 2, 3, 17] {
        let frames = plan.named_page(page, Some(name)).unwrap();
        assert_eq!(frames.named_page_index(), Some(name));
        assert_eq!(
            frames.column_layout().unwrap().balance,
            WireColumnBalance::LastPage
        );
        assert_eq!(
            (0..frames.column_count())
                .map(|i| rectangle(frames.column(i).unwrap()))
                .collect::<Vec<_>>(),
            [[43, 61, 127, 250], [183, 61, 127, 250], [323, 61, 127, 250]]
        );
        assert_eq!(rectangle(frames.body()), [43, 61, 407, 250]);
    }
    assert_eq!(rectangle(plan.measurement_body()), [31, 47, 421, 250]);
    assert_eq!(
        rectangle(plan.measurement_footnote().unwrap()),
        [57, 400, 390, 55]
    );
    assert_eq!(plan.minimum_body_height().raw(), 180);
    assert_eq!(plan.minimum_footnote_height().unwrap().raw(), 55);
    assert!(plan.requires_width_reflow());
    assert!(matches!(
        plan.named_page(0, Some(usize::MAX)),
        Err(E::Identity)
    ));
    assert!(matches!(
        plan.page(bound.get().max_pages),
        Err(E::PageLimit)
    ));
    let foreign = styled(&data, &bound);
    assert_eq!(plan.verify(&foreign), Err(E::Identity));
    // Current consumers still reject columns; this geometry grants no PDF authority.
    assert!(matches!(
        prepare_book_v2_page_frame_plan_with_regions(&flow, &mut 0, 1_000_000, 0, 0),
        Err(E::UnsupportedAdvanced)
    ));
}

#[test]
fn column_plan_shares_exact_work_record_spool_bounds_and_keeps_failures_charged() {
    let data = input();
    let bound = limits();
    let body = styled(&data, &bound);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let mut work = 19;
    let mut observed = BookV2ColumnFramePlanObservation::default();
    let full = prepare_book_v2_column_frame_plan_counted(
        &flow,
        &mut work,
        1_000_000,
        11,
        13,
        &mut observed,
    )
    .unwrap();
    assert_eq!(observed.record_charge(), 11 + full.record_charge());
    assert_eq!(observed.spool_charge(), 13 + full.spool_charge());
    let records = bound.get().max_fragments - full.record_charge();
    let spool = bound.get().max_spool_bytes - full.spool_charge();
    let exact = prepare_book_v2_column_frame_plan_counted(
        &flow,
        &mut 19,
        work,
        records,
        spool,
        &mut observed,
    )
    .unwrap();
    assert_eq!(exact.record_charge(), full.record_charge());
    assert_eq!(exact.spool_charge(), full.spool_charge());
    assert_eq!(observed.record_charge(), bound.get().max_fragments);
    assert_eq!(observed.spool_charge(), bound.get().max_spool_bytes);
    let mut failed_work = 19;
    assert!(matches!(
        prepare_book_v2_column_frame_plan(&flow, &mut failed_work, work - 1, records, spool),
        Err(E::WorkLimit)
    ));
    assert!(failed_work > 19);
    assert!(matches!(
        prepare_book_v2_column_frame_plan(&flow, &mut 19, work, records + 1, spool),
        Err(E::RecordLimit)
    ));
    assert!(matches!(
        prepare_book_v2_column_frame_plan(&flow, &mut 19, work, records, spool + 1),
        Err(E::SpoolLimit)
    ));
    // Accept some column templates, then reject the next class's reservation.
    let initial = bound.get().max_fragments - 3;
    assert!(matches!(
        prepare_book_v2_column_frame_plan_counted(
            &flow,
            &mut 0,
            1_000_000,
            initial,
            13,
            &mut observed
        ),
        Err(E::RecordLimit)
    ));
    assert_eq!(observed.record_charge(), initial + 2);
    assert_eq!(observed.spool_charge(), 13);
    // The name reservation succeeds before its string budget fails.
    assert!(matches!(
        prepare_book_v2_column_frame_plan_counted(
            &flow,
            &mut 0,
            1_000_000,
            11,
            bound.get().max_spool_bytes,
            &mut observed
        ),
        Err(E::SpoolLimit)
    ));
    assert!(observed.record_charge() > 11);
    assert_eq!(observed.spool_charge(), bound.get().max_spool_bytes);
    // Invalid caller history is retained without starting a new template.
    assert!(matches!(
        prepare_book_v2_column_frame_plan_counted(
            &flow,
            &mut 0,
            1_000_000,
            bound.get().max_fragments + 1,
            13,
            &mut observed
        ),
        Err(E::RecordLimit)
    ));
    assert_eq!(observed.record_charge(), bound.get().max_fragments + 1);
    assert_eq!(observed.spool_charge(), 13);
    let mut repeated = 0;
    for _ in 0..2 {
        let previous = repeated;
        assert!(matches!(
            prepare_book_v2_column_frame_plan(&flow, &mut repeated, 1_000_000, records + 1, spool),
            Err(E::RecordLimit)
        ));
        assert!(repeated > previous);
    }
}

#[test]
fn column_plan_covers_default_without_rules_and_maximum_authored_count() {
    for (count, width, gap, expected, reflow) in [
        (3, 303, 11, vec![93, 93, 95], true),
        (3, 301, 11, vec![93, 93, 93], false),
        (65535, 65535, 0, vec![1; 65535], false),
    ] {
        let mut data = input();
        data["page_masters"]["masters"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        data["page_masters"]["selection_rules"] = json!([]);
        data["page_masters"]["masters"][0]["body"]["width"] = width.into();
        data["page_masters"]["masters"][0]["column_layout"]["count"] = count.into();
        data["page_masters"]["masters"][0]["column_layout"]["gap"] = gap.into();
        // Eliminate source names, so every reachable class has this one partition.
        data["style_sheet"]["rules"].as_array_mut().unwrap().pop();
        let bound = limits();
        let body = styled(&data, &bound);
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
        let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
        let frames = plan.page(0).unwrap();
        assert_eq!(
            (0..frames.column_count())
                .map(|i| frames.column(i).unwrap().width().get().raw())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(plan.requires_width_reflow(), reflow);
        assert_eq!(
            plan.measurement_body().width().get().raw(),
            *expected.last().unwrap()
        );
        assert!(plan.record_charge() >= 3 * count as u64);
    }
}

#[test]
fn column_plan_rejects_selected_nonpositive_width_but_ignores_unreachable_templates() {
    let mut data = input();
    data["page_masters"]["masters"][1]["body"]["width"] = 10.into();
    let bound = limits();
    let body = styled(&data, &bound);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let mut work = 0;
    let mut observed = BookV2ColumnFramePlanObservation::default();
    assert!(matches!(
        prepare_book_v2_column_frame_plan_counted(
            &flow,
            &mut work,
            1_000_000,
            11,
            13,
            &mut observed
        ),
        Err(E::Geometry)
    ));
    assert!(work > 0);
    assert_eq!(observed.record_charge(), 13);
    assert_eq!(observed.spool_charge(), 13);
    // The same invalid first master is unreachable after its rule is removed.
    data["page_masters"]["selection_rules"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    let body = styled(&data, &bound);
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
    assert_eq!(
        (0..3)
            .map(|i| plan.page(0).unwrap().column(i).unwrap().width().get().raw())
            .collect::<Vec<_>>(),
        [93, 93, 95]
    );
    assert_eq!(
        plan.page(0).unwrap().body().x(),
        Length::from_raw(17).unwrap()
    );
}

#[test]
fn column_plan_checks_coordinate_and_gap_overflow_before_any_count_sized_storage() {
    const MAX: i64 = 9_007_199_254_740_991;
    for (x, width, count, gap, accepted) in [
        (MAX - 2, 2, 2, 0, true),
        (MAX - 1, 2, 2, 0, false),
        (MAX, 2, 2, 0, false),
        (-MAX, 2, 2, 0, true),
        (0, 206, 2, MAX, false),
        (0, 206, 65535, MAX, false),
    ] {
        let mut data = input();
        data["page_masters"]["masters"][1]["body"]["x"] = x.into();
        data["page_masters"]["masters"][1]["body"]["width"] = width.into();
        data["page_masters"]["masters"][1]["column_layout"]["count"] = count.into();
        data["page_masters"]["masters"][1]["column_layout"]["gap"] = gap.into();
        let bound = limits();
        let body = styled(&data, &bound);
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
        let mut observed = BookV2ColumnFramePlanObservation::default();
        let result = prepare_book_v2_column_frame_plan_counted(
            &flow,
            &mut 0,
            1_000_000,
            0,
            0,
            &mut observed,
        );
        if accepted {
            let frames = result.unwrap().page(0).unwrap();
            assert_eq!(frames.column(0).unwrap().x().raw(), x);
            assert_eq!(frames.column(1).unwrap().x().raw(), x + 1);
        } else {
            assert!(
                matches!(result, Err(E::Geometry)),
                "{x} {width} {count} {gap}"
            );
            assert_eq!(observed.record_charge(), count as u64);
        }
    }
}

#[test]
fn column_plan_only_reserves_classes_reachable_within_the_page_cap() {
    let data = input();
    let mut charges = Vec::new();
    for cap in [1, 2, 3] {
        let bound = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: cap,
            ..ResourceLimits::default()
        })
        .unwrap();
        let body = styled(&data, &bound);
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
        let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
        charges.push(plan.record_charge());
        assert_eq!(
            plan.minimum_body_height().raw(),
            if cap == 1 { 210 } else { 180 }
        );
        assert_eq!(
            plan.measurement_body().width().get().raw(),
            if cap < 3 { 127 } else { 421 }
        );
        assert!(matches!(plan.page(cap), Err(E::PageLimit)));
    }
    assert_eq!(charges[1] - charges[0], 7);
    assert_eq!(charges[2] - charges[1], 4);
}
