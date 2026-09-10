//! Original header source pagination, distinct from later artifact repetitions.
use super::*;
impl TableBreakKernel<'_> {
    fn defer_header(
        &mut self,
        cursor: &TablePosition,
        prefix: Length,
    ) -> Result<ParallelRows, ProductionBodyPaginationError> {
        let next = self.cell_positions(cursor)?;
        let row_remaining = cursor.cells.and_then(|i| self.cell_states[i].row_remaining);
        Ok(ParallelRows {
            next,
            row: cursor.row,
            cells: Vec::new(),
            breaks: Vec::new(),
            header_top: None,
            row_work: false,
            body_started: false,
            used: prefix,
            row_remaining,
        })
    }
    pub(super) fn first_header_rows(
        &mut self,
        cursor: &TablePosition,
        available: Length,
        prefix: Length,
    ) -> Result<ParallelRows, ProductionBodyPaginationError> {
        let region = RowRegion {
            start: 0,
            end: self.header_rows,
            repeat_header: false,
        };
        let mut head = if self.spanning_breaks {
            self.spanning_rows(cursor, available, prefix, region)?
        } else {
            self.parallel_rows(cursor, available, prefix, region)?
        };
        if !head.breaks.is_empty() {
            return Ok(head);
        }
        // Only an authored break may separate original header content from its
        // first body row. A short outside prefix must move to the next page.
        if head.row < self.header_rows {
            return self.defer_header(cursor, prefix);
        }
        let repeat = head.cells.is_empty() && cursor.cells.is_some();
        let (index, fingerprint) = self.retain_cells(head.next, None)?;
        let body_cursor = TablePosition {
            row: self.header_rows,
            header_seen: true,
            cells: Some(index),
            cell_fingerprint: fingerprint,
            ..*cursor
        };
        let region = self.body_region(repeat);
        let mut body = if self.spanning_breaks {
            self.spanning_rows(&body_cursor, available, head.used, region)?
        } else {
            self.parallel_rows(&body_cursor, available, head.used, region)?
        };
        if !body.body_started && body.row < self.input.table.rows.len() {
            return self.defer_header(cursor, prefix);
        }
        let owner = self.input.table.owner;
        self.charge.take(body.cells.len(), owner)?;
        self.work.take(body.cells.len() as u64, owner)?;
        head.cells
            .try_reserve(body.cells.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        head.cells.append(&mut body.cells);
        body.cells = head.cells;
        body.row_work |= head.row_work;
        body.body_started |= head.body_started;
        Ok(body)
    }
}
