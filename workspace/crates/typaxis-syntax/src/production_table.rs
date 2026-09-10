//! Source-bound table topology for the common production layout.
use super::*;
use std::num::NonZeroU16;
use typaxis_core::{Length, PositiveLength};
use typaxis_document::{ColumnSizing, TableColumn};
use typaxis_document_package::WireSemanticTableRow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionTableSection {
    Head,
    Body,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionTableRow {
    owner: NodeId,
    section: ProductionTableSection,
    ordinal: u32,
    cells_start: u32,
    cells_end: u32,
}
impl ProductionTableRow {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn section(&self) -> ProductionTableSection {
        self.section
    }
    /// Ordinal in the complete table, including its head rows.
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }
    pub fn cells(&self) -> std::ops::Range<usize> {
        self.cells_start as usize..self.cells_end as usize
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionTableCell {
    owner: NodeId,
    source_span: SourceSpan,
    row: u32,
    column: u32,
    colspan: NonZeroU16,
    rowspan: NonZeroU16,
}
impl ProductionTableCell {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn row(&self) -> u32 {
        self.row
    }
    pub const fn column(&self) -> u32 {
        self.column
    }
    pub const fn colspan(&self) -> NonZeroU16 {
        self.colspan
    }
    pub const fn rowspan(&self) -> NonZeroU16 {
        self.rowspan
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionTable {
    owner: NodeId,
    source_span: SourceSpan,
    pub(super) caption_events: Option<std::ops::Range<usize>>,
    columns: Vec<TableColumn>,
    rows: Vec<ProductionTableRow>,
    cells: Vec<ProductionTableCell>,
    style: SemanticContainerInheritanceStyle,
    page_name: Option<typaxis_core::PageName>,
}
impl ProductionTable {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    /// Original caption events before header/body rows; no caption is repeated
    /// as a header. This range has no page-layout authority by itself.
    pub fn caption_event_range(&self) -> Option<std::ops::Range<usize>> {
        self.caption_events.clone()
    }
    pub fn columns(&self) -> &[TableColumn] {
        &self.columns
    }
    pub fn rows(&self) -> &[ProductionTableRow] {
        &self.rows
    }
    pub fn cells(&self) -> &[ProductionTableCell] {
        &self.cells
    }
    pub fn style(&self) -> &SemanticContainerInheritanceStyle {
        &self.style
    }
    pub fn page_name(&self) -> Option<&typaxis_core::PageName> {
        self.page_name.as_ref()
    }
}
impl<'a, S: FlowSource<'a>> Collector<'a, S> {
    pub(super) fn table(
        &mut self,
        block: &WireSemanticBlock<S::Kind>,
        style: SemanticContainerInheritanceStyle,
    ) -> Result<(), ProductionFlowError> {
        use ProductionFlowErrorKind as E;
        let WireSemanticBlock::Table {
            node_id,
            span,
            columns,
            head,
            body,
            ..
        } = block
        else {
            return Err(failure(E::ReceiptMismatch, NodeId::new(block.node_id())));
        };
        let owner = NodeId::new(*node_id);
        let invalid = || failure(E::InvalidTableGrid, owner);
        let row_count = head.len().checked_add(body.len()).ok_or_else(invalid)?;
        if columns.is_empty() || row_count == 0 {
            return Err(invalid());
        }
        let cell_count = head
            .iter()
            .chain(body)
            .try_fold(0usize, |n, r| n.checked_add(r.cells.len()))
            .ok_or_else(invalid)?;
        // Bound both retained topology and occupancy visits before allocating or
        // walking a potentially sparse grid with long row/column spans.
        let charge = (columns.len() as u64)
            .checked_mul(row_count as u64)
            .and_then(|n| n.checked_add((columns.len() as u64).checked_mul(2)?))
            .and_then(|n| n.checked_add(row_count as u64))
            .and_then(|n| n.checked_add(cell_count as u64))
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| failure(E::NodeLimit, owner))?;
        self.table_record_charge = self
            .table_record_charge
            .checked_add(charge)
            .filter(|n| *n <= self.source.limits().get().max_fragments)
            .ok_or_else(|| failure(E::NodeLimit, owner))?;
        let mut typed = Vec::new();
        typed
            .try_reserve_exact(columns.len())
            .map_err(|_| failure(E::AllocationFailure, owner))?;
        for column in columns {
            let sizing = match column.get("kind").and_then(|v| v.as_str()) {
                Some("fixed") => ColumnSizing::Fixed(
                    column
                        .get("width")
                        .and_then(|v| v.as_i64())
                        .and_then(Length::from_raw)
                        .and_then(PositiveLength::new)
                        .ok_or_else(invalid)?,
                ),
                Some("fraction") => ColumnSizing::Fraction(
                    column
                        .get("weight")
                        .and_then(|v| v.as_u64())
                        .and_then(|n| u16::try_from(n).ok())
                        .and_then(NonZeroU16::new)
                        .ok_or_else(invalid)?,
                ),
                _ => return Err(invalid()),
            };
            typed.push(TableColumn { sizing });
        }
        let mut rows = Vec::new();
        let mut cells = Vec::new();
        rows.try_reserve_exact(row_count)
            .map_err(|_| failure(E::AllocationFailure, owner))?;
        cells
            .try_reserve_exact(cell_count)
            .map_err(|_| failure(E::AllocationFailure, owner))?;
        let mut remaining = Vec::new();
        remaining
            .try_reserve_exact(columns.len())
            .map_err(|_| failure(E::AllocationFailure, owner))?;
        remaining.resize(columns.len(), 0u16);
        for (section, source) in [
            (ProductionTableSection::Head, head.as_slice()),
            (ProductionTableSection::Body, body.as_slice()),
        ] {
            lower_section(section, source, &mut remaining, &mut rows, &mut cells)?;
        }
        let page_name = self
            .rules.cascade_ordinary("table", block.classes())
            .and_then(|s| s.page_name())
            .map_err(|_| failure(E::InvalidStyle, owner))?;
        self.tables
            .try_reserve(1)
            .map_err(|_| failure(E::AllocationFailure, owner))?;
        self.tables.push(ProductionTable {
            owner,
            source_span: lower_span(*span).map_err(|_| invalid())?,
            caption_events: None,
            columns: typed,
            rows,
            cells,
            style,
            page_name,
        });
        Ok(())
    }
}
fn lower_section<K>(
    section: ProductionTableSection,
    source: &[WireSemanticTableRow<K>],
    remaining: &mut [u16],
    rows: &mut Vec<ProductionTableRow>,
    cells: &mut Vec<ProductionTableCell>,
) -> Result<(), ProductionFlowError> {
    for (index, row) in source.iter().enumerate() {
        let owner = NodeId::new(row.node_id);
        let invalid = |owner| failure(ProductionFlowErrorKind::InvalidTableGrid, owner);
        let ordinal = u32::try_from(rows.len()).map_err(|_| invalid(owner))?;
        let start = u32::try_from(cells.len()).map_err(|_| invalid(owner))?;
        let mut cursor = 0;
        for cell in &row.cells {
            let cell_owner = NodeId::new(cell.node_id);
            while cursor < remaining.len() && remaining[cursor] != 0 {
                cursor += 1;
            }
            let colspan = NonZeroU16::new(cell.colspan).ok_or_else(|| invalid(cell_owner))?;
            let rowspan = NonZeroU16::new(cell.rowspan).ok_or_else(|| invalid(cell_owner))?;
            let end = cursor
                .checked_add(usize::from(colspan.get()))
                .filter(|end| *end <= remaining.len())
                .ok_or_else(|| invalid(cell_owner))?;
            if index
                .checked_add(usize::from(rowspan.get()))
                .is_none_or(|end| end > source.len())
                || remaining[cursor..end].iter().any(|n| *n != 0)
            {
                return Err(invalid(cell_owner));
            }
            remaining[cursor..end].fill(rowspan.get());
            cells.push(ProductionTableCell {
                owner: cell_owner,
                source_span: lower_span(cell.span).map_err(|_| invalid(cell_owner))?,
                row: ordinal,
                column: u32::try_from(cursor).map_err(|_| invalid(cell_owner))?,
                colspan,
                rowspan,
            });
            cursor = end;
        }
        if remaining.contains(&0) {
            return Err(invalid(owner));
        }
        for n in remaining.iter_mut() {
            *n -= 1;
        }
        rows.push(ProductionTableRow {
            owner,
            section,
            ordinal,
            cells_start: start,
            cells_end: u32::try_from(cells.len()).map_err(|_| invalid(owner))?,
        });
    }
    Ok(())
}
