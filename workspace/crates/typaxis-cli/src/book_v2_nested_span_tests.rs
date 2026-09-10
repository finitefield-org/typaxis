use super::nested_tables::nested;
use super::table_caption_breaks::renumber;
use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

pub(super) fn spanning(text: &str, split: usize, mode: &str) -> Value {
    let mut data = nested(
        text,
        split,
        if matches!(mode, "forced" | "right-span" | "simultaneous" | "notes") {
            "forced"
        } else {
            "natural"
        },
    );
    let br = super::table_cell_breaks::cells(text, split, false, false, false)["document"]
        ["blocks"][0]["body"][0]["cells"][0]["blocks"][1]
        .clone();
    let parent = &mut data["document"]["blocks"][0];
    let both = parent["body"][0]["cells"][1]["blocks"][0].clone();
    let left =
        parent["body"][0]["cells"][0]["blocks"][0]["body"][0]["cells"][0]["blocks"][0].clone();
    let mut later = parent["body"][0].clone();
    later["cells"].as_array_mut().unwrap().truncate(1);
    later["cells"][0]["blocks"] = json!([both.clone()]);
    let span_column = usize::from(mode == "right-span");
    parent["body"][0]["cells"][span_column]["rowspan"] = if mode == "three-rows" {
        3.into()
    } else {
        2.into()
    };
    if mode == "later-break" {
        later["cells"][0]["blocks"] = json!([br.clone(), both.clone()]);
    }
    if mode == "simultaneous" {
        parent["body"][0]["cells"][1]["blocks"] = json!([both.clone(), br.clone(), both.clone()]);
    }
    if mode == "three-rows" {
        let mut empty = later.clone();
        empty["cells"][0]["blocks"] = json!([]);
        parent["body"].as_array_mut().unwrap().push(empty);
    }
    parent["body"].as_array_mut().unwrap().push(later);
    if mode == "caption-child" {
        parent["caption"] = parent["body"][0]["cells"][0]["blocks"].clone();
        parent["body"][0]["cells"][0]["blocks"] = json!([left.clone()]);
    }
    if mode == "caption-keep" {
        let mut kept = both.clone();
        kept["classes"] = json!(["keep"]);
        parent["caption"] = json!([both.clone(), kept]);
    }
    if matches!(
        mode,
        "header"
            | "header-forced"
            | "header-leading"
            | "header-trailing"
            | "header-consecutive"
            | "header-caption"
            | "empty-body"
    ) {
        let child = &mut parent["body"][0]["cells"][0]["blocks"][0];
        child["body"][0]["cells"][0]["blocks"] = json!([left.clone()]);
        child["body"][0]["cells"][1]["blocks"] = json!([left.clone()]);
        if mode == "header-caption" {
            child["caption"] = json!([both.clone()]);
        }
        parent["head"] = parent["body"].clone();
        let cell = &mut parent["head"][1]["cells"][0];
        cell["blocks"] = match mode {
            "header-forced" => json!([left.clone(), br.clone(), left.clone()]),
            "header-leading" => json!([br.clone(), left.clone()]),
            "header-trailing" => json!([left.clone(), br.clone()]),
            "header-consecutive" => json!([br.clone(), br.clone(), left.clone()]),
            _ => json!([left.clone()]),
        };
        let mut row = parent["body"][0].clone();
        row["cells"][0]["rowspan"] = 1.into();
        row["cells"][0]["blocks"] = json!([left.clone()]);
        row["cells"][1]["blocks"] = json!([both.clone()]);
        parent["body"] = if mode == "empty-body" {
            json!([])
        } else {
            json!(vec![row; 5])
        };
        data["page_masters"]["masters"][0]["body"]["height"] = (80 * 65536).into();
    }
    if mode == "notes" {
        let para = &mut data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]["body"]
            [0]["cells"][0]["blocks"][0];
        para["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":left["span"],"footnote_id":"note"}));
        data["document"]["footnotes"] =
            json!([{"node_id":0,"span":left["span"],"footnote_id":"note","blocks":vec![left;8]}]);
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":80*65536,"width":180*65536,"height":48*65536});
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"nested-span-keep","selector":"paragraph.keep","source_order":rules.len(),"extends":null,"declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_nested_spans_preserve_bands_and_forced_boundaries() {
    for (mode, expected) in [
        ("natural", 2),
        ("forced", 3),
        ("right-span", 3),
        ("later-break", 3),
        ("simultaneous", 3),
        ("three-rows", 2),
        ("caption-keep", 3),
        ("header", 2),
        ("header-forced", 3),
        ("header-leading", 3),
        ("header-trailing", 3),
        ("header-consecutive", 4),
        ("header-caption", 3),
        ("empty-body", 1),
        ("notes", 4),
    ] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, spanning("LeftRight", 4, mode), b"LeftRight", &limits);
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                let pages = pdf.navigation().source().source().source().pages().len();
                assert_eq!(pages, expected, "{mode}");
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap_or_else(|e| panic!("{mode}: {e:?}"));
    }
}

#[test]
fn book_v2_nested_spans_charge_retries_and_visit_source_once() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements, prepare_book_v2_table_search,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in [
        "natural",
        "later-break",
        "header",
        "header-forced",
        "caption-keep",
        "caption-child",
    ] {
        let root = Root::new();
        let limits = limits();
        let data = spanning("LeftRight", 4, mode);
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
                            if mode == "natural" || mode == "header" {
                                2
                            } else {
                                3
                            },
                            "{mode}"
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
fn book_v2_nested_spans_render_original_harano() {
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
    let mut data = spanning(text, 6, "later-break");
    // The original font's natural child-row boundary needs more than 48 pt.
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
