//! Definition destinations and reference hit areas from actual generated labels.
use super::*;
use crate::ProductionFootnoteStructure;
use typaxis_layout::GeneratedStructureSlot;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionFootnoteDestination {
    definition_index: usize,
    owner: NodeId,
    node: StructureNodeId,
    annotation_node: StructureNodeId,
    page_index: u32,
    fragment_index: u32,
    bounds: Rect,
    label_node: StructureNodeId,
    label_bounds: Rect,
    return_link_index: Option<usize>,
}
impl ProductionFootnoteDestination {
    /// The generated number's structure owner and hit area, excluding the
    /// adjacent definition text. The forward destination covers the first row.
    pub fn label_node(&self) -> StructureNodeId {
        self.label_node
    }
    pub fn label_bounds(&self) -> Rect {
        self.label_bounds
    }
    /// Index into this navigation's reference links. A static return action
    /// points to the first authored reference, not the smallest node ID or the
    /// first reference encountered while painting a page.
    pub fn return_link_index(&self) -> Option<usize> {
        self.return_link_index
    }

    pub fn definition_index(&self) -> usize {
        self.definition_index
    }
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn annotation_node(&self) -> StructureNodeId {
        self.annotation_node
    }
    pub fn node(&self) -> StructureNodeId {
        self.node
    }
    pub fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionFootnoteReferenceLink {
    owner: NodeId,
    node: StructureNodeId,
    annotation_node: StructureNodeId,
    definition_index: usize,
    page_index: u32,
    fragment_index: u32,
    bounds: Rect,
}
impl ProductionFootnoteReferenceLink {
    pub fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn annotation_node(&self) -> StructureNodeId {
        self.annotation_node
    }
    pub fn node(&self) -> StructureNodeId {
        self.node
    }
    pub fn definition_index(&self) -> usize {
        self.definition_index
    }
    pub fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
}
pub struct ProductionFootnoteReferenceNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    structure: &'n ProductionFootnoteStructure<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    destinations: Vec<ProductionFootnoteDestination>,
    links: Vec<ProductionFootnoteReferenceLink>,
    page_links: Vec<Range<usize>>,
    record_base: u64,
    additional_records: u64,
    fingerprint: [u8; 32],
}
impl<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    ProductionFootnoteReferenceNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub fn structure(&self) -> &'n ProductionFootnoteStructure<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.structure
    }
    pub fn destinations(&self) -> &[ProductionFootnoteDestination] {
        &self.destinations
    }
    pub fn links(&self) -> &[ProductionFootnoteReferenceLink] {
        &self.links
    }
    pub fn page_links(&self, page: u32) -> Option<Range<usize>> {
        self.page_links.get(page as usize).cloned()
    }
    pub fn record_base(&self) -> u64 {
        self.record_base
    }
    pub fn additional_records(&self) -> u64 {
        self.additional_records
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        structure: &ProductionFootnoteStructure<'_, '_, '_, '_, '_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyNavigationError> {
        if !std::ptr::eq(self.structure, structure) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        structure
            .verify(structure.display(), admitted, limits)
            .map_err(|_| error(NodeId::new(0), E::ReceiptMismatch))
    }
}
pub fn build_production_footnote_reference_navigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    structure: &'n ProductionFootnoteStructure<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    retained_record_charge: u64,
) -> Result<
    ProductionFootnoteReferenceNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    ProductionBodyNavigationError,
