//! Retain converged source contexts so independent physical variants can coexist.
//! These seeds and rebuilds are not page selection or repeated-paint authorization.
use super::frames::MeasurementPageFrames;
use super::*;
use typaxis_core::Rect;
use typaxis_linebreak::BreakError;
use typaxis_shaping::{
    book_v2::shape_book_v2_authored_text_with_source_budget_counted, ProductionParagraphLineContext,
};
use typaxis_syntax::book_v2::{BookV2PageFramePlan, BookV2ResourcePolicy};

/// All replay inputs remain borrowed and immutable. No caller-supplied hash or
/// changed width/label/resource context can be substituted during reconstruction.
pub struct BookV2BodyLineVariantSeed<'a> {
    policy: &'a BookV2ResourcePolicy<'a>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &'a M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    native: Option<&'a BookV2NativeMath<'a>>,
    measured_frames: Option<MeasurementPageFrames<'a, 'a>>,
    source_widths: Option<&'a BookV2SourceWidthAssignments<'a, 'a>>,
    contexts: ProductionSelectedLineContexts,
    rebuild_records: u64,
    captured_records: u64,
    records: u64,
    work: u64,
    passes: u16,
    source_records: u64,
    retained_records: u64,
}
impl<'a> BookV2BodyLineVariantSeed<'a> {
    pub fn source_flow(&self) -> &PreparedBookV2TextFlow<'_> {
        self.flow
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.contexts.source_fingerprint()
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
    pub fn reshape_passes(&self) -> u16 {
        self.passes
    }
    /// Earlier caller history plus this seed's full source reconstructions.
    pub fn source_record_charge(&self) -> u64 {
        self.source_records
    }
    pub fn retained_record_charge(&self) -> u64 {
        self.retained_records
    }
    pub fn contexts(&self) -> &ProductionSelectedLineContexts {
        &self.contexts
    }

    /// Converge a sibling at new source widths while retaining the exact source,
    /// admitted resources, native math and page plan. Line contexts are recomputed
    /// for these widths; the original seed and its replay inputs stay immutable.
    pub fn prepare_with_source_widths<'b>(
        &'b self,
        widths: &'b BookV2SourceWidthAssignments<'b, 'b>,
        maximum_work: u64,
        prior_records: u64,
        remaining_passes: u16,
    ) -> Result<BookV2BodyLineVariantSeed<'b>, ProductionBodyReshapeError> {
        if !widths.matches_flow(self.flow) {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            )
            .into());
        }
        let mut allowance = BookV2LineVariantBudget::new(maximum_work, remaining_passes);
        prepare_variant_seed_in_measured_frames(
            self.policy,
            self.flow,
            self.admitted,
            self.bindings,
            self.limits,
            self.japanese_mode,
            self.body,
            &mut allowance,
            prior_records,
            self.native,
            self.measured_frames,
            Some(widths),
        )
    }
    pub fn prepare_budgeted_with_source_widths<'b>(
        &'b self,
        widths: &'b BookV2SourceWidthAssignments<'b, 'b>,
        allowance: &mut BookV2LineVariantBudget,
        prior_records: u64,
    ) -> Result<BookV2BodyLineVariantSeed<'b>, ProductionBodyReshapeError> {
        allowance.records = allowance.records.max(prior_records);
        if !widths.matches_flow(self.flow) {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            )
            .into());
        }
        prepare_variant_seed_in_measured_frames(
            self.policy,
            self.flow,
            self.admitted,
            self.bindings,
            self.limits,
            self.japanese_mode,
            self.body,
            allowance,
            prior_records,
            self.native,
            self.measured_frames,
            Some(widths),
        )
    }
}

/// Caller-owned work and begun passes for convergence plus context capture.
/// Replay charges can share this owner without consuming reshape passes.
/// Initial frame construction and shape internals are not fully counted here.
#[derive(Debug)]
pub struct BookV2LineVariantBudget {
    maximum_work: u64,
    maximum_passes: u16,
    work: u64,
    passes: u16,
    records: u64,
}
impl BookV2LineVariantBudget {
    pub fn new(maximum_work: u64, maximum_passes: u16) -> Self {
        Self {
            maximum_work,
            maximum_passes,
            work: 0,
            passes: 0,
            records: 0,
        }
    }
    /// Accepted cumulative prefix from seed/capture/replay construction.
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
    pub fn reshape_passes(&self) -> u16 {
        self.passes
    }
    pub fn remaining_work(&self) -> u64 {
        self.maximum_work - self.work
    }
    pub fn remaining_passes(&self) -> u16 {
        self.maximum_passes - self.passes
    }
}

