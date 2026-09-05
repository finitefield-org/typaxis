//! Selected production vector usages, sharing admitted Forms while retaining
//! every source occurrence. The font owner establishes a cumulative budget.
use crate::{
    ProductionBodyFontPlans, StagingSafeVectorFormPlansV2,
    StagingSafeVectorResourceV2Error as Error, VectorContentCandidateRegistry,
};
use typaxis_core::M4EffectiveResourceLimits;
use typaxis_resource_admission::AdmittedResourceLedger;

pub struct ProductionBodyVectorPlans<'f, 'v, 'd, 's, 'p, 'a> {
    fonts: &'f ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a>,
    registry: VectorContentCandidateRegistry,
    forms: StagingSafeVectorFormPlansV2,
    record_charge: u64,
}
impl<'f, 'v, 'd, 's, 'p, 'a> ProductionBodyVectorPlans<'f, 'v, 'd, 's, 'p, 'a> {
    pub const fn fonts(&self) -> &'f ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a> {
        self.fonts
    }
    pub const fn registry(&self) -> &VectorContentCandidateRegistry {
        &self.registry
    }
    pub const fn forms(&self) -> &StagingSafeVectorFormPlansV2 {
        &self.forms
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn verify(
        &self,
        fonts: &ProductionBodyFontPlans<'_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), Error> {
        if !std::ptr::eq(self.fonts, fonts) {
            return Err(Error::ReceiptMismatch);
        }
        fonts
            .verify(fonts.display(), admitted, limits)
            .map_err(|_| Error::DisplayMismatch)
    }
}

pub fn finalize_production_body_vectors<'f, 'v, 'd, 's, 'p, 'a>(
    fonts: &'f ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodyVectorPlans<'f, 'v, 'd, 's, 'p, 'a>, Error> {
    fonts
        .verify(fonts.display(), admitted, limits)
        .map_err(|_| Error::DisplayMismatch)?;
    let display = fonts.display();
    let mut record_charge = fonts.record_charge();
    // Include temporary Form joins, PDF usage/page bindings and the combined
    // text/vector ordering records before allocating any of those collections.
    charge(&mut record_charge, display.draws().len() as u64, 12, limits)?;
    charge(
        &mut record_charge,
        admitted.images().len() as u64,
        6,
        limits,
    )?;
    charge(
        &mut record_charge,
        display.selected().pages().len() as u64,
        4,
        limits,
    )?;
    let registry =
        VectorContentCandidateRegistry::from_admitted(admitted, display.resource_declarations())?;
    for candidate in registry.candidates() {
        charge(
            &mut record_charge,
            candidate.ext_g_state_plan().entries().len() as u64,
            4,
            limits,
        )?;
    }
    let forms =
        crate::safe_vector_v2::finalize_production_display_forms(display, &registry, limits)?;
    Ok(ProductionBodyVectorPlans {
        fonts,
        registry,
        forms,
        record_charge,
    })
}
fn charge(
    used: &mut u64,
    n: u64,
    factor: u64,
    limits: &M4EffectiveResourceLimits,
) -> Result<(), Error> {
    *used = n
        .checked_mul(factor)
        .and_then(|n| used.checked_add(n))
        .ok_or(Error::CountOverflow)?;
    if *used > limits.base().get().max_fragments {
        return Err(Error::RecordLimit);
    }
    Ok(())
}
