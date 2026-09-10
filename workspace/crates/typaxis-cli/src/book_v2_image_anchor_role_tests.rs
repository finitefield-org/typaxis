use super::*;
use typaxis_display_list::book_v2::{BookV2ImageSource, BookV2MathDisplayBuilder};

#[test]
fn book_v2_anchor_only_paragraph_has_a_nonpainting_line_and_real_pdf_destination() {
    let mut data = source_data("Result");
    data["page_masters"]["masters"][0]["body"] =
        json!({"x":500_000,"y":500_000,"width":10_000_000,"height":3_000_000});
    let paragraph = &mut data["document"]["blocks"][0]["blocks"][0];
    paragraph["children"] = json!([{"kind":"anchor","node_id":3,"span":paragraph["span"],"anchor_id":"empty-paragraph"}]);
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, data, b"Result", &limits);
    let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let bindings =
        typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits).unwrap();
    typaxis_layout::book_v2::with_converged_book_v2_body_lines(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        &limits,
        JapaneseLineBreakMode::Normal,
        rect(500_000, 500_000, 10_000_000, 3_000_000),
        1_000_000,
        |stable| {
            let paragraph = &stable.lines().paragraphs()[0];
            assert_eq!(paragraph.lines().len(), 1);
            assert!(paragraph.lines()[0].items().is_empty());
            assert_eq!(paragraph.anchors().len(), 1);
            let position = paragraph.anchors()[0].position().unwrap();
            assert_eq!(position.line_index(), 0);
            assert_eq!(position.x().raw(), 0);
            assert_eq!(position.baseline(), paragraph.lines()[0].baseline());
            let measured = prepare_book_v2_table_measurements(
                prepare_book_v2_body_flow(stable.lines(), None, stable.footnotes(), &limits, 0)
                    .unwrap(),
                &limits,
            )
            .unwrap();
            let mut search =
                prepare_book_v2_table_body_search(&measured, &limits, 10_000_000, 0).unwrap();
            let pages = search.select_stable_mixed_pages(2).unwrap();
            assert_eq!(pages.sequence().pages().len(), 1);
            let placed = search.place_mixed_pages(pages.sequence()).unwrap();
            let closure = search.close_mixed_page_sources(&pages, &placed).unwrap();
            let terminals = search
                .finalize_mixed_page_math(closure, &limits, 0)
                .unwrap();
            assert_math_display(&terminals, input.resources(), &limits);
        },
    )
    .unwrap();
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observed| {
            assert_eq!(observed.candidate_passes(), 1);
            let nav = pdf.navigation();
            assert_eq!(nav.destinations().len(), 1);
            let target = nav.destinations()[0].position().unwrap();
            assert_eq!(target.page_index(), 0);
            let empty = &nav
                .source()
                .source()
                .source()
                .source()
                .display()
                .anchors()
                .nonpainting_lines()[0];
            assert_eq!(target.x(), empty.fragment().fragment().bounds().x());
            assert!(target.y().raw() > 500_000);
            let display = nav.source().source().source().source().display();
            assert!(display.paints().is_empty());
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observed, &limits);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_images_and_anchors_keep_note_and_header_roles_without_duplicate_semantics() {
    let mut data = notes_table_data();
    let span = data["document"]["blocks"][0]["span"].clone();
    let anchor = |id| json!({"kind":"anchor","node_id":0,"span":span,"anchor_id":id});
    let figure = |alt| json!({"kind":"figure","node_id":0,"classes":[],"span":span,"image_id":0,"placement":"block","alt":alt,"caption":[]});
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    let header = &mut data["document"]["blocks"][0]["blocks"][0]["head"][0]["cells"][0]["blocks"];
    header[0]["children"]
        .as_array_mut()
        .unwrap()
        .insert(0, anchor("header-start"));
    header[0]["children"]
        .as_array_mut()
        .unwrap()
        .push(anchor("header-end"));
    header.as_array_mut().unwrap().push(figure("header image"));
    let note = &mut data["document"]["footnotes"][0]["blocks"];
    note[0]["children"]
        .as_array_mut()
        .unwrap()
        .insert(0, anchor("note-start"));
    note.as_array_mut().unwrap().insert(1, figure("note image"));
    let mut unused = paragraph.clone();
    unused["children"]
        .as_array_mut()
        .unwrap()
        .push(anchor("unreferenced-note"));
    data["document"]["footnotes"].as_array_mut().unwrap().push(json!({
        "node_id":0,"span":span,"footnote_id":"unused","blocks":[unused, figure("unreferenced image")]
    }));
    let mut body = paragraph.clone();
    let text = body["children"][0].clone();
    body["children"] = json!([
        anchor("body-start"), text, anchor("body-before-break"),
        {"kind":"hard_break","node_id":0,"span":span},
        anchor("body-after-break"), text, anchor("body-end")
    ]);
    data["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(0, body);
    let order = data["style_sheet"]["rules"].as_array().unwrap().len();
    data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"small-image","selector":"figure","source_order":order,"extends":null,"declarations":[{"name":"width","important":false,"value":{"kind":"length","value":500_000}}]}));
    with_measured_resources_options(
        data,
        limits(),
        rect(500_000, 500_000, 10_000_000, 3_000_000),
        |measured, limits, admitted| {
            let mut search =
                prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
            let stable = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
            let terminals = search.finalize_mixed_page_math(closure, limits, 0).unwrap();
            assert_math_display(&terminals, admitted, limits);
            let mut builder =
                BookV2MathDisplayBuilder::new(&terminals, admitted, limits, 10_000_000, 0, 0)
                    .unwrap();
            let images = builder.build_images().unwrap();
            let header = images
                .draws()
                .iter()
                .filter(|d| d.source().alternative() == "header image")
                .collect::<Vec<_>>();
            assert!(header.len() > 2);
            assert_eq!(
                header
                    .iter()
                    .filter(|d| !d.cell_role().unwrap().repeated_header())
                    .count(),
                1
            );
            for copy in &header[1..] {
                let (BookV2ImageSource::Figure(a), BookV2ImageSource::Figure(b)) =
                    (copy.source(), header[0].source())
                else {
                    panic!()
                };
                assert!(std::ptr::eq(a, b));
                assert!(std::ptr::eq(copy.image(), header[0].image()));
                assert!(copy.cell_role().unwrap().repeated_header());
                assert_ne!(
                    copy.fragment().fragment().page_index(),
                    header[0].fragment().fragment().page_index()
                );
            }
            let notes = images
                .draws()
                .iter()
                .filter(|d| d.source().alternative() == "note image")
                .collect::<Vec<_>>();
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].fragment().definition_index(), Some(0));
            assert!(images
                .draws()
                .iter()
                .all(|d| d.source().alternative() != "unreferenced image"));
            let anchors = builder.build_anchors().unwrap();
            assert!(anchors.unpositioned().is_empty());
            assert_eq!(
                anchors
                    .positions()
                    .iter()
                    .filter(|a| a.fragment().definition_index().is_some())
                    .count(),
                1
            );
            assert_eq!(
                anchors
                    .positions()
                    .iter()
                    .filter(|a| a.cell_role().is_some())
                    .count(),
                header.len() * 2
            );
            assert_eq!(anchors.positions().len(), 5 + header.len() * 2);
            assert_eq!(
                anchors
                    .positions()
                    .iter()
                    .filter(|a| a.repeated_header())
                    .count(),
                (header.len() - 1) * 2
            );
            // Original image bytes and authored text are separate from anchor records.
            assert!(images.draws().iter().all(|d| d.image().bytes() == PNG));
            let text = builder.build_text().unwrap();
            let visible = text
                .draws()
                .iter()
                .map(|d| d.exact_text())
                .collect::<String>();
            for id in [
                "header-start",
                "header-end",
                "note-start",
                "body-start",
                "empty-paragraph",
                "unreferenced-note",
            ] {
                assert!(!visible.contains(id));
            }
        },
    );
}

