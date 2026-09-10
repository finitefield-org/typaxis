//! Original header/footer text, kept separate from the semantic body flow.
use super::*;
use crate::book_v2::BookV2SelectedPageMaster;
use typaxis_document_package::{WirePageRegion, WirePageRegionBlock, WirePageRegionInline};

pub const BOOK_V2_PAGE_REGION_FLOW_ALGORITHM: &str = "typaxis.book-2-page-region-text-flow/1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookV2PageRegionKind {
    Header,
    Footer,
}
impl BookV2PageRegionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Footer => "footer",
        }
    }
}
/// The common paragraph carrier has a different navigation parameter from a
/// body flow; it cannot be passed to a body consumer by dereferencing this owner.
///
/// ```compile_fail
/// use typaxis_syntax::book_v2::{BookV2PageRegionTextFlow, PreparedBookV2TextFlow};
/// fn body<'a>(region: &'a BookV2PageRegionTextFlow<'a>) -> &'a PreparedBookV2TextFlow<'a> {
///     region.text_flow()
/// }
/// ```
pub struct BookV2PageRegionTextFlow<'a> {
    flow: SourceTextFlow<'a, StyledBookV2Body, ()>,
    navigation: &'a PreparedBookV2Navigation<'a>,
    master: &'a str,
    kind: BookV2PageRegionKind,
    region: &'a WirePageRegion,
    records: u64,
}
impl<'a> BookV2PageRegionTextFlow<'a> {
    pub fn text_flow(&self) -> &SourceTextFlow<'a, StyledBookV2Body, ()> {
        &self.flow
    }
    pub fn body(&self) -> &'a StyledBookV2Body {
        self.flow.package
    }
    pub fn navigation(&self) -> &'a PreparedBookV2Navigation<'a> {
        self.navigation
    }
    pub fn master_id(&self) -> &'a str {
        self.master
    }
    pub fn kind(&self) -> BookV2PageRegionKind {
        self.kind
    }
    pub fn source(&self) -> &'a WirePageRegion {
        self.region
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.flow.fingerprint
    }
    pub fn verify_for(
        &self,
        body: &StyledBookV2Body,
        navigation: &PreparedBookV2Navigation<'_>,
    ) -> Result<(), ProductionFlowError> {
        if !std::ptr::eq(body, self.body()) || !std::ptr::eq(navigation, self.navigation) {
            return Err(failure(
                ProductionFlowErrorKind::ReceiptMismatch,
                NodeId::new(self.region.node_id),
            ));
        }
        Ok(())
    }
}

