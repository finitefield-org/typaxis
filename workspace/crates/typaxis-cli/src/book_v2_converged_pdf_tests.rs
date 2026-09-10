use super::*;
#[path = "book_v2_block_width_pdf_tests.rs"]
mod block_widths;
use crate::book_v2_resources::{with_converged_book_v2_pdf, BookV2ConvergenceError as CE};

fn source_frame(mut data: Value) -> Value {
    data["page_masters"]["masters"][0]["body"] =
        json!({"x":500_000,"y":500_000,"width":10_000_000,"height":3_000_000});
    renumber(&mut data["document"], &mut 0);
    data
}
fn driver_limits() -> M4EffectiveResourceLimits {
    let base = limits();
    let mut caps = base.base().get().clone();
    caps.max_line_reshape_passes = 8;
    M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(caps).unwrap(),
        base.extension().get().clone(),
    )
    .unwrap()
}
#[test]
fn book_v2_driver_rebuilds_forward_labels_and_enforces_shared_pass_and_byte_limits() {
    let (data, forward, unused) = forward_page_reference_data();
    let data = source_frame(data);
    let run = |limits: M4EffectiveResourceLimits, work| {
        let root = Root::new();
        let input = prepare_book_v2_resources(
            body_with_source(&root, data.clone(), b"Result", &limits),
            &root.context(),
            &config_with_extension(
                limits.base().get().clone(),
                limits.extension().get().clone(),
            ),
            &limits,
        )
        .unwrap();
        let mut called = false;
        let result = with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            work,
            |pdf, observed| {
                called = true;
                assert_eq!(observed.candidate_passes(), 3);
                assert_eq!(observed.page_passes(), 6);
                assert_eq!(observed.line_reshape_passes(), 6);
                assert!(pdf.page_reference_labels_match());
                assert_eq!(pdf.page_references().len(), 2);
                let reference = pdf.page_references()[0];
                assert_eq!(reference.owner(), forward);
                assert!(reference.target_page().unwrap() > 1);
                assert_eq!(reference.candidate_page(), reference.target_page().unwrap());
                let reference = pdf.page_references()[1];
                assert_eq!(reference.owner(), unused);
                assert_eq!(reference.candidate_page(), 1);
                assert_eq!(reference.target_page(), None);
                assert!(!reference.is_placed());
                assert!(observed.output_charge() > pdf.bytes().len() as u64 * 2);
                assert_eq!(observed.output_charge(), pdf.output_charge());
                if limits == driver_limits() && work == 100_000_000 {
                    crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                }
                (observed, typaxis_core::sha256(pdf.bytes()))
            },
        );
        assert_eq!(called, result.is_ok());
        result
    };
    let full = run(driver_limits(), 100_000_000).unwrap();
    assert_eq!(run(driver_limits(), full.0.work_steps()).unwrap(), full);
    assert!(run(driver_limits(), full.0.work_steps() - 1).is_err());
    assert!(matches!(
        run(limits(), 100_000_000),
        Err(CE::Limit("line passes"))
    ));
    let base = driver_limits();
    for (field, exact) in [
        ("records", full.0.record_charge()),
        ("spool", full.0.spool_charge()),
        ("output", full.0.output_charge()),
        ("pages", 6),
        ("lines", 6),
    ] {
        for short in [false, true] {
            let mut caps = base.base().get().clone();
            let value = exact - u64::from(short);
            match field {
                "records" => caps.max_fragments = value,
                "spool" => caps.max_spool_bytes = value,
                "output" => caps.max_output_bytes = value,
                "pages" => caps.max_layout_passes = value as u16,
                "lines" => caps.max_line_reshape_passes = value as u16,
                _ => unreachable!(),
            }
            let mut extension = base.extension().get().clone();
            extension.max_font_subset_bytes =
                extension.max_font_subset_bytes.min(caps.max_spool_bytes);
            let limits = M4EffectiveResourceLimits::new(
                typaxis_core::ValidatedResourceLimits::new(caps).unwrap(),
                extension,
            )
            .unwrap();
            let result = run(limits, 100_000_000);
            if short {
                assert!(result.is_err(), "{field} unexpectedly reset its allowance");
            } else {
                let (observation, hash) = result.unwrap_or_else(|e| panic!("{field}: {e}"));
                assert_eq!(hash, full.1);
                assert_eq!(observation, full.0);
            }
        }
    }
}
#[test]
fn book_v2_driver_finishes_reference_free_pdf_with_one_candidate_and_real_page_proof() {
    let data = source_frame(notes_table_data());
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, data, b"Result", &limits);
    with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            assert!(pdf.page_references().is_empty());
            assert_eq!(observation.candidate_passes(), 1);
            assert_eq!(observation.page_passes(), 2);
            assert_eq!(observation.line_reshape_passes(), 2);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            let terminals = pdf
                .navigation()
                .source()
                .source()
                .source()
                .source()
                .display()
                .source();
            assert_math_display(terminals, input.resources(), &limits);
        },
    )
    .unwrap();
}
#[test]
fn book_v2_driver_keeps_unselected_source_masters_out_of_pdf_geometry() {
    let mut data = source_frame(source_data("Result"));
    let mut second = data["page_masters"]["masters"][0].clone();
    second["master_id"] = "unselected".into();
    data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, data, b"Result", &limits);
    with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observed| {
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_driver_selects_physical_masters_and_converts_cross_page_destinations() {
    use typaxis_syntax::book_v2::{
        select_book_v2_page_master as select, BookV2PageMasterError as PE,
    };
    let mut data = selected_master_data("Result");
    let root = Root::new();
    let limits = driver_limits();
    let input = prepared(&root, data.clone(), b"Result", &limits);
    for (page, expected) in [(0, "c-first"), (1, "b-even"), (2, "a-base")] {
        let mut work = 0;
        let choice = select(input.body().styled(), page, None, &mut work, 1000).unwrap();
        assert_eq!(choice.master().master_id, expected);
        assert_eq!(choice.advanced().master_id, expected);
        assert_eq!(choice.page_index(), page);
        assert!(std::ptr::eq(choice.source(), input.body().styled()));
        let mut exact = 0;
        assert_eq!(
            select(input.body().styled(), page, None, &mut exact, work)
                .unwrap()
                .master()
                .master_id,
            expected
        );
        assert_eq!(exact, work);
        assert!(matches!(
            select(input.body().styled(), page, None, &mut 0, work - 1),
            Err(PE::WorkLimit)
        ));
        assert_eq!(
            select(input.body().styled(), page, Some("appendix"), &mut 0, 1000)
                .unwrap()
                .master()
                .master_id,
            "d-named"
        );
    }
    assert!(matches!(
        select(
            input.body().styled(),
            limits.base().get().max_pages,
            None,
            &mut 0,
            1000
        ),
        Err(PE::PageLimit)
    ));
    assert!(matches!(
        select(input.body().styled(), u32::MAX, None, &mut 0, 1000),
        Err(PE::PageLimit)
    ));
    with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observed| {
            assert_eq!(pdf.navigation().source().source().source().pages().len(), 3);
            assert!(pdf.page_reference_labels_match());
            assert_eq!(
                pdf.page_references()
                    .iter()
                    .map(|r| r.target_page())
                    .collect::<Vec<_>>(),
                [Some(3), Some(1), Some(2)]
            );
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
        },
    )
    .unwrap();
    // Component callers without source-width feedback keep the strict contract.
    data["page_masters"]["masters"][1]["body"]["width"] = 9_000_000.into();
    let root = Root::new();
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    assert!(matches!(typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_flow(
        &flow, &mut 0, 100_000_000), Err(PE::HorizontalReflow)));

}

