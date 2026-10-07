use super::*;
#[path = "book_v2_column_page_driver_tests.rs"]
mod driver;
use crate::book_v2_resources::tests::shaping_tests::{
    figures::{figure_input, JPEG, SVG},
    native_tests::native_input_with_source,
    vector_tests::vector_input,
};
use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
use typaxis_shaping::book_v2::shape_book_v2_equation_numbers;

#[derive(Clone, Debug, PartialEq)]
struct Profiles {
    widths: Vec<Vec<PositiveLength>>,
    starts: Vec<Option<Vec<Length>>>,
    ends: Vec<Option<Vec<u32>>>,
    blocks: Vec<(NodeId, PositiveLength)>,
    block_starts: Vec<Length>,
    fingerprint: [u8; 32],
    lines_match: bool,
    blocks_match: bool,
    pages: usize,
    note_inset: i64,
}
fn snapshot(report: &BookV2ColumnWidthFeedback<'_, '_>, pages: usize, note_inset: i64) -> Profiles {
    Profiles {
        widths: report
            .paragraphs()
            .iter()
            .map(|p| p.widths().to_vec())
            .collect(),
        starts: report
            .paragraphs()
            .iter()
            .map(|p| p.source_unit_starts().map(<[_]>::to_vec))
            .collect(),
        ends: report
            .paragraphs()
            .iter()
            .map(|p| p.retained_line_ends().map(<[_]>::to_vec))
            .collect(),
        blocks: report.block_widths().to_vec(),
        block_starts: report.block_starts().to_vec(),
        fingerprint: report.assignment_fingerprint(),
        lines_match: report.matches_selected_line_widths(),
        blocks_match: report.matches_selected_block_widths(),
        pages,
        note_inset,
    }
}
fn plain_data(text: &str, notes: bool, empty: bool) -> Value {
    let mut data = super::data(false, "common", text);
    let mut paragraph = data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0].clone();
    paragraph["classes"] = json!([]);
    let mut blocks = vec![paragraph.clone(); 6];
    if notes {
        blocks[0]["children"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"footnote_reference","node_id":0,
            "span":paragraph["span"],"footnote_id":"column-width-note"}));
        data["document"]["footnotes"] = json!([
            {"node_id":0,"span":paragraph["span"],"footnote_id":"column-width-note","blocks":[paragraph.clone(),paragraph.clone()]},
            {"node_id":0,"span":paragraph["span"],"footnote_id":"unreferenced-column-note","blocks":[paragraph.clone()]}
        ]);
    } else {
        data["document"]["footnotes"] = json!([]);
    }
    if empty {
        for (i, block) in blocks.iter_mut().enumerate() {
            block["children"] = json!([{"kind":"anchor","node_id":0,"span":paragraph["span"],"anchor_id":format!("empty-column-width-{i}")}]);
        }
    }
    paragraph["classes"] = json!([]);
    data["document"]["blocks"] = json!(blocks);
    for (index, master) in data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        master["body"]["height"] = (48 * 65536).into();
        if notes {
            let width = [300, 380, 300, 380][index] * 65536;
            master["footnote"] =
                json!({"x":10*65536,"y":300*65536,"width":width,"height":120*65536});
        }
    }
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    data
}

