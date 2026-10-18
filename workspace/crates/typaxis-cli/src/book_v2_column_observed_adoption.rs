//! Column ownership orchestration on one live source-accounting loan.
//! Legacy orchestration stays available for semantic compatibility comparisons.
use super::super::header_catalog_driver::{
    header_source_scope_in_mode, HeaderScopeAccounting, HeaderWidthRequest,
};
use super::source_context::P1ObservedColumnOriginalContext;
use super::*;
use typaxis_core::PositiveLength;
use typaxis_shaping::book_v2::{
    BookV2ObservedAccountingError as A, BookV2ObservedSourceAccounting,
};
use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};

#[path = "book_v2_column_observed_pdf.rs"]
mod pdf;
pub(super) use pdf::with_candidate_math;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Storage {
    WidthViews,
    StartViews,
    EndViews,
    Roots,
    ScopeOwners,
    Scopes,
    Groups,
    RequestGroups,
    OwnerUnion,
    WidthSets,
    RootWidths,
    Profiles,
    Assignments,
    Siblings,
    Seeds,
    Variants,
    Numbers,
    Blocks,
    Measurements,
    Headers,
    HeaderRefs,
    DiscoveryRequests,
}
#[cfg(test)]
pub(super) mod reserve_controls {
    use super::*;
    #[derive(Debug)]
    pub struct Reservation {
        pub site: String,
        pub count: usize,
        pub element_bytes: usize,
        pub before: [u64; 3],
        pub paid: [u64; 3],
    }
    struct State {
        fail_at: Option<usize>,
        requests: Vec<Reservation>,
    }
    thread_local! {
        static STATE: std::cell::RefCell<Option<State>> = std::cell::RefCell::new(None);
    }
    pub fn run<R>(fail_at: Option<usize>, run: impl FnOnce() -> R) -> (R, Vec<Reservation>) {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                STATE.with(|s| {
                    s.borrow_mut().take();
                });
            }
        }
        STATE.with(|s| {
            let mut state = s.borrow_mut();
            assert!(state.is_none(), "nested reservation controls");
            *state = Some(State {
                fail_at,
                requests: Vec::new(),
            });
        });
        let reset = Reset;
        let result = run();
        let requests = STATE.with(|s| s.borrow_mut().take().unwrap().requests);
        drop(reset);
        (result, requests)
    }
    pub(super) fn charged(
        site: Storage,
        count: usize,
        element_bytes: usize,
        before: [u64; 3],
        paid: [u64; 3],
    ) -> bool {
        STATE.with(|s| {
            let mut state = s.borrow_mut();
            let Some(state) = state.as_mut() else {
                return false;
            };
            let index = state.requests.len();
            state.requests.push(Reservation {
                site: format!("{site:?}"),
                count,
                element_bytes,
                before,
                paid,
            });
            state.fail_at == Some(index)
        })
    }
}
fn allocation_error() -> E {
    E::ObservedPagination(typaxis_pagination::ProductionBodyPaginationError {
        owner: NodeId::new(0),
        kind: typaxis_pagination::ProductionBodyPaginationErrorKind::AllocationFailure,
    })
}
fn storage_request<T>(count: usize) -> Result<[u64; 3], E> {
    let owner = NodeId::new(0);
    let records = u64::try_from(count).map_err(|_| E::ObservedAccounting(A::Records { owner }))?;
    let spool = count
        .checked_mul(std::mem::size_of::<T>())
        .and_then(|n| u64::try_from(n).ok())
        .ok_or(E::ObservedAccounting(A::Spool { owner }))?;
    Ok([records, spool, records])
}
fn storage<T>(
    values: &mut Vec<T>,
    target: usize,
    _site: Storage,
    accounting: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<(), E> {
    let additional = target.checked_sub(values.len()).ok_or(E::Identity)?;
    #[cfg(test)]
    let before = accounting.counters();
    accounting
        .debit(NodeId::new(0), storage_request::<T>(target)?)
        .map_err(E::ObservedAccounting)?;
    #[cfg(test)]
    if reserve_controls::charged(
        _site,
        target,
        std::mem::size_of::<T>(),
        before,
        accounting.counters(),
    ) {
        return Err(allocation_error());
    }
    values
        .try_reserve_exact(additional)
        .map_err(|_| allocation_error())
}
fn work(accounting: &mut BookV2ObservedSourceAccounting<'_, '_>, amount: u64) -> Result<(), E> {
    accounting
        .debit_work(NodeId::new(0), amount)
        .map_err(E::ObservedAccounting)
}
fn sort_owners(
    values: &mut Vec<NodeId>,
    accounting: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<(), E> {
    fn sift(
        v: &mut [NodeId],
        mut root: usize,
        end: usize,
        a: &mut BookV2ObservedSourceAccounting<'_, '_>,
    ) -> Result<(), E> {
        while let Some(left) = root
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .filter(|n| *n < end)
        {
            let mut child = left;
            if left + 1 < end {
                work(a, 1)?;
                if v[left] < v[left + 1] {
                    child += 1;
                }
            }
            work(a, 1)?;
            if v[root] >= v[child] {
                break;
            }
            work(a, 1)?;
            v.swap(root, child);
            root = child;
        }
        Ok(())
    }
    let length = values.len();
    for root in (0..length / 2).rev() {
        sift(values, root, length, accounting)?;
    }
    for end in (1..values.len()).rev() {
        work(accounting, 1)?;
        values.swap(0, end);
        sift(values, 0, end, accounting)?;
    }
    let mut kept = 0;
    for i in 0..values.len() {
        work(accounting, 1)?;
        if kept == 0 || values[kept - 1] != values[i] {
            if kept != i {
                work(accounting, 1)?;
                values[kept] = values[i];
            }
            kept += 1;
        }
    }
    values.truncate(kept);
    Ok(())
}
struct Scope<'a, 'id, 'cells>(&'a mut BookV2ObservedSourceAccounting<'id, 'cells>);
impl HeaderScopeAccounting for Scope<'_, '_, '_> {
    fn owners(&mut self, count: usize) -> Result<Vec<NodeId>, E> {
        let mut values = Vec::new();
        storage(&mut values, count, Storage::ScopeOwners, self.0)?;
        Ok(values)
    }
    fn work(&mut self, count: u64) -> Result<(), E> {
        work(self.0, count)
    }
    fn sort(&mut self, owners: &mut Vec<NodeId>) -> Result<(), E> {
        sort_owners(owners, self.0)
    }
}
fn source_scope(
    flow: &PreparedBookV2TextFlow<'_>,
    owner: NodeId,
    a: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<(NodeId, Vec<NodeId>), E> {
    header_source_scope_in_mode(flow, owner, &mut Scope(a))
}
fn retain_records<T>(
    result: Result<T, typaxis_pagination::ProductionBodyPaginationError>,
    before: u64,
    records: u64,
    a: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<T, E> {
    let retained = records
        .checked_sub(before)
        .ok_or(E::Identity)
        .and_then(|n| {
            a.debit_records(NodeId::new(0), n)
                .map_err(E::ObservedAccounting)
        });
    let value = result.map_err(E::ObservedPagination)?;
    retained?;
    Ok(value)
}
fn retain_record_work<T>(
    result: Result<T, typaxis_pagination::ProductionBodyPaginationError>,
    before: u64,
    records: u64,
    steps: u64,
    a: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<T, E> {
    let retained_r = records
        .checked_sub(before)
        .ok_or(E::Identity)
        .and_then(|n| {
            a.debit_records(NodeId::new(0), n)
                .map_err(E::ObservedAccounting)
        });
    let retained_w = a
        .debit_work(NodeId::new(0), steps)
        .map_err(E::ObservedAccounting);
    let value = result.map_err(E::ObservedPagination)?;
    retained_r?;
    retained_w?;
    Ok(value)
}
fn retain_lines(
    progress: &mut ColumnProgressView<'_, '_, '_>,
    begun: u16,
    limits: &M4EffectiveResourceLimits,
) -> Result<(), E> {
    *progress.rest.line_passes = progress
        .rest
        .line_passes
        .checked_add(begun)
        .filter(|n| *n <= limits.base().get().max_line_reshape_passes)
        .ok_or(E::Limit("observed column line passes"))?;
    Ok(())
}
fn retain_pages(
    progress: &mut ColumnProgressView<'_, '_, '_>,
    begun: u16,
    limits: &M4EffectiveResourceLimits,
) -> Result<(), E> {
    *progress.rest.page_passes = progress
        .rest
        .page_passes
        .checked_add(begun)
        .filter(|n| *n <= limits.base().get().max_layout_passes)
        .ok_or(E::Limit("observed column page passes"))?;
    Ok(())
}
fn remaining_lines(
    progress: &ColumnProgressView<'_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) -> Result<u16, E> {
    limits
        .base()
        .get()
        .max_line_reshape_passes
        .checked_sub(*progress.rest.line_passes)
        .ok_or(E::Limit("observed column line passes"))
}
/// Search owns absolute R/S and local W until its final mutation. Import each
/// accepted axis even when another axis or the primary operation has refused.
fn retain_search(
    progress: &mut ColumnProgressView<'_, '_, '_>,
    search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
    mark: &mut ColumnSearchChargeMark,
) -> Result<(), E> {
    let r = search.record_charge();
    let s = search.terminal_spool_charge();
    let w = search.work_steps();
    let kept_r = r
        .checked_sub(mark.records)
        .ok_or(E::Identity)
        .and_then(|n| {
            progress
                .accounting
                .debit_records(NodeId::new(0), n)
                .map_err(E::ObservedAccounting)
        });
    if kept_r.is_ok() {
        mark.records = r;
    }
    let kept_s = s.checked_sub(mark.spool).ok_or(E::Identity).and_then(|n| {
        progress
            .accounting
            .debit_spool(NodeId::new(0), n)
            .map_err(E::ObservedAccounting)
    });
    if kept_s.is_ok() {
        mark.spool = s;
    }
    let kept_w = w.checked_sub(mark.work).ok_or(E::Identity).and_then(|n| {
        progress
            .accounting
            .debit_work(NodeId::new(0), n)
            .map_err(E::ObservedAccounting)
    });
    if kept_w.is_ok() {
        mark.work = w;
    }
    *progress.rest.balance_candidates =
        (*progress.rest.balance_candidates).max(search.balance_candidates());
    kept_r?;
    kept_s?;
    kept_w
}

fn with_catalog<'id, 'cells, R>(
    base: &BookV2ObservedColumnLineVariantSeed<'id, '_>,
    requests: &[HeaderWidthRequest],
    limits: &M4EffectiveResourceLimits,
    progress: &mut ColumnProgressView<'_, 'id, 'cells>,
    use_catalog: impl FnOnce(
        &BookV2ColumnTableHeaderCatalog<'_, '_, '_, '_, '_>,
        &mut ColumnProgressView<'_, 'id, 'cells>,
    ) -> Result<R, E>,
) -> Result<R, E> {
    progress
        .accounting
        .require_limit_domain(limits.base())
        .map_err(E::ObservedAccounting)?;
    progress
        .accounting
        .require_limit_domain(base.source_flow().body().body().limits())
        .map_err(E::ObservedAccounting)?;
    progress.require_pagination_domain(limits)?;
    progress
        .accounting
        .debit(NodeId::new(0), [0; 3])
        .map_err(E::ObservedAccounting)?;
    let flow = base.source_flow();
    let count = requests
        .len()
        .checked_add(1)
        .ok_or(E::Limit("observed header count"))?;
    let mut previous = None;
    for request in requests {
        work(progress.accounting, 1)?;
        let key = (request.table_index, request.parent_width.get().raw());
        if previous.is_some_and(|p| p >= key)
            || flow.tables().get(request.table_index).map(|t| t.owner()) != Some(request.owner)
        {
            return Err(E::Identity);
        }
        previous = Some(key);
    }
    let roots = if requests.is_empty() {
        Vec::new()
    } else {
        let mut replay =
            BookV2ObservedLineVariantBudget::new(progress.accounting.remaining_work(), 0);
        with_observed_rebuilt_book_v2_column_line_variants(
            &[base],
            &mut replay,
            progress.accounting,
            |set, a| -> Result<_, E> {
                let v = set.variant(0).ok_or(E::Identity)?;
                let frames = v.lines().frames().ok_or(E::Identity)?;
                let mut roots = Vec::new();
                storage(&mut roots, flow.tables().len(), Storage::Roots, a)?;
                let (mut depth, mut table_depth) = (0usize, None);
                for event in flow.events() {
                    work(a, 1)?;
                    match *event {
                        Event::Begin { owner, kind } => {
                            depth = depth.checked_add(1).ok_or(E::Identity)?;
                            if kind == Region::Table && table_depth.is_none() {
                                table_depth = Some(depth);
                                work(a, frames.measurement_region_lookup_work())?;
                                roots.push((
                                    owner,
                                    frames.measurement_region(owner).ok_or(E::Identity)?.width(),
                                ));
                            }
                        }
                        Event::End { .. } => {
                            if table_depth == Some(depth) {
                                table_depth = None;
                            }
                            depth = depth.checked_sub(1).ok_or(E::Identity)?;
                        }
                        _ => {}
                    }
                }
                if depth != 0 || table_depth.is_some() {
                    return Err(E::Identity);
                }
                Ok(roots)
            },
        )
        .map_err(E::from)??
    };
    let mut scopes = Vec::new();
    storage(
        &mut scopes,
        requests.len(),
        Storage::Scopes,
        progress.accounting,
    )?;
    for request in requests {
        work(progress.accounting, roots.len() as u64)?;
        if roots.iter().any(|r| r.0 == request.owner) {
            scopes.push((request.owner, None));
        } else {
            let (root, owners) = source_scope(flow, request.owner, progress.accounting)?;
            scopes.push((root, Some(owners)));
        }
    }
    let mut groups: Vec<(NodeId, PositiveLength, Option<Vec<NodeId>>)> = Vec::new();
    let mut request_groups = Vec::new();
    storage(
        &mut groups,
        requests.len(),
        Storage::Groups,
        progress.accounting,
    )?;
    storage(
        &mut request_groups,
        requests.len(),
        Storage::RequestGroups,
        progress.accounting,
    )?;
    for (request, (root, owners)) in requests.iter().zip(scopes) {
        work(progress.accounting, groups.len() as u64 + 1)?;
        let index = groups
            .iter()
            .position(|g| g.0 == root && g.1 == request.parent_width);
        if let Some(index) = index {
            let group = &mut groups[index];
            if group.2.is_none() {
                let (found, original) = source_scope(flow, root, progress.accounting)?;
                if found != root {
                    return Err(E::Identity);
                }
                group.2 = Some(original);
            }
            let extra = match owners {
                Some(v) => v,
                None => source_scope(flow, request.owner, progress.accounting)?.1,
            };
            let owners = group.2.as_mut().ok_or(E::Identity)?;
            let combined = owners
                .len()
                .checked_add(extra.len())
                .ok_or(E::Limit("observed header union"))?;
            storage(owners, combined, Storage::OwnerUnion, progress.accounting)?;
            owners.extend(extra);
            sort_owners(owners, progress.accounting)?;
            request_groups.push(index);
        } else {
            request_groups.push(groups.len());
            groups.push((root, request.parent_width, owners));
        }
    }
    let mut width_sets = Vec::new();
    storage(
        &mut width_sets,
        requests.len(),
        Storage::WidthSets,
        progress.accounting,
    )?;
    for (root, parent_width, _) in &groups {
        let mut widths = Vec::new();
        storage(
            &mut widths,
            roots.len(),
            Storage::RootWidths,
            progress.accounting,
        )?;
        let mut found = false;
        for &(owner, original) in &roots {
            work(progress.accounting, 1)?;
            let width = if owner == *root {
                if found {
                    return Err(E::Identity);
                }
                found = true;
                *parent_width
            } else {
                original
            };
            widths.push((owner, width));
        }
        if !found {
            return Err(E::Identity);
        }
        width_sets.push(widths);
    }
    let mut profiles = Vec::new();
    storage(
        &mut profiles,
        flow.paragraphs().len(),
        Storage::Profiles,
        progress.accounting,
    )?;
    profiles.resize(flow.paragraphs().len(), None);
    let mut assignments = Vec::new();
    storage(
        &mut assignments,
        requests.len(),
        Storage::Assignments,
        progress.accounting,
    )?;
    for (widths, (root, _, owners)) in width_sets.iter().zip(&groups) {
        work(progress.accounting, 1)?;
        let assignment = BookV2SourceWidthAssignments::new(flow, &profiles)
            .map_err(|_| E::Identity)?
            .with_root_table_widths(widths);
        assignments.push(match owners {
            Some(owners) => assignment.with_table_source_scope(*root, owners),
            None => assignment.with_table_header_scope(*root),
        });
    }
    let mut siblings = Vec::new();
    storage(
        &mut siblings,
        requests.len(),
        Storage::Siblings,
        progress.accounting,
    )?;
    for assignment in &assignments {
        let mut allowance = BookV2ObservedLineVariantBudget::new(
            progress.accounting.remaining_work(),
            remaining_lines(progress, limits)?,
        );
        let result = base.prepare_observed_with_source_widths(
            assignment,
            &mut allowance,
            progress.accounting,
        );
        let retained = retain_lines(progress, allowance.reshape_passes(), limits);
        let seed = result.map_err(E::from)?;
        retained?;
        siblings.push(seed);
    }
    let mut seeds = Vec::new();
    storage(&mut seeds, count, Storage::Seeds, progress.accounting)?;
    seeds.push(base);
    seeds.extend(siblings.iter());
    let mut replay = BookV2ObservedLineVariantBudget::new(progress.accounting.remaining_work(), 0);
    let accounting = &mut *progress.accounting;
    let rest = &mut *progress.rest;
    with_observed_rebuilt_book_v2_column_line_variants(
        &seeds,
        &mut replay,
        accounting,
        |set, a| -> Result<R, E> {
            let mut progress = ColumnProgressView {
                accounting: a,
                rest,
            };
            let mut variants = Vec::new();
            storage(&mut variants, count, Storage::Variants, progress.accounting)?;
            variants.extend(set.variants());
            let mut numbers = Vec::new();
            storage(&mut numbers, count, Storage::Numbers, progress.accounting)?;
            for v in &variants {
                work(progress.accounting, 1)?;
                let value =
                    typaxis_shaping::book_v2::shape_book_v2_equation_numbers_with_observed_storage(
                        v.lines().prepared().shaped(),
                        limits,
                        progress.accounting,
                    );
                let value = value.map_err(|err| stage("observed header equation labels", err))?;
                numbers.push(value);
            }
            let mut blocks = Vec::new();
            storage(&mut blocks, count, Storage::Blocks, progress.accounting)?;
            for (v, number) in variants.iter().zip(&numbers) {
                work(progress.accounting, 1)?;
                let value =
                    typaxis_layout::book_v2::prepare_book_v2_vector_blocks_with_observed_storage(
                        v.lines(),
                        number.as_ref(),
                        limits,
                        progress.accounting,
                    );
                let value = value.map_err(|err| stage("observed header block layout", err))?;
                blocks.push(value);
            }
            let mut measurements = Vec::new();
            storage(
                &mut measurements,
                count,
                Storage::Measurements,
                progress.accounting,
            )?;
            for (v, block) in variants.iter().zip(&blocks) {
                work(progress.accounting, 1)?;
                let before = progress.accounting.counters()[0];
                let mut records = before;
                let value = prepare_observed_book_v2_rebuilt_column_flow_counted(
                    v,
                    block.as_ref(),
                    limits,
                    before,
                    &mut records,
                );
                let flow = retain_records(value, before, records, progress.accounting)?;
                let before = progress.accounting.counters()[0];
                let value =
                    prepare_book_v2_column_table_measurements_counted(flow, limits, &mut records);
                measurements.push(retain_records(value, before, records, progress.accounting)?);
            }
            let mut headers = Vec::new();
            storage(
                &mut headers,
                requests.len(),
                Storage::Headers,
                progress.accounting,
            )?;
            for (request, &group) in requests.iter().zip(&request_groups) {
                let measurement = measurements.get(group + 1).ok_or(E::Identity)?;
                work(progress.accounting, 1)?;
                if measurements[0]
                    .tables()
                    .get(request.table_index)
                    .map(|t| t.owner())
                    != Some(request.owner)
                {
                    return Err(E::Identity);
                }
                let before = progress.accounting.counters()[0];
                let (mut records, mut steps) = (before, 0);
                let value = prepare_observed_book_v2_column_table_header_variant_counted(
                    &set,
                    &measurements[0],
                    measurement,
                    request.table_index,
                    limits,
                    progress.accounting.remaining_work(),
                    before,
                    &mut records,
                    &mut steps,
                );
                headers.push(retain_record_work(
                    value,
                    before,
                    records,
                    steps,
                    progress.accounting,
                )?);
            }
            let mut refs = Vec::new();
            storage(
                &mut refs,
                requests.len(),
                Storage::HeaderRefs,
                progress.accounting,
            )?;
            refs.extend(headers.iter());
            let catalog = prepare_book_v2_column_table_header_catalog_with_observed_source_counted(
                &measurements[0],
                &refs,
                limits,
                progress.accounting,
            )
            .map_err(|err| match err {
                BookV2ObservedColumnHeaderCatalogError::Accounting(e) => E::ObservedAccounting(e),
                BookV2ObservedColumnHeaderCatalogError::Catalog(e) => E::ObservedPagination(e),
            })?;
            use_catalog(&catalog, &mut progress)
        },
    )
    .map_err(E::from)?
}

fn with_search<'b, 'f, 's, 'p, 'a, 'id, 'cells, R>(
    catalog: &'b BookV2ColumnTableHeaderCatalog<'b, 'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    progress: &mut ColumnProgressView<'_, 'id, 'cells>,
    inspect: impl FnOnce(
        &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &mut ColumnProgressView<'_, 'id, 'cells>,
        &mut ColumnSearchChargeMark,
    ) -> Result<R, E>,
) -> Result<R, E> {
    let mut mark = progress.search_mark();
    let (mut records, mut steps) = (mark.records, 0);
    let value = prepare_book_v2_column_page_search_with_headers_counted(
        catalog,
        limits,
        progress.accounting.remaining_work(),
        mark.records,
        &mut records,
        &mut steps,
    );
    let mut search = match value {
        Ok(value) => value,
        Err(err) => {
            let _ = retain_record_work::<()>(
                Err(err),
                mark.records,
                records,
                steps,
                progress.accounting,
            );
            return Err(E::ObservedPagination(err));
        }
    };
    search.retain_prior_balance_candidates(*progress.rest.balance_candidates);
    let result = (|| {
        search
            .retain_prior_balance_spool(mark.spool)
            .map_err(E::ObservedPagination)?;
        inspect(&mut search, progress, &mut mark)
    })();
    let retained = retain_search(progress, &search, &mut mark);
    let value = result?;
    retained?;
    Ok(value)
}
fn with_discovered_catalog<'id, 'cells, R>(
    base: &BookV2ObservedColumnLineVariantSeed<'id, '_>,
    requests: &mut Vec<HeaderWidthRequest>,
    limits: &M4EffectiveResourceLimits,
    progress: &mut ColumnProgressView<'_, 'id, 'cells>,
    use_catalog: impl FnOnce(
        &BookV2ColumnTableHeaderCatalog<'_, '_, '_, '_, '_>,
        &mut ColumnProgressView<'_, 'id, 'cells>,
    ) -> Result<R, E>,
) -> Result<R, E> {
    use typaxis_pagination::ProductionBodyPaginationErrorKind as P;
    let mut use_catalog = Some(use_catalog);
    loop {
        if *progress.rest.page_passes >= limits.base().get().max_layout_passes {
            return Err(E::Limit("observed header page passes"));
        }
        let outcome = with_catalog(base, requests, limits, progress, |catalog, progress| {
            retain_pages(progress, 1, limits)?;
            let missing = with_search(catalog, limits, progress, |search, _, _| {
                match search.select_balanced_pages() {
                    Ok(_) => Ok(None),
                    Err(err) => match err.kind {
                        P::TableHeaderWidthRequired {
                            table_index,
                            parent_width,
                        } => Ok(Some(HeaderWidthRequest {
                            table_index,
                            owner: err.owner,
                            parent_width,
                        })),
                        _ => Err(E::ObservedPagination(err)),
                    },
                }
            })?;
            if let Some(request) = missing {
                Ok(Err(request))
            } else {
                Ok(Ok(use_catalog.take().ok_or(E::Identity)?(
                    catalog, progress,
                )?))
            }
        })?;
        match outcome {
            Ok(value) => return Ok(value),
            Err(request) => {
                work(
                    progress.accounting,
                    u64::from(requests.len().checked_ilog2().unwrap_or(0)) + 1,
                )?;
                let index = requests
                    .binary_search_by_key(
                        &(request.table_index, request.parent_width.get().raw()),
                        |r| (r.table_index, r.parent_width.get().raw()),
                    )
                    .err()
                    .ok_or(E::Identity)?;
                let target = requests
                    .len()
                    .checked_add(1)
                    .ok_or(E::Limit("observed header requests"))?;
                work(progress.accounting, (requests.len() - index) as u64 + 1)?;
                storage(
                    requests,
                    target,
                    Storage::DiscoveryRequests,
                    progress.accounting,
                )?;
                requests.insert(index, request);
            }
        }
    }
}

// Only implementations in this module can select a final consumer. Observers
// receive readonly search state, so a callback cannot replace/reset the owner.
trait Delivery<'id, 'cells, R> {
    fn finish<'b, 'f, 's, 'p, 'a>(
        self,
        pages: &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        search: &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        feedback: &BookV2ColumnWidthFeedback<'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        progress: &mut ColumnProgressView<'_, 'id, 'cells>,
        mark: &mut ColumnSearchChargeMark,
    ) -> Result<R, E>;
}
struct SourceDelivery<F>(F);
impl<'id, 'cells, R, F> Delivery<'id, 'cells, R> for SourceDelivery<F>
where
    F: for<'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnWidthFeedback<'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> Result<R, E>,
{
    fn finish<'b, 'f, 's, 'p, 'a>(
        self,
        pages: &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        search: &mut BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        feedback: &BookV2ColumnWidthFeedback<'p, 'a>,
        _: &M4EffectiveResourceLimits,
        progress: &mut ColumnProgressView<'_, 'id, 'cells>,
        mark: &mut ColumnSearchChargeMark,
    ) -> Result<R, E> {
        retain_search(progress, search, mark)?;
        *progress.rest.candidates = progress
            .rest
            .candidates
            .checked_add(1)
            .ok_or(E::Limit("observed column candidates"))?;
        (self.0)(pages, search, feedback, progress.snapshot())
    }
}

