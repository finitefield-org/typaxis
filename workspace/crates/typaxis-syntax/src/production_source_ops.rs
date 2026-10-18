//! Shared source storage operations. Legacy callers retain their original
//! allocation path. Observed callers pay requested storage before each request.
use super::*;
use std::marker::PhantomData;

pub(super) enum SourceOps<'s> {
    Legacy(PhantomData<&'s mut ()>),
    #[cfg(feature = "book-v2-staging")]
    Observed(&'s mut ObservedSourceState),
}
/// Issued only by FlowSource::reserve_overlay_carrier after that named complete
/// logical reservation succeeds. It cannot pay styles, records or other arrays.
pub(super) struct OverlayCarrierPermit {
    count: u64,
}
impl OverlayCarrierPermit {
    pub(super) fn after_named_reservation(count: u64) -> Self {
        Self { count }
    }
}

impl SourceOps<'_> {
    pub(super) fn legacy() -> Self {
        Self::Legacy(PhantomData)
    }
    pub(super) fn step(&mut self, owner: NodeId, work: u64) -> Result<(), ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            return state.charge(owner, 0, 0, work);
        }
        let _ = (owner, work);
        Ok(())
    }
    pub(super) fn grow<T>(
        &mut self,
        owner: NodeId,
        values: &mut Vec<T>,
        additional: usize,
        exact: bool,
    ) -> Result<(), ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            let desired = values
                .len()
                .checked_add(additional)
                .ok_or_else(|| state.allocation_error(owner))?;
            if desired > values.capacity() {
                // Explicit amortized requests for append-only carriers. Pay
                // the full new requested extent, including spare slots. The
                // allocator may return a larger capacity; we never assume it.
                let requested = if exact {
                    desired
                } else {
                    desired
                        .max(
                            values
                                .capacity()
                                .checked_mul(2)
                                .ok_or_else(|| state.allocation_error(owner))?,
                        )
                        .max(4)
                };
                let bytes = requested
                    .checked_mul(std::mem::size_of::<T>())
                    .and_then(|n| u64::try_from(n).ok())
                    .ok_or_else(|| state.allocation_error(owner))?;
                state.charge(owner, 0, bytes, 1)?;
                allocation_attempt().map_err(|_| state.allocation_error(owner))?;
                values
                    .try_reserve_exact(requested - values.len())
                    .map_err(|_| state.allocation_error(owner))?;
            }
            return Ok(());
        }
        if exact {
            values.try_reserve_exact(additional)
        } else {
            values.try_reserve(additional)
        }
        .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))
    }
    pub(super) fn string(
        &mut self,
        owner: NodeId,
        value: &str,
    ) -> Result<String, ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            state.charge(owner, 0, value.len() as u64, 1)?;
            allocation_attempt().map_err(|_| state.allocation_error(owner))?;
            let mut text = String::new();
            text.try_reserve_exact(value.len())
                .map_err(|_| state.allocation_error(owner))?;
            state.charge(owner, 0, 0, value.len() as u64)?;
            text.push_str(value);
            return Ok(text);
        }
        Ok(value.to_owned())
    }
    pub(super) fn empty_string(
        &mut self,
        owner: NodeId,
        length: usize,
    ) -> Result<String, ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            state.charge(owner, 0, length as u64, 1)?;
            allocation_attempt().map_err(|_| state.allocation_error(owner))?;
            let mut value = String::new();
            value
                .try_reserve_exact(length)
                .map_err(|_| state.allocation_error(owner))?;
            return Ok(value);
        }
        let mut value = String::new();
        value
            .try_reserve_exact(length)
            .map_err(|_| failure(ProductionFlowErrorKind::AllocationFailure, owner))?;
        Ok(value)
    }
    pub(super) fn decimal(
        &mut self,
        owner: NodeId,
        value: u32,
        dot: bool,
    ) -> Result<String, ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            let mut digits = [0u8; 11];
            let mut cursor = 10;
            let mut n = value;
            loop {
                state.charge(owner, 0, 0, 1)?;
                cursor -= 1;
                digits[cursor] = b'0' + (n % 10) as u8;
                n /= 10;
                if n == 0 {
                    break;
                }
            }
            let end = if dot {
                state.charge(owner, 0, 0, 1)?;
                digits[10] = b'.';
                11
            } else {
                10
            };
            state.charge(owner, 0, 0, (end - cursor) as u64)?;
            let text = std::str::from_utf8(&digits[cursor..end])
                .expect("decimal source labels contain only ASCII");
            return SourceOps::Observed(&mut **state).string(owner, text);
        }
        Ok(if dot {
            format!("{value}.")
        } else {
            value.to_string()
        })
    }
    pub(super) fn sort<T>(
        &mut self,
        owner: NodeId,
        values: &mut [T],
        mut compare: impl FnMut(&T, &T) -> std::cmp::Ordering,
        cost: impl Fn(&T, &T) -> u64,
    ) -> Result<(), ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if matches!(self, Self::Observed(_)) {
            fn sift<T>(
                ops: &mut SourceOps<'_>,
                owner: NodeId,
                v: &mut [T],
                mut root: usize,
                end: usize,
                compare: &mut impl FnMut(&T, &T) -> std::cmp::Ordering,
                cost: &impl Fn(&T, &T) -> u64,
            ) -> Result<(), ProductionFlowError> {
                while let Some(mut child) = root
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(1))
                    .filter(|n| *n < end)
                {
                    if child + 1 < end {
                        ops.step(owner, cost(&v[child], &v[child + 1]).saturating_add(1))?;
                        if compare(&v[child], &v[child + 1]).is_lt() {
                            child += 1;
                        }
                    }
                    ops.step(owner, cost(&v[root], &v[child]).saturating_add(1))?;
                    if !compare(&v[root], &v[child]).is_lt() {
                        return Ok(());
                    }
                    ops.step(owner, 1)?;
                    v.swap(root, child);
                    root = child;
                }
                Ok(())
            }
            for root in (0..values.len() / 2).rev() {
                sift(self, owner, values, root, values.len(), &mut compare, &cost)?;
            }
            for end in (1..values.len()).rev() {
                self.step(owner, 1)?;
                values.swap(0, end);
                sift(self, owner, values, 0, end, &mut compare, &cost)?;
            }
            return Ok(());
        }
        let _ = (owner, cost);
        values.sort_unstable_by(compare);
        Ok(())
    }
    pub(super) fn find<T>(
        &mut self,
        owner: NodeId,
        values: &[T],
        mut compare: impl FnMut(&T) -> std::cmp::Ordering,
        cost: impl Fn(&T) -> u64,
    ) -> Result<Option<usize>, ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if matches!(self, Self::Observed(_)) {
            let (mut start, mut end) = (0, values.len());
            while start < end {
                let index = start + (end - start) / 2;
                self.step(owner, cost(&values[index]).saturating_add(1))?;
                match compare(&values[index]) {
                    std::cmp::Ordering::Less => start = index + 1,
                    std::cmp::Ordering::Greater => end = index,
                    std::cmp::Ordering::Equal => return Ok(Some(index)),
                }
            }
            return Ok(None);
        }
        let _ = (owner, cost);
        Ok(values.binary_search_by(compare).ok())
    }
    pub(super) fn copy_style(
        &mut self,
        owner: NodeId,
        style: &SemanticContainerInheritanceStyle,
    ) -> Result<SemanticContainerInheritanceStyle, ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            return style
                .try_copy_book_v2_counted(&mut |r, s, w| state.charge(owner, r, s, w))
                .map_err(|e| match e {
                    typaxis_style::book_v2::BookV2StyleCopyError::Budget(e) => e,
                    typaxis_style::book_v2::BookV2StyleCopyError::Allocation => {
                        state.allocation_error(owner)
                    }
                });
        }
        Ok(style.clone())
    }
    pub(super) fn descendant(
        &mut self,
        owner: NodeId,
        rules: &StagingSemanticStyleSheets,
        block: &str,
        classes: &[String],
        sheet: &StyleSheet,
        parent: Option<&SemanticContainerInheritanceStyle>,
        basic: bool,
    ) -> Result<SemanticContainerInheritanceStyle, ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            let mut charge = |r, s, w| state.charge(owner, r, s, w);
            let mut context = initial_style::Context::counted(&mut charge);
            return context
                .descendant(rules, block, classes, sheet, parent)
                .map_err(|e| state.style_error(owner, e));
        }
        let result = if basic {
            cascade_staging_semantic_descendant_style(block, classes, sheet, parent)
        } else {
            rules.descendant_style(block, classes, sheet, parent)
        };
        result.map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, owner))
    }
    pub(super) fn page_name(
        &mut self,
        owner: NodeId,
        rules: &StagingSemanticStyleSheets,
        sheet: &StyleSheet,
        block: &str,
        classes: &[String],
        basic: bool,
    ) -> Result<Option<typaxis_core::PageName>, ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            let mut charge = |r, s, w| state.charge(owner, r, s, w);
            let result = if basic {
                typaxis_style::book_v2::cascade_book_v2_basic_page_name_counted(
                    sheet,
                    block,
                    classes,
                    &mut charge,
                )
            } else {
                typaxis_style::book_v2::cascade_book_v2_document_page_name_counted(
                    sheet,
                    block,
                    classes,
                    &mut charge,
                )
            };
            return result.map_err(|e| state.external_style_error(owner, e));
        }
        let result = if basic {
            sheet.cascade_basic_document(block, classes)
        } else {
            rules.cascade_ordinary(block, classes)
        };
        result
            .and_then(|s| s.page_name())
            .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, owner))
    }
    pub(super) fn lower(
        &mut self,
        sheet: &WireStagingStyleSheet,
        limits: &ValidatedResourceLimits,
        book_v2: bool,
    ) -> Result<StagingSemanticStyleSheets, ProductionFlowError> {
        self.lower_at(NodeId::new(0), sheet, limits, book_v2)
    }
    pub(super) fn lower_at(
        &mut self,
        owner: NodeId,
        sheet: &WireStagingStyleSheet,
        limits: &ValidatedResourceLimits,
        book_v2: bool,
    ) -> Result<StagingSemanticStyleSheets, ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            let mut charge = |r, s, w| state.charge(owner, r, s, w);
            return initial_style::lower(
                sheet,
                limits,
                book_v2,
                &mut initial_style::Context::counted(&mut charge),
            )
            .map_err(|e| state.style_error(owner, e));
        }
        lower_semantic_style_rules_version(sheet, limits, book_v2)
            .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, owner))
    }
    pub(super) fn cell(
        &mut self,
        owner: NodeId,
        classes: &[String],
        rules: &StagingSemanticStyleSheets,
        parent: &SemanticContainerInheritanceStyle,
    ) -> Result<SemanticContainerInheritanceStyle, ProductionFlowError> {
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            let mut charge = |r, s, w| state.charge(owner, r, s, w);
            return initial_style::Context::counted(&mut charge)
                .cell(owner, classes, rules, parent)
                .map_err(|e| state.style_error(owner, e));
        }
        inherit_cell_style(owner, classes, rules, parent)
            .map_err(|_| failure(ProductionFlowErrorKind::InvalidStyle, owner))
    }
    pub(super) fn overlay(
        &mut self,
        records: Vec<(typaxis_core::GeneratedBufferKey, String)>,
        limits: &ValidatedResourceLimits,
        retained: u64,
        permit: OverlayCarrierPermit,
    ) -> Result<typaxis_text::GeneratedTextOverlay, ProductionFlowError> {
        let owner = NodeId::new(0);
        // Move/consume exactly this named permit at constructor entry. On a
        // later validation/hash/allocation failure its reservation is retained.
        // A zero-record permit suppresses no positive request.
        if permit.count != records.len() as u64 {
            return Err(failure(ProductionFlowErrorKind::ReceiptMismatch, owner));
        }
        #[cfg(feature = "book-v2-staging")]
        if let Self::Observed(state) = self {
            let mut records = records;
            SourceOps::Observed(&mut **state).sort(
                owner,
                &mut records,
                |a, b| a.0.cmp(&b.0),
                |_, _| 0,
            )?;
            let count = permit.count;
            let mut consumed = false;
            // The unchanged overlay writer formats three bounded integers per
            // record, then scans/escapes its inputs before emitting sink chunks.
            // Pay those preparation units before entering that writer too.
            state.escape_input(owner, typaxis_core::ReferenceFingerprint::ALGORITHM_ID)?;
            for (key, text) in &records {
                state.charge(owner, 0, 0, 3 * 32 + 1)?;
                state.escape_input(
                    owner,
                    typaxis_core::generation_kind_wire_name(key.generation_kind()),
                )?;
                state.escape_input(owner, text)?;
            }
            let generated = typaxis_text::GeneratedTextOverlay::try_from_sorted_records_counted(
                records,
                limits,
                retained,
                &mut |r, s, w| {
                    if r != 0 {
                        if consumed
                            || r != count
                            || s != count.saturating_mul(std::mem::size_of::<
                                typaxis_text::GeneratedTextBuffer,
                            >() as u64)
                            || w != 1
                        {
                            return Err(failure(ProductionFlowErrorKind::ReceiptMismatch, owner));
                        }
                        consumed = true;
                    }
                    // The complete named overlay carrier was reserved immediately
                    // before this call. It cannot pay any style or other carrier.
                    state.charge(owner, 0, s, w)
                },
            )
            .map_err(|e| match e {
                typaxis_text::GeneratedOverlayCountedError::Budget(e) => e,
                typaxis_text::GeneratedOverlayCountedError::Allocation => {
                    state.allocation_error(owner)
                }
                typaxis_text::GeneratedOverlayCountedError::Overlay(_) => {
                    failure(ProductionFlowErrorKind::TextLimit, owner)
                }
            })?;
            if count != 0 && !consumed {
                return Err(failure(ProductionFlowErrorKind::ReceiptMismatch, owner));
            }
            return Ok(generated);
        }
        typaxis_text::GeneratedTextOverlay::new(records, limits, retained)
            .map_err(|_| failure(ProductionFlowErrorKind::TextLimit, owner))
    }
}

