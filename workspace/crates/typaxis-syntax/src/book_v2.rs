//! Internal contract-1.5 body preparation. This stage validates source spans,
//! dense node order, text boundaries and unstyled math/vector facts. It does not
//! admit styles, host resources, a production profile, layout or PDF output.

use super::*;
use typaxis_document::book_v2::{BookV2Document, BookV2SemanticContainerKind};
use typaxis_document_package::book_v2::{
    DecodedBookV2DocumentPackage, WireBookV2DocumentPackage, WireBookV2SemanticContainerKind,
};

/// Source-validated vector use without a versioned syntax or layout receipt.
/// Fields cannot be authored by a caller and cannot be converted into a 1.4
/// `ValidatedPrecomposedVectorMetrics` receipt.
///
/// ```compile_fail
/// use typaxis_syntax::{book_v2::PreparedBookVector, ValidatedPrecomposedVectorMetrics};
/// fn old_receipt(vector: &PreparedBookVector) -> &ValidatedPrecomposedVectorMetrics {
///     vector
/// }
/// ```
#[derive(Debug)]
pub struct PreparedBookVector {
    pub(super) node_id: NodeId,
    pub(super) owner_source_span: SourceSpan,
    pub(super) kind: PrecomposedVectorKind,
    pub(super) image_id: ImageResourceId,
    pub(super) payload: PrecomposedVectorMetricPayload,
    pub(super) source_tex: Option<ValidatedPrecomposedVectorTextBinding>,
    pub(super) alternative: ValidatedPrecomposedVectorAlternative,
    pub(super) language: Option<ValidatedPrecomposedVectorLanguageOverride>,
    pub(super) equation_number: Option<ValidatedPrecomposedVectorEquationNumber>,
}

