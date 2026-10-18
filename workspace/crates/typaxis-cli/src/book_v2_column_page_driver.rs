//! Automatic source-width/column-page feedback with one retained caller ledger.
//! Source and physical consumers share all width discovery and caller budgets.
//! Source closure follows physical stability on the same caller ledger.
//! Balance receipts participate in width feedback; PDF receipts remain separate.
use super::column_catalog_driver::{with_discovered_column_header_catalog, ColumnHeaderBudget};
use super::header_catalog_driver::{reserve, HeaderCatalogBudget};
use super::*;

#[path = "book_v2_column_source_context.rs"]
mod source_context;

#[path = "book_v2_column_observed_adoption.rs"]
mod observed_adoption;

#[path = "book_v2_column_page_label_state.rs"]
mod page_label_state;

#[path = "book_v2_column_converged_pdf.rs"]
mod converged_column_pdf;
pub use converged_column_pdf::{
    prepare_counted_book_v2_column_resources, BookV2ColumnInitialStyleLimit,
    BookV2ColumnInitialStylePreparationError,
};
pub use converged_column_pdf::{
    with_budgeted_converged_book_v2_column_pdf, BookV2ColumnPdfConvergenceBudget,
};
#[cfg(test)]
pub(in crate::book_v2_resources) use converged_column_pdf::{
    with_converged_column_pdf_observer, with_convergence_probe_fixture, ColumnAttemptObserver,
    ColumnPdfAttempt,
};

#[path = "book_v2_column_resources_driver.rs"]
mod resource_driver;
pub use resource_driver::{with_budgeted_book_v2_column_resources, BookV2ColumnResourceBundle};
#[path = "book_v2_column_paints_driver.rs"]
mod paints_driver;
pub use paints_driver::{with_budgeted_book_v2_column_paints, BookV2ColumnPaintBundle};
#[path = "book_v2_column_marked_driver.rs"]
mod marked_driver;
pub use marked_driver::{with_budgeted_book_v2_column_marked, BookV2ColumnMarkedBundle};
#[path = "book_v2_column_relations_driver.rs"]
mod relations_driver;
pub use relations_driver::{with_budgeted_book_v2_column_relations, BookV2ColumnRelationBundle};
#[path = "book_v2_column_navigation_driver.rs"]
mod navigation_driver;
pub use navigation_driver::{
    with_budgeted_book_v2_column_navigation, BookV2ColumnNavigationBundle,
};

#[path = "book_v2_column_pdf_driver.rs"]
mod pdf_driver;
pub use pdf_driver::{with_budgeted_book_v2_column_pdf, BookV2ColumnPdfBundle};

#[path = "book_v2_column_page_region_driver.rs"]
pub(in crate::book_v2_resources) mod column_page_region_driver;

/// Retries keep accepted work, records and begun line/page passes. The existing
/// private driver observation is shared, without granting PDF/public authority.
pub struct BookV2ColumnPageBudget {
    inner: BookV2PdfConvergenceBudget,
}
impl BookV2ColumnPageBudget {
    pub fn new(limits: &M4EffectiveResourceLimits, maximum_work: u64) -> Self {
        Self {
            inner: BookV2PdfConvergenceBudget::new(limits, maximum_work),
        }
    }
    pub fn observation(&self) -> BookV2PdfConvergenceObservation {
        self.inner.observation()
    }
}

trait ColumnDelivery<R> {
    fn finish<'b, 'f, 's, 'p, 'a>(
        self,
        pages: &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        search: &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        feedback: &BookV2ColumnWidthFeedback<'p, 'a>,
        policy: &BookV2ResourcePolicy<'_>,
        limits: &M4EffectiveResourceLimits,
        maximum: u64,
        ledger: &mut HeaderCatalogBudget,
        total: &mut BookV2PdfConvergenceObservation,
    ) -> Result<R, E>;
}
// The observed path splits the actual parent once. No library callback
// captures the whole parent while its three counter fields are loaned.
struct ColumnRestProgress<'cells> {
    candidates: &'cells mut u16,
    balance_candidates: &'cells mut u64,
    width_passes: &'cells mut u16,
    width_refinements: &'cells mut u16,
    line_passes: &'cells mut u16,
    page_passes: &'cells mut u16,
    output: &'cells mut u64,
}
struct ColumnProgressView<'call, 'id, 'cells> {
    accounting: &'call mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
    rest: &'call mut ColumnRestProgress<'cells>,
}

