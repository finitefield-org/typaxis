//! Native glyph/rule projection from common selected line and block origins.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionNativeMathPaint {
    Glyph {
        original_gid: OriginalGlyphId,
        unicode: char,
        logical_ordinal: u32,
        x: Length,
        y: Length,
        font_size: PositiveLength,
    },
    Rule(Rect),
}
#[derive(Debug)]
pub struct ProductionBodyNativeMathDraw<'d> {
    receipt: &'d typaxis_layout::ValidatedMathReceipt,
    source_span: typaxis_core::SourceSpan,
    page_index: u32,
    fragment_index: u32,
    origin_x: Length,
    baseline: Length,
    bounds: Rect,
    paints: Vec<ProductionNativeMathPaint>,
}
impl<'d> ProductionBodyNativeMathDraw<'d> {
    pub const fn owner(&self) -> NodeId {
        self.receipt.node_id()
    }
    pub const fn receipt(&self) -> &'d typaxis_layout::ValidatedMathReceipt {
        self.receipt
    }
    pub const fn source_span(&self) -> typaxis_core::SourceSpan {
        self.source_span
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn origin_x(&self) -> Length {
        self.origin_x
    }
    pub const fn baseline(&self) -> Length {
        self.baseline
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn paints(&self) -> &[ProductionNativeMathPaint] {
        &self.paints
    }
}

pub(super) fn project_native_math<'d>(
    selected: &typaxis_layout::ProductionPlacedInlineMath<'d>,
    page_index: u32,
    fragment_index: u32,
    fragment: Rect,
    admitted: &AdmittedResourceLedger,
    remaining: &mut u64,
) -> Result<ProductionBodyNativeMathDraw<'d>, ProductionBodyDisplayError> {
    let owner = selected.owner();
    project_native_origin(
        selected.receipt(),
        selected.source_span(),
        page_index,
        fragment_index,
        plus(fragment.x(), selected.pen_x(), owner)?,
        plus(fragment.y(), selected.baseline(), owner)?,
        admitted,
        remaining,
    )
}

pub(super) fn project_native_math_block<'d>(
    lines: &'d typaxis_layout::ProductionInlineLineLayout<'_, '_>,
    block_index: u32,
    fragment_index: u32,
    fragment: typaxis_pagination::ProductionBodyFragment,
    admitted: &AdmittedResourceLedger,
    remaining: &mut u64,
) -> Result<ProductionBodyNativeMathDraw<'d>, ProductionBodyDisplayError> {
    let owner = fragment.owner();
    let block = lines
        .native_math_blocks()
        .get(block_index as usize)
        .filter(|b| b.owner() == owner)
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    let receipt = lines
        .native_math_receipt(owner)
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    let viewport = fragment
        .viewport()
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    let baseline = fragment
        .baseline()
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    if receipt.kind() != typaxis_math::MathNodeKind::Display
        || viewport.width() != block.width()
        || viewport.height() != block.height()
        || baseline != plus(viewport.y(), block.baseline(), owner)?
    {
        return Err(error(owner, E::ReceiptMismatch));
    }
    let origin_x = viewport
        .x()
        .checked_sub(block.left())
        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
    project_native_origin(
        receipt,
        block.source_span(),
        fragment.page_index(),
        fragment_index,
        origin_x,
        baseline,
        admitted,
        remaining,
    )
}

#[allow(clippy::too_many_arguments)]
fn project_native_origin<'d>(
    receipt: &'d typaxis_layout::ValidatedMathReceipt,
    source_span: typaxis_core::SourceSpan,
    page_index: u32,
    fragment_index: u32,
    origin_x: Length,
    baseline: Length,
    admitted: &AdmittedResourceLedger,
    remaining: &mut u64,
) -> Result<ProductionBodyNativeMathDraw<'d>, ProductionBodyDisplayError> {
    let owner = receipt.node_id();
    let face = admitted
        .font(receipt.font_face_id())
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    if face.content_hash() != receipt.font_sha256() || face.face_index() != receipt.face_index() {
        return Err(error(owner, E::ReceiptMismatch));
    }
    let (bounds, paints) =
        project_native_paints(owner, receipt.computation(), origin_x, baseline, remaining)?;
    Ok(ProductionBodyNativeMathDraw {
        receipt,
        source_span,
        page_index,
        fragment_index,
        origin_x,
        baseline,
        bounds,
        paints,
    })
}

/// Only physical glyph/rule geometry is shared. Source/font authorization is
/// retained and checked by the caller's original or successor receipt owner.
pub(super) fn project_native_paints(
    owner: NodeId,
    computation: &typaxis_math::MathComputationReceipt,
    origin_x: Length,
    baseline: Length,
    remaining: &mut u64,
) -> Result<(Rect, Vec<ProductionNativeMathPaint>), ProductionBodyDisplayError> {
    let raw = |n| Length::from_raw(n).ok_or_else(|| error(owner, E::ArithmeticOverflow));
    let positive = |n| {
        raw(n)
            .and_then(|v| PositiveLength::new(v).ok_or_else(|| error(owner, E::ArithmeticOverflow)))
    };
    let dimensions = computation.dimensions();
    let (left, top, right, bottom) = dimensions.bbox();
    let bounds = Rect::new(
        plus(origin_x, raw(left)?, owner)?,
        plus(baseline, raw(top)?, owner)?,
        positive(
            right
                .checked_sub(left)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
        )?,
        positive(
            bottom
                .checked_sub(top)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
        )?,
    );
    take(
        remaining,
        computation
            .paints()
            .len()
            .checked_add(1)
            .ok_or_else(|| error(owner, E::RecordLimit))?,
        owner,
    )?;
    let mut paints = Vec::new();
    paints
        .try_reserve_exact(computation.paints().len())
        .map_err(|_| error(owner, E::AllocationFailure))?;
    for paint in computation.paints() {
        paints.push(match paint {
            typaxis_math::MathPaint::Glyph(g) => ProductionNativeMathPaint::Glyph {
                original_gid: g.original_gid(),
                unicode: g.unicode(),
                logical_ordinal: g.logical_ordinal(),
                x: plus(origin_x, raw(g.x())?, owner)?,
                y: plus(baseline, raw(g.y())?, owner)?,
                font_size: positive(g.font_size_raw())?,
            },
            typaxis_math::MathPaint::Rule(r) => ProductionNativeMathPaint::Rule(Rect::new(
                plus(origin_x, raw(r.x())?, owner)?,
                plus(baseline, raw(r.y())?, owner)?,
                positive(r.width())?,
                positive(r.height())?,
            )),
        });
    }
    Ok((bounds, paints))
}
