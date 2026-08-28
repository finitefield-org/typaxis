use typaxis_core::{
    push_jcs_string, sha256, EffectiveConfig, LayoutStateFingerprint, MasterId, NodeId,
    PdfStreamCompression, ValidatedResourceLimits,
};
use typaxis_display_list::{
    StagingFloatDisplay, StagingFloatPaintCommandKind, StagingPdfPageBox, StagingSelectedPageBoxes,
};
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
pub struct StagingFloatPdfObjectUsage {
    command_ordinal: u32,
    float_flow_id: u32,
    figure_node_id: NodeId,
    object_id: u32,
}

impl StagingFloatPdfObjectUsage {
    pub const fn command_ordinal(&self) -> u32 {
        self.command_ordinal
    }
    pub const fn float_flow_id(&self) -> u32 {
        self.float_flow_id
    }
    pub const fn figure_node_id(&self) -> NodeId {
        self.figure_node_id
    }
    pub const fn object_id(&self) -> u32 {
        self.object_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatPdfPageObservation {
    page_index: u32,
    master_id: MasterId,
    boxes: StagingSelectedPageBoxes,
    page_object_id: u32,
    content_object_id: u32,
    command_count: u32,
    float_object_usages: Vec<StagingFloatPdfObjectUsage>,
}

impl StagingFloatPdfPageObservation {
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
    pub fn float_object_usages(&self) -> &[StagingFloatPdfObjectUsage] {
        &self.float_object_usages
    }
}

#[derive(Debug)]
pub struct StagingFloatPdfClosureReceipt {
    display_paint_sha256: [u8; 32],
    pdf_sha256: [u8; 32],
    paint_closure_sha256: [u8; 32],
    object_count: u32,
    canonical_jcs: String,
}

impl StagingFloatPdfClosureReceipt {
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
pub struct StagingFloatPdf {
    bytes: Vec<u8>,
    pages: Vec<StagingFloatPdfPageObservation>,
    content: Option<AdvancedPdfContentObservation>,
    receipt: StagingFloatPdfClosureReceipt,
    stream_compression: PdfStreamCompression,
}

impl StagingFloatPdf {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn pages(&self) -> &[StagingFloatPdfPageObservation] {
        &self.pages
    }
    pub const fn receipt(&self) -> &StagingFloatPdfClosureReceipt {
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
        display: &StagingFloatDisplay,
    ) -> Result<(), StagingFloatPdfError> {
        display
            .verify_receipt()
            .map_err(|_| StagingFloatPdfError::DisplayClosureMismatch)?;
        if self.pages.len() != display.pages().len() {
            return Err(StagingFloatPdfError::DisplayClosureMismatch);
        }
        let page_count = u32::try_from(self.pages.len())
            .map_err(|_| StagingFloatPdfError::ArithmeticOverflow)?;
        let first_float_object = page_count
            .checked_mul(2)
            .and_then(|value| value.checked_add(3))
            .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
        let mut next_float_object = first_float_object;
        for (page_ordinal, (pdf_page, display_page)) in
            self.pages.iter().zip(display.pages()).enumerate()
        {
            let expected_page_object = u32::try_from(page_ordinal)
                .ok()
                .and_then(|value| value.checked_mul(2))
                .and_then(|value| value.checked_add(3))
                .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
            if pdf_page.page_index != display_page.page_index()
                || pdf_page.master_id != *display_page.master_id()
                || pdf_page.boxes != display_page.boxes()
                || pdf_page.command_count != display_page.command_count()
                || pdf_page.page_object_id != expected_page_object
                || pdf_page.content_object_id
                    != expected_page_object
                        .checked_add(1)
                        .ok_or(StagingFloatPdfError::ArithmeticOverflow)?
                || usize::try_from(display_page.float_command_count()).ok()
                    != Some(pdf_page.float_object_usages.len())
            {
                return Err(StagingFloatPdfError::DisplayClosureMismatch);
            }
            let first = usize::try_from(display_page.first_command())
                .map_err(|_| StagingFloatPdfError::ArithmeticOverflow)?;
            let end = first
                .checked_add(
                    usize::try_from(display_page.command_count())
                        .map_err(|_| StagingFloatPdfError::ArithmeticOverflow)?,
                )
                .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
            let expected_float_commands = display
                .commands()
                .get(first..end)
                .ok_or(StagingFloatPdfError::DisplayClosureMismatch)?
                .iter()
                .filter(|command| command.kind() == StagingFloatPaintCommandKind::Float);
            for (usage, command) in pdf_page
                .float_object_usages
                .iter()
                .zip(expected_float_commands)
            {
                if usage.command_ordinal != command.command_ordinal()
                    || Some(usage.float_flow_id) != command.float_flow_id().map(|flow| flow.get())
                    || usage.figure_node_id != command.node_id()
                    || usage.object_id != next_float_object
                {
                    return Err(StagingFloatPdfError::DisplayClosureMismatch);
                }
                next_float_object = next_float_object
                    .checked_add(1)
                    .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
            }
        }
        let base_objects = next_float_object
            .checked_sub(1)
            .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
        let expected_objects = base_objects
            .checked_add(
                self.content
                    .as_ref()
                    .map_or(0, AdvancedPdfContentObservation::extra_object_count),
            )
            .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
        match (&self.content, display.content()) {
            (None, None) => {}
            (Some(observation), Some(binding)) => observation
                .verify(binding, &self.bytes)
                .map_err(map_common_error)?,
            _ => return Err(StagingFloatPdfError::DisplayClosureMismatch),
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
            return Err(StagingFloatPdfError::DisplayClosureMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingFloatPdfError {
    DisplayClosureMismatch,
    PageObjectLimit,
    OutputLimit,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingFloatPdfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::DisplayClosureMismatch => "I9190: float PDF/Display closure mismatch",
            Self::PageObjectLimit => "D8101: float PDF object limit exceeded",
            Self::OutputLimit => "D8101: float PDF output limit exceeded",
            Self::ArithmeticOverflow => "I9190: float PDF arithmetic overflow",
            Self::AllocationFailure => "D8101: float PDF allocation failure",
        })
    }
}

impl std::error::Error for StagingFloatPdfError {}

pub fn serialize_staging_float_pdf(
    display: &StagingFloatDisplay,
    limits: &ValidatedResourceLimits,
) -> Result<StagingFloatPdf, StagingFloatPdfError> {
    serialize_float_pdf_with_compression(display, limits, PdfStreamCompression::None, None)
}

pub fn serialize_float_pdf(
    display: &StagingFloatDisplay,
    config: &EffectiveConfig,
    admitted: &AdmittedResourceLedger,
) -> Result<StagingFloatPdf, StagingFloatPdfError> {
    serialize_float_pdf_with_compression(
        display,
        config.limits(),
        config.stream_compression(),
        Some(admitted),
    )
}

fn serialize_float_pdf_with_compression(
    display: &StagingFloatDisplay,
    limits: &ValidatedResourceLimits,
    stream_compression: PdfStreamCompression,
    admitted: Option<&AdmittedResourceLedger>,
) -> Result<StagingFloatPdf, StagingFloatPdfError> {
    display
        .verify_receipt()
        .map_err(|_| StagingFloatPdfError::DisplayClosureMismatch)?;
    validate_advanced_content_resources(display.content(), admitted, display.pages().len())
        .map_err(map_common_error)?;
    if display.pages().is_empty() {
        return Err(StagingFloatPdfError::DisplayClosureMismatch);
    }
    let page_count = u32::try_from(display.pages().len())
        .map_err(|_| StagingFloatPdfError::ArithmeticOverflow)?;
    let float_count = u32::try_from(
        display
            .commands()
            .iter()
            .filter(|command| command.kind() == StagingFloatPaintCommandKind::Float)
            .count(),
    )
    .map_err(|_| StagingFloatPdfError::ArithmeticOverflow)?;
    let base_object_count = page_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(2))
        .and_then(|value| value.checked_add(float_count))
        .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
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
        _ => return Err(StagingFloatPdfError::DisplayClosureMismatch),
    };
    let object_count = base_object_count
        .checked_add(
            content_plan
                .as_ref()
                .map_or(0, |plan| plan.observation().extra_object_count()),
        )
        .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
    if object_count > limits.get().max_pdf_objects {
        return Err(StagingFloatPdfError::PageObjectLimit);
    }
    let object_capacity =
        usize::try_from(object_count).map_err(|_| StagingFloatPdfError::ArithmeticOverflow)?;
    let mut objects = Vec::<Vec<u8>>::new();
    objects
        .try_reserve_exact(object_capacity)
        .map_err(|_| StagingFloatPdfError::AllocationFailure)?;
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
    append_limited(
        &mut pages_dictionary,
        format!("<< /Type /Pages /Count {page_count} /Kids [").as_bytes(),
        limits.get().max_output_bytes,
    )
    .map_err(map_common_error)?;
    for page_index in 0..page_count {
        let page_object = page_index
            .checked_mul(2)
            .and_then(|value| value.checked_add(3))
            .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
        append_limited(
            &mut pages_dictionary,
            format!(" {page_object} 0 R").as_bytes(),
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

    let first_float_object = page_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(3))
        .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
    let mut next_float_object = first_float_object;
    let mut observations = Vec::new();
    observations
        .try_reserve_exact(display.pages().len())
        .map_err(|_| StagingFloatPdfError::AllocationFailure)?;
    let mut float_objects = Vec::new();
    float_objects
        .try_reserve_exact(
            usize::try_from(float_count).map_err(|_| StagingFloatPdfError::ArithmeticOverflow)?,
        )
        .map_err(|_| StagingFloatPdfError::AllocationFailure)?;

    for page in display.pages() {
        let page_object_id = page
            .page_index()
            .checked_mul(2)
            .and_then(|value| value.checked_add(3))
            .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
        let content_object_id = page_object_id
            .checked_add(1)
            .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
        let first = usize::try_from(page.first_command())
            .map_err(|_| StagingFloatPdfError::ArithmeticOverflow)?;
        let end = first
            .checked_add(
                usize::try_from(page.command_count())
                    .map_err(|_| StagingFloatPdfError::ArithmeticOverflow)?,
            )
            .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
        let commands = display
            .commands()
            .get(first..end)
            .ok_or(StagingFloatPdfError::DisplayClosureMismatch)?;
        let media_height = page.boxes().media_box().values()[3];
        let mut content = Vec::new();
        let mut resources = String::from("<< /XObject <<");
        let mut usages = Vec::new();
        usages
            .try_reserve_exact(
                usize::try_from(page.float_command_count())
                    .map_err(|_| StagingFloatPdfError::ArithmeticOverflow)?,
            )
            .map_err(|_| StagingFloatPdfError::AllocationFailure)?;
        for (command_index, command) in commands.iter().enumerate() {
            if command.page_index() != page.page_index() || command.master_id() != page.master_id()
            {
                return Err(StagingFloatPdfError::DisplayClosureMismatch);
            }
            let expected_ordinal = first
                .checked_add(command_index)
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
            if command.command_ordinal() != expected_ordinal {
                return Err(StagingFloatPdfError::DisplayClosureMismatch);
            }
            let bounds = command.bounds();
            let pdf_y = media_height
                .checked_sub(bounds.y().raw())
                .and_then(|value| value.checked_sub(bounds.height().get().raw()))
                .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
            match command.kind() {
                StagingFloatPaintCommandKind::Body => {
                    let record = format!(
                        "% typaxis body column={} flow={} node={} paint={}\nq 0 g {} {} {} {} re f Q\n",
                        command.column_index(),
                        command.frame_flow_id().get(),
                        command.node_id().get(),
                        command.frame_paint_ordinal(),
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
                StagingFloatPaintCommandKind::Float => {
                    let float_flow_id = command
                        .float_flow_id()
                        .ok_or(StagingFloatPdfError::DisplayClosureMismatch)?;
                    let object_id = next_float_object;
                    next_float_object = next_float_object
                        .checked_add(1)
                        .ok_or(StagingFloatPdfError::ArithmeticOverflow)?;
                    resources.push_str(&format!(" /Fl{object_id} {object_id} 0 R"));
                    let class = command
                        .placement_class()
                        .ok_or(StagingFloatPdfError::DisplayClosureMismatch)?;
                    let record = format!(
                        "% typaxis float class={} column={} flow={} node={} object={} paint={}\nq 1 0 0 1 {} {} cm /Fl{} Do Q\n",
                        class.as_str(),
                        command.column_index(),
                        float_flow_id.get(),
                        command.node_id().get(),
                        object_id,
                        command.frame_paint_ordinal(),
                        pdf_fixed(bounds.x().raw()),
                        pdf_fixed(pdf_y),
                        object_id,
                    );
                    append_limited(
                        &mut content,
                        record.as_bytes(),
                        limits.get().max_output_bytes,
                    )
                    .map_err(map_common_error)?;
                    let form_content = format!(
                        "0 g 0 0 {} {} re f\n",
                        pdf_fixed(bounds.width().get().raw()),
                        pdf_fixed(bounds.height().get().raw()),
                    );
                    let dictionary = format!(
                        " /Type /XObject /Subtype /Form /BBox [0 0 {} {}] /Resources << >>",
                        pdf_fixed(bounds.width().get().raw()),
                        pdf_fixed(bounds.height().get().raw()),
                    );
                    let form = generated_stream_object(
                        &dictionary,
                        form_content.as_bytes(),
                        stream_compression,
                        limits.get().max_output_bytes,
                    )
                    .map_err(map_common_error)?;
                    float_objects.push(form);
                    usages.push(StagingFloatPdfObjectUsage {
                        command_ordinal: command.command_ordinal(),
                        float_flow_id: float_flow_id.get(),
                        figure_node_id: command.node_id(),
                        object_id,
                    });
                }
            }
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
            resources.push_str(plan.image_resource_entries());
            resources.push_str(
                " >> /Font << /F0 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>",
            );
        } else {
            resources.push_str(" >> >>");
        }
        let content_bytes = generated_stream_object(
            "",
            &content,
            stream_compression,
            limits.get().max_output_bytes,
        )
        .map_err(map_common_error)?;
        let boxes = page.boxes();
        let annotations = content_plan
            .as_ref()
            .and_then(|plan| plan.page(page.page_index()))
            .map_or("", |plan| plan.annotation_suffix());
        let page_object = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox {} /CropBox {} /TrimBox {} /Resources {} /Contents {content_object_id} 0 R{annotations} >>",
            pdf_box(boxes.media_box()),
            pdf_box(boxes.crop_box()),
            pdf_box(boxes.trim_box()),
            resources,
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
        observations.push(StagingFloatPdfPageObservation {
            page_index: page.page_index(),
            master_id: page.master_id().clone(),
            boxes,
            page_object_id,
            content_object_id,
            command_count: page.command_count(),
            float_object_usages: usages,
        });
    }
    for object in float_objects {
        push_object(
            &mut objects,
            object,
            &mut staged_payload_bytes,
            limits.get().max_output_bytes,
        )
        .map_err(map_common_error)?;
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
    if objects.len() != object_capacity
        || next_float_object.checked_sub(1) != Some(base_object_count)
    {
        return Err(StagingFloatPdfError::DisplayClosureMismatch);
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
    let receipt = StagingFloatPdfClosureReceipt {
        display_paint_sha256: display.receipt().paint_closure_sha256(),
        pdf_sha256,
        paint_closure_sha256: sha256(canonical_jcs.as_bytes()),
        object_count,
        canonical_jcs,
    };
    let pdf = StagingFloatPdf {
        bytes,
        pages: observations,
        content: content_plan.as_ref().map(|plan| plan.observation().clone()),
        receipt,
        stream_compression,
    };
    pdf.verify_receipt(display)?;
    Ok(pdf)
}

fn map_common_error(error: crate::StagingHeaderFooterPdfError) -> StagingFloatPdfError {
    match error {
        crate::StagingHeaderFooterPdfError::DisplayClosureMismatch => {
            StagingFloatPdfError::DisplayClosureMismatch
        }
        crate::StagingHeaderFooterPdfError::PageObjectLimit => {
            StagingFloatPdfError::PageObjectLimit
        }
        crate::StagingHeaderFooterPdfError::OutputLimit => StagingFloatPdfError::OutputLimit,
        crate::StagingHeaderFooterPdfError::ArithmeticOverflow => {
            StagingFloatPdfError::ArithmeticOverflow
        }
        crate::StagingHeaderFooterPdfError::AllocationFailure => {
            StagingFloatPdfError::AllocationFailure
        }
    }
}

fn encode_pdf_closure(
    display_paint_sha256: [u8; 32],
    pages: &[StagingFloatPdfPageObservation],
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
        output.push_str(",\"float_object_usages\":[");
        for (usage_index, usage) in page.float_object_usages.iter().enumerate() {
            if usage_index > 0 {
                output.push(',');
            }
            output.push_str("{\"command_ordinal\":");
            output.push_str(&usage.command_ordinal.to_string());
            output.push_str(",\"figure_node_id\":");
            output.push_str(&usage.figure_node_id.get().to_string());
            output.push_str(",\"float_flow_id\":");
            output.push_str(&usage.float_flow_id.to_string());
            output.push_str(",\"object_id\":");
            output.push_str(&usage.object_id.to_string());
            output.push('}');
        }
        output.push_str("],\"master_id\":");
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
    fn floats_pdf_closure_has_dedicated_usage_type() {
        assert_eq!(
            STAGING_HEADER_FOOTER_PDF_CLOSURE_ALGORITHM,
            "typaxis.advanced-pagination-pdf-closure/1"
        );
    }

    #[test]
    fn floats_pdf_closes_every_placement_to_one_form_object_usage() {
        let display = typaxis_display_list::staging_float_display_fixture();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let first = serialize_staging_float_pdf(&display, &limits).unwrap();
        let second = serialize_staging_float_pdf(&display, &limits).unwrap();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.pages().len(), 3);
        assert_eq!(
            first
                .pages()
                .iter()
                .flat_map(StagingFloatPdfPageObservation::float_object_usages)
                .count(),
            5
        );
        assert_eq!(first.receipt().object_count(), 13);
        let text = String::from_utf8_lossy(first.bytes());
        assert_eq!(text.matches("/Subtype /Form").count(), 5);
        assert!(text.contains("% typaxis float class=here"));
        assert!(text.contains("% typaxis float class=top"));
        first.verify_receipt(&display).unwrap();
    }

    #[test]
    fn floats_pdf_rejects_missing_extra_wrong_page_flow_and_object_usage() {
        let display = typaxis_display_list::staging_float_display_fixture();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();

        let mut missing = serialize_staging_float_pdf(&display, &limits).unwrap();
        missing.pages[0].float_object_usages.pop();
        assert!(matches!(
            missing.verify_receipt(&display),
            Err(StagingFloatPdfError::DisplayClosureMismatch)
        ));

        let mut extra = serialize_staging_float_pdf(&display, &limits).unwrap();
        let duplicate = extra.pages[0].float_object_usages[0].clone();
        extra.pages[0].float_object_usages.push(duplicate);
        assert!(matches!(
            extra.verify_receipt(&display),
            Err(StagingFloatPdfError::DisplayClosureMismatch)
        ));

        let mut wrong_page = serialize_staging_float_pdf(&display, &limits).unwrap();
        wrong_page.pages[0].page_index = 1;
        assert!(matches!(
            wrong_page.verify_receipt(&display),
            Err(StagingFloatPdfError::DisplayClosureMismatch)
        ));

        let mut wrong_flow = serialize_staging_float_pdf(&display, &limits).unwrap();
        wrong_flow.pages[0].float_object_usages[0].float_flow_id = 99;
        assert!(matches!(
            wrong_flow.verify_receipt(&display),
            Err(StagingFloatPdfError::DisplayClosureMismatch)
        ));

        let mut wrong_object = serialize_staging_float_pdf(&display, &limits).unwrap();
        wrong_object.pages[0].float_object_usages[0].object_id = 13;
        assert!(matches!(
            wrong_object.verify_receipt(&display),
            Err(StagingFloatPdfError::DisplayClosureMismatch)
        ));
    }

    #[test]
    fn floats_pdf_object_and_output_limits_are_inclusive() {
        let display = typaxis_display_list::staging_float_display_fixture();
        let baseline = serialize_staging_float_pdf(
            &display,
            &ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
        )
        .unwrap();
        let exact = ValidatedResourceLimits::new(ResourceLimits {
            max_pdf_objects: 13,
            max_output_bytes: u64::try_from(baseline.bytes().len()).unwrap(),
            ..ResourceLimits::default()
        })
        .unwrap();
        serialize_staging_float_pdf(&display, &exact).unwrap();

        let over = ValidatedResourceLimits::new(ResourceLimits {
            max_pdf_objects: 12,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert!(matches!(
            serialize_staging_float_pdf(&display, &over),
            Err(StagingFloatPdfError::PageObjectLimit)
        ));
    }
}
