use crate::location::{
    JsonLocationIndexAxis, JsonLocationIndexBudget, JsonLocationIndexBuildError,
};
use crate::*;
use serde::de::{self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use typaxis_core::{sha256, ValidatedResourceLimits, JSON_SAFE_INTEGER_MAX};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentPackageDecodeErrorClass {
    Shape,
    Contract,
    Limit,
    CanonicalEncoding,
    InternalInvariant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentPackageDecodeLimit {
    Sources,
    TextBuffers,
    AstNodes,
    StyleRules,
    PageMasters,
    FontFaces,
    Images,
    TextBufferBytes,
    AggregateTextBytes,
    PackageItems,
    PointerBytes,
    PlatformAddressSpace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentPackageDecodePrimary {
    Key,
    Value,
    ContainingObject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentPackageTypedDecodeErrorKind {
    UnknownField,
    MissingField,
    TypeMismatch,
    IntegerOutOfRange,
    InvalidValue,
    UnknownEnumTag,
    UnknownContract,
    UnknownCoordinateUnit,
    LimitExceeded {
        limit_kind: DocumentPackageDecodeLimit,
        limit: u64,
        attempted: u64,
    },
    AllocationFailed {
        limit_kind: DocumentPackageDecodeLimit,
    },
    CanonicalEncodingRejected,
    InternalDecoderInvariant,
}

impl DocumentPackageTypedDecodeErrorKind {
    pub const fn class(self) -> DocumentPackageDecodeErrorClass {
        match self {
            Self::UnknownContract | Self::UnknownCoordinateUnit => {
                DocumentPackageDecodeErrorClass::Contract
            }
            Self::LimitExceeded { .. } | Self::AllocationFailed { .. } => {
                DocumentPackageDecodeErrorClass::Limit
            }
            Self::CanonicalEncodingRejected => DocumentPackageDecodeErrorClass::CanonicalEncoding,
            Self::InternalDecoderInvariant => DocumentPackageDecodeErrorClass::InternalInvariant,
            _ => DocumentPackageDecodeErrorClass::Shape,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentPackageDecodeLocation {
    byte_offset: u64,
    json_pointer: JsonPointer,
    primary: DocumentPackageDecodePrimary,
}

impl DocumentPackageDecodeLocation {
    pub const fn byte_offset(&self) -> u64 {
        self.byte_offset
    }

    pub const fn json_pointer(&self) -> &JsonPointer {
        &self.json_pointer
    }

    pub const fn primary(&self) -> DocumentPackageDecodePrimary {
        self.primary
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentPackageTypedDecodeError {
    kind: DocumentPackageTypedDecodeErrorKind,
    location: DocumentPackageDecodeLocation,
}

impl DocumentPackageTypedDecodeError {
    pub const fn kind(&self) -> DocumentPackageTypedDecodeErrorKind {
        self.kind
    }

    pub const fn class(&self) -> DocumentPackageDecodeErrorClass {
        self.kind.class()
    }

    pub const fn location(&self) -> &DocumentPackageDecodeLocation {
        &self.location
    }
}

impl fmt::Display for DocumentPackageTypedDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pointer = self.location.json_pointer();
        let byte = self.location.byte_offset();
        match self.kind {
            DocumentPackageTypedDecodeErrorKind::UnknownContract => {
                write!(formatter, "unknown DocumentPackage contract at {pointer}, byte {byte}")
            }
            DocumentPackageTypedDecodeErrorKind::UnknownCoordinateUnit => {
                write!(formatter, "unknown coordinate unit at {pointer}, byte {byte}")
            }
            DocumentPackageTypedDecodeErrorKind::LimitExceeded {
                limit_kind,
                limit,
                attempted,
            } => write!(
                formatter,
                "DocumentPackage {limit_kind:?} budget {limit} was exceeded by {attempted} at {pointer}, byte {byte}"
            ),
            kind => write!(
                formatter,
                "typed DocumentPackage decode failed ({kind:?}) at {pointer}, byte {byte}"
            ),
        }
    }
}

impl std::error::Error for DocumentPackageTypedDecodeError {}

#[derive(Debug)]
enum DecodeErrorCause {
    Preflight(JsonPreflightError),
    Typed(DocumentPackageTypedDecodeError),
}

#[derive(Debug)]
pub struct DocumentPackageDecodeError {
    cause: Box<DecodeErrorCause>,
}

impl DocumentPackageDecodeError {
    fn preflight(error: JsonPreflightError) -> Self {
        Self {
            cause: Box::new(DecodeErrorCause::Preflight(error)),
        }
    }

    fn typed(error: DocumentPackageTypedDecodeError) -> Self {
        Self {
            cause: Box::new(DecodeErrorCause::Typed(error)),
        }
    }

    pub fn preflight_error(&self) -> Option<&JsonPreflightError> {
        match self.cause.as_ref() {
            DecodeErrorCause::Preflight(error) => Some(error),
            DecodeErrorCause::Typed(_) => None,
        }
    }

    pub fn typed_error(&self) -> Option<&DocumentPackageTypedDecodeError> {
        match self.cause.as_ref() {
            DecodeErrorCause::Typed(error) => Some(error),
            DecodeErrorCause::Preflight(_) => None,
        }
    }
}

impl fmt::Display for DocumentPackageDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.cause.as_ref() {
            DecodeErrorCause::Preflight(error) => error.fmt(formatter),
            DecodeErrorCause::Typed(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for DocumentPackageDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.cause.as_ref() {
            DecodeErrorCause::Preflight(error) => Some(error),
            DecodeErrorCause::Typed(error) => Some(error),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DocumentPackageDecodePolicy<'a> {
    preflight_limits: DocumentPackagePreflightLimits,
    resource_limits: &'a ValidatedResourceLimits,
}

impl<'a> DocumentPackageDecodePolicy<'a> {
    pub fn new(resource_limits: &'a ValidatedResourceLimits) -> Self {
        Self {
            preflight_limits: DocumentPackagePreflightLimits::from_resource_limits(resource_limits),
            resource_limits,
        }
    }

    pub const fn with_preflight_limits(
        resource_limits: &'a ValidatedResourceLimits,
        preflight_limits: DocumentPackagePreflightLimits,
    ) -> Self {
        Self {
            preflight_limits,
            resource_limits,
        }
    }

    pub const fn preflight_limits(self) -> DocumentPackagePreflightLimits {
        self.preflight_limits
    }

    pub const fn resource_limits(self) -> &'a ValidatedResourceLimits {
        self.resource_limits
    }
}

#[derive(Debug)]
struct DecoderIssuedBinding;

/// A strict-decoder receipt. Its fields and decoder binding are private.
///
/// The wire DTO remains caller-constructible, but this receipt cannot be made
/// by attaching hashes or an index to caller-provided parts:
///
/// ```compile_fail
/// use typaxis_document_package::DecodedDocumentPackage;
/// let _forged = DecodedDocumentPackage {
///     wire: todo!(),
///     raw_sha256: todo!(),
///     canonical_jcs_sha256: todo!(),
///     locations: todo!(),
///     _binding: todo!(),
/// };
/// ```
pub struct DecodedDocumentPackage {
    wire: WireDocumentPackage,
    raw_sha256: RawDocumentPackageSha256,
    canonical_jcs_sha256: CanonicalDocumentPackageJcsSha256,
    locations: JsonLocationIndex,
    _binding: DecoderIssuedBinding,
}

impl DecodedDocumentPackage {
    pub const fn wire(&self) -> &WireDocumentPackage {
        &self.wire
    }

    pub const fn raw_sha256(&self) -> RawDocumentPackageSha256 {
        self.raw_sha256
    }

    pub const fn canonical_jcs_sha256(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.canonical_jcs_sha256
    }

    pub const fn locations(&self) -> &JsonLocationIndex {
        &self.locations
    }

    /// Consume this decoder-issued receipt without cloning its wire payload.
    ///
    /// This is a one-way handoff to later trust-boundary owners. The returned
    /// wire DTO remains untrusted; only the decoder receipt itself proves that
    /// the strict lexical and typed decode phases completed.
    pub fn into_parts(
        self,
    ) -> (
        WireDocumentPackage,
        RawDocumentPackageSha256,
        CanonicalDocumentPackageJcsSha256,
        JsonLocationIndex,
    ) {
        (
            self.wire,
            self.raw_sha256,
            self.canonical_jcs_sha256,
            self.locations,
        )
    }
}

impl fmt::Debug for DecodedDocumentPackage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecodedDocumentPackage")
            .field("contract", &self.wire.contract)
            .field("raw_sha256", &self.raw_sha256)
            .field("canonical_jcs_sha256", &self.canonical_jcs_sha256)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StrictDocumentPackageDecoder;

impl StrictDocumentPackageDecoder {
    pub const fn new() -> Self {
        Self
    }

    pub fn decode(
        &self,
        input: &[u8],
        policy: &DocumentPackageDecodePolicy<'_>,
    ) -> Result<DecodedDocumentPackage, DocumentPackageDecodeError> {
        let preflight = StrictJsonPreflight::new(policy.preflight_limits());
        let report = preflight
            .check(input)
            .map_err(DocumentPackageDecodeError::preflight)?;
        let mut context = DecodeContext::new(policy, report, DecodeDialect::Current)
            .map_err(|pending| DocumentPackageDecodeError::typed(pending.into_error(input, 0)))?;

        let mut json = serde_json::Deserializer::from_slice(input);
        // This is safe only because the iterative preflight above has already
        // enforced the profile-hard maximum of 256 JSON containers.
        json.disable_recursion_limit();
        let stacker = serde_stacker::Deserializer::new(&mut json);
        let mut track = serde_path_to_error::Track::new();
        let tracked = serde_path_to_error::Deserializer::new(stacker, &mut track);
        let decoded = DecodeSeed::<WireDocumentPackage>::new(&mut context).deserialize(tracked);

        let wire = match decoded {
            Ok(wire) => wire,
            Err(error) => {
                let fallback_path = tracked_path(track.path());
                let pending = context.pending.take().unwrap_or(PendingDecodeError {
                    kind: DocumentPackageTypedDecodeErrorKind::TypeMismatch,
                    path: fallback_path,
                    primary: DocumentPackageDecodePrimary::Value,
                });
                let fallback = line_column_to_offset(input, error.line(), error.column());
                return Err(DocumentPackageDecodeError::typed(
                    pending.into_error(input, fallback),
                ));
            }
        };

        // Preflight established a single complete root value. `end` is kept as
        // a fail-closed assertion against deserializer/preflight drift.
        if let Err(error) = json.end() {
            let pending = PendingDecodeError {
                kind: DocumentPackageTypedDecodeErrorKind::InternalDecoderInvariant,
                path: Vec::new(),
                primary: DocumentPackageDecodePrimary::ContainingObject,
            };
            return Err(DocumentPackageDecodeError::typed(pending.into_error(
                input,
                line_column_to_offset(input, error.line(), error.column()),
            )));
        }

        let encoder = DocumentPackageEncoder::new(policy.preflight_limits().max_bytes().get())
            .map_err(|_| canonical_error(input))?;
        let canonical = encoder.analyze(&wire).map_err(|_| canonical_error(input))?;
        let location_budget = context.location_budget();
        let locations = JsonLocationIndex::build(&wire, location_budget)
            .map_err(|error| location_index_error(input, error))?;

        Ok(DecodedDocumentPackage {
            wire,
            raw_sha256: RawDocumentPackageSha256::new(sha256(input)),
            canonical_jcs_sha256: CanonicalDocumentPackageJcsSha256::new(canonical.sha256()),
            locations,
            _binding: DecoderIssuedBinding,
        })
    }
}

/// Compatibility receipt retained for focused contract 1.2 slice tests. The
/// public [`StrictDocumentPackageDecoder`] accepts the same 1.2 shape; this
/// wrapper is not a production runner or an alternate profile selector.
pub struct DecodedStagingStyleDocumentPackage {
    wire: WireDocumentPackage,
    raw_sha256: RawDocumentPackageSha256,
    canonical_jcs_sha256: CanonicalDocumentPackageJcsSha256,
    locations: JsonLocationIndex,
    _binding: DecoderIssuedBinding,
}

impl DecodedStagingStyleDocumentPackage {
    pub const CONTRACT: &'static str = "typaxis.contract/1.2";

    pub const fn wire(&self) -> &WireDocumentPackage {
        &self.wire
    }

    pub const fn raw_sha256(&self) -> RawDocumentPackageSha256 {
        self.raw_sha256
    }

    pub const fn canonical_jcs_sha256(&self) -> CanonicalDocumentPackageJcsSha256 {
        self.canonical_jcs_sha256
    }

    pub const fn locations(&self) -> &JsonLocationIndex {
        &self.locations
    }

    pub fn into_parts(
        self,
    ) -> (
        WireDocumentPackage,
        RawDocumentPackageSha256,
        CanonicalDocumentPackageJcsSha256,
        JsonLocationIndex,
    ) {
        (
            self.wire,
            self.raw_sha256,
            self.canonical_jcs_sha256,
            self.locations,
        )
    }
}

impl fmt::Debug for DecodedStagingStyleDocumentPackage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecodedStagingStyleDocumentPackage")
            .field("contract", &Self::CONTRACT)
            .field("raw_sha256", &self.raw_sha256)
            .field("canonical_jcs_sha256", &self.canonical_jcs_sha256)
            .finish_non_exhaustive()
    }
}

/// Strict, bounded compatibility decoder used by focused M2 slice tests.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StagingStyleDocumentPackageDecoder;

impl StagingStyleDocumentPackageDecoder {
    pub const fn new() -> Self {
        Self
    }

    pub fn decode(
        &self,
        input: &[u8],
        policy: &DocumentPackageDecodePolicy<'_>,
    ) -> Result<DecodedStagingStyleDocumentPackage, DocumentPackageDecodeError> {
        let preflight = StrictJsonPreflight::new(policy.preflight_limits());
        let report = preflight
            .check(input)
            .map_err(DocumentPackageDecodeError::preflight)?;
        let mut context = DecodeContext::new(policy, report, DecodeDialect::StagingStyle1_2)
            .map_err(|pending| DocumentPackageDecodeError::typed(pending.into_error(input, 0)))?;

        let mut json = serde_json::Deserializer::from_slice(input);
        json.disable_recursion_limit();
        let stacker = serde_stacker::Deserializer::new(&mut json);
        let mut track = serde_path_to_error::Track::new();
        let tracked = serde_path_to_error::Deserializer::new(stacker, &mut track);
        let decoded = DecodeSeed::<WireDocumentPackage>::new(&mut context).deserialize(tracked);
        let wire = match decoded {
            Ok(wire) => wire,
            Err(error) => {
                let fallback_path = tracked_path(track.path());
                let pending = context.pending.take().unwrap_or(PendingDecodeError {
                    kind: DocumentPackageTypedDecodeErrorKind::TypeMismatch,
                    path: fallback_path,
                    primary: DocumentPackageDecodePrimary::Value,
                });
                let fallback = line_column_to_offset(input, error.line(), error.column());
                return Err(DocumentPackageDecodeError::typed(
                    pending.into_error(input, fallback),
                ));
            }
        };
        if let Err(error) = json.end() {
            let pending = PendingDecodeError {
                kind: DocumentPackageTypedDecodeErrorKind::InternalDecoderInvariant,
                path: Vec::new(),
                primary: DocumentPackageDecodePrimary::ContainingObject,
            };
            return Err(DocumentPackageDecodeError::typed(pending.into_error(
                input,
                line_column_to_offset(input, error.line(), error.column()),
            )));
        }

        let encoder =
            StagingStyleDocumentPackageEncoder::new(policy.preflight_limits().max_bytes().get())
                .map_err(|_| canonical_error(input))?;
        let canonical = encoder.analyze(&wire).map_err(|_| canonical_error(input))?;
        let location_budget = context.location_budget();
        let locations = JsonLocationIndex::build(&wire, location_budget)
            .map_err(|error| location_index_error(input, error))?;
        Ok(DecodedStagingStyleDocumentPackage {
            wire,
            raw_sha256: RawDocumentPackageSha256::new(sha256(input)),
            canonical_jcs_sha256: CanonicalDocumentPackageJcsSha256::new(canonical.sha256()),
            locations,
            _binding: DecoderIssuedBinding,
        })
    }
}

fn canonical_error(input: &[u8]) -> DocumentPackageDecodeError {
    DocumentPackageDecodeError::typed(
        PendingDecodeError {
            kind: DocumentPackageTypedDecodeErrorKind::CanonicalEncodingRejected,
            path: Vec::new(),
            primary: DocumentPackageDecodePrimary::ContainingObject,
        }
        .into_error(input, 0),
    )
}

fn location_index_error(
    input: &[u8],
    error: JsonLocationIndexBuildError,
) -> DocumentPackageDecodeError {
    let limit_kind = match error.axis {
        JsonLocationIndexAxis::PackageBytes => DocumentPackageDecodeLimit::PointerBytes,
        JsonLocationIndexAxis::Sources => DocumentPackageDecodeLimit::Sources,
        JsonLocationIndexAxis::TextBuffers => DocumentPackageDecodeLimit::TextBuffers,
        JsonLocationIndexAxis::AstNodes => DocumentPackageDecodeLimit::AstNodes,
        JsonLocationIndexAxis::StyleRules => DocumentPackageDecodeLimit::StyleRules,
        JsonLocationIndexAxis::PageMasters => DocumentPackageDecodeLimit::PageMasters,
        JsonLocationIndexAxis::Fonts => DocumentPackageDecodeLimit::FontFaces,
        JsonLocationIndexAxis::Images => DocumentPackageDecodeLimit::Images,
        JsonLocationIndexAxis::PlatformAddressSpace => {
            DocumentPackageDecodeLimit::PlatformAddressSpace
        }
    };
    DocumentPackageDecodeError::typed(
        PendingDecodeError {
            kind: DocumentPackageTypedDecodeErrorKind::LimitExceeded {
                limit_kind,
                limit: error.limit,
                attempted: error.attempted,
            },
            path: Vec::new(),
            primary: DocumentPackageDecodePrimary::ContainingObject,
        }
        .into_error(input, 0),
    )
}

#[derive(Clone, Debug)]
enum DecodePathSegment {
    Static(&'static str),
    Owned(String),
    Index(usize),
}

impl DecodePathSegment {
    fn push_pointer(&self, pointer: &mut JsonPointer) {
        match self {
            Self::Static(value) => pointer.push_segment(value),
            Self::Owned(value) => pointer.push_segment(value),
            Self::Index(value) => pointer.push_segment(&value.to_string()),
        }
    }

    fn matches_key(&self, key: &str) -> bool {
        match self {
            Self::Static(value) => *value == key,
            Self::Owned(value) => value == key,
            Self::Index(_) => false,
        }
    }
}

#[derive(Clone, Debug)]
struct PendingDecodeError {
    kind: DocumentPackageTypedDecodeErrorKind,
    path: Vec<DecodePathSegment>,
    primary: DocumentPackageDecodePrimary,
}

impl PendingDecodeError {
    fn into_error(self, input: &[u8], fallback: u64) -> DocumentPackageTypedDecodeError {
        let byte_offset = RawJsonLocator::new(input)
            .locate(&self.path, self.primary)
            .map(to_u64)
            .unwrap_or(fallback);
        let mut json_pointer = JsonPointer::root();
        for segment in &self.path {
            segment.push_pointer(&mut json_pointer);
        }
        DocumentPackageTypedDecodeError {
            kind: self.kind,
            location: DocumentPackageDecodeLocation {
                byte_offset,
                json_pointer,
                primary: self.primary,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DecodeLimits {
    package_bytes: u64,
    sources: u64,
    text_buffers: u64,
    ast_nodes: u64,
    style_rules: u64,
    page_masters: u64,
    fonts: u64,
    images: u64,
    text_buffer_bytes: u64,
    aggregate_text_bytes: u64,
}

impl DecodeLimits {
    fn from_policy(policy: &DocumentPackageDecodePolicy<'_>) -> Self {
        let limits = policy.resource_limits().get();
        Self {
            package_bytes: policy.preflight_limits().max_bytes().get(),
            sources: u64::from(limits.max_include_files).saturating_add(1),
            // There are no dedicated buffer/master limits before MI1-14. The
            // existing AST/style caps are the staging bounds for these arrays.
            text_buffers: limits.max_ast_nodes,
            ast_nodes: limits.max_ast_nodes,
            style_rules: limits.max_style_rules,
            page_masters: limits.max_style_rules,
            fonts: u64::from(limits.max_fonts),
            images: u64::from(limits.max_images),
            text_buffer_bytes: u64::from(limits.max_text_buffer_bytes),
            aggregate_text_bytes: limits.max_text_bytes,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Counter {
    Sources,
    TextBuffers,
    AstNodes,
    AstUnits,
    StyleRules,
    PageMasters,
    Fonts,
    Images,
    PackageItems,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DecodeDialect {
    Current,
    StagingStyle1_2,
}

struct DecodeContext {
    dialect: DecodeDialect,
    limits: DecodeLimits,
    sources: u64,
    text_buffers: u64,
    ast_nodes: u64,
    indexed_ast_nodes: u64,
    style_rules: u64,
    page_masters: u64,
    fonts: u64,
    images: u64,
    package_items: u64,
    aggregate_text_bytes: u64,
    path: Vec<DecodePathSegment>,
    pending: Option<PendingDecodeError>,
}

impl DecodeContext {
    fn new(
        policy: &DocumentPackageDecodePolicy<'_>,
        report: JsonPreflightReport<'_>,
        dialect: DecodeDialect,
    ) -> Result<Self, PendingDecodeError> {
        let mut path = Vec::new();
        path.try_reserve_exact(usize::from(report.maximum_depth()))
            .map_err(|_| PendingDecodeError {
                kind: DocumentPackageTypedDecodeErrorKind::AllocationFailed {
                    limit_kind: DocumentPackageDecodeLimit::PackageItems,
                },
                path: Vec::new(),
                primary: DocumentPackageDecodePrimary::ContainingObject,
            })?;
        Ok(Self {
            dialect,
            limits: DecodeLimits::from_policy(policy),
            sources: 0,
            text_buffers: 0,
            ast_nodes: 0,
            indexed_ast_nodes: 0,
            style_rules: 0,
            page_masters: 0,
            fonts: 0,
            images: 0,
            package_items: 0,
            aggregate_text_bytes: 0,
            path,
            pending: None,
        })
    }

    fn location_budget(&self) -> JsonLocationIndexBudget {
        JsonLocationIndexBudget {
            package_bytes: self.limits.package_bytes,
            sources: self.limits.sources,
            text_buffers: self.limits.text_buffers,
            ast_nodes: self.limits.ast_nodes,
            style_rules: self.limits.style_rules,
            page_masters: self.limits.page_masters,
            fonts: self.limits.fonts,
            images: self.limits.images,
            observed_ast_nodes: self.indexed_ast_nodes,
        }
    }

    fn consume<E: de::Error>(&mut self, counter: Counter) -> Result<(), E> {
        let indexes_node = matches!(counter, Counter::AstNodes);
        let (used, limit, limit_kind) = match counter {
            Counter::Sources => (
                &mut self.sources,
                self.limits.sources,
                DocumentPackageDecodeLimit::Sources,
            ),
            Counter::TextBuffers => (
                &mut self.text_buffers,
                self.limits.text_buffers,
                DocumentPackageDecodeLimit::TextBuffers,
            ),
            Counter::AstNodes => (
                &mut self.ast_nodes,
                self.limits.ast_nodes,
                DocumentPackageDecodeLimit::AstNodes,
            ),
            Counter::AstUnits => (
                &mut self.ast_nodes,
                self.limits.ast_nodes,
                DocumentPackageDecodeLimit::AstNodes,
            ),
            Counter::StyleRules => (
                &mut self.style_rules,
                self.limits.style_rules,
                DocumentPackageDecodeLimit::StyleRules,
            ),
            Counter::PageMasters => (
                &mut self.page_masters,
                self.limits.page_masters,
                DocumentPackageDecodeLimit::PageMasters,
            ),
            Counter::Fonts => (
                &mut self.fonts,
                self.limits.fonts,
                DocumentPackageDecodeLimit::FontFaces,
            ),
            Counter::Images => (
                &mut self.images,
                self.limits.images,
                DocumentPackageDecodeLimit::Images,
            ),
            Counter::PackageItems => (
                &mut self.package_items,
                self.limits.package_bytes,
                DocumentPackageDecodeLimit::PackageItems,
            ),
        };
        let attempted = used.checked_add(1).unwrap_or(u64::MAX);
        if attempted > limit {
            return Err(self.fail(
                DocumentPackageTypedDecodeErrorKind::LimitExceeded {
                    limit_kind,
                    limit,
                    attempted,
                },
                DocumentPackageDecodePrimary::Value,
            ));
        }
        *used = attempted;
        if indexes_node {
            self.indexed_ast_nodes = self.indexed_ast_nodes.saturating_add(1);
        }
        Ok(())
    }

    fn consume_text<E: de::Error>(&mut self, bytes: usize) -> Result<(), E> {
        let bytes = to_u64(bytes);
        if bytes > self.limits.text_buffer_bytes {
            return Err(self.fail(
                DocumentPackageTypedDecodeErrorKind::LimitExceeded {
                    limit_kind: DocumentPackageDecodeLimit::TextBufferBytes,
                    limit: self.limits.text_buffer_bytes,
                    attempted: bytes,
                },
                DocumentPackageDecodePrimary::Value,
            ));
        }
        let aggregate = self.aggregate_text_bytes.saturating_add(bytes);
        if aggregate > self.limits.aggregate_text_bytes {
            return Err(self.fail(
                DocumentPackageTypedDecodeErrorKind::LimitExceeded {
                    limit_kind: DocumentPackageDecodeLimit::AggregateTextBytes,
                    limit: self.limits.aggregate_text_bytes,
                    attempted: aggregate,
                },
                DocumentPackageDecodePrimary::Value,
            ));
        }
        self.aggregate_text_bytes = aggregate;
        Ok(())
    }

    fn with_segment<T, E: de::Error, F>(
        &mut self,
        segment: DecodePathSegment,
        primary: DocumentPackageDecodePrimary,
        decode: F,
    ) -> Result<T, E>
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
    {
        self.path.push(segment);
        let result = decode(self);
        if result.is_err() && self.pending.is_none() {
            self.pending = Some(PendingDecodeError {
                kind: DocumentPackageTypedDecodeErrorKind::TypeMismatch,
                path: self.path.clone(),
                primary,
            });
        }
        self.path.pop();
        result
    }

    fn fail<E: de::Error>(
        &mut self,
        kind: DocumentPackageTypedDecodeErrorKind,
        primary: DocumentPackageDecodePrimary,
    ) -> E {
        if self.pending.is_none() {
            self.pending = Some(PendingDecodeError {
                kind,
                path: self.path.clone(),
                primary,
            });
        }
        E::custom("strict typed DocumentPackage decode failed")
    }

    fn fail_child<E: de::Error>(
        &mut self,
        field: DecodePathSegment,
        kind: DocumentPackageTypedDecodeErrorKind,
        primary: DocumentPackageDecodePrimary,
    ) -> E {
        self.path.push(field);
        let error = self.fail(kind, primary);
        self.path.pop();
        error
    }
}

trait Decode<'de>: Sized {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error>;
}

struct DecodeSeed<'a, T> {
    context: &'a mut DecodeContext,
    marker: PhantomData<T>,
}

impl<'a, T> DecodeSeed<'a, T> {
    fn new(context: &'a mut DecodeContext) -> Self {
        Self {
            context,
            marker: PhantomData,
        }
    }
}

impl<'de, T: Decode<'de>> DeserializeSeed<'de> for DecodeSeed<'_, T> {
    type Value = T;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<T, D::Error> {
        T::decode(self.context, deserializer)
    }
}

struct OptionDecodeSeed<'a, T> {
    context: &'a mut DecodeContext,
    marker: PhantomData<T>,
}

impl<'de, T: Decode<'de>> DeserializeSeed<'de> for OptionDecodeSeed<'_, T> {
    type Value = Option<T>;

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        struct OptionVisitor<'a, T> {
            context: &'a mut DecodeContext,
            marker: PhantomData<T>,
        }
        impl<'de, T: Decode<'de>> Visitor<'de> for OptionVisitor<'_, T> {
            type Value = Option<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("null or a typed DocumentPackage object")
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_some<D: de::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                T::decode(self.context, deserializer).map(Some)
            }
        }
        deserializer.deserialize_option(OptionVisitor::<T> {
            context: self.context,
            marker: PhantomData,
        })
    }
}

struct VecDecodeSeed<'a, T> {
    context: &'a mut DecodeContext,
    marker: PhantomData<T>,
}

impl<'a, T> VecDecodeSeed<'a, T> {
    fn new(context: &'a mut DecodeContext) -> Self {
        Self {
            context,
            marker: PhantomData,
        }
    }
}

impl<'de, T: Decode<'de>> DeserializeSeed<'de> for VecDecodeSeed<'_, T> {
    type Value = Vec<T>;

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        struct VecVisitor<'a, T> {
            context: &'a mut DecodeContext,
            marker: PhantomData<T>,
        }
        impl<'de, T: Decode<'de>> Visitor<'de> for VecVisitor<'_, T> {
            type Value = Vec<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bounded DocumentPackage array")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Vec<T>, A::Error> {
                let mut values = Vec::new();
                let mut ordinal = 0usize;
                loop {
                    let value = self.context.with_segment(
                        DecodePathSegment::Index(ordinal),
                        DocumentPackageDecodePrimary::Value,
                        |context| {
                            sequence.next_element_seed(CountedDecodeSeed::<T> {
                                context,
                                marker: PhantomData,
                            })
                        },
                    )?;
                    let Some(value) = value else { break };
                    try_reserve_decode(&mut values, self.context)?;
                    values.push(value);
                    ordinal = ordinal.checked_add(1).ok_or_else(|| {
                        self.context.fail(
                            DocumentPackageTypedDecodeErrorKind::LimitExceeded {
                                limit_kind: DocumentPackageDecodeLimit::PackageItems,
                                limit: self.context.limits.package_bytes,
                                attempted: u64::MAX,
                            },
                            DocumentPackageDecodePrimary::Value,
                        )
                    })?;
                }
                Ok(values)
            }
        }
        deserializer.deserialize_seq(VecVisitor::<T> {
            context: self.context,
            marker: PhantomData,
        })
    }
}

struct CountedDecodeSeed<'a, T> {
    context: &'a mut DecodeContext,
    marker: PhantomData<T>,
}

impl<'de, T: Decode<'de>> DeserializeSeed<'de> for CountedDecodeSeed<'_, T> {
    type Value = T;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<T, D::Error> {
        self.context.consume(Counter::PackageItems)?;
        T::decode(self.context, deserializer)
    }
}

struct PrimitiveVecSeed<'a, T> {
    context: &'a mut DecodeContext,
    marker: PhantomData<T>,
}

impl<'a, T> PrimitiveVecSeed<'a, T> {
    fn new(context: &'a mut DecodeContext) -> Self {
        Self {
            context,
            marker: PhantomData,
        }
    }
}

impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for PrimitiveVecSeed<'_, T> {
    type Value = Vec<T>;

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        struct PrimitiveVecVisitor<'a, T> {
            context: &'a mut DecodeContext,
            marker: PhantomData<T>,
        }
        impl<'de, T: Deserialize<'de>> Visitor<'de> for PrimitiveVecVisitor<'_, T> {
            type Value = Vec<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bounded scalar array")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Vec<T>, A::Error> {
                let mut values = Vec::new();
                let mut ordinal = 0usize;
                loop {
                    let value = self.context.with_segment(
                        DecodePathSegment::Index(ordinal),
                        DocumentPackageDecodePrimary::Value,
                        |context| {
                            sequence.next_element_seed(CountedPrimitiveSeed::<T> {
                                context,
                                marker: PhantomData,
                            })
                        },
                    )?;
                    let Some(value) = value else { break };
                    try_reserve_decode(&mut values, self.context)?;
                    values.push(value);
                    ordinal = ordinal.checked_add(1).ok_or_else(|| {
                        self.context.fail(
                            DocumentPackageTypedDecodeErrorKind::LimitExceeded {
                                limit_kind: DocumentPackageDecodeLimit::PackageItems,
                                limit: self.context.limits.package_bytes,
                                attempted: u64::MAX,
                            },
                            DocumentPackageDecodePrimary::Value,
                        )
                    })?;
                }
                Ok(values)
            }
        }
        deserializer.deserialize_seq(PrimitiveVecVisitor::<T> {
            context: self.context,
            marker: PhantomData,
        })
    }
}

