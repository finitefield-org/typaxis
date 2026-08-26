#![forbid(unsafe_code)]

//! Session-bound admission of a DocumentPackage and its companion source.
//!
//! Host paths and file handles remain owned by `typaxis-host-admission`; JSON
//! decoding remains owned by `typaxis-document-package`. This crate binds both
//! receipts to one non-cloneable machine-input state transition.

mod path;

use std::cell::RefCell;
use std::fmt;
use std::sync::Arc;
use typaxis_core::{
    machine_input_fingerprint_from_jcs, push_jcs_string, sha256, DocumentPackageContractId,
    HostPath, PortablePath, PortablePathError, SourceId, ValidatedResourceLimits,
};
use typaxis_document_package::{
    DecodedDocumentPackage, DocumentPackageDecodeError, DocumentPackageDecodePolicy,
    DocumentPackagePreflightLimits, StrictDocumentPackageDecoder, WireSource,
};
use typaxis_host_admission::{
    HostAdmissionError, HostAdmissionSession, HostCapabilityToken, HostReadIdentityLedger,
    HostReadIdentityLedgerToken, StableFileBytes,
};

#[doc(hidden)]
pub use typaxis_host_admission::{
    AtomicFilePublicationCapabilityToken, HostResourceCapabilityToken, MAX_HOST_READ_CANDIDATES,
    MAX_RESOURCE_ROOTS,
};

use path::resolve_package_location;

pub use typaxis_core::MachineInputFingerprint;

/// Machine-input owner's compile-time contained-PACKAGE capability.
///
/// The token is sealed against caller-authored booleans and is also used by
/// the actual admission entrypoint below.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachineInputCapabilityToken {
    contained_package_open: bool,
}

impl MachineInputCapabilityToken {
    pub const fn compiled() -> Self {
        Self {
            contained_package_open: HostCapabilityToken::compiled()
                .contained_package_open_available(),
        }
    }

    pub const fn contained_package_open(self) -> bool {
        self.contained_package_open
    }
}

#[derive(Debug)]
struct SessionMarker;

/// Opaque identity shared by all receipts issued by one machine-input session.
#[derive(Clone)]
pub struct MachineInputSessionIdentity(Arc<SessionMarker>);

impl MachineInputSessionIdentity {
    fn fresh() -> Self {
        Self(Arc::new(SessionMarker))
    }
}

impl PartialEq for MachineInputSessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for MachineInputSessionIdentity {}

impl fmt::Debug for MachineInputSessionIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MachineInputSessionIdentity(..)")
    }
}

/// Monotonic machine-input validation stage.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MachineInputStage {
    NoInput,
    RawPackageAdmitted,
    PackageDecoded,
    SourcesAdmitted,
}

/// Portable facts established by the stable PACKAGE read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedPackageFacts {
    uri: PortablePath,
    bytes: u64,
    sha256: [u8; 32],
}

impl AdmittedPackageFacts {
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }

    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    pub const fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
}

/// Portable facts established by strict typed package decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedPackageFacts {
    contract: DocumentPackageContractId,
    canonical_sha256: [u8; 32],
}

impl DecodedPackageFacts {
    pub const fn contract(self) -> DocumentPackageContractId {
        self.contract
    }

    pub const fn canonical_sha256(self) -> [u8; 32] {
        self.canonical_sha256
    }
}

/// Portable facts established from actual, stably read source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedMachineSourceFacts {
    source_id: SourceId,
    uri: PortablePath,
    bytes: u64,
    sha256: [u8; 32],
}

impl AdmittedMachineSourceFacts {
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }

    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }

    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    pub const fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
}

#[derive(Clone, Debug)]
struct ProgressState {
    stage: MachineInputStage,
    package: Option<AdmittedPackageFacts>,
    decoded: Option<DecodedPackageFacts>,
    sources: Vec<AdmittedMachineSourceFacts>,
    fingerprint: Option<MachineInputFingerprint>,
}

impl ProgressState {
    fn no_input() -> Self {
        Self {
            stage: MachineInputStage::NoInput,
            package: None,
            decoded: None,
            sources: Vec::new(),
            fingerprint: None,
        }
    }

    fn raw(package: AdmittedPackageFacts) -> Self {
        Self {
            stage: MachineInputStage::RawPackageAdmitted,
            package: Some(package),
            decoded: None,
            sources: Vec::new(),
            fingerprint: None,
        }
    }
}

/// Sealed snapshot of the last successfully validated machine-input stage.
///
/// It has no public constructor and is deliberately not `Clone`; failure
/// publishers receive facts only from a session-issued snapshot.
pub struct MachineInputProgress {
    session: Option<MachineInputSessionIdentity>,
    state: ProgressState,
}

impl fmt::Debug for MachineInputProgress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MachineInputProgress")
            .field("stage", &self.state.stage)
            .field("package", &self.state.package)
            .field("decoded", &self.state.decoded)
            .field("sources", &self.state.sources)
            .field("fingerprint", &self.state.fingerprint)
            .finish_non_exhaustive()
    }
}

impl MachineInputProgress {
    fn no_input() -> Self {
        Self {
            session: None,
            state: ProgressState::no_input(),
        }
    }

