//! Non-current contract-1.5 carrier. This module is excluded from normal builds.
//! It preserves typed successor containers; it grants no syntax, profile,
//! resource, layout, PDF, or public-command admission.

use crate::semantic_container::{
    decode_semantic_document, encode_semantic_document, sealed_semantic_kind,
    DecodedVersionedSemanticDocumentPackage, WireSemanticBlock, WireSemanticDocument,
    WireSemanticDocumentPackage, WireSemanticFootnote, WireSemanticKind, WireSemanticListItem,
    WireSemanticTableCell, WireSemanticTableRow,
};
use crate::{DocumentPackageDecodePolicy, StagingSemanticDecodeError};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const BOOK_V2_DOCUMENT_PACKAGE_CONTRACT: &str = "typaxis.contract/1.5";

/// Closed book-2 vocabulary. Each kind is retained through nested block slots;
/// no successor kind is coerced into a result, proof, or exercise.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WireBookV2SemanticContainerKind {
    Result,
    Proof,
    Exercise,
    Solution,
    Example,
    Counterexample,
    Remark,
    Note,
    Warning,
    CommonError,
    FormalizationNote,
    Quote,
}
impl WireBookV2SemanticContainerKind {
    pub const ALL: [Self; 12] = [
        Self::Result,
        Self::Proof,
        Self::Exercise,
        Self::Solution,
        Self::Example,
        Self::Counterexample,
        Self::Remark,
        Self::Note,
        Self::Warning,
        Self::CommonError,
        Self::FormalizationNote,
        Self::Quote,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Result => "result",
            Self::Proof => "proof",
            Self::Exercise => "exercise",
            Self::Solution => "solution",
            Self::Example => "example",
            Self::Counterexample => "counterexample",
            Self::Remark => "remark",
            Self::Note => "note",
            Self::Warning => "warning",
            Self::CommonError => "common_error",
            Self::FormalizationNote => "formalization_note",
            Self::Quote => "quote",
        }
    }
}
impl sealed_semantic_kind::Sealed for WireBookV2SemanticContainerKind {}
impl WireSemanticKind for WireBookV2SemanticContainerKind {
    const CONTRACT: &'static str = BOOK_V2_DOCUMENT_PACKAGE_CONTRACT;
    const DEBUG_NAME: &'static str = "DecodedBookV2DocumentPackage";
    const ROOT_SHAPE_ERROR: &'static str = "root members differ from the contract-1.5 scaffold";
    fn contract_error() -> StagingSemanticDecodeError {
        StagingSemanticDecodeError::Contract
    }
}

pub type WireBookV2Block = WireSemanticBlock<WireBookV2SemanticContainerKind>;
pub type WireBookV2ListItem = WireSemanticListItem<WireBookV2SemanticContainerKind>;
pub type WireBookV2TableCell = WireSemanticTableCell<WireBookV2SemanticContainerKind>;
pub type WireBookV2TableRow = WireSemanticTableRow<WireBookV2SemanticContainerKind>;
pub type WireBookV2Footnote = WireSemanticFootnote<WireBookV2SemanticContainerKind>;
pub type WireBookV2Document = WireSemanticDocument<WireBookV2SemanticContainerKind>;
pub type WireBookV2DocumentPackage = WireSemanticDocumentPackage<WireBookV2SemanticContainerKind>;
pub type DecodedBookV2DocumentPackage =
    DecodedVersionedSemanticDocumentPackage<WireBookV2SemanticContainerKind>;

#[derive(Debug)]
pub struct BookV2WireError(StagingSemanticDecodeError);
impl BookV2WireError {
    pub const fn kind(&self) -> &StagingSemanticDecodeError {
        &self.0
    }
    pub fn pointer(&self) -> Option<&str> {
        self.0.pointer()
    }
}
impl fmt::Display for BookV2WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            StagingSemanticDecodeError::Contract => f.write_str("expected typaxis.contract/1.5"),
            StagingSemanticDecodeError::Shape(message) => {
                write!(f, "invalid contract-1.5 shape: {message}")
            }
            StagingSemanticDecodeError::Json(error) => {
                write!(f, "invalid contract-1.5 JSON: {error}")
            }
            StagingSemanticDecodeError::BookNavigationShape { pointer, message }
            | StagingSemanticDecodeError::PrecomposedVectorShape { pointer, message } => {
                write!(f, "invalid contract-1.5 shape at {pointer}: {message}")
            }
            StagingSemanticDecodeError::Limit => {
                f.write_str("contract-1.5 package exceeds a resource limit")
            }
            other => fmt::Display::fmt(other, f),
        }
    }
}
impl std::error::Error for BookV2WireError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BookV2DocumentPackageDecoder;
impl BookV2DocumentPackageDecoder {
    pub const fn new() -> Self {
        Self
    }
    pub fn decode(
        &self,
        input: &[u8],
        policy: &DocumentPackageDecodePolicy<'_>,
    ) -> Result<DecodedBookV2DocumentPackage, BookV2WireError> {
        decode_semantic_document(input, policy).map_err(BookV2WireError)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BookV2DocumentPackageEncoder;
impl BookV2DocumentPackageEncoder {
    pub const fn new() -> Self {
        Self
    }
    pub fn encode(&self, package: &WireBookV2DocumentPackage) -> Result<String, BookV2WireError> {
        encode_semantic_document(package).map_err(BookV2WireError)
    }
}

#[cfg(test)]
#[path = "book_v2_tests.rs"]
mod tests;