fn collect_reflow(
    input: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
    budget_checks: bool,
    expected_text: Option<&[String]>,
) -> Profiles {
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 100_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), caps).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), caps).unwrap();
    let native = compute_book_v2_native_math(&bindings, input.resources(), caps, 0, 0).unwrap();
    let run = |previous: Option<&Profiles>, retain: bool| {
        let widths = previous.map_or_else(Vec::new, |p| {
            p.widths
                .iter()
                .map(|v| Some(v.as_slice()))
                .collect::<Vec<_>>()
        });
        let starts = previous.map_or_else(Vec::new, |p| {
            p.starts.iter().map(|v| v.as_deref()).collect::<Vec<_>>()
        });
        let ends = previous.map_or_else(Vec::new, |p| {
            p.ends.iter().map(|v| v.as_deref()).collect::<Vec<_>>()
        });
        let assignment = previous.map(|p| {
            BookV2SourceWidthAssignments::with_retained_line_ends(&flow, &widths, &ends)
                .unwrap()
                .with_source_unit_starts(&starts)
                .unwrap()
                .with_table_occurrence_frames()
                .with_block_widths(&p.blocks)
                .with_block_starts(&p.block_starts)
        });
        let mut allowance =
            BookV2BodyLineBudget::new(100_000_000, caps.base().get().max_line_reshape_passes);
        with_budgeted_book_v2_column_lines(&policy, &flow, input.resources(), &bindings, caps, MODE,
            native.as_ref(), &mut allowance, &plan, assignment.as_ref(), |stable| {
                if let Some(expected) = expected_text {
                    assert_eq!(stable.lines().paragraphs().len(), expected.len());
                    for (paragraph, expected) in stable.lines().paragraphs().iter().zip(expected) {
                        let mut actual = String::new();
                        for line in paragraph.lines() {
                            for item in line.items() {
                                if let typaxis_layout::ProductionPlacedInline::Text(cluster) = item {
                                    actual.push_str(cluster.utf8());
                                    for glyph in cluster.glyphs() {
                                        assert!(glyph.glyph().original_gid.get() > 0);
                                        assert!(std::ptr::eq(glyph.glyph(), &cluster.run().glyph_run().glyphs[glyph.glyph_index() as usize]));
                                    }
                                }
                            }
                        }
                        assert_eq!(&actual, expected);
                    }
                }
                let numbers = shape_book_v2_equation_numbers(stable.lines().prepared().shaped(), caps, 0).unwrap();
                let blocks = prepare_book_v2_vector_blocks(stable.lines(), numbers.as_ref(), caps, stable.retained_record_charge()).unwrap();
                let mut records = stable.retained_record_charge();
                let source = prepare_book_v2_column_flow_counted(&stable, blocks.as_ref(), caps, records, &mut records).unwrap();
                let measured = prepare_book_v2_column_table_measurements_counted(source, caps, &mut records).unwrap();
                let evaluate = |maximum, prior| {
                    let (mut observed_records, mut observed_work) = (0, 0);
                    let mut search = match prepare_book_v2_column_page_search_counted(&measured, caps, maximum, prior,
                        &mut observed_records, &mut observed_work) {
                        Ok(search) => search, Err(error) => return (Err(error), observed_work, observed_records),
                    };
                    let result = (|| {
                        let pages = search.select_pages()?;
                        let mut report = search.paragraph_frame_feedback(&pages)?;
                        assert!(report.matches_source_flow(&flow));
                        assert!(std::ptr::eq(report.column_plan(), &plan));
                        assert_eq!(report.paragraphs().len(), flow.paragraphs().len());
                        assert_eq!((report.work_steps(), report.record_charge()), (search.work_steps(), search.record_charge()));
                        if retain {
                            let before = report.assignment_fingerprint();
                            search.retain_paragraph_line_boundaries(&pages, &mut report)?;
                            assert_ne!(report.assignment_fingerprint(), before);
                            assert!(report.paragraphs().iter().all(|p| p.retained_line_ends().is_some()));
                        }
                        if maximum == 100_000_000 {
                            let mut foreign = prepare_book_v2_column_page_search_counted(&measured, caps, maximum,
                                records, &mut 0, &mut 0).unwrap();
                            assert!(matches!(foreign.paragraph_frame_feedback(&pages),Err(e) if e.kind == E::ReceiptMismatch));
                            assert!(foreign.retain_paragraph_line_boundaries(&pages, &mut report).is_err());
                        }
                        let note_inset = stable.lines().frames().unwrap().footnotes().first().map_or(0,
                            |frame| frame.marker_width().get().raw() + frame.marker_gap().get().raw());
                        Ok(snapshot(&report, pages.pages().len(), note_inset))
                    })();
                    (result, search.work_steps(), search.record_charge())
                };
                let full = evaluate(100_000_000, records);
                if budget_checks {
                    let exact = evaluate(full.1, records);
                    assert_eq!(exact, full);
                    let short = evaluate(full.1-1, records);
                    assert!(short.0.is_err()); assert!(short.1 <= full.1-1 && short.1>0 && short.2>=records);
                    let prior = caps.base().get().max_fragments - (full.2-records);
                    let exact = evaluate(full.1, prior); assert_eq!(exact.0.as_ref().unwrap(), full.0.as_ref().unwrap());
                    assert_eq!(exact.2, caps.base().get().max_fragments);
                    let short = evaluate(full.1, prior+1); assert!(short.0.is_err());
                    assert!(short.1<=full.1 && short.2>=prior+1 && short.2<=caps.base().get().max_fragments);
                }
                full.0.unwrap()
            }).unwrap()
    };
    let mut previous = run(None, false);
    let initial = previous.clone();
    if budget_checks {
        let retained = run(None, true);
        assert!(retained.ends.iter().all(Option::is_some));
        let replay = run(Some(&retained), true);
        assert!(replay.ends.iter().all(Option::is_some));
    }
    let mut fingerprints = Vec::new();
    let mut refining = false;
    for round in 0..16 {
        if previous.lines_match && previous.blocks_match {
            break;
        }
        refining |= fingerprints.contains(&previous.fingerprint);
        fingerprints.push(previous.fingerprint);
        previous = run(Some(&previous), refining);
        eprintln!(
            "column width feedback round {}: pages={},lines={},blocks={},retained={refining}",
            round + 1,
            previous.pages,
            previous.lines_match,
            previous.blocks_match
        );
    }
    assert!(
        previous.lines_match && previous.blocks_match,
        "column widths did not converge: {previous:?}"
    );
    assert_eq!(run(Some(&previous), refining), previous);
    assert!(initial.pages > 0);
    previous
}

