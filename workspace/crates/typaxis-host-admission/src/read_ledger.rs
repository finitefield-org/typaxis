#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use crate::platform::AdmittedRoot;
use crate::platform::{CandidatePathIdentity, FileIdentity};
use crate::{HostAdmissionError, HostRootIdentity, HostSessionIdentity};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};
use typaxis_core::PortablePath;
#[path = "validation_views.rs"]
mod validation_views;

/// Fixed maximum number of effective resource-root entries in one session.
pub const MAX_RESOURCE_ROOTS: usize = 64;

/// Fixed maximum number of logical host read-candidate attempts in one command.
pub const MAX_HOST_READ_CANDIDATES: usize = 131_072;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CandidateLocator {
    pub(crate) root: FileIdentity,
    pub(crate) path: Arc<PortablePath>,
}

#[derive(Debug)]
struct CandidateRecord {
    identity: Arc<CandidatePathIdentity>,
    opened_slot: usize,
}

#[cfg_attr(
    not(any(target_os = "android", target_os = "linux", target_os = "macos")),
    allow(dead_code)
)]
#[derive(Debug)]
struct HostReadIdentityLedgerState {
    candidate_attempts: usize,
    candidates: BTreeMap<CandidateLocator, CandidateRecord>,
    candidate_identities: BTreeSet<Arc<CandidatePathIdentity>>,
    opened_slots: Vec<Option<FileIdentity>>,
    opened_candidates: usize,
    opened_identity_count: usize,
    opened_index: Option<crate::opened_identity_index::Index>,
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    roots: BTreeMap<FileIdentity, Arc<AdmittedRoot>>,
    generation: u64,
}

/// Command-wide generic host read ledger. A command owner creates it before
/// admitting its first PACKAGE/config/source root; clones then share one fixed
/// candidate budget and identity state. Mutation remains restricted to
/// host-issued root capabilities, so constructing a ledger cannot forge read
/// identities.
#[derive(Clone)]
pub struct HostReadIdentityLedger {
    state: Arc<Mutex<HostReadIdentityLedgerState>>,
}

impl fmt::Debug for HostReadIdentityLedger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Ok(state) = self.state.lock() else {
            return formatter.write_str("HostReadIdentityLedger(poisoned)");
        };
        formatter
            .debug_struct("HostReadIdentityLedger")
            .field("candidate_attempts", &state.candidate_attempts)
            .field("candidate_identities", &state.candidate_identities.len())
            .field("opened_candidates", &state.opened_candidates)
            .finish_non_exhaustive()
    }
}

