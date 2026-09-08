#![cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use super::*;
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use typaxis_core::{
    ConfigResourceRoot, EffectiveDataVersions, FontFaceId, HostPath, M4ResourceLimits,
    PdfStreamCompression, ResourceLimits, ValidatedResourceLimits, DEFAULT_ALLOWED_URI_SCHEMES,
    REGISTERED_JAPANESE_LINE_BREAK_VERSION, REGISTERED_UNICODE_VERSION,
};
use typaxis_document_package::DocumentPackageDecodePolicy;
use typaxis_machine_input::{book_v2::HostBookV2InputSession, MachineInputHostOptions};
use typaxis_syntax::book_v2::{
    prepare_admitted_book_v2_body, prepare_book_v2_navigation, prepare_book_v2_text_flow,
};
const FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));
const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/body.bin"
));
const PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/cover.bin"
));
const COLLECTION: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/collection.bin"));
const SOURCE: &[u8] = b"ResultProofExercise\n";
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "typaxis-book-v2-resources-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn context(&self) -> HostAdmissionContext {
        HostAdmissionContext::new(
            HostPath::new(self.0.join("document-package.json")).unwrap(),
            HostPath::new(self.0.clone()).unwrap(),
            None,
            vec![],
        )
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn config(limits: ResourceLimits) -> EffectiveConfig {
    EffectiveConfig::new(
        false,
        PdfStreamCompression::Flate,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        EffectiveDataVersions::new(
            REGISTERED_UNICODE_VERSION,
            REGISTERED_JAPANESE_LINE_BREAK_VERSION,
        )
        .unwrap(),
        limits,
    )
    .unwrap()
}
fn limits() -> M4EffectiveResourceLimits {
    M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap()
}
fn data() -> Value {
    let mut data: Value = serde_json::from_slice(FIXTURE).unwrap();
    data["contract"] = "typaxis.contract/1.5".into();
    data["document"]["blocks"][0]["semantic_kind"] = "solution".into();
    data["style_sheet"]["rules"].as_array_mut().unwrap().push(json!({"style_id":"paragraph-text","selector":"paragraph","source_order":2,"extends":null,"declarations":[
        {"name":"font_family","important":false,"value":{"kind":"font_family_list","families":["Body"]}},
        {"name":"font_size","important":false,"value":{"kind":"length","value":786432}},
        {"name":"line_height","important":false,"value":{"kind":"length","value":1048576}}
    ]}));
    data
}
fn body(root: &Root, data: Value) -> SourceAdmittedBookV2Body {
    body_with_source(root, data, SOURCE, &limits())
}
fn body_with_source(
    root: &Root,
    mut data: Value,
    source: &[u8],
    limits: &M4EffectiveResourceLimits,
) -> SourceAdmittedBookV2Body {
    data["sources"][0]["sha256"] = typaxis_core::sha256(source)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into();
    data["sources"][0]["utf8_byte_length"] = source.len().into();
    fs::write(
        root.0.join("document-package.json"),
        serde_json::to_vec(&data).unwrap(),
    )
    .unwrap();
    fs::write(root.0.join("input.tsf"), source).unwrap();
    fs::write(root.0.join("body.bin"), FONT).unwrap();
    fs::write(root.0.join("collection.bin"), COLLECTION).unwrap();
    fs::write(root.0.join("cover.bin"), PNG).unwrap();
    let (session, raw) = HostBookV2InputSession::open(
        MachineInputHostOptions::new(
            HostPath::new(root.0.join("document-package.json")).unwrap(),
            None,
        ),
        limits.base(),
    )
    .unwrap();
    let decoded = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(limits.base()))
        .unwrap();
    let sources = session.admit_sources(&decoded, limits.base()).unwrap();
    prepare_admitted_book_v2_body(
        session.finish(raw, decoded, sources).unwrap(),
        limits.base(),
    )
    .unwrap()
}
#[test]
fn book_v2_resources_join_stable_sources_real_fonts_images_and_successor_flow() {
    let root = Root::new();
    let body = body(&root, data());
    let expected = prepare_book_v2_resource_policy(&body, &limits())
        .unwrap()
        .fingerprint();
    let prepared = prepare_book_v2_resources(
        body,
        &root.context(),
        &config(ResourceLimits::default()),
        &limits(),
    )
    .unwrap();
    assert_eq!(prepared.resources().profile_fingerprint(), expected);
    assert_eq!(
        prepared.resources().resource_set_id(),
        "typaxis.production-book-resource-set/3"
    );
    assert_eq!(prepared.resources().fonts().len(), 2);
    assert_eq!(prepared.resources().images().len(), 1);
    assert_eq!(prepared.resources().fonts()[0].bytes(), FONT);
    assert_eq!(prepared.resources().images()[0].bytes(), PNG);
    let token = prepared.body().provenance().read_ledger_token().unwrap();
    assert_eq!(token.stored_opened_identity_count(), 5);
    assert_eq!(token.stored_candidate_identity_count(), 5);
    let navigation = prepare_book_v2_navigation(prepared.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(prepared.body().styled(), &navigation).unwrap();
    assert_eq!(flow.paragraphs().len(), 3);
    assert_eq!(
        flow.semantic_container_style(typaxis_core::NodeId::new(1))
            .unwrap()
            .semantic_kind()
            .as_str(),
        "solution"
    );
    fs::write(root.0.join("body.bin"), b"changed after stable read").unwrap();
    assert_eq!(prepared.resources().fonts()[0].bytes(), FONT);
}
#[test]
fn book_v2_resource_policy_rejects_same_hash_body_replacement_and_changed_limits() {
    let root = Root::new();
    let other_root = Root::new();
    let first = body(&root, data());
    let second = body(&other_root, data());
    let limits = limits();
    let policy = prepare_book_v2_resource_policy(&first, &limits).unwrap();
    assert_eq!(
        policy.fingerprint(),
        prepare_book_v2_resource_policy(&second, &limits)
            .unwrap()
            .fingerprint()
    );
    assert!(policy.verify_for(&second, &limits).is_err());
    let mut base = ResourceLimits::default();
    base.max_pages -= 1;
    let changed = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(base).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    assert!(prepare_book_v2_resource_policy(&first, &changed).is_err());
    assert!(policy.verify_for(&first, &changed).is_err());
    let mut extension = M4ResourceLimits::default();
    extension.max_font_glyphs -= 1;
    let changed_extension =
        M4EffectiveResourceLimits::new(limits.base().clone(), extension).unwrap();
    let other_policy = prepare_book_v2_resource_policy(&first, &changed_extension).unwrap();
    assert_ne!(policy.fingerprint(), other_policy.fingerprint());
    assert!(policy.verify_for(&first, &changed_extension).is_err());
    let parsed: Value = serde_json::from_str(policy.canonical_jcs()).unwrap();
    assert_eq!(
        parsed["resource_set"],
        "typaxis.production-book-resource-set/3"
    );
    assert_eq!(
        policy.fingerprint(),
        typaxis_core::sha256(policy.canonical_jcs().as_bytes())
    );
}
#[test]
fn book_v2_resource_failure_retains_every_candidate_and_original_source() {
    let root = Root::new();
    let body = body(&root, data());
    fs::remove_file(root.0.join("body.bin")).unwrap();
    let error = prepare_book_v2_resources(
        body,
        &root.context(),
        &config(ResourceLimits::default()),
        &limits(),
    )
    .unwrap_err();
    assert!(matches!(
        error.failure(),
        BookV2ResourceFailure::Resource(_)
    ));
    assert_eq!(
        error.subject(),
        Some(&ResourceErrorSubject::FontFace(FontFaceId::new(0)))
    );
    assert_eq!(error.body().sources()[0].text().as_bytes(), SOURCE);
    let token = error.body().provenance().read_ledger_token().unwrap();
    assert_eq!(token.stored_candidate_identity_count(), 5);
    assert_eq!(token.stored_opened_identity_count(), 2);
}
#[test]
fn book_v2_resource_config_mismatch_stops_before_resource_candidates() {
    let root = Root::new();
    let body = body(&root, data());
    let mut changed = ResourceLimits::default();
    changed.max_pages -= 1;
    let error =
        prepare_book_v2_resources(body, &root.context(), &config(changed), &limits()).unwrap_err();
    assert!(matches!(
        error.failure(),
        BookV2ResourceFailure::ConfigLimits
    ));
    assert!(error.subject().is_none());
    assert_eq!(
        error
            .body()
            .provenance()
            .read_ledger_token()
            .unwrap()
            .stored_candidate_identity_count(),
        2
    );
}
#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the original font"]
fn book_v2_source_owner_admits_original_harano_under_resource_set_three() {
    let bytes = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    let hash = typaxis_core::sha256(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        hash,
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let root = Root::new();
    let mut data = data();
    data["resources"]["font_faces"][0]["media_type"] = "sfnt-cff1".into();
    data["resources"]["font_faces"][0]["expected_sha256"] = hash.into();
    let body = body(&root, data);
    fs::write(root.0.join("body.bin"), &bytes).unwrap();
    let prepared = prepare_book_v2_resources(
        body,
        &root.context(),
        &config(ResourceLimits::default()),
        &limits(),
    )
    .unwrap();
    let font = prepared.resources().font(FontFaceId::new(0)).unwrap();
    assert!(matches!(
        font,
        typaxis_resources::AdmittedProductionFontV3::Cff1V2(_)
    ));
    assert_eq!(font.bytes(), bytes);
    assert_eq!(font.metadata().glyph_count, 23060);
}

#[test]
fn book_v2_resource_bytes_cannot_replace_a_declared_hash() {
    let root = Root::new();
    let body = body(&root, data());
    let mut bytes = FONT.to_vec();
    bytes[0] ^= 1;
    fs::write(root.0.join("body.bin"), bytes).unwrap();
    let error = prepare_book_v2_resources(
        body,
        &root.context(),
        &config(ResourceLimits::default()),
        &limits(),
    )
    .unwrap_err();
    assert!(matches!(
        error.failure(),
        BookV2ResourceFailure::Admission(ProductionResourceErrorV3::Resource(
            ResourceAdmissionError::ExpectedHashMismatch
        ))
    ));
    assert_eq!(
        error
            .body()
            .provenance()
            .read_ledger_token()
            .unwrap()
            .stored_candidate_identity_count(),
        5
    );
}

#[test]
fn book_v2_resource_extension_config_mismatch_stops_before_resource_candidates() {
    let root = Root::new();
    let body = body(&root, data());
    let mut extension = M4ResourceLimits::default();
    extension.max_font_glyphs -= 1;
    // This existing config type supplies host settings only. It is never emitted
    // as the successor's config artifact or used to select a public profile.
    let config = EffectiveConfig::new_for_contract_with_m4_limits(
        typaxis_core::DocumentPackageContractId::V1_4,
        false,
        PdfStreamCompression::Flate,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        EffectiveDataVersions::new(
            REGISTERED_UNICODE_VERSION,
            REGISTERED_JAPANESE_LINE_BREAK_VERSION,
        )
        .unwrap(),
        ResourceLimits::default(),
        extension,
    )
    .unwrap();
    let error = prepare_book_v2_resources(body, &root.context(), &config, &limits()).unwrap_err();
    assert!(matches!(
        error.failure(),
        BookV2ResourceFailure::ConfigLimits
    ));
    assert!(error.subject().is_none());
    assert_eq!(
        error
            .body()
            .provenance()
            .read_ledger_token()
            .unwrap()
            .stored_candidate_identity_count(),
        2
    );
}

#[path = "book_v2_shaping_tests.rs"]
mod shaping_tests;
