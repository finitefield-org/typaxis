use typaxis_core::{push_jcs_string, sha256, MasterId, NodeId, Rect};
use typaxis_layout::FlowId;
use typaxis_pagination::{
    StagingAdvancedPageFrameKind, StagingColumnsSelectedLayout, StagingPdfPageBox,
    StagingSelectedPageBoxes,
};

use crate::advanced_header_footer::ADVANCED_PAINT_CLOSURE_ALGORITHM;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingColumnPaintCommand {
    command_ordinal: u32,
    page_index: u32,
    master_id: MasterId,
    column_index: u32,
    frame_flow_id: FlowId,
    source_flow_id: FlowId,
    block_node_id: NodeId,
    bounds: Rect,
}

impl StagingColumnPaintCommand {
    pub const fn command_ordinal(&self) -> u32 {
        self.command_ordinal
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
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
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingColumnsDisplayPage {
    page_index: u32,
    master_id: MasterId,
    boxes: StagingSelectedPageBoxes,
    first_command: u32,
    command_count: u32,
}

impl StagingColumnsDisplayPage {
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
pub struct StagingColumnsDisplayReceipt {
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    paint_closure_sha256: [u8; 32],
    canonical_jcs: String,
}

impl StagingColumnsDisplayReceipt {
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
pub struct StagingColumnsDisplay {
    pages: Vec<StagingColumnsDisplayPage>,
    commands: Vec<StagingColumnPaintCommand>,
    receipt: StagingColumnsDisplayReceipt,
}

impl StagingColumnsDisplay {
    pub fn pages(&self) -> &[StagingColumnsDisplayPage] {
        &self.pages
    }
    pub fn commands(&self) -> &[StagingColumnPaintCommand] {
        &self.commands
    }
    pub const fn receipt(&self) -> &StagingColumnsDisplayReceipt {
        &self.receipt
    }

    pub fn verify_receipt(&self) -> Result<(), StagingColumnsDisplayError> {
        if self.pages.is_empty() {
            return Err(StagingColumnsDisplayError::SelectedLayoutMismatch);
        }
        let mut next_command = 0usize;
        for (index, page) in self.pages.iter().enumerate() {
            let expected_page =
                u32::try_from(index).map_err(|_| StagingColumnsDisplayError::ArithmeticOverflow)?;
            let expected_first = u32::try_from(next_command)
                .map_err(|_| StagingColumnsDisplayError::ArithmeticOverflow)?;
            if page.page_index != expected_page || page.first_command != expected_first {
                return Err(StagingColumnsDisplayError::NonCanonicalPage);
            }
            next_command = next_command
                .checked_add(
                    usize::try_from(page.command_count)
                        .map_err(|_| StagingColumnsDisplayError::ArithmeticOverflow)?,
                )
                .ok_or(StagingColumnsDisplayError::ArithmeticOverflow)?;
            let first = usize::try_from(page.first_command)
                .map_err(|_| StagingColumnsDisplayError::ArithmeticOverflow)?;
            let commands = self
                .commands
                .get(first..next_command)
                .ok_or(StagingColumnsDisplayError::FragmentClosure)?;
            if commands.iter().any(|command| {
                command.page_index != page.page_index || command.master_id != page.master_id
            }) {
                return Err(StagingColumnsDisplayError::FragmentClosure);
            }
        }
        if next_command != self.commands.len()
            || self
                .commands
                .iter()
                .enumerate()
                .any(|(index, command)| u32::try_from(index) != Ok(command.command_ordinal))
        {
            return Err(StagingColumnsDisplayError::FragmentClosure);
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
            return Err(StagingColumnsDisplayError::SelectedLayoutMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingColumnsDisplayError {
    SelectedLayoutMismatch,
    NonCanonicalPage,
    FrameOrder,
    FragmentClosure,
    FragmentOutsideFrame,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingColumnsDisplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::SelectedLayoutMismatch => "I9190: columns selected-layout mismatch",
            Self::NonCanonicalPage => "I9190: columns Display page order mismatch",
            Self::FrameOrder => "I9190: columns Display frame order mismatch",
            Self::FragmentClosure => "I9190: columns fragment closure mismatch",
            Self::FragmentOutsideFrame => "I9190: column fragment outside selected frame",
            Self::ArithmeticOverflow => "I9190: columns Display arithmetic overflow",
            Self::AllocationFailure => "L5110: columns Display allocation failure",
        })
    }
}

impl std::error::Error for StagingColumnsDisplayError {}

pub fn build_staging_columns_display(
    selected: &StagingColumnsSelectedLayout,
) -> Result<StagingColumnsDisplay, StagingColumnsDisplayError> {
    selected
        .verify_receipt()
        .map_err(|_| StagingColumnsDisplayError::SelectedLayoutMismatch)?;
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(selected.pages().len())
        .map_err(|_| StagingColumnsDisplayError::AllocationFailure)?;
    let fragment_count = selected
        .pages()
        .iter()
        .try_fold(0usize, |count, page| {
            count.checked_add(page.fragments().len())
        })
        .ok_or(StagingColumnsDisplayError::ArithmeticOverflow)?;
    let mut commands = Vec::new();
    commands
        .try_reserve_exact(fragment_count)
        .map_err(|_| StagingColumnsDisplayError::AllocationFailure)?;

    for (page_index, page) in selected.pages().iter().enumerate() {
        if u32::try_from(page_index) != Ok(page.page_index()) {
            return Err(StagingColumnsDisplayError::NonCanonicalPage);
        }
        validate_frames(page.frames())?;
        let first_command = u32::try_from(commands.len())
            .map_err(|_| StagingColumnsDisplayError::ArithmeticOverflow)?;
        let mut previous: Option<(u32, u32)> = None;
        for fragment in page.fragments() {
            if fragment.page_index() != page.page_index()
                || previous.is_some_and(|value| {
                    value > (fragment.column_index(), fragment.before_position())
                })
            {
                return Err(StagingColumnsDisplayError::FragmentClosure);
            }
            previous = Some((fragment.column_index(), fragment.before_position()));
            let frame = page
                .frames()
                .get(
                    usize::try_from(fragment.column_index())
                        .map_err(|_| StagingColumnsDisplayError::ArithmeticOverflow)?,
                )
                .ok_or(StagingColumnsDisplayError::FragmentClosure)?;
            if frame.column_index() != Some(fragment.column_index())
                || frame.frame_flow_id() != fragment.frame_flow_id()
                || frame.source_flow_id() != fragment.source_flow_id()
                || !rect_contains(frame.rect(), fragment.bounds())
            {
                return Err(StagingColumnsDisplayError::FragmentOutsideFrame);
            }
            commands.push(StagingColumnPaintCommand {
                command_ordinal: u32::try_from(commands.len())
                    .map_err(|_| StagingColumnsDisplayError::ArithmeticOverflow)?,
                page_index: page.page_index(),
                master_id: page.master_id().clone(),
                column_index: fragment.column_index(),
                frame_flow_id: fragment.frame_flow_id(),
                source_flow_id: fragment.source_flow_id(),
                block_node_id: fragment.block_node_id(),
                bounds: fragment.bounds(),
            });
        }
        let command_count = u32::try_from(commands.len())
            .map_err(|_| StagingColumnsDisplayError::ArithmeticOverflow)?
            .checked_sub(first_command)
            .ok_or(StagingColumnsDisplayError::ArithmeticOverflow)?;
        pages.push(StagingColumnsDisplayPage {
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
    let receipt = StagingColumnsDisplayReceipt {
        profile_receipt_sha256: selected.receipt().profile_receipt_sha256(),
        flow_registry_sha256: selected.receipt().flow_registry_sha256(),
        selected_layout_sha256: selected.receipt().selected_layout_sha256(),
        paint_closure_sha256: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    let display = StagingColumnsDisplay {
        pages,
        commands,
        receipt,
    };
    display.verify_receipt()?;
    Ok(display)
}

fn validate_frames(
    frames: &[typaxis_pagination::StagingSelectedAdvancedFrame],
) -> Result<(), StagingColumnsDisplayError> {
    if frames.is_empty()
        || frames.iter().enumerate().any(|(index, frame)| {
            frame.kind() != StagingAdvancedPageFrameKind::Body
                || frame.column_index() != u32::try_from(index).ok()
                || frame.repetition_index().is_some()
                || frame.source_flow_id() != FlowId::DOCUMENT_BODY
        })
        || frames
            .windows(2)
            .any(|pair| pair[0].after_position().ordinal() != pair[1].before_position().ordinal())
    {
        return Err(StagingColumnsDisplayError::FrameOrder);
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
    pages: &[StagingColumnsDisplayPage],
    commands: &[StagingColumnPaintCommand],
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
        output.push_str(",\"column_index\":");
        output.push_str(&command.column_index.to_string());
        output.push_str(",\"command_ordinal\":");
        output.push_str(&command.command_ordinal.to_string());
        output.push_str(",\"frame_flow_id\":");
        output.push_str(&command.frame_flow_id.get().to_string());
        output.push_str(",\"master_id\":");
        push_jcs_string(&mut output, command.master_id.as_str());
        output.push_str(",\"page_index\":");
        output.push_str(&command.page_index.to_string());
        output.push_str(",\"source_flow_id\":");
        output.push_str(&command.source_flow_id.get().to_string());
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

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_columns_display_fixture() -> StagingColumnsDisplay {
    let selected = typaxis_pagination::staging_columns_selected_fixture();
    build_staging_columns_display(&selected).expect("columns Display fixture closes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_display_closes_selected_pages_columns_and_fragments() {
        let selected = typaxis_pagination::staging_columns_selected_fixture();
        let display = build_staging_columns_display(&selected).unwrap();
        assert_eq!(display.pages().len(), 2);
        assert_eq!(display.commands().len(), 5);
        assert_eq!(
            display
                .commands()
                .iter()
                .map(StagingColumnPaintCommand::column_index)
                .collect::<Vec<_>>(),
            [0, 0, 1, 1, 0]
        );
        display.verify_receipt().unwrap();
    }

    #[test]
    fn columns_display_rejects_missing_extra_wrong_column_and_wrong_page_commands() {
        let mut display = staging_columns_display_fixture();
        display.commands[0].column_index = 1;
        assert!(matches!(
            display.verify_receipt(),
            Err(StagingColumnsDisplayError::SelectedLayoutMismatch)
        ));

        let mut missing = staging_columns_display_fixture();
        missing.commands.pop();
        assert!(matches!(
            missing.verify_receipt(),
            Err(StagingColumnsDisplayError::FragmentClosure)
        ));

        let mut extra = staging_columns_display_fixture();
        extra.commands.push(extra.commands[0].clone());
        assert!(matches!(
            extra.verify_receipt(),
            Err(StagingColumnsDisplayError::FragmentClosure)
        ));

        let mut wrong_page = staging_columns_display_fixture();
        wrong_page.commands[0].page_index = 1;
        assert!(matches!(
            wrong_page.verify_receipt(),
            Err(StagingColumnsDisplayError::FragmentClosure)
        ));
    }
}
