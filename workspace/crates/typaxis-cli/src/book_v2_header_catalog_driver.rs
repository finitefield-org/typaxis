//! Rebuild a sealed catalog from exact parent-width requests on one base source.
//! The caller owns discovery/retry policy and retains this budget across retries.
use super::{add, stage, E};
use typaxis_core::{M4EffectiveResourceLimits, NodeId, PositiveLength};
use typaxis_layout::book_v2::*;
use typaxis_pagination::book_v2::*;
use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::book_v2_resources) struct HeaderWidthRequest {
    pub table_index: usize,
    pub owner: NodeId,
    /// Width of the original ancestor root's parent, including for nested targets.
    pub parent_width: PositiveLength,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::book_v2_resources) struct HeaderCatalogBudget {
    pub work: u64,
    pub records: u64,
    pub line_passes: u16,
    pub page_passes: u16,
}
impl HeaderCatalogBudget {
    fn work(&mut self, amount: u64, maximum: u64) -> Result<(), E> {
        self.work = add(self.work, amount, maximum, "header work")?;
        Ok(())
    }
    fn storage(&mut self, amount: u64, limits: &M4EffectiveResourceLimits) -> Result<(), E> {
        self.records = add(
            self.records,
            amount,
            limits.base().get().max_fragments,
            "header records",
        )?;
        Ok(())
    }
}
fn reserve<T>(values: &mut Vec<T>, count: usize) -> Result<(), E> {
    values
        .try_reserve_exact(count)
        .map_err(|_| E::Limit("header allocation"))
}

