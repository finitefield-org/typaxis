use super::*;
#[path = "book_v2_image_anchor_role_tests.rs"]
mod image_anchor_roles;

#[test]
fn book_v2_source_closure_binds_exact_stable_pages_and_all_demanded_leaves() {
    let mut forced = empty_tables_data();
    let span = forced["document"]["blocks"][0]["span"].clone();
    forced["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"page_break","node_id":0,"classes":[],"span":span}));
    for data in [
        empty_tables_data(),
        notes_table_data(),
        long_table_data(),
        forced,
    ] {
        with_measured_options(
            data,
            limits(),
            rect(500_000, 500_000, 10_000_000, 3_000_000),
            |measured, limits| {
                let run = |prior, work| -> Result<(u64, u64), ProductionBodyPaginationError> {
                    let mut search =
                        prepare_book_v2_table_body_search(measured, limits, work, prior)?;
                    let stable = search.select_stable_mixed_pages(2)?;
                    let placed = search.place_mixed_pages(stable.sequence())?;
                    let closure = search.close_mixed_page_sources(&stable, &placed)?;
                    assert!(std::ptr::eq(closure.geometry(), &placed));
                    assert!(std::ptr::eq(closure.stable(), &stable));
                    assert!(std::ptr::eq(closure.flow(), measured.flow()));
                    let demand = stable
                        .sequence()
                        .pages()
                        .last()
                        .unwrap()
                        .next_state()
                        .source_state()
                        .demand();
                    let mut expected = measured
                        .flow()
                        .body_items()
                        .iter()
                        .filter(|i| i.source().is_some())
                        .count();
                    let mut unreferenced = 0;
                    for index in 0..measured.flow().footnotes().definitions().len() {
                        match demand.status(index).unwrap() {
                            typaxis_pagination::ProductionFootnoteDemandStatus::Complete => {
                                expected += measured
                                    .flow()
                                    .definition_items(index)
                                    .unwrap()
                                    .iter()
                                    .filter(|i| i.source().is_some())
                                    .count();
                            }
                            typaxis_pagination::ProductionFootnoteDemandStatus::Unreferenced => {
                                unreferenced += 1
                            }
                            _ => panic!("pending demand in complete pages"),
                        }
                    }
                    assert_eq!(closure.semantic_fragments(), expected);
                    assert_eq!(closure.unreferenced_definitions(), unreferenced);
                    let total: usize = placed.pages().iter().map(|p| p.fragments().len()).sum();
                    assert_eq!(
                        closure.semantic_fragments() + closure.repeated_fragments(),
                        total
                    );
                    assert_eq!(closure.record_charge(), search.record_charge());
                    assert_eq!(closure.work_steps(), search.work_steps());
                    let terminals = search.finalize_mixed_page_math(closure, limits, 0)?;
                    assert_math_terminals(&terminals);
                    assert!(terminals.terminals().is_empty());
                    assert_eq!(terminals.canonical_bytes().len(), 264);
                    Ok((search.record_charge(), search.work_steps()))
                };
                let (records, work) = run(0, 10_000_000).unwrap();
                let prior =
                    limits.base().get().max_fragments - (records - measured.record_charge());
                assert_eq!(
                    run(prior, work).unwrap(),
                    (limits.base().get().max_fragments, work)
                );
                assert!(matches!(run(prior+1,work),Err(e) if e.kind==Error::FragmentLimit));
                assert!(matches!(run(0,work-1),Err(e) if e.kind==Error::FootnoteSearchLimit));

                let mut search =
                    prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
                let first = search.select_stable_mixed_pages(2).unwrap();
                let second = search.select_stable_mixed_pages(2).unwrap();
                let placed = search.place_mixed_pages(first.sequence()).unwrap();
                let before = (search.record_charge(), search.work_steps());
                assert!(
                    matches!(search.close_mixed_page_sources(&second, &placed), Err(e) if e.kind==Error::ReceiptMismatch)
                );
                assert_eq!(before, (search.record_charge(), search.work_steps()));
                let mut foreign =
                    prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
                assert!(
                    matches!(foreign.close_mixed_page_sources(&first, &placed), Err(e) if e.kind==Error::ReceiptMismatch)
                );
                let source = search.close_mixed_page_sources(&first, &placed).unwrap();
                let before = (
                    foreign.record_charge(),
                    foreign.work_steps(),
                    foreign.terminal_spool_charge(),
                );
                assert!(
                    matches!(foreign.finalize_mixed_page_math(source, limits, 0), Err(e) if e.kind==Error::ReceiptMismatch)
                );
                assert_eq!(
                    before,
                    (
                        foreign.record_charge(),
                        foreign.work_steps(),
                        foreign.terminal_spool_charge()
                    )
                );
                let source = search.close_mixed_page_sources(&first, &placed).unwrap();
                let mut base = limits.base().get().clone();
                base.max_spool_bytes -= 1;
                let different_limits = M4EffectiveResourceLimits::new(
                    ValidatedResourceLimits::new(base).unwrap(),
                    M4ResourceLimits::default(),
                )
                .unwrap();
                let before = (
                    search.record_charge(),
                    search.work_steps(),
                    search.terminal_spool_charge(),
                );
                assert!(
                    matches!(search.finalize_mixed_page_math(source, &different_limits, 0), Err(e) if e.kind==Error::ReceiptMismatch)
                );
                assert_eq!(
                    before,
                    (
                        search.record_charge(),
                        search.work_steps(),
                        search.terminal_spool_charge()
                    )
                );
            },
        );
    }
}

#[test]
fn book_v2_stable_pages_repeat_actual_empty_table_and_note_geometry_under_one_budget() {
    for data in [empty_tables_data(), notes_table_data(), long_table_data()] {
        with_measured_options(
            data,
            limits(),
            rect(500_000, 500_000, 10_000_000, 20_000_000),
            |measured, limits| {
                let run = |prior, work| -> Result<(u64, u64), ProductionBodyPaginationError> {
                    let mut search =
                        prepare_book_v2_table_body_search(measured, limits, work, prior)?;
                    let before = (search.record_charge(), search.work_steps());
                    assert!(
                        matches!(search.select_stable_mixed_pages(1),Err(e) if e.kind==Error::PagePassLimit)
                    );
                    assert_eq!(before, (search.record_charge(), search.work_steps()));
                    let stable = search.select_stable_mixed_pages(2)?;
                    assert_eq!(stable.passes(), 2);
                    assert_eq!(stable.record_charge(), search.record_charge());
                    assert_eq!(stable.work_steps(), search.work_steps());
                    assert!(stable.record_charge() > before.0);
                    assert!(stable.work_steps() > before.1);
                    search.verify_mixed_sequence(stable.sequence())?;
                    assert!(stable
                        .sequence()
                        .pages()
                        .last()
                        .unwrap()
                        .next_state()
                        .is_complete());
                    let placed = search.place_mixed_pages(stable.sequence())?;
                    let mut body = Vec::new();
                    let mut labels = Vec::new();
                    for page in placed.pages() {
                        assert_eq!(page.fragments().len(), page.cell_roles().len());
                        for (fragment, role) in page.fragments().iter().zip(page.cell_roles()) {
                            if fragment.definition_index().is_none()
                                && !role.is_some_and(|r| r.repeated_header())
                            {
                                body.push(fragment.item_index());
                            }
                        }
                        labels.extend(page.footnote_markers().iter().map(|m| m.definition_index()));
                    }
                    body.sort_unstable();
                    assert_eq!(
                        body,
                        (0..measured.flow().body_items().len()).collect::<Vec<_>>()
                    );
                    labels.sort_unstable();
                    let before_dedup = labels.len();
                    labels.dedup();
                    assert_eq!(before_dedup, labels.len());
                    let mut foreign =
                        prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0)?;
                    assert!(
                        matches!(foreign.place_mixed_pages(stable.sequence()),Err(e) if e.kind==Error::ReceiptMismatch)
                    );
                    Ok((search.record_charge(), search.work_steps()))
                };
                let (records, work) = run(0, 10_000_000).unwrap();
                let prior =
                    limits.base().get().max_fragments - (records - measured.record_charge());
                assert_eq!(
                    run(prior, work).unwrap(),
                    (limits.base().get().max_fragments, work)
                );
                assert!(matches!(run(prior+1,work),Err(e) if e.kind==Error::FragmentLimit));
                assert!(matches!(run(0,work-1),Err(e) if e.kind==Error::FootnoteSearchLimit));
            },
        );
    }
}

