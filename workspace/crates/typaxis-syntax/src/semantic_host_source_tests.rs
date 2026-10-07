#![cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use super::*;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use typaxis_core::{HostPath, ResourceLimits};
use typaxis_document_package::{
    DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
};
use typaxis_machine_input::{HostMachineInputSession, MachineInputHostOptions, MachineInputStage};
const FIXTURE:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));
const SOURCE: &[u8] = b"ResultProofExercise\n";
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "typaxis-semantic-host-source-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn input() -> Value {
    serde_json::from_slice(FIXTURE).unwrap()
}
fn parse(mut data: Value, source: &[u8]) -> ProductionMachineParseOutcome {
    let root = Root::new();
    let bound = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    data["sources"][0]["sha256"] = sha256(source)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    data["sources"][0]["utf8_byte_length"] = source.len().into();
    fs::write(root.0.join("input.tsf"), source).unwrap();
    fs::write(
        root.0.join("document-package.json"),
        serde_json::to_vec(&data).unwrap(),
    )
    .unwrap();
    let options = MachineInputHostOptions::new(
        HostPath::new(root.0.join("document-package.json")).unwrap(),
        None,
    );
    let (session, raw) = HostMachineInputSession::open(options, &bound).unwrap();
    let decoded = session
        .decode_semantic_and_bind(
            &raw,
            &StagingSemanticDocumentPackageDecoder::new(),
            &DocumentPackageDecodePolicy::new(&bound),
        )
        .unwrap();
    let sources = session.admit_semantic_sources(&decoded, &bound).unwrap();
    let admitted = session.finish_semantic(raw, decoded, sources).unwrap();
    StagingSemanticPackageParser::new().parse_admitted(admitted, &bound)
}
fn assert_source_failure(outcome: ProductionMachineParseOutcome) {
    match outcome {
        ProductionMachineParseOutcome::Failed { progress, failure } => {
            assert_eq!(progress.stage(), MachineInputStage::SourcesAdmitted);
            assert_eq!(failure, StagingSemanticSyntaxError::InvalidSourceSpan);
        }
        _ => panic!("inconsistent host source must not issue a production syntax receipt"),
    }
}
#[test]
fn legacy_production_source_admission_rejects_same_length_identity_substitution() {
    assert!(matches!(
        parse(input(), SOURCE),
        ProductionMachineParseOutcome::Parsed { .. }
    ));
    let mut data = input();
    data["text_buffers"][0]["utf8"] = "resultProofExercise".into();
    assert_source_failure(parse(data, SOURCE));
}
#[test]
fn legacy_production_source_admission_checks_complete_maps_and_actual_source_boundaries() {
    let mut data = input();
    data["text_buffers"][0]["mappings"][0]["text_range"]["start_byte"] = 1.into();
    assert_source_failure(parse(data, SOURCE));
    let mut data = input();
    data["text_buffers"][0]["mappings"] = json!([]);
    assert_source_failure(parse(data, SOURCE));
    let mut data = input();
    data["text_buffers"][0]["mappings"][0]["kind"] = "replacement".into();
    data["document"]["blocks"][0]["blocks"][0]["children"][0]["span"]["start_byte"] = 1.into();
    assert_source_failure(parse(data, "日ultProofExercise\n".as_bytes()));
}
#[test]
fn legacy_production_source_admission_preserves_explicit_replacement_and_inserted_text() {
    let mut data = input();
    data["text_buffers"][0]["utf8"] = "AnswerProofExercise".into();
    data["text_buffers"][0]["mappings"] = json!([
        {"kind":"replacement","text_range":{"start_byte":0,"end_byte":6},"source_span":{"source_id":0,"start_byte":0,"end_byte":6}},
        {"kind":"inserted","text_range":{"start_byte":6,"end_byte":19},"source_span":null}
    ]);
    assert!(matches!(
        parse(data, SOURCE),
        ProductionMachineParseOutcome::Parsed { .. }
    ));
}
