//! Fixture-backed tests for the machine runners and public capability surface.

use super::*;

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt;
use std::ops::Index;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use typaxis_core::{
    ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, HostPath, MachinePdfProfileId,
    PdfStreamCompression, ResourceLimits, ValidatedResourceLimits,
};
use typaxis_document_package::{
    DocumentPackageDecodePolicy, DocumentPackageEncoder, StrictDocumentPackageDecoder,
};
use typaxis_machine_input::{HostMachineInputSession, MachineInputHostOptions};
use typaxis_syntax::{DocumentPackageParser, MachineParseOutcome, PackageValidationPolicy};

const PROFILE: MachinePdfProfileId = MachinePdfProfileId::PARAGRAPH_1;

#[derive(Debug)]
enum TestJson {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

impl fmt::Display for TestJson {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(value) => formatter.write_str(value),
            _ => write!(formatter, "{self:?}"),
        }
    }
}

impl TestJson {
    fn parse(input: &[u8]) -> Result<Self, String> {
        let mut parser = TestJsonParser { input, offset: 0 };
        let value = parser.value()?;
        parser.whitespace();
        if parser.offset == input.len() {
            Ok(value)
        } else {
            Err(format!("trailing JSON at byte {}", parser.offset))
        }
    }

    fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => value.parse().ok(),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    fn has_member(&self, key: &str) -> bool {
        matches!(self, Self::Object(values) if values.contains_key(key))
    }
}

impl Index<&str> for TestJson {
    type Output = Self;

    fn index(&self, key: &str) -> &Self::Output {
        match self {
            Self::Object(values) => values
                .get(key)
                .unwrap_or_else(|| panic!("missing JSON member {key:?}")),
            _ => panic!("cannot index non-object JSON with {key:?}"),
        }
    }
}

impl Index<usize> for TestJson {
    type Output = Self;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            Self::Array(values) => &values[index],
            _ => panic!("cannot index non-array JSON with {index}"),
        }
    }
}

impl PartialEq<str> for TestJson {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == Some(other)
    }
}

impl PartialEq<&str> for TestJson {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialEq<i32> for TestJson {
    fn eq(&self, other: &i32) -> bool {
        self.as_i64() == Some(i64::from(*other))
    }
}

struct TestJsonParser<'a> {
    input: &'a [u8],
    offset: usize,
}

impl TestJsonParser<'_> {
    fn value(&mut self) -> Result<TestJson, String> {
        self.whitespace();
        match self.peek() {
            Some(b'n') => self.literal(b"null", TestJson::Null),
            Some(b't') => self.literal(b"true", TestJson::Bool(true)),
            Some(b'f') => self.literal(b"false", TestJson::Bool(false)),
            Some(b'"') => self.string().map(TestJson::String),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(byte) => Err(format!(
                "unexpected JSON byte {byte:#04x} at byte {}",
                self.offset
            )),
            None => Err("unexpected end of JSON".to_owned()),
        }
    }

    fn array(&mut self) -> Result<TestJson, String> {
        self.expect(b'[')?;
        self.whitespace();
        let mut values = Vec::new();
        if self.consume(b']') {
            return Ok(TestJson::Array(values));
        }
        loop {
            values.push(self.value()?);
            self.whitespace();
            if self.consume(b']') {
                return Ok(TestJson::Array(values));
            }
            self.expect(b',')?;
        }
    }

    fn object(&mut self) -> Result<TestJson, String> {
        self.expect(b'{')?;
        self.whitespace();
        let mut values = BTreeMap::new();
        if self.consume(b'}') {
            return Ok(TestJson::Object(values));
        }
        loop {
            self.whitespace();
            let key = self.string()?;
            self.whitespace();
            self.expect(b':')?;
            let value = self.value()?;
            if values.insert(key.clone(), value).is_some() {
                return Err(format!("duplicate JSON member {key:?}"));
            }
            self.whitespace();
            if self.consume(b'}') {
                return Ok(TestJson::Object(values));
            }
            self.expect(b',')?;
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut output = Vec::new();
        loop {
            let Some(byte) = self.next() else {
                return Err("unterminated JSON string".to_owned());
            };
            match byte {
                b'"' => {
                    return String::from_utf8(output)
                        .map_err(|error| format!("invalid UTF-8 in JSON string: {error}"));
                }
                b'\\' => match self.next() {
                    Some(b'"') => output.push(b'"'),
                    Some(b'\\') => output.push(b'\\'),
                    Some(b'/') => output.push(b'/'),
                    Some(b'b') => output.push(0x08),
                    Some(b'f') => output.push(0x0c),
                    Some(b'n') => output.push(b'\n'),
                    Some(b'r') => output.push(b'\r'),
                    Some(b't') => output.push(b'\t'),
                    Some(b'u') => {
                        let mut scalar = u32::from(self.hex_quad()?);
                        if (0xd800..=0xdbff).contains(&scalar) {
                            self.expect(b'\\')?;
                            self.expect(b'u')?;
                            let low = u32::from(self.hex_quad()?);
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return Err("invalid low surrogate in JSON string".to_owned());
                            }
                            scalar = 0x1_0000 + ((scalar - 0xd800) << 10) + (low - 0xdc00);
                        } else if (0xdc00..=0xdfff).contains(&scalar) {
                            return Err("unpaired low surrogate in JSON string".to_owned());
                        }
                        let character = char::from_u32(scalar)
                            .ok_or_else(|| "invalid Unicode scalar in JSON string".to_owned())?;
                        let mut encoded = [0; 4];
                        output.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
                    }
                    Some(escape) => {
                        return Err(format!("invalid JSON escape byte {escape:#04x}"));
                    }
                    None => return Err("unterminated JSON escape".to_owned()),
                },
                0x00..=0x1f => return Err("unescaped control byte in JSON string".to_owned()),
                _ => output.push(byte),
            }
        }
    }

    fn hex_quad(&mut self) -> Result<u16, String> {
        let mut value = 0u16;
        for _ in 0..4 {
            let digit = match self.next() {
                Some(b'0'..=b'9') => u16::from(self.input[self.offset - 1] - b'0'),
                Some(b'a'..=b'f') => u16::from(self.input[self.offset - 1] - b'a' + 10),
                Some(b'A'..=b'F') => u16::from(self.input[self.offset - 1] - b'A' + 10),
                _ => return Err("invalid JSON Unicode escape".to_owned()),
            };
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<TestJson, String> {
        let start = self.offset;
        while matches!(
            self.peek(),
            Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        ) {
            self.offset += 1;
        }
        let value = std::str::from_utf8(&self.input[start..self.offset]).unwrap();
        value
            .parse::<f64>()
            .map_err(|error| format!("invalid JSON number {value:?}: {error}"))?;
        Ok(TestJson::Number(value.to_owned()))
    }

    fn literal(&mut self, literal: &[u8], value: TestJson) -> Result<TestJson, String> {
        if self.input[self.offset..].starts_with(literal) {
            self.offset += literal.len();
            Ok(value)
        } else {
            Err(format!("invalid JSON literal at byte {}", self.offset))
        }
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.offset += 1;
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), String> {
        if self.consume(expected) {
            Ok(())
        } else {
            Err(format!(
                "expected JSON byte {expected:#04x} at byte {}",
                self.offset
            ))
        }
    }

    fn consume(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.offset).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.offset += 1;
        Some(byte)
    }
}

