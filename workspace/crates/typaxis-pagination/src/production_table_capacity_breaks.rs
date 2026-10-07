//! Preserve legal common cuts, then use source-cell progress when an ancestor's
//! actual reservation leaves no common cut. The choice belongs to the cursor;
//! saved common-cut alternatives remain usable after an independent trial.
use super::*;

impl TableBreakKernel<'_> {
    pub(super) fn evaluate_capacity_cells(
        &mut self,
        cursor: &TablePosition,
        available: Length,
    ) -> Result<Option<TableFragmentProjection>, ProductionBodyPaginationError> {
        let owner = self.input.table.owner;
        if !self.input.parallel_breaks || self.input.table.keep_together {
            return Err(error(owner, E::ReceiptMismatch));
        }
        self.work
            .take(self.input.source.cells().len() as u64, owner)?;
        let spans = self
            .input
            .source
            .cells()
            .iter()
            .any(|c| c.rowspan().get() != 1);
        let previous = (
            self.cell_breaks,
            self.spanning_breaks,
            self.common_offset_cells,
        );
        self.cell_breaks = true;
        self.spanning_breaks = spans;
        self.common_offset_cells = true;
        let result = self.evaluate_parallel(cursor, available);
        // Restore policy even after an allocation or budget failure. Retained
        // cell states and all accepted work/record charges stay with this search.
        (
            self.cell_breaks,
            self.spanning_breaks,
            self.common_offset_cells,
        ) = previous;
        result
    }

    /// Only called for a validated common cursor without an independent state.
    /// The existing cell-position allocation is already reserved and charged.
    pub(super) fn common_cell_positions(
        &mut self,
        cursor: &TablePosition,
        next: &mut [usize],
    ) -> Result<(), ProductionBodyPaginationError> {
        let table = self.input.table;
        let source = self.input.source.cells();
        let owner = table.owner;
        if next.len() != source.len() || next.len() != table.cells.len() {
            return Err(error(owner, E::ReceiptMismatch));
        }
        for (i, (binding, cell)) in source.iter().zip(&table.cells).enumerate() {
            self.work.take(
                u64::from(cell.content.len().checked_ilog2().unwrap_or(0)) + 2,
                cell.owner,
            )?;
            let origin = table.rows[binding.row() as usize].top;
            let before = cursor
                .offset
                .checked_sub(origin)
                .ok_or_else(|| error(cell.owner, E::ArithmeticOverflow))?
                .max(Length::ZERO);
            let index = cell.content.partition_point(|c| c.end <= before);
            let consumed = index
                .checked_sub(1)
                .map_or(Length::ZERO, |n| cell.content[n].end);
            // A legal common boundary cannot cut an unconsumed content extent
            // or a keep group. Completed cells can already be in row padding.
            if index < cell.content.len() && consumed != before {
                return Err(error(cell.owner, E::ReceiptMismatch));
            }
            if index > 0 && index < cell.content.len() {
                let ProductionTableContentSource::FlowItem(item) = cell.content[index - 1].source
                else {
                    return Err(error(cell.owner, E::ReceiptMismatch));
                };
                self.work.take(1, cell.owner)?;
                if self.input.items[item].keep {
                    return Err(error(cell.owner, E::ReceiptMismatch));
                }
            }
            next[i] = index;
        }
        Ok(())
    }

    /// A common cursor may already have consumed part of a spanning row band.
    /// Carry only the measured remainder into independent source continuation.
    pub(super) fn common_row_remaining(
        &mut self,
        cursor: &TablePosition,
        row: usize,
    ) -> Result<Length, ProductionBodyPaginationError> {
        let owner = self.input.table.owner;
        self.work.take(1, owner)?;
        let Some(band) = self.input.table.rows.get(row) else {
            return Ok(Length::ZERO);
        };
        add(band.top, band.height, owner)?
            .checked_sub(cursor.offset.max(band.top))
            .filter(|n| *n >= Length::ZERO)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))
    }
}
