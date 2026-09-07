//! Owned source-contiguous page search; does not authorize final placement/paint.
use super::*;

pub struct ProductionBodyFootnotePageState<'b, 'f, 's, 'p, 'a> {
    demand: ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    body_start: usize,
    page_index: u32,
    empty_page: bool,
}
impl ProductionBodyFootnotePageState<'_, '_, '_, '_, '_> {
    pub fn body_start(&self) -> usize {
        self.body_start
    }
    pub fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn is_complete(&self) -> bool {
        self.body_start == self.demand.flow.body_items().len()
            && self.demand.pending.is_empty()
            && !self.empty_page
    }
}

pub struct ProductionBodyFootnotePageSelection<'b, 'f, 's, 'p, 'a> {
    page_index: u32,
    selected: ProductionBodyFootnoteCandidate<'b, 'f, 's, 'p, 'a>,
    forced_break: Option<NodeId>,
    next: ProductionBodyFootnotePageState<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionBodyFootnotePageSelection<'b, 'f, 's, 'p, 'a> {
    pub fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn candidate(&self) -> &ProductionBodyFootnoteCandidate<'b, 'f, 's, 'p, 'a> {
        &self.selected
    }
    pub fn forced_break(&self) -> Option<NodeId> {
        self.forced_break
    }
    pub fn next_state(&self) -> &ProductionBodyFootnotePageState<'b, 'f, 's, 'p, 'a> {
        &self.next
    }
}

impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn begin_pages(
        &mut self,
    ) -> Result<ProductionBodyFootnotePageState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        for pair in self.content.flow.body_items().windows(2) {
            self.step(pair[0].owner)?;
            if pair[0].keep && pair[1].source.is_none() {
                return Err(error(pair[0].owner, E::KeepAcrossForcedBreak));
            }
        }
        self.content.charge.take(1, NodeId::new(0))?;
        Ok(ProductionBodyFootnotePageState {
            demand: self.begin()?,
            body_start: 0,
            page_index: 0,
            empty_page: true,
        })
    }

    /// All legal body alternatives are evaluated before selecting the lowest
    /// common boundary cost (then source offset). Exhaustion is an error, never
    /// permission to accept a prefix of the alternatives. None means complete
    /// or no simultaneous fit; distinguish using state.is_complete().
    pub fn select_page(
        &mut self,
        state: &ProductionBodyFootnotePageState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<
        Option<ProductionBodyFootnotePageSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.verify_state(&state.demand)?;
        if state.is_complete() {
            return Ok(None);
        }
        let root = NodeId::new(0);
        if state.page_index >= self.maximum_pages {
            return Err(error(root, E::PageLimit));
        }
        self.content.charge.take(1, root)?;
        let items = self.content.flow.body_items();
        let start = state.body_start;
        let mut best = None;
        let mut best_key = None;
        let mut attempts = 0u32;
        if items.get(start).is_some_and(|i| i.source.is_some()) {
            let content = &mut self.content;
            let boundaries = page_breaks::all_boundaries(
                items,
                start,
                content.flow.blocks.page_geometry().body().height().get(),
                &content.paragraph_lengths,
                &content.headings,
                content.maximum_candidates,
                &mut content.charge,
                |owner| visit(&mut content.steps, content.maximum_steps, owner),
            )?;
            for boundary in boundaries.candidates {
                attempts += 1;
                if attempts > u32::from(self.maximum_reflows) {
                    return Err(error(boundary.owner(), E::FootnoteSearchLimit));
                }
                self.step(boundary.owner())?;
                if let Some(candidate) = self
                    .evaluate_body_candidate(&state.demand, start..boundary.end_item() as usize)?
                {
                    let key = (boundary.costs().total(), boundary.end_item());
                    if best_key.is_none_or(|old| key < old) {
                        best_key = Some(key);
                        best = Some(candidate);
                    }
                }
            }
        }
        // Incoming continuations may occupy a whole page. With no body at this
        // cursor an empty range also preserves authored blank-page semantics.
        if best.is_none()
            && (!state.demand.pending.is_empty()
                || items.get(start).is_none_or(|i| i.source.is_none()))
        {
            attempts += 1;
            if attempts > u32::from(self.maximum_reflows) {
                return Err(error(root, E::FootnoteSearchLimit));
            }
            best = self.evaluate_body_candidate(&state.demand, start..start)?;
        }
        let Some(selected) = best else {
            return Ok(None);
        };
        let end = selected.body_range().end;
        // Consume a body break only when reached by this chosen candidate.
        let forced_break = items
            .get(end)
            .filter(|i| i.source.is_none())
            .map(|i| i.owner);
        if let Some(owner) = forced_break {
            self.step(owner)?;
        }
        let next_start = end + usize::from(forced_break.is_some());
        let footnote_forced = selected
            .footnotes()
            .is_some_and(|f| f.forced_break_owner().is_some());
        self.content.charge.take(1, root)?;
        let next = ProductionBodyFootnotePageState {
            demand: self.fork(selected.next_state(), 0)?,
            body_start: next_start,
            page_index: state
                .page_index
                .checked_add(1)
                .ok_or_else(|| error(root, E::PageLimit))?,
            empty_page: forced_break.is_some() || footnote_forced,
        };
        Ok(Some(ProductionBodyFootnotePageSelection {
            page_index: state.page_index,
            selected,
            forced_break,
            next,
        }))
    }
}

