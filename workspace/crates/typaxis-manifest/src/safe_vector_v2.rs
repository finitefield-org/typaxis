use typaxis_core::M4EffectiveResourceLimits;
use typaxis_core::{
    push_jcs_string, sha256, AffineTransform, ImageResourceId, NodeId, Rect, SourceSpan,
};
use typaxis_display_list::{
    StagingCombinedVectorDisplayV2, StagingCombinedVectorKindV2,
    StagingCombinedVectorUsageRelationV2, StagingDrawVectorV2Relation,
};
use typaxis_document::{StagingM4Block, VectorProvenance};
use typaxis_layout::{
    PrecomposedVectorPlacementInput, ValidatedPrecomposedVectorBindings,
    ValidatedPrecomposedVectorReceipt,
};
use typaxis_pdf::{StagingSafeVectorPdfContributionV2, StagingTaggedPdfV2};
use typaxis_resource_admission::{
    close_staging_declared_media, AdmittedResourceLedger, VectorContentKey,
};
use typaxis_resources::{
    StagingSafeVectorFormPlansV2, VectorContentAliasProvenance, VectorContentCandidateRegistry,
};
use typaxis_syntax::{
    PrecomposedVectorKind, StagingPrecomposedVectorProfileAuthorization,
    ValidatedStagingBookNavigationV2, ValidatedStagingSemanticPackage,
};

pub const STAGING_SAFE_VECTOR_MANIFEST_V2_ALGORITHM: &str = "typaxis.safe-vector-manifest/2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingVectorMetricFactV2 {
    advance: i64,
    ascent: i64,
    baseline: i64,
    descent: i64,
    origin_x: i64,
    viewport_width: i64,
    viewport_height: i64,
}

