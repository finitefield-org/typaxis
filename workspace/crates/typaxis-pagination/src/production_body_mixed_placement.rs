//! Physical placement of selected mixed pages; repeated header copies stay explicit.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionTablePlacedCellRole {
    pub(super) owner: NodeId,
    pub(super) repeated_header: bool,
}
impl ProductionTablePlacedCellRole {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    /// This copy must become an artifact in final paint/structure, not another
    /// semantic cell or another terminal consumption of a source formula.
    pub const fn repeated_header(&self) -> bool {
        self.repeated_header
    }
}
pub struct ProductionBodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a> {
    selection: &'q ProductionBodyMixedPageSelection<'b, 'f, 's, 'p, 'a>,
    fragments: Vec<ProductionBodyFootnotePlacedFragment>,
    cells: Vec<Option<ProductionTablePlacedCellRole>>,
    lists: Vec<ProductionBodyListMarker>,
    notes: Vec<ProductionBodyFootnotePlacedMarker>,
    separator: Option<Rect>,
}
impl<'q, 'b, 'f, 's, 'p, 'a> ProductionBodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a> {
    pub fn selection(&self) -> &'q ProductionBodyMixedPageSelection<'b, 'f, 's, 'p, 'a> {
        self.selection
    }
    pub fn fragments(&self) -> &[ProductionBodyFootnotePlacedFragment] {
        &self.fragments
    }
    pub fn cell_roles(&self) -> &[Option<ProductionTablePlacedCellRole>] {
        &self.cells
    }
    pub fn list_markers(&self) -> &[ProductionBodyListMarker] {
        &self.lists
    }
    pub fn footnote_markers(&self) -> &[ProductionBodyFootnotePlacedMarker] {
        &self.notes
    }
    pub const fn separator_ink(&self) -> Option<Rect> {
        self.separator
    }
}
pub struct ProductionBodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a> {
    sequence: &'q ProductionBodyMixedPageSequence<'b, 'f, 's, 'p, 'a>,
    pages: Vec<ProductionBodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a>>,
}
impl<'q, 'b, 'f, 's, 'p, 'a> ProductionBodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a> {
    pub fn sequence(&self) -> &'q ProductionBodyMixedPageSequence<'b, 'f, 's, 'p, 'a> {
        self.sequence
    }
    pub fn pages(&self) -> &[ProductionBodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a>] {
        &self.pages
    }
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Assign actual page origins to the complete selected sequence. This does
    /// not establish stable reflow, PDF paint authorization or repeated-header
    /// Artifact structure; those must consume the explicit role on each copy.
    pub fn place_mixed_pages<'q>(
        &mut self,
        sequence: &'q ProductionBodyMixedPageSequence<'b, 'f, 's, 'p, 'a>,
    ) -> Result<
        ProductionBodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a>,
        ProductionBodyPaginationError,
    > {
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
        for selection in sequence.pages() {
            self.step(root)?;
            let candidate = selection.candidate();
            let page = selection.page_index();
            let mut fragments = Vec::new();
            let mut cells = Vec::new();
            for part in candidate.parts() {
                self.step(root)?;
                let origin = add(flow.blocks.page_geometry().body().y(), part.top(), root)?;
                if let Some(range) = part.items() {
                    self.place_content_range(
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
                    table.verify_demand_flow(flow)?;
                    for leaf in table.placement_leaves() {
                        let (cell, index, relative, repeated) = leaf?;
                        self.step(cell)?;
                        let item = flow
                            .body_items()
                            .get(index)
                            .ok_or_else(|| error(cell, E::ReceiptMismatch))?;
                        let before = relative
                            .checked_sub(item.leading)
                            .ok_or_else(|| error(item.owner, E::ArithmeticOverflow))?;
                        let end = add(
                            add(relative, item.height, item.owner)?,
                            item.trailing,
                            item.owner,
                        )?;
                        if before < Length::ZERO || end > part.height() {
                            return Err(error(item.owner, E::ReceiptMismatch));
                        }
                        let top = add(origin, relative, item.owner)?;
                        let fragment =
                            place_flow_item(flow.lines, flow.blocks, item, page, top, item.before)?;
                        self.content.charge.take(2, item.owner)?;
                        fragments
                            .try_reserve(1)
                            .map_err(|_| error(item.owner, E::AllocationFailure))?;
                        cells
                            .try_reserve(1)
                            .map_err(|_| error(item.owner, E::AllocationFailure))?;
                        fragments.push(ProductionBodyFootnotePlacedFragment {
                            definition: None,
                            item_index: index,
                            fragment,
                        });
                        cells.push(Some(ProductionTablePlacedCellRole {
                            owner: cell,
                            repeated_header: repeated,
                        }));
                    }
                }
            }
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
                        page,
                        Some(fragment.definition_index()),
                        start..start + fragment.items().len(),
                        origin,
                        fragment.used_height(),
                    )?;
                    self.extend_mixed_roles(&mut cells, fragments.len())?;
                }
            }
            let separator = self.place_separator(candidate.footnote_bounds())?;
            let (lists, notes) = self.place_page_markers(&fragments)?;
            pages.push(ProductionBodyMixedPlacedPage {
                selection,
                fragments,
                cells,
                lists,
                notes,
                separator,
            });
        }
        Ok(ProductionBodyMixedPlacedSequence { sequence, pages })
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
