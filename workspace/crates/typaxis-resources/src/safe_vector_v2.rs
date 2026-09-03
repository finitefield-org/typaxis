use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{push_jcs_string, sha256, ImageResourceId, M4EffectiveResourceLimits};
use typaxis_display_list::{StagingDrawVectorV2, StagingPrecomposedVectorDisplay};
use typaxis_resource_admission::{AdmittedSafeVector, VectorContentKey};

use crate::{VectorContentCandidateRegistry, VectorContentPlanningError, VectorExtGStateAlphaPair};

pub const STAGING_SAFE_VECTOR_FORM_PLAN_V2_ALGORITHM: &str = "typaxis.safe-vector-form-plan/2";
pub const STAGING_SAFE_VECTOR_FORM_PLANS_V2_ALGORITHM: &str = "typaxis.safe-vector-form-plans/2";

/// Usage count for one logical image alias. Zero is retained deliberately so
/// an admitted-but-unselected alias remains part of the audit closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorAliasUsageCountV2 {
    image_id: ImageResourceId,
    usage_count: u32,
}

impl StagingSafeVectorAliasUsageCountV2 {
    pub const fn image_id(self) -> ImageResourceId {
        self.image_id
    }

    pub const fn usage_count(self) -> u32 {
        self.usage_count
    }
}

/// One selected DrawVector `/2` occurrence, retained in selected page/paint
/// order. The Form plan never derives ordering from resource first use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorUsageV2 {
    usage_id: u32,
    image_id: ImageResourceId,
    page_index: u32,
    paint_ordinal: u32,
    display_command_fingerprint: [u8; 32],
}

impl StagingSafeVectorUsageV2 {
    pub const fn usage_id(&self) -> u32 {
        self.usage_id
    }

    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
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

/// One Form-local ExtGState resource. The role is relative to the complete
/// vector contribution, not an absolute PDF indirect-object number.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenSafeVectorExtGStatePlanV2 {
    alpha_pair: VectorExtGStateAlphaPair,
    relative_object_role: u32,
    resource_name: String,
}

impl FrozenSafeVectorExtGStatePlanV2 {
    pub const fn alpha_pair(&self) -> VectorExtGStateAlphaPair {
        self.alpha_pair
    }

    pub const fn relative_object_role(&self) -> u32 {
        self.relative_object_role
    }

    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }
}

