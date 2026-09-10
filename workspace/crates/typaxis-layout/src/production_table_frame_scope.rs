//! Source ancestors required by one actual table occurrence.
use super::*;

pub(super) struct SourceScope {
    active: Vec<bool>,
    root: std::ops::Range<usize>,
}
impl SourceScope {
    /// Storage and traversal are prepaid by the full frame projection caller.
    pub(super) fn new(
        flow: InlineFlow<'_>,
        root: NodeId,
        owners: &[NodeId],
    ) -> Result<Self, ProductionInlinePreparationError> {
        use ProductionInlinePreparationErrorKind as E;
        let events = flow_call!(flow, events());
        if owners.len() > events.len() || owners.windows(2).any(|w| w[0] >= w[1]) {
            return Err(error(root, E::ReceiptMismatch));
        }
        let mut active = Vec::new();
        active
            .try_reserve_exact(events.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        active.resize(events.len(), false);
        let mut stack = Vec::new();
        stack
            .try_reserve_exact(events.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        let (mut range, mut inside, mut seen) = (None, false, 0usize);
        for (index, event) in events.iter().enumerate() {
            match *event {
                Event::Begin { owner, kind } => {
                    if owner == root {
                        if inside || range.is_some() || kind != Region::Table {
                            return Err(error(root, E::ReceiptMismatch));
                        }
                        range = Some(index..events.len());
                        inside = true;
                    }
                    if owners.binary_search(&owner).is_ok() {
                        let leaf = matches!(
                            kind,
                            Region::Paragraph
                                | Region::Heading
                                | Region::Figure
                                | Region::VectorFigure
                                | Region::DisplayMath
                                | Region::MathVectorBlock
                                | Region::PageBreak
                        );
                        #[cfg(feature = "book-v2-staging")]
                        let leaf = leaf || kind == Region::DescriptionTerm;
                        if !inside || !leaf {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                        active[index] = true;
                        seen += 1;
                    }
                    stack.push((owner, index));
                }
                Event::End { owner } => {
                    let (_, begin) = stack
                        .pop()
                        .filter(|s| s.0 == owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if let Some(&(_, parent)) = stack.last() {
                        active[parent] |= active[begin];
                    }
                    if owner == root {
                        range
                            .as_mut()
                            .ok_or_else(|| error(root, E::ReceiptMismatch))?
                            .end = index;
                        inside = false;
                    }
                }
                Event::Paragraph { .. } => {}
            }
        }
        if inside || !stack.is_empty() || seen != owners.len() {
            return Err(error(root, E::ReceiptMismatch));
        }
        Ok(Self {
            active,
            root: range.ok_or_else(|| error(root, E::ReceiptMismatch))?,
        })
    }
    pub(super) fn preserve(&self, event: usize) -> bool {
        event > self.root.start && event < self.root.end && !self.active[event]
    }
}
