//! Shared pending queue and budgets for all original mixed definitions.
use super::*;
use crate::production_body::body_flow::book_v2::BookV2TableMeasurements;
use crate::production_body::body_flow::table_measurements::BookV2DefinitionTableContext;

pub fn prepare_book_v2_mixed_footnote_demand_search<'b, 'f, 's, 'p, 'a>(
    measurements: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let (contexts, charge, steps) = BookV2DefinitionTableContext::prepare_all(
        measurements,
        limits,
        maximum_work,
        prior_records,
    )?;
    let mut search = prepare_book_v2_definition_candidate_demand(
        measurements.flow(),
        limits,
        maximum_work,
        charge,
        steps,
    )?;
    search.definition_tables = Some(contexts);
    Ok(search)
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a> {
    pub fn definition_cursor(
        &self,
        definition: usize,
    ) -> Option<BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a>> {
        match self.definitions.get(definition).copied()? {
            DemandValue::Pending { cursor, .. } => Some(cursor),
            _ => None,
        }
    }
    pub fn definition_started(&self, definition: usize) -> bool {
        match self.definitions.get(definition) {
            Some(DemandValue::Complete { .. }) => true,
            Some(DemandValue::Pending { cursor, .. }) => cursor.definition_started(),
            _ => false,
        }
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Retain alternatives for one pending definition in this shared snapshot.
    /// Selecting another pending definition preserves this definition's table
    /// continuation. This API grants no region reservation or physical placement.
    pub fn enumerate_definition(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        definition: usize,
        available: Length,
    ) -> Result<BookV2DefinitionCandidates<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.verify_state(state)?;
        let root = NodeId::new(0);
        if state.status(definition) != Some(ProductionFootnoteDemandStatus::Pending) {
            return Err(error(root, E::ReceiptMismatch));
        }
        let mut context = self
            .definition_tables
            .as_mut()
            .and_then(|contexts| contexts.get_mut(definition))
            .and_then(Option::take)
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let result = (|| {
            let mut view = definition_mixed::DefinitionSearch {
                notes: self,
                tables: &mut context,
                definition,
                available,
            };
            let source = view.resume(state)?;
            view.enumerate(&source, available)
        })();
        // Restore the same arena and consumed budget after success and failure.
        self.definition_tables
            .as_mut()
            .expect("bound mixed contexts")[definition] = Some(context);
        result
    }
    pub fn select_definition(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        definition: usize,
        available: Length,
    ) -> Result<
        Option<BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        Ok(self
            .enumerate_definition(state, definition, available)?
            .into_best())
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) fn mixed_definition_start(
        &self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<Option<(NodeId, bool, Length)>, ProductionBodyPaginationError> {
        let Some(&definition) = state.pending.first() else {
            return Ok(None);
        };
        self.mixed_definition_start_at(state, definition).map(Some)
    }
    pub(super) fn mixed_definition_start_at(
        &self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        definition: usize,
    ) -> Result<(NodeId, bool, Length), ProductionBodyPaginationError> {
        let cursor = state
            .definition_cursor(definition)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let context = self
            .definition_tables
            .as_ref()
            .and_then(|contexts| contexts.get(definition))
            .and_then(Option::as_ref)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let next_table = cursor.next_table_index().unwrap_or(context.first());
        if context
            .range(next_table)
            .is_some_and(|range| range.start == cursor.next_item())
        {
            let table = context.table(next_table);
            return Ok((
                table.owner(),
                true,
                if cursor.table_continuation().is_some() {
                    Length::ZERO
                } else {
                    table.space_before()
                },
            ));
        }
        let item = self
            .content
            .flow
            .definition_items(definition)
            .and_then(|items| items.get(cursor.next_item()))
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        Ok((item.owner, item.source.is_some(), item.before))
    }
    pub(super) fn evaluate_mixed_definition(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<BookV2FootnoteDemandSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let Some(&definition) = state.pending.first() else {
            return Ok(None);
        };
        let Some(candidate) = self.select_definition(state, definition, available)? else {
            return Ok(None);
        };
        self.mixed_definition_selection(state, candidate, available)
            .map(Some)
    }
    pub(super) fn mixed_definition_selection(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        candidate: BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<BookV2FootnoteDemandSelection<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let definition = candidate.next_state().definition_index();
        candidate.verify_demand(state)?;
        let root = NodeId::new(0);
        let cursor = state
            .definition_cursor(definition)
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        self.content.charge(1, root)?;
        let mut references = Vec::new();
        let mut after = None;
        for part in candidate.parts() {
            self.content.step(root)?;
            if let Some(range) = part.items() {
                self.query_work()?;
                let original = self
                    .content
                    .flow
                    .references_in_items(Some(definition), range.clone());
                self.content.charge(original.len(), root)?;
                references
                    .try_reserve(original.len())
                    .map_err(|_| error(root, E::AllocationFailure))?;
                for reference in original {
                    self.content.step(reference.source().owner())?;
                    references.push(reference);
                }
                after = self
                    .content
                    .flow
                    .definition_items(definition)
                    .and_then(|items| items.get(range))
                    .and_then(|items| items.last())
                    .filter(|item| item.source.is_some())
                    .map(|item| item.after);
            }
            if let Some(table) = part.table() {
                let retained = self
                    .definition_table_references(table)?
                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                self.content.charge(retained.len(), root)?;
                references
                    .try_reserve(retained.len())
                    .map_err(|_| error(root, E::AllocationFailure))?;
                for reference in retained {
                    self.content.step(reference.source().owner())?;
                    references.push(reference);
                }
                let context = self
                    .definition_tables
                    .as_ref()
                    .and_then(|contexts| contexts.get(definition))
                    .and_then(Option::as_ref)
                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                after = Some(context.table(table.before().table_index()).space_after());
            }
        }
        Ok(BookV2FootnoteDemandSelection {
            owner_id: self.owner_id,
            state_id: state.state_id,
            fragment: BookV2FootnoteFragmentSelection::from_mixed(
                cursor, candidate, available, references, after,
            ),
        })
    }
}
