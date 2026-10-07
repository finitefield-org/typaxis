use super::*;
use crate::book_v2_resources::with_budgeted_book_v2_column_placement as drive;
use std::collections::BTreeSet;
use typaxis_pagination::ProductionBodyFragmentSource as Source;

#[derive(Clone, Debug, PartialEq)]
struct Geometry {
    fragments: Vec<(
        u32,
        Option<u16>,
        Option<usize>,
        usize,
        u32,
        i64,
        i64,
        i64,
        i64,
        bool,
    )>,
    headers: usize,
    lists: usize,
    notes: usize,
    numbers: usize,
}
fn physical_input(
    root: &Root,
    mut data: Value,
    text: &str,
    font: Option<&[u8]>,
    caps: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    // Preserve the caller's authored column carrier, including balancing.
    if let Some(font) = font {
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            .into();
        let body = body_with_source(root, data, text.as_bytes(), caps);
        fs::write(root.0.join("body.bin"), font).unwrap();
        prepare_book_v2_resources(
            body,
            &root.context(),
            &config(caps.base().get().clone()),
            caps,
        )
        .unwrap()
    } else {
        prepared(root, data, text.as_bytes(), caps)
    }
}
fn inside(bounds: typaxis_core::Rect, actual: typaxis_core::Rect) {
    assert!(actual.x() >= bounds.x() && actual.y() >= bounds.y());
    assert!(
        actual.x().raw() + actual.width().get().raw()
            <= bounds.x().raw() + bounds.width().get().raw()
    );
    assert!(
        actual.y().raw() + actual.height().get().raw()
            <= bounds.y().raw() + bounds.height().get().raw()
    );
}
fn geometry<'b, 'f, 's, 'p, 'a>(
    pages: &BookV2ColumnStablePages<'b, 'f, 's, 'p, 'a>,
    search: &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
) -> Geometry {
    let placed = search.place_column_pages(pages.sequence()).unwrap();
    assert!(std::ptr::eq(placed.sequence(), pages.sequence()));
    let mut result = Geometry {
        fragments: Vec::new(),
        headers: 0,
        lists: 0,
        notes: 0,
        numbers: 0,
    };
    let mut semantic = BTreeSet::new();
    let mut global = 0usize;
    for page in placed.pages() {
        let selection = page.selection();
        assert_eq!(
            page.columns().len(),
            usize::from(selection.candidate().frames().column_count())
        );
        let mut end = 0;
        for (index, column) in page.columns().iter().enumerate() {
            assert_eq!(
                column.bounds(),
                selection.candidate().frames().column(index as u16).unwrap()
            );
            assert_eq!(column.fragments().start, end);
            end = column.fragments().end;
            for index in column.fragments() {
                assert_eq!(page.fragment_column(index), Some(index_column(page, index)));
                assert!(page.fragments()[index].definition_index().is_none());
                inside(column.bounds(), page.fragments()[index].fragment().bounds());
            }
        }
        for (index, (placed, _, repeated)) in page.fragments_with_roles().enumerate() {
            let fragment = placed.fragment();
            assert_eq!(fragment.page_index(), selection.page_index());
            if !repeated {
                assert!(
                    semantic.insert((placed.definition_index(), placed.item_index())),
                    "duplicate semantic leaf"
                );
            }
            if let Some(definition) = placed.definition_index() {
                assert_eq!(page.fragment_column(index), None);
                assert!(index >= end);
                inside(
                    selection.candidate().footnote_bounds().unwrap(),
                    fragment.bounds(),
                );
                assert_eq!(definition, 0);
            }
            let lines = page
                .header_variant(index)
                .map_or(search.source_lines(), |v| v.lines());
            match fragment.source() {
                Source::ParagraphLine {
                    paragraph_index,
                    line_index,
                } => {
                    let paragraph = &lines.paragraphs()[paragraph_index as usize];
                    assert_eq!(paragraph.owner(), fragment.owner());
                    let line = &paragraph.lines()[line_index as usize];
                    if let Some(selected) = paragraph.selected() {
                        let selected = &selected.lines()[line_index as usize];
                        let width = typaxis_core::PositiveLength::new(
                            selected.required_inline_size().get(),
                        )
                        .unwrap_or(selected.inline_size());
                        assert_eq!(width, fragment.bounds().width());
                        assert!(width.get() <= selected.inline_size().get());
                    } else {
                        assert!(line.items().is_empty() && !paragraph.anchors().is_empty());
                    }
                    assert_eq!(
                        fragment.baseline().unwrap(),
                        fragment.bounds().y().checked_add(line.baseline()).unwrap()
                    );
                    for inline in line.items() {
                        if let typaxis_layout::ProductionPlacedInline::Text(cluster) = inline {
                            for glyph in cluster.glyphs() {
                                assert!(std::ptr::eq(
                                    glyph.glyph(),
                                    &cluster.run().glyph_run().glyphs[glyph.glyph_index() as usize]
                                ));
                            }
                        }
                    }
                }
                Source::Figure { .. }
                | Source::NativeMathBlock { .. }
                | Source::VectorBlock { .. } => {
                    assert!(fragment.viewport().is_some());
                }
            }
            if let Some(header) = page.header_variant(index) {
                assert!(repeated);
                assert_eq!(header.fragment_index(), index);
                assert_eq!(header.definition_index(), placed.definition_index());
                assert!(header.global_item_index() >= placed.item_index());
                result.headers += 1;
            }
            let b = fragment.bounds();
            result.fragments.push((
                selection.page_index(),
                page.fragment_column(index),
                placed.definition_index(),
                placed.item_index(),
                fragment.owner().get(),
                b.x().raw(),
                b.y().raw(),
                b.width().get().raw(),
                b.height().get().raw(),
                repeated,
            ));
        }
        for marker in page.list_markers() {
            let index = marker.fragment_index() as usize;
            let fragment = page.fragments()[index];
            assert_eq!(marker.page_index(), selection.page_index());
            assert_eq!(marker.baseline(), fragment.fragment().baseline().unwrap());
            let region = page
                .fragment_column(index)
                .map_or(selection.candidate().footnote_bounds(), |i| {
                    Some(page.columns()[usize::from(i)].bounds())
                })
                .unwrap();
            assert!(marker.bounds().x() >= region.x());
            assert!(
                marker.bounds().x().raw() + marker.bounds().width().get().raw()
                    <= fragment.fragment().bounds().x().raw()
            );
            result.lists += 1;
        }
        for marker in page.footnote_markers() {
            let fragment = page.fragments()[marker.fragment_index() as usize];
            assert_eq!(fragment.definition_index(), Some(marker.definition_index()));
            let note = selection.candidate().footnote_bounds().unwrap();
            assert!(marker.bounds().x() >= note.x());
            assert!(
                marker.bounds().x().raw() + marker.bounds().width().get().raw()
                    <= fragment.fragment().bounds().x().raw()
            );
            result.notes += 1;
        }
        for number in page.equation_numbers() {
            assert!((number.geometry().fragment_index() as usize) >= global);
            assert!(
                (number.geometry().fragment_index() as usize) < global + page.fragments().len()
            );
            result.numbers += 1;
        }
        if let Some(separator) = page.separator_ink() {
            let note = selection.candidate().footnote_bounds().unwrap();
            assert_eq!(
                (separator.x(), separator.y(), separator.width()),
                (note.x(), note.y(), note.width())
            );
        }
        global += page.fragments().len();
    }
    result
}
fn index_column(page: &BookV2ColumnPlacedPage<'_, '_, '_, '_, '_, '_>, index: usize) -> u16 {
    page.columns()
        .iter()
        .position(|c| c.fragments().contains(&index))
        .unwrap() as u16
}
fn run(
    input: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
    maximum: u64,
) -> (
    Result<Geometry, String>,
    crate::book_v2_resources::BookV2PdfConvergenceObservation,
) {
    let mut budget = BookV2ColumnPageBudget::new(caps, maximum);
    let result = drive(
        input,
        caps,
        MODE,
        &mut budget,
        |pages, search, _, observed| {
            assert!(pages.passes() >= 2);
            assert!(pages.record_charge() <= search.record_charge());
            let before = search.work_steps();
            let result = geometry(pages, search);
            assert!(search.work_steps() > before);
            (
                result,
                observed,
                search.work_steps() - before,
                search.record_charge(),
            )
        },
    )
    .map_err(|e| format!("{e:?}"));
    if let Ok((_, before, extra, records)) = &result {
        assert_eq!(
            budget.observation().work_steps(),
            before.work_steps() + extra
        );
        assert_eq!(budget.observation().record_charge(), *records);
    }
    (result.map(|r| r.0), budget.observation())
}