/// Run one closed counted Source operation on a snapshot, then publish each
/// genuine accepted axis to the live loan before propagating its Result. The
/// callback stays internal; it cannot loan a temporary budget to application
/// code, spend Source concurrently, or alter physical/output history.
fn with_observed_source_counter_operation<R>(
    progress: &mut ColumnProgressView<'_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
    operation: impl FnOnce(&mut BookV2ColumnPageBudget) -> Result<R, E>,
) -> Result<R, E> {
    progress.require_pagination_domain(limits)?;
    progress
        .accounting
        .require_limit_domain(limits.base())
        .map_err(E::ObservedAccounting)?;
    progress
        .accounting
        .debit(NodeId::new(0), [0; 3])
        .map_err(E::ObservedAccounting)?;
    let before = progress.snapshot();
    let mut local = BookV2ColumnPageBudget {
        inner: BookV2PdfConvergenceBudget {
            limits_fingerprint: limits.fingerprint(),
            maximum_work: progress.accounting.maximum_work(),
            observation: before,
        },
    };
    let result = operation(&mut local);
    let after = local.observation();
    let retained_r = after
        .records
        .checked_sub(before.records)
        .ok_or(E::Identity)
        .and_then(|n| {
            progress
                .accounting
                .debit_records(NodeId::new(0), n)
                .map_err(E::ObservedAccounting)
        });
    let retained_s = after
        .spool
        .checked_sub(before.spool)
        .ok_or(E::Identity)
        .and_then(|n| {
            progress
                .accounting
                .debit_spool(NodeId::new(0), n)
                .map_err(E::ObservedAccounting)
        });
    let retained_w = after
        .work
        .checked_sub(before.work)
        .ok_or(E::Identity)
        .and_then(|n| {
            progress
                .accounting
                .debit_work(NodeId::new(0), n)
                .map_err(E::ObservedAccounting)
        });
    let same_rest = after.candidates == before.candidates
        && after.balance_candidates == before.balance_candidates
        && after.width_passes == before.width_passes
        && after.width_refinements == before.width_refinements
        && after.line_passes == before.line_passes
        && after.page_passes == before.page_passes
        && after.output == before.output;
    let value = result?;
    if !same_rest {
        return Err(E::Identity);
    }
    retained_r?;
    retained_s?;
    retained_w?;
    Ok(value)
}
fn with_observed_column_progress<'cells, R>(
    total: &'cells mut BookV2PdfConvergenceObservation,
    limits: &'cells typaxis_core::ValidatedResourceLimits,
    maximum_records: u64,
    maximum_work: u64,
    inspect: impl for<'id> FnOnce(
        &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
        &mut ColumnRestProgress<'cells>,
    ) -> R,
) -> R {
    let BookV2PdfConvergenceObservation {
        candidates,
        balance_candidates,
        width_passes,
        width_refinements,
        line_passes,
        page_passes,
        records,
        spool,
        output,
        work,
    } = total;
    let mut rest = ColumnRestProgress {
        candidates,
        balance_candidates,
        width_passes,
        width_refinements,
        line_passes,
        page_passes,
        output,
    };
    typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(
        records,
        spool,
        work,
        limits,
        maximum_records,
        maximum_work,
        |accounting| inspect(accounting, &mut rest),
    )
}
// Each mark belongs to one live private search run. All independent source
// spending precedes the search; it resumes only after that search borrow ends.
struct ColumnSearchChargeMark {
    records: u64,
    spool: u64,
    work: u64,
}
impl ColumnProgressView<'_, '_, '_> {
    fn snapshot(&self) -> BookV2PdfConvergenceObservation {
        let [records, spool, work] = self.accounting.counters();
        BookV2PdfConvergenceObservation {
            candidates: *self.rest.candidates,
            balance_candidates: *self.rest.balance_candidates,
            width_passes: *self.rest.width_passes,
            width_refinements: *self.rest.width_refinements,
            line_passes: *self.rest.line_passes,
            page_passes: *self.rest.page_passes,
            output: *self.rest.output,
            records,
            spool,
            work,
        }
    }
    fn require_pagination_domain(&self, limits: &M4EffectiveResourceLimits) -> Result<(), E> {
        if self.accounting.maximum_records() != limits.base().get().max_fragments
            || self.accounting.maximum_spool() != limits.base().get().max_spool_bytes
        {
            return Err(E::ObservedAccounting(
                typaxis_shaping::book_v2::BookV2ObservedAccountingError::LimitsMismatch,
            ));
        }
        Ok(())
    }
    fn search_mark(&self) -> ColumnSearchChargeMark {
        let [records, spool, _] = self.accounting.counters();
        ColumnSearchChargeMark {
            records,
            spool,
            work: 0,
        }
    }
    fn retain_search(
        &mut self,
        search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
        mark: &mut ColumnSearchChargeMark,
    ) -> Result<(), E> {
        let records = search.record_charge();
        let spool = search.terminal_spool_charge();
        let work = search.work_steps();
        let delta = [
            records.checked_sub(mark.records).ok_or(E::Identity)?,
            spool.checked_sub(mark.spool).ok_or(E::Identity)?,
            work.checked_sub(mark.work).ok_or(E::Identity)?,
        ];
        self.accounting
            .debit(typaxis_core::NodeId::new(0), delta)
            .map_err(E::ObservedAccounting)?;
        *self.rest.balance_candidates =
            (*self.rest.balance_candidates).max(search.balance_candidates());
        mark.records = records;
        mark.spool = spool;
        mark.work = work;
        Ok(())
    }
}
// Narrow actual P0 nesting: no native blocks, no header catalog/prepayment.
// The existing pagination APIs use the authenticated body record ceiling, so
// reject a smaller loan ceiling before constructing any pagination carrier.
fn with_p0_observed_column_pagination<'f, 's, 'p, 'a, R>(
    stable: &'f BookV2ConvergedColumnLines<'s, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    progress: &mut ColumnProgressView<'_, '_, '_>,
    inspect: impl for<'b> FnOnce(
        &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnWidthFeedback<'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    progress.require_pagination_domain(limits)?;
    let mut prior = progress.accounting.counters()[0];
    let mut records = prior;
    let flow = prepare_book_v2_column_flow_counted(stable, None, limits, prior, &mut records);
    let retained = progress.accounting.debit(
        typaxis_core::NodeId::new(0),
        [records.checked_sub(prior).ok_or(E::Identity)?, 0, 0],
    );
    let flow = flow.map_err(E::ObservedPagination)?;
    retained.map_err(E::ObservedAccounting)?;
    prior = progress.accounting.counters()[0];
    records = prior;
    let measurements =
        prepare_book_v2_column_table_measurements_counted(flow, limits, &mut records);
    let retained = progress.accounting.debit(
        typaxis_core::NodeId::new(0),
        [records.checked_sub(prior).ok_or(E::Identity)?, 0, 0],
    );
    let measurements = measurements.map_err(E::ObservedPagination)?;
    retained.map_err(E::ObservedAccounting)?;
    let mut mark = progress.search_mark();
    records = mark.records;
    let mut work = 0;
    let search = prepare_book_v2_column_page_search_counted(
        &measurements,
        limits,
        progress.accounting.remaining_work(),
        mark.records,
        &mut records,
        &mut work,
    );
    let mut search = match search {
        Ok(search) => search,
        Err(error) => {
            let _ = progress.accounting.debit(
                typaxis_core::NodeId::new(0),
                [
                    records.checked_sub(mark.records).ok_or(E::Identity)?,
                    0,
                    work,
                ],
            );
            return Err(E::ObservedPagination(error));
        }
    };
    search.retain_prior_balance_candidates(*progress.rest.balance_candidates);
    let selected = (|| {
        search
            .retain_prior_balance_spool(progress.accounting.counters()[1])
            .map_err(E::ObservedPagination)?;
        let mut begun = 0;
        let pages = search.select_repeated_pages_counted(
            limits
                .base()
                .get()
                .max_layout_passes
                .checked_sub(*progress.rest.page_passes)
                .ok_or(E::Limit("column page passes"))?,
            &mut begun,
        );
        *progress.rest.page_passes = (*progress.rest.page_passes)
            .checked_add(begun)
            .filter(|n| *n <= limits.base().get().max_layout_passes)
            .ok_or(E::Limit("column page passes"))?;
        let pages = pages.map_err(E::ObservedPagination)?;
        let feedback = search
            .paragraph_frame_feedback(pages.sequence())
            .map_err(E::ObservedPagination)?;
        SourceColumns(inspect).finish_observed(
            &pages,
            &mut search,
            &feedback,
            progress.accounting.maximum_work(),
            progress,
            &mut mark,
        )
    })();
    let retained = progress.retain_search(&search, &mut mark);
    match selected {
        Err(error) => Err(error),
        Ok(value) => {
            retained?;
            Ok(value)
        }
    }
}
trait SourceColumnFinishProgress {
    fn observation(
        &mut self,
        search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
        maximum: u64,
    ) -> Result<BookV2PdfConvergenceObservation, E>;
    fn after_callback(
        &mut self,
        search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
    ) -> Result<(), E>;
}
struct LegacySourceColumnProgress<'loan> {
    ledger: &'loan HeaderCatalogBudget,
    total: &'loan mut BookV2PdfConvergenceObservation,
}
impl SourceColumnFinishProgress for LegacySourceColumnProgress<'_> {
    fn observation(
        &mut self,
        search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
        maximum: u64,
    ) -> Result<BookV2PdfConvergenceObservation, E> {
        column_observation(search, self.ledger, self.total, maximum)
    }
    fn after_callback(&mut self, _: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>) -> Result<(), E> {
        Ok(())
    }
}
struct ObservedSourceColumnProgress<'loan, 'call, 'id, 'cells> {
    view: &'loan mut ColumnProgressView<'call, 'id, 'cells>,
    mark: &'loan mut ColumnSearchChargeMark,
}
impl SourceColumnFinishProgress for ObservedSourceColumnProgress<'_, '_, '_, '_> {
    fn observation(
        &mut self,
        search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
        _: u64,
    ) -> Result<BookV2PdfConvergenceObservation, E> {
        self.view.retain_search(search, self.mark)?;
        *self.view.rest.candidates = (*self.view.rest.candidates)
            .checked_add(1)
            .ok_or(E::Limit("column candidates"))?;
        Ok(self.view.snapshot())
    }
    fn after_callback(
        &mut self,
        search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
    ) -> Result<(), E> {
        self.view.retain_search(search, self.mark)
    }
}
#[allow(clippy::too_many_arguments)]
fn finish_source_columns<'b, 'f, 's, 'p, 'a, R>(
    callback: impl FnOnce(
        &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnWidthFeedback<'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
    pages: &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
    search: &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
    feedback: &BookV2ColumnWidthFeedback<'p, 'a>,
    maximum: u64,
    progress: &mut impl SourceColumnFinishProgress,
) -> Result<R, E> {
    let observed = progress.observation(search, maximum)?;
    let output = callback(pages, search, feedback, observed);
    progress.after_callback(search)?;
    Ok(output)
}
struct SourceColumns<F>(F);
struct PhysicalColumns<F>(F);
fn column_observation(
    search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
    ledger: &HeaderCatalogBudget,
    total: &mut BookV2PdfConvergenceObservation,
    maximum: u64,
) -> Result<BookV2PdfConvergenceObservation, E> {
    let observation = BookV2PdfConvergenceObservation {
        candidates: total
            .candidates
            .checked_add(1)
            .ok_or(E::Limit("column candidates"))?,
        records: search.record_charge(),
        balance_candidates: search.balance_candidates(),
        spool: total.spool.max(search.terminal_spool_charge()),
        work: add(
            ledger.work,
            search.work_steps(),
            maximum,
            "column page work",
        )?,
        line_passes: ledger.line_passes,
        page_passes: ledger.page_passes,
        ..*total
    };
    total.candidates = observation.candidates;
    Ok(observation)
}
impl<R, F> ColumnDelivery<R> for SourceColumns<F>
where
    F: for<'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnWidthFeedback<'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
{
    fn finish<'b, 'f, 's, 'p, 'a>(
        self,
        pages: &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        search: &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        feedback: &BookV2ColumnWidthFeedback<'p, 'a>,
        _policy: &BookV2ResourcePolicy<'_>,
        _limits: &M4EffectiveResourceLimits,
        maximum: u64,
        ledger: &mut HeaderCatalogBudget,
        total: &mut BookV2PdfConvergenceObservation,
    ) -> Result<R, E> {
        finish_source_columns(
            self.0,
            pages,
            search,
            feedback,
            maximum,
            &mut LegacySourceColumnProgress { ledger, total },
        )
    }
}
impl<F> SourceColumns<F> {
    #[allow(clippy::too_many_arguments)]
    fn finish_observed<'b, 'f, 's, 'p, 'a, R>(
        self,
        pages: &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        search: &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        feedback: &BookV2ColumnWidthFeedback<'p, 'a>,
        maximum: u64,
        progress: &mut ColumnProgressView<'_, '_, '_>,
        mark: &mut ColumnSearchChargeMark,
    ) -> Result<R, E>
    where
        F: FnOnce(
            &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
            &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
            &BookV2ColumnWidthFeedback<'p, 'a>,
            BookV2PdfConvergenceObservation,
        ) -> R,
    {
        finish_source_columns(
            self.0,
            pages,
            search,
            feedback,
            maximum,
            &mut ObservedSourceColumnProgress {
                view: progress,
                mark,
            },
        )
    }
}
impl<R, F> ColumnDelivery<R> for PhysicalColumns<F>
where
    F: for<'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnStablePages<'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnWidthFeedback<'p, 'a>,
        &BookV2ResourcePolicy<'_>,
        &mut u16,
        BookV2PdfConvergenceObservation,
    ) -> R,
{
    fn finish<'b, 'f, 's, 'p, 'a>(
        self,
        _pages: &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        search: &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        feedback: &BookV2ColumnWidthFeedback<'p, 'a>,
        policy: &BookV2ResourcePolicy<'_>,
        limits: &M4EffectiveResourceLimits,
        maximum: u64,
        ledger: &mut HeaderCatalogBudget,
        total: &mut BookV2PdfConvergenceObservation,
    ) -> Result<R, E> {
        // Initial source widths can span a full body; place only after reflow.
        let mut begun = 0;
        let physical = search.select_stable_column_pages_counted(
            limits.base().get().max_layout_passes - ledger.page_passes,
            &mut begun,
        );
        ledger.page_passes = ledger
            .page_passes
            .checked_add(begun)
            .filter(|n| *n <= limits.base().get().max_layout_passes)
            .ok_or(E::Limit("column physical page passes"))?;
        let pages = physical.map_err(|e| stage("column physical stability", e))?;
        total.width_passes = total
            .width_passes
            .checked_add(1)
            .ok_or(E::Limit("column width passes"))?;
        let physical_feedback = search
            .paragraph_frame_feedback(pages.sequence())
            .map_err(|e| stage("column physical width feedback", e))?;
        if physical_feedback.assignment_fingerprint() != feedback.assignment_fingerprint()
            || !physical_feedback.matches_selected_line_widths()
            || !physical_feedback.matches_selected_block_widths()
        {
            return Err(E::Identity);
        }
        let observed = column_observation(search, ledger, total, maximum)?;
        Ok((self.0)(
            &pages,
            search,
            &physical_feedback,
            policy,
            &mut ledger.line_passes,
            observed,
        ))
    }
}

