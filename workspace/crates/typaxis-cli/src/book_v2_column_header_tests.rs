use super::*;
use crate::book_v2_resources::tests::shaping_tests::column_lines::input;
use std::collections::BTreeSet;
use typaxis_core::{NodeId, PositiveLength};
use typaxis_layout::book_v2::*;
use typaxis_pagination::book_v2::*;
use typaxis_syntax::book_v2::prepare_book_v2_column_frame_plan;

const MODE: JapaneseLineBreakMode = JapaneseLineBreakMode::Normal;
#[path = "book_v2_column_width_tests.rs"]
mod width_feedback;
fn data(notes: bool, mode: &str, text: &str) -> Value {
    let fixture_mode = if mode.starts_with("nested-body-") { "nested-body" } else { mode };
    let mut data = super::header_selection::fixture(notes, fixture_mode, text);
    if mode.starts_with("nested-body-") {
        let parent = if notes { &mut data["document"]["footnotes"][0]["blocks"][0] }
            else { &mut data["document"]["blocks"][0] };
        let child = &mut parent["body"][0]["cells"][0]["blocks"][0];
        if mode == "nested-body-span" {
            let mut row = child["body"][0].clone();
            row["cells"].as_array_mut().unwrap().remove(0);
            child["body"][0]["cells"][0]["rowspan"] = 2.into();
            child["body"].as_array_mut().unwrap().push(row);
        } else if mode == "nested-body-caption" {
            child["caption"] = json!([child["body"][0]["cells"][0]["blocks"][0].clone()]);
        }
        crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
            &mut data["document"], &mut 0,
        );
    }
    let height = match mode {
        "nested-header" => 1600,
        "nested-body" => 384,
        _ => 256,
    };
    if mode == "nested-header" {
        let table = if notes {
            &mut data["document"]["footnotes"][0]["blocks"][0]
        } else {
            &mut data["document"]["blocks"][0]
        };
        for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
            let paragraph = cell["blocks"][0].clone();
            cell["blocks"] = vec![paragraph; 64].into();
        }
        crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
            &mut data["document"],
            &mut 0,
        );
    }
    for (index, master) in data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let column = [140, 220, 140, 220][index];
        master["width"] = (600 * 65536).into();
        master["trim"]["width"] = (600 * 65536).into();
        master["body"]["width"] = ((2 * column + 10) * 65536).into();
        master["body"]["height"] = (height * 65536).into();
        master["column_layout"] =
            json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"none"});
        if notes {
            master["footnote"]["width"] = (column * 65536).into();
            master["footnote"]["height"] = (height * 65536).into();
        }
    }
    data
}

// Include independently continued child headers as well as complete nested
// content inside an ancestor header. Other body leaves retain the base graph.
fn header_owners(value: &Value, in_head: bool, owners: &mut BTreeSet<NodeId>) {
    if in_head && matches!(value["kind"].as_str(), Some("paragraph" | "page_break")) {
        owners.insert(NodeId::new(value["node_id"].as_u64().unwrap() as u32));
    }
    if let Some(values) = value.as_array() {
        for value in values {
            header_owners(value, in_head, owners);
        }
    } else if let Some(values) = value.as_object() {
        for (key, value) in values {
            header_owners(value, in_head || key == "head", owners);
        }
    }
}

fn check(font: Option<&[u8]>, notes: bool, mode: &str) {
    check_with_height(font, notes, mode, None);
}

