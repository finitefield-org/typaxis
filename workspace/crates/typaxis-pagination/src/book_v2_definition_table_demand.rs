//! Source-bound definition-table continuation and retained reference demands.
//! Footnote region reservation, neighboring content and page placement are separate.
use super::*;
use crate::production_body::body_flow::book_v2::{
    BookV2FootnoteDemandSearch, BookV2FootnoteDemandState,
};
use crate::production_body::body_flow::footnote_breaks::prepare_book_v2_definition_candidate_demand;

pub struct BookV2DefinitionTableDemandSearch<'m, 'f, 's, 'p, 'a> {
    table: BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a>,
    notes: BookV2FootnoteDemandSearch<'m, 'f, 's, 'p, 'a>,
    definition: usize,
}
pub struct BookV2DefinitionTableDemandState<'m, 'f, 's, 'p, 'a> {
    table: BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
    demand: BookV2FootnoteDemandState<'m, 'f, 's, 'p, 'a>,
}
impl<'m, 'f, 's, 'p, 'a> BookV2DefinitionTableDemandState<'m, 'f, 's, 'p, 'a> {
    pub fn table_cursor(&self) -> BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
        self.table
    }
    pub fn demand(&self) -> &BookV2FootnoteDemandState<'m, 'f, 's, 'p, 'a> {
        &self.demand
    }
    pub fn table_complete(&self) -> bool {
        self.table.is_terminal()
    }
}
pub struct BookV2DefinitionTableDemandSelection<'m, 'f, 's, 'p, 'a> {
    source: (u64, u64),
    table: BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>,
    next: BookV2DefinitionTableDemandState<'m, 'f, 's, 'p, 'a>,
}
impl<'m, 'f, 's, 'p, 'a> BookV2DefinitionTableDemandSelection<'m, 'f, 's, 'p, 'a> {
    pub fn table(&self) -> &BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a> {
        &self.table
    }
    pub fn next_state(&self) -> &BookV2DefinitionTableDemandState<'m, 'f, 's, 'p, 'a> {
        &self.next
    }
    pub fn into_next_state(self) -> BookV2DefinitionTableDemandState<'m, 'f, 's, 'p, 'a> {
        self.next
    }
    pub fn verify(
        &self,
        state: &BookV2DefinitionTableDemandState<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.source != state.demand.table_snapshot_id() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}
impl<'m, 'f, 's, 'p, 'a> BookV2DefinitionTableDemandSearch<'m, 'f, 's, 'p, 'a> {
    pub fn record_charge(&self) -> u64 {
        self.notes.record_charge()
    }
    pub fn work_charge(&self) -> u64 {
        self.notes.work_steps()
    }
    pub fn maximum_height(&self) -> Length {
        self.table.maximum_height()
    }
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
    /// Acquire the definition only from actual ordinary body reference items.
    /// This does not assign those items or the resulting note to a physical page.
    pub fn begin(
        &mut self,
        body: Range<usize>,
    ) -> Result<BookV2DefinitionTableDemandState<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let initial = self.notes.begin()?;
        let demand = self.notes.require_body(&initial, body)?;
        if demand.status(self.definition) != Some(ProductionFootnoteDemandStatus::Pending) {
            return Err(error(
                self.table.measurements.tables()[self.table.table_index].owner,
                E::ReceiptMismatch,
            ));
        }
        let table = self.with_table(|search| search.begin())?;
        Ok(BookV2DefinitionTableDemandState { table, demand })
    }
    pub fn evaluate(
        &mut self,
        state: &BookV2DefinitionTableDemandState<'m, 'f, 's, 'p, 'a>,
        available: Length,
    ) -> Result<
        Option<BookV2DefinitionTableDemandSelection<'m, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        self.notes.verify_table_state(&state.demand)?;
        if state.table_complete() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        let Some(table) = self.with_table(|search| search.evaluate(&state.table, available))?
        else {
            return Ok(None);
        };
        let Some(demand) = self.notes.advance_definition_table(&state.demand, &table)? else {
            return Ok(None);
        };
        let next = BookV2DefinitionTableDemandState {
            table: table.after(),
            demand,
        };
        Ok(Some(BookV2DefinitionTableDemandSelection {
            source: state.demand.table_snapshot_id(),
            table,
            next,
        }))
    }
}
pub fn prepare_book_v2_definition_table_demand_search<'m, 'f, 's, 'p, 'a>(
    measurements: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2DefinitionTableDemandSearch<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let source = measurements
        .flow()
        .collected
        .tables
        .tables
        .get(table_index)
        .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
    let definition = source
        .definition
        .filter(|_| source.parent.is_none())
        .ok_or_else(|| error(source.owner, E::ReceiptMismatch))?;
    let mut table = prepare_book_v2_table_search(
        measurements,
        table_index,
        limits,
        maximum_work,
        prior_records,
    )?;
    table.kernel.charge.take(1, source.owner)?;
    let charge = std::mem::replace(&mut table.kernel.charge, Charge { remaining: 0 });
    let steps = std::mem::replace(&mut table.kernel.work.used, 0);
    let notes = prepare_book_v2_definition_candidate_demand(
        measurements.flow(),
        limits,
        maximum_work,
        charge,
        steps,
    )?;
    Ok(BookV2DefinitionTableDemandSearch {
        table,
        notes,
        definition,
    })
}