/// Keep the exact original resources and plan across source-unit reshaping,
/// actual header-width discovery and repeated joint column/note selection.
pub fn with_budgeted_book_v2_column_pages<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnWidthFeedback<'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    with_column_pages(input, limits, japanese_mode, budget, SourceColumns(inspect))
}

/// Deliver actual repeated geometry after the common source-width convergence.
/// Begun physical passes and callback consumption retain the same caller ledger.
pub fn with_budgeted_book_v2_column_placement<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnStablePages<'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnWidthFeedback<'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    with_budgeted_book_v2_column_placement_context(
        input,
        limits,
        japanese_mode,
        budget,
        |pages, search, feedback, _policy, _line_passes, observed| {
            inspect(pages, search, feedback, observed)
        },
    )
}

fn with_budgeted_book_v2_column_placement_context<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnStablePages<'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnWidthFeedback<'p, 'a>,
        &BookV2ResourcePolicy<'_>,
        &mut u16,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    with_column_pages(
        input,
        limits,
        japanese_mode,
        budget,
        PhysicalColumns(inspect),
    )
}

/// Deliver verified original-source coverage after physical width convergence.
/// The exact stable sequence and placed geometry stay borrowed throughout the
/// callback. Closure failures retain their accepted prefix without delivery.
pub fn with_budgeted_book_v2_column_sources<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnSourceClosure<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    let maximum = budget.inner.maximum_work;
    with_budgeted_book_v2_column_placement(
        input,
        limits,
        japanese_mode,
        budget,
        |pages, search, _feedback, observed| {
            let prior_work = search.work_steps();
            let geometry = search
                .place_column_pages(pages.sequence())
                .map_err(|e| stage("column source placement", e))?;
            let source = search
                .close_column_page_sources(pages, &geometry)
                .map_err(|e| stage("column source closure", e))?;
            let observed = BookV2PdfConvergenceObservation {
                records: search.record_charge(),
                work: add(
                    observed.work,
                    search
                        .work_steps()
                        .checked_sub(prior_work)
                        .ok_or(E::Identity)?,
                    maximum,
                    "column source work",
                )?,
                ..observed
            };
            Ok(inspect(&source, search, observed))
        },
    )?
}

