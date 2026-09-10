//! Shared demand snapshots and first-reference queues. Cursor types remain
//! version-specific, and wrappers check exact search/state/flow ownership.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum DemandValue<C: Copy> {
    Unreferenced,
    Pending { first_reference: NodeId, cursor: C },
    Complete { first_reference: NodeId },
}
impl<C: Copy> DemandValue<C> {
    pub(super) fn first_reference(&self) -> Option<NodeId> {
        match self {
            Self::Unreferenced => None,
            Self::Pending {
                first_reference, ..
            }
            | Self::Complete { first_reference } => Some(*first_reference),
        }
    }
}
/// This private protocol is implemented only by the two admitted content searches.
pub(super) trait DemandContent {
    type Cursor: Copy;
    fn definition_count(&self) -> usize;
    fn definition_owner(&self, index: usize) -> Result<NodeId, ProductionBodyPaginationError>;
    fn charge(&mut self, count: usize, owner: NodeId) -> Result<(), ProductionBodyPaginationError>;
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError>;
    fn begin_cursor(&mut self, index: usize)
        -> Result<Self::Cursor, ProductionBodyPaginationError>;
}
impl<'b, 'f, 's, 'p, 'a> DemandContent for ProductionFootnoteBreakSearch<'b, 'f, 's, 'p, 'a> {
    type Cursor = ProductionFootnoteCursor<'b, 'f, 's, 'p, 'a>;
    fn definition_count(&self) -> usize {
        self.flow.footnotes.definitions().len()
    }
    fn definition_owner(&self, index: usize) -> Result<NodeId, ProductionBodyPaginationError> {
        self.flow
            .footnotes
            .definitions()
            .get(index)
            .map(|d| d.owner())
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))
    }
    fn charge(&mut self, count: usize, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.charge.take(count, owner)
    }
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        visit(&mut self.steps, self.maximum_steps, owner)
    }
    fn begin_cursor(
        &mut self,
        index: usize,
    ) -> Result<Self::Cursor, ProductionBodyPaginationError> {
        self.begin(index)
    }
}
#[cfg(feature = "book-v2-staging")]
impl<'b, 'f, 's, 'p, 'a> DemandContent
    for super::super::book_v2::BookV2FootnoteBreakSearch<'b, 'f, 's, 'p, 'a>
{
    type Cursor = super::super::book_v2::BookV2FootnoteCursor<'b, 'f, 's, 'p, 'a>;
    fn definition_count(&self) -> usize {
        self.flow.footnotes().definitions().len()
    }
    fn definition_owner(&self, index: usize) -> Result<NodeId, ProductionBodyPaginationError> {
        self.flow
            .footnotes()
            .definitions()
            .get(index)
            .map(|d| d.owner())
            .ok_or_else(|| error(NodeId::new(0), E::ReceiptMismatch))
    }
    fn charge(&mut self, count: usize, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        self.charge.take(count, owner)
    }
    fn step(&mut self, owner: NodeId) -> Result<(), ProductionBodyPaginationError> {
        visit(&mut self.steps, self.maximum_steps, owner)
    }
    fn begin_cursor(
        &mut self,
        index: usize,
    ) -> Result<Self::Cursor, ProductionBodyPaginationError> {
        self.begin(index)
    }
}

