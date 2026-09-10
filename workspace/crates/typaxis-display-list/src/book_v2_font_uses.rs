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
pub struct BookV2FontUse<'v, 'a> {
    paint: BookV2BodyPaintIndex,
    slot: usize,
    instance: AdmittedProductionFontInstanceV3<'a>,
    source: BookV2FontUseSource,
    text: BookV2FontUseText<'v>,
    glyphs: BookV2FontUseGlyphs<'v>,
    size: PositiveLength,
}
impl<'v, 'a> BookV2FontUse<'v, 'a> {
    pub fn paint(&self) -> BookV2BodyPaintIndex {
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
            P::Text(_) => 1,
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
        let fragment = match paint {
            P::Text(i) => self.text().draws()[i].fragment_index(),
            P::Marker(i) => self.markers().draws()[i].fragment_index(),
            P::EquationNumber(i) => self.numbers().draws()[i]
                .placement()
                .geometry()
                .fragment_index() as usize,
            P::Math(i) => self.math().draws()[i].terminal().fragment_index(),
            P::Image(_) | P::FootnoteSeparator(_) => return Ok(None),
        };
        let prepared = self
            .source()
            .fragment_flow(fragment)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?
            .lines()
            .prepared();
        let instances = prepared.shaped().font_instances();
        let resolve = |id: FontInstanceId, font: &typaxis_shaping::ProductionBodyFont| {
            let instance = instances
                .resolve(id)
                .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
            if !std::ptr::eq(instance.ledger(), self.admitted())
                || instance.font().font_face_id() != font.face_id()
                || instance.font().content_hash() != font.content_hash()
                || instance.font().face_index() != font.face_index()
            {
                return Err(error(NodeId::new(0), E::ReceiptMismatch));
            }
            Ok(instance)
        };
        let (instance, source, text, glyphs, size) = match paint {
            P::Text(index) => {
                let draw = &self.text().draws()[index];
                (
                    resolve(draw.cluster().run().glyph_run().font, draw.font())?,
                    BookV2FontUseSource::Text(draw.text_span()),
                    BookV2FontUseText::Text(draw.exact_text()),
                    BookV2FontUseGlyphs::Cluster(draw.glyphs()),
                    draw.font().size(),
                )
            }
            P::Marker(index) => {
                let draw = &self.markers().draws()[index];
                let cluster = &draw.clusters()[slot];
                let source = draw.source();
                (
                    resolve(source.glyph_run().font, source.font())?,
                    BookV2FontUseSource::Text(cluster.text_span()),
                    BookV2FontUseText::Text(cluster.exact_text()),
                    BookV2FontUseGlyphs::Cluster(cluster.glyphs()),
                    source.font().size(),
                )
            }
            P::EquationNumber(index) => {
                let draw = &self.numbers().draws()[index];
                let cluster = &draw.clusters()[slot];
                (
                    resolve(draw.shape().font_instance_id(), draw.font())?,
                    BookV2FontUseSource::Text(cluster.text_span()),
                    BookV2FontUseText::Text(cluster.exact_text()),
                    BookV2FontUseGlyphs::Cluster(cluster.glyphs()),
                    draw.font().size(),
                )
            }
            P::Math(index) => {
                let draw = &self.math().draws()[index];
                let BookV2MathPaint::Native(native) = draw.paint() else {
                    return Ok(None);
                };
                let ProductionNativeMathPaint::Glyph {
                    original_gid,
                    unicode,
                    logical_ordinal,
                    font_size,
                    ..
                } = native.paints()[slot]
                else {
                    return Ok(None);
                };
                let BookV2BodyMathSource::Native(receipt) = draw.terminal().source() else {
                    return Err(error(NodeId::new(0), E::ReceiptMismatch).into());
                };
                let instance = prepared
                    .native_math()
                    .and_then(|m| m.font_instances().resolve(receipt.font_instance_id()))
                    .ok_or_else(|| error(receipt.node_id(), E::ReceiptMismatch))?;
                if !std::ptr::eq(instance.ledger(), self.admitted())
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
            _ => return Ok(None),
        };
        Ok(Some(BookV2FontUse {
            paint,
            slot,
            instance,
            source,
            text,
            glyphs,
            size,
        }))
    }
}
