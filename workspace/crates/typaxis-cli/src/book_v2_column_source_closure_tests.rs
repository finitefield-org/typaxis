use super::*;
use std::collections::BTreeMap;
use typaxis_pagination::ProductionFootnoteDemandStatus as Demand;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Coverage {
    semantic: usize,
    repeated: usize,
    variants: usize,
    unreferenced: usize,
    semantic_math: usize,
    repeated_math: usize,
    body_columns: BTreeSet<u16>,
    variant_widths: BTreeSet<i64>,
    variant_units: usize,
}

fn paragraph_owners(value: &Value, owners: &mut BTreeSet<u32>) {
    if value["kind"] == "paragraph" {
        assert!(owners.insert(value["node_id"].as_u64().unwrap() as u32));
    }
    match value {
        Value::Array(values) => {
            for value in values {
                paragraph_owners(value, owners);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                paragraph_owners(value, owners);
            }
        }
        _ => {}
    }
}

fn math_owners(value: &Value, owners: &mut BTreeSet<u32>) {
    if matches!(
        value["kind"].as_str(),
        Some("math_vector" | "math_vector_block" | "inline_math" | "display_math")
    ) {
        assert!(owners.insert(value["node_id"].as_u64().unwrap() as u32));
    }
    match value {
        Value::Array(values) => {
            for value in values {
                math_owners(value, owners);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                math_owners(value, owners);
            }
        }
        _ => {}
    }
}

pub(super) fn coverage(
    closed: &BookV2ColumnSourceClosure<'_, '_, '_, '_, '_, '_, '_>,
    search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
    authored: &Value,
) -> Coverage {
    assert_eq!(closed.work_steps(), search.work_steps());
    assert_eq!(closed.record_charge(), search.record_charge());
    coverage_content(closed, search, authored)
}

pub(super) fn coverage_retained(
    closed: &BookV2ColumnSourceClosure<'_, '_, '_, '_, '_, '_, '_>,
    search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
    authored: &Value,
) -> Coverage {
    assert!(closed.work_steps() <= search.work_steps());
    assert!(closed.record_charge() <= search.record_charge());
    coverage_content(closed, search, authored)
}

