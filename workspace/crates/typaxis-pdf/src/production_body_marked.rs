//! MCID-bearing page contributions from the same selected paints as structure.
//! Object allocation, navigation and the final PDF authorization remain later
//! owners; these bytes alone are not a complete tagged document.
use crate::{ProductionBodyPageContent, ProductionBodyPageDrawSource};
use typaxis_core::{Length, M4EffectiveResourceLimits, Rect};
use typaxis_display_list::{ProductionBodyDraw, ProductionBodyStructure};
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

/// A managed, nonpainting glyph usage which supplies the Formula ActualText
/// with the selected viewport dimensions and baseline. It is not a document
/// text/source node; its synthetic font box is not the formula's painted bbox.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodySemanticAnchor {
    page_index: u32,
    group_index: usize,
    viewport: Rect,
    baseline: Length,
}
impl ProductionBodySemanticAnchor {
    pub const fn page_index(self) -> u32 {
        self.page_index
    }
    pub const fn group_index(self) -> usize {
        self.group_index
    }
    pub const fn viewport(self) -> Rect {
        self.viewport
    }
    pub const fn baseline(self) -> Length {
        self.baseline
    }
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
    anchors: Vec<ProductionBodySemanticAnchor>,
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
    pub fn anchors(&self) -> &[ProductionBodySemanticAnchor] {
        &self.anchors
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
    let record_base = content
        .record_charge()
        .checked_add(
            structure
                .record_charge()
                .checked_sub(display.record_charge())
                .ok_or(E::RecordLimit)?,
        )
        .ok_or(E::RecordLimit)?;
    let spool_base = content
        .spool_charge()
        .checked_add(
            structure
                .spool_charge()
                .checked_sub(display.selected().line_layout().native_math_spool_charge())
                .ok_or(E::OutputLimit)?,
        )
        .ok_or(E::OutputLimit)?;
    let projected = project_marked_pages(
        display.draws(),
        |_| false,
        content.pages(),
        display.selected().page_geometry().page_height().get().raw(),
        structure.registry(),
        structure.groups().len(),
        |p| structure.page_groups(p),
        |i| structure.group_actual_text(i),
        content.text().paints(),
        |i| content.text().paint_commands(i),
        |i| content.rasters().draw_plan(i),
        record_base,
        spool_base,
        limits,
    )?;
    Ok(ProductionBodyMarkedContent {
        content,
        structure,
        pages: projected.pages,
        anchors: projected.anchors,
        record_charge: projected.record_charge,
        spool_charge: projected.spool_charge,
    })
}

struct MarkedProjection {
    pages: Vec<ProductionBodyMarkedPage>,
    anchors: Vec<ProductionBodySemanticAnchor>,
    record_charge: u64,
    spool_charge: u64,
}
fn project_marked_pages<'s>(
    draws: &[ProductionBodyDraw<'_>],
    is_artifact: impl Fn(usize) -> bool,
    source_pages: &[crate::ProductionBodyPage],
    page_height: i64,
    registry: &typaxis_display_list::StructureRegistryReceiptV2,
    group_count: usize,
    page_groups: impl Fn(u32) -> Option<&'s [typaxis_display_list::ProductionBodyStructureGroup]>,
    actual_text: impl Fn(usize) -> Option<&'s str>,
    text_paints: &[crate::ProductionBodyTextPaint],
    text_commands: impl Fn(usize) -> Option<&'s [u8]>,
    raster_plan: impl Fn(usize) -> Option<usize>,
    record_base: u64,
    mut spool_charge: u64,
    limits: &M4EffectiveResourceLimits,
) -> Result<MarkedProjection, ProductionBodyMarkedError> {
    use ProductionBodyMarkedError as E;
    let mut record_charge = record_base
        .checked_add(source_pages.len() as u64)
        .ok_or(E::RecordLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
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
        crate::tagged_pdf_v2::pdf_number_v2(page_height)
    );
    let mut pages = Vec::new();
    let mut anchors = Vec::new();
    pages
        .try_reserve_exact(source_pages.len())
        .map_err(|_| E::AllocationFailure)?;
    let mut group_index = 0usize;
    for source in source_pages {
        let mut page = ProductionBodyMarkedPage {
            page_index: source.page_index(),
            content: Vec::new(),
        };
        append(&mut page.content, root.as_bytes())?;
        let mut ordinal = 0usize;
        let mut artifact_index = 0usize;
        for group in page_groups(source.page_index()).ok_or(E::ReceiptMismatch)? {
            append_header_copies(
                source,
                &is_artifact,
                Some(group.draws().start),
                &mut ordinal,
                &mut artifact_index,
                &mut page.content,
                &mut append,
            )?;
            // A separator belongs between source groups, outside every MCID
            // and ActualText scope. Reject an artifact that splits a group.
            if let Some(artifact) = source.artifacts().get(artifact_index) {
                if artifact.before_draw() < group.draws().start {
                    return Err(E::ReceiptMismatch);
                }
                if artifact.before_draw() == group.draws().start {
                    append(
                        &mut page.content,
                        source
                            .artifact_content(artifact_index)
                            .ok_or(E::ReceiptMismatch)?,
                    )?;
                    artifact_index += 1;
                } else if artifact.before_draw() < group.draws().end {
                    return Err(E::ReceiptMismatch);
                }
            }
            let node = registry.node(group.node()).ok_or(E::ReceiptMismatch)?;
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
            append(&mut page.content, b" >> BDC\n")?;
            let body_text = group.is_text();
            if body_text {
                // Only the selected fragment, never the registry's full source
                // node. Keep authored spaces and ambiguous CID text together;
                // extractors otherwise discard standalone whitespace glyphs
                // and guess their replacement from adjacent font geometry.
                append(&mut page.content, b"/Span << /ActualText <FEFF")?;
                for index in group.draws() {
                    let Some(ProductionBodyDraw::Text(text)) = draws.get(index) else {
                        return Err(E::ReceiptMismatch);
                    };
                    for unit in text.exact_text().encode_utf16() {
                        append(&mut page.content, format!("{unit:04X}").as_bytes())?;
                    }
                }
                append(&mut page.content, b"> >> BDC\n")?;
            }
            if let Some(text) = actual_text(group_index) {
                append(&mut page.content, b"q\n/Span << /ActualText <FEFF")?;
                for unit in text.encode_utf16() {
                    append(&mut page.content, format!("{unit:04X}").as_bytes())?;
                }
                append(&mut page.content, b"> >> BDC\n")?;
            }
            if actual_text(group_index).is_some() && !group.is_native_math() {
                // Give the extraction glyph the selected formula's physical
                // width and height, at its selected baseline. A fixed 1 pt
                // vertical scale changes extractor word-gap heuristics even
                // though the anchor paints no ink. Keep both matrix scales
                // exact; a width/height quotient would introduce rounding.
                if group.draws().len() != 1 {
                    return Err(E::ReceiptMismatch);
                }
                let Some(ProductionBodyDraw::Vector(vector)) = draws.get(group.draws().start)
                else {
                    return Err(E::ReceiptMismatch);
                };
                record_charge = record_charge.checked_add(1).ok_or(E::RecordLimit)?;
                if record_charge > limits.base().get().max_fragments {
                    return Err(E::RecordLimit);
                }
                anchors.try_reserve(1).map_err(|_| E::AllocationFailure)?;
                let viewport = vector.viewport();
                let baseline = vector.baseline().ok_or(E::ReceiptMismatch)?;
                struct AnchorSink<'a, F>(&'a mut F);
                impl<F: FnMut(&[u8]) -> Result<(), ProductionBodyMarkedError>>
                    crate::font_encoding::Sink for AnchorSink<'_, F>
                {
                    type Error = ProductionBodyMarkedError;
                    fn extend(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
                        (self.0)(bytes)
                    }
                }
                crate::semantic_anchor_encoding::command(
                    &mut AnchorSink(&mut |bytes: &[u8]| append(&mut page.content, bytes)),
                    b"PBA",
                    viewport,
                    baseline,
                )?;
                anchors.push(ProductionBodySemanticAnchor {
                    page_index: page.page_index,
                    group_index,
                    viewport,
                    baseline,
                });
            }
            for draw_index in group.draws() {
                let draw = source.draws().get(ordinal).ok_or(E::ReceiptMismatch)?;
                if draw.draw_index() != draw_index {
                    return Err(E::ReceiptMismatch);
                }
                match (draw.source(), group.vector_usage_id()) {
                    (ProductionBodyPageDrawSource::Text { paint_index }, None) if body_text => {
                        let paint = text_paints.get(paint_index).ok_or(E::ReceiptMismatch)?;
                        if paint.draw_index() != draw_index || paint.page_index() != page.page_index
                        {
                            return Err(E::ReceiptMismatch);
                        }
                        append(&mut page.content, b"q\n")?;
                        append(
                            &mut page.content,
                            text_commands(paint_index).ok_or(E::ReceiptMismatch)?,
                        )?;
                        if draw_index + 1 == group.draws().end {
                            // Flush ActualText with the last retained text
                            // paint's font/matrix still active, then restore.
                            append(&mut page.content, b"EMC\n")?;
                        }
                        append(&mut page.content, b"Q\n")?;
                    }
                    (ProductionBodyPageDrawSource::Vector { usage_index }, Some(id))
                        if usage_index == id as usize =>
                    {
                        append(
                            &mut page.content,
                            source.draw_content(ordinal).ok_or(E::ReceiptMismatch)?,
                        )?;
                    }
                    (ProductionBodyPageDrawSource::NativeMath { paint_index }, None)
                        if group.is_native_math() =>
                    {
                        let paint = text_paints.get(paint_index).ok_or(E::ReceiptMismatch)?;
                        if !paint.is_native_math()
                            || paint.draw_index() != draw_index
                            || paint.page_index() != page.page_index
                            || actual_text(group_index).is_none()
                        {
                            return Err(E::ReceiptMismatch);
                        }
                        // The outer ActualText scope already owns q/Q. Keep
                        // the native font active until EMC, as for authored
                        // text; restoring it first gives extractors a zero-size
                        // formula box and can reverse their reading order.
                        append(
                            &mut page.content,
                            text_commands(paint_index).ok_or(E::ReceiptMismatch)?,
                        )?;
                    }
                    (ProductionBodyPageDrawSource::Raster { plan_index }, None) if !body_text => {
                        if raster_plan(draw_index) != Some(plan_index) {
                            return Err(E::ReceiptMismatch);
                        }
                        append(
                            &mut page.content,
                            source.draw_content(ordinal).ok_or(E::ReceiptMismatch)?,
                        )?;
                    }
                    _ => return Err(E::ReceiptMismatch),
                }
                append(&mut page.content, b"\n")?;
                ordinal += 1;
            }
            if actual_text(group_index).is_some() {
                append(&mut page.content, b"EMC\nQ\n")?;
            }
            append(&mut page.content, b"EMC\n")?;
            group_index += 1;
        }
        append_header_copies(
            source,
            &is_artifact,
            None,
            &mut ordinal,
            &mut artifact_index,
            &mut page.content,
            &mut append,
        )?;
        if ordinal != source.draws().len() || artifact_index != source.artifacts().len() {
            return Err(E::ReceiptMismatch);
        }
        append(&mut page.content, b"Q\n")?;
        pages.push(page);
    }
    if group_index != group_count {
        return Err(E::ReceiptMismatch);
    }
    Ok(MarkedProjection {
        pages,
        anchors,
        record_charge,
        spool_charge,
    })
}

