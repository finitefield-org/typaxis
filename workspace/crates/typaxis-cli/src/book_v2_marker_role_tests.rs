use super::*;

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_original_harano_bullets_follow_text_figure_and_numbered_math_items() {
    let root = Root::new();
    let limits = limits();
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let mut data = numbered_data();
    data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
    data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    let span = data["document"]["blocks"][0]["span"].clone();
    let blocks = data["document"]["blocks"][0]["blocks"]
        .as_array()
        .unwrap()
        .clone();
    let items = blocks
        .into_iter()
        .map(|block| json!({"node_id":0,"span":span,"blocks":[block]}))
        .collect::<Vec<_>>();
    data["document"]["blocks"][0]["blocks"] = json!([{"kind":"list","node_id":0,"span":span,"classes":[],"ordered":false,"start":null,"items":items}]);
    let mut rule = data["style_sheet"]["rules"][2].clone();
    rule["selector"] = "list".into();
    rule["style_id"] = "list-text".into();
    rule["source_order"] = 4.into();
    data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(rule);
    fn renumber(v: &mut Value, next: &mut u32) {
        if v.get("node_id").is_some() {
            v["node_id"] = (*next).into();
            *next += 1;
        }
        for key in ["blocks", "children", "items", "caption"] {
            if let Some(children) = v.get_mut(key).and_then(Value::as_array_mut) {
                for child in children {
                    renumber(child, next);
                }
            }
        }
        if let Some(number) = v.get_mut("equation_number").filter(|n| !n.is_null()) {
            renumber(number, next);
        }
    }
    renumber(&mut data["document"], &mut 0);
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 22_000_000.into();
    master["height"] = 24_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":22_000_000,"height":24_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":20_000_000,"height":20_000_000});
    let body = body_with_source(&root, data, VECTOR_SOURCE, &limits);
    fs::write(root.0.join("body.bin"), &bytes).unwrap();
    fs::write(root.0.join("vector.svg"), VECTOR).unwrap();
    let input = prepare_book_v2_resources(
        body,
        &root.context(),
        &config(limits.base().get().clone()),
        &limits,
    )
    .unwrap();
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let raw = |n| Length::from_raw(n).unwrap();
    let body = typaxis_core::Rect::new(
        raw(500_000),
        raw(500_000),
        PositiveLength::new(raw(20_000_000)).unwrap(),
        PositiveLength::new(raw(20_000_000)).unwrap(),
    );
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        body,
        1_000_000,
        |lines| {
            let shapes =
                shape_book_v2_equation_numbers(lines.lines().prepared().shaped(), &limits, 0)
                    .unwrap()
                    .unwrap();
            let blocks = typaxis_layout::book_v2::prepare_book_v2_vector_blocks(
                lines.lines(),
                Some(&shapes),
                &limits,
                0,
            )
            .unwrap()
            .unwrap();
            let measured = typaxis_pagination::book_v2::prepare_book_v2_table_measurements(
                typaxis_pagination::book_v2::prepare_book_v2_body_flow(
                    lines.lines(),
                    Some(&blocks),
                    lines.footnotes(),
                    &limits,
                    0,
                )
                .unwrap(),
                &limits,
            )
            .unwrap();
            let mut search = typaxis_pagination::book_v2::prepare_book_v2_table_body_search(
                &measured, &limits, 1_000_000, 0,
            )
            .unwrap();
            let stable = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
            let terminals = search
                .finalize_mixed_page_math(closure, &limits, 0)
                .unwrap();
            assert_math_display(&terminals, input.resources(), &limits);
            let mut builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                &terminals,
                input.resources(),
                &limits,
                1_000_000,
                0,
                0,
            )
            .unwrap();
            let display = builder.build_markers().unwrap();
            assert_eq!(display.draws().len(), 3);
            assert!(display.draws().iter().all(|d| d.source().utf8() == "•"));
            assert!(display
                .draws()
                .iter()
                .all(|d| d.source().font().content_hash() == typaxis_core::sha256(&bytes)));
            assert!(display.draws().iter().all(|d| d
                .clusters()
                .iter()
                .flat_map(|c| c.glyphs())
                .all(|g| g.original_gid().get() != 0)));
            assert_eq!(
                display
                    .draws()
                    .iter()
                    .filter(|d| matches!(
                        d.fragment().fragment().source(),
                        typaxis_pagination::ProductionBodyFragmentSource::VectorBlock { .. }
                    ))
                    .count(),
                2
            );
        },
    )
    .unwrap();
}