fn coverage_content(
    closed: &BookV2ColumnSourceClosure<'_, '_, '_, '_, '_, '_, '_>,
    search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
    authored: &Value,
) -> Coverage {
    let geometry = closed.geometry();
    assert!(std::ptr::eq(
        geometry.sequence(),
        closed.stable().sequence()
    ));
    assert!(closed.stable().passes() >= 2);
    let state = geometry
        .pages()
        .last()
        .unwrap()
        .selection()
        .next_state()
        .source_state();
    assert!(state.is_complete() && state.pending_definitions().is_empty());
    let mut demanded = BTreeSet::new();
    paragraph_owners(&authored["document"]["blocks"], &mut demanded);
    let mut demanded_math = BTreeSet::new();
    math_owners(&authored["document"]["blocks"], &mut demanded_math);
    let mut unreferenced = 0;
    for (definition, note) in authored["document"]["footnotes"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        match state.definition_status(definition).unwrap() {
            Demand::Complete => {
                paragraph_owners(&note["blocks"], &mut demanded);
                math_owners(&note["blocks"], &mut demanded_math);
            }
            Demand::Unreferenced => unreferenced += 1,
            other => panic!("unfinished definition: {other:?}"),
        }
    }
    let original = search.source_lines();
    let expected = original
        .paragraphs()
        .iter()
        .filter(|p| demanded.contains(&p.owner().get()))
        .flat_map(|p| (0..p.lines().len()).map(move |line| (p.owner().get(), line)))
        .collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();
    let mut semantic = 0;
    let mut repeated = 0;
    let mut variants = 0;
    let mut actual_math = BTreeSet::new();
    let mut repeated_math = 0;
    let mut body_columns = BTreeSet::new();
    let mut variant_widths = BTreeSet::new();
    // One complete header paragraph copy has one source-unit interval even
    // when its actual width changes the number and ordinal of line fragments.
    let mut copies: BTreeMap<(usize, Option<u16>, usize, Option<usize>, u32), BTreeSet<u32>> =
        BTreeMap::new();
    for (page_index, page) in geometry.pages().iter().enumerate() {
        for (index, (placed, _, copy)) in page.fragments_with_roles().enumerate() {
            let fragment = placed.fragment();
            let lines = closed.fragment_lines(page_index, index).unwrap();
            assert!(std::ptr::eq(
                lines,
                page.header_variant(index).map_or(original, |v| v.lines())
            ));
            if copy {
                repeated += 1;
            } else {
                semantic += 1;
            }
            if let Some(column) = page.fragment_column(index) {
                body_columns.insert(column);
            }
            if let Some(variant) = page.header_variant(index) {
                assert!(copy);
                assert_eq!(variant.fragment_index(), index);
                variants += 1;
            }
            let mut consume_math = |owner: u32| {
                if copy {
                    repeated_math += 1;
                } else {
                    assert!(actual_math.insert(owner), "duplicate semantic math owner");
                }
            };
            if let Source::ParagraphLine {
                paragraph_index,
                line_index,
            } = fragment.source()
            {
                let paragraph = &lines.paragraphs()[paragraph_index as usize];
                assert_eq!(paragraph.owner(), fragment.owner());
                let line = &paragraph.lines()[line_index as usize];
                assert_eq!(
                    fragment.baseline(),
                    Some(fragment.bounds().y().checked_add(line.baseline()).unwrap())
                );
                for inline in line.items() {
                    match inline {
                        typaxis_layout::ProductionPlacedInline::Vector(selected) => {
                            let item = selected.occurrence().item();
                            let receipt = lines
                                .prepared()
                                .vector_bindings()
                                .unwrap()
                                .receipt(item.node_id())
                                .unwrap();
                            assert_eq!(item.paragraph_node(), fragment.owner());
                            assert_eq!(item.source_span(), receipt.owner_source_span());
                            assert_eq!(item.binding_fingerprint(), receipt.binding_fingerprint());
                            if receipt.kind() == typaxis_syntax::PrecomposedVectorKind::MathVector {
                                consume_math(item.node_id().get());
                            }
                        }
                        typaxis_layout::ProductionPlacedInline::BookV2Math(selected) => {
                            let receipt = lines
                                .prepared()
                                .native_math()
                                .unwrap()
                                .receipt(selected.owner())
                                .unwrap();
                            assert!(std::ptr::eq(receipt, selected.receipt()));
                            assert_eq!(selected.source_span(), receipt.source().domain().span);
                            assert_eq!(selected.baseline(), line.baseline());
                            consume_math(selected.owner().get());
                        }
                        typaxis_layout::ProductionPlacedInline::Math(_) => {
                            panic!("foreign math selection in Book /2")
                        }
                        _ => {}
                    }
                }
                if !copy {
                    assert!(std::ptr::eq(lines, original));
                    assert!(
                        actual.insert((fragment.owner().get(), line_index as usize)),
                        "duplicate original line"
                    );
                }
                if let Some(variant) = page.header_variant(index) {
                    let selected = &paragraph.selected().unwrap().lines()[line_index as usize];
                    variant_widths.insert(selected.inline_size().get().raw());
                    let key = (
                        page_index,
                        page.fragment_column(index),
                        variant.table_index(),
                        placed.definition_index(),
                        paragraph.owner().get(),
                    );
                    let units = copies.entry(key).or_default();
                    for unit in selected.line().start_unit()..selected.line().end_unit() {
                        assert!(
                            units.insert(unit),
                            "duplicate unit in one actual header copy"
                        );
                    }
                }
            } else if let Source::NativeMathBlock { block_index } = fragment.source() {
                let native = lines.prepared().native_math().unwrap();
                let block = &native.display_blocks()[block_index as usize];
                let receipt = native.receipt(fragment.owner()).unwrap();
                assert_eq!(block.owner(), fragment.owner());
                assert_eq!(block.source_span(), receipt.source().domain().span);
                assert_eq!(
                    fragment.baseline(),
                    Some(
                        fragment
                            .viewport()
                            .unwrap()
                            .y()
                            .checked_add(block.baseline())
                            .unwrap()
                    )
                );
                consume_math(fragment.owner().get());
            } else if let Source::VectorBlock { .. } = fragment.source() {
                let receipt = lines
                    .prepared()
                    .vector_bindings()
                    .unwrap()
                    .receipt(fragment.owner())
                    .unwrap();
                if receipt.kind() == typaxis_syntax::PrecomposedVectorKind::MathVectorBlock {
                    consume_math(fragment.owner().get());
                }
            }
        }
        assert!(closed
            .fragment_lines(page_index, page.fragments().len())
            .is_none());
    }
    assert!(closed.fragment_lines(geometry.pages().len(), 0).is_none());
    assert_eq!(
        actual, expected,
        "every demanded original paragraph line is placed once"
    );
    let mut variant_units = 0;
    for ((_, _, _, _, owner), units) in copies {
        let paragraph = original
            .paragraphs()
            .iter()
            .find(|p| p.owner().get() == owner)
            .unwrap();
        let expected = paragraph
            .selected()
            .unwrap()
            .lines()
            .iter()
            .flat_map(|line| line.line().start_unit()..line.line().end_unit())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            units, expected,
            "actual repeated header preserves original source units"
        );
        variant_units += units.len();
    }
    assert_eq!(closed.semantic_fragments(), semantic);
    assert_eq!(closed.repeated_fragments(), repeated);
    assert_eq!(closed.header_variant_fragments(), variants);
    assert_eq!(closed.has_header_variants(), variants > 0);
    assert_eq!(closed.unreferenced_definitions(), unreferenced);
    assert_eq!(
        actual_math, demanded_math,
        "each demanded math owner uses its original receipt once"
    );
    assert_eq!(closed.semantic_math(), actual_math.len());
    assert_eq!(closed.repeated_math(), repeated_math);
    Coverage {
        semantic,
        repeated,
        variants,
        unreferenced,
        semantic_math: closed.semantic_math(),
        repeated_math: closed.repeated_math(),
        body_columns,
        variant_widths,
        variant_units,
    }
}

