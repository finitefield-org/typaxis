//! Shared forward cursor for measured production body lines and vector blocks.
//! This is an internal page-placement stage, not a replacement for convergence,
//! final-line shaping, table/footnote selection or PDF paint authorization.
use typaxis_core::{sha256, Length, M4EffectiveResourceLimits, NodeId, PositiveLength, Rect};
use typaxis_layout::{ProductionInlineLineLayout, StagingPrecomposedVectorBlockLayout};
use typaxis_style::{ComputedMachineBlockStyle, MachineTextAlign};
use typaxis_syntax::{
    ProductionFlowEvent as Event, ProductionFlowRegionKind as Region, StagingM4PageGeometry,
};

#[path = "production_list.rs"]
mod list;
pub use list::ProductionBodyListMarker;
#[path = "production_breaks.rs"]
mod page_breaks;
pub use page_breaks::{
    ProductionBodyBreakCandidate, ProductionBodyBreakDecision, ProductionBodyBreakReason,
    PRODUCTION_BODY_BREAK_POLICY,
};

#[path = "production_terminals.rs"]
mod terminals;
pub use terminals::{
    finalize_production_body_math_terminals, ProductionBodyEquationNumber,
    ProductionBodyMathTerminals, PRODUCTION_BODY_TERMINAL_ALGORITHM,
};

#[path = "production_page_feedback.rs"]
mod page_feedback;
pub use page_feedback::{
    paginate_stable_production_body, ProductionBodyPagePass, ProductionBodyPageStability,
    ProductionStableBodyPages,
};

#[path = "production_body_flow.rs"]
mod body_flow;
pub use body_flow::{
    prepare_production_footnote_demand_search, ProductionFootnoteDemandSearch,
    ProductionFootnoteDemandSelection, ProductionFootnoteDemandState, ProductionFootnoteDemandStatus,
    ProductionFootnoteRegionFragment, ProductionFootnoteRegionSelection,
    prepare_production_body_flow, prepare_production_footnote_search,
    ProductionFootnoteBreakSearch, ProductionFootnoteCursor, ProductionFootnoteFragmentSelection,
    ProductionFootnoteMarkerBinding, ProductionPreparedBodyFlow,
    ProductionFootnoteFlowReference,
};

