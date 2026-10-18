use super::*;
#[path = "book_v2_column_number_storage_tests.rs"]
mod number_storage;
use crate::book_v2_resources::tests::shaping_tests::native_tests::{
    header_terminals::{header_fixture, renumber as renumber_header},
    native_input,
};
use crate::book_v2_resources::with_budgeted_book_v2_column_math as math_drive;
use typaxis_layout::{PrecomposedVectorPlacementInput, ProductionPlacedInline};
use typaxis_pagination::book_v2::{
    BookV2BodyMathSource, BookV2ColumnMathTerminals, BOOK_V2_COLUMN_MATH_TERMINAL_ALGORITHM,
};

#[derive(Debug, PartialEq)]
pub(super) struct Terminals {
    hash: [u8; 32],
    semantic: usize,
    repeated: usize,
    numbers: usize,
    empty_columns: usize,
    columns: BTreeSet<u16>,
    notes: usize,
    headers: usize,
    spool: u64,
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
fn assert_rect(bytes: &[u8], offset: usize, rect: typaxis_core::Rect) {
    for (i, value) in [rect.x(), rect.y(), rect.width().get(), rect.height().get()]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            i64::from_be_bytes(
                bytes[offset + i * 8..offset + (i + 1) * 8]
                    .try_into()
                    .unwrap()
            ),
            value.raw()
        );
    }
}
fn assert_optional_rect(bytes: &[u8], offset: usize, rect: Option<typaxis_core::Rect>) {
    assert_eq!(bytes[offset], u8::from(rect.is_some()));
    if let Some(rect) = rect {
        assert_rect(bytes, offset + 1, rect);
    } else {
        assert_eq!(&bytes[offset + 1..offset + 33], &[0; 32]);
    }
}

fn spool_limits(limits: &M4EffectiveResourceLimits, maximum: u64) -> M4EffectiveResourceLimits {
    let mut base = limits.base().get().clone();
    base.max_spool_bytes = maximum;
    let mut extension = limits.extension().get().clone();
    extension.max_font_subset_bytes = extension.max_font_subset_bytes.min(maximum);
    M4EffectiveResourceLimits::new(ValidatedResourceLimits::new(base).unwrap(), extension).unwrap()
}

