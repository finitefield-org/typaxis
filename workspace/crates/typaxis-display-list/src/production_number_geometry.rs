//! Geometry-only number projection. Callers retain source and font authority.
use super::*;
use typaxis_core::TextSpan;
use typaxis_shaping::{ProductionBodyFont, ShapeSourceSpan, StagingEquationNumberGlyphRun};

#[derive(Debug)]
pub struct ProductionEquationNumberCluster<'a> {
    #[cfg(feature = "book-v2-staging")]
    pub(super) run_index: u32,
    #[cfg(feature = "book-v2-staging")]
    pub(super) cluster_index: u32,
    pub(super) text_span: DisplayTextSpan,
    pub(super) exact_text: &'a str,
    pub(super) logical_bounds: Option<Rect>,
    pub(super) glyphs: Vec<ProductionBodyGlyph>,
}
#[cfg(feature = "book-v2-staging")]
impl<'a> ProductionEquationNumberCluster<'a> {
    pub fn run_index(&self) -> u32 {
        self.run_index
    }
    pub fn cluster_index(&self) -> u32 {
        self.cluster_index
    }
    pub fn text_span(&self) -> DisplayTextSpan {
        self.text_span
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

pub(super) struct NumberSource<'r, 'a> {
    pub owner: NodeId,
    pub text_span: TextSpan,
    pub text: &'a str,
    pub runs: &'r [StagingEquationNumberGlyphRun],
}

pub(super) fn project_number<'a, F: From<ProductionBodyDisplayError>>(
    source: NumberSource<'_, 'a>,
    font: &ProductionBodyFont,
    rect: Rect,
    remaining: &mut u64,
    mut step: impl FnMut(NodeId) -> Result<(), F>,
    mut emit: impl FnMut(ProductionEquationNumberCluster<'a>) -> Result<(), F>,
) -> Result<(), F> {
    let owner = source.owner;
    step(owner)?;
    let leading = rect
        .height()
        .get()
        .checked_sub(font.ascender())
        .and_then(|n| n.checked_add(font.descender()))
        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
    let half =
        Length::from_raw(leading.raw() / 2).ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
    let baseline = plus(plus(rect.y(), half, owner)?, font.ascender(), owner)?;
    let runs = source.runs;
    take(
        remaining,
        runs.len()
            .checked_mul(3)
            .ok_or_else(|| error(owner, E::RecordLimit))?,
        owner,
    )?;
    let mut widths = Vec::new();
    let mut origins = Vec::new();
    let mut order = Vec::new();
    for v in [&mut widths, &mut origins] {
        v.try_reserve_exact(runs.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
    }
    order
        .try_reserve_exact(runs.len())
        .map_err(|_| error(owner, E::AllocationFailure))?;
    let mut max_level = 0;
    let mut min_odd = None;
    for (index, run) in runs.iter().enumerate() {
        step(owner)?;
        let mut width = Length::ZERO;
        for glyph in run.glyphs() {
            step(owner)?;
            if glyph.advance_y != Length::ZERO {
                return Err(error(owner, E::ReceiptMismatch).into());
            }
            width = plus(width, glyph.advance_x, owner)?;
        }
        if width <= Length::ZERO {
            return Err(error(owner, E::ReceiptMismatch).into());
        }
        widths.push(width);
        origins.push(Length::ZERO);
        order.push(index);
        let level = run.bidi_level().get();
        max_level = max_level.max(level);
        if level % 2 == 1 {
            min_odd = Some(min_odd.map_or(level, |m: u8| m.min(level)));
        }
    }
    // UAX #9 L2 affects origins only: extraction remains in source order.
    if let Some(min) = min_odd {
        for level in (min..=max_level).rev() {
            step(owner)?;
            let mut at = 0;
            while at < order.len() {
                step(owner)?;
                if runs[order[at]].bidi_level().get() < level {
                    at += 1;
                    continue;
                }
                let start = at;
                while at < order.len() {
                    step(owner)?;
                    if runs[order[at]].bidi_level().get() < level {
                        break;
                    }
                    at += 1;
                }
                for _ in start..at {
                    step(owner)?;
                }
                order[start..at].reverse();
            }
        }
    }
    let mut width = Length::ZERO;
    for index in order {
        step(owner)?;
        origins[index] = width;
        width = plus(width, widths[index], owner)?;
    }
    if width != rect.width().get() {
        return Err(error(owner, E::ReceiptMismatch).into());
    }
    for (index, run) in runs.iter().enumerate() {
        step(owner)?;
        take(remaining, run.glyphs().len(), owner)?;
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(run.glyphs().len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        let mut pen = plus(rect.x(), origins[index], owner)?;
        for glyph in run.glyphs() {
            step(owner)?;
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
        for (_cluster_index, cluster) in run.clusters().iter().enumerate() {
            step(owner)?;
            let ShapeSourceSpan::Parsed(span) = cluster.source_span else {
                return Err(error(owner, E::ReceiptMismatch).into());
            };
            if span.text_id() != source.text_span.text_id() {
                return Err(error(owner, E::ReceiptMismatch).into());
            }
            let start = span
                .start_byte()
                .get()
                .checked_sub(source.text_span.start_byte().get())
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let end = span
                .end_byte()
                .get()
                .checked_sub(source.text_span.start_byte().get())
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let text = source
                .text
                .get(start as usize..end as usize)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let positioned = positions
                .get(cluster.glyph_start as usize..cluster.glyph_end as usize)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            take(
                remaining,
                positioned
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| error(owner, E::RecordLimit))?,
                owner,
            )?;
            let mut glyphs = Vec::new();
            glyphs
                .try_reserve_exact(positioned.len())
                .map_err(|_| error(owner, E::AllocationFailure))?;
            let mut advance = Length::ZERO;
            for ((glyph, _), original) in positioned
                .iter()
                .zip(&run.glyphs()[cluster.glyph_start as usize..cluster.glyph_end as usize])
            {
                step(owner)?;
                glyphs.push(*glyph);
                advance = plus(advance, original.advance_x, owner)?;
            }
            let logical_bounds = match (positioned.first(), PositiveLength::new(advance)) {
                (Some((_, start)), Some(width)) => {
                    Some(Rect::new(*start, rect.y(), width, rect.height()))
                }
                _ => None,
            };
            emit(ProductionEquationNumberCluster {
                #[cfg(feature = "book-v2-staging")]
                run_index: u32::try_from(index).map_err(|_| error(owner, E::RecordLimit))?,
                #[cfg(feature = "book-v2-staging")]
                cluster_index: u32::try_from(_cluster_index)
                    .map_err(|_| error(owner, E::RecordLimit))?,
                text_span: DisplayTextSpan::new(
                    DisplayTextBufferId::new(span.text_id().get()),
                    span.start_byte(),
                    span.end_byte(),
                )
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                exact_text: text,
                logical_bounds,
                glyphs,
            })?;
        }
    }
    Ok(())
}
