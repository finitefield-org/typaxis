//! The resource stages borrow authentic display owners without changing their
//! paint or terminal types.
use super::*;

mod sealed {
    pub trait Sealed {}
    impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> Sealed for BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {}
    impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> Sealed
        for BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    {
    }
    use super::{BookV2BodyDisplay, BookV2ColumnDisplay};
}

/// Resource access is sealed to the two verified physical display owners.
/// A forwarding wrapper cannot substitute its own paint uses.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::{BookV2ResourceDisplay, BookV2FontUseData, BookV2ImageUseData, BookV2MathDisplayError};
/// use typaxis_core::M4EffectiveResourceLimits;
/// use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
/// struct Forged<D>(D);
/// impl<'a, D: BookV2ResourceDisplay<'a>> BookV2ResourceDisplay<'a> for Forged<D> {
///     type Paint = D::Paint;
///     fn paint_count(&self) -> usize { self.0.paint_count() }
///     fn font_slot_count(&self, p: usize) -> Option<usize> { self.0.font_slot_count(p) }
///     fn font_use<'v>(&'v self, p: usize, s: usize) -> Result<Option<BookV2FontUseData<'v, 'a, Self::Paint>>, BookV2MathDisplayError> { self.0.font_use(p, s) }
///     fn image_use<'v>(&'v self, p: usize) -> Result<Option<BookV2ImageUseData<'v, Self::Paint>>, BookV2MathDisplayError> { self.0.image_use(p) }
///     fn admitted(&self) -> &AdmittedProductionResourceLedgerV3 { self.0.admitted() }
///     fn fingerprint(&self) -> [u8; 32] { self.0.fingerprint() }
///     fn record_charge(&self) -> u64 { self.0.record_charge() }
///     fn work_steps(&self) -> u64 { self.0.work_steps() }
///     fn source_spool_charge(&self) -> u64 { self.0.source_spool_charge() }
///     fn original_font_instance_count(&self) -> usize { self.0.original_font_instance_count() }
///     fn original_font_instance_fingerprint(&self) -> [u8; 32] { self.0.original_font_instance_fingerprint() }
///     fn verify_resource_selection(&self, a: &AdmittedProductionResourceLedgerV3, l: &M4EffectiveResourceLimits) -> Result<(), BookV2MathDisplayError> { self.0.verify_resource_selection(a, l) }
/// }
/// ```
pub trait BookV2ResourceDisplay<'a>: sealed::Sealed {
    type Paint: Copy + Eq;
    fn paint_count(&self) -> usize;
    fn font_slot_count(&self, paint: usize) -> Option<usize>;
    fn font_use<'v>(
        &'v self,
        paint: usize,
        slot: usize,
    ) -> Result<Option<BookV2FontUseData<'v, 'a, Self::Paint>>, BookV2MathDisplayError>;
    fn image_use<'v>(
        &'v self,
        paint: usize,
    ) -> Result<Option<BookV2ImageUseData<'v, Self::Paint>>, BookV2MathDisplayError>;
    fn admitted(&self) -> &AdmittedProductionResourceLedgerV3;
    fn fingerprint(&self) -> [u8; 32];
    fn record_charge(&self) -> u64;
    fn work_steps(&self) -> u64;
    fn source_spool_charge(&self) -> u64;
    fn original_font_instance_count(&self) -> usize;
    fn original_font_instance_fingerprint(&self) -> [u8; 32];
    fn verify_resource_selection(
        &self,
        admitted: &AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookV2MathDisplayError>;
}

macro_rules! authentic_resource_display {
    ($display:ident, $paint:ident) => {
        impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2ResourceDisplay<'a>
            for $display<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>
        {
            type Paint = $paint;
            fn paint_count(&self) -> usize {
                self.paints().len()
            }
            fn font_slot_count(&self, paint: usize) -> Option<usize> {
                self.font_slot_count(paint)
            }
            fn font_use<'v>(
                &'v self,
                paint: usize,
                slot: usize,
            ) -> Result<Option<BookV2FontUseData<'v, 'a, Self::Paint>>, BookV2MathDisplayError>
            {
                self.font_use(paint, slot)
            }
            fn image_use<'v>(
                &'v self,
                paint: usize,
            ) -> Result<Option<BookV2ImageUseData<'v, Self::Paint>>, BookV2MathDisplayError> {
                self.image_use(paint)
            }
            fn admitted(&self) -> &AdmittedProductionResourceLedgerV3 {
                self.admitted()
            }
            fn fingerprint(&self) -> [u8; 32] {
                self.fingerprint()
            }
            fn record_charge(&self) -> u64 {
                self.record_charge()
            }
            fn work_steps(&self) -> u64 {
                self.work_steps()
            }
            fn source_spool_charge(&self) -> u64 {
                self.source().spool_charge()
            }
            fn original_font_instance_count(&self) -> usize {
                self.source()
                    .original_graph()
                    .lines()
                    .prepared()
                    .shaped()
                    .font_instances()
                    .len()
            }
            fn original_font_instance_fingerprint(&self) -> [u8; 32] {
                self.source()
                    .original_graph()
                    .lines()
                    .prepared()
                    .shaped()
                    .font_instances()
                    .fingerprint()
            }
            fn verify_resource_selection(
                &self,
                admitted: &AdmittedProductionResourceLedgerV3,
                limits: &M4EffectiveResourceLimits,
            ) -> Result<(), BookV2MathDisplayError> {
                self.verify_resource_selection(admitted, limits)
            }
        }
    };
}
authentic_resource_display!(BookV2BodyDisplay, BookV2BodyPaintIndex);
authentic_resource_display!(BookV2ColumnDisplay, BookV2ColumnPaintIndex);