    fn issued(session: &MachineInputSessionIdentity, state: ProgressState) -> Self {
        Self {
            session: Some(session.clone()),
            state,
        }
    }

    pub const fn stage(&self) -> MachineInputStage {
        self.state.stage
    }

    pub const fn session_identity(&self) -> Option<&MachineInputSessionIdentity> {
        self.session.as_ref()
    }

    pub const fn package(&self) -> Option<&AdmittedPackageFacts> {
        self.state.package.as_ref()
    }

    pub const fn decoded(&self) -> Option<DecodedPackageFacts> {
        self.state.decoded
    }

    pub fn sources(&self) -> &[AdmittedMachineSourceFacts] {
        &self.state.sources
    }

    pub const fn fingerprint(&self) -> Option<MachineInputFingerprint> {
        self.state.fingerprint
    }
}

/// Receipt category used by binding failures without exposing receipt parts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineInputReceiptKind {
    RawPackage,
    DecodedPackage,
    SourceSet,
}

/// Typed reason for a machine-input admission failure.
#[derive(Debug)]
pub enum MachineInputErrorKind {
    UnsupportedContainedOpen,
    CurrentDirectoryUnavailable,
    InvalidPackagePath,
    NonPortablePackageUri,
    InvalidPackageUri(PortablePathError),
    PackageOutsideRoot,
    PackageOpen(HostAdmissionError),
    PackageTooLarge {
        maximum: u64,
        observed: u64,
    },
    DecodePolicyMismatch,
    Decode(DocumentPackageDecodeError),
    PackageHashMismatch,
    InvalidProgress {
        expected: MachineInputStage,
        actual: MachineInputStage,
    },
    ReceiptSessionMismatch(MachineInputReceiptKind),
    ReceiptPackageMismatch(MachineInputReceiptKind),
    ReceiptDeclarationMismatch,
    SourceCount {
        observed: usize,
    },
    NonzeroSourceId {
        observed: u32,
    },
    UnsafeSourceUri {
        source_id: u32,
        cause: PortablePathError,
    },
    SourceUriTooLong {
        source_id: u32,
        maximum: u32,
        observed: u64,
    },
    SourceDeclaredLimit {
        source_id: u32,
        maximum: u64,
        declared: u64,
    },
    SourceOpen {
        source_id: u32,
        cause: HostAdmissionError,
    },
    SourceLimit {
        source_id: u32,
        maximum: u64,
        observed: u64,
    },
    AggregateInputLimit {
        maximum: u64,
        attempted: u64,
    },
    SourceLengthMismatch {
        source_id: u32,
        declared: u64,
        actual: u64,
    },
    SourceHashMismatch {
        source_id: u32,
        declared: [u8; 32],
        actual: [u8; 32],
    },
    SourceNotUtf8 {
        source_id: u32,
        valid_up_to: u64,
    },
}

/// Admission error paired with the session's last sealed progress token.
#[derive(Debug)]
pub struct MachineInputError {
    kind: Box<MachineInputErrorKind>,
    progress: Box<MachineInputProgress>,
    read_ledger: HostReadIdentityLedger,
}

impl MachineInputError {
    fn no_input_with_read_ledger(
        kind: MachineInputErrorKind,
        read_ledger: &HostReadIdentityLedger,
    ) -> Self {
        Self {
            kind: Box::new(kind),
            progress: Box::new(MachineInputProgress::no_input()),
            read_ledger: read_ledger.clone(),
        }
    }

    pub const fn kind(&self) -> &MachineInputErrorKind {
        &self.kind
    }

    pub const fn progress(&self) -> &MachineInputProgress {
        &self.progress
    }

    pub fn into_parts(self) -> (MachineInputErrorKind, MachineInputProgress) {
        (*self.kind, *self.progress)
    }

    /// Seal the last PACKAGE/source candidate and opened identities retained
    /// by this failure. Even a missing logical candidate remains represented,
    /// allowing terminal sidecars to reject an aliased write target.
    pub fn read_ledger_token(&self) -> Result<HostReadIdentityLedgerToken, HostAdmissionError> {
        self.read_ledger.token()
    }

    pub fn into_parts_with_read_ledger(
        self,
    ) -> Result<
        (
            MachineInputErrorKind,
            MachineInputProgress,
            HostReadIdentityLedgerToken,
        ),
        HostAdmissionError,
    > {
        let token = self.read_ledger.token()?;
        Ok((*self.kind, *self.progress, token))
    }
}

