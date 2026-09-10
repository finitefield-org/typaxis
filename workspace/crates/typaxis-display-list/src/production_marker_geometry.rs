//! Geometry-only generated marker projection; source/font authority stays outside.
use super::*;
use typaxis_shaping::{GlyphRun, ShapeSourceSpan};
use typaxis_text::GeneratedProvenance;

#[derive(Debug)]
pub struct ProductionGeneratedMarkerCluster<'a> {
    #[cfg(feature = "book-v2-staging")]
    pub(super) cluster_index: u32,
    pub(super) text_span: DisplayTextSpan,
    pub(super) provenance: GeneratedProvenance,
    pub(super) exact_text: &'a str,
    pub(super) logical_bounds: Option<Rect>,
    pub(super) glyphs: Vec<ProductionBodyGlyph>,
}
#[cfg(feature = "book-v2-staging")]
impl<'a> ProductionGeneratedMarkerCluster<'a> {
    pub fn cluster_index(&self) -> u32 {
        self.cluster_index
    }
    pub fn text_span(&self) -> DisplayTextSpan {
        self.text_span
    }
    pub fn provenance(&self) -> GeneratedProvenance {
        self.provenance
    }
    pub fn exact_text(&self) -> &'a str {
        self.exact_text
    }
    pub fn logical_bounds(&self) -> Option<Rect> {
        self.logical_bounds
    }
    pub fn glyphs(&self) -> &[ProductionBodyGlyph] {
        &self.glyphs
    }
}
pub(super) struct MarkerSource<'r, 'a> {
    pub owner: NodeId,
    pub run: &'r GlyphRun,
    pub key: typaxis_core::GeneratedBufferKey,
    pub utf8: &'a str,
    pub provenance: GeneratedProvenance,
    pub advance: PositiveLength,
}
pub(super) fn project_marker<'a, F: From<ProductionBodyDisplayError>>(
    source: MarkerSource<'_, 'a>,
    bounds: Rect,
    baseline: Length,
    parsed_count: u32,
    remaining: &mut u64,
    mut step: impl FnMut(NodeId) -> Result<(), F>,
    mut emit: impl FnMut(ProductionGeneratedMarkerCluster<'a>) -> Result<(), F>,
) -> Result<(), F> {
    let owner = source.owner;
    step(owner)?;
    if source.provenance.buffer_key() != source.key
        || source.key.owner() != owner
        || source.run.source_span != ShapeSourceSpan::Generated(source.provenance)
    {
        return Err(error(owner, E::ReceiptMismatch).into());
    }
    let full = source.provenance.text_span();
    let range = full.range();
    if range
        .end_byte()
        .get()
        .checked_sub(range.start_byte().get())
        .map(|n| n as usize)
        != Some(source.utf8.len())
    {
        return Err(error(owner, E::ReceiptMismatch).into());
    }
    let buffer = parsed_count
        .checked_add(full.text_id().get())
        .ok_or_else(|| error(owner, E::RecordLimit))?;
    let mut pen = bounds.x();
    let mut pen_y = Length::ZERO;
    let mut glyph_cursor = 0;
    let mut text_cursor = range.start_byte();
    for (_index, cluster) in source.run.clusters.iter().enumerate() {
        step(owner)?;
        let ShapeSourceSpan::Generated(provenance) = cluster.source_span else {
            return Err(error(owner, E::ReceiptMismatch).into());
        };
        let span = provenance.text_span().range();
        if source
            .provenance
            .subspan(span.start_byte(), span.end_byte())
            != Some(provenance)
            || span.start_byte() != text_cursor
            || cluster.glyph_start != glyph_cursor
        {
            return Err(error(owner, E::ReceiptMismatch).into());
        }
        let start = span
            .start_byte()
            .get()
            .checked_sub(range.start_byte().get())
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let end = span
            .end_byte()
            .get()
            .checked_sub(range.start_byte().get())
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let text = source
            .utf8
            .get(start as usize..end as usize)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let original = source
            .run
            .glyphs
            .get(cluster.glyph_start as usize..cluster.glyph_end as usize)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        take(
            remaining,
            original
                .len()
                .checked_add(1)
                .ok_or_else(|| error(owner, E::RecordLimit))?,
            owner,
        )?;
        let mut glyphs = Vec::new();
        glyphs
            .try_reserve_exact(original.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        let start = pen;
        for glyph in original {
            step(owner)?;
            glyphs.push(ProductionBodyGlyph {
                original_gid: glyph.original_gid,
                x: plus(pen, glyph.offset_x, owner)?,
                y: baseline
                    .checked_sub(pen_y)
                    .and_then(|n| n.checked_sub(glyph.offset_y))
                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
            });
            pen = plus(pen, glyph.advance_x, owner)?;
            pen_y = plus(pen_y, glyph.advance_y, owner)?;
        }
        let advance = pen
            .checked_sub(start)
            .filter(|n| *n >= Length::ZERO)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let logical_bounds =
            PositiveLength::new(advance).map(|w| Rect::new(start, bounds.y(), w, bounds.height()));
        emit(ProductionGeneratedMarkerCluster {
            #[cfg(feature = "book-v2-staging")]
            cluster_index: u32::try_from(_index).map_err(|_| error(owner, E::RecordLimit))?,
            text_span: DisplayTextSpan::new(
                DisplayTextBufferId::new(buffer),
                span.start_byte(),
                span.end_byte(),
            )
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
            provenance,
            exact_text: text,
            logical_bounds,
            glyphs,
        })?;
        glyph_cursor = cluster.glyph_end;
        text_cursor = span.end_byte();
    }
    if glyph_cursor as usize != source.run.glyphs.len()
        || text_cursor != range.end_byte()
        || pen.checked_sub(bounds.x()) != Some(source.advance.get())
        || pen_y != Length::ZERO
    {
        return Err(error(owner, E::ReceiptMismatch).into());
    }
    Ok(())
}
