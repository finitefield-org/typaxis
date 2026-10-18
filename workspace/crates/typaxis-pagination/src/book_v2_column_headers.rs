//! Column-owned source bindings for simultaneously reconstructed table headers.
use super::*;
use typaxis_layout::book_v2::BookV2RebuiltColumnLineVariants;

/// A measured original header remains distinct from a single-body header owner.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnTableHeaderVariant, BookV2TableHeaderVariant};
/// fn single<'b, 'f, 's, 'p, 'a>(v: BookV2ColumnTableHeaderVariant<'b, 'f, 's, 'p, 'a>)
///     -> BookV2TableHeaderVariant<'b, 'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnTableHeaderVariant<'b, 'f, 's, 'p, 'a> {
    base: &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>,
    variant: &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>,
    inner: BookV2TableHeaderVariant<'b, 'f, 's, 'p, 'a>,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnTableHeaderVariant<'b, 'f, 's, 'p, 'a> {
    pub fn variant(&self) -> &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a> {
        self.variant
    }
    pub fn table_index(&self) -> usize {
        self.inner.table_index()
    }
    pub fn definition_index(&self) -> Option<usize> {
        self.inner.definition_index()
    }
    pub fn height(&self) -> Length {
        self.inner.height()
    }
    pub fn leaves(&self) -> &[BookV2TableHeaderVariantLeaf] {
        self.inner.leaves()
    }
    pub fn record_charge(&self) -> u64 {
        self.inner.record_charge()
    }
    pub fn shared_record_charge(&self) -> u64 {
        self.inner.shared_record_charge()
    }
    pub fn work_steps(&self) -> u64 {
        self.inner.work_steps()
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.inner.fingerprint()
    }
    pub fn verify(
        &self,
        base: &BookV2ColumnTableMeasurements<'_, '_, '_, '_>,
        variant: &BookV2ColumnTableMeasurements<'_, '_, '_, '_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyPaginationError> {
        if !std::ptr::eq(base, self.base) || !std::ptr::eq(variant, self.variant) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        self.inner.verify(&base.inner, &variant.inner, limits)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_book_v2_column_table_header_variant_counted<'b, 'f, 's, 'p, 'a>(
    set: &BookV2RebuiltColumnLineVariants<'_, '_, '_>,
    base: &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>,
    variant: &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>,
    table: usize,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
    observed_records: &mut u64,
    observed_work: &mut u64,
) -> Result<BookV2ColumnTableHeaderVariant<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    super::super::header_variant::prepare_header_variant_in_graphs(
        set,
        &base.inner,
        &variant.inner,
        table,
        limits,
        maximum_work,
        prior_records,
        observed_records,
        observed_work,
    )
    .map(|inner| BookV2ColumnTableHeaderVariant {
        base,
        variant,
        inner,
    })
}

/// Bind the two genuine members of an observed, simultaneously live graph set.
#[allow(clippy::too_many_arguments)]
pub fn prepare_observed_book_v2_column_table_header_variant_counted<'b, 'f, 's, 'p, 'a>(
    set: &typaxis_layout::book_v2::BookV2ObservedRebuiltColumnLineVariants<'_, '_, '_, '_>,
    base: &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>,
    variant: &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>,
    table: usize,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
    observed_records: &mut u64,
    observed_work: &mut u64,
) -> Result<BookV2ColumnTableHeaderVariant<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    super::super::header_variant::prepare_header_variant_in_graphs(
        set,
        &base.inner,
        &variant.inner,
        table,
        limits,
        maximum_work,
        prior_records,
        observed_records,
        observed_work,
    )
    .map(|inner| BookV2ColumnTableHeaderVariant {
        base,
        variant,
        inner,
    })
}

