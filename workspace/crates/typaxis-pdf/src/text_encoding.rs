//! Physical glyph/rule operators in the selected page's Y-down coordinates.
//! Font/source ownership and marked-content scope belong to the caller.
use crate::font_encoding::{self, Sink};

pub(super) fn number<S: Sink>(out: &mut S, raw: i64) -> Result<(), S::Error> {
    font_encoding::decimal(
        out,
        crate::PdfDecimal {
            coefficient: i128::from(raw) * 152_587_890_625,
            scale: 16,
        },
    )
}
pub(super) fn begin<S: Sink>(
    out: &mut S,
    font: u32,
    size: i64,
    black: bool,
) -> Result<(), S::Error> {
    if black {
        out.extend(b"0 g\n")?;
    }
    out.extend(b"BT /PB")?;
    out.unsigned(u64::from(font))?;
    out.push(b' ')?;
    number(out, size)?;
    out.extend(b" Tf 0 Tr\n0 Tc 0 Tw 100 Tz 0 TL 0 Ts\n")
}
pub(super) fn glyph<S: Sink>(out: &mut S, x: i64, y: i64, cid: u16) -> Result<(), S::Error> {
    out.extend(b"1 0 0 -1 ")?;
    number(out, x)?;
    out.push(b' ')?;
    number(out, y)?;
    out.extend(b" Tm ")?;
    font_encoding::hex(out, &cid.to_be_bytes())?;
    out.extend(b" Tj\n")
}
pub(super) fn end<S: Sink>(out: &mut S) -> Result<(), S::Error> {
    out.extend(b"ET\n")
}
pub(super) fn rule<S: Sink>(out: &mut S, rect: typaxis_core::Rect) -> Result<(), S::Error> {
    for (i, raw) in [
        rect.x().raw(),
        rect.y().raw(),
        rect.width().get().raw(),
        rect.height().get().raw(),
    ]
    .into_iter()
    .enumerate()
    {
        if i > 0 {
            out.push(b' ')?;
        }
        number(out, raw)?;
    }
    out.extend(b" re f\n")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_text_numbers_keep_full_signed_fixed_point_range() {
        for raw in [
            i64::MIN,
            i64::MAX,
            -65537,
            -65536,
            -32769,
            -1,
            0,
            1,
            32769,
            65536,
            65537,
        ] {
            let mut bytes = Vec::new();
            number(&mut bytes, raw).unwrap();
            assert_eq!(
                std::str::from_utf8(&bytes).unwrap(),
                crate::tagged_pdf_v2::pdf_number_v2(raw)
            );
        }
        let mut bytes = Vec::new();
        begin(&mut bytes, 7, 655360, true).unwrap();
        glyph(&mut bytes, -1, 65537, 0xabcd).unwrap();
        end(&mut bytes).unwrap();
        assert_eq!(bytes,b"0 g\nBT /PB7 10 Tf 0 Tr\n0 Tc 0 Tw 100 Tz 0 TL 0 Ts\n1 0 0 -1 -0.0000152587890625 1.0000152587890625 Tm <ABCD> Tj\nET\n");
    }
}
