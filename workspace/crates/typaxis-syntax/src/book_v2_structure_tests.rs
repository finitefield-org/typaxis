use super::*;
use crate::book_v2::{
    prepare_book_v2_body, prepare_book_v2_navigation, prepare_book_v2_text_flow, style_book_v2_body,
};
use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
use typaxis_document_package::{
    book_v2::{BookV2DocumentPackageDecoder, WireBookV2SemanticContainerKind},
    DocumentPackageDecodePolicy,
};
const FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));
const COMBINED: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json"));
struct Capture<'a> {
    work: usize,
    maximum: usize,
    nodes: Vec<BookV2StructureSourceNode<'a>>,
}
impl<'a> BookV2StructureVisitor<'a> for Capture<'a> {
    type Error = BookV2StructureSourceError;
    fn step(&mut self, n: usize) -> Result<(), Self::Error> {
        if n > self.maximum.saturating_sub(self.work) {
            return Err(BookV2StructureSourceError::Identity);
        }
        self.work += n;
        Ok(())
    }
    fn node(&mut self, node: BookV2StructureSourceNode<'a>) -> Result<(), Self::Error> {
        self.nodes.push(node);
        Ok(())
    }
}
#[test]
fn source_structure_preserves_all_twelve_authored_container_kinds_and_can_stop_before_visit() {
    for kind in WireBookV2SemanticContainerKind::ALL {
        let mut input: serde_json::Value = serde_json::from_slice(FIXTURE).unwrap();
        input["contract"] = "typaxis.contract/1.5".into();
        input["style_sheet"] =
            serde_json::from_slice::<serde_json::Value>(COMBINED).unwrap()["style_sheet"].clone();
        input["document"]["blocks"][0]["semantic_kind"] = kind.as_str().into();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(
                &serde_json::to_vec(&input).unwrap(),
                &DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        let body = style_book_v2_body(prepare_book_v2_body(decoded, &limits).unwrap()).unwrap();
        let navigation = prepare_book_v2_navigation(&body).unwrap();
        let flow = prepare_book_v2_text_flow(&body, &navigation).unwrap();
        let mut capture = Capture {
            work: 0,
            maximum: usize::MAX,
            nodes: Vec::new(),
        };
        visit_book_v2_structure(&flow, &mut capture).unwrap();
        let root_container = capture
            .nodes
            .iter()
            .find(|n| n.key().owner() == NodeId::new(1))
            .unwrap();
        assert_eq!(root_container.semantic_kind(), Some(kind.as_str()));
        assert_eq!(
            root_container.pdf_role(),
            if kind.as_str() == "quote" {
                "BlockQuote"
            } else {
                "Sect"
            }
        );
        assert_eq!(
            root_container.parent(),
            Some(BookV2StructureKey::new(
                NodeId::new(0),
                BookV2StructureSlot::Source
            ))
        );
        for node in &capture.nodes {
            let language = navigation.language(node.key().owner()).unwrap();
            assert!(std::ptr::eq(node.language(), language.effective_language()));
            assert_eq!(node.source_span(), language.source_span());
        }
        let mut exact = Capture {
            work: 0,
            maximum: capture.work,
            nodes: Vec::new(),
        };
        visit_book_v2_structure(&flow, &mut exact).unwrap();
        assert_eq!(exact.nodes.len(), capture.nodes.len());
        let mut short = Capture {
            work: 0,
            maximum: capture.work - 1,
            nodes: Vec::new(),
        };
        assert!(visit_book_v2_structure(&flow, &mut short).is_err());
        let mut stopped = Capture {
            work: 0,
            maximum: 0,
            nodes: Vec::new(),
        };
        assert!(visit_book_v2_structure(&flow, &mut stopped).is_err());
        assert!(stopped.nodes.is_empty());
    }
}

#[test]
fn source_structure_includes_generated_nodes_in_ast_limit() {
    let mut input: serde_json::Value = serde_json::from_slice(COMBINED).unwrap();
    input["contract"] = "typaxis.contract/1.5".into();
    let bytes = serde_json::to_vec(&input).unwrap();
    let mut required = None;
    for adjustment in [None, Some(0u64), Some(1)] {
        let mut raw = ResourceLimits::default();
        if let Some(short) = adjustment {
            raw.max_ast_nodes = required.unwrap() - short;
        }
        let limits = ValidatedResourceLimits::new(raw).unwrap();
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(&bytes, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let body = style_book_v2_body(prepare_book_v2_body(decoded, &limits).unwrap()).unwrap();
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
        let mut capture = Capture {
            work: 0,
            maximum: usize::MAX,
            nodes: Vec::new(),
        };
        let result = visit_book_v2_structure(&flow, &mut capture);
        if adjustment == Some(1) {
            assert_eq!(result, Err(BookV2StructureSourceError::Nodes));
        } else {
            result.unwrap();
            if adjustment.is_none() {
                let source = typaxis_document_package::book_v2::book_v2_wire_ast_node_count(
                    body.body().wire(),
                    limits.get().max_ast_nesting_depth,
                )
                .unwrap();
                let math: u64 = body
                    .body()
                    .math()
                    .iter()
                    .map(|m| m.parsed().ast_node_count())
                    .sum();
                let generated = capture
                    .nodes
                    .iter()
                    .filter(|n| n.key().slot() != BookV2StructureSlot::Source)
                    .count() as u64;
                assert!(generated > 0);
                required = Some(source + math + generated);
            }
        }
    }
}

#[test]
fn source_structure_bounds_generated_depth_without_rejecting_the_exact_limit() {
    let mut input: serde_json::Value = serde_json::from_slice(COMBINED).unwrap();
    input["contract"] = "typaxis.contract/1.5".into();
    let bytes = serde_json::to_vec(&input).unwrap();
    let mut required = None;
    for adjustment in [None, Some(0u32), Some(1)] {
        let mut raw = ResourceLimits::default();
        if let Some(short) = adjustment {
            raw.max_ast_nesting_depth = required.unwrap() - short;
        }
        let limits = ValidatedResourceLimits::new(raw).unwrap();
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(&bytes, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let body = style_book_v2_body(prepare_book_v2_body(decoded, &limits).unwrap()).unwrap();
        let nav = prepare_book_v2_navigation(&body).unwrap();
        let flow = prepare_book_v2_text_flow(&body, &nav).unwrap();
        let mut capture = Capture {
            work: 0,
            maximum: usize::MAX,
            nodes: Vec::new(),
        };
        let result = visit_book_v2_structure(&flow, &mut capture);
        if adjustment == Some(1) {
            assert_eq!(result, Err(BookV2StructureSourceError::Depth));
        } else {
            result.unwrap();
            if adjustment.is_none() {
                let mut depths = std::collections::BTreeMap::new();
                for node in &capture.nodes {
                    let depth = node.parent().map_or(1, |p| depths[&p] + 1);
                    depths.insert(node.key(), depth);
                }
                required = depths.values().copied().max();
            }
        }
    }
}
