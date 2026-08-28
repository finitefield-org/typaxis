use std::collections::BTreeMap;
use typaxis_core::{push_jcs_string, Rect};
use typaxis_display_list::{
    StagingFloatDisplay, StagingFloatPaintCommand, StagingFloatPaintCommandKind,
};
use typaxis_machine_profile::STAGING_FLOAT_PROFILE_ID;
use typaxis_pagination::{
    StagingAdvancedFlowPosition, StagingFloatBodyFragment, StagingFloatCarry,
    StagingFloatPlacement, StagingFloatQueueEntry, StagingFloatSelectedLayout, StagingPageMargins,
    StagingPdfPageBox, StagingSelectedAdvancedFrame,
};
use typaxis_pdf::StagingFloatPdf;

use crate::advanced_header_footer::{
    StagingAdvancedPaginationManifest, StagingAdvancedPaginationManifestError,
    ADVANCED_PAGINATION_MANIFEST_ALGORITHM,
};

pub fn project_staging_float_manifest(
    selected: &StagingFloatSelectedLayout,
    display: &StagingFloatDisplay,
    pdf: &StagingFloatPdf,
) -> Result<StagingAdvancedPaginationManifest, StagingAdvancedPaginationManifestError> {
    selected
        .verify_receipt()
        .map_err(|_| StagingAdvancedPaginationManifestError::ReceiptMismatch)?;
    display
        .verify_receipt()
        .map_err(|_| StagingAdvancedPaginationManifestError::ReceiptMismatch)?;
    pdf.verify_receipt(display)
        .map_err(|_| StagingAdvancedPaginationManifestError::ReceiptMismatch)?;
    if selected.receipt().profile_receipt_sha256() != display.receipt().profile_receipt_sha256()
        || selected.receipt().flow_registry_sha256() != display.receipt().flow_registry_sha256()
        || selected.receipt().selected_layout_sha256() != display.receipt().selected_layout_sha256()
        || display.receipt().paint_closure_sha256() != pdf.receipt().display_paint_sha256()
    {
        return Err(StagingAdvancedPaginationManifestError::ReceiptMismatch);
    }
    compare_page_count(display.pages().len(), selected.pages().len())?;
    compare_page_count(pdf.pages().len(), selected.pages().len())?;

    for ((selected_page, display_page), pdf_page) in selected
        .pages()
        .iter()
        .zip(display.pages())
        .zip(pdf.pages())
    {
        if selected_page.page_index() != display_page.page_index()
            || selected_page.page_index() != pdf_page.page_index()
        {
            return Err(StagingAdvancedPaginationManifestError::WrongPage);
        }
        if selected_page.master_id() != display_page.master_id()
            || selected_page.master_id() != pdf_page.master_id()
        {
            return Err(StagingAdvancedPaginationManifestError::WrongMaster);
        }
        if selected_page.boxes() != display_page.boxes()
            || selected_page.boxes() != pdf_page.boxes()
        {
            return Err(StagingAdvancedPaginationManifestError::WrongBox);
        }
        let expected_count = selected_page
            .body_fragments()
            .len()
            .checked_add(selected_page.placements().len())
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(StagingAdvancedPaginationManifestError::ArithmeticOverflow)?;
        if expected_count != display_page.command_count()
            || expected_count != pdf_page.command_count()
            || selected_page.placements().len() != pdf_page.float_object_usages().len()
        {
            return Err(StagingAdvancedPaginationManifestError::WrongPaint);
        }
        let first = usize::try_from(display_page.first_command())
            .map_err(|_| StagingAdvancedPaginationManifestError::ArithmeticOverflow)?;
        let end = first
            .checked_add(
                usize::try_from(display_page.command_count())
                    .map_err(|_| StagingAdvancedPaginationManifestError::ArithmeticOverflow)?,
            )
            .ok_or(StagingAdvancedPaginationManifestError::ArithmeticOverflow)?;
        let commands = display
            .commands()
            .get(first..end)
            .ok_or(StagingAdvancedPaginationManifestError::WrongPaint)?;
        close_page_paint(
            selected_page.body_fragments(),
            selected_page.placements(),
            commands,
        )?;
        for (placement, usage) in selected_page
            .placements()
            .iter()
            .zip(pdf_page.float_object_usages())
        {
            if placement.float_flow_id().get() != usage.float_flow_id()
                || placement.figure_node_id() != usage.figure_node_id()
            {
                return Err(StagingAdvancedPaginationManifestError::WrongPaint);
            }
        }
        if selected_page
            .frames()
            .iter()
            .enumerate()
            .any(|(index, frame)| {
                frame.column_index() != u32::try_from(index).ok()
                    || frame.repetition_index().is_some()
            })
        {
            return Err(StagingAdvancedPaginationManifestError::WrongColumn);
        }
    }

    let profile_receipt_sha256 = selected.receipt().profile_receipt_sha256();
    let flow_registry_sha256 = selected.receipt().flow_registry_sha256();
    let selected_layout_sha256 = selected.receipt().selected_layout_sha256();
    let paint_closure_sha256 = pdf.receipt().paint_closure_sha256();
    let canonical_jcs = encode_projection(
        selected,
        profile_receipt_sha256,
        flow_registry_sha256,
        selected_layout_sha256,
        paint_closure_sha256,
    );
    Ok(StagingAdvancedPaginationManifest::from_projection(
        canonical_jcs,
        profile_receipt_sha256,
        flow_registry_sha256,
        selected_layout_sha256,
        paint_closure_sha256,
        typaxis_core::MachinePdfProfileId::FLOAT_1,
        u32::try_from(selected.pages().len())
            .map_err(|_| StagingAdvancedPaginationManifestError::ArithmeticOverflow)?,
    ))
}

