use super::*;
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, with_converged_book_v2_body_lines_with_source_widths,
};
use typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_reflow;

#[test]
fn book_v2_variable_page_widths_reject_provisional_math_finalization() {
    let root = Root::new();
    let limits = driver_limits();
    let text = "R R R R R R";
    let input = prepared(&root, width_data(text, "body"), text.as_bytes(), &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let plan = prepare_book_v2_page_frame_plan_for_reflow(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    with_converged_book_v2_body_lines_with_source_widths(
        &policy, &flow, input.resources(), &bindings, &limits,
        JapaneseLineBreakMode::Normal, plan.measurement_body(), 10_000_000,
        None, limits.base().get().max_line_reshape_passes, Some(&plan), None,
        |lines| {
            let body = prepare_book_v2_body_flow(lines.lines(), None, lines.footnotes(), &limits, 0).unwrap();
            let measured = prepare_book_v2_table_measurements(body, &limits).unwrap();
            let mut search = prepare_book_v2_table_body_search(&measured, &limits, 10_000_000, measured.record_charge()).unwrap();
            let stable = search.select_stable_mixed_pages(limits.base().get().max_layout_passes).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            let closed = search.close_mixed_page_sources(&stable, &placed).unwrap();
            assert!(!search.paragraph_width_feedback(&closed).unwrap().matches_selected_line_widths());
            assert!(matches!(search.finalize_mixed_page_math(closed, &limits, 0),
                Err(e) if e.kind == typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch));
        },
    ).unwrap();
}

fn width_data(text: &str, mode: &str) -> Value {
    let mut data = selected_master_data(text);
    if mode != "references" {
        let mut p = data["document"]["blocks"][0]["blocks"][0].clone();
        p["children"]
            .as_array_mut()
            .unwrap()
            .retain(|n| n["kind"] != "reference");
        if mode == "notes" {
            let note = p.clone();
            p["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":note["span"],"footnote_id":"width-note"}));
            data["document"]["footnotes"] = json!([{"node_id":0,"span":note["span"],"footnote_id":"width-note","blocks":[note]}]);
        }
        data["document"]["blocks"] = json!([p]);
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":format!("variable-width-{mode}"),"selector":"paragraph","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"line_height","important":false,"value":{"kind":"length","value":20*65536}},
        {"name":"text_align","important":false,"value":{"kind":"keyword","value":"center"}}]}));
    for (i, w, nw) in [(0, 72, 60), (1, 48, 36), (2, 24, 72), (3, 60, 60)] {
        let m = &mut data["page_masters"]["masters"][i];
        m["body"] = json!({"x":10*65536,"y":10*65536,"width":w*65536,"height":20*65536});
        if mode == "notes" {
            m["footnote"] = json!({"x":20*65536,"y":100*65536,"width":nw*65536,"height":40*65536+typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW});
        }
    }
    renumber(&mut data["document"], &mut 0);
    data
}
fn check_width_pdf(mode: &str, font: Option<&[u8]>) {
    let root = Root::new();
    let base = driver_limits();
    let mut caps = base.base().get().clone();
    caps.max_layout_passes = 32;
    caps.max_line_reshape_passes = 32;
    let limits = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(caps).unwrap(),
        base.extension().get().clone(),
    )
    .unwrap();
    let text = if font.is_some() {
        "左側右側左側右側左側右側"
    } else {
        "R R R R R R"
    };
    let mut data = width_data(text, mode);
    let input = if let Some(font) = font {
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            .into();
        let body = body_with_source(&root, data, text.as_bytes(), &limits);
        fs::write(root.0.join("body.bin"), font).unwrap();
        prepare_book_v2_resources(
            body,
            &root.context(),
            &config(limits.base().get().clone()),
            &limits,
        )
        .unwrap()
    } else {
        prepared(&root, data, text.as_bytes(), &limits)
    };
    let run = |maximum| {
        with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            maximum,
            |pdf, observed| {
                assert!(observed.width_feedback_passes() >= 2);
                if font.is_some() && mode == "notes" {
                    assert!(observed.width_refinement_passes() > 0);
                }
                let closure = pdf
                    .navigation()
                    .source()
                    .source()
                    .source()
                    .source()
                    .display()
                    .source()
                    .source();
                let frames = closure.flow().lines().frames().unwrap();
                assert!(frames.page_plan().unwrap().requires_width_reflow());
                for page in closure.geometry().pages() {
                    for placed in page.fragments() {
                        let fragment = placed.fragment();
                        let region = if placed.definition_index().is_some() {
                            page.selection().declared_footnote_region().unwrap()
                        } else {
                            page.selection().body_bounds()
                        };
                        assert!(fragment.bounds().x() >= region.x());
                        assert!(
                            fragment
                                .bounds()
                                .x()
                                .checked_add(fragment.bounds().width().get())
                                .unwrap()
                                <= region.x().checked_add(region.width().get()).unwrap()
                        );
                    }
                }
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_V2_VARIABLE_WIDTH_PROBE") {
                    let directory = std::path::PathBuf::from(directory);
                    fs::create_dir_all(&directory).unwrap();
                    let name = format!(
                        "{}-{mode}",
                        if font.is_some() {
                            "harano"
                        } else {
                            "controlled"
                        }
                    );
                    fs::write(directory.join(format!("{name}.pdf")), pdf.bytes()).unwrap();
                    fs::write(directory.join(format!("{name}.json")),serde_json::to_vec_pretty(&json!({"width_passes":observed.width_feedback_passes(),"width_refinements":observed.width_refinement_passes(),"line_passes":observed.line_reshape_passes(),"page_passes":observed.page_passes(),"pdf_candidates":observed.candidate_passes(),"work":observed.work_steps(),"pages":closure.geometry().pages().len()})).unwrap()).unwrap();
                }
                (observed, typaxis_core::sha256(pdf.bytes()))
            },
        )
    };
    let full =
        run(100_000_000).unwrap_or_else(|e| panic!("{mode} harano={} {e:?}", font.is_some()));
    assert_eq!(run(full.0.work_steps()).unwrap(), full);
    assert!(run(full.0.work_steps() - 1).is_err());
}
#[test]
fn book_v2_variable_page_widths_converge_body_notes_and_reference_pdf() {
    for mode in ["body", "notes", "references"] {
        check_width_pdf(mode, None);
    }
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_variable_page_widths_render_unchanged_harano_pdf() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    for mode in ["body", "notes", "references"] {
        check_width_pdf(mode, Some(&font));
    }
}