fn spool_vector_input(
    root: &Root,
    data: Value,
    limits: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    let source = data["text_buffers"][0]["utf8"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let body = body_with_source(root, data, &source, limits);
    fs::write(root.0.join("vector.svg"), include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/x-plus-y.svg"))).unwrap();
    prepare_book_v2_resources(
        body,
        &root.context(),
        &config_with_extension(
            limits.base().get().clone(),
            limits.extension().get().clone(),
        ),
        limits,
    )
    .unwrap()
}

/// Decode the column format independently and join each occurrence to its
/// actual physical leaf, original receipt and borrowed header line graph.
pub(super) fn inspect(value: &BookV2ColumnMathTerminals<'_, '_, '_, '_, '_, '_, '_>) -> Terminals {
    let bytes = value.canonical_bytes();
    assert_eq!(
        &bytes[..32],
        &typaxis_core::sha256(BOOK_V2_COLUMN_MATH_TERMINAL_ALGORITHM.as_bytes())
    );
    assert_eq!(value.fingerprint(), typaxis_core::sha256(bytes));
    let source = value.source();
    let pages = source.geometry().pages();
    let numbers = pages
        .iter()
        .flat_map(|p| p.equation_numbers())
        .collect::<Vec<_>>();
    let fragments = pages
        .iter()
        .enumerate()
        .flat_map(|(pi, page)| {
            page.fragments_with_roles()
                .enumerate()
                .map(move |(fi, (placed, role, repeated))| {
                    (pi, fi, placed, role, repeated, page.fragment_column(fi))
                })
        })
        .collect::<Vec<_>>();
    for (offset, expected) in [
        (288, pages.len()),
        (296, source.semantic_math()),
        (304, source.repeated_math()),
        (312, source.unreferenced_definitions()),
        (320, numbers.len()),
        (328, fragments.len()),
    ] {
        assert_eq!(u64_at(bytes, offset) as usize, expected);
    }
    assert_eq!(value.fragment_count(), fragments.len());
    assert!(value.fragment_lines(fragments.len()).is_none());
    let mut semantic = BTreeSet::new();
    let mut repeated = 0;
    let mut columns = BTreeSet::new();
    let mut notes = 0;
    for (index, terminal) in value.terminals().iter().enumerate() {
        let (pi, fi, placed, role, copy, column) = fragments[terminal.fragment_index()];
        let fragment = placed.fragment();
        let lines = value.fragment_lines(terminal.fragment_index()).unwrap();
        assert!(std::ptr::eq(lines, source.fragment_lines(pi, fi).unwrap()));
        assert_eq!(terminal.page_index(), fragment.page_index());
        assert_eq!(terminal.column_index(), column);
        assert_eq!(terminal.definition_index(), placed.definition_index());
        assert_eq!(terminal.item_index(), placed.item_index());
        assert_eq!(terminal.cell_owner(), role.map(|r| r.owner()));
        assert_eq!(terminal.repeated_header(), copy);
        if copy {
            repeated += 1;
        } else {
            assert!(semantic.insert(terminal.source().owner()));
        }
        if let Some(column) = column {
            columns.insert(column);
        } else {
            notes += 1;
        }
        let record = &bytes[336 + index * 129..336 + (index + 1) * 129];
        assert_eq!(
            record[0],
            u8::from(matches!(terminal.source(), BookV2BodyMathSource::Native(_)))
        );
        assert_eq!(
            u32::from_be_bytes(record[1..5].try_into().unwrap()),
            terminal.source().owner().get()
        );
        assert_eq!(
            u32::from_be_bytes(record[5..9].try_into().unwrap()),
            terminal.page_index()
        );
        assert_eq!(u64_at(record, 9) as usize, terminal.fragment_index());
        assert_eq!(record[17], u8::from(terminal.definition_index().is_some()));
        assert_eq!(
            u64_at(record, 18) as usize,
            terminal.definition_index().unwrap_or(0)
        );
        assert_eq!(u64_at(record, 26) as usize, terminal.item_index());
        assert_eq!(record[34], u8::from(terminal.inline_index().is_some()));
        assert_eq!(
            u32::from_be_bytes(record[35..39].try_into().unwrap()),
            terminal.inline_index().unwrap_or(0)
        );
        assert_eq!(record[39], u8::from(terminal.cell_owner().is_some()));
        assert_eq!(
            u32::from_be_bytes(record[40..44].try_into().unwrap()),
            terminal.cell_owner().map_or(0, |c| c.get())
        );
        assert_eq!(record[44], u8::from(copy));
        assert_eq!(
            i64::from_be_bytes(record[45..53].try_into().unwrap()),
            terminal.origin_x().raw()
        );
        assert_eq!(
            i64::from_be_bytes(record[53..61].try_into().unwrap()),
            terminal.baseline().raw()
        );
        assert_eq!(record[61], u8::from(terminal.viewport().is_some()));
        if let Some(rect) = terminal.viewport() {
            assert_rect(record, 62, rect);
        } else {
            assert_eq!(&record[62..94], &[0; 32]);
        }
        assert_eq!(&record[94..126], &terminal.source().fingerprint());
        assert_eq!(record[126], u8::from(column.is_some()));
        assert_eq!(
            u16::from_be_bytes(record[127..129].try_into().unwrap()),
            column.unwrap_or(0)
        );
        match fragment.source() {
            Source::ParagraphLine {
                paragraph_index,
                line_index,
            } => {
                let line =
                    &lines.paragraphs()[paragraph_index as usize].lines()[line_index as usize];
                let item = &line.items()[terminal.inline_index().unwrap() as usize];
                match (item, terminal.source()) {
                    (
                        ProductionPlacedInline::Vector(selected),
                        BookV2BodyMathSource::Vector(receipt),
                    ) => {
                        let bound = lines
                            .prepared()
                            .vector_bindings()
                            .unwrap()
                            .receipt(selected.occurrence().item().node_id())
                            .unwrap();
                        assert!(std::ptr::eq(bound, receipt));
                        let local = selected.geometry();
                        assert_eq!(
                            terminal.origin_x(),
                            fragment
                                .bounds()
                                .x()
                                .checked_add(local.pen_origin_x())
                                .unwrap()
                        );
                        assert_eq!(
                            terminal.baseline(),
                            fragment
                                .bounds()
                                .y()
                                .checked_add(local.line_baseline_y())
                                .unwrap()
                        );
                        let actual = terminal.viewport().unwrap();
                        assert_eq!(
                            actual.x(),
                            fragment
                                .bounds()
                                .x()
                                .checked_add(local.viewport().x())
                                .unwrap()
                        );
                        assert_eq!(
                            actual.y(),
                            fragment
                                .bounds()
                                .y()
                                .checked_add(local.viewport().y())
                                .unwrap()
                        );
                        assert_eq!(
                            (actual.width(), actual.height()),
                            (local.viewport().width(), local.viewport().height())
                        );
                    }
                    (
                        ProductionPlacedInline::BookV2Math(selected),
                        BookV2BodyMathSource::Native(receipt),
                    ) => {
                        assert!(std::ptr::eq(receipt, selected.receipt()));
                        assert_eq!(
                            terminal.origin_x(),
                            fragment.bounds().x().checked_add(selected.pen_x()).unwrap()
                        );
                        assert_eq!(
                            terminal.baseline(),
                            fragment
                                .bounds()
                                .y()
                                .checked_add(selected.baseline())
                                .unwrap()
                        );
                        assert!(terminal.viewport().is_none());
                    }
                    _ => panic!("wrong original inline math receipt"),
                }
            }
            Source::VectorBlock { .. } => {
                let BookV2BodyMathSource::Vector(receipt) = terminal.source() else {
                    panic!("wrong vector block");
                };
                assert_eq!(receipt.node_id(), fragment.owner());
                let PrecomposedVectorPlacementInput::MathVectorBlock(input) = receipt.placement()
                else {
                    panic!("wrong math kind");
                };
                assert_eq!(terminal.viewport(), fragment.viewport());
                assert_eq!(
                    terminal.origin_x(),
                    fragment
                        .viewport()
                        .unwrap()
                        .x()
                        .checked_sub(input.metrics().origin_x())
                        .unwrap()
                );
                assert_eq!(Some(terminal.baseline()), fragment.baseline());
                assert!(terminal.inline_index().is_none());
            }
            Source::NativeMathBlock { block_index } => {
                let BookV2BodyMathSource::Native(receipt) = terminal.source() else {
                    panic!("wrong native block");
                };
                let native = lines.prepared().native_math().unwrap();
                assert!(std::ptr::eq(
                    receipt,
                    native.receipt(fragment.owner()).unwrap()
                ));
                assert_eq!(
                    terminal.origin_x(),
                    fragment
                        .viewport()
                        .unwrap()
                        .x()
                        .checked_sub(native.display_blocks()[block_index as usize].left())
                        .unwrap()
                );
                assert_eq!(Some(terminal.baseline()), fragment.baseline());
                assert!(terminal.viewport().is_none() && terminal.inline_index().is_none());
            }
            _ => panic!("nonformula leaf produced a terminal"),
        }
    }
    assert_eq!(semantic.len(), source.semantic_math());
    assert_eq!(repeated, source.repeated_math());
    let mut offset = 336 + value.terminals().len() * 129;
    for number in &numbers {
        let n = number.geometry();
        let (_, _, placed, _, copy, _) = fragments[n.fragment_index() as usize];
        assert_eq!(placed.fragment().owner(), n.parent_owner());
        assert_eq!(copy, number.repeated_header());
        assert_eq!(
            u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()),
            n.owner().get()
        );
        assert_eq!(
            u32::from_be_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()),
            n.parent_owner().get()
        );
        assert_eq!(
            u32::from_be_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()),
            n.fragment_index()
        );
        assert_eq!(
            u32::from_be_bytes(bytes[offset + 12..offset + 16].try_into().unwrap()),
            n.page_index()
        );
        assert_rect(bytes, offset + 16, n.bounds());
        assert_eq!(&bytes[offset + 48..offset + 80], &n.shape_fingerprint());
        assert_eq!(bytes[offset + 80], u8::from(copy));
        offset += 81;
    }
    if source.has_header_variants() {
        assert_eq!(
            &bytes[offset..offset + 32],
            &typaxis_core::sha256(b"typaxis.book-2-math-terminal-header-owners/1")
        );
        assert_eq!(
            u64_at(bytes, offset + 32) as usize,
            source.header_variant_fragments()
        );
        offset += 40;
        for page in pages {
            for variant in page.header_variants() {
                assert_eq!(
                    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()),
                    page.selection().page_index()
                );
                assert_eq!(u64_at(bytes, offset + 4) as usize, variant.fragment_index());
                assert_eq!(
                    u64_at(bytes, offset + 12) as usize,
                    variant.global_item_index()
                );
                assert_eq!(&bytes[offset + 20..offset + 52], &variant.fingerprint());
                assert_eq!(
                    &bytes[offset + 52..offset + 84],
                    &variant.lines().fingerprint()
                );
                offset += 180;
            }
        }
    }
    let mut empty_columns = 0;
    for page in pages {
        let frames = page.selection().candidate().frames();
        assert_eq!(
            u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()),
            frames.page_index()
        );
        assert_eq!(
            bytes[offset + 4],
            u8::from(frames.named_page_index().is_some())
        );
        assert_eq!(
            u64_at(bytes, offset + 5) as usize,
            frames.named_page_index().unwrap_or(0)
        );
        assert_rect(bytes, offset + 13, frames.body());
        assert_optional_rect(bytes, offset + 45, frames.footnote());
        assert_optional_rect(
            bytes,
            offset + 78,
            page.selection().candidate().footnote_bounds(),
        );
        assert_eq!(u64_at(bytes, offset + 111) as usize, page.columns().len());
        assert_optional_rect(bytes, offset + 119, page.separator_ink());
        offset += 152;
        for (ci, column) in page.columns().iter().enumerate() {
            let original = frames.column(ci as u16).unwrap();
            let expected = page.selection().balance().map_or(original, |balance| {
                typaxis_core::Rect::new(
                    original.x(),
                    original.y(),
                    original.width(),
                    balance.selected_height(),
                )
            });
            assert_eq!(column.bounds(), expected);
            assert_rect(bytes, offset, column.bounds());
            assert_eq!(
                u64_at(bytes, offset + 32) as usize,
                column.fragments().start
            );
            assert_eq!(u64_at(bytes, offset + 40) as usize, column.fragments().end);
            empty_columns += usize::from(column.fragments().is_empty());
            offset += 48;
        }
    }
    let balanced = pages
        .iter()
        .filter_map(|page| page.selection().balance().map(|balance| (page, balance)))
        .collect::<Vec<_>>();
    if !balanced.is_empty() {
        assert_eq!(
            &bytes[offset..offset + 32],
            &typaxis_core::sha256(b"typaxis.book-2-column-math-balance/1")
        );
        assert_eq!(u64_at(bytes, offset + 32) as usize, balanced.len());
        offset += 40;
        for (page, balance) in balanced {
            assert_eq!(
                u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()),
                page.selection().page_index()
            );
            assert_eq!(&bytes[offset + 4..offset + 36], &balance.fingerprint());
            assert_eq!(
                balance.fingerprint(),
                typaxis_core::sha256(balance.canonical_bytes())
            );
            assert_eq!(
                i64::from_be_bytes(bytes[offset + 36..offset + 44].try_into().unwrap()),
                balance.selected_height().get().raw()
            );
            offset += 44;
        }
    }
    assert_eq!(offset, bytes.len());
    Terminals {
        hash: value.fingerprint(),
        semantic: semantic.len(),
        repeated,
        numbers: numbers.len(),
        empty_columns,
        columns,
        notes,
        headers: source.header_variant_fragments(),
        spool: value.spool_charge(),
    }
}

