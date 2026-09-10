use super::*;
use serde_json::{json, Value};
use typaxis_core::ResourceLimits;
use typaxis_document::book_v2::BookV2Block;
use typaxis_document_package::book_v2::{
    BookV2DocumentPackageDecoder, BOOK_V2_DOCUMENT_PACKAGE_CONTRACT,
};
use typaxis_document_package::DocumentPackageDecodePolicy;

const FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));
const MATH: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/math/job/document-package.json"
));
const VECTOR: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"));
fn root(fixture: &[u8]) -> Value {
    let mut root: Value = serde_json::from_slice(fixture).unwrap();
    root["contract"] = BOOK_V2_DOCUMENT_PACKAGE_CONTRACT.into();
    root
}
fn limits() -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits::default()).unwrap()
}
fn decode(input: &Value, limits: &ValidatedResourceLimits) -> DecodedBookV2DocumentPackage {
    BookV2DocumentPackageDecoder::new()
        .decode(
            &serde_json::to_vec(input).unwrap(),
            &DocumentPackageDecodePolicy::new(limits),
        )
        .unwrap()
}
fn prepare(input: &Value) -> Result<PreparedBookV2Body, StagingSemanticSyntaxError> {
    let limits = limits();
    prepare_book_v2_body(decode(input, &limits), &limits)
}

// Visit typed child slots, never JSON object/key order. This makes every matrix
// input independently meet the dense preorder required by syntax preparation.
fn renumber(value: &mut Value, next: &mut u32) {
    if let Some(array) = value.as_array_mut() {
        for child in array {
            renumber(child, next);
        }
    } else if let Some(object) = value.as_object_mut() {
        if let Some(id) = object.get_mut("node_id") {
            *id = (*next).into();
            *next += 1;
        }
        for key in [
            "term",
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
            if let Some(child) = object.get_mut(key) {
                renumber(child, next);
            }
        }
    }
}
fn collect(
    blocks: &[BookV2Block],
    output: &mut Vec<(u32, BookV2SemanticContainerKind, SourceSpan)>,
) {
    for block in blocks {
        if let Some(kind) = block.semantic_kind() {
            output.push((block.node_id().get(), kind, block.span()));
        }
        match block {
            BookV2Block::DescriptionList { items, .. } => {
                for item in items {
                    collect(&item.blocks, output);
                }
            }
            BookV2Block::List { items, .. } => {
                for item in items {
                    collect(&item.blocks, output);
                }
            }
            BookV2Block::Table { caption, head, body, .. } => {
                collect(caption, output);
                for row in head.iter().chain(body) {
                    for cell in &row.cells {
                        collect(&cell.blocks, output);
                    }
                }
            }
            _ => collect(block.direct_blocks(), output),
        }
    }
}
fn wire_containers(value: &Value, output: &mut Vec<(u32, String, Value)>) {
    match value {
        Value::Array(array) => {
            for child in array {
                wire_containers(child, output);
            }
        }
        Value::Object(object) => {
            if value["kind"] == "semantic_container" {
                output.push((
                    value["node_id"].as_u64().unwrap() as u32,
                    value["semantic_kind"].as_str().unwrap().into(),
                    value["span"].clone(),
                ));
            }
            for key in [
                "blocks",
                "children",
                "items",
                "head",
                "body",
                "cells",
                "caption",
                "footnotes",
            ] {
                if let Some(child) = object.get(key) {
                    wire_containers(child, output);
                }
            }
        }
        _ => {}
    }
}

