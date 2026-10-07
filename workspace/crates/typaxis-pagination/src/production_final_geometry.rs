//! Borrowed physical pages for the common downstream pipeline. No copied layout.
use super::*;

#[derive(Clone, Copy)]
pub enum ProductionFinalPageGeometry<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    Ordinary(&'g ProductionBodyFootnotePlacedSequence<'q, 'b, 'f, 's, 'p, 'a>),
    Mixed(&'g ProductionBodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a>),
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> ProductionFinalPageGeometry<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn ordinary(
        &self,
    ) -> Option<&'g ProductionBodyFootnotePlacedSequence<'q, 'b, 'f, 's, 'p, 'a>> {
        match self {
            Self::Ordinary(v) => Some(v),
            Self::Mixed(_) => None,
        }
    }
    pub fn mixed(&self) -> Option<&'g ProductionBodyMixedPlacedSequence<'q, 'b, 'f, 's, 'p, 'a>> {
        match self {
            Self::Mixed(v) => Some(v),
            Self::Ordinary(_) => None,
        }
    }
    pub fn pages(&self) -> ProductionFinalPages<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        ProductionFinalPages { geometry: *self }
    }
}
#[derive(Clone, Copy)]
pub struct ProductionFinalPages<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    geometry: ProductionFinalPageGeometry<'g, 'q, 'b, 'f, 's, 'p, 'a>,
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> ProductionFinalPages<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn len(&self) -> usize {
        match self.geometry {
            ProductionFinalPageGeometry::Ordinary(v) => v.pages().len(),
            ProductionFinalPageGeometry::Mixed(v) => v.pages().len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get(&self, index: usize) -> Option<ProductionFinalPage<'g, 'q, 'b, 'f, 's, 'p, 'a>> {
        match self.geometry {
            ProductionFinalPageGeometry::Ordinary(v) => {
                v.pages().get(index).map(ProductionFinalPage::Ordinary)
            }
            ProductionFinalPageGeometry::Mixed(v) => {
                v.pages().get(index).map(ProductionFinalPage::Mixed)
            }
        }
    }
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = ProductionFinalPage<'g, 'q, 'b, 'f, 's, 'p, 'a>> {
        let pages = *self;
        (0..self.len()).map(move |index| pages.get(index).expect("bound page index"))
    }
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> IntoIterator for ProductionFinalPages<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    type Item = ProductionFinalPage<'g, 'q, 'b, 'f, 's, 'p, 'a>;
    type IntoIter = ProductionFinalPageIter<'g, 'q, 'b, 'f, 's, 'p, 'a>;
    fn into_iter(self) -> Self::IntoIter {
        ProductionFinalPageIter {
            pages: self,
            index: 0,
        }
    }
}
pub struct ProductionFinalPageIter<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pages: ProductionFinalPages<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    index: usize,
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> Iterator for ProductionFinalPageIter<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    type Item = ProductionFinalPage<'g, 'q, 'b, 'f, 's, 'p, 'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.pages.get(self.index)?;
        self.index += 1;
        Some(item)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.pages.len() - self.index;
        (n, Some(n))
    }
}
impl ExactSizeIterator for ProductionFinalPageIter<'_, '_, '_, '_, '_, '_, '_> {}
#[derive(Clone, Copy)]
pub enum ProductionFinalPage<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    Ordinary(&'g ProductionBodyFootnotePlacedPage<'q, 'b, 'f, 's, 'p, 'a>),
    Mixed(&'g ProductionBodyMixedPlacedPage<'q, 'b, 'f, 's, 'p, 'a>),
}
impl<'g, 'q, 'b, 'f, 's, 'p, 'a> ProductionFinalPage<'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn page_index(&self) -> u32 {
        match self {
            Self::Ordinary(p) => p.selection().page_index(),
            Self::Mixed(p) => p.selection().page_index(),
        }
    }
    pub fn fragments(&self) -> &'g [ProductionBodyFootnotePlacedFragment] {
        match self {
            Self::Ordinary(p) => p.fragments(),
            Self::Mixed(p) => p.fragments(),
        }
    }
    pub fn list_markers(&self) -> &'g [ProductionBodyListMarker] {
        match self {
            Self::Ordinary(p) => p.list_markers(),
            Self::Mixed(p) => p.list_markers(),
        }
    }
    pub fn footnote_markers(&self) -> &'g [ProductionBodyFootnotePlacedMarker] {
        match self {
            Self::Ordinary(p) => p.footnote_markers(),
            Self::Mixed(p) => p.footnote_markers(),
        }
    }
    pub fn separator_ink(&self) -> Option<Rect> {
        match self {
            Self::Ordinary(p) => p.separator_ink(),
            Self::Mixed(p) => p.separator_ink(),
        }
    }
    pub fn cell_role(&self, index: usize) -> Option<ProductionTablePlacedCellRole> {
        match self {
            Self::Ordinary(_) => None,
            Self::Mixed(p) => p.cell_roles().get(index).copied().flatten(),
        }
    }
}
