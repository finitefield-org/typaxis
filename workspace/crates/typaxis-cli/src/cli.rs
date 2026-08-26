use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::str::FromStr;

use typaxis_core::MachinePdfProfileId;

pub const COMMANDS: &[&str] = &[
    "build",
    "build-package",
    "capabilities",
    "check",
    "check-package",
    "dump-ast",
    "dump-layout",
    "inspect-font",
    "list-fonts",
];

pub const LIMIT_OPTIONS: &[&str] = &[
    "max-input-bytes",
    "max-source-bytes",
    "max-include-depth",
    "max-include-files",
    "max-ast-nesting-depth",
    "max-ast-nodes",
    "max-style-rules",
    "max-text-bytes",
    "max-text-buffer-bytes",
    "max-shaping-context-bytes",
    "max-font-bytes",
    "max-fonts",
    "max-image-bytes",
    "max-images",
    "max-resource-bytes",
    "max-image-pixels",
    "max-decoded-image-bytes",
    "max-document-package-bytes",
    "max-json-nesting-depth",
    "max-pages",
    "max-layout-passes",
    "max-uri-bytes",
    "max-line-reshape-passes",
    "max-page-break-lookback",
    "max-footnote-reflows-per-page",
    "max-column-balance-candidates",
    "max-float-queue",
    "max-float-carry-pages",
    "max-cids-per-font",
    "max-fragments",
    "max-spool-bytes",
    "max-pdf-objects",
    "max-output-bytes",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommonOptions {
    pub config: Option<PathBuf>,
    pub resource_roots: Vec<PathBuf>,
    pub strict: bool,
    pub no_compress: bool,
    pub limits: Vec<(String, u64)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildOptions {
    pub input: PathBuf,
    pub output: OsString,
    pub trace: Option<PathBuf>,
    pub trace_text: bool,
    pub manifest: Option<PathBuf>,
    pub force: bool,
    pub common: CommonOptions,
}

/// Grammar for a DocumentPackage PDF build. This remains separate
/// from [`BuildOptions`] so source and package commands cannot accidentally
/// share an input loader or silently ignore command-specific fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BuildPackageOptions {
    pub package: PathBuf,
    pub package_root: Option<PathBuf>,
    pub profile: MachinePdfProfileId,
    pub output: OsString,
    pub trace: Option<PathBuf>,
    pub trace_text: bool,
    pub manifest: Option<PathBuf>,
    pub diagnostics: Option<PathBuf>,
    pub force: bool,
    pub common: CommonOptions,
}

/// Grammar for validation through style/font-family preparation.
/// It intentionally has no output, layout, trace, compression, or replace
/// fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CheckPackageOptions {
    pub package: PathBuf,
    pub package_root: Option<PathBuf>,
    pub profile: MachinePdfProfileId,
    pub diagnostics: Option<PathBuf>,
    pub common: CommonOptions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CapabilitiesFormat {
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CapabilitiesOptions {
    pub format: CapabilitiesFormat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceOptions {
    pub input: PathBuf,
    pub common: CommonOptions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Build(BuildOptions),
    BuildPackage(BuildPackageOptions),
    Capabilities(CapabilitiesOptions),
    Check(SourceOptions),
    CheckPackage(CheckPackageOptions),
    DumpAst(SourceOptions),
    DumpLayout {
        source: SourceOptions,
        physical_page: u32,
    },
    InspectFont {
        font: PathBuf,
    },
    ListFonts {
        font_dir: PathBuf,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Invocation {
    Help(Option<String>),
    Version,
    Run(Box<Command>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsageError(pub String);

impl std::fmt::Display for UsageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Invocation, UsageError> {
    let mut args: Vec<OsString> = args.into_iter().collect();
    if args.is_empty() {
        return Ok(Invocation::Help(None));
    }
    let command = args.remove(0);
    let Some(command) = command.to_str() else {
        return Err(UsageError("command name is not valid UTF-8".to_owned()));
    };
    match command {
        "--help" | "-h" => no_extra(args, Invocation::Help(None)),
        "--version" | "-V" | "version" => no_extra(args, Invocation::Version),
        "help" => match args.as_slice() {
            [] => Ok(Invocation::Help(None)),
            [name] if name.to_str().is_some_and(|name| COMMANDS.contains(&name)) => {
                Ok(Invocation::Help(name.to_str().map(str::to_owned)))
            }
            [name] => Err(UsageError(format!(
                "unknown command `{}`",
                name.to_string_lossy()
            ))),
            _ => Err(UsageError("`help` accepts at most one command".to_owned())),
        },
        command
            if COMMANDS.contains(&command)
                && matches!(args.as_slice(), [value] if is_help(value)) =>
        {
            Ok(Invocation::Help(Some(command.to_owned())))
        }
        "build" => parse_build(args).map(Command::Build).map(run_invocation),
        "build-package" => parse_build_package(args)
            .map(Command::BuildPackage)
            .map(run_invocation),
        "capabilities" => parse_capabilities(args)
            .map(Command::Capabilities)
            .map(run_invocation),
        "check" => parse_source(args, SourceCommand::Check)
            .map(Command::Check)
            .map(run_invocation),
        "check-package" => parse_check_package(args)
            .map(Command::CheckPackage)
            .map(run_invocation),
        "dump-ast" => parse_source(args, SourceCommand::DumpAst)
            .map(Command::DumpAst)
            .map(run_invocation),
        "dump-layout" => parse_dump_layout(args).map(run_invocation),
        "inspect-font" => parse_inspect_font(args).map(run_invocation),
        "list-fonts" => parse_list_fonts(args).map(run_invocation),
        unknown => Err(UsageError(format!("unknown command `{unknown}`"))),
    }
}

fn run_invocation(command: Command) -> Invocation {
    Invocation::Run(Box::new(command))
}

fn no_extra(args: Vec<OsString>, invocation: Invocation) -> Result<Invocation, UsageError> {
    if args.is_empty() {
        Ok(invocation)
    } else {
        Err(UsageError(
            "unexpected argument after global option".to_owned(),
        ))
    }
}

fn is_help(value: &OsStr) -> bool {
    value == OsStr::new("--help") || value == OsStr::new("-h")
}

#[derive(Clone, Copy)]
enum SourceCommand {
    Check,
    DumpAst,
}

fn parse_source(args: Vec<OsString>, command: SourceCommand) -> Result<SourceOptions, UsageError> {
    let mut common = CommonOptions::default();
    let mut input = None;
    let mut format_seen = false;
    let mut options = true;
    let mut index = 0;
    while index < args.len() {
        if options && args[index] == OsStr::new("--") {
            options = false;
            index += 1;
            continue;
        }
        if options {
            if let Some(consumed) = parse_common(&args, index, &mut common)? {
                index += consumed;
                continue;
            }
            if matches!(command, SourceCommand::DumpAst)
                && option_name(&args[index]) == Some("--format")
            {
                if format_seen {
                    return Err(UsageError(
                        "`--format` may only be specified once".to_owned(),
                    ));
                }
                let (value, consumed) = option_value(&args, index, "--format")?;
                if value != OsStr::new("json") {
                    return Err(UsageError("`--format` only accepts `json`".to_owned()));
                }
                format_seen = true;
                index += consumed;
                continue;
            }
            if looks_like_option(&args[index]) {
                return Err(unknown_option(&args[index]));
            }
        }
        set_positional(&mut input, &args[index], "INPUT")?;
        index += 1;
    }
    if matches!(command, SourceCommand::DumpAst) && !format_seen {
        return Err(UsageError("missing required `--format json`".to_owned()));
    }
    Ok(SourceOptions {
        input: input.ok_or_else(|| UsageError("missing INPUT".to_owned()))?,
        common,
    })
}

fn parse_build(args: Vec<OsString>) -> Result<BuildOptions, UsageError> {
    let mut common = CommonOptions::default();
    let mut input = None;
    let mut output = None;
    let mut trace = None;
    let mut manifest = None;
    let mut trace_text = false;
    let mut force = false;
    let mut options = true;
    let mut index = 0;
    while index < args.len() {
        if options && args[index] == OsStr::new("--") {
            options = false;
            index += 1;
            continue;
        }
        if options {
            if let Some(consumed) = parse_common(&args, index, &mut common)? {
                index += consumed;
                continue;
            }
            match option_name(&args[index]) {
                Some("-o" | "--output") => {
                    let (value, consumed) = option_value_alias(&args, index, "-o", "--output")?;
                    set_once(&mut output, value.to_owned(), "output")?;
                    index += consumed;
                    continue;
                }
                Some("--trace") => {
                    let (value, consumed) = option_value(&args, index, "--trace")?;
                    set_once(&mut trace, PathBuf::from(value), "trace")?;
                    index += consumed;
                    continue;
                }
                Some("--emit-build-manifest") => {
                    let (value, consumed) = option_value(&args, index, "--emit-build-manifest")?;
                    set_once(&mut manifest, PathBuf::from(value), "build manifest")?;
                    index += consumed;
                    continue;
                }
                Some("--trace-text") => {
                    reject_attached_flag_value(&args[index], "--trace-text")?;
                    set_flag(&mut trace_text, "--trace-text")?;
                    index += 1;
                    continue;
                }
                Some("--force") => {
                    reject_attached_flag_value(&args[index], "--force")?;
                    set_flag(&mut force, "--force")?;
                    index += 1;
                    continue;
                }
                _ => {}
            }
            if looks_like_option(&args[index]) {
                return Err(unknown_option(&args[index]));
            }
        }
        set_positional(&mut input, &args[index], "INPUT")?;
        index += 1;
    }
    if trace_text && trace.is_none() {
        return Err(UsageError(
            "`--trace-text` requires `--trace PATH`".to_owned(),
        ));
    }
    Ok(BuildOptions {
        input: input.ok_or_else(|| UsageError("missing INPUT".to_owned()))?,
        output: output.ok_or_else(|| UsageError("missing required `-o OUTPUT`".to_owned()))?,
        trace,
        trace_text,
        manifest,
        force,
        common,
    })
}

/// Parse the public `build-package` grammar.
pub(crate) fn parse_build_package(args: Vec<OsString>) -> Result<BuildPackageOptions, UsageError> {
    let mut common = CommonOptions::default();
    let mut package = None;
    let mut package_root = None;
    let mut profile = None;
    let mut output = None;
    let mut trace = None;
    let mut manifest = None;
    let mut diagnostics = None;
    let mut trace_text = false;
    let mut force = false;
    let mut options = true;
    let mut index = 0;
    while index < args.len() {
        if options && args[index] == OsStr::new("--") {
            options = false;
            index += 1;
            continue;
        }
        if options {
            if let Some(consumed) = parse_common(&args, index, &mut common)? {
                index += consumed;
                continue;
            }
            match option_name(&args[index]) {
                Some("-o" | "--output") => {
                    let (value, consumed) = option_value_alias(&args, index, "-o", "--output")?;
                    set_once(&mut output, value.to_owned(), "output")?;
                    index += consumed;
                    continue;
                }
                Some("--package-root") => {
                    let (value, consumed) = option_value(&args, index, "--package-root")?;
                    set_once(
                        &mut package_root,
                        nonempty_path(value, "package-root")?,
                        "package-root",
                    )?;
                    index += consumed;
                    continue;
                }
                Some("--profile") => {
                    let (value, consumed) = option_value(&args, index, "--profile")?;
                    set_once(&mut profile, parse_machine_profile(value)?, "profile")?;
                    index += consumed;
                    continue;
                }
                Some("--trace") => {
                    let (value, consumed) = option_value(&args, index, "--trace")?;
                    set_once(&mut trace, nonempty_path(value, "trace")?, "trace")?;
                    index += consumed;
                    continue;
                }
                Some("--emit-build-manifest") => {
                    let (value, consumed) = option_value(&args, index, "--emit-build-manifest")?;
                    set_once(
                        &mut manifest,
                        nonempty_path(value, "build manifest")?,
                        "build manifest",
                    )?;
                    index += consumed;
                    continue;
                }
                Some("--emit-diagnostics") => {
                    let (value, consumed) = option_value(&args, index, "--emit-diagnostics")?;
                    set_once(
                        &mut diagnostics,
                        nonempty_path(value, "diagnostics")?,
                        "diagnostics",
                    )?;
                    index += consumed;
                    continue;
                }
                Some("--trace-text") => {
                    reject_attached_flag_value(&args[index], "--trace-text")?;
                    set_flag(&mut trace_text, "--trace-text")?;
                    index += 1;
                    continue;
                }
                Some("--force") => {
                    reject_attached_flag_value(&args[index], "--force")?;
                    set_flag(&mut force, "--force")?;
                    index += 1;
                    continue;
                }
                _ => {}
            }
            if looks_like_option(&args[index]) {
                return Err(unknown_option(&args[index]));
            }
        }
        set_positional(&mut package, &args[index], "PACKAGE")?;
        index += 1;
    }
    if trace_text && trace.is_none() {
        return Err(UsageError(
            "`--trace-text` requires `--trace PATH`".to_owned(),
        ));
    }
    Ok(BuildPackageOptions {
        package: package.ok_or_else(|| UsageError("missing PACKAGE".to_owned()))?,
        package_root,
        profile: profile.unwrap_or(MachinePdfProfileId::CURRENT),
        output: output.ok_or_else(|| UsageError("missing required `-o OUTPUT`".to_owned()))?,
        trace,
        trace_text,
        manifest,
        diagnostics,
        force,
        common,
    })
}

/// Parse the public `check-package` grammar. Layout-only
/// flags are unknown here by construction instead of being accepted and
/// ignored.
pub(crate) fn parse_check_package(args: Vec<OsString>) -> Result<CheckPackageOptions, UsageError> {
    let mut common = CommonOptions::default();
    let mut package = None;
    let mut package_root = None;
    let mut profile = None;
    let mut diagnostics = None;
    let mut options = true;
    let mut index = 0;
    while index < args.len() {
        if options && args[index] == OsStr::new("--") {
            options = false;
            index += 1;
            continue;
        }
        if options {
            if let Some(consumed) = parse_machine_check_common(&args, index, &mut common)? {
                index += consumed;
                continue;
            }
            match option_name(&args[index]) {
                Some("--package-root") => {
                    let (value, consumed) = option_value(&args, index, "--package-root")?;
                    set_once(
                        &mut package_root,
                        nonempty_path(value, "package-root")?,
                        "package-root",
                    )?;
                    index += consumed;
                    continue;
                }
                Some("--profile") => {
                    let (value, consumed) = option_value(&args, index, "--profile")?;
                    set_once(&mut profile, parse_machine_profile(value)?, "profile")?;
                    index += consumed;
                    continue;
                }
                Some("--emit-diagnostics") => {
                    let (value, consumed) = option_value(&args, index, "--emit-diagnostics")?;
                    set_once(
                        &mut diagnostics,
                        nonempty_path(value, "diagnostics")?,
                        "diagnostics",
                    )?;
                    index += consumed;
                    continue;
                }
                _ => {}
            }
            if looks_like_option(&args[index]) {
                return Err(unknown_option(&args[index]));
            }
        }
        set_positional(&mut package, &args[index], "PACKAGE")?;
        index += 1;
    }
    Ok(CheckPackageOptions {
        package: package.ok_or_else(|| UsageError("missing PACKAGE".to_owned()))?,
        package_root,
        profile: profile.unwrap_or(MachinePdfProfileId::CURRENT),
        diagnostics,
        common,
    })
}

/// Parse the public capability artifact grammar.
pub(crate) fn parse_capabilities(args: Vec<OsString>) -> Result<CapabilitiesOptions, UsageError> {
    let mut format = None;
    let mut index = 0;
    while index < args.len() {
        if option_name(&args[index]) != Some("--format") {
            return Err(if looks_like_option(&args[index]) {
                unknown_option(&args[index])
            } else {
                UsageError("unexpected positional argument".to_owned())
            });
        }
        let (value, consumed) = option_value(&args, index, "--format")?;
        if value != OsStr::new("json") {
            return Err(UsageError("`--format` only accepts `json`".to_owned()));
        }
        set_once(&mut format, CapabilitiesFormat::Json, "format")?;
        index += consumed;
    }
    Ok(CapabilitiesOptions {
        format: format.ok_or_else(|| UsageError("missing required `--format json`".to_owned()))?,
    })
}

fn parse_machine_check_common(
    args: &[OsString],
    index: usize,
    common: &mut CommonOptions,
) -> Result<Option<usize>, UsageError> {
    match option_name(&args[index]) {
        Some("--config") => {
            let (value, consumed) = option_value(args, index, "--config")?;
            set_once(
                &mut common.config,
                nonempty_path(value, "config")?,
                "config",
            )?;
            Ok(Some(consumed))
        }
        Some("--resource-root") => {
            let (value, consumed) = option_value(args, index, "--resource-root")?;
            common
                .resource_roots
                .push(nonempty_path(value, "resource-root")?);
            Ok(Some(consumed))
        }
        Some(name) => {
            let normalized = name.strip_prefix("--").unwrap_or(name);
            if LIMIT_OPTIONS.contains(&normalized) {
                let (value, consumed) = option_value(args, index, name)?;
                let value = parse_positive_u64(value, name)?;
                if common.limits.iter().any(|(found, _)| found == normalized) {
                    return Err(UsageError(format!(
                        "`--{normalized}` specified more than once"
                    )));
                }
                common.limits.push((normalized.to_owned(), value));
                Ok(Some(consumed))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

fn parse_machine_profile(value: &OsStr) -> Result<MachinePdfProfileId, UsageError> {
    let value = value
        .to_str()
        .ok_or_else(|| UsageError("`--profile` value is not valid UTF-8".to_owned()))?;
    MachinePdfProfileId::from_str(value)
        .map_err(|_| UsageError(format!("unknown machine PDF profile `{value}`")))
}

fn nonempty_path(value: &OsStr, name: &str) -> Result<PathBuf, UsageError> {
    if value.is_empty() {
        Err(UsageError(format!("{name} path must not be empty")))
    } else {
        Ok(PathBuf::from(value))
    }
}

fn parse_dump_layout(args: Vec<OsString>) -> Result<Command, UsageError> {
    let mut common = CommonOptions::default();
    let mut input = None;
    let mut physical_page = None;
    let mut options = true;
    let mut index = 0;
    while index < args.len() {
        if options && args[index] == OsStr::new("--") {
            options = false;
            index += 1;
            continue;
        }
        if options {
            if let Some(consumed) = parse_common(&args, index, &mut common)? {
                index += consumed;
                continue;
            }
            if option_name(&args[index]) == Some("--page") {
                let (value, consumed) = option_value(&args, index, "--page")?;
                let value = parse_positive_u32(value, "--page")?;
                set_once(&mut physical_page, value, "page")?;
                index += consumed;
                continue;
            }
            if looks_like_option(&args[index]) {
                return Err(unknown_option(&args[index]));
            }
        }
        set_positional(&mut input, &args[index], "INPUT")?;
        index += 1;
    }
    Ok(Command::DumpLayout {
        source: SourceOptions {
            input: input.ok_or_else(|| UsageError("missing INPUT".to_owned()))?,
            common,
        },
        physical_page: physical_page
            .ok_or_else(|| UsageError("missing required `--page N`".to_owned()))?,
    })
}

fn parse_inspect_font(args: Vec<OsString>) -> Result<Command, UsageError> {
    let font = one_positional(args, "FONT")?;
    Ok(Command::InspectFont { font })
}

fn parse_list_fonts(args: Vec<OsString>) -> Result<Command, UsageError> {
    let mut font_dir = None;
    let mut index = 0;
    while index < args.len() {
        if option_name(&args[index]) == Some("--font-dir") {
            let (value, consumed) = option_value(&args, index, "--font-dir")?;
            set_once(&mut font_dir, PathBuf::from(value), "font directory")?;
            index += consumed;
        } else {
            return Err(if looks_like_option(&args[index]) {
                unknown_option(&args[index])
            } else {
                UsageError("unexpected positional argument".to_owned())
            });
        }
    }
    Ok(Command::ListFonts {
        font_dir: font_dir
            .ok_or_else(|| UsageError("missing required `--font-dir DIR`".to_owned()))?,
    })
}

fn one_positional(args: Vec<OsString>, name: &str) -> Result<PathBuf, UsageError> {
    let mut positional = None;
    let mut options = true;
    for arg in args {
        if options && arg == OsStr::new("--") {
            options = false;
        } else if options && looks_like_option(&arg) {
            return Err(unknown_option(&arg));
        } else {
            set_positional(&mut positional, &arg, name)?;
        }
    }
    positional.ok_or_else(|| UsageError(format!("missing {name}")))
}

fn parse_common(
    args: &[OsString],
    index: usize,
    common: &mut CommonOptions,
) -> Result<Option<usize>, UsageError> {
    let name = option_name(&args[index]);
    match name {
        Some("--config") => {
            let (value, consumed) = option_value(args, index, "--config")?;
            set_once(&mut common.config, PathBuf::from(value), "config")?;
            Ok(Some(consumed))
        }
        Some("--resource-root") => {
            let (value, consumed) = option_value(args, index, "--resource-root")?;
            common.resource_roots.push(PathBuf::from(value));
            Ok(Some(consumed))
        }
        Some("--strict") => {
            reject_attached_flag_value(&args[index], "--strict")?;
            set_flag(&mut common.strict, "--strict")?;
            Ok(Some(1))
        }
        Some("--no-compress") => {
            reject_attached_flag_value(&args[index], "--no-compress")?;
            set_flag(&mut common.no_compress, "--no-compress")?;
            Ok(Some(1))
        }
        Some(name) => {
            let normalized = name.strip_prefix("--").unwrap_or(name);
            if LIMIT_OPTIONS.contains(&normalized) {
                let (value, consumed) = option_value(args, index, name)?;
                let value = parse_positive_u64(value, name)?;
                if common.limits.iter().any(|(found, _)| found == normalized) {
                    return Err(UsageError(format!(
                        "`--{normalized}` specified more than once"
                    )));
                }
                common.limits.push((normalized.to_owned(), value));
                Ok(Some(consumed))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

fn option_name(value: &OsStr) -> Option<&str> {
    let value = value.to_str()?;
    value
        .split_once('=')
        .map_or(Some(value), |(name, _)| Some(name))
}

fn option_value<'a>(
    args: &'a [OsString],
    index: usize,
    expected: &str,
) -> Result<(&'a OsStr, usize), UsageError> {
    option_value_alias(args, index, expected, expected)
}

fn option_value_alias<'a>(
    args: &'a [OsString],
    index: usize,
    first: &str,
    second: &str,
) -> Result<(&'a OsStr, usize), UsageError> {
    if let Some(value) = args[index].to_str() {
        if let Some((name, value)) = value.split_once('=') {
            if name == first || name == second {
                if value.is_empty() {
                    return Err(UsageError(format!("`{name}` requires a value")));
                }
                return Ok((OsStr::new(value), 1));
            }
        }
    }
    let value = args
        .get(index + 1)
        .ok_or_else(|| UsageError(format!("`{first}` requires a value")))?;
    if value.is_empty() {
        return Err(UsageError(format!("`{first}` requires a value")));
    }
    Ok((value, 2))
}

fn looks_like_option(value: &OsStr) -> bool {
    value.as_encoded_bytes().first() == Some(&b'-')
}

fn unknown_option(value: &OsStr) -> UsageError {
    UsageError(format!("unknown option `{}`", value.to_string_lossy()))
}

fn reject_attached_flag_value(value: &OsStr, name: &str) -> Result<(), UsageError> {
    if value == OsStr::new(name) {
        Ok(())
    } else {
        Err(UsageError(format!("`{name}` does not accept a value")))
    }
}

fn set_positional(slot: &mut Option<PathBuf>, value: &OsStr, name: &str) -> Result<(), UsageError> {
    if value.is_empty() {
        return Err(UsageError(format!("{name} path must not be empty")));
    }
    if slot.replace(PathBuf::from(value)).is_some() {
        Err(UsageError(format!(
            "too many positional arguments; expected one {name}"
        )))
    } else {
        Ok(())
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, name: &str) -> Result<(), UsageError> {
    if slot.replace(value).is_some() {
        Err(UsageError(format!("{name} specified more than once")))
    } else {
        Ok(())
    }
}

fn set_flag(slot: &mut bool, name: &str) -> Result<(), UsageError> {
    if *slot {
        Err(UsageError(format!("`{name}` specified more than once")))
    } else {
        *slot = true;
        Ok(())
    }
}

fn parse_positive_u64(value: &OsStr, option: &str) -> Result<u64, UsageError> {
    let value = value
        .to_str()
        .ok_or_else(|| UsageError(format!("`{option}` value is not valid UTF-8")))?;
    let parsed = value
        .parse::<u64>()
        .map_err(|_| UsageError(format!("`{option}` requires a positive decimal integer")))?;
    if parsed == 0 {
        Err(UsageError(format!(
            "`{option}` requires a positive decimal integer"
        )))
    } else {
        Ok(parsed)
    }
}

fn parse_positive_u32(value: &OsStr, option: &str) -> Result<u32, UsageError> {
    let parsed = parse_positive_u64(value, option)?;
    u32::try_from(parsed)
        .map_err(|_| UsageError(format!("`{option}` value exceeds the supported range")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn no_arguments_and_global_flags_are_successful_actions() {
        assert_eq!(parse(Vec::<OsString>::new()), Ok(Invocation::Help(None)));
        assert_eq!(parse(strings(&["--version"])), Ok(Invocation::Version));
        assert_eq!(
            parse(strings(&["help", "build"])),
            Ok(Invocation::Help(Some("build".to_owned())))
        );
    }

    #[test]
    fn build_parses_host_paths_and_exact_stdout_token() {
        let Invocation::Run(command) = parse(strings(&[
            "build",
            "input.tsf",
            "-o",
            "-",
            "--resource-root",
            "one",
            "--resource-root=two",
            "--trace",
            "trace.json",
            "--trace-text",
            "--max-fonts",
            "12",
            "--force",
        ]))
        .unwrap() else {
            panic!("expected build command");
        };
        let Command::Build(build) = *command else {
            panic!("expected build command");
        };
        assert_eq!(build.output, OsString::from("-"));
        assert_eq!(build.common.resource_roots.len(), 2);
        assert_eq!(build.common.limits, [("max-fonts".to_owned(), 12)]);
        assert!(build.trace_text);
        assert!(build.force);
    }

    #[test]
    fn trace_text_requires_a_trace_file() {
        let error = parse(strings(&[
            "build",
            "input.tsf",
            "-o",
            "output.pdf",
            "--trace-text",
        ]))
        .unwrap_err();
        assert!(error.0.contains("requires `--trace"));
    }

    #[test]
    fn dump_page_is_positive_and_checked() {
        assert!(parse(strings(&["dump-layout", "input.tsf", "--page", "0"])).is_err());
        assert!(parse(strings(&[
            "dump-layout",
            "input.tsf",
            "--page",
            "4294967296"
        ]))
        .is_err());
        let command = parse(strings(&["dump-layout", "input.tsf", "--page=1"])).unwrap();
        assert!(matches!(
            command,
            Invocation::Run(command)
                if matches!(*command, Command::DumpLayout { physical_page: 1, .. })
        ));
    }

    #[test]
    fn dump_ast_requires_the_documented_format() {
        assert!(parse(strings(&["dump-ast", "input.tsf"])).is_err());
        assert!(matches!(
            parse(strings(&["dump-ast", "input.tsf", "--format", "json"])),
            Ok(Invocation::Run(command)) if matches!(*command, Command::DumpAst(_))
        ));
        assert!(parse(strings(&["dump-ast", "input.tsf", "--format", "yaml"])).is_err());
    }

    #[test]
    fn build_package_parser_accepts_the_complete_machine_grammar() {
        let mut args = strings(&[
            "job/document-package.json",
            "-o",
            "output.pdf",
            "--package-root",
            "job",
            "--profile",
            "typaxis.machine-pdf/paragraph-1",
            "--config",
            "machine.toml",
            "--resource-root",
            "fonts-a",
            "--resource-root=fonts-b",
            "--strict",
            "--no-compress",
            "--trace",
            "trace.json",
            "--trace-text",
            "--emit-build-manifest",
            "manifest.json",
            "--emit-diagnostics",
            "diagnostics.json",
            "--force",
        ]);
        for option in LIMIT_OPTIONS {
            args.push(OsString::from(format!("--{option}")));
            args.push(OsString::from("1"));
        }
        let parsed = parse_build_package(args).unwrap();
        assert_eq!(parsed.package, PathBuf::from("job/document-package.json"));
        assert_eq!(parsed.package_root, Some(PathBuf::from("job")));
        assert_eq!(parsed.profile, MachinePdfProfileId::PARAGRAPH_1);
        assert_eq!(parsed.output, OsString::from("output.pdf"));
        assert_eq!(parsed.trace, Some(PathBuf::from("trace.json")));
        assert!(parsed.trace_text);
        assert_eq!(parsed.manifest, Some(PathBuf::from("manifest.json")));
        assert_eq!(parsed.diagnostics, Some(PathBuf::from("diagnostics.json")));
        assert!(parsed.force);
        assert!(parsed.common.strict);
        assert!(parsed.common.no_compress);
        assert_eq!(parsed.common.resource_roots.len(), 2);
        assert_eq!(parsed.common.limits.len(), LIMIT_OPTIONS.len());
    }

    #[test]
    fn machine_parsers_resolve_only_the_closed_profile() {
        let build = parse_build_package(strings(&["package.json", "-o", "out.pdf"])).unwrap();
        assert_eq!(build.profile, MachinePdfProfileId::PARAGRAPH_1);
        let check = parse_check_package(strings(&["package.json"])).unwrap();
        assert_eq!(check.profile, MachinePdfProfileId::PARAGRAPH_1);

        for parse_error in [
            parse_build_package(strings(&[
                "package.json",
                "-o",
                "out.pdf",
                "--profile",
                "typaxis.machine-pdf/future",
            ]))
            .unwrap_err(),
            parse_check_package(strings(&[
                "package.json",
                "--profile",
                "typaxis.machine-pdf/future",
            ]))
            .unwrap_err(),
        ] {
            assert!(parse_error.0.contains("unknown machine PDF profile"));
        }
    }

    #[test]
    fn check_package_rejects_every_build_only_flag() {
        let forbidden = [
            ("-o", Some("out.pdf")),
            ("--output", Some("out.pdf")),
            ("--strict", None),
            ("--no-compress", None),
            ("--trace", Some("trace.json")),
            ("--trace-text", None),
            ("--emit-build-manifest", Some("manifest.json")),
            ("--force", None),
        ];
        for (flag, value) in forbidden {
            let mut args = strings(&["package.json", flag]);
            if let Some(value) = value {
                args.push(OsString::from(value));
            }
            let error = parse_check_package(args).unwrap_err();
            assert!(error.0.contains("unknown option"), "{flag}: {error}");
        }

        let parsed = parse_check_package(strings(&[
            "package.json",
            "--package-root",
            "job",
            "--config",
            "machine.toml",
            "--resource-root",
            "fonts",
            "--max-fonts",
            "2",
            "--emit-diagnostics",
            "diagnostics.json",
        ]))
        .unwrap();
        assert_eq!(parsed.package_root, Some(PathBuf::from("job")));
        assert_eq!(parsed.common.limits, [("max-fonts".to_owned(), 2)]);
        assert_eq!(parsed.diagnostics, Some(PathBuf::from("diagnostics.json")));
        assert!(!parsed.common.strict);
        assert!(!parsed.common.no_compress);
    }

    #[test]
    fn capabilities_parser_requires_exact_json_format() {
        assert_eq!(
            parse_capabilities(strings(&["--format", "json"])),
            Ok(CapabilitiesOptions {
                format: CapabilitiesFormat::Json
            })
        );
        assert!(parse_capabilities(Vec::new()).is_err());
        assert!(parse_capabilities(strings(&["--format", "yaml"])).is_err());
        assert!(parse_capabilities(strings(&["--format", "json", "extra"])).is_err());
        assert!(parse_capabilities(strings(&["--format", "json", "--format", "json"])).is_err());
    }

    #[test]
    fn machine_commands_are_registered_with_their_exact_grammars() {
        for command in ["build-package", "check-package", "capabilities"] {
            assert!(COMMANDS.contains(&command));
            assert_eq!(
                parse(strings(&["help", command])),
                Ok(Invocation::Help(Some(command.to_owned())))
            );
        }
        assert!(matches!(
            parse(strings(&["build-package", "package.json", "-o", "out.pdf"])),
            Ok(Invocation::Run(command)) if matches!(*command, Command::BuildPackage(_))
        ));
        assert!(matches!(
            parse(strings(&["check-package", "package.json"])),
            Ok(Invocation::Run(command)) if matches!(*command, Command::CheckPackage(_))
        ));
        assert!(matches!(
            parse(strings(&["capabilities", "--format", "json"])),
            Ok(Invocation::Run(command)) if matches!(*command, Command::Capabilities(_))
        ));
    }

    #[test]
    fn unknown_options_and_extra_positionals_are_rejected() {
        assert!(parse(strings(&["check", "input.tsf", "--wat"])).is_err());
        assert!(parse(strings(&["inspect-font", "a.ttf", "b.ttf"])).is_err());
        assert!(parse(strings(&["inspect-font", ""])).is_err());
        assert!(parse(strings(&["check", ""])).is_err());
        assert!(parse(strings(&["unknown"])).is_err());
    }

    #[test]
    fn boolean_flags_reject_attached_values() {
        for flag in ["--strict=false", "--no-compress=yes"] {
            assert!(parse(strings(&["check", "input.tsf", flag])).is_err());
        }
        for flag in ["--trace-text=false", "--force=no"] {
            assert!(parse(strings(&["build", "input.tsf", "-o", "out.pdf", flag])).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn separated_host_paths_preserve_non_utf8_platform_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let input = OsString::from_vec(b"input-\xff.tsf".to_vec());
        let output = OsString::from_vec(b"output-\xfe.pdf".to_vec());
        let config = OsString::from_vec(b"config-\xfd.toml".to_vec());
        let resource_root = OsString::from_vec(b"resources-\xfc".to_vec());
        let trace = OsString::from_vec(b"trace-\xfb.json".to_vec());
        let manifest = OsString::from_vec(b"manifest-\xfa.json".to_vec());
        let invocation = parse(vec![
            OsString::from("build"),
            input.clone(),
            OsString::from("-o"),
            output.clone(),
            OsString::from("--config"),
            config.clone(),
            OsString::from("--resource-root"),
            resource_root.clone(),
            OsString::from("--trace"),
            trace.clone(),
            OsString::from("--emit-build-manifest"),
            manifest.clone(),
        ])
        .unwrap();
        let Invocation::Run(command) = invocation else {
            panic!("expected build invocation");
        };
        let Command::Build(build) = *command else {
            panic!("expected build invocation");
        };

        assert_eq!(build.input.as_os_str().as_bytes(), input.as_bytes());
        assert_eq!(build.output.as_bytes(), output.as_bytes());
        assert_eq!(
            build.common.config.unwrap().as_os_str().as_bytes(),
            config.as_bytes()
        );
        assert_eq!(
            build.common.resource_roots[0].as_os_str().as_bytes(),
            resource_root.as_bytes()
        );
        assert_eq!(
            build.trace.unwrap().as_os_str().as_bytes(),
            trace.as_bytes()
        );
        assert_eq!(
            build.manifest.unwrap().as_os_str().as_bytes(),
            manifest.as_bytes()
        );

        let font = OsString::from_vec(b"font-\xf9.ttf".to_vec());
        let Invocation::Run(command) =
            parse(vec![OsString::from("inspect-font"), font.clone()]).unwrap()
        else {
            panic!("expected inspect-font invocation");
        };
        let Command::InspectFont { font: parsed_font } = *command else {
            panic!("expected inspect-font invocation");
        };
        assert_eq!(parsed_font.as_os_str().as_bytes(), font.as_bytes());

        let font_dir = OsString::from_vec(b"fonts-\xf8".to_vec());
        let Invocation::Run(command) = parse(vec![
            OsString::from("list-fonts"),
            OsString::from("--font-dir"),
            font_dir.clone(),
        ])
        .unwrap() else {
            panic!("expected list-fonts invocation");
        };
        let Command::ListFonts {
            font_dir: parsed_font_dir,
        } = *command
        else {
            panic!("expected list-fonts invocation");
        };
        assert_eq!(parsed_font_dir.as_os_str().as_bytes(), font_dir.as_bytes());
    }
}
