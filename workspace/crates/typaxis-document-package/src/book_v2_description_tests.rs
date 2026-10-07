use super::*;

fn description_root() -> Value {
    let mut input = root();
    let source = input["document"]["blocks"][0].clone();
    let mut term = source["blocks"][0].clone();
    term.as_object_mut().unwrap().remove("kind");
    term["language"] = "ja".into();
    let text = term["children"][0].clone();
    term["children"] = json!([{
        "kind":"strong", "node_id":30, "span":term["span"],
        "children":[text], "language":"ja"
    }]);
    input["document"]["blocks"] = json!([{
        "kind":"description_list", "node_id":1, "span":source["span"],
        "classes":["authored-terms"], "language":"ja", "items":[{
            "node_id":20, "span":source["span"], "term":term,
            "blocks":[source["blocks"][1],source["blocks"][2]], "language":"ja"
        }]
    }]);
    input
}

#[test]
fn description_list_round_trip_retains_term_owners_and_all_recursive_slots() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    for slot in [
        "body",
        "container",
        "list",
        "description",
        "head",
        "body-cell",
        "figure",
        "vector-figure",
        "footnote",
    ] {
        let mut input = description_root();
        let child = input["document"]["blocks"][0].clone();
        let span = child["span"].clone();
        let wrapped = match slot {
            "body" | "footnote" => child.clone(),
            "container" => {
                json!({"kind":"semantic_container","semantic_kind":"solution","anchor_id":null,"node_id":40,"span":span,"classes":[],"blocks":[child]})
            }
            "list" => {
                json!({"kind":"list","node_id":40,"span":span,"classes":[],"ordered":false,"start":null,"items":[{"node_id":41,"span":span,"blocks":[child]}]})
            }
            "description" => {
                let mut outer = child.clone();
                outer["items"][0]["blocks"] = json!([child]);
                outer
            }
            "head" | "body-cell" => {
                let row = json!({"node_id":41,"span":span,"cells":[{"node_id":42,"span":span,"rowspan":1,"colspan":1,"blocks":[child]}]});
                let mut table = json!({"kind":"table","node_id":40,"span":span,"classes":[],"columns":[{"kind":"fraction","weight":1}],"head":[],"body":[]});
                table[if slot == "head" { "head" } else { "body" }] = json!([row]);
                table
            }
            "figure" => {
                json!({"kind":"figure","node_id":40,"span":span,"classes":[],"image_id":0,"placement":"block","alt":"diagram","caption":[child]})
            }
            "vector-figure" => {
                json!({"kind":"vector_figure","node_id":40,"span":span,"classes":[],"image_id":0,"viewport":{"width":65536,"height":65536},"alt":"diagram","caption":[child]})
            }
            _ => unreachable!(),
        };
        if slot == "footnote" {
            input["document"]["blocks"] = json!([]);
            input["document"]["footnotes"] = json!([{"node_id":40,"span":span,"footnote_id":"authored.note","blocks":[wrapped]}]);
        } else {
            input["document"]["blocks"] = json!([wrapped]);
        }
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(&wire(&input), &policy)
            .unwrap_or_else(|error| panic!("{slot}: {error}"));
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
        assert_eq!(decoded.canonical_jcs_sha256(), sha256(canonical.as_bytes()));
        assert_eq!(
            BookV2DocumentPackageDecoder::new()
                .decode(canonical.as_bytes(), &policy)
                .unwrap()
                .canonical_jcs(),
            canonical
        );
    }
}

#[test]
fn description_list_is_closed_to_legacy_wire_and_public_dispatch() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let mut input = description_root();
    assert!(StrictDocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .is_err());
    for empty in [false, true] {
        let mut block = input["document"]["blocks"][0].clone();
        if empty {
            block["items"] = json!([]);
        }
        assert!(serde_json::from_value::<crate::WireStagingM4Block>(block.clone()).is_err());
        input["document"]["blocks"][0] = block;
        input["contract"] = "typaxis.contract/1.4".into();
        assert!(StagingSemanticDocumentPackageDecoder::new()
            .decode(&wire(&input), &policy)
            .is_err());
    }
    // The shared generic enum is compiled for staging tests, but constructing
    // its successor variant cannot grant the old DTO serialization authority.
    let old = crate::WireStagingM4Block::DescriptionList {
        node_id: 1,
        span: serde_json::from_value(input["document"]["blocks"][0]["span"].clone()).unwrap(),
        classes: vec![],
        items: vec![],
        language: None,
    };
    assert!(serde_json::to_value(old).is_err());
}

