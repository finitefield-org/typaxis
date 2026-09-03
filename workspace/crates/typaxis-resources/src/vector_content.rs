use core::num::NonZeroU32;
use typaxis_core::{push_jcs_string, sha256, ImageResourceId, PortablePath, PositiveLength};
use typaxis_document::{
    ImageMediaDeclaration, ImageMediaType, StagingM4ResourceCatalog, VectorProvenance,
};
use typaxis_resource_admission::{
    close_staging_declared_media, AdmittedResourceLedger, AdmittedSafeVector, SafeVectorAlpha,
    VectorContentKey, VectorContentKeyError, VectorContentMediaType,
};

pub const VECTOR_FORM_DEDUPE_ALGORITHM: &str = "typaxis.vector-form-dedupe/1";

/// Provenance is conditional in the type: Safe-SVG 1 can only carry absence,
/// while Safe-SVG 2 can only carry the producer assertion retained from the
/// declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VectorContentAliasProvenance {
    SafeSvg1Absent,
    SafeSvg2(VectorProvenance),
}

impl VectorContentAliasProvenance {
    pub const fn producer(&self) -> Option<&VectorProvenance> {
        match self {
            Self::SafeSvg1Absent => None,
            Self::SafeSvg2(value) => Some(value),
        }
    }
}

/// Per-logical-resource facts retained even when another alias shares the
/// same content candidate or the resource is never selected for placement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorContentAlias {
    image_id: ImageResourceId,
    uri: PortablePath,
    expected_sha256: Option<[u8; 32]>,
    provenance: VectorContentAliasProvenance,
    admitted_sha256: [u8; 32],
    admission_allocation_charge: u64,
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
}

impl VectorContentAlias {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }

    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }

    pub const fn expected_sha256(&self) -> Option<[u8; 32]> {
        self.expected_sha256
    }

    pub const fn provenance(&self) -> &VectorContentAliasProvenance {
        &self.provenance
    }

    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }

    pub const fn admission_allocation_charge(&self) -> u64 {
        self.admission_allocation_charge
    }

    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }

    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
}

/// One resolved fill/stroke alpha pair in unsigned 16.16 representation.
/// There is intentionally no public raw-value constructor.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VectorExtGStateAlphaPair {
    fill_alpha: u32,
    stroke_alpha: u32,
}

impl VectorExtGStateAlphaPair {
    const OPAQUE: Self = Self {
        fill_alpha: SafeVectorAlpha::OPAQUE.raw(),
        stroke_alpha: SafeVectorAlpha::OPAQUE.raw(),
    };

    pub const fn fill_alpha_raw(self) -> u32 {
        self.fill_alpha
    }

    pub const fn stroke_alpha_raw(self) -> u32 {
        self.stroke_alpha
    }
}

/// ExtGState role within one potential Form plan. Role zero is reserved for
/// the Form XObject; ExtGState roles start at one in numeric alpha-pair order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VectorExtGStatePlanEntry {
    alpha_pair: VectorExtGStateAlphaPair,
    relative_object_role: u32,
}

impl VectorExtGStatePlanEntry {
    pub const fn alpha_pair(&self) -> VectorExtGStateAlphaPair {
        self.alpha_pair
    }

    pub const fn relative_object_role(&self) -> u32 {
        self.relative_object_role
    }
}

/// Deterministic per-candidate alpha plan. It assigns neither a PDF resource
/// name nor an absolute indirect-object number.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorExtGStatePlan {
    entries: Vec<VectorExtGStatePlanEntry>,
    relative_object_role_count_if_selected: u32,
}

impl VectorExtGStatePlan {
    pub const fn form_relative_object_role(&self) -> u32 {
        0
    }

    pub fn entries(&self) -> &[VectorExtGStatePlanEntry] {
        &self.entries
    }

    pub const fn relative_object_role_count_if_selected(&self) -> u32 {
        self.relative_object_role_count_if_selected
    }

    fn from_ir(ir: &AdmittedSafeVector) -> Result<Self, VectorContentPlanningError> {
        let mut pairs = Vec::new();
        let draw_count = match ir {
            AdmittedSafeVector::V1(value) => value.draws().len(),
            AdmittedSafeVector::V2(value) => value.draws().len(),
        };
        pairs
            .try_reserve_exact(draw_count)
            .map_err(|_| VectorContentPlanningError::AllocationFailure)?;
        match ir {
            AdmittedSafeVector::V1(value) => {
                pairs.extend(
                    value
                        .draws()
                        .iter()
                        .map(|_| VectorExtGStateAlphaPair::OPAQUE),
                );
            }
            AdmittedSafeVector::V2(value) => {
                pairs.extend(value.draws().iter().map(|draw| VectorExtGStateAlphaPair {
                    fill_alpha: draw.fill().alpha().raw(),
                    stroke_alpha: draw.stroke().paint().alpha().raw(),
                }));
            }
        }
        Self::from_pairs(pairs)
    }