/// Collect exactly one authored master region. No body/footnote nodes are
/// synthesized, and page selection, layout and paint authorization remain with
/// their downstream owners. All retained paragraph/site/event slots are bounded
/// before their allocations; style lowering keeps its existing source limits.
pub fn prepare_book_v2_page_region_text_flow<'a>(
    selected: BookV2SelectedPageMaster<'a>,
    kind: BookV2PageRegionKind,
    navigation: &'a PreparedBookV2Navigation<'a>,
    prior_records: u64,
) -> Result<BookV2PageRegionTextFlow<'a>, ProductionFlowError> {
    let body = selected.source();
    navigation
        .verify_for(body)
        .map_err(|_| failure(ProductionFlowErrorKind::ReceiptMismatch, NodeId::new(0)))?;
    let region = match kind {
        BookV2PageRegionKind::Header => selected.advanced().header_content.as_ref(),
        BookV2PageRegionKind::Footer => selected.advanced().footer_content.as_ref(),
    }
    .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, NodeId::new(0)))?;
    let owner = NodeId::new(region.node_id);
    let limits = body.body().limits();
    let fail = || failure(ProductionFlowErrorKind::NodeLimit, owner);
    let mut records = prior_records
        .checked_add(1)
        .filter(|n| *n <= limits.get().max_fragments)
        .ok_or_else(fail)?;
    let mut nodes = 1u64;
    for block in &region.blocks {
        let children = match block {
            WirePageRegionBlock::Paragraph { children, .. }
            | WirePageRegionBlock::Heading { children, .. } => children,
        };
        nodes = nodes
            .checked_add(1)
            .and_then(|n| n.checked_add(children.len() as u64))
            .filter(|n| *n <= limits.get().max_ast_nodes)
            .ok_or_else(fail)?;
        // One paragraph, its three events, and every inline site.
        records = records
            .checked_add(4)
            .and_then(|n| n.checked_add(children.len() as u64))
            .filter(|n| *n <= limits.get().max_fragments)
            .ok_or_else(fail)?;
    }
    let rules = lower_semantic_style_rules_version(body.body().wire().style_sheet(), limits, true)
        .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, owner))?;
    let mut collector = Collector {
        source: BookV2FlowSource { body, navigation },
        rules,
        buffers: body.body().wire().text_buffers(),
        events: Vec::new(),
        paragraphs: Vec::new(),
        figures: Vec::new(),
        named_page_breaks: Vec::new(),
        tables: Vec::new(),
        table_record_charge: 0,
        lists: Vec::new(),
        list_items: Vec::new(),
        description_lists: Vec::new(),
        description_items: Vec::new(),
        footnote_ordinals: Vec::new(),
        generated_records: Vec::new(),
        generated_bytes: 0,
        retained_text_bytes: navigation.retained_text_bytes(),
        text_bytes: 0,
        node_charge: 1,
    };
    for block in &region.blocks {
        let (id, classes, children, kind) = match block {
            WirePageRegionBlock::Paragraph {
                node_id,
                classes,
                children,
                ..
            } => (
                *node_id,
                classes,
                children,
                ProductionFlowRegionKind::Paragraph,
            ),
            WirePageRegionBlock::Heading {
                node_id,
                classes,
                children,
                ..
            } => (
                *node_id,
                classes,
                children,
                ProductionFlowRegionKind::Heading,
            ),
        };
        let owner = NodeId::new(id);
        collector.begin(owner, kind)?;
        let style = collector.ordinary(owner, kind.as_str(), classes, None)?;
        let inherited = collector.language(owner)?;
        let mut items = Vec::new();
        items
            .try_reserve_exact(children.len())
            .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
        for child in children {
            let (id, span) = match child {
                WirePageRegionInline::Text { node_id, span, .. }
                | WirePageRegionInline::SoftBreak { node_id, span }
                | WirePageRegionInline::HardBreak { node_id, span } => (*node_id, *span),
            };
            let owner = NodeId::new(id);
            collector.charge(owner)?;
            let source_span = SourceSpan::new(
                SourceId::new(span.source_id),
                Utf8ByteOffset::new(span.start_byte),
                Utf8ByteOffset::new(span.end_byte),
            )
            .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
            let language = if matches!(child, WirePageRegionInline::Text { .. }) {
                collector.language(owner)?
            } else {
                inherited
            };
            let content = match child {
                WirePageRegionInline::Text { text_span, .. } => {
                    let buffer = collector
                        .buffers
                        .get(text_span.text_id as usize)
                        .filter(|b| b.text_id == text_span.text_id)
                        .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
                    let utf8 = buffer
                        .utf8
                        .get(text_span.start_byte as usize..text_span.end_byte as usize)
                        .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
                    collector.text_bytes = collector
                        .text_bytes
                        .checked_add(utf8.len() as u64)
                        .filter(|n| *n <= limits.get().max_text_bytes)
                        .ok_or_else(|| failure(ProductionFlowErrorKind::TextLimit, owner))?;
                    let span = TextSpan::new(
                        TextBufferId::new(text_span.text_id),
                        Utf8ByteOffset::new(text_span.start_byte),
                        Utf8ByteOffset::new(text_span.end_byte),
                    )
                    .ok_or_else(|| failure(ProductionFlowErrorKind::ReceiptMismatch, owner))?;
                    ProductionInlineContent::Text { span, utf8 }
                }
                WirePageRegionInline::SoftBreak { .. } => ProductionInlineContent::SoftBreak,
                WirePageRegionInline::HardBreak { .. } => ProductionInlineContent::HardBreak,
            };
            items.push(ProductionInlineSite {
                owner,
                source_span,
                language,
                content,
                reference: None,
                link_target: None,
            });
        }
        if items
            .iter()
            .any(|i| matches!(i.content,ProductionInlineContent::Text{utf8,..} if !utf8.is_empty()))
            && (style.font_families().is_none()
                || style.font_size().is_none()
                || style.line_height().is_none())
        {
            return Err(failure(ProductionFlowErrorKind::MissingTextStyle, owner));
        }
        let index = u32::try_from(collector.paragraphs.len())
            .map_err(|_| failure(ProductionFlowErrorKind::NodeLimit, owner))?;
        collector
            .paragraphs
            .try_reserve(1)
            .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
        // A page-region paragraph never selects a new body page.
        collector.paragraphs.push(ProductionTextParagraph {
            owner,
            style,
            items,
            page_name: None,
        });
        collector.event(ProductionFlowEvent::Paragraph { index }, owner)?;
        collector.end(owner)?;
    }
    let mut identity = [0u8; 101];
    identity[..32].copy_from_slice(&sha256(BOOK_V2_PAGE_REGION_FLOW_ALGORITHM.as_bytes()));
    identity[32..64].copy_from_slice(&body.body().canonical_jcs_sha256());
    identity[64..96].copy_from_slice(&sha256(selected.master().master_id.as_bytes()));
    identity[96] = match kind {
        BookV2PageRegionKind::Header => 0,
        BookV2PageRegionKind::Footer => 1,
    };
    identity[97..].copy_from_slice(&region.node_id.to_be_bytes());
    let flow = SourceTextFlow {
        package: body,
        navigation: &(),
        events: collector.events,
        paragraphs: collector.paragraphs,
        figures: Vec::new(),
        named_page_breaks: Vec::new(),
        tables: Vec::new(),
        table_record_charge: 0,
        lists: Vec::new(),
        list_items: Vec::new(),
        description_lists: Vec::new(),
        description_items: Vec::new(),
        footnote_definitions: Vec::new(),
        footnote_base_style: None,
        page_reference_values: None,
        generated: typaxis_text::GeneratedTextOverlay::new(
            Vec::new(),
            limits,
            navigation.retained_text_bytes(),
        )
        .map_err(|_| failure(ProductionFlowErrorKind::TextLimit, owner))?,
        text_bytes: collector.text_bytes,
        fingerprint: sha256(&identity),
    };
    Ok(BookV2PageRegionTextFlow {
        flow,
        navigation,
        master: &selected.master().master_id,
        kind,
        region,
        records,
    })
}
