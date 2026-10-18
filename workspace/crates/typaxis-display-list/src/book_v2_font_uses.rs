//! Font usage is issued only from the actual ordered display. These borrowed
//! views authorize selection, not font encoding or PDF marked content.
use super::*;
use typaxis_core::FontInstanceId;
use typaxis_resource_admission::AdmittedProductionFontInstanceV3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2FontUseSource {
    Text(DisplayTextSpan),
    NativeMath {
        owner: NodeId,
        receipt: [u8; 32],
        computation: [u8; 32],
        paint_index: u32,
        logical_ordinal: u32,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2FontUseText<'v> {
    Text(&'v str),
    Scalar(char),
}
#[derive(Clone, Copy, Debug)]
pub enum BookV2FontUseGlyphs<'v> {
    Cluster(&'v [ProductionBodyGlyph]),
    Native(OriginalGlyphId),
}
impl BookV2FontUseGlyphs<'_> {
    pub fn len(&self) -> usize {
        match self {
            Self::Cluster(glyphs) => glyphs.len(),
            Self::Native(_) => 1,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get(&self, index: usize) -> Option<OriginalGlyphId> {
        match self {
            Self::Cluster(glyphs) => glyphs.get(index).map(|g| g.original_gid()),
            Self::Native(glyph) => (index == 0).then_some(*glyph),
        }
    }
}
/// The constructor is private: callers cannot substitute text, glyphs, or a
/// font instance while presenting this as an actual selected display cluster.
pub struct BookV2FontUseData<'v, 'a, P: Copy> {
    paint: P,
    slot: usize,
    instance: AdmittedProductionFontInstanceV3<'a>,
    source: BookV2FontUseSource,
    text: BookV2FontUseText<'v>,
    glyphs: BookV2FontUseGlyphs<'v>,
    size: PositiveLength,
}
impl<'v, 'a, P: Copy> BookV2FontUseData<'v, 'a, P> {
    pub fn paint(&self) -> P {
        self.paint
    }
    pub fn slot(&self) -> usize {
        self.slot
    }
    pub fn instance(&self) -> AdmittedProductionFontInstanceV3<'a> {
        self.instance
    }
    pub fn source(&self) -> BookV2FontUseSource {
        self.source
    }
    pub fn text(&self) -> BookV2FontUseText<'v> {
        self.text
    }
    pub fn glyphs(&self) -> BookV2FontUseGlyphs<'v> {
        self.glyphs
    }
    pub fn size(&self) -> PositiveLength {
        self.size
    }
}
pub type BookV2FontUse<'v, 'a> = BookV2FontUseData<'v, 'a, BookV2BodyPaintIndex>;
/// Column usage retains the actual ordered column paint.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2ColumnFontUse, BookV2FontUse};
/// fn body<'v, 'a>(usage: BookV2ColumnFontUse<'v, 'a>) -> BookV2FontUse<'v, 'a> { usage }
/// ```
pub type BookV2ColumnFontUse<'v, 'a> = BookV2FontUseData<'v, 'a, BookV2ColumnPaintIndex>;

impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn verify_resources(
        &self,
        admitted: &AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookV2MathDisplayError> {
        self.verify_resource_selection(admitted, limits)
    }
    /// The immutable display constructor has checked every actual fragment
    /// flow against this ledger and these limits. Variant glyphs are selected
    /// through their actual font-use views before PDF consumers receive them.
    pub fn verify_resource_selection(
        &self,
        admitted: &AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookV2MathDisplayError> {
        let flow = self.source().source().flow();
        if !std::ptr::eq(self.admitted(), admitted) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch).into());
        }
        flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)
            .map_err(|_| error(NodeId::new(0), E::ReceiptMismatch))?;
        Ok(())
    }
    /// Native rule positions occupy a slot but return no font use. Image and
    /// separator paints have zero slots; invalid paint indices return None.
    pub fn font_slot_count(&self, paint_index: usize) -> Option<usize> {
        use BookV2BodyPaintIndex as P;
        Some(match *self.paints().get(paint_index)? {
            P::Text(_) | P::PageRegionText { .. } => 1,
            P::Marker(index) => self.markers().draws()[index].clusters().len(),
            P::EquationNumber(index) => self.numbers().draws()[index].clusters().len(),
            P::Math(index) => match self.math().draws()[index].paint() {
                BookV2MathPaint::Native(n) => n.paints().len(),
                BookV2MathPaint::Vector(_) => 0,
            },
            P::FootnoteSeparator(_) | P::Image(_) => 0,
        })
    }
    pub fn font_use(
        &self,
        paint_index: usize,
        slot: usize,
    ) -> Result<Option<BookV2FontUse<'_, 'a>>, BookV2MathDisplayError> {
        use BookV2BodyPaintIndex as P;
        let Some(paint) = self.paints().get(paint_index).copied() else {
            return Ok(None);
        };
        if slot >= self.font_slot_count(paint_index).unwrap_or(0) {
            return Ok(None);
        }
        if let P::PageRegionText { region, draw } = paint {
            let d = &self.page_regions()[region].draws()[draw];
            let span = d.source_span();
            return Ok(Some(BookV2FontUse {
                paint,
                slot,
                instance: self
                    .source()
                    .source()
                    .flow()
                    .lines()
                    .prepared()
                    .shaped()
                    .font_instances()
                    .resolve(d.font_instance().font_instance_id())
                    .filter(|i| {
                        i.table_fingerprint() == d.font_instance().table_fingerprint()
                            && std::ptr::eq(i.ledger(), d.font_instance().ledger())
                    })
                    .ok_or_else(|| error(d.owner(), E::ReceiptMismatch))?,
                source: BookV2FontUseSource::Text(
                    DisplayTextSpan::new(
                        DisplayTextBufferId::new(span.text_id().get()),
                        span.start_byte(),
                        span.end_byte(),
                    )
                    .ok_or_else(|| error(d.owner(), E::ReceiptMismatch))?,
                ),
                text: BookV2FontUseText::Text(d.exact_text()),
                glyphs: BookV2FontUseGlyphs::Cluster(d.glyphs()),
                size: d.font().size(),
            }));
        }
        let fragment = match paint {
            P::Text(i) => self.text().draws()[i].fragment_index(),
            P::Marker(i) => self.markers().draws()[i].fragment_index(),
            P::EquationNumber(i) => self.numbers().draws()[i]
                .placement()
                .geometry()
                .fragment_index() as usize,
            P::Math(i) => self.math().draws()[i].terminal().fragment_index(),
            P::Image(_) | P::FootnoteSeparator(_) | P::PageRegionText { .. } => return Ok(None),
        };
        let graph = self
            .source()
            .fragment_graph(fragment)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let original = match paint {
            P::Text(index) => {
                let d = &self.text().draws()[index];
                OriginalFontSlot::Text {
                    id: d.cluster().run().glyph_run().font,
                    font: d.font(),
                    span: d.text_span(),
                    text: d.exact_text(),
                    glyphs: d.glyphs(),
                }
            }
            P::Marker(index) => {
                let d = &self.markers().draws()[index];
                let c = &d.clusters()[slot];
                OriginalFontSlot::Text {
                    id: d.source().glyph_run().font,
                    font: d.source().font(),
                    span: c.text_span(),
                    text: c.exact_text(),
                    glyphs: c.glyphs(),
                }
            }
            P::EquationNumber(index) => {
                let d = &self.numbers().draws()[index];
                let c = &d.clusters()[slot];
                OriginalFontSlot::Text {
                    id: d.shape().font_instance_id(),
                    font: d.font(),
                    span: c.text_span(),
                    text: c.exact_text(),
                    glyphs: c.glyphs(),
                }
            }
            P::Math(index) => {
                let d = &self.math().draws()[index];
                let BookV2MathPaint::Native(n) = d.paint() else {
                    return Ok(None);
                };
                if !matches!(n.paints()[slot], ProductionNativeMathPaint::Glyph { .. }) {
                    return Ok(None);
                }
                let BookV2BodyMathSource::Native(receipt) = d.terminal().source() else {
                    return Err(error(NodeId::new(0), E::ReceiptMismatch).into());
                };
                OriginalFontSlot::Native {
                    receipt,
                    paint: n.paints()[slot],
                }
            }
            _ => return Ok(None),
        };
        issue_original_font_use(paint, slot, graph, self.admitted(), original)
    }
}

