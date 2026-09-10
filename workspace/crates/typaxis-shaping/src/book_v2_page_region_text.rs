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
    let mismatch = || {
        error(
            NodeId::new(flow.source().node_id),
            ProductionTextShapeErrorKind::ReceiptMismatch,
        )
    };
    policy
        .verify_for(policy.body(), limits)
        .map_err(|_| mismatch())?;
    flow.verify_for(policy.body().styled(), flow.navigation())
        .map_err(|_| mismatch())?;
    if epoch == [0; 32]
        || admitted.profile_fingerprint() != policy.fingerprint()
        || admitted.effective_limits() != limits
        || !admitted.matches_declared_resources(flow.body().body().resources())
        || unicode_bidi::UNICODE_VERSION != (16, 0, 0)
        || unicode_script::UNICODE_VERSION != (16, 0, 0)
        || unicode_segmentation::UNICODE_VERSION != (16, 0, 0)
    {
        return Err(mismatch());
    }
    let instances = AdmittedProductionFontInstancesV3::from_used_faces(
        admitted,
        admitted.fonts().iter().map(|f| f.font_face_id()),
    )
    .map_err(|_| mismatch())?;
    let output = shape_document(
        BodyFlow::PageRegion(flow),
        BodyFonts::BookV2(&instances, admitted),
        limits,
        epoch,
        lines,
        BOOK_V2_PAGE_REGION_SHAPE_ALGORITHM,
    )?;
    Ok(BookV2PageRegionTextShape {
        flow,
        admitted,
        instances,
        limits_fingerprint: limits.fingerprint(),
        epoch,
        output,
    })
}
