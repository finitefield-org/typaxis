#![forbid(unsafe_code)]

mod artifacts;
mod cli;
mod config;
mod font;
#[cfg(test)]
mod machine_tests;
mod pipeline;
mod sidecar;

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use cli::{
    BuildOptions, BuildPackageOptions, CapabilitiesFormat, CapabilitiesOptions,
    CheckPackageOptions, Command, CommonOptions, Invocation, SourceOptions,
};
use pipeline::{Failure, FailureKind};
use typaxis_core::{
    BuildExecutionContext, BuildExecutionError, DiagnosticsExecutionContext, EffectiveConfig,
    HostAdmissionContext, HostPath, JsonPointer, PortablePath, ReplacePolicy, ResolvedDataTables,
    ShaperIdentity,
};
use typaxis_diagnostics::{
    encode_diagnostics_canonical, DiagnosticBuilder, DiagnosticLocation, GlobalDiagnosticScope,
    MachineDiagnosticBudget, MachineDiagnosticBudgetError, MachineDiagnosticLender,
    MachineDiagnosticPhase, PublicMachineError, Severity, L5101, R7100,
};
use typaxis_document_package::{
    DocumentPackageDecodeError, DocumentPackageDecodeErrorClass, DocumentPackageDecodePolicy,
    JsonPreflightErrorClass, StrictDocumentPackageDecoder,
};
use typaxis_machine_input::{
    HostMachineInputSession, MachineInputError, MachineInputErrorKind, MachineInputHostOptions,
};
use typaxis_machine_profile::{
    encode_capabilities_canonical, HostCapabilityDescriptor, HostCapabilityPreflightError,
    MachineProfileDescriptor,
};
use typaxis_manifest::{
    BuildOutputCommitContext, BuildOutputCommitContextError, BuiltPublicationCommitError,
    BuiltPublicationStagingError, FailedManifestPublication, ManifestAdmissionLedger,
    ManifestPublicationContext, ManifestPublicationError, ManifestSinkCommitError,
    PdfSinkCommitError, PendingFailedManifestPublication, PreparedPdfCommitError,
    PreparedStandalonePdfPublication, PublicationReadLedgerToken, StagedBuiltPublication,
    StagingMachineLayoutFacts,
};
use typaxis_resources::AdmittedResourceLedger;
use typaxis_syntax::{
    DocumentPackageParser, MachineParseOutcome, PackageValidationPolicy, ValidatedMachinePackage,
    ValidatedParsedPackage,
};

fn main() {
    let program = std::env::args_os()
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "typaxis".to_owned());
    let invocation = match cli::parse(std::env::args_os().skip(1)) {
        Ok(invocation) => invocation,
        Err(error) => {
            let report = format!("error: {error}\nrun `{program} --help` for usage\n");
            let exit_code = if write_stderr(report.as_bytes()).is_ok() {
                FailureKind::Usage.exit_code()
            } else {
                FailureKind::Io.exit_code()
            };
            std::process::exit(exit_code);
        }
    };
    let result = match invocation {
        Invocation::Help(command) => write_help(&program, command.as_deref()),
        Invocation::Version => {
            write_stdout(format!("typaxis {}\n", env!("CARGO_PKG_VERSION")).as_bytes())
        }
        Invocation::Run(command) => run(*command),
    };
    if let Err(error) = result {
        let report = format!("error: {}\n", error.message);
        let exit_code = if write_stderr(report.as_bytes()).is_ok() {
            error.kind.exit_code()
        } else {
            FailureKind::Io.exit_code()
        };
        std::process::exit(exit_code);
    }
}

fn run(command: Command) -> Result<(), Failure> {
    match command {
        Command::Build(options) => run_build(options),
        Command::BuildPackage(options) => run_build_package(options),
        Command::Capabilities(options) => run_capabilities(options),
        Command::Check(options) => {
            let loaded = load_config(&options.common)?;
            let admission = admission_context(&options, &loaded.effective, loaded.path.as_deref())?;
            pipeline::load_package(admission.entry().as_path(), &loaded.effective)?;
            Ok(())
        }
        Command::CheckPackage(options) => run_check_package(options),
        Command::DumpAst(options) => {
            let loaded = load_config(&options.common)?;
            let admission = admission_context(&options, &loaded.effective, loaded.path.as_deref())?;
            let package = pipeline::load_package(admission.entry().as_path(), &loaded.effective)?;
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            artifacts::write_document_package_json(
                &package,
                loaded.effective.limits(),
                &mut stdout,
            )
            .map_err(map_document_package_artifact_error)?;
            stdout
                .flush()
                .map_err(|error| Failure::io(format!("cannot flush stdout: {error}")))
        }
        Command::DumpLayout {
            source,
            physical_page,
        } => run_dump_layout(&source, physical_page),
        Command::InspectFont { font } => match font::inspect_font(&font) {
            Ok(json) => write_stdout(json.as_bytes()),
            Err(error) => Err(map_font_error(error)),
        },
        Command::ListFonts { font_dir } => match font::list_fonts(&font_dir) {
            Ok(json) => write_stdout(json.as_bytes()),
            Err(error) => Err(map_font_error(error)),
        },
    }
}

fn run_capabilities(options: CapabilitiesOptions) -> Result<(), Failure> {
    match options.format {
        CapabilitiesFormat::Json => {
            let encoded = encode_capabilities_canonical(HostCapabilityDescriptor::compiled());
            write_stdout(encoded.as_bytes())
        }
    }
}

/// The sole machine build owner: preparation, layout/PDF, and terminal publication
/// advance in one direction on one thread.
fn run_build_package(options: BuildPackageOptions) -> Result<(), Failure> {
    run_build_package_with_host(options, MachineHostPreflight::Compiled)
}

fn run_build_package_with_host(
    options: BuildPackageOptions,
    host_preflight: MachineHostPreflight,
) -> Result<(), Failure> {
    reject_machine_package_outside_explicit_root(
        &options.package,
        options.package_root.as_deref(),
    )?;
    let loaded = load_config(&options.common)?;
    let config = loaded.effective;
    let execution = BuildExecutionContext::from_cli_token(
        &options.output,
        optional_host_path(options.trace.clone(), "trace")?,
        optional_host_path(options.manifest.clone(), "manifest")?,
        optional_host_path(options.diagnostics.clone(), "diagnostics")?,
        if options.force {
            ReplacePolicy::Replace
        } else {
            ReplacePolicy::NoReplace
        },
    )
    .map_err(map_execution_setup_error)?;
    reject_known_machine_build_aliases(&options, loaded.path.as_deref(), &execution)?;
    let output = BuildOutputCommitContext::new_machine(
        &config,
        &execution,
        MachineProfileDescriptor::for_id(options.profile),
    )
    .map_err(map_output_context_error)?;
    let publication = if options.manifest.is_some() {
        let versions = config.data_versions();
        let tables =
            ResolvedDataTables::resolve(versions.unicode(), versions.japanese_line_break())
                .ok_or_else(|| Failure::internal("configured data tables are not linked"))?;
        Some(
            ManifestPublicationContext::new(
                &config,
                &output,
                ShaperIdentity::linked_reference(),
                &tables,
            )
            .map_err(map_publication_context_error)?,
        )
    } else {
        None
    };
    let mut manifest = publication
        .as_ref()
        .map(ManifestPublicationContext::begin_admission_ledger);
    let mut diagnostics = MachineDiagnosticBudget::new();
    {
        let _phase = lend_machine_phase(&mut diagnostics, MachineDiagnosticPhase::Config)?;
    }

    let admission = match machine_admission_context(
        &options.package,
        &options.common,
        &config,
        loaded.path.as_deref(),
    ) {
        Ok(admission) => admission,
        Err(primary) => {
            return Err(publish_machine_processing_failure(
                &execution,
                diagnostics,
                output,
                publication,
                manifest,
                None,
                FailedMachineCommand::without_reads(primary),
            ));
        }
    };
    let prepared = match prepare_machine_command(
        &options.package,
        options.package_root.as_deref(),
        options.profile,
        &config,
        &admission,
        &mut diagnostics,
        manifest.as_mut(),
        MachineWriteTargets::Build(&execution),
        host_preflight,
    ) {
        Ok(prepared) => prepared,
        Err(failed) => {
            return Err(publish_machine_processing_failure(
                &execution,
                diagnostics,
                output,
                publication,
                manifest,
                None,
                failed,
            ));
        }
    };

    let PreparedMachineCommand {
        package,
        checked,
        sidecar_read,
        terminal_read,
    } = prepared;
    let (receipt, preparation) = checked.into_parts();
    let layout = {
        let result = {
            let _phase = lend_machine_phase(&mut diagnostics, MachineDiagnosticPhase::Layout)?;
            pipeline::layout_machine_paragraphs(&package, &receipt, preparation, &config)
        };
        match result {
            Ok(layout) => layout,
            Err(primary) => {
                return Err(publish_machine_processing_failure(
                    &execution,
                    diagnostics,
                    output,
                    publication,
                    manifest,
                    None,
                    FailedMachineCommand {
                        primary,
                        sidecar_read: Some(sidecar_read),
                        terminal_read: Some(terminal_read),
                    },
                ));
            }
        }
    };
    if let Some(ledger) = manifest.as_mut() {
        if let Err(error) = ledger.admit_layout_selected(layout.pagination()) {
            return Err(publish_machine_processing_failure(
                &execution,
                diagnostics,
                output,
                publication,
                manifest,
                Some(layout.pagination()),
                FailedMachineCommand {
                    primary: Failure::internal(format!(
                        "machine manifest layout projection failed: {error:?}"
                    )),
                    sidecar_read: Some(sidecar_read),
                    terminal_read: Some(terminal_read),
                },
            ));
        }
    }
    let table_layouts_for_trace = layout.table_manifest_facts()?;
    let footnote_layout_for_trace = layout.footnote_manifest_facts(&package, &config)?;
    let trace_json = match options.trace.as_ref() {
        Some(_) => match artifacts::machine_layout_trace_json(
            layout.flow(),
            layout.initial(),
            layout.pagination(),
            config.limits().get().max_layout_passes,
            options.trace_text,
            artifacts::MachineTraceBinding::new(
                &receipt,
                layout.flow_registry_sha256(),
                &table_layouts_for_trace,
                footnote_layout_for_trace.as_ref(),
            ),
        ) {
            Ok(trace) => Some(trace),
            Err(message) => {
                return Err(publish_machine_processing_failure(
                    &execution,
                    diagnostics,
                    output,
                    publication,
                    manifest,
                    Some(layout.pagination()),
                    FailedMachineCommand {
                        primary: map_trace_artifact_error(message, "machine trace encoding failed"),
                        sidecar_read: Some(sidecar_read),
                        terminal_read: Some(terminal_read),
                    },
                ));
            }
        },
        None => None,
    };
    if let Err(primary) = pipeline::reject_machine_strict_fallback(&layout, &config) {
        return Err(publish_machine_processing_failure(
            &execution,
            diagnostics,
            output,
            publication,
            manifest,
            Some(layout.pagination()),
            FailedMachineCommand {
                primary,
                sidecar_read: Some(sidecar_read),
                terminal_read: Some(terminal_read),
            },
        ));
    }

    let pdf = {
        let result = {
            let _phase = lend_machine_phase(&mut diagnostics, MachineDiagnosticPhase::Pdf)?;
            pipeline::build_machine_pdf_graph(&package, &receipt, &config, &layout).and_then(
                |graph| {
                    let pdf = typaxis_pdf::PdfBackend::serialize(graph.clone(), &config)
                        .map_err(map_machine_pdf_error)?;
                    pipeline::validate_machine_table_pdf_closure(&layout, &graph, &pdf)?;
                    pipeline::validate_machine_footnote_pdf_closure(&layout, &graph, &pdf)?;
                    Ok(pdf)
                },
            )
        };
        match result {
            Ok(pdf) => pdf,
            Err(primary) => {
                return Err(publish_machine_processing_failure(
                    &execution,
                    diagnostics,
                    output,
                    publication,
                    manifest,
                    Some(layout.pagination()),
                    FailedMachineCommand {
                        primary,
                        sidecar_read: Some(sidecar_read),
                        terminal_read: Some(terminal_read),
                    },
                ));
            }
        }
    };
    {
        let _phase = lend_machine_phase(&mut diagnostics, MachineDiagnosticPhase::Publication)?;
    }
    publish_machine_success(
        &execution,
        diagnostics,
        output,
        publication,
        &package,
        &receipt,
        &layout,
        &config,
        pdf,
        trace_json.as_deref(),
        sidecar_read,
        terminal_read,
    )
}

