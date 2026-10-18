//! Private outer Page-label convergence. Full command allocation closure and
//! public-profile acceptance remain separate from this actual layout loop.
use super::page_label_state::{ColumnLabelDecision, ColumnPageLabels};
use super::*;

#[path = "book_v2_column_initial_style.rs"]
mod initial_style;
pub use initial_style::{
    prepare_counted_book_v2_column_resources, BookV2ColumnInitialStyleLimit,
    BookV2ColumnInitialStylePreparationError,
};

#[cfg(test)]
#[path = "book_v2_column_converged_pdf_tests.rs"]
mod tests;
#[cfg(test)]
pub(in crate::book_v2_resources) use tests::with_convergence_probe_fixture;

/// One command owns its actual CFF evaluator and accepted retry history.
/// No generated candidate or mutable builder escapes into this carrier.
pub struct BookV2ColumnPdfConvergenceBudget {
    pages: BookV2ColumnPageBudget,
    command: ObservedColumnRegionPdfCommand,
}

impl BookV2ColumnPdfConvergenceBudget {
    pub fn new(limits: &M4EffectiveResourceLimits, maximum_work: u64) -> Self {
        Self {
            pages: BookV2ColumnPageBudget::new(limits, maximum_work),
            command: ObservedColumnRegionPdfCommand::new(limits, maximum_work),
        }
    }
    pub fn observation(&self) -> BookV2PdfConvergenceObservation {
        self.pages.observation()
    }
    pub fn cff_program_statistics(&self) -> typaxis_resources::book_v2::BookV2CffProgramStatistics {
        self.command.fonts.cff_program_statistics()
    }
}

/// Static, immutable access to the very roots used by a complete attempt.
/// The event loan and every source lifetime are fresh to this method, so a host
/// can serialize facts but cannot retain a builder or a borrowed candidate.
pub(in crate::book_v2_resources) trait ColumnAttemptObserver {
    fn observe<
        'event,
        'pdf,
        'nav,
        'w,
        'l,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >(
        &mut self,
        roots: &'event BookV2ColumnPdfBundle<
            'pdf,
            'nav,
            'w,
            'l,
            'u,
            'h,
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'd,
            'g,
            'q,
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        attempt: ColumnPdfAttempt<'event>,
    );

    /// Observe the real parent ledger and retained CFF state after all source
    /// and font loans for this attempt have returned. This outer Result status
    /// does not inspect an opaque application result returned by the callback.
    fn after_attempt(
        &mut self,
        _observation: BookV2PdfConvergenceObservation,
        _cff_statistics: typaxis_resources::book_v2::BookV2CffProgramStatistics,
        _outcome_is_ok: bool,
    ) {
    }
}

struct FnAttemptObserver<F>(F);
impl<F: for<'attempt> FnMut(ColumnPdfAttempt<'attempt>)> ColumnAttemptObserver
    for FnAttemptObserver<F>
{
    fn observe<
        'event,
        'pdf,
        'nav,
        'w,
        'l,
        'u,
        'h,
        'n,
        'm,
        'k,
        'j,
        'r,
        'i,
        't,
        'o,
        'z,
        'y,
        'x,
        'c,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >(
        &mut self,
        _roots: &'event BookV2ColumnPdfBundle<
            'pdf,
            'nav,
            'w,
            'l,
            'u,
            'h,
            'n,
            'm,
            'k,
            'j,
            'r,
            'i,
            't,
            'o,
            'z,
            'y,
            'x,
            'c,
            'v,
            'd,
            'g,
            'q,
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        attempt: ColumnPdfAttempt<'event>,
    ) {
        (self.0)(attempt);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::book_v2_resources) struct ColumnAttemptSourceObservation {
    pub(in crate::book_v2_resources) census_before: BookV2PdfConvergenceObservation,
    pub(in crate::book_v2_resources) census_after: BookV2PdfConvergenceObservation,
    pub(in crate::book_v2_resources) source_before: BookV2PdfConvergenceObservation,
    pub(in crate::book_v2_resources) source_after: BookV2PdfConvergenceObservation,
    pub(in crate::book_v2_resources) copied: bool,
}

