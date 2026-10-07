//! Source-ordered column filling with joint note backtracking. No stable paint
//! receipt is issued until width feedback, balancing and placement are joined.
use super::super::mixed_pages::MixedBoundary;
use super::*;
#[path = "book_v2_column_repeated_pages.rs"]
mod repeated;
pub use repeated::BookV2ColumnRepeatedPages;

pub struct BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a> {
    source: BookV2ColumnPageState<'b, 'f, 's, 'p, 'a>,
    empty_page: bool,
    name: Option<usize>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a> {
    pub fn source_state(&self) -> &BookV2ColumnPageState<'b, 'f, 's, 'p, 'a> {
        &self.source
    }
    pub fn page_index(&self) -> u32 {
        self.source.page_index()
    }
    pub fn is_complete(&self) -> bool {
        self.source.is_complete() && !self.empty_page
    }
}
pub struct BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a> {
    candidate: BookV2ColumnPageCandidate<'b, 'f, 's, 'p, 'a>,
    next: BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a>,
    forced: Option<NodeId>,
    attempts: u32,
    name: Option<&'p str>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a> {
    pub fn candidate(&self) -> &BookV2ColumnPageCandidate<'b, 'f, 's, 'p, 'a> {
        &self.candidate
    }
    pub fn next_state(&self) -> &BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a> {
        &self.next
    }
    pub fn page_index(&self) -> u32 {
        self.candidate.frames().page_index()
    }
    pub fn forced_break(&self) -> Option<NodeId> {
        self.forced
    }
    pub fn candidate_attempts(&self) -> u32 {
        self.attempts
    }
    pub fn named_page(&self) -> Option<&'p str> {
        self.name
    }
}
/// A source-contiguous physical sequence is still distinct from a single-body
/// stable placement receipt.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnPageSequence, BookV2BodyMixedPageSequence};
/// fn single<'b, 'f, 's, 'p, 'a>(v: BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>)
///     -> BookV2BodyMixedPageSequence<'b, 'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a> {
    owner: u64,
    plan: &'p BookV2ColumnFramePlan<'a>,
    measurements: [u8; 32],
    initial: BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a>,
    pages: Vec<BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a> {
    pub fn pages(&self) -> &[BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>] {
        &self.pages
    }
    pub fn measurements_fingerprint(&self) -> [u8; 32] {
        self.measurements
    }
}
struct ColumnAlternatives<'b, 'f, 's, 'p, 'a> {
    source: BookV2BodySourceState<'b, 'f, 's, 'p, 'a>,
    alternatives: std::vec::IntoIter<MixedBoundary<'b, 'f, 's, 'p, 'a>>,
    chosen: Vec<BookV2BodyCandidatePart<'b, 'f, 's, 'p, 'a>>,
    empty_allowed: bool,
    empty_tried: bool,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    pub fn begin_pages(
        &mut self,
    ) -> Result<BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        self.inner.verify_mixed_page_breaks()?;
        self.inner.content.charge(1, NodeId::new(0))?;
        Ok(BookV2ColumnPageSequenceState {
            source: self.begin()?,
            empty_page: true,
            name: None,
        })
    }
    fn fork_source(
        &mut self,
        source: &BookV2BodySourceState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<BookV2BodySourceState<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.inner.content.charge(1, NodeId::new(0))?;
        Ok(BookV2BodySourceState {
            demand: self.inner.fork(&source.demand, 0)?,
            item: source.item,
            next_table: source.next_table,
            table_end: source.table_end,
            continuation: source.continuation,
        })
    }
    fn alternatives(
        &mut self,
        source: BookV2BodySourceState<'b, 'f, 's, 'p, 'a>,
        bounds: Rect,
        note: Option<Rect>,
        empty_allowed: bool,
    ) -> Result<ColumnAlternatives<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        self.inner.active_page_frames = Some(ActivePageFrames {
            body: bounds,
            footnote: note,
        });
        let alternatives = self
            .inner
            .mixed_page_boundaries(&source)?
            .candidates
            .into_iter();
        Ok(ColumnAlternatives {
            source,
            alternatives,
            chosen: Vec::new(),
            empty_allowed,
            empty_tried: false,
        })
    }
    /// Fill left-to-right in the shared widow/orphan/heading cost order. All
    /// descendants of an earlier column cut precede the next cut in that column.
    /// An unused suffix is tried last, and each full-page fit is cumulative.
    pub fn select_page(
        &mut self,
        state: &BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a>,
    ) -> Result<Option<BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>>, ProductionBodyPaginationError>
    {
        self.inner.verify_state(&state.source.source.demand)?;
        if state.is_complete() {
            return Ok(None);
        }
        if state.page_index() >= self.inner.maximum_pages {
            return Err(error(NodeId::new(0), E::PageLimit));
        }
        let body_name = self.inner.current_source_page_name(
            state.source.source.item,
            state.source.source.next_table,
            state.source.source.continuation,
            state.source.source.continuation.map(|_| state.name),
        )?;
        let name = match body_name {
            Some(name) => name,
            None => self
                .inner
                .next_pending_page_name(&state.source.source.demand, state.name)?,
        };
        let saved_frames = self.inner.active_page_frames;
        let saved_name = self.inner.active_page_name;
        let saved_mismatch = self.inner.named_mismatch.take();
        let mut attempts = 0;
        let result = (|| {
            if let Some(page) = self.select_in_frames(state, name, &mut attempts)? {
                return Ok(Some(page));
            }
            if !state.source.source.demand.pending.is_empty() {
                let next = self
                    .inner
                    .next_pending_page_name(&state.source.source.demand, state.name)?;
                if next != name {
                    if let Some(page) = self.select_in_frames(state, next, &mut attempts)? {
                        return Ok(Some(page));
                    }
                }
            }
            if let Some(owner) = self.inner.named_mismatch {
                return Err(error(owner, E::PendingNamedPage));
            }
            Ok(None)
        })();
        self.inner.active_page_frames = saved_frames;
        self.inner.active_page_name = saved_name;
        self.inner.named_mismatch = saved_mismatch;
        result
    }
    fn select_in_frames(
        &mut self,
        state: &BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a>,
        name: Option<usize>,
        attempts: &mut u32,
    ) -> Result<Option<BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>>, ProductionBodyPaginationError>
    {
        let root = NodeId::new(0);
        let frames = self
            .plan
            .named_page(state.page_index(), name)
            .map_err(|_| error(root, E::PageLimit))?;
        self.inner.active_page_name = name;
        let outside = self.outside_source_break(&state.source.source, name)?;
        let body_complete = state.source.source.item == self.inner.content.flow.body_items().len()
            && state.source.source.next_table == state.source.source.table_end
            && state.source.source.continuation.is_none();
        let allow_blank = outside.is_some() || (state.empty_page && state.source.is_complete());
        let empty_allowed =
            body_complete || outside.is_some() || !state.source.source.demand.pending.is_empty();
        let source = self.fork_source(&state.source.source)?;
        let first = self.alternatives(
            source,
            frames
                .column(0)
                .map_err(|_| error(root, E::ReceiptMismatch))?,
            frames.footnote(),
            empty_allowed,
        )?;
        self.inner.content.charge(1, root)?;
        let mut stack = Vec::new();
        stack
            .try_reserve(1)
            .map_err(|_| error(root, E::AllocationFailure))?;
        stack.push(first);
        // An explicit stack keeps the authored u16 column count off the call stack.
        while !stack.is_empty() {
            self.inner.content.step(root)?;
            let column = stack.len() - 1;
            let frame = stack.last_mut().unwrap();
            let boundary = frame.alternatives.next();
            let expected = if let Some(boundary) = boundary {
                frame.chosen = boundary.requests;
                Some(boundary.key)
            } else if frame.empty_allowed && !frame.empty_tried {
                frame.empty_tried = true;
                frame.chosen.clear();
                None
            } else {
                stack.pop();
                continue;
            };
            self.inner.active_page_frames = Some(ActivePageFrames {
                body: frames
                    .column(column as u16)
                    .map_err(|_| error(root, E::ReceiptMismatch))?,
                footnote: frames.footnote(),
            });
            if let Some(expected) = expected {
                let frame = stack.last().unwrap();
                self.inner
                    .verify_source_requests(&frame.source, &frame.chosen)?;
                let Some(projection) = mixed_kernel::evaluate_source(
                    &mut self.inner,
                    &frame.source.demand,
                    frame.source.item,
                    Some(frame.source.next_table),
                    frame.source.continuation,
                    &frame.chosen,
                )?
                else {
                    continue;
                };
                let forced = projection
                    .parts
                    .last()
                    .and_then(|p| p.table())
                    .is_some_and(|t| t.forced_break_owner().is_some());
                let key = self.inner.mixed_boundary_key(
                    projection.end,
                    projection.next_table.unwrap(),
                    projection.continuation,
                    projection.parts.last().and_then(|p| p.items()),
                    projection.height,
                    forced,
                )?;
                if key != expected {
                    return Err(error(root, E::ReceiptMismatch));
                }
                let source = BookV2BodySourceState {
                    demand: projection.demanded,
                    item: projection.end,
                    next_table: projection.next_table.unwrap(),
                    table_end: frame.source.table_end,
                    continuation: projection.continuation,
                };
                let closed = forced
                    || self.outside_source_break(&source, name)?.is_some()
                    || self
                        .inner
                        .current_source_page_name(
                            source.item,
                            source.next_table,
                            source.continuation,
                            source.continuation.map(|_| name),
                        )?
                        .is_none_or(|next| next != name);
                if !closed && column + 1 < usize::from(frames.column_count()) {
                    let next = self.alternatives(
                        source,
                        frames
                            .column((column + 1) as u16)
                            .map_err(|_| error(root, E::ReceiptMismatch))?,
                        frames.footnote(),
                        true,
                    )?;
                    self.inner.content.charge(1, root)?;
                    stack
                        .try_reserve(1)
                        .map_err(|_| error(root, E::AllocationFailure))?;
                    stack.push(next);
                    continue;
                }
            }
            *attempts = attempts
                .checked_add(1)
                .ok_or_else(|| error(root, E::FootnoteSearchLimit))?;
            if *attempts > u32::from(self.inner.maximum_reflows) {
                return Err(error(root, E::FootnoteSearchLimit));
            }
            self.inner
                .content
                .charge(usize::from(frames.column_count()), root)?;
            let mut requests = Vec::new();
            requests
                .try_reserve_exact(usize::from(frames.column_count()))
                .map_err(|_| error(root, E::AllocationFailure))?;
            for column in 0..usize::from(frames.column_count()) {
                self.inner.content.step(root)?;
                requests.push(
                    stack
                        .get(column)
                        .map_or(&[][..], |frame| frame.chosen.as_slice()),
                );
            }
            let candidate =
                match self.evaluate_page_with_blank(&state.source, name, &requests, allow_blank) {
                    Ok(Some(candidate)) => candidate,
                    Ok(None) => continue,
                    Err(e) if e.kind == E::PendingNamedPage => {
                        self.inner.named_mismatch.get_or_insert(e.owner);
                        continue;
                    }
                    Err(e) => return Err(e),
                };
            return self.finish_selection(candidate, *attempts, name).map(Some);
        }
        Ok(None)
    }
    fn finish_selection(
        &mut self,
        candidate: BookV2ColumnPageCandidate<'b, 'f, 's, 'p, 'a>,
        attempts: u32,
        name: Option<usize>,
    ) -> Result<BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let outside = self.outside_source_break(&candidate.next.source, name)?;
        let forced = candidate
            .columns()
            .iter()
            .rev()
            .find_map(|column| {
                column
                    .parts()
                    .last()
                    .and_then(|p| p.table())
                    .and_then(|table| table.forced_break_owner())
            })
            .or(outside);
        let note_forced = candidate
            .footnotes()
            .is_some_and(|notes| notes.forced_break_owner().is_some());
        let mut source = self.fork_source(&candidate.next.source)?;
        source.item = source
            .item
            .checked_add(usize::from(outside.is_some()))
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        self.inner.content.charge(1, root)?;
        let next = BookV2ColumnPageSequenceState {
            source: BookV2ColumnPageState {
                source,
                page: candidate.next.page,
            },
            empty_page: forced.is_some() || note_forced,
            name,
        };
        Ok(BookV2ColumnPageSelection {
            candidate,
            next,
            forced,
            attempts,
            name: name.and_then(|n| self.plan.name(n)),
        })
    }
    pub fn select_pages(
        &mut self,
    ) -> Result<BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let initial = self.begin_pages()?;
        let mut pages: Vec<BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>> = Vec::new();
        loop {
            let state = pages.last().map_or(&initial, |page| page.next_state());
            if state.is_complete() {
                break;
            }
            let selected = self
                .select_page(state)?
                .ok_or_else(|| error(root, E::JointPageNoFit))?;
            let before = &state.source.source;
            let after = &selected.next.source.source;
            let table_progress = after.next_table > before.next_table
                || match (before.continuation, after.continuation) {
                    (None, Some(c)) => !c.is_initial(),
                    (Some(a), Some(b)) => {
                        a.offset() != b.offset()
                            || a.next_row() != b.next_row()
                            || a.next_caption_item() != b.next_caption_item()
                            || a.cell_progress_fingerprint() != b.cell_progress_fingerprint()
                            || (!a.has_started_rows() && b.has_started_rows())
                    }
                    (Some(_), None) => true,
                    (None, None) => false,
                };
            let notes = selected
                .candidate
                .footnotes()
                .is_some_and(|n| !n.fragments().is_empty());
            if after.item < before.item
                || after.next_table < before.next_table
                || (after.item == before.item
                    && !table_progress
                    && !notes
                    && !(state.empty_page && selected.next.is_complete()))
            {
                return Err(error(root, E::ReceiptMismatch));
            }
            self.inner.content.charge(1, root)?;
            pages
                .try_reserve(1)
                .map_err(|_| error(root, E::AllocationFailure))?;
            pages.push(selected);
        }
        Ok(BookV2ColumnPageSequence {
            owner: self.inner.owner_id,
            plan: self.plan,
            measurements: self
                .inner
                .tables
                .as_ref()
                .unwrap()
                .measurements_fingerprint(),
            initial,
            pages,
        })
    }
    pub fn verify_sequence(
        &self,
        sequence: &BookV2ColumnPageSequence<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        if sequence.owner != self.inner.owner_id
            || !std::ptr::eq(sequence.plan, self.plan)
            || sequence.measurements
                != self
                    .inner
                    .tables
                    .as_ref()
                    .unwrap()
                    .measurements_fingerprint()
            || sequence
                .pages
                .last()
                .is_none_or(|page| !page.next.is_complete())
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        for (index, page) in sequence.pages.iter().enumerate() {
            let entry = if index == 0 {
                &sequence.initial
            } else {
                sequence.pages[index - 1].next_state()
            };
            if page.page_index() != index as u32 || page.next.page_index() != page.page_index() + 1
            {
                return Err(error(root, E::ReceiptMismatch));
            }
            page.candidate.verify(&entry.source)?;
        }
        Ok(())
    }
}
