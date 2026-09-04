use typaxis_core::{push_jcs_string, sha256, M4EffectiveResourceLimits, NodeId};
use typaxis_layout_contract::{FlowId, MathVectorFlowId, MathVectorFlowTerminal};
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_shaping::{
    shape_staging_equation_number, StagingEquationNumberShapeError,
    StagingEquationNumberShapeReceipt,
};
use typaxis_syntax::{
    PrecomposedVectorKind, StagingPrecomposedVectorProfileAuthorization,
    ValidatedStagingSemanticPackage,
};

use crate::{
    semantic_container::project_staging_precomposed_vector_parent_flows, PrecomposedMathVectorKind,
    PrecomposedVectorPlacementInput, StagingSemanticContainerFlowItemKind,
    StagingSemanticContainerFlowRegistry, ValidatedPrecomposedVectorBindings,
};

pub const MATH_VECTOR_FLOW_ALGORITHM: &str = "typaxis.math-vector-flow/1";
pub const MATH_VECTOR_TERMINAL_ALGORITHM: &str = "typaxis.math-vector-terminal/1";

/// One source-preorder producer-composed block-math flow. It projects to the
/// parent's existing atomic display-math item category while retaining the
/// exact `math_vector_block` wire kind and producer-composed binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathVectorFlowRecord {
    flow_id: MathVectorFlowId,
    owner: NodeId,
    parent_flow_id: FlowId,
    parent_position: u32,
    parent_item_kind: StagingSemanticContainerFlowItemKind,
    wire_kind: PrecomposedMathVectorKind,
    math_binding_fingerprint: [u8; 32],
    computed_style_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    terminal: MathVectorFlowTerminal,
    fingerprint: [u8; 32],
}

