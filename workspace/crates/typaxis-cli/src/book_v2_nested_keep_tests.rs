use super::nested_tables::nested;
use super::table_caption_breaks::renumber;
use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

fn kept(text: &str, split: usize, mode: &str) -> Value {
    let mut data = nested(text, split, "natural");
    let parent = &mut data["document"]["blocks"][0];
    let mut child = parent["body"][0]["cells"][0]["blocks"][0].clone();
    let left = child["body"][0]["cells"][0]["blocks"][0].clone();
    let mut right = child["body"][0]["cells"][1]["blocks"][0].clone();
    right["classes"] = json!([]);
    let both = parent["body"][0]["cells"][1]["blocks"][0].clone();
    let br = super::table_cell_breaks::cells(text, split, false, false, false)["document"]
        ["blocks"][0]["body"][0]["cells"][0]["blocks"][1]
        .clone();
    let mut keep = left.clone();
    keep["classes"] = json!(["keep"]);
    if matches!(
        mode,
        "child-kept" | "child-shorten" | "chain" | "child-conflict"
    ) {
        child["classes"] = json!(["keep"]);
    }
    if matches!(mode, "child-shorten" | "chain" | "child-conflict") {
        child["body"][0]["cells"][0]["blocks"] = json!(vec![left.clone(); 3]);
        child["body"][0]["cells"][1]["blocks"] = json!([right.clone()]);
    }
    parent["body"][0]["cells"][0]["blocks"] = match mode {
        "preceding" | "forced" | "notes" => json!([keep.clone(), child]),
        "prefix-backtrack" => json!([left.clone(), keep.clone(), child]),
        "suffix" => json!([child, keep.clone(), right.clone()]),
        "child-kept" => json!([child, left.clone()]),
        "child-shorten" => {
            let mut tall = right.clone();
            tall["classes"] = json!(["tall"]);
            json!([child, tall])
        }
        "chain" => json!([child, keep.clone(), right.clone()]),
        "flow-conflict" => json!([keep, br.clone(), child]),
        "child-conflict" => json!([child, br.clone(), left.clone()]),
        "oversize" => {
            let mut tall = right.clone();
            tall["classes"] = json!(["huge"]);
            json!([keep, tall, child])
        }
        _ => json!([child]),
    };
    if mode == "last" {
        parent["body"][0]["cells"][1]["blocks"][0]["classes"] = json!(["keep"]);
    }
    if mode == "forced" || mode == "notes" {
        parent["body"][0]["cells"][1]["blocks"] = json!([both.clone(), br]);
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    for selector in ["paragraph.keep", "table.keep"] {
        rules.push(json!({"style_id":format!("keep-{}",rules.len()),"selector":selector,"source_order":rules.len(),"extends":null,
            "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    }
    rules.push(json!({"style_id":"huge","selector":"paragraph.huge","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"line_height","important":false,"value":{"kind":"length","value":48*65536}}]}));
    if mode == "notes" {
        let para = &mut data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0];
        para["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":left["span"],"footnote_id":"note"}));
        data["document"]["footnotes"] =
            json!([{"node_id":0,"span":left["span"],"footnote_id":"note","blocks":vec![left;8]}]);
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":80*65536,"width":180*65536,"height":48*65536});
    }
    if mode == "whole" || mode == "whole-conflict" {
        let mut table = data["document"]["blocks"][0].clone();
        let child = &mut table["body"][0]["cells"][0]["blocks"][0];
        for cell in child["body"][0]["cells"].as_array_mut().unwrap() {
            cell["blocks"].as_array_mut().unwrap().truncate(1);
            cell["blocks"][0]["classes"] = json!([]);
        }
        if mode == "whole-conflict" {
            child["body"][0]["cells"][0]["blocks"]
                .as_array_mut()
                .unwrap()
                .push(
                    super::table_cell_breaks::cells(text, split, false, false, false)["document"]
                        ["blocks"][0]["body"][0]["cells"][0]["blocks"][1]
                        .clone(),
                );
        }
        let mut before = both;
        before["classes"] = json!(["tall"]);
        data["document"]["blocks"] = json!([before,{"kind":"figure","node_id":0,"span":table["span"],"classes":[],"image_id":0,"placement":"block","alt":"Original diagram","caption":[table]}]);
        let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
        rules.push(json!({"style_id":"whole","selector":"figure","source_order":rules.len(),"extends":null,
            "declarations":[{"name":"keep_caption","important":false,"value":{"kind":"boolean","value":true}},
                {"name":"width","important":false,"value":{"kind":"length","value":16*65536}}]}));
    }
    renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_nested_keeps_preserve_legal_parent_prefixes_and_child_endings() {
    for (mode, pages) in [
        ("last", 2),
        ("preceding", 2),
        ("prefix-backtrack", 3),
        ("suffix", 3),
        ("child-kept", 2),
        ("child-shorten", 2),
        ("chain", 2),
        ("forced", 3),
        ("notes", 0),
        ("whole", 2),
    ] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, kept("LeftRight", 4, mode), b"LeftRight", &limits);
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                let actual = pdf.navigation().source().source().source().pages().len();
                if pages == 0 {
                    assert!(actual >= 3);
                } else {
                    assert_eq!(actual, pages, "{mode}");
                }
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap_or_else(|e| panic!("{mode}: {e:?}"));
    }
}
#[test]
fn book_v2_nested_keeps_reject_source_break_conflicts_and_oversize_groups() {
    for mode in [
        "flow-conflict",
        "child-conflict",
        "whole-conflict",
        "oversize",
    ] {
        let root = Root::new();
        let limits = limits();
        let data = kept("LeftRight", 4, mode);
        let owner = if mode == "whole-conflict" {
            data["document"]["blocks"][1]["caption"][0]["node_id"]
                .as_u64()
                .unwrap()
        } else if mode == "oversize" {
            data["document"]["blocks"][0]["node_id"].as_u64().unwrap()
        } else {
            data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]["node_id"]
                .as_u64()
                .unwrap()
        };
        let input = prepared(&root, data, b"LeftRight", &limits);
        let error = crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |_, _| panic!("invalid keep reached PDF"),
        )
        .unwrap_err();
        let message = format!("{error:?}");
        assert!(
            message.contains(if mode == "oversize" {
                "Oversize"
            } else {
                "KeepAcrossForcedBreak"
            }),
            "{mode}: {message}"
        );
        assert!(
            message.contains(&format!("NodeId({owner})")),
            "{mode}: {message}"
        );
    }
}

#[test]
fn book_v2_nested_keeps_charge_discarded_trials_and_cover_source_once() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements, prepare_book_v2_table_search,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in ["prefix-backtrack", "child-shorten", "chain", "forced"] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, kept("LeftRight", 4, mode), b"LeftRight", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings =
            typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits)
                .unwrap();
        let raw = |v| Length::from_raw(v).unwrap();
        let rect = Rect::new(
            raw(10 * 65536),
            raw(10 * 65536),
            PositiveLength::new(raw(180 * 65536)).unwrap(),
            PositiveLength::new(raw(48 * 65536)).unwrap(),
        );
        typaxis_layout::book_v2::with_converged_book_v2_body_lines(
            &policy,
            &flow,
            input.resources(),
            &bindings,
            &limits,
            JapaneseLineBreakMode::Normal,
            rect,
            1_000_000,
            |stable| {
                let measured = prepare_book_v2_table_measurements(
                    prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                        .unwrap(),
                    &limits,
                )
                .unwrap();
                let replay =
                    |prior, work| -> Result<_, typaxis_pagination::ProductionBodyPaginationError> {
                        let mut search =
                            prepare_book_v2_table_search(&measured, 0, &limits, work, prior)?;
                        let mut cursor = search.begin()?;
                        let mut hashes = Vec::new();
                        let mut leaves = Vec::new();
                        while !cursor.is_terminal() {
                            assert!(hashes.len() < 10);
                            let selected =
                                search.evaluate(&cursor, search.maximum_height())?.unwrap();
                            leaves.extend(selected.semantic_leaf_ranges().flatten());
                            hashes.push(selected.fingerprint());
                            cursor = selected.after();
                        }
                        leaves.sort_unstable();
                        assert_eq!(
                            leaves,
                            (0..measured.flow().body_items().len()).collect::<Vec<_>>()
                        );
                        assert_eq!(
                            hashes.len(),
                            if mode == "forced" || mode == "prefix-backtrack" {
                                3
                            } else {
                                2
                            }
                        );
                        Ok((search.record_charge(), search.work_charge(), hashes))
                    };
                let (records, work, hashes) = replay(0, 1_000_000).unwrap();
                let prior =
                    limits.base().get().max_fragments - (records - measured.record_charge());
                assert_eq!(
                    replay(prior, work).unwrap(),
                    (limits.base().get().max_fragments, work, hashes)
                );
                assert_eq!(replay(prior + 1, work).unwrap_err().kind, E::FragmentLimit);
                assert_eq!(replay(0, work - 1).unwrap_err().kind, E::TableSearchLimit);
                let mut search =
                    prepare_book_v2_table_body_search(&measured, &limits, 1_000_000, 0).unwrap();
                let pages = search.select_stable_mixed_pages(2).unwrap();
                let placed = search.place_mixed_pages(pages.sequence()).unwrap();
                search.close_mixed_page_sources(&pages, &placed).unwrap();
            },
        )
        .unwrap();
    }
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_nested_keeps_render_original_harano() {
    let root = Root::new();
    let limits = limits();
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hash = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let text = "左側右側";
    let mut data = kept(text, 6, "child-shorten");
    // Original Harano metrics exceed the synthetic 16pt line box. Allow the
    // child to fit as a whole while its kept 32pt successor still requires a retry.
    data["page_masters"]["masters"][0]["body"]["height"] = (80 * 65536).into();
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
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            assert_eq!(pdf.navigation().source().source().source().pages().len(), 2);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}
