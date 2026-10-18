//! Optional accounting for the owned read buffer and bounded read/digest steps.
//! These requests confer no open, session, root, or stable-byte authority.
use crate::HostAdmissionError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostReadStage {
    OpenPath,
    OpenRoot,
    OpenLock,
    OpenLedger,
    OpenOwner,
    RevalidatePath,
    RevalidateRoot,
    DeclarationCheck,
    DeclarationUri,
    /// Resource resolver bookkeeping, before any host read begins.
    AttemptStorage,
    AttemptMigrate,
    AttemptLookup,
    AttemptMark,
    Buffer,
    Chunk,
    DigestFinish,
    OpenIdentityLookup,
    OpenIdentityStorage,
    OpenIdentityMigrate,
    LedgerView,
    LedgerCandidatePath,
    LedgerCandidateWalk,
    LedgerAliasPath,
    LedgerAliasProbe,
    /// Native canonicalization work. Its internal allocation remains separate
    /// from the explicitly reserved path scratch.
    LedgerAliasCanonicalize,
    LedgerOpenedPath,
    LedgerOpenedWalk,
    LedgerFinish,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostReadRequest {
    pub stage: HostReadStage,
    pub file_offset: u64,
    /// Logical owned-buffer capacity, requested once before native allocation.
    /// AttemptStorage uses this for the resolver's packed attempt bitmap;
    /// DeclarationUri uses it for the pending resource's owned portable URI.
    /// RevalidatePath reserves the reusable long-path C-string buffer. Short
    /// paths use a fixed stack buffer. Actual allocator overcapacity is separate.
    pub buffer_bytes: u64,
    /// Byte work plus a fixed allowance; this is not a CPU instruction count.
    pub work: u64,
}

pub trait HostReadBudget {
    type Error;
    const OBSERVED: bool = true;
    /// Opt in to resource resolver attempt bookkeeping. Host-only callers and
    /// existing read-buffer observers keep their original request sequence.
    const OBSERVE_ATTEMPTS: bool = false;
    /// Account for declaration comparison and the pending resource's URI copy.
    const OBSERVE_DECLARATIONS: bool = false;
    /// Prepare the native revalidation path and fund each admitted root walk.
    const OBSERVE_RESOLUTION: bool = false;
    /// Fund an authentic registered open, its lock, ledger update and identity.
    const OBSERVE_OPEN: bool = false;
    /// Fund distinct opened-identity counting before ledger mutation.
    const OBSERVE_OPEN_IDENTITIES: bool = false;
    /// Fund token-wide views, path scratch and filesystem checks. Native
    /// canonicalization and write-target resolution still require separate
    /// allocation accounting; these hooks do not assert whole-host closure.
    const OBSERVE_LEDGER: bool = false;
    /// Debit the whole request atomically before the operation. Accepted
    /// requests remain spent after refusal, short reads, or native failures.
    fn charge(&mut self, request: HostReadRequest) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostReadBudgetFailure<E> {
    pub error: E,
    pub request: HostReadRequest,
}

#[derive(Debug)]
pub enum HostReadError<E> {
    Native(HostAdmissionError),
    Budget(HostReadBudgetFailure<E>),
}
impl<E> From<HostAdmissionError> for HostReadError<E> {
    fn from(error: HostAdmissionError) -> Self {
        Self::Native(error)
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for HostReadError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "host read: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for HostReadError<E> {}

pub(crate) struct UnobservedRead;
impl HostReadBudget for UnobservedRead {
    type Error = std::convert::Infallible;
    const OBSERVED: bool = false;
    fn charge(&mut self, _: HostReadRequest) -> Result<(), Self::Error> {
        unreachable!("ordinary read never calls budget hooks")
    }
}

pub(crate) fn charge<B: HostReadBudget>(
    budget: &mut B,
    request: HostReadRequest,
) -> Result<(), HostReadError<B::Error>> {
    if B::OBSERVED {
        budget
            .charge(request)
            .map_err(|error| HostReadError::Budget(HostReadBudgetFailure { error, request }))?;
    }
    Ok(())
}
