use super::*;
use typaxis_shaping::book_v2::{
    shape_book_v2_equation_numbers, BookV2EquationNumberErrorKind as E,
};
fn numbered_data() -> Value {
    let mut d = vector_data();
    let mut rule = d["style_sheet"]["rules"][2].clone();
    rule["selector"] = "math_vector_block".into();
    rule["style_id"] = "equation-text".into();
    rule["source_order"] = 3.into();
    d["style_sheet"]["rules"].as_array_mut().unwrap().push(rule);
    d
}
#[test]
fn book_v2_equation_numbers_keep_actual_source_glyphs_font_and_cumulative_budget() {
    let root = Root::new();
    let limits = limits();
    let input = vector_input(&root, numbered_data(), &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let shaped = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    let numbers = shape_book_v2_equation_numbers(&shaped, &limits, 0)
        .unwrap()
        .unwrap();
    numbers.verify(&shaped, &limits).unwrap();
    assert_eq!(numbers.shapes().len(), 1);
    let number = numbers.shape(NodeId::new(8)).unwrap();
    assert!(std::ptr::eq(
        number.source(),
        bindings.receipt(NodeId::new(8)).unwrap().source()
    ));
    assert_eq!(
        number.source().equation_number().unwrap().node_id(),
        NodeId::new(9)
    );
    assert_eq!(number.text(), "(1)");
    assert_eq!(number.font_face_id(), FontFaceId::new(0));
    assert_eq!(number.font_instance_id().get(), 0);
    assert_eq!(number.font_sha256(), typaxis_core::sha256(FONT));
    assert_eq!(number.height().get().raw(), 1_048_576);
    assert_eq!(
        number.language(),
        nav.language(NodeId::new(8)).unwrap().effective_language()
    );
    assert_eq!(
        number.width().get().raw(),
        number
            .runs()
            .iter()
            .flat_map(|r| r.glyphs())
            .map(|g| g.advance_x.raw())
            .sum::<i64>()
    );
    let mut text = String::new();
    for run in number.runs() {
        assert!(run.glyphs().iter().all(|g| g.original_gid.get() > 0));
        let span = run.source_span();
        assert_eq!(span.text_id().get(), 0);
        text.push_str(
            &std::str::from_utf8(VECTOR_SOURCE).unwrap()
                [span.start_byte().get() as usize..span.end_byte().get() as usize],
        );
    }
    assert_eq!(text, "(1)");
    let prior = limits.base().get().max_fragments - numbers.record_charge();
    assert_eq!(
        shape_book_v2_equation_numbers(&shaped, &limits, prior)
            .unwrap()
            .unwrap()
            .record_charge(),
        limits.base().get().max_fragments
    );
    assert!(shape_book_v2_equation_numbers(&shaped, &limits, prior + 1).is_err());
    let other = shape_book_v2_authored_text(
        &policy,
        &flow,
        input.resources(),
        &limits,
        bindings.epoch(),
        None,
    )
    .unwrap();
    assert!(numbers.verify(&other, &limits).is_err());
    assert!(numbers.shape(NodeId::new(7)).is_none());
}
#[test]
fn book_v2_equation_numbers_reject_incomplete_style_and_skip_numberless_source() {
    let root = Root::new();
    let limits = limits();
    let input = vector_input(&root, vector_data(), &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let shaped =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
            .unwrap();
    assert!(matches!(
        shape_book_v2_equation_numbers(&shaped, &limits, 0)
            .err()
            .unwrap()
            .kind,
        E::MissingTextStyle
    ));
    let root = Root::new();
    let input = prepared(&root, source_data("Result"), b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let shaped =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
            .unwrap();
    assert!(shape_book_v2_equation_numbers(&shaped, &limits, 0)
        .unwrap()
        .is_none());
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_equation_numbers_use_original_harano_cff_font_instances() {
    let root = Root::new();
    let limits = limits();
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let mut d = numbered_data();
    d["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
    d["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    let body = body_with_source(&root, d, VECTOR_SOURCE, &limits);
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
    let shaped =
        shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
            .unwrap();
    let numbers = shape_book_v2_equation_numbers(&shaped, &limits, 0)
        .unwrap()
        .unwrap();
    let number = numbers.shape(NodeId::new(8)).unwrap();
    assert_eq!(number.font_sha256(), typaxis_core::sha256(&bytes));
    assert_eq!(number.text(), "(1)");
    assert!(number.width().get().raw() > 0);
    assert!(number
        .runs()
        .iter()
        .flat_map(|r| r.glyphs())
        .all(|g| g.original_gid.get() > 0));
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let body = typaxis_core::Rect::new(
        Length::from_raw(500_000).unwrap(),
        Length::from_raw(500_000).unwrap(),
        PositiveLength::new(Length::from_raw(10_000_000).unwrap()).unwrap(),
        PositiveLength::new(Length::from_raw(20_000_000).unwrap()).unwrap(),
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
            let numbers =
                shape_book_v2_equation_numbers(lines.lines().prepared().shaped(), &limits, 0)
                    .unwrap()
                    .unwrap();
            assert_eq!(
                numbers.shapes()[0].font_sha256(),
                typaxis_core::sha256(&bytes)
            );
            let blocks = typaxis_layout::book_v2::prepare_book_v2_vector_blocks(
                lines.lines(),
                Some(&numbers),
                &limits,
                0,
            )
            .unwrap()
            .unwrap();
            let flow = typaxis_pagination::book_v2::prepare_book_v2_body_flow(
                lines.lines(),
                Some(&blocks),
                lines.footnotes(),
                &limits,
                0,
            )
            .unwrap();
            let measured =
                typaxis_pagination::book_v2::prepare_book_v2_table_measurements(flow, &limits)
                    .unwrap();
            let mut search = typaxis_pagination::book_v2::prepare_book_v2_table_body_search(
                &measured, &limits, 1_000_000, 0,
            )
            .unwrap();
            let stable = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
            assert_eq!(closure.semantic_math(), 2);
            assert_eq!(closure.repeated_math(), 0);
            let terminals = search
                .finalize_mixed_page_math(closure, &limits, 0)
                .unwrap();
            assert_math_terminals(&terminals);
            assert_math_display(&terminals, input.resources(), &limits);
            let actual = placed
                .pages()
                .iter()
                .flat_map(|p| p.equation_numbers())
                .collect::<Vec<_>>();
            assert_eq!(actual.len(), 1);
            assert!(!actual[0].repeated_header());
            assert_eq!(
                actual[0].geometry().shape_fingerprint(),
                numbers.shapes()[0].fingerprint()
            );
            assert_eq!(
                actual[0].geometry().bounds().width(),
                numbers.shapes()[0].width()
            );
            assert_eq!(actual[0].geometry().parent_owner(), NodeId::new(8));
            assert_eq!(actual[0].geometry().owner(), NodeId::new(9));
        },
    )
    .unwrap();
}
#[path = "book_v2_block_tests.rs"]
pub(in crate::book_v2_resources::tests::shaping_tests) mod blocks;

#[path = "book_v2_number_bidi_tests.rs"]
mod number_bidi;

#[path = "book_v2_marker_role_tests.rs"]
mod marker_roles;

#[test]
fn book_v2_driver_assembles_all_vector_kinds_and_authored_equation_labels() {
    let root = Root::new();
    let limits = limits();
    let mut data = numbered_data();
    data["page_masters"]["masters"][0]["body"] =
        json!({"x":500_000,"y":500_000,"width":10_000_000,"height":20_000_000});
    let input = vector_input(&root, data, &limits);
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            assert_eq!(observation.candidate_passes(), 1);
            let display = pdf
                .navigation()
                .source()
                .source()
                .source()
                .source()
                .display();
            assert!(!display.numbers().draws().is_empty());
            assert!(!display.images().draws().is_empty());
            assert!(!display.math().draws().is_empty());
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_number_references_equation_number_leaf_keeps_math_destination() {
    let mut data = numbered_data();
    let span = json!({"source_id":0,"start_byte":19,"end_byte":19});
    data["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "kind":"paragraph", "node_id":10, "classes":[], "span":span,
            "children":[{"kind":"reference", "node_id":11, "span":span,
                "target":"equation.one", "format":"number"}]
        }));
    data["document"]["number_bindings"] = json!([{
        "anchor_id":"equation.one", "owner_node_id":8, "label_node_id":9,
        "text_span":{"text_id":0,"start_byte":17,"end_byte":18}
    }]);
    data["page_masters"]["masters"][0]["body"] =
        json!({"x":500000,"y":500000,"width":10000000,"height":3000000});
    data["page_masters"]["masters"][0]["width"] = 12_000_000.into();
    data["page_masters"]["masters"][0]["height"] = 12_000_000.into();
    data["page_masters"]["masters"][0]["trim"] =
        json!({"x":0,"y":0,"width":12_000_000,"height":12_000_000});
    let root = Root::new();
    let limits = limits();
    let input = vector_input(&root, data, &limits);
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            let marked = pdf.navigation().source().source().source();
            let display = marked.source().display();
            let flow = display
                .source()
                .source()
                .flow()
                .lines()
                .prepared()
                .source_flow();
            assert_eq!(flow.reference_text(NodeId::new(11)), Some("1"));
            assert_eq!(display.numbers().draws().len(), 1);
            assert_eq!(pdf.navigation().links().len(), 1);
            assert_eq!(pdf.navigation().rectangles().len(), 1);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}

