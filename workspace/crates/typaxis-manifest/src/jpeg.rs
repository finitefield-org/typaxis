use typaxis_core::{
    push_jcs_string, sha256, ImageResourceId, M4EffectiveResourceLimits, NodeId, Rect,
};
use typaxis_document::{ImageMediaDeclaration, ImageMediaType};
use typaxis_layout::StagingJpegSelectedLayout;
use typaxis_machine_profile::StagingJpegProfileReceipt;
use typaxis_pdf::{StagingJpegPdf, StagingJpegPdfImageObject};
use typaxis_resource_admission::{
    close_staging_declared_media, AdmittedImageMediaKind, AdmittedResourceLedger,
    StagingDeclaredMediaLedger, JPEG_DECODER_ID, JPEG_MARKER_PREFLIGHT_ID,
    JPEG_PIXEL_OBSERVATION_ID, JPEG_RESOURCE_PROFILE_ID, JPEG_SANITIZER_ID,
};
use typaxis_syntax::ValidatedStagingSemanticPackage;

pub const STAGING_JPEG_MANIFEST_ALGORITHM: &str = "typaxis.jpeg-manifest/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingJpegManifestError {
    ProfileMismatch,
    AdmissionClosure,
    AdmissionMismatch(ImageResourceId),
    MediaMismatch(ImageResourceId),
    LayoutMismatch,
    DisplayMismatch,
    PdfMismatch,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingJpegManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for StagingJpegManifestError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegManifestUsage {
    occurrence: u32,
    owner: NodeId,
    page_index: u32,
    bounds: Rect,
    alternative_sha256: [u8; 32],
    selected_placement_fingerprint: [u8; 32],
    display_draw_fingerprint: [u8; 32],
}

