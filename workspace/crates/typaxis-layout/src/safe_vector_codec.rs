//! Shared, frozen vector JCS fields for owned receipts and digest-only sinks.
use super::*;
use std::fmt;
use typaxis_core::write_jcs_string;
#[cfg(feature = "book-v2-staging")]
use typaxis_core::Sha256;

#[cfg(feature = "book-v2-staging")]
pub(super) fn book_binding(
    epoch: [u8; 32],
    node_id: NodeId,
    placement: &PrecomposedVectorPlacementInput,
    resource: &BoundPrecomposedVectorResource,
    source_span: SourceSpan,
) -> [u8; 32] {
    let mut output = Sha256::new();
    write_book_binding(
        &mut output,
        epoch,
        node_id,
        placement,
        resource,
        source_span,
    )
    .expect("SHA-256 formatting is infallible");
    output.finish()
}

#[cfg(feature = "book-v2-staging")]
fn write_book_binding<W: fmt::Write + ?Sized>(
    output: &mut W,
    epoch: [u8; 32],
    node_id: NodeId,
    placement: &PrecomposedVectorPlacementInput,
    resource: &BoundPrecomposedVectorResource,
    source_span: SourceSpan,
) -> fmt::Result {
    output.write_str("{\"algorithm\":")?;
    write_jcs_string(output, book_v2::BOOK_V2_VECTOR_BINDING_ALGORITHM)?;
    output.write_str(",\"epoch\":")?;
    write_hash(output, epoch)?;
    output.write_str(",\"node_id\":")?;
    write!(output, "{}", node_id.get())?;
    output.write_str(",\"placement\":")?;
    write_placement(output, placement)?;
    output.write_str(",\"resource\":")?;
    write_resource(output, resource)?;
    output.write_str(",\"source_span\":")?;
    write_source_span(output, source_span)?;
    output.write_char('}')?;
    Ok(())
}

pub(super) fn write_resource<W: fmt::Write + ?Sized>(
    output: &mut W,
    value: &BoundPrecomposedVectorResource,
) -> fmt::Result {
    output.write_str("{\"admitted_media\":")?;
    write_jcs_string(output, value.admitted_media.as_str())?;
    output.write_str(",\"declared_media\":")?;
    write_jcs_string(output, value.declared_media.as_str())?;
    output.write_str(",\"image_id\":")?;
    write!(output, "{}", value.image_id.get())?;
    output.write_str(",\"intrinsic_height\":")?;
    write!(output, "{}", value.intrinsic_height.get().raw())?;
    output.write_str(",\"intrinsic_width\":")?;
    write!(output, "{}", value.intrinsic_width.get().raw())?;
    output.write_str(",\"ir_fingerprint\":")?;
    write_hash(output, value.ir_fingerprint)?;
    output.write_str(",\"ir_fingerprint_id\":")?;
    write_jcs_string(output, value.ir_fingerprint_id)?;
    output.write_str(",\"ir_id\":")?;
    write_jcs_string(output, value.ir_id)?;
    output.write_str(",\"limits_fingerprint\":")?;
    write_hash(output, value.limits_fingerprint)?;
    output.write_str(",\"parser_id\":")?;
    write_jcs_string(output, value.parser_id)?;
    output.write_str(",\"profile_fingerprint\":")?;
    write_hash(output, value.profile_fingerprint)?;
    output.write_str(",\"source_sha256\":")?;
    write_hash(output, value.source_sha256)?;
    output.write_str(",\"view_box\":[")?;
    for (index, coordinate) in value.view_box.iter().enumerate() {
        if index > 0 {
            output.write_char(',')?;
        }
        write!(output, "{coordinate}")?;
    }
    output.write_str("]}")?;
    Ok(())
}

