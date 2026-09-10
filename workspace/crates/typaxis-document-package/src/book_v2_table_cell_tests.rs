use super::*;

#[test]
fn table_cell_classes_are_version_bound_optional_and_round_trip() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let source = root()["document"]["blocks"][0].clone();
    let span = &source["span"];
    let mut input = root();
    input["document"]["blocks"] = json!([{"kind":"table","node_id":1,"span":span,"classes":[],
        "columns":[{"kind":"fraction","weight":1}],"head":[],"body":[{"node_id":2,"span":span,
        "cells":[{"node_id":3,"span":span,"colspan":1,"rowspan":1,"blocks":source["blocks"]}]}]}]);
    let original = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .unwrap();
    let count = semantic_wire_ast_node_count(original.wire(), 16).unwrap();
    for classes in [json!([]), json!(["center"])] {
        let mut changed = input.clone();
        changed["document"]["blocks"][0]["body"][0]["cells"][0]["classes"] = classes;
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(&wire(&changed), &policy)
            .unwrap();
        assert_eq!(
            semantic_wire_ast_node_count(decoded.wire(), 16).unwrap(),
            count
        );
        let canonical = BookV2DocumentPackageEncoder::new()
            .encode(decoded.wire())
            .unwrap();
        assert_eq!(serde_json::from_str::<Value>(&canonical).unwrap(), changed);
        assert!(serde_json::from_value::<crate::WireStagingM4Block>(
            changed["document"]["blocks"][0].clone()
        )
        .is_err());
        changed["contract"] = "typaxis.contract/1.4".into();
        assert!(StagingSemanticDocumentPackageDecoder::new()
            .decode(&wire(&changed), &policy)
            .is_err());
    }
    for classes in [Value::Null, json!("center"), json!([1]), json!({})] {
        let mut bad = input.clone();
        bad["document"]["blocks"][0]["body"][0]["cells"][0]["classes"] = classes;
        assert!(BookV2DocumentPackageDecoder::new()
            .decode(&wire(&bad), &policy)
            .is_err());
    }
    let mut old: crate::WireStagingM4Block =
        serde_json::from_value(input["document"]["blocks"][0].clone()).unwrap();
    let crate::WireStagingM4Block::Table { body, .. } = &mut old else {
        panic!("table")
    };
    body[0].cells[0].classes = Some(vec![]);
    assert!(serde_json::to_value(old).is_err());
}
