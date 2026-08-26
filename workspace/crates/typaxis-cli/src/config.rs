use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

use toml::Value as TomlValue;
use typaxis_core::{
    ConfigResourceRoot, DocumentPackageContractId, EffectiveConfig, EffectiveConfigError,
    EffectiveDataVersions, PdfStreamCompression, ResourceLimits, CONTRACT,
    DEFAULT_ALLOWED_URI_SCHEMES, REGISTERED_JAPANESE_LINE_BREAK_VERSION,
    REGISTERED_UNICODE_VERSION,
};

const MAX_RAW_CONFIG_BYTES: u64 = 16 * 1024 * 1024;

/// Canonical command-line overrides. Host-only options such as `--config` and
/// `--resource-root` deliberately do not live here.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ConfigOverrides {
    pub(crate) strict: Option<bool>,
    pub(crate) no_compress: bool,
    limits: BTreeMap<String, u64>,
}

impl ConfigOverrides {
    pub(crate) fn set_limit(&mut self, name: &str, value: u64) -> Result<(), ConfigError> {
        let name = normalize_limit_name(name);
        if !LIMIT_NAMES.contains(&name.as_str()) {
            return Err(ConfigError::UnknownKey {
                origin: "command line".to_owned(),
                key: name,
            });
        }
        ensure_limit_storage_range(&name, value, "command line")?;
        self.limits.insert(name, value);
        Ok(())
    }
}

/// Load built-in defaults, an optional raw TOML file, the supplied environment,
/// and canonical CLI overrides, in that precedence order.
pub(crate) fn load<I, K, V>(
    config_path: Option<&Path>,
    environment: I,
    overrides: &ConfigOverrides,
) -> Result<EffectiveConfig, ConfigError>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<OsString>,
    V: Into<OsString>,
{
    let mut merged = MergedConfig::default();
    if let Some(path) = config_path {
        load_file(path, &mut merged)?;
    }
    apply_environment(environment, &mut merged)?;
    merged.apply_cli(overrides)?;
    merged.finish()
}

/// Convenience entry point for the real process environment.
pub(crate) fn load_from_process_env(
    config_path: Option<&Path>,
    overrides: &ConfigOverrides,
) -> Result<EffectiveConfig, ConfigError> {
    load(config_path, env::vars_os(), overrides)
}

#[derive(Debug)]
pub(crate) enum ConfigError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    InvalidFileUtf8 {
        path: PathBuf,
        valid_up_to: usize,
    },
    FileTooLarge {
        path: PathBuf,
        byte_length: u64,
    },
    InvalidEnvironmentName {
        name: OsString,
    },
    InvalidEnvironmentValue {
        key: String,
        value: OsString,
    },
    Syntax {
        origin: String,
        line: Option<usize>,
        detail: String,
    },
    UnknownKey {
        origin: String,
        key: String,
    },
    DuplicateKey {
        origin: String,
        key: String,
    },
    MissingContract {
        path: PathBuf,
    },
    ContractMismatch {
        origin: String,
        found: String,
    },
    InvalidValue {
        origin: String,
        key: String,
        detail: String,
    },
    Effective(EffectiveConfigError),
}

impl ConfigError {
    pub(crate) fn is_io(&self) -> bool {
        matches!(self, Self::Io { .. })
    }

    pub(crate) fn is_limit(&self) -> bool {
        matches!(
            self,
            Self::FileTooLarge { .. } | Self::Effective(EffectiveConfigError::ResourceLimits(_))
        )
    }

