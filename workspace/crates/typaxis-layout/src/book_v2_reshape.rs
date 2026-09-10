//! Actual book-2 shaping/selection feedback; this is not page convergence.
use super::*;
use typaxis_core::Rect;
use typaxis_linebreak::{
    BreakError, LineLayoutContext, LineReshapeFeedback, LineReshapeObservation,
    LineReshapePassRecord,
};
use typaxis_shaping::{book_v2::shape_book_v2_authored_text, ProductionParagraphLineContext};
use typaxis_syntax::book_v2::BookV2ResourcePolicy;

/// Only an observed stable comparison can construct this callback-scoped view.
/// The final shape, inlines and frames cannot outlive the feedback owner.
pub struct BookV2ConvergedBodyLines<'s, 'p, 'a> {
    lines: &'s BookV2InlineLineLayout<'p, 'a>,
    footnotes: BookV2FootnoteLines<'s, 'p, 'a>,
    passes: &'s [LineReshapePassRecord],
    candidate_steps: u64,
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
    let native = compute_book_v2_native_math(bindings, admitted, limits, 0, 0).map_err(|e| {
        error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::NativeMath(e),
        )
    })?;
    with_converged_book_v2_body_lines_with_native_context(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        max_candidate_steps,
        native.as_ref(),
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
    let remaining_passes = remaining_passes.min(limits.base().get().max_line_reshape_passes);
    if remaining_passes == 0 {
        return Err(BreakError::IterationLimit.into());
    }
    let epoch = bindings.epoch();
    let mut remaining_steps = max_candidate_steps;
    let (mut contexts, initial_state) = {
        let shape = shape_book_v2_authored_text(policy, flow, admitted, limits, epoch, None)?;
        let prepared = prepare_book_v2_inline_items_with_native_context(
            flow,
            &shape,
            admitted,
            bindings,
            limits,
            japanese_mode,
            native,
        )?;
        let selected = super::frames::layout_body_lines_with_source_widths(
            &prepared,
            body,
            remaining_steps,
            page_plan,
            source_widths,
        )?;
        remaining_steps -= selected.candidate_steps();
        (
            selected.selected_line_contexts()?,
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
        let shape =
            shape_book_v2_authored_text(policy, flow, admitted, limits, epoch, Some(&inputs))?;
        let prepared = prepare_book_v2_inline_items_with_native_context(
            flow,
            &shape,
            admitted,
            bindings,
            limits,
            japanese_mode,
            native,
        )?;
        let selected = super::frames::layout_body_lines_with_source_widths(
            &prepared,
            body,
            remaining_steps,
            page_plan,
            source_widths,
        )?;
        remaining_steps -= selected.candidate_steps();
        match permit.complete(super::super::reshape::selected_fingerprint_state(
            selected.fingerprint(),
        )?)? {
            LineReshapeObservation::Stable => {
                return Ok(use_stable(BookV2ConvergedBodyLines {
                    lines: &selected,
                    footnotes: prepare_book_v2_footnote_lines(&selected, limits)?,
                    passes: feedback.records(),
                    candidate_steps: max_candidate_steps - remaining_steps,
                }))
            }
            LineReshapeObservation::RebreakRequired => {
                contexts = selected.selected_line_contexts()?
            }
        }
    }
}