fn plain(font: Option<&[u8]>, notes: bool, empty: bool, budgets: bool) {
    let root = Root::new();
    let caps = limits();
    let text = if font.is_some() {
        "元の字形を保ち段の幅で本文を組み直す元の字形を保ち段の幅で本文を組み直す"
    } else {
        "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro"
    };
    let data = plain_data(text, notes, empty);
    let input = input(&root, data, text, font, &caps);
    let mut expected = vec![if empty { "" } else { text }.to_owned(); if notes { 9 } else { 6 }];
    if notes {
        expected[0].push('1');
    }
    let profile = collect_reflow(&input, &caps, budgets, Some(&expected));
    assert_eq!(profile.widths.len(), if notes { 9 } else { 6 });
    let body_widths = profile.widths[..6]
        .iter()
        .flatten()
        .map(|w| w.get().raw())
        .collect::<BTreeSet<_>>();
    assert!(body_widths
        .iter()
        .all(|w| [140 * 65536, 220 * 65536].contains(w)));
    assert!(body_widths.contains(&(140 * 65536)));
    if !empty {
        assert!(body_widths.contains(&(220 * 65536)));
    }
    assert!(profile.starts.iter().all(Option::is_none));
    if notes {
        let note_widths = profile.widths[6..8]
            .iter()
            .flatten()
            .map(|w| w.get().raw())
            .collect::<BTreeSet<_>>();
        let narrow = 300 * 65536 - profile.note_inset;
        let wide = 380 * 65536 - profile.note_inset;
        assert!(profile.note_inset > 0 && narrow > 220 * 65536);
        assert!(
            note_widths.iter().all(|w| [narrow, wide].contains(w)),
            "{note_widths:?}"
        );
        assert!(note_widths.contains(&narrow));
        assert!(profile.widths[8].iter().all(|w| w.get().raw() == wide));
    }
}
#[test]
fn book_v2_column_width_feedback_reflows_original_units_and_exact_budgets() {
    plain(None, false, false, true);
}
#[test]
fn book_v2_column_width_feedback_keeps_full_page_notes_and_unreferenced_definitions() {
    plain(None, true, false, false);
}
#[test]
fn book_v2_column_width_feedback_preserves_empty_anchor_units_and_retained_boundaries() {
    plain(None, false, true, true);
}
#[test]
fn book_v2_column_width_feedback_keeps_nested_and_repeated_header_owners() {
    for notes in [false, true] {
        for (mode, height) in [
            ("common", 256),
            ("span", 256),
            ("nested-body", 512),
            ("nested-body-span", 544),
            ("nested-body-caption", 512),
        ] {
            super::check_feedback_with_height(None, notes, mode, Some(height), true);
        }
    }
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_column_width_feedback_reflows_original_harano_units_and_notes() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    plain(Some(&font), false, false, false);
    plain(Some(&font), true, false, false);
    super::check_feedback_with_height(Some(&font), false, "common", Some(256), true);
}
#[test]
fn book_v2_column_width_feedback_reflows_vector_native_and_raster_blocks_in_tables_and_notes() {
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for notes in [false, true] {
            for tables in [false, true] {
                let root = Root::new();
                let caps = driver_limits();
                let data =
                    super::super::super::block_widths::column_block_data(kind, notes, tables);
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
                eprintln!("column block width {kind}/{notes}/{tables}");
                collect_reflow(&input, &caps, false, None);
            }
        }
    }
}