#[test]
fn book_v2_column_placement_preserves_actual_origins_notes_and_empty_anchors() {
    let caps = caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro";
    for (notes, empty) in [(false, false), (true, false), (false, true)] {
        let root = Root::new();
        let mut data = plain_data(text, notes, empty);
        for (i, m) in data["page_masters"]["masters"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            m["body"]["x"] = ([13, 47, 29, 61][i] * 65536).into();
            if notes {
                m["footnote"]["x"] = ([23, 41, 29, 67][i] * 65536).into();
            }
        }
        let input = physical_input(&root, data, text, None, &caps);
        let first = run(&input, &caps, 100_000_000);
        assert!(first.0.is_ok(), "{first:?}");
        let result = first.0.as_ref().unwrap();
        if !empty {
            assert!(result.fragments.iter().any(|r| r.1 == Some(1)));
        }
        if notes {
            assert!(result.notes > 0);
        }
        assert_eq!(run(&input, &caps, 100_000_000), first);
    }
}

#[test]
fn book_v2_column_placement_preserves_nested_and_repeated_header_owners() {
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
            let mut data = super::super::super::data(notes, mode, text);
            for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
                m["body"]["height"] = (height * 65536).into();
                if notes {
                    m["footnote"]["height"] = (height * 65536).into();
                }
            }
            let input = physical_input(&root, data, text, None, &caps);
            let result = run(&input, &caps, 1_000_000_000);
            eprintln!("column physical headers {notes}/{mode}: {:?}", result.1);
            assert!(result.0.is_ok(), "{result:?}");
            assert!(result.0.unwrap().headers > 0);
        }
    }
}

