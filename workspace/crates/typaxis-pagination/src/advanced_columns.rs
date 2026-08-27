use std::collections::BTreeSet;
use typaxis_core::{
    push_jcs_string, sha256, Length, MasterId, NodeId, NonNegativeLength, PositiveLength, Rect,
    ValidatedResourceLimits,
};
use typaxis_layout::{FlowId, StagingColumnBlockLayout, StagingColumnsLayout};

use crate::advanced_header_footer::{
    StagingAdvancedFlowPosition, StagingAdvancedPageFrameKind, StagingPageMargins,
    StagingPdfPageBox, StagingSelectedAdvancedFrame, StagingSelectedPageBoxes,
    ADVANCED_SELECTED_LAYOUT_ALGORITHM,
};

pub const COLUMN_BALANCE_ALGORITHM: &str = "typaxis.column-balance-candidates/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingColumnRejectionReceipt {
    column_index: u32,
    position: StagingAdvancedFlowPosition,
    blocked_node_id: NodeId,
    deficit: PositiveLength,
}

impl StagingColumnRejectionReceipt {
    pub const fn column_index(&self) -> u32 {
        self.column_index
    }
    pub const fn position(&self) -> StagingAdvancedFlowPosition {
        self.position
    }
    pub const fn blocked_node_id(&self) -> NodeId {
        self.blocked_node_id
    }
    pub const fn deficit(&self) -> PositiveLength {
        self.deficit
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StagingColumnBalanceCandidate {
    candidate_index: u32,
    target_height: PositiveLength,
    after_position: StagingAdvancedFlowPosition,
    terminal: bool,
    rejections: Vec<StagingColumnRejectionReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingColumnBalanceReceipt {
    input_sha256: [u8; 32],
    candidate_count: u32,
    selected_target_height: PositiveLength,
    receipt_sha256: [u8; 32],
    candidates: Vec<StagingColumnBalanceCandidate>,
    canonical_jcs: String,
}

impl StagingColumnBalanceReceipt {
    pub const fn input_sha256(&self) -> [u8; 32] {
        self.input_sha256
    }
    pub const fn candidate_count(&self) -> u32 {
        self.candidate_count
    }
    pub const fn selected_target_height(&self) -> PositiveLength {
        self.selected_target_height
    }
    pub const fn receipt_sha256(&self) -> [u8; 32] {
        self.receipt_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    fn verify(&self) -> bool {
        self.candidate_count != 0
            && usize::try_from(self.candidate_count) == Ok(self.candidates.len())
            && self.candidates.last().is_some_and(|candidate| {
                candidate.terminal
                    && candidate.target_height == self.selected_target_height
                    && candidate.rejections.is_empty()
            })
            && self
                .candidates
                .iter()
                .enumerate()
                .all(|(index, candidate)| {
                    u32::try_from(index)
                        .ok()
                        .and_then(|value| value.checked_add(1))
                        == Some(candidate.candidate_index)
                })
            && self.canonical_jcs
                == encode_balance_receipt(
                    self.input_sha256,
                    self.selected_target_height,
                    &self.candidates,
                )
            && self.receipt_sha256 == sha256(self.canonical_jcs.as_bytes())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingColumnFragment {
    page_index: u32,
    column_index: u32,
    frame_flow_id: FlowId,
    source_flow_id: FlowId,
    block_node_id: NodeId,
    before_position: u32,
    after_position: u32,
    bounds: Rect,
}

impl StagingColumnFragment {
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
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingColumnsSelectedPage {
    page_index: u32,
    master_id: MasterId,
    boxes: StagingSelectedPageBoxes,
    margins: StagingPageMargins,
    frames: Vec<StagingSelectedAdvancedFrame>,
    fragments: Vec<StagingColumnFragment>,
    balance: Option<StagingColumnBalanceReceipt>,
}

impl StagingColumnsSelectedPage {
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
    pub fn fragments(&self) -> &[StagingColumnFragment] {
        &self.fragments
    }
    pub const fn balance(&self) -> Option<&StagingColumnBalanceReceipt> {
        self.balance.as_ref()
    }
}

#[derive(Debug)]
pub struct StagingColumnsSelectedLayoutReceipt {
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    canonical_jcs: String,
}

impl StagingColumnsSelectedLayoutReceipt {
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
pub struct StagingColumnsSelectedLayout {
    pages: Vec<StagingColumnsSelectedPage>,
    receipt: StagingColumnsSelectedLayoutReceipt,
}

impl StagingColumnsSelectedLayout {
    pub fn pages(&self) -> &[StagingColumnsSelectedPage] {
        &self.pages
    }
    pub const fn receipt(&self) -> &StagingColumnsSelectedLayoutReceipt {
        &self.receipt
    }

    pub fn verify_receipt(&self) -> Result<(), StagingColumnsPaginationError> {
        if self.pages.is_empty() {
            return Err(StagingColumnsPaginationError::SelectedReceiptMismatch);
        }
        let mut expected_page_entry = 0u32;
        let last_page_index = self.pages.len() - 1;
        for (page_index, page) in self.pages.iter().enumerate() {
            if u32::try_from(page_index) != Ok(page.page_index)
                || page.frames.is_empty()
                || page.frames.iter().enumerate().any(|(index, frame)| {
                    frame.kind() != StagingAdvancedPageFrameKind::Body
                        || frame.column_index() != u32::try_from(index).ok()
                        || frame.source_flow_id() != FlowId::DOCUMENT_BODY
                        || frame.before_position().flow_id() != FlowId::DOCUMENT_BODY
                        || frame.after_position().flow_id() != FlowId::DOCUMENT_BODY
                })
                || page
                    .frames
                    .first()
                    .map(|frame| frame.before_position().ordinal())
                    != Some(expected_page_entry)
                || page.frames.windows(2).any(|pair| {
                    pair[0].after_position().ordinal() != pair[1].before_position().ordinal()
                })
                || (page_index != last_page_index && page.balance.is_some())
                || page
                    .balance
                    .as_ref()
                    .is_some_and(|balance| !balance.verify())
            {
                return Err(StagingColumnsPaginationError::SelectedReceiptMismatch);
            }
            expected_page_entry = page
                .frames
                .last()
                .ok_or(StagingColumnsPaginationError::SelectedReceiptMismatch)?
                .after_position()
                .ordinal();
            if page.fragments.iter().any(|fragment| {
                fragment.page_index != page.page_index
                    || fragment.source_flow_id != FlowId::DOCUMENT_BODY
                    || usize::try_from(fragment.column_index)
                        .ok()
                        .and_then(|index| page.frames.get(index))
                        .map_or(true, |frame| {
                            frame.frame_flow_id() != fragment.frame_flow_id
                                || !rect_contains(frame.rect(), fragment.bounds)
                                || fragment.before_position < frame.before_position().ordinal()
                                || fragment.after_position > frame.after_position().ordinal()
                        })
            }) {
                return Err(StagingColumnsPaginationError::SelectedReceiptMismatch);
            }
        }
        if !self
            .pages
            .last()
            .and_then(|page| page.frames.last())
            .is_some_and(StagingSelectedAdvancedFrame::terminal)
        {
            return Err(StagingColumnsPaginationError::SelectedReceiptMismatch);
        }
        let canonical = encode_selected_layout(
            self.receipt.profile_receipt_sha256,
            self.receipt.flow_registry_sha256,
            &self.pages,
        );
        if canonical != self.receipt.canonical_jcs
            || sha256(canonical.as_bytes()) != self.receipt.selected_layout_sha256
        {
            return Err(StagingColumnsPaginationError::SelectedReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingColumnsPaginationError {
    EmptyColumns,
    PageLimit,
    FragmentLimit,
    Oversize(NodeId),
    BalanceLimit,
    BalanceOscillation,
    ProgressContradiction,
    SelectedReceiptMismatch,
    Geometry,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingColumnsPaginationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyColumns => formatter.write_str("I9190: column template set is empty"),
            Self::PageLimit => formatter.write_str("L5110: columns page limit exceeded"),
            Self::FragmentLimit => formatter.write_str("L5110: columns fragment limit exceeded"),
            Self::Oversize(node) => write!(
                formatter,
                "L5100: indivisible columns group at node {} exceeds an empty frame",
                node.get()
            ),
            Self::BalanceLimit => formatter.write_str("G6003: column balance limit exceeded"),
            Self::BalanceOscillation => formatter.write_str("G6003: column balance oscillation"),
            Self::ProgressContradiction => {
                formatter.write_str("I9190: columns pagination made no progress")
            }
            Self::SelectedReceiptMismatch => {
                formatter.write_str("I9190: columns selected receipt mismatch")
            }
            Self::Geometry => formatter.write_str("L5101: invalid selected column geometry"),
            Self::ArithmeticOverflow => formatter.write_str("L5101: columns arithmetic overflow"),
            Self::AllocationFailure => formatter.write_str("L5110: columns allocation failure"),
        }
    }
}

impl std::error::Error for StagingColumnsPaginationError {}

#[derive(Clone, Debug)]
struct EvaluatedPage {
    frames: Vec<StagingSelectedAdvancedFrame>,
    fragments: Vec<StagingColumnFragment>,
    rejections: Vec<StagingColumnRejectionReceipt>,
    after_position: u32,
    terminal: bool,
    positive_extent: i64,
}

pub fn paginate_staging_columns(
    layout: &StagingColumnsLayout,
    limits: &ValidatedResourceLimits,
) -> Result<StagingColumnsSelectedLayout, StagingColumnsPaginationError> {
    if layout.columns().is_empty() {
        return Err(StagingColumnsPaginationError::EmptyColumns);
    }
    let boxes = derive_boxes(layout)?;
    let margins = derive_margins(layout)?;
    let full_height = layout.page_master().body.height();
    let balance_enabled = layout
        .advanced_page_master()
        .column_layout
        .as_ref()
        .is_some_and(|column| {
            column.balance == typaxis_document::ColumnBalance::LastPage
                && layout.columns().len() > 1
        });
    let mut pages = Vec::new();
    let mut cursor = 0u32;
    let mut selected_records = 0u64;
    loop {
        if u64::try_from(pages.len())
            .map_err(|_| StagingColumnsPaginationError::ArithmeticOverflow)?
            >= u64::from(limits.get().max_pages)
        {
            return Err(StagingColumnsPaginationError::PageLimit);
        }
        let page_index = u32::try_from(pages.len())
            .map_err(|_| StagingColumnsPaginationError::ArithmeticOverflow)?;
        let initial = evaluate_page(layout, page_index, cursor, full_height, true)?;
        let (selected, balance) =
            if initial.terminal && initial.positive_extent > 0 && balance_enabled {
                select_balanced_page(layout, page_index, cursor, &initial, limits)?
            } else {
                (initial, None)
            };
        if !selected.terminal && selected.after_position <= cursor {
            return Err(StagingColumnsPaginationError::ProgressContradiction);
        }
        let page_records = u64::try_from(selected.frames.len())
            .ok()
            .and_then(|value| value.checked_add(u64::try_from(selected.fragments.len()).ok()?))
            .ok_or(StagingColumnsPaginationError::ArithmeticOverflow)?;
        charge_fragments(&mut selected_records, page_records, limits)?;
        cursor = selected.after_position;
        pages.push(StagingColumnsSelectedPage {
            page_index,
            master_id: layout.page_master().master_id.clone(),
            boxes,
            margins,
            frames: selected.frames,
            fragments: selected.fragments,
            balance,
        });
        if selected.terminal {
            break;
        }
    }

    let canonical_jcs = encode_selected_layout(
        layout.receipt().profile_receipt_sha256(),
        layout.receipt().fingerprint(),
        &pages,
    );
    let receipt = StagingColumnsSelectedLayoutReceipt {
        profile_receipt_sha256: layout.receipt().profile_receipt_sha256(),
        flow_registry_sha256: layout.receipt().fingerprint(),
        selected_layout_sha256: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    let selected = StagingColumnsSelectedLayout { pages, receipt };
    selected.verify_receipt()?;
    Ok(selected)
}

fn evaluate_page(
    layout: &StagingColumnsLayout,
    page_index: u32,
    entry_position: u32,
    target_height: PositiveLength,
    full_height_evaluation: bool,
) -> Result<EvaluatedPage, StagingColumnsPaginationError> {
    let terminal = u32::try_from(layout.blocks().len())
        .map_err(|_| StagingColumnsPaginationError::ArithmeticOverflow)?;
    let mut cursor = entry_position;
    let mut frames = Vec::new();
    frames
        .try_reserve_exact(layout.columns().len())
        .map_err(|_| StagingColumnsPaginationError::AllocationFailure)?;
    let mut fragments = Vec::new();
    let mut rejections = Vec::new();
    let mut positive_extent = 0i64;
    let mut forced_page_boundary = false;

    for column in layout.columns() {
        let before = cursor;
        let frame_rect = Rect::new(
            column.rect().x(),
            column.rect().y(),
            column.rect().width(),
            target_height,
        );
        let mut used = 0i64;
        if !forced_page_boundary && cursor < terminal {
            loop {
                let block_index = usize::try_from(cursor)
                    .map_err(|_| StagingColumnsPaginationError::ArithmeticOverflow)?;
                let Some(block) = layout.blocks().get(block_index) else {
                    return Err(StagingColumnsPaginationError::ProgressContradiction);
                };
                if block.forced_page_break() {
                    cursor = block.after_position();
                    forced_page_boundary = true;
                    break;
                }
                let (group_end, group_extent) = group_extent(layout.blocks(), block_index)?;
                let required = used
                    .checked_add(group_extent)
                    .ok_or(StagingColumnsPaginationError::ArithmeticOverflow)?;
                if required > target_height.get().raw() {
                    if full_height_evaluation
                        && used == 0
                        && group_extent > target_height.get().raw()
                    {
                        return Err(StagingColumnsPaginationError::Oversize(block.node_id()));
                    }
                    let deficit = required
                        .checked_sub(target_height.get().raw())
                        .and_then(Length::from_raw)
                        .and_then(PositiveLength::new)
                        .ok_or(StagingColumnsPaginationError::ProgressContradiction)?;
                    rejections.push(StagingColumnRejectionReceipt {
                        column_index: column.column_index(),
                        position: StagingAdvancedFlowPosition::new(FlowId::DOCUMENT_BODY, cursor),
                        blocked_node_id: block.node_id(),
                        deficit,
                    });
                    break;
                }
                for source in &layout.blocks()[block_index..group_end] {
                    if source.block_extent().get().raw() > 0 {
                        let extent = PositiveLength::new(source.block_extent().get())
                            .ok_or(StagingColumnsPaginationError::ProgressContradiction)?;
                        let y = frame_rect
                            .y()
                            .raw()
                            .checked_add(used)
                            .and_then(Length::from_raw)
                            .ok_or(StagingColumnsPaginationError::Geometry)?;
                        fragments.push(StagingColumnFragment {
                            page_index,
                            column_index: column.column_index(),
                            frame_flow_id: column.frame_flow_id(),
                            source_flow_id: FlowId::DOCUMENT_BODY,
                            block_node_id: source.node_id(),
                            before_position: source.before_position(),
                            after_position: source.after_position(),
                            bounds: Rect::new(frame_rect.x(), y, frame_rect.width(), extent),
                        });
                        used = used
                            .checked_add(extent.get().raw())
                            .ok_or(StagingColumnsPaginationError::ArithmeticOverflow)?;
                        positive_extent = positive_extent
                            .checked_add(extent.get().raw())
                            .ok_or(StagingColumnsPaginationError::ArithmeticOverflow)?;
                    }
                    cursor = source.after_position();
                }
                if cursor >= terminal {
                    break;
                }
            }
        }
        frames.push(StagingSelectedAdvancedFrame::new(
            StagingAdvancedPageFrameKind::Body,
            Some(column.column_index()),
            column.frame_flow_id(),
            FlowId::DOCUMENT_BODY,
            frame_rect,
            StagingAdvancedFlowPosition::new(FlowId::DOCUMENT_BODY, before),
            StagingAdvancedFlowPosition::new(FlowId::DOCUMENT_BODY, cursor),
            cursor == terminal,
            None,
        ));
    }
    Ok(EvaluatedPage {
        frames,
        fragments,
        rejections,
        after_position: cursor,
        terminal: cursor == terminal,
        positive_extent,
    })
}

fn group_extent(
    blocks: &[StagingColumnBlockLayout],
    start: usize,
) -> Result<(usize, i64), StagingColumnsPaginationError> {
    let mut end = start;
    let mut extent = 0i64;
    loop {
        let block = blocks
            .get(end)
            .ok_or(StagingColumnsPaginationError::ProgressContradiction)?;
        if block.forced_page_break() {
            break;
        }
        extent = extent
            .checked_add(block.block_extent().get().raw())
            .ok_or(StagingColumnsPaginationError::ArithmeticOverflow)?;
        end = end
            .checked_add(1)
            .ok_or(StagingColumnsPaginationError::ArithmeticOverflow)?;
        if !block.keep_with_next() || end >= blocks.len() || blocks[end].forced_page_break() {
            break;
        }
    }
    if end == start {
        return Err(StagingColumnsPaginationError::ProgressContradiction);
    }
    Ok((end, extent))
}

fn select_balanced_page(
    layout: &StagingColumnsLayout,
    page_index: u32,
    entry_position: u32,
    initial: &EvaluatedPage,
    limits: &ValidatedResourceLimits,
) -> Result<(EvaluatedPage, Option<StagingColumnBalanceReceipt>), StagingColumnsPaginationError> {
    let count = i64::try_from(layout.columns().len())
        .map_err(|_| StagingColumnsPaginationError::ArithmeticOverflow)?;
    let full_height = layout.page_master().body.height();
    let first = initial
        .positive_extent
        .checked_add(count - 1)
        .and_then(|value| value.checked_div(count))
        .map(|value| value.clamp(1, full_height.get().raw()))
        .and_then(Length::from_raw)
        .and_then(PositiveLength::new)
        .ok_or(StagingColumnsPaginationError::ArithmeticOverflow)?;
    let initial_fragments_sha256 = fragments_fingerprint(&initial.fragments);
    let mut target = first;
    let mut candidates = Vec::new();
    let mut rejection_history = BTreeSet::new();
    loop {
        if candidates.len() >= usize::from(limits.get().max_column_balance_candidates) {
            return Err(StagingColumnsPaginationError::BalanceLimit);
        }
        let evaluated = evaluate_page(layout, page_index, entry_position, target, false)?;
        let candidate_index = u32::try_from(candidates.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(StagingColumnsPaginationError::ArithmeticOverflow)?;
        let candidate = StagingColumnBalanceCandidate {
            candidate_index,
            target_height: target,
            after_position: StagingAdvancedFlowPosition::new(
                FlowId::DOCUMENT_BODY,
                evaluated.after_position,
            ),
            terminal: evaluated.terminal,
            rejections: evaluated.rejections.clone(),
        };
        if evaluated.terminal {
            if evaluated.after_position != initial.after_position {
                return Err(StagingColumnsPaginationError::ProgressContradiction);
            }
            candidates.push(candidate);
            let input_jcs = encode_balance_input(
                layout,
                page_index,
                entry_position,
                initial_fragments_sha256,
                &candidates,
            );
            let input_sha256 = sha256(input_jcs.as_bytes());
            let canonical_jcs = encode_balance_receipt(input_sha256, target, &candidates);
            let receipt = StagingColumnBalanceReceipt {
                input_sha256,
                candidate_count: candidate_index,
                selected_target_height: target,
                receipt_sha256: sha256(canonical_jcs.as_bytes()),
                candidates,
                canonical_jcs,
            };
            if !receipt.verify() {
                return Err(StagingColumnsPaginationError::ProgressContradiction);
            }
            return Ok((evaluated, Some(receipt)));
        }
        if evaluated.rejections.is_empty() {
            return Err(StagingColumnsPaginationError::ProgressContradiction);
        }
        record_rejection_state(&mut rejection_history, &evaluated.rejections)?;
        let least = evaluated
            .rejections
            .iter()
            .min_by_key(|rejection| {
                (
                    rejection.deficit.get().raw(),
                    rejection.column_index,
                    rejection.position.ordinal(),
                )
            })
            .ok_or(StagingColumnsPaginationError::ProgressContradiction)?;
        let next = target
            .get()
            .raw()
            .checked_add(least.deficit.get().raw())
            .and_then(Length::from_raw)
            .and_then(PositiveLength::new)
            .ok_or(StagingColumnsPaginationError::ProgressContradiction)?;
        if next.get().raw() <= target.get().raw() || next.get().raw() > full_height.get().raw() {
            return Err(StagingColumnsPaginationError::ProgressContradiction);
        }
        candidates.push(candidate);
        target = next;
    }
}

fn record_rejection_state(
    history: &mut BTreeSet<[u8; 32]>,
    rejections: &[StagingColumnRejectionReceipt],
) -> Result<(), StagingColumnsPaginationError> {
    if history.insert(rejections_fingerprint(rejections)) {
        Ok(())
    } else {
        Err(StagingColumnsPaginationError::BalanceOscillation)
    }
}

fn derive_boxes(
    layout: &StagingColumnsLayout,
) -> Result<StagingSelectedPageBoxes, StagingColumnsPaginationError> {
    let width = layout.page_master().width.get().raw();
    let height = layout.page_master().height.get().raw();
    if layout.advanced_page_master().trim.x().raw() != 0
        || layout.advanced_page_master().trim.y().raw() != 0
        || layout.advanced_page_master().trim.width().get().raw() != width
        || layout.advanced_page_master().trim.height().get().raw() != height
    {
        return Err(StagingColumnsPaginationError::Geometry);
    }
    let media = StagingPdfPageBox::new(0, 0, width, height);
    Ok(StagingSelectedPageBoxes::new(media, media, media))
}

fn derive_margins(
    layout: &StagingColumnsLayout,
) -> Result<StagingPageMargins, StagingColumnsPaginationError> {
    let master = layout.page_master();
    let right = master
        .width
        .get()
        .raw()
        .checked_sub(master.body.x().raw())
        .and_then(|value| value.checked_sub(master.body.width().get().raw()))
        .ok_or(StagingColumnsPaginationError::Geometry)?;
    let bottom = master
        .height
        .get()
        .raw()
        .checked_sub(master.body.y().raw())
        .and_then(|value| value.checked_sub(master.body.height().get().raw()))
        .ok_or(StagingColumnsPaginationError::Geometry)?;
    let value = |raw| {
        Length::from_raw(raw)
            .and_then(NonNegativeLength::new)
            .ok_or(StagingColumnsPaginationError::Geometry)
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
) -> Result<(), StagingColumnsPaginationError> {
    let next = count
        .checked_add(amount)
        .ok_or(StagingColumnsPaginationError::FragmentLimit)?;
    if next > limits.get().max_fragments {
        return Err(StagingColumnsPaginationError::FragmentLimit);
    }
    *count = next;
    Ok(())
}

fn fragments_fingerprint(fragments: &[StagingColumnFragment]) -> [u8; 32] {
    let mut output = String::from("[");
    for (index, fragment) in fragments.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"after\":");
        output.push_str(&fragment.after_position.to_string());
        output.push_str(",\"before\":");
        output.push_str(&fragment.before_position.to_string());
        output.push_str(",\"column\":");
        output.push_str(&fragment.column_index.to_string());
        output.push_str(",\"node\":");
        output.push_str(&fragment.block_node_id.get().to_string());
        output.push('}');
    }
    output.push(']');
    sha256(output.as_bytes())
}

fn rejections_fingerprint(rejections: &[StagingColumnRejectionReceipt]) -> [u8; 32] {
    let mut output = String::from("[");
    for (index, rejection) in rejections.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_rejection(&mut output, rejection);
    }
    output.push(']');
    sha256(output.as_bytes())
}

fn encode_balance_input(
    layout: &StagingColumnsLayout,
    page_index: u32,
    entry_position: u32,
    initial_fragments_sha256: [u8; 32],
    candidates: &[StagingColumnBalanceCandidate],
) -> String {
    let mut output = String::from("{\"body_rect\":");
    push_rect(&mut output, layout.page_master().body);
    output.push_str(",\"candidates\":");
    push_candidates(&mut output, candidates);
    output.push_str(",\"entry_position\":");
    output.push_str(&entry_position.to_string());
    output.push_str(",\"flow_registry_sha256\":");
    push_hex(&mut output, layout.receipt().fingerprint());
    output.push_str(",\"initial_fragments_sha256\":");
    push_hex(&mut output, initial_fragments_sha256);
    output.push_str(",\"master_id\":");
    push_jcs_string(&mut output, layout.page_master().master_id.as_str());
    output.push_str(",\"package_sha256\":");
    push_hex(&mut output, layout.receipt().package_fingerprint().bytes());
    output.push_str(",\"page_index\":");
    output.push_str(&page_index.to_string());
    output.push_str(",\"profile_receipt_sha256\":");
    push_hex(&mut output, layout.receipt().profile_receipt_sha256());
    output.push('}');
    output
}

fn encode_balance_receipt(
    input_sha256: [u8; 32],
    selected_target_height: PositiveLength,
    candidates: &[StagingColumnBalanceCandidate],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, COLUMN_BALANCE_ALGORITHM);
    output.push_str(",\"candidates\":");
    push_candidates(&mut output, candidates);
    output.push_str(",\"input_sha256\":");
    push_hex(&mut output, input_sha256);
    output.push_str(",\"selected_target_height\":");
    output.push_str(&selected_target_height.get().raw().to_string());
    output.push('}');
    output
}

fn push_candidates(output: &mut String, candidates: &[StagingColumnBalanceCandidate]) {
    output.push('[');
    for (index, candidate) in candidates.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"after_position\":");
        push_position(output, candidate.after_position);
        output.push_str(",\"candidate_index\":");
        output.push_str(&candidate.candidate_index.to_string());
        output.push_str(",\"rejections\":[");
        for (rejection_index, rejection) in candidate.rejections.iter().enumerate() {
            if rejection_index > 0 {
                output.push(',');
            }
            push_rejection(output, rejection);
        }
        output.push_str("],\"target_height\":");
        output.push_str(&candidate.target_height.get().raw().to_string());
        output.push_str(",\"terminal\":");
        output.push_str(if candidate.terminal { "true" } else { "false" });
        output.push('}');
    }
    output.push(']');
}

fn push_rejection(output: &mut String, rejection: &StagingColumnRejectionReceipt) {
    output.push_str("{\"blocked_node_id\":");
    output.push_str(&rejection.blocked_node_id.get().to_string());
    output.push_str(",\"column_index\":");
    output.push_str(&rejection.column_index.to_string());
    output.push_str(",\"deficit\":");
    output.push_str(&rejection.deficit.get().raw().to_string());
    output.push_str(",\"position\":");
    push_position(output, rejection.position);
    output.push('}');
}

fn encode_selected_layout(
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    pages: &[StagingColumnsSelectedPage],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ADVANCED_SELECTED_LAYOUT_ALGORITHM);
    output.push_str(",\"flow_registry_sha256\":");
    push_hex(&mut output, flow_registry_sha256);
    output.push_str(",\"pages\":[");
    for (page_index, page) in pages.iter().enumerate() {
        if page_index > 0 {
            output.push(',');
        }
        output.push_str("{\"balance_receipt_sha256\":");
        match &page.balance {
            Some(balance) => push_hex(&mut output, balance.receipt_sha256),
            None => output.push_str("null"),
        }
        output.push_str(",\"boxes\":{\"crop\":");
        push_box(&mut output, page.boxes.crop_box());
        output.push_str(",\"media\":");
        push_box(&mut output, page.boxes.media_box());
        output.push_str(",\"trim\":");
        push_box(&mut output, page.boxes.trim_box());
        output.push_str("},\"fragments\":[");
        for (index, fragment) in page.fragments.iter().enumerate() {
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
            output.push_str(",\"source_flow_id\":");
            output.push_str(&fragment.source_flow_id.get().to_string());
            output.push('}');
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
        output.push('}');
    }
    output.push_str("],\"profile_receipt_sha256\":");
    push_hex(&mut output, profile_receipt_sha256);
    output.push('}');
    output
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
    output.push_str(",\"repetition_index\":null,\"source_flow_id\":");
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
pub fn staging_columns_selected_fixture() -> StagingColumnsSelectedLayout {
    let layout = typaxis_layout::staging_columns_layout_fixture();
    let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default())
        .expect("fixture limits are valid");
    paginate_staging_columns(&layout, &limits).expect("columns fixture paginates")
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::ResourceLimits;

    #[test]
    fn columns_partition_sequential_fill_and_final_balance_are_canonical() {
        let selected = staging_columns_selected_fixture();
        let repeated = staging_columns_selected_fixture();
        assert_eq!(selected.pages().len(), 2);
        assert_eq!(
            selected.pages()[0]
                .frames()
                .iter()
                .map(|frame| frame.rect().width().get().raw())
                .collect::<Vec<_>>(),
            [5, 6]
        );
        assert_eq!(
            selected.pages()[0]
                .frames()
                .iter()
                .map(|frame| {
                    (
                        frame.before_position().ordinal(),
                        frame.after_position().ordinal(),
                    )
                })
                .collect::<Vec<_>>(),
            [(0, 2), (2, 4)]
        );
        let balance = selected.pages()[1]
            .balance()
            .expect("the final nonempty page is balanced");
        assert_eq!(balance.candidate_count(), 2);
        assert_eq!(balance.selected_target_height().get().raw(), 4);
        assert_eq!(
            selected.pages()[1].frames()[0].rect().height().get().raw(),
            4
        );
        assert_eq!(
            selected.pages()[1].frames()[1].rect().height().get().raw(),
            4
        );
        selected.verify_receipt().unwrap();
        assert_eq!(
            selected.receipt().canonical_jcs(),
            repeated.receipt().canonical_jcs(),
            "caller/worker scheduling cannot enter the canonical selection"
        );
    }

    #[test]
    fn columns_balance_page_and_selected_record_limits_are_inclusive() {
        let layout = typaxis_layout::staging_columns_layout_fixture();
        let exact = ResourceLimits {
            max_pages: 2,
            max_fragments: 9,
            max_column_balance_candidates: 2,
            ..ResourceLimits::default()
        };
        let exact = ValidatedResourceLimits::new(exact).unwrap();
        paginate_staging_columns(&layout, &exact).unwrap();

        let candidate_over = ResourceLimits {
            max_column_balance_candidates: 1,
            ..ResourceLimits::default()
        };
        let candidate_over = ValidatedResourceLimits::new(candidate_over).unwrap();
        assert!(matches!(
            paginate_staging_columns(&layout, &candidate_over),
            Err(StagingColumnsPaginationError::BalanceLimit)
        ));

        let page_over = ResourceLimits {
            max_pages: 1,
            ..ResourceLimits::default()
        };
        let page_over = ValidatedResourceLimits::new(page_over).unwrap();
        assert!(matches!(
            paginate_staging_columns(&layout, &page_over),
            Err(StagingColumnsPaginationError::PageLimit)
        ));

        let fragment_over = ResourceLimits {
            max_fragments: 8,
            ..ResourceLimits::default()
        };
        let fragment_over = ValidatedResourceLimits::new(fragment_over).unwrap();
        assert!(matches!(
            paginate_staging_columns(&layout, &fragment_over),
            Err(StagingColumnsPaginationError::FragmentLimit)
        ));
    }

    #[test]
    fn columns_wrong_balance_target_is_rejected_by_selected_closure() {
        let mut selected = staging_columns_selected_fixture();
        let balance = selected.pages[1]
            .balance
            .as_mut()
            .expect("fixture has final balance");
        balance.selected_target_height = Length::from_raw(5)
            .and_then(PositiveLength::new)
            .expect("tampered target remains positive");
        assert!(matches!(
            selected.verify_receipt(),
            Err(StagingColumnsPaginationError::SelectedReceiptMismatch)
        ));
    }

    #[test]
    fn columns_empty_and_oversize_inputs_have_typed_terminal_results() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let empty = typaxis_layout::staging_columns_empty_layout_fixture();
        let selected = paginate_staging_columns(&empty, &limits).unwrap();
        assert_eq!(selected.pages().len(), 1);
        assert!(selected.pages()[0].fragments().is_empty());
        assert!(selected.pages()[0].balance().is_none());
        assert!(selected.pages()[0]
            .frames()
            .iter()
            .all(StagingSelectedAdvancedFrame::terminal));

        let oversize = typaxis_layout::staging_columns_oversize_layout_fixture();
        assert!(matches!(
            paginate_staging_columns(&oversize, &limits),
            Err(StagingColumnsPaginationError::Oversize(node)) if node == NodeId::new(1)
        ));
    }

    #[test]
    fn columns_repeated_rejection_state_stops_as_oscillation() {
        let rejection = StagingColumnRejectionReceipt {
            column_index: 0,
            position: StagingAdvancedFlowPosition::new(FlowId::DOCUMENT_BODY, 2),
            blocked_node_id: NodeId::new(3),
            deficit: Length::from_raw(1).and_then(PositiveLength::new).unwrap(),
        };
        let mut history = BTreeSet::new();
        record_rejection_state(&mut history, std::slice::from_ref(&rejection)).unwrap();
        assert!(matches!(
            record_rejection_state(&mut history, &[rejection]),
            Err(StagingColumnsPaginationError::BalanceOscillation)
        ));
    }
}
