//! Bounded immutable views of the current ledger; filesystem work holds no lock.
use super::*;
use crate::HostPath;
use crate::{read_budget, HostReadBudget, HostReadError, HostReadRequest, HostReadStage};

fn charge<B: HostReadBudget>(
    budget: &mut B,
    stage: HostReadStage,
    work: u64,
) -> Result<(), HostReadError<B::Error>> {
    if B::OBSERVED && B::OBSERVE_LEDGER {
        read_budget::charge(
            budget,
            HostReadRequest {
                stage,
                file_offset: 0,
                buffer_bytes: 0,
                work,
            },
        )?;
    }
    Ok(())
}

fn native<T>(
    result: Result<T, HostReadError<std::convert::Infallible>>,
) -> Result<T, HostAdmissionError> {
    match result {
        Ok(value) => Ok(value),
        Err(HostReadError::Native(error)) => Err(error),
        Err(HostReadError::Budget(failure)) => match failure.error {},
    }
}

impl HostReadIdentityLedger {
    fn ensure_generation(&self, generation: u64) -> Result<(), HostAdmissionError> {
        if self.lock()?.generation == generation {
            Ok(())
        } else {
            Err(HostAdmissionError::ReadIdentityMismatch)
        }
    }
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
const BATCH: usize = 64;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
struct View {
    locator: CandidateLocator,
    expected: Arc<CandidatePathIdentity>,
    opened: Option<FileIdentity>,
    root: Option<Arc<AdmittedRoot>>,
}
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
struct Views {
    rows: [Option<View>; BATCH],
    length: usize,
}
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
impl HostReadIdentityLedger {
    fn views(
        &self,
        generation: u64,
        after: Option<&CandidateLocator>,
    ) -> Result<Views, HostAdmissionError> {
        use std::ops::Bound::{Excluded, Unbounded};
        let mut result = Views {
            rows: std::array::from_fn(|_| None),
            length: 0,
        };
        let state = self.lock()?;
        if state.generation != generation {
            return Err(HostAdmissionError::ReadIdentityMismatch);
        }
        let start = after.map_or(Unbounded, Excluded);
        for (locator, record) in state
            .candidates
            .range::<CandidateLocator, _>((start, Unbounded))
            .take(BATCH)
        {
            result.rows[result.length] = Some(View {
                locator: locator.clone(),
                expected: record.identity.clone(),
                opened: state.opened_slots[record.opened_slot],
                root: state.roots.get(&locator.root).cloned(),
            });
            result.length += 1;
        }
        Ok(result)
    }
}

impl HostReadIdentityLedgerToken {
    fn validate_with_observer<B: HostReadBudget>(
        &self,
        budget: &mut B,
        mut observer: impl FnMut(bool, usize),
    ) -> Result<(), HostReadError<B::Error>> {
        // A stale proof never starts a callback or path allocation.
        self.owner.ensure_generation(self.generation)?;
        #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
        {
            // Preserve native precedence: all candidate identities first, then
            // all opened terminals, in the original locator order.
            for opened in [false, true] {
                let mut after = None;
                loop {
                    if B::OBSERVED && B::OBSERVE_LEDGER {
                        // A conservative ordered-map search bound, including
                        // root lookups and cloning at most BATCH Arc rows.
                        let depth = (self.candidate_attempts as u64 + 1).ilog2() as u64 + 2;
                        let key = after
                            .as_ref()
                            .map_or(0, |p: &CandidateLocator| p.path.as_str().len() as u64);
                        let work = depth
                            .checked_mul(64)
                            .and_then(|n| n.checked_mul(key.checked_add(32)?))
                            .and_then(|n| n.checked_add(BATCH as u64 * (256 + 64 * 32 * depth)))
                            .ok_or(HostAdmissionError::ReadCapacity)?;
                        charge(budget, HostReadStage::LedgerView, work)?;
                    }
                    let views = self.owner.views(self.generation, after.as_ref())?;
                    if views.length == 0 {
                        break;
                    }
                    after = Some(
                        views.rows[views.length - 1]
                            .as_ref()
                            .unwrap()
                            .locator
                            .clone(),
                    );
                    observer(opened, views.length);
                    self.owner.ensure_generation(self.generation)?;
                    for view in views.rows.into_iter().flatten() {
                        if opened && view.opened.is_none() {
                            continue;
                        }
                        let root = view.root.ok_or(HostAdmissionError::RootSetMismatch)?;
                        if opened {
                            let mut prepared = if B::OBSERVED && B::OBSERVE_LEDGER {
                                crate::read_resolution::prepare_path(
                                    &view.locator.path,
                                    HostReadStage::LedgerOpenedPath,
                                    budget,
                                )?
                            } else {
                                crate::platform::PreparedReadPath::new(&view.locator.path)?
                            };
                            charge(
                                budget,
                                HostReadStage::LedgerOpenedWalk,
                                prepared.root_work()?,
                            )?;
                            self.owner.ensure_generation(self.generation)?;
                            let actual = prepared
                                .open(&root)?
                                .ok_or(HostAdmissionError::ReadIdentityMismatch)?;
                            if Some(actual.identity()) != view.opened {
                                return Err(HostAdmissionError::ReadIdentityMismatch.into());
                            }
                        } else {
                            let matches = if B::OBSERVED && B::OBSERVE_LEDGER {
                                let mut prepared = crate::read_resolution::prepare_path(
                                    &view.locator.path,
                                    HostReadStage::LedgerCandidatePath,
                                    budget,
                                )?;
                                charge(
                                    budget,
                                    HostReadStage::LedgerCandidateWalk,
                                    prepared.root_work()?,
                                )?;
                                self.owner.ensure_generation(self.generation)?;
                                prepared.candidate_identity_matches_with_budget(
                                    &root,
                                    &view.locator.path,
                                    &view.expected,
                                    budget,
                                )?
                            } else {
                                root.candidate_identity_matches(&view.locator.path, &view.expected)?
                            };
                            if !matches {
                                return Err(HostAdmissionError::ReadIdentityMismatch.into());
                            }
                        }
                    }
                }
            }
            charge(budget, HostReadStage::LedgerFinish, 64)?;
            self.owner
                .ensure_generation(self.generation)
                .map_err(Into::into)
        }
        #[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
        {
            let _ = (&mut observer, budget);
            self.owner.ensure_generation(self.generation)?;
            Err(HostAdmissionError::UnsupportedContainedOpen.into())
        }
    }
    /// Re-resolve all registered candidates before checking opened terminals.
    /// A changed generation is rejected before and between bounded views.
    pub fn revalidate(&self) -> Result<(), HostAdmissionError> {
        native(self.validate_with_observer(&mut read_budget::UnobservedRead, |_, _| {}))
    }
    /// Observe bounded ledger views, reusable path scratch and native checks.
    /// Refused credits leave the ledger unchanged; accepted credits remain spent.
    /// Canonicalization's internal allocations and write-target resolution are
    /// outside these hooks, so this is not a whole-publication memory receipt.
    pub fn revalidate_with_budget<B: HostReadBudget>(
        &self,
        budget: &mut B,
    ) -> Result<(), HostReadError<B::Error>> {
        self.validate_with_observer(budget, |_, _| {})
    }
    /// Revalidate and report whether a target aliases a registered or opened input.
    pub fn conflicts_with_write_target(
        &self,
        target: &HostPath,
    ) -> Result<bool, HostAdmissionError> {
        self.revalidate_write_target(target)
    }
    /// Check immediately before publication. Target resolution holds no ledger
    /// mutex; membership uses the same generation that was just revalidated.
    pub fn revalidate_write_target(&self, target: &HostPath) -> Result<bool, HostAdmissionError> {
        self.revalidate()?;
        #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
        {
            let target = crate::platform::resolve_write_target(target)?;
            let state = self.owner.lock()?;
            if state.generation != self.generation {
                return Err(HostAdmissionError::ReadIdentityMismatch);
            }
            let opened = target
                .existing
                .is_some_and(|identity| match &state.opened_index {
                    Some(index) => index.contains(identity),
                    None => state.opened_slots.contains(&Some(identity)),
                });
            Ok(state.candidate_identities.contains(&target.candidate) || opened)
        }
        #[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
        {
            let _ = target;
            Err(HostAdmissionError::UnsupportedContainedOpen)
        }
    }
    #[cfg(test)]
    pub(crate) fn revalidate_with_observer_for_test(
        &self,
        observer: impl FnMut(bool, usize),
    ) -> Result<(), HostAdmissionError> {
        native(self.validate_with_observer(&mut read_budget::UnobservedRead, observer))
    }
}
