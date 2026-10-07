use super::*;

fn fixture(
    data: Value,
    text: &str,
    font: Option<&[u8]>,
    caps: &M4EffectiveResourceLimits,
    check: impl FnOnce(
        &BookV2ColumnTableMeasurements<'_, '_, '_, '_>,
        &M4EffectiveResourceLimits,
    ) -> Value,
) -> Value {
    let root = Root::new();
    let input = input(&root, data, text, font, caps);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), caps).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), caps).unwrap();
    let mut budget =
        BookV2BodyLineBudget::new(1_000_000, caps.base().get().max_line_reshape_passes);
    with_budgeted_book_v2_column_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        caps,
        MODE,
        None,
        &mut budget,
        &plan,
        None,
        |stable| {
            let flow = prepare_book_v2_column_flow_counted(
                &stable,
                None,
                caps,
                stable.retained_record_charge(),
                &mut 0,
            )
            .unwrap();
            let measured =
                prepare_book_v2_column_table_measurements_counted(flow, caps, &mut 0).unwrap();
            check(&measured, caps)
        },
    )
    .unwrap()
}
fn snapshot(sequence: &BookV2ColumnPageSequence<'_, '_, '_, '_, '_>) -> Value {
    json!(sequence.pages().iter().map(|page|json!({
        "page":page.page_index(),"name":page.named_page(),"forced":page.forced_break().map(|o|o.get()),
        "attempts":page.candidate_attempts(),"next":page.next_state().source_state().next_item(),
        "columns":page.candidate().columns().iter().map(|column|column.parts().iter().map(|part|json!({
            "items":part.items(),"table":part.table().map(|t|json!({"before":t.before().cell_progress_fingerprint(),
            "after":t.after().cell_progress_fingerprint(),"range":t.semantic_leaf_ranges().flatten().collect::<Vec<_>>() }))
        })).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "notes":page.candidate().footnotes().map(|n|n.fragments().iter().map(|f|f.fragment().definition_index()).collect::<Vec<_>>())
    })).collect::<Vec<_>>())
}
fn check_plain(font: Option<&[u8]>) {
    let text = if font.is_some() { "本文" } else { "Result" };
    let mut data = page_data(text, 100, 10, 300, 210, false, false);
    data["page_masters"]["masters"][0]["body"]["height"] = (24 * 65536).into();
    fixture(data, text, font, &limits(), |measured, caps| {
        let run = |prior, work| -> Result<_, typaxis_pagination::ProductionBodyPaginationError> {
            let mut search = prepare_book_v2_column_page_search_counted(
                measured, caps, work, prior, &mut 0, &mut 0,
            )?;
            let pages = search.select_pages()?;
            search.verify_sequence(&pages)?;
            assert_eq!(pages.pages().len(), 1);
            let page = &pages.pages()[0];
            assert_eq!(page.candidate().columns().len(), 2);
            assert_eq!(page.candidate().columns()[0].parts()[0].items(), Some(0..1));
            assert_eq!(page.candidate().columns()[1].parts()[0].items(), Some(1..2));
            assert_eq!(page.candidate().footnotes().unwrap().fragments().len(), 2);
            assert!(page.next_state().is_complete());
            let foreign = prepare_book_v2_column_page_search_counted(
                measured, caps, 1_000_000, prior, &mut 0, &mut 0,
            )
            .unwrap();
            assert!(matches!(foreign.verify_sequence(&pages),Err(e) if e.kind==E::ReceiptMismatch));
            Ok((
                snapshot(&pages),
                search.work_steps(),
                search.record_charge(),
            ))
        };
        let base = measured.record_charge();
        let full = run(base, 1_000_000).unwrap();
        assert_eq!(run(base, full.1).unwrap(), full);
        assert!(run(base, full.1 - 1).is_err());
        let exact = caps.base().get().max_fragments - (full.2 - base);
        let bounded = run(exact, full.1).unwrap();
        assert_eq!(bounded.0, full.0);
        assert_eq!(bounded.2, caps.base().get().max_fragments);
        assert!(run(exact + 1, 1_000_000).is_err());
        full.0
    });
}
#[test]
fn book_v2_column_automatic_pages_fill_two_columns_and_join_notes() {
    check_plain(None);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_column_automatic_pages_preserve_original_harano() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check_plain(Some(&font));
    check_tables(Some(&font));
}
#[test]
fn book_v2_column_automatic_pages_backtrack_body_for_one_note_region() {
    let result = fixture(
        page_data("Result", 24, 10, 300, 210, false, false),
        "Result",
        None,
        &limits(),
        |measured, caps| {
            let mut search = prepare_book_v2_column_page_search_counted(
                measured, caps, 1_000_000, 0, &mut 0, &mut 0,
            )
            .unwrap();
            let pages = search.select_pages().unwrap();
            search.verify_sequence(&pages).unwrap();
            assert_eq!(pages.pages().len(), 2);
            assert!(pages.pages()[0].candidate_attempts() >= 3);
            for (index, page) in pages.pages().iter().enumerate() {
                assert_eq!(
                    page.candidate().columns()[0].parts()[0].items(),
                    Some(index..index + 1)
                );
                assert!(page.candidate().columns()[1].parts().is_empty());
                let notes = page.candidate().footnotes().unwrap();
                assert_eq!(notes.fragments().len(), 1);
                assert_eq!(notes.fragments()[0].fragment().definition_index(), index);
            }
            snapshot(&pages)
        },
    );
    assert_eq!(result.as_array().unwrap().len(), 2);
}
fn breaks_data(text: &str) -> Value {
    let mut data = page_data(text, 100, 10, 300, 210, false, false);
    let mut para = data["document"]["blocks"][0].clone();
    para["children"]
        .as_array_mut()
        .unwrap()
        .retain(|child| child["kind"] != "footnote_reference");
    let command = json!({"kind":"page_break","node_id":0,"span":para["span"],"classes":[]});
    data["document"]["blocks"] = json!([command, command, para, command]);
    data["document"]["footnotes"] = json!([]);
    super::super::super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_column_automatic_pages_preserve_leading_consecutive_and_trailing_breaks() {
    fixture(
        breaks_data("Result"),
        "Result",
        None,
        &limits(),
        |measured, caps| {
            let mut search = prepare_book_v2_column_page_search_counted(
                measured, caps, 1_000_000, 0, &mut 0, &mut 0,
            )
            .unwrap();
            let pages = search.select_pages().unwrap();
            search.verify_sequence(&pages).unwrap();
            assert_eq!(pages.pages().len(), 4);
            assert_eq!(
                pages
                    .pages()
                    .iter()
                    .map(|p| p.next_state().source_state().next_item())
                    .collect::<Vec<_>>(),
                [1, 2, 4, 4]
            );
            for (index, page) in pages.pages().iter().enumerate() {
                assert_eq!(page.forced_break().is_some(), index < 3);
                assert!(page.candidate().columns()[1].parts().is_empty());
                assert_eq!(page.candidate().columns()[0].parts().is_empty(), index != 2);
            }
            snapshot(&pages)
        },
    );
}
fn check_tables(font: Option<&[u8]>) {
    let text = if font.is_some() { "表" } else { "Pro" };
    let mut data = super::super::super::table_width_frames::table_data(text, false);
    data["page_masters"]["masters"][0]["body"]["height"] = (64 * 65536).into();
    fixture(data, text, font, &limits(), |measured, caps| {
        let mut search = prepare_book_v2_column_page_search_counted(
            measured, caps, 1_000_000, 0, &mut 0, &mut 0,
        )
        .unwrap();
        let pages = search.select_pages().unwrap();
        search.verify_sequence(&pages).unwrap();
        let mut consumed = Vec::new();
        let mut same_page_continuation = false;
        let mut prior = None;
        for page in pages.pages() {
            for (column_index, column) in page.candidate().columns().iter().enumerate() {
                for part in column.parts() {
                    if let Some(range) = part.items() {
                        consumed.extend(range);
                    }
                    if let Some(table) = part.table() {
                        consumed.extend(table.semantic_leaf_ranges().flatten());
                        if !table.before().is_initial() {
                            if let Some((page_index, previous_column, after)) = prior {
                                same_page_continuation |= page_index == page.page_index()
                                    && previous_column < column_index;
                                assert_eq!(table.before().cell_progress_fingerprint(), after);
                            }
                        }
                        prior = Some((
                            page.page_index(),
                            column_index,
                            table.after().cell_progress_fingerprint(),
                        ));
                    }
                }
            }
        }
        consumed.sort_unstable();
        assert_eq!(
            consumed,
            (0..measured.body_items().len()).collect::<Vec<_>>()
        );
        assert!(same_page_continuation);
        assert!(pages.pages().len() > 1);
        snapshot(&pages)
    });
}
#[test]
fn book_v2_column_automatic_pages_keep_nested_table_cursors_between_columns() {
    check_tables(None);
}
#[test]
fn book_v2_column_automatic_pages_follow_named_masters_and_column_counts() {
    let mut data = page_data("Result", 100, 10, 300, 210, false, false);
    data["document"]["blocks"][0]["classes"] = json!(["column_named"]);
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"column-auto-name","selector":"paragraph.column_named","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"page","important":false,"value":{"kind":"string","value":"appendix"}}]}));
    let named = data["page_masters"]["masters"][0].clone();
    data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .push(named);
    let named = &mut data["page_masters"]["masters"][1];
    named["master_id"] = "z-auto-appendix".into();
    named["column_layout"] = json!({"count":3,"gap":10*65536,"fill":"sequential","balance":"none"});
    data["page_masters"]["selection_rules"] = json!([{"master_id":"z-auto-appendix","parity":"any","first":null,"named_page":"appendix","source_order":0}]);
    fixture(data, "Result", None, &limits(), |measured, caps| {
        let mut search = prepare_book_v2_column_page_search_counted(
            measured, caps, 1_000_000, 0, &mut 0, &mut 0,
        )
        .unwrap();
        let pages = search.select_pages().unwrap();
        assert_eq!(pages.pages().len(), 2);
        assert_eq!(pages.pages()[0].named_page(), Some("appendix"));
        assert_eq!(pages.pages()[0].candidate().columns().len(), 3);
        assert_eq!(pages.pages()[1].named_page(), None);
        assert_eq!(pages.pages()[1].candidate().columns().len(), 2);
        search.verify_sequence(&pages).unwrap();
        snapshot(&pages)
    });
}
#[test]
fn book_v2_column_automatic_pages_preserve_failed_limits_and_retry_history() {
    for kind in ["lookback", "reflows", "pages"] {
        let original = limits();
        let mut base = original.base().get().clone();
        let mut data = page_data(
            "Result",
            if kind == "reflows" { 24 } else { 100 },
            10,
            300,
            210,
            false,
            false,
        );
        match kind {
            "lookback" => base.max_page_break_lookback = 1,
            "reflows" => base.max_footnote_reflows_per_page = 1,
            "pages" => {
                base.max_pages = 1;
                data["page_masters"]["masters"][0]["body"]["height"] = (24 * 65536).into();
                let body = data["document"]["blocks"].as_array().unwrap().clone();
                data["document"]["blocks"] = json!([body[0], body[1], body[0], body[1]]);
                super::super::super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
            }
            _ => unreachable!(),
        }
        let caps = M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(base).unwrap(),
            original.extension().get().clone(),
        )
        .unwrap();
        fixture(data, "Result", None, &caps, |measured, caps| {
            let mut search = prepare_book_v2_column_page_search_counted(
                measured, caps, 1_000_000, 0, &mut 0, &mut 0,
            )
            .unwrap();
            for _ in 0..2 {
                let before = (search.work_steps(), search.record_charge());
                let error = match search.select_pages() {
                    Ok(_) => panic!("{kind} ignored"),
                    Err(e) => e,
                };
                match kind {
                    "lookback" => assert!(matches!(
                        error.kind,
                        E::PageBreakLookbackLimit { limit: 1, .. }
                    )),
                    "reflows" => assert_eq!(error.kind, E::FootnoteSearchLimit),
                    "pages" => assert_eq!(error.kind, E::PageLimit),
                    _ => unreachable!(),
                }
                assert!(search.work_steps() > before.0 && search.record_charge() > before.1);
            }
            json!(kind)
        });
    }
}

#[test]
fn book_v2_column_automatic_pages_continue_pending_notes_without_body() {
    fixture(
        page_data("Result", 60, 10, 300, 210, false, true),
        "Result",
        None,
        &limits(),
        |measured, caps| {
            let mut search = prepare_book_v2_column_page_search_counted(
                measured, caps, 1_000_000, 0, &mut 0, &mut 0,
            )
            .unwrap();
            let pages = search.select_pages().unwrap();
            search.verify_sequence(&pages).unwrap();
            assert!(pages.pages().len() > 1);
            for page in &pages.pages()[1..] {
                assert!(page
                    .candidate()
                    .columns()
                    .iter()
                    .all(|column| column.parts().is_empty()));
                assert!(!page.candidate().footnotes().unwrap().fragments().is_empty());
            }
            let last = pages.pages().last().unwrap().next_state().source_state();
            assert_eq!(last.definition_status(0), Some(S::Complete));
            assert_eq!(last.definition_status(1), Some(S::Complete));
            snapshot(&pages)
        },
    );
}
#[test]
fn book_v2_column_automatic_pages_stop_all_columns_at_table_caption_breaks() {
    for (headers, only_breaks, count) in [(false, false, 5), (true, false, 5), (false, true, 3)] {
        let data = super::super::super::table_caption_breaks::caption_breaks_with_text(
            headers,
            only_breaks,
            "Result",
        );
        let owners = data["document"]["blocks"][0]["caption"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| part["kind"] == "page_break")
            .map(|part| part["node_id"].as_u64().unwrap() as u32)
            .collect::<Vec<_>>();
        fixture(data, "Result", None, &limits(), |measured, caps| {
            let mut search = prepare_book_v2_column_page_search_counted(
                measured, caps, 1_000_000, 0, &mut 0, &mut 0,
            )
            .unwrap();
            let pages = search.select_pages().unwrap();
            search.verify_sequence(&pages).unwrap();
            assert_eq!(pages.pages().len(), count);
            assert_eq!(
                pages
                    .pages()
                    .iter()
                    .filter_map(|page| page.forced_break().map(|owner| owner.get()))
                    .collect::<Vec<_>>(),
                owners
            );
            let mut consumed = Vec::new();
            for page in pages.pages() {
                if page.forced_break().is_some() {
                    assert!(page.candidate().columns()[1].parts().is_empty());
                }
                for column in page.candidate().columns() {
                    for part in column.parts() {
                        if let Some(table) = part.table() {
                            consumed.extend(table.semantic_leaf_ranges().flatten());
                        }
                    }
                }
            }
            consumed.sort_unstable();
            assert_eq!(
                consumed,
                (0..measured.body_items().len()).collect::<Vec<_>>()
            );
            let mut manual = prepare_book_v2_column_page_search_counted(
                measured, caps, 1_000_000, 0, &mut 0, &mut 0,
            )
            .unwrap();
            let state = manual.begin().unwrap();
            let cursor = manual.begin_table(0).unwrap();
            let request = [BookV2BodyCandidatePart::Table {
                cursor,
                capacity: Length::from_raw(48 * 65536).unwrap(),
            }];
            let first = manual
                .evaluate_page(&state, None, &[&request, &[]])
                .unwrap()
                .unwrap();
            let next = first.next_state().table_continuation().unwrap();
            let after = [BookV2BodyCandidatePart::Table {
                cursor: next,
                capacity: Length::from_raw(48 * 65536).unwrap(),
            }];
            assert!(
                matches!(manual.evaluate_page(&state,None,&[&request,&after]),Err(e) if e.kind==E::ReceiptMismatch)
            );
            snapshot(&pages)
        });
    }
}
#[test]
fn book_v2_column_automatic_pages_reject_keep_across_authored_breaks() {
    let mut data = breaks_data("Result");
    data["document"]["blocks"][2]["classes"] = json!(["keep"]);
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"column-auto-keep","selector":"paragraph.keep","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    fixture(data, "Result", None, &limits(), |measured, caps| {
        let mut search = prepare_book_v2_column_page_search_counted(
            measured, caps, 1_000_000, 0, &mut 0, &mut 0,
        )
        .unwrap();
        for _ in 0..2 {
            let before = (search.work_steps(), search.record_charge());
            let error = match search.select_pages() {
                Ok(_) => panic!("keep crossed authored page break"),
                Err(e) => e,
            };
            assert_eq!(error.kind, E::KeepAcrossForcedBreak);
            assert_eq!(error.owner, measured.body_items()[2].owner());
            assert!(search.work_steps() > before.0 && search.record_charge() >= before.1);
        }
        json!("keep refused")
    });
}
