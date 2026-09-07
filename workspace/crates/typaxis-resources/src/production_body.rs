//! Font finalization authorized by selected body glyphs, never by caller-made
//! glyph lists. The PDF structure and paint authorization are separate stages.
use std::collections::BTreeMap;

use typaxis_core::M4EffectiveResourceLimits;
use typaxis_display_list::{ProductionBodyDisplay, ProductionBodyDraw};
use typaxis_resource_admission::AdmittedResourceLedger;

use crate::{
    FrozenStagingPdfTextClusterPlan, FrozenStagingPdfTextFontPlan, ResourceError,
    StagingPdfTextClusterUsage,
};

pub const PRODUCTION_BODY_FONTS_ALGORITHM: &str = "typaxis.production-body-fonts/1";

pub struct ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a> {
    display: &'v ProductionBodyDisplay<'d, 's, 'p, 'a>,
    fonts: Vec<FrozenStagingPdfTextFontPlan>,
    // One slot per draw. Vector draws deliberately have no font usage.
    draw_clusters: Vec<Option<(usize, usize)>>,
    record_charge: u64,
    spool_charge: u64,
}
impl<'v, 'd, 's, 'p, 'a> ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a> {
    pub fn fonts(&self) -> &[FrozenStagingPdfTextFontPlan] {
        &self.fonts
    }
    pub const fn display(&self) -> &'v ProductionBodyDisplay<'d, 's, 'p, 'a> {
        self.display
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn text_plan(
        &self,
        draw_index: usize,
    ) -> Option<(
        &FrozenStagingPdfTextFontPlan,
        &FrozenStagingPdfTextClusterPlan,
    )> {
        let (font, cluster) = self.draw_clusters.get(draw_index).copied().flatten()?;
        let font = &self.fonts[font];
        Some((font, &font.clusters()[cluster]))
    }
    pub fn verify(
        &self,
        display: &ProductionBodyDisplay<'_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ResourceError> {
        if !std::ptr::eq(self.display, display) {
            return Err(ResourceError::AdmittedLedgerEpochMismatch);
        }
        display
            .verify_resources(admitted, limits)
            .map_err(|_| ResourceError::AdmittedLedgerEpochMismatch)
    }
}

pub fn finalize_production_body_fonts<'v, 'd, 's, 'p, 'a>(
    display: &'v ProductionBodyDisplay<'d, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a>, ResourceError> {
    display
        .verify_resources(admitted, limits)
        .map_err(|_| ResourceError::AdmittedLedgerEpochMismatch)?;
    let projection = finalize_font_projection(
        display.draws(),
        display.record_charge(),
        display.selected().spool_charge(),
        admitted,
        limits,
    )?;
    Ok(ProductionBodyFontPlans {
        display,
        fonts: projection.fonts,
        draw_clusters: projection.draw_clusters,
        record_charge: projection.record_charge,
        spool_charge: projection.spool_charge,
    })
}

