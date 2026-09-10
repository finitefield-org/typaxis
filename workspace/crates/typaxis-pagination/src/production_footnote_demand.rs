//! Branchable demand/continuation state beneath joint body/footnote pagination.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "production_body_footnote_fit_kernel.rs"]
mod fit_kernel;
#[path = "production_body_footnote_candidate.rs"]
mod body_candidate;
#[path = "production_body_mixed_kernel.rs"]
mod mixed_kernel;
#[path = "production_body_mixed_candidate.rs"]
mod mixed_candidate;
#[path = "production_body_mixed_pages.rs"]
mod mixed_pages;
#[path = "production_body_page_ranking.rs"]
mod page_ranking;
#[path = "production_body_mixed_placement.rs"]
mod mixed_placement;
#[path = "production_body_mixed_stability.rs"]
mod mixed_stability;
#[path="production_body_page_stability_kernel.rs"]
mod page_stability_kernel;
pub use mixed_candidate::{
    prepare_production_table_body_search, ProductionBodyCandidatePart,
    ProductionBodyMixedCandidate, ProductionBodySelectedPart,
};
pub use mixed_pages::{
    ProductionBodyMixedPageSelection, ProductionBodyMixedPageSequence, ProductionBodyMixedPageState,
};
pub use mixed_placement::{
    ProductionBodyMixedPlacedPage, ProductionBodyMixedPlacedSequence, ProductionTablePlacedCellRole,
};
pub use mixed_stability::ProductionBodyMixedStablePages;
#[path = "production_body_footnote_pages.rs"]
mod pages;
#[path = "production_body_footnote_placement.rs"]
mod placement;
pub use placement::{
    ProductionBodyFootnoteMathTerminals, ProductionBodyFootnotePlacedFragment,
    ProductionBodyFootnotePlacedMarker, ProductionBodyFootnotePlacedPage,
    ProductionBodyFootnotePlacedSequence, ProductionBodyFootnoteStablePages, ProductionFinalPage,
    ProductionFinalPageGeometry, ProductionFinalPageIter, ProductionFinalPages,
};
#[path = "production_footnote_required_kernel.rs"]
mod required_kernel;
#[path = "production_footnote_required_region.rs"]
mod required_region;
pub use body_candidate::ProductionBodyFootnoteCandidate;
pub(in crate::production_body::body_flow) use body_candidate::ProductionBodyFootnoteFit;
pub use pages::{
    ProductionBodyFootnotePageSelection, ProductionBodyFootnotePageSequence,
    ProductionBodyFootnotePageState,
};

#[path = "production_footnote_demand_kernel.rs"]
mod kernel;
#[path = "production_footnote_region_kernel.rs"]
mod region_kernel;
use kernel::DemandValue;
#[cfg(feature = "book-v2-staging")]
#[path = "book_v2_footnote_demand.rs"]
pub(super) mod book_v2;

static NEXT_SEARCH: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionFootnoteDemandStatus {
    Unreferenced,
    Pending,
    Complete,
}

type Demand<'b, 'f, 's, 'p, 'a> = DemandValue<ProductionFootnoteCursor<'b, 'f, 's, 'p, 'a>>;

/// Immutable branch state. Constructed only by its search owner, never a page
/// receipt. Keeping an earlier state permits another candidate evaluation.
pub struct ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a> {
    flow: &'b ProductionPreparedBodyFlow<'f, 's, 'p, 'a>,
    owner_id: u64,
    state_id: u64,
    definitions: Vec<Demand<'b, 'f, 's, 'p, 'a>>,
    pending: Vec<usize>,
}
impl ProductionFootnoteDemandState<'_, '_, '_, '_, '_> {
    pub fn status(&self, definition: usize) -> Option<ProductionFootnoteDemandStatus> {
        self.definitions.get(definition).map(|d| match d {
            Demand::Unreferenced => ProductionFootnoteDemandStatus::Unreferenced,
            Demand::Pending { .. } => ProductionFootnoteDemandStatus::Pending,
            Demand::Complete { .. } => ProductionFootnoteDemandStatus::Complete,
        })
    }
    pub fn first_reference(&self, definition: usize) -> Option<NodeId> {
        self.definitions
            .get(definition)
            .and_then(Demand::first_reference)
    }
    /// First-demand order, including an unfinished continuation at the front.
    pub fn pending_definitions(&self) -> &[usize] {
        &self.pending
    }
}

