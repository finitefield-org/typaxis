//! Actual column origins over shared body/table/note leaf placement.
use super::super::mixed_placement::{roles_with_repetition, BookV2PageContent};
use super::*;
use crate::production_body::body_flow::book_v2::BookV2PreparedBodyFlow;

/// An original column and its exact contiguous physical fragment range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookV2ColumnPlacedBody {
    bounds: Rect,
    fragments: std::ops::Range<usize>,
}
impl BookV2ColumnPlacedBody {
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn fragments(&self) -> std::ops::Range<usize> {
        self.fragments.clone()
    }
}

/// Borrow a header's actual graph without exposing a single-body measurement.
pub struct BookV2ColumnPlacedHeaderVariant<'h, 'b, 'f, 's, 'p, 'a> {
    inner: &'h BookV2BodyPlacedHeaderVariant<'b, 'f, 's, 'p, 'a>,
}
impl<'h, 'b, 'f, 's, 'p, 'a> BookV2ColumnPlacedHeaderVariant<'h, 'b, 'f, 's, 'p, 'a> {
    pub fn fragment_index(&self) -> usize {
        self.inner.fragment_index()
    }
    pub fn global_item_index(&self) -> usize {
        self.inner.global_item_index()
    }
    pub fn table_index(&self) -> usize {
        self.inner.header().table_index()
    }
    pub fn definition_index(&self) -> Option<usize> {
        self.inner.header().definition_index()
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.inner.header().fingerprint()
    }
    pub fn lines(&self) -> &'s typaxis_layout::book_v2::BookV2InlineLineLayout<'p, 'a> {
        self.inner.measurements().flow().lines()
    }
}

pub struct BookV2ColumnPlacedPage<'q, 'b, 'f, 's, 'p, 'a> {
    selection: &'q BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>,
    content: BookV2PageContent<'b, 'f, 's, 'p, 'a>,
    columns: Vec<BookV2ColumnPlacedBody>,
    numbers: Vec<BookV2BodyPlacedEquationNumber>,
}
impl<'q, 'b, 'f, 's, 'p, 'a> BookV2ColumnPlacedPage<'q, 'b, 'f, 's, 'p, 'a> {
    pub fn selection(&self) -> &'q BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a> {
        self.selection
    }
    pub fn columns(&self) -> &[BookV2ColumnPlacedBody] {
        &self.columns
    }
    pub fn fragments(&self) -> &[ProductionBodyFootnotePlacedFragment] {
        &self.content.fragments
    }
    pub fn cell_roles(&self) -> &[Option<ProductionTablePlacedCellRole>] {
        &self.content.cells
    }
    pub fn fragments_with_roles(
        &self,
    ) -> impl Iterator<
        Item = (
            &ProductionBodyFootnotePlacedFragment,
            Option<ProductionTablePlacedCellRole>,
            bool,
        ),
    > + '_ {
        self.content
            .fragments
            .iter()
            .zip(roles_with_repetition(
                &self.content.cells,
                &self.content.repeated_captions,
            ))
            .map(|(fragment, (cell, repeated))| (fragment, cell, repeated))
    }
    pub fn header_variant(
        &self,
        fragment: usize,
    ) -> Option<BookV2ColumnPlacedHeaderVariant<'_, 'b, 'f, 's, 'p, 'a>> {
        self.content
            .variants
            .binary_search_by_key(&fragment, |v| v.fragment_index())
            .ok()
            .map(|i| BookV2ColumnPlacedHeaderVariant {
                inner: &self.content.variants[i],
            })
    }
    pub fn header_variants(
        &self,
    ) -> impl ExactSizeIterator<Item = BookV2ColumnPlacedHeaderVariant<'_, 'b, 'f, 's, 'p, 'a>>
    {
        self.content
            .variants
            .iter()
            .map(|inner| BookV2ColumnPlacedHeaderVariant { inner })
    }
    pub fn fragment_column(&self, fragment: usize) -> Option<u16> {
        column_index(&self.columns, fragment).and_then(|i| u16::try_from(i).ok())
    }
    pub fn list_markers(&self) -> &[ProductionBodyListMarker] {
        &self.content.lists
    }
    pub fn footnote_markers(&self) -> &[ProductionBodyFootnotePlacedMarker] {
        &self.content.notes
    }
    pub fn equation_numbers(&self) -> &[BookV2BodyPlacedEquationNumber] {
        &self.numbers
    }
    pub fn separator_ink(&self) -> Option<Rect> {
        self.content.separator
    }
}