/// Prepare one independently converged variant. Initial shaping retains the
/// existing stage-local record ceilings; seed/context storage includes prior
/// records and is checked before allocation. Work spans convergence and capture.
#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_body_line_variant_seed<'a>(
    policy: &'a BookV2ResourcePolicy<'a>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &'a M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    maximum_work: u64,
    prior_records: u64,
    native: Option<&'a BookV2NativeMath<'a>>,
    remaining_passes: u16,
    page_plan: Option<&'a BookV2PageFramePlan<'a>>,
    source_widths: Option<&'a BookV2SourceWidthAssignments<'a, 'a>>,
) -> Result<BookV2BodyLineVariantSeed<'a>, ProductionBodyReshapeError> {
    let mut allowance = BookV2LineVariantBudget::new(maximum_work, remaining_passes);
    prepare_budgeted_book_v2_body_line_variant_seed(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        &mut allowance,
        prior_records,
        native,
        page_plan,
        source_widths,
    )
}

/// Keep accepted convergence/capture work and begun passes even when no seed
/// can be returned. A successful seed reports only this invocation's charges.
#[allow(clippy::too_many_arguments)]
pub fn prepare_budgeted_book_v2_body_line_variant_seed<'a>(
    policy: &'a BookV2ResourcePolicy<'a>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &'a M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    allowance: &mut BookV2LineVariantBudget,
    prior_records: u64,
    native: Option<&'a BookV2NativeMath<'a>>,
    page_plan: Option<&'a BookV2PageFramePlan<'a>>,
    source_widths: Option<&'a BookV2SourceWidthAssignments<'a, 'a>>,
) -> Result<BookV2BodyLineVariantSeed<'a>, ProductionBodyReshapeError> {
    prepare_variant_seed_in_measured_frames(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        allowance,
        prior_records,
        native,
        page_plan.map(MeasurementPageFrames::Pages),
        source_widths,
    )
}
#[allow(clippy::too_many_arguments)]
fn prepare_variant_seed_in_measured_frames<'a>(
    policy: &'a BookV2ResourcePolicy<'a>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &'a M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    allowance: &mut BookV2LineVariantBudget,
    prior_records: u64,
    native: Option<&'a BookV2NativeMath<'a>>,
    measured_frames: Option<MeasurementPageFrames<'a, 'a>>,
    source_widths: Option<&'a BookV2SourceWidthAssignments<'a, 'a>>,
) -> Result<BookV2BodyLineVariantSeed<'a>, ProductionBodyReshapeError> {
    allowance.records = allowance.records.max(prior_records);
    if prior_records >= limits.base().get().max_fragments {
        return Err(error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::UnitLimit,
        )
        .into());
    }
    let maximum_work = allowance.remaining_work();
    let mut line_budget = BookV2BodyLineBudget::new_with_source_records(
        maximum_work,
        allowance.remaining_passes(),
        allowance.records,
        limits.base().get().max_fragments,
    );
    let mut capture_work = 0;
    let mut capture_records = prior_records;
    let result = super::feedback::with_budgeted_lines_in_measured_frames(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        native,
        &mut line_budget,
        measured_frames,
        source_widths,
        |stable| -> Result<_, ProductionBodyReshapeError> {
            let lines = stable.lines();
            let capture_maximum = maximum_work - stable.candidate_steps();
            for p in lines.paragraphs() {
                take_work(&mut capture_work, 3, capture_maximum)?;
                for line in p.lines() {
                    take_work(
                        &mut capture_work,
                        (line.items().len() as u64)
                            .checked_add(2)
                            .ok_or(BreakError::IterationLimit)?,
                        capture_maximum,
                    )?;
                }
            }
            take_work(&mut capture_work, 1, capture_maximum)?;
            let rebuild_records = stable.footnotes().record_charge();
            let source_prior = stable
                .source_record_charge()
                .checked_add(stable.retained_record_charge())
                .filter(|n| *n <= limits.base().get().max_fragments)
                .ok_or_else(|| {
                    error(
                        NodeId::new(0),
                        ProductionInlinePreparationErrorKind::UnitLimit,
                    )
                })?;
            let contexts = super::super::line_context::selected_contexts_counted(
                lines.paragraphs(),
                source_prior,
                limits.base().get().max_fragments - 1,
                lines.fingerprint(),
                &mut capture_records,
            )?;
            let records = contexts.record_charge().checked_add(1).ok_or_else(|| {
                error(
                    NodeId::new(0),
                    ProductionInlinePreparationErrorKind::UnitLimit,
                )
            })?;
            capture_records = records;
            Ok(BookV2BodyLineVariantSeed {
                policy,
                flow,
                admitted,
                bindings,
                limits,
                japanese_mode,
                body,
                native,
                measured_frames,
                source_widths,
                contexts,
                rebuild_records,
                captured_records: records - source_prior,
                records,
                work: stable.candidate_steps() + capture_work,
                passes: u16::try_from(stable.passes().len())
                    .map_err(|_| BreakError::IterationLimit)?,
                source_records: stable.source_record_charge(),
                retained_records: stable.retained_record_charge(),
            })
        },
    );
    let observed_records = line_budget
        .source_record_charge()
        .checked_add(line_budget.record_charge())
        .ok_or_else(|| {
            error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::UnitLimit,
            )
        })?;
    allowance.records = allowance.records.max(capture_records).max(observed_records);
    allowance.work += line_budget.candidate_steps() + capture_work;
    allowance.passes += line_budget.reshape_passes();
    result?
}

