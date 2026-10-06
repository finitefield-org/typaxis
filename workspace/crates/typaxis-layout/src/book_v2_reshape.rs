//! Actual book-2 shaping/selection feedback; this is not page convergence.
use super::*;
use typaxis_core::Rect;
use typaxis_linebreak::{
    BreakError, LineLayoutContext, LineReshapeFeedback, LineReshapeObservation,
    LineReshapePassRecord,
};
use typaxis_shaping::{
    book_v2::shape_book_v2_authored_text_with_source_budget_counted, ProductionParagraphLineContext,
};
use typaxis_syntax::book_v2::BookV2ResourcePolicy;

/// Only an observed stable comparison can construct this callback-scoped view.
/// The final shape, inlines and frames cannot outlive the feedback owner.
pub struct BookV2ConvergedBodyLines<'s, 'p, 'a> {
    lines: &'s BookV2InlineLineLayout<'p, 'a>,
    footnotes: BookV2FootnoteLines<'s, 'p, 'a>,
    passes: &'s [LineReshapePassRecord],
    candidate_steps: u64,
    source_records: u64,
    retained_records: u64,
}
impl<'s, 'p, 'a> BookV2ConvergedBodyLines<'s, 'p, 'a> {
    pub fn footnotes(&self) -> &BookV2FootnoteLines<'s, 'p, 'a> {
        &self.footnotes
    }
    pub fn lines(&self) -> &'s BookV2InlineLineLayout<'p, 'a> {
        self.lines
    }
    pub fn passes(&self) -> &'s [LineReshapePassRecord] {
        self.passes
    }
    pub fn candidate_steps(&self) -> u64 {
        self.candidate_steps
    }
    /// Caller history plus every accepted source-reconstruction reservation.
    /// Intrinsic line/footnote records remain separate.
    pub fn source_record_charge(&self) -> u64 {
        self.source_records
    }
    /// Largest accepted intrinsic graph prefix, including context capture.
    pub fn retained_record_charge(&self) -> u64 {
        self.retained_records
    }
}
#[allow(clippy::too_many_arguments)]
pub fn with_converged_book_v2_body_lines<R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    max_candidate_steps: u64,
    use_stable: impl FnOnce(BookV2ConvergedBodyLines<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    let mut allowance = BookV2BodyLineBudget::new(
        max_candidate_steps,
        limits.base().get().max_line_reshape_passes,
    );
    with_budgeted_book_v2_body_lines(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        &mut allowance,
        &mut BookV2NativeMathBudgetObservation::default(),
        use_stable,
    )
}

