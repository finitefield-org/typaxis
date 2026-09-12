//! Source-bound text inline preparation and actual line projection for book-2.
//! Native math and vector geometry share this path; block placement and pagination remain separate.
use super::*;
#[path = "book_v2_page_region_lines.rs"]
mod page_region_lines;
pub use page_region_lines::*;
pub use crate::block_vector::book_v2::{
    prepare_book_v2_vector_blocks, BookV2VectorBlock, BookV2VectorBlockError,
    BookV2VectorBlockLayout, BOOK_V2_VECTOR_BLOCK_ALGORITHM,
};
#[path = "book_v2_footnotes.rs"]
mod footnote_lines;
pub use footnote_lines::{prepare_book_v2_footnote_lines, BookV2FootnoteLines};
#[path = "book_v2_reshape.rs"]
mod feedback;
pub use feedback::{
    with_converged_book_v2_body_lines, with_converged_book_v2_body_lines_in_page_frames,
    with_converged_book_v2_body_lines_with_native_context,
    with_converged_book_v2_body_lines_with_remaining_passes,
    with_converged_book_v2_body_lines_with_source_widths, BookV2ConvergedBodyLines,
};
#[path = "book_v2_line_variant_seed.rs"]
mod line_variant_seed;
pub use line_variant_seed::{
    prepare_book_v2_body_line_variant_seed, with_rebuilt_book_v2_body_line_variant,
    BookV2BodyLineVariantSeed, BookV2RebuiltBodyLineVariant,
    with_rebuilt_book_v2_body_line_variants, BookV2RebuiltBodyLineVariants,
};
#[path = "book_v2_source_widths.rs"]
mod source_widths;
pub use source_widths::{
    layout_book_v2_source_width_lines_from_flow, BookV2SourceWidthAssignments,
};
#[path = "book_v2_frames.rs"]
mod frames;
pub use crate::math::book_v2::{
    compute_book_v2_native_math, BookV2MathReceipt, BookV2NativeMath, BookV2PlacedInlineMath,
    BOOK_V2_NATIVE_MATH_ALGORITHM, BOOK_V2_NATIVE_MATH_SET_ALGORITHM,
};
pub use crate::safe_vector::book_v2::{
    bind_book_v2_vectors, BookV2BoundVector, BookV2VectorBindingError, BookV2VectorBindings,
    BOOK_V2_VECTOR_BINDING_ALGORITHM, BOOK_V2_VECTOR_EPOCH_ALGORITHM, BOOK_V2_VECTOR_SET_ALGORITHM,
};
pub use frames::{
    layout_book_v2_body_inline_lines, prepare_book_v2_body_inline_frames, BookV2BodyInlineFrames,
    BOOK_V2_BODY_FRAMES_ALGORITHM, BookV2TableOccurrenceFrames,
};
use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
use typaxis_shaping::book_v2::BookV2AuthoredTextShape;
use typaxis_syntax::book_v2::PreparedBookV2TextFlow;

