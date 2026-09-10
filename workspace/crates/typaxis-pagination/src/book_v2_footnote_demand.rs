//! Successor demand branches, using the common snapshot/queue transition kernel.
use super::super::book_v2::{
    prepare_book_v2_footnote_context_charged, prepare_book_v2_footnote_search,
    prepare_book_v2_footnote_search_charged, BookV2FootnoteBreakSearch, BookV2FootnoteCursor,
    BookV2FootnoteFragmentSelection,
};
use super::*;
use crate::production_body::body_flow::book_v2::BookV2PreparedBodyFlow;
use kernel::DemandContent;
#[path = "book_v2_body_mixed_candidate.rs"]
mod mixed_candidate;
#[path = "book_v2_body_mixed_pages.rs"]
mod mixed_pages;
#[path = "book_v2_body_mixed_placement.rs"]
mod mixed_placement;
#[path = "book_v2_body_header_placement.rs"]
mod header_placement;
pub use header_placement::BookV2BodyPlacedHeaderVariant;
#[path = "book_v2_body_mixed_stability.rs"]
mod mixed_stability;
pub use mixed_candidate::{
    BookV2BodyCandidatePart, BookV2BodyMixedCandidate, BookV2BodySelectedPart,
    BookV2BodySourceState,
};
pub use mixed_pages::{
    BookV2BodyMixedPageSelection, BookV2BodyMixedPageSequence, BookV2BodyMixedPageState,
};
pub use mixed_placement::{
    BookV2BodyMixedPlacedPage, BookV2BodyMixedPlacedSequence, BookV2BodyPlacedEquationNumber,
};
pub use mixed_stability::BookV2BodyMixedStablePages;
#[path = "book_v2_body_source_closure.rs"]
mod source_closure;
pub use source_closure::BookV2BodySourceClosure;
#[path = "book_v2_paragraph_width_feedback.rs"]
mod paragraph_width_feedback;
pub use paragraph_width_feedback::{BookV2TableWidthSource, BookV2TableWidthPiece, BookV2TableWidthOccurrence, BookV2TableWidthOccurrences, BookV2ParagraphWidthCandidate, BookV2ParagraphWidthFeedback};
#[path = "book_v2_body_math_terminals.rs"]
mod math_terminals;
pub use math_terminals::{
    BookV2BodyMathSource, BookV2BodyMathTerminal, BookV2BodyMathTerminals,
    BOOK_V2_BODY_MATH_TERMINAL_ALGORITHM,
};

#[path = "book_v2_body_footnote_candidate.rs"]
mod body_candidate;
pub use body_candidate::BookV2BodyFootnoteCandidate;
pub(in crate::production_body::body_flow) use body_candidate::BookV2BodyFootnoteFit;

#[path = "book_v2_footnote_region.rs"]
mod region;
pub use region::{BookV2FootnoteRegionFragment, BookV2FootnoteRegionSelection};

type BookV2Demand<'b, 'f, 's, 'p, 'a> = DemandValue<BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a>>;

