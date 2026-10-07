//! Shared region selection; concrete owners retain their own state and cursor types.
use super::*;

pub(super) trait RegionSearch {
    type State;
    type Selection;
    type Fragment;
    fn verify(&self, state: &Self::State) -> Result<(), ProductionBodyPaginationError>;
    fn maximum_height(&self) -> Length;
    fn charge(&mut self, count: usize, owner: NodeId) -> Result<(), ProductionBodyPaginationError>;
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError>;
    fn next(
        &self,
        state: &Self::State,
    ) -> Result<Option<(NodeId, bool, Length)>, ProductionBodyPaginationError>;
    fn evaluate(
        &mut self,
        state: &Self::State,
        available: Length,
    ) -> Result<Option<Self::Selection>, ProductionBodyPaginationError>;
    fn geometry(
        selected: &Self::Selection,
    ) -> (
        Length,
        ProductionBodyBreakReason,
        Option<NodeId>,
        Option<Length>,
    );
    fn advance(
        &mut self,
        state: &Self::State,
        selected: &Self::Selection,
    ) -> Result<Self::State, ProductionBodyPaginationError>;
    fn fragment(offset: Length, selected: Self::Selection) -> Self::Fragment;
}
pub(super) struct Region<S: RegionSearch> {
    pub fragments: Vec<S::Fragment>,
    pub used_height: Length,
    pub forced_break_owner: Option<NodeId>,
    pub next_state: S::State,
}
pub(super) fn select<S: RegionSearch>(
    search: &mut S,
    state: &S::State,
    available: Length,
) -> Result<Option<Region<S>>, ProductionBodyPaginationError> {
    search.verify(state)?;
    let root = NodeId::new(0);
    if available < Length::ZERO || available > search.maximum_height() {
        return Err(error(root, E::InvalidFootnoteCapacity));
    }
    search.charge(1, root)?;
    let mut fragments = Vec::new();
    let mut current = None;
    let mut used = Length::ZERO;
    let mut last_after = None;
    let mut forced_break_owner = None;
    loop {
        let before = current.as_ref().unwrap_or(state);
        let Some((owner, paint, spacing_before)) = search.next(before)? else {
            break;
        };
        search.step(owner)?;
        let gap = if paint {
            last_after
                .map(|after| add(after, spacing_before, owner))
                .transpose()?
                .unwrap_or(Length::ZERO)
        } else {
            Length::ZERO
        };
        let offset = add(used, gap, owner)?;
        if offset > available {
            break;
        }
        let remaining = available
            .checked_sub(offset)
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        let Some(selected) = search.evaluate(before, remaining)? else {
            break;
        };
        let (height, reason, forced, after_spacing) = S::geometry(&selected);
        let end = add(offset, height, owner)?;
        if end > available {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let after = search.advance(before, &selected)?;
        search.charge(1, owner)?;
        fragments
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        fragments.push(S::fragment(offset, selected));
        current = Some(after);
        used = end;
        last_after = after_spacing;
        forced_break_owner = forced;
        if reason != ProductionBodyBreakReason::End {
            break;
        }
    }
    Ok(current.map(|next_state| Region {
        fragments,
        used_height: used,
        forced_break_owner,
        next_state,
    }))
}

impl<'b, 'f, 's, 'p, 'a> RegionSearch for ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    type State = ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>;
    type Selection = ProductionFootnoteDemandSelection<'b, 'f, 's, 'p, 'a>;
    type Fragment = ProductionFootnoteRegionFragment<'b, 'f, 's, 'p, 'a>;
    fn verify(&self, state: &Self::State) -> Result<(), ProductionBodyPaginationError> {
        self.verify_state(state)
    }
    fn maximum_height(&self) -> Length {
        self.content.maximum_height
    }
    fn charge(&mut self, count: usize, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.content.charge.take(count, owner)
    }
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.step(owner)
    }
    fn next(
        &self,
        state: &Self::State,
    ) -> Result<Option<(NodeId, bool, Length)>, ProductionBodyPaginationError> {
        let Some(index) = state.pending.first().copied() else {
            return Ok(None);
        };
        let Demand::Pending { cursor, .. } = state.definitions[index] else {
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
            f.items().last().map(|item| item.after),
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
        ProductionFootnoteRegionFragment { offset, selected }
    }
}
