//! Actual selected glyph positions and frozen CIDs, in the page's Y-down space.
//! These are text contributions, not a complete or authorized tagged PDF.
use typaxis_core::{FontInstanceId, M4EffectiveResourceLimits};
use typaxis_display_list::ProductionBodyDraw;
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_resources::ProductionBodyFontPlans;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyTextError {
    ReceiptMismatch,
    RecordLimit,
    OutputLimit,
    AllocationFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyTextPaint {
    draw_index: usize,
    page_index: u32,
    font_instance_id: FontInstanceId,
    start: usize,
    end: usize,
}
impl ProductionBodyTextPaint {
    pub const fn draw_index(self) -> usize {
        self.draw_index
    }
    pub const fn page_index(self) -> u32 {
        self.page_index
    }
    /// The page font dictionary must bind /PB{id} to this frozen font.
    pub const fn font_instance_id(self) -> FontInstanceId {
        self.font_instance_id
    }
}

pub struct ProductionBodyTextContribution<'f, 'v, 'd, 's, 'p, 'a> {
    fonts: &'f ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a>,
    paints: Vec<ProductionBodyTextPaint>,
    bytes: Vec<u8>,
    record_charge: u64,
}
impl<'f, 'v, 'd, 's, 'p, 'a> ProductionBodyTextContribution<'f, 'v, 'd, 's, 'p, 'a> {
    pub const fn fonts(&self) -> &'f ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a> {
        self.fonts
    }
    pub fn paints(&self) -> &[ProductionBodyTextPaint] {
        &self.paints
    }
    pub fn paint_bytes(&self, paint_index: usize) -> Option<&[u8]> {
        let p = self.paints.get(paint_index)?;
        Some(&self.bytes[p.start..p.end])
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn verify(
        &self,
        fonts: &ProductionBodyFontPlans<'_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyTextError> {
        if !std::ptr::eq(self.fonts, fonts) {
            return Err(ProductionBodyTextError::ReceiptMismatch);
        }
        fonts
            .verify(fonts.display(), admitted, limits)
            .map_err(|_| ProductionBodyTextError::ReceiptMismatch)
    }
}

pub fn encode_production_body_text<'f, 'v, 'd, 's, 'p, 'a>(
    fonts: &'f ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodyTextContribution<'f, 'v, 'd, 's, 'p, 'a>, ProductionBodyTextError> {
    use ProductionBodyTextError as E;
    fonts
        .verify(fonts.display(), admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    let mut record_charge = fonts.record_charge();
    let maximum = limits
        .base()
        .get()
        .max_output_bytes
        .min(limits.base().get().max_spool_bytes);
    let mut bytes = Vec::new();
    let mut paints = Vec::new();
    for (draw_index, draw) in fonts.display().draws().iter().enumerate() {
        let ProductionBodyDraw::Text(text) = draw else {
            continue;
        };
        let (font, cluster) = fonts.text_plan(draw_index).ok_or(E::ReceiptMismatch)?;
        if cluster.cids().len() != text.glyphs().len() {
            return Err(E::ReceiptMismatch);
        }
        record_charge = record_charge.checked_add(1).ok_or(E::RecordLimit)?;
        if record_charge > limits.base().get().max_fragments {
            return Err(E::RecordLimit);
        }
        paints.try_reserve(1).map_err(|_| E::AllocationFailure)?;
        let start = bytes.len();
        append(&mut bytes, b"q\n", maximum)?;
        if cluster.requires_actual_text() {
            append(&mut bytes, b"/Span << /ActualText <FEFF", maximum)?;
            for unit in cluster.exact_text().encode_utf16() {
                append(&mut bytes, format!("{unit:04X}").as_bytes(), maximum)?;
            }
            append(&mut bytes, b"> >> BDC\n", maximum)?;
        }
        let font_instance_id = font.pdf_font().font_instance_id();
        append(
            &mut bytes,
            format!(
                "0 g\nBT /PB{} {} Tf 0 Tr\n0 Tc 0 Tw 100 Tz 0 TL 0 Ts\n",
                font_instance_id.get(),
                number(text.font_size().get().raw())
            )
            .as_bytes(),
            maximum,
        )?;
        for (glyph, cid) in text.glyphs().iter().zip(cluster.cids()) {
            append(
                &mut bytes,
                format!(
                    "1 0 0 -1 {} {} Tm <{:04X}> Tj\n",
                    number(glyph.x().raw()),
                    number(glyph.y().raw()),
                    cid.get()
                )
                .as_bytes(),
                maximum,
            )?;
        }
        append(&mut bytes, b"ET\n", maximum)?;
        if cluster.requires_actual_text() {
            append(&mut bytes, b"EMC\n", maximum)?;
        }
        append(&mut bytes, b"Q\n", maximum)?;
        paints.push(ProductionBodyTextPaint {
            draw_index,
            page_index: text.page_index(),
            font_instance_id,
            start,
            end: bytes.len(),
        });
    }
    Ok(ProductionBodyTextContribution {
        fonts,
        paints,
        bytes,
        record_charge,
    })
}
fn number(raw: i64) -> String {
    crate::tagged_pdf_v2::pdf_number_v2(raw)
}
fn append(bytes: &mut Vec<u8>, value: &[u8], maximum: u64) -> Result<(), ProductionBodyTextError> {
    let next = bytes
        .len()
        .checked_add(value.len())
        .ok_or(ProductionBodyTextError::OutputLimit)?;
    if next as u64 > maximum {
        return Err(ProductionBodyTextError::OutputLimit);
    }
    bytes
        .try_reserve(value.len())
        .map_err(|_| ProductionBodyTextError::AllocationFailure)?;
    bytes.extend_from_slice(value);
    Ok(())
}
