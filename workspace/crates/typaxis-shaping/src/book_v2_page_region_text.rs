//! Common bidi/grapheme/shaping engine for source-bound page-region paragraphs.
use super::*;
use typaxis_syntax::book_v2::BookV2PageRegionTextFlow;
pub const BOOK_V2_PAGE_REGION_SHAPE_ALGORITHM: &str = "typaxis.book-2-page-region-text-shape/1";

pub struct BookV2PageRegionTextShape<'a> {
    flow: &'a BookV2PageRegionTextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    instances: AdmittedProductionFontInstancesV3<'a>,
    limits_fingerprint: [u8; 32],
    epoch: [u8; 32],
    output: BodyShapeOutput<'a>,
}
impl<'a> BookV2PageRegionTextShape<'a> {
    pub fn flow(&self) -> &'a BookV2PageRegionTextFlow<'a> {
        self.flow
    }
    pub fn paragraphs(&self) -> &[ProductionBodyParagraphShape<'a>] {
        &self.output.paragraphs
    }
    pub fn font_instances(&self) -> &AdmittedProductionFontInstancesV3<'a> {
        &self.instances
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.output.fingerprint
    }
    pub fn output_records(&self) -> u64 {
        self.output.output_records
    }
    pub fn binding_epoch(&self) -> [u8; 32] {
        self.epoch
    }
    pub fn line_context_fingerprint(&self) -> Option<[u8; 32]> {
        self.output.line_context_fingerprint
    }
    pub fn verify(
        &self,
        flow: &BookV2PageRegionTextFlow<'_>,
        admitted: &AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
        epoch: [u8; 32],
    ) -> Result<(), ProductionTextShapeError> {
        if !std::ptr::eq(self.flow, flow)
            || !std::ptr::eq(self.admitted, admitted)
            || self.limits_fingerprint != limits.fingerprint()
            || self.epoch != epoch
        {
            return Err(error(
                NodeId::new(flow.source().node_id),
                ProductionTextShapeErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}

/// Produces original-font glyphs and line-context observations, not page paint
/// or body flow authority. Header/footer repetition is a placement decision.
pub fn shape_book_v2_page_region_text<'a>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a BookV2PageRegionTextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
) -> Result<BookV2PageRegionTextShape<'a>, ProductionTextShapeError> {
    shape_book_v2_page_region_text_counted(policy, flow, admitted, limits, epoch, lines, &mut 0)
}

pub fn shape_book_v2_page_region_text_counted<'a>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a BookV2PageRegionTextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
    observed_records: &mut u64,
) -> Result<BookV2PageRegionTextShape<'a>, ProductionTextShapeError> {
    shape_region_kernel(
        policy,
        flow,
        admitted,
        limits,
        epoch,
        lines,
        observed_records,
        &mut LegacyRegionShapeAccounting,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ObservedPageRegionShapeError {
    Accounting(BookV2ObservedAccountingError),
    Shape(ProductionTextShapeError),
}
impl From<ProductionTextShapeError> for BookV2ObservedPageRegionShapeError {
    fn from(error: ProductionTextShapeError) -> Self {
        Self::Shape(error)
    }
}
impl std::fmt::Display for BookV2ObservedPageRegionShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 observed region shape: {self:?}")
    }
}
impl std::error::Error for BookV2ObservedPageRegionShapeError {}
trait RegionShapeAccounting {
    type Error: From<ProductionTextShapeError>;
    fn instances<'a>(
        &mut self,
        ledger: &'a AdmittedProductionResourceLedgerV3,
        owner: NodeId,
    ) -> Result<AdmittedProductionFontInstancesV3<'a>, Self::Error> {
        AdmittedProductionFontInstancesV3::from_used_faces(
            ledger,
            ledger.fonts().iter().map(|f| f.font_face_id()),
        )
        .map_err(|_| error(owner, ProductionTextShapeErrorKind::ReceiptMismatch).into())
    }
    fn shape<'a>(
        &mut self,
        request: BodyShapeRequest<'a, '_, '_>,
        records: &mut u64,
    ) -> Result<BodyShapeOutput<'a>, Self::Error> {
        request.shape(records).map_err(Self::Error::from)
    }
    fn validate(
        &self,
        flow: &BookV2PageRegionTextFlow<'_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), Self::Error>;
    fn output_limit(&self, maximum: u64) -> Result<Option<u64>, Self::Error>;
    fn retain(&mut self, records: u64) -> Result<(), Self::Error>;
}
struct LegacyRegionShapeAccounting;
impl RegionShapeAccounting for LegacyRegionShapeAccounting {
    type Error = ProductionTextShapeError;
    fn validate(
        &self,
        _: &BookV2PageRegionTextFlow<'_>,
        _: &M4EffectiveResourceLimits,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn output_limit(&self, _: u64) -> Result<Option<u64>, Self::Error> {
        Ok(None)
    }
    fn retain(&mut self, _: u64) -> Result<(), Self::Error> {
        Ok(())
    }
}
struct ObservedRegionShapeAccounting<'loan, 'id, 'cells, 'window> {
    accounting: &'loan mut BookV2ObservedSourceAccounting<'id, 'cells>,
    window: &'loan mut BookV2ObservedIntrinsicRecords<'id, 'window>,
}
impl RegionShapeAccounting for ObservedRegionShapeAccounting<'_, '_, '_, '_> {
    type Error = BookV2ObservedPageRegionShapeError;
    fn instances<'a>(
        &mut self,
        ledger: &'a AdmittedProductionResourceLedgerV3,
        owner: NodeId,
    ) -> Result<AdmittedProductionFontInstancesV3<'a>, Self::Error> {
        observed_storage::instances(ledger, owner, self.accounting).map_err(|e| match e {
            observed_storage::Error::Accounting(e) => Self::Error::Accounting(e),
            observed_storage::Error::Shape(e) => Self::Error::Shape(e),
        })
    }
    fn shape<'a>(
        &mut self,
        request: BodyShapeRequest<'a, '_, '_>,
        records: &mut u64,
    ) -> Result<BodyShapeOutput<'a>, Self::Error> {
        observed_storage::shape(request, records, self.accounting).map_err(|error| match error {
            observed_storage::Error::Accounting(e) => Self::Error::Accounting(e),
            observed_storage::Error::Shape(e) => Self::Error::Shape(e),
        })
    }
    fn validate(
        &self,
        flow: &BookV2PageRegionTextFlow<'_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), Self::Error> {
        self.accounting
            .require_limit_domain(limits.base())
            .map_err(Self::Error::Accounting)?;
        self.accounting
            .require_limit_domain(flow.body().body().limits())
            .map_err(Self::Error::Accounting)
    }
    fn output_limit(&self, _: u64) -> Result<Option<u64>, Self::Error> {
        self.accounting
            .intrinsic_limit(self.window)
            .map(Some)
            .map_err(Self::Error::Accounting)
    }
    fn retain(&mut self, records: u64) -> Result<(), Self::Error> {
        self.accounting
            .retain_intrinsic_output(self.window, NodeId::new(0), records)
            .map_err(Self::Error::Accounting)
    }
}
/// The actual region shape kernel with a same-loan, freshly empty graph window.
#[allow(clippy::too_many_arguments)]
pub fn shape_book_v2_page_region_text_with_observed_source_counted<'a, 'id>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a BookV2PageRegionTextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
    accounting: &mut BookV2ObservedSourceAccounting<'id, '_>,
    window: &mut BookV2ObservedIntrinsicRecords<'id, '_>,
    observed_records: &mut u64,
) -> Result<BookV2PageRegionTextShape<'a>, BookV2ObservedPageRegionShapeError> {
    shape_region_kernel(
        policy,
        flow,
        admitted,
        limits,
        epoch,
        lines,
        observed_records,
        &mut ObservedRegionShapeAccounting { accounting, window },
    )
}
#[allow(clippy::too_many_arguments)]
fn shape_region_kernel<'a, A: RegionShapeAccounting>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a BookV2PageRegionTextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
    observed_records: &mut u64,
    accounting: &mut A,
) -> Result<BookV2PageRegionTextShape<'a>, A::Error> {
    *observed_records = 0;
    let mismatch = || {
        error(
            NodeId::new(flow.source().node_id),
            ProductionTextShapeErrorKind::ReceiptMismatch,
        )
    };
    policy
        .verify_for(policy.body(), limits)
        .map_err(|_| A::Error::from(mismatch()))?;
    flow.verify_for(policy.body().styled(), flow.navigation())
        .map_err(|_| A::Error::from(mismatch()))?;
    if epoch == [0; 32]
        || admitted.profile_fingerprint() != policy.fingerprint()
        || admitted.effective_limits() != limits
        || !admitted.matches_declared_resources(flow.body().body().resources())
        || unicode_bidi::UNICODE_VERSION != (16, 0, 0)
        || unicode_script::UNICODE_VERSION != (16, 0, 0)
        || unicode_segmentation::UNICODE_VERSION != (16, 0, 0)
    {
        return Err(A::Error::from(mismatch()));
    }
    accounting.validate(flow, limits)?;
    // Reject a closed intrinsic window before paying for its new instance table.
    accounting.output_limit(limits.base().get().max_fragments)?;
    let instances = accounting.instances(admitted, NodeId::new(flow.source().node_id))?;
    // Instance rows have their own retained owner, outside this output window.
    let output_limit = accounting.output_limit(limits.base().get().max_fragments)?;
    let output = accounting.shape(
        BodyShapeRequest {
            flow: BodyFlow::PageRegion(flow),
            admitted: BodyFonts::BookV2(&instances, admitted),
            limits,
            epoch,
            lines,
            algorithm: BOOK_V2_PAGE_REGION_SHAPE_ALGORITHM,
            maximum_records: output_limit,
        },
        observed_records,
    );
    let retained = accounting.retain(*observed_records);
    let output = output?;
    retained?;
    Ok(BookV2PageRegionTextShape {
        flow,
        admitted,
        instances,
        limits_fingerprint: limits.fingerprint(),
        epoch,
        output,
    })
}
