use super::*;

fn caption_root() -> Value {
    let mut input = root();
    let source = input["document"]["blocks"][0].clone();
    let mut title = source["blocks"][0].clone();
    title["classes"] = json!(["authored-table-title"]);
    title["language"] = "ja".into();
    title["children"] = json!([{"kind":"strong", "node_id":30,
        "span":title["span"], "children":title["children"], "language":"ja"}]);
    input["document"]["blocks"] = json!([{
        "kind":"table", "node_id":1, "span":source["span"], "classes":[],
        "columns":[{"kind":"fraction","weight":1}], "caption":[title], "head":[],
        "body":[{"node_id":40,"span":source["span"],"cells":[{
            "node_id":41,"span":source["span"],"rowspan":1,"colspan":1,
            "blocks":[source["blocks"][1],source["blocks"][2]]}]}]
    }]);
    input
}

#[test]
fn table_caption_round_trip_keeps_original_blocks_in_recursive_slots() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    for slot in [
        "body",
        "container",
        "list",
        "table-cell",
        "figure",
        "footnote",
        "table-caption",
    ] {
        let mut input = caption_root();
        let table = input["document"]["blocks"][0].clone();
        let span = table["span"].clone();
        let wrapped = match slot {
            "body" | "footnote" => table.clone(),
            "container" => {
                json!({"kind":"semantic_container", "semantic_kind":"note", "node_id":50,
                "span":span, "classes":[], "anchor_id":null, "blocks":[table]})
            }
            "list" => json!({"kind":"list", "node_id":50,"span":span,"classes":[],"ordered":false,
                "start":null,"items":[{"node_id":51,"span":span,"blocks":[table]}]}),
            "figure" => json!({"kind":"figure","node_id":50,"span":span,"classes":[],
                "image_id":0,"alt":"diagram","placement":"block","caption":[table]}),
            "table-cell" | "table-caption" => {
                let mut outer = table.clone();
                if slot == "table-cell" {
                    outer["body"][0]["cells"][0]["blocks"] = json!([table]);
                } else {
                    outer["caption"] = json!([table]);
                }
                outer
            }
            _ => unreachable!(),
        };
        if slot == "footnote" {
            input["document"]["blocks"] = json!([]);
            input["document"]["footnotes"] = json!([{"node_id":50,"span":span,
                "footnote_id":"table.note","blocks":[wrapped]}]);
        } else {
            input["document"]["blocks"] = json!([wrapped]);
        }
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(&wire(&input), &policy)
            .unwrap_or_else(|e| panic!("{slot}: {e}"));
        assert_eq!(
            serde_json::to_value(decoded.wire().document()).unwrap(),
            input["document"],
            "{slot}"
        );
        let canonical = BookV2DocumentPackageEncoder::new()
            .encode(decoded.wire())
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&canonical).unwrap(),
            input,
            "{slot}"
        );
        assert_eq!(sha256(canonical.as_bytes()), decoded.canonical_jcs_sha256());
    }
}

#[test]
fn table_caption_is_optional_but_closed_to_null_empty_unknown_and_legacy() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let input = caption_root();
    for replacement in [Value::Null, json!([]), json!({}), json!("caption")] {
        let mut bad = input.clone();
        bad["document"]["blocks"][0]["caption"] = replacement;
        assert!(BookV2DocumentPackageDecoder::new()
            .decode(&wire(&bad), &policy)
            .is_err());
    }
    for pointer in [
        "/document/blocks/0/caption/0",
        "/document/blocks/0/caption/0/children/0",
    ] {
        let mut bad = input.clone();
        bad.pointer_mut(pointer).unwrap()["unknown"] = true.into();
        assert!(BookV2DocumentPackageDecoder::new()
            .decode(&wire(&bad), &policy)
            .is_err());
    }
    let text = String::from_utf8(wire(&input)).unwrap();
    let duplicate = text.replacen("\"caption\":", "\"caption\":[],\"caption\":", 1);
    assert_ne!(text, duplicate);
    assert!(BookV2DocumentPackageDecoder::new()
        .decode(duplicate.as_bytes(), &policy)
        .is_err());
    assert!(StrictDocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .is_err());
    for value in [
        input["document"]["blocks"][0]["caption"].clone(),
        json!([]),
        Value::Null,
    ] {
        let mut old = input.clone();
        old["contract"] = "typaxis.contract/1.4".into();
        old["document"]["blocks"][0]["caption"] = value;
        assert!(serde_json::from_value::<crate::WireStagingM4Block>(
            old["document"]["blocks"][0].clone()
        )
        .is_err());
        assert!(StagingSemanticDocumentPackageDecoder::new()
            .decode(&wire(&old), &policy)
            .is_err());
    }
    let mut omitted = input.clone();
    omitted["document"]["blocks"][0]
        .as_object_mut()
        .unwrap()
        .remove("caption");
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&omitted), &policy)
        .unwrap();
    let canonical = BookV2DocumentPackageEncoder::new()
        .encode(decoded.wire())
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&canonical).unwrap(), omitted);
    let mut old: crate::WireStagingM4Block =
        serde_json::from_value(omitted["document"]["blocks"][0].clone()).unwrap();
    let crate::WireStagingM4Block::Table { caption, .. } = &mut old else {
        unreachable!()
    };
    *caption = Some(vec![]);
    assert!(serde_json::to_value(old).is_err());
}

