use super::*;
use crate::book_v2_resources::{
    with_budgeted_book_v2_column_pages as drive, BookV2ColumnPageBudget,
};

fn caps() -> M4EffectiveResourceLimits {
    let old = limits();
    let mut base = old.base().get().clone();
    base.max_line_reshape_passes = 128;
    base.max_layout_passes = 128;
    base.max_page_break_lookback = 256;
    base.max_footnote_reflows_per_page = 256;
    M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(base).unwrap(),
        old.extension().get().clone(),
    )
    .unwrap()
}
fn changed_caps(
    limits: &M4EffectiveResourceLimits,
    change: impl FnOnce(&mut typaxis_core::ResourceLimits),
) -> M4EffectiveResourceLimits {
    let mut base = limits.base().get().clone();
    change(&mut base);
    M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(base).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap()
}
fn inspect_text(search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>, expected: &[String]) {
    let lines = search.source_lines();
    assert_eq!(lines.paragraphs().len(), expected.len());
    for (paragraph, expected) in lines.paragraphs().iter().zip(expected) {
        let mut text = String::new();
        for line in paragraph.lines() {
            for item in line.items() {
                if let typaxis_layout::ProductionPlacedInline::Text(cluster) = item {
                    text.push_str(cluster.utf8());
                    for glyph in cluster.glyphs() {
                        assert!(glyph.glyph().original_gid.get() > 0);
                        assert!(std::ptr::eq(
                            glyph.glyph(),
                            &cluster.run().glyph_run().glyphs[glyph.glyph_index() as usize]
                        ));
                    }
                }
            }
        }
        assert_eq!(&text, expected);
    }
}
fn evaluate(
    data: Value,
    text: &str,
    font: Option<&[u8]>,
    caps: &M4EffectiveResourceLimits,
    maximum: u64,
    expected: Option<&[String]>,
) -> (
    Result<Profiles, String>,
    crate::book_v2_resources::BookV2PdfConvergenceObservation,
    bool,
) {
    let root = Root::new();
    let input = input(&root, data, text, font, caps);
    evaluate_input(&input, caps, maximum, expected)
}
fn evaluate_input(
    input: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
    maximum: u64,
    expected: Option<&[String]>,
) -> (
    Result<Profiles, String>,
    crate::book_v2_resources::BookV2PdfConvergenceObservation,
    bool,
) {
    let mut budget = BookV2ColumnPageBudget::new(caps, maximum);
    let mut called = false;
    let mut observation = None;
    let result = drive(
        input,
        caps,
        MODE,
        &mut budget,
        |pages, search, report, observed| {
            called = true;
            assert!(pages.passes() >= 2);
            assert!(
                report.matches_selected_line_widths() && report.matches_selected_block_widths()
            );
            assert_eq!(report.work_steps(), search.work_steps());
            assert_eq!(report.record_charge(), search.record_charge());
            if let Some(expected) = expected {
                inspect_text(search, expected);
            }
            let inset = search
                .source_lines()
                .frames()
                .unwrap()
                .footnotes()
                .first()
                .map_or(0, |f| {
                    f.marker_width().get().raw() + f.marker_gap().get().raw()
                });
            observation = Some(observed);
            snapshot(report, pages.sequence().pages().len(), inset)
        },
    )
    .map_err(|e| format!("{e:?}"));
    if let Some(observed) = observation {
        assert_eq!(observed, budget.observation());
    }
    (result, budget.observation(), called)
}