#[test]
fn description_list_rejects_missing_null_unknown_duplicate_and_empty_members() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    for (pointer, field) in [
        ("/document/blocks/0", "items"),
        ("/document/blocks/0/items/0", "term"),
        ("/document/blocks/0/items/0", "blocks"),
        ("/document/blocks/0/items/0/term", "node_id"),
        ("/document/blocks/0/items/0/term", "span"),
        ("/document/blocks/0/items/0/term", "classes"),
        ("/document/blocks/0/items/0/term", "children"),
    ] {
        for replacement in [None, Some(Value::Null)] {
            let mut input = description_root();
            let object = input.pointer_mut(pointer).unwrap().as_object_mut().unwrap();
            if let Some(value) = replacement {
                object.insert(field.into(), value);
            } else {
                object.remove(field);
            }
            assert!(
                BookV2DocumentPackageDecoder::new()
                    .decode(&wire(&input), &policy)
                    .is_err(),
                "{pointer}/{field}"
            );
        }
    }
    for pointer in [
        "/document/blocks/0/items",
        "/document/blocks/0/items/0/blocks",
        "/document/blocks/0/items/0/term/children",
    ] {
        let mut input = description_root();
        *input.pointer_mut(pointer).unwrap() = json!([]);
        assert!(
            BookV2DocumentPackageDecoder::new()
                .decode(&wire(&input), &policy)
                .is_err(),
            "{pointer}"
        );
    }
    for (pointer, field, value) in [
        ("/document/blocks/0", "ordered", json!(false)),
        ("/document/blocks/0", "start", Value::Null),
        ("/document/blocks/0", "label", json!("bullet")),
        ("/document/blocks/0/items/0", "extra", json!(true)),
        (
            "/document/blocks/0/items/0/term",
            "kind",
            json!("paragraph"),
        ),
        (
            "/document/blocks/0/items/0/term",
            "text",
            json!("flattened"),
        ),
        ("/document/blocks/0/items/0/term", "language", Value::Null),
    ] {
        let mut input = description_root();
        input
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.into(), value);
        assert!(
            BookV2DocumentPackageDecoder::new()
                .decode(&wire(&input), &policy)
                .is_err(),
            "{pointer}/{field}"
        );
    }
    let raw = String::from_utf8(wire(&description_root())).unwrap();
    let duplicate = raw.replacen("\"term\":", "\"term\":null,\"term\":", 1);
    assert_ne!(raw, duplicate);
    assert!(BookV2DocumentPackageDecoder::new()
        .decode(duplicate.as_bytes(), &policy)
        .is_err());
}

#[test]
fn description_term_and_nested_inlines_are_charged_at_their_actual_depth() {
    let input = description_root();
    let bytes = wire(&input);
    let policy_limits = limits();
    let decoded = BookV2DocumentPackageDecoder::new()
        .decode(&bytes, &DocumentPackageDecodePolicy::new(&policy_limits))
        .unwrap();
    // Document, list, item, term, strong, text, two three-node containers,
    // and the unchanged metadata/outline root charges.
    assert_eq!(semantic_wire_ast_node_count(decoded.wire(), 6).unwrap(), 14);
    assert!(semantic_wire_ast_node_count(decoded.wire(), 5).is_err());
    for (nodes, depth, success) in [(14, 6, true), (13, 6, false), (14, 5, false)] {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_ast_nodes: nodes,
            max_ast_nesting_depth: depth,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(
            BookV2DocumentPackageDecoder::new()
                .decode(&bytes, &DocumentPackageDecodePolicy::new(&limits))
                .is_ok(),
            success
        );
    }
    for field in ["items", "term", "blocks"] {
        let mut package = decoded.wire().clone();
        let mut document = package.document().clone();
        let WireBookV2Block::DescriptionList { items, .. } = &mut document.blocks[0] else {
            unreachable!()
        };
        match field {
            "items" => items.clear(),
            "term" => items[0].term.children.clear(),
            "blocks" => items[0].blocks.clear(),
            _ => unreachable!(),
        }
        package.replace_typed_regions(document, package.resources().clone());
        assert!(
            BookV2DocumentPackageEncoder::new()
                .encode(&package)
                .is_err(),
            "{field}"
        );
    }
}

