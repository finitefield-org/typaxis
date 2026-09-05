use std::collections::BTreeSet;

use typaxis_core::{push_jcs_string, sha256, M4EffectiveResourceLimits, ValidatedResourceLimits};
use typaxis_syntax::{
    StagingAccessibilityProfileAuthorization, StagingAccessibilityProfileAuthorizationV2,
    StagingAccessibilityProfileView, StagingAccessibilityProfileViewV2,
    StagingStructureSemanticKind, StagingStructureTableSection, ValidatedStagingBookNavigation,
    ValidatedStagingBookNavigationV2, ValidatedStagingSemanticPackage,
    ValidatedStagingStructureSemantics, ValidatedStagingStructureSemanticsV2,
};

use crate::{
    book_navigation::preflight_staging_book_navigation_profile_for_tagged_pdf,
    preflight_staging_book_navigation_profile_v2, StagingBookNavigationProfileReceipt,
    StagingBookNavigationProfileReceiptV2, StagingSemanticContainerSessionIdentity,
    STAGING_PRODUCTION_BOOK_PROFILE_ID,
};

pub const STAGING_TAGGED_PDF_PROFILE_ALGORITHM: &str =
    "typaxis.production-accessibility-preflight/1";
pub const STAGING_TAGGED_PDF_PROFILE_ALGORITHM_V2: &str =
    "typaxis.production-accessibility-preflight/2";
pub const STAGING_PDFUA1_PROFILE_ID: &str = "typaxis.pdfua1-profile/1";
pub const STAGING_PDFUA1_PROFILE_ID_V2: &str = "typaxis.pdfua1-profile/2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingTaggedPdfProfileDescriptor;

impl StagingTaggedPdfProfileDescriptor {
    pub const PROFILE_ID: &'static str = STAGING_PRODUCTION_BOOK_PROFILE_ID;
    pub const CONTRACT: &'static str = "typaxis.contract/1.4";
    pub const PDF_VERSION: &'static str = "1.7";
    pub const ACCESSIBILITY_PROFILE: &'static str = STAGING_PDFUA1_PROFILE_ID;

    pub const fn structure_roles(self) -> [&'static str; 30] {
        [
            "Caption",
            "Document",
            "Em",
            "Exercise",
            "Figure",
            "Formula",
            "H1",
            "H2",
            "H3",
            "H4",
            "H5",
            "H6",
            "L",
            "LBody",
            "LI",
            "Lbl",
            "Link",
            "Note",
            "P",
            "Proof",
            "Reference",
            "Result",
            "Span",
            "Strong",
            "TBody",
            "TD",
            "TH",
            "THead",
            "TR",
            "Table",
        ]
    }

    pub const fn artifact_classes(self) -> [&'static str; 4] {
        [
            "layout",
            "pagination",
            "pagination_footer",
            "pagination_header",
        ]
    }