    fn from_pairs(
        mut pairs: Vec<VectorExtGStateAlphaPair>,
    ) -> Result<Self, VectorContentPlanningError> {
        pairs.sort_unstable();
        pairs.dedup();

        let relative_object_role_count_if_selected = u32::try_from(pairs.len())
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or(VectorContentPlanningError::ObjectRoleCountOverflow)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(pairs.len())
            .map_err(|_| VectorContentPlanningError::AllocationFailure)?;
        for (index, alpha_pair) in pairs.into_iter().enumerate() {
            let relative_object_role = u32::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(VectorContentPlanningError::ObjectRoleCountOverflow)?;
            entries.push(VectorExtGStatePlanEntry {
                alpha_pair,
                relative_object_role,
            });
        }
        Ok(Self {
            entries,
            relative_object_role_count_if_selected,
        })
    }
}

/// One content-level candidate. Its canonical IR and aliases are retained,
/// but no selected usage, Form name, object number, or serialized PDF hash is
/// available at this phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorContentCandidate {
    key: VectorContentKey,
    ir: AdmittedSafeVector,
    intrinsic_width: PositiveLength,
    intrinsic_height: PositiveLength,
    view_box: [i64; 4],
    ext_g_state_plan: VectorExtGStatePlan,
    aliases: Vec<VectorContentAlias>,
}

impl VectorContentCandidate {
    pub const fn key(&self) -> &VectorContentKey {
        &self.key
    }

    pub const fn canonical_ir(&self) -> &AdmittedSafeVector {
        &self.ir
    }

    pub const fn intrinsic_width(&self) -> PositiveLength {
        self.intrinsic_width
    }

    pub const fn intrinsic_height(&self) -> PositiveLength {
        self.intrinsic_height
    }

    pub const fn view_box(&self) -> [i64; 4] {
        self.view_box
    }

    pub const fn ext_g_state_plan(&self) -> &VectorExtGStatePlan {
        &self.ext_g_state_plan
    }

    pub fn aliases(&self) -> &[VectorContentAlias] {
        &self.aliases
    }
}

/// Canonical receipt over all admitted vector aliases and their deduplicated
/// content candidates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorFormDedupeReceipt {
    candidate_count: u32,
    alias_count: u32,
    relative_object_role_count_if_all_candidates_selected: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl VectorFormDedupeReceipt {
    pub const fn algorithm(&self) -> &'static str {
        VECTOR_FORM_DEDUPE_ALGORITHM
    }

    pub const fn candidate_count(&self) -> u32 {
        self.candidate_count
    }

    pub const fn alias_count(&self) -> u32 {
        self.alias_count
    }

    /// Planning-only upper delta when every candidate were selected. It is not
    /// a global PDF-object charge and does not consume `max_pdf_objects`.
    pub const fn relative_object_role_count_if_all_candidates_selected(&self) -> u32 {
        self.relative_object_role_count_if_all_candidates_selected
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// Sealed canonical grouping of admitted SafeVector aliases. Candidate order
/// is the component-wise [`VectorContentKey`] order and alias order is numeric
/// `image_id`, irrespective of owner-private collection or worker order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorContentCandidateRegistry {
    candidates: Vec<VectorContentCandidate>,
    receipt: VectorFormDedupeReceipt,
}

impl VectorContentCandidateRegistry {
    pub fn from_admitted(
        admitted: &AdmittedResourceLedger,
        declarations: &StagingM4ResourceCatalog,
    ) -> Result<Self, VectorContentPlanningError> {
        close_staging_declared_media(admitted, declarations)
            .map_err(|_| VectorContentPlanningError::DeclarationMismatch)?;
        canonicalize_candidates(prepare_aliases(admitted, declarations)?)
    }

    pub fn candidates(&self) -> &[VectorContentCandidate] {
        &self.candidates
    }

    pub fn candidate(&self, key: &VectorContentKey) -> Option<&VectorContentCandidate> {
        self.candidates
            .binary_search_by(|candidate| candidate.key.cmp(key))
            .ok()
            .map(|index| &self.candidates[index])
    }

    pub fn candidate_for_alias(
        &self,
        image_id: ImageResourceId,
    ) -> Option<&VectorContentCandidate> {
        self.candidates.iter().find(|candidate| {
            candidate
                .aliases
                .binary_search_by_key(&image_id, VectorContentAlias::image_id)
                .is_ok()
        })
    }

    pub const fn receipt(&self) -> &VectorFormDedupeReceipt {
        &self.receipt
    }

    pub fn checked_relative_object_role_count_if_all_candidates_selected(
        &self,
    ) -> Result<u32, VectorContentPlanningError> {
        checked_relative_object_role_count(&self.candidates)
    }

    pub fn verify(
        &self,
        admitted: &AdmittedResourceLedger,
        declarations: &StagingM4ResourceCatalog,
    ) -> Result<(), VectorContentPlanningError> {
        let expected = Self::from_admitted(admitted, declarations)?;
        if self != &expected {
            return Err(VectorContentPlanningError::ReceiptMismatch);
        }
        Ok(())
    }
}

