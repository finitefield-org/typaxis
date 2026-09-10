//! Actual table continuation capacity with a separately owned repeated header.
use super::*;
use crate::production_body::body_flow::book_v2::BookV2TableHeaderVariant;

/// The global item index belongs exclusively to `measurements`. Repeated child
/// headers retain their own independently selected header owner.
pub struct BookV2TableHeaderPaintLeaf<'m, 'f, 's, 'p, 'a> {
    measurements: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    item: usize,
    cell: Option<NodeId>,
    top: Length,
    repeated: bool,
    header: Option<(&'m BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a>, usize)>,
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableHeaderPaintLeaf<'m, 'f, 's, 'p, 'a> {
    pub fn measurements(&self) -> &'m BookV2TableMeasurements<'f, 's, 'p, 'a> {
        self.measurements
    }
    pub fn global_item_index(&self) -> usize {
        self.item
    }
    pub fn cell_owner(&self) -> Option<NodeId> {
        self.cell
    }
    pub fn top(&self) -> Length {
        self.top
    }
    pub fn repeated(&self) -> bool {
        self.repeated
    }
    pub fn uses_header_variant(&self) -> bool {
        self.header.is_some()
    }
    pub(in crate::production_body::body_flow) fn header_source(
        &self,
    ) -> Option<(&'m BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a>, usize)> {
        self.header
    }
}

/// Semantic cursor/ranges use the base, while paint keeps explicit measurement
/// owners. Only the mixed scheduler can convert this to a page selection; that
/// conversion retains its variant and rejects legacy single-owner placement.
pub struct BookV2TableHeaderFragmentSelection<'h, 'm, 'f, 's, 'p, 'a> {
    selected: BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>,
    header: &'h BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a>,
    fingerprint: [u8; 32],
}
impl<'h, 'm, 'f, 's, 'p, 'a> BookV2TableHeaderFragmentSelection<'h, 'm, 'f, 's, 'p, 'a> {
    pub fn before(&self) -> BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
        self.selected.before()
    }
    pub fn after(&self) -> BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
        self.selected.after()
    }
    pub fn header(&self) -> &'h BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a> {
        self.header
    }
    pub fn used_height(&self) -> Length {
        self.selected.used_height()
    }
    pub fn available_height(&self) -> Length {
        self.selected.available_height()
    }
    pub fn header_height(&self) -> Length {
        self.selected.header_height()
    }
    pub fn forced_break_owner(&self) -> Option<NodeId> {
        self.selected.forced_break_owner()
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn semantic_leaf_ranges(&self) -> impl Iterator<Item = Range<usize>> + '_ {
        self.selected.semantic_leaf_ranges()
    }
    pub fn source_leaf_ranges(
        &self,
    ) -> impl Iterator<Item = Result<Range<usize>, ProductionBodyPaginationError>> + '_ {
        self.selected.source_leaf_ranges()
    }
    pub fn placement_leaves(
        &self,
    ) -> impl Iterator<
        Item = Result<
            BookV2TableHeaderPaintLeaf<'_, 'f, 's, 'p, 'a>,
            ProductionBodyPaginationError,
        >,
    > + '_ {
        paint_leaves(&self.selected, Some(self.header))
    }
}

pub(super) fn paint_leaves<'q, 'm, 'f, 's, 'p, 'a>(
    selected: &'q BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>,
    header: Option<&'m BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a>>,
) -> impl Iterator<
    Item = Result<BookV2TableHeaderPaintLeaf<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>,
