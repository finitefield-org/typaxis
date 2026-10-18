//! Private counted feedback buffers. Only the enclosing driver can deliver a
//! PDF; this state records candidates and never creates placement authority.
use super::*;
use typaxis_core::Sha256;
use typaxis_pdf::book_v2::BookV2PageReferenceObservation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ColumnLabelDecision {
    Retry,
    Ready,
}

pub(super) struct ColumnPageLabels {
    values: Vec<(NodeId, u32)>,
    next: Vec<(NodeId, u32)>,
    limits: [u8; 32],
    maximum: u64,
    accepted: [u64; 3],
    previous_matching: Option<([u8; 32], [u8; 32])>,
    cycle_anchor: [u8; 32],
    cycle_power: u64,
    cycle_distance: u64,
    delivered: bool,
    values_on_loan: bool,
    pending_retry: bool,
}

fn prefix(budget: &BookV2ColumnPageBudget) -> [u64; 3] {
    observation_prefix(&budget.inner.observation)
}

fn observation_prefix(total: &BookV2PdfConvergenceObservation) -> [u64; 3] {
    [total.records, total.spool, total.work]
}

fn step(budget: &mut BookV2ColumnPageBudget, label: &'static str) -> Result<(), E> {
    step_total(
        &mut budget.inner.observation,
        budget.inner.maximum_work,
        label,
    )
}

fn step_total(
    total: &mut BookV2PdfConvergenceObservation,
    maximum: u64,
    label: &'static str,
) -> Result<(), E> {
    total.work = add(total.work, 1, maximum, label)?;
    Ok(())
}

fn reserve_buffer<T>(
    buffer: &mut Vec<T>,
    count: usize,
    limits: &M4EffectiveResourceLimits,
    budget: &mut BookV2ColumnPageBudget,
) -> Result<(), E> {
    let count_u64 = u64::try_from(count).map_err(|_| E::Limit("column Page buffers"))?;
    let bytes = count_u64
        .checked_mul(std::mem::size_of::<T>() as u64)
        .ok_or(E::Limit("column Page buffer bytes"))?;
    let total = &mut budget.inner.observation;
    total.records = add(
        total.records,
        count_u64,
        limits.base().get().max_fragments,
        "column Page buffer records",
    )?;
    total.spool = add(
        total.spool,
        bytes,
        limits.base().get().max_spool_bytes,
        "column Page buffer spool",
    )?;
    if count != 0 {
        step(budget, "column Page buffer allocation")?;
        buffer
            .try_reserve_exact(count)
            .map_err(|_| E::Limit("column Page buffer allocation"))?;
    }
    Ok(())
}

fn map_fingerprint(
    values: &[(NodeId, u32)],
    budget: &mut BookV2ColumnPageBudget,
) -> Result<[u8; 32], E> {
    map_fingerprint_total(
        values,
        budget.inner.maximum_work,
        &mut budget.inner.observation,
    )
}

fn map_fingerprint_total(
    values: &[(NodeId, u32)],
    maximum: u64,
    total: &mut BookV2PdfConvergenceObservation,
) -> Result<[u8; 32], E> {
    step_total(total, maximum, "column Page map fold")?;
    let mut hash = Sha256::new();
    hash.update(b"typaxis.book-2-column-page-label-map/1\0");
    for (owner, value) in values {
        step_total(total, maximum, "column Page map entry fold")?;
        hash.update(&owner.get().to_be_bytes());
        hash.update(&value.to_be_bytes());
    }
    Ok(hash.finish())
}

impl ColumnPageLabels {
    pub(super) fn new_observed(
        census: &source_context::ColumnPageReferenceCensus<'_>,
        limits: &M4EffectiveResourceLimits,
        progress: &mut ColumnProgressView<'_, '_, '_>,
    ) -> Result<Self, E> {
        with_observed_source_counter_operation(progress, limits, |local| {
            Self::new_counted(census, limits, local)
        })
    }

    pub(super) fn new_counted(
        census: &source_context::ColumnPageReferenceCensus<'_>,
        limits: &M4EffectiveResourceLimits,
        budget: &mut BookV2ColumnPageBudget,
    ) -> Result<Self, E> {
        if prefix(budget)
            .iter()
            .zip(census.prefix())
            .any(|(now, earlier)| *now < earlier)
        {
            return Err(E::Identity);
        }
        Self::from_owners(census.owners(), limits, budget)
    }

    fn from_owners(
        owners: &[NodeId],
        limits: &M4EffectiveResourceLimits,
        budget: &mut BookV2ColumnPageBudget,
    ) -> Result<Self, E> {
        if budget.inner.limits_fingerprint != limits.fingerprint() {
            return Err(E::Identity);
        }
        let mut values = Vec::new();
        let mut next = Vec::new();
        reserve_buffer(&mut values, owners.len(), limits, budget)?;
        reserve_buffer(&mut next, owners.len(), limits, budget)?;
        for owner in owners {
            step(budget, "column Page initial candidate")?;
            if values.last().is_some_and(|(earlier, _)| earlier >= owner) {
                return Err(E::Identity);
            }
            // This is a source candidate, never a final page or destination.
            values.push((*owner, 1));
        }
        let cycle_anchor = map_fingerprint(&values, budget)?;
        Ok(Self {
            values,
            next,
            limits: limits.fingerprint(),
            maximum: budget.inner.maximum_work,
            accepted: prefix(budget),
            previous_matching: None,
            cycle_anchor,
            cycle_power: 1,
            cycle_distance: 0,
            delivered: false,
            values_on_loan: false,
            pending_retry: false,
        })
    }