impl fmt::Display for MachineInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind.as_ref() {
            MachineInputErrorKind::UnsupportedContainedOpen => {
                formatter.write_str("contained package open is unsupported on this target")
            }
            MachineInputErrorKind::CurrentDirectoryUnavailable => {
                formatter.write_str("the current directory is unavailable")
            }
            MachineInputErrorKind::InvalidPackagePath
            | MachineInputErrorKind::NonPortablePackageUri
            | MachineInputErrorKind::InvalidPackageUri(_)
            | MachineInputErrorKind::PackageOutsideRoot => {
                formatter.write_str("PACKAGE is not a safe portable path beneath package-root")
            }
            MachineInputErrorKind::PackageOpen(cause) => {
                write!(formatter, "PACKAGE host admission failed: {cause:?}")
            }
            MachineInputErrorKind::PackageTooLarge { maximum, observed } => write!(
                formatter,
                "PACKAGE length {observed} exceeds the admitted limit {maximum}"
            ),
            MachineInputErrorKind::DecodePolicyMismatch => {
                formatter.write_str("decode policy does not match the admission session")
            }
            MachineInputErrorKind::Decode(error) => error.fmt(formatter),
            MachineInputErrorKind::PackageHashMismatch => {
                formatter.write_str("decoded PACKAGE hash differs from the admitted bytes")
            }
            MachineInputErrorKind::InvalidProgress { expected, actual } => write!(
                formatter,
                "machine-input stage mismatch: expected {expected:?}, found {actual:?}"
            ),
            MachineInputErrorKind::ReceiptSessionMismatch(receipt) => {
                write!(formatter, "{receipt:?} belongs to a different session")
            }
            MachineInputErrorKind::ReceiptPackageMismatch(receipt) => {
                write!(formatter, "{receipt:?} is bound to a different PACKAGE")
            }
            MachineInputErrorKind::ReceiptDeclarationMismatch => {
                formatter.write_str("source receipt declaration binding differs from PACKAGE")
            }
            MachineInputErrorKind::SourceCount { observed } => write!(
                formatter,
                "the machine source profile requires exactly one source, found {observed}"
            ),
            MachineInputErrorKind::NonzeroSourceId { observed } => write!(
                formatter,
                "the machine source profile requires source_id 0, found {observed}"
            ),
            MachineInputErrorKind::UnsafeSourceUri { source_id, .. } => {
                write!(formatter, "source {source_id} has an unsafe relative URI")
            }
            MachineInputErrorKind::SourceUriTooLong {
                source_id,
                maximum,
                observed,
            } => write!(
                formatter,
                "source {source_id} URI length {observed} exceeds {maximum}"
            ),
            MachineInputErrorKind::SourceDeclaredLimit {
                source_id,
                maximum,
                declared,
            } => write!(
                formatter,
                "source {source_id} declared length {declared} exceeds {maximum}"
            ),
            MachineInputErrorKind::SourceOpen { source_id, cause } => write!(
                formatter,
                "source {source_id} host admission failed: {cause:?}"
            ),
            MachineInputErrorKind::SourceLimit {
                source_id,
                maximum,
                observed,
            } => write!(
                formatter,
                "source {source_id} length {observed} exceeds {maximum}"
            ),
            MachineInputErrorKind::AggregateInputLimit { maximum, attempted } => write!(
                formatter,
                "source input total {attempted} exceeds {maximum}"
            ),
            MachineInputErrorKind::SourceLengthMismatch {
                source_id,
                declared,
                actual,
            } => write!(
                formatter,
                "source {source_id} declared length {declared} differs from actual {actual}"
            ),
            MachineInputErrorKind::SourceHashMismatch { source_id, .. } => {
                write!(
                    formatter,
                    "source {source_id} declared hash differs from actual bytes"
                )
            }
            MachineInputErrorKind::SourceNotUtf8 {
                source_id,
                valid_up_to,
            } => write!(
                formatter,
                "source {source_id} is not UTF-8 at byte {valid_up_to}"
            ),
        }
    }
}

impl std::error::Error for MachineInputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.kind.as_ref() {
            MachineInputErrorKind::Decode(error) => Some(error),
            _ => None,
        }
    }
}

/// Host-only PACKAGE location options. Absolute paths never enter portable
/// progress or fingerprint facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineInputHostOptions {
    package: HostPath,
    package_root: Option<HostPath>,
}

impl MachineInputHostOptions {
    pub const fn new(package: HostPath, package_root: Option<HostPath>) -> Self {
        Self {
            package,
            package_root,
        }
    }

    pub const fn package(&self) -> &HostPath {
        &self.package
    }

    pub const fn package_root(&self) -> Option<&HostPath> {
        self.package_root.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PackageBinding(AdmittedPackageFacts);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SourceDeclarationFingerprint([u8; 32]);

/// Raw PACKAGE bytes admitted by one host machine-input session.
///
/// ```compile_fail
/// use typaxis_machine_input::AdmittedPackageBytes;
/// let _forged = AdmittedPackageBytes {};
/// ```
pub struct AdmittedPackageBytes {
    session: MachineInputSessionIdentity,
    package: PackageBinding,
    stable: StableFileBytes,
}

impl fmt::Debug for AdmittedPackageBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmittedPackageBytes")
            .field("session", &self.session)
            .field("package", &self.package)
            .finish_non_exhaustive()
    }
}

impl AdmittedPackageBytes {
    pub const fn session_identity(&self) -> &MachineInputSessionIdentity {
        &self.session
    }

    pub const fn facts(&self) -> &AdmittedPackageFacts {
        &self.package.0
    }

    pub fn bytes(&self) -> &[u8] {
        self.stable.bytes()
    }
}

/// Strict decoder receipt bound to the raw PACKAGE receipt's session.
///
/// There is intentionally no API that binds a caller-supplied decoded value.
///
/// ```compile_fail
/// use typaxis_machine_input::SessionBoundDecodedPackage;
/// let _forged = SessionBoundDecodedPackage {};
/// ```
pub struct SessionBoundDecodedPackage {
    session: MachineInputSessionIdentity,
    package: PackageBinding,
    declaration: SourceDeclarationFingerprint,
    decoded: DecodedDocumentPackage,
}