#[test]
fn book_v2_stable_pages_reject_declared_single_pass_even_with_large_external_allowance() {
    let mut base = ResourceLimits::default();
    base.max_layout_passes = 1;
    let limits = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(base).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    with_measured_options(
        empty_tables_data(),
        limits,
        rect(500_000, 500_000, 10_000_000, 20_000_000),
        |measured, limits| {
            let mut search =
                prepare_book_v2_table_body_search(measured, limits, 1_000_000, 0).unwrap();
            assert!(
                matches!(search.select_stable_mixed_pages(u16::MAX),Err(e) if e.kind==Error::PagePassLimit)
            );
        },
    );
}

#[test]
fn book_v2_text_display_preserves_generated_notes_and_repeated_table_text() {
    with_measured_resources_options(
        notes_table_data(),
        limits(),
        rect(500_000, 500_000, 10_000_000, 3_000_000),
        |measured, limits, admitted| {
            let mut search =
                prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
            let stable = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
            let terminals = search.finalize_mixed_page_math(closure, limits, 0).unwrap();
            assert_math_display(&terminals, admitted, limits);
            let mut builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                &terminals, admitted, limits, 10_000_000, 0, 0,
            )
            .unwrap();
            let display = builder.build_text().unwrap();
            assert!(display
                .draws()
                .iter()
                .any(|d| d.definition_index().is_some()));
            assert!(display
                .draws()
                .iter()
                .any(|d| d.repeated_header() && d.cell_owner().is_some()));
            assert!(display
                .draws()
                .iter()
                .any(|d| d.generated_provenance().is_some()));
            // A repeated reference displays the same issued text but cannot consume
            // a second semantic source or acquire a different generated namespace.
            for copy in display.draws().iter().filter(|d| d.repeated_header()) {
                let original = display
                    .draws()
                    .iter()
                    .find(|d| !d.repeated_header() && std::ptr::eq(d.cluster(), copy.cluster()))
                    .unwrap();
                assert_eq!(copy.exact_text(), original.exact_text());
                assert_eq!(copy.text_span(), original.text_span());
                assert_eq!(copy.generated_provenance(), original.generated_provenance());
            }
        },
    );
}

