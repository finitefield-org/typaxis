//! Capacity-dependent table fragments over the common measured cells.
//! These fragments carry no page index, repeated-header paint or PDF authority.
use super::*;
use std::ops::Range;
#[path = "production_table_kernel.rs"]
mod kernel;
use kernel::{prepare_kernel, TableBreakKernel, TablePosition, TableSearchInput};
#[cfg(feature = "book-v2-staging")]
#[path = "book_v2_table_breaks.rs"]
pub(in crate::production_body::body_flow) mod book_v2;

#[path = "production_table_body_context.rs"]
mod body_context;
#[path = "production_table_context_kernel.rs"]
mod context_kernel;
#[path = "production_table_footnotes.rs"]
mod footnotes;
pub(in crate::production_body::body_flow) use body_context::ProductionTableBodyContext;
pub use footnotes::{
    prepare_production_table_footnote_search, ProductionTableFootnoteSearch,
    ProductionTableFootnoteSelection, ProductionTableFootnoteState,
};

#[derive(Clone, Copy)]
struct Blocked {
    start: Length,
    end: Length,
    closed_end: bool,
}
struct Work {
    used: u64,
    maximum: u64,
}
impl Work {
    fn take(&mut self, count: u64, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.used = self
            .used
            .checked_add(count)
            .filter(|n| *n <= self.maximum)
            .ok_or_else(|| error(owner, E::TableSearchLimit))?;
        Ok(())
    }
}