/// Reserved join shape for MI4-V13. No constructor exists before selected
/// Display `/2` can supply authoritative per-alias usage counts.
#[allow(dead_code)]
struct SelectedVectorContentCandidateInput<'a> {
    candidate: &'a VectorContentCandidate,
    selected_alias_usage_counts: &'a [(ImageResourceId, NonZeroU32)],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VectorContentPlanningError {
    DeclarationMismatch,
    WrongMedia(ImageResourceId),
    MissingExpectedHash(ImageResourceId),
    MissingProvenance(ImageResourceId),
    UnexpectedProvenance(ImageResourceId),
    MissingLimitsIdentity(ImageResourceId),
    MissingProfileIdentity(ImageResourceId),
    DuplicateAlias(ImageResourceId),
    ConflictingAlias(ImageResourceId),
    ContentConflict(VectorContentKey),
    ObjectRoleCountOverflow,
    AllocationFailure,
    ReceiptMismatch,
}

impl From<VectorContentKeyError> for VectorContentPlanningError {
    fn from(value: VectorContentKeyError) -> Self {
        match value {
            VectorContentKeyError::WrongMedia(image_id) => Self::WrongMedia(image_id),
        }
    }
}

impl std::fmt::Display for VectorContentPlanningError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeclarationMismatch => {
                formatter.write_str("I9190: vector declaration/admission mismatch")
            }
            Self::WrongMedia(id) => {
                write!(formatter, "I9190: image {} is not SafeVector", id.get())
            }
            Self::MissingExpectedHash(id) => write!(
                formatter,
                "I9190: Safe-SVG 2 image {} has no expected hash",
                id.get()
            ),
            Self::MissingProvenance(id) => write!(
                formatter,
                "I9190: Safe-SVG 2 image {} has no producer provenance",
                id.get()
            ),
            Self::UnexpectedProvenance(id) => write!(
                formatter,
                "I9190: Safe-SVG 1 image {} has producer provenance",
                id.get()
            ),
            Self::MissingLimitsIdentity(id) => write!(
                formatter,
                "I9190: vector {} has no admitted limits identity",
                id.get()
            ),
            Self::MissingProfileIdentity(id) => write!(
                formatter,
                "I9190: vector {} has no admitted profile identity",
                id.get()
            ),
            Self::DuplicateAlias(id) => {
                write!(formatter, "I9190: duplicate vector alias {}", id.get())
            }
            Self::ConflictingAlias(id) => {
                write!(formatter, "I9190: conflicting vector alias {}", id.get())
            }
            Self::ContentConflict(_) => {
                formatter.write_str("I9190: one vector content key has conflicting admitted IR")
            }
            Self::ObjectRoleCountOverflow => {
                formatter.write_str("D8101: vector relative object-role count overflow")
            }
            Self::AllocationFailure => {
                formatter.write_str("D8101: vector content planning allocation failed")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: vector content candidate receipt mismatch")
            }
        }
    }
}

impl std::error::Error for VectorContentPlanningError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreparedAlias {
    key: VectorContentKey,
    ir: AdmittedSafeVector,
    intrinsic_width: PositiveLength,
    intrinsic_height: PositiveLength,
    view_box: [i64; 4],
    ext_g_state_plan: VectorExtGStatePlan,
    alias: VectorContentAlias,
}

fn prepare_aliases(
    admitted: &AdmittedResourceLedger,
    declarations: &StagingM4ResourceCatalog,
) -> Result<Vec<PreparedAlias>, VectorContentPlanningError> {
    let mut prepared = Vec::new();
    prepared
        .try_reserve_exact(declarations.images.len())
        .map_err(|_| VectorContentPlanningError::AllocationFailure)?;
    for declaration in &declarations.images {
        let ImageMediaDeclaration::Declared(declared_media) = declaration.media else {
            return Err(VectorContentPlanningError::DeclarationMismatch);
        };
        if declared_media == ImageMediaType::Png {
            continue;
        }
        let image = admitted
            .image(declaration.image_id)
            .ok_or(VectorContentPlanningError::DeclarationMismatch)?;
        let ir = image
            .admitted_safe_vector()
            .cloned()
            .ok_or(VectorContentPlanningError::WrongMedia(declaration.image_id))?;
        let key = VectorContentKey::from_admitted(image)?;
        let provenance = match (key.media_type(), declaration.vector_provenance.as_ref()) {
            (VectorContentMediaType::SafeSvg1, None) => {
                VectorContentAliasProvenance::SafeSvg1Absent
            }
            (VectorContentMediaType::SafeSvg1, Some(_)) => {
                return Err(VectorContentPlanningError::UnexpectedProvenance(
                    declaration.image_id,
                ));
            }
            (VectorContentMediaType::SafeSvg2, Some(value)) => {
                VectorContentAliasProvenance::SafeSvg2(value.clone())
            }
            (VectorContentMediaType::SafeSvg2, None) => {
                return Err(VectorContentPlanningError::MissingProvenance(
                    declaration.image_id,
                ));
            }
        };
        if key.media_type() == VectorContentMediaType::SafeSvg2
            && declaration.expected_sha256.is_none()
        {
            return Err(VectorContentPlanningError::MissingExpectedHash(
                declaration.image_id,
            ));
        }
        let limits_fingerprint = image.m4_limits_fingerprint().ok_or(
            VectorContentPlanningError::MissingLimitsIdentity(declaration.image_id),
        )?;
        let profile_fingerprint = image.m4_profile_fingerprint().ok_or(
            VectorContentPlanningError::MissingProfileIdentity(declaration.image_id),
        )?;
        let ext_g_state_plan = VectorExtGStatePlan::from_ir(&ir)?;
        prepared.push(PreparedAlias {
            key,
            intrinsic_width: ir.intrinsic_width(),
            intrinsic_height: ir.intrinsic_height(),
            view_box: ir.view_box(),
            ext_g_state_plan,
            alias: VectorContentAlias {
                image_id: declaration.image_id,
                uri: declaration.uri.clone(),
                expected_sha256: declaration.expected_sha256,
                provenance,
                admitted_sha256: image.content_hash(),
                admission_allocation_charge: ir.allocation_charge(),
                profile_fingerprint,
                limits_fingerprint,
            },
            ir,
        });
    }
    Ok(prepared)
}

