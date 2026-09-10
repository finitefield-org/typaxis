//! Rebind provisional source widths after actual shaping, without retaining an
//! obsolete itemization or treating line ordinals as stable source positions.
use super::*;
use typaxis_linebreak::{ProductionInlineLogicalUnit as Unit, ProductionInlineSourceWidths};

/// Borrowed assignments for one exact immutable source flow. Each Some slice has
/// one width per source scalar/atom/break, or one for an empty paragraph. None
/// preserves the ordinary frame width. A changed generated-label flow requires
/// a new assignment; this owner is not a physical-page acceptance receipt.
pub struct BookV2SourceWidthAssignments<'w, 'a> {
    flow: &'a PreparedBookV2TextFlow<'a>,
    widths: &'w [Option<&'w [PositiveLength]>],
    starts: Option<&'w [Option<&'w [Length]>]>,
    retained_ends: Option<&'w [Option<&'w [u32]>]>,
    block_widths: &'w [(NodeId, PositiveLength)],
    block_starts: Option<&'w [Length]>,
    root_table_widths: &'w [(NodeId, PositiveLength)],
    header_scope: Option<NodeId>,
    table_sources: Option<&'w [NodeId]>,
    inherit_table_blocks: bool,
    occurrence_frames: bool,
}
impl<'w, 'a> BookV2SourceWidthAssignments<'w, 'a> {
    pub fn new(
        flow: &'a PreparedBookV2TextFlow<'a>,
        widths: &'w [Option<&'w [PositiveLength]>],
    ) -> Result<Self, ProductionInlinePreparationError> {
        if widths.len() != flow.paragraphs().len() {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        Ok(Self {
            flow,
            widths,
            starts: None,
            retained_ends: None,
            block_widths: &[],
            block_starts: None,
            root_table_widths: &[],
            header_scope: None,
            table_sources: None,
            inherit_table_blocks: false,
            occurrence_frames: false,
        })
    }
    pub fn with_retained_line_ends(
        flow: &'a PreparedBookV2TextFlow<'a>,
        widths: &'w [Option<&'w [PositiveLength]>],
        ends: &'w [Option<&'w [u32]>],
    ) -> Result<Self, ProductionInlinePreparationError> {
        let mut value = Self::new(flow, widths)?;
        if ends.len() != widths.len() {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        value.retained_ends = Some(ends);
        Ok(value)
    }
    /// Body-relative starts indexed by original logical-unit position. Each
    /// supplied paragraph must also have a complete source-width profile.
    /// These provisional origins still require independent final validation.
    pub fn with_source_unit_starts(
        mut self,
        starts: &'w [Option<&'w [Length]>],
    ) -> Result<Self, ProductionInlinePreparationError> {
        if starts.len() != self.widths.len() {
            return Err(error(
                NodeId::new(0),
                ProductionInlinePreparationErrorKind::ReceiptMismatch,
            ));
        }
        self.starts = Some(starts);
        Ok(self)
    }
    pub fn with_table_occurrence_frames(mut self) -> Self {
        self.occurrence_frames = true;
        self
    }
    pub(super) fn uses_table_occurrence_frames(&self) -> bool {
        self.occurrence_frames
    }
    pub(super) fn source_unit_starts(&self) -> Option<&[Option<&[Length]>]> {
        self.starts
    }
    pub(super) fn paragraph_widths(&self) -> &[Option<&[PositiveLength]>] {
        self.widths
    }
    /// Parent-frame widths for original block owners, in source event order.
    /// These candidates must be consumed by the framed shaping path.
    pub fn with_block_widths(mut self, widths: &'w [(NodeId, PositiveLength)]) -> Self {
        self.block_widths = widths;
        self
    }
    /// Body-relative parent starts paired with every original block width.
    /// Explicit starts cannot be combined with inherited table-block widths.
    pub fn with_block_starts(mut self, starts: &'w [Length]) -> Self {
        self.block_starts = Some(starts);
        self
    }
    pub(super) fn block_starts(&self) -> Option<&[Length]> {
        self.block_starts
    }
    /// Parent widths for all original root tables, in source event order.
    /// Nested tables derive their widths from the newly resolved parent cells.
    /// These are provisional measurements, not physical-page authorization.
    pub fn with_root_table_widths(mut self, widths: &'w [(NodeId, PositiveLength)]) -> Self {
        self.root_table_widths = widths;
        self
    }
    /// Reproject only this root table's repeated header at the candidate width.
    /// Its caption and body retain the original envelope projection. This is a
    /// provisional line graph, never authorization to place those body objects
    /// on a page with the header width. The complete source remains present.
    pub fn with_table_header_scope(mut self, owner: NodeId) -> Self {
        self.table_sources = None;
        self.header_scope = Some(owner);
        self
    }
    /// Reproject only the requested original leaves and their source ancestors.
    /// `root` is an original root table; owners must be sorted, unique leaves
    /// inside it. Unselected branches retain the original envelope geometry.
    pub fn with_table_source_scope(mut self, root: NodeId, owners: &'w [NodeId]) -> Self {
        self.header_scope = Some(root);
        self.table_sources = Some(owners);
        self
    }
    pub(super) fn table_source_owners(&self) -> Option<&[NodeId]> {
        self.table_sources
    }
    pub(super) fn table_header_scope(&self) -> Option<NodeId> {
        self.header_scope
    }
    /// Use the newly projected table parent for table-local block entries.
    /// Supplied entries still bind every original block owner in source order.
    pub fn with_inherited_table_blocks(mut self) -> Self {
        self.inherit_table_blocks = true;
        self
    }
    pub(super) fn inherits_table_blocks(&self) -> bool {
        self.inherit_table_blocks
    }
    pub(super) fn root_table_widths(&self) -> &[(NodeId, PositiveLength)] {
        self.root_table_widths
    }
    pub(super) fn block_widths(&self) -> &[(NodeId, PositiveLength)] {
        self.block_widths
    }
    pub(super) fn matches_flow(&self, flow: &PreparedBookV2TextFlow<'_>) -> bool {
        std::ptr::eq(self.flow, flow)
    }
}

