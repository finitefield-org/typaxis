//! Exhaustive bounded definition candidates, ranked before reservation.
use super::*;

use std::cmp::Reverse;
// Equal-cost candidates consume the furthest source position. This includes
// zero-height siblings, which must not force another region solely for a tie.
type Key = (
    i64,
    Reverse<usize>,
    Reverse<usize>,
    Reverse<i64>,
    Reverse<usize>,
);
type Boundary<'b, 'f, 's, 'p, 'a> =
    page_ranking::Boundary<BookV2DefinitionCandidatePart<'b, 'f, 's, 'p, 'a>, Key>;

pub struct BookV2RankedDefinitionCandidate<'b, 'f, 's, 'p, 'a> {
    cost: i64,
    candidate: BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2RankedDefinitionCandidate<'b, 'f, 's, 'p, 'a> {
    pub fn cost(&self) -> i64 {
        self.cost
    }
    pub fn candidate(&self) -> &BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a> {
        &self.candidate
    }
    pub fn into_candidate(self) -> BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a> {
        self.candidate
    }
}
/// Every retained choice is bound to the same incoming definition/demand snapshot.
/// Reservation owners may backtrack through these actual source choices.
pub struct BookV2DefinitionCandidates<'b, 'f, 's, 'p, 'a> {
    source: (u64, u64),
    definition: usize,
    available: Length,
    examined: u32,
    choices: Vec<BookV2RankedDefinitionCandidate<'b, 'f, 's, 'p, 'a>>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2DefinitionCandidates<'b, 'f, 's, 'p, 'a> {
    pub fn definition_index(&self) -> usize {
        self.definition
    }
    pub fn available_height(&self) -> Length {
        self.available
    }
    pub fn examined_boundaries(&self) -> u32 {
        self.examined
    }
    pub fn choices(&self) -> &[BookV2RankedDefinitionCandidate<'b, 'f, 's, 'p, 'a>] {
        &self.choices
    }
    pub fn into_choices(
        self,
    ) -> std::vec::IntoIter<BookV2RankedDefinitionCandidate<'b, 'f, 's, 'p, 'a>> {
        self.choices.into_iter()
    }
    pub fn into_best(self) -> Option<BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>> {
        self.choices
            .into_iter()
            .next()
            .map(|choice| choice.candidate)
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
        if self.source != state.demand.table_snapshot_id() || self.definition != state.definition {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}
impl<'b, 'f, 's, 'p, 'a> DefinitionSearch<'_, 'b, 'f, 's, 'p, 'a> {
    fn boundary_key(
        &self,
        end: usize,
        next_table: usize,
        continuation: Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>>,
        last_items: Option<Range<usize>>,
        used: Length,
        available: Length,
        forced: bool,
    ) -> Result<Key, ProductionBodyPaginationError> {
        let terminal = forced
            || (continuation.is_none()
                && end == self.items().len()
                && next_table == self.tables.end());
        let cost = if available == Length::ZERO {
            if used != Length::ZERO {
                return Err(error(NodeId::new(0), E::ReceiptMismatch));
            }
            0
        } else {
            page_ranking::cost(
                self.items(),
                last_items,
                used,
                available,
                &self.notes.content.paragraph_lengths,
                &self.notes.content.headings,
                terminal,
            )?
        };
        Ok((
            cost,
            Reverse(end),
            Reverse(next_table),
            Reverse(continuation.map_or(0, |c| c.offset().raw())),
            Reverse(continuation.map_or(0, |c| c.next_caption_item())),
        ))
    }
    fn queue_boundary(
        &mut self,
        requests: &[BookV2DefinitionCandidatePart<'b, 'f, 's, 'p, 'a>],
        key: Key,
        alternatives: &mut Vec<Boundary<'b, 'f, 's, 'p, 'a>>,
    ) -> Result<(), ProductionBodyPaginationError> {
        page_ranking::queue(
            requests,
            key,
            alternatives,
            self.notes.content.maximum_candidates,
            &mut self.notes.content.charge,
            &mut self.notes.content.steps,
            self.notes.content.maximum_steps,
        )
    }
    fn serial_break(&self, item: usize, next_table: usize) -> bool {
        !self
            .tables
            .range(next_table)
            .is_some_and(|range| range.start == item)
            && self
                .items()
                .get(item)
                .is_some_and(|item| item.source.is_none())
    }
    fn push_request(
        &mut self,
        requests: &mut Vec<BookV2DefinitionCandidatePart<'b, 'f, 's, 'p, 'a>>,
        request: BookV2DefinitionCandidatePart<'b, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        self.notes.content.charge.take(1, root)?;
        requests
            .try_reserve(1)
            .map_err(|_| error(root, E::AllocationFailure))?;
        requests.push(request);
        Ok(())
    }
    /// Enumerate every ordinary boundary and legal final table cut under the
    /// requested capacity. Retain all feasible branches for later reservation.
    pub fn enumerate(
        &mut self,
        state: &BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<BookV2DefinitionCandidates<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.notes.verify_state(&state.demand)?;
        let root = NodeId::new(0);
        if state.definition != self.definition
            || available < Length::ZERO
            || available > self.maximum_height()
        {
            return Err(error(
                root,
                if state.definition != self.definition {
                    E::ReceiptMismatch
                } else {
                    E::InvalidFootnoteCapacity
                },
            ));
        }
        self.notes.content.charge.take(2, root)?;
        let mut result = BookV2DefinitionCandidates {
            source: state.demand.table_snapshot_id(),
            definition: self.definition,
            available,
            examined: 0,
            choices: Vec::new(),
        };
        if state.is_complete() {
            return Ok(result);
        }
        let mut alternatives = Vec::new();
        let mut requests = Vec::new();
        let mut item = state.item;
        let mut next_table = state.next_table;
        let mut used = Length::ZERO;
        let mut after = Length::ZERO;
        let mut ordinary = false;
        let mut ordinary_start = item;
        let items = self
            .notes
            .content
            .flow
            .definition_items(self.definition)
            .expect("bound definition");
        loop {
            self.notes.content.step(root)?;
            if let Some(range) = self
                .tables
                .range(next_table)
                .filter(|range| range.start == item)
            {
                let cursor = match state
                    .continuation
                    .filter(|_| next_table == state.next_table)
                {
                    Some(c) => c,
                    None => self.begin_table(next_table)?,
                };
                let table = self.tables.table(next_table);
                let owner = table.owner();
                let before = table.space_before();
                let table_after = table.space_after();
                let keep = table.keep_with_next();
                let successor = self.tables.successor(next_table)?;
                let top = if requests.is_empty() {
                    used
                } else {
                    add(used, add(after, before, owner)?, owner)?
                };
                if top > available {
                    break;
                }
                let mut capacity = available
                    .checked_sub(top)
                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                self.push_request(
                    &mut requests,
                    BookV2DefinitionCandidatePart::Table { cursor, capacity },
                )?;
                let mut complete = None;
                loop {
                    let Some(selected) = self.tables.evaluate(
                        &cursor,
                        capacity,
                        self.notes.headers,
                        self.notes
                            .active_page_frames
                            .and_then(|f| f.footnote())
                            .map(|r| r.width()),
                        &mut self.notes.content.charge,
                        &mut self.notes.content.steps,
                    )?
                    else {
                        break;
                    };
                    let smaller = self.tables.earlier_capacity(
                        &cursor,
                        &selected,
                        &mut self.notes.content.steps,
                    )?;
                    let terminal = selected.after().is_terminal();
                    let forced = selected.forced_break_owner().is_some();
                    if terminal {
                        complete = Some((capacity, selected.used_height(), forced));
                    }
                    *requests.last_mut().unwrap() =
                        BookV2DefinitionCandidatePart::Table { cursor, capacity };
                    let remaining = range.end < items.len() || successor < self.tables.end();
                    if !terminal || !keep || !remaining {
                        let end = if terminal { range.end } else { item };
                        let ordinal = if terminal { successor } else { next_table };
                        // A following serial break is consumed by the complete
                        // prefix path below, never deferred to a spurious blank.
                        if !terminal || forced || !self.serial_break(end, ordinal) {
                            let key = self.boundary_key(
                                end,
                                ordinal,
                                (!terminal).then_some(selected.after()),
                                None,
                                add(top, selected.used_height(), owner)?,
                                available,
                                forced,
                            )?;
                            self.queue_boundary(&requests, key, &mut alternatives)?;
                        }
                    }
                    let Some(smaller) = smaller else {
                        break;
                    };
                    if smaller >= capacity {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    capacity = smaller;
                }
                let Some((capacity, height, forced)) = complete else {
                    break;
                };
                if forced {
                    break;
                }
                *requests.last_mut().unwrap() =
                    BookV2DefinitionCandidatePart::Table { cursor, capacity };
                used = add(top, height, owner)?;
                after = table_after;
                item = range.end;
                next_table = successor;
                ordinary = false;
                continue;
            }
            let Some(current) = items.get(item) else {
                break;
            };
            if current.source.is_none() {
                self.push_request(&mut requests, BookV2DefinitionCandidatePart::Forced)?;
                let key =
                    self.boundary_key(item + 1, next_table, None, None, used, available, true)?;
                self.queue_boundary(&requests, key, &mut alternatives)?;
                break;
            }
            let gap = if requests.is_empty() {
                Length::ZERO
            } else {
                add(after, current.before, current.owner)?
            };
            let next = add(
                add(used, gap, current.owner)?,
                current.consumed()?,
                current.owner,
            )?;
            if next > available {
                break;
            }
            if !ordinary {
                ordinary_start = item;
                self.push_request(
                    &mut requests,
                    BookV2DefinitionCandidatePart::Items { end: item + 1 },
                )?;
                ordinary = true;
            } else {
                *requests.last_mut().unwrap() =
                    BookV2DefinitionCandidatePart::Items { end: item + 1 };
            }
            used = next;
            after = current.after;
            item += 1;
            if (!current.keep || (item == items.len() && next_table == self.tables.end()))
                && !self.serial_break(item, next_table)
            {
                let key = self.boundary_key(
                    item,
                    next_table,
                    None,
                    Some(ordinary_start..item),
                    used,
                    available,
                    false,
                )?;
                self.queue_boundary(&requests, key, &mut alternatives)?;
            }
        }
        result.examined =
            u32::try_from(alternatives.len()).map_err(|_| error(root, E::FragmentLimit))?;
        for boundary in alternatives {
            self.notes.content.step(root)?;
            let Some(candidate) = self.evaluate(state, &boundary.requests, available)? else {
                continue;
            };
            let next = candidate.next_state();
            let key = self.boundary_key(
                next.next_item(),
                next.next_table_index(),
                next.table_continuation(),
                candidate.parts().last().and_then(|part| part.items()),
                candidate.used_height(),
                available,
                candidate.forced_break_owner().is_some(),
            )?;
            if key != boundary.key {
                return Err(error(root, E::ReceiptMismatch));
            }
            self.notes.content.charge.take(1, root)?;
            result
                .choices
                .try_reserve(1)
                .map_err(|_| error(root, E::AllocationFailure))?;
            result.choices.push(BookV2RankedDefinitionCandidate {
                cost: key.0,
                candidate,
            });
        }
        Ok(result)
    }
    pub fn select(
        &mut self,
        state: &BookV2DefinitionSourceState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        Ok(self.enumerate(state, available)?.into_best())
    }
}
