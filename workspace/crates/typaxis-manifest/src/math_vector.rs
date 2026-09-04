use typaxis_core::{push_jcs_string, sha256, NodeId};
use typaxis_layout::{PrecomposedMathVectorKind, ValidatedPrecomposedVectorBindings};
use typaxis_syntax::{PrecomposedVectorKind, ValidatedStagingSemanticPackage};

use crate::{
    StagingSafeVectorManifestV2, StagingSafeVectorManifestV2Error,
    StagingSafeVectorPlacementDetailsV2, StagingVectorMetricFactV2,
};

pub const STAGING_MATH_VECTOR_MANIFEST_ALGORITHM: &str = "typaxis.math-vector-manifest/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathVectorManifestFact {
    node_id: NodeId,
    kind: PrecomposedMathVectorKind,
    owner_source_id: u32,
    owner_source_start: u32,
    owner_source_end: u32,
    text_buffer_id: u32,
    text_start: u32,
    text_end: u32,
    mapped_source_id: u32,
    mapped_source_start: u32,
    mapped_source_end: u32,
    text_buffer_sha256: [u8; 32],
    source_tex_sha256: [u8; 32],
    alternative_sha256: [u8; 32],
    resolved_actual_text_sha256: [u8; 32],
    language: String,
    metrics: StagingVectorMetricFactV2,
    common_binding_fingerprint: [u8; 32],
    math_binding_fingerprint: [u8; 32],
    selected_placement_fingerprint: [u8; 32],
    display_command_fingerprint: [u8; 32],
    safe_vector_usage_fingerprint: [u8; 32],
    pdf_use_fingerprint: [u8; 32],
    placement_details_jcs: String,
    equation_number_jcs: Option<String>,
    provenance_engine_id: String,
    provenance_engine_version: String,
    provenance_rules_version: String,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathVectorManifestFact {
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn kind(&self) -> PrecomposedMathVectorKind {
        self.kind
    }
    pub const fn source_tex_sha256(&self) -> [u8; 32] {
        self.source_tex_sha256
    }
    pub const fn math_binding_fingerprint(&self) -> [u8; 32] {
        self.math_binding_fingerprint
    }
    pub const fn safe_vector_usage_fingerprint(&self) -> [u8; 32] {
        self.safe_vector_usage_fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathVectorManifest {
    facts: Vec<StagingMathVectorManifestFact>,
    package_fingerprint: [u8; 32],
    binding_set_fingerprint: [u8; 32],
    safe_vector_manifest_fingerprint: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathVectorManifest {
    pub fn facts(&self) -> &[StagingMathVectorManifestFact] {
        &self.facts
    }
    pub fn fact(&self, owner: NodeId) -> Option<&StagingMathVectorManifestFact> {
        self.facts
            .binary_search_by_key(&owner, StagingMathVectorManifestFact::node_id)
            .ok()
            .map(|index| &self.facts[index])
    }
    pub const fn safe_vector_manifest_fingerprint(&self) -> [u8; 32] {
        self.safe_vector_manifest_fingerprint
    }
    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package_fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMathVectorManifestError {
    PackageMismatch,
    BindingMismatch,
    SafeVectorMismatch,
    KindMismatch,
    CountOverflow,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingMathVectorManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "I9190: math-vector manifest {:?}", self)
    }
}
impl std::error::Error for StagingMathVectorManifestError {}

pub fn build_staging_math_vector_manifest(
    package: &ValidatedStagingSemanticPackage,
    bindings: &ValidatedPrecomposedVectorBindings,
    safe_vector: &StagingSafeVectorManifestV2,
) -> Result<StagingMathVectorManifest, StagingMathVectorManifestError> {
    if bindings.epoch().semantic_fingerprint() != package.semantic_fingerprint()
        || safe_vector.package_fingerprint() != package.semantic_fingerprint()
    {
        return Err(StagingMathVectorManifestError::PackageMismatch);
    }
    let mut facts = Vec::new();
    facts
        .try_reserve_exact(bindings.math_receipts().len())
        .map_err(|_| StagingMathVectorManifestError::AllocationFailure)?;
    for math in bindings.math_receipts() {
        let common = bindings
            .receipt(math.node_id())
            .ok_or(StagingMathVectorManifestError::BindingMismatch)?;
        if common.fingerprint() != math.common_fingerprint()
            || !matches!(
                (math.kind(), common.kind()),
                (
                    PrecomposedMathVectorKind::Inline,
                    PrecomposedVectorKind::MathVector
                ) | (
                    PrecomposedMathVectorKind::Block,
                    PrecomposedVectorKind::MathVectorBlock
                )
            )
        {
            return Err(StagingMathVectorManifestError::KindMismatch);
        }
        let placement = safe_vector
            .resources()
            .iter()
            .flat_map(|resource| resource.placements())
            .find(|placement| placement.owner() == math.node_id())
            .ok_or(StagingMathVectorManifestError::SafeVectorMismatch)?;
        if placement.binding_fingerprint() != Some(common.fingerprint())
            || placement.kind() != common.kind().into()
            || placement.details().metrics().is_none()
        {
            return Err(StagingMathVectorManifestError::SafeVectorMismatch);
        }
        let syntax = package
            .precomposed_vector_metrics_for(math.node_id())
            .ok_or(StagingMathVectorManifestError::PackageMismatch)?;
        if syntax.fingerprint() != common.metrics_fingerprint()
            || syntax.alternative().alternative_sha256() != common.alternative_sha256()
        {
            return Err(StagingMathVectorManifestError::PackageMismatch);
        }
        let source = math.source();
        let owner_span = common.owner_source_span();
        let mapped = source.mapped_source_span();
        let text = source.text_span();
        let equation_number_jcs = match math.kind() {
            PrecomposedMathVectorKind::Inline => {
                if syntax.equation_number().is_some() {
                    return Err(StagingMathVectorManifestError::KindMismatch);
                }
                None
            }
            PrecomposedMathVectorKind::Block => {
                syntax.equation_number().map(encode_equation_number)
            }
        };
        let placement_details_jcs =
            encode_math_placement_details(placement.details(), math.kind())?;
        let provenance = math.provenance();
        let mut fact = StagingMathVectorManifestFact {
            node_id: math.node_id(),
            kind: math.kind(),
            owner_source_id: owner_span.source_id().get(),
            owner_source_start: owner_span.start_byte().get(),
            owner_source_end: owner_span.end_byte().get(),
            text_buffer_id: text.text_id().get(),
            text_start: text.start_byte().get(),
            text_end: text.end_byte().get(),
            mapped_source_id: mapped.source_id().get(),
            mapped_source_start: mapped.start_byte().get(),
            mapped_source_end: mapped.end_byte().get(),
            text_buffer_sha256: source.text_buffer_sha256(),
            source_tex_sha256: source.exact_slice_sha256(),
            alternative_sha256: common.alternative_sha256(),
            resolved_actual_text_sha256: math.resolved_actual_text_sha256(),
            language: placement.language().to_owned(),
            metrics: placement
                .details()
                .metrics()
                .ok_or(StagingMathVectorManifestError::SafeVectorMismatch)?,
            common_binding_fingerprint: common.fingerprint(),
            math_binding_fingerprint: math.fingerprint(),
            selected_placement_fingerprint: placement.selected_placement_fingerprint(),
            display_command_fingerprint: placement.display_command_fingerprint(),
            safe_vector_usage_fingerprint: placement.fingerprint(),
            pdf_use_fingerprint: placement.pdf_use_fingerprint(),
            placement_details_jcs,
            equation_number_jcs,
            provenance_engine_id: provenance.engine_id.clone(),
            provenance_engine_version: provenance.engine_version.clone(),
            provenance_rules_version: provenance.rules_version.clone(),
            canonical_jcs: String::new(),
            fingerprint: [0; 32],
        };
        fact.canonical_jcs = encode_fact(&fact);
        fact.fingerprint = sha256(fact.canonical_jcs.as_bytes());
        facts.push(fact);
    }
    facts.sort_unstable_by_key(|fact| fact.node_id);
    if facts
        .windows(2)
        .any(|pair| pair[0].node_id == pair[1].node_id)
    {
        return Err(StagingMathVectorManifestError::BindingMismatch);
    }
    let canonical_jcs = encode_manifest(
        package.semantic_fingerprint(),
        bindings.fingerprint(),
        safe_vector.fingerprint(),
        &facts,
    );
    Ok(StagingMathVectorManifest {
        facts,
        package_fingerprint: package.semantic_fingerprint(),
        binding_set_fingerprint: bindings.fingerprint(),
        safe_vector_manifest_fingerprint: safe_vector.fingerprint(),
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn encode_manifest(
    package: [u8; 32],
    bindings: [u8; 32],
    safe: [u8; 32],
    facts: &[StagingMathVectorManifestFact],
) -> String {
    let mut out = String::from("{\"algorithm\":");
    push_jcs_string(&mut out, STAGING_MATH_VECTOR_MANIFEST_ALGORITHM);
    out.push_str(",\"contract\":\"typaxis.contract/1.4\",\"facts\":[");
    for (index, fact) in facts.iter().enumerate() {
        if index > 0 {
            out.push(',')
        }
        out.push_str(fact.canonical_jcs())
    }
    out.push_str("],\"fingerprints\":{\"binding_set_sha256\":");
    push_hash(&mut out, bindings);
    out.push_str(",\"package_sha256\":");
    push_hash(&mut out, package);
    out.push_str(",\"safe_vector_manifest_sha256\":");
    push_hash(&mut out, safe);
    out.push_str("}}");
    out
}

fn encode_fact(value: &StagingMathVectorManifestFact) -> String {
    let mut out = String::from("{\"alternative_sha256\":");
    push_hash(&mut out, value.alternative_sha256);
    out.push_str(",\"common_binding_fingerprint\":");
    push_hash(&mut out, value.common_binding_fingerprint);
    out.push_str(",\"display_command_fingerprint\":");
    push_hash(&mut out, value.display_command_fingerprint);
    if let Some(number) = &value.equation_number_jcs {
        out.push_str(",\"equation_number\":");
        out.push_str(number)
    }
    out.push_str(",\"kind\":");
    push_jcs_string(&mut out, value.kind.as_str());
    out.push_str(",\"language\":");
    push_jcs_string(&mut out, &value.language);
    out.push_str(",\"mapped_source_span\":");
    push_source_span(
        &mut out,
        value.mapped_source_id,
        value.mapped_source_start,
        value.mapped_source_end,
    );
    out.push_str(",\"math_binding_fingerprint\":");
    push_hash(&mut out, value.math_binding_fingerprint);
    out.push_str(",\"metrics\":");
    push_metrics(&mut out, value.metrics);
    out.push_str(",\"node_id\":");
    out.push_str(&value.node_id.get().to_string());
    out.push_str(",\"owner_source_span\":");
    push_source_span(
        &mut out,
        value.owner_source_id,
        value.owner_source_start,
        value.owner_source_end,
    );
    out.push_str(",\"pdf_use_fingerprint\":");
    push_hash(&mut out, value.pdf_use_fingerprint);
    out.push_str(",\"placement\":");
    out.push_str(&value.placement_details_jcs);
    out.push_str(",\"producer\":{\"engine_id\":");
    push_jcs_string(&mut out, &value.provenance_engine_id);
    out.push_str(",\"engine_version\":");
    push_jcs_string(&mut out, &value.provenance_engine_version);
    out.push_str(",\"rules_version\":");
    push_jcs_string(&mut out, &value.provenance_rules_version);
    out.push('}');
    out.push_str(",\"resolved_actual_text_sha256\":");
    push_hash(&mut out, value.resolved_actual_text_sha256);
    out.push_str(",\"safe_vector_usage_fingerprint\":");
    push_hash(&mut out, value.safe_vector_usage_fingerprint);
    out.push_str(",\"selected_placement_fingerprint\":");
    push_hash(&mut out, value.selected_placement_fingerprint);
    out.push_str(",\"source_tex\":{\"exact_slice_sha256\":");
    push_hash(&mut out, value.source_tex_sha256);
    out.push_str(",\"text_buffer_sha256\":");
    push_hash(&mut out, value.text_buffer_sha256);
    out.push_str(",\"text_span\":{\"end_byte\":");
    out.push_str(&value.text_end.to_string());
    out.push_str(",\"start_byte\":");
    out.push_str(&value.text_start.to_string());
    out.push_str(",\"text_id\":");
    out.push_str(&value.text_buffer_id.to_string());
    out.push_str("}}}");
    out
}

fn encode_math_placement_details(
    value: &StagingSafeVectorPlacementDetailsV2,
    kind: PrecomposedMathVectorKind,
) -> Result<String, StagingMathVectorManifestError> {
    match (kind, value) {
        (
            PrecomposedMathVectorKind::Inline,
            StagingSafeVectorPlacementDetailsV2::Inline {
                spacing_before,
                spacing_after,
                ..
            },
        ) => Ok(format!(
            "{{\"spacing_after\":{spacing_after},\"spacing_before\":{spacing_before}}}"
        )),
        (
            PrecomposedMathVectorKind::Block,
            StagingSafeVectorPlacementDetailsV2::MathVectorBlock {
                alignment,
                end_indent,
                flow_fingerprint,
                flow_id,
                keep_with_next,
                parent_flow_id,
                parent_position,
                space_after,
                space_before,
                start_indent,
                style_fingerprint,
                terminal,
                terminal_receipt_fingerprint,
                ..
            },
        ) => {
            let mut out = String::from("{\"alignment\":");
            push_jcs_string(&mut out, alignment);
            out.push_str(",\"end_indent\":");
            out.push_str(&end_indent.to_string());
            out.push_str(
                ",\"flow\":{\"algorithm\":\"typaxis.math-vector-flow/1\",\"fingerprint\":",
            );
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
            out.push_str("},\"keep_with_next\":");
            out.push_str(if *keep_with_next { "true" } else { "false" });
            out.push_str(",\"space_after\":");
            out.push_str(&space_after.to_string());
            out.push_str(",\"space_before\":");
            out.push_str(&space_before.to_string());
            out.push_str(",\"start_indent\":");
            out.push_str(&start_indent.to_string());
            out.push_str(",\"style_fingerprint\":");
            push_hash(&mut out, *style_fingerprint);
            out.push('}');
            Ok(out)
        }
        _ => Err(StagingMathVectorManifestError::KindMismatch),
    }
}

fn encode_equation_number(
    value: &typaxis_syntax::ValidatedPrecomposedVectorEquationNumber,
) -> String {
    let mut out = String::from("{\"minimum_gap\":");
    out.push_str(&value.minimum_gap().get().raw().to_string());
    out.push_str(",\"node_id\":");
    out.push_str(&value.node_id().get().to_string());
    out.push_str(",\"source_span\":");
    let span = value.span();
    push_source_span(
        &mut out,
        span.source_id().get(),
        span.start_byte().get(),
        span.end_byte().get(),
    );
    out.push_str(",\"text_span\":{\"end_byte\":");
    out.push_str(&value.text().text_span().end_byte().get().to_string());
    out.push_str(",\"start_byte\":");
    out.push_str(&value.text().text_span().start_byte().get().to_string());
    out.push_str(",\"text_id\":");
    out.push_str(&value.text().text_span().text_id().get().to_string());
    out.push_str("}}");
    out
}
fn push_source_span(out: &mut String, id: u32, start: u32, end: u32) {
    out.push_str("{\"end_byte\":");
    out.push_str(&end.to_string());
    out.push_str(",\"source_id\":");
    out.push_str(&id.to_string());
    out.push_str(",\"start_byte\":");
    out.push_str(&start.to_string());
    out.push('}')
}
fn push_metrics(out: &mut String, value: StagingVectorMetricFactV2) {
    out.push_str("{\"advance\":");
    out.push_str(&value.advance_raw().to_string());
    out.push_str(",\"ascent\":");
    out.push_str(&value.ascent_raw().to_string());
    out.push_str(",\"baseline\":");
    out.push_str(&value.baseline_raw().to_string());
    out.push_str(",\"descent\":");
    out.push_str(&value.descent_raw().to_string());
    out.push_str(",\"origin_x\":");
    out.push_str(&value.origin_x_raw().to_string());
    out.push_str(",\"viewport_height\":");
    out.push_str(&value.viewport_height_raw().to_string());
    out.push_str(",\"viewport_width\":");
    out.push_str(&value.viewport_width_raw().to_string());
    out.push('}')
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

impl From<StagingSafeVectorManifestV2Error> for StagingMathVectorManifestError {
    fn from(_: StagingSafeVectorManifestV2Error) -> Self {
        Self::SafeVectorMismatch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_v2_fixture::{build_vector_v2_manifests, manifest_vector_v2_fixture};

    #[test]
    fn math_vector_manifest_closes_tex_metrics_alternative_flow_and_safe_vector_usage() {
        let fixture = manifest_vector_v2_fixture().unwrap();
        let products = build_vector_v2_manifests(&fixture).unwrap();
        let manifest = &products.math;
        assert_eq!(
            manifest.facts().len(),
            fixture.display.layout.bindings.math_receipts().len()
        );
        assert!(manifest
            .facts()
            .iter()
            .all(|fact| fact.source_tex_sha256() != [0; 32]));
        assert!(manifest
            .facts()
            .iter()
            .all(|fact| fact.safe_vector_usage_fingerprint() != [0; 32]));
        assert!(manifest
            .facts()
            .iter()
            .any(|fact| fact.kind() == PrecomposedMathVectorKind::Inline));
        assert!(manifest
            .facts()
            .iter()
            .any(|fact| fact.kind() == PrecomposedMathVectorKind::Block));
        assert_eq!(
            manifest.fingerprint(),
            sha256(manifest.canonical_jcs().as_bytes())
        );
        assert!(manifest
            .canonical_jcs()
            .contains("\"algorithm\":\"typaxis.math-vector-manifest/1\""));
        assert!(manifest.canonical_jcs().contains("\"text_buffer_sha256\":"));
        assert!(manifest.canonical_jcs().contains("\"terminal\":1"));
    }
}