// Copy the already encoded paint bytes, retaining their real fonts, Forms and
// images. Artifacts have no semantic extraction anchor, ActualText or MCID.
fn append_header_copies(
    source: &crate::ProductionBodyPage,
    is_artifact: &impl Fn(usize) -> bool,
    stop: Option<usize>,
    ordinal: &mut usize,
    separator: &mut usize,
    out: &mut Vec<u8>,
    append: &mut impl FnMut(&mut Vec<u8>, &[u8]) -> Result<(), ProductionBodyMarkedError>,
) -> Result<(), ProductionBodyMarkedError> {
    use ProductionBodyMarkedError as E;
    while let Some(draw) = source.draws().get(*ordinal) {
        if stop == Some(draw.draw_index()) {
            break;
        }
        if stop.is_some_and(|end| draw.draw_index() > end) || !is_artifact(draw.draw_index()) {
            return Err(E::ReceiptMismatch);
        }
        if let Some(ink) = source.artifacts().get(*separator) {
            if ink.before_draw() < draw.draw_index() {
                return Err(E::ReceiptMismatch);
            }
            if ink.before_draw() == draw.draw_index() {
                append(
                    out,
                    source
                        .artifact_content(*separator)
                        .ok_or(E::ReceiptMismatch)?,
                )?;
                *separator += 1;
            }
        }
        append(out, b"/Artifact BMC\n")?;
        append(
            out,
            source.draw_content(*ordinal).ok_or(E::ReceiptMismatch)?,
        )?;
        append(out, b"\nEMC\n")?;
        *ordinal += 1;
    }
    Ok(())
}

