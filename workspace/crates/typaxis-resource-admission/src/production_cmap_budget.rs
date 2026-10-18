//! The actual host-validated declaration remains the refusal subject.
use super::*;
#[derive(Debug)]
pub struct ProductionCmapBudgetFailureV3<E> {
    pub font_face_id: FontFaceId,
    pub failure: typaxis_font::CffCmapBudgetFailureV2<E>,
}
impl<E: std::fmt::Debug> std::fmt::Display for ProductionCmapBudgetFailureV3<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "production font cmap budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for ProductionCmapBudgetFailureV3<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.failure)
    }
}
pub type ProductionCmapBudgetResultV3<T, E> =
    Result<Result<T, ProductionResourceErrorV3>, ProductionCmapBudgetFailureV3<E>>;
