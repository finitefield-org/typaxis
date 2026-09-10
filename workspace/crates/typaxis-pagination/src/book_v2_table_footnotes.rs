//! Table continuation and the common same-page footnote fit, under one budget.
//! This is region selection; shared book cursor/placement remains its caller.
use super::*;
use crate::production_body::body_flow::book_v2::{
    BookV2FootnoteDemandSearch, BookV2FootnoteDemandState, BookV2FootnoteRegionSelection,
};
use crate::production_body::body_flow::footnote_breaks::{
    prepare_book_v2_table_demand_search, BookV2BodyFootnoteFit,
};

pub struct BookV2TableFootnoteSearch<'m, 'f, 's, 'p, 'a> {
    table: BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a>,
    notes: BookV2FootnoteDemandSearch<'m, 'f, 's, 'p, 'a>,
}
pub struct BookV2TableFootnoteState<'m, 'f, 's, 'p, 'a> {
    table: BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
    demand: BookV2FootnoteDemandState<'m, 'f, 's, 'p, 'a>,
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableFootnoteState<'m, 'f, 's, 'p, 'a> {
    pub const fn table_cursor(&self) -> BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
        self.table
    }
    pub fn demand(&self) -> &BookV2FootnoteDemandState<'m, 'f, 's, 'p, 'a> {
        &self.demand
    }
    pub fn is_complete(&self) -> bool {
        self.table.is_terminal() && self.demand.pending_definitions().is_empty()
    }
}
pub struct BookV2TableFootnoteSelection<'m, 'f, 's, 'p, 'a> {
    table: Option<BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>>,
    fit: BookV2BodyFootnoteFit<'m, 'f, 's, 'p, 'a>,
    next: BookV2TableFootnoteState<'m, 'f, 's, 'p, 'a>,
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableFootnoteSelection<'m, 'f, 's, 'p, 'a> {
    pub fn table(&self) -> Option<&BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>> {
        self.table.as_ref()
    }
    pub fn footnotes(&self) -> Option<&BookV2FootnoteRegionSelection<'m, 'f, 's, 'p, 'a>> {
        self.fit.footnotes()
    }
    pub fn footnote_bounds(&self) -> Option<Rect> {
        self.fit.footnote_bounds()
    }
    pub fn next_state(&self) -> &BookV2TableFootnoteState<'m, 'f, 's, 'p, 'a> {
        &self.next
    }
    pub fn verify(
        &self,
        state: &BookV2TableFootnoteState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.fit.verify(&state.demand)
    }
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableFootnoteSearch<'m, 'f, 's, 'p, 'a> {
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
            &mut BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a>,
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
    ) -> Result<BookV2TableFootnoteState<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let table = self.with_table(|search| search.begin())?;
        let demand = self.notes.begin()?;
        Ok(BookV2TableFootnoteState { table, demand })
    }
    /// Try this table capacity at the common body origin, then reserve the
    /// actual selected references with the common same-page footnote policy.
    /// None leaves the input state unchanged, allowing a smaller table cut.
    /// Table outside spacing, neighboring body items and page ranking belong
    /// to the shared book owner, not this table-region query.
    pub fn evaluate(
        &mut self,
        state: &BookV2TableFootnoteState<'m, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<BookV2TableFootnoteSelection<'m, 'f, 's, 'p, 'a>>,
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
        let next = BookV2TableFootnoteState {
            table: table.as_ref().map_or(state.table, |f| f.after()),
            demand: self.notes.fork_table_state(fit.next_state())?,
        };
        Ok(Some(BookV2TableFootnoteSelection { table, fit, next }))
    }
}
pub fn prepare_book_v2_table_footnote_search<'m, 'f, 's, 'p, 'a>(
    measurements: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2TableFootnoteSearch<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let mut table = prepare_book_v2_table_search(
        measurements,
        table_index,
        limits,
        maximum_work,
        prior_records,
    )?;
    table
        .kernel
        .charge
        .take(1, measurements.tables()[table_index].owner)?;
    let charge = std::mem::replace(&mut table.kernel.charge, Charge { remaining: 0 });
    let steps = std::mem::replace(&mut table.kernel.work.used, 0);
    let notes = prepare_book_v2_table_demand_search(
        measurements.flow(),
        limits,
        maximum_work,
        charge,
        steps,
    )?;
    Ok(BookV2TableFootnoteSearch { table, notes })
}
