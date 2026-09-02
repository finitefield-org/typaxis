use std::sync::Arc;
use typaxis_core::{ImageResourceId, NodeId, ValidatedResourceLimits};
use typaxis_syntax::machine_profile_boundary::{
    BasicStyleProperty, FontMediaDeclaration, FontMediaType, ImageMediaDeclaration, ImageMediaType,
    SemanticContainerKind, SemanticContainerStyleKind, StagingM4Block,
    ValidatedStagingSemanticPackage,
};
use typaxis_syntax::{StagingMathProfileSessionIdentity, StagingSemanticContainerProfileView};

pub const STAGING_PRODUCTION_BOOK_PROFILE_ID: &str = "typaxis.machine-pdf/production-book-1";
pub const STAGING_PRODUCTION_BOOK_PROFILE_RECEIPT_ALGORITHM: &str =
    "typaxis.production-book-profile-receipt/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerProfileDescriptor;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSemanticContainerParentKind {
    DocumentBody,
    SemanticContainer,
    ListItem,
    TableCell,
    FigureCaption,
    FootnoteDefinition,
}

impl StagingSemanticContainerProfileDescriptor {
    pub const PROFILE_ID: &'static str = STAGING_PRODUCTION_BOOK_PROFILE_ID;
    pub const CONTRACT: &'static str = "typaxis.contract/1.4";

    pub const fn semantic_kinds(self) -> [SemanticContainerKind; 3] {
        [
            SemanticContainerKind::Result,
            SemanticContainerKind::Proof,
            SemanticContainerKind::Exercise,
        ]
    }

    pub const fn image_media(self) -> [ImageMediaType; 2] {
        [ImageMediaType::Png, ImageMediaType::SvgSafe1]
    }

    pub const fn font_media(self) -> [FontMediaType; 2] {
        [
            FontMediaType::SfntTrueTypeGlyf,
            FontMediaType::TtcTrueTypeGlyf,
        ]
    }

    pub const fn supports_outline_entries_for_containers(self) -> bool {
        false
    }

    pub const fn selector(self) -> &'static str {
        "semantic_container"
    }

    pub const fn parent_kinds(self) -> [StagingSemanticContainerParentKind; 6] {
        [
            StagingSemanticContainerParentKind::DocumentBody,
            StagingSemanticContainerParentKind::SemanticContainer,
            StagingSemanticContainerParentKind::ListItem,
            StagingSemanticContainerParentKind::TableCell,
            StagingSemanticContainerParentKind::FigureCaption,
            StagingSemanticContainerParentKind::FootnoteDefinition,
        ]
    }

    pub const fn style_properties(self) -> [BasicStyleProperty; 10] {
        [
            BasicStyleProperty::FontFamily,
            BasicStyleProperty::FontSize,
            BasicStyleProperty::LineHeight,
            BasicStyleProperty::Page,
            BasicStyleProperty::SpaceBefore,
            BasicStyleProperty::SpaceAfter,
            BasicStyleProperty::StartIndent,
            BasicStyleProperty::EndIndent,
            BasicStyleProperty::TextAlign,
            BasicStyleProperty::KeepWithNext,
        ]
    }
}

#[derive(Debug)]
struct StagingSemanticContainerSessionState {
    math_profile: StagingMathProfileSessionIdentity,
}

#[derive(Clone)]
pub struct StagingSemanticContainerSessionIdentity(Arc<StagingSemanticContainerSessionState>);

impl StagingSemanticContainerSessionIdentity {
    pub fn fresh() -> Self {
        Self(Arc::new(StagingSemanticContainerSessionState {
            math_profile: StagingMathProfileSessionIdentity::fresh(),
        }))
    }

    pub(crate) fn math_profile_session(&self) -> &StagingMathProfileSessionIdentity {
        &self.0.math_profile
    }
}

impl std::fmt::Debug for StagingSemanticContainerSessionIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StagingSemanticContainerSessionIdentity(..)")
    }
}

