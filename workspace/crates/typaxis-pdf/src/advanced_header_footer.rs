use typaxis_core::{
    push_jcs_string, sha256, EffectiveConfig, LayoutStateFingerprint, MasterId,
    PdfStreamCompression, ValidatedResourceLimits,
};
use typaxis_display_list::{
    StagingAdvancedContentBinding, StagingHeaderFooterDisplay, StagingHeaderFooterDisplayPage,
    StagingPdfPageBox, StagingSelectedPageBoxes,
};
use typaxis_resources::AdmittedResourceLedger;

use crate::advanced_content::{
    prepare_advanced_pdf_content, AdvancedPdfContentObservation, AdvancedPdfContentPlan,
};

pub const STAGING_HEADER_FOOTER_PDF_CLOSURE_ALGORITHM: &str =
    "typaxis.advanced-pagination-pdf-closure/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingHeaderFooterPdfPageObservation {
    page_index: u32,
    master_id: MasterId,
    boxes: StagingSelectedPageBoxes,
    page_object_id: u32,
    content_object_id: u32,
    command_count: u32,
}

impl StagingHeaderFooterPdfPageObservation {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn boxes(&self) -> StagingSelectedPageBoxes {
        self.boxes
    }
    pub const fn page_object_id(&self) -> u32 {
        self.page_object_id
    }
    pub const fn content_object_id(&self) -> u32 {
        self.content_object_id
    }
    pub const fn command_count(&self) -> u32 {
        self.command_count
    }
}

#[derive(Debug)]
pub struct StagingHeaderFooterPdfClosureReceipt {
    display_paint_sha256: [u8; 32],
    pdf_sha256: [u8; 32],
    paint_closure_sha256: [u8; 32],
    object_count: u32,
    canonical_jcs: String,
}

impl StagingHeaderFooterPdfClosureReceipt {
    pub const fn display_paint_sha256(&self) -> [u8; 32] {
        self.display_paint_sha256
    }
    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub const fn paint_closure_sha256(&self) -> [u8; 32] {
        self.paint_closure_sha256
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Debug)]
pub struct StagingHeaderFooterPdf {
    bytes: Vec<u8>,
    pages: Vec<StagingHeaderFooterPdfPageObservation>,
    content: Option<AdvancedPdfContentObservation>,
    receipt: StagingHeaderFooterPdfClosureReceipt,
    stream_compression: PdfStreamCompression,
}

impl StagingHeaderFooterPdf {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn pages(&self) -> &[StagingHeaderFooterPdfPageObservation] {
        &self.pages
    }
    pub const fn receipt(&self) -> &StagingHeaderFooterPdfClosureReceipt {
        &self.receipt
    }

    pub fn into_verified_receipt(
        self,
        selected_layout_sha256: [u8; 32],
        config: &EffectiveConfig,
    ) -> crate::VerifiedPdfBytesReceipt {
        crate::VerifiedPdfBytesReceipt {
            sha256: self.receipt.pdf_sha256,
            selected_layout_fingerprint: LayoutStateFingerprint::from_untrusted_bytes(
                selected_layout_sha256,
            ),
            footnote_display_sha256: None,
            page_count: u32::try_from(self.pages.len()).expect("advanced page count is bounded"),
            object_count: self.receipt.object_count,
            stream_compression: self.stream_compression,
            config_fingerprint: config.fingerprint(),
            bytes: self.bytes,
        }
    }