#[test]
fn book_v2_column_math_terminals_keep_original_inline_block_number_and_note_geometry() {
    let limits = caps();
    for kind in ["vector", "native"] {
        for notes in [false, true] {
            for tables in [false, true] {
                let root = Root::new();
                let data =
                    super::super::super::super::super::super::block_widths::column_block_data(
                        kind, notes, tables,
                    );
                let input = if kind == "vector" {
                    vector_input(&root, data.clone(), &limits)
                } else {
                    let source = data["text_buffers"][0]["utf8"].as_str().unwrap().as_bytes();
                    native_input_with_source(&root, data.clone(), source, &limits)
                };
                let mut budget = BookV2ColumnPageBudget::new(&limits, 100_000_000);
                let first = math_drive(
                    &input,
                    &limits,
                    MODE,
                    &mut budget,
                    |terminals, search, observed| {
                        assert_eq!(terminals.record_charge(), search.record_charge());
                        assert_eq!(terminals.work_steps(), search.work_steps());
                        assert_eq!(terminals.spool_charge(), search.terminal_spool_charge());
                        assert_eq!(observed.spool_charge(), terminals.spool_charge());
                        inspect(terminals)
                    },
                )
                .unwrap();
                assert_eq!(first.semantic, if notes { 3 } else { 6 });
                assert!(first.empty_columns > 0 && first.spool > 0);
                if notes {
                    assert!(first.notes > 0);
                }
                if kind == "vector" {
                    assert!(first.numbers > 0);
                }
                let mut repeated_budget = BookV2ColumnPageBudget::new(&limits, 100_000_000);
                let repeated =
                    math_drive(&input, &limits, MODE, &mut repeated_budget, |t, _, _| {
                        inspect(t)
                    })
                    .unwrap();
                assert_eq!(first, repeated);
                assert_eq!(budget.observation(), repeated_budget.observation());
            }
        }
    }
}