/// Marked content remains bound to the same joint structure through its fonts.
pub struct ProductionFootnoteMarkedContent<'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    content: &'c crate::ProductionFootnotePageContent<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    projection: MarkedProjection,
}
impl<'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    ProductionFootnoteMarkedContent<'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn content(
        &self,
    ) -> &'c crate::ProductionFootnotePageContent<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.content
    }
    pub fn structure(
        &self,
    ) -> &'t typaxis_display_list::ProductionFootnoteStructure<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    {
        self.content.plans().fonts().structure()
    }
    pub fn pages(&self) -> &[ProductionBodyMarkedPage] {
        &self.projection.pages
    }
    pub fn anchors(&self) -> &[ProductionBodySemanticAnchor] {
        &self.projection.anchors
    }
    pub fn record_charge(&self) -> u64 {
        self.projection.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.projection.spool_charge
    }
    pub fn verify(
        &self,
        content: &crate::ProductionFootnotePageContent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyMarkedError> {
        if !std::ptr::eq(self.content, content) {
            return Err(ProductionBodyMarkedError::ReceiptMismatch);
        }
        content
            .verify(content.plans().fonts(), admitted, limits)
            .map_err(|_| ProductionBodyMarkedError::ReceiptMismatch)
    }
}
pub fn build_production_footnote_marked_content<'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    content: &'c crate::ProductionFootnotePageContent<'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<
    ProductionFootnoteMarkedContent<'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    ProductionBodyMarkedError,
> {
    content
        .verify(content.plans().fonts(), admitted, limits)
        .map_err(|_| ProductionBodyMarkedError::ReceiptMismatch)?;
    let structure = content.plans().fonts().structure();
    let display = structure.display();
    // Structure precedes fonts on this path; its charges are already retained
    // in page content. Do not add the ordinary-body parallel-branch merge.
    let projection = project_marked_pages(
        display.draws(),
        |index| {
            display
                .table_draw_role(index)
                .is_some_and(|role| role.repeated_header())
        },
        content.pages(),
        display
            .source()
            .block_layout()
            .page_geometry()
            .page_height()
            .get()
            .raw(),
        structure.registry(),
        structure.groups().len(),
        |p| structure.page_groups(p),
        |i| structure.group_actual_text(i),
        content.text().paints(),
        |i| content.text().paint_commands(i),
        |i| content.rasters().draw_plan(i),
        content.record_charge(),
        content.spool_charge(),
        limits,
    )?;
    Ok(ProductionFootnoteMarkedContent {
        content,
        projection,
    })
}
