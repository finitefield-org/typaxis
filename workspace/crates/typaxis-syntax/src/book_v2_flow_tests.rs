use super::*;
use crate::book_v2::{prepare_book_v2_body, prepare_book_v2_navigation, style_book_v2_body};
use serde_json::Value;
use typaxis_core::ResourceLimits;
use typaxis_document_package::{
    book_v2::BookV2DocumentPackageDecoder, DocumentPackageDecodePolicy,
};

const COMBINED: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json"));
fn limits() -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits::default()).unwrap()
}
fn input() -> Value {
    let mut input: Value = serde_json::from_slice(COMBINED).unwrap();
    input["contract"] = "typaxis.contract/1.5".into();
    input
}
fn styled(input: &Value, limits: &ValidatedResourceLimits) -> StyledBookV2Body {
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(
            &serde_json::to_vec(input).unwrap(),
            &DocumentPackageDecodePolicy::new(limits),
        )
        .unwrap();
    style_book_v2_body(prepare_book_v2_body(decoded, limits).unwrap()).unwrap()
}

#[test]
fn successor_common_flow_retains_body_tables_lists_footnotes_and_source_text() {
    let body = styled(&input(), &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    flow.verify_for(&body, &nav).unwrap();
    let base = limits();
    let decoded = typaxis_document_package::StagingSemanticDocumentPackageDecoder::new()
        .decode(COMBINED, &DocumentPackageDecodePolicy::new(&base))
        .unwrap();
    let old = StagingSemanticPackageParser::new()
        .parse(decoded, &base)
        .unwrap();
    let old_limits = M4EffectiveResourceLimits::defaults_for(&base);
    let old_nav = crate::validate_staging_book_navigation_v2(&old, &old_limits).unwrap();
    let old_flow = prepare_production_text_flow(&old, &old_nav, &old_limits).unwrap();
    assert_eq!(flow.events(), old_flow.events());
    assert_eq!(flow.paragraphs(), old_flow.paragraphs());
    assert_eq!(flow.tables(), old_flow.tables());
    assert_eq!(flow.figures(), old_flow.figures());
    assert_eq!(flow.lists(), old_flow.lists());
    assert_eq!(flow.list_items(), old_flow.list_items());
    assert_eq!(flow.footnote_definitions(), old_flow.footnote_definitions());
    assert_eq!(flow.generated, old_flow.generated);
    assert_eq!(flow.table_record_charge(), old_flow.table_record_charge());
    assert_eq!(flow.text_bytes(), old_flow.text_bytes());
    assert_ne!(flow.fingerprint(), old_flow.fingerprint());
    assert_eq!(flow.paragraphs().len(), 27);
    assert_eq!(flow.tables().len(), 1);
    assert_eq!(flow.footnote_marker_text(NodeId::new(11)), Some("1"));
    assert_eq!(flow.footnote_marker_text(NodeId::new(92)), Some("1"));
    let reference = flow.paragraphs()[1].items()[1];
    assert_eq!(
        reference.reference(),
        Some(ProductionInlineReference::Anchor {
            target: "top",
            target_owner: NodeId::new(1),
            format: ProductionReferenceFormat::Page,
        })
    );
    assert!(flow.page_reference_text(reference.owner()).is_none());
    for site in flow.paragraphs().iter().flat_map(|p| p.items()) {
        if let ProductionInlineContent::Text { span, utf8 } = site.content() {
            let source = &body.body().wire().text_buffers()[span.text_id().get() as usize].utf8;
            assert!(utf8.as_ptr() >= source.as_ptr());
            assert!(
                (utf8.as_ptr() as usize) + utf8.len() <= (source.as_ptr() as usize) + source.len()
            );
        }
    }
}

#[test]
fn successor_flow_rejects_reparsed_body_and_separately_prepared_navigation() {
    let body = styled(&input(), &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let other = styled(&input(), &limits());
    assert_eq!(
        body.body().canonical_jcs_sha256(),
        other.body().canonical_jcs_sha256()
    );
    assert_eq!(
        prepare_book_v2_text_flow(&other, &nav).err().unwrap().kind,
        ProductionFlowErrorKind::ReceiptMismatch
    );
    assert_eq!(
        flow.verify_for(&other, &nav).unwrap_err().kind,
        ProductionFlowErrorKind::ReceiptMismatch
    );
    let other_nav = prepare_book_v2_navigation(&body).unwrap();
    assert_eq!(
        flow.verify_for(&body, &other_nav).unwrap_err().kind,
        ProductionFlowErrorKind::ReceiptMismatch
    );
    let reparsed_nav = prepare_book_v2_navigation(&other).unwrap();
    let other_flow = prepare_book_v2_text_flow(&other, &reparsed_nav);
    assert_eq!(flow.fingerprint(), other_flow.unwrap().fingerprint());
}

#[test]
fn successor_candidate_pages_are_complete_bounded_and_in_generated_namespace() {
    let body = styled(&input(), &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let owner = NodeId::new(7);
    let flow = prepare_book_v2_text_flow_with_page_references(&body, &nav, &[(owner, 12)]).unwrap();
    flow.verify_for(&body, &nav).unwrap();
    assert_eq!(flow.page_reference_text(owner), Some("12"));
    assert_eq!(flow.page_reference_values(), Some([(owner, 12)].as_slice()));
    assert!(flow.page_reference_provenance(owner).is_some());
    for values in [
        vec![],
        vec![(owner, 0)],
        vec![(owner, body.body().limits().get().max_pages + 1)],
        vec![(owner, 1), (owner, 2)],
        vec![(NodeId::new(8), 1)],
        vec![(owner, 1), (NodeId::new(8), 1)],
    ] {
        assert_eq!(
            prepare_book_v2_text_flow_with_page_references(&body, &nav, &values)
                .err()
                .unwrap()
                .kind,
            ProductionFlowErrorKind::ReceiptMismatch
        );
    }
    let mut text_input = input();
    text_input["document"]["blocks"][1]["children"][1]["format"] = "text".into();
    for (label, expected) in [
        ("text", ProductionReferenceFormat::Text),
        ("number", ProductionReferenceFormat::Number),
    ] {
        text_input["document"]["blocks"][1]["children"][1]["format"] = label.into();
        if label == "number" {
            text_input["document"]["number_bindings"] = serde_json::json!([{
                "anchor_id":"top", "owner_node_id":1, "label_node_id":2,
                "text_span":{"text_id":0,"start_byte":0,"end_byte":5}
            }]);
        }
        let body = styled(&text_input, &limits());
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
        assert!(matches!(flow.paragraphs()[1].items()[1].reference(),
            Some(ProductionInlineReference::Anchor { format, .. }) if format == expected));
        assert!(flow.page_reference_text(owner).is_none());
        assert!(
            prepare_book_v2_text_flow_with_page_references(&body, &nav, &[(owner, 1)]).is_err()
        );
    }
}

#[test]
fn successor_generated_text_shares_navigation_budget_at_exact_boundary() {
    let mut data = input();
    for text_label in [false, true] {
        if text_label {
            data["document"]["blocks"][1]["children"][1]["format"] = "text".into();
        }
        let values = if text_label {
            &[][..]
        } else {
            &[(NodeId::new(7), 12)][..]
        };
        let body = styled(&data, &limits());
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let flow = prepare_book_v2_text_flow_with_page_references(&body, &nav, values).unwrap();
        let generated = flow.generated_text_bytes();
        assert!(generated > 2);
        let total = nav.retained_text_bytes() + generated;
        for (bound, success) in [(total, true), (total - 1, false)] {
            let mut configured = ResourceLimits::default();
            configured.max_text_bytes = bound;
            configured.max_text_buffer_bytes = u32::try_from(bound).unwrap();
            configured.max_shaping_context_bytes = u32::try_from(bound).unwrap();
            let configured = ValidatedResourceLimits::new(configured).unwrap();
            let body = styled(&data, &configured);
            let nav = prepare_book_v2_navigation(&body).unwrap();
            let result = prepare_book_v2_text_flow_with_page_references(&body, &nav, values);
            if success {
                let flow = result.unwrap();
                assert_eq!(flow.generated_text_bytes(), generated);
                flow.verify_for(&body, &nav).unwrap();
            } else {
                assert_eq!(
                    result.err().unwrap().kind,
                    ProductionFlowErrorKind::TextLimit
                );
            }
        }
    }
}

#[test]
fn successor_language_reaches_text_and_candidate_markers_without_rewriting_source() {
    let mut data = input();
    data["document"]["blocks"][0]["language"] = "ja-jp".into();
    data["document"]["blocks"][0]["children"][0]["language"] = "en-us".into();
    data["document"]["footnotes"][0]["language"] = "de-de".into();
    let body = styled(&data, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    assert_eq!(flow.paragraphs()[0].items()[0].language(), "en-US");
    assert_eq!(flow.paragraphs()[0].items()[1].language(), "ja-JP");
    assert_eq!(flow.paragraphs()[0].items()[2].language(), "ja-JP");
    assert_eq!(flow.footnote_definitions()[0].language(), "de-DE");
    assert_eq!(flow.footnote_marker_text(NodeId::new(92)), Some("1"));
    assert_eq!(
        serde_json::to_value(body.body().wire().document()).unwrap(),
        data["document"]
    );
}

#[test]
fn successor_flow_requires_authored_text_style() {
    let mut data = input();
    for rule in data["style_sheet"]["rules"].as_array_mut().unwrap() {
        if rule["selector"] == "heading" {
            rule["declarations"]
                .as_array_mut()
                .unwrap()
                .retain(|d| d["name"] != "font_family");
        }
    }
    let body = styled(&data, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    assert_eq!(
        prepare_book_v2_text_flow(&body, &nav).err().unwrap(),
        ProductionFlowError {
            owner: NodeId::new(1),
            kind: ProductionFlowErrorKind::MissingTextStyle,
        }
    );
}

#[test]
fn successor_flow_precharges_table_topology_with_the_common_bound() {
    for maximum in [23, 22] {
        let configured = ValidatedResourceLimits::new(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        })
        .unwrap();
        let body = styled(&input(), &configured);
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let result = prepare_book_v2_text_flow(&body, &nav);
        if maximum == 23 {
            assert_eq!(result.unwrap().table_record_charge(), maximum);
        } else {
            assert_eq!(
                result.err().unwrap(),
                ProductionFlowError {
                    owner: NodeId::new(23),
                    kind: ProductionFlowErrorKind::NodeLimit,
                }
            );
        }
    }
}

#[test]
fn successor_verification_recomputes_events_styles_and_generated_text() {
    let body = styled(&input(), &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let mut flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    flow.events.pop();
    assert_eq!(
        flow.verify_for(&body, &nav).unwrap_err().kind,
        ProductionFlowErrorKind::ReceiptMismatch
    );
    let mut flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    flow.generated = typaxis_text::GeneratedTextOverlay::new(
        Vec::new(),
        body.body().limits(),
        nav.retained_text_bytes(),
    )
    .unwrap();
    assert_eq!(
        flow.verify_for(&body, &nav).unwrap_err().kind,
        ProductionFlowErrorKind::ReceiptMismatch
    );
    let mut flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    flow.paragraphs[0].style = flow.paragraphs[1].style.clone();
    assert_eq!(
        flow.verify_for(&body, &nav).unwrap_err().kind,
        ProductionFlowErrorKind::ReceiptMismatch
    );
}

#[test]
fn successor_text_references_use_authored_outline_then_plain_heading_in_counter_namespace() {
    let mut data = input();
    data["document"]["blocks"][1]["children"][1]["format"] = "text".into();
    let owner = NodeId::new(
        data["document"]["blocks"][1]["children"][1]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    for outline in [true, false] {
        let mut source = data.clone();
        if !outline {
            source["outline"]["entries"] = serde_json::json!([]);
        }
        let body = styled(&source, &limits());
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
        let expected = if outline {
            nav.outline()
                .iter()
                .find(|e| e.destination.as_str() == "top")
                .unwrap()
                .label
                .clone()
        } else {
            flow.paragraphs()[0]
                .items()
                .iter()
                .filter_map(|i| match i.content() {
                    ProductionInlineContent::Text { utf8, .. } => Some(utf8),
                    ProductionInlineContent::SoftBreak | ProductionInlineContent::HardBreak => {
                        Some(" ")
                    }
                    _ => None,
                })
                .collect::<String>()
        };
        assert!(!expected.is_empty());
        assert_eq!(flow.reference_text(owner), Some(expected.as_str()));
        assert!(flow.page_reference_text(owner).is_none());
        let provenance = flow.reference_provenance(owner).unwrap();
        assert_eq!(
            provenance.buffer_key(),
            typaxis_core::GeneratedBufferKey::new(owner, typaxis_core::GenerationKind::Counter, 0)
        );
        flow.verify_for(&body, &nav).unwrap();
    }
    // No generated identifier or placeholder substitutes for a complex heading
    // lacking an explicitly authored outline label.
    data["outline"]["entries"] = serde_json::json!([]);
    let old = data["document"]["blocks"][0]["children"][0].clone();
    data["document"]["blocks"][0]["children"][0] = serde_json::json!({"kind":"reference","node_id":old["node_id"],"span":old["span"],"target":"top","format":"text"});
    let body = styled(&data, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let error = prepare_book_v2_text_flow(&body, &nav).err().unwrap();
    assert_eq!(error.kind, ProductionFlowErrorKind::MissingReferenceLabel);
    assert_eq!(
        error.owner,
        NodeId::new(old["node_id"].as_u64().unwrap() as u32)
    );
}

#[path = "book_v2_number_binding_tests.rs"]
mod number_binding_tests;

#[test]
fn table_caption_references_and_nested_metadata_are_bound_to_exact_events() {
    let mut data = input();
    let mut blocks = data["document"]["blocks"].as_array().unwrap().clone();
    let last = blocks.pop().unwrap();
    let mut span = last["span"].clone();
    span["start_byte"] = 0.into();
    data["document"]["blocks"] = serde_json::json!([{"kind":"table","node_id":1,
        "span":span,"classes":[],"columns":[{"kind":"fraction","weight":1}],
        "caption":blocks,"head":[],"body":[{"node_id":0,"span":last["span"],
            "cells":[{"node_id":0,"span":last["span"],"colspan":1,"rowspan":1,"blocks":[last]}]}]}]);
    fn renumber(v: &mut Value, next: &mut u32) {
        if let Some(a) = v.as_array_mut() {
            for c in a {
                renumber(c, next);
            }
        } else if let Some(o) = v.as_object_mut() {
            if let Some(id) = o.get_mut("node_id") {
                *id = (*next).into();
                *next += 1;
            }
            for key in [
                "blocks",
                "children",
                "items",
                "caption",
                "head",
                "body",
                "cells",
                "equation_number",
                "footnotes",
            ] {
                if let Some(c) = o.get_mut(key) {
                    renumber(c, next);
                }
            }
        }
    }
    renumber(&mut data["document"], &mut 0);
    refresh_caption_fixture_outline(&mut data);
    let reference = NodeId::new(
        data["document"]["blocks"][0]["caption"][1]["children"][1]["node_id"]
            .as_u64()
            .unwrap() as u32,
    );
    for number in [false, true] {
        if number {
            data["document"]["blocks"][0]["caption"][1]["children"][1]["format"] = "number".into();
            data["document"]["number_bindings"] = serde_json::json!([{
                "anchor_id":"top","owner_node_id":2,"label_node_id":3,
                "text_span":{"text_id":0,"start_byte":0,"end_byte":5}
            }]);
        }
        let body = styled(&data, &limits());
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let mut flow = if number {
            prepare_book_v2_text_flow(&body, &nav).unwrap()
        } else {
            prepare_book_v2_text_flow_with_page_references(&body, &nav, &[(reference, 12)]).unwrap()
        };
        assert_eq!(flow.tables().len(), 2);
        assert!(flow.tables()[1].caption_event_range().is_none());
        if number {
            assert_eq!(flow.reference_text(reference), Some("Basic"));
        } else {
            assert_eq!(flow.page_reference_text(reference), Some("12"));
        }
        assert!(!flow.footnote_definitions().is_empty());
        flow.verify_for(&body, &nav).unwrap();
        let extent = flow.tables[0].caption_events.take().unwrap();
        assert_eq!(
            flow.verify_for(&body, &nav).unwrap_err().kind,
            ProductionFlowErrorKind::ReceiptMismatch
        );
        flow.tables[0].caption_events = Some(extent.start..extent.end - 1);
        assert_eq!(
            flow.verify_for(&body, &nav).unwrap_err().kind,
            ProductionFlowErrorKind::ReceiptMismatch
        );
        flow.tables[0].caption_events = Some(extent);
        flow.verify_for(&body, &nav).unwrap();
    }
}

#[test]
fn table_caption_metadata_consumes_the_shared_table_budget() {
    let mut data = input();
    let blocks = data["document"]["blocks"].as_array_mut().unwrap();
    let caption: Vec<_> = blocks.drain(..3).collect();
    blocks.iter_mut().find(|b| b["kind"] == "table").unwrap()["caption"] = caption.into();
    fn renumber(v: &mut Value, next: &mut u32) {
        if let Some(a) = v.as_array_mut() {
            for c in a {
                renumber(c, next);
            }
        } else if let Some(o) = v.as_object_mut() {
            if let Some(id) = o.get_mut("node_id") {
                *id = (*next).into();
                *next += 1;
            }
            for key in [
                "blocks",
                "children",
                "items",
                "caption",
                "head",
                "body",
                "cells",
                "equation_number",
                "footnotes",
            ] {
                if let Some(c) = o.get_mut(key) {
                    renumber(c, next);
                }
            }
        }
    }
    renumber(&mut data["document"], &mut 0);
    refresh_caption_fixture_outline(&mut data);
    for maximum in [24, 23] {
        let configured = ValidatedResourceLimits::new(ResourceLimits {
            max_fragments: maximum,
            ..ResourceLimits::default()
        })
        .unwrap();
        let body = styled(&data, &configured);
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let result = prepare_book_v2_text_flow(&body, &nav);
        if maximum == 24 {
            assert_eq!(result.unwrap().table_record_charge(), 24);
        } else {
            assert_eq!(
                result.err().unwrap().kind,
                ProductionFlowErrorKind::NodeLimit
            );
        }
    }
}

fn refresh_caption_fixture_outline(data: &mut Value) {
    fn owner(value: &Value, anchor: &str) -> Option<Value> {
        match value {
            Value::Array(a) => a.iter().find_map(|v| owner(v, anchor)),
            Value::Object(o) => {
                if o.get("anchor_id").and_then(Value::as_str) == Some(anchor) {
                    return o.get("node_id").cloned();
                }
                o.values().find_map(|v| owner(v, anchor))
            }
            _ => None,
        }
    }
    let document = data["document"].clone();
    for entry in data["outline"]["entries"].as_array_mut().unwrap() {
        entry["source_node_id"] = owner(&document, entry["destination"].as_str().unwrap()).unwrap();
    }
}
