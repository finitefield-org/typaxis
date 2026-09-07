//! Resolves the selected body graph into inspectable PDF bytes. This assembly
//! is not a VerifiedPdfBytesReceipt and cannot authorize publication. The final
//! production terminal/manifest closure remains a separate boundary.
use crate::{
    ProductionBodyObjectChunk, ProductionBodyObjectContribution, ProductionBodyObjectRole,
};
use std::collections::BTreeMap;
use typaxis_core::{sha256, EngineIdentity, M4EffectiveResourceLimits};
use typaxis_resource_admission::AdmittedResourceLedger;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyAssemblyError {
    ReceiptMismatch,
    ObjectLimit,
    RecordLimit,
    SpoolLimit,
    OutputLimit,
    AllocationFailure,
    Metadata,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyAssemblyRole {
    Catalog,
    Pages,
    Info,
    Metadata,
    Body(ProductionBodyObjectRole),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyAssemblyObject {
    number: u32,
    role: ProductionBodyAssemblyRole,
    sha256: [u8; 32],
    byte_length: u64,
    offset: u64,
}
impl ProductionBodyAssemblyObject {
    pub const fn number(self) -> u32 {
        self.number
    }
    pub const fn role(self) -> ProductionBodyAssemblyRole {
        self.role
    }
    pub const fn sha256(self) -> [u8; 32] {
        self.sha256
    }
    pub const fn byte_length(self) -> u64 {
        self.byte_length
    }
    pub const fn offset(self) -> u64 {
        self.offset
    }
}

pub struct ProductionBodyPdfAssembly<'o, 'm, 'c, 'f, 'v, 'd, 's, 'p, 'a> {
    source: &'o ProductionBodyObjectContribution<'m, 'c, 'f, 'v, 'd, 's, 'p, 'a>,
    numbers: BTreeMap<ProductionBodyObjectRole, u32>,
    observations: Vec<ProductionBodyAssemblyObject>,
    bytes: Vec<u8>,
    hash: [u8; 32],
    page_count: u32,
    record_charge: u64,
    spool_charge: u64,
}
impl ProductionBodyPdfAssembly<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.hash
    }
    pub fn object_bytes(&self, number: u32) -> Option<&[u8]> {
        assembled_object_bytes(&self.bytes, &self.observations, number)
    }
    pub fn objects(&self) -> &[ProductionBodyAssemblyObject] {
        &self.observations
    }
    pub fn object_number(&self, role: ProductionBodyObjectRole) -> Option<u32> {
        self.numbers.get(&role).copied()
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub fn verify(
        &self,
        source: &ProductionBodyObjectContribution<'_, '_, '_, '_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyAssemblyError> {
        if !std::ptr::eq(self.source, source) {
            return Err(ProductionBodyAssemblyError::ReceiptMismatch);
        }
        source
            .verify(source.marked(), admitted, limits)
            .map_err(|_| ProductionBodyAssemblyError::ReceiptMismatch)
    }
}
use ProductionBodyAssemblyError as E;
use ProductionBodyAssemblyRole as A;
use ProductionBodyObjectRole as R;
struct Budget<'a> {
    limits: &'a M4EffectiveResourceLimits,
    spool: u64,
}
impl Budget<'_> {
    fn temporary(&mut self, len: usize) -> Result<(), E> {
        self.spool = self.spool.checked_add(len as u64).ok_or(E::SpoolLimit)?;
        if self.spool > self.limits.base().get().max_spool_bytes {
            return Err(E::SpoolLimit);
        }
        Ok(())
    }
    fn append(&mut self, out: &mut Vec<u8>, bytes: impl AsRef<[u8]>) -> Result<(), E> {
        let bytes = bytes.as_ref();
        self.temporary(bytes.len())?;
        let next = (out.len() as u64)
            .checked_add(bytes.len() as u64)
            .ok_or(E::OutputLimit)?;
        if next > self.limits.base().get().max_output_bytes {
            return Err(E::OutputLimit);
        }
        out.try_reserve(bytes.len())
            .map_err(|_| E::AllocationFailure)?;
        out.extend_from_slice(bytes);
        Ok(())
    }
}

