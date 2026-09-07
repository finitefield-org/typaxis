//! Branchable demand/continuation state beneath joint body/footnote pagination.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "production_body_footnote_candidate.rs"]
mod body_candidate;
#[path = "production_body_footnote_pages.rs"]
mod pages;
#[path = "production_body_footnote_placement.rs"]
mod placement;
pub use placement::{
    ProductionBodyFootnoteMathTerminals, ProductionBodyFootnotePlacedFragment,
    ProductionBodyFootnotePlacedMarker, ProductionBodyFootnotePlacedPage,
    ProductionBodyFootnotePlacedSequence, ProductionBodyFootnoteStablePages,
};
#[path = "production_footnote_required_region.rs"]
mod required_region;
pub use body_candidate::ProductionBodyFootnoteCandidate;
pub use pages::{
    ProductionBodyFootnotePageSelection, ProductionBodyFootnotePageSequence,
    ProductionBodyFootnotePageState,
};

static NEXT_SEARCH: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionFootnoteDemandStatus {
    Unreferenced,
    Pending,
    Complete,
}

#[derive(Clone, Copy)]
enum Demand<'b, 'f, 's, 'p, 'a> {
    Unreferenced,
    Pending {
        first_reference: NodeId,
        cursor: ProductionFootnoteCursor<'b, 'f, 's, 'p, 'a>,
    },
    Complete {
        first_reference: NodeId,
    },
}
impl Demand<'_, '_, '_, '_, '_> {
    fn first_reference(&self) -> Option<NodeId> {
        match self {
            Self::Unreferenced => None,
            Self::Pending {
                first_reference, ..
            }
            | Self::Complete { first_reference } => Some(*first_reference),
        }
    }
}

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
        let root = NodeId::new(0);
        let flow = self.content.flow;
        self.content.charge.take(1, root)?;
        self.content
            .charge
            .take(flow.footnotes.definitions().len(), root)?;
        let mut definitions = Vec::new();
        definitions
            .try_reserve_exact(flow.footnotes.definitions().len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        for definition in flow.footnotes.definitions() {
            self.step(definition.owner())?;
            definitions.push(Demand::Unreferenced);
        }
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
        let root = NodeId::new(0);
        self.content.charge.take(1, root)?;
        self.content.charge.take(state.definitions.len(), root)?;
        self.content.charge.take(state.pending.len(), root)?;
        let mut definitions = Vec::new();
        definitions
            .try_reserve_exact(state.definitions.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        for (index, demand) in state.definitions.iter().enumerate() {
            self.step(state.flow.footnotes.definitions()[index].owner())?;
            definitions.push(*demand);
        }
        let capacity = state
            .pending
            .len()
            .checked_add(additional.min(state.definitions.len()))
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?
            .min(state.definitions.len());
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(capacity)
            .map_err(|_| error(root, E::AllocationFailure))?;
        for index in &state.pending {
            self.step(state.flow.footnotes.definitions()[*index].owner())?;
            pending.push(*index);
        }
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
        for reference in references {
            let source = reference.source();
            self.step(source.owner())?;
            let index = source.definition_index();
            let slot = state
                .definitions
                .get_mut(index)
                .ok_or_else(|| error(source.owner(), E::ReceiptMismatch))?;
            if matches!(slot, Demand::Unreferenced) {
                let cursor = self.content.begin(index)?;
                self.content.charge.take(1, source.owner())?;
                *slot = Demand::Pending {
                    first_reference: source.owner(),
                    cursor,
                };
                state.pending.push(index);
            }
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
        if let Some(cursor) = fragment.continuation() {
            result.definitions[index] = Demand::Pending {
                first_reference,
                cursor,
            };
        } else {
            result.definitions[index] = Demand::Complete { first_reference };
            // Charge the records shifted by removal as work, not new retention.
            for pending in result.pending.iter().skip(pending_position + 1) {
                self.step(state.flow.footnotes.definitions()[*pending].owner())?;
            }
            result.pending.remove(pending_position);
        }
        self.require(&mut result, references)?;
        Ok(result)
    }
}

pub fn prepare_production_footnote_demand_search<'b, 'f, 's, 'p, 'a>(
    flow: &'b ProductionPreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_work_steps: u64,
) -> Result<ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let mut content = prepare_production_footnote_search(flow, limits, maximum_work_steps)?;
    content.charge.take(1, NodeId::new(0))?;
    let owner_id = NEXT_SEARCH
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| error(NodeId::new(0), E::ArithmeticOverflow))?;
    Ok(ProductionFootnoteDemandSearch {
        content,
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
        self.verify_state(state)?;
        let root = NodeId::new(0);
        if available < Length::ZERO || available > self.content.maximum_height {
            return Err(error(root, E::InvalidFootnoteCapacity));
        }
        self.content.charge.take(1, root)?;
        if state.pending.is_empty() {
            return Ok(None);
        }
        let mut fragments: Vec<ProductionFootnoteRegionFragment<'b, 'f, 's, 'p, 'a>> = Vec::new();
        let mut current = None;
        let mut used = Length::ZERO;
        let mut forced_break_owner = None;
        loop {
            let before = current.as_ref().unwrap_or(state);
            let Some(index) = before.pending.first().copied() else {
                break;
            };
            let Demand::Pending { cursor, .. } = before.definitions[index] else {
                return Err(error(root, E::ReceiptMismatch));
            };
            let next = &before
                .flow
                .definition_items(index)
                .ok_or_else(|| error(root, E::ReceiptMismatch))?[cursor.next_item];
            self.step(next.owner)?;
            let gap = if next.source.is_some() {
                fragments
                    .last()
                    .and_then(|f| f.fragment().items().last())
                    .map(|previous| add(previous.after, next.before, next.owner))
                    .transpose()?
                    .unwrap_or(Length::ZERO)
            } else {
                Length::ZERO
            };
            let offset = add(used, gap, next.owner)?;
            if offset > available {
                break;
            }
            let remaining = available
                .checked_sub(offset)
                .ok_or_else(|| error(next.owner, E::ArithmeticOverflow))?;
            let Some(selected) = self.evaluate_next(before, remaining)? else {
                break;
            };
            let end = add(offset, selected.fragment.used_height, next.owner)?;
            if end > available {
                return Err(error(next.owner, E::ReceiptMismatch));
            }
            let stop = selected.fragment.reason != ProductionBodyBreakReason::End;
            forced_break_owner = selected.fragment.forced_break_owner;
            let after = self.advance(before, &selected)?;
            self.content.charge.take(1, next.owner)?;
            fragments
                .try_reserve(1)
                .map_err(|_| error(next.owner, E::AllocationFailure))?;
            fragments.push(ProductionFootnoteRegionFragment { offset, selected });
            current = Some(after);
            used = end;
            if stop {
                break;
            }
        }
        Ok(current.map(|next_state| ProductionFootnoteRegionSelection {
            owner_id: self.owner_id,
            state_id: state.state_id,
            fragments,
            used_height: used,
            available_height: available,
            forced_break_owner,
            next_state,
        }))
    }
}
