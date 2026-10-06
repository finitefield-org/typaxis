//! Physical page names follow original caption and parallel-cell continuations.
use super::*;

#[derive(Default)]
struct Names {
    first: Option<(Option<usize>, NodeId)>,
    conflict: Option<NodeId>,
    preferred_present: bool,
}
impl Names {
    fn add(&mut self, name: Option<usize>, owner: NodeId, preferred: Option<Option<usize>>) {
        self.preferred_present |= preferred == Some(name);
        match self.first {
            None => self.first = Some((name, owner)),
            Some((first, _)) if first != name => {
                self.conflict.get_or_insert(owner);
            }
            _ => {}
        };
    }
    fn resolve(
        self,
        preferred: Option<Option<usize>>,
    ) -> Result<Option<Option<usize>>, ProductionBodyPaginationError> {
        if self.preferred_present {
            return Ok(preferred);
        }
        if let Some(owner) = self.conflict {
            return Err(error(owner, E::PendingNamedPage));
        }
        Ok(self.first.map(|(name, _)| name))
    }
}

impl<'m, 'f, 's, 'p, 'a> BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) fn prepare_named_transition_scope(
        &mut self,
    ) -> Result<(), ProductionBodyPaginationError> {
        if !self.named_transitions {
            return Ok(());
        }
        let flow = self.measurements.flow();
        let source = &flow.collected.tables.tables[self.table_index];
        let owner = source.owner;
        let initial = flow.table_page_name_index(self.table_index);
        self.kernel.work.take(source.items.len() as u64, owner)?;
        self.named_transitions = source
            .items
            .clone()
            .any(|i| flow.body_page_name_index(i) != initial);
        if self.named_transitions {
            return Ok(());
        }
        // A containing root can change while this child remains homogeneous.
        // Keep the child's original kernel/keep policy in that case. Empty
        // descendants have no leaf in this range, so inspect their real ancestry.
        for (index, child) in flow.collected.tables.tables.iter().enumerate() {
            self.kernel.work.take(1, owner)?;
            if !child.items.is_empty() || flow.table_page_name_index(index) == initial {
                continue;
            }
            let mut parent = child.parent;
            while let Some(index) = parent {
                self.kernel.work.take(1, owner)?;
                if index == self.table_index {
                    self.named_transitions = true;
                    return Ok(());
                }
                parent = flow.collected.tables.tables[index].parent;
            }
        }
        Ok(())
    }

    /// Resolve the next physical name from this exact original continuation.
    /// Completed cells may wait for other cells to finish the current name.
    /// This inspection consumes the same cumulative search-work budget.
    pub fn page_name(
        &mut self,
        cursor: &BookV2TableCursor<'_, '_, '_, '_, '_>,
    ) -> Result<Option<usize>, ProductionBodyPaginationError> {
        self.cursor_page_name(Some(cursor), None)
    }

    pub(in crate::production_body::body_flow) fn cursor_page_name(
        &mut self,
        cursor: Option<&BookV2TableCursor<'_, '_, '_, '_, '_>>,
        preferred: Option<Option<usize>>,
    ) -> Result<Option<usize>, ProductionBodyPaginationError> {
        let flow = self.measurements.flow();
        let table = &self.measurements.tables()[self.table_index];
        let owner = table.owner;
        if let Some(cursor) = cursor {
            if !std::ptr::eq(cursor.measurements, self.measurements)
                || cursor.table_index != self.table_index
            {
                return Err(error(owner, E::ReceiptMismatch));
            }
        }
        if !self.named_transitions {
            return Ok(flow.table_page_name_index(self.table_index));
        }
        self.kernel.work.take(1, owner)?;
        let position = cursor.map(|c| c.position);
        let state = position.and_then(|p| p.cells);
        let retained_name = if let Some(index) = state {
            let retained = self
                .nested
                .as_ref()
                .and_then(|s| s.states.get(index))
                .filter(|s| Some(s.fingerprint) == position.map(|p| p.cell_fingerprint))
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            Some(retained.page_name)
        } else {
            if position.is_some_and(|p| p.cell_fingerprint != [0; 32]) {
                return Err(error(owner, E::ReceiptMismatch));
            }
            None
        };
        let preferred = preferred.or(retained_name);
        if let Some(caption) = &table.caption {
            let cell = self.name_cell(state, table.cells.len())?;
            if let Some(content) = caption.content.get(cell.next) {
                return self.content_page_name(content.source, cell.child, preferred);
            }
        }
        let source = flow.lines().prepared().source_flow().tables()[self.table_index].cells();
        for row in position.map_or(0, |p| p.row)..table.rows.len() {
            self.kernel.work.take(source.len() as u64 + 1, owner)?;
            let mut names = Names::default();
            for (index, binding) in source.iter().enumerate() {
                let start = binding.row() as usize;
                if start > row || start + usize::from(binding.rowspan().get()) <= row {
                    continue;
                }
                let cell = self.name_cell(state, index)?;
                let Some(content) = table.cells[index].content.get(cell.next) else {
                    continue;
                };
                let content_owner = match content.source {
                    ProductionTableContentSource::FlowItem(i) => flow.collected.items[i].owner,
                    ProductionTableContentSource::Table(i) => self.measurements.tables()[i].owner,
                };
                let name = self.content_page_name(content.source, cell.child, preferred)?;
                names.add(name, content_owner, preferred);
            }
            if let Some(name) = names.resolve(preferred)? {
                return Ok(name);
            }
        }
        // An empty original table still owns its authored scope. Once a
        // nonempty table completes, its last physical name can continue notes.
        Ok(retained_name.unwrap_or(flow.table_page_name_index(self.table_index)))
    }

    fn name_cell(
        &self,
        state: Option<usize>,
        index: usize,
    ) -> Result<Cell<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        match state {
            Some(state) => self
                .nested
                .as_ref()
                .and_then(|s| s.states.get(state))
                .and_then(|s| s.cells.get(index))
                .copied()
                .ok_or_else(|| {
                    error(
                        self.measurements.tables()[self.table_index].owner,
                        E::ReceiptMismatch,
                    )
                }),
            None => Ok(Cell {
                next: 0,
                child: None,
            }),
        }
    }

    pub(super) fn content_page_name(
        &mut self,
        source: ProductionTableContentSource,
        before: Option<BookV2TableCursor<'m, 'f, 's, 'p, 'a>>,
        preferred: Option<Option<usize>>,
    ) -> Result<Option<usize>, ProductionBodyPaginationError> {
        match source {
            ProductionTableContentSource::FlowItem(item) => {
                let flow = self.measurements.flow();
                let original = flow.body_items().get(item).ok_or_else(|| {
                    error(
                        self.measurements.tables()[self.table_index].owner,
                        E::ReceiptMismatch,
                    )
                })?;
                self.kernel.work.take(1, original.owner)?;
                if before.is_some() {
                    return Err(error(original.owner, E::ReceiptMismatch));
                }
                Ok(flow.body_page_name_index(item))
            }
            ProductionTableContentSource::Table(index) => {
                let owner = self.measurements.tables()[self.table_index].owner;
                let nested = self
                    .nested
                    .as_mut()
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                self.kernel
                    .work
                    .take(nested.children.len() as u64 + 1, owner)?;
                let child = nested
                    .children
                    .iter_mut()
                    .find(|c| c.table_index == index)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                std::mem::swap(&mut self.kernel.charge, &mut child.kernel.charge);
                std::mem::swap(&mut self.kernel.work.used, &mut child.kernel.work.used);
                let result = child.cursor_page_name(before.as_ref(), preferred);
                std::mem::swap(&mut self.kernel.charge, &mut child.kernel.charge);
                std::mem::swap(&mut self.kernel.work.used, &mut child.kernel.work.used);
                result
            }
        }
    }

    pub(in crate::production_body::body_flow) fn evaluate_named_in_frame(
        &mut self,
        cursor: &BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
        available: Length,
        catalog: Option<&'m crate::book_v2::BookV2TableHeaderCatalog<'m, 'f, 's, 'p, 'a>>,
        width: Option<PositiveLength>,
        name: Option<usize>,
    ) -> Result<
        Option<BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let previous = self.frame_page_name;
        if self.named_transitions {
            self.frame_page_name = Some(name);
        }
        let result = self.evaluate_in_frame(cursor, available, catalog, width);
        self.frame_page_name = previous;
        result
    }
}
