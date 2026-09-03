use std::collections::BTreeSet;
use typaxis_core::{
    push_jcs_string, sha256, ImageResourceId, Length, M4EffectiveResourceLimits, NodeId,
    PositiveLength, Rect, SourceSpan,
};
use typaxis_document::{
    ImageMediaDeclaration, ImageMediaType, StagingM4Block, StagingM4FigurePlacement,
};
use typaxis_layout_contract::{
    MathVectorBlockPlacementInput, MathVectorBlockStyleInput, PrecomposedVectorBindingFingerprint,
    PrecomposedVectorGeometryError, PrecomposedVectorInlinePlacementInput,
    PrecomposedVectorPlacementInput, ResolvedRgb8, VectorFigurePlacementInput,
    VectorFigureStyleInput,
};
use typaxis_resource_admission::{
    close_staging_declared_media, AdmittedImageMediaKind, AdmittedResourceLedger,
    ResourceAdmissionProgressToken, SafeVectorAdmissionAttestation, SafeVectorParserProfile,
};
#[cfg(any(test, feature = "staging-fixtures"))]
use typaxis_syntax::StagingPrecomposedVectorProfileSessionIdentity;
use typaxis_syntax::{
    PrecomposedVectorKind, PrecomposedVectorMetricPayload, StagingM4PageGeometry,
    StagingPrecomposedVectorProfileAuthorization, StagingPrecomposedVectorProfileProgressToken,
    StagingSafeVectorProfileView, ValidatedStagingSemanticPackage,
};

pub const PRECOMPOSED_VECTOR_BINDING_ALGORITHM: &str = "typaxis.precomposed-vector-binding/1";
pub const PRECOMPOSED_VECTOR_BINDING_SET_ALGORITHM: &str =
    "typaxis.precomposed-vector-binding-set/1";
const PRECOMPOSED_VECTOR_LAYOUT_EPOCH_ALGORITHM: &str = "typaxis.precomposed-vector-layout-epoch/1";
pub const STAGING_SAFE_VECTOR_SELECTED_LAYOUT_ALGORITHM: &str =
    "typaxis.safe-vector-selected-layout/1";
const FIXED_ONE: i64 = 65_536;

/// Declared and admitted vector media after the two identities have been
/// closed. Raster media is not representable.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BoundPrecomposedVectorMedia {
    SafeSvg1,
    SafeSvg2,
}

impl BoundPrecomposedVectorMedia {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SafeSvg1 => "svg-safe-1",
            Self::SafeSvg2 => "svg-safe-2",
        }
    }
}

/// Stable identities for one admitted vector resource. Raw URI, source SVG,
/// and the caller's unverified expected hash are deliberately absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundPrecomposedVectorResource {
    image_id: ImageResourceId,
    declared_media: BoundPrecomposedVectorMedia,
    admitted_media: BoundPrecomposedVectorMedia,
    source_sha256: [u8; 32],
    parser_profile: SafeVectorParserProfile,
    parser_id: &'static str,
    ir_id: &'static str,
    ir_fingerprint_id: &'static str,
    ir_fingerprint: [u8; 32],
    intrinsic_width: PositiveLength,
    intrinsic_height: PositiveLength,
    view_box: [i64; 4],
    limits_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
}

impl BoundPrecomposedVectorResource {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }

    pub const fn declared_media(&self) -> BoundPrecomposedVectorMedia {
        self.declared_media
    }

    pub const fn admitted_media(&self) -> BoundPrecomposedVectorMedia {
        self.admitted_media
    }

    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }

    pub const fn parser_profile(&self) -> SafeVectorParserProfile {
        self.parser_profile
    }

    pub const fn parser_id(&self) -> &'static str {
        self.parser_id
    }

    pub const fn ir_id(&self) -> &'static str {
        self.ir_id
    }

    pub const fn ir_fingerprint_id(&self) -> &'static str {
        self.ir_fingerprint_id
    }

    pub const fn ir_fingerprint(&self) -> [u8; 32] {
        self.ir_fingerprint
    }

    pub const fn intrinsic_width(&self) -> PositiveLength {
        self.intrinsic_width
    }

    pub const fn intrinsic_height(&self) -> PositiveLength {
        self.intrinsic_height
    }

    pub const fn view_box(&self) -> [i64; 4] {
        self.view_box
    }

    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }

    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
}

/// Deterministic identity of every stable input shared by all resource-aware
/// precomposed-vector receipts in one layout pass. Process-local session
/// identities are retained by [`ValidatedPrecomposedVectorBindings`] instead
/// of being serialized into this value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorLayoutEpoch {
    package_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    profile_authorization_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    declared_media_fingerprint: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl PrecomposedVectorLayoutEpoch {
    fn new(
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingPrecomposedVectorProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        declared_media_fingerprint: [u8; 32],
    ) -> Self {
        let mut canonical_jcs = String::from("{\"algorithm\":");
        push_jcs_string(
            &mut canonical_jcs,
            PRECOMPOSED_VECTOR_LAYOUT_EPOCH_ALGORITHM,
        );
        canonical_jcs.push_str(",\"admitted_fingerprint\":");
        push_hash(&mut canonical_jcs, admitted.fingerprint().bytes());
        canonical_jcs
            .push_str(",\"contract\":\"typaxis.contract/1.4\",\"declared_media_fingerprint\":");
        push_hash(&mut canonical_jcs, declared_media_fingerprint);
        canonical_jcs.push_str(",\"limits_fingerprint\":");
        push_hash(&mut canonical_jcs, limits.fingerprint());
        canonical_jcs.push_str(",\"package_sha256\":");
        push_hash(&mut canonical_jcs, package.canonical_jcs_sha256());
        canonical_jcs.push_str(",\"profile_authorization_fingerprint\":");
        push_hash(&mut canonical_jcs, profile.profile_fingerprint());
        canonical_jcs.push_str(",\"profile_fingerprint\":");
        push_hash(&mut canonical_jcs, profile.profile_receipt_fingerprint());
        canonical_jcs.push_str(",\"semantic_fingerprint\":");
        push_hash(&mut canonical_jcs, package.semantic_fingerprint());
        canonical_jcs.push('}');
        Self {
            package_sha256: package.canonical_jcs_sha256(),
            semantic_fingerprint: package.semantic_fingerprint(),
            profile_fingerprint: profile.profile_receipt_fingerprint(),
            profile_authorization_fingerprint: profile.profile_fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            admitted_fingerprint: admitted.fingerprint().bytes(),
            declared_media_fingerprint,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        }
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }

    pub const fn semantic_fingerprint(&self) -> [u8; 32] {
        self.semantic_fingerprint
    }

    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }

    pub const fn profile_authorization_fingerprint(&self) -> [u8; 32] {
        self.profile_authorization_fingerprint
    }

    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }

    pub const fn admitted_fingerprint(&self) -> [u8; 32] {
        self.admitted_fingerprint
    }

    pub const fn declared_media_fingerprint(&self) -> [u8; 32] {
        self.declared_media_fingerprint
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// Common four-kind resource, geometry, paint, alternative, and language
/// binding. Math-only TeX/ActualText/provenance state is held by the nominally
/// distinct [`crate::ValidatedMathVectorReceipt`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedPrecomposedVectorReceipt {
    epoch_fingerprint: [u8; 32],
    node_id: NodeId,
    kind: PrecomposedVectorKind,
    owner_source_span: SourceSpan,
    metrics_fingerprint: [u8; 32],
    resource: BoundPrecomposedVectorResource,
    placement: PrecomposedVectorPlacementInput,
    alternative: String,
    alternative_sha256: [u8; 32],
    language: Option<String>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl ValidatedPrecomposedVectorReceipt {
    pub const fn algorithm(&self) -> &'static str {
        PRECOMPOSED_VECTOR_BINDING_ALGORITHM
    }

    pub const fn epoch_fingerprint(&self) -> [u8; 32] {
        self.epoch_fingerprint
    }

    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub const fn kind(&self) -> PrecomposedVectorKind {
        self.kind
    }

    pub const fn owner_source_span(&self) -> SourceSpan {
        self.owner_source_span
    }

    pub const fn metrics_fingerprint(&self) -> [u8; 32] {
        self.metrics_fingerprint
    }

    pub const fn resource(&self) -> &BoundPrecomposedVectorResource {
        &self.resource
    }

    pub const fn placement(&self) -> &PrecomposedVectorPlacementInput {
        &self.placement
    }

    pub fn alternative(&self) -> &str {
        &self.alternative
    }

    pub const fn alternative_sha256(&self) -> [u8; 32] {
        self.alternative_sha256
    }

    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub const fn binding_fingerprint(&self) -> PrecomposedVectorBindingFingerprint {
        PrecomposedVectorBindingFingerprint::from_receipt(self.fingerprint)
    }

    fn integrity_matches(&self) -> bool {
        let observed = encode_precomposed_vector_receipt(self);
        self.alternative_sha256 == sha256(self.alternative.as_bytes())
            && self.canonical_jcs == observed
            && self.fingerprint == sha256(observed.as_bytes())
            && placement_matches_kind(&self.placement, self.kind)
            && placement_paint(&self.placement) == ResolvedRgb8::BLACK
    }
}