enum OriginalFontSlot<'v, 'p> {
    Text {
        id: FontInstanceId,
        font: &'v typaxis_shaping::ProductionBodyFont,
        span: DisplayTextSpan,
        text: &'v str,
        glyphs: &'v [ProductionBodyGlyph],
    },
    Native {
        receipt: &'v typaxis_layout::book_v2::BookV2MathReceipt<'p>,
        paint: ProductionNativeMathPaint,
    },
}

fn issue_original_font_use<'v, 'b, 'f, 's, 'p, 'a, P: Copy>(
    paint: P,
    slot: usize,
    graph: typaxis_pagination::book_v2::BookV2SourceGraph<'b, 'f, 's, 'p, 'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    original: OriginalFontSlot<'v, 'p>,
) -> Result<Option<BookV2FontUseData<'v, 'a, P>>, BookV2MathDisplayError> {
    let prepared = graph.lines().prepared();
    let (instance, source, text, glyphs, size) = match original {
        OriginalFontSlot::Text {
            id,
            font,
            span,
            text,
            glyphs,
        } => {
            let instance = prepared
                .shaped()
                .font_instances()
                .resolve(id)
                .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
            if !std::ptr::eq(instance.ledger(), admitted)
                || instance.font().font_face_id() != font.face_id()
                || instance.font().content_hash() != font.content_hash()
                || instance.font().face_index() != font.face_index()
            {
                return Err(error(NodeId::new(0), E::ReceiptMismatch).into());
            }
            (
                instance,
                BookV2FontUseSource::Text(span),
                BookV2FontUseText::Text(text),
                BookV2FontUseGlyphs::Cluster(glyphs),
                font.size(),
            )
        }
        OriginalFontSlot::Native { receipt, paint } => {
            let ProductionNativeMathPaint::Glyph {
                original_gid,
                unicode,
                logical_ordinal,
                font_size,
                ..
            } = paint
            else {
                return Ok(None);
            };
            let instance = prepared
                .native_math()
                .and_then(|m| m.font_instances().resolve(receipt.font_instance_id()))
                .ok_or_else(|| error(receipt.node_id(), E::ReceiptMismatch))?;
            if !std::ptr::eq(instance.ledger(), admitted)
                || instance.font().font_face_id() != receipt.font_face_id()
                || instance.font().content_hash() != receipt.font_sha256()
                || instance.font().face_index() != receipt.face_index()
            {
                return Err(error(receipt.node_id(), E::ReceiptMismatch).into());
            }
            (
                instance,
                BookV2FontUseSource::NativeMath {
                    owner: receipt.node_id(),
                    receipt: receipt.fingerprint(),
                    computation: receipt.computation().fingerprint(),
                    paint_index: u32::try_from(slot)
                        .map_err(|_| error(receipt.node_id(), E::RecordLimit))?,
                    logical_ordinal,
                },
                BookV2FontUseText::Scalar(unicode),
                BookV2FontUseGlyphs::Native(original_gid),
                font_size,
            )
        }
    };
    Ok(Some(BookV2FontUseData {
        paint,
        slot,
        instance,
        source,
        text,
        glyphs,
        size,
    }))
}

impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn verify_resource_selection(
        &self,
        admitted: &AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookV2MathDisplayError> {
        self.verify_resources(admitted, limits)
    }
    pub fn font_slot_count(&self, paint_index: usize) -> Option<usize> {
        use BookV2ColumnPaintIndex as P;
        Some(match *self.paints().get(paint_index)? {
            P::Text(_) | P::PageRegionText { .. } => 1,
            P::Marker(i) => self.markers().draws()[i].clusters().len(),
            P::EquationNumber(i) => self.numbers().draws()[i].clusters().len(),
            P::Math(i) => match self.math().draws()[i].paint() {
                BookV2MathPaint::Native(n) => n.paints().len(),
                BookV2MathPaint::Vector(_) => 0,
            },
            P::FootnoteSeparator(_) | P::Image(_) => 0,
        })
    }
    pub fn font_use(
        &self,
        paint_index: usize,
        slot: usize,
    ) -> Result<Option<BookV2ColumnFontUse<'_, 'a>>, BookV2MathDisplayError> {
        use BookV2ColumnPaintIndex as P;
        let Some(paint) = self.paints().get(paint_index).copied() else {
            return Ok(None);
        };
        if slot >= self.font_slot_count(paint_index).unwrap_or(0) {
            return Ok(None);
        }
        if let Some((region, draw)) = self.page_region_paint(paint) {
            let d = &self.page_regions()[region].draws()[draw];
            let span = d.source_span();
            return Ok(Some(BookV2ColumnFontUse {
                paint,
                slot,
                instance: self
                    .source()
                    .source()
                    .original_graph()
                    .lines()
                    .prepared()
                    .shaped()
                    .font_instances()
                    .resolve(d.font_instance().font_instance_id())
                    .filter(|i| {
                        i.table_fingerprint() == d.font_instance().table_fingerprint()
                            && std::ptr::eq(i.ledger(), d.font_instance().ledger())
                    })
                    .ok_or_else(|| error(d.owner(), E::ReceiptMismatch))?,
                source: BookV2FontUseSource::Text(
                    DisplayTextSpan::new(
                        DisplayTextBufferId::new(span.text_id().get()),
                        span.start_byte(),
                        span.end_byte(),
                    )
                    .ok_or_else(|| error(d.owner(), E::ReceiptMismatch))?,
                ),
                text: BookV2FontUseText::Text(d.exact_text()),
                glyphs: BookV2FontUseGlyphs::Cluster(d.glyphs()),
                size: d.font().size(),
            }));
        }
        let fragment = match paint {
            P::Text(i) => self.text().draws()[i].fragment_index(),
            P::Marker(i) => self.markers().draws()[i].fragment_index(),
            P::EquationNumber(i) => self.numbers().draws()[i]
                .placement()
                .geometry()
                .fragment_index() as usize,
            P::Math(i) => self.math().draws()[i].terminal().fragment_index(),
            P::Image(_) | P::FootnoteSeparator(_) | P::PageRegionText { .. } => return Ok(None),
        };
        let graph = self
            .source()
            .fragment_graph(fragment)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let original = match paint {
            P::Text(i) => {
                let d = &self.text().draws()[i];
                OriginalFontSlot::Text {
                    id: d.cluster().run().glyph_run().font,
                    font: d.font(),
                    span: d.text_span(),
                    text: d.exact_text(),
                    glyphs: d.glyphs(),
                }
            }
            P::Marker(i) => {
                let d = &self.markers().draws()[i];
                let c = &d.clusters()[slot];
                OriginalFontSlot::Text {
                    id: d.source().glyph_run().font,
                    font: d.source().font(),
                    span: c.text_span(),
                    text: c.exact_text(),
                    glyphs: c.glyphs(),
                }
            }
            P::EquationNumber(i) => {
                let d = &self.numbers().draws()[i];
                let c = &d.clusters()[slot];
                OriginalFontSlot::Text {
                    id: d.shape().font_instance_id(),
                    font: d.font(),
                    span: c.text_span(),
                    text: c.exact_text(),
                    glyphs: c.glyphs(),
                }
            }
            P::Math(i) => {
                let d = &self.math().draws()[i];
                let BookV2MathPaint::Native(n) = d.paint() else {
                    return Ok(None);
                };
                if !matches!(n.paints()[slot], ProductionNativeMathPaint::Glyph { .. }) {
                    return Ok(None);
                }
                let BookV2BodyMathSource::Native(receipt) = d.terminal().source() else {
                    return Err(error(NodeId::new(0), E::ReceiptMismatch).into());
                };
                OriginalFontSlot::Native {
                    receipt,
                    paint: n.paints()[slot],
                }
            }
            _ => return Ok(None),
        };
        issue_original_font_use(paint, slot, graph, self.admitted(), original)
    }
}
