//! Source-bound successor paragraphs use the common bidi, grapheme, line-context
//! and label engine, with CFF /2 coverage and a separate result identity.
use super::*;
#[path = "book_v2_equation_numbers.rs"]
mod equation_numbers;
pub use equation_numbers::{
    book_v2_equation_number_font, shape_book_v2_equation_numbers,
    shape_book_v2_equation_numbers_counted, shape_book_v2_equation_numbers_with_observed_storage,
    BookV2EquationNumberError, BookV2EquationNumberErrorKind, BookV2EquationNumberShape,
    BookV2EquationNumberShapes, BOOK_V2_EQUATION_NUMBER_ALGORITHM,
};
use typaxis_resource_admission::{
    AdmittedProductionFontInstancesV3, AdmittedProductionResourceLedgerV3,
};
use typaxis_syntax::book_v2::{
    BookV2ResourcePolicy, BookV2SourceVerificationBudget, PreparedBookV2TextFlow,
};
#[path = "book_v2_source_accounting.rs"]
mod source_accounting;
pub use source_accounting::{
    with_book_v2_observed_source_accounting, BookV2ObservedAccountingError,
    BookV2ObservedFreshGraphRequest, BookV2ObservedGraphPayment, BookV2ObservedIntrinsicRecords,
    BookV2ObservedOccurrenceKey, BookV2ObservedSourceAccounting, BookV2ObservedVerificationError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ObservedShapeError {
    Verification(BookV2ObservedVerificationError),
    Accounting(BookV2ObservedAccountingError),
    Shape(ProductionTextShapeError),
}
impl From<ProductionTextShapeError> for BookV2ObservedShapeError {
    fn from(error: ProductionTextShapeError) -> Self {
        Self::Shape(error)
    }
}
impl std::fmt::Display for BookV2ObservedShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 observed shape: {self:?}")
    }
}
impl std::error::Error for BookV2ObservedShapeError {}

trait ShapeAccounting {
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
    fn verify(
        &mut self,
        flow: &PreparedBookV2TextFlow<'_>,
        body: &typaxis_syntax::book_v2::StyledBookV2Body,
    ) -> Result<(), Self::Error>;
    fn output_limit(&self, maximum: u64) -> Result<Option<u64>, Self::Error>;
    fn retain(&mut self, records: u64) -> Result<(), Self::Error>;
}
struct LegacyShapeAccounting<'loan>(Option<&'loan mut BookV2SourceVerificationBudget>);
impl ShapeAccounting for LegacyShapeAccounting<'_> {
    type Error = ProductionTextShapeError;
    fn verify(
        &mut self,
        flow: &PreparedBookV2TextFlow<'_>,
        body: &typaxis_syntax::book_v2::StyledBookV2Body,
    ) -> Result<(), Self::Error> {
        let result = if let Some(budget) = self.0.as_deref_mut() {
            budget.verify(flow, body, flow.navigation())
        } else {
            flow.verify_for(body, flow.navigation())
        };
        result.map_err(|e| match e.kind {
            typaxis_syntax::ProductionFlowErrorKind::NodeLimit => {
                error(e.owner, ProductionTextShapeErrorKind::OutputLimit)
            }
            typaxis_syntax::ProductionFlowErrorKind::AllocationFailure => {
                error(e.owner, ProductionTextShapeErrorKind::AllocationFailure)
            }
            _ => error(
                NodeId::new(0),
                ProductionTextShapeErrorKind::ReceiptMismatch,
            ),
        })
    }
    fn output_limit(&self, maximum: u64) -> Result<Option<u64>, Self::Error> {
        self.0
            .as_deref()
            .map(|b| {
                b.output_record_limit(maximum)
                    .ok_or_else(|| error(NodeId::new(0), ProductionTextShapeErrorKind::OutputLimit))
            })
            .transpose()
    }
    fn retain(&mut self, records: u64) -> Result<(), Self::Error> {
        if let Some(budget) = self.0.as_deref_mut() {
            budget.include_retained_records(records);
        }
        Ok(())
    }
}
struct ObservedShapeAccounting<'loan, 'id, 'cells, 'window> {
    accounting: &'loan mut BookV2ObservedSourceAccounting<'id, 'cells>,
    intrinsic: &'loan mut BookV2ObservedIntrinsicRecords<'id, 'window>,
}
impl ShapeAccounting for ObservedShapeAccounting<'_, '_, '_, '_> {
    type Error = BookV2ObservedShapeError;
    fn instances<'a>(
        &mut self,
        ledger: &'a AdmittedProductionResourceLedgerV3,
        owner: NodeId,
    ) -> Result<AdmittedProductionFontInstancesV3<'a>, Self::Error> {
        self.accounting
            .intrinsic_limit(self.intrinsic)
            .map_err(Self::Error::Accounting)?;
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
    fn verify(
        &mut self,
        flow: &PreparedBookV2TextFlow<'_>,
        body: &typaxis_syntax::book_v2::StyledBookV2Body,
    ) -> Result<(), Self::Error> {
        self.accounting
            .verify_source(flow, body, flow.navigation())
            .map_err(Self::Error::Verification)
    }
    fn output_limit(&self, _: u64) -> Result<Option<u64>, Self::Error> {
        self.accounting
            .intrinsic_limit(self.intrinsic)
            .map(Some)
            .map_err(Self::Error::Accounting)
    }
    fn retain(&mut self, records: u64) -> Result<(), Self::Error> {
        self.accounting
            .retain_intrinsic_output(self.intrinsic, NodeId::new(0), records)
            .map_err(Self::Error::Accounting)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn shape_book_v2_authored_text_with_observed_source_counted<'a, 'id>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
    accounting: &mut BookV2ObservedSourceAccounting<'id, '_>,
    intrinsic: &mut BookV2ObservedIntrinsicRecords<'id, '_>,
    observed_records: &mut u64,
) -> Result<BookV2AuthoredTextShape<'a>, BookV2ObservedShapeError> {
    shape_book_v2_authored_text_kernel(
        policy,
        flow,
        admitted,
        limits,
        epoch,
        lines,
        &mut ObservedShapeAccounting {
            accounting,
            intrinsic,
        },
        observed_records,
    )
}

