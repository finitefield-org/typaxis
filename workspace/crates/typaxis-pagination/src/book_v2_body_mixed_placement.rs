//! Physical placement of selected mixed pages; repeated header copies stay explicit.
use super::*;

/// Number geometry is separate from formula replacement text. The flattened
/// fragment index addresses this sequence, including repeated header copies.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2BodyPlacedEquationNumber {
    geometry: ProductionBodyEquationNumber,
    repeated_header: bool,
}
impl BookV2BodyPlacedEquationNumber {
    pub fn geometry(&self) -> &ProductionBodyEquationNumber {
        &self.geometry
    }
    pub fn repeated_header(&self) -> bool {
        self.repeated_header
    }
}
pub struct BookV2BodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a> {
    selection: &'q BookV2BodyMixedPageSelection<'b, 'f, 's, 'p, 'a>,
    fragments: Vec<ProductionBodyFootnotePlacedFragment>,
    cells: Vec<Option<ProductionTablePlacedCellRole>>,
    variants: Vec<BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a>>,
    repeated_captions: Vec<usize>,
    lists: Vec<ProductionBodyListMarker>,
    notes: Vec<ProductionBodyFootnotePlacedMarker>,
    separator: Option<Rect>,
    numbers: Vec<BookV2BodyPlacedEquationNumber>,
}
impl<'q, 'b, 'f, 's, 'p, 'a> BookV2BodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a> {
    pub fn selection(&self) -> &'q BookV2BodyMixedPageSelection<'b, 'f, 's, 'p, 'a> {
        self.selection
    }
    /// Sparse exact owners for repeated-header fragment indexes. Other fragments
    /// retain the base flow. Consumers must resolve these before reading indexes.
    pub fn header_variants(&self) -> &[BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a>] {
        &self.variants
    }
    pub fn header_variant(
        &self,
        fragment: usize,
    ) -> Option<&BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a>> {
        self.variants
            .binary_search_by_key(&fragment, |v| v.fragment_index())
            .ok()
            .map(|i| &self.variants[i])
    }
    /// Geometry only: source indexes must use `header_variant` when present.
    pub fn fragments(&self) -> &[ProductionBodyFootnotePlacedFragment] {
        &self.fragments
    }
    pub fn cell_roles(&self) -> &[Option<ProductionTablePlacedCellRole>] {
        &self.cells
    }
    /// Original cell ownership and physical repetition are independent.
    pub fn fragments_with_roles(
        &self,
    ) -> impl Iterator<
        Item = (
            &ProductionBodyFootnotePlacedFragment,
            Option<ProductionTablePlacedCellRole>,
            bool,
        ),
    > + '_ {
        self.fragments
            .iter()
            .zip(roles_with_repetition(&self.cells, &self.repeated_captions))
            .map(|(fragment, (cell, repeated))| (fragment, cell, repeated))
    }
    pub(super) fn repeated_caption_positions(&self) -> &[usize] {
        &self.repeated_captions
    }
    pub fn list_markers(&self) -> &[ProductionBodyListMarker] {
        &self.lists
    }
    pub fn footnote_markers(&self) -> &[ProductionBodyFootnotePlacedMarker] {
        &self.notes
    }
    pub fn equation_numbers(&self) -> &[BookV2BodyPlacedEquationNumber] {
        &self.numbers
    }
    pub const fn separator_ink(&self) -> Option<Rect> {
        self.separator
    }
}
pub struct BookV2BodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a> {
    sequence: &'q BookV2BodyMixedPageSequence<'b, 'f, 's, 'p, 'a>,
    pages: Vec<BookV2BodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a>>,
}
impl<'q, 'b, 'f, 's, 'p, 'a> BookV2BodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a> {
    pub fn sequence(&self) -> &'q BookV2BodyMixedPageSequence<'b, 'f, 's, 'p, 'a> {
        self.sequence
    }
    pub fn pages(&self) -> &[BookV2BodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a>] {
        &self.pages
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    fn content_placement(&mut self) -> placement::ContentPlacement<'_, '_, 'p, 'a> {
        let flow = self.content.flow;
        placement::ContentPlacement {
            lines: BodyLines::BookV2(flow.lines()),
            blocks: BodyBlocks::BookV2(flow.blocks().map_or(&[], |b| b.blocks())),
            collected: &flow.collected,
            definition_markers: &flow.definition_markers,
            body: flow.lines().frames().expect("bound frames").body(),
            charge: &mut self.content.charge,
            steps: &mut self.content.steps,
            maximum_steps: self.content.maximum_steps,
        }
    }
    /// Assign actual page origins to the complete selected sequence. This does
    /// not establish stable reflow, PDF paint authorization or repeated-header
    /// Artifact structure; those must consume the explicit role on each copy.
    pub fn place_mixed_pages<'q>(
        &mut self,
        sequence: &'q BookV2BodyMixedPageSequence<'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2BodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.verify_mixed_sequence(sequence)?;
        let root = NodeId::new(0);
        let flow = self.content.flow;
        self.content.charge.take(
            sequence
                .pages()
                .len()
                .checked_add(1)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut pages = Vec::new();
        pages
            .try_reserve_exact(sequence.pages().len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut fragment_offset = 0usize;
        for selection in sequence.pages() {
            self.content.step(root)?;
            let candidate = selection.candidate();
            let page = selection.page_index();
            let mut fragments = Vec::new();
            let mut cells = Vec::new();
            let mut repeated_captions = Vec::new();
            let mut variants = Vec::new();
            for part in candidate.parts() {
                self.content.step(root)?;
                let origin = add(selection.body_bounds().y(), part.top(), root)?;
                if let Some(range) = part.items() {
                    self.content_placement().place_content_range(
                        &mut fragments,
                        page,
                        None,
                        range,
                        origin,
                        part.height(),
                    )?;
                    self.extend_mixed_roles(&mut cells, fragments.len())?;
                } else {
                    let table = part
                        .table()
                        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                    self.place_mixed_table_leaves(
                        table,
                        page,
                        None,
                        origin,
                        part.height(),
                        &mut fragments,
                        &mut cells,
                        &mut repeated_captions,
                        &mut variants,
                    )?;
                }
            }
            if let Some(region) = candidate.footnotes() {
                for selected in region.fragments() {
                    self.content.step(root)?;
                    let fragment = selected.fragment();
                    if fragment
                        .mixed()
                        .is_some_and(|mixed| mixed.used_height() == Length::ZERO)
                        || (fragment.mixed().is_none() && fragment.items()?.is_empty())
                    {
                        continue;
                    }
                    let bounds = candidate
                        .footnote_bounds()
                        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                    let separator = Length::from_raw(typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW)
                        .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
                    let origin = add(add(bounds.y(), separator, root)?, selected.offset(), root)?;
                    if let Some(mixed) = fragment.mixed() {
                        for part in mixed.parts() {
                            self.content.step(root)?;
                            let top = add(origin, part.top(), root)?;
                            if let Some(range) = part.items() {
                                let items = flow
                                    .definition_items(fragment.definition_index())
                                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                                if items
                                    .get(range.start)
                                    .is_some_and(|item| item.source.is_none())
                                {
                                    continue;
                                }
                                self.content_placement().place_content_range(
                                    &mut fragments,
                                    page,
                                    Some(fragment.definition_index()),
                                    range,
                                    top,
                                    part.height(),
                                )?;
                                self.extend_mixed_roles(&mut cells, fragments.len())?;
                            }
                            if let Some(table) = part.table() {
                                self.place_mixed_table_leaves(
                                    table,
                                    page,
                                    Some(fragment.definition_index()),
                                    top,
                                    part.height(),
                                    &mut fragments,
                                    &mut cells,
                                    &mut repeated_captions,
                                    &mut variants,
                                )?;
                            }
                        }
                    } else {
                        let start = fragment.consumed_range()?.start;
                        self.content_placement().place_content_range(
                            &mut fragments,
                            page,
                            Some(fragment.definition_index()),
                            start..start + fragment.items()?.len(),
                            origin,
                            fragment.used_height(),
                        )?;
                        self.extend_mixed_roles(&mut cells, fragments.len())?;
                    }
                }
            }
            let separator = self
                .content_placement()
                .place_separator(candidate.footnote_bounds())?;
            let repetitions =
                roles_with_repetition(&cells, &repeated_captions).map(|(_, repeated)| repeated);
            let (mut lists, mut notes) = if variants.is_empty() {
                self.content_placement()
                    .place_page_markers_with_repetition(&fragments, repetitions)?
            } else {
                self.place_variant_page_markers(&fragments, &variants, repetitions)?
            };
            if variants.is_empty() {
                self.translate_page_origins(selection, &mut fragments, &mut lists, &mut notes)?;
            } else {
                self.translate_variant_page_origins(
                    selection,
                    &mut fragments,
                    &variants,
                    &mut lists,
                    &mut notes,
                )?;
            }
            let numbers = self.place_page_equation_numbers(
                &fragments,
                &cells,
                &repeated_captions,
                fragment_offset,
                &variants,
            )?;
            fragment_offset = fragment_offset
                .checked_add(fragments.len())
                .ok_or_else(|| error(root, E::FragmentLimit))?;
            pages.push(BookV2BodyMixedPlacedPage {
                selection,
                fragments,
                cells,
                repeated_captions,
                variants,
                lists,
                notes,
                separator,
                numbers,
            });
        }
        Ok(BookV2BodyMixedPlacedSequence { sequence, pages })
    }
    pub(super) fn page_origin_delta(
        &self,
        selection: &BookV2BodyMixedPageSelection<'b, 'f, 's, 'p, 'a>,
        definition: bool,
    ) -> Result<Length, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let frames = self
            .content
            .flow
            .lines()
            .frames()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let regions = if definition {
            (
                selection.declared_footnote_region(),
                frames.footnote_region(),
            )
        } else {
            (Some(selection.body_bounds()), Some(frames.body()))
        };
        match regions {
            (Some(actual), Some(measured)) => actual
                .x()
                .checked_sub(measured.x())
                .ok_or_else(|| error(root, E::ArithmeticOverflow)),
            (None, None) => Ok(Length::ZERO),
            _ => Err(error(root, E::ReceiptMismatch)),
        }
    }
    /// Ordinary and table fragments share one measurement coordinate system.
    /// Translate each region once, before deriving equation-number viewports;
    /// list/note markers already placed from measured frames move with it.
    fn translate_page_origins(
        &mut self,
        selection: &BookV2BodyMixedPageSelection<'b, 'f, 's, 'p, 'a>,
        fragments: &mut [ProductionBodyFootnotePlacedFragment],
        lists: &mut [ProductionBodyListMarker],
        notes: &mut [ProductionBodyFootnotePlacedMarker],
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let body = self.page_origin_delta(selection, false)?;
        let note = self.page_origin_delta(selection, true)?;
        if body == Length::ZERO && note == Length::ZERO {
            return Ok(());
        }
        for placed in fragments.iter_mut() {
            let owner = placed.fragment.owner;
            self.content.step(owner)?;
            let delta = if placed.definition.is_some() {
                note
            } else {
                body
            };
            placed.fragment.bounds = translate_page_rect_x(placed.fragment.bounds, delta, owner)?;
            placed.fragment.viewport = placed
                .fragment
                .viewport
                .map(|rect| translate_page_rect_x(rect, delta, owner))
                .transpose()?;
        }
        for marker in lists {
            self.content.step(marker.owner())?;
            let fragment = fragments
                .get(marker.fragment_index() as usize)
                .ok_or_else(|| error(marker.owner(), E::ReceiptMismatch))?;
            marker.translate_x(if fragment.definition.is_some() {
                note
            } else {
                body
            })?;
        }
        for marker in notes {
            let fragment = fragments
                .get(marker.fragment_index() as usize)
                .filter(|p| p.definition == Some(marker.definition_index()))
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            self.content.step(fragment.fragment.owner)?;
            marker.translate_x(note, fragment.fragment.owner)?;
        }
        Ok(())
    }
    fn place_mixed_table_leaves(
        &mut self,
        table: &crate::production_body::body_flow::book_v2::BookV2TableFragmentSelection<
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        page: u32,
        definition: Option<usize>,
        origin: Length,
        height: Length,
        fragments: &mut Vec<ProductionBodyFootnotePlacedFragment>,
        cells: &mut Vec<Option<ProductionTablePlacedCellRole>>,
        repeated_captions: &mut Vec<usize>,
        variants: &mut Vec<BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a>>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let flow = self.content.flow;
        table.verify_source_flow(flow, definition)?;
        if table.has_header_variants() {
            let mut count = 0usize;
            for leaf in table.variant_placement_leaves() {
                let leaf = leaf?;
                self.content
                    .step(leaf.cell_owner().unwrap_or(NodeId::new(0)))?;
                if leaf.uses_header_variant() {
                    self.content.charge.take(1, NodeId::new(0))?;
                    count = count
                        .checked_add(1)
                        .ok_or_else(|| error(NodeId::new(0), E::FragmentLimit))?;
                }
            }
            variants
                .try_reserve_exact(count)
                .map_err(|_| error(NodeId::new(0), E::AllocationFailure))?;
        }
        let ordinary = table
            .source_placement_leaves()
            .take(if !table.has_header_variants() {
                usize::MAX
            } else {
                0
            })
            .map(|leaf| {
                leaf.map(|leaf| {
                    (
                        flow,
                        leaf.item_index(),
                        leaf.cell_owner(),
                        leaf.top(),
                        leaf.repeated_header(),
                        None,
                    )
                })
            });
        let varied = table.variant_placement_leaves().map(|leaf| {
            let leaf = leaf?;
            let flow = leaf.measurements().flow();
            let range = match definition {
                Some(d) => flow
                    .collected
                    .definitions
                    .get(d)
                    .cloned()
                    .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?,
                None => 0..flow.collected.body_end,
            };
            if !range.contains(&leaf.global_item_index()) {
                return Err(error(NodeId::new(0), E::ReceiptMismatch));
            }
            Ok((
                flow,
                leaf.global_item_index() - range.start,
                leaf.cell_owner(),
                leaf.top(),
                leaf.repeated(),
                leaf.header_source()
                    .map(|(header, _)| (header, leaf.global_item_index())),
            ))
        });
        for leaf in ordinary.chain(varied) {
            let (flow, index, cell, relative, repeated, variant) = leaf?;
            let item = definition
                .map_or_else(|| Some(flow.body_items()), |d| flow.definition_items(d))
                .and_then(|items| items.get(index))
                .ok_or_else(|| error(cell.unwrap_or(NodeId::new(0)), E::ReceiptMismatch))?;
            let owner = cell.unwrap_or(item.owner);
            self.content.step(owner)?;
            let before = relative
                .checked_sub(item.leading)
                .ok_or_else(|| error(item.owner, E::ArithmeticOverflow))?;
            let end = add(
                add(relative, item.height, item.owner)?,
                item.trailing,
                item.owner,
            )?;
            if before < Length::ZERO || end > height {
                return Err(error(item.owner, E::ReceiptMismatch));
            }
            let top = add(origin, relative, item.owner)?;
            let fragment = place_flow_item_shared(
                BodyLines::BookV2(flow.lines()),
                BodyBlocks::BookV2(flow.blocks().map_or(&[], |b| b.blocks())),
                item,
                page,
                top,
                item.before,
            )?;
            self.content.charge.take(2, item.owner)?;
            fragments
                .try_reserve(1)
                .map_err(|_| error(item.owner, E::AllocationFailure))?;
            cells
                .try_reserve(1)
                .map_err(|_| error(item.owner, E::AllocationFailure))?;
            if repeated && cell.is_none() {
                self.content.charge.take(1, item.owner)?;
                self.content.step(item.owner)?;
                repeated_captions
                    .try_reserve(1)
                    .map_err(|_| error(item.owner, E::AllocationFailure))?;
                repeated_captions.push(fragments.len());
            }
            if let Some((header, item)) = variant {
                variants.push(BookV2BodyPlacedHeaderVariant {
                    fragment: fragments.len(),
                    item,
                    header,
                });
            }
            fragments.push(ProductionBodyFootnotePlacedFragment {
                definition,
                item_index: index,
                fragment,
            });
            cells.push(cell.map(|owner| ProductionTablePlacedCellRole {
                owner,
                repeated_header: repeated,
            }));
        }
        Ok(())
    }
    fn place_page_equation_numbers(
        &mut self,
        fragments: &[ProductionBodyFootnotePlacedFragment],
        roles: &[Option<ProductionTablePlacedCellRole>],
        repeated_captions: &[usize],
        offset: usize,
        variants: &[BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a>],
    ) -> Result<Vec<BookV2BodyPlacedEquationNumber>, ProductionBodyPaginationError> {
        let mut numbers = Vec::new();
        if variants.is_empty() && self.content.flow.blocks().is_none() {
            return Ok(numbers);
        }
        for (index, (placed, (_, repeated))) in fragments
            .iter()
            .zip(roles_with_repetition(roles, repeated_captions))
            .enumerate()
        {
            let fragment = placed.fragment();
            let owner = fragment.owner();
            self.content.step(owner)?;
            let ProductionBodyFragmentSource::VectorBlock { block_index } = fragment.source()
            else {
                continue;
            };
            let flow = self.placed_header_flow(variants, index)?;
            let blocks = flow
                .blocks()
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let block = blocks
                .blocks()
                .get(block_index as usize)
                .filter(|b| b.owner() == owner)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let Some(number) = block.equation_number() else {
                continue;
            };
            let shapes = blocks
                .numbers()
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            for _ in 0..(usize::BITS - shapes.shapes().len().max(1).leading_zeros() + 1) {
                self.content.step(owner)?;
            }
            let shape = shapes
                .shape(owner)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            if !std::ptr::eq(shape.source(), block.binding().source())
                || shape.fingerprint() != number.shape_fingerprint()
                || shape.width() != number.width()
                || shape.height() != number.height()
            {
                return Err(error(owner, E::ReceiptMismatch));
            }
            self.content.charge.take(1, owner)?;
            numbers
                .try_reserve(1)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            numbers.push(BookV2BodyPlacedEquationNumber {
                geometry: terminals::place_equation_number(
                    offset
                        .checked_add(index)
                        .ok_or_else(|| error(owner, E::FragmentLimit))?,
                    &fragment,
                    number,
                )?,
                repeated_header: repeated,
            });
        }
        Ok(numbers)
    }
    fn extend_mixed_roles(
        &mut self,
        cells: &mut Vec<Option<ProductionTablePlacedCellRole>>,
        count: usize,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let extra = count
            .checked_sub(cells.len())
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        self.content.charge.take(extra, root)?;
        cells
            .try_reserve(extra)
            .map_err(|_| error(root, E::AllocationFailure))?;
        cells.resize(count, None);
        Ok(())
    }
}

/// Sparse caption copies preserve old cell-only allocations and source roles.
fn roles_with_repetition<'r>(
    cells: &'r [Option<ProductionTablePlacedCellRole>],
    captions: &'r [usize],
) -> impl Iterator<Item = (Option<ProductionTablePlacedCellRole>, bool)> + 'r {
    let mut copies = captions.iter().copied().peekable();
    cells.iter().copied().enumerate().map(move |(index, cell)| {
        let caption_copy = copies.peek() == Some(&index);
        if caption_copy {
            copies.next();
        }
        (
            cell,
            caption_copy || cell.is_some_and(|c| c.repeated_header()),
        )
    })
}

#[cfg(test)]
mod repetition_tests {
    use super::*;
    #[test]
    fn detached_caption_repetition_preserves_absent_cell_owners() {
        let cell = ProductionTablePlacedCellRole {
            owner: NodeId::new(9),
            repeated_header: false,
        };
        let header = ProductionTablePlacedCellRole {
            repeated_header: true,
            ..cell
        };
        let cells = [None, Some(cell), None, Some(header), None];
        let actual = roles_with_repetition(&cells, &[0, 2]).collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                (None, true),
                (Some(cell), false),
                (None, true),
                (Some(header), true),
                (None, false)
            ]
        );
        assert_eq!(
            roles_with_repetition(&cells, &[])
                .map(|(_, r)| r)
                .collect::<Vec<_>>(),
            vec![false, false, false, true, false]
        );
    }
}