#[cfg(feature = "book-v2-staging")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BookV2ObservedSourceFlowObservation {
    records: u64,
    spool: u64,
    work: u64,
    logical_records: u64,
}
#[cfg(feature = "book-v2-staging")]
impl BookV2ObservedSourceFlowObservation {
    pub const fn record_charge(&self) -> u64 {
        self.records
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool
    }
    pub const fn work_steps(&self) -> u64 {
        self.work
    }
    /// This invocation's unchanged intrinsic source reservations. They are a
    /// subset of cumulative accepted records, not a transferable allowance.
    pub const fn logical_source_record_charge(&self) -> u64 {
        self.logical_records
    }
}
#[cfg(feature = "book-v2-staging")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ObservedSourceFlowError {
    Source(ProductionFlowError),
    Records { owner: NodeId },
    Spool { owner: NodeId },
    Work { owner: NodeId },
    Allocation { owner: NodeId },
}
#[cfg(feature = "book-v2-staging")]
impl std::fmt::Display for BookV2ObservedSourceFlowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "observed_source_flow {self:?}")
    }
}
#[cfg(feature = "book-v2-staging")]
impl std::error::Error for BookV2ObservedSourceFlowError {}

#[cfg(feature = "book-v2-staging")]
pub(super) struct ObservedSourceState {
    pub(super) progress: BookV2ObservedSourceFlowObservation,
    pub(super) logical: u64,
    maximum_records: u64,
    maximum_spool: u64,
    maximum_work: u64,
    pub(super) cause: Option<BookV2ObservedSourceFlowError>,
}
#[cfg(feature = "book-v2-staging")]
impl ObservedSourceState {
    /// An input byte can produce at most six JCS escape bytes. This is a
    /// closed preparation-unit bound, not a machine-instruction/time claim.
    pub(super) fn escape_input(
        &mut self,
        owner: NodeId,
        text: &str,
    ) -> Result<(), ProductionFlowError> {
        let work = (text.len() as u64)
            .checked_mul(6)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| self.work_error(owner))?;
        self.charge(owner, 0, 0, work)
    }
    pub(super) fn new(prior: [u64; 3], maximum: [u64; 3]) -> Self {
        Self {
            progress: BookV2ObservedSourceFlowObservation {
                records: prior[0],
                spool: prior[1],
                work: prior[2],
                logical_records: 0,
            },
            logical: 0,
            maximum_records: maximum[0],
            maximum_spool: maximum[1],
            maximum_work: maximum[2],
            cause: None,
        }
    }
    fn capture(
        &mut self,
        error: BookV2ObservedSourceFlowError,
        owner: NodeId,
    ) -> ProductionFlowError {
        if self.cause.is_none() {
            self.cause = Some(error);
        }
        failure(ProductionFlowErrorKind::NodeLimit, owner)
    }
    pub(super) fn charge(
        &mut self,
        owner: NodeId,
        r: u64,
        s: u64,
        w: u64,
    ) -> Result<(), ProductionFlowError> {
        let Some(records) = self
            .progress
            .records
            .checked_add(r)
            .filter(|n| *n <= self.maximum_records)
        else {
            return Err(self.capture(BookV2ObservedSourceFlowError::Records { owner }, owner));
        };
        let Some(spool) = self
            .progress
            .spool
            .checked_add(s)
            .filter(|n| *n <= self.maximum_spool)
        else {
            return Err(self.capture(BookV2ObservedSourceFlowError::Spool { owner }, owner));
        };
        let Some(work) = self
            .progress
            .work
            .checked_add(w)
            .filter(|n| *n <= self.maximum_work)
        else {
            return Err(self.capture(BookV2ObservedSourceFlowError::Work { owner }, owner));
        };
        self.progress = BookV2ObservedSourceFlowObservation {
            records,
            spool,
            work,
            logical_records: self.logical,
        };
        #[cfg(test)]
        CHARGE_TRACE.with(|trace| {
            if let Some(v) = trace.borrow_mut().as_mut() {
                v.push(([r, s, w], [records, spool, work]));
            }
        });
        Ok(())
    }
    pub(super) fn logical(&mut self, owner: NodeId, count: u64) -> Result<(), ProductionFlowError> {
        let logical = self
            .logical
            .checked_add(count)
            .ok_or_else(|| self.capture(BookV2ObservedSourceFlowError::Records { owner }, owner))?;
        self.charge(owner, count, 0, 0)?;
        self.logical = logical;
        self.progress.logical_records = logical;
        Ok(())
    }
    pub(super) fn allocation_error(&mut self, owner: NodeId) -> ProductionFlowError {
        self.capture(BookV2ObservedSourceFlowError::Allocation { owner }, owner)
    }
    pub(super) fn work_error(&mut self, owner: NodeId) -> ProductionFlowError {
        self.capture(BookV2ObservedSourceFlowError::Work { owner }, owner)
    }
    fn style_error(
        &mut self,
        owner: NodeId,
        e: initial_style::BookV2InitialStyleError<ProductionFlowError>,
    ) -> ProductionFlowError {
        match e {
            initial_style::BookV2InitialStyleError::Budget(e) => e,
            initial_style::BookV2InitialStyleError::Allocation => self.allocation_error(owner),
            initial_style::BookV2InitialStyleError::Syntax(_) => {
                failure(ProductionFlowErrorKind::InvalidStyle, owner)
            }
        }
    }
    fn external_style_error(
        &mut self,
        owner: NodeId,
        e: typaxis_style::book_v2::BookV2StylePreparationError<ProductionFlowError>,
    ) -> ProductionFlowError {
        match e {
            typaxis_style::book_v2::BookV2StylePreparationError::Budget(e) => e,
            typaxis_style::book_v2::BookV2StylePreparationError::Allocation => {
                self.allocation_error(owner)
            }
            typaxis_style::book_v2::BookV2StylePreparationError::Style(_) => {
                failure(ProductionFlowErrorKind::InvalidStyle, owner)
            }
        }
    }
    pub(super) fn finish_error(
        &mut self,
        error: ProductionFlowError,
    ) -> BookV2ObservedSourceFlowError {
        self.cause
            .take()
            .unwrap_or(BookV2ObservedSourceFlowError::Source(error))
    }
}

