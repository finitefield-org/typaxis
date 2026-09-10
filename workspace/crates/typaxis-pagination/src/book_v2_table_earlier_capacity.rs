//! Earlier original table boundaries for both body and definition enumeration.
use super::*;
impl<'b, 'f, 's, 'p, 'a> BookV2TableBreakSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) fn earlier_capacity(
        &mut self,
        cursor: &BookV2TableCursor<'b, 'f, 's, 'p, 'a>,
        selected: &BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>,
        steps: &mut u64,
    ) -> Result<Option<Length>, ProductionBodyPaginationError> {
        if cursor.table_index != self.table_index
            || !std::ptr::eq(cursor.measurements, self.measurements)
            || !std::ptr::eq(selected.before.measurements, self.measurements)
            || selected.before.table_index != cursor.table_index
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        std::mem::swap(&mut self.kernel.work.used, steps);
        self.kernel.repeated_header_height = selected.header_variant.map(|h| h.height());
        let result = if let Some(nested) = &selected.nested {
            (|| {
                let owner = self.measurements.tables()[cursor.table_index].owner;
                self.kernel
                    .work
                    .take(nested.leaves.len() as u64 + 1, owner)?;
                let mut boundary = selected.header_variant.map_or(Length::ZERO, |h| h.height());
                for leaf in &nested.leaves {
                    let measurements = leaf.header.map_or(self.measurements, |(h, _)| h.variant());
                    let item = &measurements.flow().collected.items[leaf.item];
                    boundary = boundary.max(add(
                        add(leaf.top, item.height, owner)?,
                        item.trailing,
                        owner,
                    )?);
                }
                Ok(if boundary < selected.used_height() {
                    Some(boundary)
                } else if boundary > Length::ZERO {
                    boundary.checked_sub(Length::from_raw(1).expect("one layout unit"))
                } else {
                    None
                })
            })()
        } else if self.kernel.spanning_breaks {
            (|| {
                let owner = self.measurements.tables()[cursor.table_index].owner;
                self.kernel
                    .work
                    .take(selected.cells().len() as u64 + 1, owner)?;
                let mut boundary = selected.projection.header_top.map_or(Length::ZERO, |top| {
                    top.checked_add(selected.header_height())
                        .expect("validated header extent")
                });
                if let Some(last) = selected.caption_content().last() {
                    boundary = boundary.max(
                        last.end
                            .checked_sub(cursor.position.offset)
                            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
                    );
                }
                for cell in selected.cells() {
                    boundary = boundary.max(add(cell.top(), cell.height(), owner)?);
                }
                Ok(if boundary < selected.used_height() {
                    Some(boundary)
                } else if boundary > Length::ZERO {
                    boundary.checked_sub(Length::from_raw(1).expect("one layout unit"))
                } else {
                    None
                })
            })()
        } else if self.kernel.cell_breaks {
            Ok(if selected.used_height() > Length::ZERO {
                selected
                    .used_height()
                    .checked_sub(Length::from_raw(1).expect("one layout unit"))
            } else {
                None
            })
        } else {
            self.kernel
                .earlier_capacity_for(&cursor.position, selected.after().offset())
        };
        self.kernel.repeated_header_height = None;
        std::mem::swap(&mut self.kernel.work.used, steps);
        result
    }
}
