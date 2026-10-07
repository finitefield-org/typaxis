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
