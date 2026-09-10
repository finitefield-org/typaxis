//! Measured body frames retained by the exact book-2 inline owner.
use super::*;
use typaxis_core::Rect;
use typaxis_syntax::book_v2::BookV2PageFramePlan;

pub const BOOK_V2_BODY_FRAMES_ALGORITHM: &str = "typaxis.book-2-body-frames/1";

pub struct BookV2BodyInlineFrames<'p, 'a> {
    prepared: &'p BookV2PreparedInlines<'a>,
    projection: list_frames::FrameProjection,
    page_plan: Option<&'p BookV2PageFramePlan<'a>>,
    block_measurements: Vec<(NodeId, ProductionInlineFrame)>,
    source_unit_starts: Option<Vec<Option<Vec<Length>>>>,
    occurrence_frames: bool,
    block_starts: bool,
    table_measurements: Option<list_frames::FrameProjection>,
}
impl<'p, 'a> BookV2BodyInlineFrames<'p, 'a> {
    pub fn has_table_width_candidates(&self) -> bool {
        self.table_measurements.is_some()
    }
    pub fn measurement_tables(&self) -> &[ProductionTableFrame] {
        self.table_measurements
            .as_ref()
            .map_or(self.tables(), |p| &p.tables)
    }
    pub fn measurement_paragraphs(&self) -> &[ProductionInlineFrame] {
        self.table_measurements
            .as_ref()
            .map_or(self.paragraphs(), |p| &p.paragraphs)
    }
    pub fn has_block_width_candidates(&self) -> bool {
        !self.block_measurements.is_empty()
    }
    pub fn page_plan(&self) -> Option<&'p BookV2PageFramePlan<'a>> {
        self.page_plan
    }
    pub fn body(&self) -> Rect {
        self.projection.body
    }
    /// Declared maximum region; page selection has not yet occurred.
    pub fn footnote_region(&self) -> Option<Rect> {
        self.projection.footnote_region
    }
    pub fn region(&self, owner: NodeId) -> Option<ProductionInlineFrame> {
        self.projection.regions.get(&owner).copied()
    }
    /// Original maximum-envelope parent frame, before any width candidate.
    pub fn measurement_region(&self, owner: NodeId) -> Option<ProductionInlineFrame> {
        if let Some(original) = &self.table_measurements {
            return original.regions.get(&owner).copied();
        }
        self.block_measurements
            .binary_search_by_key(&owner, |(id, _)| *id)
            .ok()
            .map(|index| self.block_measurements[index].1)
            .or_else(|| self.region(owner))
    }
    /// Frame after hierarchy projection and before explicit block overrides.
    pub fn inherited_block_region(&self, owner: NodeId) -> Option<ProductionInlineFrame> {
        self.block_measurements
            .binary_search_by_key(&owner, |(id, _)| *id)
            .ok()
            .map(|index| self.block_measurements[index].1)
            .or_else(|| self.region(owner))
    }
    pub fn inherited_block_lookup_work(&self) -> u64 {
        u64::from(self.block_measurements.len().checked_ilog2().unwrap_or(0))
            + u64::from(self.projection.regions.len().checked_ilog2().unwrap_or(0))
            + 2
    }
    pub fn measurement_region_lookup_work(&self) -> u64 {
        let depth = |count: usize| u64::from(count.checked_ilog2().unwrap_or(0)) + 1;
        if let Some(original) = &self.table_measurements {
            depth(original.regions.len())
        } else {
            // A non-block owner, or the first pass without overrides, falls
            // through the block registry to the full source-region map.
            depth(self.block_measurements.len()) + depth(self.projection.regions.len())
        }
    }
    pub fn paragraphs(&self) -> &[ProductionInlineFrame] {
        &self.projection.paragraphs
    }
    pub fn lists(&self) -> &[ProductionListFrame] {
        &self.projection.lists
    }
    pub fn tables(&self) -> &[ProductionTableFrame] {
        &self.projection.tables
    }
    pub fn footnotes(&self) -> &[ProductionFootnoteFrame] {
        &self.projection.footnotes
    }
    pub fn record_charge(&self) -> u64 {
        self.projection.record_charge
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.projection.fingerprint
    }
    pub fn verify(
        &self,
        prepared: &BookV2PreparedInlines<'_>,
        body: Rect,
    ) -> Result<(), ProductionInlinePreparationError> {
        if !std::ptr::eq(self.prepared, prepared) || self.body() != body {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(())
    }
}
pub fn prepare_book_v2_body_inline_frames<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    body: Rect,
) -> Result<BookV2BodyInlineFrames<'p, 'a>, ProductionInlinePreparationError> {
    let projection = list_frames::project_frames(
        InlineFlow::BookV2(prepared.flow),
        prepared.shaped.list_markers(),
        prepared.shaped.footnote_markers(),
        prepared.max_fragments,
        prepared.fingerprint(),
        body,
        BOOK_V2_BODY_FRAMES_ALGORITHM,
    )?;
    Ok(BookV2BodyInlineFrames {
        prepared,
        projection,
        page_plan: None,
        block_measurements: Vec::new(),
        source_unit_starts: None,
        occurrence_frames: false,
        block_starts: false,
        table_measurements: None,
    })
}
pub fn layout_book_v2_body_inline_lines<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    body: Rect,
    max_candidate_steps: u64,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    layout_book_v2_body_inline_lines_in_pages(prepared, body, max_candidate_steps, None)
}
pub(super) fn layout_book_v2_body_inline_lines_in_pages<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    body: Rect,
    max_candidate_steps: u64,
    page_plan: Option<&'p BookV2PageFramePlan<'a>>,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    layout_body_lines_with_source_widths(prepared, body, max_candidate_steps, page_plan, None)
}
pub(super) fn layout_body_lines_with_source_widths<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    body: Rect,
    max_candidate_steps: u64,
    page_plan: Option<&'p BookV2PageFramePlan<'a>>,
    source_widths: Option<&BookV2SourceWidthAssignments<'_, '_>>,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    let mut frames = if let Some(plan) = page_plan {
        if !std::ptr::eq(plan.source(), prepared.flow.body()) || body != plan.measurement_body() {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        BookV2BodyInlineFrames {
            prepared,
            projection: list_frames::project_frames_in_regions(
                InlineFlow::BookV2(prepared.flow),
                prepared.shaped.list_markers(),
                prepared.shaped.footnote_markers(),
                prepared.max_fragments,
                prepared.fingerprint(),
                body,
                plan.measurement_footnote(),
                BOOK_V2_BODY_FRAMES_ALGORITHM,
            )?,
            page_plan: Some(plan),
            block_measurements: Vec::new(),
            source_unit_starts: None,
            occurrence_frames: false,
            block_starts: false,
            table_measurements: None,
        }
    } else {
        prepare_book_v2_body_inline_frames(prepared, body)?
    };
    let frame_work = if let Some(assignments) = source_widths {
        let table_work = frames.apply_table_widths(assignments, max_candidate_steps)?;
        let block_work =
            frames.apply_block_widths(assignments, max_candidate_steps - table_work)?;
        let block_start_work = frames
            .apply_block_starts(assignments, max_candidate_steps - table_work - block_work)?;
        table_work
            + block_work
            + block_start_work
            + frames.apply_source_unit_starts(
                assignments,
                max_candidate_steps - table_work - block_work - block_start_work,
            )?
    } else {
        0
    };
    let remaining_work = max_candidate_steps.checked_sub(frame_work).ok_or_else(|| {
        error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::UnitLimit,
        )
    })?;
    let mut widths = Vec::new();
    widths
        .try_reserve_exact(frames.paragraphs().len())
        .map_err(|_| {
            error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::AllocationFailure,
            )
        })?;
    widths.extend(frames.paragraphs().iter().map(|f| f.width()));
    let mut lines = if let Some(assignments) = source_widths {
        super::source_widths::layout_source_width_lines_from_flow(
            prepared,
            &widths,
            assignments,
            remaining_work,
            frames.record_charge(),
        )?
    } else {
        layout_with_record_base(prepared, &widths, remaining_work, frames.record_charge())?
    };
    lines.projection.candidate_steps = lines
        .projection
        .candidate_steps
        .checked_add(frame_work)
        .ok_or_else(|| {
            error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ArithmeticOverflow,
            )
        })?;
    let mut digest = [0; 64];
    digest[..32].copy_from_slice(&lines.fingerprint());
    digest[32..].copy_from_slice(&frames.fingerprint());
    lines.projection.fingerprint = sha256(&digest);
    lines.frames = Some(frames);
    Ok(lines)
}

