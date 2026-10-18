//! Readonly command geometry from the two authentic physical display owners.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// A readonly view cannot change its physical display's paint family.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2TextPaintView, BookV2BodyPaintIndex, BookV2ColumnPaintIndex};
/// fn body(view: BookV2TextPaintView<BookV2ColumnPaintIndex>) -> BookV2TextPaintView<BookV2BodyPaintIndex> { view }
/// ```
pub struct BookV2TextPaintView<P: Copy> {
    paint: P,
    page: u32,
    slots: usize,
}
impl<P: Copy> BookV2TextPaintView<P> {
    pub fn paint(&self) -> P {
        self.paint
    }
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn slot_count(&self) -> usize {
        self.slots
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2SourceGlyphKind {
    Cluster,
    Native {
        original_gid: OriginalGlyphId,
        x: Length,
        y: Length,
        font_size: PositiveLength,
    },
    Rule(Rect),
}

/// Original slot facts; only a footnote separator has no font-slot index.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2SourceGlyphSlot, BookV2SourceGlyphKind};
/// let forged = BookV2SourceGlyphSlot { font_slot: Some(0), kind: BookV2SourceGlyphKind::Cluster };
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2SourceGlyphSlot {
    font_slot: Option<usize>,
    kind: BookV2SourceGlyphKind,
}
impl BookV2SourceGlyphSlot {
    pub fn font_slot(&self) -> Option<usize> {
        self.font_slot
    }
    pub fn kind(&self) -> BookV2SourceGlyphKind {
        self.kind
    }
}

/// The resource supertrait seals command access to actual Body and Column
/// displays. Views retain their original typed paint and physical coordinates.
pub trait BookV2CommandDisplay<'a>: BookV2ResourceDisplay<'a> {
    fn text_paint(&self, paint: usize) -> Option<BookV2TextPaintView<Self::Paint>>;
    fn text_slot(
        &self,
        paint: usize,
        slot: usize,
    ) -> Result<Option<BookV2SourceGlyphSlot>, BookV2MathDisplayError>;
}

macro_rules! authentic_command_display {
    ($display:ident, $paint:ident $(, $regions:ident)?) => {
        impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2CommandDisplay<'a>
            for $display<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
        {
            fn text_paint(&self, index: usize) -> Option<BookV2TextPaintView<Self::Paint>> {
                use $paint as P;
                let paint = *self.paints().get(index)?;
                let (page, slots) = match paint {
                    P::Text(i) => (self.text().draws()[i].page_index(), 1),
                    $(P::PageRegionText { .. } => {
                        let (region, _) = self.page_region_paint(paint)?;
                        (self.$regions()[region].page_index(), 1)
                    },)?
                    P::Marker(i) => {
                        let draw = &self.markers().draws()[i];
                        (draw.fragment().fragment().page_index(), draw.clusters().len())
                    }
                    P::EquationNumber(i) => {
                        let draw = &self.numbers().draws()[i];
                        (draw.placement().geometry().page_index(), draw.clusters().len())
                    }
                    P::Math(i) => {
                        let draw = &self.math().draws()[i];
                        let slots = match draw.paint() {
                            BookV2MathPaint::Native(native) => native.paints().len(),
                            BookV2MathPaint::Vector(_) => 0,
                        };
                        (draw.terminal().page_index(), slots)
                    }
                    P::FootnoteSeparator(i) => (self.markers().separators()[i].page_index(), 1),
                    P::Image(_) => (0, 0),
                };
                Some(BookV2TextPaintView { paint, page, slots })
            }
            fn text_slot(
                &self,
                index: usize,
                slot: usize,
            ) -> Result<Option<BookV2SourceGlyphSlot>, BookV2MathDisplayError> {
                use $paint as P;
                let Some(view) = self.text_paint(index) else { return Ok(None); };
                if slot >= view.slot_count() { return Ok(None); }
                let mut font_slot = Some(slot);
                let kind = match view.paint() {
                    P::FootnoteSeparator(i) => {
                        font_slot = None;
                        BookV2SourceGlyphKind::Rule(self.markers().separators()[i].ink())
                    }
                    P::Math(i) => {
                        let BookV2MathPaint::Native(native) = self.math().draws()[i].paint() else {
                            return Err(error(NodeId::new(0), E::ReceiptMismatch).into());
                        };
                        match native.paints()[slot] {
                            ProductionNativeMathPaint::Rule(rect) => BookV2SourceGlyphKind::Rule(rect),
                            ProductionNativeMathPaint::Glyph { original_gid, x, y, font_size, .. } => {
                                BookV2SourceGlyphKind::Native { original_gid, x, y, font_size }
                            }
                        }
                    }
                    _ => BookV2SourceGlyphKind::Cluster,
                };
                Ok(Some(BookV2SourceGlyphSlot { font_slot, kind }))
            }
        }
    };
}
authentic_command_display!(BookV2BodyDisplay, BookV2BodyPaintIndex, page_regions);
authentic_command_display!(BookV2ColumnDisplay, BookV2ColumnPaintIndex, page_regions);
