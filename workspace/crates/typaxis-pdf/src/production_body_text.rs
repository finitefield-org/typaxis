//! Actual selected glyph positions and frozen CIDs, in the page's Y-down space.
//! These are text contributions, not a complete or authorized tagged PDF.
use typaxis_core::{FontInstanceId, M4EffectiveResourceLimits};
use typaxis_display_list::ProductionBodyDraw;
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_resources::{
    FrozenStagingPdfTextClusterPlan, FrozenStagingPdfTextFontPlan, ProductionBodyFontPlans,
    ProductionFootnoteFontPlans,
};

#[path = "production_native_math.rs"]
mod native_math;

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
    font_instance_id: Option<FontInstanceId>,
    is_native_math: bool,
    start: usize,
    commands_start: usize,
    commands_end: usize,
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
    pub const fn is_native_math(self) -> bool {
        self.is_native_math
    }
    pub const fn font_instance_id(self) -> Option<FontInstanceId> {
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
    /// The exact retained glyph/graphics commands, without the standalone
    /// q/Q and cluster ActualText wrappers. The marked-content owner supplies
    /// one selected-fragment ActualText scope instead of nesting replacements.
    pub(crate) fn paint_commands(&self, paint_index: usize) -> Option<&[u8]> {
        let p = self.paints.get(paint_index)?;
        Some(&self.bytes[p.commands_start..p.commands_end])
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
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
    let projection = encode_text_projection(
        fonts.display().draws(),
        |i| fonts.text_plan(i),
        |i, p| fonts.native_plan(i, p),
        fonts.record_charge(),
        fonts.spool_charge(),
        limits,
    )?;
    Ok(ProductionBodyTextContribution {
        fonts,
        paints: projection.paints,
        bytes: projection.bytes,
        record_charge: projection.record_charge,
    })
}

struct TextProjection {
    paints: Vec<ProductionBodyTextPaint>,
    bytes: Vec<u8>,
    record_charge: u64,
}

fn encode_text_projection<'c>(
    draws: &[ProductionBodyDraw<'_>],
    text_plan: impl Fn(
        usize,
    ) -> Option<(
        &'c FrozenStagingPdfTextFontPlan,
        &'c FrozenStagingPdfTextClusterPlan,
    )>,
    native_plan: impl Fn(
        usize,
        usize,
    ) -> Option<(
        &'c FrozenStagingPdfTextFontPlan,
        &'c FrozenStagingPdfTextClusterPlan,
    )>,
    prior_records: u64,
    prior_spool: u64,
    limits: &M4EffectiveResourceLimits,
) -> Result<TextProjection, ProductionBodyTextError> {
    use ProductionBodyTextError as E;
    let mut record_charge = prior_records;
    let remaining_spool = limits
        .base()
        .get()
        .max_spool_bytes
        .checked_sub(prior_spool)
        .ok_or(E::OutputLimit)?;
    let maximum = limits.base().get().max_output_bytes.min(remaining_spool);
    let mut bytes = Vec::new();
    let mut paints = Vec::new();
    for (draw_index, draw) in draws.iter().enumerate() {
        if let ProductionBodyDraw::Math(math) = draw {
            record_charge = record_charge.checked_add(1).ok_or(E::RecordLimit)?;
            if record_charge > limits.base().get().max_fragments {
                return Err(E::RecordLimit);
            }
            paints.try_reserve(1).map_err(|_| E::AllocationFailure)?;
            paints.push(native_math::encode_math(
                math,
                draw_index,
                &native_plan,
                &mut bytes,
                maximum,
            )?);
            continue;
        }
        let ProductionBodyDraw::Text(text) = draw else {
            continue;
        };
        let (font, cluster) = text_plan(draw_index).ok_or(E::ReceiptMismatch)?;
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
        let commands_start = bytes.len();
        let mut output = TextSink {
            bytes: &mut bytes,
            maximum,
        };
        crate::text_encoding::begin(
            &mut output,
            font_instance_id.get(),
            text.font_size().get().raw(),
            true,
        )?;
        for (glyph, cid) in text.glyphs().iter().zip(cluster.cids()) {
            crate::text_encoding::glyph(&mut output, glyph.x().raw(), glyph.y().raw(), cid.get())?;
        }
        crate::text_encoding::end(&mut output)?;
        let commands_end = bytes.len();
        if cluster.requires_actual_text() {
            append(&mut bytes, b"EMC\n", maximum)?;
        }
        append(&mut bytes, b"Q\n", maximum)?;
        paints.push(ProductionBodyTextPaint {
            draw_index,
            page_index: text.page_index(),
            font_instance_id: Some(font_instance_id),
            is_native_math: false,
            start,
            commands_start,
            commands_end,
            end: bytes.len(),
        });
    }
    Ok(TextProjection {
        paints,
        bytes,
        record_charge,
    })
}
struct TextSink<'a> {
    bytes: &'a mut Vec<u8>,
    maximum: u64,
}
impl crate::font_encoding::Sink for TextSink<'_> {
    type Error = ProductionBodyTextError;
    fn extend(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        append(self.bytes, bytes, self.maximum)
    }
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

pub struct ProductionFootnoteTextContribution<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    fonts: &'e ProductionFootnoteFontPlans<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    paints: Vec<ProductionBodyTextPaint>,
    bytes: Vec<u8>,
    record_charge: u64,
}
impl<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    ProductionFootnoteTextContribution<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub const fn fonts(
        &self,
    ) -> &'e ProductionFootnoteFontPlans<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.fonts
    }
    pub fn paints(&self) -> &[ProductionBodyTextPaint] {
        &self.paints
    }
    pub fn paint_bytes(&self, paint_index: usize) -> Option<&[u8]> {
        let p = self.paints.get(paint_index)?;
        Some(&self.bytes[p.start..p.end])
    }
    /// The exact retained glyph/graphics commands, without the standalone
    /// q/Q and cluster ActualText wrappers. The marked-content owner supplies
    /// one selected-fragment ActualText scope instead of nesting replacements.
    pub fn paint_commands(&self, paint_index: usize) -> Option<&[u8]> {
        let p = self.paints.get(paint_index)?;
        Some(&self.bytes[p.commands_start..p.commands_end])
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn verify(
        &self,
        fonts: &ProductionFootnoteFontPlans<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyTextError> {
        if !std::ptr::eq(self.fonts, fonts) {
            return Err(ProductionBodyTextError::ReceiptMismatch);
        }
        fonts
            .verify(fonts.structure(), admitted, limits)
            .map_err(|_| ProductionBodyTextError::ReceiptMismatch)
    }
}

pub fn encode_production_footnote_text<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    fonts: &'e ProductionFootnoteFontPlans<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<
    ProductionFootnoteTextContribution<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    ProductionBodyTextError,
> {
    fonts
        .verify(fonts.structure(), admitted, limits)
        .map_err(|_| ProductionBodyTextError::ReceiptMismatch)?;
    let projection = encode_text_projection(
        fonts.structure().display().draws(),
        |i| fonts.text_plan(i),
        |i, p| fonts.native_plan(i, p),
        fonts.record_charge(),
        fonts.spool_charge(),
        limits,
    )?;
    Ok(ProductionFootnoteTextContribution {
        fonts,
        paints: projection.paints,
        bytes: projection.bytes,
        record_charge: projection.record_charge,
    })
}
