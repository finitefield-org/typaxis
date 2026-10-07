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
