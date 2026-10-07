use super::table_caption_breaks::renumber;
use super::table_cell_breaks::cells;
use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

pub(super) fn nested(text: &str, split: usize, mode: &str) -> Value {
    let mut data = cells(text, split, false, mode == "caption", false);
    let mut child = data["document"]["blocks"][0].clone();
    if mode == "child-header-break" {
        data = super::table_header_breaks::headers(text, split, "asymmetric");
        child = data["document"]["blocks"][0].clone();
    }
    let left = child["body"][0]["cells"][0]["blocks"][0].clone();
    let mut right = child["body"][0]["cells"][1]["blocks"][0].clone();
    right["classes"] = json!([]);
    let br = child[if mode == "child-header-break" {
        "head"
    } else {
        "body"
    }][0]["cells"][0]["blocks"][1]
        .clone();
    let mut both = left.clone();
    both["children"][0]["text_span"]["end_byte"] = text.len().into();
    if mode == "natural" || mode == "deep" || mode == "spacing" {
        child["body"][0]["cells"][0]["blocks"] =
            json!(vec![left.clone(); if mode == "spacing" { 3 } else { 4 }]);
        if mode == "spacing" {
            child["body"][0]["cells"][1]["blocks"]
                .as_array_mut()
                .unwrap()
                .truncate(1);
            child["classes"] = json!(["child"]);
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":"child","selector":"table.child","source_order":rules.len(),"extends":null,
                "declarations":[{"name":"space_after","important":false,"value":{"kind":"length","value":8*65536}}]}));
        }
    }
    if mode == "headers" {
        let mut row = child["body"][0].clone();
        row["cells"][0]["blocks"] = json!([left.clone()]);
        row["cells"][1]["blocks"] = json!([right.clone()]);
        child["head"] = json!([row.clone()]);
        child["body"] = json!([row.clone(), row.clone(), row.clone(), row]);
    }
    if mode == "child-span" {
        let mut row = child["body"][0].clone();
        row["cells"].as_array_mut().unwrap().truncate(1);
        row["cells"][0]["blocks"] = json!([left.clone()]);
        child["body"][0]["cells"][1]["rowspan"] = 2.into();
        child["body"].as_array_mut().unwrap().push(row);
    }
    let mut parent = child.clone();
    parent["classes"] = json!([]);
    parent.as_object_mut().unwrap().remove("caption");
    parent["head"] = json!([]);
    parent["body"].as_array_mut().unwrap().truncate(1);
    for cell in parent["body"][0]["cells"].as_array_mut().unwrap() {
        cell["rowspan"] = 1.into();
    }
    parent["body"][0]["cells"][0]["blocks"] = if mode == "two-children" {
        json!([child.clone(), child.clone()])
    } else {
        json!([child.clone()])
    };
    parent["body"][0]["cells"][1]["blocks"] = if mode == "simultaneous" {
        json!([both.clone(), br.clone(), both.clone()])
    } else if mode == "earlier-parent" {
        json!([br.clone(), both.clone()])
    } else {
        json!([both.clone()])
    };
    if mode == "parallel-children" {
        parent["body"][0]["cells"][1]["blocks"] = json!([child]);
    }
    if mode == "following" {
        let mut row = parent["body"][0].clone();
        row["cells"][0]["blocks"] = json!([left.clone()]);
        row["cells"][1]["blocks"] = json!([right]);
        parent["body"].as_array_mut().unwrap().push(row);
    }
    if mode == "deep" {
        for _ in 0..2 {
            let mut outer = parent.clone();
            outer["body"][0]["cells"][1]["blocks"] = json!([]);
            outer["body"][0]["cells"][0]["blocks"] = json!([parent]);
            parent = outer;
        }
        data["page_masters"]["masters"][0]["trim"]["width"] = (800 * 65536).into();
        data["page_masters"]["masters"][0]["body"]["width"] = (780 * 65536).into();
    }
    data["document"]["blocks"] = json!([parent]);
    if mode == "following-table" {
        let mut table = data["document"]["blocks"][0].clone();
        table["body"][0]["cells"][0]["blocks"] = json!([left]);
        table["body"][0]["cells"][1]["blocks"] = json!([right]);
        data["document"]["blocks"]
            .as_array_mut()
            .unwrap()
            .push(table);
    }
    if mode == "notes" {
        let paragraph = &mut data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0]
            ["body"][0]["cells"][0]["blocks"][0];
        let para = paragraph.clone();
        paragraph["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":para["span"],"footnote_id":"note"}));
        data["document"]["footnotes"] =
            json!([{"node_id":0,"span":para["span"],"footnote_id":"note","blocks":vec![para;8]}]);
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":80*65536,"width":180*65536,"height":48*65536});
    }
    if mode == "definition-query" {
        let table = data["document"]["blocks"][0].clone();
        data["document"]["blocks"] = json!([both]);
        data["document"]["footnotes"] =
            json!([{"node_id":0,"span":table["span"],"footnote_id":"note","blocks":[table]}]);
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":80*65536,"width":180*65536,"height":48*65536});
    }
    renumber(&mut data["document"], &mut 0);
    data
}
#[test]
fn book_v2_nested_body_tables_preserve_child_fragments_and_source() {
    for (mode, expected) in [
        ("natural", 2),
        ("forced", 3),
        ("headers", 2),
        ("caption", 7),
        ("simultaneous", 3),
        ("earlier-parent", 4),
        ("following", 3),
        ("spacing", 2),
        ("deep", 2),
        ("notes", 0),
        ("child-span", 3),
        ("child-header-break", 5),
        ("two-children", 5),
        ("parallel-children", 3),
        ("following-table", 3),
    ] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, nested("LeftRight", 4, mode), b"LeftRight", &limits);
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                let actual = pdf.navigation().source().source().source().pages().len();
                if expected == 0 {
                    assert!(actual >= 3);
                } else {
                    assert_eq!(actual, expected, "{mode}");
                }
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap_or_else(|e| panic!("{mode}: {e:?}"));
    }
}

