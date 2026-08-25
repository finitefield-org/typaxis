use std::fs::File;
use std::io::Read;
use std::path::{Component, Path};

use typaxis_core::{
    EffectiveConfig, GeneratedBufferKey, GenerationKind, HostAdmissionContext, NonNegativeLength,
    PortablePath, ResolvedDataTables, SourceId, TextSpan, Utf8ByteOffset, ValidatedResourceLimits,
};
use typaxis_display_list::ValidatedDisplayDocument;
use typaxis_document::{Block, Inline, ReferenceFormat};
use typaxis_layout::{CanonicalFlowIrBuilder, FlowTree, LayoutEpoch, ShapeFontSelectionReceipt};
use typaxis_linebreak::{
    break_paragraph_validated, BoundedReferenceParagraphFactory, LineLayoutContext, LineShape,
    LineShapeExhaustion, OptimalParagraphBreaker, ParagraphShapedText, ReferenceSpaceGlue,
    ValidatedParagraphBreak, ValidatedParagraphItemRegistry,
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
    SourceFile, ValidatedParsedPackage,
};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParagraphTextSite {
    Parsed(TextSpan),
    Generated(GeneratedBufferKey),
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
            &instances,
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
    instances: &AdmittedFontInstanceTable,
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
            return Err(Failure::input(
                "L5000: unsupported block reached paragraph layout",
            ))
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
            .map_err(|error| Failure::input(format!("L5000: line layout failed: {error:?}")))
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
                Failure::input(format!("L5000: paragraph construction failed: {error:?}"))
            })?;
        return break_canonical(&paragraph).map(Some);
    }
    let mut inputs = Vec::new();
    inputs
        .try_reserve_exact(sites.len())
        .map_err(|_| Failure::limit("paragraph itemization allocation failed"))?;
    for site in sites {
        let text = bind_paragraph_text_site(package, generated, site)?;
        let computed = package.cascade_style(text.site_owner()).map_err(|error| {
            Failure::input(format!(
                "L5000: paragraph style resolution failed: {error:?}"
            ))
        })?;
        let selection =
            ShapeFontSelectionReceipt::new(package, &computed, admitted.token(), instances, epoch)
                .map_err(|error| {
                    Failure::input(format!("L5000: font selection failed: {error:?}"))
                })?;
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
            Failure::input(format!("L5000: paragraph construction failed: {error:?}"))
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
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use typaxis_core::{
        ConfigResourceRoot, EffectiveDataVersions, PdfStreamCompression, ResourceLimits,
    };

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
