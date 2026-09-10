//! Exact selected-source coverage of stable physical pages. This is not a PDF
//! authorization or a decision to admit unreferenced footnote definitions.
use super::*;
#[path = "book_v2_header_source_closure.rs"]
mod headers;

pub struct BookV2BodySourceClosure<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    geometry: &'g BookV2BodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a>,
    stable: &'q BookV2BodyMixedStablePages<'b, 'f, 's, 'p, 'a>,
    flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
    semantic_fragments: usize,
    repeated_fragments: usize,
    header_variant_fragments: usize,
    unreferenced_definitions: usize,
    semantic_math: usize,
    repeated_math: usize,
    records: u64,
    work: u64,
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2BodySourceClosure<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn geometry(&self) -> &'g BookV2BodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a> {
        self.geometry
    }
    pub fn stable(&self) -> &'q BookV2BodyMixedStablePages<'b, 'f, 's, 'p, 'a> {
        self.stable
    }
    pub fn flow(&self) -> &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a> {
        self.flow
    }
    pub fn semantic_fragments(&self) -> usize {
        self.semantic_fragments
    }
    pub fn repeated_fragments(&self) -> usize {
        self.repeated_fragments
    }
    /// Resolve source indexes on a verified placed fragment through its exact
    /// geometry owner. Page and fragment arguments are offsets in this geometry.
    pub fn fragment_flow(
        &self,
        page: usize,
        fragment: usize,
    ) -> Option<&'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>> {
        let page = self.geometry.pages().get(page)?;
        page.fragments().get(fragment)?;
        Some(page.header_variant(fragment).map_or(self.flow, |v| v.measurements().flow()))
    }
    pub fn header_variant_fragments(&self) -> usize {
        self.header_variant_fragments
    }
    pub fn has_header_variants(&self) -> bool {
        self.header_variant_fragments != 0
    }
    pub(super) fn require_single_measurement(
        &self,
        region: &'static str,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.has_header_variants() {
            return Err(error(NodeId::new(0), E::PendingRegion(region)));
        }
        Ok(())
    }
    pub fn unreferenced_definitions(&self) -> usize {
        self.unreferenced_definitions
    }
    pub fn semantic_math(&self) -> usize {
        self.semantic_math
    }
    pub fn repeated_math(&self) -> usize {
        self.repeated_math
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
}