    pub fn verify_receipt(
        &self,
        display: &StagingHeaderFooterDisplay,
    ) -> Result<(), StagingHeaderFooterPdfError> {
        display
            .verify_receipt()
            .map_err(|_| StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
        if self.pages.len() != display.pages().len() {
            return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
        }
        for (index, (pdf_page, display_page)) in self.pages.iter().zip(display.pages()).enumerate()
        {
            let expected_page_object = u32::try_from(index)
                .ok()
                .and_then(|value| value.checked_mul(2))
                .and_then(|value| value.checked_add(3))
                .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
            if pdf_page.page_index != display_page.page_index()
                || pdf_page.master_id != *display_page.master_id()
                || pdf_page.boxes != display_page.boxes()
                || pdf_page.command_count != display_page.command_count()
                || pdf_page.page_object_id != expected_page_object
                || pdf_page.content_object_id
                    != expected_page_object
                        .checked_add(1)
                        .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?
            {
                return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
            }
        }
        let base_objects = u32::try_from(self.pages.len())
            .ok()
            .and_then(|pages| pages.checked_mul(2))
            .and_then(|pages| pages.checked_add(2))
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let expected_objects = base_objects
            .checked_add(
                self.content
                    .as_ref()
                    .map_or(0, AdvancedPdfContentObservation::extra_object_count),
            )
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        match (&self.content, display.content()) {
            (None, None) => {}
            (Some(observation), Some(binding)) => observation.verify(binding, &self.bytes)?,
            _ => return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch),
        }
        let pdf_sha256 = sha256(&self.bytes);
        let canonical = encode_pdf_closure(
            display.receipt().paint_closure_sha256(),
            &self.pages,
            pdf_sha256,
            expected_objects,
        );
        if self.receipt.display_paint_sha256 != display.receipt().paint_closure_sha256()
            || self.receipt.pdf_sha256 != pdf_sha256
            || self.receipt.object_count != expected_objects
            || self.receipt.canonical_jcs != canonical
            || self.receipt.paint_closure_sha256 != sha256(canonical.as_bytes())
        {
            return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingHeaderFooterPdfError {
    DisplayClosureMismatch,
    PageObjectLimit,
    OutputLimit,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingHeaderFooterPdfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::DisplayClosureMismatch => "I9190: PDF/Display header-footer closure mismatch",
            Self::PageObjectLimit => "D8101: PDF object limit exceeded",
            Self::OutputLimit => "D8101: PDF output limit exceeded",
            Self::ArithmeticOverflow => "I9190: PDF serialization arithmetic overflow",
            Self::AllocationFailure => "D8101: PDF allocation failure",
        })
    }
}

impl std::error::Error for StagingHeaderFooterPdfError {}

pub fn serialize_staging_header_footer_pdf(
    display: &StagingHeaderFooterDisplay,
    limits: &ValidatedResourceLimits,
) -> Result<StagingHeaderFooterPdf, StagingHeaderFooterPdfError> {
    serialize_header_footer_pdf_with_compression(display, limits, PdfStreamCompression::None, None)
}

pub fn serialize_header_footer_pdf(
    display: &StagingHeaderFooterDisplay,
    config: &EffectiveConfig,
    admitted: &AdmittedResourceLedger,
) -> Result<StagingHeaderFooterPdf, StagingHeaderFooterPdfError> {
    serialize_header_footer_pdf_with_compression(
        display,
        config.limits(),
        config.stream_compression(),
        Some(admitted),
    )
}