pub(super) fn begin_definitions<T: DemandContent>(
    content: &mut T,
) -> Result<Vec<DemandValue<T::Cursor>>, ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    content.charge(1, root)?;
    content.charge(content.definition_count(), root)?;
    let mut definitions = Vec::new();
    definitions
        .try_reserve_exact(content.definition_count())
        .map_err(|_| error(root, E::AllocationFailure))?;
    for index in 0..content.definition_count() {
        content.step(content.definition_owner(index)?)?;
        definitions.push(DemandValue::Unreferenced);
    }
    Ok(definitions)
}
pub(super) fn fork_definitions<T: DemandContent>(
    content: &mut T,
    definitions: &[DemandValue<T::Cursor>],
    pending: &[usize],
    additional: usize,
) -> Result<(Vec<DemandValue<T::Cursor>>, Vec<usize>), ProductionBodyPaginationError> {
    let root = NodeId::new(0);
    content.charge(1, root)?;
    content.charge(definitions.len(), root)?;
    content.charge(pending.len(), root)?;
    let mut copied = Vec::new();
    copied
        .try_reserve_exact(definitions.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    for (index, demand) in definitions.iter().enumerate() {
        content.step(content.definition_owner(index)?)?;
        copied.push(*demand);
    }
    let capacity = pending
        .len()
        .checked_add(additional.min(definitions.len()))
        .ok_or_else(|| error(root, E::ArithmeticOverflow))?
        .min(definitions.len());
    let mut queue = Vec::new();
    queue
        .try_reserve_exact(capacity)
        .map_err(|_| error(root, E::AllocationFailure))?;
    for index in pending {
        content.step(content.definition_owner(*index)?)?;
        queue.push(*index);
    }
    Ok((copied, queue))
}
pub(super) fn require<T: DemandContent>(
    content: &mut T,
    definitions: &mut [DemandValue<T::Cursor>],
    pending: &mut Vec<usize>,
    references: &[ProductionFootnoteFlowReference<'_>],
) -> Result<(), ProductionBodyPaginationError> {
    for reference in references {
        let source = reference.source();
        content.step(source.owner())?;
        let index = source.definition_index();
        let slot = definitions
            .get_mut(index)
            .ok_or_else(|| error(source.owner(), E::ReceiptMismatch))?;
        if matches!(slot, DemandValue::Unreferenced) {
            let cursor = content.begin_cursor(index)?;
            content.charge(1, source.owner())?;
            *slot = DemandValue::Pending {
                first_reference: source.owner(),
                cursor,
            };
            pending.push(index);
        }
    }
    Ok(())
}
pub(super) fn advance_definition<T: DemandContent>(
    content: &mut T,
    definitions: &mut [DemandValue<T::Cursor>],
    pending: &mut Vec<usize>,
    index: usize,
    pending_position: usize,
    first_reference: NodeId,
    continuation: Option<T::Cursor>,
) -> Result<(), ProductionBodyPaginationError> {
    if let Some(cursor) = continuation {
        definitions[index] = DemandValue::Pending {
            first_reference,
            cursor,
        };
    } else {
        definitions[index] = DemandValue::Complete { first_reference };
        for index in pending.iter().skip(pending_position + 1) {
            content.step(content.definition_owner(*index)?)?;
        }
        pending.remove(pending_position);
    }
    Ok(())
}

/// Compare projected records under the same visited-work owner as selection.
pub(super) fn same_records<C: DemandContent, T: PartialEq>(
    content: &mut C,
    left: &[T],
    right: &[T],
) -> Result<bool, ProductionBodyPaginationError> {
    if left.len() != right.len() {
        return Ok(false);
    }
    for (l, r) in left.iter().zip(right) {
        content.step(NodeId::new(0))?;
        if l != r {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) fn same_demand<C: DemandContent, K: PartialEq>(
    content: &mut C,
    left_pending: &[usize],
    right_pending: &[usize],
    left: &[DemandValue<C::Cursor>],
    right: &[DemandValue<C::Cursor>],
    cursor_key: impl Fn(C::Cursor) -> K,
) -> Result<bool, ProductionBodyPaginationError> {
    if !same_records(content, left_pending, right_pending)? || left.len() != right.len() {
        return Ok(false);
    }
    for (l, r) in left.iter().zip(right) {
        content.step(NodeId::new(0))?;
        let equal = match (l, r) {
            (DemandValue::Unreferenced, DemandValue::Unreferenced) => true,
            (
                DemandValue::Complete { first_reference: l },
                DemandValue::Complete { first_reference: r },
            ) => l == r,
            (
                DemandValue::Pending {
                    first_reference: l,
                    cursor: lc,
                },
                DemandValue::Pending {
                    first_reference: r,
                    cursor: rc,
                },
            ) => l == r && cursor_key(*lc) == cursor_key(*rc),
            _ => false,
        };
        if !equal {
            return Ok(false);
        }
    }
    Ok(true)
}