struct CountedPrimitiveSeed<'a, T> {
    context: &'a mut DecodeContext,
    marker: PhantomData<T>,
}

impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for CountedPrimitiveSeed<'_, T> {
    type Value = T;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<T, D::Error> {
        self.context.consume(Counter::PackageItems)?;
        T::deserialize(deserializer)
    }
}

fn try_reserve_decode<T, E: de::Error>(
    values: &mut Vec<T>,
    context: &mut DecodeContext,
) -> Result<(), E> {
    values.try_reserve(1).map_err(|_| {
        context.fail(
            DocumentPackageTypedDecodeErrorKind::AllocationFailed {
                limit_kind: DocumentPackageDecodeLimit::PlatformAddressSpace,
            },
            DocumentPackageDecodePrimary::Value,
        )
    })
}

fn map_decode<'de, A: MapAccess<'de>, T: Decode<'de>>(
    map: &mut A,
    context: &mut DecodeContext,
    field: &'static str,
) -> Result<T, A::Error> {
    context.with_segment(
        DecodePathSegment::Static(field),
        DocumentPackageDecodePrimary::Value,
        |context| map.next_value_seed(DecodeSeed::<T>::new(context)),
    )
}

fn map_option_decode<'de, A: MapAccess<'de>, T: Decode<'de>>(
    map: &mut A,
    context: &mut DecodeContext,
    field: &'static str,
) -> Result<Option<T>, A::Error> {
    context.with_segment(
        DecodePathSegment::Static(field),
        DocumentPackageDecodePrimary::Value,
        |context| {
            map.next_value_seed(OptionDecodeSeed::<T> {
                context,
                marker: PhantomData,
            })
        },
    )
}

