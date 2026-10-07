use super::*;

fn numbered() -> Value {
    let mut data = root();
    data["document"]["number_bindings"] = json!([{
        "anchor_id":"result.one", "owner_node_id":1, "label_node_id":3,
        "text_span":{"text_id":0,"start_byte":0,"end_byte":1}
    }]);
    data
}

#[test]
fn number_binding_round_trips_and_remains_closed_to_frozen_contract() {
    let mut input = numbered();
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .unwrap();
    assert_eq!(
        serde_json::to_value(decoded.wire().document()).unwrap(),
        input["document"]
    );
    let output = BookV2DocumentPackageEncoder::new()
        .encode(decoded.wire())
        .unwrap();
    assert_eq!(output, decoded.canonical_jcs());
    assert!(StrictDocumentPackageDecoder::new()
        .decode(output.as_bytes(), &policy)
        .is_err());
    input["contract"] = "typaxis.contract/1.4".into();
    assert!(StagingSemanticDocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .is_err());
    assert!(
        serde_json::from_value::<crate::WireStagingM4Document>(input["document"].clone()).is_err()
    );
    let old = StagingSemanticDocumentPackageDecoder::new()
        .decode(FIXTURE, &policy)
        .unwrap();
    let mut document = old.wire().document().clone();
    document.number_bindings = decoded.wire().document().number_bindings.clone();
    assert!(serde_json::to_value(document).is_err());
}

#[test]
fn number_binding_rejects_null_empty_unknown_missing_and_wrong_types() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    for value in [Value::Null, json!([]), json!({}), json!([null])] {
        let mut data = numbered();
        data["document"]["number_bindings"] = value;
        assert!(BookV2DocumentPackageDecoder::new()
            .decode(&wire(&data), &policy)
            .is_err());
    }
    for key in ["anchor_id", "owner_node_id", "label_node_id", "text_span"] {
        let mut data = numbered();
        data["document"]["number_bindings"][0]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(BookV2DocumentPackageDecoder::new()
            .decode(&wire(&data), &policy)
            .is_err());
    }
    for (key, value) in [
        ("label", json!("fabricated")),
        ("owner_node_id", json!(-1)),
        ("label_node_id", json!(1.5)),
        ("text_span", Value::Null),
    ] {
        let mut data = numbered();
        data["document"]["number_bindings"][0][key] = value;
        assert!(BookV2DocumentPackageDecoder::new()
            .decode(&wire(&data), &policy)
            .is_err());
    }
    let duplicate = String::from_utf8(wire(&numbered())).unwrap().replace(
        "\"owner_node_id\":1",
        "\"owner_node_id\":1,\"owner_node_id\":1",
    );
    assert!(BookV2DocumentPackageDecoder::new()
        .decode(duplicate.as_bytes(), &policy)
        .is_err());
}

#[test]
fn number_binding_metadata_consumes_shared_ast_budget() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let plain = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&root()), &policy)
        .unwrap();
    let input = wire(&numbered());
    let numbered = BookV2DocumentPackageDecoder::new()
        .decode(&input, &policy)
        .unwrap();
    let count = semantic_wire_ast_node_count(numbered.wire(), 256).unwrap();
    assert_eq!(
        count,
        semantic_wire_ast_node_count(plain.wire(), 256).unwrap() + 2
    );
    for delta in [0, 1] {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_ast_nodes: count - delta,
            ..ResourceLimits::default()
        })
        .unwrap();
        let result = BookV2DocumentPackageDecoder::new()
            .decode(&input, &DocumentPackageDecodePolicy::new(&limits));
        assert_eq!(result.is_ok(), delta == 0);
    }
}
