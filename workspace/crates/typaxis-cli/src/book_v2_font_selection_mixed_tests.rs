use super::*;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_layout::book_v2::{bind_book_v2_vectors, with_converged_book_v2_body_lines};
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
    prepare_book_v2_table_measurements,
};

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_font_selection_keeps_original_harano_and_truetype_in_one_display() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hash = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let root = Root::new();
    let limits = limits();
    let mut data = data();
    data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
    data["resources"]["font_faces"][0]["expected_sha256"] = hash.into();
    data["document"]["blocks"][0]["blocks"][1]["blocks"][0]["classes"] = json!(["collection"]);
    data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"collection-text","selector":"paragraph.collection","source_order":3,"extends":null,"declarations":[{"name":"font_family","important":false,"value":{"kind":"font_family_list","families":["Collection"]}}]}));
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 12_000_000.into();
    master["height"] = 12_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":12_000_000,"height":12_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":10_000_000,"height":10_000_000});
    let source = body_with_source(&root, data, SOURCE, &limits);
    fs::write(root.0.join("body.bin"), &bytes).unwrap();
    let input = prepare_book_v2_resources(
        source,
        &root.context(),
        &config(ResourceLimits::default()),
        &limits,
    )
    .unwrap();
    let navigation = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &navigation).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let extent = PositiveLength::new(Length::from_raw(10_000_000).unwrap()).unwrap();
    let body = Rect::new(
        Length::from_raw(500_000).unwrap(),
        Length::from_raw(500_000).unwrap(),
        extent,
        extent,
    );
    with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        typaxis_linebreak::JapaneseLineBreakMode::Normal,
        body,
        1_000_000,
        |stable| {
            let measured = prepare_book_v2_table_measurements(
                prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                    .unwrap(),
                &limits,
            )
            .unwrap();
            let mut search =
                prepare_book_v2_table_body_search(&measured, &limits, 10_000_000, 0).unwrap();
            let pages = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(pages.sequence()).unwrap();
            let closure = search.close_mixed_page_sources(&pages, &placed).unwrap();
            let terminals = search
                .finalize_mixed_page_math(closure, &limits, 0)
                .unwrap();
            assert_math_display(&terminals, input.resources(), &limits);
            let mut display_builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                &terminals,
                input.resources(),
                &limits,
                10_000_000,
                0,
                0,
            )
            .unwrap();
            let display = display_builder.build_body().unwrap();
            let mut builder = typaxis_resources::book_v2::BookV2FontSelectionBuilder::new(
                &display, &limits, 10_000_000, 0, 0, 0,
            )
            .unwrap();
            let selection = builder.build().unwrap();
            assert_eq!(selection.fonts().len(), 2);
            assert!(matches!(
                selection.fonts()[0].instance().font(),
                typaxis_resources::AdmittedProductionFontV3::Cff1V2(_)
            ));
            assert!(matches!(
                selection.fonts()[1].instance().font(),
                typaxis_resources::AdmittedProductionFontV3::TrueType(_)
            ));
            assert_eq!(selection.fonts()[0].instance().font().bytes(), bytes);
            assert_eq!(selection.fonts()[1].instance().font().bytes(), COLLECTION);
            assert_eq!(
                selection
                    .fonts()
                    .iter()
                    .map(|f| f.instance().font().font_face_id().get())
                    .collect::<Vec<_>>(),
                [0, 1]
            );
            for i in 0..2 {
                assert!(selection.glyphs(i).unwrap().len() > 1);
            }
        },
    )
    .unwrap();
}
