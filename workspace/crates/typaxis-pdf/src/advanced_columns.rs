use typaxis_core::{
    push_jcs_string, sha256, EffectiveConfig, LayoutStateFingerprint, MasterId,
    PdfStreamCompression, ValidatedResourceLimits,
};
use typaxis_display_list::{StagingColumnsDisplay, StagingPdfPageBox, StagingSelectedPageBoxes};
use typaxis_resources::AdmittedResourceLedger;

use crate::advanced_content::{
    prepare_advanced_pdf_content, AdvancedPdfContentObservation, AdvancedPdfContentPlan,
};
use crate::advanced_header_footer::{
    append_limited, generated_stream_object, pdf_box, pdf_fixed, push_object,
    validate_advanced_content_resources, write_classic_pdf,
    STAGING_HEADER_FOOTER_PDF_CLOSURE_ALGORITHM,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingColumnsPdfPageObservation {
    page_index: u32,
    master_id: MasterId,
    boxes: StagingSelectedPageBoxes,
    page_object_id: u32,
    content_object_id: u32,
    command_count: u32,
}

impl StagingColumnsPdfPageObservation {
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
pub struct StagingColumnsPdfClosureReceipt {
    display_paint_sha256: [u8; 32],
    pdf_sha256: [u8; 32],
    paint_closure_sha256: [u8; 32],
    object_count: u32,
    canonical_jcs: String,
}

impl StagingColumnsPdfClosureReceipt {
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
pub struct StagingColumnsPdf {
    bytes: Vec<u8>,
    pages: Vec<StagingColumnsPdfPageObservation>,
    content: Option<AdvancedPdfContentObservation>,
    receipt: StagingColumnsPdfClosureReceipt,
    stream_compression: PdfStreamCompression,
}

impl StagingColumnsPdf {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn pages(&self) -> &[StagingColumnsPdfPageObservation] {
        &self.pages
    }
    pub const fn receipt(&self) -> &StagingColumnsPdfClosureReceipt {
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
        display: &StagingColumnsDisplay,
    ) -> Result<(), StagingColumnsPdfError> {
        display
            .verify_receipt()
            .map_err(|_| StagingColumnsPdfError::DisplayClosureMismatch)?;
        if self.pages.len() != display.pages().len() {
            return Err(StagingColumnsPdfError::DisplayClosureMismatch);
        }
        for (index, (pdf_page, display_page)) in self.pages.iter().zip(display.pages()).enumerate()
        {
            let expected_page_object = u32::try_from(index)
                .ok()
                .and_then(|value| value.checked_mul(2))
                .and_then(|value| value.checked_add(3))
                .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
            if pdf_page.page_index != display_page.page_index()
                || pdf_page.master_id != *display_page.master_id()
                || pdf_page.boxes != display_page.boxes()
                || pdf_page.command_count != display_page.command_count()
                || pdf_page.page_object_id != expected_page_object
                || pdf_page.content_object_id
                    != expected_page_object
                        .checked_add(1)
                        .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?
            {
                return Err(StagingColumnsPdfError::DisplayClosureMismatch);
            }
        }
        let base_objects = u32::try_from(self.pages.len())
            .ok()
            .and_then(|pages| pages.checked_mul(2))
            .and_then(|pages| pages.checked_add(2))
            .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
        let expected_objects = base_objects
            .checked_add(
                self.content
                    .as_ref()
                    .map_or(0, AdvancedPdfContentObservation::extra_object_count),
            )
            .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
        match (&self.content, display.content()) {
            (None, None) => {}
            (Some(observation), Some(binding)) => observation
                .verify(binding, &self.bytes)
                .map_err(map_common_error)?,
            _ => return Err(StagingColumnsPdfError::DisplayClosureMismatch),
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
            return Err(StagingColumnsPdfError::DisplayClosureMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingColumnsPdfError {
    DisplayClosureMismatch,
    PageObjectLimit,
    OutputLimit,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingColumnsPdfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::DisplayClosureMismatch => "I9190: columns PDF/Display closure mismatch",
            Self::PageObjectLimit => "D8101: columns PDF object limit exceeded",
            Self::OutputLimit => "D8101: columns PDF output limit exceeded",
            Self::ArithmeticOverflow => "I9190: columns PDF arithmetic overflow",
            Self::AllocationFailure => "D8101: columns PDF allocation failure",
        })
    }
}

impl std::error::Error for StagingColumnsPdfError {}

pub fn serialize_staging_columns_pdf(
    display: &StagingColumnsDisplay,
    limits: &ValidatedResourceLimits,
) -> Result<StagingColumnsPdf, StagingColumnsPdfError> {
    serialize_columns_pdf_with_compression(display, limits, PdfStreamCompression::None, None)
}

pub fn serialize_columns_pdf(
    display: &StagingColumnsDisplay,
    config: &EffectiveConfig,
    admitted: &AdmittedResourceLedger,
) -> Result<StagingColumnsPdf, StagingColumnsPdfError> {
    serialize_columns_pdf_with_compression(
        display,
        config.limits(),
        config.stream_compression(),
        Some(admitted),
    )
}

fn serialize_columns_pdf_with_compression(
    display: &StagingColumnsDisplay,
    limits: &ValidatedResourceLimits,
    stream_compression: PdfStreamCompression,
    admitted: Option<&AdmittedResourceLedger>,
) -> Result<StagingColumnsPdf, StagingColumnsPdfError> {
    display
        .verify_receipt()
        .map_err(|_| StagingColumnsPdfError::DisplayClosureMismatch)?;
    validate_advanced_content_resources(display.content(), admitted, display.pages().len())
        .map_err(map_common_error)?;
    if display.pages().is_empty() {
        return Err(StagingColumnsPdfError::DisplayClosureMismatch);
    }
    let page_count = u32::try_from(display.pages().len())
        .map_err(|_| StagingColumnsPdfError::ArithmeticOverflow)?;
    let base_object_count = page_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(2))
        .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
    let page_boxes = display
        .pages()
        .iter()
        .map(|page| page.boxes())
        .collect::<Vec<_>>();
    let content_plan = match (display.content(), admitted) {
        (None, None) => None,
        (Some(content), Some(admitted)) => Some(
            prepare_advanced_pdf_content(
                content,
                admitted,
                &page_boxes,
                base_object_count,
                limits,
                stream_compression,
            )
            .map_err(map_common_error)?,
        ),
        _ => return Err(StagingColumnsPdfError::DisplayClosureMismatch),
    };
    let object_count = base_object_count
        .checked_add(
            content_plan
                .as_ref()
                .map_or(0, |plan| plan.observation().extra_object_count()),
        )
        .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
    if object_count > limits.get().max_pdf_objects {
        return Err(StagingColumnsPdfError::PageObjectLimit);
    }

    let object_capacity =
        usize::try_from(object_count).map_err(|_| StagingColumnsPdfError::ArithmeticOverflow)?;
    let mut objects = Vec::<Vec<u8>>::new();
    objects
        .try_reserve_exact(object_capacity)
        .map_err(|_| StagingColumnsPdfError::AllocationFailure)?;
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
    )
    .map_err(map_common_error)?;
    let mut pages_dictionary = Vec::new();
    let pages_header = format!("<< /Type /Pages /Count {page_count} /Kids [");
    append_limited(
        &mut pages_dictionary,
        pages_header.as_bytes(),
        limits.get().max_output_bytes,
    )
    .map_err(map_common_error)?;
    for page_index in 0..page_count {
        let page_object = page_index
            .checked_mul(2)
            .and_then(|value| value.checked_add(3))
            .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
        let page_reference = format!(" {page_object} 0 R");
        append_limited(
            &mut pages_dictionary,
            page_reference.as_bytes(),
            limits.get().max_output_bytes,
        )
        .map_err(map_common_error)?;
    }
    append_limited(
        &mut pages_dictionary,
        b" ] >>",
        limits.get().max_output_bytes,
    )
    .map_err(map_common_error)?;
    push_object(
        &mut objects,
        pages_dictionary,
        &mut staged_payload_bytes,
        limits.get().max_output_bytes,
    )
    .map_err(map_common_error)?;

    let mut observations = Vec::new();
    observations
        .try_reserve_exact(display.pages().len())
        .map_err(|_| StagingColumnsPdfError::AllocationFailure)?;
    for page in display.pages() {
        let page_object_id = page
            .page_index()
            .checked_mul(2)
            .and_then(|value| value.checked_add(3))
            .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
        let content_object_id = page_object_id
            .checked_add(1)
            .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
        let first = usize::try_from(page.first_command())
            .map_err(|_| StagingColumnsPdfError::ArithmeticOverflow)?;
        let end = first
            .checked_add(
                usize::try_from(page.command_count())
                    .map_err(|_| StagingColumnsPdfError::ArithmeticOverflow)?,
            )
            .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
        let commands = display
            .commands()
            .get(first..end)
            .ok_or(StagingColumnsPdfError::DisplayClosureMismatch)?;
        let mut content = Vec::new();
        let media_height = page.boxes().media_box().values()[3];
        for (command_index, command) in commands.iter().enumerate() {
            if command.page_index() != page.page_index() || command.master_id() != page.master_id()
            {
                return Err(StagingColumnsPdfError::DisplayClosureMismatch);
            }
            let expected_ordinal = first
                .checked_add(command_index)
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
            if command.command_ordinal() != expected_ordinal {
                return Err(StagingColumnsPdfError::DisplayClosureMismatch);
            }
            let bounds = command.bounds();
            let pdf_y = media_height
                .checked_sub(bounds.y().raw())
                .and_then(|value| value.checked_sub(bounds.height().get().raw()))
                .ok_or(StagingColumnsPdfError::ArithmeticOverflow)?;
            let record = format!(
                "% typaxis column={} flow={} node={}\nq 0 g {} {} {} {} re f Q\n",
                command.column_index(),
                command.frame_flow_id().get(),
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
            )
            .map_err(map_common_error)?;
        }
        if let Some(plan) = content_plan
            .as_ref()
            .and_then(|plan| plan.page(page.page_index()))
        {
            append_limited(
                &mut content,
                plan.stream_suffix(),
                limits.get().max_output_bytes,
            )
            .map_err(map_common_error)?;
        }
        let content_bytes = generated_stream_object(
            "",
            &content,
            stream_compression,
            limits.get().max_output_bytes,
        )
        .map_err(map_common_error)?;
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
        )
        .map_err(map_common_error)?;
        push_object(
            &mut objects,
            content_bytes,
            &mut staged_payload_bytes,
            limits.get().max_output_bytes,
        )
        .map_err(map_common_error)?;
        observations.push(StagingColumnsPdfPageObservation {
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
            )
            .map_err(map_common_error)?;
        }
    }
    if objects.len() != object_capacity {
        return Err(StagingColumnsPdfError::DisplayClosureMismatch);
    }
    let bytes =
        write_classic_pdf(&objects, limits.get().max_output_bytes).map_err(map_common_error)?;
    let pdf_sha256 = sha256(&bytes);
    let canonical_jcs = encode_pdf_closure(
        display.receipt().paint_closure_sha256(),
        &observations,
        pdf_sha256,
        object_count,
    );
    let receipt = StagingColumnsPdfClosureReceipt {
        display_paint_sha256: display.receipt().paint_closure_sha256(),
        pdf_sha256,
        paint_closure_sha256: sha256(canonical_jcs.as_bytes()),
        object_count,
        canonical_jcs,
    };
    let pdf = StagingColumnsPdf {
        bytes,
        pages: observations,
        content: content_plan.as_ref().map(|plan| plan.observation().clone()),
        receipt,
        stream_compression,
    };
    pdf.verify_receipt(display)?;
    Ok(pdf)
}

fn map_common_error(error: crate::StagingHeaderFooterPdfError) -> StagingColumnsPdfError {
    match error {
        crate::StagingHeaderFooterPdfError::DisplayClosureMismatch => {
            StagingColumnsPdfError::DisplayClosureMismatch
        }
        crate::StagingHeaderFooterPdfError::PageObjectLimit => {
            StagingColumnsPdfError::PageObjectLimit
        }
        crate::StagingHeaderFooterPdfError::OutputLimit => StagingColumnsPdfError::OutputLimit,
        crate::StagingHeaderFooterPdfError::ArithmeticOverflow => {
            StagingColumnsPdfError::ArithmeticOverflow
        }
        crate::StagingHeaderFooterPdfError::AllocationFailure => {
            StagingColumnsPdfError::AllocationFailure
        }
    }
}

fn encode_pdf_closure(
    display_paint_sha256: [u8; 32],
    pages: &[StagingColumnsPdfPageObservation],
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
    use super::*;
    use typaxis_core::ResourceLimits;

    #[test]
    fn columns_pdf_closes_boxes_commands_objects_and_exact_bytes() {
        let display = typaxis_display_list::staging_columns_display_fixture();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let first = serialize_staging_columns_pdf(&display, &limits).unwrap();
        let second = serialize_staging_columns_pdf(&display, &limits).unwrap();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.pages().len(), 2);
        let text = String::from_utf8_lossy(first.bytes());
        assert_eq!(text.matches("/MediaBox").count(), 2);
        assert_eq!(text.matches("/CropBox").count(), 2);
        assert_eq!(text.matches("/TrimBox").count(), 2);
        assert!(text.contains("% typaxis column=0"));
        assert!(text.contains("% typaxis column=1"));
        first.verify_receipt(&display).unwrap();
    }

    #[test]
    fn columns_pdf_rejects_extra_wrong_page_object_and_exact_limit_tamper() {
        let display = typaxis_display_list::staging_columns_display_fixture();
        let exact = ResourceLimits {
            max_pdf_objects: 6,
            ..ResourceLimits::default()
        };
        let exact = ValidatedResourceLimits::new(exact).unwrap();
        let mut pdf = serialize_staging_columns_pdf(&display, &exact).unwrap();
        pdf.pages[0].page_object_id = 4;
        assert!(matches!(
            pdf.verify_receipt(&display),
            Err(StagingColumnsPdfError::DisplayClosureMismatch)
        ));

        let mut wrong_page = serialize_staging_columns_pdf(&display, &exact).unwrap();
        wrong_page.pages[0].page_index = 1;
        assert!(matches!(
            wrong_page.verify_receipt(&display),
            Err(StagingColumnsPdfError::DisplayClosureMismatch)
        ));

        let mut extra = serialize_staging_columns_pdf(&display, &exact).unwrap();
        extra.pages.push(extra.pages[0].clone());
        assert!(matches!(
            extra.verify_receipt(&display),
            Err(StagingColumnsPdfError::DisplayClosureMismatch)
        ));

        let over = ResourceLimits {
            max_pdf_objects: 5,
            ..ResourceLimits::default()
        };
        let over = ValidatedResourceLimits::new(over).unwrap();
        assert!(matches!(
            serialize_staging_columns_pdf(&display, &over),
            Err(StagingColumnsPdfError::PageObjectLimit)
        ));
    }
}