struct FontProjection {
    fonts: Vec<FrozenStagingPdfTextFontPlan>,
    draw_clusters: Vec<Option<(usize, usize)>>,
    record_charge: u64,
    spool_charge: u64,
}
fn finalize_font_projection(
    draws: &[ProductionBodyDraw<'_>],
    prior_records: u64,
    prior_spool: u64,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<FontProjection, ResourceError> {
    // Count all occurrences before copying text or allocating usage maps. The
    // conservative charge includes temporary glyph/CID/extraction copies.
    let mut record_charge = prior_records;
    let mut copied_bytes = prior_spool;
    if copied_bytes > limits.base().get().max_spool_bytes {
        return Err(ResourceError::ResourceLimit);
    }
    let mut text_count = 0usize;
    for draw in draws {
        record_charge = record_charge
            .checked_add(1)
            .ok_or(ResourceError::ResourceLimit)?;
        if let ProductionBodyDraw::Text(text) = draw {
            text_count = text_count
                .checked_add(1)
                .ok_or(ResourceError::ResourceLimit)?;
            let glyph_records = (text.glyphs().len() as u64)
                .checked_mul(12)
                .and_then(|n| n.checked_add(6))
                .ok_or(ResourceError::ResourceLimit)?;
            record_charge = record_charge
                .checked_add(glyph_records)
                .ok_or(ResourceError::ResourceLimit)?;
            // UTF-8 strings, Unicode scalar buffers and temporary extraction.
            copied_bytes = (text.exact_text().len() as u64)
                .checked_mul(16)
                .and_then(|n| copied_bytes.checked_add(n))
                .ok_or(ResourceError::ResourceLimit)?;
        }
        if record_charge > limits.base().get().max_fragments
            || copied_bytes > limits.base().get().max_spool_bytes
        {
            return Err(ResourceError::ResourceLimit);
        }
    }
    let mut usages = Vec::new();
    usages
        .try_reserve_exact(text_count)
        .map_err(|_| ResourceError::ResourceLimit)?;
    for draw in draws {
        if let ProductionBodyDraw::Text(text) = draw {
            usages.push(StagingPdfTextClusterUsage::new(
                text.font_face_id(),
                text.text_span(),
                text.exact_text().to_owned(),
                text.glyphs().iter().map(|g| g.original_gid()).collect(),
            )?);
        }
    }
    let fonts =
        crate::staging_text::finalize_production_text_fonts(admitted, &usages, limits.base())?;
    for font in &fonts {
        if font.pdf_font().subset_bytes().len() as u64
            > limits.extension().get().max_font_subset_bytes
        {
            return Err(ResourceError::ResourceLimit);
        }
        copied_bytes = copied_bytes
            .checked_add(font.pdf_font().subset_bytes().len() as u64)
            .ok_or(ResourceError::ResourceLimit)?;
        if copied_bytes > limits.base().get().max_spool_bytes {
            return Err(ResourceError::ResourceLimit);
        }
    }
    let face_indices = fonts
        .iter()
        .enumerate()
        .map(|(i, f)| (f.font_face_id(), i))
        .collect::<BTreeMap<_, _>>();
    let mut draw_clusters = Vec::new();
    draw_clusters
        .try_reserve_exact(draws.len())
        .map_err(|_| ResourceError::ResourceLimit)?;
    let mut usage_iter = usages.iter();
    for draw in draws {
        draw_clusters.push(if matches!(draw, ProductionBodyDraw::Text(_)) {
            let usage = usage_iter
                .next()
                .ok_or(ResourceError::IncompleteUsagePlan)?;
            let index = *face_indices
                .get(&usage.font_face_id())
                .ok_or(ResourceError::MissingLogicalResource)?;
            let cluster = fonts[index]
                .clusters()
                .binary_search_by(|c| {
                    c.text_span()
                        .cmp(&usage.text_span())
                        .then_with(|| c.exact_text().cmp(usage.exact_text()))
                        .then_with(|| c.glyphs().cmp(usage.glyphs()))
                })
                .map_err(|_| ResourceError::IncompleteUsagePlan)?;
            Some((index, cluster))
        } else {
            None
        });
    }
    Ok(FontProjection {
        fonts,
        draw_clusters,
        record_charge,
        spool_charge: copied_bytes,
    })
}

/// Shared font usage for the actual tagged joint display, including generated
/// footnote/reference/list labels. Separators deliberately have no font usage.
pub struct ProductionFootnoteFontPlans<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    structure:
        &'t typaxis_display_list::ProductionFootnoteStructure<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    projection: FontProjection,
}
impl<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    ProductionFootnoteFontPlans<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn structure(
        &self,
    ) -> &'t typaxis_display_list::ProductionFootnoteStructure<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    {
        self.structure
    }
    pub fn fonts(&self) -> &[FrozenStagingPdfTextFontPlan] {
        &self.projection.fonts
    }
    pub fn record_charge(&self) -> u64 {
        self.projection.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.projection.spool_charge
    }
    pub fn text_plan(
        &self,
        draw_index: usize,
    ) -> Option<(
        &FrozenStagingPdfTextFontPlan,
        &FrozenStagingPdfTextClusterPlan,
    )> {
        let (font, cluster) = self
            .projection
            .draw_clusters
            .get(draw_index)
            .copied()
            .flatten()?;
        let font = &self.projection.fonts[font];
        Some((font, &font.clusters()[cluster]))
    }
    pub fn verify(
        &self,
        structure: &typaxis_display_list::ProductionFootnoteStructure<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ResourceError> {
        if !std::ptr::eq(self.structure, structure) {
            return Err(ResourceError::AdmittedLedgerEpochMismatch);
        }
        structure
            .verify(structure.display(), admitted, limits)
            .map_err(|_| ResourceError::AdmittedLedgerEpochMismatch)
    }
}
pub fn finalize_production_footnote_fonts<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    structure: &'t typaxis_display_list::ProductionFootnoteStructure<
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionFootnoteFontPlans<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, ResourceError> {
    structure
        .verify(structure.display(), admitted, limits)
        .map_err(|_| ResourceError::AdmittedLedgerEpochMismatch)?;
    let prior_spool = structure
        .spool_charge()
        .checked_add(structure.display().source().spool_bytes())
        .ok_or(ResourceError::ResourceLimit)?;
    let projection = finalize_font_projection(
        structure.display().draws(),
        structure.record_charge(),
        prior_spool,
        admitted,
        limits,
    )?;
    Ok(ProductionFootnoteFontPlans {
        structure,
        projection,
    })
}
