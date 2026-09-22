//! Retain converged source contexts so independent physical variants can coexist.
//! These seeds and rebuilds are not page selection or repeated-paint authorization.
use super::*;
use typaxis_core::Rect;
use typaxis_linebreak::BreakError;
use typaxis_shaping::{book_v2::shape_book_v2_authored_text, ProductionParagraphLineContext};
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
    page_plan: Option<&'a BookV2PageFramePlan<'a>>,
    source_widths: Option<&'a BookV2SourceWidthAssignments<'a, 'a>>,
    contexts: ProductionSelectedLineContexts,
    rebuild_records: u64,
    captured_records: u64,
    records: u64,
    work: u64,
    passes: u16,
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
        prepare_book_v2_body_line_variant_seed(
            self.policy,
            self.flow,
            self.admitted,
            self.bindings,
            self.limits,
            self.japanese_mode,
            self.body,
            maximum_work,
            prior_records,
            self.native,
            remaining_passes,
            self.page_plan,
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
        prepare_budgeted_book_v2_body_line_variant_seed(
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
            self.page_plan,
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
    allowance.records = allowance.records.max(prior_records);
    if prior_records >= limits.base().get().max_fragments {
        return Err(error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::UnitLimit,
        )
        .into());
    }
    let maximum_work = allowance.remaining_work();
    let mut line_budget = BookV2BodyLineBudget::new(maximum_work, allowance.remaining_passes());
    let mut capture_work = 0;
    let mut capture_records = prior_records;
    let result = with_budgeted_book_v2_body_lines_with_source_widths(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        native,
        &mut line_budget,
        page_plan,
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
            let contexts = super::super::line_context::selected_contexts_counted(
                lines.paragraphs(),
                prior_records.max(rebuild_records),
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
                page_plan,
                source_widths,
                contexts,
                rebuild_records,
                captured_records: records - prior_records.max(rebuild_records),
                records,
                work: stable.candidate_steps() + capture_work,
                passes: u16::try_from(stable.passes().len())
                    .map_err(|_| BreakError::IterationLimit)?,
            })
        },
    );
    allowance.records = allowance
        .records
        .max(capture_records)
        .max(line_budget.record_charge());
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
        let records = prior_records
            .max(seed.records)
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
        let shape = shape_book_v2_authored_text(
            seed.policy,
            seed.flow,
            seed.admitted,
            seed.limits,
            seed.bindings.epoch(),
            Some(&inputs),
        )?;
        let prepared = prepare_book_v2_inline_items_with_native_context(
            seed.flow,
            &shape,
            seed.admitted,
            seed.bindings,
            seed.limits,
            seed.japanese_mode,
            seed.native,
        )?;
        let mut consumed = 0;
        let selected = super::frames::layout_body_lines_counted(
            &prepared,
            seed.body,
            maximum_work - work,
            seed.page_plan,
            seed.source_widths,
            &mut consumed,
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
    BookV2RebuiltBodyLineVariants,
};