    /// Stable diagnostic category for configuration parsing and resolution.
    /// Resource-limit configuration failures retain this source/config code
    /// while their process exit class remains the distinct limit code 5.
    pub(crate) const fn diagnostic_code(&self) -> &'static str {
        "P1001"
    }

    #[cfg(test)]
    pub(crate) fn is_effective_resource_limit(&self) -> bool {
        matches!(
            self,
            Self::Effective(EffectiveConfigError::ResourceLimits(_))
        )
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "cannot read config `{}`: {source}",
                    path.display()
                )
            }
            Self::InvalidFileUtf8 { path, valid_up_to } => write!(
                formatter,
                "config `{}` is not UTF-8 (invalid byte at offset {valid_up_to})",
                path.display()
            ),
            Self::FileTooLarge { path, byte_length } => write!(
                formatter,
                "config `{}` is {byte_length} bytes; the raw config limit is {MAX_RAW_CONFIG_BYTES}",
                path.display()
            ),
            Self::InvalidEnvironmentName { name } => write!(
                formatter,
                "Typaxis environment variable name is not UTF-8: {:?}",
                name
            ),
            Self::InvalidEnvironmentValue { key, value } => write!(
                formatter,
                "environment variable `{key}` is not UTF-8: {:?}",
                value
            ),
            Self::Syntax {
                origin,
                line,
                detail,
            } => {
                write!(formatter, "invalid TOML in {origin}")?;
                if let Some(line) = line {
                    write!(formatter, " at line {line}")?;
                }
                write!(formatter, ": {detail}")
            }
            Self::UnknownKey { origin, key } => {
                write!(formatter, "unknown configuration key `{key}` in {origin}")
            }
            Self::DuplicateKey { origin, key } => {
                write!(formatter, "duplicate configuration key `{key}` in {origin}")
            }
            Self::MissingContract { path } => write!(
                formatter,
                "raw config `{}` must contain a known 1.0 or 1.1 `contract`",
                path.display()
            ),
            Self::ContractMismatch { origin, found } => write!(
                formatter,
                "configuration contract in {origin} is `{found}`, expected `typaxis.contract/1.0` or `{CONTRACT}`"
            ),
            Self::InvalidValue {
                origin,
                key,
                detail,
            } => write!(
                formatter,
                "invalid value for configuration key `{key}` in {origin}: {detail}"
            ),
            Self::Effective(EffectiveConfigError::ResourceLimits(reason)) => {
                write!(formatter, "invalid effective resource limits: {reason:?}")
            }
            Self::Effective(EffectiveConfigError::NonCanonicalResourceRoots) => {
                formatter.write_str("resource_roots must be unique portable paths")
            }
            Self::Effective(EffectiveConfigError::InvalidAllowedUriSchemes) => {
                formatter.write_str("allowed_uri_schemes must be unique registered URI schemes")
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
struct MergedConfig {
    strict: bool,
    compression: PdfStreamCompression,
    resource_roots: Vec<ConfigResourceRoot>,
    allowed_uri_schemes: Vec<String>,
    unicode_version: String,
    japanese_line_break_version: String,
    limits: ResourceLimits,
}

impl Default for MergedConfig {
    fn default() -> Self {
        Self {
            strict: false,
            compression: PdfStreamCompression::Flate,
            resource_roots: vec![ConfigResourceRoot::ProjectRoot],
            allowed_uri_schemes: DEFAULT_ALLOWED_URI_SCHEMES
                .iter()
                .map(|scheme| (*scheme).to_owned())
                .collect(),
            unicode_version: REGISTERED_UNICODE_VERSION.to_owned(),
            japanese_line_break_version: REGISTERED_JAPANESE_LINE_BREAK_VERSION.to_owned(),
            limits: ResourceLimits::default(),
        }
    }
}

impl MergedConfig {
    fn apply(&mut self, key: &str, value: Value, origin: &str) -> Result<(), ConfigError> {
        match key {
            "contract" => {
                let found = expect_string(value, key, origin)?;
                if found.parse::<DocumentPackageContractId>().is_ok() {
                    Ok(())
                } else {
                    Err(ConfigError::ContractMismatch {
                        origin: origin.to_owned(),
                        found,
                    })
                }
            }
            "deterministic" => {
                let deterministic = expect_bool(value, key, origin)?;
                if deterministic {
                    Ok(())
                } else {
                    Err(ConfigError::InvalidValue {
                        origin: origin.to_owned(),
                        key: key.to_owned(),
                        detail: "the deterministic contract requires `true`".to_owned(),
                    })
                }
            }
            "strict" => {
                self.strict = expect_bool(value, key, origin)?;
                Ok(())
            }
            "pdf_stream_compression" => {
                let compression = expect_string(value, key, origin)?;
                self.compression = match compression.as_str() {
                    "flate" => PdfStreamCompression::Flate,
                    "none" => PdfStreamCompression::None,
                    _ => {
                        return Err(ConfigError::InvalidValue {
                            origin: origin.to_owned(),
                            key: key.to_owned(),
                            detail: "expected `\"flate\"` or `\"none\"`".to_owned(),
                        });
                    }
                };
                Ok(())
            }
            "resource_roots" => {
                let roots = expect_string_array(value, key, origin)?;
                self.resource_roots = roots
                    .into_iter()
                    .map(|root| {
                        ConfigResourceRoot::parse(root.clone()).map_err(|reason| {
                            ConfigError::InvalidValue {
                                origin: origin.to_owned(),
                                key: key.to_owned(),
                                detail: format!(
                                    "`{root}` is not a portable resource root: {reason:?}"
                                ),
                            }
                        })
                    })
                    .collect::<Result<_, _>>()?;
                canonicalize_roots(&mut self.resource_roots);
                Ok(())
            }
            "allowed_uri_schemes" => {
                self.allowed_uri_schemes = expect_string_array(value, key, origin)?;
                self.allowed_uri_schemes
                    .sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
                Ok(())
            }
            "data_versions.unicode" => {
                self.unicode_version = expect_string(value, key, origin)?;
                Ok(())
            }
            "data_versions.japanese_line_break" => {
                self.japanese_line_break_version = expect_string(value, key, origin)?;
                Ok(())
            }
            "data_versions" => Err(ConfigError::InvalidValue {
                origin: origin.to_owned(),
                key: key.to_owned(),
                detail: "expected a table with `unicode` and/or `japanese_line_break`".to_owned(),
            }),
            "limits" => Err(ConfigError::InvalidValue {
                origin: origin.to_owned(),
                key: key.to_owned(),
                detail: "expected a table containing resource-limit fields".to_owned(),
            }),
            _ if key.starts_with("limits.") => {
                let name = &key["limits.".len()..];
                if !LIMIT_NAMES.contains(&name) {
                    return Err(ConfigError::UnknownKey {
                        origin: origin.to_owned(),
                        key: key.to_owned(),
                    });
                }
                let value = expect_unsigned_integer(value, key, origin)?;
                ensure_limit_storage_range(name, value, origin)?;
                set_limit(&mut self.limits, name, value);
                Ok(())
            }
            _ => Err(ConfigError::UnknownKey {
                origin: origin.to_owned(),
                key: key.to_owned(),
            }),
        }
    }

    fn apply_cli(&mut self, overrides: &ConfigOverrides) -> Result<(), ConfigError> {
        if let Some(strict) = overrides.strict {
            self.strict = strict;
        }
        if overrides.no_compress {
            self.compression = PdfStreamCompression::None;
        }
        for (name, value) in &overrides.limits {
            ensure_limit_storage_range(name, *value, "command line")?;
            set_limit(&mut self.limits, name, *value);
        }
        Ok(())
    }

    fn finish(self) -> Result<EffectiveConfig, ConfigError> {
        let data_versions = EffectiveDataVersions::new(
            self.unicode_version.clone(),
            self.japanese_line_break_version.clone(),
        )
        .ok_or_else(|| ConfigError::InvalidValue {
            origin: "merged configuration".to_owned(),
            key: "data_versions".to_owned(),
            detail: format!(
                "unregistered pair unicode={:?}, japanese_line_break={:?}",
                self.unicode_version, self.japanese_line_break_version
            ),
        })?;
        EffectiveConfig::new(
            self.strict,
            self.compression,
            self.resource_roots,
            self.allowed_uri_schemes,
            data_versions,
            self.limits,
        )
        .map_err(ConfigError::Effective)
    }
}

fn canonicalize_roots(roots: &mut [ConfigResourceRoot]) {
    roots.sort_by(|left, right| {
        left.wire_value()
            .as_bytes()
            .cmp(right.wire_value().as_bytes())
    });
}

fn load_file(path: &Path, merged: &mut MergedConfig) -> Result<(), ConfigError> {
    let mut file = open_config_file(path)?;
    let snapshot = ConfigFileSnapshot::from_file(&file, path)?;
    if !snapshot.regular {
        return Err(ConfigError::Io {
            path: path.to_owned(),
            source: io::Error::new(io::ErrorKind::InvalidInput, "config is not a regular file"),
        });
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
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockShared).map_err(
        |source| ConfigError::Io {
            path: path.to_owned(),
            source: io::Error::other(format!("cannot lock config for a stable read: {source}")),
        },
    )?;
    let byte_length = snapshot.length;
    if byte_length > MAX_RAW_CONFIG_BYTES {
        return Err(ConfigError::FileTooLarge {
            path: path.to_owned(),
            byte_length,
        });
    }
    let allocation = usize::try_from(byte_length).map_err(|_| ConfigError::FileTooLarge {
        path: path.to_owned(),
        byte_length,
    })?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(allocation)
        .map_err(|_| ConfigError::FileTooLarge {
            path: path.to_owned(),
            byte_length,
        })?;
    bytes.resize(allocation, 0);
    file.read_exact(&mut bytes)
        .map_err(|source| ConfigError::Io {
            path: path.to_owned(),
            source,
        })?;

    // Never trust an initial metadata length as a read bound by itself. Probe a
    // single byte past that bound and compare a second opened-file snapshot so
    // growth, replacement-through-the-path, and same-length mutation fail
    // closed without allocating or reading an unbounded payload.
    let mut trailing = [0u8; 1];
    let trailing_length = file.read(&mut trailing).map_err(|source| ConfigError::Io {
        path: path.to_owned(),
        source,
    })?;
    let final_snapshot = ConfigFileSnapshot::from_file(&file, path)?;
    if final_snapshot.length > MAX_RAW_CONFIG_BYTES {
        return Err(ConfigError::FileTooLarge {
            path: path.to_owned(),
            byte_length: final_snapshot.length,
        });
    }
    if trailing_length != 0 || final_snapshot != snapshot {
        return Err(ConfigError::Io {
            path: path.to_owned(),
            source: io::Error::other("config changed while it was being read"),
        });
    }
    let text = std::str::from_utf8(&bytes).map_err(|error| ConfigError::InvalidFileUtf8 {
        path: path.to_owned(),
        valid_up_to: error.valid_up_to(),
    })?;
    let origin = format!("config `{}`", path.display());
    let assignments = parse_toml_document(text, &origin)?;
    let mut found_contract = false;
    for assignment in assignments {
        if assignment.key == "contract" {
            found_contract = true;
        }
        merged.apply(&assignment.key, assignment.value, &origin)?;
    }
    if !found_contract {
        return Err(ConfigError::MissingContract {
            path: path.to_owned(),
        });
    }
    Ok(())
}

#[cfg(unix)]
fn open_config_file(path: &Path) -> Result<fs::File, ConfigError> {
    let descriptor = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC | rustix::fs::OFlags::NONBLOCK,
        rustix::fs::Mode::empty(),
    )
    .map_err(|source| ConfigError::Io {
        path: path.to_owned(),
        source: io::Error::from(source),
    })?;
    Ok(fs::File::from(descriptor))
}

