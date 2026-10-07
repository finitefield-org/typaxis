//! Source-bound physical content and marker geometry before final paint closure.
use super::*;
#[path = "production_body_content_placement.rs"]
mod content_placement;
pub(in crate::production_body::body_flow) use content_placement::ContentPlacement;
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    fn content_placement(&mut self) -> ContentPlacement<'_, '_, 'p, 'a> {
        let flow = self.content.flow;
        ContentPlacement {
            lines: BodyLines::Legacy(flow.lines),
            blocks: BodyBlocks::Legacy(flow.blocks.blocks()),
            collected: &flow.collected,
            definition_markers: &flow.definition_markers,
            body: flow.blocks.page_geometry().body(),
            charge: &mut self.content.charge,
            steps: &mut self.content.steps,
            maximum_steps: self.content.maximum_steps,
        }
    }
}
#[path = "production_body_footnote_terminals.rs"]
mod math_terminals;
pub use math_terminals::{
    ProductionBodyFootnoteMathTerminals, ProductionFinalPage, ProductionFinalPageGeometry,
    ProductionFinalPageIter, ProductionFinalPages,
};
#[path = "production_body_footnote_stability.rs"]
mod stability;
pub use stability::ProductionBodyFootnoteStablePages;

/// Definition-local indices remain distinct from body-local indices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyFootnotePlacedFragment {
    pub(super) definition: Option<usize>,
    pub(super) item_index: usize,
    pub(super) fragment: ProductionBodyFragment,
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
    separator_ink: Option<Rect>,
    fragments: Vec<ProductionBodyFootnotePlacedFragment>,
    list_markers: Vec<ProductionBodyListMarker>,
    footnote_markers: Vec<ProductionBodyFootnotePlacedMarker>,
}
impl<'q, 'b, 'f, 's, 'p, 'a> ProductionBodyFootnotePlacedPage<'q, 'b, 'f, 's, 'p, 'a> {
    pub fn selection(&self) -> &'q ProductionBodyFootnotePageSelection<'b, 'f, 's, 'p, 'a> {
        self.selection
    }
    /// Full-width 0.5 pt separator ink at the top of the reserved 1 pt band.
    /// No separator exists on a page containing only an empty forced fragment.
    pub fn separator_ink(&self) -> Option<Rect> {
        self.separator_ink
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
        let separator_ink = self.place_separator(candidate.footnote_bounds())?;
        let (list_markers, footnote_markers) = self.place_page_markers(&fragments)?;
        Ok(ProductionBodyFootnotePlacedPage {
            selection,
            separator_ink,
            fragments,
            list_markers,
            footnote_markers,
        })
    }

    pub(super) fn place_separator(
        &mut self,
        bounds: Option<Rect>,
    ) -> Result<Option<Rect>, ProductionBodyPaginationError> {
        self.content_placement().place_separator(bounds)
    }
    pub(super) fn place_content_range(
        &mut self,
        result: &mut Vec<ProductionBodyFootnotePlacedFragment>,
        page: u32,
        definition: Option<usize>,
        range: std::ops::Range<usize>,
        origin: Length,
        expected: Length,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.content_placement()
            .place_content_range(result, page, definition, range, origin, expected)
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
    #[cfg(feature = "book-v2-staging")]
    pub(super) fn translate_x(
        &mut self,
        delta: Length,
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.bounds = translate_page_rect_x(self.bounds, delta, owner)?;
        Ok(())
    }

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
    pub(super) fn place_page_markers(
        &mut self,
        fragments: &[ProductionBodyFootnotePlacedFragment],
    ) -> Result<
        (
            Vec<ProductionBodyListMarker>,
            Vec<ProductionBodyFootnotePlacedMarker>,
        ),
        ProductionBodyPaginationError,
    > {
        self.content_placement().place_page_markers(fragments)
    }
}

/// Complete geometry borrowing the source-contiguous selected sequence.
/// This has no stable-page or public paint authority by itself.
pub struct ProductionBodyFootnotePlacedSequence<'q, 'b, 'f, 's, 'p, 'a> {
    sequence: &'q ProductionBodyFootnotePageSequence<'b, 'f, 's, 'p, 'a>,
    pages: Vec<ProductionBodyFootnotePlacedPage<'q, 'b, 'f, 's, 'p, 'a>>,
}
impl<'q, 'b, 'f, 's, 'p, 'a> ProductionBodyFootnotePlacedSequence<'q, 'b, 'f, 's, 'p, 'a> {
    pub fn sequence(&self) -> &'q ProductionBodyFootnotePageSequence<'b, 'f, 's, 'p, 'a> {
        self.sequence
    }
    pub fn pages(&self) -> &[ProductionBodyFootnotePlacedPage<'q, 'b, 'f, 's, 'p, 'a>] {
        &self.pages
    }
}
impl<'b, 'f, 's, 'p, 'a> ProductionFootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub fn place_pages_content<'q>(
        &mut self,
        sequence: &'q ProductionBodyFootnotePageSequence<'b, 'f, 's, 'p, 'a>,
    ) -> Result<
        ProductionBodyFootnotePlacedSequence<'q, 'b, 'f, 's, 'p, 'a>,
        ProductionBodyPaginationError,
    > {
        self.verify_sequence(sequence)?;
        let root = NodeId::new(0);
        self.content.charge.take(1, root)?;
        let mut pages = Vec::new();
        for selected in sequence.pages() {
            self.step(root)?;
            let page = self.place_page_content(selected)?;
            pages
                .try_reserve(1)
                .map_err(|_| error(root, E::AllocationFailure))?;
            pages.push(page);
        }
        Ok(ProductionBodyFootnotePlacedSequence { sequence, pages })
    }
}
