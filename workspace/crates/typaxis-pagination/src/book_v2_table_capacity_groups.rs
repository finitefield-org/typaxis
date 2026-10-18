//! Sealed nearest original-source events from the actual table evaluator.
//! Measured capacity guards skip intervals whose original source atoms are
//! constant. An uncovered branch falls back to a budgeted exhaustive walk.
use super::*;

/// An actual admission at a tested capacity, including zero. It carries no
/// nearest-group claim, column selection, or paint permission.
pub(in crate::production_body::body_flow) struct BookV2TableCapacityObservation<'m, 'f, 's, 'p, 'a>
{
    before: BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
    after: BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
    capacity: Length,
    used: Length,
    forced: Option<NodeId>,
    source_progress: bool,
    before_atoms: Vec<u8>,
    after_atoms: Vec<u8>,
    fragment: [u8; 32],
    canonical: Vec<u8>,
    fingerprint: [u8; 32],
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableCapacityObservation<'m, 'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) fn before(
        &self,
    ) -> BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
        self.before
    }
    pub(in crate::production_body::body_flow) fn after(
        &self,
    ) -> BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
        self.after
    }
    pub(in crate::production_body::body_flow) fn capacity(&self) -> Length {
        self.capacity
    }
    pub(in crate::production_body::body_flow) fn used_height(&self) -> Length {
        self.used
    }
    pub(in crate::production_body::body_flow) fn forced_break_owner(&self) -> Option<NodeId> {
        self.forced
    }
    pub(in crate::production_body::body_flow) fn source_progress(&self) -> bool {
        self.source_progress
    }
    pub(in crate::production_body::body_flow) fn before_atom_bytes(&self) -> &[u8] {
        &self.before_atoms
    }
    pub(in crate::production_body::body_flow) fn after_atom_bytes(&self) -> &[u8] {
        &self.after_atoms
    }
    pub(in crate::production_body::body_flow) fn fragment_fingerprint(&self) -> [u8; 32] {
        self.fragment
    }
    pub(in crate::production_body::body_flow) fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
    pub(in crate::production_body::body_flow) fn into_canonical_bytes(self) -> Vec<u8> {
        self.canonical
    }
    pub(in crate::production_body::body_flow) fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

