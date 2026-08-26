#![forbid(unsafe_code)]

//! Generic, host-only admission primitives.
//!
//! This crate owns configured directory capabilities, contained file opens,
//! same-handle snapshots, and exact-length reads. It deliberately has no
//! package, font, image, manifest, or diagnostic vocabulary. Domain owners
//! reserve their own budgets after inspecting [`OpenedContainedFile::observed_exact_length`]
//! and only then request a [`BoundedReadPermit`].

mod platform;
mod read_ledger;

pub use read_ledger::{
    HostReadIdentityLedger, HostReadIdentityLedgerToken, RegisteredHostReadCandidate,
    MAX_HOST_READ_CANDIDATES, MAX_RESOURCE_ROOTS,
};

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::collections::BTreeSet;
use std::fmt;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::fs::File;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::io::Read;
use std::marker::PhantomData;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use typaxis_core::ConfigResourceRoot;
use typaxis_core::{EffectiveConfig, HostAdmissionContext, HostPath, PortablePath};

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use read_ledger::CandidateLocator;
use read_ledger::HostReadLedgerSnapshot;

/// Failure while establishing or using a host admission capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostAdmissionError {
    HostLimit,
    ReadCapacity,
    Read,
    LengthMismatch,
    SessionMismatch,
    RootSetMismatch,
    ReadIdentityMismatch,
    RootUnavailable,
    RootNotDirectory,
    AliasedRoot,
    UnsupportedContainedOpen,
    MissingCandidate,
    AmbiguousCandidate,
    UnsafeCandidate,
    NotRegularFile,
    LockUnavailable,
}

/// Compile-time contained-open availability. This token has no public raw
/// constructor, so command/profile owners consume the same target-derived fact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostCapabilityToken {
    contained_open_available: bool,
}

impl HostCapabilityToken {
    pub const fn compiled() -> Self {
        Self {
            contained_open_available: platform::CONTAINED_OPEN_AVAILABLE,
        }
    }

    pub const fn contained_open_available(self) -> bool {
        self.contained_open_available
    }

    pub const fn contained_package_open_available(self) -> bool {
        self.contained_open_available
    }

    pub const fn contained_resource_open_available(self) -> bool {
        self.contained_open_available
    }
}

/// Compile-time contained-resource primitive consumed by the resource
/// admission owner. This sealed projection lets capability composition use
/// the exact target fact enforced by [`HostAdmissionSession::new`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostResourceCapabilityToken {
    available: bool,
}

impl HostResourceCapabilityToken {
    pub const fn compiled() -> Self {
        Self {
            available: HostCapabilityToken::compiled().contained_resource_open_available(),
        }
    }

    pub const fn contained_resource_open(self) -> bool {
        self.available
    }
}

/// Compile-time primitive consumed by atomic file publication owners.
///
/// This has no public raw constructor. Publication context setup and machine
/// capability composition therefore share one target-derived fact instead of
/// independently spelling platform `cfg` expressions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicFilePublicationCapabilityToken {
    available: bool,
}

impl AtomicFilePublicationCapabilityToken {
    pub const fn compiled() -> Self {
        Self {
            available: cfg!(unix),
        }
    }

    pub const fn available(self) -> bool {
        self.available
    }
}

#[derive(Debug)]
struct IdentityMarker;

macro_rules! opaque_identity {
    ($visibility:vis $name:ident, $label:literal) => {
        #[derive(Clone)]
        $visibility struct $name(Arc<IdentityMarker>);

        impl $name {
            #[allow(dead_code)] // issued only by supported contained-open targets
            fn fresh() -> Self {
                Self(Arc::new(IdentityMarker))
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.0, &other.0)
            }
        }

        impl Eq for $name {}

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str($label)
            }
        }
    };
}

opaque_identity!(pub HostSessionIdentity, "HostSessionIdentity(..)");
opaque_identity!(pub HostRootIdentity, "HostRootIdentity(..)");
opaque_identity!(HostOpenIdentity, "HostOpenIdentity(..)");

/// Opaque identity for one same-handle read in one admitted session/root set.
///
/// Equality is useful to bind a later stable-bytes receipt to the exact open;
/// its platform file identity is intentionally not exposed.
#[derive(Clone, Eq, PartialEq)]
pub struct HostReadIdentity {
    session: HostSessionIdentity,
    roots: HostRootIdentity,
    opened: HostOpenIdentity,
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    file: platform::FileIdentity,
}

impl fmt::Debug for HostReadIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HostReadIdentity(..)")
    }
}

impl HostReadIdentity {
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn new(
        session: HostSessionIdentity,
        roots: HostRootIdentity,
        file: platform::FileIdentity,
    ) -> Self {
        Self {
            session,
            roots,
            opened: HostOpenIdentity::fresh(),
            file,
        }
    }

    pub const fn session_identity(&self) -> &HostSessionIdentity {
        &self.session
    }