pub const BOOK_V2_TEXT_INLINE_ALGORITHM: &str = "typaxis.book-2-text-inline/1";
pub const BOOK_V2_TEXT_LINE_ALGORITHM: &str = "typaxis.book-2-text-lines/1";
enum PreparedBookV2Math<'a> {
    Owned(BookV2NativeMath<'a>),
    Borrowed(&'a BookV2NativeMath<'a>),
}
impl<'a> std::ops::Deref for PreparedBookV2Math<'a> {
    type Target = BookV2NativeMath<'a>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(v) => v,
            Self::Borrowed(v) => v,
        }
    }
}
pub struct BookV2PreparedInlines<'a> {
    flow: &'a PreparedBookV2TextFlow<'a>,
    shaped: &'a BookV2AuthoredTextShape<'a>,
    bindings: Option<&'a BookV2VectorBindings<'a>>,
    native_math: Option<PreparedBookV2Math<'a>>,
    paragraphs: Vec<ProductionPreparedInlineParagraph>,
    figures: Vec<ProductionPreparedFigure<'a>>,
    max_fragments: u64,
    limits_fingerprint: [u8; 32],
    fingerprint: [u8; 32],
}
impl<'a> BookV2PreparedInlines<'a> {
    pub(crate) fn effective_limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub fn figures(&self) -> &[ProductionPreparedFigure<'a>] {
        &self.figures
    }
    pub fn native_math(&self) -> Option<&BookV2NativeMath<'a>> {
        self.native_math.as_deref()
    }
    pub fn vector_bindings(&self) -> Option<&'a BookV2VectorBindings<'a>> {
        self.bindings
    }
    pub fn paragraphs(&self) -> &[ProductionPreparedInlineParagraph] {
        &self.paragraphs
    }
    pub fn source_flow(&self) -> &'a PreparedBookV2TextFlow<'a> {
        self.flow
    }
    pub fn shaped(&self) -> &'a BookV2AuthoredTextShape<'a> {
        self.shaped
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        flow: &PreparedBookV2TextFlow<'_>,
        shaped: &BookV2AuthoredTextShape<'_>,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.flow, flow) || !std::ptr::eq(self.shaped, shaped) {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}
/// Prepare all paragraph text in source order. Inline vector/native objects
/// require their successor binding stage and currently return a typed error.
/// The full source flow remains borrowed, including blocks outside paragraphs.
pub fn prepare_book_v2_text_inlines<'a>(
    flow: &'a PreparedBookV2TextFlow<'a>,
    shaped: &'a BookV2AuthoredTextShape<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    japanese_mode: JapaneseLineBreakMode,
) -> Result<BookV2PreparedInlines<'a>, ProductionInlinePreparationError> {
    prepare_inlines(
        flow,
        shaped,
        admitted,
        limits,
        epoch,
        japanese_mode,
        None,
        None,
    )
}
/// Join actual source-bound vector placements with the shared authored text.
pub fn prepare_book_v2_inline_items<'a>(
    flow: &'a PreparedBookV2TextFlow<'a>,
    shaped: &'a BookV2AuthoredTextShape<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
) -> Result<BookV2PreparedInlines<'a>, ProductionInlinePreparationError> {
    let native = compute_book_v2_native_math(bindings, admitted, limits, 0, 0).map_err(|e| {
        error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::NativeMath(e),
        )
    })?;
    prepare_inlines(
        flow,
        shaped,
        admitted,
        limits,
        bindings.epoch(),
        japanese_mode,
        Some(bindings),
        native.map(PreparedBookV2Math::Owned),
    )
}
/// Reuse the same immutable native computations across actual line/page passes.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_inline_items_with_native_context<'a>(
    flow: &'a PreparedBookV2TextFlow<'a>,
    shaped: &'a BookV2AuthoredTextShape<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    native: Option<&'a BookV2NativeMath<'a>>,
) -> Result<BookV2PreparedInlines<'a>, ProductionInlinePreparationError> {
    prepare_inlines(
        flow,
        shaped,
        admitted,
        limits,
        bindings.epoch(),
        japanese_mode,
        Some(bindings),
        native.map(PreparedBookV2Math::Borrowed),
    )
}
#[allow(clippy::too_many_arguments)]
fn prepare_inlines<'a>(
    flow: &'a PreparedBookV2TextFlow<'a>,
    shaped: &'a BookV2AuthoredTextShape<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    japanese_mode: JapaneseLineBreakMode,
    bindings: Option<&'a BookV2VectorBindings<'a>>,
    native_math: Option<PreparedBookV2Math<'a>>,
) -> Result<BookV2PreparedInlines<'a>, ProductionInlinePreparationError> {
    if let Some(bindings) = bindings {
        bindings
            .verify(bindings.body(), admitted, limits)
            .map_err(|_| {
                error(
                    NodeId::new(0),
                    ProductionInlinePreparationErrorKind::ReceiptMismatch,
                )
            })?;
        flow.verify_for(bindings.body().styled(), flow.navigation())
            .map_err(|_| {
                error(
                    NodeId::new(0),
                    ProductionInlinePreparationErrorKind::ReceiptMismatch,
                )
            })?;
        if let Some(native) = &native_math {
            native.verify(bindings, admitted, limits).map_err(|e| {
                error(
                    NodeId::new(0),
                    ProductionInlinePreparationErrorKind::NativeMath(e),
                )
            })?;
        }
    }
    if native_math.is_none() && !flow.body().body().math().is_empty() {
        return Err(error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::ReceiptMismatch,
        ));
    }
    shaped.verify(flow, admitted, limits, epoch).map_err(|e| {
        error(
            e.owner,
            ProductionInlinePreparationErrorKind::ReceiptMismatch,
        )
    })?;
    let mut charge = native_math.as_ref().map_or(0, |n| n.record_charge());
    let paragraphs = prepare_paragraphs(
        InlineFlow::BookV2(flow),
        shaped.paragraphs(),
        bindings.map(InlineVectors::BookV2),
        native_math.as_deref().map(InlineNativeMath::BookV2),
        limits,
        japanese_mode,
        &mut charge,
    )?;
    let figures = figure::prepare_figures(
        InlineFlow::BookV2(flow),
        InlineImages::BookV2(admitted),
        &mut charge,
        limits.base().get().max_fragments,
    )?;
    let mut fingerprint = sha256(BOOK_V2_TEXT_INLINE_ALGORITHM.as_bytes());
    for digest in [
        flow.fingerprint(),
        shaped.fingerprint(),
        limits.fingerprint(),
    ]
    .into_iter()
    .chain(paragraphs.iter().map(|p| {
        p.items
            .as_ref()
            .map_or([0; 32], ProductionInlineParagraph::fingerprint)
    })) {
        let mut bytes = [0; 64];
        bytes[..32].copy_from_slice(&fingerprint);
        bytes[32..].copy_from_slice(&digest);
        fingerprint = sha256(&bytes);
    }
    if let Some(bindings) = bindings {
        let mut bytes = [0; 64];
        bytes[..32].copy_from_slice(&fingerprint);
        bytes[32..].copy_from_slice(&bindings.fingerprint());
        fingerprint = sha256(&bytes);
    }
    if let Some(native) = &native_math {
        let mut bytes = [0; 64];
        bytes[..32].copy_from_slice(&fingerprint);
        bytes[32..].copy_from_slice(&native.fingerprint());
        fingerprint = sha256(&bytes);
    }
    Ok(BookV2PreparedInlines {
        flow,
        shaped,
        bindings,
        native_math,
        paragraphs,
        figures,
        max_fragments: limits.base().get().max_fragments,
        limits_fingerprint: limits.fingerprint(),
        fingerprint,
    })
}

