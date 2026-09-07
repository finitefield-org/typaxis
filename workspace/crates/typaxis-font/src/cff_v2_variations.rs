//! Validate supplemental format-14 coverage without expanding Unicode ranges.
//! The base cmap and subsequent two-scalar shaping remain separate owners.
use super::{CffTableFailureKindV2 as K, CffTableFailureV2};
const MAX_SELECTORS: u64 = 256;
const MAX_VALUES: u64 = 1_000_000;
fn fail(at: usize, kind: K) -> CffTableFailureV2 {
    CffTableFailureV2 {
        table: *b"cmap",
        table_offset: at,
        kind,
        limit: None,
        observed: None,
    }
}
fn bound(at: usize, limit: u64, observed: u64) -> CffTableFailureV2 {
    CffTableFailureV2 {
        limit: Some(limit),
        observed: Some(observed),
        ..fail(at, K::UnsupportedComplexity)
    }
}
fn range(b: &[u8], at: usize, n: usize) -> Result<&[u8], CffTableFailureV2> {
    at.checked_add(n)
        .and_then(|end| b.get(at..end))
        .ok_or_else(|| fail(at, K::InvalidLength))
}
fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes(b[at..at + 2].try_into().unwrap())
}
fn u24_at(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([0, b[at], b[at + 1], b[at + 2]])
}
fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(b[at..at + 4].try_into().unwrap())
}
fn scalar_range(start: u32, end: u32) -> bool {
    start <= end && end <= 0x10ffff && (end < 0xd800 || start > 0xdfff)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VariationCoverage {
    Default,
    NonDefault(u16),
    Missing,
}
#[derive(Debug)]
pub struct CffVariationSequencesV2<'a> {
    table: &'a [u8],
    records: usize,
    values: u64,
}
#[derive(Debug)]
pub(super) struct OwnedCffVariationSequencesV2 {
    bytes: Vec<u8>,
    records: usize,
    values: u64,
}
impl OwnedCffVariationSequencesV2 {
    pub(super) fn view(&self) -> CffVariationSequencesV2<'_> {
        CffVariationSequencesV2 {
            table: &self.bytes,
            records: self.records,
            values: self.values,
        }
    }
}
impl CffVariationSequencesV2<'_> {
    pub(super) fn into_owned(self) -> Result<OwnedCffVariationSequencesV2, CffTableFailureV2> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.table.len())
            .map_err(|_| fail(0, K::AllocationFailure))?;
        bytes.extend_from_slice(self.table);
        Ok(OwnedCffVariationSequencesV2 {
            bytes,
            records: self.records,
            values: self.values,
        })
    }
    pub const fn selector_count(&self) -> usize {
        self.records
    }
    pub const fn declared_value_count(&self) -> u64 {
        self.values
    }
    pub fn coverage(&self, base: char, selector: char) -> VariationCoverage {
        let b = self.table;
        let selector = selector as u32;
        let base = base as u32;
        let mut lo = 0;
        let mut hi = self.records;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if u24_at(b, 10 + mid * 11) < selector {
                lo = mid + 1
            } else {
                hi = mid
            }
        }
        if lo == self.records || u24_at(b, 10 + lo * 11) != selector {
            return VariationCoverage::Missing;
        }
        let at = 10 + lo * 11;
        let non = u32_at(b, at + 7) as usize;
        if non != 0 {
            let n = u32_at(b, non) as usize;
            let mut lo = 0;
            let mut hi = n;
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                if u24_at(b, non + 4 + mid * 5) < base {
                    lo = mid + 1
                } else {
                    hi = mid
                }
            }
            if lo < n && u24_at(b, non + 4 + lo * 5) == base {
                return VariationCoverage::NonDefault(u16_at(b, non + 7 + lo * 5));
            }
        }
        let default = u32_at(b, at + 3) as usize;
        if default != 0 {
            let n = u32_at(b, default) as usize;
            let mut lo = 0;
            let mut hi = n;
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                let p = default + 4 + mid * 4;
                if u24_at(b, p) + u32::from(b[p + 3]) < base {
                    lo = mid + 1
                } else {
                    hi = mid
                }
            }
            if lo < n && u24_at(b, default + 4 + lo * 4) <= base {
                return VariationCoverage::Default;
            }
        }
        VariationCoverage::Missing
    }
}

