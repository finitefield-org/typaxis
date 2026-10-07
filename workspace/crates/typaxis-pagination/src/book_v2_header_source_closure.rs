//! Original-unit closure for paint whose line ordinals differ from the base.
use super::*;
use std::ops::Range;

struct SourceGroup {
    owner: NodeId,
    items: Range<usize>,
    paragraph: Option<u32>,
}
impl SourceGroup {
    fn key(&self) -> (u8, u32) {
        self.paragraph.map_or((1, self.owner.get()), |p| (0, p))
    }
}
pub(super) struct OriginalHeaderSources {
    groups: Vec<SourceGroup>,
    units: Vec<Option<Range<u32>>>,
}
impl OriginalHeaderSources {
    pub(super) fn prepare(
        search: &mut BookV2FootnoteDemandSearch<'_, '_, '_, '_, '_>,
    ) -> Result<Self, ProductionBodyPaginationError> {
        let flow = search.content.flow;
        let root = NodeId::new(0);
        let count = flow.collected.items.len();
        search.content.charge.take(
            count
                .checked_mul(2)
                .and_then(|n| n.checked_add(2))
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut groups: Vec<SourceGroup> = Vec::new();
        let mut units = Vec::new();
        groups
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        units
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        for (index, item) in flow.collected.items.iter().enumerate() {
            let owner = item.owner();
            search.content.step(owner)?;
            let (paragraph, range) = match item.source() {
                Some(ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                }) => {
                    let p = flow
                        .lines()
                        .paragraphs()
                        .get(paragraph_index as usize)
                        .filter(|p| p.owner() == owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let line = p
                        .selected()
                        .and_then(|s| s.lines().get(line_index as usize))
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                        .line();
                    (
                        Some(paragraph_index),
                        Some(line.start_unit()..line.end_unit()),
                    )
                }
                _ => (None, None),
            };
            if let Some(last) = groups
                .last_mut()
                .filter(|g| g.owner == owner && g.paragraph == paragraph && paragraph.is_some())
            {
                if paragraph.is_none() || last.paragraph != paragraph || last.items.end != index {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                last.items.end = index + 1;
            } else {
                groups.push(SourceGroup {
                    owner,
                    items: index..index + 1,
                    paragraph,
                });
            }
            units.push(range);
        }
        let sort_work = (groups.len() as u64)
            .checked_mul(u64::from(groups.len().checked_ilog2().unwrap_or(0)) + 1)
            .and_then(|n| n.checked_mul(64))
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        for _ in 0..sort_work {
            search.content.step(root)?;
        }
        groups.sort_unstable_by_key(SourceGroup::key);
        for pair in groups.windows(2) {
            search.content.step(root)?;
            if pair[0].key() == pair[1].key() {
                return Err(error(pair[0].owner, E::ReceiptMismatch));
            }
        }
        Ok(Self { groups, units })
    }
    fn repeat(
        &self,
        search: &mut BookV2FootnoteDemandSearch<'_, '_, '_, '_, '_>,
        source: &BookV2TableWidthSource,
        actual_source: ProductionBodyFragmentSource,
        visits: &mut [SourceVisit],
    ) -> Result<(), ProductionBodyPaginationError> {
        let owner = source.owner();
        let paragraph = match actual_source {
            ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index, ..
            } => Some(paragraph_index),
            _ => None,
        };
        search.source_lookup_work(self.groups.len(), owner)?;
        let index = self
            .groups
            .binary_search_by_key(
                &paragraph.map_or((1, owner.get()), |p| (0, p)),
                SourceGroup::key,
            )
            .map_err(|_| error(owner, E::ReceiptMismatch))?;
        let group = &self.groups[index];
        if group.owner != owner {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let range = match source {
            BookV2TableWidthSource::Paragraph { units, .. } => {
                if group.paragraph.is_none() {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                let local = covered_lines(
                    &self.units[group.items.clone()],
                    units.clone(),
                    owner,
                    &mut || search.content.step(owner),
                )?;
                group.items.start + local.start..group.items.start + local.end
            }
            BookV2TableWidthSource::Block { .. } => {
                if group.paragraph.is_some()
                    || group.items.len() != 1
                    || search.content.flow.collected.items[group.items.start].source()
                        != Some(actual_source)
                {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                group.items.clone()
            }
            BookV2TableWidthSource::ForcedBreak { .. } => {
                return Err(error(owner, E::ReceiptMismatch))
            }
        };
        for index in range {
            search.content.step(owner)?;
            visits
                .get_mut(index)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                .consume(true, owner)?;
        }
        Ok(())
    }
}

/// Locate all original line records intersecting one repeated unit interval.
/// The binary lookup and every examined interval are charged before inspection.
fn covered_lines(
    ranges: &[Option<Range<u32>>],
    target: Range<u32>,
    owner: NodeId,
    step: &mut impl FnMut() -> Result<(), ProductionBodyPaginationError>,
) -> Result<Range<usize>, ProductionBodyPaginationError> {
    if target.start > target.end {
        return Err(error(owner, E::ReceiptMismatch));
    }
    let mut low = 0;
    let mut high = ranges.len();
    while low < high {
        step()?;
        let mid = low + (high - low) / 2;
        let range = ranges[mid]
            .as_ref()
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let before = if target.is_empty() {
            range.end < target.start
        } else {
            range.end <= target.start
        };
        if before {
            low = mid + 1
        } else {
            high = mid
        }
    }
    let first = low;
    let mut at = target.start;
    if target.is_empty() {
        for (index, range) in ranges.iter().enumerate().skip(first) {
            step()?;
            let range = range
                .as_ref()
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            if *range == target {
                return Ok(index..index + 1);
            }
            if range.start > target.start {
                break;
            }
        }
    } else {
        for (index, range) in ranges.iter().enumerate().skip(first) {
            step()?;
            let range = range
                .as_ref()
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            if range.start > at || range.end <= at || (index != first && range.start != at) {
                return Err(error(owner, E::ReceiptMismatch));
            }
            at = range.end.min(target.end);
            if at == target.end {
                return Ok(first..index + 1);
            }
        }
    }
    Err(error(owner, E::ReceiptMismatch))
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    pub(super) fn close_header_page_copies(
        &mut self,
        page: &BookV2BodyMixedPlacedPage<'_, 'b, 'f, 's, 'p, 'a>,
        sources: &OriginalHeaderSources,
        visits: &mut [SourceVisit],
    ) -> Result<usize, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let mut next = 0usize;
        let mut last = None;
        let mut scans = page.selection().candidate().parts().len();
        if let Some(notes) = page.selection().candidate().footnotes() {
            for note in notes.fragments() {
                self.content.step(root)?;
                scans = scans
                    .checked_add(1)
                    .and_then(|n| {
                        n.checked_add(note.fragment().mixed().map_or(0, |m| m.parts().len()))
                    })
                    .ok_or_else(|| error(root, E::FragmentLimit))?;
            }
        }
        for _ in 0..scans {
            self.content.step(root)?;
        }
        let mut table =
            |selected: &crate::book_v2::BookV2TableFragmentSelection<'b, 'f, 's, 'p, 'a>,
             origin: Length,
             definition: Option<usize>|
             -> Result<(), ProductionBodyPaginationError> {
                self.content.step(root)?;
                for paint in selected.variant_placement_leaves() {
                    let paint = paint?;
                    self.content.step(root)?;
                    let Some((header, leaf_index)) = paint.header_source() else {
                        continue;
                    };
                    if !paint.repeated() || header.definition_index() != definition {
                        return Err(error(root, E::ReceiptMismatch));
                    }
                    let leaf = header
                        .leaves()
                        .get(leaf_index)
                        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                    let owner = leaf.source().owner();
                    self.content.step(owner)?;
                    let actual = page
                        .header_variants()
                        .get(next)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    next += 1;
                    if !std::ptr::eq(actual.header(), header)
                        || actual.global_item_index() != leaf.variant_item_index()
                        || last.is_some_and(|i| i >= actual.fragment_index())
                    {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    last = Some(actual.fragment_index());
                    let placed = page
                        .fragments()
                        .get(actual.fragment_index())
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    let role = page
                        .cell_roles()
                        .get(actual.fragment_index())
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if placed.definition_index() != definition
                        || role.map(|r| r.owner()) != leaf.cell_owner()
                        || placed.fragment().bounds().y() != add(origin, paint.top(), owner)?
                    {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    if let Some(role) = role {
                        if !role.repeated_header() {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                    } else {
                        self.source_lookup_work(page.repeated_caption_positions().len(), owner)?;
                        if page
                            .repeated_caption_positions()
                            .binary_search(&actual.fragment_index())
                            .is_err()
                        {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                    }
                    let flow = header.variant().flow();
                    let item = header
                        .variant()
                        .item(leaf.variant_item_index())
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if item.owner() != owner || item.source() != Some(placed.fragment().source()) {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    let region = definition
                        .map_or_else(
                            || Some(0..flow.collected.body_end),
                            |d| flow.collected.definitions.get(d).cloned(),
                        )
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if !region.contains(&leaf.variant_item_index())
                        || placed.item_index() != leaf.variant_item_index() - region.start
                    {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    match (leaf.source(), item.source()) {
                        (
                            BookV2TableWidthSource::Paragraph { units, .. },
                            Some(ProductionBodyFragmentSource::ParagraphLine {
                                paragraph_index,
                                line_index,
                            }),
                        ) => {
                            let p = flow
                                .lines()
                                .paragraphs()
                                .get(paragraph_index as usize)
                                .filter(|p| p.owner() == owner)
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                            let line = p
                                .selected()
                                .and_then(|s| s.lines().get(line_index as usize))
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                                .line();
                            if *units != (line.start_unit()..line.end_unit()) {
                                return Err(error(owner, E::ReceiptMismatch));
                            }
                        }
                        (BookV2TableWidthSource::Block { .. }, Some(source))
                            if !matches!(
                                source,
                                ProductionBodyFragmentSource::ParagraphLine { .. }
                            ) => {}
                        _ => return Err(error(owner, E::ReceiptMismatch)),
                    }
                    sources.repeat(
                        self,
                        leaf.source(),
                        item.source()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                        visits,
                    )?;
                }
                Ok(())
            };
        for part in page.selection().candidate().parts() {
            if let Some(selected) = part.table() {
                table(
                    selected,
                    add(page.selection().body_bounds().y(), part.top(), root)?,
                    None,
                )?;
            }
        }
        if let Some(notes) = page.selection().candidate().footnotes() {
            for note in notes.fragments() {
                if let Some(mixed) = note.fragment().mixed() {
                    let origin = add(
                        add(
                            page.selection()
                                .candidate()
                                .footnote_bounds()
                                .ok_or_else(|| error(root, E::ReceiptMismatch))?
                                .y(),
                            Length::from_raw(typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW)
                                .ok_or_else(|| error(root, E::ArithmeticOverflow))?,
                            root,
                        )?,
                        note.offset(),
                        root,
                    )?;
                    for part in mixed.parts() {
                        if let Some(selected) = part.table() {
                            table(
                                selected,
                                add(origin, part.top(), root)?,
                                Some(note.fragment().definition_index()),
                            )?;
                        }
                    }
                }
            }
        }
        if next != page.header_variants().len() {
            return Err(error(root, E::ReceiptMismatch));
        }
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn covered(
        ranges: &[Range<u32>],
        target: Range<u32>,
    ) -> Result<Range<usize>, ProductionBodyPaginationError> {
        covered_lines(
            &ranges.iter().cloned().map(Some).collect::<Vec<_>>(),
            target,
            NodeId::new(1),
            &mut || Ok(()),
        )
    }
    #[test]
    fn repeated_units_cover_different_original_line_partitions() {
        assert_eq!(covered(&[0..3, 3..6, 6..9], 1..8).unwrap(), 0..3);
        assert_eq!(covered(&[0..9], 3..6).unwrap(), 0..1);
        assert_eq!(covered(&[0..3, 3..6], 0..3).unwrap(), 0..1);
        assert_eq!(covered(&[0..3, 3..6], 3..6).unwrap(), 1..2);
    }
    #[test]
    fn repeated_units_reject_missing_and_outside_source() {
        for (ranges, target) in [
            (vec![0..3, 4..7], 2..6),
            (vec![0..5, 4..8], 3..7),
            (vec![0..3], 2..4),
            (vec![2..5], 0..3),
            (vec![0..3], 3..2),
        ] {
            assert!(covered(&ranges, target).is_err());
        }
    }
    #[test]
    fn repeated_empty_units_require_an_original_empty_line() {
        assert_eq!(covered(&[0..0], 0..0).unwrap(), 0..1);
        assert!(covered(&[0..3, 3..6], 3..3).is_err());
        assert!(covered(&[], 0..0).is_err());
    }
    #[test]
    fn repeated_unit_lookup_and_walk_share_the_work_limit() {
        let ranges = [Some(0..3), Some(3..6), Some(6..9)];
        let mut used = 0;
        covered_lines(&ranges, 1..8, NodeId::new(1), &mut || {
            used += 1;
            Ok(())
        })
        .unwrap();
        for limit in [used, used - 1] {
            let mut work = 0;
            let result = covered_lines(&ranges, 1..8, NodeId::new(1), &mut || {
                work += 1;
                if work > limit {
                    Err(error(NodeId::new(1), E::FootnoteSearchLimit))
                } else {
                    Ok(())
                }
            });
            assert_eq!(result.is_ok(), limit == used);
        }
    }
}
