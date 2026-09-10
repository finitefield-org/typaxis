//! Original paragraph-unit widths observed on source-closed physical pages.
//! These are candidates for a fresh shaping pass, not PDF/page authorization.
use super::*;
#[path = "book_v2_block_width_feedback.rs"]
mod block_width_feedback;
#[path = "book_v2_table_width_feedback.rs"]
mod table_width_feedback;
#[path = "book_v2_table_width_occurrences.rs"]
mod table_width_occurrences;
pub use table_width_occurrences::{
    BookV2TableWidthOccurrence, BookV2TableWidthOccurrences, BookV2TableWidthPiece,
    BookV2TableWidthSource,
};

pub struct BookV2ParagraphWidthCandidate {
    owner: NodeId,
    widths: Vec<PositiveLength>,
    retained_ends: Option<Vec<u32>>,
    starts: Option<Vec<Length>>,
    table_frame: bool,
}
impl BookV2ParagraphWidthCandidate {
    pub fn source_unit_starts(&self) -> Option<&[Length]> {
        self.starts.as_deref()
    }
    /// Widths in a table are observations of its current cell/caption frame.
    /// Rebind with None so a fresh root-table projection supplies the new width.
    pub fn uses_table_frame(&self) -> bool {
        self.table_frame
    }
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn widths(&self) -> &[PositiveLength] {
        &self.widths
    }
    pub fn retained_line_ends(&self) -> Option<&[u32]> {
        self.retained_ends.as_deref()
    }
}
pub struct BookV2ParagraphWidthFeedback {
    source_fingerprint: [u8; 32],
    occurrence_frames: bool,
    assignment_fingerprint: [u8; 32],
    paragraphs: Vec<BookV2ParagraphWidthCandidate>,
    blocks: Vec<(NodeId, PositiveLength)>,
    block_starts: Option<Vec<Length>>,
    blocks_match: bool,
    tables: Vec<(NodeId, PositiveLength)>,
    tables_match: bool,
    matches: bool,
    records: u64,
    work: u64,
}
impl BookV2ParagraphWidthFeedback {
    pub fn uses_table_occurrence_frames(&self) -> bool {
        self.occurrence_frames
    }
    pub fn root_table_widths(&self) -> &[(NodeId, PositiveLength)] {
        &self.tables
    }
    pub fn matches_selected_table_widths(&self) -> bool {
        self.tables_match
    }
    pub fn block_starts(&self) -> Option<&[Length]> {
        self.block_starts.as_deref()
    }
    pub fn block_widths(&self) -> &[(NodeId, PositiveLength)] {
        &self.blocks
    }
    pub fn matches_selected_block_widths(&self) -> bool {
        self.blocks_match
    }
    pub fn assignment_fingerprint(&self) -> [u8; 32] {
        self.assignment_fingerprint
    }
    pub fn paragraphs(&self) -> &[BookV2ParagraphWidthCandidate] {
        &self.paragraphs
    }
    /// Exact flow content, including generated labels, must match before replay.
    /// New item bindings still need the fresh preparation's exact source owner.
    pub fn matches_source_flow(
        &self,
        flow: &typaxis_syntax::book_v2::PreparedBookV2TextFlow<'_>,
    ) -> bool {
        self.source_fingerprint == flow.fingerprint()
    }
    /// Line targets match; physical acceptance also requires matching root-table
    /// and block targets, independently rechecked during finalization.
    pub fn matches_selected_line_widths(&self) -> bool {
        self.matches
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
}
impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Translate physical region capacities to original paragraph-unit starts.
    /// Require this search's exact closed source/geometry; never consume repeated
    /// headers twice or silently discard differing capacities for repeated text.
    pub fn paragraph_width_feedback(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2ParagraphWidthFeedback, ProductionBodyPaginationError> {
        closed.require_single_measurement("table_header_variant_width_feedback")?;
        self.collect_paragraph_frame_feedback(closed, false)
    }
    /// Include per-source table widths and origins from actual occurrence frames.
    pub fn paragraph_frame_feedback(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2ParagraphWidthFeedback, ProductionBodyPaginationError> {
        self.collect_paragraph_frame_feedback(closed, true)
    }
    fn collect_paragraph_frame_feedback(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
        occurrence_frames: bool,
    ) -> Result<BookV2ParagraphWidthFeedback, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        if !std::ptr::eq(closed.flow(), self.content.flow) {
            return Err(error(root, E::ReceiptMismatch));
        }
        self.verify_mixed_sequence(closed.stable().sequence())?;
        let lines = self.content.flow.lines();
        let frames = lines
            .frames()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let count = lines.paragraphs().len();
        self.content.charge.take(
            count
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut paragraphs = Vec::new();
        let mut visits = Vec::new();
        paragraphs
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        visits
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        for (p, prepared) in lines.paragraphs().iter().zip(lines.prepared().paragraphs()) {
            self.content.step(p.owner())?;
            let units = prepared
                .items()
                .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?
                .units()
                .len()
                .max(1);
            self.content.charge.take(
                units
                    .checked_mul(2)
                    .ok_or_else(|| error(p.owner(), E::FragmentLimit))?,
                p.owner(),
            )?;
            let mut widths = Vec::new();
            let mut seen = Vec::new();
            widths
                .try_reserve_exact(units)
                .map_err(|_| error(p.owner(), E::AllocationFailure))?;
            seen.try_reserve_exact(units)
                .map_err(|_| error(p.owner(), E::AllocationFailure))?;
            for _ in 0..units {
                self.content.step(p.owner())?;
                widths.push(p.inline_size());
                seen.push(0u8);
            }
            paragraphs.push(BookV2ParagraphWidthCandidate {
                owner: p.owner(),
                widths,
                retained_ends: None,
                starts: None,
                table_frame: false,
            });
            visits.push(seen);
        }
        let mut matches = true;
        for page in closed.geometry().pages() {
            self.content.step(root)?;
            for (fragment_index, (placed, _, repeated)) in page.fragments_with_roles().enumerate() {
                let fragment = placed.fragment();
                self.content.step(fragment.owner())?;
                if !page.header_variants().is_empty() {
                    self.source_lookup_work(page.header_variants().len(), fragment.owner())?;
                    if page.header_variant(fragment_index).is_some() {
                        if !occurrence_frames || !repeated {
                            return Err(error(fragment.owner(), E::ReceiptMismatch));
                        }
                        continue; // Independently checked in the occurrence report below.
                    }
                }
                let ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                } = fragment.source()
                else {
                    continue;
                };
                let p = lines
                    .paragraphs()
                    .get(paragraph_index as usize)
                    .ok_or_else(|| error(fragment.owner(), E::ReceiptMismatch))?;
                if p.owner() != fragment.owner() {
                    return Err(error(fragment.owner(), E::ReceiptMismatch));
                }
                let selected = p
                    .selected()
                    .and_then(|p| p.lines().get(line_index as usize))
                    .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
                let (measured, actual) = if placed.definition_index().is_some() {
                    (
                        frames
                            .footnote_region()
                            .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?,
                        page.selection()
                            .declared_footnote_region()
                            .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?,
                    )
                } else {
                    (frames.body(), page.selection().body_bounds())
                };
                let delta = actual
                    .width()
                    .get()
                    .checked_sub(measured.width().get())
                    .ok_or_else(|| error(p.owner(), E::ArithmeticOverflow))?;
                let in_table = self.fragment_root_table(placed)?.is_some();
                if occurrence_frames && in_table {
                    continue;
                }
                let delta = if in_table { Length::ZERO } else { delta };
                paragraphs[paragraph_index as usize].table_frame = in_table;
                let width = p
                    .inline_size()
                    .get()
                    .checked_add(delta)
                    .and_then(PositiveLength::new)
                    .ok_or_else(|| error(p.owner(), E::WidthMismatch))?;
                matches &= selected.inline_size() == width;
                let start = selected.line().start_unit() as usize;
                let mut end = selected.line().end_unit() as usize;
                if start == end {
                    if start != 0
                        || lines.prepared().paragraphs()[paragraph_index as usize]
                            .items()
                            .unwrap()
                            .units()
                            .len()
                            != 0
                    {
                        return Err(error(p.owner(), E::ReceiptMismatch));
                    }
                    end = 1;
                }
                let target = paragraphs
                    .get_mut(paragraph_index as usize)
                    .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
                let seen = visits
                    .get_mut(paragraph_index as usize)
                    .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
                let target = target
                    .widths
                    .get_mut(start..end)
                    .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
                let seen = seen
                    .get_mut(start..end)
                    .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
                for (target, seen) in target.iter_mut().zip(seen) {
                    self.content.step(p.owner())?;
                    if *seen != 0 && *target != width {
                        return Err(error(p.owner(), E::WidthMismatch));
                    }
                    if !repeated && *seen & 1 != 0 {
                        return Err(error(p.owner(), E::ReceiptMismatch));
                    }
                    *target = width;
                    *seen |= if repeated { 2 } else { 1 };
                }
            }
        }
        let mut block_profiles = if occurrence_frames {
            Some(self.collect_table_block_profiles(closed)?)
        } else {
            None
        };
        if let Some((blocks, starts, blocks_match)) = &mut block_profiles {
            let (paragraph_match, table_blocks_match) = self.collect_table_source_profiles(
                closed,
                &mut paragraphs,
                &mut visits,
                blocks,
                starts.as_deref_mut(),
            )?;
            matches &= paragraph_match;
            *blocks_match &= table_blocks_match;
        }
        let mut assignment_fingerprint = lines.prepared().source_flow().fingerprint();
        let fold = |hash: &mut [u8; 32], value: u64| {
            let mut bytes = [0u8; 40];
            bytes[..32].copy_from_slice(hash);
            bytes[32..].copy_from_slice(&value.to_be_bytes());
            *hash = typaxis_core::sha256(&bytes);
        };
        if occurrence_frames {
            self.content.step(root)?;
            fold(&mut assignment_fingerprint, 0x5441424c455033);
        }
        for (p, seen) in paragraphs.iter().zip(visits) {
            self.content.step(p.owner())?;
            fold(&mut assignment_fingerprint, u64::from(p.owner().get()));
            self.content.step(p.owner())?;
            fold(&mut assignment_fingerprint, p.widths.len() as u64);
            let mut any = false;
            let mut complete = true;
            for (flag, width) in seen.into_iter().zip(&p.widths) {
                self.content.step(p.owner())?;
                fold(&mut assignment_fingerprint, width.get().raw() as u64);
                any |= flag != 0;
                complete &= flag & 1 != 0;
            }
            // Entire unreferenced definitions may stay unobserved. Once any line
            // was placed, every original unit needs its one semantic occurrence.
            if any && !complete {
                return Err(error(p.owner(), E::ReceiptMismatch));
            }
            if occurrence_frames {
                self.content.step(p.owner())?;
                fold(&mut assignment_fingerprint, u64::from(p.starts.is_some()));
                if let Some(starts) = &p.starts {
                    for start in starts {
                        self.content.step(p.owner())?;
                        fold(&mut assignment_fingerprint, start.raw() as u64);
                    }
                }
            }
        }
        let (blocks, block_starts, blocks_match) = if let Some(profiles) = block_profiles {
            profiles
        } else {
            let (blocks, matches) = self.collect_block_width_feedback(closed)?;
            (blocks, None, matches)
        };
        for (owner, width) in &blocks {
            self.content.step(*owner)?;
            fold(&mut assignment_fingerprint, u64::from(owner.get()));
            self.content.step(*owner)?;
            fold(&mut assignment_fingerprint, width.get().raw() as u64);
        }
        if let Some(starts) = &block_starts {
            self.content.step(root)?;
            fold(&mut assignment_fingerprint, 0x424c4b53544131);
            for start in starts {
                self.content.step(root)?;
                fold(&mut assignment_fingerprint, start.raw() as u64);
            }
        }
        let (tables, tables_match) = if occurrence_frames {
            (Vec::new(), true)
        } else {
            self.collect_root_table_width_feedback(closed)?
        };
        for (owner, width) in &tables {
            self.content.step(*owner)?;
            fold(&mut assignment_fingerprint, u64::from(owner.get()));
            self.content.step(*owner)?;
            fold(&mut assignment_fingerprint, width.get().raw() as u64);
        }
        Ok(BookV2ParagraphWidthFeedback {
            source_fingerprint: lines.prepared().source_flow().fingerprint(),
            occurrence_frames,
            assignment_fingerprint,
            paragraphs,
            blocks,
            block_starts,
            blocks_match,
            tables,
            tables_match,
            matches,
            records: self.record_charge(),
            work: self.work_steps(),
        })
    }

    /// On a width cycle, preserve this source-closed partition as legal layout
    /// boundaries. Later passes may subdivide it, but must not merge across it.
    pub fn retain_paragraph_line_boundaries(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
        feedback: &mut BookV2ParagraphWidthFeedback,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        if !std::ptr::eq(closed.flow(), self.content.flow)
            || !feedback.matches_source_flow(self.content.flow.lines().prepared().source_flow())
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        self.verify_mixed_sequence(closed.stable().sequence())?;
        let paragraphs = self.content.flow.lines().paragraphs();
        if paragraphs.len() != feedback.paragraphs.len() {
            return Err(error(root, E::ReceiptMismatch));
        }
        for (p, candidate) in paragraphs.iter().zip(&mut feedback.paragraphs) {
            self.content.step(p.owner())?;
            if p.owner() != candidate.owner || candidate.retained_ends.is_some() {
                return Err(error(p.owner(), E::ReceiptMismatch));
            }
            if candidate.table_frame {
                continue;
            }
            let selected = p
                .selected()
                .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
            self.content
                .charge
                .take(selected.lines().len(), p.owner())?;
            let mut ends = Vec::new();
            ends.try_reserve_exact(selected.lines().len())
                .map_err(|_| error(p.owner(), E::AllocationFailure))?;
            for line in selected.lines() {
                self.content.step(p.owner())?;
                ends.push(line.line().end_unit());
            }
            candidate.retained_ends = Some(ends);
        }
        feedback.records = self.record_charge();
        feedback.work = self.work_steps();
        Ok(())
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    /// Finalization must independently reject provisional widths; a driver-side
    /// match flag alone cannot authorize drawing from another component caller.
    pub(super) fn verify_physical_paragraph_widths(
        &mut self,
        closed: &BookV2BodySourceClosure<'_, '_, 'b, 'f, 's, 'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        if !std::ptr::eq(closed.flow(), self.content.flow) {
            return Err(error(root, E::ReceiptMismatch));
        }
        self.verify_mixed_sequence(closed.stable().sequence())?;
        let lines = self.content.flow.lines();
        let frames = lines
            .frames()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        if closed.has_header_variants() {
            self.verify_table_source_frames(closed)?;
        }
        for page in closed.geometry().pages() {
            self.content.step(root)?;
            for (fragment_index, placed) in page.fragments().iter().enumerate() {
                let fragment = placed.fragment();
                self.content.step(fragment.owner())?;
                if !page.header_variants().is_empty() {
                    self.source_lookup_work(page.header_variants().len(), fragment.owner())?;
                    if page.header_variant(fragment_index).is_some() {
                        continue; // Actual frame was checked above through the variant owner.
                    }
                }
                if frames.uses_table_occurrence_frames()
                    && self.fragment_root_table(placed)?.is_some()
                {
                    continue; // Independently remeasured and checked above.
                }
                let ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                } = fragment.source()
                else {
                    let target = self.physical_block_parent_width(page, placed)?;
                    if frames.region(fragment.owner()).map(|f| f.width()) != Some(target) {
                        return Err(error(fragment.owner(), E::WidthMismatch));
                    }
                    if frames.has_block_source_starts() {
                        let owner = fragment.owner();
                        for _ in 0..frames.measurement_region_lookup_work()
                            + frames.inherited_block_lookup_work()
                        {
                            self.content.step(owner)?;
                        }
                        let expected = if self.fragment_root_table(placed)?.is_some() {
                            frames.inherited_block_region(owner)
                        } else {
                            frames.measurement_region(owner)
                        };
                        if frames.region(owner).map(|f| f.start()) != expected.map(|f| f.start()) {
                            return Err(error(owner, E::WidthMismatch));
                        }
                    }
                    continue;
                };
                let p = lines
                    .paragraphs()
                    .get(paragraph_index as usize)
                    .ok_or_else(|| error(fragment.owner(), E::ReceiptMismatch))?;
                let selected = p
                    .selected()
                    .and_then(|p| p.lines().get(line_index as usize))
                    .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
                let (measured, actual) = if placed.definition_index().is_some() {
                    (
                        frames
                            .footnote_region()
                            .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?,
                        page.selection()
                            .declared_footnote_region()
                            .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?,
                    )
                } else {
                    (frames.body(), page.selection().body_bounds())
                };
                let delta = if self.fragment_root_table(placed)?.is_some() {
                    Length::ZERO
                } else {
                    actual
                        .width()
                        .get()
                        .checked_sub(measured.width().get())
                        .ok_or_else(|| error(p.owner(), E::ArithmeticOverflow))?
                };
                let expected = p
                    .inline_size()
                    .get()
                    .checked_add(delta)
                    .and_then(PositiveLength::new)
                    .ok_or_else(|| error(p.owner(), E::WidthMismatch))?;
                if selected.inline_size() != expected {
                    return Err(error(p.owner(), E::WidthMismatch));
                }
                if let Some(start) =
                    frames.source_unit_start(paragraph_index as usize, selected.line().start_unit())
                {
                    let original = frames
                        .paragraphs()
                        .get(paragraph_index as usize)
                        .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
                    if start != original.start() {
                        return Err(error(p.owner(), E::WidthMismatch));
                    }
                }
            }
        }
        Ok(())
    }
}

#[path = "book_v2_table_source_profiles.rs"]
mod table_source_profiles;