fn selected_master_data(text: &str) -> Value {
    let mut data = source_frame(source_data(text));
    data["document"]["footnotes"] = json!([]);
    let original = data["document"]["blocks"][0].clone();
    let span = original["span"].clone();
    let mut blocks = Vec::new();
    for (i, name, target) in [
        (0, "first", "last"),
        (1, "middle", "first"),
        (2, "last", "middle"),
    ] {
        if i > 0 {
            blocks.push(json!({"kind":"page_break","node_id":0,"span":span,"classes":[]}));
        }
        let mut block = original.clone();
        block["anchor_id"] = name.into();
        block["blocks"][0]["children"].as_array_mut().unwrap().push(
            json!({"kind":"reference","node_id":0,"span":span,"target":target,"format":"page"}),
        );
        blocks.push(block);
    }
    data["document"]["blocks"] = json!(blocks);
    let original = data["page_masters"]["masters"][0].clone();
    let mut masters = Vec::new();
    for (id, width, height) in [
        ("a-base", 240, 200),
        ("b-even", 260, 220),
        ("c-first", 280, 240),
        ("d-named", 300, 260),
    ] {
        let mut master = original.clone();
        master["master_id"] = id.into();
        master["width"] = (width * 65536).into();
        master["height"] = (height * 65536).into();
        master["trim"] =
            json!({"x":7*65536,"y":3*65536,"width":(width-14)*65536,"height":(height-20)*65536});
        master["footnote"] = Value::Null;
        masters.push(master);
    }
    data["page_masters"]["default_master_id"] = "a-base".into();
    data["page_masters"]["masters"] = json!(masters);
    data["page_masters"]["selection_rules"] = json!([
        {"master_id":"a-base","parity":"even","first":null,"named_page":null,"source_order":0},
        {"master_id":"c-first","parity":"any","first":true,"named_page":null,"source_order":1},
        {"master_id":"d-named","parity":"any","first":null,"named_page":"appendix","source_order":2},
        {"master_id":"b-even","parity":"even","first":null,"named_page":null,"source_order":3}
    ]);
    renumber(&mut data["document"], &mut 0);
    data
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_selected_page_masters_render_original_harano() {
    let root = Root::new();
    let limits = driver_limits();
    let text = "左側右側";
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hash = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let mut data = selected_master_data(text);
    data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
    data["resources"]["font_faces"][0]["expected_sha256"] = hash.into();
    let body = body_with_source(&root, data, text.as_bytes(), &limits);
    fs::write(root.0.join("body.bin"), bytes).unwrap();
    let input = prepare_book_v2_resources(
        body,
        &root.context(),
        &config(limits.base().get().clone()),
        &limits,
    )
    .unwrap();
    with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observed| {
            assert_eq!(pdf.navigation().source().source().source().pages().len(), 3);
            assert!(pdf.page_reference_labels_match());
            assert_eq!(
                pdf.page_references()
                    .iter()
                    .map(|r| r.target_page())
                    .collect::<Vec<_>>(),
                [Some(3), Some(1), Some(2)]
            );
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
        },
    )
    .unwrap();
}