impl fmt::Debug for SessionBoundDecodedPackage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionBoundDecodedPackage")
            .field("session", &self.session)
            .field("package", &self.package)
            .field("decoded", &self.decoded)
            .finish_non_exhaustive()
    }
}

impl SessionBoundDecodedPackage {
    pub const fn session_identity(&self) -> &MachineInputSessionIdentity {
        &self.session
    }

    pub const fn decoded(&self) -> &DecodedDocumentPackage {
        &self.decoded
    }
}

/// One actual companion source and its owned UTF-8 bytes.
pub struct AdmittedMachineSource {
    facts: AdmittedMachineSourceFacts,
    text: String,
}

impl fmt::Debug for AdmittedMachineSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmittedMachineSource")
            .field("facts", &self.facts)
            .finish_non_exhaustive()
    }
}

impl AdmittedMachineSource {
    pub const fn facts(&self) -> &AdmittedMachineSourceFacts {
        &self.facts
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn into_text(self) -> String {
        self.text
    }

    /// Consume the admission receipt while preserving ownership of the actual
    /// source buffer for the syntax boundary.
    pub fn into_parts(self) -> (AdmittedMachineSourceFacts, String) {
        (self.facts, self.text)
    }
}

/// Exact source set admitted from the bound decoded declaration.
///
/// ```compile_fail
/// use typaxis_machine_input::AdmittedMachineSourceSet;
/// let _forged = AdmittedMachineSourceSet {};
/// ```
pub struct AdmittedMachineSourceSet {
    session: MachineInputSessionIdentity,
    package: PackageBinding,
    declaration: SourceDeclarationFingerprint,
    sources: Vec<AdmittedMachineSource>,
    fingerprint: MachineInputFingerprint,
}

impl fmt::Debug for AdmittedMachineSourceSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmittedMachineSourceSet")
            .field("session", &self.session)
            .field("package", &self.package)
            .field("sources", &self.sources)
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}

impl AdmittedMachineSourceSet {
    pub const fn session_identity(&self) -> &MachineInputSessionIdentity {
        &self.session
    }

    pub fn sources(&self) -> &[AdmittedMachineSource] {
        &self.sources
    }

    pub const fn fingerprint(&self) -> MachineInputFingerprint {
        self.fingerprint
    }
}

/// Complete, still-untrusted-at-the-AST-layer machine input.
///
/// Only [`HostMachineInputSession::finish`] can issue this type.
///
/// ```compile_fail
/// use typaxis_machine_input::AdmittedMachinePackage;
/// let _forged = AdmittedMachinePackage {};
/// ```
pub struct AdmittedMachinePackage {
    session: MachineInputSessionIdentity,
    decoded: DecodedDocumentPackage,
    sources: Vec<AdmittedMachineSource>,
    progress: MachineInputProgress,
    fingerprint: MachineInputFingerprint,
    read_ledger: HostReadIdentityLedger,
}

/// Move-only admission provenance retained across syntax validation.
///
/// Its private fields keep the session/progress/fingerprint/ledger binding
/// intact while allowing the syntax crate to own it without depending on the
/// host-admission implementation crate.
///
/// ```compile_fail
/// use typaxis_machine_input::MachineInputAdmissionProvenance;
/// let _forged = MachineInputAdmissionProvenance {};
/// ```
pub struct MachineInputAdmissionProvenance {
    session: MachineInputSessionIdentity,
    progress: MachineInputProgress,
    fingerprint: MachineInputFingerprint,
    read_ledger: HostReadIdentityLedger,
}

impl fmt::Debug for MachineInputAdmissionProvenance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MachineInputAdmissionProvenance")
            .field("session", &self.session)
            .field("progress", &self.progress)
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}

impl MachineInputAdmissionProvenance {
    pub const fn session_identity(&self) -> &MachineInputSessionIdentity {
        &self.session
    }

    pub const fn progress(&self) -> &MachineInputProgress {
        &self.progress
    }

    pub const fn fingerprint(&self) -> MachineInputFingerprint {
        self.fingerprint
    }

    /// Command-wide ledger handle for later resource candidate registration.
    pub const fn read_ledger(&self) -> &HostReadIdentityLedger {
        &self.read_ledger
    }

    pub fn read_ledger_token(&self) -> Result<HostReadIdentityLedgerToken, HostAdmissionError> {
        self.read_ledger.token()
    }

    /// Relinquish all success-only provenance while preserving the last
    /// session-issued progress token for a typed failure outcome.
    pub fn into_failure_progress(self) -> MachineInputProgress {
        self.progress
    }
}

impl fmt::Debug for AdmittedMachinePackage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmittedMachinePackage")
            .field("session", &self.session)
            .field("decoded", &self.decoded)
            .field("sources", &self.sources)
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}

impl AdmittedMachinePackage {
    pub const fn session_identity(&self) -> &MachineInputSessionIdentity {
        &self.session
    }

    pub const fn decoded(&self) -> &DecodedDocumentPackage {
        &self.decoded
    }

