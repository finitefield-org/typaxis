//! Attach original running regions to the selected Column display before fonts.
use super::*;
use typaxis_display_list::book_v2::{BookV2ColumnDisplay, BookV2PageRegionDisplayBuilder};

#[allow(clippy::too_many_arguments)]
pub(in crate::book_v2_resources) fn attach<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    display: BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    policy: &BookV2ResourcePolicy<'_>,
    limits: &'d M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    maximum_work: u64,
    line_passes: &mut u16,
    observed_records: &mut u64,
    observed_work: &mut u64,
) -> Result<BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
    *observed_records = display.record_charge();
    *observed_work = display.work_steps();
    let original = display.source().original_graph().lines().prepared();
    let flow = original.source_flow();
    let source = flow.body();
    let navigation = flow.navigation();
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
    let epoch = original.shaped().binding_epoch();
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
    .map_err(|e| stage("column page-region display", e))?;
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
            .map_err(|e| stage("column page-region master", e))?;
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
                let flow = flow.map_err(|e| stage("column page-region source", e))?;
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
                        records = records.max(stable.lines().output_records());
                        work = add(work, stable.candidate_steps(), maximum_work, "work")?;
                        builder
                            .continue_with_prior(records, work)
                            .map_err(|e| stage("column page-region budget", e))?;
                        let region = builder.build(&stable);
                        records = builder.record_charge();
                        // This builder reports attempted work above its ceiling.
                        // Exhaust the remaining allowance without masking its error
                        // with a second driver limit error or exceeding the cap.
                        work = builder.work_steps().min(maximum_work);
                        region.map_err(|e| stage("column page-region display", e))
                    },
                );
                let candidate_work = if !entered {
                    records = records.max(allowance.record_charge());
                    // The allowance started with maximum_work - work, so its
                    // accepted candidate steps fit the same command ceiling.
                    // Retain records and begun passes before propagating even
                    // an unexpected violation of that arithmetic invariant.
                    let accepted = add(work, allowance.candidate_steps(), maximum_work, "work");
                    if let Ok(next) = accepted {
                        work = next;
                    }
                    accepted.map(|_| ())
                } else {
                    Ok(())
                };
                // A failed reshape still consumed a shared command pass. Preserve
                // it before propagating either line-layout or consumer errors.
                *line_passes = line_passes
                    .checked_add(allowance.reshape_passes())
                    .filter(|n| *n <= limits.base().get().max_line_reshape_passes)
                    .ok_or(E::Limit("line passes"))?;
                candidate_work?;
                regions.push(result.map_err(|e| stage("column page-region lines", e))??);
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
            .map_err(|e| stage("column page-region join", e))
    })();
    *observed_records = records;
    *observed_work = work;
    result
}

