//! Dedicated successor multi-definition region ownership.
use super::*;

/// A content fragment at a measured offset inside a candidate footnote region.
/// No physical page origin or PDF paint permission is assigned here.
pub struct BookV2FootnoteRegionFragment<'b, 'f, 's, 'p, 'a> {
    offset: Length,
    selected: BookV2FootnoteDemandSelection<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteRegionFragment<'b, 'f, 's, 'p, 'a> {
    pub const fn offset(&self) -> Length {
        self.offset
    }
    pub fn fragment(&self) -> &BookV2FootnoteFragmentSelection<'b, 'f, 's, 'p, 'a> {
        self.selected.fragment()
    }
}

pub struct BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a> {
    owner_id: u64,
    state_id: u64,
    fragments: Vec<BookV2FootnoteRegionFragment<'b, 'f, 's, 'p, 'a>>,
    used_height: Length,
    available_height: Length,
    forced_break_owner: Option<NodeId>,
    next_state: BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a> {
    pub fn fragments(&self) -> &[BookV2FootnoteRegionFragment<'b, 'f, 's, 'p, 'a>] {
        &self.fragments
    }
    pub const fn used_height(&self) -> Length {
        self.used_height
    }
    pub const fn available_height(&self) -> Length {
        self.available_height
    }
    pub const fn forced_break_owner(&self) -> Option<NodeId> {
        self.forced_break_owner
    }
    pub fn next_state(&self) -> &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a> {
        &self.next_state
    }
    pub fn into_next_state(self) -> BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a> {
        self.next_state
    }
    pub fn verify(
        &self,
        state: &BookV2FootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.owner_id != state.owner_id
            || self.state_id != state.state_id
            || !std::ptr::eq(self.next_state.flow, state.flow)
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Select pending content in order, including authored inter-definition
    /// spacing. Overflow/forced boundaries end this region. Remaining demands
    /// stay in next_state; the body-page owner must decide whether they make its
    /// candidate invalid. This method alone does not establish same-page fit.
    pub fn select_region(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        Ok(
            region_kernel::select(self, state, available)?.map(|region| {
                BookV2FootnoteRegionSelection {
                    owner_id: self.owner_id,
                    state_id: state.state_id,
                    fragments: region.fragments,
                    used_height: region.used_height,
                    available_height: available,
                    forced_break_owner: region.forced_break_owner,
                    next_state: region.next_state,
                }
            }),
        )
    }
}

impl<'b, 'f, 's, 'p, 'a> region_kernel::RegionSearch
    for BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>
{
    type State = BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>;
    type Selection = BookV2FootnoteDemandSelection<'b, 'f, 's, 'p, 'a>;
    type Fragment = BookV2FootnoteRegionFragment<'b, 'f, 's, 'p, 'a>;
    fn verify(&self, state: &Self::State) -> Result<(), ProductionBodyPaginationError> {
        self.verify_state(state)
    }
    fn maximum_height(&self) -> Length {
        self.content.maximum_height
    }
    fn charge(&mut self, count: usize, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.content.charge(count, owner)
    }
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.content.step(owner)
    }
    fn next(
        &self,
        state: &Self::State,
    ) -> Result<Option<(NodeId, bool, Length)>, ProductionBodyPaginationError> {
        if self.definition_tables.is_some() {
            return self.mixed_definition_start(state);
        }
        let Some(index) = state.pending.first().copied() else {
            return Ok(None);
        };
        let BookV2Demand::Pending { cursor, .. } = state.definitions[index] else {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        };
        let next = state
            .flow
            .definition_items(index)
            .and_then(|items| items.get(cursor.next_item()))
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        Ok(Some((next.owner, next.source.is_some(), next.before)))
    }
    fn evaluate(
        &mut self,
        state: &Self::State,
        available: Length,
    ) -> Result<Option<Self::Selection>, ProductionBodyPaginationError> {
        self.evaluate_next(state, available)
    }
    fn geometry(
        selected: &Self::Selection,
    ) -> (
        Length,
        ProductionBodyBreakReason,
        Option<NodeId>,
        Option<Length>,
    ) {
        let f = selected.fragment();
        (
            f.used_height(),
            f.reason(),
            f.forced_break_owner(),
            f.space_after(),
        )
    }
    fn advance(
        &mut self,
        state: &Self::State,
        selected: &Self::Selection,
    ) -> Result<Self::State, ProductionBodyPaginationError> {
        self.advance(state, selected)
    }
    fn fragment(offset: Length, selected: Self::Selection) -> Self::Fragment {
        BookV2FootnoteRegionFragment { offset, selected }
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Reserve a legal first fragment for each definition pending at entry.
    /// Nested demands remain explicit for the joint page owner to resolve.
    /// A forced boundary cannot precede another selected definition.
    pub fn select_required_region(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        if self.definition_tables.is_some() {
            return self.select_definition_reservation(state, available, false);
        }
        Ok(
            required_kernel::select(self, state, available)?.map(|region| {
                BookV2FootnoteRegionSelection {
                    owner_id: self.owner_id,
                    state_id: state.state_id,
                    fragments: region.fragments,
                    used_height: region.used_height,
                    available_height: available,
                    forced_break_owner: region.forced_break_owner,
                    next_state: region.next_state,
                }
            }),
        )
    }
}

impl<'b, 'f, 's, 'p, 'a> required_kernel::RequiredSearch<'b>
    for BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>
{
    type Cursor = BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a>;
    fn pending_count(&self, state: &Self::State) -> usize {
        state.pending.len()
    }
    fn cursor(
        &self,
        state: &Self::State,
        ordinal: usize,
    ) -> Result<Self::Cursor, ProductionBodyPaginationError> {
        let index = state.pending[ordinal];
        let BookV2Demand::Pending { cursor, .. } = state.definitions[index] else {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        };
        Ok(cursor)
    }
    fn items(&self, cursor: Self::Cursor) -> &'b [ProductionBodyFlowItem] {
        self.content
            .flow
            .definition_items(cursor.definition_index())
            .expect("bound definition")
    }
    fn owner(&self, cursor: Self::Cursor) -> NodeId {
        self.content.flow.footnotes().definitions()[cursor.definition_index()].owner()
    }
    fn start(cursor: Self::Cursor) -> usize {
        cursor.next_item()
    }
    fn content(&mut self) -> FootnoteSearchKernel<'_> {
        self.content.kernel()
    }
    fn selected(
        &self,
        state: &Self::State,
        cursor: Self::Cursor,
        projection: FootnoteFragmentProjection,
    ) -> Self::Selection {
        BookV2FootnoteDemandSelection {
            owner_id: self.owner_id,
            state_id: state.state_id,
            fragment: BookV2FootnoteFragmentSelection::from_projection(cursor, projection),
        }
    }
    fn continues(selected: &Self::Selection) -> bool {
        selected.fragment().continuation().is_some()
    }
    fn advance_required(
        &mut self,
        state: &Self::State,
        selected: &Self::Selection,
        position: usize,
    ) -> Result<Self::State, ProductionBodyPaginationError> {
        self.advance_at(state, selected, position)
    }
}

#[path = "book_v2_footnote_dependency_region.rs"]
mod dependency_region;