fn close_input(
    input: &PreparedBookV2Resources,
    authored: &Value,
    caps: &M4EffectiveResourceLimits,
) -> Coverage {
    let mut budget = BookV2ColumnPageBudget::new(caps, 1_000_000_000);
    let (result, observed, extra_work, records) = drive(
        input,
        caps,
        MODE,
        &mut budget,
        |stable, search, _, observed| {
            let before = search.work_steps();
            let placed = search.place_column_pages(stable.sequence()).unwrap();
            let closed = search.close_column_page_sources(stable, &placed).unwrap();
            assert!(std::ptr::eq(closed.stable(), stable));
            assert!(std::ptr::eq(closed.geometry(), &placed));
            let result = coverage(&closed, search, authored);
            (
                result,
                observed,
                search.work_steps() - before,
                search.record_charge(),
            )
        },
    )
    .unwrap();
    assert!(extra_work > 0);
    assert_eq!(
        budget.observation().work_steps(),
        observed.work_steps() + extra_work
    );
    assert_eq!(budget.observation().record_charge(), records);
    result
}

#[test]
fn book_v2_column_source_closure_covers_body_notes_unreferenced_and_empty_units() {
    let caps = caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro";
    for (notes, empty) in [(false, false), (true, false), (false, true)] {
        let root = Root::new();
        let data = plain_data(text, notes, empty);
        let input = physical_input(&root, data.clone(), text, None, &caps);
        let result = close_input(&input, &data, &caps);
        assert!(result.semantic >= 6 && result.repeated == 0);
        assert_eq!(result.unreferenced, usize::from(notes));
        assert_eq!((result.semantic_math, result.repeated_math), (0, 0));
        if !empty {
            assert_eq!(result.body_columns, BTreeSet::from([0, 1]));
        }
        assert_eq!(close_input(&input, &data, &caps), result);
    }
}

