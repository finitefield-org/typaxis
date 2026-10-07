//! Resolve global table-leaf indexes to their original body or definition stream.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2TableSourceLeaf {
    definition: Option<usize>,
    cell: Option<NodeId>,
    item: usize,
    top: Length,
    repeated: bool,
}
impl BookV2TableSourceLeaf {
    pub fn definition_index(&self) -> Option<usize> {
        self.definition
    }
    pub fn cell_owner(&self) -> Option<NodeId> {
        self.cell
    }
    /// Local item index in the original body or selected definition stream.
    pub fn item_index(&self) -> usize {
        self.item
    }
    pub fn top(&self) -> Length {
        self.top
    }
    pub fn repeated_header(&self) -> bool {
        self.repeated
    }
}
impl BookV2TableCursor<'_, '_, '_, '_, '_> {
    pub fn definition_index(&self) -> Option<usize> {
        self.measurements.flow().collected.tables.tables[self.table_index].definition
    }
    fn source_range(&self) -> Range<usize> {
        let flow = self.measurements.flow();
        self.definition_index()
            .map_or(0..flow.collected.body_end, |i| {
                flow.collected.definitions[i].clone()
            })
    }
}
impl BookV2TableFragmentSelection<'_, '_, '_, '_, '_> {
    pub fn definition_index(&self) -> Option<usize> {
        self.before.definition_index()
    }
    /// Original semantic ranges expressed locally in the owning stream. Includes
    /// consumed source break commands, never repeated-header copies.
    pub fn source_leaf_ranges(
        &self,
    ) -> impl Iterator<Item = Result<Range<usize>, ProductionBodyPaginationError>> + '_ {
        let region = self.before.source_range();
        let owner = self.before.measurements.tables()[self.before.table_index].owner;
        self.semantic_leaf_ranges().map(move |range| {
            if range.start < region.start || range.end > region.end || range.start > range.end {
                return Err(error(owner, E::ReceiptMismatch));
            }
            Ok(range.start - region.start..range.end - region.start)
        })
    }
    /// Actual caption/cell paint leaves with original stream ownership. A nested
    /// caption keeps its absent cell owner and its independent repetition flag.
    pub fn source_placement_leaves(
        &self,
    ) -> impl Iterator<Item = Result<BookV2TableSourceLeaf, ProductionBodyPaginationError>> + '_
    {
        let region = self.before.source_range();
        let definition = self.definition_index();
        let owner = self.before.measurements.tables()[self.before.table_index].owner;
        let captions = self
            .caption_placement_leaves_with_repetition()
            .map(|leaf| leaf.map(|(i, top, repeated)| (None, i, top, repeated)));
        let cells = self
            .placement_leaves()
            .map(|leaf| leaf.map(|(cell, i, top, repeated)| (Some(cell), i, top, repeated)));
        captions.chain(cells).map(move |leaf| {
            let (cell, index, top, repeated) = leaf?;
            if !region.contains(&index) {
                return Err(error(owner, E::ReceiptMismatch));
            }
            Ok(BookV2TableSourceLeaf {
                definition,
                cell,
                item: index - region.start,
                top,
                repeated,
            })
        })
    }
}
