#![forbid(unsafe_code)]

//! Untrusted DocumentPackage wire boundary.
//!
//! The public DTOs in this crate are intentionally caller-constructible. They
//! describe wire data and confer no admission or syntax-validation trust.

mod advanced;
mod decode;
mod encoder;
mod error;
mod jcs;
mod location;
mod model;
mod preflight;
mod semantic_container;

pub use advanced::{
    DecodedStagingAdvancedDocumentPackage, StagingAdvancedDecodeError,
    StagingAdvancedDocumentPackageDecoder, STAGING_ADVANCED_DOCUMENT_PACKAGE_CONTRACT,
};
pub use decode::{
    DecodedDocumentPackage, DecodedStagingStyleDocumentPackage, DocumentPackageDecodeError,
    DocumentPackageDecodeErrorClass, DocumentPackageDecodeLimit, DocumentPackageDecodeLocation,
    DocumentPackageDecodePolicy, DocumentPackageDecodePrimary, DocumentPackageTypedDecodeError,
    DocumentPackageTypedDecodeErrorKind, StagingStyleDocumentPackageDecoder,
    StrictDocumentPackageDecoder,
};
pub use encoder::{
    CanonicalJcsStats, DocumentPackageEncoder, JcsCountHashSink, JcsEncodeError,
    StagingStyleDocumentPackageEncoder,
};
pub use error::*;
pub use jcs::{
    CanonicalDocumentPackageHash, CanonicalDocumentPackageJcsSha256, RawDocumentPackageHash,
    RawDocumentPackageSha256,
};
pub use location::{DocumentPackageRootMember, JsonLocationIndex};
pub use model::*;
pub use preflight::{
    DocumentPackageByteLimit, DocumentPackagePreflightLimits, JsonNestingDepthLimit,
    JsonPreflightReport, StrictJsonPreflight,
};
pub use semantic_container::{
    DecodedStagingSemanticDocumentPackage, StagingSemanticDecodeError,
    StagingSemanticDocumentPackageDecoder, StagingSemanticDocumentPackageEncoder,
    WireFontMediaType, WireImageMediaType, WireStagingByteRange, WireStagingM4Block,
    WireStagingM4Document, WireStagingM4DocumentPackage, WireStagingM4FontFace,
    WireStagingM4Footnote, WireStagingM4Image, WireStagingM4Inline, WireStagingM4LinkTarget,
    WireStagingM4ListItem, WireStagingM4ReferenceFormat, WireStagingM4ResourceCatalog,
    WireStagingM4Source, WireStagingM4TableCell, WireStagingM4TableRow, WireStagingM4TextBuffer,
    WireStagingMathSource, WireStagingSemanticContainerKind, WireStagingSourceSpan,
    WireStagingStyleDeclaration, WireStagingStyleRule, WireStagingStyleSheet,
    WireStagingStyleValue, WireStagingTextMapKind, WireStagingTextMapSegment, WireStagingTextSpan,
    STAGING_SEMANTIC_DOCUMENT_PACKAGE_CONTRACT,
};
pub use typaxis_core::{DocumentPackageContractId, JsonPointer, MachineInputLimitBounds};
