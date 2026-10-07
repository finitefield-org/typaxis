//! Physical Image/Form dictionary fields, independent of resource ownership.
use crate::font_encoding::Sink;
#[derive(Clone, Copy)]
pub(crate) enum Color {
    Gray,
    Rgb,
}
pub(crate) fn raster_fields<S: Sink>(
    out: &mut S,
    width: u32,
    height: u32,
    color: Color,
    bits: u8,
) -> Result<(), S::Error> {
    out.extend(b" /Type /XObject /Subtype /Image /Width ")?;
    out.unsigned(width.into())?;
    out.extend(b" /Height ")?;
    out.unsigned(height.into())?;
    out.extend(b" /ColorSpace /")?;
    out.extend(match color {
        Color::Gray => b"DeviceGray",
        Color::Rgb => b"DeviceRGB",
    })?;
    out.extend(b" /BitsPerComponent ")?;
    out.unsigned(bits.into())
}
pub(crate) fn filter<S: Sink>(out: &mut S, jpeg: Option<u8>) -> Result<(), S::Error> {
    if let Some(transform) = jpeg {
        out.extend(b" /Filter /DCTDecode /DecodeParms << /ColorTransform ")?;
        out.unsigned(transform.into())?;
        out.extend(b" >>")
    } else {
        out.extend(b" /Filter /FlateDecode")
    }
}
pub(crate) fn form_header<S: Sink>(out: &mut S, bbox: [i64; 4]) -> Result<(), S::Error> {
    out.extend(b"<< /Type /XObject /Subtype /Form /FormType 1 /BBox [")?;
    for (i, raw) in bbox.into_iter().enumerate() {
        if i > 0 {
            out.push(b' ')?;
        }
        crate::text_encoding::number(out, raw)?;
    }
    out.extend(b"] /Resources << /ExtGState <<")
}

pub(crate) fn raster_placement<S: Sink>(
    out: &mut S,
    values: [i64; 4],
    name: impl FnOnce(&mut S) -> Result<(), S::Error>,
) -> Result<(), S::Error> {
    let [width, negative_height, x, bottom] = values;
    out.extend(b"q\n")?;
    crate::text_encoding::number(out, width)?;
    out.extend(b" 0 0 ")?;
    crate::text_encoding::number(out, negative_height)?;
    out.push(b' ')?;
    crate::text_encoding::number(out, x)?;
    out.push(b' ')?;
    crate::text_encoding::number(out, bottom)?;
    out.extend(b" cm\n/")?;
    name(out)?;
    out.extend(b" Do\nQ\n")
}