pub(in crate::production_body::body_flow) struct BookV2TableCapacityGroup<'m, 'f, 's, 'p, 'a> {
    before: BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
    current_after: Option<BookV2TableCursor<'m, 'f, 's, 'p, 'a>>,
    next_after: BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
    lower_after: Option<BookV2TableCursor<'m, 'f, 's, 'p, 'a>>,
    current_capacity: Length,
    required_capacity: PositiveLength,
    current_atoms: [u8; 32],
    next_atoms: [u8; 32],
    current_fragment: Option<[u8; 32]>,
    next_fragment: [u8; 32],
    lower_fragment: Option<[u8; 32]>,
    current_used: Option<Length>,
    lower_used: Option<Length>,
    next_used: Length,
    fingerprint: [u8; 32],
    trials: u64,
    skipped: u64,
    canonical: Vec<u8>,
    current_atom_bytes: Vec<u8>,
    lower_atom_bytes: Vec<u8>,
    next_atom_bytes: Vec<u8>,
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableCapacityGroup<'m, 'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) fn before(
        &self,
    ) -> BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
        self.before
    }
    pub(in crate::production_body::body_flow) fn current_after(
        &self,
    ) -> Option<BookV2TableCursor<'m, 'f, 's, 'p, 'a>> {
        self.current_after
    }
    pub(in crate::production_body::body_flow) fn next_after(
        &self,
    ) -> BookV2TableCursor<'m, 'f, 's, 'p, 'a> {
        self.next_after
    }
    pub(in crate::production_body::body_flow) fn lower_after(
        &self,
    ) -> Option<BookV2TableCursor<'m, 'f, 's, 'p, 'a>> {
        self.lower_after
    }
    pub(in crate::production_body::body_flow) fn current_capacity(&self) -> Length {
        self.current_capacity
    }
    pub(in crate::production_body::body_flow) fn required_capacity(&self) -> PositiveLength {
        self.required_capacity
    }
    pub(in crate::production_body::body_flow) fn current_used_height(&self) -> Option<Length> {
        self.current_used
    }
    pub(in crate::production_body::body_flow) fn lower_used_height(&self) -> Option<Length> {
        self.lower_used
    }
    pub(in crate::production_body::body_flow) fn next_used_height(&self) -> Length {
        self.next_used
    }
    pub(in crate::production_body::body_flow) fn current_atom_fingerprint(&self) -> [u8; 32] {
        self.current_atoms
    }
    pub(in crate::production_body::body_flow) fn next_atom_fingerprint(&self) -> [u8; 32] {
        self.next_atoms
    }
    pub(in crate::production_body::body_flow) fn current_fragment_fingerprint(
        &self,
    ) -> Option<[u8; 32]> {
        self.current_fragment
    }
    pub(in crate::production_body::body_flow) fn next_fragment_fingerprint(&self) -> [u8; 32] {
        self.next_fragment
    }
    pub(in crate::production_body::body_flow) fn lower_fragment_fingerprint(
        &self,
    ) -> Option<[u8; 32]> {
        self.lower_fragment
    }
    pub(in crate::production_body::body_flow) fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub(in crate::production_body::body_flow) fn trial_count(&self) -> u64 {
        self.trials
    }
    pub(in crate::production_body::body_flow) fn skipped_capacity_count(&self) -> u64 {
        self.skipped
    }
    pub(in crate::production_body::body_flow) fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
    pub(in crate::production_body::body_flow) fn into_canonical_bytes(self) -> Vec<u8> {
        self.canonical
    }
    pub(in crate::production_body::body_flow) fn current_atom_bytes(&self) -> &[u8] {
        &self.current_atom_bytes
    }
    pub(in crate::production_body::body_flow) fn lower_atom_bytes(&self) -> &[u8] {
        &self.lower_atom_bytes
    }
    pub(in crate::production_body::body_flow) fn next_atom_bytes(&self) -> &[u8] {
        &self.next_atom_bytes
    }
}

#[derive(Clone, Copy)]
pub(super) enum AtomMode {
    Empty,
    Complete,
    Cursor(TablePosition),
}

