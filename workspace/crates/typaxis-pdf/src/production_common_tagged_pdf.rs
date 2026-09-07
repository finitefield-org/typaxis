//! Complete common-graph PDF emission. Diagnostic assembly remains separate;
//! callers cannot select a conformance flag or supply purported writer facts.
use super::*;
use crate::{BookNavigationPdfObservationV2, VerifiedPdfBytesReceipt};
use typaxis_core::{EffectiveConfigFingerprint, LayoutStateFingerprint, PdfStreamCompression};
use typaxis_syntax::{
    StagingAccessibilityProfileAuthorizationV2, StagingBookNavigationProfileAuthorizationV2,
    ValidatedStagingStructureSemanticsV2,
};

/// Owns the single serializer emission and its source-bound closures. Manifest
/// construction and atomic publication must still consume these actual facts.
#[derive(Debug)]
pub struct ProductionCommonTaggedPdf {
    final_pdf: VerifiedPdfBytesReceipt,
    book_navigation: BookNavigationPdfObservationV2,
    safe_vector: crate::StagingSafeVectorPdfClosureV2,
    vector_final_writer: crate::StagingSafeVectorPdfFinalWriterObservationV2,
    objects: Vec<ProductionBodyAssemblyObject>,
    package_sha256: [u8; 32],
    profile_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    structure_registry_sha256: [u8; 32],
    structure_sha256: [u8; 32],
    display_sha256: [u8; 32],
    page_reference_count: u64,
    record_charge: u64,
    spool_charge: u64,
}
impl ProductionCommonTaggedPdf {
    pub fn final_pdf(&self) -> &VerifiedPdfBytesReceipt {
        &self.final_pdf
    }
    pub fn book_navigation(&self) -> &BookNavigationPdfObservationV2 {
        &self.book_navigation
    }
    pub fn safe_vector(&self) -> &crate::StagingSafeVectorPdfClosureV2 {
        &self.safe_vector
    }
    pub fn vector_final_writer(&self) -> &crate::StagingSafeVectorPdfFinalWriterObservationV2 {
        &self.vector_final_writer
    }
    pub fn objects(&self) -> &[ProductionBodyAssemblyObject] {
        &self.objects
    }
    pub fn object_bytes(&self, number: u32) -> Option<&[u8]> {
        assembled_object_bytes(self.final_pdf.bytes(), &self.objects, number)
    }
    pub fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub fn profile_sha256(&self) -> [u8; 32] {
        self.profile_sha256
    }
    pub fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub fn structure_registry_sha256(&self) -> [u8; 32] {
        self.structure_registry_sha256
    }
    pub fn structure_sha256(&self) -> [u8; 32] {
        self.structure_sha256
    }
    pub fn display_sha256(&self) -> [u8; 32] {
        self.display_sha256
    }
    pub fn page_reference_count(&self) -> u64 {
        self.page_reference_count
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub fn into_final_pdf(self) -> VerifiedPdfBytesReceipt {
        self.final_pdf
    }
}

/// Emit from the common selected graph, recheck profile and structure ownership,
/// and close actual page labels, vector objects and final book observations.
/// The returned bytes are owned (not cloned from a diagnostic PDF). In
/// particular, vector-only math never selects a body font from native-math draws.
#[allow(clippy::too_many_arguments)]
pub fn write_production_common_tagged_pdf(
    source: &crate::ProductionFootnoteResourceObjects<
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
    semantics: &ValidatedStagingStructureSemanticsV2,
    accessibility: &StagingAccessibilityProfileAuthorizationV2,
    book_profile: &StagingBookNavigationProfileAuthorizationV2,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    config: EffectiveConfigFingerprint,
) -> Result<ProductionCommonTaggedPdf, E> {
    source
        .verify(source.structure_objects(), admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    let annotations = source.structure_objects().annotations();
    let structure = annotations.marked().structure();
    let display = structure.display();
    let flow = display.source().line_layout().source_flow();
    let package = flow.package();
    let navigation = flow.navigation();
    if accessibility.book_navigation_profile_fingerprint()
        != book_profile.profile_receipt_fingerprint()
    {
        return Err(E::ReceiptMismatch);
    }
    accessibility
        .authorizes(package, navigation, semantics, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    book_profile
        .authorizes(package, navigation, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    // The registry is immutable and can only be constructed by its validating
    // owner. Bind that receipt to this call; rebuilding a duplicate tree here
    // would retain an additional document-sized allocation. Actual emitted
    // nodes, MCRs, OBJRs and ParentTree entries are compared by pdf.verify below.
    let registry = structure.registry();
    if registry.package_sha256() != package.canonical_jcs_sha256()
        || registry.semantic_sha256() != package.semantic_fingerprint()
        || registry.semantics_sha256() != semantics.fingerprint()
        || registry.authorization_sha256() != accessibility.fingerprint()
        || registry.limits_sha256() != limits.fingerprint()
        || registry.fingerprint() != sha256(registry.canonical_jcs().as_bytes())
    {
        return Err(E::ReceiptMismatch);
    }

    let pdf = assemble_production_footnote_pdf_with_metadata(source, admitted, limits, true)?;
    pdf.verify(source, admitted, limits)?;
    let safe = pdf.seal_safe_vector(admitted, limits, pdf.record_charge(), pdf.spool_charge())?;
    let book = typaxis_display_list::project_production_footnote_book_navigation(
        annotations.navigation(),
        admitted,
        limits,
        safe.record_charge(),
        safe.spool_charge(),
    )
    .map_err(map_book_projection_error)?;
    let book = typaxis_display_list::seal_production_footnote_book_navigation(
        book,
        book_profile,
        admitted,
        limits,
    )
    .map_err(map_book_projection_error)?;
    let observed =
        observe_production_footnote_book_pdf(&pdf, &book, book_profile, admitted, limits)?;
    let page_references = seal_production_page_reference_pdf(
        &pdf,
        &book,
        book_profile,
        admitted,
        limits,
        observed.record_charge(),
    )?;

    // Final book validation owns an outline-owner set, a sorted expected-paint
    // array and two page/content maps. Bound them before that validator runs.
    let paint_count = observed.writer().language_paints().len() as u64;
    let record_charge = page_references
        .record_charge()
        .checked_add(book.selected().entries().len() as u64)
        .and_then(|n| n.checked_add(paint_count.checked_mul(3)?))
        .and_then(|n| n.checked_add(4))
        .ok_or(E::RecordLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    // The legacy book closure retains a small fixed-field canonical record and
    // compares source XMP/Info. Charge its metadata temporaries as well.
    let mut spool_charge = observed
        .spool_charge()
        .checked_add(observed.writer().xmp().byte_length())
        .and_then(|n| n.checked_add(4096))
        .and_then(|n| {
            n.checked_add((navigation.languages().document_language().len() as u64).checked_mul(7)?)
        })
        .ok_or(E::SpoolLimit)?;
    for keyword in &navigation.metadata().metadata().keywords {
        spool_charge = spool_charge
            .checked_add(keyword.len() as u64)
            .and_then(|n| n.checked_add(2))
            .ok_or(E::SpoolLimit)?;
    }
    if spool_charge > limits.base().get().max_spool_bytes {
        return Err(E::SpoolLimit);
    }

    let final_pdf = VerifiedPdfBytesReceipt {
        bytes: pdf.bytes,
        sha256: pdf.hash,
        selected_layout_fingerprint: LayoutStateFingerprint::from_untrusted_bytes(
            book.selected().selected_layout_sha256(),
        ),
        footnote_display_sha256: Some(display.fingerprint()),
        page_count: pdf.page_count,
        object_count: u32::try_from(pdf.observations.len()).map_err(|_| E::ObjectLimit)?,
        stream_compression: PdfStreamCompression::None,
        config_fingerprint: config,
    };
    let book_navigation = crate::observe_staging_book_navigation_pdf_v2(
        navigation,
        book_profile,
        book.selected(),
        limits,
        &EngineIdentity::compiled(),
        observed.writer(),
        &final_pdf,
    )
    .map_err(|error| match error {
        crate::BookNavigationPdfError::SpoolLimit => E::SpoolLimit,
        crate::BookNavigationPdfError::OutputLimit => E::OutputLimit,
        crate::BookNavigationPdfError::ObjectLimit => E::ObjectLimit,
        crate::BookNavigationPdfError::AllocationFailure => E::AllocationFailure,
        _ => E::ReceiptMismatch,
    })?;
    if safe.closure().final_pdf_sha256() != final_pdf.content_hash()
        || page_references.pdf_sha256() != final_pdf.content_hash()
        || book_navigation.final_pdf_sha256() != final_pdf.content_hash()
    {
        return Err(E::ReceiptMismatch);
    }
    Ok(ProductionCommonTaggedPdf {
        final_pdf,
        book_navigation,
        safe_vector: safe.closure,
        vector_final_writer: pdf.vector_final_writer,
        objects: pdf.observations,
        package_sha256: package.canonical_jcs_sha256(),
        profile_sha256: accessibility.profile_receipt_fingerprint(),
        limits_sha256: limits.fingerprint(),
        structure_registry_sha256: structure.registry().fingerprint(),
        structure_sha256: structure.fingerprint(),
        display_sha256: display.fingerprint(),
        page_reference_count: page_references.reference_count(),
        record_charge,
        spool_charge,
    })
}

fn map_book_projection_error(error: typaxis_display_list::BookNavigationSelectedError) -> E {
    use typaxis_display_list::BookNavigationSelectedError as B;
    match error {
        B::SpoolLimit => E::SpoolLimit,
        B::FragmentLimit => E::RecordLimit,
        B::AllocationFailure => E::AllocationFailure,
        _ => E::ReceiptMismatch,
    }
}