/// Deliver physical formula terminals over the verified column source closure.
/// Accepted terminal spool survives failures and further callback consumption.
pub fn with_budgeted_book_v2_column_math<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    with_budgeted_book_v2_column_math_context(
        input,
        limits,
        japanese_mode,
        budget,
        |terminals, search, _policy, _line_passes, observed| inspect(terminals, search, observed),
    )
}

fn with_budgeted_book_v2_column_math_context<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ResourcePolicy<'_>,
        &mut u16,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    let maximum = budget.inner.maximum_work;
    let prior_spool = budget.inner.observation.spool;
    let mut retained_spool = prior_spool;
    let result = with_budgeted_book_v2_column_placement_context(
        input,
        limits,
        japanese_mode,
        budget,
        |pages, search, _feedback, policy, line_passes, observed| {
            with_column_math_from_placement(
                pages,
                search,
                policy,
                line_passes,
                observed,
                limits,
                maximum,
                prior_spool,
                &mut retained_spool,
                inspect,
            )
        },
    );
    budget.inner.observation.spool = budget.inner.observation.spool.max(retained_spool);
    result?
}

/// Deliver every column paint and nonpainting anchor after source and physical
/// convergence. Constructor, build, callback and retry prefixes share the caller
/// ledger even when a display cannot be returned.
pub fn with_budgeted_book_v2_column_display<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'loan, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &typaxis_display_list::book_v2::BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut typaxis_display_list::book_v2::BookV2ColumnDisplayContinuation<
            'loan,
            'd,
            'g,
            'q,
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    let maximum = budget.inner.maximum_work;
    let mut records = budget.inner.observation.records;
    let mut work = budget.inner.observation.work;
    let outcome = with_budgeted_book_v2_column_math_context(
        input,
        limits,
        japanese_mode,
        budget,
        |terminals, _search, policy, line_passes, observed| {
            with_column_display_from_math(
                terminals,
                policy,
                line_passes,
                observed,
                input,
                limits,
                japanese_mode,
                maximum,
                &mut records,
                &mut work,
                inspect,
            )
        },
    );
    budget.inner.observation.records = budget.inner.observation.records.max(records);
    budget.inner.observation.work = budget.inner.observation.work.max(work);
    outcome?
}

