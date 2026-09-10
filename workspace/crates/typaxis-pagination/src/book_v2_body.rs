//! Combined source-ordered measurements under the exact successor owners.
use super::*;
use typaxis_layout::book_v2::{
    BookV2FootnoteLines, BookV2InlineLineLayout, BookV2VectorBlockLayout,
};

/// Paragraphs, vector/native display math, figures and definition streams.
/// Table boundaries are retained around their actual cell leaves; cell streams
/// are not yet measured as rows or selected as pages by this owner.
pub struct BookV2PreparedBodyFlow<'f, 's, 'p, 'a> {
    lines: &'s BookV2InlineLineLayout<'p, 'a>,
    blocks: Option<&'f BookV2VectorBlockLayout<'s, 'p, 'a>>,
    footnotes: &'f BookV2FootnoteLines<'s, 'p, 'a>,
    pub(super) collected: CollectedItems,
    pub(super) definition_markers: Vec<ProductionFootnoteMarkerBinding>,
    references: Vec<ProductionFootnoteFlowReference<'f>>,
    record_charge: u64,
    prior_records: u64,
    limits_fingerprint: [u8; 32],
    body_page_names: Vec<Option<usize>>,
    table_page_names: Vec<Option<usize>>,
}
impl<'f, 's, 'p, 'a> BookV2PreparedBodyFlow<'f, 's, 'p, 'a> {
    pub fn body_page_name_index(&self, item: usize) -> Option<usize> {
        self.body_page_names.get(item).copied().flatten()
    }
    pub fn table_page_name_index(&self, table: usize) -> Option<usize> {
        self.table_page_names.get(table).copied().flatten()
    }
    pub fn body_items(&self) -> &[ProductionBodyFlowItem] {
        &self.collected.items[..self.collected.body_end]
    }
    pub fn definition_items(&self, index: usize) -> Option<&[ProductionBodyFlowItem]> {
        self.collected
            .definitions
            .get(index)
            .map(|r| &self.collected.items[r.clone()])
    }
    pub fn definition_marker(&self, index: usize) -> Option<&ProductionFootnoteMarkerBinding> {
        self.definition_markers.get(index)
    }
    pub fn references(&self) -> &[ProductionFootnoteFlowReference<'f>] {
        &self.references
    }
    pub fn references_in_items(
        &self,
        source_definition: Option<usize>,
        range: std::ops::Range<usize>,
    ) -> &[ProductionFootnoteFlowReference<'f>] {
        super::references_in_items(&self.references, source_definition, range)
    }
    pub fn list_marker_count(&self) -> usize {
        self.collected.marker_bindings.len()
    }
    pub fn table_count(&self) -> usize {
        self.collected.tables.tables.len()
    }
    /// Outer None is an unknown table index; inner None is the actual body.
    /// An empty body table can share a leaf index with a following definition.
    pub fn table_source_definition(&self, index: usize) -> Option<Option<usize>> {
        self.collected.tables.tables.get(index).map(|table| table.definition)
    }
    pub fn table_parent(&self, index: usize) -> Option<Option<usize>> {
        self.collected.tables.tables.get(index).map(|table| table.parent)
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn lines(&self) -> &'s BookV2InlineLineLayout<'p, 'a> {
        self.lines
    }
    pub fn blocks(&self) -> Option<&'f BookV2VectorBlockLayout<'s, 'p, 'a>> {
        self.blocks
    }
    pub fn footnotes(&self) -> &'f BookV2FootnoteLines<'s, 'p, 'a> {
        self.footnotes
    }
    pub fn verify(
        &self,
        lines: &BookV2InlineLineLayout<'_, '_>,
        blocks: Option<&BookV2VectorBlockLayout<'_, '_, '_>>,
        footnotes: &BookV2FootnoteLines<'_, '_, '_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyPaginationError> {
        let same_blocks = match (self.blocks, blocks) {
            (None, None) => true,
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            _ => false,
        };
        if !std::ptr::eq(self.lines, lines)
            || !same_blocks
            || !std::ptr::eq(self.footnotes, footnotes)
            || self.limits_fingerprint != limits.fingerprint()
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}
/// Collect every actual selected leaf, list label and footnote edge. The prior
/// count excludes new block/number storage; independent preparations therefore
/// cannot hide each other's retained records behind a maximum of total counts.
pub fn prepare_book_v2_body_flow<'f, 's, 'p, 'a>(
    lines: &'s BookV2InlineLineLayout<'p, 'a>,
    blocks: Option<&'f BookV2VectorBlockLayout<'s, 'p, 'a>>,
    footnotes: &'f BookV2FootnoteLines<'s, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    prior_records: u64,
) -> Result<BookV2PreparedBodyFlow<'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    footnotes
        .verify(lines, limits)
        .map_err(|e| error(e.owner, E::ReceiptMismatch))?;
    if let Some(blocks) = blocks {
        blocks
            .verify(lines, limits)
            .map_err(|_| error(root, E::ReceiptMismatch))?;
    }
    let frames = lines
        .frames()
        .ok_or_else(|| error(root, E::PendingRegion("body_frame")))?;
    frames
        .verify(lines.prepared(), frames.body())
        .map_err(|e| error(e.owner, E::ReceiptMismatch))?;
    if let Some(first) = footnotes.definitions().first() {
        if frames.footnote_region().is_none() {
            return Err(error(first.owner(), E::PendingRegion("footnote_frame")));
        }
    }
    let prior_records = prior_records
        .max(footnotes.record_charge())
        .max(blocks.map_or(0, |b| b.prior_records()));
    let base = prior_records
        .checked_add(blocks.map_or(0, |b| b.retained_records()))
        .ok_or_else(|| error(root, E::FragmentLimit))?;
    let mut charge = Charge {
        remaining: limits
            .base()
            .get()
            .max_fragments
            .checked_sub(base)
            .ok_or_else(|| error(root, E::FragmentLimit))?,
    };
    charge.take(1, root)?;
    let view = BodyLines::BookV2(lines);
    let block_view = BodyBlocks::BookV2(blocks.map_or(&[], |b| b.blocks()));
    let mut collected = collect_items_shared(
        view,
        block_view,
        frames.body(),
        Some(footnotes.definitions()),
        &mut charge,
        true,
    )?;
    let mut body_page_names = Vec::new();
    let mut table_page_names = Vec::new();
    if let Some(plan) = frames.page_plan().filter(|p|p.has_source_names()) {
        charge.take(collected.body_end.checked_add(collected.tables.tables.len())
            .and_then(|n|n.checked_add(2)).ok_or_else(||error(root,E::FragmentLimit))?,root)?;
        body_page_names.try_reserve_exact(collected.body_end).map_err(|_|error(root,E::AllocationFailure))?;
        table_page_names.try_reserve_exact(collected.tables.tables.len()).map_err(|_|error(root,E::AllocationFailure))?;
        body_page_names.extend(collected.items[..collected.body_end].iter().map(|i|plan.source_name_index(i.owner)));
        for table in &collected.tables.tables {
            let name=plan.source_name_index(table.owner);
            if table.definition.is_none() {
                if let Some(parent)=table.parent {
                    if table_page_names.get(parent).copied()!=Some(name) { return Err(error(table.owner,E::PendingNamedPage)); }
                } else {
                    let names=body_page_names.get(table.items.clone()).ok_or_else(||error(table.owner,E::ReceiptMismatch))?;
                    if let Some(i)=names.iter().position(|n|*n!=name) {
                        return Err(error(collected.items[table.items.start+i].owner,E::PendingNamedPage));
                    }
                }
            }
            table_page_names.push(name);
        }
    }
    let (definition_markers, references) = finish_collection(
        view,
        block_view,
        footnotes.definitions(),
        footnotes.references(),
        &mut collected,
        &mut charge,
    )?;
    Ok(BookV2PreparedBodyFlow {
        lines,
        blocks,
        footnotes,
        collected,
        definition_markers,
        references,
        record_charge: limits.base().get().max_fragments - charge.remaining,
        prior_records,
        limits_fingerprint: limits.fingerprint(),
        body_page_names,
        table_page_names,
    })
}

