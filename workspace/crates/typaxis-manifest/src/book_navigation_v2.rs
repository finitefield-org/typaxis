use typaxis_core::{push_jcs_string, sha256, EngineIdentity, M4EffectiveResourceLimits};
use typaxis_display_list::{BookNavigationSelectedReceiptV2, StagingPrecomposedVectorDisplay};
use typaxis_pdf::StagingTaggedPdfV2;
use typaxis_syntax::{
    StagingBookNavigationProfileAuthorizationV2, ValidatedStagingBookNavigationV2,
    ValidatedStagingSemanticPackage,
};

pub const STAGING_BOOK_NAVIGATION_MANIFEST_V2_ALGORITHM: &str =
    "typaxis.book-navigation-manifest/2";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingBookNavigationManifestV2 {
    package_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    metadata_sha256: [u8; 32],
    computed_language_sha256: [u8; 32],
    outline_sha256: [u8; 32],
    profile_view_sha256: [u8; 32],
    profile_receipt_sha256: [u8; 32],
    selected_sha256: [u8; 32],
    pdf_observation_sha256: [u8; 32],
    final_pdf_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingBookNavigationManifestV2 {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn semantic_sha256(&self) -> [u8; 32] {
        self.semantic_sha256
    }
    pub const fn computed_language_sha256(&self) -> [u8; 32] {
        self.computed_language_sha256
    }
    pub const fn profile_receipt_sha256(&self) -> [u8; 32] {
        self.profile_receipt_sha256
    }
    pub const fn selected_sha256(&self) -> [u8; 32] {
        self.selected_sha256
    }
    pub const fn pdf_observation_sha256(&self) -> [u8; 32] {
        self.pdf_observation_sha256
    }
    pub const fn final_pdf_sha256(&self) -> [u8; 32] {
        self.final_pdf_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingBookNavigationManifestV2Error {
    ProfileMismatch,
    SelectedMismatch,
    PdfMismatch,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingBookNavigationManifestV2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "I9190: book-navigation manifest /2 {:?}", self)
    }
}
impl std::error::Error for StagingBookNavigationManifestV2Error {}

#[allow(clippy::too_many_arguments)]
pub fn build_staging_book_navigation_manifest_v2(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    profile: &StagingBookNavigationProfileAuthorizationV2,
    selected: &BookNavigationSelectedReceiptV2,
    display: &StagingPrecomposedVectorDisplay,
    pdf: &StagingTaggedPdfV2,
    limits: &M4EffectiveResourceLimits,
    engine: &EngineIdentity,
) -> Result<StagingBookNavigationManifestV2, StagingBookNavigationManifestV2Error> {
    profile
        .authorizes(package, navigation, limits)
        .map_err(|_| StagingBookNavigationManifestV2Error::ProfileMismatch)?;
    selected
        .verify(navigation, profile, limits, display)
        .map_err(|_| StagingBookNavigationManifestV2Error::SelectedMismatch)?;
    let observation = pdf.book_navigation();
    if observation.metadata_sha256() != navigation.metadata().fingerprint()
        || observation.language_sha256() != navigation.languages().fingerprint()
        || observation.outline_sha256() != navigation.outline().fingerprint()
        || observation.destination_registry_sha256() != selected.destination_registry_sha256()
        || observation.profile_sha256() != profile.profile_receipt_fingerprint()
        || observation.selected_sha256() != selected.fingerprint()
        || observation.final_pdf_sha256() != pdf.final_pdf().content_hash()
        || observation.final_pdf_byte_length() != pdf.final_pdf().byte_length()
        || observation.document_language() != navigation.languages().document_language()
        || observation.xmp_sha256() != pdf.observation().xmp_sha256()
    {
        return Err(StagingBookNavigationManifestV2Error::PdfMismatch);
    }
    let canonical_jcs =
        encode_manifest(package, navigation, profile, selected, observation, engine);
    Ok(StagingBookNavigationManifestV2 {
        package_sha256: package.canonical_jcs_sha256(),
        semantic_sha256: package.semantic_fingerprint(),
        metadata_sha256: navigation.metadata().fingerprint(),
        computed_language_sha256: navigation.languages().fingerprint(),
        outline_sha256: navigation.outline().fingerprint(),
        profile_view_sha256: profile.fingerprint(),
        profile_receipt_sha256: profile.profile_receipt_fingerprint(),
        selected_sha256: selected.fingerprint(),
        pdf_observation_sha256: observation.fingerprint(),
        final_pdf_sha256: pdf.final_pdf().content_hash(),
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn encode_manifest(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigationV2,
    profile: &StagingBookNavigationProfileAuthorizationV2,
    selected: &BookNavigationSelectedReceiptV2,
    observation: &typaxis_pdf::BookNavigationPdfObservationV2,
    engine: &EngineIdentity,
) -> String {
    let mut out = String::from("{\"algorithm\":");
    push_jcs_string(&mut out, STAGING_BOOK_NAVIGATION_MANIFEST_V2_ALGORITHM);
    out.push_str(",\"contract\":\"typaxis.contract/1.4\",\"document_language\":");
    push_jcs_string(&mut out, navigation.languages().document_language());
    out.push_str(",\"engine\":{\"name\":");
    push_jcs_string(&mut out, engine.name());
    out.push_str(",\"version\":");
    push_jcs_string(&mut out, engine.version());
    out.push_str("},\"fingerprints\":{");
    for (index, (key, value)) in [
        (
            "computed_language_sha256",
            navigation.languages().fingerprint(),
        ),
        (
            "destination_registry_sha256",
            selected.destination_registry_sha256(),
        ),
        ("metadata_sha256", navigation.metadata().fingerprint()),
        ("outline_sha256", navigation.outline().fingerprint()),
        ("package_sha256", package.canonical_jcs_sha256()),
        ("pdf_observation_sha256", observation.fingerprint()),
        ("pdf_sha256", observation.final_pdf_sha256()),
        (
            "profile_receipt_sha256",
            profile.profile_receipt_fingerprint(),
        ),
        ("profile_view_sha256", profile.fingerprint()),
        ("selected_sha256", selected.fingerprint()),
        ("semantic_sha256", package.semantic_fingerprint()),
    ]
    .into_iter()
    .enumerate()
    {
        if index > 0 {
            out.push(',')
        }
        push_jcs_string(&mut out, key);
        out.push(':');
        push_hash(&mut out, value)
    }
    out.push_str("},\"language_paint_count\":");
    out.push_str(&observation.language_paint_count().to_string());
    out.push_str(",\"object_numbers\":{\"catalog\":");
    out.push_str(&observation.catalog_object().to_string());
    out.push_str(",\"info\":");
    out.push_str(&observation.info_object().to_string());
    out.push_str(",\"metadata\":");
    out.push_str(&observation.metadata_object().to_string());
    out.push_str(",\"outline_root\":");
    match observation.outline_root_object() {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    };
    out.push_str("}}");
    out
}
fn push_hash(out: &mut String, value: [u8; 32]) {
    const H: &[u8; 16] = b"0123456789abcdef";
    out.push('"');
    for byte in value {
        out.push(char::from(H[usize::from(byte >> 4)]));
        out.push(char::from(H[usize::from(byte & 15)]))
    }
    out.push('"')
}
