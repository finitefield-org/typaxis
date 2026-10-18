//! The authentic resource subject remains attached to each admission refusal.
use super::*;
#[derive(Debug)]
pub struct ProductionAdmissionBudgetFailureV3<E> {
    pub font_face_id: FontFaceId,
    pub failure: typaxis_font::CffAdmissionBudgetFailureV2<E>,
}
impl<E: std::fmt::Debug> std::fmt::Display for ProductionAdmissionBudgetFailureV3<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "production font admission budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for ProductionAdmissionBudgetFailureV3<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.failure)
    }
}
pub type ProductionAdmissionBudgetResultV3<T, E> =
    Result<Result<T, ProductionResourceErrorV3>, ProductionAdmissionBudgetFailureV3<E>>;
