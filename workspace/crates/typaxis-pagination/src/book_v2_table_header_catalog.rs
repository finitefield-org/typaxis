//! Frame-validated header choices for physical body and footnote widths.
use super::*;

pub struct BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a> {
    base: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
    entries: Vec<(
        usize,
        PositiveLength,
        &'b BookV2TableHeaderVariant<'b, 'f, 's, 'p, 'a>,
    )>,
    limits: &'b M4EffectiveResourceLimits,
    records: u64,
    work: u64,
    fingerprint: [u8; 32],
}
impl<'b, 'f, 's, 'p, 'a> BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a> {
    pub fn base(&self) -> &'b BookV2TableMeasurements<'f, 's, 'p, 'a> {
        self.base
    }
    pub fn limits(&self) -> &'b M4EffectiveResourceLimits {
        self.limits
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
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// The caller prepays a scan of `len()` entries. Pointer identity is needed:
    /// an equivalent fingerprint cannot establish ownership or prepaid storage.
    pub(in crate::production_body::body_flow) fn contains_header(
        &self,
        header: &BookV2TableHeaderVariant<'_, '_, '_, '_, '_>,
    ) -> bool {
        self.entries
            .iter()
            .any(|(_, _, stored)| std::ptr::eq(*stored, header))
    }
    /// The caller charges this bounded lookup against its shared search ledger.
    pub fn lookup_work(&self) -> u64 {
        self.base
            .flow()
            .lines()
            .frames()
            .expect("validated frames")
            .measurement_region_lookup_work()
            + u64::from(self.entries.len().checked_ilog2().unwrap_or(0))
            + self.base.tables().len() as u64
            + 3
    }
    /// Follow the original topology, never a variant's numeric item indexes.
    fn root_table_index(&self, table: usize) -> Result<usize, ProductionBodyPaginationError> {
        root_table_index(self.base, table)
    }
    /// Resolve a root or nested table at the physical body/definition width.
    /// Catalog keys retain the ancestor root parent width, including its insets.
    pub fn for_region_width(
        &self,
        table: usize,
        actual: Option<PositiveLength>,
    ) -> Result<&'b BookV2TableHeaderVariant<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError>
    {
        let source = self
            .base
            .flow()
            .collected
            .tables
            .tables
            .get(table)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        let root = self.root_table_index(table)?;
        let root_owner = self.base.tables()[root].owner();
        let frames = self
            .base
            .flow()
            .lines()
            .frames()
            .ok_or_else(|| error(source.owner, E::ReceiptMismatch))?;
        let measured = if source.definition.is_some() {
            frames
                .footnote_region()
                .ok_or_else(|| error(source.owner, E::ReceiptMismatch))?
        } else {
            frames.body()
        };
        let original = frames
            .measurement_region(root_owner)
            .ok_or_else(|| error(source.owner, E::ReceiptMismatch))?;
        let width = actual
            .unwrap_or(measured.width())
            .get()
            .checked_sub(measured.width().get())
            .and_then(|d| original.width().get().checked_add(d))
            .and_then(PositiveLength::new)
            .ok_or_else(|| error(source.owner, E::WidthMismatch))?;
        let index = self
            .entries
            .binary_search_by_key(&(table, width.get().raw()), |(i, w, _)| (*i, w.get().raw()))
            .map_err(|_| {
                error(
                    source.owner,
                    E::TableHeaderWidthRequired {
                        table_index: table,
                        parent_width: width,
                    },
                )
            })?;
        Ok(self.entries[index].2)
    }
}

