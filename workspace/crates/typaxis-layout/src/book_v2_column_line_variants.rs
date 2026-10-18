//! Simultaneously live column line graphs for original-source header variants.
use super::*;
use typaxis_syntax::book_v2::BookV2ColumnFramePlan;

/// Exact replay inputs stay private to the column protocol.
///
/// ```compile_fail
/// use typaxis_layout::book_v2::{BookV2ColumnLineVariantSeed, BookV2BodyLineVariantSeed};
/// fn single<'a>(v: BookV2ColumnLineVariantSeed<'a>) -> BookV2BodyLineVariantSeed<'a> { v }
/// ```
pub struct BookV2ColumnLineVariantSeed<'a> {
    inner: BookV2BodyLineVariantSeed<'a>,
}
impl<'a> BookV2ColumnLineVariantSeed<'a> {
    pub fn source_flow(&self) -> &PreparedBookV2TextFlow<'_> {
        self.inner.source_flow()
    }
    pub fn column_plan(&self) -> &'a BookV2ColumnFramePlan<'a> {
        match self.inner.measured_frames {
            Some(MeasurementPageFrames::Columns(plan)) => plan,
            _ => unreachable!("column seed"),
        }
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.inner.fingerprint()
    }
    pub fn record_charge(&self) -> u64 {
        self.inner.record_charge()
    }
    pub fn work_steps(&self) -> u64 {
        self.inner.work_steps()
    }
    pub fn reshape_passes(&self) -> u16 {
        self.inner.reshape_passes()
    }
    pub fn source_record_charge(&self) -> u64 {
        self.inner.source_record_charge()
    }
    pub fn retained_record_charge(&self) -> u64 {
        self.inner.retained_record_charge()
    }
    pub fn prepare_budgeted_with_source_widths<'b>(
        &'b self,
        widths: &'b BookV2SourceWidthAssignments<'b, 'b>,
        allowance: &mut BookV2LineVariantBudget,
        prior_records: u64,
    ) -> Result<BookV2ColumnLineVariantSeed<'b>, ProductionBodyReshapeError> {
        self.inner
            .prepare_budgeted_with_source_widths(widths, allowance, prior_records)
            .map(|inner| BookV2ColumnLineVariantSeed { inner })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_budgeted_book_v2_column_line_variant_seed<'a>(
    policy: &'a BookV2ResourcePolicy<'a>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &'a M4EffectiveResourceLimits,
    japanese_mode: JapaneseLineBreakMode,
    columns: &'a BookV2ColumnFramePlan<'a>,
    allowance: &mut BookV2LineVariantBudget,
    prior_records: u64,
    native: Option<&'a BookV2NativeMath<'a>>,
    source_widths: Option<&'a BookV2SourceWidthAssignments<'a, 'a>>,
) -> Result<BookV2ColumnLineVariantSeed<'a>, ProductionBodyReshapeError> {
    allowance.records = allowance.records.max(prior_records);
    columns.verify(flow.body()).map_err(|_| {
        error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::ReceiptMismatch,
        )
    })?;
    prepare_variant_seed_in_measured_frames(
        policy,
        flow,
        admitted,
        bindings,
        limits,
        japanese_mode,
        columns.measurement_body(),
        allowance,
        prior_records,
        native,
        Some(MeasurementPageFrames::Columns(columns)),
        source_widths,
    )
    .map(|inner| BookV2ColumnLineVariantSeed { inner })
}

/// Borrowed view of one immutable reconstructed column graph.
pub struct BookV2RebuiltColumnLineVariant<'v, 's, 'p, 'a> {
    inner: &'v BookV2RebuiltBodyLineVariant<'s, 'p, 'a>,
}
impl<'v, 's, 'p, 'a> BookV2RebuiltColumnLineVariant<'v, 's, 'p, 'a> {
    pub fn lines(&self) -> &'s BookV2InlineLineLayout<'p, 'a> {
        self.inner.lines()
    }
    pub fn footnotes(&self) -> &'v BookV2FootnoteLines<'s, 'p, 'a> {
        self.inner.footnotes()
    }
    pub fn column_plan(&self) -> &'p BookV2ColumnFramePlan<'a> {
        self.lines()
            .frames()
            .expect("column frames")
            .column_plan()
            .expect("column plan")
    }
    pub fn record_charge(&self) -> u64 {
        self.inner.record_charge()
    }
    pub fn work_steps(&self) -> u64 {
        self.inner.work_steps()
    }
}