pub const PRODUCTION_BODY_PAGINATION_ALGORITHM: &str = "typaxis.production-body-pagination/4";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyPaginationErrorKind {
    ReceiptMismatch,
    PendingRegion(&'static str),
    PendingNamedPage,
    PendingEquationNumber,
    EmptyListItem,
    EmptyFootnote,
    PendingContainerIndent,
    EmptyParagraph,
    WidthMismatch,
    KeepAcrossForcedBreak,
    Oversize,
    PageLimit,
    PagePassLimit,
    InvalidFootnoteCapacity,
    FootnoteSearchLimit,
    PageBreakLookbackLimit { limit: u16, observed: u32 },
    FragmentLimit,
    SpoolLimit,
    MathTerminal(typaxis_layout::StagingMathVectorTerminalError),
    AllocationFailure,
    ArithmeticOverflow,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyPaginationError {
    pub owner: NodeId,
    pub kind: ProductionBodyPaginationErrorKind,
}
impl std::fmt::Display for ProductionBodyPaginationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "production_body_pagination {:?}: node {}",
            self.kind,
            self.owner.get()
        )
    }
}
impl std::error::Error for ProductionBodyPaginationError {}
use ProductionBodyPaginationErrorKind as E;
fn error(owner: NodeId, kind: E) -> ProductionBodyPaginationError {
    ProductionBodyPaginationError { owner, kind }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyFragmentSource {
    ParagraphLine {
        paragraph_index: u32,
        line_index: u32,
    },
    VectorBlock {
        block_index: u32,
    },
    RasterFigure {
        figure_index: u32,
    },
}
/// Source order is the index in fragments(). Body glyphs and inline SVGs both
/// add bounds.x/y to their original line-local positions; no separate cursor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyFragment {
    owner: NodeId,
    page_index: u32,
    source: ProductionBodyFragmentSource,
    bounds: Rect,
    baseline: Option<Length>,
    viewport: Option<Rect>,
    effective_space_before: Length,
}
impl ProductionBodyFragment {
    pub const fn owner(self) -> NodeId {
        self.owner
    }
    pub const fn page_index(self) -> u32 {
        self.page_index
    }
    pub const fn source(self) -> ProductionBodyFragmentSource {
        self.source
    }
    pub const fn bounds(self) -> Rect {
        self.bounds
    }
    pub const fn baseline(self) -> Option<Length> {
        self.baseline
    }
    pub const fn viewport(self) -> Option<Rect> {
        self.viewport
    }
    pub const fn effective_space_before(self) -> Length {
        self.effective_space_before
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyPage {
    first_fragment: u32,
    fragment_count: u32,
    used_height: Length,
}
impl ProductionBodyPage {
    pub const fn first_fragment(self) -> u32 {
        self.first_fragment
    }
    pub const fn fragment_count(self) -> u32 {
        self.fragment_count
    }
    pub const fn used_height(self) -> Length {
        self.used_height
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyPageBreak {
    owner: NodeId,
    produced_page_index: u32,
}
impl ProductionBodyPageBreak {
    pub const fn owner(self) -> NodeId {
        self.owner
    }
    pub const fn produced_page_index(self) -> u32 {
        self.produced_page_index
    }
}
pub struct ProductionBodySelectedLayout<'s, 'p, 'a> {
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    blocks: &'s StagingPrecomposedVectorBlockLayout,
    limits_fingerprint: [u8; 32],
    pages: Vec<ProductionBodyPage>,
    fragments: Vec<ProductionBodyFragment>,
    breaks: Vec<ProductionBodyPageBreak>,
    list_markers: Vec<ProductionBodyListMarker>,
    decisions: Vec<ProductionBodyBreakDecision>,
    math_terminals: Option<ProductionBodyMathTerminals>,
    math_registry: Option<&'s typaxis_layout::StagingMathVectorFlowRegistry>,
    equation_numbers: Vec<ProductionBodyEquationNumber>,
    record_charge: u64,
    fingerprint: [u8; 32],
}
impl<'s, 'p, 'a> ProductionBodySelectedLayout<'s, 'p, 'a> {
    pub fn equation_numbers(&self) -> &[ProductionBodyEquationNumber] {
        &self.equation_numbers
    }
    pub fn math_flow_registry(&self) -> Option<&'s typaxis_layout::StagingMathVectorFlowRegistry> {
        self.math_registry
    }
    pub fn math_terminals(&self) -> Option<&ProductionBodyMathTerminals> {
        self.math_terminals.as_ref()
    }
    pub fn spool_charge(&self) -> u64 {
        self.math_terminals.as_ref().map_or(0, |t| t.spool_charge())
    }

    pub fn page_break_decisions(&self) -> &[ProductionBodyBreakDecision] {
        &self.decisions
    }
    pub fn list_markers(&self) -> &[ProductionBodyListMarker] {
        &self.list_markers
    }
    pub fn pages(&self) -> &[ProductionBodyPage] {
        &self.pages
    }
    pub fn fragments(&self) -> &[ProductionBodyFragment] {
        &self.fragments
    }
    pub fn page_breaks(&self) -> &[ProductionBodyPageBreak] {
        &self.breaks
    }
    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        self.blocks.page_geometry()
    }
    pub const fn line_layout(&self) -> &'s ProductionInlineLineLayout<'p, 'a> {
        self.lines
    }
    pub const fn block_layout(&self) -> &'s StagingPrecomposedVectorBlockLayout {
        self.blocks
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn verify(
        &self,
        lines: &ProductionInlineLineLayout<'_, '_>,
        blocks: &StagingPrecomposedVectorBlockLayout,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyPaginationError> {
        if !std::ptr::eq(self.lines, lines)
            || !std::ptr::eq(self.blocks, blocks)
            || self.limits_fingerprint != limits.fingerprint()
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}

pub struct ProductionBodyFlowItem {
    owner: NodeId,
    source: Option<ProductionBodyFragmentSource>,
    x: Length,
    width: PositiveLength,
    height: Length,
    before: Length,
    after: Length,
    keep: bool,
    leading: Length,
    trailing: Length,
    viewport_left: Option<Length>,
}
type Item = ProductionBodyFlowItem;
impl Item {
    fn consumed(&self) -> Result<Length, ProductionBodyPaginationError> {
        add(
            add(self.leading, self.height, self.owner)?,
            self.trailing,
            self.owner,
        )
    }
}
struct Frame {
    owner: NodeId,
    start: usize,
    container: Option<ComputedMachineBlockStyle>,
    vector: Option<usize>,
    raster: Option<usize>,
    marker: Option<usize>,
}
struct Charge {
    remaining: u64,
}
impl Charge {
    fn take(&mut self, n: usize, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.remaining = self
            .remaining
            .checked_sub(n as u64)
            .ok_or_else(|| error(owner, E::FragmentLimit))?;
        Ok(())
    }
}
fn add(a: Length, b: Length, owner: NodeId) -> Result<Length, ProductionBodyPaginationError> {
    a.checked_add(b)
        .ok_or_else(|| error(owner, E::ArithmeticOverflow))
}

/// Preserve every source paragraph, block SVG and explicit page break in one
/// cursor. Unsupported subflows are reported at their owning node before any
/// selected layout is returned; tables/footnotes are never body-flattened.
pub fn paginate_production_body<'s, 'p, 'a>(
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    blocks: &'s StagingPrecomposedVectorBlockLayout,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodySelectedLayout<'s, 'p, 'a>, ProductionBodyPaginationError> {
    paginate_production_body_with_prior_charge(lines, blocks, limits, 0)
}

fn paginate_production_body_with_prior_charge<'s, 'p, 'a>(
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    blocks: &'s StagingPrecomposedVectorBlockLayout,
    limits: &M4EffectiveResourceLimits,
    prior_charge: u64,
) -> Result<ProductionBodySelectedLayout<'s, 'p, 'a>, ProductionBodyPaginationError> {
    verify_flow_inputs(lines, blocks, limits)?;
    let root = NodeId::new(0);
    let mut charge = Charge {
        remaining: limits
            .base()
            .get()
            .max_fragments
            .checked_sub(prior_charge)
            .ok_or_else(|| error(root, E::FragmentLimit))?
            .checked_sub(
                lines
                    .output_records()
                    .checked_add(blocks.blocks().len() as u64)
                    .ok_or_else(|| error(root, E::FragmentLimit))?,
            )
            .ok_or_else(|| error(root, E::FragmentLimit))?,
    };
    let body = blocks.page_geometry().body();
    let CollectedItems {
        mut items,
        mut marker_bindings,
        ..
    } = collect_items(lines, blocks, None, &mut charge)?;
    list::prepare_metrics(lines, blocks, &mut items, &mut marker_bindings)?;
    // Suffix keep extents are measured from real selected lines/caption lines.
    // Validate hard keep chains before examining soft page-end candidates.
    charge.take(items.len(), root)?;
    let mut group_heights = Vec::new();
    group_heights
        .try_reserve_exact(items.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    group_heights.resize(items.len(), Length::ZERO);
    for index in (0..items.len()).rev() {
        let item = &items[index];
        let mut height = item.consumed()?;
        if item.keep {
            if let Some(next) = items.get(index + 1) {
                if next.source.is_none() {
                    return Err(error(item.owner, E::KeepAcrossForcedBreak));
                }
                height = add(
                    add(
                        add(height, item.after, item.owner)?,
                        next.before,
                        item.owner,
                    )?,
                    group_heights[index + 1],
                    item.owner,
                )?;
            }
        }
        group_heights[index] = height;
    }
    for (item, height) in items.iter().zip(&group_heights) {
        if *height > body.height().get() {
            return Err(error(item.owner, E::Oversize));
        }
    }
    let decisions = page_breaks::plan(&items, lines, body, limits, &mut charge)?;
    let mut decision_cursor = 0;
    let mut pages = Vec::new();
    let mut fragments = Vec::new();
    let mut breaks = Vec::new();
    let mut list_markers = Vec::new();
    let mut marker_cursor = 0;
    new_page(&mut pages, 0, limits, &mut charge, root)?;
    let mut after = Length::ZERO;
    for (index, item) in items.iter().enumerate() {
        let Some(source) = item.source else {
            new_page(&mut pages, fragments.len(), limits, &mut charge, item.owner)?;
            charge.take(1, item.owner)?;
            breaks
                .try_reserve(1)
                .map_err(|_| error(item.owner, E::AllocationFailure))?;
            breaks.push(ProductionBodyPageBreak {
                owner: item.owner,
                produced_page_index: (pages.len() - 1) as u32,
            });
            after = Length::ZERO;
            continue;
        };
        let decision = decisions
            .get(decision_cursor)
            .ok_or_else(|| error(item.owner, E::ReceiptMismatch))?;
        if decision.start_item() as usize == index && decision.page_index() as usize == pages.len()
        {
            new_page(&mut pages, fragments.len(), limits, &mut charge, item.owner)?;
            after = Length::ZERO;
        }
        if decision.page_index() as usize != pages.len() - 1
            || index < decision.start_item() as usize
            || index >= decision.selected().end_item() as usize
        {
            return Err(error(item.owner, E::ReceiptMismatch));
        }
        let before = if pages.last().unwrap().fragment_count == 0 {
            Length::ZERO
        } else {
            add(after, item.before, item.owner)?
        };
        let page_index = (pages.len() - 1) as u32;
        let page = pages.last_mut().unwrap();
        let top = add(
            add(
                body.y(),
                add(page.used_height, before, item.owner)?,
                item.owner,
            )?,
            item.leading,
            item.owner,
        )?;
        let height = PositiveLength::new(item.height)
            .ok_or_else(|| error(item.owner, E::ReceiptMismatch))?;
        let (baseline, viewport) = match source {
            ProductionBodyFragmentSource::RasterFigure { .. } => {
                (None, Some(Rect::new(item.x, top, item.width, height)))
            }
            ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } => {
                let local =
                    &lines.paragraphs()[paragraph_index as usize].lines()[line_index as usize];
                (Some(add(top, local.baseline(), item.owner)?), None)
            }
            ProductionBodyFragmentSource::VectorBlock { block_index } => {
                let block = &blocks.blocks()[block_index as usize];
                let y = add(top, block.viewport_top_offset().get(), item.owner)?;
                let baseline = block
                    .baseline()
                    .map(|b| add(y, b.get(), item.owner))
                    .transpose()?;
                (
                    baseline,
                    Some(Rect::new(
                        item.viewport_left
                            .ok_or_else(|| error(item.owner, E::ReceiptMismatch))?,
                        y,
                        block.viewport_width(),
                        block.viewport_height(),
                    )),
                )
            }
        };
        if fragments.len() >= u32::MAX as usize {
            return Err(error(item.owner, E::FragmentLimit));
        }
        let next_fragment_count = page
            .fragment_count
            .checked_add(1)
            .ok_or_else(|| error(item.owner, E::FragmentLimit))?;
        charge.take(1, item.owner)?;
        fragments
            .try_reserve(1)
            .map_err(|_| error(item.owner, E::AllocationFailure))?;
        while let Some(binding) = marker_bindings.get(marker_cursor) {
            if binding.item_index != Some(index) {
                break;
            }
            charge.take(1, item.owner)?;
            list_markers
                .try_reserve(1)
                .map_err(|_| error(item.owner, E::AllocationFailure))?;
            list_markers.push(list::place_marker(
                lines,
                binding,
                fragments.len() as u32,
                page_index,
                top,
                body,
            )?);
            marker_cursor += 1;
        }
        fragments.push(ProductionBodyFragment {
            owner: item.owner,
            page_index,
            source,
            bounds: Rect::new(item.x, top, item.width, height),
            baseline,
            viewport,
            effective_space_before: before,
        });
        page.fragment_count = next_fragment_count;
        page.used_height = add(
            add(page.used_height, before, item.owner)?,
            item.consumed()?,
            item.owner,
        )?;
        if page.used_height > body.height().get() {
            return Err(error(item.owner, E::ReceiptMismatch));
        }
        if index + 1 == decision.selected().end_item() as usize {
            if page.used_height != decision.selected().used_height() {
                return Err(error(item.owner, E::ReceiptMismatch));
            }
            decision_cursor += 1;
        }
        after = item.after;
    }
    if marker_cursor != marker_bindings.len() || decision_cursor != decisions.len() {
        return Err(error(root, E::ReceiptMismatch));
    }
    let mut result = ProductionBodySelectedLayout {
        lines,
        blocks,
        limits_fingerprint: limits.fingerprint(),
        pages,
        fragments,
        breaks,
        list_markers,
        decisions,
        math_terminals: None,
        math_registry: None,
        equation_numbers: Vec::new(),
        record_charge: limits.base().get().max_fragments - charge.remaining,
        fingerprint: [0; 32],
    };
    result.fingerprint = fingerprint(&result)?;
    Ok(result)
}

