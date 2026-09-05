use typaxis_core::{push_jcs_string, sha256, M4EffectiveResourceLimits, ValidatedResourceLimits};
use typaxis_syntax::machine_profile_boundary::{
    StagingComputedLanguageOwnerKindV2, StagingLanguageNodeKind, StagingOutlineSourceKind,
};
use typaxis_syntax::{
    StagingBookNavigationProfileAuthorization, StagingBookNavigationProfileAuthorizationV2,
    StagingBookNavigationProfileView, StagingBookNavigationProfileViewV2,
    ValidatedStagingBookNavigation, ValidatedStagingBookNavigationV2,
    ValidatedStagingSemanticPackage,
};

use crate::semantic_container::{
    preflight_staging_semantic_container_profile_for_book_navigation,
    preflight_staging_semantic_container_profile_for_tagged_pdf,
};
use crate::{
    preflight_staging_precomposed_vector_profile, StagingPrecomposedVectorProfileReceipt,
    StagingSemanticContainerPreflightReceipt, StagingSemanticContainerSessionIdentity,
    STAGING_PRODUCTION_BOOK_PROFILE_ID,
};

pub const STAGING_BOOK_NAVIGATION_PROFILE_ALGORITHM: &str =
    "typaxis.book-navigation-profile-receipt/1";
pub const STAGING_BOOK_NAVIGATION_PROFILE_ALGORITHM_V2: &str =
    "typaxis.book-navigation-profile-receipt/2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingBookNavigationProfileDescriptor;

impl StagingBookNavigationProfileDescriptor {
    pub const PROFILE_ID: &'static str = STAGING_PRODUCTION_BOOK_PROFILE_ID;
    pub const CONTRACT: &'static str = "typaxis.contract/1.4";

    pub const fn metadata_fields(self) -> [&'static str; 7] {
        [
            "author",
            "created",
            "identifier",
            "keywords",
            "modified",
            "subject",
            "title",
        ]
    }

    pub const fn language_owner_kinds(self) -> [StagingLanguageNodeKind; 19] {
        [
            StagingLanguageNodeKind::Document,
            StagingLanguageNodeKind::SemanticContainer,
            StagingLanguageNodeKind::Paragraph,
            StagingLanguageNodeKind::Heading,
            StagingLanguageNodeKind::List,
            StagingLanguageNodeKind::ListItem,
            StagingLanguageNodeKind::Table,
            StagingLanguageNodeKind::TableRow,
            StagingLanguageNodeKind::TableCell,
            StagingLanguageNodeKind::Figure,
            StagingLanguageNodeKind::FootnoteDefinition,
            StagingLanguageNodeKind::Text,
            StagingLanguageNodeKind::Emphasis,
            StagingLanguageNodeKind::Strong,
            StagingLanguageNodeKind::Link,
            StagingLanguageNodeKind::Reference,
            StagingLanguageNodeKind::FootnoteReference,
            StagingLanguageNodeKind::InlineMath,
            StagingLanguageNodeKind::DisplayMath,
        ]
    }

    pub const fn outline_source_kinds(self) -> [StagingOutlineSourceKind; 2] {
        [
            StagingOutlineSourceKind::Heading,
            StagingOutlineSourceKind::SemanticContainer,
        ]
    }