pub(super) fn write_placement<W: fmt::Write + ?Sized>(
    output: &mut W,
    value: &PrecomposedVectorPlacementInput,
) -> fmt::Result {
    output.write_char('{')?;
    match value {
        PrecomposedVectorPlacementInput::Inline(value) => {
            output.write_str("\"kind\":\"inline\",\"metrics\":")?;
            write_metrics(output, value.metrics())?;
            output.write_str(",\"paint\":")?;
            write_rgb(output, value.paint())?;
            output.write_str(",\"scale\":")?;
            write!(output, "{}", value.scale().get().raw())?;
            output.write_str(",\"spacing_after\":")?;
            write!(output, "{}", value.spacing_after().get().raw())?;
            output.write_str(",\"spacing_before\":")?;
            write!(output, "{}", value.spacing_before().get().raw())?;
        }
        PrecomposedVectorPlacementInput::VectorFigure(value) => {
            output.write_str("\"kind\":\"vector_figure\",\"paint\":")?;
            write_rgb(output, value.paint())?;
            output.write_str(",\"scale\":")?;
            write!(output, "{}", value.scale().get().raw())?;
            output.write_str(",\"style_fingerprint\":")?;
            write_hash(output, value.style().fingerprint())?;
            output.write_str(",\"viewport\":{\"height\":")?;
            write!(output, "{}", value.viewport_height().get().raw())?;
            output.write_str(",\"width\":")?;
            write!(output, "{}", value.viewport_width().get().raw())?;
            output.write_char('}')?;
        }
        PrecomposedVectorPlacementInput::MathVectorBlock(value) => {
            output.write_str("\"kind\":\"math_vector_block\",\"metrics\":")?;
            write_metrics(output, value.metrics())?;
            output.write_str(",\"paint\":")?;
            write_rgb(output, value.paint())?;
            output.write_str(",\"scale\":")?;
            write!(output, "{}", value.scale().get().raw())?;
            output.write_str(",\"style_fingerprint\":")?;
            write_hash(output, value.style().fingerprint())?;
        }
    }
    output.write_char('}')?;
    Ok(())
}

fn write_metrics<W: fmt::Write + ?Sized>(
    output: &mut W,
    value: typaxis_layout_contract::BoundPrecomposedVectorMetrics,
) -> fmt::Result {
    output.write_str("{\"advance\":")?;
    write!(output, "{}", value.advance().get().raw())?;
    output.write_str(",\"ascent\":")?;
    write!(output, "{}", value.ascent().get().raw())?;
    output.write_str(",\"baseline\":")?;
    write!(output, "{}", value.baseline().get().raw())?;
    output.write_str(",\"descent\":")?;
    write!(output, "{}", value.descent().get().raw())?;
    output.write_str(",\"origin_x\":")?;
    write!(output, "{}", value.origin_x().raw())?;
    output.write_str(",\"viewport\":{\"height\":")?;
    write!(output, "{}", value.viewport_height().get().raw())?;
    output.write_str(",\"width\":")?;
    write!(output, "{}", value.viewport_width().get().raw())?;
    output.write_str("},\"viewport_right_from_pen\":")?;
    write!(output, "{}", value.viewport_right_from_pen().raw())?;
    output.write_char('}')?;
    Ok(())
}

fn write_rgb<W: fmt::Write + ?Sized>(output: &mut W, value: ResolvedRgb8) -> fmt::Result {
    output.write_str("{\"blue\":")?;
    write!(output, "{}", value.blue())?;
    output.write_str(",\"green\":")?;
    write!(output, "{}", value.green())?;
    output.write_str(",\"red\":")?;
    write!(output, "{}", value.red())?;
    output.write_char('}')?;
    Ok(())
}

pub(super) fn write_source_span<W: fmt::Write + ?Sized>(
    output: &mut W,
    value: SourceSpan,
) -> fmt::Result {
    output.write_str("{\"end_byte\":")?;
    write!(output, "{}", value.end_byte().get())?;
    output.write_str(",\"source_id\":")?;
    write!(output, "{}", value.source_id().get())?;
    output.write_str(",\"start_byte\":")?;
    write!(output, "{}", value.start_byte().get())?;
    output.write_char('}')?;
    Ok(())
}

fn write_hash<W: fmt::Write + ?Sized>(output: &mut W, value: [u8; 32]) -> fmt::Result {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.write_char('"')?;
    for byte in value {
        output.write_char(char::from(HEX[usize::from(byte >> 4)]))?;
        output.write_char(char::from(HEX[usize::from(byte & 0x0f)]))?;
    }
    output.write_char('"')?;
    Ok(())
}
