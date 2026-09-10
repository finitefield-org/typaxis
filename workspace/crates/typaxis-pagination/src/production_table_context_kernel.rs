//! Prepare all source tables under one cumulative record/work allowance.
use super::*;

pub(super) fn prepare_searches<S>(
    tables: &[table_collection::Table],
    mut charge: Charge,
    mut work: Work,
    mut prepare: impl FnMut(usize, Charge, Work) -> Result<S, ProductionBodyPaginationError>,
    mut release: impl FnMut(&mut S) -> (Charge, u64),
) -> Result<(Vec<S>, Charge, u64), ProductionBodyPaginationError> {
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
        let maximum = work.maximum;
        let mut search = prepare(index, charge, work)?;
        let (remaining, used) = release(&mut search);
        charge = remaining;
        work = Work { used, maximum };
        searches.push(search);
    }
    Ok((searches, charge, work.used))
}
