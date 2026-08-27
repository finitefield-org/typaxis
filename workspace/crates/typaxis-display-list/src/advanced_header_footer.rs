use typaxis_core::{push_jcs_string, sha256, MasterId, NodeId, Rect};
use typaxis_layout::{FlowId, StagingPageRegionKind};
use typaxis_pagination::{StagingAdvancedPageFrameKind, StagingHeaderFooterSelectedLayout};
pub use typaxis_pagination::{StagingPdfPageBox, StagingSelectedPageBoxes};

pub const ADVANCED_PAINT_CLOSURE_ALGORITHM: &str = "typaxis.advanced-pagination-paint-closure/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingHeaderFooterPaintCommand {
    command_ordinal: u32,
    page_index: u32,
    master_id: MasterId,
    kind: StagingPageRegionKind,
    source_flow_id: FlowId,
    source_node_id: NodeId,
    block_node_id: NodeId,
    repetition_index: u32,
    bounds: Rect,
}

impl StagingHeaderFooterPaintCommand {
    pub const fn command_ordinal(&self) -> u32 {
        self.command_ordinal
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn kind(&self) -> StagingPageRegionKind {
        self.kind
    }
    pub const fn source_flow_id(&self) -> FlowId {
        self.source_flow_id
    }
    pub const fn source_node_id(&self) -> NodeId {
        self.source_node_id
    }
    pub const fn block_node_id(&self) -> NodeId {
        self.block_node_id
    }
    pub const fn repetition_index(&self) -> u32 {
        self.repetition_index
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingHeaderFooterDisplayPage {
    page_index: u32,
    master_id: MasterId,
    boxes: StagingSelectedPageBoxes,
    first_command: u32,
    command_count: u32,
}

impl StagingHeaderFooterDisplayPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn boxes(&self) -> StagingSelectedPageBoxes {
        self.boxes
    }
    pub const fn first_command(&self) -> u32 {
        self.first_command
    }
    pub const fn command_count(&self) -> u32 {
        self.command_count
    }
}

#[derive(Debug)]
pub struct StagingHeaderFooterDisplayReceipt {
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    paint_closure_sha256: [u8; 32],
    canonical_jcs: String,
}

impl StagingHeaderFooterDisplayReceipt {
    pub const fn profile_receipt_sha256(&self) -> [u8; 32] {
        self.profile_receipt_sha256
    }
    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn paint_closure_sha256(&self) -> [u8; 32] {
        self.paint_closure_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Debug)]
pub struct StagingHeaderFooterDisplay {
    pages: Vec<StagingHeaderFooterDisplayPage>,
    commands: Vec<StagingHeaderFooterPaintCommand>,
    receipt: StagingHeaderFooterDisplayReceipt,
}

impl StagingHeaderFooterDisplay {
    pub fn pages(&self) -> &[StagingHeaderFooterDisplayPage] {
        &self.pages
    }
    pub fn commands(&self) -> &[StagingHeaderFooterPaintCommand] {
        &self.commands
    }
    pub const fn receipt(&self) -> &StagingHeaderFooterDisplayReceipt {
        &self.receipt
    }

    pub fn verify_receipt(&self) -> Result<(), StagingHeaderFooterDisplayError> {
        if self.pages.is_empty() {
            return Err(StagingHeaderFooterDisplayError::SelectedLayoutMismatch);
        }
        let mut next_command = 0usize;
        for (page_index, page) in self.pages.iter().enumerate() {
            let expected_page = u32::try_from(page_index)
                .map_err(|_| StagingHeaderFooterDisplayError::ArithmeticOverflow)?;
            let expected_first = u32::try_from(next_command)
                .map_err(|_| StagingHeaderFooterDisplayError::ArithmeticOverflow)?;
            if page.page_index != expected_page || page.first_command != expected_first {
                return Err(StagingHeaderFooterDisplayError::NonCanonicalPage);
            }
            next_command = next_command
                .checked_add(
                    usize::try_from(page.command_count)
                        .map_err(|_| StagingHeaderFooterDisplayError::ArithmeticOverflow)?,
                )
                .ok_or(StagingHeaderFooterDisplayError::ArithmeticOverflow)?;
            let first_command = usize::try_from(page.first_command)
                .map_err(|_| StagingHeaderFooterDisplayError::ArithmeticOverflow)?;
            let commands = self
                .commands
                .get(first_command..next_command)
                .ok_or(StagingHeaderFooterDisplayError::FragmentClosure)?;
            if commands.iter().any(|command| {
                command.page_index != page.page_index || command.master_id != page.master_id
            }) {
                return Err(StagingHeaderFooterDisplayError::FragmentClosure);
            }
        }
        if next_command != self.commands.len()
            || self
                .commands
                .iter()
                .enumerate()
                .any(|(index, command)| u32::try_from(index) != Ok(command.command_ordinal))
        {
            return Err(StagingHeaderFooterDisplayError::FragmentClosure);
        }
        let canonical = encode_display(
            self.receipt.profile_receipt_sha256,
            self.receipt.flow_registry_sha256,
            self.receipt.selected_layout_sha256,
            &self.pages,
            &self.commands,
        );
        if canonical != self.receipt.canonical_jcs
            || sha256(canonical.as_bytes()) != self.receipt.paint_closure_sha256
        {
            return Err(StagingHeaderFooterDisplayError::SelectedLayoutMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingHeaderFooterDisplayError {
    SelectedLayoutMismatch,
    NonCanonicalPage,
    FrameOrder,
    FragmentClosure,
    FragmentOutsideFrame,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingHeaderFooterDisplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::SelectedLayoutMismatch => "I9190: selected layout receipt mismatch",
            Self::NonCanonicalPage => "I9190: Display page order mismatch",
            Self::FrameOrder => "I9190: Display frame order mismatch",
            Self::FragmentClosure => "I9190: repeated region fragment mismatch",
            Self::FragmentOutsideFrame => "I9190: repeated fragment outside selected frame",
            Self::ArithmeticOverflow => "I9190: Display closure arithmetic overflow",
            Self::AllocationFailure => "L5110: Display closure allocation failure",
        })
    }
}

impl std::error::Error for StagingHeaderFooterDisplayError {}

pub fn build_staging_header_footer_display(
    selected: &StagingHeaderFooterSelectedLayout,
) -> Result<StagingHeaderFooterDisplay, StagingHeaderFooterDisplayError> {
    selected
        .verify_receipt()
        .map_err(|_| StagingHeaderFooterDisplayError::SelectedLayoutMismatch)?;
    if selected.pages().is_empty() {
        return Err(StagingHeaderFooterDisplayError::SelectedLayoutMismatch);
    }
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(selected.pages().len())
        .map_err(|_| StagingHeaderFooterDisplayError::AllocationFailure)?;
    let fragment_count = selected
        .pages()
        .iter()
        .try_fold(0usize, |count, page| {
            count.checked_add(page.region_fragments().len())
        })
        .ok_or(StagingHeaderFooterDisplayError::ArithmeticOverflow)?;
    let mut commands = Vec::new();
    commands
        .try_reserve_exact(fragment_count)
        .map_err(|_| StagingHeaderFooterDisplayError::AllocationFailure)?;
    let mut repetitions =
        std::collections::BTreeMap::<(MasterId, StagingPageRegionKind), u32>::new();
    for (expected_page, page) in selected.pages().iter().enumerate() {
        if page.page_index()
            != u32::try_from(expected_page)
                .map_err(|_| StagingHeaderFooterDisplayError::ArithmeticOverflow)?
        {
            return Err(StagingHeaderFooterDisplayError::NonCanonicalPage);
        }
        validate_frame_order(page.frames())?;
        for frame in page.frames().iter().filter(|frame| {
            matches!(
                frame.kind(),
                StagingAdvancedPageFrameKind::Header | StagingAdvancedPageFrameKind::Footer
            )
        }) {
            let kind = match frame.kind() {
                StagingAdvancedPageFrameKind::Header => StagingPageRegionKind::Header,
                StagingAdvancedPageFrameKind::Footer => StagingPageRegionKind::Footer,
                StagingAdvancedPageFrameKind::Body => unreachable!("body was filtered"),
            };
            let next = repetitions
                .entry((page.master_id().clone(), kind))
                .or_insert(0);
            if frame.repetition_index() != Some(*next) {
                return Err(StagingHeaderFooterDisplayError::FragmentClosure);
            }
            *next = next
                .checked_add(1)
                .ok_or(StagingHeaderFooterDisplayError::ArithmeticOverflow)?;
        }
        let first_command = u32::try_from(commands.len())
            .map_err(|_| StagingHeaderFooterDisplayError::ArithmeticOverflow)?;
        let mut expected_positions = std::collections::BTreeMap::<(FlowId, u32), u32>::new();
        let mut previous_kind = None;
        for fragment in page.region_fragments() {
            if fragment.page_index() != page.page_index()
                || fragment.master_id() != page.master_id()
                || previous_kind.is_some_and(|kind| kind > fragment.kind())
            {
                return Err(StagingHeaderFooterDisplayError::FragmentClosure);
            }
            previous_kind = Some(fragment.kind());
            let frame_kind = match fragment.kind() {
                StagingPageRegionKind::Header => StagingAdvancedPageFrameKind::Header,
                StagingPageRegionKind::Footer => StagingAdvancedPageFrameKind::Footer,
            };
            let frame = page
                .frames()
                .iter()
                .find(|frame| frame.kind() == frame_kind)
                .ok_or(StagingHeaderFooterDisplayError::FragmentClosure)?;
            if frame.source_flow_id() != fragment.source_flow_id()
                || frame.repetition_index() != Some(fragment.repetition_index())
                || !rect_contains(frame.rect(), fragment.bounds())
            {
                return Err(StagingHeaderFooterDisplayError::FragmentOutsideFrame);
            }
            let expected = expected_positions
                .entry((fragment.source_flow_id(), fragment.repetition_index()))
                .or_insert(0);
            if fragment.before_position() != *expected
                || fragment.after_position()
                    != expected
                        .checked_add(1)
                        .ok_or(StagingHeaderFooterDisplayError::ArithmeticOverflow)?
            {
                return Err(StagingHeaderFooterDisplayError::FragmentClosure);
            }
            *expected = fragment.after_position();
            commands.push(StagingHeaderFooterPaintCommand {
                command_ordinal: u32::try_from(commands.len())
                    .map_err(|_| StagingHeaderFooterDisplayError::ArithmeticOverflow)?,
                page_index: page.page_index(),
                master_id: page.master_id().clone(),
                kind: fragment.kind(),
                source_flow_id: fragment.source_flow_id(),
                source_node_id: fragment.source_node_id(),
                block_node_id: fragment.block_node_id(),
                repetition_index: fragment.repetition_index(),
                bounds: fragment.bounds(),
            });
        }
        for frame in page.frames().iter().filter(|frame| {
            matches!(
                frame.kind(),
                StagingAdvancedPageFrameKind::Header | StagingAdvancedPageFrameKind::Footer
            )
        }) {
            let repetition = frame
                .repetition_index()
                .ok_or(StagingHeaderFooterDisplayError::FragmentClosure)?;
            if expected_positions
                .get(&(frame.source_flow_id(), repetition))
                .copied()
                .unwrap_or(0)
                != frame.after_position().ordinal()
            {
                return Err(StagingHeaderFooterDisplayError::FragmentClosure);
            }
        }
        let command_count = u32::try_from(commands.len())
            .map_err(|_| StagingHeaderFooterDisplayError::ArithmeticOverflow)?
            .checked_sub(first_command)
            .ok_or(StagingHeaderFooterDisplayError::ArithmeticOverflow)?;
        pages.push(StagingHeaderFooterDisplayPage {
            page_index: page.page_index(),
            master_id: page.master_id().clone(),
            boxes: page.boxes(),
            first_command,
            command_count,
        });
    }
    let canonical_jcs = encode_display(
        selected.receipt().profile_receipt_sha256(),
        selected.receipt().flow_registry_sha256(),
        selected.receipt().selected_layout_sha256(),
        &pages,
        &commands,
    );
    let receipt = StagingHeaderFooterDisplayReceipt {
        profile_receipt_sha256: selected.receipt().profile_receipt_sha256(),
        flow_registry_sha256: selected.receipt().flow_registry_sha256(),
        selected_layout_sha256: selected.receipt().selected_layout_sha256(),
        paint_closure_sha256: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    let display = StagingHeaderFooterDisplay {
        pages,
        commands,
        receipt,
    };
    display.verify_receipt()?;
    Ok(display)
}

fn validate_frame_order(
    frames: &[typaxis_pagination::StagingSelectedAdvancedFrame],
) -> Result<(), StagingHeaderFooterDisplayError> {
    if frames.is_empty()
        || frames
            .windows(2)
            .any(|pair| pair[0].kind() >= pair[1].kind())
        || frames
            .iter()
            .filter(|frame| frame.kind() == StagingAdvancedPageFrameKind::Body)
            .count()
            != 1
        || frames.iter().any(|frame| match frame.kind() {
            StagingAdvancedPageFrameKind::Body => {
                frame.column_index() != Some(0) || frame.repetition_index().is_some()
            }
            StagingAdvancedPageFrameKind::Header | StagingAdvancedPageFrameKind::Footer => {
                frame.column_index().is_some()
                    || frame.repetition_index().is_none()
                    || frame.before_position().ordinal() != 0
                    || !frame.terminal()
            }
        })
    {
        return Err(StagingHeaderFooterDisplayError::FrameOrder);
    }
    Ok(())
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

fn encode_display(
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    pages: &[StagingHeaderFooterDisplayPage],
    commands: &[StagingHeaderFooterPaintCommand],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ADVANCED_PAINT_CLOSURE_ALGORITHM);
    output.push_str(",\"commands\":[");
    for (index, command) in commands.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"block_node_id\":");
        output.push_str(&command.block_node_id.get().to_string());
        output.push_str(",\"bounds\":");
        push_rect(&mut output, command.bounds);
        output.push_str(",\"command_ordinal\":");
        output.push_str(&command.command_ordinal.to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, command.kind.as_str());
        output.push_str(",\"master_id\":");
        push_jcs_string(&mut output, command.master_id.as_str());
        output.push_str(",\"page_index\":");
        output.push_str(&command.page_index.to_string());
        output.push_str(",\"repetition_index\":");
        output.push_str(&command.repetition_index.to_string());
        output.push_str(",\"source_flow_id\":");
        output.push_str(&command.source_flow_id.get().to_string());
        output.push_str(",\"source_node_id\":");
        output.push_str(&command.source_node_id.get().to_string());
        output.push('}');
    }
    output.push_str("],\"flow_registry_sha256\":");
    push_hex(&mut output, flow_registry_sha256);
    output.push_str(",\"pages\":[");
    for (index, page) in pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"boxes\":{\"crop\":");
        push_box(&mut output, page.boxes.crop_box());
        output.push_str(",\"media\":");
        push_box(&mut output, page.boxes.media_box());
        output.push_str(",\"trim\":");
        push_box(&mut output, page.boxes.trim_box());
        output.push_str("},\"command_count\":");
        output.push_str(&page.command_count.to_string());
        output.push_str(",\"first_command\":");
        output.push_str(&page.first_command.to_string());
        output.push_str(",\"master_id\":");
        push_jcs_string(&mut output, page.master_id.as_str());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push('}');
    }
    output.push_str("],\"profile_receipt_sha256\":");
    push_hex(&mut output, profile_receipt_sha256);
    output.push_str(",\"selected_layout_sha256\":");
    push_hex(&mut output, selected_layout_sha256);
    output.push('}');
    output
}

fn push_rect(output: &mut String, value: Rect) {
    output.push_str("{\"height\":");
    output.push_str(&value.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&value.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&value.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&value.y().raw().to_string());
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
