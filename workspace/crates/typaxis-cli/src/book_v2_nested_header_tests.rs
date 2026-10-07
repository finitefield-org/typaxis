use super::nested_tables::nested;
use super::table_caption_breaks::renumber;
use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

fn headed(text: &str, split: usize, mode: &str) -> Value {
    let mut data = nested(
        text,
        split,
        if mode == "forced-child-header" {
            "forced"
        } else {
            "natural"
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
    let mut row = parent["body"][0].clone();
    row["cells"][0]["blocks"] = json!([left.clone()]);
    row["cells"][1]["blocks"] = json!([right.clone()]);
    let mut header = row.clone();
    if matches!(mode, "body-child" | "caption-header" | "forced-header") {
        if mode == "forced-header" {
            header["cells"][0]["blocks"] = json!([left.clone(), br, left.clone()]);
        }
    } else {
        if mode != "header-child" && mode != "forced-child-header" {
            child["caption"] = json!([both.clone()]);
            child["body"][0]["cells"][0]["blocks"] = json!([left.clone()]);
            child["body"][0]["cells"][1]["blocks"] = json!([right.clone()]);
        }
        if mode == "image-caption" {
            child["caption"] = json!([{"kind":"figure","node_id":0,"span":left["span"],"classes":[],"image_id":0,"placement":"block","alt":"Original diagram","caption":[]}]);
        }
        if mode == "anchor-caption" || mode == "empty-caption" {
            let anchor = json!({"kind":"anchor","node_id":0,"span":left["span"],"anchor_id":"nested.header.caption"});
            if mode == "empty-caption" {
                child["caption"][0]["children"] = json!([anchor]);
            } else {
                child["caption"][0]["children"]
                    .as_array_mut()
                    .unwrap()
                    .insert(0, anchor);
            }
        }
        if mode == "header-deep" {
            let grandchild = child.clone();
            child["caption"] = json!([grandchild]);
        }
        if mode == "notes" {
            child["caption"][0]["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":left["span"],"footnote_id":"note"}));
        }
        header["cells"][0]["blocks"] = json!([child]);
        parent["body"] = json!(vec![row; if mode == "forced-child-header" { 5 } else { 3 }]);
    }
    parent["head"] = json!([header]);
    if mode == "caption-header" {
        parent["caption"] = json!([both]);
    }
    if mode == "caption-backtrack" {
        let mut keep = left.clone();
        keep["classes"] = json!(["keep"]);
        parent["caption"] = json!([left.clone(), keep]);
    }
    if mode == "empty-body" {
        parent["body"] = json!([]);
    }
    if mode == "notes" {
        data["document"]["footnotes"] =
            json!([{"node_id":0,"span":left["span"],"footnote_id":"note","blocks":vec![left;8]}]);
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":80*65536,"width":180*65536,"height":48*65536});
    }
    data["page_masters"]["masters"][0]["body"]["height"] =
        (if mode == "header-child" || mode == "forced-child-header" {
            96
        } else if mode == "header-deep" {
            80
        } else {
            64
        } * 65536)
            .into();
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"nested-header-keep","selector":"paragraph.keep","source_order":rules.len(),"extends":null,"declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    if mode == "image-caption" {
        rules.push(json!({"style_id":"header-image","selector":"figure","source_order":rules.len(),"extends":null,"declarations":[{"name":"width","important":false,"value":{"kind":"length","value":16*65536}}]}));
    }
    renumber(&mut data["document"], &mut 0);
    data
}

#[test]
fn book_v2_nested_headers_preserve_original_and_repeated_regions() {
    for (mode, pages) in [
        ("body-child", 2),
        ("header-child", 2),
        ("header-caption", 2),
        ("image-caption", 2),
        ("anchor-caption", 2),
        ("empty-caption", 2),
        ("header-deep", 2),
        ("forced-header", 3),
        ("forced-child-header", 4),
        ("caption-header", 2),
        ("caption-backtrack", 3),
        ("notes", 0),
        ("empty-body", 1),
    ] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, headed("LeftRight", 4, mode), b"LeftRight", &limits);
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
fn book_v2_nested_headers_charge_prefix_rollback_and_visit_source_once() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements, prepare_book_v2_table_search,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in [
        "header-caption",
        "forced-child-header",
        "caption-backtrack",
        "forced-header",
    ] {
        let root = Root::new();
        let limits = limits();
        let data = headed("LeftRight", 4, mode);
        let frame_height = data["page_masters"]["masters"][0]["body"]["height"]
            .as_i64()
            .unwrap();
        let input = prepared(&root, data, b"LeftRight", &limits);
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
            PositiveLength::new(raw(frame_height)).unwrap(),
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
                            if mode == "forced-child-header" {
                                4
                            } else if mode == "caption-backtrack" || mode == "forced-header" {
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
fn book_v2_nested_headers_render_original_harano() {
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
    let mut data = headed(text, 6, "header-caption");
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
