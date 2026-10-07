//! Stable host-source admission joined to the successor body and its exact text
//! maps. No resource bytes, profile or selected layout are admitted here.
use super::*;
use typaxis_machine_input::book_v2::{AdmittedBookV2Input, BookV2InputProvenance};
use typaxis_machine_input::{AdmittedMachineSource, MachineInputStage};

use super::super::host_sources::validate_sources;
pub use super::super::host_sources::{
    SemanticMappingFailure as BookV2MappingFailure, SemanticSourceFailure as BookV2SourceFailure,
};

#[derive(Debug)]
pub struct BookV2SourcePreparationError {
    failure: BookV2SourceFailure,
    provenance: Box<BookV2InputProvenance>,
}
impl BookV2SourcePreparationError {
    pub const fn failure(&self) -> &BookV2SourceFailure {
        &self.failure
    }
    pub fn provenance(&self) -> &BookV2InputProvenance {
        &self.provenance
    }
}
impl std::fmt::Display for BookV2SourcePreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 source preparation {}", self.failure)
    }
}
impl std::error::Error for BookV2SourcePreparationError {}

/// Source-admitted styled body. Pure carrier preparation cannot construct this
/// owner, and it cannot be passed to consumers requiring a legacy package.
///
/// ```compile_fail
/// use typaxis_syntax::{book_v2::SourceAdmittedBookV2Body, ValidatedProductionMachinePackage};
/// fn legacy(value: SourceAdmittedBookV2Body) -> ValidatedProductionMachinePackage { value }
/// ```
#[derive(Debug)]
pub struct SourceAdmittedBookV2Body {
    styled: StyledBookV2Body,
    sources: Vec<AdmittedMachineSource>,
    provenance: BookV2InputProvenance,
}
impl SourceAdmittedBookV2Body {
    pub const fn styled(&self) -> &StyledBookV2Body {
        &self.styled
    }
    pub fn sources(&self) -> &[AdmittedMachineSource] {
        &self.sources
    }
    pub const fn provenance(&self) -> &BookV2InputProvenance {
        &self.provenance
    }
}

pub fn prepare_admitted_book_v2_body(
    input: AdmittedBookV2Input,
    limits: &ValidatedResourceLimits,
) -> Result<SourceAdmittedBookV2Body, BookV2SourcePreparationError> {
    let (decoded, sources, provenance) = input.into_parts();
    let prepare = || -> Result<StyledBookV2Body, BookV2SourceFailure> {
        let progress = provenance.progress();
        if provenance.limits() != limits
            || decoded.limits() != limits
            || progress.stage() != MachineInputStage::SourcesAdmitted
            || progress.decoded_contract()
                != Some(typaxis_document_package::book_v2::BOOK_V2_DOCUMENT_PACKAGE_CONTRACT)
            || progress.canonical_sha256() != Some(decoded.canonical_jcs_sha256())
            || progress
                .package()
                .is_none_or(|p| p.sha256() != decoded.raw_sha256())
            || progress.fingerprint().is_none()
            || sources.len() != decoded.wire().sources().len()
            || sources.len() != progress.sources().len()
            || sources
                .iter()
                .zip(decoded.wire().sources())
                .zip(progress.sources())
                .any(|((source, declared), facts)| {
                    source.facts() != facts
                        || facts.source_id().get() != declared.source_id
                        || facts.uri().as_str() != declared.uri
                        || facts.bytes() != u64::from(declared.utf8_byte_length)
                        || source.text().len() as u64 != facts.bytes()
                        || hex_sha(facts.sha256()) != declared.sha256
                })
        {
            return Err(BookV2SourceFailure::ReceiptMismatch);
        }
        let body = prepare_book_v2_body(decoded, limits).map_err(BookV2SourceFailure::Body)?;
        validate_sources(
            body.wire().text_buffers(),
            body.wire().document(),
            body.wire().advanced_page_masters(),
            &sources,
            limits,
        )?;
        style_book_v2_body(body).map_err(BookV2SourceFailure::Body)
    };
    match prepare() {
        Ok(styled) => Ok(SourceAdmittedBookV2Body {
            styled,
            sources,
            provenance,
        }),
        Err(failure) => Err(BookV2SourcePreparationError {
            failure,
            provenance: Box::new(provenance),
        }),
    }
}
fn hex_sha(hash: [u8; 32]) -> String {
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
#[path = "book_v2_source_tests.rs"]
mod tests;