pub(super) fn with_original_pages<'cells, R>(
    input: &PreparedBookV2Resources,
    limits: &'cells M4EffectiveResourceLimits,
    mode: typaxis_linebreak::JapaneseLineBreakMode,
    budget: &'cells mut BookV2ColumnPageBudget,
    inspect: impl for<'b, 'f, 's, 'p, 'a> FnOnce(
        &BookV2ColumnRepeatedPages<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a>,
        &BookV2ColumnWidthFeedback<'p, 'a>,
        BookV2PdfConvergenceObservation,
    ) -> Result<R, E>,
) -> Result<R, E> {
    source_context::with_p1_observed_original_column_context(
        input,
        limits,
        mode,
        budget,
        |ctx, accounting, rest| {
            with_source_pages(
                ctx,
                ctx.flow,
                &mut ColumnProgressView { accounting, rest },
                SourceDelivery(inspect),
            )
        },
    )
}

#[cfg(test)]
pub(super) fn with_test_catalog<R>(
    input: &PreparedBookV2Resources,
    limits: &M4EffectiveResourceLimits,
    budget: &mut BookV2ColumnPageBudget,
    requests: &[(usize, PositiveLength)],
    inspect: impl FnOnce(
        &BookV2ColumnTableHeaderCatalog<'_, '_, '_, '_, '_>,
        BookV2PdfConvergenceObservation,
    ) -> R,
) -> Result<R, E> {
    source_context::with_p1_observed_original_column_context(
        input,
        limits,
        typaxis_linebreak::JapaneseLineBreakMode::Normal,
        budget,
        |ctx, accounting, rest| {
            let mut progress = ColumnProgressView { accounting, rest };
            let mut allowance = BookV2ObservedLineVariantBudget::new(
                progress.accounting.remaining_work(),
                remaining_lines(&progress, limits)?,
            );
            let seed = prepare_observed_book_v2_column_line_variant_seed(
                ctx.policy,
                ctx.flow,
                ctx.input.resources(),
                ctx.bindings,
                limits,
                ctx.mode,
                ctx.columns,
                ctx.native,
                None,
                &mut allowance,
                progress.accounting,
            );
            let retained = retain_lines(&mut progress, allowance.reshape_passes(), limits);
            let seed = seed.map_err(E::from)?;
            retained?;
            let requests = requests
                .iter()
                .map(|&(table_index, parent_width)| {
                    Ok(HeaderWidthRequest {
                        table_index,
                        parent_width,
                        owner: ctx
                            .flow
                            .tables()
                            .get(table_index)
                            .ok_or(E::Identity)?
                            .owner(),
                    })
                })
                .collect::<Result<Vec<_>, E>>()?;
            with_catalog(
                &seed,
                &requests,
                limits,
                &mut progress,
                |catalog, progress| Ok(inspect(catalog, progress.snapshot())),
            )
        },
    )
}

