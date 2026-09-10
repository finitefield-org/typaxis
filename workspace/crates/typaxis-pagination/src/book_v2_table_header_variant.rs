//! Original-source header geometry from a different, simultaneously live line graph.
use super::*;
use typaxis_layout::book_v2::BookV2RebuiltBodyLineVariants;
use typaxis_syntax::ProductionTableSection;

pub struct BookV2TableHeaderVariantLeaf {
    cell: Option<NodeId>,
    item: usize,
    top: Length,
    source: BookV2TableWidthSource,
}
impl BookV2TableHeaderVariantLeaf {
    pub fn cell_owner(&self) -> Option<NodeId> {
        self.cell
    }
    /// Global index in the variant measurement, never in the base measurement.
    pub fn variant_item_index(&self) -> usize {
        self.item
    }
    pub fn top(&self) -> Length {
        self.top
    }
    /// Original logical units or block owner, independent of either line ordinal.
    pub fn source(&self) -> &BookV2TableWidthSource {
        &self.source
    }
}

/// A complete repeated-header geometry candidate. This is not a semantic
/// consumption receipt, a selected physical page, or PDF paint authorization.
pub struct BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a> {
    base: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    variant: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    table: usize,
    height: Length,
    leaves: Vec<BookV2TableHeaderVariantLeaf>,
    records: u64,
    shared_records: u64,
    projection_records: u64,
    work: u64,
    fingerprint: [u8; 32],
    limits: [u8; 32],
}
impl<'m, 'f, 's, 'p, 'a> BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a> {
    pub fn variant(&self) -> &'m BookV2TableMeasurements<'f, 's, 'p, 'a> {
        self.variant
    }
    pub fn table_index(&self) -> usize {
        self.table
    }
    pub fn definition_index(&self) -> Option<usize> {
        self.variant.flow().collected.tables.tables[self.table].definition
    }
    pub fn height(&self) -> Length {
        self.height
    }
    pub fn leaves(&self) -> &[BookV2TableHeaderVariantLeaf] {
        &self.leaves
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    /// Shared history/set/base, excluding this variant's retained projections.
    pub fn shared_record_charge(&self) -> u64 {
        self.shared_records
    }
    /// Prepaid traversal/output bound. Consumers may conservatively recharge it
    /// for every retained use instead of assuming separate header owners alias.
    pub fn projection_record_charge(&self) -> u64 {
        self.projection_records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    /// Reserve this actual variant height; no old-header height is substituted.
    pub fn remaining_height(
        &self,
        available: Length,
    ) -> Result<Length, ProductionBodyPaginationError> {
        if available < self.height {
            return Err(error(
                self.variant.tables()[self.table].owner(),
                E::TableHeaderOversize,
            ));
        }
        available
            .checked_sub(self.height)
            .ok_or_else(|| error(NodeId::new(0), E::ArithmeticOverflow))
    }
    pub fn verify(
        &self,
        base: &BookV2TableMeasurements<'_, '_, '_, '_>,
        variant: &BookV2TableMeasurements<'_, '_, '_, '_>,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyPaginationError> {
        if !std::ptr::eq(base, self.base)
            || !std::ptr::eq(variant, self.variant)
            || limits.fingerprint() != self.limits
        {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}

enum Visit<'a> {
    Table {
        index: usize,
        origin: Length,
        header: bool,
    },
    Content {
        content: &'a ProductionTableCellContent,
        cell: Option<NodeId>,
        origin: Length,
        owner: NodeId,
    },
}

/// Both measurements must belong to the exact source-compatible reconstructed
/// set. Content fingerprints alone cannot mint the cross-graph association.
pub fn prepare_book_v2_table_header_variant<'m, 'f, 's, 'p, 'a>(
    set: &BookV2RebuiltBodyLineVariants<'_, '_, '_>,
    base: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    variant: &'m BookV2TableMeasurements<'f, 's, 'p, 'a>,
    table_index: usize,
    limits: &M4EffectiveResourceLimits,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2TableHeaderVariant<'m, 'f, 's, 'p, 'a>, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    let mut work = 0u64;
    let mut step = |n: u64| -> Result<(), ProductionBodyPaginationError> {
        work = work
            .checked_add(n)
            .filter(|n| *n <= maximum_work)
            .ok_or_else(|| error(root, E::TableSearchLimit))?;
        Ok(())
    };
    let (mut has_base, mut has_variant) = (false, false);
    for entry in set.variants() {
        step(2)?;
        has_base |= std::ptr::eq(base.flow().lines(), entry.lines());
        has_variant |= std::ptr::eq(variant.flow().lines(), entry.lines());
    }
    if !has_base || !has_variant {
        return Err(error(root, E::ReceiptMismatch));
    }
    for measurement in [base, variant] {
        step(1)?;
        let f = measurement.flow();
        f.verify(f.lines(), f.blocks(), f.footnotes(), limits)?;
    }
    let table = variant
        .tables()
        .get(table_index)
        .ok_or_else(|| error(root, E::ReceiptMismatch))?;
    let owner = table.owner();
    step(1)?;
    if base.tables().get(table_index).map(|t| t.owner()) != Some(owner)
        || base.flow().table_source_definition(table_index)
            != variant.flow().table_source_definition(table_index)
        || base.flow().table_parent(table_index) != variant.flow().table_parent(table_index)
    {
        return Err(error(owner, E::ReceiptMismatch));
    }
    let caption_height = table.caption().map_or(Length::ZERO, |c| c.height());
    let mut end = table.height();
    for row in table.rows() {
        step(1)?;
        if row.section() != ProductionTableSection::Head {
            end = row.top();
            break;
        }
    }
    let height = end
        .checked_sub(caption_height)
        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
    // A conservative whole-graph bound covers every pending DFS task and output
    // leaf. Count without allocation, then reserve each vector exactly once.
    let mut task_bound = 1usize;
    for t in variant.tables() {
        step(1)?;
        task_bound = task_bound
            .checked_add(1)
            .ok_or_else(|| error(owner, E::FragmentLimit))?;
        for contents in t
            .caption()
            .map(|c| c.content())
            .into_iter()
            .chain(t.cells().iter().map(|c| c.content()))
        {
            step(1)?;
            task_bound = task_bound
                .checked_add(contents.len())
                .ok_or_else(|| error(owner, E::FragmentLimit))?;
        }
    }
    let leaf_bound = variant.flow().collected.items.len();
    // Set membership proves that both line graphs are already retained by the
    // set. Keep the largest history and every independently owned projection;
    // replay must not multiply the same historical ledger on each retry.
    let shared_records = set
        .record_charge()
        .max(base.prior_records())
        .max(variant.prior_records())
        .max(prior_records)
        .checked_add(base.retained_records())
        .ok_or_else(|| error(owner, E::FragmentLimit))?;
    let projection_records = (task_bound as u64)
        .checked_add(leaf_bound as u64)
        .and_then(|n| n.checked_add(3))
        .ok_or_else(|| error(owner, E::FragmentLimit))?;
    let records = shared_records
        .checked_add(variant.retained_records())
        .and_then(|n| n.checked_add(projection_records))
        .filter(|n| *n <= limits.base().get().max_fragments)
        .ok_or_else(|| error(owner, E::FragmentLimit))?;
    let mut stack = Vec::new();
    stack
        .try_reserve_exact(task_bound)
        .map_err(|_| error(owner, E::AllocationFailure))?;
    let mut leaves = Vec::new();
    leaves
        .try_reserve_exact(leaf_bound)
        .map_err(|_| error(owner, E::AllocationFailure))?;
    stack.push(Visit::Table {
        index: table_index,
        origin: Length::ZERO,
        header: true,
    });
    let mut fingerprint = sha256(b"typaxis.book-2-table-header-variant/1");
    for value in [
        set.fingerprint(),
        base.fingerprint(),
        variant.fingerprint(),
        limits.fingerprint(),
    ] {
        step(1)?;
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(&fingerprint);
        bytes[32..].copy_from_slice(&value);
        fingerprint = sha256(&bytes);
    }
    while let Some(visit) = stack.pop() {
        step(1)?;
        match visit {
            Visit::Table {
                index,
                origin,
                header,
            } => {
                let t = variant
                    .tables()
                    .get(index)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let source = variant
                    .flow()
                    .lines()
                    .prepared()
                    .source_flow()
                    .tables()
                    .get(index)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                if t.cells().len() != source.cells().len() {
                    return Err(error(t.owner(), E::ReceiptMismatch));
                }
                let offset = if header {
                    t.caption().map_or(Length::ZERO, |c| c.height())
                } else {
                    Length::ZERO
                };
                for (cell, binding) in t.cells().iter().zip(source.cells()).rev() {
                    step(1)?;
                    let row = t
                        .rows()
                        .get(binding.row() as usize)
                        .ok_or_else(|| error(cell.owner(), E::ReceiptMismatch))?;
                    if header && row.section() != ProductionTableSection::Head {
                        continue;
                    }
                    let y = origin
                        .checked_add(row.top())
                        .and_then(|v| v.checked_sub(offset))
                        .ok_or_else(|| error(cell.owner(), E::ArithmeticOverflow))?;
                    for content in cell.content().iter().rev() {
                        step(1)?;
                        stack.push(Visit::Content {
                            content,
                            cell: Some(cell.owner()),
                            origin: y,
                            owner: cell.owner(),
                        });
                    }
                }
                if !header {
                    if let Some(caption) = t.caption() {
                        for content in caption.content().iter().rev() {
                            step(1)?;
                            stack.push(Visit::Content {
                                content,
                                cell: None,
                                origin,
                                owner: t.owner(),
                            });
                        }
                    }
                }
            }
            Visit::Content {
                content,
                cell,
                origin,
                owner: content_owner,
            } => {
                let top = origin
                    .checked_add(content.top())
                    .ok_or_else(|| error(content_owner, E::ArithmeticOverflow))?;
                match content.source() {
                    ProductionTableContentSource::Table(index) => stack.push(Visit::Table {
                        index,
                        origin: top,
                        header: false,
                    }),
                    ProductionTableContentSource::FlowItem(item) => {
                        let flow_item = variant
                            .item(item)
                            .ok_or_else(|| error(content_owner, E::ReceiptMismatch))?;
                        let node = flow_item.owner();
                        let source = match flow_item.source() {
                            None => continue,
                            Some(ProductionBodyFragmentSource::ParagraphLine {
                                paragraph_index,
                                line_index,
                            }) => {
                                let p = variant
                                    .flow()
                                    .lines()
                                    .paragraphs()
                                    .get(paragraph_index as usize)
                                    .filter(|p| p.owner() == node)
                                    .ok_or_else(|| error(node, E::ReceiptMismatch))?;
                                let line = p
                                    .selected()
                                    .and_then(|s| s.lines().get(line_index as usize))
                                    .ok_or_else(|| error(node, E::ReceiptMismatch))?
                                    .line();
                                BookV2TableWidthSource::Paragraph {
                                    owner: node,
                                    units: line.start_unit()..line.end_unit(),
                                }
                            }
                            Some(_) => BookV2TableWidthSource::Block { owner: node },
                        };
                        step(1)?;
                        if leaves.len() == leaf_bound {
                            return Err(error(node, E::ReceiptMismatch));
                        }
                        leaves.push(BookV2TableHeaderVariantLeaf {
                            cell,
                            item,
                            top,
                            source,
                        });
                    }
                }
            }
        }
        if stack.len() > task_bound {
            return Err(error(owner, E::ReceiptMismatch));
        }
    }
    for values in std::iter::once([
        table_index as u64,
        height.raw() as u64,
        leaves.len() as u64,
        0,
        0,
        0,
        0,
    ])
    .chain(leaves.iter().map(|leaf| {
        let (kind, start, end) = match leaf.source() {
            BookV2TableWidthSource::Paragraph { units, .. } => (0, units.start, units.end),
            BookV2TableWidthSource::Block { .. } => (1, 0, 0),
            BookV2TableWidthSource::ForcedBreak { .. } => {
                unreachable!("forced breaks are not paint")
            }
        };
        [
            leaf.cell.map_or(u64::MAX, |n| u64::from(n.get())),
            leaf.item as u64,
            leaf.top.raw() as u64,
            u64::from(leaf.source.owner().get()),
            kind,
            u64::from(start),
            u64::from(end),
        ]
    })) {
        step(1)?;
        let mut bytes = [0u8; 88];
        bytes[..32].copy_from_slice(&fingerprint);
        for (chunk, value) in bytes[32..].chunks_exact_mut(8).zip(values) {
            chunk.copy_from_slice(&value.to_be_bytes());
        }
        fingerprint = sha256(&bytes);
    }
    Ok(BookV2TableHeaderVariant {
        base,
        variant,
        table: table_index,
        height,
        leaves,
        records,
        shared_records,
        projection_records,
        work,
        fingerprint,
        limits: limits.fingerprint(),
    })
}
