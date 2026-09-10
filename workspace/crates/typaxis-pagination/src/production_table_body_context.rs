//! Source table boundaries and inactive table searches for the common page owner.
use super::*;

pub(in crate::production_body::body_flow) struct ProductionTableBodyContext<'b, 'f, 's, 'p, 'a> {
    measurements: &'b ProductionTableMeasurements<'f, 's, 'p, 'a>,
    searches: Vec<ProductionTableBreakSearch<'b, 'f, 's, 'p, 'a>>,
}
impl<'b, 'f, 's, 'p, 'a> ProductionTableBodyContext<'b, 'f, 's, 'p, 'a> {
    pub fn prepare(
        measurements: &'b ProductionTableMeasurements<'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
    ) -> Result<(Self, Charge, u64), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        measurements.verify(measurements.flow.lines, measurements.flow.blocks, limits)?;
        let charge = Charge {
            remaining: limits
                .base()
                .get()
                .max_fragments
                .checked_sub(measurements.record_charge)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
        };
        let work = Work {
            used: 0,
            maximum: maximum_work,
        };
        let (searches, charge, steps) = context_kernel::prepare_searches(
            &measurements.flow.collected.tables.tables,
            charge,
            work,
            |index, charge, work| {
                prepare_table_search_charged(measurements, index, limits, charge, work)
            },
            |search| {
                (
                    std::mem::replace(&mut search.kernel.charge, Charge { remaining: 0 }),
                    std::mem::replace(&mut search.kernel.work.used, 0),
                )
            },
        )?;
        Ok((
            Self {
                measurements,
                searches,
            },
            charge,
            steps,
        ))
    }
    pub fn is_empty(&self) -> bool {
        self.searches.is_empty()
    }
    pub fn measurements_fingerprint(&self) -> [u8; 32] {
        self.measurements.fingerprint()
    }
    pub fn query_steps(&self) -> u32 {
        usize::BITS - self.searches.len().max(1).leading_zeros() + 1
    }
    pub fn keep_before(&self, item: usize) -> bool {
        self.measurements
            .flow
            .collected
            .tables
            .tables
            .binary_search_by_key(&item, |t| t.items.end)
            .ok()
            .is_some_and(|index| self.measurements.tables[index].keep_with_next())
    }
    pub fn at(&self, item: usize) -> Option<usize> {
        self.measurements
            .flow
            .collected
            .tables
            .tables
            .binary_search_by_key(&item, |t| t.items.start)
            .ok()
    }
    pub fn range(&self, index: usize) -> Range<usize> {
        self.measurements.flow.collected.tables.tables[index]
            .items
            .clone()
    }
    pub fn table(&self, index: usize) -> &ProductionMeasuredTable {
        &self.measurements.tables[index]
    }
    pub fn contains(&self, range: Range<usize>) -> bool {
        if range.is_empty() {
            return false;
        }
        let tables = &self.measurements.flow.collected.tables.tables;
        let first = tables.partition_point(|t| t.items.end <= range.start);
        tables.get(first).is_some_and(|t| t.items.start < range.end)
    }
    pub fn begin(
        &mut self,
        index: usize,
        charge: &mut Charge,
        steps: &mut u64,
    ) -> Result<ProductionTableCursor<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let search = self
            .searches
            .get_mut(index)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        let result = search.begin();
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        result
    }
    pub fn evaluate(
        &mut self,
        cursor: &ProductionTableCursor<'b, 'f, 's, 'p, 'a>,
        available: Length,
        charge: &mut Charge,
        steps: &mut u64,
    ) -> Result<
        Option<ProductionTableFragmentSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let search = &mut self.searches[cursor.table_index()];
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        let result = search.evaluate(cursor, available);
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        result
    }
    /// Earlier semantic/row event below a selected cut. Padding has no source
    /// event per length unit; do not turn a long blank band into millions of
    /// almost-identical alternatives. The current capacity itself is tried first.
    pub fn earlier_capacity(
        &mut self,
        cursor: &ProductionTableCursor<'b, 'f, 's, 'p, 'a>,
        selected: &ProductionTableFragmentSelection<'b, 'f, 's, 'p, 'a>,
        steps: &mut u64,
    ) -> Result<Option<Length>, ProductionBodyPaginationError> {
        let search = &mut self.searches[cursor.table_index()];
        std::mem::swap(&mut search.kernel.work.used, steps);
        let result = search
            .kernel
            .earlier_capacity(cursor.offset(), selected.after().offset());
        std::mem::swap(&mut search.kernel.work.used, steps);
        result
    }
}