pub const BOOK_V2_TABLE_MEASUREMENT_ALGORITHM: &str = "typaxis.book-2-table-measurements/1";
#[path = "book_v2_table_header_variant.rs"]
mod header_variant;
pub use header_variant::{
    prepare_book_v2_table_header_variant, BookV2TableHeaderVariant, BookV2TableHeaderVariantLeaf,
};
#[path = "book_v2_table_header_catalog.rs"]
mod header_catalog;
pub use header_catalog::{prepare_book_v2_table_header_catalog, BookV2TableHeaderCatalog};
/// Actual row bands and nested child extents, retaining the complete dedicated
/// body owner. No legacy table/page receipt is minted by this projection.
pub struct BookV2TableMeasurements<'f, 's, 'p, 'a> {
    flow: BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    projection: table_measurements::TableMeasurementProjection,
}
impl<'f, 's, 'p, 'a> BookV2TableMeasurements<'f, 's, 'p, 'a> {
    /// Historical input/line records, excluding retained block, body and table
    /// projections. Exact set membership is still required before sharing lines.
    pub fn prior_records(&self) -> u64 {
        self.flow.prior_records
    }
    pub fn retained_records(&self) -> u64 {
        self.record_charge() - self.prior_records()
    }
    pub fn flow(&self) -> &BookV2PreparedBodyFlow<'f, 's, 'p, 'a> {
        &self.flow
    }
    pub fn tables(&self) -> &[ProductionMeasuredTable] {
        &self.projection.tables
    }
    pub fn item(&self, index: usize) -> Option<&ProductionBodyFlowItem> {
        self.flow.collected.items.get(index)
    }
    pub fn record_charge(&self) -> u64 {
        self.projection.record_charge
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.projection.fingerprint
    }
}
pub fn prepare_book_v2_table_measurements<'f, 's, 'p, 'a>(
    flow: BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
) -> Result<BookV2TableMeasurements<'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    flow.verify(flow.lines, flow.blocks, flow.footnotes, limits)?;
    let projection = table_measurements::project_table_measurements(
        table_measurements::TableMeasurementInputs {
            collected: &flow.collected,
            sources: flow.lines.prepared().source_flow().tables(),
            prior_records: flow.record_charge(),
            max_canonical_bytes: Some(limits.base().get().max_spool_bytes),
            fingerprint_header: [
                sha256(BOOK_V2_TABLE_MEASUREMENT_ALGORITHM.as_bytes()),
                flow.lines.fingerprint(),
                flow.blocks.map_or([0; 32], |b| b.fingerprint()),
                limits.fingerprint(),
            ],
        },
        limits,
    )?;
    Ok(BookV2TableMeasurements { flow, projection })
}