#[test]
fn book_v2_column_page_driver_moves_feedback_from_short_graphs_and_rejects_foreign_owners() {
    let root = Root::new();
    let caps = caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro";
    let input = input(&root, plain_data(text, false, false), text, None, &caps);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let other_flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    assert_eq!(flow.fingerprint(), other_flow.fingerprint());
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 100_000_000, 0, 0).unwrap();
    let other_plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 100_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &caps).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &caps).unwrap();
    let mut allowance = BookV2BodyLineBudget::new(100_000_000, 128);
    let (captured, addresses, records, work) = with_budgeted_book_v2_column_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &caps,
        MODE,
        None,
        &mut allowance,
        &plan,
        None,
        |stable| {
            let mut records = stable.retained_record_charge();
            let source =
                prepare_book_v2_column_flow_counted(&stable, None, &caps, records, &mut records)
                    .unwrap();
            let measured =
                prepare_book_v2_column_table_measurements_counted(source, &caps, &mut records)
                    .unwrap();
            let mut search = prepare_book_v2_column_page_search_counted(
                &measured,
                &caps,
                100_000_000,
                records,
                &mut 0,
                &mut 0,
            )
            .unwrap();
            let pages = search.select_pages().unwrap();
            for foreign in 0..3 {
                let report = search.paragraph_frame_feedback(&pages).unwrap();
                let before = search.work_steps();
                let records = search.record_charge();
                if foreign == 2 {
                    let mut other = prepare_book_v2_column_page_search_counted(
                        &measured,
                        &caps,
                        100_000_000,
                        records,
                        &mut 0,
                        &mut 0,
                    )
                    .unwrap();
                    assert!(other
                        .capture_source_width_feedback(report, &flow, &plan)
                        .is_err());
                } else {
                    assert!(search
                        .capture_source_width_feedback(
                            report,
                            if foreign == 0 { &other_flow } else { &flow },
                            if foreign == 1 { &other_plan } else { &plan }
                        )
                        .is_err());
                    assert_eq!(search.work_steps(), before + 1);
                    assert_eq!(search.record_charge(), records);
                }
            }
            let mut report = search.paragraph_frame_feedback(&pages).unwrap();
            search
                .retain_paragraph_line_boundaries(&pages, &mut report)
                .unwrap();
            let addresses = report
                .paragraphs()
                .iter()
                .map(|p| p.widths().as_ptr())
                .collect::<Vec<_>>();
            let records = search.record_charge();
            let work = search.work_steps();
            let report = search
                .capture_source_width_feedback(report, &flow, &plan)
                .unwrap();
            (report, addresses, records, work)
        },
    )
    .unwrap();
    assert!(std::ptr::eq(captured.column_plan(), &plan));
    assert!(captured.matches_source_flow(&flow));
    assert_eq!(captured.record_charge(), records);
    assert_eq!(captured.work_steps(), work + 1);
    assert!(captured
        .paragraphs()
        .iter()
        .all(|p| p.retained_line_ends().is_some()));
    assert_eq!(
        captured
            .paragraphs()
            .iter()
            .map(|p| p.widths().as_ptr())
            .collect::<Vec<_>>(),
        addresses
    );
}

#[test]
fn book_v2_column_page_driver_reflows_body_full_page_notes_and_empty_anchors() {
    let caps = caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro";
    for (notes, empty) in [(false, false), (true, false), (false, true)] {
        let mut expected =
            vec![if empty { "" } else { text }.to_owned(); if notes { 9 } else { 6 }];
        if notes {
            expected[0].push('1');
        }
        let result = evaluate(
            plain_data(text, notes, empty),
            text,
            None,
            &caps,
            100_000_000,
            Some(&expected),
        );
        let profile = result.0.unwrap();
        assert!(result.2 && profile.pages > 0);
        assert!(result.1.width_feedback_passes() >= 2);
        assert_eq!(result.1.candidate_passes(), 1);
        assert!(profile.widths[..6]
            .iter()
            .flatten()
            .all(|w| [140 * 65536, 220 * 65536].contains(&w.get().raw())));
        if notes {
            assert!(profile.widths[6..8].iter().flatten().all(|w| [
                300 * 65536 - profile.note_inset,
                380 * 65536 - profile.note_inset
            ]
            .contains(&w.get().raw())));
            assert!(profile.widths[8]
                .iter()
                .all(|w| w.get().raw() == 380 * 65536 - profile.note_inset));
        }
    }
}

#[test]
fn book_v2_column_page_driver_enforces_cumulative_exact_budgets_and_failed_retries() {
    let caps = caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro";
    let data = plain_data(text, true, false);
    let full = evaluate(data.clone(), text, None, &caps, 100_000_000, None);
    let profile = full.0.as_ref().unwrap();
    let observed = full.1;
    let exact = evaluate(data.clone(), text, None, &caps, observed.work_steps(), None);
    assert_eq!(exact.0.as_ref().unwrap(), profile);
    assert_eq!(exact.1, observed);
    let short = evaluate(
        data.clone(),
        text,
        None,
        &caps,
        observed.work_steps() - 1,
        None,
    );
    assert!(short.0.is_err() && !short.2);
    for (label, needed) in [
        ("line", u64::from(observed.line_reshape_passes())),
        ("page", u64::from(observed.page_passes())),
        ("record", observed.record_charge()),
    ] {
        for short in [false, true] {
            let limit = needed - u64::from(short);
            let bounded = changed_caps(&caps, |c| match label {
                "line" => c.max_line_reshape_passes = limit as u16,
                "page" => c.max_layout_passes = limit as u16,
                "record" => c.max_fragments = limit,
                _ => unreachable!(),
            });
            let result = evaluate(data.clone(), text, None, &bounded, 100_000_000, None);
            if short {
                assert!(result.0.is_err() && !result.2, "{label}: {result:?}");
            } else {
                let accepted = result.0.as_ref().unwrap();
                assert!(
                    accepted.widths == profile.widths
                        && accepted.starts == profile.starts
                        && accepted.ends == profile.ends
                        && accepted.blocks == profile.blocks
                        && accepted.block_starts == profile.block_starts
                        && accepted.pages == profile.pages
                        && accepted.note_inset == profile.note_inset,
                    "{label} source geometry"
                );
                if label == "record" {
                    assert_eq!(result.1.record_charge(), needed);
                }
            }
        }
    }
    let root = Root::new();
    let input = input(&root, data, text, None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, observed.work_steps() - 1);
    for _ in 0..2 {
        let before = budget.observation();
        assert!(drive(&input, &caps, MODE, &mut budget, |_, _, _, _| panic!(
            "short budget callback"
        ))
        .is_err());
        let after = budget.observation();
        assert!(after.work_steps() >= before.work_steps());
        assert!(after.record_charge() >= before.record_charge());
        assert!(after.line_reshape_passes() >= before.line_reshape_passes());
        assert!(after.page_passes() >= before.page_passes());
    }
    let before = budget.observation();
    let foreign = changed_caps(&caps, |c| c.max_layout_passes -= 1);
    assert!(
        drive(&input, &foreign, MODE, &mut budget, |_, _, _, _| panic!(
            "foreign limits"
        ))
        .is_err()
    );
    assert_eq!(budget.observation(), before);
}

