use super::*;
use typaxis_syntax::book_v2::BookV2StructureSlot;

const SOURCE: &[u8] = b"Result \\frac{x}{1} x+y Proof";
const SVG: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/x-plus-y.svg"));

fn renumber(value: &mut Value, next: &mut u32) {
    match value {
        Value::Array(values) => {
            for value in values {
                renumber(value, next);
            }
        }
        Value::Object(object) => {
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
        _ => (),
    }
}
fn span(start: usize, end: usize) -> Value {
    json!({"source_id":0,"start_byte":start,"end_byte":end})
}
fn text(start: usize, end: usize) -> Value {
    json!({"kind":"text","node_id":0,"span":span(start,end),"text_span":{"text_id":0,"start_byte":start,"end_byte":end}})
}
fn paragraph() -> Value {
    json!({"kind":"paragraph","node_id":0,"span":span(23,SOURCE.len()),"classes":[],"children":[text(23,SOURCE.len())]})
}
fn description() -> Value {
    let metrics = json!({"advance":2031616,"ascent":655360,"baseline":589824,"descent":196608,"origin_x":-16384,"viewport":{"height":786432,"width":1966080}});
    json!({"kind":"description_list","node_id":0,"span":span(0,SOURCE.len()),"classes":[],"language":"en",
        "items":[{"node_id":0,"span":span(0,SOURCE.len()),"term":{"node_id":0,"span":span(0,22),"classes":[],"language":"ja","children":[
            text(0,7),
            {"kind":"inline_math","node_id":0,"span":span(7,18),"math_source":{"language":"typaxis-math","version":"1","text_span":{"text_id":0,"start_byte":7,"end_byte":18}},"speech":"x over one"},
            text(18,19),
            {"kind":"inline_vector","node_id":0,"image_id":1,"span":span(19,22),"metrics":metrics,"spacing":{"before":16384,"after":16384},"alt":"diagram","actual_text":null},
            {"kind":"math_vector","node_id":0,"image_id":1,"span":span(19,22),"metrics":metrics,"spacing":{"before":16384,"after":16384},"alt":"x plus y","actual_text":null,"source_tex":{"text_span":{"text_id":0,"start_byte":19,"end_byte":22}},"language":"fr"}
        ]},"blocks":[paragraph()]}]
    })
}
fn source(slot: &str) -> Vec<u8> {
    if slot == "nested" {
        [b"Result ".as_slice(), SOURCE].concat()
    } else {
        SOURCE.to_vec()
    }
}
fn data(slot: &str) -> Value {
    let source = source(slot);
    let mut data = source_data(std::str::from_utf8(&source).unwrap());
    let offset = if slot == "nested" { 7 } else { 0 };
    let mut ranges = Vec::new();
    if offset != 0 {
        ranges.push((0, offset));
    }
    ranges.extend(
        [(0, 7), (7, 18), (18, 19), (19, 22), (22, SOURCE.len())]
            .map(|(start, end)| (start + offset, end + offset)),
    );
    data["text_buffers"][0]["mappings"]=ranges.into_iter().map(|(start,end)|json!({"kind":"identity","source_span":span(start,end),"text_range":{"start_byte":start,"end_byte":end}})).collect::<Vec<_>>().into();
    let description = description();
    let all = span(0, source.len());
    let row = |block| json!({"node_id":0,"span":all,"cells":[{"node_id":0,"span":all,"colspan":1,"rowspan":1,"blocks":[block]}]});
    let block = match slot {
        "root" | "footnote" | "unreferenced-footnote" | "term-reference" => description,
        "container" => {
            json!({"kind":"semantic_container","semantic_kind":"example","anchor_id":null,"node_id":0,"span":all,"classes":[],"blocks":[description]})
        }
        "list" => {
            json!({"kind":"list","ordered":true,"start":3,"node_id":0,"span":all,"classes":[],"items":[{"node_id":0,"span":all,"blocks":[description]}]})
        }
        "nested" => {
            let mut child = description;
            fn shift(value: &mut Value) {
                match value {
                    Value::Object(fields) => {
                        for (key, value) in fields {
                            if key == "start_byte" || key == "end_byte" {
                                *value = (value.as_u64().unwrap() + 7).into();
                            } else {
                                shift(value);
                            }
                        }
                    }
                    Value::Array(values) => {
                        for value in values {
                            shift(value);
                        }
                    }
                    _ => (),
                }
            }
            shift(&mut child);
            json!({"kind":"description_list","node_id":0,"span":all,"classes":[],"items":[{
                "node_id":0,"span":all,"term":{"node_id":0,"span":span(0,7),"classes":[],"children":[text(0,7)]},"blocks":[child]
            }]})
        }
        "head" => {
            json!({"kind":"table","node_id":0,"span":all,"classes":[],"columns":[{"kind":"fraction","weight":1}],"head":[row(description)],"body":vec![row(paragraph());12]})
        }
        "body" => {
            json!({"kind":"table","node_id":0,"span":all,"classes":[],"columns":[{"kind":"fraction","weight":1}],"head":[],"body":[row(description)]})
        }
        "caption" => {
            json!({"kind":"figure","node_id":0,"span":all,"classes":[],"placement":"block","image_id":0,"alt":"raster caption","caption":[description]})
        }
        "vector-caption" => {
            json!({"kind":"vector_figure","node_id":0,"span":all,"classes":[],"image_id":1,"alt":"vector caption","viewport":{"height":786432,"width":1966080},"caption":[description]})
        }
        _ => panic!("unknown slot"),
    };
    data["document"]["blocks"] = json!([block]);
    if matches!(slot, "footnote" | "unreferenced-footnote") {
        data["document"]["footnotes"] = json!([{"node_id":0,"span":all,"footnote_id":"description.note","blocks":data["document"]["blocks"]}]);
        let mut p = paragraph();
        if slot == "footnote" {
            p["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":span(23,SOURCE.len()),"footnote_id":"description.note"}));
        }
        data["document"]["blocks"] = json!([p]);
    }
    if slot == "term-reference" {
        let item = &mut data["document"]["blocks"][0]["items"][0];
        item["term"]["children"].as_array_mut().unwrap().extend([
            json!({"kind":"reference","node_id":0,"span":span(22,22),"target":"definition","format":"page"}),
            json!({"kind":"footnote_reference","node_id":0,"span":span(22,22),"footnote_id":"description.note"})
        ]);
        item["blocks"][0]["children"]
            .as_array_mut()
            .unwrap()
            .insert(
                0,
                json!({"kind":"anchor","node_id":0,"span":span(23,23),"anchor_id":"definition"}),
            );
        data["document"]["footnotes"] = json!([{"node_id":0,"span":span(23,SOURCE.len()),"footnote_id":"description.note","blocks":[paragraph()]}]);
    }
    data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(NATIVE_BODY)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    data["resources"]["images"].as_array_mut().unwrap().push(json!({"image_id":1,"uri":"description.svg","media_type":"svg-safe-2","expected_sha256":typaxis_core::sha256(SVG).iter().map(|b|format!("{b:02x}")).collect::<String>(),"vector_provenance":{"engine_id":"vmb.texToSvg","engine_version":"2026.09.0","rules_version":"vmb.math-safe-svg/1"}}));
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    for selector in ["description_list", "list"] {
        let order = rules.len();
        rules.push(json!({"style_id":selector,"selector":selector,"source_order":order,"extends":"paragraph-text","declarations":[]}));
    }
    let order = rules.len();
    rules.push(json!({"style_id":"term","selector":"description_term","source_order":order,"extends":null,"declarations":[
        {"name":"font_size","important":false,"value":{"kind":"length","value":20*65536}},
        {"name":"line_height","important":false,"value":{"kind":"length","value":28*65536}}
    ]}));
    let order = rules.len();
    rules.push(json!({"style_id":"caption-figure","selector":"figure","source_order":order,"extends":null,"declarations":[{"name":"width","important":false,"value":{"kind":"length","value":500000}}]}));
    let master = &mut data["page_masters"]["masters"][0];
    master["width"] = 32_000_000.into();
    master["height"] = 32_000_000.into();
    master["trim"] = json!({"x":0,"y":0,"width":32_000_000,"height":32_000_000});
    master["body"] = json!({"x":500_000,"y":500_000,"width":30_000_000,"height":8_000_000});
    if matches!(
        slot,
        "footnote" | "unreferenced-footnote" | "term-reference"
    ) {
        master["footnote"] =
            json!({"x":500_000,"y":10_000_000,"width":30_000_000,"height":8_000_000});
    }
    renumber(&mut data["document"], &mut 0);
    data
}

