use std::fs::File;
use std::io::Read;
use std::path::{Component, Path};

use typaxis_core::{
    DocumentFingerprint, EffectiveConfig, FontInstanceId, GeneratedBufferKey, GenerationKind,
    HostAdmissionContext, MachineInputFingerprint, MachinePdfProfileId, NonNegativeLength,
    PortablePath, ResolvedDataTables, SourceId, StyleFingerprint, TextSpan, Utf8ByteOffset,
    ValidatedResourceLimits,
};
use typaxis_diagnostics::MachineDiagnosticLender;
use typaxis_display_list::ValidatedDisplayDocument;
use typaxis_document::{Block, Inline, ReferenceFormat};
use typaxis_layout::{
    CanonicalFlowIrBuilder, FlowTree, LayoutEpoch, MachineGlyphCoverage,
    MachineParagraphFlowBuilder, MachineParagraphFlowError, MachineStyleFontPreparationError,
    MachineTextSiteSource, PreparedMachineStyleFonts, ShapeFontSelectionReceipt,
};
use typaxis_linebreak::{
    break_paragraph_validated, BoundedReferenceParagraphFactory, LineLayoutContext, LineShape,
    LineShapeExhaustion, OptimalParagraphBreaker, ParagraphShapedText, ReferenceSpaceGlue,
    ValidatedParagraphBreak, ValidatedParagraphItemRegistry,
};
use typaxis_machine_input::MachineInputSessionIdentity;
use typaxis_machine_profile::{
    MachinePdfPreflight, MachinePdfPreflightFailure, MachinePdfPreflightReceipt,
    MachinePdfReceiptMismatch,
};
use typaxis_pagination::{
    ConvergenceStatus, InitialPaginationState, PaginationError, PaginationResult,
    ReferencePaginator,
};
use typaxis_resources::{
    AdmittedFontInstanceTable, AdmittedResourceLedger, AdmittedResourceResolver,
    HostResourceAdmissionSession, ReferenceResourceFinalizer, ResourceError,
    ResourceFinalizationInput, ResourceFinalizer,
};
use typaxis_shaping::{CanonicalItemizer, LinkedShaper, ParagraphItemizationInput, ShapingCache};
use typaxis_syntax::{
    PackageShapeTextReceipt, PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser,
    SourceFile, ValidatedMachinePackage, ValidatedParsedPackage,
};
use typaxis_text::GeneratedTextStore;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureKind {
    Input,
    Usage,
    Io,
    Internal,
    Limit,
}

impl FailureKind {
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Input => 1,
            Self::Usage => 2,
            Self::Io => 3,
            Self::Internal => 4,
            Self::Limit => 5,
        }
    }
}

#[derive(Debug)]
pub struct Failure {
    pub kind: FailureKind,
    pub message: String,
    failed_manifest_policy: FailedManifestPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailedManifestPolicy {
    Publish,
    LeaveTargetsUntouched,
}

impl Failure {
    pub fn input(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Input,
            message: with_default_diagnostic_code(message.into(), "P1000"),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Usage,
            message: message.into(),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }
    pub fn io(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Io,
            message: message.into(),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Internal,
            message: with_default_diagnostic_code(message.into(), "I9001"),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }
    pub fn limit(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Limit,
            message: with_default_diagnostic_code(message.into(), "I9000"),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }

    fn capability_mismatch(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Internal,
            message: with_default_diagnostic_code(message.into(), "I9190"),
            failed_manifest_policy: FailedManifestPolicy::Publish,
        }
    }

    fn unsupported_contained_open() -> Self {
        Self {
            kind: FailureKind::Io,
            message: "resource admission I/O failed: UnsupportedContainedOpen".to_owned(),
            failed_manifest_policy: FailedManifestPolicy::LeaveTargetsUntouched,
        }
    }

    pub const fn should_publish_failed_manifest(&self) -> bool {
        matches!(self.failed_manifest_policy, FailedManifestPolicy::Publish)
    }
}

fn with_default_diagnostic_code(message: String, code: &str) -> String {
    if has_diagnostic_code(&message) {
        message
    } else {
        format!("{code}: {message}")
    }
}

fn has_diagnostic_code(message: &str) -> bool {
    let bytes = message.as_bytes();
    bytes.get(5) == Some(&b':')
        && matches!(
            bytes.get(0..2),
            Some(b"P1")
                | Some(b"T2")
                | Some(b"S3")
                | Some(b"F4")
                | Some(b"L5")
                | Some(b"G6")
                | Some(b"R7")
                | Some(b"D8")
                | Some(b"I9")
        )
        && bytes
            .get(2..5)
            .is_some_and(|digits| digits.iter().all(u8::is_ascii_digit))
}

pub fn load_package(
    input: &Path,
    config: &EffectiveConfig,
) -> Result<Box<ValidatedParsedPackage>, Failure> {
    let mut file = open_entry_source(input)?;
    #[cfg(all(
        unix,
        not(any(
            target_os = "espidf",
            target_os = "horizon",
            target_os = "solaris",
            target_os = "vita",
            target_os = "wasi"
        ))
    ))]
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockShared).map_err(
        |error| {
            Failure::io(format!(
                "cannot lock input `{}` for a stable read: {error}",
                input.display()
            ))
        },
    )?;
    let snapshot = InputFileSnapshot::from_file(&file, input)?;
    if !snapshot.regular {
        return Err(Failure::io(format!(
            "input `{}` is not a regular file",
            input.display()
        )));
    }
    let byte_length = snapshot.length;
    let limits = config.limits().get();
    if byte_length > limits.max_input_bytes || byte_length > u64::from(limits.max_source_bytes) {
        return Err(Failure::limit(format!(
            "input byte length {byte_length} exceeds the configured source/input limit"
        )));
    }
    let allocation = usize::try_from(byte_length)
        .map_err(|_| Failure::limit("input byte length exceeds this platform's address space"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(allocation)
        .map_err(|_| Failure::limit("cannot reserve the bounded input buffer"))?;
    bytes.resize(allocation, 0);
    file.read_exact(&mut bytes).map_err(|error| {
        Failure::io(format!("cannot read input `{}`: {error}", input.display()))
    })?;
    let final_snapshot = InputFileSnapshot::from_file(&file, input)?;
    let final_length = final_snapshot.length;
    if final_length > limits.max_input_bytes || final_length > u64::from(limits.max_source_bytes) {
        return Err(Failure::limit(format!(
            "input byte length {final_length} exceeds the configured source/input limit"
        )));
    }
    if final_snapshot != snapshot {
        return Err(Failure::io(format!(
            "input `{}` changed while it was being read",
            input.display()
        )));
    }
    let text = String::from_utf8(bytes).map_err(|error| {
        Failure::input(format!(
            "P1000: input is not UTF-8 (invalid byte at offset {})",
            error.utf8_error().valid_up_to()
        ))
    })?;
    preflight_reference_limits(&text, config.limits())?;
    let uri = logical_entry_uri(input);
    let source = SourceFile {
        source_id: SourceId::new(0),
        uri,
        text,
    };
    let policy = PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes())
        .map_err(|error| Failure::internal(format!("invalid URI policy: {error:?}")))?;
    match ReferenceParser::new().parse(&source, &policy) {
        ParseOutcome::Parsed {
            package,
            diagnostics,
        } => {
            for diagnostic in diagnostics {
                let diagnostic = diagnostic.as_diagnostic();
                crate::write_stderr_line(&format!(
                    "{}: {}",
                    diagnostic.code().as_str(),
                    diagnostic.message()
                ))?;
            }
            Ok(package)
        }
        ParseOutcome::Failed { failure } => {
            let message = failure
                .diagnostics()
                .iter()
                .map(|diagnostic| {
                    format!("{}: {}", diagnostic.code().as_str(), diagnostic.message())
                })
                .collect::<Vec<_>>()
                .join("\n");
            Err(Failure::input(message))
        }
    }
}

fn open_entry_source(input: &Path) -> Result<File, Failure> {
    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags};

        let descriptor = rustix::fs::open(
            input,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|error| {
            Failure::io(format!("cannot open input `{}`: {error}", input.display()))
        })?;
        Ok(descriptor.into())
    }
    #[cfg(not(unix))]
    {
        let metadata = std::fs::metadata(input).map_err(|error| {
            Failure::io(format!(
                "cannot inspect input `{}` before opening it: {error}",
                input.display()
            ))
        })?;
        if !metadata.is_file() {
            return Err(Failure::io(format!(
                "input `{}` is not a regular file",
                input.display()
            )));
        }
        File::open(input).map_err(|error| {
            Failure::io(format!("cannot open input `{}`: {error}", input.display()))
        })
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InputFileSnapshot {
    device: u128,
    inode: u128,
    length: u64,
    modified_seconds: i128,
    modified_nanoseconds: u128,
    changed_seconds: i128,
    changed_nanoseconds: u128,
    regular: bool,
}

#[cfg(any(target_os = "android", target_os = "linux"))]
impl InputFileSnapshot {
    fn from_file(file: &File, input: &Path) -> Result<Self, Failure> {
        let stat = rustix::fs::fstat(file).map_err(|error| {
            Failure::io(format!(
                "cannot inspect input `{}`: {error}",
                input.display()
            ))
        })?;
        Ok(Self {
            device: u128::from(stat.st_dev),
            inode: u128::from(stat.st_ino),
            length: u64::try_from(stat.st_size).map_err(|_| {
                Failure::limit(format!(
                    "input `{}` has an unsupported byte length",
                    input.display()
                ))
            })?,
            modified_seconds: i128::from(stat.st_mtime),
            modified_nanoseconds: u128::from(stat.st_mtime_nsec),
            changed_seconds: i128::from(stat.st_ctime),
            changed_nanoseconds: u128::from(stat.st_ctime_nsec),
            regular: rustix::fs::FileType::from_raw_mode(stat.st_mode)
                == rustix::fs::FileType::RegularFile,
        })
    }
}

#[cfg(all(unix, not(any(target_os = "android", target_os = "linux"))))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InputFileSnapshot {
    device: u64,
    inode: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
    regular: bool,
}

#[cfg(all(unix, not(any(target_os = "android", target_os = "linux"))))]
impl InputFileSnapshot {
    fn from_file(file: &File, input: &Path) -> Result<Self, Failure> {
        use std::os::unix::fs::MetadataExt;

        let metadata = file.metadata().map_err(|error| {
            Failure::io(format!(
                "cannot inspect input `{}`: {error}",
                input.display()
            ))
        })?;
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
            regular: metadata.is_file(),
        })
    }
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct InputFileSnapshot {
    length: u64,
    modified: std::time::SystemTime,
    regular: bool,
}

#[cfg(not(unix))]
impl InputFileSnapshot {
    fn from_file(file: &File, input: &Path) -> Result<Self, Failure> {
        let metadata = file.metadata().map_err(|error| {
            Failure::io(format!(
                "cannot inspect input `{}`: {error}",
                input.display()
            ))
        })?;
        let modified = metadata.modified().map_err(|error| {
            Failure::io(format!(
                "cannot read input `{}` modification time for a stable read: {error}",
                input.display()
            ))
        })?;
        Ok(Self {
            length: metadata.len(),
            modified,
            regular: metadata.is_file(),
        })
    }
}

