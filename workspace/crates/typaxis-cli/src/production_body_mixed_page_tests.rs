#[test]
fn production_mixed_pages_choose_real_table_cuts_and_keep_body_and_notes_contiguous() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
    };
    let value = production_mixed_table_fixture();
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let end = production_measured_table_end(&measured.tables()[0]);
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let sequence = search.select_mixed_pages().unwrap();
        search.verify_mixed_sequence(&sequence).unwrap();
        assert_eq!(sequence.pages().len(), 2);
        let first = &sequence.pages()[0];
        let second = &sequence.pages()[1];
        assert_eq!(first.page_index(), 0);
        assert_eq!(first.candidate().parts()[0].items(), Some(0..1));
        assert_eq!(
            first.candidate().parts()[1].table().unwrap().cells()[0].content_range(),
            0..3
        );
        assert_eq!(first.candidate().used_height().raw(), 5_000_000);
        assert_eq!(first.next_state().next_item(), 1);
        assert_eq!(
            first
                .next_state()
                .table_continuation()
                .unwrap()
                .offset()
                .raw(),
            4_000_000
        );
        assert_eq!(second.page_index(), 1);
        assert!(second.candidate().parts()[0]
            .table()
            .unwrap()
            .repeats_header());
        assert_eq!(
            second.candidate().parts()[0].table().unwrap().cells()[0].content_range(),
            3..6
        );
        assert_eq!(second.candidate().parts()[1].items(), Some(end..end + 1));
        assert!(second.next_state().is_complete());
        assert_eq!(
            first
                .candidate()
                .footnotes()
                .unwrap()
                .fragments()
                .iter()
                .map(|f| f.fragment().definition_index())
                .collect::<Vec<_>>(),
            [3, 0, 1]
        );
        assert_eq!(
            second
                .candidate()
                .footnotes()
                .unwrap()
                .fragments()
                .iter()
                .map(|f| f.fragment().definition_index())
                .collect::<Vec<_>>(),
            [2]
        );
        let begin = search.begin_mixed_pages().unwrap();
        let retry = search.select_mixed_page(&begin).unwrap().unwrap();
        assert_eq!(
            retry.candidate().parts()[1].table().unwrap().fingerprint(),
            first.candidate().parts()[1].table().unwrap().fingerprint()
        );
        let mut other = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        assert!(other.verify_mixed_sequence(&sequence).is_err());
        assert!(other.select_mixed_page(&begin).is_err());
    });
}
#[test]
fn production_mixed_pages_keep_incoming_footnote_only_pages_and_final_note_continuations() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
    };
    for definition in [2, 3] {
        let mut value = production_mixed_table_fixture();
        let paragraph = value["document"]["footnotes"][definition]["blocks"][0].clone();
        value["document"]["footnotes"][definition]["blocks"] =
            serde_json::json!(vec![paragraph; 12]);
        production_body_renumber(&mut value["document"], &mut 0);
        with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured =
                prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
            let mut search =
                prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
            let sequence = search.select_mixed_pages().unwrap();
            assert!(sequence.pages().len() > 2);
            assert!(sequence.pages().last().unwrap().next_state().is_complete());
            let count = sequence
                .pages()
                .iter()
                .filter_map(|p| p.candidate().footnotes())
                .flat_map(|r| r.fragments())
                .filter(|f| f.fragment().definition_index() == definition)
                .map(|f| f.fragment().items().len())
                .sum::<usize>();
            assert_eq!(count, 12);
            let markers = sequence
                .pages()
                .iter()
                .filter_map(|p| p.candidate().footnotes())
                .flat_map(|r| r.fragments())
                .filter(|f| {
                    f.fragment().definition_index() == definition && f.fragment().marker().is_some()
                })
                .count();
            assert_eq!(markers, 1);
        });
    }
}
#[test]
fn production_mixed_pages_keep_forced_blank_pages_and_source_table_owners() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
    };
    let mut value = production_table_break_fixture(false);
    let table = value["document"]["blocks"][0]["blocks"][0].clone();
    let forced = serde_json::json!({"kind":"page_break","node_id":0,"classes":[],"span":table["head"][0]["span"]});
    value["document"]["blocks"][0]["blocks"] = serde_json::json!([forced, table, forced, forced]);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let sequence = search.select_mixed_pages().unwrap();
        assert_eq!(sequence.pages().len(), 4);
        assert!(sequence.pages()[0].candidate().parts().is_empty());
        assert!(sequence.pages()[0].forced_break().is_some());
        assert_eq!(sequence.pages()[1].candidate().parts().len(), 1);
        assert!(sequence.pages()[1].candidate().parts()[0]
            .table()
            .unwrap()
            .after()
            .is_terminal());
        assert!(sequence.pages()[2].candidate().parts().is_empty());
        assert!(sequence.pages()[2].forced_break().is_some());
        assert!(sequence.pages()[3].candidate().parts().is_empty());
        assert!(sequence.pages()[3].next_state().is_complete());
    });
}
#[test]
fn production_mixed_pages_reject_exhausted_records_work_and_page_limits() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyPaginationErrorKind as E,
    };
    let value = production_mixed_table_fixture();
    let mut required = 0;
    for delta in [None, Some(0), Some(1)] {
        let cfg = delta.map_or_else(config, |d| {
            config_with_limits(ResourceLimits {
                max_fragments: required - d,
                ..ResourceLimits::default()
            })
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured =
                prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
            let run = |work| -> Result<_, typaxis_pagination::ProductionBodyPaginationError> {
                let mut search = prepare_production_table_body_search(&measured, limits, work)?;
                let sequence = search.select_mixed_pages()?;
                search.verify_mixed_sequence(&sequence)?;
                Ok((search.record_charge(), search.work_steps()))
            };
            let result = run(100_000);
            if delta == Some(1) {
                assert_eq!(result.unwrap_err().kind, E::FragmentLimit);
                return;
            }
            let (records, work) = result.unwrap();
            required = records;
            assert_eq!(run(work).unwrap(), (records, work));
            assert!(matches!(
                run(work - 1).unwrap_err().kind,
                E::TableSearchLimit | E::FootnoteSearchLimit
            ));
        });
    }
    let cfg = config_with_limits(ResourceLimits {
        max_pages: 1,
        ..ResourceLimits::default()
    });
    with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        assert_eq!(
            search.select_mixed_pages().err().unwrap().kind,
            E::PageLimit
        );
    });
}
#[test]
fn production_mixed_pages_do_not_accept_an_early_fit_when_alternative_limit_exhausts() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyPaginationErrorKind as E,
    };
    let value = production_mixed_table_fixture();
    let cfg = config_with_limits(ResourceLimits {
        max_footnote_reflows_per_page: 1,
        ..ResourceLimits::default()
    });
    with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let state = search.begin().unwrap();
        assert!(search
            .evaluate_body_candidate(&state, 0..1)
            .unwrap()
            .is_some());
        assert_eq!(
            search.select_mixed_pages().err().unwrap().kind,
            E::FootnoteSearchLimit
        );
    });
}

