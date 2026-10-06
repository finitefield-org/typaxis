//! Named definition boundaries over original local items and table cursors.
use super::*;

impl<'b, 'f, 's, 'p, 'a> DefinitionSearch<'_, 'b, 'f, 's, 'p, 'a> {
    pub(super) fn effective_name(&self) -> Option<usize> {
        self.fragment_name.unwrap_or(self.notes.active_page_name)
    }

    pub(super) fn bind_fragment_name(
        &mut self,
        state: &BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.fragment_name.is_some() || !self.notes.content.flow.has_named_definitions() {
            return Ok(());
        }
        let name = if self.notes.active_page_frames.is_some() {
            self.notes.active_page_name
        } else {
            self.current_name(state.item, state.next_table, state.continuation, None)?
                .and_then(|(_, name)| name)
        };
        self.fragment_name = Some(name);
        Ok(())
    }

    pub(super) fn current_name(
        &mut self,
        item: usize,
        next_table: usize,
        continuation: Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>>,
        preferred: Option<Option<usize>>,
    ) -> Result<Option<(NodeId, Option<usize>)>, ProductionBodyPaginationError> {
        if self
            .tables
            .range(next_table)
            .is_some_and(|r| r.start == item)
        {
            let owner = self.tables.table(next_table).owner();
            let name = self.tables.page_name(
                next_table,
                continuation.as_ref(),
                preferred,
                &mut self.notes.content.charge,
                &mut self.notes.content.steps,
            )?;
            return Ok(Some((owner, name.or(preferred.flatten()))));
        }
        let Some(original) = self
            .notes
            .content
            .flow
            .definition_items(self.definition)
            .and_then(|items| items.get(item))
        else {
            return Ok(None);
        };
        self.notes.content.step(original.owner)?;
        Ok(Some((
            original.owner,
            self.notes
                .content
                .flow
                .definition_page_name_index(self.definition, item)
                .or(preferred.flatten()),
        )))
    }

    pub(super) fn accept_name(&mut self, owner: NodeId, name: Option<usize>) -> bool {
        let matches = name.is_none_or(|name| Some(name) == self.effective_name());
        if !matches && self.notes.active_page_frames.is_some() {
            self.notes.named_mismatch.get_or_insert(owner);
        }
        matches
    }

    pub(super) fn name_boundary(
        &mut self,
        item: usize,
        next_table: usize,
        continuation: Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        if !self.notes.content.flow.has_named_definitions() {
            return Ok(false);
        }
        let name =
            self.current_name(item, next_table, continuation, Some(self.effective_name()))?;
        Ok(name.is_some_and(|(_, name)| name != self.effective_name()))
    }
}
