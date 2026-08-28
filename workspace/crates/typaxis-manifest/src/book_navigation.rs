use typaxis_core::{push_jcs_string, sha256, EngineIdentity, ValidatedResourceLimits};
use typaxis_display_list::{BookNavigationSelectedReceipt, DestinationView};
use typaxis_machine_profile::{
    StagingBookNavigationProfileReceipt, StagingSemanticContainerSessionIdentity,
};
use typaxis_pdf::StagingBookNavigationPdf;
use typaxis_syntax::{ValidatedStagingBookNavigation, ValidatedStagingSemanticPackage};

pub const STAGING_BOOK_NAVIGATION_MANIFEST_ALGORITHM: &str = "typaxis.book-navigation-manifest/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingBookNavigationManifest {
    package_sha256: [u8; 32],
    semantic_sha256: [u8; 32],
    profile_preflight_sha256: [u8; 32],
    selected_sha256: [u8; 32],
    pdf_observation_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingBookNavigationManifest {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn semantic_sha256(&self) -> [u8; 32] {
        self.semantic_sha256
    }
    pub const fn profile_preflight_sha256(&self) -> [u8; 32] {
        self.profile_preflight_sha256
    }
    pub const fn selected_sha256(&self) -> [u8; 32] {
        self.selected_sha256
    }
    pub const fn pdf_observation_sha256(&self) -> [u8; 32] {
        self.pdf_observation_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        navigation: &ValidatedStagingBookNavigation,
        profile: &StagingBookNavigationProfileReceipt,
        session: &StagingSemanticContainerSessionIdentity,
        selected: &BookNavigationSelectedReceipt,
        pdf: &StagingBookNavigationPdf,
        limits: &ValidatedResourceLimits,
        engine: &EngineIdentity,
    ) -> Result<(), StagingBookNavigationManifestError> {
        let expected = derive_manifest(
            package, navigation, profile, session, selected, pdf, limits, engine,
        )?;
        if *self != expected {
            return Err(StagingBookNavigationManifestError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingBookNavigationManifestError {
    ProfileMismatch,
    SelectedMismatch,
    PdfMismatch,
    OutlineMismatch,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingBookNavigationManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => {
                formatter.write_str("I9190: book-navigation manifest profile mismatch")
            }
            Self::SelectedMismatch => {
                formatter.write_str("I9190: book-navigation manifest selected-state mismatch")
            }
            Self::PdfMismatch => {
                formatter.write_str("I9190: book-navigation manifest PDF mismatch")
            }
            Self::OutlineMismatch => {
                formatter.write_str("I9190: book-navigation manifest outline closure mismatch")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: book-navigation manifest receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingBookNavigationManifestError {}

#[allow(clippy::too_many_arguments)]
pub fn build_staging_book_navigation_manifest(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    profile: &StagingBookNavigationProfileReceipt,
    session: &StagingSemanticContainerSessionIdentity,
    selected: &BookNavigationSelectedReceipt,
    pdf: &StagingBookNavigationPdf,
    limits: &ValidatedResourceLimits,
    engine: &EngineIdentity,
) -> Result<StagingBookNavigationManifest, StagingBookNavigationManifestError> {
    let manifest = derive_manifest(
        package, navigation, profile, session, selected, pdf, limits, engine,
    )?;
    manifest.verify(
        package, navigation, profile, session, selected, pdf, limits, engine,
    )?;
    Ok(manifest)
}

#[allow(clippy::too_many_arguments)]
fn derive_manifest(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    profile: &StagingBookNavigationProfileReceipt,
    session: &StagingSemanticContainerSessionIdentity,
    selected: &BookNavigationSelectedReceipt,
    pdf: &StagingBookNavigationPdf,
    limits: &ValidatedResourceLimits,
    engine: &EngineIdentity,
) -> Result<StagingBookNavigationManifest, StagingBookNavigationManifestError> {
    profile
        .verify(package, navigation, limits, session)
        .map_err(|_| StagingBookNavigationManifestError::ProfileMismatch)?;
    selected
        .verify(navigation, profile.authorization(), limits)
        .map_err(|_| StagingBookNavigationManifestError::SelectedMismatch)?;
    pdf.verify(
        navigation,
        profile.authorization(),
        selected,
        limits,
        engine,
    )
    .map_err(|_| StagingBookNavigationManifestError::PdfMismatch)?;
    validate_outline_closure(navigation, selected, pdf)?;

    let package_sha256 = package.canonical_jcs_sha256();
    let semantic_sha256 = package.semantic_fingerprint();
    let profile_preflight_sha256 = profile.fingerprint();
    let selected_sha256 = selected.fingerprint();
    let pdf_observation_sha256 = pdf.observation().fingerprint();
    let canonical_jcs =
        encode_manifest(package, navigation, profile, selected, pdf, limits, engine);
    Ok(StagingBookNavigationManifest {
        package_sha256,
        semantic_sha256,
        profile_preflight_sha256,
        selected_sha256,
        pdf_observation_sha256,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn validate_outline_closure(
    navigation: &ValidatedStagingBookNavigation,
    selected: &BookNavigationSelectedReceipt,
    pdf: &StagingBookNavigationPdf,
) -> Result<(), StagingBookNavigationManifestError> {
    if navigation.outline().entries().len() != selected.entries().len()
        || selected.entries().len() != pdf.observation().outline_items().len()
    {
        return Err(StagingBookNavigationManifestError::OutlineMismatch);
    }
    for ((source, selected), observed) in navigation
        .outline()
        .entries()
        .iter()
        .zip(selected.entries())
        .zip(pdf.observation().outline_items())
    {
        if source.outline_id != selected.outline_id()
            || source.parent_outline_id != selected.parent_outline_id()
            || source.level != selected.level()
            || source.label != selected.label()
            || source.source.node_id != selected.source_node_id()
            || source.source.computed_language != selected.source_language()
            || source.destination != selected.destination().anchor_id
            || observed.outline_id() != source.outline_id
            || observed.source_node_id() != source.source.node_id.get()
            || observed.title() != source.label
            || observed.destination() != source.destination.as_str()
            || observed.structure_element_object().is_some()
        {
            return Err(StagingBookNavigationManifestError::OutlineMismatch);
        }
    }
    Ok(())
}

fn encode_manifest(
    package: &ValidatedStagingSemanticPackage,
    navigation: &ValidatedStagingBookNavigation,
    profile: &StagingBookNavigationProfileReceipt,
    selected: &BookNavigationSelectedReceipt,
    pdf: &StagingBookNavigationPdf,
    limits: &ValidatedResourceLimits,
    engine: &EngineIdentity,
) -> String {
    let metadata = navigation.metadata().metadata();
    let observation = pdf.observation();
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_BOOK_NAVIGATION_MANIFEST_ALGORITHM);
    output.push_str(",\"contract\":\"typaxis.contract/1.4\",\"document_language\":");
    push_jcs_string(&mut output, navigation.languages().document_language());
    output.push_str(",\"engine\":{\"name\":");
    push_jcs_string(&mut output, engine.name());
    output.push_str(",\"version\":");
    push_jcs_string(&mut output, engine.version());
    output.push_str("},\"fingerprints\":{\"destination_registry_sha256\":");
    push_hash(&mut output, selected.destination_registry_sha256());
    output.push_str(",\"language_sha256\":");
    push_hash(&mut output, navigation.languages().fingerprint());
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, selected.limits_sha256());
    output.push_str(",\"metadata_sha256\":");
    push_hash(&mut output, navigation.metadata().fingerprint());
    output.push_str(",\"outline_sha256\":");
    push_hash(&mut output, navigation.outline().fingerprint());
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, package.canonical_jcs_sha256());
    output.push_str(",\"pdf_observation_sha256\":");
    push_hash(&mut output, observation.fingerprint());
    output.push_str(",\"pdf_sha256\":");
    push_hash(&mut output, observation.pdf_sha256());
    output.push_str(",\"profile_authorization_sha256\":");
    push_hash(&mut output, profile.authorization().fingerprint());
    output.push_str(",\"profile_preflight_sha256\":");
    push_hash(&mut output, profile.fingerprint());
    output.push_str(",\"selected_sha256\":");
    push_hash(&mut output, selected.fingerprint());
    output.push_str(",\"semantic_sha256\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push_str(",\"xmp_sha256\":");
    push_hash(&mut output, observation.xmp_sha256());
    output.push_str("},\"languages\":[");
    for (index, record) in navigation.languages().records().iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"effective_language\":");
        push_jcs_string(&mut output, &record.effective_language);
        output.push_str(",\"explicit_language\":");
        push_nullable_string(&mut output, record.explicit_language.as_deref());
        output.push_str(",\"logical_parent_node_id\":");
        push_optional_u32(
            &mut output,
            record.logical_parent_node_id.map(|value| value.get()),
        );
        output.push_str(",\"node_id\":");
        output.push_str(&record.node_id.get().to_string());
        output.push_str(",\"node_kind\":");
        push_jcs_string(&mut output, record.node_kind.as_str());
        output.push('}');
    }
    output.push_str("],\"metadata\":{\"author\":");
    push_nullable_string(&mut output, metadata.author.as_deref());
    output.push_str(",\"created\":");
    push_nullable_string(&mut output, metadata.created.as_deref());
    output.push_str(",\"identifier\":");
    push_nullable_string(&mut output, metadata.identifier.as_deref());
    output.push_str(",\"keywords\":[");
    for (index, keyword) in metadata.keywords.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, keyword);
    }
    output.push_str("],\"modified\":");
    push_nullable_string(&mut output, metadata.modified.as_deref());
    output.push_str(",\"subject\":");
    push_nullable_string(&mut output, metadata.subject.as_deref());
    output.push_str(",\"title\":");
    push_nullable_string(&mut output, metadata.title.as_deref());
    output.push_str("},\"outline\":[");
    for (index, ((source, selected), observed)) in navigation
        .outline()
        .entries()
        .iter()
        .zip(selected.entries())
        .zip(observation.outline_items())
        .enumerate()
    {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"destination\":");
        push_jcs_string(&mut output, source.destination.as_str());
        output.push_str(",\"frame_id\":");
        output.push_str(&selected.frame_id().to_string());
        output.push_str(",\"label\":");
        push_jcs_string(&mut output, &source.label);
        output.push_str(",\"level\":");
        output.push_str(&source.level.to_string());
        output.push_str(",\"outline_id\":");
        output.push_str(&source.outline_id.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&selected.destination().page_index.to_string());
        output.push_str(",\"parent_outline_id\":");
        push_optional_u32(&mut output, source.parent_outline_id);
        output.push_str(",\"pdf_object_number\":");
        output.push_str(&observed.object_number().to_string());
        output.push_str(",\"source_kind\":");
        push_jcs_string(&mut output, source.source.kind.as_str());
        output.push_str(",\"source_language\":");
        push_jcs_string(&mut output, &source.source.computed_language);
        output.push_str(",\"source_node_id\":");
        output.push_str(&source.source.node_id.get().to_string());
        output.push_str(",\"view\":");
        push_view(&mut output, &selected.destination().view);
        output.push('}');
    }
    output.push_str("],\"pdf\":{\"byte_length\":");
    output.push_str(&observation.pdf_byte_length().to_string());
    output.push_str(",\"catalog_object\":");
    output.push_str(&observation.catalog_object().to_string());
    output.push_str(",\"info_object\":");
    output.push_str(&observation.info_object().to_string());
    output.push_str(",\"metadata_object\":");
    output.push_str(&observation.metadata_object().to_string());
    output.push_str(",\"outline_root_object\":");
    push_optional_u32(&mut output, observation.outline_root_object());
    output.push_str(",\"producer\":");
    push_jcs_string(&mut output, observation.producer());
    output.push_str("},\"profile_id\":");
    push_jcs_string(
        &mut output,
        typaxis_machine_profile::StagingBookNavigationProfileDescriptor::PROFILE_ID,
    );
    output.push_str(",\"resource_limits\":{\"max_fragments\":");
    output.push_str(&limits.get().max_fragments.to_string());
    output.push_str(",\"max_output_bytes\":");
    output.push_str(&limits.get().max_output_bytes.to_string());
    output.push_str(",\"max_pdf_objects\":");
    output.push_str(&limits.get().max_pdf_objects.to_string());
    output.push_str(",\"max_spool_bytes\":");
    output.push_str(&limits.get().max_spool_bytes.to_string());
    output.push_str("}}");
    output
}

fn push_view(output: &mut String, value: &DestinationView) {
    match value {
        DestinationView::Xyz { point } => {
            output.push_str("{\"kind\":\"xyz\",\"x\":");
            output.push_str(&point.x.raw().to_string());
            output.push_str(",\"y\":");
            output.push_str(&point.y.raw().to_string());
            output.push('}');
        }
        DestinationView::FitPage => output.push_str("{\"kind\":\"fit_page\"}"),
        DestinationView::FitWidth { top } => {
            output.push_str("{\"kind\":\"fit_width\",\"top\":");
            if let Some(top) = top {
                output.push_str(&top.raw().to_string());
            } else {
                output.push_str("null");
            }
            output.push('}');
        }
    }
}

fn push_nullable_string(output: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        push_jcs_string(output, value);
    } else {
        output.push_str("null");
    }
}

fn push_optional_u32(output: &mut String, value: Option<u32>) {
    if let Some(value) = value {
        output.push_str(&value.to_string());
    } else {
        output.push_str("null");
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