#[test]
fn book_v2_column_source_closure_consumes_authored_body_and_caption_breaks() {
    let caps = caps();
    let text = "Pro";
    let mut body = plain_data(text, false, false);
    let paragraph = body["document"]["blocks"][0].clone();
    let command = json!({"kind":"page_break","node_id":0,"classes":[],"span":paragraph["span"]});
    body["document"]["blocks"] = json!([command, command, paragraph, command]);
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut body["document"],
        &mut 0,
    );
    let mut fixtures = vec![(body, 3usize)];
    let mut notes = plain_data(text, true, false);
    let paragraph = notes["document"]["footnotes"][0]["blocks"][0].clone();
    let command = json!({"kind":"page_break","node_id":0,"classes":[],"span":paragraph["span"]});
    notes["document"]["footnotes"][0]["blocks"] = json!([command, paragraph, command, command]);
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut notes["document"],
        &mut 0,
    );
    fixtures.push((notes, 0));
    for (headers, only_breaks) in [(false, false), (true, false), (false, true)] {
        let mut data = crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::caption_breaks_with_text(headers, only_breaks, text);
        for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
            master["column_layout"] =
                json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"none"});
        }
        fixtures.push((data, if only_breaks { 2 } else { 4 }));
    }
    for (data, expected_breaks) in fixtures {
        let root = Root::new();
        let input = physical_input(&root, data.clone(), text, None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        drive(&input, &caps, MODE, &mut budget, |stable, search, _, _| {
            let breaks = stable
                .sequence()
                .pages()
                .iter()
                .filter(|p| p.forced_break().is_some())
                .count();
            assert_eq!(breaks, expected_breaks);
            let placed = search.place_column_pages(stable.sequence()).unwrap();
            let closed = search.close_column_page_sources(stable, &placed).unwrap();
            coverage(&closed, search, &data);
        })
        .unwrap();
    }
}

#[test]
fn book_v2_column_source_closure_preserves_width_variant_nested_rowspan_and_caption_units() {
    let caps = changed_caps(&caps(), |c| {
        c.max_line_reshape_passes = 512;
        c.max_layout_passes = 512;
    });
    let text = "Pro Pro Pro Pro Pro Pro Pro";
    for notes in [false, true] {
        for (mode, height) in [
            ("common", 256),
            ("span", 256),
            ("nested-body", 512),
            ("nested-body-span", 544),
            ("nested-body-caption", 512),
        ] {
            let root = Root::new();
            let mut data = super::super::super::super::data(notes, mode, text);
            for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
                master["body"]["height"] = (height * 65536).into();
                if notes {
                    master["footnote"]["height"] = (height * 65536).into();
                }
            }
            let input = physical_input(&root, data.clone(), text, None, &caps);
            let result = close_input(&input, &data, &caps);
            assert!(
                result.repeated > 0 && result.variants > 0 && result.variant_units > 0,
                "{notes}/{mode}: {result:?}"
            );
            assert!(
                result.variant_widths.len() >= 2,
                "actual header line widths: {notes}/{mode}: {result:?}"
            );
            assert_eq!(
                (
                    result.unreferenced,
                    result.semantic_math,
                    result.repeated_math
                ),
                (0, 0, 0)
            );
        }
    }
}

#[test]
fn book_v2_column_source_closure_counts_vector_and_native_math_in_body_tables_and_notes() {
    let caps = caps();
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for notes in [false, true] {
            for tables in [false, true] {
                let root = Root::new();
                let data =
                    super::super::super::super::super::super::block_widths::column_block_data(
                        kind, notes, tables,
                    );
                let input = match kind {
                    "vector" => vector_input(&root, data.clone(), &caps),
                    "native" => {
                        let source = data["text_buffers"][0]["utf8"].as_str().unwrap().as_bytes();
                        native_input_with_source(&root, data.clone(), source, &caps)
                    }
                    "png" => figure_input(&root, data.clone(), PNG, &caps),
                    "jpeg" => figure_input(&root, data.clone(), JPEG, &caps),
                    "svg" => figure_input(&root, data.clone(), SVG, &caps),
                    _ => unreachable!(),
                };
                let result = close_input(&input, &data, &caps);
                assert_eq!(
                    result.semantic_math,
                    match kind {
                        "vector" | "native" =>
                            if notes {
                                3
                            } else {
                                6
                            },
                        _ => 0,
                    },
                    "{kind}/{notes}/{tables}: {result:?}"
                );
                assert_eq!((result.repeated_math, result.unreferenced), (0, 0));
            }
        }
    }
}