/// Actual content selection tied to the state from which it was evaluated.
/// Another branch cannot consume it even if both branches have the same cursor.
pub struct ProductionFootnoteDemandSelection<'b, 'f, 's, 'p, 'a> {
    owner_id: u64,
    state_id: u64,
    fragment: ProductionFootnoteFragmentSelection<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSelection<'b, 'f, 's, 'p, 'a> {
    pub fn fragment(&self) -> &ProductionFootnoteFragmentSelection<'b, 'f, 's, 'p, 'a> {
        &self.fragment
    }
}

/// One cumulative budget for snapshots, demand traversal and content attempts.
/// Failed/dropped candidates do not change their input state or refund work.
pub struct ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    content: ProductionFootnoteBreakSearch<'b, 'f, 's, 'p, 'a>,
    tables:
        Option<super::super::table_measurements::ProductionTableBodyContext<'b, 'f, 's, 'p, 'a>>,
    owner_id: u64,
    next_state: u64,
    maximum_pages: u32,
    maximum_reflows: u16,
    maximum_passes: u16,
    terminal_spool: u64,
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn record_charge(&self) -> u64 {
        self.content.record_charge()
    }
    pub(in crate::production_body::body_flow) fn swap_table_budget(
        &mut self,
        charge: &mut Charge,
        steps: &mut u64,
    ) {
        std::mem::swap(&mut self.content.charge, charge);
        std::mem::swap(&mut self.content.steps, steps);
    }
    pub(in crate::production_body::body_flow) fn fork_table_state(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.fork(state, 0)
    }
    pub(in crate::production_body::body_flow) fn verify_table_state(
        &self,
        state: &ProductionFootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.verify_state(state)
    }

    pub fn work_steps(&self) -> u64 {
        self.content.visited_items()
    }
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        visit(&mut self.content.steps, self.content.maximum_steps, owner)
    }
    fn state_id(&mut self) -> Result<u64, ProductionBodyPaginationError> {
        let id = self.next_state;
        self.next_state = id
            .checked_add(1)
            .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow))?;
        Ok(id)
    }
    fn verify_state(
        &self,
        state: &ProductionFootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.owner_id != state.owner_id || !std::ptr::eq(self.content.flow, state.flow) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
    pub fn begin(
        &mut self,
    ) -> Result<ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let flow = self.content.flow;
        let definitions = kernel::begin_definitions(&mut self.content)?;
        Ok(ProductionFootnoteDemandState {
            flow,
            owner_id: self.owner_id,
            state_id: self.state_id()?,
            definitions,
            pending: Vec::new(),
        })
    }
    fn fork(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        additional: usize,
    ) -> Result<ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.verify_state(state)?;
        let (definitions, pending) = kernel::fork_definitions(
            &mut self.content,
            &state.definitions,
            &state.pending,
            additional,
        )?;
        Ok(ProductionFootnoteDemandState {
            flow: state.flow,
            owner_id: self.owner_id,
            state_id: self.state_id()?,
            definitions,
            pending,
        })
    }
    fn require(
        &mut self,
        state: &mut ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        references: &[ProductionFootnoteFlowReference<'f>],
    ) -> Result<(), ProductionBodyPaginationError> {
        kernel::require(
            &mut self.content,
            &mut state.definitions,
            &mut state.pending,
            references,
        )
    }
    fn body_keep_before(&mut self, start: usize) -> Result<bool, ProductionBodyPaginationError> {
        if start == 0 {
            return Ok(false);
        }
        self.table_query_work()?;
        if self.tables.as_ref().is_some_and(|t| t.keep_before(start)) {
            return Ok(true);
        }
        if !self.content.flow.body_items()[start - 1].keep {
            return Ok(false);
        }
        self.table_query_work()?;
        Ok(self
            .tables
            .as_ref()
            .is_none_or(|t| !t.contains(start - 1..start)))
    }
    fn table_query_work(&mut self) -> Result<(), ProductionBodyPaginationError> {
        let steps = self.tables.as_ref().map_or(0, |t| t.query_steps());
        for _ in 0..steps {
            self.step(NodeId::new(0))?;
        }
        Ok(())
    }
    fn query_work(&mut self) -> Result<(), ProductionBodyPaginationError> {
        // Upper bound on the two binary searches over actual occurrences.
        let bits = usize::BITS - self.content.flow.references().len().max(1).leading_zeros();
        for _ in 0..2 * bits {
            self.step(NodeId::new(0))?;
        }
        Ok(())
    }
    /// Adds actual body occurrences from a candidate range. Bounds are checked,
    /// but this method does not certify keep/break legality or advance body pages.
    pub fn require_body(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        range: std::ops::Range<usize>,
    ) -> Result<ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.verify_state(state)?;
        if range.start > range.end || range.end > self.content.flow.body_items().len() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        self.table_query_work()?;
        if self
            .tables
            .as_ref()
            .is_some_and(|t| t.contains(range.clone()))
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        self.query_work()?;
        let references = self.content.flow.references_in_items(None, range);
        let mut result = self.fork(state, references.len())?;
        self.require(&mut result, references)?;
        Ok(result)
    }
    /// None is either an empty pending queue or no fragment fitting the capacity.
    /// The input state's pending queue distinguishes those cases.
    pub fn evaluate_next(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<ProductionFootnoteDemandSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.verify_state(state)?;
        let Some(index) = state.pending.first() else {
            return Ok(None);
        };
        let Demand::Pending { cursor, .. } = state.definitions[*index] else {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        };
        self.content
            .charge
            .take(1, state.flow.footnotes.definitions()[*index].owner())?;
        Ok(self.content.evaluate(&cursor, available)?.map(|fragment| {
            ProductionFootnoteDemandSelection {
                owner_id: self.owner_id,
                state_id: state.state_id,
                fragment,
            }
        }))
    }
    pub fn advance(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        selected: &ProductionFootnoteDemandSelection<'b, 'f, 's, 'p, 'a>,
    ) -> Result<ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.advance_at(state, selected, 0)
    }
    fn advance_at(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        selected: &ProductionFootnoteDemandSelection<'b, 'f, 's, 'p, 'a>,
        pending_position: usize,
    ) -> Result<ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.verify_state(state)?;
        let fragment = &selected.fragment;
        fragment.verify(self.content.flow)?;
        if selected.owner_id != self.owner_id
            || selected.state_id != state.state_id
            || state.pending.get(pending_position) != Some(&fragment.definition_index())
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        let index = fragment.definition_index();
        let Demand::Pending {
            first_reference,
            cursor,
        } = state.definitions[index]
        else {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        };
        if cursor.next_item != fragment.cursor.next_item {
            return Err(error(first_reference, E::ReceiptMismatch));
        }
        self.query_work()?;
        let references = self
            .content
            .flow
            .references_in_items(Some(index), fragment.cursor.next_item..fragment.content_end);
        let mut result = self.fork(state, references.len())?;
        kernel::advance_definition(
            &mut self.content,
            &mut result.definitions,
            &mut result.pending,
            index,
            pending_position,
            first_reference,
            fragment.continuation(),
        )?;
        self.require(&mut result, references)?;
        Ok(result)
    }
}

