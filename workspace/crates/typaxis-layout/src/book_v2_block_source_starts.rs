//! Original block parent origins, paired with explicit source-bound widths.
use super::*;
use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};

impl BookV2BodyInlineFrames<'_, '_> {
    pub fn has_block_source_starts(&self) -> bool {
        self.block_starts
    }
    pub(super) fn apply_block_starts(
        &mut self,
        assignments: &BookV2SourceWidthAssignments<'_, '_>,
        maximum_work: u64,
    ) -> Result<u64, ProductionInlinePreparationError> {
        use ProductionInlinePreparationErrorKind as E;
        let root = NodeId::new(0);
        let Some(starts) = assignments.block_starts() else {
            return Ok(0);
        };
        if !assignments.matches_flow(self.prepared.flow)
            || starts.len() != assignments.block_widths().len()
            || assignments.inherits_table_blocks()
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        let mut work = 0u64;
        let step = |work: &mut u64, owner| {
            *work = work
                .checked_add(1)
                .filter(|n| *n <= maximum_work)
                .ok_or_else(|| {
                    error(
                        owner,
                        E::Atomic(typaxis_linebreak::AtomicVectorInlineError::CandidateLimit),
                    )
                })?;
            Ok::<_, ProductionInlinePreparationError>(())
        };
        let mut note = None;
        let mut note_index = 0;
        let mut left = Length::ZERO;
        let mut right = self.body().width().get();
        let mut index = 0;
        let lookup = u64::from(self.projection.regions.len().checked_ilog2().unwrap_or(0)) + 1;
        for event in self.prepared.flow.events() {
            step(&mut work, root)?;
            match *event {
                Event::Begin {
                    owner,
                    kind: Region::Footnote,
                } => {
                    if note.is_some() {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    let frame = self
                        .footnotes()
                        .get(note_index)
                        .filter(|f| f.owner() == owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    note_index += 1;
                    note = Some(owner);
                    left = frame.content().start();
                    right = left
                        .checked_add(frame.content().width().get())
                        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                }
                Event::End { owner } if note == Some(owner) => {
                    note = None;
                    left = Length::ZERO;
                    right = self.body().width().get();
                }
                Event::Begin {
                    owner,
                    kind:
                        Region::Figure
                        | Region::VectorFigure
                        | Region::DisplayMath
                        | Region::MathVectorBlock,
                } => {
                    let &(expected, width) = assignments
                        .block_widths()
                        .get(index)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if expected != owner {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    let start = starts[index];
                    let end = start
                        .checked_add(width.get())
                        .ok_or_else(|| error(owner, E::ArithmeticOverflow))?;
                    if start < left || end > right {
                        return Err(error(owner, E::InvalidHorizontalMetrics));
                    }
                    for _ in 0..lookup {
                        step(&mut work, owner)?;
                    }
                    let frame = self
                        .projection
                        .regions
                        .get_mut(&owner)
                        .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                    if frame.width() != width {
                        return Err(error(owner, E::ReceiptMismatch));
                    }
                    *frame = frame.with_start(start);
                    let mut digest = [0u8; 76];
                    digest[..32].copy_from_slice(&self.projection.fingerprint);
                    digest[32..64]
                        .copy_from_slice(&sha256(b"typaxis.book-2-block-source-starts/1"));
                    digest[64..68].copy_from_slice(&owner.get().to_be_bytes());
                    digest[68..].copy_from_slice(&start.raw().to_be_bytes());
                    self.projection.fingerprint = sha256(&digest);
                    index += 1;
                }
                _ => {}
            }
        }
        if index != starts.len() || note.is_some() {
            return Err(error(root, E::ReceiptMismatch));
        }
        self.block_starts = !starts.is_empty();
        Ok(work)
    }
}
