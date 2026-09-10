//! Source-position origins that survive fresh shaping and line selection.
use super::*;
use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};
impl BookV2BodyInlineFrames<'_, '_> {
    pub fn uses_table_occurrence_frames(&self) -> bool {
        self.occurrence_frames
    }
    pub fn has_source_unit_starts(&self) -> bool {
        self.source_unit_starts.is_some()
    }
    pub fn source_unit_start(&self, paragraph: usize, unit: u32) -> Option<Length> {
        self.source_unit_starts
            .as_ref()?
            .get(paragraph)?
            .as_ref()?
            .get(unit as usize)
            .copied()
    }
    pub(super) fn apply_source_unit_starts(
        &mut self,
        assignments: &BookV2SourceWidthAssignments<'_, '_>,
        maximum_work: u64,
    ) -> Result<u64, ProductionInlinePreparationError> {
        use ProductionInlinePreparationErrorKind as E;
        let root = NodeId::new(0);
        if !assignments.matches_flow(self.prepared.flow) {
            return Err(error(root, E::ReceiptMismatch));
        }
        let Some(starts) = assignments.source_unit_starts() else {
            if assignments.uses_table_occurrence_frames() {
                return Err(error(root, E::ReceiptMismatch));
            }
            return Ok(0);
        };
        if starts.len() != self.prepared.paragraphs.len() {
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
        let mut records = self
            .record_charge()
            .checked_add(starts.len() as u64)
            .and_then(|n| n.checked_add(1))
            .filter(|n| *n <= self.prepared.max_fragments)
            .ok_or_else(|| error(root, E::UnitLimit))?;
        for (index, (p, values)) in self.prepared.paragraphs.iter().zip(starts).enumerate() {
            step(&mut work, p.owner())?;
            if let Some(values) = values {
                let count = p
                    .items()
                    .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?
                    .units()
                    .len()
                    .max(1);
                if values.len() != count
                    || assignments.paragraph_widths()[index]
                        .is_none_or(|widths| widths.len() != count)
                {
                    return Err(error(p.owner(), E::ReceiptMismatch));
                }
                records = records
                    .checked_add(count as u64)
                    .filter(|n| *n <= self.prepared.max_fragments)
                    .ok_or_else(|| error(p.owner(), E::UnitLimit))?;
            }
        }
        let mut retained = Vec::new();
        retained
            .try_reserve_exact(starts.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut note = None;
        let mut note_index = 0;
        let mut left = Length::ZERO;
        let mut right = self.body().width().get();
        let mut hash = self.fingerprint();
        let mut tag = [0; 64];
        tag[..32].copy_from_slice(&hash);
        tag[32..].copy_from_slice(&typaxis_core::sha256(
            b"typaxis.book-2-source-unit-starts/1",
        ));
        hash = typaxis_core::sha256(&tag);
        if assignments.uses_table_occurrence_frames() {
            step(&mut work, root)?;
            tag[..32].copy_from_slice(&hash);
            tag[32..].copy_from_slice(&typaxis_core::sha256(
                b"typaxis.book-2-table-source-profiles/1",
            ));
            hash = typaxis_core::sha256(&tag);
        }
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
                Event::Paragraph { index } => {
                    if index as usize != retained.len() {
                        return Err(error(root, E::ReceiptMismatch));
                    }
                    let p = &self.prepared.paragraphs[index as usize];
                    let mut entry = None;
                    let mut header = [0; 37];
                    header[..32].copy_from_slice(&hash);
                    header[32..36].copy_from_slice(&p.owner().get().to_be_bytes());
                    header[36] = u8::from(starts[index as usize].is_some());
                    hash = typaxis_core::sha256(&header);
                    if let Some(values) = starts[index as usize] {
                        let widths = assignments.paragraph_widths()[index as usize].unwrap();
                        let mut copy = Vec::new();
                        copy.try_reserve_exact(values.len())
                            .map_err(|_| error(p.owner(), E::AllocationFailure))?;
                        for (&start, &width) in values.iter().zip(widths) {
                            step(&mut work, p.owner())?;
                            let end = start
                                .checked_add(width.get())
                                .ok_or_else(|| error(p.owner(), E::ArithmeticOverflow))?;
                            if start < left || end > right {
                                return Err(error(p.owner(), E::InvalidHorizontalMetrics));
                            }
                            let mut digest = [0; 48];
                            digest[..32].copy_from_slice(&hash);
                            digest[32..40].copy_from_slice(&start.raw().to_be_bytes());
                            digest[40..].copy_from_slice(&width.get().raw().to_be_bytes());
                            hash = typaxis_core::sha256(&digest);
                            copy.push(start);
                        }
                        entry = Some(copy);
                    }
                    retained.push(entry);
                }
                _ => {}
            }
        }
        if retained.len() != starts.len() || note.is_some() {
            return Err(error(root, E::ReceiptMismatch));
        }
        self.occurrence_frames = assignments.uses_table_occurrence_frames();
        self.source_unit_starts = Some(retained);
        self.projection.record_charge = records;
        self.projection.fingerprint = hash;
        Ok(work)
    }
}
