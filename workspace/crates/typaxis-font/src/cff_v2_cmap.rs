//! Base and supplemental coverage owner for CFF /2 admission. This does not
//! replace text with glyphs: the shaper must retain the original scalar pair.
use super::*;

#[derive(Debug)]
pub struct CffCmapV2 {
    base: BTreeMap<u32, u16>,
    variations: Option<variations::OwnedCffVariationSequencesV2>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffCmapFailureV2 {
    Base {
        kind: Cff1Error,
        context: FontFailureContext,
    },
    Variation(CffTableFailureV2),
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
    pub fn base_mapping_count(&self) -> usize {
        self.base.len()
    }
    pub fn variation_sequences(&self) -> Option<CffVariationSequencesV2<'_>> {
        self.variations.as_ref().map(|v| v.view())
    }
    /// Coverage only. An isolated selector, unsupported pair, absent default
    /// base, or .notdef is missing coverage; no fallback drops the selector.
    pub fn glyph_for_sequence(&self, base: char, selector: Option<char>) -> Option<u16> {
        if is_selector(base) {
            return None;
        }
        let gid = match selector {
            None => self.base.get(&(base as u32)).copied()?,
            Some(selector) => match self.variation_sequences()?.coverage(base, selector) {
                VariationCoverage::Default => self.base.get(&(base as u32)).copied()?,
                VariationCoverage::NonDefault(gid) => gid,
                VariationCoverage::Missing => return None,
            },
        };
        (gid != 0).then_some(gid)
    }
}

pub fn validate_cff_cmap_v2(bytes: &[u8], glyph_count: u16) -> Result<CffCmapV2, CffCmapFailureV2> {
    let variations = validate_cff_variation_sequences_v2(bytes, glyph_count)
        .map_err(CffCmapFailureV2::Variation)?;
    let mut context = FontFailureContext::new(0);
    context.phase = FontFailurePhase::Cmap;
    context.reason = FontFailureReason::InvalidTable;
    context.table_tag = Some(*b"cmap");
    let base = parse_base_cmap(bytes, glyph_count, &mut context, variations.is_some())
        .map_err(|kind| CffCmapFailureV2::Base { kind, context })?;
    let variations = variations
        .map(|v| v.into_owned())
        .transpose()
        .map_err(CffCmapFailureV2::Variation)?;
    Ok(CffCmapV2 { base, variations })
}

#[cfg(test)]
#[path = "cff_v2_cmap_tests.rs"]
mod tests;
