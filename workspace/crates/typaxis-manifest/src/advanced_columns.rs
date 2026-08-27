use typaxis_core::{push_jcs_string, Rect};
use typaxis_display_list::StagingColumnsDisplay;
use typaxis_machine_profile::STAGING_COLUMNS_PROFILE_ID;
use typaxis_pagination::{
    StagingAdvancedFlowPosition, StagingColumnBalanceReceipt, StagingColumnsSelectedLayout,
    StagingPageMargins, StagingPdfPageBox, StagingSelectedAdvancedFrame, COLUMN_BALANCE_ALGORITHM,
};
use typaxis_pdf::StagingColumnsPdf;

use crate::advanced_header_footer::{
    StagingAdvancedPaginationManifest, StagingAdvancedPaginationManifestError,
    ADVANCED_PAGINATION_MANIFEST_ALGORITHM,
};

pub fn project_staging_columns_manifest(
    selected: &StagingColumnsSelectedLayout,
    display: &StagingColumnsDisplay,
    pdf: &StagingColumnsPdf,
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
    match display.pages().len().cmp(&selected.pages().len()) {
        std::cmp::Ordering::Less => {
            return Err(StagingAdvancedPaginationManifestError::MissingPage)
        }
        std::cmp::Ordering::Greater => {
            return Err(StagingAdvancedPaginationManifestError::ExtraPage)
        }
        std::cmp::Ordering::Equal => {}
    }
    match pdf.pages().len().cmp(&selected.pages().len()) {
        std::cmp::Ordering::Less => {
            return Err(StagingAdvancedPaginationManifestError::MissingPage)
        }
        std::cmp::Ordering::Greater => {
            return Err(StagingAdvancedPaginationManifestError::ExtraPage)
        }
        std::cmp::Ordering::Equal => {}
    }

    let mut command_offset = 0usize;
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
        let expected_count = u32::try_from(selected_page.fragments().len())
            .map_err(|_| StagingAdvancedPaginationManifestError::ArithmeticOverflow)?;
        if display_page.command_count() != expected_count
            || pdf_page.command_count() != expected_count
            || usize::try_from(display_page.first_command()) != Ok(command_offset)
        {
            return Err(StagingAdvancedPaginationManifestError::WrongPaint);
        }
        let command_end = command_offset
            .checked_add(selected_page.fragments().len())
            .ok_or(StagingAdvancedPaginationManifestError::ArithmeticOverflow)?;
        let commands = display
            .commands()
            .get(command_offset..command_end)
            .ok_or(StagingAdvancedPaginationManifestError::WrongPaint)?;
        for (fragment, command) in selected_page.fragments().iter().zip(commands) {
            if fragment.page_index() != command.page_index()
                || fragment.column_index() != command.column_index()
                || fragment.frame_flow_id() != command.frame_flow_id()
                || fragment.source_flow_id() != command.source_flow_id()
                || fragment.block_node_id() != command.block_node_id()
                || fragment.bounds() != command.bounds()
            {
                return Err(StagingAdvancedPaginationManifestError::WrongColumn);
            }
        }
        command_offset = command_end;
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
        if selected_page.balance().is_some()
            && selected_page.page_index()
                != u32::try_from(selected.pages().len() - 1)
                    .map_err(|_| StagingAdvancedPaginationManifestError::ArithmeticOverflow)?
        {
            return Err(StagingAdvancedPaginationManifestError::WrongBalance);
        }
    }
    if command_offset != display.commands().len() {
        return Err(StagingAdvancedPaginationManifestError::WrongPaint);
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
    ))
}

fn encode_projection(
    selected: &StagingColumnsSelectedLayout,
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
        output.push_str("{\"balance\":");
        match page.balance() {
            Some(balance) => push_balance(&mut output, balance),
            None => output.push_str("null"),
        }
        output.push_str(",\"crop_box\":");
        push_box(&mut output, page.boxes().crop_box());
        output.push_str(",\"float_carries\":[],\"float_placements\":[],\"float_queue_after\":[],\"float_queue_before\":[],\"frames\":[");
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
    push_jcs_string(&mut output, STAGING_COLUMNS_PROFILE_ID);
    output.push_str(",\"profile_receipt_sha256\":");
    push_hex(&mut output, profile_receipt_sha256);
    output.push_str(",\"selected_layout_sha256\":");
    push_hex(&mut output, selected_layout_sha256);
    output.push('}');
    output
}

fn push_balance(output: &mut String, balance: &StagingColumnBalanceReceipt) {
    output.push_str("{\"algorithm\":");
    push_jcs_string(output, COLUMN_BALANCE_ALGORITHM);
    output.push_str(",\"candidate_count\":");
    output.push_str(&balance.candidate_count().to_string());
    output.push_str(",\"input_sha256\":");
    push_hex(output, balance.input_sha256());
    output.push_str(",\"receipt_sha256\":");
    push_hex(output, balance.receipt_sha256());
    output.push_str(",\"selected_target_height\":");
    output.push_str(&balance.selected_target_height().get().raw().to_string());
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
    fn columns_manifest_closes_selected_display_pdf_and_balance_receipts() {
        let selected = typaxis_pagination::staging_columns_selected_fixture();
        let display = typaxis_display_list::build_staging_columns_display(&selected).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let pdf = typaxis_pdf::serialize_staging_columns_pdf(&display, &limits).unwrap();
        let manifest = project_staging_columns_manifest(&selected, &display, &pdf).unwrap();

        assert!(manifest
            .canonical_jcs()
            .contains("\"profile\":\"typaxis.machine-pdf/columns-1\""));
        assert!(manifest
            .canonical_jcs()
            .contains("\"algorithm\":\"typaxis.column-balance-candidates/1\""));
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
