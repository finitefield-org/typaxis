//! Name-keyed CFF1 uses SIDs, one Private DICT and an implicit FD index of zero.
//! Encoding is structurally checked; Unicode selection continues to use cmap.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffSourceGlyphV2 {
    Cid(u16),
    Sid(u16),
}
#[derive(Debug)]
pub(super) enum SourceCharset {
    Cid(Vec<u16>),
    Name(Vec<u16>),
}
impl SourceCharset {
    pub(super) fn glyph(&self, gid: usize) -> Option<CffSourceGlyphV2> {
        match self {
            Self::Cid(ids) => ids.get(gid).copied().map(CffSourceGlyphV2::Cid),
            Self::Name(ids) => ids.get(gid).copied().map(CffSourceGlyphV2::Sid),
        }
    }
}
pub(super) fn charset<B: CffAdmissionBudgetV2>(
    b: &[u8],
    at: usize,
    glyph_count: u16,
    strings: usize,
    s: &mut program_budget::State<'_, B>,
) -> program_budget::Result<(Vec<u16>, Option<ProgramSpan>), B::Error> {
    let (ids, span) = if at <= 2 {
        s.work(PStage::PredefinedCharset, None, Some(15), 16)?;
        let capacity = [229, 166, 87][at];
        if usize::from(glyph_count) > capacity {
            return Err(fail(K::InvalidCharset, at).into());
        }
        let mut ids = Vec::new();
        s.reserve(&mut ids, usize::from(glyph_count), PSite::SidByGlyph, at)?;
        // read-fonts only performs static array/identity lookups for these
        // three charsets. Pay before the borrowed iterator and vector writes.
        s.work(
            PStage::PredefinedCharset,
            None,
            Some(15),
            u64::from(glyph_count) * 8 + 16,
        )?;
        let predefined = read_fonts::tables::postscript::Charset::new(
            FontData::new(b),
            at,
            u32::from(glyph_count),
        )
        .map_err(|_| fail(K::InvalidCharset, at))?;
        ids.extend(predefined.iter().map(|(_, sid)| sid.to_u16()));
        if ids.len() != usize::from(glyph_count) {
            return Err(fail(K::InvalidCharset, at).into());
        }
        (ids, None)
    } else {
        let (ids, span) = custom_charset(b, at, glyph_count, PSite::SidByGlyph, s)?;
        (ids, Some(span))
    };
    s.work(
        PStage::SidCount,
        span.map(|v| v.start),
        Some(15),
        ids.len() as u64 + 1,
    )?;
    if ids
        .iter()
        .any(|sid| !valid_sid(&DictOperand::Integer(i32::from(*sid)), strings))
    {
        return Err(fail(K::InvalidCharset, at).into());
    }
    Ok((ids, span))
}
pub(super) fn encoding<B: CffAdmissionBudgetV2>(
    b: &[u8],
    at: usize,
    glyph_count: u16,
    sids: &[u16],
    s: &mut program_budget::State<'_, B>,
) -> program_budget::Result<Option<ProgramSpan>, B::Error> {
    if at <= 1 {
        return Ok(None); // StandardEncoding or ExpertEncoding, no source span.
    }
    s.work(PStage::EncodingHeader, Some(at), Some(16), 16)?;
    let mut codes = [0u64; 4];
    let format = *b.get(at).ok_or_else(|| fail(K::InvalidEncoding, at))?;
    let count = usize::from(
        *b.get(at + 1)
            .ok_or_else(|| fail(K::InvalidEncoding, at + 1))?,
    );
    let mut cursor = at + 2;
    let mut encoded = 1usize; // GID zero is implicit .notdef and has no code.
    match format & 0x7f {
        0 => {
            for _ in 0..count {
                s.work(PStage::EncodingCode, Some(cursor), Some(16), 4)?;
                let code = *b
                    .get(cursor)
                    .ok_or_else(|| fail(K::InvalidEncoding, cursor))?;
                code_once(&mut codes, code, cursor)?;
                cursor += 1;
                encoded += 1;
            }
        }
        1 => {
            for _ in 0..count {
                s.work(PStage::EncodingRange, Some(cursor), Some(16), 4)?;
                let first = *b
                    .get(cursor)
                    .ok_or_else(|| fail(K::InvalidEncoding, cursor))?;
                let left = *b
                    .get(cursor + 1)
                    .ok_or_else(|| fail(K::InvalidEncoding, cursor + 1))?;
                let last = first
                    .checked_add(left)
                    .ok_or_else(|| fail(K::InvalidEncoding, cursor))?;
                for code in first..=last {
                    s.work(PStage::EncodingCode, Some(cursor), Some(16), 4)?;
                    code_once(&mut codes, code, cursor)?;
                }
                encoded += usize::from(left) + 1;
                cursor += 2;
            }
        }
        _ => return Err(fail(K::InvalidEncoding, at).into()),
    }
    if encoded > usize::from(glyph_count) {
        return Err(fail(K::InvalidEncoding, cursor).into());
    }
    if format & 0x80 != 0 {
        s.work(PStage::EncodingSupplement, Some(cursor), Some(16), 1)?;
        let count = *b
            .get(cursor)
            .ok_or_else(|| fail(K::InvalidEncoding, cursor))?;
        cursor += 1;
        for _ in 0..count {
            s.work(
                PStage::EncodingSupplement,
                Some(cursor),
                Some(16),
                sids.len() as u64 + 8,
            )?;
            let code = *b
                .get(cursor)
                .ok_or_else(|| fail(K::InvalidEncoding, cursor))?;
            let sid = u16_at(b, cursor + 1).map_err(|_| fail(K::InvalidEncoding, cursor + 1))?;
            // Charset validation has already established SID bounds/uniqueness.
            if sid == 0 || !sids.contains(&sid) {
                return Err(fail(K::InvalidEncoding, cursor + 1).into());
            }
            code_once(&mut codes, code, cursor)?;
            cursor += 3;
        }
    }
    Ok(Some(ProgramSpan {
        start: at,
        end: cursor,
    }))
}
fn code_once(codes: &mut [u64; 4], code: u8, at: usize) -> Result<(), CffProgramErrorV2> {
    let word = &mut codes[usize::from(code) / 64];
    let mask = 1u64 << (code % 64);
    if *word & mask != 0 {
        return Err(fail(K::InvalidEncoding, at));
    }
    *word |= mask;
    Ok(())
}

#[cfg(test)]
#[path = "cff_v2_name_keyed_tests.rs"]
mod tests;