pub struct BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a> {
    flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    owner_id: u64,
    state_id: u64,
    definitions: Vec<BookV2Demand<'b, 'f, 's, 'p, 'a>>,
    pending: Vec<usize>,
}
impl BookV2FootnoteDemandState<'_, '_, '_, '_, '_> {
    pub fn status(&self, index: usize) -> Option<ProductionFootnoteDemandStatus> {
        self.definitions.get(index).map(|d| match d {
            DemandValue::Unreferenced => ProductionFootnoteDemandStatus::Unreferenced,
            DemandValue::Pending { .. } => ProductionFootnoteDemandStatus::Pending,
            DemandValue::Complete { .. } => ProductionFootnoteDemandStatus::Complete,
        })
    }
    pub fn first_reference(&self, index: usize) -> Option<NodeId> {
        self.definitions
            .get(index)
            .and_then(DemandValue::first_reference)
    }
    pub fn pending_definitions(&self) -> &[usize] {
        &self.pending
    }
}
pub struct BookV2FootnoteDemandSelection<'b, 'f, 's, 'p, 'a> {
    owner_id: u64,
    state_id: u64,
    fragment: BookV2FootnoteFragmentSelection<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSelection<'b, 'f, 's, 'p, 'a> {
    pub fn fragment(&self) -> &BookV2FootnoteFragmentSelection<'b, 'f, 's, 'p, 'a> {
        &self.fragment
    }
}
/// Exact source/search/branch ownership, without a page or paint receipt.
pub struct BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    content: BookV2FootnoteBreakSearch<'b, 'f, 's, 'p, 'a>,
    owner_id: u64,
    next_state: u64,
    maximum_pages: u32,
    maximum_passes: u16,
    maximum_reflows: u16,
    terminal_spool: u64,
    active_page_frames: Option<typaxis_syntax::book_v2::BookV2PageFrames>,
    active_page_name: Option<usize>,
    headers: Option<&'b crate::book_v2::BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a>>,
    definition_tables: Option<Vec<Option<crate::production_body::body_flow::table_measurements::BookV2DefinitionTableContext<'b, 'f, 's, 'p, 'a>>>>,
    tables: Option<
        crate::production_body::body_flow::table_measurements::BookV2TableBodyContext<
            'b,
            'f,
            's,
            'p,
            'a,
        >,
    >,
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
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
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.fork(state, 0)
    }
    pub(in crate::production_body::body_flow) fn verify_table_state(
        &self,
        state: &BookV2FootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.verify_state(state)
    }
    /// Bind a sealed frame catalog before creating source/demand branches.
    pub fn set_table_header_catalog(
        &mut self,
        headers: &'b crate::book_v2::BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.bind_table_header_catalog(headers, false)
    }
    fn bind_table_header_catalog(
        &mut self,
        headers: &'b crate::book_v2::BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a>,
        prepaid: bool,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        if self.next_state != 0
            || self.headers.is_some()
            || self.tables.is_none()
            || !std::ptr::eq(headers.base().flow(), self.content.flow)
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        self.content.step(root)?;
        self.content.flow.verify(
            self.content.flow.lines(),
            self.content.flow.blocks(),
            self.content.flow.footnotes(),
            headers.limits(),
        )?;
        // Catalog graphs coexist with search state; retain both charges even
        // when their historical shared-input charges overlap.
        if !prepaid {
            let extra = usize::try_from(headers.record_charge())
                .map_err(|_| error(root, E::FragmentLimit))?;
            self.content.charge.take(extra, root)?;
        }
        self.headers = Some(headers);
        Ok(())
    }
    pub fn record_charge(&self) -> u64 {
        self.content.record_charge()
    }
    pub fn work_steps(&self) -> u64 {
        self.content.visited_items()
    }
    fn verify_state(
        &self,
        state: &BookV2FootnoteDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.owner_id != state.owner_id || !std::ptr::eq(self.content.flow, state.flow) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
    fn state_id(&mut self) -> Result<u64, ProductionBodyPaginationError> {
        let id = self.next_state;
        self.next_state = id
            .checked_add(1)
            .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow))?;
        Ok(id)
    }
    fn query_work(&mut self) -> Result<(), ProductionBodyPaginationError> {
        let bits = usize::BITS - self.content.flow.references().len().max(1).leading_zeros();
        for _ in 0..2 * bits {
            self.content.step(NodeId::new(0))?;
        }
        Ok(())
    }
    pub fn begin(
        &mut self,
    ) -> Result<BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let definitions = kernel::begin_definitions(&mut self.content)?;
        Ok(BookV2FootnoteDemandState {
            flow: self.content.flow,
            owner_id: self.owner_id,
            state_id: self.state_id()?,
            definitions,
            pending: Vec::new(),
        })
    }
    fn fork(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        additional: usize,
    ) -> Result<BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.verify_state(state)?;
        let (definitions, pending) = kernel::fork_definitions(
            &mut self.content,
            &state.definitions,
            &state.pending,
            additional,
        )?;
        Ok(BookV2FootnoteDemandState {
            flow: state.flow,
            owner_id: self.owner_id,
            state_id: self.state_id()?,
            definitions,
            pending,
        })
    }
    fn validate_body_range(
        &mut self,
        range: std::ops::Range<usize>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let flow = self.content.flow;
        if range.start > range.end || range.end > flow.body_items().len() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        for table in &flow.collected.tables.tables {
            self.content.step(table.owner)?;
            if range.start < range.end
                && table.items.start < range.end
                && range.start < table.items.end
            {
                return Err(error(table.owner, E::PendingRegion("table_body_demand")));
            }
        }
        Ok(())
    }
    fn body_keep_before(&mut self, start: usize) -> Result<bool, ProductionBodyPaginationError> {
        if start == 0 {
            return Ok(false);
        }
        let flow = self.content.flow;
        let mut previous_in_table = false;
        let mut table_keep = false;
        for table in &flow.collected.tables.tables {
            self.content.step(table.owner)?;
            // Only top-level body boundaries join the following body sibling.
            if table.parent.is_some() || table.definition.is_some() {
                continue;
            }
            previous_in_table |= table.items.start < start && start <= table.items.end;
            table_keep |= table.items.end == start && table.keep;
        }
        Ok(table_keep || (!previous_in_table && flow.body_items()[start - 1].keep))
    }
    /// A read-only candidate body range demands its actual references. This
    /// does not certify a page break or permit flattening table cell streams.
    pub fn require_body(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        range: std::ops::Range<usize>,
    ) -> Result<BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.verify_state(state)?;
        let flow = self.content.flow;
        self.validate_body_range(range.clone())?;
        self.query_work()?;
        let references = flow.references_in_items(None, range);
        let mut next = self.fork(state, references.len())?;
        kernel::require(
            &mut self.content,
            &mut next.definitions,
            &mut next.pending,
            references,
        )?;
        Ok(next)
    }
    pub fn evaluate_next(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<BookV2FootnoteDemandSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.verify_state(state)?;
        if self.definition_tables.is_some() {
            return self.evaluate_mixed_definition(state, available);
        }
        let Some(index) = state.pending.first() else {
            return Ok(None);
        };
        let DemandValue::Pending { cursor, .. } = state.definitions[*index] else {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        };
        self.content
            .charge(1, self.content.definition_owner(*index)?)?;
        Ok(self.content.evaluate(&cursor, available)?.map(|fragment| {
            BookV2FootnoteDemandSelection {
                owner_id: self.owner_id,
                state_id: state.state_id,
                fragment,
            }
        }))
    }
    pub fn advance(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        selected: &BookV2FootnoteDemandSelection<'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.advance_at(state, selected, 0)
    }
    fn advance_at(
        &mut self,
        state: &BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>,
        selected: &BookV2FootnoteDemandSelection<'b, 'f, 's, 'p, 'a>,
        pending_position: usize,
    ) -> Result<BookV2FootnoteDemandState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.verify_state(state)?;
        let fragment = &selected.fragment;
        fragment.verify(self.content.flow)?;
        if selected.owner_id != self.owner_id
            || selected.state_id != state.state_id
            || state.pending.get(pending_position) != Some(&fragment.definition_index())
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        if let Some(mixed) = fragment.mixed() {
            mixed.verify_demand(state)?;
            return self.fork(mixed.next_state().demand(), 0);
        }
        let index = fragment.definition_index();
        let DemandValue::Pending {
            first_reference,
            cursor,
        } = state.definitions[index]
        else {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        };
        let start = fragment.consumed_range()?.start;
        if cursor.next_item() != start {
            return Err(error(first_reference, E::ReceiptMismatch));
        }
        self.query_work()?;
        let references = self
            .content
            .flow
            .references_in_items(Some(index), start..start + fragment.items()?.len());
        let mut next = self.fork(state, references.len())?;
        kernel::advance_definition(
            &mut self.content,
            &mut next.definitions,
            &mut next.pending,
            index,
            pending_position,
            first_reference,
            fragment.continuation(),
        )?;
        kernel::require(
            &mut self.content,
            &mut next.definitions,
            &mut next.pending,
            references,
        )?;
        Ok(next)
    }
}
pub fn prepare_book_v2_footnote_demand_search<'b, 'f, 's, 'p, 'a>(
    flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let content = prepare_book_v2_footnote_search(flow, limits, maximum_work, prior_records)?;
    finish_book_v2_demand_search(content, limits)
}
pub(in crate::production_body::body_flow) fn prepare_book_v2_table_demand_search<
    'b,
    'f,
    's,
    'p,
    'a,
