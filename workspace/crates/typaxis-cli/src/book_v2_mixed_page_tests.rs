use super::*;

#[test]
fn book_v2_pages_preserve_adjacent_empty_table_order_and_forced_blank_pages() {
    let mut data = empty_tables_data();
    let table = data["document"]["blocks"][0]["blocks"][1].clone();
    let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    let span = table["span"].clone();
    let forced = json!({"kind":"page_break","node_id":0,"classes":[],"span":span});
    data["document"]["blocks"][0]["blocks"] =
        json!([forced, table, table, forced, forced, table, paragraph, forced]);
    with_measured(data, |measured, limits| {
        let run = |prior, work| -> Result<(u64, u64), ProductionBodyPaginationError> {
            let mut search = prepare_book_v2_table_body_search(measured, limits, work, prior)?;
            let sequence = search.select_mixed_pages()?;
            search.verify_mixed_sequence(&sequence)?;
            let placed = search.place_mixed_pages(&sequence)?;
            assert!(std::ptr::eq(placed.sequence(), &sequence));
            assert_eq!(placed.pages().len(), 5);
            assert_eq!(
                placed
                    .pages()
                    .iter()
                    .map(|p| p.fragments().len())
                    .sum::<usize>(),
                1
            );
            assert!(placed
                .pages()
                .iter()
                .all(|p| p.footnote_markers().is_empty() && p.separator_ink().is_none()));

            assert_eq!(sequence.measurements_fingerprint(), measured.fingerprint());
            assert_eq!(sequence.pages().len(), 5);
            assert!(sequence.pages()[0].candidate().parts().is_empty());
            assert_eq!(sequence.pages()[1].candidate().parts().len(), 2);
            assert!(sequence.pages()[2].candidate().parts().is_empty());
            assert_eq!(sequence.pages()[3].candidate().parts().len(), 2);
            assert!(sequence.pages()[4].candidate().parts().is_empty());
            let mut tables = Vec::new();
            for (index, page) in sequence.pages().iter().enumerate() {
                assert_eq!(page.page_index() as usize, index);
                assert_eq!(page.forced_break().is_some(), index < 4);
                assert_eq!(page.candidate_attempts(), 1);
                tables.extend(
                    page.candidate()
                        .parts()
                        .iter()
                        .filter_map(|p| p.table())
                        .map(|t| t.before().table_index()),
                );
            }
            assert_eq!(tables, [0, 1, 2]);
            let last = sequence.pages().last().unwrap().next_state();
            assert_eq!(last.source_state().next_table_index(), 3);
            assert_eq!(last.source_state().next_item(), 5);
            assert!(last.is_complete());
            assert!(search.select_mixed_page(last)?.is_none());
            let foreign = prepare_book_v2_table_body_search(measured, limits, 1_000_000, 0)?;
            assert!(
                matches!(foreign.verify_mixed_sequence(&sequence),Err(e) if e.kind==Error::ReceiptMismatch)
            );
            Ok((search.record_charge(), search.work_steps()))
        };
        let (records, work) = run(0, 1_000_000).unwrap();
        let prior = limits.base().get().max_fragments - (records - measured.record_charge());
        assert_eq!(
            run(prior, work).unwrap(),
            (limits.base().get().max_fragments, work)
        );
        assert!(matches!(run(prior+1,work),Err(e) if e.kind==Error::FragmentLimit));
        assert!(matches!(run(0,work-1),Err(e) if e.kind==Error::FootnoteSearchLimit));
    });
}