fn map_vec_decode<'de, A: MapAccess<'de>, T: Decode<'de>>(
    map: &mut A,
    context: &mut DecodeContext,
    field: &'static str,
) -> Result<Vec<T>, A::Error> {
    context.with_segment(
        DecodePathSegment::Static(field),
        DocumentPackageDecodePrimary::Value,
        |context| map.next_value_seed(VecDecodeSeed::<T>::new(context)),
    )
}

fn map_primitive_vec<'de, A, T>(
    map: &mut A,
    context: &mut DecodeContext,
    field: &'static str,
) -> Result<Vec<T>, A::Error>
where
    A: MapAccess<'de>,
    T: Deserialize<'de>,
{
    context.with_segment(
        DecodePathSegment::Static(field),
        DocumentPackageDecodePrimary::Value,
        |context| map.next_value_seed(PrimitiveVecSeed::<T>::new(context)),
    )
}

fn map_primitive<'de, A, T>(
    map: &mut A,
    context: &mut DecodeContext,
    field: &'static str,
) -> Result<T, A::Error>
where
    A: MapAccess<'de>,
    T: Deserialize<'de>,
{
    context.with_segment(
        DecodePathSegment::Static(field),
        DocumentPackageDecodePrimary::Value,
        |_| map.next_value::<T>(),
    )
}

fn map_checked<'de, A, T, F>(
    map: &mut A,
    context: &mut DecodeContext,
    field: &'static str,
    check: F,
) -> Result<T, A::Error>
where
    A: MapAccess<'de>,
    T: Deserialize<'de>,
    F: FnOnce(T) -> Option<T>,
{
    context.with_segment(
        DecodePathSegment::Static(field),
        DocumentPackageDecodePrimary::Value,
        |context| {
            let value = map.next_value::<T>()?;
            check(value).ok_or_else(|| {
                context.fail(
                    DocumentPackageTypedDecodeErrorKind::IntegerOutOfRange,
                    DocumentPackageDecodePrimary::Value,
                )
            })
        },
    )
}

fn map_string_parse<'de, A, T, F>(
    map: &mut A,
    context: &mut DecodeContext,
    field: &'static str,
    unknown_kind: DocumentPackageTypedDecodeErrorKind,
    parse: F,
) -> Result<T, A::Error>
where
    A: MapAccess<'de>,
    F: FnOnce(&str) -> Option<T>,
{
    context.with_segment(
        DecodePathSegment::Static(field),
        DocumentPackageDecodePrimary::Value,
        |context| {
            let value = map.next_value::<String>()?;
            parse(&value)
                .ok_or_else(|| context.fail(unknown_kind, DocumentPackageDecodePrimary::Value))
        },
    )
}

struct BoundedTextSeed<'a> {
    context: &'a mut DecodeContext,
}

impl<'de> DeserializeSeed<'de> for BoundedTextSeed<'_> {
    type Value = String;

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        struct TextVisitor<'a> {
            context: &'a mut DecodeContext,
        }
        impl Visitor<'_> for TextVisitor<'_> {
            type Value = String;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bounded UTF-8 text buffer")
            }

            fn visit_borrowed_str<E: de::Error>(self, value: &str) -> Result<String, E> {
                self.context.consume_text(value.len())?;
                let mut output = String::new();
                output.try_reserve_exact(value.len()).map_err(|_| {
                    self.context.fail(
                        DocumentPackageTypedDecodeErrorKind::AllocationFailed {
                            limit_kind: DocumentPackageDecodeLimit::TextBufferBytes,
                        },
                        DocumentPackageDecodePrimary::Value,
                    )
                })?;
                output.push_str(value);
                Ok(output)
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<String, E> {
                self.visit_borrowed_str(value)
            }

            fn visit_string<E: de::Error>(self, value: String) -> Result<String, E> {
                self.context.consume_text(value.len())?;
                Ok(value)
            }
        }
        deserializer.deserialize_string(TextVisitor {
            context: self.context,
        })
    }
}

fn set_field<T, E: de::Error>(
    slot: &mut Option<T>,
    value: T,
    context: &mut DecodeContext,
    field: &'static str,
) -> Result<(), E> {
    if slot.replace(value).is_some() {
        return Err(context.fail_child(
            DecodePathSegment::Static(field),
            DocumentPackageTypedDecodeErrorKind::UnknownField,
            DocumentPackageDecodePrimary::Key,
        ));
    }
    Ok(())
}

fn required<T, E: de::Error>(value: Option<T>, context: &mut DecodeContext) -> Result<T, E> {
    value.ok_or_else(|| {
        context.fail(
            DocumentPackageTypedDecodeErrorKind::MissingField,
            DocumentPackageDecodePrimary::ContainingObject,
        )
    })
}

fn unknown_field<E: de::Error>(context: &mut DecodeContext, field: String) -> E {
    context.fail_child(
        DecodePathSegment::Owned(field),
        DocumentPackageTypedDecodeErrorKind::UnknownField,
        DocumentPackageDecodePrimary::Key,
    )
}

fn incompatible_field<E: de::Error>(context: &mut DecodeContext, field: &'static str) -> E {
    context.fail_child(
        DecodePathSegment::Static(field),
        DocumentPackageTypedDecodeErrorKind::UnknownField,
        DocumentPackageDecodePrimary::Key,
    )
}

fn unknown_enum<E: de::Error>(context: &mut DecodeContext, field: &'static str) -> E {
    context.fail_child(
        DecodePathSegment::Static(field),
        DocumentPackageTypedDecodeErrorKind::UnknownEnumTag,
        DocumentPackageDecodePrimary::Value,
    )
}

fn safe_integer(value: i64) -> Option<i64> {
    (value.unsigned_abs() <= JSON_SAFE_INTEGER_MAX as u64).then_some(value)
}

fn positive_safe_integer(value: i64) -> Option<i64> {
    (value > 0 && value <= JSON_SAFE_INTEGER_MAX).then_some(value)
}

fn positive_safe_u64(value: u64) -> Option<u64> {
    (value > 0 && value <= JSON_SAFE_INTEGER_MAX as u64).then_some(value)
}

fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

impl<'de> Decode<'de> for WireDocumentPackage {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct PackageVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for PackageVisitor<'_> {
            type Value = WireDocumentPackage;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed DocumentPackage object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let mut contract = None;
                let mut coordinate_unit = None;
                let mut sources = None;
                let mut text_buffers = None;
                let mut document = None;
                let mut style_sheet = None;
                let mut page_masters = None;
                let mut resources = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "contract" => {
                            let dialect = context.dialect;
                            let value = map_string_parse(
                                &mut map,
                                context,
                                "contract",
                                DocumentPackageTypedDecodeErrorKind::UnknownContract,
                                |value| match dialect {
                                    DecodeDialect::Current => {
                                        DocumentPackageContractId::from_str(value).ok()
                                    }
                                    DecodeDialect::StagingStyle1_2
                                        if value
                                            == DecodedStagingStyleDocumentPackage::CONTRACT =>
                                    {
                                        // The sealed staging receipt, not this carrier field,
                                        // proves the 1.2 identity.
                                        Some(DocumentPackageContractId::CURRENT)
                                    }
                                    DecodeDialect::StagingStyle1_2 => None,
                                },
                            )?;
                            set_field(&mut contract, value, context, "contract")?;
                        }
                        "coordinate_unit" => {
                            let value = map_string_parse(
                                &mut map,
                                context,
                                "coordinate_unit",
                                DocumentPackageTypedDecodeErrorKind::UnknownCoordinateUnit,
                                |value| match value {
                                    "pdf_point_1_65536" => {
                                        Some(WireCoordinateUnit::PdfPoint1_65536)
                                    }
                                    _ => None,
                                },
                            )?;
                            set_field(&mut coordinate_unit, value, context, "coordinate_unit")?;
                        }
                        "sources" => {
                            let value = map_vec_decode(&mut map, context, "sources")?;
                            set_field(&mut sources, value, context, "sources")?;
                        }
                        "text_buffers" => {
                            let value = map_vec_decode(&mut map, context, "text_buffers")?;
                            set_field(&mut text_buffers, value, context, "text_buffers")?;
                        }
                        "document" => {
                            let value = map_decode(&mut map, context, "document")?;
                            set_field(&mut document, value, context, "document")?;
                        }
                        "style_sheet" => {
                            let value = map_decode(&mut map, context, "style_sheet")?;
                            set_field(&mut style_sheet, value, context, "style_sheet")?;
                        }
                        "page_masters" => {
                            let value = map_decode(&mut map, context, "page_masters")?;
                            set_field(&mut page_masters, value, context, "page_masters")?;
                        }
                        "resources" => {
                            let value = map_decode(&mut map, context, "resources")?;
                            set_field(&mut resources, value, context, "resources")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireDocumentPackage {
                    contract: required(contract, context)?,
                    coordinate_unit: required(coordinate_unit, context)?,
                    sources: required(sources, context)?,
                    text_buffers: required(text_buffers, context)?,
                    document: required(document, context)?,
                    style_sheet: required(style_sheet, context)?,
                    page_masters: required(page_masters, context)?,
                    resources: required(resources, context)?,
                })
            }
        }
        deserializer.deserialize_map(PackageVisitor(context))
    }
}

#[derive(Clone, Copy)]
struct WireSha256([u8; 32]);

impl<'de> Decode<'de> for WireSha256 {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct HashVisitor<'a>(&'a mut DecodeContext);
        impl Visitor<'_> for HashVisitor<'_> {
            type Value = WireSha256;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("exactly 64 lowercase hexadecimal SHA-256 digits")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                if value.len() != 64 {
                    return Err(self.0.fail(
                        DocumentPackageTypedDecodeErrorKind::InvalidValue,
                        DocumentPackageDecodePrimary::Value,
                    ));
                }
                let mut bytes = [0u8; 32];
                for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
                    let high = lowercase_hex(pair[0]).ok_or_else(|| {
                        self.0.fail(
                            DocumentPackageTypedDecodeErrorKind::InvalidValue,
                            DocumentPackageDecodePrimary::Value,
                        )
                    })?;
                    let low = lowercase_hex(pair[1]).ok_or_else(|| {
                        self.0.fail(
                            DocumentPackageTypedDecodeErrorKind::InvalidValue,
                            DocumentPackageDecodePrimary::Value,
                        )
                    })?;
                    bytes[index] = high * 16 + low;
                }
                Ok(WireSha256(bytes))
            }
        }
        deserializer.deserialize_str(HashVisitor(context))
    }
}

fn lowercase_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

impl<'de> Decode<'de> for WireSource {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::Sources)?;
        struct SourceVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for SourceVisitor<'_> {
            type Value = WireSource;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed source declaration")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut source_id, mut uri, mut byte_length, mut hash) = (None, None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "source_id" => {
                            let value = map_primitive(&mut map, context, "source_id")?;
                            set_field(&mut source_id, value, context, "source_id")?;
                        }
                        "uri" => {
                            let value = map_primitive(&mut map, context, "uri")?;
                            set_field(&mut uri, value, context, "uri")?;
                        }
                        "utf8_byte_length" => {
                            let value = map_primitive(&mut map, context, "utf8_byte_length")?;
                            set_field(&mut byte_length, value, context, "utf8_byte_length")?;
                        }
                        "sha256" => {
                            let value: WireSha256 = map_decode(&mut map, context, "sha256")?;
                            set_field(&mut hash, value.0, context, "sha256")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireSource {
                    source_id: required(source_id, context)?,
                    uri: required(uri, context)?,
                    utf8_byte_length: required(byte_length, context)?,
                    sha256: required(hash, context)?,
                })
            }
        }
        deserializer.deserialize_map(SourceVisitor(context))
    }
}

macro_rules! decode_u32_object {
    ($type:ty, $name:literal, { $($field:ident),+ $(,)? }) => {
        impl<'de> Decode<'de> for $type {
            fn decode<D: de::Deserializer<'de>>(
                context: &mut DecodeContext,
                deserializer: D,
            ) -> Result<Self, D::Error> {
                struct ValueVisitor<'a>(&'a mut DecodeContext);
                impl<'de> Visitor<'de> for ValueVisitor<'_> {
                    type Value = $type;
                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str($name)
                    }
                    fn visit_map<A: MapAccess<'de>>(
                        self,
                        mut map: A,
                    ) -> Result<Self::Value, A::Error> {
                        let context = self.0;
                        $(let mut $field = None;)+
                        while let Some(field) = map.next_key::<String>()? {
                            match field.as_str() {
                                $(stringify!($field) => {
                                    let value = map_primitive(
                                        &mut map,
                                        context,
                                        stringify!($field),
                                    )?;
                                    set_field(
                                        &mut $field,
                                        value,
                                        context,
                                        stringify!($field),
                                    )?;
                                })+
                                _ => return Err(unknown_field(context, field)),
                            }
                        }
                        Ok(Self::Value {
                            $($field: required($field, context)?,)+
                        })
                    }
                }
                deserializer.deserialize_map(ValueVisitor(context))
            }
        }
    };
}

decode_u32_object!(WireByteRange, "a byte range object", {
    start_byte,
    end_byte
});
decode_u32_object!(WireSourceSpan, "a source span object", {
    source_id,
    start_byte,
    end_byte
});
decode_u32_object!(WireTextSpan, "a text span object", {
    text_id,
    start_byte,
    end_byte
});

impl<'de> Decode<'de> for WireTextMapSegment {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct MappingVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for MappingVisitor<'_> {
            type Value = WireTextMapSegment;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed text mapping object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut text_range, mut kind, mut source_span) = (None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "text_range" => {
                            let value = map_decode(&mut map, context, "text_range")?;
                            set_field(&mut text_range, value, context, "text_range")?;
                        }
                        "kind" => {
                            let value = map_string_parse(
                                &mut map,
                                context,
                                "kind",
                                DocumentPackageTypedDecodeErrorKind::UnknownEnumTag,
                                |value| match value {
                                    "identity" => Some(WireTextMapKind::Identity),
                                    "replacement" => Some(WireTextMapKind::Replacement),
                                    "inserted" => Some(WireTextMapKind::Inserted),
                                    _ => None,
                                },
                            )?;
                            set_field(&mut kind, value, context, "kind")?;
                        }
                        "source_span" => {
                            let value = map_option_decode(&mut map, context, "source_span")?;
                            set_field(&mut source_span, value, context, "source_span")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                let kind = required(kind, context)?;
                let source_span = required(source_span, context)?;
                if matches!(kind, WireTextMapKind::Inserted) != source_span.is_none() {
                    return Err(context.fail_child(
                        DecodePathSegment::Static("source_span"),
                        DocumentPackageTypedDecodeErrorKind::InvalidValue,
                        DocumentPackageDecodePrimary::Value,
                    ));
                }
                Ok(WireTextMapSegment {
                    text_range: required(text_range, context)?,
                    kind,
                    source_span,
                })
            }
        }
        deserializer.deserialize_map(MappingVisitor(context))
    }
}

