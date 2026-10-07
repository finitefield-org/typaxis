//! Automatic source-width/column-page feedback with one retained caller ledger.
//! Repeated source selections are not yet balanced placement or PDF receipts.
use super::column_catalog_driver::with_discovered_column_header_catalog;
use super::header_catalog_driver::{reserve, HeaderCatalogBudget};
use super::*;

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

/// Keep the exact original resources and plan across source-unit reshaping,
/// actual header-width discovery and repeated joint column/note selection.
pub fn with_budgeted_book_v2_column_pages<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl FnOnce(
        &BookV2ColumnRepeatedPages<'_, '_, '_, '_, '_>,
        &mut BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
        &BookV2ColumnWidthFeedback<'_, '_>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    if budget.inner.limits_fingerprint != limits.fingerprint() {
        return Err(E::Identity);
    }
    let maximum = budget.inner.maximum_work;
    let total = &mut budget.inner.observation;
    let body = input.body();
    let navigation = prepare_reserved_book_v2_navigation(body.styled(), limits, total)?;
    let policy =
        prepare_book_v2_resource_policy(body, limits).map_err(|e| stage("column policy", e))?;
    let bindings = bind_book_v2_vectors(&policy, input.resources(), limits)
        .map_err(|e| stage("column vectors", e))?;
    let native = with_reserved_book_v2_native_math(
        &bindings,
        input.resources(),
        limits,
        maximum,
        total,
        |plan| {
            plan.map(|plan| compute_preflighted_book_v2_native_math(plan, limits))
                .transpose()
        },
    )?;
    let mut records = total.records;
    let flow = prepare_book_v2_text_flow_counted(
        body.styled(),
        &navigation,
        total.records,
        limits.base().get().max_fragments,
        &mut records,
    );
    total.records = total.records.max(records);
    let flow = flow.map_err(|e| stage("column source", e))?;
    let mut observed = BookV2ColumnFramePlanObservation::default();
    let columns = prepare_book_v2_column_frame_plan_counted(
        &flow,
        &mut total.work,
        maximum,
        total.records,
        total.spool,
        &mut observed,
    );
    total.records = total.records.max(observed.record_charge());
    total.spool = total.spool.max(observed.spool_charge());
    let columns = columns.map_err(|e| stage("column plan", e))?;
    with_column_source_pages(
        &policy,
        &flow,
        input.resources(),
        &bindings,
        limits,
        japanese_mode,
        native.as_ref(),
        &columns,
        maximum,
        total,
        inspect,
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
    inspect: impl FnOnce(
        &BookV2ColumnRepeatedPages<'_, '_, '_, '_, '_>,
        &mut BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
        &BookV2ColumnWidthFeedback<'_, '_>,
        BookV2PdfConvergenceObservation,
    ) -> R,
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
    let mut inspect = Some(inspect);
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
        let outcome = with_discovered_column_header_catalog(
            &seed,
            &mut header_requests,
            limits,
            maximum,
            &mut ledger,
            |catalog, ledger| {
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
                    let observation = BookV2PdfConvergenceObservation {
                        candidates: total
                            .candidates
                            .checked_add(1)
                            .ok_or(E::Limit("column candidates"))?,
                        records: search.record_charge(),
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
                    Ok(Some(inspect.take().ok_or(E::Identity)?(
                        &pages,
                        &mut search,
                        &feedback,
                        observation,
                    )))
                })();
                ledger.records = ledger.records.max(search.record_charge());
                ledger.work(search.work_steps(), maximum)?;
                result
            },
        );
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
