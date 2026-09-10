//! Source-contiguous successor body/table candidates over the common kernel.
use super::*;
use crate::production_body::body_flow::book_v2::{BookV2TableCursor, BookV2TableFragmentSelection};
#[derive(Clone, Copy)]
pub enum BookV2BodyCandidatePart<'b, 'f, 's, 'p, 'a> {
    /// Consume ordinary body leaves through this exclusive source index.
    Items { end: usize },
    /// Select this table continuation under the requested remaining capacity.
    Table {
        cursor: BookV2TableCursor<'b, 'f, 's, 'p, 'a>,
        capacity: Length,
    },
}
pub struct BookV2BodySelectedPart<'b, 'f, 's, 'p, 'a> {
    items: Option<std::ops::Range<usize>>,
    table: Option<BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>>,
    top: Length,
    height: Length,
}
impl<'b, 'f, 's, 'p, 'a> BookV2BodySelectedPart<'b, 'f, 's, 'p, 'a> {
    pub fn items(&self) -> Option<std::ops::Range<usize>> {
        self.items.clone()
    }
    pub fn table(&self) -> Option<&BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>> {
        self.table.as_ref()
    }
    /// Relative to the common body origin, including the preceding outside gap.
    /// Ordinary ranges still apply their first marker leading within this part.
    pub const fn top(&self) -> Length {
        self.top
    }
    pub const fn height(&self) -> Length {
        self.height
    }
}

/// The table ordinal is independent of the leaf position: empty adjacent tables
/// share a leaf index but remain different source occurrences.
pub struct BookV2BodySourceState<'b, 'f, 's, 'p, 'a> {
    pub(super) demand: BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    pub(super) item: usize,
    pub(super) next_table: usize,
    pub(super) table_end: usize,
    pub(super) continuation: Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2BodySourceState<'b, 'f, 's, 'p, 'a> {
    pub fn next_item(&self) -> usize {
        self.item
    }
    pub fn next_table_index(&self) -> usize {
        self.next_table
    }
    pub fn table_continuation(&self) -> Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>> {
        self.continuation
    }
    pub fn demand(&self) -> &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a> {
        &self.demand
    }
    pub fn is_complete(&self) -> bool {
        self.item == self.demand.flow.body_items().len()
            && self.next_table == self.table_end
            && self.continuation.is_none()
            && self.demand.pending.is_empty()
    }
}
pub struct BookV2BodyMixedCandidate<'b, 'f, 's, 'p, 'a> {
    start: usize,
    parts: Vec<BookV2BodySelectedPart<'b, 'f, 's, 'p, 'a>>,
    height: Length,
    fit: BookV2BodyFootnoteFit<'b, 'f, 's, 'p, 'a>,
    next: BookV2BodySourceState<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2BodyMixedCandidate<'b, 'f, 's, 'p, 'a> {
    pub fn start_item(&self) -> usize {
        self.start
    }
    pub fn parts(&self) -> &[BookV2BodySelectedPart<'b, 'f, 's, 'p, 'a>] {
        &self.parts
    }
    pub fn used_height(&self) -> Length {
        self.height
    }
    pub fn footnotes(&self) -> Option<&BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a>> {
        self.fit.footnotes()
    }
    pub fn footnote_bounds(&self) -> Option<Rect> {
        self.fit.footnote_bounds()
    }
    pub fn next_state(&self) -> &BookV2BodySourceState<'b, 'f, 's, 'p, 'a> {
        &self.next
    }
    pub fn verify(
        &self,
        state: &BookV2BodySourceState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.fit.verify(&state.demand)
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn begin_body_source(
        &mut self,
    ) -> Result<BookV2BodySourceState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        if self.tables.is_none() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        self.content.charge(1, NodeId::new(0))?;
        Ok(BookV2BodySourceState {
            demand: self.begin()?,
            item: 0,
            next_table: 0,
            table_end: self.body_table_count(),
            continuation: None,
        })
    }
    /// Select requested source-contiguous ordinary ranges and table fragments.
    /// This advances a candidate source branch; page ranking/placement is separate.
    pub fn evaluate_mixed_candidate(
        &mut self,
        state: &BookV2BodySourceState<'b, 'f, 's, 'p, 'a>,
        requests: &[BookV2BodyCandidatePart<'b, 'f, 's, 'p, 'a>],
    ) -> Result<Option<BookV2BodyMixedCandidate<'b, 'f, 's, 'p, 'a>>, ProductionBodyPaginationError>
    {
        self.verify_state(&state.demand)?;
        let requested = match requests.first() {
            Some(BookV2BodyCandidatePart::Table { cursor, .. }) => Some(*cursor),
            _ => None,
        };
        if let Some(expected) = state.continuation {
            if !requests.is_empty()
                && requested.is_none_or(|cursor| {
                    cursor.table_index() != expected.table_index()
                        || cursor.offset() != expected.offset()
                        || cursor.next_row() != expected.next_row()
                        || cursor.is_initial() != expected.is_initial()
                        || cursor.has_started_rows() != expected.has_started_rows()
                        || cursor.next_caption_item() != expected.next_caption_item()
                        || cursor.cell_progress_fingerprint()
                            != expected.cell_progress_fingerprint()
                })
            {
                return Err(error(NodeId::new(0), E::ReceiptMismatch));
            }
        } else if requested.is_some_and(|cursor| !cursor.is_initial()) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        let Some(projection) = mixed_kernel::evaluate(
            self,
            &state.demand,
            state.item,
            Some(state.next_table),
            state.continuation,
            requests,
        )?
        else {
            return Ok(None);
        };
        let fit = BookV2BodyFootnoteFit::from_projection(&state.demand, projection.fit);
        self.content.charge(1, NodeId::new(0))?;
        let next = BookV2BodySourceState {
            demand: self.fork(fit.next_state(), 0)?,
            item: projection.end,
            next_table: projection.next_table.expect("successor ordinal"),
            table_end: state.table_end,
            continuation: projection.continuation,
        };
        Ok(Some(BookV2BodyMixedCandidate {
            start: state.item,
            parts: projection.parts,
            height: projection.height,
            fit,
            next,
        }))
    }
}

