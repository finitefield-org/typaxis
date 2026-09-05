//! Resolves the selected body graph into inspectable PDF bytes. This assembly
//! is not a VerifiedPdfBytesReceipt and cannot authorize publication. The final
//! production terminal/navigation/manifest closure remains a separate boundary.
use crate::{
    ProductionBodyObjectChunk, ProductionBodyObjectContribution, ProductionBodyObjectRole,
};
use std::collections::BTreeMap;
use typaxis_core::{sha256, EngineIdentity, M4EffectiveResourceLimits};
use typaxis_resource_admission::AdmittedResourceLedger;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyAssemblyError {
    ReceiptMismatch,
    PendingNavigation,
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
    if !navigation.anchors().is_empty()
        || !navigation.internal_links().is_empty()
        || !navigation.outline().entries().is_empty()
    {
        return Err(E::PendingNavigation);
    }
    let page_count = u32::try_from(marked.pages().len()).map_err(|_| E::ObjectLimit)?;
    let body_count = u32::try_from(source.objects().len()).map_err(|_| E::ObjectLimit)?;
    let count = body_count
        .checked_add(page_count)
        .and_then(|n| n.checked_add(4))
        .ok_or(E::ObjectLimit)?;
    // Count the complete graph before assigning any absolute object number.
    if count > limits.base().get().max_pdf_objects || count == u32::MAX {
        return Err(E::ObjectLimit);
    }
    let record_charge = source
        .record_charge()
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
    for (index, object) in source.objects().iter().enumerate() {
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
        spool: source.spool_charge(),
    };
    let mut graph: Vec<(A, Vec<u8>)> = Vec::new();
    graph
        .try_reserve_exact(count as usize)
        .map_err(|_| E::AllocationFailure)?;
    let mut catalog = Vec::new();
    budget.append(&mut catalog, format!("<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R /Lang <FEFF{}> /MarkInfo << /Marked true >> /ViewerPreferences << /DisplayDocTitle true >> /StructTreeRoot {} >>",
        utf16(navigation.languages().document_language()), reference(R::StructureRoot)?))?;
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
    let geometry = display.selected().page_geometry();
    for page in 0..page_count {
        let mut bytes = Vec::new();
        budget.append(&mut bytes, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources {} /Contents {} /StructParents {page} /Tabs /S >>",
            number(geometry.page_width().get().raw()), number(geometry.page_height().get().raw()),
            reference(R::PageResources(page))?, reference(R::PageContent(page))?))?;
        graph.push((A::Body(R::Page(page)), bytes));
    }
    for object in source.objects() {
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
    Ok(ProductionBodyPdfAssembly {
        source,
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
