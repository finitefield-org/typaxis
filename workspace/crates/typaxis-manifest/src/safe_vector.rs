use typaxis_core::{
    push_jcs_string, sha256, ImageResourceId, M4EffectiveResourceLimits, NodeId, Rect,
};
use typaxis_display_list::StagingSafeVectorDisplay;
use typaxis_document::{ImageMediaDeclaration, ImageMediaType, StagingM4Block};
use typaxis_layout::StagingSafeVectorSelectedLayout;
use typaxis_machine_profile::StagingSafeVectorProfileReceipt;
use typaxis_pdf::StagingSafeVectorPdf;
use typaxis_resource_admission::{
    close_staging_declared_media, AdmittedImageMediaKind, AdmittedResourceLedger,
    StagingDeclaredMediaLedger, SAFE_SVG_PARSER_ID, SAFE_VECTOR_ALLOCATION_CHARGE_ID,
    SAFE_VECTOR_IR_FINGERPRINT_ID, SAFE_VECTOR_IR_ID,
};
use typaxis_resources::StagingSafeVectorFormPlans;
use typaxis_syntax::ValidatedStagingSemanticPackage;

pub const STAGING_SAFE_VECTOR_MANIFEST_ALGORITHM: &str = "typaxis.safe-vector-manifest/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorManifestUsage {
    occurrence: u32,
    owner: NodeId,
    page_index: u32,
    bounds: Rect,
    scale: i32,
    alternative_sha256: [u8; 32],
    selected_placement_fingerprint: [u8; 32],
    display_command_fingerprint: [u8; 32],
    pdf_page_object_number: u32,
    pdf_content_object_number: u32,
}

