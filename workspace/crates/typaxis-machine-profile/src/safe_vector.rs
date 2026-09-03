use std::collections::BTreeSet;
use typaxis_core::{push_jcs_string, sha256, ImageResourceId, M4EffectiveResourceLimits, NodeId};
use typaxis_syntax::machine_profile_boundary::{
    require_precomposed_vector_style_registry, ImageMediaDeclaration, ImageMediaType,
    PrecomposedVectorStyleKind, StagingM4Block, PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION,
};
use typaxis_syntax::{
    PrecomposedVectorKind, PrecomposedVectorMetricPayload,
    StagingPrecomposedVectorProfileAuthorization, StagingSafeVectorProfileView,
    ValidatedStagingSemanticPackage,
};

use crate::{
    descriptor::MachineVectorMetric,
    preflight_staging_semantic_container_profile,
    semantic_container::{
        preflight_staging_semantic_container_profile_for_math,
        validate_staging_semantic_container_domain_for_precomposed_vector,
    },
    StagingSemanticContainerPreflightReceipt, StagingSemanticContainerSessionIdentity,
};

pub const STAGING_SAFE_VECTOR_PROFILE_ALGORITHM: &str =
    "typaxis.production-book-safe-vector-profile/1";
pub const STAGING_PRECOMPOSED_VECTOR_PROFILE_ALGORITHM: &str =
    "typaxis.production-book-precomposed-vector-profile/1";
pub const STAGING_SAFE_VECTOR_PROFILE_V2: &str = "typaxis.resource-profile/safe-vector/2";
pub const STAGING_PRODUCTION_BOOK_RESOURCE_SET_V2: &str = "typaxis.production-book-resource-set/2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorProfileError {
    BaseProfile,
    MissingImage(ImageResourceId),
    WrongMedia(ImageResourceId),
    UnsupportedMath,
    ReceiptMismatch,
    PrecomposedVectorStaging(NodeId),
    SvgSafe2Staging(ImageResourceId),
}

impl std::fmt::Display for StagingSafeVectorProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BaseProfile => formatter.write_str("R7100: production-book preflight failed"),
            Self::MissingImage(id) => write!(
                formatter,
                "R7100: Figure image {} is not declared",
                id.get()
            ),
            Self::WrongMedia(id) => write!(
                formatter,
                "R7100: Figure image {} is not declared svg-safe-1",
                id.get()
            ),
            Self::UnsupportedMath => {
                formatter.write_str("L5100: SafeVector profile does not admit math")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: SafeVector profile receipt mismatch")
            }
            Self::PrecomposedVectorStaging(owner) => write!(
                formatter,
                "P1102: precomposed vector at node {} requires SafeVector /2",
                owner.get()
            ),
            Self::SvgSafe2Staging(id) => write!(
                formatter,
                "P1102: svg-safe-2 image {} requires SafeVector /2",
                id.get()
            ),
        }
    }
}

impl std::error::Error for StagingSafeVectorProfileError {}

const GENERIC_PRECOMPOSED_VECTOR_MEDIA: &[ImageMediaType] =
    &[ImageMediaType::SvgSafe1, ImageMediaType::SvgSafe2];
const MATH_PRECOMPOSED_VECTOR_MEDIA: &[ImageMediaType] = &[ImageMediaType::SvgSafe2];
const FULL_PRECOMPOSED_VECTOR_METRICS: &[MachineVectorMetric] = &[
    MachineVectorMetric::Advance,
    MachineVectorMetric::Ascent,
    MachineVectorMetric::Baseline,
    MachineVectorMetric::Descent,
    MachineVectorMetric::OriginX,
    MachineVectorMetric::Viewport,
];
const FIGURE_PRECOMPOSED_VECTOR_METRICS: &[MachineVectorMetric] = &[MachineVectorMetric::Viewport];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingPrecomposedVectorProfileDescriptor;

impl StagingPrecomposedVectorProfileDescriptor {
    pub const fn kinds(self) -> [PrecomposedVectorKind; 4] {
        [
            PrecomposedVectorKind::InlineVector,
            PrecomposedVectorKind::MathVector,
            PrecomposedVectorKind::MathVectorBlock,
            PrecomposedVectorKind::VectorFigure,
        ]
    }

    pub const fn media_for(self, kind: PrecomposedVectorKind) -> &'static [ImageMediaType] {
        match kind {
            PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::VectorFigure => {
                GENERIC_PRECOMPOSED_VECTOR_MEDIA
            }
            PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock => {
                MATH_PRECOMPOSED_VECTOR_MEDIA
            }
        }
    }

    pub const fn required_metrics_for(
        self,
        kind: PrecomposedVectorKind,
    ) -> &'static [MachineVectorMetric] {
        match kind {
            PrecomposedVectorKind::VectorFigure => FIGURE_PRECOMPOSED_VECTOR_METRICS,
            PrecomposedVectorKind::InlineVector
            | PrecomposedVectorKind::MathVector
            | PrecomposedVectorKind::MathVectorBlock => FULL_PRECOMPOSED_VECTOR_METRICS,
        }
    }

    pub const fn style_kind_for(
        self,
        kind: PrecomposedVectorKind,
    ) -> Option<PrecomposedVectorStyleKind> {
        match kind {
            PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::MathVector => None,
            PrecomposedVectorKind::VectorFigure => Some(PrecomposedVectorStyleKind::VectorFigure),
            PrecomposedVectorKind::MathVectorBlock => {
                Some(PrecomposedVectorStyleKind::MathVectorBlock)
            }
        }
    }

    pub const fn style_registry(self) -> &'static str {
        PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION
    }

    pub const fn safe_vector_profile(self) -> &'static str {
        STAGING_SAFE_VECTOR_PROFILE_V2
    }

    pub const fn resource_set(self) -> &'static str {
        STAGING_PRODUCTION_BOOK_RESOURCE_SET_V2
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingPrecomposedVectorProfileError {
    BaseProfile,
    ExistingFigureSvgSafe2 {
        owner: NodeId,
        image: ImageResourceId,
    },
    KindMediaMismatch {
        owner: NodeId,
        image: ImageResourceId,
    },
    MissingImage(ImageResourceId),
    MetricMismatch(NodeId),
    MissingProvenance(ImageResourceId),
    StyleMismatch(NodeId),
    ReceiptMismatch,
}

