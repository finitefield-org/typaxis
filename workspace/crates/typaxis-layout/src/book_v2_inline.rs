//! Source-bound text inline preparation and actual line projection for book-2.
//! Native math and vector geometry share this path; block placement and pagination remain separate.
use super::*;
#[path = "book_v2_page_region_lines.rs"]
mod page_region_lines;
pub use crate::block_vector::book_v2::{
    prepare_book_v2_vector_blocks, prepare_book_v2_vector_blocks_counted, BookV2VectorBlock, BookV2VectorBlockError,
    BookV2VectorBlockLayout, BOOK_V2_VECTOR_BLOCK_ALGORITHM,
};
pub use page_region_lines::*;
#[path = "book_v2_footnotes.rs"]
mod footnote_lines;
pub use footnote_lines::{prepare_book_v2_footnote_lines, prepare_book_v2_footnote_lines_counted, BookV2FootnoteLines};
#[path = "book_v2_reshape.rs"]
mod feedback;
pub use feedback::{
    with_budgeted_book_v2_body_lines, with_budgeted_book_v2_body_lines_with_source_widths,
    with_budgeted_book_v2_column_lines,
    with_converged_book_v2_body_lines,
    with_converged_book_v2_body_lines_in_page_frames,
    with_converged_book_v2_body_lines_with_native_context,
    with_converged_book_v2_body_lines_with_remaining_passes,
    with_converged_book_v2_body_lines_with_source_widths, BookV2BodyLineBudget,
    BookV2ConvergedBodyLines, BookV2ConvergedColumnLines,
};
#[path = "book_v2_line_variant_seed.rs"]
mod line_variant_seed;
pub use line_variant_seed::{
    prepare_book_v2_body_line_variant_seed, prepare_budgeted_book_v2_body_line_variant_seed,
    with_budgeted_rebuilt_book_v2_body_line_variant,
    with_budgeted_rebuilt_book_v2_body_line_variants, with_rebuilt_book_v2_body_line_variant,
    with_rebuilt_book_v2_body_line_variants, BookV2BodyLineVariantSeed, BookV2LineVariantBudget,
    BookV2RebuiltBodyLineVariant, BookV2RebuiltBodyLineVariants,
    prepare_budgeted_book_v2_column_line_variant_seed,
    with_budgeted_rebuilt_book_v2_column_line_variants,
    BookV2ColumnLineVariantSeed, BookV2RebuiltColumnLineVariant, BookV2RebuiltColumnLineVariants,
};
#[path = "book_v2_source_widths.rs"]
mod source_widths;
pub use source_widths::{
    layout_book_v2_source_width_lines_from_flow, BookV2SourceWidthAssignments,
};
#[path = "book_v2_frames.rs"]
mod frames;
pub use crate::math::book_v2::{
    compute_book_v2_native_math, compute_book_v2_native_math_counted,
    compute_preflighted_book_v2_native_math, preflight_book_v2_native_math_counted,
    BookV2MathReceipt, BookV2NativeMath, BookV2NativeMathBudgetObservation,
    BookV2NativeMathPreflight, BookV2PlacedInlineMath,
    BOOK_V2_NATIVE_MATH_ALGORITHM, BOOK_V2_NATIVE_MATH_SET_ALGORITHM,
};
pub use crate::safe_vector::book_v2::{
    bind_book_v2_vectors, BookV2BoundVector, BookV2VectorBindingError, BookV2VectorBindings,
    BOOK_V2_VECTOR_BINDING_ALGORITHM, BOOK_V2_VECTOR_EPOCH_ALGORITHM, BOOK_V2_VECTOR_SET_ALGORITHM,
};
pub use frames::{
    layout_book_v2_body_inline_lines, prepare_book_v2_body_inline_frames,
    prepare_book_v2_body_inline_frames_counted, BookV2BodyInlineFrames,
    BookV2TableOccurrenceFrames, BOOK_V2_BODY_FRAMES_ALGORITHM,
};
use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
use typaxis_shaping::book_v2::BookV2AuthoredTextShape;
use typaxis_syntax::book_v2::{BookV2SourceVerificationBudget, PreparedBookV2TextFlow};

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
    // Absolute caller-ledger APIs retain the declared document ceiling. Local
    // projections share the residual allowance used to construct this owner.
    max_fragments: u64,
    output_record_limit: u64,
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
        &mut 0,
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
    prepare_book_v2_inline_items_counted(
        flow, shaped, admitted, bindings, limits, japanese_mode,
        &mut BookV2NativeMathBudgetObservation::default(), &mut 0,
    )
}

/// Observe owned native reservations independently from subsequent inline
/// reservations. Native layout units do not represent line-candidate work.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_inline_items_counted<'a>(
    flow: &'a PreparedBookV2TextFlow<'a>,
    shaped: &'a BookV2AuthoredTextShape<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    native_budget: &mut BookV2NativeMathBudgetObservation,
    observed_records: &mut u64,
) -> Result<BookV2PreparedInlines<'a>, ProductionInlinePreparationError> {
    let native = compute_book_v2_native_math_counted(bindings, admitted, limits, 0, 0, native_budget);
    *observed_records = native_budget.record_charge();
    let native = native.map_err(|e| {
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
        observed_records,
        None,
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
    prepare_book_v2_inline_items_with_native_context_counted(
        flow,
        shaped,
        admitted,
        bindings,
        limits,
        japanese_mode,
        native,
        &mut 0,
    )
}

/// Preserve native history and accepted inline reservations on every return path.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_inline_items_with_native_context_counted<'a>(
    flow: &'a PreparedBookV2TextFlow<'a>,
    shaped: &'a BookV2AuthoredTextShape<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    native: Option<&'a BookV2NativeMath<'a>>,
    observed_records: &mut u64,
) -> Result<BookV2PreparedInlines<'a>, ProductionInlinePreparationError> {
    prepare_inlines(
        flow, shaped, admitted, limits, bindings.epoch(), japanese_mode,
        Some(bindings), native.map(PreparedBookV2Math::Borrowed), observed_records, None,
    )
}