fn serialize_header_footer_pdf_with_compression(
    display: &StagingHeaderFooterDisplay,
    limits: &ValidatedResourceLimits,
    stream_compression: PdfStreamCompression,
    admitted: Option<&AdmittedResourceLedger>,
) -> Result<StagingHeaderFooterPdf, StagingHeaderFooterPdfError> {
    display
        .verify_receipt()
        .map_err(|_| StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
    validate_advanced_content_resources(display.content(), admitted, display.pages().len())?;
    if display.pages().is_empty() {
        return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
    }
    let page_count = u32::try_from(display.pages().len())
        .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    let base_object_count = page_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(2))
        .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    let page_boxes = display
        .pages()
        .iter()
        .map(StagingHeaderFooterDisplayPage::boxes)
        .collect::<Vec<_>>();
    let content_plan = prepare_content_plan(
        display.content(),
        admitted,
        &page_boxes,
        base_object_count,
        limits,
        stream_compression,
    )?;
    let object_count = base_object_count
        .checked_add(
            content_plan
                .as_ref()
                .map_or(0, |plan| plan.observation().extra_object_count()),
        )
        .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    if object_count > limits.get().max_pdf_objects {
        return Err(StagingHeaderFooterPdfError::PageObjectLimit);
    }

    let object_capacity = usize::try_from(object_count)
        .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    let mut objects = Vec::<Vec<u8>>::new();
    objects
        .try_reserve_exact(object_capacity)
        .map_err(|_| StagingHeaderFooterPdfError::AllocationFailure)?;
    let mut staged_payload_bytes = 0u64;
    let catalog = format!(
        "<< /Type /Catalog /Pages 2 0 R{} >>",
        content_plan
            .as_ref()
            .map_or("", AdvancedPdfContentPlan::catalog_suffix)
    );
    push_object(
        &mut objects,
        catalog.into_bytes(),
        &mut staged_payload_bytes,
        limits.get().max_output_bytes,
    )?;
    let mut pages_dictionary = Vec::new();
    let pages_header = format!("<< /Type /Pages /Count {page_count} /Kids [");
    append_limited(
        &mut pages_dictionary,
        pages_header.as_bytes(),
        limits.get().max_output_bytes,
    )?;
    for page_index in 0..page_count {
        let page_object = page_index
            .checked_mul(2)
            .and_then(|value| value.checked_add(3))
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let page_reference = format!(" {page_object} 0 R");
        append_limited(
            &mut pages_dictionary,
            page_reference.as_bytes(),
            limits.get().max_output_bytes,
        )?;
    }
    append_limited(
        &mut pages_dictionary,
        b" ] >>",
        limits.get().max_output_bytes,
    )?;
    push_object(
        &mut objects,
        pages_dictionary,
        &mut staged_payload_bytes,
        limits.get().max_output_bytes,
    )?;

    let mut observations = Vec::new();
    observations
        .try_reserve_exact(display.pages().len())
        .map_err(|_| StagingHeaderFooterPdfError::AllocationFailure)?;
    for page in display.pages() {
        let page_object_id = page
            .page_index()
            .checked_mul(2)
            .and_then(|value| value.checked_add(3))
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let content_object_id = page_object_id
            .checked_add(1)
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let mut content = Vec::new();
        let first = usize::try_from(page.first_command())
            .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let end = first
            .checked_add(
                usize::try_from(page.command_count())
                    .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?,
            )
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let commands = display
            .commands()
            .get(first..end)
            .ok_or(StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
        let media_height = page.boxes().media_box().values()[3];
        for (command_index, command) in commands.iter().enumerate() {
            if command.page_index() != page.page_index() || command.master_id() != page.master_id()
            {
                return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
            }
            let expected_ordinal = first
                .checked_add(command_index)
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
            if command.command_ordinal() != expected_ordinal {
                return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
            }
            let bounds = command.bounds();
            let pdf_y = media_height
                .checked_sub(bounds.y().raw())
                .and_then(|value| value.checked_sub(bounds.height().get().raw()))
                .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
            let record = format!(
                "% typaxis {} flow={} repetition={} node={}\nq 0 g {} {} {} {} re f Q\n",
                command.kind().as_str(),
                command.source_flow_id().get(),
                command.repetition_index(),
                command.block_node_id().get(),
                pdf_fixed(bounds.x().raw()),
                pdf_fixed(pdf_y),
                pdf_fixed(bounds.width().get().raw()),
                pdf_fixed(bounds.height().get().raw()),
            );
            append_limited(
                &mut content,
                record.as_bytes(),
                limits.get().max_output_bytes,
            )?;
        }
        if let Some(plan) = content_plan
            .as_ref()
            .and_then(|plan| plan.page(page.page_index()))
        {
            append_limited(
                &mut content,
                plan.stream_suffix(),
                limits.get().max_output_bytes,
            )?;
        }
        let content_bytes = generated_stream_object(
            "",
            &content,
            stream_compression,
            limits.get().max_output_bytes,
        )?;
        let boxes = page.boxes();
        let (resources, annotations) = match content_plan
            .as_ref()
            .and_then(|plan| plan.page(page.page_index()))
        {
            Some(plan) => (plan.resources(), plan.annotation_suffix()),
            None => ("/Resources << >>", ""),
        };
        let page_object = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox {} /CropBox {} /TrimBox {} {resources} /Contents {content_object_id} 0 R{annotations} >>",
            pdf_box(boxes.media_box()),
            pdf_box(boxes.crop_box()),
            pdf_box(boxes.trim_box()),
        );
        push_object(
            &mut objects,
            page_object.into_bytes(),
            &mut staged_payload_bytes,
            limits.get().max_output_bytes,
        )?;
        push_object(
            &mut objects,
            content_bytes,
            &mut staged_payload_bytes,
            limits.get().max_output_bytes,
        )?;
        observations.push(StagingHeaderFooterPdfPageObservation {
            page_index: page.page_index(),
            master_id: page.master_id().clone(),
            boxes,
            page_object_id,
            content_object_id,
            command_count: page.command_count(),
        });
    }
    if let Some(plan) = &content_plan {
        for object in plan.objects() {
            push_object(
                &mut objects,
                object.clone(),
                &mut staged_payload_bytes,
                limits.get().max_output_bytes,
            )?;
        }
    }
    if objects.len() != object_capacity {
        return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
    }
    let bytes = write_classic_pdf(&objects, limits.get().max_output_bytes)?;
    let pdf_sha256 = sha256(&bytes);
    let canonical_jcs = encode_pdf_closure(
        display.receipt().paint_closure_sha256(),
        &observations,
        pdf_sha256,
        object_count,
    );
    let receipt = StagingHeaderFooterPdfClosureReceipt {
        display_paint_sha256: display.receipt().paint_closure_sha256(),
        pdf_sha256,
        paint_closure_sha256: sha256(canonical_jcs.as_bytes()),
        object_count,
        canonical_jcs,
    };
    let pdf = StagingHeaderFooterPdf {
        bytes,
        pages: observations,
        content: content_plan.as_ref().map(|plan| plan.observation().clone()),
        receipt,
        stream_compression,
    };
    pdf.verify_receipt(display)?;
    Ok(pdf)
}

