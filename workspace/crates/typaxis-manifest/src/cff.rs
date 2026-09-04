use typaxis_core::{
    push_jcs_string, sha256, FontFaceId, FontInstanceId, M4EffectiveResourceLimits,
};
use typaxis_document::{FontMediaDeclaration, FontMediaType};
use typaxis_machine_profile::StagingCffProfileReceipt;
use typaxis_pdf::{StagingCff1PdfFontObject, StagingCff1PdfObservation, VerifiedPdfBytesReceipt};
use typaxis_resource_admission::{
    close_staging_declared_media, AdmittedFontMediaKind, AdmittedResourceLedger,
    StagingDeclaredMediaLedger,
};
use typaxis_resources::{FrozenPdfResourcePlans, PdfFontProgramKind};
use typaxis_syntax::ValidatedStagingSemanticPackage;

pub const STAGING_CFF1_MANIFEST_ALGORITHM: &str = "typaxis.cff1-manifest/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingCff1ManifestError {
    ProfileMismatch,
    AdmissionClosure,
    MediaMismatch(FontFaceId),
    InstanceMismatch(FontInstanceId),
    PdfMismatch,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingCff1ManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for StagingCff1ManifestError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingCff1ManifestInstance {
    font_instance_id: FontInstanceId,
    selected_source_gids: Vec<u16>,
    dense_source_to_subset: Vec<[u16; 2]>,
    glyph_closure_fingerprint: [u8; 32],
    subset_byte_length: u64,
    subset_sha256: [u8; 32],
    subset_postscript_name: String,
    subset_fingerprint: [u8; 32],
    pdf_plan_fingerprint: [u8; 32],
    cid_count: u32,
    to_unicode_sha256: [u8; 32],
    cid_set_sha256: [u8; 32],
    pdf_resource_name: String,
    pdf_object_numbers: [u32; 6],
    pdf_object_fingerprint: [u8; 32],
}

