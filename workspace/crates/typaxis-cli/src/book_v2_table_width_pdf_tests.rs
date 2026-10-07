use super::*;
use typaxis_layout::book_v2::{
    bind_book_v2_vectors, with_converged_book_v2_body_lines_with_source_widths,
};
use typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_reflow;

#[test]
fn book_v2_table_widths_reject_unrebound_physical_finalization() {
    for notes in [false, true] {
        let root = Root::new();
        let limits = driver_limits();
        let text = "Pro Pro Pro";
        let input = prepared(
            &root,
            table_page_data(text, notes),
            text.as_bytes(),
            &limits,
        );
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let plan =
            prepare_book_v2_page_frame_plan_for_reflow(&flow, &mut 0, 1_000_000, 0, 0).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings = bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
        with_converged_book_v2_body_lines_with_source_widths(
            &policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,
            plan.measurement_body(),10_000_000,None,limits.base().get().max_line_reshape_passes,Some(&plan),None,
            |lines| {
                let body=prepare_book_v2_body_flow(lines.lines(),None,lines.footnotes(),&limits,0).unwrap();
                let measured=prepare_book_v2_table_measurements(body,&limits).unwrap();
                let mut search=prepare_book_v2_table_body_search(&measured,&limits,10_000_000,measured.record_charge()).unwrap();
                let stable=search.select_stable_mixed_pages(limits.base().get().max_layout_passes).unwrap();
                let placed=search.place_mixed_pages(stable.sequence()).unwrap();
                let closed=search.close_mixed_page_sources(&stable,&placed).unwrap();
                let feedback=search.paragraph_width_feedback(&closed).unwrap();
                assert!(!feedback.matches_selected_table_widths());
                assert_eq!(feedback.root_table_widths().len(),3);
                assert!(matches!(search.finalize_mixed_page_math(closed,&limits,0),Err(e) if e.kind==typaxis_pagination::ProductionBodyPaginationErrorKind::WidthMismatch));
            }
        ).unwrap();
    }
}

fn table_page_data(text: &str, notes: bool) -> Value {
    let mut data =
        crate::book_v2_resources::tests::shaping_tests::table_width_frames::table_data(text, false);
    let table = data["document"]["blocks"][0].clone();
    let para = data["document"]["blocks"][1].clone();
    let span = para["span"].clone();
    let br = json!({"kind":"page_break","node_id":0,"span":span,"classes":[]});
    if notes {
        let mut blocks = Vec::new();
        let mut definitions = Vec::new();
        for index in 0..3 {
            let id = format!("table-width-{index}");
            let mut p = para.clone();
            p["children"].as_array_mut().unwrap().push(
                json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":id}),
            );
            if index != 0 {
                blocks.push(br.clone());
            }
            blocks.push(p);
            definitions.push(json!({"node_id":0,"span":span,"footnote_id":id,"blocks":[table]}));
        }
        data["document"]["blocks"] = blocks.into();
        data["document"]["footnotes"] = definitions.into();
    } else {
        data["document"]["blocks"] = json!([table, br, table, br, table]);
    }
    data["page_masters"] = selected_master_data(text)["page_masters"].clone();
    for (i, w, nw) in [(0, 220, 230), (1, 180, 210), (2, 160, 200), (3, 200, 220)] {
        let m = &mut data["page_masters"]["masters"][i];
        m["width"] = (300 * 65536).into();
        m["height"] = (2100 * 65536).into();
        m["trim"] = json!({"x":0,"y":0,"width":300*65536,"height":2100*65536});
        m["body"] = json!({"x":10*65536,"y":10*65536,"width":w*65536,"height":1000*65536});
        if notes {
            m["footnote"] =
                json!({"x":20*65536,"y":1100*65536,"width":nw*65536,"height":900*65536});
        }
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":format!("table-page-width-{}",if notes {"notes"}else{"body"}),"selector":"paragraph","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"text_align","important":false,"value":{"kind":"keyword","value":"start"}}]}));
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    data
}

fn verify_table_pdfs(font: Option<&[u8]>) {
    for mode in ["body", "notes", "continuation"] {
        let notes = mode == "notes";
        let root = Root::new();
        let limits = driver_limits();
        let text = if font.is_some() {
            "表の本文を元の字形で組む"
        } else {
            "Pro Pro Pro"
        };
        let mut data = table_page_data(text, notes);
        if mode == "continuation" {
            let caption = data["document"]["blocks"][0]["caption"][0].clone();
            let br = data["document"]["blocks"][1].clone();
            data["document"]["blocks"][0]["caption"] = json!([caption, br, caption]);
            data["document"]["blocks"]
                .as_array_mut()
                .unwrap()
                .truncate(3);
            data["page_masters"]["masters"][1]["body"]["width"] = (160 * 65536).into();
            for rule in data["style_sheet"]["rules"].as_array_mut().unwrap() {
                if rule["style_id"] == "table-page-width-body" {
                    rule["style_id"] = "table-page-width-continuation".into();
                }
            }
            crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
                &mut data["document"],
                &mut 0,
            );
        }
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
                    assert_eq!(pdf.navigation().source().source().source().pages().len(), 3);
                    assert!(observed.width_feedback_passes() >= 2);
                    crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                    if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_V2_TABLE_WIDTH_PROBE") {
                        let directory = std::path::PathBuf::from(directory);
                        fs::create_dir_all(&directory).unwrap();
                        let name = format!(
                            "{}-{}",
                            if font.is_some() {
                                "harano"
                            } else {
                                "controlled"
                            },
                            mode
                        );
                        fs::write(directory.join(format!("{name}.pdf")), pdf.bytes()).unwrap();
                        fs::write(directory.join(format!("{name}.json")),serde_json::to_vec_pretty(&json!({"work":observed.work_steps(),"width_passes":observed.width_feedback_passes(),"line_passes":observed.line_reshape_passes(),"page_passes":observed.page_passes()})).unwrap()).unwrap();
                    }
                    (observed.work_steps(), typaxis_core::sha256(pdf.bytes()))
                },
            )
        };
        let full = run(100_000_000).unwrap();
        assert_eq!(run(full.0).unwrap(), full);
        assert!(run(full.0 - 1).is_err());
    }
}

#[test]
fn book_v2_table_widths_converge_original_nested_body_and_note_pdfs() {
    verify_table_pdfs(None);
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_widths_converge_original_harano_pdfs() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    verify_table_pdfs(Some(&font));
}
