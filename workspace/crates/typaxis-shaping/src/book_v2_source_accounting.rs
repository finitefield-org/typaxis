//! Borrowed observations for the additive original-source line path.
//!
//! This loan spends three existing caller counters. It grants no source,
//! placement or paint authority; source verification always recollects the
//! authentic source through Syntax's observed verifier.
use std::marker::PhantomData;
use typaxis_core::{NodeId, ValidatedResourceLimits};
use typaxis_syntax::book_v2::{
    BookV2ObservedSourceFlowError, BookV2ObservedSourceFlowObservation, PreparedBookV2Navigation,
    PreparedBookV2TextFlow, StyledBookV2Body,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ObservedAccountingError {
    LimitsMismatch,
    ClosedWindow,
    Records { owner: NodeId },
    Spool { owner: NodeId },
    Work { owner: NodeId },
    OccurrenceState { owner: NodeId },
}
impl std::fmt::Display for BookV2ObservedAccountingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 observed accounting: {self:?}")
    }
}
impl std::error::Error for BookV2ObservedAccountingError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ObservedVerificationError {
    Accounting(BookV2ObservedAccountingError),
    Source(BookV2ObservedSourceFlowError),
}
impl std::fmt::Display for BookV2ObservedVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 observed verification: {self:?}")
    }
}
impl std::error::Error for BookV2ObservedVerificationError {}

