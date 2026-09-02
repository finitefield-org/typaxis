#![forbid(unsafe_code)]

mod safe_vector;

pub use safe_vector::{
    SafeVectorAlpha, SafeVectorClipDefinition, SafeVectorClipUse, SafeVectorDraw, SafeVectorDrawV2,
    SafeVectorFillRule, SafeVectorIr, SafeVectorIrV2, SafeVectorLineCap, SafeVectorLineJoin,
    SafeVectorPaint, SafeVectorPaintLayer, SafeVectorParserProfile, SafeVectorPath,
    SafeVectorPoint, SafeVectorSegment, SafeVectorStroke, SafeVectorStrokeV2, SafeVectorTransform,
    SAFE_SVG_PARSER_ID, SAFE_SVG_PARSER_ID_V2, SAFE_VECTOR_ALLOCATION_CHARGE_ID,
    SAFE_VECTOR_ALLOCATION_CHARGE_ID_V2, SAFE_VECTOR_IR_FINGERPRINT_ID,
    SAFE_VECTOR_IR_FINGERPRINT_ID_V2, SAFE_VECTOR_IR_ID, SAFE_VECTOR_IR_ID_V2,
};

use core::num::NonZeroU32;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use typaxis_core::{
    admitted_resource_fingerprint_from_jcs, push_jcs_string, sha256, AdmittedResourceFingerprint,
    EffectiveConfig, FontFaceId, HostAdmissionContext, ImageResourceId, M4EffectiveResourceLimits,
    PortablePath, PositiveLength, ValidatedResourceLimits,
};
use typaxis_diagnostics::{DiagnosticSubject, PublicMachineError, ResourceErrorSubject};
use typaxis_document::{
    FontFaceDeclaration, FontMediaDeclaration, FontMediaType, ImageDeclaration,
    ImageMediaDeclaration, ImageMediaType, ResourceCatalog, StagingM4ResourceCatalog,
};
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmittedFontMediaKind {
    SfntTrueTypeGlyf,
    TtcTrueTypeGlyf,
}

impl AdmittedFontMediaKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SfntTrueTypeGlyf => "sfnt-truetype-glyf",
            Self::TtcTrueTypeGlyf => "ttc-truetype-glyf",
        }
    }
}

/// Media attestation issued by a crate-owned byte decoder. DocumentPackage
/// declarations intentionally carry no caller-selected media label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmittedImageMediaKind {
    Png,
    SafeVector,
    SafeVector2,
}

impl AdmittedImageMediaKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::SafeVector => "svg-safe-1",
            Self::SafeVector2 => "svg-safe-2",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmittedSafeVector {
    V1(Arc<SafeVectorIr>),
    V2(Arc<SafeVectorIrV2>),
}

impl AdmittedSafeVector {
    pub const fn parser_profile(&self) -> SafeVectorParserProfile {
        match self {
            Self::V1(_) => SafeVectorParserProfile::SafeSvg1,
            Self::V2(_) => SafeVectorParserProfile::SafeSvg2,
        }
    }

