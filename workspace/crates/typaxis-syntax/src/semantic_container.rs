use super::*;
use typaxis_document::{
    FontMediaDeclaration, FontMediaType, ImageMediaDeclaration, ImageMediaType,
    PrecomposedVectorEquationNumber, PrecomposedVectorMetrics, PrecomposedVectorSourceTex,
    PrecomposedVectorSpacing, PrecomposedVectorViewport, SemanticContainerKind, StagingM4Block,
    StagingM4BlockCommon, StagingM4Document, StagingM4FigurePlacement,
    StagingM4FontFaceDeclaration, StagingM4FootnoteDefinition, StagingM4ImageDeclaration,
    StagingM4InlineVector, StagingM4InlineVectorKind, StagingM4ListItem, StagingM4MathKind,
    StagingM4MathNode, StagingM4ResourceCatalog, StagingM4TableCell, StagingM4TableRow,
    VectorProvenance,
};
use typaxis_document_package::{
    DecodedStagingSemanticDocumentPackage, WireFontMediaType, WireImageMediaType,
    WirePrecomposedVectorEquationNumber, WirePrecomposedVectorMetrics,
    WirePrecomposedVectorSourceTex, WirePrecomposedVectorSpacing, WirePrecomposedVectorViewport,
    WireStagingM4Block, WireStagingM4Document, WireStagingM4DocumentPackage, WireStagingM4Inline,
    WireStagingM4LinkTarget, WireStagingM4ResourceCatalog, WireStagingM4Source,
    WireStagingM4TextBuffer, WireStagingMathSource, WireStagingSourceSpan, WireStagingStyleSheet,
    WireStagingStyleValue, WireStagingTextMapKind, WireStagingTextSpan,
};
use typaxis_math::{parse_math_source, MathParseLimits, ParsedMathReceipt};
use typaxis_style::{
    cascade_staging_display_math_style, cascade_staging_semantic_container_style,
    cascade_staging_semantic_descendant_style, close_staging_inline_math_style,
    PrecomposedVectorComputedStyleReceipt, PrecomposedVectorStyleKind,
    SemanticContainerComputedStyle, SemanticContainerInheritanceStyle, SemanticContainerStyleKind,
    StagingMathComputedStyle,
};

const SEMANTIC_SYNTAX_FINGERPRINT_ALGORITHM: &str = "typaxis.semantic-container-syntax/1";
const STAGING_PROFILE_ID: &str = "typaxis.machine-pdf/production-book-1";
const STAGING_PROFILE_RECEIPT_ALGORITHM: &str = "typaxis.production-book-profile-receipt/1";
const INTERNAL_HIDDEN_STYLE_CLASS: &str = "__typaxis_internal_hidden";
pub const PRECOMPOSED_VECTOR_METRICS_ALGORITHM: &str = "typaxis.precomposed-vector-metrics/1";
pub const PRECOMPOSED_VECTOR_EFFECTIVE_LANGUAGE_ALGORITHM: &str =
    "typaxis.precomposed-vector-effective-language/1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PrecomposedVectorKind {
    InlineVector,
    MathVector,
    VectorFigure,
    MathVectorBlock,
}

impl PrecomposedVectorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InlineVector => "inline_vector",
            Self::MathVector => "math_vector",
            Self::VectorFigure => "vector_figure",
            Self::MathVectorBlock => "math_vector_block",
        }
    }

    const fn is_math(self) -> bool {
        matches!(self, Self::MathVector | Self::MathVectorBlock)
    }
}

