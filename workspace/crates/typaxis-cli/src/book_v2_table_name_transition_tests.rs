use super::*;
use table_caption_breaks::renumber;

fn transition_data(text: &str, mode: &str) -> Value {
    let (mut data, _) = names_data(text, "table");
    let table = &mut data["document"]["blocks"][0];
    table["classes"] = json!(["appendix", format!("transition-{mode}")]);
    for (column, cell) in table["body"][0]["cells"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let boundary = if mode == "unequal" {
            if column == 0 {
                1
            } else {
                3
            }
        } else if mode == "return" {
            1
        } else {
            2
        };
        for (index, paragraph) in cell["blocks"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            paragraph["classes"] = if index >= boundary {
                json!(["short"])
            } else {
                json!([])
            };
            if mode == "return" && index == 3 {
                paragraph["classes"] = json!([]);
            }
            if mode == "unnamed" {
                paragraph["classes"] = if index < 2 {
                    json!(["appendix"])
                } else {
                    json!([])
                };
            }
        }
    }
    if mode == "nested" || mode == "nested-scopes" {
        let mut child = table.clone();
        child["classes"] = json!(["appendix"]);
        for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
            if mode == "nested-scopes" {
                let mut first = child.clone();
                let mut second = child.clone();
                for c in first["body"][0]["cells"].as_array_mut().unwrap() {
                    c["blocks"].as_array_mut().unwrap().truncate(2);
                }
                for c in second["body"][0]["cells"].as_array_mut().unwrap() {
                    c["blocks"] = json!(c["blocks"].as_array().unwrap()[2..].to_vec());
                }
                cell["blocks"] = json!([first, second]);
            } else {
                cell["blocks"] = json!([child.clone()]);
            }
        }
    } else if mode == "rows" || mode == "header" {
        let mut second = table["body"][0].clone();
        for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
            cell["blocks"]
                .as_array_mut()
                .unwrap()
                .truncate(if mode == "header" { 1 } else { 2 });
        }
        for cell in second["cells"].as_array_mut().unwrap() {
            cell["blocks"] = json!(cell["blocks"].as_array().unwrap()[2..].to_vec());
        }
        if mode == "header" {
            table["head"] = table["body"].clone();
            table["body"] = json!([second]);
        } else {
            table["body"].as_array_mut().unwrap().push(second);
        }
    } else if mode == "span" {
        let mut second = table["body"][0].clone();
        table["body"][0]["cells"][0]["rowspan"] = 2.into();
        table["body"][0]["cells"][1]["blocks"]
            .as_array_mut()
            .unwrap()
            .truncate(2);
        second["cells"].as_array_mut().unwrap().remove(0);
        second["cells"][0]["blocks"] =
            json!(second["cells"][0]["blocks"].as_array().unwrap()[2..].to_vec());
        table["body"].as_array_mut().unwrap().push(second);
    } else if mode == "caption" || mode == "empty-child" {
        for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
            for paragraph in cell["blocks"].as_array_mut().unwrap() {
                paragraph["classes"] = json!(["short"]);
            }
        }
        if mode == "caption" {
            let mut caption = table["body"][0]["cells"][0]["blocks"][0].clone();
            caption["classes"] = json!([]);
            table["caption"] = json!([caption]);
        } else {
            let mut child = table.clone();
            child["classes"] = json!(["appendix"]);
            for cell in child["body"][0]["cells"].as_array_mut().unwrap() {
                cell["blocks"] = json!([]);
            }
            table["body"][0]["cells"][0]["blocks"]
                .as_array_mut()
                .unwrap()
                .push(child);
        }
    } else if mode == "break" {
        for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
            let span = cell["blocks"][0]["span"].clone();
            cell["blocks"].as_array_mut().unwrap().insert(
                2,
                json!({"kind":"page_break","node_id":0,"span":span,"classes":["short"]}),
            );
        }
        page_rule(&mut data, "transition-break", "page_break.short", "short");
    }
    if mode == "unnamed" {
        data["document"]["blocks"][0]["classes"] = json!(["transition-unnamed"]);
    }
    let master = &mut data["page_masters"]["masters"][1];
    master["body"]["height"] = (40 * 65536).into();
    if mode == "width" {
        master["body"]["x"] = (18 * 65536).into();
        master["body"]["width"] = (120 * 65536).into();
    }
    renumber(&mut data["document"], &mut 0);
    data
}

