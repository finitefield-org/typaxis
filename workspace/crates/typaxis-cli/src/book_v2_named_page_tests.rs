use super::*;
fn page_rule(data: &mut Value, id: &str, selector: &str, name: &str) {
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":id,"selector":selector,"source_order":rules.len(),"extends":null,"declarations":[{"name":"page","important":false,"value":{"kind":"string","value":name}}]}));
}
pub(super) fn names_data(text: &str, mode: &str) -> (Value, Vec<Option<&'static str>>) {
    let mut data = selected_master_data(text);
    data["document"]["blocks"]
        .as_array_mut()
        .unwrap()
        .retain(|b| b["kind"] != "page_break");
    page_rule(
        &mut data,
        "named-scope",
        "semantic_container.appendix",
        "appendix",
    );
    page_rule(
        &mut data,
        "named-paragraph",
        "paragraph.appendix",
        "appendix",
    );
    page_rule(
        &mut data,
        "short-scope",
        "semantic_container.short",
        "short",
    );
    page_rule(&mut data, "short-paragraph", "paragraph.short", "short");
    page_rule(&mut data, "named-table", "table.appendix", "appendix");
    let rules = data["page_masters"]["selection_rules"]
        .as_array_mut()
        .unwrap();
    rules.push(json!({"master_id":"b-even","parity":"any","first":null,"named_page":"short","source_order":4}));
    let expected;
    if mode == "references" {
        data["document"]["blocks"][1]["classes"] = json!(["appendix"]);
        expected = vec![None, Some("appendix"), None];
    } else {
        let mut wrapper = data["document"]["blocks"][0].clone();
        wrapper["anchor_id"] = Value::Null;
        let mut plain = wrapper["blocks"][0].clone();
        plain["children"]
            .as_array_mut()
            .unwrap()
            .retain(|n| n["kind"] != "reference");
        wrapper["classes"] = json!(["appendix"]);
        if mode == "nested" {
            let mut child = wrapper.clone();
            child["classes"] = json!(["short"]);
            child["blocks"] = json!([plain.clone()]);
            wrapper["blocks"] = json!([plain.clone(), child, plain.clone()]);
            data["document"]["blocks"] = json!([wrapper, plain]);
            expected = vec![Some("appendix"), Some("short"), Some("appendix"), None];
        } else if mode == "forced" || mode == "explicit" || mode == "standalone" {
            let br = json!({"kind":"page_break","node_id":0,"span":plain["span"],"classes":[]});
            wrapper["blocks"] =
                json!([br.clone(), plain.clone(), br.clone(), br.clone(), plain, br]);
            data["document"]["blocks"] = json!([wrapper]);
            if mode == "explicit" || mode == "standalone" {
                for i in [0, 2, 5] {
                    data["document"]["blocks"][0]["blocks"][i]["classes"] = json!(["short"]);
                }
                page_rule(&mut data, "explicit-break", "page_break.short", "short");
                let enclosing = if mode == "standalone" {
                    None
                } else {
                    Some("appendix")
                };
                if mode == "standalone" {
                    data["document"]["blocks"] = data["document"]["blocks"][0]["blocks"].clone();
                }
                expected = vec![
                    Some("short"),
                    enclosing,
                    Some("short"),
                    enclosing,
                    enclosing,
                    Some("short"),
                    Some("short"),
                ];
            } else {
                expected = vec![Some("appendix"); 5];
            }
        } else if mode == "table" {
            let mut table = nested_tables::nested(text, text.len() / 2, "natural")["document"]
                ["blocks"][0]
                .clone();
            table["classes"] = json!(["appendix"]);
            for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
                cell["blocks"] = json!(vec![plain.clone(); 4]);
            }
            data["document"]["blocks"] = json!([table, plain]);
            data["page_masters"]["masters"][3]["body"]["height"] = (32 * 65536).into();
            expected = vec![Some("appendix"), Some("appendix"), None];
        } else if mode == "notes" {
            let named_styles = data["style_sheet"].clone();
            let selections = data["page_masters"]["selection_rules"].clone();
            data = varying_page_frames_data(text, true);
            data["style_sheet"] = named_styles;
            data["page_masters"]["selection_rules"] = selections;
            data["document"]["blocks"]
                .as_array_mut()
                .unwrap()
                .truncate(1);
            data["document"]["blocks"][0]["classes"] = json!(["appendix"]);
            data["page_masters"]["masters"][3]["footnote"]["height"] =
                (32 * 65536 + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
            expected = vec![Some("appendix"); 4];
        } else {
            panic!("unknown mode");
        }
    }
    renumber(&mut data["document"], &mut 0);
    (data, expected)
}
#[test]
fn book_v2_named_pages_preserve_scope_references_table_and_note_continuations() {
    for mode in [
        "references",
        "nested",
        "forced",
        "explicit",
        "standalone",
        "table",
        "notes",
    ] {
        let (data, expected) = names_data("Result", mode);
        let root = Root::new();
        let limits = driver_limits();
        let input = prepared(&root, data, b"Result", &limits);
        let run = |work, capture| {
            with_converged_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                work,
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
                    assert_eq!(
                        pages
                            .iter()
                            .map(|p| p.selection().named_page())
                            .collect::<Vec<_>>(),
                        expected,
                        "{mode}"
                    );
                    if mode == "references" {
                        assert_eq!(
                            pdf.page_references()
                                .iter()
                                .map(|r| r.target_page())
                                .collect::<Vec<_>>(),
                            [Some(3), Some(1), Some(2)]
                        );
                    }
                    if capture {
                        crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                    }
                    (observed, typaxis_core::sha256(pdf.bytes()))
                },
            )
        };
        let full = run(100_000_000, true).unwrap_or_else(|e| panic!("{mode}: {e:?}"));
        assert_eq!(run(full.0.work_steps(), false).unwrap(), full);
        assert!(run(full.0.work_steps() - 1, false).is_err());
    }
}

