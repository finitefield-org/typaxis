#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        loop {
            let ordinal = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("typaxis-cli-e2e-{}-{ordinal}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("cannot create test directory: {error}"),
            }
        }
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("test directory cleanup must succeed");
    }
}

fn cli_command(directory: &Path, arguments: &[&OsStr]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_typaxis"));
    command.current_dir(directory).args(arguments);
    for (key, _) in std::env::vars_os() {
        if key.to_str().is_some_and(|key| key.starts_with("TYPAXIS_")) {
            command.env_remove(key);
        }
    }
    command
}

fn run(directory: &Path, arguments: &[&OsStr]) -> Output {
    cli_command(directory, arguments)
        .output()
        .expect("CLI process must start")
}

fn strings<'a>(values: &'a [&'a str]) -> Vec<&'a OsStr> {
    values.iter().map(OsStr::new).collect()
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root must be available to integration tests")
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ManifestOutputFacts {
    sink: String,
    bytes: u64,
    sha256: String,
    page_count: u64,
    pdf_object_count: u64,
}

fn manifest_output_facts(manifest: &str) -> ManifestOutputFacts {
    let output = manifest
        .split_once("\"output\":{")
        .expect("built manifest must contain an output object")
        .1
        .split_once('}')
        .expect("manifest output object must terminate")
        .0;
    let number = |name: &str| {
        manifest_object_member(output, name)
            .parse::<u64>()
            .expect("manifest output number must be an unsigned integer")
    };
    ManifestOutputFacts {
        sink: manifest_object_member(output, "sink")
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .expect("manifest output sink must be a JSON string")
            .to_owned(),
        bytes: number("bytes"),
        sha256: manifest_object_member(output, "sha256")
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .expect("manifest output hash must be a JSON string")
            .to_owned(),
        page_count: number("page_count"),
        pdf_object_count: number("pdf_object_count"),
    }
}

fn manifest_object_member<'a>(object: &'a str, name: &str) -> &'a str {
    let marker = format!("\"{name}\":");
    object
        .split_once(&marker)
        .unwrap_or_else(|| panic!("manifest output must contain `{name}`"))
        .1
        .split(',')
        .next()
        .expect("manifest output member must contain a value")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(64);
    for byte in typaxis_core::sha256(bytes) {
        write!(&mut hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}

#[cfg(any(target_os = "android", target_os = "linux"))]
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
    build_test_sfnt(vec![
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

#[cfg(any(target_os = "android", target_os = "linux"))]
fn build_test_sfnt(mut tables: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
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
        output[record + 4..record + 8].copy_from_slice(&test_sfnt_checksum(bytes).to_be_bytes());
        output[record + 8..record + 12].copy_from_slice(&(offset as u32).to_be_bytes());
        output[record + 12..record + 16].copy_from_slice(&(bytes.len() as u32).to_be_bytes());
        output[offset..offset + bytes.len()].copy_from_slice(bytes);
        if tag == b"head" {
            head_adjustment = Some(offset + 8);
        }
        offset = (offset + bytes.len() + 3) & !3;
    }
    if let Some(offset) = head_adjustment {
        let adjustment = 0xB1B0_AFBAu32.wrapping_sub(test_sfnt_checksum(&output));
        output[offset..offset + 4].copy_from_slice(&adjustment.to_be_bytes());
    }
    output
}

#[cfg(any(target_os = "android", target_os = "linux"))]
fn test_sfnt_checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |checksum, chunk| {
        let mut word = [0; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        checksum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn pdf_page_count(pdf: &[u8]) -> u64 {
    let pages_type = byte_offset(pdf, b"/Type /Pages");
    let object_body = pdf[..pages_type]
        .windows(b" obj\n".len())
        .rposition(|window| window == b" obj\n")
        .map(|offset| offset + b" obj\n".len())
        .expect("page-tree type must occur inside an indirect object");
    decimal_after(&pdf[object_body..pages_type], b"/Count ")
}

fn pdf_object_count(pdf: &[u8]) -> u64 {
    decimal_after(pdf, b"\nxref\n0 ")
        .checked_sub(1)
        .expect("classic xref must include the free object zero")
}

fn decimal_after(bytes: &[u8], marker: &[u8]) -> u64 {
    let remainder = bytes_after(bytes, marker);
    let digits = remainder
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .copied()
        .collect::<Vec<_>>();
    assert!(!digits.is_empty(), "PDF marker must be followed by digits");
    std::str::from_utf8(&digits)
        .expect("PDF decimal must be ASCII")
        .parse()
        .expect("PDF decimal must fit in u64")
}

fn bytes_after<'a>(bytes: &'a [u8], marker: &[u8]) -> &'a [u8] {
    let offset = byte_offset(bytes, marker);
    &bytes[offset + marker.len()..]
}

fn byte_offset(bytes: &[u8], marker: &[u8]) -> usize {
    bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .unwrap_or_else(|| panic!("PDF must contain marker {:?}", marker))
}

#[cfg(any(target_os = "android", target_os = "linux"))]
fn assert_artifact_glyph_is_painted(pdf: &[u8]) {
    let marked = bytes_after(pdf, b"/Artifact << /ActualText <> >> BDC\n");
    let body = &marked[..byte_offset(marked, b"EMC\n")];
    assert!(body.windows(b" Tj\n".len()).any(|bytes| bytes == b" Tj\n"));
    assert!(!body
        .windows(b"<0000> Tj".len())
        .any(|bytes| bytes == b"<0000> Tj"));
}

#[test]
fn global_actions_and_exit_code_classes_are_observable() {
    let directory = TestDirectory::new();
    fs::write(directory.path().join("invalid.tsf"), b"not-a-record\n").unwrap();
    fs::write(directory.path().join("large.tsf"), b"paragraph\n").unwrap();
    fs::write(directory.path().join("invalid.toml"), b"strict = true\n").unwrap();
    fs::File::create(directory.path().join("oversized.toml"))
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();

    let help = run(directory.path(), &strings(&["--help"]));
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout)
        .unwrap()
        .contains("Typaxis reference typesetting CLI"));

    let version = run(directory.path(), &strings(&["--version"]));
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap(),
        format!("typaxis {}\n", env!("CARGO_PKG_VERSION"))
    );

    assert_eq!(
        run(directory.path(), &strings(&["unknown-command"]))
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        run(directory.path(), &strings(&["check", "invalid.tsf"]))
            .status
            .code(),
        Some(1)
    );
    // The default configured project root and this explicit root are aliases.
    // Root admission must reject them before document contents can influence
    // whether a resource-opening session happens to be constructed.
    assert_eq!(
        run(
            directory.path(),
            &strings(&["check", "invalid.tsf", "--resource-root", "."]),
        )
        .status
        .code(),
        Some(2)
    );
    assert_eq!(
        run(directory.path(), &strings(&["check", "missing.tsf"]))
            .status
            .code(),
        Some(3)
    );
    assert_eq!(
        run(
            directory.path(),
            &strings(&["check", "large.tsf", "--max-source-bytes", "1"]),
        )
        .status
        .code(),
        Some(5)
    );
    for (option, value) in [
        ("--max-ast-nesting-depth", "65"),
        ("--max-fonts", "308915777"),
    ] {
        let invalid_limit = run(
            directory.path(),
            &strings(&["check", "large.tsf", option, value]),
        );
        assert_eq!(invalid_limit.status.code(), Some(5));
        assert!(String::from_utf8_lossy(&invalid_limit.stderr).contains("P1001:"));
    }
    assert_eq!(
        run(
            directory.path(),
            &strings(&["check", "large.tsf", "--config", "invalid.toml"]),
        )
        .status
        .code(),
        Some(2)
    );
    for (name, value) in [
        ("TYPAXIS_UNKNOWN", "true"),
        ("TYPAXIS_LIMITS__MAX_PAGES", "\"not-an-integer\""),
    ] {
        let invalid_environment = cli_command(directory.path(), &strings(&["check", "large.tsf"]))
            .env(name, value)
            .output()
            .expect("CLI process must start");
        assert_eq!(invalid_environment.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&invalid_environment.stderr).contains("P1001:"));
    }
    fs::write(
        directory.path().join("untouched-manifest.json"),
        b"existing manifest",
    )
    .unwrap();
    assert_eq!(
        run(
            directory.path(),
            &strings(&[
                "build",
                "large.tsf",
                "-o",
                "config-error.pdf",
                "--config",
                "invalid.toml",
                "--emit-build-manifest",
                "untouched-manifest.json",
            ]),
        )
        .status
        .code(),
        Some(2)
    );
    assert!(!directory.path().join("config-error.pdf").exists());
    assert_eq!(
        fs::read(directory.path().join("untouched-manifest.json")).unwrap(),
        b"existing manifest"
    );
    assert_eq!(
        run(
            directory.path(),
            &strings(&["check", "large.tsf", "--config", "oversized.toml"]),
        )
        .status
        .code(),
        Some(5)
    );
    assert_eq!(
        run(
            directory.path(),
            &strings(&[
                "build",
                "large.tsf",
                "-o",
                "same-target",
                "--trace",
                "same-target",
            ]),
        )
        .status
        .code(),
        Some(2)
    );
    assert!(!directory.path().join("same-target").exists());

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            "missing-config-target",
            directory.path().join("typaxis.toml"),
        )
        .unwrap();
        assert_eq!(
            run(directory.path(), &strings(&["check", "large.tsf"]))
                .status
                .code(),
            Some(3)
        );
    }
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn public_machine_commands_execute_fixtures_and_capabilities_ignore_ambient_inputs() {
    let repository = repository_root();
    let combined = repository.join("samples/machine-package/profiles/paragraph-1/combined");
    let output = TestDirectory::new();
    let pdf = output.path().join("output.pdf");
    let trace = output.path().join("trace.json");
    let manifest = output.path().join("manifest.json");
    let diagnostics = output.path().join("diagnostics.json");
    let build_arguments = vec![
        OsStr::new("build-package"),
        OsStr::new("job/document-package.json"),
        OsStr::new("-o"),
        pdf.as_os_str(),
        OsStr::new("--package-root"),
        OsStr::new("job"),
        OsStr::new("--profile"),
        OsStr::new("typaxis.machine-pdf/paragraph-1"),
        OsStr::new("--resource-root"),
        OsStr::new("job"),
        OsStr::new("--trace"),
        trace.as_os_str(),
        OsStr::new("--trace-text"),
        OsStr::new("--emit-build-manifest"),
        manifest.as_os_str(),
        OsStr::new("--emit-diagnostics"),
        diagnostics.as_os_str(),
        OsStr::new("--no-compress"),
    ];
    let built = run(&combined, &build_arguments);
    assert!(
        built.status.success(),
        "build-package stderr: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(built.stdout.is_empty());
    assert!(built.stderr.is_empty());
    let pdf_bytes = fs::read(&pdf).unwrap();
    assert!(pdf_bytes.starts_with(b"%PDF-1.7\n"));
    assert_eq!(pdf_page_count(&pdf_bytes), 1);
    let trace_text = fs::read_to_string(&trace).unwrap();
    assert!(trace_text.contains("\"resolved_generated_text\":[{"));
    assert!(trace_text.contains("\"utf8\":\"1\""));
    let manifest_text = fs::read_to_string(&manifest).unwrap();
    assert!(manifest_text.contains("\"status\":\"built\""));
    assert!(manifest_text.contains("\"input_profile\":\"typaxis.machine-pdf/paragraph-1\""));
    assert_eq!(
        fs::read_to_string(&diagnostics).unwrap(),
        "{\"contract\":\"typaxis.contract/1.3\",\"diagnostics\":[]}"
    );

    let check_diagnostics = output.path().join("check-diagnostics.json");
    let checked = run(
        &combined,
        &[
            OsStr::new("check-package"),
            OsStr::new("job/document-package.json"),
            OsStr::new("--package-root"),
            OsStr::new("job"),
            OsStr::new("--profile"),
            OsStr::new("typaxis.machine-pdf/paragraph-1"),
            OsStr::new("--resource-root"),
            OsStr::new("job"),
            OsStr::new("--emit-diagnostics"),
            check_diagnostics.as_os_str(),
        ],
    );
    assert!(
        checked.status.success(),
        "check-package stderr: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert_eq!(
        fs::read(&check_diagnostics).unwrap(),
        fs::read(&diagnostics).unwrap()
    );

    let invalid = repository.join("samples/machine-package/invalid/p1100-bom");
    let invalid_output = TestDirectory::new();
    let invalid_pdf = invalid_output.path().join("output.pdf");
    let invalid_manifest = invalid_output.path().join("manifest.json");
    let invalid_diagnostics = invalid_output.path().join("diagnostics.json");
    let rejected = run(
        &invalid,
        &[
            OsStr::new("build-package"),
            OsStr::new("job/document-package.json"),
            OsStr::new("-o"),
            invalid_pdf.as_os_str(),
            OsStr::new("--package-root"),
            OsStr::new("job"),
            OsStr::new("--resource-root"),
            OsStr::new("job"),
            OsStr::new("--emit-build-manifest"),
            invalid_manifest.as_os_str(),
            OsStr::new("--emit-diagnostics"),
            invalid_diagnostics.as_os_str(),
        ],
    );
    assert_eq!(rejected.status.code(), Some(1));
    assert!(!invalid_pdf.exists());
    assert!(fs::read_to_string(invalid_diagnostics)
        .unwrap()
        .contains("\"code\":\"P1100\""));
    let failed_manifest = fs::read_to_string(invalid_manifest).unwrap();
    assert!(failed_manifest.contains("\"status\":\"failed\""));
    assert!(failed_manifest.contains("\"canonical_sha256\":null"));
    assert!(failed_manifest.contains("\"contract\":null"));
    assert!(failed_manifest.contains("\"inputs\":[]"));

    fs::write(output.path().join("typaxis.toml"), b"not valid TOML\0").unwrap();
    let capabilities = cli_command(
        output.path(),
        &strings(&["capabilities", "--format", "json"]),
    )
    .env("TYPAXIS_UNKNOWN", "must-not-be-read")
    .env("TYPAXIS_LIMITS__MAX_PAGES", "not-an-integer")
    .env("LC_ALL", "typaxis-invalid-locale")
    .output()
    .expect("capabilities process must start");
    assert!(
        capabilities.status.success(),
        "capabilities stderr: {}",
        String::from_utf8_lossy(&capabilities.stderr)
    );
    assert!(capabilities.stderr.is_empty());
    assert_eq!(
        capabilities.stdout,
        fs::read(repository.join("samples/machine-package/capabilities.json")).unwrap()
    );
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn basic_document_profile_executes_the_combined_public_fixture() {
    let fixture =
        repository_root().join("samples/machine-package/profiles/basic-document-1/combined");
    let output = TestDirectory::new();
    let pdf = output.path().join("output.pdf");
    let trace = output.path().join("trace.json");
    let manifest = output.path().join("manifest.json");
    let diagnostics = output.path().join("diagnostics.json");

    let built = run(
        &fixture,
        &[
            OsStr::new("build-package"),
            OsStr::new("job/document-package.json"),
            OsStr::new("-o"),
            pdf.as_os_str(),
            OsStr::new("--package-root"),
            OsStr::new("job"),
            OsStr::new("--profile"),
            OsStr::new("typaxis.machine-pdf/basic-document-1"),
            OsStr::new("--resource-root"),
            OsStr::new("job"),
            OsStr::new("--trace"),
            trace.as_os_str(),
            OsStr::new("--trace-text"),
            OsStr::new("--emit-build-manifest"),
            manifest.as_os_str(),
            OsStr::new("--emit-diagnostics"),
            diagnostics.as_os_str(),
            OsStr::new("--no-compress"),
        ],
    );
    assert!(
        built.status.success(),
        "build-package stderr: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(built.stdout.is_empty());
    assert!(built.stderr.is_empty());

    let pdf_bytes = fs::read(&pdf).unwrap();
    assert!(pdf_bytes.starts_with(b"%PDF-1.7\n"));
    assert_eq!(pdf_page_count(&pdf_bytes), 2);
    let trace = fs::read_to_string(trace).unwrap();
    assert!(trace.contains("\"profile_receipt_sha256\":\""));
    assert!(trace.contains("\"flow_registry_sha256\":\""));
    let manifest = fs::read_to_string(manifest).unwrap();
    assert!(manifest.contains("\"status\":\"built\""));
    assert!(manifest.contains("\"input_profile\":\"typaxis.machine-pdf/basic-document-1\""));
    assert!(manifest.contains("\"profile_receipt_sha256\":\""));
    assert!(manifest.contains("\"flow_registry_sha256\":\""));
    assert_eq!(
        fs::read_to_string(diagnostics).unwrap(),
        "{\"contract\":\"typaxis.contract/1.3\",\"diagnostics\":[]}"
    );

    let rejected_by_paragraph = run(
        &fixture,
        &strings(&[
            "check-package",
            "job/document-package.json",
            "--package-root",
            "job",
            "--profile",
            "typaxis.machine-pdf/paragraph-1",
            "--resource-root",
            "job",
        ]),
    );
    assert_eq!(rejected_by_paragraph.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&rejected_by_paragraph.stderr).contains("L5100:"));
}

#[test]
fn exact_profile_limit_maxima_are_accepted() {
    let directory = TestDirectory::new();
    fs::write(directory.path().join("input.tsf"), b"paragraph\n").unwrap();

    for (option, value) in [
        ("--max-ast-nesting-depth", "64"),
        ("--max-fonts", "308915776"),
    ] {
        let result = run(
            directory.path(),
            &strings(&["check", "input.tsf", option, value]),
        );
        assert!(
            result.status.success(),
            "exact maximum {option}={value} was rejected: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
    }
}

#[cfg(unix)]
#[test]
fn symlink_parent_components_cannot_bypass_initial_target_alias_rejection() {
    use std::os::unix::fs::symlink;

    let directory = TestDirectory::new();
    fs::write(directory.path().join("empty.tsf"), b"\n").unwrap();
    fs::create_dir(directory.path().join("a")).unwrap();
    fs::create_dir(directory.path().join("b")).unwrap();
    fs::create_dir(directory.path().join("b/dir")).unwrap();
    symlink(
        directory.path().join("b/dir"),
        directory.path().join("a/link"),
    )
    .unwrap();

    let result = run(
        directory.path(),
        &strings(&[
            "build",
            "empty.tsf",
            "-o",
            "a/link/../shared",
            "--trace",
            "b/shared",
        ]),
    );

    assert_eq!(result.status.code(), Some(2));
    assert!(!directory.path().join("b/shared").exists());
}

#[test]
fn empty_build_publishes_pdf_trace_and_manifest_atomically() {
    let directory = TestDirectory::new();
    let source = directory.path().join("empty.tsf");
    let pdf = directory.path().join("output.pdf");
    let trace = directory.path().join("layout.json");
    let manifest = directory.path().join("manifest.json");
    fs::write(&source, b"\n").unwrap();

    let arguments = strings(&[
        "build",
        "empty.tsf",
        "-o",
        "output.pdf",
        "--trace",
        "layout.json",
        "--trace-text",
        "--emit-build-manifest",
        "manifest.json",
        "--no-compress",
    ]);
    let first = run(directory.path(), &arguments);
    assert!(
        first.status.success(),
        "build stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(first.stdout.is_empty());

    let first_pdf = fs::read(&pdf).unwrap();
    let first_trace = fs::read(&trace).unwrap();
    let first_manifest = fs::read(&manifest).unwrap();
    assert!(first_pdf.starts_with(b"%PDF-1.7\n"));
    assert!(first_pdf.ends_with(b"%%EOF\n"));
    assert!(first_trace.starts_with(b"{\"contract\":\"typaxis.contract/1.3\""));
    assert!(first_manifest
        .windows(16)
        .any(|window| window == b"\"status\":\"built\""));

    let no_replace = run(directory.path(), &arguments);
    assert_eq!(no_replace.status.code(), Some(3));
    assert_eq!(fs::read(&pdf).unwrap(), first_pdf);
    assert_eq!(fs::read(&trace).unwrap(), first_trace);
    assert_eq!(fs::read(&manifest).unwrap(), first_manifest);

    let mut force_arguments = arguments;
    force_arguments.push(OsStr::new("--force"));
    let forced = run(directory.path(), &force_arguments);
    assert!(forced.status.success());
    assert_eq!(fs::read(&pdf).unwrap(), first_pdf);
    assert_eq!(fs::read(&trace).unwrap(), first_trace);
    assert_eq!(fs::read(&manifest).unwrap(), first_manifest);
}

#[test]
fn stdout_pdf_is_complete_before_built_manifest_publication() {
    let directory = TestDirectory::new();
    fs::write(directory.path().join("empty.tsf"), b"\n").unwrap();

    let dash_file = run(
        directory.path(),
        &strings(&["build", "empty.tsf", "-o", "./-", "--no-compress"]),
    );
    assert!(dash_file.status.success());
    assert!(dash_file.stdout.is_empty());
    assert!(fs::read(directory.path().join("-"))
        .unwrap()
        .starts_with(b"%PDF-1.7\n"));

    let arguments = strings(&[
        "build",
        "empty.tsf",
        "-o",
        "-",
        "--emit-build-manifest",
        "manifest.json",
        "--no-compress",
    ]);

    let first = run(directory.path(), &arguments);
    assert!(
        first.status.success(),
        "build stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(first.stderr.is_empty());
    assert!(first.stdout.starts_with(b"%PDF-1.7\n"));
    assert!(first.stdout.ends_with(b"%%EOF\n"));
    let manifest = fs::read_to_string(directory.path().join("manifest.json")).unwrap();
    assert!(manifest.contains("\"status\":\"built\""));
    let output = manifest_output_facts(&manifest);
    assert_eq!(output.sink, "stdout");
    assert_eq!(
        output.bytes,
        u64::try_from(first.stdout.len()).expect("PDF length must fit in the manifest integer")
    );
    assert_eq!(output.sha256, sha256_hex(&first.stdout));
    assert_eq!(output.page_count, pdf_page_count(&first.stdout));
    assert_eq!(output.pdf_object_count, pdf_object_count(&first.stdout));

    fs::remove_file(directory.path().join("manifest.json")).unwrap();
    fs::write(directory.path().join("manifest.json"), b"preserve existing").unwrap();
    let manifest_failure = run(directory.path(), &arguments);
    assert_eq!(manifest_failure.status.code(), Some(3));
    assert_eq!(manifest_failure.stdout, first.stdout);
    assert!(String::from_utf8_lossy(&manifest_failure.stderr)
        .contains("PDF was published but manifest publication failed"));
    assert_eq!(
        fs::read(directory.path().join("manifest.json")).unwrap(),
        b"preserve existing"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn stdout_failure_publishes_only_an_output_null_failed_manifest() {
    use std::process::Stdio;

    let directory = TestDirectory::new();
    fs::write(directory.path().join("empty.tsf"), b"\n").unwrap();
    let arguments = strings(&[
        "build",
        "empty.tsf",
        "-o",
        "-",
        "--emit-build-manifest",
        "manifest.json",
        "--no-compress",
    ]);
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = cli_command(directory.path(), &arguments)
        .stdout(Stdio::from(full))
        .output()
        .expect("CLI process must start");

    assert_eq!(output.status.code(), Some(3));
    let manifest = fs::read_to_string(directory.path().join("manifest.json")).unwrap();
    assert!(manifest.contains("\"status\":\"failed\""));
    assert!(manifest.contains("\"output\":null"));
    assert!(!manifest.contains("\"status\":\"built\""));
}

#[cfg(target_os = "linux")]
#[test]
fn stderr_failure_uses_the_documented_io_exit_class_without_panicking() {
    use std::process::Stdio;

    let directory = TestDirectory::new();
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = cli_command(directory.path(), &strings(&["unknown-command"]))
        .stderr(Stdio::from(full))
        .output()
        .expect("CLI process must start");

    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn strict_fallback_publishes_trace_then_failed_manifest_without_pdf() {
    let directory = TestDirectory::new();
    fs::write(directory.path().join("empty.tsf"), b"\n").unwrap();
    let result = run(
        directory.path(),
        &strings(&[
            "build",
            "empty.tsf",
            "-o",
            "output.pdf",
            "--trace",
            "trace.json",
            "--emit-build-manifest",
            "manifest.json",
            "--strict",
            "--max-layout-passes",
            "1",
            "--no-compress",
        ]),
    );

    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert!(!directory.path().join("output.pdf").exists());
    let trace = fs::read_to_string(directory.path().join("trace.json")).unwrap();
    assert!(trace.contains("\"status\":\"max_pass_fallback\""));
    let manifest = fs::read_to_string(directory.path().join("manifest.json")).unwrap();
    assert!(manifest.contains("\"status\":\"failed\""));
    assert!(manifest.contains("\"output\":null"));
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[test]
fn non_utf8_host_paths_build_without_entering_the_manifest() {
    use std::os::unix::ffi::OsStringExt;

    let directory = TestDirectory::new();
    let input = std::ffi::OsString::from_vec(b"input-\xff.tsf".to_vec());
    let output = std::ffi::OsString::from_vec(b"output-\xfe.pdf".to_vec());
    let config = std::ffi::OsString::from_vec(b"config-\xfd.toml".to_vec());
    let resource_root = std::ffi::OsString::from_vec(b"resources-\xfc".to_vec());
    let trace = std::ffi::OsString::from_vec(b"trace-\xfb.json".to_vec());
    let manifest = std::ffi::OsString::from_vec(b"manifest-\xfa.json".to_vec());
    fs::write(directory.path().join(&input), b"\n").unwrap();
    fs::write(
        directory.path().join(&config),
        b"contract = \"typaxis.contract/1.1\"\n",
    )
    .unwrap();
    fs::create_dir(directory.path().join(&resource_root)).unwrap();
    let arguments = vec![
        OsStr::new("build"),
        input.as_os_str(),
        OsStr::new("-o"),
        output.as_os_str(),
        OsStr::new("--config"),
        config.as_os_str(),
        OsStr::new("--resource-root"),
        resource_root.as_os_str(),
        OsStr::new("--trace"),
        trace.as_os_str(),
        OsStr::new("--emit-build-manifest"),
        manifest.as_os_str(),
        OsStr::new("--no-compress"),
    ];

    let result = run(directory.path(), &arguments);
    assert!(
        result.status.success(),
        "build stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
    assert!(fs::read(directory.path().join(&output))
        .unwrap()
        .starts_with(b"%PDF-1.7\n"));
    assert!(fs::read(directory.path().join(&trace)).is_ok());
    let manifest_bytes = fs::read(directory.path().join(&manifest)).unwrap();
    let manifest_json = std::str::from_utf8(&manifest_bytes).unwrap();
    assert!(manifest_json.contains("\"sink\":\"file\""));
    assert!(!manifest_json.contains('\u{fffd}'));
}

#[test]
fn dump_commands_emit_canonical_reference_artifacts() {
    let directory = TestDirectory::new();
    fs::write(
        directory.path().join("reference.tsf"),
        b"paragraph\nanchor:target\n",
    )
    .unwrap();

    let ast = run(
        directory.path(),
        &strings(&["dump-ast", "reference.tsf", "--format", "json"]),
    );
    assert!(ast.status.success());
    let ast = String::from_utf8(ast.stdout).unwrap();
    assert!(ast.starts_with("{\"contract\":\"typaxis.contract/1.3\""));
    assert!(ast.contains("\"anchor_id\":\"target\""));
    assert!(!ast.ends_with('\n'));

    let limited_ast = run(
        directory.path(),
        &strings(&[
            "dump-ast",
            "reference.tsf",
            "--format",
            "json",
            "--max-document-package-bytes",
            "1",
        ]),
    );
    assert_eq!(limited_ast.status.code(), Some(5));
    assert!(limited_ast.stdout.is_empty());

    let layout = run(
        directory.path(),
        &strings(&["dump-layout", "reference.tsf", "--page", "1"]),
    );
    assert!(layout.status.success());
    let layout = String::from_utf8(layout.stdout).unwrap();
    assert!(layout.starts_with("{\"contract\":\"typaxis.contract/1.3\""));
    assert!(layout.contains("\"fragments\":[{"));
    assert!(!layout.ends_with('\n'));

    assert_eq!(
        run(
            directory.path(),
            &strings(&["dump-layout", "reference.tsf", "--page", "2"]),
        )
        .status
        .code(),
        Some(1)
    );
}

#[test]
fn reference_paragraphs_and_anchors_build_with_selected_destinations() {
    let directory = TestDirectory::new();
    fs::write(
        directory.path().join("reference.tsf"),
        b"anchor:z\nparagraph\nanchor:a\n",
    )
    .unwrap();

    let build = run(
        directory.path(),
        &strings(&[
            "build",
            "reference.tsf",
            "-o",
            "reference.pdf",
            "--trace",
            "reference-layout.json",
            "--no-compress",
        ]),
    );
    assert!(
        build.status.success(),
        "build stderr: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let pdf = fs::read(directory.path().join("reference.pdf")).unwrap();
    let trace = fs::read_to_string(directory.path().join("reference-layout.json")).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.7\n"));
    assert!(pdf.windows(2).any(|window| window == b"/D"));
    assert!(trace.contains("\"anchor_id\":\"a\""));
    assert!(trace.contains("\"anchor_id\":\"z\""));
    assert!(trace.contains("\"fragments\":[{"));
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[test]
fn shaped_text_builds_with_a_deterministic_embedded_subset() {
    let directory = TestDirectory::new();
    fs::write(
        directory.path().join("Synthetic.ttf"),
        synthetic_ascii_ttf(),
    )
    .unwrap();
    fs::write(
        directory.path().join("text.tsf"),
        b"font:Synthetic:Synthetic.ttf\ntext:Hello world\n",
    )
    .unwrap();

    let build = run(
        directory.path(),
        &strings(&["build", "text.tsf", "-o", "text.pdf", "--no-compress"]),
    );
    assert!(
        build.status.success(),
        "build stderr: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let pdf = fs::read(directory.path().join("text.pdf")).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.7\n"));
    assert!(pdf
        .windows(b"/Subtype /Type0".len())
        .any(|bytes| bytes == b"/Subtype /Type0"));
    assert!(pdf
        .windows(b"/FontFile2".len())
        .any(|bytes| bytes == b"/FontFile2"));
    assert!(pdf
        .windows(b"/AAAAAA+Typaxis".len())
        .any(|bytes| bytes == b"/AAAAAA+Typaxis"));
    assert!(pdf.windows(b"<0048>".len()).any(|bytes| bytes == b"<0048>"));
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[test]
fn generated_page_reference_reflows_and_paints_the_selected_state() {
    let directory = TestDirectory::new();
    fs::write(
        directory.path().join("Synthetic.ttf"),
        synthetic_ascii_ttf(),
    )
    .unwrap();
    fs::write(
        directory.path().join("reference.tsf"),
        b"font:Synthetic:Synthetic.ttf\nanchor:target\nreference:target\n",
    )
    .unwrap();
    let build = run(
        directory.path(),
        &strings(&[
            "build",
            "reference.tsf",
            "-o",
            "reference.pdf",
            "--no-compress",
        ]),
    );
    assert!(
        build.status.success(),
        "build stderr: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let pdf = fs::read(directory.path().join("reference.pdf")).unwrap();
    assert_artifact_glyph_is_painted(&pdf);
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[test]
fn adjacent_parsed_and_generated_sites_use_whole_paragraph_itemization() {
    let directory = TestDirectory::new();
    fs::write(
        directory.path().join("Synthetic.ttf"),
        synthetic_ascii_ttf(),
    )
    .unwrap();
    fs::write(
        directory.path().join("inline-reference.tsf"),
        b"font:Synthetic:Synthetic.ttf\nanchor:target\ninlines:text=Page |reference=target|text= now\n",
    )
    .unwrap();
    let build = run(
        directory.path(),
        &strings(&[
            "build",
            "inline-reference.tsf",
            "-o",
            "inline-reference.pdf",
            "--no-compress",
        ]),
    );
    assert!(
        build.status.success(),
        "build stderr: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let pdf = fs::read(directory.path().join("inline-reference.pdf")).unwrap();
    assert_artifact_glyph_is_painted(&pdf);
    assert!(pdf.windows(b"<0050>".len()).any(|bytes| bytes == b"<0050>"));
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[test]
fn paragraph_lines_continue_on_a_new_physical_page() {
    let directory = TestDirectory::new();
    fs::write(
        directory.path().join("Synthetic.ttf"),
        synthetic_ascii_ttf(),
    )
    .unwrap();
    let mut source = String::from("font:Synthetic:Synthetic.ttf\n");
    // The reference default is A4 with a 20 mm body margin and a 17 pt line
    // height, so 43 single-line paragraphs must continue onto page two.
    for _ in 0..43 {
        source.push_str("text:A\n");
    }
    fs::write(directory.path().join("pages.tsf"), source).unwrap();
    let build = run(
        directory.path(),
        &strings(&[
            "build",
            "pages.tsf",
            "-o",
            "pages.pdf",
            "--emit-build-manifest",
            "pages.json",
            "--no-compress",
        ]),
    );
    assert!(
        build.status.success(),
        "build stderr: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let manifest = fs::read_to_string(directory.path().join("pages.json")).unwrap();
    assert_eq!(manifest_output_facts(&manifest).page_count, 2);
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn machine_block_styles_staging_slice_uses_the_normal_public_pipeline() {
    let fixture = repository_root()
        .join("samples/machine-package/staging/basic-document-1/machine-block-styles");
    let output = TestDirectory::new();
    let diagnostics = output.path().join("diagnostics.json");
    let rejected_contract = run(
        &fixture,
        &[
            OsStr::new("check-package"),
            OsStr::new("job/document-package.json"),
            OsStr::new("--package-root"),
            OsStr::new("job"),
            OsStr::new("--profile"),
            OsStr::new("typaxis.machine-pdf/paragraph-1"),
            OsStr::new("--emit-diagnostics"),
            diagnostics.as_os_str(),
        ],
    );
    assert_eq!(rejected_contract.status.code(), Some(1));
    assert!(!output.path().join("output.pdf").exists());
    let diagnostics = fs::read_to_string(diagnostics).unwrap();
    assert!(diagnostics.contains("\"code\":\"L5100\""));

    let rejected_profile = run(
        &fixture,
        &strings(&[
            "check-package",
            "job/document-package.json",
            "--package-root",
            "job",
            "--profile",
            "typaxis.machine-pdf/basic-document-1",
        ]),
    );
    assert_ne!(rejected_profile.status.code(), Some(2));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn machine_list_staging_slice_uses_the_normal_public_pipeline() {
    let fixture =
        repository_root().join("samples/machine-package/staging/basic-document-1/machine-list");
    let output = TestDirectory::new();
    let diagnostics = output.path().join("diagnostics.json");
    let rejected_contract = run(
        &fixture,
        &[
            OsStr::new("check-package"),
            OsStr::new("job/document-package.json"),
            OsStr::new("--package-root"),
            OsStr::new("job"),
            OsStr::new("--profile"),
            OsStr::new("typaxis.machine-pdf/paragraph-1"),
            OsStr::new("--emit-diagnostics"),
            diagnostics.as_os_str(),
        ],
    );
    assert_eq!(rejected_contract.status.code(), Some(1));
    assert!(!output.path().join("output.pdf").exists());
    let diagnostics = fs::read_to_string(diagnostics).unwrap();
    assert!(diagnostics.contains("\"code\":\"L5100\""));

    let rejected_profile = run(
        &fixture,
        &strings(&[
            "check-package",
            "job/document-package.json",
            "--package-root",
            "job",
            "--profile",
            "typaxis.machine-pdf/basic-document-1",
        ]),
    );
    assert_ne!(rejected_profile.status.code(), Some(2));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn machine_page_break_staging_slice_uses_the_normal_public_pipeline() {
    let fixture = repository_root()
        .join("samples/machine-package/staging/basic-document-1/machine-page-break");
    let output = TestDirectory::new();
    let diagnostics = output.path().join("diagnostics.json");
    let rejected_contract = run(
        &fixture,
        &[
            OsStr::new("check-package"),
            OsStr::new("job/document-package.json"),
            OsStr::new("--package-root"),
            OsStr::new("job"),
            OsStr::new("--profile"),
            OsStr::new("typaxis.machine-pdf/paragraph-1"),
            OsStr::new("--emit-diagnostics"),
            diagnostics.as_os_str(),
        ],
    );
    assert_eq!(rejected_contract.status.code(), Some(1));
    assert!(!output.path().join("output.pdf").exists());
    let diagnostics = fs::read_to_string(diagnostics).unwrap();
    assert!(diagnostics.contains("\"code\":\"L5100\""));

    let rejected_profile = run(
        &fixture,
        &strings(&[
            "check-package",
            "job/document-package.json",
            "--package-root",
            "job",
            "--profile",
            "typaxis.machine-pdf/basic-document-1",
        ]),
    );
    assert_ne!(rejected_profile.status.code(), Some(2));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn machine_figure_staging_slice_uses_the_normal_public_pipeline() {
    let fixture =
        repository_root().join("samples/machine-package/staging/basic-document-1/machine-figure");
    let output = TestDirectory::new();
    let diagnostics = output.path().join("diagnostics.json");
    let rejected_contract = run(
        &fixture,
        &[
            OsStr::new("check-package"),
            OsStr::new("job/document-package.json"),
            OsStr::new("--package-root"),
            OsStr::new("job"),
            OsStr::new("--profile"),
            OsStr::new("typaxis.machine-pdf/paragraph-1"),
            OsStr::new("--emit-diagnostics"),
            diagnostics.as_os_str(),
        ],
    );
    assert_eq!(rejected_contract.status.code(), Some(1));
    assert!(!output.path().join("output.pdf").exists());
    let diagnostics = fs::read_to_string(diagnostics).unwrap();
    assert!(diagnostics.contains("\"code\":\"L5100\""));

    let rejected_profile = run(
        &fixture,
        &strings(&[
            "check-package",
            "job/document-package.json",
            "--package-root",
            "job",
            "--profile",
            "typaxis.machine-pdf/basic-document-1",
        ]),
    );
    assert_ne!(rejected_profile.status.code(), Some(2));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn machine_link_staging_slice_uses_the_normal_public_pipeline() {
    let fixture =
        repository_root().join("samples/machine-package/staging/basic-document-1/machine-link");
    let output = TestDirectory::new();
    let diagnostics = output.path().join("diagnostics.json");
    let rejected_contract = run(
        &fixture,
        &[
            OsStr::new("check-package"),
            OsStr::new("job/document-package.json"),
            OsStr::new("--package-root"),
            OsStr::new("job"),
            OsStr::new("--profile"),
            OsStr::new("typaxis.machine-pdf/paragraph-1"),
            OsStr::new("--emit-diagnostics"),
            diagnostics.as_os_str(),
        ],
    );
    assert_eq!(rejected_contract.status.code(), Some(1));
    assert!(!output.path().join("output.pdf").exists());
    let diagnostics = fs::read_to_string(diagnostics).unwrap();
    assert!(diagnostics.contains("\"code\":\"L5100\""));

    let rejected_profile = run(
        &fixture,
        &strings(&[
            "check-package",
            "job/document-package.json",
            "--package-root",
            "job",
            "--profile",
            "typaxis.machine-pdf/basic-document-1",
        ]),
    );
    assert_ne!(rejected_profile.status.code(), Some(2));
}