    pub const fn parser_id(&self) -> &'static str {
        self.parser_profile().parser_id()
    }

    pub const fn ir_id(&self) -> &'static str {
        self.parser_profile().ir_id()
    }

    pub const fn ir_fingerprint_id(&self) -> &'static str {
        self.parser_profile().ir_fingerprint_id()
    }

    pub const fn allocation_charge_id(&self) -> &'static str {
        self.parser_profile().allocation_charge_id()
    }

    pub fn fingerprint(&self) -> [u8; 32] {
        match self {
            Self::V1(ir) => ir.fingerprint(),
            Self::V2(ir) => ir.fingerprint(),
        }
    }

    pub fn allocation_charge(&self) -> u64 {
        match self {
            Self::V1(ir) => ir.allocation_charge(),
            Self::V2(ir) => ir.allocation_charge(),
        }
    }

    pub fn intrinsic_width(&self) -> PositiveLength {
        match self {
            Self::V1(ir) => ir.intrinsic_width(),
            Self::V2(ir) => ir.intrinsic_width(),
        }
    }

    pub fn intrinsic_height(&self) -> PositiveLength {
        match self {
            Self::V1(ir) => ir.intrinsic_height(),
            Self::V2(ir) => ir.intrinsic_height(),
        }
    }

    pub fn view_box(&self) -> [i64; 4] {
        match self {
            Self::V1(ir) => ir.view_box(),
            Self::V2(ir) => ir.view_box(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedImage {
    image_id: ImageResourceId,
    uri: PortablePath,
    bytes: Vec<u8>,
    sha256: [u8; 32],
    media_kind: AdmittedImageMediaKind,
    width: NonZeroU32,
    height: NonZeroU32,
    decoded_bytes: u64,
    safe_vector: Option<AdmittedSafeVector>,
    m4_limits_fingerprint: Option<[u8; 32]>,
    m4_profile_fingerprint: Option<[u8; 32]>,
}
impl AdmittedImage {
    #[allow(clippy::too_many_arguments)] // exact identity, bytes, media, and decoded facts
    fn from_verified(
        image_id: ImageResourceId,
        uri: PortablePath,
        bytes: Vec<u8>,
        sha256: [u8; 32],
        media_kind: AdmittedImageMediaKind,
        width: NonZeroU32,
        height: NonZeroU32,
        decoded_bytes: u64,
    ) -> Self {
        Self {
            image_id,
            uri,
            bytes,
            sha256,
            media_kind,
            width,
            height,
            decoded_bytes,
            safe_vector: None,
            m4_limits_fingerprint: None,
            m4_profile_fingerprint: None,
        }
    }
    fn from_verified_safe_vector(
        image_id: ImageResourceId,
        uri: PortablePath,
        bytes: Vec<u8>,
        sha256: [u8; 32],
        ir: SafeVectorIr,
        m4_limits_fingerprint: [u8; 32],
        m4_profile_fingerprint: [u8; 32],
    ) -> Self {
        Self {
            image_id,
            uri,
            bytes,
            sha256,
            media_kind: AdmittedImageMediaKind::SafeVector,
            // Raster dimensions are deliberately not derived from vector
            // source. Callers must use `safe_vector` intrinsic geometry.
            width: NonZeroU32::MIN,
            height: NonZeroU32::MIN,
            decoded_bytes: ir.allocation_charge(),
            safe_vector: Some(AdmittedSafeVector::V1(Arc::new(ir))),
            m4_limits_fingerprint: Some(m4_limits_fingerprint),
            m4_profile_fingerprint: Some(m4_profile_fingerprint),
        }
    }
    fn from_verified_safe_vector_v2(
        image_id: ImageResourceId,
        uri: PortablePath,
        bytes: Vec<u8>,
        sha256: [u8; 32],
        ir: SafeVectorIrV2,
        m4_limits_fingerprint: [u8; 32],
        m4_profile_fingerprint: [u8; 32],
    ) -> Self {
        Self {
            image_id,
            uri,
            bytes,
            sha256,
            media_kind: AdmittedImageMediaKind::SafeVector2,
            width: NonZeroU32::MIN,
            height: NonZeroU32::MIN,
            decoded_bytes: ir.allocation_charge(),
            safe_vector: Some(AdmittedSafeVector::V2(Arc::new(ir))),
            m4_limits_fingerprint: Some(m4_limits_fingerprint),
            m4_profile_fingerprint: Some(m4_profile_fingerprint),
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
    pub const fn media_kind(&self) -> AdmittedImageMediaKind {
        self.media_kind
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
    pub fn safe_vector(&self) -> Option<&SafeVectorIr> {
        match self.safe_vector.as_ref()? {
            AdmittedSafeVector::V1(ir) => Some(ir),
            AdmittedSafeVector::V2(_) => None,
        }
    }
    pub fn safe_vector_arc(&self) -> Option<Arc<SafeVectorIr>> {
        match self.safe_vector.as_ref()? {
            AdmittedSafeVector::V1(ir) => Some(ir.clone()),
            AdmittedSafeVector::V2(_) => None,
        }
    }
    pub fn safe_vector_v2(&self) -> Option<&SafeVectorIrV2> {
        match self.safe_vector.as_ref()? {
            AdmittedSafeVector::V1(_) => None,
            AdmittedSafeVector::V2(ir) => Some(ir),
        }
    }
    pub fn safe_vector_v2_arc(&self) -> Option<Arc<SafeVectorIrV2>> {
        match self.safe_vector.as_ref()? {
            AdmittedSafeVector::V1(_) => None,
            AdmittedSafeVector::V2(ir) => Some(ir.clone()),
        }
    }
    pub const fn admitted_safe_vector(&self) -> Option<&AdmittedSafeVector> {
        self.safe_vector.as_ref()
    }
    pub const fn m4_limits_fingerprint(&self) -> Option<[u8; 32]> {
        self.m4_limits_fingerprint
    }
    pub const fn m4_profile_fingerprint(&self) -> Option<[u8; 32]> {
        self.m4_profile_fingerprint
    }
    pub fn intrinsic_width(&self) -> Option<PositiveLength> {
        self.admitted_safe_vector()
            .map(AdmittedSafeVector::intrinsic_width)
    }
    pub fn intrinsic_height(&self) -> Option<PositiveLength> {
        self.admitted_safe_vector()
            .map(AdmittedSafeVector::intrinsic_height)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SafeVectorFailureReason {
    MalformedSvg,
    ForbiddenFeature,
    ExternalReference,
    UnsupportedFeature,
    HashMismatch,
    ResourceConflict,
}

impl SafeVectorFailureReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedSvg => "malformed_svg",
            Self::ForbiddenFeature => "forbidden_feature",
            Self::ExternalReference => "external_reference",
            Self::UnsupportedFeature => "unsupported_feature",
            Self::HashMismatch => "hash_mismatch",
            Self::ResourceConflict => "resource_conflict",
        }
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
    DeclaredMediaMismatch,
    SvgSafe2Staging,
    InvalidSafeVector,
    InvalidSafeVectorV2(SafeVectorFailureReason),
    VectorNodeLimit,
    VectorPathSegmentLimit,
    VectorNestingLimit,
    DecodedImageLimit,
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
            Self::DeclaredMediaMismatch => {
                "declared media type does not match the stable resource bytes"
            }
            Self::SvgSafe2Staging => {
                "svg-safe-2 requires the versioned precomposed-vector admission pipeline"
            }
            Self::InvalidSafeVector => "safe vector bytes contain a forbidden or invalid feature",
            Self::InvalidSafeVectorV2(SafeVectorFailureReason::MalformedSvg) => {
                "R7100 malformed_svg: Safe-SVG 2 is malformed"
            }
            Self::InvalidSafeVectorV2(SafeVectorFailureReason::ForbiddenFeature) => {
                "R7100 forbidden_feature: Safe-SVG 2 contains a forbidden feature"
            }
            Self::InvalidSafeVectorV2(SafeVectorFailureReason::ExternalReference) => {
                "R7100 external_reference: Safe-SVG 2 contains an external reference"
            }
            Self::InvalidSafeVectorV2(SafeVectorFailureReason::UnsupportedFeature) => {
                "R7100 unsupported_feature: Safe-SVG 2 contains an unsupported feature"
            }
            Self::InvalidSafeVectorV2(SafeVectorFailureReason::HashMismatch) => {
                "R7100 hash_mismatch: Safe-SVG 2 hash does not match its declaration"
            }
            Self::InvalidSafeVectorV2(SafeVectorFailureReason::ResourceConflict) => {
                "R7100 resource_conflict: equal SafeVector digests name different bytes"
            }
            Self::VectorNodeLimit => "R7120: safe vector node limit was exceeded",
            Self::VectorPathSegmentLimit => "R7121: safe vector path segment limit was exceeded",
            Self::VectorNestingLimit => "R7122: safe vector nesting limit was exceeded",
            Self::DecodedImageLimit => "R7111: decoded image allocation limit was exceeded",
        }
    }
}

impl std::fmt::Display for ResourceAdmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.canonical_message())
    }
}

impl std::error::Error for ResourceAdmissionError {}

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
            ResourceAdmissionError::InvalidMetadata
            | ResourceAdmissionError::InvalidSafeVector
            | ResourceAdmissionError::InvalidSafeVectorV2(_)
            | ResourceAdmissionError::DeclaredMediaMismatch => Some(
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

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingResourceDeclaration {
    Font(FontFaceDeclaration),
    Image(ImageDeclaration),
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
    declaration: PendingResourceDeclaration,
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
        let declaration = self
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
            declaration: PendingResourceDeclaration::Font(declaration.clone()),
            opened,
        })
    }

    pub fn open_image(
        &self,
        image_id: ImageResourceId,
    ) -> Result<VerifiedResourceSource<'_>, ResourceAdmissionError> {
        let declaration = self
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
            declaration: PendingResourceDeclaration::Image(declaration.clone()),
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
        media_kind: AdmittedImageMediaKind,
        width: NonZeroU32,
        height: NonZeroU32,
        decoded_bytes: u64,
    },
    SafeVector {
        source: PendingResourceBytes,
        ir: Box<SafeVectorIr>,
        m4_limits_fingerprint: [u8; 32],
        m4_profile_fingerprint: [u8; 32],
    },
    SafeVector2 {
        source: PendingResourceBytes,
        ir: Box<SafeVectorIrV2>,
        m4_limits_fingerprint: [u8; 32],
        m4_profile_fingerprint: [u8; 32],
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
    fn issue_png(
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
            media_kind: AdmittedImageMediaKind::Png,
            width,
            height,
            decoded_bytes,
        }))
    }
    fn issue_safe_vector(
        &self,
        source: PendingResourceBytes,
        ir: SafeVectorIr,
        m4_limits_fingerprint: [u8; 32],
        m4_profile_fingerprint: [u8; 32],
    ) -> Result<VerifiedMetadataReceipt, ResourceAdmissionError> {
        if source.image_id().is_none() || ir.draws().is_empty() || ir.allocation_charge() == 0 {
            return Err(ResourceAdmissionError::InvalidSafeVector);
        }
        Ok(VerifiedMetadataReceipt(VerifiedMetadata::SafeVector {
            source,
            ir: Box::new(ir),
            m4_limits_fingerprint,
            m4_profile_fingerprint,
        }))
    }
    fn issue_safe_vector_v2(
        &self,
        source: PendingResourceBytes,
        ir: SafeVectorIrV2,
        m4_limits_fingerprint: [u8; 32],
        m4_profile_fingerprint: [u8; 32],
    ) -> Result<VerifiedMetadataReceipt, ResourceAdmissionError> {
        if source.image_id().is_none() || ir.draws().is_empty() || ir.allocation_charge() == 0 {
            return Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::MalformedSvg,
            ));
        }
        Ok(VerifiedMetadataReceipt(VerifiedMetadata::SafeVector2 {
            source,
            ir: Box::new(ir),
            m4_limits_fingerprint,
            m4_profile_fingerprint,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeclaredMediaPolicy {
    fonts: Vec<FontMediaType>,
    images: Vec<ImageMediaType>,
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
    declared_media_policy: Option<Arc<DeclaredMediaPolicy>>,
    budget: ResourceAdmissionBudget,
    attempted_fonts: BTreeSet<FontFaceId>,
    attempted_images: BTreeSet<ImageResourceId>,
    fonts: BTreeMap<FontFaceId, AdmittedFont>,
    images: BTreeMap<ImageResourceId, AdmittedImage>,
    m4_limits: Option<M4EffectiveResourceLimits>,
    m4_profile_fingerprint: Option<[u8; 32]>,
    vector_nodes_used: u64,
    vector_path_work_used: u64,
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

    /// Contract-1.4 resolver whose media policy is sealed into the resolver
    /// before any stable bytes are read. Decode calls cannot substitute a
    /// caller-selected declaration after the read.
    pub fn new_with_declared_roots(
        declarations: &StagingDeclaredBaseCatalog,
        limits: &ValidatedResourceLimits,
        roots: HostRootSetToken<'roots>,
    ) -> Result<Self, ResourceAdmissionError> {
        let mut resolver = Self::new_inner(declarations.resource_catalog(), limits, Some(roots))?;
        resolver.declared_media_policy = Some(Arc::new(DeclaredMediaPolicy {
            fonts: declarations.font_media.clone(),
            images: declarations.image_media.clone(),
        }));
        Ok(resolver)
    }

    /// Contract-1.4 resolver with the combined, sealed base/extension limit
    /// receipt required by the SafeVector decoder.
    pub fn new_with_declared_roots_and_m4_limits(
        declarations: &StagingDeclaredBaseCatalog,
        limits: &M4EffectiveResourceLimits,
        profile_fingerprint: [u8; 32],
        roots: HostRootSetToken<'roots>,
    ) -> Result<Self, ResourceAdmissionError> {
        let mut resolver =
            Self::new_inner(declarations.resource_catalog(), limits.base(), Some(roots))?;
        resolver.declared_media_policy = Some(Arc::new(DeclaredMediaPolicy {
            fonts: declarations.font_media.clone(),
            images: declarations.image_media.clone(),
        }));
        resolver.m4_limits = Some(limits.clone());
        resolver.m4_profile_fingerprint = Some(profile_fingerprint);
        Ok(resolver)
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
            declared_media_policy: None,
            budget,
            attempted_fonts: BTreeSet::new(),
            attempted_images: BTreeSet::new(),
            fonts: BTreeMap::new(),
            images: BTreeMap::new(),
            m4_limits: None,
            m4_profile_fingerprint: None,
            vector_nodes_used: 0,
            vector_path_work_used: 0,
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
        if source.declaration != PendingResourceDeclaration::Font(declaration.clone()) {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
        }
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
        if source.declaration != PendingResourceDeclaration::Image(declaration.clone()) {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
        }
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
        if self.declared_media_policy.is_some() {
            return self.parse_and_bind_declared_sfnt(source);
        }
        self.ensure_session(&source)?;
        self.parse_and_bind_sfnt_after_policy(source)
    }

    fn parse_and_bind_sfnt_after_policy(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
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

    /// Contract-1.4 path: compare the declared container with stable bytes
    /// before metric decoding or glyph-outline evaluation, then reuse the
    /// same parser which issues the admitted media attestation.
    pub fn parse_and_bind_declared_sfnt(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
        self.ensure_session(&source)?;
        let PendingResourceId::Font(font_face_id) = source.id else {
            return Err(ResourceAdmissionError::ReceiptKindMismatch);
        };
        let declared = *self
            .declared_media_policy
            .as_ref()
            .and_then(|policy| policy.fonts.get(font_face_id.get() as usize))
            .ok_or(ResourceAdmissionError::DeclaredMediaMismatch)?;
        let observed = attest_declared_font_media_kind(
            source.bytes(),
            source
                .face_index()
                .ok_or(ResourceAdmissionError::ReceiptKindMismatch)?,
        )
        .map_err(|_| ResourceAdmissionError::DeclaredMediaMismatch)?;
        let expected = match declared {
            FontMediaType::SfntTrueTypeGlyf => AdmittedFontMediaKind::SfntTrueTypeGlyf,
            FontMediaType::TtcTrueTypeGlyf => AdmittedFontMediaKind::TtcTrueTypeGlyf,
        };
        if observed != expected {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch);
        }
        self.parse_and_bind_sfnt_after_policy(source)
    }
    pub fn parse_and_bind_png(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
        if self.declared_media_policy.is_some() {
            return self.parse_and_bind_declared_png(source);
        }
        self.ensure_session(&source)?;
        self.parse_and_bind_png_after_policy(source)
    }

    fn parse_and_bind_png_after_policy(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
        let (width, height, decoded_bytes) = parse_png_metadata(source.bytes())?;
        let pixels = u64::from(width.get())
            .checked_mul(u64::from(height.get()))
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        if pixels > self.budget.limits.get().max_image_pixels
            || decoded_bytes > self.budget.limits.get().max_decoded_image_bytes
        {
            return Err(ResourceAdmissionError::ResourceLimit);
        }
        validate_png_decoder_attestation(source.bytes(), width, height, decoded_bytes)?;
        let owner = VerifiedMetadataReceiptOwner::new();
        let receipt = owner.issue_png(source, width, height, decoded_bytes)?;
        self.bind_verified_metadata(receipt)
    }

    /// Contract-1.4 path: the signature comparison happens immediately after
    /// the stable read and before PNG decoder allocation or pixel expansion.
    pub fn parse_and_bind_declared_png(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
        self.ensure_session(&source)?;
        let PendingResourceId::Image(image_id) = source.id else {
            return Err(ResourceAdmissionError::ReceiptKindMismatch);
        };
        let declared = *self
            .declared_media_policy
            .as_ref()
            .and_then(|policy| policy.images.get(image_id.get() as usize))
            .ok_or(ResourceAdmissionError::DeclaredMediaMismatch)?;
        let observed = attest_image_media_kind(source.bytes())
            .map_err(|_| ResourceAdmissionError::DeclaredMediaMismatch)?;
        let expected = match declared {
            ImageMediaType::Png => AdmittedImageMediaKind::Png,
            ImageMediaType::SvgSafe1 => return Err(ResourceAdmissionError::DeclaredMediaMismatch),
            ImageMediaType::SvgSafe2 => return Err(ResourceAdmissionError::DeclaredMediaMismatch),
        };
        if observed != expected {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch);
        }
        self.parse_and_bind_png_after_policy(source)
    }

    /// Stable-byte-only SafeVector path. The declaration/hash/limit receipt is
    /// checked before the first IR allocation; no PNG or general-XML fallback
    /// is attempted.
    pub fn parse_and_bind_declared_safe_vector(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
        self.ensure_session(&source)?;
        let PendingResourceId::Image(image_id) = source.id else {
            return Err(ResourceAdmissionError::ReceiptKindMismatch);
        };
        let declared = *self
            .declared_media_policy
            .as_ref()
            .and_then(|policy| policy.images.get(image_id.get() as usize))
            .ok_or(ResourceAdmissionError::DeclaredMediaMismatch)?;
        let parser_profile = match declared {
            ImageMediaType::SvgSafe1 => SafeVectorParserProfile::SafeSvg1,
            ImageMediaType::SvgSafe2 => SafeVectorParserProfile::SafeSvg2,
            ImageMediaType::Png => return Err(ResourceAdmissionError::DeclaredMediaMismatch),
        };
        let declaration = self
            .declarations
            .images
            .get(image_id.get() as usize)
            .filter(|declaration| declaration.image_id == image_id)
            .ok_or(ResourceAdmissionError::MissingLogicalResource)?;
        if parser_profile == SafeVectorParserProfile::SafeSvg2
            && declaration.expected_sha256.is_none()
        {
            return Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::HashMismatch,
            ));
        }
        if declaration
            .expected_sha256
            .is_some_and(|expected| expected != source.content_hash())
        {
            return Err(match parser_profile {
                SafeVectorParserProfile::SafeSvg1 => ResourceAdmissionError::ExpectedHashMismatch,
                SafeVectorParserProfile::SafeSvg2 => ResourceAdmissionError::InvalidSafeVectorV2(
                    SafeVectorFailureReason::HashMismatch,
                ),
            });
        }
        if attest_image_media_kind(source.bytes()) == Ok(AdmittedImageMediaKind::Png) {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch);
        }
        let limits = self
            .m4_limits
            .as_ref()
            .ok_or(ResourceAdmissionError::ReceiptIdentityMismatch)?;
        if self.declared_media_policy.as_ref().is_some_and(|policy| {
            policy.images[..image_id.get() as usize]
                .iter()
                .zip(&self.declarations.images[..image_id.get() as usize])
                .any(|(media, declaration)| {
                    matches!(media, ImageMediaType::SvgSafe1 | ImageMediaType::SvgSafe2)
                        && !self.images.contains_key(&declaration.image_id)
                })
        }) {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
        }
        let remaining_nodes = limits
            .extension()
            .get()
            .max_vector_nodes
            .checked_sub(self.vector_nodes_used)
            .ok_or(ResourceAdmissionError::VectorNodeLimit)?;
        if remaining_nodes == 0 {
            return Err(ResourceAdmissionError::VectorNodeLimit);
        }
        let remaining_path_work = limits
            .extension()
            .get()
            .max_vector_path_segments
            .checked_sub(self.vector_path_work_used)
            .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
        let (work, receipt) = match parser_profile {
            SafeVectorParserProfile::SafeSvg1 => {
                let decoded = safe_vector::decode_with_work_budget(
                    source.bytes(),
                    limits,
                    remaining_nodes,
                    remaining_path_work,
                )?;
                let work = decoded.work;
                let owner = VerifiedMetadataReceiptOwner::new();
                let receipt = owner.issue_safe_vector(
                    source,
                    decoded.ir,
                    limits.fingerprint(),
                    self.m4_profile_fingerprint
                        .ok_or(ResourceAdmissionError::ReceiptIdentityMismatch)?,
                )?;
                (work, receipt)
            }
            SafeVectorParserProfile::SafeSvg2 => {
                let decoded = safe_vector::decode_v2_with_work_budget(
                    source.bytes(),
                    limits,
                    remaining_nodes,
                    remaining_path_work,
                )?;
                let work = decoded.work;
                let owner = VerifiedMetadataReceiptOwner::new();
                let receipt = owner.issue_safe_vector_v2(
                    source,
                    decoded.ir,
                    limits.fingerprint(),
                    self.m4_profile_fingerprint
                        .ok_or(ResourceAdmissionError::ReceiptIdentityMismatch)?,
                )?;
                (work, receipt)
            }
        };
        let next_nodes = self
            .vector_nodes_used
            .checked_add(work.nodes)
            .ok_or(ResourceAdmissionError::VectorNodeLimit)?;
        if next_nodes > limits.extension().get().max_vector_nodes {
            return Err(ResourceAdmissionError::VectorNodeLimit);
        }
        let next_path_work = self
            .vector_path_work_used
            .checked_add(work.path_work)
            .ok_or(ResourceAdmissionError::VectorPathSegmentLimit)?;
        if next_path_work > limits.extension().get().max_vector_path_segments {
            return Err(ResourceAdmissionError::VectorPathSegmentLimit);
        }
        self.bind_verified_metadata(receipt)?;
        self.vector_nodes_used = next_nodes;
        self.vector_path_work_used = next_path_work;
        Ok(())
    }

    pub fn parse_and_bind_declared_image(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionError> {
        let image_id = source
            .image_id()
            .ok_or(ResourceAdmissionError::ReceiptKindMismatch)?;
        match self
            .declared_media_policy
            .as_ref()
            .and_then(|policy| policy.images.get(image_id.get() as usize))
            .copied()
            .ok_or(ResourceAdmissionError::DeclaredMediaMismatch)?
        {
            ImageMediaType::Png => self.parse_and_bind_declared_png(source),
            ImageMediaType::SvgSafe1 | ImageMediaType::SvgSafe2 => {
                self.parse_and_bind_declared_safe_vector(source)
            }
        }
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

    pub fn parse_and_bind_declared_sfnt_with_subject(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionFailureOutcome> {
        let subject = source.error_subject();
        self.parse_and_bind_declared_sfnt(source)
            .map_err(|error| self.failure_outcome(ResourceAdmissionFailure::new(error, subject)))
    }

    pub fn parse_and_bind_declared_png_with_subject(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionFailureOutcome> {
        let subject = source.error_subject();
        self.parse_and_bind_declared_png(source)
            .map_err(|error| self.failure_outcome(ResourceAdmissionFailure::new(error, subject)))
    }

    pub fn parse_and_bind_declared_safe_vector_with_subject(
        &mut self,
        source: PendingResourceBytes,
    ) -> Result<(), ResourceAdmissionFailureOutcome> {
        let subject = source.error_subject();
        self.parse_and_bind_declared_safe_vector(source)
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
                media_kind,
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
                    media_kind,
                    width,
                    height,
                    decoded_bytes,
                );
                let replaced = self.images.insert(id, image);
                debug_assert!(replaced.is_none());
            }
            VerifiedMetadata::SafeVector {
                source,
                ir,
                m4_limits_fingerprint,
                m4_profile_fingerprint,
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
                if self
                    .m4_limits
                    .as_ref()
                    .map_or(true, |limits| limits.fingerprint() != m4_limits_fingerprint)
                {
                    return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
                }
                if self.m4_profile_fingerprint != Some(m4_profile_fingerprint) {
                    return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
                }
                if self.images.contains_key(&id) {
                    return Err(ResourceAdmissionError::ConflictingLogicalResource);
                }
                let image = AdmittedImage::from_verified_safe_vector(
                    id,
                    source.uri,
                    source.bytes,
                    source.sha256,
                    *ir,
                    m4_limits_fingerprint,
                    m4_profile_fingerprint,
                );
                let replaced = self.images.insert(id, image);
                debug_assert!(replaced.is_none());
            }
            VerifiedMetadata::SafeVector2 {
                source,
                ir,
                m4_limits_fingerprint,
                m4_profile_fingerprint,
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
                    return Err(ResourceAdmissionError::InvalidSafeVectorV2(
                        SafeVectorFailureReason::HashMismatch,
                    ));
                }
                if self
                    .m4_limits
                    .as_ref()
                    .map_or(true, |limits| limits.fingerprint() != m4_limits_fingerprint)
                    || self.m4_profile_fingerprint != Some(m4_profile_fingerprint)
                {
                    return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
                }
                if self.images.contains_key(&id) {
                    return Err(ResourceAdmissionError::ConflictingLogicalResource);
                }
                let image = AdmittedImage::from_verified_safe_vector_v2(
                    id,
                    source.uri,
                    source.bytes,
                    source.sha256,
                    *ir,
                    m4_limits_fingerprint,
                    m4_profile_fingerprint,
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
            VerifiedMetadata::Font { source, .. }
            | VerifiedMetadata::Image { source, .. }
            | VerifiedMetadata::SafeVector { source, .. }
            | VerifiedMetadata::SafeVector2 { source, .. } => source.error_subject(),
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
        let mut vector_aliases = Vec::new();
        vector_aliases
            .try_reserve_exact(self.images.len())
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        for image in self.images.values() {
            if image.admitted_safe_vector().is_some() {
                vector_aliases.push((image.content_hash(), image.bytes()));
            }
        }
        validate_safe_vector_digest_aliases(&vector_aliases)?;
        drop(vector_aliases);
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
            declared_media_policy: self.declared_media_policy,
        })
    }
}

