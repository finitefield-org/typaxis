//! Source-bound physical content and marker geometry before final paint closure.
use super::*;

/// Definition-local indices remain distinct from body-local indices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyFootnotePlacedFragment {
    definition: Option<usize>,
    item_index: usize,
    fragment: ProductionBodyFragment,
}
impl ProductionBodyFootnotePlacedFragment {
    pub fn definition_index(&self) -> Option<usize> {
        self.definition
    }
    pub fn item_index(&self) -> usize {
        self.item_index
    }
    pub fn fragment(&self) -> ProductionBodyFragment {
        self.fragment
    }
}

/// Borrows the actual selected page. This is content geometry, not a public
/// display-list receipt: stable page closure is still required.
pub struct ProductionBodyFootnotePlacedPage<'q, 'b, 'f, 's, 'p, 'a> {
    selection: &'q ProductionBodyFootnotePageSelection<'b, 'f, 's, 'p, 'a>,
    fragments: Vec<ProductionBodyFootnotePlacedFragment>,
    list_markers: Vec<ProductionBodyListMarker>,
    footnote_markers: Vec<ProductionBodyFootnotePlacedMarker>,
}
impl<'q, 'b, 'f, 's, 'p, 'a> ProductionBodyFootnotePlacedPage<'q, 'b, 'f, 's, 'p, 'a> {
    pub fn selection(&self) -> &'q ProductionBodyFootnotePageSelection<'b, 'f, 's, 'p, 'a> {
        self.selection
    }
    pub fn list_markers(&self) -> &[ProductionBodyListMarker] {
        &self.list_markers
    }
    pub fn footnote_markers(&self) -> &[ProductionBodyFootnotePlacedMarker] {
        &self.footnote_markers
    }
    pub fn fragments(&self) -> &[ProductionBodyFootnotePlacedFragment] {
        &self.fragments
    }
}

impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn place_page_content<'q>(
        &mut self,
        selection: &'q ProductionBodyFootnotePageSelection<'b, 'f, 's, 'p, 'a>,
    ) -> Result<
        ProductionBodyFootnotePlacedPage<'q, 'b, 'f, 's, 'p, 'a>,
        ProductionBodyPaginationError,
    > {
        self.verify_state(selection.candidate().next_state())?;
        let root = NodeId::new(0);
        self.content.charge.take(1, root)?;
        let mut fragments = Vec::new();
        let candidate = selection.candidate();
        self.place_content_range(
            &mut fragments,
            selection.page_index(),
            None,
            candidate.body_range(),
            self.content.flow.blocks.page_geometry().body().y(),
            candidate.body_height(),
        )?;
        if let Some(region) = candidate.footnotes() {
            for selected in region.fragments() {
                self.step(root)?;
                let fragment = selected.fragment();
                if fragment.items().is_empty() {
                    continue;
                }
                let bounds = candidate
                    .footnote_bounds()
                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                let separator = Length::from_raw(typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW)
                    .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
                let origin = add(add(bounds.y(), separator, root)?, selected.offset(), root)?;
                let start = fragment.consumed_range().start;
                self.place_content_range(
                    &mut fragments,
                    selection.page_index(),
                    Some(fragment.definition_index()),
                    start..start + fragment.items().len(),
                    origin,
                    fragment.used_height(),
                )?;
            }
        }
        let (list_markers, footnote_markers) = self.place_page_markers(&fragments)?;
        Ok(ProductionBodyFootnotePlacedPage {
            selection,
            fragments,
            list_markers,
            footnote_markers,
        })
    }

    fn place_content_range(
        &mut self,
        result: &mut Vec<ProductionBodyFootnotePlacedFragment>,
        page: u32,
        definition: Option<usize>,
        range: std::ops::Range<usize>,
        origin: Length,
        expected: Length,
    ) -> Result<(), ProductionBodyPaginationError> {
        let flow = self.content.flow;
        let items = match definition {
            Some(index) => flow
                .definition_items(index)
                .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?,
            None => flow.body_items(),
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
            let placed = place_flow_item(flow.lines, flow.blocks, item, page, top, before)?;
            self.content.charge.take(1, item.owner)?;
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyFootnotePlacedMarker {
    definition: usize,
    fragment_index: u32,
    bounds: Rect,
    baseline: Length,
}
impl ProductionBodyFootnotePlacedMarker {
    pub fn definition_index(&self) -> usize {
        self.definition
    }
    pub fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn baseline(&self) -> Length {
        self.baseline
    }
}

impl ProductionFootnoteDemandSearch<'_, '_, '_, '_, '_> {
    fn place_page_markers(
        &mut self,
        fragments: &[ProductionBodyFootnotePlacedFragment],
    ) -> Result<
        (
            Vec<ProductionBodyListMarker>,
            Vec<ProductionBodyFootnotePlacedMarker>,
        ),
        ProductionBodyPaginationError,
    > {
        let flow = self.content.flow;
        let mut lists = Vec::new();
        let mut notes = Vec::new();
        let bindings = &flow.collected.marker_bindings;
        for (fragment_index, placed) in fragments.iter().enumerate() {
            let fragment = placed.fragment;
            let owner = fragment.owner();
            self.step(owner)?;
            let global = placed.item_index
                + placed
                    .definition
                    .map_or(0, |d| flow.collected.definitions[d].start);
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
                self.content.charge.take(1, owner)?;
                lists
                    .try_reserve(1)
                    .map_err(|_| error(owner, E::AllocationFailure))?;
                lists.push(list::place_marker(
                    flow.lines,
                    binding,
                    fragment_index,
                    fragment.page_index(),
                    fragment.bounds().y(),
                    flow.blocks.page_geometry().body(),
                )?);
            }
            let Some(definition) = placed.definition else {
                continue;
            };
            let binding = flow
                .definition_marker(definition)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            if binding.item_index() != placed.item_index {
                continue;
            }
            let shape = flow
                .lines
                .footnote_markers()
                .get(definition)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let frame = flow
                .lines
                .frames()
                .and_then(|f| f.footnotes().get(definition))
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let slack = frame
                .marker_width()
                .get()
                .checked_sub(shape.advance().get())
                .filter(|n| *n >= Length::ZERO)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let x = add(
                add(
                    flow.blocks.page_geometry().body().x(),
                    frame.marker_start(),
                    owner,
                )?,
                slack,
                owner,
            )?;
            let baseline = add(fragment.bounds().y(), binding.baseline(), owner)?;
            let y = baseline
                .checked_sub(shape.font().ascender())
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            let height = shape
                .font()
                .ascender()
                .checked_sub(shape.font().descender())
                .and_then(PositiveLength::new)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            self.content.charge.take(1, owner)?;
            notes
                .try_reserve(1)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            notes.push(ProductionBodyFootnotePlacedMarker {
                definition,
                fragment_index,
                bounds: Rect::new(x, y, shape.advance(), height),
                baseline,
            });
        }
        Ok((lists, notes))
    }
}