/// Success ends at complete resource metadata and style/font-family preparation; it does not
/// claim glyph coverage, pagination, or PDF serialization.
fn run_check_package(options: CheckPackageOptions) -> Result<(), Failure> {
    reject_machine_package_outside_explicit_root(
        &options.package,
        options.package_root.as_deref(),
    )?;
    let loaded = load_config(&options.common)?;
    let diagnostics_execution = options
        .diagnostics
        .clone()
        .map(|target| {
            HostPath::new(target)
                .map_err(|_| Failure::usage("diagnostics path must not be empty"))
                .and_then(|target| {
                    DiagnosticsExecutionContext::new(target, ReplacePolicy::NoReplace)
                        .map_err(map_execution_setup_error)
                })
        })
        .transpose()?;
    reject_known_machine_check_aliases(
        &options,
        loaded.path.as_deref(),
        diagnostics_execution.as_ref(),
    )?;
    let admission = machine_admission_context(
        &options.package,
        &options.common,
        &loaded.effective,
        loaded.path.as_deref(),
    )?;
    let mut diagnostics = MachineDiagnosticBudget::new();
    {
        let _phase = lend_machine_phase(&mut diagnostics, MachineDiagnosticPhase::Config)?;
    }
    let writes = MachineWriteTargets::Diagnostics(diagnostics_execution.as_ref());
    match prepare_machine_command(
        &options.package,
        options.package_root.as_deref(),
        options.profile,
        &loaded.effective,
        &admission,
        &mut diagnostics,
        None,
        writes,
        MachineHostPreflight::Compiled,
    ) {
        Ok(prepared) => {
            let encoded = encode_diagnostics_canonical(diagnostics.finish().diagnostics());
            if let Some(execution) = diagnostics_execution.as_ref() {
                publish_check_diagnostics(execution, &encoded, Some(&prepared.sidecar_read))?;
            }
            // Keep both trusted values alive until diagnostics publication has
            // revalidated the final read ledger. No layout consumer is called.
            let _ = (prepared.package, prepared.checked);
            Ok(())
        }
        Err(failed) => {
            let encoded = encode_diagnostics_canonical(diagnostics.finish().diagnostics());
            if let Some(execution) = diagnostics_execution.as_ref() {
                if let Err(publication) =
                    publish_check_diagnostics(execution, &encoded, failed.sidecar_read.as_ref())
                {
                    return Err(Failure::io(format!(
                        "{}; diagnostics publication also failed: {}",
                        failed.primary.message, publication.message
                    )));
                }
            }
            Err(failed.primary)
        }
    }
}

fn reject_machine_package_outside_explicit_root(
    package: &Path,
    explicit_root: Option<&Path>,
) -> Result<(), Failure> {
    let Some(explicit_root) = explicit_root else {
        return Ok(());
    };
    if explicit_root.as_os_str().is_empty() {
        return Err(Failure::usage("package-root path must not be empty"));
    }
    let current = std::env::current_dir()
        .map_err(|error| Failure::io(format!("cannot determine current directory: {error}")))?;
    let package = lexical_absolute_path(package, &current);
    let root = lexical_absolute_path(explicit_root, &current);
    if package.strip_prefix(&root).is_err() {
        return Err(Failure::usage(
            "PACKAGE is outside the explicit package root",
        ));
    }
    Ok(())
}

