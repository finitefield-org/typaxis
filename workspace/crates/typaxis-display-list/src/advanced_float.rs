use typaxis_core::{push_jcs_string, sha256, MasterId, NodeId, Rect};
use typaxis_document::FloatPlacementClass;
use typaxis_layout::FlowId;
use typaxis_pagination::{
    StagingAdvancedPageFrameKind, StagingFloatSelectedLayout, StagingPdfPageBox,
    StagingSelectedPageBoxes,
};

use crate::advanced_header_footer::ADVANCED_PAINT_CLOSURE_ALGORITHM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingFloatPaintCommandKind {
    Body,
    Float,
}

impl StagingFloatPaintCommandKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Float => "float",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatPaintCommand {
    command_ordinal: u32,
    frame_paint_ordinal: u32,
    kind: StagingFloatPaintCommandKind,
    page_index: u32,
    master_id: MasterId,
    column_index: u32,
    frame_flow_id: FlowId,
    source_flow_id: FlowId,
    node_id: NodeId,
    float_flow_id: Option<FlowId>,
    placement_class: Option<FloatPlacementClass>,
    bounds: Rect,
}

impl StagingFloatPaintCommand {
    pub const fn command_ordinal(&self) -> u32 {
        self.command_ordinal
    }
    pub const fn frame_paint_ordinal(&self) -> u32 {
        self.frame_paint_ordinal
    }
    pub const fn kind(&self) -> StagingFloatPaintCommandKind {
        self.kind
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
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn float_flow_id(&self) -> Option<FlowId> {
        self.float_flow_id
    }
    pub const fn placement_class(&self) -> Option<FloatPlacementClass> {
        self.placement_class
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatDisplayPage {
    page_index: u32,
    master_id: MasterId,
    boxes: StagingSelectedPageBoxes,
    first_command: u32,
    command_count: u32,
    float_command_count: u32,
}

impl StagingFloatDisplayPage {
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
    pub const fn float_command_count(&self) -> u32 {
        self.float_command_count
    }
}

#[derive(Debug)]
pub struct StagingFloatDisplayReceipt {
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    paint_closure_sha256: [u8; 32],
    canonical_jcs: String,
}

impl StagingFloatDisplayReceipt {
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
pub struct StagingFloatDisplay {
    pages: Vec<StagingFloatDisplayPage>,
    commands: Vec<StagingFloatPaintCommand>,
    receipt: StagingFloatDisplayReceipt,
}

impl StagingFloatDisplay {
    pub fn pages(&self) -> &[StagingFloatDisplayPage] {
        &self.pages
    }
    pub fn commands(&self) -> &[StagingFloatPaintCommand] {
        &self.commands
    }
    pub const fn receipt(&self) -> &StagingFloatDisplayReceipt {
        &self.receipt
    }

    pub fn verify_receipt(&self) -> Result<(), StagingFloatDisplayError> {
        if self.pages.is_empty() {
            return Err(StagingFloatDisplayError::SelectedLayoutMismatch);
        }
        let mut next_command = 0usize;
        for (page_ordinal, page) in self.pages.iter().enumerate() {
            let first = u32::try_from(next_command)
                .map_err(|_| StagingFloatDisplayError::ArithmeticOverflow)?;
            if u32::try_from(page_ordinal) != Ok(page.page_index) || page.first_command != first {
                return Err(StagingFloatDisplayError::NonCanonicalPage);
            }
            let count = usize::try_from(page.command_count)
                .map_err(|_| StagingFloatDisplayError::ArithmeticOverflow)?;
            let end = next_command
                .checked_add(count)
                .ok_or(StagingFloatDisplayError::ArithmeticOverflow)?;
            let commands = self
                .commands
                .get(next_command..end)
                .ok_or(StagingFloatDisplayError::PaintClosure)?;
            if commands.iter().any(|command| {
                command.page_index != page.page_index || command.master_id != page.master_id
            }) || usize::try_from(page.float_command_count).ok()
                != Some(
                    commands
                        .iter()
                        .filter(|command| command.kind == StagingFloatPaintCommandKind::Float)
                        .count(),
                )
            {
                return Err(StagingFloatDisplayError::PaintClosure);
            }
            let mut previous: Option<(u32, u32)> = None;
            for command in commands {
                let key = (command.column_index, command.frame_paint_ordinal);
                if previous.is_some_and(|value| {
                    value.0 > key.0 || (value.0 == key.0 && value.1.checked_add(1) != Some(key.1))
                }) {
                    return Err(StagingFloatDisplayError::FrameOrder);
                }
                if previous.map_or(true, |value| value.0 != key.0) && key.1 != 0 {
                    return Err(StagingFloatDisplayError::FrameOrder);
                }
                previous = Some(key);
                match command.kind {
                    StagingFloatPaintCommandKind::Body
                        if command.float_flow_id.is_some() || command.placement_class.is_some() =>
                    {
                        return Err(StagingFloatDisplayError::PaintClosure)
                    }
                    StagingFloatPaintCommandKind::Float
                        if command.float_flow_id.is_none()
                            || command.placement_class.is_none()
                            || command.placement_class == Some(FloatPlacementClass::NextPage) =>
                    {
                        return Err(StagingFloatDisplayError::PaintClosure)
                    }
                    _ => {}
                }
            }
            next_command = end;
        }
        if next_command != self.commands.len()
            || self
                .commands
                .iter()
                .enumerate()
                .any(|(index, command)| u32::try_from(index) != Ok(command.command_ordinal))
        {
            return Err(StagingFloatDisplayError::PaintClosure);
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
            return Err(StagingFloatDisplayError::SelectedLayoutMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingFloatDisplayError {
    SelectedLayoutMismatch,
    NonCanonicalPage,
    FrameOrder,
    PaintClosure,
    FragmentOutsideFrame,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingFloatDisplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::SelectedLayoutMismatch => "I9190: float selected-layout mismatch",
            Self::NonCanonicalPage => "I9190: float Display page order mismatch",
            Self::FrameOrder => "I9190: float frame paint order mismatch",
            Self::PaintClosure => "I9190: float paint closure mismatch",
            Self::FragmentOutsideFrame => "I9190: float command outside selected frame",
            Self::ArithmeticOverflow => "I9190: float Display arithmetic overflow",
            Self::AllocationFailure => "L5110: float Display allocation failure",
        })
    }
}

impl std::error::Error for StagingFloatDisplayError {}

pub fn build_staging_float_display(
    selected: &StagingFloatSelectedLayout,
) -> Result<StagingFloatDisplay, StagingFloatDisplayError> {
    selected
        .verify_receipt()
        .map_err(|_| StagingFloatDisplayError::SelectedLayoutMismatch)?;
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(selected.pages().len())
        .map_err(|_| StagingFloatDisplayError::AllocationFailure)?;
    let total_commands = selected
        .pages()
        .iter()
        .try_fold(0usize, |count, page| {
            count
                .checked_add(page.body_fragments().len())?
                .checked_add(page.placements().len())
        })
        .ok_or(StagingFloatDisplayError::ArithmeticOverflow)?;
    let mut commands = Vec::new();
    commands
        .try_reserve_exact(total_commands)
        .map_err(|_| StagingFloatDisplayError::AllocationFailure)?;

    for (page_ordinal, page) in selected.pages().iter().enumerate() {
        if u32::try_from(page_ordinal) != Ok(page.page_index()) {
            return Err(StagingFloatDisplayError::NonCanonicalPage);
        }
        validate_frames(page.frames())?;
        let first_command = u32::try_from(commands.len())
            .map_err(|_| StagingFloatDisplayError::ArithmeticOverflow)?;
        let mut page_commands = Vec::new();
        page_commands
            .try_reserve_exact(page.body_fragments().len() + page.placements().len())
            .map_err(|_| StagingFloatDisplayError::AllocationFailure)?;
        for fragment in page.body_fragments() {
            let frame = page
                .frames()
                .get(
                    usize::try_from(fragment.column_index())
                        .map_err(|_| StagingFloatDisplayError::ArithmeticOverflow)?,
                )
                .ok_or(StagingFloatDisplayError::PaintClosure)?;
            if frame.frame_flow_id() != fragment.frame_flow_id()
                || frame.source_flow_id() != fragment.source_flow_id()
                || !rect_contains(frame.rect(), fragment.bounds())
            {
                return Err(StagingFloatDisplayError::FragmentOutsideFrame);
            }
            page_commands.push(StagingFloatPaintCommand {
                command_ordinal: 0,
                frame_paint_ordinal: fragment.frame_paint_ordinal(),
                kind: StagingFloatPaintCommandKind::Body,
                page_index: page.page_index(),
                master_id: page.master_id().clone(),
                column_index: fragment.column_index(),
                frame_flow_id: fragment.frame_flow_id(),
                source_flow_id: fragment.source_flow_id(),
                node_id: fragment.block_node_id(),
                float_flow_id: None,
                placement_class: None,
                bounds: fragment.bounds(),
            });
        }
        for placement in page.placements() {
            let frame = page
                .frames()
                .get(
                    usize::try_from(placement.column_index())
                        .map_err(|_| StagingFloatDisplayError::ArithmeticOverflow)?,
                )
                .ok_or(StagingFloatDisplayError::PaintClosure)?;
            if frame.frame_flow_id() != placement.frame_flow_id()
                || frame.source_flow_id() != placement.source_flow_id()
                || !rect_contains(frame.rect(), placement.bounds())
            {
                return Err(StagingFloatDisplayError::FragmentOutsideFrame);
            }
            page_commands.push(StagingFloatPaintCommand {
                command_ordinal: 0,
                frame_paint_ordinal: placement.frame_paint_ordinal(),
                kind: StagingFloatPaintCommandKind::Float,
                page_index: page.page_index(),
                master_id: page.master_id().clone(),
                column_index: placement.column_index(),
                frame_flow_id: placement.frame_flow_id(),
                source_flow_id: placement.source_flow_id(),
                node_id: placement.figure_node_id(),
                float_flow_id: Some(placement.float_flow_id()),
                placement_class: Some(placement.class()),
                bounds: placement.bounds(),
            });
        }
        page_commands.sort_by_key(|command| (command.column_index, command.frame_paint_ordinal));
        for mut command in page_commands {
            command.command_ordinal = u32::try_from(commands.len())
                .map_err(|_| StagingFloatDisplayError::ArithmeticOverflow)?;
            commands.push(command);
        }
        let command_count = u32::try_from(commands.len())
            .map_err(|_| StagingFloatDisplayError::ArithmeticOverflow)?
            .checked_sub(first_command)
            .ok_or(StagingFloatDisplayError::ArithmeticOverflow)?;
        let float_command_count = u32::try_from(page.placements().len())
            .map_err(|_| StagingFloatDisplayError::ArithmeticOverflow)?;
        pages.push(StagingFloatDisplayPage {
            page_index: page.page_index(),
            master_id: page.master_id().clone(),
            boxes: page.boxes(),
            first_command,
            command_count,
            float_command_count,
        });
    }
    let canonical_jcs = encode_display(
        selected.receipt().profile_receipt_sha256(),
        selected.receipt().flow_registry_sha256(),
        selected.receipt().selected_layout_sha256(),
        &pages,
        &commands,
    );
    let receipt = StagingFloatDisplayReceipt {
        profile_receipt_sha256: selected.receipt().profile_receipt_sha256(),
        flow_registry_sha256: selected.receipt().flow_registry_sha256(),
        selected_layout_sha256: selected.receipt().selected_layout_sha256(),
        paint_closure_sha256: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    let display = StagingFloatDisplay {
        pages,
        commands,
        receipt,
    };
    display.verify_receipt()?;
    Ok(display)
}

fn validate_frames(
    frames: &[typaxis_pagination::StagingSelectedAdvancedFrame],
) -> Result<(), StagingFloatDisplayError> {
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
        return Err(StagingFloatDisplayError::FrameOrder);
    }
    Ok(())
}

fn encode_display(
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    pages: &[StagingFloatDisplayPage],
    commands: &[StagingFloatPaintCommand],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ADVANCED_PAINT_CLOSURE_ALGORITHM);
    output.push_str(",\"commands\":[");
    for (index, command) in commands.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"bounds\":");
        push_rect(&mut output, command.bounds);
        output.push_str(",\"column_index\":");
        output.push_str(&command.column_index.to_string());
        output.push_str(",\"command_ordinal\":");
        output.push_str(&command.command_ordinal.to_string());
        output.push_str(",\"float_flow_id\":");
        match command.float_flow_id {
            Some(value) => output.push_str(&value.get().to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"frame_flow_id\":");
        output.push_str(&command.frame_flow_id.get().to_string());
        output.push_str(",\"frame_paint_ordinal\":");
        output.push_str(&command.frame_paint_ordinal.to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, command.kind.as_str());
        output.push_str(",\"master_id\":");
        push_jcs_string(&mut output, command.master_id.as_str());
        output.push_str(",\"node_id\":");
        output.push_str(&command.node_id.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&command.page_index.to_string());
        output.push_str(",\"placement_class\":");
        match command.placement_class {
            Some(value) => push_jcs_string(&mut output, value.as_str()),
            None => output.push_str("null"),
        }
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
        output.push_str(",\"float_command_count\":");
        output.push_str(&page.float_command_count.to_string());
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
pub fn staging_float_display_fixture() -> StagingFloatDisplay {
    let selected = typaxis_pagination::staging_float_selected_fixture();
    build_staging_float_display(&selected).expect("float Display fixture closes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_paint_kind_is_closed() {
        assert_eq!(StagingFloatPaintCommandKind::Body.as_str(), "body");
        assert_eq!(StagingFloatPaintCommandKind::Float.as_str(), "float");
    }

    #[test]
    fn floats_display_closes_body_and_placed_float_frame_paint_order() {
        let display = staging_float_display_fixture();
        assert_eq!(display.pages().len(), 3);
        assert_eq!(display.commands().len(), 7);
        assert_eq!(
            display
                .commands()
                .iter()
                .filter(|command| command.kind() == StagingFloatPaintCommandKind::Float)
                .count(),
            5
        );
        assert_eq!(
            display.commands()[0].kind(),
            StagingFloatPaintCommandKind::Body
        );
        assert_eq!(
            display.commands()[1].placement_class(),
            Some(FloatPlacementClass::Here)
        );
        display.verify_receipt().unwrap();
    }

    #[test]
    fn floats_display_rejects_duplicate_missing_wrong_page_class_and_frame_order() {
        let mut duplicate = staging_float_display_fixture();
        duplicate.commands.push(duplicate.commands[0].clone());
        assert!(matches!(
            duplicate.verify_receipt(),
            Err(StagingFloatDisplayError::PaintClosure)
        ));

        let mut missing = staging_float_display_fixture();
        missing.commands.pop();
        assert!(matches!(
            missing.verify_receipt(),
            Err(StagingFloatDisplayError::PaintClosure)
        ));

        let mut wrong_page = staging_float_display_fixture();
        wrong_page.commands[0].page_index = 1;
        assert!(matches!(
            wrong_page.verify_receipt(),
            Err(StagingFloatDisplayError::PaintClosure)
        ));

        let mut wrong_class = staging_float_display_fixture();
        wrong_class.commands[1].placement_class = Some(FloatPlacementClass::Bottom);
        assert!(matches!(
            wrong_class.verify_receipt(),
            Err(StagingFloatDisplayError::SelectedLayoutMismatch)
        ));

        let mut wrong_order = staging_float_display_fixture();
        wrong_order.commands.swap(0, 1);
        assert!(matches!(
            wrong_order.verify_receipt(),
            Err(StagingFloatDisplayError::FrameOrder)
                | Err(StagingFloatDisplayError::SelectedLayoutMismatch)
        ));
    }
}
