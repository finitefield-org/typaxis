use super::*;
use typaxis_document::{
    FontMediaDeclaration, FontMediaType, ImageMediaDeclaration, ImageMediaType,
    SemanticContainerKind, StagingM4Block, StagingM4BlockCommon, StagingM4Document,
    StagingM4FigurePlacement, StagingM4FontFaceDeclaration, StagingM4FootnoteDefinition,
    StagingM4ImageDeclaration, StagingM4ListItem, StagingM4MathKind, StagingM4MathNode,
    StagingM4ResourceCatalog, StagingM4TableCell, StagingM4TableRow,
};
use typaxis_document_package::{
    DecodedStagingSemanticDocumentPackage, WireFontMediaType, WireImageMediaType,
    WireStagingM4Block, WireStagingM4Document, WireStagingM4DocumentPackage, WireStagingM4Inline,
    WireStagingM4LinkTarget, WireStagingM4ResourceCatalog, WireStagingM4Source,
    WireStagingM4TextBuffer, WireStagingMathSource, WireStagingSourceSpan, WireStagingStyleSheet,
    WireStagingStyleValue, WireStagingTextMapKind, WireStagingTextSpan,
};
use typaxis_math::{parse_math_source, MathParseLimits, ParsedMathReceipt};
use typaxis_style::{
    cascade_staging_display_math_style, cascade_staging_semantic_container_style,
    cascade_staging_semantic_descendant_style, close_staging_inline_math_style,
    SemanticContainerComputedStyle, SemanticContainerInheritanceStyle, SemanticContainerStyleKind,
    StagingMathComputedStyle,
};

const SEMANTIC_SYNTAX_FINGERPRINT_ALGORITHM: &str = "typaxis.semantic-container-syntax/1";
const STAGING_PROFILE_ID: &str = "typaxis.machine-pdf/production-book-1";
const STAGING_PROFILE_RECEIPT_ALGORITHM: &str = "typaxis.production-book-profile-receipt/1";
const INTERNAL_HIDDEN_STYLE_CLASS: &str = "__typaxis_internal_hidden";

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
            Self::MathSourceTextLimit => formatter.write_str("T2100: math source limit exceeded"),
            Self::MathSpeechLimit => formatter.write_str("T2101: math speech limit exceeded"),
            Self::InvalidResource => formatter.write_str("P1102: invalid declared-media resource"),
            Self::InvalidPageGeometry => {
                formatter.write_str("P1102: SafeVector requires one closed default page frame")
            }
            Self::InvalidStyle => formatter.write_str("L5101: invalid semantic_container style"),
            Self::InapplicableStyle => {
                formatter.write_str("L5101: inapplicable semantic_container property")
            }
            Self::AstNodeLimit => formatter.write_str("P1102: semantic AST exceeds max_ast_nodes"),
            Self::AstDepthLimit => {
                formatter.write_str("P1102: semantic AST exceeds max_ast_nesting_depth")
            }
            Self::MathAstNodeLimit => formatter.write_str("P1120: math AST exceeds max_ast_nodes"),
            Self::MathAstDepthLimit => {
                formatter.write_str("P1121: math AST exceeds max_ast_nesting_depth")
            }
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

impl std::error::Error for StagingSemanticSyntaxError {}