impl StagingSafeVectorManifestUsage {
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
    pub const fn scale_raw(&self) -> i32 {
        self.scale
    }
    pub const fn alternative_sha256(&self) -> [u8; 32] {
        self.alternative_sha256
    }
    pub const fn selected_placement_fingerprint(&self) -> [u8; 32] {
        self.selected_placement_fingerprint
    }
    pub const fn display_command_fingerprint(&self) -> [u8; 32] {
        self.display_command_fingerprint
    }
    pub const fn pdf_page_object_number(&self) -> u32 {
        self.pdf_page_object_number
    }
    pub const fn pdf_content_object_number(&self) -> u32 {
        self.pdf_content_object_number
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorManifestResource {
    image_id: ImageResourceId,
    uri: String,
    declared_media_type: &'static str,
    attested_media_kind: &'static str,
    admitted_sha256: [u8; 32],
    ir_fingerprint: [u8; 32],
    allocation_charge: u64,
    intrinsic_width: i64,
    intrinsic_height: i64,
    usages: Vec<StagingSafeVectorManifestUsage>,
    form_plan_fingerprint: Option<[u8; 32]>,
    pdf_form_object_number: Option<u32>,
    pdf_resource_name: Option<String>,
}

impl StagingSafeVectorManifestResource {
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
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn ir_fingerprint(&self) -> [u8; 32] {
        self.ir_fingerprint
    }
    pub const fn allocation_charge(&self) -> u64 {
        self.allocation_charge
    }
    pub const fn intrinsic_width(&self) -> i64 {
        self.intrinsic_width
    }
    pub const fn intrinsic_height(&self) -> i64 {
        self.intrinsic_height
    }
    pub fn usages(&self) -> &[StagingSafeVectorManifestUsage] {
        &self.usages
    }
    pub const fn form_plan_fingerprint(&self) -> Option<[u8; 32]> {
        self.form_plan_fingerprint
    }
    pub const fn pdf_form_object_number(&self) -> Option<u32> {
        self.pdf_form_object_number
    }
    pub fn pdf_resource_name(&self) -> Option<&str> {
        self.pdf_resource_name.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorManifest {
    resources: Vec<StagingSafeVectorManifestResource>,
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    declared_media_fingerprint: [u8; 32],
    selected_layout_fingerprint: [u8; 32],
    display_fingerprint: [u8; 32],
    form_plans_fingerprint: [u8; 32],
    pdf_fingerprint: [u8; 32],
    pdf_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorManifest {
    pub fn resources(&self) -> &[StagingSafeVectorManifestResource] {
        &self.resources
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
        profile: &StagingSafeVectorProfileReceipt,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        media: &StagingDeclaredMediaLedger,
        selected: &StagingSafeVectorSelectedLayout,
        display: &StagingSafeVectorDisplay,
        plans: &StagingSafeVectorFormPlans,
        pdf: &StagingSafeVectorPdf,
    ) -> Result<(), StagingSafeVectorManifestError> {
        let expected = assemble(
            package, profile, limits, admitted, media, selected, display, plans, pdf,
        )?;
        if self != &expected {
            return Err(StagingSafeVectorManifestError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorManifestError {
    ProfileMismatch,
    AdmissionMismatch(ImageResourceId),
    MediaMismatch(ImageResourceId),
    LayoutMismatch,
    DisplayMismatch,
    PlanMismatch,
    PdfMismatch,
    ReceiptMismatch,
    PrecomposedVectorStaging(NodeId),
    SvgSafe2Staging(ImageResourceId),
}

impl std::fmt::Display for StagingSafeVectorManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => formatter.write_str("I9190: SafeVector profile mismatch"),
            Self::AdmissionMismatch(id) => write!(
                formatter,
                "I9190: SafeVector admission mismatch for image {}",
                id.get()
            ),
            Self::MediaMismatch(id) => write!(
                formatter,
                "I9190: SafeVector declared-media mismatch for image {}",
                id.get()
            ),
            Self::LayoutMismatch => {
                formatter.write_str("I9190: SafeVector selected layout mismatch")
            }
            Self::DisplayMismatch => formatter.write_str("I9190: SafeVector Display mismatch"),
            Self::PlanMismatch => formatter.write_str("I9190: SafeVector Form plan mismatch"),
            Self::PdfMismatch => formatter.write_str("I9190: SafeVector PDF mismatch"),
            Self::ReceiptMismatch => formatter.write_str("I9190: SafeVector manifest mismatch"),
            Self::PrecomposedVectorStaging(owner) => write!(
                formatter,
                "P1102: precomposed vector at node {} requires SafeVector manifest /2",
                owner.get()
            ),
            Self::SvgSafe2Staging(id) => write!(
                formatter,
                "P1102: svg-safe-2 image {} requires SafeVector manifest /2",
                id.get()
            ),
        }
    }
}

impl std::error::Error for StagingSafeVectorManifestError {}

#[allow(clippy::too_many_arguments)]
pub fn build_staging_safe_vector_manifest(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSafeVectorProfileReceipt,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    media: &StagingDeclaredMediaLedger,
    selected: &StagingSafeVectorSelectedLayout,
    display: &StagingSafeVectorDisplay,
    plans: &StagingSafeVectorFormPlans,
    pdf: &StagingSafeVectorPdf,
) -> Result<StagingSafeVectorManifest, StagingSafeVectorManifestError> {
    assemble(
        package, profile, limits, admitted, media, selected, display, plans, pdf,
    )
}

#[allow(clippy::too_many_arguments)]
fn assemble(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSafeVectorProfileReceipt,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    media: &StagingDeclaredMediaLedger,
    selected: &StagingSafeVectorSelectedLayout,
    display: &StagingSafeVectorDisplay,
    plans: &StagingSafeVectorFormPlans,
    pdf: &StagingSafeVectorPdf,
) -> Result<StagingSafeVectorManifest, StagingSafeVectorManifestError> {
    if let Some(owner) = first_precomposed_vector_owner(&package.document().blocks).or_else(|| {
        package
            .document()
            .footnotes
            .iter()
            .find_map(|footnote| first_precomposed_vector_owner(&footnote.blocks))
    }) {
        return Err(StagingSafeVectorManifestError::PrecomposedVectorStaging(
            owner,
        ));
    }
    if let Some(image) = package
        .resources()
        .images
        .iter()
        .find(|image| image.media == ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2))
    {
        return Err(StagingSafeVectorManifestError::SvgSafe2Staging(
            image.image_id,
        ));
    }
    profile
        .authorizes(package, limits)
        .map_err(|_| StagingSafeVectorManifestError::ProfileMismatch)?;
    selected
        .verify(package, profile.authorization(), limits, admitted)
        .map_err(|_| StagingSafeVectorManifestError::LayoutMismatch)?;
    display
        .verify(package, profile.authorization(), limits, selected)
        .map_err(|_| StagingSafeVectorManifestError::DisplayMismatch)?;
    plans
        .verify(display, admitted, limits)
        .map_err(|_| StagingSafeVectorManifestError::PlanMismatch)?;
    pdf.verify(display, plans, limits)
        .map_err(|_| StagingSafeVectorManifestError::PdfMismatch)?;
    if close_staging_declared_media(admitted, package.resources())
        .map_or(true, |expected| &expected != media)
    {
        return Err(StagingSafeVectorManifestError::ReceiptMismatch);
    }

    let mut resources = Vec::new();
    for declaration in &package.resources().images {
        match declaration.media {
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1) => {}
            ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2) => {
                return Err(StagingSafeVectorManifestError::SvgSafe2Staging(
                    declaration.image_id,
                ));
            }
            ImageMediaDeclaration::Declared(ImageMediaType::Png)
            | ImageMediaDeclaration::LegacyUnspecified => continue,
        }
        let image = admitted.image(declaration.image_id).ok_or(
            StagingSafeVectorManifestError::AdmissionMismatch(declaration.image_id),
        )?;
        let ir = image
            .safe_vector()
            .filter(|_| image.media_kind() == AdmittedImageMediaKind::SafeVector)
            .ok_or(StagingSafeVectorManifestError::AdmissionMismatch(
                declaration.image_id,
            ))?;
        let attestation = media
            .images()
            .iter()
            .find(|attestation| attestation.image_id() == declaration.image_id)
            .ok_or(StagingSafeVectorManifestError::MediaMismatch(
                declaration.image_id,
            ))?;
        if attestation.declared() != ImageMediaType::SvgSafe1
            || attestation.attested() != AdmittedImageMediaKind::SafeVector
            || attestation.content_hash() != image.content_hash()
            || attestation.safe_vector_ir_fingerprint() != Some(ir.fingerprint())
            || attestation.safe_vector_allocation_charge() != Some(ir.allocation_charge())
            || attestation.m4_limits_fingerprint() != Some(limits.fingerprint())
            || attestation.m4_profile_fingerprint()
                != Some(profile.authorization().profile_fingerprint())
        {
            return Err(StagingSafeVectorManifestError::MediaMismatch(
                declaration.image_id,
            ));
        }
        let plan = plans.plan(declaration.image_id);
        let form = pdf
            .forms()
            .iter()
            .find(|form| form.image_id() == declaration.image_id);
        if plan.is_some() != form.is_some() {
            return Err(StagingSafeVectorManifestError::PdfMismatch);
        }
        let mut usages = Vec::new();
        for placement in selected
            .placements()
            .iter()
            .filter(|placement| placement.image_id() == declaration.image_id)
        {
            let command = display
                .commands()
                .find(|command| command.occurrence() == placement.occurrence())
                .ok_or(StagingSafeVectorManifestError::DisplayMismatch)?;
            let pdf_usage = pdf
                .usages()
                .iter()
                .find(|usage| usage.occurrence() == placement.occurrence())
                .ok_or(StagingSafeVectorManifestError::PdfMismatch)?;
            usages.push(StagingSafeVectorManifestUsage {
                occurrence: placement.occurrence(),
                owner: placement.owner(),
                page_index: placement.page_index(),
                bounds: placement.bounds(),
                scale: placement.scale_raw(),
                alternative_sha256: sha256(placement.alternative().as_bytes()),
                selected_placement_fingerprint: placement.fingerprint(),
                display_command_fingerprint: command.fingerprint(),
                pdf_page_object_number: pdf_usage.page_object_number(),
                pdf_content_object_number: pdf_usage.content_object_number(),
            });
        }
        if usages.is_empty() && (plan.is_some() || form.is_some()) {
            return Err(StagingSafeVectorManifestError::PlanMismatch);
        }
        resources.push(StagingSafeVectorManifestResource {
            image_id: declaration.image_id,
            uri: declaration.uri.as_str().to_owned(),
            declared_media_type: declaration.media.declared_str(),
            attested_media_kind: image.media_kind().as_str(),
            admitted_sha256: image.content_hash(),
            ir_fingerprint: ir.fingerprint(),
            allocation_charge: ir.allocation_charge(),
            intrinsic_width: ir.intrinsic_width().get().raw(),
            intrinsic_height: ir.intrinsic_height().get().raw(),
            usages,
            form_plan_fingerprint: plan.map(|plan| plan.fingerprint()),
            pdf_form_object_number: form.map(|form| form.object_number()),
            pdf_resource_name: form.map(|form| form.resource_name().to_owned()),
        });
    }
    let canonical_jcs = encode_manifest(
        package.semantic_fingerprint(),
        profile.fingerprint(),
        limits.fingerprint(),
        admitted.fingerprint().bytes(),
        media.fingerprint(),
        selected.receipt().fingerprint(),
        display.receipt().fingerprint(),
        plans.fingerprint(),
        pdf.receipt().fingerprint(),
        pdf.receipt().pdf_sha256(),
        &resources,
    );
    Ok(StagingSafeVectorManifest {
        resources,
        package_fingerprint: package.semantic_fingerprint(),
        profile_fingerprint: profile.fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        admitted_fingerprint: admitted.fingerprint().bytes(),
        declared_media_fingerprint: media.fingerprint(),
        selected_layout_fingerprint: selected.receipt().fingerprint(),
        display_fingerprint: display.receipt().fingerprint(),
        form_plans_fingerprint: plans.fingerprint(),
        pdf_fingerprint: pdf.receipt().fingerprint(),
        pdf_sha256: pdf.receipt().pdf_sha256(),
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
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

trait DeclaredMediaString {
    fn declared_str(self) -> &'static str;
}

impl DeclaredMediaString for ImageMediaDeclaration {
    fn declared_str(self) -> &'static str {
        match self {
            ImageMediaDeclaration::Declared(media) => media.as_str(),
            ImageMediaDeclaration::LegacyUnspecified => "legacy-unspecified",
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_manifest(
    package: [u8; 32],
    profile: [u8; 32],
    limits: [u8; 32],
    admitted: [u8; 32],
    media: [u8; 32],
    selected: [u8; 32],
    display: [u8; 32],
    plans: [u8; 32],
    pdf: [u8; 32],
    pdf_sha256: [u8; 32],
    resources: &[StagingSafeVectorManifestResource],
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_SAFE_VECTOR_MANIFEST_ALGORITHM);
    output.push_str(",\"declared_media_fingerprint\":");
    push_hash(&mut output, media);
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display);
    output.push_str(",\"form_plans_fingerprint\":");
    push_hash(&mut output, plans);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits);
    output.push_str(",\"package_fingerprint\":");
    push_hash(&mut output, package);
    output.push_str(",\"pdf_fingerprint\":");
    push_hash(&mut output, pdf);
    output.push_str(",\"pdf_sha256\":");
    push_hash(&mut output, pdf_sha256);
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
    push_hash(&mut output, selected);
    output.push('}');
    output
}

fn encode_resource(output: &mut String, resource: &StagingSafeVectorManifestResource) {
    output.push_str("{\"admitted_sha256\":");
    push_hash(output, resource.admitted_sha256);
    output.push_str(",\"allocation_charge\":");
    output.push_str(&resource.allocation_charge.to_string());
    output.push_str(",\"allocation_charge_algorithm\":");
    push_jcs_string(output, SAFE_VECTOR_ALLOCATION_CHARGE_ID);
    output.push_str(",\"attested_media_kind\":");
    push_jcs_string(output, resource.attested_media_kind);
    output.push_str(",\"declared_media_type\":");
    push_jcs_string(output, resource.declared_media_type);
    output.push_str(",\"form_plan_fingerprint\":");
    push_optional_hash(output, resource.form_plan_fingerprint);
    output.push_str(",\"image_id\":");
    output.push_str(&resource.image_id.get().to_string());
    output.push_str(",\"intrinsic_height\":");
    output.push_str(&resource.intrinsic_height.to_string());
    output.push_str(",\"intrinsic_width\":");
    output.push_str(&resource.intrinsic_width.to_string());
    output.push_str(",\"ir_algorithm\":");
    push_jcs_string(output, SAFE_VECTOR_IR_ID);
    output.push_str(",\"ir_fingerprint\":");
    push_hash(output, resource.ir_fingerprint);
    output.push_str(",\"ir_fingerprint_algorithm\":");
    push_jcs_string(output, SAFE_VECTOR_IR_FINGERPRINT_ID);
    output.push_str(",\"pdf_form_object_number\":");
    push_optional_u32(output, resource.pdf_form_object_number);
    output.push_str(",\"pdf_resource_name\":");
    if let Some(name) = &resource.pdf_resource_name {
        push_jcs_string(output, name);
    } else {
        output.push_str("null");
    }
    output.push_str(",\"safe_svg_parser\":");
    push_jcs_string(output, SAFE_SVG_PARSER_ID);
    output.push_str(",\"uri\":");
    push_jcs_string(output, &resource.uri);
    output.push_str(",\"usages\":[");
    for (index, usage) in resource.usages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"alternative_sha256\":");
        push_hash(output, usage.alternative_sha256);
        output.push_str(",\"bounds\":{");
        output.push_str("\"height\":");
        output.push_str(&usage.bounds.height().get().raw().to_string());
        output.push_str(",\"width\":");
        output.push_str(&usage.bounds.width().get().raw().to_string());
        output.push_str(",\"x\":");
        output.push_str(&usage.bounds.x().raw().to_string());
        output.push_str(",\"y\":");
        output.push_str(&usage.bounds.y().raw().to_string());
        output.push('}');
        output.push_str(",\"display_command_fingerprint\":");
        push_hash(output, usage.display_command_fingerprint);
        output.push_str(",\"occurrence\":");
        output.push_str(&usage.occurrence.to_string());
        output.push_str(",\"owner\":");
        output.push_str(&usage.owner.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&usage.page_index.to_string());
        output.push_str(",\"pdf_content_object_number\":");
        output.push_str(&usage.pdf_content_object_number.to_string());
        output.push_str(",\"pdf_page_object_number\":");
        output.push_str(&usage.pdf_page_object_number.to_string());
        output.push_str(",\"scale\":");
        output.push_str(&usage.scale.to_string());
        output.push_str(",\"selected_placement_fingerprint\":");
        push_hash(output, usage.selected_placement_fingerprint);
        output.push('}');
    }
    output.push_str("]}");
}

fn push_optional_hash(output: &mut String, value: Option<[u8; 32]>) {
    if let Some(value) = value {
        push_hash(output, value);
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

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_machine_profile::{
        preflight_staging_safe_vector_profile, StagingSemanticContainerSessionIdentity,
    };
    use typaxis_pdf::write_staging_safe_vector_pdf;

    #[test]
    fn vector_manifest_closes_every_resource_usage_plan_and_pdf_fact() {
        let fixture = typaxis_resources::staging_safe_vector_resource_fixture().unwrap();
        let layout = &fixture.display.layout;
        let profile = preflight_staging_safe_vector_profile(
            &layout.package,
            &layout.limits,
            &StagingSemanticContainerSessionIdentity::fresh(),
        )
        .unwrap();
        let pdf =
            write_staging_safe_vector_pdf(&fixture.display.display, &fixture.plans, &layout.limits)
                .unwrap();
        let manifest = build_staging_safe_vector_manifest(
            &layout.package,
            &profile,
            &layout.limits,
            &layout.admitted,
            &layout.media,
            &layout.selected,
            &fixture.display.display,
            &fixture.plans,
            &pdf,
        )
        .unwrap();
        assert_eq!(manifest.resources().len(), 2);
        let resource = &manifest.resources()[0];
        assert_eq!(resource.image_id(), ImageResourceId::new(0));
        assert_eq!(resource.declared_media_type(), "svg-safe-1");
        assert_eq!(resource.attested_media_kind(), "svg-safe-1");
        assert_eq!(resource.usages().len(), 1);
        assert!(resource.form_plan_fingerprint().is_some());
        assert!(resource.pdf_form_object_number().is_some());
        assert_eq!(resource.pdf_resource_name(), Some("V0"));
        assert_eq!(
            resource.usages()[0].alternative_sha256(),
            sha256(b"Blue vector geometry")
        );
        let unused = &manifest.resources()[1];
        assert_eq!(unused.image_id(), ImageResourceId::new(1));
        assert_eq!(unused.uri(), "unused.vector");
        assert!(unused.usages().is_empty());
        assert_eq!(unused.form_plan_fingerprint(), None);
        assert_eq!(unused.pdf_form_object_number(), None);
        assert_eq!(unused.pdf_resource_name(), None);
        assert_eq!(
            manifest.canonical_jcs(),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../samples/machine-package/staging/production-book-1/vector-media/manifest.json"
            ))
            .trim_end()
        );
        manifest
            .verify(
                &layout.package,
                &profile,
                &layout.limits,
                &layout.admitted,
                &layout.media,
                &layout.selected,
                &fixture.display.display,
                &fixture.plans,
                &pdf,
            )
            .unwrap();

        let mut tampered = manifest.clone();
        tampered.resources[0].pdf_form_object_number = None;
        assert_eq!(
            tampered.verify(
                &layout.package,
                &profile,
                &layout.limits,
                &layout.admitted,
                &layout.media,
                &layout.selected,
                &fixture.display.display,
                &fixture.plans,
                &pdf,
            ),
            Err(StagingSafeVectorManifestError::ReceiptMismatch)
        );
    }
}
