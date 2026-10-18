//! Private-staging base-cmap requests. Other admission phases are independent.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffBaseCmapStorageV2 {
    SubtableMappings,
    MergedMappings,
}

/// Accept each request before execution. Rows/bytes describe the complete
/// target of one new mapping vector, including temporary merge vectors.
/// This does not measure allocator overcapacity or other admission storage.
pub trait CffBaseCmapBudgetV2 {
    type Error;
    const OBSERVED: bool = true;
    fn reserve(
        &mut self,
        site: CffBaseCmapStorageV2,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error>;
    fn work(&mut self, amount: u64) -> Result<(), Self::Error>;
}

/// A budget refusal, separate from the existing semantic font error. Table
/// offsets are cmap-relative; sfnt admission additionally binds file_offset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CffBaseCmapBudgetFailureV2<E> {
    pub error: E,
    pub table_offset: Option<u64>,
    pub file_offset: Option<u64>,
    pub cmap_format: Option<u16>,
}
/// Budget refusal is outer; native semantic failure remains the inner Result.
pub type CffBaseCmapBudgetResultV2<T, F, E> = Result<Result<T, F>, CffBaseCmapBudgetFailureV2<E>>;
impl<E: std::fmt::Debug> std::fmt::Display for CffBaseCmapBudgetFailureV2<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFF /2 base cmap budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for CffBaseCmapBudgetFailureV2<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

pub(super) struct NoBaseCmapBudget;
impl CffBaseCmapBudgetV2 for NoBaseCmapBudget {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), Self::Error> {
        unreachable!("the ordinary path never calls a budget hook")
    }
    fn work(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("the ordinary path never calls a budget hook")
    }
}

#[cfg(test)]
#[path = "cff_v2_base_cmap_budget_tests.rs"]
mod tests;
