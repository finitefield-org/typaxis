//! Producer-authored number glyphs share body font usage and preserve their own Span.
use super::*;
use typaxis_shaping::ShapeSourceSpan;

pub(super) fn append_number<'d>(
    selected: &'d ProductionBodySelectedLayout<'_, '_, '_>,
    parent: NodeId,
    fragment_index: u32,
    admitted: &AdmittedResourceLedger,
    remaining: &mut u64,
    draws: &mut Vec<ProductionBodyDraw<'d>>,
) -> Result<(), ProductionBodyDisplayError> {
    let placement = selected
        .equation_numbers()
        .binary_search_by_key(&parent, |n| n.parent_owner())
        .ok()
        .and_then(|i| selected.equation_numbers().get(i))
        .filter(|n| n.fragment_index() == fragment_index)
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
    let rect = placement.bounds();
    // Center the actual ascent/descent in the authored line-height. Signed
    // half-leading is intentional when that line-height is smaller than the font.
    let leading = rect
        .height()
        .get()
        .checked_sub(font.ascender())
        .and_then(|n| n.checked_add(font.descender()))
        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
    let half =
        Length::from_raw(leading.raw() / 2).ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
    let baseline = plus(plus(rect.y(), half, owner)?, font.ascender(), owner)?;
    let runs = shape.runs();
    take(
        remaining,
        runs.len()
            .checked_mul(3)
            .ok_or_else(|| error(owner, E::RecordLimit))?,
        owner,
    )?;
    let mut widths = Vec::new();
    let mut order = Vec::new();
    let mut origins = Vec::new();
    for v in [&mut widths, &mut origins] {
        v.try_reserve_exact(runs.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
    }
    order
        .try_reserve_exact(runs.len())
        .map_err(|_| error(owner, E::AllocationFailure))?;
    for (index, run) in runs.iter().enumerate() {
        let mut width = Length::ZERO;
        for glyph in run.glyphs() {
            if glyph.advance_y != Length::ZERO {
                return Err(error(owner, E::ReceiptMismatch));
            }
            width = plus(width, glyph.advance_x, owner)?;
        }
        if width <= Length::ZERO {
            return Err(error(owner, E::ReceiptMismatch));
        }
        widths.push(width);
        origins.push(Length::ZERO);
        order.push(index);
    }
    // UAX #9 L2: positions are visual; draws remain in source/cluster order.
    let max = runs.iter().map(|r| r.bidi_level().get()).max().unwrap_or(0);
    let min_odd = runs
        .iter()
        .map(|r| r.bidi_level().get())
        .filter(|n| n % 2 == 1)
        .min();
    if let Some(min) = min_odd {
        for level in (min..=max).rev() {
            let mut at = 0;
            while at < order.len() {
                if runs[order[at]].bidi_level().get() < level {
                    at += 1;
                    continue;
                }
                let start = at;
                while at < order.len() && runs[order[at]].bidi_level().get() >= level {
                    at += 1;
                }
                order[start..at].reverse();
            }
        }
    }
    let mut width = Length::ZERO;
    for index in order {
        origins[index] = width;
        width = plus(width, widths[index], owner)?;
    }
    if width != rect.width().get() {
        return Err(error(owner, E::ReceiptMismatch));
    }
    for (index, run) in runs.iter().enumerate() {
        take(remaining, run.glyphs().len(), owner)?;
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(run.glyphs().len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        let mut pen = plus(rect.x(), origins[index], owner)?;
        for glyph in run.glyphs() {
            positions.push((
                ProductionBodyGlyph {
                    original_gid: glyph.original_gid,
                    x: plus(pen, glyph.offset_x, owner)?,
                    y: baseline
                        .checked_sub(glyph.offset_y)
                        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
                },
                pen,
            ));
            pen = plus(pen, glyph.advance_x, owner)?;
        }
        for cluster in run.clusters() {
            let ShapeSourceSpan::Parsed(span) = cluster.source_span else {
                return Err(error(owner, E::ReceiptMismatch));
            };
            if span.text_id() != shape.text_span().text_id() {
                return Err(error(owner, E::ReceiptMismatch));
            }
            let start = span
                .start_byte()
                .get()
                .checked_sub(shape.text_span().start_byte().get())
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let end = span
                .end_byte()
                .get()
                .checked_sub(shape.text_span().start_byte().get())
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let text = shape
                .exact_text()
                .get(start as usize..end as usize)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let source = positions
                .get(cluster.glyph_start as usize..cluster.glyph_end as usize)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            take(remaining, source.len() + 1, owner)?;
            let mut glyphs = Vec::new();
            glyphs
                .try_reserve_exact(source.len())
                .map_err(|_| error(owner, E::AllocationFailure))?;
            glyphs.extend(source.iter().map(|(glyph, _)| *glyph));
            let advance = run.glyphs()[cluster.glyph_start as usize..cluster.glyph_end as usize]
                .iter()
                .try_fold(Length::ZERO, |n, g| n.checked_add(g.advance_x))
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            let logical_bounds = match (source.first(), PositiveLength::new(advance)) {
                (Some((_, start)), Some(width)) => {
                    Some(Rect::new(*start, rect.y(), width, rect.height()))
                }
                _ => None,
            };
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
                text_span: DisplayTextSpan::new(
                    DisplayTextBufferId::new(span.text_id().get()),
                    span.start_byte(),
                    span.end_byte(),
                )
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                exact_text: text,
                generated_provenance: None,
                equation_number: Some(shape),
                logical_bounds,
                glyphs,
            }));
        }
    }
    Ok(())
}
