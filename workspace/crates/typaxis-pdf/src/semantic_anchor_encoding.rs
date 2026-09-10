//! Physical nonpainting extraction glyph shared by legacy and book-2 owners.
use crate::font_encoding::Sink;
use crate::text_encoding::number;
use typaxis_core::{Length, Rect};
pub(super) const GLYPH: &[u8] = b"1000 0 0 0 1000 1000 d1\n";
pub(super) const TO_UNICODE: &[u8] = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Typaxis) /Ordering (SemanticAnchor) /Supplement 0 >> def\n/CMapName /TypaxisSemanticAnchor def\n/CMapType 2 def\n1 begincodespacerange\n<00> <00>\nendcodespacerange\n1 beginbfchar\n<00> <FFFC>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";
#[derive(Clone, Copy)]
pub(super) enum Reference {
    Glyph,
    ToUnicode,
}
pub(super) fn font<S: Sink>(
    out: &mut S,
    name: &[u8],
    mut reference: impl FnMut(&mut S, Reference) -> Result<(), S::Error>,
) -> Result<(), S::Error> {
    out.extend(b"<< /Type /Font /Subtype /Type3 /Name /")?;
    out.extend(name)?;
    out.extend(
        b" /FontBBox [0 0 1000 1000] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /anchor ",
    )?;
    reference(out, Reference::Glyph)?;
    out.extend(b" >> /Encoding << /Type /Encoding /Differences [0 /anchor] >> /FirstChar 0 /LastChar 0 /Widths [1000] /Resources << >> /ToUnicode ")?;
    reference(out, Reference::ToUnicode)?;
    out.extend(b" >>")
}
pub(super) fn command<S: Sink>(
    out: &mut S,
    name: &[u8],
    viewport: Rect,
    baseline: Length,
) -> Result<(), S::Error> {
    out.extend(b"BT /")?;
    out.extend(name)?;
    out.extend(b" 1 Tf 3 Tr 0 Tc 0 Tw 100 Tz 0 TL 0 Ts ")?;
    number(out, viewport.width().get().raw())?;
    out.extend(b" 0 0 ")?;
    number(out, -viewport.height().get().raw())?;
    out.push(b' ')?;
    number(out, viewport.x().raw())?;
    out.push(b' ')?;
    number(out, baseline.raw())?;
    out.extend(b" Tm <00> Tj ET\n")
}