fn compare_page_count(
    observed: usize,
    expected: usize,
) -> Result<(), StagingAdvancedPaginationManifestError> {
    match observed.cmp(&expected) {
        std::cmp::Ordering::Less => Err(StagingAdvancedPaginationManifestError::MissingPage),
        std::cmp::Ordering::Greater => Err(StagingAdvancedPaginationManifestError::ExtraPage),
        std::cmp::Ordering::Equal => Ok(()),
    }
}

fn close_page_paint(
    fragments: &[StagingFloatBodyFragment],
    placements: &[StagingFloatPlacement],
    commands: &[StagingFloatPaintCommand],
) -> Result<(), StagingAdvancedPaginationManifestError> {
    let mut expected = BTreeMap::new();
    for fragment in fragments {
        if expected
            .insert(
                (fragment.column_index(), fragment.frame_paint_ordinal()),
                (
                    StagingFloatPaintCommandKind::Body,
                    fragment.block_node_id(),
                    None,
                ),
            )
            .is_some()
        {
            return Err(StagingAdvancedPaginationManifestError::WrongPaint);
        }
    }
    for placement in placements {
        if expected
            .insert(
                (placement.column_index(), placement.frame_paint_ordinal()),
                (
                    StagingFloatPaintCommandKind::Float,
                    placement.figure_node_id(),
                    Some(placement.float_flow_id()),
                ),
            )
            .is_some()
        {
            return Err(StagingAdvancedPaginationManifestError::WrongPaint);
        }
    }
    if expected.len() != commands.len() {
        return Err(StagingAdvancedPaginationManifestError::WrongPaint);
    }
    for command in commands {
        let Some((kind, node_id, float_flow_id)) =
            expected.get(&(command.column_index(), command.frame_paint_ordinal()))
        else {
            return Err(StagingAdvancedPaginationManifestError::WrongPaint);
        };
        if command.kind() != *kind
            || command.node_id() != *node_id
            || command.float_flow_id() != *float_flow_id
        {
            return Err(StagingAdvancedPaginationManifestError::WrongPaint);
        }
        match command.kind() {
            StagingFloatPaintCommandKind::Body => {
                let fragment = fragments
                    .iter()
                    .find(|fragment| {
                        fragment.column_index() == command.column_index()
                            && fragment.frame_paint_ordinal() == command.frame_paint_ordinal()
                    })
                    .ok_or(StagingAdvancedPaginationManifestError::WrongPaint)?;
                if fragment.page_index() != command.page_index()
                    || fragment.frame_flow_id() != command.frame_flow_id()
                    || fragment.source_flow_id() != command.source_flow_id()
                    || fragment.bounds() != command.bounds()
                {
                    return Err(StagingAdvancedPaginationManifestError::WrongColumn);
                }
            }
            StagingFloatPaintCommandKind::Float => {
                let placement = placements
                    .iter()
                    .find(|placement| {
                        placement.column_index() == command.column_index()
                            && placement.frame_paint_ordinal() == command.frame_paint_ordinal()
                    })
                    .ok_or(StagingAdvancedPaginationManifestError::WrongPaint)?;
                if placement.page_index() != command.page_index()
                    || placement.frame_flow_id() != command.frame_flow_id()
                    || placement.source_flow_id() != command.source_flow_id()
                    || placement.class()
                        != command
                            .placement_class()
                            .ok_or(StagingAdvancedPaginationManifestError::WrongPaint)?
                    || placement.bounds() != command.bounds()
                {
                    return Err(StagingAdvancedPaginationManifestError::WrongColumn);
                }
            }
        }
    }
    Ok(())
}