pub fn assemble_production_body_pdf<'o, 'm, 'c, 'f, 'v, 'd, 's, 'p, 'a>(
    source: &'o ProductionBodyObjectContribution<'m, 'c, 'f, 'v, 'd, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodyPdfAssembly<'o, 'm, 'c, 'f, 'v, 'd, 's, 'p, 'a>, E> {
    source
        .verify(source.marked(), admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    let marked = source.marked();
    let display = marked.structure().display();
    let navigation = display.selected().line_layout().source_flow().navigation();
    source
        .navigation()
        .verify(marked.structure())
        .map_err(|_| E::ReceiptMismatch)?;
    let projected = project_pdf_assembly(
        source.objects().iter(),
        source.objects().len(),
        marked.pages().len(),
        display.selected().page_geometry().page_width().get().raw(),
        display.selected().page_geometry().page_height().get().raw(),
        navigation,
        !source.navigation().destinations().is_empty(),
        !source.navigation().outline().is_empty(),
        |p| source.navigation().page_links(p),
        source.record_charge(),
        source.spool_charge(),
        limits,
    )?;
    Ok(ProductionBodyPdfAssembly {
        source,
        numbers: projected.numbers,
        observations: projected.observations,
        bytes: projected.bytes,
        hash: projected.hash,
        page_count: projected.page_count,
        record_charge: projected.record_charge,
        spool_charge: projected.spool_charge,
    })
}
struct AssemblyProjection {
    numbers: BTreeMap<ProductionBodyObjectRole, u32>,
    observations: Vec<ProductionBodyAssemblyObject>,
    bytes: Vec<u8>,
    hash: [u8; 32],
    page_count: u32,
    record_charge: u64,
    spool_charge: u64,
}
fn project_pdf_assembly<'s>(
    objects: impl Iterator<Item = &'s crate::ProductionBodyObject> + Clone,
    object_count: usize,
    pages: usize,
    page_width: i64,
    page_height: i64,
    navigation: &typaxis_syntax::ValidatedStagingBookNavigationV2,
    has_destinations: bool,
    has_outline: bool,
    page_links: impl Fn(u32) -> Option<std::ops::Range<usize>>,
    record_base: u64,
    spool_base: u64,
    limits: &M4EffectiveResourceLimits,
) -> Result<AssemblyProjection, E> {
    let page_count = u32::try_from(pages).map_err(|_| E::ObjectLimit)?;
    let body_count = u32::try_from(object_count).map_err(|_| E::ObjectLimit)?;
    let count = body_count
        .checked_add(page_count)
        .and_then(|n| n.checked_add(4))
        .ok_or(E::ObjectLimit)?;
    // Count the complete graph before assigning any absolute object number.
    if count > limits.base().get().max_pdf_objects || count == u32::MAX {
        return Err(E::ObjectLimit);
    }
    let record_charge = record_base
        .checked_add(u64::from(count).checked_mul(5).ok_or(E::RecordLimit)?)
        .and_then(|v| v.checked_add(1))
        .ok_or(E::RecordLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    let mut numbers = BTreeMap::new();
    for page in 0..page_count {
        numbers.insert(R::Page(page), 5 + page);
    }
    for (index, object) in objects.clone().enumerate() {
        if index >= object_count {
            return Err(E::ReceiptMismatch);
        }
        let number = 5 + page_count + u32::try_from(index).map_err(|_| E::ObjectLimit)?;
        if numbers.insert(object.role(), number).is_some() {
            return Err(E::ReceiptMismatch);
        }
    }
    let reference = |role| -> Result<String, E> {
        Ok(format!(
            "{} 0 R",
            numbers.get(&role).ok_or(E::ReceiptMismatch)?
        ))
    };
    let mut budget = Budget {
        limits,
        spool: spool_base,
    };
    let mut graph: Vec<(A, Vec<u8>)> = Vec::new();
    graph
        .try_reserve_exact(count as usize)
        .map_err(|_| E::AllocationFailure)?;
    let mut catalog = Vec::new();
    budget.append(&mut catalog, format!("<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R /Lang <FEFF{}> /MarkInfo << /Marked true >> /ViewerPreferences << /DisplayDocTitle true >> /StructTreeRoot {}",
        utf16(navigation.languages().document_language()), reference(R::StructureRoot)?))?;
    if has_destinations {
        budget.append(
            &mut catalog,
            format!(" /Names << /Dests {} >>", reference(R::Destinations)?),
        )?;
    }
    if has_outline {
        budget.append(
            &mut catalog,
            format!(" /Outlines {}", reference(R::Outlines)?),
        )?;
    }
    budget.append(&mut catalog, " >>")?;
    graph.push((A::Catalog, catalog));
    let mut pages = Vec::new();
    budget.append(
        &mut pages,
        format!("<< /Type /Pages /Count {page_count} /Kids ["),
    )?;
    for page in 0..page_count {
        budget.append(&mut pages, format!("{} ", reference(R::Page(page))?))?;
    }
    budget.append(&mut pages, "] >>")?;
    graph.push((A::Pages, pages));
    let engine = EngineIdentity::compiled();
    let info =
        crate::tagged_pdf_v2::encode_info_v2(navigation, &engine).map_err(|_| E::Metadata)?;
    budget.temporary(info.len())?;
    let mut info_object = Vec::new();
    budget.append(&mut info_object, info.as_bytes())?;
    graph.push((A::Info, info_object));
    let xmp = crate::tagged_pdf::encode_book_xmp_with_conformance(
        navigation.metadata(),
        navigation.languages().document_language(),
        &engine,
        false,
    );
    budget.temporary(xmp.len())?;
    let mut metadata = Vec::new();
    budget.append(
        &mut metadata,
        format!(
            "<< /Type /Metadata /Subtype /XML /Length {} >>\nstream\n",
            xmp.len()
        ),
    )?;
    budget.append(&mut metadata, xmp.as_bytes())?;
    budget.append(&mut metadata, "\nendstream")?;
    graph.push((A::Metadata, metadata));
    for page in 0..page_count {
        let mut bytes = Vec::new();
        budget.append(&mut bytes, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources {} /Contents {} /StructParents {page} /Tabs /S",
            number(page_width), number(page_height),
            reference(R::PageResources(page))?, reference(R::PageContent(page))?))?;
        let links = page_links(page).ok_or(E::ReceiptMismatch)?;
        if !links.is_empty() {
            budget.append(&mut bytes, " /Annots [")?;
            for index in links {
                let index = u32::try_from(index).map_err(|_| E::ObjectLimit)?;
                budget.append(
                    &mut bytes,
                    format!("{} ", reference(R::LinkAnnotation(index))?),
                )?;
            }
            budget.append(&mut bytes, "]")?;
        }
        budget.append(&mut bytes, " >>")?;
        graph.push((A::Body(R::Page(page)), bytes));
    }
    for object in objects {
        let mut bytes = Vec::new();
        for chunk in object.chunks() {
            match chunk {
                ProductionBodyObjectChunk::Bytes(raw) => budget.append(&mut bytes, raw)?,
                ProductionBodyObjectChunk::Reference(role) => {
                    budget.append(&mut bytes, reference(*role)?)?
                }
            }
        }
        graph.push((A::Body(object.role()), bytes));
    }
    if graph.len() != count as usize {
        return Err(E::ReceiptMismatch);
    }
    let mut observations = Vec::new();
    observations
        .try_reserve_exact(graph.len())
        .map_err(|_| E::AllocationFailure)?;
    let mut output = Vec::new();
    budget.append(&mut output, b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n")?;
    for (index, (role, bytes)) in graph.iter().enumerate() {
        let number = u32::try_from(index + 1).map_err(|_| E::ObjectLimit)?;
        let offset = output.len() as u64;
        // Traditional xref entries use ten decimal digits, never a wider field.
        if offset > 9_999_999_999 {
            return Err(E::OutputLimit);
        }
        budget.append(&mut output, format!("{number} 0 obj\n"))?;
        budget.append(&mut output, bytes)?;
        budget.append(&mut output, "\nendobj\n")?;
        observations.push(ProductionBodyAssemblyObject {
            number,
            role: *role,
            sha256: sha256(bytes),
            byte_length: bytes.len() as u64,
            offset,
        });
    }
    let xref = output.len();
    budget.append(
        &mut output,
        format!("xref\n0 {}\n0000000000 65535 f \n", count + 1),
    )?;
    for object in &observations {
        budget.append(&mut output, format!("{:010} 00000 n \n", object.offset))?;
    }
    budget.append(
        &mut output,
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            count + 1
        ),
    )?;
    Ok(AssemblyProjection {
        numbers,
        observations,
        hash: sha256(&output),
        bytes: output,
        page_count,
        record_charge,
        spool_charge: budget.spool,
    })
}
fn number(raw: i64) -> String {
    crate::tagged_pdf_v2::pdf_number_v2(raw)
}
fn utf16(text: &str) -> String {
    text.encode_utf16().map(|u| format!("{u:04X}")).collect()
}