#[test]
fn book_v2_marker_display_keeps_multidigit_lists_and_definition_order_notes() {
    let mut data = framed_data();
    let span = data["document"]["footnotes"][0]["span"].clone();
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    let mut list = data["document"]["blocks"][0]["blocks"][1].clone();
    let ids = (0..12)
        .map(|i| format!("note-{:02}", 12 - i))
        .collect::<Vec<_>>();
    let items = ids
        .iter()
        .map(|id| {
            let mut p = paragraph.clone();
            p["children"].as_array_mut().unwrap().push(
                json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":id}),
            );
            json!({"node_id":0,"span":span,"blocks":[p]})
        })
        .collect::<Vec<_>>();
    list["items"] = items.into();
    data["document"]["blocks"][0]["blocks"] = json!([list.clone()]);
    list["start"] = 98.into();
    list["items"] = json!([{"node_id":0,"span":span,"blocks":[paragraph]}]);
    data["document"]["footnotes"]=ids.iter().enumerate().map(|(i,id)| {
        json!({"node_id":0,"span":span,"footnote_id":id,"blocks":[if i==0 {list.clone()} else {paragraph.clone()}]})
    }).collect::<Vec<_>>().into();
    with_measured_resources_options(
        data,
        limits(),
        rect(500_000, 500_000, 10_000_000, 3_000_000),
        |measured, limits, admitted| {
            let mut search =
                prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
            let stable = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
            let terminals = search.finalize_mixed_page_math(closure, limits, 0).unwrap();
            assert_math_display(&terminals, admitted, limits);
            let mut builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                &terminals, admitted, limits, 10_000_000, 0, 0,
            )
            .unwrap();
            let display = builder.build_markers().unwrap();
            use typaxis_display_list::book_v2::BookV2MarkerSource as S;
            assert_eq!(display.draws().len(), 25);
            let notes = display
                .draws()
                .iter()
                .filter_map(|d| match d.source() {
                    S::Footnote(m) => Some((m.definition_index(), m.utf8())),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                notes,
                (0..12)
                    .map(|i| (i, (i + 1).to_string()))
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|(i, s)| (*i, s.as_str()))
                    .collect::<Vec<_>>()
            );
            assert!(display
                .draws()
                .iter()
                .any(|d| matches!(d.source(),S::List(m) if m.utf8()=="12.")));
            let nested = display
                .draws()
                .iter()
                .find(|d| matches!(d.source(),S::List(m) if m.utf8()=="98."))
                .unwrap();
            assert_eq!(nested.fragment().definition_index(), Some(0));
            let index = display
                .draws()
                .iter()
                .position(|d| std::ptr::eq(d, nested))
                .unwrap();
            assert!(matches!(
                display.draws()[index - 1].source(),
                S::Footnote(_)
            ));
            assert_eq!(
                display.draws()[index - 1].fragment_index(),
                nested.fragment_index()
            );
            assert_eq!(
                display.separators().len(),
                placed
                    .pages()
                    .iter()
                    .filter(|p| p.separator_ink().is_some())
                    .count()
            );
            assert!(!display.separators().is_empty());
        },
    );
}

