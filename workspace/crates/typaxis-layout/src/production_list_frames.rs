//! Body-relative frames computed from actual generated-marker advances.
use super::*;
use std::collections::BTreeMap;
use typaxis_core::Rect;
use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};

#[path = "production_table_frame_scope.rs"]
mod source_scope;
#[path = "production_table_frames.rs"]
mod table;
pub use table::ProductionTableFrame;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionInlineFrame {
    start: Length,
    width: PositiveLength,
}
impl ProductionInlineFrame {
    #[cfg(feature = "book-v2-staging")]
    pub(super) fn with_width(self, width: PositiveLength) -> Self {
        Self { width, ..self }
    }
    #[cfg(feature = "book-v2-staging")]
    pub(super) fn with_start(self, start: Length) -> Self {
        Self { start, ..self }
    }
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
    tables: Vec<ProductionTableFrame>,
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
    pub fn tables(&self) -> &[ProductionTableFrame] {
        &self.tables
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
    let projection = project_frames(
        InlineFlow::Legacy(prepared.source_flow()),
        prepared.list_markers(),
        prepared.footnote_markers(),
        prepared.max_fragments,
        prepared.fingerprint(),
        body,
        "typaxis.production-body-frames/3",
    )?;
    let FrameProjection {
        body,
        footnote_region,
        regions,
        paragraphs,
        lists,
        tables,
        footnotes,
        record_charge,
        fingerprint,
    } = projection;
    Ok(ProductionBodyInlineFrames {
        prepared,
        body,
        footnote_region,
        regions,
        paragraphs,
        lists,
        tables,
        footnotes,
        record_charge,
        fingerprint,
    })
}

pub(super) struct FrameProjection {
    pub(super) body: Rect,
    pub(super) footnote_region: Option<Rect>,
    pub(super) regions: BTreeMap<NodeId, ProductionInlineFrame>,
    pub(super) paragraphs: Vec<ProductionInlineFrame>,
    pub(super) lists: Vec<ProductionListFrame>,
    pub(super) tables: Vec<ProductionTableFrame>,
    pub(super) footnotes: Vec<ProductionFootnoteFrame>,
    pub(super) record_charge: u64,
    pub(super) fingerprint: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn project_frames(
    flow: InlineFlow<'_>,
    list_markers: &[typaxis_shaping::ProductionListMarkerShape<'_>],
    footnote_markers: &[typaxis_shaping::ProductionFootnoteMarkerShape<'_>],
    max_fragments: u64,
    prepared_fingerprint: [u8; 32],
    body: Rect,
    algorithm: &str,
) -> Result<FrameProjection, ProductionInlinePreparationError> {
    let footnote_region = declared_footnote_region(flow, body)?;
    project_frames_in_regions(
        flow,
        list_markers,
        footnote_markers,
        max_fragments,
        prepared_fingerprint,
        body,
        footnote_region,
        algorithm,
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn project_frames_in_regions(
    flow: InlineFlow<'_>,
    list_markers: &[typaxis_shaping::ProductionListMarkerShape<'_>],
    footnote_markers: &[typaxis_shaping::ProductionFootnoteMarkerShape<'_>],
    max_fragments: u64,
    prepared_fingerprint: [u8; 32],
    body: Rect,
    footnote_region: Option<Rect>,
    algorithm: &str,
) -> Result<FrameProjection, ProductionInlinePreparationError> {
    project_frames_with_root_table_widths(
        flow,
        list_markers,
        footnote_markers,
        max_fragments,
        prepared_fingerprint,
        body,
        footnote_region,
        algorithm,
        &[],
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn project_frames_with_root_table_widths(
    flow: InlineFlow<'_>,
    list_markers: &[typaxis_shaping::ProductionListMarkerShape<'_>],
    footnote_markers: &[typaxis_shaping::ProductionFootnoteMarkerShape<'_>],
    max_fragments: u64,
    prepared_fingerprint: [u8; 32],
    body: Rect,
    footnote_region: Option<Rect>,
    algorithm: &str,
    root_table_widths: &[(NodeId, PositiveLength)],
    table_scope: Option<(NodeId, &FrameProjection)>,
    source_owners: Option<&[NodeId]>,
) -> Result<FrameProjection, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let root = NodeId::new(0);
    // Region lookup, traversal stack, list-column summaries and temporary widths
    // are all bounded before allocation. The selected-line stage retains this base.
    let record_charge = (flow_call!(flow, events()).len() as u64)
        .checked_mul(3)
        .and_then(|n| n.checked_add(flow_call!(flow, paragraphs()).len() as u64 * 2))
        .and_then(|n| n.checked_add(flow_call!(flow, lists()).len() as u64 * 3))
        .and_then(|n| n.checked_add(u64::from(footnote_region.is_some())))
        .and_then(|n| n.checked_add(footnote_markers.len() as u64))
        .ok_or_else(|| error(root, E::UnitLimit))?;
    let record_charge = flow_call!(flow, tables())
        .iter()
        .try_fold(record_charge, |n, table| {
            n.checked_add(1)?
                .checked_add((table.columns().len() as u64).checked_mul(4)?)
        })
        .and_then(|n| n.checked_add(flow_call!(flow, table_record_charge())))
        .ok_or_else(|| error(root, E::UnitLimit))?;
    let record_charge = list_markers
        .iter()
        .try_fold(record_charge, |n, m| {
            n.checked_add(
                1 + m.glyph_run().glyphs.len() as u64 + m.glyph_run().clusters.len() as u64,
            )
        })
        .filter(|n| *n <= max_fragments)
        .ok_or_else(|| error(root, E::UnitLimit))?;
    let record_charge = record_charge
        .checked_add(if source_owners.is_some() {
            (flow_call!(flow, events()).len() as u64)
                .checked_mul(2)
                .ok_or_else(|| error(root, E::UnitLimit))?
        } else {
            0
        })
        .filter(|n| *n <= max_fragments)
        .ok_or_else(|| error(root, E::UnitLimit))?;
    let selected_scope = if let Some(owners) = source_owners {
        Some(source_scope::SourceScope::new(
            flow,
            table_scope
                .ok_or_else(|| error(root, E::ReceiptMismatch))?
                .0,
            owners,
        )?)
    } else {
        None
    };
    let mut widths = Vec::<Option<PositiveLength>>::new();
    widths
        .try_reserve_exact(flow_call!(flow, lists()).len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    widths.resize(flow_call!(flow, lists()).len(), None);
    for marker in list_markers {
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
    #[cfg(feature = "book-v2-staging")]
    let mut description_cursor = 0;
    let mut tables = Vec::new();
    tables
        .try_reserve_exact(flow_call!(flow, tables()).len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let mut current_table: Option<usize> = None;
    let mut root_table_cursor = 0;
    let mut table_scope_seen = false;
    let mut stack = Vec::new();
    paragraphs
        .try_reserve_exact(flow_call!(flow, paragraphs()).len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    lists
        .try_reserve_exact(flow_call!(flow, lists()).len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let mut footnotes = Vec::new();
    footnotes
        .try_reserve_exact(footnote_markers.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let marker_width = footnote_markers
        .iter()
        .map(|m| m.advance().get())
        .max()
        .and_then(PositiveLength::new);
    let marker_gap = footnote_markers
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
    for (event_index, event) in flow_call!(flow, events()).iter().enumerate() {
        match *event {
            Event::Begin { owner, kind } => {
                stack
                    .try_reserve(1)
                    .map_err(|_| error(owner, E::AllocationFailure))?;
                stack.push((owner, current, current_table));
                if kind == Region::Footnote {
                    let region =
                        footnote_region.ok_or_else(|| error(owner, E::MissingFootnoteRegion))?;
                    let marker = footnote_markers
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
                if kind == Region::TableCell {
                    let index = current_table.ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let table: &ProductionTableFrame = tables
                        .get(index)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let source = flow_call!(flow, tables())
                        .get(index)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let preserve = if let Some((target, _)) = table_scope
                        .filter(|(target, _)| *target == table.owner() && selected_scope.is_none())
                    {
                        let cell = source
                            .cells()
                            .binary_search_by_key(&owner, |c| c.owner())
                            .map_err(|_| error(target, E::ReceiptMismatch))?;
                        source
                            .rows()
                            .get(source.cells()[cell].row() as usize)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                            .section()
                            == typaxis_syntax::ProductionTableSection::Body
                    } else {
                        false
                    };
                    current = if preserve {
                        *table_scope
                            .unwrap()
                            .1
                            .regions
                            .get(&owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                    } else {
                        table.cell(source, owner)?
                    };
                }
                // Captions are semantic content outside the repeated header.
                // Restore their original parent before applying each local style.
                if let (Some(index), Some((target, original))) = (current_table, table_scope) {
                    let source = flow_call!(flow, tables())
                        .get(index)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if selected_scope.is_none()
                        && source.owner() == target
                        && source
                            .caption_event_range()
                            .is_some_and(|r| r.contains(&event_index))
                    {
                        current = *original
                            .regions
                            .get(&owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    }
                }
                if selected_scope
                    .as_ref()
                    .is_some_and(|s| s.preserve(event_index))
                {
                    current = *table_scope
                        .unwrap()
                        .1
                        .regions
                        .get(&owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                }
                // Only a root table consumes a candidate. Nested tables inherit
                // their newly resolved cell frame, including spans and indents.
                if kind == Region::Table && current_table.is_none() && !root_table_widths.is_empty()
                {
                    let &(expected, width) = root_table_widths
                        .get(root_table_cursor)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if expected != owner {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    if table_scope.is_some_and(|(target, _)| target == owner) {
                        table_scope_seen = true;
                    }
                    if width.get() > current.width.get() {
                        return Err(error(owner, E::InvalidHorizontalMetrics));
                    }
                    current.width = width;
                    root_table_cursor += 1;
                }
                regions.insert(owner, current);
                #[cfg(feature = "book-v2-staging")]
                if kind == Region::DescriptionList {
                    let InlineFlow::BookV2(source_flow) = flow else {
                        return Err(error(owner, E::ReceiptMismatch));
                    };
                    let source = source_flow
                        .description_lists()
                        .get(description_cursor)
                        .filter(|s| s.owner() == owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let style = source.style().block_style();
                    current = ProductionInlineFrame {
                        start: add(current.start, style.start_indent().get(), owner)?,
                        width: current
                            .width
                            .get()
                            .checked_sub(style.start_indent().get())
                            .and_then(|w| w.checked_sub(style.end_indent().get()))
                            .and_then(PositiveLength::new)
                            .ok_or_else(|| error(owner, E::ListFrameExhausted))?,
                    };
                    description_cursor += 1;
                }

                if kind == Region::Table {
                    let index = tables.len();
                    let source = flow_call!(flow, tables())
                        .get(index)
                        .filter(|t| t.owner() == owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let table = table::prepare(source, index, current)?;
                    current = table.content();
                    tables.push(table);
                    current_table = Some(index);
                } else if kind == Region::SemanticContainer {
                    let style = flow
                        .container_block_style(owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
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
                    let source = flow_call!(flow, lists())
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
                let p = flow_call!(flow, paragraphs())
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
                let saved = stack
                    .pop()
                    .filter(|s| s.0 == owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                current = saved.1;
                current_table = saved.2;
            }
        }
    }
    if !stack.is_empty()
        || root_table_cursor != root_table_widths.len()
        || (table_scope.is_some() && !table_scope_seen)
        || paragraphs.len() != flow_call!(flow, paragraphs()).len()
        || tables.len() != flow_call!(flow, tables()).len()
        || current_table.is_some()
        || lists.len() != flow_call!(flow, lists()).len()
        || footnotes.len() != footnote_markers.len()
    {
        return Err(error(root, E::ReceiptMismatch));
    }
    #[cfg(feature = "book-v2-staging")]
    if description_cursor != flow_call!(flow, description_lists()).len() {
        return Err(error(root, E::ReceiptMismatch));
    }
    let table_bytes = tables
        .iter()
        .try_fold(0usize, |n, t| {
            n.checked_add(24)?
                .checked_add(t.columns().len().checked_mul(8)?)
        })
        .and_then(|n| n.checked_add(if tables.is_empty() { 0 } else { 32 }))
        .ok_or_else(|| error(root, E::UnitLimit))?;
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
        .and_then(|n| n.checked_add(table_bytes))
        .and_then(|n| n.checked_add(if table_scope.is_some() { 4 } else { 0 }))
        .and_then(|n| {
            source_owners.map_or(Some(n), |owners| {
                owners
                    .len()
                    .checked_mul(4)
                    .and_then(|m| m.checked_add(8))
                    .and_then(|m| n.checked_add(m))
            })
        })
        .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
    let mut digest = Vec::new();
    digest
        .try_reserve_exact(capacity)
        .map_err(|_| error(root, E::AllocationFailure))?;
    digest.extend_from_slice(&sha256(algorithm.as_bytes()));
    digest.extend_from_slice(&prepared_fingerprint);
    if let Some((owner, _)) = table_scope {
        digest.extend_from_slice(&owner.get().to_be_bytes());
    }
    if let Some(owners) = source_owners {
        digest.extend_from_slice(&(owners.len() as u64).to_be_bytes());
        for owner in owners {
            digest.extend_from_slice(&owner.get().to_be_bytes());
        }
    }
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
    if !tables.is_empty() {
        digest.extend_from_slice(&sha256(b"typaxis.production-table-frames/1"));
        for table in &tables {
            table.encode(&mut digest);
        }
    }
    Ok(FrameProjection {
        body,
        footnote_region,
        regions,
        paragraphs,
        lists,
        tables,
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
    flow: InlineFlow<'_>,
    body: Rect,
) -> Result<Option<Rect>, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    macro_rules! geometry {
        ($wire:expr) => {{
            let wire = $wire;
            let Some(first) = wire.document().footnotes.first() else {
                return Ok(None);
            };
            let owner = NodeId::new(first.node_id);
            let masters = wire.page_masters();
            let [master] = masters.masters.as_slice() else {
                return Err(error(owner, E::PendingFootnoteMaster));
            };
            if masters.default_master_id != master.master_id || !masters.selection_rules.is_empty()
            {
                return Err(error(owner, E::PendingFootnoteMaster));
            }
            (
                owner,
                master.width,
                master.height,
                (
                    master.body.x,
                    master.body.y,
                    master.body.width,
                    master.body.height,
                ),
                master
                    .footnote
                    .as_ref()
                    .map(|r| (r.x, r.y, r.width, r.height)),
            )
        }};
    }
    let (owner, width, height, declared_body, footnote) = match flow {
        InlineFlow::Legacy(flow) => {
            let Some(first) = flow.package().document().footnotes.first() else {
                return Ok(None);
            };
            let wire = flow
                .package()
                .checked_wire()
                .map_err(|_| error(first.node_id, E::ReceiptMismatch))?;
            geometry!(&wire)
        }
        #[cfg(feature = "book-v2-staging")]
        InlineFlow::BookV2(flow) => geometry!(flow.body().body().wire()),
    };
    let invalid = || error(owner, E::InvalidFootnoteGeometry);
    let page_width = Length::from_raw(width)
        .filter(|v| *v > Length::ZERO)
        .ok_or_else(invalid)?;
    let page_height = Length::from_raw(height)
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
        declared_body.0,
        declared_body.1,
        declared_body.2,
        declared_body.3,
    )? != body
    {
        return Err(error(owner, E::ReceiptMismatch));
    }
    let region = footnote.ok_or_else(|| error(owner, E::MissingFootnoteRegion))?;
    Ok(Some(rectangle(region.0, region.1, region.2, region.3)?))
}