impl StagingMathVectorFlowRecord {
    pub const fn flow_id(&self) -> MathVectorFlowId {
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

    pub const fn parent_item_kind(&self) -> StagingSemanticContainerFlowItemKind {
        self.parent_item_kind
    }

    pub const fn wire_kind(&self) -> PrecomposedMathVectorKind {
        self.wire_kind
    }

    pub const fn math_binding_fingerprint(&self) -> [u8; 32] {
        self.math_binding_fingerprint
    }

    pub const fn computed_style_fingerprint(&self) -> [u8; 32] {
        self.computed_style_fingerprint
    }

    pub const fn layout_epoch_fingerprint(&self) -> [u8; 32] {
        self.layout_epoch_fingerprint
    }

    pub const fn terminal(&self) -> MathVectorFlowTerminal {
        self.terminal
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathVectorFlowRegistryReceipt {
    package_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    binding_set_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    parent_flow_registry_fingerprint: [u8; 32],
    flow_count: u32,
    equation_number_shape_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathVectorFlowRegistryReceipt {
    pub const fn algorithm(&self) -> &'static str {
        MATH_VECTOR_FLOW_ALGORITHM
    }

    pub const fn flow_count(&self) -> u32 {
        self.flow_count
    }

    pub const fn equation_number_shape_count(&self) -> u32 {
        self.equation_number_shape_count
    }

    pub const fn parent_flow_registry_fingerprint(&self) -> [u8; 32] {
        self.parent_flow_registry_fingerprint
    }

    pub const fn layout_epoch_fingerprint(&self) -> [u8; 32] {
        self.layout_epoch_fingerprint
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathVectorFlowRegistry {
    flows: Vec<StagingMathVectorFlowRecord>,
    equation_number_shapes: Vec<StagingEquationNumberShapeReceipt>,
    receipt: StagingMathVectorFlowRegistryReceipt,
}

impl StagingMathVectorFlowRegistry {
    pub fn flows(&self) -> &[StagingMathVectorFlowRecord] {
        &self.flows
    }

    pub fn flow(&self, id: MathVectorFlowId) -> Option<&StagingMathVectorFlowRecord> {
        self.flows
            .get(usize::try_from(id.get()).ok()?)
            .filter(|flow| flow.flow_id == id)
    }

    pub fn equation_number_shapes(&self) -> &[StagingEquationNumberShapeReceipt] {
        &self.equation_number_shapes
    }

    pub fn equation_number_shape(
        &self,
        owner: NodeId,
    ) -> Option<&StagingEquationNumberShapeReceipt> {
        self.equation_number_shapes
            .binary_search_by_key(&owner, StagingEquationNumberShapeReceipt::owner)
            .ok()
            .map(|index| &self.equation_number_shapes[index])
    }

    pub const fn receipt(&self) -> &StagingMathVectorFlowRegistryReceipt {
        &self.receipt
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingPrecomposedVectorProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        bindings: &ValidatedPrecomposedVectorBindings,
    ) -> Result<(), StagingMathVectorFlowError> {
        let expected = build_staging_math_vector_flows(
            package,
            profile,
            limits,
            admitted,
            bindings,
            CandidateSchedule::SourcePreorder,
        )?;
        if self != &expected || !self.integrity_matches(limits) {
            return Err(StagingMathVectorFlowError::ReceiptMismatch);
        }
        Ok(())
    }

    /// Starts the selection-time exact-terminal ledger.
    ///
    /// A native `MathFlowId` cannot be supplied here because the two ID types
    /// are nominally different:
    ///
    /// ```compile_fail
    /// use typaxis_layout::{MathFlowId, MathVectorFlowId};
    /// fn consume(_: MathVectorFlowId) {}
    /// fn wrong(id: MathFlowId) { consume(id); }
    /// ```
    pub fn terminal_ledger(
        &self,
    ) -> Result<StagingMathVectorTerminalLedger, StagingMathVectorTerminalError> {
        if !self.integrity_matches_without_limit() {
            return Err(StagingMathVectorTerminalError::RegistryMismatch);
        }
        let mut flows = Vec::new();
        flows
            .try_reserve_exact(self.flows.len())
            .map_err(|_| StagingMathVectorTerminalError::AllocationFailure)?;
        flows.extend(self.flows.iter().cloned());
        let mut receipts = Vec::new();
        receipts
            .try_reserve_exact(self.flows.len())
            .map_err(|_| StagingMathVectorTerminalError::AllocationFailure)?;
        receipts.resize_with(self.flows.len(), || None);
        Ok(StagingMathVectorTerminalLedger {
            registry_fingerprint: self.receipt.fingerprint,
            flows,
            receipts,
        })
    }

    fn integrity_matches(&self, limits: &M4EffectiveResourceLimits) -> bool {
        u64::try_from(self.flows.len())
            .is_ok_and(|count| count <= limits.base().get().max_ast_nodes)
            && self.integrity_matches_without_limit()
    }

    fn integrity_matches_without_limit(&self) -> bool {
        let canonical = encode_registry(
            self.receipt.package_sha256,
            self.receipt.semantic_fingerprint,
            self.receipt.profile_fingerprint,
            self.receipt.limits_fingerprint,
            self.receipt.admitted_fingerprint,
            self.receipt.binding_set_fingerprint,
            self.receipt.layout_epoch_fingerprint,
            self.receipt.parent_flow_registry_fingerprint,
            &self.flows,
            &self.equation_number_shapes,
        );
        usize::try_from(self.receipt.flow_count) == Ok(self.flows.len())
            && usize::try_from(self.receipt.equation_number_shape_count)
                == Ok(self.equation_number_shapes.len())
            && self.flows.iter().enumerate().all(|(index, flow)| {
                usize::try_from(flow.flow_id.get()) == Ok(index)
                    && flow.parent_item_kind == StagingSemanticContainerFlowItemKind::DisplayMath
                    && flow.wire_kind == PrecomposedMathVectorKind::Block
                    && flow.layout_epoch_fingerprint == self.receipt.layout_epoch_fingerprint
                    && flow.terminal == MathVectorFlowTerminal::ONE
                    && flow.fingerprint == sha256(encode_flow(flow).as_bytes())
            })
            && self
                .flows
                .windows(2)
                .all(|pair| pair[0].owner < pair[1].owner)
            && self
                .equation_number_shapes
                .windows(2)
                .all(|pair| pair[0].owner() < pair[1].owner())
            && self.equation_number_shapes.iter().all(|shape| {
                shape.integrity_matches()
                    && shape.layout_epoch_fingerprint() == self.receipt.layout_epoch_fingerprint
                    && self
                        .flows
                        .binary_search_by_key(&shape.owner(), |flow| flow.owner)
                        .is_ok_and(|index| {
                            shape.computed_style_fingerprint()
                                == self.flows[index].computed_style_fingerprint
                        })
            })
            && self.receipt.canonical_jcs == canonical
            && self.receipt.fingerprint == sha256(canonical.as_bytes())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMathVectorFlowError {
    BindingMismatch,
    ParentFlowMismatch(NodeId),
    LanguageRegistryMismatch,
    LanguageMismatch(NodeId),
    EquationNumberShape(NodeId, StagingEquationNumberShapeError),
    FlowLimit,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingMathVectorFlowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BindingMismatch => formatter.write_str("I9190: math-vector binding set mismatch"),
            Self::ParentFlowMismatch(owner) => write!(
                formatter,
                "I9190: math-vector parent flow mismatch at node {}",
                owner.get()
            ),
            Self::LanguageRegistryMismatch => {
                formatter.write_str("I9190: math-vector owner language registry mismatch")
            }
            Self::LanguageMismatch(owner) => write!(
                formatter,
                "I9190: math-vector owner language mismatch at node {}",
                owner.get()
            ),
            Self::EquationNumberShape(owner, error) => match error {
                StagingEquationNumberShapeError::ReceiptMismatch => write!(
                    formatter,
                    "I9190: equation-number shape binding mismatch at node {}",
                    owner.get()
                ),
                StagingEquationNumberShapeError::AllocationFailure
                | StagingEquationNumberShapeError::Backend(
                    typaxis_shaping::LinkedShaperError::AllocationFailure,
                ) => write!(
                    formatter,
                    "L5111: equation-number shaping allocation failed at node {}",
                    owner.get()
                ),
                StagingEquationNumberShapeError::MissingSelectedFont
                | StagingEquationNumberShapeError::MissingDeclaredFontCoverage
                | StagingEquationNumberShapeError::InvalidFontOrFace
                | StagingEquationNumberShapeError::Backend(_) => write!(
                    formatter,
                    "R7100: equation-number shaping failed at node {}",
                    owner.get()
                ),
                StagingEquationNumberShapeError::MissingComputedTextStyle
                | StagingEquationNumberShapeError::RequiresSecondLine
                | StagingEquationNumberShapeError::NonPositiveShape
                | StagingEquationNumberShapeError::ContextLimit
                | StagingEquationNumberShapeError::ArithmeticOverflow => write!(
                    formatter,
                    "L5100: equation number at node {} is not one positive nonwrapping line",
                    owner.get()
                ),
            },
            Self::FlowLimit => formatter.write_str("P1120: math-vector flow limit exceeded"),
            Self::AllocationFailure => {
                formatter.write_str("L5111: math-vector flow allocation failed")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: math-vector flow receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingMathVectorFlowError {}

pub fn prepare_staging_math_vector_flows(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
) -> Result<StagingMathVectorFlowRegistry, StagingMathVectorFlowError> {
    let registry = build_staging_math_vector_flows(
        package,
        profile,
        limits,
        admitted,
        bindings,
        CandidateSchedule::SourcePreorder,
    )?;
    if !registry.integrity_matches(limits) {
        return Err(StagingMathVectorFlowError::ReceiptMismatch);
    }
    Ok(registry)
}

#[derive(Clone, Copy)]
enum CandidateSchedule {
    SourcePreorder,
    #[cfg(test)]
    ReverseWorkerCompletion,
}

#[derive(Clone)]
struct MathVectorFlowCandidate {
    flow_id: MathVectorFlowId,
    owner: NodeId,
    parent_flow_id: FlowId,
    parent_position: u32,
    math_binding_fingerprint: [u8; 32],
    computed_style_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    equation_number_shape: Option<StagingEquationNumberShapeReceipt>,
}

#[derive(Clone, Copy)]
struct MathVectorFlowRegistration {
    flow_id: MathVectorFlowId,
    metrics_index: usize,
    owner: NodeId,
}

fn build_staging_math_vector_flows(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
    schedule: CandidateSchedule,
) -> Result<StagingMathVectorFlowRegistry, StagingMathVectorFlowError> {
    bindings
        .verify(package, profile, limits, admitted)
        .map_err(|_| StagingMathVectorFlowError::BindingMismatch)?;
    profile
        .authorizes(package, limits)
        .map_err(|_| StagingMathVectorFlowError::BindingMismatch)?;
    let parent_registry = project_staging_precomposed_vector_parent_flows(package, profile, limits)
        .map_err(|_| StagingMathVectorFlowError::ReceiptMismatch)?;
    let effective_languages = package
        .precomposed_vector_effective_languages()
        .map_err(|_| StagingMathVectorFlowError::LanguageRegistryMismatch)?;

    let block_count = package
        .precomposed_vector_metrics()
        .iter()
        .filter(|metrics| metrics.kind() == PrecomposedVectorKind::MathVectorBlock)
        .count();
    if u64::try_from(block_count).map_or(true, |count| count > limits.base().get().max_ast_nodes) {
        return Err(StagingMathVectorFlowError::FlowLimit);
    }
    let mut registrations = Vec::new();
    registrations
        .try_reserve_exact(block_count)
        .map_err(|_| StagingMathVectorFlowError::AllocationFailure)?;
    for (metrics_index, metrics) in package.precomposed_vector_metrics().iter().enumerate() {
        if metrics.kind() != PrecomposedVectorKind::MathVectorBlock {
            continue;
        }
        // Finish the complete source-preorder registration pass before any
        // shaping work can be dispatched or complete in a different order.
        let flow_id = MathVectorFlowId::new(
            u32::try_from(registrations.len())
                .map_err(|_| StagingMathVectorFlowError::FlowLimit)?,
        );
        registrations.push(MathVectorFlowRegistration {
            flow_id,
            metrics_index,
            owner: metrics.node_id(),
        });
    }
    if registrations.len() != block_count
        || registrations
            .iter()
            .enumerate()
            .any(|(index, registration)| {
                usize::try_from(registration.flow_id.get()) != Ok(index)
                    || index > 0 && registrations[index - 1].owner >= registration.owner
            })
    {
        return Err(StagingMathVectorFlowError::ReceiptMismatch);
    }

    #[cfg(test)]
    if matches!(schedule, CandidateSchedule::ReverseWorkerCompletion) {
        registrations.reverse();
    }
    #[cfg(not(test))]
    let _ = schedule;

    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(block_count)
        .map_err(|_| StagingMathVectorFlowError::AllocationFailure)?;
    for registration in registrations {
        let metrics = package
            .precomposed_vector_metrics()
            .get(registration.metrics_index)
            .filter(|metrics| {
                metrics.node_id() == registration.owner
                    && metrics.kind() == PrecomposedVectorKind::MathVectorBlock
            })
            .ok_or(StagingMathVectorFlowError::ReceiptMismatch)?;
        let flow_id = registration.flow_id;
        let owner = registration.owner;
        let common = bindings
            .receipt(owner)
            .ok_or(StagingMathVectorFlowError::BindingMismatch)?;
        let math = bindings
            .math_receipt(owner)
            .ok_or(StagingMathVectorFlowError::BindingMismatch)?;
        let style = package
            .precomposed_vector_style(owner)
            .ok_or(StagingMathVectorFlowError::BindingMismatch)?;
        package
            .verify_precomposed_vector_style(style)
            .map_err(|_| StagingMathVectorFlowError::BindingMismatch)?;
        let PrecomposedVectorPlacementInput::MathVectorBlock(placement) = common.placement() else {
            return Err(StagingMathVectorFlowError::BindingMismatch);
        };
        if common.kind() != PrecomposedVectorKind::MathVectorBlock
            || common.epoch_fingerprint() != bindings.epoch().fingerprint()
            || math.kind() != PrecomposedMathVectorKind::Block
            || math.common_fingerprint() != common.fingerprint()
            || placement.style().fingerprint() != style.fingerprint()
        {
            return Err(StagingMathVectorFlowError::BindingMismatch);
        }
        let (parent_flow_id, parent_position) = math_vector_parent(owner, &parent_registry)?;
        let language = effective_languages
            .binary_search_by_key(&owner, |receipt| receipt.owner())
            .ok()
            .map(|index| &effective_languages[index])
            .ok_or(StagingMathVectorFlowError::LanguageMismatch(owner))?;
        if language.kind() != PrecomposedVectorKind::MathVectorBlock {
            return Err(StagingMathVectorFlowError::LanguageMismatch(owner));
        }
        let equation_number_shape = shape_staging_equation_number(
            package,
            metrics,
            admitted,
            bindings.epoch().fingerprint(),
            language,
        )
        .map_err(|error| StagingMathVectorFlowError::EquationNumberShape(owner, error))?;
        candidates.push(MathVectorFlowCandidate {
            flow_id,
            owner,
            parent_flow_id,
            parent_position,
            math_binding_fingerprint: math.fingerprint(),
            computed_style_fingerprint: style.fingerprint(),
            layout_epoch_fingerprint: bindings.epoch().fingerprint(),
            equation_number_shape,
        });
    }

    candidates.sort_unstable_by_key(|candidate| candidate.flow_id);
    if candidates.iter().enumerate().any(|(index, candidate)| {
        usize::try_from(candidate.flow_id.get()) != Ok(index)
            || index > 0 && candidates[index - 1].owner >= candidate.owner
    }) {
        return Err(StagingMathVectorFlowError::ReceiptMismatch);
    }

    let mut flows = Vec::new();
    let mut equation_number_shapes = Vec::new();
    flows
        .try_reserve_exact(candidates.len())
        .map_err(|_| StagingMathVectorFlowError::AllocationFailure)?;
    equation_number_shapes
        .try_reserve_exact(
            candidates
                .iter()
                .filter(|candidate| candidate.equation_number_shape.is_some())
                .count(),
        )
        .map_err(|_| StagingMathVectorFlowError::AllocationFailure)?;
    for candidate in candidates {
        let mut flow = StagingMathVectorFlowRecord {
            flow_id: candidate.flow_id,
            owner: candidate.owner,
            parent_flow_id: candidate.parent_flow_id,
            parent_position: candidate.parent_position,
            parent_item_kind: StagingSemanticContainerFlowItemKind::DisplayMath,
            wire_kind: PrecomposedMathVectorKind::Block,
            math_binding_fingerprint: candidate.math_binding_fingerprint,
            computed_style_fingerprint: candidate.computed_style_fingerprint,
            layout_epoch_fingerprint: candidate.layout_epoch_fingerprint,
            terminal: MathVectorFlowTerminal::ONE,
            fingerprint: [0; 32],
        };
        flow.fingerprint = sha256(encode_flow(&flow).as_bytes());
        flows.push(flow);
        if let Some(shape) = candidate.equation_number_shape {
            equation_number_shapes.push(shape);
        }
    }

    let package_sha256 = package.canonical_jcs_sha256();
    let semantic_fingerprint = package.semantic_fingerprint();
    let profile_fingerprint = profile.profile_fingerprint();
    let limits_fingerprint = limits.fingerprint();
    let admitted_fingerprint = admitted.fingerprint().bytes();
    let binding_set_fingerprint = bindings.fingerprint();
    let layout_epoch_fingerprint = bindings.epoch().fingerprint();
    let parent_flow_registry_fingerprint = parent_registry.receipt().fingerprint();
    let canonical_jcs = encode_registry(
        package_sha256,
        semantic_fingerprint,
        profile_fingerprint,
        limits_fingerprint,
        admitted_fingerprint,
        binding_set_fingerprint,
        layout_epoch_fingerprint,
        parent_flow_registry_fingerprint,
        &flows,
        &equation_number_shapes,
    );
    Ok(StagingMathVectorFlowRegistry {
        receipt: StagingMathVectorFlowRegistryReceipt {
            package_sha256,
            semantic_fingerprint,
            profile_fingerprint,
            limits_fingerprint,
            admitted_fingerprint,
            binding_set_fingerprint,
            layout_epoch_fingerprint,
            parent_flow_registry_fingerprint,
            flow_count: u32::try_from(flows.len())
                .map_err(|_| StagingMathVectorFlowError::FlowLimit)?,
            equation_number_shape_count: u32::try_from(equation_number_shapes.len())
                .map_err(|_| StagingMathVectorFlowError::FlowLimit)?,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        flows,
        equation_number_shapes,
    })
}

fn math_vector_parent(
    owner: NodeId,
    registry: &StagingSemanticContainerFlowRegistry,
) -> Result<(FlowId, u32), StagingMathVectorFlowError> {
    let mut found = None;
    for flow in registry.flows() {
        for item in flow.items().iter().filter(|item| item.owner() == owner) {
            if item.kind() != StagingSemanticContainerFlowItemKind::DisplayMath
                || !item.child_flow_ids().is_empty()
                || found.replace((flow.flow_id(), item.position())).is_some()
            {
                return Err(StagingMathVectorFlowError::ParentFlowMismatch(owner));
            }
        }
    }
    found.ok_or(StagingMathVectorFlowError::ParentFlowMismatch(owner))
}

fn encode_flow(flow: &StagingMathVectorFlowRecord) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MATH_VECTOR_FLOW_ALGORITHM);
    output.push_str(",\"computed_style_fingerprint\":");
    push_hash(&mut output, flow.computed_style_fingerprint);
    output.push_str(",\"flow_id\":");
    output.push_str(&flow.flow_id.get().to_string());
    output.push_str(",\"layout_epoch_fingerprint\":");
    push_hash(&mut output, flow.layout_epoch_fingerprint);
    output.push_str(",\"math_binding_fingerprint\":");
    push_hash(&mut output, flow.math_binding_fingerprint);
    output.push_str(",\"owner\":");
    output.push_str(&flow.owner.get().to_string());
    output.push_str(",\"parent_flow_id\":");
    output.push_str(&flow.parent_flow_id.get().to_string());
    output.push_str(",\"parent_item_kind\":");
    push_jcs_string(&mut output, flow.parent_item_kind.as_str());
    output.push_str(",\"parent_position\":");
    output.push_str(&flow.parent_position.to_string());
    output.push_str(",\"terminal\":");
    output.push_str(&flow.terminal.get().to_string());
    output.push_str(",\"wire_kind\":");
    push_jcs_string(&mut output, flow.wire_kind.as_str());
    output.push('}');
    output
}

#[allow(clippy::too_many_arguments)]
fn encode_registry(
    package_sha256: [u8; 32],
    semantic_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    binding_set_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    parent_flow_registry_fingerprint: [u8; 32],
    flows: &[StagingMathVectorFlowRecord],
    equation_number_shapes: &[StagingEquationNumberShapeReceipt],
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted_fingerprint);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, MATH_VECTOR_FLOW_ALGORITHM);
    output.push_str(",\"binding_set_fingerprint\":");
    push_hash(&mut output, binding_set_fingerprint);
    output.push_str(",\"equation_number_shapes\":[");
    for (index, shape) in equation_number_shapes.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"fingerprint\":");
        push_hash(&mut output, shape.fingerprint());
        output.push_str(",\"owner\":");
        output.push_str(&shape.owner().get().to_string());
        output.push('}');
    }
    output.push_str("],\"flows\":[");
    for (index, flow) in flows.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&encode_flow(flow));
    }
    output.push_str("],\"layout_epoch_fingerprint\":");
    push_hash(&mut output, layout_epoch_fingerprint);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, package_sha256);
    output.push_str(",\"parent_flow_registry_fingerprint\":");
    push_hash(&mut output, parent_flow_registry_fingerprint);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, profile_fingerprint);
    output.push_str(",\"semantic_fingerprint\":");
    push_hash(&mut output, semantic_fingerprint);
    output.push('}');
    output
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathVectorTerminalReceipt {
    flow_id: MathVectorFlowId,
    owner: NodeId,
    terminal: MathVectorFlowTerminal,
    flow_fingerprint: [u8; 32],
    registry_fingerprint: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathVectorTerminalReceipt {
    pub const fn flow_id(&self) -> MathVectorFlowId {
        self.flow_id
    }

    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn terminal(&self) -> MathVectorFlowTerminal {
        self.terminal
    }

    pub const fn flow_fingerprint(&self) -> [u8; 32] {
        self.flow_fingerprint
    }

    pub const fn registry_fingerprint(&self) -> [u8; 32] {
        self.registry_fingerprint
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Debug)]
pub struct StagingMathVectorTerminalLedger {
    registry_fingerprint: [u8; 32],
    flows: Vec<StagingMathVectorFlowRecord>,
    receipts: Vec<Option<StagingMathVectorTerminalReceipt>>,
}

impl StagingMathVectorTerminalLedger {
    /// Revalidates a pending flow without advancing it. Page/column movement
    /// therefore cannot consume the atomic terminal.
    pub fn defer_page_move(
        &self,
        flow_id: MathVectorFlowId,
        owner: NodeId,
    ) -> Result<(), StagingMathVectorTerminalError> {
        let index = self.pending_index(flow_id, owner)?;
        if self.receipts[index].is_some() {
            return Err(StagingMathVectorTerminalError::AlreadyConsumed(flow_id));
        }
        Ok(())
    }

    /// Consumes terminal `1` only after the caller has selected the complete
    /// atomic block placement. V11 supplies that successful-placement event.
    pub fn consume_selected(
        &mut self,
        flow_id: MathVectorFlowId,
        owner: NodeId,
    ) -> Result<(), StagingMathVectorTerminalError> {
        let index = self.pending_index(flow_id, owner)?;
        if self.receipts[index].is_some() {
            return Err(StagingMathVectorTerminalError::AlreadyConsumed(flow_id));
        }
        let flow = &self.flows[index];
        if flow.terminal != MathVectorFlowTerminal::ONE {
            return Err(StagingMathVectorTerminalError::RegistryMismatch);
        }
        let mut receipt = StagingMathVectorTerminalReceipt {
            flow_id,
            owner,
            terminal: MathVectorFlowTerminal::ONE,
            flow_fingerprint: flow.fingerprint,
            registry_fingerprint: self.registry_fingerprint,
            canonical_jcs: String::new(),
            fingerprint: [0; 32],
        };
        receipt.canonical_jcs = encode_terminal_receipt(&receipt);
        receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
        self.receipts[index] = Some(receipt);
        Ok(())
    }

    pub fn finish(
        self,
    ) -> Result<StagingMathVectorTerminalReceiptSet, StagingMathVectorTerminalError> {
        let Self {
            registry_fingerprint,
            flows,
            receipts: pending,
        } = self;
        let mut receipts = Vec::new();
        receipts
            .try_reserve_exact(pending.len())
            .map_err(|_| StagingMathVectorTerminalError::AllocationFailure)?;
        for (index, receipt) in pending.into_iter().enumerate() {
            let flow_id = MathVectorFlowId::new(
                u32::try_from(index)
                    .map_err(|_| StagingMathVectorTerminalError::AllocationFailure)?,
            );
            let receipt =
                receipt.ok_or(StagingMathVectorTerminalError::MissingConsumption(flow_id))?;
            receipts.push(receipt);
        }
        let canonical_jcs = encode_terminal_set(registry_fingerprint, &receipts);
        let result = StagingMathVectorTerminalReceiptSet {
            registry_fingerprint,
            receipts,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        };
        if !result.integrity_matches(registry_fingerprint, &flows) {
            return Err(StagingMathVectorTerminalError::RegistryMismatch);
        }
        Ok(result)
    }

    fn pending_index(
        &self,
        flow_id: MathVectorFlowId,
        owner: NodeId,
    ) -> Result<usize, StagingMathVectorTerminalError> {
        let index = usize::try_from(flow_id.get())
            .map_err(|_| StagingMathVectorTerminalError::UnknownFlow(flow_id))?;
        let flow = self
            .flows
            .get(index)
            .filter(|flow| flow.flow_id == flow_id)
            .ok_or(StagingMathVectorTerminalError::UnknownFlow(flow_id))?;
        if flow.owner != owner {
            return Err(StagingMathVectorTerminalError::OwnerMismatch(flow_id));
        }
        Ok(index)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMathVectorTerminalReceiptSet {
    registry_fingerprint: [u8; 32],
    receipts: Vec<StagingMathVectorTerminalReceipt>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingMathVectorTerminalReceiptSet {
    pub const fn registry_fingerprint(&self) -> [u8; 32] {
        self.registry_fingerprint
    }

    pub fn receipts(&self) -> &[StagingMathVectorTerminalReceipt] {
        &self.receipts
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        registry: &StagingMathVectorFlowRegistry,
    ) -> Result<(), StagingMathVectorTerminalError> {
        if !registry.integrity_matches_without_limit()
            || !self.integrity_matches(registry.receipt.fingerprint, &registry.flows)
        {
            return Err(StagingMathVectorTerminalError::RegistryMismatch);
        }
        Ok(())
    }

    fn integrity_matches(
        &self,
        registry_fingerprint: [u8; 32],
        flows: &[StagingMathVectorFlowRecord],
    ) -> bool {
        let canonical = encode_terminal_set(registry_fingerprint, &self.receipts);
        self.registry_fingerprint == registry_fingerprint
            && self.receipts.len() == flows.len()
            && self
                .receipts
                .iter()
                .zip(flows)
                .enumerate()
                .all(|(index, (receipt, flow))| {
                    usize::try_from(receipt.flow_id.get()) == Ok(index)
                        && receipt.flow_id == flow.flow_id
                        && receipt.owner == flow.owner
                        && receipt.terminal == MathVectorFlowTerminal::ONE
                        && receipt.flow_fingerprint == flow.fingerprint
                        && receipt.registry_fingerprint == registry_fingerprint
                        && receipt.canonical_jcs == encode_terminal_receipt(receipt)
                        && receipt.fingerprint == sha256(receipt.canonical_jcs.as_bytes())
                })
            && self.canonical_jcs == canonical
            && self.fingerprint == sha256(canonical.as_bytes())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMathVectorTerminalError {
    RegistryMismatch,
    UnknownFlow(MathVectorFlowId),
    OwnerMismatch(MathVectorFlowId),
    AlreadyConsumed(MathVectorFlowId),
    MissingConsumption(MathVectorFlowId),
    AllocationFailure,
}

impl std::fmt::Display for StagingMathVectorTerminalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RegistryMismatch => {
                formatter.write_str("I9190: math-vector terminal registry mismatch")
            }
            Self::UnknownFlow(flow) => {
                write!(formatter, "I9190: unknown math-vector flow {}", flow.get())
            }
            Self::OwnerMismatch(flow) => write!(
                formatter,
                "I9190: math-vector flow {} owner mismatch",
                flow.get()
            ),
            Self::AlreadyConsumed(flow) => write!(
                formatter,
                "I9190: math-vector flow {} terminal already consumed",
                flow.get()
            ),
            Self::MissingConsumption(flow) => write!(
                formatter,
                "I9190: math-vector flow {} terminal was not consumed",
                flow.get()
            ),
            Self::AllocationFailure => {
                formatter.write_str("L5111: math-vector terminal allocation failed")
            }
        }
    }
}

impl std::error::Error for StagingMathVectorTerminalError {}

fn encode_terminal_receipt(value: &StagingMathVectorTerminalReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MATH_VECTOR_TERMINAL_ALGORITHM);
    output.push_str(",\"flow_fingerprint\":");
    push_hash(&mut output, value.flow_fingerprint);
    output.push_str(",\"flow_id\":");
    output.push_str(&value.flow_id.get().to_string());
    output.push_str(",\"owner\":");
    output.push_str(&value.owner.get().to_string());
    output.push_str(",\"registry_fingerprint\":");
    push_hash(&mut output, value.registry_fingerprint);
    output.push_str(",\"terminal\":");
    output.push_str(&value.terminal.get().to_string());
    output.push('}');
    output
}

fn encode_terminal_set(
    registry_fingerprint: [u8; 32],
    receipts: &[StagingMathVectorTerminalReceipt],
) -> String {
    let mut output =
        String::from("{\"algorithm\":\"typaxis.math-vector-terminal-set/1\",\"receipts\":[");
    for (index, receipt) in receipts.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&receipt.canonical_jcs);
    }
    output.push_str("],\"registry_fingerprint\":");
    push_hash(&mut output, registry_fingerprint);
    output.push('}');
    output
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
    use crate::safe_vector::{
        staging_precomposed_vector_binding_fixture,
        staging_precomposed_vector_binding_fixture_with_equation_font,
        staging_precomposed_vector_binding_fixture_with_mixed_native_math,
    };
    use crate::{staging_math_layout_fixture, MathFlowId};

    fn reseal_registry(value: &mut StagingMathVectorFlowRegistry) {
        for flow in &mut value.flows {
            flow.fingerprint = sha256(encode_flow(flow).as_bytes());
        }
        value.receipt.flow_count = u32::try_from(value.flows.len()).unwrap();
        value.receipt.equation_number_shape_count =
            u32::try_from(value.equation_number_shapes.len()).unwrap();
        value.receipt.canonical_jcs = encode_registry(
            value.receipt.package_sha256,
            value.receipt.semantic_fingerprint,
            value.receipt.profile_fingerprint,
            value.receipt.limits_fingerprint,
            value.receipt.admitted_fingerprint,
            value.receipt.binding_set_fingerprint,
            value.receipt.layout_epoch_fingerprint,
            value.receipt.parent_flow_registry_fingerprint,
            &value.flows,
            &value.equation_number_shapes,
        );
        value.receipt.fingerprint = sha256(value.receipt.canonical_jcs.as_bytes());
    }

    #[test]
    fn math_vector_flow_is_dense_deterministic_and_tamper_closed() {
        let fixture = staging_precomposed_vector_binding_fixture_with_equation_font().unwrap();
        let registry = prepare_staging_math_vector_flows(
            &fixture.package,
            &fixture.profile,
            &fixture.limits,
            &fixture.admitted,
            &fixture.bindings,
        )
        .unwrap();
        registry
            .verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
                &fixture.bindings,
            )
            .unwrap();
        assert_eq!(registry.flows().len(), 1);
        let flow = &registry.flows()[0];
        assert_eq!(flow.flow_id(), MathVectorFlowId::new(0));
        assert_eq!(flow.owner(), NodeId::new(6));
        assert_eq!(flow.parent_flow_id(), FlowId::new(1));
        assert_eq!(flow.parent_position(), 2);
        assert_eq!(
            flow.parent_item_kind(),
            StagingSemanticContainerFlowItemKind::DisplayMath
        );
        assert_eq!(flow.wire_kind(), PrecomposedMathVectorKind::Block);
        assert_eq!(flow.terminal(), MathVectorFlowTerminal::ONE);
        assert_eq!(
            flow.math_binding_fingerprint(),
            fixture
                .bindings
                .math_receipt(NodeId::new(6))
                .unwrap()
                .fingerprint()
        );
        assert_eq!(
            flow.computed_style_fingerprint(),
            fixture
                .package
                .precomposed_vector_style(NodeId::new(6))
                .unwrap()
                .fingerprint()
        );
        assert_eq!(
            flow.layout_epoch_fingerprint(),
            fixture.bindings.epoch().fingerprint()
        );
        assert!(registry
            .receipt()
            .canonical_jcs()
            .contains("\"wire_kind\":\"math_vector_block\""));
        assert!(!registry
            .receipt()
            .canonical_jcs()
            .contains("\"parent_item_kind\":\"math_vector_block\""));

        let permuted = build_staging_math_vector_flows(
            &fixture.package,
            &fixture.profile,
            &fixture.limits,
            &fixture.admitted,
            &fixture.bindings,
            CandidateSchedule::ReverseWorkerCompletion,
        )
        .unwrap();
        assert_eq!(registry, permuted);

        for mutate in [
            0u8, // missing
            1,   // duplicate/non-dense
            2,   // owner
            3,   // parent flow
            4,   // parent position
            5,   // epoch
            6,   // terminal
            7,   // missing equation shape
            8,   // duplicate equation shape
        ] {
            let mut tampered = registry.clone();
            match mutate {
                0 => {
                    tampered.flows.clear();
                    tampered.equation_number_shapes.clear();
                }
                1 => tampered.flows.push(tampered.flows[0].clone()),
                2 => tampered.flows[0].owner = NodeId::new(5),
                3 => tampered.flows[0].parent_flow_id = FlowId::DOCUMENT_BODY,
                4 => tampered.flows[0].parent_position = 1,
                5 => tampered.flows[0].layout_epoch_fingerprint[0] ^= 1,
                6 => tampered.flows[0].terminal = MathVectorFlowTerminal::new(2),
                7 => tampered.equation_number_shapes.clear(),
                8 => {
                    let duplicate = tampered.equation_number_shapes[0].clone();
                    tampered.equation_number_shapes.push(duplicate);
                }
                _ => unreachable!(),
            }
            reseal_registry(&mut tampered);
            let error = tampered
                .verify(
                    &fixture.package,
                    &fixture.profile,
                    &fixture.limits,
                    &fixture.admitted,
                    &fixture.bindings,
                )
                .unwrap_err();
            assert_eq!(error, StagingMathVectorFlowError::ReceiptMismatch);
            assert!(error.to_string().starts_with("I9190:"));
        }
    }