// This closed import is used only with original concrete display producers.
// Both independently accepted axes are published before the first error escapes.
pub(super) fn retain_region_display_prefix(
    accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
    before: [u64; 3],
    records: u64,
    work: u64,
) -> Result<(), E> {
    let r = records.checked_sub(before[0]).ok_or(E::Identity)?;
    let w = work.checked_sub(before[2]).ok_or(E::Identity)?;
    let accepted_r = accounting
        .debit_records(NodeId::new(0), r)
        .map_err(E::ObservedAccounting);
    let accepted_w = accounting
        .debit_work(NodeId::new(0), w)
        .map_err(E::ObservedAccounting);
    accepted_r?;
    accepted_w
}
/// Original-region delivery on the same live Source loan. The original public
/// attach path above intentionally retains its existing surrogate accounting.
#[allow(clippy::too_many_arguments)]
pub(super) fn attach_observed<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    display: BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    policy: &BookV2ResourcePolicy<'_>,
    limits: &'d M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    progress: &mut ColumnProgressView<'_, '_, '_>,
) -> Result<BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
    let original = display.source().original_graph().lines().prepared();
    let source = original.source_flow().body();
    let navigation = original.source_flow().navigation();
    let admitted = display.admitted();
    let epoch = original.shaped().binding_epoch();
    // Authentic identity/policy affiliation precedes all accounting checks.
    policy
        .verify_for(policy.body(), limits)
        .map_err(|_| E::Identity)?;
    if !std::ptr::eq(source, policy.body().styled())
        || epoch == [0; 32]
        || admitted.profile_fingerprint() != policy.fingerprint()
        || admitted.effective_limits() != limits
        || !admitted.matches_declared_resources(source.body().resources())
    {
        return Err(E::Identity);
    }
    display
        .verify_resources(admitted, limits)
        .map_err(|_| E::Identity)?;
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
    // This exact Display's historical prefixes must already belong to this
    // caller. Joining may consume new fees, never admit an unpaid ancestor.
    use typaxis_display_list::book_v2::BookV2ResourceDisplay;
    let current = progress.accounting.counters();
    if display.record_charge() > current[0]
        || display.source_spool_charge() > current[1]
        || display.work_steps() > current[2]
    {
        return Err(E::Identity);
    }
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
    let maximum = progress.accounting.maximum_work();
    let before = progress.accounting.counters();
    let (mut records, mut work) = (before[0], before[2]);
    let constructor = BookV2PageRegionDisplayBuilder::new_counted(
        source,
        admitted,
        limits,
        epoch,
        records,
        work,
        maximum,
        &mut records,
        &mut work,
    );
    let retained = retain_region_display_prefix(progress.accounting, before, records, work);
    let mut builder = constructor.map_err(E::ObservedDisplay)?;
    retained?;
    let mut regions = Vec::new();
    for (page_index, page) in display
        .source()
        .source()
        .geometry()
        .pages()
        .iter()
        .enumerate()
    {
        let before = progress.accounting.counters();
        let mut work = before[2];
        let selected = select_book_v2_page_master(
            source,
            u32::try_from(page_index).map_err(|_| E::Limit("pages"))?,
            page.selection().named_page(),
            &mut work,
            maximum,
        );
        let retained = retain_region_display_prefix(progress.accounting, before, before[0], work);
        let selected = selected.map_err(E::ObservedFrames)?;
        retained?;
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
            // Declared logical full backing plan for the following list reserve;
            // no claim that an allocator returns exactly this capacity.
            progress
                .accounting
                .debit_work(NodeId::new(0), 1)
                .map_err(E::ObservedAccounting)?;
            let target = regions.len().checked_add(1).ok_or(E::Limit("records"))?;
            let bytes = target
                .checked_mul(std::mem::size_of::<
                    typaxis_display_list::book_v2::BookV2PageRegionDisplay<'d>,
                >())
                .ok_or(E::Limit("spool"))?;
            progress
                .accounting
                .debit(NodeId::new(0), [1, bytes as u64, target as u64 + 1])
                .map_err(E::ObservedAccounting)?;
            regions
                .try_reserve_exact(1)
                .map_err(|_| E::Limit("allocation"))?;
            let flow = progress
                .accounting
                .prepare_region_source(selected, kind, navigation)
                .map_err(E::from)?;
            let remaining_passes = limits
                .base()
                .get()
                .max_line_reshape_passes
                .checked_sub(*progress.rest.line_passes)
                .ok_or(E::Limit("line passes"))?;
            let mut allowance = BookV2ObservedPageRegionLineBudget::new(
                progress.accounting.remaining_work(),
                remaining_passes,
            );
            let result = with_observed_book_v2_page_region_lines(
                policy,
                &flow,
                admitted,
                limits,
                epoch,
                mode,
                selected,
                &mut allowance,
                progress.accounting,
                |stable, accounting| {
                    let before = accounting.counters();
                    // The real builder imports current Source/line fees first; only
                    // its subsequent positive delta belongs to this operation.
                    let continued = builder.continue_with_prior(before[0], before[2]);
                    let retained = retain_region_display_prefix(
                        accounting,
                        before,
                        builder.record_charge(),
                        builder.work_steps().min(maximum),
                    );
                    continued.map_err(E::ObservedDisplay)?;
                    retained?;
                    let before = accounting.counters();
                    let region = builder.build(&stable);
                    // This particular builder reports attempted Work above its cap.
                    let retained = retain_region_display_prefix(
                        accounting,
                        before,
                        builder.record_charge(),
                        builder.work_steps().min(maximum),
                    );
                    let region = region.map_err(E::ObservedDisplay)?;
                    retained?;
                    Ok::<_, E>(region)
                },
            );
            let passes = progress
                .rest
                .line_passes
                .checked_add(allowance.reshape_passes())
                .filter(|n| *n <= limits.base().get().max_line_reshape_passes)
                .ok_or(E::Limit("line passes"));
            if let Ok(n) = passes {
                *progress.rest.line_passes = n;
            }
            let region = result.map_err(E::from)??;
            passes?;
            regions.push(region);
        }
    }
    let before = progress.accounting.counters();
    let (mut records, mut work) = (before[0], before[2]);
    let joined = display.with_page_regions_counted(
        regions,
        limits,
        maximum,
        records,
        work,
        &mut records,
        &mut work,
    );
    let retained = retain_region_display_prefix(progress.accounting, before, records, work);
    let joined = joined.map_err(E::ObservedDisplay)?;
    retained?;
    Ok(joined)
}
