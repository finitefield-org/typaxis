//! Physical font/extraction byte encoding shared by existing and book-2
//! owners. Source, budget and PDF object authority stay with each caller.
pub(super) trait Sink {
    type Error;
    fn extend(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
    fn push(&mut self, byte: u8) -> Result<(), Self::Error> {
        self.extend(&[byte])
    }
    fn unsigned(&mut self, value: u64) -> Result<(), Self::Error> {
        let (digits, start) = super::decimal_digits(u128::from(value));
        self.extend(&digits[start..])
    }
}
impl Sink for super::LimitedPdfBuffer {
    type Error = super::PdfError;
    fn extend(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        super::LimitedPdfBuffer::extend(self, bytes)
    }
}
pub(super) const HEADER: &[u8] = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n\
/CMapName /Typaxis-Identity-UCS def\n\
/CMapType 2 def\n\
1 begincodespacerange\n\
<0000> <FFFF>\n\
endcodespacerange\n";
pub(super) fn to_unicode<I, C, S>(mut mappings: I, output: &mut S) -> Result<(), S::Error>
where
    I: Iterator<Item = (u16, C)> + Clone,
    C: IntoIterator<Item = char>,
    S: Sink,
{
    output.extend(HEADER)?;
    loop {
        // At most two traversals; no temporary vector of binding references.
        let count = mappings.clone().take(100).count();
        if count == 0 {
            break;
        }
        output.unsigned(count as u64)?;
        output.extend(b" beginbfchar\n")?;
        for (cid, text) in mappings.by_ref().take(count) {
            hex(output, &cid.to_be_bytes())?;
            output.push(b' ')?;
            utf16(output, text, false)?;
            output.push(b'\n')?;
        }
        output.extend(b"endbfchar\n")?;
    }
    output.extend(b"endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n")
}
pub(super) fn hex<S: Sink>(output: &mut S, bytes: &[u8]) -> Result<(), S::Error> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    output.push(b'<')?;
    for byte in bytes {
        output.push(HEX[usize::from(byte >> 4)])?;
        output.push(HEX[usize::from(byte & 15)])?;
    }
    output.push(b'>')
}
pub(super) fn utf16<S: Sink>(
    output: &mut S,
    scalars: impl IntoIterator<Item = char>,
    bom: bool,
) -> Result<(), S::Error> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    output.push(b'<')?;
    if bom {
        output.extend(b"FEFF")?;
    }
    for scalar in scalars {
        let mut units = [0; 2];
        for unit in scalar.encode_utf16(&mut units) {
            for byte in unit.to_be_bytes() {
                output.push(HEX[usize::from(byte >> 4)])?;
                output.push(HEX[usize::from(byte & 15)])?;
            }
        }
    }
    output.push(b'>')
}
pub(super) fn gid_map<S: Sink>(
    output: &mut S,
    glyphs: impl IntoIterator<Item = u16>,
) -> Result<(), S::Error> {
    output.extend(&0u16.to_be_bytes())?;
    for glyph in glyphs {
        output.extend(&glyph.to_be_bytes())?;
    }
    Ok(())
}
pub(super) fn cid_set<S: Sink>(output: &mut S, glyph_count: usize) -> Result<(), S::Error> {
    let mut whole = glyph_count / 8;
    let block = [0xff; 256];
    while whole > 0 {
        let take = whole.min(block.len());
        output.extend(&block[..take])?;
        whole -= take;
    }
    if glyph_count % 8 != 0 {
        output.push(u8::MAX << (8 - glyph_count % 8))?;
    }
    Ok(())
}