fn canonicalize_candidates(
    mut prepared: Vec<PreparedAlias>,
) -> Result<VectorContentCandidateRegistry, VectorContentPlanningError> {
    prepared.sort_unstable_by_key(|item| item.alias.image_id);
    for pair in prepared.windows(2) {
        if pair[0].alias.image_id == pair[1].alias.image_id {
            return Err(if pair[0] == pair[1] {
                VectorContentPlanningError::DuplicateAlias(pair[0].alias.image_id)
            } else {
                VectorContentPlanningError::ConflictingAlias(pair[0].alias.image_id)
            });
        }
    }
    prepared.sort_unstable_by(|left, right| {
        left.key
            .cmp(&right.key)
            .then_with(|| left.alias.image_id.cmp(&right.alias.image_id))
    });

    let alias_count = u32::try_from(prepared.len())
        .map_err(|_| VectorContentPlanningError::ObjectRoleCountOverflow)?;
    let mut candidates: Vec<VectorContentCandidate> = Vec::new();
    candidates
        .try_reserve_exact(prepared.len())
        .map_err(|_| VectorContentPlanningError::AllocationFailure)?;
    for item in prepared {
        if let Some(candidate) = candidates.last_mut().filter(|value| value.key == item.key) {
            if candidate.ir != item.ir
                || candidate.intrinsic_width != item.intrinsic_width
                || candidate.intrinsic_height != item.intrinsic_height
                || candidate.view_box != item.view_box
                || candidate.ext_g_state_plan != item.ext_g_state_plan
            {
                return Err(VectorContentPlanningError::ContentConflict(item.key));
            }
            candidate
                .aliases
                .try_reserve(1)
                .map_err(|_| VectorContentPlanningError::AllocationFailure)?;
            candidate.aliases.push(item.alias);
            continue;
        }
        let mut aliases = Vec::new();
        aliases
            .try_reserve_exact(1)
            .map_err(|_| VectorContentPlanningError::AllocationFailure)?;
        aliases.push(item.alias);
        candidates.push(VectorContentCandidate {
            key: item.key,
            ir: item.ir,
            intrinsic_width: item.intrinsic_width,
            intrinsic_height: item.intrinsic_height,
            view_box: item.view_box,
            ext_g_state_plan: item.ext_g_state_plan,
            aliases,
        });
    }
    let candidate_count = u32::try_from(candidates.len())
        .map_err(|_| VectorContentPlanningError::ObjectRoleCountOverflow)?;
    let relative_object_role_count_if_all_candidates_selected =
        checked_relative_object_role_count(&candidates)?;
    let canonical_jcs = encode_receipt(
        candidate_count,
        alias_count,
        relative_object_role_count_if_all_candidates_selected,
        &candidates,
    );
    let receipt = VectorFormDedupeReceipt {
        candidate_count,
        alias_count,
        relative_object_role_count_if_all_candidates_selected,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    Ok(VectorContentCandidateRegistry {
        candidates,
        receipt,
    })
}

fn checked_relative_object_role_count(
    candidates: &[VectorContentCandidate],
) -> Result<u32, VectorContentPlanningError> {
    candidates.iter().try_fold(0u32, |total, candidate| {
        total
            .checked_add(
                candidate
                    .ext_g_state_plan
                    .relative_object_role_count_if_selected,
            )
            .ok_or(VectorContentPlanningError::ObjectRoleCountOverflow)
    })
}

fn encode_receipt(
    candidate_count: u32,
    alias_count: u32,
    relative_object_role_count: u32,
    candidates: &[VectorContentCandidate],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, VECTOR_FORM_DEDUPE_ALGORITHM);
    output.push_str(",\"alias_count\":");
    output.push_str(&alias_count.to_string());
    output.push_str(",\"candidate_count\":");
    output.push_str(&candidate_count.to_string());
    output.push_str(",\"candidates\":[");
    for (index, candidate) in candidates.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_candidate(&mut output, candidate);
    }
    output.push_str("],\"relative_object_role_count_if_all_candidates_selected\":");
    output.push_str(&relative_object_role_count.to_string());
    output.push('}');
    output
}