pub fn prepare_production_footnote_demand_search<'b, 'f, 's, 'p, 'a>(
    flow: &'b ProductionPreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work_steps: u64,
) -> Result<ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let content = prepare_production_footnote_search(flow, limits, maximum_work_steps)?;
    finish_demand_search(content, limits)
}
pub(in crate::production_body::body_flow) fn prepare_table_demand_search<'b, 'f, 's, 'p, 'a>(
    flow: &'b ProductionPreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work_steps: u64,
    charge: Charge,
    steps: u64,
) -> Result<ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let content =
        prepare_footnote_search_with_budget(flow, limits, maximum_work_steps, charge, steps)?;
    finish_demand_search(content, limits)
}
fn finish_demand_search<'b, 'f, 's, 'p, 'a>(
    mut content: ProductionFootnoteBreakSearch<'b, 'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    content.charge.take(1, NodeId::new(0))?;
    let owner_id = NEXT_SEARCH
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| error(NodeId::new(0), E::ArithmeticOverflow))?;
    Ok(ProductionFootnoteDemandSearch {
        content,
        tables: None,
        owner_id,
        next_state: 0,
        terminal_spool: 0,
        maximum_pages: limits.base().get().max_pages,
        maximum_passes: limits.base().get().max_layout_passes,
        maximum_reflows: limits.base().get().max_footnote_reflows_per_page,
    })
}

