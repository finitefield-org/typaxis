use super::*;

#[path = "book_v2_description_source_tests.rs"]
mod source_tests;

fn description(fixture: &[u8]) -> Value {
    let mut input = root(fixture);
    let container = input["document"]["blocks"][0].clone();
    let mut term = container["blocks"][0].clone();
    term.as_object_mut().unwrap().remove("kind");
    term["classes"] = json!(["authored-term"]);
    let definition = container["blocks"].as_array().unwrap()[1..].to_vec();
    input["document"]["blocks"] = json!([{
        "kind":"description_list","node_id":1,"span":container["span"],"classes":[],
        "items":[{"node_id":2,"span":container["span"],"term":term,"blocks":definition}]
    }]);
    renumber(&mut input["document"], &mut 0);
    input
}

#[test]
fn description_domain_preserves_distinct_term_and_definition_owners() {
    let input = description(FIXTURE);
    let prepared = prepare(&input).unwrap();
    let BookV2Block::DescriptionList { common, items } = &prepared.document().blocks[0] else {
        panic!("description kind was lost")
    };
    assert_eq!(common.node_id, NodeId::new(1));
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(item.node_id, NodeId::new(2));
    assert_eq!(item.term.common.node_id, NodeId::new(3));
    assert_eq!(item.term.common.classes, ["authored-term"]);
    assert!(item.term.has_authored_content);
    assert_eq!(item.term.common.span.start_byte().get(), 0);
    assert_eq!(item.term.common.span.end_byte().get(), 6);
    assert_eq!(item.blocks.len(), 2);
    assert_eq!(item.blocks[0].node_id(), NodeId::new(5));
    assert_eq!(
        item.blocks[0].semantic_kind(),
        Some(BookV2SemanticContainerKind::Proof)
    );
    assert!(prepared.document().blocks[0].is_semantically_nonempty());
    assert_eq!(
        serde_json::to_value(prepared.wire().document()).unwrap(),
        input["document"]
    );
}

#[test]
fn description_domain_rejects_invalid_ids_spans_classes_and_noncontent_terms() {
    for case in [
        "node",
        "item-span",
        "term-span",
        "inline-span",
        "definition-order",
        "classes",
        "break-only",
        "empty-text",
    ] {
        let mut input = description(FIXTURE);
        let item = &mut input["document"]["blocks"][0]["items"][0];
        match case {
            "node" => item["term"]["node_id"] = 99.into(),
            "item-span" => item["span"]["start_byte"] = 6.into(),
            "term-span" => item["term"]["span"]["end_byte"] = 30.into(),
            "inline-span" => item["term"]["children"][0]["span"]["end_byte"] = 19.into(),
            "definition-order" => {
                item["term"]["span"]["start_byte"] = 7.into();
                item["term"]["span"]["end_byte"] = 11.into();
                item["term"]["children"][0]["span"] = item["term"]["span"].clone();
            }
            "classes" => item["term"]["classes"] = json!(["z", "a"]),
            "break-only" => {
                let old = item["term"]["children"][0].clone();
                item["term"]["children"] =
                    json!([{"kind":"hard_break", "node_id":old["node_id"], "span":old["span"]}]);
            }
            "empty-text" => item["term"]["children"][0]["text_span"]["end_byte"] = 0.into(),
            _ => unreachable!(),
        }
        let error = prepare(&input).unwrap_err();
        assert_eq!(
            error,
            match case {
                "node" => StagingSemanticSyntaxError::InvalidNodeOrder,
                "classes" => StagingSemanticSyntaxError::InvalidClass,
                "break-only" | "empty-text" =>
                    StagingSemanticSyntaxError::InvalidBlock(NodeId::new(3)),
                _ => StagingSemanticSyntaxError::InvalidSourceSpan,
            },
            "{case}"
        );
    }
}