#[allow(clippy::too_many_arguments)]
fn with_column_math_from_placement<'b, 'f, 's, 'p, 'a, R>(
    pages: &BookV2ColumnStablePages<'b, 'f, 's, 'p, 'a>,
    search: &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
    policy: &BookV2ResourcePolicy<'_>,
    line_passes: &mut u16,
    observed: BookV2PdfConvergenceObservation,
    limits: &M4EffectiveResourceLimits,
    maximum: u64,
    prior_spool: u64,
    retained_spool: &mut u64,
    inspect: impl for<'g, 'q> FnOnce(
        &BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ResourcePolicy<'_>,
        &mut u16,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    let prior_work = search.work_steps();
    let geometry = search
        .place_column_pages(pages.sequence())
        .map_err(|e| stage("column math placement", e))?;
    let source = search
        .close_column_page_sources(pages, &geometry)
        .map_err(|e| stage("column math source closure", e))?;
    let terminals =
        search.finalize_column_page_math(source, limits, observed.spool.max(prior_spool));
    *retained_spool = (*retained_spool).max(search.terminal_spool_charge());
    let terminals = terminals.map_err(|e| stage("column math terminals", e))?;
    let observed = BookV2PdfConvergenceObservation {
        records: search.record_charge(),
        spool: *retained_spool,
        work: add(
            observed.work,
            search
                .work_steps()
                .checked_sub(prior_work)
                .ok_or(E::Identity)?,
            maximum,
            "column math work",
        )?,
        ..observed
    };
    let output = inspect(&terminals, search, policy, line_passes, observed);
    *retained_spool = (*retained_spool).max(search.terminal_spool_charge());
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn with_column_display_from_math<'g, 'q, 'b, 'f, 's, 'p, 'a, R>(
    terminals: &BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    policy: &BookV2ResourcePolicy<'_>,
    line_passes: &mut u16,
    observed: BookV2PdfConvergenceObservation,
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    maximum: u64,
    records: &mut u64,
    work: &mut u64,
    inspect: impl for<'loan, 'd> FnOnce(
        &typaxis_display_list::book_v2::BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut typaxis_display_list::book_v2::BookV2ColumnDisplayContinuation<
            'loan,
            'd,
            'g,
            'q,
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    use typaxis_display_list::book_v2::BookV2ColumnDisplayBuilder;
    let constructor = BookV2ColumnDisplayBuilder::new_counted(
        terminals,
        input.resources(),
        limits,
        maximum,
        observed.records,
        observed.work,
        records,
        work,
    );
    let mut builder = constructor.map_err(|e| stage("column display constructor", e))?;
    let display = builder.build();
    *records = (*records).max(builder.record_charge());
    *work = (*work).max(builder.work_steps());
    let display = display.map_err(|e| stage("column display", e))?;
    let joined = column_page_region_driver::attach(
        display,
        policy,
        limits,
        japanese_mode,
        maximum,
        line_passes,
        records,
        work,
    );
    *records = (*records).max(builder.record_charge());
    *work = (*work).max(builder.work_steps());
    let display = joined?;
    builder
        .continue_after_display(&display)
        .map_err(|e| stage("column display continuation", e))?;
    let observed = BookV2PdfConvergenceObservation {
        records: *records,
        work: *work,
        line_passes: *line_passes,
        ..observed
    };
    let result =
        builder.with_continuation(|continuation| inspect(&display, continuation, observed));
    *records = (*records).max(builder.record_charge());
    *work = (*work).max(builder.work_steps());
    Ok(result)
}

fn with_column_candidate_placement_context<R>(
    candidate: &source_context::ColumnCandidateContext<'_>,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnStablePages<'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnWidthFeedback<'p, 'a>,
        &BookV2ResourcePolicy<'_>,
        &mut u16,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    with_column_candidate_pages(candidate, budget, PhysicalColumns(inspect))
}

fn with_column_candidate_math_context<R>(
    candidate: &source_context::ColumnCandidateContext<'_>,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ResourcePolicy<'_>,
        &mut u16,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    let maximum = budget.inner.maximum_work;
    let prior_spool = budget.inner.observation.spool;
    let mut retained_spool = prior_spool;
    let result = with_column_candidate_placement_context(
        candidate,
        budget,
        |pages, search, _feedback, policy, line_passes, observed| {
            with_column_math_from_placement(
                pages,
                search,
                policy,
                line_passes,
                observed,
                candidate.original.limits,
                maximum,
                prior_spool,
                &mut retained_spool,
                inspect,
            )
        },
    );
    budget.inner.observation.spool = budget.inner.observation.spool.max(retained_spool);
    result?
}

/// Continue the actual generated-label source through the same physical,
/// terminal, display and original running-region kernels as the public path.
fn with_column_candidate_display<R>(
    candidate: &source_context::ColumnCandidateContext<'_>,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'loan, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> FnOnce(
        &typaxis_display_list::book_v2::BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut typaxis_display_list::book_v2::BookV2ColumnDisplayContinuation<
            'loan,
            'd,
            'g,
            'q,
            'b,
            'f,
            's,
            'p,
            'a,
        >,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    candidate.verify_budget(budget)?;
    let maximum = budget.inner.maximum_work;
    let mut records = budget.inner.observation.records;
    let mut work = budget.inner.observation.work;
    let original = candidate.original;
    let outcome = with_column_candidate_math_context(
        candidate,
        budget,
        |terminals, _search, policy, line_passes, observed| {
            with_column_display_from_math(
                terminals,
                policy,
                line_passes,
                observed,
                original.input,
                original.limits,
                original.mode,
                maximum,
                &mut records,
                &mut work,
                inspect,
            )
        },
    );
    budget.inner.observation.records = budget.inner.observation.records.max(records);
    budget.inner.observation.work = budget.inner.observation.work.max(work);
    outcome?
}

fn with_column_pages<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    consumer: impl ColumnDelivery<R>,
) -> Result<R, E> {
    source_context::with_original_column_context(
        input,
        limits,
        japanese_mode,
        budget,
        |original, budget| {
            let candidate = original.original_candidate();
            with_column_candidate_pages(&candidate, budget, consumer)
        },
    )
}

fn with_column_candidate_pages<R>(
    candidate: &source_context::ColumnCandidateContext<'_>,
    budget: &mut BookV2ColumnPageBudget,
    consumer: impl ColumnDelivery<R>,
) -> Result<R, E> {
    candidate.verify_budget(budget)?;
    let original = candidate.original;
    with_column_source_pages(
        original.policy,
        candidate.flow,
        original.input.resources(),
        original.bindings,
        original.limits,
        original.mode,
        original.native,
        original.columns,
        original.maximum,
        &mut budget.inner.observation,
        consumer,
    )
}

#[allow(clippy::too_many_arguments)]
fn with_column_source_pages<'origin, 'source, R>(
    policy: &BookV2ResourcePolicy<'_>,
    flow: &'origin PreparedBookV2TextFlow<'source>,
    admitted: &typaxis_resources::AdmittedProductionResourceLedgerV3,
    bindings: &BookV2VectorBindings<'_>,
    limits: &M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    native: Option<&BookV2NativeMath<'_>>,
    columns: &'origin BookV2ColumnFramePlan<'source>,
    maximum: u64,
    total: &mut BookV2PdfConvergenceObservation,
    consumer: impl ColumnDelivery<R>,
) -> Result<R, E> {
    let caps = limits.base().get();
    total.records = add(
        total.records,
        2,
        caps.max_fragments,
        "column cycle/header records",
    )?;
    let mut previous: Option<BookV2ColumnWidthFeedback<'origin, 'source>> = None;
    let mut cycle = WidthCycle::default();
    let mut refining = false;
    // Metadata alone survives width passes; every sibling graph is rebuilt and
    // source-bound to the current seed. Re-discovery never refunds its prefix.
    let mut header_requests = Vec::new();
    let mut consumer = Some(consumer);
    loop {
        if caps.max_layout_passes.saturating_sub(total.page_passes) < 3 {
            return Err(E::Limit("column page passes"));
        }
        let remaining_lines = caps
            .max_line_reshape_passes
            .checked_sub(total.line_passes)
            .filter(|n| *n > 0)
            .ok_or(E::Limit("column line passes"))?;
        let mut widths = Vec::new();
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        if let Some(old) = &previous {
            if !old.matches_source_flow(flow) || !std::ptr::eq(old.column_plan(), columns) {
                return Err(E::Identity);
            }
            let count = old.paragraphs().len();
            total.records = add(
                total.records,
                (count as u64)
                    .checked_mul(3)
                    .ok_or(E::Limit("column views"))?,
                caps.max_fragments,
                "column views",
            )?;
            reserve(&mut widths, count)?;
            reserve(&mut starts, count)?;
            reserve(&mut ends, count)?;
            for p in old.paragraphs() {
                total.work = add(
                    total.work,
                    2 + p.widths().len() as u64
                        + p.source_unit_starts().map_or(0, |v| v.len() as u64),
                    maximum,
                    "column widths",
                )?;
                widths.push(Some(p.widths()));
                starts.push(p.source_unit_starts());
                ends.push(p.retained_line_ends());
            }
            total.work = add(
                total.work,
                (old.block_widths().len() as u64)
                    .checked_add(old.block_starts().len() as u64)
                    .ok_or(E::Limit("column blocks"))?,
                maximum,
                "column blocks",
            )?;
        }
        let assignments = previous
            .as_ref()
            .map(|p| {
                BookV2SourceWidthAssignments::with_retained_line_ends(flow, &widths, &ends)
                    .and_then(|a| a.with_source_unit_starts(&starts))
                    .map(|a| {
                        a.with_table_occurrence_frames()
                            .with_block_widths(p.block_widths())
                            .with_block_starts(p.block_starts())
                    })
                    .map_err(|e| stage("column source widths", e))
            })
            .transpose()?;
        let mut allowance = BookV2LineVariantBudget::new(maximum - total.work, remaining_lines);
        let seed = prepare_budgeted_book_v2_column_line_variant_seed(
            policy,
            flow,
            admitted,
            bindings,
            limits,
            mode,
            columns,
            &mut allowance,
            total.records,
            native,
            assignments.as_ref(),
        );
        total.records = total.records.max(allowance.record_charge());
        total.work = add(
            total.work,
            allowance.work_steps(),
            maximum,
            "column line work",
        )?;
        total.line_passes = total
            .line_passes
            .checked_add(allowance.reshape_passes())
            .filter(|n| *n <= caps.max_line_reshape_passes)
            .ok_or(E::Limit("column line passes"))?;
        let seed = seed.map_err(|e| stage("column line convergence", e))?;
        let mut ledger = HeaderCatalogBudget {
            work: total.work,
            records: total.records,
            line_passes: total.line_passes,
            page_passes: total.page_passes,
        };
        let mut retry = None;
        let mut column_budget = ColumnHeaderBudget {
            balance_candidates: total.balance_candidates,
            spool: total.spool,
        };
        let outcome = with_discovered_column_header_catalog(
            &seed,
            &mut header_requests,
            limits,
            maximum,
            &mut ledger,
            &mut column_budget,
            |catalog, ledger, column_budget| {
                let mut records = ledger.records;
                let mut work = 0;
                let search = prepare_book_v2_column_page_search_with_headers_counted(
                    catalog,
                    limits,
                    maximum - ledger.work,
                    ledger.records,
                    &mut records,
                    &mut work,
                );
                if search.is_err() {
                    ledger.records = ledger.records.max(records);
                    ledger.work(work, maximum)?;
                }
                let mut search = search.map_err(|e| stage("column page search", e))?;
                search.retain_prior_balance_candidates(column_budget.balance_candidates);
                search
                    .retain_prior_balance_spool(column_budget.spool)
                    .map_err(|e| stage("column balance spool", e))?;
                let result = (|| {
                    let mut begun = 0;
                    let pages = search.select_repeated_pages_counted(
                        caps.max_layout_passes - ledger.page_passes,
                        &mut begun,
                    );
                    ledger.page_passes = ledger
                        .page_passes
                        .checked_add(begun)
                        .filter(|n| *n <= caps.max_layout_passes)
                        .ok_or(E::Limit("column page passes"))?;
                    let pages = pages.map_err(|e| stage("column page comparison", e))?;
                    total.balance_candidates = column_budget
                        .balance_candidates
                        .max(search.balance_candidates());
                    total.spool = total
                        .spool
                        .max(column_budget.spool)
                        .max(search.terminal_spool_charge());
                    total.width_passes = total
                        .width_passes
                        .checked_add(1)
                        .ok_or(E::Limit("column width passes"))?;
                    let mut feedback = search
                        .paragraph_frame_feedback(pages.sequence())
                        .map_err(|e| stage("column width feedback", e))?;
                    let same =
                        previous.as_ref().is_none_or(|old| {
                            old.block_widths() == feedback.block_widths()
                                && old.block_starts() == feedback.block_starts()
                                && old.paragraphs().len() == feedback.paragraphs().len()
                                && old.paragraphs().iter().zip(feedback.paragraphs()).all(
                                    |(a, b)| {
                                        a.owner() == b.owner()
                                            && a.widths() == b.widths()
                                            && a.source_unit_starts() == b.source_unit_starts()
                                            && a.uses_table_frame() == b.uses_table_frame()
                                    },
                                )
                        });
                    if !feedback.matches_selected_line_widths()
                        || !feedback.matches_selected_block_widths()
                        || !same
                    {
                        refining |= cycle.observe(feedback.assignment_fingerprint());
                        if refining {
                            search
                                .retain_paragraph_line_boundaries(pages.sequence(), &mut feedback)
                                .map_err(|e| stage("column width refinement", e))?;
                            total.width_refinements = total
                                .width_refinements
                                .checked_add(1)
                                .ok_or(E::Limit("column width refinements"))?;
                        }
                        retry = Some(
                            search
                                .capture_source_width_feedback(feedback, flow, columns)
                                .map_err(|e| stage("column width capture", e))?,
                        );
                        return Ok(None);
                    }
                    consumer
                        .take()
                        .ok_or(E::Identity)?
                        .finish(
                            &pages,
                            &mut search,
                            &feedback,
                            policy,
                            limits,
                            maximum,
                            ledger,
                            total,
                        )
                        .map(Some)
                })();
                ledger.records = ledger.records.max(search.record_charge());
                column_budget.balance_candidates = column_budget
                    .balance_candidates
                    .max(search.balance_candidates());
                column_budget.spool = column_budget.spool.max(search.terminal_spool_charge());
                ledger.work(search.work_steps(), maximum)?;
                result
            },
        );
        total.balance_candidates = total
            .balance_candidates
            .max(column_budget.balance_candidates);
        total.spool = total.spool.max(column_budget.spool);
        total.work = ledger.work;
        total.records = ledger.records;
        total.line_passes = ledger.line_passes;
        total.page_passes = ledger.page_passes;
        if let Some(value) = outcome? {
            return Ok(value);
        }
        previous = Some(retry.ok_or(E::Identity)?);
    }
}

// This existing Syntax constructor accepts the authentic body's full record
// ceiling. The guard precedes its first allocation; lower-R controls belong to
// direct Native/source/line operations, not this private context path.
fn prepare_p1_observed_column_frame_plan<'a>(
    flow: &PreparedBookV2TextFlow<'a>,
    limits: &M4EffectiveResourceLimits,
    accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<BookV2ColumnFramePlan<'a>, E> {
    accounting
        .require_limit_domain(limits.base())
        .map_err(E::ObservedAccounting)?;
    accounting
        .require_limit_domain(flow.body().body().limits())
        .map_err(E::ObservedAccounting)?;
    if accounting.maximum_records() != flow.body().body().limits().get().max_fragments {
        return Err(E::ObservedAccounting(
            typaxis_shaping::book_v2::BookV2ObservedAccountingError::LimitsMismatch,
        ));
    }
    let [prior_records, prior_spool, prior_work] = accounting.counters();
    let mut work = prior_work;
    let mut observed = BookV2ColumnFramePlanObservation::default();
    // No nested callback can borrow the loan while this concrete constructor
    // runs. Its local absolute counter starts at the actual current Work.
    let result = prepare_book_v2_column_frame_plan_counted(
        flow,
        &mut work,
        accounting.maximum_work(),
        prior_records,
        prior_spool,
        &mut observed,
    );
    let kept_work = accounting.debit_work(
        NodeId::new(0),
        work.checked_sub(prior_work).ok_or(E::Identity)?,
    );
    let kept_records = accounting.debit_records(
        NodeId::new(0),
        observed.record_charge().max(prior_records) - prior_records,
    );
    let kept_spool = accounting.debit_spool(
        NodeId::new(0),
        observed.spool_charge().max(prior_spool) - prior_spool,
    );
    // All genuine accepted deltas have already been evaluated before mapping
    // the primary constructor Result. Matching limits prove each fits the loan.
    let frames = result.map_err(E::ObservedFrames)?;
    kept_work.map_err(E::ObservedAccounting)?;
    kept_records.map_err(E::ObservedAccounting)?;
    kept_spool.map_err(E::ObservedAccounting)?;
    Ok(frames)
}

