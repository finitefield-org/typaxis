use super::table_caption_breaks::renumber;
use super::table_cell_breaks::cells;
use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

fn spanning(text: &str, split: usize, mode: &str) -> Value {
    let mut data = cells(text, split, false, mode == "caption", false);
    let table = &mut data["document"]["blocks"][0];
    let first = table["body"][0].clone();
    let left = first["cells"][0]["blocks"][0].clone();
    let mut right = first["cells"][1]["blocks"][0].clone();
    let br = first["cells"][0]["blocks"][1].clone();
    let mut row = first.clone();
    row["cells"].as_array_mut().unwrap().truncate(1);
    row["cells"][0]["blocks"] = json!([left.clone()]);
    if mode == "later-break" || mode == "three-rows" || mode == "simultaneous" {
        table["body"][0]["cells"][0]["rowspan"] = (if mode == "three-rows" { 3 } else { 2 }).into();
        right["classes"] = json!([]);
        table["body"][0]["cells"][1]["blocks"] = json!([right.clone()]);
        row["cells"][0]["blocks"] = json!([right.clone()]);
        if mode == "later-break" {
            let mut tall = left.clone();
            tall["classes"] = json!(["tall"]);
            table["body"][0]["cells"][0]["blocks"] = json!([tall.clone(), tall]);
            row["cells"][0]["blocks"] = json!([br, right]);
        } else if mode == "simultaneous" {
            table["body"][0]["cells"][1]["blocks"] = json!([right.clone(), br, right]);
        } else {
            let mut empty = row.clone();
            empty["cells"][0]["blocks"] = json!([]);
            table["body"].as_array_mut().unwrap().push(empty);
        }
    } else {
        table["body"][0]["cells"][1]["rowspan"] = 2.into();
    }
    table["body"].as_array_mut().unwrap().push(row);
    if mode == "header" {
        let mut head = first;
        for cell in head["cells"].as_array_mut().unwrap() {
            cell["blocks"].as_array_mut().unwrap().truncate(1);
        }
        table["head"] = json!([head]);
        data["page_masters"]["masters"][0]["body"]["height"] = (96 * 65536).into();
    }
    if mode == "blank" {
        let table = &mut data["document"]["blocks"][0];
        let br = table["body"][0]["cells"][0]["blocks"][1].clone();
        table["body"][0]["cells"][0]["rowspan"] = 2.into();
        table["body"][0]["cells"][0]["blocks"] = json!([br.clone(), br]);
        table["body"][0]["cells"][1]["rowspan"] = 1.into();
        table["body"][0]["cells"][1]["blocks"] = json!([]);
        table["body"][1]["cells"][0]["blocks"] = json!([]);
    }
    if mode == "caption-band-keep" {
        let table = &mut data["document"]["blocks"][0];
        let mut tall = table["body"][0]["cells"][0]["blocks"][0].clone();
        tall["classes"] = json!(["tall"]);
        let br = table["body"][0]["cells"][0]["blocks"][1].clone();
        table["body"][0]["cells"][0]["rowspan"] = 2.into();
        table["body"][0]["cells"][0]["blocks"] = json!([tall.clone(), br, tall]);
        table["body"][0]["cells"][1]["rowspan"] = 1.into();
        table["body"][0]["cells"][1]["blocks"] = json!([]);
        table["body"][1]["cells"][0]["blocks"] = json!([]);
        let para = super::table_caption_breaks::caption_breaks_with_text(false, false, text)
            ["document"]["blocks"][0]["caption"][1]
            .clone();
        let mut kept = para.clone();
        kept["classes"] = json!(["keep"]);
        table["caption"] = json!([para, kept]);
        let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
        rules.push(json!({"style_id":"keep","selector":"paragraph.keep","source_order":rules.len(),"extends":null,
            "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    }
    if mode == "notes" || mode == "span-notes" {
        let column = if mode == "span-notes" { 1 } else { 0 };
        let para = data["document"]["blocks"][0]["body"][0]["cells"][column]["blocks"][0].clone();
        data["document"]["blocks"][0]["body"][0]["cells"][column]["blocks"][0]["children"].as_array_mut().unwrap()
            .push(json!({"kind":"footnote_reference","node_id":0,"span":para["span"],"footnote_id":"note"}));
        data["document"]["footnotes"] =
            json!([{"node_id":0,"span":para["span"],"footnote_id":"note","blocks":vec![para;8]}]);
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":80*65536,"width":180*65536,"height":48*65536});
    }
    renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_table_rowspan_breaks_preserve_bands_and_retry_later_boundaries() {
    for (mode, pages) in [
        ("right-span", 3),
        ("later-break", 3),
        ("three-rows", 2),
        ("simultaneous", 2),
        ("header", 2),
        ("caption", 7),
        ("notes", 0),
        ("span-notes", 0),
        ("blank", 3),
        ("caption-band-keep", 3),
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
                let actual = pdf.navigation().source().source().source().pages().len();
                if pages == 0 {
                    assert!(actual >= 3);
                } else {
                    assert_eq!(actual, pages, "{mode}");
                }
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap_or_else(|error| panic!("{mode}: {error:?}"));
    }
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_table_rowspan_breaks_render_original_harano() {
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
    let mut data = spanning(text, 6, "right-span");
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
            assert_eq!(pdf.navigation().source().source().source().pages().len(), 3);
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_table_rowspan_breaks_charge_retry_and_cover_original_cells_once() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements, prepare_book_v2_table_search,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in ["right-span", "later-break", "blank"] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, spanning("LeftRight", 4, mode), b"LeftRight", &limits);
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
                        assert_eq!(hashes.len(), 3);
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