/// Syntax-owned proof of the complete contract-1.4 semantic and declared-media
/// lowering. The original typed carrier is retained for a checked canonical
/// re-encode; no public contract decoder can consume it.
#[derive(Debug)]
pub struct ValidatedStagingSemanticPackage {
    wire: WireStagingM4DocumentPackage,
    limits: ValidatedResourceLimits,
    document: StagingM4Document,
    resources: StagingM4ResourceCatalog,
    computed_styles: BTreeMap<NodeId, SemanticContainerComputedStyle>,
    math_nodes: Vec<ValidatedStagingMathNode>,
    raw_sha256: [u8; 32],
    canonical_jcs_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    semantic_jcs: String,
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
        package.checked_wire()?;
        if package.limits() != limits {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        let mut container_count = 0u32;
        validate_profile_container_domain(&package.document.blocks, &mut container_count)?;
        for footnote in &package.document.footnotes {
            validate_profile_container_domain(&footnote.blocks, &mut container_count)?;
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
        )?;
        for footnote in &package.document().footnotes {
            collect_vector_figure_owners(
                &footnote.blocks,
                package,
                &vector_set,
                &mut figure_owners,
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

/// Closed private production-book authorization for the MI4-05 math slice.
/// Wrapping the SafeVector authorization proves that the target's required
/// vector-media policy and page geometry were preflighted as one domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathProfileView {
    base: StagingSafeVectorProfileView,
    math_node_ids: Vec<NodeId>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathProfileView {
    pub fn new(
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<Self, StagingSemanticSyntaxError> {
        let base = StagingSafeVectorProfileView::new(package, limits)?;
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
            math_node_ids,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn base(&self) -> &StagingSafeVectorProfileView {
        &self.base
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
        let expected = StagingMathProfileView::new(package, limits)?;
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

fn collect_vector_figure_owners(
    blocks: &[StagingM4Block],
    package: &ValidatedStagingSemanticPackage,
    vectors: &BTreeSet<ImageResourceId>,
    output: &mut Vec<NodeId>,
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
                collect_vector_figure_owners(caption, package, vectors, output)?;
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    collect_vector_figure_owners(&item.blocks, package, vectors, output)?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_vector_figure_owners(&cell.blocks, package, vectors, output)?;
                }
            }
            StagingM4Block::SemanticContainer { blocks, .. } => {
                collect_vector_figure_owners(blocks, package, vectors, output)?;
            }
            _ => {}
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
                validate_profile_container_domain(blocks, count)?;
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    validate_profile_container_domain(&item.blocks, count)?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    validate_profile_container_domain(&cell.blocks, count)?;
                }
            }
            StagingM4Block::Figure { caption, .. } => {
                validate_profile_container_domain(caption, count)?;
            }
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
    pub fn math_nodes(&self) -> &[ValidatedStagingMathNode] {
        &self.math_nodes
    }
    pub fn math_node(&self, owner: NodeId) -> Option<&ValidatedStagingMathNode> {
        self.math_nodes
            .iter()
            .find(|value| value.domain.node_id == owner)
    }
    pub fn checked_wire(
        &self,
    ) -> Result<&WireStagingM4DocumentPackage, StagingSemanticSyntaxError> {
        let observed = encode_semantic_receipt(
            &self.document,
            &self.resources,
            &self.computed_styles,
            &self.math_nodes,
            self.canonical_jcs_sha256,
        );
        if observed != self.semantic_jcs || sha256(observed.as_bytes()) != self.semantic_fingerprint
        {
            return Err(StagingSemanticSyntaxError::ReceiptMismatch);
        }
        Ok(&self.wire)
    }
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
            next_node_id: 0,
            node_count: 0,
            admitted_text_and_math_speech_bytes: admitted_text_bytes,
            math_nodes: Vec::new(),
            limits,
        };
        validator.node(wire.document().node_id, None, 1)?;
        let document = lower_document(wire.document(), &mut validator)?;
        let pending_math = std::mem::take(&mut validator.math_nodes);
        let resources = lower_resources(wire.resources())?;
        let rules = lower_semantic_style_rules(wire.style_sheet(), limits)?;
        let mut computed_styles = BTreeMap::new();
        let mut math_styles = BTreeMap::new();
        collect_computed_styles(
            &document.blocks,
            &rules,
            None,
            &pending_math,
            &mut computed_styles,
            &mut math_styles,
        )?;
        for footnote in &document.footnotes {
            collect_computed_styles(
                &footnote.blocks,
                &rules,
                None,
                &pending_math,
                &mut computed_styles,
                &mut math_styles,
            )?;
        }
        if computed_styles.is_empty() && pending_math.is_empty() {
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
            canonical_jcs_sha256,
        );
        Ok(ValidatedStagingSemanticPackage {
            wire,
            limits: limits.clone(),
            document,
            resources,
            computed_styles,
            math_nodes,
            raw_sha256,
            canonical_jcs_sha256,
            semantic_fingerprint: sha256(semantic_jcs.as_bytes()),
            semantic_jcs,
        })
    }
}

struct SemanticValidator<'a> {
    sources: &'a BTreeMap<u32, u32>,
    text_buffers: &'a BTreeMap<u32, WireStagingM4TextBuffer>,
    next_node_id: u32,
    node_count: u64,
    admitted_text_and_math_speech_bytes: u64,
    math_nodes: Vec<PendingStagingMathNode>,
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
        if node_id != self.next_node_id {
            return Err(StagingSemanticSyntaxError::InvalidNodeOrder);
        }
        self.next_node_id = self
            .next_node_id
            .checked_add(1)
            .ok_or(StagingSemanticSyntaxError::AstNodeLimit)?;
        self.node_count = self
            .node_count
            .checked_add(1)
            .ok_or(StagingSemanticSyntaxError::AstNodeLimit)?;
        if self.node_count > self.limits.get().max_ast_nodes {
            return Err(StagingSemanticSyntaxError::AstNodeLimit);
        }
        if depth > self.limits.get().max_ast_nesting_depth {
            return Err(StagingSemanticSyntaxError::AstDepthLimit);
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
        validator.node(block.node_id(), Some(span), depth)?;
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
                let has_authored_content =
                    validate_inlines(children, validator, Some(span), common.node_id, depth + 1)?;
                StagingM4Block::Paragraph {
                    common,
                    has_authored_content,
                }
            }
            WireStagingM4Block::Heading {
                level, children, ..
            } => {
                if !(1..=6).contains(level) {
                    return Err(StagingSemanticSyntaxError::InvalidBlock(common.node_id));
                }
                let has_authored_content =
                    validate_inlines(children, validator, Some(span), common.node_id, depth + 1)?;
                StagingM4Block::Heading {
                    common,
                    has_authored_content,
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
) -> Result<bool, StagingSemanticSyntaxError> {
    let mut has_authored_content = false;
    let mut previous_start = None;
    for value in values {
        let span = value.span();
        validator.node(value.node_id(), Some(span), depth)?;
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
            WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. } => {
                validate_inlines(children, validator, Some(span), math_owner, depth + 1)?
            }
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
                validate_inlines(children, validator, Some(span), math_owner, depth + 1)?
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
            || (font.media_type == WireFontMediaType::SfntTrueTypeGlyf && font.face_index != 0)
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
        images.push(StagingM4ImageDeclaration {
            image_id: ImageResourceId::new(image.image_id),
            uri: PortablePath::new(image.uri.clone())
                .map_err(|_| StagingSemanticSyntaxError::InvalidResource)?,
            expected_sha256: parse_optional_hash(image.expected_sha256.as_deref())?,
            media: ImageMediaDeclaration::Declared(match image.media_type {
                WireImageMediaType::Png => ImageMediaType::Png,
                WireImageMediaType::SvgSafe1 => ImageMediaType::SvgSafe1,
            }),
        });
    }
    Ok(StagingM4ResourceCatalog { font_faces, images })
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
    }
    let validation_sheet = StyleSheet {
        rules: parsed.clone(),
    };
    validation_sheet
        .validate_table_document_styles()
        .map_err(map_semantic_style_error)?;

    let mut ordinary_rules = parsed.clone();
    for (index, rule) in ordinary_rules.iter_mut().enumerate() {
        if semantic_rules[index] || math_rules[index] {
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

    let semantic = isolate_staging_style_rules(&parsed, &semantic_rules)?;
    let math = isolate_staging_style_rules(&parsed, &math_rules)?;
    Ok(StagingSemanticStyleSheets {
        semantic,
        ordinary,
        math,
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
                    math_output,
                )?
            }
            _ => {}
        }
    }
    Ok(())
}

fn encode_semantic_receipt(
    document: &StagingM4Document,
    resources: &StagingM4ResourceCatalog,
    styles: &BTreeMap<NodeId, SemanticContainerComputedStyle>,
    math: &[ValidatedStagingMathNode],
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
            _ => {}
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

    fn parse(bytes: &[u8]) -> Result<ValidatedStagingSemanticPackage, Box<dyn std::error::Error>> {
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
            .expect("default limits are valid");
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(bytes, &DocumentPackageDecodePolicy::new(&limits))?;
        Ok(StagingSemanticPackageParser::new().parse(decoded, &limits)?)
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
                    | WireStagingM4Block::SemanticContainer {
                        blocks: caption, ..
                    } => remove_block_content(caption),
                    WireStagingM4Block::PageBreak { .. }
                    | WireStagingM4Block::DisplayMath { .. } => {}
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
