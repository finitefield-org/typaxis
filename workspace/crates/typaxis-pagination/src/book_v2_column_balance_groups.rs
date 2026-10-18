//! Capacity witnesses for actual mixed column cuts. A partial table extends
//! its original fragment; it is never appended as a second continuation paint.
use super::*;

#[derive(Clone, Copy)]
struct Point<'b, 'f, 's, 'p, 'a> {
    item: usize,
    table: usize,
    continuation: Option<BookV2TableCursor<'b, 'f, 's, 'p, 'a>>,
}
struct KeepSeed {
    owner: NodeId,
    start: usize,
    admission: Length,
    fingerprint: [u8; 32],
    proofs: Vec<Vec<u8>>,
}
pub(super) fn same_cursor(
    a: BookV2TableCursor<'_, '_, '_, '_, '_>,
    b: BookV2TableCursor<'_, '_, '_, '_, '_>,
) -> bool {
    a.table_index() == b.table_index()
        && a.offset() == b.offset()
        && a.next_row() == b.next_row()
        && a.next_caption_item() == b.next_caption_item()
        && a.is_initial() == b.is_initial()
        && a.has_started_rows() == b.has_started_rows()
        && a.cell_progress_fingerprint() == b.cell_progress_fingerprint()
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnPageSearch<'b, 'f, 's, 'p, 'a> {
    fn mixed_rejection(
        &mut self,
        column: u16,
        owner: NodeId,
        range: std::ops::Range<usize>,
        required: Length,
        target: PositiveLength,
        table_group: Option<[u8; 32]>,
        table_proofs: Vec<Vec<u8>>,
    ) -> Result<Option<BookV2ColumnBalanceRejection>, ProductionBodyPaginationError> {
        if required <= target.get() {
            return Ok(None);
        }
        let deficit = required
            .checked_sub(target.get())
            .and_then(PositiveLength::new)
            .ok_or_else(|| error(owner, E::ColumnBalanceContradiction))?;
        let required = PositiveLength::new(required)
            .ok_or_else(|| error(owner, E::ColumnBalanceContradiction))?;
        self.inner.content.charge(1, owner)?;
        Ok(Some(BookV2ColumnBalanceRejection {
            column,
            owner,
            start: range.start,
            end: range.end,
            required,
            deficit,
            table_group,
            table_proofs,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn partial_table_rejection(
        &mut self,
        column: u16,
        part: &BookV2BodySelectedPart<'b, 'f, 's, 'p, 'a>,
        target: PositiveLength,
        full: PositiveLength,
        width: PositiveLength,
        name: Option<usize>,
    ) -> Result<Option<BookV2ColumnBalanceRejection>, ProductionBodyPaginationError> {
        let table = part
            .table()
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let before = table.before();
        let available = target
            .get()
            .checked_sub(part.top())
            .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow))?;
        let context = self.inner.tables.as_ref().unwrap();
        let owner = context.table(before.table_index()).owner();
        let range = context
            .range(before.table_index())
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        let keep = context.table(before.table_index()).keep_with_next();
        let after_spacing = context.table(before.table_index()).space_after();
        let successor = context.successor(before.table_index());
        let more_source =
            range.end < self.inner.content.flow.body_items().len() || successor < context.len();
        self.inner.content.charge(1, owner)?;
        let mut proofs = Vec::new();
        let mut fingerprint = None;
        let mut capacity = table.available_height();
        let mut expected = table.after();
        let mut expected_used = table.used_height();
        loop {
            self.inner.content.step(owner)?;
            let group = self
                .inner
                .tables
                .as_mut()
                .unwrap()
                .next_capacity_group(
                    &before,
                    capacity,
                    full,
                    self.inner.headers,
                    Some(width),
                    name,
                    &mut self.inner.content.charge,
                    &mut self.inner.content.steps,
                    &mut self.inner.terminal_spool,
                )?
                .ok_or_else(|| error(owner, E::ColumnBalanceContradiction))?;
            if !same_cursor(group.before(), before)
                || group
                    .current_after()
                    .is_none_or(|c| !same_cursor(c, expected))
                || group.current_used_height() != Some(expected_used)
            {
                return Err(error(owner, E::ReceiptMismatch));
            }
            let needed = group.required_capacity().get();
            let next = group.next_after();
            let next_used = group.next_used_height();
            let required = add(part.top(), needed, owner)?;
            let previous = fingerprint;
            let group_fingerprint = group.fingerprint();
            let bytes = self.balance_blob(|this, out| {
                this.inner.content.step(owner)?;
                out.write(&sha256(b"typaxis.book-2-column-table-keep-group/1"))?;
                out.write(&previous.unwrap_or([0; 32]))?;
                out.write(&group_fingerprint)?;
                number(out, range.start)?;
                number(out, range.end)?;
                out.write(&part.top().raw().to_be_bytes())?;
                out.write(&required.raw().to_be_bytes())
            })?;
            fingerprint = Some(self.balance_hash(&bytes)?);
            self.inner.content.charge(2, owner)?;
            proofs
                .try_reserve_exact(2)
                .map_err(|_| error(owner, E::AllocationFailure))?;
            proofs.push(group.into_canonical_bytes());
            proofs.push(bytes);
            if next.is_terminal() && keep && more_source {
                let used = add(part.top(), next_used, owner)?;
                return self.next_mixed_group_rejection(
                    column,
                    Point {
                        item: range.end,
                        table: successor,
                        continuation: None,
                    },
                    used,
                    Some(after_spacing),
                    target,
                    full,
                    width,
                    name,
                    Some(KeepSeed {
                        owner,
                        start: range.start,
                        admission: required,
                        fingerprint: fingerprint.unwrap(),
                        proofs,
                    }),
                );
            }
            if needed > available {
                return self.mixed_rejection(
                    column,
                    owner,
                    range,
                    required,
                    target,
                    fingerprint,
                    proofs,
                );
            }
            if next.is_terminal() {
                return Ok(None);
            }
            capacity = needed;
            expected = next;
            expected_used = next_used;
        }
    }

    /// Measure the first hard-legal mixed group without advancing demand. A
    /// keep entering a table includes the first actual legal table source atom.
    #[allow(clippy::too_many_arguments)]
    fn next_mixed_group_rejection(
        &mut self,
        column: u16,
        mut point: Point<'b, 'f, 's, 'p, 'a>,
        mut used: Length,
        mut preceding: Option<Length>,
        target: PositiveLength,
        full: PositiveLength,
        width: PositiveLength,
        name: Option<usize>,
        seed: Option<KeepSeed>,
    ) -> Result<Option<BookV2ColumnBalanceRejection>, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let flow = self.inner.content.flow;
        let (start, mut owner, mut admission, mut proof, mut proofs) = if let Some(seed) = seed {
            (
                seed.start,
                Some(seed.owner),
                seed.admission,
                Some(seed.fingerprint),
                seed.proofs,
            )
        } else {
            self.inner.content.charge(1, root)?;
            (point.item, None, Length::ZERO, None, Vec::new())
        };
        loop {
            self.inner.content.step(root)?;
            let next_table = self
                .inner
                .tables
                .as_ref()
                .unwrap()
                .range(point.table)
                .map(|r| r.start);
            let group = if point.continuation.is_some() {
                NextOrdinarySourceGroup::Table { prefix: None }
            } else {
                next_ordinary_source_group(flow.body_items(), point.item, next_table, |owner| {
                    self.inner.content.step(owner)
                })?
            };
            match group {
                NextOrdinarySourceGroup::Complete(group) => {
                    owner.get_or_insert(group.owner);
                    let required = group.required_after(used, preceding)?.max(admission);
                    return self.mixed_rejection(
                        column,
                        owner.unwrap(),
                        start..group.end,
                        required,
                        target,
                        proof,
                        proofs,
                    );
                }
                NextOrdinarySourceGroup::Table { prefix } => {
                    if let Some(prefix) = prefix {
                        owner.get_or_insert(prefix.owner);
                        used = prefix.required_after(used, preceding)?;
                        preceding = Some(prefix.after);
                        point.item = prefix.end;
                    }
                    let context = self.inner.tables.as_ref().unwrap();
                    let measured = context.table(point.table);
                    let table_owner = measured.owner();
                    let before_spacing = measured.space_before();
                    let after_spacing = measured.space_after();
                    let keep = measured.keep_with_next();
                    let range = context
                        .range(point.table)
                        .ok_or_else(|| error(table_owner, E::ReceiptMismatch))?;
                    if range.start != point.item {
                        return Err(error(table_owner, E::ReceiptMismatch));
                    }
                    owner.get_or_insert(table_owner);
                    let top = add(
                        used,
                        preceding
                            .map(|after| add(after, before_spacing, table_owner))
                            .transpose()?
                            .unwrap_or(Length::ZERO),
                        table_owner,
                    )?;
                    let before = match point.continuation {
                        Some(cursor) => cursor,
                        None => self.inner.begin_table(point.table)?,
                    };
                    let zero = self.inner.tables.as_mut().unwrap().observe_capacity(
                        &before,
                        Length::ZERO,
                        full,
                        self.inner.headers,
                        Some(width),
                        name,
                        &mut self.inner.content.charge,
                        &mut self.inner.content.steps,
                        &mut self.inner.terminal_spool,
                    )?;
                    if let Some(zero) = zero {
                        if !same_cursor(zero.before(), before)
                            || zero.capacity() != Length::ZERO
                            || zero.used_height() != Length::ZERO
                            || !zero.source_progress()
                        {
                            return Err(error(table_owner, E::ReceiptMismatch));
                        }
                        let next = zero.after();
                        let forced = zero.forced_break_owner().is_some();
                        admission = admission.max(top);
                        let previous = proof;
                        let zero_fingerprint = zero.fingerprint();
                        let bytes = self.balance_blob(|this, out| {
                            this.inner.content.step(table_owner)?;
                            out.write(&sha256(b"typaxis.book-2-column-table-keep-group/1"))?;
                            out.write(&previous.unwrap_or([0; 32]))?;
                            out.write(&zero_fingerprint)?;
                            number(out, start)?;
                            number(out, range.end)?;
                            out.write(&top.raw().to_be_bytes())?;
                            out.write(&top.raw().to_be_bytes())
                        })?;
                        proof = Some(self.balance_hash(&bytes)?);
                        self.inner.content.charge(2, table_owner)?;
                        proofs
                            .try_reserve_exact(2)
                            .map_err(|_| error(table_owner, E::AllocationFailure))?;
                        proofs.push(zero.into_canonical_bytes());
                        proofs.push(bytes);
                        let context = self.inner.tables.as_ref().unwrap();
                        let successor = context.successor(point.table);
                        let more_source =
                            range.end < flow.body_items().len() || successor < context.len();
                        if forced || !next.is_terminal() || !keep || !more_source {
                            return self.mixed_rejection(
                                column,
                                owner.unwrap(),
                                start..range.end,
                                admission,
                                target,
                                proof,
                                proofs,
                            );
                        }
                        point.item = range.end;
                        point.table = successor;
                        point.continuation = None;
                        used = top;
                        preceding = Some(after_spacing);
                        continue;
                    }
                    // This first event is the indivisible group. It does not
                    // derive a required extent from a failed or partial fit.
                    let group = self
                        .inner
                        .tables
                        .as_mut()
                        .unwrap()
                        .next_capacity_group(
                            &before,
                            Length::ZERO,
                            full,
                            self.inner.headers,
                            Some(width),
                            name,
                            &mut self.inner.content.charge,
                            &mut self.inner.content.steps,
                            &mut self.inner.terminal_spool,
                        )?
                        .ok_or_else(|| error(table_owner, E::ColumnBalanceContradiction))?;
                    if !same_cursor(group.before(), before) || group.current_after().is_some() {
                        return Err(error(table_owner, E::ReceiptMismatch));
                    }
                    let required = add(top, group.required_capacity().get(), table_owner)?;
                    let next = group.next_after();
                    let next_used = group.next_used_height();
                    admission = admission.max(required);
                    let previous = proof;
                    let bytes = self.balance_blob(|this, out| {
                        this.inner.content.step(table_owner)?;
                        out.write(&sha256(b"typaxis.book-2-column-table-keep-group/1"))?;
                        out.write(&previous.unwrap_or([0; 32]))?;
                        out.write(&group.fingerprint())?;
                        number(out, start)?;
                        number(out, range.end)?;
                        out.write(&top.raw().to_be_bytes())?;
                        out.write(&required.raw().to_be_bytes())
                    })?;
                    proof = Some(self.balance_hash(&bytes)?);
                    self.inner.content.charge(2, table_owner)?;
                    proofs
                        .try_reserve_exact(2)
                        .map_err(|_| error(table_owner, E::AllocationFailure))?;
                    proofs.push(group.into_canonical_bytes());
                    proofs.push(bytes);
                    let context = self.inner.tables.as_ref().unwrap();
                    let successor = context.successor(point.table);
                    let more_source =
                        range.end < flow.body_items().len() || successor < context.len();
                    if !next.is_terminal() || !keep || !more_source {
                        return self.mixed_rejection(
                            column,
                            owner.unwrap(),
                            start..range.end,
                            admission,
                            target,
                            proof,
                            proofs,
                        );
                    }
                    point.item = range.end;
                    point.table = successor;
                    point.continuation = None;
                    used = add(top, next_used, table_owner)?;
                    preceding = Some(after_spacing);
                }
                NextOrdinarySourceGroup::Terminal => return Ok(None),
                NextOrdinarySourceGroup::Forced { owner, .. } => {
                    return Err(error(owner, E::KeepAcrossForcedBreak))
                }
            }
        }
    }

    pub(super) fn mixed_balance_rejections(
        &mut self,
        entry: &BookV2ColumnPageSequenceState<'b, 'f, 's, 'p, 'a>,
        initial: &BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>,
        trial: Option<&BookV2ColumnPageSelection<'b, 'f, 's, 'p, 'a>>,
        target: PositiveLength,
    ) -> Result<Vec<BookV2ColumnBalanceRejection>, ProductionBodyPaginationError> {
        let root = NodeId::new(0);
        let source = &entry.source.source;
        let endpoint = &initial.candidate().next.source;
        let mut point = Point {
            item: source.item,
            table: source.next_table,
            continuation: source.continuation,
        };
        let frames = initial.candidate().frames();
        let mut rejections = Vec::new();
        self.inner.content.charge(1, root)?;
        for column in 0..frames.column_count() {
            self.inner.content.step(root)?;
            let selected = trial.and_then(|t| t.candidate().columns().get(usize::from(column)));
            let mut used = Length::ZERO;
            let mut preceding = None;
            let mut partial = None;
            if let Some(selected) = selected {
                used = selected.used_height();
                for part in selected.parts() {
                    self.inner.content.step(root)?;
                    if let Some(items) = part.items() {
                        if items.start != point.item || point.continuation.is_some() {
                            return Err(error(root, E::ReceiptMismatch));
                        }
                        point.item = items.end;
                        preceding = Some(self.inner.content.flow.body_items()[items.end - 1].after);
                    } else if let Some(table) = part.table() {
                        let cursor = table.after();
                        let context = self.inner.tables.as_ref().unwrap();
                        if table.before().table_index() != point.table {
                            return Err(error(root, E::ReceiptMismatch));
                        }
                        preceding = Some(context.table(point.table).space_after());
                        if cursor.is_terminal() {
                            point.item = context.range(point.table).unwrap().end;
                            point.table = context.successor(point.table);
                            point.continuation = None;
                        } else {
                            point.continuation = Some(cursor);
                            partial = Some(part);
                        }
                    } else {
                        return Err(error(root, E::ReceiptMismatch));
                    }
                }
            }
            if point.item == endpoint.item
                && point.table == endpoint.next_table
                && point.continuation.is_none()
            {
                break;
            }
            let width = frames
                .column(column)
                .map_err(|_| error(root, E::ReceiptMismatch))?
                .width();
            let rejection = if let Some(part) = partial {
                self.partial_table_rejection(
                    column,
                    part,
                    target,
                    frames.body().height(),
                    width,
                    frames.named_page_index(),
                )?
            } else {
                self.next_mixed_group_rejection(
                    column,
                    point,
                    used,
                    preceding,
                    target,
                    frames.body().height(),
                    width,
                    frames.named_page_index(),
                    None,
                )?
            };
            if let Some(rejection) = rejection {
                rejections
                    .try_reserve(1)
                    .map_err(|_| error(rejection.owner, E::AllocationFailure))?;
                rejections.push(rejection);
            }
        }
        if rejections.is_empty() {
            return Err(error(root, E::ColumnBalanceContradiction));
        }
        Ok(rejections)
    }
}