#[test]
fn production_mixed_pages_place_parallel_cells_repeated_headers_and_real_note_markers() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
    };
    let value = production_mixed_table_fixture();
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let sequence = search.select_mixed_pages().unwrap();
        let placed = search.place_mixed_pages(&sequence).unwrap();
        assert!(std::ptr::eq(placed.sequence(), &sequence));
        assert_eq!(
            placed
                .pages()
                .iter()
                .map(|p| p.fragments().len())
                .collect::<Vec<_>>(),
            [11, 9]
        );
        let mut semantic = std::collections::BTreeSet::new();
        let mut repeats = 0;
        let mut markers = 0;
        for (page_index, page) in placed.pages().iter().enumerate() {
            assert_eq!(page.fragments().len(), page.cell_roles().len());
            assert!(page.separator_ink().is_some());
            markers += page.footnote_markers().len();
            for (fragment, role) in page.fragments().iter().zip(page.cell_roles()) {
                assert_eq!(fragment.fragment().page_index(), page_index as u32);
                let key = (fragment.definition_index(), fragment.item_index());
                if role.is_some_and(|r| r.repeated_header()) {
                    repeats += 1;
                    assert_eq!(page_index, 1);
                    assert!(semantic.contains(&key));
                    assert_eq!(
                        fragment.fragment().bounds().y(),
                        blocks.page_geometry().body().y()
                    );
                } else {
                    assert!(semantic.insert(key));
                }
                if fragment.definition_index().is_some() {
                    assert!(fragment.fragment().bounds().y() >= page.separator_ink().unwrap().y());
                    assert!(role.is_none());
                }
            }
        }
        assert_eq!(semantic.len(), 18);
        assert_eq!(repeats, 2);
        assert_eq!(markers, 4);
        let table = &measured.tables()[0];
        let first = &placed.pages()[0];
        let first_in_cell = |owner| {
            first
                .fragments()
                .iter()
                .zip(first.cell_roles())
                .find(|(_, r)| r.is_some_and(|r| r.owner() == owner))
                .unwrap()
                .0
                .fragment()
        };
        let left = first_in_cell(table.cells()[2].owner());
        let right = first_in_cell(table.cells()[3].owner());
        assert_eq!(left.bounds().y(), right.bounds().y());
        assert!(left.bounds().x() < right.bounds().x());
        assert_eq!(
            left.bounds().y().raw(),
            blocks.page_geometry().body().y().raw() + 2_000_000
        );
        let retry = search.place_mixed_pages(&sequence).unwrap();
        for (a, b) in placed.pages().iter().zip(retry.pages()) {
            assert_eq!(a.fragments(), b.fragments());
            assert_eq!(a.cell_roles(), b.cell_roles());
            assert_eq!(a.footnote_markers(), b.footnote_markers());
        }
        let mut other = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        assert!(other.place_mixed_pages(&sequence).is_err());
    });
}
#[test]
fn production_mixed_pages_place_real_formula_viewports_and_cell_list_markers() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyFragmentSource as Source,
    };
    let mut value = production_table_fixture();
    let cell = &mut value["document"]["blocks"][0]["blocks"][0]["body"][2]["cells"][0];
    let paragraph = cell["blocks"][0].clone();
    let span = cell["span"].clone();
    cell["blocks"] = serde_json::json!([{"kind":"list","node_id":0,"span":span,"classes":[],"ordered":true,"start":1,"items":[{"node_id":0,"span":span,"blocks":[paragraph]}]}]);
    production_body_renumber(&mut value["document"], &mut 0);
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let sequence = search.select_mixed_pages().unwrap();
        let placed = search.place_mixed_pages(&sequence).unwrap();
        let mut formulas = 0;
        let mut lists = 0;
        for page in placed.pages() {
            lists += page.list_markers().len();
            for fragment in page.fragments() {
                if let Source::VectorBlock { block_index } = fragment.fragment().source() {
                    formulas += 1;
                    let block = &blocks.blocks()[block_index as usize];
                    let viewport = fragment.fragment().viewport().unwrap();
                    assert_eq!(viewport.width(), block.viewport_width());
                    assert_eq!(viewport.height(), block.viewport_height());
                    assert_eq!(
                        viewport.y().raw(),
                        fragment.fragment().bounds().y().raw()
                            + block.viewport_top_offset().get().raw()
                    );
                    assert_eq!(fragment.fragment().owner(), block.owner());
                }
            }
            for marker in page.list_markers() {
                assert!(page.cell_roles()[marker.fragment_index() as usize].is_some());
            }
        }
        assert_eq!(formulas, 1);
        assert_eq!(lists, 1);
    });
}
#[test]
fn production_mixed_pages_placement_keeps_cumulative_budget_after_selection() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyPaginationErrorKind as E,
    };
    let value = production_mixed_table_fixture();
    let mut required = 0;
    for delta in [None, Some(0), Some(1)] {
        let cfg = delta.map_or_else(config, |d| {
            config_with_limits(ResourceLimits {
                max_fragments: required - d,
                ..ResourceLimits::default()
            })
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured =
                prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
            let run = |work| -> Result<_, typaxis_pagination::ProductionBodyPaginationError> {
                let mut search = prepare_production_table_body_search(&measured, limits, work)?;
                let sequence = search.select_mixed_pages()?;
                let placed = search.place_mixed_pages(&sequence)?;
                assert_eq!(placed.pages().len(), 2);
                Ok((search.record_charge(), search.work_steps()))
            };
            let result = run(100_000);
            if delta == Some(1) {
                assert_eq!(result.unwrap_err().kind, E::FragmentLimit);
                return;
            }
            let (records, work) = result.unwrap();
            required = records;
            assert_eq!(run(work).unwrap(), (records, work));
            assert!(matches!(
                run(work - 1).unwrap_err().kind,
                E::TableSearchLimit | E::FootnoteSearchLimit
            ));
        });
    }
}

