#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use crate::platform::AdmittedRoot;
use crate::platform::{CandidatePathIdentity, FileIdentity};
use crate::{HostAdmissionError, HostRootIdentity, HostSessionIdentity};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};
use typaxis_core::PortablePath;

/// Fixed maximum number of effective resource-root entries in one session.
pub const MAX_RESOURCE_ROOTS: usize = 64;

/// Fixed maximum number of logical host read-candidate attempts in one command.
pub const MAX_HOST_READ_CANDIDATES: usize = 131_072;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CandidateLocator {
    pub(crate) root: FileIdentity,
    pub(crate) path: PortablePath,
}

#[cfg_attr(
    not(any(target_os = "android", target_os = "linux", target_os = "macos")),
    allow(dead_code)
)]
#[derive(Debug)]
struct HostReadIdentityLedgerState {
    candidate_attempts: usize,
    candidates: BTreeMap<CandidateLocator, CandidatePathIdentity>,
    candidate_identities: BTreeSet<CandidatePathIdentity>,
    opened_by_candidate: BTreeMap<CandidateLocator, FileIdentity>,
    opened_identities: BTreeSet<FileIdentity>,
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
            .field("opened_identities", &state.opened_identities.len())
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
                opened_by_candidate: BTreeMap::new(),
                opened_identities: BTreeSet::new(),
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
        let snapshot = self.snapshot()?;
        Ok(HostReadIdentityLedgerToken::new(self, &snapshot))
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
    ) -> Result<(), HostAdmissionError> {
        let mut state = self.lock()?;
        if let Some(previous) = state.candidates.get(&locator) {
            if previous != &identity {
                return Err(HostAdmissionError::ReadIdentityMismatch);
            }
            return Ok(());
        }
        let generation = state
            .generation
            .checked_add(1)
            .ok_or(HostAdmissionError::HostLimit)?;
        state.roots.entry(locator.root).or_insert(root);
        state.candidate_identities.insert(identity.clone());
        state.candidates.insert(locator, identity);
        state.generation = generation;
        Ok(())
    }

    pub(crate) fn register_opened(
        &self,
        locator: CandidateLocator,
        identity: FileIdentity,
    ) -> Result<(), HostAdmissionError> {
        let mut state = self.lock()?;
        if let Some(previous) = state.opened_by_candidate.get(&locator) {
            if previous != &identity {
                return Err(HostAdmissionError::LengthMismatch);
            }
            return Ok(());
        }
        let generation = state
            .generation
            .checked_add(1)
            .ok_or(HostAdmissionError::HostLimit)?;
        state.opened_identities.insert(identity);
        state.opened_by_candidate.insert(locator, identity);
        state.generation = generation;
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> Result<HostReadLedgerSnapshot, HostAdmissionError> {
        let state = self.lock()?;
        Ok(HostReadLedgerSnapshot {
            candidate_attempts: state.candidate_attempts,
            candidates: state.candidates.clone(),
            candidate_identities: state.candidate_identities.clone(),
            opened_by_candidate: state.opened_by_candidate.clone(),
            opened_identities: state.opened_identities.clone(),
            #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
            roots: state.roots.clone(),
            generation: state.generation,
        })
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
#[derive(Clone, Debug)]
pub(crate) struct HostReadLedgerSnapshot {
    pub(crate) candidate_attempts: usize,
    pub(crate) candidates: BTreeMap<CandidateLocator, CandidatePathIdentity>,
    pub(crate) candidate_identities: BTreeSet<CandidatePathIdentity>,
    pub(crate) opened_by_candidate: BTreeMap<CandidateLocator, FileIdentity>,
    pub(crate) opened_identities: BTreeSet<FileIdentity>,
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    pub(crate) roots: BTreeMap<FileIdentity, Arc<AdmittedRoot>>,
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
    pub(crate) path: PortablePath,
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
    pub(crate) fn new(owner: &HostReadIdentityLedger, snapshot: &HostReadLedgerSnapshot) -> Self {
        Self {
            owner: owner.clone(),
            generation: snapshot.generation,
            candidate_attempts: snapshot.candidate_attempts,
            candidate_identities: snapshot.candidate_identities.len(),
            opened_identities: snapshot.opened_identities.len(),
        }
    }

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
