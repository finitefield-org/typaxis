//! Logical resource context for a refused authentic host read.
use super::{ResourceAdmissionError, ResourceErrorSubject};
pub use typaxis_host_admission::{
    host_open_identity_work_bound, HOST_OPEN_IDENTITY_NODE_BYTES, HOST_OPEN_IDENTITY_SLOT_BYTES,
    HostReadBudget, HostReadBudgetFailure, HostReadRequest, HostReadStage,
};

#[derive(Debug)]
pub struct ResourceReadBudgetFailure<E> {
    pub subject: ResourceErrorSubject,
    pub failure: HostReadBudgetFailure<E>,
}
impl<E: std::fmt::Debug> std::fmt::Display for ResourceReadBudgetFailure<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "resource read budget: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for ResourceReadBudgetFailure<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.failure.error)
    }
}
#[derive(Debug)]
pub enum ResourceReadError<E> {
    Native(ResourceAdmissionError),
    Budget(ResourceReadBudgetFailure<E>),
}
impl<E> From<ResourceAdmissionError> for ResourceReadError<E> {
    fn from(error: ResourceAdmissionError) -> Self {
        Self::Native(error)
    }
}

pub(super) struct UnobservedResourceRead;
impl HostReadBudget for UnobservedResourceRead {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn charge(&mut self, _: HostReadRequest) -> Result<(), Self::Error> {
        unreachable!("ordinary resource read never calls a budget hook")
    }
}
