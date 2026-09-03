use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{
    push_jcs_string, sha256, M4EffectiveResourceLimits, NodeId, ValidatedResourceLimits,
};
use typaxis_layout::{
    select_structure_bindings_v2, SelectedEquationNumberPaintBindingV2,
    SelectedStructureAnnotationInput, SelectedStructureBindingError,
    SelectedStructureBindingReceipt, SelectedStructureBindingReceiptV2, SelectedStructurePage,
    SelectedStructurePaint, SelectedStructurePaintBindingV2, SelectedStructurePaintInputV2,
    SelectedStructurePaintOwner, SelectedStructurePaintV2, SelectedVectorPaintBindingV2,
    StagingMathVectorFlowRegistry, StructureArtifactClass, StructureNodeId, StructureOwner,
    StructureRegistryReceipt, StructureRegistryReceiptV2, StructureRole,
};
use typaxis_pagination::StagingAtomicVectorBlockSelectedLayout;
use typaxis_shaping::StagingEquationNumberShapeReceipt;
use typaxis_syntax::{
    PrecomposedVectorKind, StagingAccessibilityProfileAuthorization,
    StagingAccessibilityProfileAuthorizationV2, StagingBookNavigationProfileAuthorizationV2,
    ValidatedStagingBookNavigationV2,
};

use crate::{
    BookNavigationSelectedReceiptV2, StagingPrecomposedVectorDisplay,
    VectorFormStructureIsolationReceiptV2,
};

pub const MARKED_CONTENT_PLAN_ALGORITHM: &str = "typaxis.marked-content-plan/1";
pub const MARKED_CONTENT_PLAN_ALGORITHM_V2: &str = "typaxis.marked-content-plan/2";

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
pub struct MarkedContentInnerSpanV2 {
    actual_text: Option<String>,
    language: Option<String>,
}

impl MarkedContentInnerSpanV2 {
    pub const fn role(&self) -> StructureRole {
        StructureRole::Span
    }

