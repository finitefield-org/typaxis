//! Child table selections inside independent original parent cells.
//! Serial captions and parallel body cells retain separate original cursors.
//! Spanning parent rows retain their measured remaining band separately.
use super::*;
#[path = "book_v2_nested_table_headers.rs"]
mod headers;
#[path = "book_v2_nested_table_keeps.rs"]
mod keeps;
#[path = "book_v2_nested_table_spans.rs"]
mod spans;
use keeps::Checkpoint;

#[derive(Clone, Copy)]
struct Cell<'m, 'f, 's, 'p, 'a> {
    next: usize,
    child: Option<BookV2TableCursor<'m, 'f, 's, 'p, 'a>>,
}
struct State<'m, 'f, 's, 'p, 'a> {
    cells: Vec<Cell<'m, 'f, 's, 'p, 'a>>,
    fingerprint: [u8; 32],
    remaining: Option<Length>,
}
pub(super) struct Search<'m, 'f, 's, 'p, 'a> {
    children: Vec<BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a>>,
    max_spool_bytes: u64,
    states: Vec<State<'m, 'f, 's, 'p, 'a>>,
}
pub(super) struct Leaf<'m, 'f, 's, 'p, 'a> {
    pub cell: Option<NodeId>,
    pub item: usize,
    pub top: Length,
    pub repeated: bool,
    pub header: Option<(
        &'m crate::book_v2::BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a>,
        usize,
    )>,
}
pub(super) struct Projection<'m, 'f, 's, 'p, 'a> {
    pub leaves: Vec<Leaf<'m, 'f, 's, 'p, 'a>>,
    pub semantic: Vec<usize>,
}
struct Trial<'m, 'f, 's, 'p, 'a> {
    cells: Vec<Cell<'m, 'f, 's, 'p, 'a>>,
    row: usize,
    used: Length,
    projection: Projection<'m, 'f, 's, 'p, 'a>,
    breaks: Vec<usize>,
    progress: bool,
    retry: Option<Length>,
    started_rows: bool,
    remaining: Option<Length>,
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a> {
    pub(super) fn prepare_nested(
        &mut self,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyPaginationError> {
        let table = &self.measurements.tables()[self.table_index];
        let owner = table.owner;
        if table.keep_together {
            let range = self.measurements.flow().collected.tables.tables[self.table_index]
                .items
                .clone();
            self.kernel.work.take(range.len() as u64, owner)?;
            if self.measurements.flow().collected.items[range]
                .iter()
                .any(|item| item.source.is_none())
            {
                return Err(error(owner, E::KeepAcrossForcedBreak));
            }
        }
        let mut children = Vec::new();
        for (content_owner, contents) in table
            .caption
            .iter()
            .map(|c| (owner, c.content.as_slice()))
            .chain(table.cells.iter().map(|c| (c.owner, c.content.as_slice())))
        {
            for content in contents {
                self.kernel.work.take(1, content_owner)?;
                let ProductionTableContentSource::Table(index) = content.source else {
                    continue;
                };
                self.kernel.charge.take(1, owner)?;
                children
                    .try_reserve(1)
                    .map_err(|_| error(owner, E::AllocationFailure))?;
                let charge = std::mem::replace(&mut self.kernel.charge, Charge { remaining: 0 });
                let used = std::mem::replace(&mut self.kernel.work.used, 0);
                let mut child = prepare_book_v2_table_search_charged(
                    self.measurements,
                    index,
                    limits,
                    charge,
                    Work {
                        used,
                        maximum: self.kernel.work.maximum,
                    },
                )?;
                self.kernel.charge =
                    std::mem::replace(&mut child.kernel.charge, Charge { remaining: 0 });
                self.kernel.work.used = std::mem::replace(&mut child.kernel.work.used, 0);
                children.push(child);
            }
        }
        self.nested = Some(Search {
            children,
            states: Vec::new(),
            max_spool_bytes: limits.base().get().max_spool_bytes,
        });
        Ok(())
    }
    fn nested_cells(
        &mut self,
        cursor: &TablePosition,
    ) -> Result<Vec<Cell<'m, 'f, 's, 'p, 'a>>, ProductionBodyPaginationError> {
        let owner = self.measurements.tables()[self.table_index].owner;
        let table = &self.measurements.tables()[self.table_index];
        let count = table
            .cells
            .len()
            .checked_add(usize::from(table.caption.is_some()))
            .ok_or_else(|| error(owner, E::FragmentLimit))?;
        self.kernel.charge.take(count, owner)?;
        self.kernel.work.take(count as u64, owner)?;
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(count)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        if let Some(index) = cursor.cells {
            let state = self
                .nested
                .as_ref()
                .unwrap()
                .states
                .get(index)
                .filter(|s| s.fingerprint == cursor.cell_fingerprint)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            cells.extend_from_slice(&state.cells);
        } else {
            if cursor.cell_fingerprint != [0; 32] {
                return Err(error(owner, E::ReceiptMismatch));
            }
            cells.resize(
                count,
                Cell {
                    next: 0,
                    child: None,
                },
            );
        }
        Ok(cells)
    }
    fn child_fragment(
        &mut self,
        index: usize,
        before: Option<BookV2TableCursor<'m, 'f, 's, 'p, 'a>>,
        capacity: Length,
    ) -> Result<
        Option<BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let owner = self.measurements.tables()[self.table_index].owner;
        let nested = self.nested.as_mut().unwrap();
        self.kernel.work.take(nested.children.len() as u64, owner)?;
        let child = nested
            .children
            .iter_mut()
            .find(|c| c.table_index == index)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        std::mem::swap(&mut self.kernel.charge, &mut child.kernel.charge);
        std::mem::swap(&mut self.kernel.work.used, &mut child.kernel.work.used);
        let result = (|| {
            let cursor = match before {
                Some(c) => c,
                None => child.begin()?,
            };
            child.evaluate_in_frame(&cursor, capacity, self.frame_catalog, self.frame_width)
        })();
        std::mem::swap(&mut self.kernel.charge, &mut child.kernel.charge);
        std::mem::swap(&mut self.kernel.work.used, &mut child.kernel.work.used);
        result
    }
    fn leaf(
        &mut self,
        projection: &mut Projection<'m, 'f, 's, 'p, 'a>,
        leaf: Leaf<'m, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let measurement = leaf.header.map_or(self.measurements, |(h, _)| h.variant());
        let owner = measurement.flow().collected.items[leaf.item].owner;
        self.kernel.charge.take(1, owner)?;
        self.kernel.work.take(1, owner)?;
        projection
            .leaves
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        projection.leaves.push(leaf);
        Ok(())
    }
    fn semantic(
        &mut self,
        projection: &mut Projection<'m, 'f, 's, 'p, 'a>,
        index: usize,
    ) -> Result<(), ProductionBodyPaginationError> {
        let owner = self.measurements.flow().collected.items[index].owner;
        self.kernel.charge.take(1, owner)?;
        self.kernel.work.take(1, owner)?;
        projection
            .semantic
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        projection.semantic.push(index);
        Ok(())
    }
    fn break_item(
        &mut self,
        breaks: &mut Vec<usize>,
        index: usize,
    ) -> Result<(), ProductionBodyPaginationError> {
        let owner = self.measurements.flow().collected.items[index].owner;
        self.kernel.charge.take(1, owner)?;
        self.kernel.work.take(1, owner)?;
        breaks
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        breaks.push(index);
        Ok(())
    }
    fn nested_cell(
        &mut self,
        trial: &mut Trial<'m, 'f, 's, 'p, 'a>,
        i: usize,
        row_top: Length,
        capacity: Length,
        forced_end: &mut Option<Length>,
    ) -> Result<Length, ProductionBodyPaginationError> {
        let table = &self.measurements.tables()[self.table_index];
        let (owner, contents, paint_cell) = if i == table.cells.len() {
            (
                table.owner,
                table
                    .caption
                    .as_ref()
                    .ok_or_else(|| error(table.owner, E::ReceiptMismatch))?
                    .content
                    .as_slice(),
                None,
            )
        } else {
            let cell = &table.cells[i];
            (cell.owner, cell.content.as_slice(), Some(cell.owner))
        };
        let start = Checkpoint::capture(trial, i, row_top, *forced_end);
        let mut cell_capacity = capacity;
        loop {
            let mut top = row_top;
            let mut accepted = start;
            let mut keep_pending = false;
            let mut child_retry = None;
            while trial.cells[i].next < contents.len() {
                self.kernel.work.take(1, owner)?;
                let next = trial.cells[i].next;
                let content = contents[next];
                let before = next
                    .checked_sub(1)
                    .map_or(Length::ZERO, |n| contents[n].end);
                match content.source {
                    ProductionTableContentSource::FlowItem(index) => {
                        if trial.cells[i].child.is_some() {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                        let height = content
                            .end
                            .checked_sub(before)
                            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                        let end = add(top, height, owner)?;
                        if end > cell_capacity {
                            break;
                        }
                        let relative = content
                            .top
                            .checked_sub(before)
                            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                        self.semantic(&mut trial.projection, index)?;
                        trial.cells[i].next += 1;
                        trial.progress = true;
                        let item = &self.measurements.flow().collected.items[index];
                        if item.source.is_none() {
                            self.break_item(&mut trial.breaks, index)?;
                            *forced_end = Some(forced_end.map_or(end, |old| old.min(end)));
                            top = end;
                            keep_pending = false;
                            break;
                        }
                        self.leaf(
                            &mut trial.projection,
                            Leaf {
                                cell: paint_cell,
                                item: index,
                                top: add(top, relative, owner)?,
                                repeated: false,
                                header: None,
                            },
                        )?;
                        top = end;
                        keep_pending = item.keep && trial.cells[i].next < contents.len();
                    }
                    ProductionTableContentSource::Table(index) => {
                        let child = &self.measurements.tables()[index];
                        let before_child = trial.cells[i].child;
                        let gap = if before_child.is_none() {
                            content
                                .top
                                .checked_sub(before)
                                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?
                        } else {
                            Length::ZERO
                        };
                        let origin = add(top, gap, owner)?;
                        if origin > cell_capacity {
                            break;
                        }
                        let available = cell_capacity
                            .checked_sub(origin)
                            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                        let Some(mut selected) =
                            self.child_fragment(index, before_child, available)?
                        else {
                            break;
                        };
                        let trailing = content
                            .end
                            .checked_sub(add(content.top, child.height, owner)?)
                            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                        if selected.after().is_terminal()
                            && add(selected.used_height(), trailing, owner)? > available
                        {
                            let Some(smaller) = available
                                .checked_sub(trailing)
                                .filter(|v| *v >= Length::ZERO)
                            else {
                                break;
                            };
                            let Some(earlier) =
                                self.child_fragment(index, before_child, smaller)?
                            else {
                                break;
                            };
                            selected = earlier;
                        }
                        let terminal = selected.after().is_terminal();
                        let mut end = add(origin, selected.used_height(), owner)?;
                        if terminal && add(end, trailing, owner)? > cell_capacity {
                            break;
                        }
                        for range in selected.semantic_leaf_ranges() {
                            for item in range {
                                self.semantic(&mut trial.projection, item)?;
                            }
                        }
                        if selected.has_header_variants() {
                            for leaf in selected.variant_placement_leaves() {
                                let leaf = leaf?;
                                self.leaf(
                                    &mut trial.projection,
                                    Leaf {
                                        cell: leaf.cell_owner(),
                                        item: leaf.global_item_index(),
                                        top: add(origin, leaf.top(), owner)?,
                                        repeated: leaf.repeated(),
                                        header: leaf.header_source(),
                                    },
                                )?;
                            }
                        } else {
                            for leaf in selected.caption_placement_leaves_with_repetition() {
                                let (item, relative, repeated) = leaf?;
                                self.leaf(
                                    &mut trial.projection,
                                    Leaf {
                                        cell: None,
                                        item,
                                        top: add(origin, relative, owner)?,
                                        repeated,
                                        header: None,
                                    },
                                )?;
                            }
                            for leaf in selected.placement_leaves() {
                                let (owner, item, relative, repeated) = leaf?;
                                self.leaf(
                                    &mut trial.projection,
                                    Leaf {
                                        cell: Some(owner),
                                        item,
                                        top: add(origin, relative, owner)?,
                                        repeated,
                                        header: None,
                                    },
                                )?;
                            }
                        }
                        for item in selected.forced_break_items() {
                            self.break_item(&mut trial.breaks, item)?;
                        }
                        trial.progress = true;
                        let forced = selected.forced_break_owner().is_some();
                        if forced {
                            *forced_end = Some(forced_end.map_or(end, |old| old.min(end)));
                        }
                        if terminal {
                            trial.cells[i].child = None;
                            trial.cells[i].next += 1;
                            end = add(end, trailing, owner)?;
                        } else {
                            trial.cells[i].child = Some(selected.after());
                        }
                        top = end;
                        keep_pending =
                            terminal && child.keep && trial.cells[i].next < contents.len();
                        if keep_pending {
                            if let Some(smaller) = self.nested_child_earlier(&selected)? {
                                child_retry = Some(add(origin, smaller, owner)?);
                            }
                        }
                        if forced && keep_pending {
                            return Err(error(child.owner, E::KeepAcrossForcedBreak));
                        }
                        if !terminal || forced {
                            break;
                        }
                    }
                }
                if !keep_pending {
                    accepted = Checkpoint::capture(trial, i, top, *forced_end);
                    child_retry = None;
                }
            }
            if keep_pending {
                if let Some(earlier) = child_retry {
                    if earlier >= cell_capacity {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    start.restore(trial, i, forced_end);
                    self.kernel.work.take(1, owner)?;
                    cell_capacity = earlier;
                    continue;
                }
                accepted.restore(trial, i, forced_end);
                return Ok(accepted.top);
            }
            return Ok(top);
        }
    }
    fn nested_trial(
        &mut self,
        cursor: &TablePosition,
        capacity: Length,
    ) -> Result<Trial<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let table = &self.measurements.tables()[self.table_index];
        let source = &self
            .measurements
            .flow()
            .lines()
            .prepared()
            .source_flow()
            .tables()[self.table_index];
        let owner = table.owner;
        let mut trial = Trial {
            cells: self.nested_cells(cursor)?,
            row: cursor.row,
            remaining: if self.kernel.spanning_breaks {
                Some(
                    cursor
                        .cells
                        .and_then(|i| self.nested.as_ref().unwrap().states[i].remaining)
                        .unwrap_or_else(|| {
                            table
                                .rows
                                .get(cursor.row)
                                .map_or(Length::ZERO, |r| r.height)
                        }),
                )
            } else {
                None
            },
            used: Length::ZERO,
            projection: Projection {
                leaves: Vec::new(),
                semantic: Vec::new(),
            },
            breaks: Vec::new(),
            progress: false,
            retry: None,
            started_rows: cursor.header_seen
                || (table.caption.is_none() && self.kernel.header_rows == 0),
        };
        let mut caption_keep = None;
        if let Some(caption) = &table.caption {
            let i = table.cells.len();
            if trial.cells[i].next != cursor.caption_next {
                return Err(error(owner, E::ReceiptMismatch));
            }
            if !cursor.header_seen {
                let mut forced_end = None;
                trial.used =
                    self.nested_cell(&mut trial, i, Length::ZERO, capacity, &mut forced_end)?;
                let complete = trial.cells[i].next == caption.content.len();
                let last_keep = complete
                    && table.height > caption.height
                    && caption.content.last().is_some_and(|c| match c.source {
                        ProductionTableContentSource::FlowItem(i) => {
                            self.measurements.flow().collected.items[i].keep
                        }
                        ProductionTableContentSource::Table(i) => {
                            self.measurements.tables()[i].keep
                        }
                    });
                if forced_end.is_some() {
                    if last_keep {
                        let last = caption.content.last().unwrap();
                        let origin = match last.source {
                            ProductionTableContentSource::FlowItem(i) => {
                                self.measurements.flow().collected.items[i].owner
                            }
                            ProductionTableContentSource::Table(i) => {
                                self.measurements.tables()[i].owner
                            }
                        };
                        return Err(error(origin, E::KeepAcrossForcedBreak));
                    }
                    return Ok(trial);
                }
                if !complete {
                    return Ok(trial);
                }
                if last_keep {
                    caption_keep = Some((trial.projection.semantic.len(), trial.used));
                }
                trial.started_rows = self.kernel.header_rows == 0;
                if table.rows.is_empty() {
                    trial.progress = true;
                }
            }
        }
        let header_start = if !cursor.header_seen && self.kernel.header_rows > 0 {
            Some(self.header_start(&trial)?)
        } else {
            None
        };
        let mut body_source_start = None;
        let mut header_forced = false;
        if cursor.header_seen && self.kernel.header_rows > 0 && trial.row < table.rows.len() {
            if self.kernel.paint_header_height() > capacity {
                return Ok(trial);
            }
            self.repeat_header(&mut trial.projection)?;
            trial.used = self.kernel.paint_header_height();
        }
        if self.kernel.spanning_breaks {
            self.nested_spanning_rows(
                &mut trial,
                capacity,
                &mut body_source_start,
                &mut header_forced,
            )?;
            if trial.retry.is_some() {
                return Ok(trial);
            }
        } else {
            while trial.row < table.rows.len() {
                if trial.row == self.kernel.header_rows && body_source_start.is_none() {
                    body_source_start = Some(trial.projection.semantic.len());
                }
                self.kernel
                    .work
                    .take(source.cells().len() as u64 + 1, owner)?;
                let first = source
                    .cells()
                    .partition_point(|c| (c.row() as usize) < trial.row);
                let last = source
                    .cells()
                    .partition_point(|c| (c.row() as usize) <= trial.row);
                let row_top = trial.used;
                let mut bottom = row_top;
                let mut forced_end: Option<Length> = None;
                for i in first..last {
                    let top =
                        self.nested_cell(&mut trial, i, row_top, capacity, &mut forced_end)?;
                    bottom = bottom.max(top);
                }
                if let Some(end) = forced_end {
                    if bottom > end {
                        trial.retry = Some(end);
                        return Ok(trial);
                    }
                    trial.used = bottom;
                    if trial.row < self.kernel.header_rows {
                        header_forced = true;
                        self.kernel.work.take((last - first) as u64, owner)?;
                        if (first..last)
                            .all(|i| trial.cells[i].next == table.cells[i].content.len())
                        {
                            trial.row += 1;
                        }
                        trial.started_rows = trial.row >= self.kernel.header_rows;
                    }
                    break;
                }
                trial.used = bottom;
                self.kernel.work.take((last - first) as u64, owner)?;
                if (first..last).all(|i| trial.cells[i].next == table.cells[i].content.len()) {
                    trial.row += 1;
                    trial.started_rows |= trial.row >= self.kernel.header_rows;
                    trial.progress = true;
                } else {
                    break;
                }
            }
        }
        if !header_forced {
            if let Some(start) = header_start {
                let body_height = table
                    .height
                    .checked_sub(table.caption.as_ref().map_or(Length::ZERO, |c| c.height))
                    .and_then(|v| v.checked_sub(self.kernel.header_height))
                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                if trial.row < self.kernel.header_rows
                    || (body_height > Length::ZERO
                        && body_source_start == Some(trial.projection.semantic.len()))
                {
                    start.restore(&mut trial);
                }
            }
        }
        if let Some((before_body, caption_end)) = caption_keep {
            if trial.projection.semantic.len() == before_body {
                // Empty row geometry cannot satisfy a kept final caption item.
                // Revisit the original caption cursor at a strictly earlier cut.
                trial.retry =
                    caption_end.checked_sub(Length::from_raw(1).expect("one layout unit"));
                if trial.retry.is_none() {
                    return Err(error(owner, E::Oversize));
                }
            }
        }
        Ok(trial)
    }
    fn retain_nested(
        &mut self,
        cells: Vec<Cell<'m, 'f, 's, 'p, 'a>>,
        remaining: Option<Length>,
    ) -> Result<(usize, [u8; 32]), ProductionBodyPaginationError> {
        let owner = self.measurements.tables()[self.table_index].owner;
        let size = cells
            .len()
            .checked_mul(112)
            .and_then(|n| n.checked_add(if remaining.is_some() { 32 } else { 16 }))
            .ok_or_else(|| error(owner, E::FragmentLimit))?;
        if size as u64 > self.nested.as_ref().unwrap().max_spool_bytes {
            return Err(error(owner, E::SpoolLimit));
        }
        self.kernel.charge.take(2, owner)?;
        self.kernel.work.take(cells.len() as u64, owner)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        bytes.extend_from_slice(b"NESTPOS1");
        bytes.extend_from_slice(&(cells.len() as u64).to_be_bytes());
        for c in &cells {
            bytes.extend_from_slice(&(c.next as u64).to_be_bytes());
            bytes.push(u8::from(c.child.is_some()));
            if let Some(child) = c.child {
                for n in [
                    child.table_index() as u64,
                    child.next_row() as u64,
                    child.next_caption_item() as u64,
                ] {
                    bytes.extend_from_slice(&n.to_be_bytes());
                }
                bytes.extend_from_slice(&child.offset().raw().to_be_bytes());
                bytes.extend_from_slice(&child.cell_progress_fingerprint());
                bytes.push(u8::from(child.is_initial()));
                bytes.push(u8::from(child.has_started_rows()));
            }
        }
        if let Some(remaining) = remaining {
            bytes.extend_from_slice(b"NESTROW1");
            bytes.extend_from_slice(&remaining.raw().to_be_bytes());
        }
        let fingerprint = sha256(&bytes);
        let states = &mut self.nested.as_mut().unwrap().states;
        let index = states.len();
        states
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        states.push(State {
            cells,
            fingerprint,
            remaining,
        });
        Ok((index, fingerprint))
    }
    pub(super) fn evaluate_nested(
        &mut self,
        cursor: &BookV2TableCursor<'_, '_, '_, '_, '_>,
        available: Length,
    ) -> Result<
        Option<BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let owner = self.measurements.tables()[self.table_index].owner;
        if available < Length::ZERO || available > self.maximum_height() {
            return Err(error(owner, E::InvalidTableCapacity));
        }
        if cursor.is_terminal() {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let mut capacity = available;
        let trial = loop {
            self.kernel.work.take(1, owner)?;
            let trial = self.nested_trial(&cursor.position, capacity)?;
            if let Some(earlier) = trial.retry {
                if earlier >= capacity {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                capacity = earlier;
            } else {
                break trial;
            }
        };
        let table = &self.measurements.tables()[self.table_index];
        if table.keep_together && (!trial.started_rows || trial.row < table.rows.len()) {
            if available == self.maximum_height() {
                return Err(error(owner, E::Oversize));
            }
            return Ok(None);
        }
        if !trial.progress {
            if available == self.maximum_height() {
                return Err(error(owner, E::Oversize));
            }
            return Ok(None);
        }
        let caption_next = table
            .caption
            .as_ref()
            .map_or(0, |_| trial.cells[table.cells.len()].next);
        let (index, fingerprint) = self.retain_nested(trial.cells, trial.remaining)?;
        let after = TablePosition {
            offset: add(cursor.position.offset, trial.used, owner)?,
            row: trial.row,
            initial: false,
            header_seen: trial.started_rows,
            caption_next,
            cells: Some(index),
            cell_fingerprint: fingerprint,
        };
        let forced = trial
            .breaks
            .first()
            .map(|i| self.measurements.flow().collected.items[*i].owner);
        let mut projection = self
            .kernel
            .project(
                &cursor.position,
                after,
                available,
                trial.used,
                (cursor.position.header_seen
                    && self.kernel.header_rows > 0
                    && (!trial.projection.leaves.is_empty()
                        || self.kernel.repeated_header_height.is_some()))
                .then_some(Length::ZERO),
                0..0,
                forced,
                trial.breaks,
                Vec::new(),
            )?
            .unwrap();
        self.kernel
            .work
            .take(trial.projection.leaves.len() as u64, owner)?;
        let has_nested_header_variants = trial.projection.leaves.iter().any(|l| l.header.is_some());
        let size = trial
            .projection
            .leaves
            .len()
            .checked_mul(32)
            .and_then(|n| n.checked_add(trial.projection.semantic.len().checked_mul(8)?))
            .and_then(|n| n.checked_add(56))
            .and_then(|n| {
                if has_nested_header_variants {
                    n.checked_add(
                        trial
                            .projection
                            .leaves
                            .len()
                            .checked_mul(41)?
                            .checked_add(8)?,
                    )
                } else {
                    Some(n)
                }
            })
            .ok_or_else(|| error(owner, E::FragmentLimit))?;
        if size as u64 > self.nested.as_ref().unwrap().max_spool_bytes {
            return Err(error(owner, E::SpoolLimit));
        }
        self.kernel.charge.take(1, owner)?;
        self.kernel.work.take(
            (trial.projection.leaves.len() + trial.projection.semantic.len()) as u64,
            owner,
        )?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        bytes.extend_from_slice(b"NESTFRG1");
        bytes.extend_from_slice(&projection.fingerprint);
        bytes.extend_from_slice(&(trial.projection.leaves.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&(trial.projection.semantic.len() as u64).to_be_bytes());
        for leaf in &trial.projection.leaves {
            bytes.push(u8::from(leaf.cell.is_some()));
            bytes.extend_from_slice(&leaf.cell.map_or(0, |c| c.get()).to_be_bytes());
            bytes.extend_from_slice(&(leaf.item as u64).to_be_bytes());
            bytes.extend_from_slice(&leaf.top.raw().to_be_bytes());
            bytes.push(u8::from(leaf.repeated));
        }
        for i in &trial.projection.semantic {
            bytes.extend_from_slice(&(*i as u64).to_be_bytes());
        }
        if has_nested_header_variants {
            self.kernel.work.take(size as u64, owner)?;
            bytes.extend_from_slice(b"NESTHDR1");
            for leaf in &trial.projection.leaves {
                bytes.push(u8::from(leaf.header.is_some()));
                if let Some((header, index)) = leaf.header {
                    bytes.extend_from_slice(&header.fingerprint());
                    bytes.extend_from_slice(&(index as u64).to_be_bytes());
                }
            }
        }
        projection.fingerprint = sha256(&bytes);
        Ok(Some(BookV2TableFragmentSelection {
            before: BookV2TableCursor {
                measurements: self.measurements,
                table_index: self.table_index,
                position: cursor.position,
            },
            projection,
            nested: Some(trial.projection),
            header_variant: None,
            has_nested_header_variants,
        }))
    }
}
