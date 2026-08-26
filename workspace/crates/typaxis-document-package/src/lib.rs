#![forbid(unsafe_code)]

//! Untrusted DocumentPackage wire boundary.
//!
//! The public DTOs in this crate are intentionally caller-constructible. They
//! describe wire data and confer no admission or syntax-validation trust.

mod decode;
mod encoder;
mod error;
mod jcs;
mod location;
mod model;
mod preflight;

pub use decode::{
    DecodedDocumentPackage, DocumentPackageDecodeError, DocumentPackageDecodeErrorClass,
    DocumentPackageDecodeLimit, DocumentPackageDecodeLocation, DocumentPackageDecodePolicy,
    DocumentPackageDecodePrimary, DocumentPackageTypedDecodeError,
    DocumentPackageTypedDecodeErrorKind, StrictDocumentPackageDecoder,
};
pub use encoder::{CanonicalJcsStats, DocumentPackageEncoder, JcsCountHashSink, JcsEncodeError};
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
pub use typaxis_core::{DocumentPackageContractId, JsonPointer, MachineInputLimitBounds};
