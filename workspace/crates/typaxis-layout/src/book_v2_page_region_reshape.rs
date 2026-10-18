//! Actual selected-line shaping feedback for one original page region.
use super::*;
use typaxis_linebreak::{
    BreakError, LineLayoutContext, LineReshapeFeedback, LineReshapeObservation,
    LineReshapePassRecord,
};
use typaxis_shaping::ProductionParagraphLineContext;
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
    records: u64,
}
impl BookV2PageRegionLineBudget {
    pub fn new(maximum_steps: u64, maximum_passes: u16) -> Self {
        Self {
            maximum_steps,
            remaining_steps: maximum_steps,
            maximum_passes,
            remaining_passes: maximum_passes,
            records: 0,
        }
    }
    /// Largest accepted record prefix across preparation, measurement and retries.
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    fn capture_contexts(
        &mut self,
        lines: &BookV2PageRegionLines<'_, '_>,
    ) -> Result<ProductionSelectedLineContexts, ProductionInlinePreparationError> {
        let mut records = 0;
        let result = lines.selected_line_contexts_counted(&mut records);
        self.records = self.records.max(records);
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
        prepared: &'p BookV2PageRegionInlines<'a>,
        selected: BookV2SelectedPageMaster<'a>,
    ) -> Result<BookV2PageRegionLines<'p, 'a>, BookV2PageRegionLayoutError> {
        let mut consumed = 0;
        let mut records = 0;
        let result = measure_region_counted(
            prepared,
            selected,
            self.remaining_steps,
            0,
            &mut consumed,
            &mut records,
        );
        self.records = self.records.max(records);
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
    with_region_lines_kernel(
        policy,
        flow,
        admitted,
        limits,
        epoch,
        japanese_mode,
        selected,
        allowance,
        prior_records,
        &mut RegionLineMode::Legacy,
        |stable, _| use_stable(stable),
    )
    .map_err(legacy_region_error)
}
/// Distinct allowance: accepted graph attempts never reuse another graph's positive record window.
///
/// ```
/// use typaxis_layout::book_v2::BookV2ObservedPageRegionLineBudget;
/// let b=BookV2ObservedPageRegionLineBudget::new(100,4);
/// assert_eq!((b.candidate_steps(),b.reshape_passes()),(0,0));
/// ```
///
/// ```compile_fail
/// use typaxis_layout::book_v2::BookV2ObservedPageRegionLineBudget;
/// let b=BookV2ObservedPageRegionLineBudget::new(100,4);
/// let _copy=b.clone();
/// ```
///
/// ```compile_fail
/// use typaxis_layout::book_v2::{BookV2ObservedPageRegionLineBudget,BookV2PageRegionLineBudget};
/// let _b=BookV2ObservedPageRegionLineBudget{inner:BookV2PageRegionLineBudget::new(100,4)};
/// ```
#[derive(Debug)]
pub struct BookV2ObservedPageRegionLineBudget {
    inner: BookV2PageRegionLineBudget,
}
impl BookV2ObservedPageRegionLineBudget {
    pub fn new(steps: u64, passes: u16) -> Self {
        Self {
            inner: BookV2PageRegionLineBudget::new(steps, passes),
        }
    }
    pub fn candidate_steps(&self) -> u64 {
        self.inner.candidate_steps()
    }
    pub fn reshape_passes(&self) -> u16 {
        self.inner.reshape_passes()
    }
    pub fn remaining_steps(&self) -> u64 {
        self.inner.remaining_steps()
    }
    pub fn remaining_passes(&self) -> u16 {
        self.inner.remaining_passes()
    }
    /// Stage-local diagnostic maximum; never global payment credit.
    pub fn record_charge(&self) -> u64 {
        self.inner.record_charge()
    }
}
#[allow(clippy::too_many_arguments)]
pub fn with_observed_book_v2_page_region_lines<'id, 'cells, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &BookV2PageRegionTextFlow<'_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    mode: JapaneseLineBreakMode,
    selected: BookV2SelectedPageMaster<'_>,
    allowance: &mut BookV2ObservedPageRegionLineBudget,
    accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
    inspect: impl FnOnce(
        BookV2ConvergedPageRegionLines<'_, '_, '_>,
        &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
    ) -> R,
) -> Result<R, BookV2ObservedPageRegionLayoutError> {
    with_region_lines_kernel(
        policy,
        flow,
        admitted,
        limits,
        epoch,
        mode,
        selected,
        &mut allowance.inner,
        0,
        &mut RegionLineMode::Observed(accounting),
        |stable, accounting| match accounting {
            RegionAccounting::Observed { accounting, .. } => inspect(stable, accounting),
            RegionAccounting::Legacy => unreachable!("closed observed adapter"),
        },
    )
}
enum RegionLoopOutcome<R> {
    Stable(R),
    Rebreak(ProductionSelectedLineContexts),
}
enum RegionLineMode<'loan, 'id, 'cells> {
    Legacy,
    Observed(&'loan mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>),
}
impl<'id, 'cells> RegionLineMode<'_, 'id, 'cells> {
    fn observed(&self) -> bool {
        matches!(self, Self::Observed(_))
    }
    fn with_graph<R>(
        &mut self,
        inspect: impl for<'window> FnOnce(&mut RegionAccounting<'_, 'id, 'cells, 'window>) -> R,
    ) -> R {
        match self {
            Self::Legacy => inspect(&mut RegionAccounting::Legacy),
            Self::Observed(accounting) => {
                accounting.with_zero_intrinsic_records(|accounting, window| {
                    inspect(&mut RegionAccounting::Observed { accounting, window })
                })
            }
        }
    }
    fn request<T>(
        &mut self,
        owner: NodeId,
        n: usize,
        r: u64,
    ) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        if let Self::Observed(accounting) = self {
            accounting
                .debit_work(owner, 1)
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
            let overflow = || {
                error(
                    owner,
                    ProductionInlinePreparationErrorKind::ArithmeticOverflow,
                )
            };
            let s = n
                .checked_mul(std::mem::size_of::<T>())
                .and_then(|n| u64::try_from(n).ok())
                .ok_or_else(overflow)?;
            let w = u64::try_from(n)
                .ok()
                .and_then(|n| n.checked_add(1))
                .ok_or_else(overflow)?;
            accounting
                .debit(owner, [r, s, w])
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
        }
        Ok(())
    }
    fn step(&mut self, owner: NodeId, n: u64) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        if let Self::Observed(accounting) = self {
            accounting
                .debit_work(owner, n)
                .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
        }
        Ok(())
    }
    fn boundary(
        &self,
        kind: line_context::RegionMetadataKind,
        owner: NodeId,
    ) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        if let Self::Observed(accounting) = self {
            line_context::region_metadata_boundary(kind, accounting.counters(), owner)?;
        }
        Ok(())
    }
    fn reserve_inputs<'a>(
        &mut self,
        owner: NodeId,
        inputs: &mut Vec<ProductionParagraphLineContext<'a>>,
        n: usize,
    ) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        self.input_request(owner, n)?;
        self.boundary(line_context::RegionMetadataKind::Inputs, owner)?;
        inputs
            .try_reserve_exact(n)
            .map_err(|_| BreakError::AllocationFailure)?;
        Ok(())
    }
    fn reserve_feedback<'a>(
        &mut self,
        owner: NodeId,
        request: typaxis_linebreak::LineReshapePassRequest<'a>,
    ) -> Result<typaxis_linebreak::LineReshapePassPermit<'a>, BookV2ObservedPageRegionLayoutError>
    {
        match self {
            Self::Legacy => reserve_region_feedback(owner, request, None),
            Self::Observed(accounting) => reserve_region_feedback(owner, request, Some(accounting)),
        }
    }
    fn input_request(
        &mut self,
        owner: NodeId,
        n: usize,
    ) -> Result<(), BookV2ObservedPageRegionLayoutError> {
        self.request::<ProductionParagraphLineContext<'_>>(owner, n, n as u64)
    }
}
// One concrete payment/reserve implementation for the real graph loop and
// its private request unit. The returned permit borrows only Linebreak owners.
fn reserve_region_feedback<'a>(
    owner: NodeId,
    request: typaxis_linebreak::LineReshapePassRequest<'a>,
    accounting: Option<&mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>>,
) -> Result<typaxis_linebreak::LineReshapePassPermit<'a>, BookV2ObservedPageRegionLayoutError> {
    if let Some(accounting) = accounting {
        accounting
            .debit_work(owner, 1)
            .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
        let target = request.requested_logical_target()?;
        accounting
            .debit_work(owner, 1)
            .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
        let overflow = || {
            error(
                owner,
                ProductionInlinePreparationErrorKind::ArithmeticOverflow,
            )
        };
        let spool = target
            .checked_mul(std::mem::size_of::<LineReshapePassRecord>())
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(overflow)?;
        let work = u64::try_from(target)
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or_else(overflow)?;
        accounting
            .debit(owner, [1, spool, work])
            .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
        line_context::region_metadata_boundary(
            line_context::RegionMetadataKind::Feedback,
            accounting.counters(),
            owner,
        )?;
    }
    request.reserve().map_err(Into::into)
}
fn measure_with_accounting<'p, 'a>(
    allowance: &mut BookV2PageRegionLineBudget,
    prepared: &'p BookV2PageRegionInlines<'a>,
    selected: BookV2SelectedPageMaster<'a>,
    accounting: &mut RegionAccounting<'_, '_, '_, '_>,
) -> Result<BookV2PageRegionLines<'p, 'a>, BookV2ObservedPageRegionLayoutError> {
    let mut consumed = 0;
    let mut records = 0;
    let maximum = accounting.limit(prepared.max_fragments)?;
    let result = measure_region_kernel(
        prepared,
        selected,
        allowance.remaining_steps,
        0,
        &mut consumed,
        &mut records,
        maximum,
        accounting,
    );
    allowance.records = allowance.records.max(records);
    allowance.remaining_steps -= consumed;
    result
}
#[allow(clippy::too_many_arguments)]
fn with_region_lines_kernel<'id, 'cells, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &BookV2PageRegionTextFlow<'_>,
    admitted: &AdmittedProductionResourceLedgerV3,
    limits: &M4EffectiveResourceLimits,
    epoch: [u8; 32],
    japanese_mode: JapaneseLineBreakMode,
    selected: BookV2SelectedPageMaster<'_>,
    allowance: &mut BookV2PageRegionLineBudget,
    prior_records: u64,
    mode: &mut RegionLineMode<'_, 'id, 'cells>,
    use_stable: impl FnOnce(
        BookV2ConvergedPageRegionLines<'_, '_, '_>,
        &mut RegionAccounting<'_, 'id, 'cells, '_>,
    ) -> R,
) -> Result<R, BookV2ObservedPageRegionLayoutError> {
    let mut use_stable = Some(use_stable);
    allowance.records = allowance.records.max(prior_records);
    if allowance.remaining_passes == 0 {
        return Err(BreakError::IterationLimit.into());
    }
    let owner = NodeId::new(flow.source().node_id);
    let initial_remaining_steps = allowance.remaining_steps;
    let (mut contexts, initial_state) = mode.with_graph(|accounting| {
        let mut records = 0;
        let shape = accounting.shape(policy, flow, admitted, limits, epoch, None, &mut records);
        allowance.records = allowance.records.max(records);
        let shape = shape?;
        let prepared = prepare_region_inlines_kernel(
            flow,
            &shape,
            admitted,
            limits,
            epoch,
            japanese_mode,
            prior_records,
            &mut records,
            accounting,
        );
        allowance.records = allowance.records.max(records);
        let prepared = prepared?;
        let lines = measure_with_accounting(allowance, &prepared, selected, accounting)?;
        let contexts = accounting.contexts(&lines, &mut records);
        allowance.records = allowance.records.max(records);
        Ok::<_, BookV2ObservedPageRegionLayoutError>((
            contexts?,
            accounting.state(owner, lines.fingerprint())?,
        ))
    })?;
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
        if !mode.observed() {
            retain(
                &mut records,
                1 + feedback.records().len() as u64,
                limits.base().get().max_fragments,
                owner,
            )?;
            allowance.records = allowance.records.max(records);
            for p in contexts.paragraphs() {
                retain(
                    &mut records,
                    2 + p.ends().len() as u64,
                    limits.base().get().max_fragments,
                    p.owner(),
                )?;
                allowance.records = allowance.records.max(records);
            }
        }
        let mut inputs = Vec::new();
        mode.reserve_inputs(owner, &mut inputs, contexts.paragraphs().len())?;
        for p in contexts.paragraphs() {
            mode.step(p.owner(), 1)?;
            inputs.push(ProductionParagraphLineContext {
                owner: p.owner(),
                ends: p.ends(),
            });
        }
        let observed = mode.observed();
        let outcome = mode.with_graph(
            |accounting| -> Result<RegionLoopOutcome<R>, BookV2ObservedPageRegionLayoutError> {
                let before_passes = budget.remaining_reshape_passes();
                let request = match feedback.prepare_pass_request(&mut budget) {
                    Ok(request) => {
                        if observed {
                            allowance.remaining_passes -= 1;
                        }
                        request
                    }
                    Err(e) => {
                        if observed {
                            allowance.remaining_passes -=
                                before_passes - budget.remaining_reshape_passes();
                        }
                        return Err(e.into());
                    }
                };
                // The real pass was consumed before this new request/payment boundary.
                let permit = match accounting {
                    RegionAccounting::Legacy => reserve_region_feedback(owner, request, None),
                    RegionAccounting::Observed { accounting, .. } => {
                        reserve_region_feedback(owner, request, Some(accounting))
                    }
                }?;
                if !observed {
                    allowance.remaining_passes -= 1;
                }
                let mut shape_records = 0;
                let shape = accounting.shape(
                    policy,
                    flow,
                    admitted,
                    limits,
                    epoch,
                    Some(&inputs),
                    &mut shape_records,
                );
                allowance.records = allowance.records.max(shape_records);
                let shape = shape?;
                let prepared = prepare_region_inlines_kernel(
                    flow,
                    &shape,
                    admitted,
                    limits,
                    epoch,
                    japanese_mode,
                    records,
                    &mut shape_records,
                    accounting,
                );
                allowance.records = allowance.records.max(shape_records);
                let prepared = prepared?;
                let lines = measure_with_accounting(allowance, &prepared, selected, accounting)?;
                let state = accounting.state(owner, lines.fingerprint())?;
                accounting.step(owner, 34)?;
                match permit.complete(state)? {
                    LineReshapeObservation::Stable => {
                        lines.check_fit()?;
                        accounting.seal();
                        Ok(RegionLoopOutcome::Stable(use_stable
                            .take()
                            .expect("stable consumer once")(
                            BookV2ConvergedPageRegionLines {
                                lines: &lines,
                                passes: feedback.records(),
                                candidate_steps: initial_remaining_steps
                                    - allowance.remaining_steps,
                            },
                            accounting,
                        )))
                    }
                    LineReshapeObservation::RebreakRequired => {
                        let contexts = accounting.contexts(&lines, &mut shape_records);
                        allowance.records = allowance.records.max(shape_records);
                        Ok(RegionLoopOutcome::Rebreak(contexts?))
                    }
                }
            },
        )?;
        match outcome {
            RegionLoopOutcome::Stable(result) => return Ok(result),
            RegionLoopOutcome::Rebreak(next) => contexts = next,
        }
    }
}

