//! Ordered definition leaves and original parallel table continuations.
//! These candidates carry source/demand authority, not a page reservation.
use super::*;
use crate::production_body::body_flow::book_v2::{
    BookV2TableCursor, BookV2TableFragmentSelection, BookV2TableMeasurements,
};
use crate::production_body::body_flow::table_measurements::BookV2DefinitionTableContext;
use mixed_kernel::source_kernel::{self, SourceSearch};
use std::ops::Range;

#[derive(Clone, Copy)]
pub enum BookV2DefinitionCandidatePart<'b, 'f, 's, 'p, 'a> {
    /// Exclusive local definition item index; may not cross a root table.
    Items { end: usize },
    Table {
        cursor: BookV2TableCursor<'b, 'f, 's, 'p, 'a>,
        capacity: Length,
    },
    /// Consume the next authored nonpainting page break, ending this candidate.
    Forced,
}
pub struct BookV2DefinitionSelectedPart<'b, 'f, 's, 'p, 'a> {
    items: Option<Range<usize>>,
    table: Option<BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>>,
    top: Length,
    height: Length,
}
impl<'b, 'f, 's, 'p, 'a> BookV2DefinitionSelectedPart<'b, 'f, 's, 'p, 'a> {
    pub fn items(&self) -> Option<Range<usize>> {
        self.items.clone()
    }
    pub fn table(&self) -> Option<&BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>> {
        self.table.as_ref()
    }
    pub fn top(&self) -> Length {
        self.top
    }
    pub fn height(&self) -> Length {
        self.height
    }
}
pub struct BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a> {
    demand: BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    definition: usize,
    item: usize,
    next_table: usize,
    continuation: Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a> {
    pub fn definition_index(&self) -> usize {
        self.definition
    }
    pub fn next_item(&self) -> usize {
        self.item
    }
    pub fn next_table_index(&self) -> usize {
        self.next_table
    }
    pub fn table_continuation(&self) -> Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>> {
        self.continuation
    }
    pub fn demand(&self) -> &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a> {
        &self.demand
    }
    pub fn into_demand(self) -> BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a> {
        self.demand
    }
    pub fn is_complete(&self) -> bool {
        self.demand.status(self.definition) == Some(ProductionFootnoteDemandStatus::Complete)
    }
}
pub struct BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a> {
    source: (u64, u64),
    parts: Vec<BookV2DefinitionSelectedPart<'b, 'f, 's, 'p, 'a>>,
    height: Length,
    forced: Option<NodeId>,
    next: BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a> {
    pub fn parts(&self) -> &[BookV2DefinitionSelectedPart<'b, 'f, 's, 'p, 'a>] {
        &self.parts
    }
    pub fn used_height(&self) -> Length {
        self.height
    }
    pub fn forced_break_owner(&self) -> Option<NodeId> {
        self.forced
    }
    pub fn next_state(&self) -> &BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a> {
        &self.next
    }
    pub fn into_next_state(self) -> BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a> {
        self.next
    }
    pub fn verify_demand(
        &self,
        state: &BookV2FootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.source != state.table_snapshot_id() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
    pub fn verify(
        &self,
        state: &BookV2DefinitionSourceState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.source != state.demand.table_snapshot_id()
            || self.next.definition != state.definition
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}
pub struct BookV2DefinitionMixedSearch<'b, 'f, 's, 'p, 'a> {
    notes: BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>,
    tables: BookV2DefinitionTableContext<'b, 'f, 's, 'p, 'a>,
    definition: usize,
}
pub(super) struct DefinitionSearch<'r, 'b, 'f, 's, 'p, 'a> {
    pub(super) notes: &'r mut BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>,
    pub(super) tables: &'r mut BookV2DefinitionTableContext<'b, 'f, 's, 'p, 'a>,
    pub(super) definition: usize,
    pub(super) available: Length,
}
pub fn prepare_book_v2_definition_mixed_search<'b, 'f, 's, 'p, 'a>(
    measurements: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
    definition: usize,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2DefinitionMixedSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let (tables, charge, steps) = BookV2DefinitionTableContext::prepare(
        measurements,
        definition,
        limits,
        maximum_work,
        prior_records,
    )?;
    let notes = prepare_book_v2_definition_candidate_demand(
        measurements.flow(),
        limits,
        maximum_work,
        charge,
        steps,
    )?;
    Ok(BookV2DefinitionMixedSearch {
        notes,
        tables,
        definition,
    })
}
impl<'b, 'f, 's, 'p, 'a> DefinitionSearch<'_, 'b, 'f, 's, 'p, 'a> {
    pub fn maximum_height(&self) -> Length {
        self.notes.content.maximum_height
    }
    pub fn begin(
        &mut self,
        body: Range<usize>,
    ) -> Result<BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let initial = self.notes.begin()?;
        let demand = self.notes.require_body(&initial, body)?;
        if demand.status(self.definition) != Some(ProductionFootnoteDemandStatus::Pending) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(BookV2DefinitionSourceState {
            demand,
            definition: self.definition,
            item: 0,
            next_table: self.tables.first(),
            continuation: None,
        })
    }
    pub(super) fn resume(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.notes.verify_state(state)?;
        let root = NodeId::new(0);
        let Some(DemandValue::Pending { cursor, .. }) =
            state.definitions.get(self.definition).copied()
        else {
            return Err(error(root, E::ReceiptMismatch));
        };
        let next_table = cursor.next_table_index().unwrap_or(self.tables.first());
        if next_table != self.tables.end() && self.tables.range(next_table).is_none() {
            return Err(error(root, E::ReceiptMismatch));
        }
        let (definitions, pending) = kernel::fork_definitions(
            &mut self.notes.content,
            &state.definitions,
            &state.pending,
            0,
        )?;
        Ok(BookV2DefinitionSourceState {
            demand: BookV2FootnoteDemandState {
                flow: state.flow,
                owner_id: state.owner_id,
                state_id: state.state_id,
                definitions,
                pending,
            },
            definition: self.definition,
            item: cursor.next_item(),
            next_table,
            continuation: cursor.table_continuation(),
        })
    }
    pub fn begin_table(
        &mut self,
        index: usize,
    ) -> Result<BookV2TableCursor<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.notes.content.step(NodeId::new(0))?;
        self.tables.begin(
            index,
            &mut self.notes.content.charge,
            &mut self.notes.content.steps,
        )
    }
    pub fn evaluate(
        &mut self,
        state: &BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>,
        requests: &[BookV2DefinitionCandidatePart<'b, 'f, 's, 'p, 'a>],
        available: Length,
    ) -> Result<
        Option<BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.notes.verify_state(&state.demand)?;
        let root = NodeId::new(0);
        if state.definition != self.definition || state.is_complete() || requests.is_empty() {
            return Err(error(root, E::ReceiptMismatch));
        }
        if available < Length::ZERO || available > self.maximum_height() {
            return Err(error(root, E::InvalidFootnoteCapacity));
        }
        let requested = match requests.first() {
            Some(BookV2DefinitionCandidatePart::Table { cursor, .. }) => Some(*cursor),
            _ => None,
        };
        if let Some(expected) = state.continuation {
            if requested.is_none_or(|cursor| !same_cursor(cursor, expected)) {
                return Err(error(root, E::ReceiptMismatch));
            }
        } else if requested.is_some_and(|cursor| !cursor.is_initial()) {
            return Err(error(root, E::ReceiptMismatch));
        }
        self.available = available;
        let Some(selected) = source_kernel::evaluate(
            self,
            &state.demand,
            state.item,
            Some(state.next_table),
            state.continuation,
            requests,
        )?
        else {
            return Ok(None);
        };
        let source_kernel::Projection {
            end,
            next_table,
            continuation,
            parts,
            height,
            mut demanded,
            forced,
        } = selected;
        let next_table = next_table.expect("definition source ordinal");
        let Some(DemandValue::Pending {
            first_reference,
            cursor,
        }) = demanded.definitions.get(self.definition).copied()
        else {
            return Err(error(root, E::ReceiptMismatch));
        };
        let mut position = None;
        for (p, &index) in demanded.pending.iter().enumerate() {
            self.notes.content.step(root)?;
            if index == self.definition {
                position = Some(p);
                break;
            }
        }
        let remaining = continuation.is_some() || self.remaining_source(end, Some(next_table));
        let mut marker_consumed = cursor.definition_started();
        if !marker_consumed {
            if let Some(marker) = self.notes.content.flow.definition_marker(self.definition) {
                for part in &parts {
                    self.notes.content.step(root)?;
                    marker_consumed |= part
                        .items
                        .as_ref()
                        .is_some_and(|range| range.contains(&marker.item_index()));
                    if let Some(table) = &part.table {
                        for range in table.source_leaf_ranges() {
                            self.notes.content.step(root)?;
                            marker_consumed |= range?.contains(&marker.item_index());
                        }
                    }
                }
            }
        }
        kernel::advance_definition(
            &mut self.notes.content,
            &mut demanded.definitions,
            &mut demanded.pending,
            self.definition,
            position.ok_or_else(|| error(root, E::ReceiptMismatch))?,
            first_reference,
            remaining
                .then(|| cursor.after_mixed(end, next_table, continuation, marker_consumed))
                .transpose()?,
        )?;
        Ok(Some(BookV2DefinitionMixedCandidate {
            source: state.demand.table_snapshot_id(),
            parts,
            height,
            forced,
            next: BookV2DefinitionSourceState {
                demand: demanded,
                definition: self.definition,
                item: end,
                next_table,
                continuation,
            },
        }))
    }
}
fn same_cursor(
    a: BookV2TableCursor<'_, '_, '_, '_, '_>,
    b: BookV2TableCursor<'_, '_, '_, '_, '_>,
) -> bool {
    a.table_index() == b.table_index()
        && a.offset() == b.offset()
        && a.next_row() == b.next_row()
        && a.is_initial() == b.is_initial()
        && a.has_started_rows() == b.has_started_rows()
        && a.next_caption_item() == b.next_caption_item()
        && a.cell_progress_fingerprint() == b.cell_progress_fingerprint()
}
impl<'b, 'f, 's, 'p, 'a> SourceSearch<'b> for DefinitionSearch<'_, 'b, 'f, 's, 'p, 'a> {
    type State = BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>;
    type TableFragment = BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>;
    type Cursor = BookV2TableCursor<'b, 'f, 's, 'p, 'a>;
    type Request = BookV2DefinitionCandidatePart<'b, 'f, 's, 'p, 'a>;
    type Part = BookV2DefinitionSelectedPart<'b, 'f, 's, 'p, 'a>;
    fn verify(&self, state: &Self::State) -> Result<(), ProductionBodyPaginationError> {
        self.notes.verify_state(state)
    }
    fn items(&self) -> &'b [ProductionBodyFlowItem] {
        self.notes
            .content
            .flow
            .definition_items(self.definition)
            .expect("bound definition")
    }
    fn maximum(&self) -> Length {
        self.available
    }
    fn charge(&mut self, count: usize, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.notes.content.charge(count, owner)
    }
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.notes.content.step(owner)
    }
    fn fork_table(
        &mut self,
        state: &Self::State,
    ) -> Result<Self::State, ProductionBodyPaginationError> {
        self.notes.fork(state, 0)
    }
    fn pending(&self, _state: &Self::State) -> bool {
        false
    }
    fn require_range(
        &mut self,
        state: &mut Self::State,
        range: Range<usize>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.notes.query_work()?;
        let references = self
            .notes
            .content
            .flow
            .references_in_items(Some(self.definition), range.clone());
        for reference in references {
            self.notes.content.step(reference.source().owner())?;
            if reference.first_item_index() < range.start
                || reference.last_item_index() >= range.end
            {
                return Ok(false);
            }
        }
        kernel::require(
            &mut self.notes.content,
            &mut state.definitions,
            &mut state.pending,
            references,
        )?;
        Ok(true)
    }
    fn require_table(
        &mut self,
        state: &mut Self::State,
        fragment: &Self::TableFragment,
    ) -> Result<bool, ProductionBodyPaginationError> {
        let Some(references) = self.notes.definition_table_references(fragment)? else {
            return Ok(false);
        };
        for reference in references {
            kernel::require(
                &mut self.notes.content,
                &mut state.definitions,
                &mut state.pending,
                std::slice::from_ref(reference),
            )?;
        }
        Ok(true)
    }
    fn table_height(fragment: &Self::TableFragment) -> Length {
        fragment.used_height()
    }
    fn table_forced(fragment: &Self::TableFragment) -> Option<NodeId> {
        fragment.forced_break_owner()
    }
    fn request(request: &Self::Request) -> mixed_kernel::Request<Self::Cursor> {
        match *request {
            BookV2DefinitionCandidatePart::Items { end } => mixed_kernel::Request::Items { end },
            BookV2DefinitionCandidatePart::Table { cursor, capacity } => {
                mixed_kernel::Request::Table { cursor, capacity }
            }
            BookV2DefinitionCandidatePart::Forced => mixed_kernel::Request::Forced,
        }
    }
    fn has_tables(&self) -> bool {
        true
    }
    fn source_keep_before(
        &mut self,
        item: usize,
        next: Option<usize>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.step(NodeId::new(0))?;
        if let Some(previous) = self
            .tables
            .previous_root(next.expect("definition source ordinal"))
        {
            if self
                .tables
                .range(previous)
                .is_some_and(|range| range.end == item)
            {
                return Ok(self.tables.table(previous).keep_with_next());
            }
        }
        Ok(item > 0 && self.items()[item - 1].keep)
    }
    fn table_query_work(&mut self) -> Result<(), ProductionBodyPaginationError> {
        self.step(NodeId::new(0))
    }
    fn ordinary_range(&self, range: Range<usize>, next: Option<usize>) -> bool {
        self.tables
            .range(next.expect("definition source ordinal"))
            .is_none_or(|table| table.start >= range.end)
    }
    fn table_at(&self, item: usize, next: Option<usize>) -> Option<usize> {
        next.filter(|index| {
            self.tables
                .range(*index)
                .is_some_and(|range| range.start == item)
        })
    }
    fn table_range(&self, index: usize) -> Range<usize> {
        self.tables.range(index).expect("bound root table")
    }
    fn next_table(&self, index: usize) -> Result<usize, ProductionBodyPaginationError> {
        self.tables.successor(index)
    }
    fn table_info(&self, index: usize) -> (NodeId, Length, Length, bool) {
        let table = self.tables.table(index);
        (
            table.owner(),
            table.space_before(),
            table.space_after(),
            table.keep_with_next(),
        )
    }
    fn remaining_source(&self, item: usize, next: Option<usize>) -> bool {
        item < self.items().len() || next.expect("definition source ordinal") < self.tables.end()
    }
    fn initial(cursor: Self::Cursor) -> bool {
        cursor.is_initial()
    }
    fn table_index(cursor: Self::Cursor) -> usize {
        cursor.table_index()
    }
    fn terminal(cursor: Self::Cursor) -> bool {
        cursor.is_terminal()
    }
    fn after(fragment: &Self::TableFragment) -> Self::Cursor {
        fragment.after()
    }
    fn evaluate_table(
        &mut self,
        cursor: &Self::Cursor,
        capacity: Length,
    ) -> Result<Option<Self::TableFragment>, ProductionBodyPaginationError> {
        self.tables.evaluate(
            cursor,
            capacity,
            self.notes.headers,
            self.notes
                .active_page_frames
                .and_then(|f| f.footnote())
                .map(|r| r.width()),
            &mut self.notes.content.charge,
            &mut self.notes.content.steps,
        )
    }
    fn part(
        items: Option<Range<usize>>,
        table: Option<Self::TableFragment>,
        top: Length,
        height: Length,
    ) -> Self::Part {
        BookV2DefinitionSelectedPart {
            items,
            table,
            top,
            height,
        }
    }
}

