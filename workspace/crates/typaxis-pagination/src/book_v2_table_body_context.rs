//! Source-bound inactive table searches for the successor book cursor.
use super::*;
#[path = "book_v2_table_hierarchy.rs"]
pub(super) mod hierarchy;

pub(in crate::production_body::body_flow) struct BookV2TableBodyContext<'b, 'f, 's, 'p, 'a> {
    measurements: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
    searches: Vec<BookV2TableBreakSearch<'b, 'f, 's, 'p, 'a>>,
    hierarchy: HierarchyOwner,
}
enum HierarchyOwner {
    Owned(hierarchy::Hierarchy),
    Shared(std::sync::Arc<hierarchy::Hierarchy>),
}
impl std::ops::Deref for HierarchyOwner {
    type Target = hierarchy::Hierarchy;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(value) => value,
            Self::Shared(value) => value,
        }
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2TableBodyContext<'b, 'f, 's, 'p, 'a> {
    pub fn prepare(
        measurements: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
    ) -> Result<(Self, Charge, u64), ProductionBodyPaginationError> {
        let flow = measurements.flow();
        flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)?;
        let mut charge = Charge {
            remaining: limits
                .base()
                .get()
                .max_fragments
                .checked_sub(prior_records.max(measurements.record_charge()))
                .ok_or_else(|| error(NodeId::new(0), E::FragmentLimit))?,
        };
        let mut work = Work {
            used: 0,
            maximum: maximum_work,
        };
        let hierarchy =
            hierarchy::Hierarchy::prepare(&flow.collected.tables.tables, &mut charge, &mut work)?;
        let (searches, charge, steps) = context_kernel::prepare_searches(
            &flow.collected.tables.tables,
            charge,
            work,
            |index, charge, work| {
                prepare_book_v2_table_search_charged(measurements, index, limits, charge, work)
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
                hierarchy: HierarchyOwner::Owned(hierarchy),
            },
            charge,
            steps,
        ))
    }
    pub fn prepare_joint(
        measurements: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
        maximum_work: u64,
        prior_records: u64,
    ) -> Result<
        (
            Self,
            Vec<Option<BookV2DefinitionTableContext<'b, 'f, 's, 'p, 'a>>>,
            Charge,
            u64,
        ),
        ProductionBodyPaginationError,
    > {
        let flow = measurements.flow();
        flow.verify(flow.lines(), flow.blocks(), flow.footnotes(), limits)?;
        let root = NodeId::new(0);
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
        charge.take(1, root)?;
        let sources = &flow.collected.tables.tables;
        let hierarchy = std::sync::Arc::new(hierarchy::Hierarchy::prepare(
            sources,
            &mut charge,
            &mut work,
        )?);
        let mut body_end = 0;
        while body_end < sources.len() && sources[body_end].definition.is_none() {
            work.take(1, sources[body_end].owner)?;
            body_end += 1;
        }
        let (searches, charge, used) = context_kernel::prepare_searches(
            &sources[..body_end],
            charge,
            work,
            |index, charge, work| {
                prepare_book_v2_table_search_charged(measurements, index, limits, charge, work)
            },
            |search| {
                (
                    std::mem::replace(&mut search.kernel.charge, Charge { remaining: 0 }),
                    std::mem::replace(&mut search.kernel.work.used, 0),
                )
            },
        )?;
        let (definitions, charge, used) = BookV2DefinitionTableContext::prepare_remaining(
            measurements,
            limits,
            charge,
            Work {
                used,
                maximum: maximum_work,
            },
            hierarchy.clone(),
            body_end,
        )?;
        Ok((
            Self {
                measurements,
                searches,
                hierarchy: HierarchyOwner::Shared(hierarchy),
            },
            definitions,
            charge,
            used,
        ))
    }
    pub fn measurements_fingerprint(&self) -> [u8; 32] {
        self.measurements.fingerprint()
    }
    pub fn earlier_capacity(
        &mut self,
        cursor: &BookV2TableCursor<'b, 'f, 's, 'p, 'a>,
        selected: &BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>,
        steps: &mut u64,
    ) -> Result<Option<Length>, ProductionBodyPaginationError> {
        let search = self
            .searches
            .get_mut(cursor.table_index())
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        search.earlier_capacity(cursor, selected, steps)
    }
    pub fn len(&self) -> usize {
        self.searches.len()
    }
    pub fn successor(&self, index: usize) -> usize {
        self.hierarchy.successor(index).expect("bound source table")
    }
    pub fn previous_root(&self, index: usize) -> Option<usize> {
        self.hierarchy.previous_root(index)
    }
    pub fn range(&self, index: usize) -> Option<Range<usize>> {
        self.measurements
            .flow()
            .collected
            .tables
            .tables
            .get(index)
            .filter(|t| t.parent.is_none() && t.definition.is_none())
            .map(|t| t.items.clone())
    }
    pub fn table(&self, index: usize) -> &ProductionMeasuredTable {
        &self.measurements.tables()[index]
    }
    pub fn at(&self, item: usize, index: usize) -> Option<usize> {
        self.range(index)
            .filter(|range| range.start == item)
            .map(|_| index)
    }
    pub fn ordinary_range(&self, range: Range<usize>, index: usize) -> bool {
        self.range(index).is_none_or(|next| next.start >= range.end)
    }
    pub fn evaluate(
        &mut self,
        cursor: &BookV2TableCursor<'b, 'f, 's, 'p, 'a>,
        capacity: Length,
        headers: Option<&'b crate::book_v2::BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a>>,
        region_width: Option<PositiveLength>,
        charge: &mut Charge,
        steps: &mut u64,
    ) -> Result<
        Option<BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let search = self
            .searches
            .get_mut(cursor.table_index())
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        let result = search.evaluate_in_frame(cursor, capacity, headers, region_width);
        std::mem::swap(&mut search.kernel.charge, charge);
        std::mem::swap(&mut search.kernel.work.used, steps);
        result
    }
    pub fn begin(
        &mut self,
        index: usize,
        charge: &mut Charge,
        steps: &mut u64,
    ) -> Result<BookV2TableCursor<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
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
}