impl PartialEq for StagingSemanticContainerSessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for StagingSemanticContainerSessionIdentity {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSemanticContainerPreflightError {
    UnsupportedKind(NodeId),
    UnsupportedParent(NodeId),
    EmptyContainer(NodeId),
    MissingDeclaration,
    DisallowedMedia,
    StyleMismatch(NodeId),
    UnsupportedMath,
    PrecomposedVectorStaging(NodeId),
    SvgSafe2Staging(ImageResourceId),
    ReceiptMismatch,
}

impl std::fmt::Display for StagingSemanticContainerPreflightError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedKind(owner) => write!(
                formatter,
                "L5100: unsupported semantic container kind at node {}",
                owner.get()
            ),
            Self::UnsupportedParent(owner) => write!(
                formatter,
                "L5100: unsupported semantic container parent at node {}",
                owner.get()
            ),
            Self::EmptyContainer(owner) => write!(
                formatter,
                "L5100: recursively empty semantic container at node {}",
                owner.get()
            ),
            Self::MissingDeclaration => {
                formatter.write_str("R7100: production-book media declaration is missing")
            }
            Self::DisallowedMedia => {
                formatter.write_str("R7100: production-book media declaration is disallowed")
            }
            Self::StyleMismatch(owner) => write!(
                formatter,
                "L5101: semantic container style mismatch at node {}",
                owner.get()
            ),
            Self::UnsupportedMath => {
                formatter.write_str("L5100: semantic-container profile does not admit math")
            }
            Self::PrecomposedVectorStaging(owner) => write!(
                formatter,
                "P1102: precomposed vector at node {} requires its versioned profile",
                owner.get()
            ),
            Self::SvgSafe2Staging(id) => write!(
                formatter,
                "P1102: svg-safe-2 image {} requires its versioned profile",
                id.get()
            ),
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: semantic profile receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingSemanticContainerPreflightError {}

#[derive(Debug)]
pub struct StagingSemanticContainerPreflightReceipt {
    package_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    limits: ValidatedResourceLimits,
    session: StagingSemanticContainerSessionIdentity,
    container_count: u32,
    math_extension: bool,
    book_navigation_extension: bool,
    authorization: StagingSemanticContainerProfileView,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSemanticContainerPreflightReceipt {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn semantic_fingerprint(&self) -> [u8; 32] {
        self.semantic_fingerprint
    }
    pub const fn container_count(&self) -> u32 {
        self.container_count
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub const fn authorization(&self) -> &StagingSemanticContainerProfileView {
        &self.authorization
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        limits: &ValidatedResourceLimits,
        session: &StagingSemanticContainerSessionIdentity,
    ) -> Result<(), StagingSemanticContainerPreflightError> {
        let authorization = StagingSemanticContainerProfileView::new(package, limits)
            .map_err(|_| StagingSemanticContainerPreflightError::ReceiptMismatch)?;
        let canonical = authorization.canonical_jcs();
        let has_math = !package.math_nodes().is_empty();
        if self.math_extension != has_math
            || (!self.book_navigation_extension && !has_neutral_book_navigation(package))
            || self.package_sha256 != package.canonical_jcs_sha256()
            || self.semantic_fingerprint != package.semantic_fingerprint()
            || self.limits != *limits
            || self.session != *session
            || self.container_count != authorization.container_count()
            || self.authorization != authorization
            || canonical != self.canonical_jcs
            || authorization.profile_fingerprint() != self.fingerprint
        {
            return Err(StagingSemanticContainerPreflightError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn preflight_staging_semantic_container_profile(
    package: &ValidatedStagingSemanticPackage,
    limits: &ValidatedResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingSemanticContainerPreflightReceipt, StagingSemanticContainerPreflightError> {
    reject_precomposed_vector_staging(package)?;
    if !package.math_nodes().is_empty() {
        return Err(StagingSemanticContainerPreflightError::UnsupportedMath);
    }
    if !has_neutral_book_navigation(package) {
        return Err(StagingSemanticContainerPreflightError::ReceiptMismatch);
    }
    preflight_staging_semantic_container_profile_inner(package, limits, session, false, false)
}

pub(crate) fn preflight_staging_semantic_container_profile_for_math(
    package: &ValidatedStagingSemanticPackage,
    limits: &ValidatedResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingSemanticContainerPreflightReceipt, StagingSemanticContainerPreflightError> {
    reject_precomposed_vector_staging(package)?;
    if package.math_nodes().is_empty() {
        return Err(StagingSemanticContainerPreflightError::UnsupportedMath);
    }
    if !has_neutral_book_navigation(package) {
        return Err(StagingSemanticContainerPreflightError::ReceiptMismatch);
    }
    preflight_staging_semantic_container_profile_inner(package, limits, session, true, false)
}

pub(crate) fn preflight_staging_semantic_container_profile_for_book_navigation(
    package: &ValidatedStagingSemanticPackage,
    limits: &ValidatedResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingSemanticContainerPreflightReceipt, StagingSemanticContainerPreflightError> {
    reject_precomposed_vector_staging(package)?;
    if !package.math_nodes().is_empty() {
        return Err(StagingSemanticContainerPreflightError::UnsupportedMath);
    }
    preflight_staging_semantic_container_profile_inner(package, limits, session, false, true)
}

pub(crate) fn preflight_staging_semantic_container_profile_for_tagged_pdf(
    package: &ValidatedStagingSemanticPackage,
    limits: &ValidatedResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingSemanticContainerPreflightReceipt, StagingSemanticContainerPreflightError> {
    reject_precomposed_vector_staging(package)?;
    preflight_staging_semantic_container_profile_inner(
        package,
        limits,
        session,
        !package.math_nodes().is_empty(),
        true,
    )
}

fn preflight_staging_semantic_container_profile_inner(
    package: &ValidatedStagingSemanticPackage,
    limits: &ValidatedResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
    math_extension: bool,
    book_navigation_extension: bool,
) -> Result<StagingSemanticContainerPreflightReceipt, StagingSemanticContainerPreflightError> {
    if package.limits() != limits {
        return Err(StagingSemanticContainerPreflightError::ReceiptMismatch);
    }
    package
        .checked_wire()
        .map_err(|_| StagingSemanticContainerPreflightError::ReceiptMismatch)?;
    validate_media_declarations(package.resources())?;
    let mut count = 0u32;
    validate_blocks(
        package,
        &package.document().blocks,
        StagingSemanticContainerParentKind::DocumentBody,
        &mut count,
    )?;
    for footnote in &package.document().footnotes {
        validate_blocks(
            package,
            &footnote.blocks,
            StagingSemanticContainerParentKind::FootnoteDefinition,
            &mut count,
        )?;
    }
    let authorization = StagingSemanticContainerProfileView::new(package, limits)
        .map_err(|_| StagingSemanticContainerPreflightError::ReceiptMismatch)?;
    if authorization.container_count() != count {
        return Err(StagingSemanticContainerPreflightError::ReceiptMismatch);
    }
    let canonical_jcs = authorization.canonical_jcs().to_owned();
    Ok(StagingSemanticContainerPreflightReceipt {
        package_sha256: package.canonical_jcs_sha256(),
        semantic_fingerprint: package.semantic_fingerprint(),
        limits: limits.clone(),
        session: session.clone(),
        container_count: count,
        math_extension,
        book_navigation_extension,
        fingerprint: authorization.profile_fingerprint(),
        authorization,
        canonical_jcs,
    })
}

fn reject_precomposed_vector_staging(
    package: &ValidatedStagingSemanticPackage,
) -> Result<(), StagingSemanticContainerPreflightError> {
    if let Some(image) = package
        .resources()
        .images
        .iter()
        .find(|image| image.media == ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2))
    {
        return Err(StagingSemanticContainerPreflightError::SvgSafe2Staging(
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
        return Err(StagingSemanticContainerPreflightError::PrecomposedVectorStaging(owner));
    }
    Ok(())
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

fn has_neutral_book_navigation(package: &ValidatedStagingSemanticPackage) -> bool {
    use typaxis_syntax::machine_profile_boundary::wire::{
        WireStagingM4Block as WireBlock, WireStagingM4Inline as WireInline,
    };

    fn inlines(values: &[WireInline]) -> bool {
        values.iter().all(|value| {
            let neutral = value.language().is_none();
            neutral
                && match value {
                    WireInline::Emphasis { children, .. }
                    | WireInline::Strong { children, .. }
                    | WireInline::Link { children, .. } => inlines(children),
                    WireInline::InlineVector { .. } | WireInline::MathVector { .. } => false,
                    WireInline::Text { .. }
                    | WireInline::InlineMath { .. }
                    | WireInline::Anchor { .. }
                    | WireInline::Reference { .. }
                    | WireInline::FootnoteReference { .. }
                    | WireInline::SoftBreak { .. }
                    | WireInline::HardBreak { .. } => true,
                }
        })
    }

    fn blocks(values: &[WireBlock]) -> bool {
        values.iter().all(|value| {
            if value.language().is_some() {
                return false;
            }
            match value {
                WireBlock::Paragraph { children, .. } | WireBlock::Heading { children, .. } => {
                    inlines(children)
                }
                WireBlock::List { items, .. } => items
                    .iter()
                    .all(|item| item.language.is_none() && blocks(&item.blocks)),
                WireBlock::Table { head, body, .. } => head.iter().chain(body).all(|row| {
                    row.language.is_none()
                        && row
                            .cells
                            .iter()
                            .all(|cell| cell.language.is_none() && blocks(&cell.blocks))
                }),
                WireBlock::Figure { caption, .. } => blocks(caption),
                WireBlock::SemanticContainer {
                    anchor_id,
                    blocks: children,
                    ..
                } => anchor_id.is_none() && blocks(children),
                WireBlock::VectorFigure { .. } | WireBlock::MathVectorBlock { .. } => false,
                WireBlock::PageBreak { .. } | WireBlock::DisplayMath { .. } => true,
            }
        })
    }

    let Ok(wire) = package.checked_wire() else {
        return false;
    };
    let metadata = wire.metadata();
    metadata.author.is_none()
        && metadata.created.is_none()
        && metadata.identifier.is_none()
        && metadata.keywords.is_empty()
        && metadata.modified.is_none()
        && metadata.subject.is_none()
        && metadata.title.is_none()
        && wire.document().language == "und"
        && wire.outline().entries.is_empty()
        && blocks(&wire.document().blocks)
        && wire
            .document()
            .footnotes
            .iter()
            .all(|footnote| footnote.language.is_none() && blocks(&footnote.blocks))
}

fn validate_media_declarations(
    resources: &typaxis_syntax::machine_profile_boundary::StagingM4ResourceCatalog,
) -> Result<(), StagingSemanticContainerPreflightError> {
    for font in &resources.font_faces {
        match font.media {
            FontMediaDeclaration::Declared(media)
                if StagingSemanticContainerProfileDescriptor
                    .font_media()
                    .contains(&media) => {}
            FontMediaDeclaration::Declared(_) => {
                return Err(StagingSemanticContainerPreflightError::DisallowedMedia)
            }
            FontMediaDeclaration::LegacyUnspecified => {
                return Err(StagingSemanticContainerPreflightError::MissingDeclaration)
            }
        }
    }
    for image in &resources.images {
        match image.media {
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2) => {
                return Err(StagingSemanticContainerPreflightError::SvgSafe2Staging(
                    image.image_id,
                ));
            }
            ImageMediaDeclaration::Declared(media)
                if StagingSemanticContainerProfileDescriptor
                    .image_media()
                    .contains(&media) => {}
            ImageMediaDeclaration::Declared(_) => {
                return Err(StagingSemanticContainerPreflightError::DisallowedMedia)
            }
            ImageMediaDeclaration::LegacyUnspecified => {
                return Err(StagingSemanticContainerPreflightError::MissingDeclaration)
            }
        }
    }
    Ok(())
}

fn validate_blocks(
    package: &ValidatedStagingSemanticPackage,
    blocks: &[StagingM4Block],
    parent_kind: StagingSemanticContainerParentKind,
    count: &mut u32,
) -> Result<(), StagingSemanticContainerPreflightError> {
    for block in blocks {
        match block {
            StagingM4Block::SemanticContainer {
                common,
                semantic_kind,
                blocks,
            } => {
                if !StagingSemanticContainerProfileDescriptor
                    .semantic_kinds()
                    .contains(semantic_kind)
                {
                    return Err(StagingSemanticContainerPreflightError::UnsupportedKind(
                        common.node_id,
                    ));
                }
                if !StagingSemanticContainerProfileDescriptor
                    .parent_kinds()
                    .contains(&parent_kind)
                {
                    return Err(StagingSemanticContainerPreflightError::UnsupportedParent(
                        common.node_id,
                    ));
                }
                *count = count.checked_add(1).ok_or(
                    StagingSemanticContainerPreflightError::UnsupportedKind(common.node_id),
                )?;
                let style = package.computed_style(common.node_id).ok_or(
                    StagingSemanticContainerPreflightError::StyleMismatch(common.node_id),
                )?;
                let expected_style_kind = match semantic_kind {
                    SemanticContainerKind::Result => SemanticContainerStyleKind::Result,
                    SemanticContainerKind::Proof => SemanticContainerStyleKind::Proof,
                    SemanticContainerKind::Exercise => SemanticContainerStyleKind::Exercise,
                };
                if style.semantic_kind() != expected_style_kind {
                    return Err(StagingSemanticContainerPreflightError::StyleMismatch(
                        common.node_id,
                    ));
                }
                if !blocks.iter().any(StagingM4Block::is_semantically_nonempty) {
                    return Err(StagingSemanticContainerPreflightError::EmptyContainer(
                        common.node_id,
                    ));
                }
                validate_blocks(
                    package,
                    blocks,
                    StagingSemanticContainerParentKind::SemanticContainer,
                    count,
                )?;
            }
            StagingM4Block::List { items, .. } => {
                for item in items {
                    validate_blocks(
                        package,
                        &item.blocks,
                        StagingSemanticContainerParentKind::ListItem,
                        count,
                    )?;
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    validate_blocks(
                        package,
                        &cell.blocks,
                        StagingSemanticContainerParentKind::TableCell,
                        count,
                    )?;
                }
            }
            StagingM4Block::Figure { caption, .. } => {
                validate_blocks(
                    package,
                    caption,
                    StagingSemanticContainerParentKind::FigureCaption,
                    count,
                )?;
            }
            StagingM4Block::VectorFigure { common, .. }
            | StagingM4Block::MathVectorBlock { common, .. } => {
                return Err(
                    StagingSemanticContainerPreflightError::PrecomposedVectorStaging(
                        common.node_id,
                    ),
                );
            }
            StagingM4Block::Paragraph { inline_vectors, .. }
            | StagingM4Block::Heading { inline_vectors, .. } => {
                if let Some(vector) = inline_vectors.first() {
                    return Err(
                        StagingSemanticContainerPreflightError::PrecomposedVectorStaging(
                            vector.node_id,
                        ),
                    );
                }
            }
            StagingM4Block::PageBreak { .. } | StagingM4Block::DisplayMath { .. } => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder, WireImageMediaType, WireStagingM4Block,
        WireStagingM4Inline,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"
    ));
    const PRECOMPOSED_VECTOR_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"
    ));

    fn package() -> (ValidatedStagingSemanticPackage, ValidatedResourceLimits) {
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        (package, limits)
    }

    #[test]
    fn semantic_container_private_profile_closes_kinds_styles_and_media() {
        let (package, limits) = package();
        let session = StagingSemanticContainerSessionIdentity::fresh();
        let receipt =
            preflight_staging_semantic_container_profile(&package, &limits, &session).unwrap();
        assert_eq!(receipt.container_count(), 3);
        receipt.verify(&package, &limits, &session).unwrap();
        assert!(receipt
            .canonical_jcs()
            .contains("\"contract\":\"typaxis.contract/1.4\""));
        assert!(receipt
            .canonical_jcs()
            .contains("\"max_pdf_objects\":5000000"));
        let mut altered_limits = typaxis_core::ResourceLimits::default();
        altered_limits.max_pages -= 1;
        let altered_limits = ValidatedResourceLimits::new(altered_limits).unwrap();
        assert!(matches!(
            preflight_staging_semantic_container_profile(
                &package,
                &altered_limits,
                &StagingSemanticContainerSessionIdentity::fresh()
            ),
            Err(StagingSemanticContainerPreflightError::ReceiptMismatch)
        ));
        assert_eq!(
            receipt.verify(&package, &altered_limits, &session),
            Err(StagingSemanticContainerPreflightError::ReceiptMismatch)
        );
        assert_eq!(
            StagingSemanticContainerProfileDescriptor::CONTRACT,
            "typaxis.contract/1.4"
        );
        assert_eq!(
            StagingSemanticContainerProfileDescriptor.style_properties(),
            [
                BasicStyleProperty::FontFamily,
                BasicStyleProperty::FontSize,
                BasicStyleProperty::LineHeight,
                BasicStyleProperty::Page,
                BasicStyleProperty::SpaceBefore,
                BasicStyleProperty::SpaceAfter,
                BasicStyleProperty::StartIndent,
                BasicStyleProperty::EndIndent,
                BasicStyleProperty::TextAlign,
                BasicStyleProperty::KeepWithNext,
            ]
        );
        assert_eq!(
            typaxis_core::DocumentPackageContractId::CURRENT.as_str(),
            "typaxis.contract/1.3"
        );
    }

    #[test]
    fn semantic_container_profile_receipt_is_session_bound() {
        let (package, limits) = package();
        let session = StagingSemanticContainerSessionIdentity::fresh();
        let receipt =
            preflight_staging_semantic_container_profile(&package, &limits, &session).unwrap();
        assert_eq!(
            receipt.verify(
                &package,
                &limits,
                &StagingSemanticContainerSessionIdentity::fresh()
            ),
            Err(StagingSemanticContainerPreflightError::ReceiptMismatch)
        );
    }

    #[test]
    fn semantic_container_profile_rejects_legacy_media_declaration_before_resource_open() {
        let (package, _) = package();
        let mut resources = package.resources().clone();
        resources.font_faces[0].media = FontMediaDeclaration::LegacyUnspecified;
        assert_eq!(
            validate_media_declarations(&resources),
            Err(StagingSemanticContainerPreflightError::MissingDeclaration)
        );
        resources.font_faces[0].media =
            FontMediaDeclaration::Declared(FontMediaType::SfntTrueTypeGlyf);
        resources.images[0].media = ImageMediaDeclaration::LegacyUnspecified;
        assert_eq!(
            validate_media_declarations(&resources),
            Err(StagingSemanticContainerPreflightError::MissingDeclaration)
        );
        resources.images[0].media = ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2);
        assert_eq!(
            validate_media_declarations(&resources),
            Err(StagingSemanticContainerPreflightError::SvgSafe2Staging(
                resources.images[0].image_id
            ))
        );
    }

    #[test]
    fn semantic_container_profile_rejects_new_kind_before_language_v1_projection() {
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                PRECOMPOSED_VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut resources = wire.resources().clone();
        resources.images[0].media_type = WireImageMediaType::SvgSafe1;
        resources.images[0].vector_provenance = None;
        let mut document = wire.document().clone();
        let WireStagingM4Block::SemanticContainer { blocks, .. } = &mut document.blocks[0] else {
            unreachable!();
        };
        let WireStagingM4Block::Paragraph { children, .. } = &mut blocks[0] else {
            unreachable!();
        };
        let WireStagingM4Inline::InlineVector { language, .. } = &mut children[0] else {
            unreachable!();
        };
        *language = Some("ja".to_owned());
        wire.replace_typed_regions(document, resources);
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                encoded.as_bytes(),
                &DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();

        assert!(matches!(
            preflight_staging_semantic_container_profile(
                &package,
                &limits,
                &StagingSemanticContainerSessionIdentity::fresh(),
            ),
            Err(StagingSemanticContainerPreflightError::PrecomposedVectorStaging(owner))
                if owner == NodeId::new(3)
        ));
    }

    #[test]
    fn semantic_container_profile_rejects_recursive_empty_before_flow_allocation() {
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

        fn remove_authored_content(blocks: &mut [WireStagingM4Block]) {
            for block in blocks {
                match block {
                    WireStagingM4Block::Paragraph { children, .. }
                    | WireStagingM4Block::Heading { children, .. } => {
                        for child in children {
                            remove_inline_content(child);
                        }
                    }
                    WireStagingM4Block::List { items, .. } => {
                        for item in items {
                            remove_authored_content(&mut item.blocks);
                        }
                    }
                    WireStagingM4Block::Table { head, body, .. } => {
                        for cell in head.iter_mut().chain(body).flat_map(|row| &mut row.cells) {
                            remove_authored_content(&mut cell.blocks);
                        }
                    }
                    WireStagingM4Block::Figure { alt, caption, .. } => {
                        alt.clear();
                        remove_authored_content(caption);
                    }
                    WireStagingM4Block::VectorFigure { caption, .. } => {
                        remove_authored_content(caption);
                    }
                    WireStagingM4Block::SemanticContainer { blocks, .. } => {
                        remove_authored_content(blocks);
                    }
                    WireStagingM4Block::PageBreak { .. }
                    | WireStagingM4Block::DisplayMath { .. }
                    | WireStagingM4Block::MathVectorBlock { .. } => {}
                }
            }
        }

        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut document = wire.document().clone();
        remove_authored_content(&mut document.blocks);
        wire.replace_typed_regions(document, wire.resources().clone());
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                encoded.as_bytes(),
                &DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        assert!(matches!(
            preflight_staging_semantic_container_profile(
                &package,
                &limits,
                &StagingSemanticContainerSessionIdentity::fresh(),
            ),
            Err(StagingSemanticContainerPreflightError::EmptyContainer(owner))
                if owner == NodeId::new(1)
        ));
    }
}
