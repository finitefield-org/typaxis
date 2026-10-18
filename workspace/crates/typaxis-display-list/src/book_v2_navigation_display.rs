//! Original navigation sources and selected physical coordinates.
use super::*;
use std::marker::PhantomData;
use typaxis_core::AnchorId;
use typaxis_document::StagingOutlineEntry;
use typaxis_syntax::ProductionTextParagraph;

mod navigation_sealed {
    use super::*;
    pub trait Facts {
        fn navigation_paragraphs(&self) -> &[ProductionTextParagraph<'_>];
        fn navigation_anchors(&self) -> &[(AnchorId, NodeId)];
        fn navigation_outline(&self) -> &[StagingOutlineEntry];
    }
}

/// This source view borrows original records from its authentic display owner.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2NavigationSourceView, BookV2BodyDisplay, BookV2ColumnDisplay};
/// fn body<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(view: BookV2NavigationSourceView<'v, BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>>) -> BookV2NavigationSourceView<'v, BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>> { view }
/// ```
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2NavigationSourceView, BookV2BodyDisplay};
/// fn forge<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(display: &'v BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>) -> BookV2NavigationSourceView<'v, BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>> { BookV2NavigationSourceView { display } }
/// ```
pub struct BookV2NavigationSourceView<'v, D: navigation_sealed::Facts> {
    display: &'v D,
}
impl<'v, D: navigation_sealed::Facts> BookV2NavigationSourceView<'v, D> {
    pub fn paragraphs(&self) -> &'v [ProductionTextParagraph<'v>] {
        self.display.navigation_paragraphs()
    }
    pub fn anchors(&self) -> &'v [(AnchorId, NodeId)] {
        self.display.navigation_anchors()
    }
    pub fn outline(&self) -> &'v [StagingOutlineEntry] {
        self.display.navigation_outline()
    }
}

/// Selected anchor coordinates retain the physical paint family.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2NavigationAnchorView, BookV2BodyPaintIndex, BookV2ColumnPaintIndex};
/// fn body(view: BookV2NavigationAnchorView<BookV2ColumnPaintIndex>) -> BookV2NavigationAnchorView<BookV2BodyPaintIndex> { view }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2NavigationAnchorView<P: Copy> {
    owner: NodeId,
    page: u32,
    fragment: usize,
    repeated: bool,
    x: Length,
    baseline: Length,
    paint: PhantomData<P>,
}
impl<P: Copy> BookV2NavigationAnchorView<P> {
    pub fn owner(self) -> NodeId {
        self.owner
    }
    pub fn page_index(self) -> u32 {
        self.page
    }
    pub fn fragment_index(self) -> usize {
        self.fragment
    }
    pub fn repeated_header(self) -> bool {
        self.repeated
    }
    pub fn x(self) -> Length {
        self.x
    }
    pub fn baseline(self) -> Length {
        self.baseline
    }
}

/// A selected empty line keeps its original line box without a paint or MCID.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2NavigationLineView, BookV2BodyPaintIndex, BookV2ColumnPaintIndex};
/// fn body(view: BookV2NavigationLineView<BookV2ColumnPaintIndex>) -> BookV2NavigationLineView<BookV2BodyPaintIndex> { view }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2NavigationLineView<P: Copy> {
    owner: NodeId,
    page: u32,
    fragment: usize,
    repeated: bool,
    bounds: Rect,
    paint: PhantomData<P>,
}
impl<P: Copy> BookV2NavigationLineView<P> {
    pub fn owner(self) -> NodeId {
        self.owner
    }
    pub fn page_index(self) -> u32 {
        self.page
    }
    pub fn fragment_index(self) -> usize {
        self.fragment
    }
    pub fn repeated_header(self) -> bool {
        self.repeated
    }
    pub fn bounds(self) -> Rect {
        self.bounds
    }
}

