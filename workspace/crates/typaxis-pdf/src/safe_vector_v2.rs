use std::collections::{BTreeMap, BTreeSet};

use crate::VerifiedPdfBytesReceipt;
use typaxis_core::{
    push_jcs_string, sha256, AffineTransform, ImageResourceId, M4EffectiveResourceLimits, NodeId,
};
use typaxis_display_list::{
    StagingCombinedVectorDisplayV2, StagingCombinedVectorKindV2, StagingCombinedVectorUsageV2,
    StagingDrawVectorV2, StagingPrecomposedVectorDisplay,
};
use typaxis_resources::{
    AdmittedSafeVector, FrozenSafeVectorFormPlanV2, SafeVectorClipDefinition, SafeVectorClipUse,
    SafeVectorDraw, SafeVectorDrawV2, SafeVectorFillRule, SafeVectorIr, SafeVectorIrV2,
    SafeVectorLineCap, SafeVectorLineJoin, SafeVectorPaint, SafeVectorPath, SafeVectorPoint,
    SafeVectorSegment, SafeVectorTransform, StagingSafeVectorFormPlansV2,
    VectorContentCandidateRegistry, VectorContentKey, VectorExtGStateAlphaPair,
};

pub const STAGING_SAFE_VECTOR_PDF_CONTRIBUTION_V2_ALGORITHM: &str =
    "typaxis.safe-vector-pdf-contribution/2";
pub const STAGING_SAFE_VECTOR_PDF_ALGORITHM_V2: &str = "typaxis.safe-vector-pdf-closure/2";

const FIXED_ONE: i64 = 65_536;
const MAX_COORDINATE: i64 = 1_000_000 * FIXED_ONE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorPdfRelativeObjectKindV2 {
    Form,
    ExtGState,
}

impl StagingSafeVectorPdfRelativeObjectKindV2 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Form => "form",
            Self::ExtGState => "ext-g-state",
        }
    }
}