pub const BOOK_V2_AUTHORED_TEXT_SHAPE_ALGORITHM: &str = "typaxis.book-2-authored-text-shape/1";
pub struct BookV2AuthoredTextShape<'a> {
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    instances: AdmittedProductionFontInstancesV3<'a>,
    limits_fingerprint: [u8; 32],
    epoch: [u8; 32],
    output: BodyShapeOutput<'a>,
}
impl<'a> BookV2AuthoredTextShape<'a> {
    pub fn paragraphs(&self) -> &[ProductionBodyParagraphShape<'a>] {
        &self.output.paragraphs
    }
    pub fn list_markers(&self) -> &[ProductionListMarkerShape<'a>] {
        &self.output.list_markers
    }
    pub fn footnote_markers(&self) -> &[ProductionFootnoteMarkerShape<'a>] {
        &self.output.footnote_markers
    }
    pub fn font_instances(&self) -> &AdmittedProductionFontInstancesV3<'a> {
        &self.instances
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.output.fingerprint
    }
    pub fn binding_epoch(&self) -> [u8; 32] {
        self.epoch
    }
    pub fn output_records(&self) -> u64 {
        self.output.output_records
    }
    pub fn line_context_fingerprint(&self) -> Option<[u8; 32]> {
        self.output.line_context_fingerprint
    }
    pub fn verify(
        &self,
        flow: &PreparedBookV2TextFlow<'_>,
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
                NodeId::new(0),
                ProductionTextShapeErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}
pub fn shape_book_v2_authored_text<'a>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
) -> Result<BookV2AuthoredTextShape<'a>, ProductionTextShapeError> {
    shape_book_v2_authored_text_counted(policy, flow, admitted, limits, epoch, lines, &mut 0)
}

/// Return accepted context and shaped-output reservations even on failure.
/// Font-instance construction and backend temporary allocations have separate limits.
pub fn shape_book_v2_authored_text_counted<'a>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
    observed_records: &mut u64,
) -> Result<BookV2AuthoredTextShape<'a>, ProductionTextShapeError> {
    shape_book_v2_authored_text_inner_counted(
        policy,
        flow,
        admitted,
        limits,
        epoch,
        lines,
        None,
        observed_records,
    )
}

