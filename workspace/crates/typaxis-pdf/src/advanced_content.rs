use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{ImageResourceId, PdfStreamCompression, ValidatedResourceLimits};
use typaxis_display_list::{
    StagingAdvancedContentBinding, StagingAdvancedLinkTarget, StagingSelectedPageBoxes,
};
use typaxis_resources::{
    freeze_admitted_png_images_for_pdf, AdmittedResourceLedger, FrozenPdfAlphaMask,
    FrozenPdfImagePlan, ImageColorSpace, ImageEncoding, ResourceError,
};

use crate::advanced_header_footer::{
    append_actual_text, append_limited, generated_stream_object, pdf_fixed,
    StagingHeaderFooterPdfError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AdvancedPdfContentObservation {
    binding_sha256: [u8; 32],
    resource_ledger_sha256: [u8; 32],
    image_use_count: u32,
    image_resource_count: u32,
    image_object_count: u32,
    annotation_count: u32,
    destination_count: u32,
    extra_object_count: u32,
}

impl AdvancedPdfContentObservation {
    pub(crate) const fn extra_object_count(&self) -> u32 {
        self.extra_object_count
    }

    pub(crate) fn verify(
        &self,
        binding: &StagingAdvancedContentBinding,
        pdf_bytes: &[u8],
    ) -> Result<(), StagingHeaderFooterPdfError> {
        let image_uses = binding
            .pages()
            .iter()
            .flat_map(|page| page.images())
            .collect::<Vec<_>>();
        let image_resources = image_uses
            .iter()
            .map(|usage| usage.image_id())
            .collect::<BTreeSet<_>>();
        let annotation_count = binding
            .pages()
            .iter()
            .try_fold(0usize, |count, page| count.checked_add(page.links().len()))
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let destination_count = binding
            .pages()
            .iter()
            .try_fold(0usize, |count, page| {
                count.checked_add(page.anchors().len())
            })
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        if self.binding_sha256 != binding.fingerprint()
            || self.resource_ledger_sha256 != binding.resource_ledger_sha256()
            || usize::try_from(self.image_use_count) != Ok(image_uses.len())
            || usize::try_from(self.image_resource_count) != Ok(image_resources.len())
            || usize::try_from(self.annotation_count) != Ok(annotation_count)
            || usize::try_from(self.destination_count) != Ok(destination_count)
            || self.extra_object_count
                != self
                    .image_object_count
                    .checked_add(self.annotation_count)
                    .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?
            || structural_marker_count(pdf_bytes, b"/Subtype /Image")
                != Some(self.image_object_count)
            || structural_marker_count(pdf_bytes, b"/Subtype /Link") != Some(self.annotation_count)
            || structural_marker_count(pdf_bytes, b"/Dests")
                != Some(u32::from(self.destination_count != 0))
        {
            return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct AdvancedPdfPageContentPlan {
    resources: String,
    image_resource_entries: String,
    annotation_suffix: String,
    stream_suffix: Vec<u8>,
}

impl AdvancedPdfPageContentPlan {
    pub(crate) fn resources(&self) -> &str {
        &self.resources
    }

    pub(crate) fn annotation_suffix(&self) -> &str {
        &self.annotation_suffix
    }

    pub(crate) fn image_resource_entries(&self) -> &str {
        &self.image_resource_entries
    }

    pub(crate) fn stream_suffix(&self) -> &[u8] {
        &self.stream_suffix
    }
}

#[derive(Debug)]
pub(crate) struct AdvancedPdfContentPlan {
    catalog_suffix: String,
    pages: Vec<AdvancedPdfPageContentPlan>,
    objects: Vec<Vec<u8>>,
    observation: AdvancedPdfContentObservation,
}

impl AdvancedPdfContentPlan {
    pub(crate) fn catalog_suffix(&self) -> &str {
        &self.catalog_suffix
    }

    pub(crate) fn page(&self, page_index: u32) -> Option<&AdvancedPdfPageContentPlan> {
        self.pages.get(usize::try_from(page_index).ok()?)
    }

    pub(crate) fn objects(&self) -> &[Vec<u8>] {
        &self.objects
    }

    pub(crate) fn observation(&self) -> &AdvancedPdfContentObservation {
        &self.observation
    }
}

#[derive(Clone, Debug)]
struct ImageObjectBinding {
    name: String,
    object_id: u32,
    width: u32,
    height: u32,
}

pub(crate) fn prepare_advanced_pdf_content(
    binding: &StagingAdvancedContentBinding,
    admitted: &AdmittedResourceLedger,
    page_boxes: &[StagingSelectedPageBoxes],
    base_object_count: u32,
    limits: &ValidatedResourceLimits,
    stream_compression: PdfStreamCompression,
) -> Result<AdvancedPdfContentPlan, StagingHeaderFooterPdfError> {
    binding
        .verify(page_boxes.len())
        .map_err(|_| StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
    if binding.resource_ledger_sha256() != admitted.fingerprint().bytes() {
        return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
    }
    let image_uses = binding
        .pages()
        .iter()
        .flat_map(|page| page.images())
        .collect::<Vec<_>>();
    let selected_image_ids = image_uses
        .iter()
        .map(|usage| usage.image_id())
        .collect::<Vec<_>>();
    let image_plans = freeze_admitted_png_images_for_pdf(admitted, &selected_image_ids, limits)
        .map_err(|error| match error {
            ResourceError::ResourceLimit => StagingHeaderFooterPdfError::OutputLimit,
            _ => StagingHeaderFooterPdfError::DisplayClosureMismatch,
        })?;

    let image_object_count = image_plans
        .iter()
        .try_fold(0u32, |count, plan| {
            count.checked_add(plan.indirect_object_count())
        })
        .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    let annotation_count = binding.pages().iter().try_fold(0u32, |count, page| {
        count
            .checked_add(
                u32::try_from(page.links().len())
                    .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?,
            )
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)
    })?;
    let destination_count = binding.pages().iter().try_fold(0u32, |count, page| {
        count
            .checked_add(
                u32::try_from(page.anchors().len())
                    .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?,
            )
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)
    })?;
    let extra_object_count = image_object_count
        .checked_add(annotation_count)
        .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    let final_object_count = base_object_count
        .checked_add(extra_object_count)
        .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    if final_object_count > limits.get().max_pdf_objects {
        return Err(StagingHeaderFooterPdfError::PageObjectLimit);
    }

    let mut objects = Vec::new();
    objects
        .try_reserve_exact(
            usize::try_from(extra_object_count)
                .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?,
        )
        .map_err(|_| StagingHeaderFooterPdfError::AllocationFailure)?;
    let mut image_bindings = BTreeMap::new();
    let mut next_object_id = base_object_count
        .checked_add(1)
        .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    for (resource_ordinal, plan) in image_plans.iter().enumerate() {
        let image_object_id = next_object_id;
        next_object_id = next_object_id
            .checked_add(1)
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let alpha_object_id = if plan.alpha_mask().is_some() {
            let value = next_object_id;
            next_object_id = next_object_id
                .checked_add(1)
                .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
            Some(value)
        } else {
            None
        };
        objects.push(image_stream(
            plan,
            alpha_object_id,
            stream_compression,
            limits.get().max_output_bytes,
        )?);
        if let (Some(mask), Some(_)) = (plan.alpha_mask(), alpha_object_id) {
            objects.push(alpha_stream(
                mask,
                stream_compression,
                limits.get().max_output_bytes,
            )?);
        }
        let name = format!("Im{resource_ordinal}");
        if image_bindings
            .insert(
                plan.image_id(),
                ImageObjectBinding {
                    name,
                    object_id: image_object_id,
                    width: plan.width().get(),
                    height: plan.height().get(),
                },
            )
            .is_some()
        {
            return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
        }
    }

    let first_annotation_object = next_object_id;
    let mut next_annotation_object = first_annotation_object;
    let mut page_annotation_ids = Vec::new();
    page_annotation_ids
        .try_reserve_exact(binding.pages().len())
        .map_err(|_| StagingHeaderFooterPdfError::AllocationFailure)?;
    for (page, boxes) in binding.pages().iter().zip(page_boxes) {
        let mut ids = Vec::new();
        ids.try_reserve_exact(page.links().len())
            .map_err(|_| StagingHeaderFooterPdfError::AllocationFailure)?;
        for (link_ordinal, link) in page.links().iter().enumerate() {
            let object_id = next_annotation_object;
            next_annotation_object = next_annotation_object
                .checked_add(1)
                .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
            ids.push(object_id);
            objects.push(link_annotation_object(
                link.target(),
                boxes.media_box().values(),
                link_ordinal,
            )?);
        }
        page_annotation_ids.push(ids);
    }
    if next_annotation_object
        .checked_sub(1)
        .unwrap_or(base_object_count)
        != final_object_count
        || objects.len()
            != usize::try_from(extra_object_count)
                .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?
    {
        return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
    }

    let mut pages = Vec::new();
    pages
        .try_reserve_exact(binding.pages().len())
        .map_err(|_| StagingHeaderFooterPdfError::AllocationFailure)?;
    for ((page, boxes), annotation_ids) in binding
        .pages()
        .iter()
        .zip(page_boxes)
        .zip(&page_annotation_ids)
    {
        let mut resources = String::from(
            "/Resources << /Font << /F0 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >>",
        );
        let mut image_resource_entries = String::new();
        let page_image_ids = page
            .images()
            .iter()
            .map(|usage| usage.image_id())
            .collect::<BTreeSet<_>>();
        if !page_image_ids.is_empty() {
            resources.push_str(" /XObject <<");
            for image_id in page_image_ids {
                let image = image_bindings
                    .get(&image_id)
                    .ok_or(StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
                let entry = format!(" /{} {} 0 R", image.name, image.object_id);
                resources.push_str(&entry);
                image_resource_entries.push_str(&entry);
            }
            resources.push_str(" >>");
        }
        resources.push_str(" >>");

        let mut stream_suffix = Vec::new();
        let binding_comment = format!(
            "% typaxis content-binding={} resource-ledger={}\n",
            hex(binding.fingerprint()),
            hex(binding.resource_ledger_sha256())
        );
        append_limited(
            &mut stream_suffix,
            binding_comment.as_bytes(),
            limits.get().max_output_bytes,
        )?;
        append_image_draws(
            &mut stream_suffix,
            page,
            boxes.media_box().values(),
            &image_bindings,
            limits.get().max_output_bytes,
        )?;
        append_actual_text(
            &mut stream_suffix,
            page.text(),
            limits.get().max_output_bytes,
        )?;
        let annotation_suffix = if annotation_ids.is_empty() {
            String::new()
        } else {
            let mut value = String::from(" /Annots [");
            for object_id in annotation_ids {
                value.push_str(&format!(" {object_id} 0 R"));
            }
            value.push_str(" ]");
            value
        };
        pages.push(AdvancedPdfPageContentPlan {
            resources,
            image_resource_entries,
            annotation_suffix,
            stream_suffix,
        });
    }

    let catalog_suffix = destination_name_tree(binding, page_boxes)?;
    let observation = AdvancedPdfContentObservation {
        binding_sha256: binding.fingerprint(),
        resource_ledger_sha256: binding.resource_ledger_sha256(),
        image_use_count: u32::try_from(image_uses.len())
            .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?,
        image_resource_count: u32::try_from(image_bindings.len())
            .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?,
        image_object_count,
        annotation_count,
        destination_count,
        extra_object_count,
    };
    Ok(AdvancedPdfContentPlan {
        catalog_suffix,
        pages,
        objects,
        observation,
    })
}

fn image_stream(
    plan: &FrozenPdfImagePlan,
    alpha_object_id: Option<u32>,
    compression: PdfStreamCompression,
    max_output_bytes: u64,
) -> Result<Vec<u8>, StagingHeaderFooterPdfError> {
    if plan.encoding() != ImageEncoding::Raw {
        return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
    }
    let color_space = match plan.color_space() {
        ImageColorSpace::Gray => "DeviceGray",
        ImageColorSpace::Rgb => "DeviceRGB",
        ImageColorSpace::Cmyk => "DeviceCMYK",
    };
    let mut dictionary = format!(
        " /BitsPerComponent {} /ColorSpace /{} /Height {}",
        plan.bits_per_component(),
        color_space,
        plan.height().get()
    );
    if let Some(mask) = alpha_object_id {
        dictionary.push_str(&format!(" /SMask {mask} 0 R"));
    }
    dictionary.push_str(&format!(
        " /Subtype /Image /Type /XObject /Width {}",
        plan.width().get()
    ));
    generated_stream_object(
        &dictionary,
        plan.encoded_bytes(),
        compression,
        max_output_bytes,
    )
}

fn alpha_stream(
    mask: &FrozenPdfAlphaMask,
    compression: PdfStreamCompression,
    max_output_bytes: u64,
) -> Result<Vec<u8>, StagingHeaderFooterPdfError> {
    if mask.encoding() != ImageEncoding::Raw {
        return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
    }
    generated_stream_object(
        &format!(
            " /BitsPerComponent {} /ColorSpace /DeviceGray /Height {} /Subtype /Image /Type /XObject /Width {}",
            mask.bits_per_component(),
            mask.height().get(),
            mask.width().get()
        ),
        mask.encoded_bytes(),
        compression,
        max_output_bytes,
    )
}

fn append_image_draws(
    output: &mut Vec<u8>,
    page: &typaxis_display_list::StagingAdvancedPageContent,
    media: [i64; 4],
    bindings: &BTreeMap<ImageResourceId, ImageObjectBinding>,
    max_output_bytes: u64,
) -> Result<(), StagingHeaderFooterPdfError> {
    let width = media[2]
        .checked_sub(media[0])
        .filter(|value| *value > 0)
        .ok_or(StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
    let height = media[3]
        .checked_sub(media[1])
        .filter(|value| *value > 0)
        .ok_or(StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
    const MAX_DRAW_RAW: i64 = 32 * 65_536;
    for (ordinal, usage) in page.images().iter().enumerate() {
        let image = bindings
            .get(&usage.image_id())
            .ok_or(StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
        let draw_width = width.clamp(1, MAX_DRAW_RAW);
        let scaled_height = i128::from(draw_width)
            .checked_mul(i128::from(image.height))
            .and_then(|value| value.checked_div(i128::from(image.width)))
            .and_then(|value| i64::try_from(value).ok())
            .unwrap_or(1)
            .clamp(1, height.min(MAX_DRAW_RAW));
        let ordinal =
            i64::try_from(ordinal).map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let x_slots = (width / draw_width).max(1);
        let x = ordinal
            .checked_rem(x_slots)
            .and_then(|value| value.checked_mul(draw_width))
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let row = ordinal
            .checked_div(x_slots)
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let top = row
            .checked_mul(scaled_height)
            .and_then(|value| value.checked_rem(height))
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let pdf_y = height
            .checked_sub(top)
            .and_then(|value| value.checked_sub(scaled_height))
            .unwrap_or(0);
        let command = format!(
            "% typaxis image node={} image={}\nq {} 0 0 {} {} {} cm /{} Do Q\n",
            usage.node_id().get(),
            usage.image_id().get(),
            pdf_fixed(draw_width),
            pdf_fixed(scaled_height),
            pdf_fixed(x),
            pdf_fixed(pdf_y),
            image.name,
        );
        append_limited(output, command.as_bytes(), max_output_bytes)?;
    }
    Ok(())
}

fn destination_name_tree(
    binding: &StagingAdvancedContentBinding,
    page_boxes: &[StagingSelectedPageBoxes],
) -> Result<String, StagingHeaderFooterPdfError> {
    let mut anchors = BTreeMap::new();
    for page in binding.pages() {
        for anchor in page.anchors() {
            if anchors
                .insert(anchor.anchor_id().clone(), page.page_index())
                .is_some()
            {
                return Err(StagingHeaderFooterPdfError::DisplayClosureMismatch);
            }
        }
    }
    if anchors.is_empty() {
        return Ok(String::new());
    }
    let mut output = String::from(" /Names << /Dests << /Names [");
    for (anchor_id, page_index) in anchors {
        let page_object_id = page_index
            .checked_mul(2)
            .and_then(|value| value.checked_add(3))
            .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
        let page_height = page_boxes
            .get(
                usize::try_from(page_index)
                    .map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?,
            )
            .ok_or(StagingHeaderFooterPdfError::DisplayClosureMismatch)?
            .media_box()
            .values()[3];
        output.push(' ');
        push_hex_string(&mut output, anchor_id.as_str().as_bytes());
        output.push_str(&format!(
            " [{page_object_id} 0 R /XYZ 0 {} null]",
            pdf_fixed(page_height)
        ));
    }
    output.push_str(" ] >> >>");
    Ok(output)
}

fn link_annotation_object(
    target: &StagingAdvancedLinkTarget,
    media: [i64; 4],
    ordinal: usize,
) -> Result<Vec<u8>, StagingHeaderFooterPdfError> {
    let width = media[2]
        .checked_sub(media[0])
        .filter(|value| *value > 0)
        .ok_or(StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
    let height = media[3]
        .checked_sub(media[1])
        .filter(|value| *value > 0)
        .ok_or(StagingHeaderFooterPdfError::DisplayClosureMismatch)?;
    let rect_width = width.clamp(1, 8 * 65_536);
    let rect_height = height.clamp(1, 2 * 65_536);
    let ordinal =
        i64::try_from(ordinal).map_err(|_| StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    let top = ordinal
        .checked_mul(rect_height)
        .and_then(|value| value.checked_rem(height))
        .ok_or(StagingHeaderFooterPdfError::ArithmeticOverflow)?;
    let bottom = height
        .checked_sub(top)
        .and_then(|value| value.checked_sub(rect_height))
        .unwrap_or(0);
    let mut output = format!(
        "<< /Border [0 0 0] /Rect [0 {} {} {}] /Subtype /Link",
        pdf_fixed(bottom),
        pdf_fixed(rect_width),
        pdf_fixed(bottom.saturating_add(rect_height)),
    );
    match target {
        StagingAdvancedLinkTarget::Internal { anchor_id, .. } => {
            output.push_str(" /Dest ");
            push_hex_string(&mut output, anchor_id.as_str().as_bytes());
        }
        StagingAdvancedLinkTarget::Uri(uri) => {
            output.push_str(" /A << /S /URI /URI ");
            push_hex_string(&mut output, uri.as_str().as_bytes());
            output.push_str(" >>");
        }
    }
    output.push_str(" /Type /Annot >>");
    Ok(output.into_bytes())
}

fn push_hex_string(output: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    output.push('<');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('>');
}

fn structural_marker_count(bytes: &[u8], marker: &[u8]) -> Option<u32> {
    if marker.is_empty() {
        return None;
    }
    let mut count = 0u32;
    let mut position = 0usize;
    while position < bytes.len() {
        if bytes[position..].starts_with(b"stream\n") {
            let length_marker = b"/Length ";
            let length_start = bytes[..position]
                .windows(length_marker.len())
                .rposition(|window| window == length_marker)?
                .checked_add(length_marker.len())?;
            let length_end = bytes[length_start..position]
                .iter()
                .position(|byte| !byte.is_ascii_digit())?
                .checked_add(length_start)?;
            let length = std::str::from_utf8(bytes.get(length_start..length_end)?)
                .ok()?
                .parse::<usize>()
                .ok()?;
            position = position.checked_add(b"stream\n".len())?;
            position = position.checked_add(length)?;
            let suffix = bytes.get(position..)?;
            position = if suffix.starts_with(b"endstream") {
                position.checked_add(b"endstream".len())?
            } else if suffix.starts_with(b"\nendstream") {
                position.checked_add(b"\nendstream".len())?
            } else {
                return None;
            };
            continue;
        }
        if bytes[position..].starts_with(marker) {
            count = count.checked_add(1)?;
            position = position.checked_add(marker.len())?;
        } else {
            position = position.checked_add(1)?;
        }
    }
    Some(count)
}

fn hex(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::structural_marker_count;

    #[test]
    fn structural_marker_count_ignores_stream_payload_bytes() {
        let pdf = b"1 0 obj\n<< /Length 15 >>\nstream\n/Subtype /Image\nendstream\nendobj\n2 0 obj\n<< /Length 16 >>\nstream\n/Subtype /Image\nendstream\nendobj\n3 0 obj\n<< /Subtype /Image >>\nendobj\n";
        assert_eq!(structural_marker_count(pdf, b"/Subtype /Image"), Some(1));
        assert_eq!(structural_marker_count(pdf, b"/Subtype /Link"), Some(0));
    }
}
