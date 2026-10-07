//! Original requests for the next pending definition content.
use super::*;

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) fn pending_definition_name(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        definition: usize,
        preferred: Option<usize>,
    ) -> Result<(NodeId, Option<usize>), ProductionBodyPaginationError> {
        self.verify_state(state)?;
        let cursor = state
            .definition_cursor(definition)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        if let Some(context) = self
            .definition_tables
            .as_mut()
            .and_then(|contexts| contexts.get_mut(definition))
            .and_then(Option::as_mut)
        {
            let index = cursor.next_table_index().unwrap_or(context.first());
            if context
                .range(index)
                .is_some_and(|r| r.start == cursor.next_item())
            {
                let owner = context.table(index).owner();
                let name = context.page_name(
                    index,
                    cursor.table_continuation().as_ref(),
                    Some(preferred),
                    &mut self.content.charge,
                    &mut self.content.steps,
                )?;
                return Ok((owner, name.or(preferred)));
            }
        }
        let original = self
            .content
            .flow
            .definition_items(definition)
            .and_then(|items| items.get(cursor.next_item()))
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        self.content.step(original.owner)?;
        Ok((
            original.owner,
            self.content
                .flow
                .definition_page_name_index(definition, cursor.next_item())
                .or(preferred),
        ))
    }

    pub(super) fn next_pending_page_name(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        previous: Option<usize>,
    ) -> Result<Option<usize>, ProductionBodyPaginationError> {
        if !self.content.flow.has_named_definitions() {
            return Ok(previous);
        }
        let Some(&definition) = state.pending.first() else {
            return Ok(previous);
        };
        self.pending_definition_name(state, definition, previous)
            .map(|(_, name)| name)
    }
}
