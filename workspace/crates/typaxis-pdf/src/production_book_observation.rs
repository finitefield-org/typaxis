//! Actual final-object observations; these facts alone do not authorize PDF/UA
//! claims or release a VerifiedPdfBytesReceipt.
use super::*;
use crate::{
    BookNavigationPdfFinalWriterObservationV2, BookNavigationPdfInfoObservationV2,
    BookNavigationPdfLanguagePaintObservationV2, BookNavigationPdfLanguagePaintSourceV2,
    BookNavigationPdfOutlineObservationV2, BookXmpObservationV2,
};

pub struct ProductionBookPdfObservation {
    writer: BookNavigationPdfFinalWriterObservationV2,
    child_language_paints: Vec<BookNavigationPdfLanguagePaintObservationV2>,
    selected_fingerprint: [u8; 32],
    record_charge: u64,
    spool_charge: u64,
}
impl ProductionBookPdfObservation {
    pub fn writer(&self) -> &BookNavigationPdfFinalWriterObservationV2 {
        &self.writer
    }
    /// Child fingerprints refer to the equation-number child registry.
    pub fn child_language_paints(&self) -> &[BookNavigationPdfLanguagePaintObservationV2] {
        &self.child_language_paints
    }
    pub fn selected_fingerprint(&self) -> [u8; 32] {
        self.selected_fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
}

pub fn observe_production_footnote_book_pdf(
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
) -> Result<ProductionBookPdfObservation, E> {
    pdf.verify(pdf.source, admitted, limits)?;
    let annotations = pdf.source.structure_objects().annotations();
    book.verify(annotations.navigation(), profile, admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    if book.record_charge() < pdf.record_charge() || book.spool_charge() < pdf.spool_charge() {
        return Err(E::ReceiptMismatch);
    }
    let selected = book.selected();
    let navigation = annotations
        .marked()
        .structure()
        .display()
        .source()
        .line_layout()
        .source_flow()
        .navigation();
    let paint_count = selected
        .language_paints()
        .len()
        .checked_add(selected.vector_paints_requiring_language().count())
        .ok_or(E::RecordLimit)?;
    let record_charge = book
        .record_charge()
        .checked_add(
            (selected.entries().len() as u64)
                .checked_mul(5)
                .ok_or(E::RecordLimit)?,
        )
        .and_then(|n| n.checked_add((paint_count as u64).checked_mul(2)?))
        .and_then(|n| n.checked_add((book.child_language_paints().len() as u64).checked_mul(2)?))
        .and_then(|n| n.checked_add(16))
        .ok_or(E::RecordLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    let mut spool_charge = book.spool_charge();
    let mut charge = |text: &str| -> Result<(), E> {
        spool_charge = spool_charge
            .checked_add(text.len() as u64)
            .ok_or(E::SpoolLimit)?;
        if spool_charge > limits.base().get().max_spool_bytes {
            return Err(E::SpoolLimit);
        }
        Ok(())
    };
    let engine = EngineIdentity::compiled();
    charge(engine.name())?;
    charge(" ")?;
    charge(engine.version())?;
    let metadata = navigation.metadata().metadata();
    for value in [
        &metadata.title,
        &metadata.author,
        &metadata.subject,
        &metadata.created,
        &metadata.modified,
    ]
    .into_iter()
    .flatten()
    {
        charge(value)?;
    }
    for (i, keyword) in metadata.keywords.iter().enumerate() {
        if i > 0 {
            charge("; ")?;
        }
        charge(keyword)?;
    }
    let info = BookNavigationPdfInfoObservationV2::from_final_writer(
        3,
        pdf.object_bytes(3).ok_or(E::ReceiptMismatch)?,
        format!("{} {}", engine.name(), engine.version()),
        metadata.title.clone(),
        metadata.author.clone(),
        metadata.subject.clone(),
        (!metadata.keywords.is_empty()).then(|| metadata.keywords.join("; ")),
        metadata.created.clone(),
        metadata.modified.clone(),
    )
    .map_err(|_| E::Metadata)?;
    let mut outlines = Vec::new();
    outlines
        .try_reserve_exact(selected.entries().len())
        .map_err(|_| E::AllocationFailure)?;
    for entry in selected.entries() {
        charge(entry.label())?;
        charge(entry.destination().anchor_id.as_str())?;
        let parent_role = entry
            .parent_outline_id()
            .map(R::Outline)
            .unwrap_or(R::Outlines);
        outlines.push(
            BookNavigationPdfOutlineObservationV2::from_final_writer(
                entry.outline_id(),
                pdf.object_number(R::Outline(entry.outline_id()))
                    .ok_or(E::ReceiptMismatch)?,
                pdf.object_number(parent_role).ok_or(E::ReceiptMismatch)?,
                entry.label().to_owned(),
                entry.destination().anchor_id.as_str().to_owned(),
                entry.source_node_id().get(),
            )
            .map_err(|_| E::ReceiptMismatch)?,
        );
    }
    let mut paints = Vec::new();
    paints
        .try_reserve_exact(paint_count)
        .map_err(|_| E::AllocationFailure)?;
    for paint in selected.language_paints() {
        charge(paint.language())?;
        paints.push(
            BookNavigationPdfLanguagePaintObservationV2::from_final_writer(
                BookNavigationPdfLanguagePaintSourceV2::LogicalOwnerOccurrence(paint.occurrence()),
                paint.owner_node_id().get(),
                paint.page_index(),
                paint.paint_ordinal(),
                pdf.object_number(R::PageContent(paint.page_index()))
                    .ok_or(E::ReceiptMismatch)?,
                paint.language().to_owned(),
                paint.language_record_fingerprint(),
            )
            .map_err(|_| E::ReceiptMismatch)?,
        );
    }
    for paint in selected.vector_paints_requiring_language() {
        charge(paint.language())?;
        paints.push(
            BookNavigationPdfLanguagePaintObservationV2::from_final_writer(
                BookNavigationPdfLanguagePaintSourceV2::VectorUsage(paint.usage_id()),
                paint.owner_node_id().get(),
                paint.page_index(),
                paint.paint_ordinal(),
                pdf.object_number(R::PageContent(paint.page_index()))
                    .ok_or(E::ReceiptMismatch)?,
                paint.language().to_owned(),
                paint.language_record_fingerprint(),
            )
            .map_err(|_| E::ReceiptMismatch)?,
        );
    }
    paints.sort_unstable_by_key(|p| (p.page_index(), p.paint_ordinal()));
    let mut children = Vec::new();
    children
        .try_reserve_exact(book.child_language_paints().len())
        .map_err(|_| E::AllocationFailure)?;
    for paint in book.child_language_paints() {
        charge(paint.language())?;
        children.push(
            BookNavigationPdfLanguagePaintObservationV2::from_final_writer(
                BookNavigationPdfLanguagePaintSourceV2::LogicalOwnerOccurrence(paint.occurrence()),
                paint.owner_node_id().get(),
                paint.page_index(),
                paint.paint_ordinal(),
                pdf.object_number(R::PageContent(paint.page_index()))
                    .ok_or(E::ReceiptMismatch)?,
                paint.language().to_owned(),
                paint.language_record_fingerprint(),
            )
            .map_err(|_| E::ReceiptMismatch)?,
        );
    }
    // Each language fact must agree with the actual marked-content group that
    // owns its selected draw, including equation-number children.
    let structure = annotations.marked().structure();
    let groups = structure.groups();
    for paint in paints.iter().chain(&children) {
        let draw = paint.paint_ordinal() as usize;
        let index = groups.partition_point(|group| group.draws().end <= draw);
        let group = groups.get(index).ok_or(E::ReceiptMismatch)?;
        let node = structure
            .registry()
            .node(group.node())
            .ok_or(E::ReceiptMismatch)?;
        if !group.draws().contains(&draw)
            || group.page_index() != paint.page_index()
            || node.language() != paint.language()
        {
            return Err(E::ReceiptMismatch);
        }
    }
    charge(navigation.languages().document_language())?;
    let raw_metadata = pdf.object_bytes(4).ok_or(E::ReceiptMismatch)?;
    let at = raw_metadata
        .windows(8)
        .position(|w| w == b"\nstream\n")
        .ok_or(E::Metadata)?
        + 8;
    let xmp = raw_metadata[at..]
        .strip_suffix(b"\nendstream")
        .ok_or(E::Metadata)?;
    let xmp = BookXmpObservationV2::from_final_writer(xmp).map_err(|_| E::Metadata)?;
    let writer = BookNavigationPdfFinalWriterObservationV2::from_final_writer_bounded(
        pdf.content_hash(),
        pdf.bytes().len() as u64,
        pdf.page_count(),
        u32::try_from(pdf.objects().len()).map_err(|_| E::ObjectLimit)?,
        1,
        pdf.object_bytes(1).ok_or(E::ReceiptMismatch)?,
        navigation.languages().document_language().to_owned(),
        4,
        pdf.object_number(R::Outlines),
        selected.destination_registry_sha256(),
        info,
        outlines,
        paints,
        xmp,
        &mut spool_charge,
        limits.base().get().max_spool_bytes,
    )
    .map_err(|error| match error {
        crate::BookNavigationPdfError::SpoolLimit => E::SpoolLimit,
        crate::BookNavigationPdfError::AllocationFailure => E::AllocationFailure,
        _ => E::ReceiptMismatch,
    })?;
    Ok(ProductionBookPdfObservation {
        writer,
        child_language_paints: children,
        selected_fingerprint: selected.fingerprint(),
        record_charge,
        spool_charge,
    })
}
