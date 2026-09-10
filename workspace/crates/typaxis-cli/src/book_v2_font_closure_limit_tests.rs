use super::*;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_layout::book_v2::{bind_book_v2_vectors, with_converged_book_v2_body_lines};
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
    prepare_book_v2_table_measurements,
};
use typaxis_resources::book_v2::{BookV2FontClosureError, BookV2FontSelectionBuilder};

#[test]
fn book_v2_font_closure_checks_selected_cid_capacity_before_subset_encoding() {
    for cap in [1, 6] {
        let root = Root::new();
        let mut base = ResourceLimits::default();
        base.max_cids_per_font = cap;
        let limits = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(base).unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap();
        let mut data = source_data("Result");
        let master = &mut data["page_masters"]["masters"][0];
        master["width"] = 12_000_000.into();
        master["height"] = 12_000_000.into();
        master["trim"] = json!({"x":0,"y":0,"width":12_000_000,"height":12_000_000});
        master["body"] = json!({"x":500_000,"y":500_000,"width":10_000_000,"height":10_000_000});
        let input = prepared(&root, data, b"Result", &limits);
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
                let mut builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                    &terminals,
                    input.resources(),
                    &limits,
                    10_000_000,
                    0,
                    0,
                )
                .unwrap();
                let display = builder.build_body().unwrap();
                let mut builder =
                    BookV2FontSelectionBuilder::new(&display, &limits, 10_000_000, 0, 0, 0)
                        .unwrap();
                let selection = builder.build().unwrap();
                assert_eq!(selection.glyphs(0).unwrap().len(), 6);
                let closed = builder.prepare_font_closures(&selection);
                if cap == 1 {
                    assert!(matches!(
                        closed,
                        Err(BookV2FontClosureError::SelectedGlyphLimit)
                    ));
                } else {
                    // .notdef does not consume one of the six selected CIDs.
                    assert_eq!(closed.unwrap().fonts()[0].glyphs().len(), 7);
                }
            },
        )
        .unwrap();
    }
}