/// No conversion to the single-body reconstructed set is provided.
///
/// ```compile_fail
/// use typaxis_layout::book_v2::{BookV2RebuiltColumnLineVariants, BookV2RebuiltBodyLineVariants};
/// fn single<'s, 'p, 'a>(v: BookV2RebuiltColumnLineVariants<'s, 'p, 'a>)
///     -> BookV2RebuiltBodyLineVariants<'s, 'p, 'a> { v }
/// ```
pub struct BookV2RebuiltColumnLineVariants<'s, 'p, 'a> {
    inner: BookV2RebuiltBodyLineVariants<'s, 'p, 'a>,
    work: u64,
}
impl<'s, 'p, 'a> BookV2RebuiltColumnLineVariants<'s, 'p, 'a> {
    pub fn variants(
        &self,
    ) -> impl ExactSizeIterator<Item = BookV2RebuiltColumnLineVariant<'_, 's, 'p, 'a>> {
        self.inner
            .variants()
            .iter()
            .map(|inner| BookV2RebuiltColumnLineVariant { inner })
    }
    pub fn variant(&self, index: usize) -> Option<BookV2RebuiltColumnLineVariant<'_, 's, 'p, 'a>> {
        self.inner
            .variants()
            .get(index)
            .map(|inner| BookV2RebuiltColumnLineVariant { inner })
    }
    pub fn record_charge(&self) -> u64 {
        self.inner.record_charge()
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.inner.fingerprint()
    }
}

/// Use the shared iterative ownership-layer reconstruction. Column-plan identity,
/// temporary seed views and rejected-prefix work are checked before replay.
pub fn with_budgeted_rebuilt_book_v2_column_line_variants<R>(
    seeds: &[&BookV2ColumnLineVariantSeed<'_>],
    allowance: &mut BookV2LineVariantBudget,
    prior_records: u64,
    use_variants: impl FnOnce(BookV2RebuiltColumnLineVariants<'_, '_, '_>) -> R,
) -> Result<R, ProductionBodyReshapeError> {
    allowance.records = allowance.records.max(prior_records);
    let maximum = allowance.remaining_work();
    let mut work = 0;
    let result = (|| {
        let root = NodeId::new(0);
        let first = seeds
            .first()
            .ok_or_else(|| error(root, ProductionInlinePreparationErrorKind::ReceiptMismatch))?;
        for seed in seeds {
            take_work(&mut work, 1, maximum)?;
            allowance.records = allowance.records.max(seed.record_charge());
            if !std::ptr::eq(seed.column_plan(), first.column_plan()) {
                return Err(
                    error(root, ProductionInlinePreparationErrorKind::ReceiptMismatch).into(),
                );
            }
        }
        allowance.records = allowance
            .records
            .checked_add(seeds.len() as u64)
            .filter(|n| *n <= first.inner.limits.base().get().max_fragments)
            .ok_or_else(|| error(root, ProductionInlinePreparationErrorKind::UnitLimit))?;
        let mut views = Vec::new();
        views
            .try_reserve_exact(seeds.len())
            .map_err(|_| BreakError::AllocationFailure)?;
        for seed in seeds {
            take_work(&mut work, 1, maximum)?;
            views.push(&seed.inner);
        }
        let prefix = work;
        allowance.work += work;
        work = 0;
        with_budgeted_rebuilt_book_v2_body_line_variants(
            &views,
            allowance,
            allowance.records,
            |inner| {
                use_variants(BookV2RebuiltColumnLineVariants {
                    work: prefix + inner.work_steps(),
                    inner,
                })
            },
        )
    })();
    allowance.work += work;
    result
}

/// An immutable, source-bound captured seed created in the actual caller loan.
/// Neither a raw Legacy seed nor a reported number can construct this owner.
pub struct BookV2ObservedColumnLineVariantSeed<'id, 'a> {
    pub(super) inner: ObservedVariantSeed<'id, 'a>,
}
impl<'id, 'a> BookV2ObservedColumnLineVariantSeed<'id, 'a> {
    pub fn source_flow(&self) -> &PreparedBookV2TextFlow<'_> {
        self.inner.flow
    }
    pub fn column_plan(&self) -> &'a BookV2ColumnFramePlan<'a> {
        self.inner.columns
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.inner.contexts.source_fingerprint()
    }
    pub fn contexts(&self) -> &ProductionSelectedLineContexts {
        &self.inner.contexts
    }
    pub fn replay_record_bound(&self) -> u64 {
        self.inner.rebuild_records
    }
    pub fn captured_context_records(&self) -> u64 {
        self.inner.context_records
    }
    pub fn prepared_counters(&self) -> [u64; 3] {
        self.inner.prepared_rsw
    }
    pub fn prepare_observed_with_source_widths<'b>(
        &'b self,
        widths: &'b BookV2SourceWidthAssignments<'b, 'b>,
        budget: &mut BookV2ObservedLineVariantBudget,
        accounting: &mut BookV2ObservedSourceAccounting<'id, '_>,
    ) -> Result<BookV2ObservedColumnLineVariantSeed<'id, 'b>, BookV2ObservedBodyReshapeError> {
        prepare_observed_variant_seed(
            self.inner.policy,
            self.inner.flow,
            self.inner.admitted,
            self.inner.bindings,
            self.inner.limits,
            self.inner.mode,
            self.inner.columns,
            self.inner.native,
            Some(widths),
            budget,
            accounting,
        )
        .map(|inner| BookV2ObservedColumnLineVariantSeed { inner })
    }
}