pub(super) fn header_data(vector: bool, notes: bool, nested: bool) -> Value {
    let (mut data, _) = header_fixture(vector, false, false, notes);
    if nested {
        let table = if notes {
            &mut data["document"]["footnotes"][0]["blocks"][0]
        } else {
            &mut data["document"]["blocks"][0]
        };
        let child = table.clone();
        let mut parent = child.clone();
        parent["head"][0]["cells"][0]["blocks"] = child["body"][0]["cells"][0]["blocks"].clone();
        parent["body"].as_array_mut().unwrap().truncate(1);
        parent["body"][0]["cells"][0]["blocks"] = json!([child]);
        *table = parent;
    }
    renumber_header(&mut data["document"], &mut 0);
    let original = data["page_masters"]["masters"][0].clone();
    let mut masters = Vec::new();
    for (index, (id, width)) in [("a-wide", 180), ("b-first", 100), ("c-even", 140)]
        .into_iter()
        .enumerate()
    {
        let mut master = original.clone();
        let width = width + if vector && notes { 20 } else { 0 };
        master["master_id"] = id.into();
        master["width"] = (600 * 65536).into();
        master["trim"]["width"] = (600 * 65536).into();
        master["body"]["width"] = ((2 * width + 10) * 65536).into();
        master["body"]["x"] = ((20 + index * 7) * 65536).into();
        master["column_layout"] =
            json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"none"});
        if nested {
            master["body"]["height"] =
                (master["body"]["height"].as_i64().unwrap() + 32 * 65536).into();
        }
        if notes {
            master["footnote"]["width"] = (width * 65536).into();
            master["footnote"]["x"] = ((30 + index * 11) * 65536).into();
            if nested {
                master["footnote"]["height"] =
                    (master["footnote"]["height"].as_i64().unwrap() + 32 * 65536).into();
            }
        }
        masters.push(master);
    }
    data["page_masters"]["masters"] = masters.into();
    data["page_masters"]["default_master_id"] = "a-wide".into();
    data["page_masters"]["selection_rules"] = json!([
        {"master_id":"c-even","parity":"even","first":null,"named_page":null,"source_order":0},
        {"master_id":"b-first","parity":"any","first":true,"named_page":null,"source_order":1}]);
    data
}

