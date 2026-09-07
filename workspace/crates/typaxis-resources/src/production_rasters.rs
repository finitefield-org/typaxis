//! Raster plans derived only from selected figure paints. Payload sharing never
//! merges source occurrences or their structure/Alt bindings.
use super::*;
use std::io::Write;
use typaxis_core::M4EffectiveResourceLimits;
use typaxis_display_list::ProductionBodyDraw;

pub struct ProductionBodyRasterPlans<'f, 'v, 'd, 's, 'p, 'a> {
    fonts: &'f ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a>,
    plans: Vec<FrozenPdfImagePlan>,
    draw_plans: Vec<Option<usize>>,
    record_base: u64,
    spool_base: u64,
    record_charge: u64,
    spool_charge: u64,
    peak_spool_charge: u64,
}
impl<'f, 'v, 'd, 's, 'p, 'a> ProductionBodyRasterPlans<'f, 'v, 'd, 's, 'p, 'a> {
    pub fn plans(&self) -> &[FrozenPdfImagePlan] {
        &self.plans
    }
    pub fn draw_plan(&self, draw: usize) -> Option<usize> {
        self.draw_plans.get(draw).copied().flatten()
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub const fn peak_spool_charge(&self) -> u64 {
        self.peak_spool_charge
    }
    pub fn verify(
        &self,
        fonts: &ProductionBodyFontPlans<'_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
        record_base: u64,
        spool_base: u64,
    ) -> Result<(), ResourceError> {
        if !std::ptr::eq(fonts, self.fonts)
            || self.record_base != record_base
            || self.spool_base != spool_base
        {
            return Err(ResourceError::AdmittedLedgerEpochMismatch);
        }
        fonts.verify(fonts.display(), admitted, limits)
    }
}

pub fn finalize_production_body_rasters<'f, 'v, 'd, 's, 'p, 'a>(
    fonts: &'f ProductionBodyFontPlans<'v, 'd, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    record_base: u64,
    spool_base: u64,
) -> Result<ProductionBodyRasterPlans<'f, 'v, 'd, 's, 'p, 'a>, ResourceError> {
    use ResourceError as E;
    fonts.verify(fonts.display(), admitted, limits)?;
    if record_base < fonts.record_charge() || spool_base < fonts.spool_charge() {
        return Err(E::AdmittedLedgerEpochMismatch);
    }
    let projected = finalize_raster_projection(
        fonts.display().draws(),
        admitted,
        limits,
        record_base,
        spool_base,
    )?;
    Ok(ProductionBodyRasterPlans {
        fonts,
        plans: projected.plans,
        draw_plans: projected.draw_plans,
        record_base: projected.record_base,
        spool_base: projected.spool_base,
        record_charge: projected.record_charge,
        spool_charge: projected.spool_charge,
        peak_spool_charge: projected.peak_spool_charge,
    })
}
struct RasterProjection {
    plans: Vec<FrozenPdfImagePlan>,
    draw_plans: Vec<Option<usize>>,
    record_base: u64,
    spool_base: u64,
    record_charge: u64,
    spool_charge: u64,
    peak_spool_charge: u64,
}
fn finalize_raster_projection(
    draws: &[ProductionBodyDraw<'_>],
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    record_base: u64,
    spool_base: u64,
) -> Result<RasterProjection, ResourceError> {
    use ResourceError as E;
    let maximum = limits.base().get().max_spool_bytes;
    // Include slots, content-hash lookup nodes, temporary plans and per-page
    // resource references before allocating any of these collections.
    let record_charge = draws
        .iter()
        .try_fold(record_base, |n, draw| {
            n.checked_add(if matches!(draw, ProductionBodyDraw::Raster(_)) {
                12
            } else {
                1
            })
        })
        .ok_or(E::ResourceLimit)?;
    if record_charge > limits.base().get().max_fragments || spool_base > maximum {
        return Err(E::ResourceLimit);
    }
    let mut plans: Vec<FrozenPdfImagePlan> = Vec::new();
    let mut by_hash = BTreeMap::new();
    let mut draw_plans = Vec::new();
    draw_plans
        .try_reserve_exact(draws.len())
        .map_err(|_| E::ResourceLimit)?;
    let issuer = VerifiedEncoderReceiptOwner::new();
    let mut spool_charge = spool_base;
    let mut peak_spool_charge = spool_base;
    for draw in draws {
        let ProductionBodyDraw::Raster(raster) = draw else {
            draw_plans.push(None);
            continue;
        };
        let image = raster.image();
        let index = if let Some(&index) = by_hash.get(&image.content_hash()) {
            let prior: &FrozenPdfImagePlan = &plans[index];
            let original = admitted
                .image(prior.image_id())
                .ok_or(E::MissingLogicalResource)?;
            if original.bytes() != image.bytes()
                || original.media_kind() != image.media_kind()
                || prior.width() != image.width()
                || prior.height() != image.height()
            {
                return Err(E::InvalidImagePlan);
            }
            index
        } else {
            let remaining = maximum.checked_sub(spool_charge).ok_or(E::ResourceLimit)?;
            let receipt = match image.media_kind() {
                AdmittedImageMediaKind::Png => {
                    // The decoder has its own explicit allocation ceiling.
                    // Two pixel buffers cover normalization and alpha splitting;
                    // 1 MiB covers the pinned Rust deflater's fixed workspace.
                    let decoder = image
                        .decoded_bytes()
                        .checked_add(image.bytes().len() as u64)
                        .and_then(|n| n.checked_add(65_536))
                        .ok_or(E::ResourceLimit)?;
                    let workspace = image
                        .decoded_bytes()
                        .checked_mul(2)
                        .and_then(|n| n.checked_add(decoder))
                        .and_then(|n| n.checked_add(1_048_576))
                        .ok_or(E::ResourceLimit)?;
                    let capacity = remaining.checked_sub(workspace).ok_or(E::ResourceLimit)? / 2;
                    let mut output = decode_png_bytes_for_pdf_with_workspace(
                        image.image_id(),
                        image.content_hash(),
                        image.bytes(),
                        image.width(),
                        image.height(),
                        image.decoded_bytes(),
                        usize::try_from(decoder).map_err(|_| E::ResourceLimit)?,
                    )?;
                    output.encoded_bytes = compress(&output.encoded_bytes, capacity)?;
                    output.encoding = ImageEncoding::Flate;
                    let mut bytes = output.encoded_bytes.len() as u64;
                    if let Some(mask) = &mut output.alpha_mask {
                        mask.encoded_bytes = compress(
                            &mask.encoded_bytes,
                            capacity.checked_sub(bytes).ok_or(E::ResourceLimit)?,
                        )?;
                        mask.encoding = ImageEncoding::Flate;
                        bytes = bytes
                            .checked_add(mask.encoded_bytes.len() as u64)
                            .ok_or(E::ResourceLimit)?;
                    }
                    // Vec -> Arc may temporarily copy the compressed color.
                    let peak = spool_charge
                        .checked_add(workspace)
                        .and_then(|n| bytes.checked_mul(2).and_then(|v| n.checked_add(v)))
                        .ok_or(E::ResourceLimit)?;
                    if peak > maximum {
                        return Err(E::ResourceLimit);
                    }
                    peak_spool_charge = peak_spool_charge.max(peak);
                    issuer.issue_image(output)
                }
                AdmittedImageMediaKind::JpegBaseline => {
                    let jpeg = image.jpeg_attestation().ok_or(E::InvalidImagePlan)?;
                    if jpeg.normalized_bytes().len() as u64 > remaining {
                        return Err(E::ResourceLimit);
                    }
                    issuer.issue_jpeg(image)?
                }
                _ => return Err(E::InvalidImagePlan),
            };
            let VerifiedEncoderOutput::Image(plan) = receipt.0 else {
                return Err(E::InvalidImagePlan);
            };
            spool_charge = spool_charge
                .checked_add(plan.encoded_bytes().len() as u64)
                .and_then(|n| {
                    n.checked_add(
                        plan.alpha_mask()
                            .map_or(0, |m| m.encoded_bytes().len() as u64),
                    )
                })
                .ok_or(E::ResourceLimit)?;
            if spool_charge > maximum {
                return Err(E::ResourceLimit);
            }
            peak_spool_charge = peak_spool_charge.max(spool_charge);
            let index = plans.len();
            plans.try_reserve(1).map_err(|_| E::ResourceLimit)?;
            plans.push(*plan);
            by_hash.insert(image.content_hash(), index);
            index
        };
        draw_plans.push(Some(index));
    }
    Ok(RasterProjection {
        plans,
        draw_plans,
        record_base,
        spool_base,
        record_charge,
        spool_charge,
        peak_spool_charge,
    })
}

fn compress(bytes: &[u8], maximum: u64) -> Result<Vec<u8>, ResourceError> {
    struct Bounded {
        bytes: Vec<u8>,
        maximum: u64,
    }
    impl Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let length = self
                .bytes
                .len()
                .checked_add(bytes.len())
                .filter(|n| *n as u64 <= self.maximum)
                .ok_or_else(|| std::io::Error::other("raster output limit"))?;
            self.bytes
                .try_reserve_exact(length - self.bytes.len())
                .map_err(|_| std::io::Error::other("raster allocation limit"))?;
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut encoder = flate2::write::ZlibEncoder::new(
        Bounded {
            bytes: Vec::new(),
            maximum,
        },
        flate2::Compression::new(6),
    );
    encoder
        .write_all(bytes)
        .map_err(|_| ResourceError::ResourceLimit)?;
    Ok(encoder
        .finish()
        .map_err(|_| ResourceError::ResourceLimit)?
        .bytes)
}

pub struct ProductionFootnoteRasterPlans<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    fonts: &'e ProductionFootnoteFontPlans<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    plans: Vec<FrozenPdfImagePlan>,
    draw_plans: Vec<Option<usize>>,
    record_base: u64,
    spool_base: u64,
    record_charge: u64,
    spool_charge: u64,
    peak_spool_charge: u64,
}
impl<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    ProductionFootnoteRasterPlans<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn plans(&self) -> &[FrozenPdfImagePlan] {
        &self.plans
    }
    pub fn draw_plan(&self, draw: usize) -> Option<usize> {
        self.draw_plans.get(draw).copied().flatten()
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub const fn peak_spool_charge(&self) -> u64 {
        self.peak_spool_charge
    }
    pub fn verify(
        &self,
        fonts: &ProductionFootnoteFontPlans<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
        record_base: u64,
        spool_base: u64,
    ) -> Result<(), ResourceError> {
        if !std::ptr::eq(fonts, self.fonts)
            || self.record_base != record_base
            || self.spool_base != spool_base
        {
            return Err(ResourceError::AdmittedLedgerEpochMismatch);
        }
        fonts.verify(fonts.structure(), admitted, limits)
    }
}

pub fn finalize_production_footnote_rasters<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    fonts: &'e ProductionFootnoteFontPlans<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    record_base: u64,
    spool_base: u64,
) -> Result<ProductionFootnoteRasterPlans<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, ResourceError>
{
    fonts.verify(fonts.structure(), admitted, limits)?;
    if record_base < fonts.record_charge() || spool_base < fonts.spool_charge() {
        return Err(ResourceError::AdmittedLedgerEpochMismatch);
    }
    let projected = finalize_raster_projection(
        fonts.structure().display().draws(),
        admitted,
        limits,
        record_base,
        spool_base,
    )?;
    Ok(ProductionFootnoteRasterPlans {
        fonts,
        plans: projected.plans,
        draw_plans: projected.draw_plans,
        record_base: projected.record_base,
        spool_base: projected.spool_base,
        record_charge: projected.record_charge,
        spool_charge: projected.spool_charge,
        peak_spool_charge: projected.peak_spool_charge,
    })
}
