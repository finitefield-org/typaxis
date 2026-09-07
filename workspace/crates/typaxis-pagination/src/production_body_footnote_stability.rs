//! Exact repeated-page comparison for one immutable measured flow.
use super::*;

/// Issued only after two complete selections and physical placements agree.
/// Does not certify dynamic line/reference convergence or public PDF closure.
pub struct ProductionBodyFootnoteStablePages<'b, 'f, 's, 'p, 'a> {
    sequence: ProductionBodyFootnotePageSequence<'b, 'f, 's, 'p, 'a>,
    passes: u16,
    record_charge: u64,
    work_steps: u64,
}
impl<'b, 'f, 's, 'p, 'a> ProductionBodyFootnoteStablePages<'b, 'f, 's, 'p, 'a> {
    pub fn sequence(&self) -> &ProductionBodyFootnotePageSequence<'b, 'f, 's, 'p, 'a> {
        &self.sequence
    }
    pub fn passes(&self) -> u16 {
        self.passes
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn work_steps(&self) -> u64 {
        self.work_steps
    }
}

impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn select_stable_pages(
        &mut self,
    ) -> Result<ProductionBodyFootnoteStablePages<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let root = NodeId::new(0);
        if self.maximum_passes < 2 {
            return Err(error(root, E::PagePassLimit));
        }
        self.content.charge.take(1, root)?;
        let mut previous = self.select_pages()?;
        for pass in 2..=self.maximum_passes {
            self.content.charge.take(1, root)?;
            let current = self.select_pages()?;
            let before = self.place_pages_content(&previous)?;
            let after = self.place_pages_content(&current)?;
            if self.same_placed_sequence(&before, &after)? {
                return Ok(ProductionBodyFootnoteStablePages {
                    sequence: current,
                    passes: pass,
                    record_charge: self.record_charge(),
                    work_steps: self.work_steps(),
                });
            }
            previous = current;
        }
        Err(error(root, E::PagePassLimit))
    }

    fn same_placed_sequence(
        &mut self,
        left: &ProductionBodyFootnotePlacedSequence<'_, 'b, 'f, 's, 'p, 'a>,
        right: &ProductionBodyFootnotePlacedSequence<'_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.verify_sequence(left.sequence())?;
        self.verify_sequence(right.sequence())?;
        if left.pages().len() != right.pages().len() {
            return Ok(false);
        }
        let root = NodeId::new(0);
        for (left, right) in left.pages().iter().zip(right.pages()) {
            self.step(root)?;
            let l = left.selection();
            let r = right.selection();
            let lc = l.candidate();
            let rc = r.candidate();
            if left.separator_ink() != right.separator_ink()
                || l.page_index() != r.page_index()
                || l.forced_break() != r.forced_break()
                || lc.body_range() != rc.body_range()
                || lc.body_height() != rc.body_height()
                || lc.footnote_bounds() != rc.footnote_bounds()
                || l.next_state().body_start() != r.next_state().body_start()
                || l.next_state().is_complete() != r.next_state().is_complete()
            {
                return Ok(false);
            }
            if !self.same_demand(lc.next_state(), rc.next_state())? {
                return Ok(false);
            }
            match (lc.footnotes(), rc.footnotes()) {
                (None, None) => (),
                (Some(l), Some(r)) => {
                    if l.used_height() != r.used_height()
                        || l.available_height() != r.available_height()
                        || l.forced_break_owner() != r.forced_break_owner()
                        || l.fragments().len() != r.fragments().len()
                    {
                        return Ok(false);
                    }
                    for (l, r) in l.fragments().iter().zip(r.fragments()) {
                        self.step(root)?;
                        if l.offset() != r.offset() {
                            return Ok(false);
                        }
                        let l = l.fragment();
                        let r = r.fragment();
                        if l.definition_index() != r.definition_index()
                            || l.consumed_range() != r.consumed_range()
                            || l.items().len() != r.items().len()
                            || l.used_height() != r.used_height()
                            || l.available_height() != r.available_height()
                            || l.reason() != r.reason()
                            || l.forced_break_owner() != r.forced_break_owner()
                            || l.selected_candidate_index() != r.selected_candidate_index()
                            || !self.same_records(l.candidates(), r.candidates())?
                        {
                            return Ok(false);
                        }
                    }
                }
                _ => return Ok(false),
            }
            if !self.same_records(left.fragments(), right.fragments())?
                || !self.same_records(left.list_markers(), right.list_markers())?
                || !self.same_records(left.footnote_markers(), right.footnote_markers())?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn same_records<T: PartialEq>(
        &mut self,
        left: &[T],
        right: &[T],
    ) -> Result<bool, ProductionBodyPaginationError> {
        if left.len() != right.len() {
            return Ok(false);
        }
        for (l, r) in left.iter().zip(right) {
            self.step(NodeId::new(0))?;
            if l != r {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn same_demand(
        &mut self,
        left: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        right: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.verify_state(left)?;
        self.verify_state(right)?;
        if !self.same_records(&left.pending, &right.pending)?
            || left.definitions.len() != right.definitions.len()
        {
            return Ok(false);
        }
        for (l, r) in left.definitions.iter().zip(&right.definitions) {
            self.step(NodeId::new(0))?;
            let equal = match (l, r) {
                (Demand::Unreferenced, Demand::Unreferenced) => true,
                (
                    Demand::Complete { first_reference: l },
                    Demand::Complete { first_reference: r },
                ) => l == r,
                (
                    Demand::Pending {
                        first_reference: l,
                        cursor: lc,
                    },
                    Demand::Pending {
                        first_reference: r,
                        cursor: rc,
                    },
                ) => {
                    l == r
                        && lc.definition_index == rc.definition_index
                        && lc.next_item == rc.next_item
                }
                _ => false,
            };
            if !equal {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