fn lexical_absolute_path(path: &Path, current: &Path) -> PathBuf {
    use std::path::Component;

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        current.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

struct PreparedMachineCommand {
    package: Box<ValidatedMachinePackage>,
    checked: pipeline::CheckedMachinePackage,
    sidecar_read: PublicationReadLedgerToken,
    terminal_read: PublicationReadLedgerToken,
}

struct FailedMachineCommand {
    primary: Failure,
    sidecar_read: Option<PublicationReadLedgerToken>,
    terminal_read: Option<PublicationReadLedgerToken>,
}

impl FailedMachineCommand {
    fn without_reads(primary: Failure) -> Self {
        Self {
            primary,
            sidecar_read: None,
            terminal_read: None,
        }
    }

    fn with_reads(primary: Failure, reads: MachineReadTokens) -> Self {
        Self {
            primary,
            sidecar_read: Some(reads.sidecar),
            terminal_read: Some(reads.terminal),
        }
    }
}

struct MachineReadTokens {
    sidecar: PublicationReadLedgerToken,
    terminal: PublicationReadLedgerToken,
}

#[derive(Clone, Copy)]
enum MachineWriteTargets<'a> {
    Build(&'a BuildExecutionContext),
    Diagnostics(Option<&'a DiagnosticsExecutionContext>),
}

#[derive(Clone, Copy)]
enum MachineHostPreflight {
    Compiled,
    #[cfg(test)]
    Unavailable,
}

impl MachineHostPreflight {
    fn run(
        self,
        profile: typaxis_core::MachinePdfProfileId,
        diagnostics: &mut MachineDiagnosticLender<'_>,
    ) -> Result<(), HostCapabilityPreflightError> {
        match self {
            Self::Compiled => HostCapabilityDescriptor::compiled()
                .preflight(profile, diagnostics)
                .map(|_| ()),
            #[cfg(test)]
            Self::Unavailable => {
                let unavailable = HostCapabilityPreflightError::Unavailable;
                let error = PublicMachineError::CompiledHostUnavailable;
                let diagnostic = DiagnosticBuilder::global(
                    error.code(),
                    Severity::Fatal,
                    unavailable.to_string(),
                    GlobalDiagnosticScope::Io,
                )
                .unwrap()
                .build();
                let _ = diagnostics
                    .emit(diagnostic)
                    .map_err(HostCapabilityPreflightError::DiagnosticBudget)?;
                Err(unavailable)
            }
        }
    }
}

impl MachineWriteTargets<'_> {
    fn validate(self, read: &PublicationReadLedgerToken) -> Result<(), Failure> {
        read.revalidate()
            .map_err(|_| Failure::io("I9113: an admitted input changed during validation"))?;
        match self {
            Self::Build(execution) => {
                for target in [
                    execution.output_path(),
                    execution.trace_target(),
                    execution.manifest_target(),
                    execution.diagnostics_target(),
                ]
                .into_iter()
                .flatten()
                {
                    if read.revalidate_write_target(target).map_err(|_| {
                        Failure::io("I9113: an admitted input changed during alias validation")
                    })? {
                        return Err(Failure::io(
                            "I9113: a machine write target aliases an admitted input candidate",
                        ));
                    }
                }
            }
            Self::Diagnostics(Some(execution)) => {
                if read
                    .revalidate_write_target(execution.diagnostics_target())
                    .map_err(|_| {
                        Failure::io("I9113: an admitted input changed during alias validation")
                    })?
                {
                    return Err(Failure::io(
                        "I9113: diagnostics target aliases an admitted input candidate",
                    ));
                }
            }
            Self::Diagnostics(None) => {}
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare_machine_command(
    package_path: &Path,
    package_root: Option<&Path>,
    profile: typaxis_core::MachinePdfProfileId,
    config: &EffectiveConfig,
    admission: &HostAdmissionContext,
    diagnostics: &mut MachineDiagnosticBudget,
    mut manifest: Option<&mut ManifestAdmissionLedger>,
    writes: MachineWriteTargets<'_>,
    host_preflight: MachineHostPreflight,
) -> Result<PreparedMachineCommand, FailedMachineCommand> {
    {
        let mut host = lend_machine_phase(diagnostics, MachineDiagnosticPhase::Host)
            .map_err(FailedMachineCommand::without_reads)?;
        host_preflight
            .run(profile, &mut host)
            .map_err(map_host_capability_preflight)
            .map_err(FailedMachineCommand::without_reads)?;
    }

    let package_host = HostPath::new(package_path.to_path_buf())
        .map_err(|_| Failure::usage("PACKAGE path must not be empty"))
        .map_err(FailedMachineCommand::without_reads)?;
    let root_host = package_root
        .map(Path::to_path_buf)
        .map(HostPath::new)
        .transpose()
        .map_err(|_| Failure::usage("package-root path must not be empty"))
        .map_err(FailedMachineCommand::without_reads)?;
    let host_options = MachineInputHostOptions::new(package_host, root_host);
    let (session, raw) = {
        let mut phase = lend_machine_phase(diagnostics, MachineDiagnosticPhase::Package)
            .map_err(FailedMachineCommand::without_reads)?;
        match HostMachineInputSession::open(host_options, config.limits()) {
            Ok(value) => value,
            Err(error) => {
                project_machine_progress(&mut manifest, error.progress())
                    .map_err(FailedMachineCommand::without_reads)?;
                emit_machine_input_diagnostic(&mut phase, &error, package_path)
                    .map_err(FailedMachineCommand::without_reads)?;
                let primary = map_machine_input_error(&error);
                let reads = read_tokens_from_machine_error(&error)
                    .map_err(FailedMachineCommand::without_reads)?;
                return Err(FailedMachineCommand::with_reads(primary, reads));
            }
        }
    };
    let raw_progress = session.progress();
    project_machine_progress(&mut manifest, &raw_progress)
        .map_err(FailedMachineCommand::without_reads)?;
    validate_session_reads(&session, writes)
        .map_err(|primary| session_failure_with_reads(&session, primary))?;

    let decoder = StrictDocumentPackageDecoder::new();
    let policy = DocumentPackageDecodePolicy::new(config.limits());
    let decoded = {
        let mut phase = lend_machine_phase(diagnostics, MachineDiagnosticPhase::Decode)
            .map_err(FailedMachineCommand::without_reads)?;
        match session.decode_and_bind(&raw, &decoder, &policy) {
            Ok(decoded) => decoded,
            Err(error) => {
                project_machine_progress(&mut manifest, error.progress())
                    .map_err(FailedMachineCommand::without_reads)?;
                emit_machine_input_diagnostic(&mut phase, &error, package_path)
                    .map_err(FailedMachineCommand::without_reads)?;
                let primary = map_machine_input_error(&error);
                let reads = read_tokens_from_machine_error(&error)
                    .map_err(FailedMachineCommand::without_reads)?;
                return Err(FailedMachineCommand::with_reads(primary, reads));
            }
        }
    };
    let decoded_progress = session.progress();
    project_machine_progress(&mut manifest, &decoded_progress)
        .map_err(FailedMachineCommand::without_reads)?;

    let sources = {
        let mut phase = lend_machine_phase(diagnostics, MachineDiagnosticPhase::Source)
            .map_err(FailedMachineCommand::without_reads)?;
        match session.admit_sources(&decoded, config.limits()) {
            Ok(sources) => sources,
            Err(error) => {
                project_machine_progress(&mut manifest, error.progress())
                    .map_err(FailedMachineCommand::without_reads)?;
                emit_machine_input_diagnostic(&mut phase, &error, package_path)
                    .map_err(FailedMachineCommand::without_reads)?;
                let primary = map_machine_input_error(&error);
                let reads = read_tokens_from_machine_error(&error)
                    .map_err(FailedMachineCommand::without_reads)?;
                return Err(FailedMachineCommand::with_reads(primary, reads));
            }
        }
    };
    let source_progress = session.progress();
    project_machine_progress(&mut manifest, &source_progress)
        .map_err(FailedMachineCommand::without_reads)?;
    validate_session_reads(&session, writes)
        .map_err(|primary| session_failure_with_reads(&session, primary))?;

    let admitted = match session.finish(raw, decoded, sources) {
        Ok(admitted) => admitted,
        Err(error) => {
            project_machine_progress(&mut manifest, error.progress())
                .map_err(FailedMachineCommand::without_reads)?;
            let primary = map_machine_input_error(&error);
            let reads = read_tokens_from_machine_error(&error)
                .map_err(FailedMachineCommand::without_reads)?;
            return Err(FailedMachineCommand::with_reads(primary, reads));
        }
    };
    let syntax_failure_reads =
        read_tokens_from_admitted(&admitted).map_err(FailedMachineCommand::without_reads)?;
    let syntax_policy = PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes())
        .map_err(|error| Failure::internal(format!("machine syntax policy failed: {error:?}")))
        .map_err(FailedMachineCommand::without_reads)?;
    let package = {
        let mut phase = lend_machine_phase(diagnostics, MachineDiagnosticPhase::Syntax)
            .map_err(FailedMachineCommand::without_reads)?;
        match DocumentPackageParser::new().parse(admitted, &syntax_policy) {
            MachineParseOutcome::Parsed { package } => package,
            MachineParseOutcome::Failed { progress, failure } => {
                project_machine_progress(&mut manifest, &progress)
                    .map_err(FailedMachineCommand::without_reads)?;
                let uri = progress
                    .package()
                    .map(|facts| facts.uri().clone())
                    .unwrap_or_else(|| fallback_package_uri(package_path));
                let diagnostic = failure.to_diagnostic(&uri);
                let code = *diagnostic.code();
                let _ = phase
                    .emit(diagnostic)
                    .map_err(map_diagnostic_budget_error)
                    .map_err(FailedMachineCommand::without_reads)?;
                let primary = Failure::input(format!("{}: {failure}", code.as_str()));
                return Err(FailedMachineCommand::with_reads(
                    primary,
                    syntax_failure_reads,
                ));
            }
        }
    };
    project_validated_machine_package(&mut manifest, &package)
        .map_err(FailedMachineCommand::without_reads)?;

    let candidates =
        match pipeline::register_machine_resource_candidates(&package, config, admission) {
            Ok(candidates) => candidates,
            Err(primary) => {
                let reads = read_tokens_from_package(&package)
                    .map_err(FailedMachineCommand::without_reads)?;
                return Err(FailedMachineCommand::with_reads(primary, reads));
            }
        };
    let candidate_validation = package
        .provenance()
        .admission()
        .read_ledger_token()
        .map_err(|_| Failure::internal("cannot seal resource candidate read ledger"))
        .map_err(FailedMachineCommand::without_reads)?;
    if let Err(primary) = writes.validate(&candidate_validation) {
        let reads =
            read_tokens_from_package(&package).map_err(FailedMachineCommand::without_reads)?;
        return Err(FailedMachineCommand::with_reads(primary, reads));
    }
    let capability = {
        let mut capability_phase =
            lend_machine_phase(diagnostics, MachineDiagnosticPhase::Capability)
                .map_err(FailedMachineCommand::without_reads)?;
        match pipeline::preflight_machine_package(
            &package,
            profile,
            config.limits(),
            &mut capability_phase,
            candidates,
        ) {
            Ok(capability) => capability,
            Err(primary) => {
                let reads = read_tokens_from_package(&package)
                    .map_err(FailedMachineCommand::without_reads)?;
                return Err(FailedMachineCommand::with_reads(primary, reads));
            }
        }
    };
    project_machine_capability(&mut manifest, &package, capability.receipt())
        .map_err(FailedMachineCommand::without_reads)?;

    // Resource admission is single-threaded and deterministic. Style/font
    // coverage follows it in the same owner but receives its own diagnostic
    // phase on both success and complete-resource failure.
    let checked = match pipeline::complete_machine_package_preparation(
        &package, capability, config, admission,
    ) {
        Ok(checked) => {
            project_complete_resources(&mut manifest, checked.preparation().admitted())
                .map_err(FailedMachineCommand::without_reads)?;
            {
                let _phase = lend_machine_phase(diagnostics, MachineDiagnosticPhase::Resource)
                    .map_err(FailedMachineCommand::without_reads)?;
            }
            {
                let _phase = lend_machine_phase(diagnostics, MachineDiagnosticPhase::Style)
                    .map_err(FailedMachineCommand::without_reads)?;
            }
            checked
        }
        Err(preparation) => {
            project_resource_failure(&mut manifest, preparation.resource_progress())
                .map_err(FailedMachineCommand::without_reads)?;
            let complete = matches!(
                preparation.resource_progress(),
                Some(pipeline::MachineResourcePreparationProgress::Complete(_))
            );
            let phase_kind = if complete {
                MachineDiagnosticPhase::Style
            } else {
                MachineDiagnosticPhase::Resource
            };
            let mut phase = lend_machine_phase(diagnostics, phase_kind)
                .map_err(FailedMachineCommand::without_reads)?;
            emit_preparation_diagnostic(&mut phase, &preparation, &package, complete)
                .map_err(FailedMachineCommand::without_reads)?;
            let reads =
                read_tokens_from_package(&package).map_err(FailedMachineCommand::without_reads)?;
            return Err(FailedMachineCommand::with_reads(
                preparation.into_failure(),
                reads,
            ));
        }
    };
    let final_read = package
        .provenance()
        .admission()
        .read_ledger_token()
        .map_err(|_| Failure::internal("cannot seal final machine read ledger"))
        .map_err(FailedMachineCommand::without_reads)?;
    if let Err(primary) = writes.validate(&final_read) {
        let reads =
            read_tokens_from_package(&package).map_err(FailedMachineCommand::without_reads)?;
        return Err(FailedMachineCommand::with_reads(primary, reads));
    }
    let sidecar_read = package
        .provenance()
        .admission()
        .read_ledger_token()
        .map_err(|_| Failure::internal("cannot seal final sidecar read ledger"))
        .map_err(FailedMachineCommand::without_reads)?;
    Ok(PreparedMachineCommand {
        package,
        checked,
        sidecar_read,
        terminal_read: final_read,
    })
}

fn lend_machine_phase(
    diagnostics: &mut MachineDiagnosticBudget,
    phase: MachineDiagnosticPhase,
) -> Result<MachineDiagnosticLender<'_>, Failure> {
    diagnostics.lend(phase).map_err(map_diagnostic_budget_error)
}

fn map_diagnostic_budget_error(error: MachineDiagnosticBudgetError) -> Failure {
    Failure::internal(format!(
        "machine diagnostic phase orchestration failed: {error:?}"
    ))
}

fn map_host_capability_preflight(error: HostCapabilityPreflightError) -> Failure {
    match error {
        HostCapabilityPreflightError::Unavailable => {
            Failure::io("I9110: required compiled host capability is unavailable")
        }
        HostCapabilityPreflightError::WrongDiagnosticPhase
        | HostCapabilityPreflightError::DiagnosticBudget(_) => Failure::internal(format!(
            "I9190: host capability preflight orchestration failed: {error:?}"
        )),
    }
}

fn project_machine_progress(
    manifest: &mut Option<&mut ManifestAdmissionLedger>,
    progress: &typaxis_machine_input::MachineInputProgress,
) -> Result<(), Failure> {
    if let Some(ledger) = manifest.as_deref_mut() {
        ledger
            .admit_machine_input_progress(progress)
            .map_err(|error| {
                Failure::internal(format!(
                    "machine manifest progress projection failed: {error:?}"
                ))
            })?;
    }
    Ok(())
}

fn project_validated_machine_package(
    manifest: &mut Option<&mut ManifestAdmissionLedger>,
    package: &ValidatedMachinePackage,
) -> Result<(), Failure> {
    if let Some(ledger) = manifest.as_deref_mut() {
        ledger
            .admit_validated_machine_package(package)
            .map_err(|error| {
                Failure::internal(format!(
                    "machine manifest package projection failed: {error:?}"
                ))
            })?;
    }
    Ok(())
}

fn project_machine_capability(
    manifest: &mut Option<&mut ManifestAdmissionLedger>,
    package: &ValidatedMachinePackage,
    receipt: &typaxis_machine_profile::MachinePdfPreflightReceipt,
) -> Result<(), Failure> {
    if let Some(ledger) = manifest.as_deref_mut() {
        ledger
            .admit_machine_capability(package, receipt)
            .map_err(|error| {
                Failure::internal(format!(
                    "machine manifest capability projection failed: {error:?}"
                ))
            })?;
    }
    Ok(())
}

fn project_complete_resources(
    manifest: &mut Option<&mut ManifestAdmissionLedger>,
    admitted: &AdmittedResourceLedger,
) -> Result<(), Failure> {
    if let Some(ledger) = manifest.as_deref_mut() {
        ledger.admit_resources(admitted.token()).map_err(|error| {
            Failure::internal(format!(
                "machine manifest resource projection failed: {error:?}"
            ))
        })?;
    }
    Ok(())
}

fn project_resource_failure(
    manifest: &mut Option<&mut ManifestAdmissionLedger>,
    progress: Option<&pipeline::MachineResourcePreparationProgress>,
) -> Result<(), Failure> {
    let Some(ledger) = manifest.as_deref_mut() else {
        return Ok(());
    };
    match progress {
        Some(pipeline::MachineResourcePreparationProgress::Partial(progress)) => ledger
            .admit_resource_progress(progress.clone())
            .map_err(|error| {
                Failure::internal(format!(
                    "machine manifest partial-resource projection failed: {error:?}"
                ))
            }),
        Some(pipeline::MachineResourcePreparationProgress::Complete(admitted)) => {
            ledger.admit_resources(admitted.token()).map_err(|error| {
                Failure::internal(format!(
                    "machine manifest complete-resource projection failed: {error:?}"
                ))
            })
        }
        None => Ok(()),
    }
}

fn validate_session_reads(
    session: &HostMachineInputSession,
    writes: MachineWriteTargets<'_>,
) -> Result<(), Failure> {
    let read = session
        .read_ledger_token()
        .map_err(|_| Failure::internal("cannot seal machine-input read ledger"))?;
    writes.validate(&read)
}

fn session_failure_with_reads(
    session: &HostMachineInputSession,
    primary: Failure,
) -> FailedMachineCommand {
    let Ok(sidecar) = session.read_ledger_token() else {
        return FailedMachineCommand::without_reads(primary);
    };
    let Ok(terminal) = session.read_ledger_token() else {
        return FailedMachineCommand::without_reads(primary);
    };
    FailedMachineCommand::with_reads(primary, MachineReadTokens { sidecar, terminal })
}

fn read_tokens_from_machine_error(error: &MachineInputError) -> Result<MachineReadTokens, Failure> {
    let sidecar = error
        .read_ledger_token()
        .map_err(|_| Failure::internal("cannot seal failed machine-input read ledger"))?;
    let terminal = error
        .read_ledger_token()
        .map_err(|_| Failure::internal("cannot seal failed machine-input terminal ledger"))?;
    Ok(MachineReadTokens { sidecar, terminal })
}

fn read_tokens_from_admitted(
    admitted: &typaxis_machine_input::AdmittedMachinePackage,
) -> Result<MachineReadTokens, Failure> {
    let sidecar = admitted
        .read_ledger_token()
        .map_err(|_| Failure::internal("cannot seal admitted machine-input read ledger"))?;
    let terminal = admitted
        .read_ledger_token()
        .map_err(|_| Failure::internal("cannot seal admitted machine-input terminal ledger"))?;
    Ok(MachineReadTokens { sidecar, terminal })
}

fn read_tokens_from_package(
    package: &ValidatedMachinePackage,
) -> Result<MachineReadTokens, Failure> {
    let sidecar = package
        .provenance()
        .admission()
        .read_ledger_token()
        .map_err(|_| Failure::internal("cannot seal validated machine read ledger"))?;
    let terminal = package
        .provenance()
        .admission()
        .read_ledger_token()
        .map_err(|_| Failure::internal("cannot seal validated machine terminal ledger"))?;
    Ok(MachineReadTokens { sidecar, terminal })
}

fn emit_machine_input_diagnostic(
    phase: &mut MachineDiagnosticLender<'_>,
    error: &MachineInputError,
    package_path: &Path,
) -> Result<(), Failure> {
    let public = public_machine_input_error(error.kind());
    let message = canonical_machine_input_diagnostic_message(&public);
    let builder = if matches!(
        public,
        PublicMachineError::CompiledHostUnavailable
            | PublicMachineError::PackageOpen
            | PublicMachineError::CompanionSourceOpen
            | PublicMachineError::StableReadMutation
    ) {
        DiagnosticBuilder::global(
            public.code(),
            Severity::Error,
            message,
            GlobalDiagnosticScope::Io,
        )
    } else {
        DiagnosticBuilder::located(
            public.code(),
            Severity::Error,
            message,
            machine_input_diagnostic_location(error, package_path),
        )
    }
    .map_err(|_| Failure::internal("machine input diagnostic text was not canonical"))?;
    let _ = phase
        .emit(builder.build())
        .map_err(map_diagnostic_budget_error)?;
    Ok(())
}

fn canonical_machine_input_diagnostic_message(error: &PublicMachineError) -> &'static str {
    match error {
        PublicMachineError::PackageEnvelope => "DocumentPackage envelope is invalid",
        PublicMachineError::PackageJsonGrammar => "DocumentPackage JSON grammar is invalid",
        PublicMachineError::PackageMember => "DocumentPackage member is invalid",
        PublicMachineError::PackageContract => "DocumentPackage contract is unsupported",
        PublicMachineError::SourceProfile => "DocumentPackage source profile is unsupported",
        PublicMachineError::SourcePath => "DocumentPackage source path is unsafe",
        PublicMachineError::SourceIdentity => "DocumentPackage source identity does not match",
        PublicMachineError::PackageByteLimit => "DocumentPackage byte limit was exceeded",
        PublicMachineError::JsonNestingDepthLimit => {
            "DocumentPackage JSON nesting depth limit was exceeded"
        }
        PublicMachineError::HostReadCandidateLimit => "host read candidate limit was exceeded",
        PublicMachineError::CompiledHostUnavailable => {
            "required compiled host capability is unavailable"
        }
        PublicMachineError::PackageOpen => "PACKAGE could not be opened safely",
        PublicMachineError::CompanionSourceOpen => "companion source could not be opened safely",
        PublicMachineError::StableReadMutation => "an admitted input changed during validation",
        PublicMachineError::CapabilityDomainMismatch => {
            "machine capability receipt binding does not match"
        }
        PublicMachineError::UnsupportedContent(_)
        | PublicMachineError::UnsupportedStyle(_)
        | PublicMachineError::UnsupportedMaster(_)
        | PublicMachineError::UnsupportedResource(_) => {
            "machine PDF profile rejected an unsupported feature"
        }
    }
}

fn public_machine_input_error(kind: &MachineInputErrorKind) -> PublicMachineError {
    match kind {
        MachineInputErrorKind::UnsupportedContainedOpen => {
            PublicMachineError::CompiledHostUnavailable
        }
        MachineInputErrorKind::CurrentDirectoryUnavailable
        | MachineInputErrorKind::PackageOpen(_) => PublicMachineError::PackageOpen,
        MachineInputErrorKind::InvalidPackagePath
        | MachineInputErrorKind::NonPortablePackageUri
        | MachineInputErrorKind::InvalidPackageUri(_)
        | MachineInputErrorKind::PackageOutsideRoot
        | MachineInputErrorKind::UnsafeSourceUri { .. }
        | MachineInputErrorKind::SourceUriTooLong { .. } => PublicMachineError::SourcePath,
        MachineInputErrorKind::PackageTooLarge { .. } => PublicMachineError::PackageByteLimit,
        MachineInputErrorKind::Decode(error) => public_decode_error(error),
        MachineInputErrorKind::SourceCount { .. }
        | MachineInputErrorKind::NonzeroSourceId { .. } => PublicMachineError::SourceProfile,
        MachineInputErrorKind::SourceOpen { .. } => PublicMachineError::CompanionSourceOpen,
        MachineInputErrorKind::SourceDeclaredLimit { .. }
        | MachineInputErrorKind::SourceLimit { .. }
        | MachineInputErrorKind::AggregateInputLimit { .. }
        | MachineInputErrorKind::SourceLengthMismatch { .. }
        | MachineInputErrorKind::SourceHashMismatch { .. }
        | MachineInputErrorKind::SourceNotUtf8 { .. } => PublicMachineError::SourceIdentity,
        MachineInputErrorKind::DecodePolicyMismatch
        | MachineInputErrorKind::PackageHashMismatch
        | MachineInputErrorKind::InvalidProgress { .. }
        | MachineInputErrorKind::ReceiptSessionMismatch(_)
        | MachineInputErrorKind::ReceiptPackageMismatch(_)
        | MachineInputErrorKind::ReceiptDeclarationMismatch => {
            PublicMachineError::CapabilityDomainMismatch
        }
    }
}

fn public_decode_error(error: &DocumentPackageDecodeError) -> PublicMachineError {
    if let Some(preflight) = error.preflight_error() {
        return match preflight.class() {
            JsonPreflightErrorClass::PackageByteLimit => PublicMachineError::PackageByteLimit,
            JsonPreflightErrorClass::JsonNestingDepthLimit => {
                PublicMachineError::JsonNestingDepthLimit
            }
            JsonPreflightErrorClass::PackageEnvelope => PublicMachineError::PackageEnvelope,
            JsonPreflightErrorClass::JsonSyntax => PublicMachineError::PackageJsonGrammar,
        };
    }
    match error
        .typed_error()
        .map(|typed| typed.class())
        .unwrap_or(DocumentPackageDecodeErrorClass::InternalInvariant)
    {
        DocumentPackageDecodeErrorClass::Contract => PublicMachineError::PackageContract,
        DocumentPackageDecodeErrorClass::Shape | DocumentPackageDecodeErrorClass::Limit => {
            PublicMachineError::PackageMember
        }
        DocumentPackageDecodeErrorClass::CanonicalEncoding
        | DocumentPackageDecodeErrorClass::InternalInvariant => {
            PublicMachineError::CapabilityDomainMismatch
        }
    }
}

fn machine_input_diagnostic_location(
    error: &MachineInputError,
    package_path: &Path,
) -> DiagnosticLocation {
    let uri = error
        .progress()
        .package()
        .map(|facts| facts.uri().clone())
        .unwrap_or_else(|| fallback_package_uri(package_path));
    if let MachineInputErrorKind::Decode(decode) = error.kind() {
        if let Some(preflight) = decode.preflight_error() {
            return DiagnosticLocation::package_json(
                uri,
                preflight.location().json_pointer().clone(),
                Some(preflight.location().byte_offset()),
            );
        }
        if let Some(typed) = decode.typed_error() {
            return DiagnosticLocation::package_json(
                uri,
                typed.location().json_pointer().clone(),
                Some(typed.location().byte_offset()),
            );
        }
    }
    let pointer = match error.kind() {
        MachineInputErrorKind::SourceCount { .. }
        | MachineInputErrorKind::NonzeroSourceId { .. }
        | MachineInputErrorKind::UnsafeSourceUri { .. }
        | MachineInputErrorKind::SourceUriTooLong { .. }
        | MachineInputErrorKind::SourceDeclaredLimit { .. }
        | MachineInputErrorKind::SourceLimit { .. }
        | MachineInputErrorKind::AggregateInputLimit { .. }
        | MachineInputErrorKind::SourceLengthMismatch { .. }
        | MachineInputErrorKind::SourceHashMismatch { .. }
        | MachineInputErrorKind::SourceNotUtf8 { .. } => {
            JsonPointer::from_segments(["sources", "0"])
        }
        _ => JsonPointer::root(),
    };
    DiagnosticLocation::package_json(uri, pointer, None)
}

fn fallback_package_uri(path: &Path) -> PortablePath {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| PortablePath::new(name).ok())
        .unwrap_or_else(|| {
            PortablePath::new("document-package.json").expect("static portable path is valid")
        })
}