/// Readonly observed delivery; the original raw builder remains owned here
/// across the consumer, so neither replacement nor forgotten facades lose fees.
#[allow(clippy::too_many_arguments)]
fn with_observed_column_display_from_math<'g, 'q, 'b, 'f, 's, 'p, 'a, 'id, 'cells, R>(
    terminals: &BookV2ColumnMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a>,
    policy: &BookV2ResourcePolicy<'_>,
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    progress: &mut ColumnProgressView<'_, 'id, 'cells>,
    inspect: impl for<'d> FnOnce(
        &typaxis_display_list::book_v2::BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        &mut ColumnProgressView<'_, 'id, 'cells>,
    ) -> Result<R, E>,
) -> Result<R, E> {
    use typaxis_display_list::book_v2::BookV2ColumnDisplayBuilder;
    let source = terminals
        .original_graph()
        .lines()
        .prepared()
        .source_flow()
        .body();
    policy
        .verify_for(policy.body(), limits)
        .map_err(|_| E::Identity)?;
    if !std::ptr::eq(source, policy.body().styled())
        || input.resources().effective_limits() != limits
        || input.resources().profile_fingerprint() != policy.fingerprint()
    {
        return Err(E::Identity);
    }
    progress
        .accounting
        .require_limit_domain(source.body().limits())
        .map_err(E::ObservedAccounting)?;
    progress
        .accounting
        .require_limit_domain(limits.base())
        .map_err(E::ObservedAccounting)?;
    progress.require_pagination_domain(limits)?;
    progress
        .accounting
        .debit(NodeId::new(0), [0; 3])
        .map_err(E::ObservedAccounting)?;
    let current = progress.accounting.counters();
    if terminals.record_charge() > current[0]
        || terminals.spool_charge() > current[1]
        || terminals.work_steps() > current[2]
    {
        return Err(E::Identity);
    }
    let maximum = progress.accounting.maximum_work();
    let before = progress.accounting.counters();
    let (mut records, mut work) = (before[0], before[2]);
    let constructor = BookV2ColumnDisplayBuilder::new_counted(
        terminals,
        input.resources(),
        limits,
        maximum,
        records,
        work,
        &mut records,
        &mut work,
    );
    let retained = column_page_region_driver::retain_region_display_prefix(
        progress.accounting,
        before,
        records,
        work,
    );
    let mut builder = constructor.map_err(E::ObservedDisplay)?;
    retained?;
    let before = progress.accounting.counters();
    let display = builder.build();
    let retained = column_page_region_driver::retain_region_display_prefix(
        progress.accounting,
        before,
        builder.record_charge(),
        builder.work_steps(),
    );
    let display = display.map_err(E::ObservedDisplay)?;
    retained?;
    let display =
        column_page_region_driver::attach_observed(display, policy, limits, mode, progress)?;
    let before = progress.accounting.counters();
    let continued = builder.continue_after_display(&display);
    let retained = column_page_region_driver::retain_region_display_prefix(
        progress.accounting,
        before,
        builder.record_charge(),
        builder.work_steps(),
    );
    continued.map_err(E::ObservedDisplay)?;
    retained?;
    let mark = [builder.record_charge(), builder.work_steps()];
    let result = inspect(&display, progress);
    // No mutable builder escapes. Publish only a real operation's new delta;
    // descendants may already have spent Source fees on the same live cells.
    let r = builder
        .record_charge()
        .checked_sub(mark[0])
        .ok_or(E::Identity)?;
    let w = builder
        .work_steps()
        .checked_sub(mark[1])
        .ok_or(E::Identity)?;
    let accepted_r = progress
        .accounting
        .debit_records(NodeId::new(0), r)
        .map_err(E::ObservedAccounting);
    let accepted_w = progress
        .accounting
        .debit_work(NodeId::new(0), w)
        .map_err(E::ObservedAccounting);
    let value = result?;
    accepted_r?;
    accepted_w?;
    Ok(value)
}