#[test]
fn production_mixed_pages_stabilize_actual_cell_geometry_with_one_cumulative_budget() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyPaginationErrorKind as E,
    };
    let value = production_mixed_table_fixture();
    let mut required = 0;
    for delta in [None, Some(0), Some(1)] {
        let cfg = delta.map_or_else(config, |d| {
            config_with_limits(ResourceLimits {
                max_fragments: required - d,
                ..ResourceLimits::default()
            })
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured =
                prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
            let run = |work| -> Result<_, typaxis_pagination::ProductionBodyPaginationError> {
                let mut search = prepare_production_table_body_search(&measured, limits, work)?;
                let stable = search.select_stable_mixed_pages(2)?;
                assert_eq!(stable.passes(), 2);
                assert_eq!(stable.sequence().pages().len(), 2);
                assert_eq!(stable.record_charge(), search.record_charge());
                assert_eq!(stable.work_steps(), search.work_steps());
                Ok((search.record_charge(), search.work_steps()))
            };
            let result = run(100_000);
            if delta == Some(1) {
                assert_eq!(result.unwrap_err().kind, E::FragmentLimit);
                return;
            }
            let (records, work) = result.unwrap();
            required = records;
            assert_eq!(run(work).unwrap(), (records, work));
            assert!(matches!(
                run(work - 1).unwrap_err().kind,
                E::TableSearchLimit | E::FootnoteSearchLimit
            ));
            let mut limited =
                prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
            assert_eq!(
                limited.select_stable_mixed_pages(1).err().unwrap().kind,
                E::PagePassLimit
            );
        });
    }
}
#[test]
fn production_mixed_pages_place_rowspan_continuations_without_repainting_padding() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
    };
    let mut value = production_table_break_fixture(true);
    value["page_masters"]["masters"][0]["body"]["height"] = 3_000_000.into();
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let stable = search.select_stable_mixed_pages(2).unwrap();
        let placed = search.place_mixed_pages(stable.sequence()).unwrap();
        assert_eq!(placed.pages().len(), 3);
        assert_eq!(
            placed
                .pages()
                .iter()
                .map(|p| p.fragments().len())
                .sum::<usize>(),
            13
        );
        let mut semantic = std::collections::BTreeSet::new();
        for page in placed.pages() {
            for (fragment, role) in page.fragments().iter().zip(page.cell_roles()) {
                if !role.unwrap().repeated_header() {
                    assert!(semantic.insert(fragment.item_index()));
                }
                let bounds = fragment.fragment().bounds();
                assert!(bounds.y() >= blocks.page_geometry().body().y());
                assert!(
                    bounds.y().checked_add(bounds.height().get()).unwrap()
                        <= blocks
                            .page_geometry()
                            .body()
                            .y()
                            .checked_add(blocks.page_geometry().body().height().get())
                            .unwrap()
                );
            }
        }
        assert_eq!(semantic.len(), 9);
    });
}

