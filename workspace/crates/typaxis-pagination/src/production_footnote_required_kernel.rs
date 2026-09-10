//! Reserve one legal fragment per entry demand before spending capacity on extras.
use super::*;
use region_kernel::{Region, RegionSearch};

pub(super) trait RequiredSearch<'b>: RegionSearch {
    type Cursor: Copy;
    fn pending_count(&self, state: &Self::State) -> usize;
    fn cursor(
        &self,
        state: &Self::State,
        ordinal: usize,
    ) -> Result<Self::Cursor, ProductionBodyPaginationError>;
    fn items(&self, cursor: Self::Cursor) -> &'b [ProductionBodyFlowItem];
    fn owner(&self, cursor: Self::Cursor) -> NodeId;
    fn start(cursor: Self::Cursor) -> usize;
    fn content(&mut self) -> FootnoteSearchKernel<'_>;
    fn selected(
        &self,
        state: &Self::State,
        cursor: Self::Cursor,
        projection: FootnoteFragmentProjection,
    ) -> Self::Selection;
    fn continues(selected: &Self::Selection) -> bool;
    fn advance_required(
        &mut self,
        state: &Self::State,
        selected: &Self::Selection,
        position: usize,
    ) -> Result<Self::State, ProductionBodyPaginationError>;
}
struct Plan<C> {
    cursor: C,
    choice: Option<page_breaks::BoundarySelection>,
    minimum: Length,
    suffix: Length,
}
pub(super) fn select<'b, S: RequiredSearch<'b>>(
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
    let count = search.pending_count(state);
    if count == 0 {
        return Ok(None);
    }
    search.charge(count, root)?;
    let mut plans = Vec::new();
    plans
        .try_reserve_exact(count)
        .map_err(|_| error(root, E::AllocationFailure))?;
    for ordinal in 0..count {
        let cursor = search.cursor(state, ordinal)?;
        let items = search.items(cursor);
        let start = S::start(cursor);
        let owner = items[start].owner;
        search.step(owner)?;
        let choice = if items[start].source.is_none() {
            if ordinal + 1 < count {
                return Ok(None);
            }
            None
        } else {
            let content = search.content();
            let mut choice = match page_breaks::all_boundaries(
                items,
                start,
                available,
                content.paragraph_lengths,
                content.headings,
                content.maximum_candidates,
                content.charge,
                |owner| visit(content.steps, content.maximum_steps, owner),
            ) {
                Err(e) if e.kind == E::Oversize && available < content.maximum_height => {
                    return Ok(None)
                }
                result => result?,
            };
            if ordinal + 1 < count {
                if choice.candidates.last().is_some_and(|c| {
                    items
                        .get(c.end_item() as usize)
                        .is_some_and(|i| i.source.is_none())
                }) {
                    choice.candidates.pop();
                }
                if choice.candidates.is_empty() {
                    return Ok(None);
                }
            }
            Some(choice)
        };
        let minimum = choice
            .as_ref()
            .map_or(Length::ZERO, |c| c.candidates[0].used_height());
        plans.push(Plan {
            cursor,
            choice,
            minimum,
            suffix: Length::ZERO,
        });
    }
    for i in (0..plans.len()).rev() {
        let plan = &plans[i];
        let owner = search.owner(plan.cursor);
        search.step(owner)?;
        let rest = if i + 1 < plans.len() {
            let next = &plans[i + 1];
            let next_item = &search.items(next.cursor)[S::start(next.cursor)];
            let gap = if next_item.source.is_some() {
                let end = plan
                    .choice
                    .as_ref()
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                    .candidates[0]
                    .end_item() as usize;
                add(
                    search.items(plan.cursor)[end - 1].after,
                    next_item.before,
                    owner,
                )?
            } else {
                Length::ZERO
            };
            add(gap, next.suffix, owner)?
        } else {
            Length::ZERO
        };
        plans[i].suffix = add(plans[i].minimum, rest, owner)?;
    }
    if plans[0].suffix > available {
        return Ok(None);
    }
    let mut current = None;
    let mut fragments = Vec::new();
    search.charge(plans.len(), root)?;
    fragments
        .try_reserve_exact(plans.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let mut used = Length::ZERO;
    let mut last_after = None;
    let mut pending_position = 0;
    let mut forced_break_owner = None;
    for ordinal in 0..plans.len() {
        let next_info = plans.get(ordinal + 1).map(|p| {
            let item = &search.items(p.cursor)[S::start(p.cursor)];
            (p.suffix, item.before, item.source.is_some())
        });
        let plan = &mut plans[ordinal];
        let before = current.as_ref().unwrap_or(state);
        let items = search.items(plan.cursor);
        let start = S::start(plan.cursor);
        let first = &items[start];
        let gap = if first.source.is_some() {
            last_after
                .map(|after| add(after, first.before, first.owner))
                .transpose()?
                .unwrap_or(Length::ZERO)
        } else {
            Length::ZERO
        };
        let offset = add(used, gap, first.owner)?;
        let capacity = available
            .checked_sub(offset)
            .ok_or_else(|| error(first.owner, E::ArithmeticOverflow))?;
        let projection = if let Some(mut choice) = plan.choice.take() {
            let mut best = None;
            let mut kept = 0;
            for index in 0..choice.candidates.len() {
                let candidate = choice.candidates[index];
                search.step(candidate.owner())?;
                let end = candidate.end_item() as usize;
                let rest = match next_info {
                    Some((suffix, next_before, true)) => add(
                        add(items[end - 1].after, next_before, first.owner)?,
                        suffix,
                        first.owner,
                    )?,
                    Some((suffix, _, false)) => suffix,
                    None => Length::ZERO,
                };
                if add(candidate.used_height(), rest, first.owner)? <= capacity {
                    let content = search.content();
                    let candidate = page_breaks::candidate(
                        items,
                        start,
                        end,
                        candidate.used_height(),
                        capacity,
                        content.paragraph_lengths,
                        content.headings,
                        end == items.len() || items[end].source.is_none(),
                    )?;
                    choice.candidates[kept] = candidate;
                    let key = (candidate.costs().total(), candidate.end_item());
                    if best.is_none_or(|(_, previous)| key < previous) {
                        best = Some((kept, key));
                    }
                    kept += 1;
                }
            }
            choice.candidates.truncate(kept);
            let index = best
                .ok_or_else(|| error(first.owner, E::ReceiptMismatch))?
                .0;
            choice.selected_candidate = index as u32;
            let selected = choice.candidates[index];
            let end = selected.end_item() as usize;
            let reason = if end == items.len() {
                ProductionBodyBreakReason::End
            } else if items[end].source.is_none() {
                ProductionBodyBreakReason::Forced
            } else {
                ProductionBodyBreakReason::Overflow
            };
            choice.reason = reason;
            let forced = (reason == ProductionBodyBreakReason::Forced).then(|| items[end].owner);
            if let Some(owner) = forced {
                search.step(owner)?;
            }
            search.charge(1, first.owner)?;
            FootnoteFragmentProjection {
                content_end: end,
                consumed_end: end + usize::from(forced.is_some()),
                used_height: selected.used_height(),
                available_height: capacity,
                reason,
                forced_break_owner: forced,
                choice: Some(choice),
            }
        } else {
            search
                .content()
                .evaluate(items, start, capacity)?
                .ok_or_else(|| error(first.owner, E::ReceiptMismatch))?
        };
        search.charge(1, first.owner)?;
        used = add(offset, projection.used_height, first.owner)?;
        forced_break_owner = projection.forced_break_owner;
        last_after = items[start..projection.content_end]
            .last()
            .map(|item| item.after);
        let selected = search.selected(before, plan.cursor, projection);
        let after = search.advance_required(before, &selected, pending_position)?;
        if S::continues(&selected) {
            pending_position += 1;
        }
        fragments.push(S::fragment(offset, selected));
        current = Some(after);
    }
    Ok(Some(Region {
        fragments,
        used_height: used,
        forced_break_owner,
        next_state: current.ok_or_else(|| error(root, E::ReceiptMismatch))?,
    }))
}