fn long_table_data() -> Value {
    let mut data = table_data();
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    let table = &mut data["document"]["blocks"][0]["blocks"][0];
    let row = table["head"][0].clone();
    table["body"] = json!(vec![row; 12]);
    let table = table.clone();
    data["document"]["blocks"][0]["blocks"] = json!([paragraph, table, paragraph]);
    data
}
#[test]
fn book_v2_pages_split_actual_table_cells_and_keep_source_branches_immutable() {
    with_measured_options(
        long_table_data(),
        limits(),
        rect(500_000, 500_000, 10_000_000, 3_000_000),
        |measured, limits| {
            let mut search =
                prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
            let initial = search.begin_mixed_pages().unwrap();
            let first = search.select_mixed_page(&initial).unwrap().unwrap();
            let repeat = search.select_mixed_page(&initial).unwrap().unwrap();
            assert_eq!(
                first.candidate().used_height(),
                repeat.candidate().used_height()
            );
            assert_eq!(
                first.next_state().source_state().next_item(),
                repeat.next_state().source_state().next_item()
            );
            assert_eq!(initial.source_state().next_item(), 0);
            first.candidate().verify(initial.source_state()).unwrap();
            assert!(
                matches!(first.candidate().verify(repeat.next_state().source_state()),Err(e) if e.kind==Error::ReceiptMismatch)
            );
            let mut other =
                prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
            assert!(
                matches!(other.select_mixed_page(&initial),Err(e) if e.kind==Error::ReceiptMismatch)
            );
            let sequence = search.select_mixed_pages().unwrap();
            assert!(sequence.pages().len() > 2);
            search.verify_mixed_sequence(&sequence).unwrap();
            let mut consumed = Vec::new();
            let mut fragments = 0;
            for page in sequence.pages() {
                assert!(page.candidate().used_height().raw() <= 3_000_000);
                for part in page.candidate().parts() {
                    if let Some(range) = part.items() {
                        consumed.extend(range);
                    }
                    if let Some(table) = part.table() {
                        assert_eq!(table.repeats_header(), fragments > 0);
                        assert_eq!(table.before().table_index(), 0);
                        consumed.extend(table.semantic_leaf_ranges().flatten());
                        fragments += 1;
                    }
                }
            }
            assert!(fragments > 1);
            let placed = search.place_mixed_pages(&sequence).unwrap();
            let mut repeated = 0;
            let mut actual = Vec::new();
            for page in placed.pages() {
                assert_eq!(page.fragments().len(), page.cell_roles().len());
                for (placed, role) in page.fragments().iter().zip(page.cell_roles()) {
                    assert_eq!(placed.definition_index(), None);
                    let fragment = placed.fragment();
                    assert_eq!(fragment.page_index(), page.selection().page_index());
                    assert_eq!(
                        fragment.owner(),
                        measured.flow().body_items()[placed.item_index()].owner()
                    );
                    assert!(fragment.bounds().y().raw() >= 500_000);
                    assert!(
                        fragment.bounds().y().raw() + fragment.bounds().height().get().raw()
                            <= 3_500_000
                    );
                    assert!(fragment.baseline().is_some());
                    if role.is_some_and(|r| r.repeated_header()) {
                        repeated += 1;
                    } else {
                        actual.push(placed.item_index());
                    }
                }
            }
            assert_eq!(repeated, 2 * (fragments - 1));
            actual.sort_unstable();
            assert_eq!(
                actual,
                (0..measured.flow().body_items().len()).collect::<Vec<_>>()
            );
            assert!(
                matches!(other.place_mixed_pages(&sequence),Err(e) if e.kind==Error::ReceiptMismatch)
            );

            consumed.sort_unstable();
            assert_eq!(
                consumed,
                (0..measured.flow().body_items().len()).collect::<Vec<_>>()
            );
            assert!(sequence.pages().last().unwrap().next_state().is_complete());
        },
    );
}

fn notes_table_data() -> Value {
    let mut data = table_data();
    let span = data["document"]["footnotes"][0]["span"].clone();
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    data["document"]["footnotes"] = json!([
        {"node_id":0,"span":span,"footnote_id":"note","blocks":vec![paragraph.clone();8]},
        {"node_id":0,"span":span,"footnote_id":"later","blocks":[paragraph]}
    ]);
    let table = &mut data["document"]["blocks"][0]["blocks"][0];
    table["head"][0]["cells"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"note"}));
    table["body"][2]["cells"][0]["blocks"][0]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"later"}));
    data
}
#[test]
fn book_v2_pages_retry_footnote_fit_and_drain_notes_without_repeating_semantic_headers() {
    with_measured(notes_table_data(), |measured, limits| {
        let mut search =
            prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
        let sequence = search.select_mixed_pages().unwrap();
        search.verify_mixed_sequence(&sequence).unwrap();
        assert!(sequence.pages().len() > 3);
        assert!(sequence.pages()[0].candidate_attempts() > 1);
        let mut labels = [0, 0];
        let mut references = Vec::new();
        let mut note_only = 0;
        for page in sequence.pages() {
            let candidate = page.candidate();
            if candidate.parts().is_empty() && candidate.footnotes().is_some() {
                note_only += 1;
            }
            for part in candidate.parts() {
                if let Some(table) = part.table() {
                    for range in table.semantic_leaf_ranges() {
                        references.extend(
                            measured
                                .flow()
                                .references_in_items(None, range)
                                .iter()
                                .map(|r| r.source().owner()),
                        );
                    }
                }
            }
            if let Some(region) = candidate.footnotes() {
                for fragment in region.fragments() {
                    labels[fragment.fragment().definition_index()] +=
                        usize::from(fragment.fragment().marker().is_some());
                }
            }
        }
        assert!(note_only > 0);
        assert_eq!(labels, [1, 1]);
        assert_eq!(references.len(), 2);
        assert_ne!(references[0], references[1]);
        let placed = search.place_mixed_pages(&sequence).unwrap();
        let mut placed_labels = [0, 0];
        let mut list_labels = 0;
        for page in placed.pages() {
            list_labels += page.list_markers().len();
            for marker in page.footnote_markers() {
                placed_labels[marker.definition_index()] += 1;
                let fragment = &page.fragments()[marker.fragment_index() as usize];
                assert_eq!(fragment.definition_index(), Some(marker.definition_index()));
                assert_eq!(
                    marker.baseline(),
                    fragment
                        .fragment()
                        .bounds()
                        .y()
                        .checked_add(
                            measured
                                .flow()
                                .definition_marker(marker.definition_index())
                                .unwrap()
                                .baseline()
                        )
                        .unwrap()
                );
                assert!(page.separator_ink().is_some());
            }
        }
        assert_eq!(placed_labels, [1, 1]);
        assert_eq!(list_labels, measured.flow().list_marker_count());
    });
}