/// Find and validate the supplemental Unicode/platform-0 encoding-5 subtable.
/// Other encodings still require the base-cmap validator before font admission.
pub fn validate_cff_variation_sequences_v2(
    cmap: &[u8],
    glyph_count: u16,
) -> Result<Option<CffVariationSequencesV2<'_>>, CffTableFailureV2> {
    if glyph_count == 0 {
        return Err(CffTableFailureV2 {
            table: *b"maxp",
            table_offset: 4,
            kind: K::InvalidMetricCount,
            limit: None,
            observed: None,
        });
    }
    range(cmap, 0, 4)?;
    if u16_at(cmap, 0) != 0 {
        return Err(fail(0, K::UnsupportedVersion));
    }
    let n = usize::from(u16_at(cmap, 2));
    range(cmap, 4, n * 8)?;
    let mut found = None;
    for i in 0..n {
        let at = 4 + i * 8;
        let position = u32_at(cmap, at + 4) as usize;
        if position < 4 + n * 8 {
            return Err(fail(at + 4, K::OverlappingData));
        }
        range(cmap, position, 2)?;
        let unicode_uvs = u16_at(cmap, at) == 0 && u16_at(cmap, at + 2) == 5;
        let format14 = u16_at(cmap, position) == 14;
        if unicode_uvs != format14 {
            return Err(fail(at, K::InvalidCmapEncoding));
        }
        if format14 {
            if found.is_some() {
                return Err(fail(at, K::InvalidCmapEncoding));
            }
            range(cmap, position, 10)?;
            let size = u32_at(cmap, position + 2) as usize;
            let table = range(cmap, position, size)?;
            found = Some(parse_format14(table, glyph_count).map_err(|mut e| {
                e.table_offset += position;
                e
            })?);
        }
    }
    Ok(found)
}
fn parse_format14(
    b: &[u8],
    glyph_count: u16,
) -> Result<CffVariationSequencesV2<'_>, CffTableFailureV2> {
    range(b, 0, 10)?;
    if u16_at(b, 0) != 14 || u32_at(b, 2) as usize != b.len() {
        return Err(fail(0, K::InvalidLength));
    }
    let records = u32_at(b, 6) as usize;
    if records as u64 > MAX_SELECTORS {
        return Err(bound(6, MAX_SELECTORS, records as u64));
    }
    let header_end = 10 + records * 11;
    range(b, 10, records * 11)?;
    let mut spans = Vec::new();
    spans
        .try_reserve_exact(records * 2)
        .map_err(|_| fail(6, K::AllocationFailure))?;
    let mut previous = None;
    let mut values = 0u64;
    for i in 0..records {
        let at = 10 + i * 11;
        let selector = u24_at(b, at);
        if !(0xfe00..=0xfe0f).contains(&selector) && !(0xe0100..=0xe01ef).contains(&selector) {
            return Err(fail(at, K::InvalidVariationSelector));
        }
        if previous.is_some_and(|old| old >= selector) {
            return Err(fail(at, K::InvalidVariationSelector));
        }
        previous = Some(selector);
        let mut defaults = (0, 0usize);
        for (field, stride, kind) in [(at + 3, 4usize, 0u8), (at + 7, 5usize, 1u8)] {
            let position = u32_at(b, field) as usize;
            if position == 0 {
                continue;
            }
            if position < header_end {
                return Err(fail(field, K::OverlappingData));
            }
            range(b, position, 4)?;
            let count = u32_at(b, position) as usize;
            // Every range declares >=1 value; reject before payload traversal.
            if count as u64 > MAX_VALUES - values {
                return Err(bound(position, MAX_VALUES, values + count as u64));
            }
            let size = count
                .checked_mul(stride)
                .and_then(|n| n.checked_add(4))
                .ok_or_else(|| fail(position, K::InvalidLength))?;
            range(b, position, size)?;
            spans.push((position, position + size, kind));
            let mut prev_end = None;
            let mut default_cursor = 0usize;
            for j in 0..count {
                let p = position + 4 + j * stride;
                let base = u24_at(b, p);
                let end = if kind == 0 {
                    base + u32::from(b[p + 3])
                } else {
                    base
                };
                if !scalar_range(base, end) || prev_end.is_some_and(|old| old >= base) {
                    return Err(fail(p, K::InvalidUnicodeRange));
                }
                prev_end = Some(end);
                let next = values
                    .checked_add(u64::from(end - base) + 1)
                    .ok_or_else(|| bound(p, MAX_VALUES, u64::MAX))?;
                if next > MAX_VALUES {
                    return Err(bound(p, MAX_VALUES, next));
                }
                values = next;
                if kind == 1 {
                    if u16_at(b, p + 3) >= glyph_count {
                        return Err(fail(p + 3, K::GlyphOutOfRange));
                    }
                    while default_cursor < defaults.1 {
                        let d = defaults.0 + 4 + default_cursor * 4;
                        if u24_at(b, d) + u32::from(b[d + 3]) < base {
                            default_cursor += 1;
                        } else {
                            break;
                        }
                    }
                    if default_cursor < defaults.1
                        && u24_at(b, defaults.0 + 4 + default_cursor * 4) <= base
                    {
                        return Err(fail(p, K::InvalidVariationMapping));
                    }
                }
            }
            if kind == 0 {
                defaults = (position, count);
            }
        }
    }
    spans.sort_unstable();
    for pair in spans.windows(2) {
        if pair[0].1 > pair[1].0 && pair[0] != pair[1] {
            return Err(fail(pair[1].0, K::OverlappingData));
        }
    }
    Ok(CffVariationSequencesV2 {
        table: b,
        records,
        values,
    })
}

#[cfg(test)]
#[path = "cff_v2_variations_tests.rs"]
mod tests;