    pub fn actual_text(&self) -> Option<&str> {
        self.actual_text.as_deref()
    }
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Inner property scopes deliberately cannot own an MCID. Only the
    /// surrounding Formula/Figure marked-content record is an MCR.
    pub const fn has_mcid(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkedContentBindingKindV2 {
    Standard,
    Vector {
        usage_id: u32,
        display_command_fingerprint: [u8; 32],
    },
    EquationNumber {
        parent_owner: NodeId,
        shape_fingerprint: [u8; 32],
        glyph_receipt_fingerprint: [u8; 32],
    },
}

impl MarkedContentBindingKindV2 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Vector { .. } => "vector",
            Self::EquationNumber { .. } => "equation_number",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkedContentRecordV2 {
    selected_paint_ids: Vec<u32>,
    page_index: u32,
    paint_ordinal_start: u32,
    semantic_fragment_ordinal: u32,
    owner: MarkedContentOwner,
    outer_language: Option<String>,
    outer_actual_text: Option<String>,
    inner_span: Option<MarkedContentInnerSpanV2>,
    binding: MarkedContentBindingKindV2,
}

impl MarkedContentRecordV2 {
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
    pub fn outer_language(&self) -> Option<&str> {
        self.outer_language.as_deref()
    }
    pub fn outer_actual_text(&self) -> Option<&str> {
        self.outer_actual_text.as_deref()
    }
    pub const fn inner_span(&self) -> Option<&MarkedContentInnerSpanV2> {
        self.inner_span.as_ref()
    }
    pub const fn binding(&self) -> MarkedContentBindingKindV2 {
        self.binding
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaStructureKidV2 {
    MarkedContentReference { page_index: u32, mcid: u32 },
    StructureChild(StructureNodeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormulaStructureOrderV2 {
    formula_structure_node_id: StructureNodeId,
    kids: Vec<FormulaStructureKidV2>,
}

impl FormulaStructureOrderV2 {
    pub const fn formula_structure_node_id(&self) -> StructureNodeId {
        self.formula_structure_node_id
    }
    pub fn kids(&self) -> &[FormulaStructureKidV2] {
        &self.kids
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarkedContentStandardPaintInputV2 {
    pub page_index: u32,
    pub paint_ordinal: u32,
    pub semantic_fragment_ordinal: u32,
    pub owner: SelectedStructurePaintOwner,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorMarkedContentPlanV2 {
    selected_binding: SelectedStructureBindingReceiptV2,
    marked_content: MarkedContentPlanReceiptV2,
}

impl VectorMarkedContentPlanV2 {
    pub const fn selected_binding(&self) -> &SelectedStructureBindingReceiptV2 {
        &self.selected_binding
    }

    pub const fn marked_content(&self) -> &MarkedContentPlanReceiptV2 {
        &self.marked_content
    }

    /// Issues a borrowing projection for serialization. The downstream PDF
    /// crate receives sealed paint geometry and shaping facts, never layout
    /// or pagination owners that it could use to recompute selection.
    #[allow(clippy::too_many_arguments)]
    pub fn authorize_pdf_serialization<'a>(
        &'a self,
        registry: &StructureRegistryReceiptV2,
        accessibility_authorization: &StagingAccessibilityProfileAuthorizationV2,
        limits: &M4EffectiveResourceLimits,
        navigation: &ValidatedStagingBookNavigationV2,
        navigation_authorization: &StagingBookNavigationProfileAuthorizationV2,
        navigation_selected: &BookNavigationSelectedReceiptV2,
        vector_display: &StagingPrecomposedVectorDisplay,
        form_isolation: &VectorFormStructureIsolationReceiptV2,
        block_selected: &'a StagingAtomicVectorBlockSelectedLayout,
        math_flows: &'a StagingMathVectorFlowRegistry,
    ) -> Result<VectorMarkedContentSerializationV2<'a>, MarkedContentError> {
        self.verify(
            registry,
            accessibility_authorization,
            limits,
            navigation,
            navigation_authorization,
            navigation_selected,
            vector_display,
            form_isolation,
            block_selected,
            math_flows,
        )?;
        Ok(VectorMarkedContentSerializationV2 {
            plan: self,
            block_selected,
            math_flows,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        &self,
        registry: &StructureRegistryReceiptV2,
        accessibility_authorization: &StagingAccessibilityProfileAuthorizationV2,
        limits: &M4EffectiveResourceLimits,
        navigation: &ValidatedStagingBookNavigationV2,
        navigation_authorization: &StagingBookNavigationProfileAuthorizationV2,
        navigation_selected: &BookNavigationSelectedReceiptV2,
        vector_display: &StagingPrecomposedVectorDisplay,
        form_isolation: &VectorFormStructureIsolationReceiptV2,
        block_selected: &StagingAtomicVectorBlockSelectedLayout,
        math_flows: &StagingMathVectorFlowRegistry,
    ) -> Result<(), MarkedContentError> {
        navigation_selected
            .verify(navigation, navigation_authorization, limits, vector_display)
            .map_err(|_| MarkedContentError::BindingMismatch)?;
        verify_v2_upstream_closure(
            &self.selected_binding,
            registry,
            accessibility_authorization,
            limits,
            navigation_authorization,
            navigation_selected,
            vector_display,
            form_isolation,
            block_selected,
            math_flows,
        )?;
        self.marked_content.verify_sealed(
            registry,
            &self.selected_binding,
            accessibility_authorization,
            limits,
            navigation_selected,
            vector_display,
            form_isolation,
            block_selected,
            math_flows,
        )
    }
}

/// A non-owning, privately constructed PDF projection of the selected plan.
/// Its component identities are already sealed by marked-content-plan `/2`;
/// this projection does not introduce another canonical algorithm or charge.
#[derive(Clone, Copy, Debug)]
pub struct VectorMarkedContentSerializationV2<'a> {
    plan: &'a VectorMarkedContentPlanV2,
    block_selected: &'a StagingAtomicVectorBlockSelectedLayout,
    math_flows: &'a StagingMathVectorFlowRegistry,
}

impl<'a> VectorMarkedContentSerializationV2<'a> {
    pub const fn plan(self) -> &'a VectorMarkedContentPlanV2 {
        self.plan
    }

    pub fn equation_number_shapes(self) -> &'a [StagingEquationNumberShapeReceipt] {
        self.math_flows.equation_number_shapes()
    }

    pub fn equation_number_shape(
        self,
        owner: NodeId,
    ) -> Option<&'a StagingEquationNumberShapeReceipt> {
        self.math_flows.equation_number_shape(owner)
    }

    pub fn equation_number_rect(
        self,
        parent_owner: NodeId,
        page_index: u32,
        paint_ordinal: u32,
        shape_fingerprint: [u8; 32],
    ) -> Option<typaxis_core::Rect> {
        self.block_selected
            .placements()
            .iter()
            .find(|placement| {
                placement.owner() == parent_owner && placement.page_index() == page_index
            })
            .and_then(|placement| placement.equation_number())
            .filter(|number| {
                number.paint_ordinal() == paint_ordinal
                    && number.shape_fingerprint() == shape_fingerprint
            })
            .map(|number| number.rect())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        self,
        registry: &StructureRegistryReceiptV2,
        accessibility_authorization: &StagingAccessibilityProfileAuthorizationV2,
        limits: &M4EffectiveResourceLimits,
        navigation: &ValidatedStagingBookNavigationV2,
        navigation_authorization: &StagingBookNavigationProfileAuthorizationV2,
        navigation_selected: &BookNavigationSelectedReceiptV2,
        vector_display: &StagingPrecomposedVectorDisplay,
        form_isolation: &VectorFormStructureIsolationReceiptV2,
    ) -> Result<(), MarkedContentError> {
        self.plan.verify(
            registry,
            accessibility_authorization,
            limits,
            navigation,
            navigation_authorization,
            navigation_selected,
            vector_display,
            form_isolation,
            self.block_selected,
            self.math_flows,
        )
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkedContentPlanReceiptV2 {
    structure_registry_sha256: [u8; 32],
    selected_binding_sha256: [u8; 32],
    authorization_sha256: [u8; 32],
    navigation_selected_sha256: [u8; 32],
    vector_display_sha256: [u8; 32],
    form_isolation_sha256: [u8; 32],
    block_selected_sha256: [u8; 32],
    math_flow_registry_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    pages: Vec<MarkedContentPage>,
    records: Vec<MarkedContentRecordV2>,
    formula_orders: Vec<FormulaStructureOrderV2>,
    annotations: Vec<StructureLinkAnnotation>,
    parent_tree: Vec<StructureParentTreeEntry>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl MarkedContentPlanReceiptV2 {
    pub const fn structure_registry_sha256(&self) -> [u8; 32] {
        self.structure_registry_sha256
    }
    pub const fn selected_binding_sha256(&self) -> [u8; 32] {
        self.selected_binding_sha256
    }
    pub const fn authorization_sha256(&self) -> [u8; 32] {
        self.authorization_sha256
    }
    pub const fn navigation_selected_sha256(&self) -> [u8; 32] {
        self.navigation_selected_sha256
    }
    pub const fn vector_display_sha256(&self) -> [u8; 32] {
        self.vector_display_sha256
    }
    pub const fn form_isolation_sha256(&self) -> [u8; 32] {
        self.form_isolation_sha256
    }
    pub const fn block_selected_sha256(&self) -> [u8; 32] {
        self.block_selected_sha256
    }
    pub const fn math_flow_registry_sha256(&self) -> [u8; 32] {
        self.math_flow_registry_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub fn pages(&self) -> &[MarkedContentPage] {
        &self.pages
    }
    pub fn records(&self) -> &[MarkedContentRecordV2] {
        &self.records
    }
    pub fn formula_orders(&self) -> &[FormulaStructureOrderV2] {
        &self.formula_orders
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

    #[allow(clippy::too_many_arguments)]
    pub fn verify_sealed(
        &self,
        registry: &StructureRegistryReceiptV2,
        binding: &SelectedStructureBindingReceiptV2,
        authorization: &StagingAccessibilityProfileAuthorizationV2,
        limits: &M4EffectiveResourceLimits,
        navigation_selected: &BookNavigationSelectedReceiptV2,
        vector_display: &StagingPrecomposedVectorDisplay,
        form_isolation: &VectorFormStructureIsolationReceiptV2,
        block_selected: &StagingAtomicVectorBlockSelectedLayout,
        math_flows: &StagingMathVectorFlowRegistry,
    ) -> Result<(), MarkedContentError> {
        verify_v2_sealed_upstream_closure(
            binding,
            registry,
            authorization,
            limits,
            navigation_selected,
            vector_display,
            form_isolation,
            block_selected,
            math_flows,
        )?;
        let observed = build_marked_plan_v2_from_binding(
            registry,
            binding,
            authorization,
            limits,
            navigation_selected.fingerprint(),
            vector_display.receipt().fingerprint(),
            form_isolation.fingerprint(),
            block_selected.receipt().fingerprint(),
            math_flows.receipt().fingerprint(),
        )?;
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
    VectorMismatch,
    EquationNumberMismatch,
    FormStructureViolation,
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
            Self::VectorMismatch => {
                formatter.write_str("I9190: vector marked-content binding mismatch")
            }
            Self::EquationNumberMismatch => {
                formatter.write_str("I9190: equation-number marked-content binding mismatch")
            }
            Self::FormStructureViolation => {
                formatter.write_str("I9190: reusable vector Form contains structure properties")
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

/// Joins the selected vector Display, computed-language selection, optional
/// equation-number glyph paint, and all remaining selected paints into one
/// closed structure binding and marked-content plan.
///
/// `standard_paints` owns only non-vector paints and artifacts. Vector and
/// equation-number paints are derived from their sealed producer receipts so
/// callers cannot relabel a visual usage as another structure owner.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn build_vector_marked_content_plan_v2(
    registry: &StructureRegistryReceiptV2,
    accessibility_authorization: &StagingAccessibilityProfileAuthorizationV2,
    limits: &M4EffectiveResourceLimits,
    navigation: &ValidatedStagingBookNavigationV2,
    navigation_authorization: &StagingBookNavigationProfileAuthorizationV2,
    navigation_selected: &BookNavigationSelectedReceiptV2,
    standard_paints: &[MarkedContentStandardPaintInputV2],
    annotations: &[SelectedStructureAnnotationInput],
    vector_display: &StagingPrecomposedVectorDisplay,
    form_isolation: &VectorFormStructureIsolationReceiptV2,
    block_selected: &StagingAtomicVectorBlockSelectedLayout,
    math_flows: &StagingMathVectorFlowRegistry,
) -> Result<VectorMarkedContentPlanV2, MarkedContentError> {
    navigation_selected
        .verify(navigation, navigation_authorization, limits, vector_display)
        .map_err(|_| MarkedContentError::BindingMismatch)?;
    vector_display
        .verify_resource_closure()
        .map_err(|_| MarkedContentError::VectorMismatch)?;
    form_isolation
        .verify(vector_display)
        .map_err(|_| MarkedContentError::FormStructureViolation)?;
    if accessibility_authorization.book_navigation_profile_fingerprint()
        != navigation_authorization.profile_receipt_fingerprint()
        || navigation_selected.vector_display_sha256() != vector_display.receipt().fingerprint()
        || vector_display.receipt().block_selected_layout_fingerprint()
            != block_selected.receipt().fingerprint()
        || block_selected.receipt().math_flow_registry_fingerprint()
            != math_flows.receipt().fingerprint()
        || form_isolation.form_mcid_count() != 0
        || form_isolation.form_structure_property_count() != 0
    {
        return Err(MarkedContentError::BindingMismatch);
    }

    validate_standard_language_paints_v2(registry, navigation_selected, standard_paints)?;

    let mut navigation_vectors = BTreeMap::new();
    for paint in navigation_selected.vector_paints() {
        if navigation_vectors.insert(paint.usage_id(), paint).is_some() {
            return Err(MarkedContentError::VectorMismatch);
        }
    }

    let estimated = standard_paints
        .len()
        .checked_add(vector_display.receipt().command_count() as usize)
        .and_then(|count| count.checked_add(math_flows.equation_number_shapes().len()))
        .ok_or(MarkedContentError::FragmentLimit)?;
    let mut inputs = Vec::new();
    inputs
        .try_reserve_exact(estimated)
        .map_err(|_| MarkedContentError::AllocationFailure)?;
    for paint in standard_paints {
        inputs.push(SelectedStructurePaintInputV2 {
            selected_paint_id: 0,
            page_index: paint.page_index,
            paint_ordinal: paint.paint_ordinal,
            semantic_fragment_ordinal: paint.semantic_fragment_ordinal,
            owner: paint.owner,
            binding: SelectedStructurePaintBindingV2::Standard,
        });
    }

    let mut observed_vector_usages = BTreeSet::new();
    for command in vector_display.commands() {
        let navigation_paint = navigation_vectors
            .get(&command.usage_id())
            .ok_or(MarkedContentError::VectorMismatch)?;
        let node = registry
            .source_node(command.owner())
            .ok_or(MarkedContentError::VectorMismatch)?;
        let vector = node
            .vector_binding_v2()
            .ok_or(MarkedContentError::VectorMismatch)?;
        let language = node
            .language_binding_v2()
            .ok_or(MarkedContentError::VectorMismatch)?;
        let expected_paint_language = node.language() != navigation.languages().document_language();
        if command.kind() != vector.kind()
            || navigation_paint.owner_node_id() != command.owner()
            || navigation_paint.kind() != command.kind()
            || navigation_paint.page_index() != command.page_index()
            || navigation_paint.paint_ordinal() != command.paint_ordinal()
            || navigation_paint.language() != node.language()
            || navigation_paint.language_record_fingerprint() != language.record_fingerprint()
            || navigation_paint.display_command_fingerprint() != command.fingerprint()
            || navigation_paint.requires_paint_language() != expected_paint_language
            || command
                .relation()
                .baseline_metrics()
                .is_some_and(|metrics| {
                    metrics.metric_receipt_fingerprint() != vector.metrics_fingerprint()
                })
            || !observed_vector_usages.insert(command.usage_id())
        {
            return Err(MarkedContentError::VectorMismatch);
        }
        inputs.push(SelectedStructurePaintInputV2 {
            selected_paint_id: 0,
            page_index: command.page_index(),
            paint_ordinal: command.paint_ordinal(),
            // A DrawVector usage is one indivisible semantic fragment even
            // when its upstream layout fragment ordinal is nonzero.
            semantic_fragment_ordinal: 0,
            owner: SelectedStructurePaintOwner::Structure(node.structure_node_id()),
            binding: SelectedStructurePaintBindingV2::Vector(SelectedVectorPaintBindingV2 {
                usage_id: command.usage_id(),
                kind: command.kind(),
                metrics_fingerprint: vector.metrics_fingerprint(),
                display_command_fingerprint: command.fingerprint(),
            }),
        });
    }
    if observed_vector_usages.len() != navigation_vectors.len() {
        return Err(MarkedContentError::VectorMismatch);
    }

    let mut observed_equation_owners = BTreeSet::new();
    for placement in block_selected.placements() {
        let Some(selected_number) = placement.equation_number() else {
            continue;
        };
        let shape = math_flows
            .equation_number_shape(placement.owner())
            .ok_or(MarkedContentError::EquationNumberMismatch)?;
        let parent = registry
            .source_node(placement.owner())
            .ok_or(MarkedContentError::EquationNumberMismatch)?;
        let child = registry
            .source_node(selected_number.owner())
            .ok_or(MarkedContentError::EquationNumberMismatch)?;
        let number = child
            .equation_number_binding_v2()
            .ok_or(MarkedContentError::EquationNumberMismatch)?;
        let child_language = child
            .language_binding_v2()
            .ok_or(MarkedContentError::EquationNumberMismatch)?;
        let parent_language = parent
            .language_binding_v2()
            .ok_or(MarkedContentError::EquationNumberMismatch)?;
        if parent.vector_binding_v2().map(|value| value.kind())
            != Some(PrecomposedVectorKind::MathVectorBlock)
            || parent.children().last() != Some(&child.structure_node_id())
            || child.parent() != Some(parent.structure_node_id())
            || child.role() != StructureRole::Span
            || number.parent_owner() != placement.owner()
            || selected_number.source_span() != shape.source_span()
            || selected_number.owner() != shape.node_id()
            || selected_number.shape_fingerprint() != shape.fingerprint()
            || shape.owner() != placement.owner()
            || !shape.integrity_matches()
            || shape.text_span() != number.text_span()
            || shape.text_buffer_sha256() != number.text_buffer_sha256()
            || shape.exact_text() != number.exact_text()
            || shape.exact_text_sha256() != number.exact_text_sha256()
            || shape.owner_language() != child.language()
            || child_language.parent_record_fingerprint()
                != Some(parent_language.record_fingerprint())
            || !observed_equation_owners.insert(placement.owner())
        {
            return Err(MarkedContentError::EquationNumberMismatch);
        }
        inputs.push(SelectedStructurePaintInputV2 {
            selected_paint_id: 0,
            page_index: placement.page_index(),
            paint_ordinal: selected_number.paint_ordinal(),
            semantic_fragment_ordinal: 0,
            owner: SelectedStructurePaintOwner::Structure(child.structure_node_id()),
            binding: SelectedStructurePaintBindingV2::EquationNumber(
                SelectedEquationNumberPaintBindingV2 {
                    parent_owner: placement.owner(),
                    text_span: number.text_span(),
                    text_buffer_sha256: number.text_buffer_sha256(),
                    exact_text: number.exact_text().to_owned(),
                    exact_text_sha256: number.exact_text_sha256(),
                    shape_fingerprint: shape.fingerprint(),
                    glyph_receipt_fingerprint: shape.glyph_receipt_fingerprint(),
                    shape_language_fingerprint: shape.owner_language_fingerprint(),
                    language_record_fingerprint: child_language.record_fingerprint(),
                    parent_language_record_fingerprint: parent_language.record_fingerprint(),
                },
            ),
        });
    }
    if observed_equation_owners.len() != math_flows.equation_number_shapes().len() {
        return Err(MarkedContentError::EquationNumberMismatch);
    }

    inputs.sort_by_key(|paint| (paint.page_index, paint.paint_ordinal));
    if inputs.windows(2).any(|pair| {
        (pair[0].page_index, pair[0].paint_ordinal) >= (pair[1].page_index, pair[1].paint_ordinal)
    }) {
        return Err(MarkedContentError::PaintOrder);
    }
    for (index, paint) in inputs.iter_mut().enumerate() {
        paint.selected_paint_id =
            u32::try_from(index).map_err(|_| MarkedContentError::FragmentLimit)?;
    }
    let pages = navigation_selected
        .pages()
        .iter()
        .map(|page| SelectedStructurePage {
            page_index: page.page_index,
            width_raw: page.width_raw,
            height_raw: page.height_raw,
        })
        .collect::<Vec<_>>();
    let selected_binding = select_structure_bindings_v2(
        registry,
        accessibility_authorization,
        limits,
        navigation_selected.selected_layout_sha256(),
        navigation_selected.selected_layout_fragment_count(),
        &pages,
        &inputs,
        annotations,
    )
    .map_err(map_selected_binding_error_v2)?;
    verify_v2_upstream_closure(
        &selected_binding,
        registry,
        accessibility_authorization,
        limits,
        navigation_authorization,
        navigation_selected,
        vector_display,
        form_isolation,
        block_selected,
        math_flows,
    )?;
    let marked_content = build_marked_plan_v2_from_binding(
        registry,
        &selected_binding,
        accessibility_authorization,
        limits,
        navigation_selected.fingerprint(),
        vector_display.receipt().fingerprint(),
        form_isolation.fingerprint(),
        block_selected.receipt().fingerprint(),
        math_flows.receipt().fingerprint(),
    )?;
    Ok(VectorMarkedContentPlanV2 {
        selected_binding,
        marked_content,
    })
}

fn map_selected_binding_error_v2(error: SelectedStructureBindingError) -> MarkedContentError {
    match error {
        SelectedStructureBindingError::FragmentLimit => MarkedContentError::FragmentLimit,
        SelectedStructureBindingError::AllocationFailure => MarkedContentError::AllocationFailure,
        _ => MarkedContentError::BindingMismatch,
    }
}

fn validate_standard_language_paints_v2(
    registry: &StructureRegistryReceiptV2,
    navigation_selected: &BookNavigationSelectedReceiptV2,
    standard_paints: &[MarkedContentStandardPaintInputV2],
) -> Result<(), MarkedContentError> {
    let required_owners = navigation_selected
        .language_paints()
        .iter()
        .map(|paint| paint.owner_node_id())
        .collect::<BTreeSet<_>>();
    let expected = navigation_selected
        .language_paints()
        .iter()
        .map(|paint| {
            (
                paint.owner_node_id(),
                paint.page_index(),
                paint.paint_ordinal(),
                paint.language_record_fingerprint(),
                paint.language(),
            )
        })
        .collect::<BTreeSet<_>>();
    let mut observed = BTreeSet::new();
    for paint in standard_paints {
        let SelectedStructurePaintOwner::Structure(id) = paint.owner else {
            continue;
        };
        let node = registry
            .node(id)
            .ok_or(MarkedContentError::BindingMismatch)?;
        let StructureOwner::Source(owner) = node.owner() else {
            continue;
        };
        if !required_owners.contains(&owner) {
            continue;
        }
        let language = node
            .language_binding_v2()
            .ok_or(MarkedContentError::BindingMismatch)?;
        if !observed.insert((
            owner,
            paint.page_index,
            paint.paint_ordinal,
            language.record_fingerprint(),
            node.language(),
        )) {
            return Err(MarkedContentError::BindingMismatch);
        }
    }
    if observed != expected {
        return Err(MarkedContentError::BindingMismatch);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn verify_v2_upstream_closure(
    binding: &SelectedStructureBindingReceiptV2,
    registry: &StructureRegistryReceiptV2,
    accessibility_authorization: &StagingAccessibilityProfileAuthorizationV2,
    limits: &M4EffectiveResourceLimits,
    navigation_authorization: &StagingBookNavigationProfileAuthorizationV2,
    navigation_selected: &BookNavigationSelectedReceiptV2,
    vector_display: &StagingPrecomposedVectorDisplay,
    form_isolation: &VectorFormStructureIsolationReceiptV2,
    block_selected: &StagingAtomicVectorBlockSelectedLayout,
    math_flows: &StagingMathVectorFlowRegistry,
) -> Result<(), MarkedContentError> {
    verify_v2_sealed_upstream_closure(
        binding,
        registry,
        accessibility_authorization,
        limits,
        navigation_selected,
        vector_display,
        form_isolation,
        block_selected,
        math_flows,
    )?;
    if accessibility_authorization.book_navigation_profile_fingerprint()
        != navigation_authorization.profile_receipt_fingerprint()
        || navigation_selected.profile_sha256()
            != navigation_authorization.profile_receipt_fingerprint()
    {
        return Err(MarkedContentError::BindingMismatch);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn verify_v2_sealed_upstream_closure(
    binding: &SelectedStructureBindingReceiptV2,
    registry: &StructureRegistryReceiptV2,
    accessibility_authorization: &StagingAccessibilityProfileAuthorizationV2,
    limits: &M4EffectiveResourceLimits,
    navigation_selected: &BookNavigationSelectedReceiptV2,
    vector_display: &StagingPrecomposedVectorDisplay,
    form_isolation: &VectorFormStructureIsolationReceiptV2,
    block_selected: &StagingAtomicVectorBlockSelectedLayout,
    math_flows: &StagingMathVectorFlowRegistry,
) -> Result<(), MarkedContentError> {
    binding
        .verify(registry, accessibility_authorization, limits)
        .map_err(|_| MarkedContentError::BindingMismatch)?;
    vector_display
        .verify_resource_closure()
        .map_err(|_| MarkedContentError::VectorMismatch)?;
    form_isolation
        .verify(vector_display)
        .map_err(|_| MarkedContentError::FormStructureViolation)?;
    let pages_match = binding.pages().len() == navigation_selected.pages().len()
        && binding
            .pages()
            .iter()
            .zip(navigation_selected.pages())
            .all(|(left, right)| {
                left.page_index == right.page_index
                    && left.width_raw == right.width_raw
                    && left.height_raw == right.height_raw
            });
    if accessibility_authorization.book_navigation_profile_fingerprint()
        != navigation_selected.profile_sha256()
        || accessibility_authorization.view().package_sha256() != registry.package_sha256()
        || accessibility_authorization.view().metadata_sha256()
            != navigation_selected.metadata_sha256()
        || accessibility_authorization.view().language_sha256()
            != navigation_selected.language_sha256()
        || accessibility_authorization.view().outline_sha256()
            != navigation_selected.outline_sha256()
        || registry.package_sha256() != vector_display.receipt().package_sha256()
        || registry.limits_sha256() != limits.fingerprint()
        || binding.selected_layout_sha256() != navigation_selected.selected_layout_sha256()
        || binding.selected_layout_fragment_count()
            != navigation_selected.selected_layout_fragment_count()
        || !pages_match
        || navigation_selected.vector_display_sha256() != vector_display.receipt().fingerprint()
        || vector_display.receipt().block_selected_layout_fingerprint()
            != block_selected.receipt().fingerprint()
        || block_selected.receipt().math_flow_registry_fingerprint()
            != math_flows.receipt().fingerprint()
        || form_isolation.vector_display_sha256() != vector_display.receipt().fingerprint()
        || form_isolation.page_do_usage_count() != vector_display.receipt().command_count()
        || form_isolation.form_mcid_count() != 0
        || form_isolation.form_structure_property_count() != 0
    {
        return Err(MarkedContentError::BindingMismatch);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn build_marked_plan_v2_from_binding(
    registry: &StructureRegistryReceiptV2,
    binding: &SelectedStructureBindingReceiptV2,
    authorization: &StagingAccessibilityProfileAuthorizationV2,
    limits: &M4EffectiveResourceLimits,
    navigation_selected_sha256: [u8; 32],
    vector_display_sha256: [u8; 32],
    form_isolation_sha256: [u8; 32],
    block_selected_sha256: [u8; 32],
    math_flow_registry_sha256: [u8; 32],
) -> Result<MarkedContentPlanReceiptV2, MarkedContentError> {
    binding
        .verify(registry, authorization, limits)
        .map_err(|_| MarkedContentError::BindingMismatch)?;
    if registry.fingerprint() != binding.structure_registry_sha256()
        || authorization.fingerprint() != binding.authorization_sha256()
        || authorization.view().limits_sha256() != binding.limits_sha256()
        || binding.limits_sha256() != limits.fingerprint()
        || navigation_selected_sha256 == [0; 32]
        || vector_display_sha256 == [0; 32]
        || form_isolation_sha256 == [0; 32]
        || block_selected_sha256 == [0; 32]
        || math_flow_registry_sha256 == [0; 32]
    {
        return Err(MarkedContentError::RegistryMismatch);
    }

    let group_count = binding
        .paints()
        .iter()
        .enumerate()
        .filter(|(index, paint)| {
            *index == 0 || !same_marked_group_v2(&binding.paints()[index - 1], paint)
        })
        .count();
    let charged_fragments = binding
        .selected_layout_fragment_count()
        .checked_add(u64::try_from(group_count).map_err(|_| MarkedContentError::FragmentLimit)?)
        .ok_or(MarkedContentError::FragmentLimit)?;
    if charged_fragments > limits.base().get().max_fragments {
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
        while end < binding.paints().len() && same_marked_group_v2(paint, &binding.paints()[end]) {
            end += 1;
        }
        let mut selected_paint_ids = Vec::new();
        selected_paint_ids
            .try_reserve_exact(end - start)
            .map_err(|_| MarkedContentError::AllocationFailure)?;
        for (offset, grouped) in binding.paints()[start..end].iter().enumerate() {
            let offset = u32::try_from(offset).map_err(|_| MarkedContentError::PaintOrder)?;
            if grouped.selected_paint_id()
                != paint
                    .selected_paint_id()
                    .checked_add(offset)
                    .ok_or(MarkedContentError::PaintOrder)?
                || grouped.paint_ordinal()
                    != paint
                        .paint_ordinal()
                        .checked_add(offset)
                        .ok_or(MarkedContentError::PaintOrder)?
            {
                return Err(MarkedContentError::PaintOrder);
            }
            selected_paint_ids.push(grouped.selected_paint_id());
        }

        let (owner, outer_language, outer_actual_text, inner_span, record_binding) =
            match paint.owner() {
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
                    let structure = MarkedContentOwner::Structure(MarkedContentStructure {
                        structure_node_id,
                        role: node.role(),
                        mcid,
                    });
                    match paint.binding() {
                        SelectedStructurePaintBindingV2::Standard => {
                            let (outer_language, outer_actual_text, inner_span) =
                                standard_property_scopes_v2(
                                    node.role(),
                                    paint.actual_text(),
                                    paint.language(),
                                );
                            (
                                structure,
                                outer_language,
                                outer_actual_text,
                                inner_span,
                                MarkedContentBindingKindV2::Standard,
                            )
                        }
                        SelectedStructurePaintBindingV2::Vector(vector) => {
                            let expected = node
                                .vector_binding_v2()
                                .ok_or(MarkedContentError::VectorMismatch)?;
                            if expected.kind() != vector.kind
                                || expected.metrics_fingerprint() != vector.metrics_fingerprint
                                || selected_paint_ids.len() != 1
                            {
                                return Err(MarkedContentError::VectorMismatch);
                            }
                            let inner_span = vector_inner_span_v2(
                                vector.kind,
                                paint.actual_text(),
                                paint.language(),
                            )?;
                            (
                                structure,
                                None,
                                None,
                                inner_span,
                                MarkedContentBindingKindV2::Vector {
                                    usage_id: vector.usage_id,
                                    display_command_fingerprint: vector.display_command_fingerprint,
                                },
                            )
                        }
                        SelectedStructurePaintBindingV2::EquationNumber(number) => {
                            if node.equation_number_binding_v2().is_none()
                                || selected_paint_ids.len() != 1
                                || paint.actual_text().is_some()
                            {
                                return Err(MarkedContentError::EquationNumberMismatch);
                            }
                            (
                                structure,
                                paint.language().map(str::to_owned),
                                None,
                                None,
                                MarkedContentBindingKindV2::EquationNumber {
                                    parent_owner: number.parent_owner,
                                    shape_fingerprint: number.shape_fingerprint,
                                    glyph_receipt_fingerprint: number.glyph_receipt_fingerprint,
                                },
                            )
                        }
                    }
                }
                SelectedStructurePaintOwner::Artifact { class, occurrence } => {
                    if !matches!(paint.binding(), SelectedStructurePaintBindingV2::Standard)
                        || !artifact_keys.insert((class, occurrence))
                    {
                        return Err(MarkedContentError::ArtifactMismatch);
                    }
                    let count = artifact_counts.entry(paint.page_index()).or_insert(0);
                    *count = count
                        .checked_add(1)
                        .ok_or(MarkedContentError::McidOverflow)?;
                    (
                        MarkedContentOwner::Artifact(MarkedContentArtifact { class, occurrence }),
                        None,
                        None,
                        None,
                        MarkedContentBindingKindV2::Standard,
                    )
                }
            };
        records.push(MarkedContentRecordV2 {
            selected_paint_ids,
            page_index: paint.page_index(),
            paint_ordinal_start: paint.paint_ordinal(),
            semantic_fragment_ordinal: paint.semantic_fragment_ordinal(),
            owner,
            outer_language,
            outer_actual_text,
            inner_span,
            binding: record_binding,
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

    let formula_orders = build_formula_orders_v2(registry, &records)?;
    let (pages, annotations, parent_tree) =
        build_parent_tree_v2(binding, &mcids, &artifact_counts, &mut page_arrays)?;
    let mut receipt = MarkedContentPlanReceiptV2 {
        structure_registry_sha256: registry.fingerprint(),
        selected_binding_sha256: binding.fingerprint(),
        authorization_sha256: authorization.fingerprint(),
        navigation_selected_sha256,
        vector_display_sha256,
        form_isolation_sha256,
        block_selected_sha256,
        math_flow_registry_sha256,
        limits_sha256: limits.fingerprint(),
        pages,
        records,
        formula_orders,
        annotations,
        parent_tree,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_plan_v2(&receipt);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    Ok(receipt)
}

fn vector_inner_span_v2(
    kind: PrecomposedVectorKind,
    actual_text: Option<&str>,
    language: Option<&str>,
) -> Result<Option<MarkedContentInnerSpanV2>, MarkedContentError> {
    match kind {
        PrecomposedVectorKind::MathVector | PrecomposedVectorKind::MathVectorBlock => {
            Ok(Some(MarkedContentInnerSpanV2 {
                actual_text: Some(
                    actual_text
                        .ok_or(MarkedContentError::VectorMismatch)?
                        .to_owned(),
                ),
                language: language.map(str::to_owned),
            }))
        }
        PrecomposedVectorKind::InlineVector => Ok((actual_text.is_some() || language.is_some())
            .then(|| MarkedContentInnerSpanV2 {
                actual_text: actual_text.map(str::to_owned),
                language: language.map(str::to_owned),
            })),
        PrecomposedVectorKind::VectorFigure => {
            if actual_text.is_some() {
                return Err(MarkedContentError::VectorMismatch);
            }
            Ok(language.map(|language| MarkedContentInnerSpanV2 {
                actual_text: None,
                language: Some(language.to_owned()),
            }))
        }
    }
}

fn standard_property_scopes_v2(
    role: StructureRole,
    actual_text: Option<&str>,
    language: Option<&str>,
) -> (
    Option<String>,
    Option<String>,
    Option<MarkedContentInnerSpanV2>,
) {
    if role == StructureRole::Span {
        return (
            language.map(str::to_owned),
            actual_text.map(str::to_owned),
            None,
        );
    }
    (
        None,
        None,
        (actual_text.is_some() || language.is_some()).then(|| MarkedContentInnerSpanV2 {
            actual_text: actual_text.map(str::to_owned),
            language: language.map(str::to_owned),
        }),
    )
}

fn same_marked_group_v2(left: &SelectedStructurePaintV2, right: &SelectedStructurePaintV2) -> bool {
    matches!(
        (left.binding(), right.binding()),
        (
            SelectedStructurePaintBindingV2::Standard,
            SelectedStructurePaintBindingV2::Standard
        )
    ) && left.page_index() == right.page_index()
        && left.semantic_fragment_ordinal() == right.semantic_fragment_ordinal()
        && left.owner() == right.owner()
        && left.role() == right.role()
        && left.language() == right.language()
        && left.actual_text() == right.actual_text()
}

fn build_formula_orders_v2(
    registry: &StructureRegistryReceiptV2,
    records: &[MarkedContentRecordV2],
) -> Result<Vec<FormulaStructureOrderV2>, MarkedContentError> {
    let mut orders = Vec::new();
    orders
        .try_reserve_exact(records.len())
        .map_err(|_| MarkedContentError::AllocationFailure)?;
    let mut observed_formulas = BTreeSet::new();
    let mut observed_numbers = BTreeSet::new();
    for (index, record) in records.iter().enumerate() {
        let MarkedContentOwner::Structure(owner) = record.owner else {
            continue;
        };
        let node = registry
            .node(owner.structure_node_id)
            .ok_or(MarkedContentError::VectorMismatch)?;
        if node.role() != StructureRole::Formula {
            continue;
        }
        if !observed_formulas.insert(owner.structure_node_id) {
            return Err(MarkedContentError::VectorMismatch);
        }
        let mut kids = vec![FormulaStructureKidV2::MarkedContentReference {
            page_index: record.page_index,
            mcid: owner.mcid,
        }];
        let number_children = node
            .children()
            .iter()
            .filter(|child| {
                registry
                    .node(**child)
                    .is_some_and(|child| child.equation_number_binding_v2().is_some())
            })
            .copied()
            .collect::<Vec<_>>();
        let vector_kind = match (record.binding, node.vector_binding_v2()) {
            (MarkedContentBindingKindV2::Standard, None) => {
                if !number_children.is_empty() {
                    return Err(MarkedContentError::EquationNumberMismatch);
                }
                orders.push(FormulaStructureOrderV2 {
                    formula_structure_node_id: owner.structure_node_id,
                    kids,
                });
                continue;
            }
            (MarkedContentBindingKindV2::Vector { .. }, Some(vector)) => vector.kind(),
            _ => return Err(MarkedContentError::VectorMismatch),
        };
        match vector_kind {
            PrecomposedVectorKind::MathVector => {
                if !number_children.is_empty() {
                    return Err(MarkedContentError::EquationNumberMismatch);
                }
            }
            PrecomposedVectorKind::MathVectorBlock if number_children.len() > 1 => {
                return Err(MarkedContentError::EquationNumberMismatch);
            }
            PrecomposedVectorKind::MathVectorBlock => {
                if let Some(child_id) = number_children.first().copied() {
                    let next = records
                        .get(index + 1)
                        .ok_or(MarkedContentError::EquationNumberMismatch)?;
                    let MarkedContentOwner::Structure(next_owner) = next.owner else {
                        return Err(MarkedContentError::EquationNumberMismatch);
                    };
                    if next_owner.structure_node_id != child_id
                        || next.page_index != record.page_index
                        || record.paint_ordinal_start.checked_add(1)
                            != Some(next.paint_ordinal_start)
                        || !matches!(
                            next.binding,
                            MarkedContentBindingKindV2::EquationNumber { parent_owner, .. }
                                if node.owner() == StructureOwner::Source(parent_owner)
                        )
                        || !observed_numbers.insert(child_id)
                    {
                        return Err(MarkedContentError::EquationNumberMismatch);
                    }
                    kids.push(FormulaStructureKidV2::StructureChild(child_id));
                }
            }
            PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::VectorFigure => {
                return Err(MarkedContentError::VectorMismatch);
            }
        }
        orders.push(FormulaStructureOrderV2 {
            formula_structure_node_id: owner.structure_node_id,
            kids,
        });
    }
    let expected_numbers = registry
        .nodes()
        .iter()
        .filter(|node| node.equation_number_binding_v2().is_some())
        .map(|node| node.structure_node_id())
        .collect::<BTreeSet<_>>();
    let expected_formulas = registry
        .nodes()
        .iter()
        .filter(|node| node.paint_required() && node.role() == StructureRole::Formula)
        .map(|node| node.structure_node_id())
        .collect::<BTreeSet<_>>();
    if observed_numbers != expected_numbers {
        return Err(MarkedContentError::EquationNumberMismatch);
    }
    if observed_formulas != expected_formulas {
        return Err(MarkedContentError::VectorMismatch);
    }
    orders.sort_by_key(FormulaStructureOrderV2::formula_structure_node_id);
    Ok(orders)
}

type MarkedContentTreeV2 = (
    Vec<MarkedContentPage>,
    Vec<StructureLinkAnnotation>,
    Vec<StructureParentTreeEntry>,
);

fn build_parent_tree_v2(
    binding: &SelectedStructureBindingReceiptV2,
    mcids: &BTreeMap<u32, u32>,
    artifact_counts: &BTreeMap<u32, u32>,
    page_arrays: &mut BTreeMap<u32, Vec<StructureNodeId>>,
) -> Result<MarkedContentTreeV2, MarkedContentError> {
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
    Ok((pages, annotations, parent_tree))
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

fn encode_plan_v2(value: &MarkedContentPlanReceiptV2) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MARKED_CONTENT_PLAN_ALGORITHM_V2);
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
    output.push_str(",\"block_selected_sha256\":");
    push_hash(&mut output, value.block_selected_sha256);
    output.push_str(",\"form_isolation_sha256\":");
    push_hash(&mut output, value.form_isolation_sha256);
    output.push_str(",\"formula_orders\":[");
    for (index, formula) in value.formula_orders.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"formula_structure_node_id\":");
        output.push_str(&formula.formula_structure_node_id.get().to_string());
        output.push_str(",\"kids\":[");
        for (kid_index, kid) in formula.kids.iter().enumerate() {
            if kid_index != 0 {
                output.push(',');
            }
            match kid {
                FormulaStructureKidV2::MarkedContentReference { page_index, mcid } => {
                    output.push_str("{\"kind\":\"mcr\",\"mcid\":");
                    output.push_str(&mcid.to_string());
                    output.push_str(",\"page_index\":");
                    output.push_str(&page_index.to_string());
                    output.push('}');
                }
                FormulaStructureKidV2::StructureChild(child) => {
                    output.push_str("{\"kind\":\"structure_child\",\"structure_node_id\":");
                    output.push_str(&child.get().to_string());
                    output.push('}');
                }
            }
        }
        output.push_str("]}");
    }
    output.push_str("],\"limits_sha256\":");
    push_hash(&mut output, value.limits_sha256);
    output.push_str(",\"math_flow_registry_sha256\":");
    push_hash(&mut output, value.math_flow_registry_sha256);
    output.push_str(",\"navigation_selected_sha256\":");
    push_hash(&mut output, value.navigation_selected_sha256);
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
        output.push_str("{\"binding\":");
        encode_binding_kind_v2(&mut output, record.binding);
        output.push_str(",\"inner_span\":");
        if let Some(inner) = &record.inner_span {
            output.push_str("{\"actual_text\":");
            push_optional(&mut output, inner.actual_text.as_deref());
            output.push_str(",\"language\":");
            push_optional(&mut output, inner.language.as_deref());
            output.push_str(",\"role\":\"Span\"");
            output.push('}');
        } else {
            output.push_str("null");
        }
        output.push_str(",\"outer_actual_text\":");
        push_optional(&mut output, record.outer_actual_text.as_deref());
        output.push_str(",\"outer_language\":");
        push_optional(&mut output, record.outer_language.as_deref());
        output.push_str(",\"owner\":");
        encode_marked_owner(&mut output, record.owner);
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
    output.push_str(",\"vector_display_sha256\":");
    push_hash(&mut output, value.vector_display_sha256);
    output.push('}');
    output
}

fn encode_binding_kind_v2(output: &mut String, binding: MarkedContentBindingKindV2) {
    match binding {
        MarkedContentBindingKindV2::Standard => output.push_str("{\"kind\":\"standard\"}"),
        MarkedContentBindingKindV2::Vector {
            usage_id,
            display_command_fingerprint,
        } => {
            output.push_str("{\"display_command_fingerprint\":");
            push_hash(output, display_command_fingerprint);
            output.push_str(",\"kind\":\"vector\",\"usage_id\":");
            output.push_str(&usage_id.to_string());
            output.push('}');
        }
        MarkedContentBindingKindV2::EquationNumber {
            parent_owner,
            shape_fingerprint,
            glyph_receipt_fingerprint,
        } => {
            output.push_str("{\"glyph_receipt_fingerprint\":");
            push_hash(output, glyph_receipt_fingerprint);
            output.push_str(",\"kind\":\"equation_number\",\"parent_owner\":");
            output.push_str(&parent_owner.get().to_string());
            output.push_str(",\"shape_fingerprint\":");
            push_hash(output, shape_fingerprint);
            output.push('}');
        }
    }
}

fn encode_marked_owner(output: &mut String, owner: MarkedContentOwner) {
    match owner {
        MarkedContentOwner::Structure(owner) => {
            output.push_str("{\"kind\":\"structure\",\"mcid\":");
            output.push_str(&owner.mcid.to_string());
            output.push_str(",\"role\":");
            push_jcs_string(output, owner.role.pdf_name());
            output.push_str(",\"structure_node_id\":");
            output.push_str(&owner.structure_node_id.get().to_string());
            output.push('}');
        }
        MarkedContentOwner::Artifact(owner) => {
            output.push_str("{\"class\":");
            push_jcs_string(output, owner.class.as_str());
            output.push_str(",\"kind\":\"artifact\",\"occurrence\":");
            output.push_str(&owner.occurrence.to_string());
            output.push('}');
        }
    }
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
        build_structure_registry, build_structure_registry_v2, select_structure_bindings,
        SelectedStructureAnnotationInput, SelectedStructurePage, SelectedStructurePaintInput,
        SelectedStructurePaintOwner, StructureOwner, StructureRole,
    };
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::{
        validate_staging_book_navigation, validate_staging_book_navigation_v2,
        validate_staging_structure_semantics, validate_staging_structure_semantics_v2,
        StagingAccessibilityProfileAuthorizationV2, StagingAccessibilityProfileView,
        StagingAccessibilityProfileViewV2, StagingBookNavigationProfileAuthorizationV2,
        StagingBookNavigationProfileViewV2, StagingSemanticPackageParser,
        ValidatedStagingBookNavigationV2, ValidatedStagingStructureSemanticsV2,
    };

    use crate::{
        prove_vector_form_structure_isolation_v2, select_staging_book_navigation_v2,
        staging_precomposed_vector_display_fixture,
        staging_precomposed_vector_display_language_override_fixture, BookNavigationSelectedPage,
        BookNavigationSelectedReceiptV2, StagingPrecomposedVectorDisplayFixture,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/accessibility/job/document-package.json"
    ));

    struct VectorStructureFixture {
        display: StagingPrecomposedVectorDisplayFixture,
        navigation: ValidatedStagingBookNavigationV2,
        navigation_authorization: StagingBookNavigationProfileAuthorizationV2,
        navigation_selected: BookNavigationSelectedReceiptV2,
        semantics: ValidatedStagingStructureSemanticsV2,
        accessibility_authorization: StagingAccessibilityProfileAuthorizationV2,
        registry: StructureRegistryReceiptV2,
        form_isolation: VectorFormStructureIsolationReceiptV2,
        plan: VectorMarkedContentPlanV2,
    }

    fn vector_structure_fixture(language_override: bool) -> VectorStructureFixture {
        const SCALE: i64 = 65_536;
        let display = if language_override {
            staging_precomposed_vector_display_language_override_fixture().unwrap()
        } else {
            staging_precomposed_vector_display_fixture().unwrap()
        };
        let package = &display.layout.package;
        let limits = &display.layout.limits;
        let navigation = validate_staging_book_navigation_v2(package, limits).unwrap();
        let navigation_profile_receipt = sha256(if language_override {
            &b"tagged-vector-navigation-profile-override-v2"[..]
        } else {
            &b"tagged-vector-navigation-profile-v2"[..]
        });
        let navigation_authorization =
            StagingBookNavigationProfileAuthorizationV2::bind_profile_receipt(
                StagingBookNavigationProfileViewV2::new(package, &navigation, limits).unwrap(),
                navigation_profile_receipt,
                display.layout.profile.profile_receipt_fingerprint(),
                display.layout.profile.profile_fingerprint(),
                package,
                &navigation,
                limits,
            )
            .unwrap();
        let pages = display
            .display
            .pages()
            .iter()
            .map(|page| BookNavigationSelectedPage {
                page_index: page.page_index(),
                width_raw: 1_000 * SCALE,
                height_raw: 800 * SCALE,
            })
            .collect::<Vec<_>>();
        let navigation_selected = select_staging_book_navigation_v2(
            &navigation,
            &navigation_authorization,
            limits,
            sha256(b"tagged-vector-selected-layout-v2"),
            4,
            &pages,
            &[],
            &[],
            &[],
            &display.display,
        )
        .unwrap();
        let semantics =
            validate_staging_structure_semantics_v2(package, &navigation, limits).unwrap();
        let accessibility_authorization =
            StagingAccessibilityProfileAuthorizationV2::bind_profile_receipt(
                StagingAccessibilityProfileViewV2::new(package, &navigation, &semantics, limits)
                    .unwrap(),
                sha256(b"tagged-vector-accessibility-profile-v2"),
                navigation_profile_receipt,
                package,
                &navigation,
                &semantics,
                limits,
            )
            .unwrap();
        let registry = build_structure_registry_v2(
            package,
            &navigation,
            &semantics,
            &accessibility_authorization,
            limits,
        )
        .unwrap();
        let form_isolation = prove_vector_form_structure_isolation_v2(&display.display).unwrap();
        let plan = build_vector_marked_content_plan_v2(
            &registry,
            &accessibility_authorization,
            limits,
            &navigation,
            &navigation_authorization,
            &navigation_selected,
            &[],
            &[],
            &display.display,
            &form_isolation,
            &display.block_selected,
            &display.layout.math_flows,
        )
        .unwrap();
        VectorStructureFixture {
            display,
            navigation,
            navigation_authorization,
            navigation_selected,
            semantics,
            accessibility_authorization,
            registry,
            form_isolation,
            plan,
        }
    }

    #[test]
    fn vector_marked_content_serialization_v2_keeps_selection_owners_private_and_bound() {
        let fixture = vector_structure_fixture(false);
        let projection = fixture
            .plan
            .authorize_pdf_serialization(
                &fixture.registry,
                &fixture.accessibility_authorization,
                &fixture.display.layout.limits,
                &fixture.navigation,
                &fixture.navigation_authorization,
                &fixture.navigation_selected,
                &fixture.display.display,
                &fixture.form_isolation,
                &fixture.display.block_selected,
                &fixture.display.layout.math_flows,
            )
            .unwrap();
        assert_eq!(projection.plan(), &fixture.plan);
        assert_eq!(projection.equation_number_shapes().len(), 1);
        let placement = fixture
            .display
            .block_selected
            .placements()
            .iter()
            .find(|placement| placement.equation_number().is_some())
            .unwrap();
        let number = placement.equation_number().unwrap();
        assert_eq!(
            projection.equation_number_rect(
                placement.owner(),
                placement.page_index(),
                number.paint_ordinal(),
                number.shape_fingerprint()
            ),
            Some(number.rect()),
        );
        assert_eq!(
            projection.equation_number_rect(
                placement.owner(),
                placement.page_index(),
                number.paint_ordinal() + 1,
                number.shape_fingerprint()
            ),
            None,
        );
        let other = vector_structure_fixture(true);
        let swapped = VectorMarkedContentSerializationV2 {
            plan: projection.plan,
            block_selected: &other.display.block_selected,
            math_flows: &other.display.layout.math_flows,
        };
        assert!(swapped
            .verify(
                &fixture.registry,
                &fixture.accessibility_authorization,
                &fixture.display.layout.limits,
                &fixture.navigation,
                &fixture.navigation_authorization,
                &fixture.navigation_selected,
                &fixture.display.display,
                &fixture.form_isolation,
            )
            .is_err());
    }

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

    #[test]
    fn vector_marked_content_v2_closes_outer_mcr_inner_span_and_equation_order() {
        let fixture = vector_structure_fixture(false);
        fixture
            .plan
            .verify(
                &fixture.registry,
                &fixture.accessibility_authorization,
                &fixture.display.layout.limits,
                &fixture.navigation,
                &fixture.navigation_authorization,
                &fixture.navigation_selected,
                &fixture.display.display,
                &fixture.form_isolation,
                &fixture.display.block_selected,
                &fixture.display.layout.math_flows,
            )
            .unwrap();
        assert_eq!(fixture.semantics.records().len(), 9);
        assert_eq!(fixture.plan.selected_binding().paints().len(), 5);
        let marked = fixture.plan.marked_content();
        assert_eq!(marked.records().len(), 5);
        assert_eq!(
            marked
                .pages()
                .iter()
                .map(MarkedContentPage::marked_content_count)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
        for page in marked.pages() {
            let mcids = marked
                .records()
                .iter()
                .filter(|record| record.page_index() == page.page_index())
                .filter_map(|record| match record.owner() {
                    MarkedContentOwner::Structure(owner) => Some(owner.mcid()),
                    MarkedContentOwner::Artifact(_) => None,
                })
                .collect::<Vec<_>>();
            assert!(mcids.iter().copied().eq(0..mcids.len() as u32));
        }
        let vector_records = marked
            .records()
            .iter()
            .filter(|record| matches!(record.binding(), MarkedContentBindingKindV2::Vector { .. }))
            .collect::<Vec<_>>();
        assert_eq!(vector_records.len(), 4);
        assert!(vector_records.iter().all(
            |record| record.outer_actual_text().is_none() && record.outer_language().is_none()
        ));
        assert!(vector_records[0].inner_span().is_none());
        assert_eq!(
            vector_records[1]
                .inner_span()
                .and_then(MarkedContentInnerSpanV2::actual_text),
            Some("xたすy")
        );
        assert_eq!(
            vector_records[1].inner_span().unwrap().role(),
            StructureRole::Span
        );
        assert!(!vector_records[1].inner_span().unwrap().has_mcid());
        assert!(vector_records[2].inner_span().is_none());
        assert_eq!(
            vector_records[3]
                .inner_span()
                .and_then(MarkedContentInnerSpanV2::actual_text),
            Some("xたすy、式1")
        );
        assert!(!marked.canonical_jcs().contains("x+y"));

        assert_eq!(marked.formula_orders().len(), 2);
        assert_eq!(marked.formula_orders()[0].kids().len(), 1);
        let numbered = &marked.formula_orders()[1];
        assert_eq!(numbered.kids().len(), 2);
        let FormulaStructureKidV2::MarkedContentReference { page_index, mcid } = numbered.kids()[0]
        else {
            panic!("Formula must start with its vector MCR")
        };
        assert_eq!((page_index, mcid), (1, 1));
        let FormulaStructureKidV2::StructureChild(number_id) = numbered.kids()[1] else {
            panic!("equation-number Span must follow the vector MCR")
        };
        let number = fixture.registry.node(number_id).unwrap();
        assert_eq!(number.role(), StructureRole::Span);
        assert_eq!(number.parent(), Some(numbered.formula_structure_node_id()));
        assert_eq!(fixture.form_isolation.form_mcid_count(), 0);
        assert_eq!(fixture.form_isolation.form_structure_property_count(), 0);
        assert_eq!(fixture.form_isolation.page_do_usage_count(), 4);
        assert!(fixture
            .form_isolation
            .canonical_jcs()
            .contains(MARKED_CONTENT_PLAN_ALGORITHM_V2));

        let mut tampered = marked.clone();
        tampered.formula_orders[1].kids.swap(0, 1);
        assert_eq!(
            tampered.verify_sealed(
                &fixture.registry,
                fixture.plan.selected_binding(),
                &fixture.accessibility_authorization,
                &fixture.display.layout.limits,
                &fixture.navigation_selected,
                &fixture.display.display,
                &fixture.form_isolation,
                &fixture.display.block_selected,
                &fixture.display.layout.math_flows,
            ),
            Err(MarkedContentError::ReceiptMismatch)
        );

        let mut wrong_mcid = marked.clone();
        let MarkedContentOwner::Structure(ref mut owner) = wrong_mcid.records[0].owner else {
            panic!("vector record must own an outer structure MCR")
        };
        owner.mcid = 7;
        assert_eq!(
            wrong_mcid.verify_sealed(
                &fixture.registry,
                fixture.plan.selected_binding(),
                &fixture.accessibility_authorization,
                &fixture.display.layout.limits,
                &fixture.navigation_selected,
                &fixture.display.display,
                &fixture.form_isolation,
                &fixture.display.block_selected,
                &fixture.display.layout.math_flows,
            ),
            Err(MarkedContentError::ReceiptMismatch)
        );
    }

    #[test]
    fn vector_marked_content_v2_applies_kind_specific_language_matrix() {
        let fixture = vector_structure_fixture(true);
        let marked = fixture.plan.marked_content();
        let vectors = marked
            .records()
            .iter()
            .filter(|record| matches!(record.binding(), MarkedContentBindingKindV2::Vector { .. }))
            .collect::<Vec<_>>();
        assert_eq!(vectors.len(), 4);
        assert!(vectors.iter().all(|record| {
            record.outer_language().is_none()
                && record
                    .inner_span()
                    .is_some_and(|inner| inner.language() == Some("en-US") && !inner.has_mcid())
        }));
        assert_eq!(vectors[0].inner_span().unwrap().actual_text(), None);
        assert_eq!(
            vectors[1].inner_span().unwrap().actual_text(),
            Some("xたすy")
        );
        assert_eq!(vectors[2].inner_span().unwrap().actual_text(), None);
        assert_eq!(
            vectors[3].inner_span().unwrap().actual_text(),
            Some("xたすy、式1")
        );
        let equation = marked
            .records()
            .iter()
            .find(|record| {
                matches!(
                    record.binding(),
                    MarkedContentBindingKindV2::EquationNumber { .. }
                )
            })
            .unwrap();
        assert_eq!(equation.outer_language(), Some("en-US"));
        assert!(equation.inner_span().is_none());
    }

    #[test]
    fn vector_marked_content_v2_inner_span_matrix_is_exhaustive() {
        assert_eq!(
            vector_inner_span_v2(PrecomposedVectorKind::InlineVector, None, None).unwrap(),
            None
        );
        let authored = vector_inner_span_v2(
            PrecomposedVectorKind::InlineVector,
            Some("authored extraction"),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(authored.actual_text(), Some("authored extraction"));
        assert_eq!(authored.language(), None);
        let language_only =
            vector_inner_span_v2(PrecomposedVectorKind::InlineVector, None, Some("en-US"))
                .unwrap()
                .unwrap();
        assert_eq!(language_only.actual_text(), None);
        assert_eq!(language_only.language(), Some("en-US"));
        assert!(vector_inner_span_v2(PrecomposedVectorKind::MathVector, None, None).is_err());
        assert!(
            vector_inner_span_v2(PrecomposedVectorKind::MathVectorBlock, None, Some("en-US"))
                .is_err()
        );
        assert_eq!(
            vector_inner_span_v2(PrecomposedVectorKind::VectorFigure, None, None).unwrap(),
            None
        );
        assert!(
            vector_inner_span_v2(PrecomposedVectorKind::VectorFigure, Some("forbidden"), None)
                .is_err()
        );
        assert_eq!(
            map_selected_binding_error_v2(SelectedStructureBindingError::FragmentLimit),
            MarkedContentError::FragmentLimit
        );
        assert_eq!(
            map_selected_binding_error_v2(SelectedStructureBindingError::AllocationFailure),
            MarkedContentError::AllocationFailure
        );

        let (outer_language, outer_actual, inner) = standard_property_scopes_v2(
            StructureRole::Formula,
            Some("native math speech"),
            Some("en-US"),
        );
        assert_eq!((outer_language, outer_actual), (None, None));
        let inner = inner.unwrap();
        assert_eq!(inner.actual_text(), Some("native math speech"));
        assert_eq!(inner.language(), Some("en-US"));
        assert!(!inner.has_mcid());

        let (outer_language, outer_actual, inner) =
            standard_property_scopes_v2(StructureRole::Span, Some("source text"), Some("en-US"));
        assert_eq!(outer_language.as_deref(), Some("en-US"));
        assert_eq!(outer_actual.as_deref(), Some("source text"));
        assert_eq!(inner, None);
    }
}