    pub const fn root_identity(&self) -> &HostRootIdentity {
        &self.roots
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HostReadBinding {
    session: HostSessionIdentity,
    roots: HostRootIdentity,
    read: HostReadIdentity,
}

/// Owner of one admitted root set and its host session identity.
///
/// `new` consumes the already-resolved application context/config rather than
/// accepting a raw path array. Every root is opened and de-aliased before the
/// capability is issued.
#[derive(Debug)]
pub struct HostAdmissionSession {
    session: HostSessionIdentity,
    roots: HostRootIdentity,
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    host_roots: Vec<Arc<platform::AdmittedRoot>>,
    ledger: HostReadIdentityLedger,
}

impl HostAdmissionSession {
    /// Admit the effective configured resource roots as a generic root set.
    ///
    /// The resulting open/read API has no knowledge of logical resources.
    /// Linux and Android use `openat2` with a component-walker fallback;
    /// macOS uses the same `openat(O_NOFOLLOW)` component walker. Unsupported
    /// targets fail closed before root identity resolution or handle opening.
    pub fn new(
        context: &HostAdmissionContext,
        config: &EffectiveConfig,
    ) -> Result<Self, HostAdmissionError> {
        let ledger = HostReadIdentityLedger::new();
        Self::new_with_read_ledger(context, config, &ledger)
    }

    /// Admit resource roots into an existing command-wide read ledger.
    pub fn new_with_read_ledger(
        context: &HostAdmissionContext,
        config: &EffectiveConfig,
        ledger: &HostReadIdentityLedger,
    ) -> Result<Self, HostAdmissionError> {
        if !HostCapabilityToken::compiled().contained_resource_open_available() {
            return Err(HostAdmissionError::UnsupportedContainedOpen);
        }
        #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
        {
            let host_roots = admit_host_roots(context, config)?;
            Ok(Self {
                session: HostSessionIdentity::fresh(),
                roots: HostRootIdentity::fresh(),
                host_roots,
                ledger: ledger.clone(),
            })
        }
        #[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
        {
            let _ = (context, config, ledger);
            Err(HostAdmissionError::UnsupportedContainedOpen)
        }
    }

    /// Admit one typed root for PACKAGE, config, or source candidates. This is
    /// deliberately a single root rather than a caller-supplied raw root array.
    pub fn new_contained_root(root: &HostPath) -> Result<Self, HostAdmissionError> {
        let ledger = HostReadIdentityLedger::new();
        Self::new_contained_root_with_read_ledger(root, &ledger)
    }

    /// Admit one typed root while sharing a command-wide candidate budget and
    /// identity ledger with previously admitted host sessions.
    pub fn new_contained_root_with_read_ledger(
        root: &HostPath,
        ledger: &HostReadIdentityLedger,
    ) -> Result<Self, HostAdmissionError> {
        if !HostCapabilityToken::compiled().contained_package_open_available() {
            return Err(HostAdmissionError::UnsupportedContainedOpen);
        }
        #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
        {
            let root = Arc::new(platform::admit_root(root.as_path())?);
            Ok(Self {
                session: HostSessionIdentity::fresh(),
                roots: HostRootIdentity::fresh(),
                host_roots: vec![root],
                ledger: ledger.clone(),
            })
        }
        #[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
        {
            let _ = (root, ledger);
            Err(HostAdmissionError::UnsupportedContainedOpen)
        }
    }

    pub const fn session_identity(&self) -> &HostSessionIdentity {
        &self.session
    }

    pub const fn root_identity(&self) -> &HostRootIdentity {
        &self.roots
    }

    pub const fn read_ledger(&self) -> &HostReadIdentityLedger {
        &self.ledger
    }

    pub const fn roots(&self) -> HostRootSetToken<'_> {
        HostRootSetToken { owner: self }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn register_candidates<'path>(
        &self,
        paths: impl Iterator<Item = &'path PortablePath>,
    ) -> Result<Vec<RegisteredHostReadCandidate>, HostAdmissionError> {
        let root_count = self.host_roots.len();
        let remaining_attempts = self.ledger.remaining_candidate_attempts()?;
        let max_paths = remaining_attempts
            .checked_div(root_count)
            .unwrap_or(MAX_HOST_READ_CANDIDATES);
        let mut paths_buffer = Vec::new();
        for path in paths {
            if paths_buffer.len() == max_paths {
                return Err(HostAdmissionError::HostLimit);
            }
            paths_buffer
                .try_reserve(1)
                .map_err(|_| HostAdmissionError::ReadCapacity)?;
            paths_buffer.push(path);
        }
        let path_count = paths_buffer.len();
        let attempt_count = path_count
            .checked_mul(root_count)
            .ok_or(HostAdmissionError::HostLimit)?;
        self.ledger.reserve_candidate_attempts(attempt_count)?;

        let mut registered = Vec::new();
        registered
            .try_reserve_exact(path_count)
            .map_err(|_| HostAdmissionError::ReadCapacity)?;
        for path in paths_buffer {
            for root in &self.host_roots {
                let identity = root.candidate_parent_leaf(path)?;
                self.ledger.register_candidate(
                    CandidateLocator {
                        root: root.identity(),
                        path: path.clone(),
                    },
                    identity,
                    Arc::clone(root),
                )?;
            }
            registered.push(RegisteredHostReadCandidate {
                session: self.session.clone(),
                roots: self.roots.clone(),
                path: path.clone(),
            });
        }
        Ok(registered)
    }

    #[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
    fn register_candidates<'path>(
        &self,
        _paths: impl Iterator<Item = &'path PortablePath>,
    ) -> Result<Vec<RegisteredHostReadCandidate>, HostAdmissionError> {
        Err(HostAdmissionError::UnsupportedContainedOpen)
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn open_registered(
        &self,
        registered: &RegisteredHostReadCandidate,
    ) -> Result<OpenedContainedFile<'_>, HostAdmissionError> {
        self.validate_registered(registered)?;
        let mut candidate = None;
        for root in &self.host_roots {
            let Some(opened) = root.open_candidate(&registered.path)? else {
                continue;
            };
            if candidate.is_some() {
                return Err(HostAdmissionError::AmbiguousCandidate);
            }
            candidate = Some((root.identity(), opened));
        }
        let (root, opened) = candidate.ok_or(HostAdmissionError::MissingCandidate)?;
        let (file, snapshot) = opened.lock()?;
        let locator = CandidateLocator {
            root,
            path: registered.path.clone(),
        };
        self.ledger
            .register_opened(locator.clone(), snapshot.identity)?;
        let read =
            HostReadIdentity::new(self.session.clone(), self.roots.clone(), snapshot.identity);
        Ok(OpenedContainedFile {
            owner: self,
            file,
            snapshot,
            candidate: locator,
            binding: HostReadBinding {
                session: self.session.clone(),
                roots: self.roots.clone(),
                read,
            },
            observed_exact_length: snapshot.length,
            _session: PhantomData,
        })
    }