impl StagingVectorMetricFactV2 {
    pub const fn advance_raw(self) -> i64 {
        self.advance
    }
    pub const fn ascent_raw(self) -> i64 {
        self.ascent
    }
    pub const fn baseline_raw(self) -> i64 {
        self.baseline
    }
    pub const fn descent_raw(self) -> i64 {
        self.descent
    }
    pub const fn origin_x_raw(self) -> i64 {
        self.origin_x
    }
    pub const fn viewport_width_raw(self) -> i64 {
        self.viewport_width
    }
    pub const fn viewport_height_raw(self) -> i64 {
        self.viewport_height
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorManifestAliasV2 {
    image_id: ImageResourceId,
    uri: String,
    expected_sha256: Option<[u8; 32]>,
    admitted_sha256: [u8; 32],
    admission_attestation_fingerprint: [u8; 32],
    allocation_charge: u64,
    provenance: Option<VectorProvenance>,
    placement_count: u32,
    usage_fingerprints: Vec<[u8; 32]>,
}

impl StagingSafeVectorManifestAliasV2 {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub const fn expected_sha256(&self) -> Option<[u8; 32]> {
        self.expected_sha256
    }
    pub const fn provenance(&self) -> Option<&VectorProvenance> {
        self.provenance.as_ref()
    }
    pub const fn placement_count(&self) -> u32 {
        self.placement_count
    }
    pub fn usage_fingerprints(&self) -> &[[u8; 32]] {
        &self.usage_fingerprints
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorPlacementDetailsV2 {
    Figure {
        placement: &'static str,
    },
    Inline {
        metrics: StagingVectorMetricFactV2,
        spacing_before: i64,
        spacing_after: i64,
    },
    VectorFigure {
        style_fingerprint: [u8; 32],
        alignment: &'static str,
        space_before: i64,
        space_after: i64,
        start_indent: i64,
        end_indent: i64,
        keep_caption: bool,
        keep_with_next: bool,
    },
    MathVectorBlock {
        metrics: StagingVectorMetricFactV2,
        style_fingerprint: [u8; 32],
        alignment: &'static str,
        space_before: i64,
        space_after: i64,
        start_indent: i64,
        end_indent: i64,
        keep_with_next: bool,
        flow_id: u32,
        flow_fingerprint: [u8; 32],
        parent_flow_id: u32,
        parent_position: u32,
        terminal: u32,
        terminal_receipt_fingerprint: [u8; 32],
    },
}

impl StagingSafeVectorPlacementDetailsV2 {
    pub const fn metrics(&self) -> Option<StagingVectorMetricFactV2> {
        match self {
            Self::Inline { metrics, .. } | Self::MathVectorBlock { metrics, .. } => Some(*metrics),
            Self::Figure { .. } | Self::VectorFigure { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorManifestPlacementV2 {
    usage_id: u32,
    owner: NodeId,
    kind: StagingCombinedVectorKindV2,
    image_id: ImageResourceId,
    source_id: u32,
    source_start: u32,
    source_end: u32,
    alternative_sha256: [u8; 32],
    authored_actual_text_sha256: Option<[u8; 32]>,
    language: String,
    page_index: u32,
    frame_index: u32,
    fragment_ordinal: u32,
    paint_ordinal: u32,
    viewport: Rect,
    scale: i32,
    matrix: AffineTransform,
    metric_receipt_fingerprint: Option<[u8; 32]>,
    binding_fingerprint: Option<[u8; 32]>,
    selected_placement_fingerprint: [u8; 32],
    display_command_fingerprint: [u8; 32],
    pdf_use_fingerprint: [u8; 32],
    pdf_page_object_number: u32,
    pdf_content_object_number: u32,
    pdf_form_object_number: u32,
    details: StagingSafeVectorPlacementDetailsV2,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorManifestPlacementV2 {
    pub const fn usage_id(&self) -> u32 {
        self.usage_id
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn kind(&self) -> StagingCombinedVectorKindV2 {
        self.kind
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub fn language(&self) -> &str {
        &self.language
    }
    pub const fn details(&self) -> &StagingSafeVectorPlacementDetailsV2 {
        &self.details
    }
    pub const fn binding_fingerprint(&self) -> Option<[u8; 32]> {
        self.binding_fingerprint
    }
    pub const fn metric_receipt_fingerprint(&self) -> Option<[u8; 32]> {
        self.metric_receipt_fingerprint
    }
    pub const fn selected_placement_fingerprint(&self) -> [u8; 32] {
        self.selected_placement_fingerprint
    }
    pub const fn display_command_fingerprint(&self) -> [u8; 32] {
        self.display_command_fingerprint
    }
    pub const fn pdf_use_fingerprint(&self) -> [u8; 32] {
        self.pdf_use_fingerprint
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn fragment_ordinal(&self) -> u32 {
        self.fragment_ordinal
    }
    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorManifestResourceV2 {
    content_key: VectorContentKey,
    svg_byte_length: u64,
    parser_id: &'static str,
    ir_id: &'static str,
    ir_fingerprint_id: &'static str,
    allocation_charge_id: &'static str,
    allocation_charge: u64,
    intrinsic_width: i64,
    intrinsic_height: i64,
    view_box: [i64; 4],
    aliases: Vec<StagingSafeVectorManifestAliasV2>,
    placements: Vec<StagingSafeVectorManifestPlacementV2>,
    total_placement_count: u32,
    form_plan_fingerprint: Option<[u8; 32]>,
    pdf_form_object_number: Option<u32>,
    pdf_resource_name: Option<String>,
}

impl StagingSafeVectorManifestResourceV2 {
    pub const fn content_key(&self) -> &VectorContentKey {
        &self.content_key
    }
    pub fn aliases(&self) -> &[StagingSafeVectorManifestAliasV2] {
        &self.aliases
    }
    pub fn placements(&self) -> &[StagingSafeVectorManifestPlacementV2] {
        &self.placements
    }
    pub const fn total_placement_count(&self) -> u32 {
        self.total_placement_count
    }
    pub const fn pdf_form_object_number(&self) -> Option<u32> {
        self.pdf_form_object_number
    }
    pub fn pdf_resource_name(&self) -> Option<&str> {
        self.pdf_resource_name.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorManifestV2 {
    resources: Vec<StagingSafeVectorManifestResourceV2>,
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    candidate_registry_fingerprint: [u8; 32],
    display_fingerprint: [u8; 32],
    form_plans_fingerprint: [u8; 32],
    pdf_contribution_fingerprint: [u8; 32],
    final_writer_fingerprint: [u8; 32],
    pdf_closure_fingerprint: [u8; 32],
    final_pdf_sha256: [u8; 32],
    placement_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorManifestV2 {
    pub fn resources(&self) -> &[StagingSafeVectorManifestResourceV2] {
        &self.resources
    }
    pub const fn placement_count(&self) -> u32 {
        self.placement_count
    }
    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package_fingerprint
    }
    pub const fn final_pdf_sha256(&self) -> [u8; 32] {
        self.final_pdf_sha256
    }
    pub fn placement(&self, usage_id: u32) -> Option<&StagingSafeVectorManifestPlacementV2> {
        self.resources
            .iter()
            .flat_map(|resource| resource.placements())
            .find(|value| value.usage_id == usage_id)
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorManifestV2Error {
    ProfileMismatch,
    NavigationMismatch,
    BindingMismatch,
    CandidateMismatch,
    DisplayMismatch,
    PlanMismatch,
    PdfMismatch,
    UsageMismatch,
    CountOverflow,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingSafeVectorManifestV2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "I9190: SafeVector manifest /2 {:?}", self)
    }
}
impl std::error::Error for StagingSafeVectorManifestV2Error {}

#[allow(clippy::too_many_arguments)]
pub fn build_staging_safe_vector_manifest_v2(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
    navigation: &ValidatedStagingBookNavigationV2,
    display: &StagingCombinedVectorDisplayV2,
    candidates: &VectorContentCandidateRegistry,
    plans: &StagingSafeVectorFormPlansV2,
    contribution: &StagingSafeVectorPdfContributionV2,
    pdf: &StagingTaggedPdfV2,
) -> Result<StagingSafeVectorManifestV2, StagingSafeVectorManifestV2Error> {
    profile
        .authorizes(package, limits)
        .map_err(|_| StagingSafeVectorManifestV2Error::ProfileMismatch)?;
    navigation
        .verify(package, limits)
        .map_err(|_| StagingSafeVectorManifestV2Error::NavigationMismatch)?;
    bindings
        .verify(package, profile, limits, admitted)
        .map_err(|_| StagingSafeVectorManifestV2Error::BindingMismatch)?;
    display
        .verify_resource_closure()
        .map_err(|_| StagingSafeVectorManifestV2Error::DisplayMismatch)?;
    if display.receipt().package_sha256() != package.canonical_jcs_sha256()
        || display.receipt().semantic_sha256() != package.semantic_fingerprint()
        || display.receipt().admitted_sha256() != admitted.fingerprint().bytes()
        || display.receipt().profile_sha256() != profile.profile_fingerprint()
        || display.receipt().limits_sha256() != limits.fingerprint()
        || display.receipt().binding_set_sha256() != bindings.fingerprint()
    {
        return Err(StagingSafeVectorManifestV2Error::DisplayMismatch);
    }
    candidates
        .verify(admitted, package.resources())
        .map_err(|_| StagingSafeVectorManifestV2Error::CandidateMismatch)?;
    plans
        .verify_combined_pdf_closure(display, candidates, limits)
        .map_err(|_| StagingSafeVectorManifestV2Error::PlanMismatch)?;
    contribution
        .verify_combined(display, plans, candidates, limits)
        .map_err(|_| StagingSafeVectorManifestV2Error::PdfMismatch)?;
    let media = close_staging_declared_media(admitted, package.resources())
        .map_err(|_| StagingSafeVectorManifestV2Error::CandidateMismatch)?;
    let final_writer = pdf.vector_final_writer();
    if final_writer.contribution_fingerprint() != contribution.fingerprint()
        || pdf.safe_vector().contribution_fingerprint() != contribution.fingerprint()
        || pdf.safe_vector().final_writer_observation_fingerprint() != final_writer.fingerprint()
        || pdf.safe_vector().final_pdf_sha256() != pdf.final_pdf().content_hash()
        || pdf.observation().safe_vector_pdf_sha256() != pdf.safe_vector().fingerprint()
    {
        return Err(StagingSafeVectorManifestV2Error::PdfMismatch);
    }

    let mut placement_count = 0u32;
    let mut resources = Vec::new();
    resources
        .try_reserve_exact(candidates.candidates().len())
        .map_err(|_| StagingSafeVectorManifestV2Error::AllocationFailure)?;
    for candidate in candidates.candidates() {
        let candidate_placement_count = display
            .usages()
            .iter()
            .filter(|command| command.content_key() == candidate.key())
            .count();
        let ir = candidate.canonical_ir();
        let mut placements = Vec::new();
        placements
            .try_reserve_exact(candidate_placement_count)
            .map_err(|_| StagingSafeVectorManifestV2Error::AllocationFailure)?;
        for command in display
            .usages()
            .iter()
            .filter(|command| command.content_key() == candidate.key())
        {
            let usage = contribution
                .usages()
                .iter()
                .find(|usage| usage.usage_id() == command.usage_id())
                .ok_or(StagingSafeVectorManifestV2Error::UsageMismatch)?;
            let observed = final_writer
                .usages()
                .iter()
                .copied()
                .find(|value| value.usage_id() == command.usage_id())
                .ok_or(StagingSafeVectorManifestV2Error::UsageMismatch)?;
            if usage.image_id() != command.image_id()
                || usage.content_key() != candidate.key()
                || observed.page_index() != command.page_index()
                || observed.paint_ordinal() != command.paint_ordinal()
                || observed.content_fingerprint() != usage.content_fingerprint()
                || usage.semantic_hook().display_command_fingerprint()
                    != command.display_command_fingerprint()
            {
                return Err(StagingSafeVectorManifestV2Error::UsageMismatch);
            }
            let language = navigation
                .languages()
                .record(command.owner())
                .map(|record| record.effective_language.as_ref())
                .ok_or(StagingSafeVectorManifestV2Error::NavigationMismatch)?
                .to_owned();
            let (
                span,
                alternative_sha256,
                authored_actual_text_sha256,
                metric_receipt_fingerprint,
                binding_fingerprint,
                details,
            ) = match command.relation() {
                StagingCombinedVectorUsageRelationV2::Figure { placement, .. } => {
                    let figure = figure_source_fact(package, command.owner())
                        .ok_or(StagingSafeVectorManifestV2Error::BindingMismatch)?;
                    if figure.image_id != command.image_id() || figure.placement != *placement {
                        return Err(StagingSafeVectorManifestV2Error::BindingMismatch);
                    }
                    (
                        figure.span,
                        sha256(figure.alternative.as_bytes()),
                        None,
                        None,
                        None,
                        StagingSafeVectorPlacementDetailsV2::Figure {
                            placement: placement.as_str(),
                        },
                    )
                }
                StagingCombinedVectorUsageRelationV2::Precomposed(relation) => {
                    let common = bindings
                        .receipt(command.owner())
                        .ok_or(StagingSafeVectorManifestV2Error::BindingMismatch)?;
                    if command.binding_fingerprint() != Some(common.fingerprint()) {
                        return Err(StagingSafeVectorManifestV2Error::BindingMismatch);
                    }
                    let syntax = package
                        .precomposed_vector_metrics_for(command.owner())
                        .ok_or(StagingSafeVectorManifestV2Error::BindingMismatch)?;
                    let authored = match command.kind().precomposed() {
                        Some(PrecomposedVectorKind::InlineVector) => {
                            syntax.alternative().authored_actual_text_sha256()
                        }
                        Some(
                            PrecomposedVectorKind::MathVector
                            | PrecomposedVectorKind::VectorFigure
                            | PrecomposedVectorKind::MathVectorBlock,
                        ) => None,
                        None => return Err(StagingSafeVectorManifestV2Error::BindingMismatch),
                    };
                    (
                        common.owner_source_span(),
                        common.alternative_sha256(),
                        authored,
                        Some(common.metrics_fingerprint()),
                        Some(common.fingerprint()),
                        placement_details(common, relation)?,
                    )
                }
            };
            let mut placement = StagingSafeVectorManifestPlacementV2 {
                usage_id: command.usage_id(),
                owner: command.owner(),
                kind: command.kind(),
                image_id: command.image_id(),
                source_id: span.source_id().get(),
                source_start: span.start_byte().get(),
                source_end: span.end_byte().get(),
                alternative_sha256,
                authored_actual_text_sha256,
                language,
                page_index: command.page_index(),
                frame_index: command.frame_index(),
                fragment_ordinal: command.fragment_ordinal(),
                paint_ordinal: command.paint_ordinal(),
                viewport: command.viewport(),
                scale: command.scale_raw(),
                matrix: command.matrix(),
                metric_receipt_fingerprint,
                binding_fingerprint,
                selected_placement_fingerprint: command.selected_placement_fingerprint(),
                display_command_fingerprint: command.display_command_fingerprint(),
                pdf_use_fingerprint: usage.content_fingerprint(),
                pdf_page_object_number: observed.page_object_number(),
                pdf_content_object_number: observed.page_content_object_number(),
                pdf_form_object_number: observed.form_absolute_object_number(),
                details,
                canonical_jcs: String::new(),
                fingerprint: [0; 32],
            };
            placement.canonical_jcs = encode_placement(&placement);
            placement.fingerprint = sha256(placement.canonical_jcs.as_bytes());
            placements.push(placement);
            placement_count = placement_count
                .checked_add(1)
                .ok_or(StagingSafeVectorManifestV2Error::CountOverflow)?;
        }
        placements.sort_unstable_by_key(|value| (value.page_index, value.paint_ordinal));
        let plan = plans.plan(candidate.key());
        if plan.is_some() != !placements.is_empty() {
            return Err(StagingSafeVectorManifestV2Error::PlanMismatch);
        }
        let (form_plan_fingerprint, pdf_form_object_number, pdf_resource_name) = match plan {
            Some(plan) => {
                if usize::try_from(plan.total_usage_count()).ok() != Some(placements.len()) {
                    return Err(StagingSafeVectorManifestV2Error::PlanMismatch);
                }
                let absolute = final_writer
                    .object_table()
                    .iter()
                    .copied()
                    .find(|row| row.relative_object_role() == plan.form_relative_object_role())
                    .ok_or(StagingSafeVectorManifestV2Error::PdfMismatch)?
                    .absolute_object_number();
                (
                    Some(plan.fingerprint()),
                    Some(absolute),
                    Some(plan.form_resource_name().to_owned()),
                )
            }
            None => (None, None, None),
        };
        let mut aliases = Vec::new();
        aliases
            .try_reserve_exact(candidate.aliases().len())
            .map_err(|_| StagingSafeVectorManifestV2Error::AllocationFailure)?;
        for alias in candidate.aliases() {
            let image = admitted
                .image(alias.image_id())
                .ok_or(StagingSafeVectorManifestV2Error::CandidateMismatch)?;
            let attestation = media
                .images()
                .iter()
                .find(|value| value.image_id() == alias.image_id())
                .ok_or(StagingSafeVectorManifestV2Error::CandidateMismatch)?;
            if image.content_hash() != candidate.key().source_sha256()
                || attestation.content_hash() != image.content_hash()
                || attestation.uri() != alias.uri()
                || attestation.declared().as_str() != candidate.key().media_type().as_str()
                || attestation.attested().as_str() != candidate.key().media_type().as_str()
                || attestation.safe_vector_ir_fingerprint()
                    != Some(candidate.key().ir_fingerprint())
                || attestation.safe_vector_allocation_charge()
                    != Some(alias.admission_allocation_charge())
                || attestation.m4_limits_fingerprint() != Some(alias.limits_fingerprint())
                || attestation.m4_profile_fingerprint() != Some(alias.profile_fingerprint())
            {
                return Err(StagingSafeVectorManifestV2Error::CandidateMismatch);
            }
            let mut usage_fingerprints = Vec::new();
            usage_fingerprints
                .try_reserve_exact(placements.len())
                .map_err(|_| StagingSafeVectorManifestV2Error::AllocationFailure)?;
            usage_fingerprints.extend(
                placements
                    .iter()
                    .filter(|value| value.image_id == alias.image_id())
                    .map(|value| value.fingerprint),
            );
            let provenance = match alias.provenance() {
                VectorContentAliasProvenance::SafeSvg1Absent => None,
                VectorContentAliasProvenance::SafeSvg2(value) => Some(value.clone()),
            };
            let attestation_fingerprint = declared_image_attestation_fingerprint(attestation);
            aliases.push(StagingSafeVectorManifestAliasV2 {
                image_id: alias.image_id(),
                uri: alias.uri().as_str().to_owned(),
                expected_sha256: alias.expected_sha256(),
                admitted_sha256: alias.admitted_sha256(),
                admission_attestation_fingerprint: attestation_fingerprint,
                allocation_charge: alias.admission_allocation_charge(),
                provenance,
                placement_count: u32::try_from(usage_fingerprints.len())
                    .map_err(|_| StagingSafeVectorManifestV2Error::CountOverflow)?,
                usage_fingerprints,
            });
        }
        aliases.sort_unstable_by_key(|value| value.image_id);
        let byte_length = candidate
            .aliases()
            .first()
            .and_then(|alias| admitted.image(alias.image_id()))
            .map(|image| image.byte_length())
            .ok_or(StagingSafeVectorManifestV2Error::CandidateMismatch)?;
        resources.push(StagingSafeVectorManifestResourceV2 {
            content_key: *candidate.key(),
            svg_byte_length: byte_length,
            parser_id: ir.parser_id(),
            ir_id: ir.ir_id(),
            ir_fingerprint_id: ir.ir_fingerprint_id(),
            allocation_charge_id: ir.allocation_charge_id(),
            allocation_charge: ir.allocation_charge(),
            intrinsic_width: candidate.intrinsic_width().get().raw(),
            intrinsic_height: candidate.intrinsic_height().get().raw(),
            view_box: candidate.view_box(),
            aliases,
            total_placement_count: u32::try_from(placements.len())
                .map_err(|_| StagingSafeVectorManifestV2Error::CountOverflow)?,
            placements,
            form_plan_fingerprint,
            pdf_form_object_number,
            pdf_resource_name,
        });
    }
    if placement_count != display.receipt().usage_count()
        || usize::try_from(placement_count).ok() != Some(contribution.usages().len())
    {
        return Err(StagingSafeVectorManifestV2Error::UsageMismatch);
    }
    let canonical_jcs = encode_manifest(
        package,
        profile,
        limits,
        admitted,
        candidates,
        display,
        plans,
        contribution,
        pdf,
        placement_count,
        &resources,
    );
    Ok(StagingSafeVectorManifestV2 {
        resources,
        package_fingerprint: package.semantic_fingerprint(),
        profile_fingerprint: profile.profile_fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        admitted_fingerprint: admitted.fingerprint().bytes(),
        candidate_registry_fingerprint: candidates.receipt().fingerprint(),
        display_fingerprint: display.receipt().fingerprint(),
        form_plans_fingerprint: plans.fingerprint(),
        pdf_contribution_fingerprint: contribution.fingerprint(),
        final_writer_fingerprint: final_writer.fingerprint(),
        pdf_closure_fingerprint: pdf.safe_vector().fingerprint(),
        final_pdf_sha256: pdf.final_pdf().content_hash(),
        placement_count,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn declared_image_attestation_fingerprint(
    value: &typaxis_resource_admission::StagingDeclaredImageAttestation,
) -> [u8; 32] {
    let mut out = String::from("{\"allocation_charge\":");
    push_optional_u64(&mut out, value.safe_vector_allocation_charge());
    out.push_str(",\"attested_media\":");
    push_jcs_string(&mut out, value.attested().as_str());
    out.push_str(",\"declared_media\":");
    push_jcs_string(&mut out, value.declared().as_str());
    out.push_str(",\"image_id\":");
    out.push_str(&value.image_id().get().to_string());
    out.push_str(",\"ir_fingerprint\":");
    push_optional_hash(&mut out, value.safe_vector_ir_fingerprint());
    out.push_str(",\"ir_id\":");
    push_optional_string(&mut out, value.safe_vector_ir_id());
    out.push_str(",\"limits_sha256\":");
    push_optional_hash(&mut out, value.m4_limits_fingerprint());
    out.push_str(",\"parser_id\":");
    push_optional_string(&mut out, value.safe_vector_parser_id());
    out.push_str(",\"profile_sha256\":");
    push_optional_hash(&mut out, value.m4_profile_fingerprint());
    out.push_str(",\"sha256\":");
    push_hash(&mut out, value.content_hash());
    out.push_str(",\"uri\":");
    push_jcs_string(&mut out, value.uri().as_str());
    out.push('}');
    sha256(out.as_bytes())
}

struct FigureSourceFact<'a> {
    image_id: ImageResourceId,
    placement: typaxis_document::StagingM4FigurePlacement,
    span: SourceSpan,
    alternative: &'a str,
}

fn figure_source_fact<'a>(
    package: &'a ValidatedStagingSemanticPackage,
    owner: NodeId,
) -> Option<FigureSourceFact<'a>> {
    figure_source_fact_in_blocks(&package.document().blocks, owner).or_else(|| {
        package
            .document()
            .footnotes
            .iter()
            .find_map(|footnote| figure_source_fact_in_blocks(&footnote.blocks, owner))
    })
}

fn figure_source_fact_in_blocks<'a>(
    blocks: &'a [StagingM4Block],
    owner: NodeId,
) -> Option<FigureSourceFact<'a>> {
    for block in blocks {
        match block {
            StagingM4Block::Figure {
                common,
                image_id,
                placement,
                alternative,
                caption,
                ..
            } => {
                if common.node_id == owner {
                    return Some(FigureSourceFact {
                        image_id: *image_id,
                        placement: *placement,
                        span: common.span,
                        alternative,
                    });
                }
                if let Some(found) = figure_source_fact_in_blocks(caption, owner) {
                    return Some(found);
                }
            }
            StagingM4Block::List { items, .. } => {
                if let Some(found) = items
                    .iter()
                    .find_map(|item| figure_source_fact_in_blocks(&item.blocks, owner))
                {
                    return Some(found);
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                if let Some(found) = head
                    .iter()
                    .chain(body)
                    .flat_map(|row| &row.cells)
                    .find_map(|cell| figure_source_fact_in_blocks(&cell.blocks, owner))
                {
                    return Some(found);
                }
            }
            StagingM4Block::SemanticContainer { blocks, .. } => {
                if let Some(found) = figure_source_fact_in_blocks(blocks, owner) {
                    return Some(found);
                }
            }
            StagingM4Block::Paragraph { .. }
            | StagingM4Block::Heading { .. }
            | StagingM4Block::PageBreak { .. }
            | StagingM4Block::DisplayMath { .. }
            | StagingM4Block::VectorFigure { .. }
            | StagingM4Block::MathVectorBlock { .. } => {}
        }
    }
    None
}

fn placement_details(
    common: &ValidatedPrecomposedVectorReceipt,
    relation: &StagingDrawVectorV2Relation,
) -> Result<StagingSafeVectorPlacementDetailsV2, StagingSafeVectorManifestV2Error> {
    match (common.placement(), relation) {
        (
            PrecomposedVectorPlacementInput::Inline(value),
            StagingDrawVectorV2Relation::Inline { .. },
        ) => Ok(StagingSafeVectorPlacementDetailsV2::Inline {
            metrics: metric_fact(value.metrics()),
            spacing_before: value.spacing_before().get().raw(),
            spacing_after: value.spacing_after().get().raw(),
        }),
        (
            PrecomposedVectorPlacementInput::VectorFigure(value),
            StagingDrawVectorV2Relation::VectorFigure { .. },
        ) => Ok(StagingSafeVectorPlacementDetailsV2::VectorFigure {
            style_fingerprint: value.style().fingerprint(),
            alignment: value.style().text_align().as_str(),
            space_before: value.style().space_before().get().raw(),
            space_after: value.style().space_after().get().raw(),
            start_indent: value.style().start_indent().get().raw(),
            end_indent: value.style().end_indent().get().raw(),
            keep_caption: value.style().keep_caption(),
            keep_with_next: value.style().keep_with_next(),
        }),
        (
            PrecomposedVectorPlacementInput::MathVectorBlock(value),
            StagingDrawVectorV2Relation::MathVectorBlock { math_flow, .. },
        ) => Ok(StagingSafeVectorPlacementDetailsV2::MathVectorBlock {
            metrics: metric_fact(value.metrics()),
            style_fingerprint: value.style().fingerprint(),
            alignment: value.style().text_align().as_str(),
            space_before: value.style().space_before().get().raw(),
            space_after: value.style().space_after().get().raw(),
            start_indent: value.style().start_indent().get().raw(),
            end_indent: value.style().end_indent().get().raw(),
            keep_with_next: value.style().keep_with_next(),
            flow_id: math_flow.flow_id().get(),
            flow_fingerprint: math_flow.flow_fingerprint(),
            parent_flow_id: math_flow.parent_flow_id().get(),
            parent_position: math_flow.parent_position(),
            terminal: math_flow.terminal().get(),
            terminal_receipt_fingerprint: math_flow.terminal_receipt_fingerprint(),
        }),
        _ => Err(StagingSafeVectorManifestV2Error::BindingMismatch),
    }
}

fn metric_fact(value: typaxis_layout::BoundPrecomposedVectorMetrics) -> StagingVectorMetricFactV2 {
    StagingVectorMetricFactV2 {
        advance: value.advance().get().raw(),
        ascent: value.ascent().get().raw(),
        baseline: value.baseline().get().raw(),
        descent: value.descent().get().raw(),
        origin_x: value.origin_x().raw(),
        viewport_width: value.viewport_width().get().raw(),
        viewport_height: value.viewport_height().get().raw(),
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_manifest(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    candidates: &VectorContentCandidateRegistry,
    display: &StagingCombinedVectorDisplayV2,
    plans: &StagingSafeVectorFormPlansV2,
    contribution: &StagingSafeVectorPdfContributionV2,
    pdf: &StagingTaggedPdfV2,
    placement_count: u32,
    resources: &[StagingSafeVectorManifestResourceV2],
) -> String {
    let mut out = String::from("{\"algorithm\":");
    push_jcs_string(&mut out, STAGING_SAFE_VECTOR_MANIFEST_V2_ALGORITHM);
    out.push_str(",\"contract\":\"typaxis.contract/1.4\",\"fingerprints\":{");
    for (index, (key, value)) in [
        ("admitted_sha256", admitted.fingerprint().bytes()),
        (
            "candidate_registry_sha256",
            candidates.receipt().fingerprint(),
        ),
        ("display_sha256", display.receipt().fingerprint()),
        (
            "final_writer_sha256",
            pdf.vector_final_writer().fingerprint(),
        ),
        ("form_plans_sha256", plans.fingerprint()),
        ("limits_sha256", limits.fingerprint()),
        ("package_sha256", package.semantic_fingerprint()),
        ("pdf_closure_sha256", pdf.safe_vector().fingerprint()),
        ("pdf_contribution_sha256", contribution.fingerprint()),
        ("pdf_sha256", pdf.final_pdf().content_hash()),
        ("profile_sha256", profile.profile_fingerprint()),
    ]
    .into_iter()
    .enumerate()
    {
        if index > 0 {
            out.push(',');
        }
        push_jcs_string(&mut out, key);
        out.push(':');
        push_hash(&mut out, value);
    }
    out.push_str("},\"placement_count\":");
    out.push_str(&placement_count.to_string());
    out.push_str(",\"resources\":[");
    for (index, resource) in resources.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        encode_resource(&mut out, resource);
    }
    out.push_str("]}");
    out
}

fn encode_resource(out: &mut String, value: &StagingSafeVectorManifestResourceV2) {
    out.push_str("{\"aliases\":[");
    for (index, alias) in value.aliases.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        encode_alias(out, alias);
    }
    out.push_str("],\"allocation_charge\":");
    out.push_str(&value.allocation_charge.to_string());
    out.push_str(",\"allocation_charge_id\":");
    push_jcs_string(out, value.allocation_charge_id);
    out.push_str(",\"content_key\":");
    push_content_key(out, value.content_key);
    out.push_str(",\"form_plan_fingerprint\":");
    push_optional_hash(out, value.form_plan_fingerprint);
    out.push_str(",\"intrinsic_height\":");
    out.push_str(&value.intrinsic_height.to_string());
    out.push_str(",\"intrinsic_width\":");
    out.push_str(&value.intrinsic_width.to_string());
    out.push_str(",\"ir_fingerprint_id\":");
    push_jcs_string(out, value.ir_fingerprint_id);
    out.push_str(",\"ir_id\":");
    push_jcs_string(out, value.ir_id);
    out.push_str(",\"parser_id\":");
    push_jcs_string(out, value.parser_id);
    out.push_str(",\"pdf_form_object_number\":");
    push_optional_u32(out, value.pdf_form_object_number);
    out.push_str(",\"pdf_resource_name\":");
    match &value.pdf_resource_name {
        Some(v) => push_jcs_string(out, v),
        None => out.push_str("null"),
    };
    out.push_str(",\"placements\":[");
    for (index, placement) in value.placements.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(placement.canonical_jcs());
    }
    out.push_str("],\"svg_byte_length\":");
    out.push_str(&value.svg_byte_length.to_string());
    out.push_str(",\"total_placement_count\":");
    out.push_str(&value.total_placement_count.to_string());
    out.push_str(",\"view_box\":[");
    push_i64s(out, &value.view_box);
    out.push_str("]}");
}

fn encode_alias(out: &mut String, value: &StagingSafeVectorManifestAliasV2) {
    out.push_str("{\"admission_allocation_charge\":");
    out.push_str(&value.allocation_charge.to_string());
    out.push_str(",\"admission_attestation_fingerprint\":");
    push_hash(out, value.admission_attestation_fingerprint);
    out.push_str(",\"admitted_sha256\":");
    push_hash(out, value.admitted_sha256);
    out.push_str(",\"expected_sha256\":");
    push_optional_hash(out, value.expected_sha256);
    out.push_str(",\"image_id\":");
    out.push_str(&value.image_id.get().to_string());
    out.push_str(",\"placement_count\":");
    out.push_str(&value.placement_count.to_string());
    if let Some(provenance) = &value.provenance {
        out.push_str(",\"provenance\":{\"engine_id\":");
        push_jcs_string(out, &provenance.engine_id);
        out.push_str(",\"engine_version\":");
        push_jcs_string(out, &provenance.engine_version);
        out.push_str(",\"rules_version\":");
        push_jcs_string(out, &provenance.rules_version);
        out.push('}');
    }
    out.push_str(",\"uri\":");
    push_jcs_string(out, &value.uri);
    out.push_str(",\"usage_fingerprints\":[");
    for (index, fingerprint) in value.usage_fingerprints.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_hash(out, *fingerprint);
    }
    out.push_str("]}");
}

fn encode_placement(value: &StagingSafeVectorManifestPlacementV2) -> String {
    let mut out = String::from("{\"alternative_sha256\":");
    push_hash(&mut out, value.alternative_sha256);
    if value.kind == StagingCombinedVectorKindV2::InlineVector {
        out.push_str(",\"authored_actual_text_sha256\":");
        push_optional_hash(&mut out, value.authored_actual_text_sha256);
    }
    if let Some(binding_fingerprint) = value.binding_fingerprint {
        out.push_str(",\"binding_fingerprint\":");
        push_hash(&mut out, binding_fingerprint);
    }
    match &value.details {
        StagingSafeVectorPlacementDetailsV2::VectorFigure {
            style_fingerprint,
            alignment,
            space_before,
            space_after,
            start_indent,
            end_indent,
            keep_with_next,
            ..
        }
        | StagingSafeVectorPlacementDetailsV2::MathVectorBlock {
            style_fingerprint,
            alignment,
            space_before,
            space_after,
            start_indent,
            end_indent,
            keep_with_next,
            ..
        } => {
            out.push_str(",\"block_style\":");
            push_block_style(
                &mut out,
                *style_fingerprint,
                alignment,
                *space_before,
                *space_after,
                *start_indent,
                *end_indent,
                *keep_with_next,
            );
        }
        StagingSafeVectorPlacementDetailsV2::Figure { .. }
        | StagingSafeVectorPlacementDetailsV2::Inline { .. } => {}
    }
    out.push_str(",\"display_command_fingerprint\":");
    push_hash(&mut out, value.display_command_fingerprint);
    out.push_str(",\"fragment_ordinal\":");
    out.push_str(&value.fragment_ordinal.to_string());
    out.push_str(",\"frame_index\":");
    out.push_str(&value.frame_index.to_string());
    out.push_str(",\"image_id\":");
    out.push_str(&value.image_id.get().to_string());
    if let StagingSafeVectorPlacementDetailsV2::VectorFigure { keep_caption, .. } = &value.details {
        out.push_str(",\"keep_caption\":");
        out.push_str(if *keep_caption { "true" } else { "false" });
    }
    out.push_str(",\"kind\":");
    push_jcs_string(&mut out, value.kind.as_str());
    out.push_str(",\"language\":");
    push_jcs_string(&mut out, &value.language);
    if let StagingSafeVectorPlacementDetailsV2::MathVectorBlock {
        flow_id,
        flow_fingerprint,
        parent_flow_id,
        parent_position,
        terminal,
        terminal_receipt_fingerprint,
        ..
    } = &value.details
    {
        out.push_str(",\"math_flow\":{");
        out.push_str("\"algorithm\":\"typaxis.math-vector-flow/1\",\"flow_fingerprint\":");
        push_hash(&mut out, *flow_fingerprint);
        out.push_str(",\"flow_id\":");
        out.push_str(&flow_id.to_string());
        out.push_str(",\"parent_flow_id\":");
        out.push_str(&parent_flow_id.to_string());
        out.push_str(",\"parent_position\":");
        out.push_str(&parent_position.to_string());
        out.push_str(",\"terminal\":");
        out.push_str(&terminal.to_string());
        out.push_str(",\"terminal_receipt_fingerprint\":");
        push_hash(&mut out, *terminal_receipt_fingerprint);
        out.push('}');
    }
    out.push_str(",\"matrix\":");
    push_matrix(&mut out, value.matrix);
    if let Some(metric_receipt_fingerprint) = value.metric_receipt_fingerprint {
        out.push_str(",\"metric_receipt_fingerprint\":");
        push_hash(&mut out, metric_receipt_fingerprint);
    }
    if let Some(metrics) = value.details.metrics() {
        out.push_str(",\"metrics\":");
        push_metrics(&mut out, metrics);
    }
    out.push_str(",\"node_id\":");
    out.push_str(&value.owner.get().to_string());
    out.push_str(",\"page_index\":");
    out.push_str(&value.page_index.to_string());
    out.push_str(",\"paint_ordinal\":");
    out.push_str(&value.paint_ordinal.to_string());
    out.push_str(",\"pdf_content_object_number\":");
    out.push_str(&value.pdf_content_object_number.to_string());
    out.push_str(",\"pdf_form_object_number\":");
    out.push_str(&value.pdf_form_object_number.to_string());
    out.push_str(",\"pdf_page_object_number\":");
    out.push_str(&value.pdf_page_object_number.to_string());
    out.push_str(",\"pdf_use_fingerprint\":");
    push_hash(&mut out, value.pdf_use_fingerprint);
    if let StagingSafeVectorPlacementDetailsV2::Figure { placement } = &value.details {
        out.push_str(",\"placement\":");
        push_jcs_string(&mut out, placement);
    }
    out.push_str(",\"scale\":");
    out.push_str(&value.scale.to_string());
    out.push_str(",\"selected_placement_fingerprint\":");
    push_hash(&mut out, value.selected_placement_fingerprint);
    out.push_str(",\"source_span\":{\"end_byte\":");
    out.push_str(&value.source_end.to_string());
    out.push_str(",\"source_id\":");
    out.push_str(&value.source_id.to_string());
    out.push_str(",\"start_byte\":");
    out.push_str(&value.source_start.to_string());
    out.push('}');
    if let StagingSafeVectorPlacementDetailsV2::Inline {
        spacing_before,
        spacing_after,
        ..
    } = &value.details
    {
        out.push_str(",\"spacing_after\":");
        out.push_str(&spacing_after.to_string());
        out.push_str(",\"spacing_before\":");
        out.push_str(&spacing_before.to_string());
    }
    out.push_str(",\"usage_id\":");
    out.push_str(&value.usage_id.to_string());
    out.push_str(",\"viewport\":");
    push_rect(&mut out, value.viewport);
    out.push('}');
    out
}

#[allow(clippy::too_many_arguments)]
fn push_block_style(
    out: &mut String,
    fingerprint: [u8; 32],
    alignment: &str,
    before: i64,
    after: i64,
    start: i64,
    end: i64,
    keep: bool,
) {
    out.push_str("{\"alignment\":");
    push_jcs_string(out, alignment);
    out.push_str(",\"end_indent\":");
    out.push_str(&end.to_string());
    out.push_str(",\"fingerprint\":");
    push_hash(out, fingerprint);
    out.push_str(",\"keep_with_next\":");
    out.push_str(if keep { "true" } else { "false" });
    out.push_str(",\"space_after\":");
    out.push_str(&after.to_string());
    out.push_str(",\"space_before\":");
    out.push_str(&before.to_string());
    out.push_str(",\"start_indent\":");
    out.push_str(&start.to_string());
    out.push('}');
}
fn push_metrics(out: &mut String, value: StagingVectorMetricFactV2) {
    out.push_str("{\"advance\":");
    out.push_str(&value.advance.to_string());
    out.push_str(",\"ascent\":");
    out.push_str(&value.ascent.to_string());
    out.push_str(",\"baseline\":");
    out.push_str(&value.baseline.to_string());
    out.push_str(",\"descent\":");
    out.push_str(&value.descent.to_string());
    out.push_str(",\"origin_x\":");
    out.push_str(&value.origin_x.to_string());
    out.push_str(",\"viewport_height\":");
    out.push_str(&value.viewport_height.to_string());
    out.push_str(",\"viewport_width\":");
    out.push_str(&value.viewport_width.to_string());
    out.push('}');
}
fn push_content_key(out: &mut String, key: VectorContentKey) {
    out.push_str("{\"ir_fingerprint\":");
    push_hash(out, key.ir_fingerprint());
    out.push_str(",\"ir_id\":");
    push_jcs_string(out, key.ir_id());
    out.push_str(",\"media_type\":");
    push_jcs_string(out, key.media_type().as_str());
    out.push_str(",\"parser_id\":");
    push_jcs_string(out, key.parser_id());
    out.push_str(",\"source_sha256\":");
    push_hash(out, key.source_sha256());
    out.push('}');
}
fn push_matrix(out: &mut String, value: AffineTransform) {
    out.push('[');
    push_i64s(
        out,
        &[
            i64::from(value.a.raw()),
            i64::from(value.b.raw()),
            i64::from(value.c.raw()),
            i64::from(value.d.raw()),
            value.e.raw(),
            value.f.raw(),
        ],
    );
    out.push(']');
}
fn push_rect(out: &mut String, value: Rect) {
    out.push('[');
    push_i64s(
        out,
        &[
            value.x().raw(),
            value.y().raw(),
            value.width().get().raw(),
            value.height().get().raw(),
        ],
    );
    out.push(']');
}
fn push_i64s(out: &mut String, values: &[i64]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&value.to_string());
    }
}
fn push_optional_u32(out: &mut String, value: Option<u32>) {
    match value {
        Some(v) => out.push_str(&v.to_string()),
        None => out.push_str("null"),
    }
}
fn push_optional_u64(out: &mut String, value: Option<u64>) {
    match value {
        Some(v) => out.push_str(&v.to_string()),
        None => out.push_str("null"),
    }
}
fn push_optional_string(out: &mut String, value: Option<&str>) {
    match value {
        Some(v) => push_jcs_string(out, v),
        None => out.push_str("null"),
    }
}
fn push_optional_hash(out: &mut String, value: Option<[u8; 32]>) {
    match value {
        Some(v) => push_hash(out, v),
        None => out.push_str("null"),
    }
}
fn push_hash(out: &mut String, value: [u8; 32]) {
    out.push('"');
    out.push_str(&hex(value));
    out.push('"');
}
fn hex(value: [u8; 32]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in value {
        out.push(char::from(H[usize::from(byte >> 4)]));
        out.push(char::from(H[usize::from(byte & 15)]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_v2_fixture::{
        build_figure_vector_v2_manifests, build_vector_v2_manifests,
        manifest_figure_vector_v2_fixture, manifest_vector_v2_fixture,
    };

    #[test]
    fn safe_vector_manifest_v2_closes_content_alias_placement_and_absolute_form_facts() {
        let fixture = manifest_vector_v2_fixture().unwrap();
        let products = build_vector_v2_manifests(&fixture).unwrap();
        let manifest = &products.safe;
        assert_eq!(
            manifest.placement_count(),
            fixture.display.display.receipt().command_count()
        );
        assert_eq!(
            manifest.resources().len(),
            fixture.candidates.candidates().len()
        );
        assert!(manifest
            .resources()
            .windows(2)
            .all(|pair| pair[0].content_key() < pair[1].content_key()));
        for resource in manifest.resources() {
            assert!(resource
                .aliases()
                .windows(2)
                .all(|pair| pair[0].image_id() < pair[1].image_id()));
            assert_eq!(
                resource.total_placement_count() as usize,
                resource.placements().len()
            );
            assert_eq!(
                resource.pdf_form_object_number().is_some(),
                !resource.placements().is_empty()
            );
            assert_eq!(
                resource.pdf_resource_name().is_some(),
                !resource.placements().is_empty()
            );
            for alias in resource.aliases() {
                assert_eq!(
                    alias.placement_count() as usize,
                    alias.usage_fingerprints().len()
                );
                assert_eq!(
                    alias.provenance().is_some(),
                    resource.content_key().media_type().as_str() == "svg-safe-2"
                );
            }
        }
        assert_eq!(
            manifest.fingerprint(),
            sha256(manifest.canonical_jcs().as_bytes())
        );
        assert!(manifest
            .canonical_jcs()
            .contains("\"algorithm\":\"typaxis.safe-vector-manifest/2\""));
        assert!(manifest
            .canonical_jcs()
            .contains("\"pdf_form_object_number\":"));
    }

    #[test]
    fn safe_vector_manifest_v2_projects_existing_figure_without_math_binding() {
        let fixture = manifest_figure_vector_v2_fixture().unwrap();
        let products = build_figure_vector_v2_manifests(&fixture).unwrap();
        assert_eq!(products.safe.placement_count(), 1);
        assert_eq!(products.math.facts().len(), 0);
        let used = products
            .safe
            .resources()
            .iter()
            .find(|resource| !resource.placements().is_empty())
            .unwrap();
        assert_eq!(used.content_key().media_type().as_str(), "svg-safe-1");
        assert_eq!(used.total_placement_count(), 1);
        assert_eq!(used.aliases()[0].provenance(), None);
        let [placement] = used.placements() else {
            panic!("Figure resource must contain one placement");
        };
        assert_eq!(placement.kind(), StagingCombinedVectorKindV2::Figure);
        assert_eq!(placement.binding_fingerprint(), None);
        assert_eq!(placement.metric_receipt_fingerprint(), None);
        assert!(matches!(
            placement.details(),
            StagingSafeVectorPlacementDetailsV2::Figure { placement: "block" }
        ));
        let [structure] = products.tagged.vector_structures() else {
            panic!("Figure resource must contain one structure fact");
        };
        assert_eq!(structure.kind(), StagingCombinedVectorKindV2::Figure);
        assert_eq!(structure.math_binding_fingerprint(), None);
        let root = crate::StagingProductionBuildManifestVectorFields::built(
            &products.book,
            &products.safe,
            &products.math,
            &products.tagged,
        )
        .unwrap();
        assert!(root.math_vector_record().is_some());
    }
}
