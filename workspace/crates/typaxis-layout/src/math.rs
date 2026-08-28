use std::collections::BTreeMap;

use typaxis_core::{push_jcs_string, sha256, FontFaceId, M4EffectiveResourceLimits, NodeId};
use typaxis_font::MathFontFace;
use typaxis_layout_contract::FlowId;
use typaxis_linebreak::{AtomicMathInlineItem, AtomicMathPlacement};
use typaxis_math::{
    compute_math, required_math_layout_units, MathComputationError, MathComputationInput,
    MathComputationReceipt, MathNodeKind, MATH_AST_FINGERPRINT_ID, MATH_COMPUTATION_ID,
    MATH_FORMATTER_ID, MATH_LAYOUT_WORK_ID, MATH_PARSER_ID, MATH_SOURCE_ID, MATH_VECTOR_IR_ID,
};
use typaxis_resource_admission::{AdmittedResourceLedger, ResourceAdmissionProgressToken};
use typaxis_style::{MachineTextAlign, StagingMathComputedStyle};
use typaxis_syntax::{
    StagingMathProfileAuthorization, StagingMathProfileProgressToken, StagingMathProfileView,
    StagingSemanticSyntaxError, ValidatedStagingMathNode, ValidatedStagingSemanticPackage,
};

use crate::layout_staging_semantic_containers;

pub const MATH_BINDING_ALGORITHM: &str = "typaxis.math-binding/1";
pub const MATH_DISPLAY_FLOW_ALGORITHM: &str = "typaxis.math-flow/1";
pub const MATH_SELECTED_LAYOUT_ALGORITHM: &str = "typaxis.math-selected-layout/1";
const MATH_LAYOUT_EPOCH_ALGORITHM: &str = "typaxis.math-layout-epoch/1";
const MATH_STYLE_FINGERPRINT_ALGORITHM: &str = "typaxis.math-computed-style/1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MathReceiptKey([u8; 32]);