/// One object role owned by the reusable vector contribution. Absolute object
/// numbers are intentionally absent until the complete final graph is merged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfRelativeObjectV2 {
    relative_object_role: u32,
    kind: StagingSafeVectorPdfRelativeObjectKindV2,
    content_key: VectorContentKey,
    resource_name: String,
    object_contribution_fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfRelativeObjectV2 {
    pub const fn relative_object_role(&self) -> u32 {
        self.relative_object_role
    }

    pub const fn kind(&self) -> StagingSafeVectorPdfRelativeObjectKindV2 {
        self.kind
    }

    pub const fn content_key(&self) -> &VectorContentKey {
        &self.content_key
    }

    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    pub const fn object_contribution_fingerprint(&self) -> [u8; 32] {
        self.object_contribution_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfExtGStateV2 {
    content_key: VectorContentKey,
    relative_object_role: u32,
    resource_name: String,
    fill_alpha_raw: u32,
    stroke_alpha_raw: u32,
    dictionary: Vec<u8>,
    dictionary_fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfExtGStateV2 {
    pub const fn content_key(&self) -> &VectorContentKey {
        &self.content_key
    }

    pub const fn relative_object_role(&self) -> u32 {
        self.relative_object_role
    }

    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    pub const fn fill_alpha_raw(&self) -> u32 {
        self.fill_alpha_raw
    }

    pub const fn stroke_alpha_raw(&self) -> u32 {
        self.stroke_alpha_raw
    }

    pub fn dictionary(&self) -> &[u8] {
        &self.dictionary
    }

    pub const fn dictionary_fingerprint(&self) -> [u8; 32] {
        self.dictionary_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfFormV2 {
    content_key: VectorContentKey,
    relative_object_role: u32,
    resource_name: String,
    bbox: [i64; 4],
    ext_g_state_roles: Vec<(String, u32)>,
    content_stream: Vec<u8>,
    content_stream_fingerprint: [u8; 32],
    object_contribution_fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfFormV2 {
    pub const fn content_key(&self) -> &VectorContentKey {
        &self.content_key
    }

    pub const fn relative_object_role(&self) -> u32 {
        self.relative_object_role
    }

    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    pub const fn bbox(&self) -> [i64; 4] {
        self.bbox
    }

    pub fn ext_g_state_roles(&self) -> &[(String, u32)] {
        &self.ext_g_state_roles
    }

    pub fn content_stream(&self) -> &[u8] {
        &self.content_stream
    }

    pub const fn content_stream_fingerprint(&self) -> [u8; 32] {
        self.content_stream_fingerprint
    }

    pub const fn object_contribution_fingerprint(&self) -> [u8; 32] {
        self.object_contribution_fingerprint
    }
}

/// Page-resource binding for one selected content key on one page. Repeated
/// uses on the same page share this binding and still produce separate `Do`s.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfPageResourceV2 {
    page_index: u32,
    content_key: VectorContentKey,
    form_relative_object_role: u32,
    resource_name: String,
}

impl StagingSafeVectorPdfPageResourceV2 {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn content_key(&self) -> &VectorContentKey {
        &self.content_key
    }

    pub const fn form_relative_object_role(&self) -> u32 {
        self.form_relative_object_role
    }

    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }
}

/// Information handed to the later tagging owner. It identifies the
/// page-level `Do` but deliberately has no MCID, Alt, ActualText, or Lang.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfSemanticUsageHookV2 {
    usage_id: u32,
    owner: NodeId,
    kind: StagingCombinedVectorKindV2,
    page_index: u32,
    paint_ordinal: u32,
    display_command_fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfSemanticUsageHookV2 {
    pub const fn usage_id(&self) -> u32 {
        self.usage_id
    }

    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn kind(&self) -> StagingCombinedVectorKindV2 {
        self.kind
    }

    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }

    pub const fn display_command_fingerprint(&self) -> [u8; 32] {
        self.display_command_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfUsageV2 {
    usage_id: u32,
    image_id: ImageResourceId,
    content_key: VectorContentKey,
    page_index: u32,
    paint_ordinal: u32,
    form_relative_object_role: u32,
    form_resource_name: String,
    matrix: AffineTransform,
    resolved_current_color: [u8; 3],
    content: Vec<u8>,
    content_fingerprint: [u8; 32],
    semantic_hook: StagingSafeVectorPdfSemanticUsageHookV2,
}

impl StagingSafeVectorPdfUsageV2 {
    pub const fn usage_id(&self) -> u32 {
        self.usage_id
    }

    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }

    pub const fn content_key(&self) -> &VectorContentKey {
        &self.content_key
    }

    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }

    pub const fn form_relative_object_role(&self) -> u32 {
        self.form_relative_object_role
    }

    pub fn form_resource_name(&self) -> &str {
        &self.form_resource_name
    }

    pub const fn matrix(&self) -> AffineTransform {
        self.matrix
    }

    pub const fn resolved_current_color(&self) -> [u8; 3] {
        self.resolved_current_color
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }

    pub const fn content_fingerprint(&self) -> [u8; 32] {
        self.content_fingerprint
    }

    pub const fn semantic_hook(&self) -> &StagingSafeVectorPdfSemanticUsageHookV2 {
        &self.semantic_hook
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfPageV2 {
    page_index: u32,
    resources: Vec<StagingSafeVectorPdfPageResourceV2>,
    usage_ids: Vec<u32>,
    requires_existing_top_left_page_root_y_flip: bool,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfPageV2 {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub fn resources(&self) -> &[StagingSafeVectorPdfPageResourceV2] {
        &self.resources
    }

    pub fn usage_ids(&self) -> &[u32] {
        &self.usage_ids
    }

    pub const fn requires_existing_top_left_page_root_y_flip(&self) -> bool {
        self.requires_existing_top_left_page_root_y_flip
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfContributionV2 {
    display_fingerprint: [u8; 32],
    form_plans_fingerprint: [u8; 32],
    candidate_registry_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    relative_objects: Vec<StagingSafeVectorPdfRelativeObjectV2>,
    forms: Vec<StagingSafeVectorPdfFormV2>,
    ext_g_states: Vec<StagingSafeVectorPdfExtGStateV2>,
    pages: Vec<StagingSafeVectorPdfPageV2>,
    usages: Vec<StagingSafeVectorPdfUsageV2>,
    spool_bytes: u64,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfContributionV2 {
    pub const fn algorithm(&self) -> &'static str {
        STAGING_SAFE_VECTOR_PDF_CONTRIBUTION_V2_ALGORITHM
    }

    pub const fn display_fingerprint(&self) -> [u8; 32] {
        self.display_fingerprint
    }

    pub const fn form_plans_fingerprint(&self) -> [u8; 32] {
        self.form_plans_fingerprint
    }

    pub const fn candidate_registry_fingerprint(&self) -> [u8; 32] {
        self.candidate_registry_fingerprint
    }

    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }

    pub fn relative_objects(&self) -> &[StagingSafeVectorPdfRelativeObjectV2] {
        &self.relative_objects
    }

    pub fn forms(&self) -> &[StagingSafeVectorPdfFormV2] {
        &self.forms
    }

    pub fn ext_g_states(&self) -> &[StagingSafeVectorPdfExtGStateV2] {
        &self.ext_g_states
    }

    pub fn pages(&self) -> &[StagingSafeVectorPdfPageV2] {
        &self.pages
    }

    pub fn usages(&self) -> &[StagingSafeVectorPdfUsageV2] {
        &self.usages
    }

    pub const fn spool_bytes(&self) -> u64 {
        self.spool_bytes
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        display: &StagingPrecomposedVectorDisplay,
        plans: &StagingSafeVectorFormPlansV2,
        registry: &VectorContentCandidateRegistry,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSafeVectorPdfV2Error> {
        let expected =
            build_staging_safe_vector_pdf_contribution_v2(display, plans, registry, limits)?;
        if self != &expected {
            return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
        }
        Ok(())
    }

    pub fn verify_combined(
        &self,
        display: &StagingCombinedVectorDisplayV2,
        plans: &StagingSafeVectorFormPlansV2,
        registry: &VectorContentCandidateRegistry,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSafeVectorPdfV2Error> {
        let expected = build_staging_combined_safe_vector_pdf_contribution_v2(
            display, plans, registry, limits,
        )?;
        if self != &expected {
            return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorPdfV2Error {
    DisplayMismatch,
    FormPlanMismatch,
    CandidateMismatch,
    InvalidIr,
    InvalidPlacement,
    CountOverflow,
    ArithmeticOverflow,
    SpoolLimit,
    AllocationFailure,
    ContributionMismatch,
    FinalWriterMismatch,
    FinalPdfMismatch,
}

impl std::fmt::Display for StagingSafeVectorPdfV2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DisplayMismatch => {
                formatter.write_str("I9190: DrawVector /2 Display mismatch at PDF contribution")
            }
            Self::FormPlanMismatch => {
                formatter.write_str("I9190: SafeVector Form plan /2 mismatch")
            }
            Self::CandidateMismatch => {
                formatter.write_str("I9190: vector candidate mismatch at PDF contribution")
            }
            Self::InvalidIr => formatter.write_str("I9190: invalid admitted vector IR"),
            Self::InvalidPlacement => formatter.write_str("I9190: invalid vector placement matrix"),
            Self::CountOverflow => {
                formatter.write_str("D8101: vector PDF contribution count overflow")
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("I9190: vector PDF fixed-point arithmetic overflow")
            }
            Self::SpoolLimit => {
                formatter.write_str("D8101: vector PDF contribution spool limit exceeded")
            }
            Self::AllocationFailure => {
                formatter.write_str("D8101: vector PDF contribution allocation failed")
            }
            Self::ContributionMismatch => {
                formatter.write_str("I9190: vector PDF contribution receipt mismatch")
            }
            Self::FinalWriterMismatch => {
                formatter.write_str("I9190: vector final-writer observation mismatch")
            }
            Self::FinalPdfMismatch => {
                formatter.write_str("I9190: vector final PDF receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingSafeVectorPdfV2Error {}

pub fn build_staging_safe_vector_pdf_contribution_v2(
    display: &StagingPrecomposedVectorDisplay,
    plans: &StagingSafeVectorFormPlansV2,
    registry: &VectorContentCandidateRegistry,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingSafeVectorPdfContributionV2, StagingSafeVectorPdfV2Error> {
    build_staging_safe_vector_pdf_contribution_v2_with_spool_limit(
        display,
        plans,
        registry,
        limits,
        limits.base().get().max_spool_bytes,
    )
}

/// Builds the final-writer contribution for the single production vector
/// resource set. Safe-SVG 1 Figures and all precomposed vector kinds share
/// the same content-key Form allocation and page XObject dictionaries.
pub fn build_staging_combined_safe_vector_pdf_contribution_v2(
    display: &StagingCombinedVectorDisplayV2,
    plans: &StagingSafeVectorFormPlansV2,
    registry: &VectorContentCandidateRegistry,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingSafeVectorPdfContributionV2, StagingSafeVectorPdfV2Error> {
    display
        .verify_resource_closure()
        .map_err(|_| StagingSafeVectorPdfV2Error::DisplayMismatch)?;
    plans
        .verify_combined_pdf_closure(display, registry, limits)
        .map_err(|_| StagingSafeVectorPdfV2Error::FormPlanMismatch)?;
    let mut inputs = Vec::new();
    inputs
        .try_reserve_exact(display.usages().len())
        .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
    inputs.extend(display.usages().iter().map(PdfUsageInput::from_combined));
    build_staging_safe_vector_pdf_contribution_v2_from_inputs(
        display.receipt().fingerprint(),
        display.receipt().page_count(),
        &inputs,
        plans,
        registry,
        limits,
        limits.base().get().max_spool_bytes,
    )
}

#[derive(Clone, Copy)]
struct PdfUsageInput {
    usage_id: u32,
    owner: NodeId,
    kind: StagingCombinedVectorKindV2,
    image_id: ImageResourceId,
    content_key: VectorContentKey,
    ir_fingerprint: [u8; 32],
    page_index: u32,
    paint_ordinal: u32,
    viewport: typaxis_core::Rect,
    scale: i32,
    matrix: AffineTransform,
    color: [u8; 3],
    display_command_fingerprint: [u8; 32],
}

impl PdfUsageInput {
    fn from_precomposed(command: &StagingDrawVectorV2) -> Self {
        let color = command.resolved_current_color();
        Self {
            usage_id: command.usage_id(),
            owner: command.owner(),
            kind: command.kind().into(),
            image_id: command.image_id(),
            content_key: command.content_key(),
            ir_fingerprint: command.ir_fingerprint(),
            page_index: command.page_index(),
            paint_ordinal: command.paint_ordinal(),
            viewport: command.viewport(),
            scale: command.scale_raw(),
            matrix: command.matrix(),
            color: [color.red(), color.green(), color.blue()],
            display_command_fingerprint: command.fingerprint(),
        }
    }

    fn from_combined(usage: &StagingCombinedVectorUsageV2) -> Self {
        Self {
            usage_id: usage.usage_id(),
            owner: usage.owner(),
            kind: usage.kind(),
            image_id: usage.image_id(),
            content_key: *usage.content_key(),
            ir_fingerprint: usage.ir_fingerprint(),
            page_index: usage.page_index(),
            paint_ordinal: usage.paint_ordinal(),
            viewport: usage.viewport(),
            scale: usage.scale_raw(),
            matrix: usage.matrix(),
            color: usage.resolved_current_color(),
            display_command_fingerprint: usage.display_command_fingerprint(),
        }
    }
}

fn build_staging_safe_vector_pdf_contribution_v2_with_spool_limit(
    display: &StagingPrecomposedVectorDisplay,
    plans: &StagingSafeVectorFormPlansV2,
    registry: &VectorContentCandidateRegistry,
    limits: &M4EffectiveResourceLimits,
    spool_limit: u64,
) -> Result<StagingSafeVectorPdfContributionV2, StagingSafeVectorPdfV2Error> {
    display
        .verify_resource_closure()
        .map_err(|_| StagingSafeVectorPdfV2Error::DisplayMismatch)?;
    plans
        .verify_pdf_closure(display, registry, limits)
        .map_err(|_| StagingSafeVectorPdfV2Error::FormPlanMismatch)?;

    let mut inputs = Vec::new();
    inputs
        .try_reserve_exact(
            usize::try_from(display.receipt().command_count())
                .map_err(|_| StagingSafeVectorPdfV2Error::CountOverflow)?,
        )
        .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
    inputs.extend(display.commands().map(PdfUsageInput::from_precomposed));
    build_staging_safe_vector_pdf_contribution_v2_from_inputs(
        display.receipt().fingerprint(),
        u32::try_from(display.pages().len())
            .map_err(|_| StagingSafeVectorPdfV2Error::CountOverflow)?,
        &inputs,
        plans,
        registry,
        limits,
        spool_limit,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_staging_safe_vector_pdf_contribution_v2_from_inputs(
    display_fingerprint: [u8; 32],
    page_count: u32,
    inputs: &[PdfUsageInput],
    plans: &StagingSafeVectorFormPlansV2,
    registry: &VectorContentCandidateRegistry,
    limits: &M4EffectiveResourceLimits,
    spool_limit: u64,
) -> Result<StagingSafeVectorPdfContributionV2, StagingSafeVectorPdfV2Error> {
    if plans.display_fingerprint() != display_fingerprint
        || u32::try_from(inputs.len()).ok() != Some(plans.page_do_count_delta())
    {
        return Err(StagingSafeVectorPdfV2Error::FormPlanMismatch);
    }
    if inputs.iter().any(|input| {
        registry
            .candidate(&input.content_key)
            .map_or(true, |candidate| {
                candidate.canonical_ir().fingerprint() != input.ir_fingerprint
                    || !candidate
                        .aliases()
                        .iter()
                        .any(|alias| alias.image_id() == input.image_id)
            })
    }) {
        return Err(StagingSafeVectorPdfV2Error::CandidateMismatch);
    }

    let mut spool = SpoolBudget::new(spool_limit);
    let mut forms = Vec::new();
    forms
        .try_reserve_exact(plans.plans().len())
        .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
    let ext_capacity = usize::try_from(plans.ext_g_state_object_count_delta())
        .map_err(|_| StagingSafeVectorPdfV2Error::CountOverflow)?;
    let role_capacity = usize::try_from(plans.relative_object_role_count_delta())
        .map_err(|_| StagingSafeVectorPdfV2Error::CountOverflow)?;
    let mut ext_g_states = Vec::new();
    ext_g_states
        .try_reserve_exact(ext_capacity)
        .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
    let mut relative_objects = Vec::new();
    relative_objects
        .try_reserve_exact(role_capacity)
        .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;

    for plan in plans.plans() {
        let ext_start = ext_g_states.len();
        for ext_plan in plan.ext_g_states() {
            let dictionary = encode_ext_g_state_dictionary(ext_plan.alpha_pair(), &mut spool)?;
            let dictionary_fingerprint = sha256(&dictionary);
            ext_g_states.push(StagingSafeVectorPdfExtGStateV2 {
                content_key: *plan.content_key(),
                relative_object_role: ext_plan.relative_object_role(),
                resource_name: ext_plan.resource_name().to_owned(),
                fill_alpha_raw: ext_plan.alpha_pair().fill_alpha_raw(),
                stroke_alpha_raw: ext_plan.alpha_pair().stroke_alpha_raw(),
                dictionary,
                dictionary_fingerprint,
            });
        }
        let form_ext = &ext_g_states[ext_start..];
        let content_stream = encode_form_content_v2(plan, form_ext, &mut spool)?;
        let content_stream_fingerprint = sha256(&content_stream);
        let bbox = [
            0,
            0,
            plan.ir().intrinsic_width().get().raw(),
            plan.ir().intrinsic_height().get().raw(),
        ];
        let mut ext_g_state_roles = Vec::new();
        ext_g_state_roles
            .try_reserve_exact(form_ext.len())
            .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
        ext_g_state_roles.extend(
            form_ext
                .iter()
                .map(|ext| (ext.resource_name.clone(), ext.relative_object_role)),
        );
        let object_contribution_fingerprint = form_object_contribution_fingerprint(
            plan.content_key(),
            plan.form_relative_object_role(),
            plan.form_resource_name(),
            bbox,
            &ext_g_state_roles,
            content_stream_fingerprint,
        );
        forms.push(StagingSafeVectorPdfFormV2 {
            content_key: *plan.content_key(),
            relative_object_role: plan.form_relative_object_role(),
            resource_name: plan.form_resource_name().to_owned(),
            bbox,
            ext_g_state_roles,
            content_stream,
            content_stream_fingerprint,
            object_contribution_fingerprint,
        });
    }

    let mut ext_index = 0usize;
    for form in &forms {
        relative_objects.push(StagingSafeVectorPdfRelativeObjectV2 {
            relative_object_role: form.relative_object_role,
            kind: StagingSafeVectorPdfRelativeObjectKindV2::Form,
            content_key: form.content_key,
            resource_name: form.resource_name.clone(),
            object_contribution_fingerprint: form.object_contribution_fingerprint,
        });
        for (expected_name, expected_role) in &form.ext_g_state_roles {
            let ext = ext_g_states
                .get(ext_index)
                .filter(|ext| {
                    ext.content_key == form.content_key
                        && ext.resource_name == *expected_name
                        && ext.relative_object_role == *expected_role
                })
                .ok_or(StagingSafeVectorPdfV2Error::ContributionMismatch)?;
            relative_objects.push(StagingSafeVectorPdfRelativeObjectV2 {
                relative_object_role: ext.relative_object_role,
                kind: StagingSafeVectorPdfRelativeObjectKindV2::ExtGState,
                content_key: ext.content_key,
                resource_name: ext.resource_name.clone(),
                object_contribution_fingerprint: ext.dictionary_fingerprint,
            });
            ext_index = ext_index
                .checked_add(1)
                .ok_or(StagingSafeVectorPdfV2Error::CountOverflow)?;
        }
    }
    if ext_index != ext_g_states.len() {
        return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
    }
    relative_objects.sort_unstable_by_key(|object| object.relative_object_role);
    validate_relative_objects(&relative_objects, plans.relative_object_role_count_delta())?;

    let usage_capacity = usize::try_from(plans.page_do_count_delta())
        .map_err(|_| StagingSafeVectorPdfV2Error::CountOverflow)?;
    let mut usages = Vec::new();
    usages
        .try_reserve_exact(usage_capacity)
        .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
    for input in inputs {
        let form = form_for_content_key(&input.content_key, &forms)?;
        validate_placement_values(
            input.matrix,
            input.scale,
            input.viewport,
            plans
                .plan(&input.content_key)
                .ok_or(StagingSafeVectorPdfV2Error::FormPlanMismatch)?,
        )?;
        let content = encode_page_usage_values(input.matrix, input.color, form, &mut spool)?;
        let content_fingerprint = sha256(&content);
        usages.push(StagingSafeVectorPdfUsageV2 {
            usage_id: input.usage_id,
            image_id: input.image_id,
            content_key: input.content_key,
            page_index: input.page_index,
            paint_ordinal: input.paint_ordinal,
            form_relative_object_role: form.relative_object_role,
            form_resource_name: form.resource_name.clone(),
            matrix: input.matrix,
            resolved_current_color: input.color,
            content,
            content_fingerprint,
            semantic_hook: StagingSafeVectorPdfSemanticUsageHookV2 {
                usage_id: input.usage_id,
                owner: input.owner,
                kind: input.kind,
                page_index: input.page_index,
                paint_ordinal: input.paint_ordinal,
                display_command_fingerprint: input.display_command_fingerprint,
            },
        });
    }
    if usages.len() != usage_capacity {
        return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
    }

    let pages = build_page_contributions(page_count, inputs, &forms, &usages)?;
    validate_contribution_counts(plans, &forms, &ext_g_states, &pages, &usages)?;
    let spool_bytes = spool.used();
    let canonical_jcs = encode_contribution_receipt(
        display_fingerprint,
        plans.fingerprint(),
        registry.receipt().fingerprint(),
        limits.fingerprint(),
        &relative_objects,
        &forms,
        &ext_g_states,
        &pages,
        &usages,
        spool_bytes,
    );
    Ok(StagingSafeVectorPdfContributionV2 {
        display_fingerprint,
        form_plans_fingerprint: plans.fingerprint(),
        candidate_registry_fingerprint: registry.receipt().fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        relative_objects,
        forms,
        ext_g_states,
        pages,
        usages,
        spool_bytes,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

/// Alias matching the writer-oriented naming used by existing PDF closures.
pub fn write_staging_safe_vector_pdf_contribution_v2(
    display: &StagingPrecomposedVectorDisplay,
    plans: &StagingSafeVectorFormPlansV2,
    registry: &VectorContentCandidateRegistry,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingSafeVectorPdfContributionV2, StagingSafeVectorPdfV2Error> {
    build_staging_safe_vector_pdf_contribution_v2(display, plans, registry, limits)
}

struct SpoolBudget {
    maximum: u64,
    used: u64,
}

impl SpoolBudget {
    const fn new(maximum: u64) -> Self {
        Self { maximum, used: 0 }
    }

    const fn used(&self) -> u64 {
        self.used
    }

    fn consume(&mut self, amount: usize) -> Result<(), StagingSafeVectorPdfV2Error> {
        let amount = u64::try_from(amount).map_err(|_| StagingSafeVectorPdfV2Error::SpoolLimit)?;
        let next = self
            .used
            .checked_add(amount)
            .ok_or(StagingSafeVectorPdfV2Error::SpoolLimit)?;
        if next > self.maximum {
            return Err(StagingSafeVectorPdfV2Error::SpoolLimit);
        }
        self.used = next;
        Ok(())
    }

    fn store(&mut self, value: &[u8]) -> Result<Vec<u8>, StagingSafeVectorPdfV2Error> {
        self.consume(value.len())?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(value.len())
            .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
        output.extend_from_slice(value);
        Ok(output)
    }
}

struct BoundedPdfContent<'a> {
    bytes: Vec<u8>,
    spool: &'a mut SpoolBudget,
}

impl<'a> BoundedPdfContent<'a> {
    fn new(spool: &'a mut SpoolBudget) -> Self {
        Self {
            bytes: Vec::new(),
            spool,
        }
    }

    fn push_str(&mut self, value: &str) -> Result<(), StagingSafeVectorPdfV2Error> {
        self.spool.consume(value.len())?;
        self.bytes
            .try_reserve(value.len())
            .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

fn encode_ext_g_state_dictionary(
    alpha_pair: VectorExtGStateAlphaPair,
    spool: &mut SpoolBudget,
) -> Result<Vec<u8>, StagingSafeVectorPdfV2Error> {
    encode_ext_g_state_dictionary_raw(
        alpha_pair.fill_alpha_raw(),
        alpha_pair.stroke_alpha_raw(),
        spool,
    )
}

fn encode_ext_g_state_dictionary_raw(
    fill_alpha_raw: u32,
    stroke_alpha_raw: u32,
    spool: &mut SpoolBudget,
) -> Result<Vec<u8>, StagingSafeVectorPdfV2Error> {
    if fill_alpha_raw > FIXED_ONE as u32 || stroke_alpha_raw > FIXED_ONE as u32 {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr);
    }
    let dictionary = format!(
        "<< /Type /ExtGState /ca {} /CA {} >>",
        pdf_fixed(i64::from(fill_alpha_raw)),
        pdf_fixed(i64::from(stroke_alpha_raw))
    );
    spool.store(dictionary.as_bytes())
}

fn encode_form_content_v2(
    plan: &FrozenSafeVectorFormPlanV2,
    ext_g_states: &[StagingSafeVectorPdfExtGStateV2],
    spool: &mut SpoolBudget,
) -> Result<Vec<u8>, StagingSafeVectorPdfV2Error> {
    encode_admitted_ir_content(plan.ir(), ext_g_states, spool)
}

fn encode_admitted_ir_content(
    ir: &AdmittedSafeVector,
    ext_g_states: &[StagingSafeVectorPdfExtGStateV2],
    spool: &mut SpoolBudget,
) -> Result<Vec<u8>, StagingSafeVectorPdfV2Error> {
    let mut output = BoundedPdfContent::new(spool);
    output.push_str("q\n0 0 ")?;
    output.push_str(&pdf_fixed(ir.intrinsic_width().get().raw()))?;
    output.push_str(" ")?;
    output.push_str(&pdf_fixed(ir.intrinsic_height().get().raw()))?;
    output.push_str(" re W n\n")?;

    match ir {
        AdmittedSafeVector::V1(ir) => {
            encode_root_view_box(&mut output, ir.view_box(), ir.root_scale_raw())?;
            for draw in ir.draws() {
                encode_v1_draw(&mut output, ir, draw, ext_g_states)?;
            }
        }
        AdmittedSafeVector::V2(ir) => {
            encode_root_view_box(&mut output, ir.view_box(), ir.root_scale_raw())?;
            for draw in ir.draws() {
                encode_v2_draw(&mut output, ir, draw, ext_g_states)?;
            }
        }
    }
    output.push_str("Q")?;
    let bytes = output.finish();
    if contains_forbidden_form_semantics(&bytes) {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr);
    }
    Ok(bytes)
}

fn contains_forbidden_form_semantics(content: &[u8]) -> bool {
    [
        b"/Subtype /Image".as_slice(),
        b"/MCID".as_slice(),
        b"/Alt".as_slice(),
        b"/ActualText".as_slice(),
        b"/Lang".as_slice(),
        b" BDC".as_slice(),
        b" BMC".as_slice(),
    ]
    .iter()
    .any(|needle| {
        content
            .windows(needle.len())
            .any(|window| window == *needle)
    })
}

fn encode_root_view_box(
    output: &mut BoundedPdfContent<'_>,
    view_box: [i64; 4],
    root_scale: i32,
) -> Result<(), StagingSafeVectorPdfV2Error> {
    let [min_x, min_y, width, height] = view_box;
    if root_scale <= 0 || width <= 0 || height <= 0 {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr);
    }
    let scale = i64::from(root_scale);
    let tx = fixed_mul(scale, min_x)?
        .checked_neg()
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    let ty = fixed_mul(scale, min_y)?
        .checked_neg()
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    output.push_str(&format!(
        "{} 0 0 {} {} {} cm\n",
        pdf_fixed(scale),
        pdf_fixed(scale),
        pdf_fixed(tx),
        pdf_fixed(ty)
    ))
}

fn encode_v1_draw(
    output: &mut BoundedPdfContent<'_>,
    ir: &SafeVectorIr,
    draw: &SafeVectorDraw,
    ext_g_states: &[StagingSafeVectorPdfExtGStateV2],
) -> Result<(), StagingSafeVectorPdfV2Error> {
    output.push_str("q\n")?;
    encode_draw_clips(output, ir.clips(), draw.clips())?;
    encode_transform(output, draw.transform())?;
    let ext = ext_g_state_for(ext_g_states, FIXED_ONE as u32, FIXED_ONE as u32)?;
    output.push_str("/")?;
    output.push_str(ext.resource_name())?;
    output.push_str(" gs\n")?;
    if let Some(fill) = draw.fill() {
        encode_rgb_operator(output, fill, "rg")?;
    }
    if let Some(stroke) = draw.stroke() {
        encode_rgb_operator(output, stroke.color(), "RG")?;
        encode_stroke_style(
            output,
            stroke.width_raw(),
            stroke.line_cap(),
            stroke.line_join(),
            stroke.miter_limit_raw(),
        )?;
    }
    encode_path(output, draw.path(), None)?;
    encode_paint_operator(
        output,
        draw.fill().is_some(),
        draw.stroke().is_some(),
        draw.fill_rule(),
    )?;
    output.push_str("Q\n")
}

fn encode_v2_draw(
    output: &mut BoundedPdfContent<'_>,
    ir: &SafeVectorIrV2,
    draw: &SafeVectorDrawV2,
    ext_g_states: &[StagingSafeVectorPdfExtGStateV2],
) -> Result<(), StagingSafeVectorPdfV2Error> {
    output.push_str("q\n")?;
    encode_draw_clips(output, ir.clips(), draw.clips())?;
    encode_transform(output, draw.transform())?;
    let fill = draw.fill();
    let stroke = draw.stroke();
    let ext = ext_g_state_for(
        ext_g_states,
        fill.alpha().raw(),
        stroke.paint().alpha().raw(),
    )?;
    output.push_str("/")?;
    output.push_str(ext.resource_name())?;
    output.push_str(" gs\n")?;
    encode_optional_paint(output, fill.paint(), "rg")?;
    encode_optional_paint(output, stroke.paint().paint(), "RG")?;
    if stroke.paint().paint().enabled() {
        encode_stroke_style(
            output,
            stroke.width_raw(),
            stroke.line_cap(),
            stroke.line_join(),
            stroke.miter_limit_raw(),
        )?;
    }
    encode_path(output, draw.path(), None)?;
    encode_paint_operator(
        output,
        fill.paint().enabled(),
        stroke.paint().paint().enabled(),
        draw.fill_rule(),
    )?;
    output.push_str("Q\n")
}

fn encode_draw_clips(
    output: &mut BoundedPdfContent<'_>,
    definitions: &[SafeVectorClipDefinition],
    uses: &[SafeVectorClipUse],
) -> Result<(), StagingSafeVectorPdfV2Error> {
    for clip_use in uses {
        let definition = definitions
            .get(clip_use.clip_id() as usize)
            .filter(|definition| definition.clip_id() == clip_use.clip_id())
            .ok_or(StagingSafeVectorPdfV2Error::InvalidIr)?;
        encode_path(
            output,
            definition.path(),
            Some((definition.transform(), clip_use.transform())),
        )?;
        output.push_str(match definition.fill_rule() {
            SafeVectorFillRule::NonZero => "W n\n",
            SafeVectorFillRule::EvenOdd => "W* n\n",
        })?;
    }
    Ok(())
}

fn encode_transform(
    output: &mut BoundedPdfContent<'_>,
    transform: SafeVectorTransform,
) -> Result<(), StagingSafeVectorPdfV2Error> {
    if transform.a_raw() == 0 || transform.d_raw() == 0 {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr);
    }
    output.push_str(&format!(
        "{} 0 0 {} {} {} cm\n",
        pdf_fixed(i64::from(transform.a_raw())),
        pdf_fixed(i64::from(transform.d_raw())),
        pdf_fixed(transform.e_raw()),
        pdf_fixed(transform.f_raw())
    ))
}

fn ext_g_state_for(
    ext_g_states: &[StagingSafeVectorPdfExtGStateV2],
    fill_alpha_raw: u32,
    stroke_alpha_raw: u32,
) -> Result<&StagingSafeVectorPdfExtGStateV2, StagingSafeVectorPdfV2Error> {
    ext_g_states
        .iter()
        .find(|entry| {
            entry.fill_alpha_raw == fill_alpha_raw && entry.stroke_alpha_raw == stroke_alpha_raw
        })
        .ok_or(StagingSafeVectorPdfV2Error::FormPlanMismatch)
}

fn encode_optional_paint(
    output: &mut BoundedPdfContent<'_>,
    paint: SafeVectorPaint,
    operator: &str,
) -> Result<(), StagingSafeVectorPdfV2Error> {
    match paint {
        SafeVectorPaint::None | SafeVectorPaint::CurrentColor => Ok(()),
        SafeVectorPaint::FixedRgb8(color) => encode_rgb_operator(output, color, operator),
    }
}

fn encode_rgb_operator(
    output: &mut BoundedPdfContent<'_>,
    color: [u8; 3],
    operator: &str,
) -> Result<(), StagingSafeVectorPdfV2Error> {
    output.push_str(&format!(
        "{} {} {} {}\n",
        pdf_fixed(color_fixed(color[0])?),
        pdf_fixed(color_fixed(color[1])?),
        pdf_fixed(color_fixed(color[2])?),
        operator
    ))
}

fn encode_stroke_style(
    output: &mut BoundedPdfContent<'_>,
    width: i64,
    line_cap: SafeVectorLineCap,
    line_join: SafeVectorLineJoin,
    miter_limit: i64,
) -> Result<(), StagingSafeVectorPdfV2Error> {
    if width <= 0 || miter_limit <= 0 {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr);
    }
    output.push_str(&format!(
        "{} w\n{} J\n{} j\n{} M\n",
        pdf_fixed(width),
        match line_cap {
            SafeVectorLineCap::Butt => 0,
            SafeVectorLineCap::Round => 1,
            SafeVectorLineCap::Square => 2,
        },
        match line_join {
            SafeVectorLineJoin::Miter => 0,
            SafeVectorLineJoin::Round => 1,
            SafeVectorLineJoin::Bevel => 2,
        },
        pdf_fixed(miter_limit)
    ))
}

fn encode_paint_operator(
    output: &mut BoundedPdfContent<'_>,
    fill: bool,
    stroke: bool,
    fill_rule: SafeVectorFillRule,
) -> Result<(), StagingSafeVectorPdfV2Error> {
    output.push_str(match (fill, stroke, fill_rule) {
        (true, true, SafeVectorFillRule::NonZero) => "B\n",
        (true, true, SafeVectorFillRule::EvenOdd) => "B*\n",
        (true, false, SafeVectorFillRule::NonZero) => "f\n",
        (true, false, SafeVectorFillRule::EvenOdd) => "f*\n",
        (false, true, _) => "S\n",
        (false, false, _) => return Err(StagingSafeVectorPdfV2Error::InvalidIr),
    })
}

fn encode_path(
    output: &mut BoundedPdfContent<'_>,
    path: &SafeVectorPath,
    transform: Option<(SafeVectorTransform, SafeVectorTransform)>,
) -> Result<(), StagingSafeVectorPdfV2Error> {
    let mut current = None;
    let mut subpath = None;
    for segment in path.segments() {
        match segment {
            SafeVectorSegment::Move(point) => {
                let point = maybe_transform(*point, transform)?;
                output.push_str(&format!(
                    "{} {} m\n",
                    pdf_fixed(point.x),
                    pdf_fixed(point.y)
                ))?;
                current = Some(point);
                subpath = Some(point);
            }
            SafeVectorSegment::Line(point) => {
                let point = maybe_transform(*point, transform)?;
                output.push_str(&format!(
                    "{} {} l\n",
                    pdf_fixed(point.x),
                    pdf_fixed(point.y)
                ))?;
                current = Some(point);
            }
            SafeVectorSegment::Quadratic(control, endpoint) => {
                let start = current.ok_or(StagingSafeVectorPdfV2Error::InvalidIr)?;
                let control = maybe_transform(*control, transform)?;
                let endpoint = maybe_transform(*endpoint, transform)?;
                let first = RawPoint {
                    x: rational_third(start.x, control.x)?,
                    y: rational_third(start.y, control.y)?,
                };
                let second = RawPoint {
                    x: rational_third(endpoint.x, control.x)?,
                    y: rational_third(endpoint.y, control.y)?,
                };
                output.push_str(&format!(
                    "{} {} {} {} {} {} c\n",
                    pdf_fixed(first.x),
                    pdf_fixed(first.y),
                    pdf_fixed(second.x),
                    pdf_fixed(second.y),
                    pdf_fixed(endpoint.x),
                    pdf_fixed(endpoint.y)
                ))?;
                current = Some(endpoint);
            }
            SafeVectorSegment::Cubic(first, second, endpoint) => {
                let first = maybe_transform(*first, transform)?;
                let second = maybe_transform(*second, transform)?;
                let endpoint = maybe_transform(*endpoint, transform)?;
                output.push_str(&format!(
                    "{} {} {} {} {} {} c\n",
                    pdf_fixed(first.x),
                    pdf_fixed(first.y),
                    pdf_fixed(second.x),
                    pdf_fixed(second.y),
                    pdf_fixed(endpoint.x),
                    pdf_fixed(endpoint.y)
                ))?;
                current = Some(endpoint);
            }
            SafeVectorSegment::Close => {
                output.push_str("h\n")?;
                current = subpath;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct RawPoint {
    x: i64,
    y: i64,
}

fn maybe_transform(
    point: SafeVectorPoint,
    transforms: Option<(SafeVectorTransform, SafeVectorTransform)>,
) -> Result<RawPoint, StagingSafeVectorPdfV2Error> {
    let point = RawPoint {
        x: point.x_raw(),
        y: point.y_raw(),
    };
    let Some((definition, use_site)) = transforms else {
        return Ok(point);
    };
    let transform = compose_fixed_transform(raw_transform(use_site), raw_transform(definition))?;
    apply_fixed_transform(point, transform)
}

const fn raw_transform(transform: SafeVectorTransform) -> [i64; 4] {
    [
        transform.a_raw() as i64,
        transform.d_raw() as i64,
        transform.e_raw(),
        transform.f_raw(),
    ]
}

fn compose_fixed_transform(
    left: [i64; 4],
    right: [i64; 4],
) -> Result<[i64; 4], StagingSafeVectorPdfV2Error> {
    let a = fixed_mul(left[0], right[0])?;
    let d = fixed_mul(left[1], right[1])?;
    let e = fixed_mul(left[0], right[2])?
        .checked_add(left[2])
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    let f = fixed_mul(left[1], right[3])?
        .checked_add(left[3])
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    if a == 0
        || d == 0
        || i32::try_from(a).is_err()
        || i32::try_from(d).is_err()
        || e.abs() > MAX_COORDINATE
        || f.abs() > MAX_COORDINATE
    {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr);
    }
    Ok([a, d, e, f])
}

fn apply_fixed_transform(
    point: RawPoint,
    transform: [i64; 4],
) -> Result<RawPoint, StagingSafeVectorPdfV2Error> {
    let x = fixed_mul(transform[0], point.x)?
        .checked_add(transform[2])
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    let y = fixed_mul(transform[1], point.y)?
        .checked_add(transform[3])
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    if x.abs() > MAX_COORDINATE || y.abs() > MAX_COORDINATE {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr);
    }
    Ok(RawPoint { x, y })
}

fn rational_third(endpoint: i64, control: i64) -> Result<i64, StagingSafeVectorPdfV2Error> {
    let numerator = i128::from(endpoint)
        .checked_add(
            i128::from(control)
                .checked_mul(2)
                .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?,
        )
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    i64::try_from(round_ties_even(numerator, 3)?)
        .map_err(|_| StagingSafeVectorPdfV2Error::ArithmeticOverflow)
}

fn fixed_mul(left: i64, right: i64) -> Result<i64, StagingSafeVectorPdfV2Error> {
    let numerator = i128::from(left)
        .checked_mul(i128::from(right))
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    i64::try_from(round_ties_even(numerator, i128::from(FIXED_ONE))?)
        .map_err(|_| StagingSafeVectorPdfV2Error::ArithmeticOverflow)
}

fn color_fixed(byte: u8) -> Result<i64, StagingSafeVectorPdfV2Error> {
    i64::try_from(round_ties_even(
        i128::from(byte) * i128::from(FIXED_ONE),
        255,
    )?)
    .map_err(|_| StagingSafeVectorPdfV2Error::ArithmeticOverflow)
}

fn round_ties_even(
    numerator: i128,
    denominator: i128,
) -> Result<i128, StagingSafeVectorPdfV2Error> {
    if denominator <= 0 {
        return Err(StagingSafeVectorPdfV2Error::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        return Ok(quotient);
    }
    let twice = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    let denominator = denominator as u128;
    if twice < denominator || (twice == denominator && quotient % 2 == 0) {
        Ok(quotient)
    } else {
        quotient
            .checked_add(if remainder > 0 { 1 } else { -1 })
            .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)
    }
}

fn pdf_fixed(raw: i64) -> String {
    const DECIMAL_SCALE: u64 = 10_000_000_000_000_000;
    const BINARY_TO_DECIMAL: u64 = 152_587_890_625;
    let negative = raw < 0;
    let magnitude = raw.unsigned_abs();
    let integer = magnitude / FIXED_ONE as u64;
    let fraction = magnitude % FIXED_ONE as u64;
    if fraction == 0 {
        return if negative {
            format!("-{integer}")
        } else {
            integer.to_string()
        };
    }
    let decimal = fraction * BINARY_TO_DECIMAL;
    debug_assert!(decimal < DECIMAL_SCALE);
    let mut fraction_text = format!("{decimal:016}");
    while fraction_text.ends_with('0') {
        fraction_text.pop();
    }
    if negative {
        format!("-{integer}.{fraction_text}")
    } else if integer == 0 {
        format!("0.{fraction_text}")
    } else {
        format!("{integer}.{fraction_text}")
    }
}

fn form_for_content_key<'a>(
    content_key: &VectorContentKey,
    forms: &'a [StagingSafeVectorPdfFormV2],
) -> Result<&'a StagingSafeVectorPdfFormV2, StagingSafeVectorPdfV2Error> {
    forms
        .binary_search_by(|form| form.content_key.cmp(content_key))
        .ok()
        .and_then(|index| forms.get(index))
        .ok_or(StagingSafeVectorPdfV2Error::FormPlanMismatch)
}

fn validate_placement_values(
    matrix: AffineTransform,
    scale: i32,
    viewport: typaxis_core::Rect,
    plan: &FrozenSafeVectorFormPlanV2,
) -> Result<(), StagingSafeVectorPdfV2Error> {
    if scale <= 0
        || matrix.a.raw() != scale
        || matrix.b.raw() != 0
        || matrix.c.raw() != 0
        || matrix.d.raw() != scale
        || matrix.e != viewport.x()
        || matrix.f != viewport.y()
        || fixed_mul(i64::from(scale), plan.ir().intrinsic_width().get().raw())?
            != viewport.width().get().raw()
        || fixed_mul(i64::from(scale), plan.ir().intrinsic_height().get().raw())?
            != viewport.height().get().raw()
    {
        return Err(StagingSafeVectorPdfV2Error::InvalidPlacement);
    }
    Ok(())
}

fn encode_page_usage_values(
    matrix: AffineTransform,
    color: [u8; 3],
    form: &StagingSafeVectorPdfFormV2,
    spool: &mut SpoolBudget,
) -> Result<Vec<u8>, StagingSafeVectorPdfV2Error> {
    let mut output = BoundedPdfContent::new(spool);
    output.push_str("q\n")?;
    encode_rgb_operator(&mut output, color, "rg")?;
    encode_rgb_operator(&mut output, color, "RG")?;
    output.push_str(&format!(
        "{} {} {} {} {} {} cm\n",
        pdf_fixed(i64::from(matrix.a.raw())),
        pdf_fixed(i64::from(matrix.b.raw())),
        pdf_fixed(i64::from(matrix.c.raw())),
        pdf_fixed(i64::from(matrix.d.raw())),
        pdf_fixed(matrix.e.raw()),
        pdf_fixed(matrix.f.raw())
    ))?;
    output.push_str("/")?;
    output.push_str(&form.resource_name)?;
    output.push_str(" Do\nQ")?;
    Ok(output.finish())
}

fn build_page_contributions(
    page_count: u32,
    inputs: &[PdfUsageInput],
    forms: &[StagingSafeVectorPdfFormV2],
    usages: &[StagingSafeVectorPdfUsageV2],
) -> Result<Vec<StagingSafeVectorPdfPageV2>, StagingSafeVectorPdfV2Error> {
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(
            usize::try_from(page_count).map_err(|_| StagingSafeVectorPdfV2Error::CountOverflow)?,
        )
        .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
    for page_index in 0..page_count {
        let page_usage_count = inputs
            .iter()
            .filter(|input| input.page_index == page_index)
            .count();
        let mut page_inputs: Vec<&PdfUsageInput> = Vec::new();
        page_inputs
            .try_reserve_exact(page_usage_count)
            .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
        page_inputs.extend(inputs.iter().filter(|input| input.page_index == page_index));
        page_inputs.sort_unstable_by_key(|input| input.paint_ordinal);
        let mut unique = BTreeMap::new();
        let mut usage_ids = Vec::new();
        usage_ids
            .try_reserve_exact(page_inputs.len())
            .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
        for input in page_inputs {
            let form = form_for_content_key(&input.content_key, forms)?;
            if usize::try_from(input.usage_id)
                .ok()
                .and_then(|index| usages.get(index))
                .map_or(true, |usage| {
                    usage.usage_id != input.usage_id || usage.page_index != page_index
                })
            {
                return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
            }
            usage_ids.push(input.usage_id);
            match unique.insert(
                input.content_key,
                (form.relative_object_role, form.resource_name.clone()),
            ) {
                Some(previous)
                    if previous != (form.relative_object_role, form.resource_name.clone()) =>
                {
                    return Err(StagingSafeVectorPdfV2Error::ContributionMismatch)
                }
                _ => {}
            }
        }
        let mut resources = Vec::new();
        resources
            .try_reserve_exact(unique.len())
            .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
        resources.extend(unique.into_iter().map(
            |(content_key, (form_relative_object_role, resource_name))| {
                StagingSafeVectorPdfPageResourceV2 {
                    page_index,
                    content_key,
                    form_relative_object_role,
                    resource_name,
                }
            },
        ));
        let fingerprint =
            sha256(encode_page_contribution(page_index, &resources, &usage_ids).as_bytes());
        pages.push(StagingSafeVectorPdfPageV2 {
            page_index,
            resources,
            usage_ids,
            requires_existing_top_left_page_root_y_flip: true,
            fingerprint,
        });
    }
    Ok(pages)
}

fn validate_relative_objects(
    objects: &[StagingSafeVectorPdfRelativeObjectV2],
    expected_count: u32,
) -> Result<(), StagingSafeVectorPdfV2Error> {
    if u32::try_from(objects.len()).ok() != Some(expected_count) {
        return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
    }
    for (index, object) in objects.iter().enumerate() {
        if u32::try_from(index).ok() != Some(object.relative_object_role)
            || object.resource_name.is_empty()
            || object.object_contribution_fingerprint == [0; 32]
        {
            return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
        }
    }
    Ok(())
}

fn validate_contribution_counts(
    plans: &StagingSafeVectorFormPlansV2,
    forms: &[StagingSafeVectorPdfFormV2],
    ext_g_states: &[StagingSafeVectorPdfExtGStateV2],
    pages: &[StagingSafeVectorPdfPageV2],
    usages: &[StagingSafeVectorPdfUsageV2],
) -> Result<(), StagingSafeVectorPdfV2Error> {
    let page_binding_count = pages.iter().try_fold(0usize, |total, page| {
        total.checked_add(page.resources.len())
    });
    if u32::try_from(forms.len()).ok() != Some(plans.form_object_count_delta())
        || u32::try_from(ext_g_states.len()).ok() != Some(plans.ext_g_state_object_count_delta())
        || page_binding_count.and_then(|count| u32::try_from(count).ok())
            != Some(plans.page_resource_binding_count_delta())
        || u32::try_from(usages.len()).ok() != Some(plans.page_do_count_delta())
    {
        return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
    }
    let mut keys = BTreeSet::new();
    for form in forms {
        if !keys.insert(form.content_key) {
            return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
        }
    }
    for (index, usage) in usages.iter().enumerate() {
        if u32::try_from(index).ok() != Some(usage.usage_id)
            || forms
                .binary_search_by(|form| form.content_key.cmp(&usage.content_key))
                .ok()
                .and_then(|form_index| forms.get(form_index))
                .map_or(true, |form| {
                    form.relative_object_role != usage.form_relative_object_role
                        || form.resource_name != usage.form_resource_name
                })
        {
            return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
        }
    }
    Ok(())
}

fn form_object_contribution_fingerprint(
    content_key: &VectorContentKey,
    relative_object_role: u32,
    resource_name: &str,
    bbox: [i64; 4],
    ext_g_state_roles: &[(String, u32)],
    content_stream_fingerprint: [u8; 32],
) -> [u8; 32] {
    let mut output = String::from("{\"bbox\":[");
    push_i64_array(&mut output, &bbox);
    output.push_str("],\"content_key\":");
    push_content_key(&mut output, content_key);
    output.push_str(",\"content_stream_fingerprint\":");
    push_hash(&mut output, content_stream_fingerprint);
    output.push_str(",\"ext_g_state_roles\":[");
    for (index, (name, role)) in ext_g_state_roles.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"relative_object_role\":");
        output.push_str(&role.to_string());
        output.push_str(",\"resource_name\":");
        push_jcs_string(&mut output, name);
        output.push('}');
    }
    output.push_str("],\"relative_object_role\":");
    output.push_str(&relative_object_role.to_string());
    output.push_str(",\"resource_name\":");
    push_jcs_string(&mut output, resource_name);
    output.push('}');
    sha256(output.as_bytes())
}

fn encode_page_contribution(
    page_index: u32,
    resources: &[StagingSafeVectorPdfPageResourceV2],
    usage_ids: &[u32],
) -> String {
    let mut output = String::from(
        "{\"coordinate_policy\":\"existing-single-top-left-page-root-y-flip\",\"page_index\":",
    );
    output.push_str(&page_index.to_string());
    output.push_str(",\"resources\":[");
    for (index, resource) in resources.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"content_key\":");
        push_content_key(&mut output, &resource.content_key);
        output.push_str(",\"form_relative_object_role\":");
        output.push_str(&resource.form_relative_object_role.to_string());
        output.push_str(",\"resource_name\":");
        push_jcs_string(&mut output, &resource.resource_name);
        output.push('}');
    }
    output.push_str("],\"usage_ids\":[");
    for (index, usage_id) in usage_ids.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&usage_id.to_string());
    }
    output.push_str("]}");
    output
}

#[allow(clippy::too_many_arguments)]
fn encode_contribution_receipt(
    display_fingerprint: [u8; 32],
    form_plans_fingerprint: [u8; 32],
    registry_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    relative_objects: &[StagingSafeVectorPdfRelativeObjectV2],
    forms: &[StagingSafeVectorPdfFormV2],
    ext_g_states: &[StagingSafeVectorPdfExtGStateV2],
    pages: &[StagingSafeVectorPdfPageV2],
    usages: &[StagingSafeVectorPdfUsageV2],
    spool_bytes: u64,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(
        &mut output,
        STAGING_SAFE_VECTOR_PDF_CONTRIBUTION_V2_ALGORITHM,
    );
    output.push_str(",\"candidate_registry_fingerprint\":");
    push_hash(&mut output, registry_fingerprint);
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display_fingerprint);
    output.push_str(",\"ext_g_states\":[");
    for (index, ext) in ext_g_states.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"content_key\":");
        push_content_key(&mut output, &ext.content_key);
        output.push_str(",\"dictionary_fingerprint\":");
        push_hash(&mut output, ext.dictionary_fingerprint);
        output.push_str(",\"fill_alpha\":");
        output.push_str(&ext.fill_alpha_raw.to_string());
        output.push_str(",\"relative_object_role\":");
        output.push_str(&ext.relative_object_role.to_string());
        output.push_str(",\"resource_name\":");
        push_jcs_string(&mut output, &ext.resource_name);
        output.push_str(",\"stroke_alpha\":");
        output.push_str(&ext.stroke_alpha_raw.to_string());
        output.push('}');
    }
    output.push_str("],\"form_plans_fingerprint\":");
    push_hash(&mut output, form_plans_fingerprint);
    output.push_str(",\"forms\":[");
    for (index, form) in forms.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"bbox\":[");
        push_i64_array(&mut output, &form.bbox);
        output.push_str("],\"content_key\":");
        push_content_key(&mut output, &form.content_key);
        output.push_str(",\"content_stream_fingerprint\":");
        push_hash(&mut output, form.content_stream_fingerprint);
        output.push_str(",\"object_contribution_fingerprint\":");
        push_hash(&mut output, form.object_contribution_fingerprint);
        output.push_str(",\"relative_object_role\":");
        output.push_str(&form.relative_object_role.to_string());
        output.push_str(",\"resource_name\":");
        push_jcs_string(&mut output, &form.resource_name);
        output.push('}');
    }
    output.push_str("],\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    output.push_str(",\"pages\":[");
    for (index, page) in pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"fingerprint\":");
        push_hash(&mut output, page.fingerprint);
        output.push_str(",\"record\":");
        output.push_str(&encode_page_contribution(
            page.page_index,
            &page.resources,
            &page.usage_ids,
        ));
        output.push('}');
    }
    output.push_str("],\"relative_objects\":[");
    for (index, object) in relative_objects.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"content_key\":");
        push_content_key(&mut output, &object.content_key);
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, object.kind.as_str());
        output.push_str(",\"object_contribution_fingerprint\":");
        push_hash(&mut output, object.object_contribution_fingerprint);
        output.push_str(",\"relative_object_role\":");
        output.push_str(&object.relative_object_role.to_string());
        output.push_str(",\"resource_name\":");
        push_jcs_string(&mut output, &object.resource_name);
        output.push('}');
    }
    output.push_str("],\"spool_bytes\":");
    output.push_str(&spool_bytes.to_string());
    output.push_str(",\"usages\":[");
    for (index, usage) in usages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"content_fingerprint\":");
        push_hash(&mut output, usage.content_fingerprint);
        output.push_str(",\"content_key\":");
        push_content_key(&mut output, &usage.content_key);
        output.push_str(",\"display_command_fingerprint\":");
        push_hash(&mut output, usage.semantic_hook.display_command_fingerprint);
        output.push_str(",\"form_relative_object_role\":");
        output.push_str(&usage.form_relative_object_role.to_string());
        output.push_str(",\"form_resource_name\":");
        push_jcs_string(&mut output, &usage.form_resource_name);
        output.push_str(",\"image_id\":");
        output.push_str(&usage.image_id.get().to_string());
        output.push_str(",\"matrix\":");
        push_matrix(&mut output, usage.matrix);
        output.push_str(",\"owner\":");
        output.push_str(&usage.semantic_hook.owner.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&usage.page_index.to_string());
        output.push_str(",\"paint_ordinal\":");
        output.push_str(&usage.paint_ordinal.to_string());
        output.push_str(",\"resolved_current_color\":[");
        output.push_str(&usage.resolved_current_color[0].to_string());
        output.push(',');
        output.push_str(&usage.resolved_current_color[1].to_string());
        output.push(',');
        output.push_str(&usage.resolved_current_color[2].to_string());
        output.push_str("],\"semantic_kind\":");
        push_jcs_string(&mut output, usage.semantic_hook.kind.as_str());
        output.push_str(",\"usage_id\":");
        output.push_str(&usage.usage_id.to_string());
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn push_matrix(output: &mut String, matrix: AffineTransform) {
    output.push_str("{\"a_16_16\":");
    output.push_str(&matrix.a.raw().to_string());
    output.push_str(",\"b_16_16\":");
    output.push_str(&matrix.b.raw().to_string());
    output.push_str(",\"c_16_16\":");
    output.push_str(&matrix.c.raw().to_string());
    output.push_str(",\"d_16_16\":");
    output.push_str(&matrix.d.raw().to_string());
    output.push_str(",\"e\":");
    output.push_str(&matrix.e.raw().to_string());
    output.push_str(",\"f\":");
    output.push_str(&matrix.f.raw().to_string());
    output.push('}');
}

fn push_content_key(output: &mut String, key: &VectorContentKey) {
    output.push_str("{\"ir_fingerprint\":");
    push_hash(output, key.ir_fingerprint());
    output.push_str(",\"ir_id\":");
    push_jcs_string(output, key.ir_id());
    output.push_str(",\"media_type\":");
    push_jcs_string(output, key.media_type().as_str());
    output.push_str(",\"parser_id\":");
    push_jcs_string(output, key.parser_id());
    output.push_str(",\"source_sha256\":");
    push_hash(output, key.source_sha256());
    output.push('}');
}

fn push_i64_array(output: &mut String, values: &[i64]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&value.to_string());
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

/// Final graph assignment observed by the complete writer. This type observes
/// allocation; it never allocates an absolute object number itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfFinalObjectObservationV2 {
    relative_object_role: u32,
    absolute_object_number: u32,
    object_contribution_fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfFinalObjectObservationV2 {
    pub const fn from_final_writer(
        relative_object_role: u32,
        absolute_object_number: u32,
        object_contribution_fingerprint: [u8; 32],
    ) -> Self {
        Self {
            relative_object_role,
            absolute_object_number,
            object_contribution_fingerprint,
        }
    }

    pub const fn relative_object_role(self) -> u32 {
        self.relative_object_role
    }

    pub const fn absolute_object_number(self) -> u32 {
        self.absolute_object_number
    }

    pub const fn object_contribution_fingerprint(self) -> [u8; 32] {
        self.object_contribution_fingerprint
    }
}

/// Confirmation that the final page stream consumed one exact page-level
/// vector usage contribution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfFinalUsageObservationV2 {
    usage_id: u32,
    page_index: u32,
    paint_ordinal: u32,
    page_object_number: u32,
    page_content_object_number: u32,
    form_absolute_object_number: u32,
    content_fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfFinalUsageObservationV2 {
    pub const fn from_final_writer(
        usage_id: u32,
        page_index: u32,
        paint_ordinal: u32,
        page_object_number: u32,
        page_content_object_number: u32,
        form_absolute_object_number: u32,
        content_fingerprint: [u8; 32],
    ) -> Self {
        Self {
            usage_id,
            page_index,
            paint_ordinal,
            page_object_number,
            page_content_object_number,
            form_absolute_object_number,
            content_fingerprint,
        }
    }

    pub const fn usage_id(self) -> u32 {
        self.usage_id
    }

    pub const fn page_index(self) -> u32 {
        self.page_index
    }

    pub const fn paint_ordinal(self) -> u32 {
        self.paint_ordinal
    }

    pub const fn page_object_number(self) -> u32 {
        self.page_object_number
    }

    pub const fn page_content_object_number(self) -> u32 {
        self.page_content_object_number
    }

    pub const fn form_absolute_object_number(self) -> u32 {
        self.form_absolute_object_number
    }

    pub const fn content_fingerprint(self) -> [u8; 32] {
        self.content_fingerprint
    }
}

/// Sealed facts supplied by the complete final writer after it has merged the
/// vector contribution into the whole PDF graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfFinalWriterObservationV2 {
    contribution_fingerprint: [u8; 32],
    object_table: Vec<StagingSafeVectorPdfFinalObjectObservationV2>,
    usages: Vec<StagingSafeVectorPdfFinalUsageObservationV2>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfFinalWriterObservationV2 {
    pub fn from_final_writer(
        contribution: &StagingSafeVectorPdfContributionV2,
        object_table: Vec<StagingSafeVectorPdfFinalObjectObservationV2>,
        usages: Vec<StagingSafeVectorPdfFinalUsageObservationV2>,
    ) -> Result<Self, StagingSafeVectorPdfV2Error> {
        validate_final_writer_rows(contribution, &object_table, &usages)?;
        let canonical_jcs =
            encode_final_writer_observation(contribution.fingerprint(), &object_table, &usages);
        Ok(Self {
            contribution_fingerprint: contribution.fingerprint(),
            object_table,
            usages,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn contribution_fingerprint(&self) -> [u8; 32] {
        self.contribution_fingerprint
    }

    pub fn object_table(&self) -> &[StagingSafeVectorPdfFinalObjectObservationV2] {
        &self.object_table
    }

    pub fn usages(&self) -> &[StagingSafeVectorPdfFinalUsageObservationV2] {
        &self.usages
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    fn verify(
        &self,
        contribution: &StagingSafeVectorPdfContributionV2,
    ) -> Result<(), StagingSafeVectorPdfV2Error> {
        validate_final_writer_rows(contribution, &self.object_table, &self.usages)?;
        let canonical = encode_final_writer_observation(
            contribution.fingerprint(),
            &self.object_table,
            &self.usages,
        );
        if self.contribution_fingerprint != contribution.fingerprint()
            || self.canonical_jcs != canonical
            || self.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfClosureV2 {
    contribution_fingerprint: [u8; 32],
    final_writer_observation_fingerprint: [u8; 32],
    final_pdf_sha256: [u8; 32],
    final_pdf_byte_length: u64,
    final_pdf_object_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfClosureV2 {
    pub const fn algorithm(&self) -> &'static str {
        STAGING_SAFE_VECTOR_PDF_ALGORITHM_V2
    }

    pub const fn contribution_fingerprint(&self) -> [u8; 32] {
        self.contribution_fingerprint
    }

    pub const fn final_writer_observation_fingerprint(&self) -> [u8; 32] {
        self.final_writer_observation_fingerprint
    }

    pub const fn final_pdf_sha256(&self) -> [u8; 32] {
        self.final_pdf_sha256
    }

    pub const fn final_pdf_byte_length(&self) -> u64 {
        self.final_pdf_byte_length
    }

    pub const fn final_pdf_object_count(&self) -> u32 {
        self.final_pdf_object_count
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// Seals `typaxis.safe-vector-pdf-closure/2`. A reusable contribution alone is
/// insufficient: this API requires bytes/hash facts issued by the complete
/// serializer and the final graph's exact object/use observations.
pub fn seal_staging_safe_vector_pdf_v2(
    contribution: &StagingSafeVectorPdfContributionV2,
    final_writer: &StagingSafeVectorPdfFinalWriterObservationV2,
    final_pdf: &VerifiedPdfBytesReceipt,
) -> Result<StagingSafeVectorPdfClosureV2, StagingSafeVectorPdfV2Error> {
    final_writer.verify(contribution)?;
    if final_pdf.bytes().is_empty()
        || final_pdf.content_hash() != sha256(final_pdf.bytes())
        || u64::try_from(final_pdf.bytes().len()).ok() != Some(final_pdf.byte_length())
        || usize::try_from(final_pdf.page_count()).ok() != Some(contribution.pages.len())
        || final_writer.object_table.iter().any(|object| {
            object.absolute_object_number == 0
                || object.absolute_object_number > final_pdf.object_count()
        })
        || final_writer.usages.iter().any(|usage| {
            usage.page_object_number == 0
                || usage.page_content_object_number == 0
                || usage.page_object_number > final_pdf.object_count()
                || usage.page_content_object_number > final_pdf.object_count()
        })
    {
        return Err(StagingSafeVectorPdfV2Error::FinalPdfMismatch);
    }
    let canonical_jcs = encode_pdf_closure(
        contribution.fingerprint(),
        final_writer.fingerprint(),
        final_pdf.content_hash(),
        final_pdf.byte_length(),
        final_pdf.object_count(),
    );
    Ok(StagingSafeVectorPdfClosureV2 {
        contribution_fingerprint: contribution.fingerprint(),
        final_writer_observation_fingerprint: final_writer.fingerprint(),
        final_pdf_sha256: final_pdf.content_hash(),
        final_pdf_byte_length: final_pdf.byte_length(),
        final_pdf_object_count: final_pdf.object_count(),
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn validate_final_writer_rows(
    contribution: &StagingSafeVectorPdfContributionV2,
    object_table: &[StagingSafeVectorPdfFinalObjectObservationV2],
    usages: &[StagingSafeVectorPdfFinalUsageObservationV2],
) -> Result<(), StagingSafeVectorPdfV2Error> {
    if object_table.len() != contribution.relative_objects.len()
        || usages.len() != contribution.usages.len()
    {
        return Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch);
    }
    let mut absolute_numbers = BTreeSet::new();
    let mut absolute_by_relative_role = BTreeMap::new();
    for (expected, observed) in contribution.relative_objects.iter().zip(object_table) {
        if observed.relative_object_role != expected.relative_object_role
            || observed.absolute_object_number == 0
            || observed.object_contribution_fingerprint != expected.object_contribution_fingerprint
            || !absolute_numbers.insert(observed.absolute_object_number)
            || absolute_by_relative_role
                .insert(
                    observed.relative_object_role,
                    observed.absolute_object_number,
                )
                .is_some()
        {
            return Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch);
        }
    }
    let mut page_objects = BTreeMap::new();
    let mut observed_object_numbers = absolute_numbers;
    for (expected, observed) in contribution.usages.iter().zip(usages) {
        let expected_form_object = absolute_by_relative_role
            .get(&expected.form_relative_object_role)
            .copied()
            .ok_or(StagingSafeVectorPdfV2Error::FinalWriterMismatch)?;
        if observed.usage_id != expected.usage_id
            || observed.page_index != expected.page_index
            || observed.paint_ordinal != expected.paint_ordinal
            || observed.page_object_number == 0
            || observed.page_content_object_number == 0
            || observed.page_object_number == observed.page_content_object_number
            || observed.form_absolute_object_number != expected_form_object
            || observed.content_fingerprint != expected.content_fingerprint
        {
            return Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch);
        }
        let pair = (
            observed.page_object_number,
            observed.page_content_object_number,
        );
        match page_objects.entry(observed.page_index) {
            std::collections::btree_map::Entry::Occupied(entry) => {
                if *entry.get() != pair {
                    return Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch);
                }
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                if !observed_object_numbers.insert(observed.page_object_number)
                    || !observed_object_numbers.insert(observed.page_content_object_number)
                {
                    return Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch);
                }
                entry.insert(pair);
            }
        }
    }
    Ok(())
}

fn encode_final_writer_observation(
    contribution_fingerprint: [u8; 32],
    object_table: &[StagingSafeVectorPdfFinalObjectObservationV2],
    usages: &[StagingSafeVectorPdfFinalUsageObservationV2],
) -> String {
    let mut output = String::from("{\"contribution_fingerprint\":");
    push_hash(&mut output, contribution_fingerprint);
    output.push_str(",\"object_table\":[");
    for (index, object) in object_table.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"absolute_object_number\":");
        output.push_str(&object.absolute_object_number.to_string());
        output.push_str(",\"object_contribution_fingerprint\":");
        push_hash(&mut output, object.object_contribution_fingerprint);
        output.push_str(",\"relative_object_role\":");
        output.push_str(&object.relative_object_role.to_string());
        output.push('}');
    }
    output.push_str("],\"usages\":[");
    for (index, usage) in usages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"content_fingerprint\":");
        push_hash(&mut output, usage.content_fingerprint);
        output.push_str(",\"form_absolute_object_number\":");
        output.push_str(&usage.form_absolute_object_number.to_string());
        output.push_str(",\"page_content_object_number\":");
        output.push_str(&usage.page_content_object_number.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&usage.page_index.to_string());
        output.push_str(",\"page_object_number\":");
        output.push_str(&usage.page_object_number.to_string());
        output.push_str(",\"paint_ordinal\":");
        output.push_str(&usage.paint_ordinal.to_string());
        output.push_str(",\"usage_id\":");
        output.push_str(&usage.usage_id.to_string());
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn encode_pdf_closure(
    contribution_fingerprint: [u8; 32],
    final_writer_fingerprint: [u8; 32],
    final_pdf_sha256: [u8; 32],
    final_pdf_byte_length: u64,
    final_pdf_object_count: u32,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_SAFE_VECTOR_PDF_ALGORITHM_V2);
    output.push_str(",\"contribution_fingerprint\":");
    push_hash(&mut output, contribution_fingerprint);
    output.push_str(",\"final_pdf_byte_length\":");
    output.push_str(&final_pdf_byte_length.to_string());
    output.push_str(",\"final_pdf_object_count\":");
    output.push_str(&final_pdf_object_count.to_string());
    output.push_str(",\"final_pdf_sha256\":");
    push_hash(&mut output, final_pdf_sha256);
    output.push_str(",\"final_writer_observation_fingerprint\":");
    push_hash(&mut output, final_writer_fingerprint);
    output.push('}');
    output
}

/// Complete assertion-only PDF built from one reusable contribution. This
/// fixture never acts as a production `/2` receipt; it exists so an independent
/// crate can inspect the serialized Form/object/page boundary.
#[cfg(any(test, feature = "staging-fixtures"))]
pub struct StagingSafeVectorIsolatedPdfFixtureV2 {
    bytes: Vec<u8>,
    object_table: Vec<StagingSafeVectorPdfFinalObjectObservationV2>,
    usages: Vec<StagingSafeVectorPdfFinalUsageObservationV2>,
    object_count: u32,
}

#[cfg(any(test, feature = "staging-fixtures"))]
impl StagingSafeVectorIsolatedPdfFixtureV2 {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn object_table(&self) -> &[StagingSafeVectorPdfFinalObjectObservationV2] {
        &self.object_table
    }

    pub fn usages(&self) -> &[StagingSafeVectorPdfFinalUsageObservationV2] {
        &self.usages
    }

    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_safe_vector_isolated_pdf_fixture_v2(
    contribution: &StagingSafeVectorPdfContributionV2,
    page_width: i64,
    page_height: i64,
) -> Result<StagingSafeVectorIsolatedPdfFixtureV2, StagingSafeVectorPdfV2Error> {
    if page_width <= 0 || page_height <= 0 || contribution.pages.is_empty() {
        return Err(StagingSafeVectorPdfV2Error::InvalidPlacement);
    }
    let page_object_count = contribution
        .pages
        .len()
        .checked_mul(2)
        .ok_or(StagingSafeVectorPdfV2Error::CountOverflow)?;
    let form_start = 3usize
        .checked_add(page_object_count)
        .ok_or(StagingSafeVectorPdfV2Error::CountOverflow)?;
    let object_count = 2usize
        .checked_add(page_object_count)
        .and_then(|count| count.checked_add(contribution.relative_objects.len()))
        .ok_or(StagingSafeVectorPdfV2Error::CountOverflow)?;
    let object_count_u32 =
        u32::try_from(object_count).map_err(|_| StagingSafeVectorPdfV2Error::CountOverflow)?;
    let mut objects = Vec::new();
    objects
        .try_reserve_exact(object_count)
        .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
    objects.resize_with(object_count, Vec::new);
    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    let mut kids = String::from("[");
    for page_index in 0..contribution.pages.len() {
        if page_index > 0 {
            kids.push(' ');
        }
        kids.push_str(&(3 + page_index * 2).to_string());
        kids.push_str(" 0 R");
    }
    kids.push(']');
    objects[1] = format!(
        "<< /Type /Pages /Count {} /Kids {} >>",
        contribution.pages.len(),
        kids
    )
    .into_bytes();

    let absolute_for_role = |role: u32| -> Result<u32, StagingSafeVectorPdfV2Error> {
        u32::try_from(form_start)
            .ok()
            .and_then(|start| start.checked_add(role))
            .ok_or(StagingSafeVectorPdfV2Error::CountOverflow)
    };
    for relative in &contribution.relative_objects {
        let absolute = absolute_for_role(relative.relative_object_role)?;
        let target = usize::try_from(absolute - 1)
            .map_err(|_| StagingSafeVectorPdfV2Error::CountOverflow)?;
        objects[target] = match relative.kind {
            StagingSafeVectorPdfRelativeObjectKindV2::Form => {
                let form = contribution
                    .forms
                    .iter()
                    .find(|form| form.relative_object_role == relative.relative_object_role)
                    .ok_or(StagingSafeVectorPdfV2Error::ContributionMismatch)?;
                let mut resources = String::from("<< /ExtGState <<");
                for (name, role) in &form.ext_g_state_roles {
                    resources.push_str(&format!(" /{} {} 0 R", name, absolute_for_role(*role)?));
                }
                resources.push_str(" >> >>");
                let mut object = format!(
                    "<< /Type /XObject /Subtype /Form /FormType 1 /BBox [{} {} {} {}] /Resources {} /Length {} >>\nstream\n",
                    pdf_fixed(form.bbox[0]),
                    pdf_fixed(form.bbox[1]),
                    pdf_fixed(form.bbox[2]),
                    pdf_fixed(form.bbox[3]),
                    resources,
                    form.content_stream.len()
                )
                .into_bytes();
                object.extend_from_slice(&form.content_stream);
                object.extend_from_slice(b"\nendstream");
                object
            }
            StagingSafeVectorPdfRelativeObjectKindV2::ExtGState => contribution
                .ext_g_states
                .iter()
                .find(|ext| ext.relative_object_role == relative.relative_object_role)
                .map(|ext| ext.dictionary.clone())
                .ok_or(StagingSafeVectorPdfV2Error::ContributionMismatch)?,
        };
    }

    for (index, page) in contribution.pages.iter().enumerate() {
        let page_object_number = 3usize
            .checked_add(
                index
                    .checked_mul(2)
                    .ok_or(StagingSafeVectorPdfV2Error::CountOverflow)?,
            )
            .ok_or(StagingSafeVectorPdfV2Error::CountOverflow)?;
        let content_object_number = page_object_number
            .checked_add(1)
            .ok_or(StagingSafeVectorPdfV2Error::CountOverflow)?;
        let mut resources = String::from("<< /XObject <<");
        for resource in &page.resources {
            resources.push_str(&format!(
                " /{} {} 0 R",
                resource.resource_name,
                absolute_for_role(resource.form_relative_object_role)?
            ));
        }
        resources.push_str(" >> >>");
        objects[page_object_number - 1] = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources {} /Contents {} 0 R >>",
            pdf_fixed(page_width),
            pdf_fixed(page_height),
            resources,
            content_object_number
        )
        .into_bytes();
        let mut content = format!("q\n1 0 0 -1 0 {} cm\n", pdf_fixed(page_height)).into_bytes();
        for usage_id in &page.usage_ids {
            let usage = usize::try_from(*usage_id)
                .ok()
                .and_then(|usage_index| contribution.usages.get(usage_index))
                .ok_or(StagingSafeVectorPdfV2Error::ContributionMismatch)?;
            content.extend_from_slice(&usage.content);
            content.push(b'\n');
        }
        content.extend_from_slice(b"Q");
        let mut object = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
        object.extend_from_slice(&content);
        object.extend_from_slice(b"\nendstream");
        objects[content_object_number - 1] = object;
    }

    if objects.iter().any(Vec::is_empty) {
        return Err(StagingSafeVectorPdfV2Error::ContributionMismatch);
    }
    let bytes = serialize_isolated_objects(&objects)?;
    let object_table = contribution
        .relative_objects
        .iter()
        .map(|object| {
            Ok(
                StagingSafeVectorPdfFinalObjectObservationV2::from_final_writer(
                    object.relative_object_role,
                    absolute_for_role(object.relative_object_role)?,
                    object.object_contribution_fingerprint,
                ),
            )
        })
        .collect::<Result<Vec<_>, StagingSafeVectorPdfV2Error>>()?;
    let usages = contribution
        .usages
        .iter()
        .map(|usage| {
            let page_object_number = usage
                .page_index
                .checked_mul(2)
                .and_then(|offset| offset.checked_add(3))
                .ok_or(StagingSafeVectorPdfV2Error::CountOverflow)?;
            let page_content_object_number = page_object_number
                .checked_add(1)
                .ok_or(StagingSafeVectorPdfV2Error::CountOverflow)?;
            Ok(
                StagingSafeVectorPdfFinalUsageObservationV2::from_final_writer(
                    usage.usage_id,
                    usage.page_index,
                    usage.paint_ordinal,
                    page_object_number,
                    page_content_object_number,
                    absolute_for_role(usage.form_relative_object_role)?,
                    usage.content_fingerprint,
                ),
            )
        })
        .collect::<Result<Vec<_>, StagingSafeVectorPdfV2Error>>()?;
    Ok(StagingSafeVectorIsolatedPdfFixtureV2 {
        bytes,
        object_table,
        usages,
        object_count: object_count_u32,
    })
}

#[cfg(any(test, feature = "staging-fixtures"))]
fn serialize_isolated_objects(objects: &[Vec<u8>]) -> Result<Vec<u8>, StagingSafeVectorPdfV2Error> {
    let mut output = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(objects.len())
        .map_err(|_| StagingSafeVectorPdfV2Error::AllocationFailure)?;
    for (index, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        output.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        output.extend_from_slice(object);
        output.extend_from_slice(b"\nendobj\n");
    }
    let xref = output.len();
    output.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    output.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    output.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    Ok(output)
}

#[cfg(test)]
fn byte_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{EffectiveConfigFingerprint, LayoutStateFingerprint, PdfStreamCompression};
    use typaxis_resources::{
        finalize_staging_combined_safe_vector_forms_v2, finalize_staging_safe_vector_forms_v2,
        staging_safe_vector_v2_ir_fixture, StagingSafeVectorV2IrFixture,
        VectorContentCandidateRegistry,
    };

    fn build_fixture(
        ten_uses: bool,
    ) -> (
        typaxis_display_list::StagingPrecomposedVectorDisplayFixture,
        VectorContentCandidateRegistry,
        StagingSafeVectorFormPlansV2,
        StagingSafeVectorPdfContributionV2,
    ) {
        let fixture = if ten_uses {
            typaxis_display_list::staging_precomposed_vector_display_ten_use_fixture().unwrap()
        } else {
            typaxis_display_list::staging_precomposed_vector_display_fixture().unwrap()
        };
        let registry = VectorContentCandidateRegistry::from_admitted(
            &fixture.layout.admitted,
            fixture.layout.package.resources(),
        )
        .unwrap();
        let plans = finalize_staging_safe_vector_forms_v2(
            &fixture.display,
            &registry,
            &fixture.layout.limits,
        )
        .unwrap();
        let contribution = build_staging_safe_vector_pdf_contribution_v2(
            &fixture.display,
            &plans,
            &registry,
            &fixture.layout.limits,
        )
        .unwrap();
        (fixture, registry, plans, contribution)
    }

    fn ext_g_states_for_ir(
        ir: &AdmittedSafeVector,
        key: VectorContentKey,
    ) -> Vec<StagingSafeVectorPdfExtGStateV2> {
        let pairs: BTreeSet<(u32, u32)> = match ir {
            AdmittedSafeVector::V1(_) => {
                [(FIXED_ONE as u32, FIXED_ONE as u32)].into_iter().collect()
            }
            AdmittedSafeVector::V2(ir) => ir
                .draws()
                .iter()
                .map(|draw| {
                    (
                        draw.fill().alpha().raw(),
                        draw.stroke().paint().alpha().raw(),
                    )
                })
                .collect(),
        };
        pairs
            .into_iter()
            .enumerate()
            .map(|(index, (fill_alpha_raw, stroke_alpha_raw))| {
                let mut spool = SpoolBudget::new(u64::MAX);
                let dictionary =
                    encode_ext_g_state_dictionary_raw(fill_alpha_raw, stroke_alpha_raw, &mut spool)
                        .unwrap();
                StagingSafeVectorPdfExtGStateV2 {
                    content_key: key,
                    relative_object_role: u32::try_from(index + 1).unwrap(),
                    resource_name: format!("GS{index}"),
                    fill_alpha_raw,
                    stroke_alpha_raw,
                    dictionary_fingerprint: sha256(&dictionary),
                    dictionary,
                }
            })
            .collect()
    }

    #[test]
    fn safe_vector_pdf_contribution_v2_is_deterministic_deduplicated_and_vector_only() {
        let (fixture, registry, plans, contribution) = build_fixture(true);
        let second = build_staging_safe_vector_pdf_contribution_v2(
            &fixture.display,
            &plans,
            &registry,
            &fixture.layout.limits,
        )
        .unwrap();
        assert_eq!(contribution, second);
        assert_eq!(contribution.forms().len(), 1);
        assert_eq!(contribution.ext_g_states().len(), 1);
        assert_eq!(contribution.relative_objects().len(), 2);
        assert_eq!(contribution.pages().len(), 1);
        assert_eq!(contribution.pages()[0].resources().len(), 1);
        assert_eq!(contribution.usages().len(), 10);
        assert_eq!(
            contribution.forms()[0].bbox(),
            [0, 0, 30 * FIXED_ONE, 12 * FIXED_ONE]
        );
        let form = contribution.forms()[0].content_stream();
        assert!(byte_occurrences(form, b" m\n") > 0);
        assert!(byte_occurrences(form, b" l\n") > 0);
        assert_eq!(byte_occurrences(form, b"/GS0 gs\n"), 1);
        assert_eq!(byte_occurrences(form, b"/Subtype /Image"), 0);
        assert_eq!(byte_occurrences(form, b"/MCID"), 0);
        assert_eq!(byte_occurrences(form, b"/ActualText"), 0);
        assert!(contribution
            .pages()
            .iter()
            .all(StagingSafeVectorPdfPageV2::requires_existing_top_left_page_root_y_flip));
        assert!(!contribution.canonical_jcs().contains("object_number"));
        assert!(!contribution.canonical_jcs().contains("max_pdf_objects"));
        assert!(!contribution.canonical_jcs().contains("pdf_sha256"));
        contribution
            .verify(&fixture.display, &plans, &registry, &fixture.layout.limits)
            .unwrap();

        let isolated = staging_safe_vector_isolated_pdf_fixture_v2(
            &contribution,
            240 * FIXED_ONE,
            140 * FIXED_ONE,
        )
        .unwrap();
        assert_eq!(byte_occurrences(&isolated.bytes, b"/Subtype /Form"), 1);
        assert_eq!(byte_occurrences(&isolated.bytes, b"/Subtype /Image"), 0);
        assert_eq!(byte_occurrences(&isolated.bytes, b"/V0 Do"), 10);
        assert_eq!(byte_occurrences(&isolated.bytes, b"1 0 0 -1 0 140 cm"), 1);
        assert!(isolated
            .bytes
            .windows(b"/BBox [0 0 30 12]".len())
            .any(|window| window == b"/BBox [0 0 30 12]"));
        assert!(isolated
            .bytes
            .windows(b"/Resources << /ExtGState << /GS0".len())
            .any(|window| window == b"/Resources << /ExtGState << /GS0"));

        let final_writer = StagingSafeVectorPdfFinalWriterObservationV2::from_final_writer(
            &contribution,
            isolated.object_table.clone(),
            isolated.usages.clone(),
        )
        .unwrap();
        let final_pdf = VerifiedPdfBytesReceipt {
            sha256: sha256(&isolated.bytes),
            bytes: isolated.bytes,
            selected_layout_fingerprint: LayoutStateFingerprint::from_untrusted_bytes([7; 32]),
            footnote_display_sha256: None,
            page_count: 1,
            object_count: isolated.object_count,
            stream_compression: PdfStreamCompression::None,
            config_fingerprint: EffectiveConfigFingerprint::from_untrusted_bytes([8; 32]),
        };
        let closure =
            seal_staging_safe_vector_pdf_v2(&contribution, &final_writer, &final_pdf).unwrap();
        assert_eq!(closure.algorithm(), STAGING_SAFE_VECTOR_PDF_ALGORITHM_V2);
        assert_eq!(closure.final_pdf_sha256(), sha256(final_pdf.bytes()));
    }

    #[test]
    fn safe_vector_pdf_contribution_v2_rejects_name_role_use_and_spool_tamper() {
        let (fixture, registry, plans, contribution) = build_fixture(false);

        let mut wrong_name = contribution.clone();
        wrong_name.relative_objects[0].resource_name = "V9".to_owned();
        assert_eq!(
            wrong_name.verify(&fixture.display, &plans, &registry, &fixture.layout.limits),
            Err(StagingSafeVectorPdfV2Error::ContributionMismatch)
        );

        let mut wrong_role = contribution.clone();
        wrong_role.relative_objects.swap(0, 1);
        assert_eq!(
            wrong_role.verify(&fixture.display, &plans, &registry, &fixture.layout.limits),
            Err(StagingSafeVectorPdfV2Error::ContributionMismatch)
        );

        let unused_key = *registry
            .candidates()
            .iter()
            .find(|candidate| candidate.key() != contribution.forms()[0].content_key())
            .unwrap()
            .key();
        let mut wrong_key = contribution.clone();
        wrong_key.relative_objects[0].content_key = unused_key;
        assert_eq!(
            wrong_key.verify(&fixture.display, &plans, &registry, &fixture.layout.limits),
            Err(StagingSafeVectorPdfV2Error::ContributionMismatch)
        );

        let mut extra_form = contribution.clone();
        extra_form.forms.push(extra_form.forms[0].clone());
        assert_eq!(
            extra_form.verify(&fixture.display, &plans, &registry, &fixture.layout.limits),
            Err(StagingSafeVectorPdfV2Error::ContributionMismatch)
        );

        let mut missing_use = contribution.clone();
        missing_use.usages.pop();
        assert_eq!(
            missing_use.verify(&fixture.display, &plans, &registry, &fixture.layout.limits),
            Err(StagingSafeVectorPdfV2Error::ContributionMismatch)
        );
        let mut wrong_use_order = contribution.clone();
        wrong_use_order.usages.swap(0, 1);
        assert_eq!(
            wrong_use_order.verify(&fixture.display, &plans, &registry, &fixture.layout.limits),
            Err(StagingSafeVectorPdfV2Error::ContributionMismatch)
        );

        assert_eq!(
            build_staging_safe_vector_pdf_contribution_v2_with_spool_limit(
                &fixture.display,
                &plans,
                &registry,
                &fixture.layout.limits,
                contribution.spool_bytes(),
            )
            .unwrap(),
            contribution
        );
        assert_eq!(
            build_staging_safe_vector_pdf_contribution_v2_with_spool_limit(
                &fixture.display,
                &plans,
                &registry,
                &fixture.layout.limits,
                contribution.spool_bytes() - 1,
            ),
            Err(StagingSafeVectorPdfV2Error::SpoolLimit)
        );
    }

    #[test]
    fn safe_vector_current_color_is_page_local_and_form_reusable() {
        let (_, _, _, contribution) = build_fixture(false);
        let form = &contribution.forms()[0];
        assert_eq!(byte_occurrences(form.content_stream(), b" rg\n"), 0);
        assert_eq!(byte_occurrences(form.content_stream(), b" RG\n"), 0);
        assert!(contribution.usages().iter().all(|usage| {
            usage.resolved_current_color() == [0, 0, 0]
                && byte_occurrences(usage.content(), b"0 0 0 rg\n") == 1
                && byte_occurrences(usage.content(), b"0 0 0 RG\n") == 1
        }));

        let matrix = contribution.usages()[0].matrix();
        let mut black_spool = SpoolBudget::new(u64::MAX);
        let black = encode_page_usage_values(matrix, [0, 0, 0], form, &mut black_spool).unwrap();
        let mut color_spool = SpoolBudget::new(u64::MAX);
        let color = encode_page_usage_values(matrix, [17, 34, 51], form, &mut color_spool).unwrap();
        assert_ne!(black, color);
        assert_eq!(byte_occurrences(&black, b"0 0 0 rg\n"), 1);
        assert_eq!(byte_occurrences(&black, b"0 0 0 RG\n"), 1);
        assert_eq!(byte_occurrences(&color, b" rg\n"), 1);
        assert_eq!(byte_occurrences(&color, b" RG\n"), 1);
        assert!(black.starts_with(b"q\n"));
        assert!(black.ends_with(b"\nQ"));
        assert_eq!(form, &contribution.forms()[0]);
    }

    #[test]
    fn safe_vector_ext_gstate_is_explicit_local_and_minimal() {
        let (_, _, _, contribution) = build_fixture(false);
        assert_eq!(contribution.ext_g_states().len(), 1);
        let opaque = &contribution.ext_g_states()[0];
        assert_eq!(opaque.dictionary(), b"<< /Type /ExtGState /ca 1 /CA 1 >>");
        assert_eq!(
            byte_occurrences(contribution.forms()[0].content_stream(), b"/GS0 gs\n"),
            1
        );
        let public_form = contribution.forms()[0].content_stream();
        assert_eq!(byte_occurrences(public_form, b"1.5 w\n"), 1);
        assert_eq!(byte_occurrences(public_form, b"1 J\n"), 1);
        assert_eq!(byte_occurrences(public_form, b"1 j\n"), 1);
        assert_eq!(byte_occurrences(public_form, b"4 M\n"), 1);

        let mut spool = SpoolBudget::new(128);
        let nonopaque = encode_ext_g_state_dictionary_raw(49_152, 32_768, &mut spool).unwrap();
        assert_eq!(nonopaque, b"<< /Type /ExtGState /ca 0.75 /CA 0.5 >>");
        assert_eq!(byte_occurrences(&nonopaque, b"/Type"), 1);
        assert_eq!(byte_occurrences(&nonopaque, b"/ca"), 1);
        assert_eq!(byte_occurrences(&nonopaque, b"/CA"), 1);
        assert_eq!(byte_occurrences(&nonopaque, b"/BM"), 0);
        assert_eq!(byte_occurrences(&nonopaque, b"/SMask"), 0);
        assert_eq!(byte_occurrences(&nonopaque, b"/AIS"), 0);

        let (fraction, key) =
            staging_safe_vector_v2_ir_fixture(StagingSafeVectorV2IrFixture::FractionEquality)
                .unwrap();
        let fraction_ext = ext_g_states_for_ir(&fraction, key);
        assert_eq!(fraction_ext.len(), 1);
        assert_eq!(fraction_ext[0].fill_alpha_raw(), 49_152);
        assert_eq!(fraction_ext[0].stroke_alpha_raw(), 32_768);
        let mut fraction_spool = SpoolBudget::new(u64::MAX);
        let fraction_content =
            encode_admitted_ir_content(&fraction, &fraction_ext, &mut fraction_spool).unwrap();
        let draw_count = match &fraction {
            AdmittedSafeVector::V2(ir) => ir.draws().len(),
            AdmittedSafeVector::V1(_) => unreachable!(),
        };
        assert_eq!(
            byte_occurrences(&fraction_content, b"/GS0 gs\n"),
            draw_count
        );
        assert_eq!(byte_occurrences(&fraction_content, b" rg\n"), 0);
        assert_eq!(byte_occurrences(&fraction_content, b" RG\n"), 0);
        assert_eq!(byte_occurrences(&fraction_content, b"B\n"), draw_count);

        let (matrix, matrix_key) =
            staging_safe_vector_v2_ir_fixture(StagingSafeVectorV2IrFixture::Matrix).unwrap();
        let matrix_ext = ext_g_states_for_ir(&matrix, matrix_key);
        let mut matrix_spool = SpoolBudget::new(u64::MAX);
        let matrix_content =
            encode_admitted_ir_content(&matrix, &matrix_ext, &mut matrix_spool).unwrap();
        assert!(byte_occurrences(&matrix_content, b"W n\n") > 1);
        assert!(byte_occurrences(&matrix_content, b"0 0 0 rg\n") > 0);
        assert!(byte_occurrences(&matrix_content, b" c\n") > 0);

        let v1_fixture = typaxis_resources::staging_safe_vector_resource_fixture().unwrap();
        let v1_registry = VectorContentCandidateRegistry::from_admitted(
            &v1_fixture.display.layout.admitted,
            v1_fixture.display.layout.package.resources(),
        )
        .unwrap();
        let v1_candidate = v1_registry
            .candidate_for_alias(ImageResourceId::new(0))
            .unwrap();
        assert!(matches!(
            v1_candidate.canonical_ir(),
            AdmittedSafeVector::V1(_)
        ));
        let v1_ext = ext_g_states_for_ir(v1_candidate.canonical_ir(), *v1_candidate.key());
        assert_eq!(v1_ext.len(), 1);
        assert_eq!(v1_ext[0].fill_alpha_raw(), FIXED_ONE as u32);
        assert_eq!(v1_ext[0].stroke_alpha_raw(), FIXED_ONE as u32);
        let mut v1_spool = SpoolBudget::new(u64::MAX);
        let v1_content =
            encode_admitted_ir_content(v1_candidate.canonical_ir(), &v1_ext, &mut v1_spool)
                .unwrap();
        let AdmittedSafeVector::V1(v1_ir) = v1_candidate.canonical_ir() else {
            unreachable!();
        };
        assert_eq!(
            byte_occurrences(&v1_content, b"/GS0 gs\n"),
            v1_ir.draws().len()
        );
        assert!(byte_occurrences(&v1_content, b" rg\n") > 0);
        assert_eq!(byte_occurrences(&v1_content, b"/Subtype /Image"), 0);
    }

    #[test]
    fn safe_vector_pdf_final_observation_rejects_missing_and_wrong_rows() {
        let (_, _, _, contribution) = build_fixture(false);
        let isolated = staging_safe_vector_isolated_pdf_fixture_v2(
            &contribution,
            240 * FIXED_ONE,
            140 * FIXED_ONE,
        )
        .unwrap();
        let mut missing = isolated.object_table.clone();
        missing.pop();
        assert_eq!(
            StagingSafeVectorPdfFinalWriterObservationV2::from_final_writer(
                &contribution,
                missing,
                isolated.usages.clone(),
            ),
            Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch)
        );
        let mut wrong = isolated.object_table.clone();
        wrong[0].object_contribution_fingerprint = [9; 32];
        assert_eq!(
            StagingSafeVectorPdfFinalWriterObservationV2::from_final_writer(
                &contribution,
                wrong,
                isolated.usages.clone(),
            ),
            Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch)
        );

        let mut overlapping_object = isolated.usages.clone();
        overlapping_object[0].page_object_number = isolated.object_table[0].absolute_object_number;
        assert_eq!(
            StagingSafeVectorPdfFinalWriterObservationV2::from_final_writer(
                &contribution,
                isolated.object_table.clone(),
                overlapping_object,
            ),
            Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch)
        );

        let mut wrong_form_target = isolated.usages.clone();
        wrong_form_target[0].form_absolute_object_number =
            isolated.object_table[1].absolute_object_number;
        assert_eq!(
            StagingSafeVectorPdfFinalWriterObservationV2::from_final_writer(
                &contribution,
                isolated.object_table.clone(),
                wrong_form_target,
            ),
            Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch)
        );

        let first_page_pair = (
            isolated.usages[0].page_object_number,
            isolated.usages[0].page_content_object_number,
        );
        let mut repeated_page_objects = isolated.usages;
        for usage in &mut repeated_page_objects {
            if usage.page_index == 1 {
                usage.page_object_number = first_page_pair.0;
                usage.page_content_object_number = first_page_pair.1;
            }
        }
        assert_eq!(
            StagingSafeVectorPdfFinalWriterObservationV2::from_final_writer(
                &contribution,
                isolated.object_table,
                repeated_page_objects,
            ),
            Err(StagingSafeVectorPdfV2Error::FinalWriterMismatch)
        );
    }

    #[test]
    fn safe_vector_pdf_v1_frozen_bytes() {
        let fixture = typaxis_resources::staging_safe_vector_resource_fixture().unwrap();
        let first = crate::write_staging_safe_vector_pdf(
            &fixture.display.display,
            &fixture.plans,
            &fixture.display.layout.limits,
        )
        .unwrap();
        let second = crate::write_staging_safe_vector_pdf(
            &fixture.display.display,
            &fixture.plans,
            &fixture.display.layout.limits,
        )
        .unwrap();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.bytes().len(), 1_213);
        assert_eq!(
            first.receipt().pdf_sha256(),
            [
                0xc7, 0xbb, 0x8e, 0x72, 0xad, 0xc0, 0xe6, 0x0d, 0x30, 0x31, 0x12, 0x64, 0x7e, 0x97,
                0x8d, 0xd5, 0xd2, 0xdb, 0x44, 0xfe, 0x82, 0xc5, 0x65, 0x99, 0x64, 0x4e, 0x23, 0xa6,
                0x8d, 0x61, 0xba, 0xff,
            ]
        );
        assert_eq!(first.receipt().pdf_sha256(), sha256(first.bytes()));
        assert!(first
            .bytes()
            .windows(b"/Resources << >>".len())
            .any(|window| window == b"/Resources << >>"));
        assert_eq!(byte_occurrences(first.bytes(), b"/ExtGState"), 0);
    }

    #[test]
    fn combined_safe_vector_pdf_contribution_keeps_existing_figure_vector_native() {
        let fixture = typaxis_display_list::staging_combined_vector_figure_fixture().unwrap();
        let registry = VectorContentCandidateRegistry::from_admitted(
            &fixture.figure.layout.admitted,
            fixture.figure.layout.package.resources(),
        )
        .unwrap();
        let plans = finalize_staging_combined_safe_vector_forms_v2(
            &fixture.display,
            &registry,
            &fixture.figure.layout.limits,
        )
        .unwrap();
        let contribution = build_staging_combined_safe_vector_pdf_contribution_v2(
            &fixture.display,
            &plans,
            &registry,
            &fixture.figure.layout.limits,
        )
        .unwrap();
        let [usage] = contribution.usages() else {
            panic!("Figure fixture must emit one PDF vector usage");
        };
        assert_eq!(
            usage.semantic_hook().kind(),
            StagingCombinedVectorKindV2::Figure
        );
        assert_eq!(byte_occurrences(usage.content(), b"/V0 Do"), 1);
        assert!(byte_occurrences(contribution.forms()[0].content_stream(), b" m\n") > 0);
        assert_eq!(
            byte_occurrences(contribution.forms()[0].content_stream(), b"/Subtype /Image"),
            0
        );
        contribution
            .verify_combined(
                &fixture.display,
                &plans,
                &registry,
                &fixture.figure.layout.limits,
            )
            .unwrap();
    }
}
