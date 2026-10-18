//! Private immutable roots shared by generated Page-label candidates.
//! These source candidates grant no placement, Body-flow or PDF authority.
use super::*;
use typaxis_document_package::{WirePageRegionBlock, WirePageRegionInline};

#[cfg(test)]
#[path = "book_v2_column_source_context_tests.rs"]
mod tests;

pub(super) struct ColumnOriginalContext<'a> {
    pub(super) input: &'a PreparedBookV2Resources,
    pub(super) limits: &'a M4EffectiveResourceLimits,
    pub(super) mode: typaxis_linebreak::JapaneseLineBreakMode,
    navigation: &'a PreparedBookV2Navigation<'a>,
    pub(super) policy: &'a BookV2ResourcePolicy<'a>,
    pub(super) bindings: &'a BookV2VectorBindings<'a>,
    pub(super) native: Option<&'a BookV2NativeMath<'a>>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    pub(super) columns: &'a BookV2ColumnFramePlan<'a>,
    pub(super) maximum: u64,
    prepared_progress: BookV2PdfConvergenceObservation,
}

pub(super) struct ColumnCandidateContext<'a> {
    pub(super) original: &'a ColumnOriginalContext<'a>,
    pub(super) flow: &'a PreparedBookV2TextFlow<'a>,
}

pub(super) struct ColumnPageReferenceCensus<'a> {
    source: &'a PreparedBookV2TextFlow<'a>,
    owners: Vec<NodeId>,
    regions: Vec<ColumnMasterRegionCensus<'a>>,
    progress: BookV2PdfConvergenceObservation,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ColumnMasterRegionCensus<'a> {
    pub(super) master_index: usize,
    pub(super) master_id: &'a str,
    pub(super) kind: BookV2PageRegionKind,
    pub(super) owner: Option<NodeId>,
    pub(super) blocks: usize,
    pub(super) inlines: usize,
}

fn prefix(total: &BookV2PdfConvergenceObservation) -> [u64; 3] {
    [total.records, total.spool, total.work]
}

fn retains(
    total: &BookV2PdfConvergenceObservation,
    earlier: &BookV2PdfConvergenceObservation,
) -> bool {
    // Output belongs to the separate signed typed settlement phase. Every
    // accepted source/work/spool and begun layout counter is monotone here.
    prefix(total)
        .iter()
        .zip(prefix(earlier))
        .all(|(a, b)| *a >= b)
        && total.candidates >= earlier.candidates
        && total.balance_candidates >= earlier.balance_candidates
        && total.width_passes >= earlier.width_passes
        && total.width_refinements >= earlier.width_refinements
        && total.line_passes >= earlier.line_passes
        && total.page_passes >= earlier.page_passes
}

impl ColumnCandidateContext<'_> {
    pub(super) fn verify_budget(&self, budget: &BookV2ColumnPageBudget) -> Result<(), E> {
        self.original.verify_budget(budget)
    }
}

impl<'a> ColumnOriginalContext<'a> {
    fn verify_budget(&self, budget: &BookV2ColumnPageBudget) -> Result<(), E> {
        if budget.inner.limits_fingerprint != self.limits.fingerprint()
            || budget.inner.maximum_work != self.maximum
            || !retains(&budget.inner.observation, &self.prepared_progress)
        {
            return Err(E::Identity);
        }
        Ok(())
    }