#[test]
fn book_v2_description_math_vectors_survive_recursive_source_to_pdf() {
    for slot in [
        "root",
        "container",
        "nested",
        "list",
        "head",
        "body",
        "caption",
        "vector-caption",
        "footnote",
        "unreferenced-footnote",
        "term-reference",
    ] {
        let root = Root::new();
        let limits = limits();
        let body = body_with_source(&root, data(slot), &source(slot), &limits);
        fs::write(root.0.join("body.bin"), NATIVE_BODY).unwrap();
        fs::write(root.0.join("description.svg"), SVG).unwrap();
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
                let registry = pdf.navigation().source().source();
                let display = registry.source().source().display();
                let terminals = display.source();
                assert_eq!(
                    terminals.source().semantic_math(),
                    if slot == "unreferenced-footnote" {
                        0
                    } else {
                        2
                    },
                    "{slot}"
                );
                let flow = terminals.source().flow().lines().prepared().source_flow();
                let native = input.body().styled().body().math().first().unwrap();
                let term = native.domain().owner_node_id;
                let paragraph = flow
                    .paragraphs()
                    .iter()
                    .find(|p| p.owner() == term)
                    .unwrap();
                let label = registry
                    .nodes()
                    .iter()
                    .find(|n| n.source().key().owner() == term)
                    .unwrap();
                assert_eq!(label.source().pdf_role(), "Lbl");
                assert_eq!(label.source().key().slot(), BookV2StructureSlot::Source);
                assert!(flow
                    .description_items()
                    .iter()
                    .any(|i| flow.paragraphs()[i.term_paragraph_index() as usize].owner() == term));
                assert_eq!(
                    paragraph.style().font_size().unwrap().get().raw(),
                    20 * 65536
                );
                if slot == "head" {
                    assert!(registry.source().pages().len() > 1);
                    assert!(terminals.source().repeated_math() > 0);
                }
                if slot == "unreferenced-footnote" {
                    assert_eq!(terminals.source().unreferenced_definitions(), 1);
                    assert!(terminals.terminals().is_empty());
                }
                if slot == "term-reference" {
                    assert!(pdf.page_reference_labels_match());
                    assert!(!pdf.page_references().is_empty());
                    assert_eq!(terminals.source().unreferenced_definitions(), 0);
                    let relations = pdf.navigation().source();
                    let note = relations.notes()[0].node_index();
                    let parent = relations.nodes()[note].parent().unwrap();
                    assert_eq!(
                        registry.nodes()[parent].source().key(),
                        label.source().key()
                    );
                }
                assert_math_terminals(terminals);
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap_or_else(|e| panic!("{slot}: {e:?}"));
    }
}