>(
    flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    charge: Charge,
    steps: u64,
) -> Result<BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    finish_book_v2_demand_search(
        prepare_book_v2_footnote_search_charged(flow, limits, maximum_work, charge, steps)?,
        limits,
    )
}
pub(in crate::production_body::body_flow) fn prepare_book_v2_definition_candidate_demand<
    'b,
    'f,
    's,
    'p,
    'a,
>(
    flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    charge: Charge,
    steps: u64,
) -> Result<BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    finish_book_v2_demand_search(
        prepare_book_v2_footnote_context_charged(flow, limits, maximum_work, charge, steps, true)?,
        limits,
    )
}
fn finish_book_v2_demand_search<'b, 'f, 's, 'p, 'a>(
    mut content: BookV2FootnoteBreakSearch<'b, 'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
) -> Result<BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    content.charge(1, NodeId::new(0))?;
    let owner_id = NEXT_SEARCH
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| error(NodeId::new(0), E::ArithmeticOverflow))?;
    Ok(BookV2FootnoteDemandSearch {
        content,
        owner_id,
        next_state: 0,
        maximum_pages: limits.base().get().max_pages,
        maximum_passes: limits.base().get().max_layout_passes,
        maximum_reflows: limits.base().get().max_footnote_reflows_per_page,
        terminal_spool: 0,
        active_page_frames: None,
        active_page_name: None,
        headers: None,
        definition_tables: None,
        tables: None,
    })
}

