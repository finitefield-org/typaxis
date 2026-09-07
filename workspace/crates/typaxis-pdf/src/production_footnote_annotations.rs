//! Joint annotation objects; final page/structure owners consume the bindings.
use super::*;
use crate::ProductionFootnoteMarkedContent;
use typaxis_display_list::{build_production_footnote_navigation, ProductionFootnoteNavigation};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ProductionFootnoteAnnotationSource {
    Ordinary(usize),
    Footnote(usize),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionFootnoteAnnotationBinding {
    source: ProductionFootnoteAnnotationSource,
    node: StructureNodeId,
    page: u32,
    parent_key: u32,
}
impl ProductionFootnoteAnnotationBinding {
    pub fn source(&self) -> ProductionFootnoteAnnotationSource {
        self.source
    }
    pub fn node(&self) -> StructureNodeId {
        self.node
    }
    pub fn page_index(&self) -> u32 {
        self.page
    }
    pub fn parent_key(&self) -> u32 {
        self.parent_key
    }
}
pub struct ProductionFootnoteAnnotations<'m, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    marked: &'m ProductionFootnoteMarkedContent<'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    navigation: ProductionFootnoteNavigation<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    objects: Vec<ProductionBodyObject>,
    bindings: Vec<ProductionFootnoteAnnotationBinding>,
    node_annotations: Vec<Vec<u32>>,
    page_annotations: Vec<std::ops::Range<usize>>,
    records: u64,
    spool: u64,
}
impl<'m, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    ProductionFootnoteAnnotations<'m, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn marked(
        &self,
    ) -> &'m ProductionFootnoteMarkedContent<'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.marked
    }
    pub fn navigation(
        &self,
    ) -> &ProductionFootnoteNavigation<'t, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.navigation
    }
    pub fn objects(&self) -> &[ProductionBodyObject] {
        &self.objects
    }
    pub fn bindings(&self) -> &[ProductionFootnoteAnnotationBinding] {
        &self.bindings
    }
    pub fn node_annotations(&self, node: StructureNodeId) -> Option<&[u32]> {
        self.node_annotations
            .get(node.get() as usize)
            .map(Vec::as_slice)
    }
    pub fn page_annotations(&self, page: u32) -> Option<std::ops::Range<usize>> {
        self.page_annotations.get(page as usize).cloned()
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool
    }
    pub fn verify(
        &self,
        marked: &ProductionFootnoteMarkedContent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), E> {
        if !std::ptr::eq(self.marked, marked) {
            return Err(E::ReceiptMismatch);
        }
        marked
            .verify(marked.content(), admitted, limits)
            .map_err(|_| E::ReceiptMismatch)?;
        self.navigation
            .verify(marked.structure())
            .map_err(E::Navigation)
    }
}
pub fn build_production_footnote_annotations<'m, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    marked: &'m ProductionFootnoteMarkedContent<'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionFootnoteAnnotations<'m, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>, E> {
    marked
        .verify(marked.content(), admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    let navigation = build_production_footnote_navigation(
        marked.structure(),
        admitted,
        limits,
        marked.record_charge(),
    )
    .map_err(E::Navigation)?;
    let mut b = Builder {
        object_base: 0,
        objects: Vec::new(),
        roles: BTreeSet::new(),
        records: marked.record_charge(),
        spool: marked.spool_charge(),
        limits,
    };
    b.record(navigation.additional_records())?;
    let count = navigation
        .links()
        .len()
        .checked_add(navigation.footnote_references().links().len())
        .ok_or(E::RecordLimit)?;
    let registry = marked.structure().registry();
    let pages = marked.pages().len();
    b.record(
        (count as u64)
            .checked_mul(4)
            .and_then(|n| n.checked_add((registry.nodes().len() as u64).checked_mul(2)?))
            .and_then(|n| n.checked_add(pages as u64))
            .ok_or(E::RecordLimit)?,
    )?;
    let mut order = Vec::new();
    order
        .try_reserve_exact(count)
        .map_err(|_| E::AllocationFailure)?;
    for (i, link) in navigation.links().iter().enumerate() {
        order.push((
            link.page_index(),
            link.fragment_index(),
            link.owner(),
            ProductionFootnoteAnnotationSource::Ordinary(i),
        ));
    }
    let mut inside_link = Vec::new();
    inside_link
        .try_reserve_exact(registry.nodes().len())
        .map_err(|_| E::AllocationFailure)?;
    for node in registry.nodes() {
        if node.structure_node_id().get() as usize != inside_link.len() {
            return Err(E::InvalidStructure);
        }
        let inherited = match node.parent() {
            Some(id) => *inside_link
                .get(id.get() as usize)
                .ok_or(E::InvalidStructure)?,
            None => false,
        };
        inside_link.push(inherited || node.role() == StructureRole::Link);
    }
    for (i, link) in navigation.footnote_references().links().iter().enumerate() {
        // A footnote number inside an authored Link would create overlapping
        // annotations with different destinations. Do not silently choose one.
        if *inside_link
            .get(link.node().get() as usize)
            .ok_or(E::InvalidStructure)?
        {
            return Err(E::InvalidStructure);
        }
        order.push((
            link.page_index(),
            link.fragment_index(),
            link.owner(),
            ProductionFootnoteAnnotationSource::Footnote(i),
        ));
    }
    order.sort_unstable();
    let mut bindings = Vec::new();
    bindings
        .try_reserve_exact(count)
        .map_err(|_| E::AllocationFailure)?;
    let mut node_annotations = Vec::new();
    node_annotations
        .try_reserve_exact(registry.nodes().len())
        .map_err(|_| E::AllocationFailure)?;
    node_annotations.resize_with(registry.nodes().len(), Vec::new);
    let height = marked
        .structure()
        .display()
        .source()
        .block_layout()
        .page_geometry()
        .page_height()
        .get();
    for (index, (page, _, owner, source)) in order.into_iter().enumerate() {
        let (node, bounds, accessible) = match source {
            ProductionFootnoteAnnotationSource::Ordinary(i) => {
                let link = &navigation.links()[i];
                (
                    link.node(),
                    link.bounds(),
                    registry
                        .node(link.node())
                        .and_then(|n| n.accessible_name())
                        .ok_or(E::InvalidStructure)?,
                )
            }
            ProductionFootnoteAnnotationSource::Footnote(i) => {
                let link = &navigation.footnote_references().links()[i];
                let label = marked
                    .structure()
                    .display()
                    .source()
                    .line_layout()
                    .source_flow()
                    .footnote_marker_text(owner)
                    .ok_or(E::InvalidStructure)?;
                (link.node(), link.bounds(), label)
            }
        };
        let right = bounds
            .x()
            .checked_add(bounds.width().get())
            .ok_or(E::ReceiptMismatch)?;
        let top = height.checked_sub(bounds.y()).ok_or(E::ReceiptMismatch)?;
        let bottom = top
            .checked_sub(bounds.height().get())
            .ok_or(E::ReceiptMismatch)?;
        let key = annotation_parent_key(pages, index)?;
        let index32 = u32::try_from(index).map_err(|_| E::ObjectLimit)?;
        b.start(R::LinkAnnotation(index32))?;
        b.bytes(format!("<< /Type /Annot /Subtype /Link /Rect [{} {} {} {}] /Border [0 0 0] /F 4 /StructParent {} /P ",
            number(bounds.x().raw()), number(bottom.raw()), number(right.raw()), number(top.raw()), key))?;
        b.reference(R::Page(page))?;
        match source {
            ProductionFootnoteAnnotationSource::Ordinary(i) => match navigation.links()[i].target()
            {
                typaxis_display_list::ProductionBodyLinkTarget::Internal(target) => {
                    b.bytes(" /Dest ")?;
                    b.text(
                        navigation
                            .destination_name(target)
                            .ok_or(E::ReceiptMismatch)?
                            .as_str(),
                    )?;
                }
                typaxis_display_list::ProductionBodyLinkTarget::Uri(uri) => {
                    b.bytes(" /A << /S /URI /URI <")?;
                    for byte in uri.bytes() {
                        b.bytes(format!("{byte:02X}"))?;
                    }
                    b.bytes("> >>")?;
                }
            },
            ProductionFootnoteAnnotationSource::Footnote(i) => {
                let target = navigation
                    .footnote_references()
                    .destinations()
                    .get(navigation.footnote_references().links()[i].definition_index())
                    .ok_or(E::ReceiptMismatch)?;
                let y = height
                    .checked_sub(target.bounds().y())
                    .ok_or(E::ReceiptMismatch)?;
                b.bytes(" /Dest [")?;
                b.reference(R::Page(target.page_index()))?;
                b.bytes(format!(
                    " /XYZ {} {} null]",
                    number(target.bounds().x().raw()),
                    number(y.raw())
                ))?;
            }
        }
        b.bytes(" /Contents ")?;
        b.text(accessible)?;
        b.bytes(" >>")?;
        let indices = node_annotations
            .get_mut(node.get() as usize)
            .ok_or(E::InvalidStructure)?;
        indices.try_reserve(1).map_err(|_| E::AllocationFailure)?;
        indices.push(index32);
        bindings.push(ProductionFootnoteAnnotationBinding {
            source,
            node,
            page,
            parent_key: key,
        });
    }
    let mut page_annotations = Vec::new();
    page_annotations
        .try_reserve_exact(pages)
        .map_err(|_| E::AllocationFailure)?;
    let mut cursor = 0;
    for page in 0..pages {
        let start = cursor;
        while bindings
            .get(cursor)
            .is_some_and(|a| a.page as usize == page)
        {
            cursor += 1;
        }
        page_annotations.push(start..cursor);
    }
    if cursor != bindings.len() {
        return Err(E::ReceiptMismatch);
    }
    Ok(ProductionFootnoteAnnotations {
        marked,
        navigation,
        objects: b.objects,
        bindings,
        node_annotations,
        page_annotations,
        records: b.records,
        spool: b.spool,
    })
}