/// A complete, source-contiguous selection, issued only after all demanded
/// continuations have ended. Unreferenced definitions are deliberately unplaced.
pub struct ProductionBodyFootnotePageSequence<'b, 'f, 's, 'p, 'a> {
    owner_id: u64,
    pages: Vec<ProductionBodyFootnotePageSelection<'b, 'f, 's, 'p, 'a>>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionBodyFootnotePageSequence<'b, 'f, 's, 'p, 'a> {
    pub fn pages(&self) -> &[ProductionBodyFootnotePageSelection<'b, 'f, 's, 'p, 'a>] {
        &self.pages
    }
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn select_pages(
        &mut self,
    ) -> Result<ProductionBodyFootnotePageSequence<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let root = NodeId::new(0);
        self.content.charge.take(1, root)?;
        let initial = self.begin_pages()?;
        let mut pages: Vec<ProductionBodyFootnotePageSelection<'b, 'f, 's, 'p, 'a>> = Vec::new();
        loop {
            let state = pages.last().map_or(&initial, |page| page.next_state());
            if state.is_complete() {
                break;
            }
            self.step(root)?;
            let selected = self
                .select_page(state)?
                .ok_or_else(|| error(root, E::JointPageNoFit))?;
            let next = selected.next_state();
            if next.body_start < state.body_start || next.page_index != state.page_index + 1 {
                return Err(error(root, E::ReceiptMismatch));
            }
            let mut note_progress = false;
            if let Some(region) = selected.candidate().footnotes() {
                for fragment in region.fragments() {
                    self.step(root)?;
                    note_progress |= !fragment.fragment().consumed_range().is_empty();
                }
            }
            if next.body_start == state.body_start
                && !note_progress
                && !(state.empty_page && next.is_complete())
            {
                return Err(error(root, E::ReceiptMismatch));
            }
            // The selection already paid for its record; retain it by move.
            pages
                .try_reserve(1)
                .map_err(|_| error(root, E::AllocationFailure))?;
            pages.push(selected);
        }
        if pages.is_empty() {
            return Err(error(root, E::ReceiptMismatch));
        }
        Ok(ProductionBodyFootnotePageSequence {
            owner_id: self.owner_id,
            pages,
        })
    }

    pub(super) fn verify_sequence(
        &self,
        sequence: &ProductionBodyFootnotePageSequence<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if sequence.owner_id != self.owner_id {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        let last = sequence
            .pages
            .last()
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        self.verify_state(&last.next.demand)?;
        if !last.next.is_complete() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}
