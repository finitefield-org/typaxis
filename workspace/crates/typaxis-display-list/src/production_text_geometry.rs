//! Shared geometry only. Callers retain their own source and font admission.
use super::*;

pub(super) fn project_cluster(
    cluster: &typaxis_layout::ProductionPlacedTextCluster<'_, '_>,
    font: &typaxis_shaping::ProductionBodyFont,
    fragment: typaxis_pagination::ProductionBodyFragment,
    remaining: &mut u64,
) -> Result<(Vec<ProductionBodyGlyph>, Option<Rect>), ProductionBodyDisplayError> {
    let owner = cluster.run().owner();
    take(
        remaining,
        cluster
            .glyphs()
            .len()
            .checked_add(1)
            .ok_or_else(|| error(owner, E::RecordLimit))?,
        owner,
    )?;
    let mut glyphs = Vec::new();
    let mut advance = Length::ZERO;
    glyphs
        .try_reserve_exact(cluster.glyphs().len())
        .map_err(|_| error(owner, E::AllocationFailure))?;
    for glyph in cluster.glyphs() {
        advance = plus(advance, glyph.glyph().advance_x, owner)?;
        glyphs.push(ProductionBodyGlyph {
            original_gid: glyph.glyph().original_gid,
            x: plus(fragment.bounds().x(), glyph.x(), owner)?,
            y: plus(fragment.bounds().y(), glyph.y(), owner)?,
        });
    }
    let height = font
        .ascender()
        .checked_sub(font.descender())
        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
    if advance.raw() < 0 || height.raw() < 0 {
        return Err(error(owner, E::ReceiptMismatch));
    }
    let logical_bounds = if let (Some(width), Some(height)) =
        (PositiveLength::new(advance), PositiveLength::new(height))
    {
        let baseline = fragment
            .baseline()
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        Some(Rect::new(
            plus(fragment.bounds().x(), cluster.pen_x(), owner)?,
            baseline
                .checked_sub(font.ascender())
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
            width,
            height,
        ))
    } else {
        None
    };
    Ok((glyphs, logical_bounds))
}
