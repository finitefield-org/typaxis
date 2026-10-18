//! Shared, borrowed projection over distinct physical source proofs.
use super::*;
use typaxis_pagination::book_v2::{
    BookV2BodyMixedPlacedPage, BookV2ColumnMathTerminals, BookV2ColumnPlacedPage, BookV2SourceGraph,
};
use typaxis_pagination::{
    ProductionBodyFootnotePlacedFragment, ProductionBodyFootnotePlacedMarker,
    ProductionBodyListMarker, ProductionTablePlacedCellRole,
};

#[derive(Clone, Copy)]
pub(super) enum SourceView<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    Body(&'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>),
    Column(&'d BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>),
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> SourceView<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub(super) fn original_graph(self) -> BookV2SourceGraph<'b, 'f, 's, 'p, 'a> {
        match self {
            Self::Body(s) => s.original_graph(),
            Self::Column(s) => s.original_graph(),
        }
    }
    pub(super) fn fragment_graph(
        self,
        index: usize,
    ) -> Option<BookV2SourceGraph<'b, 'f, 's, 'p, 'a>> {
        match self {
            Self::Body(s) => s.fragment_graph(index),
            Self::Column(s) => s.fragment_graph(index),
        }
    }
    pub(super) fn fingerprint(self) -> [u8; 32] {
        match self {
            Self::Body(s) => s.fingerprint(),
            Self::Column(s) => s.fingerprint(),
        }
    }
    pub(super) fn pages(self) -> Pages<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        match self {
            Self::Body(s) => Pages::Body(s.source().geometry().pages()),
            Self::Column(s) => Pages::Column(s.source().geometry().pages()),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Pages<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    Body(&'g [BookV2BodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a>]),
    Column(&'g [BookV2ColumnPlacedPage<'q, 'b, 'f, 's, 'p, 'a>]),
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> Pages<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub(super) fn len(self) -> usize {
        match self {
            Self::Body(p) => p.len(),
            Self::Column(p) => p.len(),
        }
    }
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> IntoIterator for Pages<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    type Item = PageView<'g, 'q, 'b, 'f, 's, 'p, 'a>;
    type IntoIter = PageIterator<'g, 'q, 'b, 'f, 's, 'p, 'a>;
    fn into_iter(self) -> Self::IntoIter {
        PageIterator {
            pages: self,
            index: 0,
        }
    }
}
pub(super) struct PageIterator<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pages: Pages<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    index: usize,
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> Iterator for PageIterator<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    type Item = PageView<'g, 'q, 'b, 'f, 's, 'p, 'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let item = match self.pages {
            Pages::Body(p) => p.get(self.index).map(PageView::Body),
            Pages::Column(p) => p.get(self.index).map(PageView::Column),
        };
        if item.is_some() {
            self.index += 1;
        }
        item
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.pages.len() - self.index;
        (remaining, Some(remaining))
    }
}
impl ExactSizeIterator for PageIterator<'_, '_, '_, '_, '_, '_, '_> {}

#[derive(Clone, Copy)]
pub(super) enum PageView<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    Body(&'g BookV2BodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a>),
    Column(&'g BookV2ColumnPlacedPage<'q, 'b, 'f, 's, 'p, 'a>),
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> PageView<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub(super) fn page_index(self) -> u32 {
        match self {
            Self::Body(p) => p.selection().page_index(),
            Self::Column(p) => p.selection().page_index(),
        }
    }
    pub(super) fn fragments(self) -> &'g [ProductionBodyFootnotePlacedFragment] {
        match self {
            Self::Body(p) => p.fragments(),
            Self::Column(p) => p.fragments(),
        }
    }
    pub(super) fn cell_roles(self) -> &'g [Option<ProductionTablePlacedCellRole>] {
        match self {
            Self::Body(p) => p.cell_roles(),
            Self::Column(p) => p.cell_roles(),
        }
    }
    pub(super) fn fragments_with_roles(
        self,
    ) -> impl Iterator<
        Item = (
            &'g ProductionBodyFootnotePlacedFragment,
            Option<ProductionTablePlacedCellRole>,
            bool,
        ),
    > {
        match self {
            Self::Body(p) => Either::Left(p.fragments_with_roles()),
            Self::Column(p) => Either::Right(p.fragments_with_roles()),
        }
    }
    pub(super) fn list_markers(self) -> &'g [ProductionBodyListMarker] {
        match self {
            Self::Body(p) => p.list_markers(),
            Self::Column(p) => p.list_markers(),
        }
    }
    pub(super) fn footnote_markers(self) -> &'g [ProductionBodyFootnotePlacedMarker] {
        match self {
            Self::Body(p) => p.footnote_markers(),
            Self::Column(p) => p.footnote_markers(),
        }
    }
    pub(super) fn equation_numbers(
        self,
    ) -> &'g [typaxis_pagination::book_v2::BookV2BodyPlacedEquationNumber] {
        match self {
            Self::Body(p) => p.equation_numbers(),
            Self::Column(p) => p.equation_numbers(),
        }
    }
    pub(super) fn separator_ink(self) -> Option<Rect> {
        match self {
            Self::Body(p) => p.separator_ink(),
            Self::Column(p) => p.separator_ink(),
        }
    }
    pub(super) fn header_graphs(
        self,
    ) -> impl Iterator<Item = BookV2SourceGraph<'b, 'f, 's, 'p, 'a>> + 'g {
        match self {
            Self::Body(p) => Either::Left(p.header_variants().iter().map(|v| v.graph())),
            Self::Column(p) => Either::Right(p.header_variants().map(|v| v.graph())),
        }
    }
}

enum Either<L, R> {
    Left(L),
    Right(R),
}
impl<T, L: Iterator<Item = T>, R: Iterator<Item = T>> Iterator for Either<L, R> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self {
            Self::Left(i) => i.next(),
            Self::Right(i) => i.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Left(i) => i.size_hint(),
            Self::Right(i) => i.size_hint(),
        }
    }
}
impl<T, L: ExactSizeIterator<Item = T>, R: ExactSizeIterator<Item = T>> ExactSizeIterator
    for Either<L, R>
{
}

pub(super) struct Projector<'e, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub(super) source: SourceView<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    pub(super) admitted: &'d AdmittedProductionResourceLedgerV3,
    pub(super) remaining: &'e mut u64,
    pub(super) work: &'e mut u64,
    pub(super) maximum_work: u64,
}
impl<'e, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> Projector<'e, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub(super) fn new(
        source: SourceView<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        admitted: &'d AdmittedProductionResourceLedgerV3,
        remaining: &'e mut u64,
        work: &'e mut u64,
        maximum_work: u64,
    ) -> Self {
        Self {
            source,
            admitted,
            remaining,
            work,
            maximum_work,
        }
    }
    pub(super) fn step(&mut self, owner: NodeId) -> Result<(), BookV2MathDisplayError> {
        *self.work = self
            .work
            .checked_add(1)
            .filter(|n| *n <= self.maximum_work)
            .ok_or(BookV2MathDisplayError::WorkLimit(owner))?;
        Ok(())
    }
    pub(super) fn fold(
        &mut self,
        previous: [u8; 32],
        value: &[u8],
        owner: NodeId,
    ) -> Result<[u8; 32], BookV2MathDisplayError> {
        for _ in 0..(32 + value.len()).div_ceil(64) {
            self.step(owner)?;
        }
        Ok(fold(previous, value))
    }
}
