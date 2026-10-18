//! Registered paths and their opened slots are allocated during registration.
//! An observed open funds only reusable scratch, bounded work and its identity.
use crate::{
    read_budget, read_resolution, CandidateLocator, HostAdmissionError, HostAdmissionSession,
    HostReadBinding, HostReadBudget, HostReadError, HostReadIdentity, HostReadRequest,
    HostReadStage, OpenedContainedFile, RegisteredHostReadCandidate,
};

impl HostAdmissionSession {
    pub(super) fn open_registered_with_budget<B: HostReadBudget>(
        &self,
        registered: &RegisteredHostReadCandidate,
        budget: &mut B,
    ) -> Result<OpenedContainedFile<'_>, HostReadError<B::Error>> {
        self.validate_registered(registered)?;
        if !B::OBSERVED || !B::OBSERVE_OPEN {
            return self.open_registered(registered).map_err(Into::into);
        }
        let mut prepared =
            read_resolution::prepare_path(&registered.path, HostReadStage::OpenPath, budget)?;
        let mut candidate = None;
        for (index, root) in self.host_roots.iter().enumerate() {
            read_budget::charge(
                budget,
                HostReadRequest {
                    stage: HostReadStage::OpenRoot,
                    file_offset: 0,
                    buffer_bytes: 0,
                    work: prepared.root_work()?,
                },
            )?;
            let Some(opened) = prepared.open(root)? else {
                continue;
            };
            if candidate.is_some() {
                return Err(HostAdmissionError::AmbiguousCandidate.into());
            }
            candidate = Some((root.identity(), opened, registered.opened_slots[index]));
        }
        let (root, opened, slot) = candidate.ok_or(HostAdmissionError::MissingCandidate)?;
        read_budget::charge(
            budget,
            HostReadRequest {
                stage: HostReadStage::OpenLock,
                file_offset: 0,
                buffer_bytes: 0,
                work: 128,
            },
        )?;
        let (file, snapshot) = opened.lock()?;
        // Callbacks run outside the ledger mutex. All credits precede mutation,
        // so every quota refusal leaves the ledger generation unchanged.
        read_budget::charge(
            budget,
            HostReadRequest {
                stage: HostReadStage::OpenLedger,
                file_offset: 0,
                buffer_bytes: 0,
                work: 64,
            },
        )?;
        read_budget::charge(
            budget,
            HostReadRequest {
                stage: HostReadStage::OpenOwner,
                file_offset: 0,
                buffer_bytes: std::mem::size_of::<[std::sync::atomic::AtomicUsize; 2]>() as u64,
                work: 64,
            },
        )?;
        self.ledger.register_opened_with_budget(slot, snapshot.identity, budget)?;
        let read =
            HostReadIdentity::new(self.session.clone(), self.roots.clone(), snapshot.identity);
        Ok(OpenedContainedFile {
            owner: self,
            file,
            snapshot,
            candidate: CandidateLocator {
                root,
                path: registered.path.clone(),
            },
            binding: HostReadBinding {
                session: self.session.clone(),
                roots: self.roots.clone(),
                read,
            },
            observed_exact_length: snapshot.length,
            _session: std::marker::PhantomData,
        })
    }
}