fn logical_entry_uri(input: &Path) -> PortablePath {
    if !input.is_absolute() {
        let components: Vec<_> = input
            .components()
            .filter_map(|component| match component {
                Component::Normal(value) => value.to_str(),
                _ => None,
            })
            .collect();
        if components.len()
            == input
                .components()
                .filter(|c| !matches!(c, Component::CurDir))
                .count()
        {
            let candidate = components.join("/");
            if let Ok(path) = PortablePath::new(candidate) {
                return path;
            }
        }
    }
    if let Some(name) = input.file_name().and_then(|value| value.to_str()) {
        if let Ok(path) = PortablePath::new(name) {
            return path;
        }
    }
    PortablePath::new("input.tsf").expect("static portable path is valid")
}

fn preflight_reference_limits(text: &str, limits: &ValidatedResourceLimits) -> Result<(), Failure> {
    let limits = limits.get();
    let mut ast_nodes = 1u64;
    let mut text_bytes = 0u64;
    let mut deepest = 1u32;
    for raw_line in text.split_inclusive('\n') {
        let without_lf = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        if line.is_empty() {
            continue;
        }
        ast_nodes = ast_nodes
            .checked_add(1)
            .ok_or_else(|| Failure::limit("AST node count overflow"))?;
        deepest = deepest.max(2);
        if let Some(value) = line.strip_prefix("text:") {
            let length = u64::try_from(value.len())
                .map_err(|_| Failure::limit("text buffer byte length overflow"))?;
            if length > u64::from(limits.max_text_buffer_bytes) {
                return Err(Failure::limit(format!(
                    "text buffer byte length {length} exceeds max_text_buffer_bytes"
                )));
            }
            text_bytes = text_bytes
                .checked_add(length)
                .ok_or_else(|| Failure::limit("text byte count overflow"))?;
            ast_nodes = ast_nodes
                .checked_add(1)
                .ok_or_else(|| Failure::limit("AST node count overflow"))?;
            deepest = 3;
        } else if line.starts_with("anchor:") {
            ast_nodes = ast_nodes
                .checked_add(1)
                .ok_or_else(|| Failure::limit("AST node count overflow"))?;
            deepest = 3;
        }
    }
    if ast_nodes > limits.max_ast_nodes {
        return Err(Failure::limit(format!(
            "AST node count {ast_nodes} exceeds max_ast_nodes"
        )));
    }
    if deepest > limits.max_ast_nesting_depth {
        return Err(Failure::limit(format!(
            "AST nesting depth {deepest} exceeds max_ast_nesting_depth"
        )));
    }
    if text_bytes > limits.max_text_bytes {
        return Err(Failure::limit(format!(
            "text byte count {text_bytes} exceeds max_text_bytes"
        )));
    }
    Ok(())
}

/// Shared `check-package` preparation result. It is issued only after a
/// profile receipt has been revalidated, every declared resource has produced
/// a complete ledger, and every text-producing site has a computed family and
/// dense font-instance binding. Glyph coverage is intentionally deferred to
/// build-time shaping.
pub struct MachinePackagePreparation {
    profile: MachinePdfProfileId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    package_input: MachineInputFingerprint,
    session: MachineInputSessionIdentity,
    admitted: AdmittedResourceLedger,
    generated: GeneratedTextStore,
    style_fonts: PreparedMachineStyleFonts,
}

#[allow(dead_code)] // Some receipt facts remain private-runner/test observability until MI1-17.
impl MachinePackagePreparation {
    pub const fn profile(&self) -> MachinePdfProfileId {
        self.profile
    }

    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }

    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }

    pub const fn machine_input_fingerprint(&self) -> MachineInputFingerprint {
        self.package_input
    }

    pub const fn admitted(&self) -> &AdmittedResourceLedger {
        &self.admitted
    }

    pub const fn generated(&self) -> &GeneratedTextStore {
        &self.generated
    }

    pub const fn style_fonts(&self) -> &PreparedMachineStyleFonts {
        &self.style_fonts
    }

    pub const fn glyph_coverage(&self) -> MachineGlyphCoverage {
        self.style_fonts.glyph_coverage()
    }

    fn verify(
        &self,
        package: &ValidatedMachinePackage,
        receipt: &MachinePdfPreflightReceipt,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), Failure> {
        receipt
            .verify(MachinePdfProfileId::PARAGRAPH_1, package)
            .map_err(map_machine_receipt_mismatch)?;
        let identity = package.package().epoch_identity();
        if self.profile != MachinePdfProfileId::PARAGRAPH_1
            || self.document != identity.document()
            || self.style != identity.style()
            || self.package_input != package.provenance().fingerprint()
            || self.session != *package.provenance().session_identity()
            || !self
                .admitted
                .matches_declarations(&package.package().package().resources)
        {
            return Err(Failure::capability_mismatch(
                "machine preparation does not match the validated package",
            ));
        }
        let generated = package
            .package()
            .bind_generated_text(&self.generated, limits)
            .map_err(|error| {
                Failure::capability_mismatch(format!(
                    "machine generated-text preparation mismatch: {error:?}"
                ))
            })?;
        let epoch = LayoutEpoch::from_validated_inputs(generated, self.admitted.token()).map_err(
            |error| {
                Failure::capability_mismatch(format!(
                    "machine layout-epoch preparation mismatch: {error:?}"
                ))
            },
        )?;
        if self.style_fonts.epoch() != epoch
            || !self.style_fonts.matches_package_epoch(package, epoch)
        {
            return Err(Failure::capability_mismatch(
                "machine style/font preparation does not match the package epoch",
            ));
        }
        Ok(())
    }
}

/// Successful `check-package` boundary. Destructuring is required before the
/// build-only layout entry can consume both the non-cloneable profile receipt
/// and the complete preparation.
pub struct CheckedMachinePackage {
    receipt: MachinePdfPreflightReceipt,
    preparation: MachinePackagePreparation,
}

impl CheckedMachinePackage {
    #[allow(dead_code)] // Kept for the shared check boundary's typed observability.
    pub const fn receipt(&self) -> &MachinePdfPreflightReceipt {
        &self.receipt
    }

    pub const fn preparation(&self) -> &MachinePackagePreparation {
        &self.preparation
    }

    pub fn into_parts(self) -> (MachinePdfPreflightReceipt, MachinePackagePreparation) {
        (self.receipt, self.preparation)
    }
}

/// Capability-to-style/font preparation boundary shared by the future build
/// and check commands. The resource closure is unreachable until the closed
/// descriptor has issued a receipt.
#[allow(dead_code)] // Compatibility entry retained beside the phase-split CLI owner.
pub fn check_machine_package_preparation(
    package: &ValidatedMachinePackage,
    diagnostics: &mut MachineDiagnosticLender<'_>,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<CheckedMachinePackage, Failure> {
    let candidates = register_machine_resource_candidates(package, config, admission)?;
    let capability = preflight_machine_package(package, diagnostics, candidates)?;
    complete_machine_package_preparation(package, capability, config, admission)
        .map_err(MachinePreparationFailure::into_failure)
}

/// Candidate registration is a distinct observable phase: all safe declared
/// resource paths are in the command read ledger, but no resource bytes have
/// been opened and no capability receipt has been issued yet.
pub(crate) struct RegisteredMachineResourceCandidates(Option<HostResourceAdmissionSession>);

/// Successful capability gate retaining the unopened candidate session for
/// the following resource phase.
pub(crate) struct MachineCapabilityPreparation {
    receipt: MachinePdfPreflightReceipt,
    candidates: RegisteredMachineResourceCandidates,
}

impl MachineCapabilityPreparation {
    pub(crate) const fn receipt(&self) -> &MachinePdfPreflightReceipt {
        &self.receipt
    }
}

/// Resource facts established before a preparation failure. A complete
/// ledger is kept distinct from a partial owner-issued snapshot so the
/// manifest projection can advance to the exact highest trusted stage.
pub(crate) enum MachineResourcePreparationProgress {
    Partial(typaxis_resources::ResourceAdmissionProgressToken),
    Complete(AdmittedResourceLedger),
}

pub(crate) struct MachinePreparationFailure {
    failure: Failure,
    resource_progress: Option<MachineResourcePreparationProgress>,
}

impl MachinePreparationFailure {
    pub(crate) const fn resource_progress(&self) -> Option<&MachineResourcePreparationProgress> {
        self.resource_progress.as_ref()
    }

    pub(crate) fn into_failure(self) -> Failure {
        self.failure
    }

    fn before_resources(failure: Failure) -> Self {
        Self {
            failure,
            resource_progress: None,
        }
    }

    fn partial(
        failure: Failure,
        progress: typaxis_resources::ResourceAdmissionProgressToken,
    ) -> Self {
        Self {
            failure,
            resource_progress: Some(MachineResourcePreparationProgress::Partial(progress)),
        }
    }

    fn complete(failure: Failure, admitted: AdmittedResourceLedger) -> Self {
        Self {
            failure,
            resource_progress: Some(MachineResourcePreparationProgress::Complete(admitted)),
        }
    }
}

/// Apply the closed profile gate only after candidate registration. The
/// returned value still owns the unopened resource candidate session.
pub(crate) fn preflight_machine_package(
    package: &ValidatedMachinePackage,
    diagnostics: &mut MachineDiagnosticLender<'_>,
    candidates: RegisteredMachineResourceCandidates,
) -> Result<MachineCapabilityPreparation, Failure> {
    let receipt = MachinePdfPreflight::PARAGRAPH_1
        .run(package, diagnostics)
        .map_err(map_machine_preflight_failure)?;
    Ok(MachineCapabilityPreparation {
        receipt,
        candidates,
    })
}