> {
    let root = NodeId::new(0);
    let display = structure.display();
    structure
        .verify(display, admitted, limits)
        .map_err(|_| error(root, E::ReceiptMismatch))?;
    if retained_record_charge < structure.record_charge() {
        return Err(error(root, E::ReceiptMismatch));
    }
    let source = display.source().footnote_lines();
    let pages = display.source().geometry().pages().len();
    let max = limits.base().get().max_fragments;
    let fixed = (source.definitions().len() as u64)
        .checked_mul(7)
        .and_then(|n| n.checked_add((source.references().len() as u64).checked_mul(3)?))
        .and_then(|n| n.checked_add(pages as u64))
        .ok_or_else(|| error(root, E::RecordLimit))?;
    let mut additional_records = 0;
    add_records(
        &mut additional_records,
        fixed,
        retained_record_charge,
        max,
        root,
    )?;
    let mut definitions = BTreeMap::new();
    for (i, definition) in source.definitions().iter().enumerate() {
        if definitions.insert(definition.owner(), i).is_some() {
            return Err(error(definition.owner(), E::ReceiptMismatch));
        }
    }
    let mut references = BTreeMap::new();
    for (ordinal, reference) in source.references().iter().enumerate() {
        if reference.definition_index() >= source.definitions().len()
            || references
                .insert(
                    reference.owner(),
                    (reference.definition_index(), false, ordinal),
                )
                .is_some()
        {
            return Err(error(reference.owner(), E::ReceiptMismatch));
        }
    }
    let mut first_references: Vec<Option<(usize, usize)>> = Vec::new();
    reserve(&mut first_references, definitions.len(), root)?;
    first_references.resize(definitions.len(), None);
    let mut destinations = Vec::new();
    reserve(&mut destinations, definitions.len(), root)?;
    destinations.resize(definitions.len(), None);
    let mut first_fragments = Vec::new();
    reserve(&mut first_fragments, definitions.len(), root)?;
    first_fragments.resize(definitions.len(), None);
    let mut first_by_fragment = BTreeMap::new();
    let mut global_fragment = 0u32;
    for page in display.source().geometry().pages() {
        for placed in page.fragments() {
            if let Some(index) = placed.definition_index() {
                let slot = first_fragments
                    .get_mut(index)
                    .ok_or_else(|| error(root, E::ReceiptMismatch))?;
                if slot.is_none() {
                    let fragment = placed.fragment();
                    *slot = Some((global_fragment, fragment.page_index(), fragment.bounds()));
                    first_by_fragment.insert(global_fragment, index);
                }
            }
            global_fragment = global_fragment
                .checked_add(1)
                .ok_or_else(|| error(root, E::RecordLimit))?;
        }
    }
    // Include every paint in the first row, including a large list marker.
    // Only a definition's first fragment is a destination, never a continuation.
    for draw in display.draws() {
        let (fragment, bounds) = match draw {
            ProductionBodyDraw::Text(t) => (t.fragment_index(), t.logical_bounds()),
            ProductionBodyDraw::Vector(v) => (v.fragment_index(), Some(v.viewport())),
            ProductionBodyDraw::SvgFigure(r) => (r.fragment_index(), Some(r.viewport())),
            ProductionBodyDraw::Raster(r) => (r.fragment_index(), Some(r.viewport())),
            ProductionBodyDraw::Math(m) => (m.fragment_index(), Some(m.bounds())),
        };
        if let (Some(&index), Some(bounds)) = (first_by_fragment.get(&fragment), bounds) {
            let first = first_fragments[index]
                .as_mut()
                .ok_or_else(|| error(root, E::ReceiptMismatch))?;
            first.2 = union(first.2, bounds, source.definitions()[index].owner())?;
        }
    }
    let mut links: Vec<ProductionFootnoteReferenceLink> = Vec::new();
    for group in structure.groups() {
        let label = structure
            .registry()
            .node(group.node())
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let StructureOwner::Generated(key) = label.owner() else {
            continue;
        };
        if key.slot() != GeneratedStructureSlot::FootnoteLabel {
            continue;
        }
        let link_node = label
            .parent()
            .and_then(|id| structure.registry().node(id))
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        if link_node.role() != StructureRole::Link
            || !matches!(link_node.owner(), StructureOwner::Generated(link_key)
                if link_key.slot() == GeneratedStructureSlot::FootnoteLink
                    && link_key.owner_node_id() == key.owner_node_id())
        {
            return Err(error(root, E::ReceiptMismatch));
        }
        let parent = link_node
            .parent()
            .and_then(|id| structure.registry().node(id))
            .ok_or_else(|| error(root, E::ReceiptMismatch))?;
        let StructureOwner::Source(owner) = parent.owner() else {
            return Err(error(root, E::ReceiptMismatch));
        };
        let mut bounds = None;
        let mut fragment = None;
        for index in group.draws() {
            let Some(ProductionBodyDraw::Text(text)) = display.draws().get(index) else {
                return Err(error(owner, E::ReceiptMismatch));
            };
            if text.owner() != owner
                || text.page_index() != group.page_index()
                || fragment.is_some_and(|id| id != text.fragment_index())
            {
                return Err(error(owner, E::ReceiptMismatch));
            }
            fragment = Some(text.fragment_index());
            let rect = text
                .logical_bounds()
                .ok_or_else(|| error(owner, E::InvalidGeometry))?;
            bounds = Some(match bounds {
                Some(prior) => union(prior, rect, owner)?,
                None => rect,
            });
        }
        let bounds = bounds.ok_or_else(|| error(owner, E::UnplacedLink))?;
        let fragment_index = fragment.ok_or_else(|| error(owner, E::UnplacedLink))?;
        match parent.role() {
            StructureRole::Note => {
                let index = *definitions
                    .get(&owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                let (first_index, first_page, first_bounds) =
                    first_fragments[index].ok_or_else(|| error(owner, E::UnplacedAnchor))?;
                if first_index != fragment_index || first_page != group.page_index() {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                // Include the actual first content row, which may be taller
                // than its number when a definition starts with math or a figure.
                let label_bounds = bounds;
                let bounds = union(first_bounds, bounds, owner)?;
                if destinations[index]
                    .replace(ProductionFootnoteDestination {
                        definition_index: index,
                        owner,
                        node: parent.structure_node_id(),
                        annotation_node: link_node.structure_node_id(),
                        page_index: group.page_index(),
                        fragment_index,
                        bounds,
                        label_node: label.structure_node_id(),
                        label_bounds,
                        return_link_index: None,
                    })
                    .is_some()
                {
                    return Err(error(owner, E::ReceiptMismatch));
                }
            }
            StructureRole::Reference => {
                let (definition_index, seen, ordinal) = references
                    .get_mut(&owner)
                    .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
                *seen = true;
                if let Some(prior) = links.last_mut().filter(|l| {
                    l.owner == owner
                        && l.page_index == group.page_index()
                        && l.fragment_index == fragment_index
                }) {
                    prior.bounds = union(prior.bounds, bounds, owner)?;
                } else {
                    add_records(
                        &mut additional_records,
                        1,
                        retained_record_charge,
                        max,
                        owner,
                    )?;
                    reserve(&mut links, 1, owner)?;
                    links.push(ProductionFootnoteReferenceLink {
                        owner,
                        node: parent.structure_node_id(),
                        annotation_node: link_node.structure_node_id(),
                        definition_index: *definition_index,
                        page_index: group.page_index(),
                        fragment_index,
                        bounds,
                    });
                }
                let first = &mut first_references[*definition_index];
                if first.is_none_or(|(prior, _)| *ordinal < prior) {
                    *first = Some((*ordinal, links.len() - 1));
                }
            }
            _ => return Err(error(owner, E::ReceiptMismatch)),
        }
    }
    for (&owner, (_, seen, _)) in &references {
        if !seen {
            return Err(error(owner, E::UnplacedLink));
        }
    }
    let mut retained = Vec::new();
    reserve(&mut retained, destinations.len(), root)?;
    for (index, destination) in destinations.into_iter().enumerate() {
        let mut destination = destination
            .ok_or_else(|| error(source.definitions()[index].owner(), E::UnplacedAnchor))?;
        destination.return_link_index = first_references[index].map(|(_, link)| link);
        retained.push(destination);
    }
    let mut page_links = Vec::new();
    reserve(&mut page_links, pages, root)?;
    let mut cursor = 0;
    for page in 0..pages {
        let start = cursor;
        while links
            .get(cursor)
            .is_some_and(|l| l.page_index as usize == page)
        {
            cursor += 1;
        }
        page_links.push(start..cursor);
    }
    if cursor != links.len() {
        return Err(error(root, E::ReceiptMismatch));
    }
    let mut digest = [0u8; 64];
    digest[..32].copy_from_slice(&sha256(
        b"typaxis.production-footnote-reference-navigation/2",
    ));
    digest[32..].copy_from_slice(&structure.fingerprint());
    Ok(ProductionFootnoteReferenceNavigation {
        structure,
        destinations: retained,
        links,
        page_links,
        record_base: retained_record_charge,
        additional_records,
        fingerprint: sha256(&digest),
    })
}
