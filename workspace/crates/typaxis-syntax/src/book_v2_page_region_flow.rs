//! Original header/footer text, kept separate from the semantic body flow.
use super::super::source_ops::{ObservedSourceState, OverlayCarrierPermit, SourceOps};
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
    /// Inclusive legacy source slots: earlier caller records plus one region
    /// owner, four slots per paragraph and one per inline. The observed entry
    /// separately publishes additional Style carriers and requested storage.
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
    prepare_book_v2_page_region_text_flow_counted(selected, kind, navigation, prior_records, &mut 0)
}

/// Preserve caller history and each accepted source reservation, including when
/// a later reservation or source/style check fails. Rejected reservations are
/// atomic and leave the last accepted prefix intact.
pub fn prepare_book_v2_page_region_text_flow_counted<'a>(
    selected: BookV2SelectedPageMaster<'a>,
    kind: BookV2PageRegionKind,
    navigation: &'a PreparedBookV2Navigation<'a>,
    prior_records: u64,
    observed_records: &mut u64,
) -> Result<BookV2PageRegionTextFlow<'a>, ProductionFlowError> {
    *observed_records = prior_records;
    let mut preparation = RegionPreparation {
        mode: RegionOperations::Legacy(observed_records),
        records: prior_records,
    };
    prepare_region(selected, kind, navigation, &mut preparation)
}

/// Prepare one authentic selected running region with cumulative records,
/// requested storage bytes and work. Every accepted prefix is published on
/// every Result. The logical region slots remain distinct from additional
/// Style-owned carriers; this is not an allocator heap measurement.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_page_region_text_flow_observed_counted<'a>(
    selected: BookV2SelectedPageMaster<'a>,
    kind: BookV2PageRegionKind,
    navigation: &'a PreparedBookV2Navigation<'a>,
    prior_records: u64,
    prior_spool: u64,
    prior_work: u64,
    maximum_records: u64,
    maximum_work: u64,
    observed: &mut BookV2ObservedSourceFlowObservation,
) -> Result<BookV2PageRegionTextFlow<'a>, BookV2ObservedSourceFlowError> {
    let limits = selected.source().body().limits();
    let mut state = ObservedSourceState::new(
        [prior_records, prior_spool, prior_work],
        [
            maximum_records.min(limits.get().max_fragments),
            limits.get().max_spool_bytes,
            maximum_work,
        ],
    );
    let result = {
        let mut preparation = RegionPreparation {
            mode: RegionOperations::Observed(&mut state),
            records: prior_records,
        };
        prepare_region(selected, kind, navigation, &mut preparation)
    };
    *observed = state.progress;
    result.map_err(|error| state.finish_error(error))
}

enum RegionOperations<'s> {
    Legacy(&'s mut u64),
    Observed(&'s mut ObservedSourceState),
}
struct RegionPreparation<'s> {
    mode: RegionOperations<'s>,
    // The unchanged inclusive legacy region record charge. Full observed R
    // additionally includes actual lowered/copied Style carriers.
    records: u64,
}
impl RegionPreparation<'_> {
    fn operations(&mut self) -> SourceOps<'_> {
        match &mut self.mode {
            RegionOperations::Legacy(_) => SourceOps::legacy(),
            RegionOperations::Observed(state) => SourceOps::Observed(state),
        }
    }
    fn reserve(
        &mut self,
        owner: NodeId,
        count: u64,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), ProductionFlowError> {
        match &mut self.mode {
            RegionOperations::Legacy(observed) => {
                let records = self
                    .records
                    .checked_add(count)
                    .filter(|n| *n <= limits.get().max_fragments)
                    .ok_or_else(|| failure(ProductionFlowErrorKind::NodeLimit, owner))?;
                **observed = records;
                self.records = records;
            }
            RegionOperations::Observed(state) => {
                // Full observed R is at least this legacy inclusive logical
                // count; its checked reservation owns the budget error.
                state.logical(owner, count)?;
                self.records = self
                    .records
                    .checked_add(count)
                    .ok_or_else(|| failure(ProductionFlowErrorKind::NodeLimit, owner))?;
            }
        }
        Ok(())
    }
}
struct RegionFlowSource<'a, 's, 'p> {
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    preparation: &'p mut RegionPreparation<'s>,
}
impl<'a> FlowSource<'a> for RegionFlowSource<'a, '_, '_> {
    type Kind = typaxis_document_package::book_v2::WireBookV2SemanticContainerKind;
    fn limits(&self) -> &'a ValidatedResourceLimits {
        self.body.body().limits()
    }
    fn operations(&mut self) -> SourceOps<'_> {
        self.preparation.operations()
    }
    fn container_inheritance(
        &mut self,
        owner: NodeId,
    ) -> Result<Option<&'a SemanticContainerInheritanceStyle>, ProductionFlowError> {
        match &mut self.preparation.mode {
            RegionOperations::Legacy(_) => Ok(self
                .body
                .container_style(owner)
                .map(|s| s.inheritance_style())),
            RegionOperations::Observed(state) => self
                .body
                .container_style_with_work(owner, &mut |work| state.charge(owner, 0, 0, work))
                .map(|s| s.map(|s| s.inheritance_style())),
        }
    }
    fn language(&mut self, owner: NodeId) -> Result<Option<&'a str>, ProductionFlowError> {
        match &mut self.preparation.mode {
            RegionOperations::Legacy(_) => Ok(self
                .navigation
                .language(owner)
                .map(|r| r.effective_language())),
            RegionOperations::Observed(_) => {
                let values = self.navigation.languages();
                let index =
                    self.operations()
                        .find(owner, values, |r| r.node_id().cmp(&owner), |_| 0)?;
                Ok(index.map(|i| values[i].effective_language()))
            }
        }
    }
    fn anchors(&self) -> &'a [(AnchorId, NodeId)] {
        self.navigation.anchors()
    }
    // The named complete region preflight already paid every paragraph/site/
    // event slot. Collector event reservations retain the legacy no-op here;
    // observed Style carriers cannot consume or reuse those logical slots.
}

