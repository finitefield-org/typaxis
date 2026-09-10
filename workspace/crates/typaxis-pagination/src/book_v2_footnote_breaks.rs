//! Dedicated successor definition fragments over the common break kernel.
use super::*;
use crate::production_body::body_flow::book_v2::{
    BookV2DefinitionMixedCandidate, BookV2PreparedBodyFlow, BookV2TableCursor,
};

#[derive(Clone, Copy)]
pub struct BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a> {
    flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    definition_index: usize,
    next_item: usize,
    mixed: Option<MixedProgress<'b, 'f, 's, 'p, 'a>>,
}
#[derive(Clone, Copy)]
struct MixedProgress<'b, 'f, 's, 'p, 'a> {
    next_table: usize,
    continuation: Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>>,
    marker_consumed: bool,
}
impl BookV2FootnoteCursor<'_, '_, '_, '_, '_> {
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
pub struct BookV2FootnoteFragmentSelection<'b, 'f, 's, 'p, 'a> {
    cursor: BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a>,
    content_end: usize,
    consumed_end: usize,
    used_height: Length,
    available_height: Length,
    reason: ProductionBodyBreakReason,
    forced_break_owner: Option<NodeId>,
    choice: Option<page_breaks::BoundarySelection>,
    mixed: Option<BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>>,
    mixed_references: Vec<&'b ProductionFootnoteFlowReference<'f>>,
    mixed_after: Option<Length>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteFragmentSelection<'b, 'f, 's, 'p, 'a> {
    pub(super) fn from_projection(
        cursor: BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a>,
        result: FootnoteFragmentProjection,
    ) -> Self {
        Self {
            cursor,
            content_end: result.content_end,
            consumed_end: result.consumed_end,
            used_height: result.used_height,
            available_height: result.available_height,
            reason: result.reason,
            forced_break_owner: result.forced_break_owner,
            choice: result.choice,
            mixed: None,
            mixed_references: Vec::new(),
            mixed_after: None,
        }
    }
    pub const fn definition_index(&self) -> usize {
        self.cursor.definition_index
    }
    /// A contiguous view exists only for ordinary selections. Mixed selections
    /// must be inspected through their original serial/table parts.
    pub fn items(&self) -> Result<&[ProductionBodyFlowItem], ProductionBodyPaginationError> {
        if self.mixed.is_some() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(&self
            .cursor
            .flow
            .definition_items(self.cursor.definition_index)
            .expect("bound definition")[self.cursor.next_item..self.content_end])
    }
    /// The label belongs only to the selection containing its first real item.
    /// Continuation selections do not repeat it; leading forced breaks do not
    /// consume it before any paintable content is selected.
    pub fn marker(&self) -> Option<&ProductionFootnoteMarkerBinding> {
        if let Some(mixed) = &self.mixed {
            return self
                .cursor
                .flow
                .definition_marker(self.definition_index())
                .filter(|_| {
                    !self.cursor.definition_started()
                        && mixed
                            .next_state()
                            .demand()
                            .definition_started(self.definition_index())
                });
        }
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
        let ordinary = if self.mixed.is_some() {
            &[][..]
        } else {
            self.cursor.flow.references_in_items(
                Some(self.cursor.definition_index),
                self.cursor.next_item..self.content_end,
            )
        };
        ordinary.iter().chain(self.mixed_references.iter().copied())
    }
    /// Local contiguous source indices exist only for ordinary selections.
    pub fn consumed_range(&self) -> Result<std::ops::Range<usize>, ProductionBodyPaginationError> {
        if self.mixed.is_some() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(self.cursor.next_item..self.consumed_end)
    }
    pub fn mixed(&self) -> Option<&BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>> {
        self.mixed.as_ref()
    }
    pub fn space_after(&self) -> Option<Length> {
        if self.mixed.is_some() {
            self.mixed_after
        } else {
            self.cursor
                .flow
                .definition_items(self.definition_index())
                .and_then(|items| items.get(self.cursor.next_item..self.content_end))
                .and_then(|items| items.last())
                .map(|item| item.after)
        }
    }
    pub(in crate::production_body::body_flow) fn from_mixed(
        cursor: BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a>,
        candidate: BookV2DefinitionMixedCandidate<'b, 'f, 's, 'p, 'a>,
        available: Length,
        references: Vec<&'b ProductionFootnoteFlowReference<'f>>,
        after: Option<Length>,
    ) -> Self {
        Self {
            cursor,
            content_end: cursor.next_item,
            consumed_end: cursor.next_item,
            used_height: candidate.used_height(),
            available_height: available,
            reason: if candidate.forced_break_owner().is_some() {
                ProductionBodyBreakReason::Forced
            } else if candidate.next_state().is_complete() {
                ProductionBodyBreakReason::End
            } else {
                ProductionBodyBreakReason::Overflow
            },
            forced_break_owner: candidate.forced_break_owner(),
            choice: None,
            mixed: Some(candidate),
            mixed_references: references,
            mixed_after: after,
        }
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
    pub fn continuation(&self) -> Option<BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a>> {
        if let Some(mixed) = &self.mixed {
            return mixed
                .next_state()
                .demand()
                .definition_cursor(self.definition_index());
        }
        (self.consumed_end
            < self
                .cursor
                .flow
                .definition_items(self.cursor.definition_index)?
                .len())
        .then_some(BookV2FootnoteCursor {
            next_item: self.consumed_end,
            ..self.cursor
        })
    }
    pub fn verify(
        &self,
        flow: &BookV2PreparedBodyFlow<'_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if !std::ptr::eq(self.cursor.flow, flow) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}

/// One owner for all candidate attempts. Dropping a selection or evaluating the
/// same cursor again does not refund candidate records or visited-item work.
pub struct BookV2FootnoteBreakSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    pub(super) paragraph_lengths: Vec<usize>,
    pub(super) headings: BTreeSet<NodeId>,
    pub(super) charge: Charge,
    maximum_records: u64,
    pub(super) maximum_candidates: u16,
    pub(super) maximum_height: Length,
    pub(super) maximum_steps: u64,
    pub(super) steps: u64,
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteBreakSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) fn kernel(&mut self) -> FootnoteSearchKernel<'_> {
        FootnoteSearchKernel {
            paragraph_lengths: &self.paragraph_lengths,
            headings: &self.headings,
            charge: &mut self.charge,
            steps: &mut self.steps,
            maximum_candidates: self.maximum_candidates,
            maximum_height: self.maximum_height,
            maximum_steps: self.maximum_steps,
        }
    }
    pub fn record_charge(&self) -> u64 {
        self.maximum_records - self.charge.remaining
    }
    pub const fn visited_items(&self) -> u64 {
        self.steps
    }
    pub fn begin(
        &mut self,
        definition_index: usize,
    ) -> Result<BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let definition = self
            .flow
            .footnotes()
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
        Ok(BookV2FootnoteCursor {
            flow: self.flow,
            definition_index,
            next_item: 0,
            mixed: None,
        })
    }
    /// None means no legal content fragment fits this smaller capacity. At the
    /// declared maximum, the same condition is a hard Oversize error.
    pub fn evaluate(
        &mut self,
        cursor: &BookV2FootnoteCursor<'_, '_, '_, '_, '_>,
        available_height: Length,
    ) -> Result<
        Option<BookV2FootnoteFragmentSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        if !std::ptr::eq(cursor.flow, self.flow) || cursor.mixed.is_some() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        let cursor = BookV2FootnoteCursor {
            flow: self.flow,
            definition_index: cursor.definition_index,
            next_item: cursor.next_item,
            mixed: None,
        };
        let items = self
            .flow
            .definition_items(cursor.definition_index)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let mut kernel = self.kernel();
        let Some(result) = kernel.evaluate(items, cursor.next_item, available_height)? else {
            return Ok(None);
        };
        Ok(Some(BookV2FootnoteFragmentSelection::from_projection(
            cursor, result,
        )))
    }
}

