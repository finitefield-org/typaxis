//! Source-ordered successor flow. This stage retains original source owners and
//! generated labels, but grants no host, font, layout, profile or PDF authority.
use super::*;
use crate::book_v2::{PreparedBookV2Navigation, StyledBookV2Body};
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

struct BookV2FlowSource<'a> {
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
}
impl<'a> FlowSource<'a> for BookV2FlowSource<'a> {
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
            || self.figures != observed.figures
            || self.tables != observed.tables
            || self.table_record_charge != observed.table_record_charge
            || self.lists != observed.lists
            || self.list_items != observed.list_items
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
    let root = NodeId::new(0);
    navigation
        .verify_for(body)
        .map_err(|_| failure(ProductionFlowErrorKind::ReceiptMismatch, root))?;
    let wire = body.body().wire();
    let limits = body.body().limits();
    let rules = lower_semantic_style_rules(wire.style_sheet(), limits)
        .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, root))?;
    let mut flow = collect_source_flow(
        BookV2FlowSource { body, navigation },
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
    let limits_jcs = format!(
        "[\"typaxis.book-2-source-flow-limits/1\",{},{},{},{},{}]",
        bound.max_ast_nodes,
        bound.max_fragments,
        bound.max_pages,
        bound.max_text_buffer_bytes,
        bound.max_text_bytes,
    );
    let mut languages = String::from("[\"typaxis.book-2-source-flow-languages/1\"");
    for record in navigation.languages() {
        languages.push_str(&format!(",[{},", record.node_id().get()));
        push_jcs_string(&mut languages, record.effective_language());
        languages.push(']');
    }
    languages.push(']');
    flow.fingerprint = sha256(
        encode_source_flow(
            &flow,
            BOOK_V2_TEXT_FLOW_ALGORITHM,
            sha256(languages.as_bytes()),
            sha256(limits_jcs.as_bytes()),
            body.body().canonical_jcs_sha256(),
        )
        .as_bytes(),
    );
    Ok(flow)
}

#[cfg(test)]
#[path = "book_v2_flow_tests.rs"]
mod tests;
