//! Generated labels bound to the first real fragment of each source item.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyListMarker {
    owner: NodeId,
    marker_index: u32,
    fragment_index: u32,
    page_index: u32,
    bounds: Rect,
    baseline: Length,
}
impl ProductionBodyListMarker {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn marker_index(&self) -> u32 {
        self.marker_index
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
    pub const fn baseline(&self) -> Length {
        self.baseline
    }
}
pub(super) struct MarkerBinding {
    pub marker_index: u32,
    pub item_index: Option<usize>,
    pub baseline: Length,
}

pub(super) fn region_frame(
    lines: &ProductionInlineLineLayout<'_, '_>,
    body: Rect,
    owner: NodeId,
) -> Result<(Length, PositiveLength), ProductionBodyPaginationError> {
    match lines.frames() {
        None => Ok((body.x(), body.width())),
        Some(frames) => {
            let frame = frames
                .region(owner)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            Ok((add(body.x(), frame.start(), owner)?, frame.width()))
        }
    }
}

/// Recompute alignment against the actual containing item frame. In particular,
/// center/end alignment is not a translation of a body-wide preparation.
pub(super) fn block_frame(
    lines: &ProductionInlineLineLayout<'_, '_>,
    body: Rect,
    block: &typaxis_layout::StagingPreparedVectorBlock,
) -> Result<(Length, PositiveLength, Length), ProductionBodyPaginationError> {
    let owner = block.owner();
    let (left, width) = region_frame(lines, body, owner)?;
    if left == body.x() && width == body.width() {
        return Ok((
            block.inner_frame_left(),
            block.inner_frame_width(),
            block.viewport_left(),
        ));
    }
    let width = width
        .get()
        .checked_sub(block.start_indent().get())
        .and_then(|w| w.checked_sub(block.end_indent().get()))
        .and_then(PositiveLength::new)
        .ok_or_else(|| error(owner, E::WidthMismatch))?;
    let left = add(left, block.start_indent().get(), owner)?;
    let slack = width
        .get()
        .checked_sub(block.viewport_width().get())
        .filter(|s| *s >= Length::ZERO)
        .ok_or_else(|| error(owner, E::WidthMismatch))?;
    let offset = match block.text_align() {
        MachineTextAlign::Start => Length::ZERO,
        MachineTextAlign::End => slack,
        MachineTextAlign::Center => {
            Length::from_raw(slack.raw() / 2).ok_or_else(|| error(owner, E::ArithmeticOverflow))?
        }
    };
    let viewport_left = add(left, offset, owner)?;
    if let Some(number) = block.equation_number() {
        let number_left = add(left, width.get(), owner)?
            .checked_sub(number.width().get())
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        let required = add(
            add(viewport_left, block.viewport_width().get(), owner)?,
            number.minimum_gap().get(),
            owner,
        )?;
        if number_left < required {
            return Err(error(owner, E::WidthMismatch));
        }
    }
    Ok((left, width, viewport_left))
}

pub(super) fn has_paint(item: &Item, lines: &ProductionInlineLineLayout<'_, '_>) -> bool {
    match item.source {
        None => false,
        Some(ProductionBodyFragmentSource::ParagraphLine {
            paragraph_index,
            line_index,
        }) => lines.paragraphs()[paragraph_index as usize].lines()[line_index as usize]
            .items()
            .iter()
            .any(|item| {
                matches!(
                    item,
                    typaxis_layout::ProductionPlacedInline::Text(_)
                        | typaxis_layout::ProductionPlacedInline::Vector(_)
                )
            }),
        Some(_) => true,
    }
}

pub(super) fn prepare_metrics(
    lines: &ProductionInlineLineLayout<'_, '_>,
    blocks: &StagingPrecomposedVectorBlockLayout,
    items: &mut [Item],
    bindings: &mut [MarkerBinding],
) -> Result<(), ProductionBodyPaginationError> {
    let mut previous = None;
    for binding in bindings {
        let marker = lines
            .list_markers()
            .get(binding.marker_index as usize)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let owner = marker.source().owner();
        let index = binding
            .item_index
            .ok_or_else(|| error(owner, E::EmptyListItem))?;
        if previous.is_some_and(|p| index < p) {
            return Err(error(owner, E::ReceiptMismatch));
        }
        previous = Some(index);
        let item = items
            .get_mut(index)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let baseline = match item.source.ok_or_else(|| error(owner, E::EmptyListItem))? {
            ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } => {
                lines.paragraphs()[paragraph_index as usize].lines()[line_index as usize].baseline()
            }
            ProductionBodyFragmentSource::VectorBlock { block_index } => {
                let block = &blocks.blocks()[block_index as usize];
                match block.baseline() {
                    Some(b) => add(block.viewport_top_offset().get(), b.get(), owner)?,
                    None => marker.font().ascender(),
                }
            }
            ProductionBodyFragmentSource::RasterFigure { .. } => marker.font().ascender(),
        };
        let ascent = marker.font().ascender();
        let descent = marker.font().descender();
        if ascent <= descent {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let leading = ascent
            .checked_sub(baseline)
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?
            .max(Length::ZERO);
        let trailing = baseline
            .checked_sub(descent)
            .and_then(|n| n.checked_sub(item.height))
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?
            .max(Length::ZERO);
        item.leading = item.leading.max(leading);
        item.trailing = item.trailing.max(trailing);
        binding.baseline = baseline;
    }
    Ok(())
}

pub(super) fn place_marker(
    lines: &ProductionInlineLineLayout<'_, '_>,
    binding: &MarkerBinding,
    fragment_index: u32,
    page_index: u32,
    top: Length,
    body: Rect,
) -> Result<ProductionBodyListMarker, ProductionBodyPaginationError> {
    let marker = lines
        .list_markers()
        .get(binding.marker_index as usize)
        .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
    let owner = marker.source().owner();
    let column = lines
        .frames()
        .and_then(|f| f.lists().get(marker.source().list_index() as usize))
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    let slack = column
        .marker_width()
        .get()
        .checked_sub(marker.advance().get())
        .filter(|n| *n >= Length::ZERO)
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    let x = add(add(body.x(), column.marker_start(), owner)?, slack, owner)?;
    let baseline = add(top, binding.baseline, owner)?;
    let y = baseline
        .checked_sub(marker.font().ascender())
        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
    let height = marker
        .font()
        .ascender()
        .checked_sub(marker.font().descender())
        .and_then(PositiveLength::new)
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    Ok(ProductionBodyListMarker {
        owner,
        marker_index: binding.marker_index,
        fragment_index,
        page_index,
        bounds: Rect::new(x, y, marker.advance(), height),
        baseline,
    })
}

pub(super) fn encode_marker(marker: &ProductionBodyListMarker, bytes: &mut Vec<u8>) {
    bytes.push(3);
    for n in [
        marker.owner.get(),
        marker.marker_index,
        marker.fragment_index,
        marker.page_index,
    ] {
        bytes.extend_from_slice(&n.to_be_bytes());
    }
    for n in [
        marker.bounds.x(),
        marker.bounds.y(),
        marker.bounds.width().get(),
        marker.bounds.height().get(),
        marker.baseline,
    ] {
        bytes.extend_from_slice(&n.raw().to_be_bytes());
    }
}
