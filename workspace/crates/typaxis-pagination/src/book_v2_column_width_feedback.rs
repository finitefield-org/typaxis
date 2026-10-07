//! Original-source width feedback from this search's selected column sequence.
//! A feedback profile can drive fresh shaping, but cannot authorize page paint.
use super::*;
use typaxis_syntax::book_v2::BookV2ColumnFramePlan;

/// Bound to the original column plan, independently of single-body feedback.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnWidthFeedback, BookV2ParagraphWidthFeedback};
/// fn single(v: BookV2ColumnWidthFeedback<'_, '_>) -> BookV2ParagraphWidthFeedback { v }
/// ```
pub struct BookV2ColumnWidthFeedback<'p, 'a> {
    plan: &'p BookV2ColumnFramePlan<'a>,
    owner: u64,
    measurements: [u8; 32],
    source: [u8; 32],
    fingerprint: [u8; 32],
    paragraphs: Vec<BookV2ParagraphWidthCandidate>,
    blocks: Vec<(NodeId, PositiveLength)>,
    block_starts: Vec<Length>,
    lines_match: bool,
    blocks_match: bool,
    records: u64,
    work: u64,
}
impl<'p, 'a> BookV2ColumnWidthFeedback<'p, 'a> {
    pub fn column_plan(&self) -> &'p BookV2ColumnFramePlan<'a> {
        self.plan
    }
    pub fn paragraphs(&self) -> &[BookV2ParagraphWidthCandidate] {
        &self.paragraphs
    }
    pub fn block_widths(&self) -> &[(NodeId, PositiveLength)] {
        &self.blocks
    }
    pub fn block_starts(&self) -> &[Length] {
        &self.block_starts
    }
    pub fn assignment_fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn matches_source_flow(
        &self,
        flow: &typaxis_syntax::book_v2::PreparedBookV2TextFlow<'_>,
    ) -> bool {
        self.source == flow.fingerprint()
    }
    pub fn matches_selected_line_widths(&self) -> bool {
        self.lines_match
    }
    pub fn matches_selected_block_widths(&self) -> bool {
        self.blocks_match
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    /// Observe all original body units once, then each demanded definition once.
    /// Repeated headers use their real graph without changing semantic widths.
    pub fn paragraph_frame_feedback(
        &mut self,
        sequence: &BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2ColumnWidthFeedback<'p, 'a>, ProductionBodyPaginationError> {
        self.verify_sequence(sequence)?;
        let root = NodeId::new(0);
        let search = &mut self.inner;
        let lines = search.content.flow.lines();
        let count = lines.paragraphs().len();
        search.content.charge.take(
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
        if count != lines.prepared().paragraphs().len() {
            return Err(error(root, E::ReceiptMismatch));
        }
        for (paragraph, prepared) in lines.paragraphs().iter().zip(lines.prepared().paragraphs()) {
            let owner = paragraph.owner();
            search.content.step(owner)?;
            let units = prepared
                .items()
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                .units()
                .len()
                .max(1);
            search.content.charge.take(
                units
                    .checked_mul(2)
                    .ok_or_else(|| error(owner, E::FragmentLimit))?,
                owner,
            )?;
            let mut widths = Vec::new();
            let mut seen = Vec::new();
            widths
                .try_reserve_exact(units)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            seen.try_reserve_exact(units)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            for _ in 0..units {
                search.content.step(owner)?;
                widths.push(paragraph.inline_size());
                seen.push(0);
            }
            paragraphs.push(BookV2ParagraphWidthCandidate {
                owner,
                widths,
                retained_ends: None,
                starts: None,
                table_frame: false,
            });
            visits.push(seen);
        }
        let (mut blocks, mut block_starts, mut block_visits) =
            search.begin_column_block_profiles()?;
        let mut lines_match = true;
        let mut blocks_match = true;
        for page in sequence.pages() {
            search.content.step(root)?;
            for column in page.candidate().columns() {
                search.content.step(root)?;
                for part in column.parts() {
                    search.content.step(root)?;
                    if let Some(range) = part.items() {
                        let items = search
                            .content
                            .flow
                            .body_items()
                            .get(range)
                            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                        let matches = search.observe_column_items(
                            items,
                            None,
                            column.bounds(),
                            &mut paragraphs,
                            &mut visits,
                            &mut blocks,
                            &mut block_starts,
                            &mut block_visits,
                        )?;
                        lines_match &= matches.0;
                        blocks_match &= matches.1;
                    }
                }
            }
            if let Some(notes) = page.candidate().footnotes() {
                let bounds = page
                    .candidate()
                    .frames()
                    .footnote()
                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                for selected in notes.fragments() {
                    search.content.step(root)?;
                    let fragment = selected.fragment();
                    let definition = Some(fragment.definition_index());
                    if let Some(mixed) = fragment.mixed() {
                        for part in mixed.parts() {
                            search.content.step(root)?;
                            if let Some(range) = part.items() {
                                let items = search
                                    .content
                                    .flow
                                    .definition_items(fragment.definition_index())
                                    .and_then(|items| items.get(range))
                                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                                let matches = search.observe_column_items(
                                    items,
                                    definition,
                                    bounds,
                                    &mut paragraphs,
                                    &mut visits,
                                    &mut blocks,
                                    &mut block_starts,
                                    &mut block_visits,
                                )?;
                                lines_match &= matches.0;
                                blocks_match &= matches.1;
                            }
                        }
                    } else {
                        let matches = search.observe_column_items(
                            fragment.items()?,
                            definition,
                            bounds,
                            &mut paragraphs,
                            &mut visits,
                            &mut blocks,
                            &mut block_starts,
                            &mut block_visits,
                        )?;
                        lines_match &= matches.0;
                        blocks_match &= matches.1;
                    }
                }
            }
        }
        let report = search.collect_column_table_width_frames(sequence)?;
        let matches = search.collect_table_profiles_from_report(
            &report,
            &mut paragraphs,
            &mut visits,
            &mut blocks,
            Some(&mut block_starts),
        )?;
        lines_match &= matches.0;
        blocks_match &= matches.1;
        let source = lines.prepared().source_flow().fingerprint();
        let mut fingerprint = source;
        search.fold_column_width(&mut fingerprint, 0x434f4c57494431, root)?;
        for (paragraph, seen) in paragraphs.iter().zip(visits) {
            let owner = paragraph.owner();
            search.fold_column_width(&mut fingerprint, u64::from(owner.get()), owner)?;
            search.fold_column_width(&mut fingerprint, paragraph.widths.len() as u64, owner)?;
            let mut any = false;
            let mut complete = true;
            for (width, seen) in paragraph.widths.iter().zip(seen) {
                search.fold_column_width(&mut fingerprint, width.get().raw() as u64, owner)?;
                any |= seen != 0;
                complete &= seen & 1 != 0;
            }
            if any && !complete {
                return Err(error(owner, E::ReceiptMismatch));
            }
            search.fold_column_width(
                &mut fingerprint,
                u64::from(paragraph.starts.is_some()),
                owner,
            )?;
            if let Some(starts) = &paragraph.starts {
                for start in starts {
                    search.fold_column_width(&mut fingerprint, start.raw() as u64, owner)?;
                }
            }
        }
        for ((owner, width), start) in blocks.iter().zip(&block_starts) {
            search.fold_column_width(&mut fingerprint, u64::from(owner.get()), *owner)?;
            search.fold_column_width(&mut fingerprint, width.get().raw() as u64, *owner)?;
            search.fold_column_width(&mut fingerprint, start.raw() as u64, *owner)?;
        }
        Ok(BookV2ColumnWidthFeedback {
            plan: self.plan,
            owner: search.owner_id,
            measurements: sequence.measurements_fingerprint(),
            source,
            fingerprint,
            paragraphs,
            blocks,
            block_starts,
            lines_match,
            blocks_match,
            records: search.record_charge(),
            work: search.work_steps(),
        })
    }

    /// Move the already reserved source-unit arrays out of a short line-graph
    /// callback. The caller must supply the same original flow and plan, not an
    /// equivalent reconstruction. No old graph or uncharged array copy escapes.
    pub fn capture_source_width_feedback<'origin, 'source>(
        &mut self,
        feedback: BookV2ColumnWidthFeedback<'p, 'a>,
        source: &typaxis_syntax::book_v2::PreparedBookV2TextFlow<'source>,
        columns: &'origin BookV2ColumnFramePlan<'source>,
    ) -> Result<BookV2ColumnWidthFeedback<'origin, 'source>, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        self.inner.content.step(root)?;
        if feedback.owner != self.inner.owner_id
            || !std::ptr::eq(feedback.plan, self.plan)
            || !std::ptr::eq(columns, self.plan)
            || !std::ptr::eq(source, self.inner.content.flow.lines().prepared().source_flow())
            || !feedback.matches_source_flow(source)
            || feedback.measurements
                != self.inner.tables.as_ref().unwrap().measurements_fingerprint()
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        Ok(BookV2ColumnWidthFeedback {
            plan: columns,
            owner: feedback.owner,
            measurements: feedback.measurements,
            source: feedback.source,
            fingerprint: feedback.fingerprint,
            paragraphs: feedback.paragraphs,
            blocks: feedback.blocks,
            block_starts: feedback.block_starts,
            lines_match: feedback.lines_match,
            blocks_match: feedback.blocks_match,
            records: self.record_charge(),
            work: self.work_steps(),
        })
    }

    /// On a width cycle, retain the original selected unit boundaries before
    /// another shaping pass. Never substitute current line ordinals for units.
    pub fn retain_paragraph_line_boundaries(
        &mut self,
        sequence: &BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>,
        feedback: &mut BookV2ColumnWidthFeedback<'p, 'a>,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.verify_sequence(sequence)?;
        let root = NodeId::new(0);
        let search = &mut self.inner;
        let lines = search.content.flow.lines();
        if feedback.owner != search.owner_id
            || !std::ptr::eq(feedback.plan, self.plan)
            || feedback.measurements != sequence.measurements_fingerprint()
            || !feedback.matches_source_flow(lines.prepared().source_flow())
            || feedback.paragraphs.len() != lines.paragraphs().len()
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        for (paragraph, candidate) in lines.paragraphs().iter().zip(&mut feedback.paragraphs) {
            let owner = paragraph.owner();
            search.content.step(owner)?;
            if candidate.owner != owner || candidate.retained_ends.is_some() {
                return Err(error(owner, E::ReceiptMismatch));
            }
            let selected = paragraph
                .selected()
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            search.content.charge.take(selected.lines().len(), owner)?;
            let mut ends = Vec::new();
            ends.try_reserve_exact(selected.lines().len())
                .map_err(|_| error(owner, E::AllocationFailure))?;
            search.fold_column_width(&mut feedback.fingerprint, 0x434f4c454e4431, owner)?;
            for line in selected.lines() {
                let end = line.line().end_unit();
                search.fold_column_width(&mut feedback.fingerprint, u64::from(end), owner)?;
                ends.push(end);
            }
            candidate.retained_ends = Some(ends);
        }
        feedback.records = search.record_charge();
        feedback.work = search.work_steps();
        Ok(())
    }
}

impl<'b, 'f, 's, 'p, 'a> BookV2FootnoteDemandSearch<'b, 'f, 's, 'p, 'a> {
    fn fold_column_width(
        &mut self,
        hash: &mut [u8; 32],
        value: u64,
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.content.step(owner)?;
        let mut bytes = [0u8; 40];
        bytes[..32].copy_from_slice(hash);
        bytes[32..].copy_from_slice(&value.to_be_bytes());
        *hash = typaxis_core::sha256(&bytes);
        Ok(())
    }
    fn begin_column_block_profiles(
        &mut self,
    ) -> Result<(Vec<(NodeId, PositiveLength)>, Vec<Length>, Vec<u8>), ProductionBodyPaginationError>
    {
        use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};
        let root = NodeId::new(0);
        let lines = self.content.flow.lines();
        let prepared = lines.prepared();
        let count = prepared
            .figures()
            .len()
            .checked_add(
                prepared
                    .native_math()
                    .map_or(0, |math| math.display_blocks().len()),
            )
            .and_then(|n| {
                n.checked_add(
                    self.content
                        .flow
                        .blocks()
                        .map_or(0, |blocks| blocks.blocks().len()),
                )
            })
            .ok_or_else(|| error(root, E::FragmentLimit))?;
        self.content.charge.take(
            count
                .checked_mul(3)
                .and_then(|n| n.checked_add(3))
                .ok_or_else(|| error(root, E::FragmentLimit))?,
            root,
        )?;
        let mut widths = Vec::new();
        let mut starts = Vec::new();
        let mut visits = Vec::new();
        widths
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        starts
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        visits
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        let frames = lines
            .frames()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        for event in prepared.source_flow().events() {
            self.content.step(root)?;
            let Event::Begin {
                owner,
                kind:
                    Region::Figure
                    | Region::VectorFigure
                    | Region::DisplayMath
                    | Region::MathVectorBlock,
            } = *event
            else {
                continue;
            };
            if widths.len() >= count {
                return Err(error(owner, E::ReceiptMismatch));
            }
            for _ in 0..frames.measurement_region_lookup_work() {
                self.content.step(owner)?;
            }
            let frame = frames
                .measurement_region(owner)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            widths.push((owner, frame.width()));
            starts.push(frame.start());
            visits.push(0);
        }
        if widths.len() != count {
            return Err(error(root, E::ReceiptMismatch));
        }
        Ok((widths, starts, visits))
    }
    #[allow(clippy::too_many_arguments)]
    fn observe_column_items(
        &mut self,
        items: &[ProductionBodyFlowItem],
        definition: Option<usize>,
        actual: Rect,
        paragraphs: &mut [BookV2ParagraphWidthCandidate],
        visits: &mut [Vec<u8>],
        blocks: &mut [(NodeId, PositiveLength)],
        starts: &mut [Length],
        block_visits: &mut [u8],
    ) -> Result<(bool, bool), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let lines = self.content.flow.lines();
        let frames = lines
            .frames()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let measured = if definition.is_some() {
            frames
                .footnote_region()
                .ok_or_else(|| error(root, E::ReceiptMismatch))?
        } else {
            frames.body()
        };
        let delta = actual
            .width()
            .get()
            .checked_sub(measured.width().get())
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        let mut lines_match = true;
        let mut blocks_match = true;
        for item in items {
            let owner = item.owner();
            self.content.step(owner)?;
            let Some(source) = item.source() else {
                continue;
            };
            if let ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } = source
            {
                let index = paragraph_index as usize;
                let paragraph = lines
                    .paragraphs()
                    .get(index)
                    .filter(|p| p.owner() == owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let selected = paragraph
                    .selected()
                    .and_then(|p| p.lines().get(line_index as usize))
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let width = paragraph
                    .inline_size()
                    .get()
                    .checked_add(delta)
                    .and_then(PositiveLength::new)
                    .ok_or_else(|| error(owner, E::WidthMismatch))?;
                lines_match &= selected.inline_size() == width;
                let start = selected.line().start_unit() as usize;
                let mut end = selected.line().end_unit() as usize;
                if start == end {
                    if start != 0
                        || !lines.prepared().paragraphs()[index]
                            .items()
                            .ok_or_else(|| error(owner, E::ReceiptMismatch))?
                            .units()
                            .is_empty()
                    {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    end = 1;
                }
                let candidate = paragraphs
                    .get_mut(index)
                    .filter(|p| p.owner == owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let widths = candidate
                    .widths
                    .get_mut(start..end)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let seen = visits
                    .get_mut(index)
                    .and_then(|v| v.get_mut(start..end))
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                for (width_slot, flag) in widths.iter_mut().zip(seen) {
                    self.content.step(owner)?;
                    if *flag != 0 {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    *width_slot = width;
                    *flag = 1;
                }
            } else {
                for _ in 0..frames.measurement_region_lookup_work() {
                    self.content.step(owner)?;
                }
                let original = frames
                    .measurement_region(owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let width = original
                    .width()
                    .get()
                    .checked_add(delta)
                    .and_then(PositiveLength::new)
                    .ok_or_else(|| error(owner, E::WidthMismatch))?;
                for _ in 0..blocks.len() {
                    self.content.step(owner)?;
                }
                let index = blocks
                    .iter()
                    .position(|entry| entry.0 == owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                if block_visits[index] != 0 {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                block_visits[index] = 1;
                blocks[index].1 = width;
                starts[index] = original.start();
                blocks_match &= frames.region(owner).map(|f| (f.start(), f.width()))
                    == Some((original.start(), width));
            }
        }
        Ok((lines_match, blocks_match))
    }
}