    pub const fn validators(self) -> [&'static str; 3] {
        [
            "typaxis.tagged-pdf-validator/1",
            "verapdf-greenfield/1.30.2:ua1",
            "typaxis.matterhorn-assessment/1",
        ]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingTaggedPdfProfileDescriptorV2;

impl StagingTaggedPdfProfileDescriptorV2 {
    pub const PROFILE_ID: &'static str = STAGING_PRODUCTION_BOOK_PROFILE_ID;
    pub const CONTRACT: &'static str = "typaxis.contract/1.4";
    pub const PDF_VERSION: &'static str = "1.7";
    pub const ACCESSIBILITY_PROFILE: &'static str = STAGING_PDFUA1_PROFILE_ID_V2;

    pub const fn structure_roles(self) -> [&'static str; 30] {
        StagingTaggedPdfProfileDescriptor.structure_roles()
    }

    pub const fn artifact_classes(self) -> [&'static str; 4] {
        StagingTaggedPdfProfileDescriptor.artifact_classes()
    }

    pub const fn validators(self) -> [&'static str; 3] {
        [
            "typaxis.tagged-pdf-validator/2",
            "verapdf-greenfield/1.30.2:ua1",
            "typaxis.matterhorn-assessment/2",
        ]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingTaggedPdfProfileError {
    BaseProfile,
    UnsupportedSemantic,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingTaggedPdfProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BaseProfile => {
                formatter.write_str("L5100: production-book navigation preflight failed")
            }
            Self::UnsupportedSemantic => formatter.write_str(
                "L5100: document semantics are outside the closed production-book-1 PDF/UA-1 profile",
            ),
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: tagged-PDF profile receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingTaggedPdfProfileError {}

#[derive(Debug)]
pub struct StagingTaggedPdfProfileReceipt {
    base: StagingBookNavigationProfileReceipt,
    authorization: StagingAccessibilityProfileAuthorization,
    descriptor_jcs: String,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingTaggedPdfProfileReceipt {
    pub const fn base(&self) -> &StagingBookNavigationProfileReceipt {
        &self.base
    }
    pub const fn authorization(&self) -> &StagingAccessibilityProfileAuthorization {
        &self.authorization
    }
    pub fn descriptor_jcs(&self) -> &str {
        &self.descriptor_jcs
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
        navigation: &ValidatedStagingBookNavigation,
        semantics: &ValidatedStagingStructureSemantics,
        limits: &ValidatedResourceLimits,
        session: &StagingSemanticContainerSessionIdentity,
    ) -> Result<(), StagingTaggedPdfProfileError> {
        self.base
            .verify(package, navigation, limits, session)
            .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
        semantics
            .verify(package, navigation)
            .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
        validate_accessibility_subset(navigation, semantics)?;
        let view = StagingAccessibilityProfileView::new(package, navigation, semantics)
            .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
        let descriptor_jcs = encode_descriptor();
        let canonical_jcs = encode_receipt(&self.base, semantics, &view, &descriptor_jcs);
        if self.authorization.view() != &view
            || self.authorization.profile_receipt_fingerprint() != self.fingerprint
            || self
                .authorization
                .authorizes(package, navigation, semantics)
                .is_err()
            || self.descriptor_jcs != descriptor_jcs
            || self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
        {
            return Err(StagingTaggedPdfProfileError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn preflight_staging_tagged_pdf_profile(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    semantics: &ValidatedStagingStructureSemantics,
    limits: &ValidatedResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingTaggedPdfProfileReceipt, StagingTaggedPdfProfileError> {
    let base = preflight_staging_book_navigation_profile_for_tagged_pdf(
        package, navigation, limits, session,
    )
    .map_err(|_| StagingTaggedPdfProfileError::BaseProfile)?;
    semantics
        .verify(package, navigation)
        .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
    validate_accessibility_subset(navigation, semantics)?;
    let view = StagingAccessibilityProfileView::new(package, navigation, semantics)
        .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
    let descriptor_jcs = encode_descriptor();
    let canonical_jcs = encode_receipt(&base, semantics, &view, &descriptor_jcs);
    let fingerprint = sha256(canonical_jcs.as_bytes());
    let authorization = StagingAccessibilityProfileAuthorization::bind_profile_receipt(
        view,
        fingerprint,
        package,
        navigation,
        semantics,
    )
    .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
    let receipt = StagingTaggedPdfProfileReceipt {
        base,
        authorization,
        descriptor_jcs,
        canonical_jcs,
        fingerprint,
    };
    receipt.verify(package, navigation, semantics, limits, session)?;
    Ok(receipt)
}

/// Version-2 accessibility preflight. Its base is the version-2 navigation
/// profile, preventing a precomposed-vector document from being authorized by
/// any legacy accessibility or language receipt.
#[derive(Debug)]
pub struct StagingTaggedPdfProfileReceiptV2 {
    base: StagingBookNavigationProfileReceiptV2,
    authorization: StagingAccessibilityProfileAuthorizationV2,
    descriptor_jcs: String,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingTaggedPdfProfileReceiptV2 {
    pub const fn base(&self) -> &StagingBookNavigationProfileReceiptV2 {
        &self.base
    }

    pub const fn authorization(&self) -> &StagingAccessibilityProfileAuthorizationV2 {
        &self.authorization
    }

    pub fn descriptor_jcs(&self) -> &str {
        &self.descriptor_jcs
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
        navigation: &ValidatedStagingBookNavigationV2,
        semantics: &ValidatedStagingStructureSemanticsV2,
        limits: &M4EffectiveResourceLimits,
        session: &StagingSemanticContainerSessionIdentity,
    ) -> Result<(), StagingTaggedPdfProfileError> {
        self.base
            .verify(package, navigation, limits, session)
            .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
        semantics
            .verify(package, navigation, limits)
            .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
        validate_accessibility_subset_v2(navigation, semantics)?;
        let view = StagingAccessibilityProfileViewV2::new(package, navigation, semantics, limits)
            .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
        let descriptor_jcs = encode_descriptor_v2();
        let canonical_jcs = encode_receipt_v2(&self.base, semantics, &view, &descriptor_jcs);
        if self.authorization.view() != &view
            || self.authorization.profile_receipt_fingerprint() != self.fingerprint
            || self.authorization.book_navigation_profile_fingerprint() != self.base.fingerprint()
            || self
                .authorization
                .authorizes(package, navigation, semantics, limits)
                .is_err()
            || self.descriptor_jcs != descriptor_jcs
            || self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
        {
            return Err(StagingTaggedPdfProfileError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn preflight_staging_tagged_pdf_profile_v2(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    semantics: &ValidatedStagingStructureSemanticsV2,
    limits: &M4EffectiveResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingTaggedPdfProfileReceiptV2, StagingTaggedPdfProfileError> {
    let base = preflight_staging_book_navigation_profile_v2(package, navigation, limits, session)
        .map_err(|_| StagingTaggedPdfProfileError::BaseProfile)?;
    semantics
        .verify(package, navigation, limits)
        .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
    validate_accessibility_subset_v2(navigation, semantics)?;
    let view = StagingAccessibilityProfileViewV2::new(package, navigation, semantics, limits)
        .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
    let descriptor_jcs = encode_descriptor_v2();
    let canonical_jcs = encode_receipt_v2(&base, semantics, &view, &descriptor_jcs);
    let fingerprint = sha256(canonical_jcs.as_bytes());
    let authorization = StagingAccessibilityProfileAuthorizationV2::bind_profile_receipt(
        view,
        fingerprint,
        base.fingerprint(),
        package,
        navigation,
        semantics,
        limits,
    )
    .map_err(|_| StagingTaggedPdfProfileError::ReceiptMismatch)?;
    let receipt = StagingTaggedPdfProfileReceiptV2 {
        base,
        authorization,
        descriptor_jcs,
        canonical_jcs,
        fingerprint,
    };
    receipt.verify(package, navigation, semantics, limits, session)?;
    Ok(receipt)
}

fn validate_accessibility_subset_v2(
    navigation: &ValidatedStagingBookNavigationV2,
    semantics: &ValidatedStagingStructureSemanticsV2,
) -> Result<(), StagingTaggedPdfProfileError> {
    let metadata = navigation.metadata().metadata();
    if metadata
        .title
        .as_deref()
        .map_or(true, |value| !has_non_whitespace(value))
    {
        return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
    }
    let document_language = navigation.languages().document_language();
    let mut previous_heading = None;
    let mut saw_heading = false;
    let mut footnote_definitions = 0usize;
    let mut footnote_references = BTreeSet::new();
    for record in semantics.records() {
        if record.kind().creates_structure_element() && record.language_binding_v2().is_none() {
            return Err(StagingTaggedPdfProfileError::ReceiptMismatch);
        }
        match record.kind() {
            StagingStructureSemanticKind::Paragraph { has_real_content } => {
                if !has_real_content {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
            }
            StagingStructureSemanticKind::Heading {
                level,
                has_real_content,
            } => {
                if !has_real_content
                    || !(1..=6).contains(level)
                    || (!saw_heading && *level != 1)
                    || previous_heading.is_some_and(|previous| *level > previous + 1)
                {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
                saw_heading = true;
                previous_heading = Some(*level);
            }
            StagingStructureSemanticKind::Table {
                head_rows,
                body_rows,
            } if *head_rows == 0 || *body_rows == 0 => {
                return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
            }
            StagingStructureSemanticKind::TableCell {
                section,
                header_node_ids,
                has_real_content,
                ..
            } => match section {
                StagingStructureTableSection::Head if !has_real_content => {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
                StagingStructureTableSection::Body if header_node_ids.is_empty() => {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
                _ => {}
            },
            StagingStructureSemanticKind::Figure { alternative, .. }
            | StagingStructureSemanticKind::DisplayMath { alternative }
            | StagingStructureSemanticKind::InlineMath { alternative }
            | StagingStructureSemanticKind::InlineVector { alternative, .. }
            | StagingStructureSemanticKind::MathVector { alternative, .. }
            | StagingStructureSemanticKind::VectorFigure { alternative, .. }
            | StagingStructureSemanticKind::MathVectorBlock { alternative, .. }
                if !has_non_whitespace(alternative) =>
            {
                return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
            }
            StagingStructureSemanticKind::MathVectorBlock {
                equation_number_node_id,
                ..
            } => {
                let children = semantics
                    .records()
                    .iter()
                    .filter(|candidate| candidate.parent_node_id() == Some(record.node_id()))
                    .collect::<Vec<_>>();
                match equation_number_node_id {
                    Some(number) => {
                        if children.len() != 1
                            || children[0].node_id() != *number
                            || number.get() != record.node_id().get().saturating_add(1)
                            || !matches!(
                                children[0].kind(),
                                StagingStructureSemanticKind::EquationNumber { binding }
                                    if binding.parent_owner() == record.node_id()
                            )
                        {
                            return Err(StagingTaggedPdfProfileError::ReceiptMismatch);
                        }
                    }
                    None if !children.is_empty() => {
                        return Err(StagingTaggedPdfProfileError::ReceiptMismatch);
                    }
                    None => {}
                }
            }
            StagingStructureSemanticKind::EquationNumber { binding }
                if !has_non_whitespace(binding.exact_text()) =>
            {
                return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
            }
            StagingStructureSemanticKind::Link { accessible_name } => {
                if !has_non_whitespace(accessible_name) || record.language() != document_language {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
            }
            StagingStructureSemanticKind::FootnoteReference {
                footnote_id,
                placement_valid,
                ..
            } => {
                if !placement_valid {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
                footnote_references.insert(footnote_id.as_str());
            }
            StagingStructureSemanticKind::FootnoteDefinition {
                footnote_id,
                reference_node_ids,
                placement_valid,
                ..
            } => {
                if !placement_valid || reference_node_ids.is_empty() {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
                footnote_definitions += 1;
                if !footnote_references.contains(footnote_id.as_str()) {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
            }
            _ => {}
        }
    }
    if footnote_definitions != footnote_references.len() {
        return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
    }
    for outline in navigation.outline().entries() {
        let record = semantics
            .record(outline.source.node_id)
            .ok_or(StagingTaggedPdfProfileError::ReceiptMismatch)?;
        if record.language() != document_language {
            return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
        }
    }
    Ok(())
}

fn validate_accessibility_subset(
    navigation: &ValidatedStagingBookNavigation,
    semantics: &ValidatedStagingStructureSemantics,
) -> Result<(), StagingTaggedPdfProfileError> {
    let metadata = navigation.metadata().metadata();
    if metadata
        .title
        .as_deref()
        .map_or(true, |value| !has_non_whitespace(value))
    {
        return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
    }
    let document_language = navigation.languages().document_language();
    let mut previous_heading = None;
    let mut saw_heading = false;
    let mut footnote_definitions = 0usize;
    let mut footnote_references = BTreeSet::new();
    for record in semantics.records() {
        match record.kind() {
            StagingStructureSemanticKind::Paragraph { has_real_content } => {
                if !has_real_content {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
            }
            StagingStructureSemanticKind::Heading {
                level,
                has_real_content,
            } => {
                if !has_real_content
                    || !(1..=6).contains(level)
                    || (!saw_heading && *level != 1)
                    || previous_heading.is_some_and(|previous| *level > previous + 1)
                {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
                saw_heading = true;
                previous_heading = Some(*level);
            }
            StagingStructureSemanticKind::Table {
                head_rows,
                body_rows,
            } if *head_rows == 0 || *body_rows == 0 => {
                return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
            }
            StagingStructureSemanticKind::TableCell {
                section,
                header_node_ids,
                has_real_content,
                ..
            } => match section {
                StagingStructureTableSection::Head if !has_real_content => {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
                StagingStructureTableSection::Body if header_node_ids.is_empty() => {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
                _ => {}
            },
            StagingStructureSemanticKind::Figure { alternative, .. }
            | StagingStructureSemanticKind::DisplayMath { alternative }
            | StagingStructureSemanticKind::InlineMath { alternative }
                if !has_non_whitespace(alternative) =>
            {
                return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
            }
            StagingStructureSemanticKind::Link { accessible_name } => {
                if !has_non_whitespace(accessible_name) || record.language() != document_language {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
            }
            StagingStructureSemanticKind::FootnoteReference {
                footnote_id,
                placement_valid,
                ..
            } => {
                if !placement_valid {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
                footnote_references.insert(footnote_id.as_str());
            }
            StagingStructureSemanticKind::FootnoteDefinition {
                footnote_id,
                reference_node_ids,
                placement_valid,
                ..
            } => {
                if !placement_valid || reference_node_ids.is_empty() {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
                footnote_definitions += 1;
                if !footnote_references.contains(footnote_id.as_str()) {
                    return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
                }
            }
            _ => {}
        }
    }
    if footnote_definitions != footnote_references.len() {
        return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
    }
    for outline in navigation.outline().entries() {
        let record = semantics
            .record(outline.source.node_id)
            .ok_or(StagingTaggedPdfProfileError::ReceiptMismatch)?;
        if record.language() != document_language {
            return Err(StagingTaggedPdfProfileError::UnsupportedSemantic);
        }
    }
    Ok(())
}

fn encode_descriptor() -> String {
    let descriptor = StagingTaggedPdfProfileDescriptor;
    let mut output = String::from("{\"accessibility_profile\":");
    push_jcs_string(
        &mut output,
        StagingTaggedPdfProfileDescriptor::ACCESSIBILITY_PROFILE,
    );
    output.push_str(",\"artifact_classes\":[");
    for (index, class) in descriptor.artifact_classes().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, class);
    }
    output.push_str("],\"contract\":");
    push_jcs_string(&mut output, StagingTaggedPdfProfileDescriptor::CONTRACT);
    output.push_str(",\"pdf_version\":");
    push_jcs_string(&mut output, StagingTaggedPdfProfileDescriptor::PDF_VERSION);
    output.push_str(",\"profile\":");
    push_jcs_string(&mut output, StagingTaggedPdfProfileDescriptor::PROFILE_ID);
    output.push_str(",\"structure_roles\":[");
    for (index, role) in descriptor.structure_roles().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, role);
    }
    output.push_str("],\"validators\":[");
    for (index, validator) in descriptor.validators().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, validator);
    }
    output.push_str("]}");
    output
}

fn encode_descriptor_v2() -> String {
    let descriptor = StagingTaggedPdfProfileDescriptorV2;
    let mut output = String::from("{\"accessibility_profile\":");
    push_jcs_string(
        &mut output,
        StagingTaggedPdfProfileDescriptorV2::ACCESSIBILITY_PROFILE,
    );
    output.push_str(",\"artifact_classes\":[");
    for (index, class) in descriptor.artifact_classes().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, class);
    }
    output.push_str("],\"contract\":");
    push_jcs_string(&mut output, StagingTaggedPdfProfileDescriptorV2::CONTRACT);
    output.push_str(",\"pdf_version\":");
    push_jcs_string(
        &mut output,
        StagingTaggedPdfProfileDescriptorV2::PDF_VERSION,
    );
    output.push_str(",\"profile\":");
    push_jcs_string(&mut output, StagingTaggedPdfProfileDescriptorV2::PROFILE_ID);
    output.push_str(",\"structure_roles\":[");
    for (index, role) in descriptor.structure_roles().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, role);
    }
    output.push_str("],\"validators\":[");
    for (index, validator) in descriptor.validators().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, validator);
    }
    output.push_str("]}");
    output
}

fn encode_receipt(
    base: &StagingBookNavigationProfileReceipt,
    semantics: &ValidatedStagingStructureSemantics,
    view: &StagingAccessibilityProfileView,
    descriptor_jcs: &str,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_TAGGED_PDF_PROFILE_ALGORITHM);
    output.push_str(",\"base_profile_sha256\":");
    push_hash(&mut output, base.fingerprint());
    output.push_str(",\"descriptor_sha256\":");
    push_hash(&mut output, sha256(descriptor_jcs.as_bytes()));
    output.push_str(",\"profile_view_sha256\":");
    push_hash(&mut output, view.fingerprint());
    output.push_str(",\"structure_semantics_sha256\":");
    push_hash(&mut output, semantics.fingerprint());
    output.push('}');
    output
}

fn encode_receipt_v2(
    base: &StagingBookNavigationProfileReceiptV2,
    semantics: &ValidatedStagingStructureSemanticsV2,
    view: &StagingAccessibilityProfileViewV2,
    descriptor_jcs: &str,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_TAGGED_PDF_PROFILE_ALGORITHM_V2);
    output.push_str(",\"base_profile_sha256\":");
    push_hash(&mut output, base.fingerprint());
    output.push_str(",\"descriptor_sha256\":");
    push_hash(&mut output, sha256(descriptor_jcs.as_bytes()));
    output.push_str(",\"profile_view_sha256\":");
    push_hash(&mut output, view.fingerprint());
    output.push_str(",\"structure_semantics_sha256\":");
    push_hash(&mut output, semantics.fingerprint());
    output.push('}');
    output
}

fn has_non_whitespace(value: &str) -> bool {
    value.chars().any(|character| !character.is_whitespace())
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
        StagingSemanticDocumentPackageEncoder,
    };
    use typaxis_syntax::{
        validate_staging_book_navigation, validate_staging_book_navigation_v2,
        validate_staging_structure_semantics, validate_staging_structure_semantics_v2,
        StagingSemanticPackageParser,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/accessibility/job/document-package.json"
    ));
    const VECTOR_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"
    ));

    #[test]
    fn tagged_pdf_profile_closes_math_navigation_and_accessibility() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let navigation = validate_staging_book_navigation(&package, &limits).unwrap();
        let semantics = validate_staging_structure_semantics(&package, &navigation).unwrap();
        let session = StagingSemanticContainerSessionIdentity::fresh();
        assert!(crate::preflight_staging_book_navigation_profile(
            &package,
            &navigation,
            &limits,
            &session,
        )
        .is_err());
        let receipt = preflight_staging_tagged_pdf_profile(
            &package,
            &navigation,
            &semantics,
            &limits,
            &session,
        )
        .unwrap();
        receipt
            .verify(&package, &navigation, &semantics, &limits, &session)
            .unwrap();
        assert_eq!(
            receipt.authorization().profile_receipt_fingerprint(),
            receipt.fingerprint()
        );
        assert!(receipt
            .descriptor_jcs()
            .contains("verapdf-greenfield/1.30.2:ua1"));
        assert_eq!(
            typaxis_core::DocumentPackageContractId::CURRENT,
            typaxis_core::DocumentPackageContractId::V1_4
        );
    }

    #[test]
    fn accessibility_profile_v2_authorizes_formula_figure_and_equation_span() {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let limits = M4EffectiveResourceLimits::new(base, M4ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(limits.base()),
            )
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut metadata = wire.metadata().clone();
        metadata.title = Some("Precomposed vector accessibility fixture".to_owned());
        let outline = wire.outline().clone();
        wire.replace_book_navigation(metadata, outline);
        let encoded = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                encoded.as_bytes(),
                &DocumentPackageDecodePolicy::new(limits.base()),
            )
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, limits.base())
            .unwrap();
        assert!(validate_staging_book_navigation(&package, limits.base()).is_err());
        let navigation = validate_staging_book_navigation_v2(&package, &limits).unwrap();
        let semantics =
            validate_staging_structure_semantics_v2(&package, &navigation, &limits).unwrap();
        let session = StagingSemanticContainerSessionIdentity::fresh();
        let receipt = preflight_staging_tagged_pdf_profile_v2(
            &package,
            &navigation,
            &semantics,
            &limits,
            &session,
        )
        .unwrap();
        receipt
            .verify(&package, &navigation, &semantics, &limits, &session)
            .unwrap();
        assert_eq!(
            StagingTaggedPdfProfileDescriptorV2::ACCESSIBILITY_PROFILE,
            STAGING_PDFUA1_PROFILE_ID_V2
        );
        assert_eq!(
            StagingTaggedPdfProfileDescriptorV2.structure_roles().len(),
            30
        );
        assert!(receipt.descriptor_jcs().contains("\"Formula\""));
        assert!(receipt.descriptor_jcs().contains("\"Figure\""));
        assert!(receipt.descriptor_jcs().contains("\"Span\""));
        assert_eq!(
            receipt.authorization().profile_receipt_fingerprint(),
            receipt.fingerprint()
        );
        assert_eq!(
            receipt
                .authorization()
                .book_navigation_profile_fingerprint(),
            receipt.base().fingerprint()
        );
        assert!(receipt
            .authorization()
            .view()
            .canonical_jcs()
            .contains(STAGING_TAGGED_PDF_PROFILE_ALGORITHM_V2));
        assert!(receipt
            .canonical_jcs()
            .contains(STAGING_TAGGED_PDF_PROFILE_ALGORITHM_V2));
    }
}
