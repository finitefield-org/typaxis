//! Table and cell boundaries over the shared, actual leaf measurements.
use super::*;
use std::ops::Range;

pub(super) struct Cell {
    pub source_index: usize,
    pub items: Range<usize>,
    pub children: Vec<usize>,
}
pub(super) struct Caption {
    pub items: Range<usize>,
    pub children: Vec<usize>,
    end_event: usize,
    closed: bool,
}
pub(super) struct Table {
    pub owner: NodeId,
    pub definition: Option<usize>,
    pub parent: Option<usize>,
    pub items: Range<usize>,
    pub cells: Vec<Cell>,
    pub caption: Option<Caption>,
    pub before: Length,
    pub after: Length,
    pub keep: bool,
    pub keep_together: bool,
}
struct Open {
    owner: NodeId,
    table: usize,
    cell: Option<usize>,
    previous: Option<(usize, Option<usize>)>,
}
#[derive(Default)]
pub(super) struct Collection {
    pub tables: Vec<Table>,
    closed: Vec<usize>,
    open: Vec<Open>,
    current: Option<(usize, Option<usize>)>,
}
impl Collection {
    /// Close the source caption before the first row event. Nested table ends
    /// restore their parent first, so this never closes a child's region.
    pub fn event_boundary(
        &mut self,
        event: usize,
        item: usize,
    ) -> Result<(), ProductionBodyPaginationError> {
        if let Some((index, None)) = self.current {
            let table = &mut self.tables[index];
            if let Some(caption) = &mut table.caption {
                if !caption.closed && event >= caption.end_event {
                    if event != caption.end_event {
                        return Err(error(table.owner, E::ReceiptMismatch));
                    }
                    caption.items.end = item;
                    caption.closed = true;
                }
            }
        }
        Ok(())
    }
    pub fn keep_caption(
        &mut self,
        items: &mut [Item],
        start: usize,
        last: usize,
        table_cursor: usize,
        charge: &mut Charge,
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        if table_cursor == self.tables.len() {
            for item in &mut items[start..last] {
                item.keep = true;
            }
            return Ok(());
        }
        charge.take(last - start, owner)?;
        let mut cursor = start;
        let mut table_cursor = table_cursor;
        while cursor < last {
            if let Some(table) = self
                .tables
                .get_mut(table_cursor)
                .filter(|t| t.items.start == cursor)
            {
                if table.items.end <= cursor {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                // The keep joins a table to the next caption sibling; it must
                // not join the last line of one cell to the next parallel cell.
                table.keep |= table.items.end <= last;
                table.keep_together = true;
                cursor = table.items.end;
                table_cursor +=
                    self.tables[table_cursor..].partition_point(|t| t.items.start < cursor);
            } else {
                items[cursor].keep = true;
                cursor += 1;
            }
        }
        Ok(())
    }
    pub fn begin(
        &mut self,
        lines: BodyLines<'_, '_, '_>,
        owner: NodeId,
        kind: Region,
        item_index: usize,
        definition: Option<usize>,
        charge: &mut Charge,
    ) -> Result<(), ProductionBodyPaginationError> {
        let (table, cell) = if kind == Region::Table {
            let index = self.tables.len();
            let source = lines
                .source_flow()
                .tables()
                .get(index)
                .filter(|t| t.owner() == owner)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            if source.page_name().is_some()
                && (!lines.has_named_page_plan() || definition.is_some())
            {
                return Err(error(owner, E::PendingNamedPage));
            }
            if lines
                .frames()
                .and_then(|f| f.tables().get(index))
                .map(|t| t.owner())
                != Some(owner)
            {
                return Err(error(owner, E::ReceiptMismatch));
            }
            if let Some((parent, cell)) = self.current {
                charge.take(1, owner)?;
                let parent = &mut self.tables[parent];
                let children = if let Some(cell) = cell {
                    &mut parent.cells[cell].children
                } else {
                    &mut parent
                        .caption
                        .as_mut()
                        .filter(|c| !c.closed)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                        .children
                };
                children
                    .try_reserve(1)
                    .map_err(|_| error(owner, E::AllocationFailure))?;
                children.push(index);
            }
            charge.take(1, owner)?;
            self.tables
                .try_reserve(1)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            let style = source.style().block_style();
            let caption = if let Some(events) = source.caption_event_range() {
                charge.take(1, owner)?;
                Some(Caption {
                    items: item_index..item_index,
                    children: Vec::new(),
                    end_event: events.end,
                    closed: false,
                })
            } else {
                None
            };
            self.tables.push(Table {
                owner,
                definition,
                parent: self.current.map(|(table, _)| table),
                items: item_index..item_index,
                cells: Vec::new(),
                caption,
                before: style.space_before().get(),
                after: style.space_after().get(),
                keep: style.keep_with_next(),
                keep_together: false,
            });
            (index, None)
        } else if kind == Region::TableCell {
            let (table, active_cell) = self
                .current
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            if active_cell.is_some() {
                return Err(error(owner, E::ReceiptMismatch));
            }
            let cells = &mut self.tables[table].cells;
            let index = cells.len();
            if lines.source_flow().tables()[table]
                .cells()
                .get(index)
                .map(|c| c.owner())
                != Some(owner)
            {
                return Err(error(owner, E::ReceiptMismatch));
            }
            charge.take(1, owner)?;
            cells
                .try_reserve(1)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            cells.push(Cell {
                source_index: index,
                items: item_index..item_index,
                children: Vec::new(),
            });
            (table, Some(index))
        } else {
            return Ok(());
        };
        charge.take(1, owner)?;
        self.open
            .try_reserve(1)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        self.open.push(Open {
            owner,
            table,
            cell,
            previous: self.current,
        });
        self.current = Some((table, cell));
        Ok(())
    }
    pub fn end(
        &mut self,
        owner: NodeId,
        end: usize,
        charge: &mut Charge,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.open.last().map(|o| o.owner) != Some(owner) {
            return Ok(());
        }
        let open = self
            .open
            .pop()
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        if let Some(cell) = open.cell {
            self.tables[open.table].cells[cell].items.end = end;
        } else {
            self.tables[open.table].items.end = end;
            charge.take(1, owner)?;
            self.closed
                .try_reserve(1)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            self.closed.push(open.table);
        }
        self.current = open.previous;
        Ok(())
    }
    /// A wrapper's outside margins belong to a boundary table, not to its
    /// first/last cell. Creation/completion order selects the outermost table
    /// at each boundary without rescanning nested subtrees.
    pub fn wrap(
        &mut self,
        items: &mut [Item],
        cursor: usize,
        first: usize,
        last: usize,
        before: Length,
        after: Length,
        keep: bool,
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        if let Some(table) = self
            .tables
            .get_mut(cursor)
            .filter(|t| t.items.start == first && t.items.end > first)
        {
            table.before = add(table.before, before, owner)?;
        } else {
            items[first].before = add(items[first].before, before, owner)?;
        }
        let tail = self
            .closed
            .last()
            .copied()
            .filter(|i| *i >= cursor)
            .and_then(|i| self.tables.get_mut(i))
            .filter(|t| t.items.end == last + 1 && t.items.start < t.items.end);
        if let Some(table) = tail {
            table.after = add(table.after, after, owner)?;
            table.keep |= keep;
        } else {
            items[last].after = add(items[last].after, after, owner)?;
            items[last].keep |= keep;
        }
        Ok(())
    }
    pub fn verify_closed(
        &self,
        lines: BodyLines<'_, '_, '_>,
        enabled: bool,
    ) -> Result<(), ProductionBodyPaginationError> {
        if !self.open.is_empty()
            || self.current.is_some()
            || (enabled && self.tables.len() != lines.source_flow().tables().len())
            || self.closed.len() != self.tables.len()
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        for (table, source) in self.tables.iter().zip(lines.source_flow().tables()) {
            if table.cells.len() != source.cells().len()
                || table.caption.is_some() != source.caption_event_range().is_some()
                || table.caption.as_ref().is_some_and(|c| {
                    !c.closed || c.items.start != table.items.start || c.items.end > table.items.end
                })
            {
                return Err(error(table.owner, E::ReceiptMismatch));
            }
        }
        Ok(())
    }
}
