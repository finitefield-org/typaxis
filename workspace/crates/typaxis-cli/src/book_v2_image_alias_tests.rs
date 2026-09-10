use super::*;
#[test]
fn book_v2_image_resources_share_logical_aliases_without_merging_occurrences() {
    for (media, bytes) in [
        ("png", PNG),
        ("jpeg-baseline", JPEG),
        ("svg-safe-2", SVG),
        ("svg-safe-1", MATRIX),
        ("svg-safe-2", MATRIX),
        ("svg-safe-2", FRACTION),
        ("svg-safe-2", CURVES),
    ] {
        let root = Root::new();
        let limits = limits();
        let mut data = figure_data(media, bytes, Some(2_097_152));
        let mut alias = data["resources"]["images"][0].clone();
        alias["image_id"] = 1.into();
        data["resources"]["images"]
            .as_array_mut()
            .unwrap()
            .push(alias);
        let blocks = data["document"]["blocks"][0]["blocks"]
            .as_array_mut()
            .unwrap();
        blocks[1]["image_id"] = 1.into();
        let mut second = blocks[1].clone();
        second["node_id"] = 7.into();
        second["image_id"] = 0.into();
        second["caption"][0]["node_id"] = 8.into();
        second["caption"][0]["children"][0]["node_id"] = 9.into();
        blocks.push(second);
        let input = figure_input(&root, data, bytes, &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        let size = PositiveLength::new(Length::from_raw(10_000_000).unwrap()).unwrap();
        let body = Rect::new(Length::ZERO, Length::ZERO, size, size);
        with_converged_book_v2_body_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            body,
            10_000_000,
            |stable| {
                let body = typaxis_pagination::book_v2::prepare_book_v2_body_flow(
                    stable.lines(),
                    None,
                    stable.footnotes(),
                    &limits,
                    0,
                )
                .unwrap();
                let measured =
                    typaxis_pagination::book_v2::prepare_book_v2_table_measurements(body, &limits)
                        .unwrap();
                let mut pages = typaxis_pagination::book_v2::prepare_book_v2_table_body_search(
                    &measured, &limits, 10_000_000, 0,
                )
                .unwrap();
                let stable = pages.select_stable_mixed_pages(2).unwrap();
                let placed = pages.place_mixed_pages(stable.sequence()).unwrap();
                let closed = pages.close_mixed_page_sources(&stable, &placed).unwrap();
                let terminal = pages.finalize_mixed_page_math(closed, &limits, 0).unwrap();
                assert_math_display(&terminal, input.resources(), &limits);
                let mut display_builder =
                    typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                        &terminal,
                        input.resources(),
                        &limits,
                        1_000_000_000,
                        0,
                        0,
                    )
                    .unwrap();
                let display = display_builder.build_body().unwrap();
                let mut builder = typaxis_resources::book_v2::BookV2FontSelectionBuilder::new(
                    &display,
                    &limits,
                    1_000_000_000,
                    0,
                    0,
                    0,
                )
                .unwrap();
                let selection = builder.select_images().unwrap();
                assert_eq!(selection.uses().len(), 2);
                assert_eq!(selection.images().len(), 1);
                assert_eq!(selection.images()[0].image().image_id().get(), 0);
                assert_eq!(
                    selection
                        .uses()
                        .iter()
                        .map(|u| u.usage().image().image_id().get())
                        .collect::<Vec<_>>(),
                    vec![1, 0]
                );
                assert!(selection.uses().iter().all(|u| u.resource_index() == 0));
                assert_ne!(
                    selection.uses()[0].usage().owner(),
                    selection.uses()[1].usage().owner()
                );
                let programs = builder.write_raster_programs(&selection).unwrap();
                assert_eq!(
                    programs.program(0).is_some(),
                    !media.starts_with("svg-safe-")
                );
                assert!(programs.program(1).is_none());
            },
        )
        .unwrap();
    }
}

const MATRIX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/matrix.svg"
));
const FRACTION: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg"));
const CURVES: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="48pt" height="32pt" viewBox="-2 -1 24 16"><defs><clipPath id="c"><path d="M 0 0 L 19 0 L 19 12 L 0 12 Z"/></clipPath></defs><g clip-path="url(#c)" transform="translate(1 1)"><path d="M 1 3 Q 5 0 8 4 C 10 12 12 1 16 7 L 2 10 Z" fill="#4080c0" fill-opacity="0.5" stroke="#c08040" stroke-opacity="0.75" stroke-width="0.75" fill-rule="evenodd"/><path d="M 3 4 Q 8 1 14 8" fill="none" stroke="currentColor" stroke-width="0.5"/></g></svg>"##;
