//! Automatic page candidates over ordinary ranges and source-bound table cuts.
use super::*;

pub struct ProductionBodyMixedPageState<'b, 'f, 's, 'p, 'a> {
    demand: ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    item: usize,
    table: Option<ProductionTableCursor<'b, 'f, 's, 'p, 'a>>,
    page: u32,
    empty_page: bool,
}
impl<'b, 'f, 's, 'p, 'a> ProductionBodyMixedPageState<'b, 'f, 's, 'p, 'a> {
    pub const fn next_item(&self) -> usize {
        self.item
    }
    pub const fn table_continuation(&self) -> Option<ProductionTableCursor<'b, 'f, 's, 'p, 'a>> {
        self.table
    }
    pub const fn page_index(&self) -> u32 {
        self.page
    }
    pub fn is_complete(&self) -> bool {
        self.item == self.demand.flow.body_items().len()
            && self.table.is_none()
            && self.demand.pending.is_empty()
            && !self.empty_page
    }
}
pub struct ProductionBodyMixedPageSelection<'b, 'f, 's, 'p, 'a> {
    page: u32,
    selected: ProductionBodyMixedCandidate<'b, 'f, 's, 'p, 'a>,
    forced: Option<NodeId>,
    next: ProductionBodyMixedPageState<'b, 'f, 's, 'p, 'a>,
    attempts: u32,
}
impl<'b, 'f, 's, 'p, 'a> ProductionBodyMixedPageSelection<'b, 'f, 's, 'p, 'a> {
    pub const fn page_index(&self) -> u32 {
        self.page
    }
    pub fn candidate(&self) -> &ProductionBodyMixedCandidate<'b, 'f, 's, 'p, 'a> {
        &self.selected
    }
    pub fn next_state(&self) -> &ProductionBodyMixedPageState<'b, 'f, 's, 'p, 'a> {
        &self.next
    }
    pub const fn forced_break(&self) -> Option<NodeId> {
        self.forced
    }
    pub const fn candidate_attempts(&self) -> u32 {
        self.attempts
    }
}
pub struct ProductionBodyMixedPageSequence<'b, 'f, 's, 'p, 'a> {
    owner_id: u64,
    measurements_fingerprint: [u8; 32],
    pages: Vec<ProductionBodyMixedPageSelection<'b, 'f, 's, 'p, 'a>>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionBodyMixedPageSequence<'b, 'f, 's, 'p, 'a> {
    /// Identity of the source-bound cell measurements used by this search.
    /// Page selection is still authorized by the issuing search owner.
    pub const fn measurements_fingerprint(&self) -> [u8; 32] {
        self.measurements_fingerprint
    }
    pub fn pages(&self) -> &[ProductionBodyMixedPageSelection<'b, 'f, 's, 'p, 'a>] {
        &self.pages
    }
}
type MixedBoundary<'b, 'f, 's, 'p, 'a> =
    page_ranking::Boundary<ProductionBodyCandidatePart<'b, 'f, 's, 'p, 'a>, (i64, usize, i64)>;