fn prepare_content_plan(
    content: Option<&StagingAdvancedContentBinding>,
    admitted: Option<&AdmittedResourceLedger>,
    page_boxes: &[StagingSelectedPageBoxes],
    base_object_count: u32,
    limits: &ValidatedResourceLimits,
    stream_compression: PdfStreamCompression,
) -> Result<Option<AdvancedPdfContentPlan>, StagingHeaderFooterPdfError> {
    match (content, admitted) {
        (None, None) => Ok(None),
        (Some(content), Some(admitted)) => prepare_advanced_pdf_content(
            content,
            admitted,
            page_boxes,
            base_object_count,
            limits,
            stream_compression,
        )
        .map(Some),
        _ => Err(StagingHeaderFooterPdfError::DisplayClosureMismatch),
    }
}

pub(crate) fn validate_advanced_content_resources(
    content: Option<&StagingAdvancedContentBinding>,
    admitted: Option<&AdmittedResourceLedger>,
    page_count: usize,
) -> Result<(), StagingHeaderFooterPdfError> {
    match (content, admitted) {
        (None, None) => Ok(()),
        (Some(content), Some(admitted)) => {
            content
                .verify(page_count)
                .map_err(|_| StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
            if content.resource_ledger_sha256() != admitted.fingerprint().bytes()
                || content
                    .pages()
                    .iter()
                    .flat_map(|page| page.images())
                    .any(|usage| admitted.image(usage.image_id()).is_none())
            {
                return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
            }
            Ok(())
        }
        _ => Err(StagingHeaderFooterPdfError::DisplayClosureMismatch),
    }
}

pub(crate) fn append_actual_text(
    output: &mut Vec<u8>,
    text: &str,
    max_output_bytes: u64,
) -> Result<(), StagingHeaderFooterPdfError> {
    if text.is_empty() {
        return Ok(());
    }
    append_limited(output, b"/Span << /ActualText <FEFF", max_output_bytes)?;
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for unit in text.encode_utf16() {
        let bytes = unit.to_be_bytes();
        let encoded = [
            HEX[usize::from(bytes[0] >> 4)],
            HEX[usize::from(bytes[0] & 0x0f)],
            HEX[usize::from(bytes[1] >> 4)],
            HEX[usize::from(bytes[1] & 0x0f)],
        ];
        append_limited(output, &encoded, max_output_bytes)?;
    }
    append_limited(
        output,
        b"> >> BDC\nBT /F0 8 Tf 0 Tr 1 8 Td <",
        max_output_bytes,
    )?;
    for character in text.chars() {
        let byte = if character.is_ascii() && !character.is_ascii_control() {
            u8::try_from(u32::from(character))
                .map_err(|_| StagingHeaderFooterPdfError::DisplayClosureMismatch)?
        } else {
            b'?'
        };
        let encoded = [HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 0x0f)]];
        append_limited(output, &encoded, max_output_bytes)?;
    }
    append_limited(output, b"> Tj ET\nEMC\n", max_output_bytes)
}

