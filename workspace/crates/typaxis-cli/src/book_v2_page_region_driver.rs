//! Build every running region at the actual selected physical page before PDF.
use super::*;
use typaxis_display_list::book_v2::{BookV2BodyDisplay, BookV2PageRegionDisplayBuilder};

#[allow(clippy::too_many_arguments)]
pub(in crate::book_v2_resources) fn attach<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    display: BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    policy: &BookV2ResourcePolicy<'_>,
    navigation: &PreparedBookV2Navigation<'_>,
    limits: &'d M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    maximum_work: u64,
    line_passes: &mut u16,
    observed_records: &mut u64,
    observed_work: &mut u64,
) -> Result<BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
    *observed_records = display.record_charge();
    *observed_work = display.work_steps();
    let source = display
        .source()
        .source()
        .flow()
        .lines()
        .prepared()
        .source_flow()
        .body();
    // Preserve the established no-region pipeline and its byte identities.
    if !source
        .body()
        .wire()
        .advanced_page_masters()
        .masters
        .iter()
        .any(|m| m.header_content.is_some() || m.footer_content.is_some())
    {
        return Ok(display);
    }
    let admitted = display.admitted();
    let epoch = display
        .source()
        .source()
        .flow()
        .lines()
        .prepared()
        .shaped()
        .binding_epoch();
    let mut builder = BookV2PageRegionDisplayBuilder::new_counted(
        source,
        admitted,
        limits,
        epoch,
        display.record_charge(),
        display.work_steps(),
        maximum_work,
        observed_records,
        observed_work,
    )
    .map_err(|e| stage("page-region display", e))?;
    let mut regions = Vec::new();
    let mut records = builder.record_charge();
    let mut work = builder.work_steps();
    let result = (|| {
        for (page_index, page) in display
            .source()
            .source()
            .geometry()
            .pages()
            .iter()
            .enumerate()
        {
            let selected = select_book_v2_page_master(
                source,
                u32::try_from(page_index).map_err(|_| E::Limit("pages"))?,
                page.selection().named_page(),
                &mut work,
                maximum_work,
            )
            .map_err(|e| stage("page-region master", e))?;
            for (kind, present) in [
                (
                    BookV2PageRegionKind::Header,
                    selected.advanced().header_content.is_some(),
                ),
                (
                    BookV2PageRegionKind::Footer,
                    selected.advanced().footer_content.is_some(),
                ),
            ] {
                if !present {
                    continue;
                }
                records = add(records, 1, limits.base().get().max_fragments, "records")?;
                regions
                    .try_reserve_exact(1)
                    .map_err(|_| E::Limit("allocation"))?;
                let prior_records = records;
                let flow = prepare_book_v2_page_region_text_flow_counted(
                    selected,
                    kind,
                    navigation,
                    prior_records,
                    &mut records,
                );
                // Carry source record visits as bounded work in addition to actual
                // line-candidate work. Full shaping-engine work remains stage-bound.
                let source_work = add(work, records - prior_records, maximum_work, "work");
                if let Ok(accepted) = source_work {
                    work = accepted;
                }
                // Recover accepted reservations before propagating the original
                // source failure. A rejected work reservation consumes no work;
                // on successful source preparation it keeps its existing error.
                let flow = flow.map_err(|e| stage("page-region source", e))?;
                source_work?;
                let remaining_passes = limits
                    .base()
                    .get()
                    .max_line_reshape_passes
                    .checked_sub(*line_passes)
                    .ok_or(E::Limit("line passes"))?;
                let mut allowance = BookV2PageRegionLineBudget::new(
                    maximum_work.checked_sub(work).ok_or(E::Limit("work"))?,
                    remaining_passes,
                );
                let mut entered = false;
                let result = with_budgeted_book_v2_page_region_lines(
                    policy,
                    &flow,
                    admitted,
                    limits,
                    epoch,
                    mode,
                    selected,
                    &mut allowance,
                    records,
                    |stable| -> Result<_, E> {
                        entered = true;
                        work = add(work, stable.candidate_steps(), maximum_work, "work")?;
                        records = stable.lines().output_records();
                        builder
                            .continue_with_prior(records, work)
                            .map_err(|e| stage("page-region budget", e))?;
                        let region = builder.build(&stable);
                        records = builder.record_charge();
                        // This builder reports attempted work above its ceiling.
                        // Exhaust the remaining allowance without masking its error
                        // with a second driver limit error or exceeding the cap.
                        work = builder.work_steps().min(maximum_work);
                        region.map_err(|e| stage("page-region display", e))
                    },
                );
                if !entered {
                    work = add(work, allowance.candidate_steps(), maximum_work, "work")?;
                    records = records.max(allowance.record_charge());
                }
                // A failed reshape still consumed a shared command pass. Preserve
                // it before propagating either line-layout or consumer errors.
                *line_passes = line_passes
                    .checked_add(allowance.reshape_passes())
                    .filter(|n| *n <= limits.base().get().max_line_reshape_passes)
                    .ok_or(E::Limit("line passes"))?;
                regions.push(result.map_err(|e| stage("page-region lines", e))??);
            }
        }
        display
            .with_page_regions_counted(
                regions,
                limits,
                maximum_work,
                records,
                work,
                &mut records,
                &mut work,
            )
            .map_err(|e| stage("page-region join", e))
    })();
    *observed_records = records;
    *observed_work = work;
    result
}