fn prepare_region<'a>(
    selected: BookV2SelectedPageMaster<'a>,
    kind: BookV2PageRegionKind,
    navigation: &'a PreparedBookV2Navigation<'a>,
    preparation: &mut RegionPreparation<'_>,
) -> Result<BookV2PageRegionTextFlow<'a>, ProductionFlowError> {
    preparation.operations().step(NodeId::new(0), 0)?;
    preparation.operations().step(NodeId::new(0), 1)?;
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
    preparation.reserve(owner, 1, limits)?;
    let mut nodes = 1u64;
    for block in &region.blocks {
        preparation.operations().step(owner, 1)?;
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
        let count = 4u64.checked_add(children.len() as u64).ok_or_else(fail)?;
        preparation.reserve(owner, count, limits)?;
    }
    let rules =
        preparation
            .operations()
            .lower_at(owner, body.body().wire().style_sheet(), limits, true)?;
    let mut collector = Collector {
        // Region slots have already been reserved by the region preflight.
        source: RegionFlowSource {
            body,
            navigation,
            preparation,
        },
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
        collector.source.operations().step(owner, 1)?;
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
        collector
            .source
            .operations()
            .grow(owner, &mut items, children.len(), true)?;
        for child in children {
            collector.source.operations().step(owner, 1)?;
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
                    // Eight closed metadata visits cover buffer lookup/ID,
                    // range metadata and the two UTF-8 boundary checks before
                    // those actions; the retained source text is never copied
                    // or fully scanned here.
                    collector.source.operations().step(owner, 8)?;
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
            collector.source.operations().step(owner, 1)?;
            items.push(ProductionInlineSite {
                owner,
                source_span,
                language,
                content,
                reference: None,
                link_target: None,
            });
        }
        collector
            .source
            .operations()
            .step(owner, items.len() as u64)?;
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
            .source
            .operations()
            .grow(owner, &mut collector.paragraphs, 1, false)?;
        // A page-region paragraph never selects a new body page.
        collector.source.operations().step(owner, 1)?;
        collector.paragraphs.push(ProductionTextParagraph {
            owner,
            style,
            items,
            page_name: None,
        });
        collector.event(ProductionFlowEvent::Paragraph { index }, owner)?;
        collector.end(owner)?;
    }
    // Fixed stack identity, not the main body SourceFlow canonical writer.
    // Work uses byte visits plus SHA input blocks and one finalization unit.
    let hash_work = |length: usize| {
        (length as u64)
            .checked_add((length as u64).div_ceil(64))
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| failure(ProductionFlowErrorKind::NodeLimit, owner))
    };
    // Pay both zero initialization and the101 subsequent identity writes.
    collector.source.operations().step(owner, 202)?;
    let mut identity = [0u8; 101];
    collector
        .source
        .operations()
        .step(owner, hash_work(BOOK_V2_PAGE_REGION_FLOW_ALGORITHM.len())?)?;
    identity[..32].copy_from_slice(&sha256(BOOK_V2_PAGE_REGION_FLOW_ALGORITHM.as_bytes()));
    identity[32..64].copy_from_slice(&body.body().canonical_jcs_sha256());
    collector
        .source
        .operations()
        .step(owner, hash_work(selected.master().master_id.len())?)?;
    identity[64..96].copy_from_slice(&sha256(selected.master().master_id.as_bytes()));
    identity[96] = match kind {
        BookV2PageRegionKind::Header => 0,
        BookV2PageRegionKind::Footer => 1,
    };
    identity[97..].copy_from_slice(&region.node_id.to_be_bytes());
    // Empty overlay hashing is a global preparation operation. Its shared
    // SourceOps budget/allocation failures use NodeId0, unlike the region-owned
    // stylesheet lowering above; its source TextLimit maps to this region.
    let generated = collector
        .source
        .operations()
        .overlay(
            Vec::new(),
            limits,
            navigation.retained_text_bytes(),
            OverlayCarrierPermit::after_named_reservation(0),
        )
        .map_err(|_| failure(ProductionFlowErrorKind::TextLimit, owner))?;
    collector
        .source
        .operations()
        .step(owner, hash_work(identity.len())?)?;
    let fingerprint = sha256(&identity);
    let records = collector.source.preparation.records;
    let flow = SourceTextFlow {
        package: body,
        navigation: &(),
        events: collector.events,
        paragraphs: collector.paragraphs,
        figures: Vec::new(),
        named_page_breaks: Vec::new(),
        tables: Vec::new(),
        table_record_charge: 0,
        source_record_charge: 0,
        lists: Vec::new(),
        list_items: Vec::new(),
        description_lists: Vec::new(),
        description_items: Vec::new(),
        footnote_definitions: Vec::new(),
        footnote_base_style: None,
        page_reference_values: None,
        generated,
        text_bytes: collector.text_bytes,
        fingerprint,
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

#[cfg(test)]
#[path = "book_v2_page_region_source_observed_tests.rs"]
mod observed_tests;
