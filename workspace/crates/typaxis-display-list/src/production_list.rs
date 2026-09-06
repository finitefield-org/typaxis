//! Generated text keeps its source namespace while sharing the font/PDF path.
use super::*;
use typaxis_shaping::ShapeSourceSpan;

pub(super) fn append_marker<'d>(
    selected: &'d ProductionBodySelectedLayout<'_, '_, '_>,
    placement: &typaxis_pagination::ProductionBodyListMarker,
    admitted: &AdmittedResourceLedger,
    remaining: &mut u64,
    draws: &mut Vec<ProductionBodyDraw<'d>>,
) -> Result<(), ProductionBodyDisplayError> {
    let owner = placement.owner();
    let lines = selected.line_layout();
    let marker = lines
        .list_markers()
        .get(placement.marker_index() as usize)
        .filter(|m| m.source().owner() == owner)
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    let fragment = selected
        .fragments()
        .get(placement.fragment_index() as usize)
        .filter(|f| f.page_index() == placement.page_index())
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    if fragment
        .baseline()
        .is_some_and(|b| b != placement.baseline())
    {
        return Err(error(owner, E::ReceiptMismatch));
    }
    let font = marker.font();
    admitted
        .font(font.face_id())
        .filter(|f| f.content_hash() == font.content_hash() && f.face_index() == font.face_index())
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    let parsed_count = u32::try_from(
        lines
            .source_flow()
            .package()
            .checked_wire()
            .map_err(|_| error(owner, E::ReceiptMismatch))?
            .text_buffers()
            .len(),
    )
    .map_err(|_| error(owner, E::RecordLimit))?;
    let buffer = parsed_count
        .checked_add(marker.provenance().text_span().text_id().get())
        .ok_or_else(|| error(owner, E::RecordLimit))?;
    let mut pen = placement.bounds().x();
    let mut pen_y = Length::ZERO;
    let mut glyph_cursor = 0;
    for cluster in &marker.glyph_run().clusters {
        let ShapeSourceSpan::Generated(provenance) = cluster.source_span else {
            return Err(error(owner, E::ReceiptMismatch));
        };
        if provenance.buffer_key() != marker.source().key() || cluster.glyph_start != glyph_cursor {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let range = provenance.text_span().range();
        let text = marker
            .utf8()
            .get(range.start_byte().get() as usize..range.end_byte().get() as usize)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let source = marker
            .glyph_run()
            .glyphs
            .get(cluster.glyph_start as usize..cluster.glyph_end as usize)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        take(remaining, source.len() + 1, owner)?;
        let mut glyphs = Vec::new();
        glyphs
            .try_reserve_exact(source.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        let start = pen;
        for glyph in source {
            glyphs.push(ProductionBodyGlyph {
                original_gid: glyph.original_gid,
                x: plus(pen, glyph.offset_x, owner)?,
                y: placement
                    .baseline()
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
        let logical_bounds = PositiveLength::new(advance).map(|w| {
            Rect::new(
                start,
                placement.bounds().y(),
                w,
                placement.bounds().height(),
            )
        });
        draws
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        draws.push(ProductionBodyDraw::Text(ProductionBodyTextDraw {
            owner,
            page_index: placement.page_index(),
            fragment_index: placement.fragment_index(),
            font_face_id: font.face_id(),
            font_sha256: font.content_hash(),
            face_index: font.face_index(),
            font_size: font.size(),
            text_span: DisplayTextSpan::new(
                DisplayTextBufferId::new(buffer),
                range.start_byte(),
                range.end_byte(),
            )
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
            exact_text: text,
            generated_provenance: Some(provenance),
            logical_bounds,
            glyphs,
        }));
        glyph_cursor = cluster.glyph_end;
    }
    if glyph_cursor as usize != marker.glyph_run().glyphs.len()
        || pen.checked_sub(placement.bounds().x()) != Some(marker.advance().get())
        || pen_y != Length::ZERO
    {
        return Err(error(owner, E::ReceiptMismatch));
    }
    Ok(())
}