fn varying_page_frames_data(text: &str, notes: bool) -> Value {
    let mut data = selected_master_data(text);
    let plain = data["document"]["blocks"][0]["blocks"][0].clone();
    let mut paragraph = plain.clone();
    paragraph["children"]
        .as_array_mut()
        .unwrap()
        .retain(|n| n["kind"] != "reference");
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"varying","selector":"paragraph","source_order":rules.len(),"extends":null,"declarations":[{"name":"line_height","important":false,"value":{"kind":"length","value":16*65536}}]}));
    let span = paragraph["span"].clone();
    if notes {
        paragraph["children"].as_array_mut().unwrap().push(
            json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"note"}),
        );
        let mut note = paragraph.clone();
        note["children"]
            .as_array_mut()
            .unwrap()
            .retain(|n| n["kind"] != "footnote_reference");
        data["document"]["footnotes"] = json!([{"node_id":0,"span":paragraph["span"],"footnote_id":"note","blocks":vec![note; 7]}]);
    }
    data["document"]["blocks"] = json!(vec![paragraph; 11]);
    for (i, y, height, ny, nh) in [
        (0, 30, 48, 100, 48),
        (1, 20, 32, 120, 32),
        (2, 10, 16, 150, 16),
        (3, 40, 64, 160, 48),
    ] {
        let master = &mut data["page_masters"]["masters"][i];
        master["body"]["y"] = (y * 65536).into();
        master["body"]["height"] = (height * 65536).into();
        if notes {
            master["footnote"] = json!({"x":500_000,"y":ny*65536,"width":10_000_000,"height":nh*65536+typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW});
        }
    }
    renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_driver_uses_actual_body_and_footnote_heights() {
    use typaxis_syntax::book_v2::{
        prepare_book_v2_page_frame_plan as plan, BookV2PageMasterError as PE,
    };
    for notes in [false, true] {
        let data = varying_page_frames_data("Result", notes);
        let root = Root::new();
        let limits = driver_limits();
        let input = prepared(&root, data, b"Result", &limits);
        let mut work = 0;
        let frames = plan(input.body().styled(), &mut work, 1000).unwrap();
        assert_eq!(frames.measurement_body().height().get().raw(), 48 * 65536);
        assert!(std::ptr::eq(frames.source(), input.body().styled()));
        assert_eq!(
            plan(input.body().styled(), &mut 0, work)
                .unwrap()
                .measurement_body(),
            frames.measurement_body()
        );
        assert!(matches!(
            plan(input.body().styled(), &mut 0, work - 1),
            Err(PE::WorkLimit)
        ));
        assert!(matches!(
            frames.page(limits.base().get().max_pages),
            Err(PE::PageLimit)
        ));
        assert_eq!(frames.page(3).unwrap(), frames.page(1).unwrap());
        assert_eq!(frames.page(4).unwrap(), frames.page(2).unwrap());
        let run = |maximum, capture| {
            with_converged_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                maximum,
                |pdf, observed| {
                    let marked = pdf.navigation().source().source().source();
                    let closure = marked.source().display().source().source();
                    let pages = closure.geometry().pages();
                    assert_eq!(pages.len(), 5, "notes={notes}");
                    let expected = [1, 2, 3, 2, 3];
                    for (i, page) in pages.iter().enumerate() {
                        let actual = page.selection();
                        assert_eq!(actual.body_bounds(), frames.page(i as u32).unwrap().body());
                        assert_eq!(
                            actual.declared_footnote_region(),
                            frames.page(i as u32).unwrap().footnote()
                        );
                        assert_eq!(
                            actual.candidate().used_height().raw(),
                            expected[i] * 16 * 65536
                        );
                        if let Some(bounds) = actual.candidate().footnote_bounds() {
                            let declared = actual.declared_footnote_region().unwrap();
                            assert_eq!(
                                bounds.y().checked_add(bounds.height().get()),
                                declared.y().checked_add(declared.height().get())
                            );
                            assert!(bounds.height().get() <= declared.height().get());
                        }
                    }
                    if capture {
                        crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                    }
                    (observed, typaxis_core::sha256(pdf.bytes()))
                },
            )
        };
        let full = run(100_000_000, true).unwrap();
        assert_eq!(run(full.0.work_steps(), false).unwrap(), full);
        assert!(run(full.0.work_steps() - 1, false).is_err());
    }
}