pub fn prepare_book_v2_footnote_search<'b, 'f, 's, 'p, 'a>(
    flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_visited_items: u64,
    prior_records: u64,
) -> Result<BookV2FootnoteBreakSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)?;
    let root = NodeId::new(0);
    let maximum_records = limits.base().get().max_fragments;
    let charge = Charge {
        remaining: maximum_records
            .checked_sub(prior_records.max(flow.record_charge()))
            .ok_or_else(|| error(root, E::FragmentLimit))?,
    };
    prepare_book_v2_footnote_search_charged(flow, limits, maximum_visited_items, charge, 0)
}
pub(super) fn prepare_book_v2_footnote_search_charged<'b, 'f, 's, 'p, 'a>(
    flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_visited_items: u64,
    charge: Charge,
    steps: u64,
) -> Result<BookV2FootnoteBreakSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    prepare_book_v2_footnote_context_charged(
        flow,
        limits,
        maximum_visited_items,
        charge,
        steps,
        false,
    )
}
pub(super) fn prepare_book_v2_footnote_context_charged<'b, 'f, 's, 'p, 'a>(
    flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum_visited_items: u64,
    mut charge: Charge,
    mut steps: u64,
    table_candidates: bool,
) -> Result<BookV2FootnoteBreakSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)?;
    let maximum_records = limits.base().get().max_fragments;
    // Definition tables require recursive table-aware content selection. A flat
    // definition cursor must never serialize their parallel cell streams.
    for table in &flow.collected.tables.tables {
        visit(&mut steps, maximum_visited_items, table.owner)?;
        if table.definition.is_some() && !table_candidates {
            return Err(error(
                table.owner,
                E::PendingRegion("table_footnote_definition"),
            ));
        }
    }
    let (paragraph_lengths, headings) = prepare_search_context(
        &flow.collected,
        BodyLines::BookV2(flow.lines()),
        table_candidates,
        maximum_visited_items,
        &mut charge,
        &mut steps,
    )?;
    Ok(BookV2FootnoteBreakSearch {
        flow,
        paragraph_lengths,
        headings,
        charge,
        maximum_records,
        maximum_candidates: limits.base().get().max_page_break_lookback,
        maximum_height: flow
            .lines()
            .frames()
            .and_then(|f| f.footnote_region())
            .map_or(Length::ZERO, |r| r.height().get()),
        maximum_steps: maximum_visited_items,
        steps,
    })
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a> {
    pub fn next_table_index(&self) -> Option<usize> {
        self.mixed.map(|m| m.next_table)
    }
    pub fn table_continuation(&self) -> Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>> {
        self.mixed.and_then(|m| m.continuation)
    }
    /// Whether the original definition marker's item has been consumed. This is
    /// source progress, not proof that a label was placed on a physical page.
    pub fn definition_started(&self) -> bool {
        self.mixed.map_or_else(
            || {
                self.flow
                    .definition_marker(self.definition_index)
                    .is_some_and(|marker| marker.item_index() < self.next_item)
            },
            |m| m.marker_consumed,
        )
    }
    pub(super) fn after_mixed(
        &self,
        next_item: usize,
        next_table: usize,
        continuation: Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>>,
        marker_consumed: bool,
    ) -> Result<Self, ProductionBodyPaginationError> {
        let owner = self.flow.footnotes().definitions()[self.definition_index].owner();
        if next_item < self.next_item
            || next_item
                > self
                    .flow
                    .definition_items(self.definition_index)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                    .len()
            || next_table > self.flow.table_count()
            || continuation.is_some_and(|cursor| {
                cursor.definition_index() != Some(self.definition_index)
                    || cursor.table_index() != next_table
            })
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        Ok(Self {
            next_item,
            mixed: Some(MixedProgress {
                next_table,
                continuation,
                marker_consumed: self.definition_started() || marker_consumed,
            }),
            ..*self
        })
    }
}