/// Complete owner-controlled join. The profile and resource session tokens
/// are process-local; deterministic epoch and per-node receipts are the only
/// state exposed to later layout stages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedPrecomposedVectorBindings {
    profile_progress: StagingPrecomposedVectorProfileProgressToken,
    admission_progress: ResourceAdmissionProgressToken,
    epoch: PrecomposedVectorLayoutEpoch,
    receipts: Vec<ValidatedPrecomposedVectorReceipt>,
    pub(super) math_receipts: Vec<crate::ValidatedMathVectorReceipt>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl ValidatedPrecomposedVectorBindings {
    pub const fn epoch(&self) -> &PrecomposedVectorLayoutEpoch {
        &self.epoch
    }

    pub fn receipts(&self) -> &[ValidatedPrecomposedVectorReceipt] {
        &self.receipts
    }

    pub fn math_receipts(&self) -> &[crate::ValidatedMathVectorReceipt] {
        &self.math_receipts
    }

    pub fn receipt(&self, owner: NodeId) -> Option<&ValidatedPrecomposedVectorReceipt> {
        self.receipts
            .binary_search_by_key(&owner, ValidatedPrecomposedVectorReceipt::node_id)
            .ok()
            .map(|index| &self.receipts[index])
    }

    pub fn math_receipt(&self, owner: NodeId) -> Option<&crate::ValidatedMathVectorReceipt> {
        self.math_receipts
            .binary_search_by_key(&owner, crate::ValidatedMathVectorReceipt::node_id)
            .ok()
            .map(|index| &self.math_receipts[index])
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    #[cfg(test)]
    pub(super) fn reseal_binding_set_for_test(&mut self) {
        self.canonical_jcs =
            encode_precomposed_vector_binding_set(&self.epoch, &self.receipts, &self.math_receipts);
        self.fingerprint = sha256(self.canonical_jcs.as_bytes());
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingPrecomposedVectorProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
    ) -> Result<(), PrecomposedVectorBindingError> {
        let expected = build_precomposed_vector_bindings(package, profile, limits, admitted)
            .map_err(|_| PrecomposedVectorBindingError::ReceiptMismatch)?;
        if self != &expected
            || !self
                .receipts
                .iter()
                .all(ValidatedPrecomposedVectorReceipt::integrity_matches)
            || !self
                .math_receipts
                .iter()
                .all(crate::ValidatedMathVectorReceipt::integrity_matches)
            || !profile.matches_progress(&self.profile_progress)
            || !admitted.token().matches_progress(&self.admission_progress)
        {
            return Err(PrecomposedVectorBindingError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrecomposedVectorBindingError {
    ProfileMismatch,
    AdmissionMismatch,
    ResourceMismatch(NodeId),
    InvalidScale(NodeId),
    InvalidMetrics(NodeId),
    StyleMismatch(NodeId),
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for PrecomposedVectorBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => {
                formatter.write_str("I9190: precomposed vector profile binding mismatch")
            }
            Self::AdmissionMismatch => {
                formatter.write_str("I9190: precomposed vector admission binding mismatch")
            }
            Self::ResourceMismatch(owner) => write!(
                formatter,
                "I9190: precomposed vector resource binding mismatch at node {}",
                owner.get()
            ),
            Self::InvalidScale(owner) => write!(
                formatter,
                "P1102: precomposed vector uniform scale mismatch at node {}",
                owner.get()
            ),
            Self::InvalidMetrics(owner) => write!(
                formatter,
                "P1102: precomposed vector metric relation mismatch at node {}",
                owner.get()
            ),
            Self::StyleMismatch(owner) => write!(
                formatter,
                "I9190: precomposed vector style binding mismatch at node {}",
                owner.get()
            ),
            Self::AllocationFailure => {
                formatter.write_str("P1102: precomposed vector binding allocation failed")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: precomposed vector binding receipt mismatch")
            }
        }
    }
}

impl std::error::Error for PrecomposedVectorBindingError {}

pub fn bind_staging_precomposed_vectors(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
) -> Result<ValidatedPrecomposedVectorBindings, PrecomposedVectorBindingError> {
    let bindings = build_precomposed_vector_bindings(package, profile, limits, admitted)?;
    bindings.verify(package, profile, limits, admitted)?;
    Ok(bindings)
}

fn build_precomposed_vector_bindings(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
) -> Result<ValidatedPrecomposedVectorBindings, PrecomposedVectorBindingError> {
    profile
        .authorizes(package, limits)
        .map_err(|_| PrecomposedVectorBindingError::ProfileMismatch)?;
    package
        .checked_wire()
        .map_err(|_| PrecomposedVectorBindingError::ProfileMismatch)?;
    let media = close_staging_declared_media(admitted, package.resources())
        .map_err(|_| PrecomposedVectorBindingError::AdmissionMismatch)?;
    if !admitted.matches_declarations(
        typaxis_resource_admission::staging_declared_base_catalog(package.resources())
            .map_err(|_| PrecomposedVectorBindingError::AdmissionMismatch)?
            .resource_catalog(),
    ) || profile.vector_owners().count() != package.precomposed_vector_metrics().len()
    {
        return Err(PrecomposedVectorBindingError::AdmissionMismatch);
    }
    let epoch =
        PrecomposedVectorLayoutEpoch::new(package, profile, limits, admitted, media.fingerprint());
    let mut receipts = Vec::new();
    receipts
        .try_reserve_exact(package.precomposed_vector_metrics().len())
        .map_err(|_| PrecomposedVectorBindingError::AllocationFailure)?;
    let math_count = package
        .precomposed_vector_metrics()
        .iter()
        .filter(|metrics| {
            matches!(
                metrics.kind(),
                PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock
            )
        })
        .count();
    let mut math_receipts = Vec::new();
    math_receipts
        .try_reserve_exact(math_count)
        .map_err(|_| PrecomposedVectorBindingError::AllocationFailure)?;

    for metrics in package.precomposed_vector_metrics() {
        package
            .verify_precomposed_vector_metrics(metrics)
            .map_err(|_| PrecomposedVectorBindingError::ReceiptMismatch)?;
        let owner = metrics.node_id();
        let image_id = metrics.resource_binding().image_id();
        let declaration = package
            .resources()
            .images
            .get(image_id.get() as usize)
            .filter(|declaration| declaration.image_id == image_id)
            .ok_or(PrecomposedVectorBindingError::ResourceMismatch(owner))?;
        let image = admitted
            .image(image_id)
            .ok_or(PrecomposedVectorBindingError::ResourceMismatch(owner))?;
        let attestation = image
            .safe_vector_attestation()
            .ok_or(PrecomposedVectorBindingError::ResourceMismatch(owner))?;
        let media_attestation = media
            .images()
            .get(image_id.get() as usize)
            .filter(|value| value.image_id() == image_id)
            .ok_or(PrecomposedVectorBindingError::ResourceMismatch(owner))?;
        let resource = bind_precomposed_vector_resource(
            owner,
            metrics.kind(),
            declaration,
            &attestation,
            media_attestation,
            profile,
            limits,
        )?;
        let placement = bind_precomposed_vector_placement(package, metrics, &resource)?;
        let alternative = metrics.alternative().alternative().to_owned();
        let language = metrics.language().map(|value| value.canonical().to_owned());
        let mut receipt = ValidatedPrecomposedVectorReceipt {
            epoch_fingerprint: epoch.fingerprint(),
            node_id: owner,
            kind: metrics.kind(),
            owner_source_span: metrics.owner_source_span(),
            metrics_fingerprint: metrics.fingerprint(),
            resource,
            placement,
            alternative_sha256: sha256(alternative.as_bytes()),
            alternative,
            language,
            canonical_jcs: String::new(),
            fingerprint: [0; 32],
        };
        receipt.canonical_jcs = encode_precomposed_vector_receipt(&receipt);
        receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
        if !receipt.integrity_matches() {
            return Err(PrecomposedVectorBindingError::ReceiptMismatch);
        }
        if matches!(
            metrics.kind(),
            PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock
        ) {
            math_receipts.push(crate::math::issue_precomposed_math_vector_receipt(
                &receipt,
                metrics,
                declaration,
            )?);
        }
        receipts.push(receipt);
    }
    if receipts
        .windows(2)
        .any(|pair| pair[0].node_id >= pair[1].node_id)
        || math_receipts
            .windows(2)
            .any(|pair| pair[0].node_id() >= pair[1].node_id())
    {
        return Err(PrecomposedVectorBindingError::ReceiptMismatch);
    }
    let canonical_jcs = encode_precomposed_vector_binding_set(&epoch, &receipts, &math_receipts);
    Ok(ValidatedPrecomposedVectorBindings {
        profile_progress: profile.progress_token(),
        admission_progress: admitted.progress_token(),
        epoch,
        receipts,
        math_receipts,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn bind_precomposed_vector_resource(
    owner: NodeId,
    kind: PrecomposedVectorKind,
    declaration: &typaxis_document::StagingM4ImageDeclaration,
    attestation: &SafeVectorAdmissionAttestation,
    media_attestation: &typaxis_resource_admission::StagingDeclaredImageAttestation,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
) -> Result<BoundPrecomposedVectorResource, PrecomposedVectorBindingError> {
    let declared_media = match declaration.media {
        ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1) => {
            BoundPrecomposedVectorMedia::SafeSvg1
        }
        ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2) => {
            BoundPrecomposedVectorMedia::SafeSvg2
        }
        ImageMediaDeclaration::Declared(ImageMediaType::Png)
        | ImageMediaDeclaration::LegacyUnspecified => {
            return Err(PrecomposedVectorBindingError::ResourceMismatch(owner));
        }
    };
    let admitted_media = match attestation.media_kind() {
        AdmittedImageMediaKind::SafeVector => BoundPrecomposedVectorMedia::SafeSvg1,
        AdmittedImageMediaKind::SafeVector2 => BoundPrecomposedVectorMedia::SafeSvg2,
        AdmittedImageMediaKind::Png => {
            return Err(PrecomposedVectorBindingError::ResourceMismatch(owner));
        }
    };
    let kind_media_matches = match kind {
        PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::VectorFigure => true,
        PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock => {
            declared_media == BoundPrecomposedVectorMedia::SafeSvg2
        }
    };
    let provenance_matches = match declared_media {
        BoundPrecomposedVectorMedia::SafeSvg1 => declaration.vector_provenance.is_none(),
        BoundPrecomposedVectorMedia::SafeSvg2 => declaration.vector_provenance.is_some(),
    };
    let declared_domain_media = match declared_media {
        BoundPrecomposedVectorMedia::SafeSvg1 => ImageMediaType::SvgSafe1,
        BoundPrecomposedVectorMedia::SafeSvg2 => ImageMediaType::SvgSafe2,
    };
    let media_identity_matches = match declared_media {
        BoundPrecomposedVectorMedia::SafeSvg1 => {
            media_attestation.safe_vector_parser_id().is_none()
                && media_attestation.safe_vector_ir_id().is_none()
        }
        BoundPrecomposedVectorMedia::SafeSvg2 => {
            media_attestation.safe_vector_parser_id() == Some(attestation.parser_id())
                && media_attestation.safe_vector_ir_id() == Some(attestation.ir_id())
        }
    };
    let expected_hash_matches = match declared_media {
        BoundPrecomposedVectorMedia::SafeSvg1 => declaration
            .expected_sha256
            .map_or(true, |value| value == attestation.source_sha256()),
        BoundPrecomposedVectorMedia::SafeSvg2 => {
            declaration.expected_sha256 == Some(attestation.source_sha256())
        }
    };
    if declaration.image_id != attestation.image_id()
        || media_attestation.image_id() != attestation.image_id()
        || media_attestation.declared() != declared_domain_media
        || media_attestation.attested() != attestation.media_kind()
        || media_attestation.content_hash() != attestation.source_sha256()
        || declared_media != admitted_media
        || !kind_media_matches
        || !provenance_matches
        || !expected_hash_matches
        || attestation.parser_id() != attestation.parser_profile().parser_id()
        || attestation.ir_id() != attestation.parser_profile().ir_id()
        || attestation.ir_fingerprint_id() != attestation.parser_profile().ir_fingerprint_id()
        || attestation.limits_fingerprint() != limits.fingerprint()
        || attestation.profile_fingerprint() != profile.profile_receipt_fingerprint()
        || !media_identity_matches
        || media_attestation.safe_vector_ir_fingerprint() != Some(attestation.ir_fingerprint())
        || media_attestation.m4_limits_fingerprint() != Some(limits.fingerprint())
        || media_attestation.m4_profile_fingerprint() != Some(profile.profile_receipt_fingerprint())
    {
        return Err(PrecomposedVectorBindingError::ResourceMismatch(owner));
    }
    Ok(BoundPrecomposedVectorResource {
        image_id: attestation.image_id(),
        declared_media,
        admitted_media,
        source_sha256: attestation.source_sha256(),
        parser_profile: attestation.parser_profile(),
        parser_id: attestation.parser_id(),
        ir_id: attestation.ir_id(),
        ir_fingerprint_id: attestation.ir_fingerprint_id(),
        ir_fingerprint: attestation.ir_fingerprint(),
        intrinsic_width: attestation.intrinsic_width(),
        intrinsic_height: attestation.intrinsic_height(),
        view_box: attestation.view_box(),
        limits_fingerprint: attestation.limits_fingerprint(),
        profile_fingerprint: attestation.profile_fingerprint(),
    })
}

fn bind_precomposed_vector_placement(
    package: &ValidatedStagingSemanticPackage,
    metrics: &typaxis_syntax::ValidatedPrecomposedVectorMetrics,
    resource: &BoundPrecomposedVectorResource,
) -> Result<PrecomposedVectorPlacementInput, PrecomposedVectorBindingError> {
    let owner = metrics.node_id();
    let paint = ResolvedRgb8::BLACK;
    let result = match (metrics.kind(), metrics.payload()) {
        (
            PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::MathVector,
            PrecomposedVectorMetricPayload::Inline {
                metrics: values,
                spacing,
            },
        ) => PrecomposedVectorInlinePlacementInput::from_validated_metrics(
            values,
            spacing,
            resource.intrinsic_width,
            resource.intrinsic_height,
            paint,
        )
        .map(PrecomposedVectorPlacementInput::Inline),
        (
            PrecomposedVectorKind::VectorFigure,
            PrecomposedVectorMetricPayload::Figure { viewport },
        ) => {
            let style = package
                .precomposed_vector_style(owner)
                .ok_or(PrecomposedVectorBindingError::StyleMismatch(owner))?;
            package
                .verify_precomposed_vector_style(style)
                .map_err(|_| PrecomposedVectorBindingError::StyleMismatch(owner))?;
            let style = VectorFigureStyleInput::from_computed(style)
                .map_err(|_| PrecomposedVectorBindingError::StyleMismatch(owner))?;
            VectorFigurePlacementInput::from_validated_viewport(
                viewport,
                resource.intrinsic_width,
                resource.intrinsic_height,
                paint,
                style,
            )
            .map(PrecomposedVectorPlacementInput::VectorFigure)
        }
        (
            PrecomposedVectorKind::MathVectorBlock,
            PrecomposedVectorMetricPayload::MathBlock { metrics: values },
        ) => {
            let style = package
                .precomposed_vector_style(owner)
                .ok_or(PrecomposedVectorBindingError::StyleMismatch(owner))?;
            package
                .verify_precomposed_vector_style(style)
                .map_err(|_| PrecomposedVectorBindingError::StyleMismatch(owner))?;
            let style = MathVectorBlockStyleInput::from_computed(style)
                .map_err(|_| PrecomposedVectorBindingError::StyleMismatch(owner))?;
            MathVectorBlockPlacementInput::from_validated_metrics(
                values,
                resource.intrinsic_width,
                resource.intrinsic_height,
                paint,
                style,
            )
            .map(PrecomposedVectorPlacementInput::MathVectorBlock)
        }
        _ => return Err(PrecomposedVectorBindingError::InvalidMetrics(owner)),
    };
    result.map_err(|error| match error {
        PrecomposedVectorGeometryError::NonUniformScale
        | PrecomposedVectorGeometryError::ScaleOutOfRange => {
            PrecomposedVectorBindingError::InvalidScale(owner)
        }
        PrecomposedVectorGeometryError::ArithmeticOverflow
        | PrecomposedVectorGeometryError::MetricRelation => {
            PrecomposedVectorBindingError::InvalidMetrics(owner)
        }
    })
}

fn placement_matches_kind(
    placement: &PrecomposedVectorPlacementInput,
    kind: PrecomposedVectorKind,
) -> bool {
    matches!(
        (placement, kind),
        (
            PrecomposedVectorPlacementInput::Inline(_),
            PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::MathVector
        ) | (
            PrecomposedVectorPlacementInput::VectorFigure(_),
            PrecomposedVectorKind::VectorFigure
        ) | (
            PrecomposedVectorPlacementInput::MathVectorBlock(_),
            PrecomposedVectorKind::MathVectorBlock
        )
    )
}

fn placement_paint(placement: &PrecomposedVectorPlacementInput) -> ResolvedRgb8 {
    match placement {
        PrecomposedVectorPlacementInput::Inline(value) => value.paint(),
        PrecomposedVectorPlacementInput::VectorFigure(value) => value.paint(),
        PrecomposedVectorPlacementInput::MathVectorBlock(value) => value.paint(),
    }
}

fn encode_precomposed_vector_receipt(value: &ValidatedPrecomposedVectorReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, PRECOMPOSED_VECTOR_BINDING_ALGORITHM);
    output.push_str(",\"alternative_sha256\":");
    push_hash(&mut output, value.alternative_sha256);
    output.push_str(",\"epoch\":");
    push_hash(&mut output, value.epoch_fingerprint);
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, value.kind.as_str());
    output.push_str(",\"language\":");
    match &value.language {
        Some(language) => push_jcs_string(&mut output, language),
        None => output.push_str("null"),
    }
    output.push_str(",\"metrics_fingerprint\":");
    push_hash(&mut output, value.metrics_fingerprint);
    output.push_str(",\"node_id\":");
    output.push_str(&value.node_id.get().to_string());
    output.push_str(",\"owner_source_span\":");
    push_source_span(&mut output, value.owner_source_span);
    output.push_str(",\"placement_input\":");
    push_precomposed_vector_placement(&mut output, &value.placement);
    output.push_str(",\"resource\":");
    push_bound_precomposed_vector_resource(&mut output, &value.resource);
    output.push('}');
    output
}

fn encode_precomposed_vector_binding_set(
    epoch: &PrecomposedVectorLayoutEpoch,
    receipts: &[ValidatedPrecomposedVectorReceipt],
    math_receipts: &[crate::ValidatedMathVectorReceipt],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, PRECOMPOSED_VECTOR_BINDING_SET_ALGORITHM);
    output.push_str(",\"epoch\":");
    push_hash(&mut output, epoch.fingerprint());
    output.push_str(",\"math_receipts\":[");
    for (index, receipt) in math_receipts.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_hash(&mut output, receipt.fingerprint());
    }
    output.push_str("],\"receipts\":[");
    for (index, receipt) in receipts.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_hash(&mut output, receipt.fingerprint());
    }
    output.push_str("]}");
    output
}