#[test]
fn book_v2_pages_enforce_page_reflow_and_candidate_limits_before_issuing_sequence() {
    for kind in 0..3 {
        let mut base = ResourceLimits::default();
        match kind {
            0 => base.max_pages = 1,
            1 => base.max_footnote_reflows_per_page = 1,
            _ => base.max_page_break_lookback = 1,
        }
        let limited = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(base).unwrap(),
            M4ResourceLimits::default(),
        )
        .unwrap();
        let data = if kind == 1 {
            notes_table_data()
        } else {
            long_table_data()
        };
        let height = if kind == 0 { 3_000_000 } else { 20_000_000 };
        with_measured_options(
            data,
            limited,
            rect(500_000, 500_000, 10_000_000, height),
            |measured, limits| {
                let mut search =
                    prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
                let err = search
                    .select_mixed_pages()
                    .err()
                    .unwrap_or_else(|| panic!("declared search bound {kind} must stop selection"));
                match kind {
                    0 => assert_eq!(err.kind, Error::PageLimit),
                    1 => assert_eq!(err.kind, Error::FootnoteSearchLimit),
                    _ => assert!(matches!(
                        err.kind,
                        Error::PageBreakLookbackLimit { limit: 1, .. }
                    )),
                }
            },
        );
    }
}

#[test]
fn book_v2_pages_apply_keeps_to_the_immediately_preceding_source_occurrence() {
    let mut data = empty_tables_data();
    let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    let table = data["document"]["blocks"][0]["blocks"][1].clone();
    data["document"]["blocks"][0]["blocks"] = json!([paragraph, table, table, paragraph]);
    data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"table-keep","selector":"table","source_order":4,"extends":null,"declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    with_measured_options(
        data.clone(),
        limits(),
        rect(500_000, 500_000, 10_000_000, 1_500_000),
        |measured, limits| {
            let mut search =
                prepare_book_v2_table_body_search(measured, limits, 1_000_000, 0).unwrap();
            let sequence = search.select_mixed_pages().unwrap();
            assert_eq!(sequence.pages().len(), 2);
            assert_eq!(
                sequence.pages()[0].next_state().source_state().next_item(),
                1
            );
            assert_eq!(
                sequence.pages()[0]
                    .next_state()
                    .source_state()
                    .next_table_index(),
                0
            );
            let second = sequence.pages()[1].candidate();
            assert_eq!(second.parts().len(), 3);
            assert_eq!(second.parts()[0].table().unwrap().before().table_index(), 0);
            assert_eq!(second.parts()[1].table().unwrap().before().table_index(), 1);
            assert_eq!(second.parts()[2].items(), Some(1..2));
        },
    );
    let span = data["document"]["blocks"][0]["span"].clone();
    data["document"]["blocks"][0]["blocks"][3] =
        json!({"kind":"page_break","node_id":0,"classes":[],"span":span});
    with_measured(data, |measured, limits| {
        let mut search = prepare_book_v2_table_body_search(measured, limits, 1_000_000, 0).unwrap();
        assert!(
            matches!(search.begin_mixed_pages(),Err(e) if e.kind==Error::KeepAcrossForcedBreak)
        );
    });
}

#[path="book_v2_stability_tests.rs"]
mod stability;
