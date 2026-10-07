//! Actual shaping and line feedback in a source-bound column measurement
//! envelope. A physical page still needs joint column/note selection.
use super::*;
use typaxis_syntax::book_v2::BookV2ColumnFramePlan;

/// Callback-scoped stable column measurements. Neither the maximum envelope
/// nor caller source-width assignments are physical placement receipts.
///
/// ```compile_fail
/// use typaxis_layout::book_v2::{BookV2ConvergedBodyLines, BookV2ConvergedColumnLines};
/// fn single<'s, 'p, 'a>(column: BookV2ConvergedColumnLines<'s, 'p, 'a>)
///     -> BookV2ConvergedBodyLines<'s, 'p, 'a> { column }
/// ```
pub struct BookV2ConvergedColumnLines<'s, 'p, 'a> {
    stable: BookV2ConvergedBodyLines<'s, 'p, 'a>,
}
impl<'s, 'p, 'a> BookV2ConvergedColumnLines<'s, 'p, 'a> {
    pub fn lines(&self) -> &'s BookV2InlineLineLayout<'p, 'a> {
        self.stable.lines()
    }
    pub fn footnotes(&self) -> &BookV2FootnoteLines<'s, 'p, 'a> {
        self.stable.footnotes()
    }
    pub fn column_plan(&self) -> &'p BookV2ColumnFramePlan<'a> {
        self.lines()
            .frames()
            .expect("measured frames")
            .column_plan()
            .expect("column plan")
    }
    pub fn passes(&self) -> &'s [LineReshapePassRecord] {
        self.stable.passes()
    }
    pub fn candidate_steps(&self) -> u64 {
        self.stable.candidate_steps()
    }
    pub fn source_record_charge(&self) -> u64 {
        self.stable.source_record_charge()
    }
    pub fn retained_record_charge(&self) -> u64 {
        self.stable.retained_record_charge()
    }
}

/// Reuse admitted originals, immutable native math and caller work/record/pass
/// history across actual column-width shaping/selection passes. The callback
/// receives the final shape only after an observed stable comparison.
#[allow(clippy::too_many_arguments)]
pub fn with_budgeted_book_v2_column_lines<'a, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    native: Option<&BookV2NativeMath<'_>>,
    allowance: &mut BookV2BodyLineBudget,
    columns: &BookV2ColumnFramePlan<'a>,
    source_widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    use_stable: impl FnOnce(BookV2ConvergedColumnLines<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    columns.verify(flow.body()).map_err(|_| {
        error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::ReceiptMismatch,
        )
    })?;
    with_budgeted_lines_in_measured_frames(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        columns.measurement_body(),
        native,
        allowance,
        Some(MeasurementPageFrames::Columns(columns)),
        source_widths,
        |stable| use_stable(BookV2ConvergedColumnLines { stable }),
    )
}
