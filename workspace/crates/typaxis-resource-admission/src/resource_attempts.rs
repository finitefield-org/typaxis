//! Dense declaration IDs permit a fixed, fallible bitmap on the observed path.
use super::{
    FontFaceId, HostReadBudget, HostReadBudgetFailure, HostReadRequest, HostReadStage,
    ImageResourceId, ResourceAdmissionError, ResourceErrorSubject, ResourceReadBudgetFailure,
    ResourceReadError,
};
use std::collections::BTreeSet;

pub(super) trait AttemptId: Copy + Ord {
    fn index(self) -> usize;
    fn subject(self) -> ResourceErrorSubject;
}
impl AttemptId for FontFaceId {
    fn index(self) -> usize {
        self.get() as usize
    }
    fn subject(self) -> ResourceErrorSubject {
        ResourceErrorSubject::FontFace(self)
    }
}
impl AttemptId for ImageResourceId {
    fn index(self) -> usize {
        self.get() as usize
    }
    fn subject(self) -> ResourceErrorSubject {
        ResourceErrorSubject::Image(self)
    }
}

#[derive(Debug)]
pub(super) enum Attempts<I> {
    Native(BTreeSet<I>),
    Packed { words: Vec<u64>, count: usize },
}

fn charge<I: AttemptId, B: HostReadBudget>(
    id: I,
    budget: &mut B,
    stage: HostReadStage,
    bytes: u64,
    work: u64,
) -> Result<(), ResourceReadError<B::Error>> {
    if B::OBSERVED && B::OBSERVE_ATTEMPTS {
        let request = HostReadRequest {
            stage,
            file_offset: 0,
            buffer_bytes: bytes,
            work,
        };
        budget.charge(request).map_err(|error| {
            ResourceReadError::Budget(ResourceReadBudgetFailure {
                subject: id.subject(),
                failure: HostReadBudgetFailure { error, request },
            })
        })?;
    }
    Ok(())
}

impl<I: AttemptId> Attempts<I> {
    pub(super) fn new() -> Self {
        Self::Native(BTreeSet::new())
    }

    pub(super) fn contains<B: HostReadBudget>(
        &mut self,
        id: I,
        count: usize,
        budget: &mut B,
    ) -> Result<bool, ResourceReadError<B::Error>> {
        if id.index() >= count {
            return Err(ResourceAdmissionError::MissingLogicalResource.into());
        }
        if B::OBSERVED && B::OBSERVE_ATTEMPTS {
            self.prepare(id, count, budget)?;
        }
        charge(id, budget, HostReadStage::AttemptLookup, 0, 64)?;
        match self {
            Self::Native(values) => Ok(values.contains(&id)),
            Self::Packed {
                words,
                count: bound,
            } => {
                if count != *bound {
                    return Err(ResourceAdmissionError::ResourceLimit.into());
                }
                Ok(words[id.index() / 64] & (1 << (id.index() % 64)) != 0)
            }
        }
    }

    fn prepare<B: HostReadBudget>(
        &mut self,
        id: I,
        count: usize,
        budget: &mut B,
    ) -> Result<(), ResourceReadError<B::Error>> {
        let Self::Native(values) = self else {
            return Ok(());
        };
        let rows = count.div_ceil(64);
        let bytes = rows
            .checked_mul(std::mem::size_of::<u64>())
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        let work = bytes
            .checked_add(64)
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        charge(id, budget, HostReadStage::AttemptStorage, bytes, work)?;
        // Prepay the entire migration before allocating or changing either
        // representation. A refused migration retains every native attempt.
        for previous in values.iter().copied() {
            charge(id, budget, HostReadStage::AttemptMigrate, 0, 64)?;
            if previous.index() >= count {
                return Err(ResourceAdmissionError::ResourceLimit.into());
            }
        }
        let mut words = Vec::new();
        words
            .try_reserve_exact(rows)
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        words.resize(rows, 0);
        for previous in values.iter().copied() {
            words[previous.index() / 64] |= 1 << (previous.index() % 64);
        }
        *self = Self::Packed { words, count };
        Ok(())
    }

    /// Called after the aggregate byte check. No attempt or aggregate bytes
    /// are committed if this permit is refused; host-read refusals retain both.
    pub(super) fn mark<B: HostReadBudget>(
        &mut self,
        id: I,
        budget: &mut B,
    ) -> Result<(), ResourceReadError<B::Error>> {
        if let Self::Packed { count, .. } = self {
            if id.index() >= *count {
                return Err(ResourceAdmissionError::MissingLogicalResource.into());
            }
        }
        charge(id, budget, HostReadStage::AttemptMark, 0, 64)?;
        match self {
            Self::Native(values) => {
                values.insert(id);
            }
            Self::Packed { words, .. } => {
                words[id.index() / 64] |= 1 << (id.index() % 64);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "resource_attempts_tests.rs"]
mod tests;