impl MathReceiptKey {
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MathFlowId(u32);

impl MathFlowId {
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathLayoutEpoch {
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    profile_authorization_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathLayoutEpoch {
    fn new(
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingMathProfileAuthorization,
        admitted: &AdmittedResourceLedger,
    ) -> Self {
        let admitted_fingerprint = admitted.fingerprint().bytes();
        let mut canonical_jcs = String::from("{\"algorithm\":");
        push_jcs_string(&mut canonical_jcs, MATH_LAYOUT_EPOCH_ALGORITHM);
        canonical_jcs.push_str(",\"admitted_fingerprint\":");
        push_hash(&mut canonical_jcs, admitted_fingerprint);
        canonical_jcs.push_str(",\"package_fingerprint\":");
        push_hash(&mut canonical_jcs, package.semantic_fingerprint());
        canonical_jcs.push_str(",\"profile_authorization_fingerprint\":");
        push_hash(&mut canonical_jcs, profile.profile_fingerprint());
        canonical_jcs.push_str(",\"profile_fingerprint\":");
        push_hash(&mut canonical_jcs, profile.profile_receipt_fingerprint());
        canonical_jcs.push('}');
        Self {
            package_fingerprint: package.semantic_fingerprint(),
            profile_fingerprint: profile.profile_receipt_fingerprint(),
            profile_authorization_fingerprint: profile.profile_fingerprint(),
            admitted_fingerprint,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        }
    }

    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package_fingerprint
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub const fn profile_authorization_fingerprint(&self) -> [u8; 32] {
        self.profile_authorization_fingerprint
    }
    pub const fn admitted_fingerprint(&self) -> [u8; 32] {
        self.admitted_fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedMathReceipt {
    key: MathReceiptKey,
    node_id: NodeId,
    kind: MathNodeKind,
    source_sha256: [u8; 32],
    speech_sha256: [u8; 32],
    style_fingerprint: [u8; 32],
    font_face_id: FontFaceId,
    font_sha256: [u8; 32],
    face_index: u32,
    computation: MathComputationReceipt,
    canonical_jcs: String,
}

impl ValidatedMathReceipt {
    pub const fn key(&self) -> MathReceiptKey {
        self.key
    }
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn kind(&self) -> MathNodeKind {
        self.kind
    }
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub const fn speech_sha256(&self) -> [u8; 32] {
        self.speech_sha256
    }
    pub const fn style_fingerprint(&self) -> [u8; 32] {
        self.style_fingerprint
    }
    pub const fn font_face_id(&self) -> FontFaceId {
        self.font_face_id
    }
    pub const fn font_sha256(&self) -> [u8; 32] {
        self.font_sha256
    }
    pub const fn face_index(&self) -> u32 {
        self.face_index
    }
    pub const fn computation(&self) -> &MathComputationReceipt {
        &self.computation
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathFlow {
    flow_id: MathFlowId,
    owner: NodeId,
    parent_flow_id: FlowId,
    parent_position: u32,
    receipt_key: MathReceiptKey,
    terminal: u32,
    fingerprint: [u8; 32],
}

impl StagingMathFlow {
    pub const fn flow_id(&self) -> MathFlowId {
        self.flow_id
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn parent_flow_id(&self) -> FlowId {
        self.parent_flow_id
    }
    pub const fn parent_position(&self) -> u32 {
        self.parent_position
    }
    pub const fn receipt_key(&self) -> MathReceiptKey {
        self.receipt_key
    }
    pub const fn terminal(&self) -> u32 {
        self.terminal
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathPlacement {
    occurrence: u32,
    node_id: NodeId,
    receipt_key: MathReceiptKey,
    parent_flow_id: FlowId,
    display_flow_id: Option<MathFlowId>,
    page_index: u32,
    frame_index: u32,
    fragment_ordinal: u32,
    paint_ordinal: u32,
    origin_x: i64,
    baseline_y: i64,
    fingerprint: [u8; 32],
}

impl StagingMathPlacement {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn receipt_key(&self) -> MathReceiptKey {
        self.receipt_key
    }
    pub const fn parent_flow_id(&self) -> FlowId {
        self.parent_flow_id
    }
    pub const fn display_flow_id(&self) -> Option<MathFlowId> {
        self.display_flow_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn frame_index(&self) -> u32 {
        self.frame_index
    }
    pub const fn fragment_ordinal(&self) -> u32 {
        self.fragment_ordinal
    }
    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
    pub const fn origin_x(&self) -> i64 {
        self.origin_x
    }
    pub const fn baseline_y(&self) -> i64 {
        self.baseline_y
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Debug)]
pub struct StagingMathLayout {
    epoch: StagingMathLayoutEpoch,
    profile_progress: StagingMathProfileProgressToken,
    admission_progress: ResourceAdmissionProgressToken,
    semantic_flow_registry_fingerprint: [u8; 32],
    receipts: Vec<ValidatedMathReceipt>,
    display_flows: Vec<StagingMathFlow>,
    placements: Vec<StagingMathPlacement>,
    total_layout_work: u64,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathLayout {
    pub const fn epoch(&self) -> &StagingMathLayoutEpoch {
        &self.epoch
    }
    pub const fn admission_progress(&self) -> &ResourceAdmissionProgressToken {
        &self.admission_progress
    }
    pub const fn profile_progress(&self) -> &StagingMathProfileProgressToken {
        &self.profile_progress
    }
    pub fn receipts(&self) -> &[ValidatedMathReceipt] {
        &self.receipts
    }
    pub fn display_flows(&self) -> &[StagingMathFlow] {
        &self.display_flows
    }
    pub fn placements(&self) -> &[StagingMathPlacement] {
        &self.placements
    }
    pub const fn total_layout_work(&self) -> u64 {
        self.total_layout_work
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn receipt(&self, key: MathReceiptKey) -> Option<&ValidatedMathReceipt> {
        self.receipts.iter().find(|receipt| receipt.key == key)
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingMathProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
    ) -> Result<(), StagingMathLayoutError> {
        profile
            .authorizes(package, limits)
            .map_err(|_| StagingMathLayoutError::ProfileMismatch)?;
        let epoch = StagingMathLayoutEpoch::new(package, profile, admitted);
        let semantic = layout_staging_semantic_containers(package, profile.base().base(), u32::MAX)
            .map_err(|_| StagingMathLayoutError::ParentFlow)?;
        if self.epoch != epoch
            || !profile.matches_progress(&self.profile_progress)
            || !admitted.token().matches_progress(&self.admission_progress)
            || self.semantic_flow_registry_fingerprint
                != semantic.registry().receipt().fingerprint()
            || self.receipts.len() != package.math_nodes().len()
            || self.placements.len() != self.receipts.len()
            || self.total_layout_work > limits.extension().get().max_math_layout_units
            || u64::try_from(self.placements.len())
                .map_or(true, |count| count > limits.base().get().max_fragments)
        {
            return Err(StagingMathLayoutError::ReceiptMismatch);
        }
        let mut observed_work = 0u64;
        let mut expected_display_id = 0u32;
        let mut parsed_faces = BTreeMap::new();
        for (index, ((node, receipt), placement)) in package
            .math_nodes()
            .iter()
            .zip(&self.receipts)
            .zip(&self.placements)
            .enumerate()
        {
            let (parent_flow_id, parent_position) =
                parent_flow_for_node(node, semantic.registry())?;
            if usize::try_from(placement.occurrence) != Ok(index)
                || receipt.node_id != node.domain().node_id
                || placement.node_id != node.domain().node_id
                || placement.receipt_key != receipt.key
                || placement.parent_flow_id != parent_flow_id
                || receipt.source_sha256 != node.parsed().source_sha256()
                || receipt.speech_sha256 != sha256(node.domain().speech.as_bytes())
                || receipt.style_fingerprint != math_style_fingerprint(node.computed_style())
                || receipt.computation.parsed_fingerprint() != node.parsed().fingerprint()
                || receipt.computation.kind() != receipt.kind
                || receipt.computation.font_size_raw()
                    != node.computed_style().font_size().get().raw()
                || sha256(receipt.canonical_jcs.as_bytes()) != receipt.key.0
                || sha256(encode_placement(placement).as_bytes()) != placement.fingerprint
            {
                return Err(StagingMathLayoutError::ReceiptMismatch);
            }
            let selected_face = admitted
                .font_families()
                .resolve(node.computed_style().font_families())
                .map_err(|_| StagingMathLayoutError::UnknownMathFont(node.domain().node_id))?;
            let font =
                admitted
                    .font(selected_face)
                    .ok_or(StagingMathLayoutError::UnknownMathFont(
                        node.domain().node_id,
                    ))?;
            let face = match parsed_faces.get(&selected_face) {
                Some(face) => *face,
                None => {
                    let face = MathFontFace::parse(font.bytes(), font.face_index())
                        .map_err(|_| StagingMathLayoutError::ReceiptMismatch)?;
                    parsed_faces.insert(selected_face, face);
                    face
                }
            };
            let expected_receipt_jcs = encode_receipt(
                package,
                profile,
                limits,
                &epoch,
                admitted,
                node,
                selected_face,
                font.content_hash(),
                font.face_index(),
                math_style_fingerprint(node.computed_style()),
                &receipt.computation,
            );
            if receipt.font_face_id != selected_face
                || receipt.font_sha256 != font.content_hash()
                || receipt.face_index != font.face_index()
                || receipt.canonical_jcs != expected_receipt_jcs
                || receipt
                    .computation
                    .verify_sealed(node.parsed(), face)
                    .is_err()
            {
                return Err(StagingMathLayoutError::ReceiptMismatch);
            }
            observed_work = observed_work
                .checked_add(receipt.computation.layout_work())
                .ok_or(StagingMathLayoutError::LayoutUnitLimit)?;
            if receipt.kind == MathNodeKind::Display {
                let flow = self
                    .display_flows
                    .get(expected_display_id as usize)
                    .ok_or(StagingMathLayoutError::ReceiptMismatch)?;
                if flow.flow_id.0 != expected_display_id
                    || flow.owner != receipt.node_id
                    || flow.parent_flow_id != parent_flow_id
                    || flow.parent_position != parent_position
                    || flow.receipt_key != receipt.key
                    || flow.terminal != 1
                    || sha256(encode_flow(flow).as_bytes()) != flow.fingerprint
                    || placement.display_flow_id != Some(flow.flow_id)
                {
                    return Err(StagingMathLayoutError::ReceiptMismatch);
                }
                expected_display_id = expected_display_id
                    .checked_add(1)
                    .ok_or(StagingMathLayoutError::ReceiptMismatch)?;
            } else if placement.display_flow_id.is_some() {
                return Err(StagingMathLayoutError::ReceiptMismatch);
            }
        }
        let expected_placements = select_math_placements(
            package,
            profile.view(),
            &self.receipts,
            &self.display_flows,
            semantic.registry(),
            limits,
        )?;
        let canonical_jcs = encode_layout(
            &self.epoch,
            self.semantic_flow_registry_fingerprint,
            &self.receipts,
            &self.display_flows,
            &self.placements,
            observed_work,
        );
        if usize::try_from(expected_display_id) != Ok(self.display_flows.len())
            || self.placements != expected_placements
            || observed_work != self.total_layout_work
            || canonical_jcs != self.canonical_jcs
            || sha256(canonical_jcs.as_bytes()) != self.fingerprint
        {
            return Err(StagingMathLayoutError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMathLayoutError {
    ProfileMismatch,
    ParentFlow,
    UnknownMathFont(NodeId),
    InvalidMathFont(NodeId),
    LayoutUnitLimit,
    FragmentLimit,
    PageLimit,
    Oversize(NodeId),
    ArithmeticOverflow,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingMathLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => formatter.write_str("I9190: math profile mismatch"),
            Self::ParentFlow => formatter.write_str("I9190: math parent flow mismatch"),
            Self::UnknownMathFont(node) => {
                write!(
                    formatter,
                    "R7100: math node {} has no admitted selected face",
                    node.get()
                )
            }
            Self::InvalidMathFont(node) => {
                write!(
                    formatter,
                    "R7100: math node {} selected an invalid MATH face",
                    node.get()
                )
            }
            Self::LayoutUnitLimit => formatter.write_str("L5111: math layout work limit exceeded"),
            Self::FragmentLimit => {
                formatter.write_str("L5110: math selected-fragment limit exceeded")
            }
            Self::PageLimit => formatter.write_str("L5100: math page limit exceeded"),
            Self::Oversize(node) => {
                write!(
                    formatter,
                    "L5100: atomic math node {} exceeds an empty frame",
                    node.get()
                )
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("L5100: math layout arithmetic overflow")
            }
            Self::ReceiptMismatch => formatter.write_str("I9190: math receipt mismatch"),
            Self::AllocationFailure => formatter.write_str("L5111: math layout allocation failed"),
        }
    }
}

impl std::error::Error for StagingMathLayoutError {}

pub fn layout_staging_math(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingMathProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
) -> Result<StagingMathLayout, StagingMathLayoutError> {
    profile
        .authorizes(package, limits)
        .map_err(|_| StagingMathLayoutError::ProfileMismatch)?;
    if u64::try_from(package.math_nodes().len())
        .map_or(true, |count| count > limits.base().get().max_fragments)
    {
        return Err(StagingMathLayoutError::FragmentLimit);
    }
    let semantic = layout_staging_semantic_containers(package, profile.base().base(), u32::MAX)
        .map_err(|_| StagingMathLayoutError::ParentFlow)?;
    let epoch = StagingMathLayoutEpoch::new(package, profile, admitted);
    let profile_progress = profile.progress_token();
    let admission_progress = admitted.progress_token();
    let mut layout_budget = profile
        .layout_budget(package, limits)
        .map_err(|_| StagingMathLayoutError::ProfileMismatch)?;
    let mut receipts = Vec::new();
    let mut display_flows = Vec::new();
    let mut total_layout_work = 0u64;
    let mut parsed_faces = BTreeMap::new();
    for node in package.math_nodes() {
        let required_work = required_math_layout_units(node.parsed()).map_err(map_math_error)?;
        layout_budget
            .reserve(required_work)
            .map_err(|error| match error {
                StagingSemanticSyntaxError::MathLayoutUnitLimit => {
                    StagingMathLayoutError::LayoutUnitLimit
                }
                _ => StagingMathLayoutError::ProfileMismatch,
            })?;
        let selected_face = admitted
            .font_families()
            .resolve(node.computed_style().font_families())
            .map_err(|_| StagingMathLayoutError::UnknownMathFont(node.domain().node_id))?;
        let font = admitted
            .font(selected_face)
            .ok_or(StagingMathLayoutError::UnknownMathFont(
                node.domain().node_id,
            ))?;
        let face = match parsed_faces.get(&selected_face) {
            Some(face) => *face,
            None => {
                let face = MathFontFace::parse(font.bytes(), font.face_index())
                    .map_err(|_| StagingMathLayoutError::InvalidMathFont(node.domain().node_id))?;
                parsed_faces.insert(selected_face, face);
                face
            }
        };
        let kind = lower_kind(node);
        let input = MathComputationInput::new(
            kind,
            node.computed_style().font_size().get().raw(),
            required_work,
        )
        .ok_or(StagingMathLayoutError::LayoutUnitLimit)?;
        let computation =
            compute_math(node.parsed(), face, input).map_err(|error| match error {
                MathComputationError::Font(_) => {
                    StagingMathLayoutError::InvalidMathFont(node.domain().node_id)
                }
                other => map_math_error(other),
            })?;
        if computation.layout_work() != required_work {
            return Err(StagingMathLayoutError::ReceiptMismatch);
        }
        total_layout_work = total_layout_work
            .checked_add(computation.layout_work())
            .ok_or(StagingMathLayoutError::LayoutUnitLimit)?;
        let style_fingerprint = math_style_fingerprint(node.computed_style());
        let canonical_jcs = encode_receipt(
            package,
            profile,
            limits,
            &epoch,
            admitted,
            node,
            selected_face,
            font.content_hash(),
            font.face_index(),
            style_fingerprint,
            &computation,
        );
        let receipt = ValidatedMathReceipt {
            key: MathReceiptKey(sha256(canonical_jcs.as_bytes())),
            node_id: node.domain().node_id,
            kind,
            source_sha256: node.parsed().source_sha256(),
            speech_sha256: sha256(node.domain().speech.as_bytes()),
            style_fingerprint,
            font_face_id: selected_face,
            font_sha256: font.content_hash(),
            face_index: font.face_index(),
            computation,
            canonical_jcs,
        };
        if kind == MathNodeKind::Display {
            let (parent_flow_id, parent_position) =
                parent_flow_for_node(node, semantic.registry())?;
            let flow_id = MathFlowId(
                u32::try_from(display_flows.len())
                    .map_err(|_| StagingMathLayoutError::FragmentLimit)?,
            );
            let mut flow = StagingMathFlow {
                flow_id,
                owner: node.domain().node_id,
                parent_flow_id,
                parent_position,
                receipt_key: receipt.key,
                terminal: 1,
                fingerprint: [0; 32],
            };
            flow.fingerprint = sha256(encode_flow(&flow).as_bytes());
            display_flows.push(flow);
        }
        receipts.push(receipt);
    }
    drop(layout_budget);
    let placements = select_math_placements(
        package,
        profile.view(),
        &receipts,
        &display_flows,
        semantic.registry(),
        limits,
    )?;
    let semantic_flow_registry_fingerprint = semantic.registry().receipt().fingerprint();
    let canonical_jcs = encode_layout(
        &epoch,
        semantic_flow_registry_fingerprint,
        &receipts,
        &display_flows,
        &placements,
        total_layout_work,
    );
    let layout = StagingMathLayout {
        epoch,
        profile_progress,
        admission_progress,
        semantic_flow_registry_fingerprint,
        receipts,
        display_flows,
        placements,
        total_layout_work,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    layout.verify(package, profile, limits, admitted)?;
    Ok(layout)
}

fn map_math_error(error: MathComputationError) -> StagingMathLayoutError {
    match error {
        MathComputationError::LayoutUnitLimit | MathComputationError::AllocationFailure => {
            StagingMathLayoutError::LayoutUnitLimit
        }
        MathComputationError::ArithmeticOverflow | MathComputationError::EmptyPaint => {
            StagingMathLayoutError::ArithmeticOverflow
        }
        MathComputationError::Font(_) => StagingMathLayoutError::ReceiptMismatch,
        MathComputationError::ParsedReceipt | MathComputationError::ReceiptMismatch => {
            StagingMathLayoutError::ReceiptMismatch
        }
    }
}

fn lower_kind(node: &ValidatedStagingMathNode) -> MathNodeKind {
    match node.domain().kind {
        typaxis_document::StagingM4MathKind::Inline => MathNodeKind::Inline,
        typaxis_document::StagingM4MathKind::Display => MathNodeKind::Display,
    }
}

fn parent_flow_for_node(
    node: &ValidatedStagingMathNode,
    registry: &crate::StagingSemanticContainerFlowRegistry,
) -> Result<(FlowId, u32), StagingMathLayoutError> {
    let owner = match node.domain().kind {
        typaxis_document::StagingM4MathKind::Inline => node.domain().owner_node_id,
        typaxis_document::StagingM4MathKind::Display => node.domain().node_id,
    };
    let mut found = None;
    for flow in registry.flows() {
        for item in flow.items().iter().filter(|item| item.owner() == owner) {
            if found.replace((flow.flow_id(), item.position())).is_some() {
                return Err(StagingMathLayoutError::ParentFlow);
            }
        }
    }
    found.ok_or(StagingMathLayoutError::ParentFlow)
}

#[allow(clippy::too_many_arguments)]
fn select_math_placements(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingMathProfileView,
    receipts: &[ValidatedMathReceipt],
    display_flows: &[StagingMathFlow],
    registry: &crate::StagingSemanticContainerFlowRegistry,
    limits: &M4EffectiveResourceLimits,
) -> Result<Vec<StagingMathPlacement>, StagingMathLayoutError> {
    let body = profile.page_geometry().body();
    let body_left = body.x().raw();
    let body_top = body.y().raw();
    let body_width = body.width().get().raw();
    let body_height = body.height().get().raw();
    let body_bottom = checked_add(body_top, body_height)?;
    let mut page = 0u32;
    let mut x = body_left;
    let mut y = body_top;
    let mut inline_line_height = 0i64;
    let mut paint_ordinal = 0u32;
    let mut previous_parent = None;
    let mut active_page_name = None;
    let mut page_has_content = false;
    let display_by_node: BTreeMap<_, _> = display_flows
        .iter()
        .map(|flow| (flow.owner, flow.flow_id))
        .collect();
    let mut placements = Vec::new();
    placements
        .try_reserve_exact(receipts.len())
        .map_err(|_| StagingMathLayoutError::AllocationFailure)?;
    for (index, (node, receipt)) in package.math_nodes().iter().zip(receipts).enumerate() {
        let dimensions = receipt.computation.dimensions();
        let intrinsic_height = checked_add(dimensions.ascent(), dimensions.descent())?;
        let line_height = intrinsic_height.max(node.computed_style().line_height().get().raw());
        if line_height > body_height {
            return Err(StagingMathLayoutError::Oversize(node.domain().node_id));
        }
        let extra = line_height - intrinsic_height;
        let leading_before = round_half_even(extra)?;
        let parent = parent_flow_for_node(node, registry)?;
        let (parent_flow_id, _) = parent;
        if previous_parent.is_some_and(|previous| previous != parent) && x != body_left {
            y = checked_add(y, inline_line_height)?;
            x = body_left;
            inline_line_height = 0;
        }
        previous_parent = Some(parent);
        if receipt.kind == MathNodeKind::Display {
            let requested_page_name = node.computed_style().page_name().cloned();
            if requested_page_name != active_page_name {
                if page_has_content {
                    page = next_page(page, limits)?;
                    y = body_top;
                    x = body_left;
                    inline_line_height = 0;
                }
                active_page_name = requested_page_name;
            }
        }
        let (origin_x, baseline_y, display_flow_id) = match receipt.kind {
            MathNodeKind::Inline => {
                let atomic = AtomicMathInlineItem::from_computation(&receipt.computation)
                    .map_err(|_| StagingMathLayoutError::ReceiptMismatch)?;
                let remaining = checked_sub(checked_add(body_left, body_width)?, x)?;
                match atomic
                    .place(remaining, body_width)
                    .map_err(|_| StagingMathLayoutError::ReceiptMismatch)?
                {
                    AtomicMathPlacement::FitsCurrentLine => {}
                    AtomicMathPlacement::MoveIntactToNextLine => {
                        y = checked_add(y, inline_line_height)?;
                        x = body_left;
                        inline_line_height = 0;
                    }
                    AtomicMathPlacement::Oversize => {
                        return Err(StagingMathLayoutError::Oversize(node.domain().node_id));
                    }
                }
                if checked_add(y, line_height)? > body_bottom {
                    page = next_page(page, limits)?;
                    y = body_top;
                    x = body_left;
                    inline_line_height = 0;
                }
                let baseline = checked_add(checked_add(y, leading_before)?, dimensions.ascent())?;
                let origin = x;
                x = checked_add(x, dimensions.advance())?;
                inline_line_height = inline_line_height.max(line_height);
                (origin, baseline, None)
            }
            MathNodeKind::Display => {
                if x != body_left {
                    y = checked_add(y, inline_line_height)?;
                    x = body_left;
                    inline_line_height = 0;
                }
                let block = node.computed_style().block_style();
                let outer_height = checked_add(
                    checked_add(block.space_before().get().raw(), line_height)?,
                    block.space_after().get().raw(),
                )?;
                if block.keep_with_next() {
                    if let (Some(next_node), Some(next_receipt)) =
                        (package.math_nodes().get(index + 1), receipts.get(index + 1))
                    {
                        let next_parent = parent_flow_for_node(next_node, registry)?;
                        if adjacent_flow_positions(parent, next_parent) {
                            let next_height = math_outer_height(next_node, next_receipt)?;
                            let kept_height = checked_add(outer_height, next_height)?;
                            if kept_height <= body_height
                                && checked_add(y, kept_height)? > body_bottom
                            {
                                page = next_page(page, limits)?;
                                y = body_top;
                            }
                        }
                    }
                }
                let residual = body_width
                    .checked_sub(block.start_indent().get().raw())
                    .and_then(|value| value.checked_sub(block.end_indent().get().raw()))
                    .filter(|value| *value > 0)
                    .ok_or(StagingMathLayoutError::Oversize(node.domain().node_id))?;
                if dimensions.advance() > residual {
                    return Err(StagingMathLayoutError::Oversize(node.domain().node_id));
                }
                let effective_before = if y == body_top {
                    0
                } else {
                    block.space_before().get().raw()
                };
                if checked_add(checked_add(y, effective_before)?, line_height)? > body_bottom {
                    page = next_page(page, limits)?;
                    y = body_top;
                } else {
                    y = checked_add(y, effective_before)?;
                }
                let slack = residual - dimensions.advance();
                let align_offset = match block.text_align() {
                    MachineTextAlign::Start => 0,
                    MachineTextAlign::Center => slack / 2,
                    MachineTextAlign::End => slack,
                };
                let origin = checked_add(
                    checked_add(body_left, block.start_indent().get().raw())?,
                    align_offset,
                )?;
                let baseline = checked_add(checked_add(y, leading_before)?, dimensions.ascent())?;
                y = checked_add(
                    checked_add(y, line_height)?,
                    block.space_after().get().raw(),
                )?;
                (
                    origin,
                    baseline,
                    display_by_node.get(&node.domain().node_id).copied(),
                )
            }
        };
        if page >= limits.base().get().max_pages {
            return Err(StagingMathLayoutError::PageLimit);
        }
        let mut placement = StagingMathPlacement {
            occurrence: u32::try_from(index).map_err(|_| StagingMathLayoutError::FragmentLimit)?,
            node_id: node.domain().node_id,
            receipt_key: receipt.key,
            parent_flow_id,
            display_flow_id,
            page_index: page,
            frame_index: 0,
            fragment_ordinal: u32::try_from(index)
                .map_err(|_| StagingMathLayoutError::FragmentLimit)?,
            paint_ordinal,
            origin_x,
            baseline_y,
            fingerprint: [0; 32],
        };
        paint_ordinal = paint_ordinal
            .checked_add(
                u32::try_from(receipt.computation.paints().len())
                    .map_err(|_| StagingMathLayoutError::FragmentLimit)?,
            )
            .ok_or(StagingMathLayoutError::FragmentLimit)?;
        placement.fingerprint = sha256(encode_placement(&placement).as_bytes());
        placements.push(placement);
        page_has_content = true;
    }
    Ok(placements)
}

fn adjacent_flow_positions(current: (FlowId, u32), next: (FlowId, u32)) -> bool {
    current.0 == next.0 && current.1.checked_add(1) == Some(next.1)
}

fn math_outer_height(
    node: &ValidatedStagingMathNode,
    receipt: &ValidatedMathReceipt,
) -> Result<i64, StagingMathLayoutError> {
    let dimensions = receipt.computation.dimensions();
    let intrinsic = checked_add(dimensions.ascent(), dimensions.descent())?;
    let line = intrinsic.max(node.computed_style().line_height().get().raw());
    if receipt.kind == MathNodeKind::Display {
        let block = node.computed_style().block_style();
        checked_add(
            checked_add(block.space_before().get().raw(), line)?,
            block.space_after().get().raw(),
        )
    } else {
        Ok(line)
    }
}

fn next_page(
    current: u32,
    limits: &M4EffectiveResourceLimits,
) -> Result<u32, StagingMathLayoutError> {
    let next = current
        .checked_add(1)
        .ok_or(StagingMathLayoutError::PageLimit)?;
    if next >= limits.base().get().max_pages {
        return Err(StagingMathLayoutError::PageLimit);
    }
    Ok(next)
}

fn round_half_even(value: i64) -> Result<i64, StagingMathLayoutError> {
    if value < 0 {
        return Err(StagingMathLayoutError::ArithmeticOverflow);
    }
    let quotient = value / 2;
    if value % 2 == 1 && quotient % 2 == 1 {
        quotient
            .checked_add(1)
            .ok_or(StagingMathLayoutError::ArithmeticOverflow)
    } else {
        Ok(quotient)
    }
}

fn math_style_fingerprint(style: &StagingMathComputedStyle) -> [u8; 32] {
    let block = style.block_style();
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MATH_STYLE_FINGERPRINT_ALGORITHM);
    output.push_str(",\"end_indent\":");
    output.push_str(&block.end_indent().get().raw().to_string());
    output.push_str(",\"font_families\":[");
    for (index, family) in style.font_families().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_jcs_string(&mut output, family);
    }
    output.push_str("],\"font_size\":");
    output.push_str(&style.font_size().get().raw().to_string());
    output.push_str(",\"keep_with_next\":");
    output.push_str(if block.keep_with_next() {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, style.kind().as_str());
    output.push_str(",\"line_height\":");
    output.push_str(&style.line_height().get().raw().to_string());
    output.push_str(",\"page\":");
    match style.page_name() {
        Some(value) => push_jcs_string(&mut output, value.as_str()),
        None => output.push_str("null"),
    }
    output.push_str(",\"space_after\":");
    output.push_str(&block.space_after().get().raw().to_string());
    output.push_str(",\"space_before\":");
    output.push_str(&block.space_before().get().raw().to_string());
    output.push_str(",\"start_indent\":");
    output.push_str(&block.start_indent().get().raw().to_string());
    output.push_str(",\"text_align\":");
    push_jcs_string(&mut output, block.text_align().as_str());
    output.push('}');
    sha256(output.as_bytes())
}

#[allow(clippy::too_many_arguments)]
fn encode_receipt(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingMathProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    epoch: &StagingMathLayoutEpoch,
    admitted: &AdmittedResourceLedger,
    node: &ValidatedStagingMathNode,
    font_face_id: FontFaceId,
    font_sha256: [u8; 32],
    face_index: u32,
    style_fingerprint: [u8; 32],
    computation: &MathComputationReceipt,
) -> String {
    let domain = node.domain();
    let dimensions = computation.dimensions();
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MATH_BINDING_ALGORITHM);
    output.push_str(",\"admitted_fingerprint\":");
    push_hash(&mut output, admitted.fingerprint().bytes());
    output.push_str(",\"ast_fingerprint\":");
    push_hash(&mut output, node.parsed().ast_fingerprint());
    output.push_str(",\"ast_fingerprint_algorithm\":");
    push_jcs_string(&mut output, MATH_AST_FINGERPRINT_ID);
    output.push_str(",\"computation_fingerprint\":");
    push_hash(&mut output, computation.fingerprint());
    output.push_str(",\"contract\":\"typaxis.contract/1.4\",\"dimensions\":{\"advance\":");
    output.push_str(&dimensions.advance().to_string());
    output.push_str(",\"ascent\":");
    output.push_str(&dimensions.ascent().to_string());
    output.push_str(",\"axis\":");
    output.push_str(&dimensions.axis().to_string());
    output.push_str(",\"baseline\":");
    output.push_str(&dimensions.baseline().to_string());
    output.push_str(",\"descent\":");
    output.push_str(&dimensions.descent().to_string());
    output.push_str("},\"face_index\":");
    output.push_str(&face_index.to_string());
    output.push_str(",\"font_face_id\":");
    output.push_str(&font_face_id.get().to_string());
    output.push_str(",\"font_sha256\":");
    push_hash(&mut output, font_sha256);
    output.push_str(",\"formatter\":");
    push_jcs_string(&mut output, MATH_FORMATTER_ID);
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, domain.kind.as_str());
    output.push_str(",\"language\":");
    push_jcs_string(&mut output, &domain.language);
    output.push_str(",\"layout_algorithm\":");
    push_jcs_string(&mut output, MATH_COMPUTATION_ID);
    output.push_str(",\"layout_epoch\":");
    push_hash(&mut output, epoch.fingerprint());
    output.push_str(",\"layout_work\":");
    output.push_str(&computation.layout_work().to_string());
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits.fingerprint());
    output.push_str(",\"math_table_fingerprint\":");
    push_hash(&mut output, computation.math_table_fingerprint());
    output.push_str(",\"node_id\":");
    output.push_str(&domain.node_id.get().to_string());
    output.push_str(",\"package_fingerprint\":");
    push_hash(&mut output, package.semantic_fingerprint());
    output.push_str(",\"parser\":");
    push_jcs_string(&mut output, MATH_PARSER_ID);
    output.push_str(",\"profile_authorization_fingerprint\":");
    push_hash(&mut output, profile.profile_fingerprint());
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, profile.profile_receipt_fingerprint());
    output.push_str(",\"source_identity\":");
    push_jcs_string(&mut output, MATH_SOURCE_ID);
    output.push_str(",\"source_sha256\":");
    push_hash(&mut output, node.parsed().source_sha256());
    output.push_str(",\"source_span\":{\"end_byte\":");
    output.push_str(&domain.span.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&domain.span.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&domain.span.start_byte().get().to_string());
    output.push_str("},\"speech_sha256\":");
    push_hash(&mut output, sha256(domain.speech.as_bytes()));
    output.push_str(",\"style_fingerprint\":");
    push_hash(&mut output, style_fingerprint);
    output.push_str(",\"text_span\":{\"end_byte\":");
    output.push_str(&domain.text_span.end_byte().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&domain.text_span.start_byte().get().to_string());
    output.push_str(",\"text_id\":");
    output.push_str(&domain.text_span.text_id().get().to_string());
    output.push_str("},\"vector_algorithm\":");
    push_jcs_string(&mut output, MATH_VECTOR_IR_ID);
    output.push_str(",\"vector_fingerprint\":");
    push_hash(&mut output, computation.vector_fingerprint());
    output.push_str(",\"version\":");
    push_jcs_string(&mut output, &domain.version);
    output.push_str(",\"work_algorithm\":");
    push_jcs_string(&mut output, MATH_LAYOUT_WORK_ID);
    output.push('}');
    output
}

fn encode_flow(flow: &StagingMathFlow) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MATH_DISPLAY_FLOW_ALGORITHM);
    output.push_str(",\"flow_id\":");
    output.push_str(&flow.flow_id.0.to_string());
    output.push_str(",\"owner\":");
    output.push_str(&flow.owner.get().to_string());
    output.push_str(",\"parent_flow_id\":");
    output.push_str(&flow.parent_flow_id.get().to_string());
    output.push_str(",\"parent_position\":");
    output.push_str(&flow.parent_position.to_string());
    output.push_str(",\"receipt_key\":");
    push_hash(&mut output, flow.receipt_key.0);
    output.push_str(",\"terminal\":1}");
    output
}

fn encode_placement(value: &StagingMathPlacement) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MATH_SELECTED_LAYOUT_ALGORITHM);
    output.push_str(",\"baseline_y\":");
    output.push_str(&value.baseline_y.to_string());
    output.push_str(",\"display_flow_id\":");
    match value.display_flow_id {
        Some(flow) => output.push_str(&flow.0.to_string()),
        None => output.push_str("null"),
    }
    output.push_str(",\"fragment_ordinal\":");
    output.push_str(&value.fragment_ordinal.to_string());
    output.push_str(",\"frame_index\":");
    output.push_str(&value.frame_index.to_string());
    output.push_str(",\"node_id\":");
    output.push_str(&value.node_id.get().to_string());
    output.push_str(",\"occurrence\":");
    output.push_str(&value.occurrence.to_string());
    output.push_str(",\"origin_x\":");
    output.push_str(&value.origin_x.to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&value.page_index.to_string());
    output.push_str(",\"paint_ordinal\":");
    output.push_str(&value.paint_ordinal.to_string());
    output.push_str(",\"parent_flow_id\":");
    output.push_str(&value.parent_flow_id.get().to_string());
    output.push_str(",\"receipt_key\":");
    push_hash(&mut output, value.receipt_key.0);
    output.push('}');
    output
}

fn encode_layout(
    epoch: &StagingMathLayoutEpoch,
    semantic_flow_registry_fingerprint: [u8; 32],
    receipts: &[ValidatedMathReceipt],
    flows: &[StagingMathFlow],
    placements: &[StagingMathPlacement],
    total_work: u64,
) -> String {
    let mut output =
        String::from("{\"algorithm\":\"typaxis.math-layout-binding/1\",\"display_flows\":[");
    for (index, flow) in flows.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_hash(&mut output, flow.fingerprint);
    }
    output.push_str("],\"epoch\":");
    push_hash(&mut output, epoch.fingerprint());
    output.push_str(",\"placements\":[");
    for (index, placement) in placements.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_hash(&mut output, placement.fingerprint);
    }
    output.push_str("],\"receipts\":[");
    for (index, receipt) in receipts.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_hash(&mut output, receipt.key.0);
    }
    output.push_str("],\"semantic_flow_registry_fingerprint\":");
    push_hash(&mut output, semantic_flow_registry_fingerprint);
    output.push_str(",\"total_layout_work\":");
    output.push_str(&total_work.to_string());
    output.push('}');
    output
}

fn checked_add(left: i64, right: i64) -> Result<i64, StagingMathLayoutError> {
    left.checked_add(right)
        .ok_or(StagingMathLayoutError::ArithmeticOverflow)
}

fn checked_sub(left: i64, right: i64) -> Result<i64, StagingMathLayoutError> {
    left.checked_sub(right)
        .ok_or(StagingMathLayoutError::ArithmeticOverflow)
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

#[cfg(any(test, feature = "staging-fixtures"))]
pub struct StagingMathLayoutFixture {
    pub package: ValidatedStagingSemanticPackage,
    pub profile: StagingMathProfileAuthorization,
    pub limits: M4EffectiveResourceLimits,
    pub admitted: AdmittedResourceLedger,
    pub layout: StagingMathLayout,
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_math_layout_fixture() -> Result<StagingMathLayoutFixture, Box<dyn std::error::Error>>
{
    staging_math_layout_fixture_for("document-package.json")
}

#[cfg(any(test, feature = "staging-fixtures"))]
fn staging_math_layout_fixture_for(
    document_name: &str,
) -> Result<StagingMathLayoutFixture, Box<dyn std::error::Error>> {
    staging_math_layout_fixture_for_bytes(document_name, None)
}

#[cfg(any(test, feature = "staging-fixtures"))]
fn staging_math_layout_fixture_for_bytes(
    document_name: &str,
    package_bytes: Option<&[u8]>,
) -> Result<StagingMathLayoutFixture, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::PathBuf;
    use typaxis_core::{
        ConfigResourceRoot, EffectiveConfig, EffectiveDataVersions, HostAdmissionContext, HostPath,
        M4ResourceLimits, PdfStreamCompression, ResourceLimits, ValidatedResourceLimits,
        DEFAULT_ALLOWED_URI_SCHEMES,
    };
    use typaxis_resource_admission::{
        staging_declared_base_catalog, AdmittedResourceResolver, HostResourceAdmissionSession,
    };
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    let job = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../samples/machine-package/staging/production-book-1/math/job");
    let package_path = job.join(document_name);
    let package_bytes = match package_bytes {
        Some(bytes) => bytes.to_vec(),
        None => fs::read(&package_path)?,
    };
    let base_limits = ValidatedResourceLimits::new(ResourceLimits::default())?;
    let limits = M4EffectiveResourceLimits::new(base_limits.clone(), M4ResourceLimits::default())?;
    let decoded = StagingSemanticDocumentPackageDecoder::new().decode(
        &package_bytes,
        &DocumentPackageDecodePolicy::new(&base_limits),
    )?;
    let package = StagingSemanticPackageParser::new().parse(decoded, &base_limits)?;
    let profile = issue_fixture_math_profile(&package, &limits)?;
    let base = staging_declared_base_catalog(package.resources())?;
    let config = EffectiveConfig::new(
        true,
        PdfStreamCompression::None,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        EffectiveDataVersions::new("16.0.0", "typaxis-jlreq-horizontal/1.0.0")
            .expect("registered fixture data versions"),
        ResourceLimits::default(),
    )?;
    let context = HostAdmissionContext::new(
        HostPath::new(package_path)?,
        HostPath::new(job)?,
        None,
        Vec::new(),
    );
    let session = HostResourceAdmissionSession::new(&context, &config, &base)?;
    let mut resolver = AdmittedResourceResolver::new_with_declared_roots_and_m4_limits(
        &base,
        &limits,
        profile.profile_fingerprint(),
        session.roots(),
    )?;
    for declaration in &package.resources().font_faces {
        let pending = resolver.read_font(session.open_font(declaration.font_face_id)?)?;
        resolver.parse_and_bind_sfnt(pending)?;
    }
    let admitted = resolver.finish()?;
    let layout = layout_staging_math(&package, &profile, &limits, &admitted)?;
    Ok(StagingMathLayoutFixture {
        package,
        profile,
        limits,
        admitted,
        layout,
    })
}

#[cfg(any(test, feature = "staging-fixtures"))]
fn issue_fixture_math_profile(
    package: &ValidatedStagingSemanticPackage,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingMathProfileAuthorization, StagingSemanticSyntaxError> {
    let view = StagingMathProfileView::new(package, limits)?;
    let mut binding =
        String::from("{\"algorithm\":\"typaxis.math-fixture-profile/1\",\"authorization\":");
    push_hash(&mut binding, view.profile_fingerprint());
    binding.push('}');
    StagingMathProfileAuthorization::bind_profile_receipt(
        view,
        sha256(binding.as_bytes()),
        package,
        limits,
        &typaxis_syntax::StagingMathProfileSessionIdentity::fresh(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top_level_math_package() -> String {
        let bytes = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "../../../samples/machine-package/staging/production-book-1/math/job/document-package.json",
            ),
        )
        .unwrap();
        let bytes =
            typaxis_syntax::machine_profile_boundary::wire::staging_math_document_body_fixture(
                &bytes,
            )
            .unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn math_inline_display_wrap_flow_and_receipt_chain_are_closed() {
        let fixture = staging_math_layout_fixture().unwrap();
        assert_eq!(fixture.layout.receipts().len(), 2);
        assert_eq!(fixture.layout.display_flows().len(), 1);
        assert_eq!(fixture.layout.display_flows()[0].flow_id().get(), 0);
        assert_eq!(fixture.layout.placements()[0].display_flow_id(), None);
        assert_eq!(
            fixture.layout.placements()[1].display_flow_id(),
            Some(MathFlowId(0))
        );
        let inline =
            AtomicMathInlineItem::from_computation(fixture.layout.receipts()[0].computation())
                .unwrap();
        assert_eq!(
            inline.place(inline.advance(), inline.advance()).unwrap(),
            AtomicMathPlacement::FitsCurrentLine
        );
        assert_eq!(
            inline
                .place(inline.advance() - 1, inline.advance())
                .unwrap(),
            AtomicMathPlacement::MoveIntactToNextLine
        );
        assert_eq!(
            inline.place(0, inline.advance() - 1).unwrap(),
            AtomicMathPlacement::Oversize
        );
        fixture
            .layout
            .verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
            )
            .unwrap();
    }

    #[test]
    fn math_layout_accepts_document_body_owners_without_a_container() {
        let package = top_level_math_package();
        let fixture = staging_math_layout_fixture_for_bytes(
            "document-package.json",
            Some(package.as_bytes()),
        )
        .unwrap();
        assert_eq!(fixture.package.semantic_container_count(), 0);
        assert_eq!(fixture.layout.placements().len(), 2);
        assert!(fixture
            .layout
            .placements()
            .iter()
            .all(|placement| placement.parent_flow_id() == FlowId::DOCUMENT_BODY));
        assert_eq!(fixture.layout.display_flows()[0].parent_position(), 1);
    }

    #[test]
    fn math_display_moves_whole_to_the_next_page_and_aggregate_limit_is_inclusive() {
        let page_fixture = staging_math_layout_fixture_for("page-document-package.json").unwrap();
        assert_eq!(page_fixture.layout.placements()[0].page_index(), 0);
        assert_eq!(page_fixture.layout.placements()[1].page_index(), 1);
        assert_eq!(page_fixture.layout.placements()[1].fragment_ordinal(), 1);

        let keep_fixture = staging_math_layout_fixture_for("keep-document-package.json").unwrap();
        assert_eq!(keep_fixture.layout.placements().len(), 3);
        assert_eq!(keep_fixture.layout.placements()[0].page_index(), 0);
        assert_eq!(keep_fixture.layout.placements()[1].page_index(), 1);
        assert_eq!(keep_fixture.layout.placements()[2].page_index(), 1);

        let fixture = staging_math_layout_fixture().unwrap();
        let exact_work = fixture.layout.total_layout_work();
        let exact = M4EffectiveResourceLimits::new(
            fixture.limits.base().clone(),
            typaxis_core::M4ResourceLimits {
                max_math_layout_units: exact_work,
                ..typaxis_core::M4ResourceLimits::default()
            },
        )
        .unwrap();
        let exact_profile = issue_fixture_math_profile(&fixture.package, &exact).unwrap();
        let exact_layout =
            layout_staging_math(&fixture.package, &exact_profile, &exact, &fixture.admitted)
                .unwrap();
        assert_eq!(exact_layout.total_layout_work(), exact_work);
        assert_eq!(
            layout_staging_math(&fixture.package, &exact_profile, &exact, &fixture.admitted)
                .unwrap_err(),
            StagingMathLayoutError::LayoutUnitLimit
        );

        let foreign_profile = issue_fixture_math_profile(&fixture.package, &exact).unwrap();
        assert_eq!(
            exact_layout.verify(
                &fixture.package,
                &foreign_profile,
                &exact,
                &fixture.admitted,
            ),
            Err(StagingMathLayoutError::ReceiptMismatch)
        );

        let too_small = M4EffectiveResourceLimits::new(
            fixture.limits.base().clone(),
            typaxis_core::M4ResourceLimits {
                max_math_layout_units: exact_work - 1,
                ..typaxis_core::M4ResourceLimits::default()
            },
        )
        .unwrap();
        let too_small_profile = issue_fixture_math_profile(&fixture.package, &too_small).unwrap();
        assert_eq!(
            layout_staging_math(
                &fixture.package,
                &too_small_profile,
                &too_small,
                &fixture.admitted,
            )
            .unwrap_err(),
            StagingMathLayoutError::LayoutUnitLimit
        );
    }

    #[test]
    fn display_math_page_name_change_starts_a_new_page() {
        let fixture = staging_math_layout_fixture().unwrap();
        assert_eq!(fixture.layout.placements()[1].page_index(), 0);
        let bytes = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "../../../samples/machine-package/staging/production-book-1/math/job/document-package.json",
            ),
        )
        .unwrap();
        let original = String::from_utf8(bytes).unwrap();
        let named = original.replacen(
            "\"declarations\":[{\"important\":false,\"name\":\"text_align\"",
            "\"declarations\":[{\"important\":false,\"name\":\"page\",\"value\":{\"kind\":\"string\",\"value\":\"chapter\"}},{\"important\":false,\"name\":\"text_align\"",
            1,
        );
        assert_ne!(named, original);
        let fixture =
            staging_math_layout_fixture_for_bytes("document-package.json", Some(named.as_bytes()))
                .unwrap();
        assert_eq!(fixture.layout.placements()[0].page_index(), 0);
        assert_eq!(fixture.layout.placements()[1].page_index(), 1);
    }

    #[test]
    fn math_keep_only_reaches_the_immediately_following_flow_item() {
        assert!(adjacent_flow_positions(
            (FlowId::new(7), 10),
            (FlowId::new(7), 11)
        ));
        assert!(!adjacent_flow_positions(
            (FlowId::new(7), 10),
            (FlowId::new(7), 12)
        ));
        assert!(!adjacent_flow_positions(
            (FlowId::new(7), 10),
            (FlowId::new(8), 11)
        ));
    }

    #[test]
    fn math_source_alternative_vector_and_page_tamper_are_independent_failures() {
        let fixture = staging_math_layout_fixture().unwrap();
        let mut source = fixture.layout;
        source.receipts[0].source_sha256[0] ^= 1;
        assert_eq!(
            source.verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted
            ),
            Err(StagingMathLayoutError::ReceiptMismatch)
        );

        let fixture = staging_math_layout_fixture().unwrap();
        let mut alternative = fixture.layout;
        alternative.receipts[0].speech_sha256[0] ^= 1;
        assert_eq!(
            alternative.verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted
            ),
            Err(StagingMathLayoutError::ReceiptMismatch)
        );

        let fixture = staging_math_layout_fixture().unwrap();
        let mut vector = fixture.layout;
        vector.receipts[0].canonical_jcs.push(' ');
        assert_eq!(
            vector.verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted
            ),
            Err(StagingMathLayoutError::ReceiptMismatch)
        );

        let fixture = staging_math_layout_fixture().unwrap();
        let mut page = fixture.layout;
        page.placements[0].page_index = 1;
        assert_eq!(
            page.verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted
            ),
            Err(StagingMathLayoutError::ReceiptMismatch)
        );
    }
}
