//! Base and supplemental coverage owner for CFF /2 admission. This does not
//! replace text with glyphs: the shaper must retain the original scalar pair.
use super::*;

#[derive(Debug)]
pub struct CffCmapV2 {
    base: Vec<(u32, u16)>,
    variations: Option<variations::OwnedCffVariationSequencesV2>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffCmapFailureV2 {
    Base {
        kind: Cff1Error,
        context: FontFailureContext,
    },
    Variation(CffTableFailureV2),
    Storage(CffTableFailureV2),
}
impl std::fmt::Display for CffCmapFailureV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFF /2 cmap {self:?}")
    }
}
impl std::error::Error for CffCmapFailureV2 {}

fn is_selector(c: char) -> bool {
    matches!(c as u32, 0xfe00..=0xfe0f | 0xe0100..=0xe01ef)
}
impl CffCmapV2 {
    pub(super) fn base_map(&self) -> &[(u32, u16)] {
        &self.base
    }
    pub fn base_mapping_count(&self) -> usize {
        self.base.len()
    }
    pub fn variation_sequences(&self) -> Option<CffVariationSequencesV2<'_>> {
        self.variations.as_ref().map(|v| v.view())
    }
    /// Coverage only. An isolated selector, unsupported pair, absent default
    /// base, or .notdef is missing coverage; no fallback drops the selector.
    pub fn glyph_for_sequence(&self, base: char, selector: Option<char>) -> Option<u16> {
        self.glyph_for_sequence_with_work(base, selector, &mut || {
            Ok::<_, std::convert::Infallible>(())
        })
        .unwrap()
    }
    /// Coverage lookup with a fallible visit before each search comparison.
    /// No source mappings, callbacks or receipt authority are retained.
    pub fn glyph_for_sequence_with_work<E>(
        &self,
        base: char,
        selector: Option<char>,
        work: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Option<u16>, E> {
        work()?;
        if is_selector(base) {
            return Ok(None);
        }
        let gid = match selector {
            None => self.base_glyph_with_work(base, work)?,
            Some(selector) => match self.variation_sequences() {
                None => return Ok(None),
                Some(variation) => match variation.coverage_with_work(base, selector, work)? {
                    VariationCoverage::Default => self.base_glyph_with_work(base, work)?,
                    VariationCoverage::NonDefault(gid) => gid,
                    VariationCoverage::Missing => return Ok(None),
                },
            },
        };
        Ok((gid != 0).then_some(gid))
    }
    fn base_glyph_with_work<E>(
        &self,
        base: char,
        work: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<u16, E> {
        let mut lo = 0;
        let mut hi = self.base.len();
        while lo < hi {
            work()?;
            let mid = lo + (hi - lo) / 2;
            match self.base[mid].0.cmp(&(base as u32)) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return Ok(self.base[mid].1),
            }
        }
        Ok(0)
    }
}

pub fn validate_cff_cmap_v2(bytes: &[u8], glyph_count: u16) -> Result<CffCmapV2, CffCmapFailureV2> {
    match validate_cff_cmap_v2_with_base_cmap_budget(
        bytes,
        glyph_count,
        &mut super::base_cmap_budget::NoBaseCmapBudget,
    ) {
        Ok(value) => value,
        Err(failure) => match failure.error {},
    }
}
/// Only base-cmap decoding/merge work and mapping vectors are observed here.
/// Outer Err is budget refusal; the inner result preserves semantic errors.
pub fn validate_cff_cmap_v2_with_base_cmap_budget<B: CffBaseCmapBudgetV2>(
    bytes: &[u8],
    glyph_count: u16,
    budget: &mut B,
) -> CffBaseCmapBudgetResultV2<CffCmapV2, CffCmapFailureV2, B::Error> {
    validate_cff_cmap_v2_with_cmap_budget(
        bytes,
        glyph_count,
        &mut super::cmap_budget::BaseOnly(budget),
    )
    .map_err(CffCmapBudgetFailureV2::base_only)
}
/// Observe base and supplemental validation, temporary spans, usable-pair scans
/// and the owned format-14 copy. Other font admission phases are independent.
pub fn validate_cff_cmap_v2_with_cmap_budget<B: CffCmapBudgetV2>(
    bytes: &[u8],
    glyph_count: u16,
    budget: &mut B,
) -> CffCmapBudgetResultV2<CffCmapV2, CffCmapFailureV2, B::Error> {
    let variations = match variations::validate_with_budget(bytes, glyph_count, budget)? {
        Ok(value) => value,
        Err(e) => return Ok(Err(CffCmapFailureV2::Variation(e))),
    };
    let mut context = FontFailureContext::new(0);
    context.phase = FontFailurePhase::Cmap;
    context.reason = FontFailureReason::InvalidTable;
    context.table_tag = Some(*b"cmap");
    let base = match base::parse_with_budget(
        bytes,
        glyph_count,
        &mut context,
        variations.is_some(),
        budget,
    )
    .map_err(CffCmapBudgetFailureV2::Base)?
    {
        Ok(value) => value,
        Err(e) => return Ok(Err(e)),
    };
    if base.is_empty() {
        let mut usable = false;
        if let Some((v, table_start)) = &variations {
            if B::OBSERVED && B::OBSERVE_VARIATIONS {
                let mut observer = super::cmap_budget::VariationObserver {
                    budget,
                    table_start: *table_start,
                };
                v.visit_pairs_with_work(&mut |at| observer.work(at, 1), |_, _, coverage| {
                    usable |= matches!(coverage,VariationCoverage::NonDefault(gid) if gid!=0);
                    Ok(())
                })?;
            } else {
                v.visit_pairs::<std::convert::Infallible>(|_, _, coverage| {
                    usable |= matches!(coverage,VariationCoverage::NonDefault(gid) if gid!=0);
                    Ok(())
                })
                .unwrap();
            }
        }
        if !usable {
            return Ok(Err(CffCmapFailureV2::Base {
                kind: Cff1Error::InvalidCmap,
                context,
            }));
        }
    }
    let variations = if let Some((v, table_start)) = variations {
        match v.into_owned_with_budget(budget, table_start)? {
            Ok(value) => Some(value),
            Err(e) => return Ok(Err(CffCmapFailureV2::Variation(e))),
        }
    } else {
        None
    };
    Ok(Ok(CffCmapV2 { base, variations }))
}

#[path = "cff_v2_base_cmap.rs"]
mod base;

#[cfg(test)]
#[path = "cff_v2_cmap_tests.rs"]
mod tests;