/// Entries must be strictly ordered by original table index and ancestor root
/// parent width. Independently remeasure each header's source hierarchy before accepting
/// it; labels or fingerprints alone never establish physical-frame compatibility.
/// An empty catalog discovers the exact parent width at the first attempted
/// continuation; it never authorizes reuse of an unverified base header.
pub fn prepare_book_v2_table_header_catalog<'b, 'f, 's, 'p, 'a>(
    base: &'b BookV2TableMeasurements<'f, 's, 'p, 'a>,
    headers: &[&'b BookV2TableHeaderVariant<'b, 'f, 's, 'p, 'a>],
    limits: &'b M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2TableHeaderCatalog<'b, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    let frames = base
        .flow()
        .lines()
        .frames()
        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
    let mut work = 0u64;
    let mut step = |n: u64| -> Result<(), ProductionBodyPaginationError> {
        work = work
            .checked_add(n)
            .filter(|n| *n <= maximum_work)
            .ok_or_else(|| error(root, E::TableSearchLimit))?;
        Ok(())
    };
    if headers.is_empty() {
        step(1)?;
        base.flow().verify(
            base.flow().lines(),
            base.flow().blocks(),
            base.flow().footnotes(),
            limits,
        )?;
    }
    let mut shared = base.record_charge();
    let mut independent = 0u64;
    for header in headers {
        step(1)?;
        header.verify(base, header.variant(), limits)?;
        shared = shared.max(header.shared_record_charge());
        independent = independent
            .checked_add(
                header
                    .record_charge()
                    .checked_sub(header.shared_record_charge())
                    .ok_or_else(|| error(root, E::ReceiptMismatch))?,
            )
            .ok_or_else(|| error(root, E::FragmentLimit))?;
    }
    let (projection_records, projection_work) = frames
        .table_source_occurrence_projection_budget()
        .map_err(map_error)?;
    let owner_records = headers
        .iter()
        .try_fold(0u64, |n, h| n.checked_add(h.leaves().len() as u64))
        .ok_or_else(|| error(root, E::FragmentLimit))?;
    let records = shared
        .max(prior_records)
        .checked_add(independent)
        .and_then(|n| n.checked_add(owner_records))
        .and_then(|n| n.checked_add(headers.len() as u64 + 1))
        .and_then(|n| {
            projection_records
                .checked_mul(headers.len() as u64)
                .and_then(|p| n.checked_add(p))
        })
        .filter(|n| *n <= limits.base().get().max_fragments)
        .ok_or_else(|| error(root, E::FragmentLimit))?;
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(headers.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    let mut last = None;
    let mut fingerprint = typaxis_core::sha256(b"typaxis.book-2-table-header-catalog/1");
    for header in headers {
        step(1)?;
        let index = header.table_index();
        let source = base
            .flow()
            .collected
            .tables
            .tables
            .get(index)
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        step(index as u64 + 1)?;
        let root_index = root_table_index(base, index)?;
        let root_owner = base.tables()[root_index].owner();
        let actual = header
            .variant()
            .flow()
            .lines()
            .frames()
            .ok_or_else(|| error(source.owner, E::ReceiptMismatch))?;
        step(actual.measurement_region_lookup_work())?;
        let width = actual
            .region(root_owner)
            .ok_or_else(|| error(source.owner, E::ReceiptMismatch))?
            .width();
        let key = (index, width.get().raw());
        if last.is_some_and(|p| p >= key) {
            return Err(error(source.owner, E::ReceiptMismatch));
        }
        last = Some(key);
        let mut owners = Vec::new();
        owners
            .try_reserve_exact(header.leaves().len())
            .map_err(|_| error(source.owner, E::AllocationFailure))?;
        let count = header.leaves().len();
        step(
            (count as u64)
                .checked_mul(u64::from(count.checked_ilog2().unwrap_or(0)) + 3)
                .ok_or_else(|| error(source.owner, E::ArithmeticOverflow))?,
        )?;
        owners.extend(header.leaves().iter().map(|leaf| leaf.source().owner()));
        owners.sort_unstable();
        owners.dedup();
        step(projection_work)?;
        let expected = frames
            .remeasure_table_parent_for_sources(
                root_owner,
                width,
                &owners,
                projection_work,
                frames.record_charge(),
            )
            .map_err(map_error)?;
        for leaf in header.leaves() {
            step(1)?;
            let item = header
                .variant()
                .item(leaf.variant_item_index())
                .ok_or_else(|| error(source.owner, E::ReceiptMismatch))?;
            match item.source() {
                Some(ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index,
                }) => {
                    let p = header
                        .variant()
                        .flow()
                        .lines()
                        .paragraphs()
                        .get(paragraph_index as usize)
                        .filter(|p| p.owner() == item.owner())
                        .ok_or_else(|| error(item.owner(), E::ReceiptMismatch))?;
                    let line = p
                        .selected()
                        .and_then(|s| s.lines().get(line_index as usize))
                        .ok_or_else(|| error(item.owner(), E::ReceiptMismatch))?;
                    step(expected.source_owner_lookup_work())?;
                    let target = expected
                        .paragraph(paragraph_index as usize, item.owner())
                        .ok_or_else(|| error(item.owner(), E::ReceiptMismatch))?;
                    let start = actual
                        .source_unit_start(paragraph_index as usize, line.line().start_unit())
                        .unwrap_or(actual.paragraphs()[paragraph_index as usize].start());
                    if line.inline_size() != target.width() || start != target.start() {
                        return Err(error(item.owner(), E::WidthMismatch));
                    }
                }
                Some(_) => {
                    step(expected.region_lookup_work() + actual.measurement_region_lookup_work())?;
                    let actual_region = actual
                        .region(item.owner())
                        .ok_or_else(|| error(item.owner(), E::ReceiptMismatch))?;
                    let expected_region = expected
                        .region(item.owner())
                        .ok_or_else(|| error(item.owner(), E::ReceiptMismatch))?;
                    if actual_region != expected_region {
                        return Err(error(item.owner(), E::WidthMismatch));
                    }
                }
                None => return Err(error(item.owner(), E::ReceiptMismatch)),
            }
        }
        step(1)?;
        let mut bytes = [0u8; 80];
        bytes[..32].copy_from_slice(&fingerprint);
        bytes[32..64].copy_from_slice(&header.fingerprint());
        bytes[64..72].copy_from_slice(&(index as u64).to_be_bytes());
        bytes[72..].copy_from_slice(&width.get().raw().to_be_bytes());
        fingerprint = typaxis_core::sha256(&bytes);
        entries.push((index, width, *header));
    }
    Ok(BookV2TableHeaderCatalog {
        base,
        entries,
        limits,
        records,
        work,
        fingerprint,
    })
}
fn map_error(e: typaxis_layout::ProductionInlinePreparationError) -> ProductionBodyPaginationError {
    use typaxis_layout::ProductionInlinePreparationErrorKind as L;
    error(
        e.owner,
        match e.kind {
            L::UnitLimit => E::FragmentLimit,
            L::AllocationFailure => E::AllocationFailure,
            L::ArithmeticOverflow => E::ArithmeticOverflow,
            L::Atomic(_) => E::TableSearchLimit,
            L::ReceiptMismatch => E::ReceiptMismatch,
            _ => E::WidthMismatch,
        },
    )
}

fn root_table_index(
    base: &BookV2TableMeasurements<'_, '_, '_, '_>,
    mut table: usize,
) -> Result<usize, ProductionBodyPaginationError> {
    loop {
        let source = base
            .flow()
            .collected
            .tables
            .tables
            .get(table)
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))?;
        match source.parent {
            None => return Ok(table),
            Some(parent) if parent < table => table = parent,
            Some(_) => return Err(error(source.owner, E::ReceiptMismatch)),
        }
    }
}
