//! Source-to-selected-line footnote edges for the successor owner.
use super::*;

/// Actual generated marker positions, before footnote page assignment.
pub struct BookV2FootnoteLines<'s, 'p, 'a> {
    lines: &'s BookV2InlineLineLayout<'p, 'a>,
    projection: footnotes::FootnoteLineProjection<'a>,
    limits_fingerprint: [u8; 32],
}
impl<'s, 'p, 'a> BookV2FootnoteLines<'s, 'p, 'a> {
    pub fn definitions(&self) -> &[ProductionFootnoteDefinitionLines<'a>] {
        &self.projection.definitions
    }
    pub fn references(&self) -> &[ProductionFootnoteLineReference] {
        &self.projection.references
    }
    pub fn record_charge(&self) -> u64 {
        self.projection.record_charge
    }
    pub fn verify(
        &self,
        lines: &BookV2InlineLineLayout<'_, '_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.lines, lines) || self.limits_fingerprint != limits.fingerprint() {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}
pub fn prepare_book_v2_footnote_lines<'s, 'p, 'a>(
    lines: &'s BookV2InlineLineLayout<'p, 'a>,
    limits: &M4EffectiveResourceLimits,
) -> Result<BookV2FootnoteLines<'s, 'p, 'a>, ProductionInlinePreparationError> {
    if lines.prepared().limits_fingerprint != limits.fingerprint() {
        return Err(error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::ReceiptMismatch,
        ));
    }
    let projection = footnotes::project_footnote_lines(
        InlineFlow::BookV2(lines.prepared().source_flow()),
        lines.paragraphs(),
        lines.output_records(),
        limits,
    )?;
    Ok(BookV2FootnoteLines {
        lines,
        projection,
        limits_fingerprint: limits.fingerprint(),
    })
}