/// Construct owned native math and retain its reservation independently from
/// line-candidate work. Reuse the line owner to retain begun passes and records.
#[allow(clippy::too_many_arguments)]
pub fn with_budgeted_book_v2_body_lines<R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    allowance: &mut BookV2BodyLineBudget,
    native_budget: &mut BookV2NativeMathBudgetObservation,
    use_stable: impl FnOnce(BookV2ConvergedBodyLines<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    let native =
        compute_book_v2_native_math_counted(bindings, admitted, limits, 0, 0, native_budget);
    allowance.records = allowance.records.max(native_budget.record_charge());
    let native = native.map_err(|e| {
        error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::NativeMath(e),
        )
    })?;
    with_budgeted_book_v2_body_lines_with_source_widths(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        native.as_ref(),
        allowance,
        None,
        None,
        use_stable,
    )
}
/// Reuse immutable native computations across line passes and later page passes.
/// Candidate work covers the initial break and all reshapes. Fragment ceilings
/// remain stage-local; complete page/allocation accounting belongs downstream.
#[allow(clippy::too_many_arguments)]
pub fn with_converged_book_v2_body_lines_with_native_context<R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    max_candidate_steps: u64,
    native: Option<&BookV2NativeMath<'_>>,
    use_stable: impl FnOnce(BookV2ConvergedBodyLines<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    with_converged_book_v2_body_lines_with_remaining_passes(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        max_candidate_steps,
        native,
        limits.base().get().max_line_reshape_passes,
        use_stable,
    )
}
/// Continue a command-wide reshape allowance without changing source limits or
/// resource identity. The initial break is not a reshape pass.
#[allow(clippy::too_many_arguments)]
pub fn with_converged_book_v2_body_lines_with_remaining_passes<R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    max_candidate_steps: u64,
    native: Option<&BookV2NativeMath<'_>>,
    remaining_passes: u16,
    use_stable: impl FnOnce(BookV2ConvergedBodyLines<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    with_converged_book_v2_body_lines_in_page_frames(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        max_candidate_steps,
        native,
        remaining_passes,
        None,
        use_stable,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn with_converged_book_v2_body_lines_in_page_frames<'a, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    max_candidate_steps: u64,
    native: Option<&BookV2NativeMath<'_>>,
    remaining_passes: u16,
    page_plan: Option<&typaxis_syntax::book_v2::BookV2PageFramePlan<'a>>,
    use_stable: impl FnOnce(BookV2ConvergedBodyLines<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    with_converged_book_v2_body_lines_with_source_widths(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        max_candidate_steps,
        native,
        remaining_passes,
        page_plan,
        None,
        use_stable,
    )
}
/// Rebind immutable source-start assignments after each actual shaping pass.
/// Selected physical page widths must still satisfy the page-plan contract.
#[allow(clippy::too_many_arguments)]
pub fn with_converged_book_v2_body_lines_with_source_widths<'a, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    max_candidate_steps: u64,
    native: Option<&BookV2NativeMath<'_>>,
    remaining_passes: u16,
    page_plan: Option<&typaxis_syntax::book_v2::BookV2PageFramePlan<'a>>,
    source_widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    use_stable: impl FnOnce(BookV2ConvergedBodyLines<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    let mut allowance = BookV2BodyLineBudget::new(max_candidate_steps, remaining_passes);
    with_budgeted_book_v2_body_lines_with_source_widths(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        native,
        &mut allowance,
        page_plan,
        source_widths,
        use_stable,
    )
}

/// Reuse this owner across attempts to retain begun reshape passes and all
/// charged frame/source/line-candidate work, including work preceding failure.
#[allow(clippy::too_many_arguments)]
pub fn with_budgeted_book_v2_body_lines_with_source_widths<'a, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    native: Option<&BookV2NativeMath<'_>>,
    allowance: &mut BookV2BodyLineBudget,
    page_plan: Option<&typaxis_syntax::book_v2::BookV2PageFramePlan<'a>>,
    source_widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    use_stable: impl FnOnce(BookV2ConvergedBodyLines<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    let remaining_passes = allowance
        .remaining_passes
        .min(limits.base().get().max_line_reshape_passes);
    if remaining_passes == 0 {
        return Err(BreakError::IterationLimit.into());
    }
    let epoch = bindings.epoch();
    let initial_steps = allowance.remaining_steps;
    let (mut contexts, initial_state) = {
        let mut records = 0;
        let shape = shape_book_v2_authored_text_with_source_budget_counted(
            policy,
            flow,
            admitted,
            limits,
            epoch,
            None,
            &mut allowance.source_records,
            &mut records,
        );
        allowance.records = allowance.records.max(records);
        let shape = shape?;
        let prepared = prepare_book_v2_inline_items_with_source_budget_counted(
            flow,
            &shape,
            admitted,
            bindings,
            limits,
            japanese_mode,
            native,
            &mut allowance.source_records,
            &mut records,
        );
        allowance.records = allowance.records.max(records);
        let prepared = prepared?;
        let selected = allowance.measure(&prepared, body, page_plan, source_widths)?;
        (
            allowance.capture_contexts(&selected)?,
            super::super::reshape::selected_fingerprint_state(selected.fingerprint())?,
        )
    };
    let mut feedback = LineReshapeFeedback::new(initial_state);
    let mut context = LineLayoutContext::from_limits(limits.base());
    let mut budget = context.take_budget()?;
    loop {
        if feedback.records().len() >= usize::from(remaining_passes) {
            return Err(BreakError::IterationLimit.into());
        }
        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(contexts.paragraphs().len())
            .map_err(|_| BreakError::AllocationFailure)?;
        inputs.extend(
            contexts
                .paragraphs()
                .iter()
                .map(|p| ProductionParagraphLineContext {
                    owner: p.owner(),
                    ends: p.ends(),
                }),
        );
        let permit = feedback.begin_pass(&mut budget)?;
        allowance.remaining_passes -= 1;
        let mut records = 0;
        let shape = shape_book_v2_authored_text_with_source_budget_counted(
            policy,
            flow,
            admitted,
            limits,
            epoch,
            Some(&inputs),
            &mut allowance.source_records,
            &mut records,
        );
        allowance.records = allowance.records.max(records);
        let shape = shape?;
        let prepared = prepare_book_v2_inline_items_with_source_budget_counted(
            flow,
            &shape,
            admitted,
            bindings,
            limits,
            japanese_mode,
            native,
            &mut allowance.source_records,
            &mut records,
        );
        allowance.records = allowance.records.max(records);
        let prepared = prepared?;
        let selected = allowance.measure(&prepared, body, page_plan, source_widths)?;
        match permit.complete(super::super::reshape::selected_fingerprint_state(
            selected.fingerprint(),
        )?)? {
            LineReshapeObservation::Stable => {
                let mut records = 0;
                let footnotes =
                    prepare_book_v2_footnote_lines_counted(&selected, limits, &mut records);
                allowance.records = allowance.records.max(records);
                return Ok(use_stable(BookV2ConvergedBodyLines {
                    lines: &selected,
                    footnotes: footnotes?,
                    passes: feedback.records(),
                    candidate_steps: initial_steps - allowance.remaining_steps,
                    source_records: allowance.source_record_charge(),
                    retained_records: allowance.record_charge(),
                }));
            }
            LineReshapeObservation::RebreakRequired => {
                contexts = allowance.capture_contexts(&selected)?
            }
        }
    }
}

/// Candidate/frame/source-width work and begun reshape passes. Shaper internal
/// operations and allocation byte totals are outside this allowance.
#[derive(Debug)]
pub struct BookV2BodyLineBudget {
    maximum_steps: u64,
    remaining_steps: u64,
    maximum_passes: u16,
    remaining_passes: u16,
    records: u64,
    source_records: BookV2SourceVerificationBudget,
}
impl BookV2BodyLineBudget {
    pub fn new(maximum_steps: u64, maximum_passes: u16) -> Self {
        Self::new_with_source_records(maximum_steps, maximum_passes, 0, u64::MAX)
    }
    /// Carry command history into each full source reconstruction. The source
    /// body's ceiling still applies when it is smaller than the caller ceiling.
    pub fn new_with_source_records(
        maximum_steps: u64,
        maximum_passes: u16,
        prior_records: u64,
        maximum_records: u64,
    ) -> Self {
        Self {
            maximum_steps,
            remaining_steps: maximum_steps,
            maximum_passes,
            remaining_passes: maximum_passes,
            records: 0,
            source_records: BookV2SourceVerificationBudget::new(prior_records, maximum_records),
        }
    }
    /// Largest accepted local output prefix, excluding source reconstructions;
    /// not a sum of released pass storage.
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    /// Caller history and all source reservations, including failed attempts.
    pub fn source_record_charge(&self) -> u64 {
        self.source_records.record_charge()
    }
    fn capture_contexts(
        &mut self,
        selected: &BookV2InlineLineLayout<'_, '_>,
    ) -> Result<ProductionSelectedLineContexts, ProductionInlinePreparationError> {
        let mut records = 0;
        let result = selected.selected_line_contexts_counted(&mut records);
        self.records = self.records.max(records);
        self.source_records.include_retained_records(self.records);
        result
    }
    pub fn candidate_steps(&self) -> u64 {
        self.maximum_steps - self.remaining_steps
    }
    pub fn reshape_passes(&self) -> u16 {
        self.maximum_passes - self.remaining_passes
    }
    pub fn remaining_steps(&self) -> u64 {
        self.remaining_steps
    }
    pub fn remaining_passes(&self) -> u16 {
        self.remaining_passes
    }
    fn measure<'p, 'a>(
        &mut self,
        prepared: &'p BookV2PreparedInlines<'a>,
        body: Rect,
        page_plan: Option<&'p typaxis_syntax::book_v2::BookV2PageFramePlan<'a>>,
        source_widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    ) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
        let mut consumed = 0;
        let mut records = 0;
        let result = super::frames::layout_body_lines_counted_with_records(
            prepared,
            body,
            self.remaining_steps,
            page_plan,
            source_widths,
            &mut consumed,
            &mut records,
        );
        self.remaining_steps -= consumed;
        self.records = self.records.max(records);
        self.source_records.include_retained_records(self.records);
        result
    }
}
