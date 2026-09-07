//! Source-bound final page-label validation for the common PDF writer.
//! This closes page references only; it does not claim PDF/UA, generic layout
//! convergence, or authorize publication of a VerifiedPdfBytesReceipt.
use super::*;

#[derive(Debug, Eq, PartialEq)]
pub struct ProductionPageReferencePdfClosure {
    flow_sha256: [u8; 32],
    selected_sha256: [u8; 32],
    profile_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    pdf_sha256: [u8; 32],
    reference_count: u64,
    record_charge: u64,
}
impl ProductionPageReferencePdfClosure {
    pub fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub fn reference_count(&self) -> u64 {
        self.reference_count
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn verify(
        &self,
        pdf: &ProductionFootnotePdfAssembly<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
        book: &typaxis_display_list::ProductionFootnoteBookNavigation<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
        profile: &typaxis_syntax::StagingBookNavigationProfileAuthorizationV2,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), E> {
        let base = self
            .record_charge
            .checked_sub(1)
            .ok_or(E::ReceiptMismatch)?;
        let observed =
            seal_production_page_reference_pdf(pdf, book, profile, admitted, limits, base)?;
        if &observed != self {
            return Err(E::ReceiptMismatch);
        }
        Ok(())
    }
}

/// Recheck serialized PDF/structure/source identity, then join every generated
/// Page label with its actual selected destination. No candidate value or success
/// flag is accepted from the caller. The receipt adds one retained record and
/// does not allocate a copied label array or a second PDF buffer.
pub fn seal_production_page_reference_pdf(
    pdf: &ProductionFootnotePdfAssembly<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    book: &typaxis_display_list::ProductionFootnoteBookNavigation<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    profile: &typaxis_syntax::StagingBookNavigationProfileAuthorizationV2,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    record_base: u64,
) -> Result<ProductionPageReferencePdfClosure, E> {
    pdf.verify(pdf.source, admitted, limits)?;
    let annotations = pdf.source.structure_objects().annotations();
    book.verify(annotations.navigation(), profile, admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    if record_base < pdf.record_charge() || record_base < book.record_charge() {
        return Err(E::ReceiptMismatch);
    }
    let record_charge = record_base.checked_add(1).ok_or(E::RecordLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    let flow = annotations
        .marked()
        .structure()
        .display()
        .source()
        .line_layout()
        .source_flow();
    let values = flow.page_reference_values().unwrap_or(&[]);
    let mut reference_count = 0u64;
    for actual in book.resolved_page_references() {
        let (owner, page) = actual.map_err(|_| E::ReceiptMismatch)?;
        let index = values
            .binary_search_by_key(&owner, |value| value.0)
            .map_err(|_| E::ReceiptMismatch)?;
        if values[index].1 != page {
            return Err(E::ReceiptMismatch);
        }
        reference_count = reference_count.checked_add(1).ok_or(E::RecordLimit)?;
    }
    if reference_count != values.len() as u64 {
        return Err(E::ReceiptMismatch);
    }
    Ok(ProductionPageReferencePdfClosure {
        flow_sha256: flow.fingerprint(),
        selected_sha256: book.selected().fingerprint(),
        profile_sha256: profile.profile_receipt_fingerprint(),
        limits_sha256: limits.fingerprint(),
        pdf_sha256: pdf.content_hash(),
        reference_count,
        record_charge,
    })
}
