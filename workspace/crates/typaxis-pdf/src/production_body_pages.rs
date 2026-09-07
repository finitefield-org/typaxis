//! One source-ordered page content stream for selected body text and vectors.
//! This stage exposes exact per-draw ranges for the final structure owner to
//! enclose with marked content; it is not itself a complete tagged PDF.
use crate::{
    encode_production_body_text, encode_production_footnote_text, ProductionBodyTextContribution,
    ProductionBodyTextError, ProductionFootnoteTextContribution,
    StagingSafeVectorPdfContributionV2, StagingSafeVectorPdfV2Error,
};
use typaxis_core::M4EffectiveResourceLimits;
use typaxis_display_list::ProductionBodyDraw;
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_resources::{
    finalize_production_body_rasters, finalize_production_body_vectors,
    finalize_production_footnote_rasters, finalize_production_footnote_vectors,
    ProductionBodyFontPlans, ProductionBodyRasterPlans, ProductionBodyVectorPlans,
    ProductionFootnoteFontPlans, ProductionFootnoteRasterPlans, ProductionFootnoteVectorPlans,
    ResourceError, StagingSafeVectorResourceV2Error,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyPageError {
    ReceiptMismatch,
    OutputLimit,
    AllocationFailure,
    Forms(StagingSafeVectorResourceV2Error),
    Vectors(StagingSafeVectorPdfV2Error),
    Text(ProductionBodyTextError),
    Rasters(ResourceError),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyPageDrawSource {
    Text { paint_index: usize },
    Vector { usage_index: usize },
    Raster { plan_index: usize },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyPageDraw {
    draw_index: usize,
    source: ProductionBodyPageDrawSource,
    start: usize,
    end: usize,
}
impl ProductionBodyPageDraw {
    pub const fn draw_index(self) -> usize {
        self.draw_index
    }
    pub const fn source(self) -> ProductionBodyPageDrawSource {
        self.source
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyPageArtifact {
    before_draw: usize,
    start: usize,
    end: usize,
}
impl ProductionBodyPageArtifact {
    pub const fn before_draw(self) -> usize {
        self.before_draw
    }
}
pub struct ProductionBodyPage {
    page_index: u32,
    draws: Vec<ProductionBodyPageDraw>,
    artifacts: Vec<ProductionBodyPageArtifact>,
    content: Vec<u8>,
}
impl ProductionBodyPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn draws(&self) -> &[ProductionBodyPageDraw] {
        &self.draws
    }
    pub fn artifacts(&self) -> &[ProductionBodyPageArtifact] {
        &self.artifacts
    }
    pub fn artifact_content(&self, ordinal: usize) -> Option<&[u8]> {
        let artifact = self.artifacts.get(ordinal)?;
        Some(&self.content[artifact.start..artifact.end])
    }
    /// Includes exactly one page root Y flip. The final writer must not add
    /// another one when consuming this stream.
    pub fn content(&self) -> &[u8] {
        &self.content
    }
    pub fn draw_content(&self, ordinal: usize) -> Option<&[u8]> {
        let draw = self.draws.get(ordinal)?;
        Some(&self.content[draw.start..draw.end])
    }
}
pub struct ProductionBodyPageContent<'f, 'v, 'd, 's, 'p, 'a> {
    plans: ProductionBodyVectorPlans<'f, 'v, 'd, 's, 'p, 'a>,
    text: ProductionBodyTextContribution<'f, 'v, 'd, 's, 'p, 'a>,
    vectors: StagingSafeVectorPdfContributionV2,
    rasters: ProductionBodyRasterPlans<'f, 'v, 'd, 's, 'p, 'a>,
    raster_spool_base: u64,
    pages: Vec<ProductionBodyPage>,
    spool_charge: u64,
}
impl<'f, 'v, 'd, 's, 'p, 'a> ProductionBodyPageContent<'f, 'v, 'd, 's, 'p, 'a> {
    pub const fn plans(&self) -> &ProductionBodyVectorPlans<'f, 'v, 'd, 's, 'p, 'a> {
        &self.plans
    }
    pub const fn text(&self) -> &ProductionBodyTextContribution<'f, 'v, 'd, 's, 'p, 'a> {
        &self.text
    }
    pub const fn vectors(&self) -> &StagingSafeVectorPdfContributionV2 {
        &self.vectors
    }
    pub fn pages(&self) -> &[ProductionBodyPage] {
        &self.pages
    }
    pub const fn rasters(&self) -> &ProductionBodyRasterPlans<'f, 'v, 'd, 's, 'p, 'a> {
        &self.rasters
    }
    pub const fn record_charge(&self) -> u64 {
        self.rasters.record_charge()
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub fn verify(
        &self,
        fonts: &ProductionBodyFontPlans<'_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyPageError> {
        self.plans
            .verify(fonts, admitted, limits)
            .map_err(|_| ProductionBodyPageError::ReceiptMismatch)?;
        self.text
            .verify(fonts, admitted, limits)
            .map_err(|_| ProductionBodyPageError::ReceiptMismatch)?;
        self.rasters
            .verify(
                fonts,
                admitted,
                limits,
                self.plans.record_charge(),
                self.raster_spool_base,
            )
            .map_err(|_| ProductionBodyPageError::ReceiptMismatch)
    }
}

pub fn build_production_body_page_content<'f, 'v, 'd, 's, 'p, 'a>(
    fonts: &'f ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodyPageContent<'f, 'v, 'd, 's, 'p, 'a>, ProductionBodyPageError> {
    use ProductionBodyPageError as E;
    // The plan charges text/vector order and temporary PDF contribution records
    // together, before the respective encoders allocate their collections.
    let plans = finalize_production_body_vectors(fonts, admitted, limits).map_err(E::Forms)?;
    let text = encode_production_body_text(fonts, admitted, limits).map_err(E::Text)?;
    let maximum = limits.base().get().max_spool_bytes;
    let mut spool_charge = fonts
        .spool_charge()
        .checked_add(text.byte_length())
        .ok_or(E::OutputLimit)?;
    let remaining = maximum.checked_sub(spool_charge).ok_or(E::OutputLimit)?;
    let vectors = crate::safe_vector_v2::build_production_body_vector_contribution(
        &plans, admitted, limits, remaining,
    )
    .map_err(E::Vectors)?;
    spool_charge = spool_charge
        .checked_add(vectors.spool_bytes())
        .ok_or(E::OutputLimit)?;
    let raster_spool_base = spool_charge;
    let rasters = finalize_production_body_rasters(
        fonts,
        admitted,
        limits,
        plans.record_charge(),
        raster_spool_base,
    )
    .map_err(E::Rasters)?;
    spool_charge = rasters.spool_charge();
    let display = fonts.display();
    let projected = build_page_projection(
        display.draws(),
        display.selected().pages().len(),
        display.selected().page_geometry().page_height().get().raw(),
        text.paints(),
        |i| text.paint_bytes(i),
        &vectors,
        |i| rasters.draw_plan(i),
        &[],
        limits,
        spool_charge,
    )?;
    let pages = projected.pages;
    spool_charge = projected.spool_charge;
    Ok(ProductionBodyPageContent {
        plans,
        text,
        vectors,
        rasters,
        raster_spool_base,
        pages,
        spool_charge,
    })
}

struct PageProjection {
    pages: Vec<ProductionBodyPage>,
    spool_charge: u64,
}
fn build_page_projection<'c>(
    draws: &[ProductionBodyDraw<'_>],
    page_count: usize,
    page_height: i64,
    text_paints: &[crate::ProductionBodyTextPaint],
    text_bytes: impl Fn(usize) -> Option<&'c [u8]>,
    vectors: &StagingSafeVectorPdfContributionV2,
    raster_plan: impl Fn(usize) -> Option<usize>,
    separators: &[typaxis_display_list::ProductionFootnoteSeparatorDraw],
    limits: &M4EffectiveResourceLimits,
    mut spool_charge: u64,
) -> Result<PageProjection, ProductionBodyPageError> {
    use ProductionBodyPageError as E;
    let maximum = limits.base().get().max_spool_bytes;
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(page_count)
        .map_err(|_| E::AllocationFailure)?;
    let root = format!(
        "q\n1 0 0 -1 0 {} cm\n",
        crate::tagged_pdf_v2::pdf_number_v2(page_height)
    );
    let mut draw_index = 0usize;
    let mut text_index = 0usize;
    let mut vector_index = 0usize;
    let mut output_bytes = 0u64;
    let mut separator_index = 0usize;
    for page_index in 0..page_count {
        let mut page = ProductionBodyPage {
            page_index: u32::try_from(page_index).map_err(|_| E::OutputLimit)?,
            draws: Vec::new(),
            artifacts: Vec::new(),
            content: Vec::new(),
        };
        let mut append = |out: &mut Vec<u8>, bytes: &[u8]| -> Result<(), E> {
            spool_charge = spool_charge
                .checked_add(bytes.len() as u64)
                .ok_or(E::OutputLimit)?;
            output_bytes = output_bytes
                .checked_add(bytes.len() as u64)
                .ok_or(E::OutputLimit)?;
            if spool_charge > maximum || output_bytes > limits.base().get().max_output_bytes {
                return Err(E::OutputLimit);
            }
            out.try_reserve(bytes.len())
                .map_err(|_| E::AllocationFailure)?;
            out.extend_from_slice(bytes);
            Ok(())
        };
        append(&mut page.content, root.as_bytes())?;
        while let Some(draw) = draws.get(draw_index) {
            let draw_page = match draw {
                ProductionBodyDraw::Text(t) => t.page_index(),
                ProductionBodyDraw::Vector(v) => v.page_index(),
                ProductionBodyDraw::Raster(r) => r.page_index(),
            };
            if draw_page > page.page_index {
                break;
            }
            if draw_page != page.page_index {
                return Err(E::ReceiptMismatch);
            }
            if let Some(separator) = separators.get(separator_index) {
                if separator.before_draw() == draw_index {
                    if separator.page_index() != page.page_index {
                        return Err(E::ReceiptMismatch);
                    }
                    let ink = separator.ink();
                    let n = crate::tagged_pdf_v2::pdf_number_v2;
                    let end_x = ink
                        .x()
                        .checked_add(ink.width().get())
                        .ok_or(E::OutputLimit)?;
                    let center_y = ink
                        .y()
                        .raw()
                        .checked_add(ink.height().get().raw() / 2)
                        .ok_or(E::OutputLimit)?;
                    let bytes = format!(
                        "/Artifact BMC\nq\n0 G\n{} w 0 J [] 0 d\n{} {} m {} {} l S\nQ\nEMC\n",
                        n(ink.height().get().raw()),
                        n(ink.x().raw()),
                        n(center_y),
                        n(end_x.raw()),
                        n(center_y)
                    );
                    page.artifacts
                        .try_reserve(1)
                        .map_err(|_| E::AllocationFailure)?;
                    let start = page.content.len();
                    append(&mut page.content, bytes.as_bytes())?;
                    page.artifacts.push(ProductionBodyPageArtifact {
                        before_draw: draw_index,
                        start,
                        end: page.content.len(),
                    });
                    separator_index += 1;
                }
            }
            let raster_bytes;
            let (source, bytes) = match draw {
                ProductionBodyDraw::Text(_) => {
                    let paint = text_paints.get(text_index).ok_or(E::ReceiptMismatch)?;
                    if paint.draw_index() != draw_index || paint.page_index() != page.page_index {
                        return Err(E::ReceiptMismatch);
                    }
                    let bytes = text_bytes(text_index).ok_or(E::ReceiptMismatch)?;
                    let source = ProductionBodyPageDrawSource::Text {
                        paint_index: text_index,
                    };
                    text_index += 1;
                    (source, bytes)
                }
                ProductionBodyDraw::Vector(v) => {
                    let usage = vectors
                        .usages()
                        .get(vector_index)
                        .ok_or(E::ReceiptMismatch)?;
                    if usage.paint_ordinal() as usize != draw_index
                        || usage.page_index() != page.page_index
                        || usage.semantic_hook().display_command_fingerprint() != v.fingerprint()
                    {
                        return Err(E::ReceiptMismatch);
                    }
                    let source = ProductionBodyPageDrawSource::Vector {
                        usage_index: vector_index,
                    };
                    vector_index += 1;
                    (source, usage.content())
                }
                ProductionBodyDraw::Raster(r) => {
                    let plan_index = raster_plan(draw_index).ok_or(E::ReceiptMismatch)?;
                    let viewport = r.viewport();
                    let bottom = viewport
                        .y()
                        .checked_add(viewport.height().get())
                        .ok_or(E::OutputLimit)?;
                    let n = crate::tagged_pdf_v2::pdf_number_v2;
                    raster_bytes = format!(
                        "q\n{} 0 0 -{} {} {} cm\n/PBR{plan_index} Do\nQ\n",
                        n(viewport.width().get().raw()),
                        n(viewport.height().get().raw()),
                        n(viewport.x().raw()),
                        n(bottom.raw())
                    );
                    (
                        ProductionBodyPageDrawSource::Raster { plan_index },
                        raster_bytes.as_bytes(),
                    )
                }
            };
            page.draws
                .try_reserve(1)
                .map_err(|_| E::AllocationFailure)?;
            let start = page.content.len();
            append(&mut page.content, bytes)?;
            let end = page.content.len();
            append(&mut page.content, b"\n")?;
            page.draws.push(ProductionBodyPageDraw {
                draw_index,
                source,
                start,
                end,
            });
            draw_index += 1;
        }
        append(&mut page.content, b"Q\n")?;
        pages.push(page);
    }
    if separator_index != separators.len()
        || draw_index != draws.len()
        || text_index != text_paints.len()
        || vector_index != vectors.usages().len()
    {
        return Err(E::ReceiptMismatch);
    }
    Ok(PageProjection {
        pages,
        spool_charge,
    })
}

pub struct ProductionFootnotePageContent<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    plans: ProductionFootnoteVectorPlans<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    text: ProductionFootnoteTextContribution<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    vectors: StagingSafeVectorPdfContributionV2,
    rasters: ProductionFootnoteRasterPlans<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    raster_spool_base: u64,
    pages: Vec<ProductionBodyPage>,
    spool_charge: u64,
}
impl<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    ProductionFootnotePageContent<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub const fn plans(
        &self,
    ) -> &ProductionFootnoteVectorPlans<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.plans
    }
    pub const fn text(
        &self,
    ) -> &ProductionFootnoteTextContribution<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.text
    }
    pub const fn vectors(&self) -> &StagingSafeVectorPdfContributionV2 {
        &self.vectors
    }
    pub fn pages(&self) -> &[ProductionBodyPage] {
        &self.pages
    }
    pub const fn rasters(
        &self,
    ) -> &ProductionFootnoteRasterPlans<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.rasters
    }
    pub const fn record_charge(&self) -> u64 {
        self.rasters.record_charge()
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub fn verify(
        &self,
        fonts: &ProductionFootnoteFontPlans<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyPageError> {
        self.plans
            .verify(fonts, admitted, limits)
            .map_err(|_| ProductionBodyPageError::ReceiptMismatch)?;
        self.text
            .verify(fonts, admitted, limits)
            .map_err(|_| ProductionBodyPageError::ReceiptMismatch)?;
        self.rasters
            .verify(
                fonts,
                admitted,
                limits,
                self.plans.record_charge(),
                self.raster_spool_base,
            )
            .map_err(|_| ProductionBodyPageError::ReceiptMismatch)
    }
}

pub fn build_production_footnote_page_content<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    fonts: &'e ProductionFootnoteFontPlans<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<
    ProductionFootnotePageContent<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    ProductionBodyPageError,
> {
    use ProductionBodyPageError as E;
    let plans = finalize_production_footnote_vectors(fonts, admitted, limits).map_err(E::Forms)?;
    let text = encode_production_footnote_text(fonts, admitted, limits).map_err(E::Text)?;
    let vectors =
        crate::build_production_footnote_vector_contribution(&plans, &text, admitted, limits)
            .map_err(E::Vectors)?;
    let raster_spool_base = fonts
        .spool_charge()
        .checked_add(text.byte_length())
        .and_then(|n| n.checked_add(vectors.spool_bytes()))
        .ok_or(E::OutputLimit)?;
    let rasters = finalize_production_footnote_rasters(
        fonts,
        admitted,
        limits,
        plans.record_charge(),
        raster_spool_base,
    )
    .map_err(E::Rasters)?;
    let display = fonts.structure().display();
    let projected = build_page_projection(
        display.draws(),
        display.source().geometry().pages().len(),
        display
            .source()
            .block_layout()
            .page_geometry()
            .page_height()
            .get()
            .raw(),
        text.paints(),
        |i| text.paint_bytes(i),
        &vectors,
        |i| rasters.draw_plan(i),
        display.separators(),
        limits,
        rasters.spool_charge(),
    )?;
    Ok(ProductionFootnotePageContent {
        plans,
        text,
        vectors,
        rasters,
        raster_spool_base,
        pages: projected.pages,
        spool_charge: projected.spool_charge,
    })
}
