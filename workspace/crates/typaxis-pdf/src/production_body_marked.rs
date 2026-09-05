//! MCID-bearing page contributions from the same selected paints as structure.
//! Object allocation, navigation and the final PDF authorization remain later
//! owners; these bytes alone are not a complete tagged document.
use crate::{ProductionBodyPageContent, ProductionBodyPageDrawSource};
use typaxis_core::M4EffectiveResourceLimits;
use typaxis_display_list::ProductionBodyStructure;
use typaxis_resource_admission::AdmittedResourceLedger;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyMarkedError {
    ReceiptMismatch,
    RecordLimit,
    OutputLimit,
    AllocationFailure,
}
pub struct ProductionBodyMarkedPage {
    page_index: u32,
    content: Vec<u8>,
}
impl ProductionBodyMarkedPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn content(&self) -> &[u8] {
        &self.content
    }
}
pub struct ProductionBodyMarkedContent<'c, 'f, 'v, 'd, 's, 'p, 'a> {
    content: &'c ProductionBodyPageContent<'f, 'v, 'd, 's, 'p, 'a>,
    structure: &'c ProductionBodyStructure<'v, 'd, 's, 'p, 'a>,
    pages: Vec<ProductionBodyMarkedPage>,
    record_charge: u64,
    spool_charge: u64,
}
impl<'c, 'f, 'v, 'd, 's, 'p, 'a> ProductionBodyMarkedContent<'c, 'f, 'v, 'd, 's, 'p, 'a> {
    pub const fn content(&self) -> &'c ProductionBodyPageContent<'f, 'v, 'd, 's, 'p, 'a> {
        self.content
    }
    pub const fn structure(&self) -> &'c ProductionBodyStructure<'v, 'd, 's, 'p, 'a> {
        self.structure
    }
    pub fn pages(&self) -> &[ProductionBodyMarkedPage] {
        &self.pages
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub fn verify(
        &self,
        content: &ProductionBodyPageContent<'_, '_, '_, '_, '_, '_>,
        structure: &ProductionBodyStructure<'_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyMarkedError> {
        if !std::ptr::eq(self.content, content) || !std::ptr::eq(self.structure, structure) {
            return Err(ProductionBodyMarkedError::ReceiptMismatch);
        }
        content
            .verify(content.plans().fonts(), admitted, limits)
            .map_err(|_| ProductionBodyMarkedError::ReceiptMismatch)?;
        structure
            .verify(content.plans().fonts().display(), admitted, limits)
            .map_err(|_| ProductionBodyMarkedError::ReceiptMismatch)
    }
}

pub fn build_production_body_marked_content<'c, 'f, 'v, 'd, 's, 'p, 'a>(
    content: &'c ProductionBodyPageContent<'f, 'v, 'd, 's, 'p, 'a>,
    structure: &'c ProductionBodyStructure<'v, 'd, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodyMarkedContent<'c, 'f, 'v, 'd, 's, 'p, 'a>, ProductionBodyMarkedError> {
    use ProductionBodyMarkedError as E;
    let display = content.plans().fonts().display();
    content
        .verify(content.plans().fonts(), admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    structure
        .verify(display, admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    // Combine branches without charging their common display twice. Neither
    // branch receives a fresh resource budget at this merge.
    let record_charge = content
        .plans()
        .record_charge()
        .checked_add(
            structure
                .record_charge()
                .checked_sub(display.record_charge())
                .ok_or(E::RecordLimit)?,
        )
        .and_then(|v| v.checked_add(content.pages().len() as u64))
        .ok_or(E::RecordLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    let mut spool_charge = content
        .spool_charge()
        .checked_add(structure.spool_charge())
        .ok_or(E::OutputLimit)?;
    if spool_charge > limits.base().get().max_spool_bytes {
        return Err(E::OutputLimit);
    }
    let mut output_bytes = 0u64;
    let mut append = |out: &mut Vec<u8>, bytes: &[u8]| -> Result<(), E> {
        spool_charge = spool_charge
            .checked_add(bytes.len() as u64)
            .ok_or(E::OutputLimit)?;
        output_bytes = output_bytes
            .checked_add(bytes.len() as u64)
            .ok_or(E::OutputLimit)?;
        if spool_charge > limits.base().get().max_spool_bytes
            || output_bytes > limits.base().get().max_output_bytes
        {
            return Err(E::OutputLimit);
        }
        out.try_reserve(bytes.len())
            .map_err(|_| E::AllocationFailure)?;
        out.extend_from_slice(bytes);
        Ok(())
    };
    let root = format!(
        "q\n1 0 0 -1 0 {} cm\n",
        crate::tagged_pdf_v2::pdf_number_v2(
            display.selected().page_geometry().page_height().get().raw()
        )
    );
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(content.pages().len())
        .map_err(|_| E::AllocationFailure)?;
    let mut group_index = 0usize;
    for source in content.pages() {
        let mut page = ProductionBodyMarkedPage {
            page_index: source.page_index(),
            content: Vec::new(),
        };
        append(&mut page.content, root.as_bytes())?;
        let mut ordinal = 0usize;
        for group in structure
            .page_groups(source.page_index())
            .ok_or(E::ReceiptMismatch)?
        {
            let node = structure
                .registry()
                .node(group.node())
                .ok_or(E::ReceiptMismatch)?;
            append(
                &mut page.content,
                format!(
                    "/{} << /MCID {} /Lang <FEFF",
                    node.role().pdf_name(),
                    group.mcid()
                )
                .as_bytes(),
            )?;
            for unit in node.language().encode_utf16() {
                append(&mut page.content, format!("{unit:04X}").as_bytes())?;
            }
            append(&mut page.content, b">")?;
            if let Some(text) = structure.group_actual_text(group_index) {
                append(&mut page.content, b" /ActualText <FEFF")?;
                for unit in text.encode_utf16() {
                    append(&mut page.content, format!("{unit:04X}").as_bytes())?;
                }
                append(&mut page.content, b">")?;
            }
            append(&mut page.content, b" >> BDC\n")?;
            for draw_index in group.draws() {
                let draw = source.draws().get(ordinal).ok_or(E::ReceiptMismatch)?;
                if draw.draw_index() != draw_index {
                    return Err(E::ReceiptMismatch);
                }
                match (draw.source(), group.vector_usage_id()) {
                    (ProductionBodyPageDrawSource::Text { .. }, None) => {}
                    (ProductionBodyPageDrawSource::Vector { usage_index }, Some(id))
                        if usage_index == id as usize => {}
                    _ => return Err(E::ReceiptMismatch),
                }
                append(
                    &mut page.content,
                    source.draw_content(ordinal).ok_or(E::ReceiptMismatch)?,
                )?;
                append(&mut page.content, b"\n")?;
                ordinal += 1;
            }
            append(&mut page.content, b"EMC\n")?;
            group_index += 1;
        }
        if ordinal != source.draws().len() {
            return Err(E::ReceiptMismatch);
        }
        append(&mut page.content, b"Q\n")?;
        pages.push(page);
    }
    if group_index != structure.groups().len() {
        return Err(E::ReceiptMismatch);
    }
    Ok(ProductionBodyMarkedContent {
        content,
        structure,
        pages,
        record_charge,
        spool_charge,
    })
}
