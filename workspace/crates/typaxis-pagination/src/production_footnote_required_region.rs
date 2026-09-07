//! Reserve a legal first fragment for every pending definition before extras.
use super::*;

struct Plan<'b, 'f, 's, 'p, 'a> {
    cursor: ProductionFootnoteCursor<'b, 'f, 's, 'p, 'a>,
    choice: Option<page_breaks::BoundarySelection>,
    minimum: Length,
    suffix: Length,
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Every definition already pending at entry receives a legal fragment.
    /// Later nested demands remain explicit in next_state; this is not a page
    /// receipt. Hard forced boundaries cannot precede another selected note.
    pub fn select_required_region(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<ProductionFootnoteRegionSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.verify_state(state)?;
        let root = NodeId::new(0);
        if available < Length::ZERO || available > self.content.maximum_height {
            return Err(error(root, E::InvalidFootnoteCapacity));
        }
        self.content.charge.take(1, root)?;
        if state.pending.is_empty() {
            return Ok(None);
        }
        self.content.charge.take(state.pending.len(), root)?;
        let mut plans = Vec::new();
        plans
            .try_reserve_exact(state.pending.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        for (ordinal, index) in state.pending.iter().copied().enumerate() {
            let Demand::Pending { cursor, .. } = state.definitions[index] else {
                return Err(error(root, E::ReceiptMismatch));
            };
            let items = state
                .flow
                .definition_items(index)
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            let owner = items[cursor.next_item].owner;
            self.step(owner)?;
            let choice = if items[cursor.next_item].source.is_none() {
                if ordinal + 1 < state.pending.len() {
                    return Ok(None);
                }
                None
            } else {
                let content = &mut self.content;
                let mut choice = match page_breaks::all_boundaries(
                    items,
                    cursor.next_item,
                    available,
                    &content.paragraph_lengths,
                    &content.headings,
                    content.maximum_candidates,
                    &mut content.charge,
                    |owner| visit(&mut content.steps, content.maximum_steps, owner),
                ) {
                    Err(e) if e.kind == E::Oversize && available < content.maximum_height => {
                        return Ok(None)
                    }
                    result => result?,
                };
                if ordinal + 1 < state.pending.len() {
                    // A consumed forced break ends the whole footnote region.
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
            let owner = state.flow.footnotes.definitions()[plan.cursor.definition_index].owner();
            self.step(owner)?;
            let rest = if i + 1 < plans.len() {
                let next = &plans[i + 1];
                let next_item = &state
                    .flow
                    .definition_items(next.cursor.definition_index)
                    .unwrap()[next.cursor.next_item];
                let gap = if next_item.source.is_some() {
                    let end = plan
                        .choice
                        .as_ref()
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                        .candidates[0]
                        .end_item() as usize;
                    add(
                        state
                            .flow
                            .definition_items(plan.cursor.definition_index)
                            .unwrap()[end - 1]
                            .after,
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
        let mut fragments: Vec<ProductionFootnoteRegionFragment<'b, 'f, 's, 'p, 'a>> = Vec::new();
        self.content.charge.take(plans.len(), root)?;
        fragments
            .try_reserve_exact(plans.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut used = Length::ZERO;
        let mut pending_position = 0;
        let mut forced_break_owner = None;
        for ordinal in 0..plans.len() {
            let next_info = plans.get(ordinal + 1).map(|p| {
                (
                    p.suffix,
                    state
                        .flow
                        .definition_items(p.cursor.definition_index)
                        .unwrap()[p.cursor.next_item]
                        .before,
                    state
                        .flow
                        .definition_items(p.cursor.definition_index)
                        .unwrap()[p.cursor.next_item]
                        .source
                        .is_some(),
                )
            });
            let plan = &mut plans[ordinal];
            let before = current.as_ref().unwrap_or(state);
            let items = state
                .flow
                .definition_items(plan.cursor.definition_index)
                .unwrap();
            let first = &items[plan.cursor.next_item];
            let gap = if first.source.is_some() {
                fragments
                    .last()
                    .and_then(|f| f.fragment().items().last())
                    .map(|last| add(last.after, first.before, first.owner))
                    .transpose()?
                    .unwrap_or(Length::ZERO)
            } else {
                Length::ZERO
            };
            let offset = add(used, gap, first.owner)?;
            let capacity = available
                .checked_sub(offset)
                .ok_or_else(|| error(first.owner, E::ArithmeticOverflow))?;
            let fragment = if let Some(mut choice) = plan.choice.take() {
                let mut best = None;
                let mut kept = 0;
                for index in 0..choice.candidates.len() {
                    let candidate = choice.candidates[index];
                    self.step(candidate.owner())?;
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
                        let candidate = page_breaks::candidate(
                            items,
                            plan.cursor.next_item,
                            end,
                            candidate.used_height(),
                            capacity,
                            &self.content.paragraph_lengths,
                            &self.content.headings,
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
                let forced =
                    (reason == ProductionBodyBreakReason::Forced).then(|| items[end].owner);
                if let Some(owner) = forced {
                    self.step(owner)?;
                }
                self.content.charge.take(1, first.owner)?;
                ProductionFootnoteFragmentSelection {
                    cursor: plan.cursor,
                    content_end: end,
                    consumed_end: end + usize::from(forced.is_some()),
                    used_height: selected.used_height(),
                    available_height: capacity,
                    reason,
                    forced_break_owner: forced,
                    choice: Some(choice),
                }
            } else {
                self.content
                    .evaluate(&plan.cursor, capacity)?
                    .ok_or_else(|| error(first.owner, E::ReceiptMismatch))?
            };
            self.content.charge.take(1, first.owner)?;
            used = add(offset, fragment.used_height, first.owner)?;
            forced_break_owner = fragment.forced_break_owner;
            let continues = fragment.continuation().is_some();
            let selected = ProductionFootnoteDemandSelection {
                owner_id: self.owner_id,
                state_id: before.state_id,
                fragment,
            };
            let after = self.advance_at(before, &selected, pending_position)?;
            if continues {
                pending_position += 1;
            }
            fragments.push(ProductionFootnoteRegionFragment { offset, selected });
            current = Some(after);
        }
        Ok(Some(ProductionFootnoteRegionSelection {
            owner_id: self.owner_id,
            state_id: state.state_id,
            fragments,
            used_height: used,
            available_height: available,
            forced_break_owner,
            next_state: current.ok_or_else(|| error(root, E::ReceiptMismatch))?,
        }))
    }
}
