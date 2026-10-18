//! Fallible prepayment for family index search.
use super::{FontFaceId, FontFamilyError, FontFamilyTable};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontFamilyWorkError<E> {
    Family(FontFamilyError),
    ArithmeticOverflow,
    Refused(E),
}

pub(super) fn lookup_allowance(families: usize, query_bytes: usize) -> Option<u64> {
    u64::try_from(families)
        .ok()?
        .checked_add(1)?
        .checked_mul(u64::try_from(query_bytes).ok()?.checked_add(2)?)?
        .checked_add(1)
}
impl FontFamilyTable {
    /// Resolve the same first declared fallback after accepting work in advance.
    ///
    /// Entry costs one unit. Each attempted name costs
    /// `1 + (declared families + 1) * (query UTF-8 bytes + 2)`: a conservative
    /// allowance for tree or sorted-index traversal and byte comparisons, rather than
    /// a measurement of actual comparisons or CPU instructions. No owned search
    /// index or result buffer is constructed. A refused request is never used.
    pub fn try_resolve_with_work<E>(
        &self,
        fallback: &[String],
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<FontFaceId, FontFamilyWorkError<E>> {
        work(1).map_err(FontFamilyWorkError::Refused)?;
        if fallback.is_empty() {
            return Err(FontFamilyWorkError::Family(
                FontFamilyError::EmptyFallbackList,
            ));
        }
        for family in fallback {
            let amount = lookup_allowance(self.by_family.len(), family.len())
                .ok_or(FontFamilyWorkError::ArithmeticOverflow)?;
            work(amount).map_err(FontFamilyWorkError::Refused)?;
            if let Some(face) = self.by_family.get(family) {
                return Ok(*face);
            }
        }
        Err(FontFamilyWorkError::Family(FontFamilyError::UnknownFamily))
    }
}

#[cfg(test)]
#[path = "family_work_tests.rs"]
mod tests;
