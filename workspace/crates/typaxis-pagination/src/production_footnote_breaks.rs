//! Content-fragment candidates for one definition, without page assignment.
use super::*;
use std::collections::BTreeSet;

#[path = "production_footnote_demand.rs"]
mod demand;
pub use demand::{
    prepare_production_footnote_demand_search, ProductionBodyFootnoteCandidate,
    ProductionBodyFootnoteMathTerminals, ProductionBodyFootnotePageSelection,
    ProductionBodyFootnotePageSequence, ProductionBodyFootnotePageState,
    ProductionBodyFootnotePlacedFragment, ProductionBodyFootnotePlacedMarker,
    ProductionBodyFootnotePlacedPage, ProductionBodyFootnotePlacedSequence,
    ProductionBodyFootnoteStablePages, ProductionFootnoteDemandSearch,
    ProductionFootnoteDemandSelection, ProductionFootnoteDemandState,
    ProductionFootnoteDemandStatus, ProductionFootnoteRegionFragment,
    ProductionFootnoteRegionSelection,
};

#[derive(Clone, Copy)]
pub struct ProductionFootnoteCursor<'b, 'f, 's, 'p, 'a> {
    flow: &'b ProductionPreparedBodyFlow<'f, 's, 'p, 'a>,
    definition_index: usize,
    next_item: usize,
}
impl ProductionFootnoteCursor<'_, '_, '_, '_, '_> {
    pub const fn definition_index(&self) -> usize {
        self.definition_index
    }
    pub const fn next_item(&self) -> usize {
        self.next_item
    }
}

/// A bounded selection of measured content, not a page or footnote paint receipt.
/// Definition-marker extents are included. Cross-definition spacing, page
/// reservation and reference-page coupling still belong to the final page owner.
pub struct ProductionFootnoteFragmentSelection<'b, 'f, 's, 'p, 'a> {
    cursor: ProductionFootnoteCursor<'b, 'f, 's, 'p, 'a>,
    content_end: usize,
    consumed_end: usize,
    used_height: Length,
    available_height: Length,
    reason: ProductionBodyBreakReason,
    forced_break_owner: Option<NodeId>,
    choice: Option<page_breaks::BoundarySelection>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteFragmentSelection<'b, 'f, 's, 'p, 'a> {
    pub const fn definition_index(&self) -> usize {
        self.cursor.definition_index
    }
    pub fn items(&self) -> &[ProductionBodyFlowItem] {
        &self
            .cursor
            .flow
            .definition_items(self.cursor.definition_index)
            .expect("bound definition")[self.cursor.next_item..self.content_end]
    }
    /// The label belongs only to the selection containing its first real item.
    /// Continuation selections do not repeat it; leading forced breaks do not
    /// consume it before any paintable content is selected.
    pub fn marker(&self) -> Option<&ProductionFootnoteMarkerBinding> {
        self.cursor
            .flow
            .definition_marker(self.cursor.definition_index)
            .filter(|marker| {
                (self.cursor.next_item..self.content_end).contains(&marker.item_index())
            })
    }
    /// References whose actual glyph-bearing lines intersect this selection.
    /// These are occurrences, not a deduplicated reservation or page assignment.
    pub fn references(&self) -> impl Iterator<Item = &ProductionFootnoteFlowReference<'f>> {
        self.cursor
            .flow
            .references_in_items(
                Some(self.cursor.definition_index),
                self.cursor.next_item..self.content_end,
            )
            .iter()
    }
    /// Local definition indices, including a consumed forced break if present.
    pub fn consumed_range(&self) -> std::ops::Range<usize> {
        self.cursor.next_item..self.consumed_end
    }
    pub const fn used_height(&self) -> Length {
        self.used_height
    }
    pub const fn available_height(&self) -> Length {
        self.available_height
    }
    pub const fn reason(&self) -> ProductionBodyBreakReason {
        self.reason
    }
    pub const fn forced_break_owner(&self) -> Option<NodeId> {
        self.forced_break_owner
    }
    /// Candidate end_item values are local indices in this definition's items.
    pub fn candidates(&self) -> &[ProductionBodyBreakCandidate] {
        self.choice
            .as_ref()
            .map_or(&[], |c| c.candidates.as_slice())
    }
    pub fn selected_candidate_index(&self) -> Option<u32> {
        self.choice.as_ref().map(|c| c.selected_candidate)
    }
    pub fn continuation(&self) -> Option<ProductionFootnoteCursor<'b, 'f, 's, 'p, 'a>> {
        (self.consumed_end
            < self
                .cursor
                .flow
                .definition_items(self.cursor.definition_index)?
                .len())
        .then_some(ProductionFootnoteCursor {
            next_item: self.consumed_end,
            ..self.cursor
        })
    }
    pub fn verify(
        &self,
        flow: &ProductionPreparedBodyFlow<'_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if !std::ptr::eq(self.cursor.flow, flow) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}

/// One owner for all candidate attempts. Dropping a selection or evaluating the
/// same cursor again does not refund candidate records or visited-item work.
pub struct ProductionFootnoteBreakSearch<'b, 'f, 's, 'p, 'a> {
    flow: &'b ProductionPreparedBodyFlow<'f, 's, 'p, 'a>,
    paragraph_lengths: Vec<usize>,
    headings: BTreeSet<NodeId>,
    charge: Charge,
    maximum_records: u64,
    maximum_candidates: u16,
    maximum_height: Length,
    maximum_steps: u64,
    steps: u64,
}
fn visit(
    steps: &mut u64,
    maximum: u64,
    owner: NodeId,
) -> Result<(), ProductionBodyPaginationError> {
    if *steps >= maximum {
        return Err(error(owner, E::FootnoteSearchLimit));
    }
    *steps += 1;
    Ok(())
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteBreakSearch<'b, 'f, 's, 'p, 'a> {
    pub fn record_charge(&self) -> u64 {
        self.maximum_records - self.charge.remaining
    }
    pub const fn visited_items(&self) -> u64 {
        self.steps
    }
    pub fn begin(
        &mut self,
        definition_index: usize,
    ) -> Result<ProductionFootnoteCursor<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let definition = self
            .flow
            .footnotes
            .definitions()
            .get(definition_index)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        if self
            .flow
            .definition_items(definition_index)
            .map_or(true, |i| i.is_empty())
        {
            return Err(error(definition.owner(), E::EmptyParagraph));
        }
        self.charge.take(1, definition.owner())?;
        Ok(ProductionFootnoteCursor {
            flow: self.flow,
            definition_index,
            next_item: 0,
        })
    }
    /// None means no legal content fragment fits this smaller capacity. At the
    /// declared maximum, the same condition is a hard Oversize error.
    pub fn evaluate(
        &mut self,
        cursor: &ProductionFootnoteCursor<'_, '_, '_, '_, '_>,
        available_height: Length,
    ) -> Result<
        Option<ProductionFootnoteFragmentSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        if !std::ptr::eq(cursor.flow, self.flow) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        let cursor = ProductionFootnoteCursor {
            flow: self.flow,
            definition_index: cursor.definition_index,
            next_item: cursor.next_item,
        };
        let items = self
            .flow
            .definition_items(cursor.definition_index)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let first = items
            .get(cursor.next_item)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        if available_height < Length::ZERO || available_height > self.maximum_height {
            return Err(error(first.owner, E::InvalidFootnoteCapacity));
        }
        self.charge.take(1, first.owner)?;
        if first.source.is_none() {
            visit(&mut self.steps, self.maximum_steps, first.owner)?;
            return Ok(Some(ProductionFootnoteFragmentSelection {
                cursor,
                content_end: cursor.next_item,
                consumed_end: cursor.next_item + 1,
                used_height: Length::ZERO,
                available_height,
                reason: ProductionBodyBreakReason::Forced,
                forced_break_owner: Some(first.owner),
                choice: None,
            }));
        }
        let choice = match page_breaks::select_boundary(
            items,
            cursor.next_item,
            available_height,
            &self.paragraph_lengths,
            &self.headings,
            self.maximum_candidates,
            &mut self.charge,
            |owner| visit(&mut self.steps, self.maximum_steps, owner),
        ) {
            Err(e) if e.kind == E::Oversize && available_height < self.maximum_height => {
                return Ok(None)
            }
            result => result?,
        };
        let selected = choice.candidates[choice.selected_candidate as usize];
        let content_end = selected.end_item() as usize;
        let forced_break_owner = if choice.reason == ProductionBodyBreakReason::Forced {
            let forced = items
                .get(content_end)
                .filter(|i| i.source.is_none())
                .ok_or_else(|| error(first.owner, E::ReceiptMismatch))?;
            visit(&mut self.steps, self.maximum_steps, forced.owner)?;
            Some(forced.owner)
        } else {
            None
        };
        Ok(Some(ProductionFootnoteFragmentSelection {
            cursor,
            content_end,
            consumed_end: content_end + usize::from(forced_break_owner.is_some()),
            used_height: selected.used_height(),
            available_height,
            reason: choice.reason,
            forced_break_owner,
            choice: Some(choice),
        }))
    }
}