#[test]
fn all_successor_kinds_preserve_typed_ownership_in_all_eight_recursive_slots() {
    for kind in WireBookV2SemanticContainerKind::ALL {
        for slot in [
            "body",
            "container",
            "list",
            "table-head",
            "table-body",
            "figure",
            "vector-figure",
            "footnote",
        ] {
            let mut input = root(FIXTURE);
            input["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({
                "style_id":"flow-paragraph", "selector":"paragraph", "source_order":2, "extends":null,
                "declarations":[
                    declaration("font_family",json!({"kind":"font_family_list","families":["Body"]}),false),
                    declaration("font_size",length_value(12 * 65536),false),
                    declaration("line_height",length_value(16 * 65536),false)
                ]
            }));
            let mut child = input["document"]["blocks"][0].clone();
            child["semantic_kind"] = kind.as_str().into();
            child["anchor_id"] = "authored.group".into();
            child["language"] = "ja".into();
            child["blocks"][1]["semantic_kind"] = "common_error".into();
            child["blocks"][2]["semantic_kind"] = "formalization_note".into();
            let span = child["span"].clone();
            let wrapped = match slot {
                "body" | "footnote" => child,
                "container" => {
                    json!({"kind":"semantic_container","semantic_kind":"example","anchor_id":null,"node_id":0,"span":span,"classes":[],"blocks":[child]})
                }
                "list" => {
                    json!({"kind":"list","ordered":false,"start":null,"node_id":0,"span":span,"classes":[],"items":[{"node_id":0,"span":span,"blocks":[child]}]})
                }
                "table-head" | "table-body" => {
                    let row = json!({"node_id":0,"span":span,"cells":[{"node_id":0,"span":span,"colspan":1,"rowspan":1,"blocks":[child]}]});
                    let mut table = json!({"kind":"table","node_id":0,"span":span,"classes":[],"columns":[{"kind":"fraction","weight":1}],"head":[],"body":[]});
                    table[if slot == "table-head" { "head" } else { "body" }] = json!([row]);
                    table
                }
                "figure" => {
                    json!({"kind":"figure","node_id":0,"span":span,"classes":[],"image_id":0,"placement":"block","alt":"diagram","caption":[child]})
                }
                "vector-figure" => {
                    json!({"kind":"vector_figure","node_id":0,"span":span,"classes":[],"image_id":0,"viewport":{"width":65536,"height":65536},"alt":"diagram","caption":[child]})
                }
                _ => unreachable!(),
            };
            if slot == "footnote" {
                input["document"]["blocks"] = json!([]);
                input["document"]["footnotes"] = json!([{"footnote_id":"fn.authored","node_id":0,"span":span,"blocks":[wrapped]}]);
            } else {
                input["document"]["blocks"] = json!([wrapped]);
            }
            renumber(&mut input["document"], &mut 0);
            let prepared =
                prepare(&input).unwrap_or_else(|error| panic!("{kind:?}/{slot}: {error}"));
            assert_eq!(
                serde_json::to_value(prepared.wire().document()).unwrap(),
                input["document"],
                "{kind:?}/{slot}"
            );
            let mut actual = Vec::new();
            collect(&prepared.document().blocks, &mut actual);
            for footnote in &prepared.document().footnotes {
                collect(&footnote.blocks, &mut actual);
            }
            let mut expected = Vec::new();
            wire_containers(&input["document"], &mut expected);
            assert_eq!(actual.len(), expected.len());
            for ((id, kind, span), (expected_id, expected_kind, expected_span)) in
                actual.iter().zip(&expected)
            {
                assert_eq!(*id, *expected_id);
                assert_eq!(kind.as_str(), expected_kind);
                let wire_span: WireStagingSourceSpan =
                    serde_json::from_value(expected_span.clone()).unwrap();
                assert_eq!(*span, lower_span(wire_span).unwrap());
            }
            assert!(actual
                .iter()
                .any(|(_, actual, _)| actual.as_str() == kind.as_str()));
            let styled = style_book_v2_body(prepared).unwrap();
            assert_eq!(styled.containers.len(), actual.len());
            let navigation = prepare_book_v2_navigation(&styled).unwrap();
            navigation.verify_for(&styled).unwrap();
            let flow = prepare_book_v2_text_flow(&styled, &navigation)
                .unwrap_or_else(|error| panic!("{kind:?}/{slot}: {error}"));
            flow.verify_for(&styled, &navigation).unwrap();
            for (node, kind, _) in &actual {
                assert_eq!(
                    flow.semantic_container_style(NodeId::new(*node))
                        .unwrap()
                        .semantic_kind()
                        .as_str(),
                    kind.as_str()
                );
                assert!(flow.events().iter().any(|event| matches!(event,
                    crate::ProductionFlowEvent::Begin { owner, kind: crate::ProductionFlowRegionKind::SemanticContainer }
                    if owner.get() == *node)));
            }
            for (node, kind, _) in &actual {
                assert_eq!(
                    navigation
                        .semantic_kind(NodeId::new(*node))
                        .unwrap()
                        .as_str(),
                    kind.as_str()
                );
                assert!(navigation.language(NodeId::new(*node)).is_some());
            }
            for (id, expected_kind, _) in actual {
                let style = styled.container_style(NodeId::new(id)).unwrap();
                assert_eq!(style.semantic_kind().as_str(), expected_kind.as_str());
            }
        }
    }
}