    pub fn sources(&self) -> &[AdmittedMachineSource] {
        &self.sources
    }

    pub const fn progress(&self) -> &MachineInputProgress {
        &self.progress
    }

    pub const fn fingerprint(&self) -> MachineInputFingerprint {
        self.fingerprint
    }

    /// Command-wide ledger handle for later resource candidate registration.
    /// This capability cannot open the package root as a resource root.
    pub const fn read_ledger(&self) -> &HostReadIdentityLedger {
        &self.read_ledger
    }

    pub fn read_ledger_token(&self) -> Result<HostReadIdentityLedgerToken, HostAdmissionError> {
        self.read_ledger.token()
    }

    /// One-way, move-only handoff to the syntax owner.
    ///
    /// The components cannot be recombined into an admission receipt through
    /// any public API. This method exists so large source and package text
    /// buffers cross the parsing boundary without cloning.
    pub fn into_parts(
        self,
    ) -> (
        DecodedDocumentPackage,
        Vec<AdmittedMachineSource>,
        MachineInputAdmissionProvenance,
    ) {
        (
            self.decoded,
            self.sources,
            MachineInputAdmissionProvenance {
                session: self.session,
                progress: self.progress,
                fingerprint: self.fingerprint,
                read_ledger: self.read_ledger,
            },
        )
    }
}

/// Owner of PACKAGE root policy and the raw -> decoded -> source state machine.
pub struct HostMachineInputSession {
    identity: MachineInputSessionIdentity,
    host: HostAdmissionSession,
    package_limits: DocumentPackagePreflightLimits,
    resource_limits: ValidatedResourceLimits,
    progress: RefCell<ProgressState>,
}

impl fmt::Debug for HostMachineInputSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HostMachineInputSession")
            .field("identity", &self.identity)
            .field("host", &self.host)
            .field("stage", &self.progress.borrow().stage)
            .finish_non_exhaustive()
    }
}

impl HostMachineInputSession {
    /// Admit PACKAGE with the effective configuration's package byte/depth bounds.
    pub fn open(
        options: MachineInputHostOptions,
        limits: &ValidatedResourceLimits,
    ) -> Result<(Self, AdmittedPackageBytes), MachineInputError> {
        let read_ledger = HostReadIdentityLedger::new();
        Self::open_with_preflight_limits_and_read_ledger(
            options,
            limits,
            DocumentPackagePreflightLimits::from_resource_limits(limits),
            &read_ledger,
        )
    }

    /// Admit PACKAGE into a command-wide ledger that may already contain the
    /// config target. This is the machine command's pre-open integration point.
    pub fn open_with_read_ledger(
        options: MachineInputHostOptions,
        limits: &ValidatedResourceLimits,
        read_ledger: &HostReadIdentityLedger,
    ) -> Result<(Self, AdmittedPackageBytes), MachineInputError> {
        Self::open_with_preflight_limits_and_read_ledger(
            options,
            limits,
            DocumentPackagePreflightLimits::from_resource_limits(limits),
            read_ledger,
        )
    }

    /// Admit PACKAGE with already-validated runtime package preflight bounds.
    /// The same bounds must later be supplied through the decoder policy.
    pub fn open_with_preflight_limits(
        options: MachineInputHostOptions,
        limits: &ValidatedResourceLimits,
        package_limits: DocumentPackagePreflightLimits,
    ) -> Result<(Self, AdmittedPackageBytes), MachineInputError> {
        let read_ledger = HostReadIdentityLedger::new();
        Self::open_with_preflight_limits_and_read_ledger(
            options,
            limits,
            package_limits,
            &read_ledger,
        )
    }

    pub fn open_with_preflight_limits_and_read_ledger(
        options: MachineInputHostOptions,
        limits: &ValidatedResourceLimits,
        package_limits: DocumentPackagePreflightLimits,
        read_ledger: &HostReadIdentityLedger,
    ) -> Result<(Self, AdmittedPackageBytes), MachineInputError> {
        if !MachineInputCapabilityToken::compiled().contained_package_open() {
            return Err(MachineInputError::no_input_with_read_ledger(
                MachineInputErrorKind::UnsupportedContainedOpen,
                read_ledger,
            ));
        }
        let location = resolve_package_location(&options.package, options.package_root.as_ref())
            .map_err(|kind| MachineInputError::no_input_with_read_ledger(kind, read_ledger))?;
        let host =
            HostAdmissionSession::new_contained_root_with_read_ledger(&location.root, read_ledger)
                .map_err(map_package_host_error)
                .map_err(|kind| MachineInputError::no_input_with_read_ledger(kind, read_ledger))?;
        let roots = host.roots();
        let opened = roots
            .open(&location.uri)
            .map_err(map_package_host_error)
            .map_err(|kind| MachineInputError::no_input_with_read_ledger(kind, read_ledger))?;
        let observed = opened.observed_exact_length();
        let maximum = package_limits.max_bytes().get();
        if observed > maximum {
            return Err(MachineInputError::no_input_with_read_ledger(
                MachineInputErrorKind::PackageTooLarge { maximum, observed },
                read_ledger,
            ));
        }
        let expected_read = opened.read_identity().clone();
        let permit = roots
            .issue_bounded_read_permit(opened)
            .map_err(map_package_host_error)
            .map_err(|kind| MachineInputError::no_input_with_read_ledger(kind, read_ledger))?;
        let receipt = roots
            .read_bounded(permit)
            .map_err(map_package_host_error)
            .map_err(|kind| MachineInputError::no_input_with_read_ledger(kind, read_ledger))?;
        let stable = roots
            .accept_receipt(&expected_read, receipt)
            .map_err(map_package_host_error)
            .map_err(|kind| MachineInputError::no_input_with_read_ledger(kind, read_ledger))?;
        let package = PackageBinding(AdmittedPackageFacts {
            uri: location.uri,
            bytes: stable.observed_exact_length(),
            sha256: stable.sha256(),
        });
        let identity = MachineInputSessionIdentity::fresh();
        let raw = AdmittedPackageBytes {
            session: identity.clone(),
            package: package.clone(),
            stable,
        };
        let session = Self {
            identity,
            host,
            package_limits,
            resource_limits: limits.clone(),
            progress: RefCell::new(ProgressState::raw(package.0)),
        };
        Ok((session, raw))
    }

