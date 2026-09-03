use typaxis_core::{push_jcs_string, sha256, M4EffectiveResourceLimits, NodeId, SourceSpan};
use typaxis_document::{SemanticContainerKind, StagingM4Block, StagingM4Document};
use typaxis_layout_contract::FlowId;
use typaxis_style::{SemanticContainerComputedStyle, SemanticContainerStyleKind};
use typaxis_syntax::{
    StagingPrecomposedVectorProfileAuthorization, StagingSemanticContainerProfileView,
    ValidatedStagingSemanticPackage,
};

const FLOW_REGISTRY_ALGORITHM: &str = "typaxis.semantic-container-flow-registry/1";
const SELECTED_LAYOUT_ALGORITHM: &str = "typaxis.semantic-container-selected-layout/1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingSemanticContainerFlowOwnerKind {
    DocumentBody,
    SemanticContainer,
    ListItem,
    TableCell,
    FigureCaption,
    FootnoteDefinition,
}

impl StagingSemanticContainerFlowOwnerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DocumentBody => "document_body",
            Self::SemanticContainer => "semantic_container",
            Self::ListItem => "list_item",
            Self::TableCell => "table_cell",
            Self::FigureCaption => "figure_caption",
            Self::FootnoteDefinition => "footnote_definition",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingSemanticContainerFlowItemKind {
    Paragraph,
    Heading,
    List,
    Table,
    Figure,
    PageBreak,
    DisplayMath,
    SemanticContainer,
    FootnoteDefinition,
}

impl StagingSemanticContainerFlowItemKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::Heading => "heading",
            Self::List => "list",
            Self::Table => "table",
            Self::Figure => "figure",
            Self::PageBreak => "page_break",
            Self::DisplayMath => "display_math",
            Self::SemanticContainer => "semantic_container",
            Self::FootnoteDefinition => "footnote_definition",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerFlowItem {
    position: u32,
    owner: NodeId,
    kind: StagingSemanticContainerFlowItemKind,
    child_flow_ids: Vec<FlowId>,
}

