//! Source-preorder subtree boundaries, independent of table heights or leaf counts.
use super::*;

pub(in crate::production_body::body_flow) struct Hierarchy {
    // Empty vectors retain the ordinary flat-table representation.
    ends: Vec<usize>,
    previous_roots: Vec<Option<usize>>,
    count: usize,
}
impl Hierarchy {
    pub fn prepare(
        tables: &[table_collection::Table],
        charge: &mut Charge,
        work: &mut Work,
    ) -> Result<Self, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let count = tables.len();
        work.take(count as u64, root)?;
        if tables.iter().all(|table| table.parent.is_none()) {
            return Ok(Self {
                ends: Vec::new(),
                previous_roots: Vec::new(),
                count,
            });
        }
        let records = count
            .checked_mul(3)
            .and_then(|n| n.checked_add(4))
            .ok_or_else(|| error(root, E::FragmentLimit))?;
        charge.take(records, root)?;
        let mut ends = Vec::new();
        let mut previous_roots = Vec::new();
        let mut open: Vec<usize> = Vec::new();
        ends.try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        previous_roots
            .try_reserve_exact(count + 1)
            .map_err(|_| error(root, E::AllocationFailure))?;
        open.try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut previous_root = None;
        for (index, table) in tables.iter().enumerate() {
            work.take(1, table.owner)?;
            if table.parent.is_some_and(|parent| parent >= index)
                || table.items.start > table.items.end
            {
                return Err(error(table.owner, E::ReceiptMismatch));
            }
            while open.last().copied() != table.parent {
                work.take(1, table.owner)?;
                let closing = open
                    .pop()
                    .ok_or_else(|| error(table.owner, E::ReceiptMismatch))?;
                if tables[closing].definition == table.definition
                    && table.items.start < tables[closing].items.end
                {
                    return Err(error(table.owner, E::ReceiptMismatch));
                }
                ends[closing] = index;
            }
            if let Some(parent) = table.parent {
                let parent = &tables[parent];
                if parent.definition != table.definition
                    || table.items.start < parent.items.start
                    || table.items.end > parent.items.end
                {
                    return Err(error(table.owner, E::ReceiptMismatch));
                }
                previous_roots.push(None);
            } else {
                previous_roots.push(previous_root);
                previous_root = Some(index);
            }
            ends.push(count);
            open.push(index);
        }
        previous_roots.push(previous_root);
        Ok(Self {
            ends,
            previous_roots,
            count,
        })
    }
    pub fn successor(&self, index: usize) -> Option<usize> {
        (index < self.count).then(|| self.ends.get(index).copied().unwrap_or(index + 1))
    }
    pub fn previous_root(&self, index: usize) -> Option<usize> {
        if index > self.count {
            return None;
        }
        if self.previous_roots.is_empty() {
            index.checked_sub(1)
        } else {
            self.previous_roots[index]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn table(index: u32, parent: Option<usize>, range: Range<usize>) -> table_collection::Table {
        table_collection::Table {
            owner: NodeId::new(index),
            parent,
            definition: None,
            items: range,
            cells: Vec::new(),
            caption: None,
            before: Length::ZERO,
            after: Length::ZERO,
            keep: false,
            keep_together: false,
        }
    }
    fn prepare(
        tables: &[table_collection::Table],
        records: u64,
        maximum: u64,
    ) -> Result<(Hierarchy, u64, u64), ProductionBodyPaginationError> {
        let mut charge = Charge { remaining: records };
        let mut work = Work { used: 0, maximum };
        let tree = Hierarchy::prepare(tables, &mut charge, &mut work)?;
        Ok((tree, records - charge.remaining, work.used))
    }
    #[test]
    fn nested_table_hierarchy_skips_descendants_even_when_leaf_ranges_are_empty() {
        let tables = vec![
            table(0, None, 0..0),
            table(1, Some(0), 0..0),
            table(2, Some(1), 0..0),
            table(3, Some(0), 0..0),
            table(4, None, 0..0),
            table(5, Some(4), 0..0),
            table(6, None, 0..0),
        ];
        let (tree, records, work) = prepare(&tables, 100, 100).unwrap();
        assert_eq!(
            (0..7)
                .map(|i| tree.successor(i).unwrap())
                .collect::<Vec<_>>(),
            [4, 3, 3, 4, 6, 6, 7]
        );
        assert_eq!(tree.successor(7), None);
        assert_eq!(tree.previous_root(4), Some(0));
        assert_eq!(tree.previous_root(6), Some(4));
        assert_eq!(tree.previous_root(7), Some(6));
        assert_eq!(tree.previous_root(1), None);
        assert!(prepare(&tables, records, work).is_ok());
        assert!(matches!(prepare(&tables,records-1,work),Err(e) if e.kind == E::FragmentLimit));
        assert!(matches!(prepare(&tables,records,work-1),Err(e) if e.kind == E::TableSearchLimit));
    }
    #[test]
    fn nested_table_hierarchy_rejects_reopened_parent_or_wrong_region() {
        for mode in 0..4 {
            let mut tables = vec![
                table(0, None, 0..4),
                table(1, Some(0), 0..2),
                table(2, None, 4..8),
                table(3, Some(2), 4..6),
            ];
            match mode {
                0 => tables[3].parent = Some(0),
                1 => tables[3].parent = Some(3),
                2 => tables[3].definition = Some(0),
                _ => tables[3].items = 3..9,
            }
            assert!(
                matches!(prepare(&tables,100,100),Err(e) if e.owner==NodeId::new(3) && e.kind==E::ReceiptMismatch)
            );
        }
    }
    #[test]
    fn nested_table_hierarchy_keeps_parallel_source_ranges_disjoint() {
        let mut tables = vec![
            table(0, None, 0..10),
            table(1, Some(0), 1..4),
            table(2, Some(1), 2..3),
            table(3, Some(0), 4..8),
            table(4, None, 10..11),
        ];
        let (tree, _, _) = prepare(&tables, 100, 100).unwrap();
        assert_eq!(tree.successor(0), Some(4));
        assert_eq!(tree.successor(1), Some(3));
        tables[3].items.start = 3;
        assert!(
            matches!(prepare(&tables, 100, 100), Err(e) if e.owner == NodeId::new(3) && e.kind == E::ReceiptMismatch)
        );
    }
    #[test]
    fn nested_table_hierarchy_is_iterative_and_flat_tables_allocate_no_records() {
        let flat = vec![table(0, None, 0..1), table(1, None, 1..1)];
        let (tree, records, work) = prepare(&flat, 0, 2).unwrap();
        assert_eq!((records, work), (0, 2));
        assert_eq!(tree.successor(0), Some(1));
        assert_eq!(tree.previous_root(2), Some(1));
        let deep = (0usize..4096)
            .map(|i| table(i as u32, i.checked_sub(1), 0..0))
            .collect::<Vec<_>>();
        let (tree, _, work) = prepare(&deep, 20_000, 20_000).unwrap();
        assert_eq!(tree.successor(0), Some(deep.len()));
        assert_eq!(work, 8192);
    }
}