/// Deduplicated selected Form plan. The canonical IR is carried from resource
/// admission; neither SVG bytes nor XML are reopened by the PDF backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenSafeVectorFormPlanV2 {
    content_key: VectorContentKey,
    ir: AdmittedSafeVector,
    form_relative_object_role: u32,
    form_resource_name: String,
    ext_g_states: Vec<FrozenSafeVectorExtGStatePlanV2>,
    alias_usage_counts: Vec<StagingSafeVectorAliasUsageCountV2>,
    total_usage_count: u32,
    usages: Vec<StagingSafeVectorUsageV2>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl FrozenSafeVectorFormPlanV2 {
    pub const fn algorithm(&self) -> &'static str {
        STAGING_SAFE_VECTOR_FORM_PLAN_V2_ALGORITHM
    }

    pub const fn content_key(&self) -> &VectorContentKey {
        &self.content_key
    }

    pub const fn ir(&self) -> &AdmittedSafeVector {
        &self.ir
    }

    pub const fn form_relative_object_role(&self) -> u32 {
        self.form_relative_object_role
    }

    pub fn form_resource_name(&self) -> &str {
        &self.form_resource_name
    }

    pub fn ext_g_states(&self) -> &[FrozenSafeVectorExtGStatePlanV2] {
        &self.ext_g_states
    }

    pub fn alias_usage_counts(&self) -> &[StagingSafeVectorAliasUsageCountV2] {
        &self.alias_usage_counts
    }

    pub const fn total_usage_count(&self) -> u32 {
        self.total_usage_count
    }

    pub fn usages(&self) -> &[StagingSafeVectorUsageV2] {
        &self.usages
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// Closed `/2` Form plan set. Counts are contribution deltas only: the final
/// graph owner will merge them with every other PDF role and charge the global
/// object budget exactly once.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorFormPlansV2 {
    display_fingerprint: [u8; 32],
    candidate_registry_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    audit_candidate_count: u32,
    audit_alias_count: u32,
    form_object_count_delta: u32,
    ext_g_state_object_count_delta: u32,
    relative_object_role_count_delta: u32,
    page_resource_binding_count_delta: u32,
    page_do_count_delta: u32,
    plans: Vec<FrozenSafeVectorFormPlanV2>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorFormPlansV2 {
    pub const fn algorithm(&self) -> &'static str {
        STAGING_SAFE_VECTOR_FORM_PLANS_V2_ALGORITHM
    }

    pub const fn display_fingerprint(&self) -> [u8; 32] {
        self.display_fingerprint
    }

    pub const fn candidate_registry_fingerprint(&self) -> [u8; 32] {
        self.candidate_registry_fingerprint
    }

    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }

    pub const fn audit_candidate_count(&self) -> u32 {
        self.audit_candidate_count
    }

    pub const fn audit_alias_count(&self) -> u32 {
        self.audit_alias_count
    }

    pub const fn form_object_count_delta(&self) -> u32 {
        self.form_object_count_delta
    }

    pub const fn ext_g_state_object_count_delta(&self) -> u32 {
        self.ext_g_state_object_count_delta
    }

    pub const fn relative_object_role_count_delta(&self) -> u32 {
        self.relative_object_role_count_delta
    }

    pub const fn page_resource_binding_count_delta(&self) -> u32 {
        self.page_resource_binding_count_delta
    }

    pub const fn page_do_count_delta(&self) -> u32 {
        self.page_do_count_delta
    }

    pub fn plans(&self) -> &[FrozenSafeVectorFormPlanV2] {
        &self.plans
    }

    pub fn plan(&self, key: &VectorContentKey) -> Option<&FrozenSafeVectorFormPlanV2> {
        self.plans
            .binary_search_by(|plan| plan.content_key.cmp(key))
            .ok()
            .map(|index| &self.plans[index])
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify_pdf_closure(
        &self,
        display: &StagingPrecomposedVectorDisplay,
        registry: &VectorContentCandidateRegistry,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSafeVectorResourceV2Error> {
        let expected = finalize_staging_safe_vector_forms_v2(display, registry, limits)?;
        if self != &expected {
            return Err(StagingSafeVectorResourceV2Error::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorResourceV2Error {
    DisplayMismatch,
    CandidateMismatch,
    AliasMismatch(ImageResourceId),
    LimitsMismatch,
    CountOverflow,
    ObjectRoleCountOverflow,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingSafeVectorResourceV2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DisplayMismatch => {
                formatter.write_str("I9190: DrawVector /2 Display closure mismatch")
            }
            Self::CandidateMismatch => {
                formatter.write_str("I9190: vector content candidate mismatch")
            }
            Self::AliasMismatch(id) => write!(
                formatter,
                "I9190: vector content alias {} does not close selected usage",
                id.get()
            ),
            Self::LimitsMismatch => formatter.write_str("I9190: vector Form plan limits mismatch"),
            Self::CountOverflow => formatter.write_str("D8101: vector Form plan count overflow"),
            Self::ObjectRoleCountOverflow => {
                formatter.write_str("D8101: vector relative object-role count overflow")
            }
            Self::AllocationFailure => {
                formatter.write_str("D8101: vector Form plan allocation failed")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: SafeVector Form plan /2 receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingSafeVectorResourceV2Error {}

impl From<VectorContentPlanningError> for StagingSafeVectorResourceV2Error {
    fn from(value: VectorContentPlanningError) -> Self {
        match value {
            VectorContentPlanningError::ObjectRoleCountOverflow => Self::ObjectRoleCountOverflow,
            VectorContentPlanningError::AllocationFailure => Self::AllocationFailure,
            _ => Self::CandidateMismatch,
        }
    }
}

struct CandidateUsageAccumulator {
    alias_counts: BTreeMap<ImageResourceId, u32>,
    usages: Vec<StagingSafeVectorUsageV2>,
}

pub fn finalize_staging_safe_vector_forms_v2(
    display: &StagingPrecomposedVectorDisplay,
    registry: &VectorContentCandidateRegistry,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingSafeVectorFormPlansV2, StagingSafeVectorResourceV2Error> {
    display
        .verify_resource_closure()
        .map_err(|_| StagingSafeVectorResourceV2Error::DisplayMismatch)?;
    if display.receipt().limits_fingerprint() != limits.fingerprint() {
        return Err(StagingSafeVectorResourceV2Error::LimitsMismatch);
    }

    let mut joined = BTreeMap::new();
    for candidate in registry.candidates() {
        let mut alias_counts = BTreeMap::new();
        for alias in candidate.aliases() {
            if alias.limits_fingerprint() != limits.fingerprint() {
                return Err(StagingSafeVectorResourceV2Error::LimitsMismatch);
            }
            if alias_counts.insert(alias.image_id(), 0).is_some() {
                return Err(StagingSafeVectorResourceV2Error::AliasMismatch(
                    alias.image_id(),
                ));
            }
        }
        if joined
            .insert(
                *candidate.key(),
                CandidateUsageAccumulator {
                    alias_counts,
                    usages: Vec::new(),
                },
            )
            .is_some()
        {
            return Err(StagingSafeVectorResourceV2Error::CandidateMismatch);
        }
    }

    for command in display.commands() {
        join_command(command, registry, &mut joined)?;
    }

    let selected_candidate_count = joined
        .values()
        .filter(|accumulator| !accumulator.usages.is_empty())
        .count();
    if u32::try_from(selected_candidate_count).ok() != Some(display.receipt().content_key_count()) {
        return Err(StagingSafeVectorResourceV2Error::CandidateMismatch);
    }

    let mut plans = Vec::new();
    plans
        .try_reserve_exact(selected_candidate_count)
        .map_err(|_| StagingSafeVectorResourceV2Error::AllocationFailure)?;
    let mut next_relative_role = 0u32;
    let mut ext_g_state_object_count_delta = 0u32;
    for candidate in registry.candidates() {
        let accumulator = joined
            .remove(candidate.key())
            .ok_or(StagingSafeVectorResourceV2Error::CandidateMismatch)?;
        if accumulator.usages.is_empty() {
            continue;
        }

        let plan_index = plans.len();
        let form_resource_name = format!("V{plan_index}");
        let form_relative_object_role = next_relative_role;
        next_relative_role = next_relative_role
            .checked_add(
                candidate
                    .ext_g_state_plan()
                    .relative_object_role_count_if_selected(),
            )
            .ok_or(StagingSafeVectorResourceV2Error::ObjectRoleCountOverflow)?;

        let mut ext_g_states = Vec::new();
        ext_g_states
            .try_reserve_exact(candidate.ext_g_state_plan().entries().len())
            .map_err(|_| StagingSafeVectorResourceV2Error::AllocationFailure)?;
        for (index, entry) in candidate.ext_g_state_plan().entries().iter().enumerate() {
            let relative_object_role = form_relative_object_role
                .checked_add(entry.relative_object_role())
                .ok_or(StagingSafeVectorResourceV2Error::ObjectRoleCountOverflow)?;
            ext_g_states.push(FrozenSafeVectorExtGStatePlanV2 {
                alpha_pair: entry.alpha_pair(),
                relative_object_role,
                resource_name: format!("GS{index}"),
            });
        }
        ext_g_state_object_count_delta = ext_g_state_object_count_delta
            .checked_add(
                u32::try_from(ext_g_states.len())
                    .map_err(|_| StagingSafeVectorResourceV2Error::CountOverflow)?,
            )
            .ok_or(StagingSafeVectorResourceV2Error::CountOverflow)?;

        let mut alias_usage_counts = Vec::new();
        alias_usage_counts
            .try_reserve_exact(accumulator.alias_counts.len())
            .map_err(|_| StagingSafeVectorResourceV2Error::AllocationFailure)?;
        alias_usage_counts.extend(accumulator.alias_counts.into_iter().map(
            |(image_id, usage_count)| StagingSafeVectorAliasUsageCountV2 {
                image_id,
                usage_count,
            },
        ));
        let total_usage_count = u32::try_from(accumulator.usages.len())
            .map_err(|_| StagingSafeVectorResourceV2Error::CountOverflow)?;
        let mut plan = FrozenSafeVectorFormPlanV2 {
            content_key: *candidate.key(),
            ir: candidate.canonical_ir().clone(),
            form_relative_object_role,
            form_resource_name,
            ext_g_states,
            alias_usage_counts,
            total_usage_count,
            usages: accumulator.usages,
            canonical_jcs: String::new(),
            fingerprint: [0; 32],
        };
        plan.canonical_jcs = encode_form_plan(&plan);
        plan.fingerprint = sha256(plan.canonical_jcs.as_bytes());
        plans.push(plan);
    }
    if !joined.is_empty() {
        return Err(StagingSafeVectorResourceV2Error::CandidateMismatch);
    }

    let form_object_count_delta =
        u32::try_from(plans.len()).map_err(|_| StagingSafeVectorResourceV2Error::CountOverflow)?;
    let expected_relative_roles = form_object_count_delta
        .checked_add(ext_g_state_object_count_delta)
        .ok_or(StagingSafeVectorResourceV2Error::ObjectRoleCountOverflow)?;
    if next_relative_role != expected_relative_roles {
        return Err(StagingSafeVectorResourceV2Error::ObjectRoleCountOverflow);
    }

    let mut page_bindings = BTreeSet::new();
    for command in display.commands() {
        page_bindings.insert((command.page_index(), command.content_key()));
    }
    let page_resource_binding_count_delta = u32::try_from(page_bindings.len())
        .map_err(|_| StagingSafeVectorResourceV2Error::CountOverflow)?;
    let page_do_count_delta = display.receipt().command_count();
    let audit_candidate_count = registry.receipt().candidate_count();
    let audit_alias_count = registry.receipt().alias_count();
    let canonical_jcs = encode_form_plans(
        display.receipt().fingerprint(),
        registry.receipt().fingerprint(),
        limits.fingerprint(),
        audit_candidate_count,
        audit_alias_count,
        form_object_count_delta,
        ext_g_state_object_count_delta,
        next_relative_role,
        page_resource_binding_count_delta,
        page_do_count_delta,
        &plans,
    );
    Ok(StagingSafeVectorFormPlansV2 {
        display_fingerprint: display.receipt().fingerprint(),
        candidate_registry_fingerprint: registry.receipt().fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        audit_candidate_count,
        audit_alias_count,
        form_object_count_delta,
        ext_g_state_object_count_delta,
        relative_object_role_count_delta: next_relative_role,
        page_resource_binding_count_delta,
        page_do_count_delta,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
        plans,
    })
}

/// Spelling retained for callers that name the produced value rather than the
/// historical finalizer operation.
pub fn finalize_staging_safe_vector_form_plans_v2(
    display: &StagingPrecomposedVectorDisplay,
    registry: &VectorContentCandidateRegistry,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingSafeVectorFormPlansV2, StagingSafeVectorResourceV2Error> {
    finalize_staging_safe_vector_forms_v2(display, registry, limits)
}

#[cfg(any(test, feature = "staging-fixtures"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorV2IrFixture {
    FractionEquality,
    Matrix,
}

#[cfg(any(test, feature = "staging-fixtures"))]
impl StagingSafeVectorV2IrFixture {
    const fn file_stem(self) -> &'static str {
        match self {
            Self::FractionEquality => "fraction-equality",
            Self::Matrix => "matrix",
        }
    }
}

/// Admits one checked-in Safe-SVG 2 sample through the real stable-resource
/// boundary. This is test-only and does not expose a raw IR constructor.
#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_safe_vector_v2_ir_fixture(
    fixture: StagingSafeVectorV2IrFixture,
) -> Result<(AdmittedSafeVector, VectorContentKey), Box<dyn std::error::Error>> {
    use typaxis_core::{
        ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, HostAdmissionContext, HostPath,
        M4ResourceLimits, PdfStreamCompression, PortablePath, ResourceLimits,
        ValidatedResourceLimits, DEFAULT_ALLOWED_URI_SCHEMES,
    };
    use typaxis_document::{
        ImageMediaDeclaration, ImageMediaType, StagingM4ImageDeclaration, StagingM4ResourceCatalog,
        VectorProvenance,
    };
    use typaxis_resource_admission::{
        staging_declared_base_catalog, AdmittedResourceResolver, HostResourceAdmissionSession,
    };

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../samples/machine-package/staging/production-book-1/precomposed-vector");
    let uri = format!("svg/{}.svg", fixture.file_stem());
    let bytes = std::fs::read(root.join(&uri))?;
    let declarations = StagingM4ResourceCatalog {
        font_faces: Vec::new(),
        images: vec![StagingM4ImageDeclaration {
            image_id: ImageResourceId::new(0),
            uri: PortablePath::new(&uri).map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid checked-in Safe-SVG fixture URI: {error:?}"),
                )
            })?,
            expected_sha256: Some(sha256(&bytes)),
            media: ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2),
            vector_provenance: Some(VectorProvenance {
                engine_id: "vmb.texToSvg".to_owned(),
                engine_version: "2026.09.0".to_owned(),
                rules_version: "vmb.math-safe-svg/1".to_owned(),
            }),
        }],
    };
    let base_limits = ValidatedResourceLimits::new(ResourceLimits::default())?;
    let limits = M4EffectiveResourceLimits::new(base_limits, M4ResourceLimits::default())?;
    let base = staging_declared_base_catalog(&declarations)?;
    let config = EffectiveConfig::new(
        true,
        PdfStreamCompression::None,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").ok_or_else(
            || {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "registered fixture data versions are unavailable",
                )
            },
        )?,
        ResourceLimits::default(),
    )?;
    let context = HostAdmissionContext::new(
        HostPath::new(root.join("document-package.json"))?,
        HostPath::new(root)?,
        None,
        Vec::new(),
    );
    let session = HostResourceAdmissionSession::new(&context, &config, &base)?;
    let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
        &base,
        &limits,
        [0x5a; 32],
        session.roots(),
    )?;
    let pending = resolver.read_image(session.open_image(ImageResourceId::new(0))?)?;
    resolver.parse_and_bind_declared_image(pending)?;
    let ledger = resolver.finish()?;
    let image = ledger
        .image(ImageResourceId::new(0))
        .ok_or("staging Safe-SVG 2 fixture was not admitted")?;
    Ok((
        image
            .admitted_safe_vector()
            .ok_or("staging Safe-SVG 2 fixture has no canonical IR")?
            .clone(),
        VectorContentKey::from_admitted(image)?,
    ))
}