/// A content fragment at a measured offset inside a candidate footnote region.
/// No physical page origin or PDF paint permission is assigned here.
pub struct ProductionFootnoteRegionFragment<'b, 'f, 's, 'p, 'a> {
    offset: Length,
    selected: ProductionFootnoteDemandSelection<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteRegionFragment<'b, 'f, 's, 'p, 'a> {
    pub const fn offset(&self) -> Length {
        self.offset
    }
    pub fn fragment(&self) -> &ProductionFootnoteFragmentSelection<'b, 'f, 's, 'p, 'a> {
        self.selected.fragment()
    }
}

pub struct ProductionFootnoteRegionSelection<'b, 'f, 's, 'p, 'a> {
    owner_id: u64,
    state_id: u64,
    fragments: Vec<ProductionFootnoteRegionFragment<'b, 'f, 's, 'p, 'a>>,
    used_height: Length,
    available_height: Length,
    forced_break_owner: Option<NodeId>,
    next_state: ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteRegionSelection<'b, 'f, 's, 'p, 'a> {
    pub fn fragments(&self) -> &[ProductionFootnoteRegionFragment<'b, 'f, 's, 'p, 'a>] {
        &self.fragments
    }
    pub const fn used_height(&self) -> Length {
        self.used_height
    }
    pub const fn available_height(&self) -> Length {
        self.available_height
    }
    pub const fn forced_break_owner(&self) -> Option<NodeId> {
        self.forced_break_owner
    }
    pub fn next_state(&self) -> &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a> {
        &self.next_state
    }
    pub fn verify(
        &self,
        state: &ProductionFootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.owner_id != state.owner_id
            || self.state_id != state.state_id
            || !std::ptr::eq(self.next_state.flow, state.flow)
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}

impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Select pending content in order, including authored inter-definition
    /// spacing. Overflow/forced boundaries end this region. Remaining demands
    /// stay in next_state; the body-page owner must decide whether they make its
    /// candidate invalid. This method alone does not establish same-page fit.
    pub fn select_region(
        &mut self,
        state: &ProductionFootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<ProductionFootnoteRegionSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        Ok(
            region_kernel::select(self, state, available)?.map(|region| {
                ProductionFootnoteRegionSelection {
                    owner_id: self.owner_id,
                    state_id: state.state_id,
                    fragments: region.fragments,
                    used_height: region.used_height,
                    available_height: available,
                    forced_break_owner: region.forced_break_owner,
                    next_state: region.next_state,
                }
            }),
        )
    }
}
