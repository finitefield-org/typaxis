//! Simultaneous fit of one actual body boundary and its demanded footnotes.
use super::*;
use crate::production_body::body_flow::book_v2::BookV2TableFragmentSelection;

pub struct BookV2BodyFootnoteCandidate<'b, 'f, 's, 'p, 'a> {
    body: std::ops::Range<usize>,
    body_height: Length,
    fit: BookV2BodyFootnoteFit<'b, 'f, 's, 'p, 'a>,
}
/// Shared measured reservation. Carries no body range or table cursor, so a
/// caller must retain its own real source selection alongside this fit.
pub(in crate::production_body::body_flow) struct BookV2BodyFootnoteFit<'b, 'f, 's, 'p, 'a> {
    owner_id: u64,
    state_id: u64,
    demanded: BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    footnotes: Option<BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a>>,
    footnote_bounds: Option<Rect>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2BodyFootnoteCandidate<'b, 'f, 's, 'p, 'a> {
    pub fn body_range(&self) -> std::ops::Range<usize> {
        self.body.clone()
    }
    pub const fn body_height(&self) -> Length {
        self.body_height
    }
    /// Includes the separator band, bottom-aligned in the declared region.
    pub const fn footnote_bounds(&self) -> Option<Rect> {
        self.fit.footnote_bounds()
    }
    pub fn footnotes(&self) -> Option<&BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a>> {
        self.fit.footnotes()
    }
    pub fn next_state(&self) -> &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a> {
        self.fit.next_state()
    }
    pub fn verify(
        &self,
        state: &BookV2FootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.fit.verify(state)
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2BodyFootnoteFit<'b, 'f, 's, 'p, 'a> {
    pub(super) fn from_projection(
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        projection: fit_kernel::Fit<BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>>,
    ) -> Self {
        Self {
            owner_id: state.owner_id,
            state_id: state.state_id,
            demanded: projection.demanded,
            footnotes: projection.footnotes,
            footnote_bounds: projection.footnote_bounds,
        }
    }
    pub const fn footnote_bounds(&self) -> Option<Rect> {
        self.footnote_bounds
    }
    pub fn footnotes(&self) -> Option<&BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a>> {
        self.footnotes.as_ref()
    }
    pub fn next_state(&self) -> &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a> {
        self.footnotes
            .as_ref()
            .map_or(&self.demanded, |f| f.next_state())
    }
    pub fn verify(
        &self,
        state: &BookV2FootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.owner_id != state.owner_id
            || self.state_id != state.state_id
            || !std::ptr::eq(self.demanded.flow, state.flow)
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Validates a local body cut and simultaneous measured fit. It does not
    /// certify prior/next body cursor continuity, assign a page number, rank
    /// alternative body candidates or issue a final page/paint receipt.
    pub fn evaluate_body_candidate(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        range: std::ops::Range<usize>,
    ) -> Result<
        Option<BookV2BodyFootnoteCandidate<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        Ok(
            fit_kernel::body_candidate(self, state, range.clone())?.map(|(height, projection)| {
                BookV2BodyFootnoteCandidate {
                    body: range,
                    body_height: height,
                    fit: BookV2BodyFootnoteFit::from_projection(state, projection),
                }
            }),
        )
    }
}

