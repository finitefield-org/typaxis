use super::*;
use typaxis_core::{Length, PositiveLength, Rect};
use typaxis_display_list::book_v2::BookV2FontUseText;
use typaxis_layout::book_v2::{bind_book_v2_vectors, with_converged_book_v2_body_lines};
use typaxis_pagination::book_v2::{
    prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
    prepare_book_v2_table_measurements,
};
use typaxis_resources::book_v2::BookV2FontSelectionBuilder;

#[test]
fn book_v2_cids_preserve_real_ambiguous_ligature_and_multiple_substitution_text() {
    const FONT: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/vmb-book/cid-font/extraction.ttf"));
    const TEXT: &str = "ABA fi X D E f i";
    let root = Root::new();
    let limits = limits();
    let mut data = source_data(TEXT);
    data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(FONT)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 12_000_000.into();
    master["height"] = 12_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":12_000_000,"height":12_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":10_000_000,"height":10_000_000});
    let source = body_with_source(&root, data, TEXT.as_bytes(), &limits);
    fs::write(root.0.join("body.bin"), FONT).unwrap();
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
            // Generic resource/record/spool/work/original-source checks run
            // on this actual GSUB result, in addition to the targeted cases.
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
            let mut builder =
                BookV2FontSelectionBuilder::new(&display, &limits, 10_000_000, 0, 0, 0).unwrap();
            let selected = builder.build().unwrap();
            let closed = builder.prepare_font_closures(&selected).unwrap();
            let programs = builder.write_font_programs(&closed).unwrap();
            let plan = builder.plan_cids(&programs).unwrap();
            assert_eq!(plan.fonts().len(), 1);
            let text = |index: usize| match plan.uses()[index].source().usage().text() {
                BookV2FontUseText::Text(s) => s,
                BookV2FontUseText::Scalar(_) => panic!("authored text"),
            };
            let letters = (0..plan.uses().len())
                .filter(|&i| matches!(text(i), "A" | "B"))
                .collect::<Vec<_>>();
            assert_eq!(
                letters.iter().map(|&i| text(i)).collect::<Vec<_>>(),
                ["A", "B", "A"]
            );
            let shared = plan.cids(letters[0]).unwrap();
            for i in letters {
                assert_eq!(plan.cids(i).unwrap(), shared);
                assert!(plan.uses()[i].requires_actual_text());
                assert_eq!(
                    plan.uses()[i].actual_text(),
                    Some(BookV2FontUseText::Text(text(i)))
                );
            }
            assert!(plan.bindings(0).unwrap()[shared[0].get() as usize - 1]
                .unicode()
                .is_none());
            let ligature = (0..plan.uses().len()).find(|&i| text(i) == "fi").unwrap();
            assert_eq!(plan.cids(ligature).unwrap().len(), 1);
            assert_eq!(
                plan.uses()[ligature].actual_text(),
                Some(BookV2FontUseText::Text("fi"))
            );
            let multiple = (0..plan.uses().len()).find(|&i| text(i) == "X").unwrap();
            let cids = plan.cids(multiple).unwrap();
            assert_eq!(cids.len(), 2);
            assert_eq!(
                cids.iter()
                    .map(|c| plan.bindings(0).unwrap()[c.get() as usize - 1]
                        .unicode()
                        .unwrap())
                    .collect::<String>(),
                "DE"
            );
            assert_eq!(
                plan.uses()[multiple].actual_text(),
                Some(BookV2FontUseText::Text("X"))
            );
            let mut extracted = String::new();
            for (index, usage) in plan.uses().iter().enumerate() {
                if let Some(BookV2FontUseText::Text(s)) = usage.actual_text() {
                    extracted.push_str(s);
                } else {
                    for cid in plan.cids(index).unwrap() {
                        if let Some(c) = plan.bindings(usage.font_index()).unwrap()
                            [cid.get() as usize - 1]
                            .unicode()
                        {
                            extracted.push(c);
                        }
                    }
                }
            }
            assert_eq!(extracted, TEXT);
        },
    )
    .unwrap();
}
