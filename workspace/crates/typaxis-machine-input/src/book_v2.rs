//! Private contract-1.5 host input. No legacy decoded/progress/input receipt is
//! issued for a successor package, and no public contract/profile is registered.
use super::*;
use typaxis_document_package::book_v2::{
    BookV2DocumentPackageDecoder, BookV2WireError, DecodedBookV2DocumentPackage,
    BOOK_V2_DOCUMENT_PACKAGE_CONTRACT,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2InputFingerprint([u8; 32]);
impl BookV2InputFingerprint {
    pub const ALGORITHM: &'static str = "typaxis.book-2-host-input/1";
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Debug)]
struct State {
    session: Option<MachineInputSessionIdentity>,
    stage: MachineInputStage,
    package: Option<AdmittedPackageFacts>,
    canonical_sha256: Option<[u8; 32]>,
    sources: Vec<AdmittedMachineSourceFacts>,
    fingerprint: Option<BookV2InputFingerprint>,
}
/// Sealed successor progress, deliberately distinct from 1.4 progress facts.
#[derive(Debug)]
pub struct BookV2InputProgress(State);
impl BookV2InputProgress {
    pub const fn stage(&self) -> MachineInputStage {
        self.0.stage
    }
    pub fn session_identity(&self) -> Option<&MachineInputSessionIdentity> {
        self.0.session.as_ref()
    }
    pub fn package(&self) -> Option<&AdmittedPackageFacts> {
        self.0.package.as_ref()
    }
    pub const fn canonical_sha256(&self) -> Option<[u8; 32]> {
        self.0.canonical_sha256
    }
    pub fn sources(&self) -> &[AdmittedMachineSourceFacts] {
        &self.0.sources
    }
    pub const fn fingerprint(&self) -> Option<BookV2InputFingerprint> {
        self.0.fingerprint
    }
    pub fn decoded_contract(&self) -> Option<&'static str> {
        self.0
            .canonical_sha256
            .map(|_| BOOK_V2_DOCUMENT_PACKAGE_CONTRACT)
    }
}
#[derive(Debug)]
pub enum BookV2InputErrorKind {
    Host(MachineInputErrorKind),
    Decode(BookV2WireError),
    SourceCount { maximum: u32, observed: usize },
    SourceOrder { expected: u32, observed: u32 },
    AllocationFailure,
    SourceRegistration(HostAdmissionError),
}
#[derive(Debug)]
pub struct BookV2InputError {
    kind: BookV2InputErrorKind,
    progress: BookV2InputProgress,
    read_ledger: HostReadIdentityLedger,
}
impl BookV2InputError {
    pub const fn kind(&self) -> &BookV2InputErrorKind {
        &self.kind
    }
    pub const fn progress(&self) -> &BookV2InputProgress {
        &self.progress
    }
    pub fn read_ledger_token(&self) -> Result<HostReadIdentityLedgerToken, HostAdmissionError> {
        self.read_ledger.token()
    }
}
impl fmt::Display for BookV2InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            BookV2InputErrorKind::Decode(error) => error.fmt(f),
            kind => write!(f, "book-2 host input {kind:?}"),
        }
    }
}
impl std::error::Error for BookV2InputError {}

#[derive(Debug)]
pub struct BookV2PackageBytes(AdmittedPackageBytes);
impl BookV2PackageBytes {
    pub fn bytes(&self) -> &[u8] {
        self.0.bytes()
    }
}
#[derive(Debug)]
pub struct SessionBoundBookV2Decoded {
    session: MachineInputSessionIdentity,
    package: PackageBinding,
    declaration: SourceDeclarationFingerprint,
    decoded: DecodedBookV2DocumentPackage,
}
impl SessionBoundBookV2Decoded {
    pub const fn decoded(&self) -> &DecodedBookV2DocumentPackage {
        &self.decoded
    }
}
#[derive(Debug)]
pub struct AdmittedBookV2Sources {
    session: MachineInputSessionIdentity,
    package: PackageBinding,
    declaration: SourceDeclarationFingerprint,
    sources: Vec<AdmittedMachineSource>,
    fingerprint: BookV2InputFingerprint,
}
impl AdmittedBookV2Sources {
    pub fn sources(&self) -> &[AdmittedMachineSource] {
        &self.sources
    }
}