#[cfg_attr(
    not(any(target_os = "android", target_os = "linux", target_os = "macos")),
    allow(dead_code)
)]
impl HostReadIdentityLedger {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HostReadIdentityLedgerState {
                candidate_attempts: 0,
                candidates: BTreeMap::new(),
                candidate_identities: BTreeSet::new(),
                opened_slots: Vec::new(),
                opened_candidates: 0,
                opened_identity_count: 0,
                opened_index: None,
                #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
                roots: BTreeMap::new(),
                generation: 0,
            })),
        }
    }

    /// Seal the latest command-wide candidate/open identity generation for
    /// publication-time revalidation. A later registration makes this token
    /// stale rather than silently omitting the new read target.
    pub fn token(&self) -> Result<HostReadIdentityLedgerToken, HostAdmissionError> {
        let state = self.lock()?;
        Ok(HostReadIdentityLedgerToken {
            owner: self.clone(),
            generation: state.generation,
            candidate_attempts: state.candidate_attempts,
            candidate_identities: state.candidate_identities.len(),
            opened_identities: state.opened_identity_count,
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, HostReadIdentityLedgerState>, HostAdmissionError> {
        self.state.lock().map_err(|_| HostAdmissionError::Read)
    }

    pub(crate) fn reserve_candidate_attempts(
        &self,
        count: usize,
    ) -> Result<(), HostAdmissionError> {
        let mut state = self.lock()?;
        let next = state
            .candidate_attempts
            .checked_add(count)
            .ok_or(HostAdmissionError::HostLimit)?;
        if next > MAX_HOST_READ_CANDIDATES {
            return Err(HostAdmissionError::HostLimit);
        }
        let generation = if count == 0 {
            state.generation
        } else {
            state
                .generation
                .checked_add(1)
                .ok_or(HostAdmissionError::HostLimit)?
        };
        state.candidate_attempts = next;
        state.generation = generation;
        Ok(())
    }

    pub(crate) fn remaining_candidate_attempts(&self) -> Result<usize, HostAdmissionError> {
        Ok(MAX_HOST_READ_CANDIDATES - self.lock()?.candidate_attempts)
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    pub(crate) fn register_candidate(
        &self,
        locator: CandidateLocator,
        identity: CandidatePathIdentity,
        root: Arc<AdmittedRoot>,
    ) -> Result<usize, HostAdmissionError> {
        let mut state = self.lock()?;
        if let Some(previous) = state.candidates.get(&locator) {
            if previous.identity.as_ref() != &identity {
                return Err(HostAdmissionError::ReadIdentityMismatch);
            }
            return Ok(previous.opened_slot);
        }
        let opened_slot = state.opened_slots.len();
        if opened_slot == MAX_HOST_READ_CANDIDATES {
            return Err(HostAdmissionError::HostLimit);
        }
        state
            .opened_slots
            .try_reserve(1)
            .map_err(|_| HostAdmissionError::ReadCapacity)?;
        let generation = state
            .generation
            .checked_add(1)
            .ok_or(HostAdmissionError::HostLimit)?;
        state.roots.entry(locator.root).or_insert(root);
        let identity = Arc::new(identity);
        state.candidate_identities.insert(identity.clone());
        state.candidates.insert(
            locator,
            CandidateRecord {
                identity,
                opened_slot,
            },
        );
        state.opened_slots.push(None);
        state.generation = generation;
        Ok(opened_slot)
    }

    pub(crate) fn register_opened(
        &self,
        opened_slot: usize,
        identity: FileIdentity,
    ) -> Result<(), HostAdmissionError> {
        let mut state = self.lock()?;
        Self::record_opened(&mut state, opened_slot, identity)
    }

    fn record_opened(
        state: &mut HostReadIdentityLedgerState,
        opened_slot: usize,
        identity: FileIdentity,
    ) -> Result<(), HostAdmissionError> {
        let previous = state
            .opened_slots
            .get(opened_slot)
            .ok_or(HostAdmissionError::ReadIdentityMismatch)?;
        if let Some(previous) = previous {
            return if previous == &identity {
                Ok(())
            } else {
                Err(HostAdmissionError::LengthMismatch)
            };
        }
        let generation = state
            .generation
            .checked_add(1)
            .ok_or(HostAdmissionError::HostLimit)?;
        // Preserve distinct opened-file cardinality, including hard-link aliases,
        // without allocating a second identity index for token issuance.
        let unique = match &mut state.opened_index {
            Some(index) if index.contains(identity) => false,
            Some(index) if index.len() < index.capacity() => index.insert(identity)?,
            _ => {
                state.opened_index = None;
                !state.opened_slots.contains(&Some(identity))
            }
        };
        state.opened_identity_count += usize::from(unique);
        state.opened_slots[opened_slot] = Some(identity);
        state.opened_candidates += 1;
        state.generation = generation;
        Ok(())
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    pub(crate) fn register_opened_with_budget<B: crate::HostReadBudget>(
        &self,
        opened_slot: usize,
        identity: FileIdentity,
        budget: &mut B,
    ) -> Result<(), crate::HostReadError<B::Error>> {
        if !B::OBSERVED || !B::OBSERVE_OPEN_IDENTITIES {
            return self
                .register_opened(opened_slot, identity)
                .map_err(Into::into);
        }
        let (generation, attempts, slots, opened, prepare, copy_nodes) = {
            let state = self.lock()?;
            let previous = state
                .opened_slots
                .get(opened_slot)
                .ok_or(HostAdmissionError::ReadIdentityMismatch)?;
            if let Some(previous) = previous {
                return if previous == &identity {
                    Ok(())
                } else {
                    Err(HostAdmissionError::LengthMismatch.into())
                };
            }
            state
                .generation
                .checked_add(1)
                .ok_or(HostAdmissionError::HostLimit)?;
            let prepare = state
                .opened_index
                .as_ref()
                .map_or(true, |index| index.len() == index.capacity());
            (
                state.generation,
                state.candidate_attempts,
                state.opened_slots.len(),
                state.opened_candidates,
                prepare,
                state.opened_index.as_ref().map(|index| index.len()),
            )
        };
        let permission = |stage, buffer_bytes, work, budget: &mut B| {
            crate::read_budget::charge(
                budget,
                crate::HostReadRequest {
                    stage,
                    file_offset: 0,
                    buffer_bytes,
                    work,
                },
            )
        };
        let work = crate::host_open_identity_work_bound(attempts)?;
        let mut prepared = if prepare {
            let bytes = attempts
                .checked_mul(crate::HOST_OPEN_IDENTITY_NODE_BYTES)
                .and_then(|n| u64::try_from(n).ok())
                .ok_or(HostAdmissionError::HostLimit)?;
            permission(crate::HostReadStage::OpenIdentityStorage, bytes, 64, budget)?;
            let mut index = crate::opened_identity_index::Index::reserved(attempts)?;
            let migrate = if let Some(count) = copy_nodes {
                (count as u64)
                    .checked_mul(crate::HOST_OPEN_IDENTITY_NODE_BYTES as u64)
                    .and_then(|n| n.checked_add(64))
            } else {
                (slots as u64)
                    .checked_mul(std::mem::size_of::<Option<FileIdentity>>() as u64)
                    .and_then(|n| {
                        (opened as u64)
                            .checked_mul(work)
                            .and_then(|w| n.checked_add(w))
                    })
                    .and_then(|n| n.checked_add(64))
            }
            .ok_or(HostAdmissionError::HostLimit)?;
            permission(
                crate::HostReadStage::OpenIdentityMigrate,
                0,
                migrate,
                budget,
            )?;
            {
                let state = self.lock()?;
                if state.generation != generation {
                    return Err(HostAdmissionError::ReadIdentityMismatch.into());
                }
                if let Some(old) = &state.opened_index {
                    old.copy_into(&mut index);
                } else {
                    for identity in state.opened_slots.iter().flatten() {
                        index.insert(*identity)?;
                    }
                }
            }
            Some(index)
        } else {
            None
        };
        permission(crate::HostReadStage::OpenIdentityLookup, 0, work, budget)?;
        let mut state = self.lock()?;
        if state.generation != generation {
            return Err(HostAdmissionError::ReadIdentityMismatch.into());
        }
        let index = prepared
            .as_mut()
            .or(state.opened_index.as_mut())
            .ok_or(HostAdmissionError::ReadIdentityMismatch)?;
        let unique = index.insert(identity)?;
        if let Some(index) = prepared {
            state.opened_index = Some(index);
        }
        state.opened_identity_count += usize::from(unique);
        state.opened_slots[opened_slot] = Some(identity);
        state.opened_candidates += 1;
        state.generation += 1;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn snapshot(&self) -> Result<HostReadLedgerSnapshot, HostAdmissionError> {
        let state = self.lock()?;
        Ok(Self::copy_snapshot(&state))
    }
    #[cfg(test)]
    fn copy_snapshot(state: &HostReadIdentityLedgerState) -> HostReadLedgerSnapshot {
        HostReadLedgerSnapshot {
            candidates: state
                .candidates
                .iter()
                .map(|(key, value)| (key.clone(), value.identity.as_ref().clone()))
                .collect(),
            candidate_identities: state
                .candidate_identities
                .iter()
                .map(|v| v.as_ref().clone())
                .collect(),
            opened_by_candidate: state
                .candidates
                .iter()
                .filter_map(|(key, value)| {
                    state.opened_slots[value.opened_slot].map(|identity| (key.clone(), identity))
                })
                .collect(),
            opened_identities: state.opened_slots.iter().copied().flatten().collect(),
            #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
            roots: state.roots.clone(),
            #[cfg(test)]
            generation: state.generation,
        }
    }
}

impl Default for HostReadIdentityLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(
    not(any(target_os = "android", target_os = "linux", target_os = "macos")),
    allow(dead_code)
)]
#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct HostReadLedgerSnapshot {
    pub(crate) candidates: BTreeMap<CandidateLocator, CandidatePathIdentity>,
    pub(crate) candidate_identities: BTreeSet<CandidatePathIdentity>,
    pub(crate) opened_by_candidate: BTreeMap<CandidateLocator, FileIdentity>,
    pub(crate) opened_identities: BTreeSet<FileIdentity>,
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    pub(crate) roots: BTreeMap<FileIdentity, Arc<AdmittedRoot>>,
    #[cfg(test)]
    pub(crate) generation: u64,
}

/// Opaque proof that one portable path has had all root-relative logical
/// candidate attempts reserved and registered.
#[cfg_attr(
    not(any(target_os = "android", target_os = "linux", target_os = "macos")),
    allow(dead_code)
)]
pub struct RegisteredHostReadCandidate {
    pub(crate) session: HostSessionIdentity,
    pub(crate) roots: HostRootIdentity,
    pub(crate) path: Arc<PortablePath>,
    pub(crate) opened_slots: Vec<usize>,
}

impl fmt::Debug for RegisteredHostReadCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RegisteredHostReadCandidate(..)")
    }
}

/// Owned, sealed view of the latest generic candidate and opened identities.
/// A token becomes stale if any session sharing its ledger changes the facts.
///
/// ```compile_fail
/// use typaxis_host_admission::HostReadIdentityLedgerToken;
/// let _forged: HostReadIdentityLedgerToken = HostReadIdentityLedgerToken {};
/// ```
pub struct HostReadIdentityLedgerToken {
    pub(crate) owner: HostReadIdentityLedger,
    pub(crate) generation: u64,
    candidate_attempts: usize,
    candidate_identities: usize,
    opened_identities: usize,
}

impl fmt::Debug for HostReadIdentityLedgerToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostReadIdentityLedgerToken")
            .field("candidate_attempts", &self.candidate_attempts)
            .field("candidate_identities", &self.candidate_identities)
            .field("opened_identities", &self.opened_identities)
            .finish_non_exhaustive()
    }
}

impl HostReadIdentityLedgerToken {
    pub const fn candidate_attempt_count(&self) -> usize {
        self.candidate_attempts
    }

    pub const fn stored_candidate_identity_count(&self) -> usize {
        self.candidate_identities
    }

    pub const fn stored_opened_identity_count(&self) -> usize {
        self.opened_identities
    }
}
