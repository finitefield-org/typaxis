//! Bounded stable ranking shared by both source-specific page owners.
use super::*;
pub(super) struct Boundary<R, K> {
    pub requests: Vec<R>,
    pub key: K,
}
pub(super) fn queue<R: Copy, K: Ord + Copy>(
    requests: &[R],
    key: K,
    candidates: &mut Vec<Boundary<R, K>>,
    maximum_candidates: u16,
    charge: &mut Charge,
    steps: &mut u64,
    maximum_steps: u64,
) -> Result<(), ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    let observed = u32::try_from(candidates.len())
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| error(root, E::FragmentLimit))?;
    if observed > u32::from(maximum_candidates) {
        return Err(error(
            root,
            E::PageBreakLookbackLimit {
                limit: maximum_candidates,
                observed,
            },
        ));
    }
    charge.take(
        requests
            .len()
            .checked_add(1)
            .ok_or_else(|| error(root, E::FragmentLimit))?,
        root,
    )?;
    let mut retained = Vec::new();
    retained
        .try_reserve_exact(requests.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    for request in requests {
        visit(steps, maximum_steps, root)?;
        retained.push(*request);
    }
    candidates
        .try_reserve(1)
        .map_err(|_| error(root, E::AllocationFailure))?;
    candidates.push(Boundary {
        requests: retained,
        key,
    });
    // Stable insertion preserves the old first-enumerated winner on equal
    // keys. Every comparison/swap is paid from the same finite work owner.
    let mut index = candidates.len() - 1;
    while index > 0 {
        visit(steps, maximum_steps, root)?;
        if candidates[index - 1].key <= key {
            break;
        }
        candidates.swap(index - 1, index);
        index -= 1;
    }
    Ok(())
}
pub(super) fn cost(
    items: &[Item],
    range: Option<std::ops::Range<usize>>,
    used: Length,
    height: Length,
    paragraph_lengths: &[usize],
    headings: &BTreeSet<NodeId>,
    terminal: bool,
) -> Result<i64, ProductionBodyPaginationError> {
    if let Some(range) = range {
        Ok(page_breaks::candidate(
            items,
            range.start,
            range.end,
            used,
            height,
            paragraph_lengths,
            headings,
            terminal,
        )?
        .costs()
        .total())
    } else if terminal {
        Ok(0)
    } else {
        let root = NodeId::new(0);
        let slack = height
            .checked_sub(used)
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        i64::try_from(i128::from(slack.raw()) * 1_000_000 / i128::from(height.raw()))
            .map_err(|_| error(root, E::ArithmeticOverflow))
    }
}
