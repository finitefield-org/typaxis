//! Frozen atomic-vector JCS preimages written without a heap copy.
use super::*;
use std::fmt;
use typaxis_core::{write_jcs_string, Sha256};

pub(super) fn item(value: &AtomicVectorInlineItem) -> [u8; 32] {
    let mut output = Sha256::new();
    write_item(&mut output, value).expect("SHA-256 formatting is infallible");
    output.finish()
}
pub(super) fn paragraph(value: &AtomicVectorInlineParagraph) -> [u8; 32] {
    let mut output = Sha256::new();
    write_paragraph(&mut output, value).expect("SHA-256 formatting is infallible");
    output.finish()
}

fn write_item<W: fmt::Write + ?Sized>(
    output: &mut W,
    value: &AtomicVectorInlineItem,
) -> fmt::Result {
    let metrics = value.metrics();
    output.write_str("{\"algorithm\":")?;
    write_jcs_string(output, ATOMIC_VECTOR_INLINE_ALGORITHM)?;
    output.write_str(",\"bidi\":\"ltr_isolate\",\"binding_fingerprint\":")?;
    write_hash(output, value.binding_fingerprint.bytes())?;
    output.write_str(",\"kind\":")?;
    write_jcs_string(output, value.kind.as_str())?;
    output.write_str(",\"line_break_class\":\"AL\",\"metrics\":")?;
    write_metrics(output, metrics)?;
    output.write_str(",\"node_id\":")?;
    write!(output, "{}", value.node_id.get())?;
    output.write_str(",\"paint\":{\"blue\":")?;
    write!(output, "{}", value.placement.paint().blue())?;
    output.write_str(",\"green\":")?;
    write!(output, "{}", value.placement.paint().green())?;
    output.write_str(",\"red\":")?;
    write!(output, "{}", value.placement.paint().red())?;
    output.write_str("},\"paragraph_node\":")?;
    write!(output, "{}", value.paragraph_node.get())?;
    output.write_str(",\"record\":\"item\",\"scale\":")?;
    write!(output, "{}", value.placement.scale().get().raw())?;
    output.write_str(",\"source_span\":")?;
    write_source_span(output, value.source_span)?;
    output.write_str(",\"spacing\":{\"after\":")?;
    write!(output, "{}", value.placement.spacing_after().get().raw())?;
    output.write_str(",\"before\":")?;
    write!(output, "{}", value.placement.spacing_before().get().raw())?;
    output.write_str("}}")?;
    Ok(())
}

fn write_paragraph<W: fmt::Write + ?Sized>(
    output: &mut W,
    value: &AtomicVectorInlineParagraph,
) -> fmt::Result {
    output.write_str("{\"algorithm\":")?;
    write_jcs_string(output, ATOMIC_VECTOR_INLINE_ALGORITHM)?;
    output.write_str(",\"boundaries\":[")?;
    for (index, boundary) in value.boundaries.iter().copied().enumerate() {
        if index > 0 {
            output.write_char(',')?;
        }
        output.write_str("{\"kind\":")?;
        write_jcs_string(output, break_kind_str(boundary.kind()))?;
        output.write_str(",\"logical_boundary\":")?;
        write!(output, "{}", (index + 1))?;
        output.write_str(",\"penalty\":")?;
        write!(output, "{}", boundary.penalty())?;
        output.write_str(",\"same_line_width\":")?;
        write!(output, "{}", boundary.same_line_width().get().raw())?;
        output.write_char('}')?;
    }
    output.write_str("],\"paragraph_node\":")?;
    write!(output, "{}", value.paragraph_node.get())?;
    output.write_str(",\"record\":\"itemization\",\"units\":[")?;
    for (index, unit) in value.units.iter().copied().enumerate() {
        if index > 0 {
            output.write_char(',')?;
        }
        match unit {
            AtomicVectorInlineLogicalUnit::Text(text) => {
                output.write_str("{\"advance\":")?;
                write!(output, "{}", text.advance().get().raw())?;
                output.write_str(",\"ascent\":")?;
                write!(output, "{}", text.ascent().get().raw())?;
                output.write_str(",\"descent\":")?;
                write!(output, "{}", text.descent().get().raw())?;
                output.write_str(",\"kind\":\"text\",\"scalar\":")?;
                write_jcs_string(output, text.scalar().encode_utf8(&mut [0; 4]))?;
                output.write_char('}')?;
            }
            AtomicVectorInlineLogicalUnit::Vector(item) => {
                output.write_str("{\"atomic_fingerprint\":")?;
                write_hash(output, item.fingerprint())?;
                output.write_str(",\"kind\":\"vector\",\"node_id\":")?;
                write!(output, "{}", item.node_id().get())?;
                output.write_char('}')?;
            }
        }
    }
    output.write_str("],\"vector_count\":")?;
    write!(output, "{}", value.vector_count)?;
    output.write_char('}')?;
    Ok(())
}

fn write_metrics<W: fmt::Write + ?Sized>(
    output: &mut W,
    value: BoundPrecomposedVectorMetrics,
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
    output.write_str("}}")?;
    Ok(())
}

fn write_source_span<W: fmt::Write + ?Sized>(output: &mut W, value: SourceSpan) -> fmt::Result {
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
