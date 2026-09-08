#![cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use super::*;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use typaxis_core::{HostPath, ResourceLimits};
use typaxis_document_package::DocumentPackageDecodePolicy;
use typaxis_machine_input::{book_v2::HostBookV2InputSession, MachineInputHostOptions};
const FIXTURE:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));
const SOURCE: &[u8] = b"ResultProofExercise\n";
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "typaxis-book-v2-source-{}-{}",
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
fn limits() -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits::default()).unwrap()
}
fn input() -> Value {
    let mut input: Value = serde_json::from_slice(FIXTURE).unwrap();
    input["contract"] = "typaxis.contract/1.5".into();
    input["document"]["blocks"][0]["semantic_kind"] = "solution".into();
    input["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"paragraph-text","selector":"paragraph","source_order":2,"extends":null,"declarations":[
        {"name":"font_family","important":false,"value":{"kind":"font_family_list","families":["Body"]}},
        {"name":"font_size","important":false,"value":{"kind":"length","value":786432}},
        {"name":"line_height","important":false,"value":{"kind":"length","value":1048576}}
    ]}));
    input
}
fn admitted(root: &Root, mut input: Value, source: &[u8]) -> AdmittedBookV2Input {
    input["sources"][0]["sha256"] = hex_sha(sha256(source)).into();
    input["sources"][0]["utf8_byte_length"] = source.len().into();
    fs::write(root.0.join("input.tsf"), source).unwrap();
    fs::write(
        root.0.join("document-package.json"),
        serde_json::to_vec(&input).unwrap(),
    )
    .unwrap();
    let bound = limits();
    let options = MachineInputHostOptions::new(
        HostPath::new(root.0.join("document-package.json")).unwrap(),
        None,
    );
    let (session, raw) = HostBookV2InputSession::open(options, &bound).unwrap();
    let decoded = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(&bound))
        .unwrap();
    let sources = session.admit_sources(&decoded, &bound).unwrap();
    session.finish(raw, decoded, sources).unwrap()
}
#[test]
fn stable_source_body_reaches_navigation_and_flow_without_legacy_receipts() {
    let root = Root::new();
    let admitted = admitted(&root, input(), SOURCE);
    let ptr = admitted.sources()[0].text().as_ptr();
    fs::write(root.0.join("input.tsf"), b"modified after read").unwrap();
    let body = prepare_admitted_book_v2_body(admitted, &limits()).unwrap();
    assert_eq!(body.sources()[0].text().as_bytes(), SOURCE);
    assert_eq!(body.sources()[0].text().as_ptr(), ptr);
    let navigation = prepare_book_v2_navigation(body.styled()).unwrap();
    let flow = prepare_book_v2_text_flow(body.styled(), &navigation).unwrap();
    flow.verify_for(body.styled(), &navigation).unwrap();
    assert_eq!(
        flow.semantic_container_style(NodeId::new(1))
            .unwrap()
            .semantic_kind()
            .as_str(),
        "solution"
    );
    assert_eq!(flow.paragraphs().len(), 3);
    assert_eq!(
        body.provenance()
            .read_ledger_token()
            .unwrap()
            .stored_opened_identity_count(),
        2
    );
}
#[test]
fn identity_mapping_must_match_the_actual_host_source_bytes() {
    let root = Root::new();
    let mut data = input();
    data["text_buffers"][0]["utf8"] = "resultProofExercise".into();
    let error =
        prepare_admitted_book_v2_body(admitted(&root, data, SOURCE), &limits()).unwrap_err();
    assert!(matches!(
        error.failure(),
        BookV2SourceFailure::TextMapping {
            text_id: 0,
            mapping: 0,
            reason: BookV2MappingFailure::IdentityBytes
        }
    ));
    assert_eq!(
        error.provenance().progress().stage(),
        MachineInputStage::SourcesAdmitted
    );
    assert_eq!(
        error
            .provenance()
            .read_ledger_token()
            .unwrap()
            .stored_opened_identity_count(),
        2
    );
}
#[test]
fn mapped_text_ranges_must_cover_each_buffer_at_utf8_boundaries() {
    for variant in ["gap", "empty", "boundary", "missing"] {
        let root = Root::new();
        let mut data = input();
        match variant {
            "gap" => data["text_buffers"][0]["mappings"][0]["text_range"]["start_byte"] = 1.into(),
            "empty" => data["text_buffers"][0]["mappings"][0]["text_range"]["end_byte"] = 0.into(),
            "boundary" => {
                data["text_buffers"][0]["utf8"] = "日ultProofExercise".into();
                data["text_buffers"][0]["mappings"][0]["text_range"]["end_byte"] = 1.into();
            }
            "missing" => data["text_buffers"][0]["mappings"] = json!([]),
            _ => unreachable!(),
        }
        let error =
            prepare_admitted_book_v2_body(admitted(&root, data, SOURCE), &limits()).unwrap_err();
        assert!(
            matches!(error.failure(), BookV2SourceFailure::TextMapping { .. }),
            "{variant}: {error:?}"
        );
    }
}
#[test]
fn original_node_spans_must_land_on_actual_source_utf8_boundaries() {
    let root = Root::new();
    let mut data = input();
    data["text_buffers"][0]["mappings"][0]["kind"] = "replacement".into();
    data["document"]["blocks"][0]["blocks"][0]["children"][0]["span"]["start_byte"] = 1.into();
    let error = prepare_admitted_book_v2_body(
        admitted(&root, data, "日ultProofExercise\n".as_bytes()),
        &limits(),
    )
    .unwrap_err();
    assert!(
        matches!(
            error.failure(),
            BookV2SourceFailure::SourceSpan {
                node_id: 3,
                source_id: 0,
                start_byte: 1,
                ..
            }
        ),
        "{error:?}"
    );
}
#[test]
fn declared_replacement_and_inserted_text_preserve_their_distinct_provenance() {
    let root = Root::new();
    let mut data = input();
    data["text_buffers"][0]["utf8"] = "AnswerProofExercise".into();
    data["text_buffers"][0]["mappings"] = json!([
        {"kind":"replacement","text_range":{"start_byte":0,"end_byte":6},"source_span":{"source_id":0,"start_byte":0,"end_byte":6}},
        {"kind":"inserted","text_range":{"start_byte":6,"end_byte":19},"source_span":null}
    ]);
    let body = prepare_admitted_book_v2_body(admitted(&root, data, SOURCE), &limits()).unwrap();
    let buffer = &body.styled().body().wire().text_buffers()[0];
    assert_eq!(buffer.utf8, "AnswerProofExercise");
    assert_eq!(buffer.mappings[0].kind, WireStagingTextMapKind::Replacement);
    assert_eq!(buffer.mappings[1].kind, WireStagingTextMapKind::Inserted);
    assert_eq!(body.sources()[0].text().as_bytes(), SOURCE);
}
#[test]
fn source_body_rejects_limits_different_from_host_admission() {
    let root = Root::new();
    let mut bound = ResourceLimits::default();
    bound.max_pages -= 1;
    let error = prepare_admitted_book_v2_body(
        admitted(&root, input(), SOURCE),
        &ValidatedResourceLimits::new(bound).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(
        error.failure(),
        BookV2SourceFailure::ReceiptMismatch
    ));
}