    pub(super) fn original_candidate(&self) -> ColumnCandidateContext<'_> {
        ColumnCandidateContext {
            original: self,
            flow: self.flow,
        }
    }

    pub(super) fn with_page_references<R>(
        &self,
        census: &ColumnPageReferenceCensus<'_>,
        values: &[(NodeId, u32)],
        budget: &mut BookV2ColumnPageBudget,
        inspect: impl for<'candidate> FnOnce(
            &ColumnCandidateContext<'candidate>,
            &mut BookV2ColumnPageBudget,
        ) -> Result<R, E>,
    ) -> Result<R, E> {
        self.verify_budget(budget)?;
        if !std::ptr::eq(census.source, self.flow)
            || !retains(&budget.inner.observation, &census.progress)
            || census.owners.len() != values.len()
        {
            return Err(E::Identity);
        }
        let flow = prepare_page_candidate_for_census_counted(
            self.flow,
            self.navigation,
            census,
            values,
            self.limits,
            self.maximum,
            &mut budget.inner.observation,
        )?;
        let candidate = ColumnCandidateContext {
            original: self,
            flow: &flow,
        };
        inspect(&candidate, budget)
    }

    pub(super) fn census_counted(
        &self,
        budget: &mut BookV2ColumnPageBudget,
    ) -> Result<ColumnPageReferenceCensus<'_>, E> {
        self.verify_budget(budget)?;
        prepare_column_page_census_counted(
            self.flow,
            self.limits,
            self.maximum,
            &mut budget.inner.observation,
        )
    }
}

fn prepare_page_candidate_for_census_counted<'a>(
    flow: &'a PreparedBookV2TextFlow<'a>,
    navigation: &'a PreparedBookV2Navigation<'a>,
    census: &ColumnPageReferenceCensus<'_>,
    values: &[(NodeId, u32)],
    limits: &M4EffectiveResourceLimits,
    maximum: u64,
    total: &mut BookV2PdfConvergenceObservation,
) -> Result<PreparedBookV2TextFlow<'a>, E> {
    for (owner, (supplied, _)) in census.owners.iter().zip(values) {
        step(total, maximum, "column Page owners")?;
        if owner != supplied {
            return Err(E::Identity);
        }
    }
    let mut source = typaxis_syntax::book_v2::BookV2PageCandidateObservation::default();
    let flow = typaxis_syntax::book_v2::prepare_book_v2_page_reference_candidate_counted(
        flow,
        flow.body(),
        navigation,
        values,
        total.records,
        total.spool,
        total.work,
        limits.base().get().max_fragments,
        maximum,
        &mut source,
    );
    // The authentic candidate copy reports every accepted request, including
    // requests accepted before a later allocation, source or work error.
    total.records = total.records.max(source.record_charge());
    total.spool = total.spool.max(source.spool_charge());
    total.work = total.work.max(source.work_steps());
    let flow = flow.map_err(|error| match error {
        typaxis_syntax::book_v2::BookV2PageCandidateError::Source(error) => {
            stage("column Page source", error)
        }
        error => stage("column Page source", error),
    })?;
    Ok(flow)
}

