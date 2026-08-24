#![no_main]

use libfuzzer_sys::fuzz_target;
use typaxis_core::{PortablePath, ResourceLimits, SourceId, ValidatedResourceLimits};
use typaxis_syntax::{PackageValidationPolicy, Parser, ReferenceParser, SourceFile};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
    let policy = PackageValidationPolicy::new(&limits, &schemes).unwrap();
    let source = SourceFile {
        source_id: SourceId::new(0),
        uri: PortablePath::new("fuzz-input.tsf").unwrap(),
        text: text.to_owned(),
    };
    let _ = ReferenceParser::new().parse(&source, &policy);
});