struct TestTree(PathBuf);

impl TestTree {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "typaxis-mi1-16-{label}-{}-{}",
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

impl Drop for TestTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct FixtureRun {
    _tree: TestTree,
    expected: TestJson,
    result: Result<(), Failure>,
    artifacts: PathBuf,
    job: PathBuf,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf()
}

fn fixture_root(relative: &str) -> PathBuf {
    repository_root()
        .join("samples/machine-package")
        .join(relative)
}

fn read_json(path: &Path) -> TestJson {
    TestJson::parse(&fs::read(path).unwrap())
        .unwrap_or_else(|error| panic!("{} is not JSON: {error}", path.display()))
}

fn json_strings(member: &TestJson) -> Vec<&str> {
    member
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect()
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&from).unwrap();
        if metadata.file_type().is_dir() {
            copy_tree(&from, &to);
        } else if metadata.file_type().is_symlink() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(fs::read_link(&from).unwrap(), &to).unwrap();
            #[cfg(not(unix))]
            panic!("MI1 machine fixtures require a Unix contained-open host");
        } else {
            fs::copy(&from, &to).unwrap();
        }
    }
}

#[cfg(unix)]
fn make_symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(unix)]
fn make_hard_link(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::hard_link(source, target)
}

fn limits_from_expectation(expected: &TestJson) -> Vec<(String, u64)> {
    let arguments = expected["arguments"].as_array().unwrap();
    let mut limits = Vec::new();
    let mut index = 0;
    while index + 1 < arguments.len() {
        let Some(option) = arguments[index].as_str() else {
            index += 1;
            continue;
        };
        if let Some(name) = option.strip_prefix("--max-") {
            limits.push((
                format!("max-{name}"),
                arguments[index + 1].as_str().unwrap().parse().unwrap(),
            ));
            index += 2;
        } else {
            index += 1;
        }
    }
    limits
}

fn build_options(job: &Path, artifacts: &Path, expected: &TestJson) -> BuildPackageOptions {
    fs::create_dir_all(artifacts).unwrap();
    let profile = expected["profile"]
        .as_str()
        .expect("fixture profile is a string")
        .parse()
        .expect("fixture profile is registered");
    let emit_trace = expected_visible(expected).contains("trace");
    BuildPackageOptions {
        package: job.join("document-package.json"),
        package_root: Some(job.to_path_buf()),
        profile,
        output: artifacts.join("output.pdf").into_os_string(),
        trace: emit_trace.then(|| artifacts.join("trace.json")),
        trace_text: emit_trace,
        manifest: Some(artifacts.join("manifest.json")),
        diagnostics: Some(artifacts.join("diagnostics.json")),
        force: false,
        common: CommonOptions {
            resource_roots: vec![job.to_path_buf()],
            limits: limits_from_expectation(expected),
            ..CommonOptions::default()
        },
    }
}

fn copy_fixture(relative: &str, label: &str) -> (TestTree, PathBuf, PathBuf, TestJson) {
    let source = fixture_root(relative);
    let expected = read_json(&source.join("expected.json"));
    let tree = TestTree::new(label);
    let job = tree.path().join("job");
    copy_tree(&source.join("job"), &job);
    let artifacts = tree.path().join("artifacts");
    (tree, job, artifacts, expected)
}

fn failure_exit_code(result: &Result<(), Failure>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => error.kind.exit_code(),
    }
}

fn visible_artifacts(artifacts: &Path) -> BTreeSet<&'static str> {
    [
        ("diagnostics", "diagnostics.json"),
        ("manifest", "manifest.json"),
        ("pdf", "output.pdf"),
        ("trace", "trace.json"),
    ]
    .into_iter()
    .filter_map(|(label, filename)| artifacts.join(filename).exists().then_some(label))
    .collect()
}

