#![forbid(unsafe_code)]

use core::num::NonZeroU32;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use typaxis_core::{
    admitted_resource_fingerprint_from_jcs, push_jcs_string, AdmittedResourceFingerprint,
    EffectiveConfig, FontFaceId, HostAdmissionContext, ImageResourceId, PortablePath,
    ValidatedResourceLimits,
};
use typaxis_diagnostics::{DiagnosticSubject, PublicMachineError, ResourceErrorSubject};
use typaxis_document::ResourceCatalog;
use typaxis_font::{FontFamilyError, FontFamilyTable};
use typaxis_host_admission::{
    HostAdmissionError, HostAdmissionSession, HostReadIdentityLedger, HostReadIdentityLedgerToken,
    HostRootSetToken, OpenedContainedFile, RegisteredHostReadCandidate,
};

/// Resource-admission owner's sealed compile-time capability. The generic
/// host owner defines its representation; this crate exposes and enforces it
/// at the logical resource boundary.
pub use typaxis_host_admission::HostResourceCapabilityToken as ResourceAdmissionCapabilityToken;

#[derive(Clone)]
struct ResourceAdmissionSessionIdentity(Arc<()>);

impl ResourceAdmissionSessionIdentity {
    fn fresh() -> Self {
        Self(Arc::new(()))
    }
}

impl std::fmt::Debug for ResourceAdmissionSessionIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ResourceAdmissionSessionIdentity(..)")
    }
}

impl PartialEq for ResourceAdmissionSessionIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ResourceAdmissionSessionIdentity {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedFontMetadata {
    pub units_per_em: u16,
    pub glyph_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedFont {
    font_face_id: FontFaceId,
    uri: PortablePath,
    family: String,
    face_index: u32,
    bytes: Vec<u8>,
    sha256: [u8; 32],
    metadata: AdmittedFontMetadata,
}
impl AdmittedFont {
    fn from_verified(
        font_face_id: FontFaceId,
        uri: PortablePath,
        family: String,
        face_index: u32,
        bytes: Vec<u8>,
        sha256: [u8; 32],
        metadata: AdmittedFontMetadata,
    ) -> Self {
        Self {
            font_face_id,
            uri,
            family,
            face_index,
            bytes,
            sha256,
            metadata,
        }
    }
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub fn family(&self) -> &str {
        &self.family
    }
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn metadata(&self) -> &AdmittedFontMetadata {
        &self.metadata
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedImage {
    image_id: ImageResourceId,
    uri: PortablePath,
    bytes: Vec<u8>,
    sha256: [u8; 32],
    width: NonZeroU32,
    height: NonZeroU32,
    decoded_bytes: u64,
}
impl AdmittedImage {
    fn from_verified(
        image_id: ImageResourceId,
        uri: PortablePath,
        bytes: Vec<u8>,
        sha256: [u8; 32],
        width: NonZeroU32,
        height: NonZeroU32,
        decoded_bytes: u64,
    ) -> Self {
        Self {
            image_id,
            uri,
            bytes,
            sha256,
            width,
            height,
            decoded_bytes,
        }
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn width(&self) -> NonZeroU32 {
        self.width
    }
    pub const fn height(&self) -> NonZeroU32 {
        self.height
    }
    pub const fn decoded_bytes(&self) -> u64 {
        self.decoded_bytes
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceAdmissionError {
    MissingLogicalResource,
    ConflictingLogicalResource,
    ResourceLimit,
    ExpectedHashMismatch,
    InvalidMetadata,
    InvalidFontFamily,
    NonCanonicalResourceId,
    ResourceRead,
    ResourceLengthMismatch,
    ReceiptKindMismatch,
    ReceiptIdentityMismatch,
    ReceiptSessionMismatch,
    MissingAdmittedRootSet,
    RootSetMismatch,
    RootUnavailable,
    RootNotDirectory,
    AliasedRoot,
    UnsupportedContainedOpen,
    MissingResourceCandidate,
    AmbiguousResourceCandidate,
    UnsafeResourceCandidate,
    ResourceNotRegularFile,
    ResourceLockUnavailable,
}

impl ResourceAdmissionError {
    /// Stable canonical text for projection into diagnostics. Host error text
    /// is deliberately discarded by the typed host-to-resource mapper.
    pub const fn canonical_message(self) -> &'static str {
        match self {
            Self::MissingLogicalResource => "logical resource is missing",
            Self::ConflictingLogicalResource => "logical resource was admitted more than once",
            Self::ResourceLimit => "resource admission limit was exceeded",
            Self::ExpectedHashMismatch => "resource hash does not match the declaration",
            Self::InvalidMetadata => "resource format or metadata is unsupported",
            Self::InvalidFontFamily => "font family declaration is invalid",
            Self::NonCanonicalResourceId => "resource ID is not canonical",
            Self::ResourceRead => "resource could not be read",
            Self::ResourceLengthMismatch => "resource length changed during admission",
            Self::ReceiptKindMismatch => "resource receipt kind does not match",
            Self::ReceiptIdentityMismatch => "resource receipt identity does not match",
            Self::ReceiptSessionMismatch => "resource receipt session does not match",
            Self::MissingAdmittedRootSet => "admitted resource root set is missing",
            Self::RootSetMismatch => "resource root set does not match",
            Self::RootUnavailable => "resource root is unavailable",
            Self::RootNotDirectory => "resource root is not a directory",
            Self::AliasedRoot => "resource roots resolve to the same identity",
            Self::UnsupportedContainedOpen => "contained resource open is unavailable",
            Self::MissingResourceCandidate => "resource candidate is missing",
            Self::AmbiguousResourceCandidate => "resource candidate is ambiguous",
            Self::UnsafeResourceCandidate => "resource candidate is not contained",
            Self::ResourceNotRegularFile => "resource candidate is not a regular file",
            Self::ResourceLockUnavailable => "resource read lock is unavailable",
        }
    }
}

/// A resource admission error paired with the logical font/image/URI that
/// caused it. The machine diagnostic mapper consumes this typed subject and
/// never parses `Debug` output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAdmissionFailure {
    error: ResourceAdmissionError,
    subject: ResourceErrorSubject,
}

/// A typed resource failure paired with the last successfully verified
/// resource-set snapshot from the same resolver session.
///
/// The progress receipt is owner-issued and cannot be reconstructed from the
/// public error subject or resource record fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAdmissionFailureOutcome {
    failure: ResourceAdmissionFailure,
    progress: ResourceAdmissionProgressToken,
}

impl ResourceAdmissionFailureOutcome {
    pub const fn failure(&self) -> &ResourceAdmissionFailure {
        &self.failure
    }

    pub const fn progress(&self) -> &ResourceAdmissionProgressToken {
        &self.progress
    }

    pub fn into_parts(self) -> (ResourceAdmissionFailure, ResourceAdmissionProgressToken) {
        (self.failure, self.progress)
    }
}

impl ResourceAdmissionFailure {
    fn new(error: ResourceAdmissionError, subject: ResourceErrorSubject) -> Self {
        Self { error, subject }
    }

    pub const fn error(&self) -> ResourceAdmissionError {
        self.error
    }

    pub const fn subject(&self) -> &ResourceErrorSubject {
        &self.subject
    }

    pub fn diagnostic_subject(&self) -> DiagnosticSubject {
        DiagnosticSubject::Resource(self.subject.clone())
    }

    pub const fn canonical_message(&self) -> &'static str {
        self.error.canonical_message()
    }

    /// Only errors with a code fixed by the current machine diagnostic table
    /// are mapped here. Other operational resource failures remain typed until
    /// their public code is assigned by a later integration milestone.
    pub fn public_error(&self) -> Option<PublicMachineError> {
        match self.error {
            ResourceAdmissionError::InvalidMetadata => Some(
                PublicMachineError::UnsupportedResource(self.subject.clone()),
            ),
            ResourceAdmissionError::UnsupportedContainedOpen => {
                Some(PublicMachineError::CompiledHostUnavailable)
            }
            _ => None,
        }
    }

    pub fn into_parts(self) -> (ResourceAdmissionError, ResourceErrorSubject) {
        (self.error, self.subject)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingResourceId {
    Font(FontFaceId),
    Image(ImageResourceId),
}

/// Logical resource binding around one generic host-owned contained open.
/// Only this crate can attach a font/image ID to the host read identity.
///
/// ```compile_fail
/// use typaxis_resource_admission::VerifiedResourceSource;
/// let _forged: VerifiedResourceSource<'static> = VerifiedResourceSource {};
/// ```
pub struct VerifiedResourceSource<'roots> {
    id: PendingResourceId,
    opened: OpenedContainedFile<'roots>,
}

/// Resource adapter over the generic host owner. It alone resolves logical
/// font/image IDs to declarations and binds them to host open identities.
#[derive(Debug)]
pub struct HostResourceAdmissionSession {
    host: HostAdmissionSession,
    declarations: ResourceCatalog,
    font_candidates: Vec<RegisteredHostReadCandidate>,
    image_candidates: Vec<RegisteredHostReadCandidate>,
}

impl HostResourceAdmissionSession {
    /// Resolve, securely open, and de-alias the effective resource-root set.
    ///
    /// The complete declaration-by-root candidate product is reserved and its
    /// parent+leaf identities are registered before any resource file is open.
    pub fn new(
        context: &HostAdmissionContext,
        config: &EffectiveConfig,
        declarations: &ResourceCatalog,
    ) -> Result<Self, ResourceAdmissionError> {
        Self::new_inner(context, config, declarations, None)
    }

    /// Register resource candidates into the command-wide ledger already used
    /// for package, config, and source candidates.
    pub fn new_with_read_ledger(
        context: &HostAdmissionContext,
        config: &EffectiveConfig,
        declarations: &ResourceCatalog,
        ledger: &HostReadIdentityLedger,
    ) -> Result<Self, ResourceAdmissionError> {
        Self::new_inner(context, config, declarations, Some(ledger))
    }

    fn new_inner(
        context: &HostAdmissionContext,
        config: &EffectiveConfig,
        declarations: &ResourceCatalog,
        ledger: Option<&HostReadIdentityLedger>,
    ) -> Result<Self, ResourceAdmissionError> {
        if !ResourceAdmissionCapabilityToken::compiled().contained_resource_open() {
            return Err(ResourceAdmissionError::UnsupportedContainedOpen);
        }
        validate_declaration_order(declarations)?;
        let host = match ledger {
            Some(ledger) => HostAdmissionSession::new_with_read_ledger(context, config, ledger),
            None => HostAdmissionSession::new(context, config),
        }
        .map_err(map_host_error)?;
        let font_count = declarations.font_faces.len();
        let declaration_count = font_count
            .checked_add(declarations.images.len())
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        let paths = (0..declaration_count).map(|index| {
            if index < font_count {
                &declarations.font_faces[index].uri
            } else {
                &declarations.images[index - font_count].uri
            }
        });
        let mut font_candidates = host
            .roots()
            .register_candidates(paths)
            .map_err(map_host_error)?;
        let image_candidates = font_candidates.split_off(font_count);
        Ok(Self {
            host,
            declarations: declarations.clone(),
            font_candidates,
            image_candidates,
        })
    }

    pub const fn roots(&self) -> HostRootSetToken<'_> {
        self.host.roots()
    }

    /// Seal the command-wide PACKAGE/config/source/resource read set after the
    /// latest candidate registration or open. Publication owners consume this
    /// token; resource facts themselves never expose host paths or identities.
    pub fn read_ledger_token(&self) -> Result<HostReadIdentityLedgerToken, ResourceAdmissionError> {
        self.host.read_ledger().token().map_err(map_host_error)
    }

    pub fn open_font(
        &self,
        font_face_id: FontFaceId,
    ) -> Result<VerifiedResourceSource<'_>, ResourceAdmissionError> {
        let _declaration = self
            .declarations
            .font_faces
            .get(font_face_id.get() as usize)
            .filter(|candidate| candidate.font_face_id == font_face_id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        let opened = self
            .roots()
            .open_registered(
                self.font_candidates
                    .get(font_face_id.get() as usize)
                    .ok_or(ResourceAdmissionError::MissingLogicalResource)?,
            )
            .map_err(map_host_error)?;
        Ok(VerifiedResourceSource {
            id: PendingResourceId::Font(font_face_id),
            opened,
        })
    }

    pub fn open_image(
        &self,
        image_id: ImageResourceId,
    ) -> Result<VerifiedResourceSource<'_>, ResourceAdmissionError> {
        let _declaration = self
            .declarations
            .images
            .get(image_id.get() as usize)
            .filter(|candidate| candidate.image_id == image_id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        let opened = self
            .roots()
            .open_registered(
                self.image_candidates
                    .get(image_id.get() as usize)
                    .ok_or(ResourceAdmissionError::MissingLogicalResource)?,
            )
            .map_err(map_host_error)?;
        Ok(VerifiedResourceSource {
            id: PendingResourceId::Image(image_id),
            opened,
        })
    }

    pub fn open_font_with_subject(
        &self,
        font_face_id: FontFaceId,
    ) -> Result<VerifiedResourceSource<'_>, ResourceAdmissionFailure> {
        self.open_font(font_face_id).map_err(|error| {
            ResourceAdmissionFailure::new(error, ResourceErrorSubject::FontFace(font_face_id))
        })
    }

