//! Transactional legal prefixes for nested parent-cell keep chains.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Checkpoint<'m, 'f, 's, 'p, 'a> {
    cell: Cell<'m, 'f, 's, 'p, 'a>,
    pub top: Length,
    leaves: usize,
    semantic: usize,
    breaks: usize,
    progress: bool,
    forced_end: Option<Length>,
}
impl<'m, 'f, 's, 'p, 'a> Checkpoint<'m, 'f, 's, 'p, 'a> {
    pub fn capture(
        trial: &Trial<'m, 'f, 's, 'p, 'a>,
        i: usize,
        top: Length,
        forced_end: Option<Length>,
    ) -> Self {
        Self {
            cell: trial.cells[i],
            top,
            leaves: trial.projection.leaves.len(),
            semantic: trial.projection.semantic.len(),
            breaks: trial.breaks.len(),
            progress: trial.progress,
            forced_end,
        }
    }
    pub fn restore(
        self,
        trial: &mut Trial<'m, 'f, 's, 'p, 'a>,
        i: usize,
        forced_end: &mut Option<Length>,
    ) {
        trial.cells[i] = self.cell;
        trial.projection.leaves.truncate(self.leaves);
        trial.projection.semantic.truncate(self.semantic);
        trial.breaks.truncate(self.breaks);
        trial.progress = self.progress;
        *forced_end = self.forced_end;
    }
}
impl BookV2TableBreakSearch<'_, '_, '_, '_, '_> {
    pub(super) fn nested_child_earlier(
        &mut self,
        selected: &BookV2TableFragmentSelection<'_, '_, '_, '_, '_>,
    ) -> Result<Option<Length>, ProductionBodyPaginationError> {
        let mut end = Length::ZERO;
        let owner = self.measurements.tables()[self.table_index].owner;
        if selected.has_header_variants() {
            for leaf in selected.variant_placement_leaves() {
                self.kernel.work.take(1, owner)?;
                let leaf = leaf?;
                let item = &leaf.measurements().flow().collected.items[leaf.global_item_index()];
                end = end.max(add(
                    add(leaf.top(), item.height, owner)?,
                    item.trailing,
                    owner,
                )?);
            }
        } else {
            let caption = selected.caption_placement_leaves();
            let cells = selected
                .placement_leaves()
                .map(|v| v.map(|(_, i, t, _)| (i, t)));
            for leaf in caption.chain(cells) {
                self.kernel.work.take(1, owner)?;
                let (index, top) = leaf?;
                let item = &self.measurements.flow().collected.items[index];
                end = end.max(add(add(top, item.height, owner)?, item.trailing, owner)?);
            }
        }
        Ok(if end < selected.used_height() {
            Some(end)
        } else if end > Length::ZERO {
            end.checked_sub(Length::from_raw(1).expect("one layout unit"))
        } else {
            None
        })
    }
}
