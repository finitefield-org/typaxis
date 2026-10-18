//! Sealed CFF math metadata plus caller-owned, cumulative Type2 access.
use super::*;
use typaxis_font::{Cff1AdmissionV2, CffMathFontFaceV2, CffMathGlyphMetricsV2, MathFontConstants};

pub trait CffMathProviderV2 {
    type Error;
    fn work(&mut self, amount: u64) -> Result<(), Self::Error>;
    fn glyph_metrics(
        &mut self,
        admission: &Cff1AdmissionV2,
        gid: OriginalGlyphId,
    ) -> Result<CffMathGlyphMetricsV2, Self::Error>;
}
#[derive(Debug)]
pub enum CffMathComputationErrorV2<E> {
    Access(E),
    Computation(MathComputationError),
}
impl<E: std::fmt::Debug> std::fmt::Display for CffMathComputationErrorV2<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFF /2 math: {self:?}")
    }
}
impl<E: std::error::Error + 'static> std::error::Error for CffMathComputationErrorV2<E> {}

pub fn compute_math_cff_v2<P: CffMathProviderV2>(
    parsed: &ParsedMathReceipt,
    face: CffMathFontFaceV2<'_>,
    input: MathComputationInput,
    provider: &mut P,
) -> Result<MathComputationReceipt, CffMathComputationErrorV2<P::Error>> {
    let mut access = CffAccess {
        face,
        provider,
        failure: None,
    };
    let result = compute_math_kernel(parsed, &mut access, input);
    match access.failure {
        Some(error) => Err(CffMathComputationErrorV2::Access(error)),
        None => result.map_err(CffMathComputationErrorV2::Computation),
    }
}
/// The legacy wrapper keeps its public signature and native results. This
/// private interface cannot be implemented by an untrusted geometry provider.
pub(super) trait FontAccess {
    fn units_per_em(&self) -> u16;
    fn constants(&self) -> MathFontConstants;
    fn ascent(&self) -> i16;
    fn descent(&self) -> i16;
    fn math_table_fingerprint(&self) -> [u8; 32];
    fn glyph_id(&mut self, value: char) -> Result<OriginalGlyphId, MathFontError>;
    fn advance_width(&mut self, gid: OriginalGlyphId) -> Result<u16, MathFontError>;
    fn glyph_height(&mut self, gid: OriginalGlyphId) -> Result<u16, MathFontError>;
    fn italic_correction(&mut self, gid: OriginalGlyphId) -> Result<i16, MathFontError>;
    fn vertical_variant(
        &mut self,
        gid: OriginalGlyphId,
        minimum: u16,
    ) -> Result<(OriginalGlyphId, u16), MathFontError>;
}
impl FontAccess for MathFontFace<'_> {
    fn units_per_em(&self) -> u16 {
        MathFontFace::units_per_em(*self)
    }
    fn constants(&self) -> MathFontConstants {
        MathFontFace::constants(*self)
    }
    fn ascent(&self) -> i16 {
        MathFontFace::ascent(*self)
    }
    fn descent(&self) -> i16 {
        MathFontFace::descent(*self)
    }
    fn math_table_fingerprint(&self) -> [u8; 32] {
        MathFontFace::math_table_fingerprint(*self)
    }
    fn glyph_id(&mut self, v: char) -> Result<OriginalGlyphId, MathFontError> {
        MathFontFace::glyph_id(*self, v)
    }
    fn advance_width(&mut self, g: OriginalGlyphId) -> Result<u16, MathFontError> {
        MathFontFace::advance_width(*self, g)
    }
    fn glyph_height(&mut self, g: OriginalGlyphId) -> Result<u16, MathFontError> {
        MathFontFace::glyph_height(*self, g)
    }
    fn italic_correction(&mut self, g: OriginalGlyphId) -> Result<i16, MathFontError> {
        MathFontFace::italic_correction(*self, g)
    }
    fn vertical_variant(
        &mut self,
        g: OriginalGlyphId,
        m: u16,
    ) -> Result<(OriginalGlyphId, u16), MathFontError> {
        MathFontFace::vertical_variant(*self, g, m)
    }
}
struct CffAccess<'a, 'p, P: CffMathProviderV2> {
    face: CffMathFontFaceV2<'a>,
    provider: &'p mut P,
    failure: Option<P::Error>,
}
impl<P: CffMathProviderV2> CffAccess<'_, '_, P> {
    fn access<T>(
        &mut self,
        operation: impl FnOnce(
            CffMathFontFaceV2<'_>,
            &mut P,
        ) -> Result<Result<T, MathFontError>, P::Error>,
    ) -> Result<T, MathFontError> {
        match operation(self.face, self.provider) {
            Ok(native) => native,
            Err(error) => {
                self.failure = Some(error);
                Err(MathFontError::AllocationFailure)
            }
        }
    }
    fn metrics(&mut self, gid: OriginalGlyphId) -> Result<u16, MathFontError> {
        match self.provider.glyph_metrics(self.face.admission(), gid) {
            Ok(metrics) => self.face.require_metrics(gid, metrics),
            Err(error) => {
                self.failure = Some(error);
                Err(MathFontError::AllocationFailure)
            }
        }
    }
}
impl<P: CffMathProviderV2> FontAccess for CffAccess<'_, '_, P> {
    fn units_per_em(&self) -> u16 {
        self.face.units_per_em()
    }
    fn constants(&self) -> MathFontConstants {
        self.face.constants()
    }
    fn ascent(&self) -> i16 {
        self.face.ascent()
    }
    fn descent(&self) -> i16 {
        self.face.descent()
    }
    fn math_table_fingerprint(&self) -> [u8; 32] {
        self.face.math_table_fingerprint()
    }
    fn glyph_id(&mut self, value: char) -> Result<OriginalGlyphId, MathFontError> {
        let gid = self.access(|face, p| face.glyph_id_with_work(value, &mut |n| p.work(n)))?;
        self.metrics(gid)?;
        Ok(gid)
    }
    fn advance_width(&mut self, gid: OriginalGlyphId) -> Result<u16, MathFontError> {
        self.access(|face, p| face.advance_width_with_work(gid, &mut |n| p.work(n)))
    }
    fn glyph_height(&mut self, gid: OriginalGlyphId) -> Result<u16, MathFontError> {
        self.metrics(gid)
    }
    fn italic_correction(&mut self, gid: OriginalGlyphId) -> Result<i16, MathFontError> {
        self.access(|face, p| face.italic_correction_with_work(gid, &mut |n| p.work(n)))
    }
    fn vertical_variant(
        &mut self,
        gid: OriginalGlyphId,
        min: u16,
    ) -> Result<(OriginalGlyphId, u16), MathFontError> {
        let (gid, advance) =
            self.access(|face, p| face.vertical_variant_with_work(gid, min, &mut |n| p.work(n)))?;
        self.metrics(gid)?;
        Ok((gid, advance))
    }
}

#[cfg(test)]
#[path = "cff_v2_tests.rs"]
mod tests;