// The real carrier and its maximum are minted together; no external arbitrary
// FontCommandSession can silently bring a different Work ceiling to this loan.
struct ObservedColumnRegionPdfCommand {
    fonts: typaxis_resources::book_v2::BookV2FontCommandSession,
    limits_fingerprint: [u8; 32],
    maximum: u64,
}
impl ObservedColumnRegionPdfCommand {
    fn new(limits: &M4EffectiveResourceLimits, maximum: u64) -> Self {
        Self {
            fonts: typaxis_resources::book_v2::BookV2FontCommandSession::new(limits, maximum),
            limits_fingerprint: limits.fingerprint(),
            maximum,
        }
    }
}
/// Closed actual downstream pipeline. Only this function holds the live parent
/// loan while ordinary private producer observations are temporarily outstanding.
/// The PDF consumer cannot spend Source or mutate any stage builder.
// Only trusted driver implementations receive the pending producer ledger.
// Application callbacks use the readonly adapter below and never see it.
trait ObservedColumnPdfFinish<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a, R> {
    fn finish<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v>(
        self,
        roots: &BookV2ColumnPdfBundle<
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
        assembly_before: [u64; 4],
        assembly_sizes: [usize; 4],
        producer: &mut BookV2PdfConvergenceObservation,
    ) -> Result<R, E>;
}
struct ReadonlyColumnPdfFinish<F>(F);
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a, R, F> ObservedColumnPdfFinish<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a, R> for ReadonlyColumnPdfFinish<F>
where F: for<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v> FnOnce(&BookV2ColumnPdfBundle<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>) -> Result<R, E> {
    fn finish<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v>(
        self, roots: &BookV2ColumnPdfBundle<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, _: [u64; 4], _: [usize; 4],
        _: &mut BookV2PdfConvergenceObservation,
    ) -> Result<R, E> { (self.0)(roots) }
}