fn with_source_pages<'id, 'cells, R>(
    original: &P1ObservedColumnOriginalContext<'_, 'id>,
    flow: &PreparedBookV2TextFlow<'_>,
    progress: &mut ColumnProgressView<'_, 'id, 'cells>,
    consumer: impl Delivery<'id, 'cells, R>,
) -> Result<R, E> {
    let limits = original.limits;
    let columns = original.columns;
    if !std::ptr::eq(flow.body(), original.flow.body())
        || !std::ptr::eq(flow.navigation(), original.navigation)
    {
        return Err(E::Identity);
    }
    progress
        .accounting
        .require_limit_domain(limits.base())
        .map_err(E::ObservedAccounting)?;
    progress
        .accounting
        .require_limit_domain(flow.body().body().limits())
        .map_err(E::ObservedAccounting)?;
    progress.require_pagination_domain(limits)?;
    progress
        .accounting
        .debit(NodeId::new(0), [0; 3])
        .map_err(E::ObservedAccounting)?;
    let prepared = original.prepared_progress;
    let current = progress.snapshot();
    if current.records < prepared.records
        || current.spool < prepared.spool
        || current.work < prepared.work
    {
        return Err(E::Identity);
    }
    progress
        .accounting
        .debit_records(NodeId::new(0), 2)
        .map_err(E::ObservedAccounting)?;
    let caps = limits.base().get();
    let mut previous: Option<BookV2ColumnWidthFeedback<'_, '_>> = None;
    let mut cycle = WidthCycle::default();
    let mut refining = false;
    let mut requests = Vec::new();
    let mut consumer = Some(consumer);
    loop {
        if caps
            .max_layout_passes
            .saturating_sub(*progress.rest.page_passes)
            < 3
        {
            return Err(E::Limit("observed column page passes"));
        }
        let passes = remaining_lines(progress, limits)?;
        if passes == 0 {
            return Err(E::Limit("observed column line passes"));
        }
        let mut widths = Vec::new();
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        if let Some(old) = &previous {
            if !old.matches_source_flow(flow) || !std::ptr::eq(old.column_plan(), columns) {
                return Err(E::Identity);
            }
            let n = old.paragraphs().len();
            storage(&mut widths, n, Storage::WidthViews, progress.accounting)?;
            storage(&mut starts, n, Storage::StartViews, progress.accounting)?;
            storage(&mut ends, n, Storage::EndViews, progress.accounting)?;
            for p in old.paragraphs() {
                work(
                    progress.accounting,
                    2 + p.widths().len() as u64
                        + p.source_unit_starts().map_or(0, |v| v.len() as u64),
                )?;
                widths.push(Some(p.widths()));
                starts.push(p.source_unit_starts());
                ends.push(p.retained_line_ends());
            }
            work(
                progress.accounting,
                (old.block_widths().len() as u64)
                    .checked_add(old.block_starts().len() as u64)
                    .ok_or(E::Limit("observed column blocks"))?,
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
                    .map_err(|_| E::Identity)
            })
            .transpose()?;
        let mut allowance =
            BookV2ObservedLineVariantBudget::new(progress.accounting.remaining_work(), passes);
        let result = prepare_observed_book_v2_column_line_variant_seed(
            original.policy,
            flow,
            original.input.resources(),
            original.bindings,
            limits,
            original.mode,
            columns,
            original.native,
            assignments.as_ref(),
            &mut allowance,
            progress.accounting,
        );
        let retained = retain_lines(progress, allowance.reshape_passes(), limits);
        let seed = result.map_err(E::from)?;
        retained?;
        let mut retry = None;
        let outcome = with_discovered_catalog(
            &seed,
            &mut requests,
            limits,
            progress,
            |catalog, progress| {
                with_search(catalog, limits, progress, |search, progress, mark| {
                    let mut begun = 0;
                    let result = search.select_repeated_pages_counted(
                        caps.max_layout_passes - *progress.rest.page_passes,
                        &mut begun,
                    );
                    let retained = retain_pages(progress, begun, limits);
                    let pages = result.map_err(E::ObservedPagination)?;
                    retained?;
                    *progress.rest.width_passes = progress
                        .rest
                        .width_passes
                        .checked_add(1)
                        .ok_or(E::Limit("observed column width passes"))?;
                    let mut feedback = search
                        .paragraph_frame_feedback(pages.sequence())
                        .map_err(E::ObservedPagination)?;
                    let same =
                        previous.as_ref().map_or(true, |old| {
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
                                .map_err(E::ObservedPagination)?;
                            *progress.rest.width_refinements = progress
                                .rest
                                .width_refinements
                                .checked_add(1)
                                .ok_or(E::Limit("observed column width refinements"))?;
                        }
                        retry = Some(
                            search
                                .capture_source_width_feedback(feedback, flow, columns)
                                .map_err(E::ObservedPagination)?,
                        );
                        return Ok(None);
                    }
                    consumer
                        .take()
                        .ok_or(E::Identity)?
                        .finish(&pages, search, &feedback, limits, progress, mark)
                        .map(Some)
                })
            },
        );
        if let Some(value) = outcome? {
            return Ok(value);
        }
        previous = Some(retry.ok_or(E::Identity)?);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;

    #[test]
    fn book_v2_column_observed_adoption_metadata_request_is_atomic_on_every_short_axis() {
        let prior = [7, 11, 13];
        // A real tuple used by root-width discovery, rather than an inferred
        // byte count from text or another graph's record total.
        type Entry = (NodeId, PositiveLength);
        let wanted = [10, 11 + 3 * std::mem::size_of::<Entry>() as u64, 16];
        for short in [None, Some(0), Some(1), Some(2)] {
            let mut base = ResourceLimits::default();
            base.max_fragments = wanted[0] - u64::from(short == Some(0));
            base.max_spool_bytes = wanted[1] - u64::from(short == Some(1));
            let limits = ValidatedResourceLimits::new(base).unwrap();
            let [mut r, mut s, mut w] = prior;
            let mut values: Vec<Entry> = Vec::new();
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                wanted[0],
                wanted[2] - u64::from(short == Some(2)),
                |a| storage(&mut values, 3, Storage::Roots, a),
            );
            if let Some(axis) = short {
                let owner = NodeId::new(0);
                let expected = [A::Records { owner }, A::Spool { owner }, A::Work { owner }][axis];
                assert!(matches!(result, Err(E::ObservedAccounting(e)) if e == expected));
                assert_eq!([r, s, w], prior);
                assert_eq!(values.capacity(), 0);
            } else {
                result.unwrap();
                assert_eq!([r, s, w], wanted);
                assert!(values.capacity() >= 3);
            }
        }
        assert!(matches!(
            storage_request::<[u8; 2]>(usize::MAX),
            Err(E::ObservedAccounting(A::Spool { .. }))
        ));
    }

    #[test]
    fn book_v2_column_observed_adoption_owner_sort_charges_actual_prefix_without_scratch() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let original = vec![
            NodeId::new(9),
            NodeId::new(3),
            NodeId::new(9),
            NodeId::new(1),
            NodeId::new(3),
        ];
        let (mut r, mut s, mut w) = (7, 11, 13);
        let mut values = original.clone();
        let capacity = values.capacity();
        with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            &limits,
            100,
            MAX_SORT_WORK,
            |a| sort_owners(&mut values, a),
        )
        .unwrap();
        assert_eq!(values, [NodeId::new(1), NodeId::new(3), NodeId::new(9)]);
        assert_eq!(values.capacity(), capacity);
        assert_eq!([r, s], [7, 11]);
        let exact_work = w;
        for maximum in [13, exact_work - 1, exact_work] {
            let (mut r, mut s, mut w) = (7, 11, 13);
            let mut values = original.clone();
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                100,
                maximum,
                |a| sort_owners(&mut values, a),
            );
            assert_eq!(w, maximum);
            assert_eq!([r, s], [7, 11]);
            assert_eq!(values.capacity(), capacity);
            assert_eq!(result.is_ok(), maximum == exact_work);
            if maximum == 13 {
                assert_eq!(values, original);
            }
        }
    }
    const MAX_SORT_WORK: u64 = 1_000;
}