/// The relation supertrait seals navigation to the actual Body and Column.
pub trait BookV2NavigationDisplay<'a>:
    BookV2RelationDisplay<'a> + navigation_sealed::Facts
{
    fn navigation_source(&self) -> BookV2NavigationSourceView<'_, Self>
    where
        Self: Sized,
    {
        BookV2NavigationSourceView { display: self }
    }
    fn navigation_paint_bounds(&self, index: usize)
        -> Result<Option<Rect>, BookV2MathDisplayError>;
    fn navigation_anchor_count(&self) -> usize;
    fn navigation_anchor(&self, index: usize) -> Option<BookV2NavigationAnchorView<Self::Paint>>;
    fn navigation_nonpainting_line_count(&self) -> usize;
    fn navigation_nonpainting_line(
        &self,
        index: usize,
    ) -> Option<BookV2NavigationLineView<Self::Paint>>;
}

macro_rules! authentic_navigation_display {
    ($display:ident, $paint:ident $(, $regions:ident)?) => {
        impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> navigation_sealed::Facts
            for $display<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
        {
            fn navigation_paragraphs(&self) -> &[ProductionTextParagraph<'_>] {
                self.source().source().original_graph().lines().prepared().source_flow().paragraphs()
            }
            fn navigation_anchors(&self) -> &[(AnchorId, NodeId)] {
                self.source().source().original_graph().lines().prepared().source_flow().navigation().anchors()
            }
            fn navigation_outline(&self) -> &[StagingOutlineEntry] {
                self.source().source().original_graph().lines().prepared().source_flow().navigation().outline()
            }
        }
        impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2NavigationDisplay<'a>
            for $display<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
        {
            fn navigation_paint_bounds(&self, index: usize) -> Result<Option<Rect>, BookV2MathDisplayError> {
                use $paint as P;
                let invalid = || error(NodeId::new(0), E::ReceiptMismatch);
                let paint = *self.paints().get(index).ok_or_else(invalid)?;
                Ok(match paint {
                    P::Text(i) => self.text().draws().get(i).ok_or_else(invalid)?.logical_bounds(),
                    P::Marker(i) => Some(self.markers().draws().get(i).ok_or_else(invalid)?.placement().bounds()),
                    P::Math(i) => Some(match self.math().draws().get(i).ok_or_else(invalid)?.paint() {
                        BookV2MathPaint::Native(n) => n.bounds(),
                        BookV2MathPaint::Vector(v) => v.viewport(),
                    }),
                    P::Image(i) => Some(self.images().draws().get(i).ok_or_else(invalid)?.paint().viewport()),
                    P::EquationNumber(i) => Some(self.numbers().draws().get(i).ok_or_else(invalid)?.placement().geometry().bounds()),
                    P::FootnoteSeparator(_) => return Err(invalid().into()),
                    $(P::PageRegionText { .. } => {
                        let _ = &self.$regions();
                        return Err(invalid().into());
                    },)?
                })
            }
            fn navigation_anchor_count(&self) -> usize { self.anchors().positions().len() }
            fn navigation_anchor(&self, index: usize) -> Option<BookV2NavigationAnchorView<Self::Paint>> {
                let a = self.anchors().positions().get(index)?;
                Some(BookV2NavigationAnchorView {
                    owner: a.anchor().source().owner(),
                    page: a.fragment().fragment().page_index(),
                    fragment: a.fragment_index(),
                    repeated: a.repeated_header(),
                    x: a.x(),
                    baseline: a.baseline(),
                    paint: PhantomData,
                })
            }
            fn navigation_nonpainting_line_count(&self) -> usize { self.anchors().nonpainting_lines().len() }
            fn navigation_nonpainting_line(&self, index: usize) -> Option<BookV2NavigationLineView<Self::Paint>> {
                let line = self.anchors().nonpainting_lines().get(index)?;
                let fragment = line.fragment().fragment();
                Some(BookV2NavigationLineView {
                    owner: fragment.owner(),
                    page: fragment.page_index(),
                    fragment: line.fragment_index(),
                    repeated: line.repeated_header(),
                    bounds: fragment.bounds(),
                    paint: PhantomData,
                })
            }
        }
    };
}
authentic_navigation_display!(BookV2BodyDisplay, BookV2BodyPaintIndex, page_regions);
authentic_navigation_display!(BookV2ColumnDisplay, BookV2ColumnPaintIndex, page_regions);
