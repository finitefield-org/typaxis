//! Iteratively rebuild a bounded set of independent line graphs from one source.
use super::*;

pub struct BookV2RebuiltBodyLineVariants<'s, 'p, 'a> {
    variants: Vec<BookV2RebuiltBodyLineVariant<'s, 'p, 'a>>,
    records: u64,
    work: u64,
    fingerprint: [u8; 32],
}
impl<'s, 'p, 'a> BookV2RebuiltBodyLineVariants<'s, 'p, 'a> {
    pub fn variants(&self) -> &[BookV2RebuiltBodyLineVariant<'s, 'p, 'a>] {
        &self.variants
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

/// All graphs live in one callback. Build each ownership layer in a separate
/// preallocated vector, without recursively nesting one callback per variant.
/// Source/policy/resource/native identity and the full set budget are checked
/// before allocating any replay storage. The result grants no paint permission.
pub fn with_rebuilt_book_v2_body_line_variants<R>(
    seeds: &[&BookV2BodyLineVariantSeed<'_>],
    maximum_work: u64,
    prior_records: u64,
    use_variants: impl FnOnce(BookV2RebuiltBodyLineVariants<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    let mut allowance = BookV2LineVariantBudget::new(maximum_work, 0);
    with_budgeted_rebuilt_book_v2_body_line_variants(
        seeds,
        &mut allowance,
        prior_records,
        use_variants,
    )
}

/// Keep the full accepted set traversal and every partial graph's charged work
/// on failure. A returned set reports this invocation's work only.
pub fn with_budgeted_rebuilt_book_v2_body_line_variants<R>(
    seeds: &[&BookV2BodyLineVariantSeed<'_>],
    allowance: &mut BookV2LineVariantBudget,
    prior_records: u64,
    use_variants: impl FnOnce(BookV2RebuiltBodyLineVariants<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    allowance.records = allowance.records.max(prior_records);
    let maximum_work = allowance.remaining_work();
    let mut work = 0u64;
    let result = (|| {
        use ProductionInlinePreparationErrorKind as E;
        let root = NodeId::new(0);
        let first = seeds
            .first()
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let mut captured = 0u64;
        let mut base = 0u64;
        let mut graphs = 0u64;
        let mut views = 0u64;
        let mut fingerprint = sha256(b"typaxis.book-2-line-variant-set/1");
        take_work(&mut work, 1, maximum_work)?;
        for (index, seed) in seeds.iter().enumerate() {
            take_work(&mut work, 9, maximum_work)?;
            let same_native = match (first.native, seed.native) {
                (None, None) => true,
                (Some(a), Some(b)) => std::ptr::eq(a, b),
                _ => false,
            };
            if !std::ptr::eq(first.flow, seed.flow)
                || !std::ptr::eq(first.policy, seed.policy)
                || !std::ptr::eq(first.admitted, seed.admitted)
                || !std::ptr::eq(first.bindings, seed.bindings)
                || first.limits.fingerprint() != seed.limits.fingerprint()
                || first.japanese_mode != seed.japanese_mode
                || !same_native
            {
                return Err(error(root, E::ReceiptMismatch).into());
            }
            allowance.records = allowance.records.max(seed.record_charge());
            base = base.max(seed.records - seed.captured_records);
            captured = captured
                .checked_add(seed.captured_records)
                .ok_or_else(|| error(root, E::UnitLimit))?;
            graphs = graphs
                .checked_add(seed.rebuild_records)
                .ok_or_else(|| error(root, E::UnitLimit))?;
            views = views
                .checked_add(seed.contexts.paragraphs().len() as u64)
                .ok_or_else(|| error(root, E::UnitLimit))?;
            let mut digest = [0u8; 72];
            digest[..32].copy_from_slice(&fingerprint);
            digest[32..64].copy_from_slice(&seed.fingerprint());
            digest[64..].copy_from_slice(&(index as u64).to_be_bytes());
            fingerprint = sha256(&digest);
        }
        // Independent seeds may have overlapping prior charges. Count every owned
        // context once above the largest prior base; preserve a larger caller ledger.
        let mut records = base
            .checked_add(captured)
            .map(|n| n.max(allowance.records))
            .and_then(|n| n.checked_add(graphs))
            .and_then(|n| n.checked_add(views))
            .and_then(|n| {
                (seeds.len() as u64)
                    .checked_mul(5)
                    .and_then(|v| n.checked_add(v))
            })
            .and_then(|n| n.checked_add(1))
            .filter(|n| *n <= first.limits.base().get().max_fragments)
            .ok_or_else(|| error(root, E::UnitLimit))?;
        allowance.records = allowance.records.max(records);
        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(seeds.len())
            .map_err(|_| BreakError::AllocationFailure)?;
        for seed in seeds {
            take_work(
                &mut work,
                seed.contexts.paragraphs().len() as u64,
                maximum_work,
            )?;
            let mut input = Vec::new();
            input
                .try_reserve_exact(seed.contexts.paragraphs().len())
                .map_err(|_| BreakError::AllocationFailure)?;
            input.extend(seed.contexts.paragraphs().iter().map(|p| {
                ProductionParagraphLineContext {
                    owner: p.owner(),
                    ends: p.ends(),
                }
            }));
            inputs.push(input);
        }
        let mut shapes = Vec::new();
        shapes
            .try_reserve_exact(seeds.len())
            .map_err(|_| BreakError::AllocationFailure)?;
        let mut source_budget = BookV2SourceVerificationBudget::new_with_prepaid_records(
            allowance.records,
            first.limits.base().get().max_fragments,
            graphs,
        );
        let mut local_records = 0;
        for (seed, inputs) in seeds.iter().zip(&inputs) {
            take_work(&mut work, 1, maximum_work)?;
            let shape = shape_book_v2_authored_text_with_source_budget_counted(
                seed.policy,
                seed.flow,
                seed.admitted,
                seed.limits,
                seed.bindings.epoch(),
                Some(inputs),
                &mut source_budget,
                &mut local_records,
            );
            allowance.records = source_budget.record_charge();
            shapes.push(shape?);
        }
        let mut prepared = Vec::new();
        prepared
            .try_reserve_exact(seeds.len())
            .map_err(|_| BreakError::AllocationFailure)?;
        for (seed, shape) in seeds.iter().zip(&shapes) {
            take_work(&mut work, 1, maximum_work)?;
            let inlines = prepare_book_v2_inline_items_with_source_budget_counted(
                seed.flow,
                shape,
                seed.admitted,
                seed.bindings,
                seed.limits,
                seed.japanese_mode,
                seed.native,
                &mut source_budget,
                &mut local_records,
            );
            allowance.records = source_budget.record_charge();
            prepared.push(inlines?);
        }
        records = allowance.records;
        let mut selected = Vec::new();
        selected
            .try_reserve_exact(seeds.len())
            .map_err(|_| BreakError::AllocationFailure)?;
        for (seed, prepared) in seeds.iter().zip(&prepared) {
            let mut consumed = 0;
            let lines = super::super::frames::layout_body_lines_in_measured_frames(
                prepared,
                seed.body,
                maximum_work - work,
                seed.measured_frames,
                seed.source_widths,
                &mut consumed,
                &mut 0,
            );
            take_work(&mut work, consumed, maximum_work)?;
            let lines = lines?;
            take_work(&mut work, 1, maximum_work)?;
            if lines.fingerprint() != seed.fingerprint() {
                return Err(error(root, E::ReceiptMismatch).into());
            }
            selected.push(lines);
        }
        let mut variants = Vec::new();
        variants
            .try_reserve_exact(seeds.len())
            .map_err(|_| BreakError::AllocationFailure)?;
        for (seed, lines) in seeds.iter().zip(&selected) {
            take_work(&mut work, 1, maximum_work)?;
            let footnotes = prepare_book_v2_footnote_lines(lines, seed.limits)?;
            if footnotes.record_charge() > seed.rebuild_records {
                return Err(error(root, E::ReceiptMismatch).into());
            }
            let variant_work = (seed.contexts.paragraphs().len() as u64)
                .checked_add(lines.candidate_steps())
                .and_then(|n| n.checked_add(1))
                .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
            variants.push(BookV2RebuiltBodyLineVariant {
                lines,
                footnotes,
                records,
                work: variant_work,
            });
        }
        Ok(use_variants(BookV2RebuiltBodyLineVariants {
            variants,
            records,
            work,
            fingerprint,
        }))
    })();
    allowance.work += work;
    result
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ObservedOccurrencePhase {
    Fresh,
    Shaped,
    Inlined,
    Selected,
    Footnoted,
    Sealed,
}
struct ObservedOccurrenceSlot<'id> {
    ordinal: usize,
    phase: ObservedOccurrencePhase,
    request: Option<BookV2ObservedFreshGraphRequest<'id>>,
    payment: Option<BookV2ObservedGraphPayment<'id>>,
    key: Option<BookV2ObservedOccurrenceKey<'id>>,
    initially_paid: u64,
}
impl<'id> ObservedOccurrenceSlot<'id> {
    fn require(
        &self,
        ordinal: usize,
        phase: ObservedOccurrencePhase,
        key: BookV2ObservedOccurrenceKey<'id>,
    ) -> Result<(), BookV2ObservedBodyReshapeError> {
        if self.ordinal != ordinal || self.phase != phase || self.key != Some(key) {
            return Err(BookV2ObservedAccountingError::OccurrenceState {
                owner: NodeId::new(0),
            }
            .into());
        }
        Ok(())
    }
}
struct ObservedPreparedOccurrence<'id, 'a> {
    prepared: BookV2PreparedInlines<'a>,
    key: BookV2ObservedOccurrenceKey<'id>,
    ordinal: usize,
}
struct ObservedSelectedOccurrence<'id, 'p, 'a> {
    lines: BookV2InlineLineLayout<'p, 'a>,
    key: BookV2ObservedOccurrenceKey<'id>,
    ordinal: usize,
}
struct ObservedVariantData<'id, 's, 'p, 'a> {
    lines: &'s BookV2InlineLineLayout<'p, 'a>,
    footnotes: BookV2FootnoteLines<'s, 'p, 'a>,
    key: BookV2ObservedOccurrenceKey<'id>,
    ordinal: usize,
    paid_records: u64,
}

/// Immutable view of one actually rebuilt Column occurrence.
pub struct BookV2ObservedRebuiltColumnLineVariant<'v, 'id, 's, 'p, 'a> {
    inner: &'v ObservedVariantData<'id, 's, 'p, 'a>,
}
impl<'v, 'id, 's, 'p, 'a> BookV2ObservedRebuiltColumnLineVariant<'v, 'id, 's, 'p, 'a> {
    pub fn lines(&self) -> &'s BookV2InlineLineLayout<'p, 'a> {
        self.inner.lines
    }
    pub fn footnotes(&self) -> &BookV2FootnoteLines<'s, 'p, 'a> {
        &self.inner.footnotes
    }
    pub fn occurrence_ordinal(&self) -> usize {
        self.inner.ordinal
    }
    pub fn occurrence_key(&self) -> BookV2ObservedOccurrenceKey<'id> {
        self.inner.key
    }
    pub fn paid_records(&self) -> u64 {
        self.inner.paid_records
    }
    pub fn column_plan(&self) -> &'p typaxis_syntax::book_v2::BookV2ColumnFramePlan<'a> {
        self.inner
            .lines
            .frames()
            .expect("observed Column frames")
            .column_plan()
            .expect("observed Column plan")
    }
}
/// All real graph ownership layers remain simultaneously live in this scope.
pub struct BookV2ObservedRebuiltColumnLineVariants<'id, 's, 'p, 'a> {
    variants: Vec<ObservedVariantData<'id, 's, 'p, 'a>>,
    counters: [u64; 3],
    fingerprint: [u8; 32],
}
impl<'id, 's, 'p, 'a> BookV2ObservedRebuiltColumnLineVariants<'id, 's, 'p, 'a> {
    pub fn variants(
        &self,
    ) -> impl ExactSizeIterator<Item = BookV2ObservedRebuiltColumnLineVariant<'_, 'id, 's, 'p, 'a>>
    {
        self.variants
            .iter()
            .map(|inner| BookV2ObservedRebuiltColumnLineVariant { inner })
    }
    pub fn variant(
        &self,
        index: usize,
    ) -> Option<BookV2ObservedRebuiltColumnLineVariant<'_, 'id, 's, 'p, 'a>> {
        self.variants
            .get(index)
            .map(|inner| BookV2ObservedRebuiltColumnLineVariant { inner })
    }
    pub fn counters(&self) -> [u64; 3] {
        self.counters
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

fn checked_sum(a: u64, b: u64) -> Result<u64, BookV2ObservedBodyReshapeError> {
    a.checked_add(b).ok_or_else(|| {
        BookV2ObservedAccountingError::Records {
            owner: NodeId::new(0),
        }
        .into()
    })
}
fn replay_growth(
    budget: &mut BookV2ObservedLineVariantBudget,
    before: u64,
    after: u64,
) -> Result<(), BookV2ObservedBodyReshapeError> {
    budget.replay_records = checked_sum(budget.replay_records, after.saturating_sub(before))?;
    Ok(())
}
pub(super) struct ObservedFamilyPreflight {
    views: u64,
    fingerprint: [u8; 32],
}

pub(super) fn preflight_observed_variants<'id>(
    seeds: &[&BookV2ObservedColumnLineVariantSeed<'id, '_>],
    budget: &mut BookV2ObservedLineVariantBudget,
    accounting: &mut BookV2ObservedSourceAccounting<'id, '_>,
) -> Result<ObservedFamilyPreflight, BookV2ObservedBodyReshapeError> {
    budget.begin(
        BookV2ObservedVariantPhase::ProtocolStorage,
        None,
        accounting,
        0,
        None,
        None,
    );
    let first = &seeds.first().ok_or_else(observed_variant_mismatch)?.inner;
    let count = observed_units(seeds.len())?;
    budget.request(
        accounting,
        [
            0,
            0,
            checked_sum(count.checked_mul(9).ok_or(BreakError::IterationLimit)?, 1)?,
        ],
    )?;
    let mut views = 0u64;
    let mut fingerprint = sha256(b"typaxis.book-2-line-variant-set/1");
    for (index, column_seed) in seeds.iter().enumerate() {
        let seed = &column_seed.inner;
        let native_same = match (first.native, seed.native) {
            (None, None) => true,
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            _ => false,
        };
        if !std::ptr::eq(first.flow, seed.flow)
            || !std::ptr::eq(first.policy, seed.policy)
            || !std::ptr::eq(first.admitted, seed.admitted)
            || !std::ptr::eq(first.bindings, seed.bindings)
            || !std::ptr::eq(first.columns, seed.columns)
            || first.limits != seed.limits
            || first.mode != seed.mode
            || !native_same
        {
            return Err(observed_variant_mismatch());
        }
        accounting.require_limit_domain(seed.limits.base())?;
        views = checked_sum(views, observed_units(seed.contexts.paragraphs().len())?)?;
        let mut digest = [0u8; 72];
        digest[..32].copy_from_slice(&fingerprint);
        digest[32..64].copy_from_slice(&seed.contexts.source_fingerprint());
        digest[64..].copy_from_slice(&(index as u64).to_be_bytes());
        fingerprint = sha256(&digest);
    }
    Ok(ObservedFamilyPreflight { views, fingerprint })
}

fn observed_protocol_storage_request<'id>(
    occurrences: usize,
    views: u64,
) -> Result<[u64; 3], BookV2ObservedBodyReshapeError> {
    let root = NodeId::new(0);
    let count = observed_units(occurrences)?;
    let records = checked_sum(
        checked_sum(
            count.checked_mul(7).ok_or(BreakError::IterationLimit)?,
            views,
        )?,
        1,
    )?;
    let mut bytes = 0u64;
    for value in [
        observed_bytes::<u64>(occurrences)?,
        observed_bytes::<ObservedOccurrenceSlot<'id>>(occurrences)?,
        observed_bytes::<Vec<ProductionParagraphLineContext<'_>>>(occurrences)?,
        observed_bytes::<BookV2ObservedShapedOccurrence<'id, '_>>(occurrences)?,
        observed_bytes::<ObservedPreparedOccurrence<'id, '_>>(occurrences)?,
        observed_bytes::<ObservedSelectedOccurrence<'id, '_, '_>>(occurrences)?,
        observed_bytes::<ObservedVariantData<'id, '_, '_, '_>>(occurrences)?,
        views
            .checked_mul(std::mem::size_of::<ProductionParagraphLineContext<'_>>() as u64)
            .ok_or(BookV2ObservedAccountingError::Spool { owner: root })?,
    ] {
        bytes = bytes
            .checked_add(value)
            .ok_or(BookV2ObservedAccountingError::Spool { owner: root })?;
    }
    Ok([
        records,
        bytes,
        checked_sum(
            count.checked_mul(7).ok_or(BreakError::IterationLimit)?,
            views,
        )?,
    ])
}

pub(super) fn with_observed_variants<'id, 'cells, R>(
    seeds: &[&ObservedVariantSeed<'id, '_>],
    family: ObservedFamilyPreflight,
    budget: &mut BookV2ObservedLineVariantBudget,
    accounting: &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    inspect: impl FnOnce(
        BookV2ObservedRebuiltColumnLineVariants<'id, '_, '_, '_>,
        &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    ) -> R,
) -> Result<R, BookV2ObservedBodyReshapeError> {
    let root = NodeId::new(0);
    let ObservedFamilyPreflight { views, fingerprint } = family;
    let storage_request = observed_protocol_storage_request(seeds.len(), views)?;
    budget.request(accounting, storage_request)?;
    let mut bounds = Vec::new();
    reserve_observed_protocol(
        &mut bounds,
        seeds.len(),
        ObservedProtocolReserveSite::ReplayBounds,
    )?;
    bounds.extend(seeds.iter().map(|s| s.rebuild_records));
    let mut slots = Vec::new();
    reserve_observed_protocol(
        &mut slots,
        seeds.len(),
        ObservedProtocolReserveSite::ReplaySlots,
    )?;
    for (ordinal, bound) in bounds.iter().copied().enumerate() {
        budget.begin(
            BookV2ObservedVariantPhase::ProtocolStorage,
            Some(ordinal),
            accounting,
            0,
            None,
            None,
        );
        if budget.remaining_work() == 0 {
            return Err(BookV2ObservedAccountingError::Work { owner: root }.into());
        }
        let request = accounting.reserve_graph_request(root, bound);
        budget.complete(
            accounting,
            if request.is_ok() { bound } else { 0 },
            0,
            false,
            request.is_ok(),
            request.is_err(),
        );
        let request = request?;
        budget.work += 1;
        budget.replay_records = checked_sum(budget.replay_records, bound)?;
        slots.push(ObservedOccurrenceSlot {
            ordinal,
            phase: ObservedOccurrencePhase::Fresh,
            request: Some(request),
            payment: None,
            key: None,
            initially_paid: bound,
        });
    }
    let mut inputs = Vec::new();
    reserve_observed_protocol(
        &mut inputs,
        seeds.len(),
        ObservedProtocolReserveSite::ReplayInputsOuter,
    )?;
    for seed in seeds {
        let mut row = Vec::new();
        reserve_observed_protocol(
            &mut row,
            seed.contexts.paragraphs().len(),
            ObservedProtocolReserveSite::ReplayInputViews,
        )?;
        row.extend(
            seed.contexts
                .paragraphs()
                .iter()
                .map(|p| ProductionParagraphLineContext {
                    owner: p.owner(),
                    ends: p.ends(),
                }),
        );
        inputs.push(row);
    }
    budget.complete(accounting, 0, storage_request[0], true, true, false);
    let mut shapes = Vec::new();
    reserve_observed_protocol(
        &mut shapes,
        seeds.len(),
        ObservedProtocolReserveSite::ReplayShapes,
    )?;
    for (ordinal, (seed, input)) in seeds.iter().zip(&inputs).enumerate() {
        let slot = &mut slots[ordinal];
        if slot.ordinal != ordinal || slot.phase != ObservedOccurrencePhase::Fresh {
            return Err(observed_variant_mismatch());
        }
        budget.request(accounting, [0, 0, 1])?;
        let before = slot.initially_paid;
        budget.begin(
            BookV2ObservedVariantPhase::Shape,
            Some(ordinal),
            accounting,
            before,
            None,
            None,
        );
        let mut local = 0;
        let result =
            typaxis_shaping::book_v2::shape_book_v2_authored_text_for_observed_request_counted(
                slot.request.take().ok_or_else(observed_variant_mismatch)?,
                seed.policy,
                seed.flow,
                seed.admitted,
                seed.limits,
                seed.bindings.epoch(),
                Some(input),
                accounting,
                &mut local,
            );
        let entered = super::super::observed_shape_kernel_entry_confirmed(&result, local);
        let after = before.max(local);
        replay_growth(budget, before, after)?;
        budget.complete(
            accounting,
            after,
            local,
            entered,
            result.is_ok(),
            matches!(result, Err(BookV2ObservedShapeError::Accounting(_))),
        );
        let (shape, payment) = result?;
        slot.key = Some(payment.occurrence_key());
        slot.payment = Some(payment);
        slot.phase = ObservedOccurrencePhase::Shaped;
        shapes.push(shape);
    }
    let mut prepared = Vec::new();
    reserve_observed_protocol(
        &mut prepared,
        seeds.len(),
        ObservedProtocolReserveSite::ReplayInlines,
    )?;
    for (ordinal, (seed, shape)) in seeds.iter().zip(&shapes).enumerate() {
        let slot = &mut slots[ordinal];
        slot.require(
            ordinal,
            ObservedOccurrencePhase::Shaped,
            shape.occurrence_key(),
        )?;
        budget.request(accounting, [0, 0, 1])?;
        let payment = slot
            .payment
            .as_mut()
            .ok_or_else(observed_variant_mismatch)?;
        let before = payment.paid_records();
        budget.begin(
            BookV2ObservedVariantPhase::Inline,
            Some(ordinal),
            accounting,
            before,
            None,
            None,
        );
        let mut local = 0;
        let mut entered = false;
        let result = prepare_observed_occurrence_inlines_counted(
            seed.flow,
            shape,
            seed.admitted,
            seed.bindings,
            seed.limits,
            seed.mode,
            seed.native,
            accounting,
            payment,
            &mut local,
            &mut entered,
        );
        let after = payment.paid_records();
        replay_growth(budget, before, after)?;
        budget.complete(
            accounting,
            after,
            local,
            entered,
            result.is_ok(),
            matches!(result, Err(BookV2ObservedInlineError::Accounting(_))),
        );
        let value = result?;
        prepared.push(ObservedPreparedOccurrence {
            prepared: value,
            key: shape.occurrence_key(),
            ordinal,
        });
        slot.phase = ObservedOccurrencePhase::Inlined;
    }
    let mut selected = Vec::new();
    reserve_observed_protocol(
        &mut selected,
        seeds.len(),
        ObservedProtocolReserveSite::ReplaySelected,
    )?;
    for (ordinal, (seed, p)) in seeds.iter().zip(&prepared).enumerate() {
        let slot = &mut slots[ordinal];
        slot.require(ordinal, ObservedOccurrencePhase::Inlined, p.key)?;
        if p.ordinal != ordinal {
            return Err(observed_variant_mismatch());
        }
        let payment = slot
            .payment
            .as_mut()
            .ok_or_else(observed_variant_mismatch)?;
        let cap = payment
            .output_record_limit(accounting)?
            .min(p.prepared.output_record_limit);
        let before = payment.paid_records();
        budget.begin(
            BookV2ObservedVariantPhase::Frames,
            Some(ordinal),
            accounting,
            before,
            Some(p.prepared.output_record_limit),
            Some(cap),
        );
        let (mut work, mut local) = (0, 0);
        let result = super::super::frames::layout_body_lines_in_measured_frames_with_record_limit(
            &p.prepared,
            seed.columns.measurement_body(),
            budget.remaining_work().min(accounting.remaining_work()),
            Some(MeasurementPageFrames::Columns(seed.columns)),
            seed.widths,
            &mut work,
            &mut local,
            cap,
            BookV2NativeRecordOrigin::AlreadyPaid,
        );
        let spent = budget.request(accounting, [0, 0, work]);
        let retained = payment.retain_actual_output(accounting, root, local);
        replay_growth(budget, before, payment.paid_records())?;
        budget.complete(
            accounting,
            payment.paid_records(),
            local,
            true,
            result.is_ok() && spent.is_ok() && retained.is_ok(),
            result.is_ok() && (retained.is_err() || spent.is_err()),
        );
        let lines = result?;
        spent?;
        retained?;
        budget.request(accounting, [0, 0, 1])?;
        if lines.fingerprint() != seed.contexts.source_fingerprint() {
            return Err(observed_variant_mismatch());
        }
        selected.push(ObservedSelectedOccurrence {
            lines,
            key: p.key,
            ordinal,
        });
        slot.phase = ObservedOccurrencePhase::Selected;
    }
    let mut variants = Vec::new();
    reserve_observed_protocol(
        &mut variants,
        seeds.len(),
        ObservedProtocolReserveSite::ReplayVariants,
    )?;
    for (ordinal, (seed, l)) in seeds.iter().zip(&selected).enumerate() {
        let slot = &mut slots[ordinal];
        slot.require(ordinal, ObservedOccurrencePhase::Selected, l.key)?;
        if l.ordinal != ordinal {
            return Err(observed_variant_mismatch());
        }
        budget.request(accounting, [0, 0, 1])?;
        let payment = slot
            .payment
            .as_mut()
            .ok_or_else(observed_variant_mismatch)?;
        let cap = payment
            .output_record_limit(accounting)?
            .min(l.lines.prepared.output_record_limit);
        let before = payment.paid_records();
        budget.begin(
            BookV2ObservedVariantPhase::Footnotes,
            Some(ordinal),
            accounting,
            before,
            Some(l.lines.prepared.output_record_limit),
            Some(cap),
        );
        let mut local = 0;
        let result =
            super::super::footnote_lines::prepare_book_v2_footnote_lines_with_record_limit_counted(
                &l.lines,
                seed.limits,
                &mut local,
                cap,
            );
        let retained = payment.retain_actual_output(accounting, root, local);
        replay_growth(budget, before, payment.paid_records())?;
        budget.complete(
            accounting,
            payment.paid_records(),
            local,
            true,
            result.is_ok() && retained.is_ok(),
            result.is_ok() && retained.is_err(),
        );
        let footnotes = result?;
        retained?;
        if footnotes.record_charge() > seed.rebuild_records {
            return Err(observed_variant_mismatch());
        }
        variants.push(ObservedVariantData {
            lines: &l.lines,
            footnotes,
            key: l.key,
            ordinal,
            paid_records: payment.paid_records(),
        });
        slot.phase = ObservedOccurrencePhase::Footnoted;
    }
    for slot in &mut slots {
        slot.payment
            .as_mut()
            .ok_or_else(observed_variant_mismatch)?
            .seal();
        slot.phase = ObservedOccurrencePhase::Sealed;
    }
    budget.begin(
        BookV2ObservedVariantPhase::Callback,
        None,
        accounting,
        0,
        None,
        None,
    );
    let value = inspect(
        BookV2ObservedRebuiltColumnLineVariants {
            variants,
            counters: accounting.counters(),
            fingerprint,
        },
        accounting,
    );
    budget.complete(accounting, 0, 0, false, true, false);
    Ok(value)
}

#[cfg(test)]
mod observed_p2_protocol_controls {
    use super::*;

    #[test]
    fn observed_p2_protocol_request_arithmetic_boundaries() {
        for (count, views) in [(0usize, 0u64), (1, 0), (3, 5)] {
            let count64 = count as u64;
            let bytes = count64
                * (std::mem::size_of::<u64>()
                    + std::mem::size_of::<ObservedOccurrenceSlot<'_>>()
                    + std::mem::size_of::<Vec<ProductionParagraphLineContext<'_>>>()
                    + std::mem::size_of::<BookV2ObservedShapedOccurrence<'_, '_>>()
                    + std::mem::size_of::<ObservedPreparedOccurrence<'_, '_>>()
                    + std::mem::size_of::<ObservedSelectedOccurrence<'_, '_, '_>>()
                    + std::mem::size_of::<ObservedVariantData<'_, '_, '_, '_>>())
                    as u64
                + views * std::mem::size_of::<ProductionParagraphLineContext<'_>>() as u64;
            assert_eq!(
                observed_protocol_storage_request(count, views).unwrap(),
                [7 * count64 + views + 1, bytes, 7 * count64 + views]
            );
        }
        assert!(matches!(
            checked_sum(u64::MAX, 1),
            Err(BookV2ObservedBodyReshapeError::Accounting(
                BookV2ObservedAccountingError::Records { .. }
            ))
        ));
        assert!(matches!(
            observed_protocol_storage_request(1, u64::MAX),
            Err(BookV2ObservedBodyReshapeError::Accounting(
                BookV2ObservedAccountingError::Records { .. }
            ))
        ));
        let paragraph_size = std::mem::size_of::<ProductionParagraphLineContext<'_>>() as u64;
        assert!(paragraph_size > 1);
        let overflowing_views = u64::MAX / paragraph_size + 1;
        assert!(matches!(
            observed_protocol_storage_request(1, overflowing_views),
            Err(BookV2ObservedBodyReshapeError::Accounting(
                BookV2ObservedAccountingError::Spool { .. }
            ))
        ));
        #[cfg(target_pointer_width = "64")]
        {
            assert!(matches!(
                observed_protocol_storage_request(usize::MAX, 0),
                Err(BookV2ObservedBodyReshapeError::Feedback(
                    BreakError::IterationLimit
                ))
            ));
            assert!(matches!(
                observed_bytes::<u64>(usize::MAX),
                Err(BookV2ObservedBodyReshapeError::Accounting(
                    BookV2ObservedAccountingError::Spool { .. }
                ))
            ));
        }
        // Pure numeric sizing only: no impossible slice, graph or allocation.
    }
}