    pub fn open_image_with_subject(
        &self,
        image_id: ImageResourceId,
    ) -> Result<VerifiedResourceSource<'_>, ResourceAdmissionFailure> {
        self.open_image(image_id).map_err(|error| {
            ResourceAdmissionFailure::new(error, ResourceErrorSubject::Image(image_id))
        })
    }
}

fn map_host_error(error: HostAdmissionError) -> ResourceAdmissionError {
    match error {
        HostAdmissionError::HostLimit => ResourceAdmissionError::ResourceLimit,
        HostAdmissionError::ReadCapacity => ResourceAdmissionError::ResourceLimit,
        HostAdmissionError::Read => ResourceAdmissionError::ResourceRead,
        HostAdmissionError::LengthMismatch => ResourceAdmissionError::ResourceLengthMismatch,
        HostAdmissionError::SessionMismatch | HostAdmissionError::RootSetMismatch => {
            ResourceAdmissionError::RootSetMismatch
        }
        HostAdmissionError::ReadIdentityMismatch => ResourceAdmissionError::ReceiptIdentityMismatch,
        HostAdmissionError::RootUnavailable => ResourceAdmissionError::RootUnavailable,
        HostAdmissionError::RootNotDirectory => ResourceAdmissionError::RootNotDirectory,
        HostAdmissionError::AliasedRoot => ResourceAdmissionError::AliasedRoot,
        HostAdmissionError::UnsupportedContainedOpen => {
            ResourceAdmissionError::UnsupportedContainedOpen
        }
        HostAdmissionError::MissingCandidate => ResourceAdmissionError::MissingResourceCandidate,
        HostAdmissionError::AmbiguousCandidate => {
            ResourceAdmissionError::AmbiguousResourceCandidate
        }
        HostAdmissionError::UnsafeCandidate => ResourceAdmissionError::UnsafeResourceCandidate,
        HostAdmissionError::NotRegularFile => ResourceAdmissionError::ResourceNotRegularFile,
        HostAdmissionError::LockUnavailable => ResourceAdmissionError::ResourceLockUnavailable,
    }
}

/// Bytes read under an admission permit. Only `AdmittedResourceResolver`
/// can construct this value; metadata decoders may inspect it but cannot
/// replace its logical identity, exact length, or streaming digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingResourceBytes {
    session: ResourceAdmissionSessionIdentity,
    id: PendingResourceId,
    uri: PortablePath,
    face_index: Option<u32>,
    bytes: Vec<u8>,
    sha256: [u8; 32],
}
impl PendingResourceBytes {
    pub const fn font_face_id(&self) -> Option<FontFaceId> {
        match self.id {
            PendingResourceId::Font(id) => Some(id),
            PendingResourceId::Image(_) => None,
        }
    }
    pub const fn image_id(&self) -> Option<ImageResourceId> {
        match self.id {
            PendingResourceId::Font(_) => None,
            PendingResourceId::Image(id) => Some(id),
        }
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub const fn face_index(&self) -> Option<u32> {
        self.face_index
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }

    pub fn error_subject(&self) -> ResourceErrorSubject {
        match self.id {
            PendingResourceId::Font(id) => ResourceErrorSubject::FontFace(id),
            PendingResourceId::Image(id) => ResourceErrorSubject::Image(id),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum VerifiedMetadata {
    Font {
        source: PendingResourceBytes,
        metadata: AdmittedFontMetadata,
    },
    Image {
        source: PendingResourceBytes,
        width: NonZeroU32,
        height: NonZeroU32,
        decoded_bytes: u64,
    },
}

/// Unforgeable proof that a crate-owned parser derived metadata from the exact
/// bytes and identity in a `PendingResourceBytes` value. Constructors remain
/// crate-private so arbitrary caller metadata cannot cross the trusted boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedMetadataReceipt(VerifiedMetadata);
/// Capability owned by the in-crate metadata parser. It is deliberately not
/// constructible or cloneable by callers, while its issue methods define the
/// hand-off API for the eventual parser implementation.
#[derive(Debug)]
pub struct VerifiedMetadataReceiptOwner {
    _private: (),
}
impl VerifiedMetadataReceiptOwner {
    #[allow(dead_code)] // reserved for the in-crate font/image metadata parser
    fn new() -> Self {
        Self { _private: () }
    }
    pub fn issue_font(
        &self,
        source: PendingResourceBytes,
        metadata: AdmittedFontMetadata,
    ) -> Result<VerifiedMetadataReceipt, ResourceAdmissionError> {
        if source.font_face_id().is_none()
            || source.face_index().is_none()
            || !(16..=16_384).contains(&metadata.units_per_em)
            || metadata.glyph_count == 0
        {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        Ok(VerifiedMetadataReceipt(VerifiedMetadata::Font {
            source,
            metadata,
        }))
    }
    pub fn issue_image(
        &self,
        source: PendingResourceBytes,
        width: NonZeroU32,
        height: NonZeroU32,
        decoded_bytes: u64,
    ) -> Result<VerifiedMetadataReceipt, ResourceAdmissionError> {
        if source.image_id().is_none() || decoded_bytes == 0 {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        Ok(VerifiedMetadataReceipt(VerifiedMetadata::Image {
            source,
            width,
            height,
            decoded_bytes,
        }))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResourceReadKind {
    Font,
    Image,
}

#[derive(Clone, Debug)]
struct ResourceAdmissionBudget {
    limits: ValidatedResourceLimits,
    reserved_bytes: u64,
}
impl ResourceAdmissionBudget {
    fn new(
        declarations: &ResourceCatalog,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, ResourceAdmissionError> {
        if declarations.font_faces.len() > limits.get().max_fonts as usize
            || declarations.images.len() > limits.get().max_images as usize
        {
            return Err(ResourceAdmissionError::ResourceLimit);
        }
        Ok(Self {
            limits: limits.clone(),
            reserved_bytes: 0,
        })
    }
    fn reserve(
        &mut self,
        kind: ResourceReadKind,
        exact_length: u64,
    ) -> Result<(), ResourceAdmissionError> {
        let per_resource = match kind {
            ResourceReadKind::Font => self.limits.get().max_font_bytes,
            ResourceReadKind::Image => self.limits.get().max_image_bytes,
        };
        if exact_length == 0 || exact_length > per_resource {
            return Err(ResourceAdmissionError::ResourceLimit);
        }
        let aggregate = self
            .reserved_bytes
            .checked_add(exact_length)
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        if aggregate > self.limits.get().max_resource_bytes {
            return Err(ResourceAdmissionError::ResourceLimit);
        }
        self.reserved_bytes = aggregate;
        Ok(())
    }
}

/// Stateful owner of all resource admission work. It reserves every resource
/// before reading, and issues the immutable ledger only after every declaration
/// has one matching metadata receipt.
#[derive(Debug)]
pub struct AdmittedResourceResolver<'roots> {
    session: ResourceAdmissionSessionIdentity,
    roots: Option<HostRootSetToken<'roots>>,
    declarations: ResourceCatalog,
    budget: ResourceAdmissionBudget,
    attempted_fonts: BTreeSet<FontFaceId>,
    attempted_images: BTreeSet<ImageResourceId>,
    fonts: BTreeMap<FontFaceId, AdmittedFont>,
    images: BTreeMap<ImageResourceId, AdmittedImage>,
}
impl AdmittedResourceResolver<'static> {
    /// Safe empty-package workflow for lower crates that must not depend on
    /// `typaxis-document` merely to assemble an empty catalog in tests or
    /// blank-document execution.
    pub fn new_empty(limits: &ValidatedResourceLimits) -> Result<Self, ResourceAdmissionError> {
        Self::new_inner(
            &ResourceCatalog {
                font_faces: vec![],
                images: vec![],
            },
            limits,
            None,
        )
    }

    /// Empty packages need no filesystem capability. Any non-empty resource
    /// catalog must use `new_with_roots`; this prevents a caller from omitting
    /// the configured host-admission context.
    pub fn new(
        declarations: &ResourceCatalog,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, ResourceAdmissionError> {
        if !declarations.font_faces.is_empty() || !declarations.images.is_empty() {
            return Err(ResourceAdmissionError::MissingAdmittedRootSet);
        }
        Self::new_inner(declarations, limits, None)
    }
}
impl<'roots> AdmittedResourceResolver<'roots> {
    pub fn new_with_roots(
        declarations: &ResourceCatalog,
        limits: &ValidatedResourceLimits,
        roots: HostRootSetToken<'roots>,
    ) -> Result<Self, ResourceAdmissionError> {
        Self::new_inner(declarations, limits, Some(roots))
    }

    fn new_inner(
        declarations: &ResourceCatalog,
        limits: &ValidatedResourceLimits,
        roots: Option<HostRootSetToken<'roots>>,
    ) -> Result<Self, ResourceAdmissionError> {
        let budget = ResourceAdmissionBudget::new(declarations, limits)?;
        validate_declaration_order(declarations)?;
        Ok(Self {
            session: ResourceAdmissionSessionIdentity::fresh(),
            roots,
            declarations: declarations.clone(),
            budget,
            attempted_fonts: BTreeSet::new(),
            attempted_images: BTreeSet::new(),
            fonts: BTreeMap::new(),
            images: BTreeMap::new(),
        })
    }
    pub fn read_font(
        &mut self,
        source: VerifiedResourceSource<'roots>,
    ) -> Result<PendingResourceBytes, ResourceAdmissionError> {
        let roots = self.roots.ok_or(ResourceAdmissionError::RootSetMismatch)?;
        roots
            .validate_opened(&source.opened)
            .map_err(map_host_error)?;
        let PendingResourceId::Font(font_face_id) = source.id else {
            return Err(ResourceAdmissionError::ReceiptKindMismatch);
        };
        let declaration = self
            .declarations
            .font_faces
            .get(font_face_id.get() as usize)
            .filter(|candidate| candidate.font_face_id == font_face_id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        if self.attempted_fonts.contains(&font_face_id) {
            return Err(ResourceAdmissionError::ConflictingLogicalResource);
        }
        let exact_length = source.opened.observed_exact_length();
        self.budget.reserve(ResourceReadKind::Font, exact_length)?;
        self.attempted_fonts.insert(font_face_id);
        let expected_read = source.opened.read_identity().clone();
        let permit = roots
            .issue_bounded_read_permit(source.opened)
            .map_err(map_host_error)?;
        let receipt = roots.read_bounded(permit).map_err(map_host_error)?;
        let stable = roots
            .accept_receipt(&expected_read, receipt)
            .map_err(map_host_error)?;
        let (bytes, sha256) = stable.into_bytes_and_sha256();
        Ok(PendingResourceBytes {
            session: self.session.clone(),
            id: PendingResourceId::Font(font_face_id),
            uri: declaration.uri.clone(),
            face_index: Some(declaration.face_index),
            bytes,
            sha256,
        })
    }
    pub fn read_image(
        &mut self,
        source: VerifiedResourceSource<'roots>,
    ) -> Result<PendingResourceBytes, ResourceAdmissionError> {
        let roots = self.roots.ok_or(ResourceAdmissionError::RootSetMismatch)?;
        roots
            .validate_opened(&source.opened)
            .map_err(map_host_error)?;
        let PendingResourceId::Image(image_id) = source.id else {
            return Err(ResourceAdmissionError::ReceiptKindMismatch);
        };
        let declaration = self
            .declarations
            .images
            .get(image_id.get() as usize)
            .filter(|candidate| candidate.image_id == image_id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        if self.attempted_images.contains(&image_id) {
            return Err(ResourceAdmissionError::ConflictingLogicalResource);
        }
        let exact_length = source.opened.observed_exact_length();
        self.budget.reserve(ResourceReadKind::Image, exact_length)?;
        self.attempted_images.insert(image_id);
        let expected_read = source.opened.read_identity().clone();
        let permit = roots
            .issue_bounded_read_permit(source.opened)
            .map_err(map_host_error)?;
        let receipt = roots.read_bounded(permit).map_err(map_host_error)?;
        let stable = roots
            .accept_receipt(&expected_read, receipt)
            .map_err(map_host_error)?;
        let (bytes, sha256) = stable.into_bytes_and_sha256();
        Ok(PendingResourceBytes {
            session: self.session.clone(),
            id: PendingResourceId::Image(image_id),
            uri: declaration.uri.clone(),
            face_index: None,
            bytes,
            sha256,
        })
    }

    pub fn read_font_with_subject(
        &mut self,
        source: VerifiedResourceSource<'roots>,
    ) -> Result<PendingResourceBytes, ResourceAdmissionFailureOutcome> {
        let subject = match source.id {
            PendingResourceId::Font(id) => ResourceErrorSubject::FontFace(id),
            PendingResourceId::Image(id) => ResourceErrorSubject::Image(id),
        };
        self.read_font(source)
            .map_err(|error| self.failure_outcome(ResourceAdmissionFailure::new(error, subject)))
    }

    pub fn read_image_with_subject(
        &mut self,
        source: VerifiedResourceSource<'roots>,
    ) -> Result<PendingResourceBytes, ResourceAdmissionFailureOutcome> {
        let subject = match source.id {
            PendingResourceId::Font(id) => ResourceErrorSubject::FontFace(id),
            PendingResourceId::Image(id) => ResourceErrorSubject::Image(id),
        };
        self.read_image(source)
            .map_err(|error| self.failure_outcome(ResourceAdmissionFailure::new(error, subject)))
    }

    pub fn parse_and_bind_sfnt(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
        self.ensure_session(&source)?;
        let (units_per_em, glyph_count) = parse_sfnt_metadata(
            source.bytes(),
            source
                .face_index()
                .ok_or(ResourceAdmissionError::ReceiptKindMismatch)?,
        )?;
        let owner = VerifiedMetadataReceiptOwner::new();
        let receipt = owner.issue_font(
            source,
            AdmittedFontMetadata {
                units_per_em,
                glyph_count,
            },
        )?;
        self.bind_verified_metadata(receipt)
    }
    pub fn parse_and_bind_png(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
        self.ensure_session(&source)?;
        let (width, height, decoded_bytes) = parse_png_metadata(source.bytes())?;
        let owner = VerifiedMetadataReceiptOwner::new();
        let receipt = owner.issue_image(source, width, height, decoded_bytes)?;
        self.bind_verified_metadata(receipt)
    }

    pub fn parse_and_bind_sfnt_with_subject(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionFailureOutcome> {
        let subject = source.error_subject();
        self.parse_and_bind_sfnt(source)
            .map_err(|error| self.failure_outcome(ResourceAdmissionFailure::new(error, subject)))
    }

    pub fn parse_and_bind_png_with_subject(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionFailureOutcome> {
        let subject = source.error_subject();
        self.parse_and_bind_png(source)
            .map_err(|error| self.failure_outcome(ResourceAdmissionFailure::new(error, subject)))
    }

    pub fn bind_verified_metadata(
        &mut self,
        receipt: VerifiedMetadataReceipt,
    ) -> Result<(), ResourceAdmissionError> {
        match receipt.0 {
            VerifiedMetadata::Font { source, metadata } => {
                self.ensure_session(&source)?;
                let id = source
                    .font_face_id()
                    .ok_or(ResourceAdmissionError::ReceiptKindMismatch)?;
                let declaration = self
                    .declarations
                    .font_faces
                    .get(id.get() as usize)
                    .filter(|candidate| candidate.font_face_id == id)
                    .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
                if source.uri() != &declaration.uri
                    || source.face_index() != Some(declaration.face_index)
                {
                    return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
                }
                if declaration
                    .expected_sha256
                    .is_some_and(|expected| expected != source.content_hash())
                {
                    return Err(ResourceAdmissionError::ExpectedHashMismatch);
                }
                if self.fonts.contains_key(&id) {
                    return Err(ResourceAdmissionError::ConflictingLogicalResource);
                }
                let font = AdmittedFont::from_verified(
                    id,
                    source.uri,
                    declaration.family.clone(),
                    declaration.face_index,
                    source.bytes,
                    source.sha256,
                    metadata,
                );
                let replaced = self.fonts.insert(id, font);
                debug_assert!(replaced.is_none());
            }
            VerifiedMetadata::Image {
                source,
                width,
                height,
                decoded_bytes,
            } => {
                self.ensure_session(&source)?;
                let id = source
                    .image_id()
                    .ok_or(ResourceAdmissionError::ReceiptKindMismatch)?;
                let declaration = self
                    .declarations
                    .images
                    .get(id.get() as usize)
                    .filter(|candidate| candidate.image_id == id)
                    .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
                if source.uri() != &declaration.uri || source.face_index().is_some() {
                    return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
                }
                if declaration
                    .expected_sha256
                    .is_some_and(|expected| expected != source.content_hash())
                {
                    return Err(ResourceAdmissionError::ExpectedHashMismatch);
                }
                let pixels = u64::from(width.get())
                    .checked_mul(u64::from(height.get()))
                    .ok_or(ResourceAdmissionError::ResourceLimit)?;
                if pixels > self.budget.limits.get().max_image_pixels
                    || decoded_bytes > self.budget.limits.get().max_decoded_image_bytes
                {
                    return Err(ResourceAdmissionError::ResourceLimit);
                }
                if self.images.contains_key(&id) {
                    return Err(ResourceAdmissionError::ConflictingLogicalResource);
                }
                let image = AdmittedImage::from_verified(
                    id,
                    source.uri,
                    source.bytes,
                    source.sha256,
                    width,
                    height,
                    decoded_bytes,
                );
                let replaced = self.images.insert(id, image);
                debug_assert!(replaced.is_none());
            }
        }
        Ok(())
    }

    pub fn bind_verified_metadata_with_subject(
        &mut self,
        receipt: VerifiedMetadataReceipt,
    ) -> Result<(), ResourceAdmissionFailureOutcome> {
        let subject = match &receipt.0 {
            VerifiedMetadata::Font { source, .. } | VerifiedMetadata::Image { source, .. } => {
                source.error_subject()
            }
        };
        self.bind_verified_metadata(receipt)
            .map_err(|error| self.failure_outcome(ResourceAdmissionFailure::new(error, subject)))
    }

    /// Snapshot the last set of resources whose bytes, hash, and metadata all
    /// completed successfully in this exact resolver session.
    pub fn progress_token(&self) -> ResourceAdmissionProgressToken {
        ResourceAdmissionProgressToken {
            session: self.session.clone(),
            fonts: self.fonts.values().cloned().collect(),
            images: self.images.values().cloned().collect(),
        }
    }

    fn failure_outcome(
        &self,
        failure: ResourceAdmissionFailure,
    ) -> ResourceAdmissionFailureOutcome {
        ResourceAdmissionFailureOutcome {
            failure,
            progress: self.progress_token(),
        }
    }

    fn ensure_session(&self, source: &PendingResourceBytes) -> Result<(), ResourceAdmissionError> {
        if self.session == source.session {
            Ok(())
        } else {
            Err(ResourceAdmissionError::ReceiptSessionMismatch)
        }
    }
    pub fn finish(self) -> Result<AdmittedResourceLedger, ResourceAdmissionError> {
        if self.fonts.len() != self.declarations.font_faces.len()
            || self.images.len() != self.declarations.images.len()
        {
            return Err(ResourceAdmissionError::MissingLogicalResource);
        }
        let font_families = FontFamilyTable::new(
            self.declarations
                .font_faces
                .iter()
                .map(|declaration| (declaration.family.clone(), declaration.font_face_id))
                .collect(),
        )
        .map_err(map_font_family_error)?;
        Ok(AdmittedResourceLedger {
            session: self.session,
            fonts: self.fonts.into_values().collect(),
            images: self.images.into_values().collect(),
            font_families,
        })
    }
}

fn parse_sfnt_metadata(
    bytes: &[u8],
    face_index: u32,
) -> Result<(u16, u32), ResourceAdmissionError> {
    let directory_offset = if bytes.get(..4) == Some(b"ttcf") {
        let count = read_be_u32(bytes, 8)?;
        if face_index >= count {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        let offset_position = 12usize
            .checked_add(
                usize::try_from(face_index)
                    .map_err(|_| ResourceAdmissionError::InvalidMetadata)?
                    .checked_mul(4)
                    .ok_or(ResourceAdmissionError::InvalidMetadata)?,
            )
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        usize::try_from(read_be_u32(bytes, offset_position)?)
            .map_err(|_| ResourceAdmissionError::InvalidMetadata)?
    } else {
        if face_index != 0 {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        0
    };
    let signature_end = directory_offset
        .checked_add(4)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    if bytes.get(directory_offset..signature_end) != Some(&0x0001_0000u32.to_be_bytes()) {
        // Profile 1.0 emits CIDFontType2 + FontFile2 and therefore admits
        // TrueType-outline sfnt faces only. OTTO/CFF needs a different PDF
        // object blueprint and must fail closed here.
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    let table_count_offset = directory_offset
        .checked_add(4)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let table_count = usize::from(read_be_u16(bytes, table_count_offset)?);
    let directory_start = directory_offset
        .checked_add(12)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let mut table_tags = BTreeSet::new();
    let mut head = None;
    let mut maxp = None;
    for index in 0..table_count {
        let record = directory_start
            .checked_add(
                index
                    .checked_mul(16)
                    .ok_or(ResourceAdmissionError::InvalidMetadata)?,
            )
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        let tag_end = record
            .checked_add(4)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        let tag: [u8; 4] = bytes
            .get(record..tag_end)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?
            .try_into()
            .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
        if !table_tags.insert(tag) {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        let offset_field = record
            .checked_add(8)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        let length_field = record
            .checked_add(12)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        let offset = usize::try_from(read_be_u32(bytes, offset_field)?)
            .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
        let length = usize::try_from(read_be_u32(bytes, length_field)?)
            .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
        let end = offset
            .checked_add(length)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?;
        if end > bytes.len() {
            return Err(ResourceAdmissionError::InvalidMetadata);
        }
        match &tag {
            b"head" if length >= 20 => head = Some(offset),
            b"maxp" if length >= 6 => maxp = Some(offset),
            b"head" | b"maxp" => return Err(ResourceAdmissionError::InvalidMetadata),
            _ => {}
        }
    }
    let units_offset = head
        .ok_or(ResourceAdmissionError::InvalidMetadata)?
        .checked_add(18)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let glyph_count_offset = maxp
        .ok_or(ResourceAdmissionError::InvalidMetadata)?
        .checked_add(4)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let units_per_em = read_be_u16(bytes, units_offset)?;
    let glyph_count = u32::from(read_be_u16(bytes, glyph_count_offset)?);
    if !(16..=16_384).contains(&units_per_em) || glyph_count == 0 {
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    Ok((units_per_em, glyph_count))
}

fn parse_png_metadata(
    bytes: &[u8],
) -> Result<(NonZeroU32, NonZeroU32, u64), ResourceAdmissionError> {
    if bytes.get(..8) != Some(b"\x89PNG\r\n\x1a\n")
        || read_be_u32(bytes, 8)? != 13
        || bytes.get(12..16) != Some(b"IHDR")
    {
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    let width =
        NonZeroU32::new(read_be_u32(bytes, 16)?).ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let height =
        NonZeroU32::new(read_be_u32(bytes, 20)?).ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let bit_depth = *bytes
        .get(24)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let color_type = *bytes
        .get(25)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let legal_depth = match color_type {
        0 => matches!(bit_depth, 1 | 2 | 4 | 8 | 16),
        2 => matches!(bit_depth, 8 | 16),
        3 => matches!(bit_depth, 1 | 2 | 4 | 8),
        4 | 6 => matches!(bit_depth, 8 | 16),
        _ => return Err(ResourceAdmissionError::InvalidMetadata),
    };
    if !legal_depth
        || bytes.get(26) != Some(&0)
        || bytes.get(27) != Some(&0)
        || !matches!(bytes.get(28), Some(0) | Some(1))
    {
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    // The admission budget measures the canonical decoded pixel buffer, not
    // packed scanline bytes. Formats that may carry tRNS reserve an alpha
    // channel even when a particular file omits it; palette input is RGBA8;
    // 16-bit samples remain two bytes/sample.
    let decoded_bytes_per_pixel = match (color_type, bit_depth) {
        (0, 16) => 4,
        (0, _) => 2,
        (2, 16) => 8,
        (2, _) => 4,
        (3, _) => 4,
        (4, 16) => 4,
        (4, _) => 2,
        (6, 16) => 8,
        (6, _) => 4,
        _ => return Err(ResourceAdmissionError::InvalidMetadata),
    };
    let decoded_bytes = u64::from(width.get())
        .checked_mul(u64::from(height.get()))
        .and_then(|value| value.checked_mul(decoded_bytes_per_pixel))
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    Ok((width, height, decoded_bytes))
}

fn read_be_u16(bytes: &[u8], offset: usize) -> Result<u16, ResourceAdmissionError> {
    let end = offset
        .checked_add(2)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let encoded: [u8; 2] = bytes
        .get(offset..end)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?
        .try_into()
        .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
    Ok(u16::from_be_bytes(encoded))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, ResourceAdmissionError> {
    let end = offset
        .checked_add(4)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let encoded: [u8; 4] = bytes
        .get(offset..end)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?
        .try_into()
        .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
    Ok(u32::from_be_bytes(encoded))
}

fn validate_declaration_order(
    declarations: &ResourceCatalog,
) -> Result<(), ResourceAdmissionError> {
    for (index, declaration) in declarations.font_faces.iter().enumerate() {
        if declaration.font_face_id.get()
            != u32::try_from(index).map_err(|_| ResourceAdmissionError::NonCanonicalResourceId)?
        {
            return Err(ResourceAdmissionError::NonCanonicalResourceId);
        }
    }
    for (index, declaration) in declarations.images.iter().enumerate() {
        if declaration.image_id.get()
            != u32::try_from(index).map_err(|_| ResourceAdmissionError::NonCanonicalResourceId)?
        {
            return Err(ResourceAdmissionError::NonCanonicalResourceId);
        }
    }
    FontFamilyTable::new(
        declarations
            .font_faces
            .iter()
            .map(|declaration| (declaration.family.clone(), declaration.font_face_id))
            .collect(),
    )
    .map_err(map_font_family_error)?;
    Ok(())
}

fn map_font_family_error(_error: FontFamilyError) -> ResourceAdmissionError {
    ResourceAdmissionError::InvalidFontFamily
}

/// Immutable complete-set proof emitted by `AdmittedResourceResolver`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedResourceLedger {
    session: ResourceAdmissionSessionIdentity,
    fonts: Vec<AdmittedFont>,
    images: Vec<AdmittedImage>,
    font_families: FontFamilyTable,
}
impl AdmittedResourceLedger {
    pub fn fonts(&self) -> &[AdmittedFont] {
        &self.fonts
    }
    pub fn images(&self) -> &[AdmittedImage] {
        &self.images
    }
    pub fn font(&self, id: FontFaceId) -> Option<&AdmittedFont> {
        self.fonts.iter().find(|font| font.font_face_id() == id)
    }
    pub fn image(&self, id: ImageResourceId) -> Option<&AdmittedImage> {
        self.images.iter().find(|image| image.image_id() == id)
    }
    pub const fn font_families(&self) -> &FontFamilyTable {
        &self.font_families
    }
    pub const fn token(&self) -> AdmittedResourceLedgerToken<'_> {
        AdmittedResourceLedgerToken { ledger: self }
    }

    pub fn progress_token(&self) -> ResourceAdmissionProgressToken {
        ResourceAdmissionProgressToken {
            session: self.session.clone(),
            fonts: self.fonts.clone(),
            images: self.images.clone(),
        }
    }
    pub fn matches_declarations(&self, declarations: &ResourceCatalog) -> bool {
        self.fonts.len() == declarations.font_faces.len()
            && self.images.len() == declarations.images.len()
            && self
                .fonts
                .iter()
                .zip(&declarations.font_faces)
                .all(|(font, declaration)| {
                    font.font_face_id() == declaration.font_face_id
                        && font.uri() == &declaration.uri
                        && font.family() == declaration.family
                        && font.face_index() == declaration.face_index
                        && declaration
                            .expected_sha256
                            .map_or(true, |expected| expected == font.content_hash())
                })
            && self
                .images
                .iter()
                .zip(&declarations.images)
                .all(|(image, declaration)| {
                    image.image_id() == declaration.image_id
                        && image.uri() == &declaration.uri
                        && declaration
                            .expected_sha256
                            .map_or(true, |expected| expected == image.content_hash())
                })
    }
    pub fn fingerprint(&self) -> AdmittedResourceFingerprint {
        let mut canonical = String::from("{\"algorithm\":");
        push_jcs_string(&mut canonical, AdmittedResourceFingerprint::ALGORITHM_ID);
        canonical.push_str(",\"fonts\":[");
        for (index, font) in self.fonts.iter().enumerate() {
            if index > 0 {
                canonical.push(',');
            }
            canonical.push_str("{\"face_index\":");
            canonical.push_str(&font.face_index().to_string());
            canonical.push_str(",\"family\":");
            push_jcs_string(&mut canonical, font.family());
            canonical.push_str(",\"font_face_id\":");
            canonical.push_str(&font.font_face_id().get().to_string());
            canonical.push_str(",\"glyph_count\":");
            canonical.push_str(&font.metadata().glyph_count.to_string());
            canonical.push_str(",\"sha256\":");
            push_hash_hex(&mut canonical, font.content_hash());
            canonical.push_str(",\"units_per_em\":");
            canonical.push_str(&font.metadata().units_per_em.to_string());
            canonical.push('}');
        }
        canonical.push_str("],\"images\":[");
        for (index, image) in self.images.iter().enumerate() {
            if index > 0 {
                canonical.push(',');
            }
            canonical.push_str("{\"decoded_bytes\":");
            canonical.push_str(&image.decoded_bytes().to_string());
            canonical.push_str(",\"image_id\":");
            canonical.push_str(&image.image_id().get().to_string());
            canonical.push_str(",\"pixel_height\":");
            canonical.push_str(&image.height().get().to_string());
            canonical.push_str(",\"pixel_width\":");
            canonical.push_str(&image.width().get().to_string());
            canonical.push_str(",\"sha256\":");
            push_hash_hex(&mut canonical, image.content_hash());
            canonical.push('}');
        }
        canonical.push_str("]}");
        admitted_resource_fingerprint_from_jcs(&canonical)
    }
}

/// Sealed, session-bound snapshot of successfully verified resources.
///
/// ```compile_fail
/// use typaxis_resource_admission::ResourceAdmissionProgressToken;
/// let _forged = ResourceAdmissionProgressToken {};
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAdmissionProgressToken {
    session: ResourceAdmissionSessionIdentity,
    fonts: Vec<AdmittedFont>,
    images: Vec<AdmittedImage>,
}

impl ResourceAdmissionProgressToken {
    pub fn fonts(&self) -> &[AdmittedFont] {
        &self.fonts
    }

    pub fn images(&self) -> &[AdmittedImage] {
        &self.images
    }

    /// Proves monotonic continuation without revealing the opaque resolver
    /// session identity.
    pub fn continues(&self, previous: &Self) -> bool {
        self.session == previous.session
            && previous
                .fonts
                .iter()
                .all(|established| self.fonts.iter().any(|incoming| incoming == established))
            && previous
                .images
                .iter()
                .all(|established| self.images.iter().any(|incoming| incoming == established))
            && (self.fonts.len() > previous.fonts.len()
                || self.images.len() > previous.images.len())
    }

    pub fn same_session(&self, other: &Self) -> bool {
        self.session == other.session
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AdmittedResourceLedgerToken<'a> {
    ledger: &'a AdmittedResourceLedger,
}
impl<'a> AdmittedResourceLedgerToken<'a> {
    pub const fn ledger(self) -> &'a AdmittedResourceLedger {
        self.ledger
    }
    pub fn fonts(self) -> &'a [AdmittedFont] {
        self.ledger.fonts()
    }
    pub fn images(self) -> &'a [AdmittedImage] {
        self.ledger.images()
    }
    pub fn fingerprint(self) -> AdmittedResourceFingerprint {
        self.ledger.fingerprint()
    }

    pub fn matches_progress(self, progress: &ResourceAdmissionProgressToken) -> bool {
        self.ledger.session == progress.session
            && self.ledger.fonts == progress.fonts
            && self.ledger.images == progress.images
    }

    pub fn same_session_as(self, progress: &ResourceAdmissionProgressToken) -> bool {
        self.ledger.session == progress.session
    }

    pub fn continues_progress(self, progress: &ResourceAdmissionProgressToken) -> bool {
        self.same_session_as(progress)
            && progress.fonts.iter().all(|established| {
                self.ledger
                    .fonts
                    .iter()
                    .any(|complete| complete == established)
            })
            && progress.images.iter().all(|established| {
                self.ledger
                    .images
                    .iter()
                    .any(|complete| complete == established)
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmittedFontInstance {
    font_instance_id: typaxis_core::FontInstanceId,
    font_face_id: FontFaceId,
    admitted_sha256: [u8; 32],
}
impl AdmittedFontInstance {
    pub const fn font_instance_id(self) -> typaxis_core::FontInstanceId {
        self.font_instance_id
    }
    pub const fn font_face_id(self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn admitted_sha256(self) -> [u8; 32] {
        self.admitted_sha256
    }
}

/// Canonical dense instance IDs derived from a selected set of faces in one
/// immutable admitted ledger. Caller order and worker completion order cannot
/// influence the assigned IDs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedFontInstanceTable {
    ledger_fingerprint: AdmittedResourceFingerprint,
    instances: Vec<AdmittedFontInstance>,
}
impl AdmittedFontInstanceTable {
    pub fn from_used_faces(
        ledger: &AdmittedResourceLedger,
        used_faces: impl IntoIterator<Item = FontFaceId>,
    ) -> Result<Self, ResourceAdmissionError> {
        let used_faces: BTreeSet<_> = used_faces.into_iter().collect();
        let mut keyed = Vec::new();
        keyed
            .try_reserve_exact(used_faces.len())
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        for font_face_id in used_faces {
            let font = ledger
                .font(font_face_id)
                .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
            keyed.push((font_face_id, font.content_hash()));
        }
        keyed.sort_unstable();
        let mut instances = Vec::new();
        instances
            .try_reserve_exact(keyed.len())
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        for (index, (font_face_id, admitted_sha256)) in keyed.into_iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| ResourceAdmissionError::ResourceLimit)?;
            instances.push(AdmittedFontInstance {
                font_instance_id: typaxis_core::FontInstanceId::new(index),
                font_face_id,
                admitted_sha256,
            });
        }
        Ok(Self {
            ledger_fingerprint: ledger.fingerprint(),
            instances,
        })
    }
    pub fn instances(&self) -> &[AdmittedFontInstance] {
        &self.instances
    }
    pub const fn ledger_fingerprint(&self) -> AdmittedResourceFingerprint {
        self.ledger_fingerprint
    }
    pub fn get(&self, id: typaxis_core::FontInstanceId) -> Option<&AdmittedFontInstance> {
        self.instances
            .get(id.get() as usize)
            .filter(|instance| instance.font_instance_id == id)
    }
    pub fn resolve<'a>(
        &'a self,
        id: typaxis_core::FontInstanceId,
        ledger: &'a AdmittedResourceLedger,
    ) -> Option<AdmittedFontInstanceRef<'a>> {
        if ledger.fingerprint() != self.ledger_fingerprint {
            return None;
        }
        let instance = self.get(id)?;
        let font = ledger.font(instance.font_face_id)?;
        if font.content_hash() != instance.admitted_sha256 {
            return None;
        }
        Some(AdmittedFontInstanceRef {
            ledger_fingerprint: self.ledger_fingerprint,
            instance,
            font,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmittedFontInstanceRef<'a> {
    ledger_fingerprint: AdmittedResourceFingerprint,
    instance: &'a AdmittedFontInstance,
    font: &'a AdmittedFont,
}
impl<'a> AdmittedFontInstanceRef<'a> {
    pub const fn ledger_fingerprint(self) -> AdmittedResourceFingerprint {
        self.ledger_fingerprint
    }
    pub const fn font_instance_id(self) -> typaxis_core::FontInstanceId {
        self.instance.font_instance_id
    }
    pub const fn font_face_id(self) -> FontFaceId {
        self.instance.font_face_id
    }
    pub const fn admitted_sha256(self) -> [u8; 32] {
        self.instance.admitted_sha256
    }
    pub fn font_bytes(self) -> &'a [u8] {
        self.font.bytes()
    }
    pub const fn face_index(self) -> u32 {
        self.font.face_index()
    }
    pub const fn metadata(self) -> &'a AdmittedFontMetadata {
        self.font.metadata()
    }
}

/// Compatibility name for read-only consumers; no public constructor exists.
pub type AdmittedResources = AdmittedResourceLedger;

fn push_hash_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use typaxis_core::{
        sha256, ConfigResourceRoot, EffectiveDataVersions, HostPath, PdfStreamCompression,
        ResourceLimits, DEFAULT_ALLOWED_URI_SCHEMES, REGISTERED_JAPANESE_LINE_BREAK_VERSION,
        REGISTERED_UNICODE_VERSION,
    };
    use typaxis_document::{FontFaceDeclaration, ImageDeclaration};

    fn limits(overrides: ResourceLimits) -> ValidatedResourceLimits {
        ValidatedResourceLimits::new(overrides).unwrap()
    }

    #[test]
    fn typed_resource_error_subject_mapping_is_canonical() {
        let subject = ResourceErrorSubject::FontFace(FontFaceId::new(4));
        let failure =
            ResourceAdmissionFailure::new(ResourceAdmissionError::InvalidMetadata, subject.clone());
        assert_eq!(failure.error(), ResourceAdmissionError::InvalidMetadata);
        assert_eq!(failure.subject(), &subject);
        assert_eq!(
            failure.diagnostic_subject(),
            DiagnosticSubject::Resource(subject.clone())
        );
        assert_eq!(
            failure.canonical_message(),
            "resource format or metadata is unsupported"
        );
        let public = failure.public_error().unwrap();
        assert_eq!(public.code(), typaxis_diagnostics::R7100);
        assert_eq!(public.subject(), Some(DiagnosticSubject::Resource(subject)));
    }

    struct TempTree {
        path: PathBuf,
    }

    impl TempTree {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "typaxis-resource-admission-{}-{label}-{}",
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

    fn effective_config(resource_roots: Vec<ConfigResourceRoot>) -> EffectiveConfig {
        EffectiveConfig::new(
            false,
            PdfStreamCompression::Flate,
            resource_roots,
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

    fn host_context(project_root: &Path, cli_roots: &[&Path]) -> HostAdmissionContext {
        HostAdmissionContext::new(
            HostPath::new(project_root.join("input.typ")).unwrap(),
            HostPath::new(project_root.to_path_buf()).unwrap(),
            None,
            cli_roots
                .iter()
                .map(|path| HostPath::new((*path).to_path_buf()).unwrap())
                .collect(),
        )
    }

    fn font_catalog(count: u32) -> ResourceCatalog {
        ResourceCatalog {
            font_faces: (0..count)
                .map(|id| FontFaceDeclaration {
                    font_face_id: FontFaceId::new(id),
                    family: format!("font-{id}"),
                    uri: PortablePath::new(format!("font-{id}.ttf")).unwrap(),
                    face_index: 0,
                    expected_sha256: None,
                })
                .collect(),
            images: vec![],
        }
    }

    fn png_header(
        width: u32,
        height: u32,
        bit_depth: u8,
        color_type: u8,
        compression: u8,
        filter: u8,
        interlace: u8,
    ) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[bit_depth, color_type, compression, filter, interlace]);
        bytes
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        png_header(width, height, 8, 6, 0, 0, 0)
    }

    fn sfnt_with_units_per_em(units_per_em: u16) -> Vec<u8> {
        let mut bytes = vec![0; 70];
        bytes[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        bytes[4..6].copy_from_slice(&2u16.to_be_bytes());
        bytes[12..16].copy_from_slice(b"head");
        bytes[20..24].copy_from_slice(&44u32.to_be_bytes());
        bytes[24..28].copy_from_slice(&20u32.to_be_bytes());
        bytes[28..32].copy_from_slice(b"maxp");
        bytes[36..40].copy_from_slice(&64u32.to_be_bytes());
        bytes[40..44].copy_from_slice(&6u32.to_be_bytes());
        bytes[62..64].copy_from_slice(&units_per_em.to_be_bytes());
        bytes[68..70].copy_from_slice(&3u16.to_be_bytes());
        bytes
    }

    fn sfnt() -> Vec<u8> {
        sfnt_with_units_per_em(1000)
    }

    #[test]
    fn generic_host_failures_keep_the_resource_error_contract() {
        let cases = [
            (
                HostAdmissionError::HostLimit,
                ResourceAdmissionError::ResourceLimit,
            ),
            (
                HostAdmissionError::ReadCapacity,
                ResourceAdmissionError::ResourceLimit,
            ),
            (
                HostAdmissionError::Read,
                ResourceAdmissionError::ResourceRead,
            ),
            (
                HostAdmissionError::LengthMismatch,
                ResourceAdmissionError::ResourceLengthMismatch,
            ),
            (
                HostAdmissionError::SessionMismatch,
                ResourceAdmissionError::RootSetMismatch,
            ),
            (
                HostAdmissionError::RootSetMismatch,
                ResourceAdmissionError::RootSetMismatch,
            ),
            (
                HostAdmissionError::ReadIdentityMismatch,
                ResourceAdmissionError::ReceiptIdentityMismatch,
            ),
            (
                HostAdmissionError::RootUnavailable,
                ResourceAdmissionError::RootUnavailable,
            ),
            (
                HostAdmissionError::RootNotDirectory,
                ResourceAdmissionError::RootNotDirectory,
            ),
            (
                HostAdmissionError::AliasedRoot,
                ResourceAdmissionError::AliasedRoot,
            ),
            (
                HostAdmissionError::UnsupportedContainedOpen,
                ResourceAdmissionError::UnsupportedContainedOpen,
            ),
            (
                HostAdmissionError::MissingCandidate,
                ResourceAdmissionError::MissingResourceCandidate,
            ),
            (
                HostAdmissionError::AmbiguousCandidate,
                ResourceAdmissionError::AmbiguousResourceCandidate,
            ),
            (
                HostAdmissionError::UnsafeCandidate,
                ResourceAdmissionError::UnsafeResourceCandidate,
            ),
            (
                HostAdmissionError::NotRegularFile,
                ResourceAdmissionError::ResourceNotRegularFile,
            ),
            (
                HostAdmissionError::LockUnavailable,
                ResourceAdmissionError::ResourceLockUnavailable,
            ),
        ];

        for (host, resource) in cases {
            assert_eq!(map_host_error(host), resource);
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
    #[test]
    fn unsupported_platform_fails_without_issuing_root_file_or_metadata_receipts() {
        let tree = TempTree::new("unsupported");
        let catalog = font_catalog(1);
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let context = host_context(tree.path(), &[]);

        assert!(matches!(
            HostResourceAdmissionSession::new(&context, &config, &catalog),
            Err(ResourceAdmissionError::UnsupportedContainedOpen)
        ));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn host_session_opens_declared_file_and_binds_same_root_set() {
        let tree = TempTree::new("read");
        fs::create_dir(tree.path().join("fonts")).unwrap();
        fs::write(tree.path().join("fonts/body.ttf"), sfnt()).unwrap();
        let catalog = ResourceCatalog {
            font_faces: vec![FontFaceDeclaration {
                font_face_id: FontFaceId::new(0),
                family: "Body".to_owned(),
                uri: PortablePath::new("fonts/body.ttf").unwrap(),
                face_index: 0,
                expected_sha256: Some(sha256(&sfnt())),
            }],
            images: vec![],
        };
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let context = host_context(tree.path(), &[]);
        let session = HostResourceAdmissionSession::new(&context, &config, &catalog).unwrap();
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, config.limits(), session.roots())
                .unwrap();
        let pending = resolver
            .read_font(session.open_font(FontFaceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_sfnt(pending).unwrap();
        let ledger = resolver.finish().unwrap();
        assert_eq!(ledger.font(FontFaceId::new(0)).unwrap().bytes(), sfnt());
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn progress_advances_only_after_metadata_success_and_failure_returns_last_token() {
        let tree = TempTree::new("progress-outcome");
        fs::write(tree.path().join("font-0.ttf"), sfnt()).unwrap();
        fs::write(tree.path().join("font-1.ttf"), sfnt()).unwrap();
        let catalog = font_catalog(2);
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, config.limits(), host.roots())
                .unwrap();

        let initial = resolver.progress_token();
        assert!(initial.fonts().is_empty());
        let first = resolver
            .read_font(host.open_font(FontFaceId::new(0)).unwrap())
            .unwrap();
        assert!(resolver.progress_token().fonts().is_empty());
        resolver.parse_and_bind_sfnt(first).unwrap();
        let admitted_first = resolver.progress_token();
        assert!(admitted_first.continues(&initial));
        assert_eq!(admitted_first.fonts().len(), 1);

        let mut foreign =
            AdmittedResourceResolver::new_with_roots(&catalog, config.limits(), host.roots())
                .unwrap();
        let foreign_pending = foreign
            .read_font(host.open_font(FontFaceId::new(1)).unwrap())
            .unwrap();
        let outcome = resolver
            .parse_and_bind_sfnt_with_subject(foreign_pending)
            .unwrap_err();
        assert_eq!(
            outcome.failure().error(),
            ResourceAdmissionError::ReceiptSessionMismatch
        );
        assert_eq!(outcome.progress(), &admitted_first);
        assert_eq!(resolver.progress_token(), admitted_first);

        let second = resolver
            .read_font(host.open_font(FontFaceId::new(1)).unwrap())
            .unwrap();
        resolver.parse_and_bind_sfnt(second).unwrap();
        let complete_progress = resolver.progress_token();
        assert!(complete_progress.continues(&admitted_first));
        let ledger = resolver.finish().unwrap();
        assert!(ledger.token().matches_progress(&complete_progress));
        assert!(ledger.token().continues_progress(&admitted_first));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn resource_candidates_join_the_existing_command_read_ledger() {
        let tree = TempTree::new("shared-ledger");
        fs::write(tree.path().join("font-0.ttf"), sfnt()).unwrap();
        let package = HostAdmissionSession::new_contained_root(
            &HostPath::new(tree.path().to_path_buf()).unwrap(),
        )
        .unwrap();
        package
            .roots()
            .register_candidates(std::iter::once(&PortablePath::new("package.json").unwrap()))
            .unwrap();
        let catalog = font_catalog(1);
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let resources = HostResourceAdmissionSession::new_with_read_ledger(
            &host_context(tree.path(), &[]),
            &config,
            &catalog,
            package.read_ledger(),
        )
        .unwrap();
        let token = resources.roots().read_ledger_token().unwrap();
        assert_eq!(token.candidate_attempt_count(), 2);
        assert_eq!(token.stored_candidate_identity_count(), 2);
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn root_alias_and_multi_root_candidates_are_rejected() {
        let first = TempTree::new("first");
        let second = TempTree::new("second");
        fs::write(first.path().join("font-0.ttf"), sfnt()).unwrap();
        fs::write(second.path().join("font-0.ttf"), sfnt()).unwrap();
        let catalog = font_catalog(1);
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);

        let aliased = host_context(first.path(), &[first.path()]);
        assert_eq!(
            HostResourceAdmissionSession::new(&aliased, &config, &catalog).unwrap_err(),
            ResourceAdmissionError::AliasedRoot
        );

        let ambiguous = host_context(first.path(), &[second.path()]);
        let session = HostResourceAdmissionSession::new(&ambiguous, &config, &catalog).unwrap();
        assert!(matches!(
            session.open_font(FontFaceId::new(0)),
            Err(ResourceAdmissionError::AmbiguousResourceCandidate)
        ));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn resource_session_reserves_the_complete_candidate_product_before_any_file_open() {
        use typaxis_host_admission::{MAX_HOST_READ_CANDIDATES, MAX_RESOURCE_ROOTS};

        let tree = TempTree::new("candidate-product");
        let mut extra_paths = Vec::new();
        for index in 0..(MAX_RESOURCE_ROOTS - 1) {
            let path = tree.path().join(format!("root-{index}"));
            fs::create_dir(&path).unwrap();
            extra_paths.push(path);
        }
        let extra_refs: Vec<&Path> = extra_paths.iter().map(PathBuf::as_path).collect();
        let context = host_context(tree.path(), &extra_refs);
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let declarations_at_max = MAX_HOST_READ_CANDIDATES / MAX_RESOURCE_ROOTS;
        let catalog = ResourceCatalog {
            font_faces: (0..declarations_at_max)
                .map(|index| FontFaceDeclaration {
                    font_face_id: FontFaceId::new(u32::try_from(index).unwrap()),
                    family: format!("family-{index}"),
                    uri: PortablePath::new("missing.ttf").unwrap(),
                    face_index: 0,
                    expected_sha256: None,
                })
                .collect(),
            images: vec![],
        };
        let session = HostResourceAdmissionSession::new(&context, &config, &catalog).unwrap();
        let token = session.roots().read_ledger_token().unwrap();
        assert_eq!(token.candidate_attempt_count(), MAX_HOST_READ_CANDIDATES);
        assert_eq!(token.stored_candidate_identity_count(), MAX_RESOURCE_ROOTS);

        let mut over_limit = catalog;
        over_limit.font_faces.push(FontFaceDeclaration {
            font_face_id: FontFaceId::new(u32::try_from(declarations_at_max).unwrap()),
            family: "family-over-limit".to_owned(),
            uri: PortablePath::new("unsafe-parent/missing.ttf").unwrap(),
            face_index: 0,
            expected_sha256: None,
        });
        assert_eq!(
            HostResourceAdmissionSession::new(&context, &config, &over_limit).unwrap_err(),
            ResourceAdmissionError::ResourceLimit
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn contained_open_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let tree = TempTree::new("contained");
        fs::write(tree.path().join("actual.ttf"), sfnt()).unwrap();
        symlink("actual.ttf", tree.path().join("font-0.ttf")).unwrap();
        let catalog = font_catalog(1);
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let context = host_context(tree.path(), &[]);
        let session = HostResourceAdmissionSession::new(&context, &config, &catalog).unwrap();
        assert!(matches!(
            session.open_font(FontFaceId::new(0)),
            Err(ResourceAdmissionError::UnsafeResourceCandidate)
        ));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn admission_reserves_aggregate_before_second_read() {
        let tree = TempTree::new("aggregate");
        fs::write(tree.path().join("font-0.ttf"), b"abc").unwrap();
        fs::write(tree.path().join("font-1.ttf"), b"def").unwrap();
        let limits = limits(ResourceLimits {
            max_font_bytes: 4,
            max_image_bytes: 4,
            max_resource_bytes: 4,
            ..ResourceLimits::default()
        });
        let catalog = font_catalog(2);
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let context = host_context(tree.path(), &[]);
        let session = HostResourceAdmissionSession::new(&context, &config, &catalog).unwrap();
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, &limits, session.roots()).unwrap();
        resolver
            .read_font(session.open_font(FontFaceId::new(0)).unwrap())
            .unwrap();
        assert_eq!(
            resolver.read_font(session.open_font(FontFaceId::new(1)).unwrap()),
            Err(ResourceAdmissionError::ResourceLimit)
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn nonempty_admission_requires_the_same_sealed_root_set() {
        let limits = limits(ResourceLimits::default());
        let catalog = font_catalog(1);
        assert_eq!(
            AdmittedResourceResolver::new(&catalog, &limits).unwrap_err(),
            ResourceAdmissionError::MissingAdmittedRootSet
        );

        let expected_tree = TempTree::new("expected-roots");
        let other_tree = TempTree::new("other-roots");
        fs::write(expected_tree.path().join("font-0.ttf"), b"abc").unwrap();
        fs::write(other_tree.path().join("font-0.ttf"), b"abc").unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let expected_session = HostResourceAdmissionSession::new(
            &host_context(expected_tree.path(), &[]),
            &config,
            &catalog,
        )
        .unwrap();
        let other_session = HostResourceAdmissionSession::new(
            &host_context(other_tree.path(), &[]),
            &config,
            &catalog,
        )
        .unwrap();
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, &limits, expected_session.roots())
                .unwrap();
        assert_eq!(
            resolver.read_font(other_session.open_font(FontFaceId::new(0)).unwrap()),
            Err(ResourceAdmissionError::RootSetMismatch)
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn resource_budget_accepts_exact_extent_and_rejects_max_plus_one() {
        let tree = TempTree::new("exact-max-plus-one");
        fs::write(tree.path().join("font-0.ttf"), b"abc").unwrap();
        fs::write(tree.path().join("font-1.ttf"), b"abcd").unwrap();
        let catalog = font_catalog(2);
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let session =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let limits = limits(ResourceLimits {
            max_font_bytes: 3,
            ..ResourceLimits::default()
        });
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, &limits, session.roots()).unwrap();
        let exact = resolver
            .read_font(session.open_font(FontFaceId::new(0)).unwrap())
            .unwrap();
        assert_eq!(exact.bytes(), b"abc");
        assert_eq!(
            resolver.read_font(session.open_font(FontFaceId::new(1)).unwrap()),
            Err(ResourceAdmissionError::ResourceLimit)
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn png_metadata_is_derived_from_admitted_bytes() {
        let bytes = png(2, 3);
        let catalog = ResourceCatalog {
            font_faces: vec![],
            images: vec![ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("image.png").unwrap(),
                expected_sha256: Some(sha256(&bytes)),
            }],
        };
        let limits = limits(ResourceLimits::default());
        let tree = TempTree::new("png-metadata");
        fs::write(tree.path().join("image.png"), &bytes).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let session =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, &limits, session.roots()).unwrap();
        let pending = resolver
            .read_image(session.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_png(pending).unwrap();
        let ledger = resolver.finish().unwrap();
        let image = ledger.image(ImageResourceId::new(0)).unwrap();
        assert_eq!((image.width().get(), image.height().get()), (2, 3));
        assert_eq!(image.decoded_bytes(), 24);
        assert!(ledger.matches_declarations(&catalog));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn png_decoded_budget_uses_canonical_expanded_pixels() {
        assert_eq!(
            parse_png_metadata(&png_header(2, 3, 1, 0, 0, 0, 0))
                .unwrap()
                .2,
            12
        );
        assert_eq!(
            parse_png_metadata(&png_header(2, 3, 8, 2, 0, 0, 0))
                .unwrap()
                .2,
            24
        );
        assert_eq!(
            parse_png_metadata(&png_header(2, 3, 1, 3, 0, 0, 0))
                .unwrap()
                .2,
            24
        );
        assert_eq!(
            parse_png_metadata(&png_header(2, 3, 16, 6, 0, 0, 1))
                .unwrap()
                .2,
            48
        );
        for invalid in [
            png_header(1, 1, 4, 2, 0, 0, 0),
            png_header(1, 1, 16, 3, 0, 0, 0),
            png_header(1, 1, 8, 6, 1, 0, 0),
            png_header(1, 1, 8, 6, 0, 1, 0),
            png_header(1, 1, 8, 6, 0, 0, 2),
        ] {
            assert_eq!(
                parse_png_metadata(&invalid),
                Err(ResourceAdmissionError::InvalidMetadata)
            );
        }

        let bytes = png(2, 3);
        let catalog = ResourceCatalog {
            font_faces: vec![],
            images: vec![ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("image.png").unwrap(),
                expected_sha256: Some(sha256(&bytes)),
            }],
        };
        let tree = TempTree::new("png-decoded-budget");
        fs::write(tree.path().join("image.png"), &bytes).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let session =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        for (max_decoded_image_bytes, expected) in [
            (24, Ok(())),
            (23, Err(ResourceAdmissionError::ResourceLimit)),
        ] {
            let limits = limits(ResourceLimits {
                max_decoded_image_bytes,
                ..ResourceLimits::default()
            });
            let mut resolver =
                AdmittedResourceResolver::new_with_roots(&catalog, &limits, session.roots())
                    .unwrap();
            let pending = resolver
                .read_image(session.open_image(ImageResourceId::new(0)).unwrap())
                .unwrap();
            assert_eq!(resolver.parse_and_bind_png(pending), expected);
        }
    }

    #[test]
    fn cidfont_type2_admission_rejects_cff_and_invalid_units_per_em() {
        assert!(parse_sfnt_metadata(&sfnt_with_units_per_em(16), 0).is_ok());
        assert!(parse_sfnt_metadata(&sfnt_with_units_per_em(16_384), 0).is_ok());
        assert_eq!(
            parse_sfnt_metadata(&sfnt_with_units_per_em(15), 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
        assert_eq!(
            parse_sfnt_metadata(&sfnt_with_units_per_em(16_385), 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
        let mut cff = sfnt();
        cff[..4].copy_from_slice(b"OTTO");
        assert_eq!(
            parse_sfnt_metadata(&cff, 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
        let mut duplicate_head = sfnt();
        duplicate_head[28..32].copy_from_slice(b"head");
        assert_eq!(
            parse_sfnt_metadata(&duplicate_head, 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );

        let mut duplicate_optional = vec![0; 104];
        duplicate_optional[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        duplicate_optional[4..6].copy_from_slice(&4u16.to_be_bytes());
        for (record, tag, offset, length) in [
            (12usize, b"head", 76u32, 20u32),
            (28, b"maxp", 96, 6),
            (44, b"name", 102, 1),
            (60, b"name", 103, 1),
        ] {
            duplicate_optional[record..record + 4].copy_from_slice(tag);
            duplicate_optional[record + 8..record + 12].copy_from_slice(&offset.to_be_bytes());
            duplicate_optional[record + 12..record + 16].copy_from_slice(&length.to_be_bytes());
        }
        duplicate_optional[94..96].copy_from_slice(&1_000u16.to_be_bytes());
        duplicate_optional[100..102].copy_from_slice(&3u16.to_be_bytes());
        assert_eq!(
            parse_sfnt_metadata(&duplicate_optional, 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
    }

    #[test]
    fn verified_font_metadata_receipt_rechecks_the_profile_units_range() {
        let owner = VerifiedMetadataReceiptOwner::new();
        let source = |units_per_em| {
            owner.issue_font(
                PendingResourceBytes {
                    session: ResourceAdmissionSessionIdentity::fresh(),
                    id: PendingResourceId::Font(FontFaceId::new(0)),
                    uri: PortablePath::new("font.ttf").unwrap(),
                    face_index: Some(0),
                    bytes: vec![1],
                    sha256: [2; 32],
                },
                AdmittedFontMetadata {
                    units_per_em,
                    glyph_count: 1,
                },
            )
        };
        assert!(source(16).is_ok());
        assert!(source(16_384).is_ok());
        assert_eq!(source(15), Err(ResourceAdmissionError::InvalidMetadata));
        assert_eq!(source(16_385), Err(ResourceAdmissionError::InvalidMetadata));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn pending_bytes_cannot_bypass_another_resolvers_budget_session() {
        let bytes = sfnt();
        let catalog = font_catalog(1);
        let tree = TempTree::new("resolver-session");
        fs::write(tree.path().join("font-0.ttf"), &bytes).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let permissive_limits = limits(ResourceLimits::default());
        let mut issuing =
            AdmittedResourceResolver::new_with_roots(&catalog, &permissive_limits, host.roots())
                .unwrap();
        let pending = issuing
            .read_font(host.open_font(FontFaceId::new(0)).unwrap())
            .unwrap();

        let strict_limits = limits(ResourceLimits {
            max_font_bytes: 1,
            ..ResourceLimits::default()
        });
        let mut foreign =
            AdmittedResourceResolver::new_with_roots(&catalog, &strict_limits, host.roots())
                .unwrap();
        assert_eq!(
            foreign.parse_and_bind_sfnt(pending),
            Err(ResourceAdmissionError::ReceiptSessionMismatch)
        );
        assert_eq!(
            foreign.finish(),
            Err(ResourceAdmissionError::MissingLogicalResource)
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn font_instance_identity_is_ledger_issued_and_dense() {
        let bytes = sfnt();
        let mut catalog = font_catalog(1);
        catalog.font_faces[0].expected_sha256 = Some(sha256(&bytes));
        let limits = limits(ResourceLimits::default());
        let tree = TempTree::new("font-instance");
        fs::write(tree.path().join("font-0.ttf"), &bytes).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let mut resolver =
            AdmittedResourceResolver::new_with_roots(&catalog, &limits, host.roots()).unwrap();
        let pending = resolver
            .read_font(host.open_font(FontFaceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_sfnt(pending).unwrap();
        let ledger = resolver.finish().unwrap();
        let table =
            AdmittedFontInstanceTable::from_used_faces(&ledger, [FontFaceId::new(0)]).unwrap();
        let instance = table
            .resolve(typaxis_core::FontInstanceId::new(0), &ledger)
            .unwrap();
        assert_eq!(instance.font_face_id(), FontFaceId::new(0));
        assert_eq!(instance.ledger_fingerprint(), ledger.fingerprint());
        assert_eq!(instance.font_bytes(), bytes);
        assert_eq!(instance.admitted_sha256(), sha256(&bytes));
        assert_eq!(instance.metadata().units_per_em, 1000);
        assert_eq!(instance.metadata().glyph_count, 3);
    }
}