#[test]
fn book_v2_column_page_driver_discovers_actual_nested_body_and_note_header_widths() {
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
            let mut data = super::super::data(notes, mode, text);
            for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
                m["body"]["height"] = (height * 65536).into();
                if notes {
                    m["footnote"]["height"] = (height * 65536).into();
                }
            }
            eprintln!("column driver headers {notes}/{mode}/{height}");
            if !notes && mode == "span" {
                let bounded = evaluate(data.clone(), text, None, &caps, 100_000_000, None);
                assert!(bounded.0.is_err() && !bounded.2);
                assert!(bounded.1.work_steps() > 0 && bounded.1.work_steps() <= 100_000_000);
                assert!(bounded.1.page_passes() > 0 && bounded.1.record_charge() > 0);
            }
            let result = evaluate(data, text, None, &caps, 1_000_000_000, None);
            eprintln!(
                "column driver header observation {notes}/{mode}: {:?}",
                result.1
            );
            assert!(result.0.is_ok() && result.2, "{result:?}");
        }
    }
}

#[test]
fn book_v2_column_page_driver_preserves_vector_native_raster_body_notes_and_tables() {
    let caps = caps();
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for notes in [false, true] {
            for tables in [false, true] {
                let root = Root::new();
                let data = super::super::super::super::block_widths::column_block_data(
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
                eprintln!("column driver block {kind}/{notes}/{tables}");
                let result = evaluate_input(&input, &caps, 100_000_000, None);
                assert!(result.0.is_ok() && result.2, "{result:?}");
            }
        }
    }
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_column_page_driver_reflows_original_harano_with_shared_budget() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let caps = caps();
    let text = "本文を続けて組み直す本文を続けて組み直す";
    for notes in [false, true] {
        let mut expected = vec![text.to_owned(); if notes { 9 } else { 6 }];
        if notes {
            expected[0].push('1');
        }
        let result = evaluate(
            plain_data(text, notes, false),
            text,
            Some(&font),
            &caps,
            100_000_000,
            Some(&expected),
        );
        assert!(result.0.is_ok() && result.2, "{result:?}");
    }
    // The source-dependent note reservation provokes the original variable
    // width cycle. Also exercise a physical sequence that mixes one/two columns.
    let text = "左側右側左側右側左側右側";
    for mixed in [false, true] {
        let mut data = plain_data(text, true, false);
        data["document"]["blocks"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        data["document"]["footnotes"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        data["document"]["footnotes"][0]["blocks"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
        rules.push(json!({"style_id":"column-cycle","selector":"paragraph","source_order":rules.len(),"extends":null,
            "declarations":[{"name":"line_height","important":false,"value":{"kind":"length","value":20*65536}},
                {"name":"text_align","important":false,"value":{"kind":"keyword","value":"center"}}]}));
        for (i, w, nw) in [(0, 72, 60), (1, 48, 36), (2, 24, 72), (3, 60, 60)] {
            let m = &mut data["page_masters"]["masters"][i];
            let width = if mixed && i % 2 == 1 { w * 2 + 10 } else { w };
            m["body"] = json!({"x":10*65536,"y":10*65536,"width":width*65536,"height":20*65536});
            if !(mixed && i % 2 == 1) {
                m["column_layout"] = Value::Null;
            }
            m["footnote"] = json!({"x":20*65536,"y":100*65536,"width":nw*65536,
                "height":40*65536+typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW});
        }
        crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
            &mut data["document"],
            &mut 0,
        );
        let expected = vec![format!("{text}1"), text.to_owned()];
        let result = evaluate(data, text, Some(&font), &caps, 100_000_000, Some(&expected));
        eprintln!("column original cycle mixed={mixed}: {:?}", result.1);
        assert!(result.0.is_ok() && result.2, "{result:?}");
        assert!(result.1.width_refinement_passes() > 0);
    }
}