fn join_command(
    command: &StagingDrawVectorV2,
    registry: &VectorContentCandidateRegistry,
    joined: &mut BTreeMap<VectorContentKey, CandidateUsageAccumulator>,
) -> Result<(), StagingSafeVectorResourceV2Error> {
    let candidate = registry
        .candidate(&command.content_key())
        .ok_or(StagingSafeVectorResourceV2Error::CandidateMismatch)?;
    if candidate.key() != &command.content_key()
        || candidate.canonical_ir().fingerprint() != command.ir_fingerprint()
    {
        return Err(StagingSafeVectorResourceV2Error::CandidateMismatch);
    }
    let alias = candidate
        .aliases()
        .binary_search_by_key(&command.image_id(), |alias| alias.image_id())
        .ok()
        .and_then(|index| candidate.aliases().get(index))
        .ok_or(StagingSafeVectorResourceV2Error::AliasMismatch(
            command.image_id(),
        ))?;
    if alias.admitted_sha256() != command.content_key().source_sha256() {
        return Err(StagingSafeVectorResourceV2Error::AliasMismatch(
            command.image_id(),
        ));
    }
    let accumulator = joined
        .get_mut(candidate.key())
        .ok_or(StagingSafeVectorResourceV2Error::CandidateMismatch)?;
    let count = accumulator
        .alias_counts
        .get_mut(&command.image_id())
        .ok_or(StagingSafeVectorResourceV2Error::AliasMismatch(
            command.image_id(),
        ))?;
    *count = count
        .checked_add(1)
        .ok_or(StagingSafeVectorResourceV2Error::CountOverflow)?;
    accumulator
        .usages
        .try_reserve(1)
        .map_err(|_| StagingSafeVectorResourceV2Error::AllocationFailure)?;
    accumulator.usages.push(StagingSafeVectorUsageV2 {
        usage_id: command.usage_id(),
        image_id: command.image_id(),
        page_index: command.page_index(),
        paint_ordinal: command.paint_ordinal(),
        display_command_fingerprint: command.fingerprint(),
    });
    Ok(())
}

