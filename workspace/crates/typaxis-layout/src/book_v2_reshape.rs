//! Actual book-2 shaping/selection feedback; this is not page convergence.
use super::frames::MeasurementPageFrames;
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

#[path = "book_v2_column_lines.rs"]
mod column_lines;
pub use column_lines::*;

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
    with_budgeted_lines_in_measured_frames(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        native,
        allowance,
        page_plan.map(MeasurementPageFrames::Pages),
        source_widths,
        use_stable,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn with_budgeted_lines_in_measured_frames<'a, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    native: Option<&BookV2NativeMath<'_>>,
    allowance: &mut BookV2BodyLineBudget,
    page_plan: Option<MeasurementPageFrames<'_, 'a>>,
    source_widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    use_stable: impl FnOnce(BookV2ConvergedBodyLines<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    with_lines_accounting_kernel(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        body,
        native,
        &mut LegacyLineAccounting(allowance),
        page_plan,
        source_widths,
        |stable, _| use_stable(stable),
    )
}

#[allow(clippy::too_many_arguments)]
fn with_lines_accounting_kernel<'a, R, A: LineAccounting>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    body: Rect,
    native: Option<&BookV2NativeMath<'_>>,
    allowance: &mut A,
    page_plan: Option<MeasurementPageFrames<'_, 'a>>,
    source_widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    use_stable: impl FnOnce(BookV2ConvergedBodyLines<'_, '_, '_>, &mut A) -> R,
) -> Result<R, A::Error> {
    let remaining_passes = allowance
        .remaining_passes()
        .min(limits.base().get().max_line_reshape_passes);
    if remaining_passes == 0 {
        return Err(BreakError::IterationLimit.into());
    }
    let epoch = bindings.epoch();
    let initial_steps = allowance.remaining_steps();
    let (mut contexts, initial_state) = {
        let shape = allowance.shape(policy, flow, admitted, limits, epoch, None)?;
        let prepared = allowance.inlines(
            flow,
            &shape,
            admitted,
            bindings,
            limits,
            japanese_mode,
            native,
        )?;
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
        let inputs = allowance.context_inputs(&contexts)?;
        let permit = feedback.begin_pass(&mut budget)?;
        allowance.begin_pass();
        let shape = allowance.shape(policy, flow, admitted, limits, epoch, Some(&inputs))?;
        let prepared = allowance.inlines(
            flow,
            &shape,
            admitted,
            bindings,
            limits,
            japanese_mode,
            native,
        )?;
        let selected = allowance.measure(&prepared, body, page_plan, source_widths)?;
        match permit.complete(super::super::reshape::selected_fingerprint_state(
            selected.fingerprint(),
        )?)? {
            LineReshapeObservation::Stable => {
                let mut records = 0;
                let footnotes = allowance.footnotes(&selected, limits, &mut records);
                let retained = allowance.retain(records);
                let footnotes = footnotes?;
                retained?;
                let source_records = allowance.source_records();
                let retained_records = allowance.retained_records();
                let candidate_steps = initial_steps - allowance.remaining_steps();
                allowance.before_callback();
                return Ok(use_stable(
                    BookV2ConvergedBodyLines {
                        lines: &selected,
                        footnotes,
                        passes: feedback.records(),
                        candidate_steps,
                        source_records,
                        retained_records,
                    },
                    allowance,
                ));
            }
            LineReshapeObservation::RebreakRequired => {
                contexts = allowance.capture_contexts(&selected)?
            }
        }
    }
}

#[derive(Debug)]
pub enum BookV2ObservedBodyReshapeError {
    Shape(BookV2ObservedShapeError),
    Inline(BookV2ObservedInlineError),
    Layout(ProductionInlinePreparationError),
    Accounting(BookV2ObservedAccountingError),
    Feedback(BreakError),
}
impl From<BookV2ObservedShapeError> for BookV2ObservedBodyReshapeError {
    fn from(e: BookV2ObservedShapeError) -> Self {
        Self::Shape(e)
    }
}
impl From<BookV2ObservedInlineError> for BookV2ObservedBodyReshapeError {
    fn from(e: BookV2ObservedInlineError) -> Self {
        Self::Inline(e)
    }
}
impl From<ProductionInlinePreparationError> for BookV2ObservedBodyReshapeError {
    fn from(e: ProductionInlinePreparationError) -> Self {
        Self::Layout(e)
    }
}
impl From<BookV2ObservedAccountingError> for BookV2ObservedBodyReshapeError {
    fn from(e: BookV2ObservedAccountingError) -> Self {
        Self::Accounting(e)
    }
}
impl From<BreakError> for BookV2ObservedBodyReshapeError {
    fn from(e: BreakError) -> Self {
        Self::Feedback(e)
    }
}
impl std::fmt::Display for BookV2ObservedBodyReshapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 observed lines: {self:?}")
    }
}
impl std::error::Error for BookV2ObservedBodyReshapeError {}