/// Capture a staged graph, then rebuild independently paid occurrences.
/// ```
/// use typaxis_core::M4EffectiveResourceLimits;
/// use typaxis_layout::book_v2::*;
/// use typaxis_linebreak::JapaneseLineBreakMode;
/// use typaxis_resource_admission::AdmittedProductionResourceLedgerV3;
/// use typaxis_shaping::book_v2::BookV2ObservedSourceAccounting;
/// use typaxis_syntax::book_v2::{BookV2ResourcePolicy,PreparedBookV2TextFlow,BookV2ColumnFramePlan};
/// fn capture<'id,'a>(p:&'a BookV2ResourcePolicy<'a>,flow:&'a PreparedBookV2TextFlow<'a>,
///     admitted:&'a AdmittedProductionResourceLedgerV3,b:&'a BookV2VectorBindings<'a>,
///     limits:&'a M4EffectiveResourceLimits,columns:&'a BookV2ColumnFramePlan<'a>,
///     native:Option<&'a BookV2ObservedNativeMath<'id,'a>>,mode:JapaneseLineBreakMode,
///     loan:&mut BookV2ObservedSourceAccounting<'id,'_>) {
///     let mut budget=BookV2ObservedLineVariantBudget::new(1000000,16);
///     if let Ok(seed)=prepare_observed_book_v2_column_line_variant_seed(
///         p,flow,admitted,b,limits,mode,columns,native,None,&mut budget,loan) {
///         let _=with_observed_rebuilt_book_v2_column_line_variants(&[&seed,&seed],&mut budget,loan,
///             |variants,_|assert_eq!(variants.variants().len(),2));
///     }
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn prepare_observed_book_v2_column_line_variant_seed<'id, 'a>(
    policy: &'a BookV2ResourcePolicy<'a>,
    flow: &'a PreparedBookV2TextFlow<'a>,
    admitted: &'a AdmittedProductionResourceLedgerV3,
    bindings: &'a BookV2VectorBindings<'a>,
    limits: &'a M4EffectiveResourceLimits,
    mode: JapaneseLineBreakMode,
    columns: &'a BookV2ColumnFramePlan<'a>,
    native: Option<&'a BookV2ObservedNativeMath<'id, 'a>>,
    widths: Option<&'a BookV2SourceWidthAssignments<'a, 'a>>,
    budget: &mut BookV2ObservedLineVariantBudget,
    accounting: &mut BookV2ObservedSourceAccounting<'id, '_>,
) -> Result<BookV2ObservedColumnLineVariantSeed<'id, 'a>, BookV2ObservedBodyReshapeError> {
    prepare_observed_variant_seed(
        policy, flow, admitted, bindings, limits, mode, columns, native, widths, budget, accounting,
    )
    .map(|inner| BookV2ObservedColumnLineVariantSeed { inner })
}