/// Each write retains its accepted cumulative spool prefix before allocation.
/// The bytes describe immutable original source positions, never a new cursor.
pub(super) struct ProofBytes<'s> {
    bytes: Vec<u8>,
    spool: &'s mut u64,
    maximum: u64,
}
impl<'s> ProofBytes<'s> {
    fn new(spool: &'s mut u64, maximum: u64) -> Self {
        Self {
            bytes: Vec::new(),
            spool,
            maximum,
        }
    }
    pub(super) fn write(
        &mut self,
        bytes: &[u8],
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        let next = self
            .spool
            .checked_add(bytes.len() as u64)
            .filter(|n| *n <= self.maximum)
            .ok_or_else(|| error(owner, E::SpoolLimit))?;
        *self.spool = next;
        self.bytes
            .try_reserve_exact(bytes.len())
            .map_err(|_| error(owner, E::AllocationFailure))?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    pub(super) fn number(
        &mut self,
        n: usize,
        owner: NodeId,
    ) -> Result<(), ProductionBodyPaginationError> {
        self.write(&(n as u64).to_be_bytes(), owner)
    }
}

fn extends_atoms(before: &[u8], after: &[u8]) -> bool {
    before.len() == after.len()
        && before.len().is_multiple_of(8)
        && before
            .chunks_exact(8)
            .zip(after.chunks_exact(8))
            .all(|(a, b)| a <= b)
        && before != after
}

fn encode_cursor(
    out: &mut ProofBytes<'_>,
    cursor: BookV2TableCursor<'_, '_, '_, '_, '_>,
    owner: NodeId,
) -> Result<(), ProductionBodyPaginationError> {
    for n in [
        cursor.table_index(),
        cursor.next_row(),
        cursor.next_caption_item(),
    ] {
        out.number(n, owner)?;
    }
    out.write(&cursor.offset().raw().to_be_bytes(), owner)?;
    out.write(
        &[
            u8::from(cursor.is_initial()),
            u8::from(cursor.has_started_rows()),
        ],
        owner,
    )?;
    out.write(&cursor.cell_progress_fingerprint(), owner)
}

impl<'m, 'f, 's, 'p, 'a> BookV2TableBreakSearch<'m, 'f, 's, 'p, 'a> {
    pub(super) fn append_capacity_cursor(
        &mut self,
        cursor: &BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
        out: &mut ProofBytes<'_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let table = &self.measurements.tables()[self.table_index];
        let owner = table.owner;
        self.kernel.work.take(1, owner)?;
        if !std::ptr::eq(cursor.measurements, self.measurements)
            || cursor.table_index != self.table_index
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        encode_cursor(out, *cursor, owner)?;
        out.write(
            &[
                u8::from(cursor.position.cells.is_some()),
                u8::from(self.nested.is_some()),
            ],
            owner,
        )?;
        if self.nested.is_some() {
            return self.append_nested_capacity_cursor(cursor.position, out);
        }
        let remaining = self.kernel.capacity_row_remaining(&cursor.position)?;
        out.write(&[u8::from(remaining.is_some())], owner)?;
        out.write(&remaining.map_or(0, |n| n.raw()).to_be_bytes(), owner)?;
        out.number(table.cells.len(), owner)?;
        self.kernel
            .visit_capacity_cell_positions(&cursor.position, |_, next| out.number(next, owner))
    }
    pub(super) fn append_capacity_atoms(
        &mut self,
        mode: AtomMode,
        out: &mut ProofBytes<'_>,
    ) -> Result<(), ProductionBodyPaginationError> {
        let table = &self.measurements.tables()[self.table_index];
        let owner = table.owner;
        self.kernel.work.take(1, owner)?;
        let (row, caption, caption_done, header, terminal) = match mode {
            AtomMode::Empty => (0, 0, false, false, false),
            AtomMode::Complete => (
                table.rows.len(),
                table.caption.as_ref().map_or(0, |c| c.content.len()),
                table.caption.is_some(),
                self.kernel.header_rows > 0,
                true,
            ),
            AtomMode::Cursor(p) => {
                if p.row > table.rows.len()
                    || p.caption_next > table.caption.as_ref().map_or(0, |c| c.content.len())
                {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                (
                    if p.initial && !p.header_seen {
                        0
                    } else {
                        p.row
                    },
                    p.caption_next,
                    table.caption.as_ref().is_some_and(|caption| {
                        p.caption_next == caption.content.len()
                            && (self.nested.is_some()
                                || p.header_seen
                                || p.cells.is_some()
                                || p.offset >= caption.height)
                    }),
                    p.header_seen && self.kernel.header_rows > 0,
                    !p.initial && p.header_seen && p.row == table.rows.len(),
                )
            }
        };
        for n in [
            row,
            caption,
            usize::from(caption_done),
            usize::from(header),
            usize::from(terminal),
        ] {
            out.number(n, owner)?;
        }
        if self.nested.is_some() {
            return self.append_nested_capacity_atoms(mode, out);
        }
        match mode {
            AtomMode::Empty | AtomMode::Complete => {
                for cell in &table.cells {
                    self.kernel.work.take(1, cell.owner)?;
                    out.number(
                        if matches!(mode, AtomMode::Complete) {
                            cell.content.len()
                        } else {
                            0
                        },
                        cell.owner,
                    )?;
                }
            }
            AtomMode::Cursor(p) => self
                .kernel
                .visit_capacity_cell_positions(&p, |_, next| out.number(next, owner))?,
        }
        Ok(())
    }

    fn capacity_atoms(
        &mut self,
        cursor: BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
        spool: &mut u64,
    ) -> Result<Vec<u8>, ProductionBodyPaginationError> {
        let owner = self.measurements.tables()[self.table_index].owner;
        self.kernel.charge.take(1, owner)?;
        let mut out = ProofBytes::new(
            spool,
            self.kernel.capacity_proof_spool_limit().unwrap_or(u64::MAX),
        );
        self.append_capacity_atoms(AtomMode::Cursor(cursor.position), &mut out)?;
        Ok(out.bytes)
    }

    fn capacity_proof_hash(
        &mut self,
        bytes: &[u8],
    ) -> Result<[u8; 32], ProductionBodyPaginationError> {
        self.kernel.work.take(
            bytes.len().div_ceil(64) as u64,
            self.measurements.tables()[self.table_index].owner,
        )?;
        Ok(sha256(bytes))
    }

    #[allow(clippy::too_many_arguments)]
    fn capacity_trial(
        &mut self,
        before: &BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
        available: Length,
        headers: Option<&'m crate::book_v2::BookV2TableHeaderCatalog<'m, 'f, 's, 'p, 'a>>,
        width: Option<PositiveLength>,
        name: Option<usize>,
        spool: &mut u64,
    ) -> Result<
        (
            Option<BookV2TableFragmentSelection<'m, 'f, 's, 'p, 'a>>,
            kernel::CapacityProbe,
        ),
        ProductionBodyPaginationError,
    > {
        let previous = self.kernel.capacity_probe;
        self.kernel.capacity_probe = Some(kernel::CapacityProbe::new(
            available,
            *spool,
            self.kernel.capacity_proof_spool_limit().unwrap_or(u64::MAX),
        ));
        let result = self.evaluate_named_in_frame(before, available, headers, width, name);
        let observed = self
            .kernel
            .capacity_probe
            .expect("private capacity observer");
        self.kernel.capacity_probe = previous;
        *spool = observed
            .prior_spool
            .checked_add(observed.retained_spool)
            .ok_or_else(|| {
                error(
                    self.measurements.tables()[self.table_index].owner,
                    E::ArithmeticOverflow,
                )
            })?;
        result.map(|selected| (selected, observed))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn observe_capacity(
        &mut self,
        before: &BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
        capacity: Length,
        maximum_capacity: PositiveLength,
        headers: Option<&'m crate::book_v2::BookV2TableHeaderCatalog<'m, 'f, 's, 'p, 'a>>,
        width: Option<PositiveLength>,
        page_name: Option<usize>,
        spool: &mut u64,
    ) -> Result<
        Option<BookV2TableCapacityObservation<'m, 'f, 's, 'p, 'a>>,
        ProductionBodyPaginationError,
    > {
        let owner = self.measurements.tables()[self.table_index].owner;
        self.kernel.work.take(1, owner)?;
        if !std::ptr::eq(before.measurements, self.measurements)
            || before.table_index != self.table_index
            || headers.is_some_and(|h| !std::ptr::eq(h.base(), self.measurements))
            || before.is_terminal()
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        if capacity < Length::ZERO
            || capacity > maximum_capacity.get()
            || maximum_capacity.get() > self.maximum_height()
        {
            return Err(error(owner, E::InvalidTableCapacity));
        }
        if self
            .kernel
            .capacity_proof_spool_limit()
            .is_some_and(|limit| *spool > limit)
        {
            return Err(error(owner, E::SpoolLimit));
        }
        let (selected, _) =
            self.capacity_trial(before, capacity, headers, width, page_name, spool)?;
        let Some(selected) = selected else {
            return Ok(None);
        };
        let used = selected.used_height();
        if used < Length::ZERO || used > capacity {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let before_atoms = self.capacity_atoms(*before, spool)?;
        let after_atoms = self.capacity_atoms(selected.after(), spool)?;
        self.kernel
            .work
            .take(before_atoms.len().div_ceil(8) as u64 + 1, owner)?;
        let source_progress = extends_atoms(&before_atoms, &after_atoms);
        if before_atoms != after_atoms && !source_progress {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let before_hash = self.capacity_proof_hash(&before_atoms)?;
        let after_hash = self.capacity_proof_hash(&after_atoms)?;
        let forced = selected.forced_break_owner();
        self.kernel.charge.take(2, owner)?;
        let mut out = ProofBytes::new(
            spool,
            self.kernel.capacity_proof_spool_limit().unwrap_or(u64::MAX),
        );
        out.write(
            &sha256(b"typaxis.book-2-table-capacity-observation/1"),
            owner,
        )?;
        out.write(&self.measurements.fingerprint(), owner)?;
        out.write(&headers.map_or([0; 32], |h| h.fingerprint()), owner)?;
        out.write(
            &[
                u8::from(headers.is_some()),
                u8::from(width.is_some()),
                u8::from(page_name.is_some()),
            ],
            owner,
        )?;
        out.write(&width.map_or(0, |w| w.get().raw()).to_be_bytes(), owner)?;
        out.number(page_name.unwrap_or(0), owner)?;
        out.write(&capacity.raw().to_be_bytes(), owner)?;
        out.write(&maximum_capacity.get().raw().to_be_bytes(), owner)?;
        out.write(&used.raw().to_be_bytes(), owner)?;
        out.write(&[u8::from(forced.is_some())], owner)?;
        out.write(&forced.map_or(0, |n| n.get()).to_be_bytes(), owner)?;
        out.write(&[u8::from(source_progress)], owner)?;
        for cursor in [*before, selected.after()] {
            out.write(&[1], owner)?;
            self.append_capacity_cursor(&cursor, &mut out)?;
        }
        for hash in [before_hash, after_hash, selected.fingerprint()] {
            out.write(&hash, owner)?;
        }
        for atoms in [&before_atoms, &after_atoms] {
            out.number(atoms.len(), owner)?;
            out.write(atoms, owner)?;
        }
        let fingerprint = self.capacity_proof_hash(&out.bytes)?;
        Ok(Some(BookV2TableCapacityObservation {
            before: *before,
            after: selected.after(),
            capacity,
            used,
            forced,
            source_progress,
            before_atoms,
            after_atoms,
            fragment: selected.fingerprint(),
            canonical: out.bytes,
            fingerprint,
        }))
    }

    /// The next atom is established by increasing exact replay from the same
    /// before cursor. No table search error is translated into a capacity fit.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn next_capacity_group(
        &mut self,
        before: &BookV2TableCursor<'m, 'f, 's, 'p, 'a>,
        current_capacity: Length,
        maximum_capacity: PositiveLength,
        headers: Option<&'m crate::book_v2::BookV2TableHeaderCatalog<'m, 'f, 's, 'p, 'a>>,
        width: Option<PositiveLength>,
        page_name: Option<usize>,
        spool: &mut u64,
    ) -> Result<Option<BookV2TableCapacityGroup<'m, 'f, 's, 'p, 'a>>, ProductionBodyPaginationError>
    {
        let owner = self.measurements.tables()[self.table_index].owner;
        self.kernel.work.take(1, owner)?;
        if !std::ptr::eq(before.measurements, self.measurements)
            || before.table_index != self.table_index
            || headers.is_some_and(|h| !std::ptr::eq(h.base(), self.measurements))
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        if current_capacity < Length::ZERO
            || current_capacity > maximum_capacity.get()
            || maximum_capacity.get() > self.maximum_height()
        {
            return Err(error(owner, E::InvalidTableCapacity));
        }
        if before.is_terminal() {
            return Err(error(owner, E::ReceiptMismatch));
        }
        if self
            .kernel
            .capacity_proof_spool_limit()
            .is_some_and(|limit| *spool > limit)
        {
            return Err(error(owner, E::SpoolLimit));
        }
        let mut trials = 1u64;
        self.kernel.work.take(1, owner)?;
        let (current, mut observed) =
            self.capacity_trial(before, current_capacity, headers, width, page_name, spool)?;
        let current_after = current.as_ref().map(|s| s.after());
        let current_atoms = self.capacity_atoms(current_after.unwrap_or(*before), spool)?;
        let current_fragment = current.as_ref().map(|s| s.fingerprint());
        let mut capacity = current_capacity;
        let mut skipped = 0u64;
        loop {
            if capacity == maximum_capacity.get() {
                return Ok(None);
            }
            let unit_next = add(
                capacity,
                Length::from_raw(1).expect("one layout unit"),
                owner,
            )?;
            let next = if observed.complete {
                match observed.next {
                    Some(next) => next,
                    None => return Ok(None),
                }
            } else {
                unit_next
            };
            if next <= capacity {
                return Err(error(owner, E::ReceiptMismatch));
            }
            if next > maximum_capacity.get() {
                return Ok(None);
            }
            skipped = skipped
                .checked_add((next.raw() - unit_next.raw()) as u64)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            capacity = next;
            trials = trials
                .checked_add(1)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            self.kernel.work.take(1, owner)?;
            let (selected, next_observed) =
                self.capacity_trial(before, capacity, headers, width, page_name, spool)?;
            observed = next_observed;
            let Some(selected) = selected else {
                continue;
            };
            let atoms = self.capacity_atoms(selected.after(), spool)?;
            self.kernel
                .work
                .take(atoms.len().div_ceil(8) as u64 + 1, owner)?;
            if atoms == current_atoms {
                continue;
            }
            if !extends_atoms(&current_atoms, &atoms) {
                return Err(error(owner, E::ReceiptMismatch));
            }
            let lower_capacity = capacity
                .checked_sub(Length::from_raw(1).expect("one layout unit"))
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            trials = trials
                .checked_add(1)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            self.kernel.work.take(1, owner)?;
            let (lower, _) =
                self.capacity_trial(before, lower_capacity, headers, width, page_name, spool)?;
            let lower_atoms =
                self.capacity_atoms(lower.as_ref().map_or(*before, |s| s.after()), spool)?;
            self.kernel
                .work
                .take(lower_atoms.len().div_ceil(8) as u64 + 1, owner)?;
            if lower_atoms != current_atoms {
                return Err(error(owner, E::ReceiptMismatch));
            }
            let current_hash = self.capacity_proof_hash(&current_atoms)?;
            let next_hash = self.capacity_proof_hash(&atoms)?;
            let lower_hash = self.capacity_proof_hash(&lower_atoms)?;
            let lower_after = lower.as_ref().map(|s| s.after());
            let lower_fragment = lower.as_ref().map(|s| s.fingerprint());
            let current_used = current.as_ref().map(|s| s.used_height());
            let lower_used = lower.as_ref().map(|s| s.used_height());
            let next_used = selected.used_height();
            if current_used.is_some_and(|n| n < Length::ZERO || n > current_capacity)
                || lower_used.is_some_and(|n| n < Length::ZERO || n > lower_capacity)
                || next_used < Length::ZERO
                || next_used > capacity
            {
                return Err(error(owner, E::ReceiptMismatch));
            }
            if trials.checked_sub(2).and_then(|n| n.checked_add(skipped))
                != u64::try_from(capacity.raw() - current_capacity.raw()).ok()
            {
                return Err(error(owner, E::ReceiptMismatch));
            }
            self.kernel.charge.take(2, owner)?;
            let mut out = ProofBytes::new(
                spool,
                self.kernel.capacity_proof_spool_limit().unwrap_or(u64::MAX),
            );
            out.write(&sha256(b"typaxis.book-2-table-capacity-group/1"), owner)?;
            out.write(&self.measurements.fingerprint(), owner)?;
            out.write(&headers.map_or([0; 32], |h| h.fingerprint()), owner)?;
            out.write(
                &[
                    u8::from(headers.is_some()),
                    u8::from(width.is_some()),
                    u8::from(page_name.is_some()),
                ],
                owner,
            )?;
            out.write(&width.map_or(0, |w| w.get().raw()).to_be_bytes(), owner)?;
            out.number(page_name.unwrap_or(0), owner)?;
            out.write(&current_capacity.raw().to_be_bytes(), owner)?;
            out.write(&capacity.raw().to_be_bytes(), owner)?;
            out.write(&maximum_capacity.get().raw().to_be_bytes(), owner)?;
            out.write(&trials.to_be_bytes(), owner)?;
            out.write(&skipped.to_be_bytes(), owner)?;
            for cursor in [
                Some(*before),
                current_after,
                lower_after,
                Some(selected.after()),
            ] {
                out.write(&[u8::from(cursor.is_some())], owner)?;
                if let Some(cursor) = cursor {
                    self.append_capacity_cursor(&cursor, &mut out)?;
                }
            }
            for hash in [
                current_hash,
                lower_hash,
                next_hash,
                current_fragment.unwrap_or([0; 32]),
                lower_fragment.unwrap_or([0; 32]),
                selected.fingerprint(),
            ] {
                out.write(&hash, owner)?;
            }
            for atom_bytes in [&current_atoms, &lower_atoms, &atoms] {
                out.number(atom_bytes.len(), owner)?;
                out.write(atom_bytes, owner)?;
            }
            out.write(b"USEDHGHT", owner)?;
            out.write(
                &[
                    u8::from(current_used.is_some()),
                    u8::from(lower_used.is_some()),
                ],
                owner,
            )?;
            for used in [
                current_used.unwrap_or(Length::ZERO),
                lower_used.unwrap_or(Length::ZERO),
                next_used,
            ] {
                out.write(&used.raw().to_be_bytes(), owner)?;
            }
            let fingerprint = self.capacity_proof_hash(&out.bytes)?;
            return Ok(Some(BookV2TableCapacityGroup {
                before: *before,
                current_after,
                next_after: selected.after(),
                lower_after,
                current_capacity,
                required_capacity: PositiveLength::new(capacity)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
                current_atoms: current_hash,
                next_atoms: next_hash,
                current_fragment,
                next_fragment: selected.fingerprint(),
                lower_fragment,
                current_used,
                lower_used,
                next_used,
                fingerprint,
                trials,
                skipped,
                canonical: out.bytes,
                current_atom_bytes: current_atoms,
                lower_atom_bytes: lower_atoms,
                next_atom_bytes: atoms,
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn atom(n: &[u64]) -> Vec<u8> {
        n.iter().flat_map(|v| v.to_be_bytes()).collect()
    }
    #[test]
    fn book_v2_table_nearest_atoms_require_componentwise_original_progress() {
        assert!(extends_atoms(&atom(&[1, 0, 2]), &atom(&[1, 1, 2])));
        assert!(!extends_atoms(&atom(&[1, 1, 2]), &atom(&[2, 0, 3])));
        assert!(!extends_atoms(&atom(&[1, 1, 2]), &atom(&[1, 1, 2])));
        assert!(!extends_atoms(&atom(&[1]), &atom(&[1, 2])));
    }
    #[test]
    fn book_v2_table_nearest_spool_retains_only_accepted_prefix() {
        let owner = NodeId::new(7);
        let mut spool = 2;
        let mut out = ProofBytes::new(&mut spool, 12);
        out.write(&[1, 2, 3], owner).unwrap();
        assert_eq!(out.write(&[0; 8], owner).unwrap_err().kind, E::SpoolLimit);
        assert_eq!(out.bytes, [1, 2, 3]);
        drop(out);
        assert_eq!(spool, 5);
    }
}