#[test]
fn book_v2_column_placement_keeps_vector_native_raster_geometry() {
    let caps = caps();
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for notes in [false, true] {
            for tables in [false, true] {
                let root = Root::new();
                let data = super::super::super::super::super::block_widths::column_block_data(
                    kind, notes, tables,
                );
                let input = match kind {
                    "vector" => vector_input(&root, data, &caps),
                    "native" => {
                        let source = data["text_buffers"][0]["utf8"]
                            .as_str()
                            .unwrap()
                            .as_bytes()
                            .to_vec();
                        native_input_with_source(&root, data, &source, &caps)
                    }
                    "png" => figure_input(&root, data, PNG, &caps),
                    "jpeg" => figure_input(&root, data, JPEG, &caps),
                    "svg" => figure_input(&root, data, SVG, &caps),
                    _ => unreachable!(),
                };
                let result = run(&input, &caps, 100_000_000);
                assert!(result.0.is_ok(), "{kind}/{notes}/{tables}: {result:?}");
            }
        }
    }
}

#[test]
fn book_v2_column_placement_retains_exact_callback_budgets_and_failed_retries() {
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro";
    let data = plain_data(text, true, false);
    let base = caps();
    let evaluate = |limits: &M4EffectiveResourceLimits, maximum| {
        let root = Root::new();
        let input = physical_input(&root, data.clone(), text, None, limits);
        let mut budget = BookV2ColumnPageBudget::new(limits, maximum);
        let mut called = false;
        let result = drive(&input, limits, MODE, &mut budget, |pages, search, _, _| {
            called = true;
            let first = search.place_column_pages(pages.sequence());
            let accepted = first.is_ok();
            let prefix = (search.work_steps(), search.record_charge());
            if !accepted {
                assert!(search.place_column_pages(pages.sequence()).is_err());
                assert!(search.work_steps() >= prefix.0 && search.record_charge() >= prefix.1);
            }
            accepted
        });
        (
            result.map_err(|e| format!("{e:?}")),
            budget.observation(),
            called,
        )
    };
    let full = evaluate(&base, 100_000_000);
    assert_eq!(full.0.as_ref().ok(), Some(&true));
    let exact = evaluate(&base, full.1.work_steps());
    assert_eq!(exact, full);
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
fn book_v2_column_placement_rejects_foreign_sequences_short_passes_and_unbalanced_pages() {
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
    with_budgeted_book_v2_column_lines(&policy,&flow,input.resources(),&bindings,&caps,MODE,None,&mut allowance,&plan,None,|stable| {
        let mut records=stable.retained_record_charge();
        let flow=prepare_book_v2_column_flow_counted(&stable,None,&caps,records,&mut records).unwrap();
        let measured=prepare_book_v2_column_table_measurements_counted(flow,&caps,&mut records).unwrap();
        let mut a=prepare_book_v2_column_page_search_counted(&measured,&caps,100_000_000,records,&mut 0,&mut 0).unwrap();
        let mut b=prepare_book_v2_column_page_search_counted(&measured,&caps,100_000_000,records,&mut 0,&mut 0).unwrap();
        let sequence=a.select_pages().unwrap();
        assert!(matches!(b.place_column_pages(&sequence),Err(e) if e.kind==E::ReceiptMismatch));
        for remaining in [0,1] {
            let before=(a.record_charge(),a.work_steps());
            let mut begun=99;
            assert!(matches!(a.select_stable_column_pages_counted(remaining,&mut begun),Err(e) if e.kind==E::PagePassLimit));
            assert_eq!(begun,0);
            assert_eq!(before,(a.record_charge(),a.work_steps()));
        }
    }).unwrap();
    let mut data = plain_data(text, false, false);
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        m["column_layout"]["balance"] = "last_page".into();
    }
    let input = physical_input(&root, data, text, None, &caps);
    let result = run(&input, &caps, 100_000_000);
    assert!(result.0.is_err() && result.1.candidate_passes() == 0);
    assert!(format!("{:?}", result.0).contains("column_balance"));
    assert!(
        result.1.page_passes() > 0 && result.1.record_charge() > 0 && result.1.work_steps() > 0
    );
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_column_placement_preserves_original_harano_lists_and_note_markers() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let caps = caps();
    let text = "元の字形を保って段の座標へ移す元の字形を保って段の座標へ移す";
    for ordered in [false, true] {
        for notes in [false, true] {
            let root = Root::new();
            let mut data = plain_data(text, notes, false);
            fn list(blocks: Value, ordered: bool) -> Value {
                let span = blocks[0]["span"].clone();
                let items = blocks
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|b| json!({"node_id":0,"span":span,"blocks":[b]}))
                    .collect::<Vec<_>>();
                json!([{"kind":"list","node_id":0,"span":span,"classes":[],"ordered":ordered,"start":if ordered {json!(1)} else {Value::Null},"items":items}])
            }
            data["document"]["blocks"] = list(data["document"]["blocks"].clone(), ordered);
            if notes {
                data["document"]["footnotes"][0]["blocks"] =
                    list(data["document"]["footnotes"][0]["blocks"].clone(), ordered);
            }
            crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
                &mut data["document"],
                &mut 0,
            );
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            let mut rule = rules
                .iter()
                .find(|r| {
                    r["declarations"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|d| d["name"] == "font_family")
                })
                .unwrap()
                .clone();
            rule["selector"] = "list".into();
            rule["style_id"] = "column-list-font".into();
            rule["source_order"] = (rules
                .iter()
                .filter_map(|r| r["source_order"].as_u64())
                .max()
                .unwrap()
                + 1)
            .into();
            rules.push(rule);
            let input = physical_input(&root, data, text, Some(&font), &caps);
            let result = run(&input, &caps, 100_000_000);
            assert!(result.0.is_ok(), "{ordered}/{notes}: {result:?}");
            let result = result.0.unwrap();
            assert!(result.lists >= 6);
            if notes {
                assert!(result.notes > 0 && result.lists >= 8);
            }
        }
    }
}
