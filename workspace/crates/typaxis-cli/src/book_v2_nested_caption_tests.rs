use super::nested_tables::nested;
use super::table_caption_breaks::renumber;
use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

fn captioned(text: &str, split: usize, mode: &str) -> Value {
    let mut data = nested(
        text,
        split,
        match mode {
            "forced-child" | "caption-break" => "forced",
            "child-header" => "headers",
            "child-caption" => "caption",
            "spacing" => "spacing",
            _ => "natural",
        },
    );
    let parent = &mut data["document"]["blocks"][0];
    let mut child = parent["body"][0]["cells"][0]["blocks"][0].clone();
    let left = child["body"][0]["cells"][0]["blocks"][0].clone();
    let mut right = child["body"][0]["cells"][1]["blocks"][0].clone();
    right["classes"] = json!([]);
    let both = parent["body"][0]["cells"][1]["blocks"][0].clone();
    let br = super::table_cell_breaks::cells(text, split, false, false, false)["document"]
        ["blocks"][0]["body"][0]["cells"][0]["blocks"][1]
        .clone();
    parent["body"][0]["cells"][0]["blocks"] = json!([left.clone()]);
    parent["body"][0]["cells"][1]["blocks"] = json!([right.clone()]);
    parent["caption"] = match mode {
        "ordinary" => {
            parent["body"][0]["cells"][0]["blocks"] = json!([child]);
            json!([both])
        }
        "caption-break" => json!([br.clone(), child, br.clone(), both]),
        "multiple" => json!([child.clone(), child]),
        "keep" | "notes" => {
            let mut kept = left.clone();
            kept["classes"] = json!(["keep"]);
            parent["body"][0]["cells"][0]["blocks"][0]["classes"] = json!(["tall"]);
            json!([child, kept])
        }
        "child-keep" => {
            child["classes"] = json!(["keep"]);
            child["body"][0]["cells"][0]["blocks"] = json!(vec![left.clone(); 3]);
            child["body"][0]["cells"][1]["blocks"] = json!([right.clone()]);
            parent["body"][0]["cells"][0]["blocks"][0]["classes"] = json!(["tall"]);
            json!([child])
        }
        "caption-only-break" => json!([child, br.clone()]),
        "empty-kept-body" => {
            child["classes"] = json!(["keep"]);
            json!([child])
        }
        "conflict" => {
            child["classes"] = json!(["keep"]);
            json!([child, br.clone()])
        }
        _ => json!([child]),
    };
    if matches!(
        mode,
        "caption-only" | "caption-only-break" | "empty-kept-body"
    ) {
        for cell in parent["body"][0]["cells"].as_array_mut().unwrap() {
            cell["blocks"] = json!([]);
        }
    }
    if mode == "body-forced" {
        parent["body"][0]["cells"][0]["blocks"] = json!([br, left.clone()]);
    }
    if mode == "notes" {
        parent["caption"][1]["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":left["span"],"footnote_id":"note"}));
        data["document"]["footnotes"] = json!([{"node_id":0,"span":left["span"],"footnote_id":"note","blocks":vec![left.clone();8]}]);
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":80*65536,"width":180*65536,"height":48*65536});
    }
    if mode == "deep" {
        let mut outer = data["document"]["blocks"][0].clone();
        outer["caption"] = data["document"]["blocks"].clone();
        data["document"]["blocks"] = json!([outer]);
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    for selector in ["paragraph.keep", "table.keep"] {
        rules.push(json!({"style_id":format!("cap-keep-{}",rules.len()),"selector":selector,"source_order":rules.len(),"extends":null,
            "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    }
    renumber(&mut data["document"], &mut 0);
    data
}

#[test]
fn book_v2_nested_captions_continue_before_original_body_rows() {
    for (mode, pages) in [
        ("ordinary", 2),
        ("caption-child", 2),
        ("forced-child", 3),
        ("caption-break", 5),
        ("multiple", 4),
        ("keep", 3),
        ("child-keep", 2),
        ("caption-only", 2),
        ("caption-only-break", 3),
        ("empty-kept-body", 2),
        ("body-forced", 3),
        ("child-header", 3),
        ("child-caption", 7),
        ("spacing", 2),
        ("deep", 3),
        ("notes", 0),
    ] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(
            &root,
            captioned("LeftRight", 4, mode),
            b"LeftRight",
            &limits,
        );
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
fn book_v2_nested_captions_report_kept_child_break_owner() {
    let root = Root::new();
    let limits = limits();
    let data = captioned("LeftRight", 4, "conflict");
    let owner = data["document"]["blocks"][0]["caption"][0]["node_id"]
        .as_u64()
        .unwrap();
    let input = prepared(&root, data, b"LeftRight", &limits);
    let e = crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |_, _| panic!("conflict reached PDF"),
    )
    .unwrap_err();
    let message = format!("{e:?}");
    assert!(
        message.contains("KeepAcrossForcedBreak") && message.contains(&format!("NodeId({owner})")),
        "{message}"
    );
}

#[test]
fn book_v2_nested_captions_charge_serial_cursor_retries_and_cover_source_once() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements, prepare_book_v2_table_search,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in ["caption-break", "keep", "child-keep", "body-forced"] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(
            &root,
            captioned("LeftRight", 4, mode),
            b"LeftRight",
            &limits,
        );
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
                            if mode == "caption-break" {
                                5
                            } else if mode == "keep" || mode == "body-forced" {
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
fn book_v2_nested_captions_render_original_harano() {
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
    let mut data = captioned(text, 6, "forced-child");
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
            assert_eq!(pdf.navigation().source().source().source().pages().len(), 4);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}
