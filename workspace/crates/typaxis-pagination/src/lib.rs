#![forbid(unsafe_code)]

mod advanced_header_footer;

pub use advanced_header_footer::{
    derive_staging_header_footer_body_pages, paginate_staging_header_footer,
    StagingAdvancedFlowPosition, StagingAdvancedPageFrameKind, StagingHeaderFooterBodyPage,
    StagingHeaderFooterPaginationError, StagingHeaderFooterSelectedLayout,
    StagingHeaderFooterSelectedLayoutReceipt, StagingPageMargins, StagingPdfPageBox,
    StagingRepeatedRegionFragment, StagingSelectedAdvancedFrame, StagingSelectedAdvancedPage,
    StagingSelectedPageBoxes, ADVANCED_SELECTED_LAYOUT_ALGORITHM,
};

use core::fmt;
use core::num::NonZeroU16;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use typaxis_core::{
    initial_pagination_state_fingerprint_from_jcs,
    materialized_pagination_state_fingerprint_from_jcs, push_generated_buffer_key_jcs,
    push_jcs_string, sha256, AnchorId, FootnoteId, GeneratedBufferKey, GenerationKind,
    ImageResourceId, LayoutStateFingerprint, Length, MasterId, NodeId, NonNegativeLength, Point,
    PositiveLength, Rect, ReferenceFingerprint, Utf8ByteOffset, ValidatedResourceLimits,
    JSON_SAFE_INTEGER_MAX,
};
use typaxis_diagnostics::{
    AdvisoryDiagnostic, DiagnosticBuilder, DiagnosticCode, DiagnosticLocation, DiagnosticSubject,
    LayoutErrorSubject, Severity, SourceDiagnosticLocation, G6002, I9190, L5100, L5110,
};
use typaxis_document::GeneratedSiteTarget;
use typaxis_layout::{
    footnote_page_evaluation_fingerprint_from_jcs, footnote_selected_layout_fingerprint_from_jcs,
    multi_flow_selected_state_fingerprint_from_jcs, table_selected_layout_fingerprint_from_jcs,
    Continuation, DiscoveredAnchor, FlowContentKind, FlowCursor, FlowId, FlowPosition,
    FlowRegistryFingerprint, FlowTree, FootnoteFlowId, FootnoteFlowRegistryFingerprint,
    FootnotePageEvaluationFingerprint, FootnoteProfileFingerprint,
    FootnoteSelectedLayoutFingerprint, FragmentDraft, FragmentError, FragmentRequest,
    FragmentWorkBudget, Fragmenter, LayoutEpoch, MultiFlowSelectedStateFingerprint, PageContext,
    ProductionFlowIr, ProductionFlowPosition, ReferenceFragmenter, ResolvedPageSelection,
    StagingFigureKeepPolicy, StagingFigureOversizePolicy, StagingFootnoteFlowRegistry,
    StagingForcedPageBreakLayoutReceipt, StagingMachineListLayoutReceipt, TableCellLayoutReceipt,
    TableRowBandLayoutReceipt, TableRowBandReceipt, TableSection, TableSelectedLayoutFingerprint,
    ValidatedFigureLayout, FOOTNOTE_SEPARATOR_BAND_RAW,
};
use typaxis_style::{PageMasterSet, StyleValue};
use typaxis_syntax::{PackagePaginationContext, ValidatedParsedPackage};
use typaxis_text::{
    GeneratedBufferDraft, GeneratedProvenance, GeneratedTextStore, GeneratedTextStoreError,
};

pub const FALLBACK_POLICY_ID: &str = "lowest_cost_then_earliest";

#[derive(Clone)]
struct PaginationSessionId(Arc<()>);

impl PaginationSessionId {
    fn issue() -> Self {
        Self(Arc::new(()))
    }

    fn same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for PaginationSessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PaginationSessionId(<opaque>)")
    }
}

impl PartialEq for PaginationSessionId {
    fn eq(&self, other: &Self) -> bool {
        self.same_as(other)
    }
}

impl Eq for PaginationSessionId {}

pub const MULTI_FLOW_TRACE_FACTS_ALGORITHM: &str = "typaxis.multi-flow-trace-facts/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MultiFlowError {
    RegistryMismatch,
    EpochMismatch,
    UnknownFlow(FlowId),
    WrongOwner(FlowId),
    WrongParent(FlowId),
    WrongTerminal(FlowId),
    MissingFlow(FlowId),
    ExtraFlow(FlowId),
    DuplicateFlow(FlowId),
    ChildNotAtCursor(FlowId),
    FlowAlreadyVisited(FlowId),
    BodyCannotBeNested,
    BodyCannotBeLeft,
    ArithmeticOverflow,
    AllocationFailure,
}

/// Package/epoch/registry-bound cursor ledger for nested pagination. Progress
/// is stored per flow and the active stack stores only explicit parent/child
/// transitions; subflow boundaries are never flattened into the body cursor.
#[derive(Debug)]
pub struct MultiFlowCursorReceipt {
    registry: FlowRegistryFingerprint,
    epoch: LayoutEpoch,
    progress: Vec<u32>,
    visited: Vec<bool>,
    active_stack: Vec<FlowId>,
}

impl MultiFlowCursorReceipt {
    pub fn new(ir: &ProductionFlowIr) -> Result<Self, MultiFlowError> {
        let flow_count = ir.registry().flows().len();
        if flow_count == 0
            || ir.registry().flows()[0].flow_id() != FlowId::DOCUMENT_BODY
            || ir.flows().len() != flow_count
        {
            return Err(MultiFlowError::RegistryMismatch);
        }
        let mut progress = Vec::new();
        progress
            .try_reserve_exact(flow_count)
            .map_err(|_| MultiFlowError::AllocationFailure)?;
        progress.resize(flow_count, 0);
        let mut visited = Vec::new();
        visited
            .try_reserve_exact(flow_count)
            .map_err(|_| MultiFlowError::AllocationFailure)?;
        visited.resize(flow_count, false);
        visited[FlowId::DOCUMENT_BODY.get() as usize] = true;
        Ok(Self {
            registry: ir.registry().receipt().fingerprint(),
            epoch: ir.registry().receipt().epoch(),
            progress,
            visited,
            active_stack: vec![FlowId::DOCUMENT_BODY],
        })
    }

    pub const fn registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.registry
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub fn active_stack(&self) -> &[FlowId] {
        &self.active_stack
    }

    pub fn current_flow(&self) -> FlowId {
        *self
            .active_stack
            .last()
            .expect("a validated multi-flow cursor always retains the body")
    }

    pub fn flow_progress(&self, flow_id: FlowId) -> Option<u32> {
        self.progress.get(flow_id.get() as usize).copied()
    }

    pub fn current_position<'a>(
        &self,
        ir: &'a ProductionFlowIr,
    ) -> Result<&'a ProductionFlowPosition, MultiFlowError> {
        self.validate_ir(ir)?;
        let flow_id = self.current_flow();
        let flow = ir
            .flow(flow_id)
            .ok_or(MultiFlowError::UnknownFlow(flow_id))?;
        flow.positions()
            .get(self.progress[flow_id.get() as usize] as usize)
            .ok_or(MultiFlowError::WrongTerminal(flow_id))
    }

    pub fn advance(&mut self, ir: &ProductionFlowIr) -> Result<(), MultiFlowError> {
        self.validate_ir(ir)?;
        let flow_id = self.current_flow();
        let index = flow_id.get() as usize;
        let terminal = ir
            .registry()
            .flow(flow_id)
            .ok_or(MultiFlowError::UnknownFlow(flow_id))?
            .terminal()
            .owner_local_ordinal();
        if self.progress[index] >= terminal {
            return Err(MultiFlowError::WrongTerminal(flow_id));
        }
        self.progress[index] = self.progress[index]
            .checked_add(1)
            .ok_or(MultiFlowError::ArithmeticOverflow)?;
        Ok(())
    }

    pub fn enter_child(
        &mut self,
        ir: &ProductionFlowIr,
        child_flow_id: FlowId,
    ) -> Result<(), MultiFlowError> {
        self.validate_ir(ir)?;
        if child_flow_id == FlowId::DOCUMENT_BODY {
            return Err(MultiFlowError::BodyCannotBeNested);
        }
        let parent_flow_id = self.current_flow();
        let position = self.current_position(ir)?;
        if position.is_terminal() || position.child_flow_id() != Some(child_flow_id) {
            return Err(MultiFlowError::ChildNotAtCursor(child_flow_id));
        }
        let child = ir
            .registry()
            .flow(child_flow_id)
            .ok_or(MultiFlowError::UnknownFlow(child_flow_id))?;
        if child.parent_flow_id() != Some(parent_flow_id) {
            return Err(MultiFlowError::WrongParent(child_flow_id));
        }
        let child_index = child_flow_id.get() as usize;
        if self.visited[child_index] {
            return Err(MultiFlowError::FlowAlreadyVisited(child_flow_id));
        }
        self.active_stack
            .try_reserve(1)
            .map_err(|_| MultiFlowError::AllocationFailure)?;
        self.visited[child_index] = true;
        self.active_stack.push(child_flow_id);
        Ok(())
    }

    pub fn leave_terminal(&mut self, ir: &ProductionFlowIr) -> Result<(), MultiFlowError> {
        self.validate_ir(ir)?;
        let flow_id = self.current_flow();
        if flow_id == FlowId::DOCUMENT_BODY {
            return Err(MultiFlowError::BodyCannotBeLeft);
        }
        let terminal = ir
            .registry()
            .flow(flow_id)
            .ok_or(MultiFlowError::UnknownFlow(flow_id))?
            .terminal()
            .owner_local_ordinal();
        if self.progress[flow_id.get() as usize] != terminal {
            return Err(MultiFlowError::WrongTerminal(flow_id));
        }
        self.active_stack.pop();
        Ok(())
    }

    pub fn finish(
        self,
        ir: &ProductionFlowIr,
    ) -> Result<MultiFlowSelectedStateReceipt, MultiFlowError> {
        self.validate_ir(ir)?;
        if self.active_stack != [FlowId::DOCUMENT_BODY] {
            return Err(MultiFlowError::WrongParent(self.current_flow()));
        }
        let mut completed = Vec::new();
        completed
            .try_reserve_exact(ir.registry().flows().len())
            .map_err(|_| MultiFlowError::AllocationFailure)?;
        for flow in ir.registry().flows() {
            let index = flow.flow_id().get() as usize;
            if !self.visited[index] {
                return Err(MultiFlowError::MissingFlow(flow.flow_id()));
            }
            if self.progress[index] != flow.terminal().owner_local_ordinal() {
                return Err(MultiFlowError::WrongTerminal(flow.flow_id()));
            }
            completed.push(CompletedFlowReceipt::from_ir(ir, flow.flow_id())?);
        }
        MultiFlowSelectedStateReceipt::from_completed(ir, completed)
    }

    fn validate_ir(&self, ir: &ProductionFlowIr) -> Result<(), MultiFlowError> {
        if self.registry != ir.registry().receipt().fingerprint()
            || self.progress.len() != ir.registry().flows().len()
            || self.visited.len() != ir.registry().flows().len()
        {
            return Err(MultiFlowError::RegistryMismatch);
        }
        if self.epoch != ir.registry().receipt().epoch() {
            return Err(MultiFlowError::EpochMismatch);
        }
        Ok(())
    }
}

/// Independent worker cursor. It can advance only inside one flow and seals a
/// completion receipt only at that flow's registry-issued terminal.
#[derive(Debug)]
pub struct FlowWorkerCursor {
    registry: FlowRegistryFingerprint,
    epoch: LayoutEpoch,
    flow_id: FlowId,
    next_boundary: u32,
}

impl FlowWorkerCursor {
    pub fn new(ir: &ProductionFlowIr, flow_id: FlowId) -> Result<Self, MultiFlowError> {
        ir.registry()
            .flow(flow_id)
            .ok_or(MultiFlowError::UnknownFlow(flow_id))?;
        Ok(Self {
            registry: ir.registry().receipt().fingerprint(),
            epoch: ir.registry().receipt().epoch(),
            flow_id,
            next_boundary: 0,
        })
    }

    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }

    pub const fn next_boundary(&self) -> u32 {
        self.next_boundary
    }

    pub fn advance(&mut self, ir: &ProductionFlowIr) -> Result<(), MultiFlowError> {
        self.validate_ir(ir)?;
        let terminal = ir
            .registry()
            .flow(self.flow_id)
            .ok_or(MultiFlowError::UnknownFlow(self.flow_id))?
            .terminal()
            .owner_local_ordinal();
        if self.next_boundary >= terminal {
            return Err(MultiFlowError::WrongTerminal(self.flow_id));
        }
        self.next_boundary = self
            .next_boundary
            .checked_add(1)
            .ok_or(MultiFlowError::ArithmeticOverflow)?;
        Ok(())
    }

    pub fn finish(self, ir: &ProductionFlowIr) -> Result<CompletedFlowReceipt, MultiFlowError> {
        self.validate_ir(ir)?;
        let terminal = ir
            .registry()
            .flow(self.flow_id)
            .ok_or(MultiFlowError::UnknownFlow(self.flow_id))?
            .terminal()
            .owner_local_ordinal();
        if self.next_boundary != terminal {
            return Err(MultiFlowError::WrongTerminal(self.flow_id));
        }
        CompletedFlowReceipt::from_ir(ir, self.flow_id)
    }

    fn validate_ir(&self, ir: &ProductionFlowIr) -> Result<(), MultiFlowError> {
        if self.registry != ir.registry().receipt().fingerprint() {
            return Err(MultiFlowError::RegistryMismatch);
        }
        if self.epoch != ir.registry().receipt().epoch() {
            return Err(MultiFlowError::EpochMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct CompletedFlowReceipt {
    registry: FlowRegistryFingerprint,
    epoch: LayoutEpoch,
    flow_id: FlowId,
    owner_node_id: NodeId,
    parent_flow_id: Option<FlowId>,
    terminal: u32,
}

impl CompletedFlowReceipt {
    fn from_ir(ir: &ProductionFlowIr, flow_id: FlowId) -> Result<Self, MultiFlowError> {
        let flow = ir
            .registry()
            .flow(flow_id)
            .ok_or(MultiFlowError::UnknownFlow(flow_id))?;
        Ok(Self {
            registry: ir.registry().receipt().fingerprint(),
            epoch: ir.registry().receipt().epoch(),
            flow_id,
            owner_node_id: flow.owner_node_id(),
            parent_flow_id: flow.parent_flow_id(),
            terminal: flow.terminal().owner_local_ordinal(),
        })
    }

    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
}

/// Accepts independently completed flow workers in arbitrary order, then
/// seals one canonical all-flow selected-state receipt.
pub struct MultiFlowSelectionBuilder<'a> {
    ir: &'a ProductionFlowIr,
    completed: Vec<CompletedFlowReceipt>,
}

impl<'a> MultiFlowSelectionBuilder<'a> {
    pub const fn new(ir: &'a ProductionFlowIr) -> Self {
        Self {
            ir,
            completed: Vec::new(),
        }
    }

    pub fn register(&mut self, completed: CompletedFlowReceipt) -> Result<(), MultiFlowError> {
        if self.completed.len() >= self.ir.registry().flows().len() {
            return Err(MultiFlowError::ExtraFlow(completed.flow_id));
        }
        self.completed
            .try_reserve(1)
            .map_err(|_| MultiFlowError::AllocationFailure)?;
        self.completed.push(completed);
        Ok(())
    }

    pub fn finish(self) -> Result<MultiFlowSelectedStateReceipt, MultiFlowError> {
        MultiFlowSelectedStateReceipt::from_completed(self.ir, self.completed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedFlowTerminal {
    flow_id: FlowId,
    owner_node_id: NodeId,
    parent_flow_id: Option<FlowId>,
    terminal: u32,
}

impl SelectedFlowTerminal {
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }

    pub const fn owner_node_id(&self) -> NodeId {
        self.owner_node_id
    }

    pub const fn parent_flow_id(&self) -> Option<FlowId> {
        self.parent_flow_id
    }

    pub const fn terminal(&self) -> u32 {
        self.terminal
    }
}

/// Selected-state closure over every registry flow. The receipt is
/// non-cloneable and can be built only from terminal worker/stack receipts.
#[derive(Debug)]
pub struct MultiFlowSelectedStateReceipt {
    registry: FlowRegistryFingerprint,
    epoch: LayoutEpoch,
    terminals: Vec<SelectedFlowTerminal>,
    fingerprint: MultiFlowSelectedStateFingerprint,
}

impl MultiFlowSelectedStateReceipt {
    fn from_completed(
        ir: &ProductionFlowIr,
        mut completed: Vec<CompletedFlowReceipt>,
    ) -> Result<Self, MultiFlowError> {
        completed.sort_by_key(|receipt| receipt.flow_id);
        if let Some(pair) = completed
            .windows(2)
            .find(|pair| pair[0].flow_id == pair[1].flow_id)
        {
            return Err(MultiFlowError::DuplicateFlow(pair[1].flow_id));
        }
        let mut terminals = Vec::new();
        terminals
            .try_reserve_exact(ir.registry().flows().len())
            .map_err(|_| MultiFlowError::AllocationFailure)?;
        for expected in ir.registry().flows() {
            let index = expected.flow_id().get() as usize;
            let actual = completed
                .get(index)
                .ok_or(MultiFlowError::MissingFlow(expected.flow_id()))?;
            if actual.flow_id != expected.flow_id() {
                if actual.flow_id.get() < expected.flow_id().get() {
                    return Err(MultiFlowError::ExtraFlow(actual.flow_id));
                }
                return Err(MultiFlowError::MissingFlow(expected.flow_id()));
            }
            if actual.registry != ir.registry().receipt().fingerprint() {
                return Err(MultiFlowError::RegistryMismatch);
            }
            if actual.epoch != ir.registry().receipt().epoch() {
                return Err(MultiFlowError::EpochMismatch);
            }
            if actual.owner_node_id != expected.owner_node_id() {
                return Err(MultiFlowError::WrongOwner(expected.flow_id()));
            }
            if actual.parent_flow_id != expected.parent_flow_id() {
                return Err(MultiFlowError::WrongParent(expected.flow_id()));
            }
            if actual.terminal != expected.terminal().owner_local_ordinal() {
                return Err(MultiFlowError::WrongTerminal(expected.flow_id()));
            }
            terminals.push(SelectedFlowTerminal {
                flow_id: actual.flow_id,
                owner_node_id: actual.owner_node_id,
                parent_flow_id: actual.parent_flow_id,
                terminal: actual.terminal,
            });
        }
        if completed.len() > terminals.len() {
            return Err(MultiFlowError::ExtraFlow(
                completed[terminals.len()].flow_id,
            ));
        }
        let registry = ir.registry().receipt().fingerprint();
        let epoch = ir.registry().receipt().epoch();
        let canonical_jcs = encode_multi_flow_selected_state(registry, epoch, &terminals);
        let fingerprint = multi_flow_selected_state_fingerprint_from_jcs(&canonical_jcs);
        Ok(Self {
            registry,
            epoch,
            terminals,
            fingerprint,
        })
    }

    pub const fn registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.registry
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub fn terminals(&self) -> &[SelectedFlowTerminal] {
        &self.terminals
    }

    pub const fn fingerprint(&self) -> MultiFlowSelectedStateFingerprint {
        self.fingerprint
    }
}

fn encode_multi_flow_selected_state(
    registry: FlowRegistryFingerprint,
    epoch: LayoutEpoch,
    terminals: &[SelectedFlowTerminal],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MultiFlowSelectedStateFingerprint::ALGORITHM_ID);
    output.push_str(",\"flow_registry_sha256\":");
    push_hex(&mut output, registry.bytes());
    output.push_str(",\"flow_terminals\":[");
    for (index, terminal) in terminals.iter().enumerate() {
        comma(&mut output, index);
        output.push_str("{\"flow_id\":");
        output.push_str(&terminal.flow_id.get().to_string());
        output.push_str(",\"owner_node_id\":");
        output.push_str(&terminal.owner_node_id.get().to_string());
        output.push_str(",\"parent_flow_id\":");
        push_optional_flow_id(&mut output, terminal.parent_flow_id);
        output.push_str(",\"terminal\":");
        output.push_str(&terminal.terminal.to_string());
        output.push('}');
    }
    output.push_str("],\"layout_epoch\":");
    encode_layout_epoch(&mut output, epoch);
    output.push('}');
    output
}

/// Canonical trace projection for the versioned 1.2 multi-flow facts. It carries
/// every flow position in dense flow/owner order and is derived only from a
/// matching all-flow selected receipt.
#[derive(Debug)]
pub struct MultiFlowTraceFacts {
    registry: FlowRegistryFingerprint,
    selected: MultiFlowSelectedStateFingerprint,
    positions: Vec<ProductionFlowPosition>,
    canonical_jcs: String,
}

impl MultiFlowTraceFacts {
    pub fn new(
        ir: &ProductionFlowIr,
        selected: &MultiFlowSelectedStateReceipt,
    ) -> Result<Self, MultiFlowError> {
        if selected.registry != ir.registry().receipt().fingerprint() {
            return Err(MultiFlowError::RegistryMismatch);
        }
        if selected.epoch != ir.registry().receipt().epoch()
            || selected.terminals.len() != ir.registry().flows().len()
        {
            return Err(MultiFlowError::EpochMismatch);
        }
        let position_count = ir.flows().iter().try_fold(0usize, |total, flow| {
            total
                .checked_add(flow.positions().len())
                .ok_or(MultiFlowError::ArithmeticOverflow)
        })?;
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(position_count)
            .map_err(|_| MultiFlowError::AllocationFailure)?;
        for flow in ir.flows() {
            positions.extend_from_slice(flow.positions());
        }
        let canonical_jcs = encode_multi_flow_trace_facts(
            ir.registry().receipt().fingerprint(),
            selected.fingerprint,
            &positions,
        );
        Ok(Self {
            registry: ir.registry().receipt().fingerprint(),
            selected: selected.fingerprint,
            positions,
            canonical_jcs,
        })
    }

    pub const fn registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.registry
    }

    pub const fn selected_state_fingerprint(&self) -> MultiFlowSelectedStateFingerprint {
        self.selected
    }

    pub fn positions(&self) -> &[ProductionFlowPosition] {
        &self.positions
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn encode_multi_flow_trace_facts(
    registry: FlowRegistryFingerprint,
    selected: MultiFlowSelectedStateFingerprint,
    positions: &[ProductionFlowPosition],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, MULTI_FLOW_TRACE_FACTS_ALGORITHM);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"flow_positions\":[");
    for (index, position) in positions.iter().enumerate() {
        comma(&mut output, index);
        encode_production_flow_position(&mut output, position);
    }
    output.push_str("],\"flow_registry_sha256\":");
    push_hex(&mut output, registry.bytes());
    output.push_str(",\"selected_state_sha256\":");
    push_hex(&mut output, selected.bytes());
    output.push('}');
    output
}

fn encode_production_flow_position(output: &mut String, position: &ProductionFlowPosition) {
    output.push_str("{\"block_child_path\":[");
    for (index, component) in position.block_child_path().iter().enumerate() {
        comma(output, index);
        output.push_str(&component.to_string());
    }
    output.push_str("],\"child_flow_id\":");
    push_optional_flow_id(output, position.child_flow_id());
    output.push_str(",\"content_kind\":");
    match position.content_kind() {
        Some(kind) => push_jcs_string(output, kind.as_str()),
        None => output.push_str("null"),
    }
    output.push_str(",\"content_owner_node_id\":");
    match position.content_owner_node_id() {
        Some(owner) => output.push_str(&owner.get().to_string()),
        None => output.push_str("null"),
    }
    output.push_str(",\"epoch\":");
    encode_layout_epoch(output, position.epoch());
    output.push_str(",\"flow_id\":");
    output.push_str(&position.flow_id().get().to_string());
    output.push_str(",\"flow_local_ordinal\":");
    output.push_str(&position.flow_local_ordinal().to_string());
    output.push_str(",\"owner_local_boundary\":");
    output.push_str(&position.owner_local_boundary().to_string());
    output.push_str(",\"owner_node_id\":");
    output.push_str(&position.flow_owner_node_id().get().to_string());
    output.push_str(",\"parent_flow_id\":");
    push_optional_flow_id(output, position.parent_flow_id());
    output.push_str(",\"terminal\":");
    output.push_str(if position.is_terminal() {
        "true"
    } else {
        "false"
    });
    output.push('}');
}

fn push_optional_flow_id(output: &mut String, flow_id: Option<FlowId>) {
    match flow_id {
        Some(flow_id) => output.push_str(&flow_id.get().to_string()),
        None => output.push_str("null"),
    }
}

pub const STAGING_TABLE_TRACE_ALGORITHM: &str = "typaxis.table-layout-trace/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingTableOversizeTerminal {
    row_owner: NodeId,
    page_index: u32,
    transition_count: u8,
}

impl StagingTableOversizeTerminal {
    pub const fn row_owner(self) -> NodeId {
        self.row_owner
    }
    pub const fn page_index(self) -> u32 {
        self.page_index
    }
    pub const fn transition_count(self) -> u8 {
        self.transition_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingTablePaginationError {
    Flow(MultiFlowError),
    LayoutRegistryMismatch,
    InvalidPageInput,
    HeaderOversize(NodeId),
    RowOversize(StagingTableOversizeTerminal),
    PageLimit,
    FragmentLimit,
    NoProgress(NodeId),
    SameCandidateRetry(NodeId),
    MissingContinuation { column_ordinal: u32 },
    DuplicateContinuation { column_ordinal: u32 },
    WrongContinuationOwner { column_ordinal: u32 },
    WrongRepetitionIndex { expected: u32, actual: u32 },
    SelectedStateMismatch,
    ArithmeticOverflow,
    AllocationFailure,
}

impl From<MultiFlowError> for StagingTablePaginationError {
    fn from(value: MultiFlowError) -> Self {
        Self::Flow(value)
    }
}

impl StagingTablePaginationError {
    pub const fn diagnostic_code(self) -> DiagnosticCode {
        match self {
            Self::HeaderOversize(_) | Self::RowOversize(_) => L5100,
            Self::FragmentLimit => L5110,
            Self::Flow(_)
            | Self::LayoutRegistryMismatch
            | Self::InvalidPageInput
            | Self::PageLimit
            | Self::NoProgress(_)
            | Self::SameCandidateRetry(_)
            | Self::MissingContinuation { .. }
            | Self::DuplicateContinuation { .. }
            | Self::WrongContinuationOwner { .. }
            | Self::WrongRepetitionIndex { .. }
            | Self::SelectedStateMismatch
            | Self::ArithmeticOverflow
            | Self::AllocationFailure => I9190,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingTablePageInput {
    body_block_size: PositiveLength,
    first_page_remaining_block_size: PositiveLength,
}

impl StagingTablePageInput {
    pub fn new(
        body_block_size: PositiveLength,
        first_page_remaining_block_size: PositiveLength,
    ) -> Result<Self, StagingTablePaginationError> {
        if first_page_remaining_block_size.get().raw() > body_block_size.get().raw() {
            return Err(StagingTablePaginationError::InvalidPageInput);
        }
        Ok(Self {
            body_block_size,
            first_page_remaining_block_size,
        })
    }

    pub const fn body_block_size(self) -> PositiveLength {
        self.body_block_size
    }
    pub const fn first_page_remaining_block_size(self) -> PositiveLength {
        self.first_page_remaining_block_size
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingTableCellFlowCursor {
    flow_id: FlowId,
    next_fragment_ordinal: u32,
    terminal: bool,
}

impl StagingTableCellFlowCursor {
    pub const fn flow_id(self) -> FlowId {
        self.flow_id
    }
    pub const fn next_fragment_ordinal(self) -> u32 {
        self.next_fragment_ordinal
    }
    pub const fn is_terminal(self) -> bool {
        self.terminal
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingTableRowCursor {
    logical_row_ordinal: u32,
    row_fragment_ordinal: u32,
    block_offset_within_row: i64,
    terminal: bool,
}

impl StagingTableRowCursor {
    const fn start() -> Self {
        Self {
            logical_row_ordinal: 0,
            row_fragment_ordinal: 0,
            block_offset_within_row: 0,
            terminal: false,
        }
    }

    pub const fn logical_row_ordinal(self) -> u32 {
        self.logical_row_ordinal
    }
    pub const fn row_fragment_ordinal(self) -> u32 {
        self.row_fragment_ordinal
    }
    pub const fn block_offset_within_row(self) -> i64 {
        self.block_offset_within_row
    }
    pub const fn is_terminal(self) -> bool {
        self.terminal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTableCellFragmentReceipt {
    cell_owner: NodeId,
    flow_id: FlowId,
    selected_block_extent: i64,
    vertical_offset_before: i64,
    vertical_offset_after: i64,
    before_cursor: StagingTableCellFlowCursor,
    after_cursor: StagingTableCellFlowCursor,
}

impl StagingTableCellFragmentReceipt {
    pub const fn cell_owner(&self) -> NodeId {
        self.cell_owner
    }
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
    pub const fn selected_block_extent(&self) -> i64 {
        self.selected_block_extent
    }
    pub const fn vertical_offset_before(&self) -> i64 {
        self.vertical_offset_before
    }
    pub const fn vertical_offset_after(&self) -> i64 {
        self.vertical_offset_after
    }
    pub const fn before_cursor(&self) -> StagingTableCellFlowCursor {
        self.before_cursor
    }
    pub const fn after_cursor(&self) -> StagingTableCellFlowCursor {
        self.after_cursor
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowspanContinuationEntry {
    column_ordinal: u32,
    cell_owner: NodeId,
    flow_id: FlowId,
    cell_flow_cursor: StagingTableCellFlowCursor,
    vertical_offset: i64,
    remaining_logical_rows: NonZeroU16,
}

impl RowspanContinuationEntry {
    pub const fn column_ordinal(&self) -> u32 {
        self.column_ordinal
    }
    pub const fn cell_owner(&self) -> NodeId {
        self.cell_owner
    }
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
    pub const fn cell_flow_cursor(&self) -> StagingTableCellFlowCursor {
        self.cell_flow_cursor
    }
    pub const fn vertical_offset(&self) -> i64 {
        self.vertical_offset
    }
    pub const fn remaining_logical_rows(&self) -> NonZeroU16 {
        self.remaining_logical_rows
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowspanContinuationReceipt {
    logical_row_ordinal: u32,
    entries: Vec<RowspanContinuationEntry>,
}

impl RowspanContinuationReceipt {
    fn empty(logical_row_ordinal: u32) -> Self {
        Self {
            logical_row_ordinal,
            entries: Vec::new(),
        }
    }

    pub const fn logical_row_ordinal(&self) -> u32 {
        self.logical_row_ordinal
    }
    pub fn entries(&self) -> &[RowspanContinuationEntry] {
        &self.entries
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowFragmentReceipt {
    fragment_id: u64,
    row_owner: NodeId,
    logical_row_ordinal: u32,
    row_fragment_ordinal: u32,
    page_index: u32,
    page_block_offset: i64,
    selected_block_extent: i64,
    before_cursor: StagingTableRowCursor,
    after_cursor: StagingTableRowCursor,
    cells: Vec<StagingTableCellFragmentReceipt>,
    continuation_before: RowspanContinuationReceipt,
    continuation_after: RowspanContinuationReceipt,
}

impl RowFragmentReceipt {
    pub const fn fragment_id(&self) -> u64 {
        self.fragment_id
    }
    pub const fn row_owner(&self) -> NodeId {
        self.row_owner
    }
    pub const fn logical_row_ordinal(&self) -> u32 {
        self.logical_row_ordinal
    }
    pub const fn row_fragment_ordinal(&self) -> u32 {
        self.row_fragment_ordinal
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn page_block_offset(&self) -> i64 {
        self.page_block_offset
    }
    pub const fn selected_block_extent(&self) -> i64 {
        self.selected_block_extent
    }
    pub const fn before_cursor(&self) -> StagingTableRowCursor {
        self.before_cursor
    }
    pub const fn after_cursor(&self) -> StagingTableRowCursor {
        self.after_cursor
    }
    pub fn cells(&self) -> &[StagingTableCellFragmentReceipt] {
        &self.cells
    }
    pub const fn continuation_before(&self) -> &RowspanContinuationReceipt {
        &self.continuation_before
    }
    pub const fn continuation_after(&self) -> &RowspanContinuationReceipt {
        &self.continuation_after
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTableHeaderSourceFragment {
    source_fragment_id: u64,
    row_owner: NodeId,
    row_ordinal: u32,
    group_block_offset: i64,
    selected_block_extent: i64,
    cells: Vec<StagingTableCellFragmentReceipt>,
}

impl StagingTableHeaderSourceFragment {
    pub const fn source_fragment_id(&self) -> u64 {
        self.source_fragment_id
    }
    pub const fn row_owner(&self) -> NodeId {
        self.row_owner
    }
    pub const fn row_ordinal(&self) -> u32 {
        self.row_ordinal
    }
    pub const fn group_block_offset(&self) -> i64 {
        self.group_block_offset
    }
    pub const fn selected_block_extent(&self) -> i64 {
        self.selected_block_extent
    }
    pub fn cells(&self) -> &[StagingTableCellFragmentReceipt] {
        &self.cells
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTableHeaderRowOccurrence {
    fragment_id: u64,
    source_fragment_id: u64,
    row_owner: NodeId,
    target_block_offset: i64,
}

impl StagingTableHeaderRowOccurrence {
    pub const fn fragment_id(&self) -> u64 {
        self.fragment_id
    }
    pub const fn source_fragment_id(&self) -> u64 {
        self.source_fragment_id
    }
    pub const fn row_owner(&self) -> NodeId {
        self.row_owner
    }
    pub const fn target_block_offset(&self) -> i64 {
        self.target_block_offset
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HeaderRepetitionDraft {
    repetition_index: u32,
    target_page_index: u32,
    rows: Vec<StagingTableHeaderRowOccurrence>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct HeaderRepetitionReceipt {
    table_owner: NodeId,
    selected_state: TableSelectedLayoutFingerprint,
    repetition_index: u32,
    target_page_index: u32,
    rows: Vec<StagingTableHeaderRowOccurrence>,
}

impl HeaderRepetitionReceipt {
    pub const fn table_owner(&self) -> NodeId {
        self.table_owner
    }
    pub const fn selected_state_fingerprint(&self) -> TableSelectedLayoutFingerprint {
        self.selected_state
    }
    pub const fn repetition_index(&self) -> u32 {
        self.repetition_index
    }
    pub const fn target_page_index(&self) -> u32 {
        self.target_page_index
    }
    pub fn rows(&self) -> &[StagingTableHeaderRowOccurrence] {
        &self.rows
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingTableSelectedPage {
    page_index: u32,
    header_repetition_index: Option<u32>,
    header_fragment_ids: Vec<u64>,
    row_fragment_ids: Vec<u64>,
}

impl StagingTableSelectedPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn header_repetition_index(&self) -> Option<u32> {
        self.header_repetition_index
    }
    pub fn header_fragment_ids(&self) -> &[u64] {
        &self.header_fragment_ids
    }
    pub fn row_fragment_ids(&self) -> &[u64] {
        &self.row_fragment_ids
    }
}

/// Complete MI3-03 selected table state. Its constructor is private and binds
/// the all-flow terminal receipt, every common-extent row/cell fragment, every
/// continuation transition, and every original/repeated header occurrence.
#[derive(Debug)]
pub struct SelectedTableLayoutReceipt {
    package_sha256: [u8; 32],
    epoch: LayoutEpoch,
    flow_registry: FlowRegistryFingerprint,
    grid_sha256: [u8; 32],
    row_band_sha256: [u8; 32],
    table_owner: NodeId,
    body_block_size: i64,
    first_page_remaining_block_size: i64,
    page_count: u32,
    multi_flow: MultiFlowSelectedStateReceipt,
    pages: Vec<StagingTableSelectedPage>,
    header_sources: Vec<StagingTableHeaderSourceFragment>,
    header_repetitions: Vec<HeaderRepetitionReceipt>,
    row_fragments: Vec<RowFragmentReceipt>,
    fingerprint: TableSelectedLayoutFingerprint,
    canonical_jcs: String,
}

impl SelectedTableLayoutReceipt {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn flow_registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.flow_registry
    }
    pub const fn grid_sha256(&self) -> [u8; 32] {
        self.grid_sha256
    }
    pub const fn row_band_sha256(&self) -> [u8; 32] {
        self.row_band_sha256
    }
    pub const fn table_owner(&self) -> NodeId {
        self.table_owner
    }
    pub const fn body_block_size(&self) -> i64 {
        self.body_block_size
    }
    pub const fn first_page_remaining_block_size(&self) -> i64 {
        self.first_page_remaining_block_size
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn multi_flow(&self) -> &MultiFlowSelectedStateReceipt {
        &self.multi_flow
    }
    pub fn pages(&self) -> &[StagingTableSelectedPage] {
        &self.pages
    }
    pub fn header_sources(&self) -> &[StagingTableHeaderSourceFragment] {
        &self.header_sources
    }
    pub fn header_repetitions(&self) -> &[HeaderRepetitionReceipt] {
        &self.header_repetitions
    }
    pub fn row_fragments(&self) -> &[RowFragmentReceipt] {
        &self.row_fragments
    }
    pub const fn fingerprint(&self) -> TableSelectedLayoutFingerprint {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub fn trace_facts(&self) -> Result<StagingTableTraceFacts, StagingTablePaginationError> {
        StagingTableTraceFacts::from_selected(self)
    }

    pub fn validate_closure(
        &self,
        layout: &TableRowBandLayoutReceipt,
        ir: &ProductionFlowIr,
    ) -> Result<(), StagingTablePaginationError> {
        validate_selected_table_layout(self, layout, ir)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct StagingTableTraceFacts {
    flow_registry_sha256: [u8; 32],
    grid_sha256: [u8; 32],
    row_band_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    page_count: u32,
    row_fragment_count: u64,
    header_occurrence_count: u64,
    cell_fragment_count: u64,
    canonical_jcs: String,
}

impl StagingTableTraceFacts {
    fn from_selected(
        selected: &SelectedTableLayoutReceipt,
    ) -> Result<Self, StagingTablePaginationError> {
        let row_fragment_count = u64::try_from(selected.row_fragments.len())
            .map_err(|_| StagingTablePaginationError::ArithmeticOverflow)?;
        let header_occurrence_count =
            selected
                .header_repetitions
                .iter()
                .try_fold(0u64, |total, receipt| {
                    total
                        .checked_add(
                            u64::try_from(receipt.rows.len())
                                .map_err(|_| StagingTablePaginationError::ArithmeticOverflow)?,
                        )
                        .ok_or(StagingTablePaginationError::ArithmeticOverflow)
                })?;
        let body_cells = selected
            .row_fragments
            .iter()
            .try_fold(0u64, |total, fragment| {
                total
                    .checked_add(
                        u64::try_from(fragment.cells.len())
                            .map_err(|_| StagingTablePaginationError::ArithmeticOverflow)?,
                    )
                    .ok_or(StagingTablePaginationError::ArithmeticOverflow)
            })?;
        let header_cells = selected
            .header_sources
            .iter()
            .try_fold(0u64, |total, fragment| {
                total
                    .checked_add(
                        u64::try_from(fragment.cells.len())
                            .map_err(|_| StagingTablePaginationError::ArithmeticOverflow)?,
                    )
                    .ok_or(StagingTablePaginationError::ArithmeticOverflow)
            })?;
        let cell_fragment_count = body_cells
            .checked_add(header_cells)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        let mut canonical_jcs = String::from("{\"algorithm\":\"");
        canonical_jcs.push_str(STAGING_TABLE_TRACE_ALGORITHM);
        canonical_jcs.push_str("\",\"selected_layout\":");
        canonical_jcs.push_str(&selected.canonical_jcs);
        canonical_jcs.push_str(",\"selected_layout_sha256\":");
        push_hex(&mut canonical_jcs, selected.fingerprint.bytes());
        canonical_jcs.push('}');
        Ok(Self {
            flow_registry_sha256: selected.flow_registry.bytes(),
            grid_sha256: selected.grid_sha256,
            row_band_sha256: selected.row_band_sha256,
            selected_layout_sha256: selected.fingerprint.bytes(),
            page_count: selected.page_count,
            row_fragment_count,
            header_occurrence_count,
            cell_fragment_count,
            canonical_jcs,
        })
    }

    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }
    pub const fn grid_sha256(&self) -> [u8; 32] {
        self.grid_sha256
    }
    pub const fn row_band_sha256(&self) -> [u8; 32] {
        self.row_band_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn row_fragment_count(&self) -> u64 {
        self.row_fragment_count
    }
    pub const fn header_occurrence_count(&self) -> u64 {
        self.header_occurrence_count
    }
    pub const fn cell_fragment_count(&self) -> u64 {
        self.cell_fragment_count
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TableCandidateAttempt {
    row_owner: NodeId,
    row_fragment_ordinal: u32,
    page_index: u32,
    row_block_offset: i64,
    available_block_size: i64,
}

#[derive(Debug, Default)]
struct TableAttemptGuard {
    evaluated: BTreeSet<TableCandidateAttempt>,
    oversize_rows: BTreeSet<NodeId>,
}

impl TableAttemptGuard {
    fn record(
        &mut self,
        attempt: TableCandidateAttempt,
    ) -> Result<(), StagingTablePaginationError> {
        if !self.evaluated.insert(attempt) {
            return Err(StagingTablePaginationError::SameCandidateRetry(
                attempt.row_owner,
            ));
        }
        Ok(())
    }

    fn transition_oversize(
        &mut self,
        row_owner: NodeId,
        page_index: u32,
    ) -> StagingTablePaginationError {
        if !self.oversize_rows.insert(row_owner) {
            return StagingTablePaginationError::SameCandidateRetry(row_owner);
        }
        StagingTablePaginationError::RowOversize(StagingTableOversizeTerminal {
            row_owner,
            page_index,
            transition_count: 1,
        })
    }
}

struct StagingTablePaginator<'a> {
    layout: &'a TableRowBandLayoutReceipt,
    limits: &'a ValidatedResourceLimits,
    body_block_size: i64,
    remaining_block_size: i64,
    page_block_offset: i64,
    page_index: u32,
    table_page_ordinal: u32,
    page_open: bool,
    planned_take: Option<(u32, i64, u32, i64)>,
    consumed_fragment_count: u64,
    next_fragment_id: u64,
    pages: Vec<StagingTableSelectedPage>,
    header_sources: Vec<StagingTableHeaderSourceFragment>,
    header_drafts: Vec<HeaderRepetitionDraft>,
    row_fragments: Vec<RowFragmentReceipt>,
    continuation: RowspanContinuationReceipt,
    row_cursor: StagingTableRowCursor,
    body_row_count: u32,
    attempts: TableAttemptGuard,
}

impl<'a> StagingTablePaginator<'a> {
    fn new(
        layout: &'a TableRowBandLayoutReceipt,
        input: StagingTablePageInput,
        limits: &'a ValidatedResourceLimits,
    ) -> Result<Self, StagingTablePaginationError> {
        if layout.contained_fragment_count() > limits.get().max_fragments {
            return Err(StagingTablePaginationError::FragmentLimit);
        }
        let body_row_count = u32::try_from(
            layout
                .rows()
                .iter()
                .filter(|row| row.section() == TableSection::Body)
                .count(),
        )
        .map_err(|_| StagingTablePaginationError::ArithmeticOverflow)?;
        let body_block_size = input.body_block_size.get().raw();
        let remaining_block_size = input.first_page_remaining_block_size.get().raw();
        let page_block_offset = body_block_size
            .checked_sub(remaining_block_size)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        let mut row_cursor = StagingTableRowCursor::start();
        row_cursor.terminal = body_row_count == 0;
        Ok(Self {
            layout,
            limits,
            body_block_size,
            remaining_block_size,
            page_block_offset,
            page_index: 0,
            table_page_ordinal: 0,
            page_open: false,
            planned_take: None,
            consumed_fragment_count: layout.contained_fragment_count(),
            next_fragment_id: 0,
            pages: Vec::new(),
            header_sources: Vec::new(),
            header_drafts: Vec::new(),
            row_fragments: Vec::new(),
            continuation: RowspanContinuationReceipt::empty(0),
            row_cursor,
            body_row_count,
            attempts: TableAttemptGuard::default(),
        })
    }

    fn header_rows(&self) -> Vec<TableRowBandReceipt> {
        self.layout
            .rows()
            .iter()
            .copied()
            .filter(|row| row.section() == TableSection::Head)
            .collect()
    }

    fn header_block_size(&self) -> Result<i64, StagingTablePaginationError> {
        self.header_rows().iter().try_fold(0i64, |total, row| {
            total
                .checked_add(row.block_size().get().raw())
                .ok_or(StagingTablePaginationError::ArithmeticOverflow)
        })
    }

    fn reserve_fragment(&mut self) -> Result<u64, StagingTablePaginationError> {
        if self.consumed_fragment_count >= self.limits.get().max_fragments {
            return Err(StagingTablePaginationError::FragmentLimit);
        }
        self.consumed_fragment_count = self
            .consumed_fragment_count
            .checked_add(1)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        let id = self.next_fragment_id;
        self.next_fragment_id = self
            .next_fragment_id
            .checked_add(1)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        Ok(id)
    }

    fn ensure_fragment_available(&self) -> Result<(), StagingTablePaginationError> {
        if self.consumed_fragment_count >= self.limits.get().max_fragments {
            return Err(StagingTablePaginationError::FragmentLimit);
        }
        Ok(())
    }

    fn advance_page(&mut self) -> Result<(), StagingTablePaginationError> {
        let next = self
            .page_index
            .checked_add(1)
            .ok_or(StagingTablePaginationError::PageLimit)?;
        if next >= self.limits.get().max_pages {
            return Err(StagingTablePaginationError::PageLimit);
        }
        self.page_index = next;
        self.remaining_block_size = self.body_block_size;
        self.page_block_offset = 0;
        self.page_open = false;
        self.planned_take = None;
        Ok(())
    }

    fn evaluate_take(
        &mut self,
        row: TableRowBandReceipt,
        row_block_offset: i64,
        available: i64,
        row_fragment_ordinal: u32,
    ) -> Result<Option<i64>, StagingTablePaginationError> {
        self.attempts.record(TableCandidateAttempt {
            row_owner: row.row_owner(),
            row_fragment_ordinal,
            page_index: self.page_index,
            row_block_offset,
            available_block_size: available,
        })?;
        let remaining = row
            .block_size()
            .get()
            .raw()
            .checked_sub(row_block_offset)
            .ok_or(StagingTablePaginationError::NoProgress(row.row_owner()))?;
        if remaining < 0 || available < 0 {
            return Err(StagingTablePaginationError::NoProgress(row.row_owner()));
        }
        if remaining <= available {
            return Ok(Some(remaining));
        }
        if available == 0 {
            return Ok(None);
        }
        choose_common_table_cut(
            self.layout,
            TableSection::Body,
            row.row_ordinal(),
            row_block_offset,
            available,
        )
    }

    fn open_page_for_body(
        &mut self,
        row: TableRowBandReceipt,
    ) -> Result<(), StagingTablePaginationError> {
        debug_assert!(!self.page_open);
        loop {
            let header_size = self.header_block_size()?;
            if header_size > self.body_block_size {
                return Err(StagingTablePaginationError::HeaderOversize(
                    self.layout.table_owner(),
                ));
            }
            if header_size > self.remaining_block_size {
                if self.remaining_block_size < self.body_block_size {
                    self.advance_page()?;
                    continue;
                }
                return Err(StagingTablePaginationError::HeaderOversize(
                    self.layout.table_owner(),
                ));
            }
            let usable = self
                .remaining_block_size
                .checked_sub(header_size)
                .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
            let take = self.evaluate_take(
                row,
                self.row_cursor.block_offset_within_row,
                usable,
                self.row_cursor.row_fragment_ordinal,
            )?;
            if let Some(take) = take {
                self.place_header_and_open_page()?;
                self.planned_take = Some((
                    row.row_ordinal(),
                    self.row_cursor.block_offset_within_row,
                    self.page_index,
                    take,
                ));
                return Ok(());
            }
            if self.remaining_block_size < self.body_block_size {
                self.advance_page()?;
                continue;
            }
            return Err(self
                .attempts
                .transition_oversize(row.row_owner(), self.page_index));
        }
    }

    fn place_head_only(&mut self) -> Result<(), StagingTablePaginationError> {
        let header_size = self.header_block_size()?;
        if header_size > self.body_block_size {
            return Err(StagingTablePaginationError::HeaderOversize(
                self.layout.table_owner(),
            ));
        }
        if header_size > self.remaining_block_size {
            self.advance_page()?;
        }
        self.place_header_and_open_page()
    }

    fn place_header_and_open_page(&mut self) -> Result<(), StagingTablePaginationError> {
        if self.page_open {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        }
        let header_rows = self.header_rows();
        let repetition_index = (!header_rows.is_empty()).then_some(self.table_page_ordinal);
        let mut header_fragment_ids = Vec::new();
        header_fragment_ids
            .try_reserve_exact(header_rows.len())
            .map_err(|_| StagingTablePaginationError::AllocationFailure)?;
        let mut occurrence_rows = Vec::new();
        occurrence_rows
            .try_reserve_exact(header_rows.len())
            .map_err(|_| StagingTablePaginationError::AllocationFailure)?;
        let mut group_offset = 0i64;
        for row in &header_rows {
            let fragment_id = self.reserve_fragment()?;
            let source_fragment_id = if self.table_page_ordinal == 0 {
                let cells = table_cell_fragments_for_row(
                    self.layout,
                    TableSection::Head,
                    row.row_ordinal(),
                    0,
                    row.block_size().get().raw(),
                )?;
                self.header_sources.push(StagingTableHeaderSourceFragment {
                    source_fragment_id: fragment_id,
                    row_owner: row.row_owner(),
                    row_ordinal: row.row_ordinal(),
                    group_block_offset: group_offset,
                    selected_block_extent: row.block_size().get().raw(),
                    cells,
                });
                fragment_id
            } else {
                self.header_sources
                    .iter()
                    .find(|source| source.row_ordinal == row.row_ordinal())
                    .map(|source| source.source_fragment_id)
                    .ok_or(StagingTablePaginationError::SelectedStateMismatch)?
            };
            header_fragment_ids.push(fragment_id);
            occurrence_rows.push(StagingTableHeaderRowOccurrence {
                fragment_id,
                source_fragment_id,
                row_owner: row.row_owner(),
                target_block_offset: self
                    .page_block_offset
                    .checked_add(group_offset)
                    .ok_or(StagingTablePaginationError::ArithmeticOverflow)?,
            });
            group_offset = group_offset
                .checked_add(row.block_size().get().raw())
                .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        }
        if let Some(repetition_index) = repetition_index {
            self.header_drafts.push(HeaderRepetitionDraft {
                repetition_index,
                target_page_index: self.page_index,
                rows: occurrence_rows,
            });
        }
        self.remaining_block_size = self
            .remaining_block_size
            .checked_sub(group_offset)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        self.page_block_offset = self
            .page_block_offset
            .checked_add(group_offset)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        self.pages.push(StagingTableSelectedPage {
            page_index: self.page_index,
            header_repetition_index: repetition_index,
            header_fragment_ids,
            row_fragment_ids: Vec::new(),
        });
        self.table_page_ordinal = self
            .table_page_ordinal
            .checked_add(1)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        self.page_open = true;
        Ok(())
    }

    fn take_for_open_page(
        &mut self,
        row: TableRowBandReceipt,
    ) -> Result<Option<i64>, StagingTablePaginationError> {
        if let Some((ordinal, offset, page, take)) = self.planned_take.take() {
            if ordinal != row.row_ordinal()
                || offset != self.row_cursor.block_offset_within_row
                || page != self.page_index
            {
                return Err(StagingTablePaginationError::SelectedStateMismatch);
            }
            return Ok(Some(take));
        }
        self.evaluate_take(
            row,
            self.row_cursor.block_offset_within_row,
            self.remaining_block_size,
            self.row_cursor.row_fragment_ordinal,
        )
    }

    fn materialize_body_fragment(
        &mut self,
        row: TableRowBandReceipt,
        take: i64,
    ) -> Result<bool, StagingTablePaginationError> {
        let band_size = row.block_size().get().raw();
        let before = self.row_cursor;
        let remaining = band_size
            .checked_sub(before.block_offset_within_row)
            .ok_or(StagingTablePaginationError::NoProgress(row.row_owner()))?;
        if take < 0 || take > remaining || (take == 0 && remaining != 0) {
            return Err(StagingTablePaginationError::NoProgress(row.row_owner()));
        }
        self.ensure_fragment_available()?;
        let completes_row = take == remaining;
        let next_offset = before
            .block_offset_within_row
            .checked_add(take)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        let cells = table_cell_fragments_for_row(
            self.layout,
            TableSection::Body,
            row.row_ordinal(),
            before.block_offset_within_row,
            take,
        )?;
        if cells.iter().any(|cell| cell.selected_block_extent != take) {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        }
        let expected_before = continuation_for_body_state(
            self.layout,
            row.row_ordinal(),
            before.block_offset_within_row,
            before.row_fragment_ordinal != 0,
        )?;
        validate_continuation(&self.continuation, &expected_before)?;
        let after = if completes_row {
            let next_row = before
                .logical_row_ordinal
                .checked_add(1)
                .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
            StagingTableRowCursor {
                logical_row_ordinal: next_row,
                row_fragment_ordinal: 0,
                block_offset_within_row: 0,
                terminal: next_row == self.body_row_count,
            }
        } else {
            StagingTableRowCursor {
                logical_row_ordinal: before.logical_row_ordinal,
                row_fragment_ordinal: before
                    .row_fragment_ordinal
                    .checked_add(1)
                    .ok_or(StagingTablePaginationError::ArithmeticOverflow)?,
                block_offset_within_row: next_offset,
                terminal: false,
            }
        };
        validate_table_row_cursor_advance(row.row_owner(), before, after)?;
        let continuation_after = if completes_row {
            continuation_for_body_state(self.layout, after.logical_row_ordinal, 0, false)?
        } else {
            continuation_for_body_state(self.layout, row.row_ordinal(), next_offset, true)?
        };
        let fragment_id = self.reserve_fragment()?;
        let receipt = RowFragmentReceipt {
            fragment_id,
            row_owner: row.row_owner(),
            logical_row_ordinal: row.row_ordinal(),
            row_fragment_ordinal: before.row_fragment_ordinal,
            page_index: self.page_index,
            page_block_offset: self.page_block_offset,
            selected_block_extent: take,
            before_cursor: before,
            after_cursor: after,
            cells,
            continuation_before: self.continuation.clone(),
            continuation_after: continuation_after.clone(),
        };
        self.pages
            .last_mut()
            .filter(|page| page.page_index == self.page_index)
            .ok_or(StagingTablePaginationError::SelectedStateMismatch)?
            .row_fragment_ids
            .push(fragment_id);
        self.row_fragments.push(receipt);
        self.remaining_block_size = self
            .remaining_block_size
            .checked_sub(take)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        self.page_block_offset = self
            .page_block_offset
            .checked_add(take)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        self.continuation = continuation_after;
        self.row_cursor = after;
        Ok(completes_row)
    }
}

/// Private MI3-03 pagination entry point. It chooses a single greatest legal
/// cut shared by every active cell, carries only a one-dimensional continuation
/// ledger, and seals headers against their first-occurrence source fragments.
pub fn paginate_staging_table(
    layout: &TableRowBandLayoutReceipt,
    ir: &ProductionFlowIr,
    input: StagingTablePageInput,
    limits: &ValidatedResourceLimits,
) -> Result<SelectedTableLayoutReceipt, StagingTablePaginationError> {
    if layout.flow_registry_fingerprint() != ir.registry().receipt().fingerprint()
        || layout.epoch() != ir.registry().receipt().epoch()
    {
        return Err(StagingTablePaginationError::LayoutRegistryMismatch);
    }
    let mut paginator = StagingTablePaginator::new(layout, input, limits)?;
    let body_rows: Vec<_> = layout
        .rows()
        .iter()
        .copied()
        .filter(|row| row.section() == TableSection::Body)
        .collect();
    if body_rows.is_empty() {
        paginator.place_head_only()?;
    } else {
        for row in body_rows {
            if paginator.row_cursor.logical_row_ordinal != row.row_ordinal() {
                return Err(StagingTablePaginationError::SelectedStateMismatch);
            }
            loop {
                if !paginator.page_open {
                    paginator.open_page_for_body(row)?;
                }
                let Some(take) = paginator.take_for_open_page(row)? else {
                    paginator.advance_page()?;
                    continue;
                };
                let completed = paginator.materialize_body_fragment(row, take)?;
                if completed {
                    break;
                }
                paginator.advance_page()?;
            }
        }
    }
    if !paginator.row_cursor.terminal || !paginator.continuation.entries.is_empty() {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    let page_count = paginator
        .page_index
        .checked_add(1)
        .ok_or(StagingTablePaginationError::PageLimit)?;
    if page_count > limits.get().max_pages {
        return Err(StagingTablePaginationError::PageLimit);
    }
    let multi_flow = complete_table_flow_workers(ir)?;
    let canonical_jcs = encode_staging_table_selected(
        layout,
        &multi_flow,
        input.body_block_size.get().raw(),
        input.first_page_remaining_block_size.get().raw(),
        page_count,
        &paginator.pages,
        &paginator.header_sources,
        &paginator.header_drafts,
        &paginator.row_fragments,
    );
    let fingerprint = table_selected_layout_fingerprint_from_jcs(&canonical_jcs);
    let header_repetitions = paginator
        .header_drafts
        .into_iter()
        .map(|draft| HeaderRepetitionReceipt {
            table_owner: layout.table_owner(),
            selected_state: fingerprint,
            repetition_index: draft.repetition_index,
            target_page_index: draft.target_page_index,
            rows: draft.rows,
        })
        .collect();
    let selected = SelectedTableLayoutReceipt {
        package_sha256: layout.package_sha256(),
        epoch: layout.epoch(),
        flow_registry: layout.flow_registry_fingerprint(),
        grid_sha256: layout.grid_fingerprint().bytes(),
        row_band_sha256: layout.fingerprint(),
        table_owner: layout.table_owner(),
        body_block_size: input.body_block_size.get().raw(),
        first_page_remaining_block_size: input.first_page_remaining_block_size.get().raw(),
        page_count,
        multi_flow,
        pages: paginator.pages,
        header_sources: paginator.header_sources,
        header_repetitions,
        row_fragments: paginator.row_fragments,
        fingerprint,
        canonical_jcs,
    };
    selected.validate_closure(layout, ir)?;
    Ok(selected)
}

fn complete_table_flow_workers(
    ir: &ProductionFlowIr,
) -> Result<MultiFlowSelectedStateReceipt, MultiFlowError> {
    let mut selection = MultiFlowSelectionBuilder::new(ir);
    for flow in ir.registry().flows() {
        let mut worker = FlowWorkerCursor::new(ir, flow.flow_id())?;
        while worker.next_boundary() < flow.terminal().owner_local_ordinal() {
            worker.advance(ir)?;
        }
        selection.register(worker.finish(ir)?)?;
    }
    selection.finish()
}

fn active_table_cells(
    layout: &TableRowBandLayoutReceipt,
    section: TableSection,
    row_ordinal: u32,
) -> impl Iterator<Item = &TableCellLayoutReceipt> {
    layout.cells().iter().filter(move |cell| {
        let end = cell
            .row_ordinal()
            .checked_add(u32::from(cell.rowspan().get()));
        cell.section() == section
            && cell.row_ordinal() <= row_ordinal
            && end.is_some_and(|end| row_ordinal < end)
    })
}

fn cell_vertical_offset_at_row(
    layout: &TableRowBandLayoutReceipt,
    cell: &TableCellLayoutReceipt,
    row_ordinal: u32,
    within_row: i64,
) -> Result<i64, StagingTablePaginationError> {
    if row_ordinal < cell.row_ordinal() || within_row < 0 {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    let mut offset = 0i64;
    for ordinal in cell.row_ordinal()..row_ordinal {
        offset = offset
            .checked_add(
                layout
                    .row(cell.section(), ordinal)
                    .ok_or(StagingTablePaginationError::SelectedStateMismatch)?
                    .block_size()
                    .get()
                    .raw(),
            )
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
    }
    offset
        .checked_add(within_row)
        .ok_or(StagingTablePaginationError::ArithmeticOverflow)
}

fn table_cell_cursor(
    cell: &TableCellLayoutReceipt,
    vertical_offset: i64,
) -> Result<StagingTableCellFlowCursor, StagingTablePaginationError> {
    if vertical_offset < 0 {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    let completed = cell
        .fragment_endpoints()
        .iter()
        .take_while(|endpoint| endpoint.get().raw() <= vertical_offset)
        .count();
    let next_fragment_ordinal =
        u32::try_from(completed).map_err(|_| StagingTablePaginationError::ArithmeticOverflow)?;
    Ok(StagingTableCellFlowCursor {
        flow_id: cell.flow_id(),
        next_fragment_ordinal,
        terminal: vertical_offset >= cell.natural_block_size().get().raw(),
    })
}

fn table_cell_fragments_for_row(
    layout: &TableRowBandLayoutReceipt,
    section: TableSection,
    row_ordinal: u32,
    within_row: i64,
    selected_extent: i64,
) -> Result<Vec<StagingTableCellFragmentReceipt>, StagingTablePaginationError> {
    if within_row < 0 || selected_extent < 0 {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    let active: Vec<_> = active_table_cells(layout, section, row_ordinal).collect();
    if active.is_empty() {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    let mut fragments = Vec::new();
    fragments
        .try_reserve_exact(active.len())
        .map_err(|_| StagingTablePaginationError::AllocationFailure)?;
    for cell in active {
        let vertical_offset_before =
            cell_vertical_offset_at_row(layout, cell, row_ordinal, within_row)?;
        let vertical_offset_after = vertical_offset_before
            .checked_add(selected_extent)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        fragments.push(StagingTableCellFragmentReceipt {
            cell_owner: cell.cell_owner(),
            flow_id: cell.flow_id(),
            selected_block_extent: selected_extent,
            vertical_offset_before,
            vertical_offset_after,
            before_cursor: table_cell_cursor(cell, vertical_offset_before)?,
            after_cursor: table_cell_cursor(cell, vertical_offset_after)?,
        });
    }
    let row_block_size = layout
        .row(section, row_ordinal)
        .ok_or(StagingTablePaginationError::SelectedStateMismatch)?
        .block_size()
        .get()
        .raw();
    let completes_row = within_row
        .checked_add(selected_extent)
        .ok_or(StagingTablePaginationError::ArithmeticOverflow)?
        == row_block_size;
    if completes_row {
        for (fragment, cell) in
            fragments
                .iter()
                .zip(active_table_cells(layout, section, row_ordinal))
        {
            let final_covered_row = cell
                .row_ordinal()
                .checked_add(u32::from(cell.rowspan().get()))
                .ok_or(StagingTablePaginationError::ArithmeticOverflow)?
                == row_ordinal
                    .checked_add(1)
                    .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
            if final_covered_row && !fragment.after_cursor.terminal {
                return Err(StagingTablePaginationError::SelectedStateMismatch);
            }
        }
    }
    Ok(fragments)
}

fn choose_common_table_cut(
    layout: &TableRowBandLayoutReceipt,
    section: TableSection,
    row_ordinal: u32,
    within_row: i64,
    maximum_cut: i64,
) -> Result<Option<i64>, StagingTablePaginationError> {
    if maximum_cut <= 0 {
        return Ok(None);
    }
    let active: Vec<_> = active_table_cells(layout, section, row_ordinal).collect();
    if active.is_empty() {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    let mut candidates = BTreeSet::new();
    candidates.insert(maximum_cut);
    for cell in &active {
        let start = cell_vertical_offset_at_row(layout, cell, row_ordinal, within_row)?;
        for endpoint in cell.fragment_endpoints() {
            let relative = endpoint
                .get()
                .raw()
                .checked_sub(start)
                .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
            if relative > 0 && relative <= maximum_cut {
                candidates.insert(relative);
            }
        }
    }
    for candidate in candidates.into_iter().rev() {
        let legal = active.iter().try_fold(true, |legal, cell| {
            if !legal {
                return Ok(false);
            }
            let start = cell_vertical_offset_at_row(layout, cell, row_ordinal, within_row)?;
            let end = start
                .checked_add(candidate)
                .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
            Ok::<_, StagingTablePaginationError>(
                start >= cell.natural_block_size().get().raw()
                    || end >= cell.natural_block_size().get().raw()
                    || cell
                        .fragment_endpoints()
                        .iter()
                        .any(|endpoint| endpoint.get().raw() == end),
            )
        })?;
        if legal {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn continuation_for_body_state(
    layout: &TableRowBandLayoutReceipt,
    logical_row_ordinal: u32,
    within_row: i64,
    include_current_origins: bool,
) -> Result<RowspanContinuationReceipt, StagingTablePaginationError> {
    let body_count = u32::try_from(
        layout
            .rows()
            .iter()
            .filter(|row| row.section() == TableSection::Body)
            .count(),
    )
    .map_err(|_| StagingTablePaginationError::ArithmeticOverflow)?;
    if logical_row_ordinal >= body_count {
        return Ok(RowspanContinuationReceipt::empty(logical_row_ordinal));
    }
    let mut entries = Vec::new();
    for cell in layout.cells().iter().filter(|cell| {
        if cell.section() != TableSection::Body {
            return false;
        }
        let starts = if include_current_origins {
            cell.row_ordinal() <= logical_row_ordinal
        } else {
            cell.row_ordinal() < logical_row_ordinal
        };
        let end = cell
            .row_ordinal()
            .checked_add(u32::from(cell.rowspan().get()));
        starts && end.is_some_and(|end| logical_row_ordinal < end)
    }) {
        let end = cell
            .row_ordinal()
            .checked_add(u32::from(cell.rowspan().get()))
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        let remaining = end
            .checked_sub(logical_row_ordinal)
            .and_then(|value| u16::try_from(value).ok())
            .and_then(NonZeroU16::new)
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        let vertical_offset =
            cell_vertical_offset_at_row(layout, cell, logical_row_ordinal, within_row)?;
        let cursor = table_cell_cursor(cell, vertical_offset)?;
        let column_end = cell
            .column_ordinal()
            .checked_add(u32::from(cell.colspan().get()))
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        for column_ordinal in cell.column_ordinal()..column_end {
            entries.push(RowspanContinuationEntry {
                column_ordinal,
                cell_owner: cell.cell_owner(),
                flow_id: cell.flow_id(),
                cell_flow_cursor: cursor,
                vertical_offset,
                remaining_logical_rows: remaining,
            });
        }
    }
    entries.sort_by_key(|entry| entry.column_ordinal);
    Ok(RowspanContinuationReceipt {
        logical_row_ordinal,
        entries,
    })
}

fn validate_continuation(
    actual: &RowspanContinuationReceipt,
    expected: &RowspanContinuationReceipt,
) -> Result<(), StagingTablePaginationError> {
    if actual.logical_row_ordinal != expected.logical_row_ordinal {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    if let Some(pair) = actual
        .entries
        .windows(2)
        .find(|pair| pair[0].column_ordinal >= pair[1].column_ordinal)
    {
        return Err(StagingTablePaginationError::DuplicateContinuation {
            column_ordinal: pair[1].column_ordinal,
        });
    }
    for expected_entry in &expected.entries {
        let Some(actual_entry) = actual
            .entries
            .iter()
            .find(|entry| entry.column_ordinal == expected_entry.column_ordinal)
        else {
            return Err(StagingTablePaginationError::MissingContinuation {
                column_ordinal: expected_entry.column_ordinal,
            });
        };
        if actual_entry != expected_entry {
            return Err(StagingTablePaginationError::WrongContinuationOwner {
                column_ordinal: expected_entry.column_ordinal,
            });
        }
    }
    if actual.entries.len() != expected.entries.len() {
        let extra = actual
            .entries
            .iter()
            .find(|entry| {
                !expected
                    .entries
                    .iter()
                    .any(|expected| expected.column_ordinal == entry.column_ordinal)
            })
            .map_or(0, |entry| entry.column_ordinal);
        return Err(StagingTablePaginationError::WrongContinuationOwner {
            column_ordinal: extra,
        });
    }
    Ok(())
}

fn validate_table_row_cursor_advance(
    owner: NodeId,
    before: StagingTableRowCursor,
    after: StagingTableRowCursor,
) -> Result<(), StagingTablePaginationError> {
    let physical_advance = after.logical_row_ordinal == before.logical_row_ordinal
        && before.row_fragment_ordinal.checked_add(1) == Some(after.row_fragment_ordinal)
        && after.block_offset_within_row > before.block_offset_within_row
        && !after.terminal;
    let logical_advance = before.logical_row_ordinal.checked_add(1)
        == Some(after.logical_row_ordinal)
        && after.row_fragment_ordinal == 0
        && after.block_offset_within_row == 0;
    if before.terminal || (!physical_advance && !logical_advance) {
        return Err(StagingTablePaginationError::NoProgress(owner));
    }
    Ok(())
}

fn validate_selected_table_layout(
    selected: &SelectedTableLayoutReceipt,
    layout: &TableRowBandLayoutReceipt,
    ir: &ProductionFlowIr,
) -> Result<(), StagingTablePaginationError> {
    if selected.package_sha256 != layout.package_sha256()
        || selected.epoch != layout.epoch()
        || selected.flow_registry != layout.flow_registry_fingerprint()
        || selected.flow_registry != ir.registry().receipt().fingerprint()
        || selected.grid_sha256 != layout.grid_fingerprint().bytes()
        || selected.row_band_sha256 != layout.fingerprint()
        || selected.table_owner != layout.table_owner()
        || selected.multi_flow.registry_fingerprint() != selected.flow_registry
        || selected.multi_flow.epoch() != selected.epoch
        || selected.body_block_size <= 0
        || selected.first_page_remaining_block_size <= 0
        || selected.first_page_remaining_block_size > selected.body_block_size
    {
        return Err(StagingTablePaginationError::LayoutRegistryMismatch);
    }
    for cell in layout.cells() {
        let Some(terminal) = selected
            .multi_flow
            .terminals()
            .iter()
            .find(|terminal| terminal.flow_id() == cell.flow_id())
        else {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        };
        if terminal.owner_node_id() != cell.cell_owner() {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        }
    }

    let header_rows: Vec<_> = layout
        .rows()
        .iter()
        .copied()
        .filter(|row| row.section() == TableSection::Head)
        .collect();
    if selected.header_sources.len() != header_rows.len() {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    let mut expected_group_offset = 0i64;
    for (source, row) in selected.header_sources.iter().zip(&header_rows) {
        if source.row_owner != row.row_owner()
            || source.row_ordinal != row.row_ordinal()
            || source.group_block_offset != expected_group_offset
            || source.selected_block_extent != row.block_size().get().raw()
            || source.cells
                != table_cell_fragments_for_row(
                    layout,
                    TableSection::Head,
                    row.row_ordinal(),
                    0,
                    row.block_size().get().raw(),
                )?
        {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        }
        expected_group_offset = expected_group_offset
            .checked_add(row.block_size().get().raw())
            .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
    }

    if header_rows.is_empty() && !selected.header_repetitions.is_empty() {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    for (expected_index, repetition) in selected.header_repetitions.iter().enumerate() {
        let expected_index = u32::try_from(expected_index)
            .map_err(|_| StagingTablePaginationError::ArithmeticOverflow)?;
        if repetition.repetition_index != expected_index {
            return Err(StagingTablePaginationError::WrongRepetitionIndex {
                expected: expected_index,
                actual: repetition.repetition_index,
            });
        }
        if repetition.table_owner != selected.table_owner
            || repetition.selected_state != selected.fingerprint
            || repetition.rows.len() != selected.header_sources.len()
        {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        }
        let Some(page) = selected.pages.iter().find(|page| {
            page.page_index == repetition.target_page_index
                && page.header_repetition_index == Some(repetition.repetition_index)
        }) else {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        };
        if page.header_fragment_ids.len() != repetition.rows.len() {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        }
        for ((occurrence, source), page_fragment_id) in repetition
            .rows
            .iter()
            .zip(&selected.header_sources)
            .zip(&page.header_fragment_ids)
        {
            if occurrence.fragment_id != *page_fragment_id
                || occurrence.source_fragment_id != source.source_fragment_id
                || occurrence.row_owner != source.row_owner
            {
                return Err(StagingTablePaginationError::SelectedStateMismatch);
            }
            if repetition.repetition_index == 0
                && occurrence.fragment_id != source.source_fragment_id
            {
                return Err(StagingTablePaginationError::SelectedStateMismatch);
            }
        }
    }

    let body_rows: Vec<_> = layout
        .rows()
        .iter()
        .copied()
        .filter(|row| row.section() == TableSection::Body)
        .collect();
    let body_row_count = u32::try_from(body_rows.len())
        .map_err(|_| StagingTablePaginationError::ArithmeticOverflow)?;
    let mut cursor = StagingTableRowCursor::start();
    cursor.terminal = body_rows.is_empty();
    let mut continuation = RowspanContinuationReceipt::empty(0);
    for fragment in &selected.row_fragments {
        let Some(row) = body_rows
            .get(fragment.logical_row_ordinal as usize)
            .copied()
        else {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        };
        let remaining_row_extent = row
            .block_size()
            .get()
            .raw()
            .checked_sub(cursor.block_offset_within_row)
            .ok_or(StagingTablePaginationError::SelectedStateMismatch)?;
        if fragment.selected_block_extent < 0
            || fragment.selected_block_extent > remaining_row_extent
            || (fragment.selected_block_extent == 0 && remaining_row_extent != 0)
        {
            return Err(StagingTablePaginationError::NoProgress(row.row_owner()));
        }
        let expected_after_cursor = if fragment.selected_block_extent == remaining_row_extent {
            let next_row = cursor
                .logical_row_ordinal
                .checked_add(1)
                .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
            StagingTableRowCursor {
                logical_row_ordinal: next_row,
                row_fragment_ordinal: 0,
                block_offset_within_row: 0,
                terminal: next_row == body_row_count,
            }
        } else {
            StagingTableRowCursor {
                logical_row_ordinal: cursor.logical_row_ordinal,
                row_fragment_ordinal: cursor
                    .row_fragment_ordinal
                    .checked_add(1)
                    .ok_or(StagingTablePaginationError::ArithmeticOverflow)?,
                block_offset_within_row: cursor
                    .block_offset_within_row
                    .checked_add(fragment.selected_block_extent)
                    .ok_or(StagingTablePaginationError::ArithmeticOverflow)?,
                terminal: false,
            }
        };
        validate_table_row_cursor_advance(row.row_owner(), cursor, fragment.after_cursor)?;
        if fragment.row_owner != row.row_owner()
            || fragment.before_cursor != cursor
            || fragment.after_cursor != expected_after_cursor
            || fragment.row_fragment_ordinal != cursor.row_fragment_ordinal
            || fragment.cells
                != table_cell_fragments_for_row(
                    layout,
                    TableSection::Body,
                    row.row_ordinal(),
                    cursor.block_offset_within_row,
                    fragment.selected_block_extent,
                )?
        {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        }
        validate_continuation(&fragment.continuation_before, &continuation)?;
        let expected_after =
            if fragment.after_cursor.logical_row_ordinal > cursor.logical_row_ordinal {
                continuation_for_body_state(
                    layout,
                    fragment.after_cursor.logical_row_ordinal,
                    0,
                    false,
                )?
            } else {
                continuation_for_body_state(
                    layout,
                    cursor.logical_row_ordinal,
                    fragment.after_cursor.block_offset_within_row,
                    true,
                )?
            };
        validate_continuation(&fragment.continuation_after, &expected_after)?;
        continuation = fragment.continuation_after.clone();
        cursor = fragment.after_cursor;
    }
    if !cursor.terminal || !continuation.entries.is_empty() {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }

    let mut materialized_ids = Vec::new();
    materialized_ids.extend(
        selected
            .header_repetitions
            .iter()
            .flat_map(|receipt| receipt.rows.iter().map(|row| row.fragment_id)),
    );
    materialized_ids.extend(
        selected
            .row_fragments
            .iter()
            .map(|fragment| fragment.fragment_id),
    );
    materialized_ids.sort_unstable();
    for (expected, actual) in materialized_ids.iter().enumerate() {
        let expected =
            u64::try_from(expected).map_err(|_| StagingTablePaginationError::ArithmeticOverflow)?;
        if *actual != expected {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        }
    }
    if selected.pages.is_empty()
        || selected
            .pages
            .windows(2)
            .any(|pair| pair[0].page_index.checked_add(1) != Some(pair[1].page_index))
        || match selected.pages.last() {
            Some(page) => page.page_index.checked_add(1) != Some(selected.page_count),
            None => true,
        }
    {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    for page in &selected.pages {
        let mut expected_block_offset = if page.page_index == 0 {
            selected
                .body_block_size
                .checked_sub(selected.first_page_remaining_block_size)
                .ok_or(StagingTablePaginationError::ArithmeticOverflow)?
        } else {
            0
        };
        match page.header_repetition_index {
            Some(index) => {
                let repetition = selected
                    .header_repetitions
                    .iter()
                    .find(|repetition| {
                        repetition.repetition_index == index
                            && repetition.target_page_index == page.page_index
                    })
                    .ok_or(StagingTablePaginationError::SelectedStateMismatch)?;
                for (occurrence, source) in repetition.rows.iter().zip(&selected.header_sources) {
                    if occurrence.target_block_offset != expected_block_offset {
                        return Err(StagingTablePaginationError::SelectedStateMismatch);
                    }
                    expected_block_offset = expected_block_offset
                        .checked_add(source.selected_block_extent)
                        .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
                }
            }
            None if !page.header_fragment_ids.is_empty() || !selected.header_sources.is_empty() => {
                return Err(StagingTablePaginationError::SelectedStateMismatch)
            }
            None => {}
        }
        if !body_rows.is_empty() && page.row_fragment_ids.is_empty() {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        }
        for fragment_id in &page.row_fragment_ids {
            let Some(fragment) = selected.row_fragments.iter().find(|fragment| {
                fragment.fragment_id == *fragment_id && fragment.page_index == page.page_index
            }) else {
                return Err(StagingTablePaginationError::SelectedStateMismatch);
            };
            if fragment.page_block_offset != expected_block_offset {
                return Err(StagingTablePaginationError::SelectedStateMismatch);
            }
            expected_block_offset = expected_block_offset
                .checked_add(fragment.selected_block_extent)
                .ok_or(StagingTablePaginationError::ArithmeticOverflow)?;
        }
        if expected_block_offset > selected.body_block_size {
            return Err(StagingTablePaginationError::SelectedStateMismatch);
        }
    }

    let drafts: Vec<_> = selected
        .header_repetitions
        .iter()
        .map(|receipt| HeaderRepetitionDraft {
            repetition_index: receipt.repetition_index,
            target_page_index: receipt.target_page_index,
            rows: receipt.rows.clone(),
        })
        .collect();
    let canonical_jcs = encode_staging_table_selected(
        layout,
        &selected.multi_flow,
        selected.body_block_size,
        selected.first_page_remaining_block_size,
        selected.page_count,
        &selected.pages,
        &selected.header_sources,
        &drafts,
        &selected.row_fragments,
    );
    if canonical_jcs != selected.canonical_jcs
        || table_selected_layout_fingerprint_from_jcs(&canonical_jcs) != selected.fingerprint
    {
        return Err(StagingTablePaginationError::SelectedStateMismatch);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn encode_staging_table_selected(
    layout: &TableRowBandLayoutReceipt,
    multi_flow: &MultiFlowSelectedStateReceipt,
    body_block_size: i64,
    first_page_remaining_block_size: i64,
    page_count: u32,
    pages: &[StagingTableSelectedPage],
    header_sources: &[StagingTableHeaderSourceFragment],
    header_repetitions: &[HeaderRepetitionDraft],
    row_fragments: &[RowFragmentReceipt],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, TableSelectedLayoutFingerprint::ALGORITHM_ID);
    output.push_str(",\"body_block_size\":");
    output.push_str(&body_block_size.to_string());
    output.push_str(",\"first_page_remaining_block_size\":");
    output.push_str(&first_page_remaining_block_size.to_string());
    output.push_str(",\"flow_registry_sha256\":");
    push_hex(&mut output, layout.flow_registry_fingerprint().bytes());
    output.push_str(",\"grid_sha256\":");
    push_hex(&mut output, layout.grid_fingerprint().bytes());
    output.push_str(",\"header_repetitions\":[");
    for (index, repetition) in header_repetitions.iter().enumerate() {
        comma(&mut output, index);
        encode_table_header_repetition(&mut output, repetition);
    }
    output.push_str("],\"header_sources\":[");
    for (index, source) in header_sources.iter().enumerate() {
        comma(&mut output, index);
        encode_table_header_source(&mut output, source);
    }
    output.push_str("],\"layout_epoch\":");
    encode_layout_epoch(&mut output, layout.epoch());
    output.push_str(",\"multi_flow_selected_sha256\":");
    push_hex(&mut output, multi_flow.fingerprint().bytes());
    output.push_str(",\"package_sha256\":");
    push_hex(&mut output, layout.package_sha256());
    output.push_str(",\"page_count\":");
    output.push_str(&page_count.to_string());
    output.push_str(",\"pages\":[");
    for (index, page) in pages.iter().enumerate() {
        comma(&mut output, index);
        encode_table_page(&mut output, page);
    }
    output.push_str("],\"row_band_sha256\":");
    push_hex(&mut output, layout.fingerprint());
    output.push_str(",\"row_fragments\":[");
    for (index, fragment) in row_fragments.iter().enumerate() {
        comma(&mut output, index);
        encode_table_row_fragment(&mut output, fragment);
    }
    output.push_str("],\"table_node_id\":");
    output.push_str(&layout.table_owner().get().to_string());
    output.push('}');
    output
}

fn encode_table_header_source(output: &mut String, source: &StagingTableHeaderSourceFragment) {
    output.push_str("{\"cells\":[");
    for (index, cell) in source.cells.iter().enumerate() {
        comma(output, index);
        encode_table_cell_fragment(output, cell);
    }
    output.push_str("],\"group_block_offset\":");
    output.push_str(&source.group_block_offset.to_string());
    output.push_str(",\"row_node_id\":");
    output.push_str(&source.row_owner.get().to_string());
    output.push_str(",\"row_ordinal\":");
    output.push_str(&source.row_ordinal.to_string());
    output.push_str(",\"selected_block_extent\":");
    output.push_str(&source.selected_block_extent.to_string());
    output.push_str(",\"source_fragment_id\":");
    output.push_str(&source.source_fragment_id.to_string());
    output.push('}');
}

fn encode_table_header_repetition(output: &mut String, repetition: &HeaderRepetitionDraft) {
    output.push_str("{\"repetition_index\":");
    output.push_str(&repetition.repetition_index.to_string());
    output.push_str(",\"rows\":[");
    for (index, row) in repetition.rows.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"fragment_id\":");
        output.push_str(&row.fragment_id.to_string());
        output.push_str(",\"row_node_id\":");
        output.push_str(&row.row_owner.get().to_string());
        output.push_str(",\"source_fragment_id\":");
        output.push_str(&row.source_fragment_id.to_string());
        output.push_str(",\"target_block_offset\":");
        output.push_str(&row.target_block_offset.to_string());
        output.push('}');
    }
    output.push_str("],\"target_page_index\":");
    output.push_str(&repetition.target_page_index.to_string());
    output.push('}');
}

fn encode_table_page(output: &mut String, page: &StagingTableSelectedPage) {
    output.push_str("{\"header_fragment_ids\":[");
    for (index, fragment_id) in page.header_fragment_ids.iter().enumerate() {
        comma(output, index);
        output.push_str(&fragment_id.to_string());
    }
    output.push_str("],\"header_repetition_index\":");
    match page.header_repetition_index {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
    output.push_str(",\"page_index\":");
    output.push_str(&page.page_index.to_string());
    output.push_str(",\"row_fragment_ids\":[");
    for (index, fragment_id) in page.row_fragment_ids.iter().enumerate() {
        comma(output, index);
        output.push_str(&fragment_id.to_string());
    }
    output.push_str("]}");
}

fn encode_table_row_fragment(output: &mut String, fragment: &RowFragmentReceipt) {
    output.push_str("{\"after_cursor\":");
    encode_table_row_cursor(output, fragment.after_cursor);
    output.push_str(",\"before_cursor\":");
    encode_table_row_cursor(output, fragment.before_cursor);
    output.push_str(",\"cells\":[");
    for (index, cell) in fragment.cells.iter().enumerate() {
        comma(output, index);
        encode_table_cell_fragment(output, cell);
    }
    output.push_str("],\"continuation_after\":");
    encode_rowspan_continuation(output, &fragment.continuation_after);
    output.push_str(",\"continuation_before\":");
    encode_rowspan_continuation(output, &fragment.continuation_before);
    output.push_str(",\"fragment_id\":");
    output.push_str(&fragment.fragment_id.to_string());
    output.push_str(",\"logical_row_ordinal\":");
    output.push_str(&fragment.logical_row_ordinal.to_string());
    output.push_str(",\"page_block_offset\":");
    output.push_str(&fragment.page_block_offset.to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&fragment.page_index.to_string());
    output.push_str(",\"row_fragment_ordinal\":");
    output.push_str(&fragment.row_fragment_ordinal.to_string());
    output.push_str(",\"row_node_id\":");
    output.push_str(&fragment.row_owner.get().to_string());
    output.push_str(",\"selected_block_extent\":");
    output.push_str(&fragment.selected_block_extent.to_string());
    output.push('}');
}

fn encode_table_cell_fragment(output: &mut String, cell: &StagingTableCellFragmentReceipt) {
    output.push_str("{\"after_cursor\":");
    encode_table_cell_cursor(output, cell.after_cursor);
    output.push_str(",\"before_cursor\":");
    encode_table_cell_cursor(output, cell.before_cursor);
    output.push_str(",\"cell_node_id\":");
    output.push_str(&cell.cell_owner.get().to_string());
    output.push_str(",\"flow_id\":");
    output.push_str(&cell.flow_id.get().to_string());
    output.push_str(",\"selected_block_extent\":");
    output.push_str(&cell.selected_block_extent.to_string());
    output.push_str(",\"vertical_offset_after\":");
    output.push_str(&cell.vertical_offset_after.to_string());
    output.push_str(",\"vertical_offset_before\":");
    output.push_str(&cell.vertical_offset_before.to_string());
    output.push('}');
}

fn encode_table_cell_cursor(output: &mut String, cursor: StagingTableCellFlowCursor) {
    output.push_str("{\"flow_id\":");
    output.push_str(&cursor.flow_id.get().to_string());
    output.push_str(",\"next_fragment_ordinal\":");
    output.push_str(&cursor.next_fragment_ordinal.to_string());
    output.push_str(",\"terminal\":");
    output.push_str(if cursor.terminal { "true" } else { "false" });
    output.push('}');
}

fn encode_table_row_cursor(output: &mut String, cursor: StagingTableRowCursor) {
    output.push_str("{\"block_offset_within_row\":");
    output.push_str(&cursor.block_offset_within_row.to_string());
    output.push_str(",\"logical_row_ordinal\":");
    output.push_str(&cursor.logical_row_ordinal.to_string());
    output.push_str(",\"row_fragment_ordinal\":");
    output.push_str(&cursor.row_fragment_ordinal.to_string());
    output.push_str(",\"terminal\":");
    output.push_str(if cursor.terminal { "true" } else { "false" });
    output.push('}');
}

fn encode_rowspan_continuation(output: &mut String, value: &RowspanContinuationReceipt) {
    output.push_str("{\"entries\":[");
    for (index, entry) in value.entries.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"cell_flow_cursor\":");
        encode_table_cell_cursor(output, entry.cell_flow_cursor);
        output.push_str(",\"cell_node_id\":");
        output.push_str(&entry.cell_owner.get().to_string());
        output.push_str(",\"column_ordinal\":");
        output.push_str(&entry.column_ordinal.to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&entry.flow_id.get().to_string());
        output.push_str(",\"remaining_logical_rows\":");
        output.push_str(&entry.remaining_logical_rows.get().to_string());
        output.push_str(",\"vertical_offset\":");
        output.push_str(&entry.vertical_offset.to_string());
        output.push('}');
    }
    output.push_str("],\"logical_row_ordinal\":");
    output.push_str(&value.logical_row_ordinal.to_string());
    output.push('}');
}

pub const STAGING_MACHINE_LIST_TRACE_ALGORITHM: &str = "typaxis.machine-list-trace/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMachineListPaginationError {
    Flow(MultiFlowError),
    LayoutRegistryMismatch,
    InvalidPageInput,
    OversizeKeep(NodeId),
    PageLimit,
    FragmentLimit,
    NoProgress(NodeId),
    MarkerOrphan(NodeId),
    ArithmeticOverflow,
    AllocationFailure,
}

impl From<MultiFlowError> for StagingMachineListPaginationError {
    fn from(value: MultiFlowError) -> Self {
        Self::Flow(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingMachineListPageInput {
    body_block_size: PositiveLength,
    first_page_remaining_block_size: PositiveLength,
}

impl StagingMachineListPageInput {
    pub fn new(
        body_block_size: PositiveLength,
        first_page_remaining_block_size: PositiveLength,
    ) -> Result<Self, StagingMachineListPaginationError> {
        if first_page_remaining_block_size.get().raw() > body_block_size.get().raw() {
            return Err(StagingMachineListPaginationError::InvalidPageInput);
        }
        Ok(Self {
            body_block_size,
            first_page_remaining_block_size,
        })
    }

    pub const fn body_block_size(self) -> PositiveLength {
        self.body_block_size
    }

    pub const fn first_page_remaining_block_size(self) -> PositiveLength {
        self.first_page_remaining_block_size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListSelectedList {
    list_owner: NodeId,
    list_flow_id: FlowId,
    marker_column_width: i64,
    marker_gap: i64,
    start_indent: i64,
    end_indent: i64,
    item_frame_inline_size: i64,
}

impl StagingMachineListSelectedList {
    pub const fn list_owner(&self) -> NodeId {
        self.list_owner
    }
    pub const fn list_flow_id(&self) -> FlowId {
        self.list_flow_id
    }
    pub const fn marker_column_width(&self) -> i64 {
        self.marker_column_width
    }
    pub const fn marker_gap(&self) -> i64 {
        self.marker_gap
    }
    pub const fn start_indent(&self) -> i64 {
        self.start_indent
    }
    pub const fn end_indent(&self) -> i64 {
        self.end_indent
    }
    pub const fn item_frame_inline_size(&self) -> i64 {
        self.item_frame_inline_size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListFragment {
    fragment_id: u64,
    item_owner: NodeId,
    item_flow_id: FlowId,
    page_index: u32,
    block_offset: i64,
    block_size: i64,
    contains_marker: bool,
    contains_first_painted_line: bool,
}

impl StagingMachineListFragment {
    pub const fn fragment_id(&self) -> u64 {
        self.fragment_id
    }
    pub const fn item_owner(&self) -> NodeId {
        self.item_owner
    }
    pub const fn item_flow_id(&self) -> FlowId {
        self.item_flow_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn block_offset(&self) -> i64 {
        self.block_offset
    }
    pub const fn block_size(&self) -> i64 {
        self.block_size
    }
    pub const fn contains_marker(&self) -> bool {
        self.contains_marker
    }
    pub const fn contains_first_painted_line(&self) -> bool {
        self.contains_first_painted_line
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListSelectedItem {
    list_owner: NodeId,
    item_owner: NodeId,
    item_index: u32,
    list_flow_id: FlowId,
    item_flow_id: FlowId,
    marker_key: GeneratedBufferKey,
    marker_utf8: String,
    marker_fragment_id: u64,
    first_line_fragment_id: u64,
    page_index: u32,
    fragment_ids: Vec<u64>,
    marker_inline_size: i64,
    marker_column_width: i64,
    marker_physical_left: i64,
    content_physical_left: i64,
    content_inline_size: i64,
    first_line_inline_size: i64,
    first_line_block_size: i64,
    block_offset: i64,
}

impl StagingMachineListSelectedItem {
    pub const fn list_owner(&self) -> NodeId {
        self.list_owner
    }
    pub const fn item_owner(&self) -> NodeId {
        self.item_owner
    }
    pub const fn item_index(&self) -> u32 {
        self.item_index
    }
    pub const fn list_flow_id(&self) -> FlowId {
        self.list_flow_id
    }
    pub const fn item_flow_id(&self) -> FlowId {
        self.item_flow_id
    }
    pub const fn marker_key(&self) -> GeneratedBufferKey {
        self.marker_key
    }
    pub fn marker_utf8(&self) -> &str {
        &self.marker_utf8
    }
    pub const fn marker_fragment_id(&self) -> u64 {
        self.marker_fragment_id
    }
    pub const fn first_line_fragment_id(&self) -> u64 {
        self.first_line_fragment_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn fragment_ids(&self) -> &[u64] {
        &self.fragment_ids
    }
    pub const fn marker_inline_size(&self) -> i64 {
        self.marker_inline_size
    }
    pub const fn marker_column_width(&self) -> i64 {
        self.marker_column_width
    }
    pub const fn marker_physical_left(&self) -> i64 {
        self.marker_physical_left
    }
    pub const fn content_physical_left(&self) -> i64 {
        self.content_physical_left
    }
    pub const fn content_inline_size(&self) -> i64 {
        self.content_inline_size
    }
    pub const fn first_line_inline_size(&self) -> i64 {
        self.first_line_inline_size
    }
    pub const fn first_line_block_size(&self) -> i64 {
        self.first_line_block_size
    }
    pub const fn block_offset(&self) -> i64 {
        self.block_offset
    }
}

/// Selected list state. The all-flow terminal receipt and marker keep groups
/// are sealed together so a body-only selection cannot reach Display.
#[derive(Debug)]
pub struct StagingMachineListSelectedState {
    package_sha256: [u8; 32],
    epoch: LayoutEpoch,
    flow_registry: FlowRegistryFingerprint,
    marker_usage_sha256: [u8; 32],
    policy_version: &'static str,
    page_count: u32,
    multi_flow: MultiFlowSelectedStateReceipt,
    lists: Vec<StagingMachineListSelectedList>,
    items: Vec<StagingMachineListSelectedItem>,
    fragments: Vec<StagingMachineListFragment>,
}

impl StagingMachineListSelectedState {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn flow_registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.flow_registry
    }
    pub const fn marker_usage_sha256(&self) -> [u8; 32] {
        self.marker_usage_sha256
    }
    pub const fn policy_version(&self) -> &'static str {
        self.policy_version
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn multi_flow(&self) -> &MultiFlowSelectedStateReceipt {
        &self.multi_flow
    }
    pub fn lists(&self) -> &[StagingMachineListSelectedList] {
        &self.lists
    }
    pub fn items(&self) -> &[StagingMachineListSelectedItem] {
        &self.items
    }
    pub fn fragments(&self) -> &[StagingMachineListFragment] {
        &self.fragments
    }

    pub fn validate_marker_closure(&self) -> Result<(), StagingMachineListPaginationError> {
        for item in &self.items {
            if item.marker_fragment_id != item.first_line_fragment_id
                || item.fragment_ids.first().copied() != Some(item.marker_fragment_id)
            {
                return Err(StagingMachineListPaginationError::MarkerOrphan(
                    item.item_owner,
                ));
            }
            let Some(fragment) = self.fragments.iter().find(|fragment| {
                fragment.fragment_id == item.marker_fragment_id
                    && fragment.item_owner == item.item_owner
                    && fragment.item_flow_id == item.item_flow_id
            }) else {
                return Err(StagingMachineListPaginationError::MarkerOrphan(
                    item.item_owner,
                ));
            };
            if !fragment.contains_marker
                || !fragment.contains_first_painted_line
                || fragment.page_index != item.page_index
            {
                return Err(StagingMachineListPaginationError::MarkerOrphan(
                    item.item_owner,
                ));
            }
        }
        Ok(())
    }

    pub fn trace_facts(&self) -> StagingMachineListTraceFacts {
        StagingMachineListTraceFacts::from_selected(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListTraceItem {
    item_owner: u32,
    list_flow_id: u32,
    item_flow_id: u32,
    page_index: u32,
    marker_fragment_id: u64,
}

impl StagingMachineListTraceItem {
    pub const fn item_owner(&self) -> u32 {
        self.item_owner
    }
    pub const fn list_flow_id(&self) -> u32 {
        self.list_flow_id
    }
    pub const fn item_flow_id(&self) -> u32 {
        self.item_flow_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn marker_fragment_id(&self) -> u64 {
        self.marker_fragment_id
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct StagingMachineListTraceFacts {
    flow_registry_sha256: [u8; 32],
    marker_usage_sha256: [u8; 32],
    items: Vec<StagingMachineListTraceItem>,
    canonical_jcs: String,
}

impl StagingMachineListTraceFacts {
    fn from_selected(selected: &StagingMachineListSelectedState) -> Self {
        let items: Vec<_> = selected
            .items
            .iter()
            .map(|item| StagingMachineListTraceItem {
                item_owner: item.item_owner.get(),
                list_flow_id: item.list_flow_id.get(),
                item_flow_id: item.item_flow_id.get(),
                page_index: item.page_index,
                marker_fragment_id: item.marker_fragment_id,
            })
            .collect();
        let canonical_jcs = encode_staging_machine_list_trace(
            selected.flow_registry.bytes(),
            selected.marker_usage_sha256,
            &items,
        );
        Self {
            flow_registry_sha256: selected.flow_registry.bytes(),
            marker_usage_sha256: selected.marker_usage_sha256,
            items,
            canonical_jcs,
        }
    }

    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }
    pub const fn marker_usage_sha256(&self) -> [u8; 32] {
        self.marker_usage_sha256
    }
    pub fn items(&self) -> &[StagingMachineListTraceItem] {
        &self.items
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

pub fn paginate_staging_machine_lists(
    layout: &StagingMachineListLayoutReceipt,
    ir: &ProductionFlowIr,
    input: StagingMachineListPageInput,
    limits: &ValidatedResourceLimits,
) -> Result<StagingMachineListSelectedState, StagingMachineListPaginationError> {
    if layout.flow_registry_fingerprint() != ir.registry().receipt().fingerprint()
        || layout.epoch() != ir.registry().receipt().epoch()
    {
        return Err(StagingMachineListPaginationError::LayoutRegistryMismatch);
    }
    let multi_flow = complete_staging_list_flow_stack(ir)?;
    let mut lists = Vec::new();
    lists
        .try_reserve_exact(layout.lists().len())
        .map_err(|_| StagingMachineListPaginationError::AllocationFailure)?;
    for list in layout.lists() {
        lists.push(StagingMachineListSelectedList {
            list_owner: list.list_owner(),
            list_flow_id: list.list_flow_id(),
            marker_column_width: list.marker_column_width().get().raw(),
            marker_gap: list.marker_gap().get().raw(),
            start_indent: list.start_indent().get().raw(),
            end_indent: list.end_indent().get().raw(),
            item_frame_inline_size: list.item_frame_inline_size().get().raw(),
        });
    }
    let body_raw = input.body_block_size.get().raw();
    let mut remaining_page_raw = input.first_page_remaining_block_size.get().raw();
    let mut block_offset_raw = body_raw
        .checked_sub(remaining_page_raw)
        .ok_or(StagingMachineListPaginationError::ArithmeticOverflow)?;
    let mut page_index = 0u32;
    let mut next_fragment_id = 0u64;
    let mut selected_items = Vec::new();
    selected_items
        .try_reserve_exact(layout.items().len())
        .map_err(|_| StagingMachineListPaginationError::AllocationFailure)?;
    let mut fragments = Vec::new();

    for (item_ordinal, item) in layout.items().iter().enumerate() {
        let keep_raw = item.keep_group_block_size().get().raw();
        if keep_raw > body_raw {
            return Err(StagingMachineListPaginationError::OversizeKeep(
                item.item_owner(),
            ));
        }
        if keep_raw > remaining_page_raw {
            let before = ListProgress {
                item_ordinal,
                remaining_item_raw: item.painted_block_size().get().raw(),
                page_index,
            };
            page_index = advance_list_page(page_index, limits)?;
            remaining_page_raw = body_raw;
            block_offset_raw = 0;
            ensure_list_progress(
                item.item_owner(),
                before,
                ListProgress {
                    page_index,
                    ..before
                },
            )?;
        }

        let first_page_index = page_index;
        let first_block_offset = block_offset_raw;
        let mut remaining_item_raw = item.painted_block_size().get().raw();
        let mut fragment_ids = Vec::new();
        while remaining_item_raw > 0 {
            if remaining_page_raw == 0 {
                let before = ListProgress {
                    item_ordinal,
                    remaining_item_raw,
                    page_index,
                };
                page_index = advance_list_page(page_index, limits)?;
                remaining_page_raw = body_raw;
                block_offset_raw = 0;
                ensure_list_progress(
                    item.item_owner(),
                    before,
                    ListProgress {
                        page_index,
                        ..before
                    },
                )?;
            }
            let before = ListProgress {
                item_ordinal,
                remaining_item_raw,
                page_index,
            };
            let take_raw = remaining_item_raw.min(remaining_page_raw);
            if take_raw <= 0 {
                return Err(StagingMachineListPaginationError::NoProgress(
                    item.item_owner(),
                ));
            }
            if next_fragment_id >= limits.get().max_fragments {
                return Err(StagingMachineListPaginationError::FragmentLimit);
            }
            let first_fragment = fragment_ids.is_empty();
            fragment_ids.push(next_fragment_id);
            fragments
                .try_reserve(1)
                .map_err(|_| StagingMachineListPaginationError::AllocationFailure)?;
            fragments.push(StagingMachineListFragment {
                fragment_id: next_fragment_id,
                item_owner: item.item_owner(),
                item_flow_id: item.item_flow_id(),
                page_index,
                block_offset: block_offset_raw,
                block_size: take_raw,
                contains_marker: first_fragment,
                contains_first_painted_line: first_fragment,
            });
            next_fragment_id = next_fragment_id
                .checked_add(1)
                .ok_or(StagingMachineListPaginationError::ArithmeticOverflow)?;
            remaining_item_raw -= take_raw;
            remaining_page_raw -= take_raw;
            block_offset_raw = block_offset_raw
                .checked_add(take_raw)
                .ok_or(StagingMachineListPaginationError::ArithmeticOverflow)?;
            ensure_list_progress(
                item.item_owner(),
                before,
                ListProgress {
                    item_ordinal,
                    remaining_item_raw,
                    page_index,
                },
            )?;
        }
        let marker_fragment_id =
            *fragment_ids
                .first()
                .ok_or(StagingMachineListPaginationError::NoProgress(
                    item.item_owner(),
                ))?;
        selected_items.push(StagingMachineListSelectedItem {
            list_owner: item.list_owner(),
            item_owner: item.item_owner(),
            item_index: item.item_index(),
            list_flow_id: item.list_flow_id(),
            item_flow_id: item.item_flow_id(),
            marker_key: item.marker_key(),
            marker_utf8: item.marker_utf8().to_owned(),
            marker_fragment_id,
            first_line_fragment_id: marker_fragment_id,
            page_index: first_page_index,
            fragment_ids,
            marker_inline_size: item.marker_inline_size().get().raw(),
            marker_column_width: item.marker_column_width().get().raw(),
            marker_physical_left: item.marker_physical_left().get().raw(),
            content_physical_left: item.content_physical_left().get().raw(),
            content_inline_size: item.content_inline_size().get().raw(),
            first_line_inline_size: item.first_line_inline_size().get().raw(),
            first_line_block_size: item.first_line_block_size().get().raw(),
            block_offset: first_block_offset,
        });
    }
    let page_count = page_index
        .checked_add(1)
        .ok_or(StagingMachineListPaginationError::PageLimit)?;
    if page_count > limits.get().max_pages {
        return Err(StagingMachineListPaginationError::PageLimit);
    }
    let selected = StagingMachineListSelectedState {
        package_sha256: layout.package_sha256(),
        epoch: layout.epoch(),
        flow_registry: layout.flow_registry_fingerprint(),
        marker_usage_sha256: layout.marker_usage_sha256(),
        policy_version: layout.policy_version(),
        page_count,
        multi_flow,
        lists,
        items: selected_items,
        fragments,
    };
    selected.validate_marker_closure()?;
    Ok(selected)
}

fn complete_staging_list_flow_stack(
    ir: &ProductionFlowIr,
) -> Result<MultiFlowSelectedStateReceipt, MultiFlowError> {
    let mut cursor = MultiFlowCursorReceipt::new(ir)?;
    loop {
        let position = cursor.current_position(ir)?;
        if position.is_terminal() {
            if cursor.current_flow() == FlowId::DOCUMENT_BODY {
                break;
            }
            cursor.leave_terminal(ir)?;
            cursor.advance(ir)?;
        } else if let Some(child) = position.child_flow_id() {
            cursor.enter_child(ir, child)?;
        } else {
            cursor.advance(ir)?;
        }
    }
    cursor.finish(ir)
}

fn advance_list_page(
    current: u32,
    limits: &ValidatedResourceLimits,
) -> Result<u32, StagingMachineListPaginationError> {
    let next = current
        .checked_add(1)
        .ok_or(StagingMachineListPaginationError::PageLimit)?;
    if next >= limits.get().max_pages {
        return Err(StagingMachineListPaginationError::PageLimit);
    }
    Ok(next)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ListProgress {
    item_ordinal: usize,
    remaining_item_raw: i64,
    page_index: u32,
}

fn ensure_list_progress(
    owner: NodeId,
    before: ListProgress,
    after: ListProgress,
) -> Result<(), StagingMachineListPaginationError> {
    if before == after
        || after.item_ordinal < before.item_ordinal
        || (after.item_ordinal == before.item_ordinal
            && after.page_index == before.page_index
            && after.remaining_item_raw >= before.remaining_item_raw)
    {
        return Err(StagingMachineListPaginationError::NoProgress(owner));
    }
    Ok(())
}

fn encode_staging_machine_list_trace(
    flow_registry_sha256: [u8; 32],
    marker_usage_sha256: [u8; 32],
    items: &[StagingMachineListTraceItem],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_LIST_TRACE_ALGORITHM);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"flow_registry_sha256\":");
    push_hex(&mut output, flow_registry_sha256);
    output.push_str(",\"items\":[");
    for (index, item) in items.iter().enumerate() {
        comma(&mut output, index);
        output.push_str("{\"item_flow_id\":");
        output.push_str(&item.item_flow_id.to_string());
        output.push_str(",\"item_node_id\":");
        output.push_str(&item.item_owner.to_string());
        output.push_str(",\"list_flow_id\":");
        output.push_str(&item.list_flow_id.to_string());
        output.push_str(",\"marker_fragment_id\":");
        output.push_str(&item.marker_fragment_id.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&item.page_index.to_string());
        output.push('}');
    }
    output.push_str("],\"marker_usage_sha256\":");
    push_hex(&mut output, marker_usage_sha256);
    output.push('}');
    output
}

pub const STAGING_MACHINE_FIGURE_SELECTED_ALGORITHM: &str = "typaxis.machine-figure-selected/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingFigureCaptionBlockInput {
    owner: NodeId,
    block_size: PositiveLength,
}

impl StagingFigureCaptionBlockInput {
    pub const fn new(owner: NodeId, block_size: PositiveLength) -> Self {
        Self { owner, block_size }
    }

    pub const fn owner(self) -> NodeId {
        self.owner
    }

    pub const fn block_size(self) -> PositiveLength {
        self.block_size
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMachineFigurePaginationError {
    LayoutRegistryMismatch,
    MissingCaptionMeasurement(NodeId),
    ExtraCaptionMeasurement(NodeId),
    DuplicateCaptionMeasurement(NodeId),
    InitialContentExceedsBody,
    ImageOversize(NodeId),
    CaptionOversize(NodeId),
    KeepOversize(NodeId),
    NoProgress(NodeId),
    PageLimit,
    ArithmeticOverflow,
    AllocationFailure,
}

impl StagingMachineFigurePaginationError {
    pub const fn invariant_diagnostic_code(self) -> Option<DiagnosticCode> {
        match self {
            Self::NoProgress(_) => Some(I9190),
            Self::LayoutRegistryMismatch
            | Self::MissingCaptionMeasurement(_)
            | Self::ExtraCaptionMeasurement(_)
            | Self::DuplicateCaptionMeasurement(_)
            | Self::InitialContentExceedsBody
            | Self::ImageOversize(_)
            | Self::CaptionOversize(_)
            | Self::KeepOversize(_)
            | Self::PageLimit
            | Self::ArithmeticOverflow
            | Self::AllocationFailure => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigurePaginationInput {
    initial_consumed_block_size: NonNegativeLength,
    captions: Vec<StagingFigureCaptionBlockInput>,
}

impl StagingMachineFigurePaginationInput {
    pub fn new(
        layout: &ValidatedFigureLayout,
        initial_consumed_block_size: NonNegativeLength,
        mut captions: Vec<StagingFigureCaptionBlockInput>,
    ) -> Result<Self, StagingMachineFigurePaginationError> {
        if initial_consumed_block_size.get().raw() > layout.body().height().get().raw() {
            return Err(StagingMachineFigurePaginationError::InitialContentExceedsBody);
        }
        captions.sort_by_key(|caption| caption.owner);
        if let Some(pair) = captions
            .windows(2)
            .find(|pair| pair[0].owner == pair[1].owner)
        {
            return Err(
                StagingMachineFigurePaginationError::DuplicateCaptionMeasurement(pair[1].owner),
            );
        }
        let mut expected: Vec<_> = layout
            .figures()
            .iter()
            .flat_map(|figure| figure.caption_owners().iter().copied())
            .collect();
        expected.sort_unstable();
        for owner in &expected {
            if captions
                .binary_search_by_key(owner, |caption| caption.owner)
                .is_err()
            {
                return Err(StagingMachineFigurePaginationError::MissingCaptionMeasurement(*owner));
            }
        }
        if let Some(extra) = captions
            .iter()
            .find(|caption| expected.binary_search(&caption.owner).is_err())
        {
            return Err(StagingMachineFigurePaginationError::ExtraCaptionMeasurement(extra.owner));
        }
        Ok(Self {
            initial_consumed_block_size,
            captions,
        })
    }

    pub const fn initial_consumed_block_size(&self) -> NonNegativeLength {
        self.initial_consumed_block_size
    }

    pub fn captions(&self) -> &[StagingFigureCaptionBlockInput] {
        &self.captions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigureCaptionFragment {
    caption_owner: NodeId,
    caption_flow_id: FlowId,
    page_index: u32,
    rect: Rect,
}

impl StagingMachineFigureCaptionFragment {
    pub const fn caption_owner(&self) -> NodeId {
        self.caption_owner
    }
    pub const fn caption_flow_id(&self) -> FlowId {
        self.caption_flow_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn rect(&self) -> Rect {
        self.rect
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigurePlacement {
    figure_owner: NodeId,
    document_ordinal: u32,
    figure_flow_id: FlowId,
    caption_flow_id: FlowId,
    image_id: ImageResourceId,
    alt: String,
    admitted_media_kind: &'static str,
    admitted_sha256: [u8; 32],
    admitted_byte_length: u64,
    pixel_width: u32,
    pixel_height: u32,
    decoded_bytes: u64,
    page_index: u32,
    rect: Rect,
    effective_space_before: NonNegativeLength,
    keep_policy: StagingFigureKeepPolicy,
    oversize_policy: StagingFigureOversizePolicy,
    moved_to_fresh_page: bool,
    caption_fragments: Vec<StagingMachineFigureCaptionFragment>,
}

impl StagingMachineFigurePlacement {
    pub const fn figure_owner(&self) -> NodeId {
        self.figure_owner
    }
    pub const fn document_ordinal(&self) -> u32 {
        self.document_ordinal
    }
    pub const fn figure_flow_id(&self) -> FlowId {
        self.figure_flow_id
    }
    pub const fn caption_flow_id(&self) -> FlowId {
        self.caption_flow_id
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub fn alt(&self) -> &str {
        &self.alt
    }
    pub const fn admitted_media_kind(&self) -> &'static str {
        self.admitted_media_kind
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn admitted_byte_length(&self) -> u64 {
        self.admitted_byte_length
    }
    pub const fn pixel_width(&self) -> u32 {
        self.pixel_width
    }
    pub const fn pixel_height(&self) -> u32 {
        self.pixel_height
    }
    pub const fn decoded_bytes(&self) -> u64 {
        self.decoded_bytes
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn rect(&self) -> Rect {
        self.rect
    }
    pub const fn effective_space_before(&self) -> NonNegativeLength {
        self.effective_space_before
    }
    pub const fn keep_policy(&self) -> StagingFigureKeepPolicy {
        self.keep_policy
    }
    pub const fn oversize_policy(&self) -> StagingFigureOversizePolicy {
        self.oversize_policy
    }
    pub const fn moved_to_fresh_page(&self) -> bool {
        self.moved_to_fresh_page
    }
    pub fn caption_fragments(&self) -> &[StagingMachineFigureCaptionFragment] {
        &self.caption_fragments
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigureSelectedPage {
    page_index: u32,
    figure_count: u32,
    caption_block_count: u32,
}

impl StagingMachineFigureSelectedPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn figure_count(&self) -> u32 {
        self.figure_count
    }
    pub const fn caption_block_count(&self) -> u32 {
        self.caption_block_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineFigureSelectedState {
    package_sha256: [u8; 32],
    epoch: LayoutEpoch,
    flow_registry: FlowRegistryFingerprint,
    figure_usage_sha256: [u8; 32],
    policy_version: &'static str,
    master_id: MasterId,
    page_width: PositiveLength,
    page_height: PositiveLength,
    body: Rect,
    initial_consumed_block_size: NonNegativeLength,
    pages: Vec<StagingMachineFigureSelectedPage>,
    figures: Vec<StagingMachineFigurePlacement>,
    state_fingerprint: LayoutStateFingerprint,
    canonical_jcs: String,
}

impl StagingMachineFigureSelectedState {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn flow_registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.flow_registry
    }
    pub const fn figure_usage_sha256(&self) -> [u8; 32] {
        self.figure_usage_sha256
    }
    pub const fn policy_version(&self) -> &'static str {
        self.policy_version
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn page_width(&self) -> PositiveLength {
        self.page_width
    }
    pub const fn page_height(&self) -> PositiveLength {
        self.page_height
    }
    pub const fn body(&self) -> Rect {
        self.body
    }
    pub const fn initial_consumed_block_size(&self) -> NonNegativeLength {
        self.initial_consumed_block_size
    }
    pub fn pages(&self) -> &[StagingMachineFigureSelectedPage] {
        &self.pages
    }
    pub fn figures(&self) -> &[StagingMachineFigurePlacement] {
        &self.figures
    }
    pub const fn state_fingerprint(&self) -> LayoutStateFingerprint {
        self.state_fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

pub fn paginate_staging_machine_figures(
    layout: &ValidatedFigureLayout,
    ir: &ProductionFlowIr,
    input: &StagingMachineFigurePaginationInput,
    limits: &ValidatedResourceLimits,
) -> Result<StagingMachineFigureSelectedState, StagingMachineFigurePaginationError> {
    if layout.flow_registry_fingerprint() != ir.registry().receipt().fingerprint()
        || layout.epoch() != ir.registry().receipt().epoch()
    {
        return Err(StagingMachineFigurePaginationError::LayoutRegistryMismatch);
    }
    if limits.get().max_pages == 0 {
        return Err(StagingMachineFigurePaginationError::PageLimit);
    }
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(layout.figures().len().saturating_add(1))
        .map_err(|_| StagingMachineFigurePaginationError::AllocationFailure)?;
    pages.push(StagingMachineFigureSelectedPage {
        page_index: 0,
        figure_count: 0,
        caption_block_count: 0,
    });
    let mut placements = Vec::new();
    placements
        .try_reserve_exact(layout.figures().len())
        .map_err(|_| StagingMachineFigurePaginationError::AllocationFailure)?;
    let body_height = layout.body().height().get().raw();
    let mut used = input.initial_consumed_block_size.get().raw();
    let mut pending_space_after = 0i64;

    for figure in layout.figures() {
        let caption_inputs: Vec<_> = figure
            .caption_owners()
            .iter()
            .map(|owner| {
                input
                    .captions
                    .binary_search_by_key(owner, |caption| caption.owner)
                    .map(|index| input.captions[index])
                    .map_err(|_| {
                        StagingMachineFigurePaginationError::MissingCaptionMeasurement(*owner)
                    })
            })
            .collect::<Result<_, _>>()?;
        let caption_total = caption_inputs.iter().try_fold(0i64, |total, caption| {
            total
                .checked_add(caption.block_size.get().raw())
                .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)
        })?;
        let image_height = figure.block_size().get().raw();
        if image_height > body_height {
            return Err(StagingMachineFigurePaginationError::ImageOversize(
                figure.figure_owner(),
            ));
        }
        let requested_space_before = if used == 0 {
            0
        } else {
            pending_space_after
                .checked_add(figure.space_before().get().raw())
                .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)?
        };
        let mut effective_space_before = requested_space_before;
        let mut moved_to_fresh_page = false;
        let mut caption_fragments = Vec::new();
        caption_fragments
            .try_reserve_exact(caption_inputs.len())
            .map_err(|_| StagingMachineFigurePaginationError::AllocationFailure)?;

        if figure.keep_policy() == StagingFigureKeepPolicy::KeepImageAndCaption {
            let kept_height = image_height
                .checked_add(caption_total)
                .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)?;
            if kept_height > body_height {
                return Err(StagingMachineFigurePaginationError::KeepOversize(
                    figure.figure_owner(),
                ));
            }
            let requested = effective_space_before
                .checked_add(kept_height)
                .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)?;
            if requested > body_height - used {
                add_staging_figure_page(&mut pages, limits)?;
                used = 0;
                effective_space_before = 0;
                moved_to_fresh_page = true;
            }
            let image_y = checked_figure_y(layout.body(), used, effective_space_before)?;
            let page_index = u32::try_from(pages.len() - 1)
                .map_err(|_| StagingMachineFigurePaginationError::PageLimit)?;
            let rect = Rect::new(
                figure.physical_left(),
                image_y,
                figure.inline_size(),
                figure.block_size(),
            );
            used = used
                .checked_add(effective_space_before)
                .and_then(|value| value.checked_add(image_height))
                .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)?;
            increment_figure_page_count(&mut pages, page_index, true)?;
            for caption in caption_inputs {
                let y = checked_figure_y(layout.body(), used, 0)?;
                caption_fragments.push(StagingMachineFigureCaptionFragment {
                    caption_owner: caption.owner,
                    caption_flow_id: figure.caption_flow_id(),
                    page_index,
                    rect: Rect::new(
                        layout.body().x(),
                        y,
                        layout.body().width(),
                        caption.block_size,
                    ),
                });
                used = used
                    .checked_add(caption.block_size.get().raw())
                    .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)?;
                increment_figure_page_count(&mut pages, page_index, false)?;
            }
            placements.push(staging_figure_placement(
                figure,
                page_index,
                rect,
                effective_space_before,
                moved_to_fresh_page,
                caption_fragments,
            ));
        } else {
            let requested = effective_space_before
                .checked_add(image_height)
                .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)?;
            if requested > body_height - used {
                add_staging_figure_page(&mut pages, limits)?;
                used = 0;
                effective_space_before = 0;
                moved_to_fresh_page = true;
            }
            let page_index = u32::try_from(pages.len() - 1)
                .map_err(|_| StagingMachineFigurePaginationError::PageLimit)?;
            let image_y = checked_figure_y(layout.body(), used, effective_space_before)?;
            let rect = Rect::new(
                figure.physical_left(),
                image_y,
                figure.inline_size(),
                figure.block_size(),
            );
            used = used
                .checked_add(effective_space_before)
                .and_then(|value| value.checked_add(image_height))
                .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)?;
            increment_figure_page_count(&mut pages, page_index, true)?;
            for caption in caption_inputs {
                let caption_height = caption.block_size.get().raw();
                if caption_height > body_height {
                    return Err(StagingMachineFigurePaginationError::CaptionOversize(
                        caption.owner,
                    ));
                }
                if caption_height > body_height - used {
                    add_staging_figure_page(&mut pages, limits)?;
                    used = 0;
                }
                let caption_page = u32::try_from(pages.len() - 1)
                    .map_err(|_| StagingMachineFigurePaginationError::PageLimit)?;
                let y = checked_figure_y(layout.body(), used, 0)?;
                caption_fragments.push(StagingMachineFigureCaptionFragment {
                    caption_owner: caption.owner,
                    caption_flow_id: figure.caption_flow_id(),
                    page_index: caption_page,
                    rect: Rect::new(
                        layout.body().x(),
                        y,
                        layout.body().width(),
                        caption.block_size,
                    ),
                });
                used = used
                    .checked_add(caption_height)
                    .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)?;
                increment_figure_page_count(&mut pages, caption_page, false)?;
            }
            placements.push(staging_figure_placement(
                figure,
                page_index,
                rect,
                effective_space_before,
                moved_to_fresh_page,
                caption_fragments,
            ));
        }
        if used > body_height {
            return Err(StagingMachineFigurePaginationError::NoProgress(
                figure.figure_owner(),
            ));
        }
        pending_space_after = figure.space_after().get().raw();
    }

    let mut selected = StagingMachineFigureSelectedState {
        package_sha256: layout.package_sha256(),
        epoch: layout.epoch(),
        flow_registry: layout.flow_registry_fingerprint(),
        figure_usage_sha256: layout.figure_usage_sha256(),
        policy_version: layout.policy_version(),
        master_id: layout.master_id().clone(),
        page_width: layout.page_width(),
        page_height: layout.page_height(),
        body: layout.body(),
        initial_consumed_block_size: input.initial_consumed_block_size,
        pages,
        figures: placements,
        state_fingerprint: LayoutStateFingerprint::from_untrusted_bytes([0; 32]),
        canonical_jcs: String::new(),
    };
    selected.canonical_jcs = encode_staging_machine_figure_selected(&selected);
    selected.state_fingerprint =
        materialized_pagination_state_fingerprint_from_jcs(&selected.canonical_jcs);
    Ok(selected)
}

fn add_staging_figure_page(
    pages: &mut Vec<StagingMachineFigureSelectedPage>,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingMachineFigurePaginationError> {
    let page_index =
        u32::try_from(pages.len()).map_err(|_| StagingMachineFigurePaginationError::PageLimit)?;
    if page_index >= limits.get().max_pages {
        return Err(StagingMachineFigurePaginationError::PageLimit);
    }
    pages
        .try_reserve(1)
        .map_err(|_| StagingMachineFigurePaginationError::AllocationFailure)?;
    pages.push(StagingMachineFigureSelectedPage {
        page_index,
        figure_count: 0,
        caption_block_count: 0,
    });
    Ok(())
}

fn increment_figure_page_count(
    pages: &mut [StagingMachineFigureSelectedPage],
    page_index: u32,
    figure: bool,
) -> Result<(), StagingMachineFigurePaginationError> {
    let page = pages
        .get_mut(page_index as usize)
        .ok_or(StagingMachineFigurePaginationError::PageLimit)?;
    let count = if figure {
        &mut page.figure_count
    } else {
        &mut page.caption_block_count
    };
    *count = count
        .checked_add(1)
        .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)?;
    Ok(())
}

fn checked_figure_y(
    body: Rect,
    used: i64,
    before: i64,
) -> Result<typaxis_core::Length, StagingMachineFigurePaginationError> {
    let offset = used
        .checked_add(before)
        .and_then(typaxis_core::Length::from_raw)
        .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)?;
    body.y()
        .checked_add(offset)
        .ok_or(StagingMachineFigurePaginationError::ArithmeticOverflow)
}

fn staging_figure_placement(
    figure: &typaxis_layout::ValidatedFigureLayoutItem,
    page_index: u32,
    rect: Rect,
    effective_space_before: i64,
    moved_to_fresh_page: bool,
    caption_fragments: Vec<StagingMachineFigureCaptionFragment>,
) -> StagingMachineFigurePlacement {
    StagingMachineFigurePlacement {
        figure_owner: figure.figure_owner(),
        document_ordinal: figure.document_ordinal(),
        figure_flow_id: figure.figure_flow_id(),
        caption_flow_id: figure.caption_flow_id(),
        image_id: figure.image_id(),
        alt: figure.alt().to_owned(),
        admitted_media_kind: figure.admitted_media_kind().as_str(),
        admitted_sha256: figure.admitted_sha256(),
        admitted_byte_length: figure.admitted_byte_length(),
        pixel_width: figure.pixel_width().get(),
        pixel_height: figure.pixel_height().get(),
        decoded_bytes: figure.decoded_bytes(),
        page_index,
        rect,
        effective_space_before: NonNegativeLength::new(
            typaxis_core::Length::from_raw(effective_space_before)
                .expect("selected Figure spacing remains in the fixed-point range"),
        )
        .expect("selected Figure spacing is nonnegative"),
        keep_policy: figure.keep_policy(),
        oversize_policy: figure.oversize_policy(),
        moved_to_fresh_page,
        caption_fragments,
    }
}

fn encode_staging_machine_figure_selected(value: &StagingMachineFigureSelectedState) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_FIGURE_SELECTED_ALGORITHM);
    output.push_str(",\"body\":");
    encode_rect(&mut output, value.body);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"figure_usage_sha256\":");
    push_hex(&mut output, value.figure_usage_sha256);
    output.push_str(",\"figures\":[");
    for (index, figure) in value.figures.iter().enumerate() {
        comma(&mut output, index);
        encode_staging_machine_figure_placement(&mut output, figure);
    }
    output.push_str("],\"flow_registry_sha256\":");
    push_hex(&mut output, value.flow_registry.bytes());
    output.push_str(",\"initial_consumed_block_size\":");
    output.push_str(&value.initial_consumed_block_size.get().raw().to_string());
    output.push_str(",\"layout_epoch\":");
    encode_layout_epoch(&mut output, value.epoch);
    output.push_str(",\"master_id\":");
    push_jcs_string(&mut output, value.master_id.as_str());
    output.push_str(",\"package_sha256\":");
    push_hex(&mut output, value.package_sha256);
    output.push_str(",\"page_count\":");
    output.push_str(&value.pages.len().to_string());
    output.push_str(",\"page_height\":");
    output.push_str(&value.page_height.get().raw().to_string());
    output.push_str(",\"page_width\":");
    output.push_str(&value.page_width.get().raw().to_string());
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        comma(&mut output, index);
        output.push_str("{\"caption_block_count\":");
        output.push_str(&page.caption_block_count.to_string());
        output.push_str(",\"figure_count\":");
        output.push_str(&page.figure_count.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push('}');
    }
    output.push_str("],\"policy_version\":");
    push_jcs_string(&mut output, value.policy_version);
    output.push_str(",\"state_algorithm\":");
    push_jcs_string(
        &mut output,
        LayoutStateFingerprint::MATERIALIZED_ALGORITHM_ID,
    );
    output.push('}');
    output
}

fn encode_staging_machine_figure_placement(
    output: &mut String,
    figure: &StagingMachineFigurePlacement,
) {
    output.push_str("{\"admitted_byte_length\":");
    output.push_str(&figure.admitted_byte_length.to_string());
    output.push_str(",\"admitted_sha256\":");
    push_hex(output, figure.admitted_sha256);
    output.push_str(",\"alt\":");
    push_jcs_string(output, &figure.alt);
    output.push_str(",\"attested_media_kind\":");
    push_jcs_string(output, figure.admitted_media_kind);
    output.push_str(",\"caption_flow_id\":");
    output.push_str(&figure.caption_flow_id.get().to_string());
    output.push_str(",\"caption_fragments\":[");
    for (index, caption) in figure.caption_fragments.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"caption_flow_id\":");
        output.push_str(&caption.caption_flow_id.get().to_string());
        output.push_str(",\"caption_node_id\":");
        output.push_str(&caption.caption_owner.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&caption.page_index.to_string());
        output.push_str(",\"rect\":");
        encode_rect(output, caption.rect);
        output.push('}');
    }
    output.push_str("],\"decoded_bytes\":");
    output.push_str(&figure.decoded_bytes.to_string());
    output.push_str(",\"document_ordinal\":");
    output.push_str(&figure.document_ordinal.to_string());
    output.push_str(",\"effective_space_before\":");
    output.push_str(&figure.effective_space_before.get().raw().to_string());
    output.push_str(",\"figure_flow_id\":");
    output.push_str(&figure.figure_flow_id.get().to_string());
    output.push_str(",\"figure_node_id\":");
    output.push_str(&figure.figure_owner.get().to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&figure.image_id.get().to_string());
    output.push_str(",\"keep_policy\":");
    push_jcs_string(output, figure.keep_policy.as_str());
    output.push_str(",\"moved_to_fresh_page\":");
    output.push_str(if figure.moved_to_fresh_page {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"oversize_policy\":");
    push_jcs_string(output, figure.oversize_policy.as_str());
    output.push_str(",\"page_index\":");
    output.push_str(&figure.page_index.to_string());
    output.push_str(",\"pixel_height\":");
    output.push_str(&figure.pixel_height.to_string());
    output.push_str(",\"pixel_width\":");
    output.push_str(&figure.pixel_width.to_string());
    output.push_str(",\"rect\":");
    encode_rect(output, figure.rect);
    output.push('}');
}

pub const STAGING_FORCED_PAGE_BREAK_TRACE_ALGORITHM: &str = "typaxis.forced-page-break-trace/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingForcedPageBreakPaginationError {
    Flow(MultiFlowError),
    LayoutRegistryMismatch,
    UnknownPaintOwner(NodeId),
    DuplicatePaintOwner(NodeId),
    ForcedBoundaryPaint(NodeId),
    MissingBoundary(NodeId),
    ExtraBoundary(NodeId),
    WrongBoundary(NodeId),
    CursorDidNotAdvance(NodeId),
    PageLimit,
    ArithmeticOverflow,
    AllocationFailure,
}

impl StagingForcedPageBreakPaginationError {
    /// Public diagnostic identity for contradictions in the sealed break
    /// consume chain. In particular, retrying `More` at the pre-break cursor
    /// maps to the contract's internal invariant code.
    pub const fn invariant_diagnostic_code(self) -> Option<DiagnosticCode> {
        match self {
            Self::MissingBoundary(_)
            | Self::ExtraBoundary(_)
            | Self::WrongBoundary(_)
            | Self::CursorDidNotAdvance(_) => Some(I9190),
            Self::Flow(_)
            | Self::LayoutRegistryMismatch
            | Self::UnknownPaintOwner(_)
            | Self::DuplicatePaintOwner(_)
            | Self::ForcedBoundaryPaint(_)
            | Self::PageLimit
            | Self::ArithmeticOverflow
            | Self::AllocationFailure => None,
        }
    }
}

impl From<MultiFlowError> for StagingForcedPageBreakPaginationError {
    fn from(value: MultiFlowError) -> Self {
        Self::Flow(value)
    }
}

/// Positive-area observations supplied by the staging layout fixture. The
/// constructor rejects a `page_break` owner, preserving the distinction
/// between a forced boundary and zero-sized/paintable content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakPaginationInput {
    painted_content_owners: Vec<NodeId>,
}

impl StagingForcedPageBreakPaginationInput {
    pub fn new(
        ir: &ProductionFlowIr,
        mut painted_content_owners: Vec<NodeId>,
    ) -> Result<Self, StagingForcedPageBreakPaginationError> {
        painted_content_owners.sort_unstable();
        if let Some(pair) = painted_content_owners
            .windows(2)
            .find(|pair| pair[0] == pair[1])
        {
            return Err(StagingForcedPageBreakPaginationError::DuplicatePaintOwner(
                pair[1],
            ));
        }
        for owner in &painted_content_owners {
            let kind = ir
                .flows()
                .iter()
                .flat_map(|flow| flow.positions())
                .find(|position| position.content_owner_node_id() == Some(*owner))
                .and_then(ProductionFlowPosition::content_kind)
                .ok_or(StagingForcedPageBreakPaginationError::UnknownPaintOwner(
                    *owner,
                ))?;
            if kind == FlowContentKind::PageBreak {
                return Err(StagingForcedPageBreakPaginationError::ForcedBoundaryPaint(
                    *owner,
                ));
            }
        }
        Ok(Self {
            painted_content_owners,
        })
    }

    pub fn painted_content_owners(&self) -> &[NodeId] {
        &self.painted_content_owners
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakCursor {
    flow_id: FlowId,
    flow_local_ordinal: u32,
}

impl StagingForcedPageBreakCursor {
    fn from_position(position: &ProductionFlowPosition) -> Self {
        Self {
            flow_id: position.flow_id(),
            flow_local_ordinal: position.flow_local_ordinal(),
        }
    }

    pub const fn flow_id(self) -> FlowId {
        self.flow_id
    }

    pub const fn flow_local_ordinal(self) -> u32 {
        self.flow_local_ordinal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakConsumeReceipt {
    break_owner: NodeId,
    document_ordinal: u32,
    epoch: LayoutEpoch,
    before_cursor: StagingForcedPageBreakCursor,
    after_cursor: StagingForcedPageBreakCursor,
    produced_page_index: u32,
}

impl StagingForcedPageBreakConsumeReceipt {
    pub const fn break_owner(&self) -> NodeId {
        self.break_owner
    }

    pub const fn document_ordinal(&self) -> u32 {
        self.document_ordinal
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub const fn before_cursor(&self) -> StagingForcedPageBreakCursor {
        self.before_cursor
    }

    pub const fn after_cursor(&self) -> StagingForcedPageBreakCursor {
        self.after_cursor
    }

    pub const fn produced_page_index(&self) -> u32 {
        self.produced_page_index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakSelectedPage {
    page_index: u32,
    painted_content_count: u32,
}

impl StagingForcedPageBreakSelectedPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn painted_content_count(&self) -> u32 {
        self.painted_content_count
    }

    pub const fn is_blank(&self) -> bool {
        self.painted_content_count == 0
    }
}

/// Selected forced-boundary state. It owns the all-flow terminal proof and
/// one consume receipt per layout boundary; construction cannot return a
/// continuation carrying the pre-break cursor.
#[derive(Debug)]
pub struct StagingForcedPageBreakSelectedState {
    package_sha256: [u8; 32],
    epoch: LayoutEpoch,
    flow_registry: FlowRegistryFingerprint,
    usage_sha256: [u8; 32],
    policy_version: &'static str,
    multi_flow: MultiFlowSelectedStateReceipt,
    pages: Vec<StagingForcedPageBreakSelectedPage>,
    breaks: Vec<StagingForcedPageBreakConsumeReceipt>,
}

impl StagingForcedPageBreakSelectedState {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub const fn flow_registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.flow_registry
    }

    pub const fn usage_sha256(&self) -> [u8; 32] {
        self.usage_sha256
    }

    pub const fn policy_version(&self) -> &'static str {
        self.policy_version
    }

    pub fn page_count(&self) -> u32 {
        self.pages.len() as u32
    }

    pub const fn multi_flow(&self) -> &MultiFlowSelectedStateReceipt {
        &self.multi_flow
    }

    pub fn pages(&self) -> &[StagingForcedPageBreakSelectedPage] {
        &self.pages
    }

    pub fn breaks(&self) -> &[StagingForcedPageBreakConsumeReceipt] {
        &self.breaks
    }

    pub fn validate_break_closure(&self) -> Result<(), StagingForcedPageBreakPaginationError> {
        let expected_page_count = u32::try_from(self.breaks.len())
            .map_err(|_| StagingForcedPageBreakPaginationError::ArithmeticOverflow)?
            .checked_add(1)
            .ok_or(StagingForcedPageBreakPaginationError::ArithmeticOverflow)?;
        if self.pages.len() != expected_page_count as usize {
            return Err(StagingForcedPageBreakPaginationError::PageLimit);
        }
        for (index, receipt) in self.breaks.iter().enumerate() {
            let expected_ordinal = u32::try_from(index)
                .map_err(|_| StagingForcedPageBreakPaginationError::ArithmeticOverflow)?;
            if receipt.document_ordinal != expected_ordinal
                || receipt.produced_page_index != expected_ordinal + 1
                || receipt.epoch != self.epoch
            {
                return Err(StagingForcedPageBreakPaginationError::WrongBoundary(
                    receipt.break_owner,
                ));
            }
            validate_forced_page_break_cursor_advance(
                receipt.break_owner,
                receipt.before_cursor,
                receipt.after_cursor,
            )?;
        }
        for (index, page) in self.pages.iter().enumerate() {
            if usize::try_from(page.page_index) != Ok(index) {
                return Err(StagingForcedPageBreakPaginationError::PageLimit);
            }
        }
        Ok(())
    }

    pub fn trace_facts(&self) -> StagingForcedPageBreakTraceFacts {
        StagingForcedPageBreakTraceFacts::from_selected(self)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakTraceFacts {
    flow_registry_sha256: [u8; 32],
    usage_sha256: [u8; 32],
    epoch: LayoutEpoch,
    page_count: u32,
    policy_version: &'static str,
    pages: Vec<StagingForcedPageBreakSelectedPage>,
    breaks: Vec<StagingForcedPageBreakConsumeReceipt>,
    canonical_jcs: String,
}

impl StagingForcedPageBreakTraceFacts {
    fn from_selected(selected: &StagingForcedPageBreakSelectedState) -> Self {
        let mut value = Self {
            flow_registry_sha256: selected.flow_registry.bytes(),
            usage_sha256: selected.usage_sha256,
            epoch: selected.epoch,
            page_count: selected.page_count(),
            policy_version: selected.policy_version,
            pages: selected.pages.clone(),
            breaks: selected.breaks.clone(),
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_forced_page_break_trace(&value);
        value
    }

    pub const fn flow_registry_sha256(&self) -> [u8; 32] {
        self.flow_registry_sha256
    }

    pub const fn usage_sha256(&self) -> [u8; 32] {
        self.usage_sha256
    }

    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub fn pages(&self) -> &[StagingForcedPageBreakSelectedPage] {
        &self.pages
    }

    pub fn breaks(&self) -> &[StagingForcedPageBreakConsumeReceipt] {
        &self.breaks
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

pub fn paginate_staging_forced_page_breaks(
    layout: &StagingForcedPageBreakLayoutReceipt,
    ir: &ProductionFlowIr,
    input: &StagingForcedPageBreakPaginationInput,
    limits: &ValidatedResourceLimits,
) -> Result<StagingForcedPageBreakSelectedState, StagingForcedPageBreakPaginationError> {
    if layout.flow_registry_fingerprint() != ir.registry().receipt().fingerprint()
        || layout.epoch() != ir.registry().receipt().epoch()
    {
        return Err(StagingForcedPageBreakPaginationError::LayoutRegistryMismatch);
    }
    let required_page_count = u32::try_from(layout.boundaries().len())
        .map_err(|_| StagingForcedPageBreakPaginationError::PageLimit)?
        .checked_add(1)
        .ok_or(StagingForcedPageBreakPaginationError::PageLimit)?;
    if required_page_count > limits.get().max_pages {
        return Err(StagingForcedPageBreakPaginationError::PageLimit);
    }
    let painted: BTreeSet<_> = input.painted_content_owners.iter().copied().collect();
    let mut observed_paint = BTreeSet::new();
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(layout.boundaries().len().saturating_add(1))
        .map_err(|_| StagingForcedPageBreakPaginationError::AllocationFailure)?;
    pages.push(StagingForcedPageBreakSelectedPage {
        page_index: 0,
        painted_content_count: 0,
    });
    let mut receipts = Vec::new();
    receipts
        .try_reserve_exact(layout.boundaries().len())
        .map_err(|_| StagingForcedPageBreakPaginationError::AllocationFailure)?;
    let mut cursor = MultiFlowCursorReceipt::new(ir)?;

    loop {
        let position = cursor.current_position(ir)?;
        if position.is_terminal() {
            if cursor.current_flow() == FlowId::DOCUMENT_BODY {
                break;
            }
            cursor.leave_terminal(ir)?;
            cursor.advance(ir)?;
            continue;
        }

        let owner = position
            .content_owner_node_id()
            .ok_or(StagingForcedPageBreakPaginationError::LayoutRegistryMismatch)?;
        if position.content_kind() == Some(FlowContentKind::PageBreak) {
            let expected = layout
                .boundaries()
                .get(receipts.len())
                .ok_or(StagingForcedPageBreakPaginationError::ExtraBoundary(owner))?;
            if expected.owner() != owner
                || expected.flow_id() != position.flow_id()
                || expected.flow_local_ordinal() != position.flow_local_ordinal()
                || expected.epoch() != position.epoch()
            {
                return Err(StagingForcedPageBreakPaginationError::WrongBoundary(owner));
            }
            let current_page = u32::try_from(pages.len() - 1)
                .map_err(|_| StagingForcedPageBreakPaginationError::PageLimit)?;
            let produced_page_index = current_page
                .checked_add(1)
                .ok_or(StagingForcedPageBreakPaginationError::PageLimit)?;
            if produced_page_index >= limits.get().max_pages {
                return Err(StagingForcedPageBreakPaginationError::PageLimit);
            }
            let before_cursor = StagingForcedPageBreakCursor::from_position(position);
            cursor.advance(ir)?;
            let after_cursor =
                StagingForcedPageBreakCursor::from_position(cursor.current_position(ir)?);
            validate_forced_page_break_cursor_advance(owner, before_cursor, after_cursor)?;
            pages.push(StagingForcedPageBreakSelectedPage {
                page_index: produced_page_index,
                painted_content_count: 0,
            });
            receipts.push(StagingForcedPageBreakConsumeReceipt {
                break_owner: owner,
                document_ordinal: expected.document_ordinal(),
                epoch: position.epoch(),
                before_cursor,
                after_cursor,
                produced_page_index,
            });
            continue;
        }

        if painted.contains(&owner) && observed_paint.insert(owner) {
            let page = pages
                .last_mut()
                .expect("forced-break pagination always retains one open page");
            page.painted_content_count = page
                .painted_content_count
                .checked_add(1)
                .ok_or(StagingForcedPageBreakPaginationError::ArithmeticOverflow)?;
        }
        if let Some(child) = position.child_flow_id() {
            cursor.enter_child(ir, child)?;
        } else {
            cursor.advance(ir)?;
        }
    }

    if let Some(owner) = painted.difference(&observed_paint).next().copied() {
        return Err(StagingForcedPageBreakPaginationError::UnknownPaintOwner(
            owner,
        ));
    }
    if receipts.len() != layout.boundaries().len() {
        let missing = layout.boundaries()[receipts.len()].owner();
        return Err(StagingForcedPageBreakPaginationError::MissingBoundary(
            missing,
        ));
    }
    let multi_flow = cursor.finish(ir)?;
    let selected = StagingForcedPageBreakSelectedState {
        package_sha256: layout.package_sha256(),
        epoch: layout.epoch(),
        flow_registry: layout.flow_registry_fingerprint(),
        usage_sha256: layout.usage_sha256(),
        policy_version: layout.policy_version(),
        multi_flow,
        pages,
        breaks: receipts,
    };
    selected.validate_break_closure()?;
    Ok(selected)
}

fn validate_forced_page_break_cursor_advance(
    owner: NodeId,
    before: StagingForcedPageBreakCursor,
    after: StagingForcedPageBreakCursor,
) -> Result<(), StagingForcedPageBreakPaginationError> {
    if after.flow_id != before.flow_id
        || before.flow_local_ordinal.checked_add(1) != Some(after.flow_local_ordinal)
    {
        return Err(StagingForcedPageBreakPaginationError::CursorDidNotAdvance(
            owner,
        ));
    }
    Ok(())
}

fn encode_staging_forced_page_break_trace(value: &StagingForcedPageBreakTraceFacts) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_FORCED_PAGE_BREAK_TRACE_ALGORITHM);
    output.push_str(",\"break_usage_sha256\":");
    push_hex(&mut output, value.usage_sha256);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"flow_registry_sha256\":");
    push_hex(&mut output, value.flow_registry_sha256);
    output.push_str(",\"forced_page_breaks\":[");
    for (index, boundary) in value.breaks.iter().enumerate() {
        comma(&mut output, index);
        encode_staging_forced_page_break_receipt(&mut output, boundary);
    }
    output.push_str("],\"layout_epoch\":");
    encode_layout_epoch(&mut output, value.epoch);
    output.push_str(",\"page_count\":");
    output.push_str(&value.page_count.to_string());
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        comma(&mut output, index);
        encode_staging_forced_page_break_page(&mut output, page);
    }
    output.push_str("],\"policy_version\":");
    push_jcs_string(&mut output, value.policy_version);
    output.push('}');
    output
}

fn encode_staging_forced_page_break_receipt(
    output: &mut String,
    boundary: &StagingForcedPageBreakConsumeReceipt,
) {
    output.push_str("{\"after_cursor\":");
    encode_staging_forced_page_break_cursor(output, boundary.after_cursor);
    output.push_str(",\"before_cursor\":");
    encode_staging_forced_page_break_cursor(output, boundary.before_cursor);
    output.push_str(",\"break_node_id\":");
    output.push_str(&boundary.break_owner.get().to_string());
    output.push_str(",\"document_ordinal\":");
    output.push_str(&boundary.document_ordinal.to_string());
    output.push_str(",\"produced_page_index\":");
    output.push_str(&boundary.produced_page_index.to_string());
    output.push('}');
}

fn encode_staging_forced_page_break_cursor(
    output: &mut String,
    cursor: StagingForcedPageBreakCursor,
) {
    output.push_str("{\"flow_id\":");
    output.push_str(&cursor.flow_id.get().to_string());
    output.push_str(",\"flow_local_ordinal\":");
    output.push_str(&cursor.flow_local_ordinal.to_string());
    output.push('}');
}

fn encode_staging_forced_page_break_page(
    output: &mut String,
    page: &StagingForcedPageBreakSelectedPage,
) {
    output.push_str("{\"is_blank\":");
    output.push_str(if page.is_blank() { "true" } else { "false" });
    output.push_str(",\"page_index\":");
    output.push_str(&page.page_index.to_string());
    output.push_str(",\"painted_content_count\":");
    output.push_str(&page.painted_content_count.to_string());
    output.push('}');
}

pub const FOOTNOTE_CONVERGENCE_RECEIPT_ALGORITHM: &str = "typaxis.footnote-convergence-receipt/1";
const FOOTNOTE_PAGINATION_STATE_ALGORITHM: &str = "typaxis.footnote-pagination-state/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingFootnotePaginationError {
    RegistryMismatch,
    StateMismatch,
    InvalidPageInput,
    InvalidBodyCandidate,
    BodyEvaluationFailed,
    UnknownReferenceOwner(NodeId),
    DuplicateReferenceOccurrence(NodeId),
    NonCanonicalReferenceOrder(NodeId),
    MissingDefinition(FootnoteId),
    InvalidFootnoteCursor(FootnoteFlowId),
    BodyOversize,
    DefinitionOversize(FootnoteId),
    PageLimit,
    FragmentLimit,
    ReflowLimit,
    ReflowOscillation,
    IncompleteSelectedLayout,
    DuplicateDefinitionPaint(FootnoteId),
    MissingDefinitionPaint(FootnoteId),
    WrongPageCarry(FootnoteFlowId),
    ArithmeticOverflow,
    AllocationFailure,
}

impl StagingFootnotePaginationError {
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::BodyOversize | Self::DefinitionOversize(_) => L5100,
            Self::FragmentLimit => L5110,
            Self::ReflowLimit | Self::ReflowOscillation => G6002,
            Self::RegistryMismatch
            | Self::StateMismatch
            | Self::InvalidPageInput
            | Self::InvalidBodyCandidate
            | Self::BodyEvaluationFailed
            | Self::UnknownReferenceOwner(_)
            | Self::DuplicateReferenceOccurrence(_)
            | Self::NonCanonicalReferenceOrder(_)
            | Self::MissingDefinition(_)
            | Self::InvalidFootnoteCursor(_)
            | Self::PageLimit
            | Self::IncompleteSelectedLayout
            | Self::DuplicateDefinitionPaint(_)
            | Self::MissingDefinitionPaint(_)
            | Self::WrongPageCarry(_)
            | Self::ArithmeticOverflow
            | Self::AllocationFailure => I9190,
        }
    }

    pub const fn severity(&self) -> Severity {
        Severity::Fatal
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingFootnoteBodyContinuation {
    next_flow_position: u32,
    terminal: bool,
}

impl StagingFootnoteBodyContinuation {
    pub const fn more(next_flow_position: u32) -> Self {
        Self {
            next_flow_position,
            terminal: false,
        }
    }

    pub const fn exhausted(terminal_flow_position: u32) -> Self {
        Self {
            next_flow_position: terminal_flow_position,
            terminal: true,
        }
    }

    pub const fn next_flow_position(self) -> u32 {
        self.next_flow_position
    }

    pub const fn is_terminal(self) -> bool {
        self.terminal
    }
}

/// Body result supplied by the sole page-body evaluator. The pagination owner
/// revalidates the applied frame and resolves reference owners through the
/// package-derived registry; callers cannot supply FootnoteIds or order keys.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteBodyCandidate {
    body_fingerprint: LayoutStateFingerprint,
    continuation: StagingFootnoteBodyContinuation,
    applied_reservation: NonNegativeLength,
    applied_body_cut_before_reference_owner: Option<NodeId>,
    available_body_block_size: PositiveLength,
    selected_body_fragment_count: u64,
    reference_owners: Vec<NodeId>,
}

impl StagingFootnoteBodyCandidate {
    pub fn new(
        body_fingerprint: LayoutStateFingerprint,
        continuation: StagingFootnoteBodyContinuation,
        applied_reservation: NonNegativeLength,
        available_body_block_size: PositiveLength,
        reference_owners: Vec<NodeId>,
    ) -> Self {
        Self::new_with_body_cut(
            body_fingerprint,
            continuation,
            applied_reservation,
            None,
            available_body_block_size,
            reference_owners,
        )
    }

    pub fn new_with_body_cut(
        body_fingerprint: LayoutStateFingerprint,
        continuation: StagingFootnoteBodyContinuation,
        applied_reservation: NonNegativeLength,
        applied_body_cut_before_reference_owner: Option<NodeId>,
        available_body_block_size: PositiveLength,
        reference_owners: Vec<NodeId>,
    ) -> Self {
        Self {
            body_fingerprint,
            continuation,
            applied_reservation,
            applied_body_cut_before_reference_owner,
            available_body_block_size,
            selected_body_fragment_count: 0,
            reference_owners,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_body_fragments(
        body_fingerprint: LayoutStateFingerprint,
        continuation: StagingFootnoteBodyContinuation,
        applied_reservation: NonNegativeLength,
        applied_body_cut_before_reference_owner: Option<NodeId>,
        available_body_block_size: PositiveLength,
        selected_body_fragment_count: u64,
        reference_owners: Vec<NodeId>,
    ) -> Self {
        Self {
            body_fingerprint,
            continuation,
            applied_reservation,
            applied_body_cut_before_reference_owner,
            available_body_block_size,
            selected_body_fragment_count,
            reference_owners,
        }
    }

    pub const fn body_fingerprint(&self) -> LayoutStateFingerprint {
        self.body_fingerprint
    }

    pub const fn continuation(&self) -> StagingFootnoteBodyContinuation {
        self.continuation
    }

    pub const fn applied_reservation(&self) -> NonNegativeLength {
        self.applied_reservation
    }

    pub const fn applied_body_cut_before_reference_owner(&self) -> Option<NodeId> {
        self.applied_body_cut_before_reference_owner
    }

    pub const fn available_body_block_size(&self) -> PositiveLength {
        self.available_body_block_size
    }

    pub fn reference_owners(&self) -> &[NodeId] {
        &self.reference_owners
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingFootnotePageInput {
    page_index: u32,
    body_page_start: LayoutStateFingerprint,
}

impl StagingFootnotePageInput {
    pub const fn new(page_index: u32, body_page_start: LayoutStateFingerprint) -> Self {
        Self {
            page_index,
            body_page_start,
        }
    }

    pub const fn page_index(self) -> u32 {
        self.page_index
    }

    pub const fn body_page_start(self) -> LayoutStateFingerprint {
        self.body_page_start
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnotePageEvaluationRequest {
    global_pass: u16,
    page_index: u32,
    evaluation_index: u16,
    master_id: MasterId,
    body_page_start: LayoutStateFingerprint,
    applied_reservation: NonNegativeLength,
    body_cut_before_reference_owner: Option<NodeId>,
    available_body_block_size: PositiveLength,
}

impl StagingFootnotePageEvaluationRequest {
    pub const fn global_pass(&self) -> u16 {
        self.global_pass
    }

    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn evaluation_index(&self) -> u16 {
        self.evaluation_index
    }

    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }

    pub const fn body_page_start(&self) -> LayoutStateFingerprint {
        self.body_page_start
    }

    pub const fn applied_reservation(&self) -> NonNegativeLength {
        self.applied_reservation
    }

    /// When present, the body owner must select the greatest legal body break
    /// before this reference occurrence. Returning that occurrence or a later
    /// one proves that the required body/definition keep is unsatisfiable.
    pub const fn body_cut_before_reference_owner(&self) -> Option<NodeId> {
        self.body_cut_before_reference_owner
    }

    pub const fn available_body_block_size(&self) -> PositiveLength {
        self.available_body_block_size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPageFootnoteReferenceOccurrence {
    reference_owner: NodeId,
    footnote_id: FootnoteId,
    document_logical_ordinal: u32,
}

impl StagingPageFootnoteReferenceOccurrence {
    pub const fn reference_owner(&self) -> NodeId {
        self.reference_owner
    }

    pub const fn footnote_id(&self) -> &FootnoteId {
        &self.footnote_id
    }

    pub const fn document_logical_ordinal(&self) -> u32 {
        self.document_logical_ordinal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteAssignment {
    footnote_id: FootnoteId,
    flow_id: FootnoteFlowId,
    assignment_ordinal: u32,
    first_reference_owner: NodeId,
}

impl StagingFootnoteAssignment {
    pub const fn footnote_id(&self) -> &FootnoteId {
        &self.footnote_id
    }

    pub const fn flow_id(&self) -> FootnoteFlowId {
        self.flow_id
    }

    pub const fn assignment_ordinal(&self) -> u32 {
        self.assignment_ordinal
    }

    pub const fn first_reference_owner(&self) -> NodeId {
        self.first_reference_owner
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingFootnoteFlowCursor {
    flow_id: FootnoteFlowId,
    next_fragment_ordinal: u32,
}

impl StagingFootnoteFlowCursor {
    const fn new(flow_id: FootnoteFlowId, next_fragment_ordinal: u32) -> Self {
        Self {
            flow_id,
            next_fragment_ordinal,
        }
    }

    pub const fn flow_id(self) -> FootnoteFlowId {
        self.flow_id
    }

    pub const fn next_fragment_ordinal(self) -> u32 {
        self.next_fragment_ordinal
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingFootnoteFragmentReceipt {
    fragment_ordinal: u32,
    block_extent: PositiveLength,
}

impl StagingFootnoteFragmentReceipt {
    pub const fn fragment_ordinal(self) -> u32 {
        self.fragment_ordinal
    }

    pub const fn block_extent(self) -> PositiveLength {
        self.block_extent
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteFlowEvaluationReceipt {
    assignment: StagingFootnoteAssignment,
    incoming_source_page: Option<u32>,
    before_cursor: StagingFootnoteFlowCursor,
    after_cursor: StagingFootnoteFlowCursor,
    fragments: Vec<StagingFootnoteFragmentReceipt>,
    carries_out: bool,
}

impl StagingFootnoteFlowEvaluationReceipt {
    pub const fn assignment(&self) -> &StagingFootnoteAssignment {
        &self.assignment
    }

    pub const fn incoming_source_page(&self) -> Option<u32> {
        self.incoming_source_page
    }

    pub const fn before_cursor(&self) -> StagingFootnoteFlowCursor {
        self.before_cursor
    }

    pub const fn after_cursor(&self) -> StagingFootnoteFlowCursor {
        self.after_cursor
    }

    pub fn fragments(&self) -> &[StagingFootnoteFragmentReceipt] {
        &self.fragments
    }

    pub const fn carries_out(&self) -> bool {
        self.carries_out
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteEvaluationReceipt {
    discovery: Vec<StagingPageFootnoteReferenceOccurrence>,
    ordered_footnotes: Vec<StagingFootnoteAssignment>,
    flows: Vec<StagingFootnoteFlowEvaluationReceipt>,
    reservation: NonNegativeLength,
    body_cut_before_reference_owner: Option<NodeId>,
    selected_record_count: u64,
    fingerprint: FootnotePageEvaluationFingerprint,
    canonical_jcs: String,
}

impl StagingFootnoteEvaluationReceipt {
    pub fn discovery(&self) -> &[StagingPageFootnoteReferenceOccurrence] {
        &self.discovery
    }

    pub fn ordered_footnotes(&self) -> &[StagingFootnoteAssignment] {
        &self.ordered_footnotes
    }

    pub fn flows(&self) -> &[StagingFootnoteFlowEvaluationReceipt] {
        &self.flows
    }

    pub const fn reservation(&self) -> NonNegativeLength {
        self.reservation
    }

    pub const fn body_cut_before_reference_owner(&self) -> Option<NodeId> {
        self.body_cut_before_reference_owner
    }

    pub const fn selected_record_count(&self) -> u64 {
        self.selected_record_count
    }

    pub const fn fingerprint(&self) -> FootnotePageEvaluationFingerprint {
        self.fingerprint
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteCarryReceipt {
    assignment: StagingFootnoteAssignment,
    source_page_index: u32,
    target_page_index: u32,
    next_cursor: StagingFootnoteFlowCursor,
}

impl StagingFootnoteCarryReceipt {
    pub const fn assignment(&self) -> &StagingFootnoteAssignment {
        &self.assignment
    }

    pub const fn source_page_index(&self) -> u32 {
        self.source_page_index
    }

    pub const fn target_page_index(&self) -> u32 {
        self.target_page_index
    }

    pub const fn next_cursor(&self) -> StagingFootnoteFlowCursor {
        self.next_cursor
    }
}

/// A convergence proof remains candidate-local until the bound state commits
/// it. It is intentionally non-Clone; replay is additionally rejected by the
/// state-before fingerprint and expected page index.
#[derive(Debug)]
pub struct ValidatedFootnoteConvergenceReceipt {
    profile: FootnoteProfileFingerprint,
    registry: FootnoteFlowRegistryFingerprint,
    epoch: LayoutEpoch,
    global_pass: u16,
    page_index: u32,
    body_page_start: LayoutStateFingerprint,
    state_before_sha256: [u8; 32],
    evaluation_index: u16,
    previous_evaluation: FootnotePageEvaluationFingerprint,
    final_candidate: StagingFootnoteBodyCandidate,
    final_evaluation: StagingFootnoteEvaluationReceipt,
    canonical_jcs: String,
}

impl ValidatedFootnoteConvergenceReceipt {
    pub const fn profile_fingerprint(&self) -> FootnoteProfileFingerprint {
        self.profile
    }

    pub const fn registry_fingerprint(&self) -> FootnoteFlowRegistryFingerprint {
        self.registry
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub const fn global_pass(&self) -> u16 {
        self.global_pass
    }

    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn evaluation_index(&self) -> u16 {
        self.evaluation_index
    }

    pub const fn evaluation_count(&self) -> u32 {
        self.evaluation_index as u32 + 1
    }

    pub const fn final_evaluation(&self) -> &StagingFootnoteEvaluationReceipt {
        &self.final_evaluation
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Debug)]
pub struct StagingFootnoteSelectedPageReceipt {
    profile: FootnoteProfileFingerprint,
    registry: FootnoteFlowRegistryFingerprint,
    epoch: LayoutEpoch,
    global_pass: u16,
    page_index: u32,
    body_page_start: LayoutStateFingerprint,
    evaluation_count: u32,
    body_fingerprint: LayoutStateFingerprint,
    body_continuation: StagingFootnoteBodyContinuation,
    discovery: Vec<StagingPageFootnoteReferenceOccurrence>,
    ordered_footnotes: Vec<StagingFootnoteAssignment>,
    flows: Vec<StagingFootnoteFlowEvaluationReceipt>,
    reservation: NonNegativeLength,
    body_cut_before_reference_owner: Option<NodeId>,
    evaluation_fingerprint: FootnotePageEvaluationFingerprint,
}

impl StagingFootnoteSelectedPageReceipt {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn body_page_start(&self) -> LayoutStateFingerprint {
        self.body_page_start
    }

    pub const fn global_pass(&self) -> u16 {
        self.global_pass
    }

    pub const fn profile_fingerprint(&self) -> FootnoteProfileFingerprint {
        self.profile
    }

    pub const fn registry_fingerprint(&self) -> FootnoteFlowRegistryFingerprint {
        self.registry
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub const fn evaluation_count(&self) -> u32 {
        self.evaluation_count
    }

    pub const fn body_fingerprint(&self) -> LayoutStateFingerprint {
        self.body_fingerprint
    }

    pub const fn body_continuation(&self) -> StagingFootnoteBodyContinuation {
        self.body_continuation
    }

    pub fn discovery(&self) -> &[StagingPageFootnoteReferenceOccurrence] {
        &self.discovery
    }

    pub fn ordered_footnotes(&self) -> &[StagingFootnoteAssignment] {
        &self.ordered_footnotes
    }

    pub fn flows(&self) -> &[StagingFootnoteFlowEvaluationReceipt] {
        &self.flows
    }

    pub const fn reservation(&self) -> NonNegativeLength {
        self.reservation
    }

    pub const fn body_cut_before_reference_owner(&self) -> Option<NodeId> {
        self.body_cut_before_reference_owner
    }

    pub const fn evaluation_fingerprint(&self) -> FootnotePageEvaluationFingerprint {
        self.evaluation_fingerprint
    }
}

/// Transactional selected-state owner. Candidate assignments and fragment
/// charges are committed only after a convergence receipt is rederived.
#[derive(Debug)]
pub struct StagingFootnotePaginationState {
    profile: FootnoteProfileFingerprint,
    registry: FootnoteFlowRegistryFingerprint,
    epoch: LayoutEpoch,
    global_pass: u16,
    next_page_index: u32,
    assignments: Vec<StagingFootnoteAssignment>,
    assignment_by_footnote: BTreeMap<FootnoteId, usize>,
    carries: Vec<StagingFootnoteCarryReceipt>,
    seen_reference_owners: BTreeSet<NodeId>,
    last_reference_logical_ordinal: Option<u32>,
    selected_record_count: u64,
    selected_page_fingerprints: Vec<FootnotePageEvaluationFingerprint>,
    max_pages: u32,
    max_ast_nodes: u64,
    max_fragments: u64,
    max_footnote_reflows_per_page: u16,
}

impl StagingFootnotePaginationState {
    pub fn new(
        registry: &StagingFootnoteFlowRegistry,
        global_pass: u16,
        limits: &ValidatedResourceLimits,
    ) -> Self {
        Self {
            profile: registry.receipt().profile_fingerprint(),
            registry: registry.receipt().fingerprint(),
            epoch: registry.receipt().epoch(),
            global_pass,
            next_page_index: 0,
            assignments: Vec::new(),
            assignment_by_footnote: BTreeMap::new(),
            carries: Vec::new(),
            seen_reference_owners: BTreeSet::new(),
            last_reference_logical_ordinal: None,
            selected_record_count: 0,
            selected_page_fingerprints: Vec::new(),
            max_pages: limits.get().max_pages,
            max_ast_nodes: limits.get().max_ast_nodes,
            max_fragments: limits.get().max_fragments,
            max_footnote_reflows_per_page: limits.get().max_footnote_reflows_per_page,
        }
    }

    pub const fn next_page_index(&self) -> u32 {
        self.next_page_index
    }

    pub fn assignments(&self) -> &[StagingFootnoteAssignment] {
        &self.assignments
    }

    pub fn carries(&self) -> &[StagingFootnoteCarryReceipt] {
        &self.carries
    }

    pub const fn selected_record_count(&self) -> u64 {
        self.selected_record_count
    }

    pub fn selected_page_fingerprints(&self) -> &[FootnotePageEvaluationFingerprint] {
        &self.selected_page_fingerprints
    }

    pub fn commit_page(
        &mut self,
        registry: &StagingFootnoteFlowRegistry,
        receipt: &ValidatedFootnoteConvergenceReceipt,
    ) -> Result<StagingFootnoteSelectedPageReceipt, StagingFootnotePaginationError> {
        validate_footnote_registry_state(registry, self)?;
        if receipt.profile != self.profile
            || receipt.registry != self.registry
            || receipt.epoch != self.epoch
            || receipt.global_pass != self.global_pass
            || receipt.page_index != self.next_page_index
            || receipt.page_index >= self.max_pages
            || receipt.state_before_sha256 != encode_footnote_state_sha256(self)
            || receipt.evaluation_index == 0
            || receipt.evaluation_index > self.max_footnote_reflows_per_page
            || receipt.previous_evaluation != receipt.final_evaluation.fingerprint
            || receipt.final_candidate.applied_reservation != receipt.final_evaluation.reservation
            || receipt
                .final_candidate
                .applied_body_cut_before_reference_owner
                != receipt.final_evaluation.body_cut_before_reference_owner
        {
            return Err(StagingFootnotePaginationError::StateMismatch);
        }
        let available =
            available_body_block_size(registry.body_frame(), receipt.final_evaluation.reservation)?;
        if receipt.final_candidate.available_body_block_size != available {
            return Err(StagingFootnotePaginationError::StateMismatch);
        }
        let rederived = derive_footnote_evaluation(
            registry,
            self,
            receipt.page_index,
            receipt.body_page_start,
            &receipt.final_candidate,
        )?;
        if rederived != receipt.final_evaluation
            || receipt.canonical_jcs != encode_footnote_convergence(receipt)
        {
            return Err(StagingFootnotePaginationError::StateMismatch);
        }

        let mut new_assignments = Vec::new();
        new_assignments
            .try_reserve_exact(receipt.final_evaluation.ordered_footnotes.len())
            .map_err(|_| StagingFootnotePaginationError::AllocationFailure)?;
        let mut expected_assignment_ordinal = u32::try_from(self.assignments.len())
            .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?;
        for assignment in &receipt.final_evaluation.ordered_footnotes {
            if let Some(existing) = self.assignment(&assignment.footnote_id) {
                if existing != assignment {
                    return Err(StagingFootnotePaginationError::StateMismatch);
                }
                continue;
            }
            if assignment.assignment_ordinal != expected_assignment_ordinal
                || new_assignments
                    .iter()
                    .any(|existing: &&StagingFootnoteAssignment| {
                        existing.footnote_id == assignment.footnote_id
                    })
            {
                return Err(StagingFootnotePaginationError::StateMismatch);
            }
            expected_assignment_ordinal = expected_assignment_ordinal
                .checked_add(1)
                .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
            new_assignments.push(assignment);
        }

        let target_page_index = receipt
            .page_index
            .checked_add(1)
            .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
        let mut carries = Vec::new();
        carries
            .try_reserve_exact(
                receipt
                    .final_evaluation
                    .flows
                    .iter()
                    .filter(|flow| flow.carries_out)
                    .count(),
            )
            .map_err(|_| StagingFootnotePaginationError::AllocationFailure)?;
        for flow in &receipt.final_evaluation.flows {
            if flow.carries_out {
                carries.push(StagingFootnoteCarryReceipt {
                    assignment: flow.assignment.clone(),
                    source_page_index: receipt.page_index,
                    target_page_index,
                    next_cursor: flow.after_cursor,
                });
            }
        }
        let selected_record_count = self
            .selected_record_count
            .checked_add(receipt.final_candidate.selected_body_fragment_count)
            .and_then(|count| count.checked_add(receipt.final_evaluation.selected_record_count))
            .filter(|count| *count <= self.max_fragments)
            .ok_or(StagingFootnotePaginationError::FragmentLimit)?;
        self.assignments
            .try_reserve(new_assignments.len())
            .map_err(|_| StagingFootnotePaginationError::AllocationFailure)?;
        self.selected_page_fingerprints
            .try_reserve(1)
            .map_err(|_| StagingFootnotePaginationError::AllocationFailure)?;

        let selected = StagingFootnoteSelectedPageReceipt {
            profile: self.profile,
            registry: self.registry,
            epoch: self.epoch,
            global_pass: self.global_pass,
            page_index: receipt.page_index,
            body_page_start: receipt.body_page_start,
            evaluation_count: receipt.evaluation_count(),
            body_fingerprint: receipt.final_candidate.body_fingerprint,
            body_continuation: receipt.final_candidate.continuation,
            discovery: receipt.final_evaluation.discovery.clone(),
            ordered_footnotes: receipt.final_evaluation.ordered_footnotes.clone(),
            flows: receipt.final_evaluation.flows.clone(),
            reservation: receipt.final_evaluation.reservation,
            body_cut_before_reference_owner: receipt
                .final_evaluation
                .body_cut_before_reference_owner,
            evaluation_fingerprint: receipt.final_evaluation.fingerprint,
        };
        for assignment in new_assignments {
            let index = self.assignments.len();
            self.assignments.push(assignment.clone());
            if self
                .assignment_by_footnote
                .insert(assignment.footnote_id.clone(), index)
                .is_some()
            {
                return Err(StagingFootnotePaginationError::StateMismatch);
            }
        }
        for occurrence in &receipt.final_evaluation.discovery {
            if !self
                .seen_reference_owners
                .insert(occurrence.reference_owner)
            {
                return Err(StagingFootnotePaginationError::StateMismatch);
            }
            self.last_reference_logical_ordinal = Some(occurrence.document_logical_ordinal);
        }
        self.selected_record_count = selected_record_count;
        self.carries = carries;
        self.next_page_index = target_page_index;
        self.selected_page_fingerprints
            .push(receipt.final_evaluation.fingerprint);
        Ok(selected)
    }

    fn assignment(&self, footnote_id: &FootnoteId) -> Option<&StagingFootnoteAssignment> {
        self.assignment_by_footnote
            .get(footnote_id)
            .and_then(|index| self.assignments.get(*index))
    }

    fn carry(&self, assignment_ordinal: u32) -> Option<&StagingFootnoteCarryReceipt> {
        self.carries
            .binary_search_by_key(&assignment_ordinal, |carry| {
                carry.assignment.assignment_ordinal
            })
            .ok()
            .and_then(|index| self.carries.get(index))
    }

    /// Seals the complete selected document only after all body references
    /// and every dedicated definition cursor have reached their independent
    /// terminals. Page receipts are consumed, preventing omission or replay
    /// between pagination and Display construction.
    pub fn finish(
        self,
        registry: &StagingFootnoteFlowRegistry,
        body_layout_fingerprint: LayoutStateFingerprint,
        pages: Vec<StagingFootnoteSelectedPageReceipt>,
    ) -> Result<ValidatedFootnoteSelectedLayout, StagingFootnotePaginationError> {
        validate_footnote_registry_state(registry, &self)?;
        if !self.carries.is_empty()
            || usize::try_from(self.next_page_index).ok() != Some(pages.len())
            || self.seen_reference_owners.len() != registry.references().len()
            || registry.references().iter().any(|reference| {
                !self
                    .seen_reference_owners
                    .contains(&reference.reference_owner())
            })
            || self.assignments.len() != registry.flows().len()
            || pages.iter().enumerate().any(|(index, page)| {
                usize::try_from(page.page_index).ok() != Some(index)
                    || page.profile != self.profile
                    || page.registry != self.registry
                    || page.epoch != self.epoch
                    || self.selected_page_fingerprints.get(index)
                        != Some(&page.evaluation_fingerprint)
            })
            || !pages
                .last()
                .is_some_and(|page| page.body_continuation.is_terminal())
        {
            return Err(StagingFootnotePaginationError::IncompleteSelectedLayout);
        }
        for (expected, assignment) in self.assignments.iter().enumerate() {
            let expected = u32::try_from(expected)
                .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?;
            let registered = registry.flow(assignment.flow_id).ok_or(
                StagingFootnotePaginationError::InvalidFootnoteCursor(assignment.flow_id),
            )?;
            if assignment.assignment_ordinal != expected
                || assignment.footnote_id != *registered.binding().footnote_id()
                || registry
                    .reference(assignment.first_reference_owner)
                    .map_or(true, |reference| {
                        reference.footnote_id() != &assignment.footnote_id
                    })
            {
                return Err(StagingFootnotePaginationError::IncompleteSelectedLayout);
            }
        }

        let mut next_fragment = vec![0u32; registry.flows().len()];
        let mut last_source_page = vec![None; registry.flows().len()];
        let mut previous_body_position = None;
        let mut previous_carry_progress = false;
        let mut body_terminal = false;
        for page in &pages {
            for assignment in &page.ordered_footnotes {
                let matching_flow_count = page
                    .flows
                    .iter()
                    .filter(|flow| flow.assignment == *assignment)
                    .count();
                if matching_flow_count == 0 {
                    return Err(StagingFootnotePaginationError::MissingDefinitionPaint(
                        assignment.footnote_id.clone(),
                    ));
                }
                if matching_flow_count > 1 {
                    return Err(StagingFootnotePaginationError::DuplicateDefinitionPaint(
                        assignment.footnote_id.clone(),
                    ));
                }
            }
            if page.flows.len() != page.ordered_footnotes.len()
                || page
                    .flows
                    .iter()
                    .zip(&page.ordered_footnotes)
                    .any(|(flow, assignment)| &flow.assignment != assignment)
                || page.flows.windows(2).any(|pair| {
                    pair[0].assignment.assignment_ordinal >= pair[1].assignment.assignment_ordinal
                })
            {
                return Err(StagingFootnotePaginationError::IncompleteSelectedLayout);
            }
            let body_position = page.body_continuation.next_flow_position();
            let carry_progress = page.flows.iter().any(|flow| {
                flow.incoming_source_page.is_some()
                    && flow.after_cursor.next_fragment_ordinal
                        > flow.before_cursor.next_fragment_ordinal
            });
            if body_terminal {
                if !page.body_continuation.is_terminal()
                    || previous_body_position != Some(body_position)
                    || !carry_progress
                {
                    return Err(StagingFootnotePaginationError::IncompleteSelectedLayout);
                }
            } else if let Some(previous) = previous_body_position {
                if body_position < previous
                    || (body_position == previous
                        && !carry_progress
                        && !(page.body_continuation.is_terminal() && previous_carry_progress))
                {
                    return Err(StagingFootnotePaginationError::IncompleteSelectedLayout);
                }
            }
            body_terminal = page.body_continuation.is_terminal();
            previous_body_position = Some(body_position);
            previous_carry_progress = carry_progress;
            for flow in &page.flows {
                let flow_index = usize::try_from(flow.assignment.flow_id.get())
                    .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?;
                let registered = registry
                    .flow(flow.assignment.flow_id)
                    .filter(|registered| {
                        registered.binding().footnote_id() == &flow.assignment.footnote_id
                    })
                    .ok_or_else(|| {
                        StagingFootnotePaginationError::MissingDefinitionPaint(
                            flow.assignment.footnote_id.clone(),
                        )
                    })?;
                let expected = next_fragment.get_mut(flow_index).ok_or(
                    StagingFootnotePaginationError::InvalidFootnoteCursor(flow.assignment.flow_id),
                )?;
                let assignment_index = usize::try_from(flow.assignment.assignment_ordinal)
                    .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?;
                if flow.before_cursor.next_fragment_ordinal != *expected
                    || flow.before_cursor.flow_id != flow.assignment.flow_id
                    || flow.after_cursor.flow_id != flow.assignment.flow_id
                    || flow.fragments.is_empty()
                    || self.assignments.get(assignment_index) != Some(&flow.assignment)
                    || flow.incoming_source_page != last_source_page[flow_index]
                    || flow
                        .incoming_source_page
                        .is_some_and(|source| source.checked_add(1) != Some(page.page_index))
                    || (*expected == 0
                        && page.discovery.iter().all(|occurrence| {
                            occurrence.reference_owner != flow.assignment.first_reference_owner
                                || occurrence.footnote_id != flow.assignment.footnote_id
                        }))
                {
                    return Err(StagingFootnotePaginationError::WrongPageCarry(
                        flow.assignment.flow_id,
                    ));
                }
                for fragment in &flow.fragments {
                    if fragment.fragment_ordinal != *expected {
                        return Err(StagingFootnotePaginationError::DuplicateDefinitionPaint(
                            flow.assignment.footnote_id.clone(),
                        ));
                    }
                    let expected_extent = registered
                        .fragment_extents()
                        .get(*expected as usize)
                        .ok_or_else(|| {
                            StagingFootnotePaginationError::DuplicateDefinitionPaint(
                                flow.assignment.footnote_id.clone(),
                            )
                        })?;
                    if expected_extent != &fragment.block_extent {
                        return Err(StagingFootnotePaginationError::StateMismatch);
                    }
                    *expected = expected
                        .checked_add(1)
                        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
                }
                if flow.after_cursor.next_fragment_ordinal != *expected
                    || flow.carries_out
                        != (*expected < registered.binding().terminal().fragment_count())
                {
                    return Err(StagingFootnotePaginationError::WrongPageCarry(
                        flow.assignment.flow_id,
                    ));
                }
                last_source_page[flow_index] = Some(page.page_index);
            }
        }
        for (index, flow) in registry.flows().iter().enumerate() {
            if next_fragment[index] != flow.binding().terminal().fragment_count() {
                return Err(StagingFootnotePaginationError::MissingDefinitionPaint(
                    flow.binding().footnote_id().clone(),
                ));
            }
        }

        let canonical_jcs = encode_footnote_selected_layout(
            registry,
            body_layout_fingerprint,
            &pages,
            &self.assignments,
        );
        let fingerprint = footnote_selected_layout_fingerprint_from_jcs(&canonical_jcs);
        Ok(ValidatedFootnoteSelectedLayout {
            profile: self.profile,
            registry: self.registry,
            epoch: self.epoch,
            body_layout_fingerprint,
            master_id: registry.master_id().clone(),
            body_frame: registry.body_frame(),
            maximum_footnote_frame: registry.maximum_footnote_frame(),
            assignments: self.assignments,
            pages,
            fingerprint,
            canonical_jcs,
        })
    }
}

/// Immutable ADR-0030 selected-state closure shared by Display, trace,
/// manifest, and PDF validation.
#[derive(Debug)]
pub struct ValidatedFootnoteSelectedLayout {
    profile: FootnoteProfileFingerprint,
    registry: FootnoteFlowRegistryFingerprint,
    epoch: LayoutEpoch,
    body_layout_fingerprint: LayoutStateFingerprint,
    master_id: MasterId,
    body_frame: Rect,
    maximum_footnote_frame: Rect,
    assignments: Vec<StagingFootnoteAssignment>,
    pages: Vec<StagingFootnoteSelectedPageReceipt>,
    fingerprint: FootnoteSelectedLayoutFingerprint,
    canonical_jcs: String,
}

impl ValidatedFootnoteSelectedLayout {
    pub const fn profile_fingerprint(&self) -> FootnoteProfileFingerprint {
        self.profile
    }
    pub const fn registry_fingerprint(&self) -> FootnoteFlowRegistryFingerprint {
        self.registry
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn body_layout_fingerprint(&self) -> LayoutStateFingerprint {
        self.body_layout_fingerprint
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn body_frame(&self) -> Rect {
        self.body_frame
    }
    pub const fn maximum_footnote_frame(&self) -> Rect {
        self.maximum_footnote_frame
    }
    pub fn assignments(&self) -> &[StagingFootnoteAssignment] {
        &self.assignments
    }
    pub fn pages(&self) -> &[StagingFootnoteSelectedPageReceipt] {
        &self.pages
    }
    pub const fn fingerprint(&self) -> FootnoteSelectedLayoutFingerprint {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

fn footnote_body_start_fingerprint(
    body_layout: LayoutStateFingerprint,
    page_index: u32,
    start: &FlowPosition,
) -> LayoutStateFingerprint {
    let mut jcs = String::from(
        "{\"algorithm\":\"typaxis.footnote-body-page-start/1\",\"body_layout_sha256\":",
    );
    push_hex(&mut jcs, body_layout.bytes());
    jcs.push_str(",\"flow_ordinal\":");
    jcs.push_str(&start.global_flow_ordinal().to_string());
    jcs.push_str(",\"owner_node_id\":");
    jcs.push_str(&start.owner().get().to_string());
    jcs.push_str(",\"page_index\":");
    jcs.push_str(&page_index.to_string());
    jcs.push('}');
    LayoutStateFingerprint::from_untrusted_bytes(sha256(jcs.as_bytes()))
}

/// Runs evaluation zero plus at most the configured inclusive number of
/// charged body reevaluations. The body callback is never invoked for max+1.
pub fn evaluate_staging_footnote_page<F>(
    registry: &StagingFootnoteFlowRegistry,
    state: &StagingFootnotePaginationState,
    input: StagingFootnotePageInput,
    mut evaluate_body: F,
) -> Result<ValidatedFootnoteConvergenceReceipt, StagingFootnotePaginationError>
where
    F: FnMut(
        &StagingFootnotePageEvaluationRequest,
    ) -> Result<StagingFootnoteBodyCandidate, StagingFootnotePaginationError>,
{
    validate_footnote_registry_state(registry, state)?;
    if input.page_index != state.next_page_index
        || input.page_index >= state.max_pages
        || state.max_footnote_reflows_per_page == 0
    {
        return Err(StagingFootnotePaginationError::InvalidPageInput);
    }
    let state_before_sha256 = encode_footnote_state_sha256(state);
    let carry_seed =
        fragment_footnote_ordered_set(registry, state, &state.carry_assignments())?.reservation;
    let initial_candidate = run_footnote_body_evaluation(
        registry,
        state,
        input,
        0,
        carry_seed,
        None,
        &mut evaluate_body,
    )?;
    let mut previous = derive_footnote_evaluation(
        registry,
        state,
        input.page_index,
        input.body_page_start,
        &initial_candidate,
    )?;
    let mut history = BTreeSet::new();
    history.insert(previous.fingerprint);

    for evaluation_index in 1..=state.max_footnote_reflows_per_page {
        let current_candidate = run_footnote_body_evaluation(
            registry,
            state,
            input,
            evaluation_index,
            previous.reservation,
            previous.body_cut_before_reference_owner,
            &mut evaluate_body,
        )?;
        let current = derive_footnote_evaluation(
            registry,
            state,
            input.page_index,
            input.body_page_start,
            &current_candidate,
        )?;
        if current.fingerprint == previous.fingerprint
            && current_candidate.applied_reservation == current.reservation
            && current_candidate.applied_body_cut_before_reference_owner
                == current.body_cut_before_reference_owner
            && current_candidate.available_body_block_size
                == available_body_block_size(registry.body_frame(), current.reservation)?
        {
            let mut receipt = ValidatedFootnoteConvergenceReceipt {
                profile: state.profile,
                registry: state.registry,
                epoch: state.epoch,
                global_pass: state.global_pass,
                page_index: input.page_index,
                body_page_start: input.body_page_start,
                state_before_sha256,
                evaluation_index,
                previous_evaluation: previous.fingerprint,
                final_candidate: current_candidate,
                final_evaluation: current,
                canonical_jcs: String::new(),
            };
            receipt.canonical_jcs = encode_footnote_convergence(&receipt);
            return Ok(receipt);
        }
        if history.contains(&current.fingerprint) {
            return Err(StagingFootnotePaginationError::ReflowOscillation);
        }
        if evaluation_index == state.max_footnote_reflows_per_page {
            return Err(StagingFootnotePaginationError::ReflowLimit);
        }
        history.insert(current.fingerprint);
        previous = current;
    }
    Err(StagingFootnotePaginationError::ReflowLimit)
}

fn run_footnote_body_evaluation<F>(
    registry: &StagingFootnoteFlowRegistry,
    state: &StagingFootnotePaginationState,
    input: StagingFootnotePageInput,
    evaluation_index: u16,
    applied_reservation: NonNegativeLength,
    body_cut_before_reference_owner: Option<NodeId>,
    evaluate_body: &mut F,
) -> Result<StagingFootnoteBodyCandidate, StagingFootnotePaginationError>
where
    F: FnMut(
        &StagingFootnotePageEvaluationRequest,
    ) -> Result<StagingFootnoteBodyCandidate, StagingFootnotePaginationError>,
{
    let available_body_block_size =
        available_body_block_size(registry.body_frame(), applied_reservation)?;
    let request = StagingFootnotePageEvaluationRequest {
        global_pass: state.global_pass,
        page_index: input.page_index,
        evaluation_index,
        master_id: registry.master_id().clone(),
        body_page_start: input.body_page_start,
        applied_reservation,
        body_cut_before_reference_owner,
        available_body_block_size,
    };
    let candidate = evaluate_body(&request)?;
    if candidate.applied_reservation != request.applied_reservation
        || candidate.applied_body_cut_before_reference_owner
            != request.body_cut_before_reference_owner
        || candidate.available_body_block_size != request.available_body_block_size
        || u64::try_from(candidate.reference_owners.len())
            .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?
            > state.max_ast_nodes
    {
        return Err(StagingFootnotePaginationError::InvalidBodyCandidate);
    }
    if let Some(cut_owner) = body_cut_before_reference_owner {
        let cut = registry.reference(cut_owner).ok_or(
            StagingFootnotePaginationError::UnknownReferenceOwner(cut_owner),
        )?;
        for owner in &candidate.reference_owners {
            let Some(reference) = registry.reference(*owner) else {
                continue;
            };
            if reference.logical_ordinal() >= cut.logical_ordinal() {
                return Err(StagingFootnotePaginationError::DefinitionOversize(
                    cut.footnote_id().clone(),
                ));
            }
        }
    }
    Ok(candidate)
}

fn derive_footnote_evaluation(
    registry: &StagingFootnoteFlowRegistry,
    state: &StagingFootnotePaginationState,
    page_index: u32,
    body_page_start: LayoutStateFingerprint,
    candidate: &StagingFootnoteBodyCandidate,
) -> Result<StagingFootnoteEvaluationReceipt, StagingFootnotePaginationError> {
    let mut discovery = Vec::new();
    discovery
        .try_reserve_exact(candidate.reference_owners.len())
        .map_err(|_| StagingFootnotePaginationError::AllocationFailure)?;
    let mut previous_logical_ordinal = state.last_reference_logical_ordinal;
    for owner in &candidate.reference_owners {
        if state.seen_reference_owners.contains(owner) {
            return Err(StagingFootnotePaginationError::DuplicateReferenceOccurrence(*owner));
        }
        let reference = registry.reference(*owner).ok_or(
            StagingFootnotePaginationError::UnknownReferenceOwner(*owner),
        )?;
        if previous_logical_ordinal.is_some_and(|previous| previous >= reference.logical_ordinal())
        {
            return Err(StagingFootnotePaginationError::NonCanonicalReferenceOrder(
                *owner,
            ));
        }
        previous_logical_ordinal = Some(reference.logical_ordinal());
        discovery.push(StagingPageFootnoteReferenceOccurrence {
            reference_owner: *owner,
            footnote_id: reference.footnote_id().clone(),
            document_logical_ordinal: reference.logical_ordinal(),
        });
    }

    let mut ordered_footnotes = state.carry_assignments();
    let mut active_ids: BTreeSet<_> = ordered_footnotes
        .iter()
        .map(|assignment| assignment.footnote_id.clone())
        .collect();
    let mut candidate_assigned: BTreeSet<FootnoteId> = BTreeSet::new();
    for occurrence in &discovery {
        if state.assignment(&occurrence.footnote_id).is_some()
            || !candidate_assigned.insert(occurrence.footnote_id.clone())
        {
            continue;
        }
        if !active_ids.insert(occurrence.footnote_id.clone()) {
            continue;
        }
        let flow = registry
            .flow_by_footnote_id(&occurrence.footnote_id)
            .ok_or_else(|| {
                StagingFootnotePaginationError::MissingDefinition(occurrence.footnote_id.clone())
            })?;
        let assignment_ordinal = u32::try_from(state.assignments.len())
            .ok()
            .and_then(|base| {
                let new_count = ordered_footnotes
                    .iter()
                    .filter(|assignment| state.assignment(&assignment.footnote_id).is_none())
                    .count();
                u32::try_from(new_count)
                    .ok()
                    .and_then(|offset| base.checked_add(offset))
            })
            .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
        ordered_footnotes.push(StagingFootnoteAssignment {
            footnote_id: occurrence.footnote_id.clone(),
            flow_id: flow.binding().flow_id(),
            assignment_ordinal,
            first_reference_owner: occurrence.reference_owner,
        });
    }

    let fragmented = fragment_footnote_ordered_set(registry, state, &ordered_footnotes)?;
    ordered_footnotes.truncate(fragmented.accepted_assignment_count);
    let body_cut_before_reference_owner = fragmented
        .deferred_reference_owner
        .or(candidate.applied_body_cut_before_reference_owner);
    let prospective = state
        .selected_record_count
        .checked_add(candidate.selected_body_fragment_count)
        .and_then(|count| count.checked_add(fragmented.selected_record_count))
        .filter(|count| *count <= state.max_fragments)
        .ok_or(StagingFootnotePaginationError::FragmentLimit)?;
    let _ = prospective;
    let canonical_jcs = encode_footnote_evaluation(
        registry,
        state,
        page_index,
        body_page_start,
        candidate,
        &discovery,
        &ordered_footnotes,
        &fragmented.flows,
        fragmented.reservation,
        body_cut_before_reference_owner,
    );
    let fingerprint = footnote_page_evaluation_fingerprint_from_jcs(&canonical_jcs);
    Ok(StagingFootnoteEvaluationReceipt {
        discovery,
        ordered_footnotes,
        flows: fragmented.flows,
        reservation: fragmented.reservation,
        body_cut_before_reference_owner,
        selected_record_count: fragmented.selected_record_count,
        fingerprint,
        canonical_jcs,
    })
}

#[derive(Debug)]
struct FragmentedFootnoteSet {
    flows: Vec<StagingFootnoteFlowEvaluationReceipt>,
    reservation: NonNegativeLength,
    accepted_assignment_count: usize,
    deferred_reference_owner: Option<NodeId>,
    selected_record_count: u64,
}

fn fragment_footnote_ordered_set(
    registry: &StagingFootnoteFlowRegistry,
    state: &StagingFootnotePaginationState,
    ordered: &[StagingFootnoteAssignment],
) -> Result<FragmentedFootnoteSet, StagingFootnotePaginationError> {
    if ordered.is_empty() {
        return Ok(FragmentedFootnoteSet {
            flows: Vec::new(),
            reservation: NonNegativeLength::ZERO,
            accepted_assignment_count: 0,
            deferred_reference_owner: None,
            selected_record_count: 0,
        });
    }
    let maximum = registry.maximum_footnote_frame().height().get().raw();
    let content_capacity = maximum
        .checked_sub(FOOTNOTE_SEPARATOR_BAND_RAW)
        .filter(|capacity| *capacity > 0)
        .ok_or_else(|| {
            StagingFootnotePaginationError::DefinitionOversize(ordered[0].footnote_id.clone())
        })?;
    let mut cursors = Vec::new();
    cursors
        .try_reserve_exact(ordered.len())
        .map_err(|_| StagingFootnotePaginationError::AllocationFailure)?;
    let mut minimum_sum = 0i64;
    let mut accepted_assignment_count = ordered.len();
    let mut deferred_reference_owner = None;
    for (index, assignment) in ordered.iter().enumerate() {
        let flow = registry
            .flow(assignment.flow_id)
            .filter(|flow| flow.binding().footnote_id() == &assignment.footnote_id)
            .ok_or_else(|| {
                StagingFootnotePaginationError::MissingDefinition(assignment.footnote_id.clone())
            })?;
        let incoming = state.carry(assignment.assignment_ordinal);
        let next = incoming
            .map(|carry| carry.next_cursor.next_fragment_ordinal)
            .unwrap_or(0);
        if incoming.is_some_and(|carry| {
            carry.assignment != *assignment
                || carry.next_cursor.flow_id != assignment.flow_id
                || carry.target_page_index != state.next_page_index
        }) {
            return Err(StagingFootnotePaginationError::InvalidFootnoteCursor(
                assignment.flow_id,
            ));
        }
        let extent = flow.fragment_extents().get(next as usize).ok_or(
            StagingFootnotePaginationError::InvalidFootnoteCursor(assignment.flow_id),
        )?;
        if extent.get().raw() > content_capacity {
            return Err(StagingFootnotePaginationError::DefinitionOversize(
                assignment.footnote_id.clone(),
            ));
        }
        let prospective_minimum_sum = minimum_sum
            .checked_add(extent.get().raw())
            .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
        if prospective_minimum_sum > content_capacity {
            if incoming.is_some() {
                return Err(StagingFootnotePaginationError::DefinitionOversize(
                    assignment.footnote_id.clone(),
                ));
            }
            accepted_assignment_count = index;
            deferred_reference_owner = Some(assignment.first_reference_owner);
            break;
        }
        minimum_sum = prospective_minimum_sum;
        cursors.push(next);
    }

    let ordered = &ordered[..accepted_assignment_count];

    let mut selected_counts = Vec::new();
    selected_counts
        .try_reserve_exact(ordered.len())
        .map_err(|_| StagingFootnotePaginationError::AllocationFailure)?;
    selected_counts.resize(ordered.len(), 1u32);
    let mut remaining = content_capacity
        .checked_sub(minimum_sum)
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    for (index, assignment) in ordered.iter().enumerate() {
        let flow = registry.flow(assignment.flow_id).ok_or_else(|| {
            StagingFootnotePaginationError::MissingDefinition(assignment.footnote_id.clone())
        })?;
        let mut next = cursors[index]
            .checked_add(1)
            .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
        while let Some(extent) = flow.fragment_extents().get(next as usize) {
            if extent.get().raw() > remaining {
                break;
            }
            remaining = remaining
                .checked_sub(extent.get().raw())
                .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
            selected_counts[index] = selected_counts[index]
                .checked_add(1)
                .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
            next = next
                .checked_add(1)
                .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
        }
    }

    let mut flows = Vec::new();
    flows
        .try_reserve_exact(ordered.len())
        .map_err(|_| StagingFootnotePaginationError::AllocationFailure)?;
    let mut selected_extent = 0i64;
    let mut fragment_count = 0u64;
    for ((assignment, before_ordinal), selected_count) in
        ordered.iter().zip(cursors).zip(selected_counts)
    {
        let flow = registry.flow(assignment.flow_id).ok_or_else(|| {
            StagingFootnotePaginationError::MissingDefinition(assignment.footnote_id.clone())
        })?;
        let after_ordinal = before_ordinal
            .checked_add(selected_count)
            .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
        if after_ordinal > flow.binding().terminal().fragment_count() {
            return Err(StagingFootnotePaginationError::InvalidFootnoteCursor(
                assignment.flow_id,
            ));
        }
        let selected = &flow.fragment_extents()[before_ordinal as usize..after_ordinal as usize];
        let mut fragments = Vec::new();
        fragments
            .try_reserve_exact(selected.len())
            .map_err(|_| StagingFootnotePaginationError::AllocationFailure)?;
        for (offset, extent) in selected.iter().enumerate() {
            let fragment_ordinal = before_ordinal
                .checked_add(
                    u32::try_from(offset)
                        .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?,
                )
                .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
            selected_extent = selected_extent
                .checked_add(extent.get().raw())
                .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
            fragment_count = fragment_count
                .checked_add(1)
                .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
            fragments.push(StagingFootnoteFragmentReceipt {
                fragment_ordinal,
                block_extent: *extent,
            });
        }
        let incoming_source_page = state
            .carry(assignment.assignment_ordinal)
            .map(|carry| carry.source_page_index);
        flows.push(StagingFootnoteFlowEvaluationReceipt {
            assignment: assignment.clone(),
            incoming_source_page,
            before_cursor: StagingFootnoteFlowCursor::new(assignment.flow_id, before_ordinal),
            after_cursor: StagingFootnoteFlowCursor::new(assignment.flow_id, after_ordinal),
            fragments,
            carries_out: after_ordinal < flow.binding().terminal().fragment_count(),
        });
    }
    let reservation_raw = FOOTNOTE_SEPARATOR_BAND_RAW
        .checked_add(selected_extent)
        .filter(|reservation| *reservation <= maximum)
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    let reservation = Length::from_raw(reservation_raw)
        .and_then(NonNegativeLength::new)
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    let assignment_records = u64::try_from(ordered.len())
        .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?;
    let selected_record_count = assignment_records
        .checked_add(1)
        .and_then(|count| count.checked_add(fragment_count))
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    Ok(FragmentedFootnoteSet {
        flows,
        reservation,
        accepted_assignment_count,
        deferred_reference_owner,
        selected_record_count,
    })
}

impl StagingFootnotePaginationState {
    fn carry_assignments(&self) -> Vec<StagingFootnoteAssignment> {
        self.carries
            .iter()
            .map(|carry| carry.assignment.clone())
            .collect()
    }
}

fn validate_footnote_registry_state(
    registry: &StagingFootnoteFlowRegistry,
    state: &StagingFootnotePaginationState,
) -> Result<(), StagingFootnotePaginationError> {
    if registry.receipt().profile_fingerprint() != state.profile
        || registry.receipt().fingerprint() != state.registry
        || registry.receipt().epoch() != state.epoch
        || state.assignment_by_footnote.len() != state.assignments.len()
    {
        return Err(StagingFootnotePaginationError::RegistryMismatch);
    }
    for (index, assignment) in state.assignments.iter().enumerate() {
        let expected =
            u32::try_from(index).map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?;
        let flow = registry.flow(assignment.flow_id).ok_or_else(|| {
            StagingFootnotePaginationError::MissingDefinition(assignment.footnote_id.clone())
        })?;
        if assignment.assignment_ordinal != expected
            || flow.binding().footnote_id() != &assignment.footnote_id
            || state.assignment_by_footnote.get(&assignment.footnote_id) != Some(&index)
        {
            return Err(StagingFootnotePaginationError::StateMismatch);
        }
    }
    let mut previous_assignment = None;
    for carry in &state.carries {
        let Some(assignment) = state.assignment(&carry.assignment.footnote_id) else {
            return Err(StagingFootnotePaginationError::StateMismatch);
        };
        if assignment != &carry.assignment
            || carry.target_page_index != state.next_page_index
            || carry.source_page_index.checked_add(1) != Some(carry.target_page_index)
            || previous_assignment.is_some_and(|previous| previous >= assignment.assignment_ordinal)
        {
            return Err(StagingFootnotePaginationError::StateMismatch);
        }
        previous_assignment = Some(assignment.assignment_ordinal);
    }
    Ok(())
}

fn available_body_block_size(
    body_frame: Rect,
    reservation: NonNegativeLength,
) -> Result<PositiveLength, StagingFootnotePaginationError> {
    body_frame
        .height()
        .get()
        .checked_sub(reservation.get())
        .and_then(PositiveLength::new)
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)
}

fn selected_footnote_frame(
    maximum_frame: Rect,
    reservation: NonNegativeLength,
) -> Result<Option<Rect>, StagingFootnotePaginationError> {
    if reservation == NonNegativeLength::ZERO {
        return Ok(None);
    }
    let height = PositiveLength::new(reservation.get())
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    if height.get().raw() > maximum_frame.height().get().raw() {
        return Err(StagingFootnotePaginationError::StateMismatch);
    }
    let block_end = maximum_frame
        .y()
        .checked_add(maximum_frame.height().get())
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    let y = block_end
        .checked_sub(height.get())
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    Ok(Some(Rect::new(
        maximum_frame.x(),
        y,
        maximum_frame.width(),
        height,
    )))
}

#[allow(clippy::too_many_arguments)]
fn encode_footnote_evaluation(
    registry: &StagingFootnoteFlowRegistry,
    state: &StagingFootnotePaginationState,
    page_index: u32,
    body_page_start: LayoutStateFingerprint,
    candidate: &StagingFootnoteBodyCandidate,
    discovery: &[StagingPageFootnoteReferenceOccurrence],
    ordered: &[StagingFootnoteAssignment],
    flows: &[StagingFootnoteFlowEvaluationReceipt],
    reservation: NonNegativeLength,
    body_cut_before_reference_owner: Option<NodeId>,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, FootnotePageEvaluationFingerprint::ALGORITHM_ID);
    output.push_str(",\"body_candidate_sha256\":");
    push_hex(&mut output, candidate.body_fingerprint.bytes());
    output.push_str(",\"body_continuation\":");
    encode_footnote_body_continuation(&mut output, candidate.continuation);
    output.push_str(",\"body_cut_before_reference_owner\":");
    match body_cut_before_reference_owner {
        Some(owner) => output.push_str(&owner.get().to_string()),
        None => output.push_str("null"),
    }
    output.push_str(",\"body_fragment_count\":");
    output.push_str(&candidate.selected_body_fragment_count.to_string());
    output.push_str(",\"body_page_start_sha256\":");
    push_hex(&mut output, body_page_start.bytes());
    output.push_str(",\"body_registry_sha256\":");
    push_hex(
        &mut output,
        registry.receipt().body_flow_registry_fingerprint().bytes(),
    );
    output.push_str(",\"discovery\":[");
    for (index, occurrence) in discovery.iter().enumerate() {
        comma(&mut output, index);
        output.push_str("{\"document_logical_ordinal\":");
        output.push_str(&occurrence.document_logical_ordinal.to_string());
        output.push_str(",\"footnote_id\":");
        push_jcs_string(&mut output, occurrence.footnote_id.as_str());
        output.push_str(",\"reference_owner\":");
        output.push_str(&occurrence.reference_owner.get().to_string());
        output.push('}');
    }
    output.push_str("],\"flows\":[");
    for (index, flow) in flows.iter().enumerate() {
        comma(&mut output, index);
        encode_footnote_flow_evaluation(&mut output, flow, page_index);
    }
    output.push_str("],\"footnote_registry_sha256\":");
    push_hex(&mut output, state.registry.bytes());
    output.push_str(",\"global_pass\":");
    output.push_str(&state.global_pass.to_string());
    output.push_str(",\"layout_epoch\":");
    encode_layout_epoch(&mut output, state.epoch);
    output.push_str(",\"master_id\":");
    push_jcs_string(&mut output, registry.master_id().as_str());
    output.push_str(",\"ordered_footnotes\":[");
    for (index, assignment) in ordered.iter().enumerate() {
        comma(&mut output, index);
        encode_footnote_assignment(&mut output, assignment);
    }
    output.push_str("],\"package_sha256\":");
    push_hex(
        &mut output,
        registry.receipt().package_fingerprint().bytes(),
    );
    output.push_str(",\"page_index\":");
    output.push_str(&page_index.to_string());
    output.push_str(",\"profile\":\"typaxis.machine-pdf/footnote-1\",\"profile_receipt_sha256\":");
    push_hex(&mut output, state.profile.bytes());
    output.push_str(",\"reservation\":");
    output.push_str(&reservation.get().raw().to_string());
    output.push('}');
    output
}

fn encode_footnote_assignment(output: &mut String, assignment: &StagingFootnoteAssignment) {
    output.push_str("{\"assignment_ordinal\":");
    output.push_str(&assignment.assignment_ordinal.to_string());
    output.push_str(",\"first_reference_owner\":");
    output.push_str(&assignment.first_reference_owner.get().to_string());
    output.push_str(",\"flow_id\":");
    output.push_str(&assignment.flow_id.get().to_string());
    output.push_str(",\"footnote_id\":");
    push_jcs_string(output, assignment.footnote_id.as_str());
    output.push('}');
}

fn encode_footnote_flow_evaluation(
    output: &mut String,
    flow: &StagingFootnoteFlowEvaluationReceipt,
    page_index: u32,
) {
    output.push_str("{\"after_cursor\":");
    encode_footnote_cursor(output, flow.after_cursor);
    output.push_str(",\"assignment\":");
    encode_footnote_assignment(output, &flow.assignment);
    output.push_str(",\"before_cursor\":");
    encode_footnote_cursor(output, flow.before_cursor);
    output.push_str(",\"carry_out_target_page\":");
    if flow.carries_out {
        match page_index.checked_add(1) {
            Some(target) => output.push_str(&target.to_string()),
            None => output.push_str("null"),
        }
    } else {
        output.push_str("null");
    }
    output.push_str(",\"fragments\":[");
    for (index, fragment) in flow.fragments.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"block_extent\":");
        output.push_str(&fragment.block_extent.get().raw().to_string());
        output.push_str(",\"fragment_ordinal\":");
        output.push_str(&fragment.fragment_ordinal.to_string());
        output.push('}');
    }
    output.push_str("],\"incoming_source_page\":");
    match flow.incoming_source_page {
        Some(page) => output.push_str(&page.to_string()),
        None => output.push_str("null"),
    }
    output.push('}');
}

fn encode_footnote_cursor(output: &mut String, cursor: StagingFootnoteFlowCursor) {
    output.push_str("{\"flow_id\":");
    output.push_str(&cursor.flow_id.get().to_string());
    output.push_str(",\"next_fragment_ordinal\":");
    output.push_str(&cursor.next_fragment_ordinal.to_string());
    output.push('}');
}

fn encode_footnote_body_continuation(
    output: &mut String,
    continuation: StagingFootnoteBodyContinuation,
) {
    output.push_str("{\"next_flow_position\":");
    output.push_str(&continuation.next_flow_position.to_string());
    output.push_str(",\"terminal\":");
    output.push_str(if continuation.terminal {
        "true"
    } else {
        "false"
    });
    output.push('}');
}

fn encode_footnote_selected_layout(
    registry: &StagingFootnoteFlowRegistry,
    body_layout_fingerprint: LayoutStateFingerprint,
    pages: &[StagingFootnoteSelectedPageReceipt],
    assignments: &[StagingFootnoteAssignment],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, FootnoteSelectedLayoutFingerprint::ALGORITHM_ID);
    output.push_str(",\"assignments\":[");
    for (index, assignment) in assignments.iter().enumerate() {
        comma(&mut output, index);
        encode_footnote_assignment(&mut output, assignment);
    }
    output.push_str("],\"body_frame\":");
    encode_selected_footnote_rect(&mut output, registry.body_frame());
    output.push_str(",\"body_layout_sha256\":");
    push_hex(&mut output, body_layout_fingerprint.bytes());
    output.push_str(",\"footnote_registry_sha256\":");
    push_hex(&mut output, registry.receipt().fingerprint().bytes());
    output.push_str(",\"layout_epoch\":");
    encode_layout_epoch(&mut output, registry.receipt().epoch());
    output.push_str(",\"master_id\":");
    push_jcs_string(&mut output, registry.master_id().as_str());
    output.push_str(",\"maximum_footnote_frame\":");
    encode_selected_footnote_rect(&mut output, registry.maximum_footnote_frame());
    output.push_str(",\"package_sha256\":");
    push_hex(
        &mut output,
        registry.receipt().package_fingerprint().bytes(),
    );
    output.push_str(",\"pages\":[");
    for (index, page) in pages.iter().enumerate() {
        comma(&mut output, index);
        output.push_str("{\"body_continuation\":");
        encode_footnote_body_continuation(&mut output, page.body_continuation);
        output.push_str(",\"body_cut_before_reference_owner\":");
        match page.body_cut_before_reference_owner {
            Some(owner) => output.push_str(&owner.get().to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"body_fingerprint\":");
        push_hex(&mut output, page.body_fingerprint.bytes());
        output.push_str(",\"body_page_start_sha256\":");
        push_hex(&mut output, page.body_page_start.bytes());
        output.push_str(",\"discovery\":[");
        for (discovery_index, occurrence) in page.discovery.iter().enumerate() {
            comma(&mut output, discovery_index);
            output.push_str("{\"document_logical_ordinal\":");
            output.push_str(&occurrence.document_logical_ordinal.to_string());
            output.push_str(",\"footnote_id\":");
            push_jcs_string(&mut output, occurrence.footnote_id.as_str());
            output.push_str(",\"reference_owner\":");
            output.push_str(&occurrence.reference_owner.get().to_string());
            output.push('}');
        }
        output.push_str("],\"evaluation_count\":");
        output.push_str(&page.evaluation_count.to_string());
        output.push_str(",\"evaluation_sha256\":");
        push_hex(&mut output, page.evaluation_fingerprint.bytes());
        output.push_str(",\"flows\":[");
        for (flow_index, flow) in page.flows.iter().enumerate() {
            comma(&mut output, flow_index);
            encode_footnote_flow_evaluation(&mut output, flow, page.page_index);
        }
        output.push_str("],\"global_pass\":");
        output.push_str(&page.global_pass.to_string());
        output.push_str(",\"ordered_footnotes\":[");
        for (assignment_index, assignment) in page.ordered_footnotes.iter().enumerate() {
            comma(&mut output, assignment_index);
            encode_footnote_assignment(&mut output, assignment);
        }
        output.push_str("],\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"reservation\":");
        output.push_str(&page.reservation.get().raw().to_string());
        output.push('}');
    }
    output.push_str("],\"profile\":\"typaxis.machine-pdf/footnote-1\",\"profile_receipt_sha256\":");
    push_hex(
        &mut output,
        registry.receipt().profile_fingerprint().bytes(),
    );
    output.push('}');
    output
}

fn encode_selected_footnote_rect(output: &mut String, rect: Rect) {
    output.push_str("{\"height\":");
    output.push_str(&rect.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&rect.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&rect.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&rect.y().raw().to_string());
    output.push('}');
}

fn encode_footnote_convergence(receipt: &ValidatedFootnoteConvergenceReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, FOOTNOTE_CONVERGENCE_RECEIPT_ALGORITHM);
    output.push_str(",\"body_page_start_sha256\":");
    push_hex(&mut output, receipt.body_page_start.bytes());
    output.push_str(",\"evaluation_count\":");
    output.push_str(&receipt.evaluation_count().to_string());
    output.push_str(",\"evaluation_index\":");
    output.push_str(&receipt.evaluation_index.to_string());
    output.push_str(",\"footnote_registry_sha256\":");
    push_hex(&mut output, receipt.registry.bytes());
    output.push_str(",\"global_pass\":");
    output.push_str(&receipt.global_pass.to_string());
    output.push_str(",\"layout_epoch\":");
    encode_layout_epoch(&mut output, receipt.epoch);
    output.push_str(",\"page_evaluation\":");
    output.push_str(&receipt.final_evaluation.canonical_jcs);
    output.push_str(",\"page_evaluation_sha256\":");
    push_hex(&mut output, receipt.final_evaluation.fingerprint.bytes());
    output.push_str(",\"page_index\":");
    output.push_str(&receipt.page_index.to_string());
    output.push_str(",\"previous_evaluation_sha256\":");
    push_hex(&mut output, receipt.previous_evaluation.bytes());
    output.push_str(",\"profile_receipt_sha256\":");
    push_hex(&mut output, receipt.profile.bytes());
    output.push_str(",\"state_before_sha256\":");
    push_hex(&mut output, receipt.state_before_sha256);
    output.push('}');
    output
}

fn encode_footnote_state_sha256(state: &StagingFootnotePaginationState) -> [u8; 32] {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, FOOTNOTE_PAGINATION_STATE_ALGORITHM);
    output.push_str(",\"assignments\":[");
    for (index, assignment) in state.assignments.iter().enumerate() {
        comma(&mut output, index);
        encode_footnote_assignment(&mut output, assignment);
    }
    output.push_str("],\"carries\":[");
    for (index, carry) in state.carries.iter().enumerate() {
        comma(&mut output, index);
        output.push_str("{\"assignment\":");
        encode_footnote_assignment(&mut output, &carry.assignment);
        output.push_str(",\"next_cursor\":");
        encode_footnote_cursor(&mut output, carry.next_cursor);
        output.push_str(",\"source_page_index\":");
        output.push_str(&carry.source_page_index.to_string());
        output.push_str(",\"target_page_index\":");
        output.push_str(&carry.target_page_index.to_string());
        output.push('}');
    }
    output.push_str("],\"epoch\":");
    encode_layout_epoch(&mut output, state.epoch);
    output.push_str(",\"global_pass\":");
    output.push_str(&state.global_pass.to_string());
    output.push_str(",\"last_reference_logical_ordinal\":");
    match state.last_reference_logical_ordinal {
        Some(ordinal) => output.push_str(&ordinal.to_string()),
        None => output.push_str("null"),
    }
    output.push_str(",\"max_ast_nodes\":");
    output.push_str(&state.max_ast_nodes.to_string());
    output.push_str(",\"max_footnote_reflows_per_page\":");
    output.push_str(&state.max_footnote_reflows_per_page.to_string());
    output.push_str(",\"max_fragments\":");
    output.push_str(&state.max_fragments.to_string());
    output.push_str(",\"max_pages\":");
    output.push_str(&state.max_pages.to_string());
    output.push_str(",\"next_page_index\":");
    output.push_str(&state.next_page_index.to_string());
    output.push_str(",\"profile_receipt_sha256\":");
    push_hex(&mut output, state.profile.bytes());
    output.push_str(",\"registry_sha256\":");
    push_hex(&mut output, state.registry.bytes());
    output.push_str(",\"selected_page_fingerprints\":[");
    for (index, fingerprint) in state.selected_page_fingerprints.iter().enumerate() {
        comma(&mut output, index);
        push_hex(&mut output, fingerprint.bytes());
    }
    output.push_str("],\"selected_record_count\":");
    output.push_str(&state.selected_record_count.to_string());
    output.push_str(",\"seen_reference_owners\":[");
    for (index, owner) in state.seen_reference_owners.iter().enumerate() {
        comma(&mut output, index);
        output.push_str(&owner.get().to_string());
    }
    output.push(']');
    output.push('}');
    sha256(output.as_bytes())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackPolicy {
    LowestCostThenEarliest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaginationOptions {
    max_pages: u32,
    max_layout_passes: u16,
    max_page_break_lookback: u16,
    max_fragments: u64,
    max_footnote_reflows_per_page: u16,
    max_column_balance_candidates: u16,
    max_float_queue: u32,
    max_float_carry_pages: u16,
    strict: bool,
}
impl PaginationOptions {
    pub fn from_limits(limits: &ValidatedResourceLimits, strict: bool) -> Self {
        let limits = limits.get();
        Self {
            max_pages: limits.max_pages,
            max_layout_passes: limits.max_layout_passes,
            max_page_break_lookback: limits.max_page_break_lookback,
            max_fragments: limits.max_fragments,
            max_footnote_reflows_per_page: limits.max_footnote_reflows_per_page,
            max_column_balance_candidates: limits.max_column_balance_candidates,
            max_float_queue: limits.max_float_queue,
            max_float_carry_pages: limits.max_float_carry_pages,
            strict,
        }
    }
    pub const fn fallback_policy(&self) -> FallbackPolicy {
        FallbackPolicy::LowestCostThenEarliest
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PaginationWorkBudget {
    session: PaginationSessionId,
    initial_fingerprint: LayoutStateFingerprint,
    layout_epoch: LayoutEpoch,
    next_pass_index: u16,
    max_pages: u32,
    remaining_layout_passes: u16,
    max_fragments: u64,
    max_page_break_lookback: u16,
    max_footnote_reflows_per_page: u16,
    max_column_balance_candidates: u16,
    max_float_queue: u32,
    max_float_carry_pages: u16,
    footnote_reflows: BTreeMap<(u16, u32), u16>,
    column_candidates: BTreeMap<(u16, NodeId), u16>,
    queued_floats: BTreeSet<(u16, NodeId, u32)>,
    dequeued_floats: BTreeSet<(u16, NodeId, u32)>,
    float_carries: BTreeMap<(u16, NodeId, u32), u16>,
    completed_passes: Vec<PassBudgetSummary>,
}
impl PaginationWorkBudget {
    fn new(
        session: PaginationSessionId,
        options: PaginationOptions,
        initial_fingerprint: LayoutStateFingerprint,
        layout_epoch: LayoutEpoch,
    ) -> Self {
        Self {
            session,
            initial_fingerprint,
            layout_epoch,
            next_pass_index: 0,
            max_pages: options.max_pages,
            remaining_layout_passes: options.max_layout_passes,
            max_fragments: options.max_fragments,
            max_page_break_lookback: options.max_page_break_lookback,
            max_footnote_reflows_per_page: options.max_footnote_reflows_per_page,
            max_column_balance_candidates: options.max_column_balance_candidates,
            max_float_queue: options.max_float_queue,
            max_float_carry_pages: options.max_float_carry_pages,
            footnote_reflows: BTreeMap::new(),
            column_candidates: BTreeMap::new(),
            queued_floats: BTreeSet::new(),
            dequeued_floats: BTreeSet::new(),
            float_carries: BTreeMap::new(),
            completed_passes: Vec::new(),
        }
    }
    fn consume_layout_pass(&mut self) -> Result<(), PaginationError> {
        self.remaining_layout_passes = self
            .remaining_layout_passes
            .checked_sub(1)
            .ok_or(PaginationError::ResourceLimit)?;
        Ok(())
    }
    pub fn begin_pass<'a>(
        &'a mut self,
        pass_index: u16,
        input: LayoutPassInput<'_>,
    ) -> Result<PassMaterializationPermit<'a>, PaginationError> {
        if pass_index != self.next_pass_index
            || input.state_index().get() != pass_index
            || !input.session.same_as(&self.session)
            || !input.layout_epoch().same_stable_inputs(self.layout_epoch)
            || input.layout_epoch().references() != input.generated_text().reference_fingerprint()
            || (pass_index == 0
                && (input.fingerprint() != self.initial_fingerprint
                    || input.layout_epoch() != self.layout_epoch))
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        self.consume_layout_pass()?;
        self.next_pass_index = self
            .next_pass_index
            .checked_add(1)
            .ok_or(PaginationError::PassIndexOverflow)?;
        let remaining_pages = self.max_pages;
        let remaining_fragments = self.max_fragments;
        let input_fingerprint = input.fingerprint();
        let layout_epoch = input.layout_epoch();
        let generated_text = input.generated_text().clone();
        Ok(PassMaterializationPermit {
            budget: self,
            session: input.session,
            pass_index,
            input_fingerprint,
            layout_epoch,
            generated_text,
            remaining_pages,
            remaining_fragments,
            pages: Vec::new(),
            active_page: None,
            next_fragment_ordinal: BTreeMap::new(),
            expected_page_start: None,
            pagination_exhausted: false,
            fallback_score: FallbackScoreAccumulator::default(),
        })
    }
    pub fn finish(self) -> PaginationBudgetReceipt {
        PaginationBudgetReceipt {
            session: self.session,
            initial_fingerprint: self.initial_fingerprint,
            layout_epoch: self.layout_epoch,
            passes: self.completed_passes,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PageBudgetSummary {
    page_index: u32,
    page_start: FlowPosition,
    flow_owner: NodeId,
    content_owner: NodeId,
    master_id: MasterId,
    frames: Vec<PageFramePlan>,
    consumed_fragment_count: u64,
    fragments: Vec<PlacedFragment>,
    footnote_ids: BTreeSet<FootnoteId>,
    float_decisions: Vec<FloatDecision>,
    column_decisions: Vec<ColumnDecision>,
    resolved_references: Vec<ResolvedReference>,
    placed_anchors: Vec<PlacedAnchor>,
    continuation: Option<Continuation>,
    next_page_start: Option<FlowPosition>,
    fragmenter_invoked: bool,
    footnote_evaluation: Option<FootnotePageEvaluationFingerprint>,
    footnote_cursor_progress: bool,
    next_fragment_cursor: FlowPosition,
    fragmentation_exhausted: bool,
    lookback_search_issued: bool,
    lookback_search_completed: bool,
}
impl PageBudgetSummary {
    fn matches(&self, page: &PagePlan) -> bool {
        self.page_index == page.page_index
            && self.master_id == page.master_id
            && self.frames == page.frames
            && u64::try_from(page.fragments.len()).ok() == Some(self.consumed_fragment_count)
            && self.fragments == page.fragments
            && self.footnote_ids.iter().cloned().collect::<Vec<_>>() == page.footnote_ids
            && self.float_decisions == page.float_decisions
            && self.column_decisions == page.column_decisions
            && self.resolved_references == page.resolved_references
    }
}

#[derive(Default)]
struct FallbackScoreAccumulator {
    hard_violations: u32,
    keep: i64,
    widow_orphan: i64,
    heading_isolation: i64,
    table_split: i64,
    footnote_split: i64,
    unused_space: i64,
    overflow: i64,
}
#[derive(Clone, Copy)]
enum CostComponentKind {
    Keep,
    WidowOrphan,
    HeadingIsolation,
    TableSplit,
    FootnoteSplit,
    UnusedSpace,
    Overflow,
}
impl FallbackScoreAccumulator {
    fn record_hard_violation(&mut self) -> Result<(), PaginationError> {
        self.hard_violations = self
            .hard_violations
            .checked_add(1)
            .ok_or(PaginationError::ArithmeticOverflow)?;
        Ok(())
    }
    fn add(&mut self, kind: CostComponentKind, value: i64) -> Result<(), PaginationError> {
        let mut values = [
            self.keep,
            self.widow_orphan,
            self.heading_isolation,
            self.table_split,
            self.footnote_split,
            self.unused_space,
            self.overflow,
        ];
        let index = kind as usize;
        let updated = values[index]
            .checked_add(value)
            .ok_or(PaginationError::ArithmeticOverflow)?;
        if !(-JSON_SAFE_INTEGER_MAX..=JSON_SAFE_INTEGER_MAX).contains(&updated) {
            return Err(PaginationError::FingerprintIntegerOutOfRange);
        }
        values[index] = updated;
        let total = values
            .iter()
            .try_fold(0i64, |total, component| total.checked_add(*component));
        if !matches!(total, Some(total) if (-JSON_SAFE_INTEGER_MAX..=JSON_SAFE_INTEGER_MAX).contains(&total))
        {
            return Err(PaginationError::FingerprintIntegerOutOfRange);
        }
        match kind {
            CostComponentKind::Keep => self.keep = updated,
            CostComponentKind::WidowOrphan => self.widow_orphan = updated,
            CostComponentKind::HeadingIsolation => self.heading_isolation = updated,
            CostComponentKind::TableSplit => self.table_split = updated,
            CostComponentKind::FootnoteSplit => self.footnote_split = updated,
            CostComponentKind::UnusedSpace => self.unused_space = updated,
            CostComponentKind::Overflow => self.overflow = updated,
        }
        Ok(())
    }
    fn finish(self) -> Result<FallbackScore, PaginationError> {
        let components = CostComponents::new(
            self.keep,
            self.widow_orphan,
            self.heading_isolation,
            self.table_split,
            self.footnote_split,
            self.unused_space,
            self.overflow,
        )
        .ok_or(PaginationError::FingerprintIntegerOutOfRange)?;
        Ok(FallbackScore::new(self.hard_violations, components))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct PassBudgetSummary {
    session: PaginationSessionId,
    pass_index: u16,
    input_fingerprint: LayoutStateFingerprint,
    layout_epoch: LayoutEpoch,
    generated_text: GeneratedTextStore,
    pages: Vec<PageBudgetSummary>,
    fallback_score: FallbackScore,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PassMaterializationReceipt {
    summary: PassBudgetSummary,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationBudgetReceipt {
    session: PaginationSessionId,
    initial_fingerprint: LayoutStateFingerprint,
    layout_epoch: LayoutEpoch,
    passes: Vec<PassBudgetSummary>,
}

pub struct PassMaterializationPermit<'a> {
    budget: &'a mut PaginationWorkBudget,
    session: PaginationSessionId,
    pass_index: u16,
    input_fingerprint: LayoutStateFingerprint,
    layout_epoch: LayoutEpoch,
    generated_text: GeneratedTextStore,
    remaining_pages: u32,
    remaining_fragments: u64,
    pages: Vec<PageBudgetSummary>,
    active_page: Option<PageBudgetSummary>,
    next_fragment_ordinal: BTreeMap<NodeId, u32>,
    expected_page_start: Option<FlowPosition>,
    pagination_exhausted: bool,
    fallback_score: FallbackScoreAccumulator,
}

/// Opaque proof that the designated `Fragmenter` produced the exact placed
/// fragments recorded by the active pagination pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FragmentMaterializationReceipt {
    placed_fragments: Vec<PlacedFragment>,
    continuation: Continuation,
    discovered_footnotes: Vec<FootnoteId>,
    placed_anchors: Vec<PlacedAnchor>,
}
impl FragmentMaterializationReceipt {
    pub fn placed_fragments(&self) -> &[PlacedFragment] {
        &self.placed_fragments
    }
    pub const fn continuation(&self) -> &Continuation {
        &self.continuation
    }
    pub fn discovered_footnotes(&self) -> &[FootnoteId] {
        &self.discovered_footnotes
    }
    pub fn placed_anchors(&self) -> &[PlacedAnchor] {
        &self.placed_anchors
    }
}

impl PassMaterializationPermit<'_> {
    /// Records one hard pagination violation at the decision point that
    /// produced it. The final score has no caller-supplied aggregate input.
    pub fn record_hard_violation(&mut self) -> Result<(), PaginationError> {
        self.fallback_score.record_hard_violation()
    }
    pub fn add_keep_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score.add(CostComponentKind::Keep, value)
    }
    pub fn add_widow_orphan_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score
            .add(CostComponentKind::WidowOrphan, value)
    }
    pub fn add_heading_isolation_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score
            .add(CostComponentKind::HeadingIsolation, value)
    }
    pub fn add_table_split_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score
            .add(CostComponentKind::TableSplit, value)
    }
    pub fn add_footnote_split_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score
            .add(CostComponentKind::FootnoteSplit, value)
    }
    pub fn add_unused_space_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score
            .add(CostComponentKind::UnusedSpace, value)
    }
    pub fn add_overflow_cost(&mut self, value: i64) -> Result<(), PaginationError> {
        self.fallback_score.add(CostComponentKind::Overflow, value)
    }
    pub fn begin_page(
        &mut self,
        page: &PageContext,
        cursor: &FlowCursor,
        frames: &[PageFramePlan],
    ) -> Result<(), PaginationError> {
        let page_index = page.page_index();
        if self.remaining_pages == 0 {
            return Err(PaginationError::ResourceLimit);
        }
        if self.active_page.is_some()
            || self.pagination_exhausted
            || self
                .expected_page_start
                .as_ref()
                .is_some_and(|expected| expected != cursor.position())
            || page.package_document_fingerprint() != self.layout_epoch.document()
            || page.package_style_fingerprint() != self.layout_epoch.style()
            || cursor.epoch() != self.layout_epoch
            || page.page_start() != cursor.position()
            || page.flow_owner() != cursor.owner_node()
            || page_index
                != u32::try_from(self.pages.len())
                    .map_err(|_| PaginationError::ArithmeticOverflow)?
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        validate_page_frames_for_context(page, frames)?;
        self.remaining_pages = self
            .remaining_pages
            .checked_sub(1)
            .ok_or(PaginationError::ResourceLimit)?;
        self.active_page = Some(PageBudgetSummary {
            page_index,
            page_start: page.page_start().clone(),
            flow_owner: page.flow_owner(),
            content_owner: page.content_owner(),
            master_id: page.master_id().clone(),
            frames: frames.to_vec(),
            consumed_fragment_count: 0,
            fragments: Vec::new(),
            footnote_ids: BTreeSet::new(),
            float_decisions: Vec::new(),
            column_decisions: Vec::new(),
            resolved_references: Vec::new(),
            placed_anchors: Vec::new(),
            continuation: None,
            next_page_start: None,
            fragmenter_invoked: false,
            footnote_evaluation: None,
            footnote_cursor_progress: false,
            next_fragment_cursor: page.page_start().clone(),
            fragmentation_exhausted: false,
            lookback_search_issued: false,
            lookback_search_completed: false,
        });
        Ok(())
    }
    pub fn finish_page(&mut self, page: &PagePlan) -> Result<(), PaginationError> {
        if self
            .budget
            .dequeued_floats
            .iter()
            .any(|(pass_index, _, _)| *pass_index == self.pass_index)
            || self
                .budget
                .column_candidates
                .keys()
                .any(|(pass_index, _)| *pass_index == self.pass_index)
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let active = self
            .active_page
            .take()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        let page_reflows = self
            .budget
            .footnote_reflows
            .get(&(self.pass_index, active.page_index))
            .copied()
            .unwrap_or(0);
        if !active.matches(page)
            || active.lookback_search_issued != active.lookback_search_completed
            || (page_reflows > 0 && active.footnote_ids.is_empty())
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        match active.continuation.as_ref() {
            Some(Continuation::More(cursor)) => {
                self.expected_page_start = Some(cursor.position().clone());
            }
            Some(Continuation::Exhausted(_)) | None => {
                self.expected_page_start = None;
                self.pagination_exhausted = true;
            }
        }
        let mut active = active;
        active.next_page_start = self.expected_page_start.clone();
        self.pages.push(active);
        Ok(())
    }
    /// Runs the sole fragment owner and records its exact materialization
    /// before the caller can build a `PagePlan`. `finish_page` accepts only the
    /// recorded fragment sequence.
    pub fn run_fragmenter<F: Fragmenter>(
        &mut self,
        fragmenter: &F,
        request: &FragmentRequest<'_>,
        frame_kind: PageFrameKind,
        column_index: u32,
    ) -> Result<FragmentMaterializationReceipt, FragmentError> {
        request.validate()?;
        // The reference implementation currently owns only the main/body
        // flow. Header/footer are reserved geometry and footnotes require an
        // independent subflow cursor/terminal receipt; none may consume the
        // body continuation accidentally.
        if frame_kind != PageFrameKind::Body {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        let before = self
            .active_page
            .as_ref()
            .ok_or(FragmentError::ResourceLimit)?
            .consumed_fragment_count;
        let active = self
            .active_page
            .as_ref()
            .ok_or(FragmentError::ResourceLimit)?;
        if active.fragmentation_exhausted
            || request.cursor().position() != &active.next_fragment_cursor
            || request.page().page_index() != active.page_index
            || request.page().master_id() != &active.master_id
            || request.page().page_start() != &active.page_start
            || !active.frames.iter().any(|frame| {
                frame.kind == frame_kind
                    && frame.column_index == column_index
                    && frame.bounds == request.frame()
            })
        {
            return Err(FragmentError::InvalidPageContext);
        }
        let allows_footnotes = active
            .frames
            .iter()
            .any(|frame| frame.kind == PageFrameKind::Footnote);
        let result = fragmenter.fragment(request, self)?;
        result.validate_progress(request)?;
        if !result.discovered_footnotes.is_empty() && !allows_footnotes {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        let active = self
            .active_page
            .as_mut()
            .ok_or(FragmentError::ResourceLimit)?;
        let consumed = active
            .consumed_fragment_count
            .checked_sub(before)
            .ok_or(FragmentError::ArithmeticOverflow)?;
        if consumed
            != u64::try_from(result.fragments.len())
                .map_err(|_| FragmentError::ArithmeticOverflow)?
        {
            return Err(FragmentError::InvalidFragmentKey);
        }
        let mut placed_fragments = Vec::with_capacity(result.fragments.len());
        for fragment in &result.fragments {
            let owner = fragment.start().owner();
            let ordinal = self.next_fragment_ordinal.entry(owner).or_insert(0);
            let placed = PlacedFragment {
                start: fragment.start().clone(),
                end: fragment.end().clone(),
                owner,
                owner_local_ordinal: *ordinal,
                frame_kind,
                column_index,
                bounds: fragment.bounds(),
            };
            *ordinal = ordinal
                .checked_add(1)
                .ok_or(FragmentError::ArithmeticOverflow)?;
            active.fragments.push(placed.clone());
            placed_fragments.push(placed);
        }
        let mut discovered_footnotes = result.discovered_footnotes;
        discovered_footnotes.sort();
        if discovered_footnotes.windows(2).any(|ids| ids[0] == ids[1]) {
            return Err(FragmentError::InvalidFragmentKey);
        }
        active
            .footnote_ids
            .extend(discovered_footnotes.iter().cloned());
        let mut discovered_anchors = result.discovered_anchors;
        discovered_anchors.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
        if discovered_anchors
            .windows(2)
            .any(|anchors| anchors[0].anchor_id == anchors[1].anchor_id)
            || active.placed_anchors.iter().any(|existing| {
                discovered_anchors
                    .iter()
                    .any(|anchor| &anchor.anchor_id == existing.anchor_id())
            })
        {
            return Err(FragmentError::InvalidFragmentKey);
        }
        let placed_anchors = discovered_anchors
            .iter()
            .map(|anchor| {
                PlacedAnchor::new(
                    anchor,
                    active.page_index,
                    frame_kind,
                    column_index,
                    request.frame(),
                )
            })
            .collect::<Result<Vec<_>, FragmentError>>()?;
        active.placed_anchors.extend(placed_anchors.clone());
        active
            .placed_anchors
            .sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
        active.continuation = Some(result.continuation.clone());
        match &result.continuation {
            Continuation::More(cursor) => {
                active.next_fragment_cursor = cursor.position().clone();
            }
            Continuation::Exhausted(cursor) => {
                active.next_fragment_cursor = cursor.position().clone();
                active.fragmentation_exhausted = true;
            }
        }
        active.fragmenter_invoked = true;
        Ok(FragmentMaterializationReceipt {
            placed_fragments,
            continuation: result.continuation,
            discovered_footnotes,
            placed_anchors,
        })
    }

    fn record_footnote_body_candidate(
        &mut self,
        flow: &FlowTree,
        candidate: &EvaluatedFootnoteBodyPage,
        body_frame: Rect,
        footnote_ids: &[FootnoteId],
    ) -> Result<Vec<PlacedFragment>, PaginationError> {
        let active = self
            .active_page
            .as_ref()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if active.fragmenter_invoked
            || active.next_fragment_cursor != active.page_start
            || candidate.continuation.epoch() != self.layout_epoch
            || !flow.contains_position(candidate.continuation.position())
            || active.frames.iter().all(|frame| {
                frame.kind != PageFrameKind::Body
                    || frame.column_index != 0
                    || frame.bounds != body_frame
            })
            || candidate
                .fragments
                .windows(2)
                .any(|pair| pair[0].end() != pair[1].start())
            || candidate.fragments.iter().any(|fragment| {
                !flow.contains_position(fragment.start())
                    || !flow.contains_position(fragment.end())
                    || !rect_contains(body_frame, fragment.bounds())
            })
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let fragment_count = u64::try_from(candidate.fragments.len())
            .map_err(|_| PaginationError::ArithmeticOverflow)?;
        self.remaining_fragments = self
            .remaining_fragments
            .checked_sub(fragment_count)
            .ok_or(PaginationError::ResourceLimit)?;

        let mut placed = Vec::new();
        placed
            .try_reserve_exact(candidate.fragments.len())
            .map_err(|_| PaginationError::ResourceLimit)?;
        for fragment in &candidate.fragments {
            let owner = fragment.start().owner();
            let ordinal = self.next_fragment_ordinal.entry(owner).or_insert(0);
            placed.push(PlacedFragment {
                start: fragment.start().clone(),
                end: fragment.end().clone(),
                owner,
                owner_local_ordinal: *ordinal,
                frame_kind: PageFrameKind::Body,
                column_index: 0,
                bounds: fragment.bounds(),
            });
            *ordinal = ordinal
                .checked_add(1)
                .ok_or(PaginationError::ArithmeticOverflow)?;
        }
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        active.consumed_fragment_count = active
            .consumed_fragment_count
            .checked_add(fragment_count)
            .ok_or(PaginationError::ArithmeticOverflow)?;
        active.fragments.extend(placed.iter().cloned());
        for footnote_id in footnote_ids {
            active.footnote_ids.insert(footnote_id.clone());
        }
        let mut anchors = candidate.anchors.clone();
        anchors.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
        if anchors
            .windows(2)
            .any(|pair| pair[0].anchor_id == pair[1].anchor_id)
            || active.placed_anchors.iter().any(|existing| {
                anchors
                    .iter()
                    .any(|anchor| &anchor.anchor_id == existing.anchor_id())
            })
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let placed_anchors = anchors
            .iter()
            .map(|anchor| {
                PlacedAnchor::new(
                    anchor,
                    active.page_index,
                    PageFrameKind::Body,
                    0,
                    body_frame,
                )
            })
            .collect::<Result<Vec<_>, FragmentError>>()
            .map_err(reference_fragment_error)?;
        active.placed_anchors.extend(placed_anchors);
        active
            .placed_anchors
            .sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
        active.continuation = Some(if candidate.terminal {
            Continuation::Exhausted(Box::new(candidate.continuation.clone()))
        } else {
            Continuation::More(Box::new(candidate.continuation.clone()))
        });
        active.next_fragment_cursor = candidate.continuation.position().clone();
        active.fragmentation_exhausted = candidate.terminal;
        active.fragmenter_invoked = true;
        Ok(placed)
    }

    fn record_footnote_definition_anchors(
        &mut self,
        anchors: Vec<DiscoveredAnchor>,
        frame: Rect,
    ) -> Result<(), PaginationError> {
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if active.frames.iter().all(|candidate| {
            candidate.kind != PageFrameKind::Footnote
                || candidate.column_index != 0
                || candidate.bounds != frame
        }) {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let mut anchors = anchors;
        anchors.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
        if anchors
            .windows(2)
            .any(|pair| pair[0].anchor_id == pair[1].anchor_id)
            || active.placed_anchors.iter().any(|existing| {
                anchors
                    .iter()
                    .any(|anchor| &anchor.anchor_id == existing.anchor_id())
            })
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let placed = anchors
            .iter()
            .map(|anchor| {
                PlacedAnchor::new(anchor, active.page_index, PageFrameKind::Footnote, 0, frame)
            })
            .collect::<Result<Vec<_>, FragmentError>>()
            .map_err(reference_fragment_error)?;
        active.placed_anchors.extend(placed);
        active
            .placed_anchors
            .sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
        Ok(())
    }

    fn record_footnote_frame(
        &mut self,
        maximum_frame: Rect,
        frame: &PageFramePlan,
    ) -> Result<(), PaginationError> {
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        let maximum_end = maximum_frame
            .y()
            .checked_add(maximum_frame.height().get())
            .ok_or(PaginationError::ArithmeticOverflow)?;
        let frame_end = frame
            .bounds
            .y()
            .checked_add(frame.bounds.height().get())
            .ok_or(PaginationError::ArithmeticOverflow)?;
        if frame.kind != PageFrameKind::Footnote
            || frame.column_index != 0
            || frame.bounds.x() != maximum_frame.x()
            || frame.bounds.width() != maximum_frame.width()
            || frame_end != maximum_end
            || !rect_contains(maximum_frame, frame.bounds)
            || active
                .frames
                .iter()
                .any(|existing| existing.kind == PageFrameKind::Footnote)
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        validate_rect(frame.bounds)?;
        active.frames.push(frame.clone());
        Ok(())
    }

    fn finish_footnote_page(
        &mut self,
        page: &PagePlan,
        selected: &StagingFootnoteSelectedPageReceipt,
        has_more_pages: bool,
    ) -> Result<(), PaginationError> {
        let mut active = self
            .active_page
            .take()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        let page_reflows = self
            .budget
            .footnote_reflows
            .get(&(self.pass_index, active.page_index))
            .copied()
            .unwrap_or(0);
        let expected_reflows = u16::try_from(selected.evaluation_count().saturating_sub(1))
            .map_err(|_| PaginationError::ArithmeticOverflow)?;
        let expected_ids: BTreeSet<_> = selected
            .flows()
            .iter()
            .map(|flow| flow.assignment().footnote_id().clone())
            .collect();
        if !active.matches(page)
            || selected.page_index() != active.page_index
            || selected.global_pass() != self.pass_index
            || page_reflows != expected_reflows
            || active.footnote_ids != expected_ids
            || active.lookback_search_issued != active.lookback_search_completed
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let footnote_record_count = selected_page_footnote_record_count(selected)?;
        self.remaining_fragments = self
            .remaining_fragments
            .checked_sub(footnote_record_count)
            .ok_or(PaginationError::ResourceLimit)?;
        active.footnote_evaluation = Some(selected.evaluation_fingerprint());
        active.footnote_cursor_progress = selected.flows().iter().any(|flow| {
            flow.after_cursor().next_fragment_ordinal()
                > flow.before_cursor().next_fragment_ordinal()
        });
        if has_more_pages {
            if active.next_fragment_cursor == active.page_start && !active.footnote_cursor_progress
            {
                return Err(PaginationError::NoProgress);
            }
            self.expected_page_start = Some(active.next_fragment_cursor.clone());
            active.next_page_start = self.expected_page_start.clone();
        } else {
            if !selected.body_continuation().is_terminal() {
                return Err(PaginationError::InvalidWorkPermit);
            }
            self.expected_page_start = None;
            self.pagination_exhausted = true;
            active.next_page_start = None;
        }
        self.pages.push(active);
        Ok(())
    }
    /// Finalizes a float that was enqueued and then dequeued by fragment work.
    /// The exact decision becomes part of the page materialization receipt.
    pub fn record_float_decision(
        &mut self,
        decision: FloatDecision,
    ) -> Result<(), PaginationError> {
        let key = (
            self.pass_index,
            decision.owner,
            decision.owner_local_ordinal,
        );
        let active = self
            .active_page
            .as_ref()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if !self.budget.dequeued_floats.contains(&key)
            || active.float_decisions.iter().any(|existing| {
                (existing.owner, existing.owner_local_ordinal)
                    == (decision.owner, decision.owner_local_ordinal)
            })
            || !active.frames.iter().any(|frame| {
                frame.kind == decision.frame_kind
                    && frame.column_index == decision.column_index
                    && rect_contains(frame.bounds, decision.bounds)
            })
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        validate_rect(decision.bounds)?;
        self.budget.dequeued_floats.remove(&key);
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        active.float_decisions.push(decision);
        active
            .float_decisions
            .sort_by_key(|value| (value.owner, value.owner_local_ordinal));
        Ok(())
    }
    /// Finalizes the canonical column choice for one container after at least
    /// one budgeted balance candidate was evaluated.
    pub fn record_column_decisions(
        &mut self,
        container: NodeId,
        mut decisions: Vec<ColumnDecision>,
    ) -> Result<(), PaginationError> {
        let active = self
            .active_page
            .as_ref()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if self
            .budget
            .column_candidates
            .get(&(self.pass_index, container))
            .copied()
            .unwrap_or(0)
            == 0
            || decisions.is_empty()
            || active
                .column_decisions
                .iter()
                .any(|existing| existing.container == container)
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        decisions.sort_by_key(|decision| decision.column_index);
        for (expected, decision) in decisions.iter().enumerate() {
            if decision.container != container
                || decision.column_index
                    != u32::try_from(expected).map_err(|_| PaginationError::ArithmeticOverflow)?
                || !active.frames.iter().any(|frame| {
                    frame.kind == PageFrameKind::Body
                        && rect_contains(frame.bounds, decision.bounds)
                })
            {
                return Err(PaginationError::InvalidWorkPermit);
            }
            validate_rect(decision.bounds)?;
        }
        self.budget
            .column_candidates
            .remove(&(self.pass_index, container));
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        active.column_decisions.extend(decisions);
        active
            .column_decisions
            .sort_by_key(|decision| (decision.container, decision.column_index));
        Ok(())
    }
    /// Records one store-issued reference result in canonical order.
    pub fn record_resolved_reference(
        &mut self,
        reference: ResolvedReference,
    ) -> Result<(), PaginationError> {
        if reference.reference_fingerprint != self.layout_epoch.references() {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if active
            .resolved_references
            .iter()
            .any(|existing| existing.provenance == reference.provenance)
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        active.resolved_references.push(reference);
        active
            .resolved_references
            .sort_by(compare_resolved_reference);
        Ok(())
    }
    pub fn begin_page_break_search(&mut self) -> Result<PageBreakSearchBudget, PaginationError> {
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if active.lookback_search_issued {
            return Err(PaginationError::InvalidWorkPermit);
        }
        active.lookback_search_issued = true;
        Ok(PageBreakSearchBudget {
            page_index: active.page_index,
            boundary: active.page_start.clone(),
            remaining_candidates: self.budget.max_page_break_lookback,
        })
    }
    pub fn finish_page_break_search(
        &mut self,
        search: PageBreakSearchBudget,
    ) -> Result<(), PaginationError> {
        let active = self
            .active_page
            .as_mut()
            .ok_or(PaginationError::InvalidWorkPermit)?;
        if !active.lookback_search_issued
            || active.lookback_search_completed
            || active.page_index != search.page_index
            || active.page_start != search.boundary
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        active.lookback_search_completed = true;
        Ok(())
    }
    pub fn finish(
        self,
        flow: &FlowTree,
        pages: &[PagePlan],
    ) -> Result<PassMaterializationReceipt, PaginationError> {
        if self.active_page.is_some()
            || flow.epoch() != self.layout_epoch
            || self
                .budget
                .queued_floats
                .iter()
                .any(|(pass_index, _, _)| *pass_index == self.pass_index)
            || self
                .budget
                .dequeued_floats
                .iter()
                .any(|(pass_index, _, _)| *pass_index == self.pass_index)
            || self
                .budget
                .column_candidates
                .keys()
                .any(|(pass_index, _)| *pass_index == self.pass_index)
            || pages.len() != self.pages.len()
            || self
                .pages
                .iter()
                .any(|summary| summary.page_start.epoch() != self.layout_epoch)
            || pages
                .iter()
                .zip(&self.pages)
                .any(|(page, summary)| !summary.matches(page))
            || !validate_materialization_chain(flow, &self.pages)
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let summary = PassBudgetSummary {
            session: self.session,
            pass_index: self.pass_index,
            input_fingerprint: self.input_fingerprint,
            layout_epoch: self.layout_epoch,
            generated_text: self.generated_text,
            pages: self.pages,
            fallback_score: self.fallback_score.finish()?,
        };
        self.budget.completed_passes.push(summary.clone());
        Ok(PassMaterializationReceipt { summary })
    }
}

fn selected_page_footnote_record_count(
    selected: &StagingFootnoteSelectedPageReceipt,
) -> Result<u64, PaginationError> {
    if selected.flows().is_empty() {
        return Ok(0);
    }
    let assignments =
        u64::try_from(selected.flows().len()).map_err(|_| PaginationError::ArithmeticOverflow)?;
    let fragments = selected.flows().iter().try_fold(0u64, |count, flow| {
        count.checked_add(u64::try_from(flow.fragments().len()).ok()?)
    });
    assignments
        .checked_add(1)
        .and_then(|count| count.checked_add(fragments?))
        .ok_or(PaginationError::ArithmeticOverflow)
}

impl FragmentWorkBudget for PassMaterializationPermit<'_> {
    fn consume_fragments(&mut self, count: u64) -> Result<(), FragmentError> {
        let active = self
            .active_page
            .as_mut()
            .ok_or(FragmentError::ResourceLimit)?;
        self.remaining_fragments = self
            .remaining_fragments
            .checked_sub(count)
            .ok_or(FragmentError::ResourceLimit)?;
        active.consumed_fragment_count = active
            .consumed_fragment_count
            .checked_add(count)
            .ok_or(FragmentError::ArithmeticOverflow)?;
        Ok(())
    }
    fn consume_footnote_reflow(&mut self, page_index: u32) -> Result<(), FragmentError> {
        if self.active_page.as_ref().map(|page| page.page_index) != Some(page_index) {
            return Err(FragmentError::InvalidPageContext);
        }
        let used = self
            .budget
            .footnote_reflows
            .entry((self.pass_index, page_index))
            .or_insert(0);
        *used = used
            .checked_add(1)
            .filter(|used| *used <= self.budget.max_footnote_reflows_per_page)
            .ok_or(FragmentError::ResourceLimit)?;
        Ok(())
    }
    fn consume_column_candidate(&mut self, container: NodeId) -> Result<(), FragmentError> {
        if self.active_page.is_none() {
            return Err(FragmentError::InvalidPageContext);
        }
        let used = self
            .budget
            .column_candidates
            .entry((self.pass_index, container))
            .or_insert(0);
        *used = used
            .checked_add(1)
            .filter(|used| *used <= self.budget.max_column_balance_candidates)
            .ok_or(FragmentError::ResourceLimit)?;
        Ok(())
    }
    fn enqueue_float(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError> {
        if self.active_page.is_none() {
            return Err(FragmentError::InvalidPageContext);
        }
        let key = (self.pass_index, owner, owner_local_ordinal);
        if self.budget.queued_floats.contains(&key) {
            return Err(FragmentError::InvalidFloatState);
        }
        if self.budget.queued_floats.len() >= self.budget.max_float_queue as usize {
            return Err(FragmentError::ResourceLimit);
        }
        self.budget.queued_floats.insert(key);
        Ok(())
    }
    fn dequeue_float(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError> {
        if self.active_page.is_none() {
            return Err(FragmentError::InvalidPageContext);
        }
        let key = (self.pass_index, owner, owner_local_ordinal);
        if !self.budget.queued_floats.remove(&key) || !self.budget.dequeued_floats.insert(key) {
            return Err(FragmentError::InvalidFloatState);
        }
        Ok(())
    }
    fn consume_float_carry(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError> {
        if self.active_page.is_none() {
            return Err(FragmentError::InvalidPageContext);
        }
        if !self
            .budget
            .queued_floats
            .contains(&(self.pass_index, owner, owner_local_ordinal))
        {
            return Err(FragmentError::InvalidFloatState);
        }
        let used = self
            .budget
            .float_carries
            .entry((self.pass_index, owner, owner_local_ordinal))
            .or_insert(0);
        *used = used
            .checked_add(1)
            .filter(|used| *used <= self.budget.max_float_carry_pages)
            .ok_or(FragmentError::ResourceLimit)?;
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PageBreakSearchBudget {
    page_index: u32,
    boundary: FlowPosition,
    remaining_candidates: u16,
}
impl PageBreakSearchBudget {
    pub fn consume_candidate(&mut self) -> Result<(), PaginationError> {
        self.remaining_candidates = self
            .remaining_candidates
            .checked_sub(1)
            .ok_or(PaginationError::ResourceLimit)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitialPaginationState {
    layout_epoch: LayoutEpoch,
    flow_positions: Vec<FlowPosition>,
    generated_text: GeneratedTextStore,
    fingerprint: LayoutStateFingerprint,
}
impl InitialPaginationState {
    pub fn new(
        flow: &FlowTree,
        package: &ValidatedParsedPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, PaginationError> {
        let generated_text = package
            .materialize_initial_generated_text(limits)
            .map_err(|_| PaginationError::InvalidInitialReferenceSeed)?;
        let layout_epoch = flow.epoch();
        if layout_epoch.document() != package.epoch_identity().document()
            || layout_epoch.style() != package.epoch_identity().style()
            || layout_epoch.references() != generated_text.reference_fingerprint()
        {
            return Err(PaginationError::EpochMismatch);
        }
        let mut jcs = String::from("{\"algorithm\":");
        push_jcs_string(&mut jcs, LayoutStateFingerprint::INITIAL_ALGORITHM_ID);
        jcs.push_str(",\"flow_positions\":[");
        encode_flow_positions(&mut jcs, flow.positions());
        jcs.push_str("],\"layout_epoch\":");
        encode_layout_epoch(&mut jcs, layout_epoch);
        jcs.push_str(",\"resolved_generated_text\":[");
        for (index, buffer) in generated_text.buffers().iter().enumerate() {
            comma(&mut jcs, index);
            jcs.push_str("{\"end_byte\":");
            jcs.push_str(&buffer.utf8().len().to_string());
            jcs.push_str(",\"key\":");
            encode_generated_key(&mut jcs, buffer.key());
            jcs.push_str(",\"start_byte\":0,\"utf8\":");
            push_jcs_string(&mut jcs, buffer.utf8());
            jcs.push('}');
        }
        jcs.push_str("]}");
        let fingerprint = initial_pagination_state_fingerprint_from_jcs(&jcs);
        Ok(Self {
            layout_epoch,
            flow_positions: flow.positions().to_vec(),
            generated_text,
            fingerprint,
        })
    }
    pub const fn layout_epoch(&self) -> LayoutEpoch {
        self.layout_epoch
    }
    pub const fn generated_text(&self) -> &GeneratedTextStore {
        &self.generated_text
    }
    pub fn flow_positions(&self) -> &[FlowPosition] {
        &self.flow_positions
    }
    pub const fn fingerprint(&self) -> LayoutStateFingerprint {
        self.fingerprint
    }
}

/// Sealed proof that the next pass overlay was derived from one exact
/// materialized predecessor. The owned form keeps a newly resolved overlay
/// alive through pass work; the borrowed form avoids a copy when the overlay
/// is unchanged.
#[derive(Debug, Eq, PartialEq)]
pub struct ReferenceTransitionReceipt<'a> {
    session: PaginationSessionId,
    previous_state: MaterializedStateIndex,
    previous_fingerprint: LayoutStateFingerprint,
    working_epoch: LayoutEpoch,
    generated_text: Cow<'a, GeneratedTextStore>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PaginationInput<'a> {
    session: PaginationSessionId,
    initial_state: InitialPaginationState,
    package_context: &'a PackagePaginationContext,
    options: PaginationOptions,
    work_budget: Option<PaginationWorkBudget>,
}
impl<'a> PaginationInput<'a> {
    pub fn new(
        initial_state: InitialPaginationState,
        package_context: &'a PackagePaginationContext,
        options: PaginationOptions,
    ) -> Result<Self, PaginationError> {
        if options.max_pages == 0
            || options.max_layout_passes == 0
            || options.max_page_break_lookback == 0
        {
            return Err(PaginationError::InvalidOptions);
        }
        if initial_state.layout_epoch().document() != package_context.document_fingerprint()
            || initial_state.layout_epoch().style() != package_context.style_fingerprint()
        {
            return Err(PaginationError::PackageEpochMismatch);
        }
        package_context
            .page_masters()
            .validate()
            .map_err(|_| PaginationError::MissingDefaultMaster)?;
        let session = PaginationSessionId::issue();
        let work_budget = PaginationWorkBudget::new(
            session.clone(),
            options,
            initial_state.fingerprint(),
            initial_state.layout_epoch(),
        );
        Ok(Self {
            session,
            initial_state,
            package_context,
            options,
            work_budget: Some(work_budget),
        })
    }
    pub const fn initial_fingerprint(&self) -> LayoutStateFingerprint {
        self.initial_state.fingerprint()
    }
    pub const fn initial_state(&self) -> &InitialPaginationState {
        &self.initial_state
    }
    pub const fn page_masters(&self) -> &PageMasterSet {
        self.package_context.page_masters()
    }
    pub const fn options(&self) -> PaginationOptions {
        self.options
    }
    pub fn take_work_budget(&mut self) -> Result<PaginationWorkBudget, PaginationError> {
        self.work_budget
            .take()
            .ok_or(PaginationError::WorkBudgetAlreadyIssued)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutPassInput<'a> {
    session: PaginationSessionId,
    state_index: LayoutStateIndex,
    fingerprint: LayoutStateFingerprint,
    layout_epoch: LayoutEpoch,
    generated_text: Cow<'a, GeneratedTextStore>,
}
impl<'a> LayoutPassInput<'a> {
    pub fn initial(input: &'a PaginationInput<'_>) -> Self {
        Self {
            session: input.session.clone(),
            state_index: LayoutStateIndex::INITIAL,
            fingerprint: input.initial_fingerprint(),
            layout_epoch: input.initial_state().layout_epoch(),
            generated_text: Cow::Borrowed(input.initial_state().generated_text()),
        }
    }
    pub fn transitioned(transition: ReferenceTransitionReceipt<'a>) -> Self {
        Self {
            session: transition.session,
            state_index: LayoutStateIndex::new(transition.previous_state.get()),
            fingerprint: transition.previous_fingerprint,
            layout_epoch: transition.working_epoch,
            generated_text: transition.generated_text,
        }
    }
    pub const fn state_index(&self) -> LayoutStateIndex {
        self.state_index
    }
    pub const fn fingerprint(&self) -> LayoutStateFingerprint {
        self.fingerprint
    }
    pub const fn layout_epoch(&self) -> LayoutEpoch {
        self.layout_epoch
    }
    pub fn generated_text(&self) -> &GeneratedTextStore {
        self.generated_text.as_ref()
    }
}

/// State-dependent layout rebuilt for one pagination pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedLayout {
    flow: FlowTree,
    initial_cursor: FlowCursor,
}
impl PreparedLayout {
    pub fn new(
        input: LayoutPassInput<'_>,
        flow: FlowTree,
        initial_cursor: FlowCursor,
    ) -> Result<Self, PaginationError> {
        if flow.epoch() != initial_cursor.epoch() || flow.epoch() != input.layout_epoch() {
            return Err(PaginationError::InvalidPreparedLayout);
        }
        Ok(Self {
            flow,
            initial_cursor,
        })
    }
    pub const fn flow(&self) -> &FlowTree {
        &self.flow
    }
    pub const fn initial_cursor(&self) -> &FlowCursor {
        &self.initial_cursor
    }
}

pub trait LayoutPassProvider {
    fn prepare(
        &self,
        input: LayoutPassInput<'_>,
        permit: &mut PassMaterializationPermit<'_>,
    ) -> Result<PreparedLayout, PaginationError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacedFragment {
    pub start: FlowPosition,
    pub end: FlowPosition,
    pub owner: NodeId,
    pub owner_local_ordinal: u32,
    pub frame_kind: PageFrameKind,
    pub column_index: u32,
    pub bounds: Rect,
}

/// Anchor placement bound to the exact page/frame selected by the pagination
/// receipt. Display destinations are derived from this record, not from a
/// caller-provided anchor list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacedAnchor {
    anchor_id: AnchorId,
    owner_node: NodeId,
    page_index: u32,
    frame_kind: PageFrameKind,
    column_index: u32,
    position_in_frame: Point,
}
impl PlacedAnchor {
    fn new(
        anchor: &DiscoveredAnchor,
        page_index: u32,
        frame_kind: PageFrameKind,
        column_index: u32,
        frame: Rect,
    ) -> Result<Self, FragmentError> {
        let x = anchor.position_in_frame.x.raw();
        let y = anchor.position_in_frame.y.raw();
        if x < 0 || y < 0 || x > frame.width().get().raw() || y > frame.height().get().raw() {
            return Err(FragmentError::InvalidPageContext);
        }
        Ok(Self {
            anchor_id: anchor.anchor_id.clone(),
            owner_node: anchor.owner_node,
            page_index,
            frame_kind,
            column_index,
            position_in_frame: anchor.position_in_frame,
        })
    }
    pub const fn anchor_id(&self) -> &AnchorId {
        &self.anchor_id
    }
    pub const fn owner_node(&self) -> NodeId {
        self.owner_node
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn frame_kind(&self) -> PageFrameKind {
        self.frame_kind
    }
    pub const fn column_index(&self) -> u32 {
        self.column_index
    }
    pub const fn position_in_frame(&self) -> Point {
        self.position_in_frame
    }
    pub fn position_on_page(&self, frame: Rect) -> Option<Point> {
        Some(Point {
            x: frame.x().checked_add(self.position_in_frame.x)?,
            y: frame.y().checked_add(self.position_in_frame.y)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PageFrameKind {
    Body,
    Header,
    Footer,
    Footnote,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageFramePlan {
    pub kind: PageFrameKind,
    pub column_index: u32,
    pub bounds: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FloatDecision {
    pub owner: NodeId,
    pub owner_local_ordinal: u32,
    pub frame_kind: PageFrameKind,
    pub column_index: u32,
    pub bounds: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnDecision {
    pub container: NodeId,
    pub column_index: u32,
    pub bounds: Rect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PagePlan {
    pub page_index: u32,
    pub master_id: MasterId,
    pub frames: Vec<PageFramePlan>,
    pub fragments: Vec<PlacedFragment>,
    pub footnote_ids: Vec<FootnoteId>,
    pub float_decisions: Vec<FloatDecision>,
    pub column_decisions: Vec<ColumnDecision>,
    pub resolved_references: Vec<ResolvedReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedReference {
    anchor_id: AnchorId,
    provenance: GeneratedProvenance,
    reference_fingerprint: ReferenceFingerprint,
    utf8: String,
}
impl ResolvedReference {
    pub fn from_store(
        store: &GeneratedTextStore,
        key: GeneratedBufferKey,
        start: Utf8ByteOffset,
        end: Utf8ByteOffset,
    ) -> Result<Self, ResolvedReferenceError> {
        let reference_site = store
            .document_nodes()
            .generated_site(key)
            .ok_or(ResolvedReferenceError::InvalidReferenceSite)?;
        let anchor_id = match reference_site.target() {
            GeneratedSiteTarget::Anchor(anchor_id)
                if key.generation_kind() == GenerationKind::PageReference =>
            {
                anchor_id.clone()
            }
            _ => return Err(ResolvedReferenceError::InvalidReferenceSite),
        };
        let provenance = store.provenance(key, start, end)?;
        let buffer = store
            .get(provenance.text_span().text_id())
            .ok_or(GeneratedTextStoreError::UnknownKey)?;
        let utf8 = buffer.utf8()[start.get() as usize..end.get() as usize].to_owned();
        Ok(Self {
            anchor_id,
            provenance,
            reference_fingerprint: store.reference_fingerprint(),
            utf8,
        })
    }
    pub const fn anchor_id(&self) -> &AnchorId {
        &self.anchor_id
    }
    pub const fn provenance(&self) -> GeneratedProvenance {
        self.provenance
    }
    pub fn utf8(&self) -> &str {
        &self.utf8
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolvedReferenceError {
    InvalidReferenceSite,
    GeneratedText(GeneratedTextStoreError),
}
impl From<GeneratedTextStoreError> for ResolvedReferenceError {
    fn from(value: GeneratedTextStoreError) -> Self {
        Self::GeneratedText(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationFingerprintRecord {
    layout_epoch: LayoutEpoch,
    placed_anchors: Vec<PlacedAnchor>,
    flow_positions: Vec<FlowPosition>,
    pages: Vec<PagePlan>,
    generated_text: GeneratedTextStore,
}
impl PaginationFingerprintRecord {
    pub fn new(
        flow: &FlowTree,
        mut pages: Vec<PagePlan>,
        generated_store: GeneratedTextStore,
        mut placed_anchors: Vec<PlacedAnchor>,
    ) -> Result<Self, PaginationError> {
        let layout_epoch = flow.epoch();
        if layout_epoch.references() != generated_store.reference_fingerprint() {
            return Err(PaginationError::EpochMismatch);
        }
        if generated_store.buffers().iter().any(|buffer| {
            generated_store
                .document_nodes()
                .generated_site(buffer.key())
                .is_none()
        }) {
            return Err(PaginationError::UnknownFlowOwner);
        }
        if pages.is_empty() {
            return Err(PaginationError::EmptyPages);
        }
        canonicalize_placed_anchors(&mut placed_anchors);
        if !validate_fingerprint_anchors(flow, &pages, &placed_anchors) {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let mut all_provenance = BTreeSet::new();
        let mut materialized_footnotes = BTreeSet::new();
        let mut next_fragment_ordinal = BTreeMap::<NodeId, u32>::new();
        for (expected, page) in pages.iter_mut().enumerate() {
            if page.page_index != u32::try_from(expected).map_err(|_| PaginationError::PageLimit)? {
                return Err(PaginationError::InvalidPageIndex);
            }
            page.frames
                .sort_by_key(|frame| (frame.kind, frame.column_index));
            if !matches!(
                page.frames.first(),
                Some(frame) if frame.kind == PageFrameKind::Body && frame.column_index == 0
            ) {
                return Err(PaginationError::MissingBodyFrame);
            }
            let mut previous_frame: Option<(PageFrameKind, u32)> = None;
            let mut frame_bounds = BTreeMap::new();
            for frame in &page.frames {
                validate_rect(frame.bounds)?;
                let expected_column = match previous_frame {
                    Some((kind, column)) if kind == frame.kind => column
                        .checked_add(1)
                        .ok_or(PaginationError::InvalidFrameColumn)?,
                    _ => 0,
                };
                if frame.column_index != expected_column {
                    return Err(PaginationError::InvalidFrameColumn);
                }
                previous_frame = Some((frame.kind, frame.column_index));
                frame_bounds.insert((frame.kind, frame.column_index), frame.bounds);
            }
            page.fragments.sort_by(|left, right| {
                compare_flow_position(&left.start, &right.start)
                    .then_with(|| left.owner.cmp(&right.owner))
                    .then_with(|| left.owner_local_ordinal.cmp(&right.owner_local_ordinal))
            });
            if page.fragments.windows(2).any(|pair| {
                compare_flow_position(&pair[0].start, &pair[1].start).is_eq()
                    && pair[0].owner == pair[1].owner
                    && pair[0].owner_local_ordinal == pair[1].owner_local_ordinal
            }) {
                return Err(PaginationError::DuplicateFragmentPosition);
            }
            for fragment in &page.fragments {
                let expected_ordinal = next_fragment_ordinal.entry(fragment.owner).or_insert(0);
                if fragment.start.epoch() != layout_epoch
                    || fragment.end.epoch() != layout_epoch
                    || !flow.contains_position(&fragment.start)
                    || !flow.contains_position(&fragment.end)
                    || !fragment
                        .start
                        .cmp_within_epoch(&fragment.end)
                        .map_err(|_| PaginationError::EpochMismatch)?
                        .is_lt()
                    || fragment.start.owner() != fragment.owner
                    || fragment.owner_local_ordinal != *expected_ordinal
                {
                    return Err(PaginationError::InvalidFragmentRange);
                }
                *expected_ordinal = expected_ordinal
                    .checked_add(1)
                    .ok_or(PaginationError::ArithmeticOverflow)?;
                validate_flow_position(&fragment.start)?;
                validate_flow_position(&fragment.end)?;
                validate_rect(fragment.bounds)?;
                let containing_frame = frame_bounds
                    .get(&(fragment.frame_kind, fragment.column_index))
                    .ok_or(PaginationError::UnknownFrameReference)?;
                if !rect_contains(*containing_frame, fragment.bounds) {
                    return Err(PaginationError::FragmentOutsideFrame);
                }
            }
            page.footnote_ids.sort();
            if page.footnote_ids.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(PaginationError::DuplicateFootnote);
            }
            materialized_footnotes.extend(page.footnote_ids.iter().cloned());
            page.float_decisions
                .sort_by_key(|decision| (decision.owner, decision.owner_local_ordinal));
            if page.float_decisions.windows(2).any(|pair| {
                (pair[0].owner, pair[0].owner_local_ordinal)
                    == (pair[1].owner, pair[1].owner_local_ordinal)
            }) {
                return Err(PaginationError::DuplicateFloatDecision);
            }
            for decision in &page.float_decisions {
                if !flow.contains_owner(decision.owner) {
                    return Err(PaginationError::UnknownFlowOwner);
                }
                validate_rect(decision.bounds)?;
                let containing_frame = frame_bounds
                    .get(&(decision.frame_kind, decision.column_index))
                    .ok_or(PaginationError::UnknownFrameReference)?;
                if !rect_contains(*containing_frame, decision.bounds) {
                    return Err(PaginationError::FragmentOutsideFrame);
                }
            }
            page.column_decisions
                .sort_by_key(|decision| (decision.container, decision.column_index));
            let mut previous_column: Option<(NodeId, u32)> = None;
            for decision in &page.column_decisions {
                if !flow.contains_owner(decision.container) {
                    return Err(PaginationError::UnknownFlowOwner);
                }
                let expected_column = match previous_column {
                    Some((container, column)) if container == decision.container => column
                        .checked_add(1)
                        .ok_or(PaginationError::InvalidColumnDecision)?,
                    _ => 0,
                };
                if decision.column_index != expected_column {
                    return Err(PaginationError::InvalidColumnDecision);
                }
                validate_rect(decision.bounds)?;
                previous_column = Some((decision.container, decision.column_index));
            }
            page.resolved_references.sort_by(|left, right| {
                let left_span = left.provenance.text_span().range();
                let right_span = right.provenance.text_span().range();
                (
                    left.provenance.buffer_key(),
                    left_span.start_byte(),
                    left_span.end_byte(),
                    &left.anchor_id,
                )
                    .cmp(&(
                        right.provenance.buffer_key(),
                        right_span.start_byte(),
                        right_span.end_byte(),
                        &right.anchor_id,
                    ))
            });
            for reference in &page.resolved_references {
                if reference.reference_fingerprint != generated_store.reference_fingerprint()
                    || !generated_store.validates_provenance(reference.provenance)
                    || !all_provenance.insert(reference.provenance)
                {
                    return Err(PaginationError::DuplicateResolvedReference);
                }
            }
        }
        if &materialized_footnotes
            != generated_store
                .document_nodes()
                .footnote_reference_targets()
        {
            return Err(PaginationError::InvalidFootnoteClosure);
        }
        Ok(Self {
            layout_epoch,
            placed_anchors,
            flow_positions: flow.positions().to_vec(),
            pages,
            generated_text: generated_store,
        })
    }
    pub fn pages(&self) -> &[PagePlan] {
        &self.pages
    }
    pub const fn layout_epoch(&self) -> LayoutEpoch {
        self.layout_epoch
    }
    pub const fn generated_text(&self) -> &GeneratedTextStore {
        &self.generated_text
    }
    pub fn fingerprint(&self) -> LayoutStateFingerprint {
        materialized_pagination_state_fingerprint_from_jcs(&self.to_jcs())
    }

    fn to_jcs(&self) -> String {
        let mut output = String::from("{\"algorithm\":");
        push_jcs_string(
            &mut output,
            LayoutStateFingerprint::MATERIALIZED_ALGORITHM_ID,
        );
        output.push_str(",\"flow_positions\":[");
        encode_flow_positions(&mut output, &self.flow_positions);
        output.push_str("],\"layout_epoch\":");
        encode_layout_epoch(&mut output, self.layout_epoch);
        output.push_str(",\"pages\":[");
        for (page_index, page) in self.pages.iter().enumerate() {
            if page_index > 0 {
                output.push(',');
            }
            encode_page(&mut output, page);
        }
        output.push_str("],\"placed_anchors\":[");
        for (index, anchor) in self.placed_anchors.iter().enumerate() {
            comma(&mut output, index);
            encode_placed_anchor(&mut output, anchor);
        }
        output.push_str("],\"resolved_generated_text\":[");
        for (index, buffer) in self.generated_text.buffers().iter().enumerate() {
            comma(&mut output, index);
            output.push_str("{\"end_byte\":");
            output.push_str(&buffer.utf8().len().to_string());
            output.push_str(",\"key\":");
            encode_generated_key(&mut output, buffer.key());
            output.push_str(",\"start_byte\":0,\"utf8\":");
            push_jcs_string(&mut output, buffer.utf8());
            output.push('}');
        }
        output.push_str("]}");
        output
    }
}

fn compare_flow_position(left: &FlowPosition, right: &FlowPosition) -> core::cmp::Ordering {
    (
        left.global_flow_ordinal(),
        left.owner(),
        left.block_child_path(),
        left.owner_local_boundary(),
    )
        .cmp(&(
            right.global_flow_ordinal(),
            right.owner(),
            right.block_child_path(),
            right.owner_local_boundary(),
        ))
}

fn canonicalize_placed_anchors(anchors: &mut [PlacedAnchor]) {
    anchors.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
}

fn validate_materialization_chain(flow: &FlowTree, pages: &[PageBudgetSummary]) -> bool {
    let Some(first_position) = flow.positions().first() else {
        return false;
    };
    let Some(terminal_position) = flow.positions().last() else {
        return false;
    };
    if pages.first().map(|page| &page.page_start) != Some(first_position) {
        return false;
    }

    // The canonical empty document is one default blank page. Its sole flow
    // position is both the document start and terminal boundary, so it needs
    // no synthetic Fragmenter result.
    if flow.positions().len() == 1 {
        return pages.len() == 1 && !pages[0].fragmenter_invoked && pages[0].continuation.is_none();
    }

    for (index, page) in pages.iter().enumerate() {
        if !page.fragmenter_invoked {
            return false;
        }
        let actual_next = pages.get(index + 1).map(|next| &next.page_start);
        if page.next_page_start.as_ref() != actual_next {
            return false;
        }
        match page.continuation.as_ref() {
            Some(Continuation::More(cursor)) => {
                if cursor.epoch() != flow.epoch()
                    || !flow.contains_position(cursor.position())
                    || actual_next != Some(cursor.position())
                    || (cursor.is_end() && page.footnote_evaluation.is_none())
                    || (cursor.position() == &page.page_start && !page.footnote_cursor_progress)
                {
                    return false;
                }
            }
            Some(Continuation::Exhausted(cursor)) => {
                if cursor.epoch() != flow.epoch()
                    || !cursor.is_end()
                    || cursor.position() != terminal_position
                    || (index + 1 != pages.len()
                        && (!page.footnote_cursor_progress
                            || actual_next != Some(cursor.position())))
                {
                    return false;
                }
            }
            None => return false,
        }
    }
    true
}

fn validate_materialized_anchors(flow: &FlowTree, pages: &[PageBudgetSummary]) -> bool {
    let mut ids = BTreeSet::new();
    for page in pages {
        for anchor in &page.placed_anchors {
            let Some(frame) = page.frames.iter().find(|frame| {
                frame.kind == anchor.frame_kind && frame.column_index == anchor.column_index
            }) else {
                return false;
            };
            if anchor.page_index != page.page_index
                || flow.anchor_owner(&anchor.anchor_id) != Some(anchor.owner_node)
                || !ids.insert(anchor.anchor_id.clone())
                || anchor.position_on_page(frame.bounds).is_none()
                || anchor.position_in_frame.x.raw() < 0
                || anchor.position_in_frame.y.raw() < 0
                || anchor.position_in_frame.x.raw() > frame.bounds.width().get().raw()
                || anchor.position_in_frame.y.raw() > frame.bounds.height().get().raw()
            {
                return false;
            }
        }
    }
    ids == flow
        .anchors()
        .map(|(anchor_id, _)| anchor_id.clone())
        .collect()
}

fn validate_fingerprint_anchors(
    flow: &FlowTree,
    pages: &[PagePlan],
    anchors: &[PlacedAnchor],
) -> bool {
    let mut ids = BTreeSet::new();
    let valid = anchors.iter().all(|anchor| {
        let Some(page) = pages.get(anchor.page_index as usize) else {
            return false;
        };
        if page.page_index != anchor.page_index {
            return false;
        }
        let Some(frame) = page.frames.iter().find(|frame| {
            frame.kind == anchor.frame_kind && frame.column_index == anchor.column_index
        }) else {
            return false;
        };
        flow.anchor_owner(&anchor.anchor_id) == Some(anchor.owner_node)
            && ids.insert(anchor.anchor_id.clone())
            && anchor.position_on_page(frame.bounds).is_some()
            && anchor.position_in_frame.x.raw() >= 0
            && anchor.position_in_frame.y.raw() >= 0
            && anchor.position_in_frame.x.raw() <= frame.bounds.width().get().raw()
            && anchor.position_in_frame.y.raw() <= frame.bounds.height().get().raw()
    });
    valid
        && ids
            == flow
                .anchors()
                .map(|(anchor_id, _)| anchor_id.clone())
                .collect()
}

fn compare_resolved_reference(
    left: &ResolvedReference,
    right: &ResolvedReference,
) -> core::cmp::Ordering {
    let left_span = left.provenance.text_span().range();
    let right_span = right.provenance.text_span().range();
    (
        left.provenance.buffer_key(),
        left_span.start_byte(),
        left_span.end_byte(),
        &left.anchor_id,
    )
        .cmp(&(
            right.provenance.buffer_key(),
            right_span.start_byte(),
            right_span.end_byte(),
            &right.anchor_id,
        ))
}

fn validate_flow_position(position: &FlowPosition) -> Result<(), PaginationError> {
    if position.global_flow_ordinal() > JSON_SAFE_INTEGER_MAX as u64 {
        return Err(PaginationError::FingerprintIntegerOutOfRange);
    }
    Ok(())
}

fn validate_rect(rect: Rect) -> Result<(), PaginationError> {
    for value in [
        rect.x().raw(),
        rect.y().raw(),
        rect.width().get().raw(),
        rect.height().get().raw(),
    ] {
        if !(-JSON_SAFE_INTEGER_MAX..=JSON_SAFE_INTEGER_MAX).contains(&value) {
            return Err(PaginationError::FingerprintIntegerOutOfRange);
        }
    }
    Ok(())
}

fn encode_page(output: &mut String, page: &PagePlan) {
    output.push_str("{\"column_decisions\":[");
    for (index, decision) in page.column_decisions.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"bounds\":");
        encode_rect(output, decision.bounds);
        output.push_str(",\"column_index\":");
        output.push_str(&decision.column_index.to_string());
        output.push_str(",\"container\":");
        output.push_str(&decision.container.get().to_string());
        output.push('}');
    }
    output.push_str("],\"float_decisions\":[");
    for (index, decision) in page.float_decisions.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"bounds\":");
        encode_rect(output, decision.bounds);
        output.push_str(",\"column_index\":");
        output.push_str(&decision.column_index.to_string());
        output.push_str(",\"frame_kind\":");
        push_jcs_string(output, frame_kind_name(decision.frame_kind));
        output.push_str(",\"owner\":");
        output.push_str(&decision.owner.get().to_string());
        output.push_str(",\"owner_local_ordinal\":");
        output.push_str(&decision.owner_local_ordinal.to_string());
        output.push('}');
    }
    output.push_str("],\"footnote_ids\":[");
    for (index, footnote) in page.footnote_ids.iter().enumerate() {
        comma(output, index);
        push_jcs_string(output, footnote.as_str());
    }
    output.push_str("],\"fragments\":[");
    for (index, fragment) in page.fragments.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"bounds\":");
        encode_rect(output, fragment.bounds);
        output.push_str(",\"column_index\":");
        output.push_str(&fragment.column_index.to_string());
        output.push_str(",\"end\":");
        encode_flow_position(output, &fragment.end);
        output.push_str(",\"frame_kind\":");
        push_jcs_string(output, frame_kind_name(fragment.frame_kind));
        output.push_str(",\"owner\":");
        output.push_str(&fragment.owner.get().to_string());
        output.push_str(",\"owner_local_ordinal\":");
        output.push_str(&fragment.owner_local_ordinal.to_string());
        output.push_str(",\"start\":");
        encode_flow_position(output, &fragment.start);
        output.push('}');
    }
    output.push_str("],\"frames\":[");
    for (index, frame) in page.frames.iter().enumerate() {
        comma(output, index);
        output.push_str("{\"bounds\":");
        encode_rect(output, frame.bounds);
        output.push_str(",\"column_index\":");
        output.push_str(&frame.column_index.to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(output, frame_kind_name(frame.kind));
        output.push('}');
    }
    output.push_str("],\"master_id\":");
    push_jcs_string(output, page.master_id.as_str());
    output.push_str(",\"page_index\":");
    output.push_str(&page.page_index.to_string());
    output.push_str(",\"resolved_references\":[");
    for (index, reference) in page.resolved_references.iter().enumerate() {
        comma(output, index);
        let provenance = reference.provenance;
        let span = provenance.text_span().range();
        output.push_str("{\"anchor_id\":");
        push_jcs_string(output, reference.anchor_id.as_str());
        output.push_str(",\"buffer_key\":");
        encode_generated_key(output, provenance.buffer_key());
        output.push_str(",\"end_byte\":");
        output.push_str(&span.end_byte().get().to_string());
        output.push_str(",\"start_byte\":");
        output.push_str(&span.start_byte().get().to_string());
        output.push_str(",\"utf8\":");
        push_jcs_string(output, &reference.utf8);
        output.push('}');
    }
    output.push_str("]}");
}

fn encode_placed_anchor(output: &mut String, anchor: &PlacedAnchor) {
    output.push_str("{\"anchor_id\":");
    push_jcs_string(output, anchor.anchor_id.as_str());
    output.push_str(",\"column_index\":");
    output.push_str(&anchor.column_index.to_string());
    output.push_str(",\"frame_kind\":");
    push_jcs_string(output, frame_kind_name(anchor.frame_kind));
    output.push_str(",\"owner\":");
    output.push_str(&anchor.owner_node.get().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&anchor.page_index.to_string());
    output.push_str(",\"position_in_frame\":{\"x\":");
    output.push_str(&anchor.position_in_frame.x.raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&anchor.position_in_frame.y.raw().to_string());
    output.push_str("}}");
}

fn comma(output: &mut String, index: usize) {
    if index > 0 {
        output.push(',');
    }
}

fn encode_rect(output: &mut String, rect: Rect) {
    output.push_str("{\"height\":");
    output.push_str(&rect.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&rect.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&rect.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&rect.y().raw().to_string());
    output.push('}');
}

fn encode_flow_position(output: &mut String, position: &FlowPosition) {
    output.push_str("{\"block_child_path\":[");
    for (index, child) in position.block_child_path().iter().enumerate() {
        comma(output, index);
        output.push_str(&child.to_string());
    }
    output.push_str("],\"epoch\":");
    encode_layout_epoch(output, position.epoch());
    output.push_str(",\"global_flow_ordinal\":");
    output.push_str(&position.global_flow_ordinal().to_string());
    output.push_str(",\"owner\":");
    output.push_str(&position.owner().get().to_string());
    output.push_str(",\"owner_local_boundary\":");
    output.push_str(&position.owner_local_boundary().to_string());
    output.push('}');
}

fn encode_flow_positions(output: &mut String, positions: &[FlowPosition]) {
    for (index, position) in positions.iter().enumerate() {
        comma(output, index);
        encode_flow_position(output, position);
    }
}

fn encode_layout_epoch(output: &mut String, epoch: LayoutEpoch) {
    output.push_str("{\"admitted_resources_sha256\":");
    push_hex(output, epoch.admitted_resources().bytes());
    output.push_str(",\"document_sha256\":");
    push_hex(output, epoch.document().bytes());
    output.push_str(",\"resolved_input_sha256\":");
    push_hex(output, epoch.references().bytes());
    output.push_str(",\"style_page_master_sha256\":");
    push_hex(output, epoch.style().bytes());
    output.push('}');
}

fn push_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

fn encode_generated_key(output: &mut String, key: GeneratedBufferKey) {
    push_generated_buffer_key_jcs(output, key);
}

const fn frame_kind_name(kind: PageFrameKind) -> &'static str {
    match kind {
        PageFrameKind::Body => "body",
        PageFrameKind::Header => "header",
        PageFrameKind::Footer => "footer",
        PageFrameKind::Footnote => "footnote",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CostComponents {
    keep: i64,
    widow_orphan: i64,
    heading_isolation: i64,
    table_split: i64,
    footnote_split: i64,
    unused_space: i64,
    overflow: i64,
}
impl CostComponents {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        keep: i64,
        widow_orphan: i64,
        heading_isolation: i64,
        table_split: i64,
        footnote_split: i64,
        unused_space: i64,
        overflow: i64,
    ) -> Option<Self> {
        let values = [
            keep,
            widow_orphan,
            heading_isolation,
            table_split,
            footnote_split,
            unused_space,
            overflow,
        ];
        let mut index = 0;
        let mut total = 0i64;
        while index < values.len() {
            let value = values[index];
            if value < -JSON_SAFE_INTEGER_MAX || value > JSON_SAFE_INTEGER_MAX {
                return None;
            }
            total = match total.checked_add(value) {
                Some(total) => total,
                None => return None,
            };
            if total < -JSON_SAFE_INTEGER_MAX || total > JSON_SAFE_INTEGER_MAX {
                return None;
            }
            index += 1;
        }
        Some(Self {
            keep,
            widow_orphan,
            heading_isolation,
            table_split,
            footnote_split,
            unused_space,
            overflow,
        })
    }
    pub const fn keep(self) -> i64 {
        self.keep
    }
    pub const fn widow_orphan(self) -> i64 {
        self.widow_orphan
    }
    pub const fn heading_isolation(self) -> i64 {
        self.heading_isolation
    }
    pub const fn table_split(self) -> i64 {
        self.table_split
    }
    pub const fn footnote_split(self) -> i64 {
        self.footnote_split
    }
    pub const fn unused_space(self) -> i64 {
        self.unused_space
    }
    pub const fn overflow(self) -> i64 {
        self.overflow
    }
    pub const fn total(self) -> i64 {
        self.keep
            + self.widow_orphan
            + self.heading_isolation
            + self.table_split
            + self.footnote_split
            + self.unused_space
            + self.overflow
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FallbackScore {
    hard_violations: u32,
    components: CostComponents,
    total_cost: i64,
}
impl FallbackScore {
    const fn new(hard_violations: u32, components: CostComponents) -> Self {
        Self {
            hard_violations,
            components,
            total_cost: components.total(),
        }
    }
    pub const fn hard_violations(self) -> u32 {
        self.hard_violations
    }
    pub const fn total_cost(self) -> i64 {
        self.total_cost
    }
    pub const fn components(self) -> CostComponents {
        self.components
    }
}

/// State 0 is the unmaterialized seed; pass i produces materialized state i + 1.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LayoutStateIndex(u16);
impl LayoutStateIndex {
    pub const INITIAL: Self = Self(0);
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MaterializedStateIndex(NonZeroU16);
impl MaterializedStateIndex {
    pub const fn new(value: u16) -> Option<Self> {
        match NonZeroU16::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
    pub const fn get(self) -> u16 {
        self.0.get()
    }
    pub const fn pass_index(self) -> u16 {
        self.0.get() - 1
    }
    fn from_pass_index(pass_index: u16) -> Option<Self> {
        match pass_index.checked_add(1) {
            Some(state) => Self::new(state),
            None => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutPass {
    pass_index: u16,
    materialized_state: MaterializedStateIndex,
    input_fingerprint: LayoutStateFingerprint,
    output_fingerprint: LayoutStateFingerprint,
    fingerprint_record: PaginationFingerprintRecord,
    fallback_score: FallbackScore,
    materialization: PassMaterializationReceipt,
    flow: FlowTree,
}
impl LayoutPass {
    pub fn new(
        materialization: PassMaterializationReceipt,
        input_fingerprint: LayoutStateFingerprint,
        flow: &FlowTree,
        pages: Vec<PagePlan>,
        generated_text: GeneratedTextStore,
    ) -> Result<Self, PaginationError> {
        if materialization.summary.input_fingerprint != input_fingerprint
            || materialization.summary.layout_epoch != flow.epoch()
            || materialization.summary.generated_text != generated_text
            || materialization.summary.pages.len() != pages.len()
            || !validate_materialization_chain(flow, &materialization.summary.pages)
            || !validate_materialized_anchors(flow, &materialization.summary.pages)
            || materialization
                .summary
                .pages
                .iter()
                .zip(&pages)
                .any(|(summary, page)| !summary.matches(page))
            || materialization.summary.pages.iter().any(|summary| {
                summary.page_start.epoch() != flow.epoch()
                    || !flow.contains_position(&summary.page_start)
                    || summary.page_start.owner() != summary.flow_owner
                    || !flow.contains_owner(summary.content_owner)
            })
        {
            return Err(PaginationError::InvalidWorkPermit);
        }
        let pass_index = materialization.summary.pass_index;
        let fallback_score = materialization.summary.fallback_score;
        let placed_anchors = materialization
            .summary
            .pages
            .iter()
            .flat_map(|page| page.placed_anchors.iter().cloned())
            .collect();
        let fingerprint_record =
            PaginationFingerprintRecord::new(flow, pages, generated_text, placed_anchors)?;
        let output_fingerprint = fingerprint_record.fingerprint();
        let materialized_state = MaterializedStateIndex::from_pass_index(pass_index)
            .ok_or(PaginationError::PassIndexOverflow)?;
        Ok(Self {
            pass_index,
            materialized_state,
            input_fingerprint,
            output_fingerprint,
            fingerprint_record,
            fallback_score,
            materialization,
            flow: flow.clone(),
        })
    }
    pub const fn pass_index(&self) -> u16 {
        self.pass_index
    }
    pub const fn materialized_state(&self) -> MaterializedStateIndex {
        self.materialized_state
    }
    pub const fn input_fingerprint(&self) -> LayoutStateFingerprint {
        self.input_fingerprint
    }
    pub const fn output_fingerprint(&self) -> LayoutStateFingerprint {
        self.output_fingerprint
    }
    pub fn pages(&self) -> &[PagePlan] {
        self.fingerprint_record.pages()
    }
    pub const fn fingerprint_record(&self) -> &PaginationFingerprintRecord {
        &self.fingerprint_record
    }
    pub const fn fallback_score(&self) -> FallbackScore {
        self.fallback_score
    }
    pub const fn generated_text(&self) -> &GeneratedTextStore {
        self.fingerprint_record.generated_text()
    }
    pub const fn materialization(&self) -> &PassMaterializationReceipt {
        &self.materialization
    }
    pub const fn flow(&self) -> &FlowTree {
        &self.flow
    }
    pub fn placed_anchors(&self) -> impl Iterator<Item = &PlacedAnchor> {
        self.materialization
            .summary
            .pages
            .iter()
            .flat_map(|page| page.placed_anchors.iter())
    }

    /// Derives the exact generated-text overlay used by the next pass from
    /// this materialized state. Page references are resolved from the sealed
    /// placed-anchor set using checked physical page numbers. All other site
    /// bytes remain the package-validated canonical predecessor values.
    pub fn transition_references<'a>(
        &'a self,
        package: &ValidatedParsedPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<ReferenceTransitionReceipt<'a>, PaginationError> {
        package
            .bind_generated_text(self.generated_text(), limits)
            .map_err(|_| PaginationError::PackageEpochMismatch)?;
        let next_generated_text = resolve_next_generated_text(
            package,
            self.generated_text(),
            self.placed_anchors(),
            limits,
        )?;
        let generated = package
            .bind_generated_text(&next_generated_text, limits)
            .map_err(|_| PaginationError::PackageEpochMismatch)?;
        let working_epoch = self
            .fingerprint_record
            .layout_epoch
            .with_generated_text(generated)
            .map_err(|_| PaginationError::PackageEpochMismatch)?;
        Ok(ReferenceTransitionReceipt {
            session: self.materialization.summary.session.clone(),
            previous_state: self.materialized_state,
            previous_fingerprint: self.output_fingerprint,
            working_epoch,
            generated_text: Cow::Owned(next_generated_text),
        })
    }
}

fn resolve_next_generated_text<'a>(
    package: &ValidatedParsedPackage,
    previous: &GeneratedTextStore,
    placed_anchors: impl IntoIterator<Item = &'a PlacedAnchor>,
    limits: &ValidatedResourceLimits,
) -> Result<GeneratedTextStore, PaginationError> {
    let anchors: BTreeMap<_, _> = placed_anchors
        .into_iter()
        .map(|anchor| (anchor.anchor_id().clone(), anchor.page_index()))
        .collect();
    let mut drafts = Vec::new();
    drafts
        .try_reserve_exact(package.document_nodes().generated_sites().len())
        .map_err(|_| PaginationError::ResourceLimit)?;
    for site in package.document_nodes().generated_sites() {
        let key = site.key();
        if key.generation_kind() == GenerationKind::ListMarker {
            drafts.push(
                package
                    .materialize_list_marker(key)
                    .map_err(|_| PaginationError::PackageEpochMismatch)?,
            );
            continue;
        }
        let utf8 = if key.generation_kind() == GenerationKind::PageReference {
            let GeneratedSiteTarget::Anchor(anchor_id) = site.target() else {
                return Err(PaginationError::PackageEpochMismatch);
            };
            match anchors.get(anchor_id) {
                Some(page_index) => page_index
                    .checked_add(1)
                    .ok_or(PaginationError::InvalidPageIndex)?
                    .to_string(),
                None => String::new(),
            }
        } else {
            previous
                .buffers()
                .iter()
                .find(|buffer| buffer.key() == key)
                .map(|buffer| buffer.utf8().to_owned())
                .ok_or(PaginationError::PackageEpochMismatch)?
        };
        drafts.push(
            GeneratedBufferDraft::new(package.document_nodes(), key, utf8)
                .map_err(|_| PaginationError::PackageEpochMismatch)?,
        );
    }
    let next_generated_text = GeneratedTextStore::new(
        drafts,
        package.document_nodes(),
        limits,
        &package.package().text_store,
    )
    .map_err(|_| PaginationError::PackageEpochMismatch)?;
    package
        .bind_generated_text(&next_generated_text, limits)
        .map_err(|_| PaginationError::PackageEpochMismatch)?;
    Ok(next_generated_text)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConvergenceStatus {
    Converged,
    CycleFallback {
        cycle_start_state: MaterializedStateIndex,
    },
    MaxPassFallback,
}

/// Selected page/master dimensions captured while the validated master set is
/// still in scope. Downstream crates consume this receipt instead of reopening
/// page-master selection from presentation data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedPageGeometry {
    page_index: u32,
    master_id: MasterId,
    width: PositiveLength,
    height: PositiveLength,
}
impl SelectedPageGeometry {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn width(&self) -> PositiveLength {
        self.width
    }
    pub const fn height(&self) -> PositiveLength {
        self.height
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationResult {
    passes: Vec<LayoutPass>,
    status: ConvergenceStatus,
    selected_state: MaterializedStateIndex,
    selected_page_geometry: Vec<SelectedPageGeometry>,
    work_receipt: PaginationBudgetReceipt,
}
impl PaginationResult {
    fn new(
        passes: Vec<LayoutPass>,
        status: ConvergenceStatus,
        input: &PaginationInput<'_>,
        work_receipt: PaginationBudgetReceipt,
    ) -> Result<Self, PaginationError> {
        let options = input.options();
        if options.max_pages == 0
            || options.max_layout_passes == 0
            || options.max_page_break_lookback == 0
        {
            return Err(PaginationError::InvalidOptions);
        }
        if passes.len() > usize::from(options.max_layout_passes)
            || passes
                .iter()
                .any(|pass| pass.pages().len() > options.max_pages as usize)
        {
            return Err(PaginationError::ResourceLimit);
        }
        for pass in &passes {
            let fragments = pass.pages().iter().try_fold(0u64, |count, page| {
                count.checked_add(page.fragments.len() as u64)
            });
            if !matches!(fragments, Some(count) if count <= options.max_fragments) {
                return Err(PaginationError::ResourceLimit);
            }
        }
        if matches!(status, ConvergenceStatus::MaxPassFallback)
            && passes.len() != usize::from(options.max_layout_passes)
        {
            return Err(PaginationError::InvalidMaxPassState);
        }
        validate_work_receipt(&passes, input, &work_receipt)?;
        let known_masters: BTreeMap<&MasterId, _> = input
            .page_masters()
            .masters
            .iter()
            .map(|master| (&master.master_id, master))
            .collect();
        for page in passes.iter().flat_map(|pass| pass.pages()) {
            let master = known_masters
                .get(&page.master_id)
                .ok_or(PaginationError::UnknownPageMaster)?;
            for frame in &page.frames {
                let region = match frame.kind {
                    PageFrameKind::Body => Some(master.body),
                    PageFrameKind::Header => master.header,
                    PageFrameKind::Footer => master.footer,
                    PageFrameKind::Footnote => master.footnote,
                }
                .ok_or(PaginationError::MissingMasterFrame)?;
                if !rect_contains(region, frame.bounds) {
                    return Err(PaginationError::FrameOutsideMaster);
                }
            }
        }
        validate_pass_chain(&passes, input.initial_fingerprint())?;
        validate_terminal_status(&passes, &status, options)?;
        let selected_state = match &status {
            ConvergenceStatus::Converged => {
                let last = passes.last().ok_or(PaginationError::EmptyPasses)?;
                last.materialized_state()
            }
            ConvergenceStatus::CycleFallback { .. } => {
                if options.strict {
                    return Err(PaginationError::FallbackRejectedByStrict);
                }
                select_fallback_state(&passes)?
            }
            ConvergenceStatus::MaxPassFallback => {
                if options.strict {
                    return Err(PaginationError::FallbackRejectedByStrict);
                }
                select_fallback_state(&passes)?
            }
        };
        let selected_page_geometry = passes[usize::from(selected_state.pass_index())]
            .pages()
            .iter()
            .map(|page| {
                let master = known_masters
                    .get(&page.master_id)
                    .ok_or(PaginationError::UnknownPageMaster)?;
                Ok(SelectedPageGeometry {
                    page_index: page.page_index,
                    master_id: page.master_id.clone(),
                    width: master.width,
                    height: master.height,
                })
            })
            .collect::<Result<Vec<_>, PaginationError>>()?;
        Ok(Self {
            passes,
            status,
            selected_state,
            selected_page_geometry,
            work_receipt,
        })
    }

    pub fn passes(&self) -> &[LayoutPass] {
        &self.passes
    }
    pub const fn status(&self) -> &ConvergenceStatus {
        &self.status
    }
    pub const fn selected_state(&self) -> MaterializedStateIndex {
        self.selected_state
    }
    pub fn selected_pass(&self) -> &LayoutPass {
        &self.passes[usize::from(self.selected_state.pass_index())]
    }
    pub fn selected_pages(&self) -> &[PagePlan] {
        self.selected_pass().pages()
    }
    pub fn selected_flow(&self) -> &FlowTree {
        self.selected_pass().flow()
    }
    pub fn selected_anchors(&self) -> impl Iterator<Item = &PlacedAnchor> {
        self.selected_pass().placed_anchors()
    }
    pub fn selected_page_geometry(&self) -> &[SelectedPageGeometry] {
        &self.selected_page_geometry
    }
    pub fn final_fingerprint(&self) -> LayoutStateFingerprint {
        self.selected_pass().output_fingerprint()
    }
    pub const fn fallback_policy(&self) -> FallbackPolicy {
        FallbackPolicy::LowestCostThenEarliest
    }
    pub const fn work_receipt(&self) -> &PaginationBudgetReceipt {
        &self.work_receipt
    }
}

fn validate_work_receipt(
    passes: &[LayoutPass],
    input: &PaginationInput<'_>,
    receipt: &PaginationBudgetReceipt,
) -> Result<(), PaginationError> {
    if !receipt.session.same_as(&input.session)
        || receipt.initial_fingerprint != input.initial_fingerprint()
        || receipt.layout_epoch != input.initial_state().layout_epoch()
        || receipt.passes.len() != passes.len()
        || receipt
            .passes
            .iter()
            .zip(passes)
            .any(|(summary, pass)| summary != &pass.materialization().summary)
    {
        return Err(PaginationError::InvalidWorkPermit);
    }
    Ok(())
}

fn validate_page_frames_for_context(
    page: &PageContext,
    frames: &[PageFramePlan],
) -> Result<(), PaginationError> {
    if !matches!(
        frames.first(),
        Some(frame) if frame.kind == PageFrameKind::Body && frame.column_index == 0
    ) {
        return Err(PaginationError::MissingBodyFrame);
    }

    let master = page.selected_master();
    let mut previous: Option<(PageFrameKind, u32)> = None;
    for frame in frames {
        validate_rect(frame.bounds)?;
        let expected_column = match previous {
            Some((kind, column)) if kind == frame.kind => column
                .checked_add(1)
                .ok_or(PaginationError::InvalidFrameColumn)?,
            Some((kind, _)) if kind < frame.kind => 0,
            None => 0,
            Some(_) => return Err(PaginationError::InvalidFrameColumn),
        };
        if frame.column_index != expected_column {
            return Err(PaginationError::InvalidFrameColumn);
        }
        let region = match frame.kind {
            PageFrameKind::Body => Some(master.body),
            PageFrameKind::Header => master.header,
            PageFrameKind::Footer => master.footer,
            PageFrameKind::Footnote => master.footnote,
        }
        .ok_or(PaginationError::MissingMasterFrame)?;
        if !rect_contains(region, frame.bounds) {
            return Err(PaginationError::FrameOutsideMaster);
        }
        previous = Some((frame.kind, frame.column_index));
    }
    Ok(())
}

fn rect_contains(outer: Rect, inner: Rect) -> bool {
    let Some(outer_right) = outer.x().raw().checked_add(outer.width().get().raw()) else {
        return false;
    };
    let Some(outer_bottom) = outer.y().raw().checked_add(outer.height().get().raw()) else {
        return false;
    };
    let Some(inner_right) = inner.x().raw().checked_add(inner.width().get().raw()) else {
        return false;
    };
    let Some(inner_bottom) = inner.y().raw().checked_add(inner.height().get().raw()) else {
        return false;
    };
    inner.x().raw() >= outer.x().raw()
        && inner.y().raw() >= outer.y().raw()
        && inner_right <= outer_right
        && inner_bottom <= outer_bottom
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationOutcome {
    result: PaginationResult,
    diagnostics: Vec<AdvisoryDiagnostic>,
}
impl PaginationOutcome {
    pub fn new(
        passes: Vec<LayoutPass>,
        status: ConvergenceStatus,
        input: &PaginationInput<'_>,
        work_receipt: PaginationBudgetReceipt,
    ) -> Result<Self, PaginationError> {
        let fallback = !matches!(status, ConvergenceStatus::Converged);
        let result = PaginationResult::new(passes, status, input, work_receipt)?;
        let diagnostics = if fallback {
            let code =
                DiagnosticCode::new("G6001").ok_or(PaginationError::InvalidFallbackDiagnostic)?;
            let owner = input.package_context.document_node_id();
            let diagnostic = DiagnosticBuilder::located(
                code,
                Severity::Warning,
                "pagination selected a materialized fallback state",
                DiagnosticLocation::source(
                    SourceDiagnosticLocation::new(None, None, Some(owner))
                        .expect("the document owner forms a source location"),
                ),
            )
            .map_err(|_| PaginationError::InvalidFallbackDiagnostic)?
            .subject(DiagnosticSubject::Layout(LayoutErrorSubject::new(
                owner, None,
            )))
            .build();
            vec![AdvisoryDiagnostic::new(diagnostic)
                .map_err(|_| PaginationError::InvalidFallbackDiagnostic)?]
        } else {
            vec![]
        };
        Ok(Self {
            result,
            diagnostics,
        })
    }
    pub const fn result(&self) -> &PaginationResult {
        &self.result
    }
    pub fn diagnostics(&self) -> &[AdvisoryDiagnostic] {
        &self.diagnostics
    }
    pub fn into_result(self) -> PaginationResult {
        self.result
    }
}

fn validate_pass_chain(
    passes: &[LayoutPass],
    initial_fingerprint: LayoutStateFingerprint,
) -> Result<(), PaginationError> {
    if passes.is_empty() {
        return Err(PaginationError::EmptyPasses);
    }
    for (expected_index, pass) in passes.iter().enumerate() {
        let expected_index =
            u16::try_from(expected_index).map_err(|_| PaginationError::PassIndexOverflow)?;
        if pass.pass_index() != expected_index {
            return Err(PaginationError::InvalidPassIndex);
        }
        if expected_index == 0 && pass.input_fingerprint() != initial_fingerprint {
            return Err(PaginationError::InitialFingerprintMismatch);
        }
        if let Some(previous) = expected_index
            .checked_sub(1)
            .and_then(|index| passes.get(usize::from(index)))
        {
            if pass.input_fingerprint() != previous.output_fingerprint() {
                return Err(PaginationError::BrokenFingerprintChain);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObservedTermination {
    Stable,
    Cycle(MaterializedStateIndex),
}

fn observed_termination(passes: &[LayoutPass]) -> Option<ObservedTermination> {
    let last = passes.last()?;
    if last.output_fingerprint() == last.input_fingerprint() {
        return Some(ObservedTermination::Stable);
    }
    let repeated = last.output_fingerprint();
    passes
        .iter()
        .take(passes.len() - 1)
        .find(|pass| pass.output_fingerprint() == repeated)
        .map(|pass| ObservedTermination::Cycle(pass.materialized_state()))
}

fn validate_terminal_status(
    passes: &[LayoutPass],
    status: &ConvergenceStatus,
    options: PaginationOptions,
) -> Result<(), PaginationError> {
    for prefix_len in 1..passes.len() {
        if observed_termination(&passes[..prefix_len]).is_some() {
            return Err(PaginationError::PassesContinueAfterTermination);
        }
    }
    let observed = observed_termination(passes);
    match (status, observed) {
        (ConvergenceStatus::Converged, Some(ObservedTermination::Stable)) => Ok(()),
        (
            ConvergenceStatus::CycleFallback { cycle_start_state },
            Some(ObservedTermination::Cycle(observed_start)),
        ) if *cycle_start_state == observed_start => Ok(()),
        (ConvergenceStatus::MaxPassFallback, None)
            if passes.len() == usize::from(options.max_layout_passes) =>
        {
            Ok(())
        }
        (ConvergenceStatus::Converged, _) => Err(PaginationError::InvalidConvergenceStatus),
        (ConvergenceStatus::CycleFallback { .. }, _) => Err(PaginationError::InvalidCycleState),
        (ConvergenceStatus::MaxPassFallback, _) => Err(PaginationError::InvalidMaxPassState),
    }
}

fn select_fallback_state(passes: &[LayoutPass]) -> Result<MaterializedStateIndex, PaginationError> {
    passes
        .iter()
        .min_by_key(|pass| {
            (
                pass.fallback_score().hard_violations(),
                pass.fallback_score().total_cost(),
                pass.pages().len(),
                pass.materialized_state(),
            )
        })
        .map(LayoutPass::materialized_state)
        .ok_or(PaginationError::EmptyPasses)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaginationError {
    MissingDefaultMaster,
    PageLimit,
    NoProgress,
    ArithmeticOverflow,
    ResourceLimit,
    FatalLayout,
    InvalidOptions,
    InvalidPreparedLayout,
    EmptyPasses,
    PassIndexOverflow,
    InvalidPassIndex,
    BrokenFingerprintChain,
    InvalidConvergenceStatus,
    InvalidCycleState,
    InvalidMaxPassState,
    FallbackRejectedByStrict,
    EmptyPages,
    InvalidPageIndex,
    UnknownPageMaster,
    DuplicateResolvedReference,
    DuplicateFootnote,
    InvalidFootnoteClosure,
    DuplicateFragmentPosition,
    MissingBodyFrame,
    InvalidFrameColumn,
    InvalidFragmentRange,
    DuplicateFloatDecision,
    InvalidColumnDecision,
    FingerprintIntegerOutOfRange,
    InitialFingerprintMismatch,
    PassesContinueAfterTermination,
    InvalidFallbackDiagnostic,
    EpochMismatch,
    MissingMasterFrame,
    FrameOutsideMaster,
    UnknownFrameReference,
    FragmentOutsideFrame,
    UnknownFlowOwner,
    WorkBudgetAlreadyIssued,
    InvalidWorkPermit,
    PackageEpochMismatch,
    UnsupportedReferenceTransition,
    InvalidInitialReferenceSeed,
    Footnote(StagingFootnotePaginationError),
}

pub trait Paginator {
    fn paginate<F: Fragmenter, P: LayoutPassProvider>(
        &self,
        input: PaginationInput<'_>,
        pass_provider: &P,
        fragmenter: &F,
    ) -> Result<PaginationOutcome, PaginationError>;
}

#[derive(Debug)]
struct EvaluatedFootnoteBodyPage {
    fragments: Vec<FragmentDraft>,
    continuation: FlowCursor,
    terminal: bool,
    reference_owners: Vec<NodeId>,
    anchors: Vec<DiscoveredAnchor>,
    fingerprint: LayoutStateFingerprint,
}

struct FootnoteCandidateFragmentBudget {
    remaining_fragments: u64,
}

impl FragmentWorkBudget for FootnoteCandidateFragmentBudget {
    fn consume_fragments(&mut self, count: u64) -> Result<(), FragmentError> {
        self.remaining_fragments = self
            .remaining_fragments
            .checked_sub(count)
            .ok_or(FragmentError::ResourceLimit)?;
        Ok(())
    }

    fn consume_footnote_reflow(&mut self, _page_index: u32) -> Result<(), FragmentError> {
        Err(FragmentError::UnsupportedFlowDomain)
    }

    fn consume_column_candidate(&mut self, _container: NodeId) -> Result<(), FragmentError> {
        Err(FragmentError::UnsupportedFlowDomain)
    }

    fn enqueue_float(
        &mut self,
        _owner: NodeId,
        _owner_local_ordinal: u32,
    ) -> Result<(), FragmentError> {
        Err(FragmentError::UnsupportedFlowDomain)
    }

    fn dequeue_float(
        &mut self,
        _owner: NodeId,
        _owner_local_ordinal: u32,
    ) -> Result<(), FragmentError> {
        Err(FragmentError::UnsupportedFlowDomain)
    }

    fn consume_float_carry(
        &mut self,
        _owner: NodeId,
        _owner_local_ordinal: u32,
    ) -> Result<(), FragmentError> {
        Err(FragmentError::UnsupportedFlowDomain)
    }
}

/// Joint result from the publication-only footnote paginator. The ordinary
/// body result and dedicated definition state share one selected global pass.
pub struct FootnotePaginationOutcome {
    outcome: PaginationOutcome,
    registry: StagingFootnoteFlowRegistry,
    selected: ValidatedFootnoteSelectedLayout,
}

impl FootnotePaginationOutcome {
    pub const fn outcome(&self) -> &PaginationOutcome {
        &self.outcome
    }

    pub fn into_parts(
        self,
    ) -> (
        PaginationOutcome,
        StagingFootnoteFlowRegistry,
        ValidatedFootnoteSelectedLayout,
    ) {
        (self.outcome, self.registry, self.selected)
    }
}

fn map_footnote_materialization_error(error: StagingFootnotePaginationError) -> PaginationError {
    PaginationError::Footnote(error)
}

fn map_pagination_to_footnote_error(error: PaginationError) -> StagingFootnotePaginationError {
    match error {
        PaginationError::ResourceLimit => StagingFootnotePaginationError::FragmentLimit,
        PaginationError::PageLimit => StagingFootnotePaginationError::PageLimit,
        PaginationError::ArithmeticOverflow => StagingFootnotePaginationError::ArithmeticOverflow,
        PaginationError::Footnote(error) => error,
        PaginationError::InvalidWorkPermit => StagingFootnotePaginationError::StateMismatch,
        _ => StagingFootnotePaginationError::InvalidBodyCandidate,
    }
}

fn map_pagination_page_to_footnote_error(error: PaginationError) -> StagingFootnotePaginationError {
    match error {
        PaginationError::ResourceLimit | PaginationError::PageLimit => {
            StagingFootnotePaginationError::PageLimit
        }
        error => map_pagination_to_footnote_error(error),
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_footnote_body_page(
    flow: &FlowTree,
    fragmenter: &ReferenceFragmenter<'_>,
    page: &PageContext,
    page_start: &FlowCursor,
    body_frame: Rect,
    evaluation: &StagingFootnotePageEvaluationRequest,
    pass_seed: LayoutStateFingerprint,
    remaining_fragments: u64,
    allow_held_body: bool,
) -> Result<EvaluatedFootnoteBodyPage, StagingFootnotePaginationError> {
    if page.page_index() != evaluation.page_index()
        || page.master_id() != evaluation.master_id()
        || page.page_start() != page_start.position()
        || available_body_block_size(body_frame, evaluation.applied_reservation())?
            != evaluation.available_body_block_size()
    {
        return Err(StagingFootnotePaginationError::InvalidBodyCandidate);
    }
    let mut fragments = Vec::new();
    let mut continuation = page_start.clone();
    let mut terminal = page_start.is_end();
    if !terminal {
        let mut budget = FootnoteCandidateFragmentBudget {
            remaining_fragments,
        };
        let mut cursor = page_start.clone();
        let mut bootstrap = page.page_index() == 0
            && flow.positions().first() == Some(cursor.position())
            && flow.positions().len() > 1;
        loop {
            let request = FragmentRequest::new(
                flow,
                &cursor,
                body_frame,
                evaluation.applied_reservation(),
                page.clone(),
            )
            .map_err(|_| StagingFootnotePaginationError::InvalidBodyCandidate)?;
            let result = match fragmenter.fragment(&request, &mut budget) {
                Ok(result) => result,
                Err(FragmentError::Unplaceable)
                    if allow_held_body
                        && fragments.is_empty()
                        && evaluation.applied_reservation() != NonNegativeLength::ZERO =>
                {
                    // A carry may consume enough of this page to hold the
                    // body, but it must not hide an intrinsically oversize
                    // body line/keep until a later resource limit becomes
                    // primary. Probe the same cursor against the complete
                    // empty body frame before issuing a carry-only page.
                    let full_body_request = FragmentRequest::new(
                        flow,
                        &cursor,
                        body_frame,
                        NonNegativeLength::ZERO,
                        page.clone(),
                    )
                    .map_err(|_| StagingFootnotePaginationError::InvalidBodyCandidate)?;
                    let mut full_body_budget = FootnoteCandidateFragmentBudget {
                        remaining_fragments,
                    };
                    match fragmenter.fragment(&full_body_request, &mut full_body_budget) {
                        Ok(result) => result
                            .validate_progress(&full_body_request)
                            .map_err(|_| StagingFootnotePaginationError::InvalidBodyCandidate)?,
                        Err(FragmentError::Unplaceable) => {
                            return Err(StagingFootnotePaginationError::BodyOversize)
                        }
                        Err(FragmentError::ResourceLimit) => {
                            return Err(StagingFootnotePaginationError::FragmentLimit)
                        }
                        Err(FragmentError::ArithmeticOverflow) => {
                            return Err(StagingFootnotePaginationError::ArithmeticOverflow)
                        }
                        Err(_) => return Err(StagingFootnotePaginationError::BodyEvaluationFailed),
                    }
                    // Incoming carries own composite progress for a carry-only
                    // page. Hold the independently typed body cursor here;
                    // the body Fragmenter must never issue a same-position
                    // `More` continuation itself.
                    continuation = page_start.clone();
                    terminal = false;
                    break;
                }
                Err(error) => {
                    return Err(match error {
                        FragmentError::ResourceLimit => {
                            StagingFootnotePaginationError::FragmentLimit
                        }
                        FragmentError::ArithmeticOverflow => {
                            StagingFootnotePaginationError::ArithmeticOverflow
                        }
                        FragmentError::Unplaceable => StagingFootnotePaginationError::BodyOversize,
                        _ => StagingFootnotePaginationError::BodyEvaluationFailed,
                    });
                }
            };
            result
                .validate_progress(&request)
                .map_err(|_| StagingFootnotePaginationError::InvalidBodyCandidate)?;
            fragments.extend(result.fragments);
            match result.continuation {
                Continuation::More(next)
                    if bootstrap
                        && fragmenter.ends_with_forced_break()
                        && flow.positions().len() == 3 =>
                {
                    continuation = *next;
                    terminal = false;
                    break;
                }
                Continuation::More(next) if bootstrap => {
                    cursor = *next;
                    bootstrap = false;
                }
                Continuation::More(next) => {
                    continuation = *next;
                    terminal = false;
                    break;
                }
                Continuation::Exhausted(end) => {
                    continuation = *end;
                    terminal = true;
                    break;
                }
            }
        }
    }

    if let Some(cut_owner) = evaluation.body_cut_before_reference_owner() {
        let cut_index = fragmenter
            .legal_cut_index_before_reference(&fragments, cut_owner)
            .map_err(|_| StagingFootnotePaginationError::InvalidBodyCandidate)?;
        if let Some(cut_index) = cut_index {
            let cut_position = fragments[cut_index].start().clone();
            fragments.truncate(cut_index);
            continuation = fragmenter
                .cursor_for_position(&cut_position)
                .map_err(|_| StagingFootnotePaginationError::InvalidBodyCandidate)?;
            terminal = false;
        }
    }

    let mut reference_owners = Vec::new();
    let mut anchors = Vec::new();
    for fragment in &fragments {
        reference_owners.extend(
            fragmenter
                .footnote_reference_owners_between(fragment.start(), fragment.end())
                .map_err(|_| StagingFootnotePaginationError::InvalidBodyCandidate)?,
        );
        anchors.extend(
            fragmenter
                .anchors_between(fragment.start(), fragment.end())
                .map_err(|_| StagingFootnotePaginationError::InvalidBodyCandidate)?,
        );
    }
    anchors.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
    if anchors
        .windows(2)
        .any(|pair| pair[0].anchor_id == pair[1].anchor_id)
    {
        return Err(StagingFootnotePaginationError::InvalidBodyCandidate);
    }
    let fingerprint = encode_evaluated_footnote_body_fingerprint(
        pass_seed,
        page.page_index(),
        evaluation,
        &fragments,
        &continuation,
        terminal,
        &reference_owners,
    )?;
    Ok(EvaluatedFootnoteBodyPage {
        fragments,
        continuation,
        terminal,
        reference_owners,
        anchors,
        fingerprint,
    })
}

fn encode_evaluated_footnote_body_fingerprint(
    pass_seed: LayoutStateFingerprint,
    page_index: u32,
    evaluation: &StagingFootnotePageEvaluationRequest,
    fragments: &[FragmentDraft],
    continuation: &FlowCursor,
    terminal: bool,
    reference_owners: &[NodeId],
) -> Result<LayoutStateFingerprint, StagingFootnotePaginationError> {
    let mut output = String::from(
        "{\"algorithm\":\"typaxis.footnote-body-candidate/1\",\"body_cut_before_reference_owner\":",
    );
    match evaluation.body_cut_before_reference_owner() {
        Some(owner) => output.push_str(&owner.get().to_string()),
        None => output.push_str("null"),
    }
    output.push_str(",\"continuation\":{");
    output.push_str("\"flow_ordinal\":");
    output.push_str(&continuation.position().global_flow_ordinal().to_string());
    output.push_str(",\"terminal\":");
    output.push_str(if terminal { "true" } else { "false" });
    output.push_str("},\"fragments\":[");
    for (index, fragment) in fragments.iter().enumerate() {
        comma(&mut output, index);
        output.push_str("{\"bounds\":");
        encode_selected_footnote_rect(&mut output, fragment.bounds());
        output.push_str(",\"end\":");
        output.push_str(&fragment.end().global_flow_ordinal().to_string());
        output.push_str(",\"owner\":");
        output.push_str(&fragment.start().owner().get().to_string());
        output.push_str(",\"start\":");
        output.push_str(&fragment.start().global_flow_ordinal().to_string());
        output.push('}');
    }
    output.push_str("],\"page_index\":");
    output.push_str(&page_index.to_string());
    output.push_str(",\"pass_seed_sha256\":");
    push_hex(&mut output, pass_seed.bytes());
    output.push_str(",\"reference_owners\":[");
    for (index, owner) in reference_owners.iter().enumerate() {
        comma(&mut output, index);
        output.push_str(&owner.get().to_string());
    }
    output.push_str("],\"reservation\":");
    output.push_str(&evaluation.applied_reservation().get().raw().to_string());
    output.push('}');
    Ok(LayoutStateFingerprint::from_untrusted_bytes(sha256(
        output.as_bytes(),
    )))
}

fn selected_footnote_definition_anchors(
    package: &ValidatedParsedPackage,
    flow: &FlowTree,
    registry: &StagingFootnoteFlowRegistry,
    selected: &StagingFootnoteSelectedPageReceipt,
) -> Result<Vec<DiscoveredAnchor>, StagingFootnotePaginationError> {
    if selected.flows().is_empty() {
        if selected.reservation() != NonNegativeLength::ZERO {
            return Err(StagingFootnotePaginationError::StateMismatch);
        }
        return Ok(Vec::new());
    }
    let paragraph_items = flow
        .paragraph_items()
        .ok_or(StagingFootnotePaginationError::RegistryMismatch)?;
    let body_end = registry
        .body_frame()
        .y()
        .checked_add(registry.body_frame().height().get())
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    let actual_start = body_end
        .checked_sub(selected.reservation().get())
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    let separator = Length::from_raw(FOOTNOTE_SEPARATOR_BAND_RAW)
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    let mut y = actual_start
        .checked_add(separator)
        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
    let mut anchors = Vec::new();
    for selected_flow in selected.flows() {
        let registered = registry
            .flow(selected_flow.assignment().flow_id())
            .filter(|registered| {
                registered.binding().footnote_id() == selected_flow.assignment().footnote_id()
            })
            .ok_or(StagingFootnotePaginationError::RegistryMismatch)?;
        let mut lines = Vec::new();
        for owner in registered.block_owners() {
            let computed = package
                .cascade_style(*owner)
                .map_err(|_| StagingFootnotePaginationError::StateMismatch)?;
            let line_height = match computed.computed().properties().get("line_height") {
                Some(StyleValue::Length(value)) => PositiveLength::new(*value),
                _ => None,
            }
            .ok_or(StagingFootnotePaginationError::StateMismatch)?;
            let space_before = match computed.computed().properties().get("space_before") {
                Some(StyleValue::Length(value)) => NonNegativeLength::new(*value),
                None => Some(NonNegativeLength::ZERO),
                Some(_) => None,
            }
            .ok_or(StagingFootnotePaginationError::StateMismatch)?;
            let space_after = match computed.computed().properties().get("space_after") {
                Some(StyleValue::Length(value)) => NonNegativeLength::new(*value),
                None => Some(NonNegativeLength::ZERO),
                Some(_) => None,
            }
            .ok_or(StagingFootnotePaginationError::StateMismatch)?;
            let start_indent = match computed.computed().properties().get("start_indent") {
                Some(StyleValue::Length(value)) => NonNegativeLength::new(*value),
                None => Some(NonNegativeLength::ZERO),
                Some(_) => None,
            }
            .ok_or(StagingFootnotePaginationError::StateMismatch)?;
            let end_indent = match computed.computed().properties().get("end_indent") {
                Some(StyleValue::Length(value)) => NonNegativeLength::new(*value),
                None => Some(NonNegativeLength::ZERO),
                Some(_) => None,
            }
            .ok_or(StagingFootnotePaginationError::StateMismatch)?;
            registry
                .maximum_footnote_frame()
                .width()
                .get()
                .checked_sub(start_indent.get())
                .and_then(|value| value.checked_sub(end_indent.get()))
                .and_then(PositiveLength::new)
                .ok_or(StagingFootnotePaginationError::StateMismatch)?;
            let paragraph_level = paragraph_items
                .paragraph_level(*owner)
                .ok_or(StagingFootnotePaginationError::StateMismatch)?;
            let physical_left_inset = if paragraph_level.get() % 2 == 1 {
                end_indent
            } else {
                start_indent
            };
            let line_count = paragraph_items
                .paragraph_break(*owner)
                .map(|result| result.lines.len())
                .unwrap_or(1)
                .max(1);
            for line_index in 0..line_count {
                let mut extent = line_height.get();
                if line_index == 0 {
                    extent = extent
                        .checked_add(space_before.get())
                        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
                }
                if line_index + 1 == line_count {
                    extent = extent
                        .checked_add(space_after.get())
                        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
                }
                lines.push((
                    *owner,
                    line_index == 0,
                    PositiveLength::new(extent)
                        .ok_or(StagingFootnotePaginationError::StateMismatch)?,
                    if line_index == 0 {
                        space_before
                    } else {
                        NonNegativeLength::ZERO
                    },
                    physical_left_inset,
                ));
            }
        }
        if registered.fragment_extents().len() != registered.fragment_line_counts().len() {
            return Err(StagingFootnotePaginationError::StateMismatch);
        }
        let mut fragment_line_ranges = Vec::new();
        fragment_line_ranges
            .try_reserve_exact(registered.fragment_line_counts().len())
            .map_err(|_| StagingFootnotePaginationError::AllocationFailure)?;
        let mut line_cursor = 0usize;
        for (extent, line_count) in registered
            .fragment_extents()
            .iter()
            .copied()
            .zip(registered.fragment_line_counts())
        {
            let line_count = usize::try_from(line_count.get())
                .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?;
            let line_end = line_cursor
                .checked_add(line_count)
                .filter(|end| *end <= lines.len())
                .ok_or(StagingFootnotePaginationError::StateMismatch)?;
            let measured = lines[line_cursor..line_end]
                .iter()
                .try_fold(Length::ZERO, |total, (_, _, line_extent, _, _)| {
                    total.checked_add(line_extent.get())
                })
                .and_then(PositiveLength::new)
                .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
            if measured != extent {
                return Err(StagingFootnotePaginationError::StateMismatch);
            }
            fragment_line_ranges.push((line_cursor, line_end));
            line_cursor = line_end;
        }
        if line_cursor != lines.len() {
            return Err(StagingFootnotePaginationError::StateMismatch);
        }
        for fragment in selected_flow.fragments() {
            let (line_start, line_end) = *fragment_line_ranges
                .get(fragment.fragment_ordinal() as usize)
                .ok_or(StagingFootnotePaginationError::StateMismatch)?;
            let fragment_start_y = y;
            for (owner, first_line, line_extent, space_before, physical_left_inset) in
                &lines[line_start..line_end]
            {
                if *first_line {
                    let anchor_y = y
                        .checked_add(space_before.get())
                        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
                    let relative_y = anchor_y
                        .checked_sub(actual_start)
                        .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
                    let nodes = package.document_nodes();
                    let block_path = nodes
                        .node_path(*owner)
                        .ok_or(StagingFootnotePaginationError::StateMismatch)?;
                    anchors.extend(
                        nodes
                            .anchors()
                            .filter(|(_, anchor_owner)| {
                                nodes
                                    .node_path(*anchor_owner)
                                    .is_some_and(|path| path.starts_with(block_path))
                            })
                            .map(|(anchor_id, anchor_owner)| DiscoveredAnchor {
                                anchor_id: anchor_id.clone(),
                                owner_node: anchor_owner,
                                position_in_frame: Point {
                                    x: physical_left_inset.get(),
                                    y: relative_y,
                                },
                            }),
                    );
                }
                y = y
                    .checked_add(line_extent.get())
                    .ok_or(StagingFootnotePaginationError::ArithmeticOverflow)?;
            }
            if y.checked_sub(fragment_start_y) != Some(fragment.block_extent().get()) {
                return Err(StagingFootnotePaginationError::StateMismatch);
            }
        }
    }
    if y != body_end {
        return Err(StagingFootnotePaginationError::StateMismatch);
    }
    anchors.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));
    if anchors
        .windows(2)
        .any(|pair| pair[0].anchor_id == pair[1].anchor_id)
    {
        return Err(StagingFootnotePaginationError::StateMismatch);
    }
    Ok(anchors)
}

/// Deterministic pagination owner for the reference layout domain: blank
/// documents and top-level empty paragraphs containing only direct anchors.
///
/// The paginator issues its own session and work budget, materializes every
/// pass through [`PassMaterializationPermit`], and seals the result through
/// [`PaginationOutcome`]. Callers provide only validated package/flow/limit
/// inputs and cannot supply pass receipts, placed anchors, or fingerprints.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReferencePaginator;

impl ReferencePaginator {
    pub const fn new() -> Self {
        Self
    }

    /// Runs pagination and convergence for the complete reference layout
    /// domain. The returned outcome owns a validated [`PaginationResult`] and
    /// any required fallback diagnostic.
    pub fn paginate(
        &self,
        package: &ValidatedParsedPackage,
        flow: &FlowTree,
        limits: &ValidatedResourceLimits,
        strict: bool,
    ) -> Result<PaginationOutcome, PaginationError> {
        self.paginate_with_reflow(package, flow, limits, strict, |_, epoch| {
            if epoch == flow.epoch() {
                Ok(flow.clone())
            } else {
                Err(PaginationError::UnsupportedReferenceTransition)
            }
        })
    }

    /// Runs the same sealed pagination state machine while rebuilding each
    /// post-transition FlowTree from the exact owned generated overlay and
    /// working epoch issued by the preceding pass.
    pub fn paginate_with_reflow(
        &self,
        package: &ValidatedParsedPackage,
        initial_flow: &FlowTree,
        limits: &ValidatedResourceLimits,
        strict: bool,
        mut reflow: impl FnMut(&GeneratedTextStore, LayoutEpoch) -> Result<FlowTree, PaginationError>,
    ) -> Result<PaginationOutcome, PaginationError> {
        let initial_state = InitialPaginationState::new(initial_flow, package, limits)?;
        let package_context = package.pagination_context();
        let options = PaginationOptions::from_limits(limits, strict);
        let mut input = PaginationInput::new(initial_state, &package_context, options)?;
        let mut budget = input.take_work_budget()?;
        let mut passes: Vec<LayoutPass> = Vec::new();

        let status = loop {
            let pass_index =
                u16::try_from(passes.len()).map_err(|_| PaginationError::PassIndexOverflow)?;
            let pass_input = if let Some(previous) = passes.last() {
                LayoutPassInput::transitioned(previous.transition_references(package, limits)?)
            } else {
                LayoutPassInput::initial(&input)
            };
            let pass_flow = if pass_index == 0 {
                initial_flow.clone()
            } else {
                reflow(pass_input.generated_text(), pass_input.layout_epoch())?
            };
            if pass_flow.epoch() != pass_input.layout_epoch() {
                return Err(PaginationError::InvalidPreparedLayout);
            }
            let fragmenter = if package.package().document.footnotes.is_empty()
                && package.package().document.blocks.iter().all(|block| {
                    matches!(
                        block,
                        typaxis_document::Block::Paragraph { .. }
                            | typaxis_document::Block::Heading { .. }
                    )
                }) {
                ReferenceFragmenter::for_paragraphs(package, &pass_flow)
            } else {
                ReferenceFragmenter::for_basic_document(package, &pass_flow)
            }
            .map_err(reference_fragment_error)?;
            let input_fingerprint = pass_input.fingerprint();
            let generated_text = pass_input.generated_text().clone();
            let mut permit = budget.begin_pass(pass_index, pass_input)?;
            let pages = Self::materialize_pass(
                package,
                &pass_flow,
                &fragmenter,
                &package_context,
                &mut permit,
            )?;
            let materialization = permit.finish(&pass_flow, &pages)?;
            passes.push(LayoutPass::new(
                materialization,
                input_fingerprint,
                &pass_flow,
                pages,
                generated_text,
            )?);

            if let Some(termination) = observed_termination(&passes) {
                break match termination {
                    ObservedTermination::Stable => ConvergenceStatus::Converged,
                    ObservedTermination::Cycle(cycle_start_state) => {
                        ConvergenceStatus::CycleFallback { cycle_start_state }
                    }
                };
            }
            if passes.len() == usize::from(options.max_layout_passes) {
                break ConvergenceStatus::MaxPassFallback;
            }
        };

        PaginationOutcome::new(passes, status, &input, budget.finish())
    }

    /// Runs global reference convergence and ADR-0030 page-local footnote
    /// convergence as one materialized state machine. Each pass receives the
    /// registry derived from that pass's exact generated-text epoch.
    pub fn paginate_footnote_with_reflow(
        &self,
        package: &ValidatedParsedPackage,
        initial_flow: &FlowTree,
        initial_registry: StagingFootnoteFlowRegistry,
        limits: &ValidatedResourceLimits,
        strict: bool,
        mut reflow: impl FnMut(
            &GeneratedTextStore,
            LayoutEpoch,
        )
            -> Result<(FlowTree, StagingFootnoteFlowRegistry), PaginationError>,
    ) -> Result<FootnotePaginationOutcome, PaginationError> {
        if initial_registry.receipt().epoch() != initial_flow.epoch() {
            return Err(PaginationError::InvalidPreparedLayout);
        }
        let initial_state = InitialPaginationState::new(initial_flow, package, limits)?;
        let package_context = package.pagination_context();
        let options = PaginationOptions::from_limits(limits, strict);
        let mut input = PaginationInput::new(initial_state, &package_context, options)?;
        let mut budget = input.take_work_budget()?;
        let mut passes: Vec<LayoutPass> = Vec::new();
        let mut footnote_passes: Vec<
            Option<(StagingFootnoteFlowRegistry, ValidatedFootnoteSelectedLayout)>,
        > = Vec::new();
        let mut initial_registry = Some(initial_registry);

        let status = loop {
            let pass_index =
                u16::try_from(passes.len()).map_err(|_| PaginationError::PassIndexOverflow)?;
            let pass_input = if let Some(previous) = passes.last() {
                LayoutPassInput::transitioned(previous.transition_references(package, limits)?)
            } else {
                LayoutPassInput::initial(&input)
            };
            let (pass_flow, registry) = if pass_index == 0 {
                (
                    initial_flow.clone(),
                    initial_registry
                        .take()
                        .ok_or(PaginationError::InvalidPreparedLayout)?,
                )
            } else {
                reflow(pass_input.generated_text(), pass_input.layout_epoch())?
            };
            if pass_flow.epoch() != pass_input.layout_epoch()
                || registry.receipt().epoch() != pass_flow.epoch()
                || registry.receipt().package_fingerprint() != package.epoch_identity().document()
            {
                return Err(PaginationError::InvalidPreparedLayout);
            }
            let fragmenter = ReferenceFragmenter::for_footnote_body(package, &pass_flow)
                .map_err(reference_fragment_error)?;
            let input_fingerprint = pass_input.fingerprint();
            let generated_text = pass_input.generated_text().clone();
            let mut permit = budget.begin_pass(pass_index, pass_input)?;
            let (pages, state, selected_pages) = Self::materialize_footnote_pass(
                package,
                &pass_flow,
                &fragmenter,
                &registry,
                input_fingerprint,
                &package_context,
                &mut permit,
                limits,
            )
            .map_err(map_footnote_materialization_error)?;
            let materialization = permit.finish(&pass_flow, &pages)?;
            let pass = LayoutPass::new(
                materialization,
                input_fingerprint,
                &pass_flow,
                pages,
                generated_text,
            )?;
            let selected = state
                .finish(&registry, pass.output_fingerprint(), selected_pages)
                .map_err(map_footnote_materialization_error)?;
            passes.push(pass);
            footnote_passes.push(Some((registry, selected)));

            if let Some(termination) = observed_termination(&passes) {
                break match termination {
                    ObservedTermination::Stable => ConvergenceStatus::Converged,
                    ObservedTermination::Cycle(cycle_start_state) => {
                        ConvergenceStatus::CycleFallback { cycle_start_state }
                    }
                };
            }
            if passes.len() == usize::from(options.max_layout_passes) {
                break ConvergenceStatus::MaxPassFallback;
            }
        };

        let outcome = PaginationOutcome::new(passes, status, &input, budget.finish())?;
        let selected_index = usize::from(outcome.result().selected_state().pass_index());
        let (registry, selected) = footnote_passes
            .get_mut(selected_index)
            .and_then(Option::take)
            .ok_or(PaginationError::InvalidPreparedLayout)?;
        if selected.body_layout_fingerprint() != outcome.result().final_fingerprint()
            || selected.pages().len() != outcome.result().selected_pages().len()
        {
            return Err(PaginationError::InvalidPreparedLayout);
        }
        Ok(FootnotePaginationOutcome {
            outcome,
            registry,
            selected,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn materialize_footnote_pass(
        package: &ValidatedParsedPackage,
        flow: &FlowTree,
        fragmenter: &ReferenceFragmenter<'_>,
        registry: &StagingFootnoteFlowRegistry,
        pass_seed: LayoutStateFingerprint,
        package_context: &PackagePaginationContext,
        permit: &mut PassMaterializationPermit<'_>,
        limits: &ValidatedResourceLimits,
    ) -> Result<
        (
            Vec<PagePlan>,
            StagingFootnotePaginationState,
            Vec<StagingFootnoteSelectedPageReceipt>,
        ),
        StagingFootnotePaginationError,
    > {
        let mut state = StagingFootnotePaginationState::new(registry, permit.pass_index, limits);
        let mut cursor = FlowCursor::document_start(flow);
        let mut pages = Vec::new();
        let mut selected_pages = Vec::new();

        loop {
            let page_index = state.next_page_index();
            let page_start = cursor.clone();
            let selection = if page_start.is_end() {
                ResolvedPageSelection::for_footnote_terminal_carry(flow, &page_start, package)
            } else {
                ResolvedPageSelection::new(flow, &page_start, package)
            }
            .map_err(|_| StagingFootnotePaginationError::InvalidPageInput)?;
            let page = PageContext::select(page_index, &selection, package_context)
                .map_err(|_| StagingFootnotePaginationError::InvalidPageInput)?;
            if page.master_id() != registry.master_id() {
                return Err(StagingFootnotePaginationError::RegistryMismatch);
            }
            let body_frame = PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds: registry.body_frame(),
            };
            let mut frames = vec![body_frame.clone()];
            permit
                .begin_page(&page, &page_start, &frames)
                .map_err(map_pagination_page_to_footnote_error)?;
            let body_page_start =
                footnote_body_start_fingerprint(pass_seed, page_index, page_start.position());
            let page_input = StagingFootnotePageInput::new(page_index, body_page_start);
            let mut final_body = None;
            let allow_held_body = !state.carries().is_empty();
            let convergence =
                evaluate_staging_footnote_page(registry, &state, page_input, |request| {
                    if request.evaluation_index() != 0 {
                        permit
                            .consume_footnote_reflow(page_index)
                            .map_err(|_| StagingFootnotePaginationError::ReflowLimit)?;
                    }
                    let evaluated = evaluate_footnote_body_page(
                        flow,
                        fragmenter,
                        &page,
                        &page_start,
                        body_frame.bounds,
                        request,
                        pass_seed,
                        permit.remaining_fragments,
                        allow_held_body,
                    )?;
                    let body_fragment_count = u64::try_from(evaluated.fragments.len())
                        .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?;
                    let candidate = StagingFootnoteBodyCandidate::new_with_body_fragments(
                        evaluated.fingerprint,
                        if evaluated.terminal {
                            StagingFootnoteBodyContinuation::exhausted(
                                u32::try_from(
                                    evaluated.continuation.position().global_flow_ordinal(),
                                )
                                .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?,
                            )
                        } else {
                            StagingFootnoteBodyContinuation::more(
                                u32::try_from(
                                    evaluated.continuation.position().global_flow_ordinal(),
                                )
                                .map_err(|_| StagingFootnotePaginationError::ArithmeticOverflow)?,
                            )
                        },
                        request.applied_reservation(),
                        request.body_cut_before_reference_owner(),
                        request.available_body_block_size(),
                        body_fragment_count,
                        evaluated.reference_owners.clone(),
                    );
                    final_body = Some(evaluated);
                    Ok(candidate)
                })?;
            let body = final_body.ok_or(StagingFootnotePaginationError::BodyEvaluationFailed)?;
            let selected = state.commit_page(registry, &convergence)?;
            if body.fingerprint != selected.body_fingerprint()
                || body.terminal != selected.body_continuation().is_terminal()
                || body.continuation.position().global_flow_ordinal()
                    != u64::from(selected.body_continuation().next_flow_position())
            {
                return Err(StagingFootnotePaginationError::StateMismatch);
            }
            let mut footnote_ids: Vec<_> = selected
                .flows()
                .iter()
                .map(|flow| flow.assignment().footnote_id().clone())
                .collect();
            footnote_ids.sort();
            if footnote_ids.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(StagingFootnotePaginationError::StateMismatch);
            }
            let placed = permit
                .record_footnote_body_candidate(flow, &body, body_frame.bounds, &footnote_ids)
                .map_err(map_pagination_to_footnote_error)?;
            let footnote_frame =
                selected_footnote_frame(registry.maximum_footnote_frame(), selected.reservation())?
                    .map(|bounds| PageFramePlan {
                        kind: PageFrameKind::Footnote,
                        column_index: 0,
                        bounds,
                    });
            if let Some(frame) = footnote_frame.as_ref() {
                permit
                    .record_footnote_frame(registry.maximum_footnote_frame(), frame)
                    .map_err(map_pagination_to_footnote_error)?;
                frames.push(frame.clone());
            }
            let definition_anchors =
                selected_footnote_definition_anchors(package, flow, registry, &selected)?;
            match (footnote_frame.as_ref(), definition_anchors.is_empty()) {
                (Some(frame), _) => permit
                    .record_footnote_definition_anchors(definition_anchors, frame.bounds)
                    .map_err(map_pagination_to_footnote_error)?,
                (None, true) => {}
                (None, false) => return Err(StagingFootnotePaginationError::StateMismatch),
            }
            let plan = PagePlan {
                page_index,
                master_id: page.master_id().clone(),
                frames,
                fragments: placed,
                footnote_ids,
                float_decisions: Vec::new(),
                column_decisions: Vec::new(),
                resolved_references: Vec::new(),
            };
            let has_more_pages = !body.terminal || !state.carries().is_empty();
            if has_more_pages
                && body.continuation.position() == page_start.position()
                && selected.flows().is_empty()
            {
                let footnote_id = selected
                    .body_cut_before_reference_owner()
                    .and_then(|owner| registry.reference(owner))
                    .map(|reference| reference.footnote_id().clone())
                    .or_else(|| {
                        selected
                            .discovery()
                            .first()
                            .map(|occurrence| occurrence.footnote_id().clone())
                    })
                    .ok_or(StagingFootnotePaginationError::BodyEvaluationFailed)?;
                return Err(StagingFootnotePaginationError::DefinitionOversize(
                    footnote_id,
                ));
            }
            permit
                .finish_footnote_page(&plan, &selected, has_more_pages)
                .map_err(map_pagination_to_footnote_error)?;
            cursor = body.continuation;
            pages.push(plan);
            selected_pages.push(selected);
            if !has_more_pages {
                break;
            }
        }
        Ok((pages, state, selected_pages))
    }

    fn materialize_pass(
        package: &ValidatedParsedPackage,
        flow: &FlowTree,
        fragmenter: &ReferenceFragmenter<'_>,
        package_context: &PackagePaginationContext,
        permit: &mut PassMaterializationPermit<'_>,
    ) -> Result<Vec<PagePlan>, PaginationError> {
        let mut cursor = FlowCursor::document_start(flow);
        let terminal_position = flow
            .positions()
            .last()
            .ok_or(PaginationError::FatalLayout)?;
        let mut pages = Vec::new();
        let mut page_index = 0u32;
        loop {
            let page_start = cursor.clone();
            let selection = ResolvedPageSelection::new(flow, &page_start, package)
                .map_err(|_| PaginationError::FatalLayout)?;
            let page = PageContext::select(page_index, &selection, package_context)
                .map_err(|_| PaginationError::PageLimit)?;
            let body_frame = PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds: page.selected_master().body,
            };
            let frames = vec![body_frame.clone()];
            permit.begin_page(&page, &page_start, &frames)?;
            let mut plan = PagePlan {
                page_index: page.page_index(),
                master_id: page.master_id().clone(),
                frames,
                fragments: Vec::new(),
                footnote_ids: Vec::new(),
                float_decisions: Vec::new(),
                column_decisions: Vec::new(),
                resolved_references: Vec::new(),
            };
            let mut exhausted = flow.positions().len() == 1;
            // A nonblank flow first consumes its zero-output DocumentStart
            // bootstrap. The next fragment call fills the current page; a
            // `More` continuation starts the next physical page.
            let mut bootstrap = page_index == 0
                && flow.positions().first() == Some(cursor.position())
                && flow.positions().len() > 1;
            loop {
                if exhausted {
                    break;
                }
                let request = FragmentRequest::new(
                    flow,
                    &cursor,
                    body_frame.bounds,
                    NonNegativeLength::ZERO,
                    page.clone(),
                )
                .map_err(reference_fragment_error)?;
                let receipt = permit
                    .run_fragmenter(fragmenter, &request, PageFrameKind::Body, 0)
                    .map_err(reference_fragment_error)?;
                plan.fragments.extend_from_slice(receipt.placed_fragments());
                plan.footnote_ids
                    .extend_from_slice(receipt.discovered_footnotes());
                match receipt.continuation().clone() {
                    Continuation::More(next)
                        if bootstrap
                            && fragmenter.ends_with_forced_break()
                            && flow.positions().len() == 3 =>
                    {
                        cursor = *next;
                        break;
                    }
                    Continuation::More(next) if bootstrap => {
                        cursor = *next;
                        bootstrap = false;
                    }
                    Continuation::More(next) => {
                        cursor = *next;
                        break;
                    }
                    Continuation::Exhausted(terminal) => {
                        if !terminal.is_end() || terminal.position() != terminal_position {
                            return Err(PaginationError::FatalLayout);
                        }
                        cursor = *terminal;
                        exhausted = true;
                        break;
                    }
                }
            }
            plan.footnote_ids.sort();
            if plan.footnote_ids.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(PaginationError::FatalLayout);
            }
            permit.finish_page(&plan)?;
            pages.push(plan);
            if exhausted {
                break;
            }
            page_index = page_index
                .checked_add(1)
                .ok_or(PaginationError::PageLimit)?;
        }
        Ok(pages)
    }
}

fn reference_fragment_error(error: FragmentError) -> PaginationError {
    match error {
        FragmentError::ResourceLimit => PaginationError::ResourceLimit,
        FragmentError::ArithmeticOverflow => PaginationError::ArithmeticOverflow,
        _ => PaginationError::FatalLayout,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
    use typaxis_core::{
        sha256, DocumentPackageContractId, HostPath, Length, NodeId, NonNegativeLength,
        PortablePath, PositiveLength, ResourceLimits, SourceId, ValidatedResourceLimits,
    };
    use typaxis_document::{DocumentNodeKind, ValidatedDocumentNodeIndex};
    use typaxis_layout::{
        layout_staging_forced_page_breaks, layout_staging_machine_lists, layout_table_grid,
        layout_table_row_bands, preflight_staging_footnote_profile, CanonicalFlowIrBuilder,
        FragmentDraft, FragmentResult, LayoutEpoch, ProductionFlowIrBuilder,
        StagingFootnoteFlowRegistryBuilder, StagingForcedPageBreakLayoutReceipt,
        StagingListItemPaintInput, StagingMachineListLayoutInput, StagingMachineListLayoutReceipt,
        TableCellLayoutInput,
    };
    use typaxis_linebreak::ValidatedParagraphItemRegistry;
    use typaxis_resource_admission::AdmittedResourceResolver;
    use typaxis_syntax::{
        machine_profile_boundary::{wire, HostMachineInputSession, MachineInputHostOptions},
        DocumentPackageParser, MachineParseOutcome, PackagePaginationContext,
        PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
        StagingStylePackageParser, ValidatedMachinePackage, ValidatedParsedPackage,
    };
    use typaxis_text::TextStore;

    fn validated_package_with_uri(uri: &str) -> ValidatedParsedPackage {
        parsed_reference_package(uri, "")
    }
    fn parsed_reference_package(uri: &str, text: &str) -> ValidatedParsedPackage {
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new(uri).unwrap(),
            text: text.to_owned(),
        };
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        match ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        ) {
            ParseOutcome::Parsed { package, .. } => *package,
            ParseOutcome::Failed { failure } => panic!("reference parse failed: {failure:?}"),
        }
    }

    fn machine_list_length(raw: i64) -> PositiveLength {
        PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap()
    }

    fn footnote_reflow_registry(limits: &ValidatedResourceLimits) -> StagingFootnoteFlowRegistry {
        footnote_reflow_registry_with_height(limits, 100_000)
    }

    fn footnote_reflow_registry_with_height(
        limits: &ValidatedResourceLimits,
        footnote_height: i64,
    ) -> StagingFootnoteFlowRegistry {
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let definition = |footnote_id: &str, node_id: u32| wire::WireFootnote {
            footnote_id: footnote_id.to_owned(),
            node_id,
            span,
            blocks: vec![wire::WireBlock::Paragraph {
                node_id: node_id + 1,
                span,
                classes: Vec::new(),
                children: vec![wire::WireInline::Reference {
                    node_id: node_id + 2,
                    span,
                    target: "target".to_owned(),
                    format: wire::WireReferenceFormat::Page,
                }],
            }],
        };
        let package = wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_2,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "footnote-reflow.tsf".to_owned(),
                utf8_byte_length: 0,
                sha256: sha256(&[]),
            }],
            text_buffers: Vec::new(),
            document: wire::WireDocument {
                node_id: 0,
                blocks: vec![wire::WireBlock::Heading {
                    node_id: 1,
                    span,
                    classes: Vec::new(),
                    level: 1,
                    anchor_id: Some("target".to_owned()),
                    children: vec![
                        wire::WireInline::FootnoteReference {
                            node_id: 2,
                            span,
                            footnote_id: "z".to_owned(),
                        },
                        wire::WireInline::FootnoteReference {
                            node_id: 3,
                            span,
                            footnote_id: "z".to_owned(),
                        },
                        wire::WireInline::FootnoteReference {
                            node_id: 4,
                            span,
                            footnote_id: "a".to_owned(),
                        },
                    ],
                }],
                footnotes: vec![definition("a", 5), definition("z", 8)],
            },
            style_sheet: wire::WireStyleSheet { rules: Vec::new() },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![wire::WirePageMaster {
                    master_id: "default".to_owned(),
                    width: 200_000,
                    height: 200_000,
                    body: wire::WireRect {
                        x: 0,
                        y: 0,
                        width: 200_000,
                        height: 200_000,
                    },
                    header: None,
                    footer: None,
                    footnote: Some(wire::WireRect {
                        x: 0,
                        y: 200_000 - footnote_height,
                        width: 200_000,
                        height: footnote_height,
                    }),
                }],
                selection_rules: Vec::new(),
            },
            resources: wire::WireResourceCatalog {
                font_faces: Vec::new(),
                images: Vec::new(),
            },
        };
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&package)
            .unwrap();
        let decoded = wire::StagingStyleDocumentPackageDecoder::new()
            .decode(&bytes, &wire::DocumentPackageDecodePolicy::new(limits))
            .unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let package = StagingStylePackageParser::new()
            .parse(
                decoded,
                String::new(),
                &PackageValidationPolicy::new(limits, &schemes).unwrap(),
            )
            .unwrap();
        let generated = package
            .package()
            .materialize_initial_generated_text(limits)
            .unwrap();
        let generated = package
            .package()
            .bind_generated_text(&generated, limits)
            .unwrap();
        let admitted =
            AdmittedResourceResolver::new(&package.package().package().resources, limits)
                .unwrap()
                .finish()
                .unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let body_registry = typaxis_layout::flow_registry_fingerprint_from_jcs(
            "{\"algorithm\":\"typaxis.basic-flow-registry/1\",\"fixture\":true}",
        );
        let preflight =
            preflight_staging_footnote_profile(package.package(), epoch, body_registry, limits)
                .unwrap();
        let mut builder = StagingFootnoteFlowRegistryBuilder::new(&preflight, limits);
        for id in builder
            .expected_definition_ids()
            .cloned()
            .collect::<Vec<_>>()
        {
            let fragments = if id.as_str() == "a" {
                vec![machine_list_length(10_000)]
            } else {
                vec![machine_list_length(15_000), machine_list_length(15_000)]
            };
            let measured = builder.issue_definition(&id, fragments).unwrap();
            builder.register(measured).unwrap();
        }
        builder.finish().unwrap()
    }

    fn footnote_page_input(page_index: u32) -> StagingFootnotePageInput {
        StagingFootnotePageInput::new(
            page_index,
            LayoutStateFingerprint::from_untrusted_bytes([99; 32]),
        )
    }

    fn footnote_body_candidate(
        request: &StagingFootnotePageEvaluationRequest,
        fingerprint_byte: u8,
        reference_owners: Vec<NodeId>,
    ) -> StagingFootnoteBodyCandidate {
        StagingFootnoteBodyCandidate::new_with_body_cut(
            LayoutStateFingerprint::from_untrusted_bytes([fingerprint_byte; 32]),
            StagingFootnoteBodyContinuation::more(7),
            request.applied_reservation(),
            request.body_cut_before_reference_owner(),
            request.available_body_block_size(),
            reference_owners,
        )
    }

    fn footnote_reflow_converged_page(
        registry: &StagingFootnoteFlowRegistry,
        limits: &ValidatedResourceLimits,
        reference_owners: Vec<NodeId>,
    ) -> (
        StagingFootnotePaginationState,
        StagingFootnoteSelectedPageReceipt,
    ) {
        let mut state = StagingFootnotePaginationState::new(registry, 0, limits);
        let receipt =
            evaluate_staging_footnote_page(registry, &state, footnote_page_input(0), |request| {
                Ok(footnote_body_candidate(
                    request,
                    1,
                    reference_owners.clone(),
                ))
            })
            .unwrap();
        let selected = state.commit_page(registry, &receipt).unwrap();
        (state, selected)
    }

    #[test]
    fn footnote_reflow_zero_one_multiple_and_repeat_use_first_reference_order() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let registry = footnote_reflow_registry(&limits);

        let (zero_state, zero) = footnote_reflow_converged_page(&registry, &limits, Vec::new());
        assert_eq!(zero.evaluation_count(), 2);
        assert_eq!(zero.reservation(), NonNegativeLength::ZERO);
        assert!(zero.ordered_footnotes().is_empty());
        assert_eq!(zero_state.selected_record_count(), 0);

        let (_, one) = footnote_reflow_converged_page(&registry, &limits, vec![NodeId::new(2)]);
        assert_eq!(one.discovery().len(), 1);
        assert_eq!(one.ordered_footnotes().len(), 1);
        assert_eq!(one.ordered_footnotes()[0].footnote_id().as_str(), "z");
        assert_eq!(one.reservation().get().raw(), 95_536);

        let (_, repeated) = footnote_reflow_converged_page(
            &registry,
            &limits,
            vec![NodeId::new(2), NodeId::new(3)],
        );
        assert_eq!(repeated.discovery().len(), 2);
        assert_eq!(repeated.ordered_footnotes().len(), 1);
        assert_eq!(repeated.reservation().get().raw(), 95_536);

        let (multiple_state, multiple) = footnote_reflow_converged_page(
            &registry,
            &limits,
            vec![NodeId::new(2), NodeId::new(3), NodeId::new(4)],
        );
        assert_eq!(multiple.discovery().len(), 3);
        assert_eq!(
            multiple
                .ordered_footnotes()
                .iter()
                .map(|assignment| (
                    assignment.footnote_id().as_str(),
                    assignment.flow_id().get(),
                    assignment.assignment_ordinal(),
                    assignment.first_reference_owner().get(),
                ))
                .collect::<Vec<_>>(),
            vec![("z", 1, 0, 2), ("a", 0, 1, 4)]
        );
        assert_eq!(multiple.reservation().get().raw(), 90_536);
        assert_eq!(multiple.flows()[0].fragments().len(), 1);
        assert!(multiple.flows()[0].carries_out());
        assert_eq!(multiple.flows()[1].fragments().len(), 1);
        assert!(!multiple.flows()[1].carries_out());
        assert_eq!(multiple_state.selected_record_count(), 5);
        assert_eq!(multiple_state.carries().len(), 1);
    }

    #[test]
    fn footnote_reflow_property_first_reference_projection_is_stable() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let registry = footnote_reflow_registry(&limits);
        let cases: &[(&[u32], &[&str])] = &[
            (&[], &[]),
            (&[2], &["z"]),
            (&[3], &["z"]),
            (&[4], &["a"]),
            (&[2, 3], &["z"]),
            (&[2, 4], &["z", "a"]),
            (&[3, 4], &["z", "a"]),
            (&[2, 3, 4], &["z", "a"]),
        ];
        for (owners, expected) in cases {
            let (_, selected) = footnote_reflow_converged_page(
                &registry,
                &limits,
                owners.iter().copied().map(NodeId::new).collect(),
            );
            assert_eq!(
                selected
                    .ordered_footnotes()
                    .iter()
                    .map(|assignment| assignment.footnote_id().as_str())
                    .collect::<Vec<_>>(),
                *expected
            );
        }
    }

    #[test]
    fn footnote_reflow_later_repeat_keeps_discovery_without_reassignment() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let registry = footnote_reflow_registry(&limits);
        let (mut state, first) =
            footnote_reflow_converged_page(&registry, &limits, vec![NodeId::new(2)]);
        assert_eq!(first.ordered_footnotes().len(), 1);
        assert!(state.carries().is_empty());

        let repeated =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(1), |request| {
                Ok(footnote_body_candidate(request, 2, vec![NodeId::new(3)]))
            })
            .unwrap();
        let repeated = state.commit_page(&registry, &repeated).unwrap();
        assert_eq!(repeated.discovery().len(), 1);
        assert!(repeated.ordered_footnotes().is_empty());
        assert_eq!(repeated.reservation(), NonNegativeLength::ZERO);
        assert_eq!(state.assignments().len(), 1);

        assert_eq!(
            evaluate_staging_footnote_page(
                &registry,
                &state,
                footnote_page_input(2),
                |request| Ok(footnote_body_candidate(request, 3, vec![NodeId::new(3)])),
            )
            .unwrap_err(),
            StagingFootnotePaginationError::DuplicateReferenceOccurrence(NodeId::new(3))
        );
        assert_eq!(state.next_page_index(), 2);
    }

    #[test]
    fn footnote_reflow_moves_a_trailing_minimum_only_at_the_body_cut_boundary() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let registry = footnote_reflow_registry_with_height(&limits, 85_000);
        let mut state = StagingFootnotePaginationState::new(&registry, 0, &limits);
        let mut requested_cuts = Vec::new();
        let receipt =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(0), |request| {
                requested_cuts.push(request.body_cut_before_reference_owner());
                let (fingerprint, references) =
                    if request.body_cut_before_reference_owner() == Some(NodeId::new(4)) {
                        (2, vec![NodeId::new(2)])
                    } else {
                        (1, vec![NodeId::new(2), NodeId::new(4)])
                    };
                Ok(footnote_body_candidate(request, fingerprint, references))
            })
            .unwrap();
        assert_eq!(
            requested_cuts,
            vec![None, Some(NodeId::new(4)), Some(NodeId::new(4))]
        );
        assert_eq!(receipt.evaluation_count(), 3);
        assert_eq!(
            receipt.final_evaluation().body_cut_before_reference_owner(),
            Some(NodeId::new(4))
        );
        assert_eq!(receipt.final_evaluation().ordered_footnotes().len(), 1);
        assert_eq!(
            receipt.final_evaluation().ordered_footnotes()[0]
                .footnote_id()
                .as_str(),
            "z"
        );
        assert_eq!(receipt.final_evaluation().reservation().get().raw(), 80_536);
        let selected = state.commit_page(&registry, &receipt).unwrap();
        assert_eq!(
            selected.body_cut_before_reference_owner(),
            Some(NodeId::new(4))
        );
        assert_eq!(state.assignments().len(), 1);

        let unsplittable_state = StagingFootnotePaginationState::new(&registry, 0, &limits);
        let mut calls = 0;
        let error = evaluate_staging_footnote_page(
            &registry,
            &unsplittable_state,
            footnote_page_input(0),
            |request| {
                calls += 1;
                Ok(footnote_body_candidate(
                    request,
                    1,
                    vec![NodeId::new(2), NodeId::new(4)],
                ))
            },
        )
        .unwrap_err();
        assert_eq!(
            error,
            StagingFootnotePaginationError::DefinitionOversize(FootnoteId::new("a").unwrap())
        );
        assert_eq!(error.diagnostic_code(), L5100);
        assert_eq!(calls, 2);
        assert_eq!(unsplittable_state.next_page_index(), 0);
        assert!(unsplittable_state.selected_page_fingerprints().is_empty());
    }

    #[test]
    fn footnote_reflow_footnote_carry_seeds_evaluation_zero_and_advances_separately() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let registry = footnote_reflow_registry(&limits);
        let (mut state, _) = footnote_reflow_converged_page(
            &registry,
            &limits,
            vec![NodeId::new(2), NodeId::new(4)],
        );
        assert_eq!(state.carries()[0].next_cursor().next_fragment_ordinal(), 1);
        let mut applied = Vec::new();
        let receipt =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(1), |request| {
                applied.push(request.applied_reservation().get().raw());
                Ok(footnote_body_candidate(request, 2, Vec::new()))
            })
            .unwrap();
        assert_eq!(applied, vec![80_536, 80_536]);
        let selected = state.commit_page(&registry, &receipt).unwrap();
        assert_eq!(selected.ordered_footnotes().len(), 1);
        assert_eq!(selected.ordered_footnotes()[0].footnote_id().as_str(), "z");
        assert_eq!(
            selected.flows()[0].before_cursor().next_fragment_ordinal(),
            1
        );
        assert_eq!(
            selected.flows()[0].after_cursor().next_fragment_ordinal(),
            2
        );
        assert!(state.carries().is_empty());
        assert_eq!(state.assignments().len(), 2);
    }

    #[test]
    fn footnote_carry_only_page_holds_body_cursor_until_definition_progress_completes() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let registry = footnote_reflow_registry(&limits);
        let mut state = StagingFootnotePaginationState::new(&registry, 0, &limits);
        let first =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(0), |request| {
                Ok(footnote_body_candidate(
                    request,
                    1,
                    vec![NodeId::new(2), NodeId::new(3), NodeId::new(4)],
                ))
            })
            .unwrap();
        let first = state.commit_page(&registry, &first).unwrap();
        assert_eq!(state.carries().len(), 1);

        let carry_only =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(1), |request| {
                Ok(footnote_body_candidate(request, 2, Vec::new()))
            })
            .unwrap();
        let carry_only = state.commit_page(&registry, &carry_only).unwrap();
        assert_eq!(
            carry_only.body_continuation().next_flow_position(),
            first.body_continuation().next_flow_position()
        );
        assert!(!carry_only.body_continuation().is_terminal());
        assert!(state.carries().is_empty());

        let resumed =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(2), |request| {
                let mut candidate = footnote_body_candidate(request, 3, Vec::new());
                candidate.continuation = StagingFootnoteBodyContinuation::exhausted(8);
                Ok(candidate)
            })
            .unwrap();
        let resumed = state.commit_page(&registry, &resumed).unwrap();
        let selected = state
            .finish(
                &registry,
                LayoutStateFingerprint::from_untrusted_bytes([44; 32]),
                vec![first, carry_only, resumed],
            )
            .unwrap();
        assert_eq!(selected.pages().len(), 3);
        assert_eq!(
            selected.pages()[0].body_continuation().next_flow_position(),
            selected.pages()[1].body_continuation().next_flow_position()
        );
        assert!(selected.pages()[2].body_continuation().is_terminal());
    }

    fn complete_footnote_carry_fixture() -> (
        StagingFootnoteFlowRegistry,
        StagingFootnotePaginationState,
        Vec<StagingFootnoteSelectedPageReceipt>,
    ) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let registry = footnote_reflow_registry(&limits);
        let mut state = StagingFootnotePaginationState::new(&registry, 0, &limits);
        let first =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(0), |request| {
                Ok(footnote_body_candidate(
                    request,
                    1,
                    vec![NodeId::new(2), NodeId::new(3), NodeId::new(4)],
                ))
            })
            .unwrap();
        let first = state.commit_page(&registry, &first).unwrap();
        assert_eq!(state.carries().len(), 1);

        let second =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(1), |request| {
                let mut candidate = footnote_body_candidate(request, 2, Vec::new());
                candidate.continuation = StagingFootnoteBodyContinuation::exhausted(8);
                Ok(candidate)
            })
            .unwrap();
        let second = state.commit_page(&registry, &second).unwrap();
        assert!(state.carries().is_empty());
        (registry, state, vec![first, second])
    }

    #[test]
    fn footnote_carry_finish_binds_complete_definition_paint_and_page_edges() {
        let (registry, state, pages) = complete_footnote_carry_fixture();
        let selected = state
            .finish(
                &registry,
                LayoutStateFingerprint::from_untrusted_bytes([44; 32]),
                pages,
            )
            .unwrap();
        assert_eq!(selected.pages().len(), 2);
        assert_eq!(selected.assignments().len(), 2);
        assert!(selected
            .canonical_jcs()
            .contains("\"incoming_source_page\":0"));
    }

    #[test]
    fn footnote_carry_finish_rejects_missing_duplicate_and_wrong_page_paint() {
        let (registry, state, mut pages) = complete_footnote_carry_fixture();
        pages[1].flows.clear();
        assert_eq!(
            state
                .finish(
                    &registry,
                    LayoutStateFingerprint::from_untrusted_bytes([44; 32]),
                    pages,
                )
                .unwrap_err(),
            StagingFootnotePaginationError::MissingDefinitionPaint(FootnoteId::new("z").unwrap())
        );

        let (registry, state, mut pages) = complete_footnote_carry_fixture();
        pages[1].flows[0].fragments[0].fragment_ordinal = 0;
        assert_eq!(
            state
                .finish(
                    &registry,
                    LayoutStateFingerprint::from_untrusted_bytes([44; 32]),
                    pages,
                )
                .unwrap_err(),
            StagingFootnotePaginationError::DuplicateDefinitionPaint(FootnoteId::new("z").unwrap())
        );

        let (registry, state, mut pages) = complete_footnote_carry_fixture();
        pages[1].flows[0].incoming_source_page = None;
        assert_eq!(
            state
                .finish(
                    &registry,
                    LayoutStateFingerprint::from_untrusted_bytes([44; 32]),
                    pages,
                )
                .unwrap_err(),
            StagingFootnotePaginationError::WrongPageCarry(FootnoteFlowId::new(1))
        );
    }

    #[test]
    fn footnote_carry_finish_rejects_an_empty_page_after_all_progress_is_terminal() {
        let (registry, mut state, mut pages) = complete_footnote_carry_fixture();
        let extra =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(2), |request| {
                let mut candidate = footnote_body_candidate(request, 3, Vec::new());
                candidate.continuation = StagingFootnoteBodyContinuation::exhausted(8);
                Ok(candidate)
            })
            .unwrap();
        pages.push(state.commit_page(&registry, &extra).unwrap());
        assert_eq!(
            state
                .finish(
                    &registry,
                    LayoutStateFingerprint::from_untrusted_bytes([44; 32]),
                    pages,
                )
                .unwrap_err(),
            StagingFootnotePaginationError::IncompleteSelectedLayout
        );
    }

    #[test]
    fn footnote_reflow_rejects_wrong_order_reservation_and_unknown_reference() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let registry = footnote_reflow_registry(&limits);
        let state = StagingFootnotePaginationState::new(&registry, 0, &limits);
        let wrong_order =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(0), |request| {
                Ok(footnote_body_candidate(
                    request,
                    1,
                    vec![NodeId::new(4), NodeId::new(2)],
                ))
            })
            .unwrap_err();
        assert_eq!(
            wrong_order,
            StagingFootnotePaginationError::NonCanonicalReferenceOrder(NodeId::new(2))
        );

        let unknown =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(0), |request| {
                Ok(footnote_body_candidate(request, 1, vec![NodeId::new(999)]))
            })
            .unwrap_err();
        assert_eq!(
            unknown,
            StagingFootnotePaginationError::UnknownReferenceOwner(NodeId::new(999))
        );

        let wrong_reservation =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(0), |request| {
                Ok(StagingFootnoteBodyCandidate::new(
                    LayoutStateFingerprint::from_untrusted_bytes([1; 32]),
                    StagingFootnoteBodyContinuation::more(7),
                    NonNegativeLength::ZERO,
                    request.available_body_block_size(),
                    vec![NodeId::new(2)],
                ))
            })
            .unwrap_err();
        assert_eq!(
            wrong_reservation,
            StagingFootnotePaginationError::InvalidBodyCandidate
        );
        assert_eq!(state.next_page_index(), 0);
        assert!(state.selected_page_fingerprints().is_empty());
    }

    #[test]
    fn footnote_reflow_exact_limit_converges_and_max_plus_one_never_starts() {
        let exact_raw = ResourceLimits {
            max_footnote_reflows_per_page: 2,
            ..ResourceLimits::default()
        };
        let exact_limits = ValidatedResourceLimits::new(exact_raw).unwrap();
        let exact_registry = footnote_reflow_registry(&exact_limits);
        let exact_state = StagingFootnotePaginationState::new(&exact_registry, 0, &exact_limits);
        let mut exact_calls = 0u16;
        let exact = evaluate_staging_footnote_page(
            &exact_registry,
            &exact_state,
            footnote_page_input(0),
            |request| {
                exact_calls += 1;
                let tag = if request.evaluation_index() == 0 {
                    1
                } else {
                    2
                };
                Ok(footnote_body_candidate(request, tag, vec![NodeId::new(2)]))
            },
        )
        .unwrap();
        assert_eq!(exact.evaluation_index(), 2);
        assert_eq!(exact_calls, 3);

        let over_raw = ResourceLimits {
            max_footnote_reflows_per_page: 1,
            ..ResourceLimits::default()
        };
        let over_limits = ValidatedResourceLimits::new(over_raw).unwrap();
        let over_registry = footnote_reflow_registry(&over_limits);
        let over_state = StagingFootnotePaginationState::new(&over_registry, 0, &over_limits);
        let mut over_calls = 0u16;
        let over = evaluate_staging_footnote_page(
            &over_registry,
            &over_state,
            footnote_page_input(0),
            |request| {
                over_calls += 1;
                Ok(footnote_body_candidate(
                    request,
                    u8::try_from(request.evaluation_index()).unwrap() + 1,
                    vec![NodeId::new(2)],
                ))
            },
        )
        .unwrap_err();
        assert_eq!(over, StagingFootnotePaginationError::ReflowLimit);
        assert_eq!(over.diagnostic_code(), G6002);
        assert_eq!(over.severity(), Severity::Fatal);
        assert_eq!(over_calls, 2);
        assert_eq!(over_state.next_page_index(), 0);
        assert!(over_state.selected_page_fingerprints().is_empty());
    }

    #[test]
    fn footnote_reflow_detects_oscillation_without_fallback() {
        let raw = ResourceLimits {
            max_footnote_reflows_per_page: 3,
            ..ResourceLimits::default()
        };
        let limits = ValidatedResourceLimits::new(raw).unwrap();
        let registry = footnote_reflow_registry(&limits);
        let state = StagingFootnotePaginationState::new(&registry, 0, &limits);
        let mut calls = 0u16;
        let error =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(0), |request| {
                calls += 1;
                let tag = match request.evaluation_index() {
                    0 | 2 => 1,
                    _ => 2,
                };
                Ok(footnote_body_candidate(request, tag, vec![NodeId::new(2)]))
            })
            .unwrap_err();
        assert_eq!(error, StagingFootnotePaginationError::ReflowOscillation);
        assert_eq!(error.diagnostic_code(), G6002);
        assert_eq!(calls, 3);
        assert!(state.selected_page_fingerprints().is_empty());
    }

    #[test]
    fn footnote_reflow_fragment_boundary_and_receipt_replay_are_closed() {
        let exact_raw = ResourceLimits {
            max_fragments: 5,
            ..ResourceLimits::default()
        };
        let exact_limits = ValidatedResourceLimits::new(exact_raw).unwrap();
        let exact_registry = footnote_reflow_registry(&exact_limits);
        let (exact_state, exact) = footnote_reflow_converged_page(
            &exact_registry,
            &exact_limits,
            vec![NodeId::new(2), NodeId::new(4)],
        );
        assert_eq!(exact_state.selected_record_count(), 5);
        assert_eq!(exact.ordered_footnotes().len(), 2);

        let over_raw = ResourceLimits {
            max_fragments: 4,
            ..ResourceLimits::default()
        };
        let over_limits = ValidatedResourceLimits::new(over_raw).unwrap();
        let over_registry = footnote_reflow_registry(&over_limits);
        let over_state = StagingFootnotePaginationState::new(&over_registry, 0, &over_limits);
        assert_eq!(
            evaluate_staging_footnote_page(
                &over_registry,
                &over_state,
                footnote_page_input(0),
                |request| Ok(footnote_body_candidate(
                    request,
                    1,
                    vec![NodeId::new(2), NodeId::new(4)],
                )),
            )
            .unwrap_err(),
            StagingFootnotePaginationError::FragmentLimit
        );
        assert!(over_state.selected_page_fingerprints().is_empty());

        let registry = footnote_reflow_registry(
            &ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
        );
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let mut state = StagingFootnotePaginationState::new(&registry, 0, &limits);
        let receipt =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(0), |request| {
                Ok(footnote_body_candidate(
                    request,
                    1,
                    vec![NodeId::new(2), NodeId::new(4)],
                ))
            })
            .unwrap();
        state.commit_page(&registry, &receipt).unwrap();
        assert_eq!(
            state.commit_page(&registry, &receipt).unwrap_err(),
            StagingFootnotePaginationError::StateMismatch
        );
    }

    #[test]
    fn footnote_reflow_tampered_order_and_reservation_cannot_commit() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let registry = footnote_reflow_registry(&limits);
        let state = StagingFootnotePaginationState::new(&registry, 0, &limits);
        let mut wrong_order =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(0), |request| {
                Ok(footnote_body_candidate(
                    request,
                    1,
                    vec![NodeId::new(2), NodeId::new(4)],
                ))
            })
            .unwrap();
        wrong_order.final_evaluation.ordered_footnotes.swap(0, 1);
        let mut state_for_order = StagingFootnotePaginationState::new(&registry, 0, &limits);
        assert_eq!(
            state_for_order
                .commit_page(&registry, &wrong_order)
                .unwrap_err(),
            StagingFootnotePaginationError::StateMismatch
        );

        let mut wrong_reservation =
            evaluate_staging_footnote_page(&registry, &state, footnote_page_input(0), |request| {
                Ok(footnote_body_candidate(request, 1, vec![NodeId::new(2)]))
            })
            .unwrap();
        wrong_reservation.final_evaluation.reservation = NonNegativeLength::ZERO;
        let mut state_for_reservation = StagingFootnotePaginationState::new(&registry, 0, &limits);
        assert_eq!(
            state_for_reservation
                .commit_page(&registry, &wrong_reservation)
                .unwrap_err(),
            StagingFootnotePaginationError::StateMismatch
        );
    }

    fn table_fragmentation_fixture(
        zero_height_last_row: bool,
    ) -> (TableRowBandLayoutReceipt, ProductionFlowIr) {
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let cell = |node_id, rowspan| wire::WireTableCell {
            node_id,
            span,
            colspan: 1,
            rowspan,
            blocks: Vec::new(),
        };
        let package = wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_2,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "table-fragmentation.tsf".to_owned(),
                utf8_byte_length: 0,
                sha256: sha256(&[]),
            }],
            text_buffers: Vec::new(),
            document: wire::WireDocument {
                node_id: 0,
                blocks: vec![wire::WireBlock::Table {
                    node_id: 1,
                    span,
                    classes: Vec::new(),
                    columns: vec![
                        wire::WireTableColumn::Fixed { width: 5 },
                        wire::WireTableColumn::Fixed { width: 5 },
                    ],
                    head: vec![wire::WireTableRow {
                        node_id: 2,
                        span,
                        cells: vec![cell(3, 1), cell(4, 1)],
                    }],
                    body: vec![
                        wire::WireTableRow {
                            node_id: 5,
                            span,
                            cells: vec![cell(6, 2), cell(7, 1)],
                        },
                        wire::WireTableRow {
                            node_id: 8,
                            span,
                            cells: vec![cell(9, 1)],
                        },
                        wire::WireTableRow {
                            node_id: 10,
                            span,
                            cells: vec![cell(11, 1), cell(12, 1)],
                        },
                    ],
                }],
                footnotes: Vec::new(),
            },
            style_sheet: wire::WireStyleSheet { rules: Vec::new() },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![wire::WirePageMaster {
                    master_id: "default".to_owned(),
                    width: 10,
                    height: 7,
                    body: wire::WireRect {
                        x: 0,
                        y: 0,
                        width: 10,
                        height: 7,
                    },
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: Vec::new(),
            },
            resources: wire::WireResourceCatalog {
                font_faces: Vec::new(),
                images: Vec::new(),
            },
        };
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&package)
            .unwrap();
        let decoded = wire::StagingStyleDocumentPackageDecoder::new()
            .decode(&bytes, &wire::DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let package = StagingStylePackageParser::new()
            .parse(
                decoded,
                String::new(),
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            )
            .unwrap();
        let generated = package
            .package()
            .materialize_initial_generated_text(&limits)
            .unwrap();
        let package_epoch = epoch_for(package.package(), &generated);
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(package.package(), package_epoch)
                .unwrap();
        let mut builder = ProductionFlowIrBuilder::new(
            package.package(),
            &paragraph_items,
            package_epoch,
            &limits,
        )
        .unwrap();
        let owners: Vec<_> = builder.expected_content_owners().collect();
        for owner in owners {
            let content = builder.issue_content(owner).unwrap();
            builder.register_content(content).unwrap();
        }
        let ir = builder.finish().unwrap();
        let style = package.compute_table_style(NodeId::new(1)).unwrap();
        let grid = layout_table_grid(
            &package,
            NodeId::new(1),
            &style,
            &ir,
            machine_list_length(10),
            &limits,
        )
        .unwrap();
        let inputs = grid
            .cells()
            .iter()
            .map(|binding| {
                let fragments = match binding.cell_owner().get() {
                    3 | 4 => vec![machine_list_length(2)],
                    6 => vec![machine_list_length(4), machine_list_length(5)],
                    7 => vec![machine_list_length(4)],
                    9 => vec![machine_list_length(5), machine_list_length(1)],
                    11 | 12 if !zero_height_last_row => {
                        vec![machine_list_length(4), machine_list_length(4)]
                    }
                    11 | 12 => Vec::new(),
                    _ => unreachable!(),
                };
                TableCellLayoutInput::new(binding.cell_owner(), binding.flow_id(), fragments)
            })
            .collect();
        (layout_table_row_bands(&grid, inputs, &limits).unwrap(), ir)
    }

    fn table_fragmentation_limits(max_fragments: u64) -> ValidatedResourceLimits {
        ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 8,
            max_fragments,
            ..ResourceLimits::default()
        })
        .unwrap()
    }

    #[test]
    fn table_fragmentation_selects_common_cuts_rowspan_and_bound_header_repetitions() {
        let (layout, ir) = table_fragmentation_fixture(false);
        let limits = table_fragmentation_limits(21);
        assert_eq!(layout.contained_fragment_count(), 11);
        let input =
            StagingTablePageInput::new(machine_list_length(7), machine_list_length(7)).unwrap();
        let selected = paginate_staging_table(&layout, &ir, input, &limits).unwrap();
        assert_eq!(selected.page_count(), 5);
        assert_eq!(selected.header_sources().len(), 1);
        assert_eq!(selected.header_repetitions().len(), 5);
        assert_eq!(selected.row_fragments().len(), 5);
        assert_eq!(
            selected
                .row_fragments()
                .iter()
                .map(RowFragmentReceipt::selected_block_extent)
                .collect::<Vec<_>>(),
            vec![4, 5, 4, 4, 4]
        );
        for fragment in selected.row_fragments() {
            assert!(fragment
                .cells()
                .iter()
                .all(|cell| cell.selected_block_extent() == fragment.selected_block_extent()));
        }
        assert_eq!(
            selected.row_fragments()[0]
                .continuation_after()
                .entries()
                .iter()
                .map(|entry| (entry.column_ordinal(), entry.cell_owner().get()))
                .collect::<Vec<_>>(),
            vec![(0, 6)]
        );
        let source_id = selected.header_sources()[0].source_fragment_id();
        assert!(selected.header_repetitions().iter().all(|receipt| receipt
            .rows()
            .iter()
            .all(|row| row.source_fragment_id() == source_id)));
        assert!(selected
            .header_repetitions()
            .iter()
            .enumerate()
            .all(
                |(index, receipt)| receipt.repetition_index() == index as u32
                    && receipt.selected_state_fingerprint() == selected.fingerprint()
            ));
        selected.validate_closure(&layout, &ir).unwrap();
        let trace = selected.trace_facts().unwrap();
        assert_eq!(trace.row_fragment_count(), 5);
        assert_eq!(trace.header_occurrence_count(), 5);
        assert_eq!(trace.cell_fragment_count(), 12);
        assert!(trace
            .canonical_jcs()
            .contains(TableSelectedLayoutFingerprint::ALGORITHM_ID));

        let (repeat_layout, repeat_ir) = table_fragmentation_fixture(false);
        let repeat = paginate_staging_table(&repeat_layout, &repeat_ir, input, &limits).unwrap();
        assert_eq!(selected.fingerprint(), repeat.fingerprint());
        assert_eq!(selected.canonical_jcs(), repeat.canonical_jcs());
    }

    #[test]
    fn table_fragmentation_zero_height_row_advances_structurally() {
        let (layout, ir) = table_fragmentation_fixture(true);
        let limits = table_fragmentation_limits(14);
        let input =
            StagingTablePageInput::new(machine_list_length(7), machine_list_length(7)).unwrap();
        let selected = paginate_staging_table(&layout, &ir, input, &limits).unwrap();
        let zero = selected
            .row_fragments()
            .iter()
            .find(|fragment| fragment.logical_row_ordinal() == 2)
            .unwrap();
        assert_eq!(zero.selected_block_extent(), 0);
        assert_eq!(
            zero.after_cursor().logical_row_ordinal(),
            zero.before_cursor().logical_row_ordinal() + 1
        );
        assert!(zero.after_cursor().is_terminal());
    }

    #[test]
    fn table_fragmentation_exact_limit_and_oversize_terminal_are_fail_closed() {
        let (layout, ir) = table_fragmentation_fixture(false);
        let input =
            StagingTablePageInput::new(machine_list_length(7), machine_list_length(7)).unwrap();
        let fragment_error =
            paginate_staging_table(&layout, &ir, input, &table_fragmentation_limits(20))
                .unwrap_err();
        assert_eq!(fragment_error, StagingTablePaginationError::FragmentLimit);
        assert_eq!(fragment_error.diagnostic_code(), L5110);

        let (layout, ir) = table_fragmentation_fixture(false);
        let oversize_input =
            StagingTablePageInput::new(machine_list_length(4), machine_list_length(4)).unwrap();
        let error = paginate_staging_table(
            &layout,
            &ir,
            oversize_input,
            &table_fragmentation_limits(21),
        )
        .unwrap_err();
        let StagingTablePaginationError::RowOversize(terminal) = error else {
            panic!("expected row oversize, got {error:?}");
        };
        assert_eq!(terminal.row_owner(), NodeId::new(5));
        assert_eq!(terminal.transition_count(), 1);
        assert_eq!(error.diagnostic_code(), L5100);

        let (layout, ir) = table_fragmentation_fixture(false);
        let header_error = paginate_staging_table(
            &layout,
            &ir,
            StagingTablePageInput::new(machine_list_length(1), machine_list_length(1)).unwrap(),
            &table_fragmentation_limits(21),
        )
        .unwrap_err();
        assert_eq!(
            header_error,
            StagingTablePaginationError::HeaderOversize(NodeId::new(1))
        );
        assert_eq!(header_error.diagnostic_code(), L5100);
    }

    #[test]
    fn table_fragmentation_rejects_zero_progress_retry_and_continuation_tamper() {
        let input =
            StagingTablePageInput::new(machine_list_length(7), machine_list_length(7)).unwrap();
        let limits = table_fragmentation_limits(21);

        let (layout, ir) = table_fragmentation_fixture(false);
        let mut selected = paginate_staging_table(&layout, &ir, input, &limits).unwrap();
        selected.row_fragments[0].after_cursor = selected.row_fragments[0].before_cursor;
        assert_eq!(
            selected.validate_closure(&layout, &ir).unwrap_err(),
            StagingTablePaginationError::NoProgress(NodeId::new(5))
        );

        let (layout, ir) = table_fragmentation_fixture(false);
        let mut selected = paginate_staging_table(&layout, &ir, input, &limits).unwrap();
        selected.row_fragments[1]
            .after_cursor
            .block_offset_within_row = 1;
        assert_eq!(
            selected.validate_closure(&layout, &ir).unwrap_err(),
            StagingTablePaginationError::SelectedStateMismatch
        );

        let (layout, ir) = table_fragmentation_fixture(false);
        let mut selected = paginate_staging_table(&layout, &ir, input, &limits).unwrap();
        selected.row_fragments[1]
            .continuation_before
            .entries
            .clear();
        assert_eq!(
            selected.validate_closure(&layout, &ir).unwrap_err(),
            StagingTablePaginationError::MissingContinuation { column_ordinal: 0 }
        );

        let (layout, ir) = table_fragmentation_fixture(false);
        let mut selected = paginate_staging_table(&layout, &ir, input, &limits).unwrap();
        let duplicate = selected.row_fragments[1].continuation_before.entries[0].clone();
        selected.row_fragments[1]
            .continuation_before
            .entries
            .push(duplicate);
        assert_eq!(
            selected.validate_closure(&layout, &ir).unwrap_err(),
            StagingTablePaginationError::DuplicateContinuation { column_ordinal: 0 }
        );

        let (layout, ir) = table_fragmentation_fixture(false);
        let mut selected = paginate_staging_table(&layout, &ir, input, &limits).unwrap();
        selected.row_fragments[1].continuation_before.entries[0].cell_owner = NodeId::new(99);
        assert_eq!(
            selected.validate_closure(&layout, &ir).unwrap_err(),
            StagingTablePaginationError::WrongContinuationOwner { column_ordinal: 0 }
        );

        let mut guard = TableAttemptGuard::default();
        let attempt = TableCandidateAttempt {
            row_owner: NodeId::new(5),
            row_fragment_ordinal: 0,
            page_index: 0,
            row_block_offset: 0,
            available_block_size: 2,
        };
        guard.record(attempt).unwrap();
        assert_eq!(
            guard.record(attempt).unwrap_err(),
            StagingTablePaginationError::SameCandidateRetry(NodeId::new(5))
        );
    }

    #[test]
    fn table_fragmentation_rejects_header_source_and_repetition_tamper() {
        let input =
            StagingTablePageInput::new(machine_list_length(7), machine_list_length(7)).unwrap();
        let limits = table_fragmentation_limits(21);
        let (layout, ir) = table_fragmentation_fixture(false);
        let mut selected = paginate_staging_table(&layout, &ir, input, &limits).unwrap();
        selected.header_repetitions[1].repetition_index = 7;
        assert_eq!(
            selected.validate_closure(&layout, &ir).unwrap_err(),
            StagingTablePaginationError::WrongRepetitionIndex {
                expected: 1,
                actual: 7
            }
        );

        let (layout, ir) = table_fragmentation_fixture(false);
        let mut selected = paginate_staging_table(&layout, &ir, input, &limits).unwrap();
        selected.header_repetitions[1].rows[0].source_fragment_id = 99;
        assert_eq!(
            selected.validate_closure(&layout, &ir).unwrap_err(),
            StagingTablePaginationError::SelectedStateMismatch
        );

        let (layout, ir) = table_fragmentation_fixture(false);
        let mut selected = paginate_staging_table(&layout, &ir, input, &limits).unwrap();
        selected.header_repetitions[1].rows[0].target_block_offset = 1;
        assert_eq!(
            selected.validate_closure(&layout, &ir).unwrap_err(),
            StagingTablePaginationError::SelectedStateMismatch
        );
    }

    fn forced_page_break_layout_fixture() -> (StagingForcedPageBreakLayoutReceipt, ProductionFlowIr)
    {
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let paragraph = |node_id| wire::WireBlock::Paragraph {
            node_id,
            span,
            classes: Vec::new(),
            children: Vec::new(),
        };
        let page_break = |node_id| wire::WireBlock::PageBreak {
            node_id,
            span,
            classes: Vec::new(),
        };
        let package = wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_1,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "input.tsf".to_owned(),
                utf8_byte_length: 0,
                sha256: sha256(&[]),
            }],
            text_buffers: Vec::new(),
            document: wire::WireDocument {
                node_id: 0,
                blocks: vec![
                    page_break(1),
                    paragraph(2),
                    page_break(3),
                    page_break(4),
                    paragraph(5),
                    page_break(6),
                ],
                footnotes: Vec::new(),
            },
            style_sheet: wire::WireStyleSheet { rules: Vec::new() },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![wire::WirePageMaster {
                    master_id: "default".to_owned(),
                    width: 100,
                    height: 100,
                    body: wire::WireRect {
                        x: 0,
                        y: 0,
                        width: 100,
                        height: 100,
                    },
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: Vec::new(),
            },
            resources: wire::WireResourceCatalog {
                font_faces: Vec::new(),
                images: Vec::new(),
            },
        };
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&package)
            .unwrap();
        let decoded = wire::StagingStyleDocumentPackageDecoder::new()
            .decode(&bytes, &wire::DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let package = StagingStylePackageParser::new()
            .parse(
                decoded,
                String::new(),
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            )
            .unwrap();
        let preflight = package.preflight_forced_page_break_usage().unwrap();
        let generated_store = package
            .package()
            .materialize_initial_generated_text(&limits)
            .unwrap();
        let generated = package
            .package()
            .bind_generated_text(&generated_store, &limits)
            .unwrap();
        let admitted =
            AdmittedResourceResolver::new(&package.package().package().resources, &limits)
                .unwrap()
                .finish()
                .unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let ir = ProductionFlowIr::for_empty_paragraph_content(package.package(), epoch, &limits)
            .unwrap();
        let layout = layout_staging_forced_page_breaks(&package, &preflight, &ir).unwrap();
        (layout, ir)
    }

    #[test]
    fn forced_page_break_preserves_start_middle_consecutive_and_trailing_blank_pages() {
        let (layout, ir) = forced_page_break_layout_fixture();
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 5,
            ..ResourceLimits::default()
        })
        .unwrap();
        let input =
            StagingForcedPageBreakPaginationInput::new(&ir, vec![NodeId::new(2), NodeId::new(5)])
                .unwrap();
        let selected = paginate_staging_forced_page_breaks(&layout, &ir, &input, &limits).unwrap();
        assert_eq!(selected.page_count(), 5);
        assert_eq!(
            selected
                .pages()
                .iter()
                .map(StagingForcedPageBreakSelectedPage::is_blank)
                .collect::<Vec<_>>(),
            vec![true, false, true, false, true]
        );
        assert_eq!(selected.breaks().len(), 4);
        for (index, receipt) in selected.breaks().iter().enumerate() {
            assert_eq!(receipt.document_ordinal(), index as u32);
            assert_eq!(receipt.produced_page_index(), index as u32 + 1);
            assert_eq!(
                receipt.after_cursor().flow_local_ordinal(),
                receipt.before_cursor().flow_local_ordinal() + 1
            );
        }
        assert!(selected
            .trace_facts()
            .canonical_jcs()
            .contains("\"page_count\":5"));
    }

    #[test]
    fn forced_page_break_page_limit_is_inclusive_and_rejects_max_plus_one() {
        let (layout, ir) = forced_page_break_layout_fixture();
        let input = StagingForcedPageBreakPaginationInput::new(&ir, Vec::new()).unwrap();
        let exact = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 5,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(
            paginate_staging_forced_page_breaks(&layout, &ir, &input, &exact)
                .unwrap()
                .page_count(),
            5
        );
        let below = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 4,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(
            paginate_staging_forced_page_breaks(&layout, &ir, &input, &below).unwrap_err(),
            StagingForcedPageBreakPaginationError::PageLimit
        );
    }

    #[test]
    fn forced_page_break_rejects_break_paint_and_cursor_tamper() {
        let (layout, ir) = forced_page_break_layout_fixture();
        assert_eq!(
            StagingForcedPageBreakPaginationInput::new(&ir, vec![NodeId::new(1)]).unwrap_err(),
            StagingForcedPageBreakPaginationError::ForcedBoundaryPaint(NodeId::new(1))
        );
        let input = StagingForcedPageBreakPaginationInput::new(&ir, Vec::new()).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let mut selected =
            paginate_staging_forced_page_breaks(&layout, &ir, &input, &limits).unwrap();
        selected.breaks[0].after_cursor = selected.breaks[0].before_cursor;
        let error = selected.validate_break_closure().unwrap_err();
        assert_eq!(
            error,
            StagingForcedPageBreakPaginationError::CursorDidNotAdvance(NodeId::new(1))
        );
        assert_eq!(error.invariant_diagnostic_code(), Some(I9190));
    }

    fn machine_list_layout_fixture(
        first_item_height: i64,
        first_line_height: i64,
    ) -> (StagingMachineListLayoutReceipt, ProductionFlowIr) {
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let paragraph = |node_id| wire::WireBlock::Paragraph {
            node_id,
            span,
            classes: Vec::new(),
            children: Vec::new(),
        };
        let package = wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_1,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "input.tsf".to_owned(),
                utf8_byte_length: 0,
                sha256: sha256(&[]),
            }],
            text_buffers: Vec::new(),
            document: wire::WireDocument {
                node_id: 0,
                blocks: vec![wire::WireBlock::List {
                    node_id: 1,
                    span,
                    classes: Vec::new(),
                    ordered: true,
                    start: Some(9),
                    items: vec![
                        wire::WireListItem {
                            node_id: 2,
                            span,
                            blocks: vec![
                                paragraph(3),
                                wire::WireBlock::List {
                                    node_id: 4,
                                    span,
                                    classes: Vec::new(),
                                    ordered: false,
                                    start: None,
                                    items: vec![wire::WireListItem {
                                        node_id: 5,
                                        span,
                                        blocks: vec![paragraph(6)],
                                    }],
                                },
                            ],
                        },
                        wire::WireListItem {
                            node_id: 7,
                            span,
                            blocks: vec![paragraph(8)],
                        },
                    ],
                }],
                footnotes: Vec::new(),
            },
            style_sheet: wire::WireStyleSheet {
                rules: vec![wire::WireStyleRule {
                    style_id: "list-style".to_owned(),
                    extends: None,
                    selector: "list".to_owned(),
                    source_order: 0,
                    declarations: vec![
                        wire::WireDeclaration {
                            name: wire::WireDeclarationName::FontFamily,
                            value: wire::WireStyleValue::FontFamilyList {
                                families: vec!["Fixture".to_owned()],
                            },
                            important: false,
                        },
                        wire::WireDeclaration {
                            name: wire::WireDeclarationName::FontSize,
                            value: wire::WireStyleValue::Length { value: 10 },
                            important: false,
                        },
                        wire::WireDeclaration {
                            name: wire::WireDeclarationName::LineHeight,
                            value: wire::WireStyleValue::Length { value: 12 },
                            important: false,
                        },
                        wire::WireDeclaration {
                            name: wire::WireDeclarationName::StartIndent,
                            value: wire::WireStyleValue::Length { value: 5 },
                            important: false,
                        },
                        wire::WireDeclaration {
                            name: wire::WireDeclarationName::EndIndent,
                            value: wire::WireStyleValue::Length { value: 3 },
                            important: false,
                        },
                    ],
                }],
            },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![wire::WirePageMaster {
                    master_id: "default".to_owned(),
                    width: 100,
                    height: 100,
                    body: wire::WireRect {
                        x: 0,
                        y: 0,
                        width: 100,
                        height: 100,
                    },
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: Vec::new(),
            },
            resources: wire::WireResourceCatalog {
                font_faces: Vec::new(),
                images: Vec::new(),
            },
        };
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&package)
            .unwrap();
        let decoded = wire::StagingStyleDocumentPackageDecoder::new()
            .decode(&bytes, &wire::DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let package = StagingStylePackageParser::new()
            .parse(
                decoded,
                String::new(),
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            )
            .unwrap();
        let preflight = package.preflight_list_marker_usage(&limits).unwrap();
        let generated_store = package
            .package()
            .materialize_initial_generated_text(&limits)
            .unwrap();
        let generated = package
            .package()
            .bind_generated_text(&generated_store, &limits)
            .unwrap();
        let admitted =
            AdmittedResourceResolver::new(&package.package().package().resources, &limits)
                .unwrap()
                .finish()
                .unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let ir = ProductionFlowIr::for_empty_paragraph_content(package.package(), epoch, &limits)
            .unwrap();
        let item = |owner, marker_width, line_width, line_height, total_height| {
            StagingListItemPaintInput::painted(
                NodeId::new(owner),
                machine_list_length(marker_width),
                machine_list_length(line_width),
                machine_list_length(line_height),
                machine_list_length(total_height),
            )
        };
        let layout = layout_staging_machine_lists(
            &package,
            &preflight,
            generated,
            &ir,
            StagingMachineListLayoutInput::new(
                machine_list_length(100),
                typaxis_core::BidiLevel::LTR,
                vec![
                    item(2, 4, 20, first_line_height, first_item_height),
                    item(5, 6, 18, 8, 12),
                    item(7, 8, 24, 8, 16),
                ],
            ),
        )
        .unwrap();
        (layout, ir)
    }

    #[test]
    fn machine_list_page_split_moves_marker_with_first_line_and_closes_nested_flows() {
        let (layout, ir) = machine_list_layout_fixture(18, 8);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let selected = paginate_staging_machine_lists(
            &layout,
            &ir,
            StagingMachineListPageInput::new(machine_list_length(20), machine_list_length(5))
                .unwrap(),
            &limits,
        )
        .unwrap();
        assert!(selected.validate_marker_closure().is_ok());
        assert_eq!(selected.multi_flow().terminals().len(), 4);
        let first = selected
            .items()
            .iter()
            .find(|item| item.item_owner() == NodeId::new(2))
            .unwrap();
        assert_eq!(first.page_index(), 1);
        assert_eq!(first.marker_fragment_id(), first.first_line_fragment_id());
        let first_fragment = selected
            .fragments()
            .iter()
            .find(|fragment| fragment.fragment_id() == first.marker_fragment_id())
            .unwrap();
        assert!(first_fragment.contains_marker());
        assert!(first_fragment.contains_first_painted_line());
        assert_eq!(first_fragment.item_flow_id(), FlowId::new(1));

        let nested = selected
            .items()
            .iter()
            .find(|item| item.item_owner() == NodeId::new(5))
            .unwrap();
        assert_eq!(nested.list_flow_id(), FlowId::new(1));
        assert_eq!(nested.item_flow_id(), FlowId::new(2));
        let trace = selected.trace_facts();
        let traced = trace
            .items()
            .iter()
            .find(|item| item.item_owner() == 5)
            .unwrap();
        assert_eq!(traced.list_flow_id(), nested.list_flow_id().get());
        assert_eq!(traced.item_flow_id(), nested.item_flow_id().get());
        assert_eq!(traced.marker_fragment_id(), nested.marker_fragment_id());
    }

    #[test]
    fn machine_list_fragment_limit_is_inclusive_and_consumed_before_max_plus_one() {
        let (layout, ir) = machine_list_layout_fixture(30, 8);
        let page =
            StagingMachineListPageInput::new(machine_list_length(20), machine_list_length(20))
                .unwrap();
        let exact = ValidatedResourceLimits::new(ResourceLimits {
            max_fragments: 4,
            ..ResourceLimits::default()
        })
        .unwrap();
        let selected = paginate_staging_machine_lists(&layout, &ir, page, &exact).unwrap();
        assert_eq!(selected.fragments().len(), 4);

        let below = ValidatedResourceLimits::new(ResourceLimits {
            max_fragments: 3,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(
            paginate_staging_machine_lists(&layout, &ir, page, &below).unwrap_err(),
            StagingMachineListPaginationError::FragmentLimit
        );
    }

    #[test]
    fn machine_list_oversize_keep_and_same_candidate_more_are_terminal() {
        let (layout, ir) = machine_list_layout_fixture(25, 25);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        assert_eq!(
            paginate_staging_machine_lists(
                &layout,
                &ir,
                StagingMachineListPageInput::new(machine_list_length(20), machine_list_length(20),)
                    .unwrap(),
                &limits,
            )
            .unwrap_err(),
            StagingMachineListPaginationError::OversizeKeep(NodeId::new(2))
        );

        let cursor = ListProgress {
            item_ordinal: 0,
            remaining_item_raw: 12,
            page_index: 0,
        };
        assert_eq!(
            ensure_list_progress(NodeId::new(2), cursor, cursor),
            Err(StagingMachineListPaginationError::NoProgress(NodeId::new(
                2
            )))
        );
    }

    static NEXT_MULTI_FLOW_FIXTURE: AtomicU64 = AtomicU64::new(0);

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn multi_flow_machine_package() -> Box<ValidatedMachinePackage> {
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let paragraph = |node_id| wire::WireBlock::Paragraph {
            node_id,
            span,
            classes: Vec::new(),
            children: Vec::new(),
        };
        let wire_package = wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_1,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "input.tsf".to_owned(),
                utf8_byte_length: 0,
                sha256: sha256(&[]),
            }],
            text_buffers: Vec::new(),
            document: wire::WireDocument {
                node_id: 0,
                blocks: vec![
                    paragraph(1),
                    wire::WireBlock::List {
                        node_id: 2,
                        span,
                        classes: Vec::new(),
                        ordered: true,
                        start: Some(1),
                        items: vec![wire::WireListItem {
                            node_id: 3,
                            span,
                            blocks: vec![
                                paragraph(4),
                                wire::WireBlock::List {
                                    node_id: 5,
                                    span,
                                    classes: Vec::new(),
                                    ordered: false,
                                    start: None,
                                    items: vec![wire::WireListItem {
                                        node_id: 6,
                                        span,
                                        blocks: vec![paragraph(7)],
                                    }],
                                },
                            ],
                        }],
                    },
                    wire::WireBlock::PageBreak {
                        node_id: 8,
                        span,
                        classes: Vec::new(),
                    },
                ],
                footnotes: Vec::new(),
            },
            style_sheet: wire::WireStyleSheet { rules: Vec::new() },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![wire::WirePageMaster {
                    master_id: "default".to_owned(),
                    width: 100,
                    height: 100,
                    body: wire::WireRect {
                        x: 0,
                        y: 0,
                        width: 100,
                        height: 100,
                    },
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: Vec::new(),
            },
            resources: wire::WireResourceCatalog {
                font_faces: Vec::new(),
                images: Vec::new(),
            },
        };
        let root = std::env::temp_dir().join(format!(
            "typaxis-pagination-multi-flow-{}-{}",
            std::process::id(),
            NEXT_MULTI_FLOW_FIXTURE.fetch_add(1, AtomicOrdering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let package_path = root.join("document-package.json");
        fs::write(
            &package_path,
            wire::DocumentPackageEncoder::default()
                .to_jcs_vec(&wire_package)
                .unwrap(),
        )
        .unwrap();
        fs::write(root.join("input.tsf"), []).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let (session, raw) = HostMachineInputSession::open(
            MachineInputHostOptions::new(HostPath::new(package_path).unwrap(), None),
            &limits,
        )
        .unwrap();
        let decoded = session
            .decode_and_bind(
                &raw,
                &wire::StrictDocumentPackageDecoder::new(),
                &wire::DocumentPackageDecodePolicy::new(&limits),
            )
            .unwrap();
        let sources = session.admit_sources(&decoded, &limits).unwrap();
        let admitted = session.finish(raw, decoded, sources).unwrap();
        let allowed_schemes = typaxis_core::DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|scheme| (*scheme).to_owned())
            .collect::<Vec<_>>();
        let policy = PackageValidationPolicy::new(&limits, &allowed_schemes).unwrap();
        let parsed = match DocumentPackageParser::new().parse(admitted, &policy) {
            MachineParseOutcome::Parsed { package } => package,
            MachineParseOutcome::Failed { failure, .. } => {
                panic!("multi-flow package failed: {failure}")
            }
        };
        fs::remove_dir_all(root).unwrap();
        parsed
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn multi_flow_ir() -> (Box<ValidatedMachinePackage>, ProductionFlowIr) {
        let machine = multi_flow_machine_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = machine
            .package()
            .materialize_initial_generated_text(&limits)
            .unwrap();
        let package_epoch = epoch_for(machine.package(), &generated);
        let ir = ProductionFlowIr::for_empty_paragraph_content(
            machine.package(),
            package_epoch,
            &limits,
        )
        .unwrap();
        (machine, ir)
    }
    fn validated_flow_package() -> ValidatedParsedPackage {
        parsed_reference_package("flow-input.tsf", "anchor:chapter\nparagraph")
    }
    fn reference_flow(package: &ValidatedParsedPackage) -> FlowTree {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            Vec::new(),
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let package_epoch = epoch_for(package, &generated);
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(package, package_epoch).unwrap();
        let mut builder = CanonicalFlowIrBuilder::new(package, &paragraph_items).unwrap();
        for (node, kind) in package.document_nodes().nodes() {
            if kind == DocumentNodeKind::Paragraph {
                builder.push_paragraph_item(node, 0).unwrap();
            }
        }
        builder.finish(package_epoch).unwrap()
    }
    fn validated_package() -> ValidatedParsedPackage {
        validated_package_with_uri("input.tsf")
    }
    fn pagination_context() -> PackagePaginationContext {
        validated_package().pagination_context()
    }
    fn epoch_for(package: &ValidatedParsedPackage, store: &GeneratedTextStore) -> LayoutEpoch {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let generated = package.bind_generated_text(store, &limits).unwrap();
        LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap()
    }
    fn initial_state() -> InitialPaginationState {
        let package = validated_package();
        let store = generated_store();
        let flow = FlowTree::empty(&package, epoch_for(&package, &store)).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        InitialPaginationState::new(&flow, &package, &limits).unwrap()
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn multi_flow_stack_keeps_nested_progress_independent_and_monotonic() {
        let (_machine, ir) = multi_flow_ir();
        let mut cursor = MultiFlowCursorReceipt::new(&ir).unwrap();
        assert_eq!(cursor.active_stack(), &[FlowId::DOCUMENT_BODY]);
        assert_eq!(cursor.flow_progress(FlowId::new(0)), Some(0));
        assert_eq!(cursor.flow_progress(FlowId::new(1)), Some(0));
        assert_eq!(cursor.flow_progress(FlowId::new(2)), Some(0));

        cursor.advance(&ir).unwrap();
        assert_eq!(cursor.flow_progress(FlowId::new(0)), Some(1));
        cursor.enter_child(&ir, FlowId::new(1)).unwrap();
        assert_eq!(cursor.active_stack(), &[FlowId::new(0), FlowId::new(1)]);
        cursor.advance(&ir).unwrap();
        assert_eq!(cursor.flow_progress(FlowId::new(0)), Some(1));
        assert_eq!(cursor.flow_progress(FlowId::new(1)), Some(1));

        cursor.enter_child(&ir, FlowId::new(2)).unwrap();
        cursor.advance(&ir).unwrap();
        assert!(cursor.current_position(&ir).unwrap().is_terminal());
        assert_eq!(cursor.flow_progress(FlowId::new(0)), Some(1));
        assert_eq!(cursor.flow_progress(FlowId::new(1)), Some(1));
        assert_eq!(cursor.flow_progress(FlowId::new(2)), Some(1));
        cursor.leave_terminal(&ir).unwrap();
        cursor.advance(&ir).unwrap();
        assert!(cursor.current_position(&ir).unwrap().is_terminal());
        cursor.leave_terminal(&ir).unwrap();

        cursor.advance(&ir).unwrap();
        assert_eq!(cursor.flow_progress(FlowId::new(0)), Some(2));
        cursor.advance(&ir).unwrap();
        assert_eq!(cursor.flow_progress(FlowId::new(0)), Some(3));
        assert!(cursor.current_position(&ir).unwrap().is_terminal());
        assert_eq!(
            cursor.advance(&ir),
            Err(MultiFlowError::WrongTerminal(FlowId::DOCUMENT_BODY))
        );

        let selected = cursor.finish(&ir).unwrap();
        assert_eq!(selected.terminals().len(), 3);
        assert_eq!(
            selected.registry_fingerprint(),
            ir.registry().receipt().fingerprint()
        );
        let trace = MultiFlowTraceFacts::new(&ir, &selected).unwrap();
        assert_eq!(trace.positions().len(), 9);
        assert_eq!(
            trace
                .positions()
                .iter()
                .filter(|position| position.is_terminal())
                .count(),
            3
        );
        assert!(trace.canonical_jcs().contains("\"flow_id\":2"));
        assert!(trace.canonical_jcs().contains("\"parent_flow_id\":1"));
        assert!(trace.canonical_jcs().contains("\"terminal\":true"));
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn multi_flow_worker_completion_order_has_identical_trace_and_fingerprint() {
        let (machine, ir) = multi_flow_ir();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let paragraph_items = ValidatedParagraphItemRegistry::for_empty_content(
            machine.package(),
            ir.registry().receipt().epoch(),
        )
        .unwrap();
        let mut reverse_registration = ProductionFlowIrBuilder::new(
            machine.package(),
            &paragraph_items,
            ir.registry().receipt().epoch(),
            &limits,
        )
        .unwrap();
        let mut owners: Vec<_> = reverse_registration.expected_content_owners().collect();
        owners.reverse();
        for owner in owners {
            let content = reverse_registration.issue_content(owner).unwrap();
            reverse_registration.register_content(content).unwrap();
        }
        let reverse_registration = reverse_registration.finish().unwrap();
        assert_eq!(
            reverse_registration.registry().receipt().fingerprint(),
            ir.registry().receipt().fingerprint()
        );

        let select = |selected_ir: &ProductionFlowIr, order: &[FlowId]| {
            let mut selection = MultiFlowSelectionBuilder::new(selected_ir);
            for flow_id in order {
                let mut worker = FlowWorkerCursor::new(selected_ir, *flow_id).unwrap();
                let terminal = selected_ir
                    .registry()
                    .flow(*flow_id)
                    .unwrap()
                    .terminal()
                    .owner_local_ordinal();
                while worker.next_boundary() < terminal {
                    worker.advance(selected_ir).unwrap();
                }
                selection
                    .register(worker.finish(selected_ir).unwrap())
                    .unwrap();
            }
            selection.finish().unwrap()
        };
        let reverse = select(
            &reverse_registration,
            &[FlowId::new(2), FlowId::new(0), FlowId::new(1)],
        );
        let canonical = select(&ir, &[FlowId::new(0), FlowId::new(1), FlowId::new(2)]);
        assert_eq!(reverse.fingerprint(), canonical.fingerprint());
        assert_eq!(reverse.terminals(), canonical.terminals());
        let reverse_trace = MultiFlowTraceFacts::new(&reverse_registration, &reverse).unwrap();
        let canonical_trace = MultiFlowTraceFacts::new(&ir, &canonical).unwrap();
        assert_eq!(
            reverse_trace.canonical_jcs(),
            canonical_trace.canonical_jcs()
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn multi_flow_selection_rejects_missing_duplicate_and_nonterminal_workers() {
        let (_machine, ir) = multi_flow_ir();
        let worker = FlowWorkerCursor::new(&ir, FlowId::DOCUMENT_BODY).unwrap();
        assert_eq!(
            worker.finish(&ir).unwrap_err(),
            MultiFlowError::WrongTerminal(FlowId::DOCUMENT_BODY)
        );

        let completed = |flow_id: FlowId| {
            let mut worker = FlowWorkerCursor::new(&ir, flow_id).unwrap();
            let terminal = ir
                .registry()
                .flow(flow_id)
                .unwrap()
                .terminal()
                .owner_local_ordinal();
            while worker.next_boundary() < terminal {
                worker.advance(&ir).unwrap();
            }
            worker.finish(&ir).unwrap()
        };
        let mut missing = MultiFlowSelectionBuilder::new(&ir);
        missing.register(completed(FlowId::new(0))).unwrap();
        assert_eq!(
            missing.finish().unwrap_err(),
            MultiFlowError::MissingFlow(FlowId::new(1))
        );

        let mut duplicate = MultiFlowSelectionBuilder::new(&ir);
        duplicate.register(completed(FlowId::new(0))).unwrap();
        duplicate.register(completed(FlowId::new(0))).unwrap();
        duplicate.register(completed(FlowId::new(1))).unwrap();
        assert_eq!(
            duplicate.finish().unwrap_err(),
            MultiFlowError::DuplicateFlow(FlowId::new(0))
        );

        let finish_with_tamper = |tampered: CompletedFlowReceipt| {
            let tampered_id = tampered.flow_id;
            let mut tampered = Some(tampered);
            let mut selection = MultiFlowSelectionBuilder::new(&ir);
            for flow_id in [FlowId::new(0), FlowId::new(1), FlowId::new(2)] {
                let receipt = if flow_id == tampered_id {
                    tampered.take().unwrap()
                } else {
                    completed(flow_id)
                };
                selection.register(receipt).unwrap();
            }
            selection.finish().unwrap_err()
        };

        let mut wrong_owner = completed(FlowId::new(1));
        wrong_owner.owner_node_id = NodeId::new(99);
        assert_eq!(
            finish_with_tamper(wrong_owner),
            MultiFlowError::WrongOwner(FlowId::new(1))
        );

        let mut wrong_parent = completed(FlowId::new(1));
        wrong_parent.parent_flow_id = Some(FlowId::new(2));
        assert_eq!(
            finish_with_tamper(wrong_parent),
            MultiFlowError::WrongParent(FlowId::new(1))
        );

        let mut wrong_terminal = completed(FlowId::new(1));
        wrong_terminal.terminal = wrong_terminal.terminal.checked_add(1).unwrap();
        assert_eq!(
            finish_with_tamper(wrong_terminal),
            MultiFlowError::WrongTerminal(FlowId::new(1))
        );

        let other_package = validated_package();
        let other_store = generated_store();
        let mut wrong_epoch = completed(FlowId::new(1));
        wrong_epoch.epoch = epoch_for(&other_package, &other_store);
        assert_eq!(
            finish_with_tamper(wrong_epoch),
            MultiFlowError::EpochMismatch
        );
    }
    fn transitioned_input(pass: &LayoutPass) -> LayoutPassInput<'_> {
        let package = validated_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        LayoutPassInput::transitioned(pass.transition_references(&package, &limits).unwrap())
    }
    fn fingerprint(value: u8) -> LayoutStateFingerprint {
        if value == 0 {
            initial_state().fingerprint()
        } else {
            LayoutStateFingerprint::from_untrusted_bytes([value; 32])
        }
    }
    fn generated_store() -> GeneratedTextStore {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let parsed = TextStore::new(vec![]).unwrap();
        GeneratedTextStore::new(
            vec![],
            &ValidatedDocumentNodeIndex::empty_document(),
            &limits,
            &parsed,
        )
        .unwrap()
    }
    fn page(page_index: u32, marker: &str) -> PagePlan {
        let package = validated_package();
        let body = package.package().page_masters.masters[0].body;
        let inset_raw = i64::from(marker.as_bytes().first().copied().unwrap_or(0) % 10);
        let inset = Length::from_raw(inset_raw).unwrap();
        let x = body.x().checked_add(inset).unwrap();
        let width = PositiveLength::new(body.width().get().checked_sub(inset).unwrap()).unwrap();
        PagePlan {
            page_index,
            master_id: MasterId::new("default").unwrap(),
            frames: vec![PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds: Rect::new(x, body.y(), width, body.height()),
            }],
            fragments: vec![],
            footnote_ids: vec![],
            float_decisions: vec![],
            column_decisions: vec![],
            resolved_references: vec![],
        }
    }
    fn pass(
        budget: &mut PaginationWorkBudget,
        pass_input: LayoutPassInput<'_>,
        score: FallbackScore,
        markers: &[&str],
    ) -> LayoutPass {
        let package = validated_package();
        let generated = generated_store();
        let flow = FlowTree::empty(&package, epoch_for(&package, &generated)).unwrap();
        let pages: Vec<_> = markers
            .iter()
            .enumerate()
            .map(|(index, marker)| page(index as u32, marker))
            .collect();
        let pass_index = pass_input.state_index().get();
        let input_fingerprint = pass_input.fingerprint();
        let mut permit = budget.begin_pass(pass_index, pass_input).unwrap();
        for page in &pages {
            let cursor = FlowCursor::document_start(&flow);
            let selection =
                typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
            let context =
                PageContext::select(page.page_index, &selection, &package.pagination_context())
                    .unwrap();
            permit.begin_page(&context, &cursor, &page.frames).unwrap();
            FragmentWorkBudget::consume_fragments(
                &mut permit,
                u64::try_from(page.fragments.len()).unwrap(),
            )
            .unwrap();
            permit.finish_page(page).unwrap();
        }
        for _ in 0..score.hard_violations() {
            permit.record_hard_violation().unwrap();
        }
        let components = score.components();
        permit.add_keep_cost(components.keep()).unwrap();
        permit
            .add_widow_orphan_cost(components.widow_orphan())
            .unwrap();
        permit
            .add_heading_isolation_cost(components.heading_isolation())
            .unwrap();
        permit
            .add_table_split_cost(components.table_split())
            .unwrap();
        permit
            .add_footnote_split_cost(components.footnote_split())
            .unwrap();
        permit
            .add_unused_space_cost(components.unused_space())
            .unwrap();
        permit.add_overflow_cost(components.overflow()).unwrap();
        let receipt = permit.finish(&flow, &pages).unwrap();
        LayoutPass::new(receipt, input_fingerprint, &flow, pages, generated).unwrap()
    }

    fn score(hard_violations: u32, keep: i64) -> FallbackScore {
        FallbackScore::new(
            hard_violations,
            CostComponents::new(keep, 0, 0, 0, 0, 0, 0).unwrap(),
        )
    }

    fn input<'a>(
        initial: LayoutStateFingerprint,
        options: PaginationOptions,
        package_context: &'a PackagePaginationContext,
    ) -> PaginationInput<'a> {
        let state = initial_state();
        assert_eq!(initial, state.fingerprint());
        PaginationInput::new(state, package_context, options).unwrap()
    }
    fn blank_page_context(
        page_index: u32,
        package_context: &PackagePaginationContext,
    ) -> (PageContext, FlowCursor, FlowTree) {
        let package = validated_package();
        let generated = generated_store();
        let flow = FlowTree::empty(&package, epoch_for(&package, &generated)).unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let selection =
            typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let page = PageContext::select(page_index, &selection, package_context).unwrap();
        (page, cursor, flow)
    }
    fn options(max_layout_passes: u16, strict: bool) -> PaginationOptions {
        let limits = ResourceLimits {
            max_layout_passes,
            ..ResourceLimits::default()
        };
        let limits = ValidatedResourceLimits::new(limits).unwrap();
        PaginationOptions::from_limits(&limits, strict)
    }
    fn default_options() -> PaginationOptions {
        options(ResourceLimits::default().max_layout_passes, false)
    }

    struct OneFragmenter {
        fragment: FragmentDraft,
        terminal: FlowCursor,
        discovered_anchors: Vec<DiscoveredAnchor>,
    }
    impl Fragmenter for OneFragmenter {
        fn fragment(
            &self,
            _request: &FragmentRequest<'_>,
            budget: &mut dyn FragmentWorkBudget,
        ) -> Result<FragmentResult, FragmentError> {
            budget.consume_fragments(1)?;
            Ok(FragmentResult {
                fragments: vec![self.fragment.clone()],
                continuation: Continuation::Exhausted(Box::new(self.terminal.clone())),
                discovered_footnotes: vec![],
                discovered_anchors: self.discovered_anchors.clone(),
            })
        }
    }

    struct EmptyMoreFragmenter {
        next: FlowCursor,
    }
    impl Fragmenter for EmptyMoreFragmenter {
        fn fragment(
            &self,
            _request: &FragmentRequest<'_>,
            _budget: &mut dyn FragmentWorkBudget,
        ) -> Result<FragmentResult, FragmentError> {
            Ok(FragmentResult {
                fragments: vec![],
                continuation: Continuation::More(Box::new(self.next.clone())),
                discovered_footnotes: vec![],
                discovered_anchors: vec![],
            })
        }
    }

    #[test]
    fn materialized_state_excludes_state_zero() {
        assert!(MaterializedStateIndex::new(0).is_none());
        assert_eq!(MaterializedStateIndex::new(1).unwrap().pass_index(), 0);
    }

    #[test]
    fn reference_transition_is_issued_by_the_exact_package_and_predecessor() {
        let masters = pagination_context();
        let mut pagination_input = input(fingerprint(0), options(2, false), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 0),
            &["a"],
        );
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let foreign = validated_package_with_uri("foreign-input.tsf");
        assert_eq!(
            first.transition_references(&foreign, &limits),
            Err(PaginationError::PackageEpochMismatch)
        );

        let package = validated_package();
        let next =
            LayoutPassInput::transitioned(first.transition_references(&package, &limits).unwrap());
        assert_eq!(next.state_index().get(), first.materialized_state().get());
        assert_eq!(next.fingerprint(), first.output_fingerprint());
        assert!(next
            .layout_epoch()
            .same_stable_inputs(first.fingerprint_record().layout_epoch()));
        assert_eq!(next.generated_text(), first.generated_text());
    }

    #[test]
    fn page_reference_transition_uses_checked_physical_anchor_page() {
        let package =
            parsed_reference_package("reference-input.tsf", "anchor:chapter\nreference:chapter");
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let initial = package.materialize_initial_generated_text(&limits).unwrap();
        let frame = package.package().page_masters.masters[0].body;
        let anchor = PlacedAnchor::new(
            &DiscoveredAnchor {
                anchor_id: AnchorId::new("chapter").unwrap(),
                owner_node: NodeId::new(1),
                position_in_frame: Point {
                    x: typaxis_core::Length::ZERO,
                    y: typaxis_core::Length::ZERO,
                },
            },
            2,
            PageFrameKind::Body,
            0,
            frame,
        )
        .unwrap();
        let resolved = resolve_next_generated_text(
            &package,
            &initial,
            core::slice::from_ref(&anchor),
            &limits,
        )
        .unwrap();
        let key = GeneratedBufferKey::new(NodeId::new(4), GenerationKind::PageReference, 0);
        assert_eq!(
            resolved
                .buffers()
                .iter()
                .find(|buffer| buffer.key() == key)
                .unwrap()
                .utf8(),
            "3"
        );
        assert_ne!(
            initial.reference_fingerprint(),
            resolved.reference_fingerprint()
        );
        let stable = resolve_next_generated_text(
            &package,
            &resolved,
            core::slice::from_ref(&anchor),
            &limits,
        )
        .unwrap();
        assert_eq!(resolved, stable);
    }

    #[test]
    fn pass_work_rejects_an_identical_predecessor_from_another_session() {
        let masters = pagination_context();
        let mut input_a = input(fingerprint(0), options(2, false), &masters);
        let mut input_b = input(fingerprint(0), options(2, false), &masters);
        let mut budget_a = input_a.take_work_budget().unwrap();
        let mut budget_b = input_b.take_work_budget().unwrap();
        let first_a = pass(
            &mut budget_a,
            LayoutPassInput::initial(&input_a),
            score(0, 0),
            &["same"],
        );
        let first_b = pass(
            &mut budget_b,
            LayoutPassInput::initial(&input_b),
            score(0, 0),
            &["same"],
        );
        assert_eq!(first_a.output_fingerprint(), first_b.output_fingerprint());

        let package = validated_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let foreign = LayoutPassInput::transitioned(
            first_a.transition_references(&package, &limits).unwrap(),
        );
        assert!(matches!(
            budget_b.begin_pass(1, foreign),
            Err(PaginationError::InvalidWorkPermit)
        ));

        // The rejected foreign capability did not consume pass work. The
        // exact predecessor issued by this budget's own session still starts.
        let own = LayoutPassInput::transitioned(
            first_b.transition_references(&package, &limits).unwrap(),
        );
        assert!(budget_b.begin_pass(1, own).is_ok());
    }

    #[test]
    fn pagination_session_capability_has_opaque_order_independent_debug() {
        let first = PaginationSessionId::issue();
        let second = PaginationSessionId::issue();

        assert_eq!(first, first.clone());
        assert_ne!(first, second);
        assert_eq!(format!("{first:?}"), "PaginationSessionId(<opaque>)");
        assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }

    #[test]
    fn placed_anchor_order_is_anchor_id_not_page_order() {
        let point = Point {
            x: Length::ZERO,
            y: Length::ZERO,
        };
        let mut anchors = vec![
            PlacedAnchor {
                anchor_id: AnchorId::new("z-last").unwrap(),
                owner_node: NodeId::new(1),
                page_index: 0,
                frame_kind: PageFrameKind::Body,
                column_index: 0,
                position_in_frame: point,
            },
            PlacedAnchor {
                anchor_id: AnchorId::new("a-first").unwrap(),
                owner_node: NodeId::new(2),
                page_index: 1,
                frame_kind: PageFrameKind::Body,
                column_index: 0,
                position_in_frame: point,
            },
        ];
        canonicalize_placed_anchors(&mut anchors);
        assert_eq!(anchors[0].anchor_id().as_str(), "a-first");
        assert_eq!(anchors[0].page_index(), 1);
    }

    #[test]
    fn fallback_selects_score_before_earliest_state() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(4, false), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(1, 1),
            &["a"],
        );
        let second_input = transitioned_input(&first);
        let second = pass(&mut budget, second_input, score(0, 10), &["b"]);
        let third_input = transitioned_input(&second);
        let third = pass(&mut budget, third_input, score(0, 9), &["d"]);
        let fourth_input = transitioned_input(&third);
        let fourth = pass(&mut budget, fourth_input, score(0, 10), &["e"]);
        let result = PaginationResult::new(
            vec![first, second, third, fourth],
            ConvergenceStatus::MaxPassFallback,
            &pagination_input,
            budget.finish(),
        )
        .unwrap();
        assert_eq!(result.selected_state().get(), 3);
        assert_eq!(result.selected_pass().pass_index(), 2);
        assert_eq!(
            result.final_fingerprint(),
            result.selected_pass().output_fingerprint()
        );
    }

    #[test]
    fn strict_mode_rejects_fallback() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(1, true), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let passes = vec![pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 0),
            &["a"],
        )];
        assert_eq!(
            PaginationResult::new(
                passes,
                ConvergenceStatus::MaxPassFallback,
                &pagination_input,
                budget.finish(),
            ),
            Err(PaginationError::FallbackRejectedByStrict)
        );
    }

    #[test]
    fn converged_result_selects_last_materialized_pass() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(8, true), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 2),
            &["a"],
        );
        let stable_input = transitioned_input(&first);
        let stable = pass(&mut budget, stable_input, score(0, 1), &["a"]);
        let stable_fingerprint = stable.output_fingerprint();
        let result = PaginationResult::new(
            vec![first, stable],
            ConvergenceStatus::Converged,
            &pagination_input,
            budget.finish(),
        )
        .unwrap();
        assert_eq!(result.selected_state().get(), 2);
        assert_eq!(result.final_fingerprint(), stable_fingerprint);
    }

    #[test]
    fn cycle_fallback_can_only_name_a_prior_materialized_state() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(3, false), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 0),
            &["a"],
        );
        let second_input = transitioned_input(&first);
        let second = pass(&mut budget, second_input, score(0, 0), &["b"]);
        let third_input = transitioned_input(&second);
        let third = pass(&mut budget, third_input, score(0, 0), &["a"]);
        let result = PaginationResult::new(
            vec![first, second, third],
            ConvergenceStatus::CycleFallback {
                cycle_start_state: MaterializedStateIndex::new(1).unwrap(),
            },
            &pagination_input,
            budget.finish(),
        )
        .unwrap();
        assert_eq!(result.selected_state().get(), 1);
        assert!(MaterializedStateIndex::new(0).is_none());
    }

    #[test]
    fn pass_chain_must_be_contiguous() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(2, false), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 0),
            &["a"],
        );
        let wrong_input = LayoutPassInput {
            session: pagination_input.session.clone(),
            state_index: LayoutStateIndex::new(1),
            fingerprint: fingerprint(9),
            layout_epoch: first.fingerprint_record().layout_epoch(),
            generated_text: Cow::Borrowed(first.generated_text()),
        };
        let second = pass(&mut budget, wrong_input, score(0, 0), &["b"]);
        assert_eq!(
            PaginationResult::new(
                vec![first, second],
                ConvergenceStatus::MaxPassFallback,
                &pagination_input,
                budget.finish(),
            ),
            Err(PaginationError::BrokenFingerprintChain)
        );
    }

    #[test]
    fn prepared_layout_rejects_cursor_epoch_mismatch() {
        let state = initial_state();
        let first = state.layout_epoch();
        let first_package = validated_package();
        let generated = generated_store();
        let other_package = validated_package_with_uri("other-input.tsf");
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let admitted = AdmittedResourceResolver::new(&other_package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let generated = other_package
            .bind_generated_text(&generated, &limits)
            .unwrap();
        let second = LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let flow = FlowTree::empty(&first_package, first).unwrap();
        let other = FlowTree::empty(&other_package, second).unwrap();
        let cursor = FlowCursor::document_start(&other);
        let masters = pagination_context();
        let pagination_input = PaginationInput::new(state, &masters, default_options()).unwrap();
        assert_eq!(
            PreparedLayout::new(LayoutPassInput::initial(&pagination_input), flow, cursor),
            Err(PaginationError::InvalidPreparedLayout)
        );
    }

    #[test]
    fn pagination_input_rejects_another_packages_page_master_receipt() {
        let state = initial_state();
        let other_context = validated_package_with_uri("other-input.tsf").pagination_context();
        assert_eq!(
            PaginationInput::new(state, &other_context, default_options()),
            Err(PaginationError::PackageEpochMismatch)
        );
    }

    #[test]
    fn fallback_cost_is_a_signed_json_safe_integer() {
        assert_eq!(
            score(0, -JSON_SAFE_INTEGER_MAX).total_cost(),
            -JSON_SAFE_INTEGER_MAX
        );
        assert!(CostComponents::new(JSON_SAFE_INTEGER_MAX + 1, 0, 0, 0, 0, 0, 0).is_none());
        assert!(CostComponents::new(-JSON_SAFE_INTEGER_MAX - 1, 0, 0, 0, 0, 0, 0).is_none());
        assert!(CostComponents::new(JSON_SAFE_INTEGER_MAX, 1, 0, 0, 0, 0, 0).is_none());
    }

    #[test]
    fn work_budget_rejects_each_max_plus_one_before_work() {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 1,
            max_layout_passes: 1,
            max_fragments: 1,
            max_page_break_lookback: 1,
            max_footnote_reflows_per_page: 1,
            max_column_balance_candidates: 1,
            max_float_queue: 1,
            max_float_carry_pages: 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        let options = PaginationOptions::from_limits(&limits, false);
        let masters = pagination_context();

        let mut pass_limited_input = input(fingerprint(0), options, &masters);
        let mut pass_limited_budget = pass_limited_input.take_work_budget().unwrap();
        let first = pass(
            &mut pass_limited_budget,
            LayoutPassInput::initial(&pass_limited_input),
            score(0, 0),
            &["a"],
        );
        assert!(matches!(
            pass_limited_budget.begin_pass(1, transitioned_input(&first)),
            Err(PaginationError::ResourceLimit)
        ));

        let mut page_limited_input = input(fingerprint(0), options, &masters);
        let mut page_limited_budget = page_limited_input.take_work_budget().unwrap();
        let mut page_permit = page_limited_budget
            .begin_pass(0, LayoutPassInput::initial(&page_limited_input))
            .unwrap();
        let first_page = page(0, "a");
        let (first_context, first_cursor, _first_flow) = blank_page_context(0, &masters);
        page_permit
            .begin_page(&first_context, &first_cursor, &first_page.frames)
            .unwrap();
        page_permit.finish_page(&first_page).unwrap();
        let (second_context, second_cursor, _second_flow) = blank_page_context(1, &masters);
        assert_eq!(
            page_permit.begin_page(&second_context, &second_cursor, &first_page.frames),
            Err(PaginationError::ResourceLimit)
        );

        let mut work_input = input(fingerprint(0), options, &masters);
        let mut work_budget = work_input.take_work_budget().unwrap();
        let mut permit = work_budget
            .begin_pass(0, LayoutPassInput::initial(&work_input))
            .unwrap();
        permit
            .begin_page(&first_context, &first_cursor, &first_page.frames)
            .unwrap();
        assert!(permit.consume_fragments(1).is_ok());
        assert_eq!(
            permit.consume_fragments(1),
            Err(FragmentError::ResourceLimit)
        );
        assert!(permit.consume_footnote_reflow(0).is_ok());
        assert_eq!(
            permit.consume_footnote_reflow(0),
            Err(FragmentError::ResourceLimit)
        );
        assert!(permit.consume_column_candidate(NodeId::new(1)).is_ok());
        assert_eq!(
            permit.consume_column_candidate(NodeId::new(1)),
            Err(FragmentError::ResourceLimit)
        );
        assert!(permit.enqueue_float(NodeId::new(1), 0).is_ok());
        assert_eq!(
            permit.enqueue_float(NodeId::new(2), 0),
            Err(FragmentError::ResourceLimit)
        );
        assert!(permit.consume_float_carry(NodeId::new(1), 0).is_ok());
        assert_eq!(
            permit.consume_float_carry(NodeId::new(1), 0),
            Err(FragmentError::ResourceLimit)
        );
        assert_eq!(
            permit.dequeue_float(NodeId::new(2), 0),
            Err(FragmentError::InvalidFloatState)
        );
        assert!(permit.dequeue_float(NodeId::new(1), 0).is_ok());
        assert_eq!(
            permit.consume_float_carry(NodeId::new(1), 0),
            Err(FragmentError::InvalidFloatState)
        );
        let mut lookback = permit.begin_page_break_search().unwrap();
        assert!(lookback.consume_candidate().is_ok());
        assert_eq!(
            lookback.consume_candidate(),
            Err(PaginationError::ResourceLimit)
        );
        assert!(permit.finish_page_break_search(lookback).is_ok());
        assert_eq!(
            permit.begin_page_break_search(),
            Err(PaginationError::InvalidWorkPermit)
        );
        assert_eq!(
            permit.finish_page(&first_page),
            Err(PaginationError::InvalidWorkPermit)
        );

        let mut input = PaginationInput::new(initial_state(), &masters, options).unwrap();
        assert!(input.take_work_budget().is_ok());
        assert_eq!(
            input.take_work_budget(),
            Err(PaginationError::WorkBudgetAlreadyIssued)
        );
    }

    #[test]
    fn unresolved_float_queue_cannot_materialize_a_pass() {
        let package_context = pagination_context();
        let mut input = input(fingerprint(0), default_options(), &package_context);
        let mut budget = input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();
        let plan = page(0, "d");
        let (page_context, cursor, flow) = blank_page_context(0, &package_context);
        permit
            .begin_page(&page_context, &cursor, &plan.frames)
            .unwrap();
        permit.enqueue_float(NodeId::new(0), 0).unwrap();
        permit.finish_page(&plan).unwrap();
        assert_eq!(
            permit.finish(&flow, core::slice::from_ref(&plan)),
            Err(PaginationError::InvalidWorkPermit)
        );
    }

    #[test]
    fn page_plan_decisions_require_exact_work_receipts() {
        fn assert_rejected(plan: PagePlan) {
            let package_context = pagination_context();
            let mut input = input(fingerprint(0), default_options(), &package_context);
            let mut budget = input.take_work_budget().unwrap();
            let mut permit = budget
                .begin_pass(0, LayoutPassInput::initial(&input))
                .unwrap();
            let (page_context, cursor, _flow) = blank_page_context(0, &package_context);
            permit
                .begin_page(&page_context, &cursor, &plan.frames)
                .unwrap();
            assert_eq!(
                permit.finish_page(&plan),
                Err(PaginationError::InvalidWorkPermit)
            );
        }

        let mut footnote = page(0, "a");
        footnote.footnote_ids = vec![FootnoteId::new("note").unwrap()];
        assert_rejected(footnote);

        let mut float = page(0, "a");
        float.float_decisions = vec![FloatDecision {
            owner: NodeId::new(0),
            owner_local_ordinal: 0,
            frame_kind: PageFrameKind::Body,
            column_index: 0,
            bounds: float.frames[0].bounds,
        }];
        assert_rejected(float);

        let mut column = page(0, "a");
        column.column_decisions = vec![ColumnDecision {
            container: NodeId::new(0),
            column_index: 0,
            bounds: column.frames[0].bounds,
        }];
        assert_rejected(column);
    }

    #[test]
    fn float_and_column_state_machines_issue_exact_page_receipts() {
        let package_context = pagination_context();
        let mut input = input(fingerprint(0), default_options(), &package_context);
        let mut budget = input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();
        let mut plan = page(0, "a");
        let (page_context, cursor, flow) = blank_page_context(0, &package_context);
        permit
            .begin_page(&page_context, &cursor, &plan.frames)
            .unwrap();

        permit.enqueue_float(NodeId::new(0), 0).unwrap();
        permit.dequeue_float(NodeId::new(0), 0).unwrap();
        let float = FloatDecision {
            owner: NodeId::new(0),
            owner_local_ordinal: 0,
            frame_kind: PageFrameKind::Body,
            column_index: 0,
            bounds: plan.frames[0].bounds,
        };
        permit.record_float_decision(float.clone()).unwrap();
        plan.float_decisions.push(float);

        permit.consume_column_candidate(NodeId::new(0)).unwrap();
        let column = ColumnDecision {
            container: NodeId::new(0),
            column_index: 0,
            bounds: plan.frames[0].bounds,
        };
        permit
            .record_column_decisions(NodeId::new(0), vec![column.clone()])
            .unwrap();
        plan.column_decisions.push(column);

        permit.finish_page(&plan).unwrap();
        assert!(permit.finish(&flow, core::slice::from_ref(&plan)).is_ok());
    }

    #[test]
    fn max_pass_status_requires_exact_limit_and_returns_advisory() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut default_input = input(seed, default_options(), &masters);
        let mut default_budget = default_input.take_work_budget().unwrap();
        let materialized = pass(
            &mut default_budget,
            LayoutPassInput::initial(&default_input),
            score(0, 0),
            &["a"],
        );
        assert_eq!(
            PaginationOutcome::new(
                vec![materialized],
                ConvergenceStatus::MaxPassFallback,
                &default_input,
                default_budget.finish(),
            ),
            Err(PaginationError::InvalidMaxPassState)
        );
        let mut exact_input = input(seed, options(1, false), &masters);
        let mut exact_budget = exact_input.take_work_budget().unwrap();
        let materialized = pass(
            &mut exact_budget,
            LayoutPassInput::initial(&exact_input),
            score(0, 0),
            &["a"],
        );
        let outcome = PaginationOutcome::new(
            vec![materialized],
            ConvergenceStatus::MaxPassFallback,
            &exact_input,
            exact_budget.finish(),
        )
        .unwrap();
        assert_eq!(outcome.diagnostics().len(), 1);
        assert_eq!(outcome.result().selected_state().get(), 1);
    }

    #[test]
    fn materialized_pass_requires_a_nonempty_dense_page_sequence() {
        let masters = pagination_context();
        let mut pagination_input = input(fingerprint(0), default_options(), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let empty_package = validated_package();
        let empty_store = generated_store();
        let empty_flow =
            FlowTree::empty(&empty_package, epoch_for(&empty_package, &empty_store)).unwrap();
        let empty_permit = budget
            .begin_pass(0, LayoutPassInput::initial(&pagination_input))
            .unwrap();
        assert_eq!(
            empty_permit.finish(&empty_flow, &[]),
            Err(PaginationError::InvalidWorkPermit)
        );
        let mut wrong = page(1, "a");
        wrong.page_index = 1;
        let wrong_package = validated_package();
        let wrong_store = generated_store();
        let wrong_flow =
            FlowTree::empty(&wrong_package, epoch_for(&wrong_package, &wrong_store)).unwrap();
        let forged_receipt = PassMaterializationReceipt {
            summary: PassBudgetSummary {
                session: pagination_input.session.clone(),
                pass_index: 0,
                input_fingerprint: fingerprint(0),
                layout_epoch: wrong_flow.epoch(),
                generated_text: wrong_store.clone(),
                pages: vec![PageBudgetSummary {
                    page_index: 1,
                    page_start: wrong_flow.positions()[0].clone(),
                    flow_owner: NodeId::new(0),
                    content_owner: NodeId::new(0),
                    master_id: wrong.master_id.clone(),
                    frames: wrong.frames.clone(),
                    consumed_fragment_count: 0,
                    fragments: vec![],
                    footnote_ids: BTreeSet::new(),
                    float_decisions: vec![],
                    column_decisions: vec![],
                    resolved_references: vec![],
                    placed_anchors: vec![],
                    continuation: None,
                    next_page_start: None,
                    fragmenter_invoked: false,
                    footnote_evaluation: None,
                    footnote_cursor_progress: false,
                    next_fragment_cursor: wrong_flow.positions()[0].clone(),
                    fragmentation_exhausted: false,
                    lookback_search_issued: false,
                    lookback_search_completed: false,
                }],
                fallback_score: score(0, 0),
            },
        };
        assert_eq!(
            LayoutPass::new(
                forged_receipt,
                fingerprint(0),
                &wrong_flow,
                vec![wrong],
                wrong_store,
            ),
            Err(PaginationError::InvalidPageIndex)
        );
    }

    #[test]
    fn materialized_pass_is_bound_to_the_permitted_input_identity() {
        let masters = pagination_context();
        let mut pagination_input = input(fingerprint(0), default_options(), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let package = validated_package();
        let store = generated_store();
        let flow = FlowTree::empty(&package, epoch_for(&package, &store)).unwrap();
        let pages = vec![page(0, "a")];
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&pagination_input))
            .unwrap();
        let (context, cursor, _context_flow) = blank_page_context(0, &masters);
        permit
            .begin_page(&context, &cursor, &pages[0].frames)
            .unwrap();
        permit.finish_page(&pages[0]).unwrap();
        let receipt = permit.finish(&flow, &pages).unwrap();
        assert_eq!(
            LayoutPass::new(receipt, fingerprint(9), &flow, pages, store),
            Err(PaginationError::InvalidWorkPermit)
        );
    }

    #[test]
    fn page_materialization_accepts_only_the_fragmenter_recorded_sequence() {
        let package = validated_flow_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let bound = package.bind_generated_text(&generated, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(bound, admitted.token()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(&package, epoch).unwrap();
        let mut flow_builder = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(1), 0).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(3), 0).unwrap();
        let flow = flow_builder.finish(epoch).unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let selection =
            typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let page_context =
            PageContext::select(0, &selection, &package.pagination_context()).unwrap();
        let initial = InitialPaginationState::new(&flow, &package, &limits).unwrap();
        let package_context = package.pagination_context();
        let mut input = PaginationInput::new(initial, &package_context, default_options()).unwrap();
        let mut budget = input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();
        let mut plan = page(0, "d");
        permit
            .begin_page(&page_context, &cursor, &plan.frames)
            .unwrap();

        let next =
            FlowCursor::at(&flow, 1, typaxis_layout::CursorPosition::ParagraphItem(0)).unwrap();
        let bootstrap_request = FragmentRequest::new(
            &flow,
            &cursor,
            plan.frames[0].bounds,
            NonNegativeLength::ZERO,
            page_context.clone(),
        )
        .unwrap();
        permit
            .run_fragmenter(
                &EmptyMoreFragmenter { next: next.clone() },
                &bootstrap_request,
                PageFrameKind::Body,
                0,
            )
            .unwrap();

        let fragment = FragmentDraft::new(
            flow.positions()[1].clone(),
            flow.positions().last().unwrap().clone(),
            plan.frames[0].bounds,
            0,
        )
        .unwrap();
        let fragmenter = OneFragmenter {
            fragment,
            terminal: flow.terminal_cursor(),
            discovered_anchors: vec![DiscoveredAnchor {
                anchor_id: AnchorId::new("chapter").unwrap(),
                owner_node: NodeId::new(2),
                position_in_frame: Point {
                    x: Length::ZERO,
                    y: Length::ZERO,
                },
            }],
        };
        let request = FragmentRequest::new(
            &flow,
            &next,
            plan.frames[0].bounds,
            NonNegativeLength::ZERO,
            page_context,
        )
        .unwrap();
        let materialized = permit
            .run_fragmenter(&fragmenter, &request, PageFrameKind::Body, 0)
            .unwrap();
        assert_eq!(materialized.placed_fragments()[0].owner_local_ordinal, 0);
        assert_eq!(materialized.placed_anchors()[0].page_index(), 0);
        assert_eq!(
            materialized.placed_anchors()[0]
                .position_on_page(plan.frames[0].bounds)
                .unwrap(),
            Point {
                x: plan.frames[0].bounds.x(),
                y: plan.frames[0].bounds.y(),
            }
        );
        assert_eq!(
            permit.run_fragmenter(&fragmenter, &request, PageFrameKind::Body, 0),
            Err(FragmentError::InvalidPageContext)
        );
        plan.fragments
            .extend_from_slice(materialized.placed_fragments());
        let mut sparse = plan.clone();
        sparse.fragments[0].owner_local_ordinal = 1;
        assert_eq!(
            PaginationFingerprintRecord::new(
                &flow,
                vec![sparse],
                generated.clone(),
                materialized.placed_anchors().to_vec(),
            ),
            Err(PaginationError::InvalidFragmentRange)
        );
        assert!(permit.finish_page(&plan).is_ok());
        let receipt = permit.finish(&flow, core::slice::from_ref(&plan)).unwrap();
        let mut wrong_anchor = receipt.clone();
        wrong_anchor.summary.pages[0].placed_anchors[0].owner_node = NodeId::new(1);
        assert_eq!(
            LayoutPass::new(
                wrong_anchor,
                input.initial_fingerprint(),
                &flow,
                vec![plan.clone()],
                generated.clone(),
            ),
            Err(PaginationError::InvalidWorkPermit)
        );
        let mut missing_anchor = receipt.clone();
        missing_anchor.summary.pages[0].placed_anchors.clear();
        assert_eq!(
            LayoutPass::new(
                missing_anchor,
                input.initial_fingerprint(),
                &flow,
                vec![plan.clone()],
                generated.clone(),
            ),
            Err(PaginationError::InvalidWorkPermit)
        );
        let mut moved_anchor = receipt.clone();
        moved_anchor.summary.pages[0].placed_anchors[0]
            .position_in_frame
            .x = Length::from_raw(1).unwrap();
        let moved_pass = LayoutPass::new(
            moved_anchor,
            input.initial_fingerprint(),
            &flow,
            vec![plan.clone()],
            generated.clone(),
        )
        .unwrap();
        let pass = LayoutPass::new(
            receipt,
            input.initial_fingerprint(),
            &flow,
            vec![plan],
            generated,
        )
        .unwrap();
        assert_eq!(pass.placed_anchors().count(), 1);
        assert_ne!(pass.output_fingerprint(), moved_pass.output_fingerprint());
    }

    #[test]
    fn pass_receipt_requires_an_exact_continuation_chain_to_terminal() {
        let package = validated_flow_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let bound = package.bind_generated_text(&generated, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(bound, admitted.token()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(&package, epoch).unwrap();
        let mut flow_builder = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(1), 0).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(3), 0).unwrap();
        let flow = flow_builder.finish(epoch).unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let selection =
            typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let page_context =
            PageContext::select(0, &selection, &package.pagination_context()).unwrap();
        let initial = InitialPaginationState::new(&flow, &package, &limits).unwrap();
        let package_context = package.pagination_context();
        let mut input = PaginationInput::new(initial, &package_context, default_options()).unwrap();
        let mut budget = input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();
        let plan = page(0, "d");
        permit
            .begin_page(&page_context, &cursor, &plan.frames)
            .unwrap();
        let request = FragmentRequest::new(
            &flow,
            &cursor,
            plan.frames[0].bounds,
            NonNegativeLength::ZERO,
            page_context,
        )
        .unwrap();
        let next =
            FlowCursor::at(&flow, 1, typaxis_layout::CursorPosition::ParagraphItem(0)).unwrap();
        permit
            .run_fragmenter(
                &EmptyMoreFragmenter { next },
                &request,
                PageFrameKind::Body,
                0,
            )
            .unwrap();
        permit.finish_page(&plan).unwrap();

        let wrong_selection =
            typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let wrong_page = PageContext::select(1, &wrong_selection, &package_context).unwrap();
        assert_eq!(
            permit.begin_page(&wrong_page, &cursor, &plan.frames),
            Err(PaginationError::InvalidWorkPermit)
        );
        assert_eq!(
            permit.finish(&flow, core::slice::from_ref(&plan)),
            Err(PaginationError::InvalidWorkPermit)
        );
    }

    #[test]
    fn multiple_frame_invocations_follow_the_exact_continuation() {
        let package = validated_flow_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![],
            package.document_nodes(),
            &limits,
            &package.package().text_store,
        )
        .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let bound = package.bind_generated_text(&generated, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(bound, admitted.token()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(&package, epoch).unwrap();
        let mut flow_builder = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(1), 0).unwrap();
        flow_builder.push_paragraph_item(NodeId::new(3), 0).unwrap();
        let flow = flow_builder.finish(epoch).unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let selection =
            typaxis_layout::ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let page_context =
            PageContext::select(0, &selection, &package.pagination_context()).unwrap();
        let package_context = package.pagination_context();
        let initial = InitialPaginationState::new(&flow, &package, &limits).unwrap();
        let mut input = PaginationInput::new(initial, &package_context, default_options()).unwrap();
        let mut budget = input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&input))
            .unwrap();

        let body = package.package().page_masters.masters[0].body;
        let left_width =
            PositiveLength::new(Length::from_raw(body.width().get().raw() / 2).unwrap()).unwrap();
        let right_width =
            PositiveLength::new(body.width().get().checked_sub(left_width.get()).unwrap()).unwrap();
        let right_x = body.x().checked_add(left_width.get()).unwrap();
        let frames = vec![
            PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 0,
                bounds: Rect::new(body.x(), body.y(), left_width, body.height()),
            },
            PageFramePlan {
                kind: PageFrameKind::Body,
                column_index: 1,
                bounds: Rect::new(right_x, body.y(), right_width, body.height()),
            },
        ];
        let mut plan = page(0, "d");
        plan.frames = frames;
        permit
            .begin_page(&page_context, &cursor, &plan.frames)
            .unwrap();

        let next =
            FlowCursor::at(&flow, 1, typaxis_layout::CursorPosition::ParagraphItem(0)).unwrap();
        let first_request = FragmentRequest::new(
            &flow,
            &cursor,
            plan.frames[0].bounds,
            NonNegativeLength::ZERO,
            page_context.clone(),
        )
        .unwrap();
        assert_eq!(
            permit.run_fragmenter(
                &EmptyMoreFragmenter { next: next.clone() },
                &first_request,
                PageFrameKind::Header,
                0,
            ),
            Err(FragmentError::UnsupportedFlowDomain)
        );
        permit
            .run_fragmenter(
                &EmptyMoreFragmenter { next: next.clone() },
                &first_request,
                PageFrameKind::Body,
                0,
            )
            .unwrap();

        let replay = FragmentRequest::new(
            &flow,
            &cursor,
            plan.frames[1].bounds,
            NonNegativeLength::ZERO,
            page_context.clone(),
        )
        .unwrap();
        assert_eq!(
            permit.run_fragmenter(
                &EmptyMoreFragmenter { next: next.clone() },
                &replay,
                PageFrameKind::Body,
                1,
            ),
            Err(FragmentError::InvalidPageContext)
        );

        let final_fragment = FragmentDraft::new(
            flow.positions()[1].clone(),
            flow.positions().last().unwrap().clone(),
            plan.frames[1].bounds,
            0,
        )
        .unwrap();
        let second_request = FragmentRequest::new(
            &flow,
            &next,
            plan.frames[1].bounds,
            NonNegativeLength::ZERO,
            page_context,
        )
        .unwrap();
        let final_receipt = permit
            .run_fragmenter(
                &OneFragmenter {
                    fragment: final_fragment,
                    terminal: flow.terminal_cursor(),
                    discovered_anchors: vec![DiscoveredAnchor {
                        anchor_id: AnchorId::new("chapter").unwrap(),
                        owner_node: NodeId::new(2),
                        position_in_frame: Point {
                            x: Length::ZERO,
                            y: Length::ZERO,
                        },
                    }],
                },
                &second_request,
                PageFrameKind::Body,
                1,
            )
            .unwrap();
        plan.fragments
            .extend_from_slice(final_receipt.placed_fragments());
        permit.finish_page(&plan).unwrap();
        let receipt = permit.finish(&flow, core::slice::from_ref(&plan)).unwrap();
        assert!(LayoutPass::new(
            receipt,
            input.initial_fingerprint(),
            &flow,
            vec![plan],
            generated,
        )
        .is_ok());
    }

    #[test]
    fn page_frame_geometry_is_rejected_before_page_work_begins() {
        let masters = pagination_context();
        let mut pagination_input = input(fingerprint(0), default_options(), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let mut permit = budget
            .begin_pass(0, LayoutPassInput::initial(&pagination_input))
            .unwrap();
        let (context, cursor, _flow) = blank_page_context(0, &masters);
        let other_package = validated_package_with_uri("other-input.tsf");
        let other_context = other_package.pagination_context();
        let other_generated = generated_store();
        let other_flow =
            FlowTree::empty(&other_package, epoch_for(&other_package, &other_generated)).unwrap();
        let other_cursor = FlowCursor::document_start(&other_flow);
        let other_selection =
            typaxis_layout::ResolvedPageSelection::new(&other_flow, &other_cursor, &other_package)
                .unwrap();
        let other_page = PageContext::select(0, &other_selection, &other_context).unwrap();
        let valid = page(0, "a");
        assert_eq!(
            permit.begin_page(&other_page, &other_cursor, &valid.frames),
            Err(PaginationError::InvalidWorkPermit)
        );
        let one = PositiveLength::new(Length::from_raw(1).unwrap()).unwrap();
        let outside = [PageFramePlan {
            kind: PageFrameKind::Body,
            column_index: 0,
            bounds: Rect::new(
                context.selected_master().width.get(),
                Length::ZERO,
                one,
                one,
            ),
        }];
        assert_eq!(
            permit.begin_page(&context, &cursor, &outside),
            Err(PaginationError::FrameOutsideMaster)
        );

        assert!(permit.begin_page(&context, &cursor, &valid.frames).is_ok());
        assert!(permit.finish_page(&valid).is_ok());
    }

    #[test]
    fn fingerprint_record_canonicalizes_insertion_order() {
        let mut forward = page(0, "a");
        let mut reverse = page(0, "a");
        let bounds = forward.frames[0].bounds;
        let decision = |ordinal| FloatDecision {
            owner: NodeId::new(0),
            owner_local_ordinal: ordinal,
            frame_kind: PageFrameKind::Body,
            column_index: 0,
            bounds,
        };
        forward.float_decisions = vec![decision(1), decision(0)];
        reverse.float_decisions = vec![decision(0), decision(1)];
        let forward_package = validated_package();
        let forward_store = generated_store();
        let forward_flow = FlowTree::empty(
            &forward_package,
            epoch_for(&forward_package, &forward_store),
        )
        .unwrap();
        let reverse_package = validated_package();
        let reverse_store = generated_store();
        let reverse_flow = FlowTree::empty(
            &reverse_package,
            epoch_for(&reverse_package, &reverse_store),
        )
        .unwrap();
        assert_eq!(
            PaginationFingerprintRecord::new(&forward_flow, vec![forward], forward_store, vec![],)
                .unwrap(),
            PaginationFingerprintRecord::new(&reverse_flow, vec![reverse], reverse_store, vec![],)
                .unwrap()
        );
    }

    #[test]
    fn fingerprint_is_derived_and_length_domain_is_bounded() {
        let first_package = validated_package();
        let first_store = generated_store();
        let first_flow =
            FlowTree::empty(&first_package, epoch_for(&first_package, &first_store)).unwrap();
        let first =
            PaginationFingerprintRecord::new(&first_flow, vec![page(0, "a")], first_store, vec![])
                .unwrap();
        let second_package = validated_package();
        let second_store = generated_store();
        let second_flow =
            FlowTree::empty(&second_package, epoch_for(&second_package, &second_store)).unwrap();
        let second = PaginationFingerprintRecord::new(
            &second_flow,
            vec![page(0, "b")],
            second_store,
            vec![],
        )
        .unwrap();
        assert_ne!(first.fingerprint(), second.fingerprint());

        let mut invalid = page(0, "a");
        let one = PositiveLength::new(Length::from_raw(1).unwrap()).unwrap();
        assert!(Length::from_raw(i64::MAX).is_none());
        invalid.frames[0].bounds = Rect::new(
            Length::from_raw(JSON_SAFE_INTEGER_MAX).unwrap(),
            Length::ZERO,
            one,
            one,
        );
        let invalid_package = validated_package();
        let invalid_store = generated_store();
        let invalid_flow = FlowTree::empty(
            &invalid_package,
            epoch_for(&invalid_package, &invalid_store),
        )
        .unwrap();
        assert!(PaginationFingerprintRecord::new(
            &invalid_flow,
            vec![invalid],
            invalid_store,
            vec![],
        )
        .is_ok());
    }

    #[test]
    fn result_rejects_passes_after_an_observed_terminal_state() {
        let seed = fingerprint(0);
        let masters = pagination_context();
        let mut pagination_input = input(seed, options(3, false), &masters);
        let mut budget = pagination_input.take_work_budget().unwrap();
        let first = pass(
            &mut budget,
            LayoutPassInput::initial(&pagination_input),
            score(0, 0),
            &["a"],
        );
        let stable_input = transitioned_input(&first);
        let stable = pass(&mut budget, stable_input, score(0, 0), &["a"]);
        let after_input = transitioned_input(&stable);
        let after = pass(&mut budget, after_input, score(0, 0), &["b"]);
        assert_eq!(
            PaginationResult::new(
                vec![first, stable, after],
                ConvergenceStatus::MaxPassFallback,
                &pagination_input,
                budget.finish(),
            ),
            Err(PaginationError::PassesContinueAfterTermination)
        );
    }

    #[test]
    fn reference_paginator_materializes_nonblank_anchors_and_converges() {
        let package =
            parsed_reference_package("reference-pagination.tsf", "anchor:z\nparagraph\nanchor:a");
        let flow = reference_flow(&package);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let outcome = ReferencePaginator::new()
            .paginate(&package, &flow, &limits, false)
            .unwrap();
        let result = outcome.result();

        assert_eq!(result.status(), &ConvergenceStatus::Converged);
        assert_eq!(result.passes().len(), 2);
        assert_eq!(result.selected_state().get(), 2);
        assert!(outcome.diagnostics().is_empty());
        assert_eq!(result.selected_pages().len(), 1);
        assert_eq!(result.selected_pages()[0].fragments.len(), 3);
        assert_eq!(
            result.passes()[0].output_fingerprint(),
            result.passes()[1].output_fingerprint()
        );
        assert_eq!(
            result.passes()[1].input_fingerprint(),
            result.passes()[1].output_fingerprint()
        );

        let anchors = result.selected_anchors().collect::<Vec<_>>();
        assert_eq!(anchors.len(), 2);
        assert_eq!(anchors[0].anchor_id().as_str(), "a");
        assert_eq!(anchors[1].anchor_id().as_str(), "z");
        for anchor in anchors {
            assert_eq!(
                package.document_nodes().anchor_owner(anchor.anchor_id()),
                Some(anchor.owner_node())
            );
            assert_eq!(anchor.page_index(), 0);
            assert_eq!(anchor.frame_kind(), PageFrameKind::Body);
            assert_eq!(anchor.column_index(), 0);
            assert_eq!(
                anchor.position_in_frame(),
                Point {
                    x: Length::ZERO,
                    y: Length::ZERO,
                }
            );
        }
    }

    #[test]
    fn reference_paginator_is_deterministic_across_sessions() {
        let package = parsed_reference_package(
            "reference-repeat.tsf",
            "paragraph\nanchor:repeat\nparagraph",
        );
        let flow = reference_flow(&package);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let first = ReferencePaginator::new()
            .paginate(&package, &flow, &limits, false)
            .unwrap();
        let repeated = ReferencePaginator::new()
            .paginate(&package, &flow, &limits, false)
            .unwrap();

        assert_eq!(first.result().status(), repeated.result().status());
        assert_eq!(
            first.result().selected_pages(),
            repeated.result().selected_pages()
        );
        assert_eq!(
            first
                .result()
                .selected_anchors()
                .cloned()
                .collect::<Vec<_>>(),
            repeated
                .result()
                .selected_anchors()
                .cloned()
                .collect::<Vec<_>>()
        );
        assert_eq!(
            first.result().final_fingerprint(),
            repeated.result().final_fingerprint()
        );
        assert_eq!(
            first
                .result()
                .passes()
                .iter()
                .map(LayoutPass::output_fingerprint)
                .collect::<Vec<_>>(),
            repeated
                .result()
                .passes()
                .iter()
                .map(LayoutPass::output_fingerprint)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn reference_paginator_honors_exact_page_pass_and_fragment_limits() {
        let package = parsed_reference_package("reference-limits.tsf", "anchor:limit\nparagraph");
        let flow = reference_flow(&package);
        let exact = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 1,
            max_layout_passes: 2,
            max_fragments: 2,
            ..ResourceLimits::default()
        })
        .unwrap();
        let result = ReferencePaginator::new()
            .paginate(&package, &flow, &exact, false)
            .unwrap();
        assert_eq!(result.result().status(), &ConvergenceStatus::Converged);
        assert!(result
            .result()
            .passes()
            .iter()
            .all(|pass| { pass.pages().len() == 1 && pass.pages()[0].fragments.len() == 2 }));

        let too_few_fragments = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 1,
            max_layout_passes: 2,
            max_fragments: 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(
            ReferencePaginator::new().paginate(&package, &flow, &too_few_fragments, false),
            Err(PaginationError::ResourceLimit)
        );
    }

    #[test]
    fn reference_paginator_routes_fallback_through_strict_policy() {
        let package = parsed_reference_package("reference-strict.tsf", "paragraph");
        let flow = reference_flow(&package);
        let one_pass = ValidatedResourceLimits::new(ResourceLimits {
            max_pages: 1,
            max_layout_passes: 1,
            max_fragments: 1,
            ..ResourceLimits::default()
        })
        .unwrap();

        let fallback = ReferencePaginator::new()
            .paginate(&package, &flow, &one_pass, false)
            .unwrap();
        assert_eq!(
            fallback.result().status(),
            &ConvergenceStatus::MaxPassFallback
        );
        assert_eq!(fallback.result().selected_state().get(), 1);
        assert_eq!(fallback.diagnostics().len(), 1);
        assert_eq!(
            ReferencePaginator::new().paginate(&package, &flow, &one_pass, true),
            Err(PaginationError::FallbackRejectedByStrict)
        );
    }

    #[test]
    fn reference_paginator_materializes_the_canonical_blank_page() {
        let package = parsed_reference_package("reference-blank.tsf", "");
        let flow = reference_flow(&package);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let outcome = ReferencePaginator::new()
            .paginate(&package, &flow, &limits, false)
            .unwrap();
        let result = outcome.result();
        assert_eq!(result.status(), &ConvergenceStatus::Converged);
        assert_eq!(result.passes().len(), 2);
        assert_eq!(result.selected_pages().len(), 1);
        assert!(result.selected_pages()[0].fragments.is_empty());
        assert_eq!(result.selected_anchors().count(), 0);
    }
}