#[test]
fn successor_reuses_dense_node_span_class_and_utf8_checks() {
    let original = root(FIXTURE);
    for (pointer, value, expected) in [
        (
            "/document/blocks/0/node_id",
            json!(2),
            StagingSemanticSyntaxError::InvalidNodeOrder,
        ),
        (
            "/document/blocks/0/blocks/1/span/start_byte",
            json!(20),
            StagingSemanticSyntaxError::InvalidSourceSpan,
        ),
        (
            "/document/blocks/0/blocks/0/span/end_byte",
            json!(20),
            StagingSemanticSyntaxError::InvalidSourceSpan,
        ),
        (
            "/document/blocks/0/classes",
            json!(["z", "a"]),
            StagingSemanticSyntaxError::InvalidClass,
        ),
        (
            "/document/blocks/0/classes",
            json!(["a", "a"]),
            StagingSemanticSyntaxError::InvalidClass,
        ),
        (
            "/document/blocks/0/classes",
            json!(["__typaxis_internal_hidden"]),
            StagingSemanticSyntaxError::InvalidClass,
        ),
        (
            "/document/blocks/0/blocks/0/children/0/text_span/end_byte",
            json!(99),
            StagingSemanticSyntaxError::InvalidInline,
        ),
    ] {
        let mut input = original.clone();
        *input.pointer_mut(pointer).unwrap() = value;
        assert_eq!(prepare(&input).unwrap_err(), expected, "{pointer}");
    }
    let mut input = original;
    // Same byte length as Result; offset 1 now splits a UTF-8 scalar.
    input["text_buffers"][0]["utf8"] = "結果ProofExercise".into();
    input["document"]["blocks"][0]["blocks"][0]["children"][0]["text_span"]["start_byte"] =
        1.into();
    assert_eq!(
        prepare(&input).unwrap_err(),
        StagingSemanticSyntaxError::InvalidInline
    );
}

#[test]
fn native_math_keeps_exact_source_and_unstyled_owner() {
    let mut input = root(MATH);
    input["document"]["blocks"][0]["semantic_kind"] = "solution".into();
    let prepared = prepare(&input).unwrap();
    assert!(prepared.vectors().is_empty());
    assert_eq!(prepared.math().len(), 2);
    for (math, node, owner, source, speech) in [
        (&prepared.math()[0], 3, 2, "x^{2}", "x squared"),
        (&prepared.math()[1], 4, 1, "x+1", "x plus one"),
    ] {
        assert_eq!(math.domain().node_id, NodeId::new(node));
        assert_eq!(math.domain().owner_node_id, NodeId::new(owner));
        assert_eq!(math.domain().source, source);
        assert_eq!(math.domain().speech, speech);
    }
    assert_eq!(prepared.retained_text_bytes(), 8 + 9 + 10);
    input["text_buffers"][0]["mappings"][0]["source_span"]["start_byte"] = 1.into();
    assert_eq!(
        prepare(&input).unwrap_err(),
        StagingSemanticSyntaxError::InvalidSourceSpan
    );
}