    #[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
    fn open_registered(
        &self,
        _registered: &RegisteredHostReadCandidate,
    ) -> Result<OpenedContainedFile<'_>, HostAdmissionError> {
        Err(HostAdmissionError::UnsupportedContainedOpen)
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn validate_registered(
        &self,
        registered: &RegisteredHostReadCandidate,
    ) -> Result<(), HostAdmissionError> {
        if registered.session != self.session {
            return Err(HostAdmissionError::SessionMismatch);
        }
        if registered.roots != self.roots {
            return Err(HostAdmissionError::RootSetMismatch);
        }
        Ok(())
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn revalidate_open_resolution(
        &self,
        path: &PortablePath,
        expected: platform::FileIdentity,
    ) -> Result<(), HostAdmissionError> {
        let mut found = None;
        for root in &self.host_roots {
            let opened = root
                .open_candidate(path)
                .map_err(|_| HostAdmissionError::LengthMismatch)?;
            let Some(opened) = opened else {
                continue;
            };
            let identity = opened.identity();
            if found.is_some() {
                return Err(HostAdmissionError::LengthMismatch);
            }
            found = Some(identity);
        }
        if found != Some(expected) {
            return Err(HostAdmissionError::LengthMismatch);
        }
        Ok(())
    }
}

/// Borrowed capability for exactly one admitted root set and host session.
#[derive(Clone, Copy)]
pub struct HostRootSetToken<'session> {
    owner: &'session HostAdmissionSession,
}

impl fmt::Debug for HostRootSetToken<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HostRootSetToken(..)")
    }
}

impl<'session> HostRootSetToken<'session> {
    pub const fn session_identity(self) -> &'session HostSessionIdentity {
        self.owner.session_identity()
    }

    pub const fn root_identity(self) -> &'session HostRootIdentity {
        self.owner.root_identity()
    }

    /// Open one portable path beneath the admitted roots and retain the exact
    /// same handle, lock, snapshot, and observed extent for a later read.
    pub fn open(
        self,
        uri: &PortablePath,
    ) -> Result<OpenedContainedFile<'session>, HostAdmissionError> {
        let mut registered = self.owner.register_candidates(std::iter::once(uri))?;
        let candidate = registered
            .pop()
            .ok_or(HostAdmissionError::MissingCandidate)?;
        self.owner.open_registered(&candidate)
    }

    /// Reserve the complete `paths × admitted roots` work product before any
    /// candidate component is inspected, then register each parent+leaf.
    pub fn register_candidates<'path, I>(
        self,
        paths: I,
    ) -> Result<Vec<RegisteredHostReadCandidate>, HostAdmissionError>
    where
        I: IntoIterator<Item = &'path PortablePath>,
    {
        self.owner.register_candidates(paths.into_iter())
    }

    /// Open a previously registered path without consuming the candidate work
    /// budget a second time.
    pub fn open_registered(
        self,
        registered: &RegisteredHostReadCandidate,
    ) -> Result<OpenedContainedFile<'session>, HostAdmissionError> {
        self.owner.open_registered(registered)
    }

    /// Issue a non-cloneable view of the latest ledger facts for write-target
    /// comparison and immediate revalidation.
    pub fn read_ledger_token(self) -> Result<HostReadIdentityLedgerToken, HostAdmissionError> {
        self.owner.ledger.token()
    }

    pub fn validate_opened(
        self,
        opened: &OpenedContainedFile<'_>,
    ) -> Result<(), HostAdmissionError> {
        self.validate_binding(&opened.binding)
    }

    /// Issue the bounded-read capability after the domain owner has reserved
    /// `opened.observed_exact_length()` against its own budgets.
    pub fn issue_bounded_read_permit<'opened>(
        self,
        opened: OpenedContainedFile<'opened>,
    ) -> Result<BoundedReadPermit<'opened>, HostAdmissionError> {
        self.validate_opened(&opened)?;
        Ok(BoundedReadPermit { opened })
    }

    /// Consume a permit and read exactly the opener-observed length from the
    /// retained handle. No caller-provided extent and no arbitrary `Read`
    /// implementation participates in this trust path.
    pub fn read_bounded(
        self,
        permit: BoundedReadPermit<'_>,
    ) -> Result<StableFileBytesReceipt, HostAdmissionError> {
        self.validate_binding(&permit.opened.binding)?;
        read_bounded(permit)
    }

    /// Verify a receipt against this session/root and the identity retained by
    /// the logical owner at open time, then release its owned stable bytes.
    pub fn accept_receipt(
        self,
        expected_read: &HostReadIdentity,
        receipt: StableFileBytesReceipt,
    ) -> Result<StableFileBytes, HostAdmissionError> {
        self.validate_read_identity(expected_read)?;
        self.validate_binding(&receipt.binding)?;
        if &receipt.binding.read != expected_read {
            return Err(HostAdmissionError::ReadIdentityMismatch);
        }
        if u64::try_from(receipt.bytes.len()).ok() != Some(receipt.observed_exact_length) {
            return Err(HostAdmissionError::LengthMismatch);
        }
        Ok(StableFileBytes {
            identity: receipt.binding.read,
            observed_exact_length: receipt.observed_exact_length,
            bytes: receipt.bytes,
            sha256: receipt.sha256,
        })
    }

    fn validate_read_identity(self, identity: &HostReadIdentity) -> Result<(), HostAdmissionError> {
        if identity.session != *self.session_identity() {
            return Err(HostAdmissionError::SessionMismatch);
        }
        if identity.roots != *self.root_identity() {
            return Err(HostAdmissionError::RootSetMismatch);
        }
        Ok(())
    }

    fn validate_binding(self, binding: &HostReadBinding) -> Result<(), HostAdmissionError> {
        if binding.session != *self.session_identity() {
            return Err(HostAdmissionError::SessionMismatch);
        }
        if binding.roots != *self.root_identity() {
            return Err(HostAdmissionError::RootSetMismatch);
        }
        if binding.read.session != binding.session || binding.read.roots != binding.roots {
            return Err(HostAdmissionError::ReadIdentityMismatch);
        }
        Ok(())
    }
}

