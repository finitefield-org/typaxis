//! Original header prefixes and full repeated paint retain separate ownership.
use super::*;

pub(super) struct HeaderStart<'m, 'f, 's, 'p, 'a> {
    cells: Vec<Cell<'m, 'f, 's, 'p, 'a>>,
    row: usize,
    used: Length,
    semantic: usize,
    leaves: usize,
    breaks: usize,
    progress: bool,
    started: bool,
    remaining: Option<Length>,
}
impl<'m, 'f, 's, 'p, 'a> HeaderStart<'m, 'f, 's, 'p, 'a> {
    pub(super) fn restore(self, trial: &mut Trial<'m, 'f, 's, 'p, 'a>) {
        trial.cells = self.cells;
        trial.row = self.row;
        trial.used = self.used;
        trial.projection.semantic.truncate(self.semantic);
        trial.projection.leaves.truncate(self.leaves);
        trial.breaks.truncate(self.breaks);
        trial.progress = self.progress;
        trial.started_rows = self.started;
        trial.remaining = self.remaining;
    }
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a> {
    pub(super) fn header_start(
        &mut self,
        trial: &Trial<'m, 'f, 's, 'p, 'a>,
    ) -> Result<HeaderStart<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let owner = self.measurements.tables()[self.table_index].owner;
        self.kernel.charge.take(trial.cells.len(), owner)?;
        self.kernel.work.take(trial.cells.len() as u64, owner)?;
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(trial.cells.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        cells.extend_from_slice(&trial.cells);
        Ok(HeaderStart {
            cells,
            row: trial.row,
            used: trial.used,
            semantic: trial.projection.semantic.len(),
            leaves: trial.projection.leaves.len(),
            breaks: trial.breaks.len(),
            progress: trial.progress,
            started: trial.started_rows,
            remaining: trial.remaining,
        })
    }
    pub(super) fn repeat_header(
        &mut self,
        projection: &mut Projection<'m, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        // The variant selection retains its own exact geometry owner. Do not
        // emit this table's old header indexes. Independent child searches keep
        // their own selected measurement associations in the nested projection.
        if self.kernel.repeated_header_height.is_some() {
            return Ok(());
        }
        self.repeat_table_region(self.table_index, Length::ZERO, true, projection)
    }
    fn repeat_table_region(
        &mut self,
        index: usize,
        origin: Length,
        header_only: bool,
        projection: &mut Projection<'m, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let table = &self.measurements.tables()[index];
        let source = &self
            .measurements
            .flow()
            .lines()
            .prepared()
            .source_flow()
            .tables()[index];
        if !header_only {
            if let Some(caption) = &table.caption {
                self.repeat_contents(
                    caption.content.as_slice(),
                    None,
                    origin,
                    table.owner,
                    projection,
                )?;
            }
        }
        for (cell, binding) in table.cells.iter().zip(source.cells()) {
            self.kernel.work.take(1, cell.owner)?;
            let row = &table.rows[binding.row() as usize];
            if header_only && row.section != ProductionTableSection::Head {
                break;
            }
            let top = if header_only {
                row.top
                    .checked_sub(table.caption.as_ref().map_or(Length::ZERO, |c| c.height))
                    .ok_or_else(|| error(table.owner, E::ArithmeticOverflow))?
            } else {
                row.top
            };
            self.repeat_contents(
                &cell.content,
                Some(cell.owner),
                add(origin, top, cell.owner)?,
                cell.owner,
                projection,
            )?;
        }
        Ok(())
    }
    fn repeat_contents(
        &mut self,
        contents: &[ProductionTableCellContent],
        cell: Option<NodeId>,
        origin: Length,
        owner: NodeId,
        projection: &mut Projection<'m, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        for content in contents {
            self.kernel.work.take(1, owner)?;
            let top = add(origin, content.top, owner)?;
            match content.source {
                ProductionTableContentSource::FlowItem(item) => {
                    if self.measurements.flow().collected.items[item]
                        .source
                        .is_some()
                    {
                        self.leaf(
                            projection,
                            Leaf {
                                cell,
                                item,
                                top,
                                repeated: true,
                                header: None,
                            },
                        )?;
                    }
                }
                ProductionTableContentSource::Table(index) => {
                    self.repeat_table_region(index, top, false, projection)?
                }
            }
        }
        Ok(())
    }
}
