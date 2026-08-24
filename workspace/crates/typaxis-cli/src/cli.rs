use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

pub const COMMANDS: &[&str] = &[
    "build",
    "check",
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceOptions {
    pub input: PathBuf,
    pub common: CommonOptions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Build(BuildOptions),
    Check(SourceOptions),
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
    Run(Command),
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
        "build" => parse_build(args).map(Command::Build).map(Invocation::Run),
        "check" => parse_source(args, SourceCommand::Check)
            .map(Command::Check)
            .map(Invocation::Run),
        "dump-ast" => parse_source(args, SourceCommand::DumpAst)
            .map(Command::DumpAst)
            .map(Invocation::Run),
        "dump-layout" => parse_dump_layout(args).map(Invocation::Run),
        "inspect-font" => parse_inspect_font(args).map(Invocation::Run),
        "list-fonts" => parse_list_fonts(args).map(Invocation::Run),
        unknown => Err(UsageError(format!("unknown command `{unknown}`"))),
    }
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
        let Invocation::Run(Command::Build(build)) = parse(strings(&[
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
            Invocation::Run(Command::DumpLayout {
                physical_page: 1,
                ..
            })
        ));
    }

    #[test]
    fn dump_ast_requires_the_documented_format() {
        assert!(parse(strings(&["dump-ast", "input.tsf"])).is_err());
        assert!(matches!(
            parse(strings(&["dump-ast", "input.tsf", "--format", "json"])),
            Ok(Invocation::Run(Command::DumpAst(_)))
        ));
        assert!(parse(strings(&["dump-ast", "input.tsf", "--format", "yaml"])).is_err());
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
        let Invocation::Run(Command::Build(build)) = invocation else {
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
        let Invocation::Run(Command::InspectFont { font: parsed_font }) =
            parse(vec![OsString::from("inspect-font"), font.clone()]).unwrap()
        else {
            panic!("expected inspect-font invocation");
        };
        assert_eq!(parsed_font.as_os_str().as_bytes(), font.as_bytes());

        let font_dir = OsString::from_vec(b"fonts-\xf8".to_vec());
        let Invocation::Run(Command::ListFonts {
            font_dir: parsed_font_dir,
        }) = parse(vec![
            OsString::from("list-fonts"),
            OsString::from("--font-dir"),
            font_dir.clone(),
        ])
        .unwrap()
        else {
            panic!("expected list-fonts invocation");
        };
        assert_eq!(parsed_font_dir.as_os_str().as_bytes(), font_dir.as_bytes());
    }
}
