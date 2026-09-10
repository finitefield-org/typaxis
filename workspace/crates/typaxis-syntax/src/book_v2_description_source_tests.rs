use super::*;
use crate::{ProductionFlowEvent as E, ProductionFlowRegionKind as F, ProductionInlineContent};

fn styled_input() -> Value {
    let mut input = description(FIXTURE);
    input["style_sheet"]["rules"].as_array_mut().unwrap().extend([
        json!({"style_id":"description","selector":"description_list","extends":null,"source_order":2,
            "declarations":[
                declaration("font_family",json!({"kind":"font_family_list","families":["Body"]}),false),
                declaration("font_size",length_value(12*65536),false),
                declaration("line_height",length_value(16*65536),false)]}),
        json!({"style_id":"term","selector":"description_term.authored-term","extends":null,"source_order":3,
            "declarations":[declaration("font_size",length_value(20*65536),false)]}),
    ]);
    input
}

#[test]
fn description_languages_preserve_list_item_term_and_inline_inheritance() {
    use BookV2LanguageNodeKind as K;
    let mut input = styled_input();
    input["document"]["language"] = "ja-jp".into();
    input["document"]["blocks"][0]["language"] = "en-us".into();
    input["document"]["blocks"][0]["items"][0]["language"] = "de-de".into();
    input["document"]["blocks"][0]["items"][0]["term"]["language"] = "fr-ca".into();
    let body = style_book_v2_body(prepare(&input).unwrap()).unwrap();
    let nav = prepare_book_v2_navigation(&body).unwrap();
    for (owner, kind, parent, explicit, effective) in [
        (0, K::Document, None, Some("ja-JP"), "ja-JP"),
        (1, K::DescriptionList, Some(0), Some("en-US"), "en-US"),
        (2, K::DescriptionItem, Some(1), Some("de-DE"), "de-DE"),
        (3, K::DescriptionTerm, Some(2), Some("fr-CA"), "fr-CA"),
        (4, K::Text, Some(3), None, "fr-CA"),
        (5, K::SemanticContainer, Some(2), None, "de-DE"),
        (6, K::Paragraph, Some(5), None, "de-DE"),
    ] {
        let record = nav.language(NodeId::new(owner)).unwrap();
        assert_eq!(record.kind(), kind);
        assert_eq!(record.parent(), parent.map(NodeId::new));
        assert_eq!(record.explicit_language(), explicit);
        assert_eq!(record.effective_language(), effective);
    }
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    assert_eq!(flow.description_items()[0].language(), "de-DE");
    assert_eq!(flow.paragraphs()[0].items()[0].language(), "fr-CA");
    assert_eq!(flow.paragraphs()[1].items()[0].language(), "de-DE");
    assert_eq!(
        serde_json::to_value(body.body().wire().document()).unwrap(),
        input["document"]
    );
    for path in [
        "/document/blocks/0/language",
        "/document/blocks/0/items/0/language",
        "/document/blocks/0/items/0/term/language",
    ] {
        let mut invalid = input.clone();
        *invalid.pointer_mut(path).unwrap() = "bad_tag".into();
        let body = style_book_v2_body(prepare(&invalid).unwrap()).unwrap();
        assert!(prepare_book_v2_navigation(&body).is_err(), "{path}");
    }
}

#[test]
fn description_flow_preserves_source_order_styles_and_borrowed_term_text_without_markers() {
    let input = styled_input();
    let body = style_book_v2_body(prepare(&input).unwrap()).unwrap();
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    flow.verify_for(&body, &nav).unwrap();
    assert_eq!(flow.description_lists().len(), 1);
    let item = &flow.description_items()[0];
    assert_eq!(
        (
            item.owner().get(),
            item.list_index(),
            item.item_index(),
            item.term_paragraph_index()
        ),
        (2, 0, 0, 0)
    );
    assert!(flow.lists().is_empty());
    assert!(flow.list_items().is_empty());
    assert_eq!(flow.generated_text_bytes(), 0);
    assert_eq!(
        &flow.events()[..6],
        &[
            E::Begin {
                owner: NodeId::new(1),
                kind: F::DescriptionList
            },
            E::Begin {
                owner: NodeId::new(2),
                kind: F::DescriptionItem
            },
            E::Begin {
                owner: NodeId::new(3),
                kind: F::DescriptionTerm
            },
            E::Paragraph { index: 0 },
            E::End {
                owner: NodeId::new(3)
            },
            E::Begin {
                owner: NodeId::new(5),
                kind: F::SemanticContainer
            },
        ]
    );
    assert_eq!(
        flow.paragraphs()[0]
            .style()
            .font_size()
            .unwrap()
            .get()
            .raw(),
        20 * 65536
    );
    assert_eq!(
        flow.paragraphs()[1]
            .style()
            .font_size()
            .unwrap()
            .get()
            .raw(),
        12 * 65536
    );
    assert_eq!(
        flow.description_lists()[0]
            .style()
            .font_size()
            .unwrap()
            .get()
            .raw(),
        12 * 65536
    );
    let ProductionInlineContent::Text { utf8, .. } = flow.paragraphs()[0].items()[0].content()
    else {
        panic!()
    };
    assert_eq!(utf8, "Result");
    assert_eq!(
        utf8.as_ptr(),
        body.body().wire().text_buffers()[0].utf8.as_ptr()
    );
    let mut missing = input.clone();
    missing["style_sheet"]["rules"][2]["declarations"] = json!([]);
    let body = style_book_v2_body(prepare(&missing).unwrap()).unwrap();
    let nav = prepare_book_v2_navigation(&body).unwrap();
    assert_eq!(
        prepare_book_v2_text_flow(&body, &nav).err().unwrap().kind,
        crate::ProductionFlowErrorKind::MissingTextStyle
    );
}