impl<'b, 'f, 's, 'p, 'a> fit_kernel::FitSearch for BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    type Region = BookV2FootnoteRegionSelection<'b, 'f, 's, 'p, 'a>;
    fn body(&self) -> Rect {
        if let Some(frames) = self.active_page_frames {
            return frames.body();
        }
        self.content
            .flow
            .lines()
            .frames()
            .expect("bound body frames")
            .body()
    }
    fn footnote_region(&self) -> Option<Rect> {
        if let Some(frames) = self.active_page_frames {
            return frames.footnote();
        }
        self.content
            .flow
            .lines()
            .frames()
            .expect("bound body frames")
            .footnote_region()
    }
    fn pending(&self, state: &Self::State) -> bool {
        !state.pending.is_empty()
    }
    fn required(
        &mut self,
        state: &Self::State,
        capacity: Length,
    ) -> Result<Option<Self::Region>, ProductionBodyPaginationError> {
        self.select_required_region(state, capacity)
    }
    fn resolve_unstarted(
        &mut self,
        state: &Self::State,
        capacity: Length,
    ) -> Result<Option<Self::Region>, ProductionBodyPaginationError> {
        self.select_dependency_closed_region(state, capacity)
    }
    fn fragments(region: &Self::Region) -> &[Self::Fragment] {
        region.fragments()
    }
    fn references(fragment: &Self::Fragment) -> impl Iterator<Item = (NodeId, usize)> {
        fragment
            .fragment()
            .references()
            .map(|r| (r.source().owner(), r.source().definition_index()))
    }
    fn unstarted(region: &Self::Region, definition: usize) -> bool {
        matches!(region.next_state().definitions[definition], BookV2Demand::Pending { cursor, .. } if
            if cursor.next_table_index().is_some() { !cursor.definition_started() } else { cursor.next_item()==0 })
    }
    fn height(region: &Self::Region) -> Length {
        region.used_height()
    }
    fn query_work(&mut self) -> Result<(), ProductionBodyPaginationError> {
        self.query_work()
    }
}
impl<'b, 'f, 's, 'p, 'a> fit_kernel::BodySearch<'b>
    for BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>
{
    fn items(&self) -> &'b [ProductionBodyFlowItem] {
        self.content.flow.body_items()
    }
    fn allow_range(
        &mut self,
        range: std::ops::Range<usize>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.validate_body_range(range)
    }
    fn keep_before(&mut self, start: usize) -> Result<bool, ProductionBodyPaginationError> {
        self.body_keep_before(start)
    }
    fn complete_references(
        &mut self,
        range: std::ops::Range<usize>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.query_work()?;
        fit_kernel::complete_references(
            self.content.flow.references_in_items(None, range.clone()),
            range,
            &mut self.content.steps,
            self.content.maximum_steps,
        )
    }
    fn require_body(
        &mut self,
        state: &Self::State,
        range: std::ops::Range<usize>,
    ) -> Result<Self::State, ProductionBodyPaginationError> {
        self.require_body(state, range)
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) fn evaluate_table_demand(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        fragment: Option<&BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>>,
    ) -> Result<Option<BookV2BodyFootnoteFit<'b, 'f, 's, 'p, 'a>>, ProductionBodyPaginationError>
    {
        Ok(fit_kernel::table_demand(self, state, fragment)?
            .map(|projection| BookV2BodyFootnoteFit::from_projection(state, projection)))
    }
}

impl<'b, 'f, 's, 'p, 'a> fit_kernel::TableSearch
    for BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>
{
    type TableFragment = BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>;
    fn verify_table(
        &self,
        fragment: &Self::TableFragment,
    ) -> Result<(), ProductionBodyPaginationError> {
        fragment.verify_demand_flow(self.content.flow)
    }
    fn ranges(fragment: &Self::TableFragment) -> impl Iterator<Item = std::ops::Range<usize>> {
        fragment.semantic_leaf_ranges()
    }
    fn table_height(fragment: &Self::TableFragment) -> Length {
        fragment.used_height()
    }
    fn fork_table(
        &mut self,
        state: &Self::State,
    ) -> Result<Self::State, ProductionBodyPaginationError> {
        self.fork(state, self.content.flow.footnotes().definitions().len())
    }
    fn require_range(
        &mut self,
        state: &mut Self::State,
        range: std::ops::Range<usize>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        self.query_work()?;
        let references = self.content.flow.references_in_items(None, range.clone());
        if !fit_kernel::complete_references(
            references,
            range,
            &mut self.content.steps,
            self.content.maximum_steps,
        )? {
            return Ok(false);
        }
        kernel::require(
            &mut self.content,
            &mut state.definitions,
            &mut state.pending,
            references,
        )?;
        Ok(true)
    }
}
