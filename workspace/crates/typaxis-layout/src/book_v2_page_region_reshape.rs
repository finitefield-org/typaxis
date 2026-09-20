//! Actual selected-line shaping feedback for one original page region.
use super::*;
use typaxis_linebreak::{
    BreakError, LineLayoutContext, LineReshapeFeedback, LineReshapeObservation,
    LineReshapePassRecord,
};
use typaxis_shaping::{book_v2::shape_book_v2_page_region_text, ProductionParagraphLineContext};
use typaxis_syntax::book_v2::BookV2ResourcePolicy;

/// Only a stable shape/selection comparison constructs this borrowed view.
/// The view is still not a body flow, resource closure or PDF paint receipt.
pub struct BookV2ConvergedPageRegionLines<'s, 'p, 'a> {
    lines: &'s BookV2PageRegionLines<'p, 'a>,
    passes: &'s [LineReshapePassRecord],
    candidate_steps: u64,
}
impl<'s, 'p, 'a> BookV2ConvergedPageRegionLines<'s, 'p, 'a> {
    pub fn lines(&self) -> &'s BookV2PageRegionLines<'p, 'a> {
        self.lines
    }
    pub fn passes(&self) -> &'s [LineReshapePassRecord] {
        self.passes
    }
    pub fn candidate_steps(&self) -> u64 {
        self.candidate_steps
    }
}

/// Caller-owned allowance shared across regions and retries. Counts actual
/// candidate visits and begun reshape attempts, including those ending in errors.
/// It does not count the shaping backend's internal operations.
#[derive(Debug)]
pub struct BookV2PageRegionLineBudget {
    maximum_steps: u64,
    remaining_steps: u64,
    maximum_passes: u16,
    remaining_passes: u16,
}
impl BookV2PageRegionLineBudget {
    pub fn new(maximum_steps: u64, maximum_passes: u16) -> Self {
        Self {
            maximum_steps,
            remaining_steps: maximum_steps,
            maximum_passes,
            remaining_passes: maximum_passes,
        }
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
        prepared: &'p BookV2PageRegionInlines<'a>,
        selected: BookV2SelectedPageMaster<'a>,
    ) -> Result<BookV2PageRegionLines<'p, 'a>, BookV2PageRegionLayoutError> {
        let mut consumed = 0;
        let result =
            measure_region_counted(prepared, selected, self.remaining_steps, 0, &mut consumed);
        // The projection cannot spend beyond the allowance it was given.
        self.remaining_steps -= consumed;
        result
    }
}

/// Candidate work spans the initial break and every reshape. `prior_records`
/// accounts for other retained owners; previous contexts and feedback records
/// coexist with the current shape/selection and are charged before allocation.
/// These stage ceilings are not the complete command-wide failed-work ledger.
#[allow(clippy::too_many_arguments)]
pub fn with_converged_book_v2_page_region_lines<R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &BookV2PageRegionTextFlow<'_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    japanese_mode: JapaneseLineBreakMode,
    selected: BookV2SelectedPageMaster<'_>,
    max_candidate_steps: u64,
    remaining_passes: u16,
    prior_records: u64,
    use_stable: impl FnOnce(BookV2ConvergedPageRegionLines<'_, '_, '_>) -> R,
) -> Result<R, BookV2PageRegionLayoutError> {
    let mut allowance = BookV2PageRegionLineBudget::new(max_candidate_steps, remaining_passes);
    with_budgeted_book_v2_page_region_lines(
        policy,
        flow,
        admitted,
        limits,
        epoch,
        japanese_mode,
        selected,
        &mut allowance,
        prior_records,
        use_stable,
    )
}

/// Retains completed work and begun passes even if shaping, line selection,
/// height validation or the consumer fails. Reuse the same allowance for retries.
#[allow(clippy::too_many_arguments)]
pub fn with_budgeted_book_v2_page_region_lines<R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &BookV2PageRegionTextFlow<'_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    japanese_mode: JapaneseLineBreakMode,
    selected: BookV2SelectedPageMaster<'_>,
    allowance: &mut BookV2PageRegionLineBudget,
    prior_records: u64,
    use_stable: impl FnOnce(BookV2ConvergedPageRegionLines<'_, '_, '_>) -> R,
) -> Result<R, BookV2PageRegionLayoutError> {
    if allowance.remaining_passes == 0 {
        return Err(BreakError::IterationLimit.into());
    }
    let owner = NodeId::new(flow.source().node_id);
    let initial_remaining_steps = allowance.remaining_steps;
    let (mut contexts, initial_state) = {
        let shape = shape_book_v2_page_region_text(policy, flow, admitted, limits, epoch, None)?;
        let prepared = prepare_book_v2_page_region_inlines(
            flow,
            &shape,
            admitted,
            limits,
            epoch,
            japanese_mode,
            prior_records,
        )?;
        let lines = allowance.measure(&prepared, selected)?;
        (
            lines.selected_line_contexts()?,
            super::super::super::reshape::selected_fingerprint_state(lines.fingerprint())?,
        )
    };
    let mut feedback = LineReshapeFeedback::new(initial_state);
    let mut context = LineLayoutContext::from_limits(limits.base());
    let mut budget = context.take_budget()?;
    loop {
        if allowance.remaining_passes == 0 {
            return Err(BreakError::IterationLimit.into());
        }
        // Context charge also included the now-dropped prior selection. Count
        // only the retained owned contexts and borrowed shaper views here.
        let mut records = prior_records;
        retain(
            &mut records,
            1 + feedback.records().len() as u64,
            limits.base().get().max_fragments,
            owner,
        )?;
        for p in contexts.paragraphs() {
            retain(
                &mut records,
                2 + p.ends().len() as u64,
                limits.base().get().max_fragments,
                p.owner(),
            )?;
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
        let shape =
            shape_book_v2_page_region_text(policy, flow, admitted, limits, epoch, Some(&inputs))?;
        let prepared = prepare_book_v2_page_region_inlines(
            flow,
            &shape,
            admitted,
            limits,
            epoch,
            japanese_mode,
            records,
        )?;
        let lines = allowance.measure(&prepared, selected)?;
        match permit.complete(super::super::super::reshape::selected_fingerprint_state(
            lines.fingerprint(),
        )?)? {
            LineReshapeObservation::Stable => {
                // An initial full-paragraph shape may be taller than its final
                // line-local shape. Reject overflow only after stability here.
                lines.check_fit()?;
                return Ok(use_stable(BookV2ConvergedPageRegionLines {
                    lines: &lines,
                    passes: feedback.records(),
                    candidate_steps: initial_remaining_steps - allowance.remaining_steps,
                }));
            }
            LineReshapeObservation::RebreakRequired => contexts = lines.selected_line_contexts()?,
        }
    }
}