#[test]
fn book_v2_repeated_header_list_markers_keep_original_generated_provenance() {
    let mut data = notes_table_data();
    let paragraph =
        data["document"]["blocks"][0]["blocks"][0]["head"][0]["cells"][0]["blocks"][0].clone();
    let mut list = data["document"]["blocks"][0]["blocks"][1].clone();
    list["start"] = 10.into();
    list["items"][0]["blocks"] = json!([paragraph]);
    data["document"]["blocks"][0]["blocks"][0]["head"][0]["cells"][0]["blocks"] = json!([list]);
    with_measured_resources_options(
        data,
        limits(),
        rect(500_000, 500_000, 10_000_000, 3_000_000),
        |measured, limits, admitted| {
            let mut search =
                prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
            let stable = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
            let terminals = search.finalize_mixed_page_math(closure, limits, 0).unwrap();
            assert_math_display(&terminals, admitted, limits);
            let mut builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                &terminals, admitted, limits, 10_000_000, 0, 0,
            )
            .unwrap();
            let display = builder.build_markers().unwrap();
            let header = display
                .draws()
                .iter()
                .filter(|d| d.source().utf8() == "10.")
                .collect::<Vec<_>>();
            assert!(header.len() > 2);
            assert_eq!(
                header
                    .iter()
                    .filter(|d| !d.cell_role().unwrap().repeated_header())
                    .count(),
                1
            );
            for copy in &header[1..] {
                assert!(copy.cell_role().unwrap().repeated_header());
                assert_eq!(copy.source().provenance(), header[0].source().provenance());
                assert_eq!(
                    copy.source().fingerprint(),
                    header[0].source().fingerprint()
                );
            }
        },
    );
}

#[test]
fn book_v2_note_structure_keeps_shared_branches_dependency_cycles_and_unplaced_edges() {
    for cycle in [false, true] {
        let mut data = framed_data();
        // Two simultaneously demanded definitions need room in the authored
        // footnote region; increasing only the body rectangle cannot provide it.
        data["page_masters"]["masters"][0]["height"] = 36_000_000.into();
        data["page_masters"]["masters"][0]["trim"]["height"] = 36_000_000.into();
        data["page_masters"]["masters"][0]["footnote"]["height"] = 10_000_000.into();
        let span = data["document"]["footnotes"][0]["span"].clone();
        let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
        let reference = |id: &str| json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":id});
        let mut body = paragraph.clone();
        body["language"] = "fr".into();
        body["children"].as_array_mut().unwrap().push(if cycle { reference("a") } else {
            json!({"kind":"strong","node_id":0,"span":span,"children":[reference("a"),reference("b"),reference("a")]})
        });
        data["document"]["blocks"][0]["blocks"] = json!([body]);
        let mut a = paragraph.clone();
        let mut b = paragraph.clone();
        let mut unused = paragraph.clone();
        if cycle {
            a["children"].as_array_mut().unwrap().push(reference("b"));
            b["children"].as_array_mut().unwrap().push(reference("a"));
        }
        unused["children"]
            .as_array_mut()
            .unwrap()
            .push(reference("a"));
        // Definition order runs backwards relative to the first demanded note.
        data["document"]["footnotes"] = json!([
            {"node_id":0,"span":span,"footnote_id":"b","blocks":[b],"language":"en"},
            {"node_id":0,"span":span,"footnote_id":"a","blocks":[a],"language":"de"},
            {"node_id":0,"span":span,"footnote_id":"unused","blocks":[unused],"language":"ja"}
        ]);
        with_measured_resources_options(
            data,
            limits(),
            rect(500_000, 500_000, 10_000_000, 20_000_000),
            |measured, limits, admitted| {
                let mut search =
                    prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
                let stable = search.select_stable_mixed_pages(2).unwrap();
                let placed = search.place_mixed_pages(stable.sequence()).unwrap();
                let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
                assert_eq!(closure.unreferenced_definitions(), 1);
                let terminals = search.finalize_mixed_page_math(closure, limits, 0).unwrap();
                // Reaches the real marked body, source registry and relation checks.
                assert_math_display(&terminals, admitted, limits);
            },
        );
    }
}