fn prepare_column_page_census_counted<'a>(
    flow: &'a PreparedBookV2TextFlow<'a>,
    limits: &M4EffectiveResourceLimits,
    maximum: u64,
    total: &mut BookV2PdfConvergenceObservation,
) -> Result<ColumnPageReferenceCensus<'a>, E> {
    total.records = add(
        total.records,
        1,
        limits.base().get().max_fragments,
        "column Page census record",
    )?;
    let mut owners = Vec::new();
    for paragraph in flow.paragraphs() {
        step(total, maximum, "column Page paragraphs")?;
        for site in paragraph.items() {
            step(total, maximum, "column Page sites")?;
            if matches!(
                site.reference(),
                Some(typaxis_syntax::ProductionInlineReference::Anchor {
                    format: typaxis_syntax::ProductionReferenceFormat::Page,
                    ..
                })
            ) {
                reserve_one(&mut owners, total, limits, maximum)?;
                owners.push(site.owner());
            }
        }
    }
    sort_owners(&mut owners, total, maximum)?;
    for pair in owners.windows(2) {
        step(total, maximum, "column Page unique owners")?;
        if pair[0] >= pair[1] {
            return Err(E::Identity);
        }
    }
    let mut regions = Vec::new();
    for (master_index, master) in flow
        .body()
        .body()
        .wire()
        .advanced_page_masters()
        .masters
        .iter()
        .enumerate()
    {
        step(total, maximum, "column Page masters")?;
        for (kind, region) in [
            (BookV2PageRegionKind::Header, master.header_content.as_ref()),
            (BookV2PageRegionKind::Footer, master.footer_content.as_ref()),
        ] {
            step(total, maximum, "column Page regions")?;
            let (mut blocks, mut inlines) = (0usize, 0usize);
            if let Some(region) = region {
                step(total, maximum, "column Page region source")?;
                for block in &region.blocks {
                    step(total, maximum, "column Page region blocks")?;
                    blocks = blocks
                        .checked_add(1)
                        .ok_or(E::Limit("column Page blocks"))?;
                    let children = match block {
                        WirePageRegionBlock::Paragraph { children, .. }
                        | WirePageRegionBlock::Heading { children, .. } => children,
                    };
                    for child in children {
                        step(total, maximum, "column Page region inlines")?;
                        // Exhaustive grammar check: running regions have no
                        // Page Reference variant. A future grammar extension
                        // must update this complete census explicitly.
                        match child {
                            WirePageRegionInline::Text { .. }
                            | WirePageRegionInline::SoftBreak { .. }
                            | WirePageRegionInline::HardBreak { .. } => {}
                        }
                        inlines = inlines
                            .checked_add(1)
                            .ok_or(E::Limit("column Page inlines"))?;
                    }
                }
            }
            reserve_one(&mut regions, total, limits, maximum)?;
            regions.push(ColumnMasterRegionCensus {
                master_index,
                master_id: &master.master_id,
                kind,
                owner: region.map(|r| NodeId::new(r.node_id)),
                blocks,
                inlines,
            });
        }
    }
    Ok(ColumnPageReferenceCensus {
        source: flow,
        owners,
        regions,
        progress: *total,
    })
}

impl ColumnPageReferenceCensus<'_> {
    pub(super) fn owners(&self) -> &[NodeId] {
        &self.owners
    }
    pub(super) fn regions(&self) -> &[ColumnMasterRegionCensus<'_>] {
        &self.regions
    }
    pub(super) fn prefix(&self) -> [u64; 3] {
        prefix(&self.progress)
    }
    pub(super) fn reservation_sizes(&self) -> [usize; 2] {
        [
            std::mem::size_of::<NodeId>(),
            std::mem::size_of::<ColumnMasterRegionCensus<'_>>(),
        ]
    }
}

impl ColumnMasterRegionCensus<'_> {
    pub(super) fn page_references(&self) -> usize {
        0
    }
}

fn step(
    total: &mut BookV2PdfConvergenceObservation,
    maximum: u64,
    label: &'static str,
) -> Result<(), E> {
    total.work = add(total.work, 1, maximum, label)?;
    Ok(())
}

fn reserve_one<T>(
    target: &mut Vec<T>,
    total: &mut BookV2PdfConvergenceObservation,
    limits: &M4EffectiveResourceLimits,
    maximum: u64,
) -> Result<(), E> {
    total.records = add(
        total.records,
        1,
        limits.base().get().max_fragments,
        "column Page census records",
    )?;
    if target.len() == target.capacity() {
        // The complete new request is prepaid while the old allocation can
        // still exist. Capacity is observed, never inferred from UTF-8 or a
        // prior request. This is cumulative requested storage, not a physical
        // allocator-size measurement, and earlier requests are never refunded.
        let bytes = target
            .len()
            .checked_add(1)
            .and_then(|n| n.checked_mul(std::mem::size_of::<T>()))
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(E::Limit("column Page census spool"))?;
        total.spool = add(
            total.spool,
            bytes,
            limits.base().get().max_spool_bytes,
            "column Page census spool",
        )?;
    }
    step(total, maximum, "column Page census write")?;
    target
        .try_reserve_exact(1)
        .map_err(|_| E::Limit("column Page census allocation"))
}

