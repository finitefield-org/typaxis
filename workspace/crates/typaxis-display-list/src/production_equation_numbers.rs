//! Producer-authored number glyphs share body font usage and preserve their own Span.
use super::*;

pub(super) fn append_number<'d>(
    selected: &DisplayInput<'_, 'd, '_, '_>,
    parent: NodeId,
    fragment_index: u32,
    admitted: &AdmittedResourceLedger,
    remaining: &mut u64,
    draws: &mut Vec<ProductionBodyDraw<'d>>,
) -> Result<(), ProductionBodyDisplayError> {
    let placement = selected
        .equation_numbers()
        .binary_search_by_key(&fragment_index, |n| n.fragment_index())
        .ok()
        .and_then(|i| selected.equation_numbers().get(i))
        .filter(|n| n.parent_owner() == parent)
        .ok_or_else(|| error(parent, E::PendingEquationNumber))?;
    let shape = selected
        .math_flow_registry()
        .and_then(|r| r.equation_number_shape(parent))
        .filter(|s| {
            s.node_id() == placement.owner() && s.fingerprint() == placement.shape_fingerprint()
        })
        .ok_or_else(|| error(parent, E::ReceiptMismatch))?;
    let owner = shape.node_id();
    let font = typaxis_shaping::production_equation_number_font(shape, admitted)
        .map_err(|_| error(owner, E::ReceiptMismatch))?;
    number_geometry::project_number(
        number_geometry::NumberSource {
            owner,
            text_span: shape.text_span(),
            text: shape.exact_text(),
            runs: shape.runs(),
        },
        &font,
        placement.bounds(),
        remaining,
        |_| Ok(()),
        |cluster| {
            draws
                .try_reserve(1)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            draws.push(ProductionBodyDraw::Text(ProductionBodyTextDraw {
                owner,
                page_index: placement.page_index(),
                fragment_index,
                font_face_id: font.face_id(),
                font_sha256: font.content_hash(),
                face_index: font.face_index(),
                font_size: font.size(),
                text_span: cluster.text_span,
                exact_text: cluster.exact_text,
                generated_provenance: None,
                equation_number: Some(shape),
                logical_bounds: cluster.logical_bounds,
                glyphs: cluster.glyphs,
            }));
            Ok(())
        },
    )
}