impl PreparedBookVector {
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn owner_source_span(&self) -> SourceSpan {
        self.owner_source_span
    }
    pub const fn kind(&self) -> PrecomposedVectorKind {
        self.kind
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn payload(&self) -> PrecomposedVectorMetricPayload {
        self.payload
    }
    pub fn source_tex(&self) -> Option<&ValidatedPrecomposedVectorTextBinding> {
        self.source_tex.as_ref()
    }
    pub const fn alternative(&self) -> &ValidatedPrecomposedVectorAlternative {
        &self.alternative
    }
    pub fn language(&self) -> Option<&ValidatedPrecomposedVectorLanguageOverride> {
        self.language.as_ref()
    }
    pub fn equation_number(&self) -> Option<&ValidatedPrecomposedVectorEquationNumber> {
        self.equation_number.as_ref()
    }
}

/// Parsed native math with original source ownership, before style selection.
#[derive(Debug)]
pub struct PreparedBookMath(PendingStagingMathNode);
impl PreparedBookMath {
    pub const fn domain(&self) -> &StagingM4MathNode {
        &self.0.domain
    }
    pub const fn parsed(&self) -> &ParsedMathReceipt {
        &self.0.parsed
    }
}

/// Owned result of body preparation; the complete original wire is retained
/// alongside the typed body view so anchors, attributes and text are not lost.
/// This is deliberately not a fully admitted semantic package.
///
/// ```compile_fail
/// use typaxis_syntax::{book_v2::PreparedBookV2Body, ValidatedStagingSemanticPackage};
/// fn old_package(body: PreparedBookV2Body) -> ValidatedStagingSemanticPackage {
///     body
/// }
/// ```
#[derive(Debug)]
pub struct PreparedBookV2Body {
    wire: WireBookV2DocumentPackage,
    document: BookV2Document,
    resources: StagingM4ResourceCatalog,
    vectors: Vec<PreparedBookVector>,
    math: Vec<PreparedBookMath>,
    limits: ValidatedResourceLimits,
    retained_text_bytes: u64,
    raw_sha256: [u8; 32],
    canonical_jcs_sha256: [u8; 32],
}
impl PreparedBookV2Body {
    pub const fn wire(&self) -> &WireBookV2DocumentPackage {
        &self.wire
    }
    pub const fn document(&self) -> &BookV2Document {
        &self.document
    }
    /// Declarations only: this stage has not read or admitted resource bytes.
    pub const fn resources(&self) -> &StagingM4ResourceCatalog {
        &self.resources
    }
    pub fn vectors(&self) -> &[PreparedBookVector] {
        &self.vectors
    }
    pub fn math(&self) -> &[PreparedBookMath] {
        &self.math
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub const fn retained_text_bytes(&self) -> u64 {
        self.retained_text_bytes
    }
    pub const fn raw_sha256(&self) -> [u8; 32] {
        self.raw_sha256
    }
    pub const fn canonical_jcs_sha256(&self) -> [u8; 32] {
        self.canonical_jcs_sha256
    }
}

pub fn prepare_book_v2_body(
    decoded: DecodedBookV2DocumentPackage,
    limits: &ValidatedResourceLimits,
) -> Result<PreparedBookV2Body, StagingSemanticSyntaxError> {
    if decoded.limits() != limits {
        return Err(StagingSemanticSyntaxError::ReceiptMismatch);
    }
    let raw_sha256 = decoded.raw_sha256();
    let canonical_jcs_sha256 = decoded.canonical_jcs_sha256();
    let wire = decoded.into_wire();
    let sources = parse_source_lengths(wire.sources())?;
    let text_buffers = parse_text_buffers(wire.text_buffers())?;
    let admitted_text_bytes = text_buffers.values().try_fold(0u64, |total, buffer| {
        total
            .checked_add(
                u64::try_from(buffer.utf8.len())
                    .map_err(|_| StagingSemanticSyntaxError::MathSourceTextLimit)?,
            )
            .ok_or(StagingSemanticSyntaxError::MathSourceTextLimit)
    })?;
    if admitted_text_bytes > limits.get().max_text_bytes {
        return Err(StagingSemanticSyntaxError::MathSourceTextLimit);
    }
    let mut validator = SemanticValidator {
        sources: &sources,
        text_buffers: &text_buffers,
        precomposed_vector_text_buffer_sha256: BTreeMap::new(),
        precomposed_vector_text_slice_sha256: BTreeMap::new(),
        next_node_id: 0,
        node_count: 0,
        admitted_text_and_math_speech_bytes: admitted_text_bytes,
        math_nodes: Vec::new(),
        precomposed_vector_session: None,
        book_v2_vectors: Some(Vec::new()),
        precomposed_vector_metrics: Vec::new(),
        canonical_package_sha256: canonical_jcs_sha256,
        precomposed_vector_limits_fingerprint: precomposed_vector_limits_fingerprint(limits),
        limits,
    };
    validator.node(wire.document().node_id, None, 1)?;
    let document = lower_document_kind(lower_kind, wire.document(), &mut validator)?;
    let retained_text_bytes = validator.admitted_text_and_math_speech_bytes;
    let vectors = validator
        .book_v2_vectors
        .take()
        .ok_or(StagingSemanticSyntaxError::ReceiptMismatch)?;
    if !validator.precomposed_vector_metrics.is_empty() {
        return Err(StagingSemanticSyntaxError::ReceiptMismatch);
    }
    let pending_math = std::mem::take(&mut validator.math_nodes);
    let mut math = Vec::new();
    math.try_reserve_exact(pending_math.len())
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    math.extend(pending_math.into_iter().map(PreparedBookMath));
    let resources = lower_resources(wire.resources())?;
    Ok(PreparedBookV2Body {
        wire,
        document,
        resources,
        vectors,
        math,
        limits: limits.clone(),
        retained_text_bytes,
        raw_sha256,
        canonical_jcs_sha256,
    })
}

fn lower_kind(kind: WireBookV2SemanticContainerKind) -> BookV2SemanticContainerKind {
    use BookV2SemanticContainerKind as D;
    use WireBookV2SemanticContainerKind as W;
    match kind {
        W::Result => D::Result,
        W::Proof => D::Proof,
        W::Exercise => D::Exercise,
        W::Solution => D::Solution,
        W::Example => D::Example,
        W::Counterexample => D::Counterexample,
        W::Remark => D::Remark,
        W::Note => D::Note,
        W::Warning => D::Warning,
        W::CommonError => D::CommonError,
        W::FormalizationNote => D::FormalizationNote,
        W::Quote => D::Quote,
    }
}

#[cfg(test)]
#[path = "book_v2_tests.rs"]
mod tests;

/// Style closure of a prepared successor body. It still grants no host-source,
/// resource, profile, layout, structure or PDF admission.
#[derive(Debug)]
pub struct StyledBookV2Body {
    body: PreparedBookV2Body,
    containers: BTreeMap<NodeId, typaxis_style::book_v2::BookV2SemanticContainerComputedStyle>,
    vectors: BTreeMap<NodeId, PrecomposedVectorComputedStyleReceipt>,
    math: BTreeMap<NodeId, StagingMathComputedStyle>,
}
impl StyledBookV2Body {
    pub const fn body(&self) -> &PreparedBookV2Body {
        &self.body
    }
    pub fn container_style(
        &self,
        node: NodeId,
    ) -> Option<&typaxis_style::book_v2::BookV2SemanticContainerComputedStyle> {
        self.containers.get(&node)
    }
    pub fn vector_style(&self, node: NodeId) -> Option<&PrecomposedVectorComputedStyleReceipt> {
        self.vectors.get(&node)
    }
    pub fn math_style(&self, node: NodeId) -> Option<&StagingMathComputedStyle> {
        self.math.get(&node)
    }
}

/// Applies the shared closed selector/property registry and inheritance engine.
/// Container kinds are copied into the successor style vocabulary exhaustively;
/// no 1.4 semantic-container kind or syntax package is constructed here.
pub fn style_book_v2_body(
    body: PreparedBookV2Body,
) -> Result<StyledBookV2Body, StagingSemanticSyntaxError> {
    let rules = lower_semantic_style_rules(body.wire.style_sheet(), &body.limits)?;
    let mut containers = BTreeMap::new();
    let mut vectors = BTreeMap::new();
    let mut math = BTreeMap::new();
    collect_computed_styles_kind(
        cascade_book_kind,
        &body.document.blocks,
        &rules,
        None,
        &body.math,
        &mut containers,
        &mut vectors,
        &mut math,
    )?;
    for footnote in &body.document.footnotes {
        collect_computed_styles_kind(
            cascade_book_kind,
            &footnote.blocks,
            &rules,
            None,
            &body.math,
            &mut containers,
            &mut vectors,
            &mut math,
        )?;
    }
    if math.len() != body.math.len() {
        return Err(StagingSemanticSyntaxError::ReceiptMismatch);
    }
    Ok(StyledBookV2Body {
        body,
        containers,
        vectors,
        math,
    })
}

fn cascade_book_kind(
    kind: BookV2SemanticContainerKind,
    classes: &[String],
    sheet: &StyleSheet,
    parent: Option<&SemanticContainerInheritanceStyle>,
) -> Result<typaxis_style::book_v2::BookV2SemanticContainerComputedStyle, StyleValidationError> {
    use typaxis_style::book_v2::BookV2SemanticContainerStyleKind as S;
    use BookV2SemanticContainerKind as D;
    let kind = match kind {
        D::Result => S::Result,
        D::Proof => S::Proof,
        D::Exercise => S::Exercise,
        D::Solution => S::Solution,
        D::Example => S::Example,
        D::Counterexample => S::Counterexample,
        D::Remark => S::Remark,
        D::Note => S::Note,
        D::Warning => S::Warning,
        D::CommonError => S::CommonError,
        D::FormalizationNote => S::FormalizationNote,
        D::Quote => S::Quote,
    };
    typaxis_style::book_v2::cascade_book_v2_semantic_container_style(kind, classes, sheet, parent)
}

impl AsRef<StagingM4MathNode> for PreparedBookMath {
    fn as_ref(&self) -> &StagingM4MathNode {
        self.domain()
    }
}

pub use crate::book_navigation::book_v2::{
    prepare_book_v2_navigation, BookV2ReferenceTarget, PreparedBookV2Language,
    PreparedBookV2LanguageChild, PreparedBookV2Navigation,
};