pub struct BookV2RebuiltBodyLineVariant<'s, 'p, 'a> {
    lines: &'s BookV2InlineLineLayout<'p, 'a>,
    footnotes: BookV2FootnoteLines<'s, 'p, 'a>,
    records: u64,
    work: u64,
}
impl<'s, 'p, 'a> BookV2RebuiltBodyLineVariant<'s, 'p, 'a> {
    pub fn lines(&self) -> &'s BookV2InlineLineLayout<'p, 'a> {
        self.lines
    }
    pub fn footnotes(&self) -> &BookV2FootnoteLines<'s, 'p, 'a> {
        &self.footnotes
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
}

/// Rebuild an independently owned shape/frame/line graph from the seed's exact
/// immutable inputs, and verify the observed stable fingerprint before exposing
/// it. Prepay the previously observed graph bound and input views before shaping.
pub fn with_rebuilt_book_v2_body_line_variant<R>(
    seed: &BookV2BodyLineVariantSeed<'_>,
    maximum_work: u64,
    prior_records: u64,
    use_variant: impl FnOnce(BookV2RebuiltBodyLineVariant<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    let mut allowance = BookV2LineVariantBudget::new(maximum_work, 0);
    with_budgeted_rebuilt_book_v2_body_line_variant(
        seed,
        &mut allowance,
        prior_records,
        use_variant,
    )
}

/// Replay on the caller's remaining work; retain accepted work on every return.
/// Replaying a stable context does not start a reshape feedback pass.
pub fn with_budgeted_rebuilt_book_v2_body_line_variant<R>(
    seed: &BookV2BodyLineVariantSeed<'_>,
    allowance: &mut BookV2LineVariantBudget,
    prior_records: u64,
    use_variant: impl FnOnce(BookV2RebuiltBodyLineVariant<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    allowance.records = allowance
        .records
        .max(prior_records)
        .max(seed.record_charge());
    let maximum_work = allowance.remaining_work();
    let mut work = 0;
    let result = (|| {
        let root = NodeId::new(0);
        let mut records = allowance
            .records
            .checked_add(seed.rebuild_records)
            .and_then(|n| n.checked_add(seed.contexts.paragraphs().len() as u64))
            .and_then(|n| n.checked_add(1))
            .filter(|n| *n <= seed.limits.base().get().max_fragments)
            .ok_or_else(|| error(root, ProductionInlinePreparationErrorKind::UnitLimit))?;
        allowance.records = allowance.records.max(records);
        take_work(
            &mut work,
            seed.contexts.paragraphs().len() as u64,
            maximum_work,
        )?;
        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(seed.contexts.paragraphs().len())
            .map_err(|_| BreakError::AllocationFailure)?;
        inputs.extend(
            seed.contexts
                .paragraphs()
                .iter()
                .map(|p| ProductionParagraphLineContext {
                    owner: p.owner(),
                    ends: p.ends(),
                }),
        );
        let mut source_budget = BookV2SourceVerificationBudget::new_with_prepaid_records(
            allowance.records,
            seed.limits.base().get().max_fragments,
            seed.rebuild_records,
        );
        let mut local_records = 0;
        let shape = shape_book_v2_authored_text_with_source_budget_counted(
            seed.policy,
            seed.flow,
            seed.admitted,
            seed.limits,
            seed.bindings.epoch(),
            Some(&inputs),
            &mut source_budget,
            &mut local_records,
        );
        allowance.records = source_budget.record_charge();
        let shape = shape?;
        let prepared = prepare_book_v2_inline_items_with_source_budget_counted(
            seed.flow,
            &shape,
            seed.admitted,
            seed.bindings,
            seed.limits,
            seed.japanese_mode,
            seed.native,
            &mut source_budget,
            &mut local_records,
        );
        allowance.records = source_budget.record_charge();
        let prepared = prepared?;
        records = allowance.records;
        let mut consumed = 0;
        let selected = super::frames::layout_body_lines_in_measured_frames(
            &prepared,
            seed.body,
            maximum_work - work,
            seed.measured_frames,
            seed.source_widths,
            &mut consumed,
            &mut 0,
        );
        take_work(&mut work, consumed, maximum_work)?;
        let selected = selected?;
        take_work(&mut work, 1, maximum_work)?;
        if selected.fingerprint() != seed.fingerprint() {
            return Err(error(root, ProductionInlinePreparationErrorKind::ReceiptMismatch).into());
        }
        let footnotes = prepare_book_v2_footnote_lines(&selected, seed.limits)?;
        if footnotes.record_charge() > seed.rebuild_records {
            return Err(error(root, ProductionInlinePreparationErrorKind::ReceiptMismatch).into());
        }
        Ok(use_variant(BookV2RebuiltBodyLineVariant {
            lines: &selected,
            footnotes,
            records,
            work,
        }))
    })();
    allowance.work += work;
    result
}

fn take_work(work: &mut u64, amount: u64, maximum: u64) -> Result<(), ProductionBodyReshapeError> {
    *work = work
        .checked_add(amount)
        .filter(|n| *n <= maximum)
        .ok_or_else(|| {
            error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::Atomic(
                    typaxis_linebreak::AtomicVectorInlineError::CandidateLimit,
                ),
            )
        })?;
    Ok(())
}

#[path = "book_v2_line_variant_set.rs"]
mod variant_set;
pub use variant_set::{
    with_budgeted_rebuilt_book_v2_body_line_variants, with_rebuilt_book_v2_body_line_variants,
    BookV2ObservedRebuiltColumnLineVariant, BookV2ObservedRebuiltColumnLineVariants,
    BookV2RebuiltBodyLineVariants,
};
#[path = "book_v2_column_line_variants.rs"]
mod column_variants;
pub use column_variants::*;

/// Read-only phase names for the concrete observed variant owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ObservedVariantPhase {
    ProtocolStorage,
    SourceVerify,
    Shape,
    Inline,
    Frames,
    TableWidth,
    TableOccurrence,
    SourceWidth,
    Selected,
    Footnotes,
    ContextCapture,
    Callback,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ObservedVariantOutcome {
    Success,
    AccountingRefusal,
    KernelRefusal,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2ObservedVariantObservation {
    phase: BookV2ObservedVariantPhase,
    occurrence_ordinal: Option<usize>,
    owner: NodeId,
    before_rsw: [u64; 3],
    after_rsw: [u64; 3],
    own_q_before: u64,
    own_q_after: u64,
    stored_limit: Option<u64>,
    effective_limit: Option<u64>,
    actual_local_prefix: u64,
    entered_allocating_kernel: bool,
    outcome: BookV2ObservedVariantOutcome,
}
impl BookV2ObservedVariantObservation {
    pub fn phase(&self) -> BookV2ObservedVariantPhase {
        self.phase
    }
    pub fn occurrence_ordinal(&self) -> Option<usize> {
        self.occurrence_ordinal
    }
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn before_rsw(&self) -> [u64; 3] {
        self.before_rsw
    }
    pub fn after_rsw(&self) -> [u64; 3] {
        self.after_rsw
    }
    pub fn own_q_before(&self) -> u64 {
        self.own_q_before
    }
    pub fn own_q_after(&self) -> u64 {
        self.own_q_after
    }
    pub fn stored_limit(&self) -> Option<u64> {
        self.stored_limit
    }
    pub fn effective_limit(&self) -> Option<u64> {
        self.effective_limit
    }
    pub fn actual_local_prefix(&self) -> u64 {
        self.actual_local_prefix
    }
    /// True confirms entry from the private inline hook, an actual returned
    /// value/prefix, or a uniquely output-stage typed Shape refusal. False is
    /// unconfirmed and must not be used as proof that no kernel was entered.
    pub fn entered_allocating_kernel(&self) -> bool {
        self.entered_allocating_kernel
    }
    pub fn outcome(&self) -> BookV2ObservedVariantOutcome {
        self.outcome
    }
}

/// Local work/pass observations and two fixed, nonauthoritative phase samples.
/// The actual inclusive R/S/W remain only in the original caller loan.
pub struct BookV2ObservedLineVariantBudget {
    pub(super) maximum_work: u64,
    pub(super) work: u64,
    pub(super) maximum_passes: u16,
    pub(super) passes: u16,
    pub(super) records: u64,
    pub(super) staged_records: u64,
    pub(super) replay_records: u64,
    pub(super) context_records: u64,
    last_completed: Option<BookV2ObservedVariantObservation>,
    last_attempted: Option<BookV2ObservedVariantObservation>,
}
impl BookV2ObservedLineVariantBudget {
    pub fn new(maximum_work: u64, maximum_passes: u16) -> Self {
        Self {
            maximum_work,
            work: 0,
            maximum_passes,
            passes: 0,
            records: 0,
            staged_records: 0,
            replay_records: 0,
            context_records: 0,
            last_completed: None,
            last_attempted: None,
        }
    }
    pub fn remaining_work(&self) -> u64 {
        self.maximum_work.saturating_sub(self.work)
    }
    pub fn remaining_passes(&self) -> u16 {
        self.maximum_passes.saturating_sub(self.passes)
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
    pub fn reshape_passes(&self) -> u16 {
        self.passes
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn staged_record_highwater(&self) -> u64 {
        self.staged_records
    }
    pub fn replay_record_payments(&self) -> u64 {
        self.replay_records
    }
    pub fn captured_context_records(&self) -> u64 {
        self.context_records
    }
    pub fn last_completed(&self) -> Option<BookV2ObservedVariantObservation> {
        self.last_completed
    }
    pub fn last_attempted(&self) -> Option<BookV2ObservedVariantObservation> {
        self.last_attempted
    }
    pub(super) fn begin(
        &mut self,
        phase: BookV2ObservedVariantPhase,
        ordinal: Option<usize>,
        accounting: &BookV2ObservedSourceAccounting<'_, '_>,
        q: u64,
        stored: Option<u64>,
        effective: Option<u64>,
    ) {
        let rsw = accounting.counters();
        self.last_attempted = Some(BookV2ObservedVariantObservation {
            phase,
            occurrence_ordinal: ordinal,
            owner: NodeId::new(0),
            before_rsw: rsw,
            after_rsw: rsw,
            own_q_before: q,
            own_q_after: q,
            stored_limit: stored,
            effective_limit: effective,
            actual_local_prefix: 0,
            entered_allocating_kernel: false,
            outcome: BookV2ObservedVariantOutcome::AccountingRefusal,
        });
    }
    pub(super) fn complete(
        &mut self,
        accounting: &BookV2ObservedSourceAccounting<'_, '_>,
        q: u64,
        prefix: u64,
        entered: bool,
        success: bool,
        accounting_refusal: bool,
    ) {
        self.records = accounting.counters()[0];
        if let Some(mut row) = self.last_attempted {
            row.after_rsw = accounting.counters();
            row.own_q_after = q;
            row.actual_local_prefix = prefix;
            row.entered_allocating_kernel = entered;
            row.outcome = if success {
                BookV2ObservedVariantOutcome::Success
            } else if accounting_refusal {
                BookV2ObservedVariantOutcome::AccountingRefusal
            } else {
                BookV2ObservedVariantOutcome::KernelRefusal
            };
            self.last_attempted = Some(row);
            if success {
                self.last_completed = Some(row);
            }
        }
    }
    pub(super) fn request(
        &mut self,
        accounting: &mut BookV2ObservedSourceAccounting<'_, '_>,
        delta: [u64; 3],
    ) -> Result<(), BookV2ObservedBodyReshapeError> {
        let result = if delta[2] > self.remaining_work() {
            Err(BookV2ObservedAccountingError::Work {
                owner: NodeId::new(0),
            })
        } else {
            accounting.debit(NodeId::new(0), delta)
        };
        self.records = accounting.counters()[0];
        if let Some(mut row) = self.last_attempted {
            row.after_rsw = accounting.counters();
            if result.is_err() {
                row.outcome = BookV2ObservedVariantOutcome::AccountingRefusal;
            }
            self.last_attempted = Some(row);
        }
        result?;
        self.work += delta[2];
        Ok(())
    }
    pub(super) fn absorb_candidates(&mut self, local: &BookV2ObservedBodyLineBudget) {
        self.work += local.candidate_steps();
        self.passes += local.reshape_passes();
        self.staged_records = self.staged_records.max(local.record_charge());
    }
}

pub(super) struct ObservedVariantSeed<'id, 'a> {
    policy: &'a BookV2ResourcePolicy<'a>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &'a M4EffectiveResourceLimits,
    mode: JapaneseLineBreakMode,
    columns: &'a typaxis_syntax::book_v2::BookV2ColumnFramePlan<'a>,
    native: Option<&'a BookV2ObservedNativeMath<'id, 'a>>,
    widths: Option<&'a BookV2SourceWidthAssignments<'a, 'a>>,
    contexts: ProductionSelectedLineContexts,
    rebuild_records: u64,
    context_records: u64,
    prepared_rsw: [u64; 3],
}

fn observed_variant_mismatch() -> BookV2ObservedBodyReshapeError {
    error(
        NodeId::new(0),
        ProductionInlinePreparationErrorKind::ReceiptMismatch,
    )
    .into()
}
fn observed_units(n: usize) -> Result<u64, BookV2ObservedBodyReshapeError> {
    u64::try_from(n).map_err(|_| {
        BookV2ObservedAccountingError::Records {
            owner: NodeId::new(0),
        }
        .into()
    })
}
fn observed_bytes<T>(n: usize) -> Result<u64, BookV2ObservedBodyReshapeError> {
    observed_units(n)?
        .checked_mul(std::mem::size_of::<T>() as u64)
        .ok_or_else(|| {
            BookV2ObservedAccountingError::Spool {
                owner: NodeId::new(0),
            }
            .into()
        })
}

#[allow(clippy::too_many_arguments)]
fn prepare_observed_variant_seed<'id, 'a>(
    policy: &'a BookV2ResourcePolicy<'a>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &'a M4EffectiveResourceLimits,
    mode: JapaneseLineBreakMode,
    columns: &'a typaxis_syntax::book_v2::BookV2ColumnFramePlan<'a>,
    native: Option<&'a BookV2ObservedNativeMath<'id, 'a>>,
    widths: Option<&'a BookV2SourceWidthAssignments<'a, 'a>>,
    budget: &mut BookV2ObservedLineVariantBudget,
    accounting: &mut BookV2ObservedSourceAccounting<'id, '_>,
) -> Result<ObservedVariantSeed<'id, 'a>, BookV2ObservedBodyReshapeError> {
    columns
        .verify(flow.body())
        .map_err(|_| observed_variant_mismatch())?;
    policy
        .verify_for(policy.body(), limits)
        .map_err(|_| observed_variant_mismatch())?;
    bindings
        .verify(bindings.body(), admitted, limits)
        .map_err(|_| observed_variant_mismatch())?;
    accounting.require_limit_domain(limits.base())?;
    accounting.require_limit_domain(flow.body().body().limits())?;
    if widths.is_some_and(|w| !w.matches_flow(flow)) {
        return Err(observed_variant_mismatch());
    }
    if let Some(n) = native {
        n.math().verify(bindings, admitted, limits).map_err(|e| {
            error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::NativeMath(e),
            )
        })?;
    }
    super::feedback::with_observed_variant_capture_lines(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        mode,
        columns.measurement_body(),
        native,
        budget,
        accounting,
        Some(MeasurementPageFrames::Columns(columns)),
        widths,
        |stable, accounting, budget| -> Result<_, BookV2ObservedBodyReshapeError> {
            let lines = stable.lines();
            budget.begin(
                BookV2ObservedVariantPhase::ContextCapture,
                None,
                accounting,
                0,
                None,
                None,
            );
            // These are a defined conservative logical carrier bound, including
            // future views, and the separate actual requested owned backing.
            let paragraphs = observed_units(lines.paragraphs().len())?;
            budget.request(
                accounting,
                [
                    0,
                    0,
                    paragraphs
                        .checked_add(1)
                        .ok_or(BreakError::IterationLimit)?,
                ],
            )?;
            let mut records = paragraphs
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .ok_or(BookV2ObservedAccountingError::Records {
                    owner: NodeId::new(0),
                })?;
            let mut bytes =
                observed_bytes::<ProductionSelectedParagraphContext>(lines.paragraphs().len())?;
            let mut work = paragraphs
                .checked_mul(3)
                .and_then(|n| n.checked_add(1))
                .ok_or(BreakError::IterationLimit)?;
            for p in lines.paragraphs() {
                let count = observed_units(p.lines().len())?;
                budget.request(accounting, [0, 0, count])?;
                records = records
                    .checked_add(count)
                    .ok_or(BookV2ObservedAccountingError::Records { owner: p.owner() })?;
                bytes = bytes
                    .checked_add(observed_bytes::<u32>(p.lines().len())?)
                    .ok_or(BookV2ObservedAccountingError::Spool { owner: p.owner() })?;
                for line in p.lines() {
                    work = work
                        .checked_add(
                            observed_units(line.items().len())?
                                .checked_add(2)
                                .ok_or(BreakError::IterationLimit)?,
                        )
                        .ok_or(BreakError::IterationLimit)?;
                }
            }
            budget.request(accounting, [records, bytes, work])?;
            budget.context_records = budget.context_records.checked_add(records).ok_or(
                BookV2ObservedAccountingError::Records {
                    owner: NodeId::new(0),
                },
            )?;
            let mut accepted = 0;
            let contexts = super::super::line_context::selected_contexts_counted(
                lines.paragraphs(),
                0,
                records - 1,
                lines.fingerprint(),
                &mut accepted,
            );
            budget.complete(accounting, 0, accepted, true, contexts.is_ok(), false);
            let contexts = contexts?;
            Ok(ObservedVariantSeed {
                policy,
                flow,
                admitted,
                bindings,
                limits,
                mode,
                columns,
                native,
                widths,
                contexts,
                rebuild_records: stable.footnotes().record_charge(),
                context_records: records,
                prepared_rsw: accounting.counters(),
            })
        },
    )?
}

/// Finite new-P2 protocol reservations. No source or accounting authority lives here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ObservedProtocolReserveSite {
    ColumnViews,
    ReplayBounds,
    ReplaySlots,
    ReplayInputsOuter,
    ReplayInputViews,
    ReplayShapes,
    ReplayInlines,
    ReplaySelected,
    ReplayVariants,
    StagedInputViews,
}

#[cfg(test)]
std::thread_local! {
    static OBSERVED_PROTOCOL_RESERVE_FAULT: std::cell::Cell<Option<ObservedProtocolReserveSite>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(super) fn observed_protocol_fault_once(site: ObservedProtocolReserveSite) {
    OBSERVED_PROTOCOL_RESERVE_FAULT.with(|fault| {
        assert!(
            fault.replace(Some(site)).is_none(),
            "unconsumed protocol fault"
        );
    });
}

pub(super) fn reserve_observed_protocol<T>(
    storage: &mut Vec<T>,
    count: usize,
    site: ObservedProtocolReserveSite,
) -> Result<(), BreakError> {
    #[cfg(test)]
    if OBSERVED_PROTOCOL_RESERVE_FAULT.with(|fault| {
        if fault.get() == Some(site) {
            fault.set(None);
            true
        } else {
            false
        }
    }) {
        return Err(BreakError::AllocationFailure);
    }
    #[cfg(not(test))]
    let _ = site;
    storage
        .try_reserve_exact(count)
        .map_err(|_| BreakError::AllocationFailure)
}

#[cfg(test)]
mod observed_p2_reservation_controls {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;

    #[test]
    fn observed_p2_protocol_reservation_fault_preserves_accepted_prefix() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let sites = [
            ObservedProtocolReserveSite::ColumnViews,
            ObservedProtocolReserveSite::ReplayBounds,
            ObservedProtocolReserveSite::ReplaySlots,
            ObservedProtocolReserveSite::ReplayInputsOuter,
            ObservedProtocolReserveSite::ReplayInputViews,
            ObservedProtocolReserveSite::ReplayShapes,
            ObservedProtocolReserveSite::ReplayInlines,
            ObservedProtocolReserveSite::ReplaySelected,
            ObservedProtocolReserveSite::ReplayVariants,
            ObservedProtocolReserveSite::StagedInputViews,
        ];
        for site in sites {
            let (mut r, mut s, mut w) = (3, 5, 7);
            with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                100,
                1000,
                |a| {
                    let mut budget = BookV2ObservedLineVariantBudget::new(1000, 16);
                    budget.begin(
                        BookV2ObservedVariantPhase::ProtocolStorage,
                        None,
                        a,
                        0,
                        None,
                        None,
                    );
                    let bytes = observed_bytes::<u64>(3).unwrap();
                    budget.request(a, [2, bytes, 3]).unwrap();
                    let accepted = a.counters();
                    let diagnostic = budget.last_attempted().unwrap();
                    assert_eq!(accepted, [5, 5 + bytes, 10]);
                    observed_protocol_fault_once(site);
                    let mut storage = Vec::<u64>::new();
                    assert!(matches!(
                        reserve_observed_protocol(&mut storage, 3, site),
                        Err(BreakError::AllocationFailure)
                    ));
                    assert_eq!((storage.len(), storage.capacity()), (0, 0));
                    assert_eq!(a.counters(), accepted);
                    assert_eq!(budget.work_steps(), 3);
                    assert_eq!(budget.record_charge(), accepted[0]);
                    assert_eq!(
                        budget.last_attempted().unwrap().after_rsw(),
                        diagnostic.after_rsw()
                    );
                    // The one-shot fault is consumed; retry uses the real allocator.
                    reserve_observed_protocol(&mut storage, 3, site).unwrap();
                    assert!(storage.capacity() >= 3);
                    assert_eq!(a.counters(), accepted);
                    eprintln!("P2 owning protocol fault {site:?}: retained={accepted:?}");
                },
            );
        }
        let (mut r, mut s, mut w) = (3, 5, 7);
        with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 100, 1000, |a| {
            let mut budget = BookV2ObservedLineVariantBudget::new(0, 16);
            budget.begin(
                BookV2ObservedVariantPhase::ProtocolStorage,
                None,
                a,
                0,
                None,
                None,
            );
            let site = ObservedProtocolReserveSite::ReplaySlots;
            observed_protocol_fault_once(site);
            let mut storage = Vec::<u64>::new();
            let result = (|| -> Result<(), BookV2ObservedBodyReshapeError> {
                budget.request(a, [2, 24, 1])?;
                reserve_observed_protocol(&mut storage, 3, site)?;
                Ok(())
            })();
            assert!(matches!(
                result,
                Err(BookV2ObservedBodyReshapeError::Accounting(
                    BookV2ObservedAccountingError::Work { .. }
                ))
            ));
            assert_eq!(a.counters(), [3, 5, 7]);
            assert_eq!((storage.len(), storage.capacity()), (0, 0));
            // Debit refusal returned before even the private fault backend.
            OBSERVED_PROTOCOL_RESERVE_FAULT
                .with(|fault| assert_eq!(fault.replace(None), Some(site)));
        });
    }
}