/// A borrowed observation of an actual complete attempt. The ordinary entry
/// supplies a no-op observer; there is no production trace allocation.
pub(in crate::book_v2_resources) struct ColumnPdfAttempt<'a> {
    pub(in crate::book_v2_resources) values: &'a [(NodeId, u32)],
    pub(in crate::book_v2_resources) references:
        &'a [typaxis_pdf::book_v2::BookV2PageReferenceObservation],
    pub(in crate::book_v2_resources) display_fingerprint: [u8; 32],
    pub(in crate::book_v2_resources) pdf_fingerprint: [u8; 32],
    pub(in crate::book_v2_resources) pdf_bytes: &'a [u8],
    pub(in crate::book_v2_resources) constructors: [[u64; 4]; 14],
    pub(in crate::book_v2_resources) observation: BookV2PdfConvergenceObservation,
    pub(in crate::book_v2_resources) source: ColumnAttemptSourceObservation,
    pub(in crate::book_v2_resources) display_ready: BookV2PdfConvergenceObservation,
    pub(in crate::book_v2_resources) assembly_before_labels: [u64; 4],
    pub(in crate::book_v2_resources) assembly_reservation_sizes: [usize; 4],
    pub(super) decision: ColumnLabelDecision,
}

impl ColumnPdfAttempt<'_> {
    #[cfg(test)]
    pub(in crate::book_v2_resources) fn decision_name(&self) -> &'static str {
        match self.decision {
            ColumnLabelDecision::Retry => "retry",
            ColumnLabelDecision::Ready => "ready",
        }
    }
}

/// Deliver only a completed matching candidate. With authored Page sites, two
/// consecutive complete display/PDF states must agree. Without sites, one real
/// PDF over the existing physical stability proof suffices.
///
/// Original preparation and every generated source remain inside one scoped
/// context. Work, source slots, begun passes, lower-stage reservations and the
/// retained font evaluator span all retries. Initial/admission/style/shaping
/// physical allocation accounting is not asserted by this private entry.
pub fn with_budgeted_converged_book_v2_column_pdf<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPdfConvergenceBudget,
    first_font_object: u32,
    inspect: impl for<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnPdfBundle<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    with_converged_column_pdf_observed(
        input,
        limits,
        japanese_mode,
        budget,
        first_font_object,
        inspect,
        |_| {},
    )
}

pub(super) fn with_converged_column_pdf_observed<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPdfConvergenceBudget,
    first_font_object: u32,
    inspect: impl for<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnPdfBundle<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
    observe_attempt: impl for<'attempt> FnMut(ColumnPdfAttempt<'attempt>),
) -> Result<R, E> {
    let mut observer = FnAttemptObserver(observe_attempt);
    with_converged_column_pdf_observer(
        input,
        limits,
        japanese_mode,
        budget,
        first_font_object,
        inspect,
        &mut observer,
    )
}

