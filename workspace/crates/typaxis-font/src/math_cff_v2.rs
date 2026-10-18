//! Borrowed MATH access backed exclusively by a sealed CFF /2 admission.
//! Selected outlines are supplied by the command's existing Type2 cache.
use super::*;
use crate::{Cff1AdmissionV2, Cff1Error, Cff1SubsetSessionV2, CffSelectionFailureV2};

pub const CFF_MATH_TABLE_FINGERPRINT_ALGORITHM_V2: &str = "typaxis.opentype-math-table/cff-v2/1";
#[derive(Clone, Copy, Debug)]
pub struct CffMathFontFaceV2<'a> {
    admission: &'a Cff1AdmissionV2,
    math: &'a [u8],
    constants: MathFontConstants,
    info: usize,
    variants: usize,
    ascent: i16,
    descent: i16,
    fingerprint: [u8; 32],
}
#[derive(Clone, Copy, Debug)]
pub struct CffMathGlyphMetricsV2 {
    admission: [u8; 32],
    gid: OriginalGlyphId,
    height: u16,
}
impl CffMathFontFaceV2<'_> {
    pub fn admission(&self) -> &Cff1AdmissionV2 {
        self.admission
    }
    pub fn units_per_em(self) -> u16 {
        self.admission.units_per_em()
    }
    pub fn constants(self) -> MathFontConstants {
        self.constants
    }
    pub fn ascent(self) -> i16 {
        self.ascent
    }
    pub fn descent(self) -> i16 {
        self.descent
    }
    pub fn math_table_fingerprint(self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn require_metrics(
        self,
        gid: OriginalGlyphId,
        metrics: CffMathGlyphMetricsV2,
    ) -> Result<u16, MathFontError> {
        if metrics.admission != self.admission.fingerprint() || metrics.gid != gid {
            return Err(MathFontError::MalformedTable("CFF "));
        }
        Ok(metrics.height)
    }
    pub fn glyph_id_with_work<E>(
        self,
        ch: char,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Result<OriginalGlyphId, MathFontError>, E> {
        self.admission
            .cmap()
            .glyph_for_sequence_with_work(ch, None, &mut || work(64))
            .map(|gid| {
                gid.map(OriginalGlyphId::new)
                    .ok_or(MathFontError::MissingGlyph(ch))
            })
    }
    pub fn advance_width_with_work<E>(
        self,
        gid: OriginalGlyphId,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Result<u16, MathFontError>, E> {
        work(64)?;
        Ok(self
            .admission
            .horizontal_metric(gid.get())
            .map(|m| m.0)
            .filter(|v| *v != 0)
            .ok_or(MathFontError::MalformedTable("hmtx")))
    }
    pub fn italic_correction_with_work<E>(
        self,
        gid: OriginalGlyphId,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Result<i16, MathFontError>, E> {
        work(2048)?;
        Ok((|| {
            self.require_gid(gid)?;
            let offset = usize::from(u16_at(self.math, self.info)?);
            if offset == 0 {
                return Ok(0);
            }
            let info = checked_relative(self.info, offset, self.math.len())?;
            let coverage =
                checked_relative(info, usize::from(u16_at(self.math, info)?), self.math.len())?;
            let Some(index) = coverage_binary(self.math, coverage, gid.get())? else {
                return Ok(0);
            };
            if index >= usize::from(u16_at(self.math, info + 2)?) {
                return Err(MathFontError::MalformedTable("MATH"));
            }
            read_i16(self.math, info + 4 + index * 4).ok_or(MathFontError::MalformedTable("MATH"))
        })())
    }
    /// Lower bound preserves the first source record among tied advances.
    pub fn vertical_variant_with_work<E>(
        self,
        gid: OriginalGlyphId,
        minimum: u16,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Result<(OriginalGlyphId, u16), MathFontError>, E> {
        work(2048)?;
        Ok((|| {
            self.require_gid(gid)?;
            if minimum == 0 {
                return Err(MathFontError::MalformedTable("MATH"));
            }
            let missing = MathFontError::MissingVerticalVariant(gid.get());
            let offset = usize::from(u16_at(self.math, self.variants + 2)?);
            if offset == 0 {
                return Err(missing);
            }
            let coverage = checked_relative(self.variants, offset, self.math.len())?;
            let index = coverage_binary(self.math, coverage, gid.get())?.ok_or(missing)?;
            if index >= usize::from(u16_at(self.math, self.variants + 6)?) {
                return Err(MathFontError::MalformedTable("MATH"));
            }
            let construction = checked_relative(
                self.variants,
                usize::from(u16_at(self.math, self.variants + 10 + index * 2)?),
                self.math.len(),
            )?;
            let count = usize::from(u16_at(self.math, construction + 2)?);
            let (mut lo, mut hi) = (0, count);
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                if u16_at(self.math, construction + 6 + mid * 4)? < minimum {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            if lo == count {
                return Err(missing);
            }
            let gid = OriginalGlyphId::new(u16_at(self.math, construction + 4 + lo * 4)?);
            self.require_gid(gid)?;
            Ok((gid, u16_at(self.math, construction + 6 + lo * 4)?))
        })())
    }
    fn require_gid(self, gid: OriginalGlyphId) -> Result<(), MathFontError> {
        if gid.get() >= self.admission.glyph_count() {
            Err(MathFontError::MalformedTable("MATH"))
        } else {
            Ok(())
        }
    }
}
impl<'a> CffMathFontFaceV2<'a> {
    pub fn from_admission_with_work<E>(
        admission: &'a Cff1AdmissionV2,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Result<Self, MathFontError>, E> {
        // Two bounded directory searches plus fixed constants/metric reads.
        work(admission.table_count() as u64 * 2 + 1024)?;
        let math = match admission.table_bytes(b"MATH") {
            Ok(v) => v,
            Err(_) => return Ok(Err(MathFontError::MissingTable("MATH"))),
        };
        let hhea = match admission.table_bytes(b"hhea") {
            Ok(v) => v,
            Err(_) => return Ok(Err(MathFontError::MissingTable("hhea"))),
        };
        work(math.len() as u64 * 2 + 256)?;
        Ok((|| {
            // Admission has already validated the complete /2 MATH grammar.
            // These fixed reads reuse that authority, without another full walk.
            let at = usize::from(u16_at(math, 4)?);
            let value = |index: usize| {
                read_i16(math, at + 8 + index * 4).ok_or(MathFontError::MalformedTable("MATH"))
            };
            let constants = MathFontConstants {
                script_percent_scale_down: u16_at(math, at)?,
                script_script_percent_scale_down: u16_at(math, at + 2)?,
                display_operator_min_height: u16_at(math, at + 6)?,
                axis_height: value(1)?,
                subscript_shift_down: value(4)?,
                subscript_top_max: value(5)?,
                subscript_baseline_drop_min: value(6)?,
                superscript_shift_up: value(7)?,
                superscript_shift_up_cramped: value(8)?,
                superscript_bottom_min: value(9)?,
                superscript_baseline_drop_max: value(10)?,
                sub_superscript_gap_min: value(11)?,
                superscript_bottom_max_with_subscript: value(12)?,
                space_after_script: value(13)?,
                fraction_numerator_shift_up: value(28)?,
                fraction_numerator_display_shift_up: value(29)?,
                fraction_denominator_shift_down: value(30)?,
                fraction_denominator_display_shift_down: value(31)?,
                fraction_numerator_gap_min: value(32)?,
                fraction_numerator_display_gap_min: value(33)?,
                fraction_rule_thickness: value(34)?,
                fraction_denominator_gap_min: value(35)?,
                fraction_denominator_display_gap_min: value(36)?,
                radical_vertical_gap: value(45)?,
                radical_display_vertical_gap: value(46)?,
                radical_rule_thickness: value(47)?,
            };
            let mut identity = [0u8; 128];
            let domain = CFF_MATH_TABLE_FINGERPRINT_ALGORITHM_V2.as_bytes();
            let n = domain.len();
            identity[..n].copy_from_slice(domain);
            identity[n..n + 32].copy_from_slice(&admission.fingerprint());
            identity[n + 32..n + 64].copy_from_slice(&sha256(math));
            identity[n + 64..n + 66].copy_from_slice(&admission.units_per_em().to_be_bytes());
            Ok(Self {
                admission,
                math,
                constants,
                info: usize::from(u16_at(math, 6)?),
                variants: usize::from(u16_at(math, 8)?),
                ascent: read_i16(hhea, 4).ok_or(MathFontError::MalformedTable("hhea"))?,
                descent: read_i16(hhea, 6).ok_or(MathFontError::MalformedTable("hhea"))?,
                fingerprint: sha256(&identity[..n + 66]),
            })
        })())
    }
}
impl Cff1SubsetSessionV2 {
    /// Evaluate only this selected source GID, in the same cumulative cache
    /// later used for PDF subsetting. All storage/lookup/execution is prepaid.
    pub fn math_glyph_metrics_with_charge(
        &mut self,
        admission: &Cff1AdmissionV2,
        gid: OriginalGlyphId,
        charge: &mut dyn FnMut(usize, usize, usize) -> Result<(), Cff1Error>,
    ) -> Result<CffMathGlyphMetricsV2, CffSelectionFailureV2> {
        self.prepare_math_glyph_with_charge(admission, gid, charge)?;
        let depth =
            (usize::BITS - self.cached_glyph_count().saturating_add(1).leading_zeros()) as usize;
        charge(0, 0, depth * 12 * 32 + 128)?;
        let glyph = self
            .evaluated_glyph(admission, gid)?
            .ok_or(Cff1Error::ReceiptMismatch)?;
        let bounds = glyph
            .control_bounds()
            .ok_or(Cff1Error::InvalidSelectedGlyph)?;
        let height = i64::from(bounds[3]) - i64::from(bounds[1]);
        if bounds[0] >= bounds[2] || height <= 0 {
            return Err(Cff1Error::InvalidSelectedGlyph.into());
        }
        let height =
            u16::try_from((height + 65535) / 65536).map_err(|_| Cff1Error::InvalidSelectedGlyph)?;
        Ok(CffMathGlyphMetricsV2 {
            admission: admission.fingerprint(),
            gid,
            height,
        })
    }
}
fn u16_at(bytes: &[u8], at: usize) -> Result<u16, MathFontError> {
    read_u16(bytes, at).ok_or(MathFontError::MalformedTable("MATH"))
}
fn coverage_binary(bytes: &[u8], at: usize, gid: u16) -> Result<Option<usize>, MathFontError> {
    let format = u16_at(bytes, at)?;
    let count = usize::from(u16_at(bytes, at + 2)?);
    let stride = match format {
        1 => 2,
        2 => 6,
        _ => return Err(MathFontError::MalformedTable("MATH")),
    };
    let (mut lo, mut hi) = (0, count);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let first = u16_at(bytes, at + 4 + mid * stride)?;
        if first <= gid {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return Ok(None);
    }
    let index = lo - 1;
    let record = at + 4 + index * stride;
    if format == 1 {
        return Ok((u16_at(bytes, record)? == gid).then_some(index));
    }
    let first = u16_at(bytes, record)?;
    let last = u16_at(bytes, record + 2)?;
    if gid > last {
        return Ok(None);
    }
    Ok(Some(
        usize::from(u16_at(bytes, record + 4)?) + usize::from(gid - first),
    ))
}

#[cfg(test)]
#[path = "math_cff_v2_tests.rs"]
mod tests;
