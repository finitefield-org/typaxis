//! Actual selected reference clusters bound to local measured stream items.
use super::*;
use typaxis_layout::ProductionFootnoteLineReference;

pub struct ProductionFootnoteFlowReference<'r> {
    source: &'r ProductionFootnoteLineReference,
    first_item: usize,
    last_item: usize,
}
impl<'r> ProductionFootnoteFlowReference<'r> {
    pub const fn source(&self) -> &'r ProductionFootnoteLineReference {
        self.source
    }
    /// Local body/definition item index, inclusive. The source identifies which
    /// stream, the target definition and the exact selected glyph clusters.
    pub const fn first_item_index(&self) -> usize {
        self.first_item
    }
    pub const fn last_item_index(&self) -> usize {
        self.last_item
    }
}

pub(super) fn prepare_references<'r>(
    references: &'r [ProductionFootnoteLineReference],
    collected: &CollectedItems,
    charge: &mut Charge,
) -> Result<Vec<ProductionFootnoteFlowReference<'r>>, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    charge.take(references.len(), root)?;
    let mut result: Vec<ProductionFootnoteFlowReference<'r>> = Vec::new();
    result
        .try_reserve_exact(references.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    // References are in source paragraph order. Retain the paragraph's first
    // item while joining all its references, then advance once through items.
    // No per-paragraph allocation or per-reference rescan of the book is needed.
    let mut cursor = 0usize;
    for source in references {
        let owner = source.owner();
        let first = source.first();
        let last = source.last();
        if first.paragraph_index() != last.paragraph_index()
            || first.line_index() > last.line_index()
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
        loop {
            let item = collected
                .items
                .get(cursor)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            if let Some(ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            }) = item.source
            {
                if paragraph_index as usize > first.paragraph_index() {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                if paragraph_index as usize == first.paragraph_index() {
                    if line_index != 0 {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    break;
                }
            }
            cursor += 1;
        }
        let locate = |line_index: usize| -> Result<usize, ProductionBodyPaginationError> {
            let index = cursor
                .checked_add(line_index)
                .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
            let actual = collected.items.get(index).and_then(|i| i.source);
            match actual {
                Some(ProductionBodyFragmentSource::ParagraphLine {
                    paragraph_index,
                    line_index: actual_line,
                }) if paragraph_index as usize == first.paragraph_index()
                    && actual_line as usize == line_index =>
                {
                    Ok(index)
                }
                _ => Err(error(owner, E::ReceiptMismatch)),
            }
        };
        let first_item = locate(first.line_index())?;
        let last_item = locate(last.line_index())?;
        let range = match source.source_definition() {
            None => 0..collected.body_end,
            Some(index) => collected
                .definitions
                .get(index)
                .cloned()
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?,
        };
        if !range.contains(&first_item) || !range.contains(&last_item) {
            return Err(error(owner, E::ReceiptMismatch));
        }
        if result.last().is_some_and(|previous| {
            let before = previous.source.source_definition();
            let after = source.source_definition();
            before > after
                || (before == after
                    && (previous.first_item > first_item - range.start
                        || previous.last_item > last_item - range.start))
        }) {
            return Err(error(owner, E::ReceiptMismatch));
        }
        result.push(ProductionFootnoteFlowReference {
            source,
            first_item: first_item - range.start,
            last_item: last_item - range.start,
        });
    }
    Ok(result)
}