// Every comparison and swap is charged immediately before that operation.
// No standard-library sorting comparison bound or hidden allocation is used.
fn sort_owners(
    owners: &mut [NodeId],
    total: &mut BookV2PdfConvergenceObservation,
    maximum: u64,
) -> Result<(), E> {
    fn sift(
        owners: &mut [NodeId],
        mut root: usize,
        end: usize,
        total: &mut BookV2PdfConvergenceObservation,
        maximum: u64,
    ) -> Result<(), E> {
        loop {
            step(total, maximum, "column Page sort loop")?;
            let Some(mut child) = root
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .filter(|n| *n < end)
            else {
                return Ok(());
            };
            if child + 1 < end {
                step(total, maximum, "column Page sort comparison")?;
                if owners[child] < owners[child + 1] {
                    child += 1;
                }
            }
            step(total, maximum, "column Page sort comparison")?;
            if owners[root] >= owners[child] {
                return Ok(());
            }
            step(total, maximum, "column Page sort swap")?;
            owners.swap(root, child);
            root = child;
        }
    }
    for root in (0..owners.len() / 2).rev() {
        sift(owners, root, owners.len(), total, maximum)?;
    }
    for end in (1..owners.len()).rev() {
        step(total, maximum, "column Page sort swap")?;
        owners.swap(0, end);
        sift(owners, 0, end, total, maximum)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum OriginalSourcePreparation {
    Legacy,
    Observed,
}

pub(super) fn with_original_column_context<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'ctx> FnOnce(
        &ColumnOriginalContext<'ctx>,
        &mut BookV2ColumnPageBudget,
    ) -> Result<R, E>,
) -> Result<R, E> {
    with_original_column_context_mode(
        input,
        limits,
        mode,
        budget,
        OriginalSourcePreparation::Legacy,
        inspect,
    )
}

pub(super) fn with_observed_original_column_context<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    inspect: impl for<'ctx> FnOnce(
        &ColumnOriginalContext<'ctx>,
        &mut BookV2ColumnPageBudget,
    ) -> Result<R, E>,
) -> Result<R, E> {
    with_original_column_context_mode(
        input,
        limits,
        mode,
        budget,
        OriginalSourcePreparation::Observed,
        inspect,
    )
}

fn with_original_column_context_mode<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &mut BookV2ColumnPageBudget,
    preparation: OriginalSourcePreparation,
    inspect: impl for<'ctx> FnOnce(
        &ColumnOriginalContext<'ctx>,
        &mut BookV2ColumnPageBudget,
    ) -> Result<R, E>,
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
    let flow = prepare_original_source(
        body.styled(),
        &navigation,
        limits,
        maximum,
        total,
        preparation,
    )?;
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
    let original = ColumnOriginalContext {
        input,
        limits,
        mode,
        navigation: &navigation,
        policy: &policy,
        bindings: &bindings,
        native: native.as_ref(),
        flow: &flow,
        columns: &columns,
        maximum,
        prepared_progress: *total,
    };
    inspect(&original, budget)
}

// Source preparation alone selects its accounting mode. Navigation, policy,
// native math, frame-plan construction and the context loan stay shared.
fn prepare_original_source<'a>(
    body: &'a StyledBookV2Body,
    navigation: &'a PreparedBookV2Navigation<'a>,
    limits: &M4EffectiveResourceLimits,
    maximum: u64,
    total: &mut BookV2PdfConvergenceObservation,
    preparation: OriginalSourcePreparation,
) -> Result<PreparedBookV2TextFlow<'a>, E> {
    match preparation {
        OriginalSourcePreparation::Legacy => {
            let mut records = total.records;
            let flow = prepare_book_v2_text_flow_counted(
                body,
                navigation,
                total.records,
                limits.base().get().max_fragments,
                &mut records,
            );
            total.records = total.records.max(records);
            flow.map_err(|e| stage("column source", e))
        }
        OriginalSourcePreparation::Observed => {
            let mut observed = BookV2ObservedSourceFlowObservation::default();
            let flow = prepare_book_v2_text_flow_observed_counted(
                body,
                navigation,
                total.records,
                total.spool,
                total.work,
                limits.base().get().max_fragments,
                maximum,
                &mut observed,
            );
            // Retain all accepted source requests before propagating any error.
            // Full records include Style/scratch requests; the flow's intrinsic
            // source_record_charge remains its distinct legacy semantic count.
            total.records = total.records.max(observed.record_charge());
            total.spool = total.spool.max(observed.spool_charge());
            total.work = total.work.max(observed.work_steps());
            flow.map_err(E::ObservedSource)
        }
    }
}