fn encode_projection(
    selected: &StagingFloatSelectedLayout,
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    paint_closure_sha256: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ADVANCED_PAGINATION_MANIFEST_ALGORITHM);
    output.push_str(",\"flow_registry_sha256\":");
    push_hex(&mut output, flow_registry_sha256);
    output.push_str(",\"pages\":[");
    for (index, page) in selected.pages().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"balance\":null,\"crop_box\":");
        push_box(&mut output, page.boxes().crop_box());
        output.push_str(",\"float_carries\":[");
        for (carry_index, carry) in page.carries().iter().enumerate() {
            if carry_index > 0 {
                output.push(',');
            }
            push_carry(&mut output, carry);
        }
        output.push_str("],\"float_placements\":[");
        for (placement_index, placement) in page.placements().iter().enumerate() {
            if placement_index > 0 {
                output.push(',');
            }
            push_placement(&mut output, placement);
        }
        output.push_str("],\"float_queue_after\":[");
        push_queue(&mut output, page.queue_after());
        output.push_str("],\"float_queue_before\":[");
        push_queue(&mut output, page.queue_before());
        output.push_str("],\"frames\":[");
        for (frame_index, frame) in page.frames().iter().enumerate() {
            if frame_index > 0 {
                output.push(',');
            }
            push_frame(&mut output, frame);
        }
        output.push_str("],\"margins\":");
        push_margins(&mut output, page.margins());
        output.push_str(",\"master_id\":");
        push_jcs_string(&mut output, page.master_id().as_str());
        output.push_str(",\"media_box\":");
        push_box(&mut output, page.boxes().media_box());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index().to_string());
        output.push_str(",\"trim_box\":");
        push_box(&mut output, page.boxes().trim_box());
        output.push('}');
    }
    output.push_str("],\"paint_closure_sha256\":");
    push_hex(&mut output, paint_closure_sha256);
    output.push_str(",\"profile\":");
    push_jcs_string(&mut output, STAGING_FLOAT_PROFILE_ID);
    output.push_str(",\"profile_receipt_sha256\":");
    push_hex(&mut output, profile_receipt_sha256);
    output.push_str(",\"selected_layout_sha256\":");
    push_hex(&mut output, selected_layout_sha256);
    output.push('}');
    output
}

fn push_queue(output: &mut String, queue: &[StagingFloatQueueEntry]) {
    for (index, entry) in queue.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"anchor_body_flow_id\":");
        output.push_str(&entry.anchor_body_flow_id().get().to_string());
        output.push_str(",\"anchor_position\":");
        push_position(output, entry.anchor_position());
        output.push_str(",\"carry_count\":");
        output.push_str(&entry.carry_count().to_string());
        output.push_str(",\"figure_node_id\":");
        output.push_str(&entry.figure_node_id().get().to_string());
        output.push_str(",\"float_flow_id\":");
        output.push_str(&entry.float_flow_id().get().to_string());
        output.push('}');
    }
}

fn push_placement(output: &mut String, placement: &StagingFloatPlacement) {
    output.push_str("{\"bounds\":");
    push_rect(output, placement.bounds());
    output.push_str(",\"caption_terminal\":");
    output.push_str(if placement.caption_terminal() {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"class\":");
    push_jcs_string(output, placement.class().as_str());
    output.push_str(",\"column_index\":");
    output.push_str(&placement.column_index().to_string());
    output.push_str(",\"figure_node_id\":");
    output.push_str(&placement.figure_node_id().get().to_string());
    output.push_str(",\"float_flow_id\":");
    output.push_str(&placement.float_flow_id().get().to_string());
    output.push_str(",\"float_terminal\":");
    output.push_str(if placement.float_terminal() {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"frame_paint_ordinal\":");
    output.push_str(&placement.frame_paint_ordinal().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&placement.page_index().to_string());
    output.push('}');
}

fn push_carry(output: &mut String, carry: &StagingFloatCarry) {
    output.push_str("{\"carry_count\":");
    output.push_str(&carry.carry_count().to_string());
    output.push_str(",\"figure_node_id\":");
    output.push_str(&carry.figure_node_id().get().to_string());
    output.push_str(",\"float_flow_id\":");
    output.push_str(&carry.float_flow_id().get().to_string());
    output.push_str(",\"source_page_index\":");
    output.push_str(&carry.source_page_index().to_string());
    output.push_str(",\"target_page_index\":");
    output.push_str(&carry.target_page_index().to_string());
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
    output.push_str(",\"kind\":");
    push_jcs_string(output, frame.kind().as_str());
    output.push_str(",\"rect\":");
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

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};

    #[test]
    fn floats_manifest_profile_is_reserved_target() {
        assert_eq!(STAGING_FLOAT_PROFILE_ID, "typaxis.machine-pdf/float-1");
    }

    #[test]
    fn floats_manifest_closes_queue_carry_placement_display_and_pdf_usage() {
        let selected = typaxis_pagination::staging_float_selected_fixture();
        let display = typaxis_display_list::build_staging_float_display(&selected).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let pdf = typaxis_pdf::serialize_staging_float_pdf(&display, &limits).unwrap();
        let manifest = project_staging_float_manifest(&selected, &display, &pdf).unwrap();

        assert!(manifest
            .canonical_jcs()
            .contains("\"profile\":\"typaxis.machine-pdf/float-1\""));
        assert!(manifest.canonical_jcs().contains("\"carry_count\":2"));
        assert!(manifest.canonical_jcs().contains("\"class\":\"here\""));
        assert_eq!(
            manifest.selected_layout_sha256(),
            selected.receipt().selected_layout_sha256()
        );
        assert_eq!(
            manifest.paint_closure_sha256(),
            pdf.receipt().paint_closure_sha256()
        );
    }
}
