//! Bounded validation of optional CFF vertical tables. Horizontal layout must
//! not apply these origins or advances to glyph positions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffTableFailureKindV2 {
    InvalidLength,
    UnsupportedVersion,
    InvalidMetricCount,
    InvalidReservedField,
    InvalidGlyphOrder,
    GlyphOutOfRange,
    MissingTablePair,
    InvalidCmapEncoding,
    InvalidVariationSelector,
    InvalidUnicodeRange,
    InvalidVariationMapping,
    OverlappingData,
    UnsupportedComplexity,
    AllocationFailure,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CffTableFailureV2 {
    pub table: [u8; 4],
    pub table_offset: usize,
    pub kind: CffTableFailureKindV2,
    pub limit: Option<u64>,
    pub observed: Option<u64>,
}
impl std::fmt::Display for CffTableFailureV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CFF /2 table {:?} at {}: {:?}",
            self.table, self.table_offset, self.kind
        )
    }
}
impl std::error::Error for CffTableFailureV2 {}
use CffTableFailureKindV2 as K;
fn failure(table: [u8; 4], offset: usize, kind: K) -> CffTableFailureV2 {
    CffTableFailureV2 {
        table,
        table_offset: offset,
        kind,
        limit: None,
        observed: None,
    }
}
fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}
fn i16_at(bytes: &[u8], at: usize) -> i16 {
    i16::from_be_bytes([bytes[at], bytes[at + 1]])
}

/// Borrows the exact validated table slices. No dense glyph-metric expansion or
/// font normalization is required to inspect any glyph in the verified domain.
#[derive(Debug)]
pub struct CffVerticalMetricsV2<'a> {
    glyph_count: u16,
    vorg: Option<&'a [u8]>,
    vhea: Option<&'a [u8]>,
    vmtx: Option<&'a [u8]>,
    long_metrics: u16,
}
impl CffVerticalMetricsV2<'_> {
    pub const fn long_metric_count(&self) -> u16 {
        self.long_metrics
    }
    pub fn origin_record_count(&self) -> u16 {
        self.vorg.map_or(0, |v| u16_at(v, 6))
    }
    pub fn header_version(&self) -> Option<u32> {
        self.vhea
            .map(|v| u32::from_be_bytes(v[..4].try_into().unwrap()))
    }
    pub fn vertical_origin(&self, gid: u16) -> Option<i16> {
        if gid >= self.glyph_count {
            return None;
        }
        let v = self.vorg?;
        let mut low = 0usize;
        let mut high = usize::from(u16_at(v, 6));
        while low < high {
            let mid = low + (high - low) / 2;
            match u16_at(v, 8 + 4 * mid).cmp(&gid) {
                std::cmp::Ordering::Equal => return Some(i16_at(v, 10 + 4 * mid)),
                std::cmp::Ordering::Less => low = mid + 1,
                std::cmp::Ordering::Greater => high = mid,
            }
        }
        Some(i16_at(v, 4))
    }
    /// (advance height, top side bearing), in font units. The trailing short
    /// bearings use the last long metric's advance, as specified by OpenType.
    pub fn glyph_metric(&self, gid: u16) -> Option<(u16, i16)> {
        if gid >= self.glyph_count {
            return None;
        }
        let v = self.vmtx?;
        let gid = usize::from(gid);
        let n = usize::from(self.long_metrics);
        let advance = u16_at(v, 4 * gid.min(n - 1));
        let bearing = i16_at(
            v,
            if gid < n {
                4 * gid + 2
            } else {
                4 * n + 2 * (gid - n)
            },
        );
        Some((advance, bearing))
    }
}

/// Validate vertical table structure for a known, nonzero maxp glyph count.
/// This is an input to /2 sfnt admission, not a font-admission receipt itself.
pub fn validate_cff_vertical_metrics_v2<'a>(
    glyph_count: u16,
    vorg: Option<&'a [u8]>,
    vhea: Option<&'a [u8]>,
    vmtx: Option<&'a [u8]>,
) -> Result<CffVerticalMetricsV2<'a>, CffTableFailureV2> {
    match with_budget(
        glyph_count,
        vorg,
        vhea,
        vmtx,
        &mut super::admission_budget::UnobservedAdmission,
    ) {
        Ok(native) => native,
        Err(f) => match f.error {},
    }
}
#[derive(Debug)]
enum VerticalFailure<E> {
    Native(CffTableFailureV2),
    Budget(super::CffTableAdmissionBudgetFailureV2<E>),
}
impl<E> From<CffTableFailureV2> for VerticalFailure<E> {
    fn from(e: CffTableFailureV2) -> Self {
        Self::Native(e)
    }
}
impl<E> From<super::CffTableAdmissionBudgetFailureV2<E>> for VerticalFailure<E> {
    fn from(e: super::CffTableAdmissionBudgetFailureV2<E>) -> Self {
        Self::Budget(e)
    }
}
fn charge<B: super::CffAdmissionBudgetV2>(
    budget: &mut B,
    tag: Option<[u8; 4]>,
    offset: Option<usize>,
    record: bool,
    n: u64,
) -> Result<(), super::CffTableAdmissionBudgetFailureV2<B::Error>> {
    use super::super::font_tables::{point, work};
    let stage = if record {
        super::CffTableAdmissionStageV2::VerticalRecord
    } else {
        super::CffTableAdmissionStageV2::VerticalHeader
    };
    work(budget, point(stage, tag, offset), n)
}
type VerticalBudgetResult<'a, E> = Result<
    Result<CffVerticalMetricsV2<'a>, CffTableFailureV2>,
    super::CffTableAdmissionBudgetFailureV2<E>,
