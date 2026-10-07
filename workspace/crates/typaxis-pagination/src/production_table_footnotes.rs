//! Table continuation and the common same-page footnote fit, under one budget.
//! This is region selection; shared book cursor/placement remains its caller.
use super::super::super::footnote_breaks::demand::ProductionBodyFootnoteFit;
use super::*;

pub struct ProductionTableFootnoteSearch<'m, 'f, 's, 'p, 'a> {
    table: ProductionTableBreakSearch<'m, 'f, 's, 'p, 'a>,
    notes: ProductionFootnoteDemandSearch<'m, 'f, 's, 'p, 'a>,
}
pub struct ProductionTableFootnoteState<'m, 'f, 's, 'p, 'a> {
    table: ProductionTableCursor<'m, 'f, 's, 'p, 'a>,
    demand: ProductionFootnoteDemandState<'m, 'f, 's, 'p, 'a>,
}
impl<'m, 'f, 's, 'p, 'a> ProductionTableFootnoteState<'m, 'f, 's, 'p, 'a> {
    pub const fn table_cursor(&self) -> ProductionTableCursor<'m, 'f, 's, 'p, 'a> {
        self.table
    }
    pub fn demand(&self) -> &ProductionFootnoteDemandState<'m, 'f, 's, 'p, 'a> {
        &self.demand
    }
    pub fn is_complete(&self) -> bool {
        self.table.is_terminal() && self.demand.pending_definitions().is_empty()
    }
}
pub struct ProductionTableFootnoteSelection<'m, 'f, 's, 'p, 'a> {
    table: Option<ProductionTableFragmentSelection<'m, 'f, 's, 'p, 'a>>,
    fit: ProductionBodyFootnoteFit<'m, 'f, 's, 'p, 'a>,
    next: ProductionTableFootnoteState<'m, 'f, 's, 'p, 'a>,
}
impl<'m, 'f, 's, 'p, 'a> ProductionTableFootnoteSelection<'m, 'f, 's, 'p, 'a> {
    pub fn table(&self) -> Option<&ProductionTableFragmentSelection<'m, 'f, 's, 'p, 'a>> {
        self.table.as_ref()
    }
    pub fn footnotes(&self) -> Option<&ProductionFootnoteRegionSelection<'m, 'f, 's, 'p, 'a>> {
        self.fit.footnotes()
    }
    pub fn footnote_bounds(&self) -> Option<Rect> {
        self.fit.footnote_bounds()
    }
    pub fn next_state(&self) -> &ProductionTableFootnoteState<'m, 'f, 's, 'p, 'a> {
        &self.next
    }
    pub fn verify(
        &self,
        state: &ProductionTableFootnoteState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.fit.verify(&state.demand)
    }
}
impl<'m, 'f, 's, 'p, 'a> ProductionTableFootnoteSearch<'m, 'f, 's, 'p, 'a> {
    pub fn record_charge(&self) -> u64 {
        self.notes.record_charge()
    }
    pub fn work_charge(&self) -> u64 {
        self.notes.work_steps()
    }
    /// Move the sole remaining budget into the table operation and back even
    /// on failure. Neither search owns a replenished or independent allowance.
    fn with_table<T>(
        &mut self,
        f: impl FnOnce(
            &mut ProductionTableBreakSearch<'m, 'f, 's, 'p, 'a>,
        ) -> Result<T, ProductionBodyPaginationError>,
    ) -> Result<T, ProductionBodyPaginationError> {
        self.notes.swap_table_budget(
            &mut self.table.kernel.charge,
            &mut self.table.kernel.work.used,
        );
        let result = f(&mut self.table);
        self.notes.swap_table_budget(
            &mut self.table.kernel.charge,
            &mut self.table.kernel.work.used,
        );
        result
    }
    pub fn begin(
        &mut self,
    ) -> Result<ProductionTableFootnoteState<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let table = self.with_table(|search| search.begin())?;
        let demand = self.notes.begin()?;
        Ok(ProductionTableFootnoteState { table, demand })
    }
    /// Try this table capacity at the common body origin, then reserve the
    /// actual selected references with the common same-page footnote policy.
    /// None leaves the input state unchanged, allowing a smaller table cut.
    /// Table outside spacing, neighboring body items and page ranking belong
    /// to the shared book owner, not this table-region query.
    pub fn evaluate(
        &mut self,
        state: &ProductionTableFootnoteState<'m, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<ProductionTableFootnoteSelection<'m, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.notes.verify_table_state(&state.demand)?;
        if available < Length::ZERO || available > self.table.kernel.maximum_height {
            return Err(error(NodeId::new(0), E::InvalidTableCapacity));
        }
        if state.is_complete() {
            return Ok(None);
        }
        let table = if state.table.is_terminal() {
            None
        } else {
            let fragment = self.with_table(|search| search.evaluate(&state.table, available))?;
            if fragment.is_none() && state.demand.pending_definitions().is_empty() {
                return Ok(None);
            }
            fragment
        };
        let Some(fit) = self
            .notes
            .evaluate_table_demand(&state.demand, table.as_ref())?
        else {
            return Ok(None);
        };
        let next = ProductionTableFootnoteState {
            table: table.as_ref().map_or(state.table, |f| f.after()),
            demand: self.notes.fork_table_state(fit.next_state())?,
        };
        Ok(Some(ProductionTableFootnoteSelection { table, fit, next }))
    }
}
pub fn prepare_production_table_footnote_search<'m, 'f, 's, 'p, 'a>(
    measurements: &'m ProductionTableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
) -> Result<ProductionTableFootnoteSearch<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    // The shared definition selector still consumes ordinary leaves. Do not
    // authorize it to flatten a table inside a definition.
    let table_visits = measurements.flow.collected.tables.tables.len() as u64;
    let remaining_work = maximum_work
        .checked_sub(table_visits)
        .ok_or_else(|| error(NodeId::new(0), E::TableSearchLimit))?;
    for table in &measurements.flow.collected.tables.tables {
        if table.definition.is_some() {
            return Err(error(
                table.owner,
                E::PendingRegion("table_footnote_definition"),
            ));
        }
    }
    let mut table =
        prepare_production_table_search(measurements, table_index, limits, remaining_work)?;
    table.kernel.work.maximum = maximum_work;
    table.kernel.work.used += table_visits;
    table
        .kernel
        .charge
        .take(1, measurements.tables[table_index].owner)?;
    let charge = std::mem::replace(&mut table.kernel.charge, Charge { remaining: 0 });
    let steps = std::mem::replace(&mut table.kernel.work.used, 0);
    let notes = super::super::super::footnote_breaks::demand::prepare_table_demand_search(
        &measurements.flow,
        limits,
        maximum_work,
        charge,
        steps,
    )?;
    Ok(ProductionTableFootnoteSearch { table, notes })
}