/// Column geometry cannot authorize the single-body display/PDF path.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnPlacedSequence, BookV2BodyMixedPlacedSequence};
/// fn single<'q, 'b, 'f, 's, 'p, 'a>(v: BookV2ColumnPlacedSequence<'q, 'b, 'f, 's, 'p, 'a>)
///     -> BookV2BodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnPlacedSequence<'q, 'b, 'f, 's, 'p, 'a> {
    sequence: &'q BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>,
    pages: Vec<BookV2ColumnPlacedPage<'q, 'b, 'f, 's, 'p, 'a>>,
}
impl<'q, 'b, 'f, 's, 'p, 'a> BookV2ColumnPlacedSequence<'q, 'b, 'f, 's, 'p, 'a> {
    pub fn sequence(&self) -> &'q BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a> {
        self.sequence
    }
    pub fn pages(&self) -> &[BookV2ColumnPlacedPage<'q, 'b, 'f, 's, 'p, 'a>] {
        &self.pages
    }
}

fn column_index(columns: &[BookV2ColumnPlacedBody], fragment: usize) -> Option<usize> {
    let index = columns.partition_point(|c| c.fragments.end <= fragment);
    columns
        .get(index)
        .filter(|c| c.fragments.contains(&fragment))
        .map(|_| index)
}
fn origin_delta(
    flow: &BookV2PreparedBodyFlow<'_, '_, '_, '_>,
    actual: Rect,
    note: bool,
) -> Result<Length, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    let measured = flow
        .lines()
        .frames()
        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
    let measured = if note {
        measured
            .footnote_region()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?
    } else {
        measured.body()
    };
    actual
        .x()
        .checked_sub(measured.x())
        .ok_or_else(|| error(root, E::ArithmeticOverflow))
}
fn contains(bounds: Rect, fragment: Rect) -> Result<bool, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    Ok(fragment.x() >= bounds.x()
        && fragment.y() >= bounds.y()
        && add(fragment.x(), fragment.width().get(), root)?
            <= add(bounds.x(), bounds.width().get(), root)?
        && add(fragment.y(), fragment.height().get(), root)?
            <= add(bounds.y(), bounds.height().get(), root)?)
}

impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    /// Place actual columns and one note region. Stable selection, source
    /// closure, balancing and formula/display authorization are separate proofs.
    pub fn place_column_pages<'q>(
        &mut self,
        sequence: &'q BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2ColumnPlacedSequence<'q, 'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.verify_sequence(sequence)?;
        let root = NodeId::new(0);
        self.inner.content.charge.take(
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
        let mut offset = 0usize;
        for selection in sequence.pages() {
            self.inner.content.step(root)?;
            let candidate = selection.candidate();
            let count = candidate.columns().len();
            self.inner.content.charge.take(
                count
                    .checked_add(1)
                    .ok_or_else(|| error(root, E::FragmentLimit))?,
                root,
            )?;
            let mut columns = Vec::new();
            columns
                .try_reserve_exact(count)
                .map_err(|_| error(root, E::AllocationFailure))?;
            let mut content = self.inner.place_page_regions_content(
                selection.page_index(),
                candidate.columns().iter().map(|c| (c.bounds(), c.parts())),
                candidate.footnotes(),
                candidate.footnote_bounds(),
                |bounds, fragments| {
                    columns.push(BookV2ColumnPlacedBody { bounds, fragments });
                    Ok(())
                },
            )?;
            self.translate_column_content(selection, &columns, &mut content)?;
            let numbers = self.inner.place_page_equation_numbers(
                &content.fragments,
                &content.cells,
                &content.repeated_captions,
                offset,
                &content.variants,
            )?;
            offset = offset
                .checked_add(content.fragments.len())
                .ok_or_else(|| error(root, E::FragmentLimit))?;
            pages.push(BookV2ColumnPlacedPage {
                selection,
                content,
                columns,
                numbers,
            });
        }
        Ok(BookV2ColumnPlacedSequence { sequence, pages })
    }
    fn fragment_region(
        &mut self,
        selection: &BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>,
        columns: &[BookV2ColumnPlacedBody],
        index: usize,
        definition: Option<usize>,
    ) -> Result<Rect, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        for _ in 0..(u64::from(columns.len().checked_ilog2().unwrap_or(0)) + 2) {
            self.inner.content.step(root)?;
        }
        let column = column_index(columns, index);
        match (column, definition) {
            (Some(i), None) => Ok(columns[i].bounds),
            (None, Some(_)) => selection
                .candidate()
                .footnote_bounds()
                .ok_or_else(|| error(root, E::ReceiptMismatch)),
            _ => Err(error(root, E::ReceiptMismatch)),
        }
    }
    fn translate_column_content(
        &mut self,
        selection: &BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>,
        columns: &[BookV2ColumnPlacedBody],
        content: &mut BookV2PageContent<'b, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        for (index, placed) in content.fragments.iter_mut().enumerate() {
            let owner = placed.fragment.owner;
            self.inner.content.step(owner)?;
            let flow = self.inner.placed_header_flow(&content.variants, index)?;
            let region = self.fragment_region(selection, columns, index, placed.definition)?;
            let delta = origin_delta(flow, region, placed.definition.is_some())?;
            placed.fragment.bounds = translate_page_rect_x(placed.fragment.bounds, delta, owner)?;
            placed.fragment.viewport = placed
                .fragment
                .viewport
                .map(|r| translate_page_rect_x(r, delta, owner))
                .transpose()?;
            if !contains(region, placed.fragment.bounds)? {
                return Err(error(owner, E::WidthMismatch));
            }
        }
        for marker in &mut content.lists {
            let index = marker.fragment_index() as usize;
            let placed = content
                .fragments
                .get(index)
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            let flow = self.inner.placed_header_flow(&content.variants, index)?;
            self.inner.content.step(marker.owner())?;
            let region = self.fragment_region(selection, columns, index, placed.definition)?;
            marker.translate_x(origin_delta(flow, region, placed.definition.is_some())?)?;
        }
        for marker in &mut content.notes {
            let index = marker.fragment_index() as usize;
            let placed = content
                .fragments
                .get(index)
                .filter(|p| p.definition == Some(marker.definition_index()))
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            let flow = self.inner.placed_header_flow(&content.variants, index)?;
            self.inner.content.step(placed.fragment.owner)?;
            let region = self.fragment_region(selection, columns, index, placed.definition)?;
            marker.translate_x(origin_delta(flow, region, true)?, placed.fragment.owner)?;
        }
        Ok(())
    }
    pub(in crate::production_body::body_flow) fn same_column_geometry(
        &mut self,
        left: &BookV2ColumnPlacedSequence<'_, 'b, 'f, 's, 'p, 'a>,
        right: &BookV2ColumnPlacedSequence<'_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<bool, ProductionBodyPaginationError> {
        if !self.same_column_selections(left.sequence(), right.sequence())? {
            return Ok(false);
        }
        for (a, b) in left.pages().iter().zip(right.pages()) {
            self.inner.content.step(NodeId::new(0))?;
            if a.separator_ink() != b.separator_ink()
                || !self.inner.same_records(a.columns(), b.columns())?
                || !self.inner.same_records(a.fragments(), b.fragments())?
                || !self.inner.same_records(a.cell_roles(), b.cell_roles())?
                || !self
                    .inner
                    .same_records(&a.content.repeated_captions, &b.content.repeated_captions)?
                || !self
                    .inner
                    .same_records(&a.content.variants, &b.content.variants)?
                || !self
                    .inner
                    .same_records(a.list_markers(), b.list_markers())?
                || !self
                    .inner
                    .same_records(a.footnote_markers(), b.footnote_markers())?
                || !self
                    .inner
                    .same_records(a.equation_numbers(), b.equation_numbers())?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