fn production_formula_header_table_fixture() -> serde_json::Value {
    let original = production_table_fixture();
    let math =
        original["document"]["blocks"][0]["blocks"][0]["body"][0]["cells"][1]["blocks"][0].clone();
    let mut value = production_table_break_fixture(false);
    let table = &mut value["document"]["blocks"][0]["blocks"][0];
    table["head"][0]["span"] = table["span"].clone();
    table["head"][0]["cells"][0]["span"] = table["span"].clone();
    table["head"][0]["cells"][0]["blocks"] = serde_json::json!([math]);
    value["page_masters"]["masters"][0]["body"]["height"] = 6_000_000.into();
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_mixed_pages_repeat_real_formula_headers_with_explicit_copy_roles() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyFragmentSource as Source,
    };
    let value = production_formula_header_table_fixture();
    with_production_body_inputs(&value, &config(), |lines, blocks, limits| {
        let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
        let measured =
            prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
        let mut search = prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
        let stable = search.select_stable_mixed_pages(2).unwrap();
        let placed = search.place_mixed_pages(stable.sequence()).unwrap();
        assert_eq!(placed.pages().len(), 2);
        let mut copies = 0;
        let mut semantic = 0;
        for (index, page) in placed.pages().iter().enumerate() {
            for (placed, role) in page.fragments().iter().zip(page.cell_roles()) {
                let fragment = placed.fragment();
                if let Source::VectorBlock { block_index } = fragment.source() {
                    assert_eq!(block_index, 0);
                    assert_eq!(fragment.owner(), blocks.blocks()[0].owner());
                    assert_eq!(
                        fragment.viewport().unwrap().height(),
                        blocks.blocks()[0].viewport_height()
                    );
                    assert_eq!(role.unwrap().repeated_header(), index > 0);
                    if index == 0 {
                        semantic += 1;
                    } else {
                        copies += 1;
                    }
                }
            }
        }
        assert_eq!((semantic, copies), (1, 1));
    });
}