pub(in crate::book_v2_resources) fn with_converged_column_pdf_observer<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPdfConvergenceBudget,
    first_font_object: u32,
    inspect: impl for<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnPdfBundle<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
    observer: &mut impl ColumnAttemptObserver,
) -> Result<R, E> {
    let previous = budget.pages.observation();
    let BookV2ColumnPdfConvergenceBudget { pages, command } = budget;
    source_context::with_p1_observed_original_column_command_context(
        input,
        limits,
        japanese_mode,
        pages,
        command,
        |original, accounting, rest,command| {
            let mut progress = ColumnProgressView { accounting, rest };
            let census_before = progress.snapshot();
            let census = original.census_observed(&mut progress)?;
            let census_after = progress.snapshot();
            let mut labels = ColumnPageLabels::new_observed(&census, limits, &mut progress)?;
            let mut inspect = Some(inspect);
            let mut candidate_previous = previous;
            loop {
                if limits
                    .base()
                    .get()
                    .max_layout_passes
                    .saturating_sub(*progress.rest.page_passes)
                    < 2
                {
                    return Err(E::Limit("column Page convergence passes"));
                }
                let source_before = progress.snapshot();
                let outcome = labels.with_candidate_values(|values, labels| {
                    if census.owners().is_empty() {
                        let source = ColumnAttemptSourceObservation {
                            census_before,
                            census_after,
                            source_before,
                            source_after: progress.snapshot(),
                            copied: false,
                        };
                        with_candidate_pdf(
                            original,
                            original.flow,
                            &mut progress,
                            command,
                            first_font_object,
                            candidate_previous,
                            values,
                            labels,
                            source,
                            &mut inspect,
                            observer,
                        )
                    } else {
                        original.with_page_references_observed(
                            &census,
                            values,
                            &mut progress,
                            |flow, progress| {
                                let source = ColumnAttemptSourceObservation {
                                    census_before,
                                    census_after,
                                    source_before,
                                    source_after: progress.snapshot(),
                                    copied: true,
                                };
                                with_candidate_pdf(
                                    original,
                                    flow,
                                    progress,
                                    command,
                                    first_font_object,
                                    candidate_previous,
                                    values,
                                    labels,
                                    source,
                                    &mut inspect,
                                    observer,
                                )
                            },
                        )
                    }
                });
                // Every pending producer prefix has rejoined the same live loan.
                observer.after_attempt(
                    progress.snapshot(),
                    command.fonts.cff_program_statistics(),
                    outcome.is_ok(),
                );
                if let Some(result) = outcome? {
                    return Ok(result);
                }
                candidate_previous = progress.snapshot();
            }
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn with_candidate_pdf<'id, 'cells, R>(
    original: &source_context::P1ObservedColumnOriginalContext<'_, 'id>,
    flow: &PreparedBookV2TextFlow<'_>,
    progress: &mut ColumnProgressView<'_, 'id, 'cells>,
    command: &mut ObservedColumnRegionPdfCommand,
    first_font_object: u32,
    previous: BookV2PdfConvergenceObservation,
    candidate_values: &[(NodeId, u32)],
    labels: &mut ColumnPageLabels,
    source: ColumnAttemptSourceObservation,
    inspect: &mut Option<impl for<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(&BookV2ColumnPdfBundle<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2PdfConvergenceObservation) -> R>,
    observer: &mut impl ColumnAttemptObserver,
) -> Result<Option<R>, E> {
    observed_adoption::with_candidate_math(original, flow, progress, |terminals, progress| {
        with_observed_column_display_from_math(
            terminals,
            original.policy,
            original.input,
            original.limits,
            original.mode,
            progress,
            |display, progress| {
                let display_ready = progress.snapshot();
                with_observed_column_pdf_finalized(
                    display,
                    command,
                    original.limits,
                    first_font_object,
                    previous,
                    progress,
                    ConvergedColumnPdfFinish {
                        limits: original.limits,
                        maximum: original.maximum,
                        values: candidate_values,
                        labels,
                        source,
                        display_ready,
                        inspect,
                        observer,
                    },
                )
            },
        )
    })
}

struct ConvergedColumnPdfFinish<'state, F, O> {
    limits: &'state M4EffectiveResourceLimits,
    maximum: u64,
    values: &'state [(NodeId, u32)],
    labels: &'state mut ColumnPageLabels,
    source: ColumnAttemptSourceObservation,
    display_ready: BookV2PdfConvergenceObservation,
    inspect: &'state mut Option<F>,
    observer: &'state mut O,
}
impl<'state, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a, F, O, R> ObservedColumnPdfFinish<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a, Option<R>>
    for ConvergedColumnPdfFinish<'state, F, O>
where F: for<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v> FnOnce(&BookV2ColumnPdfBundle<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2PdfConvergenceObservation) -> R,
      O: ColumnAttemptObserver,
{
    fn finish<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v>(
        self, roots: &BookV2ColumnPdfBundle<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        assembly_before: [u64; 4], assembly_sizes: [usize; 4],
        producer: &mut BookV2PdfConvergenceObservation,
    ) -> Result<Option<R>, E> {
        let pdf = roots.pdf();
        let decision = self.labels.observe_pdf_counted(self.values, pdf.page_references(),
            pdf.page_reference_labels_match(), roots.display().fingerprint(), pdf.fingerprint(),
            self.limits, self.maximum, producer)?;
        self.observer.observe(roots, ColumnPdfAttempt {
            values: self.values, references: pdf.page_references(),
            display_fingerprint: roots.display().fingerprint(), pdf_fingerprint: pdf.fingerprint(),
            pdf_bytes: pdf.bytes(), constructors: roots.constructor_charges(), observation: *producer,
            source: self.source, display_ready: self.display_ready, assembly_before_labels: assembly_before,
            assembly_reservation_sizes: assembly_sizes, decision,
        });
        match decision {
            ColumnLabelDecision::Retry => Ok(None),
            ColumnLabelDecision::Ready => Ok(Some(self.inspect.take().ok_or(E::Identity)?(roots, *producer))),
        }
    }
}