#[cfg(all(test, feature = "book-v2-staging"))]
thread_local! {static ALLOCATION_FAULT:std::cell::Cell<(usize,usize)>=const{std::cell::Cell::new((usize::MAX,0))};}
#[cfg(feature = "book-v2-staging")]
fn allocation_attempt() -> Result<(), ()> {
    #[cfg(test)]
    return ALLOCATION_FAULT.with(|p| {
        let (fail, n) = p.get();
        p.set((fail, n + 1));
        if n == fail {
            Err(())
        } else {
            Ok(())
        }
    });
    #[cfg(not(test))]
    Ok(())
}

#[cfg(all(test, feature = "book-v2-staging"))]
thread_local! {static CHARGE_TRACE:std::cell::RefCell<Option<Vec<([u64;3],[u64;3])>>>=const{std::cell::RefCell::new(None)};}
#[cfg(all(test, feature = "book-v2-staging"))]
pub(super) fn with_allocation_fault<R>(at: usize, run: impl FnOnce() -> R) -> (R, usize) {
    let previous = ALLOCATION_FAULT.with(|p| p.replace((at, 0)));
    struct Reset((usize, usize));
    impl Drop for Reset {
        fn drop(&mut self) {
            ALLOCATION_FAULT.with(|p| p.set(self.0));
        }
    }
    let reset = Reset(previous);
    let result = run();
    let count = ALLOCATION_FAULT.with(|p| p.get().1);
    drop(reset);
    (result, count)
}
#[cfg(all(test, feature = "book-v2-staging"))]
pub(super) fn with_charge_trace<R>(run: impl FnOnce() -> R) -> (R, Vec<([u64; 3], [u64; 3])>) {
    let previous = CHARGE_TRACE.with(|p| p.replace(Some(Vec::new())));
    struct Reset(Option<Vec<([u64; 3], [u64; 3])>>);
    impl Drop for Reset {
        fn drop(&mut self) {
            CHARGE_TRACE.with(|p| {
                p.replace(self.0.take());
            });
        }
    }
    let reset = Reset(previous);
    let result = run();
    let trace = CHARGE_TRACE.with(|p| p.replace(None).unwrap());
    drop(reset);
    (result, trace)
}