// Fallible in-place ordering charges each comparison before performing it.
// Exhaustion stops immediately instead of finishing an unmetered library sort.
fn sort_blocked(
    values: &mut [Blocked],
    work: &mut Work,
    owner: NodeId,
) -> Result<(), ProductionBodyPaginationError> {
    fn sift(
        values: &mut [Blocked],
        mut root: usize,
        end: usize,
        work: &mut Work,
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        while root < end / 2 {
            let mut child = root * 2 + 1;
            if child + 1 < end {
                work.take(1, owner)?;
                if (values[child].start, values[child].end)
                    < (values[child + 1].start, values[child + 1].end)
                {
                    child += 1;
                }
            }
            work.take(1, owner)?;
            if (values[root].start, values[root].end) >= (values[child].start, values[child].end) {
                break;
            }
            values.swap(root, child);
            root = child;
        }
        Ok(())
    }
    for root in (0..values.len() / 2).rev() {
        sift(values, root, values.len(), work, owner)?;
    }
    for end in (1..values.len()).rev() {
        values.swap(0, end);
        sift(values, 0, end, work, owner)?;
    }
    Ok(())
}
#[derive(Clone, Copy)]
pub struct ProductionTableCursor<'m, 'f, 's, 'p, 'a> {
    measurements: &'m ProductionTableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    offset: Length,
    row: usize,
    initial: bool,
}
impl ProductionTableCursor<'_, '_, '_, '_, '_> {
    pub const fn table_index(&self) -> usize {
        self.table_index
    }
    /// Absolute table offset, including its source head rows.
    pub const fn offset(&self) -> Length {
        self.offset
    }
    pub const fn is_initial(&self) -> bool {
        self.initial
    }
    pub const fn next_row(&self) -> usize {
        self.row
    }
    pub fn is_terminal(&self) -> bool {
        !self.initial && self.row == self.measurements.tables[self.table_index].rows.len()
    }
}
#[derive(Clone, Debug)]
pub struct ProductionTableCellSlice {
    owner: NodeId,
    cell_index: usize,
    top: Length,
    height: Length,
    before: Length,
    after: Length,
    content: Range<usize>,
}
impl ProductionTableCellSlice {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn cell_index(&self) -> usize {
        self.cell_index
    }
    /// Relative fragment top, including the reserved header height.
    pub const fn top(&self) -> Length {
        self.top
    }
    pub const fn height(&self) -> Length {
        self.height
    }
    pub const fn offset_before(&self) -> Length {
        self.before
    }
    pub const fn offset_after(&self) -> Length {
        self.after
    }
    pub fn content_range(&self) -> Range<usize> {
        self.content.clone()
    }
}
pub struct ProductionTableFragmentSelection<'m, 'f, 's, 'p, 'a> {
    before: ProductionTableCursor<'m, 'f, 's, 'p, 'a>,
    after: ProductionTableCursor<'m, 'f, 's, 'p, 'a>,
    header_height: Length,
    available_height: Length,
    used_height: Length,
    cells: Vec<ProductionTableCellSlice>,
    fingerprint: [u8; 32],
}
impl<'m, 'f, 's, 'p, 'a> ProductionTableFragmentSelection<'m, 'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) fn placement_leaves(
        &self,
    ) -> impl Iterator<Item = Result<(NodeId, usize, Length, bool), ProductionBodyPaginationError>> + '_
    {
        let measured = self.before.measurements;
        let table = &measured.tables[self.before.table_index];
        let source = &measured.flow.lines.source_flow().tables()[self.before.table_index];
        selected_placement_leaves(
            table,
            source,
            &self.cells,
            self.repeats_header(),
            Some(Length::ZERO),
        )
    }

    pub(in crate::production_body::body_flow) fn verify_demand_flow(
        &self,
        flow: &ProductionPreparedBodyFlow<'_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let measured = self.before.measurements;
        if !std::ptr::eq(&measured.flow, flow)
            || measured.flow.collected.tables.tables[self.before.table_index]
                .definition
                .is_some()
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
    /// Exact selected cell ranges, in semantic source order. Header references
    /// occur only in the first fragment; later header paint is an artifact.
    pub(in crate::production_body::body_flow) fn semantic_leaf_ranges(
        &self,
    ) -> impl Iterator<Item = Range<usize>> + '_ {
        let measured = self.before.measurements;
        let table = &measured.tables[self.before.table_index];
        let source = &measured.flow.lines.source_flow().tables()[self.before.table_index];
        selected_semantic_leaf_ranges(table, source, &self.cells, self.before.initial)
    }
    pub const fn before(&self) -> ProductionTableCursor<'m, 'f, 's, 'p, 'a> {
        self.before
    }
    pub const fn after(&self) -> ProductionTableCursor<'m, 'f, 's, 'p, 'a> {
        self.after
    }
    pub const fn header_height(&self) -> Length {
        self.header_height
    }
    pub const fn used_height(&self) -> Length {
        self.used_height
    }
    pub const fn available_height(&self) -> Length {
        self.available_height
    }
    pub const fn repeats_header(&self) -> bool {
        !self.before.initial
    }
    pub fn cells(&self) -> &[ProductionTableCellSlice] {
        &self.cells
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}
pub struct ProductionTableBreakSearch<'m, 'f, 's, 'p, 'a> {
    measurements: &'m ProductionTableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    kernel: TableBreakKernel<'m>,
}
impl<'m, 'f, 's, 'p, 'a> ProductionTableBreakSearch<'m, 'f, 's, 'p, 'a> {
    pub const fn header_height(&self) -> Length {
        self.kernel.header_height
    }
    pub const fn maximum_height(&self) -> Length {
        self.kernel.maximum_height
    }
    pub const fn work_charge(&self) -> u64 {
        self.kernel.work.used
    }
    pub fn record_charge(&self) -> u64 {
        self.kernel.maximum_records - self.kernel.charge.remaining
    }
    pub fn begin(
        &mut self,
    ) -> Result<ProductionTableCursor<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.kernel
            .charge
            .take(1, self.measurements.tables[self.table_index].owner)?;
        Ok(ProductionTableCursor {
            measurements: self.measurements,
            table_index: self.table_index,
            offset: self.kernel.header_height,
            row: self.kernel.header_rows,
            initial: true,
        })
    }
    /// The greatest shared cell boundary within this capacity. Every fragment
    /// reserves the head again; the eventual page owner distinguishes its first
    /// semantic occurrence from repeated header artifacts.
    pub fn evaluate(
        &mut self,
        cursor: &ProductionTableCursor<'_, '_, '_, '_, '_>,
        available: Length,
    ) -> Result<
        Option<ProductionTableFragmentSelection<'m, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        if !std::ptr::eq(cursor.measurements, self.measurements)
            || cursor.table_index != self.table_index
        {
            return Err(error(
                self.measurements.tables[self.table_index].owner,
                E::ReceiptMismatch,
            ));
        }
        let Some(result) = self.kernel.evaluate(
            &TablePosition {
                offset: cursor.offset,
                row: cursor.row,
                initial: cursor.initial,
                header_seen: !cursor.initial,
                caption_next: 0,
                cells: None,
                cell_fingerprint: [0; 32],
            },
            available,
        )?
        else {
            return Ok(None);
        };
        let before = ProductionTableCursor {
            measurements: self.measurements,
            table_index: self.table_index,
            offset: cursor.offset,
            row: cursor.row,
            initial: cursor.initial,
        };
        let after = ProductionTableCursor {
            offset: result.after.offset,
            row: result.after.row,
            initial: result.after.initial,
            ..before
        };
        Ok(Some(ProductionTableFragmentSelection {
            before,
            after,
            header_height: result.header_height,
            available_height: result.available_height,
            used_height: result.used_height,
            cells: result.cells,
            fingerprint: result.fingerprint,
        }))
    }
}