fn encode_form_plan(plan: &FrozenSafeVectorFormPlanV2) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_SAFE_VECTOR_FORM_PLAN_V2_ALGORITHM);
    output.push_str(",\"alias_usage_counts\":[");
    for (index, alias) in plan.alias_usage_counts.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"image_id\":");
        output.push_str(&alias.image_id.get().to_string());
        output.push_str(",\"usage_count\":");
        output.push_str(&alias.usage_count.to_string());
        output.push('}');
    }
    output.push_str("],\"content_key\":");
    push_content_key(&mut output, &plan.content_key);
    output.push_str(",\"ext_g_states\":[");
    for (index, ext) in plan.ext_g_states.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"fill_alpha\":");
        output.push_str(&ext.alpha_pair.fill_alpha_raw().to_string());
        output.push_str(",\"relative_object_role\":");
        output.push_str(&ext.relative_object_role.to_string());
        output.push_str(",\"resource_name\":");
        push_jcs_string(&mut output, &ext.resource_name);
        output.push_str(",\"stroke_alpha\":");
        output.push_str(&ext.alpha_pair.stroke_alpha_raw().to_string());
        output.push('}');
    }
    output.push_str("],\"form_relative_object_role\":");
    output.push_str(&plan.form_relative_object_role.to_string());
    output.push_str(",\"form_resource_name\":");
    push_jcs_string(&mut output, &plan.form_resource_name);
    output.push_str(",\"intrinsic_height\":");
    output.push_str(&plan.ir.intrinsic_height().get().raw().to_string());
    output.push_str(",\"intrinsic_width\":");
    output.push_str(&plan.ir.intrinsic_width().get().raw().to_string());
    output.push_str(",\"total_usage_count\":");
    output.push_str(&plan.total_usage_count.to_string());
    output.push_str(",\"usages\":[");
    for (index, usage) in plan.usages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"display_command_fingerprint\":");
        push_hash(&mut output, usage.display_command_fingerprint);
        output.push_str(",\"image_id\":");
        output.push_str(&usage.image_id.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&usage.page_index.to_string());
        output.push_str(",\"paint_ordinal\":");
        output.push_str(&usage.paint_ordinal.to_string());
        output.push_str(",\"usage_id\":");
        output.push_str(&usage.usage_id.to_string());
        output.push('}');
    }
    output.push_str("],\"view_box\":[");
    for (index, component) in plan.ir.view_box().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&component.to_string());
    }
    output.push_str("]}");
    output
}