fn check_transitions(font: Option<&[u8]>) {
    for mode in [
        "flat",
        "unequal",
        "nested",
        "nested-scopes",
        "rows",
        "header",
        "span",
        "caption",
        "empty-child",
        "break",
        "return",
        "unnamed",
        "width",
    ] {
        let text = match (font.is_some(), mode) {
            (true, "width") => "左側右側左側右側",
            (true, _) => "左右",
            (false, "width") => "Result Result",
            (false, "nested" | "nested-scopes") => "Res",
            (false, _) => "Result",
        };
        let root = Root::new();
        let limits = driver_limits();
        let mut data = transition_data(text, mode);
        let input = if let Some(bytes) = font {
            data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
            data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .into();
            let body = body_with_source(&root, data.clone(), text.as_bytes(), &limits);
            fs::write(root.0.join("body.bin"), bytes).unwrap();
            prepare_book_v2_resources(
                body,
                &root.context(),
                &config(limits.base().get().clone()),
                &limits,
            )
            .unwrap()
        } else {
            prepared(&root, data.clone(), text.as_bytes(), &limits)
        };
        let run = |work, capture| {
            with_converged_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                work,
                |pdf, observed| {
                    let body = pdf
                        .navigation()
                        .source()
                        .source()
                        .source()
                        .source()
                        .display()
                        .source()
                        .source();
                    let flow = body.flow();
                    let mut phases = Vec::new();
                    let mut consumed = Vec::new();
                    let mut placements = Vec::new();
                    for page in body.geometry().pages() {
                        let selection = page.selection();
                        phases.push(selection.named_page());
                        for part in selection.candidate().parts() {
                            if let Some(items) = part.items() {
                                consumed.extend(items);
                            }
                            if let Some(table) = part.table() {
                                consumed.extend(table.semantic_leaf_ranges().flatten());
                            }
                        }
                        for (placed, _, repeated) in page.fragments_with_roles() {
                            if placed.definition_index().is_some() || repeated {
                                continue;
                            }
                            let item = placed.item_index();
                            assert_eq!(
                                flow.body_page_name_index(item),
                                selection.named_page_index(),
                                "{mode}: item {item}"
                            );
                            placements.push(json!({"page":selection.page_index(), "item":item,
                                "owner":placed.fragment().owner().get(), "name":selection.named_page()}));
                        }
                    }
                    phases.dedup();
                    let expected = if mode == "empty-child" {
                        vec![Some("short"), Some("appendix"), None]
                    } else if mode == "return" {
                        vec![Some("appendix"), Some("short"), Some("appendix"), None]
                    } else if mode == "unnamed" {
                        vec![Some("appendix"), None]
                    } else {
                        vec![Some("appendix"), Some("short"), None]
                    };
                    assert_eq!(phases, expected, "{mode}");
                    consumed.sort_unstable();
                    assert_eq!(
                        consumed,
                        (0..flow.body_items().len()).collect::<Vec<_>>(),
                        "{mode}: original source once"
                    );
                    if capture && font.is_none() && mode == "unequal" {
                        check_search_budget(flow, &limits);
                    }
                    if capture {
                        crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                        if let Ok(directory) =
                            std::env::var("TYPAXIS_BOOK_V2_TABLE_NAME_TRANSITION_PROBE")
                        {
                            let directory = PathBuf::from(directory);
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
                            fs::write(directory.join(format!("{name}.json")), serde_json::to_vec(&json!({
                                "source":data,"text":text,"placements":placements,"consumed":consumed,
                                "page_names":body.geometry().pages().iter().map(|p|p.selection().named_page()).collect::<Vec<_>>(),
                                "sha256":typaxis_core::sha256(pdf.bytes()),"work":observed.work_steps()
                            })).unwrap()).unwrap();
                        }
                    }
                    (observed, typaxis_core::sha256(pdf.bytes()))
                },
            )
        };
        let full = run(100_000_000, true).unwrap_or_else(|e| panic!("{mode}: {e:?}"));
        assert_eq!(
            run(full.0.work_steps(), false).unwrap(),
            full,
            "{mode}: exact work"
        );
        assert!(
            run(full.0.work_steps() - 1, false).is_err(),
            "{mode}: work below"
        );
    }
}