    #[test]
    fn equation_number_shape_uses_exact_source_and_owner_language() {
        let fixture = staging_precomposed_vector_binding_fixture_with_equation_font().unwrap();
        let registry = prepare_staging_math_vector_flows(
            &fixture.package,
            &fixture.profile,
            &fixture.limits,
            &fixture.admitted,
            &fixture.bindings,
        )
        .unwrap();
        let shape = registry.equation_number_shape(NodeId::new(6)).unwrap();
        let metrics = fixture
            .package
            .precomposed_vector_metrics_for(NodeId::new(6))
            .unwrap();
        let number = metrics.equation_number().unwrap();
        let style = fixture
            .package
            .precomposed_vector_style(NodeId::new(6))
            .unwrap();
        let font = fixture.admitted.font(shape.font_face_id()).unwrap();
        assert_eq!(shape.algorithm(), "typaxis.equation-number-shape/1");
        assert_eq!(shape.node_id(), NodeId::new(7));
        assert_eq!(shape.source_span(), number.span());
        assert_eq!(shape.text_span(), number.text().text_span());
        assert_eq!(
            shape.text_buffer_sha256(),
            number.text().text_buffer_sha256()
        );
        assert_eq!(shape.exact_text(), "(1)");
        assert_eq!(shape.exact_text_sha256(), sha256(b"(1)"));
        assert_eq!(shape.computed_style_fingerprint(), style.fingerprint());
        assert_eq!(
            shape.layout_epoch_fingerprint(),
            fixture.bindings.epoch().fingerprint()
        );
        assert_eq!(shape.owner_language(), "ja");
        assert_eq!(
            shape.owner_language_fingerprint(),
            fixture
                .package
                .precomposed_vector_effective_language(NodeId::new(6))
                .unwrap()
                .fingerprint()
        );
        assert_eq!(shape.font_family(), "Math");
        assert_eq!(shape.font_sha256(), font.content_hash());
        assert_eq!(shape.face_index(), font.face_index());
        assert_eq!(shape.font_size().get().raw(), 786_432);
        assert_eq!(shape.line_height().get().raw(), 917_504);
        assert_ne!(shape.glyph_receipt_fingerprint(), [0; 32]);
        assert!(shape.width().get().raw() > 0);
        assert!(shape.height().get().raw() > 0);
        assert_eq!(shape.shaper_backend(), "typaxis-reference-shaper");
        assert_eq!(shape.unicode_version(), "16.0.0");
        assert!(!shape.runs().is_empty());
        assert!(shape.runs().iter().all(|run| !run.glyphs().is_empty()));
        assert!(shape.canonical_jcs().contains("\"line_count\":1"));
        assert!(shape.canonical_jcs().contains("\"nonwrapping\":true"));
        for forbidden in [
            "image_id",
            "source_tex",
            "actual_text",
            "alternative",
            "vector_content_key",
        ] {
            assert!(!shape.canonical_jcs().contains(forbidden));
        }

        let missing_number_style = staging_precomposed_vector_binding_fixture().unwrap();
        assert_eq!(
            prepare_staging_math_vector_flows(
                &missing_number_style.package,
                &missing_number_style.profile,
                &missing_number_style.limits,
                &missing_number_style.admitted,
                &missing_number_style.bindings,
            ),
            Err(StagingMathVectorFlowError::EquationNumberShape(
                NodeId::new(6),
                StagingEquationNumberShapeError::MissingComputedTextStyle,
            ))
        );
    }