#[test]
fn book_v2_column_math_terminals_resolve_actual_nested_repeated_header_graphs() {
    let limits = changed_caps(&caps(), |c| {
        c.max_line_reshape_passes = 512;
        c.max_layout_passes = 512;
    });
    for vector in [false, true] {
        for notes in [false, true] {
            for nested in [false, true] {
                let root = Root::new();
                let data = header_data(vector, notes, nested);
                let input = if vector {
                    vector_input(&root, data, &limits)
                } else {
                    native_input(&root, data, &limits)
                };
                let mut budget = BookV2ColumnPageBudget::new(&limits, 1_000_000_000);
                let result = math_drive(&input, &limits, MODE, &mut budget, |t, _, _| inspect(t));
                assert!(result.is_ok(), "{vector}/{notes}/{nested}: {result:?}");
                let result = result.unwrap();
                assert!(result.headers > 0 && result.repeated > 0);
                if vector {
                    assert!(result.numbers > 0);
                }
            }
        }
    }
}

#[test]
fn book_v2_column_math_terminal_driver_keeps_exact_work_records_spool_and_failures() {
    let baseline = caps();
    let mut data = super::super::super::super::super::super::block_widths::column_block_data(
        "vector", false, false,
    );
    // Enough actual formula pages to keep this boundary above the independently
    // enforced resource-ledger canonical buffer limit.
    let mut block = data["document"]["blocks"][0].clone();
    block["blocks"][2]["equation_number"] = Value::Null;
    let boundary = data["document"]["blocks"][1].clone();
    let mut blocks = Vec::new();
    for index in 0..9 {
        if index != 0 {
            blocks.push(boundary.clone());
        }
        blocks.push(block.clone());
    }
    data["document"]["blocks"] = blocks.into();
    renumber_header(&mut data["document"], &mut 0);
    let evaluate = |limits: &M4EffectiveResourceLimits, maximum| {
        let root = Root::new();
        let input = spool_vector_input(&root, data.clone(), limits);
        let mut budget = BookV2ColumnPageBudget::new(limits, maximum);
        let mut callback = false;
        let result = math_drive(&input, limits, MODE, &mut budget, |t, _, _| {
            callback = true;
            inspect(t)
        })
        .map_err(|e| format!("{e:?}"));
        (result, budget.observation(), callback)
    };
    let full = evaluate(&baseline, 100_000_000);
    eprintln!("column math full budgets: {:?}, {:?}", full.0, full.1);
    assert!(full.0.is_ok() && full.2);
    assert_eq!(evaluate(&baseline, full.1.work_steps()), full);
    let short = evaluate(&baseline, full.1.work_steps() - 1);
    assert!(short.0.is_err() && !short.2);
    assert!(short.1.work_steps() <= full.1.work_steps() - 1 && short.1.spool_charge() > 0);
    let exact = changed_caps(&baseline, |c| c.max_fragments = full.1.record_charge());
    assert!(evaluate(&exact, 100_000_000).0.is_ok());
    let short = changed_caps(&baseline, |c| c.max_fragments = full.1.record_charge() - 1);
    let failed = evaluate(&short, 100_000_000);
    assert!(
        failed.0.is_err()
            && !failed.2
            && failed.1.record_charge() <= short.base().get().max_fragments
    );
    let exact = spool_limits(&baseline, full.1.spool_charge());
    let exact = evaluate(&exact, 100_000_000);
    assert!(exact.0.is_ok(), "{exact:?}");
    let short = spool_limits(&baseline, full.1.spool_charge() - 1);
    let root = Root::new();
    let input = spool_vector_input(&root, data.clone(), &short);
    let mut budget = BookV2ColumnPageBudget::new(&short, 100_000_000);
    for _ in 0..2 {
        let before = budget.observation();
        let mut called = false;
        assert!(math_drive(&input, &short, MODE, &mut budget, |_, _, _| {
            called = true;
        })
        .is_err());
        let after = budget.observation();
        assert!(
            !called
                && after.work_steps() > before.work_steps()
                && after.record_charge() > before.record_charge()
        );
        assert!(
            after.spool_charge() >= before.spool_charge()
                && after.spool_charge() <= short.base().get().max_spool_bytes
        );
    }
}