#[test]
fn vectors_keep_use_specific_text_geometry_and_equation_number_without_old_receipts() {
    let mut input = root(VECTOR);
    input["document"]["blocks"][0]["semantic_kind"] = "example".into();
    let prepared = prepare(&input).unwrap();
    assert!(prepared.math().is_empty());
    assert_eq!(prepared.vectors().len(), 4);
    assert_eq!(
        prepared
            .vectors()
            .iter()
            .map(|vector| vector.node_id().get())
            .collect::<Vec<_>>(),
        [3, 4, 5, 6]
    );
    let inline = &prepared.vectors()[1];
    let block = &prepared.vectors()[3];
    assert_eq!(inline.kind(), PrecomposedVectorKind::MathVector);
    assert_eq!(block.kind(), PrecomposedVectorKind::MathVectorBlock);
    assert_eq!(
        inline.source_tex().unwrap().exact_text_sha256(),
        sha256(b"x+y")
    );
    assert_eq!(
        block.source_tex().unwrap().exact_text_sha256(),
        sha256(b"x+y")
    );
    assert_ne!(
        inline.source_tex().unwrap().mapped_source_span(),
        block.source_tex().unwrap().mapped_source_span()
    );
    assert_eq!(inline.alternative().resolved_actual_text(), Some("xたすy"));
    assert_eq!(
        block.alternative().resolved_actual_text(),
        Some("xたすy、式1")
    );
    let number = block.equation_number().unwrap();
    assert_eq!(number.node_id().get(), 7);
    assert_eq!(number.text().exact_text_sha256(), sha256(b"(1)"));
    input["text_buffers"][0]["mappings"][1]["source_span"]["start_byte"] = 2.into();
    assert_eq!(
        prepare(&input).unwrap_err(),
        StagingSemanticSyntaxError::InvalidSourceSpan
    );
}

#[test]
fn preparation_owns_input_hashes_and_exact_limits() {
    let mut input = root(FIXTURE);
    let limits = limits();
    let bytes = serde_json::to_vec(&input).unwrap();
    let decoded = decode(&input, &limits);
    let canonical = decoded.canonical_jcs_sha256();
    let prepared = prepare_book_v2_body(decoded, &limits).unwrap();
    input["document"]["blocks"][0]["semantic_kind"] = "quote".into();
    assert_eq!(
        prepared.document().blocks[0].semantic_kind(),
        Some(BookV2SemanticContainerKind::Result)
    );
    assert_eq!(prepared.raw_sha256(), sha256(&bytes));
    assert_eq!(prepared.canonical_jcs_sha256(), canonical);
    assert_eq!(prepared.limits(), &limits);
    let mut other = limits.get().clone();
    other.max_ast_nodes -= 1;
    let other = ValidatedResourceLimits::new(other).unwrap();
    assert_eq!(
        prepare_book_v2_body(decode(&input, &limits), &other).unwrap_err(),
        StagingSemanticSyntaxError::ReceiptMismatch
    );
}

#[test]
fn aggregate_text_budget_includes_native_speech_and_vector_alternatives() {
    for fixture in [MATH, VECTOR] {
        let input = root(fixture);
        let total = prepare(&input).unwrap().retained_text_bytes();
        for decrement in [0, 1] {
            let mut limits = ResourceLimits::default();
            limits.max_text_bytes = total - decrement;
            limits.max_text_buffer_bytes = (total - decrement) as u32;
            limits.max_shaping_context_bytes = limits.max_text_buffer_bytes;
            let limits = ValidatedResourceLimits::new(limits).unwrap();
            let result = prepare_book_v2_body(decode(&input, &limits), &limits);
            if decrement == 0 {
                assert_eq!(result.unwrap().retained_text_bytes(), total);
            } else {
                assert!(matches!(
                    result.unwrap_err(),
                    StagingSemanticSyntaxError::MathSpeechLimit
                        | StagingSemanticSyntaxError::PrecomposedVectorTextAggregateLimit { .. }
                ));
            }
        }
    }
}

