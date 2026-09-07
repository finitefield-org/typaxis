//! Shared structure-object projection bound to the exact joint annotations.
use super::*;
use crate::ProductionFootnoteAnnotations;

pub struct ProductionFootnoteStructureObjects<
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
    annotations:
        &'z ProductionFootnoteAnnotations<'m, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    objects: Vec<ProductionBodyObject>,
    record_charge: u64,
    spool_charge: u64,
}
impl<'z, 'm, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    ProductionFootnoteStructureObjects<'z, 'm, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn annotations(
        &self,
    ) -> &'z ProductionFootnoteAnnotations<'m, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.annotations
    }
    pub fn objects(&self) -> &[ProductionBodyObject] {
        &self.objects
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub fn verify(
        &self,
        annotations: &ProductionFootnoteAnnotations<
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
        if !std::ptr::eq(self.annotations, annotations) {
            return Err(E::ReceiptMismatch);
        }
        annotations.verify(annotations.marked(), admitted, limits)
    }
}
pub fn build_production_footnote_structure_objects<
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
    annotations: &'z ProductionFootnoteAnnotations<
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
    ProductionFootnoteStructureObjects<'z, 'm, 'c, 'e, 't, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    E,
> {
    annotations.verify(annotations.marked(), admitted, limits)?;
    let marked = annotations.marked();
    let structure = marked.structure();
    let registry = structure.registry();
    let extra = registry
        .nodes()
        .len()
        .checked_add(2)
        .and_then(|n| {
            n.checked_add(usize::from(
                registry.nodes().iter().any(|n| n.structure_id().is_some()),
            ))
        })
        .ok_or(E::ObjectLimit)?;
    let total = annotations
        .objects()
        .len()
        .checked_add(extra)
        .ok_or(E::ObjectLimit)?;
    if total as u64 > u64::from(limits.base().get().max_pdf_objects) {
        return Err(E::ObjectLimit);
    }
    for (index, binding) in annotations.bindings().iter().enumerate() {
        if binding.parent_key() != annotation_parent_key(marked.pages().len(), index)?
            || registry.node(binding.node()).is_none()
        {
            return Err(E::InvalidStructure);
        }
    }
    let mut b = Builder {
        objects: Vec::new(),
        roles: BTreeSet::new(),
        records: annotations.record_charge(),
        spool: annotations.spool_charge(),
        limits,
    };
    project_structure_objects(
        &mut b,
        registry,
        marked.pages(),
        structure.groups(),
        |p| structure.page_groups(p),
        |n| structure.node_groups(n),
        annotations.bindings().len(),
        |i| {
            annotations
                .bindings()
                .get(i)
                .map(|a| (a.node(), a.page_index()))
        },
        |n| annotations.node_annotations(n),
    )?;
    if b.objects.len() != extra {
        return Err(E::InvalidStructure);
    }
    Ok(ProductionFootnoteStructureObjects {
        annotations,
        objects: b.objects,
        record_charge: b.records,
        spool_charge: b.spool,
    })
}