fn observed_column_views_request<'id>(
    count: usize,
) -> Result<[u64; 3], BookV2ObservedBodyReshapeError> {
    let units = observed_units(count)?;
    Ok([
        units,
        observed_bytes::<&ObservedVariantSeed<'id, '_>>(count)?,
        units,
    ])
}

pub fn with_observed_rebuilt_book_v2_column_line_variants<'id, 'cells, R>(
    seeds: &[&BookV2ObservedColumnLineVariantSeed<'id, '_>],
    budget: &mut BookV2ObservedLineVariantBudget,
    accounting: &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    inspect: impl FnOnce(
        BookV2ObservedRebuiltColumnLineVariants<'id, '_, '_, '_>,
        &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    ) -> R,
) -> Result<R, BookV2ObservedBodyReshapeError> {
    // Authenticate the actual seed family before a short metadata allowance
    // can mask an empty/mixed-family refusal. The private preflight carries
    // only scan results; it cannot construct or select a paid occurrence.
    let family = super::variant_set::preflight_observed_variants(seeds, budget, accounting)?;
    budget.request(accounting, observed_column_views_request(seeds.len())?)?;
    let mut views = Vec::new();
    reserve_observed_protocol(
        &mut views,
        seeds.len(),
        ObservedProtocolReserveSite::ColumnViews,
    )?;
    views.extend(seeds.iter().map(|seed| &seed.inner));
    super::variant_set::with_observed_variants(&views, family, budget, accounting, inspect)
}

pub fn with_observed_rebuilt_book_v2_column_line_variant<'id, 'cells, R>(
    seed: &BookV2ObservedColumnLineVariantSeed<'id, '_>,
    budget: &mut BookV2ObservedLineVariantBudget,
    accounting: &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    inspect: impl FnOnce(
        BookV2ObservedRebuiltColumnLineVariant<'_, 'id, '_, '_, '_>,
        &mut BookV2ObservedSourceAccounting<'id, 'cells>,
    ) -> R,
) -> Result<R, BookV2ObservedBodyReshapeError> {
    with_observed_rebuilt_book_v2_column_line_variants(
        &[seed],
        budget,
        accounting,
        |variants, accounting| {
            inspect(
                variants.variant(0).expect("single observed occurrence"),
                accounting,
            )
        },
    )
}

#[cfg(test)]
mod observed_p2_column_view_controls {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_shaping::book_v2::with_book_v2_observed_source_accounting;

    #[test]
    fn observed_p2_column_view_sizing_and_fault_order() {
        for count in [0usize, 1, 3] {
            assert_eq!(
                observed_column_views_request(count).unwrap(),
                [
                    count as u64,
                    (count * std::mem::size_of::<&ObservedVariantSeed<'_, '_>>()) as u64,
                    count as u64,
                ]
            );
        }
        #[cfg(target_pointer_width = "64")]
        assert!(matches!(
            observed_column_views_request(usize::MAX),
            Err(BookV2ObservedBodyReshapeError::Accounting(
                BookV2ObservedAccountingError::Spool { .. }
            ))
        ));
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let (mut r, mut s, mut w) = (3, 5, 7);
        with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 100, 1000, |a| {
            let mut budget = BookV2ObservedLineVariantBudget::new(1000, 16);
            budget.begin(
                BookV2ObservedVariantPhase::ProtocolStorage,
                None,
                a,
                0,
                None,
                None,
            );
            let request = observed_column_views_request(3).unwrap();
            budget.request(a, request).unwrap();
            let accepted = a.counters();
            let site = ObservedProtocolReserveSite::ColumnViews;
            observed_protocol_fault_once(site);
            // Request-only unit. No fake seed references or graph roots.
            let mut storage = Vec::<usize>::new();
            assert!(matches!(
                reserve_observed_protocol(&mut storage, 3, site),
                Err(BreakError::AllocationFailure)
            ));
            assert_eq!(a.counters(), accepted);
            assert_eq!(budget.record_charge(), accepted[0]);
            assert_eq!(budget.work_steps(), request[2]);
            assert_eq!((storage.len(), storage.capacity()), (0, 0));
        });
    }
}