// P1 is deliberately a separate private context, not a mode switch for the
// closed public/default path. Native payment and source verification share one
// genuine loan while immutable roots have their own short context lifetime.
pub(super) struct P1ObservedColumnOriginalContext<'ctx, 'id> {
    pub(super) input: &'ctx PreparedBookV2Resources,
    pub(super) limits: &'ctx M4EffectiveResourceLimits,
    pub(super) mode: typaxis_linebreak::JapaneseLineBreakMode,
    pub(super) navigation: &'ctx PreparedBookV2Navigation<'ctx>,
    pub(super) policy: &'ctx BookV2ResourcePolicy<'ctx>,
    pub(super) bindings: &'ctx BookV2VectorBindings<'ctx>,
    pub(super) native: Option<&'ctx BookV2ObservedNativeMath<'id, 'ctx>>,
    pub(super) flow: &'ctx PreparedBookV2TextFlow<'ctx>,
    pub(super) columns: &'ctx BookV2ColumnFramePlan<'ctx>,
    pub(super) maximum: u64,
    pub(super) prepared_progress: BookV2PdfConvergenceObservation,
}

impl<'ctx, 'id> P1ObservedColumnOriginalContext<'ctx, 'id> {
    fn verify_progress(&self, progress: &ColumnProgressView<'_, 'id, '_>) -> Result<(), E> {
        progress
            .accounting
            .require_limit_domain(self.limits.base())
            .map_err(E::ObservedAccounting)?;
        progress.require_pagination_domain(self.limits)?;
        if progress.accounting.maximum_work() != self.maximum
            || !retains(&progress.snapshot(), &self.prepared_progress)
        {
            return Err(E::Identity);
        }
        Ok(())
    }

    pub(super) fn census_observed(
        &self,
        progress: &mut ColumnProgressView<'_, 'id, '_>,
    ) -> Result<ColumnPageReferenceCensus<'_>, E> {
        self.verify_progress(progress)?;
        with_observed_source_counter_operation(progress, self.limits, |local| {
            prepare_column_page_census_counted(
                self.flow,
                self.limits,
                self.maximum,
                &mut local.inner.observation,
            )
        })
    }

    pub(super) fn with_page_references_observed<'cells, R>(
        &self,
        census: &ColumnPageReferenceCensus<'_>,
        values: &[(NodeId, u32)],
        progress: &mut ColumnProgressView<'_, 'id, 'cells>,
        inspect: impl for<'candidate> FnOnce(
            &PreparedBookV2TextFlow<'candidate>,
            &mut ColumnProgressView<'_, 'id, 'cells>,
        ) -> Result<R, E>,
    ) -> Result<R, E> {
        self.verify_progress(progress)?;
        if !std::ptr::eq(census.source, self.flow)
            || !retains(&progress.snapshot(), &census.progress)
            || census.owners.len() != values.len()
        {
            return Err(E::Identity);
        }
        let flow = with_observed_source_counter_operation(progress, self.limits, |local| {
            prepare_page_candidate_for_census_counted(
                self.flow,
                self.navigation,
                census,
                values,
                self.limits,
                self.maximum,
                &mut local.inner.observation,
            )
        })?;
        inspect(&flow, progress)
    }
}