/// Account the independently reconstructed source separately from intrinsic
/// shaping output. Retain both observations before propagating any error.
#[allow(clippy::too_many_arguments)]
pub fn shape_book_v2_authored_text_with_source_budget_counted<'a>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
    source_budget: &mut BookV2SourceVerificationBudget,
    observed_records: &mut u64,
) -> Result<BookV2AuthoredTextShape<'a>, ProductionTextShapeError> {
    shape_book_v2_authored_text_inner_counted(
        policy,
        flow,
        admitted,
        limits,
        epoch,
        lines,
        Some(source_budget),
        observed_records,
    )
}

#[allow(clippy::too_many_arguments)]
fn shape_book_v2_authored_text_inner_counted<'a>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
    source_budget: Option<&mut BookV2SourceVerificationBudget>,
    observed_records: &mut u64,
) -> Result<BookV2AuthoredTextShape<'a>, ProductionTextShapeError> {
    shape_book_v2_authored_text_kernel(
        policy,
        flow,
        admitted,
        limits,
        epoch,
        lines,
        &mut LegacyShapeAccounting(source_budget),
        observed_records,
    )
}

#[allow(clippy::too_many_arguments)]
fn shape_book_v2_authored_text_kernel<'a, A: ShapeAccounting>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
    accounting: &mut A,
    observed_records: &mut u64,
) -> Result<BookV2AuthoredTextShape<'a>, A::Error> {
    *observed_records = 0;
    use ProductionTextShapeErrorKind as E;
    let mismatch = || error(NodeId::new(0), E::ReceiptMismatch);
    policy
        .verify_for(policy.body(), limits)
        .map_err(|_| mismatch())?;
    accounting.verify(flow, policy.body().styled())?;
    if epoch == [0; 32]
        || admitted.profile_fingerprint() != policy.fingerprint()
        || admitted.effective_limits() != limits
        || !admitted.matches_declared_resources(flow.resource_declarations())
        || unicode_bidi::UNICODE_VERSION != (16, 0, 0)
        || unicode_script::UNICODE_VERSION != (16, 0, 0)
        || unicode_segmentation::UNICODE_VERSION != (16, 0, 0)
    {
        return Err(mismatch().into());
    }
    let instances = accounting.instances(admitted, NodeId::new(0))?;
    let maximum_records = accounting.output_limit(limits.base().get().max_fragments)?;
    let output = accounting.shape(
        BodyShapeRequest {
            flow: BodyFlow::BookV2(flow),
            admitted: BodyFonts::BookV2(&instances, admitted),
            limits,
            epoch,
            lines,
            algorithm: BOOK_V2_AUTHORED_TEXT_SHAPE_ALGORITHM,
            maximum_records,
        },
        observed_records,
    );
    let retained = accounting.retain(*observed_records);
    let output = output?;
    retained?;
    Ok(BookV2AuthoredTextShape {
        flow,
        admitted,
        instances,
        limits_fingerprint: limits.fingerprint(),
        epoch,
        output,
    })
}

impl std::fmt::Debug for BookV2AuthoredTextShape<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BookV2AuthoredTextShape")
            .field("paragraphs", &self.output.paragraphs.len())
            .field("output_records", &self.output.output_records)
            .field("fingerprint", &self.output.fingerprint)
            .finish_non_exhaustive()
    }
}

#[path = "book_v2_page_region_text.rs"]
mod page_regions;
pub use page_regions::{
    shape_book_v2_page_region_text, shape_book_v2_page_region_text_counted,
    shape_book_v2_page_region_text_with_observed_source_counted,
    BookV2ObservedPageRegionShapeError, BookV2PageRegionTextShape,
    BOOK_V2_PAGE_REGION_SHAPE_ALGORITHM,
};

