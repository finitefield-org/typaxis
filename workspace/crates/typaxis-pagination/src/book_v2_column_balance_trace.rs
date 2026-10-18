//! Retained source prefixes for every column of a rejected capacity trial.
use super::*;

fn point(
    out: &mut dyn Bytes,
    item: usize,
    table: usize,
    cursor: Option<BookV2TableCursor<'_, '_, '_, '_, '_>>,
) -> Result<(), ProductionBodyPaginationError> {
    number(out, item)?;
    number(out, table)?;
    table_cursor(out, cursor)
}

impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) fn encode_balance_part(
        &mut self,
        part: &BookV2BodySelectedPart<'b, 'f, 's, 'p, 'a>,
        out: &mut dyn Bytes,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        self.inner.content.step(root)?;
        out.write(&part.top().raw().to_be_bytes())?;
        out.write(&part.height().raw().to_be_bytes())?;
        out.write(&[u8::from(part.items().is_some())])?;
        if let Some(items) = part.items() {
            number(out, items.start)?;
            number(out, items.end)?;
        }
        out.write(&part.table().map_or([0; 32], |t| t.fingerprint()))?;
        out.write(&[u8::from(part.table().is_some())])?;
        if let Some(table) = part.table() {
            table_cursor(out, Some(table.before()))?;
            table_cursor(out, Some(table.after()))?;
            out.write(&table.available_height().raw().to_be_bytes())?;
            out.write(&table.used_height().raw().to_be_bytes())?;
            out.write(&table.header_height().raw().to_be_bytes())?;
            out.write(&[
                u8::from(table.repeats_header()),
                u8::from(
                    self.inner
                        .tables
                        .as_ref()
                        .unwrap()
                        .table(table.before().table_index())
                        .caption()
                        .is_some(),
                ),
            ])?;
            out.write(
                &table
                    .header_variant()
                    .map_or([0; 32], |header| header.fingerprint()),
            )?;
            number(out, table.cells().len())?;
            for cell in table.cells() {
                self.inner.content.step(cell.owner())?;
                out.write(&cell.owner().get().to_be_bytes())?;
                number(out, cell.cell_index())?;
                for value in [
                    cell.top(),
                    cell.height(),
                    cell.offset_before(),
                    cell.offset_after(),
                ] {
                    out.write(&value.raw().to_be_bytes())?;
                }
                let range = cell.content_range();
                number(out, range.start)?;
                number(out, range.end)?;
            }
            let mut ranges = 0usize;
            for range in table.source_leaf_ranges() {
                self.inner.content.step(root)?;
                range?;
                ranges = ranges
                    .checked_add(1)
                    .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
            }
            number(out, ranges)?;
            for range in table.source_leaf_ranges() {
                self.inner.content.step(root)?;
                let range = range?;
                number(out, range.start)?;
                number(out, range.end)?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn balance_capacity_trace(
        &mut self,
        entry: &BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a>,
        initial: &BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>,
        trial: Option<&BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>>,
        target: PositiveLength,
        ordinary: bool,
        rejections: &[BookV2ColumnBalanceRejection],
    ) -> Result<Vec<u8>, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let source = &entry.source.source;
        let endpoint = &initial.candidate().next.source;
        let flow = self.inner.content.flow;
        let next_table_start = self
            .inner
            .tables
            .as_ref()
            .unwrap()
            .range(source.next_table)
            .map(|r| r.start);
        self.balance_blob(|this, out| {
            out.write(&sha256(b"typaxis.book-2-column-capacity-trace/1"))?;
            out.write(&target.get().raw().to_be_bytes())?;
            number(
                out,
                usize::from(initial.candidate().frames().column_count()),
            )?;
            let mut item = source.item;
            let mut table = source.next_table;
            let mut cursor = source.continuation;
            let mut rejection_index = 0usize;
            for column in 0..initial.candidate().frames().column_count() {
                this.inner.content.step(root)?;
                out.write(&column.to_be_bytes())?;
                point(out, item, table, cursor)?;
                let start = item;
                let mut used = Length::ZERO;
                let mut preceding = None;
                let selected = trial.and_then(|p| p.candidate().columns().get(usize::from(column)));
                if ordinary {
                    while item < endpoint.item {
                        let group = match next_ordinary_source_group(
                            flow.body_items(),
                            item,
                            next_table_start,
                            |owner| this.inner.content.step(owner),
                        )? {
                            NextOrdinarySourceGroup::Complete(group)
                                if group.end <= endpoint.item =>
                            {
                                group
                            }
                            _ => return Err(error(root, E::ColumnBalanceContradiction)),
                        };
                        let required = group.required_after(used, preceding)?;
                        if required > target.get() {
                            break;
                        }
                        used = required;
                        preceding = Some(group.after);
                        item = group.end;
                    }
                } else if let Some(selected) = selected {
                    used = selected.used_height();
                    for part in selected.parts() {
                        this.inner.content.step(root)?;
                        if let Some(items) = part.items() {
                            if items.start != item || items.is_empty() || cursor.is_some() {
                                return Err(error(root, E::ReceiptMismatch));
                            }
                            item = items.end;
                            preceding = Some(flow.body_items()[item - 1].after);
                        } else if let Some(fragment) = part.table() {
                            let context = this.inner.tables.as_ref().unwrap();
                            if fragment.before().table_index() != table
                                || context.range(table).is_none_or(|r| r.start != item)
                                || cursor.map_or(!fragment.before().is_initial(), |c| {
                                    !groups::same_cursor(c, fragment.before())
                                })
                            {
                                return Err(error(root, E::ReceiptMismatch));
                            }
                            preceding = Some(context.table(table).space_after());
                            if fragment.after().is_terminal() {
                                item = context.range(table).unwrap().end;
                                table = context.successor(table);
                                cursor = None;
                            } else {
                                cursor = Some(fragment.after());
                            }
                        } else {
                            return Err(error(root, E::ReceiptMismatch));
                        }
                    }
                }
                point(out, item, table, cursor)?;
                out.write(&used.raw().to_be_bytes())?;
                out.write(&[u8::from(preceding.is_some())])?;
                out.write(&preceding.unwrap_or(Length::ZERO).raw().to_be_bytes())?;
                if ordinary {
                    number(out, usize::from(item > start))?;
                    if item > start {
                        this.inner.content.step(root)?;
                        out.write(&Length::ZERO.raw().to_be_bytes())?;
                        out.write(&used.raw().to_be_bytes())?;
                        out.write(&[1])?;
                        number(out, start)?;
                        number(out, item)?;
                        out.write(&[0; 32])?;
                        out.write(&[0])?;
                    }
                } else {
                    number(out, selected.map_or(0, |c| c.parts().len()))?;
                    if let Some(selected) = selected {
                        for part in selected.parts() {
                            this.encode_balance_part(part, out)?;
                        }
                    }
                }
                let blocked = rejections
                    .get(rejection_index)
                    .is_some_and(|r| r.column == column);
                if blocked {
                    this.inner.content.step(rejections[rejection_index].owner)?;
                    rejection_index += 1;
                }
                out.write(&[u8::from(blocked)])?;
            }
            if rejection_index != rejections.len() {
                return Err(error(root, E::ColumnBalanceContradiction));
            }
            Ok(())
        })
    }
}