#[test]
fn description_terms_keep_native_and_precomposed_math_and_validate_their_fields() {
    fn paragraph(value: &Value) -> Option<&Value> {
        if value["kind"] == "paragraph"
            && value["children"].as_array().is_some_and(|a| !a.is_empty())
        {
            return Some(value);
        }
        match value {
            Value::Object(object) => object.values().find_map(paragraph),
            Value::Array(array) => array.iter().find_map(paragraph),
            _ => None,
        }
    }
    let cases: &[(&[u8], &str)] = &[
        (include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/math/job/document-package.json")), "inline_math"),
        (include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json")), "math_vector"),
    ];
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    for (fixture, expected) in cases {
        let mut input: Value = serde_json::from_slice(fixture).unwrap();
        input["contract"] = BOOK_V2_DOCUMENT_PACKAGE_CONTRACT.into();
        let definition = paragraph(&input["document"]).unwrap().clone();
        let mut term = definition.clone();
        term.as_object_mut().unwrap().remove("kind");
        assert!(term["children"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["kind"] == *expected));
        let span = term["span"].clone();
        input["document"]["blocks"] = json!([{
            "kind":"description_list", "node_id":40, "span":span, "classes":[],
            "items":[{"node_id":41,"span":span,"term":term,"blocks":[definition]}]
        }]);
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(&wire(&input), &policy)
            .unwrap();
        assert_eq!(
            serde_json::to_value(decoded.wire().document()).unwrap(),
            input["document"]
        );
        assert_eq!(
            serde_json::from_str::<Value>(
                &BookV2DocumentPackageEncoder::new()
                    .encode(decoded.wire())
                    .unwrap()
            )
            .unwrap(),
            input
        );
        let children = input["document"]["blocks"][0]["items"][0]["term"]["children"]
            .as_array_mut()
            .unwrap();
        let inline = children
            .iter_mut()
            .find(|i| i["kind"] == *expected)
            .unwrap();
        if *expected == "inline_math" {
            inline["math_source"]["version"] = "unknown".into();
        } else {
            inline["metrics"]["advance"] = (-1).into();
        }
        let error = BookV2DocumentPackageDecoder::new()
            .decode(&wire(&input), &policy)
            .unwrap_err();
        if *expected == "math_vector" {
            assert_eq!(
                error.pointer(),
                Some("/document/blocks/0/items/0/term/children/1/metrics/advance")
            );
        }
        let mut package = decoded.into_wire();
        let document: WireBookV2Document =
            serde_json::from_value(input["document"].clone()).unwrap();
        package.replace_typed_regions(document, package.resources().clone());
        assert!(BookV2DocumentPackageEncoder::new()
            .encode(&package)
            .is_err());
    }
}

#[test]
fn description_list_does_not_extend_header_footer_grammar() {
    let mut input = description_root();
    let list = input["document"]["blocks"][0].clone();
    let paragraph = root()["document"]["blocks"][0]["blocks"][0].clone();
    input["page_masters"]["masters"][0]["header"] = json!({"x":10,"y":0,"width":80,"height":8});
    input["page_masters"]["masters"][0]["header_content"] = json!({
        "node_id":70, "span":list["span"], "blocks":[paragraph]
    });
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    BookV2DocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .unwrap();
    input["page_masters"]["masters"][0]["header_content"]["blocks"] = json!([list]);
    assert!(BookV2DocumentPackageDecoder::new()
        .decode(&wire(&input), &policy)
        .is_err());
}