    #[test]
    fn native_and_vector_math_flow_isolation_keeps_terminals_exactly_once() {
        let fixture = staging_precomposed_vector_binding_fixture_with_mixed_native_math().unwrap();
        let typaxis_document::StagingM4Block::SemanticContainer { blocks, .. } =
            &fixture.package.document().blocks[0]
        else {
            panic!("mixed fixture root must remain a semantic container");
        };
        assert_eq!(
            blocks
                .iter()
                .map(|block| {
                    (
                        block.node_id().get(),
                        block.span().start_byte().get(),
                        block.span().end_byte().get(),
                    )
                })
                .collect::<Vec<_>>(),
            [(2, 0, 3), (3, 3, 6), (4, 6, 7), (5, 7, 13)]
        );
        let native_preorder = fixture
            .package
            .math_nodes()
            .iter()
            .filter(|node| node.domain().kind == typaxis_document::StagingM4MathKind::Display)
            .enumerate()
            .map(|(index, node)| {
                (
                    MathFlowId::new(u32::try_from(index).unwrap()),
                    node.domain().node_id.get(),
                    1u32,
                )
            })
            .collect::<Vec<_>>();
        let vector = prepare_staging_math_vector_flows(
            &fixture.package,
            &fixture.profile,
            &fixture.limits,
            &fixture.admitted,
            &fixture.bindings,
        )
        .unwrap();
        assert_eq!(
            native_preorder,
            [(MathFlowId::new(0), 2, 1), (MathFlowId::new(1), 4, 1),]
        );
        assert_eq!(
            vector
                .flows()
                .iter()
                .map(|flow| (flow.flow_id().get(), flow.owner().get()))
                .collect::<Vec<_>>(),
            [(0, 3), (1, 5)]
        );
        assert_eq!(vector.equation_number_shapes().len(), 1);
        assert!(vector.equation_number_shape(NodeId::new(3)).is_none());
        assert!(vector.equation_number_shape(NodeId::new(5)).is_some());
        let permuted = build_staging_math_vector_flows(
            &fixture.package,
            &fixture.profile,
            &fixture.limits,
            &fixture.admitted,
            &fixture.bindings,
            CandidateSchedule::ReverseWorkerCompletion,
        )
        .unwrap();
        assert_eq!(vector, permuted);
        assert_ne!(
            MATH_VECTOR_FLOW_ALGORITHM,
            crate::MATH_DISPLAY_FLOW_ALGORITHM
        );

        // The frozen native path remains separately authorized: each native
        // display terminal has exactly one selected placement, and its `/1`
        // bytes do not acquire vector vocabulary.
        let native_fixture = staging_math_layout_fixture().unwrap();
        let selected_native_flows = native_fixture
            .layout
            .placements()
            .iter()
            .filter_map(|placement| placement.display_flow_id())
            .collect::<Vec<_>>();
        assert_eq!(
            selected_native_flows,
            native_fixture
                .layout
                .display_flows()
                .iter()
                .map(|flow| flow.flow_id())
                .collect::<Vec<_>>()
        );
        assert!(native_fixture
            .layout
            .display_flows()
            .iter()
            .all(|flow| flow.terminal() == 1));
        assert_eq!(
            native_fixture.layout.display_flows()[0].fingerprint(),
            [
                0x0e, 0x85, 0x51, 0x10, 0x7f, 0x50, 0x57, 0xac, 0x1b, 0xcc, 0x1f, 0xa0, 0xe9, 0x24,
                0x67, 0x39, 0x53, 0x63, 0x52, 0xb9, 0x3e, 0xd8, 0x15, 0xb4, 0x04, 0xab, 0xdb, 0xa6,
                0x8b, 0x9f, 0x7b, 0x8a,
            ]
        );
        assert_eq!(
            sha256(native_fixture.layout.canonical_jcs().as_bytes()),
            [
                0xa7, 0x27, 0xb7, 0xa8, 0x28, 0x74, 0xcc, 0x4b, 0x68, 0xd8, 0xaa, 0xbf, 0xe3, 0xf1,
                0x90, 0x93, 0x90, 0xd4, 0x1d, 0xb0, 0x37, 0xe9, 0x55, 0xd5, 0x6a, 0xb2, 0x0d, 0x2c,
                0x69, 0x8e, 0x61, 0xb1,
            ]
        );
        assert_eq!(crate::MATH_DISPLAY_FLOW_ALGORITHM, "typaxis.math-flow/1");
        assert!(!native_fixture
            .layout
            .canonical_jcs()
            .contains("math_vector"));
        let mut ledger = vector.terminal_ledger().unwrap();
        assert_eq!(
            ledger.defer_page_move(MathVectorFlowId::new(0), NodeId::new(5)),
            Err(StagingMathVectorTerminalError::OwnerMismatch(
                MathVectorFlowId::new(0)
            ))
        );
        ledger
            .defer_page_move(MathVectorFlowId::new(1), NodeId::new(5))
            .unwrap();
        ledger
            .consume_selected(MathVectorFlowId::new(1), NodeId::new(5))
            .unwrap();
        assert_eq!(
            ledger.consume_selected(MathVectorFlowId::new(1), NodeId::new(5)),
            Err(StagingMathVectorTerminalError::AlreadyConsumed(
                MathVectorFlowId::new(1)
            ))
        );
        ledger
            .defer_page_move(MathVectorFlowId::new(0), NodeId::new(3))
            .unwrap();
        ledger
            .consume_selected(MathVectorFlowId::new(0), NodeId::new(3))
            .unwrap();
        let terminal = ledger.finish().unwrap();
        terminal.verify(&vector).unwrap();
        assert_eq!(
            terminal
                .receipts()
                .iter()
                .map(|receipt| {
                    (
                        receipt.flow_id().get(),
                        receipt.owner().get(),
                        receipt.terminal().get(),
                    )
                })
                .collect::<Vec<_>>(),
            [(0, 3, 1), (1, 5, 1)]
        );

        let mut source_order = vector.terminal_ledger().unwrap();
        source_order
            .consume_selected(MathVectorFlowId::new(0), NodeId::new(3))
            .unwrap();
        source_order
            .consume_selected(MathVectorFlowId::new(1), NodeId::new(5))
            .unwrap();
        assert_eq!(source_order.finish().unwrap(), terminal);

        let mut tampered_terminal = terminal.clone();
        tampered_terminal.receipts[0].terminal = MathVectorFlowTerminal::new(2);
        assert_eq!(
            tampered_terminal.verify(&vector),
            Err(StagingMathVectorTerminalError::RegistryMismatch)
        );

        let mut missing = vector.terminal_ledger().unwrap();
        missing
            .consume_selected(MathVectorFlowId::new(0), NodeId::new(3))
            .unwrap();
        assert_eq!(
            missing.finish(),
            Err(StagingMathVectorTerminalError::MissingConsumption(
                MathVectorFlowId::new(1)
            ))
        );
    }
}