impl Sink for Vec<u8> {
    type Error = super::PdfError;
    fn extend(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.try_reserve_exact(bytes.len())
            .map_err(|_| super::PdfError::OutputTooLarge)?;
        self.extend_from_slice(bytes);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cmap_chunks_and_utf16_preserve_supplementary_and_multiple_scalars() {
        // Include an empty binding between populated entries, and cross the
        // PDF CMap's 100-entry block limit with a two-scalar IVS mapping.
        let mut bindings = (1..=101u16)
            .map(|cid| typaxis_font::CidBinding {
                cid: typaxis_font::Cid::new(cid).unwrap(),
                width_1000: 500,
                subset_gid: typaxis_font::SubsetGlyphId::new(cid),
                unicode: "A".chars().map(typaxis_font::UnicodeScalar::new).collect(),
            })
            .collect::<Vec<_>>();
        bindings[100].unicode = "一\u{e0100}"
            .chars()
            .map(typaxis_font::UnicodeScalar::new)
            .collect();
        let mut empty = bindings[0].clone();
        empty.unicode.clear();
        bindings.insert(40, empty);
        let bytes = crate::to_unicode_bindings(&bindings, 100_000).unwrap();
        let source = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(source.matches(" beginbfchar\n").count(), 2);
        assert!(source.contains("100 beginbfchar\n<0001> <0041>\n"));
        assert!(source.contains(
            "<0064> <0041>\nendbfchar\n1 beginbfchar\n<0065> <4E00DB40DD00>\nendbfchar\n"
        ));
        assert_eq!(
            crate::to_unicode_bindings(&bindings, bytes.len() as u64).unwrap(),
            bytes
        );
        assert!(crate::to_unicode_bindings(&bindings, bytes.len() as u64 - 1).is_err());
        let mut actual = Vec::new();
        utf16(&mut actual, "一\u{e0100}".chars(), true).unwrap();
        assert_eq!(actual, b"<FEFF4E00DB40DD00>");
        for (count, expected) in [
            (1, vec![0x80]),
            (8, vec![0xff]),
            (9, vec![0xff, 0x80]),
            (17, vec![0xff, 0xff, 0x80]),
        ] {
            assert_eq!(
                crate::dense_cid_set(count, expected.len() as u64).unwrap(),
                expected
            );
            assert!(crate::dense_cid_set(count, expected.len() as u64 - 1).is_err());
        }
    }
}

pub(super) fn decimal<S: Sink>(output: &mut S, decimal: super::PdfDecimal) -> Result<(), S::Error> {
    if decimal.coefficient == 0 {
        return output.push(b'0');
    }
    let (digits, start) = super::decimal_digits(decimal.coefficient.unsigned_abs());
    let digits = &digits[start..];
    let scale = usize::from(decimal.scale);
    let mut token = [0u8; 57];
    let mut length = 0usize;
    if decimal.coefficient.is_negative() {
        token[length] = b'-';
        length += 1;
    }
    if scale == 0 {
        token[length..length + digits.len()].copy_from_slice(digits);
        length += digits.len();
    } else if digits.len() <= scale {
        token[length] = b'0';
        token[length + 1] = b'.';
        length += 2;
        let zeroes = scale - digits.len();
        token[length..length + zeroes].fill(b'0');
        length += zeroes;
        token[length..length + digits.len()].copy_from_slice(digits);
        length += digits.len();
    } else {
        let split = digits.len() - scale;
        token[length..length + split].copy_from_slice(&digits[..split]);
        length += split;
        token[length] = b'.';
        length += 1;
        token[length..length + scale].copy_from_slice(&digits[split..]);
        length += scale;
    }
    if scale > 0 {
        while token.get(length.wrapping_sub(1)) == Some(&b'0') {
            length -= 1;
        }
        if token.get(length.wrapping_sub(1)) == Some(&b'.') {
            length -= 1;
        }
    }
    output.extend(&token[..length])
}

pub(super) fn name<S: Sink>(out: &mut S, bytes: &[u8]) -> Result<(), S::Error> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push(b'/')?;
    for &b in bytes {
        if (33..=126).contains(&b) && !b"()<>[]{}/%#".contains(&b) {
            out.push(b)?;
        } else {
            out.extend(&[b'#', HEX[(b >> 4) as usize], HEX[(b & 15) as usize]])?;
        }
    }
    Ok(())
}
