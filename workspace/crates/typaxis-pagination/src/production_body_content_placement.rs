//! Shared actual-content geometry. Callers retain their source-specific page owners.
use super::*;
pub(in crate::production_body::body_flow) struct ContentPlacement<'c, 's, 'p, 'a> {
    pub lines: BodyLines<'s, 'p, 'a>,
    pub blocks: BodyBlocks<'s>,
    pub collected: &'s CollectedItems,
    pub definition_markers: &'s [ProductionFootnoteMarkerBinding],
    pub body: Rect,
    pub charge: &'c mut Charge,
    pub steps: &'c mut u64,
    pub maximum_steps: u64,
}
impl ContentPlacement<'_, '_, '_, '_> {
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        visit(self.steps, self.maximum_steps, owner)
    }
    pub(in crate::production_body::body_flow) fn place_separator(
        &mut self,
        bounds: Option<Rect>,
    ) -> Result<Option<Rect>, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let separator_ink = if let Some(bounds) = bounds {
            self.step(root)?;
            self.charge.take(1, root)?;
            let height = Length::from_raw(typaxis_layout::FOOTNOTE_SEPARATOR_STROKE_RAW)
                .and_then(PositiveLength::new)
                .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
            if height.get().raw() > typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW
                || height.get() > bounds.height().get()
            {
                return Err(error(root, E::ReceiptMismatch));
            }
            Some(Rect::new(bounds.x(), bounds.y(), bounds.width(), height))
        } else {
            None
        };
        Ok(separator_ink)
    }

    pub(in crate::production_body::body_flow) fn place_content_range(
        &mut self,
        result: &mut Vec<ProductionBodyFootnotePlacedFragment>,
        page: u32,
        definition: Option<usize>,
        range: std::ops::Range<usize>,
        origin: Length,
        expected: Length,
    ) -> Result<(), ProductionBodyPaginationError> {
        let items = match definition {
            Some(index) => {
                &self.collected.items[self
                    .collected
                    .definitions
                    .get(index)
                    .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?
                    .clone()]
            }
            None => &self.collected.items[..self.collected.body_end],
        };
        let mut used = Length::ZERO;
        for index in range.clone() {
            let item = &items[index];
            self.step(item.owner)?;
            let before = if index == range.start {
                Length::ZERO
            } else {
                add(items[index - 1].after, item.before, item.owner)?
            };
            let top = add(
                add(add(origin, used, item.owner)?, before, item.owner)?,
                item.leading,
                item.owner,
            )?;
            let placed = place_flow_item_shared(self.lines, self.blocks, item, page, top, before)?;
            self.charge.take(1, item.owner)?;
            result
                .try_reserve(1)
                .map_err(|_| error(item.owner, E::AllocationFailure))?;
            result.push(ProductionBodyFootnotePlacedFragment {
                definition,
                item_index: index,
                fragment: placed,
            });
            used = add(add(used, before, item.owner)?, item.consumed()?, item.owner)?;
        }
        if used != expected {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
    pub(in crate::production_body::body_flow) fn place_page_markers(
        &mut self,
        fragments: &[ProductionBodyFootnotePlacedFragment],
    ) -> Result<
        (
            Vec<ProductionBodyListMarker>,
            Vec<ProductionBodyFootnotePlacedMarker>,
        ),
        ProductionBodyPaginationError,
    > {
        self.place_page_markers_with_repetition(fragments, std::iter::repeat(false))
    }
    pub(in crate::production_body::body_flow) fn place_page_markers_with_repetition(
        &mut self,
        fragments: &[ProductionBodyFootnotePlacedFragment],
        mut repetitions: impl Iterator<Item = bool>,
    ) -> Result<
        (
            Vec<ProductionBodyListMarker>,
            Vec<ProductionBodyFootnotePlacedMarker>,
        ),
        ProductionBodyPaginationError,
    > {
        let mut lists = Vec::new();
        let mut notes = Vec::new();
        for (fragment_index, placed) in fragments.iter().enumerate() {
            let repeated = repetitions
                .next()
                .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
            self.place_fragment_markers(fragment_index, placed, repeated, &mut lists, &mut notes)?;
        }
        Ok((lists, notes))
    }
    pub(in crate::production_body::body_flow) fn place_fragment_markers(
        &mut self,
        fragment_index: usize,
        placed: &ProductionBodyFootnotePlacedFragment,
        repeated: bool,
        lists: &mut Vec<ProductionBodyListMarker>,
        notes: &mut Vec<ProductionBodyFootnotePlacedMarker>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let bindings = &self.collected.marker_bindings;
        let fragment = placed.fragment;
        let owner = fragment.owner();
        self.step(owner)?;
        let global = placed.item_index
            + placed
                .definition
                .map_or(0, |d| self.collected.definitions[d].start);
        // Bindings follow the original collected stream, even when demand
        // order places definitions in another order. Charge binary search.
        for _ in 0..(2 * (usize::BITS - bindings.len().max(1).leading_zeros())) {
            self.step(owner)?;
        }
        let first = bindings.partition_point(|b| b.item_index.is_some_and(|i| i < global));
        let fragment_index =
            u32::try_from(fragment_index).map_err(|_| error(owner, E::FragmentLimit))?;
        for binding in bindings[first..]
            .iter()
            .take_while(|b| b.item_index == Some(global))
        {
            self.step(owner)?;
            self.charge.take(1, owner)?;
            lists
                .try_reserve(1)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            lists.push(list::place_marker_shared(
                self.lines,
                binding,
                fragment_index,
                fragment.page_index(),
                fragment.bounds().y(),
                self.body,
            )?);
        }
        if repeated {
            return Ok(());
        }
        let Some(definition) = placed.definition else {
            return Ok(());
        };
        let binding = self
            .definition_markers
            .get(definition)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        if binding.item_index() != placed.item_index {
            return Ok(());
        }
        let shape = self
            .lines
            .marker_geometry(definition, true)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let frame = self
            .lines
            .frames()
            .and_then(|f| f.footnotes().get(definition))
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let slack = frame
            .marker_width()
            .get()
            .checked_sub(shape.advance.get())
            .filter(|n| *n >= Length::ZERO)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let x = add(
            add(self.body.x(), frame.marker_start(), owner)?,
            slack,
            owner,
        )?;
        let baseline = add(fragment.bounds().y(), binding.baseline(), owner)?;
        let y = baseline
            .checked_sub(shape.ascent)
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        let height = shape
            .ascent
            .checked_sub(shape.descent)
            .and_then(PositiveLength::new)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        self.charge.take(1, owner)?;
        notes
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        notes.push(ProductionBodyFootnotePlacedMarker {
            definition,
            fragment_index,
            bounds: Rect::new(x, y, shape.advance, height),
            baseline,
        });
        Ok(())
    }
}
