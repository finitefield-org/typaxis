//! Structure ownership for the actual selected production paints.
//! This is deliberately independent of the frozen staging layout recipes.
use crate::{ProductionBodyDisplay, ProductionBodyDraw};
use std::collections::BTreeMap;
use std::ops::Range;
use typaxis_core::{sha256, M4EffectiveResourceLimits, NodeId};
use typaxis_layout::{
    build_structure_registry_v2, StructureNodeId, StructureOwner, StructureRegistryReceiptV2,
    StructureRole,
};
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_syntax::{
    StagingAccessibilityProfileAuthorizationV2, StagingBookNavigationProfileAuthorizationV2,
    ValidatedStagingStructureSemanticsV2,
};

pub const PRODUCTION_BODY_STRUCTURE_ALGORITHM: &str = "typaxis.production-body-structure/4";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBodyStructureError {
    ReceiptMismatch,
    Registry,
    MissingPaint,
    InvalidPaint,
    RecordLimit,
    SpoolLimit,
    AllocationFailure,
}

/// One contiguous source fragment on one page. Text clusters share an MCID
/// only while their source owner and selected line fragment remain the same.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionBodyStructureGroup {
    node: StructureNodeId,
    page_index: u32,
    mcid: u32,
    semantic_fragment_ordinal: u32,
    draws: Range<usize>,
    vector_usage_id: Option<u32>,
    is_text: bool,
}
impl ProductionBodyStructureGroup {
    pub const fn is_text(&self) -> bool {
        self.is_text
    }
    pub const fn node(&self) -> StructureNodeId {
        self.node
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn mcid(&self) -> u32 {
        self.mcid
    }
    pub const fn semantic_fragment_ordinal(&self) -> u32 {
        self.semantic_fragment_ordinal
    }
    pub fn draws(&self) -> Range<usize> {
        self.draws.clone()
    }
    pub const fn vector_usage_id(&self) -> Option<u32> {
        self.vector_usage_id
    }
}

pub struct ProductionBodyStructure<'v, 'd, 's, 'p, 'a> {
    display: &'v ProductionBodyDisplay<'d, 's, 'p, 'a>,
    registry: StructureRegistryReceiptV2,
    groups: Vec<ProductionBodyStructureGroup>,
    node_groups: Vec<Vec<usize>>,
    page_groups: Vec<Range<usize>>,
    record_charge: u64,
    spool_charge: u64,
    fingerprint: [u8; 32],
}
impl<'v, 'd, 's, 'p, 'a> ProductionBodyStructure<'v, 'd, 's, 'p, 'a> {
    pub const fn display(&self) -> &'v ProductionBodyDisplay<'d, 's, 'p, 'a> {
        self.display
    }
    pub const fn registry(&self) -> &StructureRegistryReceiptV2 {
        &self.registry
    }
    pub fn groups(&self) -> &[ProductionBodyStructureGroup] {
        &self.groups
    }
    /// MCRs in reading order for a structure node. The final /K array places
    /// these before its registry children (e.g. a Figure before its Caption).
    pub fn node_groups(&self, node: StructureNodeId) -> Option<&[usize]> {
        self.node_groups.get(node.get() as usize).map(Vec::as_slice)
    }
    /// Dense MCID order, also the order of the page's ParentTree entries.
    pub fn page_groups(&self, page: u32) -> Option<&[ProductionBodyStructureGroup]> {
        self.page_groups
            .get(page as usize)
            .map(|r| &self.groups[r.clone()])
    }
    /// Registry replacement text applies only to vector occurrences. The PDF
    /// marked-content owner assembles body text from this group's selected
    /// draws; repeating the registry's full source on each line duplicates it.
    pub fn group_actual_text(&self, index: usize) -> Option<&str> {
        let group = self.groups.get(index)?;
        group.vector_usage_id?;
        self.registry.node(group.node)?.actual_text()
    }
    pub const fn record_charge(&self) -> u64 {
        self.record_charge
    }
    /// Conservative retained registry strings + canonical encoding charge.
    pub const fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn verify(
        &self,
        display: &ProductionBodyDisplay<'_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), ProductionBodyStructureError> {
        if !std::ptr::eq(self.display, display) {
            return Err(ProductionBodyStructureError::ReceiptMismatch);
        }
        display
            .verify_resources(admitted, limits)
            .map_err(|_| ProductionBodyStructureError::ReceiptMismatch)
    }
}

