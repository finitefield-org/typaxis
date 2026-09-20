//! Build every running region at the actual selected physical page before PDF.
use super::*;
use typaxis_display_list::book_v2::{BookV2BodyDisplay, BookV2PageRegionDisplayBuilder};

#[allow(clippy::too_many_arguments)]
pub(super) fn attach<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    display: BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    policy: &BookV2ResourcePolicy<'_>,
    navigation: &PreparedBookV2Navigation<'_>,
    limits: &'d M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    maximum_work: u64,
    line_passes: &mut u16,
) -> Result<BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
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
    let mut builder = BookV2PageRegionDisplayBuilder::new(
        source,
        admitted,
        limits,
        epoch,
        display.record_charge(),
        display.work_steps(),
        maximum_work,
    )
    .map_err(|e| stage("page-region display", e))?;
    let mut regions = Vec::new();
    let mut records = builder.record_charge();
    let mut work = builder.work_steps();
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
            let flow = prepare_book_v2_page_region_text_flow(selected, kind, navigation, records)
                .map_err(|e| stage("page-region source", e))?;
            // Carry source record visits as bounded work in addition to actual
            // line-candidate work. Full shaping-engine work remains stage-bound.
            work = add(work, flow.record_charge() - records, maximum_work, "work")?;
            records = flow.record_charge();
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
                    work = add(work, stable.candidate_steps(), maximum_work, "work")?;
                    records = stable.lines().output_records();
                    builder
                        .continue_with_prior(records, work)
                        .map_err(|e| stage("page-region budget", e))?;
                    let region = builder
                        .build(&stable)
                        .map_err(|e| stage("page-region display", e))?;
                    records = builder.record_charge();
                    work = builder.work_steps();
                    Ok(region)
                },
            );
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
        .with_page_regions(regions, limits, maximum_work, records, work)
        .map_err(|e| stage("page-region join", e))
}
