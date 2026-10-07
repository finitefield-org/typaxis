//! Repeated immutable column selections. This proof covers source cuts and
//! candidate geometry, not balancing, physical placement or PDF authorization.
use super::*;
use crate::production_body::body_flow::book_v2::BookV2TableCursor;

/// A repeated source selection cannot authorize single-body placement.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnRepeatedPages, BookV2BodyMixedStablePages};
/// fn single<'b, 'f, 's, 'p, 'a>(v: BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>)
///     -> BookV2BodyMixedStablePages<'b, 'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a> {
    sequence: BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>,
    passes: u16,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a> {
    pub fn sequence(&self) -> &BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a> {
        &self.sequence
    }
    pub fn passes(&self) -> u16 {
        self.passes
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    pub fn select_repeated_pages_counted(
        &mut self,
        remaining: u16,
        begun: &mut u16,
    ) -> Result<BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let result = page_stability_kernel::converge_counted(self, remaining, begun)?;
        Ok(BookV2ColumnRepeatedPages {
            sequence: result.sequence,
            passes: result.passes,
        })
    }
    pub(in crate::production_body::body_flow) fn same_column_selections(
        &mut self,
        left: &BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>,
        right: &BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.verify_sequence(left)?;
        self.verify_sequence(right)?;
        let root = NodeId::new(0);
        self.inner.content.step(root)?;
        if left.pages().len() != right.pages().len() {
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
        for (lp, rp) in left.pages().iter().zip(right.pages()) {
            self.inner.content.step(root)?;
            let lc = lp.candidate();
            let rc = rp.candidate();
            let lnext = lp.next_state().source_state();
            let rnext = rp.next_state().source_state();
            if lp.page_index() != rp.page_index()
                || lp.named_page() != rp.named_page()
                || lp.forced_break() != rp.forced_break()
                || lp.candidate_attempts() != rp.candidate_attempts()
                || lnext.next_item() != rnext.next_item()
                || lnext.next_table_index() != rnext.next_table_index()
                || cursor(lnext.table_continuation()) != cursor(rnext.table_continuation())
                || lp.next_state().is_complete() != rp.next_state().is_complete()
                || lc.footnote_bounds() != rc.footnote_bounds()
                || lc.columns().len() != rc.columns().len()
                || lc.frames().footnote() != rc.frames().footnote()
                || lc.frames().column_count() != rc.frames().column_count()
            {
                return Ok(false);
            }
            for (l, r) in lc.columns().iter().zip(rc.columns()) {
                self.inner.content.step(root)?;
                if l.bounds() != r.bounds()
                    || l.used_height() != r.used_height()
                    || l.parts().len() != r.parts().len()
                {
                    return Ok(false);
                }
                for (a, b) in l.parts().iter().zip(r.parts()) {
                    self.inner.content.step(root)?;
                    if a.top() != b.top()
                        || a.height() != b.height()
                        || a.table().map(|t| t.fingerprint()) != b.table().map(|t| t.fingerprint())
                    {
                        return Ok(false);
                    }
                    if a.items() != b.items() {
                        return Ok(false);
                    }
                }
            }
            if !self
                .inner
                .same_demand(&lc.next.source.demand, &rc.next.source.demand)?
            {
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
                        self.inner.content.step(root)?;
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
                                    self.inner.content.step(root)?;
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
                            || !self.inner.same_records(a.candidates(), b.candidates())?
                        {
                            return Ok(false);
                        }
                    }
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}
impl<'b, 'f, 's, 'p, 'a> page_stability_kernel::StableSearch
    for BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>
{
    type Sequence = BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>;
    fn maximum_passes(&self) -> u16 {
        self.inner.maximum_passes
    }
    fn charge_pass(&mut self) -> Result<(), ProductionBodyPaginationError> {
        self.inner.content.charge.take(1, NodeId::new(0))
    }
    fn select(&mut self) -> Result<Self::Sequence, ProductionBodyPaginationError> {
        self.select_pages()
    }
    fn same(
        &mut self,
        left: &Self::Sequence,
        right: &Self::Sequence,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.same_column_selections(left, right)
    }
    fn records(&self) -> u64 {
        self.record_charge()
    }
    fn work(&self) -> u64 {
        self.work_steps()
    }
}
