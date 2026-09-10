//! Physical table occurrences mapped to complete original-source profiles.
use super::*;
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    fn table_piece_line_frame(
        &mut self,
        piece: &BookV2TableWidthPiece,
    ) -> Result<(usize, PositiveLength, Length), ProductionBodyPaginationError> {
        let BookV2TableWidthSource::Paragraph { owner, units } = piece.source() else {
            return Err(error(piece.source().owner(), E::ReceiptMismatch));
        };
        let lines = self.content.flow.lines();
        let frames = lines
            .frames()
            .ok_or_else(|| error(*owner, E::ReceiptMismatch))?;
        let mut found = None;
        for (index, p) in lines.paragraphs().iter().enumerate() {
            self.content.step(*owner)?;
            if p.owner() == *owner {
                found = Some((index, p));
                break;
            }
        }
        let (index, p) = found.ok_or_else(|| error(*owner, E::ReceiptMismatch))?;
        let selected = p
            .selected()
            .ok_or_else(|| error(*owner, E::ReceiptMismatch))?;
        for _ in 0..=selected.lines().len().checked_ilog2().unwrap_or(0) {
            self.content.step(*owner)?;
        }
        let at = selected
            .lines()
            .binary_search_by_key(&units.start, |s| s.line().start_unit())
            .map_err(|_| error(*owner, E::ReceiptMismatch))?;
        let line = &selected.lines()[at];
        if line.line().end_unit() != units.end {
            return Err(error(*owner, E::ReceiptMismatch));
        }
        let start = frames
            .source_unit_start(index, units.start)
            .unwrap_or(frames.paragraphs()[index].start());
        Ok((index, line.inline_size(), start))
    }
    fn verify_table_profile_roots(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
        report: &BookV2TableWidthOccurrences,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let flow = self.content.flow;
        let tables = &flow.collected.tables.tables;
        self.content.charge.take(tables.len(), root)?;
        let mut seen = Vec::new();
        seen.try_reserve_exact(tables.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        for _ in tables {
            self.content.step(root)?;
            seen.push(false);
        }
        for occurrence in report.occurrences() {
            let mut found = false;
            for (index, table) in tables.iter().enumerate() {
                self.content.step(occurrence.owner())?;
                if table.owner == occurrence.owner() {
                    if table.parent.is_some() || table.definition != occurrence.definition_index() {
                        return Err(error(table.owner, E::ReceiptMismatch));
                    }
                    seen[index] = true;
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(error(occurrence.owner(), E::ReceiptMismatch));
            }
        }
        let demand = closed
            .stable()
            .sequence()
            .pages()
            .last()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?
            .next_state()
            .source_state()
            .demand();
        for (table, seen) in tables.iter().zip(seen) {
            self.content.step(table.owner)?;
            if table.parent.is_none() {
                let expected = table.definition.is_none_or(|d| {
                    demand.status(d) == Some(ProductionFootnoteDemandStatus::Complete)
                });
                if expected != seen {
                    return Err(error(table.owner, E::ReceiptMismatch));
                }
            }
        }
        Ok(())
    }
    pub(super) fn collect_table_block_profiles(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<
        (Vec<(NodeId, PositiveLength)>, Option<Vec<Length>>, bool),
        ProductionBodyPaginationError,
    > {
        let (blocks, matches) = self.collect_block_width_feedback_inner(closed, true)?;
        if blocks.is_empty() {
            return Ok((blocks, None, matches));
        }
        let root = NodeId::new(0);
        self.content.charge.take(
            blocks
                .len()
                .checked_add(1)
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut starts = Vec::new();
        starts
            .try_reserve_exact(blocks.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        let frames = self
            .content
            .flow
            .lines()
            .frames()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        for &(owner, _) in &blocks {
            for _ in 0..frames.measurement_region_lookup_work() {
                self.content.step(owner)?;
            }
            starts.push(
                frames
                    .measurement_region(owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                    .start(),
            );
        }
        Ok((blocks, Some(starts), matches))
    }
    pub(super) fn collect_table_source_profiles(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
        paragraphs: &mut [BookV2ParagraphWidthCandidate],
        visits: &mut [Vec<u8>],
        blocks: &mut [(NodeId, PositiveLength)],
        mut block_starts: Option<&mut [Length]>,
    ) -> Result<(bool, bool), ProductionBodyPaginationError> {
        let report = self.table_width_frames(closed)?;
        self.verify_table_profile_roots(closed, &report)?;
        let frames = self
            .content
            .flow
            .lines()
            .frames()
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let mut matches = true;
        let mut blocks_match = true;
        let mut block_visits = Vec::new();
        if !blocks.is_empty() {
            let root = NodeId::new(0);
            self.content.charge.take(
                blocks
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| error(root, E::FragmentLimit))?,
                root,
            )?;
            block_visits
                .try_reserve_exact(blocks.len())
                .map_err(|_| error(root, E::AllocationFailure))?;
            for _ in blocks.iter() {
                self.content.step(root)?;
                block_visits.push(0u8);
            }
        }
        for occurrence in report.occurrences() {
            for piece in occurrence.pieces() {
                let owner = piece.source().owner();
                self.content.step(owner)?;
                if piece.uses_header_variant() {
                    // These units already have their one semantic assignment in the base.
                    // Require the independently projected physical frame to match the
                    // actual variant; do not overwrite base widths with repeated paint.
                    if !piece.repeated()
                        || piece.frame().is_none()
                        || piece.measured_header_frame()
                            != piece.frame().map(|f| (f.start(), f.width()))
                    {
                        return Err(error(owner, E::WidthMismatch));
                    }
                    continue;
                }
                match piece.source() {
                    BookV2TableWidthSource::Paragraph { units, .. } => {
                        let target = piece
                            .frame()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        let (index, width, start) = self.table_piece_line_frame(piece)?;
                        matches &= width == target.width() && start == target.start();
                        let candidate = paragraphs
                            .get_mut(index)
                            .filter(|p| p.owner == owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        if candidate.starts.is_none() {
                            self.content.charge.take(candidate.widths.len(), owner)?;
                            let mut starts = Vec::new();
                            starts
                                .try_reserve_exact(candidate.widths.len())
                                .map_err(|_| error(owner, E::AllocationFailure))?;
                            for _ in &candidate.widths {
                                self.content.step(owner)?;
                                starts.push(frames.paragraphs()[index].start());
                            }
                            candidate.starts = Some(starts);
                        }
                        let end = if units.is_empty() {
                            if units.start != 0
                                || self.content.flow.lines().prepared().paragraphs()[index]
                                    .items()
                                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                                    .units()
                                    .len()
                                    != 0
                            {
                                return Err(error(owner, E::ReceiptMismatch));
                            }
                            1
                        } else {
                            units.end as usize
                        };
                        let starts = candidate.starts.as_mut().unwrap();
                        for unit in units.start as usize..end {
                            self.content.step(owner)?;
                            let flag = visits
                                .get_mut(index)
                                .and_then(|v| v.get_mut(unit))
                                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                            if *flag != 0
                                && (candidate.widths[unit] != target.width()
                                    || starts[unit] != target.start())
                            {
                                return Err(error(
                                    owner,
                                    E::PendingRegion("table_repeated_frame_reflow"),
                                ));
                            }
                            if !piece.repeated() && *flag & 1 != 0 {
                                return Err(error(owner, E::ReceiptMismatch));
                            }
                            candidate.widths[unit] = target.width();
                            starts[unit] = target.start();
                            *flag |= if piece.repeated() { 2 } else { 1 };
                        }
                    }
                    BookV2TableWidthSource::Block { .. } => {
                        for _ in 0..frames.inherited_block_lookup_work() {
                            self.content.step(owner)?;
                        }
                        let target = piece
                            .frame()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        blocks_match &= frames.region(owner) == Some(target);
                        for _ in 0..blocks.len() {
                            self.content.step(owner)?;
                        }
                        let index = blocks
                            .iter()
                            .position(|b| b.0 == owner)
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        let starts = block_starts
                            .as_deref_mut()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        let seen = &mut block_visits[index];
                        if *seen != 0
                            && (blocks[index].1 != target.width()
                                || starts[index] != target.start())
                        {
                            return Err(error(
                                owner,
                                E::PendingRegion("table_repeated_frame_reflow"),
                            ));
                        }
                        if !piece.repeated() && *seen & 1 != 0 {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                        *seen |= if piece.repeated() { 2 } else { 1 };
                        blocks[index].1 = target.width();
                        starts[index] = target.start();
                    }
                    BookV2TableWidthSource::ForcedBreak { .. } => {}
                }
            }
        }
        for ((owner, _), seen) in blocks.iter().zip(block_visits) {
            self.content.step(*owner)?;
            if seen != 0 && seen & 1 == 0 {
                return Err(error(*owner, E::ReceiptMismatch));
            }
        }
        Ok((matches, blocks_match))
    }
    pub(in crate::production_body::body_flow) fn verify_table_source_frames(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let report = self.table_width_frames(closed)?;
        self.verify_table_profile_roots(closed, &report)?;
        let frames = self
            .content
            .flow
            .lines()
            .frames()
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        for occurrence in report.occurrences() {
            for piece in occurrence.pieces() {
                let owner = piece.source().owner();
                self.content.step(owner)?;
                if piece.uses_header_variant() {
                    // These units already have their one semantic assignment in the base.
                    // Require the independently projected physical frame to match the
                    // actual variant; do not overwrite base widths with repeated paint.
                    if !piece.repeated()
                        || piece.frame().is_none()
                        || piece.measured_header_frame()
                            != piece.frame().map(|f| (f.start(), f.width()))
                    {
                        return Err(error(owner, E::WidthMismatch));
                    }
                    continue;
                }
                match piece.source() {
                    BookV2TableWidthSource::Paragraph { .. } => {
                        let target = piece
                            .frame()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                        let (_, width, start) = self.table_piece_line_frame(piece)?;
                        if width != target.width() || start != target.start() {
                            return Err(error(owner, E::WidthMismatch));
                        }
                    }
                    BookV2TableWidthSource::Block { .. } => {
                        for _ in 0..frames.inherited_block_lookup_work() {
                            self.content.step(owner)?;
                        }
                        if frames.region(owner) != piece.frame() {
                            return Err(error(owner, E::WidthMismatch));
                        }
                    }
                    BookV2TableWidthSource::ForcedBreak { .. } => {}
                }
            }
        }
        Ok(())
    }
}
