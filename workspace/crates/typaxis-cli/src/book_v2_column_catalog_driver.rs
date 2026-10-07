//! Discover actual column/full-page-note header widths while every original
//! and sibling graph stays alive. Source scopes and the budget are shared with
//! the single-body driver; all replay/catalog owners remain column-specific.
use super::header_catalog_driver::{
    header_source_scope, reserve, HeaderCatalogBudget, HeaderWidthRequest,
};
use super::{stage, E};
use typaxis_core::{M4EffectiveResourceLimits, NodeId, PositiveLength};
use typaxis_layout::book_v2::*;
use typaxis_pagination::book_v2::*;
use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};

pub(in crate::book_v2_resources) fn with_column_header_catalog<R>(
    base: &BookV2ColumnLineVariantSeed<'_>,
    requests: &[HeaderWidthRequest],
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    budget: &mut HeaderCatalogBudget,
    use_catalog: impl FnOnce(
        &BookV2ColumnTableHeaderCatalog<'_, '_, '_, '_, '_>,
        &mut HeaderCatalogBudget,
    ) -> Result<R, E>,
) -> Result<R, E> {
    if budget.line_passes > limits.base().get().max_line_reshape_passes {
        return Err(E::Limit("header line passes"));
    }
    let flow = base.source_flow();
    budget.work(1, maximum_work)?;
    budget.records = budget.records.max(base.record_charge());
    // All ownership-layer vectors and width views are prepaid before allocation.
    let count = requests
        .len()
        .checked_add(1)
        .ok_or(E::Limit("header count"))?;
    let storage = (count as u64)
        .checked_mul(10)
        .and_then(|n| n.checked_add(flow.paragraphs().len() as u64))
        .and_then(|n| {
            (flow.tables().len() as u64)
                .checked_mul(count as u64)
                .and_then(|m| n.checked_add(m))
        })
        .and_then(|n| n.checked_add(1))
        .ok_or(E::Limit("header records"))?;
    budget.storage(storage, limits)?;
    let mut previous = None;
    for request in requests {
        budget.work(1, maximum_work)?;
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
        let mut replay = BookV2LineVariantBudget::new(maximum_work - budget.work, 0);
        let mut entered = false;
        let result = with_budgeted_rebuilt_book_v2_column_line_variants(
            &[base],
            &mut replay,
            budget.records,
            |set| -> Result<_, E> {
                entered = true;
                let v = set.variant(0).ok_or(E::Identity)?;
                budget.work(set.work_steps(), maximum_work)?;
                budget.records = set.record_charge();
                let frames = v.lines().frames().ok_or(E::Identity)?;
                let mut roots = Vec::new();
                reserve(&mut roots, flow.tables().len())?;
                let mut depth = 0usize;
                let mut table_depth = None;
                for event in flow.events() {
                    budget.work(1, maximum_work)?;
                    match *event {
                        Event::Begin { owner, kind } => {
                            depth = depth.checked_add(1).ok_or(E::Identity)?;
                            if kind == Region::Table && table_depth.is_none() {
                                table_depth = Some(depth);
                                budget
                                    .work(frames.measurement_region_lookup_work(), maximum_work)?;
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
        );
        if !entered {
            budget.records = budget.records.max(replay.record_charge());
            budget.work(replay.work_steps(), maximum_work)?;
        }
        result.map_err(|e| stage("header base replay", e))??
    };
    let mut scopes = Vec::new();
    budget.storage(requests.len() as u64, limits)?;
    reserve(&mut scopes, requests.len())?;
    for request in requests {
        budget.work(roots.len() as u64, maximum_work)?;
        if roots.iter().any(|r| r.0 == request.owner) {
            scopes.push((request.owner, None));
        } else {
            let (root, owners) =
                header_source_scope(flow, request.owner, limits, maximum_work, budget)?;
            scopes.push((root, Some(owners)));
        }
    }
    // One physical root/width can be shared by several independently repeated
    // headers. Retain the union of their exact source paths in one sibling graph.
    budget.storage(
        (requests.len() as u64)
            .checked_mul(2)
            .ok_or(E::Limit("header groups"))?,
        limits,
    )?;
    let mut groups: Vec<(NodeId, PositiveLength, Option<Vec<NodeId>>)> = Vec::new();
    let mut request_groups = Vec::new();
    reserve(&mut groups, requests.len())?;
    reserve(&mut request_groups, requests.len())?;
    for (request, (root, owners)) in requests.iter().zip(scopes) {
        budget.work(groups.len() as u64 + 1, maximum_work)?;
        let index = groups
            .iter()
            .position(|g| g.0 == root && g.1 == request.parent_width);
        if let Some(index) = index {
            let group = &mut groups[index];
            if group.2.is_none() {
                let (found_root, original) =
                    header_source_scope(flow, root, limits, maximum_work, budget)?;
                if found_root != root {
                    return Err(E::Identity);
                }
                group.2 = Some(original);
            }
            let extra = match owners {
                Some(owners) => owners,
                None => header_source_scope(flow, request.owner, limits, maximum_work, budget)?.1,
            };
            let owners = group.2.as_mut().ok_or(E::Identity)?;
            let combined = owners
                .len()
                .checked_add(extra.len())
                .ok_or(E::Limit("header source union"))?;
            budget.storage(extra.len() as u64, limits)?;
            budget.work(
                (combined as u64)
                    .checked_mul(u64::from(combined.checked_ilog2().unwrap_or(0)) + 3)
                    .ok_or(E::Limit("header source union"))?,
                maximum_work,
            )?;
            owners
                .try_reserve_exact(extra.len())
                .map_err(|_| E::Limit("header allocation"))?;
            owners.extend(extra);
            owners.sort_unstable();
            owners.dedup();
            request_groups.push(index);
        } else {
            request_groups.push(groups.len());
            groups.push((root, request.parent_width, owners));
        }
    }
    let mut width_sets = Vec::new();
    reserve(&mut width_sets, requests.len())?;
    for (root_owner, parent_width, _) in &groups {
        budget.work(roots.len() as u64, maximum_work)?;
        let mut widths = Vec::new();
        reserve(&mut widths, roots.len())?;
        let mut found = false;
        for &(owner, original) in &roots {
            let width = if owner == *root_owner {
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
    reserve(&mut profiles, flow.paragraphs().len())?;
    budget.work(flow.paragraphs().len() as u64, maximum_work)?;
    profiles.resize(flow.paragraphs().len(), None);
    let mut assignments = Vec::new();
    reserve(&mut assignments, requests.len())?;
    for (widths, (root, _, owners)) in width_sets.iter().zip(&groups) {
        budget.work(1, maximum_work)?;
        let assignment = BookV2SourceWidthAssignments::new(flow, &profiles)
            .map_err(|e| stage("header widths", e))?
            .with_root_table_widths(widths);
        assignments.push(match owners {
            Some(owners) => assignment.with_table_source_scope(*root, owners),
            None => assignment.with_table_header_scope(*root),
        });
    }
    let mut siblings = Vec::new();
    reserve(&mut siblings, requests.len())?;
    for assignment in &assignments {
        let remaining_passes = limits
            .base()
            .get()
            .max_line_reshape_passes
            .checked_sub(budget.line_passes)
            .ok_or(E::Limit("header line passes"))?;
        let mut allowance =
            BookV2LineVariantBudget::new(maximum_work - budget.work, remaining_passes);
        let result =
            base.prepare_budgeted_with_source_widths(assignment, &mut allowance, budget.records);
        budget.records = budget.records.max(allowance.record_charge());
        budget.work(allowance.work_steps(), maximum_work)?;
        budget.line_passes = budget
            .line_passes
            .checked_add(allowance.reshape_passes())
            .filter(|n| *n <= limits.base().get().max_line_reshape_passes)
            .ok_or(E::Limit("header line passes"))?;
        let seed = result.map_err(|e| stage("header line convergence", e))?;
        budget.records = seed.record_charge();
        siblings.push(seed);
    }
    let mut seeds = Vec::new();
    reserve(&mut seeds, count)?;
    budget.work(siblings.len() as u64 + 1, maximum_work)?;
    seeds.push(base);
    seeds.extend(siblings.iter());
    let mut replay = BookV2LineVariantBudget::new(maximum_work - budget.work, 0);
    let mut entered = false;
    let result = with_budgeted_rebuilt_book_v2_column_line_variants(
        &seeds,
        &mut replay,
        budget.records,
        |set| -> Result<R, E> {
            entered = true;
            budget.work(set.work_steps(), maximum_work)?;
            budget.records = set.record_charge();
            budget.storage(count as u64, limits)?;
            budget.work(count as u64, maximum_work)?;
            let mut variants = Vec::new();
            reserve(&mut variants, count)?;
            variants.extend(set.variants());
            let mut numbers = Vec::new();
            reserve(&mut numbers, count)?;
            for v in &variants {
                budget.work(1, maximum_work)?;
                let mut records = budget.records;
                let number = typaxis_shaping::book_v2::shape_book_v2_equation_numbers_counted(
                    v.lines().prepared().shaped(),
                    limits,
                    budget.records,
                    &mut records,
                );
                budget.records = budget.records.max(records);
                let number = number.map_err(|e| stage("header equation labels", e))?;
                numbers.push(number);
            }
            let mut blocks = Vec::new();
            reserve(&mut blocks, count)?;
            for (v, number) in variants.iter().zip(&numbers) {
                budget.work(1, maximum_work)?;
                let mut records = budget.records;
                let block = prepare_book_v2_vector_blocks_counted(
                    v.lines(),
                    number.as_ref(),
                    limits,
                    budget.records,
                    &mut records,
                );
                budget.records = budget.records.max(records);
                let block = block.map_err(|e| stage("header block layout", e))?;
                blocks.push(block);
            }
            let mut measurements = Vec::new();
            reserve(&mut measurements, count)?;
            for (v, block) in variants.iter().zip(&blocks) {
                budget.work(1, maximum_work)?;
                let mut records = budget.records;
                let flow = prepare_book_v2_rebuilt_column_flow_counted(
                    v,
                    block.as_ref(),
                    limits,
                    budget.records,
                    &mut records,
                );
                budget.records = budget.records.max(records);
                let flow = flow.map_err(|e| stage("header body flow", e))?;
                let measurement =
                    prepare_book_v2_column_table_measurements_counted(flow, limits, &mut records);
                budget.records = budget.records.max(records);
                let measurement = measurement.map_err(|e| stage("header tables", e))?;
                measurements.push(measurement);
            }
            let mut headers = Vec::new();
            reserve(&mut headers, requests.len())?;
            for (request, &group) in requests.iter().zip(&request_groups) {
                let measurement = measurements.get(group + 1).ok_or(E::Identity)?;
                budget.work(1, maximum_work)?;
                if measurements[0]
                    .tables()
                    .get(request.table_index)
                    .map(|t| t.owner())
                    != Some(request.owner)
                {
                    return Err(E::Identity);
                }
                let mut records = budget.records;
                let mut work = 0;
                let header = prepare_book_v2_column_table_header_variant_counted(
                    &set,
                    &measurements[0],
                    measurement,
                    request.table_index,
                    limits,
                    maximum_work - budget.work,
                    budget.records,
                    &mut records,
                    &mut work,
                );
                budget.records = budget.records.max(records);
                budget.work(work, maximum_work)?;
                let header = header.map_err(|e| stage("header variant", e))?;
                headers.push(header);
            }
            let mut refs = Vec::new();
            reserve(&mut refs, requests.len())?;
            budget.work(requests.len() as u64, maximum_work)?;
            refs.extend(headers.iter());
            let mut records = budget.records;
            let mut work = 0;
            let catalog = prepare_book_v2_column_table_header_catalog_counted(
                &measurements[0],
                &refs,
                limits,
                maximum_work - budget.work,
                budget.records,
                &mut records,
                &mut work,
            );
            budget.records = budget.records.max(records);
            budget.work(work, maximum_work)?;
            let catalog = catalog.map_err(|e| stage("header catalog", e))?;
            use_catalog(&catalog, budget)
        },
    );
    if !entered {
        budget.records = budget.records.max(replay.record_charge());
        budget.work(replay.work_steps(), maximum_work)?;
    }
    result.map_err(|e| stage("header replay set", e))?
}

enum Discovery<R> {
    Width(HeaderWidthRequest),
    Ready(R),
}

/// Discover only widths requested by actual page-search continuations. Every
/// abandoned search, replay and request allocation remains in the caller ledger.
/// A completed discovery is provisional: the caller still needs stable pages,
/// source-width convergence and the full display/PDF proof.
pub(in crate::book_v2_resources) fn with_discovered_column_header_catalog<R>(
    base: &BookV2ColumnLineVariantSeed<'_>,
    requests: &mut Vec<HeaderWidthRequest>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    budget: &mut HeaderCatalogBudget,
    use_catalog: impl FnOnce(
        &BookV2ColumnTableHeaderCatalog<'_, '_, '_, '_, '_>,
        &mut HeaderCatalogBudget,
    ) -> Result<R, E>,
) -> Result<R, E> {
    use typaxis_pagination::ProductionBodyPaginationErrorKind as P;
    let mut use_catalog = Some(use_catalog);
    loop {
        if budget.page_passes >= limits.base().get().max_layout_passes {
            return Err(E::Limit("header page passes"));
        }
        let outcome = with_column_header_catalog(
            base,
            &requests,
            limits,
            maximum_work,
            budget,
            |catalog, budget| {
                budget.page_passes = budget
                    .page_passes
                    .checked_add(1)
                    .filter(|n| *n <= limits.base().get().max_layout_passes)
                    .ok_or(E::Limit("header page passes"))?;
                let mut records = budget.records;
                let mut work = 0;
                let search = prepare_book_v2_column_page_search_with_headers_counted(
                    catalog,
                    limits,
                    maximum_work - budget.work,
                    budget.records,
                    &mut records,
                    &mut work,
                );
                if search.is_err() {
                    budget.records = budget.records.max(records);
                    budget.work(work, maximum_work)?;
                }
                let mut search = search.map_err(|e| stage("header discovery search", e))?;
                let selected = search.select_pages();
                budget.work(search.work_steps(), maximum_work)?;
                budget.records = search.record_charge();
                match selected {
                    Err(e) => match e.kind {
                        P::TableHeaderWidthRequired {
                            table_index,
                            parent_width,
                        } => Ok(Discovery::Width(HeaderWidthRequest {
                            table_index,
                            owner: e.owner,
                            parent_width,
                        })),
                        _ => Err(stage("header discovery pages", e)),
                    },
                    Ok(_) => Ok(Discovery::Ready(use_catalog.take().ok_or(E::Identity)?(
                        catalog, budget,
                    )?)),
                }
            },
        )?;
        match outcome {
            Discovery::Ready(value) => return Ok(value),
            Discovery::Width(request) => {
                budget.work(
                    u64::from(requests.len().checked_ilog2().unwrap_or(0)) + 1,
                    maximum_work,
                )?;
                let index = requests
                    .binary_search_by_key(
                        &(request.table_index, request.parent_width.get().raw()),
                        |r| (r.table_index, r.parent_width.get().raw()),
                    )
                    .err()
                    .ok_or(E::Identity)?;
                budget.storage(1, limits)?;
                budget.work((requests.len() - index) as u64 + 1, maximum_work)?;
                requests
                    .try_reserve_exact(1)
                    .map_err(|_| E::Limit("header request allocation"))?;
                requests.insert(index, request);
            }
        }
    }
}