#[derive(Debug)]
pub struct BookV2InputProvenance {
    progress: BookV2InputProgress,
    read_ledger: HostReadIdentityLedger,
    limits: ValidatedResourceLimits,
}
impl BookV2InputProvenance {
    pub const fn progress(&self) -> &BookV2InputProgress {
        &self.progress
    }
    pub const fn read_ledger(&self) -> &HostReadIdentityLedger {
        &self.read_ledger
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub fn read_ledger_token(&self) -> Result<HostReadIdentityLedgerToken, HostAdmissionError> {
        self.read_ledger.token()
    }
}
/// Move-only stable PACKAGE and source admission. AST validation still belongs
/// to syntax; this receipt cannot become an admitted legacy package.
///
/// ```compile_fail
/// use typaxis_machine_input::{book_v2::AdmittedBookV2Input, AdmittedSemanticMachinePackage};
/// fn legacy(value: AdmittedBookV2Input) -> AdmittedSemanticMachinePackage { value }
/// ```
#[derive(Debug)]
pub struct AdmittedBookV2Input {
    decoded: DecodedBookV2DocumentPackage,
    sources: Vec<AdmittedMachineSource>,
    provenance: BookV2InputProvenance,
}
impl AdmittedBookV2Input {
    pub const fn decoded(&self) -> &DecodedBookV2DocumentPackage {
        &self.decoded
    }
    pub fn sources(&self) -> &[AdmittedMachineSource] {
        &self.sources
    }
    pub const fn provenance(&self) -> &BookV2InputProvenance {
        &self.provenance
    }
    pub fn into_parts(
        self,
    ) -> (
        DecodedBookV2DocumentPackage,
        Vec<AdmittedMachineSource>,
        BookV2InputProvenance,
    ) {
        (self.decoded, self.sources, self.provenance)
    }
}

#[derive(Debug)]
pub struct HostBookV2InputSession {
    transport: HostMachineInputSession,
    state: RefCell<State>,
}
impl HostBookV2InputSession {
    pub fn open(
        options: MachineInputHostOptions,
        limits: &ValidatedResourceLimits,
    ) -> Result<(Self, BookV2PackageBytes), BookV2InputError> {
        Self::open_with_read_ledger(options, limits, &HostReadIdentityLedger::new())
    }
    pub fn open_with_read_ledger(
        options: MachineInputHostOptions,
        limits: &ValidatedResourceLimits,
        ledger: &HostReadIdentityLedger,
    ) -> Result<(Self, BookV2PackageBytes), BookV2InputError> {
        // Raw stable reads carry no decoded contract facts. Keep the old
        // transport private so it cannot issue 1.4 decoded/source progress.
        let (transport, raw) = HostMachineInputSession::open_with_read_ledger(
            options, limits, ledger,
        )
        .map_err(|error| BookV2InputError {
            kind: BookV2InputErrorKind::Host(*error.kind),
            progress: BookV2InputProgress(State {
                session: None,
                stage: MachineInputStage::NoInput,
                package: None,
                canonical_sha256: None,
                sources: Vec::new(),
                fingerprint: None,
            }),
            read_ledger: error.read_ledger,
        })?;
        let state = State {
            session: Some(transport.identity.clone()),
            stage: MachineInputStage::RawPackageAdmitted,
            package: Some(raw.package.0.clone()),
            canonical_sha256: None,
            sources: Vec::new(),
            fingerprint: None,
        };
        Ok((
            Self {
                transport,
                state: RefCell::new(state),
            },
            BookV2PackageBytes(raw),
        ))
    }
    pub fn progress(&self) -> BookV2InputProgress {
        BookV2InputProgress(self.state.borrow().clone())
    }
    pub const fn read_ledger(&self) -> &HostReadIdentityLedger {
        self.transport.read_ledger()
    }
    fn failure(&self, kind: BookV2InputErrorKind) -> BookV2InputError {
        BookV2InputError {
            kind,
            progress: self.progress(),
            read_ledger: self.read_ledger().clone(),
        }
    }
    fn host_failure(&self, kind: MachineInputErrorKind) -> BookV2InputError {
        self.failure(BookV2InputErrorKind::Host(kind))
    }
    fn require_stage(&self, expected: MachineInputStage) -> Result<(), BookV2InputError> {
        let actual = self.state.borrow().stage;
        if actual != expected {
            return Err(
                self.host_failure(MachineInputErrorKind::InvalidProgress { expected, actual })
            );
        }
        Ok(())
    }
    fn require_binding(
        &self,
        session: &MachineInputSessionIdentity,
        package: &PackageBinding,
        kind: MachineInputReceiptKind,
    ) -> Result<(), BookV2InputError> {
        if session != &self.transport.identity {
            return Err(self.host_failure(MachineInputErrorKind::ReceiptSessionMismatch(kind)));
        }
        if self.state.borrow().package.as_ref() != Some(&package.0) {
            return Err(self.host_failure(MachineInputErrorKind::ReceiptPackageMismatch(kind)));
        }
        Ok(())
    }
    pub fn decode_and_bind(
        &self,
        raw: &BookV2PackageBytes,
        policy: &DocumentPackageDecodePolicy<'_>,
    ) -> Result<SessionBoundBookV2Decoded, BookV2InputError> {
        self.require_stage(MachineInputStage::RawPackageAdmitted)?;
        self.require_binding(
            &raw.0.session,
            &raw.0.package,
            MachineInputReceiptKind::RawPackage,
        )?;
        if policy.preflight_limits() != self.transport.package_limits
            || policy.resource_limits() != &self.transport.resource_limits
        {
            return Err(self.host_failure(MachineInputErrorKind::DecodePolicyMismatch));
        }
        let decoded = BookV2DocumentPackageDecoder::new()
            .decode(raw.bytes(), policy)
            .map_err(|error| self.failure(BookV2InputErrorKind::Decode(error)))?;
        if decoded.raw_sha256() != raw.0.package.0.sha256 {
            return Err(self.host_failure(MachineInputErrorKind::PackageHashMismatch));
        }
        let declaration = source_declaration_fingerprint_m4(decoded.wire().sources())
            .map_err(|kind| self.host_failure(kind))?;
        {
            let mut state = self.state.borrow_mut();
            state.stage = MachineInputStage::PackageDecoded;
            state.canonical_sha256 = Some(decoded.canonical_jcs_sha256());
        }
        Ok(SessionBoundBookV2Decoded {
            session: self.transport.identity.clone(),
            package: raw.0.package.clone(),
            declaration,
            decoded,
        })
    }
    pub fn admit_sources(
        &self,
        decoded: &SessionBoundBookV2Decoded,
        limits: &ValidatedResourceLimits,
    ) -> Result<AdmittedBookV2Sources, BookV2InputError> {
        self.require_stage(MachineInputStage::PackageDecoded)?;
        self.require_binding(
            &decoded.session,
            &decoded.package,
            MachineInputReceiptKind::DecodedPackage,
        )?;
        if limits != &self.transport.resource_limits {
            return Err(self.host_failure(MachineInputErrorKind::DecodePolicyMismatch));
        }
        let declarations = decoded.decoded.wire().sources();
        if declarations.is_empty()
            || declarations.len() as u64 > u64::from(limits.get().max_include_files)
        {
            return Err(self.failure(BookV2InputErrorKind::SourceCount {
                maximum: limits.get().max_include_files,
                observed: declarations.len(),
            }));
        }
        // Preflight all declaration sizes and IDs before reading any source.
        let mut total = 0u64;
        for (index, declaration) in declarations.iter().enumerate() {
            if declaration.source_id as usize != index {
                return Err(self.failure(BookV2InputErrorKind::SourceOrder {
                    expected: index as u32,
                    observed: declaration.source_id,
                }));
            }
            let declared = u64::from(declaration.utf8_byte_length);
            if declared > u64::from(limits.get().max_source_bytes) {
                return Err(
                    self.host_failure(MachineInputErrorKind::SourceDeclaredLimit {
                        source_id: declaration.source_id,
                        maximum: u64::from(limits.get().max_source_bytes),
                        declared,
                    }),
                );
            }
            total = total.checked_add(declared).unwrap_or(u64::MAX);
            if total > limits.get().max_input_bytes {
                return Err(
                    self.host_failure(MachineInputErrorKind::AggregateInputLimit {
                        maximum: limits.get().max_input_bytes,
                        attempted: total,
                    }),
                );
            }
        }
        let mut paths = Vec::new();
        paths
            .try_reserve_exact(declarations.len())
            .map_err(|_| self.failure(BookV2InputErrorKind::AllocationFailure))?;
        for declaration in declarations {
            if declaration.uri.len() as u64 > u64::from(limits.get().max_uri_bytes) {
                return Err(self.host_failure(MachineInputErrorKind::SourceUriTooLong {
                    source_id: declaration.source_id,
                    maximum: limits.get().max_uri_bytes,
                    observed: declaration.uri.len() as u64,
                }));
            }
            paths.push(PortablePath::new(declaration.uri.clone()).map_err(|cause| {
                self.host_failure(MachineInputErrorKind::UnsafeSourceUri {
                    source_id: declaration.source_id,
                    cause,
                })
            })?);
        }
        // Register the entire source set before the first file is opened, so
        // even an early read failure protects every input from sidecar writes.
        let candidates = self
            .transport
            .host
            .roots()
            .register_candidates(paths.iter())
            .map_err(|error| self.failure(BookV2InputErrorKind::SourceRegistration(error)))?;
        if candidates.len() != declarations.len() {
            return Err(self.host_failure(MachineInputErrorKind::ReceiptDeclarationMismatch));
        }
        let mut sources = Vec::new();
        let mut facts = Vec::new();
        sources
            .try_reserve_exact(declarations.len())
            .map_err(|_| self.failure(BookV2InputErrorKind::AllocationFailure))?;
        facts
            .try_reserve_exact(declarations.len())
            .map_err(|_| self.failure(BookV2InputErrorKind::AllocationFailure))?;
        let mut read_bytes = 0u64;
        for (declaration, candidate) in declarations.iter().zip(&candidates) {
            let source = read_semantic_source(
                &self.transport.host,
                declaration,
                limits,
                read_bytes,
                Some(candidate),
            )
            .map_err(|kind| self.host_failure(kind))?;
            read_bytes = read_bytes.checked_add(source.facts.bytes).ok_or_else(|| {
                self.host_failure(MachineInputErrorKind::AggregateInputLimit {
                    maximum: limits.get().max_input_bytes,
                    attempted: u64::MAX,
                })
            })?;
            facts.push(source.facts.clone());
            sources.push(source);
        }
        let fingerprint = fingerprint(
            &decoded.package.0,
            decoded.decoded.canonical_jcs_sha256(),
            &facts,
        );
        {
            let mut state = self.state.borrow_mut();
            state.stage = MachineInputStage::SourcesAdmitted;
            state.sources = facts;
            state.fingerprint = Some(fingerprint);
        }
        Ok(AdmittedBookV2Sources {
            session: self.transport.identity.clone(),
            package: decoded.package.clone(),
            declaration: decoded.declaration,
            sources,
            fingerprint,
        })
    }
    pub fn finish(
        self,
        raw: BookV2PackageBytes,
        decoded: SessionBoundBookV2Decoded,
        sources: AdmittedBookV2Sources,
    ) -> Result<AdmittedBookV2Input, BookV2InputError> {
        self.require_stage(MachineInputStage::SourcesAdmitted)?;
        self.require_binding(
            &raw.0.session,
            &raw.0.package,
            MachineInputReceiptKind::RawPackage,
        )?;
        self.require_binding(
            &decoded.session,
            &decoded.package,
            MachineInputReceiptKind::DecodedPackage,
        )?;
        self.require_binding(
            &sources.session,
            &sources.package,
            MachineInputReceiptKind::SourceSet,
        )?;
        let state = self.state.borrow();
        if decoded.declaration != sources.declaration
            || decoded.decoded.raw_sha256() != raw.0.package.0.sha256
            || state.canonical_sha256 != Some(decoded.decoded.canonical_jcs_sha256())
            || state.fingerprint != Some(sources.fingerprint)
            || sources
                .sources
                .iter()
                .map(|source| &source.facts)
                .ne(state.sources.iter())
        {
            return Err(self.host_failure(MachineInputErrorKind::ReceiptDeclarationMismatch));
        }
        drop(state);
        let provenance = BookV2InputProvenance {
            progress: self.progress(),
            read_ledger: self.read_ledger().clone(),
            limits: self.transport.resource_limits.clone(),
        };
        Ok(AdmittedBookV2Input {
            decoded: decoded.decoded,
            sources: sources.sources,
            provenance,
        })
    }
}
fn fingerprint(
    package: &AdmittedPackageFacts,
    canonical: [u8; 32],
    sources: &[AdmittedMachineSourceFacts],
) -> BookV2InputFingerprint {
    let mut jcs = String::from("{\"algorithm\":");
    push_jcs_string(&mut jcs, BookV2InputFingerprint::ALGORITHM);
    jcs.push_str(",\"package\":{\"bytes\":");
    jcs.push_str(&package.bytes.to_string());
    jcs.push_str(",\"canonical_sha256\":");
    push_sha256(&mut jcs, canonical);
    jcs.push_str(",\"contract\":");
    push_jcs_string(&mut jcs, BOOK_V2_DOCUMENT_PACKAGE_CONTRACT);
    jcs.push_str(",\"sha256\":");
    push_sha256(&mut jcs, package.sha256);
    jcs.push_str(",\"uri\":");
    push_jcs_string(&mut jcs, package.uri.as_str());
    jcs.push_str("},\"sources\":[");
    for (index, source) in sources.iter().enumerate() {
        if index != 0 {
            jcs.push(',');
        }
        jcs.push_str("{\"bytes\":");
        jcs.push_str(&source.bytes.to_string());
        jcs.push_str(",\"sha256\":");
        push_sha256(&mut jcs, source.sha256);
        jcs.push_str(",\"source_id\":");
        jcs.push_str(&source.source_id.get().to_string());
        jcs.push_str(",\"uri\":");
        push_jcs_string(&mut jcs, source.uri.as_str());
        jcs.push('}');
    }
    jcs.push_str("]}");
    BookV2InputFingerprint(sha256(jcs.as_bytes()))
}

#[cfg(test)]
#[path = "book_v2_tests.rs"]
mod tests;
