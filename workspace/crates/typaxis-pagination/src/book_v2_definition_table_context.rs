//! Original table indexes scoped to one footnote definition.
use super::*;

pub(in crate::production_body::body_flow) struct BookV2DefinitionTableContext<'b, 'f, 's, 'p, 'a> {
    measurements: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
    definition: usize,
    first: usize,
    searches: Vec<BookV2TableBreakSearch<'b, 'f, 's, 'p, 'a>>,
    hierarchy: std::sync::Arc<body_context::hierarchy::Hierarchy>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2DefinitionTableContext<'b, 'f, 's, 'p, 'a> {
    pub fn prepare_counted(
        measurements: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
        definition: usize,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        observed_records: &mut u64,
        observed_work: &mut u64,
    ) -> Result<(Self, Charge, u64), ProductionBodyPaginationError> {
        *observed_records = prior_records.max(measurements.record_charge());
        *observed_work = 0;
        let root = NodeId::new(0);
        let flow = measurements.flow();
        flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)?;
        if flow.definition_items(definition).is_none() {
            return Err(error(root, E::ReceiptMismatch));
        }
        let mut charge = Charge {
            remaining: limits
                .base()
                .get()
                .max_fragments
                .checked_sub(prior_records.max(measurements.record_charge()))
                .ok_or_else(|| error(root, E::FragmentLimit))?,
        };
        let mut work = Work {
            used: 0,
            maximum: maximum_work,
        };
        let result = (|| {
            let tables = &flow.collected.tables.tables;
            charge.take(1, root)?;
            let hierarchy = std::sync::Arc::new(body_context::hierarchy::Hierarchy::prepare(
                tables,
                &mut charge,
                &mut work,
            )?);
            let mut first = tables.len();
            let mut count = 0usize;
            for (index, table) in tables.iter().enumerate() {
                work.take(1, table.owner)?;
                if table.definition == Some(definition) {
                    if count == 0 {
                        first = index;
                    }
                    if index != first + count {
                        return Err(error(table.owner, E::ReceiptMismatch));
                    }
                    count += 1;
                }
            }
            let context = Self::from_range(
                measurements,
                definition,
                first,
                count,
                limits,
                &mut charge,
                &mut work,
                hierarchy,
            )?;
            Ok((
                context,
                Charge {
                    remaining: charge.remaining,
                },
                work.used,
            ))
        })();
        *observed_records = limits.base().get().max_fragments - charge.remaining;
        *observed_work = work.used;
        result
    }

    fn from_range(
        measurements: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
        definition: usize,
        first: usize,
        count: usize,
        limits: &M4EffectiveResourceLimits,
        charge: &mut Charge,
        work: &mut Work,
        hierarchy: std::sync::Arc<body_context::hierarchy::Hierarchy>,
    ) -> Result<Self, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let tables = &measurements.flow().collected.tables.tables;
        charge.take(
            count
                .checked_add(1)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut searches = Vec::new();
        searches
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        for index in first..first + count {
            work.take(1, tables[index].owner)?;
            let search =
                prepare_book_v2_table_search_borrowed(measurements, index, limits, charge, work)?;
            searches.push(search);
        }
        Ok(Self {
            measurements,
            definition,
            first,
            searches,
            hierarchy,
        })
    }
    pub fn prepare_all_counted(
        measurements: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
        observed_records: &mut u64,
        observed_work: &mut u64,
    ) -> Result<(Vec<Option<Self>>, Charge, u64), ProductionBodyPaginationError> {
        *observed_records = prior_records.max(measurements.record_charge());
        *observed_work = 0;
        let root = NodeId::new(0);
        let flow = measurements.flow();
        flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)?;
        let mut charge = Charge {
            remaining: limits
                .base()
                .get()
                .max_fragments
                .checked_sub(prior_records.max(measurements.record_charge()))
                .ok_or_else(|| error(root, E::FragmentLimit))?,
        };
        let mut work = Work {
            used: 0,
            maximum: maximum_work,
        };
        let result = (|| {
            charge.take(1, root)?;
            let tables = &flow.collected.tables.tables;
            let hierarchy = std::sync::Arc::new(body_context::hierarchy::Hierarchy::prepare(
                tables,
                &mut charge,
                &mut work,
            )?);
            let mut next = 0;
            while next < tables.len() && tables[next].definition.is_none() {
                work.take(1, tables[next].owner)?;
                next += 1;
            }
            let contexts = Self::prepare_remaining(
                measurements,
                limits,
                &mut charge,
                &mut work,
                hierarchy,
                next,
            )?;
            Ok((
                contexts,
                Charge {
                    remaining: charge.remaining,
                },
                work.used,
            ))
        })();
        *observed_records = limits.base().get().max_fragments - charge.remaining;
        *observed_work = work.used;
        result
    }

    pub(super) fn prepare_remaining(
        measurements: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        charge: &mut Charge,
        work: &mut Work,
        hierarchy: std::sync::Arc<body_context::hierarchy::Hierarchy>,
        mut next: usize,
    ) -> Result<Vec<Option<Self>>, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let flow = measurements.flow();
        let tables = &flow.collected.tables.tables;
        let count = flow.collected.definitions.len();
        charge.take(
            count
                .checked_add(1)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut contexts = Vec::new();
        contexts
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        for definition in 0..count {
            work.take(1, root)?;
            let first = next;
            while next < tables.len() && tables[next].definition == Some(definition) {
                work.take(1, tables[next].owner)?;
                next += 1;
            }
            let context = Self::from_range(
                measurements,
                definition,
                first,
                next - first,
                limits,
                charge,
                work,
                hierarchy.clone(),
            )?;
            contexts.push(Some(context));
        }
        if next != tables.len() {
            return Err(error(root, E::ReceiptMismatch));
        }
        Ok(contexts)
    }
    pub fn first(&self) -> usize {
        self.first
    }
    pub fn end(&self) -> usize {
        self.first + self.searches.len()
    }
    pub fn successor(&self, index: usize) -> Result<usize, ProductionBodyPaginationError> {
        self.range(index)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        self.hierarchy
            .successor(index)
            .filter(|next| *next <= self.end())
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))
    }
    pub fn previous_root(&self, index: usize) -> Option<usize> {
        (index <= self.end())
            .then(|| self.hierarchy.previous_root(index))
            .flatten()
            .filter(|previous| *previous >= self.first)
    }
    pub fn range(&self, index: usize) -> Option<Range<usize>> {
        self.measurements
            .flow()
            .collected
            .tables
            .tables
            .get(index)
            .filter(|table| table.definition == Some(self.definition) && table.parent.is_none())
            .map(|table| {
                let base = self.measurements.flow().collected.definitions[self.definition].start;
                table.items.start - base..table.items.end - base
            })
    }
    pub fn table(&self, index: usize) -> &ProductionMeasuredTable {
        &self.measurements.tables()[index]
    }
    fn search(
        &mut self,
        index: usize,
    ) -> Result<&mut BookV2TableBreakSearch<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        if self.range(index).is_none() {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        self.searches
            .get_mut(index - self.first)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))
    }
    pub fn begin(
        &mut self,
        index: usize,
        charge: &mut Charge,
        steps: &mut u64,
    ) -> Result<BookV2TableCursor<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let search = self.search(index)?;
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        let result = search.begin();
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        result
    }
    pub fn earlier_capacity(
        &mut self,
        cursor: &BookV2TableCursor<'b, 'f, 's, 'p, 'a>,
        selected: &BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>,
        steps: &mut u64,
    ) -> Result<Option<Length>, ProductionBodyPaginationError> {
        self.search(cursor.table_index())?
            .earlier_capacity(cursor, selected, steps)
    }
    pub fn evaluate(
        &mut self,
        cursor: &BookV2TableCursor<'b, 'f, 's, 'p, 'a>,
        capacity: Length,
        headers: Option<&'b crate::book_v2::BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a>>,
        region_width: Option<PositiveLength>,
        page_name: Option<usize>,
        charge: &mut Charge,
        steps: &mut u64,
    ) -> Result<
        Option<BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let search = self.search(cursor.table_index())?;
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        let result = search.evaluate_named_in_frame(cursor, capacity, headers, region_width, page_name);
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        result
    }
    pub fn page_name(
        &mut self,
        index: usize,
        cursor: Option<&BookV2TableCursor<'b, 'f, 's, 'p, 'a>>,
        preferred: Option<Option<usize>>,
        charge: &mut Charge,
        steps: &mut u64,
    ) -> Result<Option<usize>, ProductionBodyPaginationError> {
        let search = self.search(index)?;
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        let result = search.cursor_page_name(cursor, preferred);
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        result
    }
}
