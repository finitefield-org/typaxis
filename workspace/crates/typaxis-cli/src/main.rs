#![forbid(unsafe_code)]

mod artifacts;
mod cli;
mod config;
mod font;
mod pipeline;
mod sidecar;

use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use cli::{BuildOptions, Command, CommonOptions, Invocation, SourceOptions};
use pipeline::{Failure, FailureKind};
use typaxis_core::{
    BuildExecutionContext, BuildExecutionError, EffectiveConfig, HostAdmissionContext, HostPath,
    ReplacePolicy, ResolvedDataTables, ShaperIdentity,
};
use typaxis_manifest::{
    BuildOutputCommitContext, BuildOutputCommitContextError, BuiltPublicationCommitError,
    FailedManifestPublication, ManifestPublicationContext, ManifestPublicationError,
    ManifestSinkCommitError, PdfSinkCommitError,
};
use typaxis_resources::AdmittedResourceLedger;
use typaxis_syntax::ValidatedParsedPackage;

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
        Invocation::Run(command) => run(command),
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
        Command::Check(options) => {
            let loaded = load_config(&options.common)?;
            let admission = admission_context(&options, &loaded.effective, loaded.path.as_deref())?;
            pipeline::load_package(admission.entry().as_path(), &loaded.effective)?;
            Ok(())
        }
        Command::DumpAst(options) => {
            let loaded = load_config(&options.common)?;
            let admission = admission_context(&options, &loaded.effective, loaded.path.as_deref())?;
            let package = pipeline::load_package(admission.entry().as_path(), &loaded.effective)?;
            let json = artifacts::document_package_json(&package)
                .map_err(|message| Failure::internal(format!("AST encoding failed: {message}")))?;
            write_stdout(json.as_bytes())
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
            publish_failed(output, publication, Some(&package), None, None)?;
            return Err(error);
        }
    };

    if let Some(trace) = &options.trace {
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
                return Err(Failure::internal(format!(
                    "trace encoding failed: {message}"
                )));
            }
        };
        if let Err(error) = sidecar::commit(&execution, trace, json.as_bytes()) {
            let trace_was_published = error.was_published();
            publish_failed(
                output,
                publication,
                Some(&package),
                Some(&layout.admitted),
                Some(&layout.pagination),
            )?;
            return Err(Failure::io(if trace_was_published {
                format!(
                    "trace `{}` is visible but its directory synchronization failed: {error}",
                    trace.display()
                )
            } else {
                format!("cannot publish trace `{}`: {error}", trace.display())
            }));
        }
    }

    if let Err(error) = pipeline::reject_strict_fallback(&layout, &config) {
        publish_failed(
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

    match publication {
        Some(publication) => {
            let prepared = publication
                .prepare_built(&package, layout.admitted.token(), &layout.pagination, pdf)
                .map_err(|error| {
                    Failure::internal(format!("build manifest preflight failed: {error:?}"))
                })?;
            output
                .commit_prepared_built(prepared)
                .map_err(map_built_commit_error)?;
        }
        None => {
            output
                .commit_pdf_without_manifest(pdf)
                .map_err(map_pdf_commit_error)?;
        }
    }
    Ok(())
}

fn publish_failed(
    output: BuildOutputCommitContext,
    publication: Option<ManifestPublicationContext>,
    package: Option<&ValidatedParsedPackage>,
    admitted: Option<&AdmittedResourceLedger>,
    pagination: Option<&typaxis_pagination::PaginationResult>,
) -> Result<(), Failure> {
    let Some(publication) = publication else {
        return Ok(());
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
    output
        .commit_prepared_failed(prepared)
        .map_err(map_failed_commit_error)?;
    Ok(())
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

fn map_execution_setup_error(error: BuildExecutionError) -> Failure {
    match error {
        BuildExecutionError::EmptyOutput | BuildExecutionError::AliasedWriteTarget => {
            Failure::usage(format!("invalid build write targets: {error:?}"))
        }
        BuildExecutionError::CurrentDirectoryUnavailable => {
            Failure::io("cannot resolve build write targets")
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
        PdfSinkCommitError::Execution(source) => Failure::io(format!(
            "PDF targets changed before publication: {source:?}"
        )),
        PdfSinkCommitError::PublishedButDurabilityUncertain { source, .. } => Failure::io(format!(
            "PDF was published but directory synchronization failed: {source}"
        )),
        other => Failure::internal(format!("PDF publication invariant failed: {other:?}")),
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
            "Typaxis reference typesetting CLI\n\nUSAGE:\n  {program} <COMMAND> [OPTIONS]\n\nCOMMANDS:\n  build         Build a PDF\n  check         Validate an input source\n  dump-ast      Write the parsed package as JSON\n  dump-layout   Write one physical page layout as JSON\n  inspect-font  Inspect an SFNT font\n  list-fonts    List SFNT fonts in a directory\n\nRun `{program} help <COMMAND>` for command usage.\n"
        ),
        Some("build") => format!(
            "USAGE:\n  {program} build INPUT -o OUTPUT [--trace PATH] [--emit-build-manifest PATH] [OPTIONS]\n\nOPTIONS:\n  --config PATH             Use a project config\n  --resource-root DIR       Add an ordered host resource root (repeatable)\n  --strict                  Reject pagination fallback\n  --trace PATH              Atomically write a layout trace\n  --trace-text              Include opted-in trace text; requires --trace\n  --emit-build-manifest P   Atomically write a terminal build manifest\n  --no-compress             Disable PDF stream compression\n  --force                   Atomically replace existing targets\n  --max-<name> N            Override a resource limit\n"
        ),
        Some("check") => format!("USAGE:\n  {program} check INPUT [OPTIONS]\n"),
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
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn exit_codes_match_the_cli_contract() {
        assert_eq!(FailureKind::Input.exit_code(), 1);
        assert_eq!(FailureKind::Usage.exit_code(), 2);
        assert_eq!(FailureKind::Io.exit_code(), 3);
        assert_eq!(FailureKind::Internal.exit_code(), 4);
        assert_eq!(FailureKind::Limit.exit_code(), 5);
    }

    #[test]
    fn exact_dash_is_left_for_the_execution_context() {
        let output = std::ffi::OsString::from("-");
        let execution =
            BuildExecutionContext::from_cli_token(&output, None, None, ReplacePolicy::NoReplace)
                .unwrap();
        assert!(execution.output_path().is_none());
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