fn expected_visible(expected: &TestJson) -> BTreeSet<&str> {
    expected["expected"]["visible_artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect()
}

fn assert_diagnostic_location(expected: &str, actual: &TestJson) {
    if expected == "global" {
        assert!(
            actual.is_null(),
            "expected a global diagnostic, got {actual}"
        );
        return;
    }
    if let Some(offset) = expected.strip_prefix("byte:") {
        assert_eq!(actual["kind"], "package_json");
        assert_eq!(actual["byte_offset"].as_i64(), offset.parse().ok());
        return;
    }
    if let Some(pointer) = expected.strip_prefix("json:") {
        assert_eq!(actual["kind"], "package_json");
        assert_eq!(actual["json_pointer"], pointer);
        return;
    }
    if let Some(span) = expected.strip_prefix("source:") {
        let components: Vec<i64> = span
            .split(&[':', '-'][..])
            .map(|component| component.parse().unwrap())
            .collect();
        assert_eq!(components.len(), 3);
        assert_eq!(actual["kind"], "source");
        assert_eq!(
            actual["source_span"]["source_id"].as_i64(),
            Some(components[0])
        );
        assert_eq!(
            actual["source_span"]["start_byte"].as_i64(),
            Some(components[1])
        );
        assert_eq!(
            actual["source_span"]["end_byte"].as_i64(),
            Some(components[2])
        );
        return;
    }
    panic!("unknown fixture diagnostic location notation {expected:?}");
}

fn assert_fixture_outcome(run: &FixtureRun) {
    let outcome = &run.expected["expected"];
    assert_eq!(
        failure_exit_code(&run.result),
        outcome["exit_code"].as_i64().unwrap() as i32,
        "unexpected runner result for {}: {:?}",
        run.expected["fixture_id"],
        run.result,
    );
    assert_eq!(
        visible_artifacts(&run.artifacts),
        expected_visible(&run.expected),
        "visible artifact set differs for {}",
        run.expected["fixture_id"]
    );

    let expected_code = outcome["primary_code"].as_str();
    if let Some(code) = expected_code {
        let failure = run.result.as_ref().unwrap_err();
        assert!(
            failure.message.starts_with(code),
            "{} did not start with {code}: {}",
            run.expected["fixture_id"],
            failure.message
        );
        let diagnostics = read_json(&run.artifacts.join("diagnostics.json"));
        assert_eq!(diagnostics["contract"], "typaxis.contract/1.2");
        assert_eq!(diagnostics["diagnostics"][0]["code"], code);
        assert_diagnostic_location(
            outcome["location"].as_str().unwrap(),
            &diagnostics["diagnostics"][0]["location"],
        );
    } else if outcome["exit_code"] == 0 {
        let diagnostics = read_json(&run.artifacts.join("diagnostics.json"));
        assert_eq!(diagnostics["diagnostics"].as_array().unwrap().len(), 0);
    }

    if run.artifacts.join("manifest.json").exists() {
        let manifest = read_json(&run.artifacts.join("manifest.json"));
        assert_eq!(
            manifest["status"],
            if outcome["exit_code"] == 0 {
                "built"
            } else {
                "failed"
            }
        );
        assert_eq!(
            manifest["input_profile"],
            run.expected["profile"].as_str().unwrap()
        );
        if outcome["manifest_progress"]["package"] == "none" {
            assert!(manifest["package_input"].is_null());
        } else {
            assert!(manifest["package_input"].is_object());
            if outcome["manifest_progress"]["package"] == "raw" {
                assert!(manifest["package_input"]["canonical_sha256"].is_null());
                assert!(manifest["package_input"]["contract"].is_null());
            } else {
                assert!(manifest["package_input"]["canonical_sha256"].is_string());
                assert_eq!(
                    manifest["package_input"]["contract"].as_str(),
                    run.expected["contract"].as_str()
                );
            }
        }
        if outcome["manifest_progress"]["sources"] == "admitted" {
            assert_eq!(manifest["inputs"].as_array().unwrap().len(), 1);
        } else {
            assert!(manifest["inputs"].as_array().unwrap().is_empty());
        }
        let admitted_resources = manifest["fonts"].as_array().unwrap().len()
            + manifest["images"].as_array().unwrap().len();
        if outcome["manifest_progress"]["resources"] == "admitted" {
            assert_eq!(
                admitted_resources,
                run.expected["resource_hashes"].as_array().unwrap().len()
            );
        } else {
            assert_eq!(admitted_resources, 0);
        }
        if outcome["exit_code"] == 0 {
            assert!(manifest["output"].is_object());
            assert_eq!(
                manifest["output"]["page_count"].as_i64(),
                outcome["page_count"].as_i64()
            );
        } else {
            assert!(manifest["output"].is_null());
            assert!(!run.artifacts.join("output.pdf").exists());
        }
    }
}

fn run_build_fixture(relative: &str) -> FixtureRun {
    let (tree, job, artifacts, expected) = copy_fixture(relative, &relative.replace('/', "-"));
    let options = build_options(&job, &artifacts, &expected);
    let result = run_build_package(options);
    let run = FixtureRun {
        _tree: tree,
        expected,
        result,
        artifacts,
        job,
    };
    assert_fixture_outcome(&run);
    run
}

fn assert_success_fixture(relative: &str) -> FixtureRun {
    let (tree, job, artifacts, expected) = copy_fixture(relative, "success");
    let check_diagnostics = artifacts.join("check-diagnostics.json");
    fs::create_dir_all(&artifacts).unwrap();
    let profile = expected["profile"]
        .as_str()
        .expect("fixture profile is a string")
        .parse()
        .expect("fixture profile is registered");
    run_check_package(CheckPackageOptions {
        package: job.join("document-package.json"),
        package_root: Some(job.clone()),
        profile,
        diagnostics: Some(check_diagnostics.clone()),
        common: CommonOptions {
            resource_roots: vec![job.clone()],
            limits: limits_from_expectation(&expected),
            ..CommonOptions::default()
        },
    })
    .unwrap();
    assert!(read_json(&check_diagnostics)["diagnostics"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(!artifacts.join("output.pdf").exists());
    fs::remove_file(check_diagnostics).unwrap();

    let result = run_build_package(build_options(&job, &artifacts, &expected));
    let run = FixtureRun {
        _tree: tree,
        expected,
        result,
        artifacts,
        job,
    };
    assert_fixture_outcome(&run);
    assert!(fs::read(run.artifacts.join("output.pdf"))
        .unwrap()
        .starts_with(b"%PDF-"));
    run
}

#[test]
fn machine_capabilities_snapshot_is_exact_and_commands_are_public() {
    let snapshot = fs::read(fixture_root("capabilities.json")).unwrap();
    assert_eq!(
        snapshot,
        typaxis_machine_profile::encode_capabilities_canonical(
            HostCapabilityDescriptor::compiled()
        )
        .into_bytes()
    );
    for command in ["build-package", "check-package", "capabilities"] {
        assert!(cli::COMMANDS.contains(&command));
    }
}

#[test]
fn capabilities_preserve_older_profiles_and_publish_closed_m3_profiles() {
    let capabilities = read_json(&fixture_root("capabilities.json"));
    let profiles = capabilities["machine_input"]["profiles"]
        .as_array()
        .unwrap();
    assert_eq!(profiles.len(), 4);
    assert_eq!(
        capabilities["machine_input"]["default_profile"],
        "typaxis.machine-pdf/paragraph-1"
    );
    assert_eq!(
        json_strings(&capabilities["machine_input"]["document_package_contracts"]),
        [
            "typaxis.contract/1.0",
            "typaxis.contract/1.1",
            "typaxis.contract/1.2",
        ]
    );
    let profile = profiles
        .iter()
        .find(|profile| profile["id"] == PROFILE.as_str())
        .expect("paragraph-1 remains advertised");
    assert_eq!(json_strings(&profile["blocks"]), ["heading", "paragraph"]);
    assert_eq!(json_strings(&profile["image_formats"]), Vec::<&str>::new());
    assert!(!profile["footnotes"].as_bool().unwrap());
    assert!(!profile["page_master"]["selection_rules"].as_bool().unwrap());
    let footnote = profiles
        .iter()
        .find(|profile| profile["id"] == MachinePdfProfileId::FOOTNOTE_1.as_str())
        .expect("footnote-1 is advertised");
    assert!(footnote["footnotes"].as_bool().unwrap());
    assert_eq!(
        json_strings(&footnote["page_master"]["optional_frames"]),
        ["footnote"]
    );
    assert!(json_strings(&footnote["inlines"]["kinds"]).contains(&"footnote_reference"));
    for future in [
        "list",
        "figure",
        "table",
        "page_break",
        "math",
        "vector",
        "column",
        "float",
    ] {
        assert!(!json_strings(&profile["blocks"]).contains(&future));
        assert!(!json_strings(&profile["inlines"]["kinds"]).contains(&future));
        assert!(!json_strings(&profile["style_properties"]).contains(&future));
    }
}

#[test]
fn matrix_01_blank_1_1() {
    assert_success_fixture("profiles/paragraph-1/blank-1.1");
}

#[test]
fn matrix_02_blank_1_0() {
    assert_success_fixture("profiles/paragraph-1/blank-1.0");
}

#[test]
fn matrix_03_combined() {
    assert_success_fixture("profiles/paragraph-1/combined");
}

#[test]
fn matrix_m2_basic_combined() {
    assert_success_fixture("profiles/basic-document-1/combined");
}

#[test]
fn matrix_m2_basic_old_contract() {
    run_build_fixture("invalid/basic-document-1-old-contract");
}

#[test]
fn machine_table_basic_profile_rejects_table() {
    run_build_fixture("invalid/basic-document-1-table");
}

#[test]
fn machine_table_paragraph_profile_rejects_table() {
    run_build_fixture("invalid/paragraph-1-table");
}

#[test]
fn machine_table_only() {
    let run = assert_success_fixture("profiles/table-1/only");
    let trace = read_json(&run.artifacts.join("trace.json"));
    let manifest = read_json(&run.artifacts.join("manifest.json"));
    assert_eq!(trace["table_layouts"].as_array().unwrap().len(), 1);
    assert_eq!(manifest["table_layouts"].as_array().unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(run.artifacts.join("trace.json"))
            .unwrap()
            .split("\"table_layouts\":")
            .nth(1),
        fs::read_to_string(run.artifacts.join("manifest.json"))
            .unwrap()
            .split("\"table_layouts\":")
            .nth(1),
    );
}

#[test]
fn machine_table_combined() {
    let run = assert_success_fixture("profiles/table-1/combined");
    let trace = read_json(&run.artifacts.join("trace.json"));
    let manifest = read_json(&run.artifacts.join("manifest.json"));
    let trace_tables = trace["table_layouts"].as_array().unwrap();
    let manifest_tables = manifest["table_layouts"].as_array().unwrap();
    assert_eq!(trace_tables.len(), 1);
    assert_eq!(manifest_tables.len(), 1);
    let table = &manifest_tables[0];
    assert_eq!(table["page_count"], 2);
    assert_eq!(table["target_page_start"], 1);
    let headers = table["header_occurrences"].as_array().unwrap();
    assert_eq!(headers.len(), 2);
    assert_eq!(headers[0]["repetition_index"], 0);
    assert_eq!(headers[0]["target_page_index"], 1);
    assert_eq!(headers[1]["repetition_index"], 1);
    assert_eq!(headers[1]["target_page_index"], 2);
    assert_eq!(
        trace_tables[0]["selected_layout_sha256"].as_str(),
        table["selected_layout_sha256"].as_str()
    );
    assert_eq!(
        trace_tables[0]["flow_registry_sha256"].as_str(),
        table["flow_registry_sha256"].as_str()
    );
}

#[test]
fn machine_table_policy_rejections() {
    for fixture in [
        "invalid/table-1-decoration",
        "invalid/table-1-inapplicable-style",
        "invalid/table-1-old-contract",
    ] {
        run_build_fixture(fixture);
    }
}

#[test]
fn machine_table_profile_retains_basic_only_behavior_without_table_facts() {
    let (tree, job, artifacts, expected) =
        copy_fixture("profiles/basic-document-1/combined", "table-basic-only");
    fs::create_dir_all(&artifacts).unwrap();
    let mut options = build_options(&job, &artifacts, &expected);
    options.profile = MachinePdfProfileId::TABLE_1;
    options.trace = Some(artifacts.join("trace.json"));
    options.trace_text = true;
    run_build_package(options).unwrap();
    let trace = read_json(&artifacts.join("trace.json"));
    let manifest = read_json(&artifacts.join("manifest.json"));
    assert!(trace["table_layouts"].as_array().unwrap().is_empty());
    assert!(manifest["table_layouts"].as_array().unwrap().is_empty());
    assert_eq!(
        manifest["input_profile"],
        MachinePdfProfileId::TABLE_1.as_str()
    );
    drop(tree);
}

#[test]
fn machine_footnote_zero() {
    let run = assert_success_fixture("profiles/footnote-1/zero");
    let trace = read_json(&run.artifacts.join("trace.json"));
    let manifest = read_json(&run.artifacts.join("manifest.json"));
    for facts in [&trace["footnote_layout"], &manifest["footnote_layout"]] {
        assert_eq!(facts["algorithm"], "typaxis.footnote-manifest/1");
        let pages = facts["pages"].as_array().unwrap();
        assert_eq!(pages.len(), 2);
        for page in pages {
            assert!(page["ordered_footnote_ids"].as_array().unwrap().is_empty());
            assert!(page["flows"].as_array().unwrap().is_empty());
            assert_eq!(page["reservation"], 0);
        }
    }
    let selected_pass = trace["passes"].as_array().unwrap().last().unwrap();
    for page in selected_pass["state"]["pages"].as_array().unwrap() {
        assert!(page["footnote_ids"].as_array().unwrap().is_empty());
        let frames = page["frames"].as_array().unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["kind"], "body");
    }
}

#[test]
fn machine_footnote_combined() {
    let run = assert_success_fixture("profiles/footnote-1/combined");
    let trace = read_json(&run.artifacts.join("trace.json"));
    let manifest = read_json(&run.artifacts.join("manifest.json"));
    let trace_facts = &trace["footnote_layout"];
    let manifest_facts = &manifest["footnote_layout"];
    assert_eq!(trace_facts["algorithm"], "typaxis.footnote-manifest/1");
    assert_eq!(manifest_facts["algorithm"], "typaxis.footnote-manifest/1");
    assert_eq!(
        trace_facts["body_layout_sha256"].as_str(),
        trace["result"]["final_fingerprint"].as_str()
    );
    assert_eq!(
        manifest_facts["body_layout_sha256"].as_str(),
        manifest["layout"]["final_fingerprint"].as_str()
    );
    for member in [
        "body_layout_sha256",
        "paint_sha256",
        "profile_sha256",
        "registry_sha256",
        "selected_layout_sha256",
    ] {
        assert_eq!(
            trace_facts[member].as_str(),
            manifest_facts[member].as_str()
        );
    }
    let pages = manifest_facts["pages"].as_array().unwrap();
    assert_eq!(pages.len(), 3);
    let trace_pages = trace_facts["pages"].as_array().unwrap();
    let selected_pages = trace["passes"].as_array().unwrap().last().unwrap()["state"]["pages"]
        .as_array()
        .unwrap();
    assert_eq!(trace_pages.len(), pages.len());
    assert_eq!(selected_pages.len(), pages.len());
    let mut prior_body_position = None;
    for ((manifest_page, trace_page), selected_page) in
        pages.iter().zip(trace_pages).zip(selected_pages)
    {
        assert_eq!(
            manifest_page["body_continuation_position"].as_i64(),
            trace_page["body_continuation_position"].as_i64()
        );
        assert_eq!(
            manifest_page["body_continuation_terminal"].as_bool(),
            trace_page["body_continuation_terminal"].as_bool()
        );
        let body_position = manifest_page["body_continuation_position"]
            .as_i64()
            .unwrap();
        if let Some(prior) = prior_body_position {
            assert!(body_position >= prior);
        }
        prior_body_position = Some(body_position);

        let reservation = manifest_page["reservation"].as_i64().unwrap();
        let frames = selected_page["frames"].as_array().unwrap();
        assert_eq!(frames[0]["kind"], "body");
        if reservation == 0 {
            assert_eq!(frames.len(), 1);
        } else {
            assert_eq!(frames.len(), 2);
            assert_eq!(frames[1]["kind"], "footnote");
            assert_eq!(frames[1]["bounds"]["height"].as_i64(), Some(reservation));
            assert_eq!(
                frames[1]["bounds"]["x"].as_i64(),
                frames[0]["bounds"]["x"].as_i64()
            );
            assert_eq!(
                frames[1]["bounds"]["width"].as_i64(),
                frames[0]["bounds"]["width"].as_i64()
            );
            assert_eq!(
                frames[1]["bounds"]["y"].as_i64().unwrap()
                    + frames[1]["bounds"]["height"].as_i64().unwrap(),
                frames[0]["bounds"]["y"].as_i64().unwrap()
                    + frames[0]["bounds"]["height"].as_i64().unwrap()
            );
        }
    }
    assert!(pages.last().unwrap()["body_continuation_terminal"]
        .as_bool()
        .unwrap());
    assert_eq!(json_strings(&pages[0]["ordered_footnote_ids"]), ["z", "a"]);
    assert_eq!(pages[0]["flows"].as_array().unwrap().len(), 2);
    assert!(pages[0]["flows"][0]["carries_out"].as_bool().unwrap());
    assert_eq!(pages[0]["flows"][0]["before_fragment"], 0);
    assert_eq!(pages[0]["flows"][0]["after_fragment"], 1);
    assert_eq!(json_strings(&pages[1]["ordered_footnote_ids"]), ["z"]);
    assert_eq!(pages[1]["flows"].as_array().unwrap().len(), 1);
    assert_eq!(pages[1]["flows"][0]["incoming_source_page"], 0);
    assert_eq!(pages[1]["flows"][0]["before_fragment"], 1);
    assert_eq!(pages[1]["flows"][0]["after_fragment"], 3);
    assert!(pages[1]["flows"][0]["carries_out"].as_bool().unwrap());
    assert_eq!(json_strings(&pages[2]["ordered_footnote_ids"]), ["z"]);
    assert_eq!(pages[2]["flows"].as_array().unwrap().len(), 1);
    assert_eq!(pages[2]["flows"][0]["incoming_source_page"], 1);
    assert_eq!(pages[2]["flows"][0]["before_fragment"], 3);
    assert_eq!(pages[2]["flows"][0]["after_fragment"], 4);
    assert!(!pages[2]["flows"][0]["carries_out"].as_bool().unwrap());
    let selected_pass = trace["passes"].as_array().unwrap().last().unwrap();
    assert_eq!(
        json_strings(&selected_pass["state"]["pages"][0]["footnote_ids"]),
        ["a", "z"]
    );
    assert_eq!(
        json_strings(&selected_pass["state"]["pages"][1]["footnote_ids"]),
        ["z"]
    );
    assert_eq!(
        json_strings(&selected_pass["state"]["pages"][2]["footnote_ids"]),
        ["z"]
    );
    assert!(!trace.has_member("table_layouts"));
    assert!(!manifest.has_member("table_layouts"));
    let pdf = fs::read(run.artifacts.join("output.pdf")).unwrap();
    assert!(pdf
        .windows(b"/Annots".len())
        .any(|bytes| bytes == b"/Annots"));
    assert!(pdf
        .windows(b"6E6F74652D61".len())
        .any(|bytes| bytes == b"6E6F74652D61"));
}

#[test]
fn machine_footnote_rejects_a_marker_only_first_line() {
    let (tree, job, artifacts, expected) =
        copy_fixture("profiles/footnote-1/combined", "marker-only-line");
    fs::create_dir_all(&artifacts).unwrap();
    let package_path = job.join("document-package.json");
    let package = fs::read_to_string(&package_path).unwrap();
    let anchor = concat!(
        "{\"anchor_id\":\"note-a\",\"kind\":\"anchor\",\"node_id\":30,",
        "\"span\":{\"end_byte\":0,\"source_id\":0,\"start_byte\":0}}"
    );
    let hard_break = concat!(
        "{\"kind\":\"hard_break\",\"node_id\":30,",
        "\"span\":{\"end_byte\":0,\"source_id\":0,\"start_byte\":0}}"
    );
    let mutated = package.replacen(anchor, hard_break, 1).replacen(
        "\"target\":{\"anchor_id\":\"note-a\",\"kind\":\"internal\"}",
        "\"target\":{\"kind\":\"uri\",\"uri\":\"https://example.test/note\"}",
        1,
    );
    assert_ne!(mutated, package);
    fs::write(&package_path, mutated).unwrap();

    let error = run_build_package(build_options(&job, &artifacts, &expected)).unwrap_err();
    assert_eq!(error.kind, FailureKind::Input);
    assert!(error.message.contains("L5100:"));
    assert!(error.message.contains("marker and first source content"));
    assert!(!artifacts.join("output.pdf").exists());
    drop(tree);
}

#[test]
fn machine_footnote_old_contract_rejected() {
    run_build_fixture("invalid/footnote-1-old-contract");
}

#[test]
fn machine_older_profile_artifacts_do_not_gain_table_projection_members() {
    for fixture in [
        "profiles/paragraph-1/combined",
        "profiles/basic-document-1/combined",
    ] {
        let (tree, job, artifacts, expected) = copy_fixture(fixture, "old-no-table-facts");
        fs::create_dir_all(&artifacts).unwrap();
        let mut options = build_options(&job, &artifacts, &expected);
        options.trace = Some(artifacts.join("trace.json"));
        options.trace_text = true;
        run_build_package(options).unwrap();
        let trace = read_json(&artifacts.join("trace.json"));
        let manifest = read_json(&artifacts.join("manifest.json"));
        assert!(!trace.has_member("table_layouts"));
        assert!(!manifest.has_member("table_layouts"));
        assert!(!trace.has_member("footnote_layout"));
        assert!(!manifest.has_member("footnote_layout"));
        drop(tree);
    }
}

#[test]
fn matrix_04_package_envelope() {
    for fixture in ["p1100-bom", "p1100-nul", "p1100-trailing-token"] {
        run_build_fixture(&format!("invalid/{fixture}"));
    }
}

#[test]
fn matrix_05_json_grammar() {
    for fixture in ["p1101-malformed-json", "p1101-duplicate-escaped-key"] {
        run_build_fixture(&format!("invalid/{fixture}"));
    }
}

#[test]
fn matrix_06_typed_members() {
    for fixture in [
        "p1102-unknown-field",
        "p1102-missing-field",
        "p1102-float-integer",
        "p1102-range",
    ] {
        run_build_fixture(&format!("invalid/{fixture}"));
    }
}

#[test]
fn matrix_07_unknown_contract() {
    run_build_fixture("invalid/p1103-unknown-contract");
}

#[test]
fn matrix_08_package_bytes() {
    run_build_fixture("invalid/i9100-package-bytes-exact");
    run_build_fixture("invalid/i9100-package-bytes-max-plus-one");
}

#[test]
fn matrix_09_json_depth() {
    run_build_fixture("invalid/i9101-depth-exact");
    run_build_fixture("invalid/i9101-depth-max-plus-one");
}

#[test]
fn matrix_10_source_profile() {
    run_build_fixture("invalid/p1110-multiple-sources");
    run_build_fixture("invalid/p1110-nonzero-entry");
}

#[cfg(unix)]
#[test]
fn matrix_11_source_path() {
    run_build_fixture("invalid/p1111-unsafe-source");
    run_build_fixture("scenarios/i9112-source-symlink");
}

#[test]
fn matrix_12_package_root() {
    let (tree, job, artifacts, expected) =
        copy_fixture("scenarios/usage-package-outside-root", "outside-root");
    let mut options = build_options(&job, &artifacts, &expected);
    options.package_root = Some(job.join("root"));
    let result = run_build_package(options);
    let run = FixtureRun {
        _tree: tree,
        expected,
        result,
        artifacts,
        job,
    };
    assert_fixture_outcome(&run);
}

#[cfg(unix)]
#[test]
fn matrix_13_package_open() {
    run_build_fixture("scenarios/i9111-package-symlink");
}

#[test]
fn matrix_14_source_identity() {
    run_build_fixture("invalid/p1112-source-length");
    run_build_fixture("invalid/p1112-source-hash");
}

fn assert_admitted_mutation_is_detected(job: &Path, target: &Path) {
    let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    let options = MachineInputHostOptions::new(
        HostPath::new(job.join("document-package.json")).unwrap(),
        Some(HostPath::new(job.to_path_buf()).unwrap()),
    );
    let (session, raw) = HostMachineInputSession::open(options, &limits).unwrap();
    let decoded = session
        .decode_and_bind(
            &raw,
            &StrictDocumentPackageDecoder::new(),
            &DocumentPackageDecodePolicy::new(&limits),
        )
        .unwrap();
    let sources = session.admit_sources(&decoded, &limits).unwrap();
    let token = session
        .finish(raw, decoded, sources)
        .unwrap()
        .read_ledger_token()
        .unwrap();
    let mut bytes = fs::read(target).unwrap();
    bytes.push(b' ');
    let replacement = target.with_extension("replacement");
    fs::write(&replacement, bytes).unwrap();
    fs::rename(replacement, target).unwrap();
    let failure = MachineWriteTargets::Diagnostics(None)
        .validate(&token)
        .unwrap_err();
    assert_eq!(failure.kind, FailureKind::Io);
    assert!(failure.message.starts_with("I9113:"));
}

#[cfg(unix)]
#[test]
fn matrix_15_stable_read() {
    for (fixture, target) in [
        ("scenarios/i9113-package-mutation", "document-package.json"),
        ("scenarios/i9113-source-mutation", "sources/blank.json"),
    ] {
        let (tree, job, _, expected) = copy_fixture(fixture, "stable-read");
        assert_eq!(expected["expected"]["primary_code"], "I9113");
        let path = job.join(target);
        assert_admitted_mutation_is_detected(&job, &path);
        drop(tree);
    }
}

#[test]
fn matrix_16_identity_map() {
    run_build_fixture("invalid/p1112-identity-map");
}

#[test]
fn matrix_17_unsupported_content() {
    let run = run_build_fixture("invalid/l5100-unsupported-content");
    let diagnostics = read_json(&run.artifacts.join("diagnostics.json"));
    let diagnostics = diagnostics["diagnostics"].as_array().unwrap();
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic["code"] == "L5100"));
    let pointers: Vec<&str> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic["location"]["json_pointer"].as_str().unwrap())
        .collect();
    assert_eq!(
        pointers,
        [
            "/document/blocks/0/children/1",
            "/document/blocks/0/children/2",
            "/document/blocks/0/children/3",
            "/document/blocks/0/children/4",
            "/document/blocks/1",
            "/document/footnotes/0",
        ]
    );
}