fn push_bound_precomposed_vector_resource(
    output: &mut String,
    value: &BoundPrecomposedVectorResource,
) {
    output.push_str("{\"admitted_media\":");
    push_jcs_string(output, value.admitted_media.as_str());
    output.push_str(",\"declared_media\":");
    push_jcs_string(output, value.declared_media.as_str());
    output.push_str(",\"image_id\":");
    output.push_str(&value.image_id.get().to_string());
    output.push_str(",\"intrinsic_height\":");
    output.push_str(&value.intrinsic_height.get().raw().to_string());
    output.push_str(",\"intrinsic_width\":");
    output.push_str(&value.intrinsic_width.get().raw().to_string());
    output.push_str(",\"ir_fingerprint\":");
    push_hash(output, value.ir_fingerprint);
    output.push_str(",\"ir_fingerprint_id\":");
    push_jcs_string(output, value.ir_fingerprint_id);
    output.push_str(",\"ir_id\":");
    push_jcs_string(output, value.ir_id);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(output, value.limits_fingerprint);
    output.push_str(",\"parser_id\":");
    push_jcs_string(output, value.parser_id);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(output, value.profile_fingerprint);
    output.push_str(",\"source_sha256\":");
    push_hash(output, value.source_sha256);
    output.push_str(",\"view_box\":[");
    for (index, coordinate) in value.view_box.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&coordinate.to_string());
    }
    output.push_str("]}");
}

fn push_precomposed_vector_placement(output: &mut String, value: &PrecomposedVectorPlacementInput) {
    output.push('{');
    match value {
        PrecomposedVectorPlacementInput::Inline(value) => {
            output.push_str("\"kind\":\"inline\",\"metrics\":");
            push_bound_vector_metrics(output, value.metrics());
            output.push_str(",\"paint\":");
            push_resolved_rgb8(output, value.paint());
            output.push_str(",\"scale\":");
            output.push_str(&value.scale().get().raw().to_string());
            output.push_str(",\"spacing_after\":");
            output.push_str(&value.spacing_after().get().raw().to_string());
            output.push_str(",\"spacing_before\":");
            output.push_str(&value.spacing_before().get().raw().to_string());
        }
        PrecomposedVectorPlacementInput::VectorFigure(value) => {
            output.push_str("\"kind\":\"vector_figure\",\"paint\":");
            push_resolved_rgb8(output, value.paint());
            output.push_str(",\"scale\":");
            output.push_str(&value.scale().get().raw().to_string());
            output.push_str(",\"style_fingerprint\":");
            push_hash(output, value.style().fingerprint());
            output.push_str(",\"viewport\":{\"height\":");
            output.push_str(&value.viewport_height().get().raw().to_string());
            output.push_str(",\"width\":");
            output.push_str(&value.viewport_width().get().raw().to_string());
            output.push('}');
        }
        PrecomposedVectorPlacementInput::MathVectorBlock(value) => {
            output.push_str("\"kind\":\"math_vector_block\",\"metrics\":");
            push_bound_vector_metrics(output, value.metrics());
            output.push_str(",\"paint\":");
            push_resolved_rgb8(output, value.paint());
            output.push_str(",\"scale\":");
            output.push_str(&value.scale().get().raw().to_string());
            output.push_str(",\"style_fingerprint\":");
            push_hash(output, value.style().fingerprint());
        }
    }
    output.push('}');
}

fn push_bound_vector_metrics(
    output: &mut String,
    value: typaxis_layout_contract::BoundPrecomposedVectorMetrics,
) {
    output.push_str("{\"advance\":");
    output.push_str(&value.advance().get().raw().to_string());
    output.push_str(",\"ascent\":");
    output.push_str(&value.ascent().get().raw().to_string());
    output.push_str(",\"baseline\":");
    output.push_str(&value.baseline().get().raw().to_string());
    output.push_str(",\"descent\":");
    output.push_str(&value.descent().get().raw().to_string());
    output.push_str(",\"origin_x\":");
    output.push_str(&value.origin_x().raw().to_string());
    output.push_str(",\"viewport\":{\"height\":");
    output.push_str(&value.viewport_height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&value.viewport_width().get().raw().to_string());
    output.push_str("},\"viewport_right_from_pen\":");
    output.push_str(&value.viewport_right_from_pen().raw().to_string());
    output.push('}');
}

fn push_resolved_rgb8(output: &mut String, value: ResolvedRgb8) {
    output.push_str("{\"blue\":");
    output.push_str(&value.blue().to_string());
    output.push_str(",\"green\":");
    output.push_str(&value.green().to_string());
    output.push_str(",\"red\":");
    output.push_str(&value.red().to_string());
    output.push('}');
}

fn push_source_span(output: &mut String, value: SourceSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&value.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&value.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&value.start_byte().get().to_string());
    output.push('}');
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPlacement {
    occurrence: u32,
    owner: NodeId,
    image_id: ImageResourceId,
    placement: StagingM4FigurePlacement,
    alternative: String,
    source_span: SourceSpan,
    page_index: u32,
    frame_index: u32,
    bounds: Rect,
    scale: i32,
    admitted_sha256: [u8; 32],
    ir_fingerprint: [u8; 32],
    fingerprint: [u8; 32],
}