    pub(super) fn values(&self) -> &[(NodeId, u32)] {
        &self.values
    }

    /// Move one already paid buffer into a scoped local so source preparation
    /// can borrow its slice while the feedback owner observes the actual PDF.
    /// No buffer is copied, reallocated, refunded or exposed as placement.
    pub(super) fn with_candidate_values<R>(
        &mut self,
        inspect: impl FnOnce(&[(NodeId, u32)], &mut Self) -> Result<R, E>,
    ) -> Result<R, E> {
        if self.values_on_loan || self.delivered {
            return Err(E::Identity);
        }
        self.values_on_loan = true;
        let values = std::mem::take(&mut self.values);
        let result = inspect(&values, self);
        if self.pending_retry {
            self.values = std::mem::take(&mut self.next);
            self.next = values;
            self.pending_retry = false;
        } else {
            self.values = values;
        }
        self.values_on_loan = false;
        result
    }

    pub(super) fn observe_pdf_counted(
        &mut self,
        candidate_values: &[(NodeId, u32)],
        references: &[BookV2PageReferenceObservation],
        labels_match: bool,
        display: [u8; 32],
        pdf: [u8; 32],
        limits: &M4EffectiveResourceLimits,
        maximum: u64,
        progress: &mut BookV2PdfConvergenceObservation,
    ) -> Result<ColumnLabelDecision, E> {
        if limits.fingerprint() != self.limits || maximum != self.maximum {
            return Err(E::Identity);
        }
        self.observe_total(
            candidate_values,
            references.iter().map(|r| {
                (
                    r.owner(),
                    r.candidate_page(),
                    r.target_page(),
                    r.is_placed(),
                )
            }),
            labels_match,
            display,
            pdf,
            progress,
        )
    }

    #[cfg(test)]
    fn observe(
        &mut self,
        references: impl ExactSizeIterator<Item = (NodeId, u32, Option<u32>, bool)>,
        labels_match: bool,
        display: [u8; 32],
        pdf: [u8; 32],
        budget: &mut BookV2ColumnPageBudget,
    ) -> Result<ColumnLabelDecision, E> {
        if budget.inner.limits_fingerprint != self.limits
            || budget.inner.maximum_work != self.maximum
        {
            return Err(E::Identity);
        }
        self.with_candidate_values(|values, state| {
            state.observe_total(
                values,
                references,
                labels_match,
                display,
                pdf,
                &mut budget.inner.observation,
            )
        })
    }

    fn observe_total(
        &mut self,
        candidate_values: &[(NodeId, u32)],
        references: impl ExactSizeIterator<Item = (NodeId, u32, Option<u32>, bool)>,
        labels_match: bool,
        display: [u8; 32],
        pdf: [u8; 32],
        progress: &mut BookV2PdfConvergenceObservation,
    ) -> Result<ColumnLabelDecision, E> {
        if self.delivered
            || !self.values_on_loan
            || self.pending_retry
            || observation_prefix(progress)
                .iter()
                .zip(self.accepted)
                .any(|(now, earlier)| *now < earlier)
        {
            return Err(E::Identity);
        }
        let result = (|| {
            step_total(progress, self.maximum, "column Page observations")?;
            if references.len() != candidate_values.len() {
                return Err(E::Identity);
            }
            self.next.clear();
            let mut actual_matching = true;
            for ((owner, candidate), (observed, value, target, placed)) in
                candidate_values.iter().zip(references)
            {
                step_total(progress, self.maximum, "column Page observation")?;
                if observed != *owner || value != *candidate || target == Some(0) {
                    return Err(E::Identity);
                }
                actual_matching &= match target {
                    Some(page) => page == *candidate,
                    None => !placed,
                };
                // An unplaced reference to an unplaced target keeps its source
                // candidate. The genuine PDF still records target=None.
                self.next.push((*owner, target.unwrap_or(*candidate)));
            }
            if labels_match != actual_matching {
                return Err(E::Identity);
            }
            step_total(progress, self.maximum, "column Page complete state")?;
            let state = (display, pdf);
            if actual_matching
                && (candidate_values.is_empty() || self.previous_matching == Some(state))
            {
                self.delivered = true;
                return Ok(ColumnLabelDecision::Ready);
            }
            self.previous_matching = actual_matching.then_some(state);
            if !actual_matching {
                let next = map_fingerprint_total(&self.next, self.maximum, progress)?;
                step_total(progress, self.maximum, "column Page cycle comparison")?;
                if next == self.cycle_anchor {
                    return Err(E::Limit("column Page label cycle"));
                }
                self.cycle_distance = self
                    .cycle_distance
                    .checked_add(1)
                    .ok_or(E::Limit("column Page cycle"))?;
                if self.cycle_distance == self.cycle_power {
                    self.cycle_anchor = next;
                    self.cycle_power = self
                        .cycle_power
                        .checked_mul(2)
                        .ok_or(E::Limit("column Page cycle"))?;
                    self.cycle_distance = 0;
                }
            }
            self.pending_retry = true;
            Ok(ColumnLabelDecision::Retry)
        })();
        self.accepted = observation_prefix(progress);
        result
    }
}

#[cfg(test)]
#[path = "book_v2_column_page_label_state_tests.rs"]
mod tests;
