use super::*;
use crate::{
    prepare_production_text_flow, validate_staging_book_navigation_v2, StagingSemanticPackageParser,
};
use typaxis_core::{sha256, M4EffectiveResourceLimits, ResourceLimits, ValidatedResourceLimits};
use typaxis_document_package::{
    DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
};

struct BoundedSink {
    used: usize,
    limit: usize,
}
impl Write for BoundedSink {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.used = self
            .used
            .checked_add(value.len())
            .filter(|n| *n <= self.limit)
            .ok_or(fmt::Error)?;
        Ok(())
    }
}

#[test]
fn streamed_source_flow_retains_legacy_identity_and_propagates_every_sink_boundary() {
    let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    let sample = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/profiles/production-book-1/combined/job/document-package.json"));
    let decoded = StagingSemanticDocumentPackageDecoder::new()
        .decode(sample, &DocumentPackageDecodePolicy::new(&limits))
        .unwrap();
    let package = StagingSemanticPackageParser::new()
        .parse(decoded, &limits)
        .unwrap();
    let extended = M4EffectiveResourceLimits::defaults_for(&limits);
    let nav = validate_staging_book_navigation_v2(&package, &extended).unwrap();
    let flow = prepare_production_text_flow(&package, &nav, &extended).unwrap();
    let emit = |out: &mut dyn Write| {
        write_source_flow(
            &flow,
            super::super::PRODUCTION_TEXT_FLOW_ALGORITHM,
            nav.languages().fingerprint(),
            nav.limits().fingerprint(),
            package.canonical_jcs_sha256(),
            out,
        )
    };
    let mut canonical = String::new();
    emit(&mut canonical).unwrap();
    let fingerprint: String = sha256(canonical.as_bytes())
        .iter()
        .map(|n| format!("{n:02x}"))
        .collect();
    assert_eq!(
        fingerprint,
        "4335b7770893a0a4021dbe57609b7d29455f164fdf58dbe24c6ba87eac037fd1"
    );
    assert_eq!(sha256(canonical.as_bytes()), flow.fingerprint());
    let value: serde_json::Value = serde_json::from_str(&canonical).unwrap();
    assert_eq!(
        value["algorithm"],
        super::super::PRODUCTION_TEXT_FLOW_ALGORITHM
    );
    for limit in 0..=canonical.len() {
        let mut out = BoundedSink { used: 0, limit };
        let result = emit(&mut out);
        if limit == canonical.len() {
            result.unwrap();
            assert_eq!(out.used, limit);
        } else {
            assert_eq!(result, Err(fmt::Error), "capacity {limit}");
            assert!(out.used <= limit);
        }
    }
}