#[test]
fn book_v2_nested_body_tables_charge_retries_and_cover_original_cells_once() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements, prepare_book_v2_table_search,
    };
    use typaxis_pagination::ProductionBodyPaginationErrorKind as E;
    for mode in ["forced", "earlier-parent", "spacing", "definition-query"] {
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, nested("LeftRight", 4, mode), b"LeftRight", &limits);
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
                if mode == "definition-query" {
                    let mut parent =
                        prepare_book_v2_table_search(&measured, 0, &limits, 1_000_000, 0).unwrap();
                    let mut cursor = parent.begin().unwrap();
                    let mut original = Vec::new();
                    while !cursor.is_terminal() {
                        assert_eq!(cursor.definition_index(), Some(0));
                        let selected = parent
                            .evaluate(&cursor, parent.maximum_height())
                            .unwrap()
                            .unwrap();
                        original.extend(
                            selected
                                .source_leaf_ranges()
                                .collect::<Result<Vec<_>, _>>()
                                .unwrap()
                                .into_iter()
                                .flatten(),
                        );
                        for leaf in selected.source_placement_leaves() {
                            let leaf = leaf.unwrap();
                            assert_eq!(leaf.definition_index(), Some(0));
                            assert!(
                                leaf.item_index()
                                    < measured.flow().definition_items(0).unwrap().len()
                            );
                        }
                        cursor = selected.after();
                    }
                    original.sort_unstable();
                    assert_eq!(
                        original,
                        (0..measured.flow().definition_items(0).unwrap().len()).collect::<Vec<_>>()
                    );
                    let mut child =
                        prepare_book_v2_table_search(&measured, 1, &limits, 1_000_000, 0).unwrap();
                    let cursor = child.begin().unwrap();
                    let selected = child
                        .evaluate(&cursor, child.maximum_height())
                        .unwrap()
                        .unwrap();
                    let leaves = selected
                        .placement_leaves()
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();
                    assert_eq!(leaves.len(), 1);
                    assert!(leaves[0].1 >= measured.flow().body_items().len());
                    return;
                }
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
                            if mode == "forced" {
                                3
                            } else if mode == "earlier-parent" {
                                4
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
fn book_v2_nested_body_tables_render_original_harano() {
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
    let mut data = nested(text, 6, "forced");
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
fn book_v2_nested_definition_tables_remain_unpainted_when_unreferenced() {
    let root = Root::new();
    let limits = limits();
    let mut data = nested("LeftRight", 4, "forced");
    let table = data["document"]["blocks"][0].clone();
    let paragraph = table["body"][0]["cells"][1]["blocks"][0].clone();
    data["document"]["blocks"] = json!([paragraph]);
    data["document"]["footnotes"] =
        json!([{"node_id":0,"span":table["span"],"footnote_id":"note","blocks":[table]}]);
    data["page_masters"]["masters"][0]["footnote"] =
        json!({"x":10*65536,"y":80*65536,"width":180*65536,"height":48*65536});
    renumber(&mut data["document"], &mut 0);
    let input = prepared(&root, data, b"LeftRight", &limits);
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            let marked = pdf.navigation().source().source().source();
            assert_eq!(marked.pages().len(), 1);
            assert_eq!(
                marked
                    .source()
                    .display()
                    .source()
                    .source()
                    .unreferenced_definitions(),
                1
            );
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_natural_unequal_cell_lines_continue_without_common_cuts() {
    use typaxis_pagination::book_v2::prepare_book_v2_table_body_search;
    use typaxis_pagination::{
        ProductionBodyPaginationError, ProductionBodyPaginationErrorKind as E,
    };
    for mode in [
        "flat", "nested", "header", "caption", "span", "keep", "oversize",
    ] {
        let mut data = nested("LeftRight", 4, "natural");
        let mut table = data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0].clone();
        for paragraph in table["body"][0]["cells"][0]["blocks"]
            .as_array_mut()
            .unwrap()
        {
            paragraph["classes"] = json!(["natural"]);
        }
        let right = table["body"][0]["cells"][1]["blocks"][0].clone();
        table["body"][0]["cells"][1]["blocks"] = json!(vec![right; 4]);
        let left = table["body"][0]["cells"][0]["blocks"][0].clone();
        if mode == "header" {
            let mut header = table["body"][0].clone();
            for cell in header["cells"].as_array_mut().unwrap() {
                cell["blocks"].as_array_mut().unwrap().truncate(1);
                cell["blocks"][0]["classes"] = json!(["natural"]);
            }
            table["head"] = json!([header]);
        }
        if mode == "caption" {
            table["caption"] = json!([left.clone()]);
        }
        if mode == "span" {
            table["body"][0]["cells"][0]["rowspan"] = 2.into();
            let mut following = table["body"][0].clone();
            following["cells"] = json!([following["cells"][1].clone()]);
            following["cells"][0]["blocks"]
                .as_array_mut()
                .unwrap()
                .truncate(1);
            table["body"].as_array_mut().unwrap().push(following);
        }
        if mode == "nested" {
            data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"] = json!([table]);
        } else {
            data["document"]["blocks"] = json!([table]);
        }
        let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
        for (id, selector, name, value) in [
            (
                "natural",
                "paragraph.natural",
                "line_height",
                json!({"kind":"length","value":17*65536}),
            ),
            (
                "unequal",
                "paragraph.tall",
                "line_height",
                json!({"kind":"length","value":if mode=="oversize" {64*65536} else {23*65536}}),
            ),
            (
                "rigid",
                "paragraph.natural",
                "keep_with_next",
                json!({"kind":"boolean","value":mode=="keep"}),
            ),
        ] {
            rules.push(
                json!({"style_id":id,"selector":selector,"source_order":rules.len(),"extends":null,
                "declarations":[{"name":name,"important":false,"value":value}]}),
            );
        }
        renumber(&mut data["document"], &mut 0);
        super::definition_reservations::with_tables(data.clone(), |measured, limits| {
            let run = |prior, work| -> Result<_, ProductionBodyPaginationError> {
                let mut search = prepare_book_v2_table_body_search(measured, limits, work, prior)?;
                let stable =
                    search.select_stable_mixed_pages(limits.base().get().max_layout_passes)?;
                let placed = search.place_mixed_pages(stable.sequence())?;
                search.close_mixed_page_sources(&stable, &placed)?;
                assert_eq!(
                    placed.pages().len(),
                    match mode {
                        "header" | "span" => 4,
                        "caption" => 3,
                        _ => 2,
                    },
                    "{mode}"
                );
                assert!(stable.sequence().pages().iter().all(|p| p
                    .candidate()
                    .used_height()
                    .raw()
                    <= 48 * 65536));
                let repeats = placed
                    .pages()
                    .iter()
                    .flat_map(|p| p.fragments_with_roles())
                    .filter(|(_, _, repeat)| *repeat)
                    .count();
                assert_eq!(repeats > 0, mode == "header");
                Ok((
                    search.record_charge(),
                    search.work_steps(),
                    placed.pages().len(),
                ))
            };
            if matches!(mode, "keep" | "oversize") {
                assert!(
                    matches!(run(0,1_000_000),Err(e) if e.kind==E::Oversize),
                    "{mode}"
                );
                return;
            }
            let (records, work, count) =
                run(0, 1_000_000).unwrap_or_else(|e| panic!("{mode}: {e:?}"));
            let prior = limits.base().get().max_fragments - (records - measured.record_charge());
            assert_eq!(
                run(prior, work).unwrap(),
                (limits.base().get().max_fragments, work, count)
            );
            assert_eq!(run(prior + 1, work).unwrap_err().kind, E::FragmentLimit);
            assert!(matches!(
                run(prior, work - 1).unwrap_err().kind,
                E::TableSearchLimit | E::FootnoteSearchLimit
            ));
        });
        let root = Root::new();
        let limits = limits();
        let input = prepared(&root, data, b"LeftRight", &limits);
        let result = crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                assert!(!matches!(mode, "keep" | "oversize"));
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        );
        if matches!(mode, "keep" | "oversize") {
            let error = result.expect_err("oversize authored geometry must fail");
            assert!(
                matches!(error,crate::book_v2_resources::BookV2ConvergenceError::Stage {stage:"page stability",ref source}
                if source.downcast_ref::<ProductionBodyPaginationError>().is_some_and(|e|e.kind==E::Oversize)),
                "{mode}: {error:?}"
            );
        } else {
            result.unwrap_or_else(|e| panic!("{mode}: {e:?}"));
        }
    }
}
