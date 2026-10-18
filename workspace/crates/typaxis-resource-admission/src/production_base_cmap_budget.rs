//! A refusal belongs to the authentic font declaration checked by the resolver.
use super::*;
use typaxis_font::{CffBaseCmapBudgetFailureV2, CffBaseCmapBudgetV2, CffBaseCmapStorageV2};

#[derive(Debug)]
pub struct ProductionBaseCmapBudgetFailureV3<E> {
    pub font_face_id: FontFaceId,
    pub failure: CffBaseCmapBudgetFailureV2<E>,
}
impl<E: std::fmt::Debug> std::fmt::Display for ProductionBaseCmapBudgetFailureV3<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "production font base cmap budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for ProductionBaseCmapBudgetFailureV3<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.failure)
    }
}
pub type ProductionBaseCmapBudgetResultV3<T, E> =
    Result<Result<T, ProductionResourceErrorV3>, ProductionBaseCmapBudgetFailureV3<E>>;

pub(super) struct UnobservedBaseCmap;
impl CffBaseCmapBudgetV2 for UnobservedBaseCmap {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn reserve(&mut self, _: CffBaseCmapStorageV2, _: usize, _: usize) -> Result<(), Self::Error> {
        unreachable!("ordinary admission never calls a base-cmap budget hook")
    }
    fn work(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("ordinary admission never calls a base-cmap budget hook")
    }
}
