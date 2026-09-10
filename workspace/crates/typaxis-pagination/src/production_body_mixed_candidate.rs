//! Real ordinary ranges and parallel table fragments in one body candidate.
use super::*;

#[derive(Clone, Copy)]
pub enum ProductionBodyCandidatePart<'b, 'f, 's, 'p, 'a> {
    /// Consume ordinary body leaves through this exclusive source index.
    Items { end: usize },
    /// Select this table continuation under the requested remaining capacity.
    Table {
        cursor: ProductionTableCursor<'b, 'f, 's, 'p, 'a>,
        capacity: Length,
    },
}
pub struct ProductionBodySelectedPart<'b, 'f, 's, 'p, 'a> {
    items: Option<std::ops::Range<usize>>,
    table: Option<ProductionTableFragmentSelection<'b, 'f, 's, 'p, 'a>>,
    top: Length,
    height: Length,
}
impl<'b, 'f, 's, 'p, 'a> ProductionBodySelectedPart<'b, 'f, 's, 'p, 'a> {
    pub fn items(&self) -> Option<std::ops::Range<usize>> {
        self.items.clone()
    }
    pub fn table(&self) -> Option<&ProductionTableFragmentSelection<'b, 'f, 's, 'p, 'a>> {
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
pub struct ProductionBodyMixedCandidate<'b, 'f, 's, 'p, 'a> {
    start: usize,
    end: usize,
    continuation: Option<ProductionTableCursor<'b, 'f, 's, 'p, 'a>>,
    parts: Vec<ProductionBodySelectedPart<'b, 'f, 's, 'p, 'a>>,
    height: Length,
    fit: ProductionBodyFootnoteFit<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionBodyMixedCandidate<'b, 'f, 's, 'p, 'a> {
    pub const fn start_item(&self) -> usize {
        self.start
    }
    /// Next fully unconsumed source item. A partial table additionally retains
    /// its own offset; this is not an enclosing range of painted cell leaves.
    pub const fn next_item(&self) -> usize {
        self.end
    }
    pub const fn table_continuation(&self) -> Option<ProductionTableCursor<'b, 'f, 's, 'p, 'a>> {
        self.continuation
    }
    pub fn parts(&self) -> &[ProductionBodySelectedPart<'b, 'f, 's, 'p, 'a>] {
        &self.parts
    }
    pub const fn used_height(&self) -> Length {
        self.height
    }
    pub fn footnotes(&self) -> Option<&ProductionFootnoteRegionSelection<'b, 'f, 's, 'p, 'a>> {
        self.fit.footnotes()
    }
    pub fn footnote_bounds(&self) -> Option<Rect> {
        self.fit.footnote_bounds()
    }
    pub fn next_state(&self) -> &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a> {
        self.fit.next_state()
    }
    pub fn verify(
        &self,
        state: &ProductionFootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.fit.verify(state)
    }
}

pub fn prepare_production_table_body_search<'b, 'f, 's, 'p, 'a>(
    measurements: &'b ProductionTableMeasurements<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
) -> Result<ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let (tables, charge, steps) =
        super::super::super::table_measurements::ProductionTableBodyContext::prepare(
            measurements,
            limits,
            maximum_work,
        )?;
    let mut result = prepare_table_demand_search(
        measurements.shared_flow(),
        limits,
        maximum_work,
        charge,
        steps,
    )?;
    if !tables.is_empty() {
        result.tables = Some(tables);
    }
    Ok(result)
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn begin_table(
        &mut self,
        index: usize,
    ) -> Result<ProductionTableCursor<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.step(NodeId::new(0))?;
        let tables = self
            .tables
            .as_mut()
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        tables.begin(index, &mut self.content.charge, &mut self.content.steps)
    }
    /// Evaluate a locally source-contiguous mixed body candidate and its actual
    /// demanded footnotes. The page owner still ranks alternatives and certifies
    /// continuity with preceding/following pages before any placement is issued.
    pub fn evaluate_mixed_candidate(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        start: usize,
        requests: &[ProductionBodyCandidatePart<'b, 'f, 's, 'p, 'a>],
    ) -> Result<
        Option<ProductionBodyMixedCandidate<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        Ok(
            mixed_kernel::evaluate(self, state, start, None, None, requests)?.map(|projection| {
                debug_assert!(projection.next_table.is_none());
                ProductionBodyMixedCandidate {
                    start,
                    end: projection.end,
                    continuation: projection.continuation,
                    parts: projection.parts,
                    height: projection.height,
                    fit: body_candidate::ProductionBodyFootnoteFit::from_projection(
                        state,
                        projection.fit,
                    ),
                }
            }),
        )
    }
}

impl<'b, 'f, 's, 'p, 'a> mixed_kernel::MixedSearch<'b>
    for ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a>
{
    type Cursor = ProductionTableCursor<'b, 'f, 's, 'p, 'a>;
    type Request = ProductionBodyCandidatePart<'b, 'f, 's, 'p, 'a>;
    type Part = ProductionBodySelectedPart<'b, 'f, 's, 'p, 'a>;
    fn request(request: &Self::Request) -> mixed_kernel::Request<Self::Cursor> {
        match *request {
            ProductionBodyCandidatePart::Items { end } => mixed_kernel::Request::Items { end },
            ProductionBodyCandidatePart::Table { cursor, capacity } => {
                mixed_kernel::Request::Table { cursor, capacity }
            }
        }
    }
    fn has_tables(&self) -> bool {
        self.tables.is_some()
    }
    fn table_query_work(&mut self) -> Result<(), ProductionBodyPaginationError> {
        self.table_query_work()
    }
    fn ordinary_range(&self, range: std::ops::Range<usize>, _: Option<usize>) -> bool {
        !self.tables.as_ref().unwrap().contains(range)
    }
    fn table_at(&self, item: usize, _: Option<usize>) -> Option<usize> {
        self.tables.as_ref().unwrap().at(item)
    }
    fn table_range(&self, index: usize) -> std::ops::Range<usize> {
        self.tables.as_ref().unwrap().range(index)
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
    fn remaining_source(&self, item: usize, _: Option<usize>) -> bool {
        item < self.content.flow.body_items().len()
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
        ProductionBodySelectedPart {
            items,
            table,
            top,
            height,
        }
    }
}