#[test]
fn book_v2_navigation_keeps_multiline_uri_internal_reference_and_outline_geometry() {
    let mut data = notes_table_data();
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    let mut heading_style = rules
        .iter()
        .find(|r| r["style_id"] == "paragraph-text")
        .unwrap()
        .clone();
    heading_style["style_id"] = "navigation-heading".into();
    heading_style["selector"] = "heading".into();
    heading_style["source_order"] = rules.len().into();
    rules.push(heading_style);
    let span = data["document"]["blocks"][0]["span"].clone();
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    let text = paragraph["children"][0].clone();
    let link = |target: Value, children: Value| json!({"kind":"link","node_id":0,"span":span,"target":target,"children":children});
    let heading = |anchor| json!({"kind":"heading","node_id":0,"classes":[],"span":span,"level":2,"anchor_id":anchor,"children":[text]});
    let header =
        &mut data["document"]["blocks"][0]["blocks"][0]["head"][0]["cells"][0]["blocks"][0];
    header["children"][0] = link(
        json!({"kind":"uri","uri":"https://example.invalid/header"}),
        json!([text]),
    );
    header["children"].as_array_mut().unwrap().insert(
        0,
        json!({"kind":"anchor","node_id":0,"span":span,"anchor_id":"header.begin"}),
    );
    let mut body = paragraph.clone();
    let mut wrapped = Vec::new();
    for i in 0..24 {
        if i != 0 {
            wrapped.push(json!({"kind":"soft_break","node_id":0,"span":span}));
        }
        wrapped.push(text.clone());
    }
    body["children"] = json!([
        link(json!({"kind":"uri","uri":"https://example.invalid/read?x=(a)&y=b"}),json!(wrapped)),
        {"kind":"hard_break","node_id":0,"span":span},
        link(json!({"kind":"internal","anchor_id":"header.begin"}),json!([text])),
        {"kind":"reference","node_id":0,"span":span,"target":"chapter.first","format":"text"}
    ]);
    let blocks = data["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    blocks.insert(0, heading("chapter.first"));
    blocks.insert(1, body);
    blocks.push(heading("chapter.last"));
    data["document"]["blocks"][0]["anchor_id"] = "book.root".into();
    renumber(&mut data["document"], &mut 0);
    let root_id = data["document"]["blocks"][0]["node_id"].clone();
    let first_id = data["document"]["blocks"][0]["blocks"][0]["node_id"].clone();
    let last_id = data["document"]["blocks"][0]["blocks"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["node_id"]
        .clone();
    data["outline"]["entries"] = json!([
        {"outline_id":0,"parent_outline_id":null,"level":1,"destination":"book.root","label":"Book","source_node_id":root_id,"source_kind":"semantic_container"},
        {"outline_id":1,"parent_outline_id":0,"level":2,"destination":"chapter.first","label":"First","source_node_id":first_id,"source_kind":"heading"},
        {"outline_id":2,"parent_outline_id":0,"level":2,"destination":"chapter.last","label":"Last","source_node_id":last_id,"source_kind":"heading"}
    ]);
    with_measured_resources_options(
        data,
        limits(),
        rect(500_000, 500_000, 10_000_000, 3_000_000),
        |measured, limits, admitted| {
            let mut search =
                prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
            let stable = search.select_stable_mixed_pages(2).unwrap();
            let placed = search.place_mixed_pages(stable.sequence()).unwrap();
            assert!(placed.pages().len() > 2);
            let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
            let terminals = search.finalize_mixed_page_math(closure, limits, 0).unwrap();
            assert_math_display(&terminals, admitted, limits);
        },
    );
}

#[test]
fn book_v2_navigation_rejects_conflicting_note_links_and_unplaced_anchor_targets() {
    for conflict in [true, false] {
        let mut data = framed_data();
        let span = data["document"]["footnotes"][0]["span"].clone();
        let mut paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
        let text = paragraph["children"][0].clone();
        paragraph["children"] = if conflict {
            let id = data["document"]["footnotes"][0]["footnote_id"].clone();
            json!([{"kind":"link","node_id":0,"span":span,"target":{"kind":"uri","uri":"https://example.invalid/conflict"},"children":[text,{"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":id}]}])
        } else {
            data["document"]["footnotes"][0]["blocks"][0]["children"]
                .as_array_mut()
                .unwrap()
                .push(
                    json!({"kind":"anchor","node_id":0,"span":span,"anchor_id":"unplaced.target"}),
                );
            json!([{"kind":"link","node_id":0,"span":span,"target":{"kind":"internal","anchor_id":"unplaced.target"},"children":[text]}])
        };
        data["document"]["blocks"][0]["blocks"] = json!([paragraph]);
        with_measured_resources_options(
            data,
            limits(),
            rect(500_000, 500_000, 10_000_000, 3_000_000),
            |measured, limits, admitted| {
                let mut search =
                    prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
                let stable = search.select_stable_mixed_pages(2).unwrap();
                let placed = search.place_mixed_pages(stable.sequence()).unwrap();
                let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
                let terminals = search.finalize_mixed_page_math(closure, limits, 0).unwrap();
                // The actual body and structure remain valid. The navigation check
                // derives and asserts the precise source-owned semantic rejection.
                assert_math_display(&terminals, admitted, limits);
            },
        );
    }
}

#[test]
fn book_v2_pdf_assembly_keeps_source_trim_metadata_and_rejects_unselected_masters() {
    for multiple in [false, true] {
        let mut data = framed_data();
        let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
        data["document"]["blocks"][0]["blocks"] = json!([paragraph]);
        data["document"]["footnotes"] = json!([]);
        data["metadata"]["title"] = "Book 2 — 日本語".into();
        data["metadata"]["author"] = "A & B <編集>".into();
        data["metadata"]["subject"] = "元の裁ち落とし領域".into();
        data["metadata"]["identifier"] = "urn:typaxis:book-2:source-trim".into();
        data["metadata"]["keywords"] = json!(["links & notes", "本文"]);
        data["metadata"]["created"] = "2026-09-05T00:00:00Z".into();
        data["metadata"]["modified"] = "2026-09-09T01:02:03Z".into();
        data["page_masters"]["masters"][0]["trim"] =
            json!({"x":250_000,"y":125_000,"width":11_500_000,"height":23_750_000});
        if multiple {
            let mut second = data["page_masters"]["masters"][0].clone();
            second["master_id"] = "unselected-second-master".into();
            data["page_masters"]["masters"]
                .as_array_mut()
                .unwrap()
                .push(second);
        }
        with_measured_resources_options(
            data,
            limits(),
            rect(500_000, 500_000, 10_000_000, 3_000_000),
            |measured, limits, admitted| {
                let mut search =
                    prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
                let stable = search.select_stable_mixed_pages(2).unwrap();
                let placed = search.place_mixed_pages(stable.sequence()).unwrap();
                let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
                let terminals = search.finalize_mixed_page_math(closure, limits, 0).unwrap();
                assert_math_display(&terminals, admitted, limits);
            },
        );
    }
}

fn forward_page_reference_data() -> (Value, NodeId, NodeId) {
    let mut data = notes_table_data();
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    let span = paragraph["span"].clone();
    let mut reference = paragraph.clone();
    reference["children"] = json!([{"kind":"reference","node_id":0,"span":span,"target":"later.target","format":"page"}]);
    let mut target = data["document"]["blocks"][0].clone();
    target["blocks"] = json!([paragraph]);
    target["anchor_id"] = "later.target".into();
    let blocks = data["document"]["blocks"][0]["blocks"]
        .as_array_mut()
        .unwrap();
    blocks.insert(0, reference.clone());
    blocks.push(target);
    reference["children"] = json!([
        {"kind":"anchor","node_id":0,"span":span,"anchor_id":"unused.target"},
        {"kind":"reference","node_id":0,"span":span,"target":"unused.target","format":"page"}
    ]);
    data["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"node_id":0,"span":span,"footnote_id":"unused","blocks":[reference]}));
    renumber(&mut data["document"], &mut 0);
    let forward = NodeId::new(
        data["document"]["blocks"][0]["blocks"][0]["children"][0]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    let unused = NodeId::new(
        data["document"]["footnotes"][2]["blocks"][0]["children"][1]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    (data, forward, unused)
}

#[test]
fn book_v2_pdf_feedback_observes_forward_pages_and_keeps_unplaced_references_explicit() {
    let (data, forward, unused) = forward_page_reference_data();
    let mut values = vec![(forward, 1), (unused, 77)];
    let mut previous = None;
    for pass in 0..3 {
        let mut observed = None;
        with_measured_resources_candidates(
            data.clone(),
            limits(),
            rect(500_000, 500_000, 10_000_000, 3_000_000),
            Some(&values),
            |measured, limits, admitted| {
                let mut search =
                    prepare_book_v2_table_body_search(measured, limits, 10_000_000, 0).unwrap();
                let stable = search.select_stable_mixed_pages(2).unwrap();
                let placed = search.place_mixed_pages(stable.sequence()).unwrap();
                let closure = search.close_mixed_page_sources(&stable, &placed).unwrap();
                let terminals = search.finalize_mixed_page_math(closure, limits, 0).unwrap();
                assert_math_display(&terminals, admitted, limits);
                let mut display_builder =
                    typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                        &terminals,
                        admitted,
                        limits,
                        100_000_000,
                        0,
                        0,
                    )
                    .unwrap();
                let display = display_builder.build_body().unwrap();
                let mut pipeline = typaxis_pdf::book_v2::BookV2PdfPipeline::new(
                    &display,
                    11,
                    limits,
                    100_000_000,
                    0,
                    0,
                    0,
                    0,
                )
                .unwrap();
                observed = Some(
                    pipeline
                        .with_pdf(|pdf| {
                            assert_eq!(pdf.page_references().len(), 2);
                            let actual = pdf.page_references()[0];
                            assert_eq!(actual.owner(), forward);
                            assert!(actual.is_placed());
                            assert_eq!(actual.candidate_page(), values[0].1);
                            assert!(actual.target_page().unwrap() > 1);
                            let unplaced = pdf.page_references()[1];
                            assert_eq!(unplaced.owner(), unused);
                            assert!(!unplaced.is_placed());
                            assert_eq!(unplaced.candidate_page(), 77);
                            assert_eq!(unplaced.target_page(), None);
                            assert_eq!(pdf.page_reference_labels_match(), pass != 0);
                            (
                                actual.target_page().unwrap(),
                                typaxis_core::sha256(pdf.bytes()),
                            )
                        })
                        .unwrap(),
                );
            },
        );
        let (target, fingerprint) = observed.unwrap();
        if pass == 2 {
            assert_eq!(previous, Some(fingerprint));
        }
        values[0].1 = target;
        previous = Some(fingerprint);
    }
}

#[path = "book_v2_converged_pdf_tests.rs"]
mod converged_pdf;

#[path = "book_v2_empty_paragraph_tests.rs"]
mod empty_paragraphs;

#[path = "book_v2_description_pdf_tests.rs"]
mod descriptions;

#[path = "book_v2_number_binding_pdf_tests.rs"]
mod number_bindings;