    pub const fn emits_info(self) -> bool {
        true
    }
    pub const fn emits_fixed_xmp(self) -> bool {
        true
    }
    pub const fn emits_catalog_language(self) -> bool {
        true
    }
    pub const fn emits_outline_tree(self) -> bool {
        true
    }
    pub const fn emits_outline_structure_binding(self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingBookNavigationProfileDescriptorV2;

impl StagingBookNavigationProfileDescriptorV2 {
    pub const PROFILE_ID: &'static str = STAGING_PRODUCTION_BOOK_PROFILE_ID;
    pub const CONTRACT: &'static str = "typaxis.contract/1.4";

    pub const fn metadata_fields(self) -> [&'static str; 7] {
        StagingBookNavigationProfileDescriptor.metadata_fields()
    }

    pub const fn language_owner_kinds(self) -> [StagingComputedLanguageOwnerKindV2; 23] {
        StagingComputedLanguageOwnerKindV2::ALL
    }

    pub const fn outline_source_kinds(self) -> [StagingOutlineSourceKind; 2] {
        StagingBookNavigationProfileDescriptor.outline_source_kinds()
    }

    pub const fn emits_info(self) -> bool {
        true
    }
    pub const fn emits_fixed_xmp(self) -> bool {
        true
    }
    pub const fn emits_catalog_language(self) -> bool {
        true
    }
    pub const fn emits_outline_tree(self) -> bool {
        true
    }
    pub const fn emits_outline_structure_binding(self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingBookNavigationProfileError {
    UnsupportedFeature,
    BaseProfile,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingBookNavigationProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFeature => formatter.write_str(
                "L5100: metadata, language, or outline is outside the closed production-book-1 profile",
            ),
            Self::BaseProfile => {
                formatter.write_str("L5100: production-book base profile preflight failed")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: book-navigation profile receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingBookNavigationProfileError {}

#[derive(Debug)]
pub struct StagingBookNavigationProfileReceipt {
    base: StagingSemanticContainerPreflightReceipt,
    session: StagingSemanticContainerSessionIdentity,
    authorization: StagingBookNavigationProfileAuthorization,
    descriptor_jcs: String,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingBookNavigationProfileReceipt {
    pub const fn base(&self) -> &StagingSemanticContainerPreflightReceipt {
        &self.base
    }
    pub const fn authorization(&self) -> &StagingBookNavigationProfileAuthorization {
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
    pub const fn metadata_sha256(&self) -> [u8; 32] {
        self.authorization.metadata_sha256()
    }
    pub const fn language_sha256(&self) -> [u8; 32] {
        self.authorization.language_sha256()
    }
    pub const fn outline_sha256(&self) -> [u8; 32] {
        self.authorization.outline_sha256()
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigation,
        limits: &ValidatedResourceLimits,
        session: &StagingSemanticContainerSessionIdentity,
    ) -> Result<(), StagingBookNavigationProfileError> {
        self.base
            .verify(package, limits, session)
            .map_err(|_| StagingBookNavigationProfileError::ReceiptMismatch)?;
        let view = StagingBookNavigationProfileView::new(package, navigation, limits)
            .map_err(|_| StagingBookNavigationProfileError::ReceiptMismatch)?;
        let descriptor_jcs = encode_descriptor();
        let canonical_jcs = encode_receipt(&self.base, &view, &descriptor_jcs);
        if self.session != *session
            || self.authorization.view() != &view
            || self.authorization.profile_receipt_fingerprint() != self.fingerprint
            || self
                .authorization
                .authorizes(package, navigation, limits)
                .is_err()
            || self.descriptor_jcs != descriptor_jcs
            || self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
        {
            return Err(StagingBookNavigationProfileError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn preflight_staging_book_navigation_profile(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    limits: &ValidatedResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingBookNavigationProfileReceipt, StagingBookNavigationProfileError> {
    let base =
        preflight_staging_semantic_container_profile_for_book_navigation(package, limits, session)
            .map_err(|_| StagingBookNavigationProfileError::BaseProfile)?;
    finish_book_navigation_preflight(package, navigation, limits, session, base)
}

pub(crate) fn preflight_staging_book_navigation_profile_for_tagged_pdf(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    limits: &ValidatedResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingBookNavigationProfileReceipt, StagingBookNavigationProfileError> {
    let base =
        preflight_staging_semantic_container_profile_for_tagged_pdf(package, limits, session)
            .map_err(|_| StagingBookNavigationProfileError::BaseProfile)?;
    finish_book_navigation_preflight(package, navigation, limits, session, base)
}

fn finish_book_navigation_preflight(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    limits: &ValidatedResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
    base: StagingSemanticContainerPreflightReceipt,
) -> Result<StagingBookNavigationProfileReceipt, StagingBookNavigationProfileError> {
    let view = StagingBookNavigationProfileView::new(package, navigation, limits)
        .map_err(|_| StagingBookNavigationProfileError::ReceiptMismatch)?;
    let descriptor_jcs = encode_descriptor();
    let canonical_jcs = encode_receipt(&base, &view, &descriptor_jcs);
    let fingerprint = sha256(canonical_jcs.as_bytes());
    let authorization = StagingBookNavigationProfileAuthorization::bind_profile_receipt(
        view,
        fingerprint,
        package,
        navigation,
        limits,
    )
    .map_err(|_| StagingBookNavigationProfileError::ReceiptMismatch)?;
    let receipt = StagingBookNavigationProfileReceipt {
        base,
        session: session.clone(),
        authorization,
        descriptor_jcs,
        fingerprint,
        canonical_jcs,
    };
    receipt.verify(package, navigation, limits, session)?;
    Ok(receipt)
}

/// Version-2 profile closure. Its base is the sealed precomposed-vector
/// profile receipt, so an old book-navigation or non-vector base receipt is
/// not representable at this boundary.
#[derive(Debug)]
pub struct StagingBookNavigationProfileReceiptV2 {
    base: StagingPrecomposedVectorProfileReceipt,
    session: StagingSemanticContainerSessionIdentity,
    authorization: StagingBookNavigationProfileAuthorizationV2,
    descriptor_jcs: String,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingBookNavigationProfileReceiptV2 {
    pub const fn base(&self) -> &StagingPrecomposedVectorProfileReceipt {
        &self.base
    }
    pub const fn authorization(&self) -> &StagingBookNavigationProfileAuthorizationV2 {
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
    pub const fn metadata_sha256(&self) -> [u8; 32] {
        self.authorization.metadata_sha256()
    }
    pub const fn language_sha256(&self) -> [u8; 32] {
        self.authorization.language_sha256()
    }
    pub const fn outline_sha256(&self) -> [u8; 32] {
        self.authorization.outline_sha256()
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigationV2,
        limits: &M4EffectiveResourceLimits,
        session: &StagingSemanticContainerSessionIdentity,
    ) -> Result<(), StagingBookNavigationProfileError> {
        self.base
            .verify(package, limits, session)
            .map_err(|_| StagingBookNavigationProfileError::ReceiptMismatch)?;
        let view = StagingBookNavigationProfileViewV2::new(package, navigation, limits)
            .map_err(|_| StagingBookNavigationProfileError::ReceiptMismatch)?;
        let vector_owners = navigation
            .languages()
            .records()
            .iter()
            .filter(|record| record.node_kind.is_precomposed_vector())
            .map(|record| record.node_id);
        if self.base.vector_owners().ne(vector_owners) {
            return Err(StagingBookNavigationProfileError::ReceiptMismatch);
        }
        let descriptor_jcs = encode_descriptor_v2();
        let canonical_jcs = encode_receipt_v2(&self.base, &view, &descriptor_jcs);
        if self.session != *session
            || self.authorization.view() != &view
            || self.authorization.profile_receipt_fingerprint() != self.fingerprint
            || self
                .authorization
                .precomposed_vector_profile_receipt_fingerprint()
                != self.base.fingerprint()
            || self.authorization.precomposed_vector_profile_fingerprint()
                != self.base.authorization().profile_fingerprint()
            || self
                .authorization
                .authorizes(package, navigation, limits)
                .is_err()
            || self.descriptor_jcs != descriptor_jcs
            || self.canonical_jcs != canonical_jcs
            || self.fingerprint != sha256(canonical_jcs.as_bytes())
        {
            return Err(StagingBookNavigationProfileError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn preflight_staging_book_navigation_profile_v2(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    limits: &M4EffectiveResourceLimits,
    session: &StagingSemanticContainerSessionIdentity,
) -> Result<StagingBookNavigationProfileReceiptV2, StagingBookNavigationProfileError> {
    navigation
        .verify(package, limits)
        .map_err(|_| StagingBookNavigationProfileError::ReceiptMismatch)?;
    let base = preflight_staging_precomposed_vector_profile(package, limits, session)
        .map_err(|_| StagingBookNavigationProfileError::BaseProfile)?;
    let view = StagingBookNavigationProfileViewV2::new(package, navigation, limits)
        .map_err(|_| StagingBookNavigationProfileError::ReceiptMismatch)?;
    if base.vector_owners().ne(navigation
        .languages()
        .records()
        .iter()
        .filter(|record| record.node_kind.is_precomposed_vector())
        .map(|record| record.node_id))
    {
        return Err(StagingBookNavigationProfileError::ReceiptMismatch);
    }
    let descriptor_jcs = encode_descriptor_v2();
    let canonical_jcs = encode_receipt_v2(&base, &view, &descriptor_jcs);
    let fingerprint = sha256(canonical_jcs.as_bytes());
    let authorization = StagingBookNavigationProfileAuthorizationV2::bind_profile_receipt(
        view,
        fingerprint,
        base.fingerprint(),
        base.authorization().profile_fingerprint(),
        package,
        navigation,
        limits,
    )
    .map_err(|_| StagingBookNavigationProfileError::ReceiptMismatch)?;
    let receipt = StagingBookNavigationProfileReceiptV2 {
        base,
        session: session.clone(),
        authorization,
        descriptor_jcs,
        canonical_jcs,
        fingerprint,
    };
    receipt.verify(package, navigation, limits, session)?;
    Ok(receipt)
}

fn encode_descriptor() -> String {
    let descriptor = StagingBookNavigationProfileDescriptor;
    let mut output =
        String::from("{\"contract\":\"typaxis.contract/1.4\",\"language_owner_kinds\":[");
    let mut kinds = descriptor.language_owner_kinds().to_vec();
    kinds.sort_by_key(|kind| kind.as_str().as_bytes());
    kinds.dedup_by_key(|kind| kind.as_str());
    for (index, kind) in kinds.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, kind.as_str());
    }
    output.push_str("],\"metadata_fields\":[");
    for (index, field) in descriptor.metadata_fields().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, field);
    }
    output.push_str("],\"outline_source_kinds\":[\"heading\",\"semantic_container\"],\"pdf\":{\"catalog_language\":true,\"fixed_xmp\":true,\"info\":true,\"outline_se\":false,\"outline_tree\":true},\"profile\":");
    push_jcs_string(
        &mut output,
        StagingBookNavigationProfileDescriptor::PROFILE_ID,
    );
    output.push('}');
    output
}

fn encode_descriptor_v2() -> String {
    let descriptor = StagingBookNavigationProfileDescriptorV2;
    let mut output =
        String::from("{\"contract\":\"typaxis.contract/1.4\",\"language_owner_kinds\":[");
    let mut kinds = descriptor.language_owner_kinds().to_vec();
    kinds.sort_by_key(|kind| kind.as_str().as_bytes());
    kinds.dedup_by_key(|kind| kind.as_str());
    for (index, kind) in kinds.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, kind.as_str());
    }
    output.push_str("],\"metadata_fields\":[");
    for (index, field) in descriptor.metadata_fields().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, field);
    }
    output.push_str("],\"outline_source_kinds\":[\"heading\",\"semantic_container\"],\"pdf\":{\"catalog_language\":true,\"fixed_xmp\":true,\"info\":true,\"outline_se\":false,\"outline_tree\":true},\"profile\":");
    push_jcs_string(
        &mut output,
        StagingBookNavigationProfileDescriptorV2::PROFILE_ID,
    );
    output.push('}');
    output
}

fn encode_receipt(
    base: &StagingSemanticContainerPreflightReceipt,
    authorization: &StagingBookNavigationProfileView,
    descriptor_jcs: &str,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_BOOK_NAVIGATION_PROFILE_ALGORITHM);
    output.push_str(",\"authorization_sha256\":");
    push_hash(&mut output, authorization.fingerprint());
    output.push_str(",\"base_profile_sha256\":");
    push_hash(&mut output, base.fingerprint());
    output.push_str(",\"descriptor_sha256\":");
    push_hash(&mut output, sha256(descriptor_jcs.as_bytes()));
    output.push('}');
    output
}

fn encode_receipt_v2(
    base: &StagingPrecomposedVectorProfileReceipt,
    authorization: &StagingBookNavigationProfileViewV2,
    descriptor_jcs: &str,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_BOOK_NAVIGATION_PROFILE_ALGORITHM_V2);
    output.push_str(",\"authorization_sha256\":");
    push_hash(&mut output, authorization.fingerprint());
    output.push_str(",\"base_profile_sha256\":");
    push_hash(&mut output, base.fingerprint());
    output.push_str(",\"descriptor_sha256\":");
    push_hash(&mut output, sha256(descriptor_jcs.as_bytes()));
    output.push('}');
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
    };
    use typaxis_syntax::{
        validate_staging_book_navigation, validate_staging_book_navigation_v2,
        StagingSemanticPackageParser,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/book-navigation/job/document-package.json"
    ));
    const PRECOMPOSED_VECTOR_FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/document-package.json"
    ));

    #[test]
    fn book_navigation_profile_is_closed_and_receipted() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let navigation = validate_staging_book_navigation(&package, &limits).unwrap();
        let session = StagingSemanticContainerSessionIdentity::fresh();
        assert!(
            crate::preflight_staging_semantic_container_profile(&package, &limits, &session,)
                .is_err()
        );
        let receipt =
            preflight_staging_book_navigation_profile(&package, &navigation, &limits, &session)
                .unwrap();
        assert_eq!(
            receipt.authorization().profile_receipt_fingerprint(),
            receipt.fingerprint()
        );
        receipt
            .verify(&package, &navigation, &limits, &session)
            .unwrap();
        assert!(receipt.descriptor_jcs().contains("\"fixed_xmp\":true"));
        assert!(receipt.descriptor_jcs().contains(
            "\"language_owner_kinds\":[\"display_math\",\"document\",\"emphasis\",\"figure\",\"footnote_definition\",\"footnote_reference\",\"heading\",\"inline_math\",\"link\",\"list\",\"list_item\",\"paragraph\",\"reference\",\"semantic_container\",\"strong\",\"table\",\"table_cell\",\"table_row\",\"text\"]"
        ));
        assert_eq!(
            typaxis_core::DocumentPackageContractId::CURRENT,
            typaxis_core::DocumentPackageContractId::V1_4
        );
    }

    #[test]
    fn book_navigation_profile_v2_binds_complete_owner_set_and_rejects_old_path() {
        let base = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let limits = M4EffectiveResourceLimits::new(base, M4ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(
                PRECOMPOSED_VECTOR_FIXTURE,
                &DocumentPackageDecodePolicy::new(limits.base()),
            )
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, limits.base())
            .unwrap();
        assert!(validate_staging_book_navigation(&package, limits.base()).is_err());
        let navigation = validate_staging_book_navigation_v2(&package, &limits).unwrap();
        let session = StagingSemanticContainerSessionIdentity::fresh();
        let mut receipt =
            preflight_staging_book_navigation_profile_v2(&package, &navigation, &limits, &session)
                .unwrap();
        assert_eq!(
            StagingBookNavigationProfileDescriptorV2
                .language_owner_kinds()
                .len(),
            23
        );
        for kind in [
            "inline_vector",
            "math_vector",
            "vector_figure",
            "math_vector_block",
        ] {
            assert!(receipt.descriptor_jcs().contains(&format!("\"{kind}\"")));
        }
        assert_eq!(receipt.base().vector_owners().count(), 4);
        assert_eq!(receipt.authorization().view().vector_owner_count(), 4);
        assert_eq!(
            receipt.authorization().profile_receipt_fingerprint(),
            receipt.fingerprint()
        );
        assert_eq!(
            receipt
                .authorization()
                .precomposed_vector_profile_receipt_fingerprint(),
            receipt.base().fingerprint()
        );
        assert_eq!(
            receipt
                .authorization()
                .precomposed_vector_profile_fingerprint(),
            receipt.base().authorization().profile_fingerprint()
        );
        assert!(receipt
            .canonical_jcs()
            .contains("\"algorithm\":\"typaxis.book-navigation-profile-receipt/2\""));
        receipt
            .verify(&package, &navigation, &limits, &session)
            .unwrap();

        receipt.descriptor_jcs.push(' ');
        assert_eq!(
            receipt.verify(&package, &navigation, &limits, &session),
            Err(StagingBookNavigationProfileError::ReceiptMismatch)
        );
    }
}
