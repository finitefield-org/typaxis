//! Dedicated book-2 table cursors and fragment selections over the shared kernel.
use super::*;
use crate::production_body::body_flow::book_v2::BookV2TableMeasurements;

#[path = "book_v2_table_body_context.rs"]
mod body_context;
#[path = "book_v2_nested_table_breaks.rs"]
mod nested;
#[path = "book_v2_table_source_leaves.rs"]
mod source_leaves;
pub(in crate::production_body::body_flow) use body_context::BookV2TableBodyContext;
pub use source_leaves::BookV2TableSourceLeaf;
#[path = "book_v2_table_header_selection.rs"]
mod header_selection;
pub use header_selection::{BookV2TableHeaderFragmentSelection, BookV2TableHeaderPaintLeaf};

#[path = "book_v2_table_footnotes.rs"]
mod footnotes;
pub use footnotes::{
    prepare_book_v2_table_footnote_search, BookV2TableFootnoteSearch, BookV2TableFootnoteSelection,
    BookV2TableFootnoteState,
};

pub const BOOK_V2_TABLE_FRAGMENT_ALGORITHM: &str = "typaxis.book-2-table-fragment/1";
#[derive(Clone, Copy)]
pub struct BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
    measurements: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    position: TablePosition,
}
impl BookV2TableCursor<'_, '_, '_, '_, '_> {
    pub fn table_index(&self) -> usize {
        self.table_index
    }
    /// Caption/common-cut coordinate, or accumulated fragment extent while
    /// independent body-cell cursors are active. Cell source positions are
    /// separately bound by `cell_progress_fingerprint`.
    pub fn offset(&self) -> Length {
        self.position.offset
    }
    pub fn next_row(&self) -> usize {
        self.position.row
    }
    pub fn is_initial(&self) -> bool {
        self.position.initial
    }
    pub fn next_caption_item(&self) -> usize {
        self.position.caption_next
    }
    pub fn has_started_rows(&self) -> bool {
        self.position.header_seen
    }
    pub fn cell_progress_fingerprint(&self) -> [u8; 32] {
        self.position.cell_fingerprint
    }
    pub fn is_terminal(&self) -> bool {
        !self.position.initial
            && self.position.header_seen
            && self.position.row == self.measurements.tables()[self.table_index].rows.len()
    }
}
pub struct BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a> {
    before: BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
    projection: kernel::TableFragmentProjection,
    nested: Option<nested::Projection<'m, 'f, 's, 'p, 'a>>,
    has_nested_header_variants: bool,
    header_variant: Option<&'m crate::book_v2::BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a>>,
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) fn verify_demand_flow(
        &self,
        flow: &crate::production_body::body_flow::book_v2::BookV2PreparedBodyFlow<'_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.verify_source_flow(flow, None)
    }
    pub(in crate::production_body::body_flow) fn verify_source_flow(
        &self,
        flow: &crate::production_body::body_flow::book_v2::BookV2PreparedBodyFlow<'_, '_, '_, '_>,
        definition: Option<usize>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if !std::ptr::eq(self.before.measurements.flow(), flow)
            || self.definition_index() != definition
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
    /// Selected original leaf ranges. Repeated headers contribute no duplicate
    /// semantic references; this query still grants no page or paint authority.
    pub fn semantic_leaf_ranges(&self) -> impl Iterator<Item = Range<usize>> + '_ {
        let measured = self.before.measurements;
        let caption = self.caption_content();
        let caption_range = caption.first().zip(caption.last()).map(|(first, last)| {
            let (
                ProductionTableContentSource::FlowItem(first),
                ProductionTableContentSource::FlowItem(last),
            ) = (first.source, last.source)
            else {
                unreachable!("search rejects nested caption tables");
            };
            first..last + 1
        });
        caption_range
            .into_iter()
            .chain(selected_semantic_leaf_ranges(
                &measured.tables()[self.before.table_index],
                &measured.flow().lines().prepared().source_flow().tables()[self.before.table_index],
                &self.projection.cells,
                self.nested.is_none()
                    && !self.projection.header_source_slices
                    && !self.before.position.header_seen
                    && self.projection.header_top.is_some(),
            ))
            .chain(
                self.nested
                    .iter()
                    .flat_map(|p| p.semantic.iter().map(|i| *i..*i + 1)),
            )
    }
    /// This table's own repeated header, if separately measured. Child headers
    /// are exposed through `variant_placement_leaves` and `has_header_variants`.
    pub fn header_variant(
        &self,
    ) -> Option<&'m crate::book_v2::BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a>> {
        self.header_variant
    }
    pub fn has_header_variants(&self) -> bool {
        self.header_variant.is_some() || self.has_nested_header_variants
    }
    /// Variant-aware paint with exact owners, including independently continued children.
    /// Empty for ordinary fragments.
    pub fn variant_placement_leaves(
        &self,
    ) -> impl Iterator<
        Item = Result<
            BookV2TableHeaderPaintLeaf<'m, 'f, 's, 'p, 'a>,
            ProductionBodyPaginationError,
        >,
    > + '_ {
        std::iter::once(())
            .filter(|_| self.has_header_variants())
            .flat_map(move |_| header_selection::paint_leaves(self, self.header_variant))
    }
    pub fn placement_leaves(
        &self,
    ) -> impl Iterator<Item = Result<(NodeId, usize, Length, bool), ProductionBodyPaginationError>> + '_
    {
        self.has_header_variants()
            .then(|| {
                Err(error(
                    NodeId::new(0),
                    E::PendingRegion("table_header_variant_placement"),
                ))
            })
            .into_iter()
            .chain(
                self.placement_leaves_single_owner()
                    .filter(|_| !self.has_header_variants()),
            )
    }
    fn placement_leaves_single_owner(
        &self,
    ) -> impl Iterator<Item = Result<(NodeId, usize, Length, bool), ProductionBodyPaginationError>> + '_
    {
        let measured = self.before.measurements;
        selected_placement_leaves(
            &measured.tables()[self.before.table_index],
            &measured.flow().lines().prepared().source_flow().tables()[self.before.table_index],
            &self.projection.cells,
            self.repeats_header(),
            self.projection.header_top.filter(|_| self.nested.is_none()),
        )
        .filter(|leaf| match leaf {
            Ok((_, index, _, _)) => measured.flow().collected.items[*index].source.is_some(),
            Err(_) => true,
        })
        .chain(self.nested.iter().flat_map(|p| {
            p.leaves
                .iter()
                .filter_map(|l| l.cell.map(|cell| Ok((cell, l.item, l.top, l.repeated))))
        }))
    }
    fn caption_content(&self) -> &[ProductionTableCellContent] {
        self.before.measurements.tables()[self.before.table_index]
            .caption
            .as_ref()
            .map_or(&[], |caption| {
                &caption.content[self.projection.caption.clone()]
            })
    }
    /// Caption positions for callers that do not consume paint roles.
    pub fn caption_placement_leaves(
        &self,
    ) -> impl Iterator<Item = Result<(usize, Length), ProductionBodyPaginationError>> + '_ {
        self.caption_placement_leaves_with_repetition()
            .map(|leaf| leaf.map(|(i, top, _)| (i, top)))
    }
    /// Caption leaves have no cell role; a containing header can repeat them.
    pub fn caption_placement_leaves_with_repetition(
        &self,
    ) -> impl Iterator<Item = Result<(usize, Length, bool), ProductionBodyPaginationError>> + '_
    {
        self.has_header_variants()
            .then(|| {
                Err(error(
                    NodeId::new(0),
                    E::PendingRegion("table_header_variant_placement"),
                ))
            })
            .into_iter()
            .chain(
                self.caption_placement_leaves_single_owner()
                    .filter(|_| !self.has_header_variants()),
            )
    }
    fn caption_placement_leaves_single_owner(
        &self,
    ) -> impl Iterator<Item = Result<(usize, Length, bool), ProductionBodyPaginationError>> + '_
    {
        self.caption_content()
            .iter()
            .filter_map(|content| {
                let ProductionTableContentSource::FlowItem(index) = content.source else {
                    return Some(Err(error(NodeId::new(0), E::ReceiptMismatch)));
                };
                if self.before.measurements.flow().collected.items[index]
                    .source
                    .is_none()
                {
                    return None;
                }
                Some(
                    content
                        .top
                        .checked_sub(self.before.position.offset)
                        .map(|top| (index, top, false))
                        .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow)),
                )
            })
            .chain(self.nested.iter().flat_map(|p| {
                p.leaves
                    .iter()
                    .filter(|l| l.cell.is_none())
                    .map(|l| Ok((l.item, l.top, l.repeated)))
            }))
    }
    pub fn before(&self) -> BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
        self.before
    }
    pub fn after(&self) -> BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
        BookV2TableCursor {
            position: self.projection.after,
            ..self.before
        }
    }
    pub fn header_height(&self) -> Length {
        self.projection.header_height
    }
    pub fn used_height(&self) -> Length {
        self.projection.used_height
    }
    pub fn available_height(&self) -> Length {
        self.projection.available_height
    }
    pub fn repeats_header(&self) -> bool {
        (self.before.position.header_seen || self.projection.header_source_slices)
            && self.projection.header_top.is_some()
    }
    pub fn cells(&self) -> &[ProductionTableCellSlice] {
        &self.projection.cells
    }
    pub fn forced_break_owner(&self) -> Option<NodeId> {
        self.projection.forced_break
    }
    pub(in crate::production_body::body_flow) fn forced_break_items(
        &self,
    ) -> impl Iterator<Item = usize> + '_ {
        let caption =
            if self.projection.break_items.is_empty() && self.forced_break_owner().is_some() {
                self.caption_content().last().and_then(|c| match c.source {
                    ProductionTableContentSource::FlowItem(index) => Some(index),
                    ProductionTableContentSource::Table(_) => None,
                })
            } else {
                None
            };
        caption
            .into_iter()
            .chain(self.projection.break_items.iter().copied())
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.projection.fingerprint
    }
}
/// Mutable search work and retained candidates share one cumulative budget.
/// Selections have no page index or repeated-header/PDF paint permission.
pub struct BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a> {
    measurements: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    kernel: TableBreakKernel<'m>,
    nested: Option<nested::Search<'m, 'f, 's, 'p, 'a>>,
    header_measurements: Vec<&'m BookV2TableMeasurements<'f, 's, 'p, 'a>>,
    frame_catalog: Option<&'m crate::book_v2::BookV2TableHeaderCatalog<'m, 'f, 's, 'p, 'a>>,
    frame_width: Option<PositiveLength>,
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a> {
    pub fn header_height(&self) -> Length {
        self.kernel.header_height
    }
    pub fn maximum_height(&self) -> Length {
        self.kernel.maximum_height
    }
    pub fn work_charge(&self) -> u64 {
        self.kernel.work.used
    }
    pub fn record_charge(&self) -> u64 {
        self.kernel.maximum_records - self.kernel.charge.remaining
    }
    pub fn begin(
        &mut self,
    ) -> Result<BookV2TableCursor<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.kernel
            .charge
            .take(1, self.measurements.tables()[self.table_index].owner)?;
        Ok(BookV2TableCursor {
            measurements: self.measurements,
            table_index: self.table_index,
            position: TablePosition {
                offset: if self.nested.is_some()
                    || self.kernel.header_breaks
                    || self.measurements.tables()[self.table_index]
                        .caption
                        .is_some()
                {
                    Length::ZERO
                } else {
                    self.kernel.header_height
                },
                row: if self.nested.is_some()
                    || self.measurements.tables()[self.table_index]
                        .caption
                        .is_some()
                {
                    0
                } else if self.kernel.header_breaks {
                    0
                } else {
                    self.kernel.header_rows
                },
                initial: true,
                header_seen: false,
                caption_next: 0,
                cells: None,
                cell_fingerprint: [0; 32],
            },
        })
    }
    pub fn evaluate(
        &mut self,
        cursor: &BookV2TableCursor<'_, '_, '_, '_, '_>,
        available: Length,
    ) -> Result<
        Option<BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        if !std::ptr::eq(cursor.measurements, self.measurements)
            || cursor.table_index != self.table_index
        {
            return Err(error(
                self.measurements.tables()[self.table_index].owner,
                E::ReceiptMismatch,
            ));
        }
        if self.nested.is_some() {
            return self.evaluate_nested(cursor, available);
        }
        let Some(projection) = self.kernel.evaluate(&cursor.position, available)? else {
            return Ok(None);
        };
        let before = BookV2TableCursor {
            measurements: self.measurements,
            table_index: self.table_index,
            position: cursor.position,
        };
        Ok(Some(BookV2TableFragmentSelection {
            before,
            projection,
            nested: None,
            header_variant: None,
            has_nested_header_variants: false,
        }))
    }
}
pub fn prepare_book_v2_table_search<'m, 'f, 's, 'p, 'a>(
    measurements: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    measurements.flow().verify(
        measurements.flow().lines(),
        measurements.flow().blocks(),
        measurements.flow().footnotes(),
        limits,
    )?;
    let owner = measurements
        .tables()
        .get(table_index)
        .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?
        .owner;
    let charge = Charge {
        remaining: limits
            .base()
            .get()
            .max_fragments
            .checked_sub(prior_records.max(measurements.record_charge()))
            .ok_or_else(|| error(owner, E::FragmentLimit))?,
    };
    prepare_book_v2_table_search_charged(
        measurements,
        table_index,
        limits,
        charge,
        Work {
            used: 0,
            maximum: maximum_work,
        },
    )
}
fn prepare_book_v2_table_search_charged<'m, 'f, 's, 'p, 'a>(
    measurements: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    limits: &M4EffectiveResourceLimits,
    charge: Charge,
    work: Work,
) -> Result<BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let flow = measurements.flow();
    flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)?;
    let root = NodeId::new(0);
    let table = measurements
        .tables()
        .get(table_index)
        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
    let owner = table.owner;
    let frames = flow
        .lines()
        .frames()
        .ok_or_else(|| error(owner, E::PendingRegion("body_frame")))?;
    let in_note = flow.collected.tables.tables[table_index]
        .definition
        .is_some();
    let region = if in_note {
        frames
            .footnote_region()
            .ok_or_else(|| error(owner, E::PendingRegion("footnote_frame")))?
    } else {
        frames.body()
    };
    let kernel = prepare_kernel(
        TableSearchInput {
            table,
            source: &flow.lines().prepared().source_flow().tables()[table_index],
            tables: measurements.tables(),
            items: &flow.collected.items,
            fingerprint: measurements.fingerprint(),
            fragment_algorithm: BOOK_V2_TABLE_FRAGMENT_ALGORITHM,
            parallel_breaks: true,
            minimum_fragment_height: frames.page_plan().and_then(|plan| {
                let minimum = if in_note {
                    plan.minimum_footnote_height()?
                } else {
                    plan.minimum_body_height()
                };
                if minimum >= region.height().get() {
                    return None;
                }
                Some(if in_note {
                    minimum
                        .checked_sub(
                            Length::from_raw(typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW)
                                .expect("positive separator"),
                        )
                        .expect("bounded source lengths")
                        .max(Length::ZERO)
                } else {
                    minimum
                })
            }),
            max_canonical_bytes: Some(limits.base().get().max_spool_bytes),
        },
        region.height().get(),
        limits.base().get().max_fragments,
        charge,
        work,
    )?;
    let mut search = BookV2TableBreakSearch {
        measurements,
        table_index,
        kernel,
        nested: None,
        header_measurements: Vec::new(),
        frame_catalog: None,
        frame_width: None,
    };
    if search.kernel.nested_body {
        search.prepare_nested(limits)?;
    }
    Ok(search)
}

#[path = "book_v2_definition_table_demand.rs"]
mod definition_demand;
pub use definition_demand::{
    prepare_book_v2_definition_table_demand_search, BookV2DefinitionTableDemandSearch,
    BookV2DefinitionTableDemandSelection, BookV2DefinitionTableDemandState,
};

#[path = "book_v2_definition_table_context.rs"]
mod definition_context;
pub(in crate::production_body::body_flow) use definition_context::BookV2DefinitionTableContext;

#[path = "book_v2_table_earlier_capacity.rs"]
mod earlier_capacity;