#[test]
fn native_math_ast_nodes_share_the_body_budget() {
    let input = root(MATH);
    let prepared = prepare(&input).unwrap();
    let total = 5 + prepared
        .math()
        .iter()
        .map(|math| math.parsed().ast_node_count())
        .sum::<u64>();
    for decrement in [0, 1] {
        let mut limits = ResourceLimits::default();
        limits.max_ast_nodes = total - decrement;
        let limits = ValidatedResourceLimits::new(limits).unwrap();
        let result = prepare_book_v2_body(decode(&input, &limits), &limits);
        if decrement == 0 {
            result.unwrap();
        } else {
            assert_eq!(
                result.unwrap_err(),
                StagingSemanticSyntaxError::MathAstNodeLimit
            );
        }
    }
}

fn declaration(name: &str, value: Value, important: bool) -> Value {
    json!({"name":name,"value":value,"important":important})
}
fn length_value(raw: i64) -> Value {
    json!({"kind":"length","value":raw})
}

#[test]
fn successor_style_keeps_precedence_extends_and_authored_inheritance() {
    let mut input = root(FIXTURE);
    input["document"]["blocks"][0]["semantic_kind"] = "quote".into();
    input["document"]["blocks"][0]["blocks"][1]["semantic_kind"] = "solution".into();
    input["style_sheet"]["rules"][0]["declarations"][0]["important"] = true.into();
    let feature = input["style_sheet"]["rules"][1]["declarations"]
        .as_array_mut()
        .unwrap();
    feature.extend([
        declaration(
            "font_family",
            json!({"kind":"font_family_list","families":["Authored"]}),
            false,
        ),
        declaration("font_size", length_value(12 * 65536), false),
        declaration("line_height", length_value(16 * 65536), false),
        declaration(
            "text_align",
            json!({"kind":"keyword","value":"center"}),
            false,
        ),
    ]);
    input["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({
        "style_id":"nested-override","selector":"semantic_container.nested","source_order":2,"extends":"semantic-feature",
        "declarations":[declaration("space_before",length_value(11),false),declaration("text_align",json!({"kind":"keyword","value":"end"}),false)]
    }));
    let styled = style_book_v2_body(prepare(&input).unwrap()).unwrap();
    let root = styled.container_style(NodeId::new(1)).unwrap();
    let nested = styled.container_style(NodeId::new(4)).unwrap();
    let inherited = styled.container_style(NodeId::new(7)).unwrap();
    assert_eq!(root.semantic_kind().as_str(), "quote");
    assert_eq!(nested.semantic_kind().as_str(), "solution");
    for style in [root, nested, inherited] {
        assert_eq!(style.block_style().space_before().get().raw(), 2);
        assert_eq!(
            style.inheritance_style().font_families().unwrap(),
            ["Authored"]
        );
        assert_eq!(
            style.inheritance_style().font_size().unwrap().get().raw(),
            12 * 65536
        );
        assert_eq!(
            style.inheritance_style().line_height().unwrap().get().raw(),
            16 * 65536
        );
    }
    assert_eq!(
        root.block_style().text_align(),
        typaxis_style::MachineTextAlign::Center
    );
    assert_eq!(
        nested.block_style().text_align(),
        typaxis_style::MachineTextAlign::End
    );
    assert_eq!(
        inherited.block_style().text_align(),
        typaxis_style::MachineTextAlign::Center
    );
    let navigation = prepare_book_v2_navigation(&styled).unwrap();
    let flow = prepare_book_v2_text_flow(&styled, &navigation).unwrap();
    for paragraph in flow.paragraphs() {
        assert_eq!(paragraph.style().font_families().unwrap(), ["Authored"]);
        assert_eq!(
            paragraph.style().font_size().unwrap().get().raw(),
            12 * 65536
        );
        assert_eq!(
            paragraph.style().line_height().unwrap().get().raw(),
            16 * 65536
        );
        assert_eq!(
            paragraph.style().block_style().text_align(),
            if paragraph.owner() == NodeId::new(5) {
                typaxis_style::MachineTextAlign::End
            } else {
                typaxis_style::MachineTextAlign::Center
            }
        );
    }
}

