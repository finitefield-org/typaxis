#![cfg_attr(
    not(any(target_os = "android", target_os = "linux", target_os = "macos")),
    allow(dead_code, unused_imports)
)]

use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use typaxis_core::{MachineInputLimitBounds, ResourceLimits};

const MINIMAL_PACKAGE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/minimal/document-package.json"
));
const MINIMAL_SOURCE: &[u8] = b"\n";
const RICH_PACKAGE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/conformance/document-rich.json"
));
const RICH_SOURCE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/conformance/input.tsf"
));

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(label: &str) -> Self {
        let ordinal = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "typaxis-machine-input-{label}-{}-{ordinal}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create test root");
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn default_limits() -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits::default()).expect("default limits")
}

fn limits_with(update: impl FnOnce(&mut ResourceLimits)) -> ValidatedResourceLimits {
    let mut limits = ResourceLimits::default();
    update(&mut limits);
    ValidatedResourceLimits::new(limits).expect("valid test limits")
}

fn hash_hex(hash: [u8; 32]) -> String {
    let mut output = String::new();
    for byte in hash {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn source_entry(source_id: u32, uri: &str, declared: u32, hash: [u8; 32]) -> String {
    format!(
        "    {{\n      \"source_id\": {source_id},\n      \"uri\": \"{uri}\",\n      \"utf8_byte_length\": {declared},\n      \"sha256\": \"{}\"\n    }}",
        hash_hex(hash)
    )
}

fn package_with_entries(entries: &str) -> Vec<u8> {
    let mut package = MINIMAL_PACKAGE.to_owned();
    let marker = "  \"sources\": [\n";
    let start = package.find(marker).expect("sources start") + marker.len();
    let suffix = "\n  ],\n  \"text_buffers\"";
    let end = start + package[start..].find(suffix).expect("sources end");
    package.replace_range(start..end, entries);
    package.into_bytes()
}

fn package_for_source(source_id: u32, uri: &str, source: &[u8]) -> Vec<u8> {
    let length = u32::try_from(source.len()).expect("small test source");
    package_with_entries(&source_entry(source_id, uri, length, sha256(source)))
}

fn write_package(root: &Path, package: &[u8]) -> PathBuf {
    let path = root.join("document-package.json");
    fs::write(&path, package).expect("write PACKAGE");
    path
}

fn write_source(root: &Path, uri: &str, bytes: &[u8]) {
    let path = root.join(uri);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create source parent");
    }
    fs::write(path, bytes).expect("write source");
}

fn options(package: &Path, root: Option<&Path>) -> MachineInputHostOptions {
    MachineInputHostOptions::new(
        HostPath::new(package.to_path_buf()).expect("PACKAGE HostPath"),
        root.map(|path| HostPath::new(path.to_path_buf()).expect("root HostPath")),
    )
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
fn open_and_decode(
    root: &Path,
    package: &[u8],
    limits: &ValidatedResourceLimits,
) -> (
    HostMachineInputSession,
    AdmittedPackageBytes,
    SessionBoundDecodedPackage,
) {
    let package_path = write_package(root, package);
    let (session, raw) =
        HostMachineInputSession::open(options(&package_path, None), limits).expect("admit PACKAGE");
    let policy = DocumentPackageDecodePolicy::new(limits);
    let decoded = session
        .decode_and_bind(&raw, &StrictDocumentPackageDecoder::new(), &policy)
        .expect("decode PACKAGE");
    (session, raw, decoded)
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
fn source_failure(
    root: &Path,
    package: &[u8],
    limits: &ValidatedResourceLimits,
) -> MachineInputError {
    let (session, _raw, decoded) = open_and_decode(root, package, limits);
    let error = session
        .admit_sources(&decoded, limits)
        .expect_err("source admission must fail");
    assert_eq!(error.progress().stage(), MachineInputStage::PackageDecoded);
    assert!(error.progress().sources().is_empty());
    error
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn complete_flow_is_monotonic_and_retains_owned_source_bytes() {
    let root = TestRoot::new("complete");
    let package_bytes = package_for_source(0, "sources/book.json", MINIMAL_SOURCE);
    let package_path = write_package(root.path(), &package_bytes);
    write_source(root.path(), "sources/book.json", MINIMAL_SOURCE);
    let limits = default_limits();

    let (session, raw) =
        HostMachineInputSession::open(options(&package_path, None), &limits).expect("open");
    assert_eq!(
        session.progress().stage(),
        MachineInputStage::RawPackageAdmitted
    );
    assert_eq!(raw.facts().uri().as_str(), "document-package.json");
    assert_eq!(
        session
            .read_ledger_token()
            .unwrap()
            .candidate_attempt_count(),
        1
    );
    fs::remove_file(&package_path).expect("remove admitted PACKAGE path");

    let policy = DocumentPackageDecodePolicy::new(&limits);
    let decoded = session
        .decode_and_bind(&raw, &StrictDocumentPackageDecoder::new(), &policy)
        .expect("decode");
    assert_eq!(
        session.progress().stage(),
        MachineInputStage::PackageDecoded
    );
    let sources = session.admit_sources(&decoded, &limits).expect("sources");
    assert_eq!(
        session.progress().stage(),
        MachineInputStage::SourcesAdmitted
    );
    assert_eq!(sources.sources()[0].text(), "\n");
    let ledger = session.read_ledger_token().unwrap();
    assert_eq!(ledger.candidate_attempt_count(), 2);
    assert_eq!(ledger.stored_candidate_identity_count(), 2);
    assert_eq!(ledger.stored_opened_identity_count(), 2);

    fs::remove_file(root.path().join("sources/book.json")).expect("remove admitted source path");
    let admitted = session.finish(raw, decoded, sources).expect("finish");
    assert_eq!(
        admitted.progress().stage(),
        MachineInputStage::SourcesAdmitted
    );
    assert_eq!(admitted.sources()[0].text(), "\n");
    assert_eq!(
        admitted.progress().fingerprint(),
        Some(admitted.fingerprint())
    );

    let package = admitted.progress().package().unwrap();
    let decoded = admitted.progress().decoded().unwrap();
    let source = &admitted.progress().sources()[0];
    let jcs = portable_fingerprint_jcs(package, decoded, source);
    assert_eq!(
        admitted.fingerprint(),
        machine_input_fingerprint_from_jcs(&jcs)
    );
    assert!(!jcs.contains(root.path().to_string_lossy().as_ref()));
    assert!(!jcs.contains("session"));
    assert!(!jcs.contains("profile"));
    assert!(!jcs.contains("config"));
    assert!(jcs.starts_with("{\"algorithm\":\"typaxis.machine-input-sha256/1\",\"package\":{"));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn declared_resources_do_not_make_package_root_a_resource_root() {
    let root = TestRoot::new("no-implicit-resource-root");
    write_source(root.path(), "input.tsf", RICH_SOURCE);
    let limits = default_limits();
    let (session, raw, decoded) = open_and_decode(root.path(), RICH_PACKAGE, &limits);
    assert_eq!(decoded.decoded().wire().resources.font_faces.len(), 1);
    assert!(!root.path().join("test-font.bin").exists());
    let sources = session.admit_sources(&decoded, &limits).unwrap();
    let ledger = session.read_ledger_token().unwrap();
    assert_eq!(ledger.candidate_attempt_count(), 2);
    assert_eq!(ledger.stored_opened_identity_count(), 2);
    session.finish(raw, decoded, sources).unwrap();
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn explicit_root_derives_portable_uri_and_rejects_lexical_escape() {
    let root = TestRoot::new("explicit-root");
    let job = root.path().join("job");
    fs::create_dir_all(&job).unwrap();
    let package_bytes = package_for_source(0, "empty.tsf", MINIMAL_SOURCE);
    let package_path = write_package(&job, &package_bytes);
    write_source(root.path(), "empty.tsf", MINIMAL_SOURCE);
    let limits = default_limits();

    let (session, raw) =
        HostMachineInputSession::open(options(&package_path, Some(root.path())), &limits)
            .expect("explicit contained PACKAGE");
    assert_eq!(raw.facts().uri().as_str(), "job/document-package.json");
    drop((session, raw));

    let outside_root = root.path().join("inside");
    fs::create_dir_all(&outside_root).unwrap();
    let error = HostMachineInputSession::open(options(&package_path, Some(&outside_root)), &limits)
        .expect_err("lexically outside PACKAGE must fail");
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::PackageOutsideRoot
    ));
    assert_eq!(error.progress().stage(), MachineInputStage::NoInput);

    let root_name = root.path().file_name().unwrap();
    let exit_and_reenter = root
        .path()
        .join("..")
        .join(root_name)
        .join("job/document-package.json");
    let error =
        HostMachineInputSession::open(options(&exit_and_reenter, Some(root.path())), &limits)
            .expect_err("a parent component may not leave and re-enter package-root");
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::PackageOutsideRoot
    ));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn default_root_is_the_package_tokens_lexical_parent() {
    use std::os::unix::fs::symlink;

    let root = TestRoot::new("default-lexical-parent");
    let target = root.path().join("target/nested");
    fs::create_dir_all(&target).unwrap();
    symlink(&target, root.path().join("link")).unwrap();
    let package = package_for_source(0, "empty.tsf", MINIMAL_SOURCE);
    let resolved_parent = root.path().join("target");
    write_package(&resolved_parent, &package);
    write_source(&resolved_parent, "empty.tsf", MINIMAL_SOURCE);

    let lexical_package = root.path().join("link/../document-package.json");
    let limits = default_limits();
    let (session, raw) =
        HostMachineInputSession::open(options(&lexical_package, None), &limits).unwrap();
    assert_eq!(raw.facts().uri().as_str(), "document-package.json");
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let decoded = session
        .decode_and_bind(&raw, &StrictDocumentPackageDecoder::new(), &policy)
        .unwrap();
    let sources = session.admit_sources(&decoded, &limits).unwrap();
    session.finish(raw, decoded, sources).unwrap();
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn package_byte_limit_is_reserved_before_exact_read_and_policy_is_bound() {
    let exact_root = TestRoot::new("package-exact");
    let package = package_for_source(0, "empty.tsf", MINIMAL_SOURCE);
    let exact_path = write_package(exact_root.path(), &package);
    let maximum = u64::try_from(package.len()).unwrap();
    let package_limits = DocumentPackagePreflightLimits::new(
        maximum,
        MachineInputLimitBounds::DEFAULT_MAX_JSON_NESTING_DEPTH,
    )
    .unwrap();
    let limits = default_limits();
    let (session, raw) = HostMachineInputSession::open_with_preflight_limits(
        options(&exact_path, None),
        &limits,
        package_limits,
    )
    .expect("exact package maximum");
    let mismatched_policy = DocumentPackageDecodePolicy::new(&limits);
    let error = session
        .decode_and_bind(
            &raw,
            &StrictDocumentPackageDecoder::new(),
            &mismatched_policy,
        )
        .expect_err("different package bound must fail");
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::DecodePolicyMismatch
    ));
    assert_eq!(
        error.progress().stage(),
        MachineInputStage::RawPackageAdmitted
    );
    let matching_policy =
        DocumentPackageDecodePolicy::with_preflight_limits(&limits, package_limits);
    session
        .decode_and_bind(&raw, &StrictDocumentPackageDecoder::new(), &matching_policy)
        .expect("matching package bound decodes");

    let over_root = TestRoot::new("package-over");
    let mut over = package;
    over.push(b' ');
    let over_path = write_package(over_root.path(), &over);
    let error = HostMachineInputSession::open_with_preflight_limits(
        options(&over_path, None),
        &limits,
        package_limits,
    )
    .expect_err("max + 1 package must fail");
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::PackageTooLarge {
            maximum: found_maximum,
            observed
        } if *found_maximum == maximum && *observed == maximum + 1
    ));
    assert_eq!(error.progress().stage(), MachineInputStage::NoInput);
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn decode_failure_returns_raw_package_progress_only() {
    let root = TestRoot::new("decode-failure");
    let package_path = write_package(root.path(), b"{}");
    let limits = default_limits();
    let (session, raw) =
        HostMachineInputSession::open(options(&package_path, None), &limits).unwrap();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let error = session
        .decode_and_bind(&raw, &StrictDocumentPackageDecoder::new(), &policy)
        .expect_err("missing package fields");
    assert!(matches!(error.kind(), MachineInputErrorKind::Decode(_)));
    assert_eq!(
        error.progress().stage(),
        MachineInputStage::RawPackageAdmitted
    );
    assert!(error.progress().package().is_some());
    assert!(error.progress().decoded().is_none());
    assert!(error.progress().sources().is_empty());
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn source_profile_rejections_are_dedicated_and_preserve_decoded_progress() {
    let limits = default_limits();

    let two_root = TestRoot::new("two-sources");
    let first = source_entry(0, "empty.tsf", 1, sha256(MINIMAL_SOURCE));
    let second = source_entry(1, "second.tsf", 1, sha256(MINIMAL_SOURCE));
    let error = source_failure(
        two_root.path(),
        &package_with_entries(&format!("{first},\n{second}")),
        &limits,
    );
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::SourceCount { observed: 2 }
    ));

    let id_root = TestRoot::new("source-id");
    let error = source_failure(
        id_root.path(),
        &package_for_source(7, "empty.tsf", MINIMAL_SOURCE),
        &limits,
    );
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::NonzeroSourceId { observed: 7 }
    ));

    let unsafe_root = TestRoot::new("unsafe-source");
    let error = source_failure(
        unsafe_root.path(),
        &package_for_source(0, "../outside.tsf", MINIMAL_SOURCE),
        &limits,
    );
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::UnsafeSourceUri { .. }
    ));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn source_length_hash_utf8_and_limit_failures_are_distinct() {
    let limits = default_limits();

    let length_root = TestRoot::new("source-length");
    write_source(length_root.path(), "empty.tsf", MINIMAL_SOURCE);
    let error = source_failure(
        length_root.path(),
        &package_with_entries(&source_entry(0, "empty.tsf", 2, sha256(MINIMAL_SOURCE))),
        &limits,
    );
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::SourceLengthMismatch {
            declared: 2,
            actual: 1,
            ..
        }
    ));

    let hash_root = TestRoot::new("source-hash");
    write_source(hash_root.path(), "empty.tsf", MINIMAL_SOURCE);
    let error = source_failure(
        hash_root.path(),
        &package_with_entries(&source_entry(0, "empty.tsf", 1, [0; 32])),
        &limits,
    );
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::SourceHashMismatch { .. }
    ));

    let utf8_root = TestRoot::new("source-utf8");
    let invalid_utf8 = [0xff];
    write_source(utf8_root.path(), "invalid.bin", &invalid_utf8);
    let error = source_failure(
        utf8_root.path(),
        &package_for_source(0, "invalid.bin", &invalid_utf8),
        &limits,
    );
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::SourceNotUtf8 { valid_up_to: 0, .. }
    ));

    let limit_root = TestRoot::new("source-limit");
    let two_bytes = b"ab";
    write_source(limit_root.path(), "actual.tsf", two_bytes);
    let strict = limits_with(|limits| {
        limits.max_source_bytes = 1;
        limits.max_input_bytes = 1;
    });
    let exact_root = TestRoot::new("source-limit-exact");
    write_source(exact_root.path(), "actual.tsf", b"a");
    let exact_package = package_for_source(0, "actual.tsf", b"a");
    let (exact, exact_raw, exact_decoded) =
        open_and_decode(exact_root.path(), &exact_package, &strict);
    let exact_sources = exact.admit_sources(&exact_decoded, &strict).unwrap();
    assert_eq!(exact_sources.sources()[0].text(), "a");
    exact
        .finish(exact_raw, exact_decoded, exact_sources)
        .unwrap();

    let error = source_failure(
        limit_root.path(),
        &package_with_entries(&source_entry(0, "actual.tsf", 1, sha256(b"a"))),
        &strict,
    );
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::SourceLimit {
            maximum: 1,
            observed: 2,
            ..
        }
    ));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn source_symlink_is_rejected_by_the_shared_contained_opener() {
    use std::os::unix::fs::symlink;

    let root = TestRoot::new("source-symlink");
    let outside = TestRoot::new("source-symlink-target");
    write_source(outside.path(), "target.tsf", MINIMAL_SOURCE);
    symlink(
        outside.path().join("target.tsf"),
        root.path().join("link.tsf"),
    )
    .expect("create source symlink");
    let package = package_for_source(0, "link.tsf", MINIMAL_SOURCE);
    let error = source_failure(root.path(), &package, &default_limits());
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::SourceOpen {
            cause: HostAdmissionError::UnsafeCandidate,
            ..
        }
    ));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn identical_receipts_cannot_cross_session_boundaries() {
    let first_root = TestRoot::new("swap-first");
    let second_root = TestRoot::new("swap-second");
    let package = package_for_source(0, "empty.tsf", MINIMAL_SOURCE);
    write_source(first_root.path(), "empty.tsf", MINIMAL_SOURCE);
    write_source(second_root.path(), "empty.tsf", MINIMAL_SOURCE);
    let limits = default_limits();
    let first_path = write_package(first_root.path(), &package);
    let second_path = write_package(second_root.path(), &package);
    let (first, first_raw) =
        HostMachineInputSession::open(options(&first_path, None), &limits).unwrap();
    let (second, second_raw) =
        HostMachineInputSession::open(options(&second_path, None), &limits).unwrap();
    let policy = DocumentPackageDecodePolicy::new(&limits);

    let error = second
        .decode_and_bind(&first_raw, &StrictDocumentPackageDecoder::new(), &policy)
        .expect_err("raw receipt swap");
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::ReceiptSessionMismatch(MachineInputReceiptKind::RawPackage)
    ));
    let first_decoded = first
        .decode_and_bind(&first_raw, &StrictDocumentPackageDecoder::new(), &policy)
        .unwrap();
    let second_decoded = second
        .decode_and_bind(&second_raw, &StrictDocumentPackageDecoder::new(), &policy)
        .unwrap();
    let error = second
        .admit_sources(&first_decoded, &limits)
        .expect_err("decoded receipt swap");
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::ReceiptSessionMismatch(MachineInputReceiptKind::DecodedPackage)
    ));
    let second_sources = second.admit_sources(&second_decoded, &limits).unwrap();
    let error = second
        .finish(second_raw, first_decoded, second_sources)
        .expect_err("finish receipt swap");
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::ReceiptSessionMismatch(MachineInputReceiptKind::DecodedPackage)
    ));

    let third_root = TestRoot::new("swap-third");
    let fourth_root = TestRoot::new("swap-fourth");
    write_source(third_root.path(), "empty.tsf", MINIMAL_SOURCE);
    write_source(fourth_root.path(), "empty.tsf", MINIMAL_SOURCE);
    let (third, third_raw, third_decoded) = open_and_decode(third_root.path(), &package, &limits);
    let third_sources = third.admit_sources(&third_decoded, &limits).unwrap();
    let (fourth, fourth_raw, fourth_decoded) =
        open_and_decode(fourth_root.path(), &package, &limits);
    let fourth_sources = fourth.admit_sources(&fourth_decoded, &limits).unwrap();
    let error = fourth
        .finish(fourth_raw, fourth_decoded, third_sources)
        .expect_err("source-set receipt swap");
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::ReceiptSessionMismatch(MachineInputReceiptKind::SourceSet)
    ));
    drop((third, third_raw, third_decoded, fourth_sources));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn portable_fingerprint_excludes_root_session_and_config() {
    let first_root = TestRoot::new("fingerprint-first");
    let second_root = TestRoot::new("fingerprint-second");
    let package = package_for_source(0, "empty.tsf", MINIMAL_SOURCE);
    write_source(first_root.path(), "empty.tsf", MINIMAL_SOURCE);
    write_source(second_root.path(), "empty.tsf", MINIMAL_SOURCE);
    let first_limits = default_limits();
    let second_limits = limits_with(|limits| limits.max_pages -= 1);

    let (first, first_raw, first_decoded) =
        open_and_decode(first_root.path(), &package, &first_limits);
    let first_sources = first.admit_sources(&first_decoded, &first_limits).unwrap();
    let first = first
        .finish(first_raw, first_decoded, first_sources)
        .unwrap();

    let (second, second_raw, second_decoded) =
        open_and_decode(second_root.path(), &package, &second_limits);
    let second_sources = second
        .admit_sources(&second_decoded, &second_limits)
        .unwrap();
    let second = second
        .finish(second_raw, second_decoded, second_sources)
        .unwrap();

    assert_ne!(first.session_identity(), second.session_identity());
    assert_eq!(first.fingerprint(), second.fingerprint());
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn failure_retains_missing_package_candidate_for_publication_guard() {
    let root = TestRoot::new("missing-package-ledger");
    let package = root.path().join("missing-package.json");
    let error = HostMachineInputSession::open(options(&package, None), &default_limits())
        .expect_err("missing PACKAGE");
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::PackageOpen(HostAdmissionError::MissingCandidate)
    ));
    let token = error.read_ledger_token().unwrap();
    assert!(token
        .conflicts_with_write_target(&HostPath::new(package.clone()).unwrap())
        .unwrap());
    assert!(!package.exists());

    let second = HostMachineInputSession::open(options(&package, None), &default_limits())
        .expect_err("missing PACKAGE");
    let (kind, progress, token) = second.into_parts_with_read_ledger().unwrap();
    assert!(matches!(
        kind,
        MachineInputErrorKind::PackageOpen(HostAdmissionError::MissingCandidate)
    ));
    assert_eq!(progress.stage(), MachineInputStage::NoInput);
    assert!(token
        .conflicts_with_write_target(&HostPath::new(package).unwrap())
        .unwrap());
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn config_package_and_source_share_one_publication_read_ledger() {
    let root = TestRoot::new("shared-config-ledger");
    let config_path = root.path().join("typaxis.toml");
    fs::write(&config_path, b"contract = \"typaxis.contract/1.0\"\n").unwrap();
    write_source(root.path(), "empty.tsf", MINIMAL_SOURCE);
    let package_bytes = package_for_source(0, "empty.tsf", MINIMAL_SOURCE);
    let package_path = write_package(root.path(), &package_bytes);

    let ledger = HostReadIdentityLedger::new();
    let root_path = HostPath::new(root.path().to_path_buf()).unwrap();
    let config_session =
        HostAdmissionSession::new_contained_root_with_read_ledger(&root_path, &ledger).unwrap();
    drop(
        config_session
            .roots()
            .open(&PortablePath::new("typaxis.toml").unwrap())
            .unwrap(),
    );
    let limits = default_limits();
    let (session, raw) = HostMachineInputSession::open_with_read_ledger(
        options(&package_path, None),
        &limits,
        &ledger,
    )
    .unwrap();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let decoded = session
        .decode_and_bind(&raw, &StrictDocumentPackageDecoder::new(), &policy)
        .unwrap();
    let sources = session.admit_sources(&decoded, &limits).unwrap();
    let package = session.finish(raw, decoded, sources).unwrap();
    let token = package.read_ledger_token().unwrap();
    for path in [config_path, package_path, root.path().join("empty.tsf")] {
        assert!(token
            .conflicts_with_write_target(&HostPath::new(path).unwrap())
            .unwrap());
    }
}

#[cfg(not(any(target_os = "android", target_os = "linux", target_os = "macos")))]
#[test]
fn unsupported_target_fails_before_package_bytes() {
    let limits = default_limits();
    let options =
        MachineInputHostOptions::new(HostPath::new("does-not-need-to-exist.json").unwrap(), None);
    let error = HostMachineInputSession::open(options, &limits).expect_err("unsupported target");
    assert!(matches!(
        error.kind(),
        MachineInputErrorKind::UnsupportedContainedOpen
    ));
    assert_eq!(error.progress().stage(), MachineInputStage::NoInput);
}
