//! Native Book math using the same selected-program cache as later PDF output.
use super::*;
use typaxis_font::{
    Cff1AdmissionV2, Cff1SubsetSessionV2, CffMathFontFaceV2, CffMathGlyphMetricsV2,
};
use typaxis_shaping::book_v2::BookV2ObservedSourceAccounting;
#[derive(Clone, Copy)]
pub(super) enum NativeFace<'a> {
    TrueType(MathFontFace<'a>),
    Cff(CffMathFontFaceV2<'a>),
}
pub(super) enum ComputeFailure<E> {
    Native(ProductionNativeMathComputationError),
    Access(E),
}
impl<E> From<StagingMathLayoutError> for ComputeFailure<E> {
    fn from(e: StagingMathLayoutError) -> Self {
        Self::Native(e.into())
    }
}
impl<E> From<ProductionNativeMathComputationError> for ComputeFailure<E> {
    fn from(e: ProductionNativeMathComputationError) -> Self {
        Self::Native(e)
    }
}
pub(super) trait NativeProvider: typaxis_math::CffMathProviderV2 {
    fn owner(&mut self, _: NodeId) {}
    fn face(&mut self, _: FontFaceId) {}
}
pub(super) struct NoCff;
impl NativeProvider for NoCff {}
impl typaxis_math::CffMathProviderV2 for NoCff {
    type Error = std::convert::Infallible;
    fn work(&mut self, _: u64) -> Result<(), Self::Error> {
        unreachable!("legacy path does not select CFF math")
    }
    fn glyph_metrics(
        &mut self,
        _: &Cff1AdmissionV2,
        _: typaxis_font::OriginalGlyphId,
    ) -> Result<CffMathGlyphMetricsV2, Self::Error> {
        unreachable!("legacy path does not select CFF math")
    }
}
struct Provider<'p, 'id, 'cells> {
    session: &'p mut Cff1SubsetSessionV2,
    owner: NodeId,
    font_face_id: FontFaceId,
    accounting: &'p mut BookV2ObservedSourceAccounting<'id, 'cells>,
}
impl NativeProvider for Provider<'_, '_, '_> {
    fn owner(&mut self, owner: NodeId) {
        self.owner = owner;
    }
    fn face(&mut self, face: FontFaceId) {
        self.font_face_id = face;
    }
}
impl typaxis_math::CffMathProviderV2 for Provider<'_, '_, '_> {
    type Error = BookV2ObservedNativeMathError;
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.accounting
            .debit_work(self.owner, n)
            .map_err(Self::Error::Accounting)
    }
    fn glyph_metrics(
        &mut self,
        admission: &Cff1AdmissionV2,
        gid: typaxis_font::OriginalGlyphId,
    ) -> Result<CffMathGlyphMetricsV2, Self::Error> {
        let Self {
            session,
            accounting,
            owner,
            font_face_id,
        } = self;
        let owner = *owner;
        let font_face_id = *font_face_id;
        let mut refused = None;
        let result = session.math_glyph_metrics_with_charge(admission, gid, &mut |r, s, w| {
            accounting
                .debit(owner, [r as u64, s as u64, w as u64])
                .map_err(|e| {
                    refused = Some(e);
                    typaxis_font::Cff1Error::SubsetByteLimit
                })
        });
        result.map_err(|error| match refused {
            Some(error) => Self::Error::Accounting(error),
            None => Self::Error::Cff {
                owner,
                font_face_id,
                error,
            },
        })
    }
}
/// CFF metadata and selected glyph access debit the authentic Source loan.
/// The session is borrowed only by this implementation, never by user callbacks.
pub fn compute_book_v2_native_math_cff_observed_counted<'id, 'a, 'cells>(
    bindings: &'a BookV2VectorBindings<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    accounting: &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    session: &mut Cff1SubsetSessionV2,
) -> Result<Option<BookV2ObservedNativeMath<'id, 'a>>, BookV2ObservedNativeMathError> {
    use BookV2ObservedNativeMathError as E;
    bindings
        .verify(bindings.body(), admitted, limits)
        .map_err(|_| E::Native(StagingMathLayoutError::ReceiptMismatch.into()))?;
    accounting
        .require_limit_domain(limits.base())
        .map_err(E::Accounting)?;
    accounting
        .require_limit_domain(bindings.body().styled().body().limits())
        .map_err(E::Accounting)?;
    let mut observed = BookV2NativeMathBudgetObservation::default();
    let plan =
        preflight_book_v2_native_math_counted(bindings, admitted, limits, 0, 0, &mut observed);
    let root = NodeId::new(0);
    let work = accounting.debit_work(root, observed.reserved_layout_units());
    let records = accounting.debit_records(root, observed.record_charge());
    let spool = accounting.debit_spool(root, observed.spool_charge());
    let plan = plan.map_err(E::Native)?;
    work.map_err(E::Accounting)?;
    records.map_err(E::Accounting)?;
    spool.map_err(E::Accounting)?;
    plan.map(|plan| {
        let mut provider = Provider {
            session,
            accounting,
            owner: NodeId::new(0),
            font_face_id: FontFaceId::new(0),
        };
        let inner =
            compute_preflighted_kernel(plan, limits, &mut provider, true).map_err(|e| match e {
                ComputeFailure::Native(e) => E::Native(e),
                ComputeFailure::Access(e) => e,
            })?;
        Ok(BookV2ObservedNativeMath {
            inner,
            local_records: observed.record_charge(),
            local_spool: observed.spool_charge(),
            local_work: observed.reserved_layout_units(),
            identity: std::marker::PhantomData,
        })
    })
    .transpose()
}