fn map_machine_input_error(error: &MachineInputError) -> Failure {
    let public = public_machine_input_error(error.kind());
    let message = format!("{}: {error}", public.code().as_str());
    match error.kind() {
        MachineInputErrorKind::InvalidPackagePath
        | MachineInputErrorKind::NonPortablePackageUri
        | MachineInputErrorKind::InvalidPackageUri(_)
        | MachineInputErrorKind::PackageOutsideRoot => Failure::usage(message),
        MachineInputErrorKind::PackageTooLarge { .. }
        | MachineInputErrorKind::SourceDeclaredLimit { .. }
        | MachineInputErrorKind::SourceLimit { .. }
        | MachineInputErrorKind::AggregateInputLimit { .. } => Failure::limit(message),
        MachineInputErrorKind::Decode(error)
            if matches!(
                error.preflight_error().map(|preflight| preflight.class()),
                Some(
                    JsonPreflightErrorClass::PackageByteLimit
                        | JsonPreflightErrorClass::JsonNestingDepthLimit
                )
            ) || matches!(
                error.typed_error().map(|typed| typed.class()),
                Some(DocumentPackageDecodeErrorClass::Limit)
            ) =>
        {
            Failure::limit(message)
        }
        MachineInputErrorKind::UnsupportedContainedOpen
        | MachineInputErrorKind::CurrentDirectoryUnavailable
        | MachineInputErrorKind::PackageOpen(_)
        | MachineInputErrorKind::SourceOpen { .. } => Failure::io(message),
        MachineInputErrorKind::DecodePolicyMismatch
        | MachineInputErrorKind::PackageHashMismatch
        | MachineInputErrorKind::InvalidProgress { .. }
        | MachineInputErrorKind::ReceiptSessionMismatch(_)
        | MachineInputErrorKind::ReceiptPackageMismatch(_)
        | MachineInputErrorKind::ReceiptDeclarationMismatch => Failure::internal(message),
        MachineInputErrorKind::Decode(error)
            if matches!(
                error.typed_error().map(|typed| typed.class()),
                Some(
                    DocumentPackageDecodeErrorClass::CanonicalEncoding
                        | DocumentPackageDecodeErrorClass::InternalInvariant
                )
            ) =>
        {
            Failure::internal(message)
        }
        _ => Failure::input(message),
    }
}

fn emit_preparation_diagnostic(
    phase: &mut MachineDiagnosticLender<'_>,
    _failure: &pipeline::MachinePreparationFailure,
    package: &ValidatedMachinePackage,
    style_phase: bool,
) -> Result<(), Failure> {
    let code = if style_phase { L5101 } else { R7100 };
    let message = if style_phase {
        "machine style or font-family preparation failed"
    } else {
        "machine resource admission failed"
    };
    let uri = package
        .provenance()
        .progress()
        .package()
        .map(|facts| facts.uri().clone())
        .unwrap_or_else(|| {
            PortablePath::new("document-package.json").expect("static portable path is valid")
        });
    let diagnostic = DiagnosticBuilder::located(
        code,
        Severity::Error,
        message,
        DiagnosticLocation::package_json(uri, JsonPointer::root(), None),
    )
    .map_err(|_| Failure::internal("machine preparation diagnostic text was not canonical"))?
    .build();
    let _ = phase
        .emit(diagnostic)
        .map_err(map_diagnostic_budget_error)?;
    Ok(())
}

fn publish_check_diagnostics(
    execution: &DiagnosticsExecutionContext,
    encoded: &str,
    read: Option<&PublicationReadLedgerToken>,
) -> Result<(), Failure> {
    let prepared = sidecar::prepare_diagnostics(execution, encoded.as_bytes(), read)
        .map_err(map_diagnostics_publication_error)?;
    sidecar::publish_diagnostics(execution, prepared, read)
        .map(|_| ())
        .map_err(map_diagnostics_publication_error)
}

fn optional_host_path(path: Option<PathBuf>, label: &str) -> Result<Option<HostPath>, Failure> {
    path.map(HostPath::new)
        .transpose()
        .map_err(|_| Failure::usage(format!("{label} path must not be empty")))
}

fn map_machine_pdf_error(error: typaxis_pdf::PdfError) -> Failure {
    match error {
        typaxis_pdf::PdfError::ObjectLimit | typaxis_pdf::PdfError::OutputTooLarge => {
            Failure::limit(format!("machine PDF resource limit exceeded: {error:?}"))
        }
        _ => Failure::internal(format!("machine PDF serialization failed: {error:?}")),
    }
}

enum PreparedMachineTerminal {
    Manifest(Box<StagedBuiltPublication>),
    PdfOnly(Box<PreparedStandalonePdfPublication>),
}