impl HostReadIdentityLedgerToken {
    fn current_snapshot(&self) -> Result<HostReadLedgerSnapshot, HostAdmissionError> {
        let snapshot = self.owner.snapshot()?;
        if snapshot.generation != self.generation {
            return Err(HostAdmissionError::ReadIdentityMismatch);
        }
        Ok(snapshot)
    }

    /// Re-resolve every registered parent+leaf and every opened terminal.
    /// Any replacement, disappearance, new symlink, or root-relative identity
    /// change invalidates the token.
    pub fn revalidate(&self) -> Result<(), HostAdmissionError> {
        let snapshot = self.current_snapshot()?;
        #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
        {
            for (locator, expected) in &snapshot.candidates {
                let root = snapshot
                    .roots
                    .get(&locator.root)
                    .ok_or(HostAdmissionError::RootSetMismatch)?;
                if &root.candidate_parent_leaf(&locator.path)? != expected {
                    return Err(HostAdmissionError::ReadIdentityMismatch);
                }
            }
            for (locator, expected) in &snapshot.opened_by_candidate {
                let root = snapshot
                    .roots
                    .get(&locator.root)
                    .ok_or(HostAdmissionError::RootSetMismatch)?;
                let opened = root
                    .open_candidate(&locator.path)?
                    .ok_or(HostAdmissionError::ReadIdentityMismatch)?;
                if &opened.identity() != expected {
                    return Err(HostAdmissionError::ReadIdentityMismatch);
                }
            }
            Ok(())
        }
        #[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
        {
            let _ = snapshot;
            Err(HostAdmissionError::UnsupportedContainedOpen)
        }
    }

    /// Revalidate this sealed ledger and report whether `target` aliases a
    /// registered logical candidate or an opened file identity.
    pub fn conflicts_with_write_target(
        &self,
        target: &HostPath,
    ) -> Result<bool, HostAdmissionError> {
        self.revalidate_write_target(target)
    }

    /// Publication-facing spelling of [`Self::conflicts_with_write_target`].
    /// Call immediately before a write-target mutation and reject `true`.
    pub fn revalidate_write_target(&self, target: &HostPath) -> Result<bool, HostAdmissionError> {
        self.revalidate()?;
        let snapshot = self.current_snapshot()?;
        #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
        {
            let target = platform::resolve_write_target(target)?;
            Ok(snapshot.candidate_identities.contains(&target.candidate)
                || target
                    .existing
                    .is_some_and(|identity| snapshot.opened_identities.contains(&identity)))
        }
        #[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
        {
            let _ = (snapshot, target);
            Err(HostAdmissionError::UnsupportedContainedOpen)
        }
    }
}

/// A contained, locked regular-file handle with a same-handle snapshot.
///
/// Its fields are private and there is no constructor from `Read` or from a
/// caller-supplied length.
///
/// ```compile_fail
/// use typaxis_host_admission::OpenedContainedFile;
/// let _forged: OpenedContainedFile<'static> = OpenedContainedFile {};
/// ```
pub struct OpenedContainedFile<'session> {
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    owner: &'session HostAdmissionSession,
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    file: File,
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    snapshot: platform::FileSnapshot,
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    candidate: CandidateLocator,
    binding: HostReadBinding,
    observed_exact_length: u64,
    _session: PhantomData<&'session HostAdmissionSession>,
}

impl fmt::Debug for OpenedContainedFile<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenedContainedFile")
            .field("observed_exact_length", &self.observed_exact_length)
            .field("identity", &self.binding.read)
            .finish_non_exhaustive()
    }
}

impl OpenedContainedFile<'_> {
    pub const fn observed_exact_length(&self) -> u64 {
        self.observed_exact_length
    }

    pub const fn read_identity(&self) -> &HostReadIdentity {
        &self.binding.read
    }

    pub const fn session_identity(&self) -> &HostSessionIdentity {
        &self.binding.session
    }

    pub const fn root_identity(&self) -> &HostRootIdentity {
        &self.binding.roots
    }
}

/// Exact-length read capability issued only from an [`OpenedContainedFile`].
pub struct BoundedReadPermit<'session> {
    opened: OpenedContainedFile<'session>,
}

