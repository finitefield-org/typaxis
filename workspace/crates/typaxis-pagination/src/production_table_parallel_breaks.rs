//! Independent original-cell cursors for explicit body-cell page breaks.
//! Original header breaks are consumed before later artifact repetitions.
#[path = "production_table_header_breaks.rs"]
mod header;
#[path = "production_table_spanning_breaks.rs"]
mod spanning;
use super::*;

pub(super) struct CellContinuation {
    next: Vec<usize>,
    row_remaining: Option<Length>,
    fingerprint: [u8; 32],
}
impl TableBreakKernel<'_> {
    fn cell_positions(
        &mut self,
        cursor: &TablePosition,
    ) -> Result<Vec<usize>, ProductionBodyPaginationError> {
        let owner = self.input.table.owner;
        let count = self.input.table.cells.len();
        self.charge.take(count, owner)?;
        self.work.take(count as u64, owner)?;
        let mut next = Vec::new();
        next.try_reserve_exact(count)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        if let Some(index) = cursor.cells {
            let state = self
                .cell_states
                .get(index)
                .filter(|s| s.fingerprint == cursor.cell_fingerprint)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            next.extend_from_slice(&state.next);
        } else {
            if cursor.cell_fingerprint != [0; 32] {
                return Err(error(owner, E::ReceiptMismatch));
            }
            next.resize(count, 0);
        }
        Ok(next)
    }
    fn retain_cells(
        &mut self,
        next: Vec<usize>,
        row_remaining: Option<Length>,
    ) -> Result<(usize, [u8; 32]), ProductionBodyPaginationError> {
        let owner = self.input.table.owner;
        let capacity = next
            .len()
            .checked_mul(8)
            .and_then(|n| n.checked_add(if row_remaining.is_some() { 16 } else { 0 }))
            .ok_or_else(|| error(owner, E::FragmentLimit))?;
        if self
            .input
            .max_canonical_bytes
            .is_some_and(|limit| capacity as u64 > limit)
        {
            return Err(error(owner, E::SpoolLimit));
        }
        self.charge.take(2, owner)?;
        self.work.take(next.len() as u64, owner)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        for n in &next {
            bytes.extend_from_slice(&(*n as u64).to_be_bytes());
        }
        if let Some(remaining) = row_remaining {
            bytes.extend_from_slice(b"SPANROW1");
            bytes.extend_from_slice(&remaining.raw().to_be_bytes());
        }
        let fingerprint = sha256(&bytes);
        let index = self.cell_states.len();
        self.cell_states
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        self.cell_states.push(CellContinuation {
            next,
            row_remaining,
            fingerprint,
        });
        Ok((index, fingerprint))
    }
    pub(super) fn evaluate_parallel(
        &mut self,
        cursor: &TablePosition,
        available: Length,
    ) -> Result<Option<TableFragmentProjection>, ProductionBodyPaginationError> {
        let table = self.input.table;
        let owner = table.owner;
        if available < Length::ZERO || available > self.maximum_height {
            return Err(error(owner, E::InvalidTableCapacity));
        }
        if cursor.row > table.rows.len()
            || (!cursor.initial && cursor.header_seen && cursor.row == table.rows.len())
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let mut caption = 0..0;
        let mut used = Length::ZERO;
        let mut caption_next = cursor.caption_next;
        if let Some(c) = &table.caption {
            caption = caption_next..caption_next;
            if caption_next < c.content.len() || cursor.offset < c.height {
                let Some(prefix) = self.evaluate_caption(cursor, available)? else {
                    return Ok(None);
                };
                if prefix.after.caption_next < c.content.len()
                    || prefix.after.offset < c.height
                    || prefix.forced_break.is_some()
                {
                    return Ok(Some(prefix));
                }
                used = prefix.used_height;
                caption = prefix.caption;
                caption_next = prefix.after.caption_next;
            }
        }
        let ParallelRows {
            next,
            row,
            cells,
            breaks,
            mut header_top,
            row_work,
            body_started,
            used: row_used,
            row_remaining,
        } = if self.header_breaks && !cursor.header_seen {
            self.first_header_rows(cursor, available, used)?
        } else if self.spanning_breaks {
            self.spanning_rows(cursor, available, used, self.body_region(true))?
        } else {
            self.parallel_rows(cursor, available, used, self.body_region(true))?
        };
        used = row_used;
        if !body_started && row < table.rows.len() {
            // Do not paint a header on a page that cannot begin any body item.
            if header_top.is_some() {
                used = used
                    .checked_sub(self.paint_header_height())
                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                header_top = None;
            }
            let caption_keep = table
                .caption
                .as_ref()
                .and_then(|c| c.content.last())
                .is_some_and(|c| {
                    let ProductionTableContentSource::FlowItem(item) = c.source else {
                        return false;
                    };
                    self.input.items[item].keep
                });
            if !caption.is_empty() && caption_keep {
                let remaining = table
                    .caption
                    .as_ref()
                    .unwrap()
                    .height
                    .checked_sub(cursor.offset)
                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                let earlier = remaining
                    .checked_sub(Length::from_raw(1).expect("one layout unit"))
                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                let prefix = self.evaluate_caption(cursor, earlier)?;
                if prefix.is_none() && available == self.maximum_height {
                    return Err(error(owner, E::Oversize));
                }
                return Ok(prefix);
            }
            if caption.is_empty() && !row_work {
                if available == self.maximum_height {
                    return Err(error(owner, E::Oversize));
                }
                return Ok(None);
            }
        } else if !self.header_breaks
            && !cursor.header_seen
            && header_top.is_none()
            && row == table.rows.len()
        {
            // A genuinely empty body still owns its first header once.
            let with_header = add(used, self.paint_header_height(), owner)?;
            if with_header > available {
                return Ok(None);
            }
            header_top = Some(used);
            used = with_header;
        }
        let forced = breaks.first().map(|index| self.input.items[*index].owner);
        let (index, fingerprint) = self.retain_cells(next, row_remaining)?;
        let after = TablePosition {
            offset: add(cursor.offset, used, owner)?,
            row,
            initial: false,
            header_seen: if self.header_breaks {
                row >= self.header_rows
            } else {
                cursor.header_seen || header_top.is_some()
            },
            caption_next,
            cells: Some(index),
            cell_fingerprint: fingerprint,
        };
        self.project(
            cursor, after, available, used, header_top, caption, forced, breaks, cells,
        )
    }
    fn parallel_rows(
        &mut self,
        cursor: &TablePosition,
        available: Length,
        mut used: Length,
        region: RowRegion,
    ) -> Result<ParallelRows, ProductionBodyPaginationError> {
        let table = self.input.table;
        let owner = table.owner;
        self.charge.take(1, owner)?;
        self.work.take(1, owner)?;
        let mut next = self.cell_positions(cursor)?;
        let mut row = cursor.row.max(region.start);
        let mut cells = Vec::new();
        let mut breaks = Vec::new();
        let mut header_top = None;
        let mut row_work = false;
        let mut header_reserved = !region.repeat_header;
        while row < region.end {
            self.work.take(
                2 * (u64::from(table.cells.len().checked_ilog2().unwrap_or(0)) + 1),
                owner,
            )?;
            let source = self.input.source.cells();
            let first = source.partition_point(|c| (c.row() as usize) < row);
            let last = source.partition_point(|c| (c.row() as usize) <= row);
            self.work.take((last - first) as u64, owner)?;
            if (first..last).all(|i| next[i] == table.cells[i].content.len()) {
                row += 1;
                continue;
            }
            if !header_reserved {
                let with_header = add(used, self.paint_header_height(), owner)?;
                if with_header > available {
                    break;
                }
                header_top = Some(used);
                header_reserved = true;
                used = with_header;
            }
            let remaining = available
                .checked_sub(used)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            let mut cut = remaining;
            // The earliest reachable break in any parallel cell ends this page.
            for i in first..last {
                let content = &table.cells[i].content;
                let start = next[i];
                let offset = start
                    .checked_sub(1)
                    .map_or(Length::ZERO, |n| content[n].end);
                for child in &content[start..] {
                    self.work.take(1, table.cells[i].owner)?;
                    let height = child
                        .end
                        .checked_sub(offset)
                        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                    if height > cut {
                        break;
                    }
                    let ProductionTableContentSource::FlowItem(item) = child.source else {
                        unreachable!("nested contents rejected");
                    };
                    if self.input.items[item].source.is_none() {
                        cut = height;
                        break;
                    }
                }
            }
            let mut row_height = Length::ZERO;
            let mut progressed = false;
            for i in first..last {
                let cell = &table.cells[i];
                let start = next[i];
                let offset = start
                    .checked_sub(1)
                    .map_or(Length::ZERO, |n| cell.content[n].end);
                let limit = add(offset, cut, cell.owner)?;
                let mut finish = start;
                let mut forced = None;
                for (index, child) in cell.content.iter().enumerate().skip(start) {
                    self.work.take(1, cell.owner)?;
                    if child.end > limit {
                        break;
                    }
                    let ProductionTableContentSource::FlowItem(item) = child.source else {
                        unreachable!("nested contents rejected");
                    };
                    if self.input.items[item].source.is_none() {
                        forced = Some(item);
                        finish = index + 1;
                        break;
                    }
                    if !self.input.items[item].keep || index + 1 == cell.content.len() {
                        finish = index + 1;
                    }
                }
                if finish == start {
                    continue;
                }
                progressed = true;
                next[i] = finish;
                let end = cell.content[finish - 1].end;
                let height = end
                    .checked_sub(offset)
                    .ok_or_else(|| error(cell.owner, E::ArithmeticOverflow))?;
                row_height = row_height.max(height);
                self.charge.take(1, cell.owner)?;
                cells
                    .try_reserve(1)
                    .map_err(|_| error(cell.owner, E::AllocationFailure))?;
                cells.push(ProductionTableCellSlice {
                    owner: cell.owner,
                    cell_index: i,
                    top: used,
                    height,
                    before: offset,
                    after: end,
                    content: start..finish,
                });
                if let Some(item) = forced {
                    self.charge.take(1, self.input.items[item].owner)?;
                    breaks
                        .try_reserve(1)
                        .map_err(|_| error(cell.owner, E::AllocationFailure))?;
                    breaks.push(item);
                }
            }
            if !progressed {
                break;
            }
            row_work = true;
            used = add(used, row_height, owner)?;
            if !breaks.is_empty() {
                break;
            }
            self.work.take((last - first) as u64, owner)?;
            if (first..last).all(|i| next[i] == table.cells[i].content.len()) {
                row += 1;
            } else {
                break;
            }
        }
        Ok(ParallelRows {
            next,
            row,
            cells,
            breaks,
            header_top,
            row_work,
            body_started: row_work,
            used,
            row_remaining: None,
        })
    }
}

struct ParallelRows {
    next: Vec<usize>,
    row: usize,
    cells: Vec<ProductionTableCellSlice>,
    breaks: Vec<usize>,
    header_top: Option<Length>,
    row_work: bool,
    body_started: bool,
    used: Length,
    row_remaining: Option<Length>,
}

#[derive(Clone, Copy)]
struct RowRegion {
    start: usize,
    end: usize,
    repeat_header: bool,
}
impl TableBreakKernel<'_> {
    fn body_region(&self, repeat_header: bool) -> RowRegion {
        RowRegion {
            start: self.header_rows,
            end: self.input.table.rows.len(),
            repeat_header,
        }
    }
}
