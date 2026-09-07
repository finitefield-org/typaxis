//! Tagged manifest projected from the verified common serializer observation.
use super::*;
use typaxis_pdf::ProductionBodyAssemblyError as E;

#[derive(Debug)]
pub struct ProductionTaggedManifest {
    manifest: StagingTaggedPdfManifestV2,
    record_charge: u64,
    spool_charge: u64,
}
impl ProductionTaggedManifest {
    pub fn manifest(&self) -> &StagingTaggedPdfManifestV2 {
        &self.manifest
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
}

pub fn build_production_tagged_manifest(
    structure: &typaxis_display_list::ProductionFootnoteStructure<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    pdf: &typaxis_pdf::ProductionCommonTaggedPdf,
    safe: &crate::ProductionSafeVectorManifest,
    math: &crate::ProductionMathVectorManifest,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionTaggedManifest, E> {
    let display = structure.display();
    let flow = display.source().line_layout().source_flow();
    let package = flow.package();
    let navigation = flow.navigation();
    let registry = structure.registry();
    let observation = pdf.tagged_observation();
    if pdf.package_sha256() != package.canonical_jcs_sha256()
        || pdf.structure_registry_sha256() != registry.fingerprint()
        || pdf.structure_sha256() != structure.fingerprint()
        || pdf.display_sha256() != display.fingerprint()
        || pdf.limits_sha256() != limits.fingerprint()
        || registry.limits_sha256() != limits.fingerprint()
        || safe.manifest().display_fingerprint() != display.fingerprint()
        || safe.manifest().final_pdf_sha256() != pdf.final_pdf().content_hash()
        || math.manifest().safe_vector_manifest_fingerprint() != safe.manifest().fingerprint()
        || math.manifest().package_fingerprint() != package.semantic_fingerprint()
        || math.record_charge() < safe.record_charge()
        || math.spool_charge() < safe.spool_charge()
        || safe.record_charge() < pdf.record_charge()
        || safe.spool_charge() < pdf.spool_charge()
        || observation.selected_binding_sha256() != structure.fingerprint()
        || observation.vector_records().len() != safe.manifest().placement_count() as usize
    {
        return Err(E::ReceiptMismatch);
    }
    let count = u64::from(safe.manifest().placement_count());
    let record_charge = math
        .record_charge()
        .checked_add(count.checked_mul(2).ok_or(E::RecordLimit)?)
        .and_then(|n| n.checked_add(4))
        .ok_or(E::RecordLimit)?;
    let mut extra = count
        .checked_mul(4096)
        .and_then(|n| n.checked_add(4096))
        .ok_or(E::SpoolLimit)?;
    extra = extra
        .checked_add(
            (navigation.languages().document_language().len() as u64)
                .checked_mul(6)
                .ok_or(E::SpoolLimit)?,
        )
        .ok_or(E::SpoolLimit)?;
    for usage in safe
        .manifest()
        .resources()
        .iter()
        .flat_map(|r| r.placements())
    {
        extra = extra
            .checked_add(
                (usage.language().len() as u64)
                    .checked_mul(16)
                    .ok_or(E::SpoolLimit)?,
            )
            .ok_or(E::SpoolLimit)?;
    }
    let spool_charge = math
        .spool_charge()
        .checked_add(extra)
        .ok_or(E::SpoolLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    if spool_charge > limits.base().get().max_spool_bytes {
        return Err(E::SpoolLimit);
    }
    let mut vector_structures = Vec::new();
    vector_structures
        .try_reserve_exact(usize::try_from(count).map_err(|_| E::RecordLimit)?)
        .map_err(|_| E::AllocationFailure)?;
    for usage in safe
        .manifest()
        .resources()
        .iter()
        .flat_map(|r| r.placements())
    {
        let owner = usage.owner();
        let node = registry.source_node(owner).ok_or(E::ReceiptMismatch)?;
        let binding = node.vector_binding_v2().ok_or(E::ReceiptMismatch)?;
        let record = observation
            .vector_records()
            .get(usage.usage_id() as usize)
            .ok_or(E::ReceiptMismatch)?;
        if node.owner() != StructureOwner::Source(owner)
            || Some(binding.kind()) != usage.kind().precomposed()
            || Some(binding.metrics_fingerprint()) != usage.metric_receipt_fingerprint()
            || node.language() != usage.language()
            || record.usage_id() != usage.usage_id()
            || record.structure_node_id() != node.structure_node_id().get()
            || record.page_index() != usage.page_index()
            || record.paint_ordinal() != usage.paint_ordinal()
            || record.semantic_fragment_ordinal() != usage.fragment_ordinal()
        {
            return Err(E::ReceiptMismatch);
        }
        let math_binding_fingerprint = match binding.kind() {
            PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock => {
                let fact = math.manifest().fact(owner).ok_or(E::ReceiptMismatch)?;
                if fact.safe_vector_usage_fingerprint() != usage.fingerprint() {
                    return Err(E::ReceiptMismatch);
                }
                Some(fact.math_binding_fingerprint())
            }
            PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::VectorFigure => None,
        };
        let mut fact = StagingTaggedPdfVectorStructureFactV2 {
            structure_node_id: node.structure_node_id().get(),
            owner,
            kind: usage.kind(),
            role: node.role().pdf_name(),
            language: node.language().to_owned(),
            metrics_fingerprint: usage.metric_receipt_fingerprint(),
            safe_vector_usage_fingerprint: usage.fingerprint(),
            marked_content_record_fingerprint: record.fingerprint(),
            math_binding_fingerprint,
            canonical_jcs: String::new(),
        };
        fact.canonical_jcs = encode_structure_fact(&fact);
        vector_structures.push(fact);
    }
    vector_structures.sort_unstable_by_key(|f| f.structure_node_id);
    if vector_structures
        .windows(2)
        .any(|p| p[0].structure_node_id >= p[1].structure_node_id)
        || vector_structures
            .iter()
            .filter(|f| f.math_binding_fingerprint.is_some())
            .count()
            != math.manifest().facts().len()
    {
        return Err(E::ReceiptMismatch);
    }
    let canonical_jcs = encode_manifest_facts(
        navigation.languages().document_language(),
        &EngineIdentity::compiled(),
        [
            ("marked_content_sha256", observation.marked_content_sha256()),
            ("math_vector_manifest_sha256", math.manifest().fingerprint()),
            ("package_sha256", package.canonical_jcs_sha256()),
            ("pdf_observation_sha256", observation.fingerprint()),
            ("pdf_sha256", pdf.final_pdf().content_hash()),
            ("profile_sha256", registry.authorization_sha256()),
            ("safe_vector_manifest_sha256", safe.manifest().fingerprint()),
            (
                "selected_binding_sha256",
                observation.selected_binding_sha256(),
            ),
            ("structure_registry_sha256", registry.fingerprint()),
        ],
        &vector_structures,
    );
    let retained = vector_structures
        .iter()
        .try_fold(canonical_jcs.len() as u64, |n, f| {
            n.checked_add(f.canonical_jcs.len() as u64)
                .and_then(|n| n.checked_add(f.language.len() as u64))
                .ok_or(E::SpoolLimit)
        })?;
    if retained > extra {
        return Err(E::SpoolLimit);
    }
    Ok(ProductionTaggedManifest {
        manifest: StagingTaggedPdfManifestV2 {
            package_sha256: package.canonical_jcs_sha256(),
            safe_vector_manifest_fingerprint: safe.manifest().fingerprint(),
            math_vector_manifest_fingerprint: math.manifest().fingerprint(),
            structure_registry_fingerprint: registry.fingerprint(),
            selected_binding_fingerprint: observation.selected_binding_sha256(),
            marked_content_fingerprint: observation.marked_content_sha256(),
            pdf_observation_fingerprint: observation.fingerprint(),
            final_pdf_sha256: pdf.final_pdf().content_hash(),
            vector_structures,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        record_charge,
        spool_charge,
    })
}