impl PreparedMachineTerminal {
    fn fail_before_pdf(self) -> Option<PendingFailedManifestPublication> {
        match self {
            Self::Manifest(staged) => Some(staged.fail_before_pdf()),
            Self::PdfOnly(_) => None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn publish_machine_success(
    execution: &BuildExecutionContext,
    diagnostics: MachineDiagnosticBudget,
    output: BuildOutputCommitContext,
    publication: Option<ManifestPublicationContext>,
    package: &ValidatedMachinePackage,
    receipt: &typaxis_machine_profile::MachinePdfPreflightReceipt,
    layout: &pipeline::MachineParagraphLayout,
    config: &EffectiveConfig,
    pdf: typaxis_pdf::VerifiedPdfBytesReceipt,
    trace_json: Option<&str>,
    sidecar_read: PublicationReadLedgerToken,
    terminal_read: PublicationReadLedgerToken,
) -> Result<(), Failure> {
    let diagnostics_json = encode_diagnostics_canonical(diagnostics.finish().diagnostics());
    let terminal = match publication {
        Some(publication) => {
            drop(terminal_read);
            let table_layouts = layout.table_manifest_facts()?;
            let footnote_layout = layout.footnote_manifest_facts(package, config)?;
            let mut layout_facts =
                StagingMachineLayoutFacts::new(layout.flow_registry_sha256(), table_layouts);
            if let Some(footnote_layout) = footnote_layout {
                layout_facts = layout_facts.with_footnote(footnote_layout);
            }
            let prepared = match publication.prepare_machine_built(
                package,
                receipt,
                layout.preparation().admitted().token(),
                layout.pagination(),
                layout_facts,
                pdf,
            ) {
                Ok(prepared) => prepared,
                Err(error) => {
                    let primary = Failure::internal(format!(
                        "machine built-manifest preflight failed: {error:?}"
                    ));
                    let diagnostics = publish_machine_diagnostics_bytes(
                        execution,
                        &diagnostics_json,
                        Some(&sidecar_read),
                    );
                    return Err(combine_machine_publication_failures(
                        primary,
                        diagnostics.err(),
                        None,
                    ));
                }
            };
            match output.stage_prepared_built(prepared) {
                Ok(staged) => PreparedMachineTerminal::Manifest(Box::new(staged)),
                Err(error) => {
                    let primary = map_built_staging_error(error);
                    let diagnostics = publish_machine_diagnostics_bytes(
                        execution,
                        &diagnostics_json,
                        Some(&sidecar_read),
                    );
                    return Err(combine_machine_publication_failures(
                        primary,
                        diagnostics.err(),
                        None,
                    ));
                }
            }
        }
        None => match output.prepare_pdf_without_manifest_with_read_ledger(pdf, terminal_read) {
            Ok(prepared) => PreparedMachineTerminal::PdfOnly(Box::new(prepared)),
            Err(error) => {
                let primary = map_pdf_commit_error(error);
                let diagnostics = publish_machine_diagnostics_bytes(
                    execution,
                    &diagnostics_json,
                    Some(&sidecar_read),
                );
                return Err(combine_machine_publication_failures(
                    primary,
                    diagnostics.err(),
                    None,
                ));
            }
        },
    };

    // Every requested file has a complete, fsynced temporary before the first
    // visible success artifact. Publication below is deliberately individual,
    // not a multi-file transaction.
    let prepared_diagnostics = if execution.diagnostics_target().is_some() {
        match sidecar::prepare_build(
            execution,
            sidecar::SidecarArtifact::Diagnostics,
            diagnostics_json.as_bytes(),
            Some(&sidecar_read),
        ) {
            Ok(prepared) => Some(prepared),
            Err(error) => {
                let pending = terminal.fail_before_pdf();
                return Err(finish_machine_failed_publication(
                    map_diagnostics_publication_error(error),
                    None,
                    pending,
                ));
            }
        }
    } else {
        None
    };
    let prepared_trace = match trace_json {
        Some(trace) => match sidecar::prepare_build(
            execution,
            sidecar::SidecarArtifact::Trace,
            trace.as_bytes(),
            Some(&sidecar_read),
        ) {
            Ok(prepared) => Some(prepared),
            Err(error) => {
                let pending = terminal.fail_before_pdf();
                let diagnostics = publish_prepared_machine_diagnostics(
                    execution,
                    prepared_diagnostics,
                    &sidecar_read,
                );
                return Err(finish_machine_failed_publication(
                    map_trace_publication_error(error, None),
                    diagnostics.err(),
                    pending,
                ));
            }
        },
        None => None,
    };
    let trace_visible = match prepared_trace {
        Some(prepared) => match sidecar::publish_build(execution, prepared, Some(&sidecar_read)) {
            Ok(_) => true,
            Err(error) => {
                let pending = terminal.fail_before_pdf();
                let diagnostics = publish_prepared_machine_diagnostics(
                    execution,
                    prepared_diagnostics,
                    &sidecar_read,
                );
                return Err(finish_machine_failed_publication(
                    map_trace_publication_error(error, None),
                    diagnostics.err(),
                    pending,
                ));
            }
        },
        None => false,
    };

    let pending_built = match terminal {
        PreparedMachineTerminal::Manifest(staged) => match staged.commit_pdf() {
            Ok(pending) => Some(pending),
            Err(PreparedPdfCommitError::Invalid(error)) => {
                let diagnostics = publish_prepared_machine_diagnostics(
                    execution,
                    prepared_diagnostics,
                    &sidecar_read,
                );
                let mut primary = map_pdf_commit_error(error);
                if trace_visible {
                    primary.message = format!("trace is already visible; {}", primary.message);
                }
                return Err(combine_machine_publication_failures(
                    primary,
                    diagnostics.err(),
                    None,
                ));
            }
            Err(PreparedPdfCommitError::SinkFailed { source, failed }) => {
                let diagnostics = publish_prepared_machine_diagnostics(
                    execution,
                    prepared_diagnostics,
                    &sidecar_read,
                );
                let mut primary = map_pdf_commit_error(source);
                if trace_visible {
                    primary.message = format!("trace is already visible; {}", primary.message);
                }
                return Err(finish_machine_failed_publication(
                    primary,
                    diagnostics.err(),
                    Some(*failed),
                ));
            }
            Err(PreparedPdfCommitError::DurabilityUncertain { source, .. }) => {
                let diagnostics = publish_prepared_machine_diagnostics(
                    execution,
                    prepared_diagnostics,
                    &sidecar_read,
                );
                let mut primary = Failure::io(format!(
                    "PDF was published but directory synchronization failed: {source}"
                ));
                if trace_visible {
                    primary.message = format!("trace is already visible; {}", primary.message);
                }
                return Err(combine_machine_publication_failures(
                    primary,
                    diagnostics.err(),
                    None,
                ));
            }
        },
        PreparedMachineTerminal::PdfOnly(prepared) => match prepared.commit() {
            Ok(_) => None,
            Err(error) => {
                let diagnostics = publish_prepared_machine_diagnostics(
                    execution,
                    prepared_diagnostics,
                    &sidecar_read,
                );
                let mut primary = map_pdf_commit_error(error);
                if trace_visible {
                    primary.message = format!("trace is already visible; {}", primary.message);
                }
                return Err(combine_machine_publication_failures(
                    primary,
                    diagnostics.err(),
                    None,
                ));
            }
        },
    };

    if let Err(mut diagnostics) =
        publish_prepared_machine_diagnostics(execution, prepared_diagnostics, &sidecar_read)
    {
        let visible = if trace_visible {
            "trace and PDF are already visible"
        } else {
            "PDF is already visible"
        };
        diagnostics.message = format!("{visible}; {}", diagnostics.message);
        // Dropping the pending built capability intentionally keeps the built
        // manifest private after a diagnostics failure.
        drop(pending_built);
        return Err(diagnostics);
    }
    if let Some(pending) = pending_built {
        pending
            .commit_built_manifest()
            .map(|_| ())
            .map_err(|error| map_built_commit_error_with_trace(error, trace_visible))?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn publish_machine_processing_failure(
    execution: &BuildExecutionContext,
    diagnostics: MachineDiagnosticBudget,
    output: BuildOutputCommitContext,
    publication: Option<ManifestPublicationContext>,
    manifest: Option<ManifestAdmissionLedger>,
    pagination: Option<&typaxis_pagination::PaginationResult>,
    failed: FailedMachineCommand,
) -> Failure {
    let FailedMachineCommand {
        primary,
        sidecar_read,
        terminal_read,
    } = failed;
    let diagnostics_json = encode_diagnostics_canonical(diagnostics.finish().diagnostics());
    let pending = match publication {
        Some(publication) => {
            let Some(manifest) = manifest else {
                let diagnostics = publish_machine_diagnostics_bytes(
                    execution,
                    &diagnostics_json,
                    sidecar_read.as_ref(),
                );
                return combine_machine_publication_failures(
                    Failure::internal(format!(
                        "{}; machine failed-manifest ledger was unavailable",
                        primary.message
                    )),
                    diagnostics.err(),
                    None,
                );
            };
            let mut prepared = match publication.prepare_failed(manifest, pagination) {
                Ok(prepared) => prepared,
                Err(error) => {
                    let diagnostics = publish_machine_diagnostics_bytes(
                        execution,
                        &diagnostics_json,
                        sidecar_read.as_ref(),
                    );
                    return combine_machine_publication_failures(
                        Failure::internal(format!(
                            "{}; machine failed-manifest preflight failed: {error:?}",
                            primary.message
                        )),
                        diagnostics.err(),
                        None,
                    );
                }
            };
            if let Some(read) = terminal_read {
                prepared = match prepared.bind_read_ledger(read) {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        let diagnostics = publish_machine_diagnostics_bytes(
                            execution,
                            &diagnostics_json,
                            sidecar_read.as_ref(),
                        );
                        return combine_machine_publication_failures(
                            Failure::internal(format!(
                                "{}; failed-manifest read binding failed: {error:?}",
                                primary.message
                            )),
                            diagnostics.err(),
                            None,
                        );
                    }
                };
            }
            match output.stage_prepared_failed(prepared) {
                Ok(pending) => Some(pending),
                Err(error) => {
                    let diagnostics = publish_machine_diagnostics_bytes(
                        execution,
                        &diagnostics_json,
                        sidecar_read.as_ref(),
                    );
                    return combine_machine_publication_failures(
                        primary,
                        diagnostics.err(),
                        Some(map_failed_commit_error(error)),
                    );
                }
            }
        }
        None => {
            drop(output);
            None
        }
    };
    if execution.diagnostics_target().is_some() {
        let outcome = sidecar::publish_processing_failure(
            primary,
            || {
                let prepared = sidecar::prepare_build(
                    execution,
                    sidecar::SidecarArtifact::Diagnostics,
                    diagnostics_json.as_bytes(),
                    sidecar_read.as_ref(),
                )?;
                sidecar::publish_build(execution, prepared, sidecar_read.as_ref())
            },
            || match pending {
                Some(pending) => pending.commit_failed_manifest().map(|_| ()),
                None => Ok(()),
            },
        );
        let (primary, diagnostics, manifest) = outcome.into_parts();
        combine_machine_publication_failures(
            primary,
            diagnostics.err().map(map_diagnostics_publication_error),
            manifest.err().map(map_failed_commit_error),
        )
    } else {
        finish_machine_failed_publication(primary, None, pending)
    }
}

fn publish_machine_diagnostics_bytes(
    execution: &BuildExecutionContext,
    encoded: &str,
    read: Option<&PublicationReadLedgerToken>,
) -> Result<(), Failure> {
    let Some(_) = execution.diagnostics_target() else {
        return Ok(());
    };
    let prepared = sidecar::prepare_build(
        execution,
        sidecar::SidecarArtifact::Diagnostics,
        encoded.as_bytes(),
        read,
    )
    .map_err(map_diagnostics_publication_error)?;
    sidecar::publish_build(execution, prepared, read)
        .map(|_| ())
        .map_err(map_diagnostics_publication_error)
}

fn publish_prepared_machine_diagnostics(
    execution: &BuildExecutionContext,
    prepared: Option<sidecar::PreparedSidecar>,
    read: &PublicationReadLedgerToken,
) -> Result<(), Failure> {
    match prepared {
        Some(prepared) => sidecar::publish_build(execution, prepared, Some(read))
            .map(|_| ())
            .map_err(map_diagnostics_publication_error),
        None => Ok(()),
    }
}

fn finish_machine_failed_publication(
    primary: Failure,
    diagnostics: Option<Failure>,
    pending: Option<PendingFailedManifestPublication>,
) -> Failure {
    let manifest = pending.and_then(|pending| {
        pending
            .commit_failed_manifest()
            .map(|_| ())
            .map_err(map_failed_commit_error)
            .err()
    });
    combine_machine_publication_failures(primary, diagnostics, manifest)
}

fn combine_machine_publication_failures(
    primary: Failure,
    diagnostics: Option<Failure>,
    manifest: Option<Failure>,
) -> Failure {
    if diagnostics.is_none() && manifest.is_none() {
        return primary;
    }
    let mut message = primary.message;
    if let Some(diagnostics) = diagnostics {
        message.push_str("; diagnostics publication also failed: ");
        message.push_str(&diagnostics.message);
    }
    if let Some(manifest) = manifest {
        message.push_str("; failed-manifest publication also failed: ");
        message.push_str(&manifest.message);
    }
    Failure::io(message)
}

fn map_diagnostics_publication_error(error: sidecar::CommitError) -> Failure {
    let visibility = if error.was_published() {
        "diagnostics are visible but durability is uncertain"
    } else {
        "diagnostics were not published"
    };
    Failure::io(format!(
        "diagnostics publication failed ({visibility}): {error}"
    ))
}

fn run_dump_layout(source: &SourceOptions, physical_page: u32) -> Result<(), Failure> {
    let page_index = physical_page
        .checked_sub(1)
        .ok_or_else(|| Failure::usage("physical page number must be at least 1"))?;
    let loaded = load_config(&source.common)?;
    let admission = admission_context(source, &loaded.effective, loaded.path.as_deref())?;
    let package = pipeline::load_package(admission.entry().as_path(), &loaded.effective)?;
    let layout = pipeline::layout_reference(&package, &loaded.effective, &admission)?;
    pipeline::reject_strict_fallback(&layout, &loaded.effective)?;
    let page = layout
        .pagination
        .selected_pages()
        .get(page_index as usize)
        .filter(|page| page.page_index == page_index)
        .ok_or_else(|| {
            Failure::input(format!(
                "L5001: physical page {physical_page} does not exist"
            ))
        })?;
    let master = package
        .package()
        .page_masters
        .masters
        .iter()
        .find(|master| master.master_id == page.master_id)
        .ok_or_else(|| Failure::internal("selected page refers to an unknown page master"))?;
    let json = artifacts::reference_layout_page_json(
        page,
        master.width.get().raw(),
        master.height.get().raw(),
    )
    .map_err(|message| Failure::internal(format!("layout encoding failed: {message}")))?;
    write_stdout(json.as_bytes())
}

fn run_build(options: BuildOptions) -> Result<(), Failure> {
    let loaded = load_config(&options.common)?;
    let config = loaded.effective;
    let execution = BuildExecutionContext::from_cli_token(
        &options.output,
        options
            .trace
            .clone()
            .map(HostPath::new)
            .transpose()
            .map_err(|_| Failure::usage("trace path must not be empty"))?,
        options
            .manifest
            .clone()
            .map(HostPath::new)
            .transpose()
            .map_err(|_| Failure::usage("manifest path must not be empty"))?,
        None,
        if options.force {
            ReplacePolicy::Replace
        } else {
            ReplacePolicy::NoReplace
        },
    )
    .map_err(map_execution_setup_error)?;
    let output =
        BuildOutputCommitContext::new(&config, &execution).map_err(map_output_context_error)?;
    let publication = if options.manifest.is_some() {
        let versions = config.data_versions();
        let tables =
            ResolvedDataTables::resolve(versions.unicode(), versions.japanese_line_break())
                .ok_or_else(|| Failure::internal("configured data tables are not linked"))?;
        Some(
            ManifestPublicationContext::new(
                &config,
                &output,
                ShaperIdentity::linked_reference(),
                &tables,
            )
            .map_err(map_publication_context_error)?,
        )
    } else {
        None
    };

    let admission = match admission_context(
        &SourceOptions {
            input: options.input.clone(),
            common: options.common.clone(),
        },
        &config,
        loaded.path.as_deref(),
    ) {
        Ok(admission) => admission,
        Err(error) => {
            publish_failed(output, publication, None, None, None)?;
            return Err(error);
        }
    };

    let package = match pipeline::load_package(admission.entry().as_path(), &config) {
        Ok(package) => package,
        Err(error) => {
            publish_failed(output, publication, None, None, None)?;
            return Err(error);
        }
    };
    let layout = match pipeline::layout_reference(&package, &config, &admission) {
        Ok(layout) => layout,
        Err(error) => {
            if error.should_publish_failed_manifest() {
                publish_failed(output, publication, Some(&package), None, None)?;
            }
            return Err(error);
        }
    };

    let trace_json = if options.trace.is_some() {
        let json = match artifacts::reference_layout_trace_json(
            &layout.flow,
            &layout.initial,
            &layout.pagination,
            config.limits().get().max_layout_passes,
            options.trace_text,
        ) {
            Ok(json) => json,
            Err(message) => {
                publish_failed(
                    output,
                    publication,
                    Some(&package),
                    Some(&layout.admitted),
                    Some(&layout.pagination),
                )?;
                return Err(map_trace_artifact_error(message, "trace encoding failed"));
            }
        };
        Some(json)
    } else {
        None
    };

    if let Err(error) = pipeline::reject_strict_fallback(&layout, &config) {
        publish_processing_failure_with_trace(
            &execution,
            trace_json.as_deref(),
            output,
            publication,
            Some(&package),
            Some(&layout.admitted),
            Some(&layout.pagination),
        )?;
        return Err(error);
    }

    let graph = match pipeline::build_pdf_graph(&package, &config, &layout) {
        Ok(graph) => graph,
        Err(error) => {
            publish_failed(
                output,
                publication,
                Some(&package),
                Some(&layout.admitted),
                Some(&layout.pagination),
            )?;
            return Err(error);
        }
    };
    let pdf = match typaxis_pdf::PdfBackend::serialize(graph, &config) {
        Ok(pdf) => pdf,
        Err(error) => {
            publish_failed(
                output,
                publication,
                Some(&package),
                Some(&layout.admitted),
                Some(&layout.pagination),
            )?;
            return Err(match error {
                typaxis_pdf::PdfError::OutputTooLarge => {
                    Failure::limit("serialized PDF exceeds max_output_bytes")
                }
                _ => Failure::internal(format!("PDF serialization failed: {error:?}")),
            });
        }
    };

    let terminal = match publication {
        Some(publication) => {
            let prepared = publication
                .prepare_built(&package, layout.admitted.token(), &layout.pagination, pdf)
                .map_err(|error| {
                    Failure::internal(format!("build manifest preflight failed: {error:?}"))
                })?;
            let staged = output
                .stage_prepared_built(prepared)
                .map_err(map_built_staging_error)?;
            PreparedBuildTerminal::Manifest(Box::new(staged))
        }
        None => {
            let prepared = output
                .prepare_pdf_without_manifest(pdf)
                .map_err(map_pdf_commit_error)?;
            PreparedBuildTerminal::PdfOnly(Box::new(prepared))
        }
    };

    let prepared_trace = match trace_json.as_ref() {
        Some(json) => match sidecar::prepare_build(
            &execution,
            sidecar::SidecarArtifact::Trace,
            json.as_bytes(),
            None,
        ) {
            Ok(prepared) => Some(prepared),
            Err(error) => return Err(terminal.fail_before_pdf(error)),
        },
        None => None,
    };
    let trace_receipt = match prepared_trace {
        Some(prepared) => match sidecar::publish_build(&execution, prepared, None) {
            Ok(receipt) => Some(receipt),
            Err(error) => return Err(terminal.fail_before_pdf(error)),
        },
        None => None,
    };

    terminal.commit(trace_receipt.is_some())?;
    Ok(())
}

enum PreparedBuildTerminal {
    Manifest(Box<StagedBuiltPublication>),
    PdfOnly(Box<PreparedStandalonePdfPublication>),
}

impl PreparedBuildTerminal {
    fn fail_before_pdf(self, primary: sidecar::CommitError) -> Failure {
        let manifest_result = match self {
            Self::Manifest(staged) => {
                let failed = staged.fail_before_pdf();
                Some(match failed.commit_failed_manifest() {
                    Ok(publication) => FailedManifestPublication::Committed(Box::new(publication)),
                    Err(error) => FailedManifestPublication::CommitError(Box::new(error)),
                })
            }
            Self::PdfOnly(_) => None,
        };
        map_trace_publication_error(primary, manifest_result)
    }

    fn commit(self, trace_visible: bool) -> Result<(), Failure> {
        match self {
            Self::Manifest(staged) => match staged.commit_pdf() {
                Ok(pending) => pending
                    .commit_built_manifest()
                    .map(|_| ())
                    .map_err(|error| map_built_commit_error_with_trace(error, trace_visible)),
                Err(PreparedPdfCommitError::Invalid(source)) => {
                    Err(map_built_commit_error_with_trace(
                        BuiltPublicationCommitError::Pdf(source),
                        trace_visible,
                    ))
                }
                Err(PreparedPdfCommitError::SinkFailed { source, failed }) => {
                    let failed_manifest = match failed.commit_failed_manifest() {
                        Ok(publication) => {
                            FailedManifestPublication::Committed(Box::new(publication))
                        }
                        Err(error) => FailedManifestPublication::CommitError(Box::new(error)),
                    };
                    Err(map_built_commit_error_with_trace(
                        BuiltPublicationCommitError::PdfSinkFailed {
                            source,
                            failed_manifest,
                        },
                        trace_visible,
                    ))
                }
                Err(PreparedPdfCommitError::DurabilityUncertain {
                    pdf_receipt,
                    source,
                }) => Err(map_built_commit_error_with_trace(
                    BuiltPublicationCommitError::PdfDurability {
                        pdf_receipt,
                        source,
                    },
                    trace_visible,
                )),
            },
            Self::PdfOnly(prepared) => prepared.commit().map(|_| ()).map_err(|error| {
                let failure = map_pdf_commit_error(error);
                if trace_visible {
                    Failure::io(format!("trace is already visible; {}", failure.message))
                } else {
                    failure
                }
            }),
        }
    }
}

fn publish_failed(
    output: BuildOutputCommitContext,
    publication: Option<ManifestPublicationContext>,
    package: Option<&ValidatedParsedPackage>,
    admitted: Option<&AdmittedResourceLedger>,
    pagination: Option<&typaxis_pagination::PaginationResult>,
) -> Result<(), Failure> {
    let Some(pending) = stage_failed(output, publication, package, admitted, pagination)? else {
        return Ok(());
    };
    pending
        .commit_failed_manifest()
        .map_err(map_failed_commit_error)?;
    Ok(())
}

fn publish_processing_failure_with_trace(
    execution: &BuildExecutionContext,
    trace_json: Option<&str>,
    output: BuildOutputCommitContext,
    publication: Option<ManifestPublicationContext>,
    package: Option<&ValidatedParsedPackage>,
    admitted: Option<&AdmittedResourceLedger>,
    pagination: Option<&typaxis_pagination::PaginationResult>,
) -> Result<(), Failure> {
    let pending = stage_failed(output, publication, package, admitted, pagination)?;
    let prepared_trace = match trace_json {
        Some(json) => match sidecar::prepare_build(
            execution,
            sidecar::SidecarArtifact::Trace,
            json.as_bytes(),
            None,
        ) {
            Ok(prepared) => Some(prepared),
            Err(primary) => {
                return Err(map_trace_publication_error(
                    primary,
                    commit_pending_failed(pending),
                ));
            }
        },
        None => None,
    };
    let trace_visible = match prepared_trace {
        Some(prepared) => match sidecar::publish_build(execution, prepared, None) {
            Ok(_) => true,
            Err(primary) => {
                return Err(map_trace_publication_error(
                    primary,
                    commit_pending_failed(pending),
                ));
            }
        },
        None => false,
    };
    if let Some(pending) = pending {
        pending.commit_failed_manifest().map_err(|error| {
            let mut failure = map_failed_commit_error(error);
            if trace_visible {
                failure.message = format!("trace is already visible; {}", failure.message);
            }
            failure
        })?;
    }
    Ok(())
}

fn commit_pending_failed(
    pending: Option<PendingFailedManifestPublication>,
) -> Option<FailedManifestPublication> {
    pending.map(|pending| match pending.commit_failed_manifest() {
        Ok(publication) => FailedManifestPublication::Committed(Box::new(publication)),
        Err(error) => FailedManifestPublication::CommitError(Box::new(error)),
    })
}

fn stage_failed(
    output: BuildOutputCommitContext,
    publication: Option<ManifestPublicationContext>,
    package: Option<&ValidatedParsedPackage>,
    admitted: Option<&AdmittedResourceLedger>,
    pagination: Option<&typaxis_pagination::PaginationResult>,
) -> Result<Option<PendingFailedManifestPublication>, Failure> {
    let Some(publication) = publication else {
        return Ok(None);
    };
    let mut ledger = publication.begin_admission_ledger();
    if let Some(package) = package {
        ledger
            .admit_validated_package_sources(package)
            .map_err(|error| {
                Failure::internal(format!(
                    "failed-manifest source admission failed: {error:?}"
                ))
            })?;
    }
    if let Some(admitted) = admitted {
        ledger.admit_resources(admitted.token()).map_err(|error| {
            Failure::internal(format!(
                "failed-manifest resource admission failed: {error:?}"
            ))
        })?;
    }
    let prepared = publication
        .prepare_failed(ledger, pagination)
        .map_err(|error| {
            Failure::internal(format!("failed build manifest preflight failed: {error:?}"))
        })?;
    let pending = output
        .stage_prepared_failed(prepared)
        .map_err(map_failed_commit_error)?;
    Ok(Some(pending))
}

struct LoadedConfig {
    effective: EffectiveConfig,
    path: Option<PathBuf>,
}

fn load_config(common: &CommonOptions) -> Result<LoadedConfig, Failure> {
    let config_path = match &common.config {
        Some(path) => Some(path.clone()),
        None => {
            let default = PathBuf::from("typaxis.toml");
            // Inspect the directory entry itself so a dangling symlink or a
            // non-regular default config is not silently treated as absent.
            match fs::symlink_metadata(&default) {
                Ok(_) => Some(default),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(Failure::io(format!(
                        "cannot inspect default config `typaxis.toml`: {error}"
                    )))
                }
            }
        }
    };
    let mut overrides = config::ConfigOverrides::default();
    overrides.strict = common.strict.then_some(true);
    overrides.no_compress = common.no_compress;
    for (name, value) in &common.limits {
        overrides
            .set_limit(name, *value)
            .map_err(map_config_error)?;
    }
    let effective = config::load_from_process_env(config_path.as_deref(), &overrides)
        .map_err(map_config_error)?;
    Ok(LoadedConfig {
        effective,
        path: config_path,
    })
}

fn map_config_error(error: config::ConfigError) -> Failure {
    let message = format!("{}: {error}", error.diagnostic_code());
    if error.is_io() {
        Failure::io(message)
    } else if error.is_limit() {
        Failure::limit(message)
    } else {
        Failure::usage(message)
    }
}

fn admission_context(
    options: &SourceOptions,
    effective: &EffectiveConfig,
    config_path: Option<&Path>,
) -> Result<HostAdmissionContext, Failure> {
    let current = std::env::current_dir()
        .map_err(|error| Failure::io(format!("cannot determine current directory: {error}")))?;
    let project_root = config_path
        .and_then(Path::parent)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(&current)
        .to_path_buf();
    let mut root_identities = BTreeSet::new();
    for root in effective.resource_roots() {
        let resolved = match root {
            typaxis_core::ConfigResourceRoot::ProjectRoot => project_root.clone(),
            typaxis_core::ConfigResourceRoot::Relative(path) => project_root.join(path.as_str()),
        };
        admit_resource_root_identity(
            &mut root_identities,
            validate_resource_root(&resolved, "configured")?,
            &resolved,
            "configured",
        )?;
    }
    for root in &options.common.resource_roots {
        admit_resource_root_identity(
            &mut root_identities,
            validate_resource_root(root, "CLI")?,
            root,
            "CLI",
        )?;
    }
    let entry = HostPath::new(options.input.clone())
        .map_err(|_| Failure::usage("INPUT path must not be empty"))?;
    let project_root = HostPath::new(project_root)
        .map_err(|_| Failure::internal("project root resolved to an empty path"))?;
    let config = config_path
        .map(Path::to_path_buf)
        .map(HostPath::new)
        .transpose()
        .map_err(|_| Failure::usage("config path must not be empty"))?;
    let roots = options
        .common
        .resource_roots
        .iter()
        .cloned()
        .map(HostPath::new)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Failure::usage("resource-root path must not be empty"))?;
    Ok(HostAdmissionContext::new(
        entry,
        project_root,
        config,
        roots,
    ))
}

/// Machine commands construct their host context independently from source
/// commands. Only resource-root normalization is intentionally equivalent;
/// PACKAGE and companion-source admission remain owned by
/// `HostMachineInputSession`.
fn machine_admission_context(
    package: &Path,
    common: &CommonOptions,
    effective: &EffectiveConfig,
    config_path: Option<&Path>,
) -> Result<HostAdmissionContext, Failure> {
    let current = std::env::current_dir()
        .map_err(|error| Failure::io(format!("cannot determine current directory: {error}")))?;
    let project_root = config_path
        .and_then(Path::parent)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(&current)
        .to_path_buf();
    let mut root_identities = BTreeSet::new();
    for root in effective.resource_roots() {
        let resolved = match root {
            typaxis_core::ConfigResourceRoot::ProjectRoot => project_root.clone(),
            typaxis_core::ConfigResourceRoot::Relative(path) => project_root.join(path.as_str()),
        };
        admit_resource_root_identity(
            &mut root_identities,
            validate_resource_root(&resolved, "configured")?,
            &resolved,
            "configured",
        )?;
    }
    for root in &common.resource_roots {
        admit_resource_root_identity(
            &mut root_identities,
            validate_resource_root(root, "CLI")?,
            root,
            "CLI",
        )?;
    }
    let entry = HostPath::new(package.to_path_buf())
        .map_err(|_| Failure::usage("PACKAGE path must not be empty"))?;
    let project_root = HostPath::new(project_root)
        .map_err(|_| Failure::internal("project root resolved to an empty path"))?;
    let config = config_path
        .map(Path::to_path_buf)
        .map(HostPath::new)
        .transpose()
        .map_err(|_| Failure::usage("config path must not be empty"))?;
    let roots = common
        .resource_roots
        .iter()
        .cloned()
        .map(HostPath::new)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Failure::usage("resource-root path must not be empty"))?;
    Ok(HostAdmissionContext::new(
        entry,
        project_root,
        config,
        roots,
    ))
}

fn reject_known_machine_check_aliases(
    options: &CheckPackageOptions,
    config: Option<&Path>,
    execution: Option<&DiagnosticsExecutionContext>,
) -> Result<(), Failure> {
    let Some(execution) = execution else {
        return Ok(());
    };
    reject_known_machine_read_alias(
        &options.package,
        execution.diagnostics_target().as_path(),
        "PACKAGE",
        "diagnostics",
    )?;
    if let Some(config) = config {
        reject_known_machine_read_alias(
            config,
            execution.diagnostics_target().as_path(),
            "config",
            "diagnostics",
        )?;
    }
    Ok(())
}

fn reject_known_machine_build_aliases(
    options: &BuildPackageOptions,
    config: Option<&Path>,
    execution: &BuildExecutionContext,
) -> Result<(), Failure> {
    let mut targets = Vec::with_capacity(4);
    if let Some(target) = execution.output_path() {
        targets.push(("PDF", target.as_path()));
    }
    if let Some(target) = execution.trace_target() {
        targets.push(("trace", target.as_path()));
    }
    if let Some(target) = execution.manifest_target() {
        targets.push(("manifest", target.as_path()));
    }
    if let Some(target) = execution.diagnostics_target() {
        targets.push(("diagnostics", target.as_path()));
    }
    for (label, target) in targets {
        reject_known_machine_read_alias(&options.package, target, "PACKAGE", label)?;
        if let Some(config) = config {
            reject_known_machine_read_alias(config, target, "config", label)?;
        }
    }
    Ok(())
}

fn reject_known_machine_read_alias(
    read: &Path,
    write: &Path,
    read_label: &str,
    write_label: &str,
) -> Result<(), Failure> {
    if host_paths_alias(read, write)? {
        Err(Failure::usage(format!(
            "{write_label} target aliases the known {read_label} input"
        )))
    } else {
        Ok(())
    }
}

fn host_paths_alias(first: &Path, second: &Path) -> Result<bool, Failure> {
    let first_lexical = absolute_lexical_path(first)?;
    let second_lexical = absolute_lexical_path(second)?;
    if first_lexical == second_lexical {
        return Ok(true);
    }

    let first_canonical = fs::canonicalize(first).ok();
    let second_canonical = fs::canonicalize(second).ok();
    if first_canonical
        .as_ref()
        .zip(second_canonical.as_ref())
        .is_some_and(|(first, second)| first == second)
    {
        return Ok(true);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(first), Ok(second)) = (fs::metadata(first), fs::metadata(second)) {
            if first.dev() == second.dev() && first.ino() == second.ino() {
                return Ok(true);
            }
        }
    }