#[test]
fn book_v2_table_name_transitions_preserve_parallel_source_and_master() {
    check_transitions(None);
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_name_transitions_render_original_harano() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check_transitions(Some(&bytes));
}

fn check_search_budget(
    flow: &typaxis_pagination::book_v2::BookV2PreparedBodyFlow<'_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_measurements, prepare_book_v2_table_search,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    let measured = prepare_book_v2_table_measurements(
        prepare_book_v2_body_flow(flow.lines(), flow.blocks(), flow.footnotes(), limits, 0)
            .unwrap(),
        limits,
    )
    .unwrap();
    let plan = measured
        .flow()
        .lines()
        .frames()
        .unwrap()
        .page_plan()
        .unwrap();
    let replay = |prior, maximum| -> Result<_, typaxis_pagination::ProductionBodyPaginationError> {
        let mut search = prepare_book_v2_table_search(&measured, 0, limits, maximum, prior)?;
        let mut cursor = search.begin()?;
        let mut hashes = Vec::new();
        let mut items = Vec::new();
        while !cursor.is_terminal() {
            assert!(hashes.len() < 10);
            let name = search.page_name(&cursor)?;
            let frame = plan.named_page(hashes.len() as u32, name).unwrap();
            let selected = search
                .evaluate(&cursor, frame.body().height().get())?
                .unwrap();
            items.extend(selected.semantic_leaf_ranges().flatten());
            hashes.push(selected.fingerprint());
            cursor = selected.after();
        }
        items.sort_unstable();
        assert_eq!(
            items,
            (0..measured.flow().body_items().len() - 1).collect::<Vec<_>>()
        );
        Ok((search.record_charge(), search.work_charge(), hashes))
    };
    let (records, work, hashes) = replay(0, 100_000_000).unwrap();
    let prior = limits.base().get().max_fragments - (records - measured.record_charge());
    assert_eq!(
        replay(prior, work).unwrap(),
        (limits.base().get().max_fragments, work, hashes)
    );
    assert_eq!(replay(prior + 1, work).unwrap_err().kind, E::FragmentLimit);
    assert_eq!(replay(0, work - 1).unwrap_err().kind, E::TableSearchLimit);
}

#[test]
fn book_v2_table_name_transitions_reject_keeps_and_incompatible_next_phases() {
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in ["paragraph-keep", "table-keep", "next-conflict"] {
        let mut data = transition_data("Result", "flat");
        let expected;
        let owner;
        if mode == "next-conflict" {
            let table = &mut data["document"]["blocks"][0];
            table["classes"] = json!([]);
            for (column, cell) in table["body"][0]["cells"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                for (index, p) in cell["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .enumerate()
                {
                    p["classes"] = if index < 2 {
                        json!(["appendix"])
                    } else if column == 0 {
                        json!(["short"])
                    } else {
                        json!([])
                    };
                }
            }
            expected = E::PendingNamedPage;
            owner = data["document"]["blocks"][0]["body"][0]["cells"][1]["blocks"][2]["node_id"]
                .as_u64()
                .unwrap();
        } else {
            let selector;
            if mode == "paragraph-keep" {
                data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][1]["classes"] =
                    json!(["keep"]);
                owner = data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][1]
                    ["node_id"]
                    .as_u64()
                    .unwrap();
                selector = "paragraph.keep";
            } else {
                data["document"]["blocks"][0]["classes"] = json!(["appendix", "keep"]);
                owner = data["document"]["blocks"][0]["node_id"].as_u64().unwrap();
                selector = "table.keep";
            }
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":"transition-keep","selector":selector,"source_order":rules.len(),"extends":null,
                "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
            expected = E::KeepAcrossForcedBreak;
        }
        let root = Root::new();
        let limits = driver_limits();
        let input = prepared(&root, data, b"Result", &limits);
        let error = with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |_, _| panic!("conflicting table reached PDF"),
        )
        .err()
        .unwrap();
        assert!(
            matches!(error, CE::Stage { ref source, .. }
            if source.downcast_ref::<typaxis_pagination::ProductionBodyPaginationError>().is_some_and(|e|
                e.kind == expected && u64::from(e.owner.get()) == owner)),
            "{mode}: {error:?}"
        );
    }
}
