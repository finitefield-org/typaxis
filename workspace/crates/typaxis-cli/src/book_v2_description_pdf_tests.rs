use super::*;

fn renumber(value: &mut Value, next: &mut u32) {
    if let Some(nodes) = value.as_array_mut() {
        for node in nodes {
            renumber(node, next);
        }
    } else if let Some(object) = value.as_object_mut() {
        if let Some(id) = object.get_mut("node_id") {
            *id = (*next).into();
            *next += 1;
        }
        for key in [
            "term",
            "blocks",
            "children",
            "items",
            "head",
            "body",
            "cells",
            "caption",
            "equation_number",
            "footnotes",
        ] {
            if let Some(child) = object.get_mut(key) {
                renumber(child, next);
            }
        }
    }
}

#[test]
fn book_v2_description_term_splits_with_real_links_and_authored_label_structure() {
    check_description(false);
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_description_original_harano_term_splits_with_exact_japanese_text() {
    check_description(true);
}

fn check_description(original_font: bool) {
    let term = if original_font {
        "用語 ".repeat(12)
    } else {
        "Result ".repeat(12)
    };
    let definition = if original_font {
        "説明本文"
    } else {
        "Proof"
    };
    let text = format!("{term}{definition}");
    let mut data = source_data(&text);
    let span = |start, end| json!({"source_id":0,"start_byte":start,"end_byte":end});
    let text_node = |start, end| json!({"kind":"text","node_id":0,"span":span(start,end),"text_span":{"text_id":0,"start_byte":start,"end_byte":end}});
    data["document"]["blocks"] = json!([{
        "kind":"description_list","node_id":0,"classes":[],"span":span(0,text.len()),"language":"en-us",
        "items":[{"node_id":0,"span":span(0,text.len()),"language":"de-de",
            "term":{"node_id":0,"span":span(0,term.len()),"classes":[],"language":"fr-ca","children":[
                {"kind":"link","node_id":0,"span":span(0,term.len()),"target":{"kind":"internal","anchor_id":"definition"},"children":[text_node(0,term.len())]}
            ]},
            "blocks":[{"kind":"paragraph","node_id":0,"classes":[],"span":span(term.len(),text.len()),"children":[
                {"kind":"anchor","node_id":0,"span":span(term.len(),term.len()),"anchor_id":"definition"},
                text_node(term.len(),text.len())
            ]}]
        }]
    }]);
    let length = |name, value| json!({"name":name,"important":false,"value":{"kind":"length","value":value}});
    data["style_sheet"]["rules"].as_array_mut().unwrap().extend([
        json!({"style_id":"description","selector":"description_list","source_order":3,"extends":"paragraph-text","declarations":[length("start_indent",131072),length("end_indent",65536),length("space_before",65536),length("space_after",65536)]}),
        json!({"style_id":"term","selector":"description_term","source_order":4,"extends":null,"declarations":[length("start_indent",65536)]}),
    ]);
    data["page_masters"]["masters"][0]["body"] =
        json!({"x":500000,"y":500000,"width":4000000,"height":3000000});
    data["page_masters"]["masters"][0]["width"] = 12_000_000.into();
    data["page_masters"]["masters"][0]["height"] = 12_000_000.into();
    data["page_masters"]["masters"][0]["trim"] =
        json!({"x":0,"y":0,"width":12_000_000,"height":12_000_000});
    renumber(&mut data["document"], &mut 0);
    let term_owner = NodeId::new(
        data["document"]["blocks"][0]["items"][0]["term"]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    let definition_owner = NodeId::new(
        data["document"]["blocks"][0]["items"][0]["blocks"][0]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    let root = Root::new();
    let limits = limits();
    let input = if original_font {
        let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
        let hash = typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(
            hash,
            "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
        );
        data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
        data["resources"]["font_faces"][0]["expected_sha256"] = hash.into();
        let body = body_with_source(&root, data, text.as_bytes(), &limits);
        fs::write(root.0.join("body.bin"), font).unwrap();
        prepare_book_v2_resources(
            body,
            &root.context(),
            &config(limits.base().get().clone()),
            &limits,
        )
        .unwrap()
    } else {
        prepared(&root, data, text.as_bytes(), &limits)
    };
    crate::book_v2_resources::with_converged_book_v2_pdf(
        &input,
        &limits,
        JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observation| {
            let registry = pdf.navigation().source().source();
            let marked = registry.source();
            let display = marked.source().display();
            let lines = display.source().source().flow().lines();
            let flow = lines.prepared().source_flow();
            assert_eq!(flow.description_items().len(), 1);
            assert!(flow.list_items().is_empty());
            assert_eq!(flow.generated_text_bytes(), 0);
            assert!(marked.pages().len() > 1);
            let term_lines = lines
                .paragraphs()
                .iter()
                .find(|p| p.owner() == term_owner)
                .unwrap();
            assert!(term_lines.lines().len() > 2);
            let frames = lines.frames().unwrap();
            assert_eq!(frames.paragraphs()[0].start().raw(), 131072 + 65536);
            assert_eq!(frames.paragraphs()[1].start().raw(), 131072);
            let label = registry
                .nodes()
                .iter()
                .find(|n| n.source().key().owner() == term_owner)
                .unwrap();
            assert_eq!(label.source().pdf_role(), "Lbl");
            assert_eq!(label.source().language(), "fr-CA");
            assert_eq!(
                label.source().key().slot(),
                typaxis_syntax::book_v2::BookV2StructureSlot::Source
            );
            let definition = registry
                .nodes()
                .iter()
                .find(|n| n.source().key().owner() == definition_owner)
                .unwrap();
            assert_eq!(definition.source().pdf_role(), "P");
            assert_eq!(definition.source().language(), "de-DE");
            assert!(!pdf.navigation().links().is_empty());
            crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
        },
    )
    .unwrap();
}