fn check_with_height(font: Option<&[u8]>, notes: bool, mode: &str, height: Option<i64>) {
    check_feedback_with_height(font, notes, mode, height, false);
}
fn check_feedback_with_height(font: Option<&[u8]>, notes: bool, mode: &str, height: Option<i64>, feedback: bool) {
    let text = if font.is_some() {
        "本文を続けて組み直す本文を続けて組み直す"
    } else {
        "Pro Pro Pro Pro Pro Pro Pro"
    };
    let root = Root::new();
    let original = limits();
    let mut base = original.base().get().clone();
    base.max_page_break_lookback = 256;
    base.max_footnote_reflows_per_page = 256;
    let caps = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(base).unwrap(),
        original.extension().get().clone(),
    )
    .unwrap();
    let mut data = data(notes, mode, text);
    if let Some(height) = height {
        for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
            master["body"]["height"] = (height * 65536).into();
            if notes {
                master["footnote"]["height"] = (height * 65536).into();
            }
        }
    }
    let mut owners = BTreeSet::new();
    header_owners(&data["document"], false, &mut owners);
    let owners = owners.into_iter().collect::<Vec<_>>();
    let input = input(&root, data, text, font, &caps);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 10_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &caps).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &caps).unwrap();
    let mut allowance =
        BookV2LineVariantBudget::new(10_000_000, caps.base().get().max_line_reshape_passes);
    let seed = prepare_budgeted_book_v2_column_line_variant_seed(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &caps,
        MODE,
        &plan,
        &mut allowance,
        0,
        None,
        None,
    )
    .unwrap();
    let budgets = font.is_none() && !notes && mode == "common";
    if budgets {
        let make = |work, passes, prior| {
            let mut budget = BookV2LineVariantBudget::new(work, passes);
            let result = prepare_budgeted_book_v2_column_line_variant_seed(
                &policy,
                &flow,
                input.resources(),
                &bindings,
                &caps,
                MODE,
                &plan,
                &mut budget,
                prior,
                None,
                None,
            )
            .map(|v| (v.fingerprint(), v.record_charge()));
            (result, budget)
        };
        let exact = make(seed.work_steps(), seed.reshape_passes(), 0);
        assert_eq!(exact.0.unwrap(), (seed.fingerprint(), seed.record_charge()));
        assert_eq!(exact.1.work_steps(), seed.work_steps());
        assert!(make(seed.work_steps() - 1, seed.reshape_passes(), 0)
            .0
            .is_err());
        assert!(make(seed.work_steps(), seed.reshape_passes() - 1, 0)
            .0
            .is_err());
        let prior = caps.base().get().max_fragments - seed.record_charge();
        assert_eq!(
            make(seed.work_steps(), seed.reshape_passes(), prior)
                .0
                .unwrap()
                .1,
            caps.base().get().max_fragments
        );
        let short = make(seed.work_steps(), seed.reshape_passes(), prior + 1);
        assert!(short.0.is_err());
        assert!(short.1.record_charge() >= prior + 1);
        assert!(short.1.record_charge() <= caps.base().get().max_fragments);
    }
    let (owner, original_parent) = {
        let mut replay = BookV2LineVariantBudget::new(10_000_000, 0);
        with_budgeted_rebuilt_book_v2_column_line_variants(
            &[&seed],
            &mut replay,
            seed.record_charge(),
            |set| {
                let v = set.variant(0).unwrap();
                let owner = flow.tables()[0].owner();
                (
                    owner,
                    v.lines()
                        .frames()
                        .unwrap()
                        .measurement_region(owner)
                        .unwrap()
                        .width(),
                )
            },
        )
        .unwrap()
    };
    let root_widths = [
        [(
            owner,
            PositiveLength::new(
                original_parent
                    .get()
                    .checked_sub(Length::from_raw(80 * 65536).unwrap())
                    .unwrap(),
            )
            .unwrap(),
        )],
        [(owner, original_parent)],
    ];
    let profiles = vec![None; flow.paragraphs().len()];
    let assignments = root_widths
        .iter()
        .map(|w| {
            BookV2SourceWidthAssignments::new(&flow, &profiles)
                .unwrap()
                .with_root_table_widths(w)
                .with_table_source_scope(owner, &owners)
        })
        .collect::<Vec<_>>();
    let mut siblings = Vec::new();
    for assignment in &assignments {
        let mut allowance =
            BookV2LineVariantBudget::new(10_000_000, caps.base().get().max_line_reshape_passes);
        siblings.push(
            seed.prepare_budgeted_with_source_widths(
                assignment,
                &mut allowance,
                seed.record_charge(),
            )
            .unwrap(),
        );
    }
    if budgets {
        let seeds = [&seed, &siblings[0], &siblings[1]];
        let replay = |work, prior| {
            let mut budget = BookV2LineVariantBudget::new(work, 0);
            let result = with_budgeted_rebuilt_book_v2_column_line_variants(
                &seeds,
                &mut budget,
                prior,
                |set| (set.fingerprint(), set.record_charge(), set.work_steps()),
            );
            (result, budget)
        };
        let full = replay(20_000_000, 0);
        let accepted = full.0.unwrap();
        assert_eq!(full.1.work_steps(), accepted.2);
        assert_eq!(replay(accepted.2, 0).0.unwrap(), accepted);
        let short = replay(accepted.2 - 1, 0);
        assert!(short.0.is_err());
        assert!(short.1.work_steps() > 0);
        assert!(short.1.work_steps() <= accepted.2 - 1);
        assert!(short.1.record_charge() >= seed.record_charge());
        let raised = caps.base().get().max_fragments / 2;
        let extra = replay(accepted.2, raised).0.unwrap().1 - raised;
        let prior = caps.base().get().max_fragments - extra;
        assert_eq!(
            replay(accepted.2, prior).0.unwrap().1,
            caps.base().get().max_fragments
        );
        assert!(replay(accepted.2, prior + 1).0.is_err());
        let mut empty = BookV2LineVariantBudget::new(20_000_000, 0);
        assert!(
            with_budgeted_rebuilt_book_v2_column_line_variants(&[], &mut empty, 17, |_| ())
                .is_err()
        );
        assert_eq!(empty.record_charge(), 17);
        let equivalent =
            prepare_book_v2_column_frame_plan(&flow, &mut 0, 10_000_000, 0, 0).unwrap();
        let mut budget =
            BookV2LineVariantBudget::new(10_000_000, caps.base().get().max_line_reshape_passes);
        let foreign = prepare_budgeted_book_v2_column_line_variant_seed(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &caps,
            MODE,
            &equivalent,
            &mut budget,
            0,
            None,
            None,
        )
        .unwrap();
        assert_eq!(foreign.fingerprint(), seed.fingerprint());
        let mut replay = BookV2LineVariantBudget::new(20_000_000, 0);
        assert!(with_budgeted_rebuilt_book_v2_column_line_variants(
            &[&seed, &foreign],
            &mut replay,
            0,
            |_| panic!("different column plan reached callback")
        )
        .is_err());
        assert_eq!(replay.work_steps(), 2);
        assert!(replay.record_charge() >= seed.record_charge());
    }
    let mut replay = BookV2LineVariantBudget::new(20_000_000, 0);
    with_budgeted_rebuilt_book_v2_column_line_variants(&[&seed, &siblings[0], &siblings[1]], &mut replay, 0, |set| {
        let views = set.variants().collect::<Vec<_>>();
        let mut records = set.record_charge();
        let measured = views.iter().map(|v| {
            assert!(std::ptr::eq(v.column_plan(), &plan));
            assert!(prepare_book_v2_body_flow_counted(v.lines(), None, v.footnotes(), &caps, records, &mut 0).is_err());
            let source = prepare_book_v2_rebuilt_column_flow_counted(v, None, &caps, records, &mut records).unwrap();
            prepare_book_v2_column_table_measurements_counted(source, &caps, &mut records).unwrap()
        }).collect::<Vec<_>>();
        let base = &measured[0];
        let mut headers = Vec::new();
        for table in 0..base.tables().len() {
            for graph in [1, 2] {
                let mut work = 0;
                let prior = records;
                let header = prepare_book_v2_column_table_header_variant_counted(&set, base, &measured[graph], table, &caps,
                    10_000_000, records, &mut records, &mut work).unwrap();
                assert_eq!(header.work_steps(), work);
                if budgets {
                    let make = |maximum, prior| {
                        let (mut records, mut work) = (0, 0);
                        let result = prepare_book_v2_column_table_header_variant_counted(&set, base, &measured[graph], table,
                            &caps, maximum, prior, &mut records, &mut work)
                            .map(|v| (v.fingerprint(), v.record_charge()));
                        (result, records, work)
                    };
                    assert_eq!(make(work, prior).0.unwrap(), (header.fingerprint(), header.record_charge()));
                    let short = make(work - 1, prior);
                    assert!(short.0.is_err());
                    assert!(short.1 >= prior && short.2 <= work - 1);
                    let raised = caps.base().get().max_fragments / 2;
                    let extra = make(work, raised).0.unwrap().1 - raised;
                    let exact = caps.base().get().max_fragments - extra;
                    assert_eq!(make(work, exact).0.unwrap().1, caps.base().get().max_fragments);
                    assert!(make(work, exact + 1).0.is_err());
                }
                header.verify(base, &measured[graph], &caps).unwrap();
                assert!(header.verify(&measured[1], &measured[graph], &caps).is_err());
                headers.push(header);
            }
        }
        assert!(headers[0].height() > headers[1].height(), "{mode}/{notes}: reflow did not change header height");
        let refs = headers.iter().collect::<Vec<_>>();
        let mut work = 0;
        let prior = records;
        let catalog = prepare_book_v2_column_table_header_catalog_counted(base, &refs, &caps, 10_000_000, records,
            &mut records, &mut work).unwrap();
        assert_eq!(catalog.len(), headers.len());
        assert_eq!(catalog.work_steps(), work);
        assert!(std::ptr::eq(catalog.base(), base));
        if budgets {
            let make = |maximum, prior| {
                let (mut records, mut work) = (0, 0);
                let result = prepare_book_v2_column_table_header_catalog_counted(base, &refs, &caps, maximum, prior,
                    &mut records, &mut work).map(|v| (v.fingerprint(), v.record_charge(), v.work_steps()));
                (result, records, work)
            };
            assert_eq!(make(work, prior).0.unwrap(), (catalog.fingerprint(), catalog.record_charge(), catalog.work_steps()));
            let short = make(work - 1, prior);
            assert!(short.0.is_err());
            assert!(short.1 >= prior && short.2 <= work - 1);
            let raised = caps.base().get().max_fragments / 2;
            let extra = make(work, raised).0.unwrap().1 - raised;
            let exact = caps.base().get().max_fragments - extra;
            assert_eq!(make(work, exact).0.unwrap().1, caps.base().get().max_fragments);
            assert!(make(work, exact + 1).0.is_err());
            for refs in [vec![refs[1], refs[0]], vec![refs[0], refs[0]]] {
                let (mut records, mut work) = (0, 0);
                assert!(prepare_book_v2_column_table_header_catalog_counted(base, &refs, &caps, 10_000_000, prior,
                    &mut records, &mut work).is_err());
                assert!(records >= prior && work > 0);
            }
            let empty = prepare_book_v2_column_table_header_catalog_counted(base, &[], &caps, 10_000_000, prior,
                &mut 0, &mut 0).unwrap();
            let mut search = prepare_book_v2_column_page_search_with_headers_counted(&empty, &caps, 20_000_000,
                empty.record_charge(), &mut 0, &mut 0).unwrap();
            let error = match search.select_pages() { Ok(_) => panic!("missing header width accepted"), Err(error) => error };
            assert!(matches!(error.kind, typaxis_pagination::ProductionBodyPaginationErrorKind::TableHeaderWidthRequired { .. }));
        }
        let mut search = prepare_book_v2_column_page_search_with_headers_counted(&catalog, &caps, 20_000_000, records,
            &mut 0, &mut 0).unwrap();
        let pages = search.select_pages().unwrap_or_else(|e| panic!("{mode}/{notes}/harano={}: {e:?}", font.is_some()));
        search.verify_sequence(&pages).unwrap();
        if height == Some(512) && font.is_none() && !notes && mode == "nested-body" {
            let snapshot = |pages: &BookV2ColumnPageSequence<'_, '_, '_, '_, '_>| {
                let mut digest = [0u8; 32];
                for page in pages.pages() {
                    for column in page.candidate().columns() {
                        for table in column.parts().iter().filter_map(|part| part.table()) {
                            let mut bytes = [0u8; 64];
                            bytes[..32].copy_from_slice(&digest);
                            bytes[32..].copy_from_slice(&table.fingerprint());
                            digest = typaxis_core::sha256(&bytes);
                        }
                    }
                }
                (digest, pages.pages().len())
            };
            let full = (snapshot(&pages), search.work_steps(), search.record_charge());
            let run = |maximum, prior| {
                let (mut observed_records, mut observed_work) = (0, 0);
                let mut search = match prepare_book_v2_column_page_search_with_headers_counted(
                    &catalog, &caps, maximum, prior, &mut observed_records, &mut observed_work,
                ) {
                    Ok(search) => search,
                    Err(error) => return (Err(error), observed_records, observed_work),
                };
                let result = search.select_pages().and_then(|pages| {
                    search.verify_sequence(&pages)?;
                    Ok(snapshot(&pages))
                });
                (result, search.record_charge(), search.work_steps())
            };
            let exact = run(full.1, records);
            assert_eq!(exact.0.unwrap(), full.0);
            assert_eq!((exact.2, exact.1), (full.1, full.2));
            let short = run(full.1 - 1, records);
            assert!(short.0.is_err());
            assert!(short.1 >= records && short.2 > 0 && short.2 <= full.1 - 1);
            let raised = caps.base().get().max_fragments / 2;
            let extra = run(full.1, raised).1 - raised;
            let exact_records = caps.base().get().max_fragments - extra;
            let exact = run(full.1, exact_records);
            assert_eq!(exact.0.unwrap(), full.0);
            assert_eq!(exact.1, caps.base().get().max_fragments);
            let short = run(full.1, exact_records + 1);
            assert!(short.0.is_err());
            assert!(short.1 >= exact_records + 1 && short.1 <= caps.base().get().max_fragments);
            assert!(short.2 <= full.1);
        }
        let mut covered = BTreeSet::new();
        let mut repeated = 0;
        let mut actual_widths = BTreeSet::new();
        let mut table_visit = |table: &BookV2TableFragmentSelection<'_, '_, '_, '_, '_>, actual: PositiveLength| {
            assert!(table.used_height() <= table.available_height());
            actual_widths.insert(actual.get().raw());
            for range in table.semantic_leaf_ranges() { for item in range { assert!(covered.insert(item)); } }
            if table.repeats_header() {
                repeated += 1;
                assert!(table.has_header_variants());
                let index = match actual.get().raw() { n if n == 140 * 65536 => 0, n if n == 220 * 65536 => 1,
                    _ => panic!("unexpected physical width") };
                let header = &headers[table.before().table_index() * 2 + index];
                assert_eq!(table.header_variant().unwrap().fingerprint(), header.fingerprint());
                assert_eq!(table.header_height(), header.height());
            }
            for leaf in table.variant_placement_leaves() {
                let leaf = leaf.unwrap();
                let item = leaf.measurements().item(leaf.global_item_index()).unwrap();
                assert!(leaf.top() >= Length::ZERO);
                assert!(leaf.top().checked_add(item.consumed_height().unwrap()).unwrap() <= table.used_height());
                if leaf.uses_header_variant() {
                    let index = usize::from(actual.get().raw() == 220 * 65536);
                    assert_eq!(leaf.measurements().flow().lines().frames().unwrap().region(owner).unwrap().width(),
                        root_widths[index][0].1);
                }
            }
            if !table.has_header_variants() {
                for leaf in table.placement_leaves() {
                    let (_, index, top, _) = leaf.unwrap();
                    assert!(top >= Length::ZERO);
                    assert!(top.checked_add(base.item(index).unwrap().consumed_height().unwrap()).unwrap() <= table.used_height());
                }
            }
        };
        for page in pages.pages() {
            for column in page.candidate().columns() {
                for part in column.parts() {
                    if let Some(table) = part.table() { table_visit(table, column.bounds().width()); }
                }
            }
            if let Some(notes) = page.candidate().footnotes() {
                for selected in notes.fragments() {
                    if let Some(mixed) = selected.fragment().mixed() {
                        for part in mixed.parts() {
                            if let Some(table) = part.table() { table_visit(table, page.candidate().frames().footnote().unwrap().width()); }
                        }
                    }
                }
            }
        }
        assert!(repeated > 0, "{mode}/{notes}");
        assert!(actual_widths.len() > 1, "{mode}/{notes}: one actual width");
        let expected = if notes { base.definition_items(0).unwrap().len() } else { base.body_table_range(0).unwrap().len() };
        assert_eq!(covered.len(), expected);
        if feedback {
            let report = search.paragraph_frame_feedback(&pages).unwrap_or_else(|e| panic!("feedback/{mode}/{notes}: {e:?}"));
            assert!(report.matches_source_flow(&flow));
            assert!(std::ptr::eq(report.column_plan(), &plan));
            assert_eq!(report.paragraphs().len(), flow.paragraphs().len());
            assert!(report.paragraphs().iter().any(|p| p.source_unit_starts().is_some()));
            assert_eq!((report.work_steps(), report.record_charge()), (search.work_steps(), search.record_charge()));
            if budgets {
                let run = |maximum, prior, retain| {
                    let (mut observed_records, mut observed_work) = (0, 0);
                    let mut search = match prepare_book_v2_column_page_search_with_headers_counted(
                        &catalog, &caps, maximum, prior, &mut observed_records, &mut observed_work,
                    ) {
                        Ok(search) => search,
                        Err(error) => return (Err(error), observed_records, observed_work),
                    };
                    let result = (|| {
                        let pages = search.select_pages()?;
                        let mut report = search.paragraph_frame_feedback(&pages)?;
                        if retain { search.retain_paragraph_line_boundaries(&pages, &mut report)?; }
                        Ok(report.assignment_fingerprint())
                    })();
                    (result, search.record_charge(), search.work_steps())
                };
                for retain in [false, true] {
                    let full = run(20_000_000, records, retain);
                    assert!(full.0.is_ok());
                    if !retain { assert_eq!(full.0.as_ref().unwrap(), &report.assignment_fingerprint()); }
                    assert_eq!(run(full.2, records, retain), full);
                    let short = run(full.2 - 1, records, retain);
                    assert!(short.0.is_err());
                    assert!(short.1 >= records && short.2 > 0 && short.2 <= full.2 - 1);
                    let raised = caps.base().get().max_fragments / 2;
                    let extra = run(full.2, raised, retain).1 - raised;
                    let prior = caps.base().get().max_fragments - extra;
                    let exact = run(full.2, prior, retain);
                    assert_eq!(exact.0, full.0);
                    assert_eq!(exact.1, caps.base().get().max_fragments);
                    let short = run(full.2, prior + 1, retain);
                    assert!(short.0.is_err());
                    assert!(short.1 >= prior + 1 && short.1 <= caps.base().get().max_fragments && short.2 <= full.2);
                }
            }
            let mut foreign = prepare_book_v2_column_page_search_with_headers_counted(
                &catalog, &caps, 10_000_000, records, &mut 0, &mut 0).unwrap();
            assert!(matches!(foreign.paragraph_frame_feedback(&pages), Err(e) if e.kind == typaxis_pagination::ProductionBodyPaginationErrorKind::ReceiptMismatch));
        }
        json!({"pages":pages.pages().len(), "repeated":repeated, "covered":covered.len()})
    }).unwrap();
}

