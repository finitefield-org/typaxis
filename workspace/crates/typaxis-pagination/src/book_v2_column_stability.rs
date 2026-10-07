//! Repeat actual column selection and placement with one search budget.
use super::*;

/// Physical column stability remains separate from source/formula closure.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnStablePages, BookV2BodyMixedStablePages};
/// fn single<'b, 'f, 's, 'p, 'a>(v: BookV2ColumnStablePages<'b, 'f, 's, 'p, 'a>)
///     -> BookV2BodyMixedStablePages<'b, 'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnStablePages<'b, 'f, 's, 'p, 'a> {
    sequence: BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>,
    passes: u16,
    records: u64,
    work: u64,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnStablePages<'b, 'f, 's, 'p, 'a> {
    pub fn sequence(&self) -> &BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a> {
        &self.sequence
    }
    pub fn passes(&self) -> u16 {
        self.passes
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
}
struct PhysicalColumns<'c, 'b, 'f, 's, 'p, 'a>(&'c mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>);
impl<'b, 'f, 's, 'p, 'a> page_stability_kernel::StableSearch
    for PhysicalColumns<'_, 'b, 'f, 's, 'p, 'a>
{
    type Sequence = BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>;
    fn maximum_passes(&self) -> u16 {
        self.0.inner.maximum_passes
    }
    fn charge_pass(&mut self) -> Result<(), ProductionBodyPaginationError> {
        self.0.inner.content.charge.take(1, NodeId::new(0))
    }
    fn select(&mut self) -> Result<Self::Sequence, ProductionBodyPaginationError> {
        let sequence = self.0.select_pages()?;
        for page in sequence.pages() {
            self.0.inner.content.step(NodeId::new(0))?;
            if page.candidate().frames().requires_last_page_balance() {
                return Err(error(NodeId::new(0), E::PendingRegion("column_balance")));
            }
        }
        Ok(sequence)
    }
    fn same(
        &mut self,
        left: &Self::Sequence,
        right: &Self::Sequence,
    ) -> Result<bool, ProductionBodyPaginationError> {
        let left = self.0.place_column_pages(left)?;
        let right = self.0.place_column_pages(right)?;
        self.0.same_column_geometry(&left, &right)
    }
    fn records(&self) -> u64 {
        self.0.record_charge()
    }
    fn work(&self) -> u64 {
        self.0.work_steps()
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    pub fn select_stable_column_pages_counted(
        &mut self,
        remaining: u16,
        begun: &mut u16,
    ) -> Result<BookV2ColumnStablePages<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let result =
            page_stability_kernel::converge_counted(&mut PhysicalColumns(self), remaining, begun)?;
        Ok(BookV2ColumnStablePages {
            sequence: result.sequence,
            passes: result.passes,
            records: result.records,
            work: result.work,
        })
    }
}