fn encode_candidate(output: &mut String, candidate: &VectorContentCandidate) {
    output.push_str("{\"aliases\":[");
    for (index, alias) in candidate.aliases.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        encode_alias(output, alias);
    }
    output.push_str("],\"ext_g_state_plan\":[");
    for (index, entry) in candidate.ext_g_state_plan.entries.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"fill_alpha\":");
        output.push_str(&entry.alpha_pair.fill_alpha.to_string());
        output.push_str(",\"relative_object_role\":");
        output.push_str(&entry.relative_object_role.to_string());
        output.push_str(",\"stroke_alpha\":");
        output.push_str(&entry.alpha_pair.stroke_alpha.to_string());
        output.push('}');
    }
    output.push_str("],\"intrinsic_height\":");
    output.push_str(&candidate.intrinsic_height.get().raw().to_string());
    output.push_str(",\"intrinsic_width\":");
    output.push_str(&candidate.intrinsic_width.get().raw().to_string());
    output.push_str(",\"key\":");
    encode_key(output, &candidate.key);
    output.push_str(",\"relative_object_role_count_if_selected\":");
    output.push_str(
        &candidate
            .ext_g_state_plan
            .relative_object_role_count_if_selected
            .to_string(),
    );
    output.push_str(",\"view_box\":[");
    for (index, component) in candidate.view_box.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&component.to_string());
    }
    output.push_str("]}");
}

