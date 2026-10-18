//! Command-owned Type2 state, borrowed by one scoped font builder at a time.
use super::*;
use typaxis_core::NodeId;
use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
use typaxis_resource_admission::ProductionResourceAdmissionSessionIdentityV3;

pub(super) enum BookV2FontCffSession<'v> {
    Owned(typaxis_font::Cff1SubsetSessionV2),
    Borrowed(&'v mut typaxis_font::Cff1SubsetSessionV2),
}
impl std::ops::Deref for BookV2FontCffSession<'_> {
    type Target = typaxis_font::Cff1SubsetSessionV2;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(session) => session,
            Self::Borrowed(session) => session,
        }
    }
}
impl std::ops::DerefMut for BookV2FontCffSession<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Owned(session) => session,
            Self::Borrowed(session) => session,
        }
    }
}
impl BookV2FontCffSession<'_> {
    fn is_borrowed_from(&self, original: *const typaxis_font::Cff1SubsetSessionV2) -> bool {
        match self {
            Self::Borrowed(session) => std::ptr::eq(&**session, original),
            Self::Owned(_) => false,
        }
    }
}

/// Persistent CFF evaluator/cache for one command and its failed retries.
///
/// The carrier borrows no display or generated source flow. Its first valid
/// builder loan binds the authoritative admission identity. There is no reset,
/// cloning or access to the mutable evaluator; statistics are observations.
///
/// A complete resource chain stays within one loan; its ordinary leaf receipts
/// retain the same source display, while the command keeps its real evaluator.
///
/// ```
/// use typaxis_core::M4EffectiveResourceLimits;
/// use typaxis_display_list::book_v2::BookV2ResourceDisplay;
/// use typaxis_resources::book_v2::BookV2FontCommandSession;
/// fn resources<'a, D: BookV2ResourceDisplay<'a>>(
///     session: &mut BookV2FontCommandSession, display: &D,
///     limits: &M4EffectiveResourceLimits,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     let (mut r, mut s, mut w) = (0, 0, 0);
///     session.with_builder_counted(
///         display, limits, 0, 0, 0, &mut r, &mut s, &mut w,
///         |builder| -> Result<(), Box<dyn std::error::Error>> {
///             let selection = builder.build()?;
///             let closures = builder.prepare_font_closures(&selection)?;
///             let programs = builder.write_font_programs(&closures)?;
///             let cids = builder.plan_cids(&programs)?;
///             let (mut r, mut s, mut w) = (
///                 builder.record_charge(), builder.spool_charge(), builder.work_steps(),
///             );
///             builder.next_stage_after_counted(r, s, w, &mut r, &mut s, &mut w)?;
///             let images = builder.select_images()?;
///             let rasters = builder.write_raster_programs(&images)?;
///             Ok(())
///         },
///     )??;
///     Ok(())
/// }
/// ```
///
/// ```compile_fail
/// use typaxis_resources::book_v2::BookV2FontCommandSession;
/// fn duplicate(session: &BookV2FontCommandSession) -> BookV2FontCommandSession {
///     session.clone()
/// }
/// ```
///
/// Generated resource selections cannot carry a command loan out of its scope.
///
/// ```compile_fail
/// use typaxis_core::M4EffectiveResourceLimits;
/// use typaxis_display_list::book_v2::BookV2ResourceDisplay;
/// use typaxis_resources::book_v2::BookV2FontCommandSession;
/// fn escape<'a, D: BookV2ResourceDisplay<'a>>(
///     session: &mut BookV2FontCommandSession, display: &D,
///     limits: &M4EffectiveResourceLimits,
/// ) {
///     let (mut r, mut s, mut w) = (0, 0, 0);
///     let escaped = session.with_builder_counted(
///         display, limits, 0, 0, 0, &mut r, &mut s, &mut w,
///         |builder| builder.build(),
///     );
/// }
/// ```
pub struct BookV2FontCommandSession {
    session: typaxis_font::Cff1SubsetSessionV2,
    admission: Option<ProductionResourceAdmissionSessionIdentityV3>,
    limits: [u8; 32],
    maximum_work: u64,
    history: [u64; 3],
}
impl BookV2FontCommandSession {
    pub fn new(limits: &M4EffectiveResourceLimits, maximum_work: u64) -> Self {
        Self {
            session: typaxis_font::Cff1SubsetSessionV2::new(limits),
            admission: None,
            limits: limits.fingerprint(),
            maximum_work,
            history: [0; 3],
        }
    }
    /// Native math and PDF subsetting borrow this same command-owned cache.
    /// The layout implementation alone receives its mutable session; callers
    /// cannot replace it or reset counters through a callback.
    pub fn compute_native_math_observed_counted<'id, 'a, 'cells>(
        &mut self,
        bindings: &'a typaxis_layout::book_v2::BookV2VectorBindings<'a>,
        admitted: &'a AdmittedProductionResourceLedgerV3,
        limits: &M4EffectiveResourceLimits,
        accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
    ) -> Result<
        Option<typaxis_layout::book_v2::BookV2ObservedNativeMath<'id, 'a>>,
        typaxis_layout::book_v2::BookV2ObservedNativeMathError,
    > {
        use typaxis_layout::book_v2::BookV2ObservedNativeMathError as Error;
        let mismatch = || {
            Error::Native(typaxis_layout::ProductionNativeMathComputationError::Math(
                typaxis_layout::StagingMathLayoutError::ReceiptMismatch,
            ))
        };
        if self.limits != limits.fingerprint()
            || admitted.effective_limits().fingerprint() != self.limits
        {
            return Err(mismatch());
        }
        let admission = admitted.session_identity();
        if self
            .admission
            .as_ref()
            .is_some_and(|prior| prior != &admission)
        {
            return Err(mismatch());
        }
        accounting
            .require_limit_domain(limits.base())
            .map_err(Error::Accounting)?;
        if accounting.maximum_work() != self.maximum_work {
            return Err(Error::Accounting(
                typaxis_shaping::book_v2::BookV2ObservedAccountingError::LimitsMismatch,
            ));
        }

        let current = accounting.counters();
        accounting
            .debit(
                NodeId::new(0),
                std::array::from_fn(|i| self.history[i].saturating_sub(current[i])),
            )
            .map_err(Error::Accounting)?;
        if self.admission.is_none() {
            self.admission = Some(admission);
        }
        let history = NativeMathHistory {
            history: &mut self.history,
            accounting,
        };
        typaxis_layout::book_v2::compute_book_v2_native_math_cff_observed_counted(
            bindings,
            admitted,
            limits,
            history.accounting,
            &mut self.session,
        )
    }
    pub fn cff_program_statistics(&self) -> BookV2CffProgramStatistics {
        BookV2CffProgramStatistics {
            operations_used: self.session.operations_used(),
            outline_segments_used: self.session.outline_segments_used(),
            cached_glyphs: self.session.cached_glyph_count(),
        }
    }
    pub fn record_charge(&self) -> u64 {
        self.history[0]
    }
    pub fn spool_charge(&self) -> u64 {
        self.history[1]
    }
    pub fn work_steps(&self) -> u64 {
        self.history[2]
    }
    /// Loan the same actual evaluator to a source-bound builder. The callback
    /// may return any value, including an error. Its private builder cannot be
    /// replaced or taken from the scoped facade. The higher-rank callback and
    /// invariant loan prevent the facade or session from escaping or crossing
    /// independent command loans. Accepted history is also retained on unwind.
    #[allow(clippy::too_many_arguments)]
    pub fn with_builder_counted<'a, D: BookV2ResourceDisplay<'a>, R>(
        &mut self,
        display: &D,
        limits: &M4EffectiveResourceLimits,
        prior_records: u64,
        prior_spool: u64,
        prior_work: u64,
        observed_records: &mut u64,
        observed_spool: &mut u64,
        observed_work: &mut u64,
        inspect: impl for<'loan> FnOnce(&mut BookV2CommandFontBuilder<'loan, 'a, D>) -> R,
    ) -> Result<R, E> {
        self.history[0] = self.history[0].max(prior_records);
        self.history[1] = self.history[1].max(prior_spool);
        self.history[2] = self.history[2].max(prior_work);
        *observed_records = self.history[0];
        *observed_spool = self.history[1];
        *observed_work = self.history[2];
        if limits.fingerprint() != self.limits
            || display.admitted().effective_limits().fingerprint() != self.limits
        {
            return Err(E::Identity);
        }
        let admission = display.admitted().session_identity();
        if self
            .admission
            .as_ref()
            .is_some_and(|bound| bound != &admission)
        {
            return Err(E::Identity);
        }
        let constructor = BookV2FontSelectionBuilderData::new_counted(
            display,
            limits,
            self.maximum_work,
            self.history[0],
            self.history[1],
            self.history[2],
            observed_records,
            observed_spool,
            observed_work,
        );
        self.history[0] = self.history[0].max(*observed_records);
        self.history[1] = self.history[1].max(*observed_spool);
        self.history[2] = self.history[2].max(*observed_work);
        // Source identity is verified before constructor reservations. Bind a
        // valid source even if a later reservation/work check refused the loan.
        if !matches!(&constructor, Err(E::Identity)) && self.admission.is_none() {
            self.admission = Some(admission);
        }
        let mut builder = constructor?;
        let original = &self.session as *const typaxis_font::Cff1SubsetSessionV2;
        builder.cff_session = Some(BookV2FontCffSession::Borrowed(&mut self.session));
        let mut builder = BookV2CommandFontBuilder {
            inner: builder,
            history: &mut self.history,
            observed_records,
            observed_spool,
            observed_work,
            invariant: std::marker::PhantomData,
        };
        let result = inspect(&mut builder);
        // The facade is owned here, never handed to the callback by value.
        // Read the retained original builder unconditionally after that loan.
        builder.retain_history();
        let authentic = builder
            .inner
            .cff_session
            .as_ref()
            .is_some_and(|session| session.is_borrowed_from(original));
        drop(builder);
        if !authentic {
            return Err(E::Identity);
        }
        Ok(result)
    }
}

