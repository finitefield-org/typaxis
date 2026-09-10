//! Common ordered body/table candidate evaluation with typed source cursors.
use super::*;
use fit_kernel::{BodySearch, TableSearch};

pub(super) enum Request<C> {
    Items {
        end: usize,
    },
    #[cfg(feature = "book-v2-staging")]
    Forced,
    Table {
        cursor: C,
        capacity: Length,
    },
}
pub(super) trait MixedSearch<'b>: BodySearch<'b> + TableSearch {
    type Cursor: Copy;
    type Request;
    type Part;
    fn request(request: &Self::Request) -> Request<Self::Cursor>;
    fn has_tables(&self) -> bool;
    fn source_keep_before(
        &mut self,
        item: usize,
        _next_table: Option<usize>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.keep_before(item)
    }
    fn table_query_work(&mut self) -> Result<(), ProductionBodyPaginationError>;
    fn ordinary_range(&self, range: std::ops::Range<usize>, next_table: Option<usize>) -> bool;
    fn table_at(&self, item: usize, next_table: Option<usize>) -> Option<usize>;
    fn table_range(&self, index: usize) -> std::ops::Range<usize>;
    fn next_table(&self, index: usize) -> Result<usize, ProductionBodyPaginationError> {
        index
            .checked_add(1)
            .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow))
    }
    fn table_info(&self, index: usize) -> (NodeId, Length, Length, bool);
    fn remaining_source(&self, item: usize, next_table: Option<usize>) -> bool;
    fn initial(cursor: Self::Cursor) -> bool;
    fn table_index(cursor: Self::Cursor) -> usize;
    fn terminal(cursor: Self::Cursor) -> bool;
    fn after(fragment: &Self::TableFragment) -> Self::Cursor;
    fn evaluate_table(
        &mut self,
        cursor: &Self::Cursor,
        capacity: Length,
    ) -> Result<Option<Self::TableFragment>, ProductionBodyPaginationError>;
    fn part(
        items: Option<std::ops::Range<usize>>,
        table: Option<Self::TableFragment>,
        top: Length,
        height: Length,
    ) -> Self::Part;
}
pub(super) struct Projection<'b, S: MixedSearch<'b>> {
    pub end: usize,
    pub next_table: Option<usize>,
    pub continuation: Option<S::Cursor>,
    pub parts: Vec<S::Part>,
    pub height: Length,
    pub fit: fit_kernel::Fit<S>,
}
pub(super) fn evaluate<'b, S: MixedSearch<'b>>(
    search: &mut S,
    state: &S::State,
    start: usize,
    next_table: Option<usize>,
    incoming: Option<S::Cursor>,
    requests: &[S::Request],
) -> Result<Option<Projection<'b, S>>, ProductionBodyPaginationError> {
    let Some(source) = source_kernel::evaluate(
        &mut BodySource(search),
        state,
        start,
        next_table,
        incoming,
        requests,
    )?
    else {
        return Ok(None);
    };
    let source_kernel::Projection {
        end,
        next_table,
        continuation,
        parts,
        height,
        demanded,
        forced,
    } = source;
    // The body adapter retains its existing page-break ownership. Only the
    // definition adapter requests explicit breaks through this source kernel.
    if forced.is_some() {
        return Err(error(NodeId::new(0), E::ReceiptMismatch));
    }
    let Some(fit) = fit_kernel::fit(search, height, demanded)? else {
        return Ok(None);
    };
    Ok(Some(Projection {
        end,
        next_table,
        continuation,
        parts,
        height,
        fit,
    }))
}

#[path = "production_mixed_source_kernel.rs"]
pub(super) mod source_kernel;
struct BodySource<'r, S>(&'r mut S);
impl<'b, S: MixedSearch<'b>> source_kernel::SourceSearch<'b> for BodySource<'_, S> {
    type State = S::State;
    type TableFragment = S::TableFragment;
    type Cursor = S::Cursor;
    type Request = S::Request;
    type Part = S::Part;
    fn verify(&self, state: &Self::State) -> Result<(), ProductionBodyPaginationError> {
        self.0.verify(state)
    }
    fn items(&self) -> &'b [ProductionBodyFlowItem] {
        self.0.items()
    }
    fn maximum(&self) -> Length {
        self.0.body().height().get()
    }
    fn charge(&mut self, count: usize, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.0.charge(count, owner)
    }
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.0.step(owner)
    }
    fn fork_table(
        &mut self,
        state: &Self::State,
    ) -> Result<Self::State, ProductionBodyPaginationError> {
        self.0.fork_table(state)
    }
    fn pending(&self, state: &Self::State) -> bool {
        self.0.pending(state)
    }
    fn require_range(
        &mut self,
        state: &mut Self::State,
        range: std::ops::Range<usize>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.0.require_range(state, range)
    }
    fn table_height(fragment: &Self::TableFragment) -> Length {
        S::table_height(fragment)
    }
    fn request(request: &Self::Request) -> Request<Self::Cursor> {
        S::request(request)
    }
    fn has_tables(&self) -> bool {
        self.0.has_tables()
    }
    fn source_keep_before(
        &mut self,
        item: usize,
        next_table: Option<usize>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.0.source_keep_before(item, next_table)
    }
    fn table_query_work(&mut self) -> Result<(), ProductionBodyPaginationError> {
        self.0.table_query_work()
    }
    fn ordinary_range(&self, range: std::ops::Range<usize>, next_table: Option<usize>) -> bool {
        self.0.ordinary_range(range, next_table)
    }
    fn table_at(&self, item: usize, next_table: Option<usize>) -> Option<usize> {
        self.0.table_at(item, next_table)
    }
    fn table_range(&self, index: usize) -> std::ops::Range<usize> {
        self.0.table_range(index)
    }
    fn next_table(&self, index: usize) -> Result<usize, ProductionBodyPaginationError> {
        self.0.next_table(index)
    }
    fn table_info(&self, index: usize) -> (NodeId, Length, Length, bool) {
        self.0.table_info(index)
    }
    fn remaining_source(&self, item: usize, next_table: Option<usize>) -> bool {
        self.0.remaining_source(item, next_table)
    }
    fn initial(cursor: Self::Cursor) -> bool {
        S::initial(cursor)
    }
    fn table_index(cursor: Self::Cursor) -> usize {
        S::table_index(cursor)
    }
    fn terminal(cursor: Self::Cursor) -> bool {
        S::terminal(cursor)
    }
    fn after(fragment: &Self::TableFragment) -> Self::Cursor {
        S::after(fragment)
    }
    fn evaluate_table(
        &mut self,
        cursor: &Self::Cursor,
        capacity: Length,
    ) -> Result<Option<Self::TableFragment>, ProductionBodyPaginationError> {
        self.0.evaluate_table(cursor, capacity)
    }
    fn part(
        items: Option<std::ops::Range<usize>>,
        table: Option<Self::TableFragment>,
        top: Length,
        height: Length,
    ) -> Self::Part {
        S::part(items, table, top, height)
    }
    fn require_table(
        &mut self,
        state: &mut Self::State,
        fragment: &Self::TableFragment,
    ) -> Result<bool, ProductionBodyPaginationError> {
        for range in S::ranges(fragment) {
            self.0
                .step(self.0.table_info(S::table_index(S::after(fragment))).0)?;
            if !self.0.require_range(state, range)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
