//! Source math bindings joined to the common display's final vector usages.
use super::*;
use std::collections::BTreeMap;
use typaxis_pdf::ProductionBodyAssemblyError as E;

#[derive(Debug)]
pub struct ProductionMathVectorManifest {
    manifest: StagingMathVectorManifest,
    record_charge: u64,
    spool_charge: u64,
}
impl ProductionMathVectorManifest {
    pub fn manifest(&self) -> &StagingMathVectorManifest {
        &self.manifest
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
}

pub fn build_production_math_vector_manifest(
    display: &typaxis_display_list::ProductionBodyFootnoteDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    safe: &crate::ProductionSafeVectorManifest,
    limits: &typaxis_core::M4EffectiveResourceLimits,
) -> Result<ProductionMathVectorManifest, E> {
    let layout = display.source().line_layout();
    let package = layout.source_flow().package();
    let manifest = safe.manifest();
    if manifest.display_fingerprint() != display.fingerprint()
        || manifest.package_fingerprint() != package.semantic_fingerprint()
        || manifest.limits_fingerprint() != limits.fingerprint()
    {
        return Err(E::ReceiptMismatch);
    }
    let mut count = 0u64;
    let mut extra = 4096u64;
    for draw in display.draws() {
        if let typaxis_display_list::ProductionBodyDraw::Vector(vector) = draw {
            if let Some(math) = vector.math_binding() {
                count = count.checked_add(1).ok_or(E::RecordLimit)?;
                extra = extra.checked_add(16384).ok_or(E::SpoolLimit)?;
                let provenance = math.provenance();
                let language = layout
                    .source_flow()
                    .navigation()
                    .languages()
                    .record(math.node_id())
                    .ok_or(E::ReceiptMismatch)?;
                for text in [
                    provenance.engine_id.as_str(),
                    provenance.engine_version.as_str(),
                    provenance.rules_version.as_str(),
                    language.effective_language.as_ref(),
                ] {
                    extra = extra
                        .checked_add((text.len() as u64).checked_mul(16).ok_or(E::SpoolLimit)?)
                        .ok_or(E::SpoolLimit)?;
                }
            }
        }
    }
    // Charge retained facts and the temporary owner index before allocating.
    let record_charge = safe
        .record_charge()
        .checked_add(count.checked_mul(4).ok_or(E::RecordLimit)?)
        .and_then(|v| v.checked_add(u64::from(manifest.placement_count())))
        .and_then(|v| v.checked_add(4))
        .ok_or(E::RecordLimit)?;
    let spool_charge = safe
        .spool_charge()
        .checked_add(extra)
        .ok_or(E::SpoolLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    if spool_charge > limits.base().get().max_spool_bytes {
        return Err(E::SpoolLimit);
    }
    let mut placements = BTreeMap::new();
    for placement in manifest.resources().iter().flat_map(|r| r.placements()) {
        if placements.insert(placement.owner(), placement).is_some() {
            return Err(E::ReceiptMismatch);
        }
    }
    let mut facts = Vec::new();
    facts
        .try_reserve_exact(usize::try_from(count).map_err(|_| E::RecordLimit)?)
        .map_err(|_| E::AllocationFailure)?;
    for draw in display.draws() {
        if let typaxis_display_list::ProductionBodyDraw::Vector(vector) = draw {
            if let Some(math) = vector.math_binding() {
                let placement = placements
                    .remove(&math.node_id())
                    .ok_or(E::ReceiptMismatch)?;
                if placement.display_command_fingerprint() != vector.fingerprint() {
                    return Err(E::ReceiptMismatch);
                }
                facts.push(
                    build_math_fact(package, math, vector.binding(), placement)
                        .map_err(|_| E::ReceiptMismatch)?,
                );
            }
        }
    }
    if placements.values().any(|p| {
        matches!(
            p.kind(),
            typaxis_display_list::StagingCombinedVectorKindV2::MathVector
                | typaxis_display_list::StagingCombinedVectorKindV2::MathVectorBlock
        )
    }) {
        return Err(E::ReceiptMismatch);
    }
    facts.sort_unstable_by_key(|f| f.node_id());
    let canonical_jcs = encode_manifest(
        package.semantic_fingerprint(),
        layout.binding_set_fingerprint(),
        manifest.fingerprint(),
        &facts,
    );
    if canonical_jcs.len() as u64 > extra {
        return Err(E::SpoolLimit);
    }
    Ok(ProductionMathVectorManifest {
        manifest: StagingMathVectorManifest {
            facts,
            package_fingerprint: package.semantic_fingerprint(),
            binding_set_fingerprint: layout.binding_set_fingerprint(),
            safe_vector_manifest_fingerprint: manifest.fingerprint(),
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        record_charge,
        spool_charge,
    })
}