/// Prepare every actual body table once, with the same cumulative budget used
/// by subsequent body/definition demand branches. No pages are issued here.
pub fn prepare_book_v2_table_body_search<'b, 'f, 's, 'p, 'a>(
    measurements: &'b crate::production_body::body_flow::book_v2::BookV2TableMeasurements<
        'f,
        's,
        'p,
        'a,
    >,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    if measurements.flow().collected.tables.tables.last().is_some_and(|table| table.definition.is_some()) {
        let (tables, definitions, charge, steps) =
            crate::production_body::body_flow::table_measurements::BookV2TableBodyContext::prepare_joint(
                measurements, limits, maximum_work, prior_records,
            )?;
        let mut search = prepare_book_v2_definition_candidate_demand(
            measurements.flow(), limits, maximum_work, charge, steps,
        )?;
        search.tables = Some(tables);
        search.definition_tables = Some(definitions);
        return Ok(search);
    }
    let (tables, charge, steps) =
        crate::production_body::body_flow::table_measurements::BookV2TableBodyContext::prepare(
            measurements,
            limits,
            maximum_work,
            prior_records,
        )?;
    let mut search = prepare_book_v2_table_demand_search(
        measurements.flow(),
        limits,
        maximum_work,
        charge,
        steps,
    )?;
    search.tables = Some(tables);
    Ok(search)
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn body_table_count(&self) -> usize {
        self.tables.as_ref().map_or(0, |tables| tables.len())
    }
    pub fn body_table_range(&self, index: usize) -> Option<std::ops::Range<usize>> {
        self.tables.as_ref().and_then(|tables| tables.range(index))
    }
    pub fn begin_table(
        &mut self,
        index: usize,
    ) -> Result<
        crate::production_body::body_flow::book_v2::BookV2TableCursor<'b, 'f, 's, 'p, 'a>,
        ProductionBodyPaginationError,
    > {
        self.content.step(NodeId::new(0))?;
        let tables = self
            .tables
            .as_mut()
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        tables.begin(index, &mut self.content.charge, &mut self.content.steps)
    }
}

#[path = "book_v2_definition_table_demand_transition.rs"]
mod definition_table_transition;

#[path = "book_v2_definition_mixed_candidate.rs"]
mod definition_mixed;
pub use definition_mixed::{BookV2DefinitionCandidates, BookV2RankedDefinitionCandidate, prepare_book_v2_definition_mixed_search, BookV2DefinitionMixedSearch, BookV2DefinitionCandidatePart, BookV2DefinitionSelectedPart, BookV2DefinitionSourceState, BookV2DefinitionMixedCandidate,};

#[path = "book_v2_definition_queue.rs"]
mod definition_queue;
pub use definition_queue::prepare_book_v2_mixed_footnote_demand_search;


/// Construct search directly from its sealed catalog. Its complete retained
/// graph ledger is prepaid before search allocation, so binding cannot charge
/// that same history a second time. The independent setter remains conservative.
pub fn prepare_book_v2_table_body_search_with_headers<'b, 'f, 's, 'p, 'a>(
    catalog: &'b crate::book_v2::BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let mut search=prepare_book_v2_table_body_search(catalog.base(),limits,maximum_work,prior_records.max(catalog.record_charge()))?;
    search.bind_table_header_catalog(catalog,true)?;
    Ok(search)
}