impl fmt::Debug for BoundedReadPermit<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundedReadPermit")
            .field("opened", &self.opened)
            .finish()
    }
}

/// Unforgeable result of one stable, exact-length same-handle read.
///
/// Bytes are released only through [`HostRootSetToken::accept_receipt`], which
/// rejects session, root, and read-identity swaps.
pub struct StableFileBytesReceipt {
    binding: HostReadBinding,
    observed_exact_length: u64,
    bytes: Vec<u8>,
    sha256: [u8; 32],
}

impl fmt::Debug for StableFileBytesReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StableFileBytesReceipt")
            .field("observed_exact_length", &self.observed_exact_length)
            .field("identity", &self.binding.read)
            .finish_non_exhaustive()
    }
}

impl StableFileBytesReceipt {
    pub const fn observed_exact_length(&self) -> u64 {
        self.observed_exact_length
    }

    pub const fn read_identity(&self) -> &HostReadIdentity {
        &self.binding.read
    }

    pub const fn session_identity(&self) -> &HostSessionIdentity {
        &self.binding.session
    }

    pub const fn root_identity(&self) -> &HostRootIdentity {
        &self.binding.roots
    }
}

/// Stable bytes after the expected open identity and session/root have been
/// checked. Logical owners consume this value to bind their own IDs/policy.
pub struct StableFileBytes {
    identity: HostReadIdentity,
    observed_exact_length: u64,
    bytes: Vec<u8>,
    sha256: [u8; 32],
}

impl fmt::Debug for StableFileBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StableFileBytes")
            .field("observed_exact_length", &self.observed_exact_length)
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}

impl StableFileBytes {
    pub const fn read_identity(&self) -> &HostReadIdentity {
        &self.identity
    }

