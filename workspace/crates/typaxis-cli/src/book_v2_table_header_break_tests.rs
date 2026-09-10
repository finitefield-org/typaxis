use super::table_caption_breaks::renumber;
use super::table_cell_breaks::cells;
use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

pub(super) fn headers(text: &str, split: usize, mode: &str) -> Value {
    let asymmetric = matches!(
        mode,
        "asymmetric" | "caption" | "oversize" | "notes" | "head-span" | "body-span" | "body-break"
    );
    let mut data = cells(text, split, !asymmetric, mode == "caption", false);
    let table = &mut data["document"]["blocks"][0];
    let mut head = table["body"][0].clone();
    let left = head["cells"][0]["blocks"][0].clone();
    let right = head["cells"][1]["blocks"][0].clone();
    let br = head["cells"][0]["blocks"][1].clone();
    let body = &mut table["body"][0];
    body["cells"][0]["blocks"] = json!([left.clone()]);
    body["cells"][1]["blocks"] = json!([right.clone()]);
    let row = body.clone();
    table["body"] = json!(vec![row; if asymmetric { 4 } else { 3 }]);
    if mode == "leading" || mode == "consecutive" {
        head["cells"][0]["blocks"] = if mode == "leading" {
            json!([br.clone(), left])
        } else {
            json!([br.clone(), br, left])
        };
        head["cells"][1]["blocks"] = json!([right]);
    } else if mode == "trailing" {
        head["cells"][0]["blocks"] = json!([left, br]);
        head["cells"][1]["blocks"] = json!([right]);
    }
    table["head"] = json!([head]);
    if mode == "head-span" {
        let mut row = table["body"][0].clone();
        row["cells"].as_array_mut().unwrap().truncate(1);
        table["head"][0]["cells"][1]["rowspan"] = 2.into();
        table["head"].as_array_mut().unwrap().push(row);
    } else if mode == "body-span" {
        table["body"][0]["cells"][1]["rowspan"] = 2.into();
        table["body"][1]["cells"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
    } else if mode == "body-break" {
        let left = table["body"][1]["cells"][0]["blocks"][0].clone();
        let br = table["head"][0]["cells"][0]["blocks"][1].clone();
        table["body"][1]["cells"][0]["blocks"] = json!([left.clone(), br, left]);
    } else if mode == "empty-body" {
        table["body"] = json!([]);
    } else if mode == "blank" {
        let br = table["head"][0]["cells"][0]["blocks"][1].clone();
        table["head"][0]["cells"][0]["blocks"] = json!([br.clone(), br]);
        table["head"][0]["cells"][1]["blocks"] = json!([]);
        for row in table["body"].as_array_mut().unwrap() {
            for cell in row["cells"].as_array_mut().unwrap() {
                cell["blocks"] = json!([]);
            }
        }
    }
    let height = if mode == "oversize" {
        48
    } else if asymmetric {
        96
    } else if mode == "simultaneous" {
        64
    } else {
        48
    };
    data["page_masters"]["masters"][0]["body"]["height"] = (height * 65536).into();
    if mode == "notes" {
        let para = data["document"]["blocks"][0]["head"][0]["cells"][0]["blocks"][0].clone();
        data["document"]["blocks"][0]["head"][0]["cells"][0]["blocks"][0]["children"].as_array_mut().unwrap()
            .push(json!({"kind":"footnote_reference","node_id":0,"span":para["span"],"footnote_id":"note"}));
        data["document"]["footnotes"] =
            json!([{"node_id":0,"span":para["span"],"footnote_id":"note","blocks":vec![para;8]}]);
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":110*65536,"width":180*65536,"height":32*65536});
    }
    renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_table_header_breaks_consume_original_head_before_repeating() {
    for (mode, expected) in [
        ("asymmetric", 5),
        ("leading", 3),
        ("consecutive", 4),
        ("trailing", 3),
        ("simultaneous", 2),
        ("caption", 9),
        ("notes", 0),
        ("head-span", 5),
        ("body-span", 4),
        ("body-break", 6),
        ("empty-body", 2),
        ("blank", 3),
    ] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, headers("LeftRight", 4, mode), b"LeftRight", &limits);
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                let pages = pdf.navigation().source().source().source().pages().len();
                if expected == 0 {
                    assert!(pages >= 5);
                } else {
                    assert_eq!(pages, expected, "{mode}");
                }
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap_or_else(|e| panic!("{mode}: {e:?}"));
    }
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_header_breaks_render_original_harano() {
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
    let mut data = headers(text, 6, "asymmetric");
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
            assert_eq!(pdf.navigation().source().source().source().pages().len(), 5);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_table_header_breaks_reject_oversize_repetition_and_keep_conflicts() {
    for mode in ["oversize", "keep"] {
        let root = Root::new();
        let limits = limits();
        let mut data = headers(
            "LeftRight",
            4,
            if mode == "keep" { "asymmetric" } else { mode },
        );
        if mode == "keep" {
            data["document"]["blocks"][0]["head"][0]["cells"][0]["blocks"][0]["classes"] =
                json!(["keep"]);
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":"keep","selector":"paragraph.keep","source_order":rules.len(),"extends":null,
                "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
        }
        let owner = if mode == "keep" {
            &data["document"]["blocks"][0]["head"][0]["cells"][0]["blocks"][0]["node_id"]
        } else {
            &data["document"]["blocks"][0]["node_id"]
        }
        .as_u64()
        .unwrap();
        let input = prepared(&root, data, b"LeftRight", &limits);
        let error = crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |_, _| panic!("invalid header reached PDF"),
        )
        .unwrap_err();
        let message = format!("{error:?}");
        assert!(
            message.contains(if mode == "keep" {
                "KeepAcrossForcedBreak"
            } else {
                "TableHeaderOversize"
            }),
            "{message}"
        );
        assert!(message.contains(&format!("NodeId({owner})")), "{message}");
    }
}

#[test]
fn book_v2_table_header_breaks_charge_retry_and_cover_original_cells_once() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements, prepare_book_v2_table_search,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in ["asymmetric", "head-span", "body-span", "blank"] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, headers("LeftRight", 4, mode), b"LeftRight", &limits);
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
            PositiveLength::new(raw(if mode == "blank" {
                48 * 65536
            } else {
                96 * 65536
            }))
            .unwrap(),
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
                            if mode == "blank" {
                                3
                            } else if mode == "body-span" {
                                4
                            } else {
                                5
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