#[allow(clippy::too_many_arguments)]
fn encode_form_plans(
    display_fingerprint: [u8; 32],
    registry_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    audit_candidate_count: u32,
    audit_alias_count: u32,
    form_count: u32,
    ext_count: u32,
    role_count: u32,
    page_binding_count: u32,
    page_do_count: u32,
    plans: &[FrozenSafeVectorFormPlanV2],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_SAFE_VECTOR_FORM_PLANS_V2_ALGORITHM);
    output.push_str(",\"audit_alias_count\":");
    output.push_str(&audit_alias_count.to_string());
    output.push_str(",\"audit_candidate_count\":");
    output.push_str(&audit_candidate_count.to_string());
    output.push_str(",\"candidate_registry_fingerprint\":");
    push_hash(&mut output, registry_fingerprint);
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display_fingerprint);
    output.push_str(",\"ext_g_state_object_count_delta\":");
    output.push_str(&ext_count.to_string());
    output.push_str(",\"form_object_count_delta\":");
    output.push_str(&form_count.to_string());
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    output.push_str(",\"page_do_count_delta\":");
    output.push_str(&page_do_count.to_string());
    output.push_str(",\"page_resource_binding_count_delta\":");
    output.push_str(&page_binding_count.to_string());
    output.push_str(",\"plans\":[");
    for (index, plan) in plans.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"fingerprint\":");
        push_hash(&mut output, plan.fingerprint);
        output.push_str(",\"record\":");
        output.push_str(&plan.canonical_jcs);
        output.push('}');
    }
    output.push_str("],\"relative_object_role_count_delta\":");
    output.push_str(&role_count.to_string());
    output.push('}');
    output
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

    fn fixture() -> (
        typaxis_display_list::StagingPrecomposedVectorDisplayFixture,
        VectorContentCandidateRegistry,
    ) {
        let fixture = typaxis_display_list::staging_precomposed_vector_display_fixture().unwrap();
        let registry = VectorContentCandidateRegistry::from_admitted(
            &fixture.layout.admitted,
            fixture.layout.package.resources(),
        )
        .unwrap();
        (fixture, registry)
    }

    #[test]
    fn safe_vector_form_plans_v2_join_selected_aliases_and_keep_zero_use_audit() {
        let (fixture, registry) = fixture();
        let plans = finalize_staging_safe_vector_forms_v2(
            &fixture.display,
            &registry,
            &fixture.layout.limits,
        )
        .unwrap();
        assert_eq!(registry.candidates().len(), 2);
        assert_eq!(plans.audit_candidate_count(), 2);
        assert_eq!(plans.audit_alias_count(), 2);
        assert_eq!(plans.plans().len(), 1);
        assert_eq!(plans.form_object_count_delta(), 1);
        assert_eq!(plans.ext_g_state_object_count_delta(), 1);
        assert_eq!(plans.relative_object_role_count_delta(), 2);
        assert_eq!(plans.page_resource_binding_count_delta(), 2);
        assert_eq!(plans.page_do_count_delta(), 4);
        let plan = &plans.plans()[0];
        assert_eq!(plan.form_relative_object_role(), 0);
        assert_eq!(plan.form_resource_name(), "V0");
        assert_eq!(plan.total_usage_count(), 4);
        assert_eq!(plan.alias_usage_counts().len(), 1);
        assert_eq!(plan.alias_usage_counts()[0].usage_count(), 4);
        assert_eq!(
            plan.usages()
                .iter()
                .map(|usage| (usage.usage_id(), usage.page_index(), usage.paint_ordinal()))
                .collect::<Vec<_>>(),
            vec![(0, 0, 0), (1, 0, 1), (2, 1, 0), (3, 1, 2)]
        );
        assert_eq!(plan.ext_g_states()[0].relative_object_role(), 1);
        assert_eq!(plan.ext_g_states()[0].resource_name(), "GS0");
        assert!(!plans.canonical_jcs().contains("max_pdf_objects"));
        assert!(!plans.canonical_jcs().contains("object_number"));
        plans
            .verify_pdf_closure(&fixture.display, &registry, &fixture.layout.limits)
            .unwrap();
    }

    #[test]
    fn safe_vector_form_plans_v2_tamper_is_rejected() {
        let (fixture, registry) = fixture();
        let mut plans = finalize_staging_safe_vector_forms_v2(
            &fixture.display,
            &registry,
            &fixture.layout.limits,
        )
        .unwrap();
        plans.plans[0].form_resource_name = "V9".to_owned();
        assert_eq!(
            plans.verify_pdf_closure(&fixture.display, &registry, &fixture.layout.limits),
            Err(StagingSafeVectorResourceV2Error::ReceiptMismatch)
        );
    }
}