/// Inspectable PDF bytes; public production receipt/manifest closure is separate.
pub struct ProductionFootnotePdfAssembly<
    'o,
    'r,
    'z,
    'm,
    'c,
    'e,
    't,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
> {
    source: &'o crate::ProductionFootnoteResourceObjects<
        'r,
        'z,
        'm,
        'c,
        'e,
        't,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >,
    vector_final_writer: crate::StagingSafeVectorPdfFinalWriterObservationV2,
    numbers: BTreeMap<ProductionBodyObjectRole, u32>,
    observations: Vec<ProductionBodyAssemblyObject>,
    bytes: Vec<u8>,
    hash: [u8; 32],
    page_count: u32,
    record_charge: u64,
    spool_charge: u64,
}
impl ProductionFootnotePdfAssembly<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn vector_final_writer(&self) -> &crate::StagingSafeVectorPdfFinalWriterObservationV2 {
        &self.vector_final_writer
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub const fn content_hash(&self) -> [u8; 32] {
        self.hash
    }
    pub fn object_bytes(&self, number: u32) -> Option<&[u8]> {
        assembled_object_bytes(&self.bytes, &self.observations, number)
    }
    pub fn objects(&self) -> &[ProductionBodyAssemblyObject] {
        &self.observations
    }
    pub fn object_number(&self, role: ProductionBodyObjectRole) -> Option<u32> {
        self.numbers.get(&role).copied()
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub fn verify(
        &self,
        source: &crate::ProductionFootnoteResourceObjects<
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyAssemblyError> {
        if !std::ptr::eq(self.source, source) {
            return Err(ProductionBodyAssemblyError::ReceiptMismatch);
        }
        source
            .verify(source.structure_objects(), admitted, limits)
            .map_err(|_| ProductionBodyAssemblyError::ReceiptMismatch)
    }
}
pub fn assemble_production_footnote_pdf<
    'o,
    'r,
    'z,
    'm,
    'c,
    'e,
    't,
    'v,
    'd,
    'g,
    'q,
    'b,
    'f,
    's,
    'p,
    'a,
>(
    source: &'o crate::ProductionFootnoteResourceObjects<
        'r,
        'z,
        'm,
        'c,
        'e,
        't,
        'v,
        'd,
        'g,
        'q,
        'b,
        'f,
        's,
        'p,
        'a,
    >,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<
    ProductionFootnotePdfAssembly<'o, 'r, 'z, 'm, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    E,
> {
    source
        .verify(source.structure_objects(), admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    let structure = source.structure_objects();
    let annotations = structure.annotations();
    let marked = annotations.marked();
    let display = marked.structure().display();
    let navigation = annotations.navigation();
    navigation
        .verify(marked.structure())
        .map_err(|_| E::ReceiptMismatch)?;
    let objects = annotations
        .objects()
        .iter()
        .chain(structure.objects())
        .chain(source.objects());
    let geometry = display.source().block_layout().page_geometry();
    let mut projected = project_pdf_assembly(
        objects,
        source.retained_object_count(),
        marked.pages().len(),
        geometry.page_width().get().raw(),
        geometry.page_height().get().raw(),
        display.source().line_layout().source_flow().navigation(),
        !navigation.destinations().is_empty(),
        !navigation.outline().is_empty(),
        |p| annotations.page_annotations(p),
        source.record_charge(),
        source.spool_charge(),
        limits,
    )?;
    let vector_final_writer =
        project_vector_final_writer(&mut projected, marked.content().vectors(), limits)?;
    Ok(ProductionFootnotePdfAssembly {
        source,
        vector_final_writer,
        numbers: projected.numbers,
        observations: projected.observations,
        bytes: projected.bytes,
        hash: projected.hash,
        page_count: projected.page_count,
        record_charge: projected.record_charge,
        spool_charge: projected.spool_charge,
    })
}

fn project_vector_final_writer(
    pdf: &mut AssemblyProjection,
    vector: &crate::StagingSafeVectorPdfContributionV2,
    limits: &M4EffectiveResourceLimits,
) -> Result<crate::StagingSafeVectorPdfFinalWriterObservationV2, E> {
    // Retained rows plus the constructor's validation maps/sets. Charge before
    // allocating either the rows or its canonical observation record.
    let rows = vector
        .relative_objects()
        .len()
        .checked_add(vector.usages().len())
        .ok_or(E::RecordLimit)?;
    pdf.record_charge = pdf
        .record_charge
        .checked_add((rows as u64).checked_mul(4).ok_or(E::RecordLimit)?)
        .and_then(|n| n.checked_add(1))
        .ok_or(E::RecordLimit)?;
    if pdf.record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    let number = |role| pdf.numbers.get(&role).copied().ok_or(E::ReceiptMismatch);
    let mut objects = Vec::new();
    objects
        .try_reserve_exact(vector.relative_objects().len())
        .map_err(|_| E::AllocationFailure)?;
    for object in vector.relative_objects() {
        objects.push(
            crate::StagingSafeVectorPdfFinalObjectObservationV2::from_final_writer(
                object.relative_object_role(),
                number(R::Vector(object.relative_object_role()))?,
                object.object_contribution_fingerprint(),
            ),
        );
    }
    let mut usages = Vec::new();
    usages
        .try_reserve_exact(vector.usages().len())
        .map_err(|_| E::AllocationFailure)?;
    for usage in vector.usages() {
        usages.push(
            crate::StagingSafeVectorPdfFinalUsageObservationV2::from_final_writer(
                usage.usage_id(),
                usage.page_index(),
                usage.paint_ordinal(),
                number(R::Page(usage.page_index()))?,
                number(R::PageContent(usage.page_index()))?,
                number(R::Vector(usage.form_relative_object_role()))?,
                usage.content_fingerprint(),
            ),
        );
    }
    let available = limits
        .base()
        .get()
        .max_spool_bytes
        .checked_sub(pdf.spool_charge)
        .ok_or(E::SpoolLimit)?;
    let result = crate::StagingSafeVectorPdfFinalWriterObservationV2::from_final_writer_bounded(
        vector, objects, usages, available,
    )
    .map_err(|error| match error {
        crate::StagingSafeVectorPdfV2Error::SpoolLimit => E::SpoolLimit,
        crate::StagingSafeVectorPdfV2Error::AllocationFailure => E::AllocationFailure,
        _ => E::ReceiptMismatch,
    })?;
    pdf.spool_charge = pdf
        .spool_charge
        .checked_add(result.canonical_jcs().len() as u64)
        .ok_or(E::SpoolLimit)?;
    Ok(result)
}

fn assembled_object_bytes<'a>(
    bytes: &'a [u8],
    observations: &[ProductionBodyAssemblyObject],
    number: u32,
) -> Option<&'a [u8]> {
    let index = usize::try_from(number.checked_sub(1)?).ok()?;
    let object = observations.get(index)?;
    if object.number != number {
        return None;
    }
    let start = usize::try_from(object.offset)
        .ok()?
        .checked_add(number.ilog10() as usize + 8)?;
    let end = start.checked_add(usize::try_from(object.byte_length).ok()?)?;
    bytes.get(start..end)
}

#[path = "production_book_observation.rs"]
mod production_book_observation;
pub use production_book_observation::{
    observe_production_footnote_book_pdf, ProductionBookPdfObservation,
};