impl<'de> Decode<'de> for WireTextBuffer {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::TextBuffers)?;
        struct TextBufferVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for TextBufferVisitor<'_> {
            type Value = WireTextBuffer;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed text buffer object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut text_id, mut utf8, mut mappings) = (None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "text_id" => {
                            let value = map_primitive(&mut map, context, "text_id")?;
                            set_field(&mut text_id, value, context, "text_id")?;
                        }
                        "utf8" => {
                            let value = context.with_segment(
                                DecodePathSegment::Static("utf8"),
                                DocumentPackageDecodePrimary::Value,
                                |context| map.next_value_seed(BoundedTextSeed { context }),
                            )?;
                            set_field(&mut utf8, value, context, "utf8")?;
                        }
                        "mappings" => {
                            let value = map_vec_decode(&mut map, context, "mappings")?;
                            set_field(&mut mappings, value, context, "mappings")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireTextBuffer {
                    text_id: required(text_id, context)?,
                    utf8: required(utf8, context)?,
                    mappings: required(mappings, context)?,
                })
            }
        }
        deserializer.deserialize_map(TextBufferVisitor(context))
    }
}

impl<'de> Decode<'de> for WireLinkTarget {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct TargetVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for TargetVisitor<'_> {
            type Value = WireLinkTarget;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed link target object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                decode_link_target_map(self.0, &mut map)
            }
        }
        deserializer.deserialize_map(TargetVisitor(context))
    }
}

fn decode_link_target_map<'de, A: MapAccess<'de>>(
    context: &mut DecodeContext,
    map: &mut A,
) -> Result<WireLinkTarget, A::Error> {
    let (mut kind, mut anchor_id, mut uri) = (None, None, None);
    while let Some(field) = map.next_key::<String>()? {
        match field.as_str() {
            "kind" => {
                let value: String = map_primitive(map, context, "kind")?;
                set_field(&mut kind, value, context, "kind")?;
            }
            "anchor_id" => {
                let value = map_primitive(map, context, "anchor_id")?;
                set_field(&mut anchor_id, value, context, "anchor_id")?;
            }
            "uri" => {
                let value = map_primitive(map, context, "uri")?;
                set_field(&mut uri, value, context, "uri")?;
            }
            _ => return Err(unknown_field(context, field)),
        }
    }
    match required::<_, A::Error>(kind, context)?.as_str() {
        "internal" => {
            if uri.is_some() {
                return Err(incompatible_field(context, "uri"));
            }
            Ok(WireLinkTarget::Internal {
                anchor_id: required(anchor_id, context)?,
            })
        }
        "uri" => {
            if anchor_id.is_some() {
                return Err(incompatible_field(context, "anchor_id"));
            }
            Ok(WireLinkTarget::Uri {
                uri: required(uri, context)?,
            })
        }
        _ => Err(unknown_enum(context, "kind")),
    }
}

enum RawInlineTarget {
    Link(WireLinkTarget),
    Reference(String),
}

impl<'de> Decode<'de> for RawInlineTarget {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct TargetVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for TargetVisitor<'_> {
            type Value = RawInlineTarget;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a link-target object or reference-target string")
            }
            fn visit_borrowed_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(RawInlineTarget::Reference(value.to_owned()))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                self.visit_borrowed_str(value)
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(RawInlineTarget::Reference(value))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                decode_link_target_map(self.0, &mut map).map(RawInlineTarget::Link)
            }
        }
        deserializer.deserialize_any(TargetVisitor(context))
    }
}

impl<'de> Decode<'de> for WireInline {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::AstNodes)?;
        struct InlineVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for InlineVisitor<'_> {
            type Value = WireInline;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed inline node object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let mut kind = None;
                let mut node_id = None;
                let mut span = None;
                let mut text_span = None;
                let mut children = None;
                let mut target = None;
                let mut anchor_id = None;
                let mut format = None;
                let mut footnote_id = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "kind" => {
                            let value = map_primitive(&mut map, context, "kind")?;
                            set_field(&mut kind, value, context, "kind")?;
                        }
                        "node_id" => {
                            let value = map_primitive(&mut map, context, "node_id")?;
                            set_field(&mut node_id, value, context, "node_id")?;
                        }
                        "span" => {
                            let value = map_decode(&mut map, context, "span")?;
                            set_field(&mut span, value, context, "span")?;
                        }
                        "text_span" => {
                            let value = map_decode(&mut map, context, "text_span")?;
                            set_field(&mut text_span, value, context, "text_span")?;
                        }
                        "children" => {
                            let value = map_vec_decode(&mut map, context, "children")?;
                            set_field(&mut children, value, context, "children")?;
                        }
                        "target" => {
                            let value = map_decode(&mut map, context, "target")?;
                            set_field(&mut target, value, context, "target")?;
                        }
                        "anchor_id" => {
                            let value = map_primitive(&mut map, context, "anchor_id")?;
                            set_field(&mut anchor_id, value, context, "anchor_id")?;
                        }
                        "format" => {
                            let value = map_string_parse(
                                &mut map,
                                context,
                                "format",
                                DocumentPackageTypedDecodeErrorKind::UnknownEnumTag,
                                |value| match value {
                                    "text" => Some(WireReferenceFormat::Text),
                                    "page" => Some(WireReferenceFormat::Page),
                                    "number" => Some(WireReferenceFormat::Number),
                                    _ => None,
                                },
                            )?;
                            set_field(&mut format, value, context, "format")?;
                        }
                        "footnote_id" => {
                            let value = map_primitive(&mut map, context, "footnote_id")?;
                            set_field(&mut footnote_id, value, context, "footnote_id")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                let kind: String = required(kind, context)?;
                let node_id = required(node_id, context)?;
                let span = required(span, context)?;
                match kind.as_str() {
                    "text" => {
                        reject_inline_extras(
                            context,
                            &[
                                (children.is_some(), "children"),
                                (target.is_some(), "target"),
                                (anchor_id.is_some(), "anchor_id"),
                                (format.is_some(), "format"),
                                (footnote_id.is_some(), "footnote_id"),
                            ],
                        )?;
                        Ok(WireInline::Text {
                            node_id,
                            span,
                            text_span: required(text_span, context)?,
                        })
                    }
                    "emphasis" | "strong" => {
                        reject_inline_extras(
                            context,
                            &[
                                (text_span.is_some(), "text_span"),
                                (target.is_some(), "target"),
                                (anchor_id.is_some(), "anchor_id"),
                                (format.is_some(), "format"),
                                (footnote_id.is_some(), "footnote_id"),
                            ],
                        )?;
                        let children = required(children, context)?;
                        if kind == "emphasis" {
                            Ok(WireInline::Emphasis {
                                node_id,
                                span,
                                children,
                            })
                        } else {
                            Ok(WireInline::Strong {
                                node_id,
                                span,
                                children,
                            })
                        }
                    }
                    "link" => {
                        reject_inline_extras(
                            context,
                            &[
                                (text_span.is_some(), "text_span"),
                                (anchor_id.is_some(), "anchor_id"),
                                (format.is_some(), "format"),
                                (footnote_id.is_some(), "footnote_id"),
                            ],
                        )?;
                        let target = match required(target, context)? {
                            RawInlineTarget::Link(target) => target,
                            RawInlineTarget::Reference(_) => {
                                return Err(context.fail_child(
                                    DecodePathSegment::Static("target"),
                                    DocumentPackageTypedDecodeErrorKind::TypeMismatch,
                                    DocumentPackageDecodePrimary::Value,
                                ));
                            }
                        };
                        Ok(WireInline::Link {
                            node_id,
                            span,
                            target,
                            children: required(children, context)?,
                        })
                    }
                    "anchor" => {
                        reject_inline_extras(
                            context,
                            &[
                                (text_span.is_some(), "text_span"),
                                (children.is_some(), "children"),
                                (target.is_some(), "target"),
                                (format.is_some(), "format"),
                                (footnote_id.is_some(), "footnote_id"),
                            ],
                        )?;
                        Ok(WireInline::Anchor {
                            node_id,
                            span,
                            anchor_id: required(anchor_id, context)?,
                        })
                    }
                    "reference" => {
                        reject_inline_extras(
                            context,
                            &[
                                (text_span.is_some(), "text_span"),
                                (children.is_some(), "children"),
                                (anchor_id.is_some(), "anchor_id"),
                                (footnote_id.is_some(), "footnote_id"),
                            ],
                        )?;
                        let target = match required(target, context)? {
                            RawInlineTarget::Reference(target) => target,
                            RawInlineTarget::Link(_) => {
                                return Err(context.fail_child(
                                    DecodePathSegment::Static("target"),
                                    DocumentPackageTypedDecodeErrorKind::TypeMismatch,
                                    DocumentPackageDecodePrimary::Value,
                                ));
                            }
                        };
                        Ok(WireInline::Reference {
                            node_id,
                            span,
                            target,
                            format: required(format, context)?,
                        })
                    }
                    "footnote_reference" => {
                        reject_inline_extras(
                            context,
                            &[
                                (text_span.is_some(), "text_span"),
                                (children.is_some(), "children"),
                                (target.is_some(), "target"),
                                (anchor_id.is_some(), "anchor_id"),
                                (format.is_some(), "format"),
                            ],
                        )?;
                        Ok(WireInline::FootnoteReference {
                            node_id,
                            span,
                            footnote_id: required(footnote_id, context)?,
                        })
                    }
                    "soft_break" | "hard_break" => {
                        reject_inline_extras(
                            context,
                            &[
                                (text_span.is_some(), "text_span"),
                                (children.is_some(), "children"),
                                (target.is_some(), "target"),
                                (anchor_id.is_some(), "anchor_id"),
                                (format.is_some(), "format"),
                                (footnote_id.is_some(), "footnote_id"),
                            ],
                        )?;
                        if kind == "soft_break" {
                            Ok(WireInline::SoftBreak { node_id, span })
                        } else {
                            Ok(WireInline::HardBreak { node_id, span })
                        }
                    }
                    _ => Err(unknown_enum(context, "kind")),
                }
            }
        }
        deserializer.deserialize_map(InlineVisitor(context))
    }
}

fn reject_inline_extras<E: de::Error>(
    context: &mut DecodeContext,
    fields: &[(bool, &'static str)],
) -> Result<(), E> {
    if let Some((_, field)) = fields.iter().find(|(present, _)| *present) {
        Err(incompatible_field(context, field))
    } else {
        Ok(())
    }
}

impl<'de> Decode<'de> for WireListItem {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::AstNodes)?;
        struct ListItemVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for ListItemVisitor<'_> {
            type Value = WireListItem;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed list item object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut node_id, mut span, mut blocks) = (None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "node_id" => {
                            let value = map_primitive(&mut map, context, "node_id")?;
                            set_field(&mut node_id, value, context, "node_id")?;
                        }
                        "span" => {
                            let value = map_decode(&mut map, context, "span")?;
                            set_field(&mut span, value, context, "span")?;
                        }
                        "blocks" => {
                            let value = map_vec_decode(&mut map, context, "blocks")?;
                            set_field(&mut blocks, value, context, "blocks")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireListItem {
                    node_id: required(node_id, context)?,
                    span: required(span, context)?,
                    blocks: required(blocks, context)?,
                })
            }
        }
        deserializer.deserialize_map(ListItemVisitor(context))
    }
}

impl<'de> Decode<'de> for WireTableColumn {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        // Columns have no NodeId, but ADR-0029 assigns each wire column one
        // max_ast_nodes unit. Consume it before constructing the column value.
        context.consume(Counter::AstUnits)?;
        struct ColumnVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for ColumnVisitor<'_> {
            type Value = WireTableColumn;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed table column object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut kind, mut width, mut weight) = (None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "kind" => {
                            let value = map_primitive(&mut map, context, "kind")?;
                            set_field(&mut kind, value, context, "kind")?;
                        }
                        "width" => {
                            let value =
                                map_checked(&mut map, context, "width", positive_safe_integer)?;
                            set_field(&mut width, value, context, "width")?;
                        }
                        "weight" => {
                            let value = map_checked(&mut map, context, "weight", |value: u16| {
                                (value > 0).then_some(value)
                            })?;
                            set_field(&mut weight, value, context, "weight")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                match required::<String, A::Error>(kind, context)?.as_str() {
                    "fixed" => {
                        if weight.is_some() {
                            return Err(incompatible_field(context, "weight"));
                        }
                        Ok(WireTableColumn::Fixed {
                            width: required(width, context)?,
                        })
                    }
                    "fraction" => {
                        if width.is_some() {
                            return Err(incompatible_field(context, "width"));
                        }
                        Ok(WireTableColumn::Fraction {
                            weight: required(weight, context)?,
                        })
                    }
                    _ => Err(unknown_enum(context, "kind")),
                }
            }
        }
        deserializer.deserialize_map(ColumnVisitor(context))
    }
}

impl<'de> Decode<'de> for WireTableCell {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::AstNodes)?;
        struct CellVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for CellVisitor<'_> {
            type Value = WireTableCell;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed table cell object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut node_id, mut span, mut colspan, mut rowspan, mut blocks) =
                    (None, None, None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "node_id" => {
                            let value = map_primitive(&mut map, context, "node_id")?;
                            set_field(&mut node_id, value, context, "node_id")?;
                        }
                        "span" => {
                            let value = map_decode(&mut map, context, "span")?;
                            set_field(&mut span, value, context, "span")?;
                        }
                        "colspan" => {
                            let value = map_checked(&mut map, context, "colspan", |value: u16| {
                                (value > 0).then_some(value)
                            })?;
                            set_field(&mut colspan, value, context, "colspan")?;
                        }
                        "rowspan" => {
                            let value = map_checked(&mut map, context, "rowspan", |value: u16| {
                                (value > 0).then_some(value)
                            })?;
                            set_field(&mut rowspan, value, context, "rowspan")?;
                        }
                        "blocks" => {
                            let value = map_vec_decode(&mut map, context, "blocks")?;
                            set_field(&mut blocks, value, context, "blocks")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireTableCell {
                    node_id: required(node_id, context)?,
                    span: required(span, context)?,
                    colspan: required(colspan, context)?,
                    rowspan: required(rowspan, context)?,
                    blocks: required(blocks, context)?,
                })
            }
        }
        deserializer.deserialize_map(CellVisitor(context))
    }
}

impl<'de> Decode<'de> for WireTableRow {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::AstNodes)?;
        struct RowVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for RowVisitor<'_> {
            type Value = WireTableRow;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed table row object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut node_id, mut span, mut cells) = (None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "node_id" => {
                            let value = map_primitive(&mut map, context, "node_id")?;
                            set_field(&mut node_id, value, context, "node_id")?;
                        }
                        "span" => {
                            let value = map_decode(&mut map, context, "span")?;
                            set_field(&mut span, value, context, "span")?;
                        }
                        "cells" => {
                            let value = map_vec_decode(&mut map, context, "cells")?;
                            set_field(&mut cells, value, context, "cells")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireTableRow {
                    node_id: required(node_id, context)?,
                    span: required(span, context)?,
                    cells: required(cells, context)?,
                })
            }
        }
        deserializer.deserialize_map(RowVisitor(context))
    }
}

const BLOCK_KIND: u32 = 1 << 0;
const BLOCK_NODE_ID: u32 = 1 << 1;
const BLOCK_SPAN: u32 = 1 << 2;
const BLOCK_CLASSES: u32 = 1 << 3;
const BLOCK_CHILDREN: u32 = 1 << 4;
const BLOCK_LEVEL: u32 = 1 << 5;
const BLOCK_ANCHOR_ID: u32 = 1 << 6;
const BLOCK_ORDERED: u32 = 1 << 7;
const BLOCK_START: u32 = 1 << 8;
const BLOCK_ITEMS: u32 = 1 << 9;
const BLOCK_COLUMNS: u32 = 1 << 10;
const BLOCK_HEAD: u32 = 1 << 11;
const BLOCK_BODY: u32 = 1 << 12;
const BLOCK_IMAGE_ID: u32 = 1 << 13;
const BLOCK_ALT: u32 = 1 << 14;
const BLOCK_CAPTION: u32 = 1 << 15;
const BLOCK_COMMON: u32 = BLOCK_KIND | BLOCK_NODE_ID | BLOCK_SPAN | BLOCK_CLASSES;

#[derive(Default)]
struct BlockFields {
    present: u32,
    kind: Option<String>,
    node_id: Option<u32>,
    span: Option<WireSourceSpan>,
    classes: Option<Vec<String>>,
    children: Option<Vec<WireInline>>,
    level: Option<u8>,
    anchor_id: Option<Option<String>>,
    ordered: Option<bool>,
    start: Option<Option<u32>>,
    items: Option<Vec<WireListItem>>,
    columns: Option<Vec<WireTableColumn>>,
    head: Option<Vec<WireTableRow>>,
    body: Option<Vec<WireTableRow>>,
    image_id: Option<u32>,
    alt: Option<String>,
    caption: Option<Vec<WireBlock>>,
}

