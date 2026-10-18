//! Bind shared MATH validation work to the CFF caller's existing cells.
use super::*;
struct Relay<'a, B> {
    budget: &'a mut B,
    base: usize,
}
impl<B: CffAdmissionBudgetV2> crate::math::validation::Budget for Relay<'_, B> {
    type Error = CffTableAdmissionBudgetFailureV2<B::Error>;
    const OBSERVED: bool = B::OBSERVE_MATH;
    fn work(&mut self, p: crate::math::validation::Point, n: u64) -> Result<(), Self::Error> {
        let stage = match p.stage {
            crate::math::validation::Stage::Header => CffTableAdmissionStageV2::MathHeader,
            crate::math::validation::Stage::Field => CffTableAdmissionStageV2::MathField,
            crate::math::validation::Stage::RelativeOffset => CffTableAdmissionStageV2::MathOffset,
        };
        self.budget
            .work_math(n)
            .map_err(|error| CffTableAdmissionBudgetFailureV2 {
                error,
                point: CffTableAdmissionWorkPointV2 {
                    stage,
                    table_tag: Some(*b"MATH"),
                    table_offset: p.table_offset,
                    file_offset: p
                        .table_offset
                        .and_then(|n| (self.base as u64).checked_add(n)),
                },
            })
    }
}
pub(super) fn validate<B: CffAdmissionBudgetV2>(
    bytes: &[u8],
    glyphs: u16,
    base: usize,
    order: crate::math::validation::VariantOrder,
    budget: &mut B,
) -> local_optional::Result<(), B::Error> {
    if !B::OBSERVE_MATH && order == crate::math::validation::VariantOrder::LegacyStrict {
        return crate::math::validate_cff_math_table(bytes, glyphs)
            .map_err(|_| local_optional::Failure::Native(Cff1Error::InvalidOptionalTable));
    }
    crate::math::validate_cff_math_table_with_variant_order(
        bytes,
        glyphs,
        order,
        &mut Relay { budget, base },
    )
    .map_err(local_optional::Failure::Budget)?
    .map_err(|_| local_optional::Failure::Native(Cff1Error::InvalidOptionalTable))
}