/// A short loan of the actual caller cells, created only by the scoped factory.
/// The invariant identity prevents replacement with another factory's loan.
pub struct BookV2ObservedSourceAccounting<'id, 'cells> {
    records: &'cells mut u64,
    spool: &'cells mut u64,
    work: &'cells mut u64,
    limits: &'cells ValidatedResourceLimits,
    maximum_records: u64,
    maximum_work: u64,
    next_occurrence: u64,
    identity: PhantomData<fn(&'id mut ()) -> &'id mut ()>,
}

/// Paid intrinsic output in one scoped graph window. There is no constructor
/// for a positive credit, and sealing this window never refunds caller records.
pub struct BookV2ObservedIntrinsicRecords<'id, 'window> {
    paid: u64,
    closed: bool,
    identity: PhantomData<fn(&'id mut ()) -> &'id mut ()>,
    window: PhantomData<fn(&'window mut ()) -> &'window mut ()>,
}

/// Borrow counters without reordering the stage's existing identity checks.
/// The spool ceiling is exactly the supplied authentic source limit domain;
/// `verify_source` rejects a different domain before recollection begins.
///
/// ```
/// use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
/// use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;
/// let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
/// let (mut r, mut s, mut w) = (3, 5, 7);
/// with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 100, 100,
///     |loan| assert_eq!(loan.counters(), [3, 5, 7]));
/// assert_eq!((r, s, w), (3, 5, 7));
/// ```
///
/// A loan cannot escape its fresh identity.
/// ```compile_fail
/// use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
/// use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;
/// let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
/// let (mut r, mut s, mut w) = (0, 0, 0);
/// let escaped = with_book_v2_observed_source_accounting(
///     &mut r, &mut s, &mut w, &limits, 100, 100, |loan| loan);
/// let _ = escaped.counters();
/// ```
///
/// Independent factory loans cannot be swapped.
/// ```compile_fail
/// use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
/// use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;
/// let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
/// let (mut r, mut s, mut w) = (0, 0, 0);
/// let (mut r2, mut s2, mut w2) = (0, 0, 0);
/// with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 100, 100, |a| {
///     with_book_v2_observed_source_accounting(&mut r2, &mut s2, &mut w2, &limits, 100, 100, |b| {
///         std::mem::swap(a, b);
///     });
/// });
/// ```
pub fn with_book_v2_observed_source_accounting<'cells, R>(
    records: &'cells mut u64,
    spool: &'cells mut u64,
    work: &'cells mut u64,
    limits: &'cells ValidatedResourceLimits,
    maximum_records: u64,
    maximum_work: u64,
    inspect: impl for<'id> FnOnce(&mut BookV2ObservedSourceAccounting<'id, 'cells>) -> R,
) -> R {
    let mut loan = BookV2ObservedSourceAccounting {
        records,
        spool,
        work,
        limits,
        maximum_records: maximum_records.min(limits.get().max_fragments),
        maximum_work,
        next_occurrence: 0,
        identity: PhantomData,
    };
    inspect(&mut loan)
}

impl<'id, 'cells> BookV2ObservedSourceAccounting<'id, 'cells> {
    /// Pay for one new retained graph occurrence. The resulting request is
    /// consumed by the concrete shaper; it cannot be reconstructed from an
    /// earlier graph's reported records.
    pub fn reserve_graph_request(
        &mut self,
        owner: NodeId,
        requested_records: u64,
    ) -> Result<BookV2ObservedFreshGraphRequest<'id>, BookV2ObservedAccountingError> {
        let next = self
            .next_occurrence
            .checked_add(1)
            .ok_or(BookV2ObservedAccountingError::OccurrenceState { owner })?;
        self.debit(owner, [requested_records, 0, 1])?;
        self.next_occurrence = next;
        Ok(BookV2ObservedFreshGraphRequest {
            paid: requested_records,
            key: BookV2ObservedOccurrenceKey {
                sequence: next,
                identity: PhantomData,
            },
        })
    }
    /// Prepare one authentic selected Header/Footer on the actual caller cells.
    /// Later region verification is pointer affiliation, not body recollection.
    pub fn prepare_region_source<'src>(
        &mut self,
        selected: typaxis_syntax::book_v2::BookV2SelectedPageMaster<'src>,
        kind: typaxis_syntax::book_v2::BookV2PageRegionKind,
        navigation: &'src PreparedBookV2Navigation<'src>,
    ) -> Result<
        typaxis_syntax::book_v2::BookV2PageRegionTextFlow<'src>,
        BookV2ObservedVerificationError,
    > {
        self.require_limit_domain(selected.source().body().limits())
            .map_err(BookV2ObservedVerificationError::Accounting)?;
        let mut observed = BookV2ObservedSourceFlowObservation::default();
        let result =
            typaxis_syntax::book_v2::prepare_book_v2_page_region_text_flow_observed_counted(
                selected,
                kind,
                navigation,
                *self.records,
                *self.spool,
                *self.work,
                self.maximum_records,
                self.maximum_work,
                &mut observed,
            );
        *self.records = observed.record_charge();
        *self.spool = observed.spool_charge();
        *self.work = observed.work_steps();
        result.map_err(BookV2ObservedVerificationError::Source)
    }
    pub fn counters(&self) -> [u64; 3] {
        [*self.records, *self.spool, *self.work]
    }
    pub fn remaining_work(&self) -> u64 {
        self.maximum_work.saturating_sub(*self.work)
    }
    pub fn maximum_work(&self) -> u64 {
        self.maximum_work
    }
    pub fn maximum_records(&self) -> u64 {
        self.maximum_records
    }
    pub fn maximum_spool(&self) -> u64 {
        self.limits.get().max_spool_bytes
    }
    /// Check the authentic domain without charging or changing a caller cell.
    pub fn require_limit_domain(
        &self,
        authentic: &ValidatedResourceLimits,
    ) -> Result<(), BookV2ObservedAccountingError> {
        if authentic != self.limits {
            return Err(BookV2ObservedAccountingError::LimitsMismatch);
        }
        Ok(())
    }
    /// Accept only work. Other axes may have independently refused a request.
    /// A zero request still validates this axis's existing prior.
    pub fn debit_work(
        &mut self,
        owner: NodeId,
        amount: u64,
    ) -> Result<(), BookV2ObservedAccountingError> {
        let next = self
            .work
            .checked_add(amount)
            .filter(|n| *n <= self.maximum_work)
            .ok_or(BookV2ObservedAccountingError::Work { owner })?;
        *self.work = next;
        Ok(())
    }
    /// Accept only records, without checking or changing spool or work.
    pub fn debit_records(
        &mut self,
        owner: NodeId,
        amount: u64,
    ) -> Result<(), BookV2ObservedAccountingError> {
        let next = self
            .records
            .checked_add(amount)
            .filter(|n| *n <= self.maximum_records)
            .ok_or(BookV2ObservedAccountingError::Records { owner })?;
        *self.records = next;
        Ok(())
    }
    /// Accept only requested spool bytes, without checking the other axes.
    pub fn debit_spool(
        &mut self,
        owner: NodeId,
        amount: u64,
    ) -> Result<(), BookV2ObservedAccountingError> {
        let next = self
            .spool
            .checked_add(amount)
            .filter(|n| *n <= self.maximum_spool())
            .ok_or(BookV2ObservedAccountingError::Spool { owner })?;
        *self.spool = next;
        Ok(())
    }
    /// Prepare the actual original source while this same caller loan is live.
    /// No observation or receipt can be supplied by the caller.
    pub fn prepare_source<'a>(
        &mut self,
        body: &'a StyledBookV2Body,
        navigation: &'a PreparedBookV2Navigation<'a>,
    ) -> Result<PreparedBookV2TextFlow<'a>, BookV2ObservedVerificationError> {
        self.require_limit_domain(body.body().limits())
            .map_err(BookV2ObservedVerificationError::Accounting)?;
        let mut observed = BookV2ObservedSourceFlowObservation::default();
        let result = typaxis_syntax::book_v2::prepare_book_v2_text_flow_observed_counted(
            body,
            navigation,
            *self.records,
            *self.spool,
            *self.work,
            self.maximum_records,
            self.maximum_work,
            &mut observed,
        );
        *self.records = observed.record_charge();
        *self.spool = observed.spool_charge();
        *self.work = observed.work_steps();
        result.map_err(BookV2ObservedVerificationError::Source)
    }
    /// Each successful debit publishes all three positive deltas atomically.
    pub fn debit(
        &mut self,
        owner: NodeId,
        delta: [u64; 3],
    ) -> Result<(), BookV2ObservedAccountingError> {
        use BookV2ObservedAccountingError as E;
        let r = self
            .records
            .checked_add(delta[0])
            .filter(|n| *n <= self.maximum_records)
            .ok_or(E::Records { owner })?;
        let s = self
            .spool
            .checked_add(delta[1])
            .filter(|n| *n <= self.maximum_spool())
            .ok_or(E::Spool { owner })?;
        let w = self
            .work
            .checked_add(delta[2])
            .filter(|n| *n <= self.maximum_work)
            .ok_or(E::Work { owner })?;
        *self.records = r;
        *self.spool = s;
        *self.work = w;
        Ok(())
    }
    /// Full recollection with inclusive caller history. No graph credit enters
    /// Syntax and no source-derived record count replaces the accepted total.
    pub fn verify_source(
        &mut self,
        flow: &PreparedBookV2TextFlow<'_>,
        body: &StyledBookV2Body,
        navigation: &PreparedBookV2Navigation<'_>,
    ) -> Result<(), BookV2ObservedVerificationError> {
        if body.body().limits() != self.limits {
            return Err(BookV2ObservedVerificationError::Accounting(
                BookV2ObservedAccountingError::LimitsMismatch,
            ));
        }
        let mut observed = BookV2ObservedSourceFlowObservation::default();
        let result = flow.verify_for_observed_counted(
            body,
            navigation,
            *self.records,
            *self.spool,
            *self.work,
            self.maximum_records,
            self.maximum_work,
            &mut observed,
        );
        *self.records = observed.record_charge();
        *self.spool = observed.spool_charge();
        *self.work = observed.work_steps();
        result.map_err(BookV2ObservedVerificationError::Source)
    }
    /// Start an empty record window. Both owner and window have private fresh
    /// identities, and the factory retains ownership across normal returns.
    ///
    /// ```
    /// use typaxis_core::{NodeId, ResourceLimits, ValidatedResourceLimits};
    /// use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;
    /// let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    /// let (mut r, mut s, mut w) = (3, 5, 7);
    /// with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 100, 100, |loan| {
    ///     loan.with_zero_intrinsic_records(|loan, window| {
    ///         loan.retain_intrinsic_output(window, NodeId::new(0), 4).unwrap();
    ///         loan.retain_intrinsic_output(window, NodeId::new(0), 3).unwrap();
    ///         assert_eq!(loan.counters(), [7, 5, 7]);
    ///         window.seal();
    ///     });
    /// });
    /// assert_eq!(r, 7);
    /// ```
    ///
    /// A window cannot escape its owning call.
    /// ```compile_fail
    /// use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    /// use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;
    /// let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    /// let (mut r, mut s, mut w) = (0, 0, 0);
    /// with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 100, 100, |loan| {
    ///     let escaped = loan.with_zero_intrinsic_records(|_, window| window);
    ///     escaped.seal();
    /// });
    /// ```
    ///
    /// Independent windows cannot be swapped.
    /// ```compile_fail
    /// use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    /// use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;
    /// let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
    /// let (mut r, mut s, mut w) = (0, 0, 0);
    /// with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 100, 100, |loan| {
    ///     loan.with_zero_intrinsic_records(|loan, a| {
    ///         loan.with_zero_intrinsic_records(|_, b| std::mem::swap(a, b));
    ///     });
    /// });
    /// ```
    pub fn with_zero_intrinsic_records<R>(
        &mut self,
        inspect: impl for<'window> FnOnce(
            &mut Self,
            &mut BookV2ObservedIntrinsicRecords<'id, 'window>,
        ) -> R,
    ) -> R {
        let mut window = BookV2ObservedIntrinsicRecords {
            paid: 0,
            closed: false,
            identity: PhantomData,
            window: PhantomData,
        };
        let result = inspect(self, &mut window);
        window.seal();
        result
    }
    pub fn intrinsic_limit(
        &self,
        window: &BookV2ObservedIntrinsicRecords<'id, '_>,
    ) -> Result<u64, BookV2ObservedAccountingError> {
        if window.closed {
            return Err(BookV2ObservedAccountingError::ClosedWindow);
        }
        self.maximum_records
            .checked_sub(*self.records)
            .and_then(|n| n.checked_add(window.paid))
            .map(|n| n.min(self.limits.get().max_fragments))
            .ok_or(BookV2ObservedAccountingError::Records {
                owner: NodeId::new(0),
            })
    }
    pub fn retain_intrinsic_output(
        &mut self,
        window: &mut BookV2ObservedIntrinsicRecords<'id, '_>,
        owner: NodeId,
        prefix: u64,
    ) -> Result<(), BookV2ObservedAccountingError> {
        if window.closed {
            return Err(BookV2ObservedAccountingError::ClosedWindow);
        }
        if prefix <= window.paid {
            return Ok(());
        }
        self.debit(owner, [prefix - window.paid, 0, 0])?;
        window.paid = window.paid.max(prefix);
        Ok(())
    }
}
impl BookV2ObservedIntrinsicRecords<'_, '_> {
    pub fn paid_records(&self) -> u64 {
        self.paid
    }
    pub fn seal(&mut self) {
        self.closed = true;
    }
}