    pub const fn observed_exact_length(&self) -> u64 {
        self.observed_exact_length
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub const fn sha256(&self) -> [u8; 32] {
        self.sha256
    }

    pub fn into_bytes_and_sha256(self) -> (Vec<u8>, [u8; 32]) {
        (self.bytes, self.sha256)
    }
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
fn read_bounded(
    permit: BoundedReadPermit<'_>,
) -> Result<StableFileBytesReceipt, HostAdmissionError> {
    read_bounded_with_observer(permit, || {})
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
fn read_bounded_with_observer(
    permit: BoundedReadPermit<'_>,
    observer: impl FnOnce(),
) -> Result<StableFileBytesReceipt, HostAdmissionError> {
    let BoundedReadPermit { opened } = permit;
    let OpenedContainedFile {
        owner,
        mut file,
        snapshot,
        candidate,
        binding,
        observed_exact_length,
        _session: _,
    } = opened;
    let capacity =
        usize::try_from(observed_exact_length).map_err(|_| HostAdmissionError::ReadCapacity)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| HostAdmissionError::ReadCapacity)?;
    let mut hasher = StreamingSha256::new();
    let mut remaining = observed_exact_length;
    let mut chunk = [0u8; 8192];
    let mut observer = Some(observer);
    while remaining > 0 {
        let allowed = usize::try_from(remaining.min(chunk.len() as u64))
            .map_err(|_| HostAdmissionError::ReadCapacity)?;
        let read = file
            .read(&mut chunk[..allowed])
            .map_err(|_| HostAdmissionError::Read)?;
        if read == 0 {
            return Err(HostAdmissionError::LengthMismatch);
        }
        hasher.update(&chunk[..read]);
        bytes.extend_from_slice(&chunk[..read]);
        remaining -= u64::try_from(read).map_err(|_| HostAdmissionError::ReadCapacity)?;
        if let Some(observer) = observer.take() {
            observer();
        }
    }
    let after_read = platform::FileSnapshot::from_file(&file)?;
    if after_read != snapshot {
        return Err(HostAdmissionError::LengthMismatch);
    }
    owner.revalidate_open_resolution(&candidate.path, snapshot.identity)?;
    Ok(StableFileBytesReceipt {
        binding,
        observed_exact_length,
        bytes,
        sha256: hasher.finalize(),
    })
}

#[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
fn read_bounded(
    _permit: BoundedReadPermit<'_>,
) -> Result<StableFileBytesReceipt, HostAdmissionError> {
    Err(HostAdmissionError::UnsupportedContainedOpen)
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
fn admit_host_roots(
    context: &HostAdmissionContext,
    config: &EffectiveConfig,
) -> Result<Vec<Arc<platform::AdmittedRoot>>, HostAdmissionError> {
    let root_count = config
        .resource_roots()
        .len()
        .checked_add(context.cli_resource_roots().len())
        .ok_or(HostAdmissionError::HostLimit)?;
    if root_count > MAX_RESOURCE_ROOTS {
        return Err(HostAdmissionError::HostLimit);
    }

    let project_root = context.project_root().as_path();
    let configured = config.resource_roots().iter().map(|root| match root {
        ConfigResourceRoot::ProjectRoot => project_root.to_path_buf(),
        ConfigResourceRoot::Relative(path) => project_root.join(portable_to_host_path(path)),
    });
    let explicit = context
        .cli_resource_roots()
        .iter()
        .map(|root| root.as_path().to_path_buf());
    let mut identities = BTreeSet::new();
    let mut roots = Vec::new();
    for path in configured.chain(explicit) {
        let root = Arc::new(platform::admit_root(&path)?);
        if !identities.insert(root.identity()) {
            return Err(HostAdmissionError::AliasedRoot);
        }
        roots.push(root);
    }
    Ok(roots)
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
fn portable_to_host_path(path: &PortablePath) -> PathBuf {
    path.as_str().split('/').collect()
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[derive(Clone, Debug)]
struct StreamingSha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffered: usize,
    byte_length: u64,
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
impl StreamingSha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            buffered: 0,
            byte_length: 0,
        }
    }

    fn update(&mut self, mut input: &[u8]) {
        self.byte_length = self.byte_length.wrapping_add(input.len() as u64);
        if self.buffered != 0 {
            let copied = (64 - self.buffered).min(input.len());
            self.buffer[self.buffered..self.buffered + copied].copy_from_slice(&input[..copied]);
            self.buffered += copied;
            input = &input[copied..];
            if self.buffered == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
        while input.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&input[..64]);
            self.compress(&block);
            input = &input[64..];
        }
        self.buffer[..input.len()].copy_from_slice(input);
        self.buffered = input.len();
    }

    fn finalize(mut self) -> [u8; 32] {
        let bit_length = self.byte_length.wrapping_mul(8);
        let mut final_blocks = [0u8; 128];
        final_blocks[..self.buffered].copy_from_slice(&self.buffer[..self.buffered]);
        final_blocks[self.buffered] = 0x80;
        let used = if self.buffered < 56 { 64 } else { 128 };
        final_blocks[used - 8..used].copy_from_slice(&bit_length.to_be_bytes());
        for block in final_blocks[..used].chunks_exact(64) {
            let mut owned = [0u8; 64];
            owned.copy_from_slice(block);
            self.compress(&owned);
        }
        let mut output = [0u8; 32];
        for (chunk, word) in output.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        output
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut words = [0u32; 64];
        for (index, word) in words[..16].iter_mut().enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes([
                block[start],
                block[start + 1],
                block[start + 2],
                block[start + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for (index, constant) in Self::K.iter().enumerate() {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(*constant)
                .wrapping_add(words[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    use typaxis_core::sha256;
    use typaxis_core::{
        ConfigResourceRoot, EffectiveDataVersions, HostPath, PdfStreamCompression, ResourceLimits,
        DEFAULT_ALLOWED_URI_SCHEMES, REGISTERED_JAPANESE_LINE_BREAK_VERSION,
        REGISTERED_UNICODE_VERSION,
    };

    struct TempTree {
        path: PathBuf,
    }

    impl TempTree {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "typaxis-host-admission-{}-{label}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn effective_config() -> EffectiveConfig {
        EffectiveConfig::new(
            false,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            DEFAULT_ALLOWED_URI_SCHEMES
                .iter()
                .map(|scheme| (*scheme).to_owned())
                .collect(),
            EffectiveDataVersions::new(
                REGISTERED_UNICODE_VERSION,
                REGISTERED_JAPANESE_LINE_BREAK_VERSION,
            )
            .unwrap(),
            ResourceLimits::default(),
        )
        .unwrap()
    }

    fn context(project_root: &Path, extra_roots: &[&Path]) -> HostAdmissionContext {
        HostAdmissionContext::new(
            HostPath::new(project_root.join("input.typ")).unwrap(),
            HostPath::new(project_root.to_path_buf()).unwrap(),
            None,
            extra_roots
                .iter()
                .map(|path| HostPath::new((*path).to_path_buf()).unwrap())
                .collect(),
        )
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn contained_open_and_stable_read_use_the_observed_exact_length() {
        let tree = TempTree::new("exact");
        let bytes = vec![0x5a; 20_000];
        fs::write(tree.path().join("payload.bin"), &bytes).unwrap();
        let session =
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
        let roots = session.roots();
        let opened = roots
            .open(&PortablePath::new("payload.bin").unwrap())
            .unwrap();
        assert_eq!(opened.observed_exact_length(), bytes.len() as u64);
        let expected_read = opened.read_identity().clone();
        let permit = roots.issue_bounded_read_permit(opened).unwrap();
        let receipt = roots.read_bounded(permit).unwrap();
        let stable = roots.accept_receipt(&expected_read, receipt).unwrap();
        assert_eq!(stable.bytes(), bytes);
        assert_eq!(stable.sha256(), sha256(&bytes));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn another_session_cannot_use_a_permit_or_receipt() {
        let first = TempTree::new("session-a");
        let second = TempTree::new("session-b");
        fs::write(first.path().join("payload.bin"), b"first").unwrap();
        fs::write(second.path().join("payload.bin"), b"second").unwrap();
        let config = effective_config();
        let first_session =
            HostAdmissionSession::new(&context(first.path(), &[]), &config).unwrap();
        let second_session =
            HostAdmissionSession::new(&context(second.path(), &[]), &config).unwrap();
        let first_roots = first_session.roots();
        let second_roots = second_session.roots();
        let uri = PortablePath::new("payload.bin").unwrap();

        let opened = first_roots.open(&uri).unwrap();
        let permit = first_roots.issue_bounded_read_permit(opened).unwrap();
        assert_eq!(
            second_roots.read_bounded(permit).unwrap_err(),
            HostAdmissionError::SessionMismatch
        );

        let opened = first_roots.open(&uri).unwrap();
        let expected_read = opened.read_identity().clone();
        let receipt = first_roots
            .read_bounded(first_roots.issue_bounded_read_permit(opened).unwrap())
            .unwrap();
        assert_eq!(
            second_roots
                .accept_receipt(&expected_read, receipt)
                .unwrap_err(),
            HostAdmissionError::SessionMismatch
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn same_session_receipt_cannot_swap_reopened_handle_identity() {
        let tree = TempTree::new("read-swap");
        fs::write(tree.path().join("first.bin"), b"same").unwrap();
        let session =
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
        let roots = session.roots();
        let uri = PortablePath::new("first.bin").unwrap();
        let first = roots.open(&uri).unwrap();
        let expected_read = first.read_identity().clone();
        let second = roots.open(&uri).unwrap();
        let receipt = roots
            .read_bounded(roots.issue_bounded_read_permit(second).unwrap())
            .unwrap();
        assert_eq!(
            roots.accept_receipt(&expected_read, receipt).unwrap_err(),
            HostAdmissionError::ReadIdentityMismatch
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn roots_are_dealiased_and_multiple_candidates_are_rejected() {
        let first = TempTree::new("first-root");
        let second = TempTree::new("second-root");
        fs::write(first.path().join("payload.bin"), b"one").unwrap();
        fs::write(second.path().join("payload.bin"), b"two").unwrap();
        let config = effective_config();

        assert_eq!(
            HostAdmissionSession::new(&context(first.path(), &[first.path()]), &config)
                .unwrap_err(),
            HostAdmissionError::AliasedRoot
        );

        let session =
            HostAdmissionSession::new(&context(first.path(), &[second.path()]), &config).unwrap();
        assert_eq!(
            session
                .roots()
                .open(&PortablePath::new("payload.bin").unwrap())
                .unwrap_err(),
            HostAdmissionError::AmbiguousCandidate
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn contained_open_preserves_nonblocking_shared_lock_behavior() {
        let tree = TempTree::new("locked-writer");
        fs::write(tree.path().join("payload.bin"), b"bytes").unwrap();
        let writer = File::options()
            .read(true)
            .write(true)
            .open(tree.path().join("payload.bin"))
            .unwrap();
        rustix::fs::flock(
            &writer,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .unwrap();
        let session =
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();

        assert_eq!(
            session
                .roots()
                .open(&PortablePath::new("payload.bin").unwrap())
                .unwrap_err(),
            HostAdmissionError::LockUnavailable
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn fixed_root_limit_accepts_exact_max_and_rejects_max_plus_one_before_open() {
        let tree = TempTree::new("root-limit");
        let mut extra_paths = Vec::new();
        for index in 0..(MAX_RESOURCE_ROOTS - 1) {
            let path = tree.path().join(format!("root-{index}"));
            fs::create_dir(&path).unwrap();
            extra_paths.push(path);
        }
        let extra_refs: Vec<&Path> = extra_paths.iter().map(PathBuf::as_path).collect();
        HostAdmissionSession::new(&context(tree.path(), &extra_refs), &effective_config()).unwrap();

        let unavailable: Vec<HostPath> = (0..MAX_RESOURCE_ROOTS)
            .map(|index| HostPath::new(tree.path().join(format!("unavailable-{index}"))).unwrap())
            .collect();
        let over_limit = HostAdmissionContext::new(
            HostPath::new(tree.path().join("input.typ")).unwrap(),
            HostPath::new(tree.path().to_path_buf()).unwrap(),
            None,
            unavailable,
        );
        assert_eq!(
            HostAdmissionSession::new(&over_limit, &effective_config()).unwrap_err(),
            HostAdmissionError::HostLimit
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn candidate_limit_counts_duplicate_work_but_deduplicates_identity_storage() {
        let tree = TempTree::new("candidate-limit");
        let session =
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
        let roots = session.roots();
        let duplicate = PortablePath::new("missing.bin").unwrap();
        let mut paths = vec![duplicate.clone(); MAX_HOST_READ_CANDIDATES];
        let registered = roots.register_candidates(&paths).unwrap();
        assert_eq!(registered.len(), MAX_HOST_READ_CANDIDATES);
        let token = roots.read_ledger_token().unwrap();
        assert_eq!(token.candidate_attempt_count(), MAX_HOST_READ_CANDIDATES);
        assert_eq!(token.stored_candidate_identity_count(), 1);
        assert_eq!(token.stored_opened_identity_count(), 0);
        assert_eq!(
            roots
                .register_candidates(std::slice::from_ref(&duplicate))
                .unwrap_err(),
            HostAdmissionError::HostLimit
        );

        paths.push(PortablePath::new("unsafe/leaf.bin").unwrap());
        let fresh =
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
        assert_eq!(
            fresh.roots().register_candidates(&paths).unwrap_err(),
            HostAdmissionError::HostLimit
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn contained_open_rejects_escape_symlinks_and_non_regular_components() {
        use std::os::unix::fs::symlink;

        let tree = TempTree::new("unsafe-components");
        let outside = TempTree::new("outside-root");
        fs::write(outside.path().join("outside.bin"), b"outside").unwrap();
        fs::create_dir(tree.path().join("directory")).unwrap();
        fs::write(tree.path().join("regular"), b"file").unwrap();
        symlink(outside.path(), tree.path().join("linked-directory")).unwrap();
        symlink(
            outside.path().join("outside.bin"),
            tree.path().join("linked-file"),
        )
        .unwrap();
        let session =
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();

        assert!(PortablePath::new("../outside.bin").is_err());
        let linked_candidate = PortablePath::new("linked-directory/outside.bin").unwrap();
        session
            .roots()
            .register_candidates(std::iter::once(&linked_candidate))
            .unwrap();
        assert!(session
            .roots()
            .read_ledger_token()
            .unwrap()
            .conflicts_with_write_target(
                &HostPath::new(outside.path().join("outside.bin")).unwrap()
            )
            .unwrap());
        for path in [
            "linked-directory/outside.bin",
            "linked-file",
            "regular/leaf",
        ] {
            assert_eq!(
                session
                    .roots()
                    .open(&PortablePath::new(path).unwrap())
                    .unwrap_err(),
                HostAdmissionError::UnsafeCandidate
            );
        }
        assert_eq!(
            session
                .roots()
                .open(&PortablePath::new("directory").unwrap())
                .unwrap_err(),
            HostAdmissionError::NotRegularFile
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn stable_read_rejects_truncate_growth_and_path_replacement() {
        let tree = TempTree::new("read-mutation");
        let path = tree.path().join("payload.bin");
        let original = vec![0x41; 20_000];
        let uri = PortablePath::new("payload.bin").unwrap();

        for mutation in 0..3 {
            fs::write(&path, &original).unwrap();
            let session =
                HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
            let roots = session.roots();
            let opened = roots.open(&uri).unwrap();
            let permit = roots.issue_bounded_read_permit(opened).unwrap();
            let replacement = tree.path().join("replacement.bin");
            let result = read_bounded_with_observer(permit, || match mutation {
                0 => File::options()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_len(1)
                    .unwrap(),
                1 => File::options()
                    .append(true)
                    .open(&path)
                    .unwrap()
                    .write_all(b"growth")
                    .unwrap(),
                2 => {
                    fs::write(&replacement, &original).unwrap();
                    fs::rename(&replacement, &path).unwrap();
                }
                _ => unreachable!(),
            });
            assert_eq!(result.unwrap_err(), HostAdmissionError::LengthMismatch);
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn sealed_ledger_revalidates_candidates_and_detects_write_aliases() {
        let tree = TempTree::new("ledger-token");
        fs::write(tree.path().join("input.bin"), b"input").unwrap();
        fs::hard_link(
            tree.path().join("input.bin"),
            tree.path().join("hard-link.bin"),
        )
        .unwrap();
        let session =
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap();
        let roots = session.roots();
        let opened = roots
            .open(&PortablePath::new("input.bin").unwrap())
            .unwrap();
        let token = roots.read_ledger_token().unwrap();

        token.revalidate().unwrap();
        assert!(token
            .conflicts_with_write_target(&HostPath::new(tree.path().join("input.bin")).unwrap())
            .unwrap());
        assert!(token
            .conflicts_with_write_target(&HostPath::new(tree.path().join("hard-link.bin")).unwrap())
            .unwrap());
        assert!(!token
            .conflicts_with_write_target(&HostPath::new(tree.path().join("unrelated.bin")).unwrap())
            .unwrap());

        drop(opened);
        fs::write(tree.path().join("replacement.bin"), b"input").unwrap();
        fs::rename(
            tree.path().join("replacement.bin"),
            tree.path().join("input.bin"),
        )
        .unwrap();
        assert_eq!(
            token.revalidate().unwrap_err(),
            HostAdmissionError::ReadIdentityMismatch
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn shared_ledger_spans_root_sessions_and_owns_revalidation_handles() {
        let package = TempTree::new("shared-package-root");
        let resource = TempTree::new("shared-resource-root");
        let package_session = HostAdmissionSession::new_contained_root(
            &HostPath::new(package.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        package_session
            .roots()
            .register_candidates(std::iter::once(&PortablePath::new("package.json").unwrap()))
            .unwrap();
        let stale = package_session.roots().read_ledger_token().unwrap();

        let resource_session = HostAdmissionSession::new_contained_root_with_read_ledger(
            &HostPath::new(resource.path().to_path_buf()).unwrap(),
            package_session.read_ledger(),
        )
        .unwrap();
        resource_session
            .roots()
            .register_candidates(std::iter::once(&PortablePath::new("font.ttf").unwrap()))
            .unwrap();
        assert_eq!(
            stale.revalidate().unwrap_err(),
            HostAdmissionError::ReadIdentityMismatch
        );
        let final_token = resource_session.roots().read_ledger_token().unwrap();
        assert_eq!(final_token.candidate_attempt_count(), 2);
        assert_eq!(final_token.stored_candidate_identity_count(), 2);

        drop(package_session);
        drop(resource_session);
        final_token.revalidate().unwrap();
        assert!(final_token
            .conflicts_with_write_target(
                &HostPath::new(package.path().join("package.json")).unwrap()
            )
            .unwrap());
    }

    #[test]
    fn compiled_capability_matches_the_selected_target() {
        let capability = HostCapabilityToken::compiled();
        assert_eq!(
            capability.contained_open_available(),
            cfg!(any(
                target_os = "android",
                target_os = "linux",
                target_os = "macos"
            ))
        );
        assert_eq!(
            capability.contained_package_open_available(),
            capability.contained_resource_open_available()
        );
        assert_eq!(
            HostResourceCapabilityToken::compiled().contained_resource_open(),
            capability.contained_resource_open_available()
        );
        assert_eq!(
            AtomicFilePublicationCapabilityToken::compiled().available(),
            cfg!(unix)
        );
    }

    #[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
    #[test]
    fn unsupported_platform_fails_before_issuing_a_root_capability() {
        let tree = TempTree::new("unsupported");
        assert!(!HostCapabilityToken::compiled().contained_open_available());
        assert_eq!(
            HostAdmissionSession::new(&context(tree.path(), &[]), &effective_config()).unwrap_err(),
            HostAdmissionError::UnsupportedContainedOpen
        );
        assert_eq!(
            HostAdmissionSession::new_contained_root(
                &HostPath::new(tree.path().to_path_buf()).unwrap()
            )
            .unwrap_err(),
            HostAdmissionError::UnsupportedContainedOpen
        );
    }
}