pub(super) fn generated_stream_object(
    dictionary_entries: &str,
    raw_data: &[u8],
    compression: PdfStreamCompression,
    max_output_bytes: u64,
) -> Result<Vec<u8>, StagingHeaderFooterPdfError> {
    let encoded;
    let (data, filter) = match compression {
        PdfStreamCompression::None => (raw_data, ""),
        PdfStreamCompression::Flate => {
            encoded = crate::zlib_stored(raw_data, max_output_bytes)
                .map_err(|_| StagingHeaderFooterPdfError::OutputLimit)?;
            (encoded.as_slice(), " /Filter /FlateDecode")
        }
    };
    let mut output = Vec::new();
    let header = format!(
        "<<{dictionary_entries}{filter} /Length {} >>\nstream\n",
        data.len()
    );
    append_limited(&mut output, header.as_bytes(), max_output_bytes)?;
    append_limited(&mut output, data, max_output_bytes)?;
    if !data.ends_with(b"\n") {
        append_limited(&mut output, b"\n", max_output_bytes)?;
    }
    append_limited(&mut output, b"endstream", max_output_bytes)?;
    Ok(output)
}

pub(super) fn push_object(
    objects: &mut Vec<Vec<u8>>,
    object: Vec<u8>,
    staged_payload_bytes: &mut u64,
    max_output_bytes: u64,
) -> Result<(), StagingHeaderFooterPdfError> {
    let object_bytes =
        u64::try_from(object.len()).map_err(|_| StagingHeaderFooterPdfError::OutputLimit)?;
    let next = staged_payload_bytes
        .checked_add(object_bytes)
        .ok_or(StagingHeaderFooterPdfError::OutputLimit)?;
    if next > max_output_bytes {
        return Err(StagingHeaderFooterPdfError::OutputLimit);
    }
    *staged_payload_bytes = next;
    objects.push(object);
    Ok(())
}

pub(super) fn append_limited(
    output: &mut Vec<u8>,
    bytes: &[u8],
    maximum: u64,
) -> Result<(), StagingHeaderFooterPdfError> {
    let next = output
        .len()
        .checked_add(bytes.len())
        .ok_or(StagingHeaderFooterPdfError::OutputLimit)?;
    if u64::try_from(next).map_err(|_| StagingHeaderFooterPdfError::OutputLimit)? > maximum {
        return Err(StagingHeaderFooterPdfError::OutputLimit);
    }
    output
        .try_reserve(bytes.len())
        .map_err(|_| StagingHeaderFooterPdfError::AllocationFailure)?;
    output.extend_from_slice(bytes);
    Ok(())
}