#[test]
fn matrix_18_unsupported_style() {
    let run = run_build_fixture("invalid/l5101-unsupported-style-master");
    let diagnostics = read_json(&run.artifacts.join("diagnostics.json"));
    assert!(diagnostics["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .all(|diagnostic| diagnostic["code"] == "L5101"));
}

#[test]
fn matrix_19_unsupported_image() {
    let run = run_build_fixture("invalid/r7100-unsupported-image");
    assert!(!run.job.join("images/missing.png").exists());
    let manifest = read_json(&run.artifacts.join("manifest.json"));
    assert!(manifest["images"].as_array().unwrap().is_empty());
}

#[test]
fn matrix_20_host_unavailable() {
    let (tree, job, artifacts, expected) =
        copy_fixture("scenarios/i9110-host-unavailable", "host-unavailable");
    fs::remove_file(job.join("document-package.json")).unwrap();
    let options = build_options(&job, &artifacts, &expected);
    let result = run_build_package_with_host(options, MachineHostPreflight::Unavailable);
    let run = FixtureRun {
        _tree: tree,
        expected,
        result,
        artifacts,
        job,
    };
    assert_fixture_outcome(&run);
}

#[test]
fn matrix_21_unknown_profile() {
    let fixture = fixture_root("scenarios/usage-unknown-profile");
    let expected = read_json(&fixture.join("expected.json"));
    let arguments: Vec<OsString> = expected["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|argument| OsString::from(argument.as_str().unwrap()))
        .collect();
    let error = cli::parse_build_package(arguments).unwrap_err();
    assert!(error.to_string().contains("unknown machine PDF profile"));
    assert_eq!(expected["expected"]["exit_code"], 2);
}

#[test]
fn matrix_22_blank_1_2() {
    assert_success_fixture("profiles/paragraph-1/blank-1.2");
}

#[test]
fn command_diagnostic_budget_retains_primary_and_omission_note() {
    let run = run_build_fixture("scenarios/diagnostics-max-plus-one");
    let diagnostics = read_json(&run.artifacts.join("diagnostics.json"));
    let values = diagnostics["diagnostics"].as_array().unwrap();
    assert_eq!(values.len(), typaxis_diagnostics::MAX_MACHINE_DIAGNOSTICS);
    assert_eq!(values[0]["code"], "L5100");
    assert_eq!(values[0]["severity"], "error");
    assert!(values.last().unwrap()["notes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|note| note["message"].as_str().unwrap().contains("omitted")));
}

#[test]
fn receipt_sessions_are_not_interchangeable() {
    let (first_tree, first_job, _, _) = copy_fixture("scenarios/receipt-swap", "receipt-first");
    let (second_tree, second_job, _, _) = copy_fixture("scenarios/receipt-swap", "receipt-second");
    let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    let options = |job: &Path| {
        MachineInputHostOptions::new(
            HostPath::new(job.join("document-package.json")).unwrap(),
            Some(HostPath::new(job.to_path_buf()).unwrap()),
        )
    };
    let (first, first_raw) = HostMachineInputSession::open(options(&first_job), &limits).unwrap();
    let (second, second_raw) =
        HostMachineInputSession::open(options(&second_job), &limits).unwrap();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    assert!(first
        .decode_and_bind(&second_raw, &StrictDocumentPackageDecoder::new(), &policy)
        .is_err());
    let first_decoded = first
        .decode_and_bind(&first_raw, &StrictDocumentPackageDecoder::new(), &policy)
        .unwrap();
    let second_decoded = second
        .decode_and_bind(&second_raw, &StrictDocumentPackageDecoder::new(), &policy)
        .unwrap();
    assert!(first.admit_sources(&second_decoded, &limits).is_err());
    let first_sources = first.admit_sources(&first_decoded, &limits).unwrap();
    let second_sources = second.admit_sources(&second_decoded, &limits).unwrap();
    assert!(first
        .finish(first_raw, first_decoded, second_sources)
        .is_err());
    assert!(second
        .finish(second_raw, second_decoded, first_sources)
        .is_err());
    drop((first_tree, second_tree));
}

#[test]
fn canonical_round_trip_relations_hold() {
    let root = fixture_root("scenarios/round-trip/job");
    let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let decoder = StrictDocumentPackageDecoder::new();
    let canonical = decoder
        .decode(
            &fs::read(root.join("document-package.json")).unwrap(),
            &policy,
        )
        .unwrap();
    let equivalent = decoder
        .decode(&fs::read(root.join("equivalent.json")).unwrap(), &policy)
        .unwrap();
    let semantic = decoder
        .decode(&fs::read(root.join("semantic.json")).unwrap(), &policy)
        .unwrap();
    assert_ne!(canonical.raw_sha256(), equivalent.raw_sha256());
    assert_eq!(
        canonical.canonical_jcs_sha256(),
        equivalent.canonical_jcs_sha256()
    );
    assert_ne!(
        canonical.canonical_jcs_sha256(),
        semantic.canonical_jcs_sha256()
    );
    let document_fingerprint = |filename: &str| {
        let options = MachineInputHostOptions::new(
            HostPath::new(root.join(filename)).unwrap(),
            Some(HostPath::new(root.clone()).unwrap()),
        );
        let (session, raw) = HostMachineInputSession::open(options, &limits).unwrap();
        let decoded = session
            .decode_and_bind(&raw, &StrictDocumentPackageDecoder::new(), &policy)
            .unwrap();
        let sources = session.admit_sources(&decoded, &limits).unwrap();
        let admitted = session.finish(raw, decoded, sources).unwrap();
        let allowed_uri_schemes = ["http".to_owned()];
        let validation = PackageValidationPolicy::new(&limits, &allowed_uri_schemes).unwrap();
        match DocumentPackageParser::new().parse(admitted, &validation) {
            MachineParseOutcome::Parsed { package } => {
                package.package().epoch_identity().document()
            }
            MachineParseOutcome::Failed { failure, .. } => {
                panic!("{filename} failed semantic validation: {failure}")
            }
        }
    };
    let canonical_fingerprint = document_fingerprint("document-package.json");
    assert_eq!(
        canonical_fingerprint,
        document_fingerprint("equivalent.json")
    );
    let reencoded = DocumentPackageEncoder::default()
        .to_jcs_vec(canonical.wire())
        .unwrap();
    let round_trip = decoder.decode(&reencoded, &policy).unwrap();
    assert_eq!(
        canonical.canonical_jcs_sha256(),
        round_trip.canonical_jcs_sha256()
    );

    let (tree, job, artifacts, expected) =
        copy_fixture("scenarios/round-trip", "dump-build-round-trip");
    let config = EffectiveConfig::new(
        false,
        PdfStreamCompression::Flate,
        vec![ConfigResourceRoot::ProjectRoot],
        ["http", "https", "mailto", "tel"]
            .map(str::to_owned)
            .to_vec(),
        EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0").unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    let source_package = pipeline::load_package(&job.join("input.tsf"), &config).unwrap();
    let source_fingerprint = source_package.epoch_identity().document();
    let dumped_path = job.join("dumped.json");
    let mut dumped = fs::File::create(&dumped_path).unwrap();
    artifacts::write_document_package_json(&source_package, config.limits(), &mut dumped).unwrap();
    drop(dumped);

    let options = MachineInputHostOptions::new(
        HostPath::new(dumped_path.clone()).unwrap(),
        Some(HostPath::new(job.clone()).unwrap()),
    );
    let (session, raw) = HostMachineInputSession::open(options, config.limits()).unwrap();
    let decoded = session
        .decode_and_bind(
            &raw,
            &StrictDocumentPackageDecoder::new(),
            &DocumentPackageDecodePolicy::new(config.limits()),
        )
        .unwrap();
    let sources = session.admit_sources(&decoded, config.limits()).unwrap();
    let admitted = session.finish(raw, decoded, sources).unwrap();
    let policy =
        PackageValidationPolicy::new(config.limits(), config.allowed_uri_schemes()).unwrap();
    let machine_package = match DocumentPackageParser::new().parse(admitted, &policy) {
        MachineParseOutcome::Parsed { package } => package,
        MachineParseOutcome::Failed { failure, .. } => panic!("dumped package failed: {failure}"),
    };
    assert_eq!(
        source_fingerprint,
        machine_package.package().epoch_identity().document()
    );
    let mut options = build_options(&job, &artifacts, &expected);
    options.package = dumped_path;
    run_build_package(options).unwrap();
    assert!(artifacts.join("output.pdf").is_file());
    drop(tree);
}

#[test]
fn all_machine_targets_reject_input_aliases() {
    let (tree, job, artifacts, expected) = copy_fixture("scenarios/alias-race", "aliases");
    let candidates = [
        job.join("document-package.json"),
        job.join("sources/blank.json"),
    ];
    for candidate in candidates {
        for role in 0..4 {
            let mut options =
                build_options(&job, &artifacts.join(format!("role-{role}")), &expected);
            match role {
                0 => options.output = candidate.clone().into_os_string(),
                1 => options.trace = Some(candidate.clone()),
                2 => options.manifest = Some(candidate.clone()),
                3 => options.diagnostics = Some(candidate.clone()),
                _ => unreachable!(),
            }
            let error = run_build_package(options).unwrap_err();
            assert!(matches!(error.kind, FailureKind::Usage | FailureKind::Io));
        }
    }

    for (left, right) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
        let run_artifacts = artifacts.join(format!("pair-{left}-{right}"));
        let mut options = build_options(&job, &run_artifacts, &expected);
        let alias = run_artifacts.join("same-target");
        let mut targets = [
            PathBuf::from(&options.output),
            options
                .trace
                .clone()
                .unwrap_or_else(|| run_artifacts.join("trace.json")),
            options.manifest.clone().unwrap(),
            options.diagnostics.clone().unwrap(),
        ];
        targets[left] = alias.clone();
        targets[right] = alias;
        options.output = targets[0].clone().into_os_string();
        options.trace = Some(targets[1].clone());
        options.manifest = Some(targets[2].clone());
        options.diagnostics = Some(targets[3].clone());
        assert!(run_build_package(options).is_err());
    }

    #[cfg(unix)]
    for (label, make_alias) in [
        (
            "symlink",
            make_symlink as fn(&Path, &Path) -> std::io::Result<()>,
        ),
        (
            "hard-link",
            make_hard_link as fn(&Path, &Path) -> std::io::Result<()>,
        ),
    ] {
        let run_artifacts = artifacts.join(label);
        fs::create_dir_all(&run_artifacts).unwrap();
        let alias = run_artifacts.join("output.pdf");
        make_alias(&job.join("sources/blank.json"), &alias).unwrap();
        let mut options = build_options(&job, &run_artifacts, &expected);
        options.output = alias.into_os_string();
        let error = run_build_package(options).unwrap_err();
        assert!(matches!(error.kind, FailureKind::Usage | FailureKind::Io));
        assert_eq!(fs::read(job.join("sources/blank.json")).unwrap(), b"");
    }
    drop(tree);
}

#[test]
fn publication_failure_artifact_sets_are_typed() {
    let (tree, job, artifacts, expected) = copy_fixture("scenarios/partial-failure", "publication");

    let diagnostics_case = artifacts.join("diagnostics-failure");
    fs::create_dir_all(diagnostics_case.join("diagnostics.json")).unwrap();
    let mut options = build_options(&job, &diagnostics_case, &expected);
    options.package = fixture_root("invalid/l5100-unsupported-content/job/document-package.json");
    options.package_root = Some(fixture_root("invalid/l5100-unsupported-content/job"));
    options.common.resource_roots = vec![fixture_root("invalid/l5100-unsupported-content/job")];
    let error = run_build_package(options).unwrap_err();
    assert_eq!(error.kind, FailureKind::Io);
    assert!(error
        .message
        .contains("diagnostics publication also failed"));
    assert!(diagnostics_case.join("manifest.json").is_file());
    assert!(!diagnostics_case.join("output.pdf").exists());

    let pdf_case = artifacts.join("pdf-failure");
    fs::create_dir_all(pdf_case.join("output.pdf")).unwrap();
    let error = run_build_package(build_options(&job, &pdf_case, &expected)).unwrap_err();
    assert_eq!(error.kind, FailureKind::Io);
    assert!(pdf_case.join("manifest.json").is_file());
    assert!(pdf_case.join("output.pdf").is_dir());

    let manifest_case = artifacts.join("manifest-failure");
    fs::create_dir_all(manifest_case.join("manifest.json")).unwrap();
    let error = run_build_package(build_options(&job, &manifest_case, &expected)).unwrap_err();
    assert_eq!(error.kind, FailureKind::Io);
    assert!(manifest_case.join("output.pdf").is_file());
    assert!(manifest_case.join("manifest.json").is_dir());
    drop(tree);
}
