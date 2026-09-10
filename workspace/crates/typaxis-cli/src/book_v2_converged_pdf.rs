//! Private source-to-PDF driver. A callback receives only an actually stable
//! candidate; this grants neither public-profile nor PDF/UA authority.
use super::PreparedBookV2Resources;
use typaxis_core::{M4EffectiveResourceLimits, NodeId};
use typaxis_layout::book_v2::*;
use typaxis_pagination::book_v2::*;
use typaxis_pdf::book_v2::{BookV2PdfAssembly, BookV2PdfPipeline};
use typaxis_syntax::book_v2::*;

#[derive(Debug)]
pub enum BookV2ConvergenceError {
    Limit(&'static str),
    Identity,
    UnsupportedPageMaster,
    Stage {
        stage: &'static str,
        source: Box<dyn std::error::Error>,
    },
}
impl std::fmt::Display for BookV2ConvergenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 convergence: {self:?}")
    }
}
impl std::error::Error for BookV2ConvergenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Stage { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}
use BookV2ConvergenceError as E;
#[path = "book_v2_header_catalog_driver.rs"]
pub(super) mod header_catalog_driver;
fn stage<T: std::error::Error + 'static>(stage: &'static str, source: T) -> E {
    E::Stage {
        stage,
        source: Box::new(source),
    }
}
fn add(a: u64, b: u64, ceiling: u64, name: &'static str) -> Result<u64, E> {
    a.checked_add(b)
        .filter(|n| *n <= ceiling)
        .ok_or(E::Limit(name))
}
#[derive(Default)]
struct WidthCycle {
    anchor: Option<[u8; 32]>,
    power: u32,
    distance: u32,
}
impl WidthCycle {
    fn observe(&mut self, value: [u8; 32]) -> bool {
        if self.anchor.is_none() {
            self.anchor = Some(value);
            self.power = 1;
            return false;
        }
        self.distance += 1;
        if self.anchor == Some(value) {
            return true;
        }
        if self.distance == self.power {
            self.anchor = Some(value);
            // There are at most u16::MAX page passes in this driver.
            self.power *= 2;
            self.distance = 0;
        }
        false
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BookV2PdfConvergenceObservation {
    candidates: u16,
    width_passes: u16,
    width_refinements: u16,
    line_passes: u16,
    page_passes: u16,
    records: u64,
    spool: u64,
    output: u64,
    work: u64,
}
impl BookV2PdfConvergenceObservation {
    pub fn width_refinement_passes(self) -> u16 {
        self.width_refinements
    }
    pub fn width_feedback_passes(self) -> u16 {
        self.width_passes
    }
    pub fn candidate_passes(self) -> u16 {
        self.candidates
    }
    pub fn line_reshape_passes(self) -> u16 {
        self.line_passes
    }
    pub fn page_passes(self) -> u16 {
        self.page_passes
    }
    pub fn record_charge(self) -> u64 {
        self.records
    }
    pub fn spool_charge(self) -> u64 {
        self.spool
    }
    pub fn output_charge(self) -> u64 {
        self.output
    }
    pub fn work_steps(self) -> u64 {
        self.work
    }
}
/// Reuse the admitted originals, vector bindings and native computations across
/// actual label -> shaping -> line -> page -> display -> PDF passes. All page,
/// reshape and downstream record/spool/output/work ceilings span the loop.
/// Source admission and temporary shaping storage retain their own stage caps.
/// With Page references, two consecutive matching complete PDFs are required.
/// Without references, the actual two-pass physical-page proof suffices.
pub fn with_converged_book_v2_pdf<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    japanese_mode: typaxis_linebreak::JapaneseLineBreakMode,
    maximum_work: u64,
    inspect: impl FnOnce(
        &BookV2PdfAssembly<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    let caps = limits.base().get();
    let body = input.body();
    let navigation =
        prepare_book_v2_navigation(body.styled()).map_err(|e| stage("navigation", e))?;
    let policy = prepare_book_v2_resource_policy(body, limits).map_err(|e| stage("policy", e))?;
    let admitted = input.resources();
    let bindings =
        bind_book_v2_vectors(&policy, admitted, limits).map_err(|e| stage("vectors", e))?;
    let native = compute_book_v2_native_math(&bindings, admitted, limits, 0, 0)
        .map_err(|e| stage("native math", e))?;
    let mut total = BookV2PdfConvergenceObservation {
        work: add(
            native.as_ref().map_or(0, |n| n.layout_work()),
            0,
            maximum_work,
            "work",
        )?,
        records: native.as_ref().map_or(0, |n| n.record_charge()),
        spool: native.as_ref().map_or(0, |n| n.spool_charge()),
        ..Default::default()
    };
    let mut values = Vec::new();
    let page_plan;
    {
        // Collect every authored Page reference, including unplaced definitions.
        let source = prepare_book_v2_text_flow(body.styled(), &navigation)
            .map_err(|e| stage("source", e))?;
        page_plan = prepare_book_v2_page_frame_plan_for_reflow(
            &source,
            &mut total.work,
            maximum_work,
            total.records,
            total.spool,
        )
        .map_err(|e| {
            if e == BookV2PageMasterError::UnsupportedAdvanced {
                E::UnsupportedPageMaster
            } else {
                stage("page frames", e)
            }
        })?;
        total.records = add(
            total.records,
            page_plan.record_charge(),
            caps.max_fragments,
            "records",
        )?;
        total.spool = add(
            total.spool,
            page_plan.spool_charge(),
            caps.max_spool_bytes,
            "spool",
        )?;
        for p in source.paragraphs() {
            total.work = add(total.work, 1, maximum_work, "work")?;
            for site in p.items() {
                total.work = add(total.work, 1, maximum_work, "work")?;
                if matches!(
                    site.reference(),
                    Some(typaxis_syntax::ProductionInlineReference::Anchor {
                        format: typaxis_syntax::ProductionReferenceFormat::Page,
                        ..
                    })
                ) {
                    // Reserve both feedback buffers before allocating either.
                    total.records = add(total.records, 2, caps.max_fragments, "records")?;
                    total.spool = add(
                        total.spool,
                        2 * std::mem::size_of::<(NodeId, u32)>() as u64,
                        caps.max_spool_bytes,
                        "spool",
                    )?;
                    values.try_reserve(1).map_err(|_| E::Limit("allocation"))?;
                    values.push((site.owner(), 1u32));
                }
            }
        }
    }
    let frame = page_plan.measurement_body();
    // Unstable sorting has no hidden heap allocation; charge its comparison bound.
    if !values.is_empty() {
        let sort_work = (values.len() as u64)
            .checked_mul(u64::from(values.len().ilog2()) + 1)
            .ok_or(E::Limit("work"))?;
        total.work = add(total.work, sort_work, maximum_work, "work")?;
    }
    values.sort_unstable_by_key(|v| v.0);
    if values.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(E::Identity);
    }
    let mut next = Vec::new();
    next.try_reserve_exact(values.len())
        .map_err(|_| E::Limit("allocation"))?;
    let mut previous = None;
    let mut width_feedback: Option<BookV2ParagraphWidthFeedback> = None;
    // Brent's cycle detection retains one digest, independently of source size.
    let mut width_cycle = WidthCycle::default();
    let mut refining_widths = false;
    let mut header_mode = false;
    if page_plan.requires_width_reflow() {
        total.records = add(total.records, 1, caps.max_fragments, "records")?;
    }
    let mut inspect = Some(inspect);
    loop {
        if caps.max_layout_passes.saturating_sub(total.page_passes) < 2 {
            return Err(E::Limit("page passes"));
        }
        let reshape_left = caps
            .max_line_reshape_passes
            .checked_sub(total.line_passes)
            .filter(|n| *n > 0)
            .ok_or(E::Limit("line passes"))?;
        let flow =
            prepare_book_v2_text_flow_with_page_references(body.styled(), &navigation, &values)
                .map_err(|e| stage("source labels", e))?;
        if width_feedback
            .as_ref()
            .is_some_and(|f| !f.matches_source_flow(&flow))
        {
            width_feedback = None;
            width_cycle = WidthCycle::default();
            refining_widths = false;
        }
        let mut width_views = Vec::new();
        let mut end_views = Vec::new();
        let mut start_views = Vec::new();
        if page_plan.requires_width_reflow() {
            total.work = add(total.work, 1, maximum_work, "work")?;
        }
        if let Some(feedback) = &width_feedback {
            total.records = add(
                total.records,
                (feedback.paragraphs().len() as u64)
                    .checked_mul(if feedback.uses_table_occurrence_frames() {
                        3
                    } else {
                        2
                    })
                    .ok_or(E::Limit("records"))?,
                caps.max_fragments,
                "records",
            )?;
            width_views
                .try_reserve_exact(feedback.paragraphs().len())
                .map_err(|_| E::Limit("allocation"))?;
            end_views
                .try_reserve_exact(feedback.paragraphs().len())
                .map_err(|_| E::Limit("allocation"))?;
            if feedback.uses_table_occurrence_frames() {
                start_views
                    .try_reserve_exact(feedback.paragraphs().len())
                    .map_err(|_| E::Limit("allocation"))?;
            }
            for paragraph in feedback.paragraphs() {
                // Reserve the full later candidate comparison before page work.
                total.work = add(
                    total.work,
                    2 + paragraph.widths().len() as u64
                        + paragraph.source_unit_starts().map_or(0, |v| v.len() as u64),
                    maximum_work,
                    "work",
                )?;
                width_views.push((!paragraph.uses_table_frame()).then(|| paragraph.widths()));
                end_views.push(paragraph.retained_line_ends());
                if feedback.uses_table_occurrence_frames() {
                    start_views.push(paragraph.source_unit_starts());
                }
            }
            total.work = add(
                total.work,
                (feedback.block_widths().len() as u64)
                    .checked_add(feedback.root_table_widths().len() as u64)
                    .and_then(|n| {
                        n.checked_add(feedback.block_starts().map_or(0, |v| v.len() as u64))
                    })
                    .ok_or(E::Limit("work"))?,
                maximum_work,
                "work",
            )?;
        }
        let mut assignments = if width_feedback.is_some() {
            Some(
                BookV2SourceWidthAssignments::with_retained_line_ends(
                    &flow,
                    &width_views,
                    &end_views,
                )
                .map_err(|e| stage("source widths", e))?
                .with_block_widths(width_feedback.as_ref().unwrap().block_widths())
                .with_root_table_widths(width_feedback.as_ref().unwrap().root_table_widths()),
            )
        } else {
            None
        };
        if let Some(feedback) = &width_feedback {
            assignments = Some(if let Some(starts) = feedback.block_starts() {
                assignments.unwrap().with_block_starts(starts)
            } else {
                assignments.unwrap().with_inherited_table_blocks()
            });
        }
        if width_feedback
            .as_ref()
            .is_some_and(|f| f.uses_table_occurrence_frames())
        {
            assignments = Some(
                assignments
                    .unwrap()
                    .with_source_unit_starts(&start_views)
                    .map_err(|e| stage("source origins", e))?
                    .with_table_occurrence_frames(),
            );
        }
        let remaining_work = maximum_work
            .checked_sub(total.work)
            .ok_or(E::Limit("work"))?;
        let mut retry_widths = None;
        next.clear();
        let headers_enabled = header_mode;
        let mut retry_headers = false;
        let mut finish = |mut search: BookV2FootnoteDemandSearch<'_, '_, '_, '_, '_>,
                          total: &mut BookV2PdfConvergenceObservation,
                          remaining: u64|
         -> Result<Option<R>, E> {
            let stable = search
                .select_stable_mixed_pages(caps.max_layout_passes - total.page_passes)
                .map_err(|e| stage("page stability", e))?;
            total.page_passes = total
                .page_passes
                .checked_add(stable.passes())
                .filter(|n| *n <= caps.max_layout_passes)
                .ok_or(E::Limit("page passes"))?;
            let placed = search
                .place_mixed_pages(stable.sequence())
                .map_err(|e| stage("placement", e))?;
            let closure = search
                .close_mixed_page_sources(&stable, &placed)
                .map_err(|e| stage("source closure", e))?;
            if page_plan.requires_width_reflow() {
                total.width_passes = total
                    .width_passes
                    .checked_add(1)
                    .ok_or(E::Limit("width passes"))?;
                let feedback_result = if closure.has_header_variants()
                    || width_feedback
                        .as_ref()
                        .is_some_and(|f| f.uses_table_occurrence_frames())
                {
                    search.paragraph_frame_feedback(&closure)
                } else {
                    match search.paragraph_width_feedback(&closure) {
                            Err(e) if e.kind == typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion("table_continuation_width_reflow") => search.paragraph_frame_feedback(&closure),
                            result => result,
                        }
                };
                let mut feedback = match feedback_result {
                        Err(e) if !headers_enabled && e.kind == typaxis_pagination::ProductionBodyPaginationErrorKind::PendingRegion("table_repeated_frame_reflow") => {
                            total.records = search.record_charge();
                            total.work = add(total.work, search.work_steps(), maximum_work, "work")?;
                            header_mode = true;
                            retry_headers = true;
                            return Ok(None);
                        }
                        result => result.map_err(|e|stage("page widths",e))?,
                    };
                let same_assignments = width_feedback.as_ref().is_none_or(|old| {
                    old.uses_table_occurrence_frames() == feedback.uses_table_occurrence_frames()
                        && old.root_table_widths() == feedback.root_table_widths()
                        && old.block_widths() == feedback.block_widths()
                        && old.block_starts() == feedback.block_starts()
                        && old.paragraphs().len() == feedback.paragraphs().len()
                        && old
                            .paragraphs()
                            .iter()
                            .zip(feedback.paragraphs())
                            .all(|(a, b)| {
                                a.owner() == b.owner()
                                    && a.widths() == b.widths()
                                    && a.source_unit_starts() == b.source_unit_starts()
                                    && a.uses_table_frame() == b.uses_table_frame()
                            })
                });
                if !feedback.matches_selected_line_widths()
                    || !feedback.matches_selected_block_widths()
                    || !feedback.matches_selected_table_widths()
                    || !same_assignments
                {
                    refining_widths |= width_cycle.observe(feedback.assignment_fingerprint());
                    if refining_widths {
                        search
                            .retain_paragraph_line_boundaries(&closure, &mut feedback)
                            .map_err(|e| stage("width refinement", e))?;
                        total.width_refinements = total
                            .width_refinements
                            .checked_add(1)
                            .ok_or(E::Limit("width refinements"))?;
                    }
                    total.records = feedback.record_charge();
                    total.work = add(total.work, feedback.work_steps(), maximum_work, "work")?;
                    retry_widths = Some(feedback);
                    return Ok(None);
                }
            }
            let terminals = search
                .finalize_mixed_page_math(closure, limits, total.spool)
                .map_err(|e| stage("math terminals", e))?;
            let mut display = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                &terminals,
                admitted,
                limits,
                remaining,
                terminals.record_charge(),
                terminals.work_steps(),
            )
            .map_err(|e| stage("display", e))?;
            let display = display.build_body().map_err(|e| stage("body display", e))?;
            let mut pipeline = BookV2PdfPipeline::new(
                &display,
                1,
                limits,
                remaining,
                display.record_charge(),
                terminals.spool_charge(),
                total.output,
                display.work_steps(),
            )
            .map_err(|e| stage("PDF pipeline", e))?;
            pipeline
                .with_pdf(|pdf| -> Result<Option<R>, E> {
                    total.records = pdf.record_charge();
                    total.spool = pdf.spool_charge();
                    total.output = pdf.output_charge();
                    total.work = add(total.work, pdf.work_steps(), maximum_work, "work")?;
                    total.candidates = total
                        .candidates
                        .checked_add(1)
                        .ok_or(E::Limit("candidates"))?;
                    if pdf.page_references().len() != values.len() {
                        return Err(E::Identity);
                    }
                    for (old, observed) in values.iter().zip(pdf.page_references()) {
                        total.work = add(total.work, 1, maximum_work, "work")?;
                        if observed.owner() != old.0 || observed.candidate_page() != old.1 {
                            return Err(E::Identity);
                        }
                        next.push((old.0, observed.target_page().unwrap_or(old.1)));
                    }
                    let state = (display.fingerprint(), pdf.fingerprint());
                    let matching = pdf.page_reference_labels_match();
                    if matching && (values.is_empty() || previous == Some(state)) {
                        return Ok(Some(inspect.take().ok_or(E::Identity)?(pdf, *total)));
                    }
                    previous = matching.then_some(state);
                    Ok(None)
                })
                .map_err(|e| stage("PDF", e))?
        };
        let result = if headers_enabled {
            use header_catalog_driver::{with_discovered_header_catalog, HeaderCatalogBudget};
            let seed = prepare_book_v2_body_line_variant_seed(
                &policy,
                &flow,
                admitted,
                &bindings,
                limits,
                japanese_mode,
                frame,
                remaining_work,
                total.records,
                native.as_ref(),
                reshape_left,
                Some(&page_plan),
                assignments.as_ref(),
            )
            .map_err(|e| stage("header base convergence", e))?;
            total.work = add(total.work, seed.work_steps(), maximum_work, "work")?;
            total.records = seed.record_charge();
            total.line_passes = total
                .line_passes
                .checked_add(seed.reshape_passes())
                .filter(|n| *n <= caps.max_line_reshape_passes)
                .ok_or(E::Limit("line passes"))?;
            let mut budget = HeaderCatalogBudget {
                work: total.work,
                records: total.records,
                line_passes: total.line_passes,
                page_passes: total.page_passes,
            };
            let result = with_discovered_header_catalog(
                &seed,
                limits,
                maximum_work,
                &mut budget,
                |catalog, budget| {
                    total.work = budget.work;
                    total.records = budget.records;
                    total.line_passes = budget.line_passes;
                    total.page_passes = budget.page_passes;
                    let remaining = maximum_work - budget.work;
                    let search = prepare_book_v2_table_body_search_with_headers(
                        catalog,
                        limits,
                        remaining,
                        budget.records,
                    )
                    .map_err(|e| stage("page headers", e))?;
                    let result = finish(search, &mut total, remaining);
                    budget.work = total.work;
                    budget.records = total.records;
                    budget.line_passes = total.line_passes;
                    budget.page_passes = total.page_passes;
                    result
                },
            );
            total.work = budget.work;
            total.records = budget.records;
            total.line_passes = budget.line_passes;
            total.page_passes = budget.page_passes;
            result?
        } else {
            with_converged_book_v2_body_lines_with_source_widths(
                &policy,
                &flow,
                admitted,
                &bindings,
                limits,
                japanese_mode,
                frame,
                remaining_work,
                native.as_ref(),
                reshape_left,
                Some(&page_plan),
                assignments.as_ref(),
                |lines| -> Result<Option<R>, E> {
                    let line_passes =
                        u16::try_from(lines.passes().len()).map_err(|_| E::Limit("line passes"))?;
                    total.line_passes = total
                        .line_passes
                        .checked_add(line_passes)
                        .filter(|n| *n <= caps.max_line_reshape_passes)
                        .ok_or(E::Limit("line passes"))?;
                    total.work = add(total.work, lines.candidate_steps(), maximum_work, "work")?;
                    let remaining = maximum_work - total.work;
                    let records = add(
                        total.records,
                        lines.footnotes().record_charge(),
                        caps.max_fragments,
                        "records",
                    )?;
                    let numbers = typaxis_shaping::book_v2::shape_book_v2_equation_numbers(
                        lines.lines().prepared().shaped(),
                        limits,
                        records,
                    )
                    .map_err(|e| stage("equation labels", e))?;
                    let blocks = prepare_book_v2_vector_blocks(
                        lines.lines(),
                        numbers.as_ref(),
                        limits,
                        records,
                    )
                    .map_err(|e| stage("block layout", e))?;
                    let body_flow = prepare_book_v2_body_flow(
                        lines.lines(),
                        blocks.as_ref(),
                        lines.footnotes(),
                        limits,
                        records,
                    )
                    .map_err(|e| stage("body flow", e))?;
                    let measured = prepare_book_v2_table_measurements(body_flow, limits)
                        .map_err(|e| stage("tables", e))?;
                    let search = prepare_book_v2_table_body_search(
                        &measured,
                        limits,
                        remaining,
                        measured.record_charge(),
                    )
                    .map_err(|e| stage("page search", e))?;
                    finish(search, &mut total, remaining)
                },
            )
            .map_err(|e| stage("line feedback", e))??
        };
        if retry_headers {
            previous = None;
            continue;
        }
        if let Some(result) = result {
            return Ok(result);
        }
        if let Some(feedback) = retry_widths {
            width_feedback = Some(feedback);
            previous = None;
            continue;
        }
        std::mem::swap(&mut values, &mut next);
    }
}

#[cfg(test)]
mod cycle_tests {
    use super::WidthCycle;
    #[test]
    fn book_v2_variable_page_widths_detect_cycles_with_bounded_state() {
        for period in 1..=17u8 {
            let mut cycle = WidthCycle::default();
            for value in 100..105u8 {
                assert!(!cycle.observe([value; 32]));
            }
            assert!((0..80).any(|i| cycle.observe([(i % usize::from(period)) as u8; 32])));
        }
        let mut cycle = WidthCycle::default();
        for value in 0..=255 {
            assert!(!cycle.observe([value; 32]));
        }
    }
}
