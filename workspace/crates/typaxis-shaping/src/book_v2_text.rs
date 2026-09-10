//! Source-bound successor paragraphs use the common bidi, grapheme, line-context
//! and label engine, with CFF /2 coverage and a separate result identity.
use super::*;
#[path = "book_v2_equation_numbers.rs"]
mod equation_numbers;
pub use equation_numbers::{
    book_v2_equation_number_font, shape_book_v2_equation_numbers, BookV2EquationNumberError, BookV2EquationNumberErrorKind,
    BookV2EquationNumberShape, BookV2EquationNumberShapes, BOOK_V2_EQUATION_NUMBER_ALGORITHM,
};
use typaxis_resource_admission::{
    AdmittedProductionFontInstancesV3, AdmittedProductionResourceLedgerV3,
};
use typaxis_syntax::book_v2::{BookV2ResourcePolicy, PreparedBookV2TextFlow};
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
    use ProductionTextShapeErrorKind as E;
    let mismatch = || error(NodeId::new(0), E::ReceiptMismatch);
    policy
        .verify_for(policy.body(), limits)
        .map_err(|_| mismatch())?;
    flow.verify_for(policy.body().styled(), flow.navigation())
        .map_err(|_| mismatch())?;
    if epoch == [0; 32]
        || admitted.profile_fingerprint() != policy.fingerprint()
        || admitted.effective_limits() != limits
        || !admitted.matches_declared_resources(flow.resource_declarations())
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
        BodyFlow::BookV2(flow),
        BodyFonts::BookV2(&instances, admitted),
        limits,
        epoch,
        lines,
        BOOK_V2_AUTHORED_TEXT_SHAPE_ALGORITHM,
    )?;
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
    shape_book_v2_page_region_text, BookV2PageRegionTextShape, BOOK_V2_PAGE_REGION_SHAPE_ALGORITHM,
};
