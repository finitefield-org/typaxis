//! Stream the frozen inline fingerprint preimage; no serialized copy is retained.
use super::{
    break_kind_str, AtomicVectorInlineBoundary, JapaneseLineBreakMode, ProductionInlineLogicalUnit,
    ProductionTextClusterRange, PRODUCTION_INLINE_BREAK_ALGORITHM,
};
use std::fmt;
use typaxis_core::{NodeId, SourceSpan};

pub(super) fn write_paragraph<W: fmt::Write + ?Sized>(
    output: &mut W,
    owner: NodeId,
    units: &[ProductionInlineLogicalUnit],
    boundaries: &[AtomicVectorInlineBoundary],
    clusters: &[ProductionTextClusterRange],
    japanese_mode: JapaneseLineBreakMode,
) -> fmt::Result {
    use ProductionInlineLogicalUnit as U;
    output.write_str(PRODUCTION_INLINE_BREAK_ALGORITHM)?;
    output.write_str(match japanese_mode {
        JapaneseLineBreakMode::Loose => "/loose/",
        JapaneseLineBreakMode::Normal => "/normal/",
        JapaneseLineBreakMode::Strict => "/strict/",
    })?;
    write!(output, "/{}/", owner.get())?;
    for (index, unit) in units.iter().enumerate() {
        match unit {
            U::Text(t) => write!(
                output,
                "/text/{}/{}/{}/{}",
                t.scalar() as u32,
                t.advance().get().raw(),
                t.ascent().get().raw(),
                t.descent().get().raw()
            )?,
            U::Vector(v) => {
                output.write_str("/vector/")?;
                write_hash(output, v.fingerprint())?;
            }
            U::Math(m) => {
                output.write_str("/native_math/")?;
                write_hash(output, m.fingerprint())?;
            }
            U::Break(b) => {
                write!(
                    output,
                    "/break/{}/{}/{}/",
                    index,
                    b.owner.get(),
                    break_kind_str(b.kind)
                )?;
                write_source_span(output, b.source_span)?;
            }
        }
    }
    for b in boundaries {
        write!(
            output,
            "/boundary/{}/{}",
            break_kind_str(b.kind()),
            b.penalty()
        )?;
    }
    for range in clusters {
        write!(output, "/{},{}", range.start_unit, range.end_unit)?;
    }
    Ok(())
}

pub(super) fn write_source_span<W: fmt::Write + ?Sized>(
    output: &mut W,
    value: SourceSpan,
) -> fmt::Result {
    write!(
        output,
        "{{\"end_byte\":{},\"source_id\":{},\"start_byte\":{}}}",
        value.end_byte().get(),
        value.source_id().get(),
        value.start_byte().get()
    )
}

pub(super) fn write_hash<W: fmt::Write + ?Sized>(output: &mut W, value: [u8; 32]) -> fmt::Result {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.write_char('"')?;
    for byte in value {
        output.write_char(char::from(HEX[usize::from(byte >> 4)]))?;
        output.write_char(char::from(HEX[usize::from(byte & 0x0f)]))?;
    }
    output.write_char('"')
}
