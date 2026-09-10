//! Column frames issued from the same source and inline owner as body shaping.
use super::*;
use typaxis_syntax::ProductionTable;

pub struct ProductionTableFrame {
    source_index: u32,
    owner: NodeId,
    content: ProductionInlineFrame,
    columns: Vec<crate::ResolvedTableColumn>,
    offsets: Vec<Length>,
    rounding_residual: Length,
    last_fraction: Option<u32>,
}
impl ProductionTableFrame {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn source_index(&self) -> u32 {
        self.source_index
    }
    pub const fn content(&self) -> ProductionInlineFrame {
        self.content
    }
    pub fn columns(&self) -> &[crate::ResolvedTableColumn] {
        &self.columns
    }
    pub const fn rounding_residual(&self) -> Length {
        self.rounding_residual
    }
    pub const fn last_fraction(&self) -> Option<u32> {
        self.last_fraction
    }
    pub(super) fn cell(
        &self,
        source: &ProductionTable,
        owner: NodeId,
    ) -> Result<ProductionInlineFrame, ProductionInlinePreparationError> {
        use ProductionInlinePreparationErrorKind as E;
        if source.owner() != self.owner {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let i = source
            .cells()
            .binary_search_by_key(&owner, |c| c.owner())
            .map_err(|_| error(owner, E::ReceiptMismatch))?;
        let cell = &source.cells()[i];
        let start = cell.column() as usize;
        let end = start
            .checked_add(usize::from(cell.colspan().get()))
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        let left = *self
            .offsets
            .get(start)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let right = *self
            .offsets
            .get(end)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        Ok(ProductionInlineFrame {
            start: self
                .content
                .start
                .checked_add(left)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
            width: right
                .checked_sub(left)
                .and_then(PositiveLength::new)
                .ok_or_else(|| error(owner, E::InvalidTableColumns))?,
        })
    }
    pub(super) fn encode(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&self.owner.get().to_be_bytes());
        bytes.extend_from_slice(&self.source_index.to_be_bytes());
        bytes.extend_from_slice(&self.rounding_residual.raw().to_be_bytes());
        bytes.extend_from_slice(&self.last_fraction.unwrap_or(u32::MAX).to_be_bytes());
        bytes.extend_from_slice(&(self.columns.len() as u32).to_be_bytes());
        for column in &self.columns {
            bytes.extend_from_slice(&column.final_width().get().raw().to_be_bytes());
        }
    }
}
pub(super) fn prepare(
    source: &ProductionTable,
    index: usize,
    parent: ProductionInlineFrame,
) -> Result<ProductionTableFrame, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let owner = source.owner();
    let style = source.style().block_style();
    let content = ProductionInlineFrame {
        start: parent
            .start
            .checked_add(style.start_indent().get())
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?,
        width: parent
            .width
            .get()
            .checked_sub(style.start_indent().get())
            .and_then(|w| w.checked_sub(style.end_indent().get()))
            .and_then(PositiveLength::new)
            .ok_or_else(|| error(owner, E::InvalidTableColumns))?,
    };
    // Reuse the exact fixed/fraction rounding policy, not an independent table
    // layout or legacy placement receipt. No cell prefix is rescanned per cell.
    let (columns, rounding_residual, last_fraction) =
        crate::resolve_table_columns(source.columns(), content.width).map_err(|cause| {
            error(
                owner,
                match cause {
                    crate::TableGridLayoutError::AllocationFailure => E::AllocationFailure,
                    _ => E::InvalidTableColumns,
                },
            )
        })?;
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(
            columns
                .len()
                .checked_add(1)
                .ok_or_else(|| error(owner, E::UnitLimit))?,
        )
        .map_err(|_| error(owner, E::AllocationFailure))?;
    let mut x = Length::ZERO;
    offsets.push(x);
    for column in &columns {
        x = x
            .checked_add(column.final_width().get())
            .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
        offsets.push(x);
    }
    if x != content.width.get() {
        return Err(error(owner, E::ReceiptMismatch));
    }
    Ok(ProductionTableFrame {
        source_index: u32::try_from(index).map_err(|_| error(owner, E::UnitLimit))?,
        owner,
        content,
        columns,
        offsets,
        rounding_residual,
        last_fraction,
    })
}