pub(in crate::book_v2_resources) fn with_header_catalog<R>(
    base: &BookV2BodyLineVariantSeed<'_>,
    requests: &[HeaderWidthRequest],
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    budget: &mut HeaderCatalogBudget,
    use_catalog: impl FnOnce(
        &BookV2TableHeaderCatalog<'_, '_, '_, '_, '_>,
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
        with_rebuilt_book_v2_body_line_variant(
            base,
            maximum_work - budget.work,
            budget.records,
            |v| -> Result<_, E> {
                budget.work(v.work_steps(), maximum_work)?;
                budget.records = v.record_charge();
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
        )
        .map_err(|e| stage("header base replay", e))??
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
        let seed = base
            .prepare_with_source_widths(
                assignment,
                maximum_work - budget.work,
                budget.records,
                remaining_passes,
            )
            .map_err(|e| stage("header line convergence", e))?;
        budget.work(seed.work_steps(), maximum_work)?;
        budget.records = seed.record_charge();
        budget.line_passes = budget
            .line_passes
            .checked_add(seed.reshape_passes())
            .filter(|n| *n <= limits.base().get().max_line_reshape_passes)
            .ok_or(E::Limit("header line passes"))?;
        siblings.push(seed);
    }
    let mut seeds = Vec::new();
    reserve(&mut seeds, count)?;
    budget.work(siblings.len() as u64 + 1, maximum_work)?;
    seeds.push(base);
    seeds.extend(siblings.iter());
    with_rebuilt_book_v2_body_line_variants(
        &seeds,
        maximum_work - budget.work,
        budget.records,
        |set| -> Result<R, E> {
            budget.work(set.work_steps(), maximum_work)?;
            budget.records = set.record_charge();
            let mut numbers = Vec::new();
            reserve(&mut numbers, count)?;
            for v in set.variants() {
                budget.work(1, maximum_work)?;
                let number = typaxis_shaping::book_v2::shape_book_v2_equation_numbers(
                    v.lines().prepared().shaped(),
                    limits,
                    budget.records,
                )
                .map_err(|e| stage("header equation labels", e))?;
                if let Some(number) = &number {
                    budget.records = number.record_charge();
                }
                numbers.push(number);
            }
            let mut blocks = Vec::new();
            reserve(&mut blocks, count)?;
            for (v, number) in set.variants().iter().zip(&numbers) {
                budget.work(1, maximum_work)?;
                let block = prepare_book_v2_vector_blocks(
                    v.lines(),
                    number.as_ref(),
                    limits,
                    budget.records,
                )
                .map_err(|e| stage("header block layout", e))?;
                if let Some(block) = &block {
                    budget.records = block.record_charge();
                }
                blocks.push(block);
            }
            let mut measurements = Vec::new();
            reserve(&mut measurements, count)?;
            for (v, block) in set.variants().iter().zip(&blocks) {
                budget.work(1, maximum_work)?;
                let flow = prepare_book_v2_body_flow(
                    v.lines(),
                    block.as_ref(),
                    v.footnotes(),
                    limits,
                    budget.records,
                )
                .map_err(|e| stage("header body flow", e))?;
                let measurement = prepare_book_v2_table_measurements(flow, limits)
                    .map_err(|e| stage("header tables", e))?;
                budget.records = measurement.record_charge();
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
                let header = prepare_book_v2_table_header_variant(
                    &set,
                    &measurements[0],
                    measurement,
                    request.table_index,
                    limits,
                    maximum_work - budget.work,
                    budget.records,
                )
                .map_err(|e| stage("header variant", e))?;
                budget.work(header.work_steps(), maximum_work)?;
                budget.records = header.record_charge();
                headers.push(header);
            }
            let mut refs = Vec::new();
            reserve(&mut refs, requests.len())?;
            budget.work(requests.len() as u64, maximum_work)?;
            refs.extend(headers.iter());
            let catalog = prepare_book_v2_table_header_catalog(
                &measurements[0],
                &refs,
                limits,
                maximum_work - budget.work,
                budget.records,
            )
            .map_err(|e| stage("header catalog", e))?;
            budget.work(catalog.work_steps(), maximum_work)?;
            budget.records = catalog.record_charge();
            use_catalog(&catalog, budget)
        },
    )
    .map_err(|e| stage("header replay set", e))?
}

enum Discovery<R> {
    Width(HeaderWidthRequest),
    Ready(R),
}

/// Discover only widths requested by actual page-search continuations. Every
/// abandoned search, replay and request allocation remains in the caller ledger.
/// A completed discovery is provisional: the caller still needs stable pages,
/// source-width convergence and the full display/PDF proof.
pub(in crate::book_v2_resources) fn with_discovered_header_catalog<R>(
    base: &BookV2BodyLineVariantSeed<'_>,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    budget: &mut HeaderCatalogBudget,
    use_catalog: impl FnOnce(
        &BookV2TableHeaderCatalog<'_, '_, '_, '_, '_>,
        &mut HeaderCatalogBudget,
    ) -> Result<R, E>,
) -> Result<R, E> {
    use typaxis_pagination::ProductionBodyPaginationErrorKind as P;
    budget.storage(1, limits)?;
    let mut requests: Vec<HeaderWidthRequest> = Vec::new();
    let mut use_catalog = Some(use_catalog);
    loop {
        if budget.page_passes >= limits.base().get().max_layout_passes {
            return Err(E::Limit("header page passes"));
        }
        let outcome = with_header_catalog(
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
                let mut search = prepare_book_v2_table_body_search_with_headers(
                    catalog,
                    limits,
                    maximum_work - budget.work,
                    budget.records,
                )
                .map_err(|e| stage("header discovery search", e))?;
                let selected = search.select_mixed_pages();
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

/// Collect complete original header leaves, including whole tables inside the
/// header, and the ancestor root. Storage and traversal are charged before use.
fn header_source_scope(
    flow: &typaxis_syntax::book_v2::PreparedBookV2TextFlow<'_>,
    target_owner: NodeId,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    budget: &mut HeaderCatalogBudget,
) -> Result<(NodeId, Vec<NodeId>), E> {
    budget.storage(flow.events().len() as u64, limits)?;
    let mut owners = Vec::new();
    reserve(&mut owners, flow.events().len())?;
    let (mut depth, mut root, mut target, mut head) = (0usize, None, None, None);
    let mut target_root = None;
    for event in flow.events() {
        budget.work(1, maximum_work)?;
        match *event {
            Event::Begin { owner, kind } => {
                depth = depth.checked_add(1).ok_or(E::Identity)?;
                if kind == Region::Table && root.is_none() {
                    root = Some((depth, owner));
                }
                if owner == target_owner {
                    if kind != Region::Table || target_root.is_some() {
                        return Err(E::Identity);
                    }
                    target = Some(depth);
                    target_root = Some(root.ok_or(E::Identity)?.1);
                }
                if kind == Region::TableHeadRow && target == depth.checked_sub(1) {
                    head = Some(depth);
                }
                if head.is_some()
                    && matches!(
                        kind,
                        Region::Paragraph
                            | Region::Heading
                            | Region::Figure
                            | Region::VectorFigure
                            | Region::DisplayMath
                            | Region::MathVectorBlock
                            | Region::PageBreak
                            | Region::DescriptionTerm
                    )
                {
                    owners.push(owner);
                }
            }
            Event::End { .. } => {
                if head == Some(depth) {
                    head = None;
                }
                if target == Some(depth) {
                    target = None;
                }
                if root.is_some_and(|r| r.0 == depth) {
                    root = None;
                }
                depth = depth.checked_sub(1).ok_or(E::Identity)?;
            }
            _ => {}
        }
    }
    if depth != 0 || root.is_some() || target.is_some() || head.is_some() {
        return Err(E::Identity);
    }
    budget.work(
        (owners.len() as u64)
            .checked_mul(u64::from(owners.len().checked_ilog2().unwrap_or(0)) + 2)
            .ok_or(E::Limit("header source sorting"))?,
        maximum_work,
    )?;
    owners.sort_unstable();
    owners.dedup();
    Ok((target_root.ok_or(E::Identity)?, owners))
}