fn validate_safe_vector_digest_aliases(
    aliases: &[([u8; 32], &[u8])],
) -> Result<(), ResourceAdmissionError> {
    let mut first_bytes_by_digest = BTreeMap::new();
    for (digest, bytes) in aliases {
        if first_bytes_by_digest
            .get(digest)
            .is_some_and(|first_bytes| *first_bytes != *bytes)
        {
            return Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::ResourceConflict,
            ));
        }
        first_bytes_by_digest.entry(*digest).or_insert(*bytes);
    }
    Ok(())
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

fn attest_font_media_kind(bytes: &[u8]) -> Result<AdmittedFontMediaKind, ResourceAdmissionError> {
    if bytes.get(..4) == Some(b"ttcf") {
        Ok(AdmittedFontMediaKind::TtcTrueTypeGlyf)
    } else if bytes.get(..4) == Some(&0x0001_0000u32.to_be_bytes()) {
        Ok(AdmittedFontMediaKind::SfntTrueTypeGlyf)
    } else {
        Err(ResourceAdmissionError::InvalidMetadata)
    }
}

/// The M4 declaration check classifies both the outer container and the
/// selected face's TrueType outline directory. It intentionally stops before
/// metric decoding or glyph-outline evaluation.
fn attest_declared_font_media_kind(
    bytes: &[u8],
    face_index: u32,
) -> Result<AdmittedFontMediaKind, ResourceAdmissionError> {
    let media_kind = attest_font_media_kind(bytes)?;
    let directory_offset = match media_kind {
        AdmittedFontMediaKind::SfntTrueTypeGlyf => {
            if face_index != 0 {
                return Err(ResourceAdmissionError::InvalidMetadata);
            }
            0usize
        }
        AdmittedFontMediaKind::TtcTrueTypeGlyf => {
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
        }
    };
    let signature_end = directory_offset
        .checked_add(4)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    if bytes.get(directory_offset..signature_end) != Some(&0x0001_0000u32.to_be_bytes()) {
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    let table_count = usize::from(read_be_u16(
        bytes,
        directory_offset
            .checked_add(4)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?,
    )?);
    let directory_start = directory_offset
        .checked_add(12)
        .ok_or(ResourceAdmissionError::InvalidMetadata)?;
    let mut has_glyf = false;
    let mut has_loca = false;
    let mut has_cff = false;
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
        match bytes
            .get(record..tag_end)
            .ok_or(ResourceAdmissionError::InvalidMetadata)?
        {
            b"glyf" => has_glyf = true,
            b"loca" => has_loca = true,
            b"CFF " | b"CFF2" => has_cff = true,
            _ => {}
        }
    }
    if !has_glyf || !has_loca || has_cff {
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    Ok(media_kind)
}

fn attest_image_media_kind(bytes: &[u8]) -> Result<AdmittedImageMediaKind, ResourceAdmissionError> {
    if bytes.get(..8) == Some(b"\x89PNG\r\n\x1a\n") {
        Ok(AdmittedImageMediaKind::Png)
    } else {
        Err(ResourceAdmissionError::InvalidMetadata)
    }
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

/// Fully decodes the bounded stable-read payload before issuing the PNG media
/// attestation. IHDR sniffing alone is intentionally insufficient: corrupt
/// chunks or pixel data may not enter the admitted ledger as PNG.
fn validate_png_decoder_attestation(
    bytes: &[u8],
    width: NonZeroU32,
    height: NonZeroU32,
    decoded_byte_budget: u64,
) -> Result<(), ResourceAdmissionError> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
    let output_len = reader
        .output_buffer_size()
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    let admitted_budget =
        usize::try_from(decoded_byte_budget).map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    if output_len == 0 || output_len > admitted_budget {
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    let mut decoded = Vec::new();
    decoded
        .try_reserve_exact(output_len)
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    decoded.resize(output_len, 0);
    let frame = reader
        .next_frame(&mut decoded)
        .map_err(|_| ResourceAdmissionError::InvalidMetadata)?;
    if frame.width != width.get()
        || frame.height != height.get()
        || frame.bit_depth != png::BitDepth::Eight
        || frame.buffer_size() == 0
        || frame.buffer_size() > decoded.len()
    {
        return Err(ResourceAdmissionError::InvalidMetadata);
    }
    Ok(())
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
    declared_media_policy: Option<Arc<DeclaredMediaPolicy>>,
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
            canonical.push('{');
            if let Some(vector) = image.admitted_safe_vector() {
                canonical.push_str("\"allocation_charge\":");
                canonical.push_str(&vector.allocation_charge().to_string());
                canonical.push_str(",\"image_id\":");
                canonical.push_str(&image.image_id().get().to_string());
                canonical.push_str(",\"intrinsic_height\":");
                canonical.push_str(&vector.intrinsic_height().get().raw().to_string());
                canonical.push_str(",\"intrinsic_width\":");
                canonical.push_str(&vector.intrinsic_width().get().raw().to_string());
                canonical.push_str(",\"ir_fingerprint\":");
                push_hash_hex(&mut canonical, vector.fingerprint());
                if matches!(vector, AdmittedSafeVector::V2(_)) {
                    canonical.push_str(",\"ir_id\":");
                    push_jcs_string(&mut canonical, vector.ir_id());
                }
                canonical.push_str(",\"limits_fingerprint\":");
                push_hash_hex(
                    &mut canonical,
                    image
                        .m4_limits_fingerprint()
                        .expect("SafeVector admission carries its limits identity"),
                );
                canonical.push_str(",\"media_kind\":");
                push_jcs_string(&mut canonical, image.media_kind().as_str());
                if matches!(vector, AdmittedSafeVector::V2(_)) {
                    canonical.push_str(",\"parser_id\":");
                    push_jcs_string(&mut canonical, vector.parser_id());
                }
                canonical.push_str(",\"profile_fingerprint\":");
                push_hash_hex(
                    &mut canonical,
                    image
                        .m4_profile_fingerprint()
                        .expect("SafeVector admission carries its profile identity"),
                );
                canonical.push_str(",\"sha256\":");
                push_hash_hex(&mut canonical, image.content_hash());
                canonical.push('}');
                continue;
            }
            canonical.push_str("\"decoded_bytes\":");
            canonical.push_str(&image.decoded_bytes().to_string());
            canonical.push_str(",\"image_id\":");
            canonical.push_str(&image.image_id().get().to_string());
            canonical.push_str(",\"media_kind\":");
            push_jcs_string(&mut canonical, image.media_kind().as_str());
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

/// Stable M4 media observation issued only from a complete admitted ledger.
/// It is separate from the frozen admitted-resource fingerprint so old
/// manifests and layout epochs retain their exact bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingDeclaredFontAttestation {
    font_face_id: FontFaceId,
    uri: PortablePath,
    family: String,
    face_index: u32,
    declared: FontMediaType,
    attested: AdmittedFontMediaKind,
    sha256: [u8; 32],
}

impl StagingDeclaredFontAttestation {
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
    pub const fn declared(&self) -> FontMediaType {
        self.declared
    }
    pub const fn attested(&self) -> AdmittedFontMediaKind {
        self.attested
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingDeclaredImageAttestation {
    image_id: ImageResourceId,
    uri: PortablePath,
    declared: ImageMediaType,
    attested: AdmittedImageMediaKind,
    sha256: [u8; 32],
    safe_vector_ir_fingerprint: Option<[u8; 32]>,
    safe_vector_ir_id: Option<&'static str>,
    safe_vector_allocation_charge: Option<u64>,
    m4_limits_fingerprint: Option<[u8; 32]>,
    safe_vector_parser_id: Option<&'static str>,
    m4_profile_fingerprint: Option<[u8; 32]>,
}

impl StagingDeclaredImageAttestation {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub const fn declared(&self) -> ImageMediaType {
        self.declared
    }
    pub const fn attested(&self) -> AdmittedImageMediaKind {
        self.attested
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn safe_vector_ir_fingerprint(&self) -> Option<[u8; 32]> {
        self.safe_vector_ir_fingerprint
    }
    pub const fn safe_vector_ir_id(&self) -> Option<&'static str> {
        self.safe_vector_ir_id
    }
    pub const fn safe_vector_allocation_charge(&self) -> Option<u64> {
        self.safe_vector_allocation_charge
    }
    pub const fn m4_limits_fingerprint(&self) -> Option<[u8; 32]> {
        self.m4_limits_fingerprint
    }
    pub const fn safe_vector_parser_id(&self) -> Option<&'static str> {
        self.safe_vector_parser_id
    }
    pub const fn m4_profile_fingerprint(&self) -> Option<[u8; 32]> {
        self.m4_profile_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingDeclaredMediaLedger {
    fonts: Vec<StagingDeclaredFontAttestation>,
    images: Vec<StagingDeclaredImageAttestation>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingDeclaredMediaLedger {
    pub fn fonts(&self) -> &[StagingDeclaredFontAttestation] {
        &self.fonts
    }
    pub fn images(&self) -> &[StagingDeclaredImageAttestation] {
        &self.images
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// Base resource catalog plus the profile-approved media policy sealed before
/// stable reads. Deref exposes only the frozen catalog shape to host admission;
/// the resolver constructor consumes the private typed media vectors as well.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingDeclaredBaseCatalog {
    resource_catalog: ResourceCatalog,
    font_media: Vec<FontMediaType>,
    image_media: Vec<ImageMediaType>,
}

impl StagingDeclaredBaseCatalog {
    pub const fn resource_catalog(&self) -> &ResourceCatalog {
        &self.resource_catalog
    }
}

impl std::ops::Deref for StagingDeclaredBaseCatalog {
    type Target = ResourceCatalog;

    fn deref(&self) -> &Self::Target {
        self.resource_catalog()
    }
}

/// Builds the ordinary resolver catalog without inventing media labels. The
/// caller must first hold the M4 profile receipt; legacy variants are rejected
/// before any host-open API can be reached.
pub fn staging_declared_base_catalog(
    declarations: &StagingM4ResourceCatalog,
) -> Result<StagingDeclaredBaseCatalog, ResourceAdmissionError> {
    let mut font_faces = Vec::new();
    let mut font_media = Vec::new();
    font_faces
        .try_reserve_exact(declarations.font_faces.len())
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    font_media
        .try_reserve_exact(declarations.font_faces.len())
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    for declaration in &declarations.font_faces {
        let FontMediaDeclaration::Declared(media) = declaration.media else {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch);
        };
        font_media.push(media);
        font_faces.push(FontFaceDeclaration {
            font_face_id: declaration.font_face_id,
            family: declaration.family.clone(),
            uri: declaration.uri.clone(),
            face_index: declaration.face_index,
            expected_sha256: declaration.expected_sha256,
        });
    }
    let mut images = Vec::new();
    let mut image_media = Vec::new();
    images
        .try_reserve_exact(declarations.images.len())
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    image_media
        .try_reserve_exact(declarations.images.len())
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    for declaration in &declarations.images {
        let ImageMediaDeclaration::Declared(media) = declaration.media else {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch);
        };
        if media == ImageMediaType::SvgSafe2 && declaration.expected_sha256.is_none() {
            return Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::HashMismatch,
            ));
        }
        image_media.push(media);
        images.push(ImageDeclaration {
            image_id: declaration.image_id,
            uri: declaration.uri.clone(),
            expected_sha256: declaration.expected_sha256,
        });
    }
    let catalog = ResourceCatalog { font_faces, images };
    validate_declaration_order(&catalog)?;
    Ok(StagingDeclaredBaseCatalog {
        resource_catalog: catalog,
        font_media,
        image_media,
    })
}

/// Exact declaration/attestation closure used by the internal dump-ast
/// exporter and the new manifest branch. URI suffixes are never consulted.
pub fn close_staging_declared_media(
    admitted: &AdmittedResourceLedger,
    declarations: &StagingM4ResourceCatalog,
) -> Result<StagingDeclaredMediaLedger, ResourceAdmissionError> {
    let expected_font_media: Vec<_> = declarations
        .font_faces
        .iter()
        .map(|declaration| match declaration.media {
            FontMediaDeclaration::Declared(media) => Ok(media),
            FontMediaDeclaration::LegacyUnspecified => {
                Err(ResourceAdmissionError::DeclaredMediaMismatch)
            }
        })
        .collect::<Result<_, _>>()?;
    let expected_image_media: Vec<_> = declarations
        .images
        .iter()
        .map(|declaration| match declaration.media {
            ImageMediaDeclaration::Declared(media) => Ok(media),
            ImageMediaDeclaration::LegacyUnspecified => {
                Err(ResourceAdmissionError::DeclaredMediaMismatch)
            }
        })
        .collect::<Result<_, _>>()?;
    let Some(policy) = admitted.declared_media_policy.as_deref() else {
        return Err(ResourceAdmissionError::DeclaredMediaMismatch);
    };
    if policy.fonts != expected_font_media || policy.images != expected_image_media {
        return Err(ResourceAdmissionError::DeclaredMediaMismatch);
    }
    if admitted.fonts.len() != declarations.font_faces.len()
        || admitted.images.len() != declarations.images.len()
    {
        return Err(ResourceAdmissionError::MissingLogicalResource);
    }
    let mut fonts = Vec::new();
    fonts
        .try_reserve_exact(declarations.font_faces.len())
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    for (font, declaration) in admitted.fonts.iter().zip(&declarations.font_faces) {
        let FontMediaDeclaration::Declared(declared) = declaration.media else {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch);
        };
        let expected = match declared {
            FontMediaType::SfntTrueTypeGlyf => AdmittedFontMediaKind::SfntTrueTypeGlyf,
            FontMediaType::TtcTrueTypeGlyf => AdmittedFontMediaKind::TtcTrueTypeGlyf,
        };
        let observed = attest_declared_font_media_kind(font.bytes(), font.face_index())
            .map_err(|_| ResourceAdmissionError::DeclaredMediaMismatch)?;
        if font.font_face_id() != declaration.font_face_id
            || font.uri() != &declaration.uri
            || font.family() != declaration.family
            || font.face_index() != declaration.face_index
            || observed != expected
            || declaration
                .expected_sha256
                .is_some_and(|hash| hash != font.content_hash())
        {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch);
        }
        fonts.push(StagingDeclaredFontAttestation {
            font_face_id: font.font_face_id(),
            uri: font.uri().clone(),
            family: font.family().to_owned(),
            face_index: font.face_index(),
            declared,
            attested: observed,
            sha256: font.content_hash(),
        });
    }
    let mut images = Vec::new();
    images
        .try_reserve_exact(declarations.images.len())
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    for (image, declaration) in admitted.images.iter().zip(&declarations.images) {
        let ImageMediaDeclaration::Declared(declared) = declaration.media else {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch);
        };
        let expected = match declared {
            ImageMediaType::Png => AdmittedImageMediaKind::Png,
            ImageMediaType::SvgSafe1 => AdmittedImageMediaKind::SafeVector,
            ImageMediaType::SvgSafe2 => AdmittedImageMediaKind::SafeVector2,
        };
        if image.image_id() != declaration.image_id
            || image.uri() != &declaration.uri
            || image.media_kind() != expected
            || declaration
                .expected_sha256
                .is_some_and(|hash| hash != image.content_hash())
        {
            return Err(ResourceAdmissionError::DeclaredMediaMismatch);
        }
        let vector = image.admitted_safe_vector();
        let is_v2 = matches!(vector, Some(AdmittedSafeVector::V2(_)));
        images.push(StagingDeclaredImageAttestation {
            image_id: image.image_id(),
            uri: image.uri().clone(),
            declared,
            attested: image.media_kind(),
            sha256: image.content_hash(),
            safe_vector_ir_fingerprint: vector.map(AdmittedSafeVector::fingerprint),
            safe_vector_ir_id: is_v2.then_some(SAFE_VECTOR_IR_ID_V2),
            safe_vector_allocation_charge: vector.map(AdmittedSafeVector::allocation_charge),
            m4_limits_fingerprint: image.m4_limits_fingerprint(),
            safe_vector_parser_id: is_v2.then_some(SAFE_SVG_PARSER_ID_V2),
            m4_profile_fingerprint: image.m4_profile_fingerprint(),
        });
    }
    let canonical_jcs = encode_staging_declared_media(&fonts, &images);
    Ok(StagingDeclaredMediaLedger {
        fonts,
        images,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    })
}