/// A scoped command loan, with no public constructor or mutable builder escape.
/// Every operation delegates to the existing resource kernel. Its invariant
/// loan brand keeps independent command loans from being exchanged.
///
/// A fresh manual builder cannot replace the command's private builder.
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::BookV2ResourceDisplay;
/// use typaxis_resources::book_v2::{BookV2CommandFontBuilder, BookV2FontSelectionBuilderData};
/// fn replace<'loan, 'a, D: BookV2ResourceDisplay<'a>>(
///     loan: &mut BookV2CommandFontBuilder<'loan, 'a, D>,
///     fresh: BookV2FontSelectionBuilderData<'loan, 'a, D>,
/// ) { *loan = fresh; }
/// ```
///
/// ```compile_fail
/// use typaxis_display_list::book_v2::BookV2ResourceDisplay;
/// use typaxis_resources::book_v2::BookV2CommandFontBuilder;
/// fn exchange<'first, 'second, 'a, D: BookV2ResourceDisplay<'a>>(
///     first: &mut BookV2CommandFontBuilder<'first, 'a, D>,
///     second: &mut BookV2CommandFontBuilder<'second, 'a, D>,
/// ) { std::mem::swap(first, second); }
/// ```
pub struct BookV2CommandFontBuilder<'loan, 'a, D: BookV2ResourceDisplay<'a>> {
    inner: BookV2FontSelectionBuilderData<'loan, 'a, D>,
    history: &'loan mut [u64; 3],
    observed_records: &'loan mut u64,
    observed_spool: &'loan mut u64,
    observed_work: &'loan mut u64,
    invariant: std::marker::PhantomData<fn(&'loan ()) -> &'loan ()>,
}
impl<'loan, 'a, D: BookV2ResourceDisplay<'a>> BookV2CommandFontBuilder<'loan, 'a, D> {
    fn retain_history(&mut self) {
        self.history[0] = self.history[0].max(self.inner.record_charge());
        self.history[1] = self.history[1].max(self.inner.spool_charge());
        self.history[2] = self.history[2].max(self.inner.work_steps());
        *self.observed_records = self.history[0];
        *self.observed_spool = self.history[1];
        *self.observed_work = self.history[2];
    }
    pub fn record_charge(&self) -> u64 {
        self.inner.record_charge()
    }
    pub fn spool_charge(&self) -> u64 {
        self.inner.spool_charge()
    }
    pub fn work_steps(&self) -> u64 {
        self.inner.work_steps()
    }
    pub fn reservation_sizes(&self) -> [usize; 2] {
        self.inner.reservation_sizes()
    }
    pub fn cff_program_statistics(&self) -> BookV2CffProgramStatistics {
        self.inner.cff_program_statistics()
    }
    pub fn build(&mut self) -> Result<BookV2FontSelectionData<'loan, 'a, D>, E> {
        self.inner.build()
    }
    pub fn prepare_font_closures<'c>(
        &mut self,
        selection: &'c BookV2FontSelectionData<'loan, 'a, D>,
    ) -> Result<BookV2FontClosuresData<'c, 'loan, 'a, D>, BookV2FontClosureError> {
        self.inner.prepare_font_closures(selection)
    }
    pub fn prepare_cff_programs(
        &mut self,
        closures: &BookV2FontClosuresData<'_, 'loan, 'a, D>,
    ) -> Result<BookV2CffProgramStatistics, BookV2CffProgramError> {
        self.inner.prepare_cff_programs(closures)
    }
    pub fn write_font_programs<'x, 'c>(
        &mut self,
        closures: &'x BookV2FontClosuresData<'c, 'loan, 'a, D>,
    ) -> Result<BookV2FontProgramsData<'x, 'c, 'loan, 'a, D>, BookV2FontProgramsError> {
        self.inner.write_font_programs(closures)
    }
    pub fn plan_cids<'y, 'x, 'c>(
        &mut self,
        programs: &'y BookV2FontProgramsData<'x, 'c, 'loan, 'a, D>,
    ) -> Result<BookV2CidPlansData<'y, 'x, 'c, 'loan, 'a, D>, BookV2CidError> {
        self.inner.plan_cids(programs)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn next_stage_after_counted(
        &mut self,
        prior_records: u64,
        prior_spool: u64,
        prior_work: u64,
        observed_records: &mut u64,
        observed_spool: &mut u64,
        observed_work: &mut u64,
    ) -> Result<(), E> {
        self.inner.next_stage_after_in_place_counted(
            prior_records,
            prior_spool,
            prior_work,
            observed_records,
            observed_spool,
            observed_work,
        )
    }
    pub fn select_images(&mut self) -> Result<BookV2ImageSelectionData<'loan, 'a, D>, E> {
        self.inner.select_images()
    }
    pub fn write_raster_programs<'i>(
        &mut self,
        source: &'i BookV2ImageSelectionData<'loan, 'a, D>,
    ) -> Result<BookV2RasterProgramsData<'i, 'loan, 'a, D>, BookV2RasterError> {
        self.inner.write_raster_programs(source)
    }
}
impl<'loan, 'a, D: BookV2ResourceDisplay<'a>> Drop for BookV2CommandFontBuilder<'loan, 'a, D> {
    fn drop(&mut self) {
        self.retain_history();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use typaxis_core::{M4ResourceLimits, ResourceLimits, ValidatedResourceLimits};

    fn effective(extension: M4ResourceLimits) -> M4EffectiveResourceLimits {
        M4EffectiveResourceLimits::new(
            ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
            extension,
        )
        .unwrap()
    }

    #[test]
    fn book_v2_command_cff_borrow_keeps_exact_carrier_and_statistics() {
        let limits = effective(M4ResourceLimits::default());
        let mut carrier = BookV2FontCommandSession::new(&limits, 1000);
        let original = &carrier.session as *const typaxis_font::Cff1SubsetSessionV2;
        let before = carrier.cff_program_statistics();
        {
            let borrowed = BookV2FontCffSession::Borrowed(&mut carrier.session);
            assert!(borrowed.is_borrowed_from(original));
            let owned =
                BookV2FontCffSession::Owned(typaxis_font::Cff1SubsetSessionV2::new(&limits));
            assert!(!owned.is_borrowed_from(original));
            assert_eq!(borrowed.operations_used(), before.operations_used);
            assert_eq!(borrowed.cached_glyph_count(), before.cached_glyphs);
        }
        assert_eq!(carrier.cff_program_statistics(), before);
        assert_eq!(
            (
                carrier.record_charge(),
                carrier.spool_charge(),
                carrier.work_steps()
            ),
            (0, 0, 0)
        );
    }

    #[test]
    #[ignore = "requires explicit TYPAXIS_HARANO_FONT with pinned unchanged original CID CFF; no substitute"]
    fn book_v2_command_cff_original_cache_and_failed_quota_survive_borrowed_retries() {
        let bytes: std::sync::Arc<[u8]> =
            std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap())
                .unwrap()
                .into();
        assert_eq!(
            sha256(&bytes)
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect::<String>(),
            "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
        );
        let limits = effective(M4ResourceLimits::default());
        let admitted = typaxis_font::admit_sfnt_cff1_v2(bytes.clone(), 0, &limits).unwrap();
        let gid = OriginalGlyphId::new(admitted.cmap().glyph_for_sequence('A', None).unwrap());
        let selected = BTreeSet::from([gid]);
        let mut carrier = BookV2FontCommandSession::new(&limits, u64::MAX);
        {
            let mut borrowed = BookV2FontCffSession::Borrowed(&mut carrier.session);
            borrowed.prepare_face(&admitted, &selected).unwrap();
        }
        let measured = carrier.cff_program_statistics();
        assert!(measured.operations_used > 0);
        assert!(measured.cached_glyphs > 0);
        {
            let mut borrowed = BookV2FontCffSession::Borrowed(&mut carrier.session);
            borrowed.prepare_face(&admitted, &selected).unwrap();
        }
        assert_eq!(carrier.cff_program_statistics(), measured);

        let bounded = effective(M4ResourceLimits {
            max_cff_charstring_operations: 1,
            ..M4ResourceLimits::default()
        });
        let admitted = typaxis_font::admit_sfnt_cff1_v2(bytes, 0, &bounded).unwrap();
        let mut failed = BookV2FontCommandSession::new(&bounded, u64::MAX);
        {
            let mut borrowed = BookV2FontCffSession::Borrowed(&mut failed.session);
            assert!(borrowed.prepare_face(&admitted, &selected).is_err());
        }
        let spent = failed.cff_program_statistics();
        assert!(spent.operations_used > 0);
        {
            let mut borrowed = BookV2FontCffSession::Borrowed(&mut failed.session);
            assert!(borrowed.prepare_face(&admitted, &selected).is_err());
        }
        let after = failed.cff_program_statistics();
        assert!(after.operations_used >= spent.operations_used);
        assert!(after.outline_segments_used >= spent.outline_segments_used);
        assert!(after.cached_glyphs >= spent.cached_glyphs);
    }
}

// Keep the accepted Source prefix even when an internal native parser unwinds.
struct NativeMathHistory<'a, 'id, 'cells> {
    history: &'a mut [u64; 3],
    accounting: &'a mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
}
impl Drop for NativeMathHistory<'_, '_, '_> {
    fn drop(&mut self) {
        let current = self.accounting.counters();
        for (i, value) in self.history.iter_mut().enumerate() {
            *value = (*value).max(current[i]);
        }
    }
}
