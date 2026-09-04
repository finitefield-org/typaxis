use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{
    push_jcs_string, sha256, AdmittedResourceFingerprint, EffectiveConfig,
    EffectiveConfigFingerprint, ImageResourceId, LayoutStateFingerprint, M4EffectiveResourceLimits,
};
use typaxis_display_list::{StagingJpegDisplay, StagingJpegDisplayFacts};
use typaxis_resource_admission::{
    AdmittedImageMediaKind, AdmittedResourceLedger, JpegColorKind, JpegSampling,
};
use typaxis_resources::{
    FrozenPdfResourcePlans, ImageColorSpace, ImageEncoding, ReferenceResourceFinalizer,
    ResourceFinalizationInput, ResourceFinalizer, JPEG_PDF_PLAN_ID,
};

use super::{PdfBackend, PdfError, VerifiedPdfBytesReceipt};

pub const STAGING_JPEG_PDF_CLOSURE_ALGORITHM: &str = "typaxis.jpeg-pdf-closure/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingJpegPdfError {
    DisplayMismatch,
    AdmissionMismatch,
    Finalization,
    ResourceClosure,
    Pdf(PdfError),
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingJpegPdfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for StagingJpegPdfError {}

impl From<PdfError> for StagingJpegPdfError {
    fn from(error: PdfError) -> Self {
        Self::Pdf(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegPdfImageObject {
    image_id: ImageResourceId,
    resource_name: String,
    object_number: u32,
    placement_count: u32,
    source_sha256: [u8; 32],
    normalized_byte_length: u64,
    normalized_sha256: [u8; 32],
    pixel_sha256: [u8; 32],
    width: u32,
    height: u32,
    decoded_byte_length: u64,
    peak_workspace_bytes: u64,
    color_kind: JpegColorKind,
    sampling: JpegSampling,
    color_transform: u8,
    limits_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    plan_fingerprint: [u8; 32],
}

impl StagingJpegPdfImageObject {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }
    pub const fn object_number(&self) -> u32 {
        self.object_number
    }
    pub const fn placement_count(&self) -> u32 {
        self.placement_count
    }
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub const fn normalized_byte_length(&self) -> u64 {
        self.normalized_byte_length
    }
    pub const fn normalized_sha256(&self) -> [u8; 32] {
        self.normalized_sha256
    }
    pub const fn pixel_sha256(&self) -> [u8; 32] {
        self.pixel_sha256
    }
    pub const fn width(&self) -> u32 {
        self.width
    }
    pub const fn height(&self) -> u32 {
        self.height
    }
    pub const fn decoded_byte_length(&self) -> u64 {
        self.decoded_byte_length
    }
    pub const fn peak_workspace_bytes(&self) -> u64 {
        self.peak_workspace_bytes
    }
    pub const fn color_kind(&self) -> JpegColorKind {
        self.color_kind
    }
    pub const fn sampling(&self) -> JpegSampling {
        self.sampling
    }
    pub const fn color_transform(&self) -> u8 {
        self.color_transform
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub const fn plan_fingerprint(&self) -> [u8; 32] {
        self.plan_fingerprint
    }
    pub const fn plan_id(&self) -> &'static str {
        JPEG_PDF_PLAN_ID
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingJpegPdfFacts {
    display_fingerprint: [u8; 32],
    selected_state_fingerprint: LayoutStateFingerprint,
    admitted_fingerprint: AdmittedResourceFingerprint,
    limits_fingerprint: [u8; 32],
    config_fingerprint: EffectiveConfigFingerprint,
    page_count: u32,
    object_count: u32,
    resources: Vec<StagingJpegPdfImageObject>,
    pdf_sha256: [u8; 32],
    pdf_byte_length: u64,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingJpegPdfFacts {
    pub const fn display_fingerprint(&self) -> [u8; 32] {
        self.display_fingerprint
    }
    pub const fn selected_state_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_state_fingerprint
    }
    pub const fn admitted_fingerprint(&self) -> AdmittedResourceFingerprint {
        self.admitted_fingerprint
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub const fn config_fingerprint(&self) -> EffectiveConfigFingerprint {
        self.config_fingerprint
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub fn resources(&self) -> &[StagingJpegPdfImageObject] {
        &self.resources
    }
    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub const fn pdf_byte_length(&self) -> u64 {
        self.pdf_byte_length
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        display: &StagingJpegDisplayFacts,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
        pdf: &VerifiedPdfBytesReceipt,
    ) -> Result<(), StagingJpegPdfError> {
        let canonical = encode_pdf_facts(
            display.fingerprint(),
            display.selected_state_fingerprint(),
            admitted.fingerprint(),
            limits.fingerprint(),
            self.config_fingerprint,
            self.page_count,
            self.object_count,
            &self.resources,
            pdf.content_hash(),
            pdf.byte_length(),
        );
        if self.display_fingerprint != display.fingerprint()
            || self.selected_state_fingerprint != display.selected_state_fingerprint()
            || self.admitted_fingerprint != admitted.fingerprint()
            || self.limits_fingerprint != limits.fingerprint()
            || self.page_count != display.page_count()
            || self.page_count != pdf.page_count()
            || self.object_count != pdf.object_count()
            || self.pdf_sha256 != pdf.content_hash()
            || self.pdf_byte_length != pdf.byte_length()
            || pdf.selected_layout_fingerprint() != display.selected_state_fingerprint()
            || pdf.config_fingerprint() != self.config_fingerprint
            || self.canonical_jcs != canonical
            || self.fingerprint != sha256(canonical.as_bytes())
            || !resources_match_admission_and_usage(&self.resources, display, admitted, limits)
            || !serialized_pdf_matches_resources(pdf.bytes(), &self.resources, admitted)
        {
            return Err(StagingJpegPdfError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct StagingJpegPdf {
    pdf: VerifiedPdfBytesReceipt,
    display: StagingJpegDisplayFacts,
    facts: StagingJpegPdfFacts,
}

impl StagingJpegPdf {
    pub const fn pdf(&self) -> &VerifiedPdfBytesReceipt {
        &self.pdf
    }
    pub const fn display_facts(&self) -> &StagingJpegDisplayFacts {
        &self.display
    }
    pub const fn facts(&self) -> &StagingJpegPdfFacts {
        &self.facts
    }
    pub fn verify(
        &self,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingJpegPdfError> {
        self.facts
            .verify(&self.display, admitted, limits, &self.pdf)
    }
    pub fn into_pdf(self) -> VerifiedPdfBytesReceipt {
        self.pdf
    }
}

pub fn write_staging_jpeg_pdf(
    display: StagingJpegDisplay,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    config: &EffectiveConfig,
) -> Result<StagingJpegPdf, StagingJpegPdfError> {
    if config.limits() != limits.base() {
        return Err(StagingJpegPdfError::AdmissionMismatch);
    }
    let (trusted, display_facts) = display.into_parts();
    if trusted.document().source_layout().state_fingerprint()
        != display_facts.selected_state_fingerprint()
        || trusted.document().pages.len()
            != usize::try_from(display_facts.page_count())
                .map_err(|_| StagingJpegPdfError::DisplayMismatch)?
    {
        return Err(StagingJpegPdfError::DisplayMismatch);
    }

    let plans = ReferenceResourceFinalizer::new()
        .finalize(ResourceFinalizationInput {
            display: &trusted,
            admitted,
            limits: limits.base(),
        })
        .map_err(|_| StagingJpegPdfError::Finalization)?;
    let plan_facts = collect_plan_facts(&plans, &display_facts, admitted, limits)?;
    let graph = PdfBackend::build(trusted, plans, limits.base())?;
    let resource_objects: BTreeMap<_, _> = graph
        .image_resource_objects()
        .map(|(image_id, name, object)| (image_id, (name.encoded(), object)))
        .collect();
    if resource_objects.len() != plan_facts.len()
        || resource_objects.keys().copied().collect::<BTreeSet<_>>()
            != plan_facts
                .iter()
                .map(StagingJpegPdfImageObject::image_id)
                .collect()
    {
        return Err(StagingJpegPdfError::ResourceClosure);
    }
    let mut resources = Vec::new();
    resources
        .try_reserve_exact(plan_facts.len())
        .map_err(|_| StagingJpegPdfError::AllocationFailure)?;
    for mut facts in plan_facts {
        let (name, object) = resource_objects
            .get(&facts.image_id)
            .ok_or(StagingJpegPdfError::ResourceClosure)?;
        facts.resource_name =
            String::from_utf8(name.clone()).map_err(|_| StagingJpegPdfError::ResourceClosure)?;
        facts.object_number = object.get();
        resources.push(facts);
    }
    let page_count = graph.page_count();
    let object_count = graph.object_count();
    if page_count != display_facts.page_count()
        || graph.selected_layout_fingerprint() != display_facts.selected_state_fingerprint()
    {
        return Err(StagingJpegPdfError::ResourceClosure);
    }
    let pdf = PdfBackend::serialize(graph, config)?;
    let canonical_jcs = encode_pdf_facts(
        display_facts.fingerprint(),
        display_facts.selected_state_fingerprint(),
        admitted.fingerprint(),
        limits.fingerprint(),
        config.fingerprint(),
        page_count,
        object_count,
        &resources,
        pdf.content_hash(),
        pdf.byte_length(),
    );
    let output = StagingJpegPdf {
        facts: StagingJpegPdfFacts {
            display_fingerprint: display_facts.fingerprint(),
            selected_state_fingerprint: display_facts.selected_state_fingerprint(),
            admitted_fingerprint: admitted.fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            config_fingerprint: config.fingerprint(),
            page_count,
            object_count,
            resources,
            pdf_sha256: pdf.content_hash(),
            pdf_byte_length: pdf.byte_length(),
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        pdf,
        display: display_facts,
    };
    output.verify(admitted, limits)?;
    Ok(output)
}

fn collect_plan_facts(
    plans: &FrozenPdfResourcePlans,
    display: &StagingJpegDisplayFacts,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<Vec<StagingJpegPdfImageObject>, StagingJpegPdfError> {
    if !plans.fonts().is_empty() {
        return Err(StagingJpegPdfError::ResourceClosure);
    }
    let used = display.used_image_ids();
    if used.len() != plans.images().len() {
        return Err(StagingJpegPdfError::ResourceClosure);
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(plans.images().len())
        .map_err(|_| StagingJpegPdfError::AllocationFailure)?;
    for plan in plans.images() {
        let image = admitted
            .image(plan.image_id())
            .ok_or(StagingJpegPdfError::ResourceClosure)?;
        let admission = image
            .jpeg_attestation()
            .ok_or(StagingJpegPdfError::ResourceClosure)?;
        let jpeg = plan
            .jpeg_plan()
            .ok_or(StagingJpegPdfError::ResourceClosure)?;
        let expected_color = match admission.color_kind() {
            JpegColorKind::Grayscale => (ImageColorSpace::Gray, 0),
            JpegColorKind::YCbCr => (ImageColorSpace::Rgb, 1),
        };
        let placement_count = u32::try_from(
            display
                .draws()
                .iter()
                .filter(|draw| draw.image_id() == plan.image_id())
                .count(),
        )
        .map_err(|_| StagingJpegPdfError::ResourceClosure)?;
        if !used.contains(&plan.image_id())
            || image.media_kind() != AdmittedImageMediaKind::JpegBaseline
            || plan.admitted_sha256() != image.content_hash()
            || plan.width() != admission.width()
            || plan.height() != admission.height()
            || plan.color_space() != expected_color.0
            || plan.bits_per_component() != 8
            || plan.encoding() != ImageEncoding::Jpeg
            || plan.alpha_mask().is_some()
            || jpeg.color_transform() != expected_color.1
            || jpeg.source_sha256() != admission.source_sha256()
            || jpeg.normalized_sha256() != admission.normalized_sha256()
            || sha256(plan.encoded_bytes()) != admission.normalized_sha256()
            || jpeg.pixel_sha256() != admission.pixel_sha256()
            || jpeg.decoded_byte_length() != admission.decoded_byte_length()
            || jpeg.peak_workspace_bytes() != admission.peak_workspace_bytes()
            || jpeg.color_kind() != admission.color_kind()
            || jpeg.sampling() != admission.sampling()
            || jpeg.limits_fingerprint() != limits.fingerprint()
            || jpeg.profile_fingerprint() != admission.profile_fingerprint()
            || placement_count == 0
        {
            return Err(StagingJpegPdfError::ResourceClosure);
        }
        let normalized_byte_length = u64::try_from(plan.encoded_bytes().len())
            .map_err(|_| StagingJpegPdfError::ResourceClosure)?;
        let plan_fingerprint = encode_plan_fingerprint(plan, jpeg, normalized_byte_length);
        result.push(StagingJpegPdfImageObject {
            image_id: plan.image_id(),
            resource_name: String::new(),
            object_number: 0,
            placement_count,
            source_sha256: jpeg.source_sha256(),
            normalized_byte_length,
            normalized_sha256: jpeg.normalized_sha256(),
            pixel_sha256: jpeg.pixel_sha256(),
            width: plan.width().get(),
            height: plan.height().get(),
            decoded_byte_length: jpeg.decoded_byte_length(),
            peak_workspace_bytes: jpeg.peak_workspace_bytes(),
            color_kind: jpeg.color_kind(),
            sampling: jpeg.sampling(),
            color_transform: jpeg.color_transform(),
            limits_fingerprint: jpeg.limits_fingerprint(),
            profile_fingerprint: jpeg.profile_fingerprint(),
            plan_fingerprint,
        });
    }
    Ok(result)
}

fn resources_match_admission_and_usage(
    resources: &[StagingJpegPdfImageObject],
    display: &StagingJpegDisplayFacts,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> bool {
    let expected = display.used_image_ids();
    if resources.len() != expected.len()
        || resources
            .iter()
            .map(StagingJpegPdfImageObject::image_id)
            .collect::<BTreeSet<_>>()
            != expected
    {
        return false;
    }
    resources.iter().all(|resource| {
        let Some(image) = admitted.image(resource.image_id) else {
            return false;
        };
        let Some(jpeg) = image.jpeg_attestation() else {
            return false;
        };
        let count = display
            .draws()
            .iter()
            .filter(|draw| draw.image_id() == resource.image_id)
            .count();
        usize::try_from(resource.placement_count) == Ok(count)
            && resource.placement_count > 0
            && image.media_kind() == AdmittedImageMediaKind::JpegBaseline
            && resource.source_sha256 == jpeg.source_sha256()
            && u64::try_from(jpeg.normalized_bytes().len()) == Ok(resource.normalized_byte_length)
            && resource.normalized_sha256 == jpeg.normalized_sha256()
            && resource.pixel_sha256 == jpeg.pixel_sha256()
            && resource.width == jpeg.width().get()
            && resource.height == jpeg.height().get()
            && resource.decoded_byte_length == jpeg.decoded_byte_length()
            && resource.peak_workspace_bytes == jpeg.peak_workspace_bytes()
            && resource.color_kind == jpeg.color_kind()
            && resource.sampling == jpeg.sampling()
            && resource.color_transform
                == match jpeg.color_kind() {
                    JpegColorKind::Grayscale => 0,
                    JpegColorKind::YCbCr => 1,
                }
            && resource.limits_fingerprint == limits.fingerprint()
            && resource.profile_fingerprint == jpeg.profile_fingerprint()
            && resource.resource_name.starts_with("/Im")
            && resource.object_number > 0
    })
}

fn serialized_pdf_matches_resources(
    bytes: &[u8],
    resources: &[StagingJpegPdfImageObject],
    admitted: &AdmittedResourceLedger,
) -> bool {
    resources.iter().all(|resource| {
        let Some(normalized) = admitted
            .image(resource.image_id)
            .and_then(|image| image.jpeg_attestation())
            .map(|jpeg| jpeg.normalized_bytes())
        else {
            return false;
        };
        count_jpeg_stream_object(bytes, normalized, resource) == 1
    })
}

fn count_jpeg_stream_object(
    pdf: &[u8],
    payload: &[u8],
    resource: &StagingJpegPdfImageObject,
) -> usize {
    const SUFFIX: &[u8] = b"\nendstream\nendobj\n";
    if payload.is_empty() {
        return 0;
    }
    let color_space = match resource.color_kind {
        JpegColorKind::Grayscale => "DeviceGray",
        JpegColorKind::YCbCr => "DeviceRGB",
    };
    let prefix = format!(
        "{} 0 obj\n<< /BitsPerComponent 8 /ColorSpace /{} /DecodeParms << /ColorTransform {} >> /Filter /DCTDecode /Height {} /Length {} /Subtype /Image /Type /XObject /Width {} >>\nstream\n",
        resource.object_number,
        color_space,
        resource.color_transform,
        resource.height,
        resource.normalized_byte_length,
        resource.width,
    );
    pdf.windows(payload.len())
        .enumerate()
        .filter(|(start, bytes)| {
            *bytes == payload
                && start
                    .checked_sub(prefix.len())
                    .and_then(|prefix_start| pdf.get(prefix_start..*start))
                    == Some(prefix.as_bytes())
                && start.checked_add(payload.len()).and_then(|suffix_start| {
                    suffix_start
                        .checked_add(SUFFIX.len())
                        .and_then(|suffix_end| pdf.get(suffix_start..suffix_end))
                }) == Some(SUFFIX)
        })
        .count()
}

fn encode_plan_fingerprint(
    plan: &typaxis_resources::FrozenPdfImagePlan,
    jpeg: &typaxis_resources::FrozenPdfJpegPlan,
    normalized_byte_length: u64,
) -> [u8; 32] {
    let mut output = String::from("{\"bits_per_component\":8,\"color_kind\":");
    push_jcs_string(&mut output, jpeg.color_kind().as_str());
    output.push_str(",\"color_space\":");
    push_jcs_string(
        &mut output,
        match plan.color_space() {
            ImageColorSpace::Gray => "DeviceGray",
            ImageColorSpace::Rgb => "DeviceRGB",
            ImageColorSpace::Cmyk => "DeviceCMYK",
        },
    );
    output.push_str(",\"color_transform\":");
    output.push_str(&jpeg.color_transform().to_string());
    output.push_str(",\"decoded_byte_length\":");
    output.push_str(&jpeg.decoded_byte_length().to_string());
    output.push_str(",\"filter\":\"DCTDecode\",\"height\":");
    output.push_str(&plan.height().get().to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&plan.image_id().get().to_string());
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, jpeg.limits_fingerprint());
    output.push_str(",\"normalized_byte_length\":");
    output.push_str(&normalized_byte_length.to_string());
    output.push_str(",\"normalized_sha256\":");
    push_hash(&mut output, jpeg.normalized_sha256());
    output.push_str(",\"peak_workspace_bytes\":");
    output.push_str(&jpeg.peak_workspace_bytes().to_string());
    output.push_str(",\"pixel_sha256\":");
    push_hash(&mut output, jpeg.pixel_sha256());
    output.push_str(",\"plan_id\":");
    push_jcs_string(&mut output, JPEG_PDF_PLAN_ID);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, jpeg.profile_fingerprint());
    output.push_str(",\"sampling\":");
    push_jcs_string(&mut output, jpeg.sampling().as_str());
    output.push_str(",\"source_sha256\":");
    push_hash(&mut output, jpeg.source_sha256());
    output.push_str(",\"width\":");
    output.push_str(&plan.width().get().to_string());
    output.push('}');
    sha256(output.as_bytes())
}

#[allow(clippy::too_many_arguments)]
fn encode_pdf_facts(
    display_fingerprint: [u8; 32],
    selected_state: LayoutStateFingerprint,
    admitted_fingerprint: AdmittedResourceFingerprint,
    limits_fingerprint: [u8; 32],
    config_fingerprint: EffectiveConfigFingerprint,
    page_count: u32,
    object_count: u32,
    resources: &[StagingJpegPdfImageObject],
    pdf_sha256: [u8; 32],
    pdf_byte_length: u64,
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted_fingerprint.bytes());
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_JPEG_PDF_CLOSURE_ALGORITHM);
    output.push_str(",\"config_fingerprint\":");
    push_hash(&mut output, config_fingerprint.bytes());
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display_fingerprint);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    output.push_str(",\"object_count\":");
    output.push_str(&object_count.to_string());
    output.push_str(",\"page_count\":");
    output.push_str(&page_count.to_string());
    output.push_str(",\"pdf_byte_length\":");
    output.push_str(&pdf_byte_length.to_string());
    output.push_str(",\"pdf_sha256\":");
    push_hash(&mut output, pdf_sha256);
    output.push_str(",\"resources\":[");
    for (index, resource) in resources.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"color\":");
        push_jcs_string(&mut output, resource.color_kind.as_str());
        output.push_str(",\"color_transform\":");
        output.push_str(&resource.color_transform.to_string());
        output.push_str(",\"decoded_byte_length\":");
        output.push_str(&resource.decoded_byte_length.to_string());
        output.push_str(",\"height\":");
        output.push_str(&resource.height.to_string());
        output.push_str(",\"image_id\":");
        output.push_str(&resource.image_id.get().to_string());
        output.push_str(",\"normalized_byte_length\":");
        output.push_str(&resource.normalized_byte_length.to_string());
        output.push_str(",\"normalized_sha256\":");
        push_hash(&mut output, resource.normalized_sha256);
        output.push_str(",\"object_number\":");
        output.push_str(&resource.object_number.to_string());
        output.push_str(",\"pixel_sha256\":");
        push_hash(&mut output, resource.pixel_sha256);
        output.push_str(",\"placement_count\":");
        output.push_str(&resource.placement_count.to_string());
        output.push_str(",\"plan_fingerprint\":");
        push_hash(&mut output, resource.plan_fingerprint);
        output.push_str(",\"plan_id\":");
        push_jcs_string(&mut output, JPEG_PDF_PLAN_ID);
        output.push_str(",\"profile_fingerprint\":");
        push_hash(&mut output, resource.profile_fingerprint);
        output.push_str(",\"resource_name\":");
        push_jcs_string(&mut output, &resource.resource_name);
        output.push_str(",\"sampling\":");
        push_jcs_string(&mut output, resource.sampling.as_str());
        output.push_str(",\"source_sha256\":");
        push_hash(&mut output, resource.source_sha256);
        output.push_str(",\"width\":");
        output.push_str(&resource.width.to_string());
        output.push('}');
    }
    output.push_str("],\"selected_state_fingerprint\":");
    push_hash(&mut output, selected_state.bytes());
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

    #[test]
    fn serialized_jpeg_closure_does_not_count_dictionary_text_inside_payload() {
        let payload = b"\xff\xd8/Filter /DCTDecode/DecodeParms << /ColorTransform 0 >>\xff\xd9";
        let resource = StagingJpegPdfImageObject {
            image_id: ImageResourceId::new(0),
            resource_name: "/Im0".to_owned(),
            object_number: 7,
            placement_count: 1,
            source_sha256: [1; 32],
            normalized_byte_length: u64::try_from(payload.len()).unwrap(),
            normalized_sha256: sha256(payload),
            pixel_sha256: [2; 32],
            width: 2,
            height: 1,
            decoded_byte_length: 6,
            peak_workspace_bytes: 128,
            color_kind: JpegColorKind::YCbCr,
            sampling: JpegSampling::YCbCr444,
            color_transform: 1,
            limits_fingerprint: [3; 32],
            profile_fingerprint: [4; 32],
            plan_fingerprint: [5; 32],
        };
        let prefix = format!(
            "7 0 obj\n<< /BitsPerComponent 8 /ColorSpace /DeviceRGB /DecodeParms << /ColorTransform 1 >> /Filter /DCTDecode /Height 1 /Length {} /Subtype /Image /Type /XObject /Width 2 >>\nstream\n",
            payload.len()
        );
        let pdf = [
            b"%PDF-1.7\n".as_slice(),
            prefix.as_bytes(),
            payload,
            b"\nendstream\nendobj\n".as_slice(),
        ]
        .concat();
        assert_eq!(count_jpeg_stream_object(&pdf, payload, &resource), 1);
    }
}