/// Keep full source revalidation history independent from native/inline output.
/// The same caller budget can span shaping, line passes and replayed graphs.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_inline_items_with_source_budget_counted<'a>(
    flow: &'a PreparedBookV2TextFlow<'a>,
    shaped: &'a BookV2AuthoredTextShape<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    native: Option<&'a BookV2NativeMath<'a>>,
    source_budget: &mut BookV2SourceVerificationBudget,
    observed_records: &mut u64,
) -> Result<BookV2PreparedInlines<'a>, ProductionInlinePreparationError> {
    source_budget.include_retained_records(native.map_or(0, |n| n.record_charge()));
    source_budget.include_retained_records(shaped.output_records());
    let result = prepare_inlines(
        flow,
        shaped,
        admitted,
        limits,
        bindings.epoch(),
        japanese_mode,
        Some(bindings),
        native.map(PreparedBookV2Math::Borrowed),
        observed_records,
        Some(&mut *source_budget),
    );
    source_budget.include_retained_records(*observed_records);
    result
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
    charge: &mut u64,
    mut source_budget: Option<&mut BookV2SourceVerificationBudget>,
) -> Result<BookV2PreparedInlines<'a>, ProductionInlinePreparationError> {
    *charge = native_math.as_ref().map_or(0, |n| n.record_charge());
    if let Some(bindings) = bindings {
        bindings
            .verify(bindings.body(), admitted, limits)
            .map_err(|_| {
                error(
                    NodeId::new(0),
                    ProductionInlinePreparationErrorKind::ReceiptMismatch,
                )
            })?;
    }
    let body = bindings.map_or(flow.body(), |b| b.body().styled());
    let verification = if let Some(budget) = source_budget.as_deref_mut() {
        budget.verify(flow, body, flow.navigation())
    } else {
        flow.verify_for(body, flow.navigation())
    };
    verification.map_err(|e| {
        let kind = match e.kind {
            typaxis_syntax::ProductionFlowErrorKind::NodeLimit => ProductionInlinePreparationErrorKind::UnitLimit,
            typaxis_syntax::ProductionFlowErrorKind::AllocationFailure => ProductionInlinePreparationErrorKind::AllocationFailure,
            _ => ProductionInlinePreparationErrorKind::ReceiptMismatch,
        };
        error(e.owner, kind)
    })?;
    if let Some(bindings) = bindings {
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
    let maximum_records = source_budget.as_deref()
        .map(|b| b.output_record_limit(limits.base().get().max_fragments)
            .ok_or_else(|| error(NodeId::new(0), ProductionInlinePreparationErrorKind::UnitLimit)))
        .transpose()?.unwrap_or(limits.base().get().max_fragments);
    let paragraphs = prepare_paragraphs_with_record_limit(
        InlineFlow::BookV2(flow),
        shaped.paragraphs(),
        bindings.map(InlineVectors::BookV2),
        native_math.as_deref().map(InlineNativeMath::BookV2),
        limits,
        japanese_mode,
        charge,
        maximum_records,
    )?;
    let figures = figure::prepare_figures(
        InlineFlow::BookV2(flow),
        InlineImages::BookV2(admitted),
        charge,
        maximum_records,
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
        output_record_limit: maximum_records,
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
        self.selected_line_contexts_counted(&mut 0)
    }
    /// Preserve accepted paragraph/context records when capture fails.
    pub fn selected_line_contexts_counted(
        &self,
        observed_records: &mut u64,
    ) -> Result<ProductionSelectedLineContexts, ProductionInlinePreparationError> {
        line_context::selected_contexts_counted(
            self.paragraphs(),
            self.output_records(),
            self.prepared.output_record_limit,
            self.fingerprint(),
            observed_records,
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
    layout_with_source_widths_counted(
        prepared,
        inline_sizes,
        max_candidate_steps,
        record_base,
        source_widths,
        &mut 0,
        &mut 0,
    )
}
fn layout_with_source_widths_counted<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    inline_sizes: &[PositiveLength],
    max_candidate_steps: u64,
    record_base: u64,
    source_widths: Option<&[Option<typaxis_linebreak::ProductionInlineSourceWidths<'_>>]>,
    consumed: &mut u64,
    observed_records: &mut u64,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    let projection = selected::project_lines_counted_with_records(
        selected::LineInputs {
            max_fragments: prepared.output_record_limit,
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
        consumed,
        observed_records,
    )?;
    Ok(BookV2InlineLineLayout {
        prepared,
        projection,
        frames: None,
    })
}

pub use layout_book_v2_inline_lines as layout_book_v2_text_lines;