fn encode_key(output: &mut String, key: &VectorContentKey) {
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

fn encode_alias(output: &mut String, alias: &VectorContentAlias) {
    output.push_str("{\"admission_allocation_charge\":");
    output.push_str(&alias.admission_allocation_charge.to_string());
    output.push_str(",\"admitted_sha256\":");
    push_hash(output, alias.admitted_sha256);
    output.push_str(",\"expected_sha256\":");
    push_optional_hash(output, alias.expected_sha256);
    output.push_str(",\"image_id\":");
    output.push_str(&alias.image_id.get().to_string());
    output.push_str(",\"limits_fingerprint\":");
    push_hash(output, alias.limits_fingerprint);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(output, alias.profile_fingerprint);
    if let VectorContentAliasProvenance::SafeSvg2(value) = &alias.provenance {
        output.push_str(",\"provenance\":");
        output.push_str("{\"engine_id\":");
        push_jcs_string(output, &value.engine_id);
        output.push_str(",\"engine_version\":");
        push_jcs_string(output, &value.engine_version);
        output.push_str(",\"rules_version\":");
        push_jcs_string(output, &value.rules_version);
        output.push('}');
    }
    output.push_str(",\"uri\":");
    push_jcs_string(output, alias.uri.as_str());
    output.push('}');
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };
    use typaxis_core::{
        ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, HostAdmissionContext, HostPath,
        M4EffectiveResourceLimits, M4ResourceLimits, PdfStreamCompression, ResourceLimits,
        ValidatedResourceLimits, DEFAULT_ALLOWED_URI_SCHEMES,
    };
    use typaxis_document::StagingM4ImageDeclaration;
    use typaxis_resource_admission::{
        staging_declared_base_catalog, AdmittedResourceResolver, HostResourceAdmissionSession,
        SAFE_SVG_PARSER_ID, SAFE_SVG_PARSER_ID_V2, SAFE_VECTOR_IR_ID, SAFE_VECTOR_IR_ID_V2,
    };

    const TEST_PROFILE_FINGERPRINT: [u8; 32] = [0x5a; 32];

    struct TempTree {
        path: PathBuf,
    }

    impl TempTree {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "typaxis-vector-content-{}-{label}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn workspace_sample(path: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join(path)
    }

    fn provenance(version: &str) -> VectorProvenance {
        VectorProvenance {
            engine_id: "vmb.texToSvg".to_owned(),
            engine_version: version.to_owned(),
            rules_version: "vmb.math-safe-svg/1".to_owned(),
        }
    }

    fn declaration(
        image_id: u32,
        uri: &str,
        bytes: &[u8],
        media: ImageMediaType,
        vector_provenance: Option<VectorProvenance>,
    ) -> StagingM4ImageDeclaration {
        StagingM4ImageDeclaration {
            image_id: ImageResourceId::new(image_id),
            uri: PortablePath::new(uri).unwrap(),
            expected_sha256: Some(sha256(bytes)),
            media: ImageMediaDeclaration::Declared(media),
            vector_provenance,
        }
    }

    fn admit_catalog(
        project_root: &Path,
        declarations: &StagingM4ResourceCatalog,
    ) -> AdmittedResourceLedger {
        let base_limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let limits =
            M4EffectiveResourceLimits::new(base_limits, M4ResourceLimits::default()).unwrap();
        let base = staging_declared_base_catalog(declarations).unwrap();
        let config = EffectiveConfig::new(
            true,
            PdfStreamCompression::None,
            vec![ConfigResourceRoot::ProjectRoot],
            DEFAULT_ALLOWED_URI_SCHEMES
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            ResourceLimits::default(),
        )
        .unwrap();
        let document_path = project_root.join("document-package.json");
        let context = HostAdmissionContext::new(
            HostPath::new(document_path).unwrap(),
            HostPath::new(project_root.to_path_buf()).unwrap(),
            None,
            Vec::new(),
        );
        let session = HostResourceAdmissionSession::new(&context, &config, &base).unwrap();
        let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &base,
            &limits,
            TEST_PROFILE_FINGERPRINT,
            session.roots(),
        )
        .unwrap();
        for declaration in &declarations.images {
            let pending = resolver
                .read_image(session.open_image(declaration.image_id).unwrap())
                .unwrap();
            resolver.parse_and_bind_declared_image(pending).unwrap();
        }
        resolver.finish().unwrap()
    }

    fn v2_alias_fixture() -> (AdmittedResourceLedger, StagingM4ResourceCatalog) {
        let root = workspace_sample(
            "samples/machine-package/staging/production-book-1/precomposed-vector",
        );
        let uri = "svg/fraction-equality.svg";
        let bytes = std::fs::read(root.join(uri)).unwrap();
        let declarations = StagingM4ResourceCatalog {
            font_faces: Vec::new(),
            images: vec![
                declaration(
                    0,
                    uri,
                    &bytes,
                    ImageMediaType::SvgSafe2,
                    Some(provenance("2026.09.0")),
                ),
                declaration(
                    1,
                    uri,
                    &bytes,
                    ImageMediaType::SvgSafe2,
                    Some(provenance("2026.09.1")),
                ),
            ],
        };
        let admitted = admit_catalog(&root, &declarations);
        (admitted, declarations)
    }

    fn same_ir_different_source_fixture() -> (AdmittedResourceLedger, StagingM4ResourceCatalog) {
        let sample_root = workspace_sample(
            "samples/machine-package/staging/production-book-1/precomposed-vector",
        );
        let source =
            std::fs::read_to_string(sample_root.join("svg/fraction-equality.svg")).unwrap();
        let alternate = source.replacen(
            "width=\"70pt\" height=\"24pt\"",
            "height=\"24pt\" width=\"70pt\"",
            1,
        );
        assert_ne!(source, alternate);
        let tree = TempTree::new("same-ir-different-source");
        std::fs::write(tree.path().join("first.svg"), source.as_bytes()).unwrap();
        std::fs::write(tree.path().join("second.svg"), alternate.as_bytes()).unwrap();
        let declarations = StagingM4ResourceCatalog {
            font_faces: Vec::new(),
            images: vec![
                declaration(
                    0,
                    "first.svg",
                    source.as_bytes(),
                    ImageMediaType::SvgSafe2,
                    Some(provenance("2026.09.0")),
                ),
                declaration(
                    1,
                    "second.svg",
                    alternate.as_bytes(),
                    ImageMediaType::SvgSafe2,
                    Some(provenance("2026.09.0")),
                ),
            ],
        };
        let admitted = admit_catalog(tree.path(), &declarations);
        (admitted, declarations)
    }

    fn cross_media_fixture() -> (AdmittedResourceLedger, StagingM4ResourceCatalog) {
        let root =
            workspace_sample("samples/machine-package/staging/production-book-1/vector-media/job");
        let uri = "art.vector";
        let bytes = std::fs::read(root.join(uri)).unwrap();
        let declarations = StagingM4ResourceCatalog {
            font_faces: Vec::new(),
            images: vec![
                declaration(0, uri, &bytes, ImageMediaType::SvgSafe1, None),
                declaration(
                    1,
                    uri,
                    &bytes,
                    ImageMediaType::SvgSafe2,
                    Some(provenance("2026.09.0")),
                ),
            ],
        };
        let admitted = admit_catalog(&root, &declarations);
        (admitted, declarations)
    }

    #[test]
    fn vector_content_key_is_issued_from_exact_admitted_identity() {
        let fixture = crate::staging_safe_vector_resource_fixture().unwrap();
        let image = fixture
            .display
            .layout
            .admitted
            .image(ImageResourceId::new(0))
            .unwrap();
        let vector = image.admitted_safe_vector().unwrap();
        let key = VectorContentKey::from_admitted(image).unwrap();
        assert_eq!(key.source_sha256(), image.content_hash());
        assert_eq!(key.media_type(), VectorContentMediaType::SafeSvg1);
        assert_eq!(key.parser_id(), SAFE_SVG_PARSER_ID);
        assert_eq!(key.ir_id(), SAFE_VECTOR_IR_ID);
        assert_eq!(key.ir_fingerprint(), vector.fingerprint());
    }

    #[test]
    fn vector_content_candidates_group_same_key_and_retain_alias_provenance() {
        let (admitted, declarations) = v2_alias_fixture();
        let registry =
            VectorContentCandidateRegistry::from_admitted(&admitted, &declarations).unwrap();
        assert_eq!(registry.receipt().candidate_count(), 1);
        assert_eq!(registry.receipt().alias_count(), 2);
        let candidate = &registry.candidates()[0];
        assert_eq!(candidate.aliases().len(), 2);
        assert_eq!(candidate.aliases()[0].image_id(), ImageResourceId::new(0));
        assert_eq!(candidate.aliases()[1].image_id(), ImageResourceId::new(1));
        assert_eq!(candidate.aliases()[0].uri(), &declarations.images[0].uri);
        assert_eq!(
            candidate.aliases()[0].expected_sha256(),
            declarations.images[0].expected_sha256
        );
        assert_eq!(
            candidate.aliases()[0].admitted_sha256(),
            admitted
                .image(ImageResourceId::new(0))
                .unwrap()
                .content_hash()
        );
        assert_eq!(
            candidate.aliases()[0].profile_fingerprint(),
            TEST_PROFILE_FINGERPRINT
        );
        assert_eq!(
            candidate.aliases()[0].limits_fingerprint(),
            admitted
                .image(ImageResourceId::new(0))
                .unwrap()
                .m4_limits_fingerprint()
                .unwrap()
        );
        assert_ne!(
            candidate.aliases()[0]
                .provenance()
                .producer()
                .unwrap()
                .engine_version,
            candidate.aliases()[1]
                .provenance()
                .producer()
                .unwrap()
                .engine_version
        );
        assert_eq!(
            candidate.aliases()[0].admission_allocation_charge(),
            candidate.aliases()[1].admission_allocation_charge()
        );
        assert_eq!(registry.receipt().algorithm(), VECTOR_FORM_DEDUPE_ALGORITHM);
        assert_eq!(
            registry.receipt().fingerprint(),
            sha256(registry.receipt().canonical_jcs().as_bytes())
        );
        registry.verify(&admitted, &declarations).unwrap();
    }

    #[test]
    fn vector_content_candidates_enforce_conditional_hash_and_provenance() {
        let (admitted, declarations) = v2_alias_fixture();

        let mut missing_hash = declarations.clone();
        missing_hash.images[0].expected_sha256 = None;
        assert_eq!(
            VectorContentCandidateRegistry::from_admitted(&admitted, &missing_hash),
            Err(VectorContentPlanningError::MissingExpectedHash(
                ImageResourceId::new(0)
            ))
        );

        let mut missing_provenance = declarations;
        missing_provenance.images[0].vector_provenance = None;
        assert_eq!(
            VectorContentCandidateRegistry::from_admitted(&admitted, &missing_provenance),
            Err(VectorContentPlanningError::MissingProvenance(
                ImageResourceId::new(0)
            ))
        );

        let fixture = crate::staging_safe_vector_resource_fixture().unwrap();
        let mut unexpected_provenance = fixture.display.layout.package.resources().clone();
        unexpected_provenance.images[0].vector_provenance = Some(provenance("2026.09.0"));
        assert_eq!(
            VectorContentCandidateRegistry::from_admitted(
                &fixture.display.layout.admitted,
                &unexpected_provenance,
            ),
            Err(VectorContentPlanningError::UnexpectedProvenance(
                ImageResourceId::new(0)
            ))
        );
    }

    #[test]
    fn vector_content_candidates_keep_unused_alias_facts_without_usage_or_form() {
        let fixture = crate::staging_safe_vector_resource_fixture().unwrap();
        let registry = VectorContentCandidateRegistry::from_admitted(
            &fixture.display.layout.admitted,
            fixture.display.layout.package.resources(),
        )
        .unwrap();
        assert_eq!(registry.receipt().alias_count(), 2);
        assert!(registry
            .candidate_for_alias(ImageResourceId::new(1))
            .is_some());
        assert!(fixture.plans.plan(ImageResourceId::new(1)).is_none());
        assert!(!registry.receipt().canonical_jcs().contains("usage"));
        assert!(!registry.receipt().canonical_jcs().contains("form_name"));
        assert!(!registry.receipt().canonical_jcs().contains("object_number"));
        assert!(!registry
            .receipt()
            .canonical_jcs()
            .contains("max_pdf_objects"));
        let alias = &registry
            .candidate_for_alias(ImageResourceId::new(0))
            .unwrap()
            .aliases()[0];
        let mut encoded_alias = String::new();
        encode_alias(&mut encoded_alias, alias);
        assert!(!encoded_alias.contains("\"provenance\""));
    }

    #[test]
    fn vector_content_candidates_reject_same_id_with_different_content() {
        let fixture = crate::staging_safe_vector_resource_fixture().unwrap();
        let mut prepared = prepare_aliases(
            &fixture.display.layout.admitted,
            fixture.display.layout.package.resources(),
        )
        .unwrap();
        assert_ne!(prepared[0].key, prepared[1].key);
        prepared[1].alias.image_id = prepared[0].alias.image_id;
        assert!(matches!(
            canonicalize_candidates(prepared),
            Err(VectorContentPlanningError::ConflictingAlias(id))
                if id == ImageResourceId::new(0)
        ));
    }

    #[test]
    fn vector_content_candidates_do_not_merge_same_ir_with_different_source_hash() {
        let (admitted, declarations) = same_ir_different_source_fixture();
        let prepared = prepare_aliases(&admitted, &declarations).unwrap();
        assert_eq!(prepared[0].ir, prepared[1].ir);
        assert_eq!(
            prepared[0].key.ir_fingerprint(),
            prepared[1].key.ir_fingerprint()
        );
        assert_ne!(
            prepared[0].key.source_sha256(),
            prepared[1].key.source_sha256()
        );
        let registry = canonicalize_candidates(prepared).unwrap();
        assert_eq!(registry.candidates().len(), 2);
        assert_eq!(
            registry.candidates()[0].canonical_ir(),
            registry.candidates()[1].canonical_ir()
        );
    }

    #[test]
    fn vector_content_candidates_separate_same_hash_under_different_media_identity() {
        let (admitted, declarations) = cross_media_fixture();
        let registry =
            VectorContentCandidateRegistry::from_admitted(&admitted, &declarations).unwrap();
        assert_eq!(registry.candidates().len(), 2);
        let first = registry
            .candidate_for_alias(ImageResourceId::new(0))
            .unwrap();
        let second = registry
            .candidate_for_alias(ImageResourceId::new(1))
            .unwrap();
        assert_eq!(first.key().source_sha256(), second.key().source_sha256());
        assert_eq!(first.key().media_type(), VectorContentMediaType::SafeSvg1);
        assert_eq!(second.key().media_type(), VectorContentMediaType::SafeSvg2);
        assert_eq!(first.key().parser_id(), SAFE_SVG_PARSER_ID);
        assert_eq!(second.key().parser_id(), SAFE_SVG_PARSER_ID_V2);
        assert_eq!(first.key().ir_id(), SAFE_VECTOR_IR_ID);
        assert_eq!(second.key().ir_id(), SAFE_VECTOR_IR_ID_V2);
    }

    #[test]
    fn vector_content_candidates_are_worker_and_collection_order_independent() {
        let (admitted, declarations) = v2_alias_fixture();
        let forward_input = prepare_aliases(&admitted, &declarations).unwrap();
        let mut reverse_input = forward_input.clone();
        reverse_input.reverse();
        let forward = canonicalize_candidates(forward_input).unwrap();
        let reverse = canonicalize_candidates(reverse_input).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(
            forward.receipt().canonical_jcs(),
            reverse.receipt().canonical_jcs()
        );
        assert_eq!(
            forward.receipt().fingerprint(),
            reverse.receipt().fingerprint()
        );

        let (admitted, declarations) = cross_media_fixture();
        let forward_input = prepare_aliases(&admitted, &declarations).unwrap();
        let mut reverse_input = forward_input.clone();
        reverse_input.reverse();
        assert_eq!(
            canonicalize_candidates(forward_input).unwrap(),
            canonicalize_candidates(reverse_input).unwrap()
        );
    }

    #[test]
    fn vector_ext_gstate_plan_is_unique_numeric_and_includes_opaque() {
        let (admitted, declarations) = v2_alias_fixture();
        let registry =
            VectorContentCandidateRegistry::from_admitted(&admitted, &declarations).unwrap();
        let plan = registry.candidates()[0].ext_g_state_plan();
        let observed: Vec<_> = plan
            .entries()
            .iter()
            .map(|entry| {
                (
                    entry.alpha_pair().fill_alpha_raw(),
                    entry.alpha_pair().stroke_alpha_raw(),
                    entry.relative_object_role(),
                )
            })
            .collect();
        assert_eq!(observed, vec![(49_152, 32_768, 1)]);
        assert_eq!(plan.form_relative_object_role(), 0);
        assert_eq!(plan.relative_object_role_count_if_selected(), 2);
        assert_eq!(
            registry
                .checked_relative_object_role_count_if_all_candidates_selected()
                .unwrap(),
            2
        );
        assert_eq!(
            registry
                .receipt()
                .relative_object_role_count_if_all_candidates_selected(),
            2
        );

        let plan = VectorExtGStatePlan::from_pairs(vec![
            VectorExtGStateAlphaPair::OPAQUE,
            VectorExtGStateAlphaPair {
                fill_alpha: 49_152,
                stroke_alpha: 32_768,
            },
            VectorExtGStateAlphaPair::OPAQUE,
        ])
        .unwrap();
        let observed: Vec<_> = plan
            .entries()
            .iter()
            .map(|entry| {
                (
                    entry.alpha_pair().fill_alpha_raw(),
                    entry.alpha_pair().stroke_alpha_raw(),
                    entry.relative_object_role(),
                )
            })
            .collect();
        assert_eq!(observed, vec![(49_152, 32_768, 1), (65_536, 65_536, 2)]);
        assert_eq!(plan.relative_object_role_count_if_selected(), 3);
    }
}
