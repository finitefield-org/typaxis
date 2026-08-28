use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{push_jcs_string, sha256, ValidatedResourceLimits};
use typaxis_layout::{
    SelectedStructureBindingReceipt, SelectedStructurePaint, SelectedStructurePaintOwner,
    StructureArtifactClass, StructureNodeId, StructureRegistryReceipt, StructureRole,
};
use typaxis_syntax::StagingAccessibilityProfileAuthorization;

pub const MARKED_CONTENT_PLAN_ALGORITHM: &str = "typaxis.marked-content-plan/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkedContentPage {
    page_index: u32,
    width_raw: i64,
    height_raw: i64,
    structure_parent_key: Option<u32>,
    marked_content_count: u32,
    artifact_count: u32,
}

impl MarkedContentPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn width_raw(&self) -> i64 {
        self.width_raw
    }
    pub const fn height_raw(&self) -> i64 {
        self.height_raw
    }
    pub const fn structure_parent_key(&self) -> Option<u32> {
        self.structure_parent_key
    }
    pub const fn marked_content_count(&self) -> u32 {
        self.marked_content_count
    }
    pub const fn artifact_count(&self) -> u32 {
        self.artifact_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkedContentStructure {
    structure_node_id: StructureNodeId,
    role: StructureRole,
    mcid: u32,
}

impl MarkedContentStructure {
    pub const fn structure_node_id(&self) -> StructureNodeId {
        self.structure_node_id
    }
    pub const fn role(&self) -> StructureRole {
        self.role
    }
    pub const fn mcid(&self) -> u32 {
        self.mcid
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkedContentArtifact {
    class: StructureArtifactClass,
    occurrence: u32,
}

impl MarkedContentArtifact {
    pub const fn class(&self) -> StructureArtifactClass {
        self.class
    }
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkedContentOwner {
    Structure(MarkedContentStructure),
    Artifact(MarkedContentArtifact),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkedContentRecord {
    selected_paint_ids: Vec<u32>,
    page_index: u32,
    paint_ordinal_start: u32,
    semantic_fragment_ordinal: u32,
    owner: MarkedContentOwner,
    language: Option<String>,
    actual_text: Option<String>,
}

impl MarkedContentRecord {
    pub fn selected_paint_ids(&self) -> &[u32] {
        &self.selected_paint_ids
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn paint_ordinal_start(&self) -> u32 {
        self.paint_ordinal_start
    }
    pub const fn semantic_fragment_ordinal(&self) -> u32 {
        self.semantic_fragment_ordinal
    }
    pub const fn owner(&self) -> MarkedContentOwner {
        self.owner
    }
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }
    pub fn actual_text(&self) -> Option<&str> {
        self.actual_text.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureLinkAnnotation {
    annotation_id: u32,
    page_index: u32,
    annotation_ordinal: u32,
    structure_node_id: StructureNodeId,
    structure_parent_key: u32,
    accessible_name: String,
}

impl StructureLinkAnnotation {
    pub const fn annotation_id(&self) -> u32 {
        self.annotation_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn annotation_ordinal(&self) -> u32 {
        self.annotation_ordinal
    }
    pub const fn structure_node_id(&self) -> StructureNodeId {
        self.structure_node_id
    }
    pub const fn structure_parent_key(&self) -> u32 {
        self.structure_parent_key
    }
    pub fn accessible_name(&self) -> &str {
        &self.accessible_name
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructureParentTreeValue {
    Page(Vec<StructureNodeId>),
    Annotation(StructureNodeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructureParentTreeEntry {
    key: u32,
    value: StructureParentTreeValue,
}

impl StructureParentTreeEntry {
    pub const fn key(&self) -> u32 {
        self.key
    }
    pub const fn value(&self) -> &StructureParentTreeValue {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkedContentPlanReceipt {
    structure_registry_sha256: [u8; 32],
    selected_binding_sha256: [u8; 32],
    authorization_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    pages: Vec<MarkedContentPage>,
    records: Vec<MarkedContentRecord>,
    annotations: Vec<StructureLinkAnnotation>,
    parent_tree: Vec<StructureParentTreeEntry>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl MarkedContentPlanReceipt {
    pub const fn structure_registry_sha256(&self) -> [u8; 32] {
        self.structure_registry_sha256
    }
    pub const fn selected_binding_sha256(&self) -> [u8; 32] {
        self.selected_binding_sha256
    }
    pub const fn authorization_sha256(&self) -> [u8; 32] {
        self.authorization_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub fn pages(&self) -> &[MarkedContentPage] {
        &self.pages
    }
    pub fn records(&self) -> &[MarkedContentRecord] {
        &self.records
    }
    pub fn annotations(&self) -> &[StructureLinkAnnotation] {
        &self.annotations
    }
    pub fn parent_tree(&self) -> &[StructureParentTreeEntry] {
        &self.parent_tree
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        registry: &StructureRegistryReceipt,
        binding: &SelectedStructureBindingReceipt,
        authorization: &StagingAccessibilityProfileAuthorization,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), MarkedContentError> {
        let observed = build_marked_content_plan(registry, binding, authorization, limits)?;
        if self != &observed {
            return Err(MarkedContentError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkedContentError {
    RegistryMismatch,
    BindingMismatch,
    MissingPaint,
    ExtraPaint,
    PaintOrder,
    FragmentLimit,
    McidOverflow,
    ArtifactMismatch,
    ParentTreeMismatch,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for MarkedContentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RegistryMismatch => {
                formatter.write_str("I9190: marked-content registry mismatch")
            }
            Self::BindingMismatch => {
                formatter.write_str("I9190: selected structure binding mismatch")
            }
            Self::MissingPaint => formatter.write_str("I9190: marked content is missing"),
            Self::ExtraPaint => formatter.write_str("I9190: marked content has no selected paint"),
            Self::PaintOrder => formatter.write_str("I9190: marked-content order mismatch"),
            Self::FragmentLimit => {
                formatter.write_str("L5110: marked-content fragment limit exceeded")
            }
            Self::McidOverflow => formatter.write_str("L5110: page-local MCID limit exceeded"),
            Self::ArtifactMismatch => {
                formatter.write_str("I9190: artifact classification mismatch")
            }
            Self::ParentTreeMismatch => {
                formatter.write_str("I9190: structure parent tree mismatch")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5110: marked-content allocation failed")
            }
            Self::ReceiptMismatch => formatter.write_str("I9190: marked-content receipt mismatch"),
        }
    }
}

impl std::error::Error for MarkedContentError {}

pub fn build_marked_content_plan(
    registry: &StructureRegistryReceipt,
    binding: &SelectedStructureBindingReceipt,
    authorization: &StagingAccessibilityProfileAuthorization,
    limits: &ValidatedResourceLimits,
) -> Result<MarkedContentPlanReceipt, MarkedContentError> {
    binding
        .verify(registry, authorization, limits)
        .map_err(|_| MarkedContentError::BindingMismatch)?;
    if registry.fingerprint() != binding.structure_registry_sha256()
        || authorization.fingerprint() != binding.authorization_sha256()
        || authorization.view().limits_sha256() != binding.limits_sha256()
    {
        return Err(MarkedContentError::RegistryMismatch);
    }

    let group_count = binding
        .paints()
        .iter()
        .enumerate()
        .filter(|(index, paint)| {
            *index == 0 || !same_marked_group(&binding.paints()[index - 1], paint)
        })
        .count();
    let charged_fragments = binding
        .selected_layout_fragment_count()
        .checked_add(u64::try_from(group_count).map_err(|_| MarkedContentError::FragmentLimit)?)
        .ok_or(MarkedContentError::FragmentLimit)?;
    if charged_fragments > limits.get().max_fragments {
        return Err(MarkedContentError::FragmentLimit);
    }
    let mut records = Vec::new();
    records
        .try_reserve_exact(group_count)
        .map_err(|_| MarkedContentError::AllocationFailure)?;
    let mut mcids = BTreeMap::<u32, u32>::new();
    let mut artifact_counts = BTreeMap::<u32, u32>::new();
    let mut artifact_keys = BTreeSet::new();
    let mut page_arrays = binding
        .pages()
        .iter()
        .map(|page| (page.page_index, Vec::new()))
        .collect::<BTreeMap<_, Vec<StructureNodeId>>>();

    let mut start = 0usize;
    while start < binding.paints().len() {
        let paint = &binding.paints()[start];
        let mut end = start + 1;
        while end < binding.paints().len() && same_marked_group(paint, &binding.paints()[end]) {
            end += 1;
        }
        let mut selected_paint_ids = Vec::new();
        selected_paint_ids
            .try_reserve_exact(end - start)
            .map_err(|_| MarkedContentError::AllocationFailure)?;
        for (offset, grouped_paint) in binding.paints()[start..end].iter().enumerate() {
            if grouped_paint.selected_paint_id()
                != paint
                    .selected_paint_id()
                    .checked_add(u32::try_from(offset).map_err(|_| MarkedContentError::PaintOrder)?)
                    .ok_or(MarkedContentError::PaintOrder)?
                || grouped_paint.paint_ordinal()
                    != paint
                        .paint_ordinal()
                        .checked_add(
                            u32::try_from(offset).map_err(|_| MarkedContentError::PaintOrder)?,
                        )
                        .ok_or(MarkedContentError::PaintOrder)?
            {
                return Err(MarkedContentError::PaintOrder);
            }
            selected_paint_ids.push(grouped_paint.selected_paint_id());
        }
        let owner = match paint.owner() {
            SelectedStructurePaintOwner::Structure(structure_node_id) => {
                let node = registry
                    .node(structure_node_id)
                    .ok_or(MarkedContentError::ExtraPaint)?;
                if paint.role() != Some(node.role()) {
                    return Err(MarkedContentError::ExtraPaint);
                }
                let mcid = *mcids.entry(paint.page_index()).or_insert(0);
                if mcid > i32::MAX as u32 {
                    return Err(MarkedContentError::McidOverflow);
                }
                *mcids
                    .get_mut(&paint.page_index())
                    .ok_or(MarkedContentError::McidOverflow)? = mcid
                    .checked_add(1)
                    .ok_or(MarkedContentError::McidOverflow)?;
                page_arrays
                    .get_mut(&paint.page_index())
                    .ok_or(MarkedContentError::ParentTreeMismatch)?
                    .push(structure_node_id);
                MarkedContentOwner::Structure(MarkedContentStructure {
                    structure_node_id,
                    role: node.role(),
                    mcid,
                })
            }
            SelectedStructurePaintOwner::Artifact { class, occurrence } => {
                if !artifact_keys.insert((class, occurrence)) {
                    return Err(MarkedContentError::ArtifactMismatch);
                }
                let count = artifact_counts.entry(paint.page_index()).or_insert(0);
                *count = count
                    .checked_add(1)
                    .ok_or(MarkedContentError::McidOverflow)?;
                MarkedContentOwner::Artifact(MarkedContentArtifact { class, occurrence })
            }
        };
        records.push(MarkedContentRecord {
            selected_paint_ids,
            page_index: paint.page_index(),
            paint_ordinal_start: paint.paint_ordinal(),
            semantic_fragment_ordinal: paint.semantic_fragment_ordinal(),
            owner,
            language: paint.language().map(str::to_owned),
            actual_text: paint.actual_text().map(str::to_owned),
        });
        start = end;
    }
    if records.len() != group_count
        || records
            .iter()
            .map(|record| record.selected_paint_ids.len())
            .sum::<usize>()
            != binding.paints().len()
    {
        return Err(MarkedContentError::MissingPaint);
    }

    let mut next_parent_key = 0u32;
    let mut parent_tree = Vec::new();
    let pages = binding
        .pages()
        .iter()
        .map(|page| {
            let marked_content_count = mcids.get(&page.page_index).copied().unwrap_or(0);
            let structure_parent_key = if marked_content_count == 0 {
                None
            } else {
                let key = next_parent_key;
                next_parent_key = next_parent_key
                    .checked_add(1)
                    .ok_or(MarkedContentError::ParentTreeMismatch)?;
                let nodes = page_arrays
                    .remove(&page.page_index)
                    .ok_or(MarkedContentError::ParentTreeMismatch)?;
                parent_tree.push(StructureParentTreeEntry {
                    key,
                    value: StructureParentTreeValue::Page(nodes),
                });
                Some(key)
            };
            Ok(MarkedContentPage {
                page_index: page.page_index,
                width_raw: page.width_raw,
                height_raw: page.height_raw,
                structure_parent_key,
                marked_content_count,
                artifact_count: artifact_counts.get(&page.page_index).copied().unwrap_or(0),
            })
        })
        .collect::<Result<Vec<_>, MarkedContentError>>()?;
    if page_arrays.values().any(|nodes| !nodes.is_empty()) {
        return Err(MarkedContentError::ParentTreeMismatch);
    }
    let mut annotations = Vec::new();
    annotations
        .try_reserve_exact(binding.annotations().len())
        .map_err(|_| MarkedContentError::AllocationFailure)?;
    for annotation in binding.annotations() {
        let structure_parent_key = next_parent_key
            .checked_add(annotation.annotation_id())
            .ok_or(MarkedContentError::ParentTreeMismatch)?;
        annotations.push(StructureLinkAnnotation {
            annotation_id: annotation.annotation_id(),
            page_index: annotation.page_index(),
            annotation_ordinal: annotation.annotation_ordinal(),
            structure_node_id: annotation.structure_node_id(),
            structure_parent_key,
            accessible_name: annotation.accessible_name().to_owned(),
        });
    }
    parent_tree.extend(
        annotations
            .iter()
            .map(|annotation| StructureParentTreeEntry {
                key: annotation.structure_parent_key,
                value: StructureParentTreeValue::Annotation(annotation.structure_node_id),
            }),
    );
    if parent_tree
        .iter()
        .enumerate()
        .any(|(index, entry)| usize::try_from(entry.key) != Ok(index))
    {
        return Err(MarkedContentError::ParentTreeMismatch);
    }

    let mut receipt = MarkedContentPlanReceipt {
        structure_registry_sha256: registry.fingerprint(),
        selected_binding_sha256: binding.fingerprint(),
        authorization_sha256: authorization.fingerprint(),
        limits_sha256: binding.limits_sha256(),
        pages,
        records,
        annotations,
        parent_tree,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_plan(&receipt);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    Ok(receipt)
}

fn same_marked_group(left: &SelectedStructurePaint, right: &SelectedStructurePaint) -> bool {
    left.page_index() == right.page_index()
        && left.semantic_fragment_ordinal() == right.semantic_fragment_ordinal()
        && left.owner() == right.owner()
        && left.role() == right.role()
        && left.language() == right.language()
        && left.actual_text() == right.actual_text()
}

fn encode_plan(value: &MarkedContentPlanReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MARKED_CONTENT_PLAN_ALGORITHM);
    output.push_str(",\"annotations\":[");
    for (index, annotation) in value.annotations.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"accessible_name\":");
        push_jcs_string(&mut output, &annotation.accessible_name);
        output.push_str(",\"annotation_id\":");
        output.push_str(&annotation.annotation_id.to_string());
        output.push_str(",\"annotation_ordinal\":");
        output.push_str(&annotation.annotation_ordinal.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&annotation.page_index.to_string());
        output.push_str(",\"structure_node_id\":");
        output.push_str(&annotation.structure_node_id.get().to_string());
        output.push_str(",\"structure_parent_key\":");
        output.push_str(&annotation.structure_parent_key.to_string());
        output.push('}');
    }
    output.push_str("],\"authorization_sha256\":");
    push_hash(&mut output, value.authorization_sha256);
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, value.limits_sha256);
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"artifact_count\":");
        output.push_str(&page.artifact_count.to_string());
        output.push_str(",\"height_raw\":");
        output.push_str(&page.height_raw.to_string());
        output.push_str(",\"marked_content_count\":");
        output.push_str(&page.marked_content_count.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"structure_parent_key\":");
        if let Some(key) = page.structure_parent_key {
            output.push_str(&key.to_string());
        } else {
            output.push_str("null");
        }
        output.push_str(",\"width_raw\":");
        output.push_str(&page.width_raw.to_string());
        output.push('}');
    }
    output.push_str("],\"parent_tree\":[");
    for (index, entry) in value.parent_tree.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"key\":");
        output.push_str(&entry.key.to_string());
        match &entry.value {
            StructureParentTreeValue::Page(nodes) => {
                output.push_str(",\"kind\":\"page\",\"structure_node_ids\":[");
                for (node_index, node) in nodes.iter().enumerate() {
                    if node_index != 0 {
                        output.push(',');
                    }
                    output.push_str(&node.get().to_string());
                }
                output.push(']');
            }
            StructureParentTreeValue::Annotation(node) => {
                output.push_str(",\"kind\":\"annotation\",\"structure_node_id\":");
                output.push_str(&node.get().to_string());
            }
        }
        output.push('}');
    }
    output.push_str("],\"records\":[");
    for (index, record) in value.records.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"actual_text\":");
        push_optional(&mut output, record.actual_text.as_deref());
        output.push_str(",\"language\":");
        push_optional(&mut output, record.language.as_deref());
        output.push_str(",\"owner\":");
        match record.owner {
            MarkedContentOwner::Structure(owner) => {
                output.push_str("{\"kind\":\"structure\",\"mcid\":");
                output.push_str(&owner.mcid.to_string());
                output.push_str(",\"role\":");
                push_jcs_string(&mut output, owner.role.pdf_name());
                output.push_str(",\"structure_node_id\":");
                output.push_str(&owner.structure_node_id.get().to_string());
                output.push('}');
            }
            MarkedContentOwner::Artifact(owner) => {
                output.push_str("{\"class\":");
                push_jcs_string(&mut output, owner.class.as_str());
                output.push_str(",\"kind\":\"artifact\",\"occurrence\":");
                output.push_str(&owner.occurrence.to_string());
                output.push('}');
            }
        }
        output.push_str(",\"page_index\":");
        output.push_str(&record.page_index.to_string());
        output.push_str(",\"paint_ordinal_start\":");
        output.push_str(&record.paint_ordinal_start.to_string());
        output.push_str(",\"selected_paint_ids\":[");
        for (paint_index, paint_id) in record.selected_paint_ids.iter().enumerate() {
            if paint_index != 0 {
                output.push(',');
            }
            output.push_str(&paint_id.to_string());
        }
        output.push(']');
        output.push_str(",\"semantic_fragment_ordinal\":");
        output.push_str(&record.semantic_fragment_ordinal.to_string());
        output.push('}');
    }
    output.push_str("],\"selected_binding_sha256\":");
    push_hash(&mut output, value.selected_binding_sha256);
    output.push_str(",\"structure_registry_sha256\":");
    push_hash(&mut output, value.structure_registry_sha256);
    output.push('}');
    output
}

fn push_optional(output: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        push_jcs_string(output, value);
    } else {
        output.push_str("null");
    }
}

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_layout::{
        build_structure_registry, select_structure_bindings, SelectedStructureAnnotationInput,
        SelectedStructurePage, SelectedStructurePaintInput, SelectedStructurePaintOwner,
        StructureOwner, StructureRole,
    };
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::{
        validate_staging_book_navigation, validate_staging_structure_semantics,
        StagingAccessibilityProfileView, StagingSemanticPackageParser,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/accessibility/job/document-package.json"
    ));

    #[test]
    fn tagged_structure_assigns_dense_page_local_mcids_and_closes_artifacts() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let navigation = validate_staging_book_navigation(&package, &limits).unwrap();
        let semantics = validate_staging_structure_semantics(&package, &navigation).unwrap();
        let view = StagingAccessibilityProfileView::new(&package, &navigation, &semantics).unwrap();
        let authorization = StagingAccessibilityProfileAuthorization::bind_profile_receipt(
            view,
            sha256(b"tagged-structure-test-profile"),
            &package,
            &navigation,
            &semantics,
        )
        .unwrap();
        let registry =
            build_structure_registry(&package, &navigation, &semantics, &authorization, &limits)
                .unwrap();
        assert_eq!(registry.nodes()[0].role(), StructureRole::Document);
        assert!(registry.generated_node_count() > 0);
        let table = registry
            .nodes()
            .iter()
            .find(|node| node.role() == StructureRole::Table)
            .unwrap();
        let head = registry.node(table.children()[0]).unwrap();
        let body = registry.node(table.children()[1]).unwrap();
        assert_eq!(head.role(), StructureRole::TableHead);
        assert_eq!(body.role(), StructureRole::TableBody);
        assert!(head
            .children()
            .iter()
            .all(|child| *child < body.structure_node_id()));
        for generated in registry
            .nodes()
            .iter()
            .filter(|node| matches!(node.owner(), StructureOwner::Generated(_)))
        {
            let StructureOwner::Generated(key) = generated.owner() else {
                unreachable!();
            };
            assert_eq!(
                generated.source_span(),
                registry
                    .source_node(key.owner_node_id())
                    .unwrap()
                    .source_span()
            );
        }
        let note = registry
            .nodes()
            .iter()
            .find(|node| node.role() == StructureRole::Note)
            .unwrap();
        assert!(!note.related_nodes().is_empty());
        for reference_id in note.related_nodes() {
            let reference = registry.node(*reference_id).unwrap();
            assert_eq!(reference.role(), StructureRole::Reference);
            assert_eq!(reference.related_nodes(), &[note.structure_node_id()]);
        }

        let mut owners = registry
            .nodes()
            .iter()
            .filter(|node| node.paint_required())
            .map(|node| (node.structure_node_id(), 0u32))
            .collect::<Vec<_>>();
        let split = owners[0].0;
        owners.insert(1, (split, 0));
        owners.insert(2, (split, 1));
        let mut paints = owners
            .iter()
            .enumerate()
            .map(|(index, (owner, fragment))| SelectedStructurePaintInput {
                selected_paint_id: index as u32,
                page_index: 0,
                paint_ordinal: index as u32,
                semantic_fragment_ordinal: *fragment,
                owner: SelectedStructurePaintOwner::Structure(*owner),
            })
            .collect::<Vec<_>>();
        for (class, occurrence) in [
            (StructureArtifactClass::Pagination, 0),
            (StructureArtifactClass::PaginationHeader, 0),
            (StructureArtifactClass::PaginationFooter, 0),
            (StructureArtifactClass::Layout, 0),
        ] {
            let id = paints.len() as u32;
            paints.push(SelectedStructurePaintInput {
                selected_paint_id: id,
                page_index: 0,
                paint_ordinal: id,
                semantic_fragment_ordinal: 0,
                owner: SelectedStructurePaintOwner::Artifact { class, occurrence },
            });
        }
        let annotations = registry
            .nodes()
            .iter()
            .filter(|node| node.role() == StructureRole::Link)
            .enumerate()
            .map(|(index, node)| SelectedStructureAnnotationInput {
                annotation_id: index as u32,
                page_index: 0,
                annotation_ordinal: index as u32,
                owner_node_id: match node.owner() {
                    StructureOwner::Source(source) => source,
                    StructureOwner::Generated(_) => panic!("Link cannot be generated"),
                },
            })
            .collect::<Vec<_>>();
        let pages = [
            SelectedStructurePage {
                page_index: 0,
                width_raw: 30_000_000,
                height_raw: 20_000_000,
            },
            SelectedStructurePage {
                page_index: 1,
                width_raw: 30_000_000,
                height_raw: 20_000_000,
            },
        ];
        let mut cross_page_duplicate = owners
            .iter()
            .enumerate()
            .map(|(index, (owner, fragment))| SelectedStructurePaintInput {
                selected_paint_id: index as u32,
                page_index: 0,
                paint_ordinal: index as u32,
                semantic_fragment_ordinal: *fragment,
                owner: SelectedStructurePaintOwner::Structure(*owner),
            })
            .collect::<Vec<_>>();
        let duplicate_owner = cross_page_duplicate
            .last()
            .map(|paint| paint.owner)
            .unwrap();
        cross_page_duplicate.push(SelectedStructurePaintInput {
            selected_paint_id: cross_page_duplicate.len() as u32,
            page_index: 1,
            paint_ordinal: 0,
            semantic_fragment_ordinal: 0,
            owner: duplicate_owner,
        });
        assert_eq!(
            select_structure_bindings(
                &registry,
                &authorization,
                &limits,
                sha256(b"cross-page-duplicate-fragment"),
                cross_page_duplicate.len() as u64,
                &pages,
                &cross_page_duplicate,
                &annotations,
            ),
            Err(typaxis_layout::SelectedStructureBindingError::PaintOrder)
        );
        let binding = select_structure_bindings(
            &registry,
            &authorization,
            &limits,
            sha256(b"tagged-selected-layout"),
            (paints.len() - 1) as u64,
            &pages,
            &paints,
            &annotations,
        )
        .unwrap();
        let plan = build_marked_content_plan(&registry, &binding, &authorization, &limits).unwrap();
        plan.verify(&registry, &binding, &authorization, &limits)
            .unwrap();
        assert_eq!(plan.pages()[0].artifact_count(), 4);
        assert_eq!(plan.pages()[0].structure_parent_key(), Some(0));
        assert_eq!(plan.pages()[1].structure_parent_key(), None);
        assert_eq!(plan.records().len(), paints.len() - 1);
        assert_eq!(plan.records()[0].selected_paint_ids(), &[0, 1]);
        let mcids = plan
            .records()
            .iter()
            .filter_map(|record| match record.owner() {
                MarkedContentOwner::Structure(owner) => Some(owner.mcid()),
                MarkedContentOwner::Artifact(_) => None,
            })
            .collect::<Vec<_>>();
        assert!(mcids.iter().copied().eq(0..mcids.len() as u32));
        assert_eq!(plan.parent_tree().len(), 1 + annotations.len());

        let mut tampered = plan.clone();
        let MarkedContentOwner::Structure(ref mut owner) = tampered.records[0].owner else {
            panic!("first record must be structure");
        };
        owner.mcid = 7;
        assert_eq!(
            tampered.verify(&registry, &binding, &authorization, &limits),
            Err(MarkedContentError::ReceiptMismatch)
        );
    }
}