pub fn prepare_production_footnote_search<'b, 'f, 's, 'p, 'a>(
    flow: &'b ProductionPreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_visited_items: u64,
) -> Result<ProductionFootnoteBreakSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    flow.verify(flow.lines, flow.blocks, limits)?;
    let maximum_records = limits.base().get().max_fragments;
    let mut charge = Charge {
        remaining: maximum_records
            .checked_sub(flow.record_charge())
            .ok_or_else(|| error(NodeId::new(0), E::FragmentLimit))?,
    };
    charge.take(1, NodeId::new(0))?;
    let mut steps = 0;
    // Keep cannot silently cross an authored forced page boundary. Definition
    // ends themselves are independent boundaries, never a keep to another note.
    for index in 0..flow.footnotes.definitions().len() {
        for pair in flow
            .definition_items(index)
            .expect("bound definition")
            .windows(2)
        {
            visit(&mut steps, maximum_visited_items, pair[0].owner)?;
            if pair[0].keep && pair[1].source.is_none() {
                return Err(error(pair[0].owner, E::KeepAcrossForcedBreak));
            }
        }
    }
    let (paragraph_lengths, headings) = page_breaks::prepare_context(flow.lines, &mut charge)?;
    Ok(ProductionFootnoteBreakSearch {
        flow,
        paragraph_lengths,
        headings,
        charge,
        maximum_records,
        maximum_candidates: limits.base().get().max_page_break_lookback,
        maximum_height: flow
            .footnote_region()
            .map_or(Length::ZERO, |r| r.height().get()),
        maximum_steps: maximum_visited_items,
        steps,
    })
}
