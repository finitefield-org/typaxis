//! Selected body PDF objects with typed, unresolved indirect references.
//! The final document owner must merge pages, catalog, metadata and navigation
//! before assigning absolute numbers or serializing a PDF.
use crate::{ProductionBodyMarkedContent, ProductionBodyPageDrawSource};
use std::collections::BTreeSet;
use typaxis_core::{FontInstanceId, M4EffectiveResourceLimits};
use typaxis_display_list::{StructureNodeId, StructureRole};
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_resources::{FrozenPdfFontPlan, PdfFontProgramKind};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProductionBodyFontObjectPart {
    Type0,
    CidFont,
    Descriptor,
    Program,
    ToUnicode,
    Auxiliary,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProductionBodyObjectRole {
    SemanticAnchorFont,
    SemanticAnchorGlyph,
    SemanticAnchorToUnicode,
    Font {
        instance: FontInstanceId,
        part: ProductionBodyFontObjectPart,
    },
    Vector(u32),
    PageContent(u32),
    PageResources(u32),
    /// Owned by the final page tree, not by this contribution.
    Page(u32),
    StructureRoot,
    ParentTree,
    IdTree,
    StructureNode(StructureNodeId),
}
#[derive(Debug, Eq, PartialEq)]
pub enum ProductionBodyObjectChunk {
    Bytes(Vec<u8>),
    Reference(ProductionBodyObjectRole),
}
#[derive(Debug, Eq, PartialEq)]
pub struct ProductionBodyObject {
    role: ProductionBodyObjectRole,
    chunks: Vec<ProductionBodyObjectChunk>,
}
impl ProductionBodyObject {
    pub const fn role(&self) -> ProductionBodyObjectRole {
        self.role
    }
    pub fn chunks(&self) -> &[ProductionBodyObjectChunk] {
        &self.chunks
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyObjectError {
    ReceiptMismatch,
    PendingNavigation,
    InvalidFont,
    InvalidStructure,
    ObjectLimit,
    RecordLimit,
    SpoolLimit,
    OutputLimit,
    AllocationFailure,
}

pub struct ProductionBodyObjectContribution<'m, 'c, 'f, 'v, 'd, 's, 'p, 'a> {
    marked: &'m ProductionBodyMarkedContent<'c, 'f, 'v, 'd, 's, 'p, 'a>,
    objects: Vec<ProductionBodyObject>,
    record_charge: u64,
    spool_charge: u64,
}
impl<'m, 'c, 'f, 'v, 'd, 's, 'p, 'a>
    ProductionBodyObjectContribution<'m, 'c, 'f, 'v, 'd, 's, 'p, 'a>
{
    pub const fn marked(&self) -> &'m ProductionBodyMarkedContent<'c, 'f, 'v, 'd, 's, 'p, 'a> {
        self.marked
    }
    pub fn objects(&self) -> &[ProductionBodyObject] {
        &self.objects
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub fn verify(
        &self,
        marked: &ProductionBodyMarkedContent<'_, '_, '_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyObjectError> {
        if !std::ptr::eq(self.marked, marked) {
            return Err(ProductionBodyObjectError::ReceiptMismatch);
        }
        marked
            .verify(marked.content(), marked.structure(), admitted, limits)
            .map_err(|_| ProductionBodyObjectError::ReceiptMismatch)
    }
}
use ProductionBodyObjectError as E;
use ProductionBodyObjectRole as R;

struct Builder<'a> {
    objects: Vec<ProductionBodyObject>,
    roles: BTreeSet<R>,
    records: u64,
    spool: u64,
    limits: &'a M4EffectiveResourceLimits,
}
impl Builder<'_> {
    fn temporary(&mut self, bytes: usize) -> Result<(), E> {
        self.spool = self.spool.checked_add(bytes as u64).ok_or(E::SpoolLimit)?;
        if self.spool > self.limits.base().get().max_spool_bytes {
            return Err(E::SpoolLimit);
        }
        Ok(())
    }
    fn font_error(&self, error: crate::PdfError) -> E {
        match error {
            crate::PdfError::OutputTooLarge
                if self.limits.base().get().max_output_bytes
                    <= self
                        .limits
                        .base()
                        .get()
                        .max_spool_bytes
                        .saturating_sub(self.spool)
                        / 2 =>
            {
                E::OutputLimit
            }
            crate::PdfError::OutputTooLarge => E::SpoolLimit,
            _ => E::InvalidFont,
        }
    }
    fn record(&mut self, n: u64) -> Result<(), E> {
        self.records = self.records.checked_add(n).ok_or(E::RecordLimit)?;
        if self.records > self.limits.base().get().max_fragments {
            return Err(E::RecordLimit);
        }
        Ok(())
    }
    fn start(&mut self, role: R) -> Result<(), E> {
        if self.objects.len() as u64 >= u64::from(self.limits.base().get().max_pdf_objects) {
            return Err(E::ObjectLimit);
        }
        self.record(2)?;
        if !self.roles.insert(role) {
            return Err(E::ReceiptMismatch);
        }
        self.objects
            .try_reserve(1)
            .map_err(|_| E::AllocationFailure)?;
        self.objects.push(ProductionBodyObject {
            role,
            chunks: Vec::new(),
        });
        Ok(())
    }
    fn bytes(&mut self, bytes: impl AsRef<[u8]>) -> Result<(), E> {
        let bytes = bytes.as_ref();
        self.spool = self
            .spool
            .checked_add(bytes.len() as u64)
            .ok_or(E::SpoolLimit)?;
        if self.spool > self.limits.base().get().max_spool_bytes {
            return Err(E::SpoolLimit);
        }
        let needs_chunk = !self
            .objects
            .last()
            .and_then(|o| o.chunks.last())
            .is_some_and(|c| matches!(c, ProductionBodyObjectChunk::Bytes(_)));
        if needs_chunk {
            self.record(1)?;
            let chunks = &mut self.objects.last_mut().ok_or(E::ReceiptMismatch)?.chunks;
            chunks.try_reserve(1).map_err(|_| E::AllocationFailure)?;
            chunks.push(ProductionBodyObjectChunk::Bytes(Vec::new()));
        }
        let Some(ProductionBodyObjectChunk::Bytes(out)) =
            self.objects.last_mut().and_then(|o| o.chunks.last_mut())
        else {
            return Err(E::ReceiptMismatch);
        };
        out.try_reserve(bytes.len())
            .map_err(|_| E::AllocationFailure)?;
        out.extend_from_slice(bytes);
        Ok(())
    }
    fn reference(&mut self, role: R) -> Result<(), E> {
        self.record(1)?;
        let chunks = &mut self.objects.last_mut().ok_or(E::ReceiptMismatch)?.chunks;
        chunks.try_reserve(1).map_err(|_| E::AllocationFailure)?;
        chunks.push(ProductionBodyObjectChunk::Reference(role));
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<(), E> {
        self.bytes("<FEFF")?;
        for unit in text.encode_utf16() {
            self.bytes(format!("{unit:04X}"))?;
        }
        self.bytes(">")
    }
    fn stream(&mut self, prefix: &str, content: &[u8]) -> Result<(), E> {
        self.bytes(format!("<< {prefix}/Length {} >>\nstream\n", content.len()))?;
        self.bytes(content)?;
        self.bytes("\nendstream")
    }
    fn temporary_maximum(&self) -> u64 {
        self.limits.base().get().max_output_bytes.min(
            self.limits
                .base()
                .get()
                .max_spool_bytes
                .saturating_sub(self.spool)
                / 2,
        )
    }
}

pub fn build_production_body_objects<'m, 'c, 'f, 'v, 'd, 's, 'p, 'a>(
    marked: &'m ProductionBodyMarkedContent<'c, 'f, 'v, 'd, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodyObjectContribution<'m, 'c, 'f, 'v, 'd, 's, 'p, 'a>, E> {
    marked
        .verify(marked.content(), marked.structure(), admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    // Link StructElems need selected annotation OBJRs. Until the navigation
    // owner is connected, do not issue a seemingly complete but missing /K.
    if marked
        .structure()
        .registry()
        .nodes()
        .iter()
        .any(|n| n.role() == StructureRole::Link)
    {
        return Err(E::PendingNavigation);
    }
    let mut b = Builder {
        objects: Vec::new(),
        roles: BTreeSet::new(),
        records: marked.record_charge(),
        spool: marked.spool_charge(),
        limits,
    };
    for font in marked.content().plans().fonts().fonts() {
        font_objects(&mut b, font.pdf_font())?;
    }
    if !marked.anchors().is_empty() {
        b.start(R::SemanticAnchorFont)?;
        b.bytes("<< /Type /Font /Subtype /Type3 /Name /PBA /FontBBox [0 0 1000 1000] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /anchor ")?;
        b.reference(R::SemanticAnchorGlyph)?;
        b.bytes(" >> /Encoding << /Type /Encoding /Differences [0 /anchor] >> /FirstChar 0 /LastChar 0 /Widths [1000] /Resources << >> /ToUnicode ")?;
        b.reference(R::SemanticAnchorToUnicode)?;
        b.bytes(" >>")?;
        b.start(R::SemanticAnchorGlyph)?;
        // A declared bounding box/advance with no paint operator. Tr=3 gives a
        // second, independent guarantee that this usage cannot add visible ink.
        b.stream("", b"1000 0 0 0 1000 1000 d1\n")?;
        b.start(R::SemanticAnchorToUnicode)?;
        b.stream("", b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Typaxis) /Ordering (SemanticAnchor) /Supplement 0 >> def\n/CMapName /TypaxisSemanticAnchor def\n/CMapType 2 def\n1 begincodespacerange\n<00> <00>\nendcodespacerange\n1 beginbfchar\n<00> <FFFC>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n")?;
    }
    for ext in marked.content().vectors().ext_g_states() {
        b.start(R::Vector(ext.relative_object_role()))?;
        b.bytes(ext.dictionary())?;
    }
    for form in marked.content().vectors().forms() {
        b.start(R::Vector(form.relative_object_role()))?;
        let bbox = form.bbox();
        b.bytes(format!("<< /Type /XObject /Subtype /Form /FormType 1 /BBox [{} {} {} {}] /Resources << /ExtGState <<",
            number(bbox[0]), number(bbox[1]), number(bbox[2]), number(bbox[3])))?;
        for (name, role) in form.ext_g_state_roles() {
            b.bytes(format!(" /{name} "))?;
            b.reference(R::Vector(*role))?;
        }
        b.bytes(format!(
            " >> >> /Length {} >>\nstream\n",
            form.content_stream().len()
        ))?;
        b.bytes(form.content_stream())?;
        b.bytes("\nendstream")?;
    }
    let mut anchor_cursor = marked.anchors().iter().peekable();
    for page in marked.pages() {
        b.start(R::PageContent(page.page_index()))?;
        b.stream("", page.content())?;
        b.start(R::PageResources(page.page_index()))?;
        b.bytes("<< /Font <<")?;
        if anchor_cursor
            .peek()
            .is_some_and(|a| a.page_index() == page.page_index())
        {
            b.bytes(" /PBA ")?;
            b.reference(R::SemanticAnchorFont)?;
            while anchor_cursor
                .peek()
                .is_some_and(|a| a.page_index() == page.page_index())
            {
                anchor_cursor.next();
            }
        }
        let mut fonts = BTreeSet::new();
        for draw in marked.content().pages()[page.page_index() as usize].draws() {
            if let ProductionBodyPageDrawSource::Text { paint_index } = draw.source() {
                let id = marked.content().text().paints()[paint_index].font_instance_id();
                if !fonts.contains(&id) {
                    b.record(1)?;
                    fonts.insert(id);
                    b.bytes(format!(" /PB{} ", id.get()))?;
                    b.reference(R::Font {
                        instance: id,
                        part: ProductionBodyFontObjectPart::Type0,
                    })?;
                }
            }
        }
        b.bytes(" >> /XObject <<")?;
        for resource in marked.content().vectors().pages()[page.page_index() as usize].resources() {
            b.bytes(format!(" /{} ", resource.resource_name()))?;
            b.reference(R::Vector(resource.form_relative_object_role()))?;
        }
        b.bytes(" >> >>")?;
    }
    structure_objects(&mut b, marked)?;
    for object in &b.objects {
        for chunk in &object.chunks {
            if let ProductionBodyObjectChunk::Reference(role) = chunk {
                match role {
                    R::Page(index) if (*index as usize) < marked.pages().len() => {}
                    _ if b.roles.contains(role) => {}
                    _ => return Err(E::ReceiptMismatch),
                }
            }
        }
    }
    Ok(ProductionBodyObjectContribution {
        marked,
        objects: b.objects,
        record_charge: b.records,
        spool_charge: b.spool,
    })
}
fn number(raw: i64) -> String {
    crate::tagged_pdf_v2::pdf_number_v2(raw)
}

fn font_objects(b: &mut Builder<'_>, font: &FrozenPdfFontPlan) -> Result<(), E> {
    use ProductionBodyFontObjectPart as P;
    let role = |part| R::Font {
        instance: font.font_instance_id(),
        part,
    };
    let name = font.embedded_postscript_name();
    if name.is_empty()
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-'))
    {
        return Err(E::InvalidFont);
    }
    b.start(role(P::Type0))?;
    b.bytes(format!(
        "<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding /Identity-H /DescendantFonts ["
    ))?;
    b.reference(role(P::CidFont))?;
    b.bytes("] /ToUnicode ")?;
    b.reference(role(P::ToUnicode))?;
    b.bytes(" >>")?;
    b.start(role(P::CidFont))?;
    let cff = match font.program_kind() {
        PdfFontProgramKind::TrueTypeGlyf => false,
        PdfFontProgramKind::OpenTypeCff1 => true,
    };
    b.bytes(format!("<< /Type /Font /Subtype /{} /BaseFont /{name} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor ", if cff { "CIDFontType0" } else { "CIDFontType2" }))?;
    b.reference(role(P::Descriptor))?;
    b.bytes(" /DW 1000 /W [")?;
    if cff {
        b.bytes("0 [")?;
        for width in font.cff1_plan().ok_or(E::InvalidFont)?.dense_widths_1000() {
            b.bytes(format!("{width} "))?;
        }
        b.bytes("]")?;
    } else {
        for binding in &font.subset_plan().cids {
            b.bytes(format!("{} [{}] ", binding.cid.get(), binding.width_1000))?;
        }
    }
    b.bytes("]")?;
    if !cff {
        b.bytes(" /CIDToGIDMap ")?;
        b.reference(role(P::Auxiliary))?;
    }
    b.bytes(" >>")?;
    b.start(role(P::Descriptor))?;
    let m = font.metrics();
    b.bytes(format!("<< /Type /FontDescriptor /FontName /{name} /Flags {} /FontBBox [{} {} {} {}] /ItalicAngle {} /Ascent {} /Descent {} /CapHeight {} /StemV {} /{} ",
        m.flags, m.bbox_1000[0], m.bbox_1000[1], m.bbox_1000[2], m.bbox_1000[3], number(i64::from(m.italic_angle_fixed_16_16)),
        m.ascent_1000, m.descent_1000, m.cap_height_1000, m.stem_v_1000, if cff { "FontFile3" } else { "FontFile2" }))?;
    b.reference(role(P::Program))?;
    if cff {
        b.bytes(" /CIDSet ")?;
        b.reference(role(P::Auxiliary))?;
    }
    b.bytes(" >>")?;
    b.start(role(P::Program))?;
    b.stream(
        &if cff {
            "/Subtype /OpenType ".to_owned()
        } else {
            format!("/Length1 {} ", font.subset_bytes().len())
        },
        font.subset_bytes(),
    )?;
    b.start(role(P::ToUnicode))?;
    let unicode =
        crate::to_unicode_cmap(font, b.temporary_maximum()).map_err(|e| b.font_error(e))?;
    b.temporary(unicode.len())?;
    b.stream("", &unicode)?;
    b.start(role(P::Auxiliary))?;
    let auxiliary = if cff {
        crate::cid_set(font, b.temporary_maximum())
    } else {
        crate::cid_to_gid_map(font, b.temporary_maximum())
    }
    .map_err(|e| b.font_error(e))?;
    b.temporary(auxiliary.len())?;
    b.stream("", &auxiliary)
}

fn structure_objects(
    b: &mut Builder<'_>,
    marked: &ProductionBodyMarkedContent<'_, '_, '_, '_, '_, '_, '_>,
) -> Result<(), E> {
    let structure = marked.structure();
    let registry = structure.registry();
    let has_ids = registry.nodes().iter().any(|n| n.structure_id().is_some());
    b.start(R::StructureRoot)?;
    b.bytes("<< /Type /StructTreeRoot /RoleMap << /Em /Span /Exercise /Div /Proof /Div /Result /Div /Strong /Span >> /ParentTree ")?;
    b.reference(R::ParentTree)?;
    b.bytes(format!(" /ParentTreeNextKey {} /K [", marked.pages().len()))?;
    for node in registry.nodes().iter().filter(|n| n.parent().is_none()) {
        b.reference(R::StructureNode(node.structure_node_id()))?;
        b.bytes(" ")?;
    }
    b.bytes("]")?;
    if has_ids {
        b.bytes(" /IDTree ")?;
        b.reference(R::IdTree)?;
    }
    b.bytes(" >>")?;
    b.start(R::ParentTree)?;
    b.bytes("<< /Nums [")?;
    for page in marked.pages() {
        b.bytes(format!("{} [", page.page_index()))?;
        for group in structure
            .page_groups(page.page_index())
            .ok_or(E::InvalidStructure)?
        {
            b.reference(R::StructureNode(group.node()))?;
            b.bytes(" ")?;
        }
        b.bytes("] ")?;
    }
    b.bytes("] >>")?;
    if has_ids {
        let count = registry
            .nodes()
            .iter()
            .filter(|n| n.structure_id().is_some())
            .count();
        b.record(count as u64)?;
        let mut ids = Vec::new();
        ids.try_reserve_exact(count)
            .map_err(|_| E::AllocationFailure)?;
        ids.extend(
            registry
                .nodes()
                .iter()
                .filter_map(|n| n.structure_id().map(|id| (id, n.structure_node_id()))),
        );
        ids.sort_unstable_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        if ids.windows(2).any(|w| w[0].0 == w[1].0) {
            return Err(E::InvalidStructure);
        }
        b.start(R::IdTree)?;
        b.bytes("<< /Names [")?;
        for (id, node) in ids {
            b.text(id)?;
            b.bytes(" ")?;
            b.reference(R::StructureNode(node))?;
            b.bytes(" ")?;
        }
        b.bytes("] >>")?;
    }
    for node in registry.nodes() {
        b.start(R::StructureNode(node.structure_node_id()))?;
        b.bytes(format!(
            "<< /Type /StructElem /S /{} /P ",
            node.role().pdf_name()
        ))?;
        b.reference(node.parent().map_or(R::StructureRoot, R::StructureNode))?;
        b.bytes(" /Lang ")?;
        b.text(node.language())?;
        if let Some(alt) = node.alternative() {
            b.bytes(" /Alt ")?;
            b.text(alt)?;
        }
        if let Some(id) = node.structure_id() {
            b.bytes(" /ID ")?;
            b.text(id)?;
        }
        if let Some(numbering) = node.list_numbering() {
            b.bytes(format!(
                " /A << /O /List /ListNumbering /{} >>",
                numbering.pdf_name()
            ))?;
        } else if let Some(table) = node.table_attributes() {
            b.bytes(" /A << /O /Table")?;
            if node.role() == StructureRole::TableHeader {
                b.bytes(" /Scope /Column")?;
            }
            if table.rowspan() > 1 {
                b.bytes(format!(" /RowSpan {}", table.rowspan()))?;
            }
            if table.colspan() > 1 {
                b.bytes(format!(" /ColSpan {}", table.colspan()))?;
            }
            if !table.header_ids().is_empty() {
                b.bytes(" /Headers [")?;
                for id in table.header_ids() {
                    b.text(id)?;
                    b.bytes(" ")?;
                }
                b.bytes("]")?;
            }
            b.bytes(" >>")?;
        }
        b.bytes(" /K [")?;
        for &index in structure
            .node_groups(node.structure_node_id())
            .ok_or(E::InvalidStructure)?
        {
            let group = &structure.groups()[index];
            b.bytes("<< /Type /MCR /Pg ")?;
            b.reference(R::Page(group.page_index()))?;
            b.bytes(format!(" /MCID {} >> ", group.mcid()))?;
        }
        for &child in node.children() {
            b.reference(R::StructureNode(child))?;
            b.bytes(" ")?;
        }
        b.bytes("] >>")?;
    }
    Ok(())
}
