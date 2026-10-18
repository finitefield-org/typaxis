//! Full cmap observation extends the earlier base-only request policy.
use super::{CffBaseCmapBudgetFailureV2, CffBaseCmapBudgetV2, CffBaseCmapStorageV2};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffVariationStorageV2 {
    PayloadSpans,
    OwnedBytes,
}
pub trait CffCmapBudgetV2: CffBaseCmapBudgetV2 {
    const OBSERVE_VARIATIONS: bool = true;
    fn reserve_variation(
        &mut self,
        site: CffVariationStorageV2,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error>;
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CffVariationBudgetFailureV2<E> {
    pub error: E,
    pub table_offset: u64,
    pub file_offset: Option<u64>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CffCmapBudgetFailureV2<E> {
    Base(CffBaseCmapBudgetFailureV2<E>),
    Variation(CffVariationBudgetFailureV2<E>),
}
impl<E> CffCmapBudgetFailureV2<E> {
    pub(super) fn bind_file_offset(&mut self, cmap_start: usize) {
        match self {
            Self::Base(f) => {
                f.file_offset = f
                    .table_offset
                    .and_then(|v| (cmap_start as u64).checked_add(v))
            }
            Self::Variation(f) => f.file_offset = (cmap_start as u64).checked_add(f.table_offset),
        }
    }
    pub(super) fn base_only(self) -> CffBaseCmapBudgetFailureV2<E> {
        match self {
            Self::Base(f) => f,
            Self::Variation(_) => {
                unreachable!("base-only observation cannot refuse variation work")
            }
        }
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for CffCmapBudgetFailureV2<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFF /2 cmap budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for CffCmapBudgetFailureV2<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Base(f) => &f.error,
            Self::Variation(f) => &f.error,
        })
    }
}
pub type CffCmapBudgetResultV2<T, F, E> = Result<Result<T, F>, CffCmapBudgetFailureV2<E>>;

pub(super) struct BaseOnly<'a, B>(pub &'a mut B);
impl<B: CffBaseCmapBudgetV2> CffBaseCmapBudgetV2 for BaseOnly<'_, B> {
    type Error = B::Error;
    const OBSERVED: bool = B::OBSERVED;
    fn reserve(
        &mut self,
        site: CffBaseCmapStorageV2,
        rows: usize,
        bytes: usize,
    ) -> Result<(), Self::Error> {
        self.0.reserve(site, rows, bytes)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.0.work(n)
    }
}
impl<B: CffBaseCmapBudgetV2> CffCmapBudgetV2 for BaseOnly<'_, B> {
    const OBSERVE_VARIATIONS: bool = false;
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        unreachable!("base-only observation never calls variation hooks")
    }
}
impl CffCmapBudgetV2 for super::base_cmap_budget::NoBaseCmapBudget {
    const OBSERVE_VARIATIONS: bool = false;
    fn reserve_variation(
        &mut self,
        _: CffVariationStorageV2,
        _: usize,
        _: usize,
    ) -> Result<(), Self::Error> {
        unreachable!("ordinary admission never calls variation hooks")
    }
}
pub(super) struct VariationObserver<'a, B> {
    pub budget: &'a mut B,
    pub table_start: usize,
}
impl<B: CffCmapBudgetV2> VariationObserver<'_, B> {
    pub fn work(&mut self, at: usize, n: u64) -> Result<(), CffCmapBudgetFailureV2<B::Error>> {
        if B::OBSERVED && B::OBSERVE_VARIATIONS {
            self.budget.work(n).map_err(|e| self.failure(at, e))?;
        }
        Ok(())
    }
    pub fn reserve(
        &mut self,
        at: usize,
        site: CffVariationStorageV2,
        rows: usize,
        bytes: usize,
    ) -> Result<(), CffCmapBudgetFailureV2<B::Error>> {
        if B::OBSERVED && B::OBSERVE_VARIATIONS {
            self.budget
                .reserve_variation(site, rows, bytes)
                .map_err(|e| self.failure(at, e))?;
        }
        Ok(())
    }
    fn failure(&self, at: usize, error: B::Error) -> CffCmapBudgetFailureV2<B::Error> {
        CffCmapBudgetFailureV2::Variation(CffVariationBudgetFailureV2 {
            error,
            table_offset: self.table_start as u64 + at as u64,
            file_offset: None,
        })
    }
}
