//! Page-end choices over real body fragments. All feasible non-keep boundaries
//! on an overflowing page are examined; budget exhaustion never truncates them.
use super::*;
use crate::CostComponents;
use std::collections::BTreeSet;

pub const PRODUCTION_BODY_BREAK_POLICY: &str = "typaxis.production-body-break/1";
const ISOLATION_COST: i64 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyBreakReason {
    Overflow,
    Forced,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyBreakCandidate {
    end_item: u32,
    owner: NodeId,
    used_height: Length,
    costs: CostComponents,
}
impl ProductionBodyBreakCandidate {
    /// Exclusive end in the common Item stream, including explicit page breaks.
    pub const fn end_item(self) -> u32 {
        self.end_item
    }
    pub const fn owner(self) -> NodeId {
        self.owner
    }
    pub const fn used_height(self) -> Length {
        self.used_height
    }
    pub const fn costs(self) -> CostComponents {
        self.costs
    }
}

#[derive(Debug)]
pub struct ProductionBodyBreakDecision {
    page_index: u32,
    start_item: u32,
    selected_candidate: u32,
    reason: ProductionBodyBreakReason,
    // Canonical source boundary order, independent of backwards enumeration.
    candidates: Vec<ProductionBodyBreakCandidate>,
}
impl ProductionBodyBreakDecision {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    /// Inclusive start in the same Item stream as each candidate end.
    pub const fn start_item(&self) -> u32 {
        self.start_item
    }
    pub const fn reason(&self) -> ProductionBodyBreakReason {
        self.reason
    }
    pub const fn selected_candidate_index(&self) -> u32 {
        self.selected_candidate
    }
    pub fn candidates(&self) -> &[ProductionBodyBreakCandidate] {
        &self.candidates
    }
    pub fn selected(&self) -> ProductionBodyBreakCandidate {
        self.candidates[self.selected_candidate as usize]
    }
}

fn candidate(
    items: &[Item],
    start: usize,
    end: usize,
    used: Length,
    height: Length,
    paragraph_lengths: &[usize],
    headings: &BTreeSet<NodeId>,
    terminal: bool,
) -> Result<ProductionBodyBreakCandidate, ProductionBodyPaginationError> {
    let last = &items[end - 1];
    let mut isolated = 0;
    let mut heading = 0;
    if !terminal {
        if let Some(ProductionBodyFragmentSource::ParagraphLine {
            paragraph_index,
            line_index,
        }) = last.source
        {
            let split = matches!(items.get(end).and_then(|i| i.source),
                Some(ProductionBodyFragmentSource::ParagraphLine { paragraph_index: next, .. }) if next == paragraph_index);
            if split {
                let first = match items[start].source {
                    Some(ProductionBodyFragmentSource::ParagraphLine {
                        paragraph_index: p,
                        line_index,
                    }) if p == paragraph_index => line_index,
                    _ => 0,
                };
                let on_page = line_index
                    .checked_add(1)
                    .and_then(|n| n.checked_sub(first))
                    .ok_or_else(|| error(last.owner, E::ReceiptMismatch))?;
                let total = *paragraph_lengths
                    .get(paragraph_index as usize)
                    .ok_or_else(|| error(last.owner, E::ReceiptMismatch))?;
                let remaining = total
                    .checked_sub(line_index as usize + 1)
                    .ok_or_else(|| error(last.owner, E::ReceiptMismatch))?;
                isolated = i64::from(on_page < 2) + i64::from(remaining < 2);
            }
            heading = i64::from(headings.contains(&last.owner));
        }
    }
    let unused = if terminal {
        0
    } else {
        let slack = height
            .checked_sub(used)
            .filter(|n| *n >= Length::ZERO)
            .ok_or_else(|| error(last.owner, E::ReceiptMismatch))?;
        i64::try_from(
            i128::from(slack.raw()) * i128::from(ISOLATION_COST) / i128::from(height.raw()),
        )
        .map_err(|_| error(last.owner, E::ArithmeticOverflow))?
    };
    // Keep is a hard boundary restriction; unsupported table/footnote subflows
    // are rejected upstream. Their costs are not guessed here.
    let costs = CostComponents::new(
        0,
        isolated * ISOLATION_COST,
        heading * 2 * ISOLATION_COST,
        0,
        0,
        unused,
        0,
    )
    .ok_or_else(|| error(last.owner, E::ArithmeticOverflow))?;
    Ok(ProductionBodyBreakCandidate {
        end_item: u32::try_from(end).map_err(|_| error(last.owner, E::FragmentLimit))?,
        owner: last.owner,
        used_height: used,
        costs,
    })
}

fn select(
    items: &[Item],
    start: usize,
    page_index: u32,
    height: Length,
    paragraph_lengths: &[usize],
    headings: &BTreeSet<NodeId>,
    maximum: u16,
    charge: &mut Charge,
) -> Result<ProductionBodyBreakDecision, ProductionBodyPaginationError> {
    let owner = items[start].owner;
    let mut used = Length::ZERO;
    let mut end = start;
    while let Some(item) = items.get(end).filter(|i| i.source.is_some()) {
        let spacing = if end == start {
            Length::ZERO
        } else {
            add(items[end - 1].after, item.before, item.owner)?
        };
        let next = add(
            add(used, spacing, item.owner)?,
            item.consumed()?,
            item.owner,
        )?;
        if next > height {
            break;
        }
        used = next;
        end += 1;
    }
    if end == start {
        return Err(error(owner, E::Oversize));
    }
    let reason = if end == items.len() {
        ProductionBodyBreakReason::End
    } else if items[end].source.is_none() {
        ProductionBodyBreakReason::Forced
    } else {
        ProductionBodyBreakReason::Overflow
    };
    let mut candidates = Vec::new();
    if reason != ProductionBodyBreakReason::Overflow {
        // A mandatory boundary that already fits has no alternative cut search.
        charge.take(1, owner)?;
        candidates
            .try_reserve_exact(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        candidates.push(candidate(
            items,
            start,
            end,
            used,
            height,
            paragraph_lengths,
            headings,
            true,
        )?);
    } else {
        let mut boundary = end;
        let mut height_at_boundary = used;
        while boundary > start {
            let item = &items[boundary - 1];
            if !item.keep {
                let observed = candidates.len() as u32 + 1;
                if observed > u32::from(maximum) {
                    return Err(error(
                        item.owner,
                        E::PageBreakLookbackLimit {
                            limit: maximum,
                            observed,
                        },
                    ));
                }
                charge.take(1, item.owner)?;
                candidates
                    .try_reserve(1)
                    .map_err(|_| error(item.owner, E::AllocationFailure))?;
                candidates.push(candidate(
                    items,
                    start,
                    boundary,
                    height_at_boundary,
                    height,
                    paragraph_lengths,
                    headings,
                    false,
                )?);
            }
            height_at_boundary = height_at_boundary
                .checked_sub(item.consumed()?)
                .ok_or_else(|| error(item.owner, E::ArithmeticOverflow))?;
            if boundary > start + 1 {
                height_at_boundary = height_at_boundary
                    .checked_sub(add(items[boundary - 2].after, item.before, item.owner)?)
                    .ok_or_else(|| error(item.owner, E::ArithmeticOverflow))?;
            }
            boundary -= 1;
        }
        if candidates.is_empty() {
            return Err(error(owner, E::Oversize));
        }
        candidates.reverse();
    }
    let selected_candidate = candidates
        .iter()
        .enumerate()
        .min_by_key(|(_, c)| (c.costs.total(), c.end_item))
        .map(|(index, _)| index as u32)
        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
    Ok(ProductionBodyBreakDecision {
        page_index,
        start_item: u32::try_from(start).map_err(|_| error(owner, E::FragmentLimit))?,
        selected_candidate,
        reason,
        candidates,
    })
}

pub(super) fn plan(
    items: &[Item],
    lines: &ProductionInlineLineLayout<'_, '_>,
    body: Rect,
    limits: &M4EffectiveResourceLimits,
    charge: &mut Charge,
) -> Result<Vec<ProductionBodyBreakDecision>, ProductionBodyPaginationError> {
    let mut headings = BTreeSet::new();
    for event in lines.source_flow().events() {
        if let Event::Begin {
            owner,
            kind: Region::Heading,
        } = event
        {
            charge.take(1, *owner)?;
            headings.insert(*owner);
        }
    }
    charge.take(lines.paragraphs().len(), NodeId::new(0))?;
    let mut lengths = Vec::new();
    lengths
        .try_reserve_exact(lines.paragraphs().len())
        .map_err(|_| error(NodeId::new(0), E::AllocationFailure))?;
    lengths.extend(lines.paragraphs().iter().map(|p| p.lines().len()));
    plan_items(
        items,
        &lengths,
        &headings,
        body.height().get(),
        limits.base().get().max_pages,
        limits.base().get().max_page_break_lookback,
        charge,
    )
}

fn plan_items(
    items: &[Item],
    lengths: &[usize],
    headings: &BTreeSet<NodeId>,
    height: Length,
    maximum_pages: u32,
    maximum_candidates: u16,
    charge: &mut Charge,
) -> Result<Vec<ProductionBodyBreakDecision>, ProductionBodyPaginationError> {
    let mut decisions = Vec::new();
    let mut start = 0;
    let mut page = 0u32;
    while start < items.len() {
        let owner = items[start].owner;
        if items[start].source.is_none() {
            page = page
                .checked_add(1)
                .filter(|p| *p < maximum_pages)
                .ok_or_else(|| error(owner, E::PageLimit))?;
            start += 1;
            continue;
        }
        if page >= maximum_pages {
            return Err(error(owner, E::PageLimit));
        }
        charge.take(1, owner)?;
        let decision = select(
            items,
            start,
            page,
            height,
            lengths,
            headings,
            maximum_candidates,
            charge,
        )?;
        start = decision.selected().end_item as usize;
        decisions
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        decisions.push(decision);
        if start < items.len() && items[start].source.is_some() {
            page = page
                .checked_add(1)
                .filter(|p| *p < maximum_pages)
                .ok_or_else(|| error(owner, E::PageLimit))?;
        }
    }
    Ok(decisions)
}

pub(super) fn encode(decision: &ProductionBodyBreakDecision, bytes: &mut Vec<u8>) {
    bytes.push(4);
    for n in [
        decision.page_index,
        decision.start_item,
        decision.selected_candidate,
        decision.candidates.len() as u32,
    ] {
        bytes.extend_from_slice(&n.to_be_bytes());
    }
    bytes.push(match decision.reason {
        ProductionBodyBreakReason::Overflow => 0,
        ProductionBodyBreakReason::Forced => 1,
        ProductionBodyBreakReason::End => 2,
    });
    for c in &decision.candidates {
        bytes.extend_from_slice(&c.end_item.to_be_bytes());
        bytes.extend_from_slice(&c.owner.get().to_be_bytes());
        for n in [
            c.used_height.raw(),
            c.costs.keep(),
            c.costs.widow_orphan(),
            c.costs.heading_isolation(),
            c.costs.table_split(),
            c.costs.footnote_split(),
            c.costs.unused_space(),
            c.costs.overflow(),
        ] {
            bytes.extend_from_slice(&n.to_be_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn length(n: i64) -> Length {
        Length::from_raw(n).unwrap()
    }
    fn line(paragraph: u32, index: u32) -> Item {
        Item {
            owner: NodeId::new(paragraph + 1),
            source: Some(ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index: paragraph,
                line_index: index,
            }),
            x: Length::ZERO,
            width: PositiveLength::new(length(10)).unwrap(),
            height: length(10),
            before: Length::ZERO,
            after: Length::ZERO,
            keep: false,
            leading: Length::ZERO,
            trailing: Length::ZERO,
            viewport_left: None,
        }
    }
    fn charge() -> Charge {
        Charge { remaining: 10_000 }
    }
    fn choice(items: &[Item], lengths: &[usize], height: i64) -> ProductionBodyBreakDecision {
        select(
            items,
            0,
            0,
            length(height),
            lengths,
            &BTreeSet::new(),
            32,
            &mut charge(),
        )
        .unwrap()
    }
    #[test]
    fn avoids_one_remaining_line_and_one_first_line_using_measured_boundaries() {
        let items = (0..4).map(|i| line(0, i)).collect::<Vec<_>>();
        let decision = choice(&items, &[4], 30);
        assert_eq!(decision.selected().end_item(), 2);
        assert_eq!(
            decision
                .candidates()
                .iter()
                .map(|c| c.end_item())
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(decision.candidates()[0].costs().widow_orphan(), 1_000_000);
        assert_eq!(decision.candidates()[2].costs().widow_orphan(), 1_000_000);
        let items = vec![line(0, 0), line(1, 0), line(1, 1), line(1, 2)];
        assert_eq!(choice(&items, &[1, 3], 20).selected().end_item(), 1);
    }
    #[test]
    fn moves_heading_to_following_page_and_keeps_are_hard_boundaries() {
        let mut items = vec![line(0, 0), line(1, 0), line(2, 0)];
        let headings = BTreeSet::from([NodeId::new(2)]);
        let decision = select(
            &items,
            0,
            0,
            length(20),
            &[1, 1, 1],
            &headings,
            32,
            &mut charge(),
        )
        .unwrap();
        assert_eq!(decision.selected().end_item(), 1);
        assert_eq!(
            decision.candidates()[1].costs().heading_isolation(),
            2_000_000
        );
        items[1].keep = true;
        let decision = choice(&items, &[1, 1, 1], 20);
        assert_eq!(decision.candidates().len(), 1);
        assert_eq!(decision.selected().end_item(), 1);
        items[0].keep = true;
        assert_eq!(
            select(
                &items,
                0,
                0,
                length(20),
                &[1, 1, 1],
                &headings,
                32,
                &mut charge()
            )
            .unwrap_err()
            .kind,
            E::Oversize
        );
    }
    #[test]
    fn spaces_and_marker_extents_are_counted_once_and_page_top_space_is_discarded() {
        let mut items = vec![line(0, 0), line(1, 0), line(2, 0)];
        items[0].before = length(200);
        items[0].after = length(3);
        items[1].before = length(4);
        items[1].leading = length(2);
        items[1].trailing = length(1);
        items[1].source = Some(ProductionBodyFragmentSource::VectorBlock { block_index: 0 });
        let d = choice(&items, &[1, 1, 1], 30);
        assert_eq!(d.selected().end_item(), 2);
        assert_eq!(d.selected().used_height(), length(30));
        assert_eq!(d.candidates()[0].used_height(), length(10));
        assert_eq!(d.candidates()[0].costs().unused_space(), 666_666);
    }
    #[test]
    fn candidate_limit_is_exact_and_never_a_truncated_best_effort() {
        let items = (0..4).map(|i| line(i, 0)).collect::<Vec<_>>();
        let d = select(
            &items,
            0,
            0,
            length(30),
            &[1; 4],
            &BTreeSet::new(),
            3,
            &mut charge(),
        )
        .unwrap();
        assert_eq!(d.candidates().len(), 3);
        let mut budget = charge();
        let e = select(
            &items,
            0,
            0,
            length(30),
            &[1; 4],
            &BTreeSet::new(),
            2,
            &mut budget,
        )
        .unwrap_err();
        assert_eq!(
            e.kind,
            E::PageBreakLookbackLimit {
                limit: 2,
                observed: 3
            }
        );
        assert_eq!(e.owner, NodeId::new(1));
        assert_eq!(budget.remaining, 9_998); // The rejected candidate was not allocated/charged.
        let d = select(
            &items,
            0,
            0,
            length(40),
            &[1; 4],
            &BTreeSet::new(),
            1,
            &mut charge(),
        )
        .unwrap();
        assert_eq!(d.reason(), ProductionBodyBreakReason::End);
        assert_eq!(d.candidates().len(), 1);
    }
    #[test]
    fn explicit_blank_pages_and_final_forced_page_consume_page_budget() {
        let mut blank = line(0, 0);
        blank.source = None;
        blank.height = Length::ZERO;
        let mut last = line(0, 0);
        last.source = None;
        last.height = Length::ZERO;
        let items = [blank, line(0, 0), last];
        let d = plan_items(
            &items,
            &[1],
            &BTreeSet::new(),
            length(20),
            3,
            1,
            &mut charge(),
        )
        .unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].page_index(), 1);
        assert_eq!(d[0].start_item(), 1);
        assert_eq!(d[0].reason(), ProductionBodyBreakReason::Forced);
        assert_eq!(
            plan_items(
                &items,
                &[1],
                &BTreeSet::new(),
                length(20),
                2,
                1,
                &mut charge()
            )
            .unwrap_err()
            .kind,
            E::PageLimit
        );
    }
    #[test]
    fn equal_quantized_costs_choose_the_earliest_source_boundary() {
        let mut items = vec![line(0, 0), line(1, 0), line(2, 0)];
        items[2].height = length(100_000_000);
        let d = choice(&items, &[1, 1, 1], 100_000_000);
        assert_eq!(
            d.candidates()[0].costs().total(),
            d.candidates()[1].costs().total()
        );
        assert_eq!(d.selected().end_item(), 1);
    }
    #[test]
    fn continuation_counts_only_lines_on_current_page_and_fingerprints_all_choices() {
        let items = (0..7).map(|i| line(0, i)).collect::<Vec<_>>();
        let d = plan_items(
            &items,
            &[7],
            &BTreeSet::new(),
            length(30),
            3,
            3,
            &mut charge(),
        )
        .unwrap();
        assert_eq!(
            d.iter()
                .map(|d| d.selected().end_item())
                .collect::<Vec<_>>(),
            [3, 5, 7]
        );
        assert_eq!(d[1].candidates()[0].costs().widow_orphan(), 1_000_000);
        let mut bytes = Vec::new();
        encode(&d[0], &mut bytes);
        let mut changed = select(
            &items,
            0,
            0,
            length(30),
            &[7],
            &BTreeSet::new(),
            3,
            &mut charge(),
        )
        .unwrap();
        let mut again = Vec::new();
        encode(&changed, &mut again);
        assert_eq!(bytes, again);
        changed.candidates[0].used_height = length(11);
        again.clear();
        encode(&changed, &mut again);
        assert_ne!(bytes, again);
        let mut exhausted = Charge { remaining: 1 };
        assert_eq!(
            plan_items(
                &items,
                &[7],
                &BTreeSet::new(),
                length(30),
                3,
                3,
                &mut exhausted
            )
            .unwrap_err()
            .kind,
            E::FragmentLimit
        );
    }
}
