//! Source-ordered successor flow. This stage retains original source owners and
//! generated labels, but grants no host, font, layout, profile or PDF authority.
use super::*;
use crate::book_v2::{PreparedBookV2Navigation, StyledBookV2Body};
use std::fmt::Write as _;
use typaxis_core::{write_jcs_string, Sha256};
use typaxis_style::book_v2::BookV2SemanticContainerComputedStyle;

pub const BOOK_V2_TEXT_FLOW_ALGORITHM: &str = "typaxis.book-2-source-text-flow/1";

/// A successor flow cannot be supplied to consumers requiring a legacy flow.
///
/// ```compile_fail
/// use typaxis_syntax::{book_v2::PreparedBookV2TextFlow, ProductionTextFlow};
/// fn legacy<'a>(flow: PreparedBookV2TextFlow<'a>) -> ProductionTextFlow<'a> {
///     flow
/// }
/// ```
pub type PreparedBookV2TextFlow<'a> =
    SourceTextFlow<'a, StyledBookV2Body, PreparedBookV2Navigation<'a>>;

struct BookV2FlowSource<'a, 'b> {
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    records: Option<BookV2SourceRecordBudget<'b>>,
}
struct BookV2SourceRecordBudget<'a> {
    prior: u64,
    maximum: u64,
    observed: &'a mut u64,
}
impl BookV2SourceRecordBudget<'_> {
    fn reserve(&mut self, owner: NodeId, count: u64) -> Result<(), ProductionFlowError> {
        let next = self.observed
            .checked_add(count)
            .filter(|n| *n <= self.maximum)
            .ok_or_else(|| failure(ProductionFlowErrorKind::NodeLimit, owner))?;
        *self.observed = next;
        Ok(())
    }
}
impl<'a> FlowSource<'a> for BookV2FlowSource<'a, '_> {
    type Kind = typaxis_document_package::book_v2::WireBookV2SemanticContainerKind;
    fn limits(&self) -> &'a ValidatedResourceLimits {
        self.body.body().limits()
    }
    fn container_inheritance(
        &self,
        owner: NodeId,
    ) -> Option<&'a SemanticContainerInheritanceStyle> {
        self.body
            .container_style(owner)
            .map(|style| style.inheritance_style())
    }
    fn language(&self, owner: NodeId) -> Option<&'a str> {
        self.navigation
            .language(owner)
            .map(|record| record.effective_language())
    }
    fn anchors(&self) -> &'a [(AnchorId, NodeId)] {
        self.navigation.anchors()
    }
    fn source_text_references(&self) -> bool {
        true
    }
    fn reference_number(&self, anchor: &str) -> Option<&'a str> {
        self.navigation.reference_number(anchor)
    }
    fn reference_label(&self, target: NodeId) -> Option<&'a str> {
        self.navigation
            .outline()
            .iter()
            .find(|e| e.source.node_id == target)
            .map(|e| e.label.as_str())
    }
    fn heading_label(&self, target: NodeId) -> bool {
        self.navigation
            .language(target)
            .is_some_and(|r| r.kind() == typaxis_document::book_v2::BookV2LanguageNodeKind::Heading)
    }
    fn reserve_records(&mut self, owner: NodeId, count: u64) -> Result<(), ProductionFlowError> {
        if let Some(records) = &mut self.records {
            records.reserve(owner, count)?;
        }
        Ok(())
    }
    fn source_record_charge(&self) -> u64 {
        self.records.as_ref().map_or(0, |r| *r.observed - r.prior)
    }
}

impl<'a> PreparedBookV2TextFlow<'a> {
    pub const fn body(&self) -> &'a StyledBookV2Body {
        self.package
    }
    pub const fn navigation(&self) -> &'a PreparedBookV2Navigation<'a> {
        self.navigation
    }
    /// Declarations only; the flow has not read any resource bytes.
    pub fn resource_declarations(&self) -> &typaxis_document::StagingM4ResourceCatalog {
        self.package.body().resources()
    }
    pub fn semantic_container_style(
        &self,
        owner: NodeId,
    ) -> Option<&BookV2SemanticContainerComputedStyle> {
        self.package.container_style(owner)
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package.body().canonical_jcs_sha256()
    }
    /// Logical source reservations, excluding the caller's earlier history.
    /// Includes temporary ordinal/generated slots and the table occupancy bound;
    /// this is neither a byte allocation measurement nor a layout receipt.
    pub const fn source_record_charge(&self) -> u64 {
        self.source_record_charge
    }
    /// Recomputes the shared collection from the exact immutable source owners.
    /// Equal canonical input hashes do not authorize substituting another body
    /// or another navigation owner.
    pub fn verify_for(
        &self,
        body: &StyledBookV2Body,
        navigation: &PreparedBookV2Navigation<'_>,
    ) -> Result<(), ProductionFlowError> {
        let mismatch = || failure(ProductionFlowErrorKind::ReceiptMismatch, NodeId::new(0));
        if !std::ptr::eq(self.package, body) || !std::ptr::eq(self.navigation, navigation) {
            return Err(mismatch());
        }
        let observed = prepare_inner(self.package, self.navigation, self.page_reference_values())?;
        if self.events != observed.events
            || self.paragraphs != observed.paragraphs
            || self.named_page_breaks != observed.named_page_breaks
            || self.figures != observed.figures
            || self.tables != observed.tables
            || self.table_record_charge != observed.table_record_charge
            || self.source_record_charge != observed.source_record_charge
            || self.lists != observed.lists
            || self.list_items != observed.list_items
            || self.description_lists != observed.description_lists
            || self.description_items != observed.description_items
            || self.footnote_definitions != observed.footnote_definitions
            || self.footnote_base_style != observed.footnote_base_style
            || self.generated != observed.generated
            || self.text_bytes != observed.text_bytes
            || self.fingerprint != observed.fingerprint
        {
            return Err(mismatch());
        }
        Ok(())
    }
}

