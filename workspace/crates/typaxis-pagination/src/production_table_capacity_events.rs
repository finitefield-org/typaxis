//! Private lower bounds on the next capacity branch that can change original
//! source atoms. A disabled observer adds no work or retained records.
use super::*;

#[derive(Clone, Copy)]
pub(in crate::production_body::body_flow) struct CapacityProbe {
    pub(in crate::production_body::body_flow) available: Length,
    pub(in crate::production_body::body_flow) next: Option<Length>,
    pub(in crate::production_body::body_flow) complete: bool,
    pub(in crate::production_body::body_flow) prior_spool: u64,
    pub(in crate::production_body::body_flow) retained_spool: u64,
    pub(in crate::production_body::body_flow) maximum_spool: u64,
}
impl CapacityProbe {
    pub(in crate::production_body::body_flow) fn new(
        available: Length,
        prior_spool: u64,
        maximum_spool: u64,
    ) -> Self {
        Self {
            available,
            next: None,
            complete: true,
            prior_spool,
            retained_spool: 0,
            maximum_spool,
        }
    }
    fn reserve_spool(
        &mut self,
        bytes: u64,
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        let retained = self
            .retained_spool
            .checked_add(bytes)
            .filter(|n| {
                self.prior_spool
                    .checked_add(*n)
                    .is_some_and(|total| total <= self.maximum_spool)
            })
            .ok_or_else(|| error(owner, E::SpoolLimit))?;
        self.retained_spool = retained;
        Ok(())
    }
}
fn first_legal_coordinate(
    blocked: &[Blocked],
    mut coordinate: Length,
    owner: NodeId,
) -> Result<Length, ProductionBodyPaginationError> {
    for block in blocked {
        if coordinate > block.start
            && (coordinate < block.end || (coordinate == block.end && block.closed_end))
        {
            coordinate = if block.closed_end {
                add(
                    block.end,
                    Length::from_raw(1).expect("one layout unit"),
                    owner,
                )?
            } else {
                block.end
            };
        }
    }
    Ok(coordinate)
}
impl TableBreakKernel<'_> {
    pub(in crate::production_body::body_flow) fn probe_capacity_spool(
        &mut self,
        bytes: u64,
    ) -> Result<(), ProductionBodyPaginationError> {
        if let Some(probe) = &mut self.capacity_probe {
            probe.reserve_spool(bytes, self.input.table.owner)?;
        }
        Ok(())
    }
    pub(in crate::production_body::body_flow) fn probe_capacity_event(
        &mut self,
        required: Length,
    ) -> Result<(), ProductionBodyPaginationError> {
        if let Some(probe) = &mut self.capacity_probe {
            self.work.take(1, self.input.table.owner)?;
            if required > probe.available {
                probe.next = Some(probe.next.map_or(required, |n| n.min(required)));
            }
        }
        Ok(())
    }
    pub(in crate::production_body::body_flow) fn probe_strict_capacity_event(
        &mut self,
        required: Length,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.capacity_probe.is_some() {
            let required = add(
                required,
                Length::from_raw(1).expect("one layout unit"),
                self.input.table.owner,
            )?;
            self.probe_capacity_event(required)?;
        }
        Ok(())
    }
    pub(super) fn probe_common_capacity_events(
        &mut self,
        cursor: &TablePosition,
    ) -> Result<(), ProductionBodyPaginationError> {
        if self.capacity_probe.is_none() {
            return Ok(());
        }
        let table = self.input.table;
        let owner = table.owner;
        let source_cells = self.input.source.cells();
        let prefix = if table.caption.is_some() && !cursor.header_seen {
            Length::ZERO
        } else {
            self.paint_header_height()
        };
        let mut consider = |coordinate: Length| -> Result<(), ProductionBodyPaginationError> {
            self.work.take(self.blocked.len() as u64 + 1, owner)?;
            let coordinate = first_legal_coordinate(&self.blocked, coordinate, owner)?;
            if coordinate <= table.height {
                self.probe_capacity_event(add(
                    prefix,
                    coordinate
                        .checked_sub(cursor.offset)
                        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
                    owner,
                )?)?;
            }
            Ok(())
        };
        for row in &table.rows {
            consider(add(row.top, row.height, owner)?)?;
        }
        for (cell, source) in table.cells.iter().zip(source_cells) {
            let origin = table.rows[source.row() as usize].top;
            for content in &cell.content {
                consider(add(origin, content.end, cell.owner)?)?;
            }
        }
        if let Some(caption) = &table.caption {
            for content in &caption.content {
                consider(content.end)?;
            }
            consider(caption.height)?;
        }
        consider(add(
            cursor.offset,
            Length::from_raw(1).expect("one layout unit"),
            owner,
        )?)?;
        consider(table.height)?;
        if self.cell_breaks && !cursor.header_seen {
            if let Some(caption) = &table.caption {
                self.probe_capacity_event(
                    caption
                        .height
                        .checked_sub(cursor.offset)
                        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn length(n: i64) -> Length {
        Length::from_raw(n).unwrap()
    }
    #[test]
    fn book_v2_table_nearest_common_guards_invert_closed_and_open_source_cuts() {
        let owner = NodeId::new(9);
        let blocked = [
            Blocked {
                start: length(2),
                end: length(7),
                closed_end: true,
            },
            Blocked {
                start: length(10),
                end: length(20),
                closed_end: false,
            },
        ];
        let previous = |mut n: Length| loop {
            let Some(block) = blocked.iter().rev().find(|b| b.start < n) else {
                return n;
            };
            if n < block.end || (n == block.end && block.closed_end) {
                n = block.start;
            } else {
                return n;
            }
        };
        for raw in 0..=24 {
            let expected = (raw..=25)
                .find(|n| previous(length(*n)) >= length(raw))
                .unwrap();
            let actual = first_legal_coordinate(&blocked, length(raw), owner).unwrap();
            assert_eq!(actual, length(expected), "original atom coordinate {raw}");
            assert!(previous(actual) >= length(raw));
            if actual > Length::ZERO {
                assert!(previous(length(actual.raw() - 1)) < length(raw) || actual == length(raw));
            }
        }
    }
    #[test]
    fn book_v2_table_nearest_replay_spool_preserves_child_failure_prefix() {
        let owner = NodeId::new(10);
        let mut parent = CapacityProbe::new(length(100), 2, 11);
        parent.reserve_spool(7, owner).unwrap();
        let mut child = CapacityProbe::new(
            length(60),
            parent.prior_spool + parent.retained_spool,
            parent.maximum_spool,
        );
        child.reserve_spool(2, owner).unwrap();
        assert_eq!(
            child.reserve_spool(1, owner).unwrap_err().kind,
            E::SpoolLimit
        );
        parent.reserve_spool(child.retained_spool, owner).unwrap();
        assert_eq!(parent.prior_spool + parent.retained_spool, 11);
        assert_eq!(
            parent.reserve_spool(1, owner).unwrap_err().kind,
            E::SpoolLimit
        );
        assert_eq!(parent.retained_spool, 9);
    }
}
