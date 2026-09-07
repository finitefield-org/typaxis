//! Body-relative frames computed from actual generated-marker advances.
use super::*;
use std::collections::BTreeMap;
use typaxis_core::Rect;
use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionInlineFrame {
    start: Length,
    width: PositiveLength,
}
impl ProductionInlineFrame {
    pub const fn start(&self) -> Length {
        self.start
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionListFrame {
    owner: NodeId,
    marker_start: Length,
    marker_width: PositiveLength,
    marker_gap: PositiveLength,
    content: ProductionInlineFrame,
}
impl ProductionListFrame {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn marker_start(&self) -> Length {
        self.marker_start
    }
    pub const fn marker_width(&self) -> PositiveLength {
        self.marker_width
    }
    pub const fn marker_gap(&self) -> PositiveLength {
        self.marker_gap
    }
    pub const fn content(&self) -> ProductionInlineFrame {
        self.content
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionFootnoteFrame {
    owner: NodeId,
    marker_start: Length,
    marker_width: PositiveLength,
    marker_gap: PositiveLength,
    content: ProductionInlineFrame,
}
impl ProductionFootnoteFrame {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn marker_start(&self) -> Length {
        self.marker_start
    }
    pub const fn marker_width(&self) -> PositiveLength {
        self.marker_width
    }
    pub const fn marker_gap(&self) -> PositiveLength {
        self.marker_gap
    }
    pub const fn content(&self) -> ProductionInlineFrame {
        self.content
    }
}
pub struct ProductionBodyInlineFrames<'p, 'a> {
    prepared: &'p ProductionPreparedInlines<'a>,
    body: Rect,
    footnote_region: Option<Rect>,
    regions: BTreeMap<NodeId, ProductionInlineFrame>,
    paragraphs: Vec<ProductionInlineFrame>,
    lists: Vec<ProductionListFrame>,
    footnotes: Vec<ProductionFootnoteFrame>,
    record_charge: u64,
    fingerprint: [u8; 32],
}
impl<'p, 'a> ProductionBodyInlineFrames<'p, 'a> {
    pub const fn body(&self) -> Rect {
        self.body
    }
    /// Declared maximum region, not selected height or page assignment.
    pub const fn footnote_region(&self) -> Option<Rect> {
        self.footnote_region
    }
    pub fn region(&self, owner: NodeId) -> Option<ProductionInlineFrame> {
        self.regions.get(&owner).copied()
    }
    pub fn paragraphs(&self) -> &[ProductionInlineFrame] {
        &self.paragraphs
    }
    pub fn lists(&self) -> &[ProductionListFrame] {
        &self.lists
    }
    pub fn footnotes(&self) -> &[ProductionFootnoteFrame] {
        &self.footnotes
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        prepared: &ProductionPreparedInlines<'_>,
        body: Rect,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.prepared, prepared) || self.body != body {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}

pub(super) fn prepare_frames<'p, 'a>(
    prepared: &'p ProductionPreparedInlines<'a>,
    body: Rect,
) -> Result<ProductionBodyInlineFrames<'p, 'a>, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let flow = prepared.source_flow();
    let root = NodeId::new(0);
    let footnote_region = declared_footnote_region(prepared, body)?;
    // Region lookup, traversal stack, list-column summaries and temporary widths
    // are all bounded before allocation. The selected-line stage retains this base.
    let record_charge = (flow.events().len() as u64)
        .checked_mul(3)
        .and_then(|n| n.checked_add(flow.paragraphs().len() as u64 * 2))
        .and_then(|n| n.checked_add(flow.lists().len() as u64 * 3))
        .and_then(|n| n.checked_add(u64::from(footnote_region.is_some())))
        .and_then(|n| n.checked_add(prepared.footnote_markers().len() as u64))
        .ok_or_else(|| error(root, E::UnitLimit))?;
    let record_charge = prepared
        .shaped
        .list_markers()
        .iter()
        .try_fold(record_charge, |n, m| {
            n.checked_add(
                1 + m.glyph_run().glyphs.len() as u64 + m.glyph_run().clusters.len() as u64,
            )
        })
        .filter(|n| *n <= prepared.max_fragments)
        .ok_or_else(|| error(root, E::UnitLimit))?;
    let mut widths = Vec::<Option<PositiveLength>>::new();
    widths
        .try_reserve_exact(flow.lists().len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    widths.resize(flow.lists().len(), None);
    for marker in prepared.shaped.list_markers() {
        let slot = widths
            .get_mut(marker.source().list_index() as usize)
            .ok_or_else(|| error(marker.source().owner(), E::ReceiptMismatch))?;
        if slot.map_or(true, |old| old.get() < marker.advance().get()) {
            *slot = Some(marker.advance());
        }
    }
    let mut regions = BTreeMap::new();
    let mut paragraphs = Vec::new();
    let mut lists = Vec::new();
    let mut stack = Vec::new();
    paragraphs
        .try_reserve_exact(flow.paragraphs().len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    lists
        .try_reserve_exact(flow.lists().len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let mut footnotes = Vec::new();
    footnotes
        .try_reserve_exact(prepared.footnote_markers().len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let marker_width = prepared
        .footnote_markers()
        .iter()
        .map(|m| m.advance().get())
        .max()
        .and_then(PositiveLength::new);
    let marker_gap = prepared
        .footnote_markers()
        .iter()
        .map(|m| m.font().size().get())
        .max()
        .and_then(PositiveLength::new);
    let mut current = ProductionInlineFrame {
        start: Length::ZERO,
        width: body.width(),
    };
    let add = |a: Length, b: Length, owner| {
        a.checked_add(b)
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))
    };
    for event in flow.events() {
        match *event {
            Event::Begin { owner, kind } => {
                stack
                    .try_reserve(1)
                    .map_err(|_| error(owner, E::AllocationFailure))?;
                stack.push((owner, current));
                if kind == Region::Footnote {
                    let region =
                        footnote_region.ok_or_else(|| error(owner, E::MissingFootnoteRegion))?;
                    let marker = prepared
                        .footnote_markers()
                        .get(footnotes.len())
                        .filter(|m| m.source().owner() == owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let marker_width =
                        marker_width.ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let marker_gap = marker_gap.ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let marker_start = region
                        .x()
                        .checked_sub(body.x())
                        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                    current = ProductionInlineFrame {
                        start: add(
                            add(marker_start, marker_width.get(), owner)?,
                            marker_gap.get(),
                            owner,
                        )?,
                        width: region
                            .width()
                            .get()
                            .checked_sub(marker_width.get())
                            .and_then(|w| w.checked_sub(marker_gap.get()))
                            .and_then(PositiveLength::new)
                            .ok_or_else(|| error(owner, E::FootnoteFrameExhausted))?,
                    };
                    footnotes.push(ProductionFootnoteFrame {
                        owner: marker.source().owner(),
                        marker_start,
                        marker_width,
                        marker_gap,
                        content: current,
                    });
                }
                regions.insert(owner, current);
                if kind == Region::SemanticContainer {
                    let style = flow
                        .semantic_container_style(owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                        .block_style();
                    let start = add(current.start, style.start_indent().get(), owner)?;
                    let width = current
                        .width
                        .get()
                        .checked_sub(style.start_indent().get())
                        .and_then(|n| n.checked_sub(style.end_indent().get()))
                        .and_then(PositiveLength::new)
                        .ok_or_else(|| error(owner, E::ContainerFrameExhausted))?;
                    current = ProductionInlineFrame { start, width };
                } else if kind == Region::List {
                    let index = lists.len();
                    let source = flow
                        .lists()
                        .get(index)
                        .filter(|s| s.owner() == owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let style = source.style().block_style();
                    let marker_width =
                        widths[index].ok_or_else(|| error(owner, E::InvalidListMarker))?;
                    let marker_gap = source
                        .style()
                        .font_size()
                        .ok_or_else(|| error(owner, E::MissingTextStyle))?;
                    let marker_start = add(current.start, style.start_indent().get(), owner)?;
                    let start = add(
                        add(marker_start, marker_width.get(), owner)?,
                        marker_gap.get(),
                        owner,
                    )?;
                    let width = current
                        .width
                        .get()
                        .checked_sub(style.start_indent().get())
                        .and_then(|n| n.checked_sub(style.end_indent().get()))
                        .and_then(|n| n.checked_sub(marker_width.get()))
                        .and_then(|n| n.checked_sub(marker_gap.get()))
                        .and_then(PositiveLength::new)
                        .ok_or_else(|| error(owner, E::ListFrameExhausted))?;
                    current = ProductionInlineFrame { start, width };
                    lists.push(ProductionListFrame {
                        owner,
                        marker_start,
                        marker_width,
                        marker_gap,
                        content: current,
                    });
                }
            }
            Event::Paragraph { index } => {
                let p = flow
                    .paragraphs()
                    .get(index as usize)
                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                if index as usize != paragraphs.len()
                    || stack.last().map(|s| s.0) != Some(p.owner())
                {
                    return Err(error(p.owner(), E::ReceiptMismatch));
                }
                let style = p.style().block_style();
                paragraphs.push(ProductionInlineFrame {
                    start: add(current.start, style.start_indent().get(), p.owner())?,
                    width: current
                        .width
                        .get()
                        .checked_sub(style.start_indent().get())
                        .and_then(|n| n.checked_sub(style.end_indent().get()))
                        .and_then(PositiveLength::new)
                        .ok_or_else(|| error(p.owner(), E::ListFrameExhausted))?,
                });
            }
            Event::End { owner } => {
                current = stack
                    .pop()
                    .filter(|s| s.0 == owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                    .1;
            }
        }
    }
    if !stack.is_empty()
        || paragraphs.len() != flow.paragraphs().len()
        || lists.len() != flow.lists().len()
        || footnotes.len() != prepared.footnote_markers().len()
    {
        return Err(error(root, E::ReceiptMismatch));
    }
    let capacity = regions
        .len()
        .checked_mul(20)
        .and_then(|n| {
            paragraphs
                .len()
                .checked_mul(16)
                .and_then(|m| n.checked_add(m))
        })
        .and_then(|n| lists.len().checked_mul(44).and_then(|m| n.checked_add(m)))
        .and_then(|n| {
            footnotes
                .len()
                .checked_mul(44)
                .and_then(|m| n.checked_add(m))
        })
        .and_then(|n| n.checked_add(192))
        .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
    let mut digest = Vec::new();
    digest
        .try_reserve_exact(capacity)
        .map_err(|_| error(root, E::AllocationFailure))?;
    digest.extend_from_slice(&sha256(b"typaxis.production-body-frames/3"));
    digest.extend_from_slice(&prepared.fingerprint());
    for n in [body.x(), body.y(), body.width().get(), body.height().get()] {
        digest.extend_from_slice(&n.raw().to_be_bytes());
    }
    digest.push(u8::from(footnote_region.is_some()));
    if let Some(region) = footnote_region {
        for n in [
            region.x(),
            region.y(),
            region.width().get(),
            region.height().get(),
        ] {
            digest.extend_from_slice(&n.raw().to_be_bytes());
        }
    }
    for (owner, frame) in &regions {
        digest.extend_from_slice(&owner.get().to_be_bytes());
        for n in [frame.start, frame.width.get()] {
            digest.extend_from_slice(&n.raw().to_be_bytes());
        }
    }
    for frame in &paragraphs {
        for n in [frame.start, frame.width.get()] {
            digest.extend_from_slice(&n.raw().to_be_bytes());
        }
    }
    for list in &lists {
        digest.extend_from_slice(&list.owner.get().to_be_bytes());
        for n in [
            list.marker_start,
            list.marker_width.get(),
            list.marker_gap.get(),
            list.content.start,
            list.content.width.get(),
        ] {
            digest.extend_from_slice(&n.raw().to_be_bytes());
        }
    }
    for frame in &footnotes {
        digest.extend_from_slice(&frame.owner.get().to_be_bytes());
        for n in [
            frame.marker_start,
            frame.marker_width.get(),
            frame.marker_gap.get(),
            frame.content.start,
            frame.content.width.get(),
        ] {
            digest.extend_from_slice(&n.raw().to_be_bytes());
        }
    }
    Ok(ProductionBodyInlineFrames {
        prepared,
        body,
        footnote_region,
        regions,
        paragraphs,
        lists,
        footnotes,
        record_charge,
        fingerprint: sha256(&digest),
    })
}

pub fn layout_production_body_inline_lines<'p, 'a>(
    prepared: &'p ProductionPreparedInlines<'a>,
    body: Rect,
    max_candidate_steps: u64,
) -> Result<ProductionInlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    let frames = prepare_frames(prepared, body)?;
    let mut widths = Vec::new();
    widths
        .try_reserve_exact(frames.paragraphs.len())
        .map_err(|_| {
            error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::AllocationFailure,
            )
        })?;
    widths.extend(frames.paragraphs.iter().map(|f| f.width));
    let mut lines = selected::layout_with_record_base(
        prepared,
        &widths,
        max_candidate_steps,
        frames.record_charge,
    )?;
    let mut digest = [0u8; 64];
    digest[..32].copy_from_slice(&lines.fingerprint);
    digest[32..].copy_from_slice(&frames.fingerprint);
    lines.fingerprint = sha256(&digest);
    lines.frames = Some(frames);
    Ok(lines)
}

// The common owner currently selects one explicit master. Resolve its actual
// region before shaping definitions; a caller-supplied body is not a substitute.
fn declared_footnote_region(
    prepared: &ProductionPreparedInlines<'_>,
    body: Rect,
) -> Result<Option<Rect>, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let package = prepared.source_flow().package();
    let Some(first) = package.document().footnotes.first() else {
        return Ok(None);
    };
    let owner = first.node_id;
    let wire = package
        .checked_wire()
        .map_err(|_| error(owner, E::ReceiptMismatch))?;
    let masters = wire.page_masters();
    let [master] = masters.masters.as_slice() else {
        return Err(error(owner, E::PendingFootnoteMaster));
    };
    if masters.default_master_id != master.master_id || !masters.selection_rules.is_empty() {
        return Err(error(owner, E::PendingFootnoteMaster));
    }
    let invalid = || error(owner, E::InvalidFootnoteGeometry);
    let page_width = Length::from_raw(master.width)
        .filter(|v| *v > Length::ZERO)
        .ok_or_else(invalid)?;
    let page_height = Length::from_raw(master.height)
        .filter(|v| *v > Length::ZERO)
        .ok_or_else(invalid)?;
    let rectangle = |x, y, width, height| -> Result<Rect, ProductionInlinePreparationError> {
        let x = Length::from_raw(x)
            .filter(|v| *v >= Length::ZERO)
            .ok_or_else(invalid)?;
        let y = Length::from_raw(y)
            .filter(|v| *v >= Length::ZERO)
            .ok_or_else(invalid)?;
        let width = Length::from_raw(width)
            .and_then(PositiveLength::new)
            .ok_or_else(invalid)?;
        let height = Length::from_raw(height)
            .and_then(PositiveLength::new)
            .ok_or_else(invalid)?;
        if x.checked_add(width.get())
            .map_or(true, |right| right > page_width)
            || y.checked_add(height.get())
                .map_or(true, |bottom| bottom > page_height)
        {
            return Err(invalid());
        }
        Ok(Rect::new(x, y, width, height))
    };
    if rectangle(
        master.body.x,
        master.body.y,
        master.body.width,
        master.body.height,
    )? != body
    {
        return Err(error(owner, E::ReceiptMismatch));
    }
    let region = master
        .footnote
        .as_ref()
        .ok_or_else(|| error(owner, E::MissingFootnoteRegion))?;
    Ok(Some(rectangle(
        region.x,
        region.y,
        region.width,
        region.height,
    )?))
}
