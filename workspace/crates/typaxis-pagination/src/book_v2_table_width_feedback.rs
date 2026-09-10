//! Root-table parent widths observed on source-closed selected pages.
use super::*;

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) fn fragment_root_table(
        &mut self,
        placed: &ProductionBodyFootnotePlacedFragment,
    ) -> Result<Option<usize>, ProductionBodyPaginationError> {
        let flow = self.content.flow;
        if flow.table_count() == 0 {
            return Ok(None);
        }
        let owner = placed.fragment().owner();
        let range = match placed.definition_index() {
            None => 0..flow.collected.body_end,
            Some(index) => flow
                .collected
                .definitions
                .get(index)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                .clone(),
        };
        if placed.item_index() >= range.len() {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let item = range.start + placed.item_index();
        for (index, table) in flow.collected.tables.tables.iter().enumerate() {
            self.content.step(owner)?;
            if table.parent.is_none()
                && table.definition == placed.definition_index()
                && table.items.contains(&item)
            {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }

    pub(super) fn physical_table_parent_width(
        &mut self,
        page: &BookV2BodyMixedPageSelection<'b, 'f, 's, 'p, 'a>,
        index: usize,
    ) -> Result<PositiveLength, ProductionBodyPaginationError> {
        let table = self
            .content
            .flow
            .collected
            .tables
            .tables
            .get(index)
            .filter(|t| t.parent.is_none())
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let owner = table.owner;
        let frames = self
            .content
            .flow
            .lines()
            .frames()
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        for _ in 0..frames.measurement_region_lookup_work() {
            self.content.step(owner)?;
        }
        let original = frames
            .measurement_region(owner)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let (measured, actual) = if table.definition.is_some() {
            (
                frames
                    .footnote_region()
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                page.declared_footnote_region()
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
            )
        } else {
            (frames.body(), page.body_bounds())
        };
        actual
            .width()
            .get()
            .checked_sub(measured.width().get())
            .and_then(|d| original.width().get().checked_add(d))
            .and_then(PositiveLength::new)
            .ok_or_else(|| error(owner, E::WidthMismatch))
    }

    pub(in crate::production_body::body_flow) fn collect_root_table_width_feedback(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<(Vec<(NodeId, PositiveLength)>, bool), ProductionBodyPaginationError> {
        closed.require_single_measurement("table_header_variant_width_feedback")?;
        let root = NodeId::new(0);
        if !std::ptr::eq(closed.flow(), self.content.flow) {
            return Err(error(root, E::ReceiptMismatch));
        }
        let count = self.content.flow.table_count();
        if count == 0 {
            return Ok((Vec::new(), true));
        }
        self.content.charge.take(
            count
                .checked_mul(2)
                .and_then(|n| n.checked_add(2))
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let frames = self
            .content
            .flow
            .lines()
            .frames()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let mut targets = Vec::new();
        let mut observed = Vec::new();
        targets
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        observed
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        for table in &self.content.flow.collected.tables.tables {
            self.content.step(table.owner)?;
            for _ in 0..frames.measurement_region_lookup_work() {
                self.content.step(table.owner)?;
            }
            targets.push((
                table.owner,
                frames
                    .measurement_region(table.owner)
                    .ok_or_else(|| error(table.owner, E::ReceiptMismatch))?
                    .width(),
            ));
            observed.push(false);
        }
        for page in closed.geometry().pages() {
            self.content.step(root)?;
            let mut visit = |index: Option<usize>| -> Result<(), ProductionBodyPaginationError> {
                self.content.step(root)?;
                let Some(index) = index else {
                    return Ok(());
                };
                let width = self.physical_table_parent_width(page.selection(), index)?;
                self.content.step(targets[index].0)?;
                if observed[index] && targets[index].1 != width {
                    return Err(error(
                        targets[index].0,
                        E::PendingRegion("table_continuation_width_reflow"),
                    ));
                }
                targets[index].1 = width;
                observed[index] = true;
                Ok(())
            };
            for part in page.selection().candidate().parts() {
                visit(part.table().map(|table| table.before().table_index()))?;
            }
            if let Some(notes) = page.selection().candidate().footnotes() {
                for selected in notes.fragments() {
                    visit(None)?;
                    if let Some(mixed) = selected.fragment().mixed() {
                        for part in mixed.parts() {
                            visit(part.table().map(|table| table.before().table_index()))?;
                        }
                    }
                }
            }
        }
        let demand = closed
            .stable()
            .sequence()
            .pages()
            .last()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?
            .next_state()
            .source_state()
            .demand();
        let mut matches = true;
        let mut write = 0;
        for (index, table) in self.content.flow.collected.tables.tables.iter().enumerate() {
            self.content.step(table.owner)?;
            if table.parent.is_some() {
                continue;
            }
            let expected = table
                .definition
                .is_none_or(|d| demand.status(d) == Some(ProductionFootnoteDemandStatus::Complete));
            if observed[index] != expected {
                return Err(error(table.owner, E::ReceiptMismatch));
            }
            if observed[index] {
                for _ in 0..frames.measurement_region_lookup_work() {
                    self.content.step(table.owner)?;
                }
                matches &= frames.region(table.owner).map(|f| f.width()) == Some(targets[index].1);
            }
            targets[write] = targets[index];
            write += 1;
        }
        targets.truncate(write);
        Ok((targets, matches))
    }
}