/// Entries retain column measurements; no conversion to a single-body catalog.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnTableHeaderCatalog, BookV2TableHeaderCatalog};
/// fn single<'b, 'f, 's, 'p, 'a>(v: BookV2ColumnTableHeaderCatalog<'b, 'f, 's, 'p, 'a>)
///     -> BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnTableHeaderCatalog<'b, 'f, 's, 'p, 'a> {
    base: &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>,
    pub(in crate::production_body::body_flow) inner: BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a>,
    work: u64,
}
impl<'b, 'f, 's, 'p, 'a> BookV2ColumnTableHeaderCatalog<'b, 'f, 's, 'p, 'a> {
    pub fn base(&self) -> &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a> {
        self.base
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
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

pub fn prepare_book_v2_column_table_header_catalog_counted<'b, 'f, 's, 'p, 'a>(
    base: &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>,
    headers: &[&'b BookV2ColumnTableHeaderVariant<'b, 'f, 's, 'p, 'a>],
    limits: &'b M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
    observed_records: &mut u64,
    observed_work: &mut u64,
) -> Result<BookV2ColumnTableHeaderCatalog<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    *observed_records = prior_records.max(base.record_charge());
    *observed_work = 0;
    // The temporary reference view is reserved before conversion/allocation.
    *observed_records = observed_records
        .checked_add(headers.len() as u64)
        .filter(|n| *n <= limits.base().get().max_fragments)
        .ok_or_else(|| error(NodeId::new(0), E::FragmentLimit))?;
    let mut refs = Vec::new();
    refs.try_reserve_exact(headers.len())
        .map_err(|_| error(NodeId::new(0), E::AllocationFailure))?;
    for header in headers {
        *observed_work = observed_work
            .checked_add(1)
            .filter(|n| *n <= maximum_work)
            .ok_or_else(|| error(NodeId::new(0), E::TableSearchLimit))?;
        header.verify(base, header.variant(), limits)?;
        refs.push(&header.inner);
    }
    let prefix = *observed_work;
    let mut work = 0;
    let result = prepare_book_v2_table_header_catalog_counted(
        &base.inner,
        &refs,
        limits,
        maximum_work - prefix,
        *observed_records,
        observed_records,
        &mut work,
    );
    *observed_work = prefix + work;
    result.map(|inner| BookV2ColumnTableHeaderCatalog {
        base,
        inner,
        work: *observed_work,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ObservedColumnHeaderCatalogError {
    Accounting(typaxis_shaping::book_v2::BookV2ObservedAccountingError),
    Catalog(ProductionBodyPaginationError),
}
impl std::fmt::Display for BookV2ObservedColumnHeaderCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "observed column header catalog: {self:?}")
    }
}
impl std::error::Error for BookV2ObservedColumnHeaderCatalogError {}

fn observed_catalog_view_request(
    n: usize,
) -> Result<[u64; 3], BookV2ObservedColumnHeaderCatalogError> {
    use typaxis_shaping::book_v2::BookV2ObservedAccountingError as A;
    let owner = NodeId::new(0);
    let slots = u64::try_from(n)
        .map_err(|_| BookV2ObservedColumnHeaderCatalogError::Accounting(A::Records { owner }))?;
    let bytes = n
        .checked_mul(std::mem::size_of::<
            &BookV2TableHeaderVariant<'_, '_, '_, '_, '_>,
        >())
        .and_then(|n| u64::try_from(n).ok())
        .ok_or(BookV2ObservedColumnHeaderCatalogError::Accounting(
            A::Spool { owner },
        ))?;
    Ok([slots, bytes, slots])
}