#[test]
fn description_term_native_math_uses_term_style_and_keeps_parsed_source() {
    let mut input = description(MATH);
    let rules = input["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"description","selector":"description_list","extends":"math-base","source_order":2,"declarations":[]}));
    rules.push(json!({"style_id":"term","selector":"description_term.authored-term","extends":null,"source_order":3,
        "declarations":[declaration("font_size",length_value(20*65536),false)]}));
    rules.push(
        json!({"style_id":"ordinary","selector":"paragraph","extends":null,"source_order":4,
        "declarations":[declaration("font_size",length_value(50*65536),true)]}),
    );
    let prepared = prepare(&input).unwrap();
    let math = prepared
        .math()
        .iter()
        .find(|m| m.domain().kind == StagingM4MathKind::Inline)
        .unwrap();
    assert_eq!(math.domain().owner_node_id, NodeId::new(3));
    let math_id = math.domain().node_id;
    let original = math.domain().source.as_ptr();
    let styled = style_book_v2_body(prepared).unwrap();
    let style = styled.math_style(math_id).unwrap();
    assert_eq!(style.font_families(), ["Math"]);
    assert_eq!(style.font_size().get().raw(), 20 * 65536);
    let display = styled
        .body()
        .math()
        .iter()
        .find(|m| m.domain().kind == StagingM4MathKind::Display)
        .unwrap();
    assert_eq!(
        styled
            .math_style(display.domain().node_id)
            .unwrap()
            .font_size()
            .get()
            .raw(),
        12 * 65536
    );
    assert_eq!(
        styled
            .body()
            .math()
            .iter()
            .find(|m| m.domain().node_id == math_id)
            .unwrap()
            .domain()
            .source
            .as_ptr(),
        original
    );
    // Description selectors are still rejected by the frozen style entry point.
    assert!(lower_semantic_style_rules(styled.body().wire().style_sheet(), &limits()).is_err());
}

#[test]
fn description_term_vectors_retain_original_inline_ownership() {
    let input = description(VECTOR);
    let prepared = prepare(&input).unwrap();
    let BookV2Block::DescriptionList { items, .. } = &prepared.document().blocks[0] else {
        unreachable!()
    };
    assert_eq!(items[0].term.inline_vectors.len(), 2);
    for vector in &items[0].term.inline_vectors {
        assert_eq!(vector.owner_node_id, items[0].term.common.node_id);
        assert!(prepared
            .vectors()
            .iter()
            .any(|v| v.node_id() == vector.node_id && v.image_id() == vector.image_id));
    }
}

#[test]
fn description_domains_are_preserved_in_all_recursive_block_slots() {
    fn descriptions(blocks: &[BookV2Block]) -> usize {
        blocks
            .iter()
            .map(|block| match block {
                BookV2Block::DescriptionList { items, .. } => {
                    1 + items.iter().map(|i| descriptions(&i.blocks)).sum::<usize>()
                }
                BookV2Block::List { items, .. } => {
                    items.iter().map(|i| descriptions(&i.blocks)).sum()
                }
                BookV2Block::Table { head, body, .. } => head
                    .iter()
                    .chain(body)
                    .flat_map(|r| &r.cells)
                    .map(|c| descriptions(&c.blocks))
                    .sum(),
                _ => descriptions(block.direct_blocks()),
            })
            .sum()
    }
    for slot in [
        "container",
        "list",
        "description",
        "head",
        "body",
        "figure",
        "vector-figure",
        "footnote",
    ] {
        let mut input = description(FIXTURE);
        let child = input["document"]["blocks"][0].clone();
        let span = child["span"].clone();
        let outer = match slot {
            "container" => {
                json!({"kind":"semantic_container","semantic_kind":"solution","anchor_id":null,"node_id":1,"span":span,"classes":[],"blocks":[child]})
            }
            "list" => {
                json!({"kind":"list","node_id":1,"span":span,"classes":[],"ordered":false,"start":null,"items":[{"node_id":2,"span":span,"blocks":[child]}]})
            }
            "description" => {
                let mut outer = child.clone();
                outer["items"][0]["blocks"] = json!([child]);
                outer
            }
            "head" | "body" => {
                let mut table = json!({"kind":"table","node_id":1,"span":span,"classes":[],"columns":[{"kind":"fraction","weight":1}],"head":[],"body":[]});
                table[slot] = json!([{"node_id":2,"span":span,"cells":[{"node_id":3,"span":span,"rowspan":1,"colspan":1,"blocks":[child]}]}]);
                table
            }
            "figure" => {
                json!({"kind":"figure","node_id":1,"span":span,"classes":[],"image_id":0,"placement":"block","alt":"diagram","caption":[child]})
            }
            "vector-figure" => {
                json!({"kind":"vector_figure","node_id":1,"span":span,"classes":[],"image_id":0,"viewport":{"width":65536,"height":65536},"alt":"diagram","caption":[child]})
            }
            "footnote" => child,
            _ => unreachable!(),
        };
        if slot == "footnote" {
            input["document"]["blocks"] = json!([]);
            input["document"]["footnotes"] =
                json!([{"node_id":1,"span":span,"footnote_id":"term.note","blocks":[outer]}]);
        } else {
            input["document"]["blocks"] = json!([outer]);
        }
        renumber(&mut input["document"], &mut 0);
        let prepared = prepare(&input).unwrap_or_else(|e| panic!("{slot}: {e}"));
        let count = descriptions(&prepared.document().blocks)
            + prepared
                .document()
                .footnotes
                .iter()
                .map(|note| descriptions(&note.blocks))
                .sum::<usize>();
        assert_eq!(count, if slot == "description" { 2 } else { 1 }, "{slot}");
        assert_eq!(
            serde_json::to_value(prepared.wire().document()).unwrap(),
            input["document"]
        );
    }
}