struct Alternatives<'b, 'f, 's, 'p, 'a> {
    candidates: Vec<MixedBoundary<'b, 'f, 's, 'p, 'a>>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn begin_mixed_pages(
        &mut self,
    ) -> Result<ProductionBodyMixedPageState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        if self.tables.is_none() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        let mut index = 0;
        let items = self.content.flow.body_items();
        while index < items.len() {
            self.step(items[index].owner)?;
            self.table_query_work()?;
            if let Some(table) = self.tables.as_ref().unwrap().at(index) {
                let context = self.tables.as_ref().unwrap();
                let end = context.range(table).end;
                if context.table(table).keep_with_next()
                    && items.get(end).is_some_and(|i| i.source.is_none())
                {
                    return Err(error(
                        context.table(table).owner(),
                        E::KeepAcrossForcedBreak,
                    ));
                }
                index = end;
            } else {
                if items[index].keep && items.get(index + 1).is_some_and(|i| i.source.is_none()) {
                    return Err(error(items[index].owner, E::KeepAcrossForcedBreak));
                }
                index += 1;
            }
        }
        self.content.charge.take(1, NodeId::new(0))?;
        Ok(ProductionBodyMixedPageState {
            demand: self.begin()?,
            item: 0,
            table: None,
            page: 0,
            empty_page: true,
        })
    }
    fn mixed_boundary_key(
        &self,
        end: usize,
        continuation: Option<ProductionTableCursor<'b, 'f, 's, 'p, 'a>>,
        last_items: Option<std::ops::Range<usize>>,
        used: Length,
    ) -> Result<(i64, usize, i64), ProductionBodyPaginationError> {
        let items = self.content.flow.body_items();
        let terminal = continuation.is_none() && items.get(end).is_none_or(|i| i.source.is_none());
        let height = self
            .content
            .flow
            .blocks
            .page_geometry()
            .body()
            .height()
            .get();
        let cost = page_ranking::cost(
            items,
            last_items,
            used,
            height,
            &self.content.paragraph_lengths,
            &self.content.headings,
            terminal,
        )?;
        Ok((cost, end, continuation.map_or(0, |c| c.offset().raw())))
    }
    fn queue_mixed(
        &mut self,
        requests: &[ProductionBodyCandidatePart<'b, 'f, 's, 'p, 'a>],
        key: (i64, usize, i64),
        alternatives: &mut Alternatives<'b, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        page_ranking::queue(
            requests,
            key,
            &mut alternatives.candidates,
            self.content.maximum_candidates,
            &mut self.content.charge,
            &mut self.content.steps,
            self.content.maximum_steps,
        )
    }
    /// Enumerate complete ordinary/table prefixes and each legal final table
    /// cut before testing footnotes in cost order. Exhausting enumeration is
    /// still an error. After the first fit, every remaining key is dominated.
    pub fn select_mixed_page(
        &mut self,
        state: &ProductionBodyMixedPageState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<
        Option<ProductionBodyMixedPageSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.verify_state(&state.demand)?;
        if state.is_complete() {
            return Ok(None);
        }
        let root = NodeId::new(0);
        if state.page >= self.maximum_pages {
            return Err(error(root, E::PageLimit));
        }
        self.content.charge.take(1, root)?;
        let items = self.content.flow.body_items();
        let maximum = self
            .content
            .flow
            .blocks
            .page_geometry()
            .body()
            .height()
            .get();
        let mut alternatives = Alternatives {
            candidates: Vec::new(),
        };
        let mut requests = Vec::new();
        let mut index = state.item;
        let mut used = Length::ZERO;
        let mut after = Length::ZERO;
        let mut ordinary = false;
        let mut ordinary_start = state.item;
        loop {
            self.step(root)?;
            self.table_query_work()?;
            if let Some(table_index) = self.tables.as_ref().unwrap().at(index) {
                let table_cursor = match state.table.filter(|_| index == state.item) {
                    Some(c) => c,
                    None => self.begin_table(table_index)?,
                };
                let context = self.tables.as_ref().unwrap();
                let table = context.table(table_index);
                let gap = if requests.is_empty() {
                    Length::ZERO
                } else {
                    add(after, table.space_before(), table.owner())?
                };
                let top = add(used, gap, table.owner())?;
                if top > maximum {
                    break;
                }
                let mut capacity = maximum
                    .checked_sub(top)
                    .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
                self.content.charge.take(1, root)?;
                requests
                    .try_reserve(1)
                    .map_err(|_| error(root, E::AllocationFailure))?;
                requests.push(ProductionBodyCandidatePart::Table {
                    cursor: table_cursor,
                    capacity,
                });
                let mut complete = None;
                loop {
                    let context = self.tables.as_mut().unwrap();
                    let selected = context.evaluate(
                        &table_cursor,
                        capacity,
                        &mut self.content.charge,
                        &mut self.content.steps,
                    )?;
                    let Some(selected) = selected else {
                        break;
                    };
                    let smaller = self.tables.as_mut().unwrap().earlier_capacity(
                        &table_cursor,
                        &selected,
                        &mut self.content.steps,
                    )?;
                    let terminal = selected.after().is_terminal();
                    if terminal {
                        complete = Some((capacity, selected.used_height()));
                    }
                    let context = self.tables.as_ref().unwrap();
                    let end = context.range(table_index).end;
                    if !terminal
                        || !context.table(table_index).keep_with_next()
                        || end == items.len()
                    {
                        *requests.last_mut().unwrap() = ProductionBodyCandidatePart::Table {
                            cursor: table_cursor,
                            capacity,
                        };
                        let next = if terminal { end } else { index };
                        let continuation = (!terminal).then_some(selected.after());
                        let key = self.mixed_boundary_key(
                            next,
                            continuation,
                            None,
                            add(top, selected.used_height(), root)?,
                        )?;
                        self.queue_mixed(&requests, key, &mut alternatives)?;
                    }
                    let Some(next) = smaller else {
                        break;
                    };
                    if next >= capacity {
                        return Err(error(root, E::ReceiptMismatch));
                    }
                    capacity = next;
                }
                let Some((capacity, height)) = complete else {
                    break;
                };
                *requests.last_mut().unwrap() = ProductionBodyCandidatePart::Table {
                    cursor: table_cursor,
                    capacity,
                };
                used = add(top, height, root)?;
                let context = self.tables.as_ref().unwrap();
                after = context.table(table_index).space_after();
                index = context.range(table_index).end;
                ordinary = false;
                continue;
            }
            let Some(item) = items.get(index).filter(|i| i.source.is_some()) else {
                break;
            };
            let gap = if requests.is_empty() {
                Length::ZERO
            } else {
                add(after, item.before, item.owner)?
            };
            let next = add(add(used, gap, item.owner)?, item.consumed()?, item.owner)?;
            if next > maximum {
                break;
            }
            if !ordinary {
                ordinary_start = index;
                self.content.charge.take(1, item.owner)?;
                requests
                    .try_reserve(1)
                    .map_err(|_| error(item.owner, E::AllocationFailure))?;
                requests.push(ProductionBodyCandidatePart::Items { end: index + 1 });
                ordinary = true;
            } else {
                *requests.last_mut().unwrap() =
                    ProductionBodyCandidatePart::Items { end: index + 1 };
            }
            used = next;
            after = item.after;
            index += 1;
            if !item.keep || index == items.len() {
                let key =
                    self.mixed_boundary_key(index, None, Some(ordinary_start..index), used)?;
                self.queue_mixed(&requests, key, &mut alternatives)?;
            }
        }
        let mut best = None;
        let mut attempts = 0u32;
        let candidate_count = alternatives.candidates.len() as u32;
        for boundary in alternatives.candidates {
            attempts = attempts
                .checked_add(1)
                .ok_or_else(|| error(root, E::FootnoteSearchLimit))?;
            if attempts > u32::from(self.maximum_reflows) {
                return Err(error(root, E::FootnoteSearchLimit));
            }
            self.step(root)?;
            if let Some(candidate) =
                self.evaluate_mixed_candidate(&state.demand, state.item, &boundary.requests)?
            {
                let key = self.mixed_boundary_key(
                    candidate.next_item(),
                    candidate.table_continuation(),
                    candidate.parts().last().and_then(|p| p.items()),
                    candidate.used_height(),
                )?;
                if key != boundary.key {
                    return Err(error(root, E::ReceiptMismatch));
                }
                best = Some(candidate);
                break;
            }
        }
        if best.is_none()
            && (!state.demand.pending.is_empty()
                || items.get(state.item).is_none_or(|i| i.source.is_none()))
        {
            let observed = candidate_count + 1;
            if observed > u32::from(self.content.maximum_candidates) {
                return Err(error(
                    root,
                    E::PageBreakLookbackLimit {
                        limit: self.content.maximum_candidates,
                        observed,
                    },
                ));
            }
            requests.clear();
            if let Some(cursor) = state.table {
                self.content.charge.take(1, root)?;
                requests
                    .try_reserve(1)
                    .map_err(|_| error(root, E::AllocationFailure))?;
                requests.push(ProductionBodyCandidatePart::Table {
                    cursor,
                    capacity: Length::ZERO,
                });
            }
            attempts = attempts
                .checked_add(1)
                .ok_or_else(|| error(root, E::FootnoteSearchLimit))?;
            if attempts > u32::from(self.maximum_reflows) {
                return Err(error(root, E::FootnoteSearchLimit));
            }
            best = self.evaluate_mixed_candidate(&state.demand, state.item, &requests)?;
        }
        let Some(selected) = best else {
            return Ok(None);
        };
        let end = selected.next_item();
        let table = selected.table_continuation();
        let forced = if table.is_none() {
            items
                .get(end)
                .filter(|i| i.source.is_none())
                .map(|i| i.owner)
        } else {
            None
        };
        let footnote_forced = selected
            .footnotes()
            .is_some_and(|f| f.forced_break_owner().is_some());
        self.content.charge.take(1, root)?;
        let next = ProductionBodyMixedPageState {
            demand: self.fork(selected.next_state(), 0)?,
            item: end + usize::from(forced.is_some()),
            table,
            page: state
                .page
                .checked_add(1)
                .ok_or_else(|| error(root, E::PageLimit))?,
            empty_page: forced.is_some() || footnote_forced,
        };
        Ok(Some(ProductionBodyMixedPageSelection {
            page: state.page,
            selected,
            forced,
            next,
            attempts,
        }))
    }
    pub fn select_mixed_pages(
        &mut self,
    ) -> Result<ProductionBodyMixedPageSequence<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let root = NodeId::new(0);
        self.content.charge.take(1, root)?;
        let initial = self.begin_mixed_pages()?;
        let mut pages: Vec<ProductionBodyMixedPageSelection<'b, 'f, 's, 'p, 'a>> = Vec::new();
        loop {
            let state = pages.last().map_or(&initial, |p| p.next_state());
            if state.is_complete() {
                break;
            }
            let selected = self
                .select_mixed_page(state)?
                .ok_or_else(|| error(root, E::JointPageNoFit))?;
            let next = selected.next_state();
            let table_progress = match (state.table, next.table) {
                (None, Some(c)) => !c.is_initial(),
                (Some(a), Some(b)) => b.offset() > a.offset() || b.next_row() > a.next_row(),
                (Some(_), None) => true,
                (None, None) => false,
            };
            let mut notes = false;
            if let Some(region) = selected.candidate().footnotes() {
                for fragment in region.fragments() {
                    self.step(root)?;
                    notes |= !fragment.fragment().consumed_range().is_empty();
                }
            }
            if next.item < state.item
                || (next.item == state.item
                    && !table_progress
                    && !notes
                    && !(state.empty_page && next.is_complete()))
            {
                return Err(error(root, E::ReceiptMismatch));
            }
            pages
                .try_reserve(1)
                .map_err(|_| error(root, E::AllocationFailure))?;
            pages.push(selected);
        }
        if pages.is_empty() {
            return Err(error(root, E::ReceiptMismatch));
        }
        Ok(ProductionBodyMixedPageSequence {
            owner_id: self.owner_id,
            measurements_fingerprint: self.tables.as_ref().unwrap().measurements_fingerprint(),
            pages,
        })
    }
    pub fn verify_mixed_sequence(
        &self,
        sequence: &ProductionBodyMixedPageSequence<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if sequence.owner_id != self.owner_id
            || self.tables.as_ref().is_none_or(|tables| {
                sequence.measurements_fingerprint != tables.measurements_fingerprint()
            })
            || sequence
                .pages
                .last()
                .is_none_or(|p| !p.next_state().is_complete())
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}