#[test]
fn production_mixed_pages_complete_math_once_and_retain_every_header_draw() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
    };
    let value = production_formula_header_table_fixture();
    with_production_body_math_resources(
        &value,
        &config(),
        |lines, blocks, limits, admitted, semantics, profile, registry| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured =
                prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
            let mut search =
                prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
            let stable = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            let terminals = search
                .finalize_mixed_page_math(&stable, &placed, registry, limits)
                .unwrap();
            terminals.terminals().verify(registry).unwrap();
            assert_eq!(terminals.terminals().receipts().len(), 1);
            assert!(std::ptr::eq(terminals.geometry().mixed().unwrap(), &placed));
            let display = typaxis_display_list::build_production_footnote_display(
                &terminals, admitted, limits,
            )
            .unwrap();
            let copies = display.repeated_header_draws().collect::<Vec<_>>();
            assert!(!copies.is_empty());
            let mut formulas = (0, 0);
            for (index, draw) in display.draws().iter().enumerate() {
                if matches!(draw, typaxis_display_list::ProductionBodyDraw::Vector(_)) {
                    if display.table_draw_role(index).unwrap().repeated_header() {
                        formulas.1 += 1;
                    } else {
                        formulas.0 += 1;
                    }
                }
            }
            assert_eq!(formulas, (1, 1));
            let structure = typaxis_display_list::build_production_footnote_structure(
                &display,
                semantics,
                profile.authorization(),
                profile.base().authorization(),
                admitted,
                limits,
            )
            .unwrap();
            assert_eq!(
                structure
                    .groups()
                    .iter()
                    .filter(|g| g.vector_usage_id().is_some())
                    .count(),
                1
            );
            for index in copies {
                assert!(!structure
                    .groups()
                    .iter()
                    .any(|group| group.draws().contains(&index)));
            }
            let fonts =
                typaxis_resources::finalize_production_footnote_fonts(&structure, admitted, limits)
                    .unwrap();
            let content =
                typaxis_pdf::build_production_footnote_page_content(&fonts, admitted, limits)
                    .unwrap();
            let marked =
                typaxis_pdf::build_production_footnote_marked_content(&content, admitted, limits)
                    .unwrap();
            assert_eq!(marked.pages().len(), 2);
            let first = String::from_utf8_lossy(marked.pages()[0].content());
            let second = String::from_utf8_lossy(marked.pages()[1].content());
            assert!(first.contains("/PBA 1 Tf"));
            assert!(!second.contains("/PBA 1 Tf"));
            assert_eq!(
                second.matches("/Artifact BMC").count(),
                display.repeated_header_draws().count()
            );
            for (page, source) in marked.pages().iter().zip(content.pages()) {
                let bytes = String::from_utf8_lossy(page.content());
                assert_eq!(
                    bytes.matches("/MCID ").count(),
                    structure.page_groups(page.page_index()).unwrap().len()
                );
                assert_eq!(
                    bytes.matches(" Do").count(),
                    String::from_utf8_lossy(source.content())
                        .matches(" Do")
                        .count()
                );
            }
            let mut other =
                prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
            assert!(other
                .finalize_mixed_page_math(&stable, &placed, registry, limits)
                .is_err());
            let different = search.select_stable_mixed_pages(2).unwrap();
            assert!(search
                .finalize_mixed_page_math(&different, &placed, registry, limits)
                .is_err());
        },
    );
}

