use super::*;
use typaxis_core::{ImageResourceId, Length, NodeId, PositiveLength, Rect};
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, layout_book_v2_body_inline_lines, prepare_book_v2_text_inlines,
    with_converged_book_v2_body_lines,
};
use typaxis_layout::{ProductionFigureMedia, ProductionInlinePreparationErrorKind as E};
use typaxis_linebreak::JapaneseLineBreakMode;
pub(in crate::book_v2_resources::tests::shaping_tests) const JPEG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/profiles/production-book-1/combined/job/color-2x1.jpg"
));
pub(in crate::book_v2_resources::tests::shaping_tests) const SVG: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/x-plus-y.svg"));
pub(in crate::book_v2_resources::tests::shaping_tests) fn figure_data(media: &str, bytes: &[u8], width: Option<i64>) -> Value {
    let mut data = source_data("Result");
    let span = data["document"]["blocks"][0]["span"].clone();
    let mut caption = data["document"]["blocks"][0]["blocks"][0].clone();
    caption["node_id"] = 5.into();
    caption["children"][0]["node_id"] = 6.into();
    data["document"]["blocks"][0]["blocks"].as_array_mut().unwrap().push(json!({"kind":"figure","node_id":4,"classes":[],"span":span,"image_id":0,"placement":"block","alt":"Original diagram","caption":[caption]}));
    data["resources"]["images"] = json!([{"image_id":0,"uri":"figure.bin","media_type":media,"expected_sha256":typaxis_core::sha256(bytes).iter().map(|b|format!("{b:02x}")).collect::<String>()}]);
    if media == "svg-safe-2" {
        data["resources"]["images"][0]["vector_provenance"] = json!({"engine_id":"vmb.texToSvg","engine_version":"2026.09.0","rules_version":"vmb.math-safe-svg/1"});
    }
    if let Some(width) = width {
        data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"figure-width","selector":"figure","source_order":3,"extends":null,"declarations":[{"name":"width","important":false,"value":{"kind":"length","value":width}}]}));
    }
    data
}
pub(in crate::book_v2_resources::tests::shaping_tests) fn figure_input(
    root: &Root,
    data: Value,
    bytes: &[u8],
    limits: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    let body = body_with_source(root, data, b"Result", limits);
    fs::write(root.0.join("figure.bin"), bytes).unwrap();
    prepare_book_v2_resources(
        body,
        &root.context(),
        &config(limits.base().get().clone()),
        limits,
    )
    .unwrap()
}
#[test]
fn book_v2_ordinary_figures_use_actual_png_jpeg_svg_geometry_and_source_captions() {
    for (media, bytes, width, expected_width, expected_height, scale) in [
        ("png", PNG, Some(2_097_152), 2_097_152, 1_048_576, None),
        (
            "jpeg-baseline",
            JPEG,
            Some(2_097_152),
            2_097_152,
            1_048_576,
            None,
        ),
        ("svg-safe-2", SVG, None, 1_966_080, 786_432, Some(65_536)),
        (
            "svg-safe-2",
            SVG,
            Some(1_048_576),
            1_048_560,
            419_424,
            Some(34_952),
        ),
    ] {
        let root = Root::new();
        let limits = limits();
        let input = figure_input(&root, figure_data(media, bytes, width), bytes, &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let shaped =
            shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
                .unwrap();
        let prepared = prepare_book_v2_text_inlines(
            &flow,
            &shaped,
            input.resources(),
            &limits,
            EPOCH,
            JapaneseLineBreakMode::Normal,
        )
        .unwrap();
        assert_eq!(prepared.figures().len(), 1);
        assert_eq!(prepared.paragraphs().len(), 2);
        let figure = &prepared.figures()[0];
        assert!(std::ptr::eq(figure.source(), &flow.figures()[0]));
        assert_eq!(figure.source_index(), 0);
        assert_eq!(figure.owner(), NodeId::new(4));
        assert_eq!(figure.image_id(), ImageResourceId::new(0));
        assert_eq!(figure.admitted_sha256(), typaxis_core::sha256(bytes));
        assert_eq!(figure.source().alternative(), "Original diagram");
        assert_eq!(figure.width().get().raw(), expected_width);
        assert_eq!(figure.height().get().raw(), expected_height);
        match (figure.media(), scale) {
            (
                ProductionFigureMedia::Raster {
                    pixel_width,
                    pixel_height,
                },
                None,
            ) => assert_eq!((pixel_width, pixel_height), (2, 1)),
            (
                ProductionFigureMedia::Svg {
                    content_key,
                    scale_raw,
                },
                Some(scale),
            ) => {
                assert_eq!(scale_raw, scale);
                assert_eq!(
                    content_key,
                    typaxis_resources::VectorContentKey::from_admitted(
                        input.resources().image(ImageResourceId::new(0)).unwrap()
                    )
                    .unwrap()
                );
            }
            _ => panic!("actual media"),
        }
        let size = PositiveLength::new(Length::from_raw(10_000_000).unwrap()).unwrap();
        let body = Rect::new(Length::ZERO, Length::ZERO, size, size);
        let lines = layout_book_v2_body_inline_lines(&prepared, body, 1_000_000).unwrap();
        assert_eq!(lines.paragraphs()[1].owner(), NodeId::new(5));
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
                let final_figure = &stable.lines().prepared().figures()[0];
                let body_flow = typaxis_pagination::book_v2::prepare_book_v2_body_flow(
                    stable.lines(),
                    None,
                    stable.footnotes(),
                    &limits,
                    0,
                )
                .unwrap();
                let item = body_flow
                    .body_items()
                    .iter()
                    .find(|i| {
                        matches!(
                            i.source(),
                            Some(typaxis_pagination::ProductionBodyFragmentSource::Figure { .. })
                        )
                    })
                    .unwrap();
                assert_eq!(item.owner(), final_figure.owner());
                assert_eq!(item.width(), final_figure.width());
                assert_eq!(item.height(), final_figure.height().get());
                let figure_index = body_flow
                    .body_items()
                    .iter()
                    .position(|i| i.owner() == final_figure.owner())
                    .unwrap();
                assert!(matches!(
                    body_flow.body_items()[figure_index + 1].source(),
                    Some(typaxis_pagination::ProductionBodyFragmentSource::ParagraphLine { .. })
                ));

                assert!(std::ptr::eq(final_figure.source(), figure.source()));
                assert_eq!(final_figure.width(), figure.width());
                assert_eq!(final_figure.height(), figure.height());
                assert_eq!(final_figure.media(), figure.media());
                let measured=typaxis_pagination::book_v2::prepare_book_v2_table_measurements(body_flow,&limits).unwrap();
                let mut pages=typaxis_pagination::book_v2::prepare_book_v2_table_body_search(&measured,&limits,1_000_000,0).unwrap();
                let stable_pages=pages.select_stable_mixed_pages(2).unwrap();
                let placed=pages.place_mixed_pages(stable_pages.sequence()).unwrap();
                let closure=pages.close_mixed_page_sources(&stable_pages,&placed).unwrap();
                assert_eq!(closure.semantic_math(),0);
                assert_eq!(closure.repeated_math(),0);
                let terminals=pages.finalize_mixed_page_math(closure,&limits,0).unwrap();
                assert_math_terminals(&terminals);
                assert_math_display(&terminals,input.resources(),&limits);
                assert!(terminals.terminals().is_empty());
                let actual=placed.pages().iter().flat_map(|p|p.fragments()).find(|f|f.fragment().owner()==final_figure.owner()).unwrap().fragment();
                assert_eq!(actual.viewport().unwrap(),actual.bounds());
                assert_eq!(actual.bounds().width(),final_figure.width());
                assert_eq!(actual.bounds().height(),final_figure.height());
                assert!(actual.baseline().is_none());

            },
        )
        .unwrap();
    }
}
#[test]
fn book_v2_figures_reject_missing_raster_width_and_unrepresentable_svg_scale() {
    for (media, bytes, width) in [("png", PNG, None), ("svg-safe-2", SVG, Some(1))] {
        let root = Root::new();
        let limits = limits();
        let input = figure_input(&root, figure_data(media, bytes, width), bytes, &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let shaped =
            shape_book_v2_authored_text(&policy, &flow, input.resources(), &limits, EPOCH, None)
                .unwrap();
        let err = prepare_book_v2_text_inlines(
            &flow,
            &shaped,
            input.resources(),
            &limits,
            EPOCH,
            JapaneseLineBreakMode::Normal,
        )
        .err()
        .unwrap();
        assert_eq!(err.owner, NodeId::new(4));
        assert!(matches!(
            (width, err.kind),
            (None, E::MissingFigureWidth) | (Some(1), E::InvalidFigureGeometry)
        ));
    }
}

#[path = "book_v2_image_alias_tests.rs"]
mod image_aliases;