#[test]
fn table_caption_nodes_and_inline_depth_share_the_document_limits() {
    let input = caption_root();
    let defaults = limits();
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&input), &DocumentPackageDecodePolicy::new(&defaults))
        .unwrap();
    assert_eq!(semantic_wire_ast_node_count(decoded.wire(), 7).unwrap(), 16);
    for (nodes, success) in [(16, true), (15, false)] {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_ast_nodes: nodes,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(
            BookV2DocumentPackageDecoder::new()
                .decode(&wire(&input), &DocumentPackageDecodePolicy::new(&limits))
                .is_ok(),
            success
        );
    }
    let mut deep = input.clone();
    for _ in 0..3 {
        let child = deep["document"]["blocks"][0]["caption"][0]["children"][0].clone();
        deep["document"]["blocks"][0]["caption"][0]["children"] = json!([{
            "kind":"emphasis","node_id":99,"span":child["span"],"children":[child]}]);
    }
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&deep), &DocumentPackageDecodePolicy::new(&defaults))
        .unwrap();
    assert_eq!(semantic_wire_ast_node_count(decoded.wire(), 8).unwrap(), 19);
    assert!(semantic_wire_ast_node_count(decoded.wire(), 7).is_err());
}

#[test]
fn table_caption_native_math_version_is_checked_on_decode_and_reencode() {
    let mut input = caption_root();
    let original =
        input["document"]["blocks"][0]["caption"][0]["children"][0]["children"][0].clone();
    input["document"]["blocks"][0]["caption"][0]["children"] = json!([{
        "kind":"inline_math","node_id":3,"span":original["span"],"speech":"original formula",
        "math_source":{"language":"typaxis-math","version":"1","text_span":original["text_span"]}}]);
    let defaults = limits();
    let policy = DocumentPackageDecodePolicy::new(&defaults);
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .unwrap();
    let mut package = decoded.into_wire();
    let mut document = package.document().clone();
    let WireBookV2Block::Table {
        caption: Some(caption),
        ..
    } = &mut document.blocks[0]
    else {
        unreachable!()
    };
    let WireBookV2Block::Paragraph { children, .. } = &mut caption[0] else {
        unreachable!()
    };
    let crate::WireStagingM4Inline::InlineMath { math_source, .. } = &mut children[0] else {
        unreachable!()
    };
    math_source.version = "999".into();
    package.replace_typed_regions(document, package.resources().clone());
    assert!(BookV2DocumentPackageEncoder::new()
        .encode(&package)
        .is_err());
    input["document"]["blocks"][0]["caption"][0]["children"][0]["math_source"]["version"] =
        "999".into();
    assert!(BookV2DocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .is_err());
}

#[test]
fn table_caption_vector_errors_retain_the_actual_nested_pointer() {
    fn paragraph(value: &Value) -> Option<&Value> {
        if value["kind"] == "paragraph"
            && value["children"]
                .as_array()
                .is_some_and(|children| children.iter().any(|child| child["kind"] == "math_vector"))
        {
            return Some(value);
        }
        match value {
            Value::Array(values) => values.iter().find_map(paragraph),
            Value::Object(values) => values.values().find_map(paragraph),
            _ => None,
        }
    }
    let mut input: Value = serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"))).unwrap();
    input["contract"] = BOOK_V2_DOCUMENT_PACKAGE_CONTRACT.into();
    let caption = paragraph(&input["document"]).unwrap().clone();
    let span = caption["span"].clone();
    input["document"]["blocks"] = json!([{"kind":"table","node_id":40,"span":span,"classes":[],
        "columns":[{"kind":"fraction","weight":1}],"caption":[caption],"head":[],"body":[{
            "node_id":41,"span":span,"cells":[{"node_id":42,"span":span,"rowspan":1,"colspan":1,"blocks":[]}]}]}]);
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .unwrap();
    assert_eq!(
        serde_json::to_value(decoded.wire().document()).unwrap(),
        input["document"]
    );
    let children = input["document"]["blocks"][0]["caption"][0]["children"]
        .as_array_mut()
        .unwrap();
    let index = children
        .iter()
        .position(|child| child["kind"] == "math_vector")
        .unwrap();
    children[index]["metrics"]["advance"] = (-1).into();
    let error = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .unwrap_err();
    let pointer = format!("/document/blocks/0/caption/0/children/{index}/metrics/advance");
    assert_eq!(error.pointer(), Some(pointer.as_str()));
    let mut package = decoded.into_wire();
    let document = serde_json::from_value(input["document"].clone()).unwrap();
    package.replace_typed_regions(document, package.resources().clone());
    assert_eq!(
        BookV2DocumentPackageEncoder::new()
            .encode(&package)
            .unwrap_err()
            .pointer(),
        Some(pointer.as_str())
    );
}

#[test]
fn table_caption_rechecks_nested_semantic_shape_and_typed_empty_caption() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let input = caption_root();
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .unwrap();
    for empty_caption in [true, false] {
        let mut bad = input.clone();
        bad["document"]["blocks"][0]["caption"] = if empty_caption {
            json!([])
        } else {
            json!([{"kind":"semantic_container","semantic_kind":"note","node_id":2,
                "span":input["document"]["blocks"][0]["span"],"classes":[],"anchor_id":null,"blocks":[]}])
        };
        assert!(BookV2DocumentPackageDecoder::new()
            .decode(&wire(&bad), &policy)
            .is_err());
        let mut package = decoded.wire().clone();
        let mut document = package.document().clone();
        let WireBookV2Block::Table { caption, .. } = &mut document.blocks[0] else {
            unreachable!()
        };
        // Construct the typed value directly to test the encoder's own guard.
        *caption =
            Some(serde_json::from_value(bad["document"]["blocks"][0]["caption"].clone()).unwrap());
        package.replace_typed_regions(document, package.resources().clone());
        assert!(BookV2DocumentPackageEncoder::new()
            .encode(&package)
            .is_err());
    }
}