    let first_parent_leaf = canonical_parent_leaf(first);
    let second_parent_leaf = canonical_parent_leaf(second);
    Ok(first_parent_leaf
        .zip(second_parent_leaf)
        .is_some_and(|(first, second)| first == second))
}

fn absolute_lexical_path(path: &Path) -> Result<PathBuf, Failure> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| Failure::io(format!("cannot determine current directory: {error}")))?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}

fn canonical_parent_leaf(path: &Path) -> Option<(PathBuf, std::ffi::OsString)> {
    let leaf = path.file_name()?.to_os_string();
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    Some((fs::canonicalize(parent).ok()?, leaf))
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ResourceRootIdentity {
    #[cfg(unix)]
    Unix { device: u64, inode: u64 },
    #[cfg(not(unix))]
    Canonical(PathBuf),
}

fn validate_resource_root(root: &Path, origin: &str) -> Result<ResourceRootIdentity, Failure> {
    let canonical = fs::canonicalize(root).map_err(|error| {
        Failure::io(format!(
            "cannot resolve {origin} resource root `{}`: {error}",
            root.display()
        ))
    })?;
    let metadata = fs::metadata(&canonical).map_err(|error| {
        Failure::io(format!(
            "cannot inspect {origin} resource root `{}`: {error}",
            root.display()
        ))
    })?;
    if metadata.is_dir() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(ResourceRootIdentity::Unix {
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        }
        #[cfg(not(unix))]
        {
            Ok(ResourceRootIdentity::Canonical(canonical))
        }
    } else {
        Err(Failure::io(format!(
            "{origin} resource root `{}` is not a directory",
            root.display()
        )))
    }
}

fn admit_resource_root_identity(
    identities: &mut BTreeSet<ResourceRootIdentity>,
    identity: ResourceRootIdentity,
    root: &Path,
    origin: &str,
) -> Result<(), Failure> {
    if identities.insert(identity) {
        Ok(())
    } else {
        Err(Failure::usage(format!(
            "{origin} resource root `{}` aliases another configured resource root",
            root.display()
        )))
    }
}

fn map_font_error(error: font::FontCommandError) -> Failure {
    if error.is_io() {
        Failure::io(error.to_string())
    } else if error.is_resource_limit() {
        Failure::limit(format!("I9000: {error}"))
    } else {
        Failure::input(format!("F4000: {error}"))
    }
}

fn map_document_package_artifact_error(error: artifacts::DocumentPackageArtifactError) -> Failure {
    match error {
        limit @ artifacts::DocumentPackageArtifactError::Encoding(
            typaxis_document_package::JcsEncodeError::ByteLimitExceeded { .. },
        ) => Failure::limit(format!("DocumentPackage encoding limit exceeded: {limit}")),
        artifacts::DocumentPackageArtifactError::Encoding(
            typaxis_document_package::JcsEncodeError::Write(source),
        ) => Failure::io(format!("cannot write DocumentPackage to stdout: {source}")),
        other => Failure::internal(format!("AST encoding failed: {other}")),
    }
}

fn map_trace_artifact_error(message: &str, context: &str) -> Failure {
    if message == artifacts::GENERATED_TRACE_TEXT_REQUIRES_OPT_IN {
        Failure::usage(message)
    } else {
        Failure::internal(format!("{context}: {message}"))
    }
}

fn map_execution_setup_error(error: BuildExecutionError) -> Failure {
    match error {
        BuildExecutionError::EmptyOutput | BuildExecutionError::AliasedWriteTarget => {
            Failure::usage(format!("invalid build write targets: {error:?}"))
        }
        BuildExecutionError::CurrentDirectoryUnavailable => {
            Failure::io("cannot resolve build write targets")
        }
        BuildExecutionError::AliasedReadWriteTarget => {
            Failure::io("a build write target aliases an admitted input")
        }
        BuildExecutionError::ReadTargetChanged => {
            Failure::io("an admitted input changed before publication")
        }
    }
}

fn map_output_context_error(error: BuildOutputCommitContextError) -> Failure {
    match error {
        BuildOutputCommitContextError::Execution(error) => map_execution_setup_error(error),
        BuildOutputCommitContextError::SessionIdentityExhausted => {
            Failure::internal("build output session identity exhausted")
        }
    }
}

fn map_publication_context_error(error: ManifestPublicationError) -> Failure {
    match error {
        ManifestPublicationError::MissingManifestTarget => {
            Failure::internal("manifest publication has no target")
        }
        _ => Failure::internal(format!("manifest publication setup failed: {error:?}")),
    }
}

fn map_pdf_commit_error(error: PdfSinkCommitError) -> Failure {
    match error {
        PdfSinkCommitError::Io(source) => Failure::io(format!("cannot publish PDF: {source}")),
        PdfSinkCommitError::StdoutPartial {
            bytes_written,
            source,
        } => Failure::io(format!(
            "stdout accepted {bytes_written} PDF bytes before publication failed; the partial stream cannot be rolled back: {source}"
        )),
        PdfSinkCommitError::Execution(source) => Failure::io(format!(
            "PDF targets changed before publication: {source:?}"
        )),
        PdfSinkCommitError::PublishedButDurabilityUncertain { source, .. } => Failure::io(format!(
            "PDF was published but directory synchronization failed: {source}"
        )),
        other => Failure::internal(format!("PDF publication invariant failed: {other:?}")),
    }
}

fn map_built_staging_error(error: BuiltPublicationStagingError) -> Failure {
    match error {
        BuiltPublicationStagingError::Invalid(error) => map_pdf_commit_error(error),
        BuiltPublicationStagingError::Pdf(error) => {
            let failure = map_pdf_commit_error(error);
            Failure::io(format!(
                "cannot stage PDF before publication: {}",
                failure.message
            ))
        }
        BuiltPublicationStagingError::BuiltManifest(error) => Failure::io(format!(
            "cannot stage built manifest before publication: {}",
            map_pdf_commit_error(error).message
        )),
        BuiltPublicationStagingError::FailedManifest(error) => Failure::io(format!(
            "cannot stage failed manifest before publication: {}",
            map_pdf_commit_error(error).message
        )),
    }
}

fn map_built_commit_error(error: BuiltPublicationCommitError) -> Failure {
    match error {
        BuiltPublicationCommitError::Pdf(error) => map_pdf_commit_error(error),
        BuiltPublicationCommitError::PdfSinkFailed {
            source,
            failed_manifest,
        } => map_pdf_sink_failure(source, failed_manifest),
        BuiltPublicationCommitError::PdfDurability { source, .. } => Failure::io(format!(
            "PDF was published but directory synchronization failed: {source}"
        )),
        BuiltPublicationCommitError::ManifestExecution { source, .. } => Failure::io(format!(
            "PDF was published but manifest targets changed: {source:?}"
        )),
        BuiltPublicationCommitError::ManifestIo { source, .. } => Failure::io(format!(
            "PDF was published but manifest publication failed: {source}"
        )),
        BuiltPublicationCommitError::ManifestInvariant { .. } => {
            Failure::internal("PDF was published but a manifest publication invariant failed")
        }
        BuiltPublicationCommitError::ManifestDurability { source, .. } => Failure::io(format!(
            "PDF and manifest were published but directory synchronization failed: {source}"
        )),
    }
}

fn map_built_commit_error_with_trace(
    error: BuiltPublicationCommitError,
    trace_visible: bool,
) -> Failure {
    let mut failure = map_built_commit_error(error);
    if trace_visible {
        failure.message = format!("trace is already visible; {}", failure.message);
    }
    failure
}

fn map_trace_publication_error(
    primary: sidecar::CommitError,
    failed_manifest: Option<FailedManifestPublication>,
) -> Failure {
    let visibility = if primary.was_published() {
        "trace is visible but durability is uncertain"
    } else {
        "trace was not published"
    };
    let artifact = primary.artifact();
    match failed_manifest {
        None => Failure::io(format!(
            "{artifact:?} publication failed ({visibility}): {primary}"
        )),
        Some(FailedManifestPublication::Committed(_)) => Failure::io(format!(
            "{artifact:?} publication failed ({visibility}): {primary}; failed manifest was published"
        )),
        Some(FailedManifestPublication::CommitError(error)) => {
            let secondary = map_failed_commit_error(*error);
            Failure::io(format!(
                "{artifact:?} publication failed ({visibility}): {primary}; failed-manifest publication also failed: {}",
                secondary.message
            ))
        }
    }
}

fn map_pdf_sink_failure(
    source: PdfSinkCommitError,
    failed_manifest: FailedManifestPublication,
) -> Failure {
    match failed_manifest {
        FailedManifestPublication::Committed(_) => map_pdf_commit_error(source),
        FailedManifestPublication::CommitError(error) => {
            let pdf_failure = format!("{source:?}");
            match *error {
                ManifestSinkCommitError::Io(source) => Failure::io(format!(
                    "PDF sink failed ({pdf_failure}); failed-manifest publication also failed: {source}"
                )),
                ManifestSinkCommitError::Execution(source) => Failure::io(format!(
                    "PDF sink failed ({pdf_failure}); failed-manifest targets changed: {source:?}"
                )),
                ManifestSinkCommitError::PublishedButDurabilityUncertain { source, .. } => {
                    Failure::io(format!(
                        "PDF sink failed ({pdf_failure}); failed manifest is visible but directory synchronization failed: {source}"
                    ))
                }
                ManifestSinkCommitError::InvalidFacts(error) => Failure::internal(format!(
                    "PDF sink failed ({pdf_failure}); failed-manifest facts were invalid: {error:?}"
                )),
                ManifestSinkCommitError::MissingManifestTarget => Failure::internal(format!(
                    "PDF sink failed ({pdf_failure}); failed-manifest target was missing"
                )),
            }
        }
    }
}

fn map_failed_commit_error(error: ManifestSinkCommitError) -> Failure {
    match error {
        ManifestSinkCommitError::Io(source) => {
            Failure::io(format!("cannot publish failed build manifest: {source}"))
        }
        ManifestSinkCommitError::Execution(source) => Failure::io(format!(
            "manifest targets changed before failed publication: {source:?}"
        )),
        ManifestSinkCommitError::PublishedButDurabilityUncertain { source, .. } => Failure::io(
            format!("failed manifest was published but directory sync failed: {source}"),
        ),
        other => Failure::internal(format!(
            "failed build manifest publication invariant failed: {other:?}"
        )),
    }
}

fn write_stdout(bytes: &[u8]) -> Result<(), Failure> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout
        .write_all(bytes)
        .and_then(|_| stdout.flush())
        .map_err(|error| Failure::io(format!("cannot write stdout: {error}")))
}

