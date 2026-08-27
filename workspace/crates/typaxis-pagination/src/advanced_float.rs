use std::collections::{BTreeSet, VecDeque};
use typaxis_core::{
    push_jcs_string, sha256, Length, MasterId, NodeId, NonNegativeLength, PositiveLength, Rect,
    ValidatedResourceLimits,
};
use typaxis_document::{FloatClearance, FloatPlacementClass};
use typaxis_layout::{FlowId, StagingFloatBodyItem, StagingFloatBodyItemKind, StagingFloatLayout};

use crate::advanced_header_footer::{
    StagingAdvancedFlowPosition, StagingAdvancedPageFrameKind, StagingPageMargins,
    StagingPdfPageBox, StagingSelectedAdvancedFrame, StagingSelectedPageBoxes,
    ADVANCED_SELECTED_LAYOUT_ALGORITHM,
};

pub const FLOAT_QUEUE_ALGORITHM: &str = "typaxis.float-queue/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatQueueEntry {
    float_flow_id: FlowId,
    caption_flow_id: FlowId,
    figure_node_id: NodeId,
    anchor_body_flow_id: FlowId,
    anchor_position: StagingAdvancedFlowPosition,
    anchor_page_index: u32,
    anchor_column_index: u32,
    carry_count: u32,
    image_width: PositiveLength,
    float_extent: PositiveLength,
    here_evaluated: bool,
    top_evaluated_frame: Option<(u32, u32)>,
}

impl StagingFloatQueueEntry {
    pub const fn float_flow_id(&self) -> FlowId {
        self.float_flow_id
    }
    pub const fn caption_flow_id(&self) -> FlowId {
        self.caption_flow_id
    }
    pub const fn figure_node_id(&self) -> NodeId {
        self.figure_node_id
    }
    pub const fn anchor_body_flow_id(&self) -> FlowId {
        self.anchor_body_flow_id
    }
    pub const fn anchor_position(&self) -> StagingAdvancedFlowPosition {
        self.anchor_position
    }
    pub const fn carry_count(&self) -> u32 {
        self.carry_count
    }
    pub const fn image_width(&self) -> PositiveLength {
        self.image_width
    }
    pub const fn float_extent(&self) -> PositiveLength {
        self.float_extent
    }

