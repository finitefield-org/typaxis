//! Distinct measured source owners for physical column-page candidates.
use super::*;
use typaxis_layout::book_v2::{BookV2ConvergedColumnLines, BookV2RebuiltColumnLineVariant};
use typaxis_syntax::book_v2::BookV2ColumnFramePlan;
#[path = "book_v2_column_headers.rs"]
mod headers;
pub use headers::*;

/// Source items preserve ordinary/table/definition namespaces and original
/// page names. No conversion into the single-body page owner is provided.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2PreparedColumnFlow, BookV2PreparedBodyFlow};
/// fn single<'f, 's, 'p, 'a>(v: BookV2PreparedColumnFlow<'f, 's, 'p, 'a>)
///     -> BookV2PreparedBodyFlow<'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2PreparedColumnFlow<'f, 's, 'p, 'a> {
    inner: BookV2PreparedBodyFlow<'f, 's, 'p, 'a>,
}
impl<'f, 's, 'p, 'a> BookV2PreparedColumnFlow<'f, 's, 'p, 'a> {
    pub fn lines(&self) -> &'s BookV2InlineLineLayout<'p, 'a> {
        self.inner.lines()
    }
    pub fn body_items(&self) -> &[ProductionBodyFlowItem] {
        self.inner.body_items()
    }
    pub fn definition_items(&self, index: usize) -> Option<&[ProductionBodyFlowItem]> {
        self.inner.definition_items(index)
    }
    pub fn body_page_name_index(&self, item: usize) -> Option<usize> {
        self.inner.body_page_name_index(item)
    }
    pub fn table_count(&self) -> usize {
        self.inner.table_count()
    }
    pub fn record_charge(&self) -> u64 {
        self.inner.record_charge()
    }
    pub fn column_plan(&self) -> &'p BookV2ColumnFramePlan<'a> {
        self.lines()
            .frames()
            .expect("column frames")
            .column_plan()
            .expect("column plan")
    }
}

pub fn prepare_book_v2_column_flow_counted<'f, 's, 'p, 'a>(
    stable: &'f BookV2ConvergedColumnLines<'s, 'p, 'a>,
    blocks: Option<&'f BookV2VectorBlockLayout<'s, 'p, 'a>>,
    limits: &M4EffectiveResourceLimits,
    prior_records: u64,
    observed_records: &mut u64,
) -> Result<BookV2PreparedColumnFlow<'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    prepare_body_flow_in_frames(
        stable.lines(),
        blocks,
        stable.footnotes(),
        limits,
        prior_records,
        observed_records,
        true,
    )
    .map(|inner| BookV2PreparedColumnFlow { inner })
}

/// The immutable reconstructed column graph retains its actual column plan.
pub fn prepare_book_v2_rebuilt_column_flow_counted<'f, 's, 'p, 'a>(
    variant: &'f BookV2RebuiltColumnLineVariant<'_, 's, 'p, 'a>,
    blocks: Option<&'f BookV2VectorBlockLayout<'s, 'p, 'a>>,
    limits: &M4EffectiveResourceLimits,
    prior_records: u64,
    observed_records: &mut u64,
) -> Result<BookV2PreparedColumnFlow<'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    prepare_body_flow_in_frames(variant.lines(), blocks, variant.footnotes(), limits,
        prior_records, observed_records, true)
        .map(|inner| BookV2PreparedColumnFlow { inner })
}

/// Kept separate from single-frame table measurements so old page/PDF consumers
/// cannot receive column measurements through a public accessor.
///
/// ```compile_fail
/// use typaxis_pagination::book_v2::{BookV2ColumnTableMeasurements, BookV2TableMeasurements};
/// fn single<'f, 's, 'p, 'a>(v: BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>)
///     -> BookV2TableMeasurements<'f, 's, 'p, 'a> { v }
/// ```
pub struct BookV2ColumnTableMeasurements<'f, 's, 'p, 'a> {
    pub(in crate::production_body::body_flow) inner: BookV2TableMeasurements<'f, 's, 'p, 'a>,
}
impl<'f, 's, 'p, 'a> BookV2ColumnTableMeasurements<'f, 's, 'p, 'a> {
    pub fn fingerprint(&self) -> [u8; 32] { self.inner.fingerprint() }
    pub fn item(&self, index: usize) -> Option<&ProductionBodyFlowItem> { self.inner.item(index) }
    pub fn table_source_definition(&self, index: usize) -> Option<Option<usize>> {
        self.inner.flow().table_source_definition(index)
    }
    pub fn table_parent(&self, index: usize) -> Option<Option<usize>> {
        self.inner.flow().table_parent(index)
    }
    pub fn body_items(&self) -> &[ProductionBodyFlowItem] {
        self.inner.flow().body_items()
    }
    pub fn definition_items(&self, index: usize) -> Option<&[ProductionBodyFlowItem]> {
        self.inner.flow().definition_items(index)
    }
    pub fn tables(&self) -> &[ProductionMeasuredTable] {
        self.inner.tables()
    }
    /// Original body leaf range, including empty tables, without flattening the
    /// table into an ordinary source request. Definition tables return None.
    pub fn body_table_range(&self, index: usize) -> Option<std::ops::Range<usize>> {
        let table = self.inner.flow().collected.tables.tables.get(index)?;
        table.definition.is_none().then(|| table.items.clone())
    }
    pub fn record_charge(&self) -> u64 {
        self.inner.record_charge()
    }
    pub fn column_plan(&self) -> &'p BookV2ColumnFramePlan<'a> {
        self.inner
            .flow()
            .lines()
            .frames()
            .expect("column frames")
            .column_plan()
            .expect("column plan")
    }
}
pub fn prepare_book_v2_column_table_measurements_counted<'f, 's, 'p, 'a>(
    flow: BookV2PreparedColumnFlow<'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    observed_records: &mut u64,
) -> Result<BookV2ColumnTableMeasurements<'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    prepare_book_v2_table_measurements_counted(flow.inner, limits, observed_records)
        .map(|inner| BookV2ColumnTableMeasurements { inner })
}
