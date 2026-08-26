use std::panic::{catch_unwind, AssertUnwindSafe};

use typaxis_core::{MachineInputLimitBounds, ResourceLimits, ValidatedResourceLimits};
use typaxis_document_package::{
    DocumentPackageDecodePolicy, DocumentPackageEncoder, JsonPreflightErrorClass,
    StrictDocumentPackageDecoder, StrictJsonPreflight,
};

const BLANK_PACKAGE: &[u8] = include_bytes!(
    "../../../../samples/machine-package/profiles/paragraph-1/blank-1.1/job/document-package.json"
);

fn limits() -> ValidatedResourceLimits {
    ValidatedResourceLimits::new(ResourceLimits::default()).unwrap()
}

fn nested_document(depth: u16) -> Vec<u8> {
    assert!(depth >= 1);
    let mut input = b"{\"value\":".to_vec();
    input.resize(input.len() + usize::from(depth - 1), b'[');
    input.extend_from_slice(b"null");
    input.resize(input.len() + usize::from(depth - 1), b']');
    input.push(b'}');
    input
}

#[test]
fn arbitrary_json_bytes_are_total_and_never_panic() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let decoder = StrictDocumentPackageDecoder::new();
    let mut state = 0x7a11_c0de_5eed_u64;
    for length in 0..=1_024usize {
        let mut bytes = Vec::with_capacity(length);
        for _ in 0..length {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            bytes.push(state as u8);
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| decoder.decode(&bytes, &policy)));
        assert!(
            outcome.is_ok(),
            "decoder panicked for {length} arbitrary bytes"
        );
    }
}

#[test]
fn escaped_unicode_keys_cannot_hide_object_local_duplicates() {
    let escaped_keys = [
        r#"c\u006fntract"#,
        r#"contr\u0061ct"#,
        r#"\u0063\u006f\u006e\u0074\u0072\u0061\u0063\u0074"#,
    ];
    for escaped in escaped_keys {
        let mut duplicate = Vec::from(&b"{"[..]);
        duplicate.extend_from_slice(format!(r#""{escaped}":"typaxis.contract/1.1","#).as_bytes());
        duplicate.extend_from_slice(&BLANK_PACKAGE[1..]);
        let error = StrictJsonPreflight::default()
            .check(&duplicate)
            .expect_err("decoded duplicate member must fail");
        assert_eq!(error.class(), JsonPreflightErrorClass::JsonSyntax);
        assert_eq!(error.location().json_pointer().as_str(), "/contract");
    }
}

#[test]
fn hard_depth_boundary_is_inclusive_and_iterative() {
    let exact_depth = MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH;
    let exact = nested_document(exact_depth);
    assert_eq!(
        StrictJsonPreflight::default()
            .check(&exact)
            .unwrap()
            .maximum_depth(),
        exact_depth
    );

    let over = nested_document(exact_depth + 1);
    let error = StrictJsonPreflight::default().check(&over).unwrap_err();
    assert_eq!(
        error.class(),
        JsonPreflightErrorClass::JsonNestingDepthLimit
    );
    assert_eq!(
        error
            .location()
            .json_pointer()
            .as_str()
            .matches('/')
            .count(),
        usize::from(exact_depth)
    );
}

#[test]
fn canonical_hash_properties_distinguish_formatting_from_semantics() {
    let limits = limits();
    let policy = DocumentPackageDecodePolicy::new(&limits);
    let decoder = StrictDocumentPackageDecoder::new();
    let canonical = decoder.decode(BLANK_PACKAGE, &policy).unwrap();

    let mut formatted = DocumentPackageEncoder::default()
        .to_jcs_vec(canonical.wire())
        .unwrap();
    formatted.insert(1, b' ');
    let formatted = decoder.decode(&formatted, &policy).unwrap();
    assert_ne!(canonical.raw_sha256(), formatted.raw_sha256());
    assert_eq!(
        canonical.canonical_jcs_sha256(),
        formatted.canonical_jcs_sha256()
    );

    let mut changed = canonical.wire().clone();
    changed.page_masters.masters[0].width += 1;
    let changed = DocumentPackageEncoder::default()
        .to_jcs_vec(&changed)
        .unwrap();
    let changed = decoder.decode(&changed, &policy).unwrap();
    assert_ne!(
        canonical.canonical_jcs_sha256(),
        changed.canonical_jcs_sha256()
    );
}