/// Complete resource admission plus style/font-family coverage. Glyph
/// coverage, layout, pagination, and PDF construction remain outside this
/// boundary, which is the successful `check-package` endpoint.
pub(crate) fn complete_machine_package_preparation(
    package: &ValidatedMachinePackage,
    capability: MachineCapabilityPreparation,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<CheckedMachinePackage, MachinePreparationFailure> {
    let MachineCapabilityPreparation {
        receipt,
        candidates,
    } = capability;
    let preparation = prepare_machine_package_with_registered_progress(
        package, &receipt, config, admission, candidates,
    )?;
    Ok(CheckedMachinePackage {
        receipt,
        preparation,
    })
}

#[allow(dead_code)] // production seam for side-effect ordering tests
fn check_machine_package_preparation_with(
    package: &ValidatedMachinePackage,
    diagnostics: &mut MachineDiagnosticLender<'_>,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
    admit: impl FnOnce(
        &ValidatedParsedPackage,
        &EffectiveConfig,
        &HostAdmissionContext,
    ) -> Result<AdmittedResourceLedger, Failure>,
) -> Result<CheckedMachinePackage, Failure> {
    // Candidate parent+leaf identities are deliberately registered before the
    // capability gate. Unsupported resources/content remain unopened, but a
    // failure sidecar can no longer overwrite an existing or missing input
    // candidate through `--force`.
    let _candidates = register_machine_resource_candidates(package, config, admission)?;
    let receipt = MachinePdfPreflight::PARAGRAPH_1
        .run(package, diagnostics)
        .map_err(map_machine_preflight_failure)?;
    let preparation = prepare_machine_package_with(package, &receipt, config, admission, admit)?;
    Ok(CheckedMachinePackage {
        receipt,
        preparation,
    })
}

/// The shared internal boundary used by future `build-package` and
/// `check-package` orchestration. Receipt verification happens before the
/// resource-admission closure can be invoked.
#[allow(dead_code)] // Public lower boundary retained for receipt-gating tests.
pub fn prepare_machine_package(
    package: &ValidatedMachinePackage,
    receipt: &MachinePdfPreflightReceipt,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<MachinePackagePreparation, Failure> {
    let candidates = register_machine_resource_candidates(package, config, admission)?;
    prepare_machine_package_with_registered_progress(
        package, receipt, config, admission, candidates,
    )
    .map_err(MachinePreparationFailure::into_failure)
}

pub(crate) fn register_machine_resource_candidates(
    package: &ValidatedMachinePackage,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<RegisteredMachineResourceCandidates, Failure> {
    let declarations = &package.package().package().resources;
    if declarations.font_faces.is_empty() && declarations.images.is_empty() {
        return Ok(RegisteredMachineResourceCandidates(None));
    }
    HostResourceAdmissionSession::new_with_read_ledger(
        admission,
        config,
        declarations,
        package.provenance().admission().read_ledger(),
    )
    .map(Some)
    .map(RegisteredMachineResourceCandidates)
    .map_err(map_admission_error)
}

fn prepare_machine_package_with_registered_progress(
    package: &ValidatedMachinePackage,
    receipt: &MachinePdfPreflightReceipt,
    config: &EffectiveConfig,
    _admission: &HostAdmissionContext,
    candidates: RegisteredMachineResourceCandidates,
) -> Result<MachinePackagePreparation, MachinePreparationFailure> {
    receipt
        .verify(MachinePdfProfileId::PARAGRAPH_1, package)
        .map_err(map_machine_receipt_mismatch)
        .map_err(MachinePreparationFailure::before_resources)?;
    let parsed = package.package();
    let generated = parsed
        .materialize_initial_generated_text(config.limits())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "machine generated-text setup contradicted preflight: {error:?}"
            ))
        })
        .map_err(MachinePreparationFailure::before_resources)?;
    let admitted = admit_registered_resources_with_progress(parsed, config, candidates.0).map_err(
        |failure| match failure.progress {
            Some(progress) => MachinePreparationFailure::partial(failure.failure, progress),
            None => MachinePreparationFailure::before_resources(failure.failure),
        },
    )?;
    let generated_binding = match parsed.bind_generated_text(&generated, config.limits()) {
        Ok(binding) => binding,
        Err(error) => {
            return Err(MachinePreparationFailure::complete(
                Failure::capability_mismatch(format!(
                    "machine generated-text binding contradicted preflight: {error:?}"
                )),
                admitted,
            ));
        }
    };
    let style_fonts =
        match PreparedMachineStyleFonts::prepare(package, generated_binding, admitted.token()) {
            Ok(prepared) => prepared,
            Err(error) => {
                return Err(MachinePreparationFailure::complete(
                    map_machine_style_font_error(error),
                    admitted,
                ));
            }
        };
    let identity = parsed.epoch_identity();
    let preparation = MachinePackagePreparation {
        profile: MachinePdfProfileId::PARAGRAPH_1,
        document: identity.document(),
        style: identity.style(),
        package_input: package.provenance().fingerprint(),
        session: package.provenance().session_identity().clone(),
        admitted,
        generated,
        style_fonts,
    };
    if let Err(failure) = preparation.verify(package, receipt, config.limits()) {
        let MachinePackagePreparation { admitted, .. } = preparation;
        return Err(MachinePreparationFailure::complete(failure, admitted));
    }
    Ok(preparation)
}

#[allow(dead_code)] // production seam for side-effect ordering tests
fn prepare_machine_package_with(
    package: &ValidatedMachinePackage,
    receipt: &MachinePdfPreflightReceipt,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
    admit: impl FnOnce(
        &ValidatedParsedPackage,
        &EffectiveConfig,
        &HostAdmissionContext,
    ) -> Result<AdmittedResourceLedger, Failure>,
) -> Result<MachinePackagePreparation, Failure> {
    receipt
        .verify(MachinePdfProfileId::PARAGRAPH_1, package)
        .map_err(map_machine_receipt_mismatch)?;
    let parsed = package.package();
    let generated = parsed
        .materialize_initial_generated_text(config.limits())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "machine generated-text setup contradicted preflight: {error:?}"
            ))
        })?;
    // This is the first operation permitted to open declared resource bytes.
    let admitted = admit(parsed, config, admission)?;
    let generated_binding = parsed
        .bind_generated_text(&generated, config.limits())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "machine generated-text binding contradicted preflight: {error:?}"
            ))
        })?;
    let style_fonts =
        PreparedMachineStyleFonts::prepare(package, generated_binding, admitted.token())
            .map_err(map_machine_style_font_error)?;
    let identity = parsed.epoch_identity();
    let preparation = MachinePackagePreparation {
        profile: MachinePdfProfileId::PARAGRAPH_1,
        document: identity.document(),
        style: identity.style(),
        package_input: package.provenance().fingerprint(),
        session: package.provenance().session_identity().clone(),
        admitted,
        generated,
        style_fonts,
    };
    preparation.verify(package, receipt, config.limits())?;
    Ok(preparation)
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_receipt_mismatch(error: MachinePdfReceiptMismatch) -> Failure {
    Failure::capability_mismatch(format!("machine capability receipt mismatch: {error}"))
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_preflight_failure(error: MachinePdfPreflightFailure) -> Failure {
    match error {
        MachinePdfPreflightFailure::Unsupported {
            violation_count,
            primary_code,
        } => Failure::input(format!(
            "{primary_code}: machine PDF profile rejected {violation_count} capability violation(s)"
        )),
        MachinePdfPreflightFailure::WrongDiagnosticPhase
        | MachinePdfPreflightFailure::DiagnosticBudget(_) => Failure::internal(format!(
            "machine capability preflight orchestration failed: {error:?}"
        )),
    }
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_style_font_error(error: MachineStyleFontPreparationError) -> Failure {
    match error {
        MachineStyleFontPreparationError::Style(_)
        | MachineStyleFontPreparationError::LayoutStyle(_) => Failure::input(format!(
            "L5101: machine style/font coverage rejected the input: {error:?}"
        )),
        MachineStyleFontPreparationError::Resource(
            typaxis_resources::ResourceAdmissionError::ResourceLimit,
        )
        | MachineStyleFontPreparationError::ResourceLimit => {
            Failure::limit("machine style/font coverage exceeded a resource limit")
        }
        MachineStyleFontPreparationError::Resource(error) => map_admission_error(error),
        _ => Failure::capability_mismatch(format!(
            "machine style/font coverage contradicted preflight: {error:?}"
        )),
    }
}

#[derive(Debug)]
pub struct ReferenceLayout {
    pub admitted: AdmittedResourceLedger,
    pub flow: FlowTree,
    pub initial: InitialPaginationState,
    pub pagination: PaginationResult,
}

pub fn layout_reference(
    package: &ValidatedParsedPackage,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<ReferenceLayout, Failure> {
    if !package.package().document.footnotes.is_empty() {
        return Err(Failure::input(
            "L5000: the reference layout backend does not yet accept footnotes",
        ));
    }
    let limits = config.limits();
    let generated = package
        .materialize_initial_generated_text(limits)
        .map_err(|error| Failure::internal(format!("generated text setup failed: {error:?}")))?;
    let admitted = admit_resources(package, config, admission)?;
    let generated_binding = package
        .bind_generated_text(&generated, limits)
        .map_err(|error| Failure::internal(format!("generated text binding failed: {error:?}")))?;
    let epoch = LayoutEpoch::from_validated_inputs(generated_binding, admitted.token())
        .map_err(|error| Failure::internal(format!("layout epoch setup failed: {error:?}")))?;
    let flow = build_reference_flow(package, generated_binding, &admitted, epoch, config)?;
    let initial =
        InitialPaginationState::new(&flow, package, limits).map_err(map_pagination_error)?;
    // Always materialize a fallback in the reference owner. The caller emits
    // trace/failed-manifest facts before applying strict rejection.
    let outcome = ReferencePaginator::new()
        .paginate_with_reflow(package, &flow, limits, false, |store, working_epoch| {
            let binding = package
                .bind_generated_text(store, limits)
                .map_err(|_| PaginationError::PackageEpochMismatch)?;
            build_reference_flow(package, binding, &admitted, working_epoch, config)
                .map_err(|_| PaginationError::FatalLayout)
        })
        .map_err(map_pagination_error)?;
    if !config.strict() {
        for advisory in outcome.diagnostics() {
            let diagnostic = advisory.as_diagnostic();
            crate::write_stderr_line(&format!(
                "{}: {}",
                diagnostic.code().as_str(),
                diagnostic.message()
            ))?;
        }
    }
    let pagination = outcome.into_result();
    Ok(ReferenceLayout {
        admitted,
        flow,
        initial,
        pagination,
    })
}

/// Machine paragraph layout owns the already-complete preparation; no layout
/// or finalization consumer can observe a partial resource resolver.
#[allow(dead_code)] // wired to public command dispatch by MI1-15
pub struct MachineParagraphLayout {
    preparation: MachinePackagePreparation,
    flow: FlowTree,
    initial: InitialPaginationState,
    pagination: PaginationResult,
}

#[allow(dead_code)] // wired to public command dispatch by MI1-15
impl MachineParagraphLayout {
    pub const fn preparation(&self) -> &MachinePackagePreparation {
        &self.preparation
    }

    pub const fn flow(&self) -> &FlowTree {
        &self.flow
    }

    pub const fn initial(&self) -> &InitialPaginationState {
        &self.initial
    }

    pub const fn pagination(&self) -> &PaginationResult {
        &self.pagination
    }
}

/// Receipt-gated machine layout entry. There is no profile-ID argument and no
/// overload accepting a bare `ValidatedParsedPackage`.
#[allow(dead_code)] // wired to public command dispatch by MI1-15
pub fn layout_machine_paragraphs(
    package: &ValidatedMachinePackage,
    receipt: &MachinePdfPreflightReceipt,
    preparation: MachinePackagePreparation,
    config: &EffectiveConfig,
) -> Result<MachineParagraphLayout, Failure> {
    preparation.verify(package, receipt, config.limits())?;
    let parsed = package.package();
    let generated = parsed
        .bind_generated_text(preparation.generated(), config.limits())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "machine generated-text binding mismatch at layout: {error:?}"
            ))
        })?;
    let epoch = LayoutEpoch::from_validated_inputs(generated, preparation.admitted().token())
        .map_err(|error| {
            Failure::capability_mismatch(format!(
                "machine layout epoch mismatch at layout: {error:?}"
            ))
        })?;
    let flow = build_machine_flow(package, generated, &preparation, epoch, config)?;
    let initial = InitialPaginationState::new(&flow, parsed, config.limits())
        .map_err(map_machine_pagination_error)?;
    let mut reflow_failure = None;
    let outcome = ReferencePaginator::new().paginate_with_reflow(
        parsed,
        &flow,
        config.limits(),
        false,
        |store, working_epoch| {
            let binding = match parsed.bind_generated_text(store, config.limits()) {
                Ok(binding) => binding,
                Err(error) => {
                    reflow_failure = Some(Failure::capability_mismatch(format!(
                        "machine generated-text reflow mismatch: {error:?}"
                    )));
                    return Err(PaginationError::PackageEpochMismatch);
                }
            };
            match build_machine_flow(package, binding, &preparation, working_epoch, config) {
                Ok(flow) => Ok(flow),
                Err(failure) => {
                    reflow_failure = Some(failure);
                    Err(PaginationError::FatalLayout)
                }
            }
        },
    );
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(_error) if reflow_failure.is_some() => {
            return Err(reflow_failure.expect("guarded machine reflow failure"))
        }
        Err(error) => return Err(map_machine_pagination_error(error)),
    };
    if !config.strict() {
        for advisory in outcome.diagnostics() {
            let diagnostic = advisory.as_diagnostic();
            crate::write_stderr_line(&format!(
                "{}: {}",
                diagnostic.code().as_str(),
                diagnostic.message()
            ))?;
        }
    }
    let pagination = outcome.into_result();
    Ok(MachineParagraphLayout {
        preparation,
        flow,
        initial,
        pagination,
    })
}