/// Identity of a freshly paid occurrence. It grants no source or graph authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookV2ObservedOccurrenceKey<'id> {
    sequence: u64,
    identity: PhantomData<fn(&'id mut ()) -> &'id mut ()>,
}

/// One newly accepted graph request. Only the concrete shaper consumes it.
pub struct BookV2ObservedFreshGraphRequest<'id> {
    paid: u64,
    key: BookV2ObservedOccurrenceKey<'id>,
}
impl<'id> BookV2ObservedFreshGraphRequest<'id> {
    pub(super) fn into_payment(self) -> BookV2ObservedGraphPayment<'id> {
        BookV2ObservedGraphPayment {
            paid: self.paid,
            key: self.key,
            closed: false,
        }
    }
}

/// Pure accounting for the graph created by the one-shot request. There is no
/// conversion back to a fresh request or to a positive intrinsic window.
pub struct BookV2ObservedGraphPayment<'id> {
    paid: u64,
    key: BookV2ObservedOccurrenceKey<'id>,
    closed: bool,
}
impl<'id> BookV2ObservedGraphPayment<'id> {
    pub fn occurrence_key(&self) -> BookV2ObservedOccurrenceKey<'id> {
        self.key
    }
    pub fn paid_records(&self) -> u64 {
        self.paid
    }
    pub fn output_record_limit(
        &self,
        accounting: &BookV2ObservedSourceAccounting<'id, '_>,
    ) -> Result<u64, BookV2ObservedAccountingError> {
        if self.closed {
            return Err(BookV2ObservedAccountingError::ClosedWindow);
        }
        accounting
            .maximum_records
            .checked_sub(accounting.counters()[0])
            .and_then(|n| n.checked_add(self.paid))
            .map(|n| n.min(accounting.limits.get().max_fragments))
            .ok_or(BookV2ObservedAccountingError::Records {
                owner: NodeId::new(0),
            })
    }
    pub fn retain_actual_output(
        &mut self,
        accounting: &mut BookV2ObservedSourceAccounting<'id, '_>,
        owner: NodeId,
        actual_local_prefix: u64,
    ) -> Result<(), BookV2ObservedAccountingError> {
        if self.closed {
            return Err(BookV2ObservedAccountingError::ClosedWindow);
        }
        if actual_local_prefix > self.paid {
            accounting.debit(owner, [actual_local_prefix - self.paid, 0, 0])?;
            self.paid = actual_local_prefix;
        }
        Ok(())
    }
    pub fn seal(&mut self) {
        self.closed = true;
    }
}

