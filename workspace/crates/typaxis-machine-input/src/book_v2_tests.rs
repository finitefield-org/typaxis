#![cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use super::*;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use typaxis_core::ResourceLimits;
const FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "typaxis-book-v2-host-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn write(&self, name: &str, bytes: &[u8]) {
        fs::write(self.0.join(name), bytes).unwrap();
    }
    fn options(&self) -> MachineInputHostOptions {
        MachineInputHostOptions::new(
            HostPath::new(self.0.join("document-package.json")).unwrap(),
            Some(HostPath::new(self.0.clone()).unwrap()),
        )
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn limits() -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits::default()).unwrap()
}
fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn input(contents: &[(&str, &[u8])]) -> Value {
    let mut v: Value = serde_json::from_slice(FIXTURE).unwrap();
    v["contract"] = "typaxis.contract/1.5".into();
    v["document"]["blocks"][0]["semantic_kind"] = "solution".into();
    v["sources"]=json!(contents.iter().enumerate().map(|(i,(uri,bytes))|json!({"source_id":i,"uri":uri,"utf8_byte_length":bytes.len(),"sha256":hex(sha256(bytes))})).collect::<Vec<_>>());
    v
}
fn opened(
    root: &Root,
    v: &Value,
    bound: &ValidatedResourceLimits,
) -> (
    HostBookV2InputSession,
    BookV2PackageBytes,
    SessionBoundBookV2Decoded,
) {
    root.write("document-package.json", &serde_json::to_vec(v).unwrap());
    let (session, raw) = HostBookV2InputSession::open(root.options(), bound).unwrap();
    let decoded = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(bound))
        .unwrap();
    (session, raw, decoded)
}
#[test]
fn source_set_is_stable_portable_and_moves_its_original_buffers() {
    let content = [("a.tsf", b"abc".as_slice()), ("b.tsf", "日本".as_bytes())];
    let data = input(&content);
    let mut fingerprints = Vec::new();
    for _ in 0..2 {
        let root = Root::new();
        for (name, bytes) in content {
            root.write(name, bytes);
        }
        let (session, raw, decoded) = opened(&root, &data, &limits());
        let sources = session.admit_sources(&decoded, &limits()).unwrap();
        let ptr = sources.sources()[1].text().as_ptr();
        root.write("b.tsf", b"changed after stable read");
        let admitted = session.finish(raw, decoded, sources).unwrap();
        assert_eq!(admitted.sources()[1].text(), "日本");
        assert_eq!(admitted.sources()[1].text().as_ptr(), ptr);
        let progress = admitted.provenance().progress();
        assert_eq!(progress.stage(), MachineInputStage::SourcesAdmitted);
        assert_eq!(progress.decoded_contract(), Some("typaxis.contract/1.5"));
        assert_eq!(
            admitted
                .provenance()
                .read_ledger_token()
                .unwrap()
                .stored_opened_identity_count(),
            3
        );
        fingerprints.push(progress.fingerprint().unwrap());
        let (_, sources, _) = admitted.into_parts();
        assert_eq!(sources[1].text().as_ptr(), ptr);
    }
    assert_eq!(fingerprints[0], fingerprints[1]);
}
#[test]
fn same_hash_sessions_and_changed_limits_cannot_exchange_receipts() {
    let root = Root::new();
    root.write("a.tsf", b"abc");
    let data = input(&[("a.tsf", b"abc")]);
    let (first, raw, decoded) = opened(&root, &data, &limits());
    let (other, other_raw, other_decoded) = opened(&root, &data, &limits());
    assert!(matches!(
        other.admit_sources(&decoded, &limits()).unwrap_err().kind(),
        BookV2InputErrorKind::Host(MachineInputErrorKind::ReceiptSessionMismatch(_))
    ));
    let mut changed = ResourceLimits::default();
    changed.max_pages -= 1;
    assert!(matches!(
        first
            .admit_sources(&decoded, &ValidatedResourceLimits::new(changed).unwrap())
            .unwrap_err()
            .kind(),
        BookV2InputErrorKind::Host(MachineInputErrorKind::DecodePolicyMismatch)
    ));
    let sources = first.admit_sources(&decoded, &limits()).unwrap();
    let other_sources = other.admit_sources(&other_decoded, &limits()).unwrap();
    assert!(matches!(
        first
            .finish(raw, decoded, other_sources)
            .unwrap_err()
            .kind(),
        BookV2InputErrorKind::Host(MachineInputErrorKind::ReceiptSessionMismatch(_))
    ));
    assert!(matches!(
        other
            .finish(other_raw, other_decoded, sources)
            .unwrap_err()
            .kind(),
        BookV2InputErrorKind::Host(MachineInputErrorKind::ReceiptSessionMismatch(_))
    ));
}
#[test]
fn source_failures_retain_decoded_progress_and_attempted_read_identities() {
    for (actual, declared, expected) in [
        (b"ab".as_slice(), b"abc".as_slice(), "length"),
        (b"xyz", b"abc", "hash"),
        (&[0xff, 0xfe], &[0xff, 0xfe], "utf8"),
    ] {
        let root = Root::new();
        root.write("a.tsf", actual);
        let (session, _raw, decoded) = opened(&root, &input(&[("a.tsf", declared)]), &limits());
        let error = session.admit_sources(&decoded, &limits()).unwrap_err();
        assert_eq!(error.progress().stage(), MachineInputStage::PackageDecoded);
        assert!(error.progress().sources().is_empty());
        assert!(error.progress().fingerprint().is_none());
        assert_eq!(
            error.read_ledger_token().unwrap().candidate_attempt_count(),
            2
        );
        match (expected, error.kind()) {
            (
                "length",
                BookV2InputErrorKind::Host(MachineInputErrorKind::SourceLengthMismatch { .. }),
            )
            | (
                "hash",
                BookV2InputErrorKind::Host(MachineInputErrorKind::SourceHashMismatch { .. }),
            )
            | ("utf8", BookV2InputErrorKind::Host(MachineInputErrorKind::SourceNotUtf8 { .. })) => {
            }
            (_, error) => panic!("unexpected {error:?}"),
        }
    }
}
#[test]
fn source_totals_are_preflighted_before_any_source_open() {
    let root = Root::new();
    let data = input(&[("absent-a.tsf", b"abc"), ("absent-b.tsf", b"def")]);
    let bound = ValidatedResourceLimits::new(ResourceLimits {
        max_input_bytes: 5,
        max_source_bytes: 5,
        ..ResourceLimits::default()
    })
    .unwrap();
    let (session, _raw, decoded) = opened(&root, &data, &bound);
    let error = session.admit_sources(&decoded, &bound).unwrap_err();
    assert!(matches!(
        error.kind(),
        BookV2InputErrorKind::Host(MachineInputErrorKind::AggregateInputLimit {
            maximum: 5,
            attempted: 6
        })
    ));
    assert_eq!(
        error.read_ledger_token().unwrap().candidate_attempt_count(),
        1
    );
    root.write("absent-a.tsf", b"abc");
    root.write("absent-b.tsf", b"def");
    let bound = ValidatedResourceLimits::new(ResourceLimits {
        max_input_bytes: 6,
        max_source_bytes: 6,
        ..ResourceLimits::default()
    })
    .unwrap();
    let (session, raw, decoded) = opened(&root, &data, &bound);
    let sources = session.admit_sources(&decoded, &bound).unwrap();
    session.finish(raw, decoded, sources).unwrap();
}
#[test]
fn source_ids_counts_and_paths_remain_bounded_and_contained() {
    for variant in ["empty", "order", "count", "path", "missing"] {
        let root = Root::new();
        let mut data = input(&[("a.tsf", b"abc")]);
        let mut bound = ResourceLimits::default();
        match variant {
            "empty" => data["sources"] = json!([]),
            "order" => data["sources"][0]["source_id"] = 1.into(),
            "count" => {
                data = input(&[("a.tsf", b"abc"), ("b.tsf", b"def")]);
                bound.max_include_files = 1;
            }
            "path" => data["sources"][0]["uri"] = "../outside.tsf".into(),
            _ => {}
        }
        let bound = ValidatedResourceLimits::new(bound).unwrap();
        let (session, _raw, decoded) = opened(&root, &data, &bound);
        let error = session.admit_sources(&decoded, &bound).unwrap_err();
        assert_eq!(error.progress().stage(), MachineInputStage::PackageDecoded);
        match (variant, error.kind()) {
            ("empty" | "count", BookV2InputErrorKind::SourceCount { .. })
            | ("order", BookV2InputErrorKind::SourceOrder { .. })
            | ("path", BookV2InputErrorKind::Host(MachineInputErrorKind::UnsafeSourceUri { .. }))
            | ("missing", BookV2InputErrorKind::Host(MachineInputErrorKind::SourceOpen { .. })) => {
            }
            (_, e) => panic!("{variant}: {e:?}"),
        }
    }
}
#[test]
fn decoder_binds_only_its_session_raw_bytes_and_exact_successor_contract() {
    let root = Root::new();
    let mut data = input(&[("a.tsf", b"abc")]);
    data["contract"] = "typaxis.contract/1.4".into();
    root.write("document-package.json", &serde_json::to_vec(&data).unwrap());
    let (session, raw) = HostBookV2InputSession::open(root.options(), &limits()).unwrap();
    let error = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(&limits()))
        .unwrap_err();
    assert!(matches!(error.kind(), BookV2InputErrorKind::Decode(_)));
    assert_eq!(
        error.progress().stage(),
        MachineInputStage::RawPackageAdmitted
    );
    assert!(error.progress().decoded_contract().is_none());
    let data = input(&[("a.tsf", b"abc")]);
    let (session, raw, decoded) = opened(&root, &data, &limits());
    assert!(matches!(
        session
            .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(&limits()))
            .unwrap_err()
            .kind(),
        BookV2InputErrorKind::Host(MachineInputErrorKind::InvalidProgress { .. })
    ));
    assert!(
        matches!(&decoded.decoded().wire().document().blocks[0], typaxis_document_package::book_v2::WireBookV2Block::SemanticContainer { semantic_kind, .. } if semantic_kind.as_str() == "solution")
    );
}

#[test]
fn early_source_failure_still_registers_every_declared_source_for_publication_guard() {
    let root = Root::new();
    root.write("later.tsf", b"def");
    let data = input(&[("missing.tsf", b"abc"), ("later.tsf", b"def")]);
    let (session, _raw, decoded) = opened(&root, &data, &limits());
    let error = session.admit_sources(&decoded, &limits()).unwrap_err();
    assert!(matches!(
        error.kind(),
        BookV2InputErrorKind::Host(MachineInputErrorKind::SourceOpen { source_id: 0, .. })
    ));
    let ledger = error.read_ledger_token().unwrap();
    assert_eq!(ledger.candidate_attempt_count(), 3);
    assert_eq!(ledger.stored_candidate_identity_count(), 3);
    assert_eq!(ledger.stored_opened_identity_count(), 1);
    assert_eq!(fs::read(root.0.join("later.tsf")).unwrap(), b"def");
}
