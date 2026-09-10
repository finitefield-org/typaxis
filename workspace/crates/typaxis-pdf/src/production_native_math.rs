//! CID-backed native glyphs and rules in common page coordinates.
use super::*;

pub(super) fn encode_math<'c>(
    math: &typaxis_display_list::ProductionBodyNativeMathDraw<'_>,
    draw_index: usize,
    plan: &impl Fn(
        usize,
        usize,
    ) -> Option<(
        &'c FrozenStagingPdfTextFontPlan,
        &'c FrozenStagingPdfTextClusterPlan,
    )>,
    bytes: &mut Vec<u8>,
    maximum: u64,
) -> Result<ProductionBodyTextPaint, ProductionBodyTextError> {
    use ProductionBodyTextError as E;
    let start = bytes.len();
    append(bytes, b"q\n", maximum)?;
    let commands_start = bytes.len();
    append(bytes, b"0 g\n", maximum)?;
    let mut font_instance_id = None;
    for (index, paint) in math.paints().iter().enumerate() {
        match paint {
            typaxis_display_list::ProductionNativeMathPaint::Glyph {
                original_gid,
                unicode,
                logical_ordinal,
                x,
                y,
                font_size,
            } => {
                let (font, cluster) = plan(draw_index, index).ok_or(E::ReceiptMismatch)?;
                let typaxis_resources::PdfFontClusterSource::NativeMathGlyph(source) =
                    cluster.source()
                else {
                    return Err(E::ReceiptMismatch);
                };
                if font.font_face_id() != math.receipt().font_face_id()
                    || source.owner() != math.owner()
                    || source.receipt_sha256() != math.receipt().key().bytes()
                    || source.computation_sha256() != math.receipt().computation().fingerprint()
                    || source.paint_index() as usize != index
                    || source.logical_ordinal() != *logical_ordinal
                    || cluster.glyphs() != &[*original_gid]
                    || !cluster.exact_text().chars().eq(std::iter::once(*unicode))
                    || cluster.cids().len() != 1
                {
                    return Err(E::ReceiptMismatch);
                }
                let id = font.pdf_font().font_instance_id();
                if font_instance_id.is_some_and(|old| old != id) {
                    return Err(E::ReceiptMismatch);
                }
                font_instance_id = Some(id);
                let mut output = TextSink { bytes, maximum };
                crate::text_encoding::begin(&mut output, id.get(), font_size.get().raw(), false)?;
                crate::text_encoding::glyph(
                    &mut output,
                    x.raw(),
                    y.raw(),
                    cluster.cids()[0].get(),
                )?;
                crate::text_encoding::end(&mut output)?;
            }
            typaxis_display_list::ProductionNativeMathPaint::Rule(r) => {
                if plan(draw_index, index).is_some() {
                    return Err(E::ReceiptMismatch);
                }
                crate::text_encoding::rule(&mut TextSink { bytes, maximum }, *r)?;
            }
        }
    }
    let commands_end = bytes.len();
    append(bytes, b"Q\n", maximum)?;
    Ok(ProductionBodyTextPaint {
        draw_index,
        page_index: math.page_index(),
        font_instance_id,
        is_native_math: true,
        start,
        commands_start,
        commands_end,
        end: bytes.len(),
    })
}