#[cfg(test)]
mod observed_p2_controls {
    use super::*;
    use typaxis_core::ResourceLimits;

    #[test]
    fn observed_p2_request_sequence_overflow_is_atomic() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let owner = NodeId::new(23);
        let (mut r, mut s, mut w) = (3, 5, 7);
        with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 100, 1000, |a| {
            a.next_occurrence = u64::MAX - 1;
            let request = a.reserve_graph_request(owner, 0).unwrap();
            assert_eq!(request.key.sequence, u64::MAX);
            assert_eq!(a.counters(), [3, 5, 8]);
            for amount in [0, 3] {
                assert!(matches!(a.reserve_graph_request(owner, amount),
                    Err(BookV2ObservedAccountingError::OccurrenceState { owner: o }) if o == owner));
                assert_eq!(a.counters(), [3, 5, 8]);
                assert_eq!(a.next_occurrence, u64::MAX);
            }
        });
        let (mut r, mut s, mut w) = (u64::MAX, u64::MAX, u64::MAX);
        with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 100, 1000, |a| {
            a.next_occurrence = u64::MAX;
            assert!(matches!(a.reserve_graph_request(owner, u64::MAX),
                Err(BookV2ObservedAccountingError::OccurrenceState { owner: o }) if o == owner));
            assert_eq!(a.counters(), [u64::MAX; 3]);
        });
    }

    #[test]
    fn observed_p2_failed_request_does_not_consume_sequence() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let owner = NodeId::new(23);
        let (mut r, mut s, mut w) = (3, 5, 7);
        with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 10, 1000, |a| {
            assert!(matches!(a.reserve_graph_request(owner, 8),
                Err(BookV2ObservedAccountingError::Records { owner: o }) if o == owner));
            assert_eq!(a.counters(), [3, 5, 7]);
            assert_eq!(a.next_occurrence, 0);
            let request = a.reserve_graph_request(owner, 0).unwrap();
            assert_eq!(request.key.sequence, 1);
            assert_eq!(a.counters(), [3, 5, 8]);
        });
        // These are pure numeric negative states, never source/graph witnesses.
        for (prior, expected) in [
            (
                [u64::MAX, 0, 0],
                BookV2ObservedAccountingError::Records { owner },
            ),
            (
                [0, u64::MAX, 0],
                BookV2ObservedAccountingError::Spool { owner },
            ),
            (
                [0, 0, u64::MAX],
                BookV2ObservedAccountingError::Work { owner },
            ),
        ] {
            let [mut r, mut s, mut w] = prior;
            with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                100,
                1000,
                |a| {
                    assert_eq!(a.debit(owner, [1, 1, 1]), Err(expected));
                    assert_eq!(a.counters(), prior);
                    assert_eq!(a.next_occurrence, 0);
                },
            );
        }
    }

    #[test]
    fn observed_p2_payment_own_bound_growth_and_closed_state() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let owner = NodeId::new(23);
        let (mut r, mut s, mut w) = (0, 5, 7);
        with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 40, 1000, |a| {
            // Actual debit-created requests become only pure bookkeeping here.
            // No shape/source/native authority or allocating graph is created.
            let mut first = a.reserve_graph_request(owner, 3).unwrap().into_payment();
            let second = a.reserve_graph_request(owner, 7).unwrap().into_payment();
            assert_ne!(first.occurrence_key(), second.occurrence_key());
            assert_eq!(a.counters(), [10, 5, 9]);
            assert_eq!(first.output_record_limit(a), Ok(33));
            assert_eq!(second.output_record_limit(a), Ok(37));
            first.retain_actual_output(a, owner, 5).unwrap();
            assert_eq!(a.counters(), [12, 5, 9]);
            assert_eq!(first.paid_records(), 5);
            assert_eq!(first.output_record_limit(a), Ok(33));
            first.retain_actual_output(a, owner, 5).unwrap();
            assert_eq!(a.counters(), [12, 5, 9]);
            assert_eq!(
                first.retain_actual_output(a, owner, 100),
                Err(BookV2ObservedAccountingError::Records { owner })
            );
            assert_eq!(first.paid_records(), 5);
            assert_eq!(a.counters(), [12, 5, 9]);
            first.seal();
            assert_eq!(
                first.output_record_limit(a),
                Err(BookV2ObservedAccountingError::ClosedWindow)
            );
            for prefix in [0, 5, 100] {
                assert_eq!(
                    first.retain_actual_output(a, owner, prefix),
                    Err(BookV2ObservedAccountingError::ClosedWindow)
                );
                assert_eq!(first.paid_records(), 5);
                assert_eq!(a.counters(), [12, 5, 9]);
            }
            // A bad actual prior cannot turn the second own-q into a cap.
            *a.records = u64::MAX;
            assert!(matches!(
                second.output_record_limit(a),
                Err(BookV2ObservedAccountingError::Records { .. })
            ));
        });
    }
}