impl std::fmt::Display for StagingPrecomposedVectorProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BaseProfile => {
                formatter.write_str("R7100: production-book vector preflight failed")
            }
            Self::ExistingFigureSvgSafe2 { owner, image } => write!(
                formatter,
                "R7100: existing Figure node {} cannot use svg-safe-2 image {}",
                owner.get(),
                image.get()
            ),
            Self::KindMediaMismatch { owner, image } => write!(
                formatter,
                "R7100: vector node {} cannot use image {} with its declared media",
                owner.get(),
                image.get()
            ),
            Self::MissingImage(image) => write!(
                formatter,
                "R7100: precomposed vector image {} is not declared",
                image.get()
            ),
            Self::MetricMismatch(owner) => write!(
                formatter,
                "P1102: precomposed vector metrics mismatch at node {}",
                owner.get()
            ),
            Self::MissingProvenance(image) => write!(
                formatter,
                "R7100: svg-safe-2 image {} lacks closed provenance",
                image.get()
            ),
            Self::StyleMismatch(owner) => write!(
                formatter,
                "L5101: precomposed vector style mismatch at node {}",
                owner.get()
            ),
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: precomposed vector profile receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingPrecomposedVectorProfileError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrecomposedVectorResourceAuthorization {
    image_id: ImageResourceId,
    media: ImageMediaType,
    provenance_jcs: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrecomposedVectorUseAuthorization {
    owner: NodeId,
    image_id: ImageResourceId,
    kind: PrecomposedVectorKind,
    media: ImageMediaType,
    metric_names: Vec<MachineVectorMetric>,
    metrics_fingerprint: [u8; 32],
    style_fingerprint: Option<[u8; 32]>,
}

#[derive(Debug)]
pub struct StagingPrecomposedVectorProfileReceipt {
    package_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    session: StagingSemanticContainerSessionIdentity,
    semantic_container_count: u32,
    resources: Vec<PrecomposedVectorResourceAuthorization>,
    uses: Vec<PrecomposedVectorUseAuthorization>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
    authorization: StagingPrecomposedVectorProfileAuthorization,
}

impl StagingPrecomposedVectorProfileReceipt {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }

    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }

    pub fn vector_owners(&self) -> impl ExactSizeIterator<Item = NodeId> + '_ {
        self.uses.iter().map(|usage| usage.owner)
    }

    pub fn vector_resource_ids(&self) -> impl ExactSizeIterator<Item = ImageResourceId> + '_ {
        self.resources.iter().map(|resource| resource.image_id)
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub const fn authorization(&self) -> &StagingPrecomposedVectorProfileAuthorization {
        &self.authorization
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
        session: &StagingSemanticContainerSessionIdentity,
    ) -> Result<(), StagingPrecomposedVectorProfileError> {
        if self.session != *session
            || !self
                .authorization
                .belongs_to_session(session.precomposed_vector_profile_session())
        {
            return Err(StagingPrecomposedVectorProfileError::ReceiptMismatch);
        }
        self.authorizes(package, limits)
    }

    pub fn authorizes(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingPrecomposedVectorProfileError> {
        let semantic_container_count =
            validate_staging_semantic_container_domain_for_precomposed_vector(
                package,
                limits.base(),
            )
            .map_err(|_| StagingPrecomposedVectorProfileError::BaseProfile)?;
        let (resources, uses) = collect_precomposed_vector_authorization(package)?;
        let canonical_jcs = encode_precomposed_vector_profile(
            package,
            limits.fingerprint(),
            semantic_container_count,
            &resources,
            &uses,
        );
        if self.package_sha256 != package.canonical_jcs_sha256()
            || self.semantic_fingerprint != package.semantic_fingerprint()
            || self.limits_fingerprint != limits.fingerprint()
            || self.semantic_container_count != semantic_container_count
            || self.resources != resources
            || self.uses != uses
            || self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
            || self.authorization.profile_receipt_fingerprint() != self.fingerprint
            || self.authorization.vector_owners().ne(self.vector_owners())
            || self.authorization.authorizes(package, limits).is_err()
        {
            return Err(StagingPrecomposedVectorProfileError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct StagingSafeVectorProfileReceipt {
    base: StagingSemanticContainerPreflightReceipt,
    limits_fingerprint: [u8; 32],
    vector_resource_ids: Vec<ImageResourceId>,
    figure_owners: Vec<NodeId>,
    page_geometry_fingerprint: [u8; 32],
    math_extension: bool,
    canonical_jcs: String,
    fingerprint: [u8; 32],
    authorization: StagingSafeVectorProfileView,
}

impl StagingSafeVectorProfileReceipt {
    pub const fn base(&self) -> &StagingSemanticContainerPreflightReceipt {
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
    pub const fn page_geometry_fingerprint(&self) -> [u8; 32] {
        self.page_geometry_fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub const fn authorization(&self) -> &StagingSafeVectorProfileView {
        &self.authorization
    }

    pub fn authorizes(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSafeVectorProfileError> {
        let (vector_resource_ids, figure_owners) = collect_vector_contract(package)?;
        let has_math = !package.math_nodes().is_empty();
        if self.math_extension != has_math {
            return Err(StagingSafeVectorProfileError::UnsupportedMath);
        }
        let expected_authorization = StagingSafeVectorProfileView::new(package, limits)
            .map_err(|_| StagingSafeVectorProfileError::ReceiptMismatch)?;
        let canonical_jcs = encode(
            package,
            self.base.fingerprint(),
            limits.fingerprint(),
            &vector_resource_ids,
            &figure_owners,
            expected_authorization.page_geometry().fingerprint(),
            self.math_extension,
        );
        if self.base.package_sha256() != package.canonical_jcs_sha256()
            || self.base.semantic_fingerprint() != package.semantic_fingerprint()
            || self.base.limits() != limits.base()
            || self.limits_fingerprint != limits.fingerprint()
            || self.vector_resource_ids != vector_resource_ids
            || self.figure_owners != figure_owners
            || self.page_geometry_fingerprint
                != expected_authorization.page_geometry().fingerprint()
            || self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
            || self.authorization != expected_authorization
        {
            return Err(StagingSafeVectorProfileError::ReceiptMismatch);
        }
        Ok(())
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &M4EffectiveResourceLimits,
        session: &StagingSemanticContainerSessionIdentity,
    ) -> Result<(), StagingSafeVectorProfileError> {
        self.base
            .verify(package, limits.base(), session)
            .map_err(|_| StagingSafeVectorProfileError::ReceiptMismatch)?;
        self.authorizes(package, limits)
    }
}

pub fn preflight_staging_safe_vector_profile(
    package: &ValidatedStagingSemanticPackage,
    limits: &M4EffectiveResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingSafeVectorProfileReceipt, StagingSafeVectorProfileError> {
    if !package.math_nodes().is_empty() {
        return Err(StagingSafeVectorProfileError::UnsupportedMath);
    }
    preflight_staging_safe_vector_profile_inner(package, limits, session, false)
}

pub(crate) fn preflight_staging_safe_vector_profile_for_math(
    package: &ValidatedStagingSemanticPackage,
    limits: &M4EffectiveResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingSafeVectorProfileReceipt, StagingSafeVectorProfileError> {
    if package.math_nodes().is_empty() {
        return Err(StagingSafeVectorProfileError::UnsupportedMath);
    }
    preflight_staging_safe_vector_profile_inner(package, limits, session, true)
}

fn preflight_staging_safe_vector_profile_inner(
    package: &ValidatedStagingSemanticPackage,
    limits: &M4EffectiveResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
    math_extension: bool,
) -> Result<StagingSafeVectorProfileReceipt, StagingSafeVectorProfileError> {
    if let Some(image) = package
        .resources()
        .images
        .iter()
        .find(|image| image.media == ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2))
    {
        return Err(StagingSafeVectorProfileError::SvgSafe2Staging(
            image.image_id,
        ));
    }
    if let Some(owner) = first_precomposed_vector_owner(&package.document().blocks).or_else(|| {
        package
            .document()
            .footnotes
            .iter()
            .find_map(|footnote| first_precomposed_vector_owner(&footnote.blocks))
    }) {
        return Err(StagingSafeVectorProfileError::PrecomposedVectorStaging(
            owner,
        ));
    }
    let base = if math_extension {
        preflight_staging_semantic_container_profile_for_math(package, limits.base(), session)
    } else {
        preflight_staging_semantic_container_profile(package, limits.base(), session)
    }
    .map_err(|_| StagingSafeVectorProfileError::BaseProfile)?;
    let (vector_resource_ids, figure_owners) = collect_vector_contract(package)?;
    let authorization = StagingSafeVectorProfileView::new(package, limits)
        .map_err(|_| StagingSafeVectorProfileError::ReceiptMismatch)?;
    let page_geometry_fingerprint = authorization.page_geometry().fingerprint();
    let canonical_jcs = encode(
        package,
        base.fingerprint(),
        limits.fingerprint(),
        &vector_resource_ids,
        &figure_owners,
        page_geometry_fingerprint,
        math_extension,
    );
    let receipt = StagingSafeVectorProfileReceipt {
        base,
        limits_fingerprint: limits.fingerprint(),
        vector_resource_ids,
        figure_owners,
        page_geometry_fingerprint,
        math_extension,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
        authorization,
    };
    receipt.authorizes(package, limits)?;
    Ok(receipt)
}

pub fn preflight_staging_precomposed_vector_profile(
    package: &ValidatedStagingSemanticPackage,
    limits: &M4EffectiveResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingPrecomposedVectorProfileReceipt, StagingPrecomposedVectorProfileError> {
    let semantic_container_count =
        validate_staging_semantic_container_domain_for_precomposed_vector(package, limits.base())
            .map_err(|_| StagingPrecomposedVectorProfileError::BaseProfile)?;
    let (resources, uses) = collect_precomposed_vector_authorization(package)?;
    let canonical_jcs = encode_precomposed_vector_profile(
        package,
        limits.fingerprint(),
        semantic_container_count,
        &resources,
        &uses,
    );
    let fingerprint = sha256(canonical_jcs.as_bytes());
    let authorization = StagingPrecomposedVectorProfileAuthorization::bind_profile_receipt(
        fingerprint,
        package,
        limits,
        session.precomposed_vector_profile_session(),
    )
    .map_err(|_| StagingPrecomposedVectorProfileError::ReceiptMismatch)?;
    let receipt = StagingPrecomposedVectorProfileReceipt {
        package_sha256: package.canonical_jcs_sha256(),
        semantic_fingerprint: package.semantic_fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        session: session.clone(),
        semantic_container_count,
        resources,
        uses,
        fingerprint,
        canonical_jcs,
        authorization,
    };
    receipt.verify(package, limits, session)?;
    Ok(receipt)
}

fn collect_precomposed_vector_authorization(
    package: &ValidatedStagingSemanticPackage,
) -> Result<
    (
        Vec<PrecomposedVectorResourceAuthorization>,
        Vec<PrecomposedVectorUseAuthorization>,
    ),
    StagingPrecomposedVectorProfileError,
> {
    package
        .checked_wire()
        .map_err(|_| StagingPrecomposedVectorProfileError::ReceiptMismatch)?;
    require_precomposed_vector_style_registry(PRECOMPOSED_VECTOR_STYLE_REGISTRY_VERSION)
        .map_err(|_| StagingPrecomposedVectorProfileError::ReceiptMismatch)?;

    let mut resources = Vec::new();
    for image in &package.resources().images {
        let ImageMediaDeclaration::Declared(media) = image.media else {
            continue;
        };
        if !matches!(media, ImageMediaType::SvgSafe1 | ImageMediaType::SvgSafe2) {
            continue;
        }
        let provenance_jcs = match (media, image.vector_provenance.as_ref()) {
            (ImageMediaType::SvgSafe1, None) => None,
            (ImageMediaType::SvgSafe2, Some(provenance)) => {
                let mut value = String::from("{\"engine_id\":");
                push_jcs_string(&mut value, &provenance.engine_id);
                value.push_str(",\"engine_version\":");
                push_jcs_string(&mut value, &provenance.engine_version);
                value.push_str(",\"rules_version\":");
                push_jcs_string(&mut value, &provenance.rules_version);
                value.push('}');
                Some(value)
            }
            (ImageMediaType::SvgSafe2, None) => {
                return Err(StagingPrecomposedVectorProfileError::MissingProvenance(
                    image.image_id,
                ));
            }
            (ImageMediaType::SvgSafe1, Some(_)) | (ImageMediaType::Png, _) => {
                return Err(StagingPrecomposedVectorProfileError::ReceiptMismatch);
            }
        };
        resources.push(PrecomposedVectorResourceAuthorization {
            image_id: image.image_id,
            media,
            provenance_jcs,
        });
    }

    validate_existing_figure_vector_media(&package.document().blocks, package)?;
    for footnote in &package.document().footnotes {
        validate_existing_figure_vector_media(&footnote.blocks, package)?;
    }

    let mut domain_uses = Vec::new();
    collect_precomposed_vector_domain_uses(&package.document().blocks, &mut domain_uses);
    for footnote in &package.document().footnotes {
        collect_precomposed_vector_domain_uses(&footnote.blocks, &mut domain_uses);
    }
    if domain_uses.len() != package.precomposed_vector_metrics().len() {
        return Err(StagingPrecomposedVectorProfileError::ReceiptMismatch);
    }

    let descriptor = StagingPrecomposedVectorProfileDescriptor;
    let mut uses = Vec::new();
    for ((owner, kind, image_id), metrics) in domain_uses
        .into_iter()
        .zip(package.precomposed_vector_metrics())
    {
        package
            .verify_precomposed_vector_metrics(metrics)
            .map_err(|_| StagingPrecomposedVectorProfileError::ReceiptMismatch)?;
        if metrics.node_id() != owner
            || metrics.kind() != kind
            || metrics.resource_binding().image_id() != image_id
            || !metric_payload_matches_kind(metrics.payload(), kind)
        {
            return Err(StagingPrecomposedVectorProfileError::MetricMismatch(owner));
        }
        let image = package
            .resources()
            .images
            .get(image_id.get() as usize)
            .filter(|image| image.image_id == image_id)
            .ok_or(StagingPrecomposedVectorProfileError::MissingImage(image_id))?;
        let ImageMediaDeclaration::Declared(media) = image.media else {
            return Err(StagingPrecomposedVectorProfileError::KindMediaMismatch {
                owner,
                image: image_id,
            });
        };
        if !descriptor.media_for(kind).contains(&media) {
            return Err(StagingPrecomposedVectorProfileError::KindMediaMismatch {
                owner,
                image: image_id,
            });
        }
        let style_fingerprint = match descriptor.style_kind_for(kind) {
            Some(style_kind) => {
                let style = package
                    .precomposed_vector_style(owner)
                    .ok_or(StagingPrecomposedVectorProfileError::StyleMismatch(owner))?;
                package
                    .verify_precomposed_vector_style(style)
                    .map_err(|_| StagingPrecomposedVectorProfileError::StyleMismatch(owner))?;
                style
                    .verify_for(style_kind)
                    .map_err(|_| StagingPrecomposedVectorProfileError::StyleMismatch(owner))?;
                Some(style.fingerprint())
            }
            None => None,
        };
        uses.push(PrecomposedVectorUseAuthorization {
            owner,
            image_id,
            kind,
            media,
            metric_names: descriptor.required_metrics_for(kind).to_vec(),
            metrics_fingerprint: metrics.fingerprint(),
            style_fingerprint,
        });
    }
    let block_style_count = uses
        .iter()
        .filter(|usage| descriptor.style_kind_for(usage.kind).is_some())
        .count();
    if block_style_count != package.precomposed_vector_style_count() {
        return Err(StagingPrecomposedVectorProfileError::ReceiptMismatch);
    }
    Ok((resources, uses))
}

fn metric_payload_matches_kind(
    payload: PrecomposedVectorMetricPayload,
    kind: PrecomposedVectorKind,
) -> bool {
    matches!(
        (payload, kind),
        (
            PrecomposedVectorMetricPayload::Inline { .. },
            PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::MathVector
        ) | (
            PrecomposedVectorMetricPayload::Figure { .. },
            PrecomposedVectorKind::VectorFigure
        ) | (
            PrecomposedVectorMetricPayload::MathBlock { .. },
            PrecomposedVectorKind::MathVectorBlock
        )
    )
}

fn collect_precomposed_vector_domain_uses(
    blocks: &[StagingM4Block],
    output: &mut Vec<(NodeId, PrecomposedVectorKind, ImageResourceId)>,
) {
    for block in blocks {
        match block {
            StagingM4Block::Paragraph { inline_vectors, .. }
            | StagingM4Block::Heading { inline_vectors, .. } => {
                output.extend(inline_vectors.iter().map(|vector| {
                    let kind = match vector.kind {
                        typaxis_syntax::machine_profile_boundary::StagingM4InlineVectorKind::InlineVector => {
                            PrecomposedVectorKind::InlineVector
                        }
                        typaxis_syntax::machine_profile_boundary::StagingM4InlineVectorKind::MathVector => {
                            PrecomposedVectorKind::MathVector
                        }
                    };
                    (vector.node_id, kind, vector.image_id)
                }));
            }
            StagingM4Block::VectorFigure {
                common, image_id, ..
            } => output.push((
                common.node_id,
                PrecomposedVectorKind::VectorFigure,
                *image_id,
            )),
            StagingM4Block::MathVectorBlock {
                common, image_id, ..
            } => output.push((
                common.node_id,
                PrecomposedVectorKind::MathVectorBlock,
                *image_id,
            )),
            StagingM4Block::List { items, .. } => {
                for item in items {
                    collect_precomposed_vector_domain_uses(&item.blocks, output);
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_precomposed_vector_domain_uses(&cell.blocks, output);
                }
            }
            StagingM4Block::Figure { caption, .. } => {
                collect_precomposed_vector_domain_uses(caption, output)
            }
            StagingM4Block::SemanticContainer { blocks, .. } => {
                collect_precomposed_vector_domain_uses(blocks, output)
            }
            StagingM4Block::PageBreak { .. } | StagingM4Block::DisplayMath { .. } => {}
        }
    }
}

fn validate_existing_figure_vector_media(
    blocks: &[StagingM4Block],
    package: &ValidatedStagingSemanticPackage,
) -> Result<(), StagingPrecomposedVectorProfileError> {
    for block in blocks {
        match block {
            StagingM4Block::Figure {
                common,
                image_id,
                caption,
                ..
            } => {
                let image = package
                    .resources()
                    .images
                    .get(image_id.get() as usize)
                    .filter(|image| image.image_id == *image_id)
                    .ok_or(StagingPrecomposedVectorProfileError::MissingImage(
                        *image_id,
                    ))?;
                if image.media == ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2) {
                    return Err(
                        StagingPrecomposedVectorProfileError::ExistingFigureSvgSafe2 {
                            owner: common.node_id,
                            image: *image_id,
                        },
                    );
                }
                validate_existing_figure_vector_media(caption, package)?;
            }
            StagingM4Block::VectorFigure { caption, .. } => {
                validate_existing_figure_vector_media(caption, package)?
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    validate_existing_figure_vector_media(&item.blocks, package)?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    validate_existing_figure_vector_media(&cell.blocks, package)?;
                }
            }
            StagingM4Block::SemanticContainer { blocks, .. } => {
                validate_existing_figure_vector_media(blocks, package)?
            }
            StagingM4Block::Paragraph { .. }
            | StagingM4Block::Heading { .. }
            | StagingM4Block::PageBreak { .. }
            | StagingM4Block::DisplayMath { .. }
            | StagingM4Block::MathVectorBlock { .. } => {}
        }
    }
    Ok(())
}

fn encode_precomposed_vector_profile(
    package: &ValidatedStagingSemanticPackage,
    limits_fingerprint: [u8; 32],
    semantic_container_count: u32,
    resources: &[PrecomposedVectorResourceAuthorization],
    uses: &[PrecomposedVectorUseAuthorization],
) -> String {
    let descriptor = StagingPrecomposedVectorProfileDescriptor;
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_PRECOMPOSED_VECTOR_PROFILE_ALGORITHM);
    output.push_str(",\"contract\":\"typaxis.contract/1.4\",\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, package.canonical_jcs_sha256());
    output.push_str(",\"resource_set\":");
    push_jcs_string(&mut output, descriptor.resource_set());
    output.push_str(",\"resources\":[");
    for (index, resource) in resources.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"image_id\":");
        output.push_str(&resource.image_id.get().to_string());
        output.push_str(",\"media\":");
        push_jcs_string(&mut output, resource.media.as_str());
        output.push_str(",\"provenance\":");
        match &resource.provenance_jcs {
            Some(value) => output.push_str(value),
            None => output.push_str("null"),
        }
        output.push('}');
    }
    output.push_str("],\"safe_vector_profile\":");
    push_jcs_string(&mut output, descriptor.safe_vector_profile());
    output.push_str(",\"semantic_container_count\":");
    output.push_str(&semantic_container_count.to_string());
    output.push_str(",\"semantic_fingerprint\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push_str(",\"style_registry\":");
    push_jcs_string(&mut output, descriptor.style_registry());
    output.push_str(",\"uses\":[");
    for (index, usage) in uses.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"image_id\":");
        output.push_str(&usage.image_id.get().to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, usage.kind.as_str());
        output.push_str(",\"media\":");
        push_jcs_string(&mut output, usage.media.as_str());
        output.push_str(",\"metric_names\":[");
        for (metric_index, metric) in usage.metric_names.iter().enumerate() {
            if metric_index > 0 {
                output.push(',');
            }
            push_jcs_string(&mut output, metric.as_str());
        }
        output.push_str("],\"metrics_fingerprint\":");
        push_hash(&mut output, usage.metrics_fingerprint);
        output.push_str(",\"node_id\":");
        output.push_str(&usage.owner.get().to_string());
        output.push_str(",\"style_fingerprint\":");
        match usage.style_fingerprint {
            Some(value) => push_hash(&mut output, value),
            None => output.push_str("null"),
        }
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn first_precomposed_vector_owner(blocks: &[StagingM4Block]) -> Option<NodeId> {
    for block in blocks {
        let owner = match block {
            StagingM4Block::Paragraph { inline_vectors, .. }
            | StagingM4Block::Heading { inline_vectors, .. } => {
                inline_vectors.first().map(|vector| vector.node_id)
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

fn collect_vector_contract(
    package: &ValidatedStagingSemanticPackage,
) -> Result<(Vec<ImageResourceId>, Vec<NodeId>), StagingSafeVectorProfileError> {
    let mut vector_resource_ids = Vec::new();
    for image in &package.resources().images {
        match image.media {
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1) => {
                vector_resource_ids.push(image.image_id);
            }
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2) => {
                return Err(StagingSafeVectorProfileError::SvgSafe2Staging(
                    image.image_id,
                ));
            }
            ImageMediaDeclaration::Declared(ImageMediaType::Png)
            | ImageMediaDeclaration::LegacyUnspecified => {}
        }
    }
    let vector_ids: BTreeSet<_> = vector_resource_ids.iter().copied().collect();
    let mut figure_owners = Vec::new();
    collect_figures(
        &package.document().blocks,
        package,
        &vector_ids,
        &mut figure_owners,
    )?;
    for footnote in &package.document().footnotes {
        collect_figures(&footnote.blocks, package, &vector_ids, &mut figure_owners)?;
    }
    Ok((vector_resource_ids, figure_owners))
}

fn collect_figures(
    blocks: &[StagingM4Block],
    package: &ValidatedStagingSemanticPackage,
    vector_ids: &BTreeSet<ImageResourceId>,
    output: &mut Vec<NodeId>,
) -> Result<(), StagingSafeVectorProfileError> {
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
                    .ok_or(StagingSafeVectorProfileError::MissingImage(*image_id))?;
                if declaration.media == ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1) {
                    if !vector_ids.contains(image_id) {
                        return Err(StagingSafeVectorProfileError::WrongMedia(*image_id));
                    }
                    output.push(common.node_id);
                }
                collect_figures(caption, package, vector_ids, output)?;
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    collect_figures(&item.blocks, package, vector_ids, output)?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_figures(&cell.blocks, package, vector_ids, output)?;
                }
            }
            StagingM4Block::SemanticContainer { blocks, .. } => {
                collect_figures(blocks, package, vector_ids, output)?;
            }
            StagingM4Block::VectorFigure { common, .. }
            | StagingM4Block::MathVectorBlock { common, .. } => {
                return Err(StagingSafeVectorProfileError::PrecomposedVectorStaging(
                    common.node_id,
                ));
            }
            StagingM4Block::Paragraph { inline_vectors, .. }
            | StagingM4Block::Heading { inline_vectors, .. } => {
                if let Some(vector) = inline_vectors.first() {
                    return Err(StagingSafeVectorProfileError::PrecomposedVectorStaging(
                        vector.node_id,
                    ));
                }
            }
            StagingM4Block::PageBreak { .. } | StagingM4Block::DisplayMath { .. } => {}
        }
    }
    Ok(())
}

fn encode(
    package: &ValidatedStagingSemanticPackage,
    base_profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    resources: &[ImageResourceId],
    figures: &[NodeId],
    page_geometry: [u8; 32],
    math_extension: bool,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_SAFE_VECTOR_PROFILE_ALGORITHM);
    output.push_str(",\"base_profile_fingerprint\":");
    push_hash(&mut output, base_profile_fingerprint);
    output.push_str(",\"figure_owners\":[");
    for (index, owner) in figures.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&owner.get().to_string());
    }
    output.push_str("],\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    if math_extension {
        output.push_str(",\"math_extension\":\"typaxis.production-book-math-profile/1\"");
    }
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
    use typaxis_core::{M4ResourceLimits, ResourceLimits, ValidatedResourceLimits};
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder, WireImageMediaType, WireStagingM4Block,
        WireStagingM4Inline, WireStagingStyleDeclaration, WireStagingStyleRule,
        WireStagingStyleSheet, WireStagingStyleValue, WireVectorProvenance,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    const VECTOR_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vector-media/job/document-package.json"
    ));
    const PRECOMPOSED_VECTOR_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"
    ));

    fn fixture() -> (ValidatedStagingSemanticPackage, M4EffectiveResourceLimits) {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(VECTOR_FIXTURE, &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &base)
            .unwrap();
        let limits = M4EffectiveResourceLimits::new(base, M4ResourceLimits::default()).unwrap();
        (package, limits)
    }

    fn precomposed_fixture_with(
        mutate: impl FnOnce(
            &mut typaxis_syntax::machine_profile_boundary::wire::WireStagingM4DocumentPackage,
        ),
    ) -> (ValidatedStagingSemanticPackage, M4EffectiveResourceLimits) {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                PRECOMPOSED_VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(&base),
            )
            .unwrap();
        let mut wire = decoded.into_wire();
        mutate(&mut wire);
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(encoded.as_bytes(), &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &base)
            .unwrap();
        let limits = M4EffectiveResourceLimits::new(base, M4ResourceLimits::default()).unwrap();
        (package, limits)
    }

    #[test]
    fn vector_media_profile_preflight_is_session_and_limit_bound() {
        let (package, limits) = fixture();
        let session = StagingSemanticContainerSessionIdentity::fresh();
        let receipt = preflight_staging_safe_vector_profile(&package, &limits, &session).unwrap();
        assert_eq!(
            receipt.vector_resource_ids(),
            [ImageResourceId::new(0), ImageResourceId::new(1)]
        );
        assert_eq!(receipt.figure_owners(), [NodeId::new(2)]);
        receipt.verify(&package, &limits, &session).unwrap();
        assert!(receipt
            .canonical_jcs()
            .contains(STAGING_SAFE_VECTOR_PROFILE_ALGORITHM));

        let changed = M4EffectiveResourceLimits::new(
            limits.base().clone(),
            M4ResourceLimits {
                max_vector_nodes: limits.extension().get().max_vector_nodes - 1,
                ..M4ResourceLimits::default()
            },
        )
        .unwrap();
        assert_eq!(
            receipt.authorizes(&package, &changed),
            Err(StagingSafeVectorProfileError::ReceiptMismatch)
        );
        assert_eq!(
            receipt.verify(
                &package,
                &limits,
                &StagingSemanticContainerSessionIdentity::fresh()
            ),
            Err(StagingSafeVectorProfileError::ReceiptMismatch)
        );
        assert_eq!(
            typaxis_core::DocumentPackageContractId::CURRENT.as_str(),
            "typaxis.contract/1.3"
        );
    }

    #[test]
    fn vector_media_profile_rejects_precomposed_kind_and_media_before_base_profile() {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                PRECOMPOSED_VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(&base),
            )
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &base)
            .unwrap();
        let limits =
            M4EffectiveResourceLimits::new(base.clone(), M4ResourceLimits::default()).unwrap();
        assert!(matches!(
            preflight_staging_safe_vector_profile(
                &package,
                &limits,
                &StagingSemanticContainerSessionIdentity::fresh(),
            ),
            Err(StagingSafeVectorProfileError::SvgSafe2Staging(id))
                if id == ImageResourceId::new(0)
        ));

        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                PRECOMPOSED_VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(&base),
            )
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut resources = wire.resources().clone();
        resources.images[0].media_type = WireImageMediaType::SvgSafe1;
        resources.images[0].vector_provenance = None;
        wire.replace_typed_regions(wire.document().clone(), resources);
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(encoded.as_bytes(), &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &base)
            .unwrap();
        assert!(matches!(
            preflight_staging_safe_vector_profile(
                &package,
                &limits,
                &StagingSemanticContainerSessionIdentity::fresh(),
            ),
            Err(StagingSafeVectorProfileError::PrecomposedVectorStaging(owner))
                if owner == NodeId::new(3)
        ));
    }

    #[test]
    fn precomposed_vector_profile_closes_kind_media_metrics_styles_and_identities() {
        let (all_safe_2, all_safe_2_limits) = precomposed_fixture_with(|_| {});
        preflight_staging_precomposed_vector_profile(
            &all_safe_2,
            &all_safe_2_limits,
            &StagingSemanticContainerSessionIdentity::fresh(),
        )
        .unwrap();

        let (package, limits) = precomposed_fixture_with(|wire| {
            let mut resources = wire.resources().clone();
            let mut safe_1 = resources.images[0].clone();
            safe_1.image_id = 1;
            safe_1.media_type = WireImageMediaType::SvgSafe1;
            safe_1.vector_provenance = None;
            resources.images.push(safe_1);
            let mut document = wire.document().clone();
            let WireStagingM4Block::SemanticContainer { blocks, .. } = &mut document.blocks[0]
            else {
                panic!("fixture root must be a semantic container");
            };
            let WireStagingM4Block::Paragraph { children, .. } = &mut blocks[0] else {
                panic!("fixture first child must be a paragraph");
            };
            let WireStagingM4Inline::InlineVector { image_id, .. } = &mut children[0] else {
                panic!("fixture first inline must be an inline vector");
            };
            *image_id = 1;
            let WireStagingM4Block::VectorFigure { image_id, .. } = &mut blocks[1] else {
                panic!("fixture second child must be a vector figure");
            };
            *image_id = 1;
            wire.replace_typed_regions(document, resources);
            wire.replace_style_sheet(WireStagingStyleSheet {
                rules: vec![
                    WireStagingStyleRule {
                        style_id: "math-vector".to_owned(),
                        extends: None,
                        selector: "math_vector_block".to_owned(),
                        source_order: 0,
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
                                value: WireStagingStyleValue::Length { value: 655_360 },
                                important: false,
                            },
                            WireStagingStyleDeclaration {
                                name: "line_height".to_owned(),
                                value: WireStagingStyleValue::Length { value: 786_432 },
                                important: false,
                            },
                            WireStagingStyleDeclaration {
                                name: "text_align".to_owned(),
                                value: WireStagingStyleValue::Keyword {
                                    value: "center".to_owned(),
                                },
                                important: false,
                            },
                        ],
                    },
                    WireStagingStyleRule {
                        style_id: "vector-figure".to_owned(),
                        extends: None,
                        selector: "vector_figure".to_owned(),
                        source_order: 1,
                        declarations: vec![WireStagingStyleDeclaration {
                            name: "keep_caption".to_owned(),
                            value: WireStagingStyleValue::Boolean { value: false },
                            important: false,
                        }],
                    },
                ],
            });
        });
        let session = StagingSemanticContainerSessionIdentity::fresh();
        let receipt =
            preflight_staging_precomposed_vector_profile(&package, &limits, &session).unwrap();
        assert_eq!(
            receipt.vector_owners().collect::<Vec<_>>(),
            [
                NodeId::new(3),
                NodeId::new(4),
                NodeId::new(5),
                NodeId::new(6)
            ]
        );
        assert_eq!(
            receipt.vector_resource_ids().collect::<Vec<_>>(),
            [ImageResourceId::new(0), ImageResourceId::new(1)]
        );
        assert!(receipt
            .canonical_jcs()
            .contains(STAGING_PRODUCTION_BOOK_RESOURCE_SET_V2));
        assert!(receipt
            .canonical_jcs()
            .contains(STAGING_SAFE_VECTOR_PROFILE_V2));
        assert!(receipt
            .canonical_jcs()
            .contains("\"metric_names\":[\"advance\",\"ascent\",\"baseline\",\"descent\",\"origin_x\",\"viewport\"]"));
        assert!(receipt
            .canonical_jcs()
            .contains("\"metric_names\":[\"viewport\"]"));
        assert_eq!(
            receipt.authorization().profile_receipt_fingerprint(),
            receipt.fingerprint()
        );
        assert_eq!(
            receipt.authorization().vector_owners().collect::<Vec<_>>(),
            receipt.vector_owners().collect::<Vec<_>>()
        );
        receipt
            .authorization()
            .authorizes(&package, &limits)
            .unwrap();
        receipt.verify(&package, &limits, &session).unwrap();
        assert_eq!(
            receipt.verify(
                &package,
                &limits,
                &StagingSemanticContainerSessionIdentity::fresh()
            ),
            Err(StagingPrecomposedVectorProfileError::ReceiptMismatch)
        );

        let foreign_session = StagingSemanticContainerSessionIdentity::fresh();
        let foreign =
            preflight_staging_precomposed_vector_profile(&package, &limits, &foreign_session)
                .unwrap();
        let mut swapped =
            preflight_staging_precomposed_vector_profile(&package, &limits, &session).unwrap();
        swapped.authorization = foreign.authorization;
        assert_eq!(
            swapped.verify(&package, &limits, &session),
            Err(StagingPrecomposedVectorProfileError::ReceiptMismatch)
        );

        let math = package.precomposed_vector_style(NodeId::new(6)).unwrap();
        assert_eq!(math.kind(), PrecomposedVectorStyleKind::MathVectorBlock);
        assert_eq!(
            math.equation_number_text_style()
                .unwrap()
                .font_families()
                .unwrap(),
            ["Math"]
        );
        assert_eq!(
            math.text_align(),
            typaxis_syntax::machine_profile_boundary::MachineTextAlign::Center
        );
        let figure = package.precomposed_vector_style(NodeId::new(5)).unwrap();
        assert_eq!(figure.keep_caption(), Some(false));

        let descriptor = StagingPrecomposedVectorProfileDescriptor;
        for kind in descriptor.kinds() {
            for media in [
                ImageMediaType::Png,
                ImageMediaType::SvgSafe1,
                ImageMediaType::SvgSafe2,
            ] {
                let expected = matches!(
                    (kind, media),
                    (
                        PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::VectorFigure,
                        ImageMediaType::SvgSafe1 | ImageMediaType::SvgSafe2
                    ) | (
                        PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock,
                        ImageMediaType::SvgSafe2
                    )
                );
                assert_eq!(descriptor.media_for(kind).contains(&media), expected);
            }
        }
    }

    #[test]
    fn precomposed_vector_profile_rejects_math_safe1_and_existing_figure_safe2() {
        let (package, limits) = precomposed_fixture_with(|wire| {
            let mut resources = wire.resources().clone();
            resources.images[0].media_type = WireImageMediaType::SvgSafe1;
            resources.images[0].vector_provenance = None;
            wire.replace_typed_regions(wire.document().clone(), resources);
        });
        assert!(matches!(
            preflight_staging_precomposed_vector_profile(
                &package,
                &limits,
                &StagingSemanticContainerSessionIdentity::fresh()
            ),
            Err(StagingPrecomposedVectorProfileError::KindMediaMismatch { owner, image })
                if owner == NodeId::new(4) && image == ImageResourceId::new(0)
        ));

        let (existing_safe_1, existing_safe_1_limits) = fixture();
        preflight_staging_precomposed_vector_profile(
            &existing_safe_1,
            &existing_safe_1_limits,
            &StagingSemanticContainerSessionIdentity::fresh(),
        )
        .unwrap();

        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(VECTOR_FIXTURE, &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut resources = wire.resources().clone();
        resources.images[0].media_type = WireImageMediaType::SvgSafe2;
        resources.images[0].vector_provenance = Some(WireVectorProvenance {
            engine_id: "vmb.texToSvg".to_owned(),
            engine_version: "2026.09.0".to_owned(),
            rules_version: "vmb.math-safe-svg/1".to_owned(),
        });
        wire.replace_typed_regions(wire.document().clone(), resources);
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(encoded.as_bytes(), &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &base)
            .unwrap();
        let limits = M4EffectiveResourceLimits::new(base, M4ResourceLimits::default()).unwrap();
        assert!(matches!(
            preflight_staging_precomposed_vector_profile(
                &package,
                &limits,
                &StagingSemanticContainerSessionIdentity::fresh()
            ),
            Err(StagingPrecomposedVectorProfileError::ExistingFigureSvgSafe2 { owner, image })
                if owner == NodeId::new(2) && image == ImageResourceId::new(0)
        ));
    }

    #[test]
    fn precomposed_vector_profile_keeps_new_style_out_of_old_profile() {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(VECTOR_FIXTURE, &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut sheet = wire.style_sheet().clone();
        sheet.rules.push(WireStagingStyleRule {
            style_id: "future-vector".to_owned(),
            extends: None,
            selector: "vector_figure".to_owned(),
            source_order: u32::try_from(sheet.rules.len()).unwrap(),
            declarations: vec![WireStagingStyleDeclaration {
                name: "space_before".to_owned(),
                value: WireStagingStyleValue::Length { value: 0 },
                important: false,
            }],
        });
        wire.replace_style_sheet(sheet);
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(encoded.as_bytes(), &DocumentPackageDecodePolicy::new(&base))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &base)
            .unwrap();
        assert!(matches!(
            preflight_staging_semantic_container_profile(
                &package,
                &base,
                &StagingSemanticContainerSessionIdentity::fresh()
            ),
            Err(crate::StagingSemanticContainerPreflightError::PrecomposedVectorStyleStaging)
        ));
    }

    #[test]
    fn precomposed_vector_profile_rejects_style_applicability_and_bounds_before_preflight() {
        for (property, value) in [
            ("width", WireStagingStyleValue::Length { value: 1 }),
            (
                "keep_caption",
                WireStagingStyleValue::Boolean { value: true },
            ),
        ] {
            let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
            let decoded = StagingSemanticDocumentPackageDecoder::new()
                .decode(
                    PRECOMPOSED_VECTOR_FIXTURE,
                    &DocumentPackageDecodePolicy::new(&base),
                )
                .unwrap();
            let mut wire = decoded.into_wire();
            wire.replace_style_sheet(WireStagingStyleSheet {
                rules: vec![WireStagingStyleRule {
                    style_id: "invalid-math-vector".to_owned(),
                    extends: None,
                    selector: "math_vector_block".to_owned(),
                    source_order: 0,
                    declarations: vec![WireStagingStyleDeclaration {
                        name: property.to_owned(),
                        value,
                        important: false,
                    }],
                }],
            });
            let encoded = StagingSemanticDocumentPackageEncoder::new()
                .encode(&wire)
                .unwrap();
            let decoded = StagingSemanticDocumentPackageDecoder::new()
                .decode(encoded.as_bytes(), &DocumentPackageDecodePolicy::new(&base))
                .unwrap();
            let error = StagingSemanticPackageParser::new()
                .parse(decoded, &base)
                .unwrap_err();
            assert_eq!(
                error,
                typaxis_syntax::StagingSemanticSyntaxError::InapplicableStyle
            );
            assert!(error.to_string().starts_with("L5101:"));
        }

        for (selector, property, value) in [
            (
                "unknown_vector_block",
                "space_before",
                WireStagingStyleValue::Length { value: 0 },
            ),
            (
                "math_vector_block",
                "unknown_vector_property",
                WireStagingStyleValue::Length { value: 0 },
            ),
        ] {
            let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
            let decoded = StagingSemanticDocumentPackageDecoder::new()
                .decode(
                    PRECOMPOSED_VECTOR_FIXTURE,
                    &DocumentPackageDecodePolicy::new(&base),
                )
                .unwrap();
            let mut wire = decoded.into_wire();
            wire.replace_style_sheet(WireStagingStyleSheet {
                rules: vec![WireStagingStyleRule {
                    style_id: "unknown-vector-style".to_owned(),
                    extends: None,
                    selector: selector.to_owned(),
                    source_order: 0,
                    declarations: vec![WireStagingStyleDeclaration {
                        name: property.to_owned(),
                        value,
                        important: false,
                    }],
                }],
            });
            let encoded = StagingSemanticDocumentPackageEncoder::new()
                .encode(&wire)
                .unwrap();
            match StagingSemanticDocumentPackageDecoder::new()
                .decode(encoded.as_bytes(), &DocumentPackageDecodePolicy::new(&base))
            {
                Err(_) => {}
                Ok(decoded) => assert_eq!(
                    StagingSemanticPackageParser::new()
                        .parse(decoded, &base)
                        .unwrap_err(),
                    typaxis_syntax::StagingSemanticSyntaxError::InvalidStyle
                ),
            }
        }

        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                PRECOMPOSED_VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(&base),
            )
            .unwrap();
        let mut wire = decoded.into_wire();
        wire.replace_style_sheet(WireStagingStyleSheet {
            rules: vec![WireStagingStyleRule {
                style_id: "max-plus-one".to_owned(),
                extends: None,
                selector: "math_vector_block".to_owned(),
                source_order: 0,
                declarations: vec![WireStagingStyleDeclaration {
                    name: "space_before".to_owned(),
                    value: WireStagingStyleValue::Length {
                        value: typaxis_core::JSON_SAFE_INTEGER_MAX + 1,
                    },
                    important: false,
                }],
            }],
        });
        match StagingSemanticDocumentPackageEncoder::new().encode(&wire) {
            Err(_) => {}
            Ok(encoded) => match StagingSemanticDocumentPackageDecoder::new()
                .decode(encoded.as_bytes(), &DocumentPackageDecodePolicy::new(&base))
            {
                Err(_) => {}
                Ok(decoded) => assert_eq!(
                    StagingSemanticPackageParser::new()
                        .parse(decoded, &base)
                        .unwrap_err(),
                    typaxis_syntax::StagingSemanticSyntaxError::InvalidStyle
                ),
            },
        }
    }
}