#[test]
fn book_v2_named_page_plan_binds_scope_and_charges_cumulative_limits() {
    use typaxis_syntax::book_v2::{
        prepare_book_v2_page_frame_plan_for_flow_with_prior as plan, BookV2PageMasterError as PE,
    };
    let (data, _) = names_data("Result", "nested");
    let root = Root::new();
    let limits = driver_limits();
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let mut work = 0;
    let full = plan(&flow, &mut work, 100_000_000, 0, 0).unwrap();
    let maximum = work;
    assert!(full.has_source_names());
    assert!(std::ptr::eq(full.source(), input.body().styled()));
    let caps = limits.base().get();
    let records = caps.max_fragments - full.record_charge();
    let spool = caps.max_spool_bytes - full.spool_charge();
    let exact = plan(&flow, &mut 0, maximum, records, spool).unwrap();
    assert_eq!(exact.record_charge(), full.record_charge());
    assert_eq!(exact.spool_charge(), full.spool_charge());
    assert!(matches!(
        plan(&flow, &mut 0, maximum - 1, records, spool),
        Err(PE::WorkLimit)
    ));
    assert!(matches!(
        plan(&flow, &mut 0, maximum, records + 1, spool),
        Err(PE::RecordLimit)
    ));
    assert!(matches!(
        plan(&flow, &mut 0, maximum, records, spool + 1),
        Err(PE::SpoolLimit)
    ));
    let owners = flow
        .paragraphs()
        .iter()
        .map(|p| p.owner())
        .collect::<Vec<_>>();
    assert_eq!(
        owners
            .iter()
            .map(|o| full.source_name_index(*o).map(|i| full.name(i).unwrap()))
            .collect::<Vec<_>>(),
        [Some("appendix"), Some("short"), Some("appendix"), None]
    );
    assert!(matches!(
        full.named_page(0, Some(usize::MAX)),
        Err(PE::Identity)
    ));
}