/// Actual shaped output from exactly one consumed, freshly paid request.
/// Immutable source roots do not borrow the mutable accounting owner.
pub struct BookV2ObservedShapedOccurrence<'id, 'a> {
    shape: BookV2AuthoredTextShape<'a>,
    key: BookV2ObservedOccurrenceKey<'id>,
}
impl<'id, 'a> BookV2ObservedShapedOccurrence<'id, 'a> {
    pub fn shape(&self) -> &BookV2AuthoredTextShape<'a> {
        &self.shape
    }
    pub fn occurrence_key(&self) -> BookV2ObservedOccurrenceKey<'id> {
        self.key
    }
}

struct RequestShapeAccounting<'loan, 'id, 'cells> {
    accounting: &'loan mut BookV2ObservedSourceAccounting<'id, 'cells>,
    payment: &'loan mut BookV2ObservedGraphPayment<'id>,
}
impl ShapeAccounting for RequestShapeAccounting<'_, '_, '_> {
    type Error = BookV2ObservedShapeError;
    fn verify(
        &mut self,
        flow: &PreparedBookV2TextFlow<'_>,
        body: &typaxis_syntax::book_v2::StyledBookV2Body,
    ) -> Result<(), Self::Error> {
        self.accounting
            .verify_source(flow, body, flow.navigation())
            .map_err(Self::Error::Verification)
    }
    fn output_limit(&self, _: u64) -> Result<Option<u64>, Self::Error> {
        self.payment
            .output_record_limit(self.accounting)
            .map(Some)
            .map_err(Self::Error::Accounting)
    }
    fn retain(&mut self, records: u64) -> Result<(), Self::Error> {
        self.payment
            .retain_actual_output(self.accounting, NodeId::new(0), records)
            .map_err(Self::Error::Accounting)
    }
}