impl StagingSemanticContainerFlowItem {
    pub const fn position(&self) -> u32 {
        self.position
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn kind(&self) -> StagingSemanticContainerFlowItemKind {
        self.kind
    }
    pub fn child_flow_ids(&self) -> &[FlowId] {
        &self.child_flow_ids
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerFlow {
    flow_id: FlowId,
    owner: NodeId,
    owner_kind: StagingSemanticContainerFlowOwnerKind,
    semantic_kind: Option<SemanticContainerKind>,
    parent_flow_id: Option<FlowId>,
    parent_position: Option<u32>,
    depth: u32,
    terminal: u32,
    items: Vec<StagingSemanticContainerFlowItem>,
}

impl StagingSemanticContainerFlow {
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn owner_kind(&self) -> StagingSemanticContainerFlowOwnerKind {
        self.owner_kind
    }
    pub const fn semantic_kind(&self) -> Option<SemanticContainerKind> {
        self.semantic_kind
    }
    pub const fn parent_flow_id(&self) -> Option<FlowId> {
        self.parent_flow_id
    }
    pub const fn parent_position(&self) -> Option<u32> {
        self.parent_position
    }
    pub const fn depth(&self) -> u32 {
        self.depth
    }
    pub fn items(&self) -> &[StagingSemanticContainerFlowItem] {
        &self.items
    }
    pub const fn terminal(&self) -> u32 {
        self.terminal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerFlowRegistryReceipt {
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    flow_count: u32,
    fingerprint: [u8; 32],
    canonical_jcs: String,
}

impl StagingSemanticContainerFlowRegistryReceipt {
    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package_fingerprint
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub const fn flow_count(&self) -> u32 {
        self.flow_count
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerFlowRegistry {
    flows: Vec<StagingSemanticContainerFlow>,
    receipt: StagingSemanticContainerFlowRegistryReceipt,
}

impl StagingSemanticContainerFlowRegistry {
    pub fn flows(&self) -> &[StagingSemanticContainerFlow] {
        &self.flows
    }
    pub fn flow(&self, id: FlowId) -> Option<&StagingSemanticContainerFlow> {
        self.flows
            .get(usize::try_from(id.get()).ok()?)
            .filter(|flow| flow.flow_id == id)
    }
    pub const fn receipt(&self) -> &StagingSemanticContainerFlowRegistryReceipt {
        &self.receipt
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerFragment {
    owner: NodeId,
    semantic_kind: SemanticContainerKind,
    flow_id: FlowId,
    parent_flow_id: FlowId,
    parent_position: u32,
    fragment_index: u32,
    page_index: u32,
    frame_index: u32,
    before_cursor: u32,
    after_cursor: u32,
    first: bool,
    last: bool,
    source_span: SourceSpan,
    computed_style: SemanticContainerComputedStyle,
    style_fingerprint: [u8; 32],
    child_owners: Vec<NodeId>,
    fingerprint: [u8; 32],
}

impl StagingSemanticContainerFragment {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn semantic_kind(&self) -> SemanticContainerKind {
        self.semantic_kind
    }
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
    pub const fn parent_flow_id(&self) -> FlowId {
        self.parent_flow_id
    }
    pub const fn parent_position(&self) -> u32 {
        self.parent_position
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn frame_index(&self) -> u32 {
        self.frame_index
    }
    pub const fn before_cursor(&self) -> u32 {
        self.before_cursor
    }
    pub const fn after_cursor(&self) -> u32 {
        self.after_cursor
    }
    pub const fn is_first(&self) -> bool {
        self.first
    }
    pub const fn is_last(&self) -> bool {
        self.last
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn computed_style(&self) -> &SemanticContainerComputedStyle {
        &self.computed_style
    }
    pub const fn style_fingerprint(&self) -> [u8; 32] {
        self.style_fingerprint
    }
    pub fn child_owners(&self) -> &[NodeId] {
        &self.child_owners
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerSelectedLayoutReceipt {
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    flow_registry_fingerprint: [u8; 32],
    fragment_count: u32,
    fingerprint: [u8; 32],
    canonical_jcs: String,
}

impl StagingSemanticContainerSelectedLayoutReceipt {
    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package_fingerprint
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub const fn flow_registry_fingerprint(&self) -> [u8; 32] {
        self.flow_registry_fingerprint
    }
    pub const fn fragment_count(&self) -> u32 {
        self.fragment_count
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerSelectedLayout {
    registry: StagingSemanticContainerFlowRegistry,
    fragments: Vec<StagingSemanticContainerFragment>,
    receipt: StagingSemanticContainerSelectedLayoutReceipt,
}

impl StagingSemanticContainerSelectedLayout {
    pub const fn registry(&self) -> &StagingSemanticContainerFlowRegistry {
        &self.registry
    }
    pub fn fragments(&self) -> &[StagingSemanticContainerFragment] {
        &self.fragments
    }
    pub const fn receipt(&self) -> &StagingSemanticContainerSelectedLayoutReceipt {
        &self.receipt
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingSemanticContainerProfileView,
    ) -> Result<(), StagingSemanticContainerLayoutError> {
        verify_registry(package, profile, &self.registry)?;
        let canonical = encode_selected_layout(&self.registry, &self.fragments);
        let fragment_count = u64::try_from(self.fragments.len())
            .map_err(|_| StagingSemanticContainerLayoutError::FragmentLimit)?;
        if self.receipt.package_fingerprint != package.semantic_fingerprint()
            || self.receipt.profile_fingerprint != profile.profile_fingerprint()
            || self.receipt.flow_registry_fingerprint != self.registry.receipt.fingerprint
            || usize::try_from(self.receipt.fragment_count) != Ok(self.fragments.len())
            || self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
            || fragment_count > profile.limits().get().max_fragments
            || fragment_count > u64::from(profile.limits().get().max_pages)
            || !fragments_are_closed(&self.registry, &self.fragments, package)
        {
            return Err(StagingSemanticContainerLayoutError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSemanticContainerLayoutError {
    ProfileMismatch,
    PackageMismatch,
    FlowLimit,
    FlowDepthLimit,
    FragmentLimit,
    PageLimit,
    EmptyFlow(NodeId),
    MissingStyle(NodeId),
    InvalidGeometry(NodeId),
    ZeroFragmentCapacity,
    ArithmeticOverflow,
    ReceiptMismatch,
    AllocationFailure,
    PrecomposedVectorStaging(NodeId),
}

impl std::fmt::Display for StagingSemanticContainerLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => {
                formatter.write_str("I9190: semantic profile receipt mismatch")
            }
            Self::PackageMismatch => {
                formatter.write_str("I9190: semantic package receipt mismatch")
            }
            Self::FlowLimit => formatter.write_str("L5100: semantic flow count limit exceeded"),
            Self::FlowDepthLimit => {
                formatter.write_str("L5100: semantic flow depth limit exceeded")
            }
            Self::FragmentLimit => formatter.write_str("L5100: semantic fragment limit exceeded"),
            Self::PageLimit => formatter.write_str("L5100: semantic page limit exceeded"),
            Self::EmptyFlow(owner) => write!(
                formatter,
                "L5100: empty semantic flow at node {}",
                owner.get()
            ),
            Self::MissingStyle(owner) => write!(
                formatter,
                "L5101: missing typed semantic style at node {}",
                owner.get()
            ),
            Self::InvalidGeometry(owner) => write!(
                formatter,
                "L5101: semantic container indents exhaust the child frame at node {}",
                owner.get()
            ),
            Self::ZeroFragmentCapacity => {
                formatter.write_str("L5100: semantic fragment capacity is zero")
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("L5100: semantic layout arithmetic overflow")
            }
            Self::ReceiptMismatch => formatter.write_str("I9190: semantic layout receipt mismatch"),
            Self::AllocationFailure => {
                formatter.write_str("L5100: semantic layout allocation failed")
            }
            Self::PrecomposedVectorStaging(owner) => write!(
                formatter,
                "P1102: precomposed vector at node {} requires its versioned layout",
                owner.get()
            ),
        }
    }
}

impl std::error::Error for StagingSemanticContainerLayoutError {}

/// Builds only the existing parent-flow registry for the private precomposed
/// vector path. The public semantic-container profile remains fail-closed for
/// precomposed vectors; this projection reuses its fixed `/1` item vocabulary
/// and maps producer-composed blocks onto existing atomic categories.
pub(crate) fn project_staging_precomposed_vector_parent_flows(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingSemanticContainerFlowRegistry, StagingSemanticContainerLayoutError> {
    profile
        .authorizes(package, limits)
        .map_err(|_| StagingSemanticContainerLayoutError::ProfileMismatch)?;
    let flows = build_package_flows(package, limits.base())?;
    let profile_fingerprint = profile.profile_receipt_fingerprint();
    let registry_jcs = encode_registry(&flows, package.semantic_fingerprint(), profile_fingerprint);
    let registry = StagingSemanticContainerFlowRegistry {
        receipt: StagingSemanticContainerFlowRegistryReceipt {
            package_fingerprint: package.semantic_fingerprint(),
            profile_fingerprint,
            flow_count: u32::try_from(flows.len())
                .map_err(|_| StagingSemanticContainerLayoutError::FlowLimit)?,
            fingerprint: sha256(registry_jcs.as_bytes()),
            canonical_jcs: registry_jcs,
        },
        flows,
    };
    verify_registry_binding(package, profile_fingerprint, limits.base(), &registry)?;
    Ok(registry)
}

pub fn layout_staging_semantic_containers(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSemanticContainerProfileView,
    fragment_item_capacity: u32,
) -> Result<StagingSemanticContainerSelectedLayout, StagingSemanticContainerLayoutError> {
    if profile.package_sha256() != package.canonical_jcs_sha256()
        || profile.semantic_fingerprint() != package.semantic_fingerprint()
    {
        return Err(StagingSemanticContainerLayoutError::ProfileMismatch);
    }
    if fragment_item_capacity == 0 {
        return Err(StagingSemanticContainerLayoutError::ZeroFragmentCapacity);
    }
    let flows = build_package_flows(package, profile.limits())?;
    let registry_jcs = encode_registry(
        &flows,
        package.semantic_fingerprint(),
        profile.profile_fingerprint(),
    );
    let registry = StagingSemanticContainerFlowRegistry {
        receipt: StagingSemanticContainerFlowRegistryReceipt {
            package_fingerprint: package.semantic_fingerprint(),
            profile_fingerprint: profile.profile_fingerprint(),
            flow_count: u32::try_from(flows.len())
                .map_err(|_| StagingSemanticContainerLayoutError::FlowLimit)?,
            fingerprint: sha256(registry_jcs.as_bytes()),
            canonical_jcs: registry_jcs,
        },
        flows,
    };
    verify_registry(package, profile, &registry)?;

    let mut fragments = Vec::new();
    let mut page_index = 0u32;
    for flow in registry
        .flows
        .iter()
        .filter(|flow| flow.owner_kind == StagingSemanticContainerFlowOwnerKind::SemanticContainer)
    {
        if flow.items.is_empty() {
            return Err(StagingSemanticContainerLayoutError::EmptyFlow(flow.owner));
        }
        let semantic_kind = flow
            .semantic_kind
            .ok_or(StagingSemanticContainerLayoutError::PackageMismatch)?;
        let style = package.computed_style(flow.owner).ok_or(
            StagingSemanticContainerLayoutError::MissingStyle(flow.owner),
        )?;
        ensure_style_kind(semantic_kind, style, flow.owner)?;
        let block_style = style.block_style();
        100i64
            .checked_sub(block_style.start_indent().get().raw())
            .and_then(|remaining| remaining.checked_sub(block_style.end_indent().get().raw()))
            .filter(|remaining| *remaining > 0)
            .ok_or(StagingSemanticContainerLayoutError::InvalidGeometry(
                flow.owner,
            ))?;
        let source_span = find_block(package.document(), flow.owner)
            .ok_or(StagingSemanticContainerLayoutError::PackageMismatch)?
            .span();
        let style_fingerprint = style_fingerprint(flow.owner, style);
        let mut before = 0u32;
        let terminal = flow.terminal();
        let mut fragment_index = 0u32;
        while before < terminal {
            if u64::try_from(fragments.len())
                .map_err(|_| StagingSemanticContainerLayoutError::FragmentLimit)?
                >= profile.limits().get().max_fragments
            {
                return Err(StagingSemanticContainerLayoutError::FragmentLimit);
            }
            if page_index >= profile.limits().get().max_pages {
                return Err(StagingSemanticContainerLayoutError::PageLimit);
            }
            let after = before
                .checked_add(fragment_item_capacity)
                .map_or(terminal, |value| value.min(terminal));
            if after <= before {
                return Err(StagingSemanticContainerLayoutError::ReceiptMismatch);
            }
            let start = usize::try_from(before)
                .map_err(|_| StagingSemanticContainerLayoutError::ArithmeticOverflow)?;
            let end = usize::try_from(after)
                .map_err(|_| StagingSemanticContainerLayoutError::ArithmeticOverflow)?;
            let mut child_owners = Vec::new();
            child_owners
                .try_reserve_exact(end - start)
                .map_err(|_| StagingSemanticContainerLayoutError::AllocationFailure)?;
            child_owners.extend(
                flow.items[start..end]
                    .iter()
                    .map(StagingSemanticContainerFlowItem::owner),
            );
            let mut fragment = StagingSemanticContainerFragment {
                owner: flow.owner,
                semantic_kind,
                flow_id: flow.flow_id,
                parent_flow_id: flow
                    .parent_flow_id
                    .ok_or(StagingSemanticContainerLayoutError::PackageMismatch)?,
                parent_position: flow
                    .parent_position
                    .ok_or(StagingSemanticContainerLayoutError::PackageMismatch)?,
                fragment_index,
                page_index,
                frame_index: 0,
                before_cursor: before,
                after_cursor: after,
                first: before == 0,
                last: after == terminal,
                source_span,
                computed_style: style.clone(),
                style_fingerprint,
                child_owners,
                fingerprint: [0; 32],
            };
            fragment.fingerprint = sha256(encode_fragment(&fragment).as_bytes());
            fragments
                .try_reserve(1)
                .map_err(|_| StagingSemanticContainerLayoutError::AllocationFailure)?;
            fragments.push(fragment);
            before = after;
            fragment_index = fragment_index
                .checked_add(1)
                .ok_or(StagingSemanticContainerLayoutError::ArithmeticOverflow)?;
            page_index = page_index
                .checked_add(1)
                .ok_or(StagingSemanticContainerLayoutError::ArithmeticOverflow)?;
        }
    }
    let selected_jcs = encode_selected_layout(&registry, &fragments);
    let selected = StagingSemanticContainerSelectedLayout {
        receipt: StagingSemanticContainerSelectedLayoutReceipt {
            package_fingerprint: package.semantic_fingerprint(),
            profile_fingerprint: profile.profile_fingerprint(),
            flow_registry_fingerprint: registry.receipt.fingerprint,
            fragment_count: u32::try_from(fragments.len())
                .map_err(|_| StagingSemanticContainerLayoutError::FragmentLimit)?,
            fingerprint: sha256(selected_jcs.as_bytes()),
            canonical_jcs: selected_jcs,
        },
        registry,
        fragments,
    };
    selected.verify(package, profile)?;
    Ok(selected)
}

fn build_package_flows(
    package: &ValidatedStagingSemanticPackage,
    limits: &typaxis_core::ValidatedResourceLimits,
) -> Result<Vec<StagingSemanticContainerFlow>, StagingSemanticContainerLayoutError> {
    let mut flows = Vec::new();
    build_flow(
        limits,
        package.document().node_id,
        StagingSemanticContainerFlowOwnerKind::DocumentBody,
        None,
        None,
        None,
        1,
        &package.document().blocks,
        &mut flows,
    )?;
    for footnote in &package.document().footnotes {
        let position = u32::try_from(
            flows
                .first()
                .ok_or(StagingSemanticContainerLayoutError::PackageMismatch)?
                .items
                .len(),
        )
        .map_err(|_| StagingSemanticContainerLayoutError::FlowLimit)?;
        let child_flow_id = build_flow(
            limits,
            footnote.node_id,
            StagingSemanticContainerFlowOwnerKind::FootnoteDefinition,
            None,
            Some(FlowId::DOCUMENT_BODY),
            Some(position),
            2,
            &footnote.blocks,
            &mut flows,
        )?;
        let root = flows
            .get_mut(
                usize::try_from(FlowId::DOCUMENT_BODY.get())
                    .map_err(|_| StagingSemanticContainerLayoutError::FlowLimit)?,
            )
            .ok_or(StagingSemanticContainerLayoutError::PackageMismatch)?;
        root.items
            .try_reserve(1)
            .map_err(|_| StagingSemanticContainerLayoutError::AllocationFailure)?;
        root.items.push(StagingSemanticContainerFlowItem {
            position,
            owner: footnote.node_id,
            kind: StagingSemanticContainerFlowItemKind::FootnoteDefinition,
            child_flow_ids: vec![child_flow_id],
        });
    }
    Ok(flows)
}

#[allow(clippy::too_many_arguments)]
fn build_flow(
    limits: &typaxis_core::ValidatedResourceLimits,
    owner: NodeId,
    owner_kind: StagingSemanticContainerFlowOwnerKind,
    semantic_kind: Option<SemanticContainerKind>,
    parent_flow_id: Option<FlowId>,
    parent_position: Option<u32>,
    depth: u32,
    blocks: &[StagingM4Block],
    flows: &mut Vec<StagingSemanticContainerFlow>,
) -> Result<FlowId, StagingSemanticContainerLayoutError> {
    if depth > limits.get().max_ast_nesting_depth {
        return Err(StagingSemanticContainerLayoutError::FlowDepthLimit);
    }
    if u64::try_from(flows.len()).map_err(|_| StagingSemanticContainerLayoutError::FlowLimit)?
        >= limits.get().max_ast_nodes
    {
        return Err(StagingSemanticContainerLayoutError::FlowLimit);
    }
    let flow_id = FlowId::new(
        u32::try_from(flows.len()).map_err(|_| StagingSemanticContainerLayoutError::FlowLimit)?,
    );
    let terminal =
        u32::try_from(blocks.len()).map_err(|_| StagingSemanticContainerLayoutError::FlowLimit)?;
    flows
        .try_reserve(1)
        .map_err(|_| StagingSemanticContainerLayoutError::AllocationFailure)?;
    flows.push(StagingSemanticContainerFlow {
        flow_id,
        owner,
        owner_kind,
        semantic_kind,
        parent_flow_id,
        parent_position,
        depth,
        terminal,
        items: Vec::new(),
    });
    let index = usize::try_from(flow_id.get())
        .map_err(|_| StagingSemanticContainerLayoutError::FlowLimit)?;
    let mut items = Vec::new();
    items
        .try_reserve_exact(blocks.len())
        .map_err(|_| StagingSemanticContainerLayoutError::AllocationFailure)?;
    for (position, block) in blocks.iter().enumerate() {
        let position =
            u32::try_from(position).map_err(|_| StagingSemanticContainerLayoutError::FlowLimit)?;
        let mut child_flow_ids = Vec::new();
        match block {
            StagingM4Block::SemanticContainer {
                common,
                semantic_kind,
                blocks,
            } => child_flow_ids.push(build_flow(
                limits,
                common.node_id,
                StagingSemanticContainerFlowOwnerKind::SemanticContainer,
                Some(*semantic_kind),
                Some(flow_id),
                Some(position),
                depth
                    .checked_add(1)
                    .ok_or(StagingSemanticContainerLayoutError::FlowDepthLimit)?,
                blocks,
                flows,
            )?),
            StagingM4Block::List {
                items: list_items, ..
            } => {
                for item in list_items {
                    child_flow_ids.push(build_flow(
                        limits,
                        item.node_id,
                        StagingSemanticContainerFlowOwnerKind::ListItem,
                        None,
                        Some(flow_id),
                        Some(position),
                        next_flow_depth(depth)?,
                        &item.blocks,
                        flows,
                    )?);
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    child_flow_ids.push(build_flow(
                        limits,
                        cell.node_id,
                        StagingSemanticContainerFlowOwnerKind::TableCell,
                        None,
                        Some(flow_id),
                        Some(position),
                        next_flow_depth(depth)?,
                        &cell.blocks,
                        flows,
                    )?);
                }
            }
            StagingM4Block::Figure {
                common, caption, ..
            }
            | StagingM4Block::VectorFigure {
                common, caption, ..
            } => child_flow_ids.push(build_flow(
                limits,
                common.node_id,
                StagingSemanticContainerFlowOwnerKind::FigureCaption,
                None,
                Some(flow_id),
                Some(position),
                next_flow_depth(depth)?,
                caption,
                flows,
            )?),
            StagingM4Block::Paragraph { .. }
            | StagingM4Block::Heading { .. }
            | StagingM4Block::PageBreak { .. }
            | StagingM4Block::DisplayMath { .. }
            | StagingM4Block::MathVectorBlock { .. } => {}
        }
        items.push(StagingSemanticContainerFlowItem {
            position,
            owner: block.node_id(),
            kind: block_item_kind(block)?,
            child_flow_ids,
        });
    }
    flows[index].items = items;
    Ok(flow_id)
}

fn next_flow_depth(depth: u32) -> Result<u32, StagingSemanticContainerLayoutError> {
    depth
        .checked_add(1)
        .ok_or(StagingSemanticContainerLayoutError::FlowDepthLimit)
}

fn block_item_kind(
    block: &StagingM4Block,
) -> Result<StagingSemanticContainerFlowItemKind, StagingSemanticContainerLayoutError> {
    Ok(match block {
        StagingM4Block::Paragraph { .. } => StagingSemanticContainerFlowItemKind::Paragraph,
        StagingM4Block::Heading { .. } => StagingSemanticContainerFlowItemKind::Heading,
        StagingM4Block::List { .. } => StagingSemanticContainerFlowItemKind::List,
        StagingM4Block::Table { .. } => StagingSemanticContainerFlowItemKind::Table,
        StagingM4Block::Figure { .. } | StagingM4Block::VectorFigure { .. } => {
            StagingSemanticContainerFlowItemKind::Figure
        }
        StagingM4Block::PageBreak { .. } => StagingSemanticContainerFlowItemKind::PageBreak,
        StagingM4Block::DisplayMath { .. } | StagingM4Block::MathVectorBlock { .. } => {
            StagingSemanticContainerFlowItemKind::DisplayMath
        }
        StagingM4Block::SemanticContainer { .. } => {
            StagingSemanticContainerFlowItemKind::SemanticContainer
        }
    })
}

fn verify_registry(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSemanticContainerProfileView,
    registry: &StagingSemanticContainerFlowRegistry,
) -> Result<(), StagingSemanticContainerLayoutError> {
    verify_registry_binding(
        package,
        profile.profile_fingerprint(),
        profile.limits(),
        registry,
    )
}

fn verify_registry_binding(
    package: &ValidatedStagingSemanticPackage,
    profile_fingerprint: [u8; 32],
    limits: &typaxis_core::ValidatedResourceLimits,
    registry: &StagingSemanticContainerFlowRegistry,
) -> Result<(), StagingSemanticContainerLayoutError> {
    let expected = build_package_flows(package, limits)?;
    if registry.receipt.package_fingerprint != package.semantic_fingerprint()
        || registry.receipt.profile_fingerprint != profile_fingerprint
        || usize::try_from(registry.receipt.flow_count) != Ok(registry.flows.len())
        || registry
            .flows
            .iter()
            .enumerate()
            .any(|(index, flow)| usize::try_from(flow.flow_id.get()) != Ok(index))
        || registry.flows != expected
    {
        return Err(StagingSemanticContainerLayoutError::ReceiptMismatch);
    }
    let canonical = encode_registry(
        &registry.flows,
        package.semantic_fingerprint(),
        profile_fingerprint,
    );
    if canonical != registry.receipt.canonical_jcs
        || sha256(canonical.as_bytes()) != registry.receipt.fingerprint
    {
        return Err(StagingSemanticContainerLayoutError::ReceiptMismatch);
    }
    let mut parent_references = vec![0u32; registry.flows.len()];
    for flow in &registry.flows {
        for (position, item) in flow.items.iter().enumerate() {
            if usize::try_from(item.position) != Ok(position)
                || item.child_flow_ids.iter().any(|child| {
                    let Some(reference_count) = usize::try_from(child.get())
                        .ok()
                        .and_then(|index| parent_references.get_mut(index))
                    else {
                        return true;
                    };
                    let Some(next_count) = reference_count.checked_add(1) else {
                        return true;
                    };
                    *reference_count = next_count;
                    registry.flow(*child).map_or(true, |child_flow| {
                        child_flow.parent_flow_id != Some(flow.flow_id)
                            || child_flow.parent_position != Some(item.position)
                            || flow.depth.checked_add(1) != Some(child_flow.depth)
                    })
                })
            {
                return Err(StagingSemanticContainerLayoutError::ReceiptMismatch);
            }
        }
    }
    if registry.flows.iter().enumerate().any(|(index, flow)| {
        if index == usize::try_from(FlowId::DOCUMENT_BODY.get()).unwrap_or(usize::MAX) {
            flow.parent_flow_id.is_some()
                || flow.parent_position.is_some()
                || parent_references[index] != 0
        } else {
            flow.parent_flow_id.is_none()
                || flow.parent_position.is_none()
                || parent_references[index] != 1
        }
    }) {
        return Err(StagingSemanticContainerLayoutError::ReceiptMismatch);
    }
    Ok(())
}

fn fragments_are_closed(
    registry: &StagingSemanticContainerFlowRegistry,
    fragments: &[StagingSemanticContainerFragment],
    package: &ValidatedStagingSemanticPackage,
) -> bool {
    let mut offset = 0usize;
    for flow in registry
        .flows
        .iter()
        .filter(|flow| flow.owner_kind == StagingSemanticContainerFlowOwnerKind::SemanticContainer)
    {
        let mut cursor = 0u32;
        let mut fragment_index = 0u32;
        while offset < fragments.len() && fragments[offset].owner == flow.owner {
            let fragment = &fragments[offset];
            let Some(kind) = flow.semantic_kind else {
                return false;
            };
            let Some(style) = package.computed_style(flow.owner) else {
                return false;
            };
            if fragment.semantic_kind != kind
                || fragment.flow_id != flow.flow_id
                || fragment.parent_flow_id != flow.parent_flow_id.unwrap_or(FlowId::DOCUMENT_BODY)
                || fragment.parent_position != flow.parent_position.unwrap_or(0)
                || fragment.fragment_index != fragment_index
                || usize::try_from(fragment.page_index) != Ok(offset)
                || fragment.frame_index != 0
                || fragment.before_cursor != cursor
                || fragment.after_cursor <= cursor
                || fragment.after_cursor > flow.terminal()
                || fragment.first != (fragment_index == 0)
                || fragment.last != (fragment.after_cursor == flow.terminal())
                || &fragment.computed_style != style
                || fragment.style_fingerprint != style_fingerprint(flow.owner, style)
                || fragment.fingerprint != sha256(encode_fragment(fragment).as_bytes())
            {
                return false;
            }
            let start = usize::try_from(fragment.before_cursor).ok();
            let end = usize::try_from(fragment.after_cursor).ok();
            if start.zip(end).map_or(true, |(start, end)| {
                fragment.child_owners
                    != flow.items[start..end]
                        .iter()
                        .map(StagingSemanticContainerFlowItem::owner)
                        .collect::<Vec<_>>()
            }) {
                return false;
            }
            cursor = fragment.after_cursor;
            let Some(next_fragment_index) = fragment_index.checked_add(1) else {
                return false;
            };
            fragment_index = next_fragment_index;
            let Some(next_offset) = offset.checked_add(1) else {
                return false;
            };
            offset = next_offset;
        }
        if cursor != flow.terminal() {
            return false;
        }
    }
    offset == fragments.len()
}

fn ensure_style_kind(
    kind: SemanticContainerKind,
    style: &SemanticContainerComputedStyle,
    owner: NodeId,
) -> Result<(), StagingSemanticContainerLayoutError> {
    let expected = match kind {
        SemanticContainerKind::Result => SemanticContainerStyleKind::Result,
        SemanticContainerKind::Proof => SemanticContainerStyleKind::Proof,
        SemanticContainerKind::Exercise => SemanticContainerStyleKind::Exercise,
    };
    if style.semantic_kind() != expected {
        return Err(StagingSemanticContainerLayoutError::MissingStyle(owner));
    }
    Ok(())
}

fn find_block(document: &StagingM4Document, owner: NodeId) -> Option<&StagingM4Block> {
    fn find(blocks: &[StagingM4Block], owner: NodeId) -> Option<&StagingM4Block> {
        for block in blocks {
            if block.node_id() == owner {
                return Some(block);
            }
            let found = match block {
                StagingM4Block::SemanticContainer { blocks, .. } => find(blocks, owner),
                StagingM4Block::List { items, .. } => {
                    items.iter().find_map(|item| find(&item.blocks, owner))
                }
                StagingM4Block::Table { head, body, .. } => head
                    .iter()
                    .chain(body)
                    .flat_map(|row| &row.cells)
                    .find_map(|cell| find(&cell.blocks, owner)),
                StagingM4Block::Figure { caption, .. } => find(caption, owner),
                StagingM4Block::Paragraph { .. }
                | StagingM4Block::Heading { .. }
                | StagingM4Block::PageBreak { .. }
                | StagingM4Block::DisplayMath { .. }
                | StagingM4Block::VectorFigure { .. }
                | StagingM4Block::MathVectorBlock { .. } => None,
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }
    find(&document.blocks, owner).or_else(|| {
        document
            .footnotes
            .iter()
            .find_map(|footnote| find(&footnote.blocks, owner))
    })
}

fn style_fingerprint(owner: NodeId, style: &SemanticContainerComputedStyle) -> [u8; 32] {
    let block = style.block_style();
    let mut canonical = String::from("{\"end_indent\":");
    canonical.push_str(&block.end_indent().get().raw().to_string());
    canonical.push_str(",\"font_families\":");
    match style.inheritance_style().font_families() {
        Some(families) => {
            canonical.push('[');
            for (index, family) in families.iter().enumerate() {
                if index > 0 {
                    canonical.push(',');
                }
                push_jcs_string(&mut canonical, family);
            }
            canonical.push(']');
        }
        None => canonical.push_str("null"),
    }
    canonical.push_str(",\"font_size\":");
    push_optional_length(&mut canonical, style.inheritance_style().font_size());
    canonical.push_str(",\"keep_with_next\":");
    canonical.push_str(if block.keep_with_next() {
        "true"
    } else {
        "false"
    });
    canonical.push_str(",\"kind\":");
    push_jcs_string(&mut canonical, style.semantic_kind().as_str());
    canonical.push_str(",\"line_height\":");
    push_optional_length(&mut canonical, style.inheritance_style().line_height());
    canonical.push_str(",\"node_id\":");
    canonical.push_str(&owner.get().to_string());
    canonical.push_str(",\"page\":");
    match style.page_name() {
        Some(page) => push_jcs_string(&mut canonical, page.as_str()),
        None => canonical.push_str("null"),
    }
    canonical.push_str(",\"space_after\":");
    canonical.push_str(&block.space_after().get().raw().to_string());
    canonical.push_str(",\"space_before\":");
    canonical.push_str(&block.space_before().get().raw().to_string());
    canonical.push_str(",\"start_indent\":");
    canonical.push_str(&block.start_indent().get().raw().to_string());
    canonical.push_str(",\"text_align\":");
    push_jcs_string(&mut canonical, block.text_align().as_str());
    canonical.push('}');
    sha256(canonical.as_bytes())
}

fn push_optional_length(output: &mut String, value: Option<typaxis_core::PositiveLength>) {
    match value {
        Some(value) => output.push_str(&value.get().raw().to_string()),
        None => output.push_str("null"),
    }
}

fn encode_registry(
    flows: &[StagingSemanticContainerFlow],
    package: [u8; 32],
    profile: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, FLOW_REGISTRY_ALGORITHM);
    output.push_str(",\"flows\":[");
    for (index, flow) in flows.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"depth\":");
        output.push_str(&flow.depth.to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&flow.flow_id.get().to_string());
        output.push_str(",\"items\":[");
        for (item_index, item) in flow.items.iter().enumerate() {
            if item_index > 0 {
                output.push(',');
            }
            output.push_str("{\"child_flow_ids\":[");
            for (child_index, child) in item.child_flow_ids.iter().enumerate() {
                if child_index > 0 {
                    output.push(',');
                }
                output.push_str(&child.get().to_string());
            }
            output.push_str("],\"kind\":");
            push_jcs_string(&mut output, item.kind.as_str());
            output.push_str(",\"owner\":");
            output.push_str(&item.owner.get().to_string());
            output.push_str(",\"position\":");
            output.push_str(&item.position.to_string());
            output.push('}');
        }
        output.push_str("],\"owner\":");
        output.push_str(&flow.owner.get().to_string());
        output.push_str(",\"owner_kind\":");
        push_jcs_string(&mut output, flow.owner_kind.as_str());
        output.push_str(",\"parent_flow_id\":");
        push_optional_u32(&mut output, flow.parent_flow_id.map(FlowId::get));
        output.push_str(",\"parent_position\":");
        push_optional_u32(&mut output, flow.parent_position);
        output.push_str(",\"semantic_kind\":");
        match flow.semantic_kind {
            Some(kind) => push_jcs_string(&mut output, kind.as_str()),
            None => output.push_str("null"),
        };
        output.push_str(",\"terminal\":");
        output.push_str(&flow.terminal().to_string());
        output.push('}');
    }
    output.push_str("],\"package_fingerprint\":");
    push_hash(&mut output, package);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, profile);
    output.push('}');
    output
}

fn encode_fragment(fragment: &StagingSemanticContainerFragment) -> String {
    let mut output = String::from("{\"after_cursor\":");
    output.push_str(&fragment.after_cursor.to_string());
    output.push_str(",\"before_cursor\":");
    output.push_str(&fragment.before_cursor.to_string());
    output.push_str(",\"child_owners\":[");
    for (index, owner) in fragment.child_owners.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&owner.get().to_string());
    }
    output.push_str("],\"first\":");
    output.push_str(if fragment.first { "true" } else { "false" });
    output.push_str(",\"flow_id\":");
    output.push_str(&fragment.flow_id.get().to_string());
    output.push_str(",\"fragment_index\":");
    output.push_str(&fragment.fragment_index.to_string());
    output.push_str(",\"frame_index\":");
    output.push_str(&fragment.frame_index.to_string());
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, fragment.semantic_kind.as_str());
    output.push_str(",\"last\":");
    output.push_str(if fragment.last { "true" } else { "false" });
    output.push_str(",\"owner\":");
    output.push_str(&fragment.owner.get().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&fragment.page_index.to_string());
    output.push_str(",\"parent_flow_id\":");
    output.push_str(&fragment.parent_flow_id.get().to_string());
    output.push_str(",\"parent_position\":");
    output.push_str(&fragment.parent_position.to_string());
    output.push_str(",\"source_span\":{\"end_byte\":");
    output.push_str(&fragment.source_span.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&fragment.source_span.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&fragment.source_span.start_byte().get().to_string());
    output.push('}');
    output.push_str(",\"style_fingerprint\":");
    push_hash(&mut output, fragment.style_fingerprint);
    output.push('}');
    output
}

fn encode_selected_layout(
    registry: &StagingSemanticContainerFlowRegistry,
    fragments: &[StagingSemanticContainerFragment],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, SELECTED_LAYOUT_ALGORITHM);
    output.push_str(",\"flow_registry_fingerprint\":");
    push_hash(&mut output, registry.receipt.fingerprint);
    output.push_str(",\"fragments\":[");
    for (index, fragment) in fragments.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&encode_fragment(fragment));
    }
    output.push_str("],\"package_fingerprint\":");
    push_hash(&mut output, registry.receipt.package_fingerprint);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, registry.receipt.profile_fingerprint);
    output.push('}');
    output
}

fn push_optional_u32(output: &mut String, value: Option<u32>) {
    match value {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
}

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0xf)]));
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
        StagingSemanticDocumentPackageEncoder, WireStagingM4Block, WireStagingM4Footnote,
        WireStagingM4Inline, WireStagingSourceSpan, WireStagingStyleValue,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    const FIXTURE: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"));

    fn test_profile(
        package: &ValidatedStagingSemanticPackage,
        limits: &ValidatedResourceLimits,
    ) -> StagingSemanticContainerProfileView {
        StagingSemanticContainerProfileView::new(package, limits).unwrap()
    }

    fn selected(capacity: u32) -> StagingSemanticContainerSelectedLayout {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        layout_staging_semantic_containers(&package, &profile, capacity).unwrap()
    }

    #[test]
    fn semantic_container_flow_ids_and_page_split_are_canonical() {
        let selected = selected(2);
        let semantic: Vec<_> = selected
            .registry
            .flows
            .iter()
            .filter(|flow| {
                flow.owner_kind == StagingSemanticContainerFlowOwnerKind::SemanticContainer
            })
            .collect();
        assert_eq!(
            semantic
                .iter()
                .map(|flow| flow.flow_id.get())
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(
            selected
                .fragments
                .iter()
                .filter(|fragment| fragment.owner == NodeId::new(1))
                .count(),
            2
        );
        assert!(selected
            .fragments
            .iter()
            .all(|fragment| fragment.after_cursor > fragment.before_cursor));
    }

    #[test]
    fn semantic_container_zero_capacity_is_rejected() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        assert_eq!(
            layout_staging_semantic_containers(&package, &profile, 0),
            Err(StagingSemanticContainerLayoutError::ZeroFragmentCapacity)
        );
    }

    #[test]
    fn semantic_container_selected_layout_detects_stale_cursor_tamper() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        let mut selected = layout_staging_semantic_containers(&package, &profile, 2).unwrap();
        selected.fragments[0].after_cursor = selected.fragments[0].before_cursor;
        assert_eq!(
            selected.verify(&package, &profile),
            Err(StagingSemanticContainerLayoutError::ReceiptMismatch)
        );
    }

    #[test]
    fn semantic_container_registry_is_rederived_after_self_consistent_tamper() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        let mut selected = layout_staging_semantic_containers(&package, &profile, 2).unwrap();
        selected.registry.flows[0].owner = NodeId::new(99);
        let registry_jcs = encode_registry(
            &selected.registry.flows,
            package.semantic_fingerprint(),
            profile.profile_fingerprint(),
        );
        selected.registry.receipt.fingerprint = sha256(registry_jcs.as_bytes());
        selected.registry.receipt.canonical_jcs = registry_jcs;
        selected.receipt.flow_registry_fingerprint = selected.registry.receipt.fingerprint;
        let selected_jcs = encode_selected_layout(&selected.registry, &selected.fragments);
        selected.receipt.fingerprint = sha256(selected_jcs.as_bytes());
        selected.receipt.canonical_jcs = selected_jcs;
        assert_eq!(
            selected.verify(&package, &profile),
            Err(StagingSemanticContainerLayoutError::ReceiptMismatch)
        );
    }

    #[test]
    fn footnote_definition_flow_has_one_canonical_parent_edge() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut document = wire.document().clone();
        let span = WireStagingSourceSpan {
            source_id: 0,
            start_byte: 19,
            end_byte: 19,
        };
        document.footnotes.push(WireStagingM4Footnote {
            footnote_id: "note".to_owned(),
            node_id: 10,
            span,
            language: None,
            blocks: vec![WireStagingM4Block::Paragraph {
                node_id: 11,
                span,
                classes: Vec::new(),
                language: None,
                children: vec![WireStagingM4Inline::HardBreak { node_id: 12, span }],
            }],
        });
        let resources = wire.resources().clone();
        wire.replace_typed_regions(document, resources);
        let bytes = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(bytes.as_bytes(), &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        let selected = layout_staging_semantic_containers(&package, &profile, 2).unwrap();
        let root = selected.registry().flow(FlowId::DOCUMENT_BODY).unwrap();
        let footnote_item = root.items().last().unwrap();
        assert_eq!(
            footnote_item.kind(),
            StagingSemanticContainerFlowItemKind::FootnoteDefinition
        );
        assert_eq!(footnote_item.owner(), NodeId::new(10));
        assert_eq!(footnote_item.child_flow_ids().len(), 1);
        let footnote_flow = selected
            .registry()
            .flow(footnote_item.child_flow_ids()[0])
            .unwrap();
        assert_eq!(footnote_flow.parent_flow_id(), Some(FlowId::DOCUMENT_BODY));
        assert_eq!(
            footnote_flow.parent_position(),
            Some(footnote_item.position())
        );
        selected.verify(&package, &profile).unwrap();
    }

    #[test]
    fn semantic_container_indents_fail_during_layout_before_display() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let mut wire = decoded.into_wire();
        let mut sheet = wire.style_sheet().clone();
        let start_indent = sheet.rules[0]
            .declarations
            .iter_mut()
            .find(|declaration| declaration.name == "start_indent")
            .unwrap();
        start_indent.value = WireStagingStyleValue::Length { value: 96 };
        wire.replace_style_sheet(sheet);
        let bytes = StagingSemanticDocumentPackageEncoder::new()
            .encode(&wire)
            .unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(bytes.as_bytes(), &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        assert_eq!(
            layout_staging_semantic_containers(&package, &profile, 2),
            Err(StagingSemanticContainerLayoutError::InvalidGeometry(
                NodeId::new(1)
            ))
        );
    }

    #[test]
    fn semantic_container_layout_enforces_receipted_fragment_and_page_limits() {
        fn run_with_limits(
            update: impl FnOnce(&mut ResourceLimits),
        ) -> Result<StagingSemanticContainerSelectedLayout, StagingSemanticContainerLayoutError>
        {
            let mut raw_limits = ResourceLimits::default();
            update(&mut raw_limits);
            let limits = ValidatedResourceLimits::new(raw_limits).unwrap();
            let decoded = StagingSemanticDocumentPackageDecoder::new()
                .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
                .unwrap();
            let package = StagingSemanticPackageParser::new()
                .parse(decoded, &limits)
                .unwrap();
            let profile = test_profile(&package, &limits);
            layout_staging_semantic_containers(&package, &profile, 1)
        }

        assert_eq!(
            run_with_limits(|limits| limits.max_fragments = 3),
            Err(StagingSemanticContainerLayoutError::FragmentLimit)
        );
        assert_eq!(
            run_with_limits(|limits| limits.max_pages = 3),
            Err(StagingSemanticContainerLayoutError::PageLimit)
        );
        assert!(run_with_limits(|limits| {
            limits.max_fragments = 5;
            limits.max_pages = 5;
        })
        .is_ok());
    }
}
