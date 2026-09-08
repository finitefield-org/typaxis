use super::*;
use crate::book_v2::{prepare_book_v2_body, style_book_v2_body};
use serde_json::{json, Value};
use typaxis_core::ResourceLimits;
use typaxis_document_package::book_v2::{
    BookV2DocumentPackageDecoder, WireBookV2SemanticContainerKind,
};
use typaxis_document_package::DocumentPackageDecodePolicy;
const FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));
const VECTOR: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"));
fn root(bytes: &[u8]) -> Value {
    let mut input: Value = serde_json::from_slice(bytes).unwrap();
    input["contract"] = "typaxis.contract/1.5".into();
    input
}
fn limits() -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits::default()).unwrap()
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
            "blocks",
            "children",
            "items",
            "head",
            "body",
            "cells",
            "caption",
            "equation_number",
            "footnotes",
        ] {
            if let Some(child) = object.get_mut(key) {
                renumber(child, next);
            }
        }
    }
}
fn outline_input() -> Value {
    let mut input = root(FIXTURE);
    input["document"]["blocks"][0]["anchor_id"] = "group.root".into();
    input["document"]["blocks"][0]["blocks"][1]["anchor_id"] = "group.child".into();
    input["outline"]["entries"] = json!([
        {"outline_id":0,"parent_outline_id":null,"level":1,"destination":"group.root","label":"Root","source_node_id":1,"source_kind":"semantic_container"},
        {"outline_id":1,"parent_outline_id":0,"level":2,"destination":"group.child","label":"Solution","source_node_id":4,"source_kind":"semantic_container"}
    ]);
    input
}
#[test]
fn every_successor_kind_keeps_outline_source_anchor_and_typed_kind() {
    for kind in WireBookV2SemanticContainerKind::ALL {
        let mut input = outline_input();
        input["document"]["blocks"][0]["semantic_kind"] = kind.as_str().into();
        input["document"]["blocks"][0]["blocks"][1]["semantic_kind"] = "solution".into();
        let body = styled(&input, &limits());
        let nav = prepare_book_v2_navigation(&body).unwrap();
        assert_eq!(nav.outline().len(), 2);
        assert_eq!(
            nav.outline()[0].source.semantic_kind.as_deref(),
            Some(kind.as_str())
        );
        assert_eq!(
            nav.semantic_kind(NodeId::new(1)).unwrap().as_str(),
            kind.as_str()
        );
        assert_eq!(
            nav.outline()[1].source.semantic_kind.as_deref(),
            Some("solution")
        );
        assert_eq!(nav.outline()[1].parent_outline_id, Some(0));
        assert_eq!(
            nav.outline()[0].source.source_span,
            body.body().document().blocks[0].span()
        );
        assert_eq!(nav.outline()[0].destination.as_str(), "group.root");
        assert_eq!(nav.anchors().len(), 2);
    }
}
#[test]
fn languages_follow_logical_parents_and_canonicalize_explicit_spelling() {
    let mut input = root(FIXTURE);
    input["document"]["blocks"][0]["language"] = "JA-jp".into();
    input["document"]["blocks"][0]["blocks"][0]["language"] = "en-us".into();
    let body = styled(&input, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    assert_eq!(
        nav.language(NodeId::new(0)).unwrap().effective_language(),
        "und"
    );
    assert_eq!(
        nav.language(NodeId::new(1)).unwrap().explicit_language(),
        Some("ja-JP")
    );
    assert_eq!(
        nav.language(NodeId::new(3)).unwrap().effective_language(),
        "en-US"
    );
    assert_eq!(
        nav.language(NodeId::new(4)).unwrap().parent(),
        Some(NodeId::new(1))
    );
    assert_eq!(
        nav.language(NodeId::new(6)).unwrap().effective_language(),
        "ja-JP"
    );
    let other = styled(&input, &limits());
    assert_eq!(
        nav.verify_for(&other).unwrap_err().kind(),
        BookNavigationSyntaxErrorKind::ReceiptMismatch
    );
    nav.verify_for(&body).unwrap();
}
#[test]
fn vector_language_joins_exact_facts_and_equation_child_without_double_charge() {
    let mut input = root(VECTOR);
    input["document"]["blocks"][0]["semantic_kind"] = "example".into();
    input["document"]["blocks"][0]["blocks"][0]["children"][1]["language"] = "EN-us".into();
    let body = styled(&input, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let math = nav.language(NodeId::new(4)).unwrap();
    assert_eq!(math.effective_language(), "en-US");
    assert!(std::ptr::eq(
        math.vector().unwrap(),
        &body.body().vectors()[1]
    ));
    assert_eq!(
        math.vector()
            .unwrap()
            .source_tex()
            .unwrap()
            .exact_text_sha256(),
        sha256(b"x+y")
    );
    assert_eq!(nav.language_children().len(), 1);
    assert_eq!(nav.language_children()[0].node_id(), NodeId::new(7));
    assert_eq!(nav.language_children()[0].parent(), NodeId::new(6));
    assert_eq!(nav.language_children()[0].effective_language(), "ja");
    assert!(nav.language(NodeId::new(7)).is_none());
    // Six non-prepaid owners each retain the two-byte inherited language ja.
    assert_eq!(
        nav.retained_text_bytes(),
        body.body().retained_text_bytes() + 12
    );
}
fn reference_input() -> Value {
    let mut input = root(FIXTURE);
    input["document"]["blocks"][0]["anchor_id"] = "section".into();
    let span = json!({"source_id":0,"start_byte":6,"end_byte":6});
    let link_span = json!({"source_id":0,"start_byte":6,"end_byte":11});
    let tail = json!({"source_id":0,"start_byte":11,"end_byte":11});
    let original = input["document"]["blocks"][0]["blocks"][0]["children"][0].clone();
    input["document"]["blocks"][0]["blocks"][0]["span"]["end_byte"] = 19.into();
    input["document"]["blocks"][0]["blocks"][0]["children"] = json!([
        original,
        {"kind":"reference","node_id":0,"span":span,"target":"inline.dest","format":"text"},
        {"kind":"reference","node_id":0,"span":span,"target":"inline.dest","format":"page"},
        {"kind":"reference","node_id":0,"span":span,"target":"inline.dest","format":"number"},
        {"kind":"link","node_id":0,"span":link_span,"target":{"kind":"internal","anchor_id":"section"},"children":[{"kind":"text","node_id":0,"span":link_span,"text_span":{"text_id":0,"start_byte":6,"end_byte":11}}]},
        {"kind":"footnote_reference","node_id":0,"span":tail,"footnote_id":"fn.x"},
        {"kind":"anchor","node_id":0,"span":tail,"anchor_id":"inline.dest"}
    ]);
    input["document"]["footnotes"] = json!([{"footnote_id":"fn.x","node_id":0,"span":{"source_id":0,"start_byte":11,"end_byte":19},"blocks":[input["document"]["blocks"][0]["blocks"][2]["blocks"][0].clone()]}]);
    renumber(&mut input["document"], &mut 0);
    input
}
#[test]
fn forward_references_links_and_footnotes_keep_explicit_target_and_format() {
    let input = reference_input();
    let body = styled(&input, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    assert_eq!(nav.references().len(), 4);
    assert_eq!(nav.internal_links().len(), 1);
    for (entry, format) in nav.references().iter().zip([
        WireStagingM4ReferenceFormat::Text,
        WireStagingM4ReferenceFormat::Page,
        WireStagingM4ReferenceFormat::Number,
    ]) {
        assert_eq!(
            entry.1,
            BookV2ReferenceTarget::Anchor {
                target: AnchorId::new("inline.dest").unwrap(),
                format
            }
        );
    }
    assert!(
        matches!(&nav.references()[3].1,BookV2ReferenceTarget::Footnote { id } if id.as_str()=="fn.x")
    );
    assert_eq!(nav.internal_links()[0].1.as_str(), "section");
    for pointer in [
        "/document/blocks/0/blocks/0/children/1/target",
        "/document/blocks/0/blocks/0/children/4/target/anchor_id",
        "/document/blocks/0/blocks/0/children/5/footnote_id",
    ] {
        let mut bad = input.clone();
        *bad.pointer_mut(pointer).unwrap() = "missing".into();
        let body = styled(&bad, &limits());
        assert_eq!(
            prepare_book_v2_navigation(&body).unwrap_err().kind(),
            BookNavigationSyntaxErrorKind::InvalidOutline
        );
    }
}
#[test]
fn duplicate_anchors_invalid_language_and_outline_parent_are_rejected() {
    for (pointer, value, kind) in [
        (
            "/document/blocks/0/blocks/1/anchor_id",
            json!("group.root"),
            BookNavigationSyntaxErrorKind::DuplicateAnchor,
        ),
        (
            "/document/blocks/0/language",
            json!("en--US"),
            BookNavigationSyntaxErrorKind::InvalidLanguage,
        ),
        (
            "/outline/entries/1/parent_outline_id",
            json!(null),
            BookNavigationSyntaxErrorKind::InvalidOutline,
        ),
        (
            "/outline/entries/1/destination",
            json!("missing"),
            BookNavigationSyntaxErrorKind::InvalidOutline,
        ),
    ] {
        let mut input = outline_input();
        if pointer.ends_with("/language") {
            input["document"]["blocks"][0]["language"] = value;
        } else {
            *input.pointer_mut(pointer).unwrap() = value;
        }
        let body = styled(&input, &limits());
        assert_eq!(
            prepare_book_v2_navigation(&body).unwrap_err().kind(),
            kind,
            "{pointer}"
        );
    }
}
#[test]
fn exact_navigation_text_budget_and_one_over_use_the_body_ledger() {
    let input = root(FIXTURE);
    for maximum in [49, 48] {
        let mut raw = ResourceLimits::default();
        raw.max_text_bytes = maximum;
        raw.max_text_buffer_bytes = maximum as u32;
        raw.max_shaping_context_bytes = maximum as u32;
        let limits = ValidatedResourceLimits::new(raw).unwrap();
        let body = styled(&input, &limits);
        let result = prepare_book_v2_navigation(&body);
        if maximum == 49 {
            assert_eq!(result.unwrap().retained_text_bytes(), 49);
        } else {
            assert_eq!(
                result.unwrap_err().kind(),
                BookNavigationSyntaxErrorKind::TextAggregateLimit
            );
        }
    }
}

const NAVIGATION: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/book-navigation/job/document-package.json"));
const MATH: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/math/job/document-package.json"
));
#[test]
fn metadata_heading_outline_and_page_region_languages_keep_source_ownership() {
    let bytes =
        typaxis_document_package::staging_book_navigation_page_region_fixture(NAVIGATION).unwrap();
    let mut input = root(&bytes);
    input["document"]["blocks"][0]["semantic_kind"] = "note".into();
    let body = styled(&input, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    assert_eq!(nav.metadata().title.as_deref(), Some("Typaxis Book"));
    assert_eq!(
        nav.outline()[1].source.kind,
        StagingOutlineSourceKind::Heading
    );
    assert_eq!(nav.outline()[1].source.heading_level, Some(2));
    assert_eq!(nav.outline()[1].source.computed_language, "fr-Latn-FR");
    assert_eq!(
        nav.outline()[0].source.semantic_kind.as_deref(),
        Some("note")
    );
    assert!(nav.language(NodeId::new(10)).is_none());
    assert_eq!(
        nav.language(NodeId::new(11)).unwrap().parent(),
        Some(NodeId::new(0))
    );
    assert_eq!(
        nav.language(NodeId::new(12)).unwrap().effective_language(),
        "en-US"
    );
    for (pointer, value, kind) in [
        (
            "/metadata/modified",
            json!("2026-08-27T00:00:00Z"),
            BookNavigationSyntaxErrorKind::InvalidTimestamp,
        ),
        (
            "/metadata/keywords",
            json!(["same", "same"]),
            BookNavigationSyntaxErrorKind::InvalidMetadata,
        ),
        (
            "/outline/entries/1/source_kind",
            json!("semantic_container"),
            BookNavigationSyntaxErrorKind::InvalidOutline,
        ),
        (
            "/outline/entries/1/level",
            json!(3),
            BookNavigationSyntaxErrorKind::InvalidOutline,
        ),
    ] {
        let mut bad = input.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        let body = styled(&bad, &limits());
        assert_eq!(
            prepare_book_v2_navigation(&body).unwrap_err().kind(),
            kind,
            "{pointer}"
        );
    }
}
#[test]
fn native_math_and_navigation_nodes_share_one_ast_budget() {
    let input = root(MATH);
    let sample = styled(&input, &limits());
    // Five wire body nodes plus the metadata/outline wrapper nodes.
    let total = 7 + sample
        .body()
        .math()
        .iter()
        .map(|m| m.parsed().ast_node_count())
        .sum::<u64>();
    for decrement in [0, 1] {
        let mut raw = ResourceLimits::default();
        raw.max_ast_nodes = total - decrement;
        let limits = ValidatedResourceLimits::new(raw).unwrap();
        let body = styled(&input, &limits);
        let nav = prepare_book_v2_navigation(&body);
        if decrement == 0 {
            nav.unwrap();
        } else {
            assert_eq!(
                nav.unwrap_err().kind(),
                BookNavigationSyntaxErrorKind::AstNodeLimit
            );
        }
    }
}

#[test]
fn page_region_source_error_remains_an_input_diagnostic() {
    let bytes =
        typaxis_document_package::staging_book_navigation_page_region_fixture(NAVIGATION).unwrap();
    let mut input = root(&bytes);
    input["page_masters"]["masters"][0]["header_content"]["blocks"][0]["span"]["end_byte"] =
        100.into();
    let body = styled(&input, &limits());
    let error = prepare_book_v2_navigation(&body).unwrap_err();
    assert_eq!(
        error.kind(),
        BookNavigationSyntaxErrorKind::InvalidSourceSpan
    );
    assert_eq!(error.code(), "P1102");
    assert_eq!(
        error.pointer().as_str(),
        "/page_masters/masters/0/header_content/blocks/0/span"
    );
}

#[test]
fn metadata_and_outline_wrappers_require_their_own_depth() {
    let mut input = root(FIXTURE);
    input["document"]["blocks"] = json!([]);
    for depth in [1, 2] {
        let mut raw = ResourceLimits::default();
        raw.max_ast_nesting_depth = depth;
        let limits = ValidatedResourceLimits::new(raw).unwrap();
        let bytes = serde_json::to_vec(&input).unwrap();
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(&bytes, &DocumentPackageDecodePolicy::new(&limits));
        if depth == 1 {
            assert!(matches!(
                decoded.unwrap_err().kind(),
                typaxis_document_package::StagingSemanticDecodeError::Limit
            ));
        } else {
            let body = style_book_v2_body(prepare_book_v2_body(decoded.unwrap(), &limits).unwrap())
                .unwrap();
            prepare_book_v2_navigation(&body).unwrap();
        }
    }
}
