//! Shared capacity search over actual table/cell measurements. Versioned
//! wrappers bind exact source owners before invoking this private kernel.
use super::*;
#[cfg(feature = "book-v2-staging")]
#[path = "production_table_parallel_breaks.rs"]
mod parallel;
#[cfg(feature = "book-v2-staging")]
use parallel::CellContinuation;

#[derive(Clone, Copy)]
pub(super) struct TablePosition {
    pub offset: Length,
    pub row: usize,
    pub initial: bool,
    pub header_seen: bool,
    pub caption_next: usize,
    pub cells: Option<usize>,
    pub cell_fingerprint: [u8; 32],
}
pub(super) struct TableFragmentProjection {
    pub after: TablePosition,
    pub header_height: Length,
    pub available_height: Length,
    pub used_height: Length,
    pub cells: Vec<ProductionTableCellSlice>,
    #[cfg(feature = "book-v2-staging")]
    pub break_items: Vec<usize>,
    #[cfg(feature = "book-v2-staging")]
    pub header_source_slices: bool,
    #[cfg(feature = "book-v2-staging")]
    pub caption: std::ops::Range<usize>,
    #[cfg(feature = "book-v2-staging")]
    pub header_top: Option<Length>,
    #[cfg(feature = "book-v2-staging")]
    pub forced_break: Option<NodeId>,
    pub fingerprint: [u8; 32],
}
pub(super) struct TableSearchInput<'m> {
    pub table: &'m ProductionMeasuredTable,
    pub source: &'m typaxis_syntax::ProductionTable,
    pub tables: &'m [ProductionMeasuredTable],
    pub items: &'m [Item],
    pub fingerprint: [u8; 32],
    pub fragment_algorithm: &'static str,
    pub max_canonical_bytes: Option<u64>,
    pub parallel_breaks: bool,
    pub minimum_fragment_height: Option<Length>,
}
pub(super) struct TableBreakKernel<'m> {
    input: TableSearchInput<'m>,
    pub(super) header_rows: usize,
    pub(super) header_height: Length,
    pub(super) repeated_header_height: Option<Length>,
    pub(super) maximum_height: Length,
    pub(super) blocked: Vec<Blocked>,
    caption_breaks: Vec<usize>,
    pub(super) cell_breaks: bool,
    pub(super) header_breaks: bool,
    pub(super) nested_body: bool,
    #[cfg(feature = "book-v2-staging")]
    pub(super) spanning_breaks: bool,
    #[cfg(feature = "book-v2-staging")]
    cell_states: Vec<CellContinuation>,
    pub(super) max_ends: Vec<Length>,
    pub(super) tree_base: usize,
    pub(super) charge: Charge,
    pub(super) maximum_records: u64,
    pub(super) work: Work,
}
impl TableBreakKernel<'_> {
    /// Physical reservation may vary after the original header was consumed.
    /// Source offsets and row bands continue to use `header_height`.
    pub(super) fn paint_header_height(&self) -> Length {
        self.repeated_header_height.unwrap_or(self.header_height)
    }

    /// Step between semantic/row events rather than raw padding units.
    pub(super) fn earlier_capacity(
        &mut self,
        cursor_offset: Length,
        end: Length,
    ) -> Result<Option<Length>, ProductionBodyPaginationError> {
        let table = self.input.table;
        let owner = table.owner;
        self.work.take(
            2 * (u64::from(self.blocked.len().checked_ilog2().unwrap_or(0)) + 1)
                + u64::from(table.rows.len().checked_ilog2().unwrap_or(0))
                + 1,
            owner,
        )?;
        let starts = self.blocked.partition_point(|b| b.start < end);
        let ends = self.blocked.partition_point(|b| b.end < end);
        let rows = table.rows.partition_point(|r| r.top < end);
        let mut previous = cursor_offset;
        if starts > 0 {
            previous = previous.max(self.blocked[starts - 1].start);
        }
        if ends > 0 {
            previous = previous.max(self.blocked[ends - 1].end);
        }
        if rows > 0 {
            previous = previous.max(table.rows[rows - 1].top);
        }
        if previous <= cursor_offset {
            return Ok(None);
        }
        Ok(Some(add(
            self.paint_header_height(),
            previous
                .checked_sub(cursor_offset)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
            owner,
        )?))
    }
    #[cfg(feature = "book-v2-staging")]
    pub(super) fn earlier_capacity_for(
        &mut self,
        cursor: &TablePosition,
        end: Length,
    ) -> Result<Option<Length>, ProductionBodyPaginationError> {
        let result = self.earlier_capacity(cursor.offset, end)?;
        if self.input.table.caption.is_some() && !cursor.header_seen {
            result
                .map(|n| {
                    n.checked_sub(self.header_height)
                        .ok_or_else(|| error(self.input.table.owner, E::ArithmeticOverflow))
                })
                .transpose()
        } else {
            Ok(result)
        }
    }
    pub(super) fn evaluate(
        &mut self,
        cursor: &TablePosition,
        available: Length,
    ) -> Result<Option<TableFragmentProjection>, ProductionBodyPaginationError> {
        if self.nested_body {
            return Err(error(
                self.input.table.owner,
                E::PendingRegion("nested_table_breaks"),
            ));
        }
        #[cfg(feature = "book-v2-staging")]
        if self.cell_breaks {
            return self.evaluate_parallel(cursor, available);
        }
        if cursor.cells.is_some() {
            return Err(error(self.input.table.owner, E::ReceiptMismatch));
        }
        if self.input.table.caption.is_some() {
            return self.evaluate_caption(cursor, available);
        }
        let table = self.input.table;
        let owner = table.owner;
        if (!cursor.initial && cursor.row == table.rows.len())
            || cursor.offset < self.header_height
            || cursor.offset > table.height
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        if available < Length::ZERO || available > self.maximum_height {
            return Err(error(owner, E::InvalidTableCapacity));
        }
        self.charge.take(1, owner)?;
        self.work.take(1, owner)?;
        let header_height = self.paint_header_height();
        if available < header_height {
            return Ok(None);
        }
        let capacity = available
            .checked_sub(header_height)
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        let remaining = table
            .height
            .checked_sub(cursor.offset)
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        let end = add(cursor.offset, capacity.min(remaining), owner)?;
        let end = self.previous_boundary(end)?;
        if end <= cursor.offset && cursor.offset != table.height {
            if available == self.maximum_height {
                return Err(error(owner, E::Oversize));
            }
            return Ok(None);
        }
        self.work.take(
            u64::from(table.rows.len().checked_ilog2().unwrap_or(0)) + 1,
            owner,
        )?;
        let row = table
            .rows
            .partition_point(|r| r.top.checked_add(r.height).is_some_and(|y| y <= end));
        if row < cursor.row || (end == cursor.offset && row == cursor.row && !cursor.initial) {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let after = TablePosition {
            offset: end,
            row,
            initial: false,
            header_seen: true,
            caption_next: 0,
            cells: None,
            cell_fingerprint: [0; 32],
        };
        let mut cells = Vec::new();
        self.select_cells(
            1,
            0,
            self.tree_base,
            cursor.offset,
            end,
            header_height,
            &mut cells,
        )?;
        let used_height = add(
            header_height,
            end.checked_sub(cursor.offset)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
            owner,
        )?;
        self.project(
            cursor,
            after,
            available,
            used_height,
            Some(Length::ZERO),
            0..0,
            None,
            Vec::new(),
            cells,
        )
    }
    pub(super) fn project(
        &mut self,
        cursor: &TablePosition,
        after: TablePosition,
        available: Length,
        used_height: Length,
        header_top: Option<Length>,
        caption: std::ops::Range<usize>,
        forced_break: Option<NodeId>,
        break_items: Vec<usize>,
        cells: Vec<ProductionTableCellSlice>,
    ) -> Result<Option<TableFragmentProjection>, ProductionBodyPaginationError> {
        let table = self.input.table;
        let owner = table.owner;
        self.charge
            .take(if table.caption.is_some() { 3 } else { 2 }, owner)?;
        let canonical_capacity = cells
            .len()
            .checked_mul(72)
            .and_then(|n| n.checked_add(if table.caption.is_some() { 192 } else { 128 }))
            .and_then(|n| {
                n.checked_add(if self.caption_breaks.is_empty() {
                    0
                } else {
                    32
                })
            })
            .and_then(|n| {
                if self.cell_breaks {
                    n.checked_add(112)?
                        .checked_add(break_items.len().checked_mul(8)?)
                } else {
                    Some(n)
                }
            })
            .and_then(|n| n.checked_add(if self.header_breaks { 8 } else { 0 }))
            .ok_or_else(|| error(owner, E::FragmentLimit))?;
        if self
            .input
            .max_canonical_bytes
            .is_some_and(|limit| canonical_capacity as u64 > limit)
        {
            return Err(error(owner, E::SpoolLimit));
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(canonical_capacity)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        bytes.extend_from_slice(&sha256(self.input.fragment_algorithm.as_bytes()));
        bytes.extend_from_slice(&self.input.fingerprint);
        bytes.extend_from_slice(&owner.get().to_be_bytes());
        bytes.push(u8::from(cursor.initial));
        bytes.extend_from_slice(&(cursor.row as u64).to_be_bytes());
        bytes.extend_from_slice(&(after.row as u64).to_be_bytes());
        bytes.extend_from_slice(&available.raw().to_be_bytes());
        for n in [cursor.offset, after.offset, self.header_height] {
            bytes.extend_from_slice(&n.raw().to_be_bytes());
        }
        if table.caption.is_some() {
            bytes.push(u8::from(cursor.header_seen));
            bytes.push(u8::from(after.header_seen));
            bytes.push(u8::from(header_top.is_some()));
            bytes.extend_from_slice(&header_top.unwrap_or(Length::ZERO).raw().to_be_bytes());
            bytes.extend_from_slice(&(caption.start as u64).to_be_bytes());
            bytes.extend_from_slice(&(caption.end as u64).to_be_bytes());
            bytes.extend_from_slice(&used_height.raw().to_be_bytes());
        }
        if self.header_breaks {
            bytes.extend_from_slice(b"HEADBRK1");
        }
        if !self.caption_breaks.is_empty() {
            bytes.extend_from_slice(b"CAPBRK01");
            bytes.extend_from_slice(&(cursor.caption_next as u64).to_be_bytes());
            bytes.extend_from_slice(&(after.caption_next as u64).to_be_bytes());
            bytes.extend_from_slice(&forced_break.map_or(0, |owner| owner.get()).to_be_bytes());
        }
        if self.cell_breaks {
            bytes.extend_from_slice(b"CELLBRK1");
            bytes.push(u8::from(cursor.header_seen));
            bytes.push(u8::from(after.header_seen));
            bytes.push(u8::from(header_top.is_some()));
            bytes.extend_from_slice(&header_top.unwrap_or(Length::ZERO).raw().to_be_bytes());
            bytes.extend_from_slice(&cursor.cell_fingerprint);
            bytes.extend_from_slice(&after.cell_fingerprint);
            bytes.extend_from_slice(&used_height.raw().to_be_bytes());
            bytes.extend_from_slice(&(break_items.len() as u64).to_be_bytes());
            for item in &break_items {
                bytes.extend_from_slice(&(*item as u64).to_be_bytes());
            }
        }
        for cell in &cells {
            bytes.extend_from_slice(&cell.owner.get().to_be_bytes());
            for n in [cell.top, cell.height, cell.before, cell.after] {
                bytes.extend_from_slice(&n.raw().to_be_bytes());
            }
            bytes.extend_from_slice(&(cell.content.start as u64).to_be_bytes());
            bytes.extend_from_slice(&(cell.content.end as u64).to_be_bytes());
        }
        Ok(Some(TableFragmentProjection {
            after,
            header_height: header_top.map_or(Length::ZERO, |_| self.paint_header_height()),
            available_height: available,
            used_height,
            cells,
            #[cfg(feature = "book-v2-staging")]
            break_items,
            #[cfg(feature = "book-v2-staging")]
            header_source_slices: self.header_breaks,
            #[cfg(feature = "book-v2-staging")]
            caption,
            #[cfg(feature = "book-v2-staging")]
            header_top,
            #[cfg(feature = "book-v2-staging")]
            forced_break,
            fingerprint: sha256(&bytes),
        }))
    }
    fn evaluate_caption(
        &mut self,
        cursor: &TablePosition,
        available: Length,
    ) -> Result<Option<TableFragmentProjection>, ProductionBodyPaginationError> {
        let table = self.input.table;
        let owner = table.owner;
        let caption = table
            .caption
            .as_ref()
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let rows_start = add(caption.height, self.header_height, owner)?;
        if (!cursor.initial && cursor.header_seen && cursor.row == table.rows.len())
            || cursor.caption_next > caption.content.len()
            || cursor.offset < Length::ZERO
            || cursor.offset > table.height
            || (cursor.header_seen && cursor.offset < rows_start)
            || (!cursor.header_seen && cursor.offset > caption.height)
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        if available < Length::ZERO || available > self.maximum_height {
            return Err(error(owner, E::InvalidTableCapacity));
        }
        self.charge.take(1, owner)?;
        self.work.take(1, owner)?;
        let repeated_height = if cursor.header_seen {
            self.paint_header_height()
        } else {
            Length::ZERO
        };
        if available < repeated_height {
            return Ok(None);
        }
        let capacity = available
            .checked_sub(repeated_height)
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        let remaining = table
            .height
            .checked_sub(cursor.offset)
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        let mut proposed = add(cursor.offset, capacity.min(remaining), owner)?;
        if self.cell_breaks {
            proposed = proposed.min(caption.height);
        }
        let next_break = if self.caption_breaks.is_empty() {
            None
        } else {
            self.work.take(
                u64::from(self.caption_breaks.len().checked_ilog2().unwrap_or(0)) + 1,
                owner,
            )?;
            self.caption_breaks
                .get(
                    self.caption_breaks
                        .partition_point(|i| *i < cursor.caption_next),
                )
                .copied()
        };
        if let Some(index) = next_break {
            proposed = proposed.min(caption.content[index].end);
        }
        // A complete caption may tentatively join the row prefix even when its
        // final paragraph keeps with that prefix. The parallel path rolls this
        // choice back if no body item can start on the same page.
        let end = if self.cell_breaks && proposed == caption.height {
            proposed
        } else {
            self.previous_boundary(proposed)?
        };
        let forced_index = next_break.filter(|index| caption.content[*index].end == end);
        let forced_break = forced_index.map(|index| {
            let ProductionTableContentSource::FlowItem(item) = caption.content[index].source else {
                unreachable!("validated caption break")
            };
            self.input.items[item].owner
        });
        if end <= cursor.offset && cursor.offset != table.height && forced_break.is_none() {
            if available == self.maximum_height {
                return Err(error(owner, E::Oversize));
            }
            return Ok(None);
        }
        let rows_present = !self.cell_breaks
            && forced_break.is_none()
            && end >= rows_start
            && (end > rows_start || end == table.height);
        if !rows_present && end > caption.height {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let header_top = if rows_present {
            Some(if cursor.header_seen {
                Length::ZERO
            } else {
                caption
                    .height
                    .checked_sub(cursor.offset)
                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?
            })
        } else {
            None
        };
        self.work.take(
            u64::from(table.rows.len().checked_ilog2().unwrap_or(0))
                + 1
                + 2 * (u64::from(caption.content.len().checked_ilog2().unwrap_or(0)) + 1),
            owner,
        )?;
        let row = if rows_present {
            table
                .rows
                .partition_point(|r| r.top.checked_add(r.height).is_some_and(|y| y <= end))
        } else {
            0
        };
        let begin = cursor.caption_next;
        let finish = forced_index.map_or_else(
            || {
                caption
                    .content
                    .partition_point(|c| c.end <= end.min(caption.height))
            },
            |index| index + 1,
        );
        if finish < begin || row < cursor.row {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let mut cells = Vec::new();
        if let Some(top) = header_top {
            self.select_cells(
                1,
                0,
                self.tree_base,
                cursor.offset.max(rows_start),
                end,
                add(top, self.paint_header_height(), owner)?,
                &mut cells,
            )?;
        }
        let after = TablePosition {
            offset: end,
            row,
            initial: false,
            header_seen: cursor.header_seen || rows_present,
            caption_next: finish,
            cells: cursor.cells,
            cell_fingerprint: cursor.cell_fingerprint,
        };
        let used_height = add(
            repeated_height,
            end.checked_sub(cursor.offset)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
            owner,
        )?;
        self.project(
            cursor,
            after,
            available,
            used_height,
            header_top,
            begin..finish,
            forced_break,
            Vec::new(),
            cells,
        )
    }
    fn previous_boundary(
        &mut self,
        mut end: Length,
    ) -> Result<Length, ProductionBodyPaginationError> {
        let owner = self.input.table.owner;
        loop {
            self.work.take(
                u64::from(self.blocked.len().checked_ilog2().unwrap_or(0)) + 1,
                owner,
            )?;
            let index = self.blocked.partition_point(|b| b.start < end);
            let Some(block) = index
                .checked_sub(1)
                .and_then(|i| self.blocked.get(i))
                .copied()
            else {
                return Ok(end);
            };
            if end < block.end || (end == block.end && block.closed_end) {
                end = block.start;
            } else {
                return Ok(end);
            }
        }
    }
    fn select_cells(
        &mut self,
        node: usize,
        first: usize,
        last: usize,
        start: Length,
        end: Length,
        body_top: Length,
        output: &mut Vec<ProductionTableCellSlice>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let table = self.input.table;
        let source = self.input.source;
        self.work.take(1, table.owner)?;
        if first >= source.cells().len()
            || self.max_ends[node] <= start
            || table.rows[source.cells()[first].row() as usize].top >= end
        {
            return Ok(());
        }
        if last - first > 1 {
            let middle = first + (last - first) / 2;
            self.select_cells(node * 2, first, middle, start, end, body_top, output)?;
            return self.select_cells(node * 2 + 1, middle, last, start, end, body_top, output);
        }
        let binding = &source.cells()[first];
        let cell = &table.cells[first];
        let origin = table.rows[binding.row() as usize].top;
        let top = origin.max(start);
        let bottom = self.max_ends[node].min(end);
        let before = top
            .checked_sub(origin)
            .ok_or_else(|| error(cell.owner, E::ArithmeticOverflow))?;
        let after = bottom
            .checked_sub(origin)
            .ok_or_else(|| error(cell.owner, E::ArithmeticOverflow))?;
        self.work.take(
            2 * (u64::from(cell.content.len().checked_ilog2().unwrap_or(0)) + 1),
            cell.owner,
        )?;
        let begin = cell.content.partition_point(|c| c.end <= before);
        let finish = cell.content.partition_point(|c| c.end <= after);
        if finish < cell.content.len() {
            let item_start = if finish == 0 {
                Length::ZERO
            } else {
                cell.content[finish - 1].end
            };
            if item_start < after {
                return Err(error(cell.owner, E::ReceiptMismatch));
            }
        }
        if begin < cell.content.len() {
            let item_start = if begin == 0 {
                Length::ZERO
            } else {
                cell.content[begin - 1].end
            };
            if item_start < before && before < cell.natural_height {
                return Err(error(cell.owner, E::ReceiptMismatch));
            }
        }
        self.charge.take(1, cell.owner)?;
        output
            .try_reserve(1)
            .map_err(|_| error(cell.owner, E::AllocationFailure))?;
        output.push(ProductionTableCellSlice {
            owner: cell.owner,
            cell_index: first,
            top: add(
                body_top,
                top.checked_sub(start)
                    .ok_or_else(|| error(cell.owner, E::ArithmeticOverflow))?,
                cell.owner,
            )?,
            height: bottom
                .checked_sub(top)
                .ok_or_else(|| error(cell.owner, E::ArithmeticOverflow))?,
            before,
            after,
            content: begin..finish,
        });
        Ok(())
    }
}

pub(super) fn prepare_kernel<'m>(
    input: TableSearchInput<'m>,
    maximum_height: Length,
    maximum_records: u64,
    mut charge: Charge,
    mut work: Work,
) -> Result<TableBreakKernel<'m>, ProductionBodyPaginationError> {
    let table = input.table;
    let source = input.source;
    let owner = table.owner;
    work.take(
        u64::from(table.rows.len().checked_ilog2().unwrap_or(0)) + 1,
        owner,
    )?;
    let header_rows = table
        .rows
        .partition_point(|r| r.section == ProductionTableSection::Head);
    let caption_height = table.caption.as_ref().map_or(Length::ZERO, |c| c.height);
    let header_end = table.rows.get(header_rows).map_or(table.height, |r| r.top);
    let header_height = header_end
        .checked_sub(caption_height)
        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
    if header_height > maximum_height {
        return Err(error(owner, E::TableHeaderOversize));
    }
    let mut blocked: Vec<Blocked> = Vec::new();
    let mut caption_breaks = Vec::new();
    let mut cell_breaks = false;
    let mut header_breaks = false;
    let mut nested_body = false;
    let mut has_spans = None;
    if let Some(caption) = &table.caption {
        let mut previous = Length::ZERO;
        for (index, content) in caption.content.iter().enumerate() {
            work.take(1, owner)?;
            let item_index = match content.source {
                ProductionTableContentSource::FlowItem(index) => index,
                ProductionTableContentSource::Table(index) => {
                    let child = input
                        .tables
                        .get(index)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if input.parallel_breaks {
                        work.take(source.cells().len() as u64, owner)?;
                        has_spans = Some(source.cells().iter().any(|c| c.rowspan().get() != 1));
                        nested_body = true;
                        cell_breaks = true;
                        previous = content.end;
                        continue;
                    }
                    return Err(error(child.owner, E::PendingRegion("nested_table_breaks")));
                }
            };
            let item = input
                .items
                .get(item_index)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            if item.source.is_none() {
                if table.keep_together {
                    return Err(error(owner, E::KeepAcrossForcedBreak));
                }
                if index > 0 {
                    let prior = match caption.content[index - 1].source {
                        ProductionTableContentSource::FlowItem(i) => {
                            input.items[i].keep.then_some(input.items[i].owner)
                        }
                        ProductionTableContentSource::Table(i) => {
                            input.tables[i].keep.then_some(input.tables[i].owner)
                        }
                    };
                    if let Some(owner) = prior {
                        return Err(error(owner, E::KeepAcrossForcedBreak));
                    }
                }
                charge.take(1, item.owner)?;
                caption_breaks
                    .try_reserve(1)
                    .map_err(|_| error(item.owner, E::AllocationFailure))?;
                caption_breaks.push(index);
                previous = content.end;
                continue;
            }
            charge.take(1, item.owner)?;
            blocked
                .try_reserve(1)
                .map_err(|_| error(item.owner, E::AllocationFailure))?;
            blocked.push(Blocked {
                start: previous,
                end: content.end,
                closed_end: item.keep
                    && (index + 1 < caption.content.len() || table.height > caption.height),
            });
            previous = content.end;
        }
        // A header cannot be broken internally or orphaned before its body.
        if header_height > Length::ZERO {
            charge.take(1, owner)?;
            blocked
                .try_reserve(1)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            blocked.push(Blocked {
                start: caption.height,
                end: header_end,
                closed_end: header_end < table.height,
            });
        }
    }
    for (binding, cell) in source.cells().iter().zip(&table.cells) {
        let origin = table.rows[binding.row() as usize].top;
        let mut previous = Length::ZERO;
        for (index, content) in cell.content.iter().enumerate() {
            work.take(1, cell.owner)?;
            let item_index = match content.source {
                ProductionTableContentSource::FlowItem(index) => index,
                ProductionTableContentSource::Table(index) => {
                    let child = input
                        .tables
                        .get(index)
                        .ok_or_else(|| error(cell.owner, E::ReceiptMismatch))?;
                    if input.parallel_breaks {
                        if !nested_body {
                            work.take(source.cells().len() as u64, owner)?;
                            has_spans = Some(source.cells().iter().any(|c| c.rowspan().get() != 1));
                        }
                        nested_body = true;
                        cell_breaks = true;
                        previous = content.end;
                        continue;
                    }
                    return Err(error(child.owner, E::PendingRegion("nested_table_breaks")));
                }
            };
            let item = input
                .items
                .get(item_index)
                .ok_or_else(|| error(cell.owner, E::ReceiptMismatch))?;
            if item.source.is_none() {
                if !input.parallel_breaks {
                    return Err(error(item.owner, E::PendingRegion("table_forced_break")));
                }
                if has_spans.is_none() {
                    work.take(source.cells().len() as u64, owner)?;
                    has_spans = Some(source.cells().iter().any(|c| c.rowspan().get() != 1));
                }
                if table.keep_together {
                    return Err(error(owner, E::KeepAcrossForcedBreak));
                }
                if index > 0 {
                    let prior = match cell.content[index - 1].source {
                        ProductionTableContentSource::FlowItem(i) => {
                            input.items[i].keep.then_some(input.items[i].owner)
                        }
                        ProductionTableContentSource::Table(i) => {
                            input.tables[i].keep.then_some(input.tables[i].owner)
                        }
                    };
                    if let Some(owner) = prior {
                        return Err(error(owner, E::KeepAcrossForcedBreak));
                    }
                }
                cell_breaks = true;
                header_breaks |=
                    table.rows[binding.row() as usize].section == ProductionTableSection::Head;
                previous = content.end;
                continue;
            }
            charge.take(1, cell.owner)?;
            blocked
                .try_reserve(1)
                .map_err(|_| error(cell.owner, E::AllocationFailure))?;
            blocked.push(Blocked {
                start: add(origin, previous, cell.owner)?,
                end: add(origin, content.end, cell.owner)?,
                closed_end: item.keep && index + 1 < cell.content.len(),
            });
            previous = content.end;
        }
    }
    if table.keep_together {
        charge.take(1, owner)?;
        blocked
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        blocked.push(Blocked {
            start: Length::ZERO,
            end: table.height,
            closed_end: false,
        });
    }
    sort_blocked(&mut blocked, &mut work, owner)?;
    let mut retained = 0usize;
    for index in 0..blocked.len() {
        work.take(1, owner)?;
        let next = blocked[index];
        if retained != 0 {
            let last = &mut blocked[retained - 1];
            if next.start < last.end || (next.start == last.end && last.closed_end) {
                if next.end > last.end {
                    last.end = next.end;
                    last.closed_end = next.closed_end;
                } else if next.end == last.end {
                    last.closed_end |= next.closed_end;
                }
                continue;
            }
        }
        blocked[retained] = next;
        retained += 1;
    }
    blocked.truncate(retained);
    // Different cell line heights can merge every common vertical cut across
    // an entire row. In the successor, retain independent source positions if
    // such a gap cannot fit the smallest reachable empty-page capacity. Keep
    // the common-cut path where it fits, and the frozen profile's refusal policy.
    if input.parallel_breaks && !cell_breaks && !table.keep_together {
        let body_capacity = input
            .minimum_fragment_height
            .unwrap_or(maximum_height)
            .checked_sub(header_height)
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?
            .max(Length::ZERO);
        for interval in &blocked {
            work.take(1, owner)?;
            if interval.end <= header_end {
                continue;
            }
            let gap = interval
                .end
                .checked_sub(interval.start.max(header_end))
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            if gap > body_capacity || (gap == body_capacity && interval.closed_end) {
                work.take(source.cells().len() as u64, owner)?;
                has_spans = Some(source.cells().iter().any(|c| c.rowspan().get() != 1));
                cell_breaks = true;
                break;
            }
        }
    }
    let tree_base = source
        .cells()
        .len()
        .checked_next_power_of_two()
        .ok_or_else(|| error(owner, E::FragmentLimit))?;
    let nodes = tree_base
        .checked_mul(2)
        .ok_or_else(|| error(owner, E::FragmentLimit))?;
    charge.take(
        nodes
            .checked_add(1)
            .ok_or_else(|| error(owner, E::FragmentLimit))?,
        owner,
    )?;
    work.take(nodes as u64, owner)?;
    let mut max_ends = Vec::new();
    max_ends
        .try_reserve_exact(nodes)
        .map_err(|_| error(owner, E::AllocationFailure))?;
    max_ends.resize(nodes, Length::ZERO);
    for (index, cell) in source.cells().iter().enumerate() {
        let end_row = cell.row() as usize + usize::from(cell.rowspan().get());
        max_ends[tree_base + index] = table.rows.get(end_row).map_or(table.height, |r| r.top);
    }
    for index in (1..tree_base).rev() {
        max_ends[index] = max_ends[index * 2].max(max_ends[index * 2 + 1]);
    }
    Ok(TableBreakKernel {
        input,
        header_rows,
        header_height,
        repeated_header_height: None,
        maximum_height,
        blocked,
        caption_breaks,
        cell_breaks,
        header_breaks,
        nested_body,
        #[cfg(feature = "book-v2-staging")]
        spanning_breaks: has_spans.unwrap_or(false),
        #[cfg(feature = "book-v2-staging")]
        cell_states: Vec::new(),
        max_ends,
        tree_base,
        charge,
        maximum_records,
        work,
    })
}
