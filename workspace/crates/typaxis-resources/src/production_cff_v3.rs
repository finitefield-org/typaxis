//! CFF plans from actual ledger-bound /3 shaping. A mixed-font document
//! assembler must dispatch TrueType explicitly; it is never discarded here.
use super::cff_v2::*;
use std::collections::BTreeMap;
use typaxis_core::FontInstanceId;
use typaxis_resource_admission::{AdmittedProductionFontV3, AdmittedProductionResourceLedgerV3};
use typaxis_shaping::ShapedProductionRunV3;

#[derive(Debug)]
pub struct FrozenProductionCff1FontsV3<'a> {
    ledger: &'a AdmittedProductionResourceLedgerV3,
    instance_table_fingerprint: [u8; 32],
    fonts: Vec<FrozenPdfCff1PlanV2>,
}
impl<'a> FrozenProductionCff1FontsV3<'a> {
    pub fn ledger(&self) -> &'a AdmittedProductionResourceLedgerV3 {
        self.ledger
    }
    pub fn instance_table_fingerprint(&self) -> [u8; 32] {
        self.instance_table_fingerprint
    }
    pub fn fonts(&self) -> &[FrozenPdfCff1PlanV2] {
        &self.fonts
    }
}
/// All runs must originate in the same actual admission session and selected
/// instance table. Face IDs and admissions are derived from sealed instances.
pub fn freeze_production_cff1_fonts_v3<'a>(
    runs: &[&ShapedProductionRunV3<'a>],
) -> Result<FrozenProductionCff1FontsV3<'a>, Cff1PdfPlanErrorV2> {
    let first = runs
        .first()
        .ok_or(Cff1PdfPlanErrorV2::InvalidInput)?
        .instance();
    let ledger = first.ledger();
    let table = first.table_fingerprint();
    let mut groups: BTreeMap<FontInstanceId, (_, Vec<_>)> = BTreeMap::new();
    if runs.len() as u64 > ledger.effective_limits().base().get().max_fragments {
        return Err(Cff1PdfPlanErrorV2::ResourceLimit);
    }
    for run in runs {
        let instance = run.instance();
        if !instance.ledger().same_session_as(ledger)
            || instance.ledger().fingerprint() != ledger.fingerprint()
            || instance.table_fingerprint() != table
        {
            return Err(Cff1PdfPlanErrorV2::IdentityMismatch);
        }
        let cff = run.cff1_v2().ok_or(Cff1PdfPlanErrorV2::InvalidInput)?;
        let (_, group) = groups
            .entry(instance.font_instance_id())
            .or_insert_with(|| (instance, Vec::new()));
        group
            .try_reserve(1)
            .map_err(|_| Cff1PdfPlanErrorV2::ResourceLimit)?;
        group.push(cff);
    }
    let mut inputs = Vec::new();
    inputs
        .try_reserve_exact(groups.len())
        .map_err(|_| Cff1PdfPlanErrorV2::ResourceLimit)?;
    for (instance, runs) in groups.values() {
        let AdmittedProductionFontV3::Cff1V2(font) = instance.font() else {
            return Err(Cff1PdfPlanErrorV2::InvalidInput);
        };
        inputs.push(Cff1PdfFontInputV2 {
            font_face_id: font.declaration().font_face_id,
            font_instance_id: instance.font_instance_id(),
            admission: font.admission(),
            runs,
        });
    }
    let fonts = freeze_cff1_pdf_fonts_v2(&inputs)?;
    Ok(FrozenProductionCff1FontsV3 {
        ledger,
        instance_table_fingerprint: table,
        fonts,
    })
}

#[cfg(test)]
#[path = "production_cff_v3_tests.rs"]
mod tests;
