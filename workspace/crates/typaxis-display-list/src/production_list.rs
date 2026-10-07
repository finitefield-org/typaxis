//! Generated text keeps its source namespace while sharing the font/PDF path.
use super::*;

pub(super) fn append_marker<'d>(
    selected: &DisplayInput<'_, 'd, '_, '_>,
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
    append_generated_marker(
        GeneratedPaint {
            owner,
            font: marker.font(),
            run: marker.glyph_run(),
            key: marker.source().key(),
            utf8: marker.utf8(),
            provenance: marker.provenance(),
            advance: marker.advance(),
            bounds: placement.bounds(),
            baseline: placement.baseline(),
            page_index: placement.page_index(),
            fragment_index: placement
                .fragment_index()
                .checked_add(selected.fragment_offset)
                .ok_or_else(|| error(owner, E::RecordLimit))?,
        },
        lines,
        admitted,
        remaining,
        draws,
    )
}

struct GeneratedPaint<'d> {
    owner: NodeId,
    font: &'d typaxis_shaping::ProductionBodyFont,
    run: &'d typaxis_shaping::GlyphRun,
    key: typaxis_core::GeneratedBufferKey,
    utf8: &'d str,
    provenance: typaxis_text::GeneratedProvenance,
    advance: PositiveLength,
    bounds: Rect,
    baseline: Length,
    page_index: u32,
    fragment_index: u32,
}

pub(super) fn append_footnote_marker<'d>(
    selected: &DisplayInput<'_, 'd, '_, '_>,
    placement: &typaxis_pagination::ProductionBodyFootnotePlacedMarker,
    page_index: u32,
    admitted: &AdmittedResourceLedger,
    remaining: &mut u64,
    draws: &mut Vec<ProductionBodyDraw<'d>>,
) -> Result<(), ProductionBodyDisplayError> {
    let lines = selected.line_layout();
    let marker = lines
        .footnote_markers()
        .get(placement.definition_index())
        .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
    let owner = marker.source().owner();
    append_generated_marker(
        GeneratedPaint {
            owner,
            font: marker.font(),
            run: marker.glyph_run(),
            key: marker.provenance().buffer_key(),
            utf8: marker.utf8(),
            provenance: marker.provenance(),
            advance: marker.advance(),
            bounds: placement.bounds(),
            baseline: placement.baseline(),
            page_index,
            fragment_index: placement
                .fragment_index()
                .checked_add(selected.fragment_offset)
                .ok_or_else(|| error(owner, E::RecordLimit))?,
        },
        lines,
        admitted,
        remaining,
        draws,
    )
}

fn append_generated_marker<'d>(
    paint: GeneratedPaint<'d>,
    lines: &typaxis_layout::ProductionInlineLineLayout<'_, '_>,
    admitted: &AdmittedResourceLedger,
    remaining: &mut u64,
    draws: &mut Vec<ProductionBodyDraw<'d>>,
) -> Result<(), ProductionBodyDisplayError> {
    let owner = paint.owner;
    let font = paint.font;
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
    marker_geometry::project_marker(
        marker_geometry::MarkerSource {
            owner,
            run: paint.run,
            key: paint.key,
            utf8: paint.utf8,
            provenance: paint.provenance,
            advance: paint.advance,
        },
        paint.bounds,
        paint.baseline,
        parsed_count,
        remaining,
        |_| Ok(()),
        |cluster| {
            draws
                .try_reserve(1)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            draws.push(ProductionBodyDraw::Text(ProductionBodyTextDraw {
                owner,
                page_index: paint.page_index,
                fragment_index: paint.fragment_index,
                font_face_id: font.face_id(),
                font_sha256: font.content_hash(),
                face_index: font.face_index(),
                font_size: font.size(),
                text_span: cluster.text_span,
                exact_text: cluster.exact_text,
                generated_provenance: Some(cluster.provenance),
                equation_number: None,
                logical_bounds: cluster.logical_bounds,
                glyphs: cluster.glyphs,
            }));
            Ok(())
        },
    )
}