fn new_page(
    pages: &mut Vec<ProductionBodyPage>,
    first: usize,
    limits: &M4EffectiveResourceLimits,
    charge: &mut Charge,
    owner: NodeId,
) -> Result<(), ProductionBodyPaginationError> {
    if pages.len() as u64 >= u64::from(limits.base().get().max_pages) {
        return Err(error(owner, E::PageLimit));
    }
    let first_fragment = u32::try_from(first).map_err(|_| error(owner, E::FragmentLimit))?;
    charge.take(1, owner)?;
    pages
        .try_reserve(1)
        .map_err(|_| error(owner, E::AllocationFailure))?;
    pages.push(ProductionBodyPage {
        first_fragment,
        fragment_count: 0,
        used_height: Length::ZERO,
    });
    Ok(())
}

fn verify_flow_inputs(
    lines: &ProductionInlineLineLayout<'_, '_>,
    blocks: &StagingPrecomposedVectorBlockLayout,
    limits: &M4EffectiveResourceLimits,
) -> Result<(), ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    let epoch = lines.binding_epoch();
    let receipt = blocks.receipt();
    if !blocks.integrity_matches()
        || receipt.package_sha256() != lines.source_flow().package_sha256()
        || receipt.binding_set_fingerprint() != lines.binding_set_fingerprint()
        || receipt.layout_epoch_fingerprint() != epoch.fingerprint()
        || receipt.profile_fingerprint() != epoch.profile_authorization_fingerprint()
        || receipt.admitted_fingerprint() != epoch.admitted_fingerprint()
        || receipt.limits_fingerprint() != limits.fingerprint()
        || epoch.limits_fingerprint() != limits.fingerprint()
    {
        return Err(error(root, E::ReceiptMismatch));
    }
    if lines
        .frames()
        .is_some_and(|f| f.body() != blocks.page_geometry().body())
    {
        return Err(error(root, E::ReceiptMismatch));
    }
    Ok(())
}