impl BlockFields {
    fn reject_extras<E: de::Error>(
        &self,
        context: &mut DecodeContext,
        allowed: u32,
    ) -> Result<(), E> {
        const FIELDS: &[(u32, &str)] = &[
            (BLOCK_KIND, "kind"),
            (BLOCK_NODE_ID, "node_id"),
            (BLOCK_SPAN, "span"),
            (BLOCK_CLASSES, "classes"),
            (BLOCK_CHILDREN, "children"),
            (BLOCK_LEVEL, "level"),
            (BLOCK_ANCHOR_ID, "anchor_id"),
            (BLOCK_ORDERED, "ordered"),
            (BLOCK_START, "start"),
            (BLOCK_ITEMS, "items"),
            (BLOCK_COLUMNS, "columns"),
            (BLOCK_HEAD, "head"),
            (BLOCK_BODY, "body"),
            (BLOCK_IMAGE_ID, "image_id"),
            (BLOCK_ALT, "alt"),
            (BLOCK_CAPTION, "caption"),
        ];
        if let Some((_, field)) = FIELDS
            .iter()
            .find(|(bit, _)| self.present & *bit != 0 && allowed & *bit == 0)
        {
            Err(incompatible_field(context, field))
        } else {
            Ok(())
        }
    }
}

impl<'de> Decode<'de> for WireBlock {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::AstNodes)?;
        struct BlockVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for BlockVisitor<'_> {
            type Value = WireBlock;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed block node object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let mut fields = BlockFields::default();
                while let Some(field) = map.next_key::<String>()? {
                    let bit = match field.as_str() {
                        "kind" => {
                            let value = map_primitive(&mut map, context, "kind")?;
                            set_field(&mut fields.kind, value, context, "kind")?;
                            BLOCK_KIND
                        }
                        "node_id" => {
                            let value = map_primitive(&mut map, context, "node_id")?;
                            set_field(&mut fields.node_id, value, context, "node_id")?;
                            BLOCK_NODE_ID
                        }
                        "span" => {
                            let value = map_decode(&mut map, context, "span")?;
                            set_field(&mut fields.span, value, context, "span")?;
                            BLOCK_SPAN
                        }
                        "classes" => {
                            let value = map_primitive_vec(&mut map, context, "classes")?;
                            set_field(&mut fields.classes, value, context, "classes")?;
                            BLOCK_CLASSES
                        }
                        "children" => {
                            let value = map_vec_decode(&mut map, context, "children")?;
                            set_field(&mut fields.children, value, context, "children")?;
                            BLOCK_CHILDREN
                        }
                        "level" => {
                            let value = map_checked(&mut map, context, "level", |value: u8| {
                                (1..=6).contains(&value).then_some(value)
                            })?;
                            set_field(&mut fields.level, value, context, "level")?;
                            BLOCK_LEVEL
                        }
                        "anchor_id" => {
                            let value = map_primitive(&mut map, context, "anchor_id")?;
                            set_field(&mut fields.anchor_id, value, context, "anchor_id")?;
                            BLOCK_ANCHOR_ID
                        }
                        "ordered" => {
                            let value = map_primitive(&mut map, context, "ordered")?;
                            set_field(&mut fields.ordered, value, context, "ordered")?;
                            BLOCK_ORDERED
                        }
                        "start" => {
                            let value = context.with_segment(
                                DecodePathSegment::Static("start"),
                                DocumentPackageDecodePrimary::Value,
                                |context| {
                                    let value = map.next_value::<Option<u32>>()?;
                                    if value == Some(0) {
                                        Err(context.fail(
                                            DocumentPackageTypedDecodeErrorKind::IntegerOutOfRange,
                                            DocumentPackageDecodePrimary::Value,
                                        ))
                                    } else {
                                        Ok(value)
                                    }
                                },
                            )?;
                            set_field(&mut fields.start, value, context, "start")?;
                            BLOCK_START
                        }
                        "items" => {
                            let value = map_vec_decode(&mut map, context, "items")?;
                            set_field(&mut fields.items, value, context, "items")?;
                            BLOCK_ITEMS
                        }
                        "columns" => {
                            let value = map_vec_decode(&mut map, context, "columns")?;
                            set_field(&mut fields.columns, value, context, "columns")?;
                            BLOCK_COLUMNS
                        }
                        "head" => {
                            let value = map_vec_decode(&mut map, context, "head")?;
                            set_field(&mut fields.head, value, context, "head")?;
                            BLOCK_HEAD
                        }
                        "body" => {
                            let value = map_vec_decode(&mut map, context, "body")?;
                            set_field(&mut fields.body, value, context, "body")?;
                            BLOCK_BODY
                        }
                        "image_id" => {
                            let value = map_primitive(&mut map, context, "image_id")?;
                            set_field(&mut fields.image_id, value, context, "image_id")?;
                            BLOCK_IMAGE_ID
                        }
                        "alt" => {
                            let value = map_primitive(&mut map, context, "alt")?;
                            set_field(&mut fields.alt, value, context, "alt")?;
                            BLOCK_ALT
                        }
                        "caption" => {
                            let value = map_vec_decode(&mut map, context, "caption")?;
                            set_field(&mut fields.caption, value, context, "caption")?;
                            BLOCK_CAPTION
                        }
                        _ => return Err(unknown_field(context, field)),
                    };
                    fields.present |= bit;
                }
                let kind = required::<String, A::Error>(fields.kind.take(), context)?;
                let node_id = required(fields.node_id.take(), context)?;
                let span = required(fields.span.take(), context)?;
                let classes = required(fields.classes.take(), context)?;
                match kind.as_str() {
                    "paragraph" => {
                        fields.reject_extras(context, BLOCK_COMMON | BLOCK_CHILDREN)?;
                        Ok(WireBlock::Paragraph {
                            node_id,
                            span,
                            classes,
                            children: required(fields.children.take(), context)?,
                        })
                    }
                    "heading" => {
                        fields.reject_extras(
                            context,
                            BLOCK_COMMON | BLOCK_CHILDREN | BLOCK_LEVEL | BLOCK_ANCHOR_ID,
                        )?;
                        Ok(WireBlock::Heading {
                            node_id,
                            span,
                            classes,
                            level: required(fields.level.take(), context)?,
                            anchor_id: required(fields.anchor_id.take(), context)?,
                            children: required(fields.children.take(), context)?,
                        })
                    }
                    "list" => {
                        fields.reject_extras(
                            context,
                            BLOCK_COMMON | BLOCK_ORDERED | BLOCK_START | BLOCK_ITEMS,
                        )?;
                        let ordered = required(fields.ordered.take(), context)?;
                        let start = required(fields.start.take(), context)?;
                        if ordered != start.is_some() {
                            return Err(context.fail_child(
                                DecodePathSegment::Static("start"),
                                DocumentPackageTypedDecodeErrorKind::InvalidValue,
                                DocumentPackageDecodePrimary::Value,
                            ));
                        }
                        let items = required(fields.items.take(), context)?;
                        if items.is_empty() {
                            return Err(context.fail_child(
                                DecodePathSegment::Static("items"),
                                DocumentPackageTypedDecodeErrorKind::InvalidValue,
                                DocumentPackageDecodePrimary::Value,
                            ));
                        }
                        Ok(WireBlock::List {
                            node_id,
                            span,
                            classes,
                            ordered,
                            start,
                            items,
                        })
                    }
                    "table" => {
                        fields.reject_extras(
                            context,
                            BLOCK_COMMON | BLOCK_COLUMNS | BLOCK_HEAD | BLOCK_BODY,
                        )?;
                        let columns = required(fields.columns.take(), context)?;
                        let head = required(fields.head.take(), context)?;
                        let body = required(fields.body.take(), context)?;
                        if columns.is_empty() {
                            return Err(context.fail_child(
                                DecodePathSegment::Static("columns"),
                                DocumentPackageTypedDecodeErrorKind::InvalidValue,
                                DocumentPackageDecodePrimary::Value,
                            ));
                        }
                        if head.is_empty() && body.is_empty() {
                            return Err(context.fail_child(
                                DecodePathSegment::Static("head"),
                                DocumentPackageTypedDecodeErrorKind::InvalidValue,
                                DocumentPackageDecodePrimary::Value,
                            ));
                        }
                        Ok(WireBlock::Table {
                            node_id,
                            span,
                            classes,
                            columns,
                            head,
                            body,
                        })
                    }
                    "figure" => {
                        fields.reject_extras(
                            context,
                            BLOCK_COMMON | BLOCK_IMAGE_ID | BLOCK_ALT | BLOCK_CAPTION,
                        )?;
                        Ok(WireBlock::Figure {
                            node_id,
                            span,
                            classes,
                            image_id: required(fields.image_id.take(), context)?,
                            alt: required(fields.alt.take(), context)?,
                            caption: required(fields.caption.take(), context)?,
                        })
                    }
                    "page_break" => {
                        fields.reject_extras(context, BLOCK_COMMON)?;
                        Ok(WireBlock::PageBreak {
                            node_id,
                            span,
                            classes,
                        })
                    }
                    _ => Err(unknown_enum(context, "kind")),
                }
            }
        }
        deserializer.deserialize_map(BlockVisitor(context))
    }
}

impl<'de> Decode<'de> for WireFootnote {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::AstNodes)?;
        struct FootnoteVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for FootnoteVisitor<'_> {
            type Value = WireFootnote;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed footnote object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut footnote_id, mut node_id, mut span, mut blocks) = (None, None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "footnote_id" => {
                            let value = map_primitive(&mut map, context, "footnote_id")?;
                            set_field(&mut footnote_id, value, context, "footnote_id")?;
                        }
                        "node_id" => {
                            let value = map_primitive(&mut map, context, "node_id")?;
                            set_field(&mut node_id, value, context, "node_id")?;
                        }
                        "span" => {
                            let value = map_decode(&mut map, context, "span")?;
                            set_field(&mut span, value, context, "span")?;
                        }
                        "blocks" => {
                            let value = map_vec_decode(&mut map, context, "blocks")?;
                            set_field(&mut blocks, value, context, "blocks")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireFootnote {
                    footnote_id: required(footnote_id, context)?,
                    node_id: required(node_id, context)?,
                    span: required(span, context)?,
                    blocks: required(blocks, context)?,
                })
            }
        }
        deserializer.deserialize_map(FootnoteVisitor(context))
    }
}

impl<'de> Decode<'de> for WireDocument {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::AstNodes)?;
        struct DocumentVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for DocumentVisitor<'_> {
            type Value = WireDocument;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed document object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut node_id, mut blocks, mut footnotes) = (None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "node_id" => {
                            let value = map_primitive(&mut map, context, "node_id")?;
                            set_field(&mut node_id, value, context, "node_id")?;
                        }
                        "blocks" => {
                            let value = map_vec_decode(&mut map, context, "blocks")?;
                            set_field(&mut blocks, value, context, "blocks")?;
                        }
                        "footnotes" => {
                            let value = map_vec_decode(&mut map, context, "footnotes")?;
                            set_field(&mut footnotes, value, context, "footnotes")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireDocument {
                    node_id: required(node_id, context)?,
                    blocks: required(blocks, context)?,
                    footnotes: required(footnotes, context)?,
                })
            }
        }
        deserializer.deserialize_map(DocumentVisitor(context))
    }
}

enum RawStyleScalar {
    String(String),
    Integer(i64),
    Boolean(bool),
}

impl<'de> Decode<'de> for RawStyleScalar {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct ScalarVisitor<'a>(&'a mut DecodeContext);
        impl Visitor<'_> for ScalarVisitor<'_> {
            type Value = RawStyleScalar;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string, boolean, or JSON-safe integer")
            }
            fn visit_borrowed_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(RawStyleScalar::String(value.to_owned()))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                self.visit_borrowed_str(value)
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(RawStyleScalar::String(value))
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(RawStyleScalar::Boolean(value))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                safe_integer(value)
                    .map(RawStyleScalar::Integer)
                    .ok_or_else(|| {
                        self.0.fail(
                            DocumentPackageTypedDecodeErrorKind::IntegerOutOfRange,
                            DocumentPackageDecodePrimary::Value,
                        )
                    })
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                if value <= JSON_SAFE_INTEGER_MAX as u64 {
                    Ok(RawStyleScalar::Integer(value as i64))
                } else {
                    Err(self.0.fail(
                        DocumentPackageTypedDecodeErrorKind::IntegerOutOfRange,
                        DocumentPackageDecodePrimary::Value,
                    ))
                }
            }
            fn visit_f64<E: de::Error>(self, _value: f64) -> Result<Self::Value, E> {
                Err(self.0.fail(
                    DocumentPackageTypedDecodeErrorKind::TypeMismatch,
                    DocumentPackageDecodePrimary::Value,
                ))
            }
        }
        deserializer.deserialize_any(ScalarVisitor(context))
    }
}

impl<'de> Decode<'de> for WireStyleValue {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct StyleValueVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for StyleValueVisitor<'_> {
            type Value = WireStyleValue;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed style value object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let mut kind = None;
                let mut value = None;
                let mut families = None;
                let mut numerator = None;
                let mut denominator = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "kind" => {
                            let decoded = map_primitive(&mut map, context, "kind")?;
                            set_field(&mut kind, decoded, context, "kind")?;
                        }
                        "value" => {
                            let decoded = map_decode(&mut map, context, "value")?;
                            set_field(&mut value, decoded, context, "value")?;
                        }
                        "families" => {
                            let decoded = map_primitive_vec(&mut map, context, "families")?;
                            set_field(&mut families, decoded, context, "families")?;
                        }
                        "numerator" => {
                            let decoded =
                                map_checked(&mut map, context, "numerator", safe_integer)?;
                            set_field(&mut numerator, decoded, context, "numerator")?;
                        }
                        "denominator" => {
                            let decoded =
                                map_checked(&mut map, context, "denominator", positive_safe_u64)?;
                            set_field(&mut denominator, decoded, context, "denominator")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                let kind: String = required(kind, context)?;
                match kind.as_str() {
                    "keyword" | "string" | "integer" | "length" | "boolean" => {
                        if families.is_some() {
                            return Err(incompatible_field(context, "families"));
                        }
                        if numerator.is_some() {
                            return Err(incompatible_field(context, "numerator"));
                        }
                        if denominator.is_some() {
                            return Err(incompatible_field(context, "denominator"));
                        }
                        let value = required(value, context)?;
                        match (kind.as_str(), value) {
                            ("keyword", RawStyleScalar::String(value)) if !value.is_empty() => {
                                Ok(WireStyleValue::Keyword { value })
                            }
                            ("string", RawStyleScalar::String(value)) => {
                                Ok(WireStyleValue::String { value })
                            }
                            ("integer", RawStyleScalar::Integer(value)) => {
                                Ok(WireStyleValue::Integer { value })
                            }
                            ("length", RawStyleScalar::Integer(value)) => {
                                Ok(WireStyleValue::Length { value })
                            }
                            ("boolean", RawStyleScalar::Boolean(value)) => {
                                Ok(WireStyleValue::Boolean { value })
                            }
                            _ => Err(context.fail_child(
                                DecodePathSegment::Static("value"),
                                DocumentPackageTypedDecodeErrorKind::TypeMismatch,
                                DocumentPackageDecodePrimary::Value,
                            )),
                        }
                    }
                    "font_family_list" => {
                        if value.is_some() {
                            return Err(incompatible_field(context, "value"));
                        }
                        if numerator.is_some() {
                            return Err(incompatible_field(context, "numerator"));
                        }
                        if denominator.is_some() {
                            return Err(incompatible_field(context, "denominator"));
                        }
                        let families: Vec<String> = required(families, context)?;
                        if families.is_empty() || families.iter().any(String::is_empty) {
                            return Err(context.fail_child(
                                DecodePathSegment::Static("families"),
                                DocumentPackageTypedDecodeErrorKind::InvalidValue,
                                DocumentPackageDecodePrimary::Value,
                            ));
                        }
                        Ok(WireStyleValue::FontFamilyList { families })
                    }
                    "ratio" => {
                        if value.is_some() {
                            return Err(incompatible_field(context, "value"));
                        }
                        if families.is_some() {
                            return Err(incompatible_field(context, "families"));
                        }
                        Ok(WireStyleValue::Ratio {
                            numerator: required(numerator, context)?,
                            denominator: required(denominator, context)?,
                        })
                    }
                    _ => Err(unknown_enum(context, "kind")),
                }
            }
        }
        deserializer.deserialize_map(StyleValueVisitor(context))
    }
}