#[derive(Clone, Copy, Default)]
struct SourceVisit {
    semantic: bool,
    repeated: bool,
}
impl SourceVisit {
    fn consume(
        &mut self,
        repeated: bool,
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        if repeated {
            self.repeated = true;
        } else if self.semantic {
            return Err(error(owner, E::ReceiptMismatch));
        } else {
            self.semantic = true;
        }
        Ok(())
    }
    fn finish(self, expected: bool, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        if self.semantic != expected || (self.repeated && !expected) {
            return Err(error(owner, E::ReceiptMismatch));
        }
        Ok(())
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Bind an exact stable sequence and its placed geometry to every actual
    /// paintable source leaf. Forced breaks are source events, not paint leaves.
    /// Unreferenced definitions remain explicit and are not certified for PDF.
    pub fn close_mixed_page_sources<'g, 'q>(
        &mut self,
        stable: &'q BookV2BodyMixedStablePages<'b, 'f, 's, 'p, 'a>,
        geometry: &'g BookV2BodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2BodySourceClosure<'g, 'q, 'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.verify_mixed_sequence(stable.sequence())?;
        self.verify_mixed_sequence(geometry.sequence())?;
        let root = NodeId::new(0);
        if !std::ptr::eq(stable.sequence(), geometry.sequence()) {
            return Err(error(root, E::ReceiptMismatch));
        }
        let header_sources = if self.headers.is_some() {
            Some(headers::OriginalHeaderSources::prepare(self)?)
        } else {
            None
        };
        let flow = self.content.flow;
        let demand = stable
            .sequence()
            .pages()
            .last()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?
            .next_state()
            .source_state()
            .demand();
        self.verify_state(demand)?;
        if !demand.pending_definitions().is_empty() {
            return Err(error(root, E::ReceiptMismatch));
        }
        self.content.charge.take(
            flow.collected
                .items
                .len()
                .checked_add(1)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut visits = Vec::new();
        visits
            .try_reserve_exact(flow.collected.items.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        visits.resize(flow.collected.items.len(), SourceVisit::default());
        let mut semantic_fragments = 0usize;
        let mut repeated_fragments = 0usize;
        let mut header_variant_fragments = 0usize;
        let mut semantic_math = 0usize;
        let mut repeated_math = 0usize;
        for page in geometry.pages() {
            self.content.step(root)?;
            if let Some(sources) = &header_sources {
                header_variant_fragments = header_variant_fragments
                    .checked_add(self.close_header_page_copies(page, sources, &mut visits)?)
                    .ok_or_else(|| error(root, E::FragmentLimit))?;
            } else if !page.header_variants().is_empty() {
                return Err(error(root, E::ReceiptMismatch));
            }
            if let Some(owner) = page.selection().forced_break() {
                self.content.step(owner)?;
                let candidate = page.selection().candidate();
                if let Some(table) = candidate
                    .parts()
                    .last()
                    .and_then(|part| part.table())
                    .filter(|t| t.forced_break_owner().is_some())
                {
                    let mut first = None;
                    for index in table.forced_break_items() {
                        self.content.step(owner)?;
                        let item = flow
                            .body_items()
                            .get(index)
                            .filter(|item| item.source().is_none())
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        first.get_or_insert(item.owner());
                        visits[index].consume(false, item.owner())?;
                    }
                    if first != Some(owner) {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                } else {
                    let index = candidate.next_state().next_item();
                    let item = flow
                        .body_items()
                        .get(index)
                        .filter(|item| item.owner() == owner && item.source().is_none())
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    visits[index].consume(false, item.owner())?;
                }
            }
            if let Some(region) = page.selection().candidate().footnotes() {
                for placed in region.fragments() {
                    self.content.step(root)?;
                    let fragment = placed.fragment();
                    let definition = fragment.definition_index();
                    let base = flow.collected.definitions[definition].start;
                    let mut consume = |range: std::ops::Range<usize>| -> Result<(), ProductionBodyPaginationError> {
                        for local in range {
                            let index = base.checked_add(local).ok_or_else(|| error(root, E::ArithmeticOverflow))?;
                            let item = flow.collected.items.get(index)
                                .filter(|_| local < flow.collected.definitions[definition].len())
                                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                            self.content.step(item.owner())?;
                            if item.source().is_none() { visits[index].consume(false, item.owner())?; }
                        }
                        Ok(())
                    };
                    if let Some(mixed) = fragment.mixed() {
                        for part in mixed.parts() {
                            if let Some(range) = part.items() {
                                consume(range)?;
                            }
                            if let Some(table) = part.table() {
                                for range in table.source_leaf_ranges() {
                                    consume(range?)?;
                                }
                            }
                        }
                    } else {
                        consume(fragment.consumed_range()?)?;
                    }
                }
            }
            if page.fragments().len() != page.cell_roles().len() {
                return Err(error(root, E::ReceiptMismatch));
            }
            for (fragment_index, (placed, _, repeated)) in page.fragments_with_roles().enumerate() {
                let fragment = placed.fragment();
                let owner = fragment.owner();
                self.content.step(owner)?;
                let variant = if page.header_variants().is_empty() {
                    None
                } else {
                    self.source_lookup_work(page.header_variants().len(), owner)?;
                    page.header_variant(fragment_index)
                };
                let paint_flow = variant.map_or(flow, |v| v.measurements().flow());
                let range = match placed.definition_index() {
                    None => 0..paint_flow.collected.body_end,
                    Some(index) => paint_flow
                        .collected
                        .definitions
                        .get(index)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                        .clone(),
                };
                if placed.item_index() >= range.len() {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                let index = range.start + placed.item_index();
                let item = &paint_flow.collected.items[index];
                if item.owner() != owner
                    || item.source() != Some(fragment.source())
                    || fragment.page_index() != page.selection().page_index()
                {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                if variant.is_none() {
                    visits[index].consume(repeated, owner)?;
                } else if !repeated {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                let horizontal = if variant.is_some() {
                    super::header_placement::origin_delta(
                        paint_flow, page.selection(), placed.definition_index().is_some(),
                    )?
                } else {
                    self.page_origin_delta(page.selection(), placed.definition_index().is_some())?
                };
                if variant.is_some()
                    && (fragment.bounds().x() != add(item.x(), horizontal, owner)?
                        || fragment.bounds().width() != item.width()
                        || fragment.bounds().height().get() != item.height())
                {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                let math = self.verify_source_math(paint_flow, fragment, horizontal)?;
                let math_count = if repeated {
                    &mut repeated_math
                } else {
                    &mut semantic_math
                };
                *math_count = math_count
                    .checked_add(math)
                    .ok_or_else(|| error(owner, E::FragmentLimit))?;
                let count = if repeated {
                    &mut repeated_fragments
                } else {
                    &mut semantic_fragments
                };
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| error(owner, E::FragmentLimit))?;
            }
        }
        for (index, item) in flow.body_items().iter().enumerate() {
            self.content.step(item.owner())?;
            // Nonpainting body breaks still require exactly one source visit.
            visits[index].finish(true, item.owner())?;
        }
        let mut unreferenced_definitions = 0usize;
        for (index, range) in flow.collected.definitions.iter().enumerate() {
            self.content.step(root)?;
            let referenced = match demand.status(index) {
                Some(ProductionFootnoteDemandStatus::Complete) => true,
                Some(ProductionFootnoteDemandStatus::Unreferenced) => {
                    unreferenced_definitions += 1;
                    false
                }
                _ => return Err(error(root, E::ReceiptMismatch)),
            };
            for index in range.clone() {
                let item = &flow.collected.items[index];
                self.content.step(item.owner())?;
                visits[index].finish(referenced, item.owner())?;
            }
        }
        Ok(BookV2BodySourceClosure {
            geometry,
            stable,
            flow,
            semantic_fragments,
            repeated_fragments,
            header_variant_fragments,
            unreferenced_definitions,
            semantic_math,
            repeated_math,
            records: self.record_charge(),
            work: self.work_steps(),
        })
    }

    pub(super) fn source_lookup_work(
        &mut self,
        count: usize,
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        for _ in 0..(usize::BITS - count.max(1).leading_zeros() + 1) {
            self.content.step(owner)?;
        }
        Ok(())
    }

    /// Verify actual selected inline receipts, not just a count of math nodes.
    /// Equation labels remain separate placed objects on the borrowed geometry.
    fn verify_source_math(
        &mut self,
        flow: &'b BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
        fragment: ProductionBodyFragment,
        horizontal: Length,
    ) -> Result<usize, ProductionBodyPaginationError> {
        use typaxis_layout::{PrecomposedVectorPlacementInput, ProductionPlacedInline};
        use typaxis_syntax::PrecomposedVectorKind;
        let lines = flow.lines();
        let prepared = lines.prepared();
        let owner = fragment.owner();
        let mismatch = || error(owner, E::ReceiptMismatch);
        match fragment.source() {
            ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } => {
                let paragraph = lines
                    .paragraphs()
                    .get(paragraph_index as usize)
                    .filter(|p| p.owner() == owner)
                    .ok_or_else(mismatch)?;
                let line = paragraph
                    .lines()
                    .get(line_index as usize)
                    .ok_or_else(mismatch)?;
                if fragment.baseline() != Some(add(fragment.bounds().y(), line.baseline(), owner)?)
                {
                    return Err(mismatch());
                }
                let mut math = 0usize;
                for inline in line.items() {
                    self.content.step(owner)?;
                    let is_math = match inline {
                        ProductionPlacedInline::Vector(selected) => {
                            let occurrence = selected.occurrence();
                            let item = occurrence.item();
                            let bindings = prepared.vector_bindings().ok_or_else(mismatch)?;
                            self.source_lookup_work(bindings.receipts().len(), item.node_id())?;
                            let binding = bindings.receipt(item.node_id()).ok_or_else(mismatch)?;
                            let PrecomposedVectorPlacementInput::Inline(placement) =
                                binding.placement()
                            else {
                                return Err(mismatch());
                            };
                            if item.paragraph_node() != owner
                                || item.source_span() != binding.owner_source_span()
                                || item.binding_fingerprint() != binding.binding_fingerprint()
                                || item.placement() != *placement
                                || selected.geometry()
                                    != item
                                        .metrics()
                                        .select_inline_geometry(occurrence.pen_x(), line.baseline())
                                        .map_err(|_| mismatch())?
                            {
                                return Err(mismatch());
                            }
                            binding.kind() == PrecomposedVectorKind::MathVector
                        }
                        ProductionPlacedInline::BookV2Math(selected) => {
                            let native = prepared.native_math().ok_or_else(mismatch)?;
                            self.source_lookup_work(native.receipts().len(), selected.owner())?;
                            let receipt = native.receipt(selected.owner()).ok_or_else(mismatch)?;
                            if !std::ptr::eq(receipt, selected.receipt())
                                || selected.source_span() != receipt.source().domain().span
                                || selected.baseline() != line.baseline()
                            {
                                return Err(mismatch());
                            }
                            true
                        }
                        ProductionPlacedInline::Math(_) => return Err(mismatch()),
                        ProductionPlacedInline::Text(_) | ProductionPlacedInline::Break(_) => false,
                    };
                    math = math
                        .checked_add(usize::from(is_math))
                        .ok_or_else(|| error(owner, E::FragmentLimit))?;
                }
                Ok(math)
            }
            ProductionBodyFragmentSource::VectorBlock { block_index } => {
                let block = flow
                    .blocks()
                    .and_then(|b| b.blocks().get(block_index as usize))
                    .filter(|b| b.owner() == owner)
                    .ok_or_else(mismatch)?;
                let bindings = prepared.vector_bindings().ok_or_else(mismatch)?;
                self.source_lookup_work(bindings.receipts().len(), owner)?;
                let binding = bindings.receipt(owner).ok_or_else(mismatch)?;
                let viewport = fragment.viewport().ok_or_else(mismatch)?;
                if !std::ptr::eq(binding, block.binding())
                    || viewport.x() != add(block.viewport_left(), horizontal, owner)?
                    || viewport.y()
                        != add(
                            fragment.bounds().y(),
                            block.viewport_top_offset().get(),
                            owner,
                        )?
                    || viewport.width() != block.viewport_width()
                    || viewport.height() != block.viewport_height()
                    || fragment.baseline()
                        != block
                            .baseline()
                            .map(|b| add(viewport.y(), b.get(), owner))
                            .transpose()?
                {
                    return Err(mismatch());
                }
                Ok(usize::from(
                    binding.kind() == PrecomposedVectorKind::MathVectorBlock,
                ))
            }
            ProductionBodyFragmentSource::NativeMathBlock { block_index } => {
                let native = prepared.native_math().ok_or_else(mismatch)?;
                let block = native
                    .display_blocks()
                    .get(block_index as usize)
                    .filter(|b| b.owner() == owner)
                    .ok_or_else(mismatch)?;
                self.source_lookup_work(native.receipts().len(), owner)?;
                let receipt = native.receipt(owner).ok_or_else(mismatch)?;
                let viewport = fragment.viewport().ok_or_else(mismatch)?;
                if block.source_span() != receipt.source().domain().span
                    || viewport.width() != block.width()
                    || viewport.height() != block.height()
                    || fragment.baseline() != Some(add(viewport.y(), block.baseline(), owner)?)
                {
                    return Err(mismatch());
                }
                Ok(1)
            }
            ProductionBodyFragmentSource::Figure { .. } => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_visits_reject_duplicates_missing_originals_and_unselected_copies() {
        let owner = NodeId::new(7);
        let mut visit = SourceVisit::default();
        assert!(visit.finish(true, owner).is_err());
        visit.consume(true, owner).unwrap();
        assert!(visit.finish(true, owner).is_err());
        assert!(visit.finish(false, owner).is_err());
        visit.consume(false, owner).unwrap();
        visit.consume(true, owner).unwrap();
        visit.finish(true, owner).unwrap();
        assert!(visit.consume(false, owner).is_err());
        assert!(visit.finish(false, owner).is_err());
        SourceVisit::default().finish(false, owner).unwrap();
    }
}
