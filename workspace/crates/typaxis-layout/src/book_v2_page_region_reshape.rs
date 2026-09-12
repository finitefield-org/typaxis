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
    if remaining_passes == 0 {
        return Err(BreakError::IterationLimit.into());
    }
    let owner = NodeId::new(flow.source().node_id);
    let mut remaining_steps = max_candidate_steps;
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
        let lines = measure_region(&prepared, selected, remaining_steps, 0)?;
        remaining_steps -= lines.candidate_steps();
        (
            lines.selected_line_contexts()?,
            super::super::super::reshape::selected_fingerprint_state(lines.fingerprint())?,
        )
    };
    let mut feedback = LineReshapeFeedback::new(initial_state);
    let mut context = LineLayoutContext::from_limits(limits.base());
    let mut budget = context.take_budget()?;
    loop {
        if feedback.records().len() >= usize::from(remaining_passes) {
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
        let lines = measure_region(&prepared, selected, remaining_steps, 0)?;
        remaining_steps -= lines.candidate_steps();
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
                    candidate_steps: max_candidate_steps - remaining_steps,
                }));
            }
            LineReshapeObservation::RebreakRequired => contexts = lines.selected_line_contexts()?,
        }
    }
}
