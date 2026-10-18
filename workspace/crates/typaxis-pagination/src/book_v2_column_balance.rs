//! Source-bound final-page balancing. A capacity change is admissible only
//! after an exact endpoint trial or a proved positive original-source blocker.
use super::*;
use mixed_kernel::source_kernel::{next_ordinary_source_group, NextOrdinarySourceGroup};
#[path = "book_v2_column_balance_groups.rs"]
mod groups;
#[path = "book_v2_column_balance_trace.rs"]
mod trace;

pub const BOOK_V2_COLUMN_BALANCE_ALGORITHM: &str = "typaxis.book-2-column-balance/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookV2ColumnBalanceRejection {
    column: u16,
    owner: NodeId,
    start: usize,
    end: usize,
    required: PositiveLength,
    deficit: PositiveLength,
    table_group: Option<[u8; 32]>,
    table_proofs: Vec<Vec<u8>>,
}
impl BookV2ColumnBalanceRejection {
    pub fn column_index(&self) -> u16 {
        self.column
    }
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    /// Original ordinary group, or the original table/keep source envelope.
    /// Table certificates carry its exact cell, caption and nested positions.
    pub fn source_items(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
    pub fn required_height(&self) -> PositiveLength {
        self.required
    }
    pub fn deficit(&self) -> PositiveLength {
        self.deficit
    }
    pub fn table_group_fingerprint(&self) -> Option<[u8; 32]> {
        self.table_group
    }
    pub fn table_group_certificates(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        self.table_proofs.iter().map(Vec::as_slice)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookV2ColumnBalanceCandidate {
    target: PositiveLength,
    terminal: bool,
    rejections: Vec<BookV2ColumnBalanceRejection>,
    rejection_fingerprint: [u8; 32],
    capacity_canonical: Vec<u8>,
    capacity_fingerprint: [u8; 32],
}
impl BookV2ColumnBalanceCandidate {
    pub fn target(&self) -> PositiveLength {
        self.target
    }
    pub fn target_height(&self) -> PositiveLength {
        self.target
    }
    pub fn terminal(&self) -> bool {
        self.terminal
    }
    pub fn rejections(&self) -> &[BookV2ColumnBalanceRejection] {
        &self.rejections
    }
    pub fn rejection_fingerprint(&self) -> [u8; 32] {
        self.rejection_fingerprint
    }
    pub fn capacity_canonical_bytes(&self) -> &[u8] {
        &self.capacity_canonical
    }
    pub fn capacity_fingerprint(&self) -> [u8; 32] {
        self.capacity_fingerprint
    }
}

/// This receipt belongs to the issuing column source selection. It cannot
/// provide a single-body placement or PDF authorization.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnBalanceReceipt, BookV2BodyMixedPageSelection};
/// fn single<'b, 'f, 's, 'p, 'a>(v: BookV2ColumnBalanceReceipt)
///     -> BookV2BodyMixedPageSelection<'b, 'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnBalanceReceipt {
    owner: u64,
    limits: [u8; 32],
    measurements: [u8; 32],
    page: u32,
    extent: Length,
    full: PositiveLength,
    first: PositiveLength,
    selected: PositiveLength,
    input: [u8; 32],
    input_canonical: Vec<u8>,
    candidates: Vec<BookV2ColumnBalanceCandidate>,
    canonical: Vec<u8>,
    fingerprint: [u8; 32],
}
impl BookV2ColumnBalanceReceipt {
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn initial_extent(&self) -> Length {
        self.extent
    }
    pub fn full_height(&self) -> PositiveLength {
        self.full
    }
    pub fn initial_height(&self) -> PositiveLength {
        self.first
    }
    pub fn selected_height(&self) -> PositiveLength {
        self.selected
    }
    pub fn candidates(&self) -> &[BookV2ColumnBalanceCandidate] {
        &self.candidates
    }
    pub fn input_fingerprint(&self) -> [u8; 32] {
        self.input
    }
    pub fn input_canonical_bytes(&self) -> &[u8] {
        &self.input_canonical
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

trait Bytes {
    fn write(&mut self, value: &[u8]) -> Result<(), ProductionBodyPaginationError>;
}
#[derive(Default)]
struct Count(usize);
impl Bytes for Count {
    fn write(&mut self, value: &[u8]) -> Result<(), ProductionBodyPaginationError> {
        self.0 = self
            .0
            .checked_add(value.len())
            .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow))?;
        Ok(())
    }
}
impl Bytes for Vec<u8> {
    fn write(&mut self, value: &[u8]) -> Result<(), ProductionBodyPaginationError> {
        self.extend_from_slice(value);
        Ok(())
    }
}
fn number(out: &mut dyn Bytes, value: usize) -> Result<(), ProductionBodyPaginationError> {
    out.write(&(value as u64).to_be_bytes())
}
fn rect(out: &mut dyn Bytes, value: Option<Rect>) -> Result<(), ProductionBodyPaginationError> {
    out.write(&[u8::from(value.is_some())])?;
    for raw in value.map_or([0; 4], |r| {
        [
            r.x().raw(),
            r.y().raw(),
            r.width().get().raw(),
            r.height().get().raw(),
        ]
    }) {
        out.write(&raw.to_be_bytes())?;
    }
    Ok(())
}
fn table_cursor(
    out: &mut dyn Bytes,
    cursor: Option<BookV2TableCursor<'_, '_, '_, '_, '_>>,
) -> Result<(), ProductionBodyPaginationError> {
    out.write(&[u8::from(cursor.is_some())])?;
    for value in [
        cursor.map_or(0, |c| c.table_index()),
        cursor.map_or(0, |c| c.next_row()),
        cursor.map_or(0, |c| c.next_caption_item()),
    ] {
        number(out, value)?;
    }
    out.write(&cursor.map_or(0, |c| c.offset().raw()).to_be_bytes())?;
    out.write(&[
        u8::from(cursor.is_some_and(|c| c.is_initial())),
        u8::from(cursor.is_some_and(|c| c.has_started_rows())),
    ])?;
    out.write(&cursor.map_or([0; 32], |c| c.cell_progress_fingerprint()))
}

impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    fn balance_blob(
        &mut self,
        encode: impl Fn(&mut Self, &mut dyn Bytes) -> Result<(), ProductionBodyPaginationError>,
    ) -> Result<Vec<u8>, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let mut count = Count::default();
        encode(self, &mut count)?;
        let next = self
            .inner
            .terminal_spool
            .checked_add(count.0 as u64)
            .filter(|n| *n <= self.maximum_balance_spool)
            .ok_or_else(|| error(root, E::SpoolLimit))?;
        self.inner.terminal_spool = next;
        self.inner.content.charge(1, root)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(count.0)
            .map_err(|_| error(root, E::AllocationFailure))?;
        encode(self, &mut bytes)?;
        if bytes.len() != count.0 {
            return Err(error(root, E::ColumnBalanceContradiction));
        }
        Ok(bytes)
    }
    fn balance_hash(&mut self, bytes: &[u8]) -> Result<[u8; 32], ProductionBodyPaginationError> {
        for _ in 0..bytes.len().div_ceil(64) {
            self.inner.content.step(NodeId::new(0))?;
        }
        Ok(sha256(bytes))
    }
    fn encode_balance_state(
        &mut self,
        state: &BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a>,
        out: &mut dyn Bytes,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let source = &state.source.source;
        self.inner.verify_state(&source.demand)?;
        self.inner.content.step(root)?;
        out.write(&state.page_index().to_be_bytes())?;
        out.write(&[u8::from(state.empty_page), u8::from(state.name.is_some())])?;
        number(out, state.name.unwrap_or(0))?;
        number(out, source.item)?;
        number(out, source.next_table)?;
        number(out, source.table_end)?;
        table_cursor(out, source.continuation)?;
        number(out, source.demand.definitions.len())?;
        for definition in &source.demand.definitions {
            self.inner.content.step(root)?;
            match definition {
                DemandValue::Unreferenced => out.write(&[0])?,
                DemandValue::Complete { first_reference } => {
                    out.write(&[2])?;
                    out.write(&first_reference.get().to_be_bytes())?;
                }
                DemandValue::Pending {
                    first_reference,
                    cursor,
                } => {
                    out.write(&[1])?;
                    out.write(&first_reference.get().to_be_bytes())?;
                    number(out, cursor.definition_index())?;
                    number(out, cursor.next_item())?;
                    out.write(&[
                        u8::from(cursor.next_table_index().is_some()),
                        u8::from(cursor.definition_started()),
                    ])?;
                    number(out, cursor.next_table_index().unwrap_or(0))?;
                    table_cursor(out, cursor.table_continuation())?;
                }
            }
        }
        number(out, source.demand.pending.len())?;
        for &definition in &source.demand.pending {
            self.inner.content.step(root)?;
            number(out, definition)?;
        }
        Ok(())
    }
    fn balance_input(
        &mut self,
        sequence: &BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>,
        index: usize,
        extent: Length,
    ) -> Result<([u8; 32], Vec<u8>), ProductionBodyPaginationError> {
        let page = &sequence.pages[index];
        let flow = self.inner.content.flow;
        let bytes = self.balance_blob(|this, out| {
            out.write(&sha256(BOOK_V2_COLUMN_BALANCE_ALGORITHM.as_bytes()))?;
            out.write(&this.balance_limits)?;
            out.write(&this.plan.source().body().canonical_jcs_sha256())?;
            out.write(&sequence.measurements_fingerprint())?;
            out.write(&flow.lines().fingerprint())?;
            out.write(&flow.blocks().map_or([0; 32], |b| b.fingerprint()))?;
            rect(out, Some(page.candidate().frames().body()))?;
            rect(out, page.candidate().frames().footnote())?;
            number(out, usize::from(page.candidate().frames().column_count()))?;
            out.write(&extent.raw().to_be_bytes())?;
            this.encode_balance_state(sequence.entry_state(index).unwrap(), out)?;
            this.encode_balance_state(page.next_state(), out)?;
            for column in page.candidate().columns() {
                this.inner.content.step(NodeId::new(0))?;
                rect(out, Some(column.bounds()))?;
                out.write(&column.used_height().raw().to_be_bytes())?;
                number(out, column.parts().len())?;
                for part in column.parts() {
                    this.encode_balance_part(part, out)?;
                }
            }
            Ok(())
        })?;
        Ok((self.balance_hash(&bytes)?, bytes))
    }
    fn rejection_hash(
        &mut self,
        rejections: &[BookV2ColumnBalanceRejection],
    ) -> Result<[u8; 32], ProductionBodyPaginationError> {
        let bytes = self.balance_blob(|this, out| {
            number(out, rejections.len())?;
            for r in rejections {
                this.inner.content.step(r.owner)?;
                out.write(&r.column.to_be_bytes())?;
                out.write(&r.owner.get().to_be_bytes())?;
                number(out, r.start)?;
                number(out, r.end)?;
                out.write(&r.required.get().raw().to_be_bytes())?;
                out.write(&r.deficit.get().raw().to_be_bytes())?;
                out.write(&[u8::from(r.table_group.is_some())])?;
                if let Some(group) = r.table_group {
                    out.write(&group)?;
                }
            }
            Ok(())
        })?;
        self.balance_hash(&bytes)
    }
    /// Greedy capacity blockers use hard source legality, independently of
    /// widow/orphan ranking. The endpoint trial has already exhausted ranking.
    /// A table or keep entering one requires a separately proved next event.
    fn ordinary_balance_capacity(
        &mut self,
        entry: &BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a>,
        initial: &BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>,
        target: PositiveLength,
    ) -> Result<(bool, Vec<BookV2ColumnBalanceRejection>), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let before = &entry.source.source;
        let after = &initial.candidate().next.source;
        if before.next_table != after.next_table
            || before.continuation.is_some()
            || after.continuation.is_some()
        {
            return Err(error(
                root,
                E::PendingRegion("column_balance_table_threshold"),
            ));
        }
        let flow = self.inner.content.flow;
        let next_table = self
            .inner
            .tables
            .as_ref()
            .unwrap()
            .range(before.next_table)
            .map(|r| r.start);
        let mut item = before.item;
        let mut rejections = Vec::new();
        self.inner.content.charge(1, root)?;
        for column in 0..initial.candidate().frames().column_count() {
            self.inner.content.step(root)?;
            let mut used = Length::ZERO;
            let mut preceding = None;
            while item < after.item {
                let group = match next_ordinary_source_group(
                    flow.body_items(),
                    item,
                    next_table,
                    |owner| self.inner.content.step(owner),
                )? {
                    NextOrdinarySourceGroup::Complete(group) if group.end <= after.item => group,
                    NextOrdinarySourceGroup::Table { .. } => {
                        return Err(error(
                            root,
                            E::PendingRegion("column_balance_table_threshold"),
                        ))
                    }
                    _ => return Err(error(root, E::ColumnBalanceContradiction)),
                };
                let required = group.required_after(used, preceding)?;
                if required > target.get() {
                    let deficit = required
                        .checked_sub(target.get())
                        .and_then(PositiveLength::new)
                        .ok_or_else(|| error(group.owner, E::ColumnBalanceContradiction))?;
                    let required = PositiveLength::new(required)
                        .ok_or_else(|| error(group.owner, E::ColumnBalanceContradiction))?;
                    self.inner.content.charge(1, group.owner)?;
                    rejections
                        .try_reserve(1)
                        .map_err(|_| error(group.owner, E::AllocationFailure))?;
                    rejections.push(BookV2ColumnBalanceRejection {
                        column,
                        owner: group.owner,
                        start: group.start,
                        end: group.end,
                        required,
                        deficit,
                        table_group: None,
                        table_proofs: Vec::new(),
                    });
                    break;
                }
                used = required;
                preceding = Some(group.after);
                item = group.end;
            }
        }
        let terminal = item == after.item;
        if !terminal && rejections.is_empty() {
            return Err(error(root, E::ColumnBalanceContradiction));
        }
        Ok((terminal, rejections))
    }
    fn balance_receipt_bytes(
        &mut self,
        input: [u8; 32],
        selected: PositiveLength,
        candidates: &[BookV2ColumnBalanceCandidate],
    ) -> Result<Vec<u8>, ProductionBodyPaginationError> {
        self.balance_blob(|this, out| {
            out.write(&sha256(BOOK_V2_COLUMN_BALANCE_ALGORITHM.as_bytes()))?;
            out.write(&input)?;
            out.write(&selected.get().raw().to_be_bytes())?;
            number(out, candidates.len())?;
            for candidate in candidates {
                this.inner.content.step(NodeId::new(0))?;
                out.write(&candidate.target.get().raw().to_be_bytes())?;
                out.write(&[u8::from(candidate.terminal)])?;
                out.write(&candidate.rejection_fingerprint)?;
                number(out, candidate.rejections.len())?;
                for r in &candidate.rejections {
                    this.inner.content.step(r.owner)?;
                    out.write(&r.column.to_be_bytes())?;
                    out.write(&r.owner.get().to_be_bytes())?;
                    number(out, r.start)?;
                    number(out, r.end)?;
                    out.write(&r.required.get().raw().to_be_bytes())?;
                    out.write(&r.deficit.get().raw().to_be_bytes())?;
                    out.write(&[u8::from(r.table_group.is_some())])?;
                    if let Some(group) = r.table_group {
                        out.write(&group)?;
                    }
                }
                out.write(&candidate.capacity_fingerprint)?;
                number(out, candidate.capacity_canonical.len())?;
                out.write(&candidate.capacity_canonical)?;
            }
            Ok(())
        })
    }
    /// Full-height source selection precedes every balance attempt. Earlier
    /// physical pages stay fixed; a replacement rebuilds the original suffix.
    pub fn select_balanced_pages(
        &mut self,
    ) -> Result<BookV2ColumnPageSequence<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
        let sequence = self.select_pages()?;
        if !self.plan.requests_last_page_balance() {
            return Ok(sequence);
        }
        self.verify_sequence(&sequence)?;
        let root = NodeId::new(0);
        let mut eligible = None;
        for (index, page) in sequence.pages().iter().enumerate().rev() {
            self.inner.content.step(root)?;
            let mut body = false;
            for column in page.candidate().columns() {
                self.inner.content.step(root)?;
                body |= column.used_height() > Length::ZERO;
            }
            if body
                || page
                    .candidate()
                    .footnotes()
                    .is_some_and(|n| !n.fragments().is_empty())
            {
                eligible = Some(index);
                break;
            }
        }
        let Some(index) = eligible else {
            return Ok(sequence);
        };
        let initial = &sequence.pages[index];
        let frames = initial.candidate().frames();
        if !frames.requires_last_page_balance() {
            return Ok(sequence);
        }
        let mut extent = Length::ZERO;
        for column in initial.candidate().columns() {
            self.inner.content.step(root)?;
            if column.used_height() > Length::ZERO {
                extent = add(extent, column.used_height(), root)?;
            }
        }
        if extent == Length::ZERO {
            return Ok(sequence);
        }
        let full = frames.body().height();
        let n = i64::from(frames.column_count());
        let first = extent
            .raw()
            .checked_add(n - 1)
            .and_then(|e| e.checked_div(n))
            .map(|h| h.clamp(1, full.get().raw()))
            .and_then(Length::from_raw)
            .and_then(PositiveLength::new)
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        let next_candidate = self
            .balance_candidates
            .checked_add(1)
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        if next_candidate > u64::from(self.maximum_balance_candidates) {
            return Err(error(
                root,
                E::ColumnBalanceLimit {
                    limit: self.maximum_balance_candidates,
                    observed: next_candidate,
                },
            ));
        }
        let (input, input_canonical) = self.balance_input(&sequence, index, extent)?;
        self.inner.content.charge(1, root)?;
        let mut candidates: Vec<BookV2ColumnBalanceCandidate> = Vec::new();
        let mut target = first;
        let mut attempts = 0;
        loop {
            let observed = self
                .balance_candidates
                .checked_add(1)
                .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
            if observed > u64::from(self.maximum_balance_candidates) {
                return Err(error(
                    root,
                    E::ColumnBalanceLimit {
                        limit: self.maximum_balance_candidates,
                        observed,
                    },
                ));
            }
            self.balance_candidates = observed;
            self.inner.content.charge(1, root)?;
            candidates
                .try_reserve(1)
                .map_err(|_| error(root, E::AllocationFailure))?;
            let before = &sequence.entry_state(index).unwrap().source.source;
            let after = &initial.candidate().next.source;
            let ordinary = before.next_table == after.next_table
                && before.continuation.is_none()
                && after.continuation.is_none();
            let capacity = if ordinary {
                Some(self.ordinary_balance_capacity(
                    sequence.entry_state(index).unwrap(),
                    initial,
                    target,
                )?)
            } else {
                None
            };
            // With nonnegative original extents and gaps, starting later can
            // only remove occupancy. Greedy hard-legal cuts dominate every
            // earlier cut, so a nonterminal maximal prefix proves this height
            // cannot reach the baseline endpoint, independent of soft ranking.
            let (selected, first_nonterminal) =
                if capacity.as_ref().is_some_and(|(terminal, _)| !terminal) {
                    (None, None)
                } else {
                    self.select_page_to_endpoint_at_height(
                        sequence.entry_state(index).unwrap(),
                        frames.named_page_index(),
                        target,
                        &mut attempts,
                        initial.next_state(),
                    )?
                };
            if let Some(mut selected) = selected {
                candidates.push(BookV2ColumnBalanceCandidate {
                    target,
                    terminal: true,
                    rejections: Vec::new(),
                    rejection_fingerprint: [0; 32],
                    capacity_canonical: Vec::new(),
                    capacity_fingerprint: [0; 32],
                });
                let canonical = self.balance_receipt_bytes(input, target, &candidates)?;
                let fingerprint = self.balance_hash(&canonical)?;
                selected.balance = Some(BookV2ColumnBalanceReceipt {
                    owner: self.inner.owner_id,
                    limits: self.balance_limits,
                    measurements: sequence.measurements,
                    page: initial.page_index(),
                    extent,
                    full,
                    first,
                    selected: target,
                    input,
                    input_canonical,
                    candidates,
                    canonical,
                    fingerprint,
                });
                return self.replace_page_and_select_suffix(sequence, index, selected);
            }
            let rejections = if let Some((_, rejections)) = capacity {
                if rejections.is_empty() {
                    return Err(error(root, E::ColumnBalanceContradiction));
                }
                rejections
            } else {
                self.mixed_balance_rejections(
                    sequence.entry_state(index).unwrap(),
                    initial,
                    first_nonterminal.as_ref(),
                    target,
                )?
            };
            let rejection_fingerprint = self.rejection_hash(&rejections)?;
            let capacity_canonical = self.balance_capacity_trace(
                sequence.entry_state(index).unwrap(),
                initial,
                first_nonterminal.as_ref(),
                target,
                ordinary,
                &rejections,
            )?;
            let capacity_fingerprint = self.balance_hash(&capacity_canonical)?;
            for previous in &candidates {
                self.inner.content.step(root)?;
                if previous.rejection_fingerprint == rejection_fingerprint {
                    return Err(error(root, E::ColumnBalanceOscillation));
                }
            }
            let mut least = None;
            for rejection in &rejections {
                self.inner.content.step(rejection.owner)?;
                if least.is_none_or(|prior: &BookV2ColumnBalanceRejection| {
                    (
                        rejection.deficit.get().raw(),
                        rejection.column,
                        rejection.start,
                    ) < (prior.deficit.get().raw(), prior.column, prior.start)
                }) {
                    least = Some(rejection);
                }
            }
            let least = least.ok_or_else(|| error(root, E::ColumnBalanceContradiction))?;
            let next = add(target.get(), least.deficit.get(), least.owner)?;
            if next <= target.get() || next > full.get() {
                return Err(error(root, E::ColumnBalanceContradiction));
            }
            candidates.push(BookV2ColumnBalanceCandidate {
                target,
                terminal: false,
                rejections,
                rejection_fingerprint,
                capacity_canonical,
                capacity_fingerprint,
            });
            target = PositiveLength::new(next)
                .ok_or_else(|| error(root, E::ColumnBalanceContradiction))?;
        }
    }
    pub(super) fn verify_balance_page(
        &self,
        page: &BookV2ColumnPageSelection<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let Some(receipt) = page.balance() else {
            return Ok(());
        };
        if receipt.owner != self.inner.owner_id
            || receipt.limits != self.balance_limits
            || receipt.measurements
                != self
                    .inner
                    .tables
                    .as_ref()
                    .unwrap()
                    .measurements_fingerprint()
            || receipt.page != page.page_index()
            || receipt.full != page.candidate().frames().body().height()
            || receipt.selected.get() > receipt.full.get()
            || receipt
                .candidates
                .last()
                .is_none_or(|c| !c.terminal || c.target != receipt.selected)
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        Ok(())
    }
}
