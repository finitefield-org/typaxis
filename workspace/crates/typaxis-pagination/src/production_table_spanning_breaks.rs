//! Measured row bands with independent spanning-cell source progress.
use super::*;

enum Trial {
    Retry(Length),
    Selected(ParallelRows),
}
impl TableBreakKernel<'_> {
    pub(super) fn spanning_rows(
        &mut self,
        cursor: &TablePosition,
        available: Length,
        prefix: Length,
        region: RowRegion,
    ) -> Result<ParallelRows, ProductionBodyPaginationError> {
        let mut capacity = available;
        loop {
            self.work.take(1, self.input.table.owner)?;
            match self.spanning_trial(cursor, capacity, prefix, region)? {
                Trial::Selected(rows) => return Ok(rows),
                Trial::Retry(earlier) => {
                    if earlier < prefix || earlier >= capacity {
                        return Err(error(self.input.table.owner, E::ReceiptMismatch));
                    }
                    capacity = earlier;
                }
            }
        }
    }
    fn spanning_trial(
        &mut self,
        cursor: &TablePosition,
        mut capacity: Length,
        prefix: Length,
        region: RowRegion,
    ) -> Result<Trial, ProductionBodyPaginationError> {
        let table = self.input.table;
        let source = self.input.source.cells();
        let owner = table.owner;
        self.charge.take(1, owner)?;
        let mut next = self.cell_positions(cursor)?;
        let mut row = cursor.row.max(region.start);
        let mut remaining = cursor
            .cells
            .and_then(|i| self.cell_states[i].row_remaining)
            .unwrap_or_else(|| table.rows.get(row).map_or(Length::ZERO, |r| r.height));
        self.charge.take(
            source
                .len()
                .checked_mul(2)
                .ok_or_else(|| error(owner, E::FragmentLimit))?,
            owner,
        )?;
        self.work.take(source.len() as u64, owner)?;
        let mut ends = Vec::new();
        ends.try_reserve_exact(source.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        ends.resize(source.len(), Length::ZERO);
        let mut stopped = Vec::new();
        stopped
            .try_reserve_exact(source.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        stopped.resize(source.len(), false);
        let mut cells = Vec::new();
        let mut breaks = Vec::new();
        let mut header_top = None;
        let mut row_top = prefix;
        let mut used = prefix;
        let mut row_work = false;
        let mut body_started = false;
        while row < region.end {
            self.work.take(source.len() as u64 + 1, owner)?;
            let active = |i: usize| {
                (source[i].row() as usize) <= row
                    && (source[i].row() as usize + usize::from(source[i].rowspan().get())) > row
                    && source[i].row() as usize >= region.start
            };
            let pending =
                (0..source.len()).any(|i| active(i) && next[i] < table.cells[i].content.len());
            if pending && region.repeat_header && header_top.is_none() {
                let top = add(row_top, self.paint_header_height(), owner)?;
                if top > capacity {
                    break;
                }
                header_top = Some(prefix);
                row_top = top;
                used = add(used, self.paint_header_height(), owner)?;
            }
            // A spanning cell may already have painted beyond this row's end.
            // A later row can introduce a still-earlier break; retry the whole
            // candidate before accepting paint beyond that newly found boundary.
            let mut cut = capacity;
            for i in 0..source.len() {
                self.work.take(1, owner)?;
                if !active(i) || stopped[i] {
                    continue;
                }
                let content = &table.cells[i].content;
                let start = next[i];
                let offset = start
                    .checked_sub(1)
                    .map_or(Length::ZERO, |n| content[n].end);
                let top = row_top.max(ends[i]);
                for child in &content[start..] {
                    self.work.take(1, table.cells[i].owner)?;
                    let end = add(
                        top,
                        child
                            .end
                            .checked_sub(offset)
                            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
                        owner,
                    )?;
                    if end > cut {
                        break;
                    }
                    let ProductionTableContentSource::FlowItem(item) = child.source else {
                        unreachable!("nested contents rejected");
                    };
                    if self.input.items[item].source.is_none() {
                        cut = end;
                        break;
                    }
                }
            }
            if cut < used {
                return Ok(Trial::Retry(cut));
            }
            capacity = cut;
            for i in 0..source.len() {
                self.work.take(1, owner)?;
                if !active(i) || stopped[i] {
                    continue;
                }
                let cell = &table.cells[i];
                let start = next[i];
                let offset = start
                    .checked_sub(1)
                    .map_or(Length::ZERO, |n| cell.content[n].end);
                let top = row_top.max(ends[i]);
                let mut finish = start;
                let mut forced = None;
                for (index, child) in cell.content.iter().enumerate().skip(start) {
                    self.work.take(1, cell.owner)?;
                    let end = add(
                        top,
                        child
                            .end
                            .checked_sub(offset)
                            .ok_or_else(|| error(cell.owner, E::ArithmeticOverflow))?,
                        cell.owner,
                    )?;
                    if end > capacity {
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
                row_work = true;
                body_started = true;
                next[i] = finish;
                let end = cell.content[finish - 1].end;
                let height = end
                    .checked_sub(offset)
                    .ok_or_else(|| error(cell.owner, E::ArithmeticOverflow))?;
                ends[i] = add(top, height, cell.owner)?;
                used = used.max(ends[i]);
                self.charge.take(1, cell.owner)?;
                cells
                    .try_reserve(1)
                    .map_err(|_| error(cell.owner, E::AllocationFailure))?;
                cells.push(ProductionTableCellSlice {
                    owner: cell.owner,
                    cell_index: i,
                    top,
                    height,
                    before: offset,
                    after: end,
                    content: start..finish,
                });
                if let Some(item) = forced {
                    stopped[i] = true;
                    self.charge.take(1, self.input.items[item].owner)?;
                    breaks
                        .try_reserve(1)
                        .map_err(|_| error(cell.owner, E::AllocationFailure))?;
                    breaks.push(item);
                }
            }
            self.work.take(source.len() as u64, owner)?;
            let mut ending_done = true;
            let mut ending_bottom = row_top;
            for i in 0..source.len() {
                if !active(i)
                    || source[i].row() as usize + usize::from(source[i].rowspan().get()) != row + 1
                {
                    continue;
                }
                ending_done &= next[i] == table.cells[i].content.len();
                ending_bottom = ending_bottom.max(ends[i]);
            }
            let minimum_end = add(row_top, remaining, owner)?;
            let boundary = minimum_end.max(ending_bottom);
            if ending_done && boundary <= capacity && (breaks.is_empty() || boundary < capacity) {
                row_work |= boundary > row_top || row + 1 < region.end;
                used = used.max(boundary);
                row_top = boundary;
                row += 1;
                remaining = table.rows.get(row).map_or(Length::ZERO, |r| r.height);
                continue;
            }
            let end = if ending_done || !breaks.is_empty() {
                capacity
            } else {
                used.max(row_top)
            };
            let advance = end
                .checked_sub(row_top)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            remaining = remaining
                .checked_sub(advance.min(remaining))
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            used = used.max(end);
            row_work |= advance > Length::ZERO;
            break;
        }
        Ok(Trial::Selected(ParallelRows {
            next,
            row,
            cells,
            breaks,
            header_top,
            row_work,
            body_started,
            used,
            row_remaining: Some(remaining),
        }))
    }
}
