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
    let data = input();
    let body = styled(&data, &limits());
    let nav = prepare_book_v2_navigation(&body).unwrap();
    let flow = prepare_book_v2_text_flow_with_page_references(&body, &nav, &[(NodeId::new(7), 12)])
        .unwrap();
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
        let result =
            prepare_book_v2_text_flow_with_page_references(&body, &nav, &[(NodeId::new(7), 12)]);
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