pub use table_measurements::{
    prepare_book_v2_table_footnote_search, BookV2TableFootnoteSearch, BookV2TableFootnoteState, BookV2TableFootnoteSelection,
    prepare_book_v2_table_search, BookV2TableBreakSearch, BookV2TableCursor,
    BookV2TableFragmentSelection, BookV2TableSourceLeaf, BOOK_V2_TABLE_FRAGMENT_ALGORITHM,
    BookV2TableHeaderFragmentSelection, BookV2TableHeaderPaintLeaf,
    prepare_book_v2_definition_table_demand_search,BookV2DefinitionTableDemandSearch,BookV2DefinitionTableDemandState,BookV2DefinitionTableDemandSelection,
};

pub use footnote_breaks::book_v2::{
    prepare_book_v2_footnote_search, BookV2FootnoteBreakSearch, BookV2FootnoteCursor,
    BookV2FootnoteFragmentSelection,
};

pub use footnote_breaks::{
    BookV2DefinitionCandidates, BookV2RankedDefinitionCandidate, prepare_book_v2_mixed_footnote_demand_search, prepare_book_v2_definition_mixed_search, BookV2DefinitionMixedSearch, BookV2DefinitionCandidatePart, BookV2DefinitionSelectedPart, BookV2DefinitionSourceState, BookV2DefinitionMixedCandidate,
    BookV2FootnoteRegionFragment, BookV2FootnoteRegionSelection,
    BookV2BodyFootnoteCandidate,
    BookV2BodyMixedStablePages, BookV2BodyPlacedHeaderVariant, BookV2BodyPlacedEquationNumber, BookV2BodySourceClosure,
    BookV2TableWidthSource, BookV2TableWidthPiece, BookV2TableWidthOccurrence, BookV2TableWidthOccurrences, BookV2ParagraphWidthCandidate, BookV2ParagraphWidthFeedback,
    BookV2BodyMathSource, BookV2BodyMathTerminal, BookV2BodyMathTerminals, BOOK_V2_BODY_MATH_TERMINAL_ALGORITHM,
    BookV2BodyMixedPlacedPage, BookV2BodyMixedPlacedSequence,
    BookV2BodyMixedPageState, BookV2BodyMixedPageSelection, BookV2BodyMixedPageSequence,
    BookV2BodyCandidatePart,BookV2BodySelectedPart,BookV2BodyMixedCandidate,BookV2BodySourceState,
    prepare_book_v2_table_body_search, prepare_book_v2_table_body_search_with_headers,
    prepare_book_v2_footnote_demand_search, BookV2FootnoteDemandSearch,
    BookV2FootnoteDemandSelection, BookV2FootnoteDemandState,
};
