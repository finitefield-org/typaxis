//! Source-only demand transitions for selected parallel definition-table leaves.
use super::*;
use crate::production_body::body_flow::book_v2::BookV2TableFragmentSelection;

impl BookV2FootnoteDemandState<'_, '_, '_, '_, '_> {
    pub(in crate::production_body::body_flow) fn table_snapshot_id(&self) -> (u64, u64) {
        (self.owner_id, self.state_id)
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) fn advance_definition_table(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        selected: &BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>,
    ) -> Result<Option<BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>>, ProductionBodyPaginationError>
    {
        self.verify_state(state)?;
        let root = NodeId::new(0);
        let definition = selected
            .definition_index()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let flow = self.content.flow;
        selected.verify_source_flow(flow, Some(definition))?;
        let table_index = selected.before().table_index();
        let table = &flow.collected.tables.tables[table_index];
        let owner = table.owner;
        self.content.charge(1, owner)?;
        if table.parent.is_some() {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let region = flow
            .collected
            .definitions
            .get(definition)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let start = table
            .items
            .start
            .checked_sub(region.start)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let end = table
            .items
            .end
            .checked_sub(region.start)
            .filter(|end| *end <= region.len())
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let Some(DemandValue::Pending {
            first_reference,
            cursor,
        }) = state.definitions.get(definition).copied()
        else {
            return Err(error(owner, E::ReceiptMismatch));
        };
        if cursor.next_item() != start {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let mut position = None;
        for (i, &index) in state.pending.iter().enumerate() {
            self.content.step(owner)?;
            if index == definition {
                position = Some(i);
                break;
            }
        }
        let position = position.ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let Some(retained) = self.definition_table_references(selected)? else {
            return Ok(None);
        };
        let mut next = self.fork(state, retained.len())?;
        let mut marker_consumed = cursor.definition_started();
        if !marker_consumed {
            if let Some(marker) = flow.definition_marker(definition) {
                for range in selected.source_leaf_ranges() {
                    self.content.step(owner)?;
                    marker_consumed |= range?.contains(&marker.item_index());
                }
            }
        }
        let continuation = if selected.after().is_terminal() {
            let mut later_table = None;
            let mut stream_end = flow.table_count();
            for (index, source) in flow
                .collected
                .tables
                .tables
                .iter()
                .enumerate()
                .skip(table_index + 1)
            {
                self.content.step(owner)?;
                if source.definition != Some(definition) {
                    stream_end = index;
                    break;
                }
                if later_table.is_none() && source.parent.is_none() {
                    later_table = Some(index);
                }
            }
            (end < region.len() || later_table.is_some())
                .then(|| {
                    cursor.after_mixed(
                        end,
                        later_table.unwrap_or(stream_end),
                        None,
                        marker_consumed,
                    )
                })
                .transpose()?
        } else {
            Some(cursor.after_mixed(start, table_index, Some(selected.after()), marker_consumed)?)
        };
        kernel::advance_definition(
            &mut self.content,
            &mut next.definitions,
            &mut next.pending,
            definition,
            position,
            first_reference,
            continuation,
        )?;
        for reference in retained {
            kernel::require(
                &mut self.content,
                &mut next.definitions,
                &mut next.pending,
                std::slice::from_ref(reference),
            )?;
        }
        Ok(Some(next))
    }
    pub(super) fn definition_table_references(
        &mut self,
        selected: &BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>,
    ) -> Result<Option<Vec<&'b ProductionFootnoteFlowReference<'f>>>, ProductionBodyPaginationError>
    {
        let flow = self.content.flow;
        let definition = selected
            .definition_index()
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        selected.verify_source_flow(flow, Some(definition))?;
        let table = &flow.collected.tables.tables[selected.before().table_index()];
        let owner = table.owner;
        let base = flow.collected.definitions[definition].start;
        let start = table.items.start - base;
        let end = table.items.end - base;
        self.query_work()?;
        let references = flow.references_in_items(Some(definition), start..end);
        self.content.charge(references.len(), owner)?;
        let mut retained = Vec::new();
        retained
            .try_reserve_exact(references.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        // A reference can occupy several lines. Accept it once only when every
        // original item carrying its glyph clusters belongs to this selection.
        for reference in references {
            self.content.step(reference.source().owner())?;
            let mut any = false;
            let mut all = true;
            for item in reference.first_item_index()..=reference.last_item_index() {
                self.content.step(owner)?;
                let mut covered = false;
                for range in selected.source_leaf_ranges() {
                    self.content.step(owner)?;
                    if range?.contains(&item) {
                        covered = true;
                        break;
                    }
                }
                any |= covered;
                all &= covered;
            }
            if any && !all {
                return Ok(None);
            }
            if any {
                retained.push(reference);
            }
        }
        Ok(Some(retained))
    }
}
