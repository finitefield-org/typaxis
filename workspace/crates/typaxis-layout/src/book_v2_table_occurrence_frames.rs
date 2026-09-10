//! Reproject one original root table without replacing its measurement frames.
use super::*;
use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};

pub struct BookV2TableOccurrenceFrames<'f, 'p, 'a> {
    source: &'f BookV2BodyInlineFrames<'p, 'a>,
    owners: Option<&'f [NodeId]>,
    owner: NodeId,
    width: PositiveLength,
    projection: list_frames::FrameProjection,
    records: u64,
    work: u64,
}
impl BookV2TableOccurrenceFrames<'_, '_, '_> {
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn parent_width(&self) -> PositiveLength {
        self.width
    }
    /// Full provisional storage. Use paragraph() for source-scoped queries.
    pub fn paragraphs(&self) -> &[ProductionInlineFrame] {
        &self.projection.paragraphs
    }
    pub fn region(&self, owner: NodeId) -> Option<ProductionInlineFrame> {
        if !self.covers_owner(owner) {
            return None;
        }
        self.projection.regions.get(&owner).copied()
    }
    pub fn covers_owner(&self, owner: NodeId) -> bool {
        self.owners
            .is_none_or(|owners| owners.binary_search(&owner).is_ok())
    }
    pub fn source_owner_lookup_work(&self) -> u64 {
        self.owners.map_or(0, |owners| {
            u64::from(owners.len().checked_ilog2().unwrap_or(0)) + 1
        })
    }
    pub fn paragraph(&self, index: usize, owner: NodeId) -> Option<ProductionInlineFrame> {
        if !self.covers_owner(owner)
            || self.source.prepared.flow.paragraphs().get(index)?.owner() != owner
        {
            return None;
        }
        self.projection.paragraphs.get(index).copied()
    }
    pub fn tables(&self) -> &[ProductionTableFrame] {
        &self.projection.tables
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.projection.fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }
    pub fn region_lookup_work(&self) -> u64 {
        u64::from(self.projection.regions.len().checked_ilog2().unwrap_or(0))
            + 1
            + self.source_owner_lookup_work()
    }
    pub fn verify(
        &self,
        frames: &BookV2BodyInlineFrames<'_, '_>,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.source, frames) {
            return Err(error(
                self.owner,
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}

impl<'p, 'a> BookV2BodyInlineFrames<'p, 'a> {
    /// Additional conservative records/work to precharge before an occurrence
    /// projection, including its temporary full-source hierarchy and root list.
    pub fn table_occurrence_projection_budget(
        &self,
    ) -> Result<(u64, u64), ProductionInlinePreparationError> {
        use ProductionInlinePreparationErrorKind as E;
        let root = NodeId::new(0);
        let original = self.table_measurements.as_ref().unwrap_or(&self.projection);
        let records = original
            .record_charge
            .checked_add(self.prepared.flow.tables().len() as u64)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| error(root, E::UnitLimit))?;
        let depth = u64::from(
            self.prepared
                .flow
                .events()
                .len()
                .checked_ilog2()
                .unwrap_or(0),
        ) + 1;
        let work = records
            .checked_mul(64)
            .and_then(|n| n.checked_mul(depth))
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        Ok((records, work))
    }
    /// Charge the source-ancestor bitmap/stack before selective projection.
    pub fn table_source_occurrence_projection_budget(
        &self,
    ) -> Result<(u64, u64), ProductionInlinePreparationError> {
        let (records, work) = self.table_occurrence_projection_budget()?;
        let events = self.prepared.flow.events().len() as u64;
        let extra = events.checked_mul(2).ok_or_else(|| {
            error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::UnitLimit,
            )
        })?;
        let depth = u64::from(
            self.prepared
                .flow
                .events()
                .len()
                .checked_ilog2()
                .unwrap_or(0),
        ) + 1;
        let records = records.checked_add(extra).ok_or_else(|| {
            error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::UnitLimit,
            )
        })?;
        let work = extra
            .checked_mul(8)
            .and_then(|n| n.checked_mul(depth))
            .and_then(|n| n.checked_add(work))
            .ok_or_else(|| {
                error(
                    NodeId::new(0),
                    ProductionInlinePreparationErrorKind::ArithmeticOverflow,
                )
            })?;
        Ok((records, work))
    }
    /// Remeasure one root at an observed parent width. The result retains source
    /// hierarchy and original markers; it is provisional geometry, not a page
    /// receipt. Other roots use their original measurement widths. Repeated
    /// occurrences can therefore request distinct frames for the same source.
    pub fn remeasure_table_parent(
        &self,
        owner: NodeId,
        width: PositiveLength,
        maximum_work: u64,
        prior_records: u64,
    ) -> Result<BookV2TableOccurrenceFrames<'_, 'p, 'a>, ProductionInlinePreparationError> {
        self.remeasure_table_parent_scoped(owner, width, maximum_work, prior_records, None)
    }
    /// Remeasure just the source leaves observed in one physical occurrence and
    /// all their ancestors. Sorted unique owners must be inside the given root.
    /// Unobserved branches retain the original envelope and are not validated
    /// at this width. The paragraph/region leaf accessors enforce membership.
    pub fn remeasure_table_parent_for_sources<'f>(
        &'f self,
        owner: NodeId,
        width: PositiveLength,
        owners: &'f [NodeId],
        maximum_work: u64,
        prior_records: u64,
    ) -> Result<BookV2TableOccurrenceFrames<'f, 'p, 'a>, ProductionInlinePreparationError> {
        self.remeasure_table_parent_scoped(owner, width, maximum_work, prior_records, Some(owners))
    }
    fn remeasure_table_parent_scoped<'f>(
        &'f self,
        owner: NodeId,
        width: PositiveLength,
        maximum_work: u64,
        prior_records: u64,
        owners: Option<&'f [NodeId]>,
    ) -> Result<BookV2TableOccurrenceFrames<'f, 'p, 'a>, ProductionInlinePreparationError> {
        use ProductionInlinePreparationErrorKind as E;
        let (additional, work) = if owners.is_some() {
            self.table_source_occurrence_projection_budget()?
        } else {
            self.table_occurrence_projection_budget()?
        };
        if maximum_work < work {
            return Err(error(
                owner,
                E::Atomic(typaxis_linebreak::AtomicVectorInlineError::CandidateLimit),
            ));
        }
        let records = prior_records
            .checked_add(additional)
            .filter(|n| *n <= self.prepared.max_fragments && prior_records >= self.record_charge())
            .ok_or_else(|| error(owner, E::UnitLimit))?;
        let count = self.prepared.flow.tables().len();
        let mut widths = Vec::new();
        widths
            .try_reserve_exact(count)
            .map_err(|_| error(owner, E::AllocationFailure))?;
        let mut depth = 0usize;
        let mut table_depth = None;
        let mut found = false;
        for event in self.prepared.flow.events() {
            match *event {
                Event::Begin {
                    owner: current,
                    kind,
                } => {
                    depth = depth
                        .checked_add(1)
                        .ok_or_else(|| error(current, E::ArithmeticOverflow))?;
                    if kind == Region::Table && table_depth.is_none() {
                        table_depth = Some(depth);
                        let original = self
                            .measurement_region(current)
                            .ok_or_else(|| error(current, E::ReceiptMismatch))?;
                        let candidate = if current == owner {
                            if found {
                                return Err(error(owner, E::ReceiptMismatch));
                            }
                            found = true;
                            width
                        } else {
                            original.width()
                        };
                        widths.push((current, candidate));
                    }
                }
                Event::End { owner: current } => {
                    if table_depth == Some(depth) {
                        table_depth = None;
                    }
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| error(current, E::ReceiptMismatch))?;
                }
                _ => {}
            }
        }
        if !found || depth != 0 || table_depth.is_some() {
            return Err(error(owner, E::ReceiptMismatch));
        }
        let projection = list_frames::project_frames_with_root_table_widths(
            InlineFlow::BookV2(self.prepared.flow),
            self.prepared.shaped.list_markers(),
            self.prepared.shaped.footnote_markers(),
            additional - count as u64 - 1,
            self.prepared.fingerprint(),
            self.body(),
            self.footnote_region(),
            if owners.is_some() {
                "typaxis.book-2-table-source-occurrence-frames/1"
            } else {
                "typaxis.book-2-table-occurrence-frames/1"
            },
            &widths,
            owners.map(|_| {
                (
                    owner,
                    self.table_measurements.as_ref().unwrap_or(&self.projection),
                )
            }),
            owners,
        )?;
        // Include the selected root even when two no-op projections happen to
        // have identical full-source geometry.
        let mut projection = projection;
        let mut digest = [0; 44];
        digest[..32].copy_from_slice(&projection.fingerprint);
        digest[32..36].copy_from_slice(&owner.get().to_be_bytes());
        digest[36..].copy_from_slice(&width.get().raw().to_be_bytes());
        projection.fingerprint = typaxis_core::sha256(&digest);
        Ok(BookV2TableOccurrenceFrames {
            source: self,
            owners,
            owner,
            width,
            projection,
            records,
            work,
        })
    }
}