pub(super) fn write_classic_pdf(
    objects: &[Vec<u8>],
    max_output_bytes: u64,
) -> Result<Vec<u8>, StagingHeaderFooterPdfError> {
    let mut output = Vec::new();
    let initial_capacity = usize::try_from(max_output_bytes.min(1024))
        .map_err(|_| StagingHeaderFooterPdfError::OutputLimit)?;
    output
        .try_reserve(initial_capacity)
        .map_err(|_| StagingHeaderFooterPdfError::AllocationFailure)?;
    append_limited(
        &mut output,
        b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n",
        max_output_bytes,
    )?;
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(objects.len())
        .map_err(|_| StagingHeaderFooterPdfError::AllocationFailure)?;
    for (index, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        let object_id = index
            .checked_add(1)
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let header = format!("{object_id} 0 obj\n");
        append_limited(&mut output, header.as_bytes(), max_output_bytes)?;
        append_limited(&mut output, object, max_output_bytes)?;
        append_limited(&mut output, b"\nendobj\n", max_output_bytes)?;
    }
    let xref = output.len();
    let size = objects
        .len()
        .checked_add(1)
        .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    let xref_header = format!("xref\n0 {size}\n0000000000 65535 f \n");
    append_limited(&mut output, xref_header.as_bytes(), max_output_bytes)?;
    for offset in offsets {
        if offset > 9_999_999_999usize {
            return Err(StagingHeaderFooterPdfError::OutputLimit);
        }
        let record = format!("{offset:010} 00000 n \n");
        append_limited(&mut output, record.as_bytes(), max_output_bytes)?;
    }
    let trailer = format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n");
    append_limited(&mut output, trailer.as_bytes(), max_output_bytes)?;
    Ok(output)
}

pub(super) fn pdf_box(value: StagingPdfPageBox) -> String {
    let values = value.values();
    format!(
        "[{} {} {} {}]",
        pdf_fixed(values[0]),
        pdf_fixed(values[1]),
        pdf_fixed(values[2]),
        pdf_fixed(values[3])
    )
}

pub(super) fn pdf_fixed(raw: i64) -> String {
    const SCALE: i64 = 65_536;
    const DECIMAL_SCALE: u64 = 10_000_000_000_000_000;
    const BINARY_TO_DECIMAL: u64 = 152_587_890_625; // 5^16
    let negative = raw < 0;
    let magnitude = raw.unsigned_abs();
    let whole = magnitude / SCALE as u64;
    let remainder = magnitude % SCALE as u64;
    if remainder == 0 {
        return if negative {
            format!("-{whole}")
        } else {
            whole.to_string()
        };
    }
    let fraction = remainder * BINARY_TO_DECIMAL;
    debug_assert!(fraction < DECIMAL_SCALE);
    let mut fraction = format!("{fraction:016}");
    while fraction.ends_with('0') {
        fraction.pop();
    }
    if negative {
        format!("-{whole}.{fraction}")
    } else {
        format!("{whole}.{fraction}")
    }
}

fn encode_pdf_closure(
    display_paint_sha256: [u8; 32],
    pages: &[StagingHeaderFooterPdfPageObservation],
    pdf_sha256: [u8; 32],
    object_count: u32,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_HEADER_FOOTER_PDF_CLOSURE_ALGORITHM);
    output.push_str(",\"display_paint_sha256\":");
    push_hex(&mut output, display_paint_sha256);
    output.push_str(",\"object_count\":");
    output.push_str(&object_count.to_string());
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
        output.push_str(",\"content_object_id\":");
        output.push_str(&page.content_object_id.to_string());
        output.push_str(",\"master_id\":");
        push_jcs_string(&mut output, page.master_id.as_str());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"page_object_id\":");
        output.push_str(&page.page_object_id.to_string());
        output.push('}');
    }
    output.push_str("],\"pdf_sha256\":");
    push_hex(&mut output, pdf_sha256);
    output.push('}');
    output
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
    use super::pdf_fixed;

    #[test]
    fn fixed_point_pdf_numbers_are_exact_and_minimal() {
        assert_eq!(pdf_fixed(0), "0");
        assert_eq!(pdf_fixed(65_536), "1");
        assert_eq!(pdf_fixed(32_768), "0.5");
        assert_eq!(pdf_fixed(1), "0.0000152587890625");
        assert_eq!(pdf_fixed(-98_304), "-1.5");
    }
}