pub fn prepare_production_table_search<'m, 'f, 's, 'p, 'a>(
    measurements: &'m ProductionTableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
) -> Result<ProductionTableBreakSearch<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let charge = Charge {
        remaining: limits
            .base()
            .get()
            .max_fragments
            .checked_sub(measurements.record_charge)
            .ok_or_else(|| error(NodeId::new(0), E::FragmentLimit))?,
    };
    prepare_table_search_charged(
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
fn prepare_table_search_charged<'m, 'f, 's, 'p, 'a>(
    measurements: &'m ProductionTableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    limits: &M4EffectiveResourceLimits,
    charge: Charge,
    work: Work,
) -> Result<ProductionTableBreakSearch<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    measurements.verify(measurements.flow.lines, measurements.flow.blocks, limits)?;
    let table = measurements
        .tables
        .get(table_index)
        .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
    let source = &measurements.flow.lines.source_flow().tables()[table_index];
    let owner = table.owner;
    let in_note = measurements.flow.collected.tables.tables[table_index]
        .definition
        .is_some();
    let maximum_height = if in_note {
        measurements
            .flow
            .footnote_region()
            .ok_or_else(|| error(owner, E::PendingRegion("footnote_frame")))?
            .height()
            .get()
    } else {
        measurements
            .flow
            .blocks
            .page_geometry()
            .body()
            .height()
            .get()
    };
    let kernel = prepare_kernel(
        TableSearchInput {
            table,
            source,
            tables: &measurements.tables,
            items: &measurements.flow.collected.items,
            fingerprint: measurements.fingerprint,
            fragment_algorithm: "typaxis.production-table-fragment/1",
            parallel_breaks: false,
            minimum_fragment_height: None,
            max_canonical_bytes: None,
        },
        maximum_height,
        limits.base().get().max_fragments,
        charge,
        work,
    )?;
    Ok(ProductionTableBreakSearch {
        measurements,
        table_index,
        kernel,
    })
}

fn selected_placement_leaves<'t>(
    table: &'t ProductionMeasuredTable,
    source: &'t typaxis_syntax::ProductionTable,
    selected_cells: &'t [ProductionTableCellSlice],
    repeats_header: bool,
    header_top: Option<Length>,
) -> impl Iterator<Item = Result<(NodeId, usize, Length, bool), ProductionBodyPaginationError>> + 't
{
    let head = table
        .cells
        .iter()
        .zip(source.cells())
        .take_while(move |(_, cell)| {
            header_top.is_some()
                && table.rows[cell.row() as usize].section == ProductionTableSection::Head
        })
        .map(move |(cell, binding)| {
            (
                cell,
                table.rows[binding.row() as usize].top,
                0..cell.content.len(),
                repeats_header,
            )
        });
    let body = selected_cells.iter().map(move |slice| {
        (
            &table.cells[slice.cell_index],
            slice.top,
            slice.content.clone(),
            false,
            slice.before,
            false,
        )
    });
    head.map(|(cell, top, range, repeated)| (cell, top, range, repeated, Length::ZERO, true))
        .chain(body)
        .flat_map(move |(cell, origin, range, repeated, offset, header)| {
            cell.content[range].iter().map(move |content| {
                let ProductionTableContentSource::FlowItem(index) = content.source else {
                    return Err(error(cell.owner, E::ReceiptMismatch));
                };
                let origin = if header {
                    add(
                        origin
                            .checked_sub(table.caption.as_ref().map_or(Length::ZERO, |c| c.height))
                            .ok_or_else(|| error(cell.owner, E::ArithmeticOverflow))?,
                        header_top.ok_or_else(|| error(cell.owner, E::ReceiptMismatch))?,
                        cell.owner,
                    )?
                } else {
                    origin
                };
                let relative = content
                    .top
                    .checked_sub(offset)
                    .ok_or_else(|| error(cell.owner, E::ArithmeticOverflow))?;
                Ok((
                    cell.owner,
                    index,
                    add(origin, relative, cell.owner)?,
                    repeated,
                ))
            })
        })
}

fn selected_semantic_leaf_ranges<'t>(
    table: &'t ProductionMeasuredTable,
    source: &'t typaxis_syntax::ProductionTable,
    selected_cells: &'t [ProductionTableCellSlice],
    initial: bool,
) -> impl Iterator<Item = Range<usize>> + 't {
    let head = table
        .cells
        .iter()
        .zip(source.cells())
        .take_while(move |(_, source)| {
            initial && table.rows[source.row() as usize].section == ProductionTableSection::Head
        })
        .map(|(cell, _)| cell.content.as_slice());
    let body = selected_cells
        .iter()
        .map(|slice| &table.cells[slice.cell_index].content[slice.content.clone()]);
    head.chain(body).filter_map(|content| {
        let ProductionTableContentSource::FlowItem(first) = content.first()?.source else {
            unreachable!("table search rejects nested contents before issuing a selection")
        };
        let ProductionTableContentSource::FlowItem(last) = content.last()?.source else {
            unreachable!("table search rejects nested contents before issuing a selection")
        };
        Some(first..last + 1)
    })
}
