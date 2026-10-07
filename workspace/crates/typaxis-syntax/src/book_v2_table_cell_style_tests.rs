use super::*;
use typaxis_style::MachineTextAlign;

fn table_input() -> Value {
    let mut input = root(FIXTURE);
    let original = input["document"]["blocks"][0].clone();
    let span = &original["span"];
    input["document"]["blocks"] = json!([{"kind":"table","node_id":0,"span":span,
        "classes":[],"columns":[{"kind":"fraction","weight":1}],
        "caption":[original["blocks"][0]],"head":[],
        "body":[{"node_id":0,"span":span,"cells":[{"node_id":0,"span":span,
            "classes":["aligned"],"colspan":1,"rowspan":1,
            "blocks":[original["blocks"][1],original["blocks"][2]]}]}]}]);
    input["style_sheet"]["rules"] = json!([
        {"style_id":"body","selector":"paragraph.caption","source_order":0,"extends":null,
         "declarations":[
             declaration("font_family",json!({"kind":"font_family_list","families":["Body"]}),false),
             declaration("font_size",length_value(12*65536),false),
             declaration("line_height",length_value(16*65536),false)]},
        {"style_id":"cell","selector":"table_cell.aligned","source_order":1,"extends":"body",
         "declarations":[declaration("text_align",json!({"kind":"keyword","value":"end"}),true)]}
    ]);
    input["document"]["blocks"][0]["caption"][0]["classes"] = json!(["caption"]);
    renumber(&mut input["document"], &mut 0);
    input
}

#[test]
fn cell_styles_inherit_into_children_without_styling_caption_or_overriding_children() {
    for (keyword, expected) in [
        ("start", MachineTextAlign::Start),
        ("end", MachineTextAlign::End),
        ("center", MachineTextAlign::Center),
    ] {
        let mut input = table_input();
        input["style_sheet"]["rules"][1]["declarations"][0]["value"]["value"] = keyword.into();
        let styled = style_book_v2_body(prepare(&input).unwrap()).unwrap();
        let nav = prepare_book_v2_navigation(&styled).unwrap();
        let flow = prepare_book_v2_text_flow(&styled, &nav).unwrap();
        assert_eq!(flow.paragraphs().len(), 3);
        assert_eq!(
            flow.paragraphs()[0].style().block_style().text_align(),
            MachineTextAlign::Start
        );
        for para in &flow.paragraphs()[1..] {
            assert_eq!(para.style().block_style().text_align(), expected);
            assert_eq!(para.style().font_families().unwrap(), ["Body"]);
        }
        input["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({
            "style_id":"child","selector":"paragraph","source_order":2,"extends":null,
            "declarations":[declaration("text_align",json!({"kind":"keyword","value":"center"}),false)]}));
        let styled = style_book_v2_body(prepare(&input).unwrap()).unwrap();
        let nav = prepare_book_v2_navigation(&styled).unwrap();
        let flow = prepare_book_v2_text_flow(&styled, &nav).unwrap();
        assert!(flow
            .paragraphs()
            .iter()
            .all(|p| p.style().block_style().text_align() == MachineTextAlign::Center));
    }
}

#[test]
fn cell_styles_keep_font_inheritance_and_extends_but_reject_geometry_even_when_unused() {
    let mut input = table_input();
    input["style_sheet"]["rules"][1]["extends"] = "body".into();
    let declarations = input["style_sheet"]["rules"][1]["declarations"]
        .as_array_mut()
        .unwrap();
    declarations.push(declaration("font_size", length_value(14 * 65536), false));
    declarations.push(declaration("line_height", length_value(20 * 65536), false));
    let styled = style_book_v2_body(prepare(&input).unwrap()).unwrap();
    let nav = prepare_book_v2_navigation(&styled).unwrap();
    let flow = prepare_book_v2_text_flow(&styled, &nav).unwrap();
    assert_eq!(
        flow.paragraphs()[0]
            .style()
            .font_size()
            .unwrap()
            .get()
            .raw(),
        12 * 65536
    );
    assert_eq!(
        flow.paragraphs()[1]
            .style()
            .font_size()
            .unwrap()
            .get()
            .raw(),
        14 * 65536
    );
    assert_eq!(
        flow.paragraphs()[1]
            .style()
            .line_height()
            .unwrap()
            .get()
            .raw(),
        20 * 65536
    );
    for ancestor in [0, 1] {
        for (name, value) in [
            ("space_before", length_value(1)),
            ("keep_with_next", json!({"kind":"boolean","value":true})),
            ("page", json!({"kind":"string","value":"other"})),
        ] {
            let mut bad = input.clone();
            bad["document"]["blocks"][0]["body"][0]["cells"][0]["classes"] = json!([]);
            bad["style_sheet"]["rules"][ancestor]["declarations"]
                .as_array_mut()
                .unwrap()
                .push(declaration(name, value, false));
            assert!(
                style_book_v2_body(prepare(&bad).unwrap()).is_err(),
                "{name} ancestor={ancestor}"
            );
        }
    }
}

#[test]
fn cell_classes_obey_canonical_identifier_rules_and_keep_original_owners() {
    let input = table_input();
    let prepared = prepare(&input).unwrap();
    let BookV2Block::Table { body, .. } = &prepared.document().blocks[0] else {
        panic!("table")
    };
    assert_eq!(body[0].cells[0].classes, ["aligned"]);
    assert_eq!(
        body[0].cells[0].node_id.get() as u64,
        input["document"]["blocks"][0]["body"][0]["cells"][0]["node_id"]
            .as_u64()
            .unwrap()
    );
    for classes in [
        json!(["z", "a"]),
        json!(["a", "a"]),
        json!(["__typaxis_internal_hidden"]),
        json!(["bad class"]),
    ] {
        let mut bad = input.clone();
        bad["document"]["blocks"][0]["body"][0]["cells"][0]["classes"] = classes;
        assert!(matches!(
            prepare(&bad),
            Err(StagingSemanticSyntaxError::InvalidClass)
        ));
    }
}

#[test]
fn cell_selector_is_private_to_book_v2() {
    let input = table_input();
    let sheet: WireStagingStyleSheet =
        serde_json::from_value(input["style_sheet"].clone()).unwrap();
    assert!(matches!(
        lower_semantic_style_rules(&sheet, &limits()),
        Err(StagingSemanticSyntaxError::InvalidStyle)
    ));
}