/// Local candidate/output/pass observations. Source R/S/W remain in the
/// separately loaned actual caller cells; native/P/header replay are excluded.
#[derive(Debug)]
pub struct BookV2ObservedBodyLineBudget {
    maximum_steps: u64,
    remaining_steps: u64,
    maximum_passes: u16,
    remaining_passes: u16,
    records: u64,
}
impl BookV2ObservedBodyLineBudget {
    pub fn new(maximum_steps: u64, maximum_passes: u16) -> Self {
        Self {
            maximum_steps,
            remaining_steps: maximum_steps,
            maximum_passes,
            remaining_passes: maximum_passes,
            records: 0,
        }
    }
    pub fn candidate_steps(&self) -> u64 {
        self.maximum_steps - self.remaining_steps
    }
    pub fn reshape_passes(&self) -> u16 {
        self.maximum_passes - self.remaining_passes
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn remaining_passes(&self) -> u16 {
        self.remaining_passes
    }
}
trait LineAccounting {
    type Error: From<BreakError> + From<ProductionInlinePreparationError>;
    fn remaining_steps(&self) -> u64;
    fn remaining_passes(&self) -> u16;
    fn retained_records(&self) -> u64;
    fn begin_pass(&mut self);
    fn shape<'a>(
        &mut self,
        policy: &BookV2ResourcePolicy<'_>,
        flow: &'a PreparedBookV2TextFlow<'a>,
        admitted: &'a AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
        epoch: [u8; 32],
        lines: Option<&[ProductionParagraphLineContext<'_>]>,
    ) -> Result<typaxis_shaping::book_v2::BookV2AuthoredTextShape<'a>, Self::Error>;
    fn inlines<'a>(
        &mut self,
        flow: &'a PreparedBookV2TextFlow<'a>,
        shape: &'a typaxis_shaping::book_v2::BookV2AuthoredTextShape<'a>,
        admitted: &AdmittedProductionResourceLedgerV3,
        bindings: &'a BookV2VectorBindings<'a>,
        limits: &M4EffectiveResourceLimits,
        mode: JapaneseLineBreakMode,
        native: Option<&'a BookV2NativeMath<'a>>,
    ) -> Result<BookV2PreparedInlines<'a>, Self::Error>;
    fn measure<'p, 'a>(
        &mut self,
        prepared: &'p BookV2PreparedInlines<'a>,
        body: Rect,
        frames: Option<MeasurementPageFrames<'p, 'a>>,
        widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    ) -> Result<BookV2InlineLineLayout<'p, 'a>, Self::Error>;
    fn capture_contexts(
        &mut self,
        lines: &BookV2InlineLineLayout<'_, '_>,
    ) -> Result<ProductionSelectedLineContexts, Self::Error>;
    fn context_inputs<'a>(
        &mut self,
        contexts: &'a ProductionSelectedLineContexts,
    ) -> Result<Vec<ProductionParagraphLineContext<'a>>, Self::Error> {
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
        Ok(inputs)
    }
    fn footnotes<'s, 'p, 'a>(
        &mut self,
        lines: &'s BookV2InlineLineLayout<'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        records: &mut u64,
    ) -> Result<BookV2FootnoteLines<'s, 'p, 'a>, Self::Error> {
        Ok(prepare_book_v2_footnote_lines_counted(
            lines, limits, records,
        )?)
    }
    fn retain(&mut self, records: u64) -> Result<(), Self::Error>;
    fn source_records(&self) -> u64;
    fn before_callback(&mut self);
}
struct LegacyLineAccounting<'loan>(&'loan mut BookV2BodyLineBudget);
impl LineAccounting for LegacyLineAccounting<'_> {
    type Error = ProductionBodyReshapeError;
    fn remaining_steps(&self) -> u64 {
        self.0.remaining_steps
    }
    fn remaining_passes(&self) -> u16 {
        self.0.remaining_passes
    }
    fn retained_records(&self) -> u64 {
        self.0.records
    }
    fn begin_pass(&mut self) {
        self.0.remaining_passes -= 1;
    }
    fn shape<'a>(
        &mut self,
        p: &BookV2ResourcePolicy<'_>,
        f: &'a PreparedBookV2TextFlow<'a>,
        a: &'a AdmittedProductionResourceLedgerV3,
        l: &M4EffectiveResourceLimits,
        e: [u8; 32],
        c: Option<&[ProductionParagraphLineContext<'_>]>,
    ) -> Result<typaxis_shaping::book_v2::BookV2AuthoredTextShape<'a>, Self::Error> {
        let mut records = 0;
        let r = shape_book_v2_authored_text_with_source_budget_counted(
            p,
            f,
            a,
            l,
            e,
            c,
            &mut self.0.source_records,
            &mut records,
        );
        self.0.records = self.0.records.max(records);
        Ok(r?)
    }
    fn inlines<'a>(
        &mut self,
        f: &'a PreparedBookV2TextFlow<'a>,
        s: &'a typaxis_shaping::book_v2::BookV2AuthoredTextShape<'a>,
        a: &AdmittedProductionResourceLedgerV3,
        b: &'a BookV2VectorBindings<'a>,
        l: &M4EffectiveResourceLimits,
        m: JapaneseLineBreakMode,
        n: Option<&'a BookV2NativeMath<'a>>,
    ) -> Result<BookV2PreparedInlines<'a>, Self::Error> {
        let mut records = 0;
        let r = prepare_book_v2_inline_items_with_source_budget_counted(
            f,
            s,
            a,
            b,
            l,
            m,
            n,
            &mut self.0.source_records,
            &mut records,
        );
        self.0.records = self.0.records.max(records);
        Ok(r?)
    }
    fn measure<'p, 'a>(
        &mut self,
        p: &'p BookV2PreparedInlines<'a>,
        b: Rect,
        f: Option<MeasurementPageFrames<'p, 'a>>,
        w: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    ) -> Result<BookV2InlineLineLayout<'p, 'a>, Self::Error> {
        Ok(self.0.measure(p, b, f, w)?)
    }
    fn capture_contexts(
        &mut self,
        l: &BookV2InlineLineLayout<'_, '_>,
    ) -> Result<ProductionSelectedLineContexts, Self::Error> {
        Ok(self.0.capture_contexts(l)?)
    }
    fn retain(&mut self, r: u64) -> Result<(), Self::Error> {
        self.0.records = self.0.records.max(r);
        self.0
            .source_records
            .include_retained_records(self.0.records);
        Ok(())
    }
    fn source_records(&self) -> u64 {
        self.0.source_record_charge()
    }
    fn before_callback(&mut self) {}
}
struct ObservedLineAccounting<'loan, 'id, 'cells, 'window> {
    local: &'loan mut BookV2ObservedBodyLineBudget,
    accounting: &'loan mut BookV2ObservedSourceAccounting<'id, 'cells>,
    intrinsic: &'loan mut BookV2ObservedIntrinsicRecords<'id, 'window>,
    paid_native: bool,
}
impl LineAccounting for ObservedLineAccounting<'_, '_, '_, '_> {
    type Error = BookV2ObservedBodyReshapeError;
    fn remaining_steps(&self) -> u64 {
        self.local.remaining_steps
    }
    fn remaining_passes(&self) -> u16 {
        self.local.remaining_passes
    }
    fn retained_records(&self) -> u64 {
        self.intrinsic.paid_records()
    }
    fn begin_pass(&mut self) {
        self.local.remaining_passes -= 1;
    }
    fn shape<'a>(
        &mut self,
        p: &BookV2ResourcePolicy<'_>,
        f: &'a PreparedBookV2TextFlow<'a>,
        a: &'a AdmittedProductionResourceLedgerV3,
        l: &M4EffectiveResourceLimits,
        e: [u8; 32],
        c: Option<&[ProductionParagraphLineContext<'_>]>,
    ) -> Result<typaxis_shaping::book_v2::BookV2AuthoredTextShape<'a>, Self::Error> {
        let mut records = 0;
        let r = typaxis_shaping::book_v2::shape_book_v2_authored_text_with_observed_source_counted(
            p,
            f,
            a,
            l,
            e,
            c,
            self.accounting,
            self.intrinsic,
            &mut records,
        );
        self.local.records = self.local.records.max(records);
        Ok(r?)
    }
    fn inlines<'a>(
        &mut self,
        f: &'a PreparedBookV2TextFlow<'a>,
        s: &'a typaxis_shaping::book_v2::BookV2AuthoredTextShape<'a>,
        a: &AdmittedProductionResourceLedgerV3,
        b: &'a BookV2VectorBindings<'a>,
        l: &M4EffectiveResourceLimits,
        m: JapaneseLineBreakMode,
        n: Option<&'a BookV2NativeMath<'a>>,
    ) -> Result<BookV2PreparedInlines<'a>, Self::Error> {
        debug_assert!(self.paid_native || n.is_none());
        let mut records = 0;
        // The bool is private to the typed entry: only a matching observed
        // Native owner can enter the line kernel with paid_native=true.
        let r = prepare_observed_inline_items_counted(
            f,
            s,
            a,
            b,
            l,
            m,
            n,
            self.paid_native,
            self.accounting,
            self.intrinsic,
            &mut records,
        );
        self.local.records = self.local.records.max(records);
        Ok(r?)
    }
    fn measure<'p, 'a>(
        &mut self,
        p: &'p BookV2PreparedInlines<'a>,
        b: Rect,
        f: Option<MeasurementPageFrames<'p, 'a>>,
        w: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    ) -> Result<BookV2InlineLineLayout<'p, 'a>, Self::Error> {
        let mut work = 0;
        let mut records = 0;
        let result = super::frames::layout_body_lines_in_measured_frames(
            p,
            b,
            self.local
                .remaining_steps
                .min(self.accounting.remaining_work()),
            f,
            w,
            &mut work,
            &mut records,
        );
        self.local.remaining_steps -= work;
        self.local.records = self.local.records.max(records);
        let spent = self.accounting.debit(NodeId::new(0), [0, 0, work]);
        let retained =
            self.accounting
                .retain_intrinsic_output(self.intrinsic, NodeId::new(0), records);
        let lines = result?;
        spent?;
        retained?;
        Ok(lines)
    }
    fn capture_contexts(
        &mut self,
        l: &BookV2InlineLineLayout<'_, '_>,
    ) -> Result<ProductionSelectedLineContexts, Self::Error> {
        let mut records = 0;
        let result = l.selected_line_contexts_counted(&mut records);
        let retained = self.retain(records);
        let contexts = result?;
        retained?;
        Ok(contexts)
    }
    fn retain(&mut self, r: u64) -> Result<(), Self::Error> {
        self.local.records = self.local.records.max(r);
        Ok(self
            .accounting
            .retain_intrinsic_output(self.intrinsic, NodeId::new(0), r)?)
    }
    fn source_records(&self) -> u64 {
        self.accounting.counters()[0] - self.intrinsic.paid_records()
    }
    fn before_callback(&mut self) {
        self.intrinsic.seal();
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn with_observed_lines_in_measured_frames<'a, 'id, 'cells, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    mode: JapaneseLineBreakMode,
    body: Rect,
    local: &mut BookV2ObservedBodyLineBudget,
    accounting: &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    frames: Option<MeasurementPageFrames<'_, 'a>>,
    widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    inspect: impl FnOnce(
        BookV2ConvergedBodyLines<'_, '_, '_>,
        &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    ) -> R,
) -> Result<R, BookV2ObservedBodyReshapeError> {
    with_observed_lines_native_kernel(
        policy, flow, admitted, bindings, limits, mode, body, None, false, local, accounting,
        frames, widths, inspect,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn with_observed_native_lines_in_measured_frames<'a, 'id, 'cells, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    mode: JapaneseLineBreakMode,
    body: Rect,
    native: Option<&BookV2ObservedNativeMath<'id, '_>>,
    local: &mut BookV2ObservedBodyLineBudget,
    accounting: &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    frames: Option<MeasurementPageFrames<'_, 'a>>,
    widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    inspect: impl FnOnce(
        BookV2ConvergedBodyLines<'_, '_, '_>,
        &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    ) -> R,
) -> Result<R, BookV2ObservedBodyReshapeError> {
    with_observed_lines_native_kernel(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        mode,
        body,
        native.map(BookV2ObservedNativeMath::math),
        true,
        local,
        accounting,
        frames,
        widths,
        inspect,
    )
}

#[allow(clippy::too_many_arguments)]
fn with_observed_lines_native_kernel<'a, 'id, 'cells, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    mode: JapaneseLineBreakMode,
    body: Rect,
    native: Option<&BookV2NativeMath<'_>>,
    paid_native: bool,
    local: &mut BookV2ObservedBodyLineBudget,
    accounting: &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    frames: Option<MeasurementPageFrames<'_, 'a>>,
    widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    inspect: impl FnOnce(
        BookV2ConvergedBodyLines<'_, '_, '_>,
        &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    ) -> R,
) -> Result<R, BookV2ObservedBodyReshapeError> {
    accounting.with_zero_intrinsic_records(|accounting, intrinsic| {
        let mut state = ObservedLineAccounting {
            local,
            accounting,
            intrinsic,
            paid_native,
        };
        with_lines_accounting_kernel(
            policy,
            flow,
            admitted,
            bindings,
            limits,
            mode,
            body,
            native,
            &mut state,
            frames,
            widths,
            |stable, state| inspect(stable, state.accounting),
        )
    })
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
        page_plan: Option<MeasurementPageFrames<'p, 'a>>,
        source_widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    ) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
        let mut consumed = 0;
        let mut records = 0;
        let result = super::frames::layout_body_lines_in_measured_frames(
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

// The staged capture owner never exposes this positive window. It is not a
// replay request, and its temporary graph is dropped before the next pass.
struct VariantLineAccounting<'loan, 'id, 'cells, 'window> {
    inner: ObservedLineAccounting<'loan, 'id, 'cells, 'window>,
    variant: &'loan mut BookV2ObservedLineVariantBudget,
    committed: bool,
}
impl VariantLineAccounting<'_, '_, '_, '_> {
    fn pay_local(
        &mut self,
        records: u64,
        spool: u64,
        work: u64,
    ) -> Result<(), BookV2ObservedBodyReshapeError> {
        if work > self.inner.local.remaining_steps {
            return Err(BookV2ObservedAccountingError::Work {
                owner: NodeId::new(0),
            }
            .into());
        }
        self.inner
            .accounting
            .debit(NodeId::new(0), [records, spool, work])?;
        self.inner.local.remaining_steps -= work;
        Ok(())
    }
    fn begin(&mut self, phase: BookV2ObservedVariantPhase, stored: Option<u64>, cap: Option<u64>) {
        self.variant.begin(
            phase,
            None,
            self.inner.accounting,
            self.inner.intrinsic.paid_records(),
            stored,
            cap,
        );
    }
    fn finish(&mut self, prefix: u64, entered: bool, ok: bool, accounting_refusal: bool) {
        self.variant.complete(
            self.inner.accounting,
            self.inner.intrinsic.paid_records(),
            prefix,
            entered,
            ok,
            accounting_refusal,
        );
    }
    fn commit(&mut self) {
        if !self.committed {
            self.variant.absorb_candidates(self.inner.local);
            self.committed = true;
        }
    }
}
impl LineAccounting for VariantLineAccounting<'_, '_, '_, '_> {
    type Error = BookV2ObservedBodyReshapeError;
    fn remaining_steps(&self) -> u64 {
        self.inner.remaining_steps()
    }
    fn remaining_passes(&self) -> u16 {
        self.inner.remaining_passes()
    }
    fn retained_records(&self) -> u64 {
        self.inner.retained_records()
    }
    fn begin_pass(&mut self) {
        self.inner.begin_pass()
    }
    fn shape<'a>(
        &mut self,
        p: &BookV2ResourcePolicy<'_>,
        f: &'a PreparedBookV2TextFlow<'a>,
        a: &'a AdmittedProductionResourceLedgerV3,
        l: &M4EffectiveResourceLimits,
        e: [u8; 32],
        c: Option<&[ProductionParagraphLineContext<'_>]>,
    ) -> Result<BookV2AuthoredTextShape<'a>, Self::Error> {
        self.begin(BookV2ObservedVariantPhase::Shape, None, None);
        let mut records = 0;
        let result =
            typaxis_shaping::book_v2::shape_book_v2_authored_text_with_observed_source_counted(
                p,
                f,
                a,
                l,
                e,
                c,
                self.inner.accounting,
                self.inner.intrinsic,
                &mut records,
            );
        let entered = super::observed_shape_kernel_entry_confirmed(&result, records);
        let result = result.map_err(BookV2ObservedBodyReshapeError::Shape);
        self.inner.local.records = self.inner.local.records.max(records);
        let accounting_refusal = matches!(
            &result,
            Err(BookV2ObservedBodyReshapeError::Shape(
                BookV2ObservedShapeError::Accounting(_)
            ))
        );
        self.finish(records, entered, result.is_ok(), accounting_refusal);
        result
    }
    fn inlines<'a>(
        &mut self,
        f: &'a PreparedBookV2TextFlow<'a>,
        s: &'a BookV2AuthoredTextShape<'a>,
        a: &AdmittedProductionResourceLedgerV3,
        b: &'a BookV2VectorBindings<'a>,
        l: &M4EffectiveResourceLimits,
        m: JapaneseLineBreakMode,
        n: Option<&'a BookV2NativeMath<'a>>,
    ) -> Result<BookV2PreparedInlines<'a>, Self::Error> {
        self.begin(BookV2ObservedVariantPhase::Inline, None, None);
        let mut records = 0;
        let mut entered = false;
        let result = prepare_observed_inline_items_with_entry_counted(
            f,
            s,
            a,
            b,
            l,
            m,
            n,
            true,
            self.inner.accounting,
            self.inner.intrinsic,
            &mut records,
            Some(&mut entered),
        )
        .map_err(BookV2ObservedBodyReshapeError::Inline);
        self.inner.local.records = self.inner.local.records.max(records);
        let accounting_refusal = matches!(
            &result,
            Err(BookV2ObservedBodyReshapeError::Inline(
                BookV2ObservedInlineError::Accounting(_)
            ))
        );
        self.finish(records, entered, result.is_ok(), accounting_refusal);
        result
    }
    fn measure<'p, 'a>(
        &mut self,
        p: &'p BookV2PreparedInlines<'a>,
        b: Rect,
        f: Option<MeasurementPageFrames<'p, 'a>>,
        w: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    ) -> Result<BookV2InlineLineLayout<'p, 'a>, Self::Error> {
        let cap = self
            .inner
            .accounting
            .intrinsic_limit(self.inner.intrinsic)?
            .min(p.output_record_limit);
        self.begin(
            BookV2ObservedVariantPhase::Frames,
            Some(p.output_record_limit),
            Some(cap),
        );
        let (mut work, mut records) = (0, 0);
        let result = super::frames::layout_body_lines_in_measured_frames_with_record_limit(
            p,
            b,
            self.inner
                .local
                .remaining_steps
                .min(self.inner.accounting.remaining_work()),
            f,
            w,
            &mut work,
            &mut records,
            cap,
            BookV2NativeRecordOrigin::AlreadyPaid,
        );
        self.inner.local.remaining_steps -= work;
        self.inner.local.records = self.inner.local.records.max(records);
        let spent = self.inner.accounting.debit(NodeId::new(0), [0, 0, work]);
        let retained = self.inner.accounting.retain_intrinsic_output(
            self.inner.intrinsic,
            NodeId::new(0),
            records,
        );
        self.finish(
            records,
            true,
            result.is_ok() && spent.is_ok() && retained.is_ok(),
            result.is_ok() && (spent.is_err() || retained.is_err()),
        );
        let value = result?;
        spent?;
        retained?;
        Ok(value)
    }
    fn capture_contexts(
        &mut self,
        l: &BookV2InlineLineLayout<'_, '_>,
    ) -> Result<ProductionSelectedLineContexts, Self::Error> {
        let cap = self
            .inner
            .accounting
            .intrinsic_limit(self.inner.intrinsic)?
            .min(l.prepared.output_record_limit);
        self.begin(
            BookV2ObservedVariantPhase::ContextCapture,
            Some(l.prepared.output_record_limit),
            Some(cap),
        );
        let mut bytes = (l.paragraphs().len() as u64)
            .checked_mul(std::mem::size_of::<ProductionSelectedParagraphContext>() as u64)
            .ok_or(BookV2ObservedAccountingError::Spool {
                owner: NodeId::new(0),
            })?;
        let mut work = 1u64;
        for p in l.paragraphs() {
            self.pay_local(0, 0, 1)?;
            bytes = bytes
                .checked_add(
                    (p.lines().len() as u64)
                        .checked_mul(std::mem::size_of::<u32>() as u64)
                        .ok_or(BookV2ObservedAccountingError::Spool { owner: p.owner() })?,
                )
                .ok_or(BookV2ObservedAccountingError::Spool { owner: p.owner() })?;
            for line in p.lines() {
                self.pay_local(0, 0, 1)?;
                work = work
                    .checked_add(line.items().len() as u64 + 2)
                    .ok_or(BreakError::IterationLimit)?;
            }
        }
        self.pay_local(0, bytes, work)?;
        let mut records = 0;
        let result = l.selected_line_contexts_with_record_limit_counted(cap, &mut records);
        let retained = self.retain(records);
        self.finish(
            records,
            true,
            result.is_ok() && retained.is_ok(),
            result.is_ok() && retained.is_err(),
        );
        let value = result?;
        retained?;
        Ok(value)
    }
    fn context_inputs<'a>(
        &mut self,
        c: &'a ProductionSelectedLineContexts,
    ) -> Result<Vec<ProductionParagraphLineContext<'a>>, Self::Error> {
        let count = c.paragraphs().len() as u64;
        let bytes = count
            .checked_mul(std::mem::size_of::<ProductionParagraphLineContext<'a>>() as u64)
            .ok_or(BookV2ObservedAccountingError::Spool {
                owner: NodeId::new(0),
            })?;
        self.pay_local(count, bytes, count)?;
        let mut inputs = Vec::new();
        super::line_variant_seed::reserve_observed_protocol(
            &mut inputs,
            c.paragraphs().len(),
            super::line_variant_seed::ObservedProtocolReserveSite::StagedInputViews,
        )?;
        inputs.extend(
            c.paragraphs()
                .iter()
                .map(|p| ProductionParagraphLineContext {
                    owner: p.owner(),
                    ends: p.ends(),
                }),
        );
        Ok(inputs)
    }
    fn footnotes<'s, 'p, 'a>(
        &mut self,
        l: &'s BookV2InlineLineLayout<'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        records: &mut u64,
    ) -> Result<BookV2FootnoteLines<'s, 'p, 'a>, Self::Error> {
        let cap = self
            .inner
            .accounting
            .intrinsic_limit(self.inner.intrinsic)?
            .min(l.prepared.output_record_limit);
        self.begin(
            BookV2ObservedVariantPhase::Footnotes,
            Some(l.prepared.output_record_limit),
            Some(cap),
        );
        let result = footnote_lines::prepare_book_v2_footnote_lines_with_record_limit_counted(
            l, limits, records, cap,
        );
        let retained = self.retain(*records);
        self.finish(
            *records,
            true,
            result.is_ok() && retained.is_ok(),
            result.is_ok() && retained.is_err(),
        );
        let value = result?;
        retained?;
        Ok(value)
    }
    fn retain(&mut self, r: u64) -> Result<(), Self::Error> {
        self.inner.retain(r)
    }
    fn source_records(&self) -> u64 {
        self.inner.source_records()
    }
    fn before_callback(&mut self) {
        self.inner.before_callback();
        self.commit();
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn with_observed_variant_capture_lines<'a, 'id, 'cells, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &PreparedBookV2TextFlow<'a>,
    admitted: &AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    mode: JapaneseLineBreakMode,
    body: Rect,
    native: Option<&BookV2ObservedNativeMath<'id, '_>>,
    budget: &mut BookV2ObservedLineVariantBudget,
    accounting: &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    frames: Option<MeasurementPageFrames<'_, 'a>>,
    widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
    inspect: impl FnOnce(
        BookV2ConvergedBodyLines<'_, '_, '_>,
        &mut BookV2ObservedSourceAccounting<'id, 'cells>,
        &mut BookV2ObservedLineVariantBudget,
    ) -> R,
) -> Result<R, BookV2ObservedBodyReshapeError> {
    let mut local =
        BookV2ObservedBodyLineBudget::new(budget.remaining_work(), budget.remaining_passes());
    accounting.with_zero_intrinsic_records(|accounting, intrinsic| {
        let mut state = VariantLineAccounting {
            inner: ObservedLineAccounting {
                local: &mut local,
                accounting,
                intrinsic,
                paid_native: true,
            },
            variant: budget,
            committed: false,
        };
        let result = with_lines_accounting_kernel(
            policy,
            flow,
            admitted,
            bindings,
            limits,
            mode,
            body,
            native.map(BookV2ObservedNativeMath::math),
            &mut state,
            frames,
            widths,
            |stable, state| inspect(stable, state.inner.accounting, state.variant),
        );
        state.commit();
        result
    })
}