impl<'b, 'f, 's, 'p, 'a> mixed_kernel::MixedSearch<'b>
    for BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>
{
    type Cursor = BookV2TableCursor<'b, 'f, 's, 'p, 'a>;
    type Request = BookV2BodyCandidatePart<'b, 'f, 's, 'p, 'a>;
    type Part = BookV2BodySelectedPart<'b, 'f, 's, 'p, 'a>;
    fn request(request: &Self::Request) -> mixed_kernel::Request<Self::Cursor> {
        match *request {
            BookV2BodyCandidatePart::Items { end } => mixed_kernel::Request::Items { end },
            BookV2BodyCandidatePart::Table { cursor, capacity } => {
                mixed_kernel::Request::Table { cursor, capacity }
            }
        }
    }
    fn has_tables(&self) -> bool {
        self.tables.is_some()
    }
    fn source_keep_before(
        &mut self,
        item: usize,
        next: Option<usize>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.content.step(NodeId::new(0))?;
        let context = self.tables.as_ref().unwrap();
        if let Some(previous) = context.previous_root(next.expect("successor ordinal")) {
            if context
                .range(previous)
                .is_some_and(|range| range.end == item)
            {
                return Ok(context.table(previous).keep_with_next());
            }
        }
        Ok(item > 0 && self.content.flow.body_items()[item - 1].keep)
    }
    fn table_query_work(&mut self) -> Result<(), ProductionBodyPaginationError> {
        self.content.step(NodeId::new(0))
    }
    fn ordinary_range(&self, range: std::ops::Range<usize>, next: Option<usize>) -> bool {
        self.tables
            .as_ref()
            .unwrap()
            .ordinary_range(range, next.expect("successor ordinal"))
    }
    fn table_at(&self, item: usize, next: Option<usize>) -> Option<usize> {
        self.tables
            .as_ref()
            .unwrap()
            .at(item, next.expect("successor ordinal"))
    }
    fn table_range(&self, index: usize) -> std::ops::Range<usize> {
        self.tables
            .as_ref()
            .unwrap()
            .range(index)
            .expect("bound table")
    }
    fn table_info(&self, index: usize) -> (NodeId, Length, Length, bool) {
        let table = self.tables.as_ref().unwrap().table(index);
        (
            table.owner(),
            table.space_before(),
            table.space_after(),
            table.keep_with_next(),
        )
    }
    fn next_table(&self, index: usize) -> Result<usize, ProductionBodyPaginationError> {
        Ok(self.tables.as_ref().unwrap().successor(index))
    }
    fn remaining_source(&self, item: usize, next: Option<usize>) -> bool {
        item < self.content.flow.body_items().len()
            || next.expect("successor ordinal") < self.tables.as_ref().unwrap().len()
    }
    fn initial(cursor: Self::Cursor) -> bool {
        cursor.is_initial()
    }
    fn table_index(cursor: Self::Cursor) -> usize {
        cursor.table_index()
    }
    fn terminal(cursor: Self::Cursor) -> bool {
        cursor.is_terminal()
    }
    fn after(fragment: &Self::TableFragment) -> Self::Cursor {
        fragment.after()
    }
    fn evaluate_table(
        &mut self,
        cursor: &Self::Cursor,
        capacity: Length,
    ) -> Result<Option<Self::TableFragment>, ProductionBodyPaginationError> {
        self.tables.as_mut().unwrap().evaluate(
            cursor,
            capacity,
            self.headers,
            self.active_page_frames.map(|f| f.body()).map(|r| r.width()),
            &mut self.content.charge,
            &mut self.content.steps,
        )
    }
    fn part(
        items: Option<std::ops::Range<usize>>,
        table: Option<Self::TableFragment>,
        top: Length,
        height: Length,
    ) -> Self::Part {
        BookV2BodySelectedPart {
            items,
            table,
            top,
            height,
        }
    }
}