struct CollectedItems {
    items: Vec<Item>,
    marker_bindings: Vec<list::MarkerBinding>,
    body_end: usize,
    definitions: Vec<std::ops::Range<usize>>,
}

fn collect_items(
    lines: &ProductionInlineLineLayout<'_, '_>,
    blocks: &StagingPrecomposedVectorBlockLayout,
    definitions: Option<&[typaxis_layout::ProductionFootnoteDefinitionLines<'_>]>,
    charge: &mut Charge,
) -> Result<CollectedItems, ProductionBodyPaginationError> {
    let mut definition_ranges = Vec::new();
    let mut definition_start = None;
    let mut body_end = None;
    if let Some(definitions) = definitions {
        charge.take(definitions.len(), NodeId::new(0))?;
        definition_ranges
            .try_reserve_exact(definitions.len())
            .map_err(|_| error(NodeId::new(0), E::AllocationFailure))?;
    }
    let flow = lines.source_flow();
    let body = blocks.page_geometry().body();
    let mut items: Vec<Item> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut paragraph_cursor = 0;
    let mut block_cursor = 0;
    let mut figure_cursor = 0;
    let mut list_cursor = 0;
    let mut marker_bindings = Vec::new();
    let push = |items: &mut Vec<Item>,
                charge: &mut Charge,
                item: Item|
     -> Result<(), ProductionBodyPaginationError> {
        charge.take(1, item.owner)?;
        items
            .try_reserve(1)
            .map_err(|_| error(item.owner, E::AllocationFailure))?;
        items.push(item);
        Ok(())
    };
    for (event_index, event) in flow.events().iter().enumerate() {
        match *event {
            Event::Begin { owner, kind } => {
                let mut frame = Frame {
                    owner,
                    start: items.len(),
                    container: None,
                    vector: None,
                    raster: None,
                    marker: None,
                };
                match kind {
                    Region::Footnote => {
                        let definition = definitions
                            .ok_or_else(|| error(owner, E::PendingRegion("footnote")))?
                            .get(definition_ranges.len())
                            .filter(|d| d.owner() == owner && d.event_range().start == event_index)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        if !stack.is_empty()
                            || definition_start.is_some()
                            || definition.event_range().end <= event_index
                        {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                        body_end.get_or_insert(items.len());
                        definition_start = Some(items.len());
                    }
                    Region::Paragraph | Region::Heading => (),
                    Region::List => {
                        let frames = lines
                            .frames()
                            .ok_or_else(|| error(owner, E::PendingRegion("list")))?;
                        let source = flow
                            .lists()
                            .get(list_cursor)
                            .filter(|l| l.owner() == owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        if frames.lists().get(list_cursor).map(|l| l.owner()) != Some(owner) {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                        if source.page_name().is_some() {
                            return Err(error(owner, E::PendingNamedPage));
                        }
                        frame.container = Some(source.style().block_style());
                        list_cursor += 1;
                    }
                    Region::ListItem => {
                        let index = marker_bindings.len();
                        let source = flow
                            .list_items()
                            .get(index)
                            .filter(|i| i.owner() == owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        charge.take(1, owner)?;
                        marker_bindings
                            .try_reserve(1)
                            .map_err(|_| error(owner, E::AllocationFailure))?;
                        marker_bindings.push(list::MarkerBinding {
                            marker_index: index as u32,
                            item_index: None,
                            baseline: Length::ZERO,
                        });
                        frame.marker = Some(index);
                        if flow.lists().get(source.list_index() as usize).is_none() {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                    }
                    Region::Figure => {
                        let figure = lines
                            .figures()
                            .get(figure_cursor)
                            .filter(|f| f.owner() == owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        if figure.source().page_name().is_some() {
                            return Err(error(owner, E::PendingNamedPage));
                        }
                        let style = figure.source().style().block_style();
                        let (region_left, region_width) = list::region_frame(lines, body, owner)?;
                        let available = region_width
                            .get()
                            .checked_sub(style.start_indent().get())
                            .and_then(|w| w.checked_sub(style.end_indent().get()))
                            .and_then(PositiveLength::new)
                            .ok_or_else(|| error(owner, E::WidthMismatch))?;
                        if figure.width().get() > available.get() {
                            return Err(error(owner, E::WidthMismatch));
                        }
                        frame.raster = Some(figure_cursor);
                        push(
                            &mut items,
                            charge,
                            Item {
                                leading: Length::ZERO,
                                trailing: Length::ZERO,
                                viewport_left: None,
                                owner,
                                source: Some(ProductionBodyFragmentSource::RasterFigure {
                                    figure_index: figure.source_index(),
                                }),
                                x: add(region_left, style.start_indent().get(), owner)?,
                                width: figure.width(),
                                height: figure.height().get(),
                                before: style.space_before().get(),
                                after: Length::ZERO,
                                keep: false,
                            },
                        )?;
                        figure_cursor += 1;
                    }
                    Region::SemanticContainer => {
                        let style = flow
                            .semantic_container_style(owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        if style.page_name().is_some() {
                            return Err(error(owner, E::PendingNamedPage));
                        }
                        if (style.block_style().start_indent().get() != Length::ZERO
                            || style.block_style().end_indent().get() != Length::ZERO)
                            && lines.frames().is_none()
                        {
                            return Err(error(owner, E::PendingContainerIndent));
                        }
                        frame.container = Some(style.block_style());
                    }
                    Region::VectorFigure | Region::MathVectorBlock => {
                        let block = blocks
                            .blocks()
                            .get(block_cursor)
                            .filter(|b| b.owner() == owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        if block.page_name().is_some() {
                            return Err(error(owner, E::PendingNamedPage));
                        }
                        let (block_left, block_width, viewport_left) =
                            list::block_frame(lines, body, block)?;
                        frame.vector = Some(block_cursor);
                        push(
                            &mut items,
                            charge,
                            Item {
                                leading: Length::ZERO,
                                trailing: Length::ZERO,
                                viewport_left: Some(viewport_left),
                                owner,
                                source: Some(ProductionBodyFragmentSource::VectorBlock {
                                    block_index: block_cursor as u32,
                                }),
                                x: block_left,
                                width: block_width,
                                height: block.content_height().get(),
                                before: block.space_before().get(),
                                after: Length::ZERO,
                                keep: false,
                            },
                        )?;
                        block_cursor += 1;
                    }
                    Region::PageBreak => push(
                        &mut items,
                        charge,
                        Item {
                            leading: Length::ZERO,
                            trailing: Length::ZERO,
                            viewport_left: None,
                            owner,
                            source: None,
                            x: body.x(),
                            width: body.width(),
                            height: Length::ZERO,
                            before: Length::ZERO,
                            after: Length::ZERO,
                            keep: false,
                        },
                    )?,
                    other => return Err(error(owner, E::PendingRegion(other.as_str()))),
                }
                stack
                    .try_reserve(1)
                    .map_err(|_| error(owner, E::AllocationFailure))?;
                stack.push(frame);
            }
            Event::Paragraph { index } => {
                let p = flow
                    .paragraphs()
                    .get(index as usize)
                    .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
                let owner = p.owner();
                if index as usize != paragraph_cursor
                    || stack.last().map(|f| f.owner) != Some(owner)
                {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                let selected = &lines.paragraphs()[index as usize];
                if selected.owner() != owner {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                if p.page_name().is_some() {
                    return Err(error(owner, E::PendingNamedPage));
                }
                let style = p.style().block_style();
                let (region_left, region_width) = list::region_frame(lines, body, owner)?;
                let width = region_width
                    .get()
                    .checked_sub(style.start_indent().get())
                    .and_then(|w| w.checked_sub(style.end_indent().get()))
                    .and_then(PositiveLength::new)
                    .ok_or_else(|| error(owner, E::WidthMismatch))?;
                if selected.inline_size() != width {
                    return Err(error(owner, E::WidthMismatch));
                }
                let selected_lines = selected
                    .selected()
                    .ok_or_else(|| error(owner, E::EmptyParagraph))?;
                for (line_index, line) in selected_lines.lines().iter().enumerate() {
                    let slack = width
                        .get()
                        .checked_sub(line.required_inline_size().get())
                        .filter(|s| *s >= Length::ZERO)
                        .ok_or_else(|| error(owner, E::WidthMismatch))?;
                    let offset = match (
                        line.required_inline_size().get() == Length::ZERO,
                        style.text_align(),
                    ) {
                        (true, _) => Length::ZERO,
                        (false, MachineTextAlign::Start) => Length::ZERO,
                        (false, MachineTextAlign::End) => slack,
                        (false, MachineTextAlign::Center) => Length::from_raw(slack.raw() / 2)
                            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
                    };
                    let x = add(
                        add(region_left, style.start_indent().get(), owner)?,
                        offset,
                        owner,
                    )?;
                    // Bounds cover the required visual width, avoiding a full
                    // frame rectangle translated beyond its end by alignment.
                    let bounds_width =
                        PositiveLength::new(line.required_inline_size().get()).unwrap_or(width);
                    let last = line_index + 1 == selected_lines.lines().len();
                    push(
                        &mut items,
                        charge,
                        Item {
                            leading: Length::ZERO,
                            trailing: Length::ZERO,
                            viewport_left: None,
                            owner,
                            source: Some(ProductionBodyFragmentSource::ParagraphLine {
                                paragraph_index: index,
                                line_index: line_index as u32,
                            }),
                            x,
                            width: bounds_width,
                            height: line.line().metrics().line_height().get(),
                            before: if line_index == 0 {
                                style.space_before().get()
                            } else {
                                Length::ZERO
                            },
                            after: if last {
                                style.space_after().get()
                            } else {
                                Length::ZERO
                            },
                            keep: last && style.keep_with_next(),
                        },
                    )?;
                }
                paragraph_cursor += 1;
            }
            Event::End { owner } => {
                let frame = stack
                    .pop()
                    .filter(|f| f.owner == owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                if let Some(definition) = definitions.and_then(|d| d.get(definition_ranges.len())) {
                    if definition.owner() == owner {
                        if !stack.is_empty() || definition.event_range().end != event_index + 1 {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                        let start = definition_start
                            .take()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        definition_ranges.push(start..items.len());
                    }
                }
                if let Some(index) = frame.marker {
                    marker_bindings[index].item_index = Some(
                        (frame.start..items.len())
                            .find(|i| list::has_paint(&items[*i], lines))
                            .ok_or_else(|| error(owner, E::EmptyListItem))?,
                    );
                }
                if let Some(style) = frame.container {
                    let first = (frame.start..items.len())
                        .find(|i| items[*i].source.is_some())
                        .ok_or_else(|| error(owner, E::EmptyParagraph))?;
                    let last = (frame.start..items.len())
                        .rev()
                        .find(|i| items[*i].source.is_some())
                        .unwrap();
                    items[first].before =
                        add(items[first].before, style.space_before().get(), owner)?;
                    items[last].after = add(items[last].after, style.space_after().get(), owner)?;
                    items[last].keep |= style.keep_with_next();
                }
                if let Some(index) = frame.vector {
                    let block = &blocks.blocks()[index];
                    let last = (frame.start..items.len())
                        .rev()
                        .find(|i| items[*i].source.is_some())
                        .ok_or_else(|| error(owner, E::EmptyParagraph))?;
                    if block.keep_caption() {
                        for item in &mut items[frame.start..last] {
                            item.keep = true;
                        }
                    }
                    items[last].after = add(items[last].after, block.space_after().get(), owner)?;
                    items[last].keep |= block.keep_with_next();
                }
                if let Some(index) = frame.raster {
                    let style = lines.figures()[index].source().style().block_style();
                    let last = (frame.start..items.len())
                        .rev()
                        .find(|i| items[*i].source.is_some())
                        .ok_or_else(|| error(owner, E::EmptyParagraph))?;
                    if style.keep_caption() {
                        for item in &mut items[frame.start..last] {
                            item.keep = true;
                        }
                    }
                    items[last].after = add(items[last].after, style.space_after().get(), owner)?;
                    items[last].keep |= style.keep_with_next();
                }
            }
        }
    }
    if !stack.is_empty()
        || paragraph_cursor != lines.paragraphs().len()
        || block_cursor != blocks.blocks().len()
        || figure_cursor != lines.figures().len()
        || list_cursor != flow.lists().len()
        || marker_bindings.len() != flow.list_items().len()
    {
        return Err(error(NodeId::new(0), E::ReceiptMismatch));
    }
    if definition_start.is_some() || definition_ranges.len() != definitions.map_or(0, |d| d.len()) {
        return Err(error(NodeId::new(0), E::ReceiptMismatch));
    }
    Ok(CollectedItems {
        body_end: body_end.unwrap_or(items.len()),
        items,
        marker_bindings,
        definitions: definition_ranges,
    })
}

fn fingerprint(
    layout: &ProductionBodySelectedLayout<'_, '_, '_>,
) -> Result<[u8; 32], ProductionBodyPaginationError> {
    // Fixed-size records avoid one unbounded decimal/JSON rendering per glyph.
    let decision_records = layout
        .decisions
        .iter()
        .try_fold(0usize, |sum, d| {
            sum.checked_add(1)?.checked_add(d.candidates().len())
        })
        .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow))?;
    let records = layout
        .pages
        .len()
        .checked_add(decision_records)
        .and_then(|n| n.checked_add(layout.fragments.len()))
        .and_then(|n| n.checked_add(layout.breaks.len()))
        .and_then(|n| n.checked_add(layout.list_markers.len()))
        .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(
            records
                .checked_mul(128)
                .and_then(|n| n.checked_add(160))
                .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow))?,
        )
        .map_err(|_| error(NodeId::new(0), E::AllocationFailure))?;
    bytes.extend_from_slice(&sha256(PRODUCTION_BODY_PAGINATION_ALGORITHM.as_bytes()));
    bytes.extend_from_slice(&sha256(PRODUCTION_BODY_BREAK_POLICY.as_bytes()));
    bytes.extend_from_slice(&layout.lines.fingerprint());
    bytes.extend_from_slice(&layout.blocks.receipt().fingerprint());
    bytes.extend_from_slice(&layout.limits_fingerprint);
    for p in &layout.pages {
        bytes.push(0);
        bytes.extend_from_slice(&p.first_fragment.to_be_bytes());
        bytes.extend_from_slice(&p.fragment_count.to_be_bytes());
        bytes.extend_from_slice(&p.used_height.raw().to_be_bytes());
    }
    for f in &layout.fragments {
        bytes.push(1);
        bytes.extend_from_slice(&f.owner.get().to_be_bytes());
        bytes.extend_from_slice(&f.page_index.to_be_bytes());
        match f.source {
            ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } => {
                bytes.push(0);
                bytes.extend_from_slice(&paragraph_index.to_be_bytes());
                bytes.extend_from_slice(&line_index.to_be_bytes());
            }
            ProductionBodyFragmentSource::VectorBlock { block_index } => {
                bytes.push(1);
                bytes.extend_from_slice(&block_index.to_be_bytes());
            }
            ProductionBodyFragmentSource::RasterFigure { figure_index } => {
                bytes.push(2);
                bytes.extend_from_slice(&figure_index.to_be_bytes());
            }
        }
        for v in [
            f.bounds.x().raw(),
            f.bounds.y().raw(),
            f.bounds.width().get().raw(),
            f.bounds.height().get().raw(),
            f.effective_space_before.raw(),
        ] {
            bytes.extend_from_slice(&v.to_be_bytes());
        }
        bytes.push(u8::from(f.baseline.is_some()));
        if let Some(b) = f.baseline {
            bytes.extend_from_slice(&b.raw().to_be_bytes());
        }
        bytes.push(u8::from(f.viewport.is_some()));
        if let Some(r) = f.viewport {
            for v in [
                r.x().raw(),
                r.y().raw(),
                r.width().get().raw(),
                r.height().get().raw(),
            ] {
                bytes.extend_from_slice(&v.to_be_bytes());
            }
        }
    }
    for b in &layout.breaks {
        bytes.push(2);
        bytes.extend_from_slice(&b.owner.get().to_be_bytes());
        bytes.extend_from_slice(&b.produced_page_index.to_be_bytes());
    }
    for marker in &layout.list_markers {
        list::encode_marker(marker, &mut bytes);
    }
    for decision in &layout.decisions {
        page_breaks::encode(decision, &mut bytes);
    }
    Ok(sha256(&bytes))
}
