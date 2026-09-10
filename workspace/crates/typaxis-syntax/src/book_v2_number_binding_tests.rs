use super::*;
use crate::BookNavigationSyntaxErrorKind;
use serde_json::json;

fn numbered() -> Value {
    let mut data = input();
    let text = "定理1.12（補足）";
    data["text_buffers"][0]["utf8"] = text.into();
    data["text_buffers"][0]["mappings"][0]["text_range"]["end_byte"] = text.len().into();
    data["document"]["blocks"][0]["children"][0]["text_span"]["end_byte"] = text.len().into();
    data["document"]["blocks"][1]["children"][1]["format"] = "number".into();
    data["document"]["number_bindings"] = json!([{
        "anchor_id":"top", "owner_node_id":1, "label_node_id":2,
        "text_span":{"text_id":0,"start_byte":6,"end_byte":10}
    }]);
    data
}

#[test]
fn number_reference_uses_exact_displayed_range_and_counter_provenance() {
    let input = numbered();
    let body = styled(&input, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let binding = &nav.number_bindings()[0];
    assert_eq!(binding.owner(), NodeId::new(1));
    assert_eq!(binding.label_owner(), NodeId::new(2));
    assert_eq!(binding.label(), "1.12");
    assert_eq!(
        binding.label().as_ptr(),
        body.body().wire().text_buffers()[0].utf8[6..].as_ptr()
    );
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let owner = NodeId::new(7);
    assert_eq!(flow.reference_text(owner), Some("1.12"));
    assert!(flow.page_reference_text(owner).is_none());
    assert!(flow.generated.buffer(text_reference_key(owner)).is_some());
    assert!(flow.reference_provenance(owner).is_some());
    assert!(matches!(
        flow.paragraphs()[1].items()[1].reference(),
        Some(ProductionInlineReference::Anchor {
            format: ProductionReferenceFormat::Number,
            ..
        })
    ));
    flow.verify_for(&body, &nav).unwrap();
    assert_eq!(
        serde_json::to_value(body.body().wire().document()).unwrap(),
        input["document"]
    );
}

#[test]
fn number_reference_requires_binding_even_when_target_has_outline_label() {
    let mut input = numbered();
    input["document"]
        .as_object_mut()
        .unwrap()
        .remove("number_bindings");
    let body = styled(&input, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    assert!(!nav.outline().is_empty());
    assert_eq!(
        prepare_book_v2_text_flow(&body, &nav).err().unwrap().kind,
        ProductionFlowErrorKind::MissingReferenceLabel
    );
}

#[test]
fn number_binding_rejects_wrong_owner_source_range_and_duplicate() {
    for (path, value) in [
        ("/owner_node_id", json!(0)),
        ("/owner_node_id", json!(2)),
        ("/owner_node_id", json!(5)),
        ("/label_node_id", json!(1)),
        ("/label_node_id", json!(3)),
        ("/label_node_id", json!(u32::MAX)),
        ("/anchor_id", json!("links")),
        ("/anchor_id", json!("")),
        ("/text_span/text_id", json!(1)),
        ("/text_span/start_byte", json!(1)),
        ("/text_span/start_byte", json!(10)),
        ("/text_span/end_byte", json!(11)),
        ("/text_span/end_byte", json!(1000)),
    ] {
        let mut input = numbered();
        *input["document"]["number_bindings"][0]
            .pointer_mut(path)
            .unwrap() = value;
        let body = styled(&input, &limits());
        assert_eq!(
            prepare_book_v2_navigation(&body).err().unwrap().kind(),
            BookNavigationSyntaxErrorKind::InvalidNumberBinding,
            "{path}"
        );
    }
    let mut input = numbered();
    let duplicate = input["document"]["number_bindings"][0].clone();
    input["document"]["number_bindings"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    let body = styled(&input, &limits());
    assert_eq!(
        prepare_book_v2_navigation(&body).err().unwrap().kind(),
        BookNavigationSyntaxErrorKind::InvalidNumberBinding
    );
}

#[test]
fn equation_binding_declares_owner_anchor_and_selects_number_without_parentheses() {
    let mut input: Value = serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"))).unwrap();
    input["contract"] = "typaxis.contract/1.5".into();
    input["document"]["number_bindings"] = json!([{
        "anchor_id":"equation.one", "owner_node_id":6, "label_node_id":7,
        "text_span":{"text_id":0,"start_byte":11,"end_byte":12}
    }]);
    let body = styled(&input, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    assert_eq!(nav.reference_number("equation.one"), Some("1"));
    assert!(nav
        .anchors()
        .iter()
        .any(|(a, n)| a.as_str() == "equation.one" && *n == NodeId::new(6)));
}

#[test]
fn number_generated_copy_shares_exact_text_budget() {
    let input = numbered();
    let body = styled(&input, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
    let total = nav.retained_text_bytes() + flow.generated_text_bytes();
    for (bound, success) in [(total, true), (total - 1, false)] {
        let mut limits = ResourceLimits::default();
        limits.max_text_bytes = bound;
        limits.max_text_buffer_bytes = bound as u32;
        limits.max_shaping_context_bytes = bound as u32;
        let limits = ValidatedResourceLimits::new(limits).unwrap();
        let body = styled(&input, &limits);
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let result = prepare_book_v2_text_flow(&body, &nav);
        if success {
            result.unwrap().verify_for(&body, &nav).unwrap();
        } else {
            assert_eq!(
                result.err().unwrap().kind,
                ProductionFlowErrorKind::TextLimit
            );
        }
    }
}