/// Project a fresh preparation using widths bound to its original source flow.
/// The caller's prior records and work allowance include all rebinding visits.
pub fn layout_book_v2_source_width_lines_from_flow<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    envelopes: &[PositiveLength],
    assignments: &BookV2SourceWidthAssignments<'_, '_>,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    if assignments.occurrence_frames
        || assignments.starts.is_some()
        || assignments.block_starts.is_some()
        || !assignments.block_widths.is_empty()
        || !assignments.root_table_widths.is_empty()
        || assignments.header_scope.is_some()
    {
        return Err(error(
            NodeId::new(0),
            ProductionInlinePreparationErrorKind::ReceiptMismatch,
        ));
    }
    layout_source_width_lines_from_flow(
        prepared,
        envelopes,
        assignments,
        maximum_work,
        prior_records,
    )
}
pub(super) fn layout_source_width_lines_from_flow<'p, 'a>(
    prepared: &'p BookV2PreparedInlines<'a>,
    envelopes: &[PositiveLength],
    assignments: &BookV2SourceWidthAssignments<'_, '_>,
    maximum_work: u64,
    prior_records: u64,
) -> Result<BookV2InlineLineLayout<'p, 'a>, ProductionInlinePreparationError> {
    use ProductionInlinePreparationErrorKind as E;
    let root = NodeId::new(0);
    if !std::ptr::eq(assignments.flow, prepared.flow)
        || assignments.widths.len() != prepared.paragraphs.len()
        || envelopes.len() != prepared.paragraphs.len()
    {
        return Err(error(root, E::ReceiptMismatch));
    }
    // These binding records and width slots are charged by the shared projection.
    // Check their capacity before allocating the temporary binding vector.
    let mut records = prior_records
        .checked_add(prepared.native_math().map_or(0, |m| m.record_charge()))
        .filter(|n| *n <= prepared.max_fragments)
        .ok_or_else(|| error(root, E::UnitLimit))?;
    records = records
        .checked_add(assignments.widths.len() as u64)
        .filter(|n| *n <= prepared.max_fragments)
        .ok_or_else(|| error(root, E::UnitLimit))?;
    let mut remaining = maximum_work;
    let step = |remaining: &mut u64, owner| {
        *remaining = remaining.checked_sub(1).ok_or_else(|| {
            error(
                owner,
                E::Atomic(typaxis_linebreak::AtomicVectorInlineError::CandidateLimit),
            )
        })?;
        Ok::<_, ProductionInlinePreparationError>(())
    };
    for (index, (p, widths)) in prepared
        .paragraphs
        .iter()
        .zip(assignments.widths)
        .enumerate()
    {
        step(&mut remaining, p.owner())?;
        if let Some(ends) = assignments.retained_ends.and_then(|ends| ends[index]) {
            if widths.is_none() {
                return Err(error(p.owner(), E::ReceiptMismatch));
            }
            records = records
                .checked_add(ends.len() as u64)
                .filter(|n| *n <= prepared.max_fragments)
                .ok_or_else(|| error(p.owner(), E::UnitLimit))?;
        }
        if let Some(widths) = widths {
            let items = p
                .items()
                .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
            if widths.len() != items.units().len().max(1) {
                return Err(error(p.owner(), E::ReceiptMismatch));
            }
            records = records
                .checked_add(widths.len() as u64)
                .filter(|n| *n <= prepared.max_fragments)
                .ok_or_else(|| error(p.owner(), E::UnitLimit))?;
        }
    }
    let mut bindings = Vec::new();
    bindings
        .try_reserve_exact(assignments.widths.len())
        .map_err(|_| error(root, E::AllocationFailure))?;
    for ((p, source), widths) in prepared
        .paragraphs
        .iter()
        .zip(assignments.flow.paragraphs())
        .zip(assignments.widths)
    {
        if p.owner() != source.owner() {
            return Err(error(p.owner(), E::ReceiptMismatch));
        }
        let Some(widths) = widths else {
            bindings.push(None);
            continue;
        };
        let items = p
            .items()
            .ok_or_else(|| error(p.owner(), E::ReceiptMismatch))?;
        let mut units = items.units().iter();
        for site in source.items() {
            let owner = site.owner();
            step(&mut remaining, owner)?;
            let matches = match site.content() {
                ProductionInlineContent::Text { .. }
                | ProductionInlineContent::FootnoteReference
                | ProductionInlineContent::Reference => {
                    let (_, text) = inline_shape_text(InlineFlow::BookV2(assignments.flow), site)?;
                    for scalar in text.chars() {
                        step(&mut remaining, owner)?;
                        if !matches!(units.next(), Some(Unit::Text(t)) if t.scalar() == scalar) {
                            return Err(error(owner, E::ReceiptMismatch));
                        }
                    }
                    true
                }
                ProductionInlineContent::InlineVector | ProductionInlineContent::MathVector => {
                    matches!(units.next(), Some(Unit::Vector(v)) if v.node_id() == owner && v.source_span() == site.source_span())
                }
                ProductionInlineContent::NativeMath => {
                    matches!(units.next(), Some(Unit::Math(m)) if m.owner() == owner && m.source_span() == site.source_span())
                }
                ProductionInlineContent::SoftBreak | ProductionInlineContent::HardBreak => {
                    let kind = if matches!(site.content(), ProductionInlineContent::SoftBreak) {
                        BreakKind::Allowed
                    } else {
                        BreakKind::Mandatory
                    };
                    matches!(units.next(), Some(Unit::Break(b)) if b.owner() == owner && b.source_span() == site.source_span() && b.kind() == kind)
                }
                ProductionInlineContent::Anchor
                | ProductionInlineContent::BeginEmphasis
                | ProductionInlineContent::BeginStrong
                | ProductionInlineContent::BeginLink
                | ProductionInlineContent::EndContainer => true,
            };
            if !matches {
                return Err(error(owner, E::ReceiptMismatch));
            }
        }
        if units.next().is_some() {
            return Err(error(p.owner(), E::ReceiptMismatch));
        }
        let index = bindings.len();
        bindings.push(Some(
            if let Some(ends) = assignments.retained_ends.and_then(|ends| ends[index]) {
                ProductionInlineSourceWidths::with_retained_line_ends(items, widths, ends)
            } else {
                ProductionInlineSourceWidths::new(items, widths)
            }
            .map_err(|e| error(p.owner(), E::Atomic(e)))?,
        ));
    }
    let mut lines = layout_book_v2_source_width_lines_charged(
        prepared,
        envelopes,
        &bindings,
        remaining,
        prior_records,
    )?;
    lines.projection.candidate_steps = lines
        .projection
        .candidate_steps
        .checked_add(maximum_work - remaining)
        .ok_or_else(|| error(root, E::ArithmeticOverflow))?;
    Ok(lines)
}
