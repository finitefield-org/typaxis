//! Required-demand region wrapper over the shared reservation kernel.
use super::*;
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Reserve a legal first fragment for each definition pending at entry.
    /// Nested demands remain explicit for the joint page owner to resolve.
    /// A forced boundary cannot precede another selected definition.
    pub fn select_required_region(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<ProductionFootnoteRegionSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        Ok(
            required_kernel::select(self, state, available)?.map(|region| {
                ProductionFootnoteRegionSelection {
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
    for ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a>
{
    type Cursor = ProductionFootnoteCursor<'b, 'f, 's, 'p, 'a>;
    fn pending_count(&self, state: &Self::State) -> usize {
        state.pending.len()
    }
    fn cursor(
        &self,
        state: &Self::State,
        ordinal: usize,
    ) -> Result<Self::Cursor, ProductionBodyPaginationError> {
        let index = state.pending[ordinal];
        let Demand::Pending { cursor, .. } = state.definitions[index] else {
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
        self.content.flow.footnotes.definitions()[cursor.definition_index()].owner()
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
        ProductionFootnoteDemandSelection {
            owner_id: self.owner_id,
            state_id: state.state_id,
            fragment: ProductionFootnoteFragmentSelection::from_projection(cursor, projection),
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
