use typaxis_core::{push_jcs_string, sha256, Rect};
use typaxis_display_list::StagingHeaderFooterDisplay;
use typaxis_machine_profile::STAGING_HEADER_FOOTER_PROFILE_ID;
use typaxis_pagination::{
    StagingAdvancedFlowPosition, StagingHeaderFooterSelectedLayout, StagingPageMargins,
    StagingPdfPageBox, StagingSelectedAdvancedFrame,
};
use typaxis_pdf::StagingHeaderFooterPdf;

pub const ADVANCED_PAGINATION_MANIFEST_ALGORITHM: &str = "typaxis.advanced-pagination-manifest/1";

#[derive(Debug)]
pub struct StagingAdvancedPaginationManifest {
    canonical_jcs: String,
    fingerprint: [u8; 32],
    profile_receipt_sha256: [u8; 32],
    flow_registry_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    paint_closure_sha256: [u8; 32],
}

impl StagingAdvancedPaginationManifest {
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
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

    /// Both artifact owners embed this exact byte sequence.  They do not
    /// independently reconstruct or reorder the advanced projection.
    pub fn wrapped_artifact_jcs(&self) -> String {
        let mut output = String::from("{\"advanced_pagination\":");
        output.push_str(&self.canonical_jcs);
        output.push('}');
        output
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingAdvancedPaginationManifestError {
    ReceiptMismatch,
    MissingPage,
    ExtraPage,
    WrongPage,
    WrongMaster,
    WrongBox,
    WrongRepetition,
    WrongPaint,
    ArithmeticOverflow,
}

impl std::fmt::Display for StagingAdvancedPaginationManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::ReceiptMismatch => "I9190: advanced receipt mismatch",
            Self::MissingPage => "I9190: advanced artifact page is missing",
            Self::ExtraPage => "I9190: advanced artifact has an extra page",
            Self::WrongPage => "I9190: advanced artifact page mismatch",
            Self::WrongMaster => "I9190: advanced artifact master mismatch",
            Self::WrongBox => "I9190: advanced artifact page-box mismatch",
            Self::WrongRepetition => "I9190: advanced repetition mismatch",
            Self::WrongPaint => "I9190: advanced paint mismatch",
            Self::ArithmeticOverflow => "I9190: advanced manifest arithmetic overflow",
        })
    }
}

impl std::error::Error for StagingAdvancedPaginationManifestError {}

pub fn project_staging_header_footer_manifest(
    selected: &StagingHeaderFooterSelectedLayout,
    display: &StagingHeaderFooterDisplay,
    pdf: &StagingHeaderFooterPdf,
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
    let mut repetitions = std::collections::BTreeMap::<
        (
            typaxis_core::MasterId,
            typaxis_pagination::StagingAdvancedPageFrameKind,
        ),
        u32,
    >::new();
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
        let expected_paint = u32::try_from(selected_page.region_fragments().len())
            .map_err(|_| StagingAdvancedPaginationManifestError::ArithmeticOverflow)?;
        if display_page.command_count() != expected_paint
            || pdf_page.command_count() != expected_paint
        {
            return Err(StagingAdvancedPaginationManifestError::WrongPaint);
        }
        for frame in selected_page.frames() {
            if frame.kind() == typaxis_pagination::StagingAdvancedPageFrameKind::Body {
                if frame.repetition_index().is_some() {
                    return Err(StagingAdvancedPaginationManifestError::WrongRepetition);
                }
            } else {
                let kind = match frame.kind() {
                    kind @ (typaxis_pagination::StagingAdvancedPageFrameKind::Header
                    | typaxis_pagination::StagingAdvancedPageFrameKind::Footer) => kind,
                    typaxis_pagination::StagingAdvancedPageFrameKind::Body => {
                        unreachable!("body was handled above")
                    }
                };
                let next = repetitions
                    .entry((selected_page.master_id().clone(), kind))
                    .or_insert(0);
                if frame.repetition_index() != Some(*next) {
                    return Err(StagingAdvancedPaginationManifestError::WrongRepetition);
                }
                *next = next
                    .checked_add(1)
                    .ok_or(StagingAdvancedPaginationManifestError::ArithmeticOverflow)?;
            }
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
    Ok(StagingAdvancedPaginationManifest {
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
        profile_receipt_sha256,
        flow_registry_sha256,
        selected_layout_sha256,
        paint_closure_sha256,
    })
}

fn encode_projection(
    selected: &StagingHeaderFooterSelectedLayout,
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
    push_jcs_string(&mut output, STAGING_HEADER_FOOTER_PROFILE_ID);
    output.push_str(",\"profile_receipt_sha256\":");
    push_hex(&mut output, profile_receipt_sha256);
    output.push_str(",\"selected_layout_sha256\":");
    push_hex(&mut output, selected_layout_sha256);
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
    output.push_str(",\"kind\":");
    push_jcs_string(output, frame.kind().as_str());
    output.push_str(",\"rect\":");
    push_rect(output, frame.rect());
    output.push_str(",\"repetition_index\":");
    match frame.repetition_index() {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
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