#[test]
fn book_v2_column_header_variants_follow_physical_body_widths() {
    check(None, false, "common");
}
#[test]
fn book_v2_column_header_variants_follow_physical_note_widths() {
    check(None, true, "common");
}
#[test]
fn book_v2_column_header_variants_preserve_nested_headers() {
    check(None, false, "nested-header");
}
#[test]
fn book_v2_column_header_variants_preserve_child_continuations() {
    check(None, false, "nested-body");
}
#[test]
fn book_v2_column_header_variants_reserve_ancestor_height() {
    for height in [512, 480, 544] {
        check_with_height(None, false, "nested-body", Some(height));
    }
}
#[test]
fn book_v2_column_header_variants_reserve_ancestor_note_height() {
    for height in [512, 544] {
        check_with_height(None, true, "nested-body", Some(height));
    }
}
#[test]
fn book_v2_column_header_variants_reserve_ancestor_spanning_height() {
    for notes in [false, true] {
        check_with_height(None, notes, "nested-body-span", Some(544));
    }
}
#[test]
fn book_v2_column_header_variants_reserve_ancestor_caption_height() {
    for notes in [false, true] {
        check_with_height(None, notes, "nested-body-caption", Some(512));
    }
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_column_header_variants_reserve_original_harano_ancestor_height() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    for notes in [false, true] {
        check_with_height(Some(&font), notes, "nested-body", Some(512));
        check_with_height(Some(&font), notes, "nested-body-span", Some(544));
    }
}
#[test]
fn book_v2_column_header_variants_preserve_rowspans() {
    check(None, false, "span");
}
#[test]
fn book_v2_column_header_variants_preserve_original_breaks() {
    check(None, false, "forced");
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_column_header_variants_preserve_original_harano() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check(Some(&font), false, "common");
    check(Some(&font), true, "common");
    check(Some(&font), false, "nested-header");
}
