//! Optional path storage and root-walk credits for authentic read revalidation.
use crate::{
    platform, read_budget, HostAdmissionError, HostAdmissionSession, HostReadBudget, HostReadError,
    HostReadRequest, HostReadStage,
};
use typaxis_core::PortablePath;

pub(super) fn prepare_path<B: HostReadBudget>(
    path: &PortablePath,
    stage: HostReadStage,
    budget: &mut B,
) -> Result<platform::PreparedReadPath, HostReadError<B::Error>> {
    let length =
        u64::try_from(path.as_str().len()).map_err(|_| HostAdmissionError::ReadCapacity)?;
    let storage_bytes = platform::PreparedReadPath::storage_bytes(path)?;
    let fixed_work = if storage_bytes == 0 { 256 + 64 } else { 64 };
    let work = length
        .checked_mul(16)
        .and_then(|n| n.checked_add(fixed_work))
        .ok_or(HostAdmissionError::ReadCapacity)?;
    read_budget::charge(
        budget,
        HostReadRequest {
            stage,
            file_offset: 0,
            buffer_bytes: storage_bytes,
            work,
        },
    )?;
    platform::PreparedReadPath::new(path).map_err(Into::into)
}

impl HostAdmissionSession {
    pub(super) fn revalidate_open_resolution_with_budget<B: HostReadBudget>(
        &self,
        path: &PortablePath,
        expected: platform::FileIdentity,
        budget: &mut B,
    ) -> Result<(), HostReadError<B::Error>> {
        if !B::OBSERVED || !B::OBSERVE_RESOLUTION {
            return self
                .revalidate_open_resolution(path, expected)
                .map_err(Into::into);
        }
        let mut prepared = prepare_path(path, HostReadStage::RevalidatePath, budget)?;
        let mut found = None;
        for root in &self.host_roots {
            read_budget::charge(
                budget,
                HostReadRequest {
                    stage: HostReadStage::RevalidateRoot,
                    file_offset: 0,
                    buffer_bytes: 0,
                    work: prepared.root_work()?,
                },
            )?;
            let opened = prepared
                .open(root)
                .map_err(|_| HostAdmissionError::LengthMismatch)?;
            let Some(opened) = opened else { continue };
            if found.is_some() {
                return Err(HostAdmissionError::LengthMismatch.into());
            }
            found = Some(opened.identity());
        }
        if found != Some(expected) {
            return Err(HostAdmissionError::LengthMismatch.into());
        }
        Ok(())
    }
}
