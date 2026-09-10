//! Reproject the exact source hierarchy using root-table parent width candidates.
use super::*;

impl BookV2BodyInlineFrames<'_, '_> {
    pub(super) fn apply_table_widths(
        &mut self,
        assignments: &BookV2SourceWidthAssignments<'_, '_>,
        maximum_work: u64,
    ) -> Result<u64, ProductionInlinePreparationError> {
        use ProductionInlinePreparationErrorKind as E;
        let root = NodeId::new(0);
        if !assignments.matches_flow(self.prepared.flow) {
            return Err(error(root, E::ReceiptMismatch));
        }
        let widths = assignments.root_table_widths();
        if widths.is_empty() {
            if assignments.table_header_scope().is_some() {
                return Err(error(root, E::ReceiptMismatch));
            }
            return Ok(0);
        }
        // Keep the original envelope projection alive alongside the rebound one.
        // Its complete record bound includes traversal scratch, column offsets,
        // marker summaries and fingerprint storage. Check both before allocation.
        let retained = self.projection.record_charge;
        let scratch = if assignments.table_source_owners().is_some() {
            (self.prepared.flow.events().len() as u64)
                .checked_mul(2)
                .ok_or_else(|| error(root, E::UnitLimit))?
        } else {
            0
        };
        let projected = retained
            .checked_add(scratch)
            .ok_or_else(|| error(root, E::UnitLimit))?;
        let remaining_records = self
            .prepared
            .max_fragments
            .checked_sub(retained)
            .filter(|remaining| *remaining >= projected)
            .ok_or_else(|| error(root, E::UnitLimit))?;
        // Conservative prepaid traversal bound: records cover all events,
        // columns and glyph/marker entries; 64 covers frame encoding and each
        // linear resolver pass, with a logarithmic allowance for owner lookups.
        let lookup = u64::from(
            self.prepared
                .flow
                .events()
                .len()
                .checked_ilog2()
                .unwrap_or(0),
        ) + 1;
        let work = projected
            .checked_mul(if assignments.table_header_scope().is_some() {
                96
            } else {
                64
            })
            .and_then(|n| n.checked_mul(lookup))
            .filter(|n| *n <= maximum_work)
            .ok_or_else(|| {
                error(
                    root,
                    E::Atomic(typaxis_linebreak::AtomicVectorInlineError::CandidateLimit),
                )
            })?;
        let mut rebound = list_frames::project_frames_with_root_table_widths(
            InlineFlow::BookV2(self.prepared.flow),
            self.prepared.shaped.list_markers(),
            self.prepared.shaped.footnote_markers(),
            remaining_records,
            self.prepared.fingerprint(),
            self.body(),
            self.footnote_region(),
            if assignments.table_source_owners().is_some() {
                "typaxis.book-2-table-source-width-frames/1"
            } else if assignments.table_header_scope().is_some() {
                "typaxis.book-2-table-header-width-frames/1"
            } else {
                "typaxis.book-2-table-width-frames/1"
            },
            widths,
            assignments
                .table_header_scope()
                .map(|owner| (owner, &self.projection)),
            assignments.table_source_owners(),
        )?;
        rebound.record_charge = rebound
            .record_charge
            .checked_add(retained)
            .filter(|n| *n <= self.prepared.max_fragments)
            .ok_or_else(|| error(root, E::UnitLimit))?;
        self.table_measurements = Some(std::mem::replace(&mut self.projection, rebound));
        Ok(work)
    }
}