    pub const fn session_identity(&self) -> &MachineInputSessionIdentity {
        &self.identity
    }

    pub const fn read_ledger(&self) -> &HostReadIdentityLedger {
        self.host.read_ledger()
    }

    pub fn read_ledger_token(&self) -> Result<HostReadIdentityLedgerToken, MachineInputError> {
        self.host
            .roots()
            .read_ledger_token()
            .map_err(map_package_host_error)
            .map_err(|kind| self.failure(kind))
    }

    pub fn progress(&self) -> MachineInputProgress {
        self.progress_snapshot()
    }

    /// Decode only the exact bytes owned by this session's raw receipt.
    pub fn decode_and_bind(
        &self,
        raw: &AdmittedPackageBytes,
        decoder: &StrictDocumentPackageDecoder,
        policy: &DocumentPackageDecodePolicy<'_>,
    ) -> Result<SessionBoundDecodedPackage, MachineInputError> {
        self.require_stage(MachineInputStage::RawPackageAdmitted)?;
        self.validate_session(&raw.session, MachineInputReceiptKind::RawPackage)?;
        self.validate_package(&raw.package, MachineInputReceiptKind::RawPackage)?;
        if policy.preflight_limits() != self.package_limits
            || policy.resource_limits() != &self.resource_limits
        {
            return Err(self.failure(MachineInputErrorKind::DecodePolicyMismatch));
        }
        let decoded = decoder
            .decode(raw.bytes(), policy)
            .map_err(|error| self.failure(MachineInputErrorKind::Decode(error)))?;
        if decoded.raw_sha256().as_bytes() != &raw.package.0.sha256 {
            return Err(self.failure(MachineInputErrorKind::PackageHashMismatch));
        }
        let declaration = source_declaration_fingerprint(&decoded.wire().sources);
        let facts = DecodedPackageFacts {
            contract: decoded.wire().contract,
            canonical_sha256: decoded.canonical_jcs_sha256().into_bytes(),
        };
        {
            let mut progress = self.progress.borrow_mut();
            progress.stage = MachineInputStage::PackageDecoded;
            progress.decoded = Some(facts);
        }
        Ok(SessionBoundDecodedPackage {
            session: self.identity.clone(),
            package: raw.package.clone(),
            declaration,
            decoded,
        })
    }