impl StagingSafeVectorPlacement {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn placement(&self) -> StagingM4FigurePlacement {
        self.placement
    }
    pub fn alternative(&self) -> &str {
        &self.alternative
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn frame_index(&self) -> u32 {
        self.frame_index
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
    pub const fn scale_raw(&self) -> i32 {
        self.scale
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn ir_fingerprint(&self) -> [u8; 32] {
        self.ir_fingerprint
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorSelectedLayoutReceipt {
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    page_geometry_fingerprint: [u8; 32],
    placement_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorSelectedLayoutReceipt {
    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package_fingerprint
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub const fn admitted_fingerprint(&self) -> [u8; 32] {
        self.admitted_fingerprint
    }
    pub const fn page_geometry_fingerprint(&self) -> [u8; 32] {
        self.page_geometry_fingerprint
    }
    pub const fn placement_count(&self) -> u32 {
        self.placement_count
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorSelectedLayout {
    placements: Vec<StagingSafeVectorPlacement>,
    page_geometry: StagingM4PageGeometry,
    receipt: StagingSafeVectorSelectedLayoutReceipt,
}

impl StagingSafeVectorSelectedLayout {
    pub fn placements(&self) -> &[StagingSafeVectorPlacement] {
        &self.placements
    }
    pub const fn receipt(&self) -> &StagingSafeVectorSelectedLayoutReceipt {
        &self.receipt
    }
    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        &self.page_geometry
    }

    pub fn verify_downstream(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingSafeVectorProfileView,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSafeVectorLayoutError> {
        let expected_profile = StagingSafeVectorProfileView::new(package, limits)
            .map_err(|_| StagingSafeVectorLayoutError::ReceiptMismatch)?;
        let canonical = encode_layout(
            package.semantic_fingerprint(),
            profile.profile_fingerprint(),
            limits.fingerprint(),
            self.receipt.admitted_fingerprint,
            &self.placements,
            &self.page_geometry,
        );
        if *profile != expected_profile
            || self.page_geometry != *profile.page_geometry()
            || self.receipt.package_fingerprint != package.semantic_fingerprint()
            || self.receipt.profile_fingerprint != profile.profile_fingerprint()
            || self.receipt.limits_fingerprint != limits.fingerprint()
            || self.receipt.page_geometry_fingerprint != self.page_geometry.fingerprint()
            || usize::try_from(self.receipt.placement_count) != Ok(self.placements.len())
            || self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
            || self.placements.len() as u64 > limits.base().get().max_fragments
            || !placements_match_package(&self.placements, package, profile)
            || self
                .placements
                .iter()
                .enumerate()
                .any(|(index, placement)| {
                    usize::try_from(placement.occurrence) != Ok(index)
                        || placement.scale <= 0
                        || i64::from(placement.scale) > FIXED_ONE
                        || sha256(encode_placement(placement).as_bytes()) != placement.fingerprint
                })
            || !placements_are_closed(&self.placements, &self.page_geometry, limits)
        {
            return Err(StagingSafeVectorLayoutError::ReceiptMismatch);
        }
        Ok(())
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingSafeVectorProfileView,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
    ) -> Result<(), StagingSafeVectorLayoutError> {
        self.verify_downstream(package, profile, limits)?;
        if self.receipt.admitted_fingerprint != admitted.fingerprint().bytes() {
            return Err(StagingSafeVectorLayoutError::ReceiptMismatch);
        }
        for (index, placement) in self.placements.iter().enumerate() {
            let image = admitted.image(placement.image_id).ok_or(
                StagingSafeVectorLayoutError::MissingAdmittedVector(placement.image_id),
            )?;
            let vector = image
                .safe_vector()
                .ok_or(StagingSafeVectorLayoutError::WrongMedia(placement.image_id))?;
            if usize::try_from(placement.occurrence) != Ok(index)
                || image.media_kind() != AdmittedImageMediaKind::SafeVector
                || image.content_hash() != placement.admitted_sha256
                || vector.fingerprint() != placement.ir_fingerprint
                || image.m4_limits_fingerprint() != Some(limits.fingerprint())
                || image.m4_profile_fingerprint() != Some(profile.profile_fingerprint())
                || scale_to_fit(
                    vector.intrinsic_width().get().raw(),
                    self.page_geometry.body().width().get().raw(),
                )? != Some(placement.scale)
                || scaled_dimension(vector.intrinsic_width().get().raw(), placement.scale)?
                    != placement.bounds.width().get().raw()
                || scaled_dimension(vector.intrinsic_height().get().raw(), placement.scale)?
                    != placement.bounds.height().get().raw()
                || sha256(encode_placement(placement).as_bytes()) != placement.fingerprint
            {
                return Err(StagingSafeVectorLayoutError::ReceiptMismatch);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorLayoutError {
    ProfileMismatch,
    MissingAdmittedVector(ImageResourceId),
    WrongMedia(ImageResourceId),
    IntrinsicGeometry(ImageResourceId),
    PlacementLimit,
    PageLimit,
    Oversize(NodeId),
    ArithmeticOverflow,
    ReceiptMismatch,
    AllocationFailure,
    PrecomposedVectorStaging(NodeId),
}

impl std::fmt::Display for StagingSafeVectorLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => formatter.write_str("I9190: SafeVector profile mismatch"),
            Self::MissingAdmittedVector(id) => {
                write!(formatter, "R7100: missing admitted vector {}", id.get())
            }
            Self::WrongMedia(id) => {
                write!(formatter, "R7100: image {} is not SafeVector", id.get())
            }
            Self::IntrinsicGeometry(id) => write!(
                formatter,
                "L5100: invalid vector intrinsic geometry {}",
                id.get()
            ),
            Self::PlacementLimit => {
                formatter.write_str("L5110: SafeVector placement limit exceeded")
            }
            Self::PageLimit => formatter.write_str("L5100: SafeVector page limit exceeded"),
            Self::Oversize(owner) => write!(
                formatter,
                "L5100: vector Figure {} exceeds an empty frame",
                owner.get()
            ),
            Self::ArithmeticOverflow => {
                formatter.write_str("L5100: SafeVector layout arithmetic overflow")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: SafeVector layout receipt mismatch")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5100: SafeVector layout allocation failed")
            }
            Self::PrecomposedVectorStaging(owner) => write!(
                formatter,
                "P1102: precomposed vector at node {} requires SafeVector /2 layout",
                owner.get()
            ),
        }
    }
}

impl std::error::Error for StagingSafeVectorLayoutError {}

pub fn layout_staging_safe_vectors(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSafeVectorProfileView,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
) -> Result<StagingSafeVectorSelectedLayout, StagingSafeVectorLayoutError> {
    if *profile
        != StagingSafeVectorProfileView::new(package, limits)
            .map_err(|_| StagingSafeVectorLayoutError::ProfileMismatch)?
        || !admitted.matches_declarations(
            typaxis_resource_admission::staging_declared_base_catalog(package.resources())
                .map_err(|_| StagingSafeVectorLayoutError::ProfileMismatch)?
                .resource_catalog(),
        )
        || !admitted_matches_profile(package, profile, limits, admitted)
    {
        return Err(StagingSafeVectorLayoutError::ProfileMismatch);
    }
    let mut figures = Vec::new();
    let vector_ids: BTreeSet<_> = profile.vector_resource_ids().iter().copied().collect();
    collect_figures(&package.document().blocks, &vector_ids, &mut figures)?;
    for footnote in &package.document().footnotes {
        collect_figures(&footnote.blocks, &vector_ids, &mut figures)?;
    }
    if figures.len() as u64 > limits.base().get().max_fragments {
        return Err(StagingSafeVectorLayoutError::PlacementLimit);
    }
    let mut placements = Vec::new();
    placements
        .try_reserve_exact(figures.len())
        .map_err(|_| StagingSafeVectorLayoutError::AllocationFailure)?;
    let mut page_index = 0u32;
    let mut cursor = 0i64;
    let page_geometry = profile.page_geometry().clone();
    let body = page_geometry.body();
    for (index, figure) in figures.into_iter().enumerate() {
        let image = admitted.image(figure.image_id).ok_or(
            StagingSafeVectorLayoutError::MissingAdmittedVector(figure.image_id),
        )?;
        let ir = image
            .safe_vector()
            .ok_or(StagingSafeVectorLayoutError::WrongMedia(figure.image_id))?;
        if image.m4_limits_fingerprint() != Some(limits.fingerprint())
            || image.m4_profile_fingerprint() != Some(profile.profile_fingerprint())
        {
            return Err(StagingSafeVectorLayoutError::ProfileMismatch);
        }
        let scale = scale_to_fit(ir.intrinsic_width().get().raw(), body.width().get().raw())?
            .ok_or(StagingSafeVectorLayoutError::Oversize(figure.owner))?;
        let width_raw = scaled_dimension(ir.intrinsic_width().get().raw(), scale)?;
        let height_raw = scaled_dimension(ir.intrinsic_height().get().raw(), scale)?;
        let width = PositiveLength::new(
            Length::from_raw(width_raw).ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
        )
        .ok_or(StagingSafeVectorLayoutError::IntrinsicGeometry(
            figure.image_id,
        ))?;
        let height = PositiveLength::new(
            Length::from_raw(height_raw).ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
        )
        .ok_or(StagingSafeVectorLayoutError::IntrinsicGeometry(
            figure.image_id,
        ))?;
        if height.get().raw() > body.height().get().raw() {
            return Err(StagingSafeVectorLayoutError::Oversize(figure.owner));
        }
        if cursor
            .checked_add(height.get().raw())
            .map_or(true, |end| end > body.height().get().raw())
        {
            page_index = page_index
                .checked_add(1)
                .ok_or(StagingSafeVectorLayoutError::PageLimit)?;
            cursor = 0;
        }
        if page_index >= limits.base().get().max_pages {
            return Err(StagingSafeVectorLayoutError::PageLimit);
        }
        let x_raw = match figure.placement {
            StagingM4FigurePlacement::Block => body.x().raw(),
            StagingM4FigurePlacement::Float => body
                .x()
                .raw()
                .checked_add(
                    body.width()
                        .get()
                        .raw()
                        .checked_sub(width.get().raw())
                        .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
                )
                .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
        };
        let y_raw = body
            .y()
            .raw()
            .checked_add(cursor)
            .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?;
        let mut placement = StagingSafeVectorPlacement {
            occurrence: u32::try_from(index)
                .map_err(|_| StagingSafeVectorLayoutError::PlacementLimit)?,
            owner: figure.owner,
            image_id: figure.image_id,
            placement: figure.placement,
            alternative: figure.alternative.to_owned(),
            source_span: figure.span,
            page_index,
            frame_index: 0,
            bounds: Rect::new(
                Length::from_raw(x_raw).ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
                Length::from_raw(y_raw).ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
                width,
                height,
            ),
            scale,
            admitted_sha256: image.content_hash(),
            ir_fingerprint: ir.fingerprint(),
            fingerprint: [0; 32],
        };
        placement.fingerprint = sha256(encode_placement(&placement).as_bytes());
        cursor = cursor
            .checked_add(height.get().raw())
            .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?;
        placements.push(placement);
    }
    let canonical_jcs = encode_layout(
        package.semantic_fingerprint(),
        profile.profile_fingerprint(),
        limits.fingerprint(),
        admitted.fingerprint().bytes(),
        &placements,
        &page_geometry,
    );
    let selected = StagingSafeVectorSelectedLayout {
        receipt: StagingSafeVectorSelectedLayoutReceipt {
            package_fingerprint: package.semantic_fingerprint(),
            profile_fingerprint: profile.profile_fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            admitted_fingerprint: admitted.fingerprint().bytes(),
            page_geometry_fingerprint: page_geometry.fingerprint(),
            placement_count: u32::try_from(placements.len())
                .map_err(|_| StagingSafeVectorLayoutError::PlacementLimit)?,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        placements,
        page_geometry,
    };
    selected.verify(package, profile, limits, admitted)?;
    Ok(selected)
}

struct FigureRef<'a> {
    owner: NodeId,
    image_id: ImageResourceId,
    placement: StagingM4FigurePlacement,
    alternative: &'a str,
    span: SourceSpan,
}

fn collect_figures<'a>(
    blocks: &'a [StagingM4Block],
    vector_ids: &BTreeSet<ImageResourceId>,
    output: &mut Vec<FigureRef<'a>>,
) -> Result<(), StagingSafeVectorLayoutError> {
    for block in blocks {
        match block {
            StagingM4Block::Figure {
                common,
                image_id,
                placement,
                alternative,
                caption,
                ..
            } => {
                if vector_ids.contains(image_id) {
                    output.push(FigureRef {
                        owner: common.node_id,
                        image_id: *image_id,
                        placement: *placement,
                        alternative,
                        span: common.span,
                    });
                }
                collect_figures(caption, vector_ids, output)?;
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    collect_figures(&item.blocks, vector_ids, output)?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_figures(&cell.blocks, vector_ids, output)?;
                }
            }
            StagingM4Block::SemanticContainer { blocks, .. } => {
                collect_figures(blocks, vector_ids, output)?;
            }
            StagingM4Block::VectorFigure { common, .. }
            | StagingM4Block::MathVectorBlock { common, .. } => {
                return Err(StagingSafeVectorLayoutError::PrecomposedVectorStaging(
                    common.node_id,
                ));
            }
            StagingM4Block::Paragraph { inline_vectors, .. }
            | StagingM4Block::Heading { inline_vectors, .. } => {
                if let Some(vector) = inline_vectors.first() {
                    return Err(StagingSafeVectorLayoutError::PrecomposedVectorStaging(
                        vector.node_id,
                    ));
                }
            }
            StagingM4Block::PageBreak { .. } | StagingM4Block::DisplayMath { .. } => {}
        }
    }
    Ok(())
}

fn admitted_matches_profile(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSafeVectorProfileView,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
) -> bool {
    package.resources().images.iter().all(|declaration| {
        let Some(image) = admitted.image(declaration.image_id) else {
            return false;
        };
        match declaration.media {
            ImageMediaDeclaration::Declared(ImageMediaType::Png) => {
                image.media_kind() == AdmittedImageMediaKind::Png
                    && image.safe_vector().is_none()
                    && image.m4_limits_fingerprint().is_none()
                    && image.m4_profile_fingerprint().is_none()
            }
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1) => {
                image.media_kind() == AdmittedImageMediaKind::SafeVector
                    && image.safe_vector().is_some()
                    && image.m4_limits_fingerprint() == Some(limits.fingerprint())
                    && image.m4_profile_fingerprint() == Some(profile.profile_fingerprint())
            }
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2) => false,
            ImageMediaDeclaration::LegacyUnspecified => false,
        }
    })
}

fn placements_match_package(
    placements: &[StagingSafeVectorPlacement],
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSafeVectorProfileView,
) -> bool {
    fn visit(
        blocks: &[StagingM4Block],
        vector_ids: &[ImageResourceId],
        figure_owners: &[NodeId],
        placements: &[StagingSafeVectorPlacement],
        next: &mut usize,
    ) -> bool {
        for block in blocks {
            match block {
                StagingM4Block::Figure {
                    common,
                    image_id,
                    placement,
                    alternative,
                    caption,
                    ..
                } => {
                    if vector_ids.binary_search(image_id).is_ok() {
                        let Some(expected) = placements.get(*next) else {
                            return false;
                        };
                        if figure_owners.get(*next) != Some(&common.node_id)
                            || usize::try_from(expected.occurrence) != Ok(*next)
                            || expected.owner != common.node_id
                            || expected.image_id != *image_id
                            || expected.placement != *placement
                            || expected.alternative != *alternative
                            || expected.source_span != common.span
                        {
                            return false;
                        }
                        *next += 1;
                    }
                    if !visit(caption, vector_ids, figure_owners, placements, next) {
                        return false;
                    }
                }
                StagingM4Block::List { items, .. } => {
                    for item in items {
                        if !visit(&item.blocks, vector_ids, figure_owners, placements, next) {
                            return false;
                        }
                    }
                }
                StagingM4Block::Table { head, body, .. } => {
                    for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                        if !visit(&cell.blocks, vector_ids, figure_owners, placements, next) {
                            return false;
                        }
                    }
                }
                StagingM4Block::SemanticContainer { blocks, .. }
                    if !visit(blocks, vector_ids, figure_owners, placements, next) =>
                {
                    return false;
                }
                StagingM4Block::SemanticContainer { .. } => {}
                StagingM4Block::VectorFigure { .. } | StagingM4Block::MathVectorBlock { .. } => {
                    return false
                }
                StagingM4Block::Paragraph { inline_vectors, .. }
                | StagingM4Block::Heading { inline_vectors, .. }
                    if !inline_vectors.is_empty() =>
                {
                    return false;
                }
                StagingM4Block::Paragraph { .. }
                | StagingM4Block::Heading { .. }
                | StagingM4Block::PageBreak { .. }
                | StagingM4Block::DisplayMath { .. } => {}
            }
        }
        true
    }

    let mut next = 0usize;
    if !visit(
        &package.document().blocks,
        profile.vector_resource_ids(),
        profile.figure_owners(),
        placements,
        &mut next,
    ) {
        return false;
    }
    for footnote in &package.document().footnotes {
        if !visit(
            &footnote.blocks,
            profile.vector_resource_ids(),
            profile.figure_owners(),
            placements,
            &mut next,
        ) {
            return false;
        }
    }
    next == placements.len() && next == profile.figure_owners().len()
}

fn round_ratio(numerator: i128, denominator: i128) -> Result<i64, StagingSafeVectorLayoutError> {
    if denominator <= 0 {
        return Err(StagingSafeVectorLayoutError::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let twice = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?;
    let denominator_unsigned = denominator as u128;
    let rounded =
        if twice < denominator_unsigned || (twice == denominator_unsigned && quotient % 2 == 0) {
            quotient
        } else {
            quotient
                .checked_add(if remainder >= 0 { 1 } else { -1 })
                .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?
        };
    i64::try_from(rounded).map_err(|_| StagingSafeVectorLayoutError::ArithmeticOverflow)
}

fn scale_to_fit(
    intrinsic_width: i64,
    available_width: i64,
) -> Result<Option<i32>, StagingSafeVectorLayoutError> {
    if intrinsic_width <= 0 || available_width <= 0 {
        return Err(StagingSafeVectorLayoutError::ArithmeticOverflow);
    }
    if intrinsic_width <= available_width {
        return Ok(Some(FIXED_ONE as i32));
    }
    let candidate = i128::from(available_width)
        .checked_mul(i128::from(FIXED_ONE))
        .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?
        / i128::from(intrinsic_width);
    let candidate =
        i32::try_from(candidate).map_err(|_| StagingSafeVectorLayoutError::ArithmeticOverflow)?;
    Ok((candidate > 0).then_some(candidate))
}

fn scaled_dimension(intrinsic: i64, scale: i32) -> Result<i64, StagingSafeVectorLayoutError> {
    round_ratio(
        i128::from(intrinsic)
            .checked_mul(i128::from(scale))
            .ok_or(StagingSafeVectorLayoutError::ArithmeticOverflow)?,
        i128::from(FIXED_ONE),
    )
}

fn placements_are_closed(
    placements: &[StagingSafeVectorPlacement],
    page_geometry: &StagingM4PageGeometry,
    limits: &M4EffectiveResourceLimits,
) -> bool {
    let body = page_geometry.body();
    let mut expected_page = 0u32;
    let mut cursor = 0i64;
    for placement in placements {
        let height = placement.bounds.height().get().raw();
        if height > body.height().get().raw() {
            return false;
        }
        if cursor
            .checked_add(height)
            .map_or(true, |end| end > body.height().get().raw())
        {
            let Some(next_page) = expected_page.checked_add(1) else {
                return false;
            };
            expected_page = next_page;
            cursor = 0;
        }
        let expected_x = match placement.placement {
            StagingM4FigurePlacement::Block => body.x().raw(),
            StagingM4FigurePlacement::Float => {
                let Some(remaining) = body
                    .width()
                    .get()
                    .raw()
                    .checked_sub(placement.bounds.width().get().raw())
                else {
                    return false;
                };
                let Some(x) = body.x().raw().checked_add(remaining) else {
                    return false;
                };
                x
            }
        };
        let Some(expected_y) = body.y().raw().checked_add(cursor) else {
            return false;
        };
        if placement.page_index != expected_page
            || placement.frame_index != 0
            || placement.bounds.x().raw() != expected_x
            || placement.bounds.y().raw() != expected_y
            || placement.bounds.width().get().raw() > body.width().get().raw()
            || placement.page_index >= limits.base().get().max_pages
        {
            return false;
        }
        let Some(next_cursor) = cursor.checked_add(height) else {
            return false;
        };
        cursor = next_cursor;
    }
    true
}

fn encode_layout(
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    placements: &[StagingSafeVectorPlacement],
    page_geometry: &StagingM4PageGeometry,
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted_fingerprint);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_SAFE_VECTOR_SELECTED_LAYOUT_ALGORITHM);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    output.push_str(",\"package_fingerprint\":");
    push_hash(&mut output, package_fingerprint);
    output.push_str(",\"page_geometry\":");
    output.push_str(page_geometry.canonical_jcs());
    output.push_str(",\"placements\":[");
    for (index, placement) in placements.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&encode_placement(placement));
    }
    output.push_str("],\"profile_fingerprint\":");
    push_hash(&mut output, profile_fingerprint);
    output.push('}');
    output
}

fn encode_placement(value: &StagingSafeVectorPlacement) -> String {
    let mut output = String::from("{\"admitted_sha256\":");
    push_hash(&mut output, value.admitted_sha256);
    output.push_str(",\"alternative_sha256\":");
    push_hash(&mut output, sha256(value.alternative.as_bytes()));
    output.push_str(",\"bounds\":{");
    output.push_str("\"height\":");
    output.push_str(&value.bounds.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&value.bounds.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&value.bounds.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&value.bounds.y().raw().to_string());
    output.push_str("},\"frame_index\":");
    output.push_str(&value.frame_index.to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&value.image_id.get().to_string());
    output.push_str(",\"ir_fingerprint\":");
    push_hash(&mut output, value.ir_fingerprint);
    output.push_str(",\"occurrence\":");
    output.push_str(&value.occurrence.to_string());
    output.push_str(",\"owner\":");
    output.push_str(&value.owner.get().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&value.page_index.to_string());
    output.push_str(",\"placement\":");
    push_jcs_string(&mut output, value.placement.as_str());
    output.push_str(",\"scale\":");
    output.push_str(&value.scale.to_string());
    output.push_str(",\"source_span\":{");
    output.push_str("\"end_byte\":");
    output.push_str(&value.source_span.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&value.source_span.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&value.source_span.start_byte().get().to_string());
    output.push_str("}}");
    output
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

#[cfg(any(test, feature = "staging-fixtures"))]
pub(crate) struct StagingPrecomposedVectorBindingFixture {
    pub package: ValidatedStagingSemanticPackage,
    pub profile: StagingPrecomposedVectorProfileAuthorization,
    pub limits: M4EffectiveResourceLimits,
    pub admitted: AdmittedResourceLedger,
    pub bindings: ValidatedPrecomposedVectorBindings,
}

#[cfg(test)]
pub(crate) fn staging_precomposed_vector_binding_fixture(
) -> Result<StagingPrecomposedVectorBindingFixture, Box<dyn std::error::Error>> {
    staging_precomposed_vector_binding_fixture_with_media(
        false,
        None,
        false,
        crate::StagingPrecomposedVectorBlockFixtureCase::Default,
    )
}

#[cfg(test)]
pub(crate) fn staging_precomposed_vector_binding_fixture_with_fragment_limit(
    max_fragments: u64,
) -> Result<StagingPrecomposedVectorBindingFixture, Box<dyn std::error::Error>> {
    staging_precomposed_vector_binding_fixture_with_media(
        false,
        Some(max_fragments),
        false,
        crate::StagingPrecomposedVectorBlockFixtureCase::Default,
    )
}

#[cfg(test)]
fn staging_precomposed_vector_binding_fixture_with_generic_safe_svg1(
) -> Result<StagingPrecomposedVectorBindingFixture, Box<dyn std::error::Error>> {
    staging_precomposed_vector_binding_fixture_with_media(
        true,
        None,
        false,
        crate::StagingPrecomposedVectorBlockFixtureCase::Default,
    )
}

#[cfg(test)]
pub(crate) fn staging_precomposed_vector_binding_fixture_with_equation_font(
) -> Result<StagingPrecomposedVectorBindingFixture, Box<dyn std::error::Error>> {
    staging_precomposed_vector_binding_fixture_with_media(
        false,
        None,
        true,
        crate::StagingPrecomposedVectorBlockFixtureCase::Default,
    )
}

#[cfg(test)]
pub(crate) fn staging_precomposed_vector_binding_fixture_with_mixed_native_math(
) -> Result<StagingPrecomposedVectorBindingFixture, Box<dyn std::error::Error>> {
    staging_precomposed_vector_binding_fixture_with_media(
        false,
        None,
        true,
        crate::StagingPrecomposedVectorBlockFixtureCase::MixedNativeMath,
    )
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub(crate) fn staging_precomposed_vector_binding_fixture_for_block_case(
    case: crate::StagingPrecomposedVectorBlockFixtureCase,
) -> Result<StagingPrecomposedVectorBindingFixture, Box<dyn std::error::Error>> {
    staging_precomposed_vector_binding_fixture_with_media(false, None, true, case)
}

#[cfg(any(test, feature = "staging-fixtures"))]
fn staging_precomposed_vector_binding_fixture_with_media(
    generic_safe_svg1: bool,
    max_fragments: Option<u64>,
    equation_font: bool,
    block_case: crate::StagingPrecomposedVectorBlockFixtureCase,
) -> Result<StagingPrecomposedVectorBindingFixture, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::PathBuf;
    use typaxis_core::{
        ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, HostAdmissionContext, HostPath,
        M4ResourceLimits, PdfStreamCompression, ResourceLimits, ValidatedResourceLimits,
        DEFAULT_ALLOWED_URI_SCHEMES,
    };
    use typaxis_resource_admission::{
        staging_declared_base_catalog, AdmittedResourceResolver, HostResourceAdmissionSession,
    };
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder, WireFontMediaType, WireImageMediaType,
        WireStagingM4Block, WireStagingM4FontFace, WireStagingM4Inline, WireStagingMathSource,
        WireStagingSourceSpan, WireStagingStyleDeclaration, WireStagingStyleRule,
        WireStagingStyleValue, WireStagingTextSpan,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    let job = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../samples/machine-package/staging/production-book-1/precomposed-vector");
    let package_path = job.join("document-package.json");
    let mut resource_limits = ResourceLimits::default();
    if let Some(max_fragments) = max_fragments {
        resource_limits.max_fragments = max_fragments;
    }
    let base_limits = ValidatedResourceLimits::new(resource_limits)?;
    let limits = M4EffectiveResourceLimits::new(base_limits.clone(), M4ResourceLimits::default())?;
    let decoded = StagingSemanticDocumentPackageDecoder::new().decode(
        &fs::read(&package_path)?,
        &DocumentPackageDecodePolicy::new(&base_limits),
    )?;
    let mut wire = decoded.into_wire();
    let mut document = wire.document().clone();
    let mut resources = wire.resources().clone();
    let WireStagingM4Block::SemanticContainer { blocks, .. } = &mut document.blocks[0] else {
        panic!("precomposed-vector fixture root is not a semantic container");
    };
    if generic_safe_svg1 {
        let mut safe_svg1 = resources.images[0].clone();
        safe_svg1.image_id = 1;
        safe_svg1.uri = "svg/ordered-pair.svg".to_owned();
        safe_svg1.expected_sha256 =
            Some("1d2a24ba4c7ecf28e586e988ffef2c079bded1397cf9994b89c5aa4cd5b8f7b3".to_owned());
        safe_svg1.media_type = WireImageMediaType::SvgSafe1;
        safe_svg1.vector_provenance = None;
        resources.images.push(safe_svg1);

        let WireStagingM4Block::Paragraph { children, .. } = &mut blocks[0] else {
            panic!("precomposed-vector fixture first child is not a paragraph");
        };
        let WireStagingM4Inline::InlineVector { image_id, .. } = &mut children[0] else {
            panic!("precomposed-vector fixture first inline is not a vector");
        };
        *image_id = 1;

        let WireStagingM4Block::VectorFigure {
            image_id, viewport, ..
        } = &mut blocks[1]
        else {
            panic!("precomposed-vector fixture second block is not a vector Figure");
        };
        *image_id = 1;
        viewport.width = 1_835_008;
    } else {
        // The syntax-only fixture predates intrinsic-resource scale proof and
        // gives this shared 30pt resource a 28pt generic-inline viewport.
        // Make only the positive binding fixture uniform; the scale test below
        // owns the explicit nonuniform negative case.
        let WireStagingM4Block::Paragraph { children, .. } = &mut blocks[0] else {
            panic!("precomposed-vector fixture first child is not a paragraph");
        };
        let WireStagingM4Inline::InlineVector { metrics, .. } = &mut children[0] else {
            panic!("precomposed-vector fixture first inline is not a vector");
        };
        metrics.viewport.width = 1_966_080;
    }
    let mut style_sheet = wire.style_sheet().clone();
    if equation_font {
        resources.font_faces.push(WireStagingM4FontFace {
            font_face_id: 0,
            family: "Math".to_owned(),
            uri: "math.ttf".to_owned(),
            face_index: 0,
            expected_sha256: Some(
                "dc3862c12ad95f75d7c21cb3c37487e220182aa5088c537c634c194ee83ee894".to_owned(),
            ),
            media_type: WireFontMediaType::SfntTrueTypeGlyf,
        });
        style_sheet.rules.push(WireStagingStyleRule {
            style_id: "equation-number-text".to_owned(),
            extends: None,
            selector: "semantic_container".to_owned(),
            source_order: u32::try_from(style_sheet.rules.len())?,
            declarations: vec![
                WireStagingStyleDeclaration {
                    name: "font_family".to_owned(),
                    value: WireStagingStyleValue::FontFamilyList {
                        families: vec!["Math".to_owned()],
                    },
                    important: false,
                },
                WireStagingStyleDeclaration {
                    name: "font_size".to_owned(),
                    value: WireStagingStyleValue::Length { value: 786_432 },
                    important: false,
                },
                WireStagingStyleDeclaration {
                    name: "line_height".to_owned(),
                    value: WireStagingStyleValue::Length { value: 917_504 },
                    important: false,
                },
            ],
        });
    }
    let mut add_block_rule = |style_id: String,
                              selector: &str,
                              declarations: Vec<WireStagingStyleDeclaration>|
     -> Result<(), Box<dyn std::error::Error>> {
        let source_order = u32::try_from(style_sheet.rules.len())?;
        style_sheet.rules.push(WireStagingStyleRule {
            style_id,
            extends: None,
            selector: selector.to_owned(),
            source_order,
            declarations,
        });
        Ok(())
    };
    let alignment = match block_case {
        crate::StagingPrecomposedVectorBlockFixtureCase::AlignmentStart => Some("start"),
        crate::StagingPrecomposedVectorBlockFixtureCase::AlignmentCenter => Some("center"),
        crate::StagingPrecomposedVectorBlockFixtureCase::AlignmentEnd => Some("end"),
        _ => None,
    };
    if let Some(alignment) = alignment {
        let WireStagingM4Block::MathVectorBlock {
            equation_number, ..
        } = &mut blocks[2]
        else {
            panic!("precomposed-vector fixture third child is not block math");
        };
        *equation_number = None;
        for selector in ["vector_figure", "math_vector_block"] {
            add_block_rule(
                format!("block-alignment-{selector}-{alignment}"),
                selector,
                vec![
                    WireStagingStyleDeclaration {
                        name: "end_indent".to_owned(),
                        value: WireStagingStyleValue::Length { value: 1_310_720 },
                        important: false,
                    },
                    WireStagingStyleDeclaration {
                        name: "space_after".to_owned(),
                        value: WireStagingStyleValue::Length { value: 196_608 },
                        important: false,
                    },
                    WireStagingStyleDeclaration {
                        name: "space_before".to_owned(),
                        value: WireStagingStyleValue::Length { value: 131_072 },
                        important: false,
                    },
                    WireStagingStyleDeclaration {
                        name: "start_indent".to_owned(),
                        value: WireStagingStyleValue::Length { value: 655_360 },
                        important: false,
                    },
                    WireStagingStyleDeclaration {
                        name: "text_align".to_owned(),
                        value: WireStagingStyleValue::Keyword {
                            value: alignment.to_owned(),
                        },
                        important: false,
                    },
                ],
            )?;
        }
    }
    if block_case == crate::StagingPrecomposedVectorBlockFixtureCase::NumberShort {
        add_block_rule(
            "short-equation-number".to_owned(),
            "math_vector_block",
            vec![
                WireStagingStyleDeclaration {
                    name: "font_size".to_owned(),
                    value: WireStagingStyleValue::Length { value: 393_216 },
                    important: false,
                },
                WireStagingStyleDeclaration {
                    name: "line_height".to_owned(),
                    value: WireStagingStyleValue::Length { value: 524_288 },
                    important: false,
                },
            ],
        )?;
    }
    if block_case == crate::StagingPrecomposedVectorBlockFixtureCase::NumberCollision {
        add_block_rule(
            "colliding-equation-number".to_owned(),
            "math_vector_block",
            vec![WireStagingStyleDeclaration {
                name: "text_align".to_owned(),
                value: WireStagingStyleValue::Keyword {
                    value: "end".to_owned(),
                },
                important: false,
            }],
        )?;
    }
    if block_case == crate::StagingPrecomposedVectorBlockFixtureCase::NarrowInnerFrame {
        add_block_rule(
            "narrow-vector-inner-frame".to_owned(),
            "math_vector_block",
            vec![
                WireStagingStyleDeclaration {
                    name: "end_indent".to_owned(),
                    value: WireStagingStyleValue::Length { value: 5_898_240 },
                    important: false,
                },
                WireStagingStyleDeclaration {
                    name: "start_indent".to_owned(),
                    value: WireStagingStyleValue::Length { value: 5_898_240 },
                    important: false,
                },
            ],
        )?;
    }
    if matches!(
        block_case,
        crate::StagingPrecomposedVectorBlockFixtureCase::FigureCaption
            | crate::StagingPrecomposedVectorBlockFixtureCase::FigureCaptionSplit
    ) {
        let WireStagingM4Block::VectorFigure { caption, .. } = &mut blocks[1] else {
            panic!("precomposed-vector fixture second block is not a vector Figure");
        };
        caption.push(WireStagingM4Block::Paragraph {
            node_id: 6,
            span: WireStagingSourceSpan {
                source_id: 0,
                start_byte: 7,
                end_byte: 7,
            },
            classes: Vec::new(),
            children: Vec::new(),
            language: None,
        });
        let WireStagingM4Block::MathVectorBlock {
            node_id,
            equation_number,
            ..
        } = &mut blocks[2]
        else {
            panic!("precomposed-vector fixture third child is not block math");
        };
        *node_id = 7;
        equation_number
            .as_mut()
            .expect("fixture block math has an equation number")
            .node_id = 8;
    }
    if block_case == crate::StagingPrecomposedVectorBlockFixtureCase::FigureCaptionSplit {
        add_block_rule(
            "vector-figure-splittable-caption".to_owned(),
            "vector_figure",
            vec![WireStagingStyleDeclaration {
                name: "keep_caption".to_owned(),
                value: WireStagingStyleValue::Boolean { value: false },
                important: false,
            }],
        )?;
    }
    if block_case == crate::StagingPrecomposedVectorBlockFixtureCase::KeepWithNext {
        add_block_rule(
            "vector-figure-keep-with-next".to_owned(),
            "vector_figure",
            vec![WireStagingStyleDeclaration {
                name: "keep_with_next".to_owned(),
                value: WireStagingStyleValue::Boolean { value: true },
                important: false,
            }],
        )?;
    }
    if block_case == crate::StagingPrecomposedVectorBlockFixtureCase::ForcedPageBreak {
        let WireStagingM4Block::MathVectorBlock {
            node_id,
            equation_number,
            ..
        } = &mut blocks[2]
        else {
            panic!("precomposed-vector fixture third child is not block math");
        };
        *node_id = 7;
        equation_number
            .as_mut()
            .expect("fixture block math has an equation number")
            .node_id = 8;
        blocks.insert(
            2,
            WireStagingM4Block::PageBreak {
                node_id: 6,
                span: WireStagingSourceSpan {
                    source_id: 0,
                    start_byte: 7,
                    end_byte: 7,
                },
                classes: Vec::new(),
            },
        );
    }
    if block_case == crate::StagingPrecomposedVectorBlockFixtureCase::NamedPage {
        add_block_rule(
            "math-vector-named-page".to_owned(),
            "math_vector_block",
            vec![WireStagingStyleDeclaration {
                name: "page".to_owned(),
                value: WireStagingStyleValue::String {
                    value: "default".to_owned(),
                },
                important: false,
            }],
        )?;
    }
    if block_case == crate::StagingPrecomposedVectorBlockFixtureCase::MixedNativeMath {
        let mut first_vector = blocks[2].clone();
        let WireStagingM4Block::MathVectorBlock {
            alt,
            node_id,
            span,
            source_tex,
            equation_number,
            ..
        } = &mut first_vector
        else {
            panic!("precomposed-vector fixture third child is not block math");
        };
        *node_id = 3;
        *span = WireStagingSourceSpan {
            source_id: 0,
            start_byte: 3,
            end_byte: 6,
        };
        *alt = "xたすy".to_owned();
        source_tex.text_span = WireStagingTextSpan {
            text_id: 0,
            start_byte: 3,
            end_byte: 6,
        };
        *equation_number = None;

        let mut second_vector = blocks[2].clone();
        let WireStagingM4Block::MathVectorBlock {
            node_id,
            span,
            source_tex,
            equation_number,
            ..
        } = &mut second_vector
        else {
            unreachable!();
        };
        *node_id = 5;
        *span = WireStagingSourceSpan {
            source_id: 0,
            start_byte: 7,
            end_byte: 13,
        };
        source_tex.text_span = WireStagingTextSpan {
            text_id: 0,
            start_byte: 7,
            end_byte: 10,
        };
        let number = equation_number
            .as_mut()
            .expect("fixture block math has an equation number");
        number.node_id = 6;
        number.span = WireStagingSourceSpan {
            source_id: 0,
            start_byte: 10,
            end_byte: 13,
        };
        number.text_span = WireStagingTextSpan {
            text_id: 0,
            start_byte: 10,
            end_byte: 13,
        };

        *blocks = vec![
            WireStagingM4Block::DisplayMath {
                node_id: 2,
                span: WireStagingSourceSpan {
                    source_id: 0,
                    start_byte: 0,
                    end_byte: 3,
                },
                classes: Vec::new(),
                math_source: WireStagingMathSource {
                    language: "typaxis-math".to_owned(),
                    version: "1".to_owned(),
                    text_span: WireStagingTextSpan {
                        text_id: 0,
                        start_byte: 0,
                        end_byte: 3,
                    },
                },
                speech: "ordered pair a".to_owned(),
                language: None,
            },
            first_vector,
            WireStagingM4Block::DisplayMath {
                node_id: 4,
                span: WireStagingSourceSpan {
                    source_id: 0,
                    start_byte: 6,
                    end_byte: 7,
                },
                classes: Vec::new(),
                math_source: WireStagingMathSource {
                    language: "typaxis-math".to_owned(),
                    version: "1".to_owned(),
                    text_span: WireStagingTextSpan {
                        text_id: 0,
                        start_byte: 6,
                        end_byte: 7,
                    },
                },
                speech: "capital M".to_owned(),
                language: None,
            },
            second_vector,
        ];
    }
    wire.replace_typed_regions(document, resources);
    wire.replace_style_sheet(style_sheet);
    let mut encoded = StagingSemanticDocumentPackageEncoder::new().encode(&wire)?;
    if block_case == crate::StagingPrecomposedVectorBlockFixtureCase::ShortBody {
        const BODY_HEIGHT: &str = "\"body\":{\"height\":6553600,";
        if !encoded.contains(BODY_HEIGHT) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "precomposed-vector fixture page body changed",
            )
            .into());
        }
        encoded = encoded.replacen(BODY_HEIGHT, "\"body\":{\"height\":655360,", 1);
    }
    let decoded = StagingSemanticDocumentPackageDecoder::new().decode(
        encoded.as_bytes(),
        &DocumentPackageDecodePolicy::new(&base_limits),
    )?;
    let package = StagingSemanticPackageParser::new().parse(decoded, &base_limits)?;
    let profile_session = StagingPrecomposedVectorProfileSessionIdentity::fresh();
    let profile = StagingPrecomposedVectorProfileAuthorization::bind_profile_receipt(
        sha256(b"typaxis.precomposed-vector-fixture-profile/1"),
        &package,
        &limits,
        &profile_session,
    )?;
    let base = staging_declared_base_catalog(package.resources())?;
    let config = EffectiveConfig::new(
        true,
        PdfStreamCompression::None,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0")
            .expect("registered fixture data versions"),
        ResourceLimits::default(),
    )?;
    let cli_resource_roots = if equation_font {
        vec![HostPath::new(
            job.parent()
                .expect("precomposed-vector fixture has a parent")
                .join("math/job"),
        )?]
    } else {
        Vec::new()
    };
    let context = HostAdmissionContext::new(
        HostPath::new(package_path)?,
        HostPath::new(job)?,
        None,
        cli_resource_roots,
    );
    let host = HostResourceAdmissionSession::new(&context, &config, &base)?;
    let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
        &base,
        &limits,
        profile.profile_receipt_fingerprint(),
        host.roots(),
    )?;
    for declaration in &package.resources().font_faces {
        let pending = resolver.read_font(host.open_font(declaration.font_face_id)?)?;
        resolver.parse_and_bind_sfnt(pending)?;
    }
    for declaration in &package.resources().images {
        let pending = resolver.read_image(host.open_image(declaration.image_id)?)?;
        resolver.parse_and_bind_declared_image(pending)?;
    }
    let admitted = resolver.finish()?;
    let bindings = bind_staging_precomposed_vectors(&package, &profile, &limits, &admitted)?;
    Ok(StagingPrecomposedVectorBindingFixture {
        package,
        profile,
        limits,
        admitted,
        bindings,
    })
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub struct StagingSafeVectorLayoutFixture {
    pub package: ValidatedStagingSemanticPackage,
    pub profile: StagingSafeVectorProfileView,
    pub limits: M4EffectiveResourceLimits,
    pub admitted: AdmittedResourceLedger,
    pub media: typaxis_resource_admission::StagingDeclaredMediaLedger,
    pub selected: StagingSafeVectorSelectedLayout,
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_safe_vector_layout_fixture(
) -> Result<StagingSafeVectorLayoutFixture, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::PathBuf;
    use typaxis_core::{
        ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, HostAdmissionContext, HostPath,
        M4ResourceLimits, PdfStreamCompression, ResourceLimits, ValidatedResourceLimits,
        DEFAULT_ALLOWED_URI_SCHEMES,
    };
    use typaxis_resource_admission::{
        close_staging_declared_media, staging_declared_base_catalog, AdmittedResourceResolver,
        HostResourceAdmissionSession,
    };
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    let job = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../samples/machine-package/staging/production-book-1/vector-media/job");
    let package_path = job.join("document-package.json");
    let base_limits = ValidatedResourceLimits::new(ResourceLimits::default())?;
    let limits = M4EffectiveResourceLimits::new(base_limits.clone(), M4ResourceLimits::default())?;
    let decoded = StagingSemanticDocumentPackageDecoder::new().decode(
        &fs::read(&package_path)?,
        &DocumentPackageDecodePolicy::new(&base_limits),
    )?;
    let package = StagingSemanticPackageParser::new().parse(decoded, &base_limits)?;
    let profile = StagingSafeVectorProfileView::new(&package, &limits)?;
    let base = staging_declared_base_catalog(package.resources())?;
    let config = EffectiveConfig::new(
        true,
        PdfStreamCompression::None,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0")
            .expect("registered fixture data versions"),
        ResourceLimits::default(),
    )?;
    let context = HostAdmissionContext::new(
        HostPath::new(package_path)?,
        HostPath::new(job)?,
        None,
        Vec::new(),
    );
    let session = HostResourceAdmissionSession::new(&context, &config, &base)?;
    let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
        &base,
        &limits,
        profile.profile_fingerprint(),
        session.roots(),
    )?;
    for declaration in &package.resources().images {
        let pending = resolver.read_image(session.open_image(declaration.image_id)?)?;
        resolver.parse_and_bind_declared_image(pending)?;
    }
    let admitted = resolver.finish()?;
    let media = close_staging_declared_media(&admitted, package.resources())?;
    let selected = layout_staging_safe_vectors(&package, &profile, &limits, &admitted)?;
    Ok(StagingSafeVectorLayoutFixture {
        package,
        profile,
        limits,
        admitted,
        media,
        selected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_length(value: i64) -> Length {
        Length::from_raw(value).unwrap()
    }

    fn positive(value: i64) -> PositiveLength {
        PositiveLength::new(raw_length(value)).unwrap()
    }

    fn nonnegative(value: i64) -> typaxis_core::NonNegativeLength {
        typaxis_core::NonNegativeLength::new(raw_length(value)).unwrap()
    }

    fn reseal_receipt(value: &mut ValidatedPrecomposedVectorReceipt) {
        value.canonical_jcs = encode_precomposed_vector_receipt(value);
        value.fingerprint = sha256(value.canonical_jcs.as_bytes());
    }

    fn reseal_bindings(value: &mut ValidatedPrecomposedVectorBindings) {
        value.canonical_jcs = encode_precomposed_vector_binding_set(
            &value.epoch,
            &value.receipts,
            &value.math_receipts,
        );
        value.fingerprint = sha256(value.canonical_jcs.as_bytes());
    }

    fn assert_binding_tamper_fails(
        fixture: &StagingPrecomposedVectorBindingFixture,
        mut bindings: ValidatedPrecomposedVectorBindings,
    ) {
        for receipt in &mut bindings.receipts {
            reseal_receipt(receipt);
        }
        reseal_bindings(&mut bindings);
        assert_eq!(
            bindings.verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
            ),
            Err(PrecomposedVectorBindingError::ReceiptMismatch)
        );
        assert!(bindings
            .verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
            )
            .unwrap_err()
            .to_string()
            .starts_with("I9190:"));
    }

    #[test]
    fn precomposed_vector_binding_closes_four_kinds_and_backend_inputs() {
        let fixture = staging_precomposed_vector_binding_fixture().unwrap();
        assert_eq!(fixture.bindings.receipts().len(), 4);
        assert_eq!(fixture.bindings.math_receipts().len(), 2);
        assert_eq!(
            fixture
                .bindings
                .receipts()
                .iter()
                .map(ValidatedPrecomposedVectorReceipt::kind)
                .collect::<Vec<_>>(),
            [
                PrecomposedVectorKind::InlineVector,
                PrecomposedVectorKind::MathVector,
                PrecomposedVectorKind::VectorFigure,
                PrecomposedVectorKind::MathVectorBlock,
            ]
        );
        fixture
            .bindings
            .verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
            )
            .unwrap();
        let repeated = staging_precomposed_vector_binding_fixture().unwrap();
        assert_eq!(fixture.bindings.epoch(), repeated.bindings.epoch());
        assert_eq!(
            fixture.bindings.canonical_jcs(),
            repeated.bindings.canonical_jcs()
        );
        assert_eq!(
            fixture.bindings.fingerprint(),
            repeated.bindings.fingerprint()
        );
        let foreign_session = StagingPrecomposedVectorProfileSessionIdentity::fresh();
        let foreign_profile = StagingPrecomposedVectorProfileAuthorization::bind_profile_receipt(
            fixture.profile.profile_receipt_fingerprint(),
            &fixture.package,
            &fixture.limits,
            &foreign_session,
        )
        .unwrap();
        assert_eq!(
            fixture.bindings.verify(
                &fixture.package,
                &foreign_profile,
                &fixture.limits,
                &fixture.admitted,
            ),
            Err(PrecomposedVectorBindingError::ReceiptMismatch)
        );
        assert!(fixture
            .bindings
            .canonical_jcs()
            .contains(PRECOMPOSED_VECTOR_BINDING_SET_ALGORITHM));
        assert!(!fixture
            .bindings
            .canonical_jcs()
            .contains("svg/x-plus-y.svg"));
        assert!(!fixture.bindings.canonical_jcs().contains("(a)x+y"));

        let inline = fixture.bindings.receipt(NodeId::new(3)).unwrap();
        let PrecomposedVectorPlacementInput::Inline(inline_input) = inline.placement() else {
            panic!("inline vector must have only inline placement input");
        };
        assert_eq!(inline_input.spacing_before().get().raw(), 16_384);
        assert_eq!(inline_input.spacing_after().get().raw(), 16_384);
        assert_eq!(inline_input.scale().get().raw(), 65_536);
        assert_eq!(inline_input.paint(), ResolvedRgb8::BLACK);
        let metrics = inline_input.metrics();
        assert_eq!(metrics.advance().get().raw(), 1_900_544);
        assert_eq!(metrics.viewport_width().get().raw(), 1_966_080);
        let line_baseline = raw_length(4_000_000);
        let viewport_top = metrics.viewport_top(line_baseline).unwrap();
        assert_eq!(metrics.line_baseline(viewport_top), Some(line_baseline));

        let figure = fixture.bindings.receipt(NodeId::new(5)).unwrap();
        let PrecomposedVectorPlacementInput::VectorFigure(figure_input) = figure.placement() else {
            panic!("vector Figure must have only Figure placement input");
        };
        assert!(figure_input.style().keep_caption());
        assert_eq!(figure_input.paint(), ResolvedRgb8::BLACK);

        let block = fixture.bindings.receipt(NodeId::new(6)).unwrap();
        let PrecomposedVectorPlacementInput::MathVectorBlock(block_input) = block.placement()
        else {
            panic!("math block must have only math-block placement input");
        };
        assert!(block_input
            .style()
            .equation_number_style()
            .font_families()
            .is_none());
        assert_eq!(block_input.paint(), ResolvedRgb8::BLACK);
        assert_eq!(
            block_input.metrics().viewport_width().get().raw(),
            1_966_080
        );

        for receipt in fixture.bindings.receipts() {
            assert_eq!(receipt.resource().image_id(), ImageResourceId::new(0));
            assert_eq!(
                receipt.resource().declared_media(),
                BoundPrecomposedVectorMedia::SafeSvg2
            );
            assert_eq!(
                receipt.resource().admitted_media(),
                BoundPrecomposedVectorMedia::SafeSvg2
            );
            assert_eq!(
                receipt.resource().parser_profile(),
                SafeVectorParserProfile::SafeSvg2
            );
            assert_eq!(
                receipt.resource().limits_fingerprint(),
                fixture.limits.fingerprint()
            );
            assert_eq!(
                receipt.resource().profile_fingerprint(),
                fixture.profile.profile_receipt_fingerprint()
            );
        }

        let mixed = staging_precomposed_vector_binding_fixture_with_generic_safe_svg1().unwrap();
        mixed
            .bindings
            .verify(
                &mixed.package,
                &mixed.profile,
                &mixed.limits,
                &mixed.admitted,
            )
            .unwrap();
        for owner in [NodeId::new(3), NodeId::new(5)] {
            let resource = mixed.bindings.receipt(owner).unwrap().resource();
            assert_eq!(resource.image_id(), ImageResourceId::new(1));
            assert_eq!(
                resource.declared_media(),
                BoundPrecomposedVectorMedia::SafeSvg1
            );
            assert_eq!(
                resource.admitted_media(),
                BoundPrecomposedVectorMedia::SafeSvg1
            );
            assert_eq!(resource.parser_profile(), SafeVectorParserProfile::SafeSvg1);
        }
        for owner in [NodeId::new(4), NodeId::new(6)] {
            let resource = mixed.bindings.receipt(owner).unwrap().resource();
            assert_eq!(resource.image_id(), ImageResourceId::new(0));
            assert_eq!(
                resource.declared_media(),
                BoundPrecomposedVectorMedia::SafeSvg2
            );
            assert_eq!(
                resource.admitted_media(),
                BoundPrecomposedVectorMedia::SafeSvg2
            );
            assert_eq!(resource.parser_profile(), SafeVectorParserProfile::SafeSvg2);
        }
    }

    #[test]
    fn precomposed_vector_scale_uses_one_half_even_scale_for_both_axes() {
        let metrics = typaxis_document::PrecomposedVectorMetrics {
            advance: positive(2),
            ascent: positive(1),
            baseline: nonnegative(1),
            descent: nonnegative(0),
            origin_x: raw_length(0),
            viewport: typaxis_document::PrecomposedVectorViewport {
                width: positive(2),
                height: positive(1),
            },
        };
        let spacing = typaxis_document::PrecomposedVectorSpacing {
            before: nonnegative(0),
            after: nonnegative(0),
        };
        let input = PrecomposedVectorInlinePlacementInput::from_validated_metrics(
            metrics,
            spacing,
            positive(3),
            positive(2),
            ResolvedRgb8::BLACK,
        )
        .unwrap();
        assert_eq!(input.scale().get().raw(), 43_691);
        assert_ne!(
            i128::from(input.metrics().viewport_width().get().raw()) * 2,
            i128::from(input.metrics().viewport_height().get().raw()) * 3,
            "rounded axes must not be rejected by an exact cross-product test"
        );

        let mut nonuniform = metrics;
        nonuniform.viewport.height = positive(2);
        assert_eq!(
            PrecomposedVectorInlinePlacementInput::from_validated_metrics(
                nonuniform,
                spacing,
                positive(3),
                positive(2),
                ResolvedRgb8::BLACK,
            ),
            Err(PrecomposedVectorGeometryError::NonUniformScale)
        );

        let mut tie_to_zero = metrics;
        tie_to_zero.viewport.width = positive(1);
        assert_eq!(
            PrecomposedVectorInlinePlacementInput::from_validated_metrics(
                tie_to_zero,
                spacing,
                positive(131_072),
                positive(65_536),
                ResolvedRgb8::BLACK,
            ),
            Err(PrecomposedVectorGeometryError::ScaleOutOfRange)
        );
    }

    #[test]
    fn precomposed_vector_binding_rejects_self_consistent_component_swaps() {
        let fixture = staging_precomposed_vector_binding_fixture().unwrap();

        let mut wrong_image = fixture.bindings.clone();
        wrong_image.receipts[0].resource.image_id = ImageResourceId::new(1);
        assert_binding_tamper_fails(&fixture, wrong_image);

        let mut wrong_media = fixture.bindings.clone();
        wrong_media.receipts[0].resource.declared_media = BoundPrecomposedVectorMedia::SafeSvg1;
        assert_binding_tamper_fails(&fixture, wrong_media);

        let mut wrong_admitted_media = fixture.bindings.clone();
        wrong_admitted_media.receipts[0].resource.admitted_media =
            BoundPrecomposedVectorMedia::SafeSvg1;
        assert_binding_tamper_fails(&fixture, wrong_admitted_media);

        let mut wrong_source_hash = fixture.bindings.clone();
        wrong_source_hash.receipts[0].resource.source_sha256[0] ^= 1;
        assert_binding_tamper_fails(&fixture, wrong_source_hash);

        let mut wrong_parser = fixture.bindings.clone();
        wrong_parser.receipts[0].resource.parser_id = "typaxis.safe-svg-parser/wrong";
        assert_binding_tamper_fails(&fixture, wrong_parser);

        let mut wrong_parser_profile = fixture.bindings.clone();
        wrong_parser_profile.receipts[0].resource.parser_profile =
            SafeVectorParserProfile::SafeSvg1;
        assert_binding_tamper_fails(&fixture, wrong_parser_profile);

        let mut wrong_ir = fixture.bindings.clone();
        wrong_ir.receipts[0].resource.ir_fingerprint[0] ^= 1;
        assert_binding_tamper_fails(&fixture, wrong_ir);

        let mut wrong_ir_id = fixture.bindings.clone();
        wrong_ir_id.receipts[0].resource.ir_id = "typaxis.safe-vector-ir/wrong";
        assert_binding_tamper_fails(&fixture, wrong_ir_id);

        let mut wrong_profile = fixture.bindings.clone();
        wrong_profile.receipts[0].resource.profile_fingerprint[0] ^= 1;
        assert_binding_tamper_fails(&fixture, wrong_profile);

        let mut wrong_limits = fixture.bindings.clone();
        wrong_limits.receipts[0].resource.limits_fingerprint[0] ^= 1;
        assert_binding_tamper_fails(&fixture, wrong_limits);

        let mut wrong_epoch = fixture.bindings.clone();
        wrong_epoch.receipts[0].epoch_fingerprint[0] ^= 1;
        assert_binding_tamper_fails(&fixture, wrong_epoch);

        let mut wrong_metrics = fixture.bindings.clone();
        wrong_metrics.receipts[0].metrics_fingerprint[0] ^= 1;
        assert_binding_tamper_fails(&fixture, wrong_metrics);

        let mut wrong_style = fixture.bindings.clone();
        let (before_block, block) = wrong_style.receipts.split_at_mut(3);
        std::mem::swap(&mut before_block[2].placement, &mut block[0].placement);
        assert_binding_tamper_fails(&fixture, wrong_style);

        let mut wrong_alternative = fixture.bindings.clone();
        wrong_alternative.receipts[0].alternative = "別の代替テキスト".to_owned();
        wrong_alternative.receipts[0].alternative_sha256 =
            sha256(wrong_alternative.receipts[0].alternative.as_bytes());
        assert_binding_tamper_fails(&fixture, wrong_alternative);

        let mut wrong_language = fixture.bindings.clone();
        wrong_language.receipts[0].language = Some("en-US".to_owned());
        assert_binding_tamper_fails(&fixture, wrong_language);

        let mut wrong_paint = fixture.bindings.clone();
        let syntax_metrics = fixture
            .package
            .precomposed_vector_metrics_for(NodeId::new(3))
            .unwrap();
        let PrecomposedVectorMetricPayload::Inline { metrics, spacing } = syntax_metrics.payload()
        else {
            unreachable!();
        };
        let resource = &wrong_paint.receipts[0].resource;
        wrong_paint.receipts[0].placement = PrecomposedVectorPlacementInput::Inline(
            PrecomposedVectorInlinePlacementInput::from_validated_metrics(
                metrics,
                spacing,
                resource.intrinsic_width,
                resource.intrinsic_height,
                ResolvedRgb8::new(1, 2, 3),
            )
            .unwrap(),
        );
        assert_binding_tamper_fails(&fixture, wrong_paint);
    }

    #[test]
    fn vector_layout_preserves_intrinsic_ratio_and_closes_selected_figure() {
        let fixture = staging_safe_vector_layout_fixture().unwrap();
        assert_eq!(fixture.selected.placements().len(), 1);
        let placement = &fixture.selected.placements()[0];
        assert_eq!(placement.image_id(), ImageResourceId::new(0));
        assert_eq!(placement.bounds().width().get().raw(), 80 * 65_536);
        assert_eq!(placement.bounds().height().get().raw(), 40 * 65_536);
        assert_eq!(placement.bounds().x().raw(), 100 * 65_536);
        assert_eq!(placement.bounds().y().raw(), 100 * 65_536);
        assert_eq!(
            fixture.selected.page_geometry().page_width().get().raw(),
            1_000 * 65_536
        );
        assert_eq!(
            fixture.selected.page_geometry().page_height().get().raw(),
            800 * 65_536
        );
        fixture
            .selected
            .verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
            )
            .unwrap();
        assert!(fixture
            .selected
            .receipt()
            .canonical_jcs()
            .contains("\"admitted_fingerprint\""));
        assert!(placements_match_package(
            fixture.selected.placements(),
            &fixture.package,
            &fixture.profile,
        ));
        let mut wrong_resource = fixture.selected.placements().to_vec();
        wrong_resource[0].image_id = ImageResourceId::new(1);
        assert!(!placements_match_package(
            &wrong_resource,
            &fixture.package,
            &fixture.profile,
        ));

        let odd_ratio_scale = scale_to_fit(3 * FIXED_ONE, 2 * FIXED_ONE).unwrap().unwrap();
        assert_eq!(odd_ratio_scale, 43_690);
        assert_eq!(
            scaled_dimension(3 * FIXED_ONE, odd_ratio_scale).unwrap(),
            131_070
        );
        assert_eq!(
            scaled_dimension(2 * FIXED_ONE, odd_ratio_scale).unwrap(),
            87_380
        );
    }
}