    fn manifest_eq(&self, other: &Self) -> bool {
        self.float_flow_id == other.float_flow_id
            && self.figure_node_id == other.figure_node_id
            && self.anchor_body_flow_id == other.anchor_body_flow_id
            && self.anchor_position == other.anchor_position
            && self.carry_count == other.carry_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatCandidateDecision {
    float_flow_id: FlowId,
    figure_node_id: NodeId,
    class: FloatPlacementClass,
    page_index: u32,
    column_index: u32,
    applicable: bool,
    accepted: bool,
}

impl StagingFloatCandidateDecision {
    pub const fn float_flow_id(&self) -> FlowId {
        self.float_flow_id
    }
    pub const fn figure_node_id(&self) -> NodeId {
        self.figure_node_id
    }
    pub const fn class(&self) -> FloatPlacementClass {
        self.class
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn column_index(&self) -> u32 {
        self.column_index
    }
    pub const fn applicable(&self) -> bool {
        self.applicable
    }
    pub const fn accepted(&self) -> bool {
        self.accepted
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatPlacement {
    float_flow_id: FlowId,
    caption_flow_id: FlowId,
    figure_node_id: NodeId,
    class: FloatPlacementClass,
    clearance: FloatClearance,
    page_index: u32,
    column_index: u32,
    frame_flow_id: FlowId,
    source_flow_id: FlowId,
    anchor_position: StagingAdvancedFlowPosition,
    frame_paint_ordinal: u32,
    bounds: Rect,
    image_width: PositiveLength,
    float_terminal: bool,
    caption_terminal: bool,
}

impl StagingFloatPlacement {
    pub const fn float_flow_id(&self) -> FlowId {
        self.float_flow_id
    }
    pub const fn caption_flow_id(&self) -> FlowId {
        self.caption_flow_id
    }
    pub const fn figure_node_id(&self) -> NodeId {
        self.figure_node_id
    }
    pub const fn class(&self) -> FloatPlacementClass {
        self.class
    }
    pub const fn clearance(&self) -> FloatClearance {
        self.clearance
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn column_index(&self) -> u32 {
        self.column_index
    }
    pub const fn frame_flow_id(&self) -> FlowId {
        self.frame_flow_id
    }
    pub const fn source_flow_id(&self) -> FlowId {
        self.source_flow_id
    }
    pub const fn anchor_position(&self) -> StagingAdvancedFlowPosition {
        self.anchor_position
    }
    pub const fn frame_paint_ordinal(&self) -> u32 {
        self.frame_paint_ordinal
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
    pub const fn image_width(&self) -> PositiveLength {
        self.image_width
    }
    pub const fn float_terminal(&self) -> bool {
        self.float_terminal
    }
    pub const fn caption_terminal(&self) -> bool {
        self.caption_terminal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatCarry {
    float_flow_id: FlowId,
    figure_node_id: NodeId,
    source_page_index: u32,
    target_page_index: u32,
    carry_count: u32,
}

impl StagingFloatCarry {
    pub const fn float_flow_id(&self) -> FlowId {
        self.float_flow_id
    }
    pub const fn figure_node_id(&self) -> NodeId {
        self.figure_node_id
    }
    pub const fn source_page_index(&self) -> u32 {
        self.source_page_index
    }
    pub const fn target_page_index(&self) -> u32 {
        self.target_page_index
    }
    pub const fn carry_count(&self) -> u32 {
        self.carry_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatBodyFragment {
    page_index: u32,
    column_index: u32,
    frame_flow_id: FlowId,
    source_flow_id: FlowId,
    block_node_id: NodeId,
    before_position: u32,
    after_position: u32,
    frame_paint_ordinal: u32,
    bounds: Rect,
}

impl StagingFloatBodyFragment {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn column_index(&self) -> u32 {
        self.column_index
    }
    pub const fn frame_flow_id(&self) -> FlowId {
        self.frame_flow_id
    }
    pub const fn source_flow_id(&self) -> FlowId {
        self.source_flow_id
    }
    pub const fn block_node_id(&self) -> NodeId {
        self.block_node_id
    }
    pub const fn before_position(&self) -> u32 {
        self.before_position
    }
    pub const fn after_position(&self) -> u32 {
        self.after_position
    }
    pub const fn frame_paint_ordinal(&self) -> u32 {
        self.frame_paint_ordinal
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatSelectedPage {
    page_index: u32,
    master_id: MasterId,
    boxes: StagingSelectedPageBoxes,
    margins: StagingPageMargins,
    frames: Vec<StagingSelectedAdvancedFrame>,
    body_fragments: Vec<StagingFloatBodyFragment>,
    queue_before: Vec<StagingFloatQueueEntry>,
    placements: Vec<StagingFloatPlacement>,
    candidates: Vec<StagingFloatCandidateDecision>,
    carries: Vec<StagingFloatCarry>,
    queue_after: Vec<StagingFloatQueueEntry>,
}

impl StagingFloatSelectedPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn boxes(&self) -> StagingSelectedPageBoxes {
        self.boxes
    }
    pub const fn margins(&self) -> StagingPageMargins {
        self.margins
    }
    pub fn frames(&self) -> &[StagingSelectedAdvancedFrame] {
        &self.frames
    }
    pub fn body_fragments(&self) -> &[StagingFloatBodyFragment] {
        &self.body_fragments
    }
    pub fn queue_before(&self) -> &[StagingFloatQueueEntry] {
        &self.queue_before
    }
    pub fn placements(&self) -> &[StagingFloatPlacement] {
        &self.placements
    }
    pub fn candidates(&self) -> &[StagingFloatCandidateDecision] {
        &self.candidates
    }
    pub fn carries(&self) -> &[StagingFloatCarry] {
        &self.carries
    }
    pub fn queue_after(&self) -> &[StagingFloatQueueEntry] {
        &self.queue_after
    }
}

#[derive(Debug)]
pub struct StagingFloatSelectedLayoutReceipt {
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    canonical_jcs: String,
}

impl StagingFloatSelectedLayoutReceipt {
    pub const fn profile_receipt_sha256(&self) -> [u8; 32] {
        self.profile_receipt_sha256
    }
    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Debug)]
pub struct StagingFloatSelectedLayout {
    pages: Vec<StagingFloatSelectedPage>,
    receipt: StagingFloatSelectedLayoutReceipt,
}

impl StagingFloatSelectedLayout {
    pub fn pages(&self) -> &[StagingFloatSelectedPage] {
        &self.pages
    }
    pub const fn receipt(&self) -> &StagingFloatSelectedLayoutReceipt {
        &self.receipt
    }

    pub fn verify_receipt(&self) -> Result<(), StagingFloatPaginationError> {
        if self.pages.is_empty() {
            return Err(StagingFloatPaginationError::SelectedReceiptMismatch);
        }
        let mut expected_cursor = 0u32;
        let mut previous_queue: Option<&[StagingFloatQueueEntry]> = None;
        let mut placed = BTreeSet::new();
        for (page_ordinal, page) in self.pages.iter().enumerate() {
            if u32::try_from(page_ordinal) != Ok(page.page_index)
                || page.frames.is_empty()
                || page
                    .frames
                    .first()
                    .map(|frame| frame.before_position().ordinal())
                    != Some(expected_cursor)
                || page.frames.iter().enumerate().any(|(index, frame)| {
                    frame.kind() != StagingAdvancedPageFrameKind::Body
                        || frame.column_index() != u32::try_from(index).ok()
                        || frame.source_flow_id() != FlowId::DOCUMENT_BODY
                        || frame.before_position().flow_id() != FlowId::DOCUMENT_BODY
                        || frame.after_position().flow_id() != FlowId::DOCUMENT_BODY
                })
                || page.frames.windows(2).any(|pair| {
                    pair[0].after_position().ordinal() != pair[1].before_position().ordinal()
                })
                || !queue_is_valid(&page.queue_before)
                || !queue_is_valid(&page.queue_after)
            {
                return Err(StagingFloatPaginationError::SelectedReceiptMismatch);
            }
            if let Some(previous) = previous_queue {
                if !queues_manifest_equal(previous, &page.queue_before) {
                    return Err(StagingFloatPaginationError::SelectedReceiptMismatch);
                }
            } else if !page.queue_before.is_empty() {
                return Err(StagingFloatPaginationError::SelectedReceiptMismatch);
            }
            expected_cursor = page
                .frames
                .last()
                .ok_or(StagingFloatPaginationError::SelectedReceiptMismatch)?
                .after_position()
                .ordinal();
            let mut frame_ordinals = vec![BTreeSet::new(); page.frames.len()];
            for fragment in &page.body_fragments {
                let frame = frame_for(page, fragment.column_index)?;
                if fragment.page_index != page.page_index
                    || fragment.frame_flow_id != frame.frame_flow_id()
                    || fragment.source_flow_id != FlowId::DOCUMENT_BODY
                    || !rect_contains(frame.rect(), fragment.bounds)
                    || !frame_ordinals[usize::try_from(fragment.column_index)
                        .map_err(|_| StagingFloatPaginationError::SelectedReceiptMismatch)?]
                    .insert(fragment.frame_paint_ordinal)
                {
                    return Err(StagingFloatPaginationError::SelectedReceiptMismatch);
                }
            }
            for placement in &page.placements {
                let frame = frame_for(page, placement.column_index)?;
                if placement.page_index != page.page_index
                    || placement.frame_flow_id != frame.frame_flow_id()
                    || placement.source_flow_id != FlowId::DOCUMENT_BODY
                    || placement.clearance != FloatClearance::Zero
                    || placement.class == FloatPlacementClass::NextPage
                    || !placement.float_terminal
                    || !placement.caption_terminal
                    || !rect_contains(frame.rect(), placement.bounds)
                    || !placed.insert(placement.float_flow_id)
                    || !frame_ordinals[usize::try_from(placement.column_index)
                        .map_err(|_| StagingFloatPaginationError::SelectedReceiptMismatch)?]
                    .insert(placement.frame_paint_ordinal)
                {
                    return Err(StagingFloatPaginationError::SelectedReceiptMismatch);
                }
            }
            for ordinals in frame_ordinals {
                if ordinals.iter().copied().ne(0..u32::try_from(ordinals.len())
                    .map_err(|_| StagingFloatPaginationError::SelectedReceiptMismatch)?)
                {
                    return Err(StagingFloatPaginationError::SelectedReceiptMismatch);
                }
            }
            if page.carries.iter().enumerate().any(|(index, carry)| {
                page.queue_after.get(index).map_or(true, |entry| {
                    carry.float_flow_id != entry.float_flow_id
                        || carry.figure_node_id != entry.figure_node_id
                        || carry.source_page_index != page.page_index
                        || carry.target_page_index != page.page_index.checked_add(1).unwrap_or(0)
                        || carry.carry_count != entry.carry_count
                })
            }) || page.carries.len() != page.queue_after.len()
            {
                return Err(StagingFloatPaginationError::SelectedReceiptMismatch);
            }
            previous_queue = Some(&page.queue_after);
        }
        let last = self
            .pages
            .last()
            .ok_or(StagingFloatPaginationError::SelectedReceiptMismatch)?;
        if !last.queue_after.is_empty()
            || !last
                .frames
                .last()
                .is_some_and(StagingSelectedAdvancedFrame::terminal)
        {
            return Err(StagingFloatPaginationError::SelectedReceiptMismatch);
        }
        let canonical = encode_selected_layout(
            self.receipt.profile_receipt_sha256,
            self.receipt.flow_registry_sha256,
            &self.pages,
        );
        if canonical != self.receipt.canonical_jcs
            || sha256(canonical.as_bytes()) != self.receipt.selected_layout_sha256
        {
            return Err(StagingFloatPaginationError::SelectedReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingFloatPaginationError {
    EmptyColumns,
    PageLimit,
    FragmentLimit,
    QueueLimit(NodeId),
    CarryLimit(NodeId),
    Oversize(NodeId),
    ProgressContradiction,
    SelectedReceiptMismatch,
    Geometry,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingFloatPaginationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyColumns => formatter.write_str("I9190: float column set is empty"),
            Self::PageLimit => formatter.write_str("L5110: float page limit exceeded"),
            Self::FragmentLimit => formatter.write_str("L5110: float fragment limit exceeded"),
            Self::QueueLimit(node) => write!(
                formatter,
                "G6004: float queue limit exceeded before node {} enqueue",
                node.get()
            ),
            Self::CarryLimit(node) => write!(
                formatter,
                "G6004: float carry limit exceeded before node {} page crossing",
                node.get()
            ),
            Self::Oversize(node) => write!(
                formatter,
                "L5100: unsplittable float or body group at node {} exceeds an empty column",
                node.get()
            ),
            Self::ProgressContradiction => {
                formatter.write_str("I9190: float pagination made no composite progress")
            }
            Self::SelectedReceiptMismatch => {
                formatter.write_str("I9190: float selected receipt mismatch")
            }
            Self::Geometry => formatter.write_str("L5101: invalid selected float geometry"),
            Self::ArithmeticOverflow => formatter.write_str("L5101: float arithmetic overflow"),
            Self::AllocationFailure => formatter.write_str("L5110: float allocation failure"),
        }
    }
}

impl std::error::Error for StagingFloatPaginationError {}

pub fn paginate_staging_float(
    layout: &StagingFloatLayout,
    limits: &ValidatedResourceLimits,
) -> Result<StagingFloatSelectedLayout, StagingFloatPaginationError> {
    if layout.columns().is_empty() {
        return Err(StagingFloatPaginationError::EmptyColumns);
    }
    let boxes = derive_boxes(layout)?;
    let margins = derive_margins(layout)?;
    let terminal = u32::try_from(layout.body_items().len())
        .map_err(|_| StagingFloatPaginationError::ArithmeticOverflow)?;
    let min_column_width = layout
        .columns()
        .iter()
        .map(|column| column.rect().width())
        .min_by_key(|width| width.get().raw())
        .ok_or(StagingFloatPaginationError::EmptyColumns)?;
    let mut queue = VecDeque::new();
    let mut pages = Vec::new();
    let mut cursor = 0u32;
    let mut selected_records = 0u64;
    let mut page_required_by_break = false;

    loop {
        if u64::try_from(pages.len())
            .map_err(|_| StagingFloatPaginationError::ArithmeticOverflow)?
            >= u64::from(limits.get().max_pages)
        {
            return Err(StagingFloatPaginationError::PageLimit);
        }
        let page_index = u32::try_from(pages.len())
            .map_err(|_| StagingFloatPaginationError::ArithmeticOverflow)?;
        let page_entry_cursor = cursor;
        let (mut page, forced_page_boundary, last_eligible_column) = evaluate_page(
            layout,
            page_index,
            &mut cursor,
            terminal,
            min_column_width,
            &mut queue,
            limits,
            boxes,
            margins,
            &mut selected_records,
        )?;

        if !queue.is_empty() {
            let target_page = page_index
                .checked_add(1)
                .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
            for entry in &queue {
                if entry.carry_count >= u32::from(limits.get().max_float_carry_pages) {
                    return Err(StagingFloatPaginationError::CarryLimit(
                        entry.figure_node_id,
                    ));
                }
            }
            if let Some(head) = queue.front() {
                page.candidates.push(StagingFloatCandidateDecision {
                    float_flow_id: head.float_flow_id,
                    figure_node_id: head.figure_node_id,
                    class: FloatPlacementClass::NextPage,
                    page_index,
                    column_index: last_eligible_column,
                    applicable: true,
                    accepted: true,
                });
            }
            charge_fragments(
                &mut selected_records,
                u64::try_from(queue.len())
                    .map_err(|_| StagingFloatPaginationError::ArithmeticOverflow)?,
                limits,
            )?;
            page.carries
                .try_reserve_exact(queue.len())
                .map_err(|_| StagingFloatPaginationError::AllocationFailure)?;
            for entry in &mut queue {
                entry.carry_count = entry
                    .carry_count
                    .checked_add(1)
                    .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
                page.carries.push(StagingFloatCarry {
                    float_flow_id: entry.float_flow_id,
                    figure_node_id: entry.figure_node_id,
                    source_page_index: page_index,
                    target_page_index: target_page,
                    carry_count: entry.carry_count,
                });
            }
        }
        page.queue_after = queue.iter().cloned().collect();
        assign_paint_ordinals(&mut page)?;
        let terminal_blank_page = (pages.is_empty() || page_required_by_break)
            && cursor == terminal
            && queue.is_empty()
            && page.body_fragments.is_empty();
        if cursor == page_entry_cursor
            && page.placements.is_empty()
            && page.carries.is_empty()
            && !terminal_blank_page
        {
            return Err(StagingFloatPaginationError::ProgressContradiction);
        }
        let done = cursor == terminal && queue.is_empty() && !forced_page_boundary;
        pages.push(page);
        if done {
            break;
        }
        page_required_by_break = forced_page_boundary;
    }

    let canonical_jcs = encode_selected_layout(
        layout.receipt().profile_receipt_sha256(),
        layout.receipt().fingerprint(),
        &pages,
    );
    let receipt = StagingFloatSelectedLayoutReceipt {
        profile_receipt_sha256: layout.receipt().profile_receipt_sha256(),
        flow_registry_sha256: layout.receipt().fingerprint(),
        selected_layout_sha256: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    let selected = StagingFloatSelectedLayout { pages, receipt };
    selected.verify_receipt()?;
    Ok(selected)
}

#[allow(clippy::too_many_arguments)]
fn evaluate_page(
    layout: &StagingFloatLayout,
    page_index: u32,
    cursor: &mut u32,
    terminal: u32,
    min_column_width: PositiveLength,
    queue: &mut VecDeque<StagingFloatQueueEntry>,
    limits: &ValidatedResourceLimits,
    boxes: StagingSelectedPageBoxes,
    margins: StagingPageMargins,
    selected_records: &mut u64,
) -> Result<(StagingFloatSelectedPage, bool, u32), StagingFloatPaginationError> {
    let queue_before = queue.iter().cloned().collect();
    charge_fragments(
        selected_records,
        u64::try_from(layout.columns().len())
            .map_err(|_| StagingFloatPaginationError::ArithmeticOverflow)?,
        limits,
    )?;
    let mut frames = Vec::new();
    frames
        .try_reserve_exact(layout.columns().len())
        .map_err(|_| StagingFloatPaginationError::AllocationFailure)?;
    let mut page = StagingFloatSelectedPage {
        page_index,
        master_id: layout.page_master().master_id.clone(),
        boxes,
        margins,
        frames: Vec::new(),
        body_fragments: Vec::new(),
        queue_before,
        placements: Vec::new(),
        candidates: Vec::new(),
        carries: Vec::new(),
        queue_after: Vec::new(),
    };
    let mut forced_page_boundary = false;
    let mut last_eligible_column = 0u32;

    for column in layout.columns() {
        let before = *cursor;
        let height = column.rect().height().get().raw();
        let mut used = 0i64;
        let boundary_precedes_column = forced_page_boundary;
        if !boundary_precedes_column {
            last_eligible_column = column.column_index();
            place_top_floats(
                queue,
                column,
                page_index,
                &mut used,
                &mut page,
                selected_records,
                limits,
            )?;
        }
        if !boundary_precedes_column && *cursor < terminal {
            loop {
                let item_index = usize::try_from(*cursor)
                    .map_err(|_| StagingFloatPaginationError::ArithmeticOverflow)?;
                let item = layout
                    .body_items()
                    .get(item_index)
                    .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
                if item.forced_page_break() {
                    *cursor = item.after_position();
                    forced_page_boundary = true;
                    break;
                }
                if item.kind() == StagingFloatBodyItemKind::FloatAnchor {
                    enqueue_float(
                        item,
                        page_index,
                        column.column_index(),
                        min_column_width,
                        column.rect().height(),
                        queue,
                        limits,
                    )?;
                    *cursor = item.after_position();
                    evaluate_here_head(
                        queue,
                        column,
                        page_index,
                        &mut used,
                        &mut page,
                        selected_records,
                        limits,
                    )?;
                    if *cursor >= terminal {
                        break;
                    }
                    continue;
                }
                let (group_end, group_extent) = group_extent(layout.body_items(), item_index)?;
                if group_extent > height {
                    return Err(StagingFloatPaginationError::Oversize(item.node_id()));
                }
                let required = used
                    .checked_add(group_extent)
                    .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
                if required > height {
                    break;
                }
                for source in &layout.body_items()[item_index..group_end] {
                    if source.block_extent().get().raw() > 0 {
                        charge_fragments(selected_records, 1, limits)?;
                        page.body_fragments
                            .try_reserve(1)
                            .map_err(|_| StagingFloatPaginationError::AllocationFailure)?;
                        let extent = PositiveLength::new(source.block_extent().get())
                            .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
                        let y = column
                            .rect()
                            .y()
                            .raw()
                            .checked_add(used)
                            .and_then(Length::from_raw)
                            .ok_or(StagingFloatPaginationError::Geometry)?;
                        page.body_fragments.push(StagingFloatBodyFragment {
                            page_index,
                            column_index: column.column_index(),
                            frame_flow_id: column.frame_flow_id(),
                            source_flow_id: FlowId::DOCUMENT_BODY,
                            block_node_id: source.node_id(),
                            before_position: source.before_position(),
                            after_position: source.after_position(),
                            frame_paint_ordinal: 0,
                            bounds: Rect::new(column.rect().x(), y, column.rect().width(), extent),
                        });
                        used = used
                            .checked_add(extent.get().raw())
                            .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
                    }
                    *cursor = source.after_position();
                }
                if *cursor >= terminal {
                    break;
                }
            }
        }
        if !boundary_precedes_column {
            place_bottom_floats(
                queue,
                column,
                page_index,
                used,
                &mut page,
                selected_records,
                limits,
            )?;
        }
        frames.push(StagingSelectedAdvancedFrame::new(
            StagingAdvancedPageFrameKind::Body,
            Some(column.column_index()),
            column.frame_flow_id(),
            FlowId::DOCUMENT_BODY,
            column.rect(),
            StagingAdvancedFlowPosition::new(FlowId::DOCUMENT_BODY, before),
            StagingAdvancedFlowPosition::new(FlowId::DOCUMENT_BODY, *cursor),
            *cursor == terminal,
            None,
        ));
    }
    page.frames = frames;
    Ok((page, forced_page_boundary, last_eligible_column))
}

fn enqueue_float(
    item: &StagingFloatBodyItem,
    page_index: u32,
    column_index: u32,
    min_column_width: PositiveLength,
    full_column_height: PositiveLength,
    queue: &mut VecDeque<StagingFloatQueueEntry>,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingFloatPaginationError> {
    let image_width = item
        .image_width()
        .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
    let float_extent = item
        .float_extent()
        .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
    if image_width.get().raw() > min_column_width.get().raw()
        || float_extent.get().raw() > full_column_height.get().raw()
    {
        return Err(StagingFloatPaginationError::Oversize(item.node_id()));
    }
    if queue.len()
        >= usize::try_from(limits.get().max_float_queue)
            .map_err(|_| StagingFloatPaginationError::ArithmeticOverflow)?
    {
        return Err(StagingFloatPaginationError::QueueLimit(item.node_id()));
    }
    let float_flow_id = item
        .float_flow_id()
        .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
    let caption_flow_id = item
        .caption_flow_id()
        .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
    if queue
        .iter()
        .any(|entry| entry.float_flow_id == float_flow_id)
    {
        return Err(StagingFloatPaginationError::ProgressContradiction);
    }
    queue
        .try_reserve(1)
        .map_err(|_| StagingFloatPaginationError::AllocationFailure)?;
    queue.push_back(StagingFloatQueueEntry {
        float_flow_id,
        caption_flow_id,
        figure_node_id: item.node_id(),
        anchor_body_flow_id: FlowId::DOCUMENT_BODY,
        anchor_position: StagingAdvancedFlowPosition::new(
            FlowId::DOCUMENT_BODY,
            item.before_position(),
        ),
        anchor_page_index: page_index,
        anchor_column_index: column_index,
        carry_count: 0,
        image_width,
        float_extent,
        here_evaluated: false,
        top_evaluated_frame: None,
    });
    Ok(())
}

fn place_top_floats(
    queue: &mut VecDeque<StagingFloatQueueEntry>,
    column: &typaxis_layout::StagingFloatColumnTemplate,
    page_index: u32,
    used: &mut i64,
    page: &mut StagingFloatSelectedPage,
    selected_records: &mut u64,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingFloatPaginationError> {
    while let Some(head) = queue.front() {
        if !head.here_evaluated {
            record_candidate(page, head, FloatPlacementClass::Here, column, false, false);
            queue
                .front_mut()
                .ok_or(StagingFloatPaginationError::ProgressContradiction)?
                .here_evaluated = true;
        }
        let head = queue
            .front()
            .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
        let fits = used
            .checked_add(head.float_extent.get().raw())
            .is_some_and(|required| required <= column.rect().height().get().raw());
        record_candidate(page, head, FloatPlacementClass::Top, column, true, fits);
        queue
            .front_mut()
            .ok_or(StagingFloatPaginationError::ProgressContradiction)?
            .top_evaluated_frame = Some((page_index, column.column_index()));
        if !fits {
            break;
        }
        charge_fragments(selected_records, 1, limits)?;
        let entry = queue
            .pop_front()
            .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
        let y = column
            .rect()
            .y()
            .raw()
            .checked_add(*used)
            .and_then(Length::from_raw)
            .ok_or(StagingFloatPaginationError::Geometry)?;
        push_placement(page, entry, FloatPlacementClass::Top, page_index, column, y)?;
        *used = used
            .checked_add(
                page.placements
                    .last()
                    .ok_or(StagingFloatPaginationError::ProgressContradiction)?
                    .bounds
                    .height()
                    .get()
                    .raw(),
            )
            .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
    }
    Ok(())
}

fn evaluate_here_head(
    queue: &mut VecDeque<StagingFloatQueueEntry>,
    column: &typaxis_layout::StagingFloatColumnTemplate,
    page_index: u32,
    used: &mut i64,
    page: &mut StagingFloatSelectedPage,
    selected_records: &mut u64,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingFloatPaginationError> {
    let Some(head) = queue.front() else {
        return Ok(());
    };
    if head.here_evaluated {
        return Ok(());
    }
    let at_anchor =
        head.anchor_page_index == page_index && head.anchor_column_index == column.column_index();
    let fits = at_anchor
        && used
            .checked_add(head.float_extent.get().raw())
            .is_some_and(|required| required <= column.rect().height().get().raw());
    record_candidate(
        page,
        head,
        FloatPlacementClass::Here,
        column,
        at_anchor,
        fits,
    );
    queue
        .front_mut()
        .ok_or(StagingFloatPaginationError::ProgressContradiction)?
        .here_evaluated = true;
    if !fits {
        return Ok(());
    }
    charge_fragments(selected_records, 1, limits)?;
    let entry = queue
        .pop_front()
        .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
    let y = column
        .rect()
        .y()
        .raw()
        .checked_add(*used)
        .and_then(Length::from_raw)
        .ok_or(StagingFloatPaginationError::Geometry)?;
    let extent = entry.float_extent.get().raw();
    push_placement(
        page,
        entry,
        FloatPlacementClass::Here,
        page_index,
        column,
        y,
    )?;
    *used = used
        .checked_add(extent)
        .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
    Ok(())
}

fn place_bottom_floats(
    queue: &mut VecDeque<StagingFloatQueueEntry>,
    column: &typaxis_layout::StagingFloatColumnTemplate,
    page_index: u32,
    top_used: i64,
    page: &mut StagingFloatSelectedPage,
    selected_records: &mut u64,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingFloatPaginationError> {
    let mut bottom_used = 0i64;
    while let Some(head) = queue.front() {
        if !head.here_evaluated {
            record_candidate(page, head, FloatPlacementClass::Here, column, false, false);
            queue
                .front_mut()
                .ok_or(StagingFloatPaginationError::ProgressContradiction)?
                .here_evaluated = true;
        }
        let head = queue
            .front()
            .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
        if head.top_evaluated_frame != Some((page_index, column.column_index())) {
            record_candidate(page, head, FloatPlacementClass::Top, column, false, false);
            queue
                .front_mut()
                .ok_or(StagingFloatPaginationError::ProgressContradiction)?
                .top_evaluated_frame = Some((page_index, column.column_index()));
        }
        let head = queue
            .front()
            .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
        let required = top_used
            .checked_add(bottom_used)
            .and_then(|value| value.checked_add(head.float_extent.get().raw()))
            .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
        let fits = required <= column.rect().height().get().raw();
        record_candidate(page, head, FloatPlacementClass::Bottom, column, true, fits);
        if !fits {
            break;
        }
        charge_fragments(selected_records, 1, limits)?;
        let entry = queue
            .pop_front()
            .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
        let extent = entry.float_extent.get().raw();
        bottom_used = bottom_used
            .checked_add(extent)
            .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
        let y = column
            .rect()
            .y()
            .raw()
            .checked_add(column.rect().height().get().raw())
            .and_then(|bottom| bottom.checked_sub(bottom_used))
            .and_then(Length::from_raw)
            .ok_or(StagingFloatPaginationError::Geometry)?;
        push_placement(
            page,
            entry,
            FloatPlacementClass::Bottom,
            page_index,
            column,
            y,
        )?;
    }
    Ok(())
}

fn record_candidate(
    page: &mut StagingFloatSelectedPage,
    entry: &StagingFloatQueueEntry,
    class: FloatPlacementClass,
    column: &typaxis_layout::StagingFloatColumnTemplate,
    applicable: bool,
    accepted: bool,
) {
    page.candidates.push(StagingFloatCandidateDecision {
        float_flow_id: entry.float_flow_id,
        figure_node_id: entry.figure_node_id,
        class,
        page_index: page.page_index,
        column_index: column.column_index(),
        applicable,
        accepted,
    });
}

fn push_placement(
    page: &mut StagingFloatSelectedPage,
    entry: StagingFloatQueueEntry,
    class: FloatPlacementClass,
    page_index: u32,
    column: &typaxis_layout::StagingFloatColumnTemplate,
    y: Length,
) -> Result<(), StagingFloatPaginationError> {
    page.placements
        .try_reserve(1)
        .map_err(|_| StagingFloatPaginationError::AllocationFailure)?;
    page.placements.push(StagingFloatPlacement {
        float_flow_id: entry.float_flow_id,
        caption_flow_id: entry.caption_flow_id,
        figure_node_id: entry.figure_node_id,
        class,
        clearance: FloatClearance::Zero,
        page_index,
        column_index: column.column_index(),
        frame_flow_id: column.frame_flow_id(),
        source_flow_id: FlowId::DOCUMENT_BODY,
        anchor_position: entry.anchor_position,
        frame_paint_ordinal: 0,
        bounds: Rect::new(
            column.rect().x(),
            y,
            column.rect().width(),
            entry.float_extent,
        ),
        image_width: entry.image_width,
        float_terminal: true,
        caption_terminal: true,
    });
    Ok(())
}

fn assign_paint_ordinals(
    page: &mut StagingFloatSelectedPage,
) -> Result<(), StagingFloatPaginationError> {
    for column_index in 0..page.frames.len() {
        let column = u32::try_from(column_index)
            .map_err(|_| StagingFloatPaginationError::ArithmeticOverflow)?;
        let mut next = 0u32;
        for placement in page.placements.iter_mut().filter(|placement| {
            placement.column_index == column && placement.class == FloatPlacementClass::Top
        }) {
            placement.frame_paint_ordinal = next;
            next = next
                .checked_add(1)
                .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
        }
        let mut middle = Vec::new();
        for (index, fragment) in page.body_fragments.iter().enumerate() {
            if fragment.column_index == column {
                middle.push((fragment.before_position, 1u8, index, false));
            }
        }
        for (index, placement) in page.placements.iter().enumerate() {
            if placement.column_index == column && placement.class == FloatPlacementClass::Here {
                middle.push((placement.anchor_position.ordinal(), 0u8, index, true));
            }
        }
        middle.sort_by_key(|value| (value.0, value.1));
        for (_, _, index, is_float) in middle {
            if is_float {
                page.placements[index].frame_paint_ordinal = next;
            } else {
                page.body_fragments[index].frame_paint_ordinal = next;
            }
            next = next
                .checked_add(1)
                .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
        }
        for placement in page.placements.iter_mut().filter(|placement| {
            placement.column_index == column && placement.class == FloatPlacementClass::Bottom
        }) {
            placement.frame_paint_ordinal = next;
            next = next
                .checked_add(1)
                .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
        }
    }
    Ok(())
}

fn group_extent(
    items: &[StagingFloatBodyItem],
    start: usize,
) -> Result<(usize, i64), StagingFloatPaginationError> {
    let mut end = start;
    let mut extent = 0i64;
    loop {
        let item = items
            .get(end)
            .ok_or(StagingFloatPaginationError::ProgressContradiction)?;
        if item.kind() != StagingFloatBodyItemKind::Block || item.forced_page_break() {
            break;
        }
        extent = extent
            .checked_add(item.block_extent().get().raw())
            .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
        end = end
            .checked_add(1)
            .ok_or(StagingFloatPaginationError::ArithmeticOverflow)?;
        if !item.keep_with_next()
            || end >= items.len()
            || items[end].forced_page_break()
            || items[end].kind() == StagingFloatBodyItemKind::FloatAnchor
        {
            break;
        }
    }
    if end == start {
        return Err(StagingFloatPaginationError::ProgressContradiction);
    }
    Ok((end, extent))
}

fn frame_for(
    page: &StagingFloatSelectedPage,
    column_index: u32,
) -> Result<&StagingSelectedAdvancedFrame, StagingFloatPaginationError> {
    page.frames
        .get(
            usize::try_from(column_index)
                .map_err(|_| StagingFloatPaginationError::SelectedReceiptMismatch)?,
        )
        .filter(|frame| frame.column_index() == Some(column_index))
        .ok_or(StagingFloatPaginationError::SelectedReceiptMismatch)
}

fn queue_is_valid(queue: &[StagingFloatQueueEntry]) -> bool {
    let mut ids = BTreeSet::new();
    queue.iter().all(|entry| {
        entry.anchor_body_flow_id == FlowId::DOCUMENT_BODY
            && entry.anchor_position.flow_id() == FlowId::DOCUMENT_BODY
            && ids.insert(entry.float_flow_id)
    })
}

fn queues_manifest_equal(
    left: &[StagingFloatQueueEntry],
    right: &[StagingFloatQueueEntry],
) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.manifest_eq(right))
}

fn derive_boxes(
    layout: &StagingFloatLayout,
) -> Result<StagingSelectedPageBoxes, StagingFloatPaginationError> {
    let width = layout.page_master().width.get().raw();
    let height = layout.page_master().height.get().raw();
    let trim = layout.advanced_page_master().trim;
    if trim.x().raw() != 0
        || trim.y().raw() != 0
        || trim.width().get().raw() != width
        || trim.height().get().raw() != height
    {
        return Err(StagingFloatPaginationError::Geometry);
    }
    let media = StagingPdfPageBox::new(0, 0, width, height);
    Ok(StagingSelectedPageBoxes::new(media, media, media))
}

fn derive_margins(
    layout: &StagingFloatLayout,
) -> Result<StagingPageMargins, StagingFloatPaginationError> {
    let master = layout.page_master();
    let right = master
        .width
        .get()
        .raw()
        .checked_sub(master.body.x().raw())
        .and_then(|value| value.checked_sub(master.body.width().get().raw()))
        .ok_or(StagingFloatPaginationError::Geometry)?;
    let bottom = master
        .height
        .get()
        .raw()
        .checked_sub(master.body.y().raw())
        .and_then(|value| value.checked_sub(master.body.height().get().raw()))
        .ok_or(StagingFloatPaginationError::Geometry)?;
    let value = |raw| {
        Length::from_raw(raw)
            .and_then(NonNegativeLength::new)
            .ok_or(StagingFloatPaginationError::Geometry)
    };
    Ok(StagingPageMargins::new(
        value(master.body.y().raw())?,
        value(right)?,
        value(bottom)?,
        value(master.body.x().raw())?,
    ))
}

fn charge_fragments(
    count: &mut u64,
    amount: u64,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingFloatPaginationError> {
    let next = count
        .checked_add(amount)
        .ok_or(StagingFloatPaginationError::FragmentLimit)?;
    if next > limits.get().max_fragments {
        return Err(StagingFloatPaginationError::FragmentLimit);
    }
    *count = next;
    Ok(())
}

fn encode_selected_layout(
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    pages: &[StagingFloatSelectedPage],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ADVANCED_SELECTED_LAYOUT_ALGORITHM);
    output.push_str(",\"float_queue_algorithm\":");
    push_jcs_string(&mut output, FLOAT_QUEUE_ALGORITHM);
    output.push_str(",\"flow_registry_sha256\":");
    push_hex(&mut output, flow_registry_sha256);
    output.push_str(",\"pages\":[");
    for (page_index, page) in pages.iter().enumerate() {
        if page_index > 0 {
            output.push(',');
        }
        output.push_str("{\"body_fragments\":[");
        for (index, fragment) in page.body_fragments.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str("{\"after_position\":");
            output.push_str(&fragment.after_position.to_string());
            output.push_str(",\"before_position\":");
            output.push_str(&fragment.before_position.to_string());
            output.push_str(",\"block_node_id\":");
            output.push_str(&fragment.block_node_id.get().to_string());
            output.push_str(",\"bounds\":");
            push_rect(&mut output, fragment.bounds);
            output.push_str(",\"column_index\":");
            output.push_str(&fragment.column_index.to_string());
            output.push_str(",\"frame_flow_id\":");
            output.push_str(&fragment.frame_flow_id.get().to_string());
            output.push_str(",\"frame_paint_ordinal\":");
            output.push_str(&fragment.frame_paint_ordinal.to_string());
            output.push('}');
        }
        output.push_str("],\"boxes\":{\"crop\":");
        push_box(&mut output, page.boxes.crop_box());
        output.push_str(",\"media\":");
        push_box(&mut output, page.boxes.media_box());
        output.push_str(",\"trim\":");
        push_box(&mut output, page.boxes.trim_box());
        output.push_str("},\"candidates\":[");
        for (index, candidate) in page.candidates.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str("{\"accepted\":");
            output.push_str(if candidate.accepted { "true" } else { "false" });
            output.push_str(",\"applicable\":");
            output.push_str(if candidate.applicable {
                "true"
            } else {
                "false"
            });
            output.push_str(",\"class\":");
            push_jcs_string(&mut output, candidate.class.as_str());
            output.push_str(",\"column_index\":");
            output.push_str(&candidate.column_index.to_string());
            output.push_str(",\"figure_node_id\":");
            output.push_str(&candidate.figure_node_id.get().to_string());
            output.push_str(",\"float_flow_id\":");
            output.push_str(&candidate.float_flow_id.get().to_string());
            output.push_str(",\"page_index\":");
            output.push_str(&candidate.page_index.to_string());
            output.push('}');
        }
        output.push_str("],\"carries\":[");
        for (index, carry) in page.carries.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            push_carry(&mut output, carry);
        }
        output.push_str("],\"frames\":[");
        for (index, frame) in page.frames.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            push_frame(&mut output, frame);
        }
        output.push_str("],\"margins\":");
        push_margins(&mut output, page.margins);
        output.push_str(",\"master_id\":");
        push_jcs_string(&mut output, page.master_id.as_str());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"placements\":[");
        for (index, placement) in page.placements.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            push_placement_json(&mut output, placement);
        }
        output.push_str("],\"queue_after\":[");
        push_queue(&mut output, &page.queue_after);
        output.push_str("],\"queue_before\":[");
        push_queue(&mut output, &page.queue_before);
        output.push_str("]}");
    }
    output.push_str("],\"profile_receipt_sha256\":");
    push_hex(&mut output, profile_receipt_sha256);
    output.push('}');
    output
}

fn push_queue(output: &mut String, queue: &[StagingFloatQueueEntry]) {
    for (index, entry) in queue.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"anchor_body_flow_id\":");
        output.push_str(&entry.anchor_body_flow_id.get().to_string());
        output.push_str(",\"anchor_position\":");
        push_position(output, entry.anchor_position);
        output.push_str(",\"caption_flow_id\":");
        output.push_str(&entry.caption_flow_id.get().to_string());
        output.push_str(",\"carry_count\":");
        output.push_str(&entry.carry_count.to_string());
        output.push_str(",\"figure_node_id\":");
        output.push_str(&entry.figure_node_id.get().to_string());
        output.push_str(",\"float_extent\":");
        output.push_str(&entry.float_extent.get().raw().to_string());
        output.push_str(",\"float_flow_id\":");
        output.push_str(&entry.float_flow_id.get().to_string());
        output.push_str(",\"image_width\":");
        output.push_str(&entry.image_width.get().raw().to_string());
        output.push('}');
    }
}

fn push_placement_json(output: &mut String, placement: &StagingFloatPlacement) {
    output.push_str("{\"anchor_position\":");
    push_position(output, placement.anchor_position);
    output.push_str(",\"bounds\":");
    push_rect(output, placement.bounds);
    output.push_str(",\"caption_flow_id\":");
    output.push_str(&placement.caption_flow_id.get().to_string());
    output.push_str(",\"caption_terminal\":");
    output.push_str(if placement.caption_terminal {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"class\":");
    push_jcs_string(output, placement.class.as_str());
    output.push_str(",\"clearance\":0,\"column_index\":");
    output.push_str(&placement.column_index.to_string());
    output.push_str(",\"figure_node_id\":");
    output.push_str(&placement.figure_node_id.get().to_string());
    output.push_str(",\"float_flow_id\":");
    output.push_str(&placement.float_flow_id.get().to_string());
    output.push_str(",\"float_terminal\":");
    output.push_str(if placement.float_terminal {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"frame_flow_id\":");
    output.push_str(&placement.frame_flow_id.get().to_string());
    output.push_str(",\"frame_paint_ordinal\":");
    output.push_str(&placement.frame_paint_ordinal.to_string());
    output.push_str(",\"image_width\":");
    output.push_str(&placement.image_width.get().raw().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&placement.page_index.to_string());
    output.push('}');
}

fn push_carry(output: &mut String, carry: &StagingFloatCarry) {
    output.push_str("{\"carry_count\":");
    output.push_str(&carry.carry_count.to_string());
    output.push_str(",\"figure_node_id\":");
    output.push_str(&carry.figure_node_id.get().to_string());
    output.push_str(",\"float_flow_id\":");
    output.push_str(&carry.float_flow_id.get().to_string());
    output.push_str(",\"source_page_index\":");
    output.push_str(&carry.source_page_index.to_string());
    output.push_str(",\"target_page_index\":");
    output.push_str(&carry.target_page_index.to_string());
    output.push('}');
}

fn push_frame(output: &mut String, frame: &StagingSelectedAdvancedFrame) {
    output.push_str("{\"after_position\":");
    push_position(output, frame.after_position());
    output.push_str(",\"before_position\":");
    push_position(output, frame.before_position());
    output.push_str(",\"column_index\":");
    match frame.column_index() {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
    output.push_str(",\"frame_flow_id\":");
    output.push_str(&frame.frame_flow_id().get().to_string());
    output.push_str(",\"kind\":\"body\",\"rect\":");
    push_rect(output, frame.rect());
    output.push_str(",\"source_flow_id\":");
    output.push_str(&frame.source_flow_id().get().to_string());
    output.push_str(",\"terminal\":");
    output.push_str(if frame.terminal() { "true" } else { "false" });
    output.push('}');
}

fn push_position(output: &mut String, position: StagingAdvancedFlowPosition) {
    output.push_str("{\"flow_id\":");
    output.push_str(&position.flow_id().get().to_string());
    output.push_str(",\"ordinal\":");
    output.push_str(&position.ordinal().to_string());
    output.push('}');
}

fn push_rect(output: &mut String, rect: Rect) {
    output.push_str("{\"height\":");
    output.push_str(&rect.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&rect.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&rect.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&rect.y().raw().to_string());
    output.push('}');
}

fn push_margins(output: &mut String, margins: StagingPageMargins) {
    output.push_str("{\"bottom\":");
    output.push_str(&margins.bottom().get().raw().to_string());
    output.push_str(",\"left\":");
    output.push_str(&margins.left().get().raw().to_string());
    output.push_str(",\"right\":");
    output.push_str(&margins.right().get().raw().to_string());
    output.push_str(",\"top\":");
    output.push_str(&margins.top().get().raw().to_string());
    output.push('}');
}

fn push_box(output: &mut String, value: StagingPdfPageBox) {
    output.push('[');
    for (index, value) in value.values().into_iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&value.to_string());
    }
    output.push(']');
}

fn push_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

fn rect_contains(outer: Rect, inner: Rect) -> bool {
    let Some(outer_right) = outer.x().raw().checked_add(outer.width().get().raw()) else {
        return false;
    };
    let Some(outer_bottom) = outer.y().raw().checked_add(outer.height().get().raw()) else {
        return false;
    };
    let Some(inner_right) = inner.x().raw().checked_add(inner.width().get().raw()) else {
        return false;
    };
    let Some(inner_bottom) = inner.y().raw().checked_add(inner.height().get().raw()) else {
        return false;
    };
    inner.x().raw() >= outer.x().raw()
        && inner.y().raw() >= outer.y().raw()
        && inner_right <= outer_right
        && inner_bottom <= outer_bottom
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_float_selected_fixture() -> StagingFloatSelectedLayout {
    let layout = typaxis_layout::staging_float_layout_fixture();
    let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
        .expect("fixture limits are valid");
    paginate_staging_float(&layout, &limits).expect("float fixture paginates")
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::ResourceLimits;

    #[test]
    fn floats_queue_identity_and_candidate_order_are_typed() {
        assert_eq!(FLOAT_QUEUE_ALGORITHM, "typaxis.float-queue/1");
        assert_eq!(
            FloatPlacementClass::ORDERED.map(FloatPlacementClass::as_str),
            ["here", "top", "bottom", "next_page"]
        );
    }

    #[test]
    fn floats_fifo_place_here_then_top_and_carry_across_columns_and_pages() {
        let selected = staging_float_selected_fixture();
        assert_eq!(selected.pages().len(), 3);
        assert_eq!(
            selected
                .pages()
                .iter()
                .flat_map(StagingFloatSelectedPage::placements)
                .map(StagingFloatPlacement::class)
                .collect::<Vec<_>>(),
            [
                FloatPlacementClass::Here,
                FloatPlacementClass::Top,
                FloatPlacementClass::Top,
                FloatPlacementClass::Top,
                FloatPlacementClass::Top,
            ]
        );
        assert_eq!(
            selected
                .pages()
                .iter()
                .map(|page| page.queue_after().len())
                .collect::<Vec<_>>(),
            [3, 1, 0]
        );
        assert_eq!(
            selected
                .pages()
                .iter()
                .flat_map(StagingFloatSelectedPage::carries)
                .map(StagingFloatCarry::carry_count)
                .collect::<Vec<_>>(),
            [1, 1, 1, 2]
        );
        for page in selected.pages() {
            let mut evaluated = BTreeSet::new();
            for candidate in page.candidates() {
                assert!(evaluated.insert((
                    candidate.float_flow_id(),
                    candidate.page_index(),
                    candidate.column_index(),
                    candidate.class(),
                )));
            }
        }
        selected.verify_receipt().unwrap();
    }

    #[test]
    fn floats_forced_page_boundary_does_not_open_later_columns() {
        let layout = typaxis_layout::staging_float_forced_break_layout_fixture();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let selected = paginate_staging_float(&layout, &limits).unwrap();
        let first = &selected.pages()[0];
        assert!(first
            .placements()
            .iter()
            .all(|placement| placement.column_index() == 0));
        assert_eq!(first.carries().len(), 1);
        assert_eq!(first.queue_after().len(), 1);
        assert_eq!(
            first
                .candidates()
                .iter()
                .find(|candidate| candidate.class() == FloatPlacementClass::NextPage)
                .map(StagingFloatCandidateDecision::column_index),
            Some(0)
        );
        assert_eq!(
            first.frames()[1].before_position(),
            first.frames()[1].after_position()
        );
    }

    #[test]
    fn floats_trailing_forced_break_materializes_the_post_break_blank_page() {
        let layout = typaxis_layout::staging_float_trailing_break_layout_fixture();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let selected = paginate_staging_float(&layout, &limits).unwrap();
        assert_eq!(selected.pages().len(), 2);
        assert!(selected.pages().iter().all(|page| {
            page.body_fragments().is_empty()
                && page.placements().is_empty()
                && page.carries().is_empty()
        }));
        assert_eq!(
            selected.pages()[1].frames()[0].before_position().ordinal(),
            1
        );
        assert!(selected.pages()[1]
            .frames()
            .iter()
            .all(StagingSelectedAdvancedFrame::terminal));
    }

    #[test]
    fn floats_queue_and_carry_maxima_are_inclusive_and_refuse_max_plus_one() {
        let layout = typaxis_layout::staging_float_layout_fixture();
        let exact = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 3,
            max_fragments: 17,
            max_float_queue: 4,
            max_float_carry_pages: 2,
            ..ResourceLimits::default()
        })
        .unwrap();
        paginate_staging_float(&layout, &exact).unwrap();

        let queue_over = ValidatedResourceLimits::new(ResourceLimits {
            max_float_queue: 3,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert!(matches!(
            paginate_staging_float(&layout, &queue_over),
            Err(StagingFloatPaginationError::QueueLimit(node)) if node == NodeId::new(17)
        ));

        let carry_over = ValidatedResourceLimits::new(ResourceLimits {
            max_float_carry_pages: 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert!(matches!(
            paginate_staging_float(&layout, &carry_over),
            Err(StagingFloatPaginationError::CarryLimit(node)) if node == NodeId::new(17)
        ));
    }

    #[test]
    fn floats_reject_queue_reorder_carry_replay_wrong_anchor_and_oversize() {
        let mut reordered = staging_float_selected_fixture();
        reordered.pages[0].queue_after.swap(0, 1);
        assert!(matches!(
            reordered.verify_receipt(),
            Err(StagingFloatPaginationError::SelectedReceiptMismatch)
        ));

        let mut replay = staging_float_selected_fixture();
        replay.pages[1].carries[0].carry_count = 1;
        assert!(matches!(
            replay.verify_receipt(),
            Err(StagingFloatPaginationError::SelectedReceiptMismatch)
        ));

        let mut wrong_anchor = staging_float_selected_fixture();
        wrong_anchor.pages[0].queue_after[0].anchor_position =
            StagingAdvancedFlowPosition::new(FlowId::DOCUMENT_BODY, 5);
        assert!(matches!(
            wrong_anchor.verify_receipt(),
            Err(StagingFloatPaginationError::SelectedReceiptMismatch)
        ));

        let oversize = typaxis_layout::staging_float_oversize_layout_fixture();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        assert!(matches!(
            paginate_staging_float(&oversize, &limits),
            Err(StagingFloatPaginationError::Oversize(node)) if node == NodeId::new(3)
        ));
    }
}