impl StagingJpegManifestUsage {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
    pub const fn alternative_sha256(&self) -> [u8; 32] {
        self.alternative_sha256
    }
    pub const fn selected_placement_fingerprint(&self) -> [u8; 32] {
        self.selected_placement_fingerprint
    }
    pub const fn display_draw_fingerprint(&self) -> [u8; 32] {
        self.display_draw_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegManifestResource {
    image_id: ImageResourceId,
    uri: String,
    declared_media_type: &'static str,
    attested_media_kind: &'static str,
    source_byte_length: u64,
    source_sha256: [u8; 32],
    width: u32,
    height: u32,
    color: &'static str,
    sampling: &'static str,
    decoded_byte_length: u64,
    peak_workspace_bytes: u64,
    pixel_sha256: [u8; 32],
    normalized_byte_length: u64,
    normalized_sha256: [u8; 32],
    usages: Vec<StagingJpegManifestUsage>,
    pdf_plan_fingerprint: Option<[u8; 32]>,
    pdf_object_number: Option<u32>,
    pdf_resource_name: Option<String>,
    pdf_color_transform: Option<u8>,
}

impl StagingJpegManifestResource {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub const fn declared_media_type(&self) -> &'static str {
        self.declared_media_type
    }
    pub const fn attested_media_kind(&self) -> &'static str {
        self.attested_media_kind
    }
    pub const fn source_byte_length(&self) -> u64 {
        self.source_byte_length
    }
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub const fn width(&self) -> u32 {
        self.width
    }
    pub const fn height(&self) -> u32 {
        self.height
    }
    pub const fn color(&self) -> &'static str {
        self.color
    }
    pub const fn sampling(&self) -> &'static str {
        self.sampling
    }
    pub const fn decoded_byte_length(&self) -> u64 {
        self.decoded_byte_length
    }
    pub const fn peak_workspace_bytes(&self) -> u64 {
        self.peak_workspace_bytes
    }
    pub const fn pixel_sha256(&self) -> [u8; 32] {
        self.pixel_sha256
    }
    pub const fn normalized_byte_length(&self) -> u64 {
        self.normalized_byte_length
    }
    pub const fn normalized_sha256(&self) -> [u8; 32] {
        self.normalized_sha256
    }
    pub fn usages(&self) -> &[StagingJpegManifestUsage] {
        &self.usages
    }
    pub const fn pdf_plan_fingerprint(&self) -> Option<[u8; 32]> {
        self.pdf_plan_fingerprint
    }
    pub const fn pdf_object_number(&self) -> Option<u32> {
        self.pdf_object_number
    }
    pub fn pdf_resource_name(&self) -> Option<&str> {
        self.pdf_resource_name.as_deref()
    }
    pub const fn pdf_color_transform(&self) -> Option<u8> {
        self.pdf_color_transform
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegManifest {
    resources: Vec<StagingJpegManifestResource>,
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    authorization_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    declared_media_fingerprint: [u8; 32],
    selected_layout_fingerprint: [u8; 32],
    selected_state_fingerprint: [u8; 32],
    display_fingerprint: [u8; 32],
    pdf_closure_fingerprint: [u8; 32],
    pdf_sha256: [u8; 32],
    page_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingJpegManifest {
    pub fn resources(&self) -> &[StagingJpegManifestResource] {
        &self.resources
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingJpegProfileReceipt,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        media: &StagingDeclaredMediaLedger,
        selected: &StagingJpegSelectedLayout,
        pdf: &StagingJpegPdf,
    ) -> Result<(), StagingJpegManifestError> {
        let expected = assemble(package, profile, limits, admitted, media, selected, pdf)?;
        if self != &expected {
            return Err(StagingJpegManifestError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_staging_jpeg_manifest(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingJpegProfileReceipt,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    media: &StagingDeclaredMediaLedger,
    selected: &StagingJpegSelectedLayout,
    pdf: &StagingJpegPdf,
) -> Result<StagingJpegManifest, StagingJpegManifestError> {
    assemble(package, profile, limits, admitted, media, selected, pdf)
}

#[allow(clippy::too_many_arguments)]
fn assemble(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingJpegProfileReceipt,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    media: &StagingDeclaredMediaLedger,
    selected: &StagingJpegSelectedLayout,
    pdf: &StagingJpegPdf,
) -> Result<StagingJpegManifest, StagingJpegManifestError> {
    profile
        .verify(package, limits)
        .map_err(|_| StagingJpegManifestError::ProfileMismatch)?;
    selected
        .verify(package, profile.authorization(), limits, admitted)
        .map_err(|_| StagingJpegManifestError::LayoutMismatch)?;
    pdf.verify(admitted, limits)
        .map_err(|_| StagingJpegManifestError::PdfMismatch)?;
    if pdf.display_facts().selected_layout_fingerprint() != selected.fingerprint()
        || pdf.display_facts().selected_state_fingerprint() != selected.state_fingerprint()
    {
        return Err(StagingJpegManifestError::DisplayMismatch);
    }
    let expected_media = close_staging_declared_media(admitted, package.resources())
        .map_err(|_| StagingJpegManifestError::AdmissionClosure)?;
    if &expected_media != media || media.fingerprint() != selected.declared_media_fingerprint() {
        return Err(StagingJpegManifestError::AdmissionClosure);
    }

    let mut resources = Vec::new();
    resources
        .try_reserve_exact(package.resources().images.len())
        .map_err(|_| StagingJpegManifestError::AllocationFailure)?;
    for declaration in &package.resources().images {
        if declaration.media != ImageMediaDeclaration::Declared(ImageMediaType::JpegBaseline) {
            return Err(StagingJpegManifestError::MediaMismatch(
                declaration.image_id,
            ));
        }
        let admitted_image = admitted.image(declaration.image_id).ok_or(
            StagingJpegManifestError::AdmissionMismatch(declaration.image_id),
        )?;
        let declared = media
            .images()
            .iter()
            .find(|image| image.image_id() == declaration.image_id)
            .ok_or(StagingJpegManifestError::MediaMismatch(
                declaration.image_id,
            ))?;
        let jpeg = declared
            .jpeg_attestation()
            .ok_or(StagingJpegManifestError::MediaMismatch(
                declaration.image_id,
            ))?;
        if declared.declared() != ImageMediaType::JpegBaseline
            || declared.attested() != AdmittedImageMediaKind::JpegBaseline
            || declared.content_hash() != admitted_image.content_hash()
            || admitted_image.jpeg_attestation() != Some(jpeg)
            || jpeg.limits_fingerprint() != limits.fingerprint()
            || jpeg.profile_fingerprint() != profile.authorization().profile_fingerprint()
        {
            return Err(StagingJpegManifestError::MediaMismatch(
                declaration.image_id,
            ));
        }
        let pdf_resource = pdf
            .facts()
            .resources()
            .iter()
            .find(|resource| resource.image_id() == declaration.image_id);
        let usages = assemble_usages(declaration.image_id, selected, pdf)?;
        if usages.is_empty() != pdf_resource.is_none()
            || pdf_resource.is_some_and(|resource| {
                resource.placement_count() != u32::try_from(usages.len()).unwrap_or(u32::MAX)
                    || !pdf_resource_matches_admission(resource, jpeg)
            })
        {
            return Err(StagingJpegManifestError::PdfMismatch);
        }
        resources.push(StagingJpegManifestResource {
            image_id: declaration.image_id,
            uri: declaration.uri.as_str().to_owned(),
            declared_media_type: ImageMediaType::JpegBaseline.as_str(),
            attested_media_kind: AdmittedImageMediaKind::JpegBaseline.as_str(),
            source_byte_length: admitted_image.byte_length(),
            source_sha256: jpeg.source_sha256(),
            width: jpeg.width().get(),
            height: jpeg.height().get(),
            color: jpeg.color_kind().as_str(),
            sampling: jpeg.sampling().as_str(),
            decoded_byte_length: jpeg.decoded_byte_length(),
            peak_workspace_bytes: jpeg.peak_workspace_bytes(),
            pixel_sha256: jpeg.pixel_sha256(),
            normalized_byte_length: u64::try_from(jpeg.normalized_bytes().len())
                .map_err(|_| StagingJpegManifestError::MediaMismatch(declaration.image_id))?,
            normalized_sha256: jpeg.normalized_sha256(),
            usages,
            pdf_plan_fingerprint: pdf_resource.map(StagingJpegPdfImageObject::plan_fingerprint),
            pdf_object_number: pdf_resource.map(StagingJpegPdfImageObject::object_number),
            pdf_resource_name: pdf_resource.map(|resource| resource.resource_name().to_owned()),
            pdf_color_transform: pdf_resource.map(StagingJpegPdfImageObject::color_transform),
        });
    }
    let canonical_jcs = encode_manifest(
        package.semantic_fingerprint(),
        profile.fingerprint(),
        profile.authorization().profile_fingerprint(),
        limits.fingerprint(),
        admitted.fingerprint().bytes(),
        media.fingerprint(),
        selected.fingerprint(),
        selected.state_fingerprint().bytes(),
        pdf.display_facts().fingerprint(),
        pdf.facts().fingerprint(),
        pdf.pdf().content_hash(),
        pdf.pdf().page_count(),
        &resources,
    );
    Ok(StagingJpegManifest {
        resources,
        package_fingerprint: package.semantic_fingerprint(),
        profile_fingerprint: profile.fingerprint(),
        authorization_fingerprint: profile.authorization().profile_fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        admitted_fingerprint: admitted.fingerprint().bytes(),
        declared_media_fingerprint: media.fingerprint(),
        selected_layout_fingerprint: selected.fingerprint(),
        selected_state_fingerprint: selected.state_fingerprint().bytes(),
        display_fingerprint: pdf.display_facts().fingerprint(),
        pdf_closure_fingerprint: pdf.facts().fingerprint(),
        pdf_sha256: pdf.pdf().content_hash(),
        page_count: pdf.pdf().page_count(),
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn assemble_usages(
    image_id: ImageResourceId,
    selected: &StagingJpegSelectedLayout,
    pdf: &StagingJpegPdf,
) -> Result<Vec<StagingJpegManifestUsage>, StagingJpegManifestError> {
    let mut usages = Vec::new();
    for placement in selected
        .placements()
        .iter()
        .filter(|placement| placement.image_id() == image_id)
    {
        let draw = pdf
            .display_facts()
            .draws()
            .iter()
            .find(|draw| draw.occurrence() == placement.occurrence())
            .ok_or(StagingJpegManifestError::DisplayMismatch)?;
        if draw.image_id() != image_id
            || draw.page_index() != placement.page_index()
            || draw.rect() != placement.rect()
            || draw.placement_fingerprint() != placement.fingerprint()
        {
            return Err(StagingJpegManifestError::DisplayMismatch);
        }
        usages.push(StagingJpegManifestUsage {
            occurrence: placement.occurrence(),
            owner: placement.owner(),
            page_index: placement.page_index(),
            bounds: placement.rect(),
            alternative_sha256: sha256(placement.alternative().as_bytes()),
            selected_placement_fingerprint: placement.fingerprint(),
            display_draw_fingerprint: draw.placement_fingerprint(),
        });
    }
    Ok(usages)
}

fn pdf_resource_matches_admission(
    resource: &StagingJpegPdfImageObject,
    jpeg: &typaxis_resource_admission::JpegAdmissionAttestation,
) -> bool {
    resource.source_sha256() == jpeg.source_sha256()
        && resource.normalized_byte_length()
            == u64::try_from(jpeg.normalized_bytes().len()).unwrap_or(u64::MAX)
        && resource.normalized_sha256() == jpeg.normalized_sha256()
        && resource.pixel_sha256() == jpeg.pixel_sha256()
        && resource.width() == jpeg.width().get()
        && resource.height() == jpeg.height().get()
        && resource.decoded_byte_length() == jpeg.decoded_byte_length()
        && resource.peak_workspace_bytes() == jpeg.peak_workspace_bytes()
        && resource.color_kind() == jpeg.color_kind()
        && resource.sampling() == jpeg.sampling()
        && resource.limits_fingerprint() == jpeg.limits_fingerprint()
        && resource.profile_fingerprint() == jpeg.profile_fingerprint()
}

#[allow(clippy::too_many_arguments)]
fn encode_manifest(
    package: [u8; 32],
    profile: [u8; 32],
    authorization: [u8; 32],
    limits: [u8; 32],
    admitted: [u8; 32],
    media: [u8; 32],
    layout: [u8; 32],
    state: [u8; 32],
    display: [u8; 32],
    pdf_closure: [u8; 32],
    pdf: [u8; 32],
    page_count: u32,
    resources: &[StagingJpegManifestResource],
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_JPEG_MANIFEST_ALGORITHM);
    output.push_str(",\"authorization_fingerprint\":");
    push_hash(&mut output, authorization);
    output.push_str(",\"component_ids\":{");
    output.push_str("\"decoder\":");
    push_jcs_string(&mut output, JPEG_DECODER_ID);
    output.push_str(",\"marker_preflight\":");
    push_jcs_string(&mut output, JPEG_MARKER_PREFLIGHT_ID);
    output.push_str(",\"pdf_plan\":");
    push_jcs_string(&mut output, typaxis_resources::JPEG_PDF_PLAN_ID);
    output.push_str(",\"pixel_observation\":");
    push_jcs_string(&mut output, JPEG_PIXEL_OBSERVATION_ID);
    output.push_str(",\"resource_profile\":");
    push_jcs_string(&mut output, JPEG_RESOURCE_PROFILE_ID);
    output.push_str(",\"sanitizer\":");
    push_jcs_string(&mut output, JPEG_SANITIZER_ID);
    output.push_str("},\"declared_media_fingerprint\":");
    push_hash(&mut output, media);
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits);
    output.push_str(",\"package_fingerprint\":");
    push_hash(&mut output, package);
    output.push_str(",\"page_count\":");
    output.push_str(&page_count.to_string());
    output.push_str(",\"pdf_closure_fingerprint\":");
    push_hash(&mut output, pdf_closure);
    output.push_str(",\"pdf_sha256\":");
    push_hash(&mut output, pdf);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, profile);
    output.push_str(",\"resources\":[");
    for (index, resource) in resources.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_resource(&mut output, resource);
    }
    output.push_str("],\"selected_layout_fingerprint\":");
    push_hash(&mut output, layout);
    output.push_str(",\"selected_state_fingerprint\":");
    push_hash(&mut output, state);
    output.push('}');
    output
}

fn encode_resource(output: &mut String, resource: &StagingJpegManifestResource) {
    output.push_str("{\"attested_media_kind\":");
    push_jcs_string(output, resource.attested_media_kind);
    output.push_str(",\"color\":");
    push_jcs_string(output, resource.color);
    output.push_str(",\"declared_media_type\":");
    push_jcs_string(output, resource.declared_media_type);
    output.push_str(",\"decoded_byte_length\":");
    output.push_str(&resource.decoded_byte_length.to_string());
    output.push_str(",\"height\":");
    output.push_str(&resource.height.to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&resource.image_id.get().to_string());
    output.push_str(",\"normalized_byte_length\":");
    output.push_str(&resource.normalized_byte_length.to_string());
    output.push_str(",\"normalized_sha256\":");
    push_hash(output, resource.normalized_sha256);
    output.push_str(",\"pdf_color_transform\":");
    push_optional_u8(output, resource.pdf_color_transform);
    output.push_str(",\"pdf_object_number\":");
    push_optional_u32(output, resource.pdf_object_number);
    output.push_str(",\"pdf_plan_fingerprint\":");
    push_optional_hash(output, resource.pdf_plan_fingerprint);
    output.push_str(",\"pdf_resource_name\":");
    match &resource.pdf_resource_name {
        Some(value) => push_jcs_string(output, value),
        None => output.push_str("null"),
    }
    output.push_str(",\"peak_workspace_bytes\":");
    output.push_str(&resource.peak_workspace_bytes.to_string());
    output.push_str(",\"pixel_sha256\":");
    push_hash(output, resource.pixel_sha256);
    output.push_str(",\"sampling\":");
    push_jcs_string(output, resource.sampling);
    output.push_str(",\"source_byte_length\":");
    output.push_str(&resource.source_byte_length.to_string());
    output.push_str(",\"source_sha256\":");
    push_hash(output, resource.source_sha256);
    output.push_str(",\"uri\":");
    push_jcs_string(output, &resource.uri);
    output.push_str(",\"usages\":[");
    for (index, usage) in resource.usages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"alternative_sha256\":");
        push_hash(output, usage.alternative_sha256);
        output.push_str(",\"bounds\":[");
        output.push_str(&usage.bounds.x().raw().to_string());
        output.push(',');
        output.push_str(&usage.bounds.y().raw().to_string());
        output.push(',');
        output.push_str(&usage.bounds.width().get().raw().to_string());
        output.push(',');
        output.push_str(&usage.bounds.height().get().raw().to_string());
        output.push_str("],\"display_draw_fingerprint\":");
        push_hash(output, usage.display_draw_fingerprint);
        output.push_str(",\"occurrence\":");
        output.push_str(&usage.occurrence.to_string());
        output.push_str(",\"owner\":");
        output.push_str(&usage.owner.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&usage.page_index.to_string());
        output.push_str(",\"selected_placement_fingerprint\":");
        push_hash(output, usage.selected_placement_fingerprint);
        output.push('}');
    }
    output.push_str("],\"width\":");
    output.push_str(&resource.width.to_string());
    output.push('}');
}

fn push_optional_u8(output: &mut String, value: Option<u8>) {
    match value {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
}

fn push_optional_u32(output: &mut String, value: Option<u32>) {
    match value {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
}

fn push_optional_hash(output: &mut String, value: Option<[u8; 32]>) {
    match value {
        Some(value) => push_hash(output, value),
        None => output.push_str("null"),
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