impl StagingCff1ManifestInstance {
    pub const fn font_instance_id(&self) -> FontInstanceId {
        self.font_instance_id
    }
    pub fn selected_source_gids(&self) -> &[u16] {
        &self.selected_source_gids
    }
    pub fn dense_source_to_subset(&self) -> &[[u16; 2]] {
        &self.dense_source_to_subset
    }
    pub const fn glyph_closure_fingerprint(&self) -> [u8; 32] {
        self.glyph_closure_fingerprint
    }
    pub const fn subset_byte_length(&self) -> u64 {
        self.subset_byte_length
    }
    pub const fn subset_sha256(&self) -> [u8; 32] {
        self.subset_sha256
    }
    pub fn subset_postscript_name(&self) -> &str {
        &self.subset_postscript_name
    }
    pub const fn subset_fingerprint(&self) -> [u8; 32] {
        self.subset_fingerprint
    }
    pub const fn pdf_plan_fingerprint(&self) -> [u8; 32] {
        self.pdf_plan_fingerprint
    }
    pub const fn cid_count(&self) -> u32 {
        self.cid_count
    }
    pub const fn to_unicode_sha256(&self) -> [u8; 32] {
        self.to_unicode_sha256
    }
    pub const fn cid_set_sha256(&self) -> [u8; 32] {
        self.cid_set_sha256
    }
    pub fn pdf_resource_name(&self) -> &str {
        &self.pdf_resource_name
    }
    pub const fn pdf_object_numbers(&self) -> [u32; 6] {
        self.pdf_object_numbers
    }
    pub const fn pdf_object_fingerprint(&self) -> [u8; 32] {
        self.pdf_object_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingCff1ManifestResource {
    font_face_id: FontFaceId,
    uri: String,
    declared_family: String,
    declared_media_type: &'static str,
    attested_media_kind: &'static str,
    face_index: u32,
    source_byte_length: u64,
    source_sha256: [u8; 32],
    table_count: u32,
    glyph_count: u32,
    subroutine_count: u32,
    source_family: String,
    source_subfamily: String,
    source_postscript_name: String,
    fs_type: u16,
    embedding_permission: &'static str,
    admission_fingerprint: [u8; 32],
    instances: Vec<StagingCff1ManifestInstance>,
}

impl StagingCff1ManifestResource {
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub fn declared_family(&self) -> &str {
        &self.declared_family
    }
    pub const fn declared_media_type(&self) -> &'static str {
        self.declared_media_type
    }
    pub const fn attested_media_kind(&self) -> &'static str {
        self.attested_media_kind
    }
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }
    pub const fn source_byte_length(&self) -> u64 {
        self.source_byte_length
    }
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub const fn table_count(&self) -> u32 {
        self.table_count
    }
    pub const fn glyph_count(&self) -> u32 {
        self.glyph_count
    }
    pub const fn subroutine_count(&self) -> u32 {
        self.subroutine_count
    }
    pub fn source_family(&self) -> &str {
        &self.source_family
    }
    pub fn source_subfamily(&self) -> &str {
        &self.source_subfamily
    }
    pub fn source_postscript_name(&self) -> &str {
        &self.source_postscript_name
    }
    pub const fn fs_type(&self) -> u16 {
        self.fs_type
    }
    pub const fn embedding_permission(&self) -> &'static str {
        self.embedding_permission
    }
    pub const fn admission_fingerprint(&self) -> [u8; 32] {
        self.admission_fingerprint
    }
    pub fn instances(&self) -> &[StagingCff1ManifestInstance] {
        &self.instances
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingCff1Manifest {
    resources: Vec<StagingCff1ManifestResource>,
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    authorization_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    declared_media_fingerprint: [u8; 32],
    pdf_observation_fingerprint: [u8; 32],
    pdf_sha256: [u8; 32],
    page_count: u32,
    object_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingCff1Manifest {
    pub fn resources(&self) -> &[StagingCff1ManifestResource] {
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
    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingCffProfileReceipt,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        media: &StagingDeclaredMediaLedger,
        plans: &FrozenPdfResourcePlans,
        pdf_observation: &StagingCff1PdfObservation,
        pdf: &VerifiedPdfBytesReceipt,
    ) -> Result<(), StagingCff1ManifestError> {
        let expected = assemble(
            package,
            profile,
            limits,
            admitted,
            media,
            plans,
            pdf_observation,
            pdf,
        )?;
        if self == &expected {
            Ok(())
        } else {
            Err(StagingCff1ManifestError::ReceiptMismatch)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_staging_cff1_manifest(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingCffProfileReceipt,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    media: &StagingDeclaredMediaLedger,
    plans: &FrozenPdfResourcePlans,
    pdf_observation: &StagingCff1PdfObservation,
    pdf: &VerifiedPdfBytesReceipt,
) -> Result<StagingCff1Manifest, StagingCff1ManifestError> {
    assemble(
        package,
        profile,
        limits,
        admitted,
        media,
        plans,
        pdf_observation,
        pdf,
    )
}

#[allow(clippy::too_many_arguments)]
fn assemble(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingCffProfileReceipt,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    media: &StagingDeclaredMediaLedger,
    plans: &FrozenPdfResourcePlans,
    pdf_observation: &StagingCff1PdfObservation,
    pdf: &VerifiedPdfBytesReceipt,
) -> Result<StagingCff1Manifest, StagingCff1ManifestError> {
    profile
        .verify(package, limits)
        .map_err(|_| StagingCff1ManifestError::ProfileMismatch)?;
    let expected_media = close_staging_declared_media(admitted, package.resources())
        .map_err(|_| StagingCff1ManifestError::AdmissionClosure)?;
    if &expected_media != media {
        return Err(StagingCff1ManifestError::AdmissionClosure);
    }
    if pdf_observation.selected_layout_fingerprint() != pdf.selected_layout_fingerprint()
        || pdf_observation.object_count() != pdf.object_count()
    {
        return Err(StagingCff1ManifestError::PdfMismatch);
    }

    let mut resources = Vec::new();
    resources
        .try_reserve_exact(package.resources().font_faces.len())
        .map_err(|_| StagingCff1ManifestError::AllocationFailure)?;
    let mut observed_instances = 0usize;
    for declaration in &package.resources().font_faces {
        if declaration.media != FontMediaDeclaration::Declared(FontMediaType::SfntCff1)
            || declaration.face_index != 0
        {
            return Err(StagingCff1ManifestError::MediaMismatch(
                declaration.font_face_id,
            ));
        }
        let admitted_font = admitted.font(declaration.font_face_id).ok_or(
            StagingCff1ManifestError::MediaMismatch(declaration.font_face_id),
        )?;
        let declared = media
            .fonts()
            .iter()
            .find(|font| font.font_face_id() == declaration.font_face_id)
            .ok_or(StagingCff1ManifestError::MediaMismatch(
                declaration.font_face_id,
            ))?;
        let admission =
            declared
                .cff1_admission()
                .ok_or(StagingCff1ManifestError::MediaMismatch(
                    declaration.font_face_id,
                ))?;
        if admitted_font.media_kind() != AdmittedFontMediaKind::SfntCff1
            || admitted_font.cff1_admission() != Some(admission)
            || declared.declared() != FontMediaType::SfntCff1
            || declared.attested() != AdmittedFontMediaKind::SfntCff1
            || declared.content_hash() != admitted_font.content_hash()
            || admission.source_sha256() != admitted_font.content_hash()
            || admission.source_byte_length() != admitted_font.byte_length()
            || admission.limits_fingerprint() != limits.fingerprint()
            || declared.m4_profile_fingerprint()
                != Some(profile.authorization().profile_fingerprint())
            || admitted_font.m4_profile_fingerprint()
                != Some(profile.authorization().profile_fingerprint())
        {
            return Err(StagingCff1ManifestError::MediaMismatch(
                declaration.font_face_id,
            ));
        }

        let mut instances = Vec::new();
        for plan in plans.fonts().iter().filter(|plan| {
            plan.cff1_plan()
                .is_some_and(|cff| cff.font_face_id() == declaration.font_face_id)
        }) {
            if plan.program_kind() != PdfFontProgramKind::OpenTypeCff1
                || plan.admitted_sha256() != admitted_font.content_hash()
            {
                return Err(StagingCff1ManifestError::InstanceMismatch(
                    plan.font_instance_id(),
                ));
            }
            let cff = plan
                .cff1_plan()
                .ok_or(StagingCff1ManifestError::InstanceMismatch(
                    plan.font_instance_id(),
                ))?;
            let observation = pdf_observation
                .fonts()
                .iter()
                .find(|observed| {
                    observed.font_face_id() == declaration.font_face_id
                        && observed.font_instance_id() == plan.font_instance_id()
                })
                .ok_or(StagingCff1ManifestError::InstanceMismatch(
                    plan.font_instance_id(),
                ))?;
            validate_pdf_instance(plan, cff, observation)?;
            let dense_source_to_subset = plan
                .subset_plan()
                .glyphs
                .iter()
                .map(|binding| [binding.original_gid.get(), binding.subset_gid.get()])
                .collect::<Vec<_>>();
            instances.push(StagingCff1ManifestInstance {
                font_instance_id: plan.font_instance_id(),
                selected_source_gids: cff
                    .selected_source_gids()
                    .iter()
                    .map(|gid| gid.get())
                    .collect(),
                dense_source_to_subset,
                glyph_closure_fingerprint: cff.glyph_closure_fingerprint(),
                subset_byte_length: observation.subset_byte_length(),
                subset_sha256: cff.subset_sha256(),
                subset_postscript_name: plan.embedded_postscript_name().to_owned(),
                subset_fingerprint: cff.subset_fingerprint(),
                pdf_plan_fingerprint: cff.fingerprint(),
                cid_count: observation.cid_count(),
                to_unicode_sha256: observation.to_unicode_sha256(),
                cid_set_sha256: observation.cid_set_sha256(),
                pdf_resource_name: observation.resource_name().to_owned(),
                pdf_object_numbers: observation.object_numbers(),
                pdf_object_fingerprint: observation.fingerprint(),
            });
            observed_instances = observed_instances
                .checked_add(1)
                .ok_or(StagingCff1ManifestError::AllocationFailure)?;
        }
        instances.sort_by_key(StagingCff1ManifestInstance::font_instance_id);
        resources.push(StagingCff1ManifestResource {
            font_face_id: declaration.font_face_id,
            uri: declaration.uri.as_str().to_owned(),
            declared_family: declaration.family.clone(),
            declared_media_type: FontMediaType::SfntCff1.as_str(),
            attested_media_kind: AdmittedFontMediaKind::SfntCff1.as_str(),
            face_index: declaration.face_index,
            source_byte_length: admission.source_byte_length(),
            source_sha256: admission.source_sha256(),
            table_count: admission.table_count(),
            glyph_count: admission.glyph_count(),
            subroutine_count: admission.subroutine_count(),
            source_family: admission.family().to_owned(),
            source_subfamily: admission.subfamily().to_owned(),
            source_postscript_name: admission.postscript_name().to_owned(),
            fs_type: admission.fs_type(),
            embedding_permission: admission.embedding_permission().as_str(),
            admission_fingerprint: admission.fingerprint(),
            instances,
        });
    }
    if observed_instances != pdf_observation.fonts().len()
        || plans
            .fonts()
            .iter()
            .any(|plan| plan.program_kind() != PdfFontProgramKind::OpenTypeCff1)
        || !plans.images().is_empty()
    {
        return Err(StagingCff1ManifestError::PdfMismatch);
    }
    resources.sort_by_key(StagingCff1ManifestResource::font_face_id);
    let canonical_jcs = encode_manifest(
        package.semantic_fingerprint(),
        profile.fingerprint(),
        profile.authorization().profile_fingerprint(),
        limits.fingerprint(),
        admitted.fingerprint().bytes(),
        media.fingerprint(),
        pdf_observation.fingerprint(),
        pdf.content_hash(),
        pdf.page_count(),
        pdf.object_count(),
        profile,
        &resources,
    );
    Ok(StagingCff1Manifest {
        resources,
        package_fingerprint: package.semantic_fingerprint(),
        profile_fingerprint: profile.fingerprint(),
        authorization_fingerprint: profile.authorization().profile_fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        admitted_fingerprint: admitted.fingerprint().bytes(),
        declared_media_fingerprint: media.fingerprint(),
        pdf_observation_fingerprint: pdf_observation.fingerprint(),
        pdf_sha256: pdf.content_hash(),
        page_count: pdf.page_count(),
        object_count: pdf.object_count(),
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn validate_pdf_instance(
    plan: &typaxis_resources::FrozenPdfFontPlan,
    cff: &typaxis_resources::FrozenPdfCff1Plan,
    observation: &StagingCff1PdfFontObject,
) -> Result<(), StagingCff1ManifestError> {
    let instance_id = plan.font_instance_id();
    let glyph_count = u32::try_from(plan.subset_plan().glyphs.len())
        .map_err(|_| StagingCff1ManifestError::InstanceMismatch(instance_id))?;
    if observation.subset_sha256() != cff.subset_sha256()
        || observation.subset_sha256() != sha256(plan.subset_bytes())
        || observation.pdf_plan_fingerprint() != cff.fingerprint()
        || observation.cid_count() != glyph_count
        || cff.selected_source_gids().len() != plan.subset_plan().glyphs.len()
        || plan
            .subset_plan()
            .glyphs
            .iter()
            .enumerate()
            .any(|(index, binding)| {
                usize::from(binding.subset_gid.get()) != index
                    || cff.selected_source_gids().get(index).copied() != Some(binding.original_gid)
            })
    {
        return Err(StagingCff1ManifestError::InstanceMismatch(instance_id));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn encode_manifest(
    package: [u8; 32],
    profile: [u8; 32],
    authorization: [u8; 32],
    limits: [u8; 32],
    admitted: [u8; 32],
    media: [u8; 32],
    pdf_observation: [u8; 32],
    pdf_sha256: [u8; 32],
    page_count: u32,
    object_count: u32,
    profile_receipt: &StagingCffProfileReceipt,
    resources: &[StagingCff1ManifestResource],
) -> String {
    let descriptor = profile_receipt.descriptor();
    let mut output = String::from("{\"admission_id\":");
    push_jcs_string(&mut output, descriptor.admission());
    output.push_str(",\"admitted_fingerprint\":");
    push_hash(&mut output, admitted);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_CFF1_MANIFEST_ALGORITHM);
    output.push_str(",\"authorization_fingerprint\":");
    push_hash(&mut output, authorization);
    output.push_str(",\"charstring_evaluator_id\":");
    push_jcs_string(&mut output, descriptor.evaluator());
    output.push_str(",\"declared_media_fingerprint\":");
    push_hash(&mut output, media);
    output.push_str(",\"embedding_permission_id\":");
    push_jcs_string(&mut output, descriptor.embedding_permission());
    output.push_str(",\"glyph_closure_id\":");
    push_jcs_string(&mut output, descriptor.glyph_closure());
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits);
    output.push_str(",\"object_count\":");
    output.push_str(&object_count.to_string());
    output.push_str(",\"package_fingerprint\":");
    push_hash(&mut output, package);
    output.push_str(",\"page_count\":");
    output.push_str(&page_count.to_string());
    output.push_str(",\"pdf_observation_fingerprint\":");
    push_hash(&mut output, pdf_observation);
    output.push_str(",\"pdf_plan_id\":");
    push_jcs_string(&mut output, descriptor.pdf_plan());
    output.push_str(",\"pdf_sha256\":");
    push_hash(&mut output, pdf_sha256);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, profile);
    output.push_str(",\"resource_profile_id\":");
    push_jcs_string(&mut output, descriptor.resource_profile_id());
    output.push_str(",\"resources\":[");
    for (index, resource) in resources.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_resource(&mut output, resource);
    }
    output.push_str("],\"subsetter_id\":");
    push_jcs_string(&mut output, descriptor.subsetter());
    output.push('}');
    output
}

fn encode_resource(output: &mut String, resource: &StagingCff1ManifestResource) {
    output.push_str("{\"admission_fingerprint\":");
    push_hash(output, resource.admission_fingerprint);
    output.push_str(",\"attested_media_kind\":");
    push_jcs_string(output, resource.attested_media_kind);
    output.push_str(",\"declared_family\":");
    push_jcs_string(output, &resource.declared_family);
    output.push_str(",\"declared_media_type\":");
    push_jcs_string(output, resource.declared_media_type);
    output.push_str(",\"embedding_permission\":");
    push_jcs_string(output, resource.embedding_permission);
    output.push_str(",\"face_index\":");
    output.push_str(&resource.face_index.to_string());
    output.push_str(",\"font_face_id\":");
    output.push_str(&resource.font_face_id.get().to_string());
    output.push_str(",\"fs_type\":");
    output.push_str(&resource.fs_type.to_string());
    output.push_str(",\"glyph_count\":");
    output.push_str(&resource.glyph_count.to_string());
    output.push_str(",\"instances\":[");
    for (index, instance) in resource.instances.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_instance(output, instance);
    }
    output.push_str("],\"source_byte_length\":");
    output.push_str(&resource.source_byte_length.to_string());
    output.push_str(",\"source_family\":");
    push_jcs_string(output, &resource.source_family);
    output.push_str(",\"source_postscript_name\":");
    push_jcs_string(output, &resource.source_postscript_name);
    output.push_str(",\"source_sha256\":");
    push_hash(output, resource.source_sha256);
    output.push_str(",\"source_subfamily\":");
    push_jcs_string(output, &resource.source_subfamily);
    output.push_str(",\"subroutine_count\":");
    output.push_str(&resource.subroutine_count.to_string());
    output.push_str(",\"table_count\":");
    output.push_str(&resource.table_count.to_string());
    output.push_str(",\"uri\":");
    push_jcs_string(output, &resource.uri);
    output.push('}');
}

fn encode_instance(output: &mut String, instance: &StagingCff1ManifestInstance) {
    output.push_str("{\"cid_count\":");
    output.push_str(&instance.cid_count.to_string());
    output.push_str(",\"cid_set_sha256\":");
    push_hash(output, instance.cid_set_sha256);
    output.push_str(",\"dense_source_to_subset\":[");
    for (index, pair) in instance.dense_source_to_subset.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push('[');
        output.push_str(&pair[0].to_string());
        output.push(',');
        output.push_str(&pair[1].to_string());
        output.push(']');
    }
    output.push_str("],\"font_instance_id\":");
    output.push_str(&instance.font_instance_id.get().to_string());
    output.push_str(",\"glyph_closure_fingerprint\":");
    push_hash(output, instance.glyph_closure_fingerprint);
    output.push_str(",\"pdf_object_fingerprint\":");
    push_hash(output, instance.pdf_object_fingerprint);
    output.push_str(",\"pdf_object_numbers\":[");
    for (index, object) in instance.pdf_object_numbers.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&object.to_string());
    }
    output.push_str("],\"pdf_plan_fingerprint\":");
    push_hash(output, instance.pdf_plan_fingerprint);
    output.push_str(",\"pdf_resource_name\":");
    push_jcs_string(output, &instance.pdf_resource_name);
    output.push_str(",\"selected_source_gids\":[");
    for (index, gid) in instance.selected_source_gids.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&gid.to_string());
    }
    output.push_str("],\"subset_byte_length\":");
    output.push_str(&instance.subset_byte_length.to_string());
    output.push_str(",\"subset_fingerprint\":");
    push_hash(output, instance.subset_fingerprint);
    output.push_str(",\"subset_postscript_name\":");
    push_jcs_string(output, &instance.subset_postscript_name);
    output.push_str(",\"subset_sha256\":");
    push_hash(output, instance.subset_sha256);
    output.push_str(",\"to_unicode_sha256\":");
    push_hash(output, instance.to_unicode_sha256);
    output.push('}');
}

fn push_hash(output: &mut String, hash: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in hash {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}
