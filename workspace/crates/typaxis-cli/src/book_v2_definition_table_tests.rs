use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

#[test]
fn book_v2_definition_tables_preserve_recursive_streams_and_budgets() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements, prepare_book_v2_table_search,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in [
        "natural",
        "forced",
        "headers",
        "caption",
        "deep",
        "span",
        "header-span",
    ] {
        let root = Root::new();
        let limits = limits();
        let mut data = if mode == "span" || mode == "header-span" {
            super::nested_spans::spanning(
                "LeftRight",
                4,
                if mode == "span" {
                    "later-break"
                } else {
                    "header-caption"
                },
            )
        } else {
            super::nested_tables::nested("LeftRight", 4, mode)
        };
        let table = data["document"]["blocks"][0].clone();
        let paragraph = super::nested_tables::nested("LeftRight", 4, "forced")["document"]
            ["blocks"][0]["body"][0]["cells"][1]["blocks"][0]
            .clone();
        data["document"]["blocks"] = json!([paragraph.clone(), paragraph.clone()]);
        data["document"]["footnotes"] = json!([
            {"node_id":0,"span":table["span"],"footnote_id":"before","blocks":[paragraph.clone(),paragraph.clone(),paragraph.clone()]},
            {"node_id":0,"span":table["span"],"footnote_id":"table","blocks":[table]}
        ]);
        let frame_width = data["page_masters"]["masters"][0]["body"]["width"]
            .as_i64()
            .unwrap();
        data["page_masters"]["masters"][0]["width"] = (frame_width + 20 * 65536).into();
        data["page_masters"]["masters"][0]["height"] = (320 * 65536).into();
        data["page_masters"]["masters"][0]["trim"]["height"] = (320 * 65536).into();
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":150*65536,"width":frame_width,"height":128*65536});
        super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
        let frame_height = data["page_masters"]["masters"][0]["body"]["height"]
            .as_i64()
            .unwrap();
        let input = prepared(&root, data, b"LeftRight", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings =
            typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits)
                .unwrap();
        let raw = |v| Length::from_raw(v).unwrap();
        let rect = Rect::new(
            raw(10 * 65536),
            raw(10 * 65536),
            PositiveLength::new(raw(frame_width)).unwrap(),
            PositiveLength::new(raw(frame_height)).unwrap(),
        );
        typaxis_layout::book_v2::with_converged_book_v2_body_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            rect,
            1_000_000,
            |stable| {
                let measured = prepare_book_v2_table_measurements(
                    prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                        .unwrap(),
                    &limits,
                )
                .unwrap();
                let replay =
                    |prior, work| -> Result<_, typaxis_pagination::ProductionBodyPaginationError> {
                        let mut search =
                            prepare_book_v2_table_search(&measured, 0, &limits, work, prior)?;
                        let mut cursor = search.begin()?;
                        let mut hashes = Vec::new();
                        let mut leaves = Vec::new();
                        while !cursor.is_terminal() {
                            assert!(hashes.len() < 10);
                            let selected =
                                search.evaluate(&cursor, search.maximum_height())?.unwrap();
                            assert_eq!(cursor.definition_index(), Some(1));
                            assert_eq!(selected.definition_index(), Some(1));
                            let ranges = selected
                                .source_leaf_ranges()
                                .collect::<Result<Vec<_>, _>>()?;
                            leaves.extend(ranges.into_iter().flatten());
                            for leaf in selected.source_placement_leaves() {
                                let leaf = leaf?;
                                assert_eq!(leaf.definition_index(), Some(1));
                                let item = &measured.flow().definition_items(1).unwrap()
                                    [leaf.item_index()];
                                assert!(item.source().is_some());
                                assert!(leaf.top() >= Length::ZERO);
                                assert!(
                                    leaf.top().checked_add(item.height()).unwrap()
                                        <= selected.used_height()
                                );
                            }
                            hashes.push(selected.fingerprint());
                            cursor = selected.after();
                        }
                        leaves.sort_unstable();
                        assert_eq!(
                            leaves,
                            (0..measured.flow().definition_items(1).unwrap().len())
                                .collect::<Vec<_>>()
                        );
                        assert!(!hashes.is_empty(), "{mode}");
                        Ok((search.record_charge(), search.work_charge(), hashes))
                    };
                let (records, work, hashes) = replay(0, 1_000_000).unwrap();
                let prior =
                    limits.base().get().max_fragments - (records - measured.record_charge());
                assert_eq!(
                    replay(prior, work).unwrap(),
                    (limits.base().get().max_fragments, work, hashes)
                );
                assert_eq!(replay(prior + 1, work).unwrap_err().kind, E::FragmentLimit);
                assert_eq!(replay(0, work - 1).unwrap_err().kind, E::TableSearchLimit);
                assert_eq!(
                    prepare_book_v2_table_body_search(&measured, &limits, 1_000_000, 0)
                        .unwrap()
                        .body_table_count(),
                    (0..measured.flow().table_count())
                        .filter(
                            |&index| measured.flow().table_source_definition(index) == Some(None)
                        )
                        .count()
                );
            },
        )
        .unwrap();
    }
}
