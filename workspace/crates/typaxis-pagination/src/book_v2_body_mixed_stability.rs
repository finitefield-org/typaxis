//! Repeat complete mixed selection and placement without resetting their budgets.
use super::*;
use crate::production_body::body_flow::book_v2::BookV2TableCursor;
pub struct BookV2BodyMixedStablePages<'b, 'f, 's, 'p, 'a> {
    sequence: BookV2BodyMixedPageSequence<'b, 'f, 's, 'p, 'a>,
    passes: u16,
    records: u64,
    work: u64,
}
impl<'b, 'f, 's, 'p, 'a> BookV2BodyMixedStablePages<'b, 'f, 's, 'p, 'a> {
    pub fn sequence(&self) -> &BookV2BodyMixedPageSequence<'b, 'f, 's, 'p, 'a> {
        &self.sequence
    }
    pub const fn passes(&self) -> u16 {
        self.passes
    }
    pub const fn record_charge(&self) -> u64 {
        self.records
    }
    pub const fn work_steps(&self) -> u64 {
        self.work
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Stable physical pages over this immutable measured flow. Dynamic source
    /// label/line convergence and final PDF closure are still separate owners.
    pub fn select_stable_mixed_pages(
        &mut self,
        remaining_passes: u16,
    ) -> Result<BookV2BodyMixedStablePages<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let projection = page_stability_kernel::converge(self, remaining_passes)?;
        Ok(BookV2BodyMixedStablePages {
            sequence: projection.sequence,
            passes: projection.passes,
            records: projection.records,
            work: projection.work,
        })
    }
    fn same_mixed_geometry(
        &mut self,
        left: &BookV2BodyMixedPlacedSequence<'_, 'b, 'f, 's, 'p, 'a>,
        right: &BookV2BodyMixedPlacedSequence<'_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.verify_mixed_sequence(left.sequence())?;
        self.verify_mixed_sequence(right.sequence())?;
        if left.pages().len() != right.pages().len() {
            return Ok(false);
        }
        let root = NodeId::new(0);
        for (l, r) in left.pages().iter().zip(right.pages()) {
            self.content.step(root)?;
            let lp = l.selection();
            let rp = r.selection();
            let lc = lp.candidate();
            let rc = rp.candidate();
            if lp.page_index() != rp.page_index()
                || lp.named_page() != rp.named_page()
                || lp.body_bounds() != rp.body_bounds()
                || lp.declared_footnote_region() != rp.declared_footnote_region()
                || lp.forced_break() != rp.forced_break()
                || lp.candidate_attempts() != rp.candidate_attempts()
                || lp.next_state().source_state().next_item()
                    != rp.next_state().source_state().next_item()
                || lp.next_state().source_state().next_table_index()
                    != rp.next_state().source_state().next_table_index()
                || lc.next_state().next_table_index() != rc.next_state().next_table_index()
                || lp.next_state().is_complete() != rp.next_state().is_complete()
                || lc.start_item() != rc.start_item()
                || lc.next_state().next_item() != rc.next_state().next_item()
                || lc.used_height() != rc.used_height()
                || lc.footnote_bounds() != rc.footnote_bounds()
                || lc.parts().len() != rc.parts().len()
                || l.separator_ink() != r.separator_ink()
            {
                return Ok(false);
            }
            let cursor = |c: Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>>| {
                c.map(|c| {
                    (
                        c.table_index(),
                        c.offset(),
                        c.next_row(),
                        c.is_initial(),
                        c.has_started_rows(),
                        c.next_caption_item(),
                        c.cell_progress_fingerprint(),
                    )
                })
            };
            if cursor(lp.next_state().source_state().table_continuation())
                != cursor(rp.next_state().source_state().table_continuation())
            {
                return Ok(false);
            }
            for (a, b) in lc.parts().iter().zip(rc.parts()) {
                self.content.step(root)?;
                if a.items() != b.items()
                    || a.top() != b.top()
                    || a.height() != b.height()
                    || a.table().map(|t| t.fingerprint()) != b.table().map(|t| t.fingerprint())
                {
                    return Ok(false);
                }
            }
            if !self.same_demand(lc.next_state().demand(), rc.next_state().demand())? {
                return Ok(false);
            }
            match (lc.footnotes(), rc.footnotes()) {
                (None, None) => (),
                (Some(a), Some(b)) => {
                    if a.used_height() != b.used_height()
                        || a.available_height() != b.available_height()
                        || a.forced_break_owner() != b.forced_break_owner()
                        || a.fragments().len() != b.fragments().len()
                    {
                        return Ok(false);
                    }
                    for (a, b) in a.fragments().iter().zip(b.fragments()) {
                        self.content.step(root)?;
                        if a.offset() != b.offset() {
                            return Ok(false);
                        }
                        let a = a.fragment();
                        let b = b.fragment();
                        match (a.mixed(), b.mixed()) {
                            (None, None) => {
                                if a.consumed_range()? != b.consumed_range()?
                                    || a.items()?.len() != b.items()?.len()
                                {
                                    return Ok(false);
                                }
                            }
                            (Some(a), Some(b)) => {
                                if a.parts().len() != b.parts().len()
                                    || a.next_state().next_item() != b.next_state().next_item()
                                    || a.next_state().next_table_index()
                                        != b.next_state().next_table_index()
                                    || cursor(a.next_state().table_continuation())
                                        != cursor(b.next_state().table_continuation())
                                {
                                    return Ok(false);
                                }
                                for (a, b) in a.parts().iter().zip(b.parts()) {
                                    self.content.step(root)?;
                                    if a.items() != b.items()
                                        || a.top() != b.top()
                                        || a.height() != b.height()
                                        || a.table().map(|table| table.fingerprint())
                                            != b.table().map(|table| table.fingerprint())
                                    {
                                        return Ok(false);
                                    }
                                }
                            }
                            _ => return Ok(false),
                        }
                        if a.definition_index() != b.definition_index()
                            || a.used_height() != b.used_height()
                            || a.available_height() != b.available_height()
                            || a.reason() != b.reason()
                            || a.forced_break_owner() != b.forced_break_owner()
                            || a.selected_candidate_index() != b.selected_candidate_index()
                            || !self.same_records(a.candidates(), b.candidates())?
                        {
                            return Ok(false);
                        }
                    }
                }
                _ => return Ok(false),
            }
            if !self.same_records(l.header_variants(), r.header_variants())?
                || !self.same_records(l.equation_numbers(), r.equation_numbers())?
                || !self.same_records(l.fragments(), r.fragments())?
                || !self.same_records(l.cell_roles(), r.cell_roles())?
                || !self.same_records(
                    l.repeated_caption_positions(),
                    r.repeated_caption_positions(),
                )?
                || !self.same_records(l.list_markers(), r.list_markers())?
                || !self.same_records(l.footnote_markers(), r.footnote_markers())?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl<'b, 'f, 's, 'p, 'a> page_stability_kernel::StableSearch
    for BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>
{
    type Sequence = BookV2BodyMixedPageSequence<'b, 'f, 's, 'p, 'a>;
    fn maximum_passes(&self) -> u16 {
        self.maximum_passes
    }
    fn charge_pass(&mut self) -> Result<(), ProductionBodyPaginationError> {
        self.content.charge.take(1, NodeId::new(0))
    }
    fn select(&mut self) -> Result<Self::Sequence, ProductionBodyPaginationError> {
        self.select_mixed_pages()
    }
    fn same(
        &mut self,
        left: &Self::Sequence,
        right: &Self::Sequence,
    ) -> Result<bool, ProductionBodyPaginationError> {
        let left = self.place_mixed_pages(left)?;
        let right = self.place_mixed_pages(right)?;
        self.same_mixed_geometry(&left, &right)
    }
    fn records(&self) -> u64 {
        self.record_charge()
    }
    fn work(&self) -> u64 {
        self.work_steps()
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    fn same_records<T: PartialEq>(
        &mut self,
        left: &[T],
        right: &[T],
    ) -> Result<bool, ProductionBodyPaginationError> {
        kernel::same_records(&mut self.content, left, right)
    }
    fn same_demand(
        &mut self,
        left: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        right: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.verify_state(left)?;
        self.verify_state(right)?;
        kernel::same_demand(
            &mut self.content,
            &left.pending,
            &right.pending,
            &left.definitions,
            &right.definitions,
            |cursor| {
                (
                    cursor.definition_index(),
                    cursor.next_item(),
                    cursor.next_table_index(),
                    cursor.definition_started(),
                    cursor.table_continuation().map(|table| {
                        (
                            table.table_index(),
                            table.offset(),
                            table.next_row(),
                            table.next_caption_item(),
                            table.is_initial(),
                            table.has_started_rows(),
                            table.cell_progress_fingerprint(),
                        )
                    }),
                )
            },
        )
    }
}