/// Consume one genuinely new request in the actual source-bound shaper.
///
/// ```
/// use typaxis_core::{NodeId,M4EffectiveResourceLimits};
/// use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
/// use typaxis_shaping::book_v2::*;
/// use typaxis_syntax::book_v2::{BookV2ResourcePolicy,PreparedBookV2TextFlow};
/// fn once<'id,'a>(policy:&BookV2ResourcePolicy<'_>,flow:&'a PreparedBookV2TextFlow<'a>,
///     admitted:&'a AdmittedProductionResourceLedgerV3,limits:&M4EffectiveResourceLimits,
///     loan:&mut BookV2ObservedSourceAccounting<'id,'_>) {
///     let request=loan.reserve_graph_request(NodeId::new(0),0).unwrap();
///     if let Ok((shape,payment))=shape_book_v2_authored_text_for_observed_request_counted(
///         request,policy,flow,admitted,limits,[1;32],None,loan,&mut 0) {
///         assert_eq!(shape.occurrence_key(),payment.occurrence_key());
///         assert!(payment.paid_records()>=shape.shape().output_records());
///     }
/// }
/// ```
///
/// No private fields can create an unpaid fresh request.
/// ```compile_fail
/// use typaxis_shaping::book_v2::{BookV2ObservedFreshGraphRequest,BookV2ObservedOccurrenceKey};
/// fn forge<'id>(key:BookV2ObservedOccurrenceKey<'id>)->BookV2ObservedFreshGraphRequest<'id> {
///     BookV2ObservedFreshGraphRequest {paid:100,key}
/// }
/// ```
///
/// An independent loan cannot spend another loan's fresh request.
/// ```compile_fail
/// use typaxis_core::{NodeId,M4EffectiveResourceLimits};
/// use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
/// use typaxis_shaping::book_v2::*;
/// use typaxis_syntax::book_v2::{BookV2ResourcePolicy,PreparedBookV2TextFlow};
/// fn cross<'a>(policy:&BookV2ResourcePolicy<'_>,flow:&'a PreparedBookV2TextFlow<'a>,
///     admitted:&'a AdmittedProductionResourceLedgerV3,limits:&M4EffectiveResourceLimits) {
///     let(mut r,mut s,mut w,mut r2,mut s2,mut w2)=(0,0,0,0,0,0);
///     with_book_v2_observed_source_accounting(&mut r,&mut s,&mut w,limits.base(),10000,1000000,|first| {
///         let request=first.reserve_graph_request(NodeId::new(0),0).unwrap();
///         with_book_v2_observed_source_accounting(&mut r2,&mut s2,&mut w2,limits.base(),10000,1000000,|second| {
///             let _=shape_book_v2_authored_text_for_observed_request_counted(
///                 request,policy,flow,admitted,limits,[1;32],None,second,&mut 0);
///         });
///     });
/// }
/// ```
///
/// Post-shape payment cannot mint a second allocating shape.
/// ```compile_fail
/// use typaxis_core::M4EffectiveResourceLimits;
/// use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
/// use typaxis_shaping::book_v2::*;
/// use typaxis_syntax::book_v2::{BookV2ResourcePolicy,PreparedBookV2TextFlow};
/// fn again<'id,'a>(paid:BookV2ObservedGraphPayment<'id>,policy:&BookV2ResourcePolicy<'_>,
///     flow:&'a PreparedBookV2TextFlow<'a>,admitted:&'a AdmittedProductionResourceLedgerV3,
///     limits:&M4EffectiveResourceLimits,loan:&mut BookV2ObservedSourceAccounting<'id,'_>) {
///     let _=shape_book_v2_authored_text_for_observed_request_counted(
///         paid,policy,flow,admitted,limits,[1;32],None,loan,&mut 0);
/// }
/// ```
///
/// A request cannot be reused to create a second simultaneously retained shape.
/// ```compile_fail
/// use typaxis_core::{NodeId, M4EffectiveResourceLimits};
/// use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
/// use typaxis_shaping::book_v2::*;
/// use typaxis_syntax::book_v2::{BookV2ResourcePolicy, PreparedBookV2TextFlow};
/// fn twice<'id,'a>(policy:&BookV2ResourcePolicy<'_>,flow:&'a PreparedBookV2TextFlow<'a>,
///     admitted:&'a AdmittedProductionResourceLedgerV3, limits:&M4EffectiveResourceLimits,
///     loan:&mut BookV2ObservedSourceAccounting<'id,'_>) {
///     let request=loan.reserve_graph_request(NodeId::new(0),0).unwrap();
///     let _=shape_book_v2_authored_text_for_observed_request_counted(request,policy,flow,admitted,limits,[1;32],None,loan,&mut 0);
///     let _=shape_book_v2_authored_text_for_observed_request_counted(request,policy,flow,admitted,limits,[1;32],None,loan,&mut 0);
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn shape_book_v2_authored_text_for_observed_request_counted<'id, 'a>(
    request: BookV2ObservedFreshGraphRequest<'id>,
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    lines: Option<&[ProductionParagraphLineContext<'_>]>,
    accounting: &mut BookV2ObservedSourceAccounting<'id, '_>,
    observed_records: &mut u64,
) -> Result<
    (
        BookV2ObservedShapedOccurrence<'id, 'a>,
        BookV2ObservedGraphPayment<'id>,
    ),
    BookV2ObservedShapeError,
> {
    let mut payment = request.into_payment();
    let result = shape_book_v2_authored_text_kernel(
        policy,
        flow,
        admitted,
        limits,
        epoch,
        lines,
        &mut RequestShapeAccounting {
            accounting,
            payment: &mut payment,
        },
        observed_records,
    );
    match result {
        Ok(shape) => Ok((
            BookV2ObservedShapedOccurrence {
                shape,
                key: payment.occurrence_key(),
            },
            payment,
        )),
        Err(error) => {
            payment.seal();
            Err(error)
        }
    }
}

#[path = "book_v2_base_cmap_admission.rs"]
mod base_cmap_admission;
#[path = "book_v2_resource_read_budget.rs"]
mod resource_read_budget;
pub use resource_read_budget::BookV2ObservedResourceAdmissionBudget;
pub use base_cmap_admission::{
    BookV2ObservedAdmissionBudget, BookV2ObservedBaseCmapBudget, BookV2ObservedCmapBudget,
    BookV2ObservedFontJstfBudget, BookV2ObservedFontMathBudget, BookV2ObservedFontMetadataBudget,
    BookV2ObservedFontOptionalLocalBudget, BookV2ObservedFontProgramBudget,
    BookV2ObservedFontTablesBudget,
};