>;
pub(in crate::cff) fn with_budget<'a, B: super::CffAdmissionBudgetV2>(
    glyph_count: u16,
    vorg: Option<&'a [u8]>,
    vhea: Option<&'a [u8]>,
    vmtx: Option<&'a [u8]>,
    budget: &mut B,
) -> VerticalBudgetResult<'a, B::Error> {
    match kernel(glyph_count, vorg, vhea, vmtx, budget) {
        Ok(v) => Ok(Ok(v)),
        Err(VerticalFailure::Native(e)) => Ok(Err(e)),
        Err(VerticalFailure::Budget(e)) => Err(e),
    }
}
fn kernel<'a, B: super::CffAdmissionBudgetV2>(
    glyph_count: u16,
    vorg: Option<&'a [u8]>,
    vhea: Option<&'a [u8]>,
    vmtx: Option<&'a [u8]>,
    budget: &mut B,
) -> Result<CffVerticalMetricsV2<'a>, VerticalFailure<B::Error>> {
    charge(budget, None, None, false, 16)?;
    if glyph_count == 0 {
        return Err(failure(*b"maxp", 4, K::InvalidMetricCount).into());
    }
    if vhea.is_some() != vmtx.is_some() {
        return Err(failure(
            if vhea.is_none() { *b"vhea" } else { *b"vmtx" },
            0,
            K::MissingTablePair,
        )
        .into());
    }
    if let Some(v) = vorg {
        charge(budget, Some(*b"VORG"), Some(0), false, 16)?;
        if v.len() < 8 {
            return Err(failure(*b"VORG", v.len(), K::InvalidLength).into());
        }
        if v[..4] != [0, 1, 0, 0] {
            return Err(failure(*b"VORG", 0, K::UnsupportedVersion).into());
        }
        let count = usize::from(u16_at(v, 6));
        if count > usize::from(glyph_count) {
            return Err(failure(*b"VORG", 6, K::InvalidMetricCount).into());
        }
        if v.len() != 8 + count * 4 {
            return Err(failure(*b"VORG", 6, K::InvalidLength).into());
        }
        let mut previous = None;
        for i in 0..count {
            let at = 8 + i * 4;
            charge(budget, Some(*b"VORG"), Some(at), true, 1)?;
            let gid = u16_at(v, at);
            if gid >= glyph_count {
                return Err(failure(*b"VORG", at, K::GlyphOutOfRange).into());
            }
            if previous.is_some_and(|old| old >= gid) {
                return Err(failure(*b"VORG", at, K::InvalidGlyphOrder).into());
            }
            previous = Some(gid);
        }
    }
    let mut long_metrics = 0;
    if let (Some(h), Some(m)) = (vhea, vmtx) {
        charge(budget, Some(*b"vhea"), Some(0), false, 16)?;
        if h.len() != 36 {
            return Err(failure(*b"vhea", h.len().min(36), K::InvalidLength).into());
        }
        let version = u32::from_be_bytes(h[..4].try_into().unwrap());
        if !matches!(version, 0x00010000 | 0x00011000) {
            return Err(failure(*b"vhea", 0, K::UnsupportedVersion).into());
        }
        if version == 0x00010000 && u16_at(h, 8) != 0 {
            return Err(failure(*b"vhea", 8, K::InvalidReservedField).into());
        }
        for at in (24..=32).step_by(2) {
            charge(budget, Some(*b"vhea"), Some(at), true, 1)?;
            if u16_at(h, at) != 0 {
                return Err(failure(*b"vhea", at, K::InvalidReservedField).into());
            }
        }
        long_metrics = u16_at(h, 34);
        if long_metrics == 0 || long_metrics > glyph_count {
            return Err(failure(*b"vhea", 34, K::InvalidMetricCount).into());
        }
        let n = usize::from(long_metrics);
        let expected = 4 * n + 2 * (usize::from(glyph_count) - n);
        charge(budget, Some(*b"vmtx"), Some(0), false, 1)?;
        if m.len() != expected {
            return Err(failure(*b"vmtx", m.len().min(expected), K::InvalidLength).into());
        }
    }
    Ok(CffVerticalMetricsV2 {
        glyph_count,
        vorg,
        vhea,
        vmtx,
        long_metrics,
    })
}

#[cfg(test)]
#[path = "cff_v2_vertical_tests.rs"]
mod tests;