fn write_stderr(bytes: &[u8]) -> io::Result<()> {
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    stderr.write_all(bytes).and_then(|_| stderr.flush())
}

pub(crate) fn write_stderr_line(message: &str) -> Result<(), Failure> {
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    stderr
        .write_all(message.as_bytes())
        .and_then(|_| stderr.write_all(b"\n"))
        .and_then(|_| stderr.flush())
        .map_err(|error| Failure::io(format!("cannot write stderr: {error}")))
}

fn write_help(program: &str, command: Option<&str>) -> Result<(), Failure> {
    let text = match command {
        None => format!(
            "Typaxis reference typesetting CLI\n\nUSAGE:\n  {program} <COMMAND> [OPTIONS]\n\nCOMMANDS:\n  build          Build a PDF from reference TSF\n  build-package  Build a PDF from a DocumentPackage\n  capabilities   Write the machine capability descriptor\n  check          Validate a reference TSF input\n  check-package  Validate and prepare a DocumentPackage\n  dump-ast       Write the parsed package as JSON\n  dump-layout    Write one physical page layout as JSON\n  inspect-font   Inspect an SFNT font\n  list-fonts     List SFNT fonts in a directory\n\nRun `{program} help <COMMAND>` for command usage.\n"
        ),
        Some("build") => format!(
            "USAGE:\n  {program} build INPUT -o OUTPUT [--trace PATH] [--emit-build-manifest PATH] [OPTIONS]\n\nOPTIONS:\n  --config PATH             Use a project config\n  --resource-root DIR       Add an ordered host resource root (repeatable)\n  --strict                  Reject pagination fallback\n  --trace PATH              Atomically write a layout trace\n  --trace-text              Include opted-in trace text; requires --trace\n  --emit-build-manifest P   Atomically write a terminal build manifest\n  --no-compress             Disable PDF stream compression\n  --force                   Atomically replace existing targets\n  --max-<name> N            Override a resource limit\n"
        ),
        Some("build-package") => format!(
            "USAGE:\n  {program} build-package PACKAGE -o OUTPUT [OPTIONS]\n\nOPTIONS:\n  --package-root DIR        Resolve PACKAGE and companion sources beneath DIR\n  --profile ID              Select the machine PDF profile (default: typaxis.machine-pdf/paragraph-1)\n  --config PATH             Use a project config\n  --resource-root DIR       Add an ordered host resource root (repeatable)\n  --strict                  Reject pagination fallback\n  --trace PATH              Atomically write a layout trace\n  --trace-text              Include opted-in trace text; requires --trace\n  --emit-build-manifest P   Atomically write a terminal build manifest\n  --emit-diagnostics PATH   Atomically write canonical diagnostics\n  --no-compress             Disable PDF stream compression\n  --force                   Atomically replace existing targets\n  --max-<name> N            Override a resource limit\n"
        ),
        Some("capabilities") => {
            format!("USAGE:\n  {program} capabilities --format json\n")
        }
        Some("check") => format!("USAGE:\n  {program} check INPUT [OPTIONS]\n"),
        Some("check-package") => format!(
            "USAGE:\n  {program} check-package PACKAGE [OPTIONS]\n\nOPTIONS:\n  --package-root DIR        Resolve PACKAGE and companion sources beneath DIR\n  --profile ID              Select the machine PDF profile (default: typaxis.machine-pdf/paragraph-1)\n  --config PATH             Use a project config\n  --resource-root DIR       Add an ordered host resource root (repeatable)\n  --emit-diagnostics PATH   Atomically write canonical diagnostics\n  --max-<name> N            Override a resource limit\n"
        ),
        Some("dump-ast") => {
            format!("USAGE:\n  {program} dump-ast INPUT --format json [OPTIONS]\n")
        }
        Some("dump-layout") => {
            format!("USAGE:\n  {program} dump-layout INPUT --page N [OPTIONS]\n")
        }
        Some("inspect-font") => format!("USAGE:\n  {program} inspect-font FONT\n"),
        Some("list-fonts") => format!("USAGE:\n  {program} list-fonts --font-dir DIR\n"),
        Some(_) => return Err(Failure::internal("help dispatch received an unknown command")),
    };
    write_stdout(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    struct MachineRunnerTree(PathBuf);

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    impl MachineRunnerTree {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "typaxis-mi1-15-{label}-{}-{}",
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
    impl Drop for MachineRunnerTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_common(root: &Path) -> CommonOptions {
        CommonOptions {
            resource_roots: vec![root.to_path_buf()],
            ..CommonOptions::default()
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn machine_build_options(root: &Path) -> BuildPackageOptions {
        BuildPackageOptions {
            package: root.join("document-package.json"),
            package_root: Some(root.to_path_buf()),
            profile: typaxis_core::MachinePdfProfileId::PARAGRAPH_1,
            output: root.join("output.pdf").into_os_string(),
            trace: Some(root.join("trace.json")),
            trace_text: false,
            manifest: Some(root.join("manifest.json")),
            diagnostics: Some(root.join("diagnostics.json")),
            force: false,
            common: machine_common(root),
        }
    }

    #[test]
    fn exit_codes_match_the_cli_contract() {
        assert_eq!(FailureKind::Input.exit_code(), 1);
        assert_eq!(FailureKind::Usage.exit_code(), 2);
        assert_eq!(FailureKind::Io.exit_code(), 3);
        assert_eq!(FailureKind::Internal.exit_code(), 4);
        assert_eq!(FailureKind::Limit.exit_code(), 5);
    }

    #[test]
    fn generated_trace_text_opt_in_failure_is_a_usage_error() {
        let failure = map_trace_artifact_error(
            artifacts::GENERATED_TRACE_TEXT_REQUIRES_OPT_IN,
            "machine trace encoding failed",
        );
        assert_eq!(failure.kind, FailureKind::Usage);
        assert_eq!(
            failure.message,
            artifacts::GENERATED_TRACE_TEXT_REQUIRES_OPT_IN
        );
    }

    #[test]
    fn exact_dash_is_left_for_the_execution_context() {
        let output = std::ffi::OsString::from("-");
        let execution = BuildExecutionContext::from_cli_token(
            &output,
            None,
            None,
            None,
            ReplacePolicy::NoReplace,
        )
        .unwrap();
        assert!(execution.output_path().is_none());
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn private_machine_runners_accept_blank_and_paragraph_packages() {
        for kind in ["blank", "paragraph"] {
            let tree = MachineRunnerTree::new(kind);
            pipeline::tests::write_machine_runner_fixture(tree.path(), kind);
            run_check_package(CheckPackageOptions {
                package: tree.path().join("document-package.json"),
                package_root: Some(tree.path().to_path_buf()),
                profile: typaxis_core::MachinePdfProfileId::PARAGRAPH_1,
                diagnostics: Some(tree.path().join("check-diagnostics.json")),
                common: machine_common(tree.path()),
            })
            .unwrap_or_else(|error| panic!("{kind} check failed: {error:?}"));
            assert!(
                fs::read_to_string(tree.path().join("check-diagnostics.json"))
                    .unwrap()
                    .contains("\"diagnostics\":[]")
            );
            assert!(!tree.path().join("output.pdf").exists());
            assert!(!tree.path().join("manifest.json").exists());
            run_build_package(machine_build_options(tree.path()))
                .unwrap_or_else(|error| panic!("{kind} build failed: {error:?}"));

            assert!(fs::read(tree.path().join("output.pdf"))
                .unwrap()
                .starts_with(b"%PDF-"));
            assert!(fs::read_to_string(tree.path().join("trace.json"))
                .unwrap()
                .contains("\"contract\":\"typaxis.contract/1.2\""));
            let manifest = fs::read_to_string(tree.path().join("manifest.json")).unwrap();
            assert!(manifest.contains("\"status\":\"built\""));
            assert!(manifest.contains("\"input_profile\":\"typaxis.machine-pdf/paragraph-1\""));
            assert!(fs::read_to_string(tree.path().join("diagnostics.json"))
                .unwrap()
                .contains("\"diagnostics\":[]"));
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn private_machine_capability_failure_stops_before_resource_and_pdf_work() {
        let tree = MachineRunnerTree::new("capability-failure");
        pipeline::tests::write_machine_runner_fixture(tree.path(), "unsupported-inline");
        fs::remove_file(tree.path().join("body.ttf")).unwrap();

        let error = run_build_package(machine_build_options(tree.path())).unwrap_err();
        assert_eq!(error.kind, FailureKind::Input);
        assert!(error.message.contains("L5100"));
        assert!(!tree.path().join("body.ttf").exists());
        assert!(!tree.path().join("output.pdf").exists());
        assert!(!tree.path().join("trace.json").exists());
        assert!(fs::read_to_string(tree.path().join("diagnostics.json"))
            .unwrap()
            .contains("\"code\":\"L5100\""));
        let manifest = fs::read_to_string(tree.path().join("manifest.json")).unwrap();
        assert!(manifest.contains("\"status\":\"failed\""));
        assert!(manifest.contains("\"input_profile\":\"typaxis.machine-pdf/paragraph-1\""));
        assert!(manifest.contains("\"package_input\":{"));
        assert!(manifest.contains("\"fonts\":[]"));
    }

    #[test]
    fn resource_roots_must_resolve_to_directories() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("typaxis-cli-root-{nonce}"));
        fs::create_dir(&directory).unwrap();
        assert!(validate_resource_root(&directory, "test").is_ok());

        let file = directory.join("file");
        fs::write(&file, b"not a directory").unwrap();
        assert_eq!(
            validate_resource_root(&file, "test").unwrap_err().kind,
            FailureKind::Io
        );
        assert_eq!(
            validate_resource_root(&directory.join("missing"), "test")
                .unwrap_err()
                .kind,
            FailureKind::Io
        );

        fs::remove_file(file).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn resource_root_aliases_are_usage_errors_before_content_is_loaded() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("typaxis-cli-root-alias-{nonce}"));
        fs::create_dir(&directory).unwrap();

        let mut identities = BTreeSet::new();
        let first = validate_resource_root(&directory, "CLI").unwrap();
        admit_resource_root_identity(&mut identities, first, &directory, "CLI").unwrap();
        let lexical_alias = directory.join(".");
        let duplicate = validate_resource_root(&lexical_alias, "CLI").unwrap();
        let error = admit_resource_root_identity(&mut identities, duplicate, &lexical_alias, "CLI")
            .unwrap_err();
        assert_eq!(error.kind, FailureKind::Usage);

        fs::remove_dir(directory).unwrap();
    }
}
