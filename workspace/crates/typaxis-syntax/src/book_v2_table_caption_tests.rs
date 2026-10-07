use super::*;
use crate::book_v2::{
    prepare_book_v2_navigation, prepare_book_v2_text_flow, visit_book_v2_structure,
    BookV2StructureSourceError, BookV2StructureSourceNode, BookV2StructureVisitor,
};
use crate::{ProductionFlowEvent, ProductionFlowRegionKind};

fn captioned(nested: bool) -> Value {
    let mut input = root(FIXTURE);
    let original = input["document"]["blocks"][0].clone();
    let span = original["span"].clone();
    let title = original["blocks"][0].clone();
    let row = |blocks: Value| {
        json!({"node_id":0,"span":span,"cells":[{
        "node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":blocks}]})
    };
    let mut caption = vec![title];
    let body = if nested {
        caption.push(json!({"kind":"table","node_id":0,"span":span,"classes":[],
            "columns":[{"kind":"fraction","weight":1}],"head":[],
            "body":[row(json!([original["blocks"][1]]))]}));
        vec![original["blocks"][2].clone()]
    } else {
        vec![original["blocks"][1].clone(), original["blocks"][2].clone()]
    };
    input["document"]["blocks"] = json!([{"kind":"table","node_id":0,"span":span,
        "classes":[],"language":"ja","columns":[{"kind":"fraction","weight":1}],
        "caption":caption,"head":[],"body":[row(json!(body))]}]);
    input["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({
        "style_id":"caption-table","selector":"paragraph","source_order":2,"extends":null,
        "declarations":[
            declaration("font_family",json!({"kind":"font_family_list","families":["Body"]}),false),
            declaration("font_size",length_value(12 * 65536),false),
            declaration("line_height",length_value(16 * 65536),false)
        ]
    }));
    renumber(&mut input["document"], &mut 0);
    input
}

struct Capture<'a>(Vec<BookV2StructureSourceNode<'a>>);
impl<'a> BookV2StructureVisitor<'a> for Capture<'a> {
    type Error = BookV2StructureSourceError;
    fn step(&mut self, _: usize) -> Result<(), Self::Error> {
        Ok(())
    }
    fn node(&mut self, node: BookV2StructureSourceNode<'a>) -> Result<(), Self::Error> {
        self.0.push(node);
        Ok(())
    }
}

#[test]
fn table_caption_preserves_domain_language_source_flow_and_structure() {
    for nested in [false, true] {
        let input = captioned(nested);
        let body = style_book_v2_body(prepare(&input).unwrap()).unwrap();
        let BookV2Block::Table {
            caption,
            head,
            body: rows,
            ..
        } = &body.body().document().blocks[0]
        else {
            panic!("table")
        };
        assert_eq!(caption.len(), if nested { 2 } else { 1 });
        assert_eq!(body.body().document().blocks[0].direct_blocks(), caption);
        assert!(head.is_empty());
        assert_eq!(rows.len(), 1);
        assert_eq!(caption[0].span().start_byte().get(), 0);
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let owner = caption[0].node_id();
        assert_eq!(nav.language(owner).unwrap().effective_language(), "ja");
        let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
        flow.verify_for(&body, &nav).unwrap();
        assert_eq!(flow.tables().len(), if nested { 2 } else { 1 });
        assert_eq!(flow.paragraphs().len(), 3);
        let extent = flow.tables()[0].caption_event_range().unwrap();
        assert_eq!(
            flow.events()[extent.start],
            ProductionFlowEvent::Begin {
                owner,
                kind: ProductionFlowRegionKind::Paragraph,
            }
        );
        assert!(matches!(
            flow.events()[extent.end],
            ProductionFlowEvent::Begin {
                kind: ProductionFlowRegionKind::TableBodyRow,
                ..
            }
        ));
        if nested {
            assert!(flow.events()[extent.clone()].iter().any(|e| matches!(e,
                ProductionFlowEvent::Begin { kind: ProductionFlowRegionKind::Table, owner }
                if *owner == flow.tables()[1].owner())));
            assert!(flow.tables()[1].caption_event_range().is_none());
        }
        let mut capture = Capture(Vec::new());
        visit_book_v2_structure(&flow, &mut capture).unwrap();
        let table = capture
            .0
            .iter()
            .position(|n| n.pdf_role() == "Table")
            .unwrap();
        let cap = &capture.0[table + 1];
        assert_eq!(cap.pdf_role(), "Caption");
        assert_eq!(cap.parent(), Some(capture.0[table].key()));
        assert!(cap.table_cell().is_none());
        let para = &capture.0[table + 2];
        assert_eq!(para.key().owner(), owner);
        assert_eq!(para.parent(), Some(cap.key()));
        assert!(para.table_cell().is_none());
        assert!(capture.0.iter().any(|n| n.table_cell().is_some()));
        assert_eq!(
            serde_json::to_value(body.body().wire().document()).unwrap(),
            input["document"]
        );
    }
}

#[test]
fn table_caption_number_binding_uses_real_descendant_text() {
    let mut input = captioned(false);
    input["document"]["number_bindings"] = json!([{
        "anchor_id":"table.result","owner_node_id":1,"label_node_id":3,
        "text_span":{"text_id":0,"start_byte":0,"end_byte":6}
    }]);
    let body = style_book_v2_body(prepare(&input).unwrap()).unwrap();
    let nav = prepare_book_v2_navigation(&body).unwrap();
    assert_eq!(nav.reference_number("table.result"), Some("Result"));
    assert_eq!(nav.number_bindings()[0].owner(), NodeId::new(1));
    assert_eq!(nav.number_bindings()[0].label_owner(), NodeId::new(3));
}

#[test]
fn table_caption_rejects_duplicate_owners_and_out_of_order_source() {
    let mut input = captioned(false);
    input["document"]["blocks"][0]["caption"][0]["node_id"] = 1.into();
    assert!(prepare(&input).is_err());
    let mut input = captioned(false);
    input["document"]["blocks"][0]["caption"][0]["span"]["start_byte"] = 6.into();
    input["document"]["blocks"][0]["caption"][0]["children"][0]["span"]["start_byte"] = 6.into();
    assert!(matches!(
        prepare(&input),
        Err(StagingSemanticSyntaxError::InvalidSourceSpan)
    ));
}