#[test]
fn book_v2_named_pages_reject_keep_conflicts_and_parallel_or_definition_names() {
    for mode in [
        "keep",
        "parallel",
        "definition",
        "parallel-break",
        "definition-break",
    ] {
        let (mut data, _) = names_data(
            "Result",
            if mode.starts_with("parallel") {
                "table"
            } else if mode.starts_with("definition") {
                "notes"
            } else {
                "references"
            },
        );
        if mode.ends_with("-break") {
            page_rule(&mut data, "explicit-break", "page_break.short", "short");
            let span = data["document"]["blocks"][0]["span"].clone();
            let br = json!({"kind":"page_break","node_id":0,"span":span,"classes":["short"]});
            if mode == "parallel-break" {
                data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .insert(0, br);
            } else {
                data["document"]["footnotes"][0]["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .insert(0, br);
            }
            renumber(&mut data["document"], &mut 0);
        } else if mode == "keep" {
            data["document"]["blocks"][0]["blocks"][0]["classes"] = json!(["keep"]);
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":"keep","selector":"paragraph.keep","source_order":rules.len(),"extends":null,"declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
        } else if mode == "parallel" {
            data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]["classes"] =
                json!(["short"]);
        } else {
            data["document"]["footnotes"][0]["blocks"][0]["classes"] = json!(["short"]);
        }
        let root = Root::new();
        let limits = driver_limits();
        let input = prepared(&root, data, b"Result", &limits);
        let error = with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |_, _| panic!("conflicting named pages reached PDF"),
        )
        .err()
        .unwrap();
        let expected = if mode == "keep" {
            typaxis_pagination::ProductionBodyPaginationErrorKind::KeepAcrossForcedBreak
        } else {
            typaxis_pagination::ProductionBodyPaginationErrorKind::PendingNamedPage
        };
        assert!(
            matches!(error,CE::Stage{ref source,..} if source.downcast_ref::<typaxis_pagination::ProductionBodyPaginationError>().is_some_and(|e|e.kind==expected)),
            "{mode}: {error:?}"
        );
    }
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_named_pages_render_original_harano() {
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
    for mode in ["references", "nested", "explicit", "standalone"] {
        let root = Root::new();
        let limits = driver_limits();
        let (mut data, expected) = names_data(text, mode);
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = hash.clone().into();
        let body = body_with_source(&root, data, text.as_bytes(), &limits);
        fs::write(root.0.join("body.bin"), &bytes).unwrap();
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
                assert_eq!(
                    pages
                        .iter()
                        .map(|p| p.selection().named_page())
                        .collect::<Vec<_>>(),
                    expected
                );
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_named_break_source_and_plan_keep_exact_original_owners_and_budgets() {
    use typaxis_syntax::book_v2::prepare_book_v2_page_frame_plan_for_flow_with_prior as plan;
    let (data, _) = names_data("Result", "explicit");
    let root = Root::new();
    let limits = driver_limits();
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    flow.verify_for(input.body().styled(), &nav).unwrap();
    assert_eq!(flow.named_page_breaks().len(), 3);
    let mut work = 0;
    let full = plan(&flow, &mut work, 100_000_000, 0, 0).unwrap();
    for (owner, name) in flow.named_page_breaks() {
        assert_eq!(name.as_str(), "short");
        assert_eq!(
            full.name(full.source_name_index(*owner).unwrap()),
            Some("short")
        );
    }
    let records = limits.base().get().max_fragments - full.record_charge();
    let spool = limits.base().get().max_spool_bytes - full.spool_charge();
    assert!(plan(&flow, &mut 0, work, records, spool).is_ok());
    assert!(plan(&flow, &mut 0, work - 1, records, spool).is_err());
    assert!(plan(&flow, &mut 0, work, records + 1, spool).is_err());
    assert!(plan(&flow, &mut 0, work, records, spool + 1).is_err());
}
