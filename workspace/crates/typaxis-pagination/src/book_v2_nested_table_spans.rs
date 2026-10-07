//! Measured parent row bands with independent original child-table cursors.
use super::*;

impl<'m, 'f, 's, 'p, 'a> BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a> {
    pub(super) fn nested_spanning_rows(
        &mut self,
        trial: &mut Trial<'m, 'f, 's, 'p, 'a>,
        mut capacity: Length,
        body_source_start: &mut Option<usize>,
        header_forced: &mut bool,
    ) -> Result<(), ProductionBodyPaginationError> {
        let table = &self.measurements.tables()[self.table_index];
        let source = self
            .measurements
            .flow()
            .lines()
            .prepared()
            .source_flow()
            .tables()[self.table_index]
            .cells();
        let owner = table.owner;
        self.kernel.charge.take(
            source
                .len()
                .checked_mul(2)
                .ok_or_else(|| error(owner, E::FragmentLimit))?,
            owner,
        )?;
        self.kernel.work.take(source.len() as u64, owner)?;
        let mut ends = Vec::new();
        let mut stopped = Vec::new();
        ends.try_reserve_exact(source.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        stopped
            .try_reserve_exact(source.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        ends.resize(source.len(), Length::ZERO);
        stopped.resize(source.len(), false);
        let mut row_top = trial.used;
        let mut remaining = trial
            .remaining
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        while trial.row < table.rows.len() {
            let row = trial.row;
            let in_header = row < self.kernel.header_rows;
            if row == self.kernel.header_rows && body_source_start.is_none() {
                *body_source_start = Some(trial.projection.semantic.len());
            }
            let region_start = if in_header {
                0
            } else {
                self.kernel.header_rows
            };
            self.kernel.work.take(source.len() as u64 + 1, owner)?;
            let active = |i: usize| {
                let start = source[i].row() as usize;
                start >= region_start
                    && start <= row
                    && start + usize::from(source[i].rowspan().get()) > row
            };
            let mut forced_end = None;
            for i in 0..source.len() {
                self.kernel.work.take(1, owner)?;
                if !active(i) || stopped[i] {
                    continue;
                }
                let top = row_top.max(ends[i]);
                let breaks = trial.breaks.len();
                let end = self.nested_cell(trial, i, top, capacity, &mut forced_end)?;
                ends[i] = end;
                trial.used = trial.used.max(end);
                if trial.breaks.len() > breaks {
                    stopped[i] = true;
                    *header_forced |= in_header;
                }
                if let Some(cut) = forced_end {
                    if cut < trial.used {
                        trial.retry = Some(cut);
                        return Ok(());
                    }
                    capacity = cut;
                }
            }
            self.kernel.work.take(source.len() as u64, owner)?;
            let mut ending_done = true;
            let mut ending_bottom = row_top;
            for i in 0..source.len() {
                if !active(i)
                    || source[i].row() as usize + usize::from(source[i].rowspan().get()) != row + 1
                {
                    continue;
                }
                ending_done &= trial.cells[i].next == table.cells[i].content.len();
                ending_bottom = ending_bottom.max(ends[i]);
            }
            let boundary = add(row_top, remaining, owner)?.max(ending_bottom);
            if ending_done
                && boundary <= capacity
                && (trial.breaks.is_empty() || boundary < capacity || in_header)
            {
                trial.used = trial.used.max(boundary);
                trial.progress = true;
                row_top = boundary;
                trial.row += 1;
                trial.started_rows |= trial.row >= self.kernel.header_rows;
                remaining = table.rows.get(trial.row).map_or(Length::ZERO, |r| r.height);
                if in_header
                    && !trial.breaks.is_empty()
                    && (boundary == capacity || trial.row == self.kernel.header_rows)
                {
                    break;
                }
                continue;
            }
            let end = if ending_done || !trial.breaks.is_empty() {
                capacity
            } else {
                trial.used.max(row_top)
            };
            let advance = end
                .checked_sub(row_top)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            remaining = remaining
                .checked_sub(advance.min(remaining))
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            trial.used = trial.used.max(end);
            trial.progress |= advance > Length::ZERO;
            break;
        }
        trial.remaining = Some(remaining);
        Ok(())
    }
}
