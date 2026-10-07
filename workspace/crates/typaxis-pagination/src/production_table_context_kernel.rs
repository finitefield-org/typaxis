//! Prepare all source tables under one cumulative record/work allowance.
use super::*;

pub(super) fn prepare_searches<S>(
    tables: &[table_collection::Table],
    mut charge: Charge,
    mut work: Work,
    mut prepare: impl FnMut(usize, Charge, Work) -> Result<S, ProductionBodyPaginationError>,
    mut release: impl FnMut(&mut S) -> (Charge, u64),
) -> Result<(Vec<S>, Charge, u64), ProductionBodyPaginationError> {
    let searches =
        prepare_searches_borrowed(tables, &mut charge, &mut work, |index, charge, work| {
            let mut search = prepare(
                index,
                Charge {
                    remaining: charge.remaining,
                },
                Work {
                    used: work.used,
                    maximum: work.maximum,
                },
            )?;
            let (remaining, used) = release(&mut search);
            *charge = remaining;
            work.used = used;
            Ok(search)
        })?;
    Ok((searches, charge, work.used))
}

/// Keep accepted reservations and traversal work in the caller's ledger even
/// when an individual constructor fails. The callback must do the same.
pub(super) fn prepare_searches_borrowed<S>(
    tables: &[table_collection::Table],
    charge: &mut Charge,
    work: &mut Work,
    mut prepare: impl FnMut(usize, &mut Charge, &mut Work) -> Result<S, ProductionBodyPaginationError>,
) -> Result<Vec<S>, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    charge.take(
        tables
            .len()
            .checked_add(1)
            .ok_or_else(|| error(root, E::FragmentLimit))?,
        root,
    )?;
    let mut searches = Vec::new();
    searches
        .try_reserve_exact(tables.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    for (index, table) in tables.iter().enumerate() {
        work.take(1, table.owner)?;
        if table.definition.is_some() {
            return Err(error(
                table.owner,
                E::PendingRegion("table_footnote_definition"),
            ));
        }
        let search = prepare(index, charge, work)?;
        searches.push(search);
    }
    Ok(searches)
}