#[test]
fn book_v2_column_math_finalization_rejects_foreign_owner_and_oversized_prior_spool() {
    let root = Root::new();
    let limits = caps();
    let text = "Pro";
    let input = physical_input(&root, plain_data(text, false, false), text, None, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 100_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut allowance = BookV2BodyLineBudget::new(100_000_000, 128);
    with_budgeted_book_v2_column_lines(&policy,&flow,input.resources(),&bindings,&limits,MODE,None,&mut allowance,&plan,None,|stable| {
        let mut records=stable.retained_record_charge();
        let flow=prepare_book_v2_column_flow_counted(&stable,None,&limits,records,&mut records).unwrap();
        let measured=prepare_book_v2_column_table_measurements_counted(flow,&limits,&mut records).unwrap();
        let mut a=prepare_book_v2_column_page_search_counted(&measured,&limits,100_000_000,records,&mut 0,&mut 0).unwrap();
        let mut b=prepare_book_v2_column_page_search_counted(&measured,&limits,100_000_000,records,&mut 0,&mut 0).unwrap();
        let stable=a.select_stable_column_pages_counted(2,&mut 0).unwrap();
        let placed=a.place_column_pages(stable.sequence()).unwrap();
        let closure=a.close_column_page_sources(&stable,&placed).unwrap();
        assert!(matches!(b.finalize_column_page_math(closure,&limits,0),Err(e) if e.kind==E::ReceiptMismatch));
        assert_eq!(b.terminal_spool_charge(),0);
    }).unwrap();
    let mut budget = BookV2ColumnPageBudget::new(&limits, 100_000_000);
    math_drive(
        &input,
        &limits,
        MODE,
        &mut budget,
        |terminals, search, _| {
            let prior = search.terminal_spool_charge();
            let closed = search
                .close_column_page_sources(
                    terminals.source().stable(),
                    terminals.source().geometry(),
                )
                .unwrap();
            let result = search.finalize_column_page_math(closed, &limits, u64::MAX);
            assert!(
                matches!(&result, Err(e) if e.kind == E::SpoolLimit),
                "{:?}",
                result.err()
            );
            assert_eq!(search.terminal_spool_charge(), prior);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_math_finalization_rejects_preliminary_source_widths() {
    let root = Root::new();
    let limits = caps();
    let text = "Pro";
    let mut data = plain_data(text, false, false);
    for m in data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
    {
        let narrow = m["master_id"] == "c-first";
        m["body"]["width"] = ((if narrow { 190 } else { 450 }) * 65536).into();
        m["column_layout"] = json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"none"});
    }
    let input = physical_input(&root, data, text, None, &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 100_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    let mut allowance = BookV2BodyLineBudget::new(100_000_000, 128);
    with_budgeted_book_v2_column_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        MODE,
        None,
        &mut allowance,
        &plan,
        None,
        |stable| {
            let mut records = stable.retained_record_charge();
            let flow =
                prepare_book_v2_column_flow_counted(&stable, None, &limits, records, &mut records)
                    .unwrap();
            let measured =
                prepare_book_v2_column_table_measurements_counted(flow, &limits, &mut records)
                    .unwrap();
            let mut search = prepare_book_v2_column_page_search_counted(
                &measured,
                &limits,
                100_000_000,
                records,
                &mut 0,
                &mut 0,
            )
            .unwrap();
            let stable = search
                .select_stable_column_pages_counted(2, &mut 0)
                .unwrap();
            let geometry = search.place_column_pages(stable.sequence()).unwrap();
            let closed = search
                .close_column_page_sources(&stable, &geometry)
                .unwrap();
            let result = search.finalize_column_page_math(closed, &limits, 0);
            assert!(
                matches!(&result,Err(e) if e.kind==E::WidthMismatch),
                "{:?}",
                result.err()
            );
            assert_eq!(search.terminal_spool_charge(), 0);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_math_callbacks_retain_accepted_spool_after_errors_and_retries() {
    let limits = caps();
    let root = Root::new();
    let data = super::super::super::super::super::super::block_widths::column_block_data(
        "vector", false, false,
    );
    let input = vector_input(&root, data, &limits);
    for source_callback in [false, true] {
        let mut budget = BookV2ColumnPageBudget::new(&limits, 100_000_000);
        for _ in 0..2 {
            let before = budget.observation();
            let mut accepted_spool = 0;
            let result = if source_callback {
                crate::book_v2_resources::with_budgeted_book_v2_column_sources(
                    &input,
                    &limits,
                    MODE,
                    &mut budget,
                    |closed, search, observed| {
                        let closed = search
                            .close_column_page_sources(closed.stable(), closed.geometry())
                            .unwrap();
                        let math = search
                            .finalize_column_page_math(closed, &limits, observed.spool_charge())
                            .unwrap();
                        accepted_spool = math.spool_charge();
                        let _ = inspect(&math);
                        Err::<(), _>("callback failure")
                    },
                )
            } else {
                crate::book_v2_resources::with_budgeted_book_v2_column_placement(
                    &input,
                    &limits,
                    MODE,
                    &mut budget,
                    |stable, search, _, observed| {
                        let geometry = search.place_column_pages(stable.sequence()).unwrap();
                        let closed = search.close_column_page_sources(stable, &geometry).unwrap();
                        let math = search
                            .finalize_column_page_math(closed, &limits, observed.spool_charge())
                            .unwrap();
                        accepted_spool = math.spool_charge();
                        let _ = inspect(&math);
                        Err::<(), _>("callback failure")
                    },
                )
            };
            assert!(matches!(result, Ok(Err("callback failure"))));
            let after = budget.observation();
            assert_eq!(after.spool_charge(), accepted_spool);
            assert!(accepted_spool > before.spool_charge());
            assert!(
                after.work_steps() > before.work_steps()
                    && after.record_charge() > before.record_charge()
            );
        }
    }
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_column_math_terminals_keep_original_harano_header_and_note_receipts() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let limits = changed_caps(&caps(), |c| {
        c.max_line_reshape_passes = 512;
        c.max_layout_passes = 512;
    });
    for notes in [false, true] {
        let root = Root::new();
        let data = header_data(true, notes, true);
        let (_, text) = header_fixture(true, false, false, notes);
        let input =
            crate::book_v2_resources::tests::shaping_tests::vector_tests::vector_input_with_font(
                &root, data, &limits, &font, text,
            );
        let mut budget = BookV2ColumnPageBudget::new(&limits, 1_000_000_000);
        let result = math_drive(&input, &limits, MODE, &mut budget, |t, _, _| inspect(t)).unwrap();
        assert!(
            result.semantic > 0 && result.repeated > 0 && result.headers > 0 && result.numbers > 0
        );
        if notes {
            assert!(result.notes > 0);
        }
    }
}