#[allow(clippy::too_many_arguments)]
fn with_observed_column_pdf_from_display<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a, R>(
    display: &typaxis_display_list::book_v2::BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    command: &mut ObservedColumnRegionPdfCommand,
    limits: &M4EffectiveResourceLimits,
    first_font_object: u32,
    progress: &mut ColumnProgressView<'_, '_, '_>,
    inspect: impl for<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v> FnOnce(&BookV2ColumnPdfBundle<'pdf, 'nav, 'w, 'l, 'u, 'h, 'n, 'm, 'k, 'j, 'r, 'i, 't, 'o, 'z, 'y, 'x, 'c, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>) -> Result<R, E>,
) -> Result<R, E> {
    let previous = progress.snapshot();
    with_observed_column_pdf_finalized(
        display,
        command,
        limits,
        first_font_object,
        previous,
        progress,
        ReadonlyColumnPdfFinish(inspect),
    )
}

#[allow(clippy::too_many_arguments)]
fn with_observed_column_pdf_finalized<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a, R>(
    display: &typaxis_display_list::book_v2::BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    command: &mut ObservedColumnRegionPdfCommand,
    limits: &M4EffectiveResourceLimits,
    first_font_object: u32,
    previous: BookV2PdfConvergenceObservation,
    progress: &mut ColumnProgressView<'_, '_, '_>,
    finish: impl ObservedColumnPdfFinish<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a, R>,
) -> Result<R, E> {
    use typaxis_display_list::book_v2::BookV2ResourceDisplay;
    display
        .verify_resources(display.admitted(), limits)
        .map_err(|_| E::Identity)?;
    progress
        .accounting
        .require_limit_domain(limits.base())
        .map_err(E::ObservedAccounting)?;
    progress.require_pagination_domain(limits)?;
    if command.limits_fingerprint != limits.fingerprint()
        || command.maximum != progress.accounting.maximum_work()
    {
        return Err(E::Identity);
    }
    let before = progress.snapshot();
    if previous.records > before.records
        || previous.spool > before.spool
        || previous.work > before.work
        || previous.output != before.output
    {
        return Err(E::Identity);
    }

    if display.record_charge() > before.records
        || display.source_spool_charge() > before.spool
        || display.work_steps() > before.work
        || command.fonts.record_charge() > before.records
        || command.fonts.spool_charge() > before.spool
        || command.fonts.work_steps() > before.work
    {
        return Err(E::Identity);
    }
    progress
        .accounting
        .debit(NodeId::new(0), [0; 3])
        .map_err(E::ObservedAccounting)?;
    let maximum = command.maximum;
    let mut producer = before;
    let mut phase = paints_driver::BookV2ColumnOutputPhase::default();
    let outcome = paints_driver::with_column_paints_from_session(
        display,
        &mut command.fonts,
        limits,
        maximum,
        first_font_object,
        [
            previous.records,
            previous.spool,
            previous.output,
            previous.work,
        ],
        &mut producer,
        &mut phase,
        |paints, _, encoded, phase, producer| {
            marked_driver::with_column_marked_from_paints(
                paints,
                limits,
                maximum,
                encoded,
                producer,
                phase,
                |marked, _, observed, phase, producer| {
                    relations_driver::with_column_relations_from_marked(
                        marked,
                        limits,
                        maximum,
                        observed,
                        producer,
                        phase,
                        |relations, _, observed, phase, producer| {
                            navigation_driver::with_column_navigation_from_relations(
                                relations,
                                limits,
                                maximum,
                                observed,
                                producer,
                                phase,
                                |navigation, _, observed, phase, producer| {
                                    pdf_driver::with_column_pdf_from_navigation(
                                        navigation,
                                        limits,
                                        maximum,
                                        observed,
                                        previous,
                                        producer,
                                        phase,
                                        |roots, assembly, _, _, producer| {
                                            finish.finish(
                                                roots,
                                                [
                                                    assembly.record_charge(),
                                                    assembly.spool_charge(),
                                                    assembly.output_charge(),
                                                    assembly.work_steps(),
                                                ],
                                                assembly.reservation_sizes(),
                                                producer,
                                            )
                                        },
                                    )?
                                },
                            )?
                        },
                    )?
                },
            )?
        },
    )
    .and_then(|consumer| consumer);
    // Evaluate every calculation/debit independently, including on primary Err.
    // No producer here reports attempted-above-cap R/S/W; no clamp is applied.
    let accepted_r = producer
        .records
        .checked_sub(before.records)
        .ok_or(E::Identity)
        .and_then(|n| {
            progress
                .accounting
                .debit_records(NodeId::new(0), n)
                .map_err(E::ObservedAccounting)
        });
    let accepted_s = producer
        .spool
        .checked_sub(before.spool)
        .ok_or(E::Identity)
        .and_then(|n| {
            progress
                .accounting
                .debit_spool(NodeId::new(0), n)
                .map_err(E::ObservedAccounting)
        });
    let accepted_w = producer
        .work
        .checked_sub(before.work)
        .ok_or(E::Identity)
        .and_then(|n| {
            progress
                .accounting
                .debit_work(NodeId::new(0), n)
                .map_err(E::ObservedAccounting)
        });
    *progress.rest.output = phase.accepted_output().unwrap_or(producer.output);
    let result = outcome?;
    accepted_r?;
    accepted_s?;
    accepted_w?;
    Ok(result)
}