#[test]
fn successor_styles_close_math_and_equation_number_without_reparsing_sources() {
    let mut native = root(MATH);
    native["document"]["blocks"][0]["semantic_kind"] = "formalization_note".into();
    let prepared = prepare(&native).unwrap();
    let source_ptr = prepared.math()[0].domain().source.as_ptr();
    let styled = style_book_v2_body(prepared).unwrap();
    assert_eq!(styled.body().math()[0].domain().source.as_ptr(), source_ptr);
    let inline = styled.math_style(NodeId::new(3)).unwrap();
    let display = styled.math_style(NodeId::new(4)).unwrap();
    assert_eq!(inline.font_families(), ["Math"]);
    assert_eq!(display.font_size().get().raw(), 12 * 65536);
    assert_eq!(display.block_style().start_indent().get().raw(), 4 * 65536);
    assert_eq!(
        display.block_style().text_align(),
        typaxis_style::MachineTextAlign::Center
    );

    let mut vectors = root(VECTOR);
    vectors["document"]["blocks"][0]["semantic_kind"] = "common_error".into();
    vectors["style_sheet"]["rules"] = json!([{
        "style_id":"parent-font","selector":"semantic_container","source_order":0,"extends":null,
        "declarations":[declaration("font_family",json!({"kind":"font_family_list","families":["Body"]}),false),declaration("font_size",length_value(10*65536),false),declaration("line_height",length_value(14*65536),false)]
    }]);
    let styled = style_book_v2_body(prepare(&vectors).unwrap()).unwrap();
    let number = styled
        .vector_style(NodeId::new(6))
        .unwrap()
        .equation_number_text_style()
        .unwrap();
    assert_eq!(number.font_families().unwrap(), ["Body"]);
    assert_eq!(number.font_size().unwrap().get().raw(), 10 * 65536);
    assert_eq!(number.line_height().unwrap().get().raw(), 14 * 65536);
    assert!(styled
        .vector_style(NodeId::new(5))
        .unwrap()
        .equation_number_text_style()
        .is_none());
    assert_eq!(
        styled.body().vectors()[3]
            .source_tex()
            .unwrap()
            .exact_text_sha256(),
        sha256(b"x+y")
    );
}

#[test]
fn successor_style_rejects_unknown_selectors_inapplicable_properties_and_bad_extends() {
    for (pointer, value) in [
        ("/style_sheet/rules/0/selector", json!("solution")),
        (
            "/style_sheet/rules/0/declarations/0/name",
            json!("unrecognized_property"),
        ),
        ("/style_sheet/rules/1/extends", json!("missing-style")),
        ("/style_sheet/rules/1/extends", json!("semantic-feature")),
    ] {
        let mut input = root(FIXTURE);
        *input.pointer_mut(pointer).unwrap() = value;
        // Some invalid selectors are already rejected by the carrier. Check
        // the shared style stage directly as well as the end-to-end boundary.
        let sheet: WireStagingStyleSheet =
            serde_json::from_value(input["style_sheet"].clone()).unwrap();
        assert!(
            lower_semantic_style_rules(&sheet, &limits()).is_err(),
            "{pointer}"
        );
        assert_style_input_rejected(&input);
    }
    let mut input = root(FIXTURE);
    input["style_sheet"]["rules"][0]["declarations"] =
        json!([declaration("width", length_value(65536), false)]);
    assert_style_input_rejected(&input);
}

fn assert_style_input_rejected(input: &Value) {
    let limits = limits();
    let bytes = serde_json::to_vec(input).unwrap();
    if let Ok(decoded) = BookV2DocumentPackageDecoder::new()
        .decode(&bytes, &DocumentPackageDecodePolicy::new(&limits))
    {
        let body = prepare_book_v2_body(decoded, &limits).unwrap();
        assert!(style_book_v2_body(body).is_err());
    }
}

#[path = "book_v2_description_domain_tests.rs"]
mod description_domains;

#[path = "book_v2_table_caption_tests.rs"]
mod table_captions;

#[path = "book_v2_table_cell_style_tests.rs"]
mod table_cell_styles;