impl<'de> Decode<'de> for WireDeclaration {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct DeclarationVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for DeclarationVisitor<'_> {
            type Value = WireDeclaration;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed style declaration object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut name, mut value, mut important) = (None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "name" => {
                            let decoded = map_string_parse(
                                &mut map,
                                context,
                                "name",
                                DocumentPackageTypedDecodeErrorKind::UnknownEnumTag,
                                |value| match value {
                                    "font_family" => Some(WireDeclarationName::FontFamily),
                                    "font_size" => Some(WireDeclarationName::FontSize),
                                    "line_height" => Some(WireDeclarationName::LineHeight),
                                    "page" => Some(WireDeclarationName::Page),
                                    "space_before" => Some(WireDeclarationName::SpaceBefore),
                                    "space_after" => Some(WireDeclarationName::SpaceAfter),
                                    "start_indent" => Some(WireDeclarationName::StartIndent),
                                    "end_indent" => Some(WireDeclarationName::EndIndent),
                                    "text_align" => Some(WireDeclarationName::TextAlign),
                                    "width" => Some(WireDeclarationName::Width),
                                    "keep_with_next" => Some(WireDeclarationName::KeepWithNext),
                                    "keep_caption" => Some(WireDeclarationName::KeepCaption),
                                    _ => None,
                                },
                            )?;
                            set_field(&mut name, decoded, context, "name")?;
                        }
                        "value" => {
                            let decoded = map_decode(&mut map, context, "value")?;
                            set_field(&mut value, decoded, context, "value")?;
                        }
                        "important" => {
                            let decoded = map_primitive(&mut map, context, "important")?;
                            set_field(&mut important, decoded, context, "important")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                let name = required(name, context)?;
                let value = required(value, context)?;
                let compatible = match (&name, &value) {
                    (WireDeclarationName::FontFamily, WireStyleValue::FontFamilyList { .. }) => {
                        true
                    }
                    (
                        WireDeclarationName::FontSize | WireDeclarationName::LineHeight,
                        WireStyleValue::Length { value },
                    ) => *value > 0,
                    (WireDeclarationName::Page, WireStyleValue::Keyword { value }) => {
                        value == "auto"
                    }
                    (WireDeclarationName::Page, WireStyleValue::String { value }) => {
                        !value.is_empty()
                    }
                    (
                        WireDeclarationName::SpaceBefore
                        | WireDeclarationName::SpaceAfter
                        | WireDeclarationName::StartIndent
                        | WireDeclarationName::EndIndent,
                        WireStyleValue::Length { value },
                    ) => *value >= 0,
                    (WireDeclarationName::TextAlign, WireStyleValue::Keyword { value }) => {
                        matches!(value.as_str(), "start" | "end" | "center")
                    }
                    (WireDeclarationName::Width, WireStyleValue::Keyword { value }) => {
                        value == "auto"
                    }
                    (WireDeclarationName::Width, WireStyleValue::Length { value }) => *value > 0,
                    (
                        WireDeclarationName::KeepWithNext | WireDeclarationName::KeepCaption,
                        WireStyleValue::Boolean { .. },
                    ) => true,
                    _ => false,
                };
                if !compatible {
                    return Err(context.fail_child(
                        DecodePathSegment::Static("value"),
                        DocumentPackageTypedDecodeErrorKind::InvalidValue,
                        DocumentPackageDecodePrimary::Value,
                    ));
                }
                Ok(WireDeclaration {
                    name,
                    value,
                    important: required(important, context)?,
                })
            }
        }
        deserializer.deserialize_map(DeclarationVisitor(context))
    }
}

impl<'de> Decode<'de> for WireStyleRule {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::StyleRules)?;
        struct RuleVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for RuleVisitor<'_> {
            type Value = WireStyleRule;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed style rule object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let mut style_id = None;
                let mut extends = None;
                let mut selector = None;
                let mut source_order = None;
                let mut declarations = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "style_id" => {
                            let decoded = map_primitive(&mut map, context, "style_id")?;
                            set_field(&mut style_id, decoded, context, "style_id")?;
                        }
                        "extends" => {
                            let decoded = map_primitive(&mut map, context, "extends")?;
                            set_field(&mut extends, decoded, context, "extends")?;
                        }
                        "selector" => {
                            let decoded = map_primitive(&mut map, context, "selector")?;
                            set_field(&mut selector, decoded, context, "selector")?;
                        }
                        "source_order" => {
                            let decoded = map_primitive(&mut map, context, "source_order")?;
                            set_field(&mut source_order, decoded, context, "source_order")?;
                        }
                        "declarations" => {
                            let decoded = map_vec_decode(&mut map, context, "declarations")?;
                            set_field(&mut declarations, decoded, context, "declarations")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireStyleRule {
                    style_id: required(style_id, context)?,
                    extends: required(extends, context)?,
                    selector: required(selector, context)?,
                    source_order: required(source_order, context)?,
                    declarations: required(declarations, context)?,
                })
            }
        }
        deserializer.deserialize_map(RuleVisitor(context))
    }
}

impl<'de> Decode<'de> for WireStyleSheet {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct SheetVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for SheetVisitor<'_> {
            type Value = WireStyleSheet;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed style sheet object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let mut rules = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "rules" => {
                            let decoded = map_vec_decode(&mut map, context, "rules")?;
                            set_field(&mut rules, decoded, context, "rules")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireStyleSheet {
                    rules: required(rules, context)?,
                })
            }
        }
        deserializer.deserialize_map(SheetVisitor(context))
    }
}

impl<'de> Decode<'de> for WireRect {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct RectVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for RectVisitor<'_> {
            type Value = WireRect;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed page rectangle object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut x, mut y, mut width, mut height) = (None, None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "x" => {
                            let decoded = map_checked(&mut map, context, "x", safe_integer)?;
                            set_field(&mut x, decoded, context, "x")?;
                        }
                        "y" => {
                            let decoded = map_checked(&mut map, context, "y", safe_integer)?;
                            set_field(&mut y, decoded, context, "y")?;
                        }
                        "width" => {
                            let decoded =
                                map_checked(&mut map, context, "width", positive_safe_integer)?;
                            set_field(&mut width, decoded, context, "width")?;
                        }
                        "height" => {
                            let decoded =
                                map_checked(&mut map, context, "height", positive_safe_integer)?;
                            set_field(&mut height, decoded, context, "height")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireRect {
                    x: required(x, context)?,
                    y: required(y, context)?,
                    width: required(width, context)?,
                    height: required(height, context)?,
                })
            }
        }
        deserializer.deserialize_map(RectVisitor(context))
    }
}

impl<'de> Decode<'de> for WirePageMaster {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::PageMasters)?;
        struct MasterVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for MasterVisitor<'_> {
            type Value = WirePageMaster;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed page master object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let mut master_id = None;
                let mut width = None;
                let mut height = None;
                let mut body = None;
                let mut header = None;
                let mut footer = None;
                let mut footnote = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "master_id" => {
                            let decoded = map_primitive(&mut map, context, "master_id")?;
                            set_field(&mut master_id, decoded, context, "master_id")?;
                        }
                        "width" => {
                            let decoded =
                                map_checked(&mut map, context, "width", positive_safe_integer)?;
                            set_field(&mut width, decoded, context, "width")?;
                        }
                        "height" => {
                            let decoded =
                                map_checked(&mut map, context, "height", positive_safe_integer)?;
                            set_field(&mut height, decoded, context, "height")?;
                        }
                        "body" => {
                            let decoded = map_decode(&mut map, context, "body")?;
                            set_field(&mut body, decoded, context, "body")?;
                        }
                        "header" => {
                            let decoded = map_option_decode(&mut map, context, "header")?;
                            set_field(&mut header, decoded, context, "header")?;
                        }
                        "footer" => {
                            let decoded = map_option_decode(&mut map, context, "footer")?;
                            set_field(&mut footer, decoded, context, "footer")?;
                        }
                        "footnote" => {
                            let decoded = map_option_decode(&mut map, context, "footnote")?;
                            set_field(&mut footnote, decoded, context, "footnote")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WirePageMaster {
                    master_id: required(master_id, context)?,
                    width: required(width, context)?,
                    height: required(height, context)?,
                    body: required(body, context)?,
                    header: required(header, context)?,
                    footer: required(footer, context)?,
                    footnote: required(footnote, context)?,
                })
            }
        }
        deserializer.deserialize_map(MasterVisitor(context))
    }
}

impl<'de> Decode<'de> for WirePageMasterRule {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::PageMasters)?;
        struct MasterRuleVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for MasterRuleVisitor<'_> {
            type Value = WirePageMasterRule;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed page master selection rule")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let mut master_id = None;
                let mut parity = None;
                let mut first = None;
                let mut named_page = None;
                let mut source_order = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "master_id" => {
                            let decoded = map_primitive(&mut map, context, "master_id")?;
                            set_field(&mut master_id, decoded, context, "master_id")?;
                        }
                        "parity" => {
                            let decoded = map_string_parse(
                                &mut map,
                                context,
                                "parity",
                                DocumentPackageTypedDecodeErrorKind::UnknownEnumTag,
                                |value| match value {
                                    "any" => Some(WirePageParity::Any),
                                    "odd" => Some(WirePageParity::Odd),
                                    "even" => Some(WirePageParity::Even),
                                    _ => None,
                                },
                            )?;
                            set_field(&mut parity, decoded, context, "parity")?;
                        }
                        "first" => {
                            let decoded = map_primitive(&mut map, context, "first")?;
                            set_field(&mut first, decoded, context, "first")?;
                        }
                        "named_page" => {
                            let decoded = map_primitive(&mut map, context, "named_page")?;
                            set_field(&mut named_page, decoded, context, "named_page")?;
                        }
                        "source_order" => {
                            let decoded = map_primitive(&mut map, context, "source_order")?;
                            set_field(&mut source_order, decoded, context, "source_order")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WirePageMasterRule {
                    master_id: required(master_id, context)?,
                    parity: required(parity, context)?,
                    first: required(first, context)?,
                    named_page: required(named_page, context)?,
                    source_order: required(source_order, context)?,
                })
            }
        }
        deserializer.deserialize_map(MasterRuleVisitor(context))
    }
}

impl<'de> Decode<'de> for WirePageMasterSet {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct MasterSetVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for MasterSetVisitor<'_> {
            type Value = WirePageMasterSet;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed page master set")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut default_master_id, mut masters, mut selection_rules) = (None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "default_master_id" => {
                            let decoded = map_primitive(&mut map, context, "default_master_id")?;
                            set_field(
                                &mut default_master_id,
                                decoded,
                                context,
                                "default_master_id",
                            )?;
                        }
                        "masters" => {
                            let decoded = map_vec_decode(&mut map, context, "masters")?;
                            set_field(&mut masters, decoded, context, "masters")?;
                        }
                        "selection_rules" => {
                            let decoded = map_vec_decode(&mut map, context, "selection_rules")?;
                            set_field(&mut selection_rules, decoded, context, "selection_rules")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WirePageMasterSet {
                    default_master_id: required(default_master_id, context)?,
                    masters: required(masters, context)?,
                    selection_rules: required(selection_rules, context)?,
                })
            }
        }
        deserializer.deserialize_map(MasterSetVisitor(context))
    }
}

impl<'de> Decode<'de> for WireFontFace {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::Fonts)?;
        struct FontVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for FontVisitor<'_> {
            type Value = WireFontFace;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed font face declaration")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let mut font_face_id = None;
                let mut family = None;
                let mut uri = None;
                let mut face_index = None;
                let mut expected_sha256 = None;
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "font_face_id" => {
                            let decoded = map_primitive(&mut map, context, "font_face_id")?;
                            set_field(&mut font_face_id, decoded, context, "font_face_id")?;
                        }
                        "family" => {
                            let decoded: String = map_primitive(&mut map, context, "family")?;
                            if decoded.is_empty() {
                                return Err(context.fail_child(
                                    DecodePathSegment::Static("family"),
                                    DocumentPackageTypedDecodeErrorKind::InvalidValue,
                                    DocumentPackageDecodePrimary::Value,
                                ));
                            }
                            set_field(&mut family, decoded, context, "family")?;
                        }
                        "uri" => {
                            let decoded = map_primitive(&mut map, context, "uri")?;
                            set_field(&mut uri, decoded, context, "uri")?;
                        }
                        "face_index" => {
                            let decoded = map_primitive(&mut map, context, "face_index")?;
                            set_field(&mut face_index, decoded, context, "face_index")?;
                        }
                        "expected_sha256" => {
                            let decoded = map_option_decode::<_, WireSha256>(
                                &mut map,
                                context,
                                "expected_sha256",
                            )?
                            .map(|value| value.0);
                            set_field(&mut expected_sha256, decoded, context, "expected_sha256")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireFontFace {
                    font_face_id: required(font_face_id, context)?,
                    family: required(family, context)?,
                    uri: required(uri, context)?,
                    face_index: required(face_index, context)?,
                    expected_sha256: required(expected_sha256, context)?,
                })
            }
        }
        deserializer.deserialize_map(FontVisitor(context))
    }
}

impl<'de> Decode<'de> for WireImage {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        context.consume(Counter::Images)?;
        struct ImageVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for ImageVisitor<'_> {
            type Value = WireImage;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed image declaration")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut image_id, mut uri, mut expected_sha256) = (None, None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "image_id" => {
                            let decoded = map_primitive(&mut map, context, "image_id")?;
                            set_field(&mut image_id, decoded, context, "image_id")?;
                        }
                        "uri" => {
                            let decoded = map_primitive(&mut map, context, "uri")?;
                            set_field(&mut uri, decoded, context, "uri")?;
                        }
                        "expected_sha256" => {
                            let decoded = map_option_decode::<_, WireSha256>(
                                &mut map,
                                context,
                                "expected_sha256",
                            )?
                            .map(|value| value.0);
                            set_field(&mut expected_sha256, decoded, context, "expected_sha256")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireImage {
                    image_id: required(image_id, context)?,
                    uri: required(uri, context)?,
                    expected_sha256: required(expected_sha256, context)?,
                })
            }
        }
        deserializer.deserialize_map(ImageVisitor(context))
    }
}

impl<'de> Decode<'de> for WireResourceCatalog {
    fn decode<D: de::Deserializer<'de>>(
        context: &mut DecodeContext,
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct ResourcesVisitor<'a>(&'a mut DecodeContext);
        impl<'de> Visitor<'de> for ResourcesVisitor<'_> {
            type Value = WireResourceCatalog;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a closed resource catalog")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let context = self.0;
                let (mut font_faces, mut images) = (None, None);
                while let Some(field) = map.next_key::<String>()? {
                    match field.as_str() {
                        "font_faces" => {
                            let decoded = map_vec_decode(&mut map, context, "font_faces")?;
                            set_field(&mut font_faces, decoded, context, "font_faces")?;
                        }
                        "images" => {
                            let decoded = map_vec_decode(&mut map, context, "images")?;
                            set_field(&mut images, decoded, context, "images")?;
                        }
                        _ => return Err(unknown_field(context, field)),
                    }
                }
                Ok(WireResourceCatalog {
                    font_faces: required(font_faces, context)?,
                    images: required(images, context)?,
                })
            }
        }
        deserializer.deserialize_map(ResourcesVisitor(context))
    }
}

fn tracked_path(path: serde_path_to_error::Path) -> Vec<DecodePathSegment> {
    path.iter()
        .filter_map(|segment| match segment {
            serde_path_to_error::Segment::Seq { index } => Some(DecodePathSegment::Index(*index)),
            serde_path_to_error::Segment::Map { key }
            | serde_path_to_error::Segment::Enum { variant: key } => {
                Some(DecodePathSegment::Owned(key.clone()))
            }
            serde_path_to_error::Segment::Unknown => None,
        })
        .collect()
}

/// serde_json reports a one-based line and column. Convert it by inspecting
/// the admitted raw bytes only on the failure path; no offset table is kept.
fn line_column_to_offset(input: &[u8], line: usize, column: usize) -> u64 {
    if line == 0 {
        return 0;
    }
    let mut current_line = 1usize;
    let mut line_start = 0usize;
    for (index, byte) in input.iter().enumerate() {
        if current_line == line {
            line_start = index;
            break;
        }
        if *byte == b'\n' {
            current_line += 1;
            line_start = index + 1;
        }
    }
    if current_line != line {
        return to_u64(input.len());
    }
    let zero_based_column = column.saturating_sub(1);
    to_u64(
        line_start
            .saturating_add(zero_based_column)
            .min(input.len()),
    )
}