pub(super) fn with_p1_observed_original_column_context<'cells, R>(
    input: &PreparedBookV2Resources,
    limits: &'cells M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &'cells mut BookV2ColumnPageBudget,
    inspect: impl for<'ctx, 'id> FnOnce(
        &P1ObservedColumnOriginalContext<'ctx, 'id>,
        &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
        &mut ColumnRestProgress<'cells>,
    ) -> Result<R, E>,
) -> Result<R, E> {
    with_original_context_impl(input,limits,mode,budget,None,|ctx,a,r,_|inspect(ctx,a,r))
}
pub(super) fn with_p1_observed_original_column_command_context<'cells,R>(
    input:&PreparedBookV2Resources,limits:&'cells M4EffectiveResourceLimits,
    mode:typaxis_linebreak::JapaneseLineBreakMode,budget:&'cells mut BookV2ColumnPageBudget,
    command:&mut ObservedColumnRegionPdfCommand,
    inspect:impl for<'ctx,'id> FnOnce(&P1ObservedColumnOriginalContext<'ctx,'id>,
        &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id,'cells>,
        &mut ColumnRestProgress<'cells>,&mut ObservedColumnRegionPdfCommand)->Result<R,E>,
)->Result<R,E>{
    if command.limits_fingerprint!=limits.fingerprint()||command.maximum!=budget.inner.maximum_work{return Err(E::Identity);}
    with_original_context_impl(input,limits,mode,budget,Some(command),|ctx,a,r,command|inspect(ctx,a,r,command.expect("command context")))
}
fn with_original_context_impl<'cells, R>(
    input: &PreparedBookV2Resources,
    limits: &'cells M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &'cells mut BookV2ColumnPageBudget,
    mut command:Option<&mut ObservedColumnRegionPdfCommand>,
    inspect: impl for<'ctx, 'id> FnOnce(
        &P1ObservedColumnOriginalContext<'ctx, 'id>,
        &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
        &mut ColumnRestProgress<'cells>,
        Option<&mut ObservedColumnRegionPdfCommand>,
    ) -> Result<R, E>,
) -> Result<R, E> {
    if budget.inner.limits_fingerprint != limits.fingerprint() {
        return Err(E::Identity);
    }
    let maximum = budget.inner.maximum_work;
    let total = &mut budget.inner.observation;
    let body = input.body();
    let navigation = prepare_reserved_book_v2_navigation(body.styled(), limits, total)?;
    let policy = prepare_book_v2_resource_policy(body, limits)
        .map_err(|error| stage("column policy", error))?;
    let bindings = bind_book_v2_vectors(&policy, input.resources(), limits)
        .map_err(|error| stage("column vectors", error))?;
    // Loan setup is nonfallible and follows the existing earlier policy and
    // binding checks. The owner keeps the actual original cells on all Results.
    with_observed_column_progress(
        total,
        limits.base(),
        limits.base().get().max_fragments,
        maximum,
        |accounting, rest| {
            let native=match command.as_deref_mut(){
                Some(command)=>command.fonts.compute_native_math_observed_counted(&bindings,input.resources(),limits,accounting),
                None=>compute_book_v2_native_math_observed_counted(&bindings,input.resources(),limits,accounting),
            }.map_err(E::from)?;
            let flow = accounting
                .prepare_source(body.styled(), &navigation)
                .map_err(E::from)?;
            let columns = prepare_p1_observed_column_frame_plan(&flow, limits, accounting)?;
            let prepared_progress = ColumnProgressView {
                accounting: &mut *accounting,
                rest: &mut *rest,
            }
            .snapshot();
            let original = P1ObservedColumnOriginalContext {
                input,
                limits,
                mode,
                navigation: &navigation,
                policy: &policy,
                bindings: &bindings,
                native: native.as_ref(),
                flow: &flow,
                columns: &columns,
                maximum,
                prepared_progress,
            };
            inspect(&original, accounting, rest,command)
        },
    )
}