pub fn prepare_book_v2_text_flow<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
) -> Result<PreparedBookV2TextFlow<'a>, ProductionFlowError> {
    prepare_inner(body, navigation, None)
}

/// Candidate page labels for every Page-format reference, sorted by source node.
/// Values are not proof of final placement. Text/Number targets remain typed.
pub fn prepare_book_v2_text_flow_with_page_references<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    values: &[(NodeId, u32)],
) -> Result<PreparedBookV2TextFlow<'a>, ProductionFlowError> {
    prepare_inner(body, navigation, Some(values))
}

fn prepare_inner<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    values: Option<&[(NodeId, u32)]>,
) -> Result<PreparedBookV2TextFlow<'a>, ProductionFlowError> {
    prepare_inner_counted(
        body, navigation, values, 0, body.body().limits().get().max_fragments, &mut 0,
    )
}

/// Reserve source slots against both the body and caller ceilings before their
/// construction. Observations always start with caller history, and retain each
/// accepted prefix on a later source/style/allocation failure. A rejected
/// reservation does not advance the observation.
pub fn prepare_book_v2_text_flow_counted<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    prior_records: u64,
    maximum_records: u64,
    observed_records: &mut u64,
) -> Result<PreparedBookV2TextFlow<'a>, ProductionFlowError> {
    prepare_inner_counted(
        body, navigation, None, prior_records, maximum_records, observed_records,
    )
}

/// Candidate labels have the same reservation/failure history as an initial
/// source flow. Every retained value and both generated-record carriers count.
pub fn prepare_book_v2_text_flow_with_page_references_counted<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    values: &[(NodeId, u32)],
    prior_records: u64,
    maximum_records: u64,
    observed_records: &mut u64,
) -> Result<PreparedBookV2TextFlow<'a>, ProductionFlowError> {
    prepare_inner_counted(
        body, navigation, Some(values), prior_records, maximum_records, observed_records,
    )
}

fn prepare_inner_counted<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    values: Option<&[(NodeId, u32)]>,
    prior_records: u64,
    maximum_records: u64,
    observed_records: &mut u64,
) -> Result<PreparedBookV2TextFlow<'a>, ProductionFlowError> {
    *observed_records = prior_records;
    let root = NodeId::new(0);
    navigation
        .verify_for(body)
        .map_err(|_| failure(ProductionFlowErrorKind::ReceiptMismatch, root))?;
    let wire = body.body().wire();
    let limits = body.body().limits();
    let mut source = BookV2FlowSource {
        body,
        navigation,
        records: Some(BookV2SourceRecordBudget {
            prior: prior_records,
            maximum: maximum_records.min(limits.get().max_fragments),
            observed: observed_records,
        }),
    };
    source.reserve_records(root, 1)?;
    let rules = lower_semantic_style_rules_version(wire.style_sheet(), limits, true)
        .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, root))?;
    let mut flow = collect_source_flow(
        source,
        body,
        navigation,
        wire.document(),
        wire.text_buffers(),
        rules,
        navigation.retained_text_bytes(),
        limits,
        values,
    )?;
    // Only the limits used by this source-flow stage enter this projection.
    // Full source-body limits remain owned and compared by the immutable body.
    let bound = limits.get();
    let mut limits_jcs = Sha256::new();
    write!(
        limits_jcs,
        "[\"typaxis.book-2-source-flow-limits/1\",{},{},{},{},{}]",
        bound.max_ast_nodes,
        bound.max_fragments,
        bound.max_pages,
        bound.max_text_buffer_bytes,
        bound.max_text_bytes,
    )
    .expect("writing source-flow limits to SHA-256 cannot fail");
    let mut languages = Sha256::new();
    languages
        .write_str("[\"typaxis.book-2-source-flow-languages/1\"")
        .expect("writing source-flow languages to SHA-256 cannot fail");
    for record in navigation.languages() {
        write!(languages, ",[{},", record.node_id().get())
            .expect("writing a language owner to SHA-256 cannot fail");
        write_jcs_string(&mut languages, record.effective_language())
            .expect("writing a canonical language to SHA-256 cannot fail");
        languages
            .write_char(']')
            .expect("writing to SHA-256 cannot fail");
    }
    languages
        .write_char(']')
        .expect("writing to SHA-256 cannot fail");
    flow.fingerprint = fingerprint_source_flow(
        &flow,
        BOOK_V2_TEXT_FLOW_ALGORITHM,
        languages.finish(),
        limits_jcs.finish(),
        body.body().canonical_jcs_sha256(),
    );
    Ok(flow)
}

#[cfg(test)]
#[path = "book_v2_flow_tests.rs"]
mod tests;

#[path = "book_v2_page_region_flow.rs"]
mod page_regions;
pub use page_regions::{
    prepare_book_v2_page_region_text_flow, prepare_book_v2_page_region_text_flow_counted,
    BookV2PageRegionKind, BookV2PageRegionTextFlow, BOOK_V2_PAGE_REGION_FLOW_ALGORITHM,
};