fn build_reference_flow(
    package: &ValidatedParsedPackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    admitted: &AdmittedResourceLedger,
    epoch: LayoutEpoch,
    config: &EffectiveConfig,
) -> Result<FlowTree, Failure> {
    let paragraph_breaks = layout_paragraphs(package, generated, admitted, epoch, config)?;
    let paragraph_items = ValidatedParagraphItemRegistry::from_breaks_allowing_empty(
        package,
        epoch,
        &paragraph_breaks,
    )
    .map_err(|error| Failure::input(format!("L5000: unsupported flow content: {error:?}")))?;
    let mut flow_builder = CanonicalFlowIrBuilder::new(package, &paragraph_items)
        .map_err(|error| Failure::input(format!("L5000: unsupported flow content: {error:?}")))?;
    for (node, item_count) in paragraph_items.paragraphs() {
        for item_index in 0..item_count {
            flow_builder
                .push_paragraph_item(node, item_index)
                .map_err(|error| {
                    Failure::internal(format!("flow construction failed: {error:?}"))
                })?;
        }
    }
    let flow = flow_builder
        .finish(epoch)
        .map_err(|error| Failure::internal(format!("flow construction failed: {error:?}")))?;
    Ok(flow)
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn build_machine_flow(
    package: &ValidatedMachinePackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    preparation: &MachinePackagePreparation,
    epoch: LayoutEpoch,
    config: &EffectiveConfig,
) -> Result<FlowTree, Failure> {
    let parsed = package.package();
    if !preparation
        .style_fonts()
        .matches_package_epoch(package, epoch)
    {
        return Err(Failure::capability_mismatch(
            "machine style/font coverage does not match the working layout epoch",
        ));
    }
    let paragraph_breaks =
        layout_machine_paragraphs_for_epoch(package, generated, preparation, epoch, config)?;
    let paragraph_items = ValidatedParagraphItemRegistry::from_breaks_allowing_empty(
        parsed,
        epoch,
        &paragraph_breaks,
    )
    .map_err(map_machine_break_error)?;
    let mut flow_builder =
        MachineParagraphFlowBuilder::new(package, &paragraph_items, preparation.style_fonts())
            .map_err(map_machine_flow_error)?;
    for (node, item_count) in paragraph_items.paragraphs() {
        for item_index in 0..item_count {
            flow_builder
                .push_paragraph_item(node, item_index)
                .map_err(map_machine_flow_error)?;
        }
    }
    flow_builder.finish(epoch).map_err(map_machine_flow_error)
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_break_error(error: typaxis_linebreak::BreakError) -> Failure {
    if error == typaxis_linebreak::BreakError::UnsupportedFlowDomain {
        Failure::capability_mismatch(
            "descriptor-approved content reached an unsupported paragraph-flow domain",
        )
    } else {
        Failure::internal(format!(
            "machine paragraph registry invariant failed: {error:?}"
        ))
    }
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_flow_error(error: MachineParagraphFlowError) -> Failure {
    match error {
        MachineParagraphFlowError::PreparationMismatch
        | MachineParagraphFlowError::Flow(typaxis_layout::FlowTreeError::UnsupportedFlowDomain) => {
            Failure::capability_mismatch(
                "descriptor-approved content reached an unsupported layout domain",
            )
        }
        MachineParagraphFlowError::Flow(error) => {
            Failure::internal(format!("machine flow construction failed: {error:?}"))
        }
    }
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn map_machine_pagination_error(error: PaginationError) -> Failure {
    if error == PaginationError::FatalLayout {
        Failure::capability_mismatch(
            "descriptor-approved content reached an unsupported pagination/layout domain",
        )
    } else {
        map_pagination_error(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParagraphTextSite {
    Parsed(TextSpan),
    Generated(GeneratedBufferKey),
}

#[derive(Clone, Copy)]
#[allow(dead_code)] // machine variant is wired to dispatch by MI1-15
enum ParagraphStyleFonts<'a> {
    Reference(&'a AdmittedFontInstanceTable),
    Machine(&'a PreparedMachineStyleFonts),
}

#[allow(dead_code)] // machine variant is wired to dispatch by MI1-15
impl<'a> ParagraphStyleFonts<'a> {
    const fn font_instances(self) -> &'a AdmittedFontInstanceTable {
        match self {
            Self::Reference(instances) => instances,
            Self::Machine(prepared) => prepared.font_instances(),
        }
    }

    const fn is_machine(self) -> bool {
        matches!(self, Self::Machine(_))
    }

    fn computed_style(
        self,
        package: &ValidatedParsedPackage,
        site: ParagraphTextSite,
        text: &PackageShapeTextReceipt<'_>,
    ) -> Result<(typaxis_syntax::PackageComputedStyle, Option<FontInstanceId>), Failure> {
        match self {
            Self::Reference(_) => package
                .cascade_style(text.site_owner())
                .map(|computed| (computed, None))
                .map_err(|error| {
                    Failure::input(format!(
                        "L5000: paragraph style resolution failed: {error:?}"
                    ))
                }),
            Self::Machine(prepared) => {
                let source = match site {
                    ParagraphTextSite::Parsed(span) => MachineTextSiteSource::Parsed(span),
                    ParagraphTextSite::Generated(key) => MachineTextSiteSource::Generated(key),
                };
                let prepared_site = prepared.site(source, text.site_owner()).ok_or_else(|| {
                    Failure::capability_mismatch(
                        "descriptor-approved text site is missing from style/font preparation",
                    )
                })?;
                if prepared_site.site_owner() != text.site_owner()
                    || prepared_site.style_owner() != text.style_owner()
                {
                    return Err(Failure::capability_mismatch(
                        "prepared text-site ownership does not match the package",
                    ));
                }
                Ok((
                    prepared_site.computed().clone(),
                    Some(prepared_site.font_instance_id()),
                ))
            }
        }
    }

    fn unsupported(self, message: &'static str) -> Failure {
        if self.is_machine() {
            Failure::capability_mismatch(message)
        } else {
            Failure::input(format!("L5000: {message}"))
        }
    }
}

fn map_paragraph_break_failure(
    error: typaxis_linebreak::BreakError,
    style_fonts: ParagraphStyleFonts<'_>,
    operation: &'static str,
) -> Failure {
    if style_fonts.is_machine() && error == typaxis_linebreak::BreakError::UnsupportedFlowDomain {
        Failure::capability_mismatch(
            "descriptor-approved content reached an unsupported paragraph operation",
        )
    } else {
        Failure::input(format!("L5000: {operation} failed: {error:?}"))
    }
}

fn layout_paragraphs(
    package: &ValidatedParsedPackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    admitted: &AdmittedResourceLedger,
    epoch: LayoutEpoch,
    config: &EffectiveConfig,
) -> Result<Vec<ValidatedParagraphBreak>, Failure> {
    if package
        .package()
        .document
        .blocks
        .iter()
        .any(|block| !matches!(block, Block::Paragraph { .. } | Block::Heading { .. }))
    {
        return Err(Failure::input(
            "L5000: the reference layout backend accepts only paragraph and heading blocks",
        ));
    }
    let used_faces = package
        .package()
        .resources
        .font_faces
        .iter()
        .map(|font| font.font_face_id);
    let instances = AdmittedFontInstanceTable::from_used_faces(admitted, used_faces)
        .map_err(map_admission_error)?;
    layout_paragraphs_with_fonts(
        package,
        generated,
        admitted,
        epoch,
        config,
        ParagraphStyleFonts::Reference(&instances),
    )
}

#[allow(dead_code)] // reachable from the MI1-15 command integration
fn layout_machine_paragraphs_for_epoch(
    package: &ValidatedMachinePackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    preparation: &MachinePackagePreparation,
    epoch: LayoutEpoch,
    config: &EffectiveConfig,
) -> Result<Vec<ValidatedParagraphBreak>, Failure> {
    let parsed = package.package();
    if !parsed.package().document.footnotes.is_empty()
        || parsed
            .package()
            .document
            .blocks
            .iter()
            .any(|block| !matches!(block, Block::Paragraph { .. } | Block::Heading { .. }))
    {
        return Err(Failure::capability_mismatch(
            "descriptor-approved content reached an unsupported paragraph backend domain",
        ));
    }
    layout_paragraphs_with_fonts(
        parsed,
        generated,
        preparation.admitted(),
        epoch,
        config,
        ParagraphStyleFonts::Machine(preparation.style_fonts()),
    )
}

fn layout_paragraphs_with_fonts(
    package: &ValidatedParsedPackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    admitted: &AdmittedResourceLedger,
    epoch: LayoutEpoch,
    config: &EffectiveConfig,
    style_fonts: ParagraphStyleFonts<'_>,
) -> Result<Vec<ValidatedParagraphBreak>, Failure> {
    let versions = config.data_versions();
    let data_tables =
        ResolvedDataTables::resolve(versions.unicode(), versions.japanese_line_break())
            .ok_or_else(|| Failure::internal("configured Unicode data tables are not linked"))?;
    let inline_size = package
        .package()
        .page_masters
        .masters
        .iter()
        .find(|master| master.master_id == package.package().page_masters.default_master_id)
        .map(|master| master.body.width())
        .ok_or_else(|| Failure::internal("default page master is missing"))?;
    let line_shapes = [LineShape { inline_size }];
    let space_glue = ReferenceSpaceGlue::new(NonNegativeLength::ZERO, NonNegativeLength::ZERO);
    let mut cache = ShapingCache::new(config.limits());
    let mut breaks = Vec::new();
    breaks
        .try_reserve_exact(package.package().document.blocks.len())
        .map_err(|_| Failure::limit("paragraph layout allocation failed"))?;
    for block in &package.package().document.blocks {
        if let Some(receipt) = layout_paragraph(
            package,
            generated,
            admitted,
            style_fonts,
            epoch,
            &data_tables,
            block,
            space_glue,
            &line_shapes,
            &mut cache,
            config.limits(),
        )? {
            breaks.push(receipt);
        }
    }
    Ok(breaks)
}

#[allow(clippy::too_many_arguments)]
fn layout_paragraph(
    package: &ValidatedParsedPackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'_>,
    admitted: &AdmittedResourceLedger,
    style_fonts: ParagraphStyleFonts<'_>,
    epoch: LayoutEpoch,
    data_tables: &ResolvedDataTables,
    block: &Block,
    space_glue: ReferenceSpaceGlue,
    line_shapes: &[LineShape],
    cache: &mut ShapingCache,
    limits: &ValidatedResourceLimits,
) -> Result<Option<ValidatedParagraphBreak>, Failure> {
    let (paragraph_node, children) = match block {
        Block::Paragraph {
            node_id, children, ..
        }
        | Block::Heading {
            node_id, children, ..
        } => (*node_id, children.as_slice()),
        _ => {
            return Err(style_fonts
                .unsupported("unsupported block reached paragraph layout after profile preflight"))
        }
    };
    let mut sites = Vec::new();
    let mut has_explicit_break = false;
    collect_paragraph_text_sites(children, &mut sites, &mut has_explicit_break);
    if sites.is_empty() && !has_explicit_break {
        return Ok(None);
    }
    let breaker = OptimalParagraphBreaker;
    let factory = BoundedReferenceParagraphFactory::new();
    let break_canonical = |paragraph: &typaxis_linebreak::CanonicalParagraph| {
        let mut context = LineLayoutContext::from_limits(limits);
        let mut budget = context.take_budget().map_err(|error| {
            Failure::internal(format!("line-layout budget setup failed: {error:?}"))
        })?;
        break_paragraph_validated(&breaker, &paragraph.input(), &mut budget)
            .map_err(|error| map_paragraph_break_failure(error, style_fonts, "line layout"))
    };
    if sites.is_empty() {
        let paragraph = factory
            .build(
                generated,
                paragraph_node,
                epoch,
                &[],
                space_glue,
                line_shapes,
                LineShapeExhaustion::RepeatLast,
                limits,
            )
            .map_err(|error| {
                map_paragraph_break_failure(error, style_fonts, "paragraph construction")
            })?;
        return break_canonical(&paragraph).map(Some);
    }
    let mut inputs = Vec::new();
    inputs
        .try_reserve_exact(sites.len())
        .map_err(|_| Failure::limit("paragraph itemization allocation failed"))?;
    for site in sites {
        let text = bind_paragraph_text_site(package, generated, site)?;
        let (computed, expected_instance) = style_fonts.computed_style(package, site, &text)?;
        let selection = ShapeFontSelectionReceipt::new(
            package,
            &computed,
            admitted.token(),
            style_fonts.font_instances(),
            epoch,
        )
        .map_err(|error| {
            if style_fonts.is_machine() {
                Failure::capability_mismatch(format!(
                    "prepared font selection contradicted machine preflight: {error:?}"
                ))
            } else {
                Failure::input(format!("L5000: font selection failed: {error:?}"))
            }
        })?;
        if expected_instance
            .is_some_and(|expected| expected != selection.admitted_font().font_instance_id())
        {
            return Err(Failure::capability_mismatch(
                "prepared font instance changed before shaping",
            ));
        }
        inputs.push(ParagraphItemizationInput::new(computed, text, selection));
    }
    let itemized = CanonicalItemizer::new()
        .itemize_paragraph(package, paragraph_node, &inputs, epoch, data_tables, limits)
        .map_err(|error| Failure::input(format!("L5000: itemization failed: {error:?}")))?;
    let mut run_sets = Vec::new();
    run_sets
        .try_reserve_exact(itemized.len())
        .map_err(|_| Failure::limit("shaped-run allocation failed"))?;
    for site in &itemized {
        let mut runs = Vec::new();
        if let Some(site) = site {
            runs.try_reserve_exact(site.requests().len())
                .map_err(|_| Failure::limit("shaped-run allocation failed"))?;
            for request in site.requests() {
                runs.push(
                    cache
                        .shape(&LinkedShaper::new(), request.clone())
                        .map_err(|error| {
                            Failure::input(format!("L5000: shaping failed: {error:?}"))
                        })?,
                );
            }
        }
        run_sets.push(runs);
    }
    let shaped: Vec<_> = inputs
        .iter()
        .zip(&itemized)
        .zip(&run_sets)
        .map(|((input, itemized), runs)| match itemized {
            Some(itemized) => ParagraphShapedText::from_itemized(itemized, runs),
            None => ParagraphShapedText::empty(input.text_receipt()),
        })
        .collect();
    let paragraph = factory
        .build(
            generated,
            paragraph_node,
            epoch,
            &shaped,
            space_glue,
            line_shapes,
            LineShapeExhaustion::RepeatLast,
            limits,
        )
        .map_err(|error| {
            map_paragraph_break_failure(error, style_fonts, "paragraph construction")
        })?;
    break_canonical(&paragraph).map(Some)
}

fn collect_paragraph_text_sites(
    inlines: &[Inline],
    output: &mut Vec<ParagraphTextSite>,
    has_explicit_break: &mut bool,
) {
    for inline in inlines {
        match inline {
            Inline::Text { text_span, .. } => {
                output.push(ParagraphTextSite::Parsed(*text_span));
            }
            Inline::Reference {
                node_id, format, ..
            } => {
                let kind = match format {
                    ReferenceFormat::Page => GenerationKind::PageReference,
                    ReferenceFormat::Text | ReferenceFormat::Number => GenerationKind::Counter,
                };
                output.push(ParagraphTextSite::Generated(GeneratedBufferKey::new(
                    *node_id, kind, 0,
                )));
            }
            Inline::FootnoteReference { node_id, .. } => {
                output.push(ParagraphTextSite::Generated(GeneratedBufferKey::new(
                    *node_id,
                    GenerationKind::FootnoteMarker,
                    0,
                )));
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => {
                collect_paragraph_text_sites(children, output, has_explicit_break);
            }
            Inline::SoftBreak { .. } | Inline::HardBreak { .. } => {
                *has_explicit_break = true;
            }
            Inline::Anchor { .. } => {}
        }
    }
}

fn bind_paragraph_text_site<'a>(
    package: &'a ValidatedParsedPackage,
    generated: typaxis_syntax::PackageGeneratedTextBinding<'a>,
    site: ParagraphTextSite,
) -> Result<PackageShapeTextReceipt<'a>, Failure> {
    match site {
        ParagraphTextSite::Parsed(span) => package
            .bind_parsed_shape_text(span)
            .map_err(|error| Failure::internal(format!("parsed text binding failed: {error:?}"))),
        ParagraphTextSite::Generated(key) => {
            let buffer = generated
                .generated_text()
                .buffers()
                .iter()
                .find(|buffer| buffer.key() == key)
                .ok_or_else(|| Failure::internal("generated text site is missing"))?;
            let end = u32::try_from(buffer.utf8().len())
                .map_err(|_| Failure::limit("generated text site is too large"))?;
            let provenance = generated
                .generated_text()
                .provenance(key, Utf8ByteOffset::new(0), Utf8ByteOffset::new(end))
                .map_err(|error| {
                    Failure::internal(format!("generated text provenance failed: {error:?}"))
                })?;
            generated
                .bind_generated_shape_text(provenance)
                .map_err(|error| {
                    Failure::internal(format!("generated text binding failed: {error:?}"))
                })
        }
    }
}

pub fn admit_resources(
    package: &ValidatedParsedPackage,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
) -> Result<AdmittedResourceLedger, Failure> {
    let declarations = &package.package().resources;
    if declarations.font_faces.is_empty() && declarations.images.is_empty() {
        return AdmittedResourceResolver::new(declarations, config.limits())
            .and_then(AdmittedResourceResolver::finish)
            .map_err(map_admission_error);
    }
    let session = HostResourceAdmissionSession::new(admission, config, declarations)
        .map_err(map_admission_error)?;
    admit_registered_resources(package, config, Some(session))
}

fn admit_registered_resources(
    package: &ValidatedParsedPackage,
    config: &EffectiveConfig,
    session: Option<HostResourceAdmissionSession>,
) -> Result<AdmittedResourceLedger, Failure> {
    let declarations = &package.package().resources;
    let Some(session) = session.as_ref() else {
        return AdmittedResourceResolver::new(declarations, config.limits())
            .and_then(AdmittedResourceResolver::finish)
            .map_err(map_admission_error);
    };
    let mut resolver =
        AdmittedResourceResolver::new_with_roots(declarations, config.limits(), session.roots())
            .map_err(map_admission_error)?;
    for declaration in &declarations.font_faces {
        let source = session
            .open_font(declaration.font_face_id)
            .map_err(map_admission_error)?;
        let pending = resolver.read_font(source).map_err(map_admission_error)?;
        resolver
            .parse_and_bind_sfnt(pending)
            .map_err(map_admission_error)?;
    }
    for declaration in &declarations.images {
        let source = session
            .open_image(declaration.image_id)
            .map_err(map_admission_error)?;
        let pending = resolver.read_image(source).map_err(map_admission_error)?;
        resolver
            .parse_and_bind_png(pending)
            .map_err(map_admission_error)?;
    }
    resolver.finish().map_err(map_admission_error)
}

struct ResourceAdmissionOrchestrationFailure {
    failure: Failure,
    progress: Option<typaxis_resources::ResourceAdmissionProgressToken>,
}

/// Machine orchestration retains the resolver-issued progress snapshot on
/// every failure after resolver construction. The compatibility wrapper above
/// intentionally keeps its historical `Failure` surface for reference code.
fn admit_registered_resources_with_progress(
    package: &ValidatedParsedPackage,
    config: &EffectiveConfig,
    session: Option<HostResourceAdmissionSession>,
) -> Result<AdmittedResourceLedger, ResourceAdmissionOrchestrationFailure> {
    let declarations = &package.package().resources;
    let mut resolver = match session.as_ref() {
        Some(session) => {
            AdmittedResourceResolver::new_with_roots(declarations, config.limits(), session.roots())
        }
        None => AdmittedResourceResolver::new(declarations, config.limits()),
    }
    .map_err(|error| ResourceAdmissionOrchestrationFailure {
        failure: map_admission_error(error),
        progress: None,
    })?;

    let Some(session) = session.as_ref() else {
        let progress = resolver.progress_token();
        return resolver
            .finish()
            .map_err(|error| ResourceAdmissionOrchestrationFailure {
                failure: map_admission_error(error),
                progress: Some(progress),
            });
    };

    for declaration in &declarations.font_faces {
        let source = session
            .open_font_with_subject(declaration.font_face_id)
            .map_err(|failure| ResourceAdmissionOrchestrationFailure {
                failure: map_admission_error(failure.error()),
                progress: Some(resolver.progress_token()),
            })?;
        let pending = resolver
            .read_font_with_subject(source)
            .map_err(map_resource_admission_outcome)?;
        resolver
            .parse_and_bind_sfnt_with_subject(pending)
            .map_err(map_resource_admission_outcome)?;
    }
    for declaration in &declarations.images {
        let source = session
            .open_image_with_subject(declaration.image_id)
            .map_err(|failure| ResourceAdmissionOrchestrationFailure {
                failure: map_admission_error(failure.error()),
                progress: Some(resolver.progress_token()),
            })?;
        let pending = resolver
            .read_image_with_subject(source)
            .map_err(map_resource_admission_outcome)?;
        resolver
            .parse_and_bind_png_with_subject(pending)
            .map_err(map_resource_admission_outcome)?;
    }
    let progress = resolver.progress_token();
    resolver
        .finish()
        .map_err(|error| ResourceAdmissionOrchestrationFailure {
            failure: map_admission_error(error),
            progress: Some(progress),
        })
}

fn map_resource_admission_outcome(
    outcome: typaxis_resources::ResourceAdmissionFailureOutcome,
) -> ResourceAdmissionOrchestrationFailure {
    let (failure, progress) = outcome.into_parts();
    ResourceAdmissionOrchestrationFailure {
        failure: map_admission_error(failure.error()),
        progress: Some(progress),
    }
}

pub fn reject_strict_fallback(
    layout: &ReferenceLayout,
    config: &EffectiveConfig,
) -> Result<(), Failure> {
    if config.strict() && !matches!(layout.pagination.status(), ConvergenceStatus::Converged) {
        Err(Failure::input(
            "G6001: strict mode rejected pagination fallback; no PDF was generated",
        ))
    } else {
        Ok(())
    }
}

pub fn reject_machine_strict_fallback(
    layout: &MachineParagraphLayout,
    config: &EffectiveConfig,
) -> Result<(), Failure> {
    if config.strict() && !matches!(layout.pagination.status(), ConvergenceStatus::Converged) {
        Err(Failure::input(
            "G6001: strict mode rejected pagination fallback; no PDF was generated",
        ))
    } else {
        Ok(())
    }
}

pub fn build_pdf_graph(
    package: &ValidatedParsedPackage,
    config: &EffectiveConfig,
    layout: &ReferenceLayout,
) -> Result<typaxis_pdf::FrozenPdfGraph, Failure> {
    let display = ValidatedDisplayDocument::paint_reference_paragraphs(
        package,
        &layout.pagination,
        layout.pagination.selected_flow(),
        config,
    )
    .map_err(|error| Failure::internal(format!("display construction failed: {error:?}")))?;
    let plans = ReferenceResourceFinalizer::new()
        .finalize(ResourceFinalizationInput {
            display: &display,
            admitted: &layout.admitted,
            limits: config.limits(),
        })
        .map_err(map_resource_error)?;
    typaxis_pdf::PdfBackend::build(display, plans, config.limits()).map_err(|error| match error {
        typaxis_pdf::PdfError::ObjectLimit | typaxis_pdf::PdfError::OutputTooLarge => {
            Failure::limit(format!("PDF resource limit exceeded: {error:?}"))
        }
        _ => Failure::internal(format!("PDF graph construction failed: {error:?}")),
    })
}

/// Machine PDF graph entry rechecks the same capability/session preparation
/// before consuming the complete admitted ledger. `check-package` stops before
/// this function and therefore performs no pagination, shaping, or PDF work.
#[allow(dead_code)] // wired to public command dispatch by MI1-15
pub fn build_machine_pdf_graph(
    package: &ValidatedMachinePackage,
    receipt: &MachinePdfPreflightReceipt,
    config: &EffectiveConfig,
    layout: &MachineParagraphLayout,
) -> Result<typaxis_pdf::FrozenPdfGraph, Failure> {
    layout
        .preparation
        .verify(package, receipt, config.limits())?;
    let display = ValidatedDisplayDocument::paint_reference_paragraphs(
        package.package(),
        &layout.pagination,
        layout.pagination.selected_flow(),
        config,
    )
    .map_err(|error| Failure::internal(format!("display construction failed: {error:?}")))?;
    let plans = ReferenceResourceFinalizer::new()
        .finalize(ResourceFinalizationInput {
            display: &display,
            admitted: layout.preparation.admitted(),
            limits: config.limits(),
        })
        .map_err(map_resource_error)?;
    typaxis_pdf::PdfBackend::build(display, plans, config.limits()).map_err(|error| match error {
        typaxis_pdf::PdfError::ObjectLimit | typaxis_pdf::PdfError::OutputTooLarge => {
            Failure::limit(format!("PDF resource limit exceeded: {error:?}"))
        }
        _ => Failure::internal(format!("PDF graph construction failed: {error:?}")),
    })
}

fn map_pagination_error(error: PaginationError) -> Failure {
    match error {
        PaginationError::ResourceLimit | PaginationError::PageLimit => {
            Failure::limit(format!("pagination resource limit exceeded: {error:?}"))
        }
        PaginationError::FallbackRejectedByStrict => {
            Failure::input("G6001: strict mode rejected pagination fallback; no PDF was generated")
        }
        _ => Failure::internal(format!("pagination invariant failed: {error:?}")),
    }
}

fn map_admission_error(error: typaxis_resources::ResourceAdmissionError) -> Failure {
    use typaxis_resources::ResourceAdmissionError as Error;
    match error {
        Error::ResourceLimit => {
            Failure::limit(format!("resource admission limit exceeded: {error:?}"))
        }
        Error::MissingLogicalResource
        | Error::ConflictingLogicalResource
        | Error::ExpectedHashMismatch
        | Error::InvalidMetadata
        | Error::InvalidFontFamily
        | Error::MissingResourceCandidate
        | Error::AmbiguousResourceCandidate
        | Error::UnsafeResourceCandidate
        | Error::ResourceNotRegularFile => {
            Failure::input(format!("resource admission rejected the input: {error:?}"))
        }
        Error::UnsupportedContainedOpen => Failure::unsupported_contained_open(),
        Error::RootUnavailable
        | Error::RootNotDirectory
        | Error::ResourceRead
        | Error::ResourceLengthMismatch
        | Error::ResourceLockUnavailable => {
            Failure::io(format!("resource admission I/O failed: {error:?}"))
        }
        Error::AliasedRoot => {
            Failure::usage("configured resource roots resolve to the same directory")
        }
        Error::NonCanonicalResourceId
        | Error::ReceiptKindMismatch
        | Error::ReceiptIdentityMismatch
        | Error::ReceiptSessionMismatch
        | Error::MissingAdmittedRootSet
        | Error::RootSetMismatch => {
            Failure::internal(format!("resource admission invariant failed: {error:?}"))
        }
    }
}

fn map_resource_error(error: ResourceError) -> Failure {
    match error {
        ResourceError::ResourceLimit => {
            Failure::limit(format!("resource finalization limit exceeded: {error:?}"))
        }
        _ => Failure::internal(format!("resource finalization failed: {error:?}")),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::Cell;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use typaxis_core::{
        sha256, ConfigResourceRoot, DocumentPackageContractId, EffectiveDataVersions, HostPath,
        PdfStreamCompression, ResourceLimits,
    };
    use typaxis_diagnostics::{MachineDiagnosticBudget, MachineDiagnosticPhase, L5100};
    use typaxis_document_package as wire;
    use typaxis_machine_input::{HostMachineInputSession, MachineInputHostOptions};
    use typaxis_syntax::{DocumentPackageParser, MachineParseOutcome};

    fn config() -> EffectiveConfig {
        EffectiveConfig::new(
            false,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            ["http", "https", "mailto", "tel"]
                .map(str::to_owned)
                .to_vec(),
            EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
            ResourceLimits::default(),
        )
        .unwrap()
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    struct MachineFixtureRoot(PathBuf);

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    impl MachineFixtureRoot {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "typaxis-mi1-11-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    impl Drop for MachineFixtureRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    struct MachineFixture {
        _root: MachineFixtureRoot,
        package: Box<ValidatedMachinePackage>,
        admission: HostAdmissionContext,
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[derive(Clone, Copy)]
    enum MachineFixtureKind {
        Blank,
        Paragraph,
        Heading,
        AnchorPageReference,
        SoftHardBreaks,
        MissingGlyph,
        UnsupportedInline,
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_source_span() -> wire::WireSourceSpan {
        wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_default_master() -> wire::WirePageMaster {
        wire::WirePageMaster {
            master_id: "default".to_owned(),
            width: 10_000_000,
            height: 10_000_000,
            body: wire::WireRect {
                x: 0,
                y: 0,
                width: 10_000_000,
                height: 10_000_000,
            },
            header: None,
            footer: None,
            footnote: None,
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_style(selector: &str) -> wire::WireStyleRule {
        wire::WireStyleRule {
            style_id: format!("{selector}_style"),
            extends: None,
            selector: selector.to_owned(),
            source_order: 0,
            declarations: vec![
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::FontFamily,
                    value: wire::WireStyleValue::FontFamilyList {
                        families: vec!["Fixture".to_owned()],
                    },
                    important: false,
                },
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::FontSize,
                    value: wire::WireStyleValue::Length { value: 786_432 },
                    important: false,
                },
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::LineHeight,
                    value: wire::WireStyleValue::Length { value: 917_504 },
                    important: false,
                },
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::Page,
                    value: wire::WireStyleValue::Keyword {
                        value: "auto".to_owned(),
                    },
                    important: false,
                },
            ],
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_text_buffer(text: &str) -> wire::WireTextBuffer {
        wire::WireTextBuffer {
            text_id: 0,
            utf8: text.to_owned(),
            mappings: vec![wire::WireTextMapSegment {
                text_range: wire::WireByteRange {
                    start_byte: 0,
                    end_byte: u32::try_from(text.len()).unwrap(),
                },
                kind: wire::WireTextMapKind::Inserted,
                source_span: None,
            }],
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_text_inline(text: &str, node_id: u32) -> wire::WireInline {
        wire::WireInline::Text {
            node_id,
            span: machine_source_span(),
            text_span: wire::WireTextSpan {
                text_id: 0,
                start_byte: 0,
                end_byte: u32::try_from(text.len()).unwrap(),
            },
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_paragraph(node_id: u32, children: Vec<wire::WireInline>) -> wire::WireBlock {
        wire::WireBlock::Paragraph {
            node_id,
            span: machine_source_span(),
            classes: Vec::new(),
            children,
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_wire(kind: MachineFixtureKind) -> wire::WireDocumentPackage {
        let mut package = wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_0,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "input.tsf".to_owned(),
                utf8_byte_length: 0,
                sha256: sha256(&[]),
            }],
            text_buffers: Vec::new(),
            document: wire::WireDocument {
                node_id: 0,
                blocks: Vec::new(),
                footnotes: Vec::new(),
            },
            style_sheet: wire::WireStyleSheet { rules: Vec::new() },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![machine_default_master()],
                selection_rules: Vec::new(),
            },
            resources: wire::WireResourceCatalog {
                font_faces: Vec::new(),
                images: Vec::new(),
            },
        };
        let mut add_font = || {
            package.resources.font_faces = vec![wire::WireFontFace {
                font_face_id: 0,
                family: "Fixture".to_owned(),
                uri: "body.ttf".to_owned(),
                face_index: 0,
                expected_sha256: Some(sha256(&synthetic_ascii_ttf())),
            }];
        };
        match kind {
            MachineFixtureKind::Blank => {}
            MachineFixtureKind::Paragraph => {
                let text = "paragraph";
                add_font();
                package.text_buffers = vec![machine_text_buffer(text)];
                package.document.blocks =
                    vec![machine_paragraph(1, vec![machine_text_inline(text, 2)])];
                package.style_sheet.rules = vec![machine_style("paragraph")];
            }
            MachineFixtureKind::Heading => {
                let text = "heading";
                add_font();
                package.text_buffers = vec![machine_text_buffer(text)];
                package.document.blocks = vec![wire::WireBlock::Heading {
                    node_id: 1,
                    span: machine_source_span(),
                    classes: Vec::new(),
                    level: 2,
                    anchor_id: Some("heading".to_owned()),
                    children: vec![machine_text_inline(text, 2)],
                }];
                package.style_sheet.rules = vec![machine_style("heading")];
            }
            MachineFixtureKind::AnchorPageReference => {
                add_font();
                package.document.blocks = vec![
                    machine_paragraph(
                        1,
                        vec![wire::WireInline::Anchor {
                            node_id: 2,
                            span: machine_source_span(),
                            anchor_id: "target".to_owned(),
                        }],
                    ),
                    machine_paragraph(
                        3,
                        vec![wire::WireInline::Reference {
                            node_id: 4,
                            span: machine_source_span(),
                            target: "target".to_owned(),
                            format: wire::WireReferenceFormat::Page,
                        }],
                    ),
                ];
                package.style_sheet.rules = vec![machine_style("paragraph")];
            }
            MachineFixtureKind::SoftHardBreaks => {
                package.document.blocks = vec![machine_paragraph(
                    1,
                    vec![
                        wire::WireInline::SoftBreak {
                            node_id: 2,
                            span: machine_source_span(),
                        },
                        wire::WireInline::HardBreak {
                            node_id: 3,
                            span: machine_source_span(),
                        },
                    ],
                )];
            }
            MachineFixtureKind::MissingGlyph => {
                let text = "é";
                add_font();
                package.text_buffers = vec![machine_text_buffer(text)];
                package.document.blocks =
                    vec![machine_paragraph(1, vec![machine_text_inline(text, 2)])];
                package.style_sheet.rules = vec![machine_style("paragraph")];
            }
            MachineFixtureKind::UnsupportedInline => {
                let text = "unsupported";
                add_font();
                package.text_buffers = vec![machine_text_buffer(text)];
                package.document.blocks = vec![machine_paragraph(
                    1,
                    vec![wire::WireInline::Emphasis {
                        node_id: 2,
                        span: machine_source_span(),
                        children: vec![machine_text_inline(text, 3)],
                    }],
                )];
                package.style_sheet.rules = vec![machine_style("paragraph")];
            }
        }
        package
    }

    /// Materialize the same sealed machine fixtures used by the lower-pipeline
    /// tests for private CLI-runner tests. Keeping one wire fixture owner avoids
    /// a second hand-maintained paragraph/font model in `main.rs`.
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    pub(crate) fn write_machine_runner_fixture(root: &Path, kind: &str) {
        let kind = match kind {
            "blank" => MachineFixtureKind::Blank,
            "paragraph" => MachineFixtureKind::Paragraph,
            "unsupported-inline" => MachineFixtureKind::UnsupportedInline,
            other => panic!("unknown machine runner fixture `{other}`"),
        };
        fs::create_dir_all(root).unwrap();
        let bytes = wire::DocumentPackageEncoder::default()
            .to_jcs_vec(&machine_wire(kind))
            .unwrap();
        fs::write(root.join("document-package.json"), bytes).unwrap();
        fs::write(root.join("input.tsf"), []).unwrap();
        fs::write(root.join("body.ttf"), synthetic_ascii_ttf()).unwrap();
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn parse_machine_fixture(label: &str, kind: MachineFixtureKind) -> MachineFixture {
        let root = MachineFixtureRoot::new(label);
        let package_path = root.path().join("document-package.json");
        let bytes = wire::DocumentPackageEncoder::default()
            .to_jcs_vec(&machine_wire(kind))
            .unwrap();
        fs::write(&package_path, bytes).unwrap();
        fs::write(root.path().join("input.tsf"), []).unwrap();
        fs::write(root.path().join("body.ttf"), synthetic_ascii_ttf()).unwrap();
        let config = config();
        let options =
            MachineInputHostOptions::new(HostPath::new(package_path.clone()).unwrap(), None);
        let (session, raw) = HostMachineInputSession::open(options, config.limits()).unwrap();
        let decoded = session
            .decode_and_bind(
                &raw,
                &wire::StrictDocumentPackageDecoder::new(),
                &wire::DocumentPackageDecodePolicy::new(config.limits()),
            )
            .unwrap();
        let sources = session.admit_sources(&decoded, config.limits()).unwrap();
        let admitted = session.finish(raw, decoded, sources).unwrap();
        let policy =
            PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes()).unwrap();
        let package = match DocumentPackageParser::new().parse(admitted, &policy) {
            MachineParseOutcome::Parsed { package } => package,
            MachineParseOutcome::Failed { failure, .. } => panic!("fixture failed: {failure}"),
        };
        let admission = HostAdmissionContext::new(
            HostPath::new(package_path).unwrap(),
            HostPath::new(root.path().to_path_buf()).unwrap(),
            None,
            Vec::new(),
        );
        MachineFixture {
            _root: root,
            package,
            admission,
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn synthetic_ascii_ttf() -> Vec<u8> {
        const GLYPHS: u16 = 96;
        let mut head = vec![0; 54];
        head[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
        head[18..20].copy_from_slice(&1000u16.to_be_bytes());
        head[38..40].copy_from_slice(&(-200i16).to_be_bytes());
        head[40..42].copy_from_slice(&1000i16.to_be_bytes());
        head[42..44].copy_from_slice(&800i16.to_be_bytes());
        head[46..48].copy_from_slice(&8u16.to_be_bytes());
        head[48..50].copy_from_slice(&2i16.to_be_bytes());
        head[50..52].copy_from_slice(&1i16.to_be_bytes());
        let mut hhea = vec![0; 36];
        hhea[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        hhea[4..6].copy_from_slice(&800i16.to_be_bytes());
        hhea[6..8].copy_from_slice(&(-200i16).to_be_bytes());
        hhea[10..12].copy_from_slice(&600u16.to_be_bytes());
        hhea[34..36].copy_from_slice(&GLYPHS.to_be_bytes());
        let mut maxp = vec![0; 32];
        maxp[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        maxp[4..6].copy_from_slice(&GLYPHS.to_be_bytes());
        let mut hmtx = Vec::with_capacity(usize::from(GLYPHS) * 4);
        for glyph in 0..GLYPHS {
            hmtx.extend_from_slice(&(if glyph == 1 { 300u16 } else { 600u16 }).to_be_bytes());
            hmtx.extend_from_slice(&0i16.to_be_bytes());
        }
        let loca = vec![0; (usize::from(GLYPHS) + 1) * 4];
        let mut cmap = vec![0; 44];
        cmap[2..4].copy_from_slice(&1u16.to_be_bytes());
        cmap[4..6].copy_from_slice(&3u16.to_be_bytes());
        cmap[6..8].copy_from_slice(&1u16.to_be_bytes());
        cmap[8..12].copy_from_slice(&12u32.to_be_bytes());
        cmap[12..14].copy_from_slice(&4u16.to_be_bytes());
        cmap[14..16].copy_from_slice(&32u16.to_be_bytes());
        cmap[18..20].copy_from_slice(&4u16.to_be_bytes());
        cmap[20..22].copy_from_slice(&4u16.to_be_bytes());
        cmap[22..24].copy_from_slice(&1u16.to_be_bytes());
        cmap[26..28].copy_from_slice(&0x007eu16.to_be_bytes());
        cmap[28..30].copy_from_slice(&0xffffu16.to_be_bytes());
        cmap[32..34].copy_from_slice(&0x0020u16.to_be_bytes());
        cmap[34..36].copy_from_slice(&0xffffu16.to_be_bytes());
        cmap[36..38].copy_from_slice(&(-31i16).to_be_bytes());
        cmap[38..40].copy_from_slice(&1i16.to_be_bytes());
        let postscript_name = b"TypaxisSynthetic";
        let mut name = vec![0; 18 + postscript_name.len() * 2];
        name[2..4].copy_from_slice(&1u16.to_be_bytes());
        name[4..6].copy_from_slice(&18u16.to_be_bytes());
        name[6..8].copy_from_slice(&3u16.to_be_bytes());
        name[8..10].copy_from_slice(&1u16.to_be_bytes());
        name[10..12].copy_from_slice(&0x0409u16.to_be_bytes());
        name[12..14].copy_from_slice(&6u16.to_be_bytes());
        name[14..16].copy_from_slice(&(postscript_name.len() as u16 * 2).to_be_bytes());
        for (index, byte) in postscript_name.iter().copied().enumerate() {
            name[19 + index * 2] = byte;
        }
        let mut post = vec![0; 32];
        post[..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
        build_machine_test_sfnt(vec![
            (*b"cmap", cmap),
            (*b"glyf", vec![]),
            (*b"head", head),
            (*b"hhea", hhea),
            (*b"hmtx", hmtx),
            (*b"loca", loca),
            (*b"maxp", maxp),
            (*b"name", name),
            (*b"post", post),
        ])
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn build_machine_test_sfnt(mut tables: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
        tables.sort_by_key(|(tag, _)| *tag);
        let count = tables.len() as u16;
        let directory_len = 12 + tables.len() * 16;
        let payload_len: usize = tables.iter().map(|(_, bytes)| (bytes.len() + 3) & !3).sum();
        let mut output = vec![0; directory_len + payload_len];
        output[..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        output[4..6].copy_from_slice(&count.to_be_bytes());
        let selector = u16::try_from(u16::BITS - 1 - count.leading_zeros()).unwrap();
        let search = 16u16 * (1u16 << selector);
        output[6..8].copy_from_slice(&search.to_be_bytes());
        output[8..10].copy_from_slice(&selector.to_be_bytes());
        output[10..12].copy_from_slice(&(count * 16 - search).to_be_bytes());
        let mut offset = directory_len;
        let mut head_adjustment = None;
        for (index, (tag, bytes)) in tables.iter().enumerate() {
            let record = 12 + index * 16;
            output[record..record + 4].copy_from_slice(tag);
            output[record + 4..record + 8]
                .copy_from_slice(&machine_test_sfnt_checksum(bytes).to_be_bytes());
            output[record + 8..record + 12].copy_from_slice(&(offset as u32).to_be_bytes());
            output[record + 12..record + 16].copy_from_slice(&(bytes.len() as u32).to_be_bytes());
            output[offset..offset + bytes.len()].copy_from_slice(bytes);
            if tag == b"head" {
                head_adjustment = Some(offset + 8);
            }
            offset = (offset + bytes.len() + 3) & !3;
        }
        if let Some(offset) = head_adjustment {
            let adjustment = 0xB1B0_AFBAu32.wrapping_sub(machine_test_sfnt_checksum(&output));
            output[offset..offset + 4].copy_from_slice(&adjustment.to_be_bytes());
        }
        output
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_test_sfnt_checksum(bytes: &[u8]) -> u32 {
        bytes.chunks(4).fold(0u32, |checksum, chunk| {
            let mut word = [0; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            checksum.wrapping_add(u32::from_be_bytes(word))
        })
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_preflight_receipt(package: &ValidatedMachinePackage) -> MachinePdfPreflightReceipt {
        let mut budget = MachineDiagnosticBudget::new();
        let receipt = {
            let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
            MachinePdfPreflight::PARAGRAPH_1
                .run(package, &mut diagnostics)
                .unwrap()
        };
        assert!(budget.finish().diagnostics().is_empty());
        receipt
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn machine_supported_domain_reaches_a_frozen_pdf_graph() {
        for (label, kind) in [
            ("blank", MachineFixtureKind::Blank),
            ("paragraph", MachineFixtureKind::Paragraph),
            ("heading", MachineFixtureKind::Heading),
            ("anchor-reference", MachineFixtureKind::AnchorPageReference),
            ("soft-hard-breaks", MachineFixtureKind::SoftHardBreaks),
        ] {
            let fixture = parse_machine_fixture(label, kind);
            let config = config();
            let mut budget = MachineDiagnosticBudget::new();
            let checked = {
                let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
                check_machine_package_preparation(
                    &fixture.package,
                    &mut diagnostics,
                    &config,
                    &fixture.admission,
                )
                .unwrap()
            };
            assert!(budget.finish().diagnostics().is_empty());
            assert_eq!(
                checked.preparation().glyph_coverage(),
                MachineGlyphCoverage::DeferredToBuildShaping
            );
            if label == "soft-hard-breaks" {
                assert_eq!(
                    checked
                        .preparation()
                        .style_fonts()
                        .non_text_generated_sites()
                        .len(),
                    2
                );
                assert!(checked.preparation().style_fonts().sites().is_empty());
            }
            if label == "anchor-reference" {
                assert_eq!(checked.preparation().style_fonts().sites().len(), 1);
                assert_eq!(
                    checked.preparation().style_fonts().sites()[0].site_owner(),
                    typaxis_core::NodeId::new(4)
                );
            }
            let (receipt, preparation) = checked.into_parts();
            let layout =
                layout_machine_paragraphs(&fixture.package, &receipt, preparation, &config)
                    .unwrap_or_else(|error| panic!("{label} layout failed: {error:?}"));
            assert!(!layout.pagination().selected_pages().is_empty());
            let _graph = build_machine_pdf_graph(&fixture.package, &receipt, &config, &layout)
                .unwrap_or_else(|error| panic!("{label} PDF graph failed: {error:?}"));
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn check_preparation_resolves_family_but_defers_missing_glyph_coverage() {
        let fixture = parse_machine_fixture("missing-glyph", MachineFixtureKind::MissingGlyph);
        let config = config();
        let receipt = machine_preflight_receipt(&fixture.package);
        let preparation =
            prepare_machine_package(&fixture.package, &receipt, &config, &fixture.admission)
                .unwrap();
        assert_eq!(preparation.style_fonts().sites().len(), 1);
        assert_eq!(
            preparation.style_fonts().sites()[0].site_owner(),
            typaxis_core::NodeId::new(2)
        );
        assert_eq!(
            preparation.glyph_coverage(),
            MachineGlyphCoverage::DeferredToBuildShaping
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn receipt_swap_and_wrong_document_are_i9190_internal_failures() {
        let first = parse_machine_fixture("receipt-first", MachineFixtureKind::Paragraph);
        let same_bytes_other_session =
            parse_machine_fixture("receipt-session", MachineFixtureKind::Paragraph);
        let different_document =
            parse_machine_fixture("receipt-document", MachineFixtureKind::Heading);
        let config = config();
        let first_receipt = machine_preflight_receipt(&first.package);
        let session_receipt = machine_preflight_receipt(&same_bytes_other_session.package);
        let document_receipt = machine_preflight_receipt(&different_document.package);

        let preparation =
            prepare_machine_package(&first.package, &first_receipt, &config, &first.admission)
                .unwrap();
        let error =
            match layout_machine_paragraphs(&first.package, &session_receipt, preparation, &config)
            {
                Err(error) => error,
                Ok(_) => panic!("a receipt from another session must fail"),
            };
        assert_eq!(error.kind, FailureKind::Internal);
        assert!(error.message.starts_with("I9190:"));

        let foreign_preparation = prepare_machine_package(
            &same_bytes_other_session.package,
            &session_receipt,
            &config,
            &same_bytes_other_session.admission,
        )
        .unwrap();
        let error = match layout_machine_paragraphs(
            &first.package,
            &first_receipt,
            foreign_preparation,
            &config,
        ) {
            Err(error) => error,
            Ok(_) => panic!("a preparation from another session must fail"),
        };
        assert_eq!(error.kind, FailureKind::Internal);
        assert!(error.message.starts_with("I9190:"));

        let preparation =
            prepare_machine_package(&first.package, &first_receipt, &config, &first.admission)
                .unwrap();
        let error = match layout_machine_paragraphs(
            &first.package,
            &document_receipt,
            preparation,
            &config,
        ) {
            Err(error) => error,
            Ok(_) => panic!("a receipt for another document must fail"),
        };
        assert_eq!(error.kind, FailureKind::Internal);
        assert!(error.message.starts_with("I9190:"));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn unsupported_capability_stops_resource_layout_and_pdf_stages() {
        let fixture = parse_machine_fixture("unsupported", MachineFixtureKind::UnsupportedInline);
        let config = config();
        let resource_opens = Cell::new(0u32);
        let layout_starts = Cell::new(0u32);
        let pdf_starts = Cell::new(0u32);
        let mut budget = MachineDiagnosticBudget::new();
        let checked = {
            let mut diagnostics = budget.lend(MachineDiagnosticPhase::Capability).unwrap();
            check_machine_package_preparation_with(
                &fixture.package,
                &mut diagnostics,
                &config,
                &fixture.admission,
                |package, config, admission| {
                    resource_opens.set(resource_opens.get() + 1);
                    admit_resources(package, config, admission)
                },
            )
        };
        let result = checked.and_then(|checked| {
            layout_starts.set(layout_starts.get() + 1);
            let (receipt, preparation) = checked.into_parts();
            let layout =
                layout_machine_paragraphs(&fixture.package, &receipt, preparation, &config)?;
            pdf_starts.set(pdf_starts.get() + 1);
            build_machine_pdf_graph(&fixture.package, &receipt, &config, &layout).map(|_| ())
        });
        let error = result.unwrap_err();
        assert_eq!(error.kind, FailureKind::Input);
        assert_eq!(resource_opens.get(), 0);
        assert_eq!(layout_starts.get(), 0);
        assert_eq!(pdf_starts.get(), 0);
        let read_ledger = fixture
            .package
            .provenance()
            .admission()
            .read_ledger_token()
            .unwrap();
        assert!(read_ledger
            .conflicts_with_write_target(
                &HostPath::new(fixture._root.path().join("body.ttf")).unwrap()
            )
            .unwrap());
        assert_eq!(read_ledger.stored_opened_identity_count(), 2);
        let diagnostics = budget.finish();
        assert_eq!(diagnostics.diagnostics().len(), 1);
        assert_eq!(*diagnostics.diagnostics()[0].code(), L5100);
    }

    #[test]
    fn unexpected_machine_flow_domain_is_i9190_not_user_input() {
        let failure = map_machine_flow_error(MachineParagraphFlowError::Flow(
            typaxis_layout::FlowTreeError::UnsupportedFlowDomain,
        ));
        assert_eq!(failure.kind, FailureKind::Internal);
        assert!(failure.message.starts_with("I9190:"));
        let pagination = map_machine_pagination_error(PaginationError::FatalLayout);
        assert_eq!(pagination.kind, FailureKind::Internal);
        assert!(pagination.message.starts_with("I9190:"));
    }

    #[test]
    fn diagnostic_failures_receive_stable_default_codes_without_double_prefixing() {
        assert_eq!(Failure::input("bad source").message, "P1000: bad source");
        assert_eq!(
            Failure::limit("too much work").message,
            "I9000: too much work"
        );
        assert_eq!(
            Failure::internal("broken invariant").message,
            "I9001: broken invariant"
        );
        assert_eq!(
            Failure::input("L5000: unsupported layout").message,
            "L5000: unsupported layout"
        );
        assert_eq!(
            Failure::limit("P1001: invalid limit configuration").message,
            "P1001: invalid limit configuration"
        );
    }

    #[test]
    fn unsupported_contained_open_leaves_requested_artifact_targets_untouched() {
        let failure = map_admission_error(
            typaxis_resources::ResourceAdmissionError::UnsupportedContainedOpen,
        );
        assert_eq!(failure.kind, FailureKind::Io);
        assert_eq!(
            failure.message,
            "resource admission I/O failed: UnsupportedContainedOpen"
        );
        assert!(!failure.should_publish_failed_manifest());
        assert!(Failure::io("ordinary I/O failure").should_publish_failed_manifest());
    }

    static NEXT_TEMP_SOURCE: AtomicU64 = AtomicU64::new(0);

    fn temp_source(contents: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_TEMP_SOURCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "typaxis-cli-{}-{unique}-{sequence}.tsf",
            std::process::id()
        ));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        path
    }

    #[test]
    fn empty_source_reaches_a_converged_blank_page() {
        let path = temp_source("\n");
        let config = config();
        let package = load_package(&path, &config).unwrap();
        let admission = HostAdmissionContext::new(
            typaxis_core::HostPath::new(path.clone()).unwrap(),
            typaxis_core::HostPath::new(path.parent().unwrap().to_path_buf()).unwrap(),
            None,
            vec![],
        );
        let layout = layout_reference(&package, &config, &admission).unwrap();
        assert_eq!(layout.pagination.selected_pages().len(), 1);
        assert_eq!(layout.pagination.passes().len(), 2);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn empty_paragraph_uses_reference_fragmentation_and_pagination() {
        let path = temp_source("paragraph\n");
        let config = config();
        let package = load_package(&path, &config).unwrap();
        let admission = HostAdmissionContext::new(
            typaxis_core::HostPath::new(path.clone()).unwrap(),
            typaxis_core::HostPath::new(path.parent().unwrap().to_path_buf()).unwrap(),
            None,
            vec![],
        );
        let layout = layout_reference(&package, &config, &admission).unwrap();
        assert_eq!(layout.pagination.selected_pages().len(), 1);
        assert_eq!(layout.pagination.selected_pages()[0].fragments.len(), 1);
        assert_eq!(layout.pagination.passes().len(), 2);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn entry_source_rejects_a_directory() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("typaxis-cli-directory-{unique}"));
        fs::create_dir(&path).unwrap();

        let error = load_package(&path, &config()).unwrap_err();
        assert_eq!(error.kind, FailureKind::Io);
        assert!(error.message.contains("not a regular file"));

        fs::remove_dir(path).unwrap();
    }

    #[test]
    fn entry_snapshot_detects_same_length_timestamp_change() {
        let path = temp_source("paragraph\n");
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.set_times(
            fs::FileTimes::new()
                .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000)),
        )
        .unwrap();
        let first = InputFileSnapshot::from_file(&file, &path).unwrap();

        file.set_times(
            fs::FileTimes::new()
                .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_001)),
        )
        .unwrap();
        let second = InputFileSnapshot::from_file(&file, &path).unwrap();

        assert_eq!(first.length, second.length);
        assert_ne!(first, second);
        drop(file);
        fs::remove_file(path).unwrap();
    }

    #[cfg(all(
        unix,
        not(any(
            target_os = "espidf",
            target_os = "horizon",
            target_os = "solaris",
            target_os = "vita",
            target_os = "wasi"
        ))
    ))]
    #[test]
    fn entry_source_rejects_a_conflicting_writer_lock() {
        let path = temp_source("\n");
        let writer = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        rustix::fs::flock(
            &writer,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .unwrap();

        let error = load_package(&path, &config()).unwrap_err();
        assert_eq!(error.kind, FailureKind::Io);
        assert!(error.message.contains("stable read"));

        drop(writer);
        fs::remove_file(path).unwrap();
    }

    #[cfg(any(target_os = "android", target_os = "linux"))]
    #[test]
    fn entry_source_rejects_a_fifo_without_blocking() {
        use rustix::fs::{Mode, CWD};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("typaxis-cli-fifo-{unique}"));
        rustix::fs::mkfifoat(CWD, &path, Mode::RUSR | Mode::WUSR).unwrap();

        let error = load_package(&path, &config()).unwrap_err();
        assert_eq!(error.kind, FailureKind::Io);
        assert!(error.message.contains("not a regular file"));

        fs::remove_file(path).unwrap();
    }
}
