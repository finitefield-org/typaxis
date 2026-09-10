//! Navigation derived only from selected production fragments and source owners.
use crate::{ProductionBodyDraw, ProductionBodyStructure};
use std::{collections::BTreeMap, ops::Range};
use typaxis_core::{
    sha256, AnchorId, Length, M4EffectiveResourceLimits, NodeId, PositiveLength, Rect,
};
use typaxis_document::StagingOutlineEntry;
use typaxis_layout::{StructureNodeId, StructureOwner, StructureRole};
use typaxis_resource_admission::AdmittedResourceLedger;

pub const PRODUCTION_BODY_NAVIGATION_ALGORITHM: &str = "typaxis.production-body-navigation/1";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyNavigationErrorKind {
    ReceiptMismatch,
    UnplacedAnchor,
    UnplacedLink,
    UnsupportedLinkTarget,
    NestedLink,
    InvalidGeometry,
    RecordLimit,
    AllocationFailure,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyNavigationError {
    pub owner: NodeId,
    pub kind: ProductionBodyNavigationErrorKind,
}
use ProductionBodyNavigationErrorKind as E;
fn error(owner: NodeId, kind: E) -> ProductionBodyNavigationError {
    ProductionBodyNavigationError { owner, kind }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyDestination {
    anchor_index: u32,
    owner: NodeId,
    page_index: u32,
    fragment_index: u32,
    x: Length,
    y: Length,
}
impl ProductionBodyDestination {
    pub const fn anchor_index(&self) -> u32 {
        self.anchor_index
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn x(&self) -> Length {
        self.x
    }
    pub const fn y(&self) -> Length {
        self.y
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyLinkTarget<'a> {
    Internal(u32),
    Uri(&'a str),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductionBodyLink<'a> {
    owner: NodeId,
    node: StructureNodeId,
    target: ProductionBodyLinkTarget<'a>,
    page_index: u32,
    fragment_index: u32,
    bounds: Rect,
}
impl<'a> ProductionBodyLink<'a> {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn node(&self) -> StructureNodeId {
        self.node
    }
    pub const fn target(&self) -> ProductionBodyLinkTarget<'a> {
        self.target
    }
    pub const fn destination_index(&self) -> Option<u32> {
        match self.target {
            ProductionBodyLinkTarget::Internal(index) => Some(index),
            _ => None,
        }
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProductionBodyOutlineTopology {
    parent: Option<u32>,
    previous: Option<u32>,
    next: Option<u32>,
    first: Option<u32>,
    last: Option<u32>,
    descendants: u32,
}
impl ProductionBodyOutlineTopology {
    pub const fn parent(&self) -> Option<u32> {
        self.parent
    }
    pub const fn previous(&self) -> Option<u32> {
        self.previous
    }
    pub const fn next(&self) -> Option<u32> {
        self.next
    }
    pub const fn first(&self) -> Option<u32> {
        self.first
    }
    pub const fn last(&self) -> Option<u32> {
        self.last
    }
    pub const fn descendants(&self) -> u32 {
        self.descendants
    }
}

pub struct ProductionBodyNavigation<'n, 'v, 'd, 's, 'p, 'a> {
    structure: &'n ProductionBodyStructure<'v, 'd, 's, 'p, 'a>,
    destinations: Vec<ProductionBodyDestination>,
    links: Vec<ProductionBodyLink<'a>>,
    node_links: Vec<Vec<u32>>,
    page_links: Vec<Range<usize>>,
    outline: Vec<ProductionBodyOutlineTopology>,
    outline_root: ProductionBodyOutlineTopology,
    additional_records: u64,
    record_base: u64,
    fingerprint: [u8; 32],
}
impl<'n, 'v, 'd, 's, 'p, 'a> ProductionBodyNavigation<'n, 'v, 'd, 's, 'p, 'a> {
    pub const fn structure(&self) -> &'n ProductionBodyStructure<'v, 'd, 's, 'p, 'a> {
        self.structure
    }
    pub fn destinations(&self) -> &[ProductionBodyDestination] {
        &self.destinations
    }
    pub fn destination_name(&self, index: u32) -> Option<&AnchorId> {
        self.destinations.get(index as usize)?;
        self.structure
            .display()
            .selected()
            .line_layout()
            .source_flow()
            .navigation()
            .anchors()
            .get(index as usize)
            .map(|a| &a.0)
    }
    pub fn links(&self) -> &[ProductionBodyLink<'a>] {
        &self.links
    }
    pub fn node_links(&self, id: StructureNodeId) -> Option<&[u32]> {
        self.structure.registry().node(id)?;
        Some(
            self.node_links
                .get(id.get() as usize)
                .map_or(&[], Vec::as_slice),
        )
    }
    pub fn page_links(&self, page: u32) -> Option<Range<usize>> {
        self.structure
            .display()
            .selected()
            .pages()
            .get(page as usize)?;
        Some(self.page_links.get(page as usize).cloned().unwrap_or(0..0))
    }
    pub fn outline_entries(&self) -> &[StagingOutlineEntry] {
        self.structure
            .display()
            .selected()
            .line_layout()
            .source_flow()
            .navigation()
            .outline()
            .entries()
    }
    pub fn outline(&self) -> &[ProductionBodyOutlineTopology] {
        &self.outline
    }
    pub const fn outline_root(&self) -> &ProductionBodyOutlineTopology {
        &self.outline_root
    }
    /// Includes bounded temporary indexing work; add once to the shared marked
    /// content charge, not a second copy of its structure/display ancestors.
    pub const fn additional_records(&self) -> u64 {
        self.additional_records
    }
    pub const fn record_base(&self) -> u64 {
        self.record_base
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        structure: &ProductionBodyStructure<'_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyNavigationError> {
        if !std::ptr::eq(self.structure, structure) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct Point {
    page: u32,
    fragment: u32,
    x: Length,
    y: Length,
}
fn first(target: &mut Option<Point>, point: Point) {
    if target.is_none_or(|old| point.fragment < old.fragment) {
        *target = Some(point);
    }
}
fn reserve<T>(
    v: &mut Vec<T>,
    n: usize,
    owner: NodeId,
) -> Result<(), ProductionBodyNavigationError> {
    v.try_reserve(n)
        .map_err(|_| error(owner, E::AllocationFailure))
}
fn add_records(
    used: &mut u64,
    n: u64,
    base: u64,
    max: u64,
    owner: NodeId,
) -> Result<(), ProductionBodyNavigationError> {
    *used = used
        .checked_add(n)
        .filter(|v| base.checked_add(*v).is_some_and(|v| v <= max))
        .ok_or_else(|| error(owner, E::RecordLimit))?;
    Ok(())
}
fn union(a: Rect, b: Rect, owner: NodeId) -> Result<Rect, ProductionBodyNavigationError> {
    let x = a.x().min(b.x());
    let y = a.y().min(b.y());
    let right = a
        .x()
        .checked_add(a.width().get())
        .zip(b.x().checked_add(b.width().get()))
        .map(|(a, b)| a.max(b));
    let bottom = a
        .y()
        .checked_add(a.height().get())
        .zip(b.y().checked_add(b.height().get()))
        .map(|(a, b)| a.max(b));
    let width = right
        .and_then(|r| r.checked_sub(x))
        .and_then(PositiveLength::new)
        .ok_or_else(|| error(owner, E::InvalidGeometry))?;
    let height = bottom
        .and_then(|r| r.checked_sub(y))
        .and_then(PositiveLength::new)
        .ok_or_else(|| error(owner, E::InvalidGeometry))?;
    Ok(Rect::new(x, y, width, height))
}

pub fn build_production_body_navigation<'n, 'v, 'd, 's, 'p, 'a>(
    structure: &'n ProductionBodyStructure<'v, 'd, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    retained_record_charge: u64,
) -> Result<ProductionBodyNavigation<'n, 'v, 'd, 's, 'p, 'a>, ProductionBodyNavigationError> {
    let root = NodeId::new(0);
    let display = structure.display();
    structure
        .verify(display, admitted, limits)
        .map_err(|_| error(root, E::ReceiptMismatch))?;
    // The caller may already hold text/vector/marked-content contributions
    // sharing this structure. Charge navigation before allocating against that
    // complete live ancestor, never less than the structure's own charge.
    if retained_record_charge < structure.record_charge() {
        return Err(error(root, E::ReceiptMismatch));
    }
    if retained_record_charge > limits.base().get().max_fragments {
        return Err(error(root, E::RecordLimit));
    }
    let projection = project_navigation(
        display.selected().line_layout().source_flow(),
        structure.registry(),
        structure.fingerprint(),
        structure.groups(),
        display.draws(),
        display.inline_anchors(),
        display.selected().fragments().iter().copied(),
        display.selected().pages().len(),
        limits,
        retained_record_charge,
    )?;
    Ok(ProductionBodyNavigation {
        structure,
        destinations: projection.destinations,
        links: projection.links,
        node_links: projection.node_links,
        page_links: projection.page_links,
        outline: projection.outline,
        outline_root: projection.outline_root,
        additional_records: projection.additional_records,
        record_base: projection.record_base,
        fingerprint: projection.fingerprint,
    })
}

struct NavigationProjection<'a> {
    destinations: Vec<ProductionBodyDestination>,
    links: Vec<ProductionBodyLink<'a>>,
    node_links: Vec<Vec<u32>>,
    page_links: Vec<Range<usize>>,
    outline: Vec<ProductionBodyOutlineTopology>,
    outline_root: ProductionBodyOutlineTopology,
    additional_records: u64,
    record_base: u64,
    fingerprint: [u8; 32],
}
fn project_navigation<'a>(
    flow: &'a typaxis_syntax::ProductionTextFlow<'a>,
    registry: &typaxis_layout::StructureRegistryReceiptV2,
    structure_fingerprint: [u8; 32],
    groups: &[crate::ProductionBodyStructureGroup],
    draws: &[ProductionBodyDraw<'_>],
    inline_anchors: &[crate::ProductionBodyInlineAnchor<'_>],
    fragments: impl Iterator<Item = typaxis_pagination::ProductionBodyFragment>,
    page_count: usize,
    limits: &M4EffectiveResourceLimits,
    retained_record_charge: u64,
) -> Result<NavigationProjection<'a>, ProductionBodyNavigationError> {
    let root = NodeId::new(0);
    let source = flow.navigation();
    let mut digest = [0u8; 64];
    digest[..32].copy_from_slice(&sha256(PRODUCTION_BODY_NAVIGATION_ALGORITHM.as_bytes()));
    digest[32..].copy_from_slice(&structure_fingerprint);
    let mut result = NavigationProjection {
        destinations: Vec::new(),
        links: Vec::new(),
        node_links: Vec::new(),
        page_links: Vec::new(),
        outline: Vec::new(),
        outline_root: ProductionBodyOutlineTopology::default(),
        additional_records: 0,
        record_base: retained_record_charge,
        fingerprint: sha256(&digest),
    };
    if source.anchors().is_empty()
        && source.internal_links().is_empty()
        && source.outline().entries().is_empty()
        && !registry
            .nodes()
            .iter()
            .any(|n| n.role() == StructureRole::Link && !matches!(n.owner(),
                StructureOwner::Generated(key) if key.slot() == typaxis_layout::GeneratedStructureSlot::FootnoteLink))
    {
        return Ok(result);
    }
    let base = retained_record_charge;
    let max = limits.base().get().max_fragments;
    let n = registry.nodes().len();
    let pages = page_count;
    let fixed = (n as u64)
        .checked_mul(4)
        .and_then(|v| v.checked_add(pages as u64))
        .and_then(|v| v.checked_add((source.anchors().len() as u64).checked_mul(2)?))
        .and_then(|v| v.checked_add(inline_anchors.len() as u64))
        .and_then(|v| {
            v.checked_add(
                flow.paragraphs()
                    .iter()
                    .flat_map(|p| p.items())
                    .filter(|site| site.link_target().is_some())
                    .count() as u64,
            )
        })
        .and_then(|v| v.checked_add(source.outline().entries().len() as u64))
        .ok_or_else(|| error(root, E::RecordLimit))?;
    add_records(&mut result.additional_records, fixed, base, max, root)?;
    let mut nodes = BTreeMap::new();
    let mut nearest_link: Vec<Option<StructureNodeId>> = Vec::new();
    reserve(&mut nearest_link, n, root)?;
    let mut points = Vec::new();
    reserve(&mut points, n, root)?;
    points.resize(n, None);
    reserve(&mut result.node_links, n, root)?;
    result.node_links.resize_with(n, Vec::new);
    let mut targets = BTreeMap::new();
    for site in flow.paragraphs().iter().flat_map(|p| p.items()) {
        let Some(target) = site.link_target() else {
            continue;
        };
        let owner = site.owner();
        let target = match target {
            typaxis_syntax::ProductionInlineLinkTarget::Internal { anchor_id } => {
                let index = source
                    .anchors()
                    .binary_search_by(|(a, _)| a.as_str().cmp(anchor_id))
                    .map_err(|_| error(owner, E::ReceiptMismatch))?;
                if source.internal_link_target(owner).map(|a| a.as_str()) != Some(anchor_id) {
                    return Err(error(owner, E::ReceiptMismatch));
                }
                ProductionBodyLinkTarget::Internal(
                    u32::try_from(index).map_err(|_| error(owner, E::RecordLimit))?,
                )
            }
            typaxis_syntax::ProductionInlineLinkTarget::Uri { uri } => {
                ProductionBodyLinkTarget::Uri(uri)
            }
        };
        if targets.insert(owner, target).is_some() {
            return Err(error(owner, E::ReceiptMismatch));
        }
    }
    for (index, node) in registry.nodes().iter().enumerate() {
        if node.structure_node_id().get() as usize != index {
            return Err(error(root, E::ReceiptMismatch));
        }
        let inherited = if let Some(parent) = node.parent() {
            *nearest_link
                .get(parent.get() as usize)
                .ok_or_else(|| error(root, E::ReceiptMismatch))?
        } else {
            None
        };
        if let StructureOwner::Source(owner) = node.owner() {
            nodes.insert(owner, index);
        }
        let generated_footnote = matches!(node.owner(), StructureOwner::Generated(key)
            if key.slot() == typaxis_layout::GeneratedStructureSlot::FootnoteLink);
        let link = if node.role() == StructureRole::Link && !generated_footnote {
            let StructureOwner::Source(owner) = node.owner() else {
                return Err(error(root, E::ReceiptMismatch));
            };
            if inherited.is_some() {
                return Err(error(owner, E::NestedLink));
            }
            if !targets.contains_key(&owner) {
                return Err(error(owner, E::UnsupportedLinkTarget));
            }
            Some(node.structure_node_id())
        } else {
            inherited
        };
        nearest_link.push(link);
    }
    for (index, fragment) in fragments.enumerate() {
        let node = *nodes
            .get(&fragment.owner())
            .ok_or_else(|| error(fragment.owner(), E::ReceiptMismatch))?;
        first(
            &mut points[node],
            Point {
                page: fragment.page_index(),
                fragment: u32::try_from(index)
                    .map_err(|_| error(fragment.owner(), E::RecordLimit))?,
                x: fragment.bounds().x(),
                y: fragment.bounds().y(),
            },
        );
    }
    // Registry is parent-before-child. Each actual first descendant is carried
    // up once, instead of searching the whole document for each outline target.
    for node in registry.nodes().iter().rev() {
        if let (Some(parent), Some(point)) = (
            node.parent(),
            points[node.structure_node_id().get() as usize],
        ) {
            first(&mut points[parent.get() as usize], point);
        }
    }
    let mut inline = BTreeMap::new();
    for anchor in inline_anchors {
        let owner = anchor.source().source().owner();
        if inline
            .insert(
                owner,
                Point {
                    page: anchor.page_index(),
                    fragment: anchor.fragment_index(),
                    x: anchor.x(),
                    y: anchor.baseline(),
                },
            )
            .is_some()
        {
            return Err(error(owner, E::ReceiptMismatch));
        }
    }
    reserve(&mut result.destinations, source.anchors().len(), root)?;
    for (index, (_, owner)) in source.anchors().iter().enumerate() {
        let point = inline
            .get(owner)
            .copied()
            .or_else(|| nodes.get(owner).and_then(|n| points[*n]))
            .ok_or_else(|| error(*owner, E::UnplacedAnchor))?;
        result.destinations.push(ProductionBodyDestination {
            anchor_index: u32::try_from(index).map_err(|_| error(*owner, E::RecordLimit))?,
            owner: *owner,
            page_index: point.page,
            fragment_index: point.fragment,
            x: point.x,
            y: point.y,
        });
    }
    for group in groups {
        let Some(node) = nearest_link[group.node().get() as usize] else {
            continue;
        };
        let StructureOwner::Source(owner) = registry
            .node(node)
            .ok_or_else(|| error(root, E::ReceiptMismatch))?
            .owner()
        else {
            return Err(error(root, E::ReceiptMismatch));
        };
        let target = *targets
            .get(&owner)
            .ok_or_else(|| error(owner, E::ReceiptMismatch))?;
        for draw in &draws[group.draws()] {
            let (bounds, page, fragment) = match draw {
                ProductionBodyDraw::Math(m) => {
                    (Some(m.bounds()), m.page_index(), m.fragment_index())
                }
                ProductionBodyDraw::SvgFigure(r) => {
                    (Some(r.viewport()), r.page_index(), r.fragment_index())
                }
                ProductionBodyDraw::Raster(r) => {
                    (Some(r.viewport()), r.page_index(), r.fragment_index())
                }
                ProductionBodyDraw::Text(t) => {
                    (t.logical_bounds(), t.page_index(), t.fragment_index())
                }
                ProductionBodyDraw::Vector(v) => {
                    (Some(v.viewport()), v.page_index(), v.fragment_index())
                }
            };
            let Some(bounds) = bounds else {
                continue;
            };
            if let Some(last) = result.links.last_mut().filter(|last| {
                last.node == node && last.page_index == page && last.fragment_index == fragment
            }) {
                last.bounds = union(last.bounds, bounds, owner)?;
            } else {
                add_records(&mut result.additional_records, 2, base, max, owner)?;
                reserve(&mut result.links, 1, owner)?;
                let index =
                    u32::try_from(result.links.len()).map_err(|_| error(owner, E::RecordLimit))?;
                let indices = &mut result.node_links[node.get() as usize];
                reserve(indices, 1, owner)?;
                indices.push(index);
                result.links.push(ProductionBodyLink {
                    owner,
                    node,
                    target,
                    page_index: page,
                    fragment_index: fragment,
                    bounds,
                });
            }
        }
    }
    for node in registry
        .nodes()
        .iter()
        .filter(|n| n.role() == StructureRole::Link && !matches!(n.owner(),
            StructureOwner::Generated(key) if key.slot() == typaxis_layout::GeneratedStructureSlot::FootnoteLink))
    {
        if result.node_links[node.structure_node_id().get() as usize].is_empty() {
            let StructureOwner::Source(owner) = node.owner() else {
                return Err(error(root, E::ReceiptMismatch));
            };
            return Err(error(owner, E::UnplacedLink));
        }
    }
    reserve(&mut result.page_links, pages, root)?;
    let mut cursor = 0;
    for page in 0..pages {
        let start = cursor;
        while result
            .links
            .get(cursor)
            .is_some_and(|l| l.page_index as usize == page)
        {
            cursor += 1;
        }
        result.page_links.push(start..cursor);
    }
    if cursor != result.links.len() {
        return Err(error(root, E::ReceiptMismatch));
    }
    let entries = source.outline().entries();
    reserve(&mut result.outline, entries.len(), root)?;
    for (index, entry) in entries.iter().enumerate() {
        if entry.outline_id as usize != index
            || entry.parent_outline_id.is_some_and(|p| p as usize >= index)
            || source.anchor_owner(&entry.destination) != Some(entry.source.node_id)
        {
            return Err(error(entry.source.node_id, E::ReceiptMismatch));
        }
        let parent = entry.parent_outline_id;
        let siblings = parent.map_or(&mut result.outline_root, |p| {
            &mut result.outline[p as usize]
        });
        let previous = siblings.last;
        if siblings.first.is_none() {
            siblings.first = Some(entry.outline_id);
        }
        siblings.last = Some(entry.outline_id);
        if let Some(previous) = previous {
            result.outline[previous as usize].next = Some(entry.outline_id);
        }
        result.outline.push(ProductionBodyOutlineTopology {
            parent,
            previous,
            ..ProductionBodyOutlineTopology::default()
        });
    }
    for index in (0..result.outline.len()).rev() {
        let count = result.outline[index]
            .descendants
            .checked_add(1)
            .ok_or_else(|| error(root, E::RecordLimit))?;
        let parent = result.outline[index].parent;
        let target = parent.map_or(&mut result.outline_root, |p| {
            &mut result.outline[p as usize]
        });
        target.descendants = target
            .descendants
            .checked_add(count)
            .ok_or_else(|| error(root, E::RecordLimit))?;
    }
    Ok(result)
}

#[path = "production_footnote_navigation.rs"]
mod production_footnote_navigation;
pub use production_footnote_navigation::{
    build_production_footnote_reference_navigation, ProductionFootnoteDestination,
    ProductionFootnoteReferenceLink, ProductionFootnoteReferenceNavigation,
};

pub struct ProductionFootnoteNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    structure: &'n crate::ProductionFootnoteStructure<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    projection: NavigationProjection<'a>,
    footnotes: ProductionFootnoteReferenceNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    additional_records: u64,
    fingerprint: [u8; 32],
}
impl<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
    ProductionFootnoteNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>
{
    pub const fn structure(
        &self,
    ) -> &'n crate::ProductionFootnoteStructure<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.structure
    }
    pub fn footnote_references(
        &self,
    ) -> &ProductionFootnoteReferenceNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.footnotes
    }
    pub fn destinations(&self) -> &[ProductionBodyDestination] {
        &self.projection.destinations
    }
    pub fn destination_name(&self, index: u32) -> Option<&AnchorId> {
        self.projection.destinations.get(index as usize)?;
        self.structure
            .display()
            .source()
            .line_layout()
            .source_flow()
            .navigation()
            .anchors()
            .get(index as usize)
            .map(|a| &a.0)
    }
    pub fn links(&self) -> &[ProductionBodyLink<'a>] {
        &self.projection.links
    }
    pub fn node_links(&self, id: StructureNodeId) -> Option<&[u32]> {
        self.structure.registry().node(id)?;
        Some(
            self.projection
                .node_links
                .get(id.get() as usize)
                .map_or(&[], Vec::as_slice),
        )
    }
    pub fn page_links(&self, page: u32) -> Option<Range<usize>> {
        self.structure
            .display()
            .source()
            .geometry()
            .pages()
            .get(page as usize)?;
        Some(
            self.projection
                .page_links
                .get(page as usize)
                .cloned()
                .unwrap_or(0..0),
        )
    }
    pub fn outline_entries(&self) -> &[StagingOutlineEntry] {
        self.structure
            .display()
            .source()
            .line_layout()
            .source_flow()
            .navigation()
            .outline()
            .entries()
    }
    pub fn outline(&self) -> &[ProductionBodyOutlineTopology] {
        &self.projection.outline
    }
    pub const fn outline_root(&self) -> &ProductionBodyOutlineTopology {
        &self.projection.outline_root
    }
    /// Includes bounded temporary indexing work; add once to the shared marked
    /// content charge, not a second copy of its structure/display ancestors.
    pub const fn additional_records(&self) -> u64 {
        self.additional_records
    }
    pub const fn record_base(&self) -> u64 {
        self.projection.record_base
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        structure: &crate::ProductionFootnoteStructure<'_, '_, '_, '_, '_, '_, '_, '_, '_>,
    ) -> Result<(), ProductionBodyNavigationError> {
        if !std::ptr::eq(self.structure, structure) {
            return Err(error(NodeId::new(0), E::ReceiptMismatch));
        }
        Ok(())
    }
}
pub fn build_production_footnote_navigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    structure: &'n crate::ProductionFootnoteStructure<'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    retained_record_charge: u64,
) -> Result<
    ProductionFootnoteNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
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
    if retained_record_charge > limits.base().get().max_fragments {
        return Err(error(root, E::RecordLimit));
    }
    let projection = project_navigation(
        display.source().line_layout().source_flow(),
        structure.registry(),
        structure.fingerprint(),
        structure.groups(),
        display.draws(),
        display.inline_anchors(),
        display
            .source()
            .geometry()
            .pages()
            .iter()
            .flat_map(|p| p.fragments().iter().map(|f| f.fragment())),
        display.source().geometry().pages().len(),
        limits,
        retained_record_charge,
    )?;
    let next_base = retained_record_charge
        .checked_add(projection.additional_records)
        .ok_or_else(|| error(root, E::RecordLimit))?;
    let footnotes =
        build_production_footnote_reference_navigation(structure, admitted, limits, next_base)?;
    let additional_records = projection
        .additional_records
        .checked_add(footnotes.additional_records())
        .ok_or_else(|| error(root, E::RecordLimit))?;
    let mut digest = [0u8; 96];
    digest[..32].copy_from_slice(&sha256(b"typaxis.production-footnote-navigation/1"));
    digest[32..64].copy_from_slice(&projection.fingerprint);
    digest[64..].copy_from_slice(&footnotes.fingerprint());
    Ok(ProductionFootnoteNavigation {
        structure,
        projection,
        footnotes,
        additional_records,
        fingerprint: sha256(&digest),
    })
}