#[test]
fn production_mixed_pages_single_page_table_reaches_common_pdf_with_real_math() {
    let value = production_table_fixture();
    with_production_inline_tagged_context(
        &serde_json::to_vec(&value).unwrap(),
        &config(),
        |prepared, package, _, limits, admitted, _, semantics, profile| {
            let run = || {
                with_production_common_footnote_pdf(
                    package,
                    prepared.source_flow().navigation(),
                    semantics,
                    profile,
                    admitted,
                    limits,
                    typaxis_linebreak::JapaneseLineBreakMode::Normal,
                    100_000,
                    |pdf, stable, _, _, observation| {
                        assert!(matches!(stable, ProductionCommonStablePages::Mixed(_)));
                        assert_eq!(stable.page_count(), 1);
                        assert_eq!(pdf.page_count(), 1);
                        assert_eq!(observation.block_math_terminals, 1);
                        assert!(pdf.bytes().starts_with(b"%PDF-1.7"));
                        Ok(pdf.content_hash())
                    },
                )
            };
            assert_eq!(run().unwrap(), run().unwrap());
        },
    );
}

#[test]
fn production_mixed_pages_numbered_header_copies_keep_terminal_and_display_budgets() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
    };
    let value = production_numbered_formula_header_table_fixture();
    let mut required = 0;
    for delta in [None, Some(0), Some(1)] {
        let cfg = delta.map_or_else(config, |d| {
            config_with_limits(ResourceLimits {
                max_fragments: required - d,
                ..ResourceLimits::default()
            })
        });
        with_production_body_math_resources(
            &value,
            &cfg,
            |lines, blocks, limits, admitted, _, _, registry| {
                let notes =
                    typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
                let measured =
                    prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
                let mut search =
                    prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
                let stable = search.select_stable_mixed_pages(2).unwrap();
                let placed = search.place_mixed_pages(stable.sequence()).unwrap();
                let terminals = search
                    .finalize_mixed_page_math(&stable, &placed, registry, limits)
                    .unwrap();
                assert_eq!(terminals.terminals().receipts().len(), 1);
                assert_eq!(terminals.equation_numbers().len(), 2);
                assert_eq!(terminals.equation_numbers()[0].page_index(), 0);
                assert_eq!(terminals.equation_numbers()[1].page_index(), 1);
                assert_eq!(
                    terminals.equation_numbers()[0].owner(),
                    terminals.equation_numbers()[1].owner()
                );
                let display = typaxis_display_list::build_production_footnote_display(
                    &terminals, admitted, limits,
                );
                if delta == Some(1) {
                    assert_eq!(
                        display.err().unwrap().kind,
                        typaxis_display_list::ProductionBodyDisplayErrorKind::RecordLimit
                    );
                    return;
                }
                let display = display.unwrap();
                required = display.record_charge();
                let numbers = display
                    .draws()
                    .iter()
                    .enumerate()
                    .filter_map(|(index, draw)| {
                        let typaxis_display_list::ProductionBodyDraw::Text(text) = draw else {
                            return None;
                        };
                        text.equation_number().map(|_| {
                            (
                                text.page_index(),
                                display.table_draw_role(index).unwrap().repeated_header(),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                assert_eq!(numbers, [(0, false), (1, true)]);
            },
        );
    }
}

fn production_formula_header_table_with_tail_fixture() -> serde_json::Value {
    let mut value = production_formula_header_table_fixture();
    let formula =
        value["document"]["blocks"][0]["blocks"][0]["head"][0]["cells"][0]["blocks"][0].clone();
    value["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(formula);
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_mixed_pages_final_observation_covers_semantic_vectors_after_header_artifacts() {
    let value = production_formula_header_table_with_tail_fixture();
    let cfg = config();
    with_production_inline_tagged_context(
        &serde_json::to_vec(&value).unwrap(),
        &cfg,
        |prepared, package, _, limits, admitted, _, semantics, profile| {
            with_production_common_tagged_pdf(
                package,
                prepared.source_flow().navigation(),
                semantics,
                profile,
                admitted,
                limits,
                typaxis_linebreak::JapaneseLineBreakMode::Normal,
                100_000,
                cfg.fingerprint(),
                |diagnostic, pdf, _| {
                    let observation = pdf.tagged_observation();
                    assert_eq!(diagnostic.page_count(), 3);
                    assert_eq!(
                        observation
                            .vector_records()
                            .iter()
                            .map(|r| r.usage_id())
                            .collect::<Vec<_>>(),
                        [0, 2]
                    );
                    assert_eq!(observation.vector_artifact_count(), 1);
                    assert!(observation.vector_is_artifact(1));
                    assert!(observation.vector_record(1).is_none());
                    assert_eq!(observation.vector_record(2).unwrap().page_index(), 2);
                    let encoded: serde_json::Value =
                        serde_json::from_str(observation.canonical_jcs()).unwrap();
                    assert_eq!(
                        encoded["algorithm"],
                        "typaxis.production-common-tagged-pdf-observation/2"
                    );
                    let copy = &encoded["vector_artifacts"][0];
                    assert_eq!(copy["usage_id"], 1);
                    assert_eq!(copy["page_index"], 1);
                    assert!(copy.get("mcid").is_none());
                    assert!(copy.get("structure_object").is_none());
                    assert_eq!(copy["artifact"], "table_header_copy");
                    Ok(())
                },
            )
            .unwrap();
        },
    );
}

fn production_numbered_formula_header_table_fixture() -> serde_json::Value {
    let numbered = production_numbered_body_fixture(6_000_000);
    let mut value = production_formula_header_table_fixture();
    value["sources"] = numbered["sources"].clone();
    value["text_buffers"] = numbered["text_buffers"].clone();
    let span = numbered["document"]["blocks"][0]["span"].clone();
    value["document"]["blocks"][0]["span"] = span.clone();
    let table = &mut value["document"]["blocks"][0]["blocks"][0];
    table["span"] = span.clone();
    table["head"][0]["span"] = span.clone();
    table["head"][0]["cells"][0]["span"] = span;
    table["head"][0]["cells"][0]["blocks"][0] =
        numbered["document"]["blocks"][0]["blocks"][1].clone();
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

fn production_inline_formula_header_table_fixture() -> serde_json::Value {
    let original = production_table_fixture();
    let mut value = production_formula_header_table_fixture();
    value["document"]["blocks"][0]["blocks"][0]["head"][0]["cells"][0]["blocks"][0] =
        original["document"]["blocks"][0]["blocks"][0]["body"][0]["cells"][0]["blocks"][0].clone();
    production_body_renumber(&mut value["document"], &mut 0);
    value
}

#[test]
fn production_mixed_pages_rank_all_boundaries_before_one_necessary_footnote_fit() {
    use typaxis_pagination::{
        prepare_production_table_body_search, prepare_production_table_measurements,
        ProductionBodyPaginationErrorKind as E,
    };
    let mut value = production_table_fixture();
    let table = value["document"]["blocks"][0]["blocks"][0].clone();
    let paragraph = table["head"][0]["cells"][0]["blocks"][0].clone();
    let mut parts = vec![paragraph; 20];
    parts.push(table);
    value["document"]["blocks"][0]["blocks"] = parts.into();
    production_body_renumber(&mut value["document"], &mut 0);
    let mut selected_ends = None;
    for (reflows, lookback) in [(64, 32), (1, 32), (1, 1)] {
        let cfg = config_with_limits(ResourceLimits {
            max_footnote_reflows_per_page: reflows,
            max_page_break_lookback: lookback,
            ..ResourceLimits::default()
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let measured =
                prepare_production_table_measurements(lines, blocks, &notes, limits).unwrap();
            let mut search =
                prepare_production_table_body_search(&measured, limits, 100_000).unwrap();
            let result = search.select_mixed_pages();
            if lookback == 1 {
                assert!(matches!(
                    result.err().unwrap().kind,
                    E::PageBreakLookbackLimit {
                        limit: 1,
                        observed: 2
                    }
                ));
                return;
            }
            let pages = result.unwrap();
            assert!(pages.pages().len() >= 2);
            assert!(pages.pages().iter().all(|p| p.candidate_attempts() == 1));
            let ends = pages
                .pages()
                .iter()
                .map(|p| {
                    (
                        p.next_state().next_item(),
                        p.next_state()
                            .table_continuation()
                            .map(|c| c.offset().raw()),
                    )
                })
                .collect::<Vec<_>>();
            if let Some(previous) = &selected_ends {
                assert_eq!(previous, &ends);
            }
            selected_ends = Some(ends);
        });
    }
}

#[test]
fn production_footnote_pages_rank_complete_ordinary_boundaries_without_spending_reflows_on_worse_keys(
) {
    use typaxis_pagination::{
        prepare_production_body_flow, prepare_production_footnote_demand_search,
        ProductionBodyPaginationErrorKind as E,
    };
    let mut value = production_body_fixture(12_000_000);
    let paragraph = value["document"]["blocks"][0]["blocks"][2].clone();
    value["document"]["blocks"][0]["blocks"] = serde_json::json!(vec![paragraph; 20]);
    production_body_renumber(&mut value["document"], &mut 0);
    let mut selected_ends = None;
    for (reflows, lookback) in [(64, 32), (1, 32), (1, 1)] {
        let cfg = config_with_limits(ResourceLimits {
            max_footnote_reflows_per_page: reflows,
            max_page_break_lookback: lookback,
            ..ResourceLimits::default()
        });
        with_production_body_inputs(&value, &cfg, |lines, blocks, limits| {
            let notes = typaxis_layout::prepare_production_footnote_lines(lines, limits).unwrap();
            let flow = prepare_production_body_flow(lines, blocks, &notes, limits).unwrap();
            let mut search =
                prepare_production_footnote_demand_search(&flow, limits, 100_000).unwrap();
            let result = search.select_pages();
            if lookback == 1 {
                assert!(matches!(
                    result.err().unwrap().kind,
                    E::PageBreakLookbackLimit {
                        limit: 1,
                        observed: 2
                    }
                ));
                return;
            }
            let pages = result.unwrap();
            let ends = pages
                .pages()
                .iter()
                .map(|p| p.next_state().body_start())
                .collect::<Vec<_>>();
            assert!(ends.len() >= 2);
            assert_eq!(ends.last(), Some(&20));
            if let Some(previous) = &selected_ends {
                assert_eq!(previous, &ends);
            }
            selected_ends = Some(ends);
        });
    }
}