#[cfg(test)]
mod observed_region_pass_request_tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;
    #[test]
    fn book_v2_observed_region_input_request_keeps_all_axes_on_fault() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        for fault in [false, true] {
            let mut cells = [5, 7, 11];
            let (result, trace) =
                line_context::with_region_metadata_fault(fault.then_some(0), || {
                    let [r, s, w] = &mut cells;
                    with_book_v2_observed_source_accounting(r, s, w, &limits, 8, 16, |a| {
                        let mut out = Vec::new();
                        RegionLineMode::Observed(a).reserve_inputs(NodeId::new(4), &mut out, 3)
                    })
                });
            assert_eq!(
                cells,
                [
                    8,
                    7 + 3 * std::mem::size_of::<ProductionParagraphLineContext<'_>>() as u64,
                    16
                ]
            );
            assert_eq!(trace, [(line_context::RegionMetadataKind::Inputs, cells)]);
            assert_eq!(result.is_ok(), !fault);
        }
    }
    #[test]
    fn book_v2_observed_region_feedback_fault_preserves_real_begun_pass() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let state =
            super::super::super::super::reshape::selected_fingerprint_state([3; 32]).unwrap();
        for fault in [false, true] {
            let mut cells = [5, 7, 11];
            let mut feedback = LineReshapeFeedback::new(state);
            let mut context = LineLayoutContext::from_limits(&limits);
            let mut budget = context.take_budget().unwrap();
            let initial = budget.remaining_reshape_passes();
            let (result, trace) =
                line_context::with_region_metadata_fault(fault.then_some(0), || {
                    let [r, s, w] = &mut cells;
                    with_book_v2_observed_source_accounting(r, s, w, &limits, 6, 1000, |a| {
                        let request = feedback.prepare_pass_request(&mut budget).unwrap();
                        let permit = RegionLineMode::Observed(a)
                            .reserve_feedback(NodeId::new(7), request)?;
                        // Complete uses a separately paid closed fixed bound.
                        a.debit_work(NodeId::new(7), 34)
                            .map_err(BookV2ObservedPageRegionLayoutError::Accounting)?;
                        permit
                            .complete(state)
                            .map_err(BookV2ObservedPageRegionLayoutError::from)
                    })
                });
            assert_eq!(initial - budget.remaining_reshape_passes(), 1);
            let paid = [
                6,
                7 + std::mem::size_of::<LineReshapePassRecord>() as u64,
                15,
            ];
            assert_eq!(trace, [(line_context::RegionMetadataKind::Feedback, paid)]);
            assert_eq!(
                cells,
                if fault {
                    paid
                } else {
                    [paid[0], paid[1], paid[2] + 34]
                }
            );
            assert_eq!(result.is_ok(), !fault);
            assert_eq!(feedback.records().len(), usize::from(!fault));
        }
    }
}
