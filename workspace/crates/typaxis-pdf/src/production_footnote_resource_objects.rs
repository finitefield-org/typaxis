//! Actual font/image/vector programs and page resources after joint structure.
use super::*;
use crate::ProductionFootnoteStructureObjects;

pub struct ProductionFootnoteResourceObjects<
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
    structure: &'r ProductionFootnoteStructureObjects<
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
    objects: Vec<ProductionBodyObject>,
    records: u64,
    spool: u64,
    object_count: usize,
}
impl<'r, 'z, 'm, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    ProductionFootnoteResourceObjects<'r, 'z, 'm, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn structure_objects(
        &self,
    ) -> &'r ProductionFootnoteStructureObjects<
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
        self.structure
    }
    pub fn objects(&self) -> &[ProductionBodyObject] {
        &self.objects
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool
    }
    /// All objects retained in annotation, structure and resource contributions.
    pub fn retained_object_count(&self) -> usize {
        self.object_count
    }
    pub fn verify(
        &self,
        structure: &ProductionFootnoteStructureObjects<
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
    ) -> Result<(), E> {
        if !std::ptr::eq(self.structure, structure) {
            return Err(E::ReceiptMismatch);
        }
        structure.verify(structure.annotations(), admitted, limits)
    }
}
pub fn build_production_footnote_resource_objects<
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
    structure: &'r ProductionFootnoteStructureObjects<
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
    ProductionFootnoteResourceObjects<'r, 'z, 'm, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    E,
> {
    structure.verify(structure.annotations(), admitted, limits)?;
    let annotations = structure.annotations();
    let marked = annotations.marked();
    let object_base = structure
        .objects()
        .len()
        .checked_add(annotations.objects().len())
        .ok_or(E::ObjectLimit)?;
    let mut b = Builder {
        object_base,
        objects: Vec::new(),
        roles: BTreeSet::new(),
        records: structure.record_charge(),
        spool: structure.spool_charge(),
        limits,
    };
    project_resource_objects(
        &mut b,
        marked.content().plans().fonts().fonts(),
        marked.content().vectors(),
        marked.content().rasters().plans(),
        marked.pages(),
        marked.content().pages(),
        marked.content().text().paints(),
        marked.anchors(),
    )?;
    let navigation = annotations.navigation();
    project_navigation_targets(
        &mut b,
        marked
            .structure()
            .display()
            .source()
            .block_layout()
            .page_geometry()
            .page_height()
            .get(),
        navigation.destinations(),
        |i| navigation.destination_name(i).map(|n| n.as_str()),
        navigation
            .outline_entries()
            .iter()
            .map(|e| (e.outline_id, e.label.as_str(), e.destination.as_str())),
        navigation.outline(),
        navigation.outline_root(),
    )?;
    // Resolve all local resources/outline roles here; destinations retain
    // checked Page references for the final page-tree owner.
    for object in &b.objects {
        for chunk in object.chunks() {
            if let ProductionBodyObjectChunk::Reference(role) = chunk {
                match role {
                    R::Page(page) if (*page as usize) < marked.pages().len() => {}
                    _ if b.roles.contains(role) => {}
                    _ => return Err(E::ReceiptMismatch),
                }
            }
        }
    }
    let object_count = object_base
        .checked_add(b.objects.len())
        .ok_or(E::ObjectLimit)?;
    Ok(ProductionFootnoteResourceObjects {
        structure,
        objects: b.objects,
        records: b.records,
        spool: b.spool,
        object_count,
    })
}
