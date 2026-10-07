//! Stream the existing source-flow canonical bytes to a caller-owned writer.
use super::{ProductionFlowEvent, ProductionInlineContent, SourceTextFlow};
use std::fmt::{self, Write};
use typaxis_core::{write_jcs_string, Sha256};

pub(super) fn fingerprint_source_flow<P, N>(
    flow: &SourceTextFlow<'_, P, N>,
    algorithm: &str,
    language_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    package_sha256: [u8; 32],
) -> [u8; 32] {
    let mut output = Sha256::new();
    write_source_flow(
        flow,
        algorithm,
        language_sha256,
        limits_sha256,
        package_sha256,
        &mut output,
    )
    .expect("writing source-flow canonical bytes to SHA-256 cannot fail");
    output.finish()
}

fn write_source_flow<P, N, W: Write + ?Sized>(
    flow: &SourceTextFlow<'_, P, N>,
    algorithm: &str,
    language_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    package_sha256: [u8; 32],
    output: &mut W,
) -> fmt::Result {
    output.write_str("{\"algorithm\":")?;
    write_jcs_string(output, algorithm)?;
    output.write_str(",\"events\":[")?;
    for (i, event) in flow.events.iter().enumerate() {
        if i != 0 {
            output.write_char(',')?;
        }
        match event {
            ProductionFlowEvent::Begin { owner, kind } => {
                output.write_str("[\"begin\",")?;
                write_jcs_string(output, kind.as_str())?;
                write!(output, ",{}]", owner.get())?;
            }
            ProductionFlowEvent::Paragraph { index } => write!(output, "[\"paragraph\",{index}]")?,
            ProductionFlowEvent::End { owner } => write!(output, "[\"end\",{}]", owner.get())?,
        }
    }
    #[cfg(feature = "book-v2-staging")]
    if !flow.named_page_breaks.is_empty() {
        output.write_str("],\"explicit_page_break_names\":[")?;
        for (index, (owner, name)) in flow.named_page_breaks.iter().enumerate() {
            if index != 0 {
                output.write_char(',')?;
            }
            write!(output, "[{},", owner.get())?;
            write_jcs_string(output, name.as_str())?;
            output.write_char(']')?;
        }
    }
    output.write_str("],\"footnote_definitions\":[")?;
    for (index, definition) in flow.footnote_definitions.iter().enumerate() {
        if index != 0 {
            output.write_char(',')?;
        }
        write!(output, "[{},", definition.owner.get())?;
        write_jcs_string(output, definition.id)?;
        output.write_char(',')?;
        write_jcs_string(output, definition.language)?;
        output.write_char(',')?;
        match definition.style_paragraph {
            Some(index) => write!(output, "{index}")?,
            None => output.write_str("null")?,
        }
        output.write_char(']')?;
    }
    output.write_str("],\"generated_text_sha256\":")?;
    write_hash(output, flow.generated.reference_fingerprint().bytes())?;
    output.write_str(",\"language_sha256\":")?;
    write_hash(output, language_sha256)?;
    output.write_str(",\"limits_sha256\":")?;
    write_hash(output, limits_sha256)?;
    output.write_str(",\"package_sha256\":")?;
    write_hash(output, package_sha256)?;
    output.write_str(",\"paragraphs\":[")?;
    for (i, paragraph) in flow.paragraphs.iter().enumerate() {
        if i != 0 {
            output.write_char(',')?;
        }
        output.write_str("{\"font_families\":[")?;
        for (j, family) in paragraph
            .style
            .font_families()
            .unwrap_or(&[])
            .iter()
            .enumerate()
        {
            if j != 0 {
                output.write_char(',')?;
            }
            write_jcs_string(output, family)?;
        }
        write!(
            output,
            "],\"font_size\":{},\"items\":[",
            paragraph.style.font_size().map_or(0, |v| v.get().raw())
        )?;
        for (j, item) in paragraph.items.iter().enumerate() {
            if j != 0 {
                output.write_char(',')?;
            }
            output.write_char('[')?;
            write_jcs_string(output, item.content.as_str())?;
            write!(output, ",{},", item.owner.get())?;
            write_jcs_string(output, item.language)?;
            if let ProductionInlineContent::Text { span, utf8 } = item.content {
                write!(
                    output,
                    ",{{\"end_byte\":{},\"start_byte\":{},\"text_id\":{}}},",
                    span.end_byte().get(),
                    span.start_byte().get(),
                    span.text_id().get()
                )?;
                write_jcs_string(output, utf8)?;
            }
            output.write_char(']')?;
        }
        write!(
            output,
            "],\"line_height\":{},\"owner\":{},\"page\":",
            paragraph.style.line_height().map_or(0, |v| v.get().raw()),
            paragraph.owner.get()
        )?;
        if let Some(page) = &paragraph.page_name {
            write_jcs_string(output, page.as_str())?;
        } else {
            output.write_str("null")?;
        }
        output.write_char('}')?;
    }
    // Remaining block/source fields stay bound by package_sha256. Verification
    // still independently rebuilds all styles/events, not just this hash.
    write!(output, "],\"text_bytes\":{}}}", flow.text_bytes)
}

fn write_hash<W: Write + ?Sized>(output: &mut W, bytes: [u8; 32]) -> fmt::Result {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.write_char('"')?;
    for byte in bytes {
        output.write_char(char::from(HEX[usize::from(byte >> 4)]))?;
        output.write_char(char::from(HEX[usize::from(byte & 15)]))?;
    }
    output.write_char('"')
}

#[cfg(test)]
#[path = "production_flow_canonical_tests.rs"]
mod tests;