#[cfg(test)]
thread_local! {
    static FAIL_OBSERVED_CATALOG_RESERVE: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

fn reserve_observed_catalog_view<'h, 'b, 'f, 's, 'p, 'a>(
    refs: &mut Vec<&'h BookV2TableHeaderVariant<'b, 'f, 's, 'p, 'a>>,
    n: usize,
    accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<(), BookV2ObservedColumnHeaderCatalogError> {
    use BookV2ObservedColumnHeaderCatalogError as C;
    accounting
        .debit(NodeId::new(0), observed_catalog_view_request(n)?)
        .map_err(C::Accounting)?;
    #[cfg(test)]
    if FAIL_OBSERVED_CATALOG_RESERVE.with(|flag| flag.replace(false)) {
        return Err(C::Catalog(error(NodeId::new(0), E::AllocationFailure)));
    }
    refs.try_reserve_exact(n)
        .map_err(|_| C::Catalog(error(NodeId::new(0), E::AllocationFailure)))
}

/// Pay for the real inner header-reference buffer on the live source counters.
/// Earlier graph/measurement/header charges must already be retained by that
/// caller. This adds no conversion from observed graphs into a legacy graph set.
pub fn prepare_book_v2_column_table_header_catalog_with_observed_source_counted<
    'id,
    'cells,
    'b,
    'f,
    's,
    'p,
    'a,
>(
    base: &'b BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>,
    headers: &[&'b BookV2ColumnTableHeaderVariant<'b, 'f, 's, 'p, 'a>],
    limits: &'b M4EffectiveResourceLimits,
    accounting: &mut typaxis_shaping::book_v2::BookV2ObservedSourceAccounting<'id, 'cells>,
) -> Result<
    BookV2ColumnTableHeaderCatalog<'b, 'f, 's, 'p, 'a>,
    BookV2ObservedColumnHeaderCatalogError,
> {
    use BookV2ObservedColumnHeaderCatalogError as C;
    let root = NodeId::new(0);
    accounting
        .require_limit_domain(limits.base())
        .map_err(C::Accounting)?;
    // The pagination kernel uses the authenticated fragment ceiling. A loan
    // with a smaller private ceiling cannot authorize that kernel's requests.
    if accounting.maximum_records() != limits.base().get().max_fragments {
        return Err(C::Accounting(
            typaxis_shaping::book_v2::BookV2ObservedAccountingError::LimitsMismatch,
        ));
    }
    accounting
        .require_limit_domain(
            base.inner
                .flow()
                .lines()
                .prepared()
                .source_flow()
                .body()
                .body()
                .limits(),
        )
        .map_err(C::Accounting)?;
    accounting.debit(root, [0; 3]).map_err(C::Accounting)?;
    if base.record_charge() > accounting.counters()[0] {
        return Err(C::Catalog(error(root, E::ReceiptMismatch)));
    }
    for header in headers {
        accounting.debit_work(root, 1).map_err(C::Accounting)?;
        header
            .verify(base, header.variant(), limits)
            .map_err(C::Catalog)?;
        if header.record_charge() > accounting.counters()[0] {
            return Err(C::Catalog(error(root, E::ReceiptMismatch)));
        }
    }
    let mut refs = Vec::new();
    reserve_observed_catalog_view(&mut refs, headers.len(), accounting)?;
    refs.extend(headers.iter().map(|header| &header.inner));
    let before = accounting.counters();
    let (mut records, mut work) = (before[0], 0);
    let result = prepare_book_v2_table_header_catalog_counted(
        &base.inner,
        &refs,
        limits,
        accounting.remaining_work(),
        before[0],
        &mut records,
        &mut work,
    );
    // Publish independently accepted axes before propagating the primary error.
    let kept_records = records
        .checked_sub(before[0])
        .ok_or(C::Catalog(error(root, E::ReceiptMismatch)))
        .and_then(|n| accounting.debit_records(root, n).map_err(C::Accounting));
    let kept_work = accounting.debit_work(root, work).map_err(C::Accounting);
    let inner = result.map_err(C::Catalog)?;
    kept_records?;
    kept_work?;
    Ok(BookV2ColumnTableHeaderCatalog {
        base,
        inner,
        work: headers.len() as u64 + work,
    })
}

#[cfg(test)]
mod observed_catalog_controls {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_shaping::book_v2::{
        with_book_v2_observed_source_accounting, BookV2ObservedAccountingError as A,
    };

    #[test]
    fn observed_column_catalog_reference_storage_exact_and_each_short_axis_are_atomic() {
        let prior = [7, 11, 13];
        let bytes = 3 * std::mem::size_of::<&BookV2TableHeaderVariant<'_, '_, '_, '_, '_>>() as u64;
        let wanted = [prior[0] + 3, prior[1] + bytes, prior[2] + 3];
        for short in [None, Some(0), Some(1), Some(2)] {
            let mut base = ResourceLimits::default();
            base.max_fragments = wanted[0] - u64::from(short == Some(0));
            base.max_spool_bytes = wanted[1] - u64::from(short == Some(1));
            let limits = ValidatedResourceLimits::new(base).unwrap();
            let [mut r, mut s, mut w] = prior;
            let mut refs = Vec::new();
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                wanted[0],
                wanted[2] - u64::from(short == Some(2)),
                |a| reserve_observed_catalog_view(&mut refs, 3, a),
            );
            if let Some(axis) = short {
                let owner = NodeId::new(0);
                let expected = [A::Records { owner }, A::Spool { owner }, A::Work { owner }][axis];
                assert_eq!(
                    result,
                    Err(BookV2ObservedColumnHeaderCatalogError::Accounting(expected))
                );
                assert_eq!([r, s, w], prior);
                assert_eq!(refs.capacity(), 0);
            } else {
                result.unwrap();
                assert_eq!([r, s, w], wanted);
                assert!(refs.capacity() >= 3);
            }
        }
    }

    #[test]
    fn observed_column_catalog_postpayment_allocation_failure_keeps_all_three_axes() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let (mut r, mut s, mut w) = (7, 11, 13);
        let mut refs = Vec::new();
        FAIL_OBSERVED_CATALOG_RESERVE.with(|f| f.set(true));
        let result = with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            &limits,
            100,
            100,
            |a| reserve_observed_catalog_view(&mut refs, 3, a),
        );
        assert_eq!(
            result,
            Err(BookV2ObservedColumnHeaderCatalogError::Catalog(error(
                NodeId::new(0),
                E::AllocationFailure
            )))
        );
        assert_eq!(
            [r, s, w],
            [
                10,
                11 + 3 * std::mem::size_of::<&BookV2TableHeaderVariant<'_, '_, '_, '_, '_>>()
                    as u64,
                16
            ]
        );
        assert_eq!(refs.capacity(), 0);
        assert!(!FAIL_OBSERVED_CATALOG_RESERVE.with(|f| f.get()));
    }

    #[test]
    fn observed_column_catalog_reference_storage_overflow_and_zero_request() {
        assert_eq!(observed_catalog_view_request(0).unwrap(), [0; 3]);
        assert!(matches!(
            observed_catalog_view_request(usize::MAX),
            Err(BookV2ObservedColumnHeaderCatalogError::Accounting(
                A::Spool { .. }
            ))
        ));
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let (mut r, mut s, mut w) = (7, 11, 13);
        let mut refs = Vec::new();
        with_book_v2_observed_source_accounting(&mut r, &mut s, &mut w, &limits, 7, 13, |a| {
            reserve_observed_catalog_view(&mut refs, 0, a)
        })
        .unwrap();
        assert_eq!([r, s, w], [7, 11, 13]);
        assert_eq!(refs.capacity(), 0);
    }
}
