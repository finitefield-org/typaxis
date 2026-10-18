//! Validate supplemental format-14 coverage without expanding Unicode ranges.
//! The base cmap and subsequent two-scalar shaping remain separate owners.
use super::cmap_budget::VariationObserver;
use super::{
    CffCmapBudgetFailureV2, CffCmapBudgetResultV2, CffCmapBudgetV2, CffTableFailureKindV2 as K,
    CffTableFailureV2, CffVariationStorageV2,
};
enum Failure<E> {
    Native(CffTableFailureV2),
    Budget(CffCmapBudgetFailureV2<E>),
}
impl<E> From<CffTableFailureV2> for Failure<E> {
    fn from(e: CffTableFailureV2) -> Self {
        Self::Native(e)
    }
}
impl<E> From<CffCmapBudgetFailureV2<E>> for Failure<E> {
    fn from(e: CffCmapBudgetFailureV2<E>) -> Self {
        Self::Budget(e)
    }
}
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
    pub(super) fn visit_pairs<E>(
        &self,
        visit: impl FnMut(u32, u32, VariationCoverage) -> Result<(), E>,
    ) -> Result<(), E> {
        self.visit_pairs_with_work(&mut |_| Ok(()), visit)
    }
    pub(super) fn visit_pairs_with_work<E>(
        &self,
        work: &mut impl FnMut(usize) -> Result<(), E>,
        mut visit: impl FnMut(u32, u32, VariationCoverage) -> Result<(), E>,
    ) -> Result<(), E> {
        work(0)?;
        let b = self.table;
        for i in 0..self.records {
            let at = 10 + i * 11;
            work(at)?;
            let selector = u24_at(b, at);
            for (field, stride) in [(at + 3, 4), (at + 7, 5)] {
                work(field)?;
                let offset = u32_at(b, field) as usize;
                if offset == 0 {
                    continue;
                }
                for j in 0..u32_at(b, offset) as usize {
                    let at = offset + 4 + j * stride;
                    work(at)?;
                    let base = u24_at(b, at);
                    if stride == 4 {
                        for scalar in base..=base + u32::from(b[at + 3]) {
                            work(at)?;
                            visit(scalar, selector, VariationCoverage::Default)?;
                        }
                    } else {
                        visit(
                            base,
                            selector,
                            VariationCoverage::NonDefault(u16_at(b, at + 3)),
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
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
    pub(super) fn into_owned_with_budget<B: CffCmapBudgetV2>(
        self,
        budget: &mut B,
        table_start: usize,
    ) -> CffCmapBudgetResultV2<OwnedCffVariationSequencesV2, CffTableFailureV2, B::Error> {
        if !B::OBSERVED || !B::OBSERVE_VARIATIONS {
            return Ok(self.into_owned());
        }
        let mut observer = VariationObserver {
            budget,
            table_start,
        };
        observer.reserve(0, CffVariationStorageV2::OwnedBytes, 0, self.table.len())?;
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(self.table.len()).is_err() {
            return Ok(Err(fail(0, K::AllocationFailure)));
        }
        for (i, chunk) in self.table.chunks(256).enumerate() {
            observer.work(i * 256, chunk.len() as u64)?;
            bytes.extend_from_slice(chunk);
        }
        Ok(Ok(OwnedCffVariationSequencesV2 {
            bytes,
            records: self.records,
            values: self.values,
        }))
    }
    pub const fn selector_count(&self) -> usize {
        self.records
    }
    pub const fn declared_value_count(&self) -> u64 {
        self.values
    }
    pub fn coverage(&self, base: char, selector: char) -> VariationCoverage {
        self.coverage_with_work(
            base,
            selector,
            &mut || Ok::<_, std::convert::Infallible>(()),
        )
        .unwrap()
    }
    /// The same admitted format-14 search, with refusal before each visit.
    pub fn coverage_with_work<E>(
        &self,
        base: char,
        selector: char,
        work: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<VariationCoverage, E> {
        let b = self.table;
        let selector = selector as u32;
        let base = base as u32;
        let mut lo = 0;
        let mut hi = self.records;
        while lo < hi {
            work()?;
            let mid = lo + (hi - lo) / 2;
            if u24_at(b, 10 + mid * 11) < selector {
                lo = mid + 1
            } else {
                hi = mid
            }
        }
        work()?;
        if lo == self.records || u24_at(b, 10 + lo * 11) != selector {
            return Ok(VariationCoverage::Missing);
        }
        let at = 10 + lo * 11;
        let non = u32_at(b, at + 7) as usize;
        if non != 0 {
            let n = u32_at(b, non) as usize;
            let mut lo = 0;
            let mut hi = n;
            while lo < hi {
                work()?;
                let mid = lo + (hi - lo) / 2;
                if u24_at(b, non + 4 + mid * 5) < base {
                    lo = mid + 1
                } else {
                    hi = mid
                }
            }
            work()?;
            if lo < n && u24_at(b, non + 4 + lo * 5) == base {
                return Ok(VariationCoverage::NonDefault(u16_at(b, non + 7 + lo * 5)));
            }
        }
        let default = u32_at(b, at + 3) as usize;
        if default != 0 {
            let n = u32_at(b, default) as usize;
            let mut lo = 0;
            let mut hi = n;
            while lo < hi {
                work()?;
                let mid = lo + (hi - lo) / 2;
                let p = default + 4 + mid * 4;
                if u24_at(b, p) + u32::from(b[p + 3]) < base {
                    lo = mid + 1
                } else {
                    hi = mid
                }
            }
            work()?;
            if lo < n && u24_at(b, default + 4 + lo * 4) <= base {
                return Ok(VariationCoverage::Default);
            }
        }
        Ok(VariationCoverage::Missing)
    }
}

/// Find and validate the supplemental Unicode/platform-0 encoding-5 subtable.
/// Other encodings still require the base-cmap validator before font admission.
pub fn validate_cff_variation_sequences_v2(
    cmap: &[u8],
    glyph_count: u16,
) -> Result<Option<CffVariationSequencesV2<'_>>, CffTableFailureV2> {
    match validate_with_budget(
        cmap,
        glyph_count,
        &mut super::base_cmap_budget::NoBaseCmapBudget,
    ) {
        Ok(value) => value.map(|v| v.map(|(view, _)| view)),
        Err(_) => unreachable!("ordinary variation validation has no budget hooks"),
    }
}
pub(super) fn validate_with_budget<'a, B: CffCmapBudgetV2>(
    cmap: &'a [u8],
    glyph_count: u16,
    budget: &mut B,
) -> CffCmapBudgetResultV2<Option<(CffVariationSequencesV2<'a>, usize)>, CffTableFailureV2, B::Error>
{
    match validate_inner(
        cmap,
        glyph_count,
        &mut VariationObserver {
            budget,
            table_start: 0,
        },
    ) {
        Ok(value) => Ok(Ok(value)),
        Err(Failure::Native(e)) => Ok(Err(e)),
        Err(Failure::Budget(e)) => Err(e),
    }
}
#[cfg(test)]
fn parse_format14(
    b: &[u8],
    glyph_count: u16,
) -> Result<CffVariationSequencesV2<'_>, CffTableFailureV2> {
    match parse_format14_inner(
        b,
        glyph_count,
        &mut VariationObserver {
            budget: &mut super::base_cmap_budget::NoBaseCmapBudget,
            table_start: 0,
        },
    ) {
        Ok(value) => Ok(value),
        Err(Failure::Native(e)) => Err(e),
        Err(Failure::Budget(_)) => unreachable!("ordinary parser has no budget hooks"),
    }
}
fn validate_inner<'a, B: CffCmapBudgetV2>(
    cmap: &'a [u8],
    glyph_count: u16,
    observer: &mut VariationObserver<'_, B>,
) -> Result<Option<(CffVariationSequencesV2<'a>, usize)>, Failure<B::Error>> {
    observer.work(0, 1)?;
    if glyph_count == 0 {
        return Err(CffTableFailureV2 {
            table: *b"maxp",
            table_offset: 4,
            kind: K::InvalidMetricCount,
            limit: None,
            observed: None,
        }
        .into());
    }
    range(cmap, 0, 4)?;
    if u16_at(cmap, 0) != 0 {
        return Err((fail(0, K::UnsupportedVersion)).into());
    }
    let n = usize::from(u16_at(cmap, 2));
    range(cmap, 4, n * 8)?;
    let mut found = None;
    for i in 0..n {
        let at = 4 + i * 8;
        observer.work(at, 1)?;
        let position = u32_at(cmap, at + 4) as usize;
        if position < 4 + n * 8 {
            return Err((fail(at + 4, K::OverlappingData)).into());
        }
        range(cmap, position, 2)?;
        let unicode_uvs = u16_at(cmap, at) == 0 && u16_at(cmap, at + 2) == 5;
        let format14 = u16_at(cmap, position) == 14;
        if unicode_uvs != format14 {
            return Err((fail(at, K::InvalidCmapEncoding)).into());
        }
        if format14 {
            if found.is_some() {
                return Err((fail(at, K::InvalidCmapEncoding)).into());
            }
            range(cmap, position, 10)?;
            let size = u32_at(cmap, position + 2) as usize;
            let table = range(cmap, position, size)?;
            observer.table_start = position;
            let parsed =
                parse_format14_inner(table, glyph_count, observer).map_err(|e| match e {
                    Failure::Native(mut e) => {
                        e.table_offset += position;
                        Failure::Native(e)
                    }
                    other => other,
                })?;
            observer.table_start = 0;
            found = Some((parsed, position));
        }
    }
    Ok(found)
}
fn parse_format14_inner<'a, B: CffCmapBudgetV2>(
    b: &'a [u8],
    glyph_count: u16,
    observer: &mut VariationObserver<'_, B>,
) -> Result<CffVariationSequencesV2<'a>, Failure<B::Error>> {
    observer.work(0, 1)?;
    range(b, 0, 10)?;
    if u16_at(b, 0) != 14 || u32_at(b, 2) as usize != b.len() {
        return Err((fail(0, K::InvalidLength)).into());
    }
    let records = u32_at(b, 6) as usize;
    if records as u64 > MAX_SELECTORS {
        return Err((bound(6, MAX_SELECTORS, records as u64)).into());
    }
    let header_end = 10 + records * 11;
    range(b, 10, records * 11)?;
    let mut spans = Vec::new();
    observer.reserve(
        6,
        CffVariationStorageV2::PayloadSpans,
        records * 2,
        records * 2 * std::mem::size_of::<(usize, usize, u8)>(),
    )?;
    spans
        .try_reserve_exact(records * 2)
        .map_err(|_| fail(6, K::AllocationFailure))?;
    let mut previous = None;
    let mut values = 0u64;
    for i in 0..records {
        let at = 10 + i * 11;
        observer.work(at, 1)?;
        let selector = u24_at(b, at);
        if !(0xfe00..=0xfe0f).contains(&selector) && !(0xe0100..=0xe01ef).contains(&selector) {
            return Err((fail(at, K::InvalidVariationSelector)).into());
        }
        if previous.is_some_and(|old| old >= selector) {
            return Err((fail(at, K::InvalidVariationSelector)).into());
        }
        previous = Some(selector);
        let mut defaults = (0, 0usize);
        for (field, stride, kind) in [(at + 3, 4usize, 0u8), (at + 7, 5usize, 1u8)] {
            observer.work(field, 1)?;
            let position = u32_at(b, field) as usize;
            if position == 0 {
                continue;
            }
            if position < header_end {
                return Err((fail(field, K::OverlappingData)).into());
            }
            range(b, position, 4)?;
            let count = u32_at(b, position) as usize;
            // Every range declares >=1 value; reject before payload traversal.
            if count as u64 > MAX_VALUES - values {
                return Err((bound(position, MAX_VALUES, values + count as u64)).into());
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
                observer.work(p, 1)?;
                let base = u24_at(b, p);
                let end = if kind == 0 {
                    base + u32::from(b[p + 3])
                } else {
                    base
                };
                if !scalar_range(base, end) || prev_end.is_some_and(|old| old >= base) {
                    return Err((fail(p, K::InvalidUnicodeRange)).into());
                }
                prev_end = Some(end);
                let next = values
                    .checked_add(u64::from(end - base) + 1)
                    .ok_or_else(|| bound(p, MAX_VALUES, u64::MAX))?;
                if next > MAX_VALUES {
                    return Err((bound(p, MAX_VALUES, next)).into());
                }
                values = next;
                if kind == 1 {
                    if u16_at(b, p + 3) >= glyph_count {
                        return Err((fail(p + 3, K::GlyphOutOfRange)).into());
                    }
                    while default_cursor < defaults.1 {
                        let d = defaults.0 + 4 + default_cursor * 4;
                        observer.work(d, 1)?;
                        if u24_at(b, d) + u32::from(b[d + 3]) < base {
                            default_cursor += 1;
                        } else {
                            break;
                        }
                    }
                    observer.work(p, 1)?;
                    if default_cursor < defaults.1
                        && u24_at(b, defaults.0 + 4 + default_cursor * 4) <= base
                    {
                        return Err((fail(p, K::InvalidVariationMapping)).into());
                    }
                }
            }
            if kind == 0 {
                defaults = (position, count);
            }
        }
    }
    if B::OBSERVED && B::OBSERVE_VARIATIONS {
        sort_spans(&mut spans, observer)?;
    } else {
        spans.sort_unstable();
    }
    for pair in spans.windows(2) {
        observer.work(pair[1].0, 1)?;
        if pair[0].1 > pair[1].0 && pair[0] != pair[1] {
            return Err((fail(pair[1].0, K::OverlappingData)).into());
        }
    }
    Ok(CffVariationSequencesV2 {
        table: b,
        records,
        values,
    })
}

fn sort_spans<B: CffCmapBudgetV2>(
    spans: &mut [(usize, usize, u8)],
    observer: &mut VariationObserver<'_, B>,
) -> Result<(), CffCmapBudgetFailureV2<B::Error>> {
    fn sift<B: CffCmapBudgetV2>(
        a: &mut [(usize, usize, u8)],
        mut root: usize,
        end: usize,
        o: &mut VariationObserver<'_, B>,
    ) -> Result<(), CffCmapBudgetFailureV2<B::Error>> {
        while root < end / 2 {
            let mut child = 2 * root + 1;
            if child + 1 < end {
                o.work(a[child].0, 1)?;
                if a[child] < a[child + 1] {
                    child += 1;
                }
            }
            o.work(a[root].0, 1)?;
            if a[root] >= a[child] {
                break;
            }
            o.work(a[root].0, 1)?;
            a.swap(root, child);
            root = child;
        }
        Ok(())
    }
    observer.work(0, 1)?;
    for root in (0..spans.len() / 2).rev() {
        sift(spans, root, spans.len(), observer)?;
    }
    for end in (1..spans.len()).rev() {
        observer.work(spans[0].0, 1)?;
        spans.swap(0, end);
        sift(spans, 0, end, observer)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "cff_v2_variations_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cff_v2_variation_budget_tests.rs"]
mod budget_tests;