struct RawJsonLocator<'a> {
    bytes: &'a [u8],
    position: usize,
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use typaxis_core::ResourceLimits;

    const MINIMAL: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/minimal/document-package.json"
    ));
    const RICH: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/conformance/document-rich.json"
    ));

    fn validated_limits() -> ValidatedResourceLimits {
        ValidatedResourceLimits::new(ResourceLimits::default()).expect("default limits")
    }

    fn decode(
        input: &[u8],
        limits: &ValidatedResourceLimits,
    ) -> Result<DecodedDocumentPackage, DocumentPackageDecodeError> {
        StrictDocumentPackageDecoder::new().decode(input, &DocumentPackageDecodePolicy::new(limits))
    }

    fn encode(package: &WireDocumentPackage) -> Vec<u8> {
        DocumentPackageEncoder::default()
            .to_jcs_vec(package)
            .expect("test wire package is encodable")
    }

    fn minimal_wire() -> WireDocumentPackage {
        let limits = validated_limits();
        decode(MINIMAL, &limits)
            .expect("minimal fixture")
            .wire()
            .clone()
    }

    fn limits_with(update: impl FnOnce(&mut ResourceLimits)) -> ValidatedResourceLimits {
        let mut limits = ResourceLimits::default();
        update(&mut limits);
        ValidatedResourceLimits::new(limits).expect("test limits are internally valid")
    }

    fn typed_error(error: &DocumentPackageDecodeError) -> &DocumentPackageTypedDecodeError {
        error.typed_error().expect("typed decode error")
    }

    fn assert_limit_error(
        result: Result<DecodedDocumentPackage, DocumentPackageDecodeError>,
        limit_kind: DocumentPackageDecodeLimit,
        limit: u64,
        attempted: u64,
        pointer: &str,
    ) {
        let error = result.expect_err("limit must reject max + 1");
        let typed = typed_error(&error);
        assert_eq!(
            typed.kind(),
            DocumentPackageTypedDecodeErrorKind::LimitExceeded {
                limit_kind,
                limit,
                attempted,
            }
        );
        assert_eq!(typed.location().json_pointer().as_str(), pointer);
        assert_eq!(
            typed.location().primary(),
            DocumentPackageDecodePrimary::Value
        );
    }

    #[test]
    fn decoder_accepts_schema_positive_fixtures_and_all_compatible_contracts() {
        let limits = validated_limits();
        let minimal = decode(MINIMAL, &limits).expect("minimal Schema fixture");
        assert_eq!(
            minimal.wire().contract,
            DocumentPackageContractId::CONTRACT_1_2
        );
        decode(RICH, &limits).expect("rich Schema fixture");

        let current = String::from_utf8(MINIMAL.to_vec()).expect("fixture UTF-8");
        for (wire_contract, contract, canonical_sha256) in [
            (
                "typaxis.contract/1.0",
                DocumentPackageContractId::CONTRACT_1_0,
                "797e2522187b12d48c47866fa13cde09819817d4fd61748d352564f563932152",
            ),
            (
                "typaxis.contract/1.1",
                DocumentPackageContractId::CONTRACT_1_1,
                "8f01dd947733db349290d235624f9d979b2a6dc9be56657cc85c795c0e40ac8c",
            ),
        ] {
            let compatible = current.replacen("typaxis.contract/1.2", wire_contract, 1);
            let decoded = decode(compatible.as_bytes(), &limits).expect("compatibility input");
            assert_eq!(decoded.wire().contract, contract);
            assert_eq!(decoded.canonical_jcs_sha256().to_string(), canonical_sha256);
        }
    }

    #[test]
    fn decoder_round_trips_every_recursive_wire_variant() {
        let span = WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let text_span = WireTextSpan {
            text_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let text = || WireInline::Text {
            node_id: 10,
            span,
            text_span,
        };
        let children = vec![
            text(),
            WireInline::Emphasis {
                node_id: 11,
                span,
                children: vec![WireInline::SoftBreak { node_id: 12, span }],
            },
            WireInline::Strong {
                node_id: 13,
                span,
                children: vec![WireInline::HardBreak { node_id: 14, span }],
            },
            WireInline::Link {
                node_id: 15,
                span,
                target: WireLinkTarget::Internal {
                    anchor_id: "anchor".to_owned(),
                },
                children: vec![text()],
            },
            WireInline::Link {
                node_id: 16,
                span,
                target: WireLinkTarget::Uri {
                    uri: "https://example.invalid/".to_owned(),
                },
                children: Vec::new(),
            },
            WireInline::Anchor {
                node_id: 17,
                span,
                anchor_id: "anchor".to_owned(),
            },
            WireInline::Reference {
                node_id: 18,
                span,
                target: "anchor".to_owned(),
                format: WireReferenceFormat::Text,
            },
            WireInline::Reference {
                node_id: 19,
                span,
                target: "anchor".to_owned(),
                format: WireReferenceFormat::Page,
            },
            WireInline::Reference {
                node_id: 20,
                span,
                target: "anchor".to_owned(),
                format: WireReferenceFormat::Number,
            },
            WireInline::FootnoteReference {
                node_id: 21,
                span,
                footnote_id: "note".to_owned(),
            },
            WireInline::SoftBreak { node_id: 22, span },
            WireInline::HardBreak { node_id: 23, span },
        ];
        let page_break = |node_id| WireBlock::PageBreak {
            node_id,
            span,
            classes: Vec::new(),
        };
        let mut package = minimal_wire();
        package.text_buffers = vec![WireTextBuffer {
            text_id: 0,
            utf8: String::new(),
            mappings: vec![
                WireTextMapSegment {
                    text_range: WireByteRange {
                        start_byte: 0,
                        end_byte: 0,
                    },
                    kind: WireTextMapKind::Identity,
                    source_span: Some(span),
                },
                WireTextMapSegment {
                    text_range: WireByteRange {
                        start_byte: 0,
                        end_byte: 0,
                    },
                    kind: WireTextMapKind::Replacement,
                    source_span: Some(span),
                },
                WireTextMapSegment {
                    text_range: WireByteRange {
                        start_byte: 0,
                        end_byte: 0,
                    },
                    kind: WireTextMapKind::Inserted,
                    source_span: None,
                },
            ],
        }];
        package.document.blocks = vec![
            WireBlock::Paragraph {
                node_id: 1,
                span,
                classes: vec!["body".to_owned()],
                children,
            },
            WireBlock::Heading {
                node_id: 2,
                span,
                classes: Vec::new(),
                level: 6,
                anchor_id: None,
                children: vec![text()],
            },
            WireBlock::List {
                node_id: 3,
                span,
                classes: Vec::new(),
                ordered: true,
                start: Some(u32::MAX),
                items: vec![WireListItem {
                    node_id: 30,
                    span,
                    blocks: vec![page_break(31)],
                }],
            },
            WireBlock::Table {
                node_id: 4,
                span,
                classes: Vec::new(),
                columns: vec![
                    WireTableColumn::Fixed { width: 1 },
                    WireTableColumn::Fraction { weight: u16::MAX },
                ],
                head: vec![WireTableRow {
                    node_id: 40,
                    span,
                    cells: vec![WireTableCell {
                        node_id: 41,
                        span,
                        colspan: u16::MAX,
                        rowspan: 1,
                        blocks: vec![page_break(42)],
                    }],
                }],
                body: Vec::new(),
            },
            WireBlock::Figure {
                node_id: 5,
                span,
                classes: Vec::new(),
                image_id: u32::MAX,
                alt: "alternative".to_owned(),
                caption: vec![page_break(50)],
            },
            page_break(6),
        ];
        package.document.footnotes = vec![WireFootnote {
            footnote_id: "note".to_owned(),
            node_id: 60,
            span,
            blocks: vec![page_break(61)],
        }];
        package.style_sheet.rules = vec![WireStyleRule {
            style_id: "rule".to_owned(),
            extends: None,
            selector: "paragraph".to_owned(),
            source_order: 0,
            declarations: vec![
                WireDeclaration {
                    name: WireDeclarationName::FontFamily,
                    value: WireStyleValue::FontFamilyList {
                        families: vec!["Family".to_owned()],
                    },
                    important: false,
                },
                WireDeclaration {
                    name: WireDeclarationName::FontSize,
                    value: WireStyleValue::Length { value: 1 },
                    important: true,
                },
                WireDeclaration {
                    name: WireDeclarationName::LineHeight,
                    value: WireStyleValue::Length {
                        value: JSON_SAFE_INTEGER_MAX,
                    },
                    important: false,
                },
                WireDeclaration {
                    name: WireDeclarationName::Page,
                    value: WireStyleValue::Keyword {
                        value: "auto".to_owned(),
                    },
                    important: false,
                },
                WireDeclaration {
                    name: WireDeclarationName::Page,
                    value: WireStyleValue::String {
                        value: "named".to_owned(),
                    },
                    important: false,
                },
            ],
        }];
        let frame = WireRect {
            x: -1,
            y: JSON_SAFE_INTEGER_MAX,
            width: 1,
            height: JSON_SAFE_INTEGER_MAX,
        };
        package.page_masters.masters[0].header = Some(frame);
        package.page_masters.masters[0].footer = Some(frame);
        package.page_masters.masters[0].footnote = Some(frame);
        package.page_masters.selection_rules = vec![
            WirePageMasterRule {
                master_id: "a4".to_owned(),
                parity: WirePageParity::Any,
                first: None,
                named_page: None,
                source_order: 0,
            },
            WirePageMasterRule {
                master_id: "a4".to_owned(),
                parity: WirePageParity::Odd,
                first: Some(true),
                named_page: Some("named".to_owned()),
                source_order: 1,
            },
            WirePageMasterRule {
                master_id: "a4".to_owned(),
                parity: WirePageParity::Even,
                first: Some(false),
                named_page: None,
                source_order: 2,
            },
        ];
        package.resources.font_faces = vec![WireFontFace {
            font_face_id: u32::MAX,
            family: "Family".to_owned(),
            uri: "font.ttf".to_owned(),
            face_index: u32::MAX,
            expected_sha256: Some([0xab; 32]),
        }];
        package.resources.images = vec![WireImage {
            image_id: u32::MAX,
            uri: "image.png".to_owned(),
            expected_sha256: Some([0xcd; 32]),
        }];

        let limits = validated_limits();
        let decoded = decode(&encode(&package), &limits).expect("all wire variants");
        assert_eq!(decoded.wire(), &package);
    }

    #[test]
    fn canonical_hash_ignores_formatting_but_raw_hash_does_not() {
        let limits = validated_limits();
        let pretty = decode(MINIMAL, &limits).expect("pretty fixture");
        let canonical_bytes = DocumentPackageEncoder::default()
            .to_jcs_vec(pretty.wire())
            .expect("canonical fixture");
        let canonical = decode(&canonical_bytes, &limits).expect("canonical fixture decode");
        assert_ne!(pretty.raw_sha256(), canonical.raw_sha256());
        assert_eq!(
            pretty.canonical_jcs_sha256(),
            canonical.canonical_jcs_sha256()
        );

        let mut changed_wire = pretty.wire().clone();
        changed_wire.page_masters.masters[0].width += 1;
        let changed_bytes = DocumentPackageEncoder::default()
            .to_jcs_vec(&changed_wire)
            .expect("changed fixture");
        let changed = decode(&changed_bytes, &limits).expect("changed fixture decode");
        assert_ne!(
            pretty.canonical_jcs_sha256(),
            changed.canonical_jcs_sha256()
        );
    }

    #[test]
    fn typed_errors_have_closed_kinds_and_primary_token_locations() {
        let limits = validated_limits();
        let canonical = String::from_utf8(encode(&minimal_wire())).expect("canonical UTF-8");

        let unknown_contract =
            canonical.replacen("typaxis.contract/1.2", "typaxis.contract/9.9", 1);
        let error = decode(unknown_contract.as_bytes(), &limits).expect_err("unknown contract");
        let typed = typed_error(&error);
        assert_eq!(
            typed.kind(),
            DocumentPackageTypedDecodeErrorKind::UnknownContract
        );
        assert_eq!(typed.class(), DocumentPackageDecodeErrorClass::Contract);
        assert_eq!(typed.location().json_pointer().as_str(), "/contract");
        assert_eq!(
            typed.location().byte_offset(),
            to_u64(unknown_contract.find("\"typaxis.contract/9.9\"").unwrap())
        );

        let unknown_unit = canonical.replacen("pdf_point_1_65536", "css_pixel_________", 1);
        let error = decode(unknown_unit.as_bytes(), &limits).expect_err("unknown unit");
        let typed = typed_error(&error);
        assert_eq!(
            typed.kind(),
            DocumentPackageTypedDecodeErrorKind::UnknownCoordinateUnit
        );
        assert_eq!(typed.class(), DocumentPackageDecodeErrorClass::Contract);
        assert_eq!(typed.location().json_pointer().as_str(), "/coordinate_unit");

        let unknown = canonical.replacen("\"source_id\":0", "\"mystery\":0,\"source_id\":0", 1);
        let error = decode(unknown.as_bytes(), &limits).expect_err("unknown nested field");
        let typed = typed_error(&error);
        assert_eq!(
            typed.kind(),
            DocumentPackageTypedDecodeErrorKind::UnknownField
        );
        assert_eq!(
            typed.location().json_pointer().as_str(),
            "/sources/0/mystery"
        );
        assert_eq!(
            typed.location().primary(),
            DocumentPackageDecodePrimary::Key
        );
        assert_eq!(
            typed.location().byte_offset(),
            to_u64(unknown.find("\"mystery\"").unwrap())
        );

        let missing = canonical.replacen("\"source_id\":0,", "", 1);
        let error = decode(missing.as_bytes(), &limits).expect_err("missing nested field");
        let typed = typed_error(&error);
        assert_eq!(
            typed.kind(),
            DocumentPackageTypedDecodeErrorKind::MissingField
        );
        assert_eq!(typed.location().json_pointer().as_str(), "/sources/0");
        assert_eq!(
            typed.location().primary(),
            DocumentPackageDecodePrimary::ContainingObject
        );
        let source_object = missing.find("\"sources\":[{").unwrap() + "\"sources\":[".len();
        assert_eq!(typed.location().byte_offset(), to_u64(source_object));

        for invalid_integer in ["1.0", "1e0", "-1", "4294967296"] {
            let invalid = canonical.replacen(
                "\"source_id\":0",
                &format!("\"source_id\":{invalid_integer}"),
                1,
            );
            let error = decode(invalid.as_bytes(), &limits).expect_err("non-u32 integer");
            let typed = typed_error(&error);
            assert!(matches!(
                typed.kind(),
                DocumentPackageTypedDecodeErrorKind::TypeMismatch
                    | DocumentPackageTypedDecodeErrorKind::IntegerOutOfRange
            ));
            assert_eq!(
                typed.location().json_pointer().as_str(),
                "/sources/0/source_id"
            );
            assert_eq!(
                typed.location().primary(),
                DocumentPackageDecodePrimary::Value
            );
            assert_eq!(
                typed.location().byte_offset(),
                to_u64(
                    invalid
                        .find(&format!("\"source_id\":{invalid_integer}"))
                        .unwrap()
                        + "\"source_id\":".len()
                )
            );
        }
    }

    #[test]
    fn decoder_limits_and_locations() {
        let base = minimal_wire();

        let source_limits = limits_with(|limits| limits.max_include_files = 1);
        let mut exact_sources = base.clone();
        exact_sources.sources.push(exact_sources.sources[0].clone());
        decode(&encode(&exact_sources), &source_limits).expect("source exact max");
        let mut too_many_sources = exact_sources;
        too_many_sources
            .sources
            .push(too_many_sources.sources[0].clone());
        assert_limit_error(
            decode(&encode(&too_many_sources), &source_limits),
            DocumentPackageDecodeLimit::Sources,
            2,
            3,
            "/sources/2",
        );

        let ast_limits = limits_with(|limits| limits.max_ast_nodes = 1);
        decode(&encode(&base), &ast_limits).expect("AST exact max");
        let mut too_many_nodes = base.clone();
        too_many_nodes.document.blocks.push(WireBlock::PageBreak {
            node_id: u32::MAX,
            span: WireSourceSpan {
                source_id: 0,
                start_byte: 0,
                end_byte: 0,
            },
            classes: Vec::new(),
        });
        assert_limit_error(
            decode(&encode(&too_many_nodes), &ast_limits),
            DocumentPackageDecodeLimit::AstNodes,
            1,
            2,
            "/document/blocks/0",
        );

        let mut exact_text = base.clone();
        exact_text.text_buffers.push(WireTextBuffer {
            text_id: u32::MAX,
            utf8: "a".to_owned(),
            mappings: Vec::new(),
        });
        decode(&encode(&exact_text), &ast_limits).expect("text-buffer exact max");
        let mut too_many_text = exact_text.clone();
        too_many_text.text_buffers.push(WireTextBuffer {
            text_id: 1,
            utf8: String::new(),
            mappings: Vec::new(),
        });
        assert_limit_error(
            decode(&encode(&too_many_text), &ast_limits),
            DocumentPackageDecodeLimit::TextBuffers,
            1,
            2,
            "/text_buffers/1",
        );

        let text_byte_limits = limits_with(|limits| {
            limits.max_text_buffer_bytes = 1;
            limits.max_shaping_context_bytes = 1;
            limits.max_text_bytes = 2;
        });
        decode(&encode(&exact_text), &text_byte_limits).expect("per-text exact max");
        let mut too_many_text_bytes = exact_text.clone();
        too_many_text_bytes.text_buffers[0].utf8 = "ab".to_owned();
        assert_limit_error(
            decode(&encode(&too_many_text_bytes), &text_byte_limits),
            DocumentPackageDecodeLimit::TextBufferBytes,
            1,
            2,
            "/text_buffers/0/utf8",
        );

        let aggregate_limits = limits_with(|limits| {
            limits.max_ast_nodes = 2;
            limits.max_text_buffer_bytes = 1;
            limits.max_shaping_context_bytes = 1;
            limits.max_text_bytes = 1;
        });
        decode(&encode(&exact_text), &aggregate_limits).expect("aggregate exact max");
        let mut aggregate_plus_one = exact_text.clone();
        aggregate_plus_one.text_buffers.push(WireTextBuffer {
            text_id: 1,
            utf8: "b".to_owned(),
            mappings: Vec::new(),
        });
        assert_limit_error(
            decode(&encode(&aggregate_plus_one), &aggregate_limits),
            DocumentPackageDecodeLimit::AggregateTextBytes,
            1,
            2,
            "/text_buffers/1/utf8",
        );

        let style_limits = limits_with(|limits| limits.max_style_rules = 1);
        let rule = WireStyleRule {
            style_id: "same".to_owned(),
            extends: None,
            selector: "paragraph".to_owned(),
            source_order: u32::MAX,
            declarations: Vec::new(),
        };
        let mut exact_styles = base.clone();
        exact_styles.style_sheet.rules.push(rule.clone());
        decode(&encode(&exact_styles), &style_limits).expect("style exact max");
        let mut too_many_styles = exact_styles.clone();
        too_many_styles.style_sheet.rules.push(rule.clone());
        assert_limit_error(
            decode(&encode(&too_many_styles), &style_limits),
            DocumentPackageDecodeLimit::StyleRules,
            1,
            2,
            "/style_sheet/rules/1",
        );

        decode(&encode(&base), &style_limits).expect("master exact max");
        let mut too_many_masters = base.clone();
        too_many_masters
            .page_masters
            .masters
            .push(too_many_masters.page_masters.masters[0].clone());
        assert_limit_error(
            decode(&encode(&too_many_masters), &style_limits),
            DocumentPackageDecodeLimit::PageMasters,
            1,
            2,
            "/page_masters/masters/1",
        );

        let resource_limits = limits_with(|limits| {
            limits.max_fonts = 1;
            limits.max_images = 1;
        });
        let font = WireFontFace {
            font_face_id: u32::MAX,
            family: "Test".to_owned(),
            uri: "font.ttf".to_owned(),
            face_index: 0,
            expected_sha256: None,
        };
        let image = WireImage {
            image_id: u32::MAX,
            uri: "image.png".to_owned(),
            expected_sha256: None,
        };
        let mut exact_resources = base.clone();
        exact_resources.resources.font_faces.push(font.clone());
        exact_resources.resources.images.push(image.clone());
        decode(&encode(&exact_resources), &resource_limits).expect("resource exact max");
        let mut too_many_fonts = exact_resources.clone();
        too_many_fonts.resources.font_faces.push(font);
        assert_limit_error(
            decode(&encode(&too_many_fonts), &resource_limits),
            DocumentPackageDecodeLimit::FontFaces,
            1,
            2,
            "/resources/font_faces/1",
        );
        let mut too_many_images = exact_resources;
        too_many_images.resources.images.push(image);
        assert_limit_error(
            decode(&encode(&too_many_images), &resource_limits),
            DocumentPackageDecodeLimit::Images,
            1,
            2,
            "/resources/images/1",
        );

        let mut indexed = base.clone();
        indexed.sources.push(indexed.sources[0].clone());
        indexed.sources[0].source_id = u32::MAX;
        indexed.sources[1].source_id = u32::MAX;
        indexed.text_buffers = vec![WireTextBuffer {
            text_id: u32::MAX,
            utf8: String::new(),
            mappings: vec![WireTextMapSegment {
                text_range: WireByteRange {
                    start_byte: 0,
                    end_byte: 0,
                },
                kind: WireTextMapKind::Inserted,
                source_span: None,
            }],
        }];
        indexed.document.node_id = u32::MAX;
        indexed.document.blocks.push(WireBlock::PageBreak {
            node_id: u32::MAX,
            span: WireSourceSpan {
                source_id: 0,
                start_byte: 0,
                end_byte: 0,
            },
            classes: Vec::new(),
        });
        let declaration = WireDeclaration {
            name: WireDeclarationName::Page,
            value: WireStyleValue::Keyword {
                value: "auto".to_owned(),
            },
            important: false,
        };
        indexed.style_sheet.rules = vec![
            WireStyleRule {
                declarations: vec![declaration.clone()],
                ..rule.clone()
            },
            WireStyleRule {
                declarations: vec![declaration],
                ..rule
            },
        ];
        indexed
            .page_masters
            .masters
            .push(indexed.page_masters.masters[0].clone());
        indexed.resources.font_faces = vec![
            WireFontFace {
                font_face_id: u32::MAX,
                family: "Test".to_owned(),
                uri: "a.ttf".to_owned(),
                face_index: 0,
                expected_sha256: None,
            },
            WireFontFace {
                font_face_id: u32::MAX,
                family: "Test".to_owned(),
                uri: "b.ttf".to_owned(),
                face_index: 0,
                expected_sha256: None,
            },
        ];
        indexed.resources.images = vec![
            WireImage {
                image_id: u32::MAX,
                uri: "a.png".to_owned(),
                expected_sha256: None,
            },
            WireImage {
                image_id: u32::MAX,
                uri: "b.png".to_owned(),
                expected_sha256: None,
            },
        ];
        let decoded = decode(&encode(&indexed), &validated_limits()).expect("sparse ID index");
        let locations = decoded.locations();
        assert_eq!(
            locations
                .root_member(DocumentPackageRootMember::Resources)
                .as_str(),
            "/resources"
        );
        assert_eq!(
            locations.source(u32::MAX, 1).unwrap().as_str(),
            "/sources/1"
        );
        assert_eq!(
            locations.text_mapping(u32::MAX, 0, 0).unwrap().as_str(),
            "/text_buffers/0/mappings/0"
        );
        assert_eq!(locations.node(u32::MAX, 0).unwrap().as_str(), "/document");
        assert_eq!(
            locations.node(u32::MAX, 1).unwrap().as_str(),
            "/document/blocks/0"
        );
        assert_eq!(
            locations.style_rule("same", 1).unwrap().as_str(),
            "/style_sheet/rules/1"
        );
        assert_eq!(
            locations.style_declaration("same", 1, 0).unwrap().as_str(),
            "/style_sheet/rules/1/declarations/0"
        );
        assert_eq!(
            locations.page_master("default", 1).unwrap().as_str(),
            "/page_masters/masters/1"
        );
        assert_eq!(
            locations.font_face(u32::MAX, 1).unwrap().as_str(),
            "/resources/font_faces/1"
        );
        assert_eq!(
            locations.image(u32::MAX, 1).unwrap().as_str(),
            "/resources/images/1"
        );
    }

    #[test]
    fn decoder_table_columns_consume_ast_units_at_exact_and_plus_one() {
        let span = WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let mut table = minimal_wire();
        table.document.blocks.push(WireBlock::Table {
            node_id: 1,
            span,
            classes: Vec::new(),
            columns: vec![
                WireTableColumn::Fixed { width: 1 },
                WireTableColumn::Fraction { weight: 1 },
            ],
            head: Vec::new(),
            body: vec![WireTableRow {
                node_id: 2,
                span,
                cells: vec![WireTableCell {
                    node_id: 3,
                    span,
                    colspan: 2,
                    rowspan: 1,
                    blocks: Vec::new(),
                }],
            }],
        });
        let exact = limits_with(|limits| limits.max_ast_nodes = 6);
        decode(&encode(&table), &exact).expect("root/table/row/cell/two-column exact max");

        let plus_one = limits_with(|limits| limits.max_ast_nodes = 5);
        assert_limit_error(
            decode(&encode(&table), &plus_one),
            DocumentPackageDecodeLimit::AstNodes,
            5,
            6,
            "/document/blocks/0/columns/1",
        );
    }

    #[test]
    fn current_machine_properties_are_exact_for_both_encoder_entry_points() {
        let mut package = minimal_wire();
        package.style_sheet.rules = vec![WireStyleRule {
            style_id: "typed".to_owned(),
            extends: None,
            selector: "paragraph".to_owned(),
            source_order: 0,
            declarations: vec![
                WireDeclaration {
                    name: WireDeclarationName::SpaceBefore,
                    value: WireStyleValue::Length { value: 0 },
                    important: false,
                },
                WireDeclaration {
                    name: WireDeclarationName::SpaceAfter,
                    value: WireStyleValue::Length {
                        value: JSON_SAFE_INTEGER_MAX,
                    },
                    important: false,
                },
                WireDeclaration {
                    name: WireDeclarationName::StartIndent,
                    value: WireStyleValue::Length { value: 1 },
                    important: false,
                },
                WireDeclaration {
                    name: WireDeclarationName::EndIndent,
                    value: WireStyleValue::Length { value: 2 },
                    important: false,
                },
                WireDeclaration {
                    name: WireDeclarationName::TextAlign,
                    value: WireStyleValue::Keyword {
                        value: "center".to_owned(),
                    },
                    important: false,
                },
                WireDeclaration {
                    name: WireDeclarationName::Width,
                    value: WireStyleValue::Keyword {
                        value: "auto".to_owned(),
                    },
                    important: false,
                },
                WireDeclaration {
                    name: WireDeclarationName::KeepWithNext,
                    value: WireStyleValue::Boolean { value: true },
                    important: false,
                },
                WireDeclaration {
                    name: WireDeclarationName::KeepCaption,
                    value: WireStyleValue::Boolean { value: false },
                    important: false,
                },
            ],
        }];
        let current_bytes = DocumentPackageEncoder::default()
            .to_jcs_vec(&package)
            .unwrap();
        let bytes = StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&package)
            .unwrap();
        assert_eq!(bytes, current_bytes);
        let canonical = String::from_utf8(bytes.clone()).unwrap();
        assert!(canonical.starts_with("{\"contract\":\"typaxis.contract/1.2\""));
        let limits = validated_limits();
        let policy = DocumentPackageDecodePolicy::new(&limits);
        let decoded = StagingStyleDocumentPackageDecoder::new()
            .decode(&bytes, &policy)
            .unwrap();
        assert_eq!(decoded.wire().style_sheet.rules[0].declarations.len(), 8);
        let decoded = StrictDocumentPackageDecoder::new()
            .decode(&bytes, &policy)
            .unwrap();
        assert_eq!(decoded.wire().style_sheet.rules[0].declarations.len(), 8);

        for invalid in [
            canonical.replacen("9007199254740991", "9007199254740992", 1),
            canonical.replacen("\"space_before\"", "\"future_property\"", 1),
            canonical.replacen(
                "\"kind\":\"length\",\"value\":0",
                "\"kind\":\"integer\",\"value\":0",
                1,
            ),
        ] {
            let error = StrictDocumentPackageDecoder::new()
                .decode(invalid.as_bytes(), &policy)
                .unwrap_err();
            let pointer = typed_error(&error).location().json_pointer().as_str();
            assert!(
                pointer.starts_with("/style_sheet/rules/0/declarations/"),
                "unexpected pointer {pointer}"
            );
        }
    }
}