fn encode_staging_declared_media(
    fonts: &[StagingDeclaredFontAttestation],
    images: &[StagingDeclaredImageAttestation],
) -> String {
    let mut output =
        String::from("{\"algorithm\":\"typaxis.declared-media-attestation/1\",\"fonts\":[");
    for (index, font) in fonts.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"attested_media_kind\":");
        push_jcs_string(&mut output, font.attested.as_str());
        output.push_str(",\"declared_media_type\":");
        push_jcs_string(&mut output, font.declared.as_str());
        output.push_str(",\"face_index\":");
        output.push_str(&font.face_index.to_string());
        output.push_str(",\"family\":");
        push_jcs_string(&mut output, &font.family);
        output.push_str(",\"font_face_id\":");
        output.push_str(&font.font_face_id.get().to_string());
        output.push_str(",\"sha256\":");
        push_hash_hex(&mut output, font.sha256);
        output.push_str(",\"uri\":");
        push_jcs_string(&mut output, font.uri.as_str());
        output.push('}');
    }
    output.push_str("],\"images\":[");
    for (index, image) in images.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"attested_media_kind\":");
        push_jcs_string(&mut output, image.attested.as_str());
        output.push_str(",\"declared_media_type\":");
        push_jcs_string(&mut output, image.declared.as_str());
        output.push_str(",\"image_id\":");
        output.push_str(&image.image_id.get().to_string());
        if let Some(charge) = image.safe_vector_allocation_charge {
            output.push_str(",\"safe_vector_allocation_charge\":");
            output.push_str(&charge.to_string());
        }
        if let Some(fingerprint) = image.safe_vector_ir_fingerprint {
            output.push_str(",\"safe_vector_ir_fingerprint\":");
            push_hash_hex(&mut output, fingerprint);
        }
        if let Some(ir_id) = image.safe_vector_ir_id {
            output.push_str(",\"safe_vector_ir_id\":");
            push_jcs_string(&mut output, ir_id);
        }
        if let Some(fingerprint) = image.m4_limits_fingerprint {
            output.push_str(",\"safe_vector_limits_fingerprint\":");
            push_hash_hex(&mut output, fingerprint);
        }
        if let Some(parser_id) = image.safe_vector_parser_id {
            output.push_str(",\"safe_vector_parser_id\":");
            push_jcs_string(&mut output, parser_id);
        }
        if let Some(fingerprint) = image.m4_profile_fingerprint {
            output.push_str(",\"safe_vector_profile_fingerprint\":");
            push_hash_hex(&mut output, fingerprint);
        }
        output.push_str(",\"sha256\":");
        push_hash_hex(&mut output, image.sha256);
        output.push_str(",\"uri\":");
        push_jcs_string(&mut output, image.uri.as_str());
        output.push('}');
    }
    output.push_str("]}");
    output
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
        sha256, ConfigResourceRoot, EffectiveDataVersions, HostPath, M4ResourceLimits,
        PdfStreamCompression, ResourceLimits, DEFAULT_ALLOWED_URI_SCHEMES,
        REGISTERED_JAPANESE_LINE_BREAK_VERSION, REGISTERED_UNICODE_VERSION,
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

        let mismatch = ResourceAdmissionFailure::new(
            ResourceAdmissionError::DeclaredMediaMismatch,
            ResourceErrorSubject::Image(ImageResourceId::new(2)),
        );
        assert_eq!(
            mismatch.public_error().unwrap().code(),
            typaxis_diagnostics::R7100
        );
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
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&vec![0; width as usize * height as usize * 4])
                .unwrap();
        }
        bytes
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
        assert_eq!(image.media_kind(), AdmittedImageMediaKind::Png);
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
    fn png_media_attestation_requires_a_complete_decoder_pass() {
        let mut bytes = png(2, 3);
        bytes.truncate(bytes.len() - 20);
        let (width, height, decoded_bytes) = parse_png_metadata(&bytes).unwrap();
        assert_eq!(
            validate_png_decoder_attestation(&bytes, width, height, decoded_bytes),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
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
    fn host_open_cannot_be_rebound_to_a_different_resource_declaration() {
        let bytes = sfnt();
        let host_catalog = font_catalog(1);
        let mut resolver_catalog = host_catalog.clone();
        resolver_catalog.font_faces[0].family = "different-family".to_owned();
        let tree = TempTree::new("resolver-declaration-identity");
        fs::write(tree.path().join("font-0.ttf"), bytes).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let host = HostResourceAdmissionSession::new(
            &host_context(tree.path(), &[]),
            &config,
            &host_catalog,
        )
        .unwrap();
        let mut resolver = AdmittedResourceResolver::new_with_roots(
            &resolver_catalog,
            config.limits(),
            host.roots(),
        )
        .unwrap();
        assert!(matches!(
            resolver.read_font(host.open_font(FontFaceId::new(0)).unwrap()),
            Err(ResourceAdmissionError::ReceiptIdentityMismatch)
        ));
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

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn declared_media_base_exactly_attests_png_sfnt_and_ttc_without_suffixes() {
        let sfnt = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"
        ));
        let ttc = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/collection.bin"
        ));
        let png = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/cover.bin"
        ));
        let declared = StagingM4ResourceCatalog {
            font_faces: vec![
                typaxis_document::StagingM4FontFaceDeclaration {
                    font_face_id: FontFaceId::new(0),
                    family: "Body".to_owned(),
                    uri: PortablePath::new("body.bin").unwrap(),
                    face_index: 0,
                    expected_sha256: Some(sha256(sfnt)),
                    media: FontMediaDeclaration::Declared(FontMediaType::SfntTrueTypeGlyf),
                },
                typaxis_document::StagingM4FontFaceDeclaration {
                    font_face_id: FontFaceId::new(1),
                    family: "Collection".to_owned(),
                    uri: PortablePath::new("collection.bin").unwrap(),
                    face_index: 0,
                    expected_sha256: Some(sha256(ttc)),
                    media: FontMediaDeclaration::Declared(FontMediaType::TtcTrueTypeGlyf),
                },
            ],
            images: vec![typaxis_document::StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("cover.bin").unwrap(),
                expected_sha256: Some(sha256(png)),
                media: ImageMediaDeclaration::Declared(ImageMediaType::Png),
                vector_provenance: None,
            }],
        };
        let catalog = staging_declared_base_catalog(&declared).unwrap();
        let tree = TempTree::new("declared-media-base");
        fs::write(tree.path().join("body.bin"), sfnt).unwrap();
        fs::write(tree.path().join("collection.bin"), ttc).unwrap();
        fs::write(tree.path().join("cover.bin"), png).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let mut resolver = AdmittedResourceResolver::new_with_declared_roots(
            &catalog,
            config.limits(),
            host.roots(),
        )
        .unwrap();
        let first = resolver
            .read_font(host.open_font(FontFaceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_declared_sfnt(first).unwrap();
        let second = resolver
            .read_font(host.open_font(FontFaceId::new(1)).unwrap())
            .unwrap();
        resolver.parse_and_bind_declared_sfnt(second).unwrap();
        let image = resolver
            .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_declared_png(image).unwrap();
        let ledger = resolver.finish().unwrap();
        let mut unbound = ledger.clone();
        unbound.declared_media_policy = None;
        assert_eq!(
            close_staging_declared_media(&unbound, &declared),
            Err(ResourceAdmissionError::DeclaredMediaMismatch)
        );
        let closed = close_staging_declared_media(&ledger, &declared).unwrap();
        assert_eq!(closed.fonts().len(), 2);
        assert_eq!(closed.images().len(), 1);
        assert_eq!(
            closed.fonts()[0].attested(),
            AdmittedFontMediaKind::SfntTrueTypeGlyf
        );
        assert_eq!(
            closed.fonts()[1].attested(),
            AdmittedFontMediaKind::TtcTrueTypeGlyf
        );
        assert!(closed
            .canonical_jcs()
            .contains("\"declared_media_type\":\"ttc-truetype-glyf\""));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn declared_media_base_rejects_legacy_before_open_and_mismatch_after_stable_read() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"
        ));
        let mut wrong_outline = bytes.to_vec();
        let glyf = wrong_outline
            .windows(4)
            .position(|window| window == b"glyf")
            .unwrap();
        wrong_outline[glyf..glyf + 4].copy_from_slice(b"CFF ");
        assert_eq!(
            attest_declared_font_media_kind(&wrong_outline, 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
        let mut ambiguous_outline = bytes.to_vec();
        let name = ambiguous_outline
            .windows(4)
            .position(|window| window == b"name")
            .unwrap();
        ambiguous_outline[name..name + 4].copy_from_slice(b"CFF ");
        assert_eq!(
            attest_declared_font_media_kind(&ambiguous_outline, 0),
            Err(ResourceAdmissionError::InvalidMetadata)
        );
        let mut declared = StagingM4ResourceCatalog {
            font_faces: vec![typaxis_document::StagingM4FontFaceDeclaration {
                font_face_id: FontFaceId::new(0),
                family: "Body".to_owned(),
                uri: PortablePath::new("font.resource").unwrap(),
                face_index: 0,
                expected_sha256: Some(sha256(bytes)),
                media: FontMediaDeclaration::LegacyUnspecified,
            }],
            images: vec![],
        };
        assert_eq!(
            staging_declared_base_catalog(&declared),
            Err(ResourceAdmissionError::DeclaredMediaMismatch)
        );

        declared.font_faces[0].media =
            FontMediaDeclaration::Declared(FontMediaType::TtcTrueTypeGlyf);
        let catalog = staging_declared_base_catalog(&declared).unwrap();
        let tree = TempTree::new("declared-media-mismatch");
        fs::write(tree.path().join("font.resource"), bytes).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let mut resolver = AdmittedResourceResolver::new_with_declared_roots(
            &catalog,
            config.limits(),
            host.roots(),
        )
        .unwrap();
        let pending = resolver
            .read_font(host.open_font(FontFaceId::new(0)).unwrap())
            .unwrap();
        assert_eq!(
            // The legacy-named parser cannot bypass a resolver whose M4
            // declaration policy was sealed before the stable read.
            resolver.parse_and_bind_sfnt(pending),
            Err(ResourceAdmissionError::DeclaredMediaMismatch)
        );
        assert!(resolver.progress_token().fonts().is_empty());
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn vector_stable_read_binds_hash_media_limits_ir_and_attestation() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/vector-media/job/art.vector"
        ));
        let declarations = StagingM4ResourceCatalog {
            font_faces: vec![],
            images: vec![typaxis_document::StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("art.vector").unwrap(),
                expected_sha256: Some(sha256(bytes)),
                media: ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1),
                vector_provenance: None,
            }],
        };
        let catalog = staging_declared_base_catalog(&declarations).unwrap();
        let tree = TempTree::new("safe-vector");
        fs::write(tree.path().join("art.vector"), bytes).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let limits = M4EffectiveResourceLimits::defaults_for(config.limits());
        let profile_fingerprint = sha256(b"typaxis.test-safe-vector-profile/1");
        let host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &catalog,
            &limits,
            profile_fingerprint,
            host.roots(),
        )
        .unwrap();
        let pending = resolver
            .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_declared_image(pending).unwrap();
        let ledger = resolver.finish().unwrap();
        let image = ledger.image(ImageResourceId::new(0)).unwrap();
        assert_eq!(image.media_kind(), AdmittedImageMediaKind::SafeVector);
        assert_eq!(image.content_hash(), sha256(bytes));
        assert_eq!(image.m4_limits_fingerprint(), Some(limits.fingerprint()));
        assert_eq!(image.m4_profile_fingerprint(), Some(profile_fingerprint));
        let ir = image.safe_vector().unwrap();
        assert!(!ir.draws().is_empty());
        assert_eq!(
            ir.fingerprint(),
            safe_vector::decode(bytes, &limits)
                .unwrap()
                .ir
                .fingerprint()
        );

        let closed = close_staging_declared_media(&ledger, &declarations).unwrap();
        assert_eq!(
            closed.images()[0].attested(),
            AdmittedImageMediaKind::SafeVector
        );
        assert_eq!(
            closed.images()[0].safe_vector_ir_fingerprint(),
            Some(ir.fingerprint())
        );
        assert_eq!(
            closed.images()[0].m4_profile_fingerprint(),
            Some(profile_fingerprint)
        );
        let canonical = closed.canonical_jcs();
        assert!(
            canonical.find("safe_vector_ir_fingerprint").unwrap()
                < canonical.find("sha256").unwrap()
        );
        assert!(canonical.contains("safe_vector_profile_fingerprint"));

        let altered_limits = M4EffectiveResourceLimits::new(
            config.limits().clone(),
            M4ResourceLimits {
                max_vector_nodes: M4ResourceLimits::default().max_vector_nodes - 1,
                ..M4ResourceLimits::default()
            },
        )
        .unwrap();
        let altered_host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let mut altered_resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &catalog,
            &altered_limits,
            profile_fingerprint,
            altered_host.roots(),
        )
        .unwrap();
        let pending = altered_resolver
            .read_image(altered_host.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        altered_resolver
            .parse_and_bind_declared_image(pending)
            .unwrap();
        let altered_ledger = altered_resolver.finish().unwrap();
        assert_ne!(ledger.fingerprint(), altered_ledger.fingerprint());

        let wrong_declarations = StagingM4ResourceCatalog {
            font_faces: vec![],
            images: vec![typaxis_document::StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("art.vector").unwrap(),
                expected_sha256: Some(sha256(bytes)),
                media: ImageMediaDeclaration::Declared(ImageMediaType::Png),
                vector_provenance: None,
            }],
        };
        let wrong_catalog = staging_declared_base_catalog(&wrong_declarations).unwrap();
        let wrong_host = HostResourceAdmissionSession::new(
            &host_context(tree.path(), &[]),
            &config,
            &wrong_catalog,
        )
        .unwrap();
        let mut wrong_resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &wrong_catalog,
            &limits,
            profile_fingerprint,
            wrong_host.roots(),
        )
        .unwrap();
        let pending = wrong_resolver
            .read_image(wrong_host.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        assert_eq!(
            wrong_resolver.parse_and_bind_declared_image(pending),
            Err(ResourceAdmissionError::DeclaredMediaMismatch)
        );
        assert!(wrong_resolver.progress_token().images().is_empty());

        let png_bytes = png(1, 1);
        fs::write(tree.path().join("wrong.vector"), &png_bytes).unwrap();
        let png_as_vector = StagingM4ResourceCatalog {
            font_faces: vec![],
            images: vec![typaxis_document::StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("wrong.vector").unwrap(),
                expected_sha256: Some(sha256(&png_bytes)),
                media: ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1),
                vector_provenance: None,
            }],
        };
        let png_as_vector_catalog = staging_declared_base_catalog(&png_as_vector).unwrap();
        let png_as_vector_host = HostResourceAdmissionSession::new(
            &host_context(tree.path(), &[]),
            &config,
            &png_as_vector_catalog,
        )
        .unwrap();
        let mut png_as_vector_resolver =
            AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
                &png_as_vector_catalog,
                &limits,
                profile_fingerprint,
                png_as_vector_host.roots(),
            )
            .unwrap();
        let pending = png_as_vector_resolver
            .read_image(
                png_as_vector_host
                    .open_image(ImageResourceId::new(0))
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            png_as_vector_resolver.parse_and_bind_declared_image(pending),
            Err(ResourceAdmissionError::DeclaredMediaMismatch)
        );
        assert!(png_as_vector_resolver.progress_token().images().is_empty());
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn vector_hash_mismatch_precedes_ir_and_session_aggregate_is_monotonic() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/vector-media/job/art.vector"
        ));
        let single = safe_vector::decode(
            bytes,
            &M4EffectiveResourceLimits::defaults_for(&limits(ResourceLimits::default())),
        )
        .unwrap();
        let declarations = StagingM4ResourceCatalog {
            font_faces: vec![],
            images: (0..2)
                .map(|id| typaxis_document::StagingM4ImageDeclaration {
                    image_id: ImageResourceId::new(id),
                    uri: PortablePath::new(format!("art-{id}.vector")).unwrap(),
                    expected_sha256: Some(sha256(bytes)),
                    media: ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe1),
                    vector_provenance: None,
                })
                .collect(),
        };
        let catalog = staging_declared_base_catalog(&declarations).unwrap();
        let tree = TempTree::new("safe-vector-aggregate");
        fs::write(tree.path().join("art-0.vector"), bytes).unwrap();
        fs::write(tree.path().join("art-1.vector"), bytes).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let aggregate_limits = M4EffectiveResourceLimits::new(
            config.limits().clone(),
            M4ResourceLimits {
                max_vector_nodes: single.work.nodes,
                max_vector_path_segments: single.work.path_work,
                ..M4ResourceLimits::default()
            },
        )
        .unwrap();
        let profile_fingerprint = sha256(b"typaxis.test-safe-vector-profile/1");
        let host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let out_of_order_host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let mut out_of_order = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &catalog,
            &aggregate_limits,
            profile_fingerprint,
            out_of_order_host.roots(),
        )
        .unwrap();
        let second = out_of_order
            .read_image(
                out_of_order_host
                    .open_image(ImageResourceId::new(1))
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            out_of_order.parse_and_bind_declared_safe_vector(second),
            Err(ResourceAdmissionError::ReceiptIdentityMismatch)
        );
        assert!(out_of_order.progress_token().images().is_empty());

        let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &catalog,
            &aggregate_limits,
            profile_fingerprint,
            host.roots(),
        )
        .unwrap();
        let first = resolver
            .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_declared_safe_vector(first).unwrap();
        let second = resolver
            .read_image(host.open_image(ImageResourceId::new(1)).unwrap())
            .unwrap();
        assert_eq!(
            resolver.parse_and_bind_declared_safe_vector(second),
            Err(ResourceAdmissionError::VectorNodeLimit)
        );
        assert_eq!(resolver.progress_token().images().len(), 1);

        let mut wrong_hash = declarations.clone();
        wrong_hash.images.truncate(1);
        wrong_hash.images[0].expected_sha256 = Some([0; 32]);
        let wrong_catalog = staging_declared_base_catalog(&wrong_hash).unwrap();
        let wrong_host = HostResourceAdmissionSession::new(
            &host_context(tree.path(), &[]),
            &config,
            &wrong_catalog,
        )
        .unwrap();
        let mut wrong_resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &wrong_catalog,
            &aggregate_limits,
            profile_fingerprint,
            wrong_host.roots(),
        )
        .unwrap();
        let pending = wrong_resolver
            .read_image(wrong_host.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        assert_eq!(
            wrong_resolver.parse_and_bind_declared_safe_vector(pending),
            Err(ResourceAdmissionError::ExpectedHashMismatch)
        );
        assert!(wrong_resolver.progress_token().images().is_empty());
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn safe_svg_2_stable_read_binds_media_parser_ir_hash_and_limits() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg"
        ));
        let declarations = StagingM4ResourceCatalog {
            font_faces: vec![],
            images: vec![typaxis_document::StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("fraction-equality.svg").unwrap(),
                expected_sha256: Some(sha256(bytes)),
                media: ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2),
                vector_provenance: Some(typaxis_document::VectorProvenance {
                    engine_id: "vmb.texToSvg".to_owned(),
                    engine_version: "2026.09.0".to_owned(),
                    rules_version: "vmb.math-safe-svg/1".to_owned(),
                }),
            }],
        };
        let catalog = staging_declared_base_catalog(&declarations).unwrap();
        let tree = TempTree::new("safe-svg-2");
        fs::write(tree.path().join("fraction-equality.svg"), bytes).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let limits = M4EffectiveResourceLimits::defaults_for(config.limits());
        let profile_fingerprint = sha256(b"typaxis.test-safe-svg-2-profile/1");
        let host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &catalog,
            &limits,
            profile_fingerprint,
            host.roots(),
        )
        .unwrap();
        let pending = resolver
            .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        resolver.parse_and_bind_declared_image(pending).unwrap();
        let ledger = resolver.finish().unwrap();
        let image = ledger.image(ImageResourceId::new(0)).unwrap();
        assert_eq!(image.media_kind(), AdmittedImageMediaKind::SafeVector2);
        assert_eq!(image.content_hash(), sha256(bytes));
        assert!(image.safe_vector().is_none());
        let ir = image.safe_vector_v2().unwrap();
        assert_eq!(ir.parser_profile(), SafeVectorParserProfile::SafeSvg2);
        assert_eq!(
            image.admitted_safe_vector().unwrap().parser_id(),
            SAFE_SVG_PARSER_ID_V2
        );
        assert_eq!(
            image.admitted_safe_vector().unwrap().ir_id(),
            SAFE_VECTOR_IR_ID_V2
        );
        assert_eq!(image.m4_limits_fingerprint(), Some(limits.fingerprint()));
        assert_eq!(image.m4_profile_fingerprint(), Some(profile_fingerprint));
        assert!(ir
            .draws()
            .iter()
            .any(|draw| draw.fill().paint() == SafeVectorPaint::CurrentColor));
        assert!(ir
            .draws()
            .iter()
            .any(|draw| draw.fill().alpha().raw() == 49_152));
        assert!(ir
            .draws()
            .iter()
            .any(|draw| draw.stroke().paint().alpha().raw() == 32_768));

        let closed = close_staging_declared_media(&ledger, &declarations).unwrap();
        let attestation = &closed.images()[0];
        assert_eq!(attestation.attested(), AdmittedImageMediaKind::SafeVector2);
        assert_eq!(
            attestation.safe_vector_ir_fingerprint(),
            Some(ir.fingerprint())
        );
        assert_eq!(attestation.safe_vector_ir_id(), Some(SAFE_VECTOR_IR_ID_V2));
        assert_eq!(
            attestation.safe_vector_parser_id(),
            Some(SAFE_SVG_PARSER_ID_V2)
        );
        assert!(closed.canonical_jcs().contains(SAFE_SVG_PARSER_ID_V2));
        assert!(closed.canonical_jcs().contains(SAFE_VECTOR_IR_ID_V2));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn safe_svg_2_declared_hash_mismatch_precedes_parser_work() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/x-plus-y.svg"
        ));
        let missing_hash = StagingM4ResourceCatalog {
            font_faces: vec![],
            images: vec![typaxis_document::StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("x-plus-y.svg").unwrap(),
                expected_sha256: None,
                media: ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2),
                vector_provenance: Some(typaxis_document::VectorProvenance {
                    engine_id: "vmb.texToSvg".to_owned(),
                    engine_version: "2026.09.0".to_owned(),
                    rules_version: "vmb.math-safe-svg/1".to_owned(),
                }),
            }],
        };
        assert_eq!(
            staging_declared_base_catalog(&missing_hash),
            Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::HashMismatch
            ))
        );
        let declarations = StagingM4ResourceCatalog {
            font_faces: vec![],
            images: vec![typaxis_document::StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("x-plus-y.svg").unwrap(),
                expected_sha256: Some([0; 32]),
                media: ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2),
                vector_provenance: Some(typaxis_document::VectorProvenance {
                    engine_id: "vmb.texToSvg".to_owned(),
                    engine_version: "2026.09.0".to_owned(),
                    rules_version: "vmb.math-safe-svg/1".to_owned(),
                }),
            }],
        };
        let catalog = staging_declared_base_catalog(&declarations).unwrap();
        let tree = TempTree::new("safe-svg-2-hash");
        fs::write(tree.path().join("x-plus-y.svg"), bytes).unwrap();
        let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
        let limits = M4EffectiveResourceLimits::defaults_for(config.limits());
        let host =
            HostResourceAdmissionSession::new(&host_context(tree.path(), &[]), &config, &catalog)
                .unwrap();
        let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
            &catalog,
            &limits,
            sha256(b"typaxis.test-safe-svg-2-profile/1"),
            host.roots(),
        )
        .unwrap();
        let pending = resolver
            .read_image(host.open_image(ImageResourceId::new(0)).unwrap())
            .unwrap();
        assert_eq!(
            resolver.parse_and_bind_declared_image(pending),
            Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::HashMismatch
            ))
        );
        assert!(resolver.progress_token().images().is_empty());
    }

    #[test]
    fn safe_svg_2_collision_guard_uses_owner_private_admitted_digest_records() {
        let digest = [7; 32];
        assert_eq!(
            validate_safe_vector_digest_aliases(&[
                (digest, b"same"),
                ([8; 32], b"other"),
                (digest, b"same"),
            ]),
            Ok(())
        );
        assert_eq!(
            validate_safe_vector_digest_aliases(&[(digest, b"first"), (digest, b"second")]),
            Err(ResourceAdmissionError::InvalidSafeVectorV2(
                SafeVectorFailureReason::ResourceConflict
            ))
        );
        assert_eq!(
            validate_safe_vector_digest_aliases(&[([1; 32], b"first"), ([2; 32], b"second")]),
            Ok(())
        );
    }
}