> + 'q {
    let base = selected.before.measurements;
    let owner = base.tables()[selected.before.table_index].owner;
    let captions = selected.caption_content().iter().map(move |content| {
        let ProductionTableContentSource::FlowItem(item) = content.source else {
            return Err(error(owner, E::ReceiptMismatch));
        };
        let top = content
            .top
            .checked_sub(selected.before.position.offset)
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        Ok((None, item, top, false, None))
    });
    let cells = selected_placement_leaves(
        &base.tables()[selected.before.table_index],
        &base.flow().lines().prepared().source_flow().tables()[selected.before.table_index],
        &selected.projection.cells,
        header.is_none() && selected.repeats_header(),
        selected
            .projection
            .header_top
            .filter(|_| header.is_none() && selected.nested.is_none()),
    )
    .map(|leaf| leaf.map(|(cell, item, top, repeated)| (Some(cell), item, top, repeated, None)));
    let nested = selected.nested.iter().flat_map(|p| {
        p.leaves
            .iter()
            .map(|l| Ok((l.cell, l.item, l.top, l.repeated, l.header)))
    });
    let body = captions.chain(cells).chain(nested).filter_map(move |leaf| {
        let (cell, item, top, repeated, header) = match leaf {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        let measurements = header.map_or(base, |(h, _)| h.variant());
        match measurements.item(item) {
            Some(i) if i.source().is_none() => None,
            Some(_) => Some(Ok(BookV2TableHeaderPaintLeaf {
                measurements,
                item,
                cell,
                top,
                repeated,
                header,
            })),
            None => Some(Err(error(owner, E::ReceiptMismatch))),
        }
    });
    let header = header.into_iter().flat_map(move |header| {
        header
            .leaves()
            .iter()
            .enumerate()
            .map(move |(index, leaf)| {
                Ok(BookV2TableHeaderPaintLeaf {
                    measurements: header.variant(),
                    item: leaf.variant_item_index(),
                    cell: leaf.cell_owner(),
                    top: leaf.top(),
                    repeated: true,
                    header: Some((header, index)),
                })
            })
    });
    header.chain(body)
}
impl<'b, 'f, 's, 'p, 'a> BookV2TableHeaderFragmentSelection<'b, 'b, 'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) fn into_mixed_selection(
        mut self,
    ) -> BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a> {
        self.selected.header_variant = Some(self.header);
        self.selected.projection.fingerprint = self.fingerprint;
        self.selected
    }
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) fn evaluate_in_frame(
        &mut self,
        cursor: &BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
        available: Length,
        catalog: Option<&'m crate::book_v2::BookV2TableHeaderCatalog<'m, 'f, 's, 'p, 'a>>,
        region_width: Option<PositiveLength>,
    ) -> Result<
        Option<BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let previous_catalog = self.frame_catalog;
        let previous_width = self.frame_width;
        self.frame_catalog = catalog;
        self.frame_width = region_width;
        let result = (|| {
            if let Some(catalog) =
                catalog.filter(|_| cursor.has_started_rows() && self.kernel.header_rows > 0)
            {
                let owner = self.measurements.tables()[self.table_index].owner;
                self.kernel.work.take(catalog.lookup_work(), owner)?;
                if !std::ptr::eq(catalog.base(), self.measurements) {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                let header = catalog.for_region_width(self.table_index, region_width)?;
                self.evaluate_with_header(cursor, available, header, catalog.limits())
                    .map(|selected| selected.map(|s| s.into_mixed_selection()))
            } else {
                self.evaluate(cursor, available)
            }
        })();
        self.frame_catalog = previous_catalog;
        self.frame_width = previous_width;
        result
    }
    /// Select a real continuation using the supplied variant's actual header
    /// height. Original header/caption consumption must already be complete.
    pub fn evaluate_with_header<'h>(
        &mut self,
        cursor: &BookV2TableCursor<'_, '_, '_, '_, '_>,
        available: Length,
        header: &'h BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<
        Option<BookV2TableHeaderFragmentSelection<'h, 'm, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let owner = self.measurements.tables()[self.table_index].owner;
        self.kernel.work.take(4, owner)?;
        header.verify(self.measurements, header.variant(), limits)?;
        if !std::ptr::eq(cursor.measurements, self.measurements)
            || cursor.table_index != self.table_index
            || header.table_index() != self.table_index
            || cursor.is_initial()
            || !cursor.has_started_rows()
            || cursor.is_terminal()
            || self.kernel.header_rows == 0
            || self.kernel.repeated_header_height.is_some()
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        if available < Length::ZERO || available > self.maximum_height() {
            return Err(error(owner, E::InvalidTableCapacity));
        }
        let prepaid = if let Some(catalog) = self.frame_catalog {
            self.kernel.work.take(catalog.len() as u64 + 1, owner)?;
            if !std::ptr::eq(catalog.base(), self.measurements)
                || self.record_charge() < catalog.record_charge()
                || !catalog.contains_header(header)
            {
                return Err(error(owner, E::ReceiptMismatch));
            }
            true
        } else {
            false
        };
        if prepaid {
            // The bound catalog already retains this exact header projection and
            // its measurement. A trial borrows them and adds one selection owner;
            // copied nested paint leaves are charged separately by `leaf`.
            self.kernel.charge.take(1, owner)?;
        } else {
            self.kernel
                .work
                .take(self.header_measurements.len() as u64 + 1, owner)?;
            let new_measurement = !self
                .header_measurements
                .iter()
                .any(|m| std::ptr::eq(*m, header.variant()));
            let prior = self.record_charge().max(header.shared_record_charge());
            self.kernel.charge.remaining = self
                .kernel
                .maximum_records
                .checked_sub(prior)
                .ok_or_else(|| error(owner, E::FragmentLimit))?;
            // Each independently owned measurement is retained by exact reference
            // and charged once. Header projection storage is conservatively prepaid
            // for every attempt, including repeated uses of the same header object.
            let extra = header
                .projection_record_charge()
                .checked_add(if new_measurement {
                    header.variant().retained_records()
                } else {
                    0
                })
                .and_then(|n| n.checked_add(if new_measurement { 2 } else { 1 }))
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| error(owner, E::FragmentLimit))?;
            self.kernel.charge.take(extra, owner)?;
            if new_measurement {
                self.kernel
                    .work
                    .take(self.header_measurements.len() as u64, owner)?;
                self.header_measurements
                    .try_reserve_exact(1)
                    .map_err(|_| error(owner, E::AllocationFailure))?;
                self.header_measurements.push(header.variant());
            }
        }
        if header.height() > self.maximum_height() {
            return Err(error(owner, E::TableHeaderOversize));
        }
        if header.height() > available {
            return Ok(None);
        }
        // Restore even when selection fails. Work and retained candidates remain
        // charged to this search, so a failed trial never refunds its budget.
        self.kernel.repeated_header_height = Some(header.height());
        let result = self.evaluate(cursor, available);
        self.kernel.repeated_header_height = None;
        let Some(selected) = result? else {
            return Ok(None);
        };
        if !selected.repeats_header()
            || selected.header_height() != header.height()
            || selected.used_height() > available
            || !selected.caption_content().is_empty()
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        self.kernel.work.take(1, owner)?;
        let mut bytes = [0u8; 96];
        bytes[..32].copy_from_slice(&sha256(b"typaxis.book-2-table-header-selection/1"));
        bytes[32..64].copy_from_slice(&selected.fingerprint());
        bytes[64..].copy_from_slice(&header.fingerprint());
        Ok(Some(BookV2TableHeaderFragmentSelection {
            selected,
            header,
            fingerprint: sha256(&bytes),
        }))
    }
}
