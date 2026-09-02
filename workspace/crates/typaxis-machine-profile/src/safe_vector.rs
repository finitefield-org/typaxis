use std::collections::BTreeSet;
use typaxis_core::{push_jcs_string, sha256, ImageResourceId, M4EffectiveResourceLimits, NodeId};
use typaxis_syntax::machine_profile_boundary::{
    ImageMediaDeclaration, ImageMediaType, StagingM4Block,
};
use typaxis_syntax::{StagingSafeVectorProfileView, ValidatedStagingSemanticPackage};

use crate::{
    preflight_staging_semantic_container_profile,
    semantic_container::preflight_staging_semantic_container_profile_for_math,
    StagingSemanticContainerPreflightReceipt, StagingSemanticContainerSessionIdentity,
};

pub const STAGING_SAFE_VECTOR_PROFILE_ALGORITHM: &str =
    "typaxis.production-book-safe-vector-profile/1";

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
        StagingSemanticDocumentPackageEncoder, WireImageMediaType,
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
}