#[test]
fn book_v2_column_source_closure_rejects_foreign_search_and_equal_but_distinct_stability() {
    let root = Root::new();
    let caps = caps();
    let text = "Pro";
    let input = physical_input(&root, plain_data(text, false, false), text, None, &caps);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 100_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &caps).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &caps).unwrap();
    let mut allowance = BookV2BodyLineBudget::new(100_000_000, 128);
    with_budgeted_book_v2_column_lines(&policy, &flow, input.resources(), &bindings, &caps, MODE, None, &mut allowance, &plan, None, |lines| {
        let mut records = lines.retained_record_charge();
        let source = prepare_book_v2_column_flow_counted(&lines, None, &caps, records, &mut records).unwrap();
        let measured = prepare_book_v2_column_table_measurements_counted(source, &caps, &mut records).unwrap();
        let mut a = prepare_book_v2_column_page_search_counted(&measured, &caps, 100_000_000, records, &mut 0, &mut 0).unwrap();
        let mut b = prepare_book_v2_column_page_search_counted(&measured, &caps, 100_000_000, records, &mut 0, &mut 0).unwrap();
        let first = a.select_stable_column_pages_counted(128, &mut 0).unwrap();
        let second = a.select_stable_column_pages_counted(128, &mut 0).unwrap();
        let placed = a.place_column_pages(first.sequence()).unwrap();
        let before = (b.work_steps(), b.record_charge());
        assert!(matches!(b.close_column_page_sources(&first, &placed), Err(e) if e.kind == E::ReceiptMismatch));
        assert_eq!(before, (b.work_steps(), b.record_charge()));
        let before = (a.work_steps(), a.record_charge());
        assert!(matches!(a.close_column_page_sources(&second, &placed), Err(e) if e.kind == E::ReceiptMismatch));
        assert_eq!(before, (a.work_steps(), a.record_charge()));
        let closed = a.close_column_page_sources(&first, &placed).unwrap();
        assert_eq!(closed.semantic_fragments(), 6);
    }).unwrap();
}

#[test]
fn book_v2_column_source_closure_retains_exact_callback_budgets_and_failed_prefixes() {
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro";
    let data = plain_data(text, true, false);
    let base = caps();
    let evaluate = |caps: &M4EffectiveResourceLimits, maximum| {
        let root = Root::new();
        let input = physical_input(&root, data.clone(), text, None, caps);
        let mut budget = BookV2ColumnPageBudget::new(caps, maximum);
        let mut called = false;
        let result = drive(&input, caps, MODE, &mut budget, |stable, search, _, _| {
            called = true;
            let placed = search.place_column_pages(stable.sequence()).unwrap();
            let before = (search.work_steps(), search.record_charge());
            let first = search.close_column_page_sources(stable, &placed);
            let accepted = first.is_ok();
            if accepted {
                assert!(search.work_steps() > before.0 && search.record_charge() > before.1);
            }
            if !accepted {
                let prefix = (search.work_steps(), search.record_charge());
                assert!(prefix.0 >= before.0 && prefix.1 >= before.1);
                assert!(search.close_column_page_sources(stable, &placed).is_err());
                assert!(search.work_steps() >= prefix.0 && search.record_charge() >= prefix.1);
            }
            accepted
        })
        .map_err(|e| format!("{e:?}"));
        (result, budget.observation(), called)
    };
    let full = evaluate(&base, 100_000_000);
    assert_eq!(full.0.as_ref().ok(), Some(&true));
    assert_eq!(evaluate(&base, full.1.work_steps()), full);
    let short = evaluate(&base, full.1.work_steps() - 1);
    assert_eq!(short.0.as_ref().ok(), Some(&false));
    assert!(short.2 && short.1.work_steps() <= full.1.work_steps() - 1);
    let exact = changed_caps(&base, |c| c.max_fragments = full.1.record_charge());
    assert_eq!(evaluate(&exact, 100_000_000).0.as_ref().ok(), Some(&true));
    let short = changed_caps(&base, |c| c.max_fragments = full.1.record_charge() - 1);
    let result = evaluate(&short, 100_000_000);
    assert_eq!(result.0.as_ref().ok(), Some(&false));
    assert!(result.2 && result.1.record_charge() <= short.base().get().max_fragments);
}