/// Per-owner effective language proof used by private vector layout stages.
///
/// This is not the complete computed-language registry. The versioned `/2`
/// registry joins these same inherited values to its complete owner records;
/// this narrow receipt remains the earlier syntax proof that lets an
/// equation-number child reference its parent without becoming an independent
/// language owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedPrecomposedVectorEffectiveLanguage {
    package_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    owner: NodeId,
    kind: PrecomposedVectorKind,
    language: String,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl ValidatedPrecomposedVectorEffectiveLanguage {
    pub const fn algorithm(&self) -> &'static str {
        PRECOMPOSED_VECTOR_EFFECTIVE_LANGUAGE_ALGORITHM
    }

    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn kind(&self) -> PrecomposedVectorKind {
        self.kind
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PrecomposedVectorField {
    MetricsAdvance,
    MetricsAscent,
    MetricsBaseline,
    MetricsDescent,
    MetricsOriginX,
    MetricsViewportHeight,
    MetricsViewportWidth,
    SpacingAfter,
    SpacingBefore,
    SourceTexTextSpan,
    Alternative,
    ActualText,
    Language,
    EquationNumberMinimumGap,
    EquationNumberNodeId,
    EquationNumberSpan,
    EquationNumberTextSpan,
}

impl PrecomposedVectorField {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MetricsAdvance => "metrics.advance",
            Self::MetricsAscent => "metrics.ascent",
            Self::MetricsBaseline => "metrics.baseline",
            Self::MetricsDescent => "metrics.descent",
            Self::MetricsOriginX => "metrics.origin_x",
            Self::MetricsViewportHeight => "metrics.viewport.height",
            Self::MetricsViewportWidth => "metrics.viewport.width",
            Self::SpacingAfter => "spacing.after",
            Self::SpacingBefore => "spacing.before",
            Self::SourceTexTextSpan => "source_tex.text_span",
            Self::Alternative => "alt",
            Self::ActualText => "actual_text",
            Self::Language => "language",
            Self::EquationNumberMinimumGap => "equation_number.minimum_gap",
            Self::EquationNumberNodeId => "equation_number.node_id",
            Self::EquationNumberSpan => "equation_number.span",
            Self::EquationNumberTextSpan => "equation_number.text_span",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSemanticSyntaxError {
    InvalidNodeOrder,
    InvalidSource,
    InvalidSourceSpan,
    InvalidClass,
    InvalidNesting,
    EmptyContainer(NodeId),
    InvalidBlock(NodeId),
    InvalidInline,
    InvalidMath,
    InvalidMathSource {
        source_id: SourceId,
        byte_offset: Utf8ByteOffset,
    },
    InvalidMathSourceVersion,
    InvalidPrecomposedVector {
        owner: NodeId,
        field: PrecomposedVectorField,
    },
    PrecomposedVectorStaging(NodeId),
    SvgSafe2Staging(ImageResourceId),
    JpegStaging(ImageResourceId),
    CffStaging(FontFaceId),
    MathSourceTextLimit,
    MathSpeechLimit,
    InvalidResource,
    InvalidPageGeometry,
    InvalidStyle,
    InapplicableStyle,
    AstNodeLimit,
    AstDepthLimit,
    MathAstNodeLimit,
    MathAstDepthLimit,
    PrecomposedVectorAstNodeLimit,
    PrecomposedVectorAstDepthLimit,
    PrecomposedVectorTextBufferLimit {
        owner: NodeId,
        field: PrecomposedVectorField,
    },
    PrecomposedVectorTextAggregateLimit {
        owner: NodeId,
        field: PrecomposedVectorField,
    },
    MathLayoutUnitLimit,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingSemanticSyntaxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidNodeOrder => {
                formatter.write_str("P1102: semantic NodeIds are not dense preorder")
            }
            Self::InvalidSource => formatter.write_str("P1102: invalid semantic source catalog"),
            Self::InvalidSourceSpan => {
                formatter.write_str("P1102: semantic source span ownership mismatch")
            }
            Self::InvalidClass => {
                formatter.write_str("P1102: semantic class list is not canonical")
            }
            Self::InvalidNesting => {
                formatter.write_str("L5100: semantic_container is not allowed in this owner")
            }
            Self::EmptyContainer(owner) => write!(
                formatter,
                "L5100: recursively empty semantic_container at node {}",
                owner.get()
            ),
            Self::InvalidBlock(owner) => write!(
                formatter,
                "L5100: invalid block owned by node {}",
                owner.get()
            ),
            Self::InvalidInline => formatter.write_str("L5100: invalid semantic inline nesting"),
            Self::InvalidMath => {
                formatter.write_str("P1102: invalid typaxis-math source or speech")
            }
            Self::InvalidMathSource {
                source_id,
                byte_offset,
            } => write!(
                formatter,
                "P1102: invalid typaxis-math source at source {} byte {}",
                source_id.get(),
                byte_offset.get()
            ),
            Self::InvalidMathSourceVersion => {
                formatter.write_str("P1102: unsupported math source language/version")
            }
            Self::InvalidPrecomposedVector { owner, field } => write!(
                formatter,
                "P1102: invalid precomposed vector field `{}` at node {}",
                field.as_str(),
                owner.get(),
            ),
            Self::PrecomposedVectorStaging(owner) => write!(
                formatter,
                "P1102: precomposed vector at node {} requires the versioned vector pipeline",
                owner.get()
            ),
            Self::SvgSafe2Staging(id) => write!(
                formatter,
                "P1102: svg-safe-2 image {} requires the versioned vector pipeline",
                id.get()
            ),
            Self::JpegStaging(id) => write!(
                formatter,
                "R7100: jpeg-baseline image {} requires the JPEG profile",
                id.get()
            ),
            Self::CffStaging(id) => write!(
                formatter,
                "R7100: sfnt-cff1 font {} requires the CFF profile",
                id.get()
            ),
            Self::MathSourceTextLimit => formatter.write_str("T2100: math source limit exceeded"),
            Self::MathSpeechLimit => formatter.write_str("T2101: math speech limit exceeded"),
            Self::InvalidResource => formatter.write_str("P1102: invalid declared-media resource"),
            Self::InvalidPageGeometry => formatter
                .write_str("P1102: production-book-1 requires one closed default page frame"),
            Self::InvalidStyle => formatter.write_str("L5101: invalid production style"),
            Self::InapplicableStyle => {
                formatter.write_str("L5101: inapplicable production style property")
            }
            Self::AstNodeLimit => formatter.write_str("P1102: semantic AST exceeds max_ast_nodes"),
            Self::AstDepthLimit => {
                formatter.write_str("P1102: semantic AST exceeds max_ast_nesting_depth")
            }
            Self::MathAstNodeLimit => formatter.write_str("P1120: math AST exceeds max_ast_nodes"),
            Self::MathAstDepthLimit => {
                formatter.write_str("P1121: math AST exceeds max_ast_nesting_depth")
            }
            Self::PrecomposedVectorAstNodeLimit => {
                formatter.write_str("P1120: precomposed vector AST exceeds max_ast_nodes")
            }
            Self::PrecomposedVectorAstDepthLimit => {
                formatter.write_str("P1121: precomposed vector AST exceeds max_ast_nesting_depth")
            }
            Self::PrecomposedVectorTextBufferLimit { owner, field } => write!(
                formatter,
                "T2100: precomposed vector field `{}` at node {} exceeds max_text_buffer_bytes",
                field.as_str(),
                owner.get(),
            ),
            Self::PrecomposedVectorTextAggregateLimit { owner, field } => write!(
                formatter,
                "T2101: precomposed vector field `{}` at node {} exceeds max_text_bytes",
                field.as_str(),
                owner.get(),
            ),
            Self::MathLayoutUnitLimit => {
                formatter.write_str("L5111: math layout work limit exceeded")
            }
            Self::ReceiptMismatch => formatter.write_str("I9190: semantic syntax receipt mismatch"),
            Self::AllocationFailure => {
                formatter.write_str("P1102: semantic syntax allocation failed")
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingMathNode {
    domain: StagingM4MathNode,
    parsed: ParsedMathReceipt,
    computed_style: StagingMathComputedStyle,
}

impl ValidatedStagingMathNode {
    pub const fn domain(&self) -> &StagingM4MathNode {
        &self.domain
    }
    pub const fn parsed(&self) -> &ParsedMathReceipt {
        &self.parsed
    }
    pub const fn computed_style(&self) -> &StagingMathComputedStyle {
        &self.computed_style
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingStagingMathNode {
    domain: StagingM4MathNode,
    parsed: ParsedMathReceipt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrecomposedVectorMetricPayload {
    Inline {
        metrics: PrecomposedVectorMetrics,
        spacing: PrecomposedVectorSpacing,
    },
    MathBlock {
        metrics: PrecomposedVectorMetrics,
    },
    Figure {
        viewport: PrecomposedVectorViewport,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnresolvedPrecomposedVectorResourceBinding {
    image_id: ImageResourceId,
}

impl UnresolvedPrecomposedVectorResourceBinding {
    pub const fn image_id(self) -> ImageResourceId {
        self.image_id
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ValidatedPrecomposedVectorTextBinding {
    text_span: TextSpan,
    mapped_source_span: SourceSpan,
    text_buffer_sha256: [u8; 32],
    exact_text_sha256: [u8; 32],
}

impl ValidatedPrecomposedVectorTextBinding {
    pub const fn text_span(&self) -> TextSpan {
        self.text_span
    }
    pub const fn mapped_source_span(&self) -> SourceSpan {
        self.mapped_source_span
    }
    pub const fn text_buffer_sha256(&self) -> [u8; 32] {
        self.text_buffer_sha256
    }
    pub const fn exact_text_sha256(&self) -> [u8; 32] {
        self.exact_text_sha256
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrecomposedVectorActualTextResolution {
    Authored,
    AlternativeFallback,
    Absent,
}

impl PrecomposedVectorActualTextResolution {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Authored => "authored",
            Self::AlternativeFallback => "alternative_fallback",
            Self::Absent => "absent",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ValidatedPrecomposedVectorAlternative {
    alternative: String,
    authored_actual_text: Option<String>,
    resolution: PrecomposedVectorActualTextResolution,
}

impl ValidatedPrecomposedVectorAlternative {
    pub fn alternative(&self) -> &str {
        &self.alternative
    }
    pub fn alternative_sha256(&self) -> [u8; 32] {
        sha256(self.alternative.as_bytes())
    }
    pub fn authored_actual_text(&self) -> Option<&str> {
        self.authored_actual_text.as_deref()
    }
    pub fn authored_actual_text_sha256(&self) -> Option<[u8; 32]> {
        self.authored_actual_text
            .as_deref()
            .map(|value| sha256(value.as_bytes()))
    }
    pub const fn resolution(&self) -> PrecomposedVectorActualTextResolution {
        self.resolution
    }
    pub fn resolved_actual_text(&self) -> Option<&str> {
        match self.resolution {
            PrecomposedVectorActualTextResolution::Authored => self.authored_actual_text.as_deref(),
            PrecomposedVectorActualTextResolution::AlternativeFallback => Some(&self.alternative),
            PrecomposedVectorActualTextResolution::Absent => None,
        }
    }
    pub fn resolved_actual_text_sha256(&self) -> Option<[u8; 32]> {
        self.resolved_actual_text()
            .map(|value| sha256(value.as_bytes()))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ValidatedPrecomposedVectorLanguageOverride {
    raw: String,
    canonical: String,
    charged_bytes: u64,
}

impl ValidatedPrecomposedVectorLanguageOverride {
    pub fn raw(&self) -> &str {
        &self.raw
    }
    pub fn canonical(&self) -> &str {
        &self.canonical
    }
    /// Raw spelling is charged only when it differs from canonical spelling.
    pub const fn charged_bytes(&self) -> u64 {
        self.charged_bytes
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ValidatedPrecomposedVectorEquationNumber {
    node_id: NodeId,
    span: SourceSpan,
    minimum_gap: PositiveLength,
    text: ValidatedPrecomposedVectorTextBinding,
}

impl ValidatedPrecomposedVectorEquationNumber {
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    pub const fn minimum_gap(&self) -> PositiveLength {
        self.minimum_gap
    }
    pub const fn text(&self) -> &ValidatedPrecomposedVectorTextBinding {
        &self.text
    }
}

#[derive(Clone)]
struct PrecomposedVectorSyntaxSessionIdentity(std::sync::Arc<()>);

impl PrecomposedVectorSyntaxSessionIdentity {
    fn fresh() -> Self {
        Self(std::sync::Arc::new(()))
    }
}

impl std::fmt::Debug for PrecomposedVectorSyntaxSessionIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PrecomposedVectorSyntaxSessionIdentity(..)")
    }
}

impl PartialEq for PrecomposedVectorSyntaxSessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for PrecomposedVectorSyntaxSessionIdentity {}

/// Syntax-owned, session-bound proof for one precomposed vector owner.
///
/// This type deliberately does not implement `Clone`. Construction is private
/// to the semantic parser, and downstream stages must ask the owning package to
/// verify a borrowed receipt before consuming it.
///
/// ```compile_fail
/// use typaxis_syntax::ValidatedPrecomposedVectorMetrics;
///
/// fn duplicate(receipt: ValidatedPrecomposedVectorMetrics) {
///     let _second = receipt.clone();
/// }
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct ValidatedPrecomposedVectorMetrics {
    package_sha256: [u8; 32],
    session: PrecomposedVectorSyntaxSessionIdentity,
    limits_fingerprint: [u8; 32],
    node_id: NodeId,
    owner_source_span: SourceSpan,
    kind: PrecomposedVectorKind,
    resource: UnresolvedPrecomposedVectorResourceBinding,
    payload: PrecomposedVectorMetricPayload,
    source_tex: Option<ValidatedPrecomposedVectorTextBinding>,
    alternative: ValidatedPrecomposedVectorAlternative,
    language: Option<ValidatedPrecomposedVectorLanguageOverride>,
    equation_number: Option<ValidatedPrecomposedVectorEquationNumber>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl ValidatedPrecomposedVectorMetrics {
    pub const fn algorithm(&self) -> &'static str {
        PRECOMPOSED_VECTOR_METRICS_ALGORITHM
    }
    pub const fn contract(&self) -> &'static str {
        typaxis_document_package::STAGING_SEMANTIC_DOCUMENT_PACKAGE_CONTRACT
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn owner_source_span(&self) -> SourceSpan {
        self.owner_source_span
    }
    pub const fn kind(&self) -> PrecomposedVectorKind {
        self.kind
    }
    pub const fn resource_binding(&self) -> UnresolvedPrecomposedVectorResourceBinding {
        self.resource
    }
    pub const fn payload(&self) -> PrecomposedVectorMetricPayload {
        self.payload
    }
    pub const fn source_tex(&self) -> Option<&ValidatedPrecomposedVectorTextBinding> {
        self.source_tex.as_ref()
    }
    pub const fn alternative(&self) -> &ValidatedPrecomposedVectorAlternative {
        &self.alternative
    }
    pub const fn language(&self) -> Option<&ValidatedPrecomposedVectorLanguageOverride> {
        self.language.as_ref()
    }
    pub const fn equation_number(&self) -> Option<&ValidatedPrecomposedVectorEquationNumber> {
        self.equation_number.as_ref()
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    fn verify_integrity(
        &self,
        package_sha256: [u8; 32],
        limits_fingerprint: [u8; 32],
        session: &PrecomposedVectorSyntaxSessionIdentity,
    ) -> bool {
        if self.package_sha256 != package_sha256
            || self.limits_fingerprint != limits_fingerprint
            || &self.session != session
        {
            return false;
        }
        let observed = encode_precomposed_vector_metrics_receipt(self);
        observed == self.canonical_jcs && sha256(observed.as_bytes()) == self.fingerprint
    }
}

impl std::error::Error for StagingSemanticSyntaxError {}

/// Syntax-owned proof of the complete contract-1.4 semantic and declared-media
/// lowering. The original typed carrier is retained for a checked canonical
/// re-encode; no public contract decoder can consume it.
#[derive(Debug)]
pub struct ValidatedStagingSemanticPackage {
    wire: WireStagingM4DocumentPackage,
    limits: ValidatedResourceLimits,
    precomposed_vector_session: PrecomposedVectorSyntaxSessionIdentity,
    precomposed_vector_metrics: Vec<ValidatedPrecomposedVectorMetrics>,
    document: StagingM4Document,
    resources: StagingM4ResourceCatalog,
    computed_styles: BTreeMap<NodeId, SemanticContainerComputedStyle>,
    precomposed_vector_styles: BTreeMap<NodeId, PrecomposedVectorComputedStyleReceipt>,
    math_nodes: Vec<ValidatedStagingMathNode>,
    raw_sha256: [u8; 32],
    canonical_jcs_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    semantic_jcs: String,
}

/// Public contract-1.4 syntax result that retains the exact host-admission
/// provenance used to decode and stable-read the package and its source.
///
/// Keeping this wrapper distinct from the staging semantic package prevents a
/// decoder-only value from crossing the public production-book trust boundary.
#[derive(Debug)]
pub struct ValidatedProductionMachinePackage {
    package: ValidatedStagingSemanticPackage,
    provenance: MachineInputAdmissionProvenance,
}

impl ValidatedProductionMachinePackage {
    pub const fn package(&self) -> &ValidatedStagingSemanticPackage {
        &self.package
    }

    pub const fn provenance(&self) -> &MachineInputAdmissionProvenance {
        &self.provenance
    }

    pub const fn contract(&self) -> typaxis_core::DocumentPackageContractId {
        typaxis_core::DocumentPackageContractId::V1_4
    }
}

/// Result of consuming a host-admitted contract-1.4 package at the syntax
/// boundary. Failure returns only the sealed progress receipt; a caller cannot
/// recover or replay the decoder-owned carrier.
#[derive(Debug)]
pub enum ProductionMachineParseOutcome {
    Parsed {
        package: Box<ValidatedProductionMachinePackage>,
    },
    Failed {
        progress: Box<MachineInputProgress>,
        failure: StagingSemanticSyntaxError,
    },
}

/// Dependency-inversion view of the profile-owned authorization consumed by
/// downstream staging phases. Its private fields prevent callers from
/// implementing a look-alike receipt; construction rechecks the fixed M4
/// domain and exact effective limits before producing the profile fingerprint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerProfileView {
    package_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    limits: ValidatedResourceLimits,
    container_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSemanticContainerProfileView {
    pub fn new(
        package: &ValidatedStagingSemanticPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        Self::new_with_media_policy(package, limits, false, false, false)
    }

    fn new_with_jpeg_policy(
        package: &ValidatedStagingSemanticPackage,
        limits: &ValidatedResourceLimits,
        permits_jpeg: bool,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        Self::new_with_media_policy(package, limits, permits_jpeg, false, false)
    }

    fn new_with_media_policy(
        package: &ValidatedStagingSemanticPackage,
        limits: &ValidatedResourceLimits,
        permits_jpeg: bool,
        permits_cff: bool,
        permits_precomposed: bool,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        package.checked_wire()?;
        if package.limits() != limits {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        if !permits_precomposed {
            if let Some(image) = package.resources.images.iter().find(|image| {
                image.media == ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2)
            }) {
                return Err(StagingSemanticSyntaxError::SvgSafe2Staging(image.image_id));
            }
        }
        if !permits_jpeg {
            if let Some(image) = package.resources.images.iter().find(|image| {
                image.media == ImageMediaDeclaration::Declared(ImageMediaType::JpegBaseline)
            }) {
                return Err(StagingSemanticSyntaxError::JpegStaging(image.image_id));
            }
        }
        if !permits_cff {
            if let Some(font) =
                package.resources.font_faces.iter().find(|font| {
                    font.media == FontMediaDeclaration::Declared(FontMediaType::SfntCff1)
                })
            {
                return Err(StagingSemanticSyntaxError::CffStaging(font.font_face_id));
            }
        }
        if !permits_precomposed {
            if let Some(owner) =
                first_precomposed_vector_owner(&package.document.blocks).or_else(|| {
                    package
                        .document
                        .footnotes
                        .iter()
                        .find_map(|footnote| first_precomposed_vector_owner(&footnote.blocks))
                })
            {
                return Err(StagingSemanticSyntaxError::PrecomposedVectorStaging(owner));
            }
        }
        let mut container_count = 0u32;
        validate_profile_container_domain(
            &package.document.blocks,
            &mut container_count,
            permits_precomposed,
        )?;
        for footnote in &package.document.footnotes {
            validate_profile_container_domain(
                &footnote.blocks,
                &mut container_count,
                permits_precomposed,
            )?;
        }
        if usize::try_from(container_count) != Ok(package.semantic_container_count()) {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        let canonical_jcs = encode_profile_view(package, limits, container_count);
        Ok(Self {
            package_sha256: package.canonical_jcs_sha256(),
            semantic_fingerprint: package.semantic_fingerprint(),
            limits: limits.clone(),
            container_count,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }

    pub const fn semantic_fingerprint(&self) -> [u8; 32] {
        self.semantic_fingerprint
    }

    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }

    pub const fn container_count(&self) -> u32 {
        self.container_count
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

/// Dependency-inversion projection for the MI4-12 CFF component. It is the
/// only production syntax authorization that admits `sfnt-cff1`; all older
/// contract/profile views fail before resource open.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingCffProfileView {
    base: StagingSemanticContainerProfileView,
    limits_fingerprint: [u8; 32],
    font_face_ids: Vec<FontFaceId>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingCffProfileView {
    pub fn new(
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        let base = StagingSemanticContainerProfileView::new_with_media_policy(
            package,
            limits.base(),
            false,
            true,
            false,
        )?;
        if !package.resources().images.is_empty() {
            return Err(StagingSemanticSyntaxError::InvalidResource);
        }
        let mut font_face_ids = Vec::new();
        font_face_ids
            .try_reserve_exact(package.resources().font_faces.len())
            .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
        for font in &package.resources().font_faces {
            if font.media != FontMediaDeclaration::Declared(FontMediaType::SfntCff1)
                || font.face_index != 0
            {
                return Err(StagingSemanticSyntaxError::InvalidResource);
            }
            font_face_ids.push(font.font_face_id);
        }
        if font_face_ids.is_empty() || font_face_ids.windows(2).any(|ids| ids[0] >= ids[1]) {
            return Err(StagingSemanticSyntaxError::InvalidResource);
        }
        let mut canonical_jcs = String::from(
            "{\"algorithm\":\"typaxis.production-book-cff-authorization/1\",\"base_profile_fingerprint\":",
        );
        push_hash(&mut canonical_jcs, base.profile_fingerprint());
        canonical_jcs.push_str(",\"font_face_ids\":[");
        for (index, id) in font_face_ids.iter().enumerate() {
            if index > 0 {
                canonical_jcs.push(',');
            }
            canonical_jcs.push_str(&id.get().to_string());
        }
        canonical_jcs.push_str("],\"limits_fingerprint\":");
        push_hash(&mut canonical_jcs, limits.fingerprint());
        canonical_jcs.push_str(",\"package_fingerprint\":");
        push_hash(&mut canonical_jcs, package.semantic_fingerprint());
        canonical_jcs.push('}');
        Ok(Self {
            base,
            limits_fingerprint: limits.fingerprint(),
            font_face_ids,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn base(&self) -> &StagingSemanticContainerProfileView {
        &self.base
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub fn font_face_ids(&self) -> &[FontFaceId] {
        &self.font_face_ids
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn authorizes(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSemanticSyntaxError> {
        let expected = Self::new(package, limits)?;
        if *self == expected {
            Ok(())
        } else {
            Err(StagingSemanticSyntaxError::ReceiptMismatch)
        }
    }
}

#[derive(Clone)]
pub struct StagingPrecomposedVectorProfileSessionIdentity(std::sync::Arc<()>);

impl StagingPrecomposedVectorProfileSessionIdentity {
    pub fn fresh() -> Self {
        Self(std::sync::Arc::new(()))
    }
}

impl std::fmt::Debug for StagingPrecomposedVectorProfileSessionIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StagingPrecomposedVectorProfileSessionIdentity(..)")
    }
}

impl PartialEq for StagingPrecomposedVectorProfileSessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for StagingPrecomposedVectorProfileSessionIdentity {}

/// Session-bound dependency-inversion projection issued only after the
/// machine-profile owner has accepted the complete precomposed-vector policy.
/// The deterministic portion closes the exact syntax receipts and effective
/// limits; the machine-profile receipt fingerprint closes the policy itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPrecomposedVectorProfileAuthorization {
    package_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    vector_bindings: Vec<(NodeId, [u8; 32])>,
    page_geometry: StagingM4PageGeometry,
    profile_receipt_fingerprint: [u8; 32],
    session: StagingPrecomposedVectorProfileSessionIdentity,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPrecomposedVectorProfileProgressToken {
    session: StagingPrecomposedVectorProfileSessionIdentity,
    authorization_fingerprint: [u8; 32],
    profile_receipt_fingerprint: [u8; 32],
}

impl StagingPrecomposedVectorProfileAuthorization {
    #[doc(hidden)]
    pub fn bind_profile_receipt(
        profile_receipt_fingerprint: [u8; 32],
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
        session: &StagingPrecomposedVectorProfileSessionIdentity,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        if profile_receipt_fingerprint == [0; 32] {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        let vector_bindings = precomposed_vector_profile_bindings(package, limits)?;
        let page_geometry =
            StagingM4PageGeometry::from_wire(package.checked_wire()?.page_masters())?;
        let canonical_jcs =
            encode_precomposed_vector_profile_authorization(package, limits, &vector_bindings);
        let authorization = Self {
            package_sha256: package.canonical_jcs_sha256(),
            semantic_fingerprint: package.semantic_fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            vector_bindings,
            page_geometry,
            profile_receipt_fingerprint,
            session: session.clone(),
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        };
        authorization.authorizes(package, limits)?;
        Ok(authorization)
    }

    pub fn vector_owners(&self) -> impl ExactSizeIterator<Item = NodeId> + '_ {
        self.vector_bindings.iter().map(|(owner, _)| *owner)
    }

    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub const fn profile_receipt_fingerprint(&self) -> [u8; 32] {
        self.profile_receipt_fingerprint
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    /// Page geometry is a derived copy of the exact package already bound by
    /// this authorization. It is intentionally not a new profile input.
    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        &self.page_geometry
    }

    pub fn progress_token(&self) -> StagingPrecomposedVectorProfileProgressToken {
        StagingPrecomposedVectorProfileProgressToken {
            session: self.session.clone(),
            authorization_fingerprint: self.fingerprint,
            profile_receipt_fingerprint: self.profile_receipt_fingerprint,
        }
    }

    pub fn matches_progress(&self, token: &StagingPrecomposedVectorProfileProgressToken) -> bool {
        self.session == token.session
            && self.fingerprint == token.authorization_fingerprint
            && self.profile_receipt_fingerprint == token.profile_receipt_fingerprint
    }

    #[doc(hidden)]
    pub fn belongs_to_session(
        &self,
        session: &StagingPrecomposedVectorProfileSessionIdentity,
    ) -> bool {
        self.session == *session
    }

    pub fn authorizes(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSemanticSyntaxError> {
        let vector_bindings = precomposed_vector_profile_bindings(package, limits)?;
        let page_geometry =
            StagingM4PageGeometry::from_wire(package.checked_wire()?.page_masters())?;
        let canonical_jcs =
            encode_precomposed_vector_profile_authorization(package, limits, &vector_bindings);
        if self.package_sha256 != package.canonical_jcs_sha256()
            || self.semantic_fingerprint != package.semantic_fingerprint()
            || self.limits_fingerprint != limits.fingerprint()
            || self.vector_bindings != vector_bindings
            || self.page_geometry != page_geometry
            || self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
        {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        Ok(())
    }
}

fn precomposed_vector_profile_bindings(
    package: &ValidatedStagingSemanticPackage,
    limits: &M4EffectiveResourceLimits,
) -> Result<Vec<(NodeId, [u8; 32])>, StagingSemanticSyntaxError> {
    package.checked_wire()?;
    if package.limits() != limits.base() {
        return Err(StagingSemanticSyntaxError::ReceiptMismatch);
    }
    let mut bindings = Vec::new();
    bindings
        .try_reserve_exact(package.precomposed_vector_metrics().len())
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    for metrics in package.precomposed_vector_metrics() {
        package.verify_precomposed_vector_metrics(metrics)?;
        bindings.push((metrics.node_id(), metrics.fingerprint()));
    }
    if bindings.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
        return Err(StagingSemanticSyntaxError::ReceiptMismatch);
    }
    Ok(bindings)
}

fn encode_precomposed_vector_profile_authorization(
    package: &ValidatedStagingSemanticPackage,
    limits: &M4EffectiveResourceLimits,
    vector_bindings: &[(NodeId, [u8; 32])],
) -> String {
    let mut output = String::from(
        "{\"algorithm\":\"typaxis.precomposed-vector-profile-authorization/1\",\"limits_fingerprint\":",
    );
    push_hash(&mut output, limits.fingerprint());
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, package.canonical_jcs_sha256());
    output.push_str(",\"semantic_fingerprint\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push_str(",\"vector_bindings\":[");
    for (index, (owner, metrics)) in vector_bindings.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"metrics_fingerprint\":");
        push_hash(&mut output, *metrics);
        output.push_str(",\"node_id\":");
        output.push_str(&owner.get().to_string());
        output.push('}');
    }
    output.push_str("]}");
    output
}

/// Dependency-inversion view retained by the profile owner and consumed by
/// downstream private stages without a reverse dependency on
/// `typaxis-machine-profile`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorProfileView {
    base: StagingSemanticContainerProfileView,
    limits_fingerprint: [u8; 32],
    vector_resource_ids: Vec<ImageResourceId>,
    figure_owners: Vec<NodeId>,
    page_geometry: StagingM4PageGeometry,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingM4PageGeometry {
    master_id: MasterId,
    page_width: PositiveLength,
    page_height: PositiveLength,
    body: Rect,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingM4PageGeometry {
    fn from_wire(
        page_masters: &typaxis_document_package::WirePageMasterSet,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        if page_masters.masters.len() != 1 || !page_masters.selection_rules.is_empty() {
            return Err(StagingSemanticSyntaxError::InvalidPageGeometry);
        }
        let master = &page_masters.masters[0];
        if master.master_id != page_masters.default_master_id {
            return Err(StagingSemanticSyntaxError::InvalidPageGeometry);
        }
        let page_width = positive_length(master.width)?;
        let page_height = positive_length(master.height)?;
        let body_width = positive_length(master.body.width)?;
        let body_height = positive_length(master.body.height)?;
        let body_x = Length::from_raw(master.body.x)
            .filter(|value| value.raw() >= 0)
            .ok_or(StagingSemanticSyntaxError::InvalidPageGeometry)?;
        let body_y = Length::from_raw(master.body.y)
            .filter(|value| value.raw() >= 0)
            .ok_or(StagingSemanticSyntaxError::InvalidPageGeometry)?;
        if body_x
            .raw()
            .checked_add(body_width.get().raw())
            .map_or(true, |right| right > page_width.get().raw())
            || body_y
                .raw()
                .checked_add(body_height.get().raw())
                .map_or(true, |bottom| bottom > page_height.get().raw())
        {
            return Err(StagingSemanticSyntaxError::InvalidPageGeometry);
        }
        let master_id = MasterId::new(master.master_id.clone())
            .map_err(|_| StagingSemanticSyntaxError::InvalidPageGeometry)?;
        let body = Rect::new(body_x, body_y, body_width, body_height);
        let canonical_jcs = encode_page_geometry(&master_id, page_width, page_height, body);
        Ok(Self {
            master_id,
            page_width,
            page_height,
            body,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn page_width(&self) -> PositiveLength {
        self.page_width
    }
    pub const fn page_height(&self) -> PositiveLength {
        self.page_height
    }
    pub const fn body(&self) -> Rect {
        self.body
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// Closed Figure use accepted by the private baseline-JPEG component.  The
/// profile intentionally admits only image-only, non-floating Figure blocks;
/// a later production-profile composition can add text/caption layout without
/// weakening this receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegFigureProfileUse {
    owner: NodeId,
    image_id: ImageResourceId,
    alternative: String,
    source_span: SourceSpan,
    page_break_before: bool,
}

impl StagingJpegFigureProfileUse {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub fn alternative(&self) -> &str {
        &self.alternative
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn page_break_before(&self) -> bool {
        self.page_break_before
    }
}

/// Dependency-inversion projection for MI4-11.  It binds the exact private
/// contract package, JPEG resource set, Figure use order, page geometry, and
/// effective limits before the host is permitted to open image resources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegProfileView {
    base: StagingSemanticContainerProfileView,
    limits_fingerprint: [u8; 32],
    jpeg_resource_ids: Vec<ImageResourceId>,
    figures: Vec<StagingJpegFigureProfileUse>,
    page_geometry: StagingM4PageGeometry,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingJpegProfileView {
    pub fn new(
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        let base = StagingSemanticContainerProfileView::new_with_jpeg_policy(
            package,
            limits.base(),
            true,
        )?;
        let wire = package.checked_wire()?;
        validate_jpeg_profile_styles(wire.style_sheet())?;
        let page_geometry = StagingM4PageGeometry::from_wire(wire.page_masters())?;
        validate_jpeg_profile_page_master_extensions(
            wire.page_masters(),
            wire.advanced_page_masters(),
        )?;
        if !package.document().footnotes.is_empty() {
            return Err(StagingSemanticSyntaxError::InvalidNesting);
        }
        if !package.resources().font_faces.is_empty() {
            return Err(StagingSemanticSyntaxError::InvalidResource);
        }
        let mut jpeg_resource_ids = Vec::new();
        jpeg_resource_ids
            .try_reserve_exact(package.resources().images.len())
            .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
        for image in &package.resources().images {
            match image.media {
                ImageMediaDeclaration::Declared(ImageMediaType::JpegBaseline) => {
                    jpeg_resource_ids.push(image.image_id)
                }
                _ => return Err(StagingSemanticSyntaxError::InvalidResource),
            }
        }
        if jpeg_resource_ids.is_empty() || jpeg_resource_ids.windows(2).any(|ids| ids[0] >= ids[1])
        {
            return Err(StagingSemanticSyntaxError::InvalidResource);
        }
        let resources: BTreeSet<_> = jpeg_resource_ids.iter().copied().collect();
        let mut figures = Vec::new();
        let mut page_break_before = false;
        collect_jpeg_profile_figures(
            &package.document().blocks,
            &resources,
            &mut page_break_before,
            &mut figures,
        )?;
        if figures.is_empty() || page_break_before {
            return Err(StagingSemanticSyntaxError::InvalidNesting);
        }
        let canonical_jcs = encode_jpeg_profile_view(
            package,
            base.profile_fingerprint(),
            limits.fingerprint(),
            &jpeg_resource_ids,
            &figures,
            page_geometry.fingerprint(),
        );
        Ok(Self {
            base,
            limits_fingerprint: limits.fingerprint(),
            jpeg_resource_ids,
            figures,
            page_geometry,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn base(&self) -> &StagingSemanticContainerProfileView {
        &self.base
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub fn jpeg_resource_ids(&self) -> &[ImageResourceId] {
        &self.jpeg_resource_ids
    }
    pub fn figures(&self) -> &[StagingJpegFigureProfileUse] {
        &self.figures
    }
    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        &self.page_geometry
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn authorizes(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSemanticSyntaxError> {
        let expected = Self::new(package, limits)?;
        if self == &expected {
            Ok(())
        } else {
            Err(StagingSemanticSyntaxError::ReceiptMismatch)
        }
    }
}

fn collect_jpeg_profile_figures(
    blocks: &[StagingM4Block],
    resources: &BTreeSet<ImageResourceId>,
    page_break_before: &mut bool,
    output: &mut Vec<StagingJpegFigureProfileUse>,
) -> Result<(), StagingSemanticSyntaxError> {
    for block in blocks {
        match block {
            StagingM4Block::Figure {
                common,
                image_id,
                placement,
                alternative,
                has_nonempty_alternative,
                caption,
            } => {
                if !common.classes.is_empty()
                    || *placement != StagingM4FigurePlacement::Block
                    || !*has_nonempty_alternative
                    || alternative.is_empty()
                    || !caption.is_empty()
                    || !resources.contains(image_id)
                {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                output
                    .try_reserve(1)
                    .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
                output.push(StagingJpegFigureProfileUse {
                    owner: common.node_id,
                    image_id: *image_id,
                    alternative: alternative.clone(),
                    source_span: common.span,
                    page_break_before: std::mem::take(page_break_before),
                });
            }
            StagingM4Block::PageBreak { common } => {
                if !common.classes.is_empty() || *page_break_before {
                    return Err(StagingSemanticSyntaxError::InvalidNesting);
                }
                *page_break_before = true;
            }
            StagingM4Block::SemanticContainer { common, blocks, .. } => {
                if !common.classes.is_empty() {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                collect_jpeg_profile_figures(blocks, resources, page_break_before, output)?
            }
            other => {
                return Err(StagingSemanticSyntaxError::InvalidBlock(
                    other.common().node_id,
                ))
            }
        }
    }
    Ok(())
}

/// The private JPEG slice has an explicit body-width placement policy and no
/// style-layout input. Accepting an authored Figure rule or class here would
/// make a valid declaration appear to work while silently ignoring it. Keep
/// the profile closed to absent styles or declarations that are exactly the
/// neutral semantic-container defaults used by the staging fixture.
fn validate_jpeg_profile_styles(
    sheet: &WireStagingStyleSheet,
) -> Result<(), StagingSemanticSyntaxError> {
    for rule in &sheet.rules {
        if rule.extends.is_some()
            || rule.selector != "semantic_container"
            || rule
                .declarations
                .iter()
                .any(|declaration| !jpeg_profile_declaration_is_neutral(declaration))
        {
            return Err(StagingSemanticSyntaxError::InapplicableStyle);
        }
    }
    Ok(())
}

/// The dedicated JPEG paginator consumes only a single body rectangle. Close
/// every page-master field that it does not consume so a header, footer,
/// footnote region, trim override, or column request can never be accepted and
/// then disappear from the rendered document.
fn validate_jpeg_profile_page_master_extensions(
    base: &typaxis_document_package::WirePageMasterSet,
    advanced: &typaxis_document_package::WireAdvancedPageMasterSet,
) -> Result<(), StagingSemanticSyntaxError> {
    let [master] = base.masters.as_slice() else {
        return Err(StagingSemanticSyntaxError::InvalidPageGeometry);
    };
    let [extension] = advanced.masters.as_slice() else {
        return Err(StagingSemanticSyntaxError::InvalidPageGeometry);
    };
    let expected_trim = typaxis_document_package::WireRect {
        x: 0,
        y: 0,
        width: master.width,
        height: master.height,
    };
    if master.header.is_some()
        || master.footer.is_some()
        || master.footnote.is_some()
        || extension.master_id != master.master_id
        || extension.trim != expected_trim
        || extension.header_content.is_some()
        || extension.footer_content.is_some()
        || extension.column_layout.is_some()
        || !matches!(
            advanced.page_progression,
            typaxis_document_package::WirePageProgression::LeftToRight
        )
        || !matches!(
            advanced.writing_mode,
            typaxis_document_package::WirePageWritingMode::HorizontalTopToBottom
        )
    {
        return Err(StagingSemanticSyntaxError::InvalidPageGeometry);
    }
    Ok(())
}

fn jpeg_profile_declaration_is_neutral(
    declaration: &typaxis_document_package::WireStagingStyleDeclaration,
) -> bool {
    match (declaration.name.as_str(), &declaration.value) {
        (
            "space_before" | "space_after" | "start_indent" | "end_indent",
            WireStagingStyleValue::Length { value: 0 },
        )
        | ("keep_with_next", WireStagingStyleValue::Boolean { value: false })
        | ("keep_caption", WireStagingStyleValue::Boolean { value: true }) => true,
        ("text_align", WireStagingStyleValue::Keyword { value }) => value == "start",
        ("width" | "page", WireStagingStyleValue::Keyword { value }) => value == "auto",
        _ => false,
    }
}

fn encode_jpeg_profile_view(
    package: &ValidatedStagingSemanticPackage,
    base: [u8; 32],
    limits: [u8; 32],
    resources: &[ImageResourceId],
    figures: &[StagingJpegFigureProfileUse],
    page_geometry: [u8; 32],
) -> String {
    let mut output = String::from(
        "{\"algorithm\":\"typaxis.production-book-jpeg-authorization/1\",\"base_profile_fingerprint\":",
    );
    push_hash(&mut output, base);
    output.push_str(",\"figures\":[");
    for (index, figure) in figures.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"alternative_sha256\":");
        push_hash(&mut output, sha256(figure.alternative.as_bytes()));
        output.push_str(",\"image_id\":");
        output.push_str(&figure.image_id.get().to_string());
        output.push_str(",\"node_id\":");
        output.push_str(&figure.owner.get().to_string());
        output.push_str(",\"page_break_before\":");
        output.push_str(if figure.page_break_before {
            "true"
        } else {
            "false"
        });
        output.push_str(",\"span\":{");
        output.push_str("\"end_byte\":");
        output.push_str(&figure.source_span.end_byte().get().to_string());
        output.push_str(",\"source_id\":");
        output.push_str(&figure.source_span.source_id().get().to_string());
        output.push_str(",\"start_byte\":");
        output.push_str(&figure.source_span.start_byte().get().to_string());
        output.push_str("}}");
    }
    output.push_str("],\"limits_fingerprint\":");
    push_hash(&mut output, limits);
    output.push_str(",\"package_fingerprint\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push_str(",\"page_geometry_fingerprint\":");
    push_hash(&mut output, page_geometry);
    output.push_str(",\"resources\":[");
    for (index, resource) in resources.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&resource.get().to_string());
    }
    output.push_str("]}");
    output
}

fn positive_length(raw: i64) -> Result<PositiveLength, StagingSemanticSyntaxError> {
    Length::from_raw(raw)
        .and_then(PositiveLength::new)
        .ok_or(StagingSemanticSyntaxError::InvalidPageGeometry)
}

fn encode_page_geometry(
    master_id: &MasterId,
    page_width: PositiveLength,
    page_height: PositiveLength,
    body: Rect,
) -> String {
    let mut output = String::from(
        "{\"algorithm\":\"typaxis.safe-vector-page-geometry/1\",\"body\":{\"height\":",
    );
    output.push_str(&body.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&body.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&body.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&body.y().raw().to_string());
    output.push_str("},\"master_id\":");
    push_jcs_string(&mut output, master_id.as_str());
    output.push_str(",\"page_height\":");
    output.push_str(&page_height.get().raw().to_string());
    output.push_str(",\"page_width\":");
    output.push_str(&page_width.get().raw().to_string());
    output.push('}');
    output
}

impl StagingSafeVectorProfileView {
    pub fn new(
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        let base = StagingSemanticContainerProfileView::new(package, limits.base())?;
        let page_geometry =
            StagingM4PageGeometry::from_wire(package.checked_wire()?.page_masters())?;
        let vector_resource_ids: Vec<_> = package
            .resources()
            .images
            .iter()
            .filter_map(|image| {
                (image.media == ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1))
                    .then_some(image.image_id)
            })
            .collect();
        let vector_set: BTreeSet<_> = vector_resource_ids.iter().copied().collect();
        let mut figure_owners = Vec::new();
        collect_vector_figure_owners(
            &package.document().blocks,
            package,
            &vector_set,
            &mut figure_owners,
            true,
        )?;
        for footnote in &package.document().footnotes {
            collect_vector_figure_owners(
                &footnote.blocks,
                package,
                &vector_set,
                &mut figure_owners,
                true,
            )?;
        }
        let canonical_jcs = encode_safe_vector_profile_view(
            package,
            base.profile_fingerprint(),
            limits.fingerprint(),
            &vector_resource_ids,
            &figure_owners,
            page_geometry.fingerprint(),
        );
        Ok(Self {
            base,
            limits_fingerprint: limits.fingerprint(),
            vector_resource_ids,
            figure_owners,
            page_geometry,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    fn new_for_production(
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        let base = StagingSemanticContainerProfileView::new_with_media_policy(
            package,
            limits.base(),
            true,
            true,
            true,
        )?;
        let page_geometry =
            StagingM4PageGeometry::from_wire(package.checked_wire()?.page_masters())?;
        let vector_resource_ids = package
            .resources()
            .images
            .iter()
            .filter_map(|image| {
                (image.media == ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1))
                    .then_some(image.image_id)
            })
            .collect::<Vec<_>>();
        let vector_set = vector_resource_ids.iter().copied().collect::<BTreeSet<_>>();
        let mut figure_owners = Vec::new();
        collect_vector_figure_owners(
            &package.document().blocks,
            package,
            &vector_set,
            &mut figure_owners,
            false,
        )?;
        for footnote in &package.document().footnotes {
            collect_vector_figure_owners(
                &footnote.blocks,
                package,
                &vector_set,
                &mut figure_owners,
                false,
            )?;
        }
        let canonical_jcs = encode_safe_vector_profile_view(
            package,
            base.profile_fingerprint(),
            limits.fingerprint(),
            &vector_resource_ids,
            &figure_owners,
            page_geometry.fingerprint(),
        );
        Ok(Self {
            base,
            limits_fingerprint: limits.fingerprint(),
            vector_resource_ids,
            figure_owners,
            page_geometry,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn base(&self) -> &StagingSemanticContainerProfileView {
        &self.base
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub fn vector_resource_ids(&self) -> &[ImageResourceId] {
        &self.vector_resource_ids
    }
    pub fn figure_owners(&self) -> &[NodeId] {
        &self.figure_owners
    }
    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        &self.page_geometry
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// Closed production-book authorization for the MI4-05 math slice.
/// Wrapping the SafeVector authorization proves that the target's required
/// vector-media policy and page geometry were preflighted as one domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathProfileView {
    base: StagingSafeVectorProfileView,
    production: bool,
    math_node_ids: Vec<NodeId>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathProfileView {
    pub fn new(
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        Self::new_with_mode(package, limits, false)
    }

    pub fn new_for_production(
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        Self::new_with_mode(package, limits, true)
    }

    fn new_with_mode(
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
        production: bool,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        let base = if production {
            StagingSafeVectorProfileView::new_for_production(package, limits)?
        } else {
            StagingSafeVectorProfileView::new(package, limits)?
        };
        if package.math_nodes().is_empty() || package.resources().font_faces.is_empty() {
            return Err(StagingSemanticSyntaxError::InvalidNesting);
        }
        let mut math_node_ids = Vec::new();
        math_node_ids
            .try_reserve_exact(package.math_nodes().len())
            .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
        let mut previous = None;
        for value in package.math_nodes() {
            let domain = value.domain();
            if domain.language != typaxis_math::MATH_SOURCE_LANGUAGE
                || domain.version != typaxis_math::MATH_SOURCE_VERSION
                || previous.is_some_and(|node_id| node_id >= domain.node_id)
                || domain.speech.is_empty()
            {
                return Err(StagingSemanticSyntaxError::InvalidMathSourceVersion);
            }
            previous = Some(domain.node_id);
            math_node_ids.push(domain.node_id);
        }
        let mut canonical_jcs = String::from(
            "{\"algorithm\":\"typaxis.production-book-math-authorization/1\",\"base_profile_fingerprint\":",
        );
        push_hash(&mut canonical_jcs, base.profile_fingerprint());
        canonical_jcs.push_str(",\"math\":[");
        for (index, value) in package.math_nodes().iter().enumerate() {
            if index > 0 {
                canonical_jcs.push(',');
            }
            canonical_jcs.push_str("{\"kind\":");
            push_jcs_string(&mut canonical_jcs, value.domain().kind.as_str());
            canonical_jcs.push_str(",\"language\":\"typaxis-math\",\"node_id\":");
            canonical_jcs.push_str(&value.domain().node_id.get().to_string());
            canonical_jcs.push_str(",\"speech_sha256\":");
            push_hash(&mut canonical_jcs, sha256(value.domain().speech.as_bytes()));
            canonical_jcs.push_str(",\"version\":\"1\"}");
        }
        canonical_jcs.push_str("],\"package_fingerprint\":");
        push_hash(&mut canonical_jcs, package.semantic_fingerprint());
        canonical_jcs.push('}');
        Ok(Self {
            base,
            production,
            math_node_ids,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn base(&self) -> &StagingSafeVectorProfileView {
        &self.base
    }
    pub const fn is_production(&self) -> bool {
        self.production
    }
    pub fn math_node_ids(&self) -> &[NodeId] {
        &self.math_node_ids
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        self.base.page_geometry()
    }
    pub const fn ast_fingerprint_algorithm(&self) -> &'static str {
        typaxis_math::MATH_AST_FINGERPRINT_ID
    }
    pub const fn formatter(&self) -> &'static str {
        typaxis_math::MATH_FORMATTER_ID
    }
    pub const fn layout_algorithm(&self) -> &'static str {
        typaxis_math::MATH_COMPUTATION_ID
    }
    pub const fn layout_work_algorithm(&self) -> &'static str {
        typaxis_math::MATH_LAYOUT_WORK_ID
    }
    pub const fn parser(&self) -> &'static str {
        typaxis_math::MATH_PARSER_ID
    }
    pub const fn source_identity(&self) -> &'static str {
        typaxis_math::MATH_SOURCE_ID
    }
    pub const fn vector_algorithm(&self) -> &'static str {
        typaxis_math::MATH_VECTOR_IR_ID
    }
}

#[derive(Debug)]
struct StagingMathLayoutBudget {
    package_fingerprint: [u8; 32],
    authorization_fingerprint: [u8; 32],
    profile_receipt_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    maximum: u64,
    remaining: u64,
}

#[derive(Clone)]
pub struct StagingMathProfileSessionIdentity(
    std::sync::Arc<std::sync::Mutex<Option<StagingMathLayoutBudget>>>,
);

impl StagingMathProfileSessionIdentity {
    pub fn fresh() -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(None)))
    }
}

impl std::fmt::Debug for StagingMathProfileSessionIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StagingMathProfileSessionIdentity(..)")
    }
}

impl PartialEq for StagingMathProfileSessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for StagingMathProfileSessionIdentity {}

/// Session-bound dependency-inversion projection issued by math profile
/// preflight. Its deterministic view and receipt fingerprint are serializable;
/// the opaque progress token and shared work permit are process-local.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathProfileAuthorization {
    view: StagingMathProfileView,
    profile_receipt_fingerprint: [u8; 32],
    session: StagingMathProfileSessionIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathProfileProgressToken {
    session: StagingMathProfileSessionIdentity,
    authorization_fingerprint: [u8; 32],
    profile_receipt_fingerprint: [u8; 32],
}

pub struct StagingMathLayoutBudgetGuard<'a> {
    budget: std::sync::MutexGuard<'a, Option<StagingMathLayoutBudget>>,
}

impl std::fmt::Debug for StagingMathLayoutBudgetGuard<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StagingMathLayoutBudgetGuard(..)")
    }
}

impl StagingMathLayoutBudgetGuard<'_> {
    pub fn reserve(&mut self, units: u64) -> Result<(), StagingSemanticSyntaxError> {
        let budget = self
            .budget
            .as_mut()
            .ok_or(StagingSemanticSyntaxError::ReceiptMismatch)?;
        if units == 0 || units > budget.remaining {
            return Err(StagingSemanticSyntaxError::MathLayoutUnitLimit);
        }
        budget.remaining = budget
            .remaining
            .checked_sub(units)
            .ok_or(StagingSemanticSyntaxError::MathLayoutUnitLimit)?;
        Ok(())
    }
}

impl StagingMathProfileAuthorization {
    #[doc(hidden)]
    pub fn bind_profile_receipt(
        view: StagingMathProfileView,
        profile_receipt_fingerprint: [u8; 32],
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
        session: &StagingMathProfileSessionIdentity,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        let expected = StagingMathProfileView::new(package, limits)?;
        if view != expected || profile_receipt_fingerprint == [0; 32] {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        Self::bind_checked(view, profile_receipt_fingerprint, package, limits, session)
    }

    fn bind_checked(
        view: StagingMathProfileView,
        profile_receipt_fingerprint: [u8; 32],
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
        session: &StagingMathProfileSessionIdentity,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        {
            let mut budget = session
                .0
                .lock()
                .map_err(|_| StagingSemanticSyntaxError::ReceiptMismatch)?;
            match budget.as_ref() {
                Some(existing)
                    if existing.package_fingerprint == package.semantic_fingerprint()
                        && existing.authorization_fingerprint == view.profile_fingerprint()
                        && existing.profile_receipt_fingerprint == profile_receipt_fingerprint
                        && existing.limits_fingerprint == limits.fingerprint()
                        && existing.maximum == limits.extension().get().max_math_layout_units => {}
                Some(_) => return Err(StagingSemanticSyntaxError::ReceiptMismatch),
                None => {
                    *budget = Some(StagingMathLayoutBudget {
                        package_fingerprint: package.semantic_fingerprint(),
                        authorization_fingerprint: view.profile_fingerprint(),
                        profile_receipt_fingerprint,
                        limits_fingerprint: limits.fingerprint(),
                        maximum: limits.extension().get().max_math_layout_units,
                        remaining: limits.extension().get().max_math_layout_units,
                    });
                }
            }
        }
        let authorization = Self {
            view,
            profile_receipt_fingerprint,
            session: session.clone(),
        };
        authorization.authorizes(package, limits)?;
        Ok(authorization)
    }

    #[doc(hidden)]
    pub fn bind_production_profile_receipt(
        view: StagingMathProfileView,
        profile_receipt_fingerprint: [u8; 32],
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
        session: &StagingMathProfileSessionIdentity,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        let expected = StagingMathProfileView::new_for_production(package, limits)?;
        if view != expected || profile_receipt_fingerprint == [0; 32] {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        Self::bind_checked(view, profile_receipt_fingerprint, package, limits, session)
    }

    pub const fn view(&self) -> &StagingMathProfileView {
        &self.view
    }
    pub const fn base(&self) -> &StagingSafeVectorProfileView {
        self.view.base()
    }
    pub fn math_node_ids(&self) -> &[NodeId] {
        self.view.math_node_ids()
    }
    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        self.view.page_geometry()
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.view.profile_fingerprint()
    }
    pub const fn profile_receipt_fingerprint(&self) -> [u8; 32] {
        self.profile_receipt_fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        self.view.canonical_jcs()
    }
    pub fn progress_token(&self) -> StagingMathProfileProgressToken {
        StagingMathProfileProgressToken {
            session: self.session.clone(),
            authorization_fingerprint: self.profile_fingerprint(),
            profile_receipt_fingerprint: self.profile_receipt_fingerprint,
        }
    }
    pub fn matches_progress(&self, token: &StagingMathProfileProgressToken) -> bool {
        self.session == token.session
            && self.profile_fingerprint() == token.authorization_fingerprint
            && self.profile_receipt_fingerprint == token.profile_receipt_fingerprint
    }
    pub fn authorizes(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSemanticSyntaxError> {
        let expected = if self.view.is_production() {
            StagingMathProfileView::new_for_production(package, limits)?
        } else {
            StagingMathProfileView::new(package, limits)?
        };
        let budget_matches = self
            .session
            .0
            .lock()
            .map_err(|_| StagingSemanticSyntaxError::ReceiptMismatch)?
            .as_ref()
            .is_some_and(|budget| {
                budget.package_fingerprint == package.semantic_fingerprint()
                    && budget.authorization_fingerprint == self.profile_fingerprint()
                    && budget.profile_receipt_fingerprint == self.profile_receipt_fingerprint
                    && budget.limits_fingerprint == limits.fingerprint()
                    && budget.maximum == limits.extension().get().max_math_layout_units
                    && budget.remaining <= budget.maximum
            });
        if self.view != expected || !budget_matches {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        Ok(())
    }
    pub fn layout_budget(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<StagingMathLayoutBudgetGuard<'_>, StagingSemanticSyntaxError> {
        self.authorizes(package, limits)?;
        let budget = self
            .session
            .0
            .lock()
            .map_err(|_| StagingSemanticSyntaxError::ReceiptMismatch)?;
        Ok(StagingMathLayoutBudgetGuard { budget })
    }
}

fn first_precomposed_vector_owner(blocks: &[StagingM4Block]) -> Option<NodeId> {
    for block in blocks {
        let owner = match block {
            StagingM4Block::Paragraph { inline_vectors, .. }
            | StagingM4Block::Heading { inline_vectors, .. } => {
                inline_vectors.first().map(|value| value.node_id)
            }
            StagingM4Block::VectorFigure { common, .. }
            | StagingM4Block::MathVectorBlock { common, .. } => Some(common.node_id),
            StagingM4Block::List { items, .. } => items
                .iter()
                .find_map(|item| first_precomposed_vector_owner(&item.blocks)),
            StagingM4Block::Table { head, body, .. } => head
                .iter()
                .chain(body)
                .flat_map(|row| &row.cells)
                .find_map(|cell| first_precomposed_vector_owner(&cell.blocks)),
            StagingM4Block::Figure { caption, .. } => first_precomposed_vector_owner(caption),
            StagingM4Block::SemanticContainer { blocks, .. } => {
                first_precomposed_vector_owner(blocks)
            }
            StagingM4Block::PageBreak { .. } | StagingM4Block::DisplayMath { .. } => None,
        };
        if owner.is_some() {
            return owner;
        }
    }
    None
}

fn collect_vector_figure_owners(
    blocks: &[StagingM4Block],
    package: &ValidatedStagingSemanticPackage,
    vectors: &BTreeSet<ImageResourceId>,
    output: &mut Vec<NodeId>,
    reject_precomposed: bool,
) -> Result<(), StagingSemanticSyntaxError> {
    for block in blocks {
        match block {
            StagingM4Block::Figure {
                common,
                image_id,
                caption,
                ..
            } => {
                let declaration = package
                    .resources()
                    .images
                    .get(image_id.get() as usize)
                    .filter(|image| image.image_id == *image_id)
                    .ok_or(StagingSemanticSyntaxError::InvalidResource)?;
                if declaration.media == ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1) {
                    if !vectors.contains(image_id) {
                        return Err(StagingSemanticSyntaxError::InvalidResource);
                    }
                    output.push(common.node_id);
                }
                collect_vector_figure_owners(
                    caption,
                    package,
                    vectors,
                    output,
                    reject_precomposed,
                )?;
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    collect_vector_figure_owners(
                        &item.blocks,
                        package,
                        vectors,
                        output,
                        reject_precomposed,
                    )?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_vector_figure_owners(
                        &cell.blocks,
                        package,
                        vectors,
                        output,
                        reject_precomposed,
                    )?;
                }
            }
            StagingM4Block::SemanticContainer { blocks, .. } => {
                collect_vector_figure_owners(blocks, package, vectors, output, reject_precomposed)?;
            }
            StagingM4Block::VectorFigure { common, .. }
            | StagingM4Block::MathVectorBlock { common, .. }
                if reject_precomposed =>
            {
                return Err(StagingSemanticSyntaxError::PrecomposedVectorStaging(
                    common.node_id,
                ));
            }
            StagingM4Block::Paragraph { inline_vectors, .. }
            | StagingM4Block::Heading { inline_vectors, .. }
                if reject_precomposed =>
            {
                if let Some(vector) = inline_vectors.first() {
                    return Err(StagingSemanticSyntaxError::PrecomposedVectorStaging(
                        vector.node_id,
                    ));
                }
            }
            StagingM4Block::VectorFigure { caption, .. } => {
                collect_vector_figure_owners(caption, package, vectors, output, reject_precomposed)?
            }
            StagingM4Block::MathVectorBlock { .. }
            | StagingM4Block::Paragraph { .. }
            | StagingM4Block::Heading { .. }
            | StagingM4Block::PageBreak { .. }
            | StagingM4Block::DisplayMath { .. } => {}
        }
    }
    Ok(())
}

fn encode_safe_vector_profile_view(
    package: &ValidatedStagingSemanticPackage,
    base: [u8; 32],
    limits: [u8; 32],
    resources: &[ImageResourceId],
    figures: &[NodeId],
    page_geometry: [u8; 32],
) -> String {
    let mut output = String::from(
        "{\"algorithm\":\"typaxis.production-book-safe-vector-authorization/1\",\"base_profile_fingerprint\":",
    );
    push_hash(&mut output, base);
    output.push_str(",\"figure_owners\":[");
    for (index, owner) in figures.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&owner.get().to_string());
    }
    output.push_str("],\"limits_fingerprint\":");
    push_hash(&mut output, limits);
    output.push_str(",\"package_fingerprint\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push_str(",\"page_geometry_fingerprint\":");
    push_hash(&mut output, page_geometry);
    output.push_str(",\"vector_resource_ids\":[");
    for (index, id) in resources.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&id.get().to_string());
    }
    output.push_str("]}");
    output
}

fn validate_profile_container_domain(
    blocks: &[StagingM4Block],
    count: &mut u32,
    permits_precomposed: bool,
) -> Result<(), StagingSemanticSyntaxError> {
    for block in blocks {
        match block {
            StagingM4Block::SemanticContainer { common, blocks, .. } => {
                if !blocks.iter().any(StagingM4Block::is_semantically_nonempty) {
                    return Err(StagingSemanticSyntaxError::EmptyContainer(common.node_id));
                }
                *count = count
                    .checked_add(1)
                    .ok_or(StagingSemanticSyntaxError::AstNodeLimit)?;
                validate_profile_container_domain(blocks, count, permits_precomposed)?;
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    validate_profile_container_domain(&item.blocks, count, permits_precomposed)?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    validate_profile_container_domain(&cell.blocks, count, permits_precomposed)?;
                }
            }
            StagingM4Block::Figure { caption, .. } => {
                validate_profile_container_domain(caption, count, permits_precomposed)?;
            }
            StagingM4Block::VectorFigure { common, .. }
            | StagingM4Block::MathVectorBlock { common, .. }
                if !permits_precomposed =>
            {
                return Err(StagingSemanticSyntaxError::PrecomposedVectorStaging(
                    common.node_id,
                ));
            }
            StagingM4Block::VectorFigure { caption, .. } => {
                validate_profile_container_domain(caption, count, permits_precomposed)?;
            }
            StagingM4Block::MathVectorBlock { .. } => {}
            StagingM4Block::Paragraph { .. }
            | StagingM4Block::Heading { .. }
            | StagingM4Block::PageBreak { .. }
            | StagingM4Block::DisplayMath { .. } => {}
        }
    }
    Ok(())
}

fn encode_profile_view(
    package: &ValidatedStagingSemanticPackage,
    limits: &ValidatedResourceLimits,
    container_count: u32,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_PROFILE_RECEIPT_ALGORITHM);
    output.push_str(",\"canonical_package_sha256\":");
    push_hash(&mut output, package.canonical_jcs_sha256());
    output.push_str(",\"container_count\":");
    output.push_str(&container_count.to_string());
    output.push_str(",\"contract\":\"typaxis.contract/1.4\"");
    output.push_str(",\"effective_limits\":{");
    push_profile_limits(&mut output, limits);
    output.push('}');
    output.push_str(",\"profile\":");
    push_jcs_string(&mut output, STAGING_PROFILE_ID);
    output.push_str(",\"semantic_fingerprint\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push('}');
    output
}

fn push_profile_limits(output: &mut String, limits: &ValidatedResourceLimits) {
    let limits = limits.get();
    macro_rules! fields {
        ($(($name:literal, $value:expr)),+ $(,)?) => {{
            let mut first = true;
            $(
                if !first {
                    output.push(',');
                }
                first = false;
                output.push_str(concat!("\"", $name, "\":"));
                output.push_str(&$value.to_string());
            )+
            let _ = first;
        }};
    }
    fields!(
        ("max_ast_nesting_depth", limits.max_ast_nesting_depth),
        ("max_ast_nodes", limits.max_ast_nodes),
        ("max_cids_per_font", limits.max_cids_per_font),
        (
            "max_column_balance_candidates",
            limits.max_column_balance_candidates
        ),
        ("max_decoded_image_bytes", limits.max_decoded_image_bytes),
        (
            "max_document_package_bytes",
            limits.max_document_package_bytes
        ),
        ("max_float_carry_pages", limits.max_float_carry_pages),
        ("max_float_queue", limits.max_float_queue),
        ("max_font_bytes", limits.max_font_bytes),
        ("max_fonts", limits.max_fonts),
        (
            "max_footnote_reflows_per_page",
            limits.max_footnote_reflows_per_page
        ),
        ("max_fragments", limits.max_fragments),
        ("max_image_bytes", limits.max_image_bytes),
        ("max_image_pixels", limits.max_image_pixels),
        ("max_images", limits.max_images),
        ("max_include_depth", limits.max_include_depth),
        ("max_include_files", limits.max_include_files),
        ("max_input_bytes", limits.max_input_bytes),
        ("max_json_nesting_depth", limits.max_json_nesting_depth),
        ("max_layout_passes", limits.max_layout_passes),
        ("max_line_reshape_passes", limits.max_line_reshape_passes),
        ("max_output_bytes", limits.max_output_bytes),
        ("max_page_break_lookback", limits.max_page_break_lookback),
        ("max_pages", limits.max_pages),
        ("max_pdf_objects", limits.max_pdf_objects),
        ("max_resource_bytes", limits.max_resource_bytes),
        (
            "max_shaping_context_bytes",
            limits.max_shaping_context_bytes
        ),
        ("max_source_bytes", limits.max_source_bytes),
        ("max_spool_bytes", limits.max_spool_bytes),
        ("max_style_rules", limits.max_style_rules),
        ("max_text_buffer_bytes", limits.max_text_buffer_bytes),
        ("max_text_bytes", limits.max_text_bytes),
        ("max_uri_bytes", limits.max_uri_bytes),
    );
}

impl ValidatedStagingSemanticPackage {
    pub const fn document(&self) -> &StagingM4Document {
        &self.document
    }
    pub const fn resources(&self) -> &StagingM4ResourceCatalog {
        &self.resources
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub const fn raw_sha256(&self) -> [u8; 32] {
        self.raw_sha256
    }
    pub const fn canonical_jcs_sha256(&self) -> [u8; 32] {
        self.canonical_jcs_sha256
    }
    pub const fn semantic_fingerprint(&self) -> [u8; 32] {
        self.semantic_fingerprint
    }
    pub fn semantic_jcs(&self) -> &str {
        &self.semantic_jcs
    }
    pub fn computed_style(&self, owner: NodeId) -> Option<&SemanticContainerComputedStyle> {
        self.computed_styles.get(&owner)
    }
    pub fn semantic_container_count(&self) -> usize {
        self.computed_styles.len()
    }
    pub fn precomposed_vector_style(
        &self,
        owner: NodeId,
    ) -> Option<&PrecomposedVectorComputedStyleReceipt> {
        self.precomposed_vector_styles.get(&owner)
    }
    pub fn precomposed_vector_style_count(&self) -> usize {
        self.precomposed_vector_styles.len()
    }
    pub fn verify_precomposed_vector_style(
        &self,
        receipt: &PrecomposedVectorComputedStyleReceipt,
    ) -> Result<(), StagingSemanticSyntaxError> {
        self.checked_wire()?;
        if !self
            .precomposed_vector_styles
            .values()
            .any(|owned| std::ptr::eq(owned, receipt))
            || receipt.verify_for(receipt.kind()).is_err()
        {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        Ok(())
    }
    pub fn math_nodes(&self) -> &[ValidatedStagingMathNode] {
        &self.math_nodes
    }
    pub fn math_node(&self, owner: NodeId) -> Option<&ValidatedStagingMathNode> {
        self.math_nodes
            .iter()
            .find(|value| value.domain.node_id == owner)
    }
    pub fn precomposed_vector_metrics(&self) -> &[ValidatedPrecomposedVectorMetrics] {
        &self.precomposed_vector_metrics
    }
    pub fn precomposed_vector_metrics_for(
        &self,
        owner: NodeId,
    ) -> Option<&ValidatedPrecomposedVectorMetrics> {
        self.precomposed_vector_metrics
            .binary_search_by_key(&owner, ValidatedPrecomposedVectorMetrics::node_id)
            .ok()
            .map(|index| &self.precomposed_vector_metrics[index])
    }
    pub fn precomposed_vector_effective_language(
        &self,
        owner: NodeId,
    ) -> Result<ValidatedPrecomposedVectorEffectiveLanguage, StagingSemanticSyntaxError> {
        self.precomposed_vector_effective_languages()?
            .into_iter()
            .find(|receipt| receipt.owner == owner)
            .ok_or(StagingSemanticSyntaxError::ReceiptMismatch)
    }
    pub fn precomposed_vector_effective_languages(
        &self,
    ) -> Result<Vec<ValidatedPrecomposedVectorEffectiveLanguage>, StagingSemanticSyntaxError> {
        let wire = self.checked_wire()?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(self.precomposed_vector_metrics.len())
            .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
        collect_precomposed_vector_languages(wire, &mut values)?;
        values.sort_unstable_by_key(|(owner, _)| *owner);
        if values.len() != self.precomposed_vector_metrics.len()
            || values.windows(2).any(|pair| pair[0].0 >= pair[1].0)
        {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        let mut receipts = Vec::new();
        receipts
            .try_reserve_exact(values.len())
            .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
        for ((owner, language), metrics) in values.into_iter().zip(&self.precomposed_vector_metrics)
        {
            if owner != metrics.node_id() {
                return Err(StagingSemanticSyntaxError::ReceiptMismatch);
            }
            let mut receipt = ValidatedPrecomposedVectorEffectiveLanguage {
                package_sha256: self.canonical_jcs_sha256,
                semantic_fingerprint: self.semantic_fingerprint,
                owner,
                kind: metrics.kind(),
                language,
                canonical_jcs: String::new(),
                fingerprint: [0; 32],
            };
            receipt.canonical_jcs = encode_precomposed_vector_effective_language(&receipt);
            receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
            receipts.push(receipt);
        }
        Ok(receipts)
    }
    pub fn verify_precomposed_vector_effective_language(
        &self,
        receipt: &ValidatedPrecomposedVectorEffectiveLanguage,
    ) -> Result<(), StagingSemanticSyntaxError> {
        self.checked_wire()?;
        let metrics = self
            .precomposed_vector_metrics_for(receipt.owner)
            .ok_or(StagingSemanticSyntaxError::ReceiptMismatch)?;
        let canonical_language = crate::canonicalize_bcp47_language(&receipt.language)
            .map_err(|_| StagingSemanticSyntaxError::ReceiptMismatch)?;
        let canonical = encode_precomposed_vector_effective_language(receipt);
        if receipt.package_sha256 != self.canonical_jcs_sha256
            || receipt.semantic_fingerprint != self.semantic_fingerprint
            || receipt.kind != metrics.kind()
            || receipt.language != canonical_language
            || receipt.canonical_jcs != canonical
            || receipt.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        Ok(())
    }
    pub fn verify_precomposed_vector_metrics(
        &self,
        receipt: &ValidatedPrecomposedVectorMetrics,
    ) -> Result<(), StagingSemanticSyntaxError> {
        self.checked_wire()?;
        let Some(owned) = self.precomposed_vector_metrics_for(receipt.node_id()) else {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        };
        if !std::ptr::eq(owned, receipt)
            || !receipt.verify_integrity(
                self.canonical_jcs_sha256,
                precomposed_vector_limits_fingerprint(&self.limits),
                &self.precomposed_vector_session,
            )
        {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        Ok(())
    }
    pub fn checked_wire(
        &self,
    ) -> Result<&WireStagingM4DocumentPackage, StagingSemanticSyntaxError> {
        for receipt in self.precomposed_vector_styles.values() {
            receipt
                .verify_for(receipt.kind())
                .map_err(|_| StagingSemanticSyntaxError::ReceiptMismatch)?;
        }
        let mut previous = None;
        let limits_fingerprint = precomposed_vector_limits_fingerprint(&self.limits);
        for receipt in &self.precomposed_vector_metrics {
            if previous.is_some_and(|value| value >= receipt.node_id())
                || !receipt.verify_integrity(
                    self.canonical_jcs_sha256,
                    limits_fingerprint,
                    &self.precomposed_vector_session,
                )
            {
                return Err(StagingSemanticSyntaxError::ReceiptMismatch);
            }
            previous = Some(receipt.node_id());
        }
        let observed = encode_semantic_receipt(
            &self.document,
            &self.resources,
            &self.computed_styles,
            &self.math_nodes,
            &self.precomposed_vector_metrics,
            self.canonical_jcs_sha256,
        );
        if observed != self.semantic_jcs || sha256(observed.as_bytes()) != self.semantic_fingerprint
        {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        Ok(&self.wire)
    }
}

fn collect_precomposed_vector_languages(
    wire: &WireStagingM4DocumentPackage,
    output: &mut Vec<(NodeId, String)>,
) -> Result<(), StagingSemanticSyntaxError> {
    let document_language = crate::canonicalize_bcp47_language(&wire.document().language)
        .map_err(|_| StagingSemanticSyntaxError::ReceiptMismatch)?;
    collect_precomposed_vector_language_from_blocks(
        &wire.document().blocks,
        &document_language,
        output,
    )?;
    for footnote in &wire.document().footnotes {
        let inherited =
            canonical_inherited_language(footnote.language.as_deref(), &document_language)?;
        collect_precomposed_vector_language_from_blocks(&footnote.blocks, &inherited, output)?;
    }
    Ok(())
}

fn collect_precomposed_vector_language_from_blocks(
    blocks: &[WireStagingM4Block],
    inherited: &str,
    output: &mut Vec<(NodeId, String)>,
) -> Result<(), StagingSemanticSyntaxError> {
    for block in blocks {
        let effective = canonical_inherited_language(block.language(), inherited)?;
        match block {
            WireStagingM4Block::MathVectorBlock { node_id, .. } => {
                push_precomposed_vector_language(output, NodeId::new(*node_id), effective)?;
            }
            WireStagingM4Block::Paragraph { children, .. }
            | WireStagingM4Block::Heading { children, .. } => {
                collect_precomposed_vector_language_from_inlines(children, &effective, output)?;
            }
            WireStagingM4Block::List { items, .. } => {
                for item in items {
                    let item_language =
                        canonical_inherited_language(item.language.as_deref(), &effective)?;
                    collect_precomposed_vector_language_from_blocks(
                        &item.blocks,
                        &item_language,
                        output,
                    )?;
                }
            }
            WireStagingM4Block::Table { head, body, .. } => {
                for row in head.iter().chain(body) {
                    let row_language =
                        canonical_inherited_language(row.language.as_deref(), &effective)?;
                    for cell in &row.cells {
                        let cell_language =
                            canonical_inherited_language(cell.language.as_deref(), &row_language)?;
                        collect_precomposed_vector_language_from_blocks(
                            &cell.blocks,
                            &cell_language,
                            output,
                        )?;
                    }
                }
            }
            WireStagingM4Block::Figure { caption, .. } => {
                collect_precomposed_vector_language_from_blocks(caption, &effective, output)?;
            }
            WireStagingM4Block::VectorFigure {
                node_id, caption, ..
            } => {
                push_precomposed_vector_language(output, NodeId::new(*node_id), effective.clone())?;
                collect_precomposed_vector_language_from_blocks(caption, &effective, output)?;
            }
            WireStagingM4Block::SemanticContainer { blocks, .. } => {
                collect_precomposed_vector_language_from_blocks(blocks, &effective, output)?;
            }
            WireStagingM4Block::PageBreak { .. } | WireStagingM4Block::DisplayMath { .. } => {}
        }
    }
    Ok(())
}

fn collect_precomposed_vector_language_from_inlines(
    inlines: &[WireStagingM4Inline],
    inherited: &str,
    output: &mut Vec<(NodeId, String)>,
) -> Result<(), StagingSemanticSyntaxError> {
    for inline in inlines {
        let effective = canonical_inherited_language(inline.language(), inherited)?;
        match inline {
            WireStagingM4Inline::InlineVector { node_id, .. }
            | WireStagingM4Inline::MathVector { node_id, .. } => {
                push_precomposed_vector_language(output, NodeId::new(*node_id), effective)?;
            }
            WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. }
            | WireStagingM4Inline::Link { children, .. } => {
                collect_precomposed_vector_language_from_inlines(children, &effective, output)?;
            }
            WireStagingM4Inline::Text { .. }
            | WireStagingM4Inline::InlineMath { .. }
            | WireStagingM4Inline::Anchor { .. }
            | WireStagingM4Inline::Reference { .. }
            | WireStagingM4Inline::FootnoteReference { .. }
            | WireStagingM4Inline::SoftBreak { .. }
            | WireStagingM4Inline::HardBreak { .. } => {}
        }
    }
    Ok(())
}

fn canonical_inherited_language(
    authored: Option<&str>,
    inherited: &str,
) -> Result<String, StagingSemanticSyntaxError> {
    match authored {
        Some(authored) => crate::canonicalize_bcp47_language(authored)
            .map_err(|_| StagingSemanticSyntaxError::ReceiptMismatch),
        None => Ok(inherited.to_owned()),
    }
}

fn push_precomposed_vector_language(
    output: &mut Vec<(NodeId, String)>,
    owner: NodeId,
    language: String,
) -> Result<(), StagingSemanticSyntaxError> {
    output
        .try_reserve(1)
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    output.push((owner, language));
    Ok(())
}

fn encode_precomposed_vector_effective_language(
    value: &ValidatedPrecomposedVectorEffectiveLanguage,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, PRECOMPOSED_VECTOR_EFFECTIVE_LANGUAGE_ALGORITHM);
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, value.kind.as_str());
    output.push_str(",\"language\":");
    push_jcs_string(&mut output, &value.language);
    output.push_str(",\"owner\":");
    output.push_str(&value.owner.get().to_string());
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, value.package_sha256);
    output.push_str(",\"semantic_fingerprint\":");
    push_hash(&mut output, value.semantic_fingerprint);
    output.push('}');
    output
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StagingSemanticPackageParser;

impl StagingSemanticPackageParser {
    pub const fn new() -> Self {
        Self
    }

    pub fn parse(
        &self,
        decoded: DecodedStagingSemanticDocumentPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<ValidatedStagingSemanticPackage, StagingSemanticSyntaxError> {
        if decoded.limits() != limits {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        let raw_sha256 = decoded.raw_sha256();
        let canonical_jcs_sha256 = decoded.canonical_jcs_sha256();
        let wire = decoded.into_wire();
        let precomposed_vector_session = PrecomposedVectorSyntaxSessionIdentity::fresh();
        let precomposed_vector_limits_fingerprint = precomposed_vector_limits_fingerprint(limits);
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
            precomposed_vector_session: &precomposed_vector_session,
            precomposed_vector_metrics: Vec::new(),
            canonical_package_sha256: canonical_jcs_sha256,
            precomposed_vector_limits_fingerprint,
            limits,
        };
        validator.node(wire.document().node_id, None, 1)?;
        let document = lower_document(wire.document(), &mut validator)?;
        let pending_math = std::mem::take(&mut validator.math_nodes);
        let precomposed_vector_metrics = std::mem::take(&mut validator.precomposed_vector_metrics);
        let resources = lower_resources(wire.resources())?;
        let rules = lower_semantic_style_rules(wire.style_sheet(), limits)?;
        let mut computed_styles = BTreeMap::new();
        let mut precomposed_vector_styles = BTreeMap::new();
        let mut math_styles = BTreeMap::new();
        collect_computed_styles(
            &document.blocks,
            &rules,
            None,
            &pending_math,
            &mut computed_styles,
            &mut precomposed_vector_styles,
            &mut math_styles,
        )?;
        for footnote in &document.footnotes {
            collect_computed_styles(
                &footnote.blocks,
                &rules,
                None,
                &pending_math,
                &mut computed_styles,
                &mut precomposed_vector_styles,
                &mut math_styles,
            )?;
        }
        if computed_styles.is_empty()
            && pending_math.is_empty()
            && first_precomposed_vector_owner(&document.blocks)
                .or_else(|| {
                    document
                        .footnotes
                        .iter()
                        .find_map(|footnote| first_precomposed_vector_owner(&footnote.blocks))
                })
                .is_none()
        {
            return Err(StagingSemanticSyntaxError::InvalidNesting);
        }
        let mut math_nodes = Vec::new();
        math_nodes
            .try_reserve_exact(pending_math.len())
            .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
        for pending in pending_math {
            let computed_style = math_styles
                .remove(&pending.domain.node_id)
                .ok_or(StagingSemanticSyntaxError::InvalidStyle)?;
            math_nodes.push(ValidatedStagingMathNode {
                domain: pending.domain,
                parsed: pending.parsed,
                computed_style,
            });
        }
        if !math_styles.is_empty() {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        let semantic_jcs = encode_semantic_receipt(
            &document,
            &resources,
            &computed_styles,
            &math_nodes,
            &precomposed_vector_metrics,
            canonical_jcs_sha256,
        );
        Ok(ValidatedStagingSemanticPackage {
            wire,
            limits: limits.clone(),
            precomposed_vector_session,
            precomposed_vector_metrics,
            document,
            resources,
            computed_styles,
            precomposed_vector_styles,
            math_nodes,
            raw_sha256,
            canonical_jcs_sha256,
            semantic_fingerprint: sha256(semantic_jcs.as_bytes()),
            semantic_jcs,
        })
    }

    /// Consume the public host-admission receipt and bind the decoded M4
    /// package to its stable source facts before returning syntax authority.
    pub fn parse_admitted(
        &self,
        input: AdmittedSemanticMachinePackage,
        limits: &ValidatedResourceLimits,
    ) -> ProductionMachineParseOutcome {
        let (decoded, sources, admission) = input.into_parts();
        let consistent = admission.progress().stage() == MachineInputStage::SourcesAdmitted
            && admission.progress().session_identity() == Some(admission.session_identity())
            && admission.progress().fingerprint() == Some(admission.fingerprint())
            && admission.progress().decoded().is_some_and(|facts| {
                facts.contract() == typaxis_core::DocumentPackageContractId::V1_4
                    && facts.canonical_sha256() == decoded.canonical_jcs_sha256()
            })
            && admission
                .progress()
                .package()
                .is_some_and(|facts| facts.sha256() == decoded.raw_sha256())
            && sources.len() == decoded.wire().sources().len()
            && admission.progress().sources().len() == sources.len()
            && sources
                .iter()
                .zip(decoded.wire().sources())
                .zip(admission.progress().sources())
                .all(|((source, declaration), progress)| {
                    source.facts() == progress
                        && source.facts().source_id().get() == declaration.source_id
                        && source.facts().uri().as_str() == declaration.uri
                        && source.facts().bytes() == u64::from(declaration.utf8_byte_length)
                        && source.text().len() as u64 == source.facts().bytes()
                });
        if !consistent {
            return ProductionMachineParseOutcome::Failed {
                progress: Box::new(admission.into_failure_progress()),
                failure: StagingSemanticSyntaxError::ReceiptMismatch,
            };
        }
        match self.parse(decoded, limits) {
            Ok(package)
                if package.raw_sha256()
                    == admission
                        .progress()
                        .package()
                        .expect("consistent admission retains package facts")
                        .sha256()
                    && package.canonical_jcs_sha256()
                        == admission
                            .progress()
                            .decoded()
                            .expect("consistent admission retains decoded facts")
                            .canonical_sha256() =>
            {
                ProductionMachineParseOutcome::Parsed {
                    package: Box::new(ValidatedProductionMachinePackage {
                        package,
                        provenance: admission,
                    }),
                }
            }
            Ok(_) => ProductionMachineParseOutcome::Failed {
                progress: Box::new(admission.into_failure_progress()),
                failure: StagingSemanticSyntaxError::ReceiptMismatch,
            },
            Err(failure) => ProductionMachineParseOutcome::Failed {
                progress: Box::new(admission.into_failure_progress()),
                failure,
            },
        }
    }
}

struct SemanticValidator<'a> {
    sources: &'a BTreeMap<u32, u32>,
    text_buffers: &'a BTreeMap<u32, WireStagingM4TextBuffer>,
    precomposed_vector_text_buffer_sha256: BTreeMap<u32, [u8; 32]>,
    precomposed_vector_text_slice_sha256: BTreeMap<(u32, u32, u32), [u8; 32]>,
    next_node_id: u32,
    node_count: u64,
    admitted_text_and_math_speech_bytes: u64,
    math_nodes: Vec<PendingStagingMathNode>,
    precomposed_vector_session: &'a PrecomposedVectorSyntaxSessionIdentity,
    precomposed_vector_metrics: Vec<ValidatedPrecomposedVectorMetrics>,
    canonical_package_sha256: [u8; 32],
    precomposed_vector_limits_fingerprint: [u8; 32],
    limits: &'a ValidatedResourceLimits,
}

fn parse_text_buffers(
    buffers: &[WireStagingM4TextBuffer],
) -> Result<BTreeMap<u32, WireStagingM4TextBuffer>, StagingSemanticSyntaxError> {
    let mut result = BTreeMap::new();
    for (index, buffer) in buffers.iter().enumerate() {
        if usize::try_from(buffer.text_id) != Ok(index)
            || result.insert(buffer.text_id, buffer.clone()).is_some()
        {
            return Err(StagingSemanticSyntaxError::InvalidInline);
        }
    }
    Ok(result)
}

impl SemanticValidator<'_> {
    fn node(
        &mut self,
        node_id: u32,
        span: Option<WireStagingSourceSpan>,
        depth: u32,
    ) -> Result<(), StagingSemanticSyntaxError> {
        self.node_with_limit_kind(node_id, span, depth, false)
    }

    fn precomposed_vector_node(
        &mut self,
        node_id: u32,
        span: Option<WireStagingSourceSpan>,
        depth: u32,
    ) -> Result<(), StagingSemanticSyntaxError> {
        self.node_with_limit_kind(node_id, span, depth, true)
    }

    fn node_with_limit_kind(
        &mut self,
        node_id: u32,
        span: Option<WireStagingSourceSpan>,
        depth: u32,
        precomposed_vector: bool,
    ) -> Result<(), StagingSemanticSyntaxError> {
        let node_limit = || {
            if precomposed_vector {
                StagingSemanticSyntaxError::PrecomposedVectorAstNodeLimit
            } else {
                StagingSemanticSyntaxError::AstNodeLimit
            }
        };
        let depth_limit = || {
            if precomposed_vector {
                StagingSemanticSyntaxError::PrecomposedVectorAstDepthLimit
            } else {
                StagingSemanticSyntaxError::AstDepthLimit
            }
        };
        if node_id != self.next_node_id {
            return Err(StagingSemanticSyntaxError::InvalidNodeOrder);
        }
        self.next_node_id = self.next_node_id.checked_add(1).ok_or_else(node_limit)?;
        self.node_count = self.node_count.checked_add(1).ok_or_else(node_limit)?;
        if self.node_count > self.limits.get().max_ast_nodes {
            return Err(node_limit());
        }
        if depth > self.limits.get().max_ast_nesting_depth {
            return Err(depth_limit());
        }
        if let Some(span) = span {
            self.validate_span(span)?;
        }
        Ok(())
    }

    fn validate_span(&self, span: WireStagingSourceSpan) -> Result<(), StagingSemanticSyntaxError> {
        let length = self
            .sources
            .get(&span.source_id)
            .ok_or(StagingSemanticSyntaxError::InvalidSourceSpan)?;
        if span.start_byte > span.end_byte || span.end_byte > *length {
            return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)] // exact wire ownership inputs
    fn math(
        &mut self,
        node_id: u32,
        owner_node_id: NodeId,
        kind: StagingM4MathKind,
        span: WireStagingSourceSpan,
        classes: &[String],
        math_source: &WireStagingMathSource,
        speech: &str,
        owner_depth: u32,
    ) -> Result<(), StagingSemanticSyntaxError> {
        if math_source.language != typaxis_math::MATH_SOURCE_LANGUAGE
            || math_source.version != typaxis_math::MATH_SOURCE_VERSION
        {
            return Err(StagingSemanticSyntaxError::InvalidMathSourceVersion);
        }
        if speech.is_empty()
            || !speech
                .chars()
                .any(|value| !is_unicode_16_white_space(value))
            || speech.chars().any(|value| {
                ('\u{0000}'..='\u{001f}').contains(&value)
                    || ('\u{007f}'..='\u{009f}').contains(&value)
            })
        {
            return Err(StagingSemanticSyntaxError::InvalidMath);
        }
        let speech_bytes =
            u64::try_from(speech.len()).map_err(|_| StagingSemanticSyntaxError::MathSpeechLimit)?;
        if speech_bytes > u64::from(self.limits.get().max_text_buffer_bytes) {
            return Err(StagingSemanticSyntaxError::MathSpeechLimit);
        }
        self.admitted_text_and_math_speech_bytes = self
            .admitted_text_and_math_speech_bytes
            .checked_add(speech_bytes)
            .ok_or(StagingSemanticSyntaxError::MathSpeechLimit)?;
        if self.admitted_text_and_math_speech_bytes > self.limits.get().max_text_bytes {
            return Err(StagingSemanticSyntaxError::MathSpeechLimit);
        }

        let text_span = math_source.text_span;
        let buffer = self
            .text_buffers
            .get(&text_span.text_id)
            .ok_or(StagingSemanticSyntaxError::InvalidMath)?;
        let start = usize::try_from(text_span.start_byte)
            .map_err(|_| StagingSemanticSyntaxError::InvalidMath)?;
        let end = usize::try_from(text_span.end_byte)
            .map_err(|_| StagingSemanticSyntaxError::InvalidMath)?;
        if start >= end
            || end > buffer.utf8.len()
            || !buffer.utf8.is_char_boundary(start)
            || !buffer.utf8.is_char_boundary(end)
        {
            return Err(StagingSemanticSyntaxError::InvalidMath);
        }
        if u64::from(text_span.end_byte - text_span.start_byte)
            > u64::from(self.limits.get().max_text_buffer_bytes)
        {
            return Err(StagingSemanticSyntaxError::MathSourceTextLimit);
        }
        let mut overlapping_mappings = buffer.mappings.iter().filter(|mapping| {
            mapping.text_range.start_byte < text_span.end_byte
                && text_span.start_byte < mapping.text_range.end_byte
        });
        let mapping = overlapping_mappings
            .next()
            .ok_or(StagingSemanticSyntaxError::InvalidSourceSpan)?;
        if overlapping_mappings.next().is_some()
            || mapping.text_range.start_byte != text_span.start_byte
            || mapping.text_range.end_byte != text_span.end_byte
            || mapping.kind != WireStagingTextMapKind::Identity
            || mapping.source_span != Some(span)
            || span.end_byte - span.start_byte != text_span.end_byte - text_span.start_byte
        {
            return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
        }
        if self.math_nodes.iter().any(|value| {
            let prior_text = value.domain.text_span;
            let prior_source = value.domain.span;
            (prior_text.text_id().get() == text_span.text_id
                && prior_text.start_byte().get() < text_span.end_byte
                && text_span.start_byte < prior_text.end_byte().get())
                || (prior_source.source_id().get() == span.source_id
                    && prior_source.start_byte().get() < span.end_byte
                    && span.start_byte < prior_source.end_byte().get())
        }) {
            return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
        }
        let source = &buffer.utf8[start..end];
        let remaining_nodes = self
            .limits
            .get()
            .max_ast_nodes
            .checked_sub(self.node_count)
            .ok_or(StagingSemanticSyntaxError::MathAstNodeLimit)?;
        let remaining_depth = self
            .limits
            .get()
            .max_ast_nesting_depth
            .checked_sub(owner_depth)
            .ok_or(StagingSemanticSyntaxError::MathAstDepthLimit)?;
        if remaining_nodes == 0 {
            return Err(StagingSemanticSyntaxError::MathAstNodeLimit);
        }
        if remaining_depth == 0 {
            return Err(StagingSemanticSyntaxError::MathAstDepthLimit);
        }
        let parse_limits = MathParseLimits::new(remaining_nodes, remaining_depth)
            .ok_or(StagingSemanticSyntaxError::ReceiptMismatch)?;
        let parsed =
            parse_math_source(source, parse_limits).map_err(|error| match error.kind() {
                typaxis_math::MathSourceErrorKind::NodeLimit => {
                    StagingSemanticSyntaxError::MathAstNodeLimit
                }
                typaxis_math::MathSourceErrorKind::DepthLimit => {
                    StagingSemanticSyntaxError::MathAstDepthLimit
                }
                typaxis_math::MathSourceErrorKind::AllocationFailure => {
                    StagingSemanticSyntaxError::AllocationFailure
                }
                typaxis_math::MathSourceErrorKind::ReceiptMismatch => {
                    StagingSemanticSyntaxError::ReceiptMismatch
                }
                _ => span.start_byte.checked_add(error.byte_offset()).map_or(
                    StagingSemanticSyntaxError::ReceiptMismatch,
                    |byte_offset| StagingSemanticSyntaxError::InvalidMathSource {
                        source_id: SourceId::new(span.source_id),
                        byte_offset: Utf8ByteOffset::new(byte_offset),
                    },
                ),
            })?;
        self.node_count = self
            .node_count
            .checked_add(parsed.ast_node_count())
            .ok_or(StagingSemanticSyntaxError::MathAstNodeLimit)?;
        let span = lower_span(span)?;
        let text_span = TextSpan::new(
            TextBufferId::new(text_span.text_id),
            Utf8ByteOffset::new(text_span.start_byte),
            Utf8ByteOffset::new(text_span.end_byte),
        )
        .ok_or(StagingSemanticSyntaxError::InvalidMath)?;
        self.math_nodes.push(PendingStagingMathNode {
            domain: StagingM4MathNode {
                node_id: NodeId::new(node_id),
                owner_node_id,
                kind,
                span,
                text_span,
                language: math_source.language.clone(),
                version: math_source.version.clone(),
                source: source.to_owned(),
                speech: speech.to_owned(),
                classes: classes.to_vec(),
            },
            parsed,
        });
        Ok(())
    }
}

fn is_unicode_16_white_space(value: char) -> bool {
    matches!(
        value,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
    )
}

fn parse_source_lengths(
    sources: &[WireStagingM4Source],
) -> Result<BTreeMap<u32, u32>, StagingSemanticSyntaxError> {
    let mut result = BTreeMap::new();
    for (index, source) in sources.iter().enumerate() {
        if usize::try_from(source.source_id) != Ok(index) {
            return Err(StagingSemanticSyntaxError::InvalidSource);
        }
        if result
            .insert(source.source_id, source.utf8_byte_length)
            .is_some()
        {
            return Err(StagingSemanticSyntaxError::InvalidSource);
        }
    }
    Ok(result)
}

fn lower_document(
    wire: &WireStagingM4Document,
    validator: &mut SemanticValidator<'_>,
) -> Result<StagingM4Document, StagingSemanticSyntaxError> {
    let blocks = lower_blocks(&wire.blocks, validator, None, NodeId::new(wire.node_id), 2)?;
    let mut footnotes = Vec::new();
    footnotes
        .try_reserve_exact(wire.footnotes.len())
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    for footnote in &wire.footnotes {
        validator.node(footnote.node_id, Some(footnote.span), 2)?;
        let span = lower_span(footnote.span)?;
        footnotes.push(StagingM4FootnoteDefinition {
            node_id: NodeId::new(footnote.node_id),
            span,
            blocks: lower_blocks(
                &footnote.blocks,
                validator,
                Some(footnote.span),
                NodeId::new(footnote.node_id),
                3,
            )?,
        });
    }
    Ok(StagingM4Document {
        node_id: NodeId::new(wire.node_id),
        blocks,
        footnotes,
    })
}

fn lower_blocks(
    values: &[WireStagingM4Block],
    validator: &mut SemanticValidator<'_>,
    semantic_owner: Option<WireStagingSourceSpan>,
    math_owner: NodeId,
    depth: u32,
) -> Result<Vec<StagingM4Block>, StagingSemanticSyntaxError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(values.len())
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    let mut previous_direct_start = None;
    for block in values {
        let span = wire_block_span(block);
        if matches!(
            block,
            WireStagingM4Block::VectorFigure { .. } | WireStagingM4Block::MathVectorBlock { .. }
        ) {
            validator.precomposed_vector_node(block.node_id(), Some(span), depth)?;
        } else {
            validator.node(block.node_id(), Some(span), depth)?;
        }
        validate_classes(block.classes())?;
        if let Some(owner) = semantic_owner {
            validate_owned_span(owner, span)?;
            if previous_direct_start.is_some_and(|previous| previous > span.start_byte) {
                return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
            }
            previous_direct_start = Some(span.start_byte);
        }
        let common = StagingM4BlockCommon {
            node_id: NodeId::new(block.node_id()),
            span: lower_span(span)?,
            classes: block.classes().to_vec(),
        };
        let lowered = match block {
            WireStagingM4Block::Paragraph { children, .. } => {
                let mut inline_vectors = Vec::new();
                let has_authored_content = validate_inlines(
                    children,
                    validator,
                    Some(span),
                    common.node_id,
                    depth + 1,
                    &mut inline_vectors,
                )?;
                StagingM4Block::Paragraph {
                    common,
                    has_authored_content,
                    inline_vectors,
                }
            }
            WireStagingM4Block::Heading {
                level, children, ..
            } => {
                if !(1..=6).contains(level) {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                let mut inline_vectors = Vec::new();
                let has_authored_content = validate_inlines(
                    children,
                    validator,
                    Some(span),
                    common.node_id,
                    depth + 1,
                    &mut inline_vectors,
                )?;
                StagingM4Block::Heading {
                    common,
                    has_authored_content,
                    inline_vectors,
                }
            }
            WireStagingM4Block::List {
                items,
                ordered,
                start,
                ..
            } => {
                if items.is_empty()
                    || (*ordered && start.map_or(true, |value| value == 0))
                    || (!*ordered && start.is_some())
                {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                let mut lowered_items = Vec::new();
                let mut previous_item_start = None;
                for item in items {
                    validator.node(item.node_id, Some(item.span), depth + 1)?;
                    validate_owned_span(span, item.span)?;
                    if previous_item_start.is_some_and(|previous| previous > item.span.start_byte) {
                        return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
                    }
                    previous_item_start = Some(item.span.start_byte);
                    lowered_items.push(StagingM4ListItem {
                        node_id: NodeId::new(item.node_id),
                        span: lower_span(item.span)?,
                        blocks: lower_blocks(
                            &item.blocks,
                            validator,
                            Some(item.span),
                            NodeId::new(item.node_id),
                            depth + 2,
                        )?,
                    });
                }
                StagingM4Block::List {
                    common,
                    items: lowered_items,
                }
            }
            WireStagingM4Block::Table {
                columns,
                head,
                body,
                ..
            } => {
                if columns.is_empty() || (head.is_empty() && body.is_empty()) {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                StagingM4Block::Table {
                    common,
                    head: lower_rows(head, validator, span, depth + 1)?,
                    body: lower_rows(body, validator, span, depth + 1)?,
                }
            }
            WireStagingM4Block::Figure {
                image_id,
                placement,
                alt,
                caption,
                ..
            } => {
                if !matches!(placement.as_str(), "block" | "float") {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                let caption_owner = common.node_id;
                StagingM4Block::Figure {
                    common,
                    image_id: ImageResourceId::new(*image_id),
                    placement: match placement.as_str() {
                        "block" => StagingM4FigurePlacement::Block,
                        "float" => StagingM4FigurePlacement::Float,
                        _ => unreachable!("placement was checked above"),
                    },
                    alternative: alt.clone(),
                    has_nonempty_alternative: !alt.is_empty(),
                    caption: lower_blocks(
                        caption,
                        validator,
                        Some(span),
                        caption_owner,
                        depth + 1,
                    )?,
                }
            }
            WireStagingM4Block::PageBreak { .. } => StagingM4Block::PageBreak { common },
            WireStagingM4Block::DisplayMath {
                math_source,
                speech,
                ..
            } => {
                validator.math(
                    block.node_id(),
                    math_owner,
                    StagingM4MathKind::Display,
                    span,
                    &common.classes,
                    math_source,
                    speech,
                    depth,
                )?;
                StagingM4Block::DisplayMath { common }
            }
            WireStagingM4Block::VectorFigure {
                image_id,
                viewport,
                alt,
                caption,
                language,
                ..
            } => {
                let owner = common.node_id;
                let viewport = lower_vector_viewport(*viewport, owner)?;
                let validated_alternative = validate_precomposed_vector_alternative(
                    validator,
                    owner,
                    PrecomposedVectorKind::VectorFigure,
                    alt,
                    None,
                )?;
                let validated_language =
                    validate_precomposed_vector_language(validator, owner, language.as_deref())?;
                issue_precomposed_vector_metrics(
                    validator,
                    owner,
                    span,
                    PrecomposedVectorKind::VectorFigure,
                    ImageResourceId::new(*image_id),
                    PrecomposedVectorMetricPayload::Figure { viewport },
                    None,
                    validated_alternative,
                    validated_language,
                    None,
                )?;
                StagingM4Block::VectorFigure {
                    common,
                    image_id: ImageResourceId::new(*image_id),
                    viewport,
                    alternative: alt.clone(),
                    caption: lower_blocks(caption, validator, Some(span), owner, depth + 1)?,
                    language: language.clone(),
                }
            }
            WireStagingM4Block::MathVectorBlock {
                actual_text,
                alt,
                equation_number,
                image_id,
                metrics,
                source_tex,
                language,
                ..
            } => {
                let owner = common.node_id;
                let metrics = lower_vector_metrics(*metrics, owner)?;
                let (source_tex, validated_source_tex) =
                    lower_vector_source_tex(*source_tex, validator, span, owner)?;
                let validated_alternative = validate_precomposed_vector_alternative(
                    validator,
                    owner,
                    PrecomposedVectorKind::MathVectorBlock,
                    alt,
                    actual_text.as_deref(),
                )?;
                let validated_language =
                    validate_precomposed_vector_language(validator, owner, language.as_deref())?;
                let (equation_number, validated_equation_number) = match equation_number.as_ref() {
                    Some(number) => {
                        let (domain, binding) = lower_vector_equation_number(
                            number,
                            validator,
                            span,
                            validated_source_tex.mapped_source_span(),
                            owner,
                            depth + 1,
                        )?;
                        (Some(domain), Some(binding))
                    }
                    None => (None, None),
                };
                issue_precomposed_vector_metrics(
                    validator,
                    owner,
                    span,
                    PrecomposedVectorKind::MathVectorBlock,
                    ImageResourceId::new(*image_id),
                    PrecomposedVectorMetricPayload::MathBlock { metrics },
                    Some(validated_source_tex),
                    validated_alternative,
                    validated_language,
                    validated_equation_number,
                )?;
                StagingM4Block::MathVectorBlock {
                    common,
                    image_id: ImageResourceId::new(*image_id),
                    metrics,
                    source_tex,
                    alternative: alt.clone(),
                    actual_text: actual_text.clone(),
                    equation_number,
                    language: language.clone(),
                }
            }
            WireStagingM4Block::SemanticContainer {
                semantic_kind,
                blocks,
                ..
            } => {
                if blocks.is_empty() {
                    return Err(StagingSemanticSyntaxError::EmptyContainer(common.node_id));
                }
                let semantic_kind = match semantic_kind {
                    typaxis_document_package::WireStagingSemanticContainerKind::Result => {
                        SemanticContainerKind::Result
                    }
                    typaxis_document_package::WireStagingSemanticContainerKind::Proof => {
                        SemanticContainerKind::Proof
                    }
                    typaxis_document_package::WireStagingSemanticContainerKind::Exercise => {
                        SemanticContainerKind::Exercise
                    }
                };
                let blocks =
                    lower_blocks(blocks, validator, Some(span), common.node_id, depth + 1)?;
                StagingM4Block::SemanticContainer {
                    common,
                    semantic_kind,
                    blocks,
                }
            }
        };
        output.push(lowered);
    }
    Ok(output)
}

fn lower_rows(
    rows: &[typaxis_document_package::WireStagingM4TableRow],
    validator: &mut SemanticValidator<'_>,
    table_owner: WireStagingSourceSpan,
    depth: u32,
) -> Result<Vec<StagingM4TableRow>, StagingSemanticSyntaxError> {
    let mut output = Vec::new();
    let mut previous_row_start = None;
    for row in rows {
        validator.node(row.node_id, Some(row.span), depth)?;
        validate_owned_span(table_owner, row.span)?;
        if previous_row_start.is_some_and(|previous| previous > row.span.start_byte) {
            return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
        }
        previous_row_start = Some(row.span.start_byte);
        let mut cells = Vec::new();
        let mut previous_cell_start = None;
        for cell in &row.cells {
            validator.node(cell.node_id, Some(cell.span), depth + 1)?;
            validate_owned_span(row.span, cell.span)?;
            if previous_cell_start.is_some_and(|previous| previous > cell.span.start_byte) {
                return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
            }
            previous_cell_start = Some(cell.span.start_byte);
            cells.push(StagingM4TableCell {
                node_id: NodeId::new(cell.node_id),
                span: lower_span(cell.span)?,
                colspan: NonZeroU16::new(cell.colspan).ok_or(
                    StagingSemanticSyntaxError::InvalidBlock(NodeId::new(cell.node_id)),
                )?,
                rowspan: NonZeroU16::new(cell.rowspan).ok_or(
                    StagingSemanticSyntaxError::InvalidBlock(NodeId::new(cell.node_id)),
                )?,
                blocks: lower_blocks(
                    &cell.blocks,
                    validator,
                    Some(cell.span),
                    NodeId::new(cell.node_id),
                    depth + 2,
                )?,
            });
        }
        output.push(StagingM4TableRow {
            node_id: NodeId::new(row.node_id),
            span: lower_span(row.span)?,
            cells,
        });
    }
    Ok(output)
}

fn validate_inlines(
    values: &[WireStagingM4Inline],
    validator: &mut SemanticValidator<'_>,
    owner: Option<WireStagingSourceSpan>,
    math_owner: NodeId,
    depth: u32,
    precomposed_vectors: &mut Vec<StagingM4InlineVector>,
) -> Result<bool, StagingSemanticSyntaxError> {
    let mut has_authored_content = false;
    let mut previous_start = None;
    for value in values {
        let span = value.span();
        if matches!(
            value,
            WireStagingM4Inline::InlineVector { .. } | WireStagingM4Inline::MathVector { .. }
        ) {
            validator.precomposed_vector_node(value.node_id(), Some(span), depth)?;
        } else {
            validator.node(value.node_id(), Some(span), depth)?;
        }
        if let Some(owner) = owner {
            validate_owned_span(owner, span)?;
        }
        if previous_start.is_some_and(|previous| previous > span.start_byte) {
            return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
        }
        previous_start = Some(span.start_byte);
        let inline_has_content = match value {
            WireStagingM4Inline::Text { text_span, .. } => {
                validate_text_span(*text_span, validator)?
            }
            WireStagingM4Inline::InlineMath {
                node_id,
                math_source,
                speech,
                ..
            } => {
                validator.math(
                    *node_id,
                    math_owner,
                    StagingM4MathKind::Inline,
                    span,
                    &[],
                    math_source,
                    speech,
                    depth,
                )?;
                true
            }
            WireStagingM4Inline::InlineVector {
                actual_text,
                alt,
                image_id,
                metrics,
                node_id,
                spacing,
                language,
                ..
            } => {
                let node_id = NodeId::new(*node_id);
                let metrics = lower_vector_metrics(*metrics, node_id)?;
                let spacing = lower_vector_spacing(*spacing, node_id)?;
                let validated_alternative = validate_precomposed_vector_alternative(
                    validator,
                    node_id,
                    PrecomposedVectorKind::InlineVector,
                    alt,
                    actual_text.as_deref(),
                )?;
                let validated_language =
                    validate_precomposed_vector_language(validator, node_id, language.as_deref())?;
                issue_precomposed_vector_metrics(
                    validator,
                    node_id,
                    span,
                    PrecomposedVectorKind::InlineVector,
                    ImageResourceId::new(*image_id),
                    PrecomposedVectorMetricPayload::Inline { metrics, spacing },
                    None,
                    validated_alternative,
                    validated_language,
                    None,
                )?;
                precomposed_vectors.push(StagingM4InlineVector {
                    node_id,
                    owner_node_id: math_owner,
                    kind: StagingM4InlineVectorKind::InlineVector,
                    span: lower_span(span)?,
                    image_id: ImageResourceId::new(*image_id),
                    metrics,
                    spacing,
                    source_tex: None,
                    alternative: alt.clone(),
                    actual_text: actual_text.clone(),
                    language: language.clone(),
                });
                true
            }
            WireStagingM4Inline::MathVector {
                actual_text,
                alt,
                image_id,
                metrics,
                node_id,
                source_tex,
                spacing,
                language,
                ..
            } => {
                let node_id = NodeId::new(*node_id);
                let metrics = lower_vector_metrics(*metrics, node_id)?;
                let spacing = lower_vector_spacing(*spacing, node_id)?;
                let (source_tex, validated_source_tex) =
                    lower_vector_source_tex(*source_tex, validator, span, node_id)?;
                let validated_alternative = validate_precomposed_vector_alternative(
                    validator,
                    node_id,
                    PrecomposedVectorKind::MathVector,
                    alt,
                    actual_text.as_deref(),
                )?;
                let validated_language =
                    validate_precomposed_vector_language(validator, node_id, language.as_deref())?;
                issue_precomposed_vector_metrics(
                    validator,
                    node_id,
                    span,
                    PrecomposedVectorKind::MathVector,
                    ImageResourceId::new(*image_id),
                    PrecomposedVectorMetricPayload::Inline { metrics, spacing },
                    Some(validated_source_tex),
                    validated_alternative,
                    validated_language,
                    None,
                )?;
                precomposed_vectors.push(StagingM4InlineVector {
                    node_id,
                    owner_node_id: math_owner,
                    kind: StagingM4InlineVectorKind::MathVector,
                    span: lower_span(span)?,
                    image_id: ImageResourceId::new(*image_id),
                    metrics,
                    spacing,
                    source_tex: Some(source_tex),
                    alternative: alt.clone(),
                    actual_text: actual_text.clone(),
                    language: language.clone(),
                });
                true
            }
            WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. } => validate_inlines(
                children,
                validator,
                Some(span),
                math_owner,
                depth + 1,
                precomposed_vectors,
            )?,
            WireStagingM4Inline::Link {
                target, children, ..
            } => {
                let valid_target = match target {
                    WireStagingM4LinkTarget::Internal { anchor_id } => {
                        AnchorId::new(anchor_id.clone()).is_ok()
                    }
                    WireStagingM4LinkTarget::Uri { uri } => SafeUri::new(uri.clone()).is_ok(),
                };
                if !valid_target {
                    return Err(StagingSemanticSyntaxError::InvalidInline);
                }
                validate_inlines(
                    children,
                    validator,
                    Some(span),
                    math_owner,
                    depth + 1,
                    precomposed_vectors,
                )?
            }
            WireStagingM4Inline::Anchor { anchor_id, .. } => {
                if AnchorId::new(anchor_id.clone()).is_err() {
                    return Err(StagingSemanticSyntaxError::InvalidInline);
                }
                false
            }
            WireStagingM4Inline::Reference { target, .. } => {
                if AnchorId::new(target.clone()).is_err() {
                    return Err(StagingSemanticSyntaxError::InvalidInline);
                }
                true
            }
            WireStagingM4Inline::FootnoteReference { footnote_id, .. } => {
                if FootnoteId::new(footnote_id.clone()).is_err() {
                    return Err(StagingSemanticSyntaxError::InvalidInline);
                }
                true
            }
            WireStagingM4Inline::SoftBreak { .. } | WireStagingM4Inline::HardBreak { .. } => false,
        };
        has_authored_content |= inline_has_content;
    }
    Ok(has_authored_content)
}

fn invalid_precomposed_vector(
    owner: NodeId,
    field: PrecomposedVectorField,
) -> StagingSemanticSyntaxError {
    StagingSemanticSyntaxError::InvalidPrecomposedVector { owner, field }
}

fn vector_length(
    raw: i64,
    owner: NodeId,
    field: PrecomposedVectorField,
) -> Result<Length, StagingSemanticSyntaxError> {
    Length::from_raw(raw).ok_or_else(|| invalid_precomposed_vector(owner, field))
}

fn vector_positive_length(
    raw: i64,
    owner: NodeId,
    field: PrecomposedVectorField,
) -> Result<PositiveLength, StagingSemanticSyntaxError> {
    vector_length(raw, owner, field).and_then(|value| {
        PositiveLength::new(value).ok_or_else(|| invalid_precomposed_vector(owner, field))
    })
}

fn vector_nonnegative_length(
    raw: i64,
    owner: NodeId,
    field: PrecomposedVectorField,
) -> Result<NonNegativeLength, StagingSemanticSyntaxError> {
    vector_length(raw, owner, field).and_then(|value| {
        NonNegativeLength::new(value).ok_or_else(|| invalid_precomposed_vector(owner, field))
    })
}

fn lower_vector_viewport(
    wire: WirePrecomposedVectorViewport,
    owner: NodeId,
) -> Result<PrecomposedVectorViewport, StagingSemanticSyntaxError> {
    Ok(PrecomposedVectorViewport {
        width: vector_positive_length(
            wire.width,
            owner,
            PrecomposedVectorField::MetricsViewportWidth,
        )?,
        height: vector_positive_length(
            wire.height,
            owner,
            PrecomposedVectorField::MetricsViewportHeight,
        )?,
    })
}

fn lower_vector_metrics(
    wire: WirePrecomposedVectorMetrics,
    owner: NodeId,
) -> Result<PrecomposedVectorMetrics, StagingSemanticSyntaxError> {
    let metrics = PrecomposedVectorMetrics {
        advance: vector_positive_length(
            wire.advance,
            owner,
            PrecomposedVectorField::MetricsAdvance,
        )?,
        ascent: vector_positive_length(wire.ascent, owner, PrecomposedVectorField::MetricsAscent)?,
        baseline: vector_nonnegative_length(
            wire.baseline,
            owner,
            PrecomposedVectorField::MetricsBaseline,
        )?,
        descent: vector_nonnegative_length(
            wire.descent,
            owner,
            PrecomposedVectorField::MetricsDescent,
        )?,
        origin_x: vector_length(wire.origin_x, owner, PrecomposedVectorField::MetricsOriginX)?,
        viewport: lower_vector_viewport(wire.viewport, owner)?,
    };
    if metrics.baseline.get() > metrics.viewport.height.get() {
        return Err(invalid_precomposed_vector(
            owner,
            PrecomposedVectorField::MetricsBaseline,
        ));
    }
    if metrics.ascent.get() < metrics.baseline.get() {
        return Err(invalid_precomposed_vector(
            owner,
            PrecomposedVectorField::MetricsAscent,
        ));
    }
    let below_baseline = metrics
        .viewport
        .height
        .get()
        .checked_sub(metrics.baseline.get())
        .ok_or_else(|| {
            invalid_precomposed_vector(owner, PrecomposedVectorField::MetricsBaseline)
        })?;
    if metrics.descent.get() < below_baseline {
        return Err(invalid_precomposed_vector(
            owner,
            PrecomposedVectorField::MetricsDescent,
        ));
    }
    metrics
        .origin_x
        .checked_add(metrics.viewport.width.get())
        .ok_or_else(|| invalid_precomposed_vector(owner, PrecomposedVectorField::MetricsOriginX))?;
    Ok(metrics)
}

fn lower_vector_spacing(
    wire: WirePrecomposedVectorSpacing,
    owner: NodeId,
) -> Result<PrecomposedVectorSpacing, StagingSemanticSyntaxError> {
    Ok(PrecomposedVectorSpacing {
        before: vector_nonnegative_length(
            wire.before,
            owner,
            PrecomposedVectorField::SpacingBefore,
        )?,
        after: vector_nonnegative_length(wire.after, owner, PrecomposedVectorField::SpacingAfter)?,
    })
}

fn lower_vector_text_span(
    wire: WireStagingTextSpan,
    owner: NodeId,
    field: PrecomposedVectorField,
) -> Result<TextSpan, StagingSemanticSyntaxError> {
    TextSpan::new(
        TextBufferId::new(wire.text_id),
        Utf8ByteOffset::new(wire.start_byte),
        Utf8ByteOffset::new(wire.end_byte),
    )
    .ok_or_else(|| invalid_precomposed_vector(owner, field))
}

#[derive(Clone, Copy)]
enum PrecomposedVectorMappedTextPolicy {
    SourceTex,
    EquationNumber,
}

fn cached_precomposed_vector_sha256<K: Copy + Ord>(
    cache: &mut BTreeMap<K, [u8; 32]>,
    key: K,
    calculate: impl FnOnce() -> [u8; 32],
) -> [u8; 32] {
    match cache.entry(key) {
        std::collections::btree_map::Entry::Occupied(entry) => *entry.get(),
        std::collections::btree_map::Entry::Vacant(entry) => *entry.insert(calculate()),
    }
}

fn validate_precomposed_vector_mapped_text(
    wire: WireStagingTextSpan,
    validator: &mut SemanticValidator<'_>,
    owner_span: WireStagingSourceSpan,
    declared_source_span: Option<WireStagingSourceSpan>,
    owner: NodeId,
    field: PrecomposedVectorField,
    policy: PrecomposedVectorMappedTextPolicy,
) -> Result<ValidatedPrecomposedVectorTextBinding, StagingSemanticSyntaxError> {
    let text_span = lower_vector_text_span(wire, owner, field)?;
    let buffer = validator
        .text_buffers
        .get(&wire.text_id)
        .ok_or_else(|| invalid_precomposed_vector(owner, field))?;
    let start =
        usize::try_from(wire.start_byte).map_err(|_| invalid_precomposed_vector(owner, field))?;
    let end =
        usize::try_from(wire.end_byte).map_err(|_| invalid_precomposed_vector(owner, field))?;
    if start >= end
        || end > buffer.utf8.len()
        || !buffer.utf8.is_char_boundary(start)
        || !buffer.utf8.is_char_boundary(end)
    {
        return Err(invalid_precomposed_vector(owner, field));
    }
    let slice_length = u64::from(wire.end_byte - wire.start_byte);
    if slice_length > u64::from(validator.limits.get().max_text_buffer_bytes) {
        return Err(StagingSemanticSyntaxError::PrecomposedVectorTextBufferLimit { owner, field });
    }
    let exact_text = &buffer.utf8[start..end];
    match policy {
        PrecomposedVectorMappedTextPolicy::SourceTex => {
            if exact_text.contains('\0') || exact_text.contains('\u{feff}') {
                return Err(invalid_precomposed_vector(owner, field));
            }
        }
        PrecomposedVectorMappedTextPolicy::EquationNumber => {
            if !is_meaningful_precomposed_vector_text(exact_text) {
                return Err(invalid_precomposed_vector(owner, field));
            }
        }
    }

    let mut overlapping = buffer.mappings.iter().filter(|mapping| {
        mapping.text_range.start_byte < wire.end_byte
            && wire.start_byte < mapping.text_range.end_byte
    });
    let mapping = overlapping
        .next()
        .ok_or(StagingSemanticSyntaxError::InvalidSourceSpan)?;
    let mapped_source_span = mapping
        .source_span
        .ok_or(StagingSemanticSyntaxError::InvalidSourceSpan)?;
    validator.validate_span(mapped_source_span)?;
    if overlapping.next().is_some()
        || mapping.text_range.start_byte != wire.start_byte
        || mapping.text_range.end_byte != wire.end_byte
        || mapping.kind != WireStagingTextMapKind::Identity
        || mapped_source_span
            .end_byte
            .checked_sub(mapped_source_span.start_byte)
            != Some(wire.end_byte - wire.start_byte)
        || declared_source_span.is_some_and(|declared| declared != mapped_source_span)
    {
        return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
    }
    validate_owned_span(owner_span, mapped_source_span)?;
    let text_buffer_sha256 = cached_precomposed_vector_sha256(
        &mut validator.precomposed_vector_text_buffer_sha256,
        wire.text_id,
        || sha256(buffer.utf8.as_bytes()),
    );
    let slice_key = (wire.text_id, wire.start_byte, wire.end_byte);
    let exact_text_sha256 = cached_precomposed_vector_sha256(
        &mut validator.precomposed_vector_text_slice_sha256,
        slice_key,
        || sha256(exact_text.as_bytes()),
    );
    Ok(ValidatedPrecomposedVectorTextBinding {
        text_span,
        mapped_source_span: lower_span(mapped_source_span)?,
        text_buffer_sha256,
        exact_text_sha256,
    })
}

fn lower_vector_source_tex(
    wire: WirePrecomposedVectorSourceTex,
    validator: &mut SemanticValidator<'_>,
    owner_span: WireStagingSourceSpan,
    owner: NodeId,
) -> Result<
    (
        PrecomposedVectorSourceTex,
        ValidatedPrecomposedVectorTextBinding,
    ),
    StagingSemanticSyntaxError,
> {
    let binding = validate_precomposed_vector_mapped_text(
        wire.text_span,
        validator,
        owner_span,
        None,
        owner,
        PrecomposedVectorField::SourceTexTextSpan,
        PrecomposedVectorMappedTextPolicy::SourceTex,
    )?;
    Ok((
        PrecomposedVectorSourceTex {
            text_span: binding.text_span(),
        },
        binding,
    ))
}

fn is_meaningful_precomposed_vector_text(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .any(|character| !is_unicode_16_white_space(character))
        && !value.chars().any(|character| {
            ('\u{0000}'..='\u{001f}').contains(&character)
                || ('\u{007f}'..='\u{009f}').contains(&character)
        })
}

fn charge_precomposed_vector_authored_text(
    validator: &mut SemanticValidator<'_>,
    owner: NodeId,
    field: PrecomposedVectorField,
    value: &str,
) -> Result<u64, StagingSemanticSyntaxError> {
    let bytes = u64::try_from(value.len()).map_err(|_| {
        StagingSemanticSyntaxError::PrecomposedVectorTextBufferLimit { owner, field }
    })?;
    if bytes > u64::from(validator.limits.get().max_text_buffer_bytes) {
        return Err(StagingSemanticSyntaxError::PrecomposedVectorTextBufferLimit { owner, field });
    }
    let total = validator
        .admitted_text_and_math_speech_bytes
        .checked_add(bytes)
        .ok_or(StagingSemanticSyntaxError::PrecomposedVectorTextAggregateLimit { owner, field })?;
    if total > validator.limits.get().max_text_bytes {
        return Err(
            StagingSemanticSyntaxError::PrecomposedVectorTextAggregateLimit { owner, field },
        );
    }
    validator.admitted_text_and_math_speech_bytes = total;
    Ok(bytes)
}

fn validate_precomposed_vector_alternative(
    validator: &mut SemanticValidator<'_>,
    owner: NodeId,
    kind: PrecomposedVectorKind,
    alternative: &str,
    actual_text: Option<&str>,
) -> Result<ValidatedPrecomposedVectorAlternative, StagingSemanticSyntaxError> {
    if !is_meaningful_precomposed_vector_text(alternative) {
        return Err(invalid_precomposed_vector(
            owner,
            PrecomposedVectorField::Alternative,
        ));
    }
    charge_precomposed_vector_authored_text(
        validator,
        owner,
        PrecomposedVectorField::Alternative,
        alternative,
    )?;
    if let Some(value) = actual_text {
        if !is_meaningful_precomposed_vector_text(value) {
            return Err(invalid_precomposed_vector(
                owner,
                PrecomposedVectorField::ActualText,
            ));
        }
        charge_precomposed_vector_authored_text(
            validator,
            owner,
            PrecomposedVectorField::ActualText,
            value,
        )?;
    }
    let resolution = if actual_text.is_some() {
        PrecomposedVectorActualTextResolution::Authored
    } else if kind.is_math() {
        PrecomposedVectorActualTextResolution::AlternativeFallback
    } else {
        PrecomposedVectorActualTextResolution::Absent
    };
    Ok(ValidatedPrecomposedVectorAlternative {
        alternative: alternative.to_owned(),
        authored_actual_text: actual_text.map(str::to_owned),
        resolution,
    })
}

fn validate_precomposed_vector_language(
    validator: &mut SemanticValidator<'_>,
    owner: NodeId,
    raw: Option<&str>,
) -> Result<Option<ValidatedPrecomposedVectorLanguageOverride>, StagingSemanticSyntaxError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let canonical = crate::canonicalize_bcp47_language(raw)
        .map_err(|_| invalid_precomposed_vector(owner, PrecomposedVectorField::Language))?;
    let mut charged_bytes = 0u64;
    if raw != canonical {
        charged_bytes = charge_precomposed_vector_authored_text(
            validator,
            owner,
            PrecomposedVectorField::Language,
            raw,
        )?;
    }
    charged_bytes = charged_bytes
        .checked_add(charge_precomposed_vector_authored_text(
            validator,
            owner,
            PrecomposedVectorField::Language,
            &canonical,
        )?)
        .ok_or(
            StagingSemanticSyntaxError::PrecomposedVectorTextAggregateLimit {
                owner,
                field: PrecomposedVectorField::Language,
            },
        )?;
    Ok(Some(ValidatedPrecomposedVectorLanguageOverride {
        raw: raw.to_owned(),
        canonical,
        charged_bytes,
    }))
}

fn lower_vector_equation_number(
    wire: &WirePrecomposedVectorEquationNumber,
    validator: &mut SemanticValidator<'_>,
    owner_span: WireStagingSourceSpan,
    formula_source_span: SourceSpan,
    owner: NodeId,
    depth: u32,
) -> Result<
    (
        PrecomposedVectorEquationNumber,
        ValidatedPrecomposedVectorEquationNumber,
    ),
    StagingSemanticSyntaxError,
> {
    if wire.node_id != validator.next_node_id {
        return Err(invalid_precomposed_vector(
            owner,
            PrecomposedVectorField::EquationNumberNodeId,
        ));
    }
    validator.precomposed_vector_node(wire.node_id, Some(wire.span), depth)?;
    validate_owned_span(owner_span, wire.span)?;
    if formula_source_span.source_id().get() != wire.span.source_id
        || formula_source_span.end_byte().get() > wire.span.start_byte
    {
        return Err(invalid_precomposed_vector(
            owner,
            PrecomposedVectorField::EquationNumberSpan,
        ));
    }
    let minimum_gap = vector_positive_length(
        wire.minimum_gap,
        owner,
        PrecomposedVectorField::EquationNumberMinimumGap,
    )?;
    let text = validate_precomposed_vector_mapped_text(
        wire.text_span,
        validator,
        owner_span,
        Some(wire.span),
        owner,
        PrecomposedVectorField::EquationNumberTextSpan,
        PrecomposedVectorMappedTextPolicy::EquationNumber,
    )?;
    let span = lower_span(wire.span)?;
    let node_id = NodeId::new(wire.node_id);
    Ok((
        PrecomposedVectorEquationNumber {
            minimum_gap,
            node_id,
            span,
            text_span: text.text_span(),
        },
        ValidatedPrecomposedVectorEquationNumber {
            node_id,
            span,
            minimum_gap,
            text,
        },
    ))
}

#[allow(clippy::too_many_arguments)] // exact sealed receipt inputs
fn issue_precomposed_vector_metrics(
    validator: &mut SemanticValidator<'_>,
    node_id: NodeId,
    owner_span: WireStagingSourceSpan,
    kind: PrecomposedVectorKind,
    image_id: ImageResourceId,
    payload: PrecomposedVectorMetricPayload,
    source_tex: Option<ValidatedPrecomposedVectorTextBinding>,
    alternative: ValidatedPrecomposedVectorAlternative,
    language: Option<ValidatedPrecomposedVectorLanguageOverride>,
    equation_number: Option<ValidatedPrecomposedVectorEquationNumber>,
) -> Result<(), StagingSemanticSyntaxError> {
    let shape_matches = matches!(
        (kind, payload),
        (
            PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::MathVector,
            PrecomposedVectorMetricPayload::Inline { .. },
        ) | (
            PrecomposedVectorKind::MathVectorBlock,
            PrecomposedVectorMetricPayload::MathBlock { .. },
        ) | (
            PrecomposedVectorKind::VectorFigure,
            PrecomposedVectorMetricPayload::Figure { .. }
        )
    );
    let equation_shape_matches =
        equation_number.is_none() || kind == PrecomposedVectorKind::MathVectorBlock;
    if !shape_matches
        || kind.is_math() != source_tex.is_some()
        || !equation_shape_matches
        || validator
            .precomposed_vector_metrics
            .last()
            .is_some_and(|previous| previous.node_id() >= node_id)
    {
        return Err(StagingSemanticSyntaxError::ReceiptMismatch);
    }
    validator
        .precomposed_vector_metrics
        .try_reserve(1)
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    let mut receipt = ValidatedPrecomposedVectorMetrics {
        package_sha256: validator.canonical_package_sha256,
        session: validator.precomposed_vector_session.clone(),
        limits_fingerprint: validator.precomposed_vector_limits_fingerprint,
        node_id,
        owner_source_span: lower_span(owner_span)?,
        kind,
        resource: UnresolvedPrecomposedVectorResourceBinding { image_id },
        payload,
        source_tex,
        alternative,
        language,
        equation_number,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_precomposed_vector_metrics_receipt(&receipt);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    validator.precomposed_vector_metrics.push(receipt);
    Ok(())
}

fn validate_text_span(
    value: WireStagingTextSpan,
    validator: &SemanticValidator<'_>,
) -> Result<bool, StagingSemanticSyntaxError> {
    let text = &validator
        .text_buffers
        .get(&value.text_id)
        .ok_or(StagingSemanticSyntaxError::InvalidInline)?
        .utf8;
    let start_index =
        usize::try_from(value.start_byte).map_err(|_| StagingSemanticSyntaxError::InvalidInline)?;
    let end_index =
        usize::try_from(value.end_byte).map_err(|_| StagingSemanticSyntaxError::InvalidInline)?;
    if value.start_byte > value.end_byte
        || end_index > text.len()
        || !text.is_char_boundary(start_index)
        || !text.is_char_boundary(end_index)
    {
        return Err(StagingSemanticSyntaxError::InvalidInline);
    }
    Ok(value.start_byte < value.end_byte)
}

fn wire_block_span(block: &WireStagingM4Block) -> WireStagingSourceSpan {
    match block {
        WireStagingM4Block::Paragraph { span, .. }
        | WireStagingM4Block::Heading { span, .. }
        | WireStagingM4Block::List { span, .. }
        | WireStagingM4Block::Table { span, .. }
        | WireStagingM4Block::Figure { span, .. }
        | WireStagingM4Block::PageBreak { span, .. }
        | WireStagingM4Block::DisplayMath { span, .. }
        | WireStagingM4Block::VectorFigure { span, .. }
        | WireStagingM4Block::MathVectorBlock { span, .. }
        | WireStagingM4Block::SemanticContainer { span, .. } => *span,
    }
}

fn validate_owned_span(
    owner: WireStagingSourceSpan,
    child: WireStagingSourceSpan,
) -> Result<(), StagingSemanticSyntaxError> {
    if owner.source_id != child.source_id
        || child.start_byte < owner.start_byte
        || child.end_byte > owner.end_byte
    {
        return Err(StagingSemanticSyntaxError::InvalidSourceSpan);
    }
    Ok(())
}

fn lower_span(span: WireStagingSourceSpan) -> Result<SourceSpan, StagingSemanticSyntaxError> {
    SourceSpan::new(
        SourceId::new(span.source_id),
        Utf8ByteOffset::new(span.start_byte),
        Utf8ByteOffset::new(span.end_byte),
    )
    .ok_or(StagingSemanticSyntaxError::InvalidSourceSpan)
}

fn validate_classes(classes: &[String]) -> Result<(), StagingSemanticSyntaxError> {
    let mut previous: Option<&[u8]> = None;
    for class in classes {
        if class == INTERNAL_HIDDEN_STYLE_CLASS
            || !is_style_identifier(class)
            || previous.is_some_and(|value| value >= class.as_bytes())
        {
            return Err(StagingSemanticSyntaxError::InvalidClass);
        }
        previous = Some(class.as_bytes());
    }
    Ok(())
}

fn lower_resources(
    wire: &WireStagingM4ResourceCatalog,
) -> Result<StagingM4ResourceCatalog, StagingSemanticSyntaxError> {
    let mut font_faces = Vec::new();
    let mut families = BTreeSet::new();
    font_faces
        .try_reserve_exact(wire.font_faces.len())
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    for (index, font) in wire.font_faces.iter().enumerate() {
        if usize::try_from(font.font_face_id) != Ok(index)
            || font.family.trim().is_empty()
            || font.family.chars().any(char::is_control)
            || !families.insert(font.family.as_str())
            || (matches!(
                font.media_type,
                WireFontMediaType::SfntTrueTypeGlyf | WireFontMediaType::SfntCff1
            ) && font.face_index != 0)
        {
            return Err(StagingSemanticSyntaxError::InvalidResource);
        }
        font_faces.push(StagingM4FontFaceDeclaration {
            font_face_id: FontFaceId::new(font.font_face_id),
            family: font.family.clone(),
            uri: PortablePath::new(font.uri.clone())
                .map_err(|_| StagingSemanticSyntaxError::InvalidResource)?,
            face_index: font.face_index,
            expected_sha256: parse_optional_hash(font.expected_sha256.as_deref())?,
            media: FontMediaDeclaration::Declared(match font.media_type {
                WireFontMediaType::SfntTrueTypeGlyf => FontMediaType::SfntTrueTypeGlyf,
                WireFontMediaType::TtcTrueTypeGlyf => FontMediaType::TtcTrueTypeGlyf,
                WireFontMediaType::SfntCff1 => FontMediaType::SfntCff1,
            }),
        });
    }
    let mut images = Vec::new();
    images
        .try_reserve_exact(wire.images.len())
        .map_err(|_| StagingSemanticSyntaxError::AllocationFailure)?;
    for (index, image) in wire.images.iter().enumerate() {
        if usize::try_from(image.image_id) != Ok(index) {
            return Err(StagingSemanticSyntaxError::InvalidResource);
        }
        let expected_sha256 = parse_optional_hash(image.expected_sha256.as_deref())?;
        let (media, vector_provenance) = match image.media_type {
            WireImageMediaType::Png => {
                if image.vector_provenance.is_some() {
                    return Err(StagingSemanticSyntaxError::InvalidResource);
                }
                (ImageMediaType::Png, None)
            }
            WireImageMediaType::JpegBaseline => {
                if image.vector_provenance.is_some() {
                    return Err(StagingSemanticSyntaxError::InvalidResource);
                }
                (ImageMediaType::JpegBaseline, None)
            }
            WireImageMediaType::SvgSafe1 => {
                if image.vector_provenance.is_some() {
                    return Err(StagingSemanticSyntaxError::InvalidResource);
                }
                (ImageMediaType::SvgSafe1, None)
            }
            WireImageMediaType::SvgSafe2 => {
                let provenance = image
                    .vector_provenance
                    .as_ref()
                    .ok_or(StagingSemanticSyntaxError::InvalidResource)?;
                if expected_sha256.is_none()
                    || !is_valid_precomposed_vector_provenance(&provenance.engine_id)
                    || !is_valid_precomposed_vector_provenance(&provenance.engine_version)
                    || !is_valid_precomposed_vector_provenance(&provenance.rules_version)
                {
                    return Err(StagingSemanticSyntaxError::InvalidResource);
                }
                (
                    ImageMediaType::SvgSafe2,
                    Some(VectorProvenance {
                        engine_id: provenance.engine_id.clone(),
                        engine_version: provenance.engine_version.clone(),
                        rules_version: provenance.rules_version.clone(),
                    }),
                )
            }
        };
        images.push(StagingM4ImageDeclaration {
            image_id: ImageResourceId::new(image.image_id),
            uri: PortablePath::new(image.uri.clone())
                .map_err(|_| StagingSemanticSyntaxError::InvalidResource)?,
            expected_sha256,
            media: ImageMediaDeclaration::Declared(media),
            vector_provenance,
        });
    }
    Ok(StagingM4ResourceCatalog { font_faces, images })
}

fn is_valid_precomposed_vector_provenance(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
}

fn parse_optional_hash(
    value: Option<&str>,
) -> Result<Option<[u8; 32]>, StagingSemanticSyntaxError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.len() != 64 {
        return Err(StagingSemanticSyntaxError::InvalidResource);
    }
    let mut result = [0u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let digit = |byte: u8| match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            _ => None,
        };
        result[index] = digit(pair[0])
            .and_then(|high| digit(pair[1]).map(|low| high * 16 + low))
            .ok_or(StagingSemanticSyntaxError::InvalidResource)?;
    }
    Ok(Some(result))
}

struct StagingSemanticStyleSheets {
    semantic: StyleSheet,
    ordinary: StyleSheet,
    math: StyleSheet,
    vector: StyleSheet,
}

fn lower_semantic_style_rules(
    sheet: &WireStagingStyleSheet,
    limits: &ValidatedResourceLimits,
) -> Result<StagingSemanticStyleSheets, StagingSemanticSyntaxError> {
    let rules = &sheet.rules;
    if u64::try_from(rules.len()).map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?
        > limits.get().max_style_rules
    {
        return Err(StagingSemanticSyntaxError::InvalidStyle);
    }
    let mut parsed = Vec::new();
    let mut semantic_rules = Vec::new();
    let mut math_rules = Vec::new();
    let mut vector_rules = Vec::new();
    for (index, rule) in rules.iter().enumerate() {
        let source_order = rule.source_order;
        if usize::try_from(source_order) != Ok(index) {
            return Err(StagingSemanticSyntaxError::InvalidStyle);
        }
        let selector = rule.selector.as_str();
        let mut parts = selector.split('.');
        let block_type = parts
            .next()
            .ok_or(StagingSemanticSyntaxError::InvalidStyle)?;
        if !matches!(
            block_type,
            "paragraph"
                | "heading"
                | "list"
                | "table"
                | "figure"
                | "page_break"
                | "semantic_container"
                | "display_math"
                | "math_vector_block"
                | "vector_figure"
        ) {
            return Err(StagingSemanticSyntaxError::InvalidStyle);
        }
        let required_classes: Vec<String> = parts.map(str::to_owned).collect();
        validate_classes(&required_classes)?;
        let style_id = StyleId::new(rule.style_id.clone())
            .map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
        let extends = rule
            .extends
            .as_ref()
            .map(|value| {
                StyleId::new(value.clone()).map_err(|_| StagingSemanticSyntaxError::InvalidStyle)
            })
            .transpose()?;
        let mut typed_declarations = Vec::new();
        for declaration in &rule.declarations {
            let name = declaration.name.as_str();
            if BasicStyleProperty::from_str(name).is_none() {
                return Err(StagingSemanticSyntaxError::InvalidStyle);
            }
            typed_declarations.push(Declaration {
                name: name.to_owned(),
                value: lower_semantic_style_value(&declaration.value)?,
                important: declaration.important,
            });
        }
        let mapped_selector = if matches!(block_type, "semantic_container" | "display_math") {
            format!("paragraph{}", &selector[block_type.len()..])
        } else {
            selector.to_owned()
        };
        parsed.push(StyleRule {
            style_id,
            extends,
            selector: mapped_selector,
            source_order,
            declarations: typed_declarations,
        });
        semantic_rules.push(block_type == "semantic_container");
        math_rules.push(block_type == "display_math");
        vector_rules.push(matches!(block_type, "math_vector_block" | "vector_figure"));
    }
    let validation_sheet = StyleSheet {
        rules: parsed.clone(),
    };
    validation_sheet
        .validate_staging_precomposed_vector_style_shape()
        .map_err(map_semantic_style_error)?;

    let by_id: BTreeMap<&StyleId, usize> = parsed
        .iter()
        .enumerate()
        .map(|(index, rule)| (&rule.style_id, index))
        .collect();
    for (index, rule) in parsed.iter().enumerate() {
        if let Some(parent) = rule.extends.as_ref() {
            let parent_index = *by_id
                .get(parent)
                .ok_or(StagingSemanticSyntaxError::InvalidStyle)?;
            if vector_rules[index] != vector_rules[parent_index] {
                return Err(StagingSemanticSyntaxError::InvalidStyle);
            }
        }
    }

    let mut current_parsed = Vec::new();
    let mut current_semantic_rules = Vec::new();
    let mut current_math_rules = Vec::new();
    let mut vector_parsed = Vec::new();
    for (index, mut rule) in parsed.into_iter().enumerate() {
        if vector_rules[index] {
            rule.source_order = u32::try_from(vector_parsed.len())
                .map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
            vector_parsed.push(rule);
        } else {
            rule.source_order = u32::try_from(current_parsed.len())
                .map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
            current_parsed.push(rule);
            current_semantic_rules.push(semantic_rules[index]);
            current_math_rules.push(math_rules[index]);
        }
    }
    let current_validation_sheet = StyleSheet {
        rules: current_parsed.clone(),
    };
    current_validation_sheet
        .validate_table_document_styles()
        .map_err(map_semantic_style_error)?;
    let vector = StyleSheet {
        rules: vector_parsed,
    };
    vector
        .validate_precomposed_vector_styles()
        .map_err(map_semantic_style_error)?;

    let mut ordinary_rules = current_parsed.clone();
    for (index, rule) in ordinary_rules.iter_mut().enumerate() {
        if current_semantic_rules[index] || current_math_rules[index] {
            // This sheet is queried only for list/table/figure ancestors.
            // Keeping semantic rules on paragraph preserves `extends` edges
            // by routing it through a reserved class rejected from authored input.
            rule.selector = format!("paragraph.{INTERNAL_HIDDEN_STYLE_CLASS}");
        }
    }
    let ordinary = StyleSheet {
        rules: ordinary_rules,
    };
    ordinary
        .validate_table_document_styles()
        .map_err(map_semantic_style_error)?;

    let semantic = isolate_staging_style_rules(&current_parsed, &current_semantic_rules)?;
    let math = isolate_staging_style_rules(&current_parsed, &current_math_rules)?;
    Ok(StagingSemanticStyleSheets {
        semantic,
        ordinary,
        math,
        vector,
    })
}

fn isolate_staging_style_rules(
    parsed: &[StyleRule],
    direct: &[bool],
) -> Result<StyleSheet, StagingSemanticSyntaxError> {
    let by_id: BTreeMap<&StyleId, usize> = parsed
        .iter()
        .enumerate()
        .map(|(index, rule)| (&rule.style_id, index))
        .collect();
    let mut included = BTreeSet::new();
    for (index, include) in direct.iter().copied().enumerate() {
        if !include {
            continue;
        }
        let mut current = Some(index);
        while let Some(rule_index) = current {
            if !included.insert(rule_index) {
                break;
            }
            current = parsed[rule_index]
                .extends
                .as_ref()
                .and_then(|parent| by_id.get(parent).copied());
        }
    }
    let mut cascade_rules = Vec::new();
    for (original_index, mut rule) in parsed.iter().cloned().enumerate() {
        if !included.contains(&original_index) {
            continue;
        }
        if rule.declarations.iter().any(|declaration| {
            matches!(
                BasicStyleProperty::from_str(&declaration.name),
                Some(BasicStyleProperty::Width | BasicStyleProperty::KeepCaption)
            )
        }) {
            return Err(StagingSemanticSyntaxError::InapplicableStyle);
        }
        if !direct[original_index] {
            rule.selector = format!("paragraph.{INTERNAL_HIDDEN_STYLE_CLASS}");
        }
        rule.source_order = u32::try_from(cascade_rules.len())
            .map_err(|_| StagingSemanticSyntaxError::InvalidStyle)?;
        cascade_rules.push(rule);
    }
    let sheet = StyleSheet {
        rules: cascade_rules,
    };
    sheet
        .validate_basic_document_styles()
        .map_err(map_semantic_style_error)?;
    Ok(sheet)
}

fn lower_semantic_style_value(
    value: &WireStagingStyleValue,
) -> Result<StyleValue, StagingSemanticSyntaxError> {
    match value {
        WireStagingStyleValue::Keyword { value } => Ok(StyleValue::Keyword(value.clone())),
        WireStagingStyleValue::String { value } => Ok(StyleValue::Text(value.clone())),
        WireStagingStyleValue::Integer { value } => Ok(StyleValue::Integer(*value)),
        WireStagingStyleValue::Length { value } => Length::from_raw(*value)
            .map(StyleValue::Length)
            .ok_or(StagingSemanticSyntaxError::InvalidStyle),
        WireStagingStyleValue::Boolean { value } => Ok(StyleValue::Boolean(*value)),
        WireStagingStyleValue::FontFamilyList { families } => {
            Ok(StyleValue::FontFamilyList(families.clone()))
        }
        WireStagingStyleValue::Ratio {
            numerator,
            denominator,
        } => NonZeroU64::new(*denominator)
            .map(|denominator| StyleValue::Ratio {
                numerator: *numerator,
                denominator,
            })
            .ok_or(StagingSemanticSyntaxError::InvalidStyle),
    }
}

fn map_semantic_style_error(error: StyleValidationError) -> StagingSemanticSyntaxError {
    match error {
        StyleValidationError::InapplicableProperty => StagingSemanticSyntaxError::InapplicableStyle,
        _ => StagingSemanticSyntaxError::InvalidStyle,
    }
}

fn collect_computed_styles(
    blocks: &[StagingM4Block],
    rules: &StagingSemanticStyleSheets,
    parent: Option<&SemanticContainerInheritanceStyle>,
    pending_math: &[PendingStagingMathNode],
    output: &mut BTreeMap<NodeId, SemanticContainerComputedStyle>,
    vector_output: &mut BTreeMap<NodeId, PrecomposedVectorComputedStyleReceipt>,
    math_output: &mut BTreeMap<NodeId, StagingMathComputedStyle>,
) -> Result<(), StagingSemanticSyntaxError> {
    for block in blocks {
        match block {
            StagingM4Block::SemanticContainer {
                common,
                semantic_kind,
                blocks,
            } => {
                let kind = match semantic_kind {
                    SemanticContainerKind::Result => SemanticContainerStyleKind::Result,
                    SemanticContainerKind::Proof => SemanticContainerStyleKind::Proof,
                    SemanticContainerKind::Exercise => SemanticContainerStyleKind::Exercise,
                };
                let style = cascade_staging_semantic_container_style(
                    kind,
                    &common.classes,
                    &rules.semantic,
                    parent,
                )
                .map_err(map_semantic_style_error)?;
                let inheritance = style.inheritance_style().clone();
                if output.insert(common.node_id, style).is_some() {
                    return Err(StagingSemanticSyntaxError::InvalidNodeOrder);
                }
                collect_computed_styles(
                    blocks,
                    rules,
                    Some(&inheritance),
                    pending_math,
                    output,
                    vector_output,
                    math_output,
                )?;
            }
            StagingM4Block::Paragraph { common, .. } | StagingM4Block::Heading { common, .. } => {
                let block_type = match block {
                    StagingM4Block::Paragraph { .. } => "paragraph",
                    StagingM4Block::Heading { .. } => "heading",
                    _ => unreachable!("matched paragraph or heading"),
                };
                let inheritance = cascade_staging_semantic_descendant_style(
                    block_type,
                    &common.classes,
                    &rules.ordinary,
                    parent,
                )
                .map_err(map_semantic_style_error)?;
                for math in pending_math.iter().filter(|math| {
                    math.domain.owner_node_id == common.node_id
                        && math.domain.kind == StagingM4MathKind::Inline
                }) {
                    let style = close_staging_inline_math_style(&inheritance)
                        .map_err(map_semantic_style_error)?;
                    if math_output.insert(math.domain.node_id, style).is_some() {
                        return Err(StagingSemanticSyntaxError::InvalidNodeOrder);
                    }
                }
            }
            StagingM4Block::DisplayMath { common } => {
                let style =
                    cascade_staging_display_math_style(&common.classes, &rules.math, parent)
                        .map_err(map_semantic_style_error)?;
                if !pending_math.iter().any(|math| {
                    math.domain.node_id == common.node_id
                        && math.domain.kind == StagingM4MathKind::Display
                }) || math_output.insert(common.node_id, style).is_some()
                {
                    return Err(StagingSemanticSyntaxError::ReceiptMismatch);
                }
            }
            StagingM4Block::List { common, items } => {
                let inheritance = cascade_staging_semantic_descendant_style(
                    "list",
                    &common.classes,
                    &rules.ordinary,
                    parent,
                )
                .map_err(map_semantic_style_error)?;
                for item in items {
                    collect_computed_styles(
                        &item.blocks,
                        rules,
                        Some(&inheritance),
                        pending_math,
                        output,
                        vector_output,
                        math_output,
                    )?;
                }
            }
            StagingM4Block::Table { common, head, body } => {
                let inheritance = cascade_staging_semantic_descendant_style(
                    "table",
                    &common.classes,
                    &rules.ordinary,
                    parent,
                )
                .map_err(map_semantic_style_error)?;
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_computed_styles(
                        &cell.blocks,
                        rules,
                        Some(&inheritance),
                        pending_math,
                        output,
                        vector_output,
                        math_output,
                    )?;
                }
            }
            StagingM4Block::Figure {
                common, caption, ..
            } => {
                let inheritance = cascade_staging_semantic_descendant_style(
                    "figure",
                    &common.classes,
                    &rules.ordinary,
                    parent,
                )
                .map_err(map_semantic_style_error)?;
                collect_computed_styles(
                    caption,
                    rules,
                    Some(&inheritance),
                    pending_math,
                    output,
                    vector_output,
                    math_output,
                )?
            }
            StagingM4Block::VectorFigure {
                common, caption, ..
            } => {
                let style = rules
                    .vector
                    .cascade_precomposed_vector_style(
                        PrecomposedVectorStyleKind::VectorFigure,
                        &common.classes,
                        parent,
                    )
                    .map_err(map_semantic_style_error)?;
                if vector_output.insert(common.node_id, style).is_some() {
                    return Err(StagingSemanticSyntaxError::InvalidNodeOrder);
                }
                collect_computed_styles(
                    caption,
                    rules,
                    parent,
                    pending_math,
                    output,
                    vector_output,
                    math_output,
                )?
            }
            StagingM4Block::MathVectorBlock { common, .. } => {
                let style = rules
                    .vector
                    .cascade_precomposed_vector_style(
                        PrecomposedVectorStyleKind::MathVectorBlock,
                        &common.classes,
                        parent,
                    )
                    .map_err(map_semantic_style_error)?;
                if vector_output.insert(common.node_id, style).is_some() {
                    return Err(StagingSemanticSyntaxError::InvalidNodeOrder);
                }
            }
            StagingM4Block::PageBreak { .. } => {}
        }
    }
    Ok(())
}

fn precomposed_vector_limits_fingerprint(limits: &ValidatedResourceLimits) -> [u8; 32] {
    let mut canonical_jcs = String::from("{\"effective_limits\":{");
    push_profile_limits(&mut canonical_jcs, limits);
    canonical_jcs.push_str("}}");
    sha256(canonical_jcs.as_bytes())
}

fn encode_precomposed_vector_metrics_receipt(
    receipt: &ValidatedPrecomposedVectorMetrics,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, PRECOMPOSED_VECTOR_METRICS_ALGORITHM);
    output.push_str(",\"alternative\":{\"alt_sha256\":");
    push_hash(&mut output, receipt.alternative.alternative_sha256());
    output.push_str(",\"authored_actual_text_sha256\":");
    push_optional_vector_hash(
        &mut output,
        receipt.alternative.authored_actual_text_sha256(),
    );
    output.push_str(",\"resolution\":");
    push_jcs_string(&mut output, receipt.alternative.resolution().as_str());
    output.push_str(",\"resolved_actual_text_sha256\":");
    push_optional_vector_hash(
        &mut output,
        receipt.alternative.resolved_actual_text_sha256(),
    );
    output.push_str("},\"canonical_package_sha256\":");
    push_hash(&mut output, receipt.package_sha256);
    output.push_str(",\"contract\":");
    push_jcs_string(
        &mut output,
        typaxis_document_package::STAGING_SEMANTIC_DOCUMENT_PACKAGE_CONTRACT,
    );
    output.push_str(",\"equation_number\":");
    match &receipt.equation_number {
        Some(number) => {
            output.push_str("{\"minimum_gap\":");
            output.push_str(&number.minimum_gap.get().raw().to_string());
            output.push_str(",\"node_id\":");
            output.push_str(&number.node_id.get().to_string());
            output.push_str(",\"source_span\":");
            push_vector_source_span_jcs(&mut output, number.span);
            output.push_str(",\"text\":");
            push_vector_text_binding_jcs(&mut output, &number.text);
            output.push('}');
        }
        None => output.push_str("null"),
    }
    output.push_str(",\"image_id\":");
    output.push_str(&receipt.resource.image_id.get().to_string());
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, receipt.kind.as_str());
    output.push_str(",\"language\":");
    match &receipt.language {
        Some(language) => {
            output.push_str("{\"canonical\":");
            push_jcs_string(&mut output, &language.canonical);
            output.push_str(",\"charged_bytes\":");
            output.push_str(&language.charged_bytes.to_string());
            output.push_str(",\"raw\":");
            push_jcs_string(&mut output, &language.raw);
            output.push('}');
        }
        None => output.push_str("null"),
    }
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, receipt.limits_fingerprint);
    output.push_str(",\"metrics\":");
    push_vector_metric_payload_jcs(&mut output, receipt.payload);
    output.push_str(",\"node_id\":");
    output.push_str(&receipt.node_id.get().to_string());
    output.push_str(",\"owner_source_span\":");
    push_vector_source_span_jcs(&mut output, receipt.owner_source_span);
    output.push_str(",\"resource_binding\":{\"image_id\":");
    output.push_str(&receipt.resource.image_id.get().to_string());
    output.push_str(",\"state\":\"unresolved\"},\"source_tex\":");
    match &receipt.source_tex {
        Some(source_tex) => push_vector_text_binding_jcs(&mut output, source_tex),
        None => output.push_str("null"),
    }
    output.push('}');
    output
}

fn push_optional_vector_hash(output: &mut String, value: Option<[u8; 32]>) {
    match value {
        Some(value) => push_hash(output, value),
        None => output.push_str("null"),
    }
}

fn push_vector_text_binding_jcs(
    output: &mut String,
    value: &ValidatedPrecomposedVectorTextBinding,
) {
    output.push_str("{\"exact_slice_sha256\":");
    push_hash(output, value.exact_text_sha256);
    output.push_str(",\"mapped_source_span\":");
    push_vector_source_span_jcs(output, value.mapped_source_span);
    output.push_str(",\"text_buffer_sha256\":");
    push_hash(output, value.text_buffer_sha256);
    output.push_str(",\"text_span\":");
    push_vector_text_span_jcs(output, value.text_span);
    output.push('}');
}

fn push_vector_source_span_jcs(output: &mut String, value: SourceSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&value.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&value.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&value.start_byte().get().to_string());
    output.push('}');
}

fn push_vector_text_span_jcs(output: &mut String, value: TextSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&value.end_byte().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&value.start_byte().get().to_string());
    output.push_str(",\"text_id\":");
    output.push_str(&value.text_id().get().to_string());
    output.push('}');
}

fn push_vector_metric_payload_jcs(output: &mut String, value: PrecomposedVectorMetricPayload) {
    output.push('{');
    match value {
        PrecomposedVectorMetricPayload::Inline { metrics, spacing } => {
            push_vector_metric_scalar_members_jcs(output, metrics);
            output.push_str(",\"spacing\":{\"after\":");
            output.push_str(&spacing.after.get().raw().to_string());
            output.push_str(",\"before\":");
            output.push_str(&spacing.before.get().raw().to_string());
            output.push_str("},\"viewport\":");
            push_vector_viewport_jcs(output, metrics.viewport);
        }
        PrecomposedVectorMetricPayload::MathBlock { metrics } => {
            push_vector_metric_scalar_members_jcs(output, metrics);
            output.push_str(",\"viewport\":");
            push_vector_viewport_jcs(output, metrics.viewport);
        }
        PrecomposedVectorMetricPayload::Figure { viewport } => {
            output.push_str("\"viewport\":");
            push_vector_viewport_jcs(output, viewport);
        }
    }
    output.push('}');
}

fn push_vector_metric_scalar_members_jcs(output: &mut String, metrics: PrecomposedVectorMetrics) {
    output.push_str("\"advance\":");
    output.push_str(&metrics.advance.get().raw().to_string());
    output.push_str(",\"ascent\":");
    output.push_str(&metrics.ascent.get().raw().to_string());
    output.push_str(",\"baseline\":");
    output.push_str(&metrics.baseline.get().raw().to_string());
    output.push_str(",\"descent\":");
    output.push_str(&metrics.descent.get().raw().to_string());
    output.push_str(",\"origin_x\":");
    output.push_str(&metrics.origin_x.raw().to_string());
}

fn push_vector_viewport_jcs(output: &mut String, viewport: PrecomposedVectorViewport) {
    output.push_str("{\"height\":");
    output.push_str(&viewport.height.get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&viewport.width.get().raw().to_string());
    output.push('}');
}

fn encode_semantic_receipt(
    document: &StagingM4Document,
    resources: &StagingM4ResourceCatalog,
    styles: &BTreeMap<NodeId, SemanticContainerComputedStyle>,
    math: &[ValidatedStagingMathNode],
    precomposed_vectors: &[ValidatedPrecomposedVectorMetrics],
    canonical_package: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, SEMANTIC_SYNTAX_FINGERPRINT_ALGORITHM);
    output.push_str(",\"canonical_package_sha256\":");
    push_hash(&mut output, canonical_package);
    output.push_str(",\"containers\":[");
    let mut first = true;
    encode_container_records(&document.blocks, styles, &mut first, &mut output);
    for footnote in &document.footnotes {
        encode_container_records(&footnote.blocks, styles, &mut first, &mut output);
    }
    output.push(']');
    if !math.is_empty() {
        output.push_str(",\"math\":[");
        for (index, value) in math.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            encode_math_syntax_record(value, &mut output);
        }
        output.push(']');
    }
    if !precomposed_vectors.is_empty() {
        output.push_str(",\"precomposed_vectors\":[");
        for (index, value) in precomposed_vectors.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str("{\"fingerprint\":");
            push_hash(&mut output, value.fingerprint());
            output.push_str(",\"node_id\":");
            output.push_str(&value.node_id().get().to_string());
            output.push('}');
        }
        output.push(']');
    }
    output.push_str(",\"resources\":{");
    output.push_str("\"fonts\":[");
    for (index, font) in resources.font_faces.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"font_face_id\":");
        output.push_str(&font.font_face_id.get().to_string());
        output.push_str(",\"media_type\":");
        push_jcs_string(
            &mut output,
            match font.media {
                FontMediaDeclaration::Declared(value) => value.as_str(),
                FontMediaDeclaration::LegacyUnspecified => "legacy_unspecified",
            },
        );
        output.push('}');
    }
    output.push_str("],\"images\":[");
    for (index, image) in resources.images.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"image_id\":");
        output.push_str(&image.image_id.get().to_string());
        output.push_str(",\"media_type\":");
        push_jcs_string(
            &mut output,
            match image.media {
                ImageMediaDeclaration::Declared(value) => value.as_str(),
                ImageMediaDeclaration::LegacyUnspecified => "legacy_unspecified",
            },
        );
        output.push('}');
    }
    output.push_str("]}}");
    output
}

fn encode_math_syntax_record(value: &ValidatedStagingMathNode, output: &mut String) {
    let domain = value.domain();
    let style = value.computed_style();
    let block = style.block_style();
    output.push_str("{\"ast_fingerprint\":");
    push_hash(output, value.parsed().ast_fingerprint());
    output.push_str(",\"classes\":[");
    for (index, class) in domain.classes.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_jcs_string(output, class);
    }
    output.push_str("],\"kind\":");
    push_jcs_string(output, domain.kind.as_str());
    output.push_str(",\"language\":");
    push_jcs_string(output, &domain.language);
    output.push_str(",\"node_id\":");
    output.push_str(&domain.node_id.get().to_string());
    output.push_str(",\"owner_node_id\":");
    output.push_str(&domain.owner_node_id.get().to_string());
    output.push_str(",\"parsed_receipt\":");
    push_hash(output, value.parsed().fingerprint());
    output.push_str(",\"source\":");
    push_jcs_string(output, &domain.source);
    output.push_str(",\"source_span\":{\"end_byte\":");
    output.push_str(&domain.span.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&domain.span.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&domain.span.start_byte().get().to_string());
    output.push_str("},\"speech\":");
    push_jcs_string(output, &domain.speech);
    output.push_str(",\"style\":{\"end_indent\":");
    output.push_str(&block.end_indent().get().raw().to_string());
    output.push_str(",\"font_families\":[");
    for (index, family) in style.font_families().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_jcs_string(output, family);
    }
    output.push_str("],\"font_size\":");
    output.push_str(&style.font_size().get().raw().to_string());
    output.push_str(",\"keep_with_next\":");
    output.push_str(if block.keep_with_next() {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"line_height\":");
    output.push_str(&style.line_height().get().raw().to_string());
    output.push_str(",\"page\":");
    match style.page_name() {
        Some(value) => push_jcs_string(output, value.as_str()),
        None => output.push_str("null"),
    }
    output.push_str(",\"space_after\":");
    output.push_str(&block.space_after().get().raw().to_string());
    output.push_str(",\"space_before\":");
    output.push_str(&block.space_before().get().raw().to_string());
    output.push_str(",\"start_indent\":");
    output.push_str(&block.start_indent().get().raw().to_string());
    output.push_str(",\"text_align\":");
    push_jcs_string(output, block.text_align().as_str());
    output.push_str("},\"text_span\":{\"end_byte\":");
    output.push_str(&domain.text_span.end_byte().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&domain.text_span.start_byte().get().to_string());
    output.push_str(",\"text_id\":");
    output.push_str(&domain.text_span.text_id().get().to_string());
    output.push_str("},\"version\":");
    push_jcs_string(output, &domain.version);
    output.push('}');
}

fn encode_container_records(
    blocks: &[StagingM4Block],
    styles: &BTreeMap<NodeId, SemanticContainerComputedStyle>,
    first: &mut bool,
    output: &mut String,
) {
    for block in blocks {
        match block {
            StagingM4Block::SemanticContainer {
                common,
                semantic_kind,
                blocks,
            } => {
                if !*first {
                    output.push(',');
                }
                *first = false;
                let style = &styles[&common.node_id];
                output.push_str("{\"child_node_ids\":[");
                for (index, child) in blocks.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    output.push_str(&child.node_id().get().to_string());
                }
                output.push_str("],\"classes\":[");
                for (index, class) in common.classes.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    push_jcs_string(output, class);
                }
                output.push_str("],\"kind\":");
                push_jcs_string(output, semantic_kind.as_str());
                output.push_str(",\"node_id\":");
                output.push_str(&common.node_id.get().to_string());
                output.push_str(",\"source_span\":{");
                output.push_str("\"end_byte\":");
                output.push_str(&common.span.end_byte().get().to_string());
                output.push_str(",\"source_id\":");
                output.push_str(&common.span.source_id().get().to_string());
                output.push_str(",\"start_byte\":");
                output.push_str(&common.span.start_byte().get().to_string());
                output.push_str("},\"style\":{");
                let block_style = style.block_style();
                output.push_str("\"end_indent\":");
                output.push_str(&block_style.end_indent().get().raw().to_string());
                output.push_str(",\"font_families\":");
                match style.inheritance_style().font_families() {
                    Some(families) => {
                        output.push('[');
                        for (index, family) in families.iter().enumerate() {
                            if index > 0 {
                                output.push(',');
                            }
                            push_jcs_string(output, family);
                        }
                        output.push(']');
                    }
                    None => output.push_str("null"),
                }
                output.push_str(",\"font_size\":");
                match style.inheritance_style().font_size() {
                    Some(value) => output.push_str(&value.get().raw().to_string()),
                    None => output.push_str("null"),
                }
                output.push_str(",\"keep_with_next\":");
                output.push_str(if block_style.keep_with_next() {
                    "true"
                } else {
                    "false"
                });
                output.push_str(",\"line_height\":");
                match style.inheritance_style().line_height() {
                    Some(value) => output.push_str(&value.get().raw().to_string()),
                    None => output.push_str("null"),
                }
                output.push_str(",\"page\":");
                match style.page_name() {
                    Some(value) => push_jcs_string(output, value.as_str()),
                    None => output.push_str("null"),
                }
                output.push_str(",\"space_after\":");
                output.push_str(&block_style.space_after().get().raw().to_string());
                output.push_str(",\"space_before\":");
                output.push_str(&block_style.space_before().get().raw().to_string());
                output.push_str(",\"start_indent\":");
                output.push_str(&block_style.start_indent().get().raw().to_string());
                output.push_str(",\"text_align\":");
                push_jcs_string(output, block_style.text_align().as_str());
                output.push_str("}}");
                encode_container_records(blocks, styles, first, output);
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    encode_container_records(&item.blocks, styles, first, output);
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    encode_container_records(&cell.blocks, styles, first, output);
                }
            }
            StagingM4Block::Figure { caption, .. } => {
                encode_container_records(caption, styles, first, output)
            }
            StagingM4Block::VectorFigure { caption, .. } => {
                encode_container_records(caption, styles, first, output)
            }
            StagingM4Block::Paragraph { .. }
            | StagingM4Block::Heading { .. }
            | StagingM4Block::PageBreak { .. }
            | StagingM4Block::DisplayMath { .. }
            | StagingM4Block::MathVectorBlock { .. } => {}
        }
    }
}

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_document_package::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder, WireStagingM4Inline, WireStagingStyleDeclaration,
        WireStagingStyleValue,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));
    const VECTOR_FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/vector-media/job/document-package.json"));
    const MATH_FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/math/job/document-package.json"));
    const PRECOMPOSED_VECTOR_FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"));
    const JPEG_FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/jpeg-media/job/document-package.json"));
    const CFF_FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/cff-media/job/document-package.json"));

    fn parse(bytes: &[u8]) -> Result<ValidatedStagingSemanticPackage, Box<dyn std::error::Error>> {
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .expect("default limits are valid");
        parse_with_limits(bytes, &limits)
    }

    fn parse_with_limits(
        bytes: &[u8],
        limits: &ValidatedResourceLimits,
    ) -> Result<ValidatedStagingSemanticPackage, Box<dyn std::error::Error>> {
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(bytes, &DocumentPackageDecodePolicy::new(limits))?;
        Ok(StagingSemanticPackageParser::new().parse(decoded, limits)?)
    }

    fn precomposed_vector_syntax_error(bytes: &[u8]) -> StagingSemanticSyntaxError {
        let error = parse(bytes).expect_err("mutant must fail syntax validation");
        *error
            .downcast_ref::<StagingSemanticSyntaxError>()
            .expect("mutant must reach semantic syntax validation")
    }

    fn mutate_and_encode(update: impl FnOnce(&mut WireStagingM4DocumentPackage)) -> Vec<u8> {
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .expect("default limits are valid");
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let mut wire = decoded.into_wire();
        update(&mut wire);
        StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap()
            .into_bytes()
    }

    fn mutate_jpeg_and_encode(update: impl FnOnce(&mut WireStagingM4DocumentPackage)) -> Vec<u8> {
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .expect("default limits are valid");
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(JPEG_FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let mut wire = decoded.into_wire();
        update(&mut wire);
        StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap()
            .into_bytes()
    }

    fn mutate_precomposed_and_encode(
        update: impl FnOnce(&mut typaxis_document_package::WireStagingM4Document),
    ) -> Vec<u8> {
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .expect("default limits are valid");
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                PRECOMPOSED_VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut document = wire.document().clone();
        update(&mut document);
        wire.replace_typed_regions(document, wire.resources().clone());
        StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap()
            .into_bytes()
    }

    fn precomposed_children_mut(
        document: &mut typaxis_document_package::WireStagingM4Document,
    ) -> &mut Vec<WireStagingM4Block> {
        let WireStagingM4Block::SemanticContainer { blocks, .. } = &mut document.blocks[0] else {
            panic!("fixture root must remain a semantic container");
        };
        blocks
    }

    fn inline_math_vector_mut(
        document: &mut typaxis_document_package::WireStagingM4Document,
    ) -> &mut WireStagingM4Inline {
        let blocks = precomposed_children_mut(document);
        let WireStagingM4Block::Paragraph { children, .. } = &mut blocks[0] else {
            panic!("fixture first child must remain a paragraph");
        };
        &mut children[1]
    }

    fn math_vector_block_mut(
        document: &mut typaxis_document_package::WireStagingM4Document,
    ) -> &mut WireStagingM4Block {
        &mut precomposed_children_mut(document)[2]
    }

    fn text_limits(maximum_buffer: usize, maximum_total: u64) -> ValidatedResourceLimits {
        let mut raw = typaxis_core::ResourceLimits::default();
        raw.max_text_buffer_bytes = u32::try_from(maximum_buffer).unwrap();
        raw.max_shaping_context_bytes = raw.max_text_buffer_bytes;
        raw.max_text_bytes = maximum_total;
        ValidatedResourceLimits::new(raw).unwrap()
    }

    #[test]
    fn semantic_container_validates_ownership_style_and_typed_round_trip() {
        let package = parse(FIXTURE).unwrap();
        assert_eq!(package.semantic_container_count(), 3);
        assert_eq!(
            package
                .computed_style(NodeId::new(1))
                .unwrap()
                .semantic_kind(),
            SemanticContainerStyleKind::Result
        );
        assert_eq!(
            package
                .computed_style(NodeId::new(1))
                .unwrap()
                .block_style()
                .space_before()
                .get()
                .raw(),
            7
        );
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(package.checked_wire().unwrap())
            .unwrap();
        let reparsed = parse(encoded.as_bytes()).unwrap();
        assert_eq!(
            package.semantic_fingerprint(),
            reparsed.semantic_fingerprint()
        );
    }

    #[test]
    fn jpeg_media_lowering_and_private_profile_are_closed() {
        let package = parse(JPEG_FIXTURE).unwrap();
        assert_eq!(package.resources().images.len(), 3);
        assert!(package.resources().images.iter().all(|image| {
            image.media == ImageMediaDeclaration::Declared(ImageMediaType::JpegBaseline)
                && image.vector_provenance.is_none()
        }));
        let base = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        assert!(matches!(
            StagingSemanticContainerProfileView::new(&package, &base),
            Err(StagingSemanticSyntaxError::JpegStaging(_))
        ));
        let limits = M4EffectiveResourceLimits::defaults_for(&base);
        let view = StagingJpegProfileView::new(&package, &limits).unwrap();
        assert_eq!(
            view.jpeg_resource_ids(),
            [
                ImageResourceId::new(0),
                ImageResourceId::new(1),
                ImageResourceId::new(2)
            ]
        );
        assert_eq!(view.figures().len(), 3);
        assert_eq!(view.figures()[0].image_id(), ImageResourceId::new(0));
        assert_eq!(view.figures()[1].image_id(), ImageResourceId::new(0));
        assert_eq!(view.figures()[2].image_id(), ImageResourceId::new(1));
        assert!(!view.figures()[0].page_break_before());
        assert!(!view.figures()[1].page_break_before());
        assert!(view.figures()[2].page_break_before());
        view.authorizes(&package, &limits).unwrap();
    }

    #[test]
    fn font_media_cff_lowering_is_private_and_profile_bound() {
        let package = parse(CFF_FIXTURE).unwrap();
        assert_eq!(package.resources().font_faces.len(), 1);
        assert_eq!(
            package.resources().font_faces[0].media,
            FontMediaDeclaration::Declared(FontMediaType::SfntCff1)
        );
        let base = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        assert_eq!(
            StagingSemanticContainerProfileView::new(&package, &base).unwrap_err(),
            StagingSemanticSyntaxError::CffStaging(FontFaceId::new(0))
        );
        let limits = M4EffectiveResourceLimits::defaults_for(&base);
        let view = StagingCffProfileView::new(&package, &limits).unwrap();
        assert_eq!(view.font_face_ids(), [FontFaceId::new(0)]);
        assert_eq!(view.limits_fingerprint(), limits.fingerprint());
        view.authorizes(&package, &limits).unwrap();

        let truetype = String::from_utf8(CFF_FIXTURE.to_vec()).unwrap().replacen(
            "sfnt-cff1",
            "sfnt-truetype-glyf",
            1,
        );
        let package = parse(truetype.as_bytes()).unwrap();
        assert!(StagingCffProfileView::new(&package, &limits).is_err());

        let nonzero_face = String::from_utf8(CFF_FIXTURE.to_vec()).unwrap().replacen(
            "\"face_index\":0",
            "\"face_index\":1",
            1,
        );
        assert!(parse(nonzero_face.as_bytes()).is_err());
    }

    #[test]
    fn jpeg_private_profile_rejects_unhandled_styles_and_classes() {
        let base = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        let limits = M4EffectiveResourceLimits::defaults_for(&base);

        let figure_rule = mutate_jpeg_and_encode(|wire| {
            let mut sheet = wire.style_sheet().clone();
            sheet.rules[0].selector = "figure".to_owned();
            wire.replace_style_sheet(sheet);
        });
        let package = parse(&figure_rule).unwrap();
        assert!(matches!(
            StagingJpegProfileView::new(&package, &limits),
            Err(StagingSemanticSyntaxError::InapplicableStyle)
        ));

        let nonneutral_container = mutate_jpeg_and_encode(|wire| {
            let mut sheet = wire.style_sheet().clone();
            sheet.rules[0].declarations[0].value = WireStagingStyleValue::Length { value: 1 };
            wire.replace_style_sheet(sheet);
        });
        let package = parse(&nonneutral_container).unwrap();
        assert!(matches!(
            StagingJpegProfileView::new(&package, &limits),
            Err(StagingSemanticSyntaxError::InapplicableStyle)
        ));

        let figure_class = mutate_jpeg_and_encode(|wire| {
            let mut document = wire.document().clone();
            let WireStagingM4Block::SemanticContainer { blocks, .. } = &mut document.blocks[0]
            else {
                panic!("fixture root must remain a semantic container");
            };
            let WireStagingM4Block::Figure { classes, .. } = &mut blocks[0] else {
                panic!("fixture first child must remain a figure");
            };
            classes.push("styled".to_owned());
            wire.replace_typed_regions(document, wire.resources().clone());
        });
        let package = parse(&figure_class).unwrap();
        assert!(matches!(
            StagingJpegProfileView::new(&package, &limits),
            Err(StagingSemanticSyntaxError::InvalidBlock(_))
        ));
    }

    #[test]
    fn jpeg_private_profile_rejects_non_jpeg_resources() {
        let base = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        let limits = M4EffectiveResourceLimits::defaults_for(&base);

        let unused_font = mutate_jpeg_and_encode(|wire| {
            let document = wire.document().clone();
            let mut resources = wire.resources().clone();
            resources
                .font_faces
                .push(typaxis_document_package::WireStagingM4FontFace {
                    font_face_id: 0,
                    family: "Unused".to_owned(),
                    uri: "unused.ttf".to_owned(),
                    face_index: 0,
                    expected_sha256: None,
                    media_type: WireFontMediaType::SfntTrueTypeGlyf,
                });
            wire.replace_typed_regions(document, resources);
        });
        let package = parse(&unused_font).unwrap();
        assert_eq!(
            StagingJpegProfileView::new(&package, &limits),
            Err(StagingSemanticSyntaxError::InvalidResource)
        );

        let png_declaration = mutate_jpeg_and_encode(|wire| {
            let document = wire.document().clone();
            let mut resources = wire.resources().clone();
            resources.images[0].media_type = WireImageMediaType::Png;
            wire.replace_typed_regions(document, resources);
        });
        let package = parse(&png_declaration).unwrap();
        assert_eq!(
            StagingJpegProfileView::new(&package, &limits),
            Err(StagingSemanticSyntaxError::InvalidResource)
        );
    }

    #[test]
    fn jpeg_private_profile_rejects_unhandled_page_master_extensions() {
        let package = parse(JPEG_FIXTURE).unwrap();
        let wire = package.checked_wire().unwrap();
        let base = wire.page_masters();
        let advanced = wire.advanced_page_masters();

        let mut with_header = base.clone();
        with_header.masters[0].header = Some(typaxis_document_package::WireRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        });
        assert_eq!(
            validate_jpeg_profile_page_master_extensions(&with_header, advanced),
            Err(StagingSemanticSyntaxError::InvalidPageGeometry)
        );

        let mut with_trim_override = advanced.clone();
        with_trim_override.masters[0].trim.width -= 1;
        assert_eq!(
            validate_jpeg_profile_page_master_extensions(base, &with_trim_override),
            Err(StagingSemanticSyntaxError::InvalidPageGeometry)
        );

        let mut with_columns = advanced.clone();
        with_columns.masters[0].column_layout = Some(typaxis_document_package::WireColumnLayout {
            count: 2,
            gap: 0,
            fill: typaxis_document_package::WireColumnFill::Sequential,
            balance: typaxis_document_package::WireColumnBalance::None,
        });
        assert_eq!(
            validate_jpeg_profile_page_master_extensions(base, &with_columns),
            Err(StagingSemanticSyntaxError::InvalidPageGeometry)
        );
    }

    #[test]
    fn precomposed_vector_staging_dispatch_retains_domain_and_rejects_legacy_profile() {
        let package = parse(PRECOMPOSED_VECTOR_FIXTURE).unwrap();
        assert_eq!(package.semantic_container_count(), 1);
        assert!(package.math_nodes().is_empty());
        assert_eq!(
            package.resources().images[0].media,
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2)
        );
        assert_eq!(
            package.resources().images[0]
                .vector_provenance
                .as_ref()
                .map(|value| value.engine_id.as_str()),
            Some("vmb.texToSvg")
        );

        let StagingM4Block::SemanticContainer { blocks, .. } = &package.document().blocks[0] else {
            panic!("fixture root must remain a semantic container");
        };
        let StagingM4Block::Paragraph {
            has_authored_content,
            inline_vectors,
            ..
        } = &blocks[0]
        else {
            panic!("first child must remain a paragraph");
        };
        assert!(*has_authored_content);
        assert_eq!(inline_vectors.len(), 2);
        assert_eq!(
            inline_vectors[0].kind,
            StagingM4InlineVectorKind::InlineVector
        );
        assert_eq!(
            inline_vectors[1].kind,
            StagingM4InlineVectorKind::MathVector
        );
        assert!(matches!(blocks[1], StagingM4Block::VectorFigure { .. }));
        let StagingM4Block::MathVectorBlock {
            equation_number: Some(number),
            ..
        } = &blocks[2]
        else {
            panic!("numbered math-vector block must remain lossless");
        };
        assert_eq!(number.node_id, NodeId::new(7));
        let vector_languages = package.precomposed_vector_effective_languages().unwrap();
        assert_eq!(
            vector_languages
                .iter()
                .map(|receipt| (receipt.owner().get(), receipt.language()))
                .collect::<Vec<_>>(),
            [(3, "ja"), (4, "ja"), (5, "ja"), (6, "ja")]
        );
        assert!(vector_languages
            .iter()
            .all(|receipt| receipt.owner() != number.node_id));

        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        assert_eq!(
            StagingSemanticContainerProfileView::new(&package, &limits),
            Err(StagingSemanticSyntaxError::SvgSafe2Staging(
                ImageResourceId::new(0)
            ))
        );
        let navigation_error = crate::validate_staging_book_navigation(&package, &limits)
            .expect_err("computed-language /1 must not absorb new language owners");
        assert_eq!(
            navigation_error.kind(),
            crate::BookNavigationSyntaxErrorKind::PrecomposedVectorStaging
        );
    }

    #[test]
    fn precomposed_vector_staging_dispatch_null_equation_consumes_no_node_id() {
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                PRECOMPOSED_VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut document = wire.document().clone();
        let WireStagingM4Block::SemanticContainer { blocks, .. } = &mut document.blocks[0] else {
            unreachable!();
        };
        let WireStagingM4Block::MathVectorBlock {
            equation_number, ..
        } = &mut blocks[2]
        else {
            unreachable!();
        };
        *equation_number = None;
        wire.replace_typed_regions(document, wire.resources().clone());
        let canonical = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        assert!(canonical.contains("\"equation_number\":null"));
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                canonical.as_bytes(),
                &DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        assert!(StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .is_ok());
    }

    #[test]
    fn precomposed_vector_metrics_seals_relations_package_session_and_raw_payload() {
        let package = parse(PRECOMPOSED_VECTOR_FIXTURE).unwrap();
        assert_eq!(package.precomposed_vector_metrics().len(), 4);
        assert_eq!(
            package
                .precomposed_vector_metrics()
                .iter()
                .map(ValidatedPrecomposedVectorMetrics::node_id)
                .collect::<Vec<_>>(),
            [
                NodeId::new(3),
                NodeId::new(4),
                NodeId::new(5),
                NodeId::new(6)
            ]
        );
        let math = package
            .precomposed_vector_metrics_for(NodeId::new(4))
            .unwrap();
        assert_eq!(math.algorithm(), PRECOMPOSED_VECTOR_METRICS_ALGORITHM);
        assert_eq!(math.contract(), "typaxis.contract/1.4");
        assert_eq!(math.package_sha256(), package.canonical_jcs_sha256());
        assert_eq!(
            math.limits_fingerprint(),
            precomposed_vector_limits_fingerprint(package.limits())
        );
        assert_eq!(math.kind(), PrecomposedVectorKind::MathVector);
        assert_eq!(math.resource_binding().image_id(), ImageResourceId::new(0));
        let PrecomposedVectorMetricPayload::Inline { metrics, spacing } = math.payload() else {
            panic!("math inline receipt must retain inline metrics");
        };
        assert_eq!(metrics.advance.get().raw(), 2_031_616);
        assert_eq!(metrics.viewport.width.get().raw(), 1_966_080);
        assert_eq!(spacing.before.get().raw(), 16_384);
        assert!(math.canonical_jcs().starts_with(
            "{\"algorithm\":\"typaxis.precomposed-vector-metrics/1\",\"alternative\":"
        ));
        assert_ne!(math.fingerprint(), [0; 32]);
        package.verify_precomposed_vector_metrics(math).unwrap();

        let same_input_new_session = parse(PRECOMPOSED_VECTOR_FIXTURE).unwrap();
        assert_eq!(
            math.fingerprint(),
            same_input_new_session
                .precomposed_vector_metrics_for(NodeId::new(4))
                .unwrap()
                .fingerprint()
        );
        assert_eq!(
            same_input_new_session.verify_precomposed_vector_metrics(math),
            Err(StagingSemanticSyntaxError::ReceiptMismatch)
        );

        let mut tampered = parse(PRECOMPOSED_VECTOR_FIXTURE).unwrap();
        tampered.precomposed_vector_metrics[0].fingerprint = [0; 32];
        let otherwise_intact = &tampered.precomposed_vector_metrics[1];
        assert_eq!(
            tampered.verify_precomposed_vector_metrics(otherwise_intact),
            Err(StagingSemanticSyntaxError::ReceiptMismatch)
        );
        assert_eq!(
            tampered.checked_wire(),
            Err(StagingSemanticSyntaxError::ReceiptMismatch)
        );

        let zero_advance = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replacen("\"advance\":2031616", "\"advance\":0", 1);
        assert!(parse(zero_advance.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("/document/blocks/0/blocks/0/children/1/metrics/advance"));

        let negative_descent = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replacen("\"descent\":196608", "\"descent\":-1", 1);
        assert!(parse(negative_descent.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("/document/blocks/0/blocks/0/children/0/metrics/descent"));

        let zero_width = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replacen("\"width\":1966080", "\"width\":0", 1);
        assert!(parse(zero_width.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("/document/blocks/0/blocks/0/children/0/metrics/viewport/width"));

        let baseline_outside = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Inline::MathVector { metrics, .. } = inline_math_vector_mut(document)
            else {
                unreachable!();
            };
            metrics.baseline = metrics.viewport.height + 1;
        });
        assert_eq!(
            precomposed_vector_syntax_error(&baseline_outside),
            invalid_precomposed_vector(NodeId::new(4), PrecomposedVectorField::MetricsBaseline)
        );

        let short_ascent = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Inline::MathVector { metrics, .. } = inline_math_vector_mut(document)
            else {
                unreachable!();
            };
            metrics.ascent = metrics.baseline - 1;
        });
        assert_eq!(
            precomposed_vector_syntax_error(&short_ascent),
            invalid_precomposed_vector(NodeId::new(4), PrecomposedVectorField::MetricsAscent)
        );

        let short_descent = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Inline::MathVector { metrics, .. } = inline_math_vector_mut(document)
            else {
                unreachable!();
            };
            metrics.descent = metrics.viewport.height - metrics.baseline - 1;
        });
        assert_eq!(
            precomposed_vector_syntax_error(&short_descent),
            invalid_precomposed_vector(NodeId::new(4), PrecomposedVectorField::MetricsDescent)
        );

        let overflowing_right_edge = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Inline::MathVector { metrics, .. } = inline_math_vector_mut(document)
            else {
                unreachable!();
            };
            metrics.origin_x = typaxis_core::JSON_SAFE_INTEGER_MAX;
        });
        assert_eq!(
            precomposed_vector_syntax_error(&overflowing_right_edge),
            invalid_precomposed_vector(NodeId::new(4), PrecomposedVectorField::MetricsOriginX)
        );

        let missing_advance = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replacen("\"advance\":2031616,", "", 1);
        let missing_error = parse(missing_advance.as_bytes()).unwrap_err();
        assert!(missing_error
            .to_string()
            .contains("/document/blocks/0/blocks/0/children/1/metrics/advance"));
    }

    #[test]
    fn precomposed_vector_alternative_validates_exact_source_resolution_language_and_number() {
        let package = parse(PRECOMPOSED_VECTOR_FIXTURE).unwrap();
        let figure = package
            .precomposed_vector_metrics_for(NodeId::new(3))
            .unwrap();
        assert_eq!(
            figure.alternative().resolution(),
            PrecomposedVectorActualTextResolution::Absent
        );
        assert_eq!(figure.alternative().resolved_actual_text(), None);

        let math = package
            .precomposed_vector_metrics_for(NodeId::new(4))
            .unwrap();
        assert_eq!(
            math.source_tex().unwrap().mapped_source_span(),
            SourceSpan::new(
                SourceId::new(0),
                Utf8ByteOffset::new(3),
                Utf8ByteOffset::new(6)
            )
            .unwrap()
        );
        assert_eq!(
            math.source_tex().unwrap().exact_text_sha256(),
            sha256(b"x+y")
        );
        assert_eq!(
            math.alternative().resolution(),
            PrecomposedVectorActualTextResolution::AlternativeFallback
        );
        assert_eq!(math.alternative().resolved_actual_text(), Some("xたすy"));
        assert_eq!(math.alternative().authored_actual_text(), None);

        let block = package
            .precomposed_vector_metrics_for(NodeId::new(6))
            .unwrap();
        let number = block.equation_number().unwrap();
        assert_eq!(number.node_id(), NodeId::new(7));
        assert_eq!(number.text().exact_text_sha256(), sha256(b"(1)"));
        assert_eq!(number.minimum_gap().get().raw(), 65_536);

        let authored = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Inline::MathVector {
                actual_text,
                language,
                ..
            } = inline_math_vector_mut(document)
            else {
                unreachable!();
            };
            *actual_text = Some("x plus y".to_owned());
            *language = Some("JA-latn".to_owned());
        });
        let authored = parse(&authored).unwrap();
        let receipt = authored
            .precomposed_vector_metrics_for(NodeId::new(4))
            .unwrap();
        assert_eq!(
            receipt.alternative().resolution(),
            PrecomposedVectorActualTextResolution::Authored
        );
        assert_eq!(
            receipt.alternative().resolved_actual_text(),
            Some("x plus y")
        );
        let language = receipt.language().unwrap();
        assert_eq!(language.raw(), "JA-latn");
        assert_eq!(language.canonical(), "ja-Latn");
        let effective = authored
            .precomposed_vector_effective_language(NodeId::new(4))
            .unwrap();
        assert_eq!(effective.language(), "ja-Latn");
        assert_eq!(
            effective.algorithm(),
            PRECOMPOSED_VECTOR_EFFECTIVE_LANGUAGE_ALGORITHM
        );
        assert_eq!(
            language.charged_bytes(),
            u64::try_from("JA-latn".len() + "ja-Latn".len()).unwrap()
        );

        let whitespace_alt = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Inline::MathVector { alt, .. } = inline_math_vector_mut(document)
            else {
                unreachable!();
            };
            *alt = "\u{2007}".to_owned();
        });
        assert_eq!(
            precomposed_vector_syntax_error(&whitespace_alt),
            invalid_precomposed_vector(NodeId::new(4), PrecomposedVectorField::Alternative)
        );

        let control_actual = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Inline::MathVector { actual_text, .. } =
                inline_math_vector_mut(document)
            else {
                unreachable!();
            };
            *actual_text = Some("read\nme".to_owned());
        });
        assert_eq!(
            precomposed_vector_syntax_error(&control_actual),
            invalid_precomposed_vector(NodeId::new(4), PrecomposedVectorField::ActualText)
        );

        let invalid_language = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Inline::MathVector { language, .. } = inline_math_vector_mut(document)
            else {
                unreachable!();
            };
            *language = Some("ja_JP".to_owned());
        });
        assert_eq!(
            precomposed_vector_syntax_error(&invalid_language),
            invalid_precomposed_vector(NodeId::new(4), PrecomposedVectorField::Language)
        );

        let empty_source = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Inline::MathVector { source_tex, .. } =
                inline_math_vector_mut(document)
            else {
                unreachable!();
            };
            source_tex.text_span.end_byte = source_tex.text_span.start_byte;
        });
        assert_eq!(
            precomposed_vector_syntax_error(&empty_source),
            invalid_precomposed_vector(NodeId::new(4), PrecomposedVectorField::SourceTexTextSpan)
        );

        let nul_source = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replace(
                "\"utf8\":\"(a)x+yMx+y(1)\"",
                "\"utf8\":\"(a)\\u0000+yMx+y(1)\"",
            );
        assert_eq!(
            precomposed_vector_syntax_error(nul_source.as_bytes()),
            invalid_precomposed_vector(NodeId::new(4), PrecomposedVectorField::SourceTexTextSpan)
        );

        let bom_source = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replace(
                "\"utf8\":\"(a)x+yMx+y(1)\"",
                "\"utf8\":\"(a)\u{feff}Mx+y(1)\"",
            );
        assert_eq!(
            precomposed_vector_syntax_error(bom_source.as_bytes()),
            invalid_precomposed_vector(NodeId::new(4), PrecomposedVectorField::SourceTexTextSpan)
        );

        let non_identity = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replace(
                "{\"kind\":\"identity\",\"source_span\":{\"end_byte\":6,\"source_id\":0,\"start_byte\":3},\"text_range\":{\"end_byte\":6,\"start_byte\":3}}",
                "{\"kind\":\"replacement\",\"source_span\":{\"end_byte\":6,\"source_id\":0,\"start_byte\":3},\"text_range\":{\"end_byte\":6,\"start_byte\":3}}",
            );
        assert_eq!(
            precomposed_vector_syntax_error(non_identity.as_bytes()),
            StagingSemanticSyntaxError::InvalidSourceSpan
        );

        let reversed_source_span = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replace(
                "{\"kind\":\"identity\",\"source_span\":{\"end_byte\":6,\"source_id\":0,\"start_byte\":3},\"text_range\":{\"end_byte\":6,\"start_byte\":3}}",
                "{\"kind\":\"identity\",\"source_span\":{\"end_byte\":3,\"source_id\":0,\"start_byte\":6},\"text_range\":{\"end_byte\":6,\"start_byte\":3}}",
            );
        assert_eq!(
            precomposed_vector_syntax_error(reversed_source_span.as_bytes()),
            StagingSemanticSyntaxError::InvalidSourceSpan
        );

        let zero_gap = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replace("\"minimum_gap\":65536", "\"minimum_gap\":0");
        assert!(parse(zero_gap.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("/document/blocks/0/blocks/2/equation_number/minimum_gap"));

        let nondense_number = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Block::MathVectorBlock {
                equation_number: Some(number),
                ..
            } = math_vector_block_mut(document)
            else {
                unreachable!();
            };
            number.node_id = 8;
        });
        assert_eq!(
            precomposed_vector_syntax_error(&nondense_number),
            invalid_precomposed_vector(
                NodeId::new(6),
                PrecomposedVectorField::EquationNumberNodeId
            )
        );

        let number_before_formula = mutate_precomposed_and_encode(|document| {
            let WireStagingM4Block::MathVectorBlock {
                equation_number: Some(number),
                ..
            } = math_vector_block_mut(document)
            else {
                unreachable!();
            };
            number.span.start_byte = 9;
        });
        assert_eq!(
            precomposed_vector_syntax_error(&number_before_formula),
            invalid_precomposed_vector(NodeId::new(6), PrecomposedVectorField::EquationNumberSpan)
        );

        let whitespace_number = String::from_utf8(PRECOMPOSED_VECTOR_FIXTURE.to_vec())
            .unwrap()
            .replace("\"utf8\":\"(a)x+yMx+y(1)\"", "\"utf8\":\"(a)x+yMx+y   \"");
        assert_eq!(
            precomposed_vector_syntax_error(whitespace_number.as_bytes()),
            invalid_precomposed_vector(
                NodeId::new(6),
                PrecomposedVectorField::EquationNumberTextSpan
            )
        );
    }

    #[test]
    fn precomposed_vector_limits_charge_once_at_exact_max_and_report_max_plus_one() {
        let package = parse(PRECOMPOSED_VECTOR_FIXTURE).unwrap();
        let text_buffer_bytes = package
            .checked_wire()
            .unwrap()
            .text_buffers()
            .iter()
            .map(|buffer| u64::try_from(buffer.utf8.len()).unwrap())
            .sum::<u64>();
        let authored_bytes = package
            .precomposed_vector_metrics()
            .iter()
            .map(|receipt| {
                u64::try_from(receipt.alternative().alternative().len()).unwrap()
                    + receipt
                        .alternative()
                        .authored_actual_text()
                        .map_or(0, |value| u64::try_from(value.len()).unwrap())
                    + receipt
                        .language()
                        .map_or(0, ValidatedPrecomposedVectorLanguageOverride::charged_bytes)
            })
            .sum::<u64>();
        let exact_total = text_buffer_bytes + authored_bytes;
        let largest_authored = package
            .precomposed_vector_metrics()
            .iter()
            .map(|receipt| receipt.alternative().alternative().len())
            .chain(
                package
                    .checked_wire()
                    .unwrap()
                    .text_buffers()
                    .iter()
                    .map(|buffer| buffer.utf8.len()),
            )
            .max()
            .unwrap();

        let exact_limits = text_limits(largest_authored, exact_total);
        assert!(parse_with_limits(PRECOMPOSED_VECTOR_FIXTURE, &exact_limits).is_ok());
        let max_plus_one_limits = text_limits(largest_authored, exact_total - 1);
        let error = parse_with_limits(PRECOMPOSED_VECTOR_FIXTURE, &max_plus_one_limits)
            .expect_err("one byte above aggregate maximum must fail");
        assert!(matches!(
            error.downcast_ref::<StagingSemanticSyntaxError>(),
            Some(StagingSemanticSyntaxError::PrecomposedVectorTextAggregateLimit { .. })
        ));

        let per_buffer_limits = text_limits(largest_authored - 1, exact_total);
        let error = parse_with_limits(PRECOMPOSED_VECTOR_FIXTURE, &per_buffer_limits)
            .expect_err("one byte above per-string maximum must fail");
        assert!(matches!(
            error.downcast_ref::<StagingSemanticSyntaxError>(),
            Some(
                StagingSemanticSyntaxError::PrecomposedVectorTextBufferLimit {
                    field: PrecomposedVectorField::Alternative,
                    ..
                }
            )
        ));

        let session = PrecomposedVectorSyntaxSessionIdentity::fresh();
        let sources = BTreeMap::new();
        let text_buffers = BTreeMap::new();
        let raw_limits = typaxis_core::ResourceLimits {
            max_ast_nodes: 1,
            max_ast_nesting_depth: 2,
            ..typaxis_core::ResourceLimits::default()
        };
        let node_limits = ValidatedResourceLimits::new(raw_limits).unwrap();
        let mut validator = SemanticValidator {
            sources: &sources,
            text_buffers: &text_buffers,
            precomposed_vector_text_buffer_sha256: BTreeMap::new(),
            precomposed_vector_text_slice_sha256: BTreeMap::new(),
            next_node_id: 0,
            node_count: 0,
            admitted_text_and_math_speech_bytes: 0,
            math_nodes: Vec::new(),
            precomposed_vector_session: &session,
            precomposed_vector_metrics: Vec::new(),
            canonical_package_sha256: [1; 32],
            precomposed_vector_limits_fingerprint: precomposed_vector_limits_fingerprint(
                &node_limits,
            ),
            limits: &node_limits,
        };
        assert!(validator.precomposed_vector_node(0, None, 2).is_ok());
        assert_eq!(
            validator.precomposed_vector_node(1, None, 2),
            Err(StagingSemanticSyntaxError::PrecomposedVectorAstNodeLimit)
        );

        let raw_limits = typaxis_core::ResourceLimits {
            max_ast_nodes: 2,
            max_ast_nesting_depth: 2,
            ..typaxis_core::ResourceLimits::default()
        };
        let depth_limits = ValidatedResourceLimits::new(raw_limits).unwrap();
        let mut validator = SemanticValidator {
            sources: &sources,
            text_buffers: &text_buffers,
            precomposed_vector_text_buffer_sha256: BTreeMap::new(),
            precomposed_vector_text_slice_sha256: BTreeMap::new(),
            next_node_id: 0,
            node_count: 0,
            admitted_text_and_math_speech_bytes: 0,
            math_nodes: Vec::new(),
            precomposed_vector_session: &session,
            precomposed_vector_metrics: Vec::new(),
            canonical_package_sha256: [1; 32],
            precomposed_vector_limits_fingerprint: precomposed_vector_limits_fingerprint(
                &depth_limits,
            ),
            limits: &depth_limits,
        };
        assert_eq!(
            validator.precomposed_vector_node(0, None, 3),
            Err(StagingSemanticSyntaxError::PrecomposedVectorAstDepthLimit)
        );

        assert!(is_valid_precomposed_vector_provenance(&"x".repeat(128)));
        assert!(!is_valid_precomposed_vector_provenance(""));
        assert!(!is_valid_precomposed_vector_provenance(&"x".repeat(129)));
        assert!(!is_valid_precomposed_vector_provenance("engine\nversion"));

        let mut hash_cache = BTreeMap::new();
        let mut calculations = 0;
        let first = cached_precomposed_vector_sha256(&mut hash_cache, 7u32, || {
            calculations += 1;
            sha256(b"repeated source")
        });
        let second = cached_precomposed_vector_sha256(&mut hash_cache, 7u32, || {
            calculations += 1;
            [0; 32]
        });
        assert_eq!(first, second);
        assert_eq!(calculations, 1);
        assert_eq!(hash_cache.len(), 1);
    }

    #[test]
    fn vector_media_lowering_retains_resource_figure_and_round_trip_identity() {
        let package = parse(VECTOR_FIXTURE).unwrap();
        assert_eq!(package.resources().images.len(), 2);
        assert_eq!(
            package.resources().images[0].media,
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1)
        );
        let StagingM4Block::SemanticContainer { blocks, .. } = &package.document().blocks[0] else {
            panic!("fixture root must be a semantic container")
        };
        let StagingM4Block::Figure {
            common,
            image_id,
            placement,
            alternative,
            ..
        } = &blocks[0]
        else {
            panic!("fixture child must be a figure")
        };
        assert_eq!(common.node_id, NodeId::new(2));
        assert_eq!(*image_id, ImageResourceId::new(0));
        assert_eq!(*placement, StagingM4FigurePlacement::Block);
        assert_eq!(alternative, "Blue vector geometry");

        let limits = M4EffectiveResourceLimits::defaults_for(package.limits());
        let profile = StagingSafeVectorProfileView::new(&package, &limits).unwrap();
        assert_eq!(
            profile.vector_resource_ids(),
            [ImageResourceId::new(0), ImageResourceId::new(1)]
        );
        assert_eq!(profile.figure_owners(), [NodeId::new(2)]);
        assert_eq!(profile.page_geometry().body().x().raw(), 100 * 65_536);
        assert_eq!(
            profile.page_geometry().body().width().get().raw(),
            800 * 65_536
        );

        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(package.checked_wire().unwrap())
            .unwrap();
        let reparsed = parse(encoded.as_bytes()).unwrap();
        assert_eq!(
            package.semantic_fingerprint(),
            reparsed.semantic_fingerprint()
        );
    }

    #[test]
    fn math_lowering_binds_exact_source_mapping_parser_speech_and_style() {
        let package = parse(MATH_FIXTURE).unwrap();
        assert_eq!(package.math_nodes().len(), 2);
        let inline = &package.math_nodes()[0];
        assert_eq!(inline.domain().kind, StagingM4MathKind::Inline);
        assert_eq!(inline.domain().source, "x^{2}");
        assert_eq!(inline.domain().speech, "x squared");
        assert_eq!(inline.domain().span.start_byte().get(), 0);
        assert_eq!(inline.domain().span.end_byte().get(), 5);
        assert_eq!(inline.domain().text_span.start_byte().get(), 0);
        assert_eq!(inline.domain().text_span.end_byte().get(), 5);
        assert_eq!(inline.parsed().canonical_source(), "x^{2}");
        inline.parsed().verify().unwrap();
        assert_eq!(inline.computed_style().font_families(), ["Math"]);
        assert_eq!(inline.computed_style().font_size().get().raw(), 12 * 65_536);

        let display = &package.math_nodes()[1];
        assert_eq!(display.domain().kind, StagingM4MathKind::Display);
        assert_eq!(display.domain().source, "x+1");
        assert_eq!(display.domain().speech, "x plus one");
        assert_eq!(display.domain().text_span.start_byte().get(), 5);
        assert_eq!(display.domain().text_span.end_byte().get(), 8);
        assert_eq!(
            display.computed_style().block_style().text_align(),
            typaxis_style::MachineTextAlign::Center
        );

        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(package.checked_wire().unwrap())
            .unwrap();
        let reparsed = parse(encoded.as_bytes()).unwrap();
        assert_eq!(
            package.semantic_fingerprint(),
            reparsed.semantic_fingerprint()
        );

        let wrong_mapping = String::from_utf8(MATH_FIXTURE.to_vec()).unwrap().replacen(
            "\"text_span\":{\"end_byte\":5,\"start_byte\":0,\"text_id\":0}",
            "\"text_span\":{\"end_byte\":4,\"start_byte\":0,\"text_id\":0}",
            1,
        );
        assert!(parse(wrong_mapping.as_bytes()).is_err());
        let overlapping_mapping = String::from_utf8(MATH_FIXTURE.to_vec()).unwrap().replacen(
            "{\"kind\":\"identity\",\"source_span\":{\"end_byte\":5,\"source_id\":0,\"start_byte\":0},\"text_range\":{\"end_byte\":5,\"start_byte\":0}}",
            "{\"kind\":\"identity\",\"source_span\":{\"end_byte\":5,\"source_id\":0,\"start_byte\":0},\"text_range\":{\"end_byte\":5,\"start_byte\":0}},{\"kind\":\"identity\",\"source_span\":{\"end_byte\":6,\"source_id\":0,\"start_byte\":1},\"text_range\":{\"end_byte\":6,\"start_byte\":1}}",
            1,
        );
        assert!(parse(overlapping_mapping.as_bytes()).is_err());
        let control_speech = String::from_utf8(MATH_FIXTURE.to_vec()).unwrap().replacen(
            "\"speech\":\"x squared\"",
            "\"speech\":\"x\\nsquared\"",
            1,
        );
        assert!(parse(control_speech.as_bytes()).is_err());
        let unicode_whitespace = String::from_utf8(MATH_FIXTURE.to_vec()).unwrap().replacen(
            "\"speech\":\"x squared\"",
            "\"speech\":\"\u{2007}\"",
            1,
        );
        assert!(parse(unicode_whitespace.as_bytes()).is_err());
        let non_whitespace_format = String::from_utf8(MATH_FIXTURE.to_vec()).unwrap().replacen(
            "\"speech\":\"x squared\"",
            "\"speech\":\"\u{200b}\"",
            1,
        );
        assert!(parse(non_whitespace_format.as_bytes()).is_ok());

        let malformed_display = String::from_utf8(MATH_FIXTURE.to_vec()).unwrap().replacen(
            "\"utf8\":\"x^{2}x+1\"",
            "\"utf8\":\"x^{2}x+}\"",
            1,
        );
        let error = parse(malformed_display.as_bytes()).unwrap_err();
        assert_eq!(
            error.downcast_ref::<StagingSemanticSyntaxError>(),
            Some(&StagingSemanticSyntaxError::InvalidMathSource {
                source_id: SourceId::new(0),
                byte_offset: Utf8ByteOffset::new(7),
            })
        );
    }

    #[test]
    fn math_is_admitted_directly_in_the_document_body() {
        let top_level =
            typaxis_document_package::staging_math_document_body_fixture(MATH_FIXTURE).unwrap();
        let package = parse(&top_level).unwrap();
        assert_eq!(package.semantic_container_count(), 0);
        assert_eq!(package.math_nodes().len(), 2);
        assert_eq!(package.math_nodes()[0].domain().node_id, NodeId::new(2));
        assert_eq!(package.math_nodes()[1].domain().node_id, NodeId::new(3));
        assert_eq!(
            package.math_nodes()[1].domain().owner_node_id,
            NodeId::new(0)
        );
    }

    #[test]
    fn semantic_container_decode_and_syntax_limits_are_one_receipted_input() {
        let decode_limits =
            ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&decode_limits))
            .unwrap();
        let mut different = typaxis_core::ResourceLimits::default();
        different.max_pages -= 1;
        let different = ValidatedResourceLimits::new(different).unwrap();
        assert!(matches!(
            StagingSemanticPackageParser::new().parse(decoded, &different),
            Err(StagingSemanticSyntaxError::ReceiptMismatch)
        ));
    }

    #[test]
    fn semantic_container_recursive_empty_reaches_profile_boundary_but_bad_owner_and_style_do_not()
    {
        fn remove_inline_content(inline: &mut WireStagingM4Inline) {
            match inline {
                WireStagingM4Inline::Text { node_id, span, .. }
                | WireStagingM4Inline::Reference { node_id, span, .. }
                | WireStagingM4Inline::FootnoteReference { node_id, span, .. } => {
                    *inline = WireStagingM4Inline::HardBreak {
                        node_id: *node_id,
                        span: *span,
                    };
                }
                WireStagingM4Inline::Emphasis { children, .. }
                | WireStagingM4Inline::Strong { children, .. }
                | WireStagingM4Inline::Link { children, .. } => {
                    children.iter_mut().for_each(remove_inline_content);
                }
                WireStagingM4Inline::Anchor { .. }
                | WireStagingM4Inline::InlineMath { .. }
                | WireStagingM4Inline::InlineVector { .. }
                | WireStagingM4Inline::MathVector { .. }
                | WireStagingM4Inline::SoftBreak { .. }
                | WireStagingM4Inline::HardBreak { .. } => {}
            }
        }

        fn remove_block_content(blocks: &mut [WireStagingM4Block]) {
            for block in blocks {
                match block {
                    WireStagingM4Block::Paragraph { children, .. }
                    | WireStagingM4Block::Heading { children, .. } => {
                        children.iter_mut().for_each(remove_inline_content);
                    }
                    WireStagingM4Block::List { items, .. } => {
                        for item in items {
                            remove_block_content(&mut item.blocks);
                        }
                    }
                    WireStagingM4Block::Table { head, body, .. } => {
                        for cell in head.iter_mut().chain(body).flat_map(|row| &mut row.cells) {
                            remove_block_content(&mut cell.blocks);
                        }
                    }
                    WireStagingM4Block::Figure { caption, .. }
                    | WireStagingM4Block::VectorFigure { caption, .. }
                    | WireStagingM4Block::SemanticContainer {
                        blocks: caption, ..
                    } => remove_block_content(caption),
                    WireStagingM4Block::PageBreak { .. }
                    | WireStagingM4Block::DisplayMath { .. }
                    | WireStagingM4Block::MathVectorBlock { .. } => {}
                }
            }
        }

        let empty = mutate_and_encode(|wire| {
            let mut document = wire.document().clone();
            remove_block_content(&mut document.blocks);
            wire.replace_typed_regions(document, wire.resources().clone());
        });
        assert!(parse(&empty).is_ok());

        let foreign = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
            "\"end_byte\":6,\"source_id\":0,\"start_byte\":0},\"text_span\"",
            "\"end_byte\":20,\"source_id\":0,\"start_byte\":0},\"text_span\"",
            1,
        );
        assert!(parse(foreign.as_bytes()).is_err());

        let width = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
            "\"name\":\"space_before\"",
            "\"name\":\"width\"",
            1,
        );
        assert!(parse(width.as_bytes()).is_err());
    }

    #[test]
    fn semantic_container_style_honors_important_extends_and_rejects_prefix_aliases() {
        let inherited = mutate_and_encode(|wire| {
            let mut sheet = wire.style_sheet().clone();
            sheet.rules[0].declarations[0].important = true;
            sheet.rules[1].extends = Some("semantic-base".to_owned());
            wire.replace_style_sheet(sheet);
        });
        let package = parse(&inherited).unwrap();
        assert_eq!(
            package
                .computed_style(NodeId::new(1))
                .unwrap()
                .block_style()
                .space_before()
                .get()
                .raw(),
            2
        );

        let malformed = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
            "\"selector\":\"semantic_container\"",
            "\"selector\":\"semantic_container_alias\"",
            1,
        );
        assert!(parse(malformed.as_bytes()).is_err());

        let unknown_parent = String::from_utf8(FIXTURE.to_vec()).unwrap().replacen(
            "\"extends\":null,\"selector\":\"semantic_container.feature\"",
            "\"extends\":\"missing\",\"selector\":\"semantic_container.feature\"",
            1,
        );
        assert!(parse(unknown_parent.as_bytes()).is_err());

        let inherited_align = mutate_and_encode(|wire| {
            let mut sheet = wire.style_sheet().clone();
            sheet.rules[1]
                .declarations
                .push(WireStagingStyleDeclaration {
                    important: false,
                    name: "text_align".to_owned(),
                    value: WireStagingStyleValue::Keyword {
                        value: "end".to_owned(),
                    },
                });
            wire.replace_style_sheet(sheet);
        });
        let package = parse(&inherited_align).unwrap();
        assert_eq!(
            package
                .computed_style(NodeId::new(4))
                .unwrap()
                .block_style()
                .text_align()
                .as_str(),
            "end"
        );

        let inherited_text = mutate_and_encode(|wire| {
            let mut sheet = wire.style_sheet().clone();
            sheet.rules[1].declarations.extend([
                WireStagingStyleDeclaration {
                    important: false,
                    name: "font_family".to_owned(),
                    value: WireStagingStyleValue::FontFamilyList {
                        families: vec!["Body".to_owned()],
                    },
                },
                WireStagingStyleDeclaration {
                    important: false,
                    name: "font_size".to_owned(),
                    value: WireStagingStyleValue::Length { value: 10 },
                },
                WireStagingStyleDeclaration {
                    important: false,
                    name: "line_height".to_owned(),
                    value: WireStagingStyleValue::Length { value: 12 },
                },
                WireStagingStyleDeclaration {
                    important: false,
                    name: "page".to_owned(),
                    value: WireStagingStyleValue::String {
                        value: "chapter".to_owned(),
                    },
                },
            ]);
            wire.replace_style_sheet(sheet);
        });
        let package = parse(&inherited_text).unwrap();
        let result = package.computed_style(NodeId::new(1)).unwrap();
        let proof = package.computed_style(NodeId::new(4)).unwrap();
        assert_eq!(
            result.inheritance_style().font_families().unwrap(),
            ["Body"]
        );
        assert_eq!(proof.inheritance_style().font_families().unwrap(), ["Body"]);
        assert_eq!(
            proof.inheritance_style().font_size().unwrap().get().raw(),
            10
        );
        assert_eq!(
            proof.inheritance_style().line_height().unwrap().get().raw(),
            12
        );
        assert_eq!(result.page_name().unwrap().as_str(), "chapter");
        assert!(proof.page_name().is_none());

        let isolated_extends = mutate_and_encode(|wire| {
            let mut document = wire.document().clone();
            let WireStagingM4Block::SemanticContainer { classes, .. } = &mut document.blocks[0]
            else {
                panic!("fixture root must remain semantic")
            };
            classes.insert(0, "__m4_inheritance_only".to_owned());
            let resources = wire.resources().clone();
            wire.replace_typed_regions(document, resources);

            let mut sheet = wire.style_sheet().clone();
            let mut ordinary_parent = sheet.rules[0].clone();
            ordinary_parent.style_id = "ordinary-parent".to_owned();
            ordinary_parent.extends = None;
            ordinary_parent.selector = "paragraph".to_owned();
            ordinary_parent.source_order = 2;
            ordinary_parent.declarations = vec![WireStagingStyleDeclaration {
                important: false,
                name: "space_after".to_owned(),
                value: WireStagingStyleValue::Length { value: 99 },
            }];
            let mut nested = sheet.rules[0].clone();
            nested.style_id = "semantic-nested".to_owned();
            nested.extends = Some("ordinary-parent".to_owned());
            nested.selector = "semantic_container.nested".to_owned();
            nested.source_order = 3;
            nested.declarations = vec![WireStagingStyleDeclaration {
                important: false,
                name: "space_before".to_owned(),
                value: WireStagingStyleValue::Length { value: 2 },
            }];
            sheet.rules.extend([ordinary_parent, nested]);
            wire.replace_style_sheet(sheet);
        });
        let package = parse(&isolated_extends).unwrap();
        assert_eq!(
            package
                .computed_style(NodeId::new(1))
                .unwrap()
                .block_style()
                .space_after()
                .get()
                .raw(),
            3
        );
        assert_eq!(
            package
                .computed_style(NodeId::new(4))
                .unwrap()
                .block_style()
                .space_after()
                .get()
                .raw(),
            99
        );

        for (name, value) in [
            (
                "width",
                WireStagingStyleValue::Keyword {
                    value: "auto".to_owned(),
                },
            ),
            (
                "keep_caption",
                WireStagingStyleValue::Boolean { value: true },
            ),
        ] {
            let inapplicable = mutate_and_encode(|wire| {
                let mut sheet = wire.style_sheet().clone();
                sheet.rules[0].declarations[0].name = name.to_owned();
                sheet.rules[0].declarations[0].value = value;
                wire.replace_style_sheet(sheet);
            });
            assert!(matches!(
                parse(&inapplicable),
                Err(error) if error.to_string().contains("inapplicable")
            ));
        }
    }
}