    /// Preflight and stably read the exactly-one companion source from the
    /// package root. No resource root participates in this lookup.
    pub fn admit_sources(
        &self,
        decoded: &SessionBoundDecodedPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<AdmittedMachineSourceSet, MachineInputError> {
        self.require_stage(MachineInputStage::PackageDecoded)?;
        self.validate_session(&decoded.session, MachineInputReceiptKind::DecodedPackage)?;
        self.validate_package(&decoded.package, MachineInputReceiptKind::DecodedPackage)?;
        if limits != &self.resource_limits {
            return Err(self.failure(MachineInputErrorKind::DecodePolicyMismatch));
        }
        let declarations = &decoded.decoded.wire().sources;
        if declarations.len() != 1 {
            return Err(self.failure(MachineInputErrorKind::SourceCount {
                observed: declarations.len(),
            }));
        }
        let declaration = &declarations[0];
        if declaration.source_id != 0 {
            return Err(self.failure(MachineInputErrorKind::NonzeroSourceId {
                observed: declaration.source_id,
            }));
        }
        let uri_bytes = u64::try_from(declaration.uri.len()).unwrap_or(u64::MAX);
        let maximum_uri = limits.get().max_uri_bytes;
        if uri_bytes > u64::from(maximum_uri) {
            return Err(self.failure(MachineInputErrorKind::SourceUriTooLong {
                source_id: declaration.source_id,
                maximum: maximum_uri,
                observed: uri_bytes,
            }));
        }
        let uri = PortablePath::new(declaration.uri.clone()).map_err(|cause| {
            self.failure(MachineInputErrorKind::UnsafeSourceUri {
                source_id: declaration.source_id,
                cause,
            })
        })?;
        let declared = u64::from(declaration.utf8_byte_length);
        let maximum_source = u64::from(limits.get().max_source_bytes);
        if declared > maximum_source {
            return Err(self.failure(MachineInputErrorKind::SourceDeclaredLimit {
                source_id: declaration.source_id,
                maximum: maximum_source,
                declared,
            }));
        }
        if declared > limits.get().max_input_bytes {
            return Err(self.failure(MachineInputErrorKind::AggregateInputLimit {
                maximum: limits.get().max_input_bytes,
                attempted: declared,
            }));
        }

        let roots = self.host.roots();
        let opened = roots.open(&uri).map_err(|cause| {
            self.failure(MachineInputErrorKind::SourceOpen {
                source_id: declaration.source_id,
                cause,
            })
        })?;
        let observed = opened.observed_exact_length();
        if observed > maximum_source {
            return Err(self.failure(MachineInputErrorKind::SourceLimit {
                source_id: declaration.source_id,
                maximum: maximum_source,
                observed,
            }));
        }
        let aggregate = 0u64.checked_add(observed).ok_or_else(|| {
            self.failure(MachineInputErrorKind::AggregateInputLimit {
                maximum: limits.get().max_input_bytes,
                attempted: u64::MAX,
            })
        })?;
        if aggregate > limits.get().max_input_bytes {
            return Err(self.failure(MachineInputErrorKind::AggregateInputLimit {
                maximum: limits.get().max_input_bytes,
                attempted: aggregate,
            }));
        }
        if observed != declared {
            return Err(self.failure(MachineInputErrorKind::SourceLengthMismatch {
                source_id: declaration.source_id,
                declared,
                actual: observed,
            }));
        }
        let expected_read = opened.read_identity().clone();
        let permit = roots.issue_bounded_read_permit(opened).map_err(|cause| {
            self.failure(MachineInputErrorKind::SourceOpen {
                source_id: declaration.source_id,
                cause,
            })
        })?;
        let receipt = roots.read_bounded(permit).map_err(|cause| {
            self.failure(MachineInputErrorKind::SourceOpen {
                source_id: declaration.source_id,
                cause,
            })
        })?;
        let stable = roots
            .accept_receipt(&expected_read, receipt)
            .map_err(|cause| {
                self.failure(MachineInputErrorKind::SourceOpen {
                    source_id: declaration.source_id,
                    cause,
                })
            })?;
        let (bytes, actual_hash) = stable.into_bytes_and_sha256();
        if actual_hash != declaration.sha256 {
            return Err(self.failure(MachineInputErrorKind::SourceHashMismatch {
                source_id: declaration.source_id,
                declared: declaration.sha256,
                actual: actual_hash,
            }));
        }
        let text = String::from_utf8(bytes).map_err(|error| {
            self.failure(MachineInputErrorKind::SourceNotUtf8 {
                source_id: declaration.source_id,
                valid_up_to: u64::try_from(error.utf8_error().valid_up_to()).unwrap_or(u64::MAX),
            })
        })?;
        let facts = AdmittedMachineSourceFacts {
            source_id: SourceId::new(declaration.source_id),
            uri,
            bytes: observed,
            sha256: actual_hash,
        };
        let decoded_facts = DecodedPackageFacts {
            contract: decoded.decoded.wire().contract,
            canonical_sha256: decoded.decoded.canonical_jcs_sha256().into_bytes(),
        };
        let fingerprint = portable_fingerprint(&decoded.package.0, decoded_facts, &facts);
        let source = AdmittedMachineSource {
            facts: facts.clone(),
            text,
        };
        {
            let mut progress = self.progress.borrow_mut();
            progress.stage = MachineInputStage::SourcesAdmitted;
            progress.sources = vec![facts];
            progress.fingerprint = Some(fingerprint);
        }
        Ok(AdmittedMachineSourceSet {
            session: self.identity.clone(),
            package: decoded.package.clone(),
            declaration: decoded.declaration,
            sources: vec![source],
            fingerprint,
        })
    }

    /// Consume and exactly cross-check every session/package/declaration
    /// binding before issuing the package accepted by the syntax owner.
    pub fn finish(
        self,
        raw: AdmittedPackageBytes,
        decoded: SessionBoundDecodedPackage,
        sources: AdmittedMachineSourceSet,
    ) -> Result<AdmittedMachinePackage, MachineInputError> {
        self.require_stage(MachineInputStage::SourcesAdmitted)?;
        self.validate_session(&raw.session, MachineInputReceiptKind::RawPackage)?;
        self.validate_session(&decoded.session, MachineInputReceiptKind::DecodedPackage)?;
        self.validate_session(&sources.session, MachineInputReceiptKind::SourceSet)?;
        self.validate_package(&raw.package, MachineInputReceiptKind::RawPackage)?;
        self.validate_package(&decoded.package, MachineInputReceiptKind::DecodedPackage)?;
        self.validate_package(&sources.package, MachineInputReceiptKind::SourceSet)?;
        if raw.package != decoded.package || raw.package != sources.package {
            return Err(self.failure(MachineInputErrorKind::ReceiptPackageMismatch(
                MachineInputReceiptKind::SourceSet,
            )));
        }
        if decoded.declaration != sources.declaration {
            return Err(self.failure(MachineInputErrorKind::ReceiptDeclarationMismatch));
        }
        if decoded.decoded.raw_sha256().as_bytes() != &raw.package.0.sha256 {
            return Err(self.failure(MachineInputErrorKind::PackageHashMismatch));
        }
        let state_fingerprint = self.progress.borrow().fingerprint;
        if state_fingerprint != Some(sources.fingerprint) {
            return Err(self.failure(MachineInputErrorKind::ReceiptDeclarationMismatch));
        }

        let progress = self.progress_snapshot();
        let read_ledger = self.host.read_ledger().clone();
        let SessionBoundDecodedPackage { decoded, .. } = decoded;
        let AdmittedMachineSourceSet {
            sources,
            fingerprint,
            ..
        } = sources;
        let _ = raw;
        Ok(AdmittedMachinePackage {
            session: self.identity.clone(),
            decoded,
            sources,
            progress,
            fingerprint,
            read_ledger,
        })
    }

    fn require_stage(&self, expected: MachineInputStage) -> Result<(), MachineInputError> {
        let actual = self.progress.borrow().stage;
        if actual != expected {
            return Err(self.failure(MachineInputErrorKind::InvalidProgress { expected, actual }));
        }
        Ok(())
    }

    fn validate_session(
        &self,
        session: &MachineInputSessionIdentity,
        receipt: MachineInputReceiptKind,
    ) -> Result<(), MachineInputError> {
        if session != &self.identity {
            return Err(self.failure(MachineInputErrorKind::ReceiptSessionMismatch(receipt)));
        }
        Ok(())
    }

    fn validate_package(
        &self,
        package: &PackageBinding,
        receipt: MachineInputReceiptKind,
    ) -> Result<(), MachineInputError> {
        if self.progress.borrow().package.as_ref() != Some(&package.0) {
            return Err(self.failure(MachineInputErrorKind::ReceiptPackageMismatch(receipt)));
        }
        Ok(())
    }

    fn progress_snapshot(&self) -> MachineInputProgress {
        MachineInputProgress::issued(&self.identity, self.progress.borrow().clone())
    }

    fn failure(&self, kind: MachineInputErrorKind) -> MachineInputError {
        MachineInputError {
            kind: Box::new(kind),
            progress: Box::new(self.progress_snapshot()),
            read_ledger: self.host.read_ledger().clone(),
        }
    }
}

fn map_package_host_error(error: HostAdmissionError) -> MachineInputErrorKind {
    match error {
        HostAdmissionError::UnsupportedContainedOpen => {
            MachineInputErrorKind::UnsupportedContainedOpen
        }
        error => MachineInputErrorKind::PackageOpen(error),
    }
}

fn source_declaration_fingerprint(sources: &[WireSource]) -> SourceDeclarationFingerprint {
    let mut jcs = String::from("[");
    for (index, source) in sources.iter().enumerate() {
        if index != 0 {
            jcs.push(',');
        }
        jcs.push_str("{\"sha256\":");
        push_sha256(&mut jcs, source.sha256);
        jcs.push_str(",\"source_id\":");
        jcs.push_str(&source.source_id.to_string());
        jcs.push_str(",\"uri\":");
        push_jcs_string(&mut jcs, &source.uri);
        jcs.push_str(",\"utf8_byte_length\":");
        jcs.push_str(&source.utf8_byte_length.to_string());
        jcs.push('}');
    }
    jcs.push(']');
    SourceDeclarationFingerprint(sha256(jcs.as_bytes()))
}

fn portable_fingerprint(
    package: &AdmittedPackageFacts,
    decoded: DecodedPackageFacts,
    source: &AdmittedMachineSourceFacts,
) -> MachineInputFingerprint {
    machine_input_fingerprint_from_jcs(&portable_fingerprint_jcs(package, decoded, source))
}

fn portable_fingerprint_jcs(
    package: &AdmittedPackageFacts,
    decoded: DecodedPackageFacts,
    source: &AdmittedMachineSourceFacts,
) -> String {
    let mut jcs = String::from("{\"algorithm\":");
    push_jcs_string(&mut jcs, MachineInputFingerprint::ALGORITHM_ID);
    jcs.push_str(",\"package\":{\"bytes\":");
    jcs.push_str(&package.bytes.to_string());
    jcs.push_str(",\"canonical_sha256\":");
    push_sha256(&mut jcs, decoded.canonical_sha256);
    jcs.push_str(",\"contract\":");
    push_jcs_string(&mut jcs, decoded.contract.as_str());
    jcs.push_str(",\"sha256\":");
    push_sha256(&mut jcs, package.sha256);
    jcs.push_str(",\"uri\":");
    push_jcs_string(&mut jcs, package.uri.as_str());
    jcs.push_str("},\"sources\":[{\"bytes\":");
    jcs.push_str(&source.bytes.to_string());
    jcs.push_str(",\"sha256\":");
    push_sha256(&mut jcs, source.sha256);
    jcs.push_str(",\"source_id\":");
    jcs.push_str(&source.source_id.get().to_string());
    jcs.push_str(",\"uri\":");
    push_jcs_string(&mut jcs, source.uri.as_str());
    jcs.push_str("}]}");
    jcs
}

fn push_sha256(output: &mut String, hash: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in hash {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests;