#[test]
fn book_v2_column_source_driver_delivers_only_complete_closures_and_charges_callbacks() {
    use crate::book_v2_resources::with_budgeted_book_v2_column_sources as source_drive;
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro";
    let data = plain_data(text, true, false);
    let base = caps();
    let evaluate = |caps: &M4EffectiveResourceLimits, maximum, extra: bool, retry: bool| {
        let root = Root::new();
        let input = physical_input(&root, data.clone(), text, None, caps);
        let mut budget = BookV2ColumnPageBudget::new(caps, maximum);
        let mut called = false;
        let mut delivered = None;
        let result = source_drive(
            &input,
            caps,
            MODE,
            &mut budget,
            |closed, search, observed| {
                called = true;
                assert_eq!(observed.record_charge(), search.record_charge());
                let snapshot = coverage(closed, search, &data);
                let before = search.work_steps();
                if extra {
                    search
                        .place_column_pages(closed.stable().sequence())
                        .unwrap();
                }
                delivered = Some((
                    observed,
                    search.work_steps() - before,
                    search.record_charge(),
                ));
                snapshot
            },
        )
        .map_err(|e| format!("{e:?}"));
        if let Some((observed, extra, records)) = delivered {
            assert_eq!(
                budget.observation().work_steps(),
                observed.work_steps() + extra
            );
            assert_eq!(budget.observation().record_charge(), records);
        }
        if retry {
            assert!(result.is_err() && !called);
            let prefix = budget.observation();
            assert!(
                source_drive(&input, caps, MODE, &mut budget, |_, _, _| panic!(
                    "failed closure delivered"
                ))
                .is_err()
            );
            assert!(budget.observation().work_steps() >= prefix.work_steps());
            assert!(budget.observation().record_charge() >= prefix.record_charge());
            assert!(budget.observation().work_steps() <= maximum);
            assert!(budget.observation().record_charge() <= caps.base().get().max_fragments);
        }
        (result, budget.observation(), called)
    };
    let full = evaluate(&base, 100_000_000, false, false);
    assert!(full.0.is_ok() && full.2);
    assert_eq!(evaluate(&base, full.1.work_steps(), false, false), full);
    let short = evaluate(&base, full.1.work_steps() - 1, false, true);
    assert!(short.0.is_err() && !short.2);
    assert!(short
        .0
        .as_ref()
        .unwrap_err()
        .contains("column source closure"));
    let exact = changed_caps(&base, |c| c.max_fragments = full.1.record_charge());
    assert_eq!(evaluate(&exact, 100_000_000, false, false).0, full.0);
    let short = changed_caps(&base, |c| c.max_fragments = full.1.record_charge() - 1);
    let refused = evaluate(&short, 100_000_000, false, true);
    assert!(refused.0.is_err() && !refused.2);
    assert!(refused
        .0
        .as_ref()
        .unwrap_err()
        .contains("column source closure"));
    let extra = evaluate(&base, 100_000_000, true, false);
    assert_eq!(extra.0, full.0);
    assert!(extra.1.work_steps() > full.1.work_steps());
    assert!(extra.1.record_charge() > full.1.record_charge());
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_column_source_closure_covers_original_harano_units_and_notes() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let caps = caps();
    let text =
        "元の字形を保って段の元単位を一度ずつ閉じる元の字形を保って段の元単位を一度ずつ閉じる";
    for notes in [false, true] {
        let root = Root::new();
        let data = plain_data(text, notes, false);
        let input = physical_input(&root, data.clone(), text, Some(&font), &caps);
        let result = close_input(&input, &data, &caps);
        assert_eq!(result.unreferenced, usize::from(notes));
        assert_eq!(result.body_columns, BTreeSet::from([0, 1]));
    }
}