impl BookV2BodyInlineFrames<'_, '_> {
    fn apply_block_widths(
        &mut self,
        assignments: &BookV2SourceWidthAssignments<'_, '_>,
        maximum_work: u64,
    ) -> Result<u64, ProductionInlinePreparationError> {
        use typaxis_syntax::{ProductionFlowEvent as Event, ProductionFlowRegionKind as Region};
        use ProductionInlinePreparationErrorKind as E;
        let root = NodeId::new(0);
        if !assignments.matches_flow(self.prepared.flow) {
            return Err(error(root, E::ReceiptMismatch));
        }
        let widths = assignments.block_widths();
        if widths.is_empty() {
            return Ok(0);
        }
        self.projection.record_charge = self
            .projection
            .record_charge
            .checked_add(widths.len() as u64)
            .filter(|n| *n <= self.prepared.max_fragments)
            .ok_or_else(|| error(root, E::UnitLimit))?;
        self.block_measurements
            .try_reserve_exact(widths.len())
            .map_err(|_| error(root, E::AllocationFailure))?;
        let mut work = 0u64;
        let mut index = 0;
        let mut depth = 0usize;
        let mut table_depth = None;
        for event in self.prepared.flow.events() {
            work = work
                .checked_add(1)
                .filter(|n| *n <= maximum_work)
                .ok_or_else(|| {
                    error(
                        root,
                        E::Atomic(typaxis_linebreak::AtomicVectorInlineError::CandidateLimit),
                    )
                })?;
            if assignments.inherits_table_blocks() {
                match *event {
                    Event::Begin { kind, .. } => {
                        depth = depth
                            .checked_add(1)
                            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
                        if kind == Region::Table && table_depth.is_none() {
                            table_depth = Some(depth);
                        }
                    }
                    Event::End { .. } => {
                        if table_depth == Some(depth) {
                            table_depth = None;
                        }
                        depth = depth
                            .checked_sub(1)
                            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                    }
                    _ => {}
                }
            }
            let Event::Begin {
                owner,
                kind:
                    Region::Figure
                    | Region::VectorFigure
                    | Region::DisplayMath
                    | Region::MathVectorBlock,
            } = *event
            else {
                continue;
            };
            let Some(&(expected, width)) = widths.get(index) else {
                return Err(error(owner, E::ReceiptMismatch));
            };
            if owner != expected {
                return Err(error(owner, E::ReceiptMismatch));
            }
            let current = self
                .projection
                .regions
                .get_mut(&owner)
                .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
            let width = if assignments.inherits_table_blocks() && table_depth.is_some() {
                current.width()
            } else {
                width
            };
            if width.get() > current.width().get() {
                return Err(error(owner, E::InvalidHorizontalMetrics));
            }
            self.block_measurements.push((owner, *current));
            *current = current.with_width(width);
            let mut digest = [0u8; 76];
            digest[..32].copy_from_slice(&self.projection.fingerprint);
            digest[32..64].copy_from_slice(&typaxis_core::sha256(
                b"typaxis.book-2-block-width-frames/1",
            ));
            digest[64..68].copy_from_slice(&owner.get().to_be_bytes());
            digest[68..].copy_from_slice(&width.get().raw().to_be_bytes());
            self.projection.fingerprint = sha256(&digest);
            index += 1;
        }
        if index != widths.len() {
            return Err(error(root, E::ReceiptMismatch));
        }
        let sort_work = (widths.len() as u64)
            .checked_mul(self.measurement_region_lookup_work())
            .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
        work = work
            .checked_add(sort_work)
            .filter(|n| *n <= maximum_work)
            .ok_or_else(|| {
                error(
                    root,
                    E::Atomic(typaxis_linebreak::AtomicVectorInlineError::CandidateLimit),
                )
            })?;
        self.block_measurements
            .sort_unstable_by_key(|(owner, _)| *owner);
        Ok(work)
    }
}

#[path = "book_v2_table_width_frames.rs"]
mod table_width_frames;

#[path = "book_v2_table_occurrence_frames.rs"]
mod table_occurrence_frames;
pub use table_occurrence_frames::BookV2TableOccurrenceFrames;

#[path = "book_v2_source_unit_starts.rs"]
mod source_unit_starts;

#[path = "book_v2_block_source_starts.rs"]
mod block_source_starts;
