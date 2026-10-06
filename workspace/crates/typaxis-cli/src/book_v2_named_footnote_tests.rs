use super::*;
use crate::book_v2_resources::{with_budgeted_book_v2_pdf, BookV2PdfConvergenceBudget};
use table_caption_breaks::renumber;

fn note_data(text: &str, mode: &str) -> Value {
    let (mut data, _) = names_data(text, "notes");
    data["document"]["blocks"][0]["classes"] = json!(["appendix", format!("named-note-{mode}")]);
    data["page_masters"]["masters"][3]["footnote"]["height"] =
        (64 * 65536 + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
    let notes = data["document"]["footnotes"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    for (i, note) in notes.iter_mut().enumerate() {
        note["classes"] = json!([
            if mode == "aligned" || i < 3 || (mode == "return" && i >= 5) {
                "appendix"
            } else {
                "short"
            }
        ]);
    }
    if mode == "scope" {
        let (scopes, _) = names_data(text, "nested");
        let mut first = scopes["document"]["blocks"][0].clone();
        let mut second = first.clone();
        first["blocks"] = json!(notes[..3].to_vec());
        second["classes"] = json!(["short"]);
        second["blocks"] = json!(notes[3..].to_vec());
        for region in [&mut first, &mut second] {
            for p in region["blocks"].as_array_mut().unwrap() {
                p["classes"] = json!([]);
            }
        }
        data["document"]["footnotes"][0]["blocks"] = json!([first, second]);
    } else if mode == "forced" {
        let span = notes[0]["span"].clone();
        notes.insert(
            3,
            json!({"kind":"page_break","node_id":0,"span":span,"classes":["short"]}),
        );
        page_rule(&mut data, "note-break", "page_break.short", "short");
    } else if matches!(
        mode,
        "flat" | "nested" | "header" | "span" | "caption" | "empty-child"
    ) {
        let table =
            table_transitions::transition_data(text, if mode == "flat" { "flat" } else { mode })
                ["document"]["blocks"][0]
                .clone();
        data["document"]["footnotes"][0]["blocks"] = json!([table]);
        if mode == "empty-child" {
            data["document"]["blocks"][0]["classes"] = json!(["named-note-empty-child", "short"]);
        }
        if mode == "header" {
            data["page_masters"]["masters"][1]["footnote"]["height"] =
                (48 * 65536 + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
        }
    } else if mode == "width" {
        data["page_masters"]["masters"][1]["footnote"]["x"] = (12 * 65536).into();
        data["page_masters"]["masters"][1]["footnote"]["width"] = (72 * 65536).into();
    } else if mode == "two-notes" {
        let mut other = data["document"]["footnotes"][0].clone();
        other["footnote_id"] = "second".into();
        other["blocks"].as_array_mut().unwrap().truncate(4);
        data["document"]["footnotes"]
            .as_array_mut()
            .unwrap()
            .push(other);
        let mut reference = data["document"]["blocks"][0]["children"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()
            .clone();
        reference["footnote_id"] = "second".into();
        data["document"]["blocks"][0]["children"]
            .as_array_mut()
            .unwrap()
            .push(reference);
    }
    renumber(&mut data["document"], &mut 0);
    data
}

fn check(font: Option<&[u8]>) {
    for mode in [
        "aligned",
        "serial",
        "return",
        "scope",
        "forced",
        "flat",
        "nested",
        "header",
        "span",
        "caption",
        "empty-child",
        "width",
        "two-notes",
    ] {
        let text = if font.is_some() {
            "左右"
        } else if mode == "nested" {
            "Res"
        } else {
            "Result"
        };
        let mut data = note_data(text, mode);
        let root = Root::new();
        let limits = driver_limits();
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
        let run = |maximum, capture| {
            with_converged_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                maximum,
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
                    let count = flow.footnotes().definitions().len();
                    let mut consumed = vec![Vec::<usize>::new(); count];
                    let mut marker_count = vec![0usize; count];
                    let mut placements = Vec::new();
                    let mut names = Vec::new();
                    for page in body.geometry().pages() {
                        let selection = page.selection();
                        names.push(selection.named_page());
                        if let Some(region) = selection.candidate().footnotes() {
                            for fragment in region.fragments() {
                                let fragment = fragment.fragment();
                                let index = fragment.definition_index();
                                if let Some(mixed) = fragment.mixed() {
                                    for part in mixed.parts() {
                                        if let Some(range) = part.items() {
                                            consumed[index].extend(range);
                                        }
                                        if let Some(table) = part.table() {
                                            consumed[index].extend(
                                                table
                                                    .source_leaf_ranges()
                                                    .map(Result::unwrap)
                                                    .flatten(),
                                            );
                                        }
                                    }
                                } else {
                                    consumed[index].extend(fragment.consumed_range().unwrap());
                                }
                            }
                        }
                        for marker in page.footnote_markers() {
                            marker_count[marker.definition_index()] += 1;
                        }
                        for (placed, _, repeated) in page.fragments_with_roles() {
                            if repeated {
                                continue;
                            }
                            if let Some(index) = placed.definition_index() {
                                let request =
                                    flow.definition_page_name_index(index, placed.item_index());
                                if request.is_some() {
                                    assert_eq!(
                                        request,
                                        selection.named_page_index(),
                                        "{mode}: actual definition request"
                                    );
                                }
                                placements.push(json!({"definition":index,"item":placed.item_index(),"owner":placed.fragment().owner().get(),"page":selection.page_index()}));
                            }
                        }
                    }
                    for index in 0..count {
                        consumed[index].sort_unstable();
                        assert_eq!(
                            consumed[index],
                            (0..flow.definition_items(index).unwrap().len()).collect::<Vec<_>>(),
                            "{mode}: definition {index} once"
                        );
                        assert_eq!(marker_count[index], 1, "{mode}: original marker once");
                    }
                    let mut phases = names.clone();
                    phases.dedup();
                    let expected = if mode == "aligned" {
                        vec![Some("appendix")]
                    } else if mode == "empty-child" {
                        vec![Some("short"), Some("appendix")]
                    } else if mode == "return" {
                        vec![Some("appendix"), Some("short"), Some("appendix")]
                    } else {
                        vec![Some("appendix"), Some("short")]
                    };
                    if mode != "two-notes" {
                        assert_eq!(phases, expected, "{mode}");
                    } else {
                        assert!(phases.contains(&Some("short")));
                    }
                    if capture {
                        if mode == "flat" {
                            check_source_record_budget(flow, &limits);
                        }
                        crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
                        if let Ok(directory) = std::env::var("TYPAXIS_BOOK_V2_NAMED_FOOTNOTE_PROBE")
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
                            fs::write(directory.join(format!("{name}.json")), serde_json::to_vec(&json!({"source":data,"text":text,"page_names":names,"placements":placements,"consumed":consumed,"marker_count":marker_count,"sha256":typaxis_core::sha256(pdf.bytes()),"work":observed.work_steps()})).unwrap()).unwrap();
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
            "{mode}: one below work"
        );
    }
}

fn check_source_record_budget(
    flow: &typaxis_pagination::book_v2::BookV2PreparedBodyFlow<'_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    use typaxis_pagination::book_v2::prepare_book_v2_body_flow_counted;
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    let maximum = limits.base().get().max_fragments;
    let initial = maximum / 2;
    let construct = |prior, observed: &mut u64| {
        prepare_book_v2_body_flow_counted(
            flow.lines(),
            flow.blocks(),
            flow.footnotes(),
            limits,
            prior,
            observed,
        )
    };
    let mut records = 0;
    let accepted = construct(initial, &mut records).unwrap();
    assert_eq!(records, accepted.record_charge());
    let extra = records - initial;
    let exact = construct(maximum - extra, &mut records).unwrap();
    assert_eq!(records, maximum);
    assert_eq!(exact.record_charge(), maximum);
    for (index, _) in flow.footnotes().definitions().iter().enumerate() {
        for item in 0..flow.definition_items(index).unwrap().len() {
            assert_eq!(
                exact.definition_page_name_index(index, item),
                accepted.definition_page_name_index(index, item)
            );
        }
    }
    let prior = maximum - extra + 1;
    assert_eq!(
        construct(prior, &mut records).err().unwrap().kind,
        E::FragmentLimit
    );
    assert!(records >= prior && records <= maximum);
    assert_eq!(
        construct(prior, &mut records).err().unwrap().kind,
        E::FragmentLimit
    );
    assert!(records >= prior && records <= maximum);
}

#[test]
fn book_v2_named_footnotes_follow_source_cursors_and_physical_masters() {
    check(None);
}

#[test]
fn book_v2_named_footnotes_reject_conflicts_and_keep_original_failure_budgets() {
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in ["first-conflict", "paragraph-keep", "table-keep"] {
        let mut data = note_data(
            "Result",
            if mode == "table-keep" {
                "flat"
            } else {
                "serial"
            },
        );
        let owner;
        let expected;
        if mode == "first-conflict" {
            let first = &mut data["document"]["footnotes"][0]["blocks"][0];
            first["classes"] = json!(["short"]);
            owner = first["node_id"].as_u64().unwrap();
            expected = E::PendingNamedPage;
        } else {
            let selector;
            if mode == "paragraph-keep" {
                let last = &mut data["document"]["footnotes"][0]["blocks"][2];
                last["classes"] = json!(["appendix", "keep"]);
                owner = last["node_id"].as_u64().unwrap();
                selector = "paragraph.keep";
            } else {
                let table = &mut data["document"]["footnotes"][0]["blocks"][0];
                table["classes"] = json!(["appendix", "keep"]);
                owner = table["node_id"].as_u64().unwrap();
                selector = "table.keep";
                let mut following = data["document"]["blocks"][0].clone();
                following["classes"] = json!(["appendix"]);
                following["children"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|child| child["kind"] != "footnote_reference");
                data["document"]["footnotes"][0]["blocks"]
                    .as_array_mut()
                    .unwrap()
                    .push(following);
                renumber(&mut data["document"], &mut 0);
            }
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":"note-keep","selector":selector,"source_order":rules.len(),"extends":null,
                "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
            expected = E::KeepAcrossForcedBreak;
        }
        let root = Root::new();
        let limits = driver_limits();
        let input = prepared(&root, data, b"Result", &limits);
        let mut budget = BookV2PdfConvergenceBudget::new(&limits, 100_000_000);
        let mut previous = budget.observation();
        for _ in 0..2 {
            let failure = with_budgeted_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                &mut budget,
                |_, _| panic!("{mode}: conflicting named definition reached PDF"),
            )
            .err()
            .unwrap();
            assert!(
                matches!(failure, CE::Stage { ref source, .. }
                if source.downcast_ref::<typaxis_pagination::ProductionBodyPaginationError>()
                    .is_some_and(|e| e.kind == expected && u64::from(e.owner.get()) == owner)),
                "{mode}: {failure:?}"
            );
            let observed = budget.observation();
            assert!(
                observed.record_charge() > previous.record_charge(),
                "{mode}: retained records"
            );
            assert!(
                observed.work_steps() > previous.work_steps(),
                "{mode}: retained work"
            );
            assert_eq!(observed.output_charge(), 0);
            previous = observed;
        }
    }
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_named_footnotes_render_original_harano() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    check(Some(&bytes));
}
