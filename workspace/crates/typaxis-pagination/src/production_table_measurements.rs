//! Common table measurements. This owner grants no page/paint authorization.
use super::*;
use typaxis_syntax::ProductionTableSection;

#[path = "production_table_breaks.rs"]
mod breaks;
#[cfg(feature = "book-v2-staging")]
pub(super) use breaks::book_v2::{BookV2TableBodyContext, BookV2DefinitionTableContext};
#[cfg(feature = "book-v2-staging")]
pub use breaks::book_v2::*;
pub(super) use breaks::ProductionTableBodyContext;
pub use breaks::{
    prepare_production_table_footnote_search, prepare_production_table_search,
    ProductionTableBreakSearch, ProductionTableCellSlice, ProductionTableCursor,
    ProductionTableFootnoteSearch, ProductionTableFootnoteSelection, ProductionTableFootnoteState,
    ProductionTableFragmentSelection,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionTableContentSource {
    /// Index in this measurement owner's shared body/definition leaf stream.
    FlowItem(usize),
    /// A nested table is one child extent, not the sum of its parallel cells.
    Table(usize),
}
/// Shared child extent for cells and captions; carries no cell role or ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionTableCellContent {
    source: ProductionTableContentSource,
    top: Length,
    end: Length,
}
impl ProductionTableCellContent {
    pub const fn source(&self) -> ProductionTableContentSource {
        self.source
    }
    /// Content top relative to its cell or caption, after spacing/marker leading.
    pub const fn top(&self) -> Length {
        self.top
    }
    /// End of this child including its trailing extent and outside spacing.
    pub const fn end(&self) -> Length {
        self.end
    }
}
pub struct ProductionMeasuredTableCell {
    owner: NodeId,
    source_index: usize,
    natural_height: Length,
    content: Vec<ProductionTableCellContent>,
}
impl ProductionMeasuredTableCell {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn source_index(&self) -> usize {
        self.source_index
    }
    pub const fn natural_height(&self) -> Length {
        self.natural_height
    }
    pub fn content(&self) -> &[ProductionTableCellContent] {
        &self.content
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionMeasuredTableRow {
    owner: NodeId,
    section: ProductionTableSection,
    top: Length,
    height: Length,
}
impl ProductionMeasuredTableRow {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn section(&self) -> ProductionTableSection {
        self.section
    }
    pub const fn top(&self) -> Length {
        self.top
    }
    pub const fn height(&self) -> Length {
        self.height
    }
}
/// Measured caption subflow, distinct from cells and repeated table headers.
pub struct ProductionMeasuredTableCaption {
    height: Length,
    content: Vec<ProductionTableCellContent>,
}
impl ProductionMeasuredTableCaption {
    pub const fn height(&self) -> Length {
        self.height
    }
    pub fn content(&self) -> &[ProductionTableCellContent] {
        &self.content
    }
}
pub struct ProductionMeasuredTable {
    owner: NodeId,
    source_index: usize,
    cells: Vec<ProductionMeasuredTableCell>,
    caption: Option<ProductionMeasuredTableCaption>,
    rows: Vec<ProductionMeasuredTableRow>,
    height: Length,
    before: Length,
    after: Length,
    keep: bool,
    keep_together: bool,
}
impl ProductionMeasuredTable {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn source_index(&self) -> usize {
        self.source_index
    }
    pub fn caption(&self) -> Option<&ProductionMeasuredTableCaption> {
        self.caption.as_ref()
    }
    pub fn cells(&self) -> &[ProductionMeasuredTableCell] {
        &self.cells
    }
    pub fn rows(&self) -> &[ProductionMeasuredTableRow] {
        &self.rows
    }
    pub const fn height(&self) -> Length {
        self.height
    }
    pub const fn space_before(&self) -> Length {
        self.before
    }
    pub const fn space_after(&self) -> Length {
        self.after
    }
    pub const fn keep_with_next(&self) -> bool {
        self.keep
    }
    /// An authored enclosing keep-caption requires this complete table extent.
    pub const fn keep_together(&self) -> bool {
        self.keep_together
    }
}
pub struct ProductionTableMeasurements<'f, 's, 'p, 'a> {
    flow: ProductionPreparedBodyFlow<'f, 's, 'p, 'a>,
    tables: Vec<ProductionMeasuredTable>,
    record_charge: u64,
    fingerprint: [u8; 32],
}
impl<'f, 's, 'p, 'a> ProductionTableMeasurements<'f, 's, 'p, 'a> {
    pub(super) fn shared_flow(&self) -> &ProductionPreparedBodyFlow<'f, 's, 'p, 'a> {
        &self.flow
    }
    pub fn tables(&self) -> &[ProductionMeasuredTable] {
        &self.tables
    }
    /// Read-only actual leaf measurement. Its index is not a page position.
    pub fn item(&self, index: usize) -> Option<&ProductionBodyFlowItem> {
        self.flow.collected.items.get(index)
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        lines: &ProductionInlineLineLayout<'_, '_>,
        blocks: &StagingPrecomposedVectorBlockLayout,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.flow.verify(lines, blocks, limits)
    }
}

/// Use the same leaf collector and list/definition marker metrics as the common
/// body flow. Keep table-containing streams private until table page selection
/// can consume their hierarchy; no flattened body stream escapes this API.
pub fn prepare_production_table_measurements<'f, 's, 'p, 'a>(
    lines: &'s ProductionInlineLineLayout<'p, 'a>,
    blocks: &'s StagingPrecomposedVectorBlockLayout,
    footnotes: &'f typaxis_layout::ProductionFootnoteLines<'s, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionTableMeasurements<'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let flow = prepare_body_flow_inner(lines, blocks, footnotes, limits, true)?;
    let projection = project_table_measurements(
        TableMeasurementInputs {
            collected: &flow.collected,
            sources: lines.source_flow().tables(),
            prior_records: flow.record_charge(),
            max_canonical_bytes: None,
            fingerprint_header: [
                sha256(b"typaxis.production-table-measurements/1"),
                lines.fingerprint(),
                blocks.receipt().fingerprint(),
                limits.fingerprint(),
            ],
        },
        limits,
    )?;
    Ok(ProductionTableMeasurements {
        flow,
        tables: projection.tables,
        record_charge: projection.record_charge,
        fingerprint: projection.fingerprint,
    })
}

pub(super) struct TableMeasurementInputs<'a> {
    pub collected: &'a CollectedItems,
    pub sources: &'a [typaxis_syntax::ProductionTable],
    pub prior_records: u64,
    pub max_canonical_bytes: Option<u64>,
    pub fingerprint_header: [[u8; 32]; 4],
}
pub(super) struct TableMeasurementProjection {
    pub tables: Vec<ProductionMeasuredTable>,
    pub record_charge: u64,
    pub fingerprint: [u8; 32],
}
pub(super) fn project_table_measurements(
    inputs: TableMeasurementInputs<'_>,
    limits: &M4EffectiveResourceLimits,
) -> Result<TableMeasurementProjection, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    let mut charge = Charge {
        remaining: limits
            .base()
            .get()
            .max_fragments
            .checked_sub(inputs.prior_records)
            .ok_or_else(|| error(root, E::FragmentLimit))?,
    };
    let sources = inputs.sources;
    let content_records =
        inputs
            .collected
            .tables
            .tables
            .iter()
            .try_fold(0usize, |sum, table| {
                let sum = if let Some(caption) = &table.caption {
                    let count = content_count(
                        &caption.items,
                        &caption.children,
                        &inputs.collected.tables.tables,
                        table.owner,
                    )?;
                    sum.checked_add(1)
                        .and_then(|n| n.checked_add(count))
                        .ok_or_else(|| error(table.owner, E::FragmentLimit))?
                } else {
                    sum
                };
                table.cells.iter().try_fold(sum, |sum, cell| {
                    sum.checked_add(content_count(
                        &cell.items,
                        &cell.children,
                        &inputs.collected.tables.tables,
                        table.owner,
                    )?)
                    .ok_or_else(|| error(table.owner, E::FragmentLimit))
                })
            })?;
    // Retained tables/cells/rows/content, scratch row sizes, verification prefix
    // work and rowspan visits are bounded before allocating the measurement.
    let records = sources
        .iter()
        .try_fold(sources.len(), |n, table| {
            let spans = table
                .cells()
                .iter()
                .try_fold(0usize, |n, c| n.checked_add(usize::from(c.rowspan().get())))?;
            n.checked_add(table.cells().len().checked_mul(2)?)?
                .checked_add(table.rows().len().checked_mul(3)?)?
                .checked_add(spans)
        })
        .and_then(|n| n.checked_add(content_records))
        .and_then(|n| n.checked_add(sources.len().checked_mul(2)?))
        .ok_or_else(|| error(root, E::FragmentLimit))?;
    charge.take(records, root)?;
    let mut reversed: Vec<ProductionMeasuredTable> = Vec::new();
    reversed
        .try_reserve_exact(sources.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    for index in (0..sources.len()).rev() {
        let source = &sources[index];
        let collected = &inputs.collected.tables.tables[index];
        let owner = source.owner();
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(source.cells().len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        let mut row_sizes = Vec::new();
        row_sizes
            .try_reserve_exact(source.rows().len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        row_sizes.resize(source.rows().len(), Length::ZERO);
        for (binding, cell) in source.cells().iter().zip(&collected.cells) {
            let (content, end) = measure_content(
                &cell.items,
                &cell.children,
                inputs.collected,
                &reversed,
                sources.len(),
                owner,
            )?;
            // Same source-order policy as layout_table_row_bands: place the
            // complete deficit in the last covered row, including zero bands.
            let start = binding.row() as usize;
            let last = start
                .checked_add(usize::from(binding.rowspan().get()))
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            let covered = row_sizes
                .get(start..last)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                .iter()
                .try_fold(Length::ZERO, |sum, n| add(sum, *n, owner))?;
            if end > covered {
                let deficit = end
                    .checked_sub(covered)
                    .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                row_sizes[last - 1] = add(row_sizes[last - 1], deficit, owner)?;
            }
            cells.push(ProductionMeasuredTableCell {
                owner: binding.owner(),
                source_index: cell.source_index,
                natural_height: end,
                content,
            });
        }
        let caption = collected
            .caption
            .as_ref()
            .map(|caption| {
                let (content, height) = measure_content(
                    &caption.items,
                    &caption.children,
                    inputs.collected,
                    &reversed,
                    sources.len(),
                    owner,
                )?;
                Ok::<_, ProductionBodyPaginationError>(ProductionMeasuredTableCaption {
                    height,
                    content,
                })
            })
            .transpose()?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(source.rows().len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        let mut height = caption.as_ref().map_or(Length::ZERO, |c| c.height);
        for (row, size) in source.rows().iter().zip(row_sizes) {
            rows.push(ProductionMeasuredTableRow {
                owner: row.owner(),
                section: row.section(),
                top: height,
                height: size,
            });
            height = add(height, size, owner)?;
        }
        for (binding, cell) in source.cells().iter().zip(&cells) {
            let start = binding.row() as usize;
            let end = start + usize::from(binding.rowspan().get());
            let bottom = rows.get(end).map_or(height, |r| r.top);
            let available = bottom
                .checked_sub(rows[start].top)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            if available < cell.natural_height {
                return Err(error(cell.owner, E::ReceiptMismatch));
            }
        }
        reversed.push(ProductionMeasuredTable {
            owner,
            source_index: index,
            cells,
            caption,
            rows,
            height,
            before: collected.before,
            after: collected.after,
            keep: collected.keep,
            keep_together: collected.keep_together,
        });
    }
    reversed.reverse();
    let canonical_capacity = records
        .checked_mul(64)
        .and_then(|n| n.checked_add(128))
        .ok_or_else(|| error(root, E::FragmentLimit))?;
    if inputs
        .max_canonical_bytes
        .is_some_and(|limit| canonical_capacity as u64 > limit)
    {
        return Err(error(root, E::SpoolLimit));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(canonical_capacity)
        .map_err(|_| error(root, E::AllocationFailure))?;
    for value in inputs.fingerprint_header {
        bytes.extend_from_slice(&value);
    }
    for table in &reversed {
        bytes.extend_from_slice(&table.owner.get().to_be_bytes());
        bytes.push(u8::from(table.keep));
        bytes.push(u8::from(table.keep_together));
        for n in [table.height, table.before, table.after] {
            bytes.extend_from_slice(&n.raw().to_be_bytes());
        }
        if let Some(caption) = &table.caption {
            bytes.extend_from_slice(b"caption");
            bytes.extend_from_slice(&caption.height.raw().to_be_bytes());
            bytes.extend_from_slice(&(caption.content.len() as u64).to_be_bytes());
            for item in &caption.content {
                let (kind, index) = match item.source {
                    ProductionTableContentSource::FlowItem(i) => (0, i),
                    ProductionTableContentSource::Table(i) => (1, i),
                };
                bytes.push(kind);
                bytes.extend_from_slice(&(index as u64).to_be_bytes());
                bytes.extend_from_slice(&item.top.raw().to_be_bytes());
                bytes.extend_from_slice(&item.end.raw().to_be_bytes());
            }
        }
        for row in &table.rows {
            bytes.extend_from_slice(&row.owner.get().to_be_bytes());
            for n in [row.top, row.height] {
                bytes.extend_from_slice(&n.raw().to_be_bytes());
            }
        }
        for cell in &table.cells {
            bytes.extend_from_slice(&cell.owner.get().to_be_bytes());
            bytes.extend_from_slice(&cell.natural_height.raw().to_be_bytes());
            for item in &cell.content {
                let (kind, index) = match item.source {
                    ProductionTableContentSource::FlowItem(i) => (0, i),
                    ProductionTableContentSource::Table(i) => (1, i),
                };
                bytes.push(kind);
                bytes.extend_from_slice(&(index as u64).to_be_bytes());
                for n in [item.top, item.end] {
                    bytes.extend_from_slice(&n.raw().to_be_bytes());
                }
            }
        }
    }
    Ok(TableMeasurementProjection {
        tables: reversed,
        record_charge: limits.base().get().max_fragments - charge.remaining,
        fingerprint: sha256(&bytes),
    })
}

fn content_count(
    items: &std::ops::Range<usize>,
    children: &[usize],
    tables: &[table_collection::Table],
    owner: NodeId,
) -> Result<usize, ProductionBodyPaginationError> {
    let mut covered = 0usize;
    let mut end = items.start;
    for index in children {
        let child = tables
            .get(*index)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        if !child
            .parent
            .and_then(|index| tables.get(index))
            .is_some_and(|parent| parent.owner == owner && parent.definition == child.definition)
            || child.items.start < end
            || child.items.end < child.items.start
            || child.items.end > items.end
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        covered = covered
            .checked_add(child.items.len())
            .ok_or_else(|| error(owner, E::FragmentLimit))?;
        end = child.items.end;
    }
    items
        .len()
        .checked_sub(covered)
        .and_then(|n| n.checked_add(children.len()))
        .ok_or_else(|| error(owner, E::ReceiptMismatch))
}

fn measure_content(
    items: &std::ops::Range<usize>,
    children: &[usize],
    collected: &CollectedItems,
    reversed: &[ProductionMeasuredTable],
    source_count: usize,
    owner: NodeId,
) -> Result<(Vec<ProductionTableCellContent>, Length), ProductionBodyPaginationError> {
    let mut content = Vec::new();
    let count = content_count(items, children, &collected.tables.tables, owner)?;
    content
        .try_reserve_exact(count)
        .map_err(|_| error(owner, E::AllocationFailure))?;
    let mut cursor = items.start;
    let mut child_cursor = 0;
    let mut end = Length::ZERO;
    while cursor < items.end || child_cursor < children.len() {
        let nested = children
            .get(child_cursor)
            .copied()
            .filter(|i| collected.tables.tables[*i].items.start == cursor);
        let (content_source, top, next_end, next_cursor) = if let Some(child_index) = nested {
            let child = reversed
                .get(source_count - child_index - 1)
                .filter(|t| t.source_index == child_index)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let range = &collected.tables.tables[child_index].items;
            if range.end < cursor || range.end > items.end {
                return Err(error(owner, E::ReceiptMismatch));
            }
            let top = add(end, child.before, owner)?;
            child_cursor += 1;
            (
                ProductionTableContentSource::Table(child_index),
                top,
                add(add(top, child.height, owner)?, child.after, owner)?,
                range.end,
            )
        } else {
            if cursor >= items.end {
                return Err(error(owner, E::ReceiptMismatch));
            }
            let item = &collected.items[cursor];
            let start = add(end, item.before, item.owner)?;
            let top = add(start, item.leading, item.owner)?;
            (
                ProductionTableContentSource::FlowItem(cursor),
                top,
                add(
                    add(start, item.consumed()?, item.owner)?,
                    item.after,
                    item.owner,
                )?,
                cursor + 1,
            )
        };
        content.push(ProductionTableCellContent {
            source: content_source,
            top,
            end: next_end,
        });
        end = next_end;
        cursor = next_cursor;
    }
    if child_cursor != children.len() || content.len() != count {
        return Err(error(owner, E::ReceiptMismatch));
    }
    Ok((content, end))
}