impl<'a> RawJsonLocator<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn locate(
        mut self,
        path: &[DecodePathSegment],
        primary: DocumentPackageDecodePrimary,
    ) -> Option<usize> {
        self.locate_value(path, primary, 0)
    }

    fn locate_value(
        &mut self,
        path: &[DecodePathSegment],
        primary: DocumentPackageDecodePrimary,
        depth: u16,
    ) -> Option<usize> {
        if depth > MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH {
            return None;
        }
        self.skip_whitespace();
        let value_start = self.position;
        if path.is_empty() {
            return Some(value_start);
        }
        match (&path[0], self.bytes.get(self.position).copied()) {
            (DecodePathSegment::Static(_) | DecodePathSegment::Owned(_), Some(b'{')) => {
                self.position += 1;
                self.skip_whitespace();
                if self.bytes.get(self.position) == Some(&b'}') {
                    return None;
                }
                loop {
                    self.skip_whitespace();
                    let key_start = self.position;
                    let key_end = self.scan_string()?;
                    let key: String =
                        serde_json::from_slice(&self.bytes[key_start..key_end]).ok()?;
                    self.skip_whitespace();
                    if self.bytes.get(self.position) != Some(&b':') {
                        return None;
                    }
                    self.position += 1;
                    self.skip_whitespace();
                    if path[0].matches_key(&key) {
                        if path.len() == 1 && primary == DocumentPackageDecodePrimary::Key {
                            return Some(key_start);
                        }
                        return self.locate_value(&path[1..], primary, depth + 1);
                    }
                    self.skip_value(depth + 1)?;
                    self.skip_whitespace();
                    match self.bytes.get(self.position) {
                        Some(b',') => self.position += 1,
                        Some(b'}') => return None,
                        _ => return None,
                    }
                }
            }
            (DecodePathSegment::Index(wanted), Some(b'[')) => {
                self.position += 1;
                self.skip_whitespace();
                if self.bytes.get(self.position) == Some(&b']') {
                    return None;
                }
                let mut ordinal = 0usize;
                loop {
                    self.skip_whitespace();
                    if ordinal == *wanted {
                        return self.locate_value(&path[1..], primary, depth + 1);
                    }
                    self.skip_value(depth + 1)?;
                    ordinal = ordinal.checked_add(1)?;
                    self.skip_whitespace();
                    match self.bytes.get(self.position) {
                        Some(b',') => self.position += 1,
                        Some(b']') => return None,
                        _ => return None,
                    }
                }
            }
            _ => None,
        }
    }

    fn skip_value(&mut self, depth: u16) -> Option<()> {
        if depth > MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH {
            return None;
        }
        self.skip_whitespace();
        match self.bytes.get(self.position).copied()? {
            b'"' => {
                self.scan_string()?;
            }
            b'{' => {
                self.position += 1;
                self.skip_whitespace();
                if self.bytes.get(self.position) == Some(&b'}') {
                    self.position += 1;
                    return Some(());
                }
                loop {
                    self.skip_whitespace();
                    self.scan_string()?;
                    self.skip_whitespace();
                    if self.bytes.get(self.position) != Some(&b':') {
                        return None;
                    }
                    self.position += 1;
                    self.skip_value(depth + 1)?;
                    self.skip_whitespace();
                    match self.bytes.get(self.position) {
                        Some(b',') => self.position += 1,
                        Some(b'}') => {
                            self.position += 1;
                            break;
                        }
                        _ => return None,
                    }
                }
            }
            b'[' => {
                self.position += 1;
                self.skip_whitespace();
                if self.bytes.get(self.position) == Some(&b']') {
                    self.position += 1;
                    return Some(());
                }
                loop {
                    self.skip_value(depth + 1)?;
                    self.skip_whitespace();
                    match self.bytes.get(self.position) {
                        Some(b',') => self.position += 1,
                        Some(b']') => {
                            self.position += 1;
                            break;
                        }
                        _ => return None,
                    }
                }
            }
            _ => {
                while let Some(byte) = self.bytes.get(self.position) {
                    if byte.is_ascii_whitespace() || matches!(byte, b',' | b']' | b'}') {
                        break;
                    }
                    self.position += 1;
                }
            }
        }
        Some(())
    }

    fn scan_string(&mut self) -> Option<usize> {
        if self.bytes.get(self.position) != Some(&b'"') {
            return None;
        }
        self.position += 1;
        while let Some(byte) = self.bytes.get(self.position).copied() {
            match byte {
                b'"' => {
                    self.position += 1;
                    return Some(self.position);
                }
                b'\\' => {
                    self.position = self.position.checked_add(2)?;
                    if self.position > self.bytes.len() {
                        return None;
                    }
                }
                _ => self.position += 1,
            }
        }
        None
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }
}