struct Capture<'a>(Vec<BookV2StructureSourceNode<'a>>);
impl<'a> BookV2StructureVisitor<'a> for Capture<'a> {
    type Error = BookV2StructureSourceError;
    fn step(&mut self, _: usize) -> Result<(), Self::Error> {
        Ok(())
    }
    fn node(&mut self, n: BookV2StructureSourceNode<'a>) -> Result<(), Self::Error> {
        self.0.push(n);
        Ok(())
    }
}

#[test]
fn description_source_structure_uses_authored_label_and_definition_body() {
    use BookV2StructureSlot as S;
    let body = style_book_v2_body(prepare(&styled_input()).unwrap()).unwrap();
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let mut capture = Capture(Vec::new());
    visit_book_v2_structure(&flow, &mut capture).unwrap();
    let observed: Vec<_> = capture
        .0
        .iter()
        .map(|n| {
            (
                n.key().owner().get(),
                n.key().slot(),
                n.pdf_role(),
                n.parent(),
            )
        })
        .collect();
    let key = |n, s| Some(BookV2StructureKey::new(NodeId::new(n), s));
    assert_eq!(
        &observed[..7],
        &[
            (0, S::Source, "Document", None),
            (1, S::Source, "L", key(0, S::Source)),
            (2, S::Source, "LI", key(1, S::Source)),
            (3, S::Source, "Lbl", key(2, S::Source)),
            (4, S::Source, "Span", key(3, S::Source)),
            (2, S::ListBody, "LBody", key(2, S::Source)),
            (5, S::Source, "Sect", key(2, S::ListBody)),
        ]
    );
    assert!(!observed.iter().any(|(_, s, _, _)| *s == S::ListLabel));
}

#[test]
fn description_terms_keep_vector_language_owners() {
    let input = description(VECTOR);
    let body = style_book_v2_body(prepare(&input).unwrap()).unwrap();
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let BookV2Block::DescriptionList { items, .. } = &body.body().document().blocks[0] else {
        panic!()
    };
    for vector in &items[0].term.inline_vectors {
        let record = nav.language(vector.node_id).unwrap();
        assert_eq!(record.parent(), Some(items[0].term.common.node_id));
        assert!(std::ptr::eq(
            record.vector().unwrap(),
            body.body()
                .vectors()
                .iter()
                .find(|v| v.node_id() == vector.node_id)
                .unwrap()
        ));
    }
}

#[test]
fn description_topology_and_inherited_languages_share_the_actual_source_limits() {
    let mut input = styled_input();
    input["document"]["blocks"][0]["items"][0]["term"]["language"] = "fr-ca".into();
    let styled = style_book_v2_body(prepare(&input).unwrap()).unwrap();
    let nav = prepare_book_v2_navigation(&styled).unwrap();
    let retained = nav.retained_text_bytes();
    for (max_fragments, max_text_bytes, expected) in [(2, retained, true), (1, retained, false)] {
        let configured = ValidatedResourceLimits::new(ResourceLimits {
            max_fragments,
            max_text_bytes,
            max_text_buffer_bytes: max_text_bytes as u32,
            max_shaping_context_bytes: max_text_bytes as u32,
            ..ResourceLimits::default()
        })
        .unwrap();
        let body = style_book_v2_body(
            prepare_book_v2_body(decode(&input, &configured), &configured).unwrap(),
        )
        .unwrap();
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let result = prepare_book_v2_text_flow(&body, &nav);
        if expected {
            result.unwrap().verify_for(&body, &nav).unwrap();
        } else {
            assert_eq!(
                result.err().unwrap().kind,
                crate::ProductionFlowErrorKind::NodeLimit
            );
        }
    }
    let configured = ValidatedResourceLimits::new(ResourceLimits {
        max_text_bytes: retained - 1,
        max_text_buffer_bytes: (retained - 1) as u32,
        max_shaping_context_bytes: (retained - 1) as u32,
        ..ResourceLimits::default()
    })
    .unwrap();
    let body =
        style_book_v2_body(prepare_book_v2_body(decode(&input, &configured), &configured).unwrap())
            .unwrap();
    assert!(prepare_book_v2_navigation(&body).is_err());
}