pub fn build_production_body_structure<'v, 'd, 's, 'p, 'a>(
    display: &'v ProductionBodyDisplay<'d, 's, 'p, 'a>,
    semantics: &ValidatedStagingStructureSemanticsV2,
    accessibility: &StagingAccessibilityProfileAuthorizationV2,
    navigation: &StagingBookNavigationProfileAuthorizationV2,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<ProductionBodyStructure<'v, 'd, 's, 'p, 'a>, ProductionBodyStructureError> {
    use ProductionBodyStructureError as E;
    display
        .verify_resources(admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    let flow = display.selected().line_layout().source_flow();
    let epoch = display.selected().line_layout().binding_epoch();
    navigation
        .authorizes(flow.package(), flow.navigation(), limits)
        .map_err(|_| E::ReceiptMismatch)?;
    if navigation.precomposed_vector_profile_fingerprint()
        != epoch.profile_authorization_fingerprint()
        || navigation.precomposed_vector_profile_receipt_fingerprint()
            != epoch.profile_fingerprint()
        || accessibility.book_navigation_profile_fingerprint()
            != navigation.profile_receipt_fingerprint()
    {
        return Err(E::ReceiptMismatch);
    }
    let registry = build_structure_registry_v2(
        flow.package(),
        flow.navigation(),
        semantics,
        accessibility,
        limits,
    )
    .map_err(|_| E::Registry)?;
    let spool_charge = (registry.canonical_jcs().len() as u64)
        .checked_mul(2)
        .ok_or(E::SpoolLimit)?;
    if spool_charge > limits.base().get().max_spool_bytes {
        return Err(E::SpoolLimit);
    }
    // Charge index, coverage and group records before allocating. The registry
    // separately enforces its source/generated node and derived-text budgets.
    let added = (registry.nodes().len() as u64)
        .checked_mul(3)
        .and_then(|v| v.checked_add((display.draws().len() as u64).checked_mul(3)?))
        .and_then(|v| v.checked_add(display.selected().pages().len() as u64))
        .ok_or(E::RecordLimit)?;
    let record_charge = display
        .record_charge()
        .checked_add(added)
        .ok_or(E::RecordLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    let mut source_nodes = BTreeMap::<NodeId, StructureNodeId>::new();
    let mut label_nodes = BTreeMap::<NodeId, StructureNodeId>::new();
    let mut node_groups = Vec::new();
    node_groups
        .try_reserve_exact(registry.nodes().len())
        .map_err(|_| E::AllocationFailure)?;
    for node in registry.nodes() {
        if let StructureOwner::Source(source) = node.owner() {
            source_nodes.insert(source, node.structure_node_id());
        }
        if let StructureOwner::Generated(key) = node.owner() {
            if key.slot() == typaxis_layout::GeneratedStructureSlot::ListLabel && key.ordinal() == 0
            {
                label_nodes.insert(key.owner_node_id(), node.structure_node_id());
            }
        }
        node_groups.push(Vec::new());
    }
    let mut groups: Vec<ProductionBodyStructureGroup> = Vec::new();
    groups
        .try_reserve_exact(display.draws().len())
        .map_err(|_| E::AllocationFailure)?;
    let mut page_groups = Vec::new();
    page_groups
        .try_reserve_exact(display.selected().pages().len())
        .map_err(|_| E::AllocationFailure)?;
    let mut draw_index = 0usize;
    let mut vector_usage = 0u32;
    for page in 0..display.selected().pages().len() {
        let page_index = u32::try_from(page).map_err(|_| E::RecordLimit)?;
        let page_start = groups.len();
        let mut previous_fragment = None;
        while let Some(draw) = display.draws().get(draw_index) {
            let (source, draw_page, fragment) = match draw {
                ProductionBodyDraw::Text(t) => (t.owner(), t.page_index(), t.fragment_index()),
                ProductionBodyDraw::Raster(r) => (r.owner(), r.page_index(), r.fragment_index()),
                ProductionBodyDraw::Vector(v) => {
                    (v.binding().node_id(), v.page_index(), v.fragment_index())
                }
            };
            if draw_page > page_index {
                break;
            }
            if draw_page != page_index {
                return Err(E::InvalidPaint);
            }
            let id = if matches!(draw,ProductionBodyDraw::Text(t) if t.generated_provenance().is_some())
            {
                *label_nodes.get(&source).ok_or(E::InvalidPaint)?
            } else {
                *source_nodes.get(&source).ok_or(E::InvalidPaint)?
            };
            let node = registry.node(id).ok_or(E::InvalidPaint)?;
            if !node.paint_required() {
                return Err(E::InvalidPaint);
            }
            let usage = match draw {
                ProductionBodyDraw::Raster(r) => {
                    if node.role() != StructureRole::Figure
                        || node.vector_binding_v2().is_some()
                        || node.alternative() != Some(r.alternative())
                        || !node_groups[id.get() as usize].is_empty()
                    {
                        return Err(E::InvalidPaint);
                    }
                    None
                }
                ProductionBodyDraw::Text(t) => {
                    if node.vector_binding_v2().is_some()
                        || (node.actual_text().is_none()
                            && node.equation_number_binding_v2().is_none())
                    {
                        return Err(E::InvalidPaint);
                    }
                    match (node.equation_number_binding_v2(), t.equation_number()) {
                        (None, None) => (),
                        (Some(binding), Some(shape)) => {
                            if binding.parent_owner() != shape.owner()
                                || binding.text_span() != shape.text_span()
                                || binding.text_buffer_sha256() != shape.text_buffer_sha256()
                                || binding.exact_text() != shape.exact_text()
                                || binding.exact_text_sha256() != shape.exact_text_sha256()
                                || node.actual_text().is_some()
                                || t.generated_provenance().is_some()
                            {
                                return Err(E::InvalidPaint);
                            }
                        }
                        _ => return Err(E::InvalidPaint),
                    }
                    match t.generated_provenance() {
                        None if node.role() != StructureRole::Span => return Err(E::InvalidPaint),
                        None => (),
                        Some(provenance) => {
                            let index = flow
                                .list_items()
                                .binary_search_by_key(&source, |i| i.owner())
                                .map_err(|_| E::InvalidPaint)?;
                            let item = &flow.list_items()[index];
                            let text = flow.list_marker_text(index).ok_or(E::InvalidPaint)?;
                            let range = provenance.text_span().range();
                            if node.role() != StructureRole::Label
                                || provenance.buffer_key() != item.key()
                                || node.actual_text() != Some(text)
                                || text.get(
                                    range.start_byte().get() as usize
                                        ..range.end_byte().get() as usize,
                                ) != Some(t.exact_text())
                            {
                                return Err(E::InvalidPaint);
                            }
                        }
                    }
                    None
                }
                ProductionBodyDraw::Vector(v) => {
                    let expected = node.vector_binding_v2().ok_or(E::InvalidPaint)?;
                    if expected.kind() != v.binding().kind()
                        || expected.metrics_fingerprint() != v.binding().metrics_fingerprint()
                        || node.alternative() != Some(v.binding().alternative())
                        || v.math_binding()
                            .is_some_and(|m| node.actual_text() != Some(m.resolved_actual_text()))
                        || v.binding()
                            .language()
                            .is_some_and(|lang| node.language() != lang)
                        || !node_groups[id.get() as usize].is_empty()
                    {
                        return Err(E::InvalidPaint);
                    }
                    let usage = vector_usage;
                    vector_usage = vector_usage.checked_add(1).ok_or(E::RecordLimit)?;
                    Some(usage)
                }
            };
            let merge = matches!(draw, ProductionBodyDraw::Text(_))
                && previous_fragment == Some((id, fragment))
                && groups
                    .last()
                    .is_some_and(|g| g.page_index == page_index && g.is_text);
            if merge {
                groups.last_mut().ok_or(E::InvalidPaint)?.draws.end = draw_index + 1;
            } else {
                let indices = &mut node_groups[id.get() as usize];
                let ordinal = u32::try_from(indices.len()).map_err(|_| E::RecordLimit)?;
                indices.try_reserve(1).map_err(|_| E::AllocationFailure)?;
                indices.push(groups.len());
                groups.push(ProductionBodyStructureGroup {
                    node: id,
                    page_index,
                    mcid: u32::try_from(groups.len() - page_start).map_err(|_| E::RecordLimit)?,
                    semantic_fragment_ordinal: ordinal,
                    draws: draw_index..draw_index + 1,
                    vector_usage_id: usage,
                    is_text: matches!(draw, ProductionBodyDraw::Text(_)),
                });
            }
            previous_fragment = Some((id, fragment));
            draw_index += 1;
        }
        page_groups.push(page_start..groups.len());
    }
    if draw_index != display.draws().len() {
        return Err(E::InvalidPaint);
    }
    for node in registry.nodes() {
        if node.paint_required() && node_groups[node.structure_node_id().get() as usize].is_empty()
        {
            return Err(E::MissingPaint);
        }
        if node.role() == StructureRole::Label {
            let owned = &node_groups[node.structure_node_id().get() as usize];
            if owned.len() != 1 {
                return Err(E::InvalidPaint);
            }
            let mut end = 0;
            for draw in &display.draws()[groups[owned[0]].draws()] {
                let ProductionBodyDraw::Text(t) = draw else {
                    return Err(E::InvalidPaint);
                };
                let provenance = t.generated_provenance().ok_or(E::InvalidPaint)?;
                let range = provenance.text_span().range();
                if range.start_byte().get() != end {
                    return Err(E::InvalidPaint);
                }
                end = range.end_byte().get();
            }
            if end as usize != node.actual_text().ok_or(E::InvalidPaint)?.len() {
                return Err(E::InvalidPaint);
            }
        }
    }
    // All derived records are deterministic projections of these sealed owners.
    let mut digest = [0u8; 96];
    digest[..32].copy_from_slice(&sha256(PRODUCTION_BODY_STRUCTURE_ALGORITHM.as_bytes()));
    digest[32..64].copy_from_slice(&display.fingerprint());
    digest[64..].copy_from_slice(&registry.fingerprint());
    Ok(ProductionBodyStructure {
        display,
        registry,
        groups,
        node_groups,
        page_groups,
        record_charge,
        spool_charge,
        fingerprint: sha256(&digest),
    })
}
