#![forbid(unsafe_code)]

use core::num::{NonZeroU16, NonZeroU64};
use std::collections::BTreeMap;
#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use typaxis_core::{
    AdmittedResourceFingerprint, BuildExecutionContext, BuildExecutionError, EffectiveConfig,
    EffectiveConfigFingerprint, EngineIdentity, FontFaceId, HostPath, ImageResourceId,
    LayoutStateFingerprint, PortablePath, ReplacePolicy, ResolvedDataTables, ShaperIdentity,
    SourceId, ValidatedResourceLimits, JSON_SAFE_INTEGER_MAX,
};
pub use typaxis_core::{OutputSink, PdfStreamCompression};
use typaxis_pagination::{ConvergenceStatus, PaginationResult};
use typaxis_pdf::{PdfStreamWriteFacts, VerifiedPdfBytesReceipt};
use typaxis_resources::AdmittedResourceLedgerToken;
use typaxis_syntax::{PackageEpochIdentity, ValidatedParsedPackage};
use typaxis_text::SourceRecord;

pub const CONTRACT: &str = typaxis_core::CONTRACT;
pub const ENGINE_NAME: &str = typaxis_core::PRODUCT_NAME;
pub const PDF_PROFILE: &str = "pdf-1.7-classic-xref";

#[cfg(unix)]
static OUTPUT_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static OUTPUT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static PUBLICATION_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildStatus {
    Built,
    Failed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutStatus {
    Converged,
    CycleFallback,
    MaxPassFallback,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutFallbackPolicy {
    LowestCostThenEarliest,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LayoutRecord {
    status: LayoutStatus,
    pass_count: NonZeroU16,
    selected_state: NonZeroU16,
    final_fingerprint: LayoutStateFingerprint,
    fallback_policy: Option<LayoutFallbackPolicy>,
}
impl LayoutRecord {
    fn new(
        status: LayoutStatus,
        pass_count: NonZeroU16,
        selected_state: NonZeroU16,
        final_fingerprint: LayoutStateFingerprint,
    ) -> Option<Self> {
        if selected_state.get() > pass_count.get()
            || (status == LayoutStatus::Converged && selected_state != pass_count)
        {
            return None;
        }
        let fallback_policy = match status {
            LayoutStatus::Converged => None,
            LayoutStatus::CycleFallback | LayoutStatus::MaxPassFallback => {
                Some(LayoutFallbackPolicy::LowestCostThenEarliest)
            }
        };
        Some(Self {
            status,
            pass_count,
            selected_state,
            final_fingerprint,
            fallback_policy,
        })
    }

    fn from_pagination(result: &PaginationResult) -> Result<Self, BuildManifestError> {
        let pass_count = u16::try_from(result.passes().len())
            .ok()
            .and_then(NonZeroU16::new)
            .ok_or(BuildManifestError::LayoutPassCountMismatch)?;
        let selected_state = NonZeroU16::new(result.selected_state().get())
            .ok_or(BuildManifestError::LayoutPassCountMismatch)?;
        let status = match result.status() {
            ConvergenceStatus::Converged => LayoutStatus::Converged,
            ConvergenceStatus::CycleFallback { .. } => LayoutStatus::CycleFallback,
            ConvergenceStatus::MaxPassFallback => LayoutStatus::MaxPassFallback,
        };
        Self::new(
            status,
            pass_count,
            selected_state,
            result.final_fingerprint(),
        )
        .ok_or(BuildManifestError::LayoutPassCountMismatch)
    }
    pub const fn status(self) -> LayoutStatus {
        self.status
    }
    pub const fn pass_count(self) -> NonZeroU16 {
        self.pass_count
    }
    pub const fn selected_state(self) -> NonZeroU16 {
        self.selected_state
    }
    pub const fn final_fingerprint(self) -> LayoutStateFingerprint {
        self.final_fingerprint
    }
    pub const fn fallback_policy(self) -> Option<LayoutFallbackPolicy> {
        self.fallback_policy
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineRecord {
    name: String,
    version: String,
    rust_version: String,
    git_commit: Option<String>,
}
impl EngineRecord {
    pub fn from_identity(identity: &EngineIdentity) -> Self {
        Self {
            name: identity.name().to_owned(),
            version: identity.version().to_owned(),
            rust_version: identity.rust_version().to_owned(),
            git_commit: identity.git_commit().map(str::to_owned),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn rust_version(&self) -> &str {
        &self.rust_version
    }
    pub fn git_commit(&self) -> Option<&str> {
        self.git_commit.as_deref()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataVersions {
    unicode: String,
    japanese_line_break: String,
    shaper_backend: String,
    shaper_version: String,
}
impl DataVersions {
    fn from_runtime(tables: &ResolvedDataTables, shaper: ShaperIdentity) -> Self {
        let config = tables.versions();
        Self {
            unicode: config.unicode().to_owned(),
            japanese_line_break: config.japanese_line_break().to_owned(),
            shaper_backend: shaper.backend().to_owned(),
            shaper_version: shaper.version().to_owned(),
        }
    }
    pub fn unicode(&self) -> &str {
        &self.unicode
    }
    pub fn japanese_line_break(&self) -> &str {
        &self.japanese_line_break
    }
    pub fn shaper_backend(&self) -> &str {
        &self.shaper_backend
    }
    pub fn shaper_version(&self) -> &str {
        &self.shaper_version
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileRecord {
    uri: PortablePath,
    bytes: u64,
    sha256: [u8; 32],
}
impl FileRecord {
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontRecord {
    font_face_id: FontFaceId,
    uri: PortablePath,
    face_index: u32,
    bytes: u64,
    sha256: [u8; 32],
    units_per_em: u16,
    glyph_count: u32,
}
impl FontRecord {
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn units_per_em(&self) -> u16 {
        self.units_per_em
    }
    pub const fn glyph_count(&self) -> u32 {
        self.glyph_count
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageRecord {
    image_id: ImageResourceId,
    uri: PortablePath,
    bytes: u64,
    sha256: [u8; 32],
    pixel_width: u32,
    pixel_height: u32,
    decoded_bytes: u64,
}
impl ImageRecord {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn uri(&self) -> &PortablePath {
        &self.uri
    }
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn pixel_width(&self) -> u32 {
        self.pixel_width
    }
    pub const fn pixel_height(&self) -> u32 {
        self.pixel_height
    }
    pub const fn decoded_bytes(&self) -> u64 {
        self.decoded_bytes
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputRecord {
    sink: OutputSink,
    bytes: u64,
    sha256: [u8; 32],
    page_count: u32,
    pdf_object_count: u32,
}
impl OutputRecord {
    pub const fn sink(&self) -> OutputSink {
        self.sink
    }
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn pdf_object_count(&self) -> u32 {
        self.pdf_object_count
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildManifest {
    contract: String,
    status: BuildStatus,
    deterministic: bool,
    engine: EngineRecord,
    data_versions: DataVersions,
    config_sha256: [u8; 32],
    inputs: Vec<FileRecord>,
    fonts: Vec<FontRecord>,
    images: Vec<ImageRecord>,
    pdf_profile: String,
    stream_compression: PdfStreamCompression,
    layout: Option<LayoutRecord>,
    output: Option<OutputRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildManifestError {
    WrongContract,
    NonDeterministic,
    WrongEngineName,
    EmptyVersion,
    WrongPdfProfile,
    IntegerNotJsonSafe,
    BuiltRequiresLayoutAndOutput,
    NonBuiltMustNotHaveOutput,
    NonCanonicalInputs,
    NonCanonicalFonts,
    NonCanonicalImages,
    StreamCompressionMismatch,
    DataVersionMismatch,
    ConfigFingerprintMismatch,
    EmptyBuiltOutput,
    EmptyAdmittedResource,
    InputSourceLimit,
    InputAggregateLimit,
    FontCountLimit,
    FontBytesLimit,
    ImageCountLimit,
    ImageBytesLimit,
    ResourceAggregateLimit,
    PageLimit,
    OutputBytesLimit,
    PdfObjectLimit,
    MissingEntryInput,
    IncludeFileLimit,
    ImagePixelLimit,
    ImageDecodedBytesLimit,
    InvalidFontMetadata,
    OutputSinkMismatch,
    OutputReceiptBindingMismatch,
    AdmissionLedgerBindingMismatch,
    NonDenseAdmissionSource,
    DuplicateAdmissionRecord,
    PackageResourceMismatch,
    PackagePaginationMismatch,
    LayoutPassCountMismatch,
    LayoutLimit,
    PaginationReceiptMismatch,
    PdfGraphReceiptMismatch,
    IncompleteLayoutAdmission,
}

/// Per-build owner of the configured PDF sink. This context exists whether
/// or not a build manifest was requested. Its private session identity keeps
/// trusted output capabilities from different build executions separate even
/// when their configuration and host paths happen to be identical.
#[derive(Debug, Eq, PartialEq)]
pub struct BuildOutputCommitContext {
    session: OutputCommitSessionId,
    config_fingerprint: EffectiveConfigFingerprint,
    stream_compression: PdfStreamCompression,
    limits: ValidatedResourceLimits,
    execution: BuildExecutionContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildOutputCommitContextError {
    Execution(BuildExecutionError),
    SessionIdentityExhausted,
}

impl BuildOutputCommitContext {
    pub fn new(
        config: &EffectiveConfig,
        execution: &BuildExecutionContext,
    ) -> Result<Self, BuildOutputCommitContextError> {
        execution
            .revalidate_write_targets()
            .map_err(BuildOutputCommitContextError::Execution)?;
        Ok(Self {
            session: OutputCommitSessionId::allocate()?,
            config_fingerprint: config.fingerprint(),
            stream_compression: config.stream_compression(),
            limits: config.limits().clone(),
            execution: execution.clone(),
        })
    }

    pub const fn config_fingerprint(&self) -> EffectiveConfigFingerprint {
        self.config_fingerprint
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub const fn output_sink(&self) -> OutputSink {
        self.execution.output_sink()
    }
    pub const fn manifest_requested(&self) -> bool {
        self.execution.manifest_target().is_some()
    }

    /// Commits serializer-owned PDF bytes when no build manifest was
    /// requested. If a manifest target is configured, callers must instead
    /// obtain a sealed `PreparedBuiltPublication` so a terminal built record
    /// is completely preflighted before the PDF becomes visible.
    ///
    /// Arbitrary bytes and success callbacks cannot enter this API:
    ///
    /// ```compile_fail
    /// # use typaxis_manifest::BuildOutputCommitContext;
    /// # fn forge(output: &BuildOutputCommitContext) {
    /// let _ = output.commit_pdf_without_manifest(b"caller bytes".to_vec());
    /// let _ = output.clone();
    /// # }
    /// ```
    pub fn commit_pdf_without_manifest(
        self,
        pdf: VerifiedPdfBytesReceipt,
    ) -> Result<PdfSinkCommitReceipt, PdfSinkCommitError> {
        if self.manifest_requested() {
            return Err(PdfSinkCommitError::ManifestPreflightRequired);
        }
        let facts = validate_standalone_pdf_output_facts(&self, &pdf)
            .map_err(PdfSinkCommitError::InvalidFacts)?;
        let durability = commit_verified_pdf(&self.execution, &pdf)?;
        let receipt = self.issue_receipt(facts);
        match durability {
            SinkCommitDurability::Durable => Ok(receipt),
            SinkCommitDurability::PublishedButDurabilityUncertain(source) => {
                Err(PdfSinkCommitError::PublishedButDurabilityUncertain {
                    receipt: Box::new(receipt),
                    source,
                })
            }
        }
    }

    /// Commits a manifest-bound publication that has already passed every
    /// fallible built-manifest check. The manifest and sink receipt are
    /// released together only after the configured sink accepts all bytes.
    /// If the PDF sink rejects the bytes, this consumes the failed-manifest
    /// counterpart sealed by the same preflight and attempts its atomic
    /// publication before returning the original sink error.
    ///
    /// ```compile_fail
    /// # use typaxis_manifest::{BuildOutputCommitContext, PreparedBuiltPublication};
    /// # fn forge(output: &BuildOutputCommitContext, prepared: PreparedBuiltPublication) {
    /// let _ = output.commit_prepared_built(prepared, || Ok::<(), ()>(()));
    /// # }
    /// ```
    pub fn commit_prepared_built(
        self,
        prepared: PreparedBuiltPublication,
    ) -> Result<CommittedBuiltPublication, BuiltPublicationCommitError> {
        if prepared.binding.output != self.binding() {
            return Err(BuiltPublicationCommitError::Pdf(
                PdfSinkCommitError::InvalidFacts(BuildManifestError::OutputReceiptBindingMismatch),
            ));
        }
        let manifest_target = self
            .execution
            .manifest_target()
            .ok_or(BuiltPublicationCommitError::Pdf(
                PdfSinkCommitError::ManifestPreflightRequired,
            ))?
            .clone();
        let pdf_durability = match commit_verified_pdf(&self.execution, &prepared.pdf) {
            Ok(durability) => durability,
            Err(source) => {
                let failed = PreparedFailedPublication {
                    binding: prepared.binding,
                    manifest: prepared.failed_manifest,
                    manifest_bytes: prepared.failed_manifest_bytes,
                };
                let failed_manifest = match self.commit_prepared_failed(failed) {
                    Ok(publication) => FailedManifestPublication::Committed(Box::new(publication)),
                    Err(error) => FailedManifestPublication::CommitError(Box::new(error)),
                };
                return Err(BuiltPublicationCommitError::PdfSinkFailed {
                    source,
                    failed_manifest,
                });
            }
        };
        let receipt = self.issue_receipt(prepared.output);
        if let SinkCommitDurability::PublishedButDurabilityUncertain(source) = pdf_durability {
            return Err(BuiltPublicationCommitError::PdfDurability {
                pdf_receipt: Box::new(receipt),
                source,
            });
        }
        let manifest_durability = match commit_file_bytes(
            &self.execution,
            manifest_target.as_path(),
            &prepared.manifest_bytes,
        ) {
            Ok(durability) => durability,
            Err(error) => {
                return Err(match error {
                    PdfSinkCommitError::Execution(source) => {
                        BuiltPublicationCommitError::ManifestExecution {
                            pdf_receipt: Box::new(receipt),
                            source,
                        }
                    }
                    PdfSinkCommitError::Io(source) => BuiltPublicationCommitError::ManifestIo {
                        pdf_receipt: Box::new(receipt),
                        source,
                    },
                    PdfSinkCommitError::InvalidFacts(_)
                    | PdfSinkCommitError::ManifestPreflightRequired => {
                        BuiltPublicationCommitError::ManifestInvariant {
                            pdf_receipt: Box::new(receipt),
                        }
                    }
                    PdfSinkCommitError::PublishedButDurabilityUncertain { .. } => {
                        BuiltPublicationCommitError::ManifestInvariant {
                            pdf_receipt: Box::new(receipt),
                        }
                    }
                })
            }
        };
        let manifest_receipt = ManifestSinkCommitReceipt {
            binding: prepared.binding,
            bytes: prepared.manifest_bytes.len() as u64,
        };
        let committed = CommittedBuiltPublication {
            manifest: prepared.manifest,
            receipt,
            manifest_receipt,
        };
        match manifest_durability {
            SinkCommitDurability::Durable => Ok(committed),
            SinkCommitDurability::PublishedButDurabilityUncertain(source) => {
                Err(BuiltPublicationCommitError::ManifestDurability {
                    publication: Box::new(committed),
                    source,
                })
            }
        }
    }

    /// Atomically publishes a preflighted terminal failed manifest without
    /// emitting PDF bytes. The output context is consumed even on failure, so
    /// another terminal result cannot be published through the same session.
    pub fn commit_prepared_failed(
        self,
        prepared: PreparedFailedPublication,
    ) -> Result<CommittedFailedPublication, ManifestSinkCommitError> {
        if prepared.binding.output != self.binding() {
            return Err(ManifestSinkCommitError::InvalidFacts(
                BuildManifestError::OutputReceiptBindingMismatch,
            ));
        }
        let manifest_target = self
            .execution
            .manifest_target()
            .ok_or(ManifestSinkCommitError::MissingManifestTarget)?
            .clone();
        let durability = commit_file_bytes(
            &self.execution,
            manifest_target.as_path(),
            &prepared.manifest_bytes,
        )
        .map_err(|error| match error {
            PdfSinkCommitError::Execution(source) => ManifestSinkCommitError::Execution(source),
            PdfSinkCommitError::Io(source) => ManifestSinkCommitError::Io(source),
            PdfSinkCommitError::InvalidFacts(error) => ManifestSinkCommitError::InvalidFacts(error),
            PdfSinkCommitError::ManifestPreflightRequired => {
                ManifestSinkCommitError::MissingManifestTarget
            }
            PdfSinkCommitError::PublishedButDurabilityUncertain { .. } => {
                ManifestSinkCommitError::InvalidFacts(
                    BuildManifestError::OutputReceiptBindingMismatch,
                )
            }
        })?;
        let receipt = ManifestSinkCommitReceipt {
            binding: prepared.binding,
            bytes: prepared.manifest_bytes.len() as u64,
        };
        let committed = CommittedFailedPublication {
            manifest: prepared.manifest,
            receipt,
        };
        match durability {
            SinkCommitDurability::Durable => Ok(committed),
            SinkCommitDurability::PublishedButDurabilityUncertain(source) => {
                Err(ManifestSinkCommitError::PublishedButDurabilityUncertain {
                    publication: Box::new(committed),
                    source,
                })
            }
        }
    }

    fn issue_receipt(&self, facts: PreparedPdfOutputFacts) -> PdfSinkCommitReceipt {
        PdfSinkCommitReceipt {
            binding: self.binding(),
            selected_fingerprint: facts.selected_fingerprint,
            sink: facts.sink,
            bytes: facts.bytes,
            sha256: facts.sha256,
            page_count: facts.page_count,
            pdf_object_count: facts.pdf_object_count,
        }
    }

    fn binding(&self) -> OutputCommitBinding {
        OutputCommitBinding {
            session: self.session,
            config_fingerprint: self.config_fingerprint,
            stream_compression: self.stream_compression,
            limits: self.limits.clone(),
            execution: self.execution.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OutputCommitSessionId(NonZeroU64);
impl OutputCommitSessionId {
    fn allocate() -> Result<Self, BuildOutputCommitContextError> {
        let value = OUTPUT_SESSION_COUNTER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| BuildOutputCommitContextError::SessionIdentityExhausted)?;
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(BuildOutputCommitContextError::SessionIdentityExhausted)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OutputCommitBinding {
    session: OutputCommitSessionId,
    config_fingerprint: EffectiveConfigFingerprint,
    stream_compression: PdfStreamCompression,
    limits: ValidatedResourceLimits,
    execution: BuildExecutionContext,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ManifestPublicationContext {
    session: PublicationSessionId,
    output: OutputCommitBinding,
    manifest_target: HostPath,
    config_fingerprint: EffectiveConfigFingerprint,
    stream_compression: PdfStreamCompression,
    data_versions: DataVersions,
    engine: EngineRecord,
    limits: ValidatedResourceLimits,
    execution: BuildExecutionContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestPublicationError {
    MissingManifestTarget,
    DataTableMismatch,
    OutputContextMismatch,
    SessionIdentityExhausted,
}

impl ManifestPublicationContext {
    pub fn new(
        config: &EffectiveConfig,
        output: &BuildOutputCommitContext,
        shaper: ShaperIdentity,
        tables: &ResolvedDataTables,
    ) -> Result<Self, ManifestPublicationError> {
        if tables.versions() != config.data_versions() {
            return Err(ManifestPublicationError::DataTableMismatch);
        }
        if config.fingerprint() != output.config_fingerprint
            || config.stream_compression() != output.stream_compression
            || config.limits() != &output.limits
        {
            return Err(ManifestPublicationError::OutputContextMismatch);
        }
        let manifest_target = output
            .execution
            .manifest_target()
            .ok_or(ManifestPublicationError::MissingManifestTarget)?
            .clone();
        let session = PublicationSessionId::allocate()?;
        Ok(Self {
            session,
            output: output.binding(),
            manifest_target,
            config_fingerprint: config.fingerprint(),
            stream_compression: config.stream_compression(),
            data_versions: DataVersions::from_runtime(tables, shaper),
            engine: EngineRecord::from_identity(&EngineIdentity::compiled()),
            limits: config.limits().clone(),
            execution: output.execution.clone(),
        })
    }
    pub const fn manifest_target(&self) -> &HostPath {
        &self.manifest_target
    }
    pub const fn config_fingerprint(&self) -> EffectiveConfigFingerprint {
        self.config_fingerprint
    }
    pub const fn limits(&self) -> &ValidatedResourceLimits {
        &self.limits
    }
    pub const fn output_sink(&self) -> OutputSink {
        self.execution.output_sink()
    }

    /// Starts a publication-bound ledger. The ledger owns only facts copied
    /// from phase artifacts; callers cannot insert manifest records directly.
    pub fn begin_admission_ledger(&self) -> ManifestAdmissionLedger {
        ManifestAdmissionLedger {
            binding: self.binding(),
            sources: BTreeMap::new(),
            fonts: BTreeMap::new(),
            images: BTreeMap::new(),
            package_epoch: None,
            resource_fingerprint: None,
        }
    }

    /// Preflights the complete built publication without performing output
    /// I/O. The returned one-shot token owns the serializer-issued bytes and a
    /// fully validated manifest, but does not expose that manifest before the
    /// configured sink has accepted the bytes.
    pub fn prepare_built(
        self,
        package: &ValidatedParsedPackage,
        admitted: AdmittedResourceLedgerToken<'_>,
        pagination: &PaginationResult,
        pdf: VerifiedPdfBytesReceipt,
    ) -> Result<PreparedBuiltPublication, BuildManifestError> {
        let output = validate_pdf_output_facts(&self, pagination, &pdf)?;
        let manifest = prepare_built_manifest(&self, package, admitted, pagination, output)?;
        let manifest_bytes = manifest.manifest().to_canonical_json_bytes();
        let mut failure_ledger = self.begin_admission_ledger();
        failure_ledger.admit_validated_package_sources(package)?;
        failure_ledger.admit_resources(admitted)?;
        let failed_manifest =
            ValidatedBuildManifest::failed(&self, &failure_ledger, Some(pagination))?;
        let failed_manifest_bytes = failed_manifest.manifest().to_canonical_json_bytes();
        Ok(PreparedBuiltPublication {
            binding: self.binding(),
            manifest,
            manifest_bytes,
            failed_manifest,
            failed_manifest_bytes,
            pdf,
            output,
        })
    }

    /// Preflights a terminal failed manifest from facts admitted by this exact
    /// publication session. The validated record remains inaccessible until
    /// the paired output owner atomically publishes its canonical bytes.
    pub fn prepare_failed(
        self,
        ledger: ManifestAdmissionLedger,
        pagination: Option<&PaginationResult>,
    ) -> Result<PreparedFailedPublication, BuildManifestError> {
        let manifest = ValidatedBuildManifest::failed(&self, &ledger, pagination)?;
        let manifest_bytes = manifest.manifest().to_canonical_json_bytes();
        Ok(PreparedFailedPublication {
            binding: self.binding(),
            manifest,
            manifest_bytes,
        })
    }

    fn binding(&self) -> PublicationBinding {
        PublicationBinding {
            session: self.session,
            output: self.output.clone(),
            config_fingerprint: self.config_fingerprint,
            execution: self.execution.clone(),
            limits: self.limits.clone(),
            stream_compression: self.stream_compression,
            data_versions: self.data_versions.clone(),
            engine: self.engine.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PublicationSessionId(NonZeroU64);
impl PublicationSessionId {
    fn allocate() -> Result<Self, ManifestPublicationError> {
        let value = PUBLICATION_SESSION_COUNTER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| ManifestPublicationError::SessionIdentityExhausted)?;
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(ManifestPublicationError::SessionIdentityExhausted)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PublicationBinding {
    session: PublicationSessionId,
    output: OutputCommitBinding,
    config_fingerprint: EffectiveConfigFingerprint,
    execution: BuildExecutionContext,
    limits: ValidatedResourceLimits,
    stream_compression: PdfStreamCompression,
    data_versions: DataVersions,
    engine: EngineRecord,
}

/// Canonical facts admitted so far for a terminal failed manifest. This token
/// is tied to one publication context and can only copy facts from owner types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestAdmissionLedger {
    binding: PublicationBinding,
    sources: BTreeMap<SourceId, FileRecord>,
    fonts: BTreeMap<FontFaceId, FontRecord>,
    images: BTreeMap<ImageResourceId, ImageRecord>,
    package_epoch: Option<PackageEpochIdentity>,
    resource_fingerprint: Option<AdmittedResourceFingerprint>,
}
impl ManifestAdmissionLedger {
    pub fn admit_source(&mut self, source: &SourceRecord) -> Result<(), BuildManifestError> {
        if self.package_epoch.is_some() {
            return Err(BuildManifestError::DuplicateAdmissionRecord);
        }
        let expected = u32::try_from(self.sources.len())
            .map_err(|_| BuildManifestError::NonDenseAdmissionSource)?;
        if source.source_id().get() != expected {
            return Err(BuildManifestError::NonDenseAdmissionSource);
        }
        if self
            .sources
            .values()
            .any(|record| record.uri == *source.uri())
        {
            return Err(BuildManifestError::DuplicateAdmissionRecord);
        }
        let mut sources = self.sources.clone();
        if sources
            .insert(
                source.source_id(),
                FileRecord {
                    uri: source.uri().clone(),
                    bytes: u64::from(source.utf8_byte_length()),
                    sha256: source.content_hash(),
                },
            )
            .is_some()
        {
            return Err(BuildManifestError::DuplicateAdmissionRecord);
        }
        validate_admission_limits(&sources, &self.fonts, &self.images, &self.binding)?;
        self.sources = sources;
        Ok(())
    }

    pub fn admit_validated_package_sources(
        &mut self,
        package: &ValidatedParsedPackage,
    ) -> Result<(), BuildManifestError> {
        let mut candidate = self.clone();
        for source in package.package().sources.records() {
            candidate.admit_source(source)?;
        }
        candidate.package_epoch = Some(package.epoch_identity().clone());
        *self = candidate;
        Ok(())
    }

    /// Copies the complete canonical resource set after resource admission.
    /// The later built factory additionally matches these facts against the
    /// exact validated package declarations.
    pub fn admit_resources(
        &mut self,
        admitted: AdmittedResourceLedgerToken<'_>,
    ) -> Result<(), BuildManifestError> {
        if !self.fonts.is_empty() || !self.images.is_empty() || self.resource_fingerprint.is_some()
        {
            return Err(BuildManifestError::DuplicateAdmissionRecord);
        }
        let mut fonts = BTreeMap::new();
        for font in admitted.fonts() {
            if fonts
                .insert(
                    font.font_face_id(),
                    FontRecord {
                        font_face_id: font.font_face_id(),
                        uri: font.uri().clone(),
                        face_index: font.face_index(),
                        bytes: font.byte_length(),
                        sha256: font.content_hash(),
                        units_per_em: font.metadata().units_per_em,
                        glyph_count: font.metadata().glyph_count,
                    },
                )
                .is_some()
            {
                return Err(BuildManifestError::DuplicateAdmissionRecord);
            }
        }
        let mut images = BTreeMap::new();
        for image in admitted.images() {
            if images
                .insert(
                    image.image_id(),
                    ImageRecord {
                        image_id: image.image_id(),
                        uri: image.uri().clone(),
                        bytes: image.byte_length(),
                        sha256: image.content_hash(),
                        pixel_width: image.width().get(),
                        pixel_height: image.height().get(),
                        decoded_bytes: image.decoded_bytes(),
                    },
                )
                .is_some()
            {
                return Err(BuildManifestError::DuplicateAdmissionRecord);
            }
        }
        validate_admission_limits(&self.sources, &fonts, &images, &self.binding)?;
        self.fonts = fonts;
        self.images = images;
        self.resource_fingerprint = Some(admitted.fingerprint());
        Ok(())
    }

    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    fn manifest_records(&self) -> (Vec<FileRecord>, Vec<FontRecord>, Vec<ImageRecord>) {
        let mut inputs: Vec<_> = self.sources.values().cloned().collect();
        inputs.sort_by(|left, right| left.uri.cmp(&right.uri));
        (
            inputs,
            self.fonts.values().cloned().collect(),
            self.images.values().cloned().collect(),
        )
    }
}

/// One-shot capability proving that all built-publication invariants were
/// checked before output I/O. Both terminal manifest shapes are sealed here:
/// the built record remains inaccessible until PDF commit, while its
/// output-null failed counterpart can only be released if that sink commit
/// fails. `BuildOutputCommitContext::commit_prepared_built` consumes the token
/// through the exact output session captured here.
pub struct PreparedBuiltPublication {
    binding: PublicationBinding,
    manifest: ValidatedBuildManifest,
    manifest_bytes: Vec<u8>,
    failed_manifest: ValidatedBuildManifest,
    failed_manifest_bytes: Vec<u8>,
    pdf: VerifiedPdfBytesReceipt,
    output: PreparedPdfOutputFacts,
}

/// One-shot capability for atomic publication of a terminal failed manifest.
/// Its record and canonical bytes are private until the bound output session
/// consumes it.
pub struct PreparedFailedPublication {
    binding: PublicationBinding,
    manifest: ValidatedBuildManifest,
    manifest_bytes: Vec<u8>,
}

/// Trusted artifacts released together after the configured PDF sink commit
/// succeeds. No fallible manifest validation remains after that commit.
#[derive(Debug, Eq, PartialEq)]
pub struct CommittedBuiltPublication {
    manifest: ValidatedBuildManifest,
    receipt: PdfSinkCommitReceipt,
    manifest_receipt: ManifestSinkCommitReceipt,
}
impl CommittedBuiltPublication {
    pub const fn manifest(&self) -> &ValidatedBuildManifest {
        &self.manifest
    }
    pub const fn receipt(&self) -> &PdfSinkCommitReceipt {
        &self.receipt
    }
    pub const fn manifest_receipt(&self) -> &ManifestSinkCommitReceipt {
        &self.manifest_receipt
    }
    pub fn into_parts(
        self,
    ) -> (
        ValidatedBuildManifest,
        PdfSinkCommitReceipt,
        ManifestSinkCommitReceipt,
    ) {
        (self.manifest, self.receipt, self.manifest_receipt)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CommittedFailedPublication {
    manifest: ValidatedBuildManifest,
    receipt: ManifestSinkCommitReceipt,
}
impl CommittedFailedPublication {
    pub const fn manifest(&self) -> &ValidatedBuildManifest {
        &self.manifest
    }
    pub const fn receipt(&self) -> &ManifestSinkCommitReceipt {
        &self.receipt
    }
    pub fn into_parts(self) -> (ValidatedBuildManifest, ManifestSinkCommitReceipt) {
        (self.manifest, self.receipt)
    }
}

/// Proof that the canonical manifest bytes reached the configured manifest
/// file through the publication owner's atomic sidecar committer. The target
/// HostPath remains runtime-only and is not exposed as a manifest fact.
#[derive(Debug, Eq, PartialEq)]
pub struct ManifestSinkCommitReceipt {
    binding: PublicationBinding,
    bytes: u64,
}
impl ManifestSinkCommitReceipt {
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreparedPdfOutputFacts {
    selected_fingerprint: LayoutStateFingerprint,
    sink: OutputSink,
    bytes: u64,
    sha256: [u8; 32],
    page_count: u32,
    pdf_object_count: u32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PdfSinkCommitReceipt {
    binding: OutputCommitBinding,
    selected_fingerprint: LayoutStateFingerprint,
    sink: OutputSink,
    bytes: u64,
    sha256: [u8; 32],
    page_count: u32,
    pdf_object_count: u32,
}
impl PdfSinkCommitReceipt {
    pub const fn sink(&self) -> OutputSink {
        self.sink
    }
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.sha256
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn pdf_object_count(&self) -> u32 {
        self.pdf_object_count
    }
    pub const fn selected_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_fingerprint
    }
}

#[derive(Debug)]
pub enum PdfSinkCommitError {
    InvalidFacts(BuildManifestError),
    ManifestPreflightRequired,
    Execution(BuildExecutionError),
    Io(io::Error),
    /// The atomic target update completed, but synchronizing the containing
    /// directory failed. The receipt proves visibility and prevents callers
    /// from treating this as a rollback or blindly retrying the publication.
    PublishedButDurabilityUncertain {
        receipt: Box<PdfSinkCommitReceipt>,
        source: io::Error,
    },
}

#[derive(Debug)]
pub enum BuiltPublicationCommitError {
    /// The PDF was not committed.
    Pdf(PdfSinkCommitError),
    /// The PDF sink rejected the bytes before issuing a receipt. A terminal
    /// output-null manifest publication was attempted from the failed record
    /// sealed during built preflight. `CommitError` can itself own a visible
    /// publication when only its directory synchronization failed.
    PdfSinkFailed {
        source: PdfSinkCommitError,
        failed_manifest: FailedManifestPublication,
    },
    /// The PDF target became visible, but its containing directory could not
    /// be synchronized. No manifest write was attempted.
    PdfDurability {
        pdf_receipt: Box<PdfSinkCommitReceipt>,
        source: io::Error,
    },
    /// The PDF commit succeeded, but a current target-alias check rejected the
    /// manifest write. The existing manifest, if any, was preserved.
    ManifestExecution {
        pdf_receipt: Box<PdfSinkCommitReceipt>,
        source: BuildExecutionError,
    },
    /// The PDF commit succeeded, but atomic manifest publication failed. The
    /// existing manifest, if any, was preserved.
    ManifestIo {
        pdf_receipt: Box<PdfSinkCommitReceipt>,
        source: io::Error,
    },
    /// A private publication invariant failed after PDF commit. This variant
    /// is fail-closed and still reports that the PDF reached its sink.
    ManifestInvariant {
        pdf_receipt: Box<PdfSinkCommitReceipt>,
    },
    /// Both target updates became visible, but the manifest directory sync
    /// failed. The complete publication is returned so callers cannot report
    /// either artifact as rolled back.
    ManifestDurability {
        publication: Box<CommittedBuiltPublication>,
        source: io::Error,
    },
}

/// Outcome of publishing the terminal failed manifest after the PDF sink
/// rejected a fully preflighted built publication.
#[derive(Debug)]
pub enum FailedManifestPublication {
    Committed(Box<CommittedFailedPublication>),
    CommitError(Box<ManifestSinkCommitError>),
}

#[derive(Debug)]
pub enum ManifestSinkCommitError {
    InvalidFacts(BuildManifestError),
    MissingManifestTarget,
    Execution(BuildExecutionError),
    Io(io::Error),
    /// The failed manifest became visible, but its directory sync failed.
    PublishedButDurabilityUncertain {
        publication: Box<CommittedFailedPublication>,
        source: io::Error,
    },
}

#[derive(Debug)]
enum SinkCommitDurability {
    Durable,
    PublishedButDurabilityUncertain(io::Error),
}

fn commit_verified_pdf(
    execution: &BuildExecutionContext,
    pdf: &VerifiedPdfBytesReceipt,
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    match execution.output_sink() {
        OutputSink::Stdout => {
            execution
                .revalidate_write_targets()
                .map_err(PdfSinkCommitError::Execution)?;
            let stdout = io::stdout();
            let mut sink = stdout.lock();
            let streamed = pdf
                .write_streaming(&mut sink)
                .map_err(PdfSinkCommitError::Io)?;
            validate_streamed_pdf_facts(pdf, streamed).map_err(PdfSinkCommitError::InvalidFacts)?;
            sink.flush().map_err(PdfSinkCommitError::Io)?;
            Ok(SinkCommitDurability::Durable)
        }
        OutputSink::File => commit_file_pdf_bytes(execution, pdf.bytes()),
    }
}

fn validate_streamed_pdf_facts(
    pdf: &VerifiedPdfBytesReceipt,
    streamed: PdfStreamWriteFacts,
) -> Result<(), BuildManifestError> {
    if streamed.byte_length() != pdf.byte_length() || streamed.content_hash() != pdf.content_hash()
    {
        return Err(BuildManifestError::OutputReceiptBindingMismatch);
    }
    if streamed.selected_layout_fingerprint() != pdf.selected_layout_fingerprint()
        || streamed.page_count() != pdf.page_count()
        || streamed.object_count() != pdf.object_count()
    {
        return Err(BuildManifestError::PdfGraphReceiptMismatch);
    }
    if streamed.stream_compression() != pdf.stream_compression() {
        return Err(BuildManifestError::StreamCompressionMismatch);
    }
    if streamed.config_fingerprint() != pdf.config_fingerprint() {
        return Err(BuildManifestError::ConfigFingerprintMismatch);
    }
    Ok(())
}

fn commit_file_pdf_bytes(
    execution: &BuildExecutionContext,
    bytes: &[u8],
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    let target = execution.output_path().ok_or_else(|| {
        PdfSinkCommitError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "file output has no target",
        ))
    })?;
    commit_file_bytes(execution, target.as_path(), bytes)
}

fn commit_file_bytes(
    execution: &BuildExecutionContext,
    target: &Path,
    bytes: &[u8],
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    execution
        .revalidate_write_targets()
        .map_err(PdfSinkCommitError::Execution)?;
    commit_file_atomically(execution, target, bytes, execution.replace_policy())
}

#[cfg(unix)]
fn commit_file_atomically(
    execution: &BuildExecutionContext,
    target: &Path,
    bytes: &[u8],
    replace_policy: ReplacePolicy,
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    let parent = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let leaf = target.file_name().ok_or_else(|| {
        PdfSinkCommitError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output target has no file name",
        ))
    })?;
    let (temporary, mut file) =
        create_output_temporary(parent, leaf).map_err(PdfSinkCommitError::Io)?;
    let prepublication = (|| {
        file.write_all(bytes).map_err(PdfSinkCommitError::Io)?;
        file.sync_all().map_err(PdfSinkCommitError::Io)?;
        execution
            .revalidate_write_targets()
            .map_err(PdfSinkCommitError::Execution)?;
        match replace_policy {
            ReplacePolicy::NoReplace => {
                fs::hard_link(&temporary, target).map_err(PdfSinkCommitError::Io)?;
            }
            ReplacePolicy::Replace => {
                fs::rename(&temporary, target).map_err(PdfSinkCommitError::Io)?
            }
        }
        Ok(())
    })();
    if let Err(error) = prepublication {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if replace_policy == ReplacePolicy::NoReplace {
        // The target link is already the published artifact. Temporary-name
        // cleanup is best effort and must not turn a successful publication
        // into a false rollback report.
        let _ = fs::remove_file(&temporary);
    }
    Ok(classify_parent_sync(sync_parent_directory(parent)))
}

fn classify_parent_sync(result: io::Result<()>) -> SinkCommitDurability {
    match result {
        Ok(()) => SinkCommitDurability::Durable,
        Err(source) => SinkCommitDurability::PublishedButDurabilityUncertain(source),
    }
}

#[cfg(unix)]
fn create_output_temporary(parent: &Path, leaf: &std::ffi::OsStr) -> io::Result<(PathBuf, File)> {
    for _ in 0..128 {
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(".");
        temporary_name.push(leaf);
        temporary_name.push(format!(".typaxis-{}-{ordinal}.tmp", std::process::id()));
        let temporary = parent.join(temporary_name);
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
        {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique output temporary",
    ))
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn commit_file_atomically(
    _execution: &BuildExecutionContext,
    _target: &Path,
    _bytes: &[u8],
    _replace_policy: ReplacePolicy,
) -> Result<SinkCommitDurability, PdfSinkCommitError> {
    Err(PdfSinkCommitError::Io(io::Error::new(
        io::ErrorKind::Unsupported,
        "no atomic file-output committer is registered for this platform",
    )))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ManifestExpectations<'a> {
    publication: &'a ManifestPublicationContext,
}
impl<'a> ManifestExpectations<'a> {
    const fn from_publication(publication: &'a ManifestPublicationContext) -> Self {
        Self { publication }
    }
}

impl BuildManifest {
    fn terminal(
        publication: &ManifestPublicationContext,
        status: BuildStatus,
        inputs: Vec<FileRecord>,
        fonts: Vec<FontRecord>,
        images: Vec<ImageRecord>,
        layout: Option<LayoutRecord>,
        output: Option<OutputRecord>,
    ) -> Self {
        Self {
            contract: CONTRACT.to_owned(),
            status,
            deterministic: true,
            engine: publication.engine.clone(),
            data_versions: publication.data_versions.clone(),
            config_sha256: publication.config_fingerprint.bytes(),
            inputs,
            fonts,
            images,
            pdf_profile: PDF_PROFILE.to_owned(),
            stream_compression: publication.stream_compression,
            layout,
            output,
        }
    }

    pub fn contract(&self) -> &str {
        &self.contract
    }
    pub const fn status(&self) -> BuildStatus {
        self.status
    }
    pub const fn deterministic(&self) -> bool {
        self.deterministic
    }
    pub const fn engine(&self) -> &EngineRecord {
        &self.engine
    }
    pub const fn data_versions(&self) -> &DataVersions {
        &self.data_versions
    }
    pub const fn config_sha256(&self) -> [u8; 32] {
        self.config_sha256
    }
    pub fn inputs(&self) -> &[FileRecord] {
        &self.inputs
    }
    pub fn fonts(&self) -> &[FontRecord] {
        &self.fonts
    }
    pub fn images(&self) -> &[ImageRecord] {
        &self.images
    }
    pub fn pdf_profile(&self) -> &str {
        &self.pdf_profile
    }
    pub const fn stream_compression(&self) -> PdfStreamCompression {
        self.stream_compression
    }
    pub const fn layout(&self) -> Option<LayoutRecord> {
        self.layout
    }
    pub const fn output(&self) -> Option<&OutputRecord> {
        self.output.as_ref()
    }

    fn to_canonical_json_bytes(&self) -> Vec<u8> {
        canonical_manifest_json(self).into_bytes()
    }

    fn validate(&self, expectations: ManifestExpectations<'_>) -> Result<(), BuildManifestError> {
        if self.contract != CONTRACT {
            return Err(BuildManifestError::WrongContract);
        }
        if !self.deterministic {
            return Err(BuildManifestError::NonDeterministic);
        }
        if self.engine.name != ENGINE_NAME {
            return Err(BuildManifestError::WrongEngineName);
        }
        if self.engine.version.is_empty() || self.engine.rust_version.is_empty() {
            return Err(BuildManifestError::EmptyVersion);
        }
        if self.engine != expectations.publication.engine {
            return Err(BuildManifestError::WrongEngineName);
        }
        if self.pdf_profile != PDF_PROFILE {
            return Err(BuildManifestError::WrongPdfProfile);
        }
        if self.stream_compression != expectations.publication.stream_compression {
            return Err(BuildManifestError::StreamCompressionMismatch);
        }
        if self.data_versions != expectations.publication.data_versions {
            return Err(BuildManifestError::DataVersionMismatch);
        }
        if self.config_sha256 != expectations.publication.config_fingerprint.bytes() {
            return Err(BuildManifestError::ConfigFingerprintMismatch);
        }
        if self
            .inputs
            .windows(2)
            .any(|pair| pair[0].uri >= pair[1].uri)
        {
            return Err(BuildManifestError::NonCanonicalInputs);
        }
        let include_count = match self.inputs.len().checked_sub(1) {
            Some(count) => count,
            None if self.status == BuildStatus::Failed => 0,
            None => return Err(BuildManifestError::MissingEntryInput),
        };
        if self
            .fonts
            .windows(2)
            .any(|pair| pair[0].font_face_id >= pair[1].font_face_id)
        {
            return Err(BuildManifestError::NonCanonicalFonts);
        }
        if self
            .images
            .windows(2)
            .any(|pair| pair[0].image_id >= pair[1].image_id)
        {
            return Err(BuildManifestError::NonCanonicalImages);
        }
        let largest_bytes = self
            .inputs
            .iter()
            .map(|record| record.bytes)
            .chain(self.fonts.iter().map(|record| record.bytes))
            .chain(self.images.iter().map(|record| record.bytes))
            .chain(self.output.iter().map(|record| record.bytes))
            .max()
            .unwrap_or(0);
        if largest_bytes > JSON_SAFE_INTEGER_MAX as u64 {
            return Err(BuildManifestError::IntegerNotJsonSafe);
        }
        if self.fonts.iter().any(|record| record.bytes == 0)
            || self.images.iter().any(|record| record.bytes == 0)
        {
            return Err(BuildManifestError::EmptyAdmittedResource);
        }
        if self
            .fonts
            .iter()
            .any(|record| record.units_per_em == 0 || record.glyph_count == 0)
        {
            return Err(BuildManifestError::InvalidFontMetadata);
        }
        let limits = expectations.publication.limits.get();
        if self
            .layout
            .is_some_and(|layout| layout.pass_count.get() > limits.max_layout_passes)
        {
            return Err(BuildManifestError::LayoutLimit);
        }
        if include_count > limits.max_include_files as usize {
            return Err(BuildManifestError::IncludeFileLimit);
        }
        if self
            .inputs
            .iter()
            .any(|record| record.bytes > u64::from(limits.max_source_bytes))
        {
            return Err(BuildManifestError::InputSourceLimit);
        }
        if checked_byte_sum(self.inputs.iter().map(|record| record.bytes))
            .map_or(true, |bytes| bytes > limits.max_input_bytes)
        {
            return Err(BuildManifestError::InputAggregateLimit);
        }
        if self.fonts.len() > limits.max_fonts as usize {
            return Err(BuildManifestError::FontCountLimit);
        }
        if self
            .fonts
            .iter()
            .any(|record| record.bytes > limits.max_font_bytes)
        {
            return Err(BuildManifestError::FontBytesLimit);
        }
        if self.images.len() > limits.max_images as usize {
            return Err(BuildManifestError::ImageCountLimit);
        }
        if self
            .images
            .iter()
            .any(|record| record.bytes > limits.max_image_bytes)
        {
            return Err(BuildManifestError::ImageBytesLimit);
        }
        for image in &self.images {
            let pixels = u64::from(image.pixel_width)
                .checked_mul(u64::from(image.pixel_height))
                .ok_or(BuildManifestError::ImagePixelLimit)?;
            if image.pixel_width == 0 || image.pixel_height == 0 || pixels > limits.max_image_pixels
            {
                return Err(BuildManifestError::ImagePixelLimit);
            }
            if image.decoded_bytes == 0 || image.decoded_bytes > limits.max_decoded_image_bytes {
                return Err(BuildManifestError::ImageDecodedBytesLimit);
            }
        }
        if checked_byte_sum(
            self.fonts
                .iter()
                .map(|record| record.bytes)
                .chain(self.images.iter().map(|record| record.bytes)),
        )
        .map_or(true, |bytes| bytes > limits.max_resource_bytes)
        {
            return Err(BuildManifestError::ResourceAggregateLimit);
        }
        match self.status {
            BuildStatus::Built if self.layout.is_none() || self.output.is_none() => {
                return Err(BuildManifestError::BuiltRequiresLayoutAndOutput)
            }
            BuildStatus::Built
                if self.output.as_ref().is_some_and(|output| {
                    output.bytes == 0 || output.page_count == 0 || output.pdf_object_count == 0
                }) =>
            {
                return Err(BuildManifestError::EmptyBuiltOutput)
            }
            BuildStatus::Failed if self.output.is_some() => {
                return Err(BuildManifestError::NonBuiltMustNotHaveOutput)
            }
            _ => {}
        }
        if let Some(output) = &self.output {
            if output.sink != expectations.publication.output_sink() {
                return Err(BuildManifestError::OutputSinkMismatch);
            }
            if output.page_count > limits.max_pages {
                return Err(BuildManifestError::PageLimit);
            }
            if output.bytes > limits.max_output_bytes {
                return Err(BuildManifestError::OutputBytesLimit);
            }
            if output.pdf_object_count > limits.max_pdf_objects {
                return Err(BuildManifestError::PdfObjectLimit);
            }
        }
        Ok(())
    }
}

fn canonical_manifest_json(manifest: &BuildManifest) -> String {
    let mut json = String::new();
    json.push('{');
    push_json_member_name(&mut json, "config_sha256", true);
    push_json_hex(&mut json, &manifest.config_sha256);
    push_json_member_name(&mut json, "contract", false);
    push_json_string(&mut json, &manifest.contract);
    push_json_member_name(&mut json, "data_versions", false);
    push_data_versions_json(&mut json, &manifest.data_versions);
    push_json_member_name(&mut json, "deterministic", false);
    json.push_str("true");
    push_json_member_name(&mut json, "engine", false);
    push_engine_json(&mut json, &manifest.engine);
    push_json_member_name(&mut json, "fonts", false);
    push_fonts_json(&mut json, &manifest.fonts);
    push_json_member_name(&mut json, "images", false);
    push_images_json(&mut json, &manifest.images);
    push_json_member_name(&mut json, "inputs", false);
    push_inputs_json(&mut json, &manifest.inputs);
    push_json_member_name(&mut json, "layout", false);
    match manifest.layout {
        Some(layout) => push_layout_json(&mut json, layout),
        None => json.push_str("null"),
    }
    push_json_member_name(&mut json, "output", false);
    match &manifest.output {
        Some(output) => push_output_json(&mut json, output),
        None => json.push_str("null"),
    }
    push_json_member_name(&mut json, "pdf_profile", false);
    push_json_string(&mut json, &manifest.pdf_profile);
    push_json_member_name(&mut json, "status", false);
    push_json_string(
        &mut json,
        match manifest.status {
            BuildStatus::Built => "built",
            BuildStatus::Failed => "failed",
        },
    );
    push_json_member_name(&mut json, "stream_compression", false);
    push_json_string(
        &mut json,
        match manifest.stream_compression {
            PdfStreamCompression::Flate => "flate",
            PdfStreamCompression::None => "none",
        },
    );
    json.push('}');
    json
}

fn push_engine_json(json: &mut String, engine: &EngineRecord) {
    json.push('{');
    push_json_member_name(json, "git_commit", true);
    match &engine.git_commit {
        Some(commit) => push_json_string(json, commit),
        None => json.push_str("null"),
    }
    push_json_member_name(json, "name", false);
    push_json_string(json, &engine.name);
    push_json_member_name(json, "rust_version", false);
    push_json_string(json, &engine.rust_version);
    push_json_member_name(json, "version", false);
    push_json_string(json, &engine.version);
    json.push('}');
}

fn push_data_versions_json(json: &mut String, versions: &DataVersions) {
    json.push('{');
    push_json_member_name(json, "japanese_line_break", true);
    push_json_string(json, &versions.japanese_line_break);
    push_json_member_name(json, "shaper_backend", false);
    push_json_string(json, &versions.shaper_backend);
    push_json_member_name(json, "shaper_version", false);
    push_json_string(json, &versions.shaper_version);
    push_json_member_name(json, "unicode", false);
    push_json_string(json, &versions.unicode);
    json.push('}');
}

fn push_inputs_json(json: &mut String, records: &[FileRecord]) {
    json.push('[');
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push('{');
        push_json_member_name(json, "bytes", true);
        json.push_str(&record.bytes.to_string());
        push_json_member_name(json, "sha256", false);
        push_json_hex(json, &record.sha256);
        push_json_member_name(json, "uri", false);
        push_json_string(json, record.uri.as_str());
        json.push('}');
    }
    json.push(']');
}

fn push_fonts_json(json: &mut String, records: &[FontRecord]) {
    json.push('[');
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push('{');
        push_json_member_name(json, "bytes", true);
        json.push_str(&record.bytes.to_string());
        push_json_member_name(json, "face_index", false);
        json.push_str(&record.face_index.to_string());
        push_json_member_name(json, "font_face_id", false);
        json.push_str(&record.font_face_id.get().to_string());
        push_json_member_name(json, "glyph_count", false);
        json.push_str(&record.glyph_count.to_string());
        push_json_member_name(json, "sha256", false);
        push_json_hex(json, &record.sha256);
        push_json_member_name(json, "units_per_em", false);
        json.push_str(&record.units_per_em.to_string());
        push_json_member_name(json, "uri", false);
        push_json_string(json, record.uri.as_str());
        json.push('}');
    }
    json.push(']');
}

fn push_images_json(json: &mut String, records: &[ImageRecord]) {
    json.push('[');
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push('{');
        push_json_member_name(json, "bytes", true);
        json.push_str(&record.bytes.to_string());
        push_json_member_name(json, "decoded_bytes", false);
        json.push_str(&record.decoded_bytes.to_string());
        push_json_member_name(json, "image_id", false);
        json.push_str(&record.image_id.get().to_string());
        push_json_member_name(json, "pixel_height", false);
        json.push_str(&record.pixel_height.to_string());
        push_json_member_name(json, "pixel_width", false);
        json.push_str(&record.pixel_width.to_string());
        push_json_member_name(json, "sha256", false);
        push_json_hex(json, &record.sha256);
        push_json_member_name(json, "uri", false);
        push_json_string(json, record.uri.as_str());
        json.push('}');
    }
    json.push(']');
}

fn push_layout_json(json: &mut String, layout: LayoutRecord) {
    json.push('{');
    push_json_member_name(json, "fallback_policy", true);
    match layout.fallback_policy {
        Some(LayoutFallbackPolicy::LowestCostThenEarliest) => {
            push_json_string(json, "lowest_cost_then_earliest")
        }
        None => json.push_str("null"),
    }
    push_json_member_name(json, "final_fingerprint", false);
    push_json_hex(json, &layout.final_fingerprint.bytes());
    push_json_member_name(json, "pass_count", false);
    json.push_str(&layout.pass_count.get().to_string());
    push_json_member_name(json, "selected_state", false);
    json.push_str(&layout.selected_state.get().to_string());
    push_json_member_name(json, "status", false);
    push_json_string(
        json,
        match layout.status {
            LayoutStatus::Converged => "converged",
            LayoutStatus::CycleFallback => "cycle_fallback",
            LayoutStatus::MaxPassFallback => "max_pass_fallback",
        },
    );
    json.push('}');
}

fn push_output_json(json: &mut String, output: &OutputRecord) {
    json.push('{');
    push_json_member_name(json, "bytes", true);
    json.push_str(&output.bytes.to_string());
    push_json_member_name(json, "page_count", false);
    json.push_str(&output.page_count.to_string());
    push_json_member_name(json, "pdf_object_count", false);
    json.push_str(&output.pdf_object_count.to_string());
    push_json_member_name(json, "sha256", false);
    push_json_hex(json, &output.sha256);
    push_json_member_name(json, "sink", false);
    push_json_string(
        json,
        match output.sink {
            OutputSink::File => "file",
            OutputSink::Stdout => "stdout",
        },
    );
    json.push('}');
}

fn push_json_member_name(json: &mut String, name: &str, first: bool) {
    if !first {
        json.push(',');
    }
    push_json_string(json, name);
    json.push(':');
}

fn push_json_hex(json: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    json.push('"');
    for byte in bytes {
        json.push(char::from(HEX[usize::from(byte >> 4)]));
        json.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    json.push('"');
}

fn push_json_string(json: &mut String, value: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    json.push('"');
    for character in value.chars() {
        match character {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\u{08}' => json.push_str("\\b"),
            '\u{09}' => json.push_str("\\t"),
            '\u{0a}' => json.push_str("\\n"),
            '\u{0c}' => json.push_str("\\f"),
            '\u{0d}' => json.push_str("\\r"),
            character if character <= '\u{1f}' => {
                let byte = character as u8;
                json.push_str("\\u00");
                json.push(char::from(HEX[usize::from(byte >> 4)]));
                json.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
            character => json.push(character),
        }
    }
    json.push('"');
}

fn checked_byte_sum(mut values: impl Iterator<Item = u64>) -> Option<u64> {
    values.try_fold(0u64, u64::checked_add)
}

fn validate_admission_limits(
    sources: &BTreeMap<SourceId, FileRecord>,
    fonts: &BTreeMap<FontFaceId, FontRecord>,
    images: &BTreeMap<ImageResourceId, ImageRecord>,
    binding: &PublicationBinding,
) -> Result<(), BuildManifestError> {
    let limits = binding.limits.get();
    let include_count = sources
        .len()
        .checked_sub(1)
        .ok_or(BuildManifestError::MissingEntryInput)?;
    if include_count > limits.max_include_files as usize {
        return Err(BuildManifestError::IncludeFileLimit);
    }
    if sources
        .values()
        .any(|record| record.bytes > u64::from(limits.max_source_bytes))
    {
        return Err(BuildManifestError::InputSourceLimit);
    }
    if checked_byte_sum(sources.values().map(|record| record.bytes))
        .map_or(true, |bytes| bytes > limits.max_input_bytes)
    {
        return Err(BuildManifestError::InputAggregateLimit);
    }
    if fonts.len() > limits.max_fonts as usize {
        return Err(BuildManifestError::FontCountLimit);
    }
    if fonts.values().any(|record| {
        record.bytes == 0
            || record.bytes > limits.max_font_bytes
            || record.units_per_em == 0
            || record.glyph_count == 0
    }) {
        return Err(BuildManifestError::FontBytesLimit);
    }
    if images.len() > limits.max_images as usize {
        return Err(BuildManifestError::ImageCountLimit);
    }
    for image in images.values() {
        if image.bytes == 0 || image.bytes > limits.max_image_bytes {
            return Err(BuildManifestError::ImageBytesLimit);
        }
        let pixels = u64::from(image.pixel_width)
            .checked_mul(u64::from(image.pixel_height))
            .ok_or(BuildManifestError::ImagePixelLimit)?;
        if image.pixel_width == 0 || image.pixel_height == 0 || pixels > limits.max_image_pixels {
            return Err(BuildManifestError::ImagePixelLimit);
        }
        if image.decoded_bytes == 0 || image.decoded_bytes > limits.max_decoded_image_bytes {
            return Err(BuildManifestError::ImageDecodedBytesLimit);
        }
    }
    if checked_byte_sum(
        fonts
            .values()
            .map(|record| record.bytes)
            .chain(images.values().map(|record| record.bytes)),
    )
    .map_or(true, |bytes| bytes > limits.max_resource_bytes)
    {
        return Err(BuildManifestError::ResourceAggregateLimit);
    }
    if sources
        .values()
        .map(|record| record.bytes)
        .chain(fonts.values().map(|record| record.bytes))
        .chain(images.values().map(|record| record.bytes))
        .any(|bytes| bytes > JSON_SAFE_INTEGER_MAX as u64)
    {
        return Err(BuildManifestError::IntegerNotJsonSafe);
    }
    Ok(())
}

fn layout_record_for(
    publication: &ManifestPublicationContext,
    pagination: &PaginationResult,
) -> Result<LayoutRecord, BuildManifestError> {
    if pagination.passes().len() > usize::from(publication.limits.get().max_layout_passes) {
        return Err(BuildManifestError::LayoutLimit);
    }
    let page_count = u32::try_from(pagination.selected_pages().len())
        .map_err(|_| BuildManifestError::PageLimit)?;
    if page_count == 0 || page_count > publication.limits.get().max_pages {
        return Err(BuildManifestError::PageLimit);
    }
    LayoutRecord::from_pagination(pagination)
}

fn validate_complete_resource_closure(
    package: &ValidatedParsedPackage,
    admitted: AdmittedResourceLedgerToken<'_>,
) -> Result<(), BuildManifestError> {
    let declarations = &package.package().resources;
    if declarations.font_faces.len() != admitted.fonts().len()
        || declarations.images.len() != admitted.images().len()
    {
        return Err(BuildManifestError::PackageResourceMismatch);
    }
    for (declaration, resource) in declarations.font_faces.iter().zip(admitted.fonts()) {
        if declaration.font_face_id != resource.font_face_id()
            || declaration.uri != *resource.uri()
            || declaration.family != resource.family()
            || declaration.face_index != resource.face_index()
            || declaration
                .expected_sha256
                .is_some_and(|expected| expected != resource.content_hash())
        {
            return Err(BuildManifestError::PackageResourceMismatch);
        }
    }
    for (declaration, resource) in declarations.images.iter().zip(admitted.images()) {
        if declaration.image_id != resource.image_id()
            || declaration.uri != *resource.uri()
            || declaration
                .expected_sha256
                .is_some_and(|expected| expected != resource.content_hash())
        {
            return Err(BuildManifestError::PackageResourceMismatch);
        }
    }
    Ok(())
}

fn validate_ledger_pagination_closure(
    ledger: &ManifestAdmissionLedger,
    pagination: &PaginationResult,
) -> Result<(), BuildManifestError> {
    let package = ledger
        .package_epoch
        .as_ref()
        .ok_or(BuildManifestError::IncompleteLayoutAdmission)?;
    let resources = ledger
        .resource_fingerprint
        .ok_or(BuildManifestError::IncompleteLayoutAdmission)?;
    let epoch = pagination
        .selected_pass()
        .fingerprint_record()
        .layout_epoch();
    if epoch.document() != package.document()
        || epoch.style() != package.style()
        || epoch.admitted_resources() != resources
    {
        return Err(BuildManifestError::PackagePaginationMismatch);
    }
    Ok(())
}

fn validate_pdf_output_facts(
    publication: &ManifestPublicationContext,
    pagination: &PaginationResult,
    pdf: &VerifiedPdfBytesReceipt,
) -> Result<PreparedPdfOutputFacts, BuildManifestError> {
    let page_count = u32::try_from(pagination.selected_pages().len())
        .map_err(|_| BuildManifestError::PageLimit)?;
    let facts = validate_pdf_receipt_facts(
        publication.config_fingerprint,
        publication.stream_compression,
        &publication.limits,
        publication.output_sink(),
        pdf,
    )?;
    if pdf.selected_layout_fingerprint() != pagination.final_fingerprint()
        || pdf.page_count() != page_count
    {
        return Err(BuildManifestError::PdfGraphReceiptMismatch);
    }
    Ok(facts)
}

fn validate_standalone_pdf_output_facts(
    output: &BuildOutputCommitContext,
    pdf: &VerifiedPdfBytesReceipt,
) -> Result<PreparedPdfOutputFacts, BuildManifestError> {
    validate_pdf_receipt_facts(
        output.config_fingerprint,
        output.stream_compression,
        &output.limits,
        output.output_sink(),
        pdf,
    )
}

fn validate_pdf_receipt_facts(
    config_fingerprint: EffectiveConfigFingerprint,
    stream_compression: PdfStreamCompression,
    limits: &ValidatedResourceLimits,
    sink: OutputSink,
    pdf: &VerifiedPdfBytesReceipt,
) -> Result<PreparedPdfOutputFacts, BuildManifestError> {
    let page_count = pdf.page_count();
    let pdf_object_count = pdf.object_count();
    let bytes = pdf.byte_length();
    let limits = limits.get();
    if pdf.config_fingerprint() != config_fingerprint {
        return Err(BuildManifestError::ConfigFingerprintMismatch);
    }
    if pdf.stream_compression() != stream_compression {
        return Err(BuildManifestError::StreamCompressionMismatch);
    }
    if page_count == 0 || pdf_object_count == 0 || bytes == 0 {
        return Err(BuildManifestError::EmptyBuiltOutput);
    }
    if page_count > limits.max_pages {
        return Err(BuildManifestError::PageLimit);
    }
    if pdf_object_count > limits.max_pdf_objects {
        return Err(BuildManifestError::PdfObjectLimit);
    }
    if bytes > limits.max_output_bytes || bytes > JSON_SAFE_INTEGER_MAX as u64 {
        return Err(BuildManifestError::OutputBytesLimit);
    }
    Ok(PreparedPdfOutputFacts {
        selected_fingerprint: pdf.selected_layout_fingerprint(),
        sink,
        bytes,
        sha256: pdf.content_hash(),
        page_count,
        pdf_object_count,
    })
}

fn prepare_built_manifest(
    publication: &ManifestPublicationContext,
    package: &ValidatedParsedPackage,
    admitted: AdmittedResourceLedgerToken<'_>,
    pagination: &PaginationResult,
    output: PreparedPdfOutputFacts,
) -> Result<ValidatedBuildManifest, BuildManifestError> {
    if output.sink != publication.output_sink() {
        return Err(BuildManifestError::OutputSinkMismatch);
    }
    let layout = layout_record_for(publication, pagination)?;
    let selected_epoch = pagination
        .selected_pass()
        .fingerprint_record()
        .layout_epoch();
    if selected_epoch.document() != package.epoch_identity().document()
        || selected_epoch.style() != package.epoch_identity().style()
        || selected_epoch.admitted_resources() != admitted.fingerprint()
    {
        return Err(BuildManifestError::PackagePaginationMismatch);
    }
    let page_count = u32::try_from(pagination.selected_pages().len())
        .map_err(|_| BuildManifestError::PageLimit)?;
    if output.selected_fingerprint != pagination.final_fingerprint()
        || output.page_count != page_count
    {
        return Err(BuildManifestError::PaginationReceiptMismatch);
    }
    let mut ledger = publication.begin_admission_ledger();
    ledger.admit_validated_package_sources(package)?;
    validate_complete_resource_closure(package, admitted)?;
    ledger.admit_resources(admitted)?;
    let (inputs, fonts, images) = ledger.manifest_records();
    let output = OutputRecord {
        sink: output.sink,
        bytes: output.bytes,
        sha256: output.sha256,
        page_count: output.page_count,
        pdf_object_count: output.pdf_object_count,
    };
    let manifest = BuildManifest::terminal(
        publication,
        BuildStatus::Built,
        inputs,
        fonts,
        images,
        Some(layout),
        Some(output),
    );
    ValidatedBuildManifest::new(
        manifest,
        ManifestExpectations::from_publication(publication),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedBuildManifest(BuildManifest);
impl ValidatedBuildManifest {
    fn new(
        manifest: BuildManifest,
        expectations: ManifestExpectations<'_>,
    ) -> Result<Self, BuildManifestError> {
        manifest.validate(expectations)?;
        Ok(Self(manifest))
    }

    fn failed(
        publication: &ManifestPublicationContext,
        ledger: &ManifestAdmissionLedger,
        pagination: Option<&PaginationResult>,
    ) -> Result<Self, BuildManifestError> {
        if ledger.binding != publication.binding() {
            return Err(BuildManifestError::AdmissionLedgerBindingMismatch);
        }
        let layout = pagination
            .map(|pagination| {
                validate_ledger_pagination_closure(ledger, pagination)?;
                layout_record_for(publication, pagination)
            })
            .transpose()?;
        let (inputs, fonts, images) = ledger.manifest_records();
        let manifest = BuildManifest::terminal(
            publication,
            BuildStatus::Failed,
            inputs,
            fonts,
            images,
            layout,
            None,
        );
        Self::new(
            manifest,
            ManifestExpectations::from_publication(publication),
        )
    }

    pub const fn manifest(&self) -> &BuildManifest {
        &self.0
    }
    pub fn into_manifest(self) -> BuildManifest {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use typaxis_core::{
        BuildExecutionContext, ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions,
        HostPath, ReplacePolicy, ResourceLimits,
    };
    use typaxis_display_list::ValidatedDisplayDocument;
    use typaxis_layout::{FlowCursor, FlowTree, LayoutEpoch, PageContext, ResolvedPageSelection};
    use typaxis_pagination::{
        InitialPaginationState, LayoutPass, LayoutPassInput, PageFrameKind, PageFramePlan,
        PagePlan, PaginationInput, PaginationOptions, PaginationOutcome,
    };
    use typaxis_pdf::PdfBackend;
    use typaxis_resources::{
        AdmittedResourceLedger, AdmittedResourceResolver, FrozenPdfResourcePlans,
    };
    use typaxis_syntax::{
        PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
        ValidatedParsedPackage,
    };
    use typaxis_text::GeneratedTextStore;

    fn shaper_identity() -> ShaperIdentity {
        ShaperIdentity::linked_reference()
    }

    fn tables() -> ResolvedDataTables {
        ResolvedDataTables::resolve("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap()
    }

    fn effective_config() -> EffectiveConfig {
        effective_config_with_limits(ResourceLimits::default())
    }

    fn effective_config_with_limits(limits: ResourceLimits) -> EffectiveConfig {
        EffectiveConfig::new(
            false,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            ["http", "https", "mailto"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            limits,
        )
        .unwrap()
    }

    fn publication() -> ManifestPublicationContext {
        publication_for(&effective_config())
    }

    fn publication_for(config: &EffectiveConfig) -> ManifestPublicationContext {
        let execution = BuildExecutionContext::from_cli_token(
            OsStr::new("-"),
            None,
            Some(HostPath::new("target/build-manifest.json").unwrap()),
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new(config, &execution).unwrap();
        ManifestPublicationContext::new(config, &output, shaper_identity(), &tables()).unwrap()
    }

    fn failed_manifest() -> BuildManifest {
        let config = effective_config();
        failed_manifest_for(&config)
    }

    fn failed_manifest_for(config: &EffectiveConfig) -> BuildManifest {
        BuildManifest {
            contract: CONTRACT.to_owned(),
            status: BuildStatus::Failed,
            deterministic: true,
            engine: EngineRecord::from_identity(&EngineIdentity::compiled()),
            data_versions: DataVersions::from_runtime(&tables(), shaper_identity()),
            config_sha256: config.fingerprint().bytes(),
            inputs: vec![FileRecord {
                uri: PortablePath::new("entry.tsf").unwrap(),
                bytes: 0,
                sha256: [0; 32],
            }],
            fonts: vec![],
            images: vec![],
            pdf_profile: PDF_PROFILE.to_owned(),
            stream_compression: PdfStreamCompression::Flate,
            layout: None,
            output: None,
        }
    }

    fn validate(manifest: BuildManifest) -> Result<ValidatedBuildManifest, BuildManifestError> {
        let publication = publication();
        validate_against(manifest, &publication)
    }

    fn validate_against(
        manifest: BuildManifest,
        publication: &ManifestPublicationContext,
    ) -> Result<ValidatedBuildManifest, BuildManifestError> {
        ValidatedBuildManifest::new(
            manifest,
            ManifestExpectations::from_publication(publication),
        )
    }

    fn validated_package(source_text: &str) -> ValidatedParsedPackage {
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new(format!("entry-{}.tsf", source_text.replace(' ', "-"))).unwrap(),
            text: String::new(),
        };
        let config = effective_config();
        let outcome = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes()).unwrap(),
        );
        let ParseOutcome::Parsed { package, .. } = outcome else {
            panic!("reference package must parse");
        };
        *package
    }

    fn pagination_for(
        package: &ValidatedParsedPackage,
        admitted: &typaxis_resources::AdmittedResourceLedger,
    ) -> PaginationResult {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let store = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let generated = package.bind_generated_text(&store, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let flow = FlowTree::empty(package, epoch).unwrap();
        let initial = InitialPaginationState::new(&flow, package, &limits).unwrap();
        let package_context = package.pagination_context();
        let mut input = PaginationInput::new(
            initial,
            &package_context,
            PaginationOptions::from_limits(&limits, false),
        )
        .unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let master = &package.package().page_masters.masters[0];
        let pages = vec![PagePlan {
            page_index: 0,
            master_id: master.master_id.clone(),
            frames: vec![PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds: master.body,
            }],
            fragments: vec![],
            footnote_ids: vec![],
            float_decisions: vec![],
            column_decisions: vec![],
            resolved_references: vec![],
        }];
        let page_selection = ResolvedPageSelection::new(&flow, &cursor, package).unwrap();
        let page_context = PageContext::select(0, &page_selection, &package_context).unwrap();
        let mut budget = input.take_work_budget().unwrap();
        let first_input = LayoutPassInput::initial(&input);
        let first_fingerprint = first_input.fingerprint();
        let mut first_permit = budget.begin_pass(0, first_input).unwrap();
        for page in &pages {
            first_permit
                .begin_page(&page_context, &cursor, &page.frames)
                .unwrap();
            first_permit.finish_page(page).unwrap();
        }
        let first_receipt = first_permit.finish(&flow, &pages).unwrap();
        let first = LayoutPass::new(
            first_receipt,
            first_fingerprint,
            &flow,
            pages.clone(),
            store.clone(),
        )
        .unwrap();
        let transition = first.transition_references(package, &limits).unwrap();
        let second_input = LayoutPassInput::transitioned(transition);
        let second_fingerprint = second_input.fingerprint();
        let mut second_permit = budget.begin_pass(1, second_input).unwrap();
        for page in &pages {
            second_permit
                .begin_page(&page_context, &cursor, &page.frames)
                .unwrap();
            second_permit.finish_page(page).unwrap();
        }
        let second_receipt = second_permit.finish(&flow, &pages).unwrap();
        let second =
            LayoutPass::new(second_receipt, second_fingerprint, &flow, pages, store).unwrap();
        PaginationOutcome::new(
            vec![first, second],
            ConvergenceStatus::Converged,
            &input,
            budget.finish(),
        )
        .unwrap()
        .into_result()
    }

    fn serialized_pdf_for(
        package: &ValidatedParsedPackage,
        admitted: &AdmittedResourceLedger,
        pagination: &PaginationResult,
        config: &EffectiveConfig,
    ) -> VerifiedPdfBytesReceipt {
        let display =
            ValidatedDisplayDocument::paint_blank_selected(package, pagination, config).unwrap();
        let resources = FrozenPdfResourcePlans::from_verified_receipts(
            &display,
            admitted,
            config.limits(),
            vec![],
        )
        .unwrap();
        let graph = PdfBackend::build(display, resources, config.limits()).unwrap();
        PdfBackend::serialize(graph, config).unwrap()
    }

    #[test]
    fn streamed_pdf_facts_close_over_the_serializer_receipt() {
        let config = effective_config();
        let package = validated_package("");
        let admitted = AdmittedResourceResolver::new(&package.package().resources, config.limits())
            .unwrap()
            .finish()
            .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let pdf = serialized_pdf_for(&package, &admitted, &pagination, &config);
        let mut sink = Vec::new();
        let streamed = pdf.write_streaming(&mut sink).unwrap();

        assert_eq!(sink, pdf.bytes());
        assert_eq!(validate_streamed_pdf_facts(&pdf, streamed), Ok(()));
    }

    #[test]
    fn failed_factory_owns_only_publication_bound_admission_facts() {
        let publication = publication();
        let mut ledger = publication.begin_admission_ledger();
        ledger
            .admit_source(
                &SourceRecord::new(
                    SourceId::new(0),
                    PortablePath::new("z-entry.tsf").unwrap(),
                    "entry".to_owned(),
                )
                .unwrap(),
            )
            .unwrap();
        ledger
            .admit_source(
                &SourceRecord::new(
                    SourceId::new(1),
                    PortablePath::new("a-include.tsf").unwrap(),
                    "include".to_owned(),
                )
                .unwrap(),
            )
            .unwrap();

        let failed = ValidatedBuildManifest::failed(&publication, &ledger, None).unwrap();
        assert_eq!(failed.manifest().status(), BuildStatus::Failed);
        assert_eq!(failed.manifest().inputs().len(), 2);
        assert_eq!(
            failed.manifest().inputs()[0].uri().as_str(),
            "a-include.tsf"
        );
        assert_eq!(failed.manifest().inputs()[1].uri().as_str(), "z-entry.tsf");
        assert!(failed.manifest().output().is_none());

        let other_config = effective_config_with_limits(ResourceLimits {
            max_pages: ResourceLimits::default().max_pages - 1,
            ..ResourceLimits::default()
        });
        assert_eq!(
            ValidatedBuildManifest::failed(&publication_for(&other_config), &ledger, None),
            Err(BuildManifestError::AdmissionLedgerBindingMismatch)
        );
        assert_eq!(
            ValidatedBuildManifest::failed(&publication_for(&effective_config()), &ledger, None,),
            Err(BuildManifestError::AdmissionLedgerBindingMismatch)
        );
    }

    #[cfg(unix)]
    #[test]
    fn file_output_owner_commits_atomically_and_honors_replace_policy() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-commit-{}-{ordinal}", std::process::id());
        let output = output_root.join(format!("{stem}.pdf"));

        let no_replace = BuildExecutionContext::from_cli_token(
            output.as_os_str(),
            None,
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        commit_file_pdf_bytes(&no_replace, b"first complete pdf").unwrap();
        assert_eq!(fs::read(&output).unwrap(), b"first complete pdf");
        let second = commit_file_pdf_bytes(&no_replace, b"must not replace").unwrap_err();
        assert!(matches!(
            second,
            PdfSinkCommitError::Io(error) if error.kind() == io::ErrorKind::AlreadyExists
        ));
        assert_eq!(fs::read(&output).unwrap(), b"first complete pdf");

        let replace = BuildExecutionContext::from_cli_token(
            output.as_os_str(),
            None,
            None,
            ReplacePolicy::Replace,
        )
        .unwrap();
        commit_file_pdf_bytes(&replace, b"replacement pdf").unwrap();
        assert_eq!(fs::read(&output).unwrap(), b"replacement pdf");
        fs::remove_file(output).unwrap();
    }

    #[test]
    fn post_publish_sync_failure_is_not_classified_as_rollback() {
        let outcome =
            classify_parent_sync(Err(io::Error::other("injected directory sync failure")));
        assert!(matches!(
            outcome,
            SinkCommitDurability::PublishedButDurabilityUncertain(error)
                if error.kind() == io::ErrorKind::Other
        ));
    }

    #[cfg(unix)]
    #[test]
    fn sink_owner_revalidates_target_aliases_immediately_before_write() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-alias-recheck-{}-{ordinal}", std::process::id());
        let output = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        fs::write(&output, b"previous pdf").unwrap();
        fs::write(&manifest_path, b"previous manifest").unwrap();
        let execution = BuildExecutionContext::from_cli_token(
            output.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            ReplacePolicy::Replace,
        )
        .unwrap();

        fs::remove_file(&manifest_path).unwrap();
        fs::hard_link(&output, &manifest_path).unwrap();
        assert!(matches!(
            commit_file_pdf_bytes(&execution, b"new pdf"),
            Err(PdfSinkCommitError::Execution(
                BuildExecutionError::AliasedWriteTarget
            ))
        ));
        assert_eq!(fs::read(&output).unwrap(), b"previous pdf");

        fs::remove_file(manifest_path).unwrap();
        fs::remove_file(output).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn failed_manifest_is_session_bound_and_atomically_published() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-failed-publish-{}-{ordinal}", std::process::id());
        let output_path = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        let config = effective_config();
        let execution = BuildExecutionContext::from_cli_token(
            output_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            ReplacePolicy::Replace,
        )
        .unwrap();

        let expected_output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let wrong_output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        assert_ne!(expected_output.binding(), wrong_output.binding());
        let publication = ManifestPublicationContext::new(
            &config,
            &expected_output,
            shaper_identity(),
            &tables(),
        )
        .unwrap();
        let ledger = publication.begin_admission_ledger();
        let prepared = publication.prepare_failed(ledger, None).unwrap();
        assert!(matches!(
            wrong_output.commit_prepared_failed(prepared),
            Err(ManifestSinkCommitError::InvalidFacts(
                BuildManifestError::OutputReceiptBindingMismatch
            ))
        ));
        assert!(!manifest_path.exists());

        let output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output, shaper_identity(), &tables())
                .unwrap();
        let ledger = publication.begin_admission_ledger();
        let prepared = publication.prepare_failed(ledger, None).unwrap();
        let committed = output.commit_prepared_failed(prepared).unwrap();
        let bytes = fs::read(&manifest_path).unwrap();
        assert!(bytes.starts_with(b"{\"config_sha256\":"));
        assert!(!bytes.contains(&b'\n'));
        assert_eq!(
            committed.manifest().manifest().status(),
            BuildStatus::Failed
        );
        assert_eq!(committed.receipt().bytes(), bytes.len() as u64);
        assert!(!output_path.exists());

        fs::remove_file(manifest_path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn failed_manifest_no_replace_preserves_existing_sidecar() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-no-replace-{}-{ordinal}", std::process::id());
        let output_path = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        fs::write(&manifest_path, b"existing manifest").unwrap();
        let config = effective_config();
        let execution = BuildExecutionContext::from_cli_token(
            output_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output, shaper_identity(), &tables())
                .unwrap();
        let ledger = publication.begin_admission_ledger();
        let prepared = publication.prepare_failed(ledger, None).unwrap();
        assert!(matches!(
            output.commit_prepared_failed(prepared),
            Err(ManifestSinkCommitError::Io(error))
                if error.kind() == io::ErrorKind::AlreadyExists
        ));
        assert_eq!(fs::read(&manifest_path).unwrap(), b"existing manifest");
        assert!(!output_path.exists());

        fs::remove_file(manifest_path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn prepared_build_pdf_sink_failure_publishes_failed_manifest() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!(
            "manifest-built-sink-failure-{}-{ordinal}",
            std::process::id()
        );
        let output_path = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        fs::write(&output_path, b"existing pdf must be preserved").unwrap();

        let config = effective_config();
        let package = validated_package("");
        let admitted = AdmittedResourceResolver::new(&package.package().resources, config.limits())
            .unwrap()
            .finish()
            .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let pdf = serialized_pdf_for(&package, &admitted, &pagination, &config);
        let execution = BuildExecutionContext::from_cli_token(
            output_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output, shaper_identity(), &tables())
                .unwrap();
        let prepared = publication
            .prepare_built(&package, admitted.token(), &pagination, pdf)
            .unwrap();

        let error = output.commit_prepared_built(prepared).unwrap_err();
        let BuiltPublicationCommitError::PdfSinkFailed {
            source,
            failed_manifest,
        } = error
        else {
            panic!("PDF no-replace failure must publish the sealed failed manifest");
        };
        assert!(matches!(
            source,
            PdfSinkCommitError::Io(error) if error.kind() == io::ErrorKind::AlreadyExists
        ));
        let FailedManifestPublication::Committed(committed) = failed_manifest else {
            panic!("failed manifest must commit when its target is available");
        };
        assert_eq!(
            committed.manifest().manifest().status(),
            BuildStatus::Failed
        );
        assert!(committed.manifest().manifest().layout().is_some());
        assert!(committed.manifest().manifest().output().is_none());

        let manifest_bytes = fs::read(&manifest_path).unwrap();
        assert!(manifest_bytes
            .windows(17)
            .any(|bytes| bytes == b"\"status\":\"failed\""));
        assert!(manifest_bytes
            .windows(13)
            .any(|bytes| bytes == b"\"output\":null"));
        assert_eq!(committed.receipt().bytes(), manifest_bytes.len() as u64);
        assert_eq!(
            fs::read(&output_path).unwrap(),
            b"existing pdf must be preserved"
        );

        fs::remove_file(manifest_path).unwrap();
        fs::remove_file(output_path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn successful_pdf_commit_still_preserves_manifest_failure_receipt() {
        let output_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target");
        fs::create_dir_all(&output_root).unwrap();
        let ordinal = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stem = format!("manifest-post-pdf-failure-{}-{ordinal}", std::process::id());
        let output_path = output_root.join(format!("{stem}.pdf"));
        let manifest_path = output_root.join(format!("{stem}.json"));
        fs::write(&manifest_path, b"existing manifest must be preserved").unwrap();

        let config = effective_config();
        let package = validated_package("");
        let admitted = AdmittedResourceResolver::new(&package.package().resources, config.limits())
            .unwrap()
            .finish()
            .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let pdf = serialized_pdf_for(&package, &admitted, &pagination, &config);
        let expected_pdf_bytes = pdf.byte_length();
        let execution = BuildExecutionContext::from_cli_token(
            output_path.as_os_str(),
            None,
            Some(HostPath::new(manifest_path.clone()).unwrap()),
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let output = BuildOutputCommitContext::new(&config, &execution).unwrap();
        let publication =
            ManifestPublicationContext::new(&config, &output, shaper_identity(), &tables())
                .unwrap();
        let prepared = publication
            .prepare_built(&package, admitted.token(), &pagination, pdf)
            .unwrap();

        let error = output.commit_prepared_built(prepared).unwrap_err();
        let BuiltPublicationCommitError::ManifestIo {
            pdf_receipt,
            source,
        } = error
        else {
            panic!("a post-PDF no-replace failure must retain the PDF receipt");
        };
        assert_eq!(source.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(pdf_receipt.bytes(), expected_pdf_bytes);
        assert_eq!(
            fs::metadata(&output_path).unwrap().len(),
            expected_pdf_bytes
        );
        assert_eq!(
            fs::read(&manifest_path).unwrap(),
            b"existing manifest must be preserved"
        );

        fs::remove_file(manifest_path).unwrap();
        fs::remove_file(output_path).unwrap();
    }

    #[test]
    fn built_preflight_closes_package_resources_pagination_and_output_facts() {
        let publication = publication();
        let package = validated_package("entry");
        let admitted =
            AdmittedResourceResolver::new(&package.package().resources, publication.limits())
                .unwrap()
                .finish()
                .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let output = PreparedPdfOutputFacts {
            selected_fingerprint: pagination.final_fingerprint(),
            sink: publication.output_sink(),
            bytes: 4,
            sha256: [7; 32],
            page_count: 1,
            pdf_object_count: 3,
        };

        let built = prepare_built_manifest(
            &publication,
            &package,
            admitted.token(),
            &pagination,
            output,
        )
        .unwrap();
        assert_eq!(built.manifest().status(), BuildStatus::Built);
        assert_eq!(built.manifest().inputs().len(), 1);
        assert_eq!(
            built.manifest().layout().unwrap().final_fingerprint(),
            pagination.final_fingerprint()
        );
        assert_eq!(built.manifest().output().unwrap().page_count(), 1);
        assert_eq!(built.manifest().output().unwrap().pdf_object_count(), 3);

        let other_package = validated_package("different document");
        assert_eq!(
            prepare_built_manifest(
                &publication,
                &other_package,
                admitted.token(),
                &pagination,
                output,
            ),
            Err(BuildManifestError::PackagePaginationMismatch)
        );

        let mut wrong_output = output;
        wrong_output.page_count = 2;
        assert_eq!(
            prepare_built_manifest(
                &publication,
                &package,
                admitted.token(),
                &pagination,
                wrong_output,
            ),
            Err(BuildManifestError::PaginationReceiptMismatch)
        );
    }

    #[test]
    fn failed_layout_requires_complete_package_and_resource_identity() {
        let publication = publication();
        let package = validated_package("entry");
        let admitted =
            AdmittedResourceResolver::new(&package.package().resources, publication.limits())
                .unwrap()
                .finish()
                .unwrap();
        let pagination = pagination_for(&package, &admitted);
        let ledger = publication.begin_admission_ledger();
        assert_eq!(
            ValidatedBuildManifest::failed(&publication, &ledger, Some(&pagination)),
            Err(BuildManifestError::IncompleteLayoutAdmission)
        );

        let mut complete = publication.begin_admission_ledger();
        complete.admit_validated_package_sources(&package).unwrap();
        complete.admit_resources(admitted.token()).unwrap();
        let failed =
            ValidatedBuildManifest::failed(&publication, &complete, Some(&pagination)).unwrap();
        assert!(failed.manifest().layout().is_some());
    }

    #[test]
    fn layout_record_selects_only_a_materialized_state() {
        let pass_count = NonZeroU16::new(2).unwrap();
        let selected = NonZeroU16::new(2).unwrap();
        let record = LayoutRecord::new(
            LayoutStatus::Converged,
            pass_count,
            selected,
            LayoutStateFingerprint::from_untrusted_bytes([1; 32]),
        )
        .unwrap();
        assert_eq!(record.selected_state(), selected);
        assert_eq!(record.fallback_policy(), None);

        let fallback = LayoutRecord::new(
            LayoutStatus::MaxPassFallback,
            pass_count,
            selected,
            LayoutStateFingerprint::from_untrusted_bytes([1; 32]),
        )
        .unwrap();
        assert_eq!(
            fallback.fallback_policy(),
            Some(LayoutFallbackPolicy::LowestCostThenEarliest)
        );

        let out_of_range = NonZeroU16::new(3).unwrap();
        assert!(LayoutRecord::new(
            LayoutStatus::MaxPassFallback,
            pass_count,
            out_of_range,
            LayoutStateFingerprint::from_untrusted_bytes([2; 32]),
        )
        .is_none());
        assert!(LayoutRecord::new(
            LayoutStatus::Converged,
            pass_count,
            NonZeroU16::new(1).unwrap(),
            LayoutStateFingerprint::from_untrusted_bytes([2; 32]),
        )
        .is_none());
    }

    #[test]
    fn validated_manifest_enforces_determinism_and_status_shape() {
        assert!(validate(failed_manifest()).is_ok());

        let mut failed_before_entry_admission = failed_manifest();
        failed_before_entry_admission.inputs.clear();
        assert!(validate(failed_before_entry_admission).is_ok());

        let mut nondeterministic = failed_manifest();
        nondeterministic.deterministic = false;
        assert_eq!(
            validate(nondeterministic),
            Err(BuildManifestError::NonDeterministic)
        );

        let mut built_without_artifacts = failed_manifest();
        built_without_artifacts.status = BuildStatus::Built;
        assert_eq!(
            validate(built_without_artifacts),
            Err(BuildManifestError::BuiltRequiresLayoutAndOutput)
        );

        let mut failed_with_output = failed_manifest();
        failed_with_output.output = Some(OutputRecord {
            sink: OutputSink::File,
            bytes: 1,
            sha256: [1; 32],
            page_count: 1,
            pdf_object_count: 1,
        });
        assert_eq!(
            validate(failed_with_output),
            Err(BuildManifestError::NonBuiltMustNotHaveOutput)
        );
    }

    #[test]
    fn manifest_requires_canonical_records_and_nonempty_built_output() {
        let mut unsorted = failed_manifest();
        unsorted.inputs = vec![
            FileRecord {
                uri: PortablePath::new("b.tsf").unwrap(),
                bytes: 1,
                sha256: [1; 32],
            },
            FileRecord {
                uri: PortablePath::new("a.tsf").unwrap(),
                bytes: 1,
                sha256: [2; 32],
            },
        ];
        assert_eq!(
            validate(unsorted),
            Err(BuildManifestError::NonCanonicalInputs)
        );

        let mut empty_built = failed_manifest();
        empty_built.status = BuildStatus::Built;
        empty_built.layout = LayoutRecord::new(
            LayoutStatus::Converged,
            NonZeroU16::new(1).unwrap(),
            NonZeroU16::new(1).unwrap(),
            LayoutStateFingerprint::from_untrusted_bytes([1; 32]),
        );
        empty_built.output = Some(OutputRecord {
            sink: OutputSink::File,
            bytes: 0,
            sha256: [0; 32],
            page_count: 0,
            pdf_object_count: 0,
        });
        assert_eq!(
            validate(empty_built),
            Err(BuildManifestError::EmptyBuiltOutput)
        );
    }

    #[test]
    fn publication_context_exists_only_after_config_and_manifest_target() {
        let config = effective_config();
        let without_target = BuildExecutionContext::from_cli_token(
            OsStr::new("-"),
            None,
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        let without_target = BuildOutputCommitContext::new(&config, &without_target).unwrap();
        assert!(!without_target.manifest_requested());
        let _manifest_free_commit_api: fn(
            BuildOutputCommitContext,
            VerifiedPdfBytesReceipt,
        )
            -> Result<PdfSinkCommitReceipt, PdfSinkCommitError> =
            BuildOutputCommitContext::commit_pdf_without_manifest;
        assert_eq!(
            ManifestPublicationContext::new(&config, &without_target, shaper_identity(), &tables(),),
            Err(ManifestPublicationError::MissingManifestTarget)
        );
        let context = publication();
        assert_eq!(context.config_fingerprint(), config.fingerprint());

        let mut forged = failed_manifest();
        forged.config_sha256 = [9; 32];
        assert_eq!(
            validate(forged),
            Err(BuildManifestError::ConfigFingerprintMismatch)
        );
    }

    fn manifest_limit_config() -> EffectiveConfig {
        let limits = ResourceLimits {
            max_input_bytes: 6,
            max_source_bytes: 3,
            max_font_bytes: 4,
            max_fonts: 2,
            max_image_bytes: 4,
            max_images: 2,
            max_resource_bytes: 15,
            max_image_pixels: 1,
            max_decoded_image_bytes: 4,
            max_pages: 2,
            max_pdf_objects: 2,
            max_output_bytes: 6,
            ..ResourceLimits::default()
        };
        effective_config_with_limits(limits)
    }

    fn built_manifest_at_limits(config: &EffectiveConfig) -> BuildManifest {
        let mut manifest = failed_manifest_for(config);
        manifest.status = BuildStatus::Built;
        manifest.inputs = vec![
            FileRecord {
                uri: PortablePath::new("a.tsf").unwrap(),
                bytes: 3,
                sha256: [1; 32],
            },
            FileRecord {
                uri: PortablePath::new("b.tsf").unwrap(),
                bytes: 3,
                sha256: [2; 32],
            },
        ];
        manifest.fonts = vec![
            FontRecord {
                font_face_id: FontFaceId::new(0),
                uri: PortablePath::new("fonts/a.ttf").unwrap(),
                face_index: 0,
                bytes: 4,
                sha256: [3; 32],
                units_per_em: 1000,
                glyph_count: 1,
            },
            FontRecord {
                font_face_id: FontFaceId::new(1),
                uri: PortablePath::new("fonts/b.ttf").unwrap(),
                face_index: 0,
                bytes: 4,
                sha256: [4; 32],
                units_per_em: 1000,
                glyph_count: 1,
            },
        ];
        manifest.images = vec![
            ImageRecord {
                image_id: ImageResourceId::new(0),
                uri: PortablePath::new("images/a.png").unwrap(),
                bytes: 4,
                sha256: [5; 32],
                pixel_width: 1,
                pixel_height: 1,
                decoded_bytes: 4,
            },
            ImageRecord {
                image_id: ImageResourceId::new(1),
                uri: PortablePath::new("images/b.png").unwrap(),
                bytes: 3,
                sha256: [6; 32],
                pixel_width: 1,
                pixel_height: 1,
                decoded_bytes: 4,
            },
        ];
        manifest.layout = LayoutRecord::new(
            LayoutStatus::Converged,
            NonZeroU16::new(1).unwrap(),
            NonZeroU16::new(1).unwrap(),
            LayoutStateFingerprint::from_untrusted_bytes([7; 32]),
        );
        manifest.output = Some(OutputRecord {
            sink: OutputSink::Stdout,
            bytes: 6,
            sha256: [8; 32],
            page_count: 2,
            pdf_object_count: 2,
        });
        manifest
    }

    #[test]
    fn manifest_actual_facts_accept_exact_limits_and_reject_max_plus_one() {
        let config = manifest_limit_config();
        let publication = publication_for(&config);
        let exact = built_manifest_at_limits(&config);
        assert!(validate_against(exact.clone(), &publication).is_ok());
        assert_eq!(publication.limits(), config.limits());

        let mut source = exact.clone();
        source.inputs[0].bytes = 4;
        assert_eq!(
            validate_against(source, &publication),
            Err(BuildManifestError::InputSourceLimit)
        );

        let mut input_aggregate = exact.clone();
        input_aggregate.inputs.push(FileRecord {
            uri: PortablePath::new("c.tsf").unwrap(),
            bytes: 1,
            sha256: [9; 32],
        });
        assert_eq!(
            validate_against(input_aggregate, &publication),
            Err(BuildManifestError::InputAggregateLimit)
        );

        let mut font_count = exact.clone();
        font_count.fonts.push(FontRecord {
            font_face_id: FontFaceId::new(2),
            uri: PortablePath::new("fonts/c.ttf").unwrap(),
            face_index: 0,
            bytes: 1,
            sha256: [10; 32],
            units_per_em: 1000,
            glyph_count: 1,
        });
        assert_eq!(
            validate_against(font_count, &publication),
            Err(BuildManifestError::FontCountLimit)
        );

        let mut font_bytes = exact.clone();
        font_bytes.fonts[0].bytes = 5;
        assert_eq!(
            validate_against(font_bytes, &publication),
            Err(BuildManifestError::FontBytesLimit)
        );

        let mut image_count = exact.clone();
        image_count.images.push(ImageRecord {
            image_id: ImageResourceId::new(2),
            uri: PortablePath::new("images/c.png").unwrap(),
            bytes: 1,
            sha256: [11; 32],
            pixel_width: 1,
            pixel_height: 1,
            decoded_bytes: 1,
        });
        assert_eq!(
            validate_against(image_count, &publication),
            Err(BuildManifestError::ImageCountLimit)
        );

        let mut image_bytes = exact.clone();
        image_bytes.images[0].bytes = 5;
        assert_eq!(
            validate_against(image_bytes, &publication),
            Err(BuildManifestError::ImageBytesLimit)
        );

        let mut aggregate = exact.clone();
        aggregate.images[1].bytes = 4;
        assert_eq!(
            validate_against(aggregate, &publication),
            Err(BuildManifestError::ResourceAggregateLimit)
        );

        let mut pages = exact.clone();
        pages.output.as_mut().unwrap().page_count = 3;
        assert_eq!(
            validate_against(pages, &publication),
            Err(BuildManifestError::PageLimit)
        );

        let mut output_bytes = exact.clone();
        output_bytes.output.as_mut().unwrap().bytes = 7;
        assert_eq!(
            validate_against(output_bytes, &publication),
            Err(BuildManifestError::OutputBytesLimit)
        );

        let mut objects = exact;
        objects.output.as_mut().unwrap().pdf_object_count = 3;
        assert_eq!(
            validate_against(objects, &publication),
            Err(BuildManifestError::PdfObjectLimit)
        );
    }

    #[test]
    fn failed_manifest_partial_facts_are_checked_against_limits() {
        let config = manifest_limit_config();
        let publication = publication_for(&config);
        let mut partial = built_manifest_at_limits(&config);
        partial.status = BuildStatus::Failed;
        partial.layout = None;
        partial.output = None;
        assert!(validate_against(partial.clone(), &publication).is_ok());

        partial.inputs.push(FileRecord {
            uri: PortablePath::new("c.tsf").unwrap(),
            bytes: 1,
            sha256: [9; 32],
        });
        assert_eq!(
            validate_against(partial, &publication),
            Err(BuildManifestError::InputAggregateLimit)
        );
    }
}