#[cfg(not(unix))]
fn open_config_file(path: &Path) -> Result<fs::File, ConfigError> {
    let metadata = fs::metadata(path).map_err(|source| ConfigError::Io {
        path: path.to_owned(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(ConfigError::Io {
            path: path.to_owned(),
            source: io::Error::new(io::ErrorKind::InvalidInput, "config is not a regular file"),
        });
    }
    fs::File::open(path).map_err(|source| ConfigError::Io {
        path: path.to_owned(),
        source,
    })
}

#[cfg(unix)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct ConfigFileSnapshot {
    device: u64,
    inode: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
    regular: bool,
}

#[cfg(unix)]
impl ConfigFileSnapshot {
    fn from_file(file: &fs::File, path: &Path) -> Result<Self, ConfigError> {
        use std::os::unix::fs::MetadataExt;

        let metadata = file.metadata().map_err(|source| ConfigError::Io {
            path: path.to_owned(),
            source,
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
#[derive(Clone, Debug, Eq, PartialEq)]
struct ConfigFileSnapshot {
    length: u64,
    modified: std::time::SystemTime,
    regular: bool,
}

#[cfg(not(unix))]
impl ConfigFileSnapshot {
    fn from_file(file: &fs::File, path: &Path) -> Result<Self, ConfigError> {
        let metadata = file.metadata().map_err(|source| ConfigError::Io {
            path: path.to_owned(),
            source,
        })?;
        let modified = metadata.modified().map_err(|source| ConfigError::Io {
            path: path.to_owned(),
            source: io::Error::other(format!(
                "cannot read config modification time for a stable read: {source}"
            )),
        })?;
        Ok(Self {
            length: metadata.len(),
            modified,
            regular: metadata.is_file(),
        })
    }
}

fn apply_environment<I, K, V>(environment: I, merged: &mut MergedConfig) -> Result<(), ConfigError>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<OsString>,
    V: Into<OsString>,
{
    let mut entries = Vec::new();
    for (name, value) in environment {
        let name = name.into();
        if !has_typaxis_prefix(&name) {
            continue;
        }
        let name = name
            .into_string()
            .map_err(|name| ConfigError::InvalidEnvironmentName { name })?;
        let value = value.into();
        let value = value
            .into_string()
            .map_err(|value| ConfigError::InvalidEnvironmentValue {
                key: name.clone(),
                value,
            })?;
        entries.push((name, value));
    }
    entries.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));

    let mut seen = BTreeSet::new();
    for (name, raw_value) in entries {
        if !seen.insert(name.clone()) {
            return Err(ConfigError::DuplicateKey {
                origin: "environment".to_owned(),
                key: name,
            });
        }
        let key = environment_key(&name).ok_or_else(|| ConfigError::UnknownKey {
            origin: "environment".to_owned(),
            key: name.clone(),
        })?;
        let origin = format!("environment variable `{name}`");
        let value = parse_toml_value(&raw_value).map_err(|detail| ConfigError::Syntax {
            origin: origin.clone(),
            line: None,
            detail,
        })?;
        merged.apply(&key, value, &origin)?;
    }
    Ok(())
}

fn has_typaxis_prefix(name: &OsStr) -> bool {
    if let Some(name) = name.to_str() {
        return name.starts_with("TYPAXIS_");
    }

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        return name.as_bytes().starts_with(b"TYPAXIS_");
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let prefix: Vec<u16> = "TYPAXIS_".encode_utf16().collect();
        let encoded: Vec<u16> = name.encode_wide().take(prefix.len()).collect();
        return encoded == prefix;
    }

    #[allow(unreachable_code)]
    false
}

fn environment_key(name: &str) -> Option<String> {
    let suffix = name.strip_prefix("TYPAXIS_")?;
    let direct = match suffix {
        "CONTRACT" => Some("contract"),
        "DETERMINISTIC" => Some("deterministic"),
        "STRICT" => Some("strict"),
        "PDF_STREAM_COMPRESSION" => Some("pdf_stream_compression"),
        "RESOURCE_ROOTS" => Some("resource_roots"),
        "ALLOWED_URI_SCHEMES" => Some("allowed_uri_schemes"),
        "DATA_VERSIONS__UNICODE" => Some("data_versions.unicode"),
        "DATA_VERSIONS__JAPANESE_LINE_BREAK" => Some("data_versions.japanese_line_break"),
        _ => None,
    };
    if let Some(direct) = direct {
        return Some(direct.to_owned());
    }
    let limit = suffix.strip_prefix("LIMITS__")?;
    if limit.is_empty()
        || !limit
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return None;
    }
    let limit = limit.to_ascii_lowercase();
    if LIMIT_NAMES.contains(&limit.as_str()) {
        Some(format!("limits.{limit}"))
    } else {
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Value {
    Bool(bool),
    Integer(i128),
    String(String),
    Array(Vec<Value>),
    Other(&'static str),
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Self::Bool(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Other(name) => name,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Assignment {
    key: String,
    value: Value,
}

const MAX_TOML_VALUE_NESTING: usize = 64;
const ENVIRONMENT_VALUE_KEY: &str = "__typaxis_environment_value";

fn parse_toml_document(text: &str, origin: &str) -> Result<Vec<Assignment>, ConfigError> {
    ensure_toml_nesting(text).map_err(|offset| ConfigError::Syntax {
        origin: origin.to_owned(),
        line: Some(line_at_offset(text, offset)),
        detail: format!("TOML value nesting exceeds {MAX_TOML_VALUE_NESTING}"),
    })?;
    let document = text
        .parse::<toml::Table>()
        .map_err(|error| document_parse_error(text, origin, error))?;
    let mut assignments = Vec::new();
    for (key, value) in document {
        match key.as_str() {
            "contract"
            | "deterministic"
            | "strict"
            | "pdf_stream_compression"
            | "resource_roots"
            | "allowed_uri_schemes" => assignments.push(Assignment {
                key,
                value: convert_toml_value(value),
            }),
            "data_versions" => flatten_known_table(
                "data_versions",
                value,
                &["unicode", "japanese_line_break"],
                &mut assignments,
                origin,
            )?,
            "limits" => {
                flatten_known_table("limits", value, LIMIT_NAMES, &mut assignments, origin)?
            }
            _ => {
                return Err(ConfigError::UnknownKey {
                    origin: origin.to_owned(),
                    key,
                });
            }
        }
    }
    Ok(assignments)
}

fn flatten_known_table(
    table_name: &str,
    value: TomlValue,
    known_keys: &[&str],
    assignments: &mut Vec<Assignment>,
    origin: &str,
) -> Result<(), ConfigError> {
    let TomlValue::Table(table) = value else {
        assignments.push(Assignment {
            key: table_name.to_owned(),
            value: convert_toml_value(value),
        });
        return Ok(());
    };
    for (key, value) in table {
        let full_key = format!("{table_name}.{key}");
        if !known_keys.contains(&key.as_str()) {
            return Err(ConfigError::UnknownKey {
                origin: origin.to_owned(),
                key: full_key,
            });
        }
        assignments.push(Assignment {
            key: full_key,
            value: convert_toml_value(value),
        });
    }
    Ok(())
}

fn convert_toml_value(value: TomlValue) -> Value {
    match value {
        TomlValue::String(value) => Value::String(value),
        TomlValue::Integer(value) => Value::Integer(i128::from(value)),
        TomlValue::Float(_) => Value::Other("float"),
        TomlValue::Boolean(value) => Value::Bool(value),
        TomlValue::Datetime(_) => Value::Other("date/time"),
        TomlValue::Array(values) => Value::Array(
            values
                .into_iter()
                .map(convert_toml_value)
                .collect::<Vec<_>>(),
        ),
        TomlValue::Table(_) => Value::Other("table"),
    }
}

fn document_parse_error(text: &str, origin: &str, error: toml::de::Error) -> ConfigError {
    let detail = error.message().to_owned();
    if let Some(key) = duplicate_key(&detail) {
        return ConfigError::DuplicateKey {
            origin: origin.to_owned(),
            key,
        };
    }
    ConfigError::Syntax {
        origin: origin.to_owned(),
        line: error.span().map(|span| line_at_offset(text, span.start)),
        detail,
    }
}

fn duplicate_key(message: &str) -> Option<String> {
    message.lines().find_map(|line| {
        let key = line.strip_prefix("duplicate key `")?;
        let key = key.split_once('`')?.0;
        Some(key.trim_matches(['\'', '"']).to_owned())
    })
}

fn line_at_offset(text: &str, offset: usize) -> usize {
    text.as_bytes()[..offset.min(text.len())]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

fn parse_toml_value(input: &str) -> Result<Value, String> {
    ensure_toml_nesting(input)
        .map_err(|_| format!("TOML value nesting exceeds {MAX_TOML_VALUE_NESTING}"))?;
    let document = format!("{ENVIRONMENT_VALUE_KEY} = {input}\n");
    let mut table = document
        .parse::<toml::Table>()
        .map_err(|error| error.message().to_owned())?;
    if table.len() != 1 {
        return Err("expected exactly one TOML value".to_owned());
    }
    table
        .remove(ENVIRONMENT_VALUE_KEY)
        .map(convert_toml_value)
        .ok_or_else(|| "expected exactly one TOML value".to_owned())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TomlScanState {
    Normal,
    Comment,
    BasicString,
    LiteralString,
    MultilineBasicString,
    MultilineLiteralString,
}

/// Applies the project's delimiter-nesting bound before the official parser
/// constructs a value tree. Delimiters inside comments or any TOML string
/// form do not count.
fn ensure_toml_nesting(input: &str) -> Result<(), usize> {
    let bytes = input.as_bytes();
    let mut state = TomlScanState::Normal;
    let mut closers = Vec::with_capacity(MAX_TOML_VALUE_NESTING);
    let mut offset = 0;
    while offset < bytes.len() {
        match state {
            TomlScanState::Normal => match bytes[offset] {
                b'#' => {
                    state = TomlScanState::Comment;
                    offset += 1;
                }
                b'"' if bytes.get(offset..offset + 3) == Some(b"\"\"\"") => {
                    state = TomlScanState::MultilineBasicString;
                    offset += 3;
                }
                b'\'' if bytes.get(offset..offset + 3) == Some(b"'''") => {
                    state = TomlScanState::MultilineLiteralString;
                    offset += 3;
                }
                b'"' => {
                    state = TomlScanState::BasicString;
                    offset += 1;
                }
                b'\'' => {
                    state = TomlScanState::LiteralString;
                    offset += 1;
                }
                b'[' => {
                    if closers.len() == MAX_TOML_VALUE_NESTING {
                        return Err(offset);
                    }
                    closers.push(b']');
                    offset += 1;
                }
                b'{' => {
                    if closers.len() == MAX_TOML_VALUE_NESTING {
                        return Err(offset);
                    }
                    closers.push(b'}');
                    offset += 1;
                }
                closer @ (b']' | b'}') => {
                    if closers.last() == Some(&closer) {
                        closers.pop();
                    }
                    offset += 1;
                }
                _ => offset += 1,
            },
            TomlScanState::Comment => {
                if bytes[offset] == b'\n' {
                    state = TomlScanState::Normal;
                }
                offset += 1;
            }
            TomlScanState::BasicString => match bytes[offset] {
                b'\\' => offset = (offset + 2).min(bytes.len()),
                b'"' => {
                    state = TomlScanState::Normal;
                    offset += 1;
                }
                _ => offset += 1,
            },
            TomlScanState::LiteralString => {
                if bytes[offset] == b'\'' {
                    state = TomlScanState::Normal;
                }
                offset += 1;
            }
            TomlScanState::MultilineBasicString => {
                if bytes[offset] == b'"' {
                    let quote_count = bytes[offset..]
                        .iter()
                        .take_while(|byte| **byte == b'"')
                        .count();
                    if quote_count >= 3 {
                        state = TomlScanState::Normal;
                    }
                    offset += quote_count;
                } else if bytes[offset] == b'\\' {
                    offset = (offset + 2).min(bytes.len());
                } else {
                    offset += 1;
                }
            }
            TomlScanState::MultilineLiteralString => {
                if bytes[offset] == b'\'' {
                    let quote_count = bytes[offset..]
                        .iter()
                        .take_while(|byte| **byte == b'\'')
                        .count();
                    if quote_count >= 3 {
                        state = TomlScanState::Normal;
                    }
                    offset += quote_count;
                } else {
                    offset += 1;
                }
            }
        }
    }
    Ok(())
}

fn expect_bool(value: Value, key: &str, origin: &str) -> Result<bool, ConfigError> {
    match value {
        Value::Bool(value) => Ok(value),
        other => Err(wrong_type(key, origin, "a boolean", &other)),
    }
}

fn expect_string(value: Value, key: &str, origin: &str) -> Result<String, ConfigError> {
    match value {
        Value::String(value) => Ok(value),
        other => Err(wrong_type(key, origin, "a string", &other)),
    }
}

fn expect_string_array(value: Value, key: &str, origin: &str) -> Result<Vec<String>, ConfigError> {
    let Value::Array(values) = value else {
        return Err(wrong_type(key, origin, "an array of strings", &value));
    };
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| match value {
            Value::String(value) => Ok(value),
            other => Err(ConfigError::InvalidValue {
                origin: origin.to_owned(),
                key: key.to_owned(),
                detail: format!(
                    "array item {} must be a string, found {}",
                    index + 1,
                    other.type_name()
                ),
            }),
        })
        .collect()
}

fn expect_unsigned_integer(value: Value, key: &str, origin: &str) -> Result<u64, ConfigError> {
    match value {
        Value::Integer(value) => u64::try_from(value).map_err(|_| ConfigError::InvalidValue {
            origin: origin.to_owned(),
            key: key.to_owned(),
            detail: "expected a non-negative integer representable as u64".to_owned(),
        }),
        other => Err(wrong_type(key, origin, "an integer", &other)),
    }
}

fn wrong_type(key: &str, origin: &str, expected: &str, actual: &Value) -> ConfigError {
    ConfigError::InvalidValue {
        origin: origin.to_owned(),
        key: key.to_owned(),
        detail: format!("expected {expected}, found {}", actual.type_name()),
    }
}

const LIMIT_NAMES: &[&str] = &[
    "max_input_bytes",
    "max_source_bytes",
    "max_include_depth",
    "max_include_files",
    "max_ast_nesting_depth",
    "max_ast_nodes",
    "max_style_rules",
    "max_text_bytes",
    "max_text_buffer_bytes",
    "max_shaping_context_bytes",
    "max_font_bytes",
    "max_fonts",
    "max_image_bytes",
    "max_images",
    "max_resource_bytes",
    "max_image_pixels",
    "max_decoded_image_bytes",
    "max_document_package_bytes",
    "max_json_nesting_depth",
    "max_pages",
    "max_layout_passes",
    "max_uri_bytes",
    "max_line_reshape_passes",
    "max_page_break_lookback",
    "max_footnote_reflows_per_page",
    "max_column_balance_candidates",
    "max_float_queue",
    "max_float_carry_pages",
    "max_cids_per_font",
    "max_fragments",
    "max_spool_bytes",
    "max_pdf_objects",
    "max_output_bytes",
];

fn normalize_limit_name(name: &str) -> String {
    let name = name.trim_start_matches("--").replace('-', "_");
    if name.starts_with("max_") {
        name
    } else {
        format!("max_{name}")
    }
}

fn ensure_limit_storage_range(name: &str, value: u64, origin: &str) -> Result<(), ConfigError> {
    let maximum = match name {
        "max_json_nesting_depth"
        | "max_layout_passes"
        | "max_line_reshape_passes"
        | "max_page_break_lookback"
        | "max_footnote_reflows_per_page"
        | "max_column_balance_candidates"
        | "max_float_carry_pages"
        | "max_cids_per_font" => u64::from(u16::MAX),
        "max_source_bytes"
        | "max_include_depth"
        | "max_include_files"
        | "max_ast_nesting_depth"
        | "max_text_buffer_bytes"
        | "max_shaping_context_bytes"
        | "max_fonts"
        | "max_images"
        | "max_pages"
        | "max_uri_bytes"
        | "max_float_queue"
        | "max_pdf_objects" => u64::from(u32::MAX),
        _ => u64::MAX,
    };
    if value > maximum {
        Err(ConfigError::InvalidValue {
            origin: origin.to_owned(),
            key: format!("limits.{name}"),
            detail: format!("integer {value} exceeds the field's storage maximum {maximum}"),
        })
    } else {
        Ok(())
    }
}

fn set_limit(limits: &mut ResourceLimits, name: &str, value: u64) {
    match name {
        "max_input_bytes" => limits.max_input_bytes = value,
        "max_source_bytes" => limits.max_source_bytes = value as u32,
        "max_include_depth" => limits.max_include_depth = value as u32,
        "max_include_files" => limits.max_include_files = value as u32,
        "max_ast_nesting_depth" => limits.max_ast_nesting_depth = value as u32,
        "max_ast_nodes" => limits.max_ast_nodes = value,
        "max_style_rules" => limits.max_style_rules = value,
        "max_text_bytes" => limits.max_text_bytes = value,
        "max_text_buffer_bytes" => limits.max_text_buffer_bytes = value as u32,
        "max_shaping_context_bytes" => limits.max_shaping_context_bytes = value as u32,
        "max_font_bytes" => limits.max_font_bytes = value,
        "max_fonts" => limits.max_fonts = value as u32,
        "max_image_bytes" => limits.max_image_bytes = value,
        "max_images" => limits.max_images = value as u32,
        "max_resource_bytes" => limits.max_resource_bytes = value,
        "max_image_pixels" => limits.max_image_pixels = value,
        "max_decoded_image_bytes" => limits.max_decoded_image_bytes = value,
        "max_document_package_bytes" => limits.max_document_package_bytes = value,
        "max_json_nesting_depth" => limits.max_json_nesting_depth = value as u16,
        "max_pages" => limits.max_pages = value as u32,
        "max_layout_passes" => limits.max_layout_passes = value as u16,
        "max_uri_bytes" => limits.max_uri_bytes = value as u32,
        "max_line_reshape_passes" => limits.max_line_reshape_passes = value as u16,
        "max_page_break_lookback" => limits.max_page_break_lookback = value as u16,
        "max_footnote_reflows_per_page" => {
            limits.max_footnote_reflows_per_page = value as u16;
        }
        "max_column_balance_candidates" => limits.max_column_balance_candidates = value as u16,
        "max_float_queue" => limits.max_float_queue = value as u32,
        "max_float_carry_pages" => limits.max_float_carry_pages = value as u16,
        "max_cids_per_font" => limits.max_cids_per_font = value as u16,
        "max_fragments" => limits.max_fragments = value,
        "max_spool_bytes" => limits.max_spool_bytes = value,
        "max_pdf_objects" => limits.max_pdf_objects = value as u32,
        "max_output_bytes" => limits.max_output_bytes = value,
        _ => unreachable!("limit name was checked before assignment"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

    struct TempConfig(PathBuf);

    impl TempConfig {
        fn new(contents: &[u8]) -> Self {
            let id = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "typaxis-config-test-{}-{id}.toml",
                std::process::id()
            ));
            fs::write(&path, contents).unwrap();
            Self(path)
        }
    }

    impl Drop for TempConfig {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn defaults_form_a_complete_effective_config() {
        let config = load(
            None,
            std::iter::empty::<(OsString, OsString)>(),
            &ConfigOverrides::default(),
        )
        .unwrap();
        assert!(!config.strict());
        assert_eq!(config.stream_compression(), PdfStreamCompression::Flate);
        assert_eq!(config.resource_roots(), &[ConfigResourceRoot::ProjectRoot]);
        assert_eq!(
            config.allowed_uri_schemes(),
            &["http", "https", "mailto", "tel"]
        );
        assert_eq!(config.limits().get(), &ResourceLimits::default());
    }

    #[test]
    fn precedence_is_defaults_file_environment_then_cli() {
        let file = TempConfig::new(
            br#"
contract = "typaxis.contract/1.1"
strict = true
pdf_stream_compression = "flate"
resource_roots = ["resources", "."]
allowed_uri_schemes = ["mailto", "https"]

[data_versions]
unicode = "16.0.0"
japanese_line_break = "typaxis-jlreq-horizontal/1.0.0"

[limits]
max_pages = 20
"#,
        );
        let environment = [
            ("TYPAXIS_STRICT", "false"),
            ("TYPAXIS_PDF_STREAM_COMPRESSION", "\"none\""),
            ("TYPAXIS_LIMITS__MAX_PAGES", "30"),
        ];
        let mut overrides = ConfigOverrides {
            strict: Some(true),
            no_compress: true,
            ..ConfigOverrides::default()
        };
        overrides.set_limit("--max-pages", 40).unwrap();

        let config = load(Some(&file.0), environment, &overrides).unwrap();
        assert!(config.strict());
        assert_eq!(config.stream_compression(), PdfStreamCompression::None);
        assert_eq!(config.limits().get().max_pages, 40);
        assert_eq!(
            config
                .resource_roots()
                .iter()
                .map(ConfigResourceRoot::wire_value)
                .collect::<Vec<_>>(),
            vec![".", "resources"]
        );
        assert_eq!(config.allowed_uri_schemes(), &["https", "mailto"]);
    }

    #[test]
    fn raw_file_requires_exact_contract_and_rejects_unknown_and_wrong_types() {
        let missing = TempConfig::new(b"strict = true\n");
        assert!(matches!(
            load(
                Some(&missing.0),
                std::iter::empty::<(OsString, OsString)>(),
                &ConfigOverrides::default()
            ),
            Err(ConfigError::MissingContract { .. })
        ));

        let wrong = TempConfig::new(b"contract = \"future\"\n");
        assert!(matches!(
            load(
                Some(&wrong.0),
                std::iter::empty::<(OsString, OsString)>(),
                &ConfigOverrides::default()
            ),
            Err(ConfigError::ContractMismatch { .. })
        ));

        let unknown = TempConfig::new(b"contract = \"typaxis.contract/1.1\"\nunknown = true\n");
        assert!(matches!(
            load(
                Some(&unknown.0),
                std::iter::empty::<(OsString, OsString)>(),
                &ConfigOverrides::default()
            ),
            Err(ConfigError::UnknownKey { .. })
        ));

        let wrong_type =
            TempConfig::new(b"contract = \"typaxis.contract/1.1\"\nstrict = \"true\"\n");
        assert!(matches!(
            load(
                Some(&wrong_type.0),
                std::iter::empty::<(OsString, OsString)>(),
                &ConfigOverrides::default()
            ),
            Err(ConfigError::InvalidValue { .. })
        ));
    }

    #[test]
    fn raw_config_read_is_regular_stable_and_bounded() {
        let oversized = TempConfig::new(b"");
        fs::OpenOptions::new()
            .write(true)
            .open(&oversized.0)
            .unwrap()
            .set_len(MAX_RAW_CONFIG_BYTES + 1)
            .unwrap();
        let error = load(
            Some(&oversized.0),
            std::iter::empty::<(OsString, OsString)>(),
            &ConfigOverrides::default(),
        )
        .unwrap_err();
        assert!(matches!(&error, ConfigError::FileTooLarge { .. }));
        assert!(error.is_limit());

        let id = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let directory = env::temp_dir().join(format!(
            "typaxis-config-directory-test-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let error = load(
            Some(&directory),
            std::iter::empty::<(OsString, OsString)>(),
            &ConfigOverrides::default(),
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::Io { .. }));
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn raw_config_snapshot_detects_same_length_timestamp_change() {
        let config = TempConfig::new(b"contract = \"typaxis.contract/1.1\"\n");
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&config.0)
            .unwrap();
        file.set_times(
            fs::FileTimes::new().set_modified(
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000),
            ),
        )
        .unwrap();
        let first = ConfigFileSnapshot::from_file(&file, &config.0).unwrap();

        file.set_times(
            fs::FileTimes::new().set_modified(
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_001),
            ),
        )
        .unwrap();
        let second = ConfigFileSnapshot::from_file(&file, &config.0).unwrap();

        assert_eq!(first.length, second.length);
        assert_ne!(first, second);
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
    fn raw_config_rejects_a_concurrent_exclusive_writer() {
        let file = TempConfig::new(b"contract = \"typaxis.contract/1.1\"\n");
        let writer = fs::OpenOptions::new().write(true).open(&file.0).unwrap();
        rustix::fs::flock(
            &writer,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .unwrap();

        let error = load(
            Some(&file.0),
            std::iter::empty::<(OsString, OsString)>(),
            &ConfigOverrides::default(),
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::Io { .. }));
        assert!(error.to_string().contains("stable read"));
    }

    #[cfg(any(target_os = "android", target_os = "linux"))]
    #[test]
    fn raw_config_fifo_is_rejected_without_waiting_for_a_writer() {
        let id = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "typaxis-config-fifo-test-{}-{id}",
            std::process::id()
        ));
        rustix::fs::mkfifoat(
            rustix::fs::CWD,
            &path,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .unwrap();

        let error = load(
            Some(&path),
            std::iter::empty::<(OsString, OsString)>(),
            &ConfigOverrides::default(),
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::Io { .. }));
        assert!(error.to_string().contains("not a regular file"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn every_resource_limit_has_an_environment_and_cli_route() {
        let defaults = ResourceLimits::default();
        let values = [
            ("max_input_bytes", defaults.max_input_bytes),
            ("max_source_bytes", u64::from(defaults.max_source_bytes)),
            ("max_include_depth", u64::from(defaults.max_include_depth)),
            ("max_include_files", u64::from(defaults.max_include_files)),
            (
                "max_ast_nesting_depth",
                u64::from(defaults.max_ast_nesting_depth),
            ),
            ("max_ast_nodes", defaults.max_ast_nodes),
            ("max_style_rules", defaults.max_style_rules),
            ("max_text_bytes", defaults.max_text_bytes),
            (
                "max_text_buffer_bytes",
                u64::from(defaults.max_text_buffer_bytes),
            ),
            (
                "max_shaping_context_bytes",
                u64::from(defaults.max_shaping_context_bytes),
            ),
            ("max_font_bytes", defaults.max_font_bytes),
            ("max_fonts", u64::from(defaults.max_fonts)),
            ("max_image_bytes", defaults.max_image_bytes),
            ("max_images", u64::from(defaults.max_images)),
            ("max_resource_bytes", defaults.max_resource_bytes),
            ("max_image_pixels", defaults.max_image_pixels),
            ("max_decoded_image_bytes", defaults.max_decoded_image_bytes),
            (
                "max_document_package_bytes",
                defaults.max_document_package_bytes,
            ),
            (
                "max_json_nesting_depth",
                u64::from(defaults.max_json_nesting_depth),
            ),
            ("max_pages", u64::from(defaults.max_pages)),
            ("max_layout_passes", u64::from(defaults.max_layout_passes)),
            ("max_uri_bytes", u64::from(defaults.max_uri_bytes)),
            (
                "max_line_reshape_passes",
                u64::from(defaults.max_line_reshape_passes),
            ),
            (
                "max_page_break_lookback",
                u64::from(defaults.max_page_break_lookback),
            ),
            (
                "max_footnote_reflows_per_page",
                u64::from(defaults.max_footnote_reflows_per_page),
            ),
            (
                "max_column_balance_candidates",
                u64::from(defaults.max_column_balance_candidates),
            ),
            ("max_float_queue", u64::from(defaults.max_float_queue)),
            (
                "max_float_carry_pages",
                u64::from(defaults.max_float_carry_pages),
            ),
            ("max_cids_per_font", u64::from(defaults.max_cids_per_font)),
            ("max_fragments", defaults.max_fragments),
            ("max_spool_bytes", defaults.max_spool_bytes),
            ("max_pdf_objects", u64::from(defaults.max_pdf_objects)),
            ("max_output_bytes", defaults.max_output_bytes),
        ];
        assert_eq!(values.len(), LIMIT_NAMES.len());
        assert_eq!(
            values.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
            LIMIT_NAMES
        );

        let mut overrides = ConfigOverrides::default();
        let environment: Vec<_> = values
            .iter()
            .map(|(name, value)| {
                overrides.set_limit(name, *value).unwrap();
                (
                    format!("TYPAXIS_LIMITS__{}", name.to_ascii_uppercase()),
                    value.to_string(),
                )
            })
            .collect();
        let config = load(None, environment, &overrides).unwrap();
        assert_eq!(config.limits().get(), &defaults);
    }

    #[test]
    fn environment_is_strict_and_parses_toml_values() {
        let config = load(
            None,
            [
                ("TYPAXIS_RESOURCE_ROOTS", "[\"z\", \".\", \"a\"]"),
                ("TYPAXIS_ALLOWED_URI_SCHEMES", "['tel', 'http']"),
                ("TYPAXIS_STRICT", "true"),
            ],
            &ConfigOverrides::default(),
        )
        .unwrap();
        assert!(config.strict());
        assert_eq!(config.allowed_uri_schemes(), &["http", "tel"]);
        assert_eq!(
            config
                .resource_roots()
                .iter()
                .map(ConfigResourceRoot::wire_value)
                .collect::<Vec<_>>(),
            vec![".", "a", "z"]
        );

        assert!(matches!(
            load(
                None,
                [("TYPAXIS_NOT_A_FIELD", "true")],
                &ConfigOverrides::default()
            ),
            Err(ConfigError::UnknownKey { .. })
        ));
        assert!(matches!(
            load(None, [("TYPAXIS_STRICT", "1")], &ConfigOverrides::default()),
            Err(ConfigError::InvalidValue { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn prefixed_non_utf8_environment_is_rejected() {
        use std::os::unix::ffi::OsStringExt;

        let invalid_name = OsString::from_vec(b"TYPAXIS_\xff".to_vec());
        assert!(matches!(
            load(
                None,
                [(invalid_name, OsString::from("true"))],
                &ConfigOverrides::default()
            ),
            Err(ConfigError::InvalidEnvironmentName { .. })
        ));

        let invalid_value = OsString::from_vec(vec![0xff]);
        assert!(matches!(
            load(
                None,
                [(OsString::from("TYPAXIS_STRICT"), invalid_value)],
                &ConfigOverrides::default()
            ),
            Err(ConfigError::InvalidEnvironmentValue { .. })
        ));
    }

    #[test]
    fn final_resource_relations_are_validated() {
        let mut overrides = ConfigOverrides::default();
        overrides.set_limit("max_input_bytes", 10).unwrap();
        overrides.set_limit("max_source_bytes", 11).unwrap();
        let error = load(None, std::iter::empty::<(OsString, OsString)>(), &overrides).unwrap_err();
        assert!(error.is_effective_resource_limit());
        assert!(error.is_limit());
        assert_eq!(error.diagnostic_code(), "P1001");
    }

    #[test]
    fn raw_1_0_and_1_1_configs_normalize_to_the_same_1_1_jcs() {
        let legacy =
            TempConfig::new(b"contract = \"typaxis.contract/1.0\"\n[limits]\nmax_pages = 321\n");
        let current = TempConfig::new(
            format!(
                "contract = \"typaxis.contract/1.1\"\n[limits]\nmax_document_package_bytes = {}\nmax_json_nesting_depth = {}\nmax_pages = 321\n",
                typaxis_core::MachineInputLimitBounds::DEFAULT_MAX_DOCUMENT_PACKAGE_BYTES,
                typaxis_core::MachineInputLimitBounds::DEFAULT_MAX_JSON_NESTING_DEPTH,
            )
            .as_bytes(),
        );
        let environment = [("TYPAXIS_LIMITS__MAX_PAGES", "654")];
        let legacy = load(Some(&legacy.0), environment, &ConfigOverrides::default()).unwrap();
        let current = load(Some(&current.0), environment, &ConfigOverrides::default()).unwrap();
        assert_eq!(legacy, current);
        assert!(legacy
            .canonical_jcs()
            .contains("\"contract\":\"typaxis.contract/1.1\""));
    }

    #[test]
    fn comments_multiline_arrays_and_integer_spellings_are_supported() {
        let file = TempConfig::new(
            br#"contract = "typaxis.contract/1.1" # required
resource_roots = [
  'assets', # an inline comment
  ".",
]
[limits]
max_pages = 0x2_710
"#,
        );
        let config = load(
            Some(&file.0),
            std::iter::empty::<(OsString, OsString)>(),
            &ConfigOverrides::default(),
        )
        .unwrap();
        assert_eq!(config.limits().get().max_pages, 10_000);
        assert_eq!(
            config
                .resource_roots()
                .iter()
                .map(ConfigResourceRoot::wire_value)
                .collect::<Vec<_>>(),
            vec![".", "assets"]
        );
    }

    #[test]
    fn inline_tables_and_quoted_dotted_known_keys_are_supported() {
        let inline = TempConfig::new(
            br#"contract = "typaxis.contract/1.1"
data_versions = { unicode = "16.0.0", japanese_line_break = "typaxis-jlreq-horizontal/1.0.0" }
limits = { max_pages = 321 }
"#,
        );
        let config = load(
            Some(&inline.0),
            std::iter::empty::<(OsString, OsString)>(),
            &ConfigOverrides::default(),
        )
        .unwrap();
        assert_eq!(config.limits().get().max_pages, 321);

        let dotted = TempConfig::new(
            br#""contract" = "typaxis.contract/1.1"
"strict" = true
"data_versions"."unicode" = "16.0.0"
'data_versions'.'japanese_line_break' = "typaxis-jlreq-horizontal/1.0.0"
"limits"."max_pages" = 654
"#,
        );
        let config = load(
            Some(&dotted.0),
            std::iter::empty::<(OsString, OsString)>(),
            &ConfigOverrides::default(),
        )
        .unwrap();
        assert!(config.strict());
        assert_eq!(config.limits().get().max_pages, 654);

        let literal_dotted = TempConfig::new(
            br#"contract = "typaxis.contract/1.1"
"limits.max_pages" = 1
"#,
        );
        assert!(matches!(
            load(
                Some(&literal_dotted.0),
                std::iter::empty::<(OsString, OsString)>(),
                &ConfigOverrides::default(),
            ),
            Err(ConfigError::UnknownKey { .. })
        ));
    }

    #[test]
    fn environment_arrays_accept_toml_comments_and_newlines() {
        let config = load(
            None,
            [(
                "TYPAXIS_ALLOWED_URI_SCHEMES",
                "[\n  'tel', # telephone links\n  \"http\",\n]",
            )],
            &ConfigOverrides::default(),
        )
        .unwrap();
        assert_eq!(config.allowed_uri_schemes(), &["http", "tel"]);
    }

    #[test]
    fn toml_rejects_nbsp_as_whitespace() {
        let file =
            TempConfig::new("contract = \"typaxis.contract/1.1\"\nstrict =\u{a0}true\n".as_bytes());
        assert!(matches!(
            load(
                Some(&file.0),
                std::iter::empty::<(OsString, OsString)>(),
                &ConfigOverrides::default(),
            ),
            Err(ConfigError::Syntax { .. })
        ));
        assert!(matches!(
            load(
                None,
                [("TYPAXIS_STRICT", "\u{a0}true")],
                &ConfigOverrides::default(),
            ),
            Err(ConfigError::Syntax { .. })
        ));
    }

    #[test]
    fn official_value_parser_enforces_signed_64_integer_bounds_and_nesting() {
        assert!(matches!(
            parse_toml_value("9223372036854775807"),
            Ok(Value::Integer(value)) if value == i128::from(i64::MAX)
        ));
        assert!(matches!(
            parse_toml_value("-9223372036854775808"),
            Ok(Value::Integer(value)) if value == i128::from(i64::MIN)
        ));
        assert!(parse_toml_value("9223372036854775808").is_err());
        assert!(parse_toml_value("-9223372036854775809").is_err());
        assert!(parse_toml_value("+0x10").is_err());
        assert!(parse_toml_value("-0b1").is_err());

        let nested = format!("{}0{}", "[".repeat(65), "]".repeat(65));
        assert!(parse_toml_value(&nested).is_err());
        let quoted_delimiters = format!("[\"{}\"]", "[".repeat(80));
        assert!(parse_toml_value(&quoted_delimiters).is_ok());
        let multiline_delimiters = format!("\"\"\"{}\"\"\"", "[".repeat(80));
        assert!(ensure_toml_nesting(&multiline_delimiters).is_ok());
        let nested_after_basic_string = format!("\"\"\"text\"\"\"\"{nested}");
        assert!(ensure_toml_nesting(&nested_after_basic_string).is_err());
        let nested_after_literal_string = format!("'''text''''{nested}");
        assert!(ensure_toml_nesting(&nested_after_literal_string).is_err());
        let commented_delimiters = format!("# {}\n[]", "[".repeat(80));
        assert!(ensure_toml_nesting(&commented_delimiters).is_ok());
    }

    #[test]
    fn raw_toml_rejects_unknown_and_duplicate_keys() {
        let unknown_nested = TempConfig::new(
            br#"contract = "typaxis.contract/1.1"
limits = { max_pages = 10, surprise = 1 }
"#,
        );
        assert!(matches!(
            load(
                Some(&unknown_nested.0),
                std::iter::empty::<(OsString, OsString)>(),
                &ConfigOverrides::default(),
            ),
            Err(ConfigError::UnknownKey { key, .. }) if key == "limits.surprise"
        ));

        let duplicate = TempConfig::new(
            br#"contract = "typaxis.contract/1.1"
strict = true
"strict" = false
"#,
        );
        assert!(matches!(
            load(
                Some(&duplicate.0),
                std::iter::empty::<(OsString, OsString)>(),
                &ConfigOverrides::default(),
            ),
            Err(ConfigError::DuplicateKey { .. })
        ));

        let duplicate_nested = TempConfig::new(
            br#"contract = "typaxis.contract/1.1"
limits.max_pages = 10
[limits]
"max_pages" = 11
"#,
        );
        let error = load(
            Some(&duplicate_nested.0),
            std::iter::empty::<(OsString, OsString)>(),
            &ConfigOverrides::default(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ConfigError::DuplicateKey { .. }),
            "{error:?}"
        );
    }
}