fn varying_table_frames_data(in_note: bool) -> Value {
    let mut data = varying_page_frames_data("Result", in_note);
    let mut paragraph = data["document"]["blocks"][0].clone();
    paragraph["children"]
        .as_array_mut()
        .unwrap()
        .retain(|n| n["kind"] != "footnote_reference");
    let mut right = paragraph.clone();
    paragraph["classes"] = json!(["left"]);
    right["classes"] = json!(["right"]);
    let mut table = nested_tables::nested("Result", 3, "natural")["document"]["blocks"][0].clone();
    table["body"][0]["cells"][0]["blocks"] = json!(vec![paragraph; 4]);
    table["body"][0]["cells"][1]["blocks"] = json!(vec![right; 4]);
    if in_note {
        data["document"]["blocks"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        data["document"]["footnotes"][0]["blocks"] = json!([table]);
    } else {
        data["document"]["blocks"] = json!([table]);
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    for (name, height) in [("left", 17), ("right", 23)] {
        rules.push(json!({"style_id":name,"selector":format!("paragraph.{name}"),"source_order":rules.len(),"extends":null,"declarations":[{"name":"line_height","important":false,"value":{"kind":"length","value":height*65536}}]}));
    }
    for (i, height) in [(0, 96), (1, 48), (2, 24)] {
        let master = &mut data["page_masters"]["masters"][i];
        master["body"]["height"] = (height * 65536).into();
        if in_note {
            master["footnote"]["y"] = (80 * 65536).into();
            master["footnote"]["height"] =
                (height * 65536 + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
        }
    }
    renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_driver_continues_unequal_cells_on_smaller_selected_pages() {
    for in_note in [false, true] {
        let data = varying_table_frames_data(in_note);
        let root = Root::new();
        let limits = driver_limits();
        let input = prepared(&root, data, b"Result", &limits);
        with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observed| {
                let pages = pdf
                    .navigation()
                    .source()
                    .source()
                    .source()
                    .source()
                    .display()
                    .source()
                    .source()
                    .geometry()
                    .pages();
                assert_eq!(pages.len(), 3, "in_note={in_note}");
                for (page, height) in pages.iter().zip([23, 46, 23]) {
                    let candidate = page.selection().candidate();
                    let used = if in_note {
                        candidate.footnotes().unwrap().used_height()
                    } else {
                        candidate.used_height()
                    };
                    assert_eq!(used.raw(), height * 65536);
                }
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_page_frames_reflow_tables_and_do_not_skip_a_short_page() {
    for (region, field) in [("body", "width"), ("footnote", "width")] {
        let mut data = varying_table_frames_data(region == "footnote");
        let value = &mut data["page_masters"]["masters"][1][region][field];
        *value = (value.as_i64().unwrap() + 1).into();
        let root = Root::new();
        let limits = driver_limits();
        let input = prepared(&root, data, b"Result", &limits);
        let result = with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observed| {
                assert!(observed.width_feedback_passes() >= 2);
                assert!(pdf.navigation().source().source().source().source().display().source().source().flow().lines().frames().unwrap().uses_table_occurrence_frames());
            },
        );
        result.unwrap();
    }
    let mut data = varying_page_frames_data("Result", false);
    data["page_masters"]["masters"][2]["body"]["height"] = (8 * 65536).into();
    let root = Root::new();
    let limits = driver_limits();
    let input = prepared(&root, data, b"Result", &limits);
    let error = with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |_, _| panic!("short first page skipped"),
    )
    .err()
    .unwrap();
    assert!(
        matches!(error,CE::Stage{stage:"page stability",ref source} if source.downcast_ref::<typaxis_pagination::ProductionBodyPaginationError>().is_some_and(|e| e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::JointPageNoFit)),
        "{error:?}"
    );
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_varying_page_frames_render_original_harano() {
    let root = Root::new();
    let limits = driver_limits();
    let text = "左側右側";
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hash = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let mut data = varying_page_frames_data(text, true);
    data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
    data["resources"]["font_faces"][0]["expected_sha256"] = hash.into();
    data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["declarations"][0]["value"]["value"] = (20 * 65536).into();
    for (i, height) in [(0, 60), (1, 40), (2, 20)] {
        let master = &mut data["page_masters"]["masters"][i];
        master["body"]["height"] = (height * 65536).into();
        master["footnote"]["height"] =
            (height * 65536 + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
    }
    let body = body_with_source(&root, data, text.as_bytes(), &limits);
    fs::write(root.0.join("body.bin"), bytes).unwrap();
    let input = prepare_book_v2_resources(
        body,
        &root.context(),
        &config(limits.base().get().clone()),
        &limits,
    )
    .unwrap();
    with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observed| {
            assert_eq!(pdf.navigation().source().source().source().pages().len(), 5);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
        },
    )
    .unwrap();
}

#[path = "book_v2_named_page_tests.rs"]
mod named_pages;

#[path = "book_v2_horizontal_origin_tests.rs"]
mod horizontal_origins;

#[path = "book_v2_variable_width_pdf_tests.rs"]
mod variable_widths;

#[path = "book_v2_table_width_pdf_tests.rs"]
mod table_widths;

#[path = "book_v2_table_width_occurrence_tests.rs"]
mod table_width_occurrences;

#[path = "book_v2_source_unit_start_tests.rs"]
mod source_unit_starts;

#[path = "book_v2_table_source_profile_pdf_tests.rs"]
mod table_source_profiles;

#[path = "book_v2_line_variant_seed_tests.rs"]
mod line_variant_seeds;
