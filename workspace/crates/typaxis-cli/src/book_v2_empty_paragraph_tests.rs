use super::*;
use typaxis_pdf::book_v2::BookV2DestinationKind;

#[test]
fn book_v2_empty_paragraph_consumes_height_and_anchors_the_start_of_its_container() {
    let mut data = source_data("Result");
    data["page_masters"]["masters"][0]["body"] =
        json!({"x":500_000,"y":500_000,"width":10_000_000,"height":3_000_000});
    let paragraph = data["document"]["blocks"][0]["blocks"][0].clone();
    let mut blank = paragraph.clone();
    blank["children"] =
        json!([{"kind":"anchor","node_id":0,"span":blank["span"],"anchor_id":"empty.line"}]);
    let mut reference = paragraph.clone();
    let reference_span = reference["span"].clone();
    reference["children"].as_array_mut().unwrap().insert(0,json!({"kind":"reference","node_id":0,"span":reference_span,"target":"blank-container","format":"page"}));
    data["document"]["blocks"][0]["anchor_id"] = "blank-container".into();
    data["document"]["blocks"][0]["blocks"] = json!([blank, paragraph, reference]);
    renumber(&mut data["document"], &mut 0);
    let empty_owner = NodeId::new(
        data["document"]["blocks"][0]["blocks"][0]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, data, b"Result", &limits);
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            assert_eq!(observation.candidate_passes(), 2);
            let nav = pdf.navigation();
            let registry = nav.source().source();
            let marked = registry.source();
            assert_eq!(marked.pages().len(), 2);
            let display = marked.source().display();
            assert_eq!(display.anchors().nonpainting_lines().len(), 1);
            let empty = &display.anchors().nonpainting_lines()[0];
            assert_eq!(empty.fragment().fragment().owner(), empty_owner);
            assert_eq!(
                empty.fragment().fragment().bounds().height().get().raw(),
                1_048_576
            );
            let destination = |name| {
                nav.destinations()
                    .iter()
                    .find(|d| matches!(d.kind(), BookV2DestinationKind::Anchor(id) if id == name))
                    .unwrap()
                    .position()
                    .unwrap()
            };
            let container = destination("blank-container");
            let anchor = destination("empty.line");
            let bounds = empty.fragment().fragment().bounds();
            assert_eq!(
                (container.page_index(), container.x(), container.y()),
                (0, bounds.x(), bounds.y())
            );
            assert_eq!(
                (anchor.page_index(), anchor.x().raw(), anchor.y().raw()),
                (0, bounds.x().raw(), bounds.y().raw() + 524_288)
            );
            let node = registry
                .nodes()
                .iter()
                .find(|n| n.source().key().owner() == empty_owner && n.source().pdf_role() == "P")
                .unwrap();
            assert!(node.first_binding().is_none());
            assert!(marked
                .source()
                .groups()
                .iter()
                .all(|g| g.owner() != Some(empty_owner)));
            assert!(pdf.page_reference_labels_match());
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}
#[test]
fn book_v2_empty_header_and_demanded_note_keep_source_lines_and_repeated_roles() {
    let mut data = notes_table_data();
    data["page_masters"]["masters"][0]["body"] =
        json!({"x":500_000,"y":500_000,"width":10_000_000,"height":3_000_000});
    let header =
        &mut data["document"]["blocks"][0]["blocks"][0]["head"][0]["cells"][0]["blocks"][0];
    header["children"] =
        json!([{"kind":"anchor","node_id":0,"span":header["span"],"anchor_id":"empty.header"}]);
    data["document"]["footnotes"][0]["blocks"][0]["children"] = json!([]);
    renumber(&mut data["document"], &mut 0);
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, data, b"Result", &limits);
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            let nav = pdf.navigation();
            let display = nav.source().source().source().source().display();
            let empty = display.anchors().nonpainting_lines();
            assert!(empty.iter().any(|p| p.repeated_header()));
            assert!(empty
                .iter()
                .any(|p| p.fragment().definition_index() == Some(0)));
            let destination = nav
                .destinations()
                .iter()
                .find(|d| matches!(d.kind(), BookV2DestinationKind::Anchor("empty.header")))
                .unwrap()
                .position()
                .unwrap();
            assert_eq!(destination.page_index(), 0);
            assert_eq!(
                nav.source().notes().iter().filter(|n| n.painted()).count(),
                2
            );
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_consecutive_authored_breaks_preserve_the_nonpainting_middle_line() {
    let mut data = source_data("Result");
    data["page_masters"]["masters"][0]["body"] =
        json!({"x":500_000,"y":500_000,"width":10_000_000,"height":3_000_000});
    data["document"]["blocks"][0]["anchor_id"] = "text-before-blank".into();
    let paragraph = &mut data["document"]["blocks"][0]["blocks"][0];
    let text = paragraph["children"][0].clone();
    let span = paragraph["span"].clone();
    paragraph["children"] = json!([text,
        {"kind":"hard_break","node_id":0,"span":span},
        {"kind":"anchor","node_id":0,"span":span,"anchor_id":"middle.blank"},
        {"kind":"hard_break","node_id":0,"span":span}, text]);
    renumber(&mut data["document"], &mut 0);
    let root = Root::new();
    let limits = limits();
    let input = prepared(&root, data, b"Result", &limits);
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            let nav = pdf.navigation();
            let display = nav.source().source().source().source().display();
            let selected = &display.source().source().flow().lines().paragraphs()[0];
            assert_eq!(selected.lines().len(), 3);
            assert_eq!(display.anchors().nonpainting_lines().len(), 1);
            assert_eq!(nav.source().source().source().pages().len(), 2);
            let target = nav
                .destinations()
                .iter()
                .find(|d| matches!(d.kind(), BookV2DestinationKind::Anchor("middle.blank")))
                .unwrap()
                .position()
                .unwrap();
            let empty = &display.anchors().nonpainting_lines()[0];
            let start = nav
                .destinations()
                .iter()
                .find(|d| matches!(d.kind(), BookV2DestinationKind::Anchor("text-before-blank")))
                .unwrap()
                .position()
                .unwrap();
            assert_eq!(start.fragment_index(), 0);
            assert!(start.y() < empty.fragment().fragment().bounds().y());
            assert_eq!(target.fragment_index(), empty.fragment_index());
            assert_eq!(target.y(), empty.fragment().fragment().baseline().unwrap());
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}
