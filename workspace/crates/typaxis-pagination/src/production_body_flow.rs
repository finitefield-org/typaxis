//! Measured body and definition streams, before page reservation or assignment.
use super::*;

#[path = "production_footnote_breaks.rs"]
mod footnote_breaks;
pub use footnote_breaks::{
    prepare_production_footnote_demand_search, prepare_production_footnote_search,
    ProductionFootnoteBreakSearch, ProductionFootnoteCursor, ProductionFootnoteDemandSearch,
    ProductionFootnoteDemandSelection, ProductionFootnoteDemandState,
    ProductionFootnoteDemandStatus, ProductionFootnoteFragmentSelection,
    ProductionFootnoteRegionFragment, ProductionFootnoteRegionSelection,
};

#[path = "production_footnote_references.rs"]
mod footnote_references;
pub use footnote_references::ProductionFootnoteFlowReference;

impl ProductionBodyFlowItem {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    /// None is an authored forced page break, not a paint fragment.
    pub const fn source(&self) -> Option<ProductionBodyFragmentSource> {
        self.source
    }
    /// Absolute page x for paintable items; forced-break geometry is unused.
    pub const fn x(&self) -> Length {
        self.x
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
    /// Content height, before the marker leading/trailing expansion.
    pub const fn height(&self) -> Length {
        self.height
    }
    /// Authored accumulated spacing; page-start suppression is not selected yet.
    pub const fn space_before(&self) -> Length {
        self.before
    }
    pub const fn space_after(&self) -> Length {
        self.after
    }
    pub const fn keep_with_next(&self) -> bool {
        self.keep
    }
    pub const fn leading(&self) -> Length {
        self.leading
    }
    pub const fn trailing(&self) -> Length {
        self.trailing
    }
    pub const fn viewport_left(&self) -> Option<Length> {
        self.viewport_left
    }
    pub fn consumed_height(&self) -> Result<Length, ProductionBodyPaginationError> {
        self.consumed()
    }
}

/// Definition label bound to one item in that definition's local stream.
/// Baseline is relative to the content top, before the item's leading offset.
/// This binding carries no page or paint permission.
pub struct ProductionFootnoteMarkerBinding {
    owner: NodeId,
    definition_index: usize,
    item_index: usize,
    baseline: Length,
}
impl ProductionFootnoteMarkerBinding {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn definition_index(&self) -> usize {
        self.definition_index
    }
    pub const fn item_index(&self) -> usize {
        self.item_index
    }
    pub const fn baseline(&self) -> Length {
        self.baseline
    }
}

/// Owns one source-ordered collection. Definition slices never join the body
/// slice; no page index, vertical position, continuation or paint is authorized.
pub struct ProductionPreparedBodyFlow<'f, 's, 'p, 'a> {
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    blocks: &'s StagingPrecomposedVectorBlockLayout,
    footnotes: &'f typaxis_layout::ProductionFootnoteLines<'s, 'p, 'a>,
    collected: CollectedItems,
    definition_markers: Vec<ProductionFootnoteMarkerBinding>,
    references: Vec<ProductionFootnoteFlowReference<'f>>,
    record_charge: u64,
    limits_fingerprint: [u8; 32],
}
impl<'f, 's, 'p, 'a> ProductionPreparedBodyFlow<'f, 's, 'p, 'a> {
    pub fn body_items(&self) -> &[ProductionBodyFlowItem] {
        &self.collected.items[..self.collected.body_end]
    }
    pub fn definition_items(&self, definition_index: usize) -> Option<&[ProductionBodyFlowItem]> {
        self.collected
            .definitions
            .get(definition_index)
            .map(|r| &self.collected.items[r.clone()])
    }
    pub fn definition_marker(
        &self,
        definition_index: usize,
    ) -> Option<&ProductionFootnoteMarkerBinding> {
        self.definition_markers.get(definition_index)
    }
    pub fn references(&self) -> &[ProductionFootnoteFlowReference<'f>] {
        &self.references
    }
    /// Actual occurrences intersecting a local item range. This read-only query
    /// does not authorize the supplied range as a page or fragment selection.
    pub fn references_in_items(
        &self,
        source_definition: Option<usize>,
        range: std::ops::Range<usize>,
    ) -> &[ProductionFootnoteFlowReference<'f>] {
        if range.start >= range.end {
            return &[];
        }
        let start = self.references.partition_point(|r| {
            r.source().source_definition() < source_definition
                || (r.source().source_definition() == source_definition
                    && r.last_item_index() < range.start)
        });
        let remaining = &self.references[start..];
        let end = remaining.partition_point(|r| {
            r.source().source_definition() == source_definition && r.first_item_index() < range.end
        });
        &remaining[..end]
    }
    pub fn footnotes(&self) -> &typaxis_layout::ProductionFootnoteLines<'s, 'p, 'a> {
        self.footnotes
    }
    pub fn footnote_region(&self) -> Option<Rect> {
        self.lines.frames().and_then(|f| f.footnote_region())
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn list_marker_count(&self) -> usize {
        self.collected.marker_bindings.len()
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
        self.footnotes
            .verify(lines, limits)
            .map_err(|e| error(e.owner, E::ReceiptMismatch))
    }
}

pub fn prepare_production_body_flow<'f, 's, 'p, 'a>(
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    blocks: &'s StagingPrecomposedVectorBlockLayout,
    footnotes: &'f typaxis_layout::ProductionFootnoteLines<'s, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionPreparedBodyFlow<'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    verify_flow_inputs(lines, blocks, limits)?;
    let root = NodeId::new(0);
    footnotes
        .verify(lines, limits)
        .map_err(|e| error(e.owner, E::ReceiptMismatch))?;
    if let Some(first) = footnotes.definitions().first() {
        if lines.frames().and_then(|f| f.footnote_region()).is_none() {
            return Err(error(first.owner(), E::PendingRegion("footnote_frame")));
        }
    }
    let mut charge = Charge {
        remaining: limits
            .base()
            .get()
            .max_fragments
            .checked_sub(footnotes.record_charge())
            .and_then(|n| n.checked_sub(blocks.blocks().len() as u64))
            .ok_or_else(|| error(root, E::FragmentLimit))?,
    };
    let mut collected = collect_items(lines, blocks, Some(footnotes.definitions()), &mut charge)?;
    list::prepare_metrics(
        lines,
        blocks,
        &mut collected.items,
        &mut collected.marker_bindings,
    )?;
    let markers = lines.footnote_markers();
    if markers.len() != collected.definitions.len() {
        return Err(error(root, E::ReceiptMismatch));
    }
    charge.take(markers.len(), root)?;
    let mut definition_markers = Vec::new();
    definition_markers
        .try_reserve_exact(markers.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    for (definition_index, (marker, range)) in
        markers.iter().zip(&collected.definitions).enumerate()
    {
        let owner = marker.source().owner();
        if footnotes.definitions()[definition_index].owner() != owner {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let items = &mut collected.items[range.clone()];
        let item_index = items
            .iter()
            .position(|item| list::has_paint(item, lines))
            .ok_or_else(|| error(owner, E::EmptyFootnote))?;
        let baseline = list::prepare_marker_metrics(
            lines,
            blocks,
            &mut items[item_index],
            marker.font().ascender(),
            marker.font().descender(),
            owner,
        )?;
        definition_markers.push(ProductionFootnoteMarkerBinding {
            owner,
            definition_index,
            item_index,
            baseline,
        });
    }
    let references =
        footnote_references::prepare_references(footnotes.references(), &collected, &mut charge)?;
    let record_charge = limits.base().get().max_fragments - charge.remaining;
    Ok(ProductionPreparedBodyFlow {
        lines,
        blocks,
        footnotes,
        collected,
        definition_markers,
        references,
        record_charge,
        limits_fingerprint: limits.fingerprint(),
    })
}