pub struct BookV2InlineLineLayout<'p, 'a> {
    prepared: &'p BookV2PreparedInlines<'a>,
    projection: selected::LineProjection<'p, 'a>,
    frames: Option<BookV2BodyInlineFrames<'p, 'a>>,
}
impl<'p, 'a> BookV2InlineLineLayout<'p, 'a> {
    pub fn frames(&self) -> Option<&BookV2BodyInlineFrames<'p, 'a>> {
        self.frames.as_ref()
    }
    pub fn paragraphs(&self) -> &[ProductionInlineParagraphLineLayout<'p, 'a>] {
        &self.projection.paragraphs
    }
    pub fn prepared(&self) -> &'p BookV2PreparedInlines<'a> {
        self.prepared
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.projection.fingerprint
    }
    pub fn output_records(&self) -> u64 {
        self.projection.output_records
    }
    pub fn candidate_steps(&self) -> u64 {
        self.projection.candidate_steps
    }
    pub fn verify(
        &self,
        prepared: &BookV2PreparedInlines<'_>,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.prepared, prepared) {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
    pub fn selected_line_contexts(
        &self,
    ) -> Result<ProductionSelectedLineContexts, ProductionInlinePreparationError> {
        line_context::selected_contexts(
            self.paragraphs(),
            self.output_records(),
            self.prepared.max_fragments,
            self.fingerprint(),
        )
    }
}
pub fn layout_book_v2_inline_lines<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    inline_sizes: &[PositiveLength],
    max_candidate_steps: u64,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    layout_with_record_base(prepared, inline_sizes, max_candidate_steps, 0)
}
/// Project original-source width assignments through actual glyph/atomic-item
/// selection. This frameless result is not a selected physical page geometry.
/// Every profile must borrow this preparation's exact paragraph item owner.
pub fn layout_book_v2_inline_lines_with_source_widths<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    inline_sizes: &[PositiveLength],
    source_widths: &[Option<typaxis_linebreak::ProductionInlineSourceWidths<'_>>],
    max_candidate_steps: u64,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    layout_book_v2_source_width_lines_charged(
        prepared,
        inline_sizes,
        source_widths,
        max_candidate_steps,
        0,
    )
}
/// Continue the document's retained-record allowance; failure never creates a
/// physical frame or resets a caller's previously consumed records.
pub fn layout_book_v2_source_width_lines_charged<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    inline_sizes: &[PositiveLength],
    source_widths: &[Option<typaxis_linebreak::ProductionInlineSourceWidths<'_>>],
    max_candidate_steps: u64,
    prior_records: u64,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    layout_with_source_widths(
        prepared,
        inline_sizes,
        max_candidate_steps,
        prior_records,
        Some(source_widths),
    )
}
fn layout_with_record_base<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    inline_sizes: &[PositiveLength],
    max_candidate_steps: u64,
    record_base: u64,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    layout_with_source_widths(
        prepared,
        inline_sizes,
        max_candidate_steps,
        record_base,
        None,
    )
}
fn layout_with_source_widths<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    inline_sizes: &[PositiveLength],
    max_candidate_steps: u64,
    record_base: u64,
    source_widths: Option<&[Option<typaxis_linebreak::ProductionInlineSourceWidths<'_>>]>,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    let projection = selected::project_lines_with_source_widths(
        selected::LineInputs {
            max_fragments: prepared.max_fragments,
            flow: InlineFlow::BookV2(prepared.flow),
            shaped: prepared.shaped.paragraphs(),
            paragraphs: &prepared.paragraphs,
            footnote_markers: prepared.shaped.footnote_markers(),
            native_math: prepared.native_math().map(InlineNativeMath::BookV2),
            figure_count: prepared.figures.len(),
            fingerprint: prepared.fingerprint,
        },
        inline_sizes,
        max_candidate_steps,
        record_base,
        BOOK_V2_TEXT_LINE_ALGORITHM,
        source_widths,
    )?;
    Ok(BookV2InlineLineLayout {
        prepared,
        projection,
        frames: None,
    })
}

pub use layout_book_v2_inline_lines as layout_book_v2_text_lines;