#[path = "book_v2_definition_candidates.rs"]
mod candidates;
pub use candidates::{BookV2DefinitionCandidates, BookV2RankedDefinitionCandidate};

impl<'b, 'f, 's, 'p, 'a> BookV2DefinitionMixedSearch<'b, 'f, 's, 'p, 'a> {
    fn view(&mut self) -> DefinitionSearch<'_, 'b, 'f, 's, 'p, 'a> {
        let available = self.notes.content.maximum_height;
        DefinitionSearch {
            notes: &mut self.notes,
            tables: &mut self.tables,
            definition: self.definition,
            available,
        }
    }
    pub fn record_charge(&self) -> u64 {
        self.notes.record_charge()
    }
    pub fn work_charge(&self) -> u64 {
        self.notes.work_steps()
    }
    pub fn maximum_height(&self) -> Length {
        self.notes.content.maximum_height
    }
    pub fn table_range(&self, index: usize) -> Option<Range<usize>> {
        self.tables.range(index)
    }
    pub fn begin(
        &mut self,
        body: Range<usize>,
    ) -> Result<BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.view().begin(body)
    }
    pub fn begin_table(
        &mut self,
        index: usize,
    ) -> Result<BookV2TableCursor<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.view().begin_table(index)
    }
    pub fn evaluate(
        &mut self,
        state: &BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>,
        requests: &[BookV2DefinitionCandidatePart<'b, 'f, 's, 'p, 'a>],
        available: Length,
    ) -> Result<
        Option<BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.view().evaluate(state, requests, available)
    }
    pub fn enumerate(
        &mut self,
        state: &BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<BookV2DefinitionCandidates<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.view().enumerate(state, available)
    }
    pub fn select(
        &mut self,
        state: &BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.view().select(state, available)
    }
}
