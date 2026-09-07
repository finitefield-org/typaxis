//! Font-object contribution only. This cannot issue a publication receipt.
use super::*;
use typaxis_core::M4EffectiveResourceLimits;
use typaxis_resources::FrozenPdfCff1PlanV2;

#[derive(Debug)]
pub struct Cff1PdfObjectsV2 {
    bytes: Vec<u8>,
    offsets: [u64; 6],
    first: ObjectId,
    plan_fingerprint: [u8; 32],
}
impl Cff1PdfObjectsV2 {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Offsets relative to this contribution, in the plan's blueprint order.
    pub fn offsets(&self) -> &[u64; 6] {
        &self.offsets
    }
    pub fn first_object(&self) -> ObjectId {
        self.first
    }
    pub fn plan_fingerprint(&self) -> [u8; 32] {
        self.plan_fingerprint
    }
}

/// Serialize the sealed /2 plan directly, without making a /1 font receipt.
/// The document assembler owns reservation, collision checks and publication.
pub fn encode_cff1_pdf_objects_v2(
    plan: &FrozenPdfCff1PlanV2,
    first: ObjectId,
    limits: &M4EffectiveResourceLimits,
) -> Result<Cff1PdfObjectsV2, PdfError> {
    if plan.limits_fingerprint() != limits.fingerprint() {
        return Err(PdfError::ResourcePlanMismatch);
    }
    let last = first
        .get()
        .checked_add(5)
        .ok_or(PdfError::ObjectCountOverflow)?;
    if last > limits.base().get().max_pdf_objects {
        return Err(PdfError::ObjectLimit);
    }
    let [type0, cid, descriptor, program, unicode, cidset] =
        std::array::from_fn(|i| first.get() + i as u32);
    let mut out = LimitedPdfBuffer::new(
        limits
            .base()
            .get()
            .max_output_bytes
            .min(limits.base().get().max_spool_bytes),
    );
    let mut offsets = [0; 6];
    let name = subset_base_font_name(plan.subset().postscript_name())?;
    let begin = |out: &mut LimitedPdfBuffer, id: u32| -> Result<(), PdfError> {
        out.unsigned(u64::from(id))?;
        out.extend(b" 0 obj\n")
    };
    let end = |out: &mut LimitedPdfBuffer| out.extend(b"\nendobj\n");
    begin(&mut out, type0)?;
    out.extend(b"<< /Type /Font /Subtype /Type0 /BaseFont ")?;
    write_pdf_name(&mut out, &name)?;
    out.extend(
        format!(" /Encoding /Identity-H /DescendantFonts [{cid} 0 R] /ToUnicode {unicode} 0 R >>")
            .as_bytes(),
    )?;
    end(&mut out)?;
    offsets[1] = out.len_u64()?;
    begin(&mut out, cid)?;
    out.extend(b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont ")?;
    write_pdf_name(&mut out, &name)?;
    out.extend(format!(" /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {descriptor} 0 R /DW 1000 /W [0 [").as_bytes())?;
    for (i, width) in plan.dense_widths_1000().iter().enumerate() {
        if i != 0 {
            out.push(b' ')?;
        }
        out.unsigned(u64::from(*width))?;
    }
    out.extend(b"]] >>")?;
    end(&mut out)?;
    offsets[2] = out.len_u64()?;
    begin(&mut out, descriptor)?;
    let metrics = plan.subset().metrics();
    out.extend(b"<< /Type /FontDescriptor /FontName ")?;
    write_pdf_name(&mut out, &name)?;
    out.extend(
        format!(
            " /Flags {} /FontBBox [{} {} {} {}] /ItalicAngle ",
            metrics.flags,
            metrics.bbox_1000[0],
            metrics.bbox_1000[1],
            metrics.bbox_1000[2],
            metrics.bbox_1000[3]
        )
        .as_bytes(),
    )?;
    write_pdf_decimal(
        &mut out,
        PdfDecimal::from_fixed_16_16(metrics.italic_angle_fixed_16_16),
    )?;
    out.extend(format!(" /Ascent {} /Descent {} /CapHeight {} /StemV {} /FontFile3 {program} 0 R /CIDSet {cidset} 0 R >>",metrics.ascent_1000,metrics.descent_1000,metrics.cap_height_1000,metrics.stem_v_1000).as_bytes())?;
    end(&mut out)?;
    offsets[3] = out.len_u64()?;
    begin(&mut out, program)?;
    stream(&mut out, b"/Subtype /OpenType ", plan.subset().bytes())?;
    end(&mut out)?;
    offsets[4] = out.len_u64()?;
    begin(&mut out, unicode)?;
    // Stream buffers are bounded before growth; the aggregate output is also
    // bounded. These local limits do not replace cumulative document accounting.
    let cmap = to_unicode_bindings(plan.bindings(), out.remaining()?)?;
    stream(&mut out, b"", &cmap)?;
    end(&mut out)?;
    offsets[5] = out.len_u64()?;
    begin(&mut out, cidset)?;
    let set = dense_cid_set(plan.dense_widths_1000().len(), out.remaining()?)?;
    stream(&mut out, b"", &set)?;
    end(&mut out)?;
    Ok(Cff1PdfObjectsV2 {
        bytes: out.into_bytes(),
        offsets,
        first,
        plan_fingerprint: plan.fingerprint(),
    })
}
fn stream(out: &mut LimitedPdfBuffer, dictionary: &[u8], bytes: &[u8]) -> Result<(), PdfError> {
    out.extend(b"<< ")?;
    out.extend(dictionary)?;
    out.extend(b"/Length ")?;
    out.unsigned(u64::try_from(bytes.len()).map_err(|_| PdfError::OutputTooLarge)?)?;
    out.extend(b" >>\nstream\n")?;
    out.extend(bytes)?;
    out.extend(b"\nendstream")
}

#[cfg(test)]
#[path = "cff_v2_tests.rs"]
mod tests;
