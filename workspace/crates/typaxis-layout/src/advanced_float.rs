use typaxis_core::{
    push_jcs_string, sha256, DocumentFingerprint, Length, MasterId, NodeId, NonNegativeLength,
    PositiveLength, Rect, StyleFingerprint, ValidatedResourceLimits,
};
use typaxis_document::{Block, FigurePlacement, Inline, ListItem};
use typaxis_layout_contract::FlowId;
use typaxis_style::{MachineFigureWidth, PageMaster, StyleValue};
use typaxis_syntax::ValidatedStagingAdvancedPackage;

pub const FLOAT_FLOW_REGISTRY_ALGORITHM: &str = "typaxis.advanced-flow-registry/1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingFloatFlowOwnerKind {
    DocumentBody,
    ListItem,
    BlockFigureCaption,
    Float,
    FloatCaption,
    ColumnTemplate,
}

impl StagingFloatFlowOwnerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DocumentBody => "document_body",
            Self::ListItem => "list_item",
            Self::BlockFigureCaption => "block_figure_caption",
            Self::Float => "float",
            Self::FloatCaption => "float_caption",
            Self::ColumnTemplate => "column_template",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatFlowRecord {
    flow_id: FlowId,
    owner_node_id: Option<NodeId>,
    owner_kind: StagingFloatFlowOwnerKind,
    parent_flow_id: Option<FlowId>,
    source_flow_id: FlowId,
    depth: u32,
    terminal: u32,
    master_id: Option<MasterId>,
    column_index: Option<u32>,
}

impl StagingFloatFlowRecord {
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
    pub const fn owner_node_id(&self) -> Option<NodeId> {
        self.owner_node_id
    }
    pub const fn owner_kind(&self) -> StagingFloatFlowOwnerKind {
        self.owner_kind
    }
    pub const fn parent_flow_id(&self) -> Option<FlowId> {
        self.parent_flow_id
    }
    pub const fn source_flow_id(&self) -> FlowId {
        self.source_flow_id
    }
    pub const fn depth(&self) -> u32 {
        self.depth
    }
    pub const fn terminal(&self) -> u32 {
        self.terminal
    }
    pub const fn master_id(&self) -> Option<&MasterId> {
        self.master_id.as_ref()
    }
    pub const fn column_index(&self) -> Option<u32> {
        self.column_index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatColumnTemplate {
    master_id: MasterId,
    column_index: u32,
    frame_flow_id: FlowId,
    source_flow_id: FlowId,
    rect: Rect,
}

impl StagingFloatColumnTemplate {
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn column_index(&self) -> u32 {
        self.column_index
    }
    pub const fn frame_flow_id(&self) -> FlowId {
        self.frame_flow_id
    }
    pub const fn source_flow_id(&self) -> FlowId {
        self.source_flow_id
    }
    pub const fn rect(&self) -> Rect {
        self.rect
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingFloatBodyItemKind {
    Block,
    FloatAnchor,
}

impl StagingFloatBodyItemKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::FloatAnchor => "float_anchor",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFloatBodyItem {
    node_id: NodeId,
    before_position: u32,
    after_position: u32,
    kind: StagingFloatBodyItemKind,
    block_extent: NonNegativeLength,
    keep_with_next: bool,
    forced_page_break: bool,
    float_flow_id: Option<FlowId>,
    caption_flow_id: Option<FlowId>,
    image_width: Option<PositiveLength>,
    float_extent: Option<PositiveLength>,
}

impl StagingFloatBodyItem {
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn before_position(&self) -> u32 {
        self.before_position
    }
    pub const fn after_position(&self) -> u32 {
        self.after_position
    }
    pub const fn kind(&self) -> StagingFloatBodyItemKind {
        self.kind
    }
    pub const fn block_extent(&self) -> NonNegativeLength {
        self.block_extent
    }
    pub const fn keep_with_next(&self) -> bool {
        self.keep_with_next
    }
    pub const fn forced_page_break(&self) -> bool {
        self.forced_page_break
    }
    pub const fn float_flow_id(&self) -> Option<FlowId> {
        self.float_flow_id
    }
    pub const fn caption_flow_id(&self) -> Option<FlowId> {
        self.caption_flow_id
    }
    pub const fn image_width(&self) -> Option<PositiveLength> {
        self.image_width
    }
    pub const fn float_extent(&self) -> Option<PositiveLength> {
        self.float_extent
    }
}

#[derive(Debug)]
pub struct StagingFloatFlowRegistryReceipt {
    package: DocumentFingerprint,
    style: StyleFingerprint,
    profile_receipt_sha256: [u8; 32],
    fingerprint: [u8; 32],
    flow_count: u32,
    body_terminal: u32,
    canonical_jcs: String,
}

impl StagingFloatFlowRegistryReceipt {
    pub const fn package_fingerprint(&self) -> DocumentFingerprint {
        self.package
    }
    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }
    pub const fn profile_receipt_sha256(&self) -> [u8; 32] {
        self.profile_receipt_sha256
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub const fn flow_count(&self) -> u32 {
        self.flow_count
    }
    pub const fn body_terminal(&self) -> u32 {
        self.body_terminal
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Debug)]
pub struct StagingFloatLayout {
    page_master: PageMaster,
    advanced_page_master: typaxis_document::AdvancedPageMaster,
    flows: Vec<StagingFloatFlowRecord>,
    columns: Vec<StagingFloatColumnTemplate>,
    body_items: Vec<StagingFloatBodyItem>,
    receipt: StagingFloatFlowRegistryReceipt,
}

impl StagingFloatLayout {
    pub const fn page_master(&self) -> &PageMaster {
        &self.page_master
    }
    pub const fn advanced_page_master(&self) -> &typaxis_document::AdvancedPageMaster {
        &self.advanced_page_master
    }
    pub fn flows(&self) -> &[StagingFloatFlowRecord] {
        &self.flows
    }
    pub fn flow(&self, flow_id: FlowId) -> Option<&StagingFloatFlowRecord> {
        self.flows
            .get(usize::try_from(flow_id.get()).ok()?)
            .filter(|flow| flow.flow_id == flow_id)
    }
    pub fn columns(&self) -> &[StagingFloatColumnTemplate] {
        &self.columns
    }
    pub fn body_items(&self) -> &[StagingFloatBodyItem] {
        &self.body_items
    }
    pub const fn receipt(&self) -> &StagingFloatFlowRegistryReceipt {
        &self.receipt
    }

    pub fn verify_receipt(
        &self,
        package: &ValidatedStagingAdvancedPackage,
        profile_receipt_sha256: [u8; 32],
        limits: &ValidatedResourceLimits,
    ) -> Result<(), StagingFloatLayoutError> {
        let expected_master = package
            .package()
            .package()
            .page_masters
            .masters
            .first()
            .ok_or(StagingFloatLayoutError::ReceiptMismatch)?;
        let expected_advanced = package
            .page_masters()
            .masters
            .first()
            .ok_or(StagingFloatLayoutError::ReceiptMismatch)?;
        let derived = derive_parts(package, limits)?;
        let canonical = encode_registry(
            package.raw_sha256(),
            profile_receipt_sha256,
            &self.flows,
            &self.columns,
            &self.body_items,
        );
        let epoch = package.package().epoch_identity();
        if self.page_master != *expected_master
            || self.advanced_page_master != *expected_advanced
            || self.flows != derived.flows
            || self.columns != derived.columns
            || self.body_items != derived.body_items
            || self.receipt.package != epoch.document()
            || self.receipt.style != epoch.style()
            || self.receipt.profile_receipt_sha256 != profile_receipt_sha256
            || usize::try_from(self.receipt.flow_count) != Ok(self.flows.len())
            || usize::try_from(self.receipt.body_terminal) != Ok(self.body_items.len())
            || self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingFloatLayoutError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingFloatLayoutError {
    UnsupportedContent(NodeId),
    InvalidStyle(NodeId),
    InvalidGeometry,
    AstNodeLimit,
    FlowDepthLimit,
    ReceiptMismatch,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingFloatLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContent(node) => write!(
                formatter,
                "L5100: unsupported float layout content at node {}",
                node.get()
            ),
            Self::InvalidStyle(node) => {
                write!(
                    formatter,
                    "L5101: invalid float style at node {}",
                    node.get()
                )
            }
            Self::InvalidGeometry => formatter.write_str("L5101: invalid float geometry"),
            Self::AstNodeLimit => formatter.write_str("P1120: float flow limit exceeded"),
            Self::FlowDepthLimit => formatter.write_str("P1121: float flow depth exceeded"),
            Self::ReceiptMismatch => formatter.write_str("I9190: float flow receipt mismatch"),
            Self::ArithmeticOverflow => formatter.write_str("L5101: float layout overflow"),
            Self::AllocationFailure => formatter.write_str("L5110: float allocation failure"),
        }
    }
}

impl std::error::Error for StagingFloatLayoutError {}

pub fn layout_staging_float(
    package: &ValidatedStagingAdvancedPackage,
    verified_profile_receipt_sha256: [u8; 32],
    limits: &ValidatedResourceLimits,
) -> Result<StagingFloatLayout, StagingFloatLayoutError> {
    let page_master = package
        .package()
        .package()
        .page_masters
        .masters
        .first()
        .ok_or(StagingFloatLayoutError::InvalidGeometry)?
        .clone();
    let advanced_page_master = package
        .page_masters()
        .masters
        .first()
        .ok_or(StagingFloatLayoutError::InvalidGeometry)?
        .clone();
    let derived = derive_parts(package, limits)?;
    let DerivedFloatParts {
        flows,
        columns,
        body_items,
    } = derived;
    let canonical_jcs = encode_registry(
        package.raw_sha256(),
        verified_profile_receipt_sha256,
        &flows,
        &columns,
        &body_items,
    );
    let receipt = StagingFloatFlowRegistryReceipt {
        package: package.package().epoch_identity().document(),
        style: package.package().epoch_identity().style(),
        profile_receipt_sha256: verified_profile_receipt_sha256,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        flow_count: u32::try_from(flows.len())
            .map_err(|_| StagingFloatLayoutError::AstNodeLimit)?,
        body_terminal: u32::try_from(body_items.len())
            .map_err(|_| StagingFloatLayoutError::AstNodeLimit)?,
        canonical_jcs,
    };
    let layout = StagingFloatLayout {
        page_master,
        advanced_page_master,
        flows,
        columns,
        body_items,
        receipt,
    };
    layout.verify_receipt(package, verified_profile_receipt_sha256, limits)?;
    Ok(layout)
}

struct DerivedFloatParts {
    flows: Vec<StagingFloatFlowRecord>,
    columns: Vec<StagingFloatColumnTemplate>,
    body_items: Vec<StagingFloatBodyItem>,
}

fn derive_parts(
    package: &ValidatedStagingAdvancedPackage,
    limits: &ValidatedResourceLimits,
) -> Result<DerivedFloatParts, StagingFloatLayoutError> {
    let document = &package.package().package().document;
    let page_master = package
        .package()
        .package()
        .page_masters
        .masters
        .first()
        .ok_or(StagingFloatLayoutError::InvalidGeometry)?;
    let advanced = package
        .page_masters()
        .masters
        .first()
        .ok_or(StagingFloatLayoutError::InvalidGeometry)?;
    let semantic_nodes = package.package().document_nodes().node_count();
    let style_units = package
        .package()
        .package()
        .style_sheet
        .rules
        .iter()
        .try_fold(0usize, |total, rule| {
            rule.declarations
                .len()
                .checked_mul(2)
                .and_then(|count| total.checked_add(count))
        })
        .ok_or(StagingFloatLayoutError::AstNodeLimit)?;
    let column_units = advanced
        .column_layout
        .as_ref()
        .map_or(0usize, |layout| usize::from(layout.count.get()));
    let effective_ast_units = semantic_nodes
        .checked_add(style_units)
        .and_then(|total| total.checked_add(column_units))
        .ok_or(StagingFloatLayoutError::AstNodeLimit)?;
    if u64::try_from(effective_ast_units).map_err(|_| StagingFloatLayoutError::AstNodeLimit)?
        > limits.get().max_ast_nodes
    {
        return Err(StagingFloatLayoutError::AstNodeLimit);
    }
    let body_terminal =
        u32::try_from(document.blocks.len()).map_err(|_| StagingFloatLayoutError::AstNodeLimit)?;
    let mut flows = Vec::new();
    flows
        .try_reserve_exact(effective_ast_units)
        .map_err(|_| StagingFloatLayoutError::AllocationFailure)?;
    flows.push(StagingFloatFlowRecord {
        flow_id: FlowId::DOCUMENT_BODY,
        owner_node_id: Some(document.node_id),
        owner_kind: StagingFloatFlowOwnerKind::DocumentBody,
        parent_flow_id: None,
        source_flow_id: FlowId::DOCUMENT_BODY,
        depth: 1,
        terminal: body_terminal,
        master_id: None,
        column_index: None,
    });
    collect_descendant_flows(
        package,
        &document.blocks,
        FlowId::DOCUMENT_BODY,
        1,
        true,
        &mut flows,
        limits,
    )?;
    let mut columns =
        derive_column_templates(page_master, advanced, &mut flows, body_terminal, limits)?;
    columns.sort_by_key(|column| column.column_index);
    if columns.is_empty() {
        return Err(StagingFloatLayoutError::InvalidGeometry);
    }
    let min_column_width = columns
        .iter()
        .map(|column| column.rect.width())
        .min_by_key(|width| width.get().raw())
        .ok_or(StagingFloatLayoutError::InvalidGeometry)?;
    let mut body_items = Vec::new();
    body_items
        .try_reserve_exact(document.blocks.len())
        .map_err(|_| StagingFloatLayoutError::AllocationFailure)?;
    for (index, block) in document.blocks.iter().enumerate() {
        let before = u32::try_from(index).map_err(|_| StagingFloatLayoutError::AstNodeLimit)?;
        let after = before
            .checked_add(1)
            .ok_or(StagingFloatLayoutError::ArithmeticOverflow)?;
        let node_id = block_node_id(block);
        if matches!(block, Block::Figure { .. })
            && package.figure_placement(node_id) == Some(FigurePlacement::Float)
        {
            let (float_flow_id, caption_flow_id) = float_flows_for_node(&flows, node_id)?;
            let (image_width, float_extent) = measure_float(package, block, min_column_width)?;
            body_items.push(StagingFloatBodyItem {
                node_id,
                before_position: before,
                after_position: after,
                kind: StagingFloatBodyItemKind::FloatAnchor,
                block_extent: NonNegativeLength::ZERO,
                keep_with_next: false,
                forced_page_break: false,
                float_flow_id: Some(float_flow_id),
                caption_flow_id: Some(caption_flow_id),
                image_width: Some(image_width),
                float_extent: Some(float_extent),
            });
        } else {
            let (extent, keep_with_next) = measure_block(package, block, page_master.body.width())?;
            body_items.push(StagingFloatBodyItem {
                node_id,
                before_position: before,
                after_position: after,
                kind: StagingFloatBodyItemKind::Block,
                block_extent: extent,
                keep_with_next,
                forced_page_break: matches!(block, Block::PageBreak { .. }),
                float_flow_id: None,
                caption_flow_id: None,
                image_width: None,
                float_extent: None,
            });
        }
    }
    Ok(DerivedFloatParts {
        flows,
        columns,
        body_items,
    })
}

fn float_flows_for_node(
    flows: &[StagingFloatFlowRecord],
    node_id: NodeId,
) -> Result<(FlowId, FlowId), StagingFloatLayoutError> {
    let float = flows
        .iter()
        .find(|flow| {
            flow.owner_node_id == Some(node_id)
                && flow.owner_kind == StagingFloatFlowOwnerKind::Float
        })
        .ok_or(StagingFloatLayoutError::ReceiptMismatch)?;
    let caption = flows
        .iter()
        .find(|flow| {
            flow.owner_node_id == Some(node_id)
                && flow.owner_kind == StagingFloatFlowOwnerKind::FloatCaption
                && flow.parent_flow_id == Some(float.flow_id)
        })
        .ok_or(StagingFloatLayoutError::ReceiptMismatch)?;
    Ok((float.flow_id, caption.flow_id))
}

#[allow(clippy::too_many_arguments)]
fn collect_descendant_flows(
    package: &ValidatedStagingAdvancedPackage,
    blocks: &[Block],
    parent: FlowId,
    parent_depth: u32,
    direct_body: bool,
    flows: &mut Vec<StagingFloatFlowRecord>,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingFloatLayoutError> {
    for block in blocks {
        match block {
            Block::List { items, .. } => {
                for item in items {
                    collect_list_item_flow(package, item, parent, parent_depth, flows, limits)?;
                }
            }
            Block::Figure {
                node_id, caption, ..
            } => {
                let placement = package.figure_placement(*node_id);
                if placement == Some(FigurePlacement::Float) {
                    if !direct_body {
                        return Err(StagingFloatLayoutError::UnsupportedContent(*node_id));
                    }
                    let depth = checked_depth(parent_depth, limits)?;
                    let float_flow_id = allocate_flow(
                        flows,
                        Some(*node_id),
                        StagingFloatFlowOwnerKind::Float,
                        parent,
                        depth,
                        1,
                        None,
                        None,
                        limits,
                    )?;
                    let caption_depth = checked_depth(depth, limits)?;
                    let caption_flow_id = allocate_flow(
                        flows,
                        Some(*node_id),
                        StagingFloatFlowOwnerKind::FloatCaption,
                        float_flow_id,
                        caption_depth,
                        u32::try_from(caption.len())
                            .map_err(|_| StagingFloatLayoutError::AstNodeLimit)?,
                        None,
                        None,
                        limits,
                    )?;
                    collect_descendant_flows(
                        package,
                        caption,
                        caption_flow_id,
                        caption_depth,
                        false,
                        flows,
                        limits,
                    )?;
                } else {
                    let depth = checked_depth(parent_depth, limits)?;
                    let caption_flow_id = allocate_flow(
                        flows,
                        Some(*node_id),
                        StagingFloatFlowOwnerKind::BlockFigureCaption,
                        parent,
                        depth,
                        u32::try_from(caption.len())
                            .map_err(|_| StagingFloatLayoutError::AstNodeLimit)?,
                        None,
                        None,
                        limits,
                    )?;
                    collect_descendant_flows(
                        package,
                        caption,
                        caption_flow_id,
                        depth,
                        false,
                        flows,
                        limits,
                    )?;
                }
            }
            Block::Table { node_id, .. } => {
                return Err(StagingFloatLayoutError::UnsupportedContent(*node_id));
            }
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    Ok(())
}

fn collect_list_item_flow(
    package: &ValidatedStagingAdvancedPackage,
    item: &ListItem,
    parent: FlowId,
    parent_depth: u32,
    flows: &mut Vec<StagingFloatFlowRecord>,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingFloatLayoutError> {
    let depth = checked_depth(parent_depth, limits)?;
    let flow_id = allocate_flow(
        flows,
        Some(item.node_id),
        StagingFloatFlowOwnerKind::ListItem,
        parent,
        depth,
        u32::try_from(item.blocks.len()).map_err(|_| StagingFloatLayoutError::AstNodeLimit)?,
        None,
        None,
        limits,
    )?;
    collect_descendant_flows(package, &item.blocks, flow_id, depth, false, flows, limits)
}

fn checked_depth(
    parent_depth: u32,
    limits: &ValidatedResourceLimits,
) -> Result<u32, StagingFloatLayoutError> {
    parent_depth
        .checked_add(1)
        .filter(|depth| *depth <= limits.get().max_ast_nesting_depth)
        .ok_or(StagingFloatLayoutError::FlowDepthLimit)
}

fn derive_column_templates(
    master: &PageMaster,
    advanced: &typaxis_document::AdvancedPageMaster,
    flows: &mut Vec<StagingFloatFlowRecord>,
    body_terminal: u32,
    limits: &ValidatedResourceLimits,
) -> Result<Vec<StagingFloatColumnTemplate>, StagingFloatLayoutError> {
    let Some(layout) = &advanced.column_layout else {
        return Ok(vec![StagingFloatColumnTemplate {
            master_id: master.master_id.clone(),
            column_index: 0,
            frame_flow_id: FlowId::DOCUMENT_BODY,
            source_flow_id: FlowId::DOCUMENT_BODY,
            rect: master.body,
        }]);
    };
    if layout.balance != typaxis_document::ColumnBalance::None {
        return Err(StagingFloatLayoutError::InvalidGeometry);
    }
    let count = i64::from(layout.count.get());
    let total_gap = count
        .checked_sub(1)
        .and_then(|value| value.checked_mul(layout.gap.get().raw()))
        .ok_or(StagingFloatLayoutError::InvalidGeometry)?;
    let available = master
        .body
        .width()
        .get()
        .raw()
        .checked_sub(total_gap)
        .ok_or(StagingFloatLayoutError::InvalidGeometry)?;
    let base = available
        .checked_div(count)
        .filter(|value| *value > 0)
        .ok_or(StagingFloatLayoutError::InvalidGeometry)?;
    let residual = available
        .checked_rem(count)
        .ok_or(StagingFloatLayoutError::InvalidGeometry)?;
    let mut columns = Vec::new();
    columns
        .try_reserve_exact(usize::from(layout.count.get()))
        .map_err(|_| StagingFloatLayoutError::AllocationFailure)?;
    let mut x = master.body.x().raw();
    for index in 0..layout.count.get() {
        let column_index = u32::from(index);
        let width = if index + 1 == layout.count.get() {
            base.checked_add(residual)
                .ok_or(StagingFloatLayoutError::InvalidGeometry)?
        } else {
            base
        };
        let width = Length::from_raw(width)
            .and_then(PositiveLength::new)
            .ok_or(StagingFloatLayoutError::InvalidGeometry)?;
        let frame_flow_id = allocate_flow(
            flows,
            None,
            StagingFloatFlowOwnerKind::ColumnTemplate,
            FlowId::DOCUMENT_BODY,
            2,
            body_terminal,
            Some(master.master_id.clone()),
            Some(column_index),
            limits,
        )?;
        columns.push(StagingFloatColumnTemplate {
            master_id: master.master_id.clone(),
            column_index,
            frame_flow_id,
            source_flow_id: FlowId::DOCUMENT_BODY,
            rect: Rect::new(
                Length::from_raw(x).ok_or(StagingFloatLayoutError::InvalidGeometry)?,
                master.body.y(),
                width,
                master.body.height(),
            ),
        });
        x = x
            .checked_add(width.get().raw())
            .and_then(|value| {
                if index + 1 == layout.count.get() {
                    Some(value)
                } else {
                    value.checked_add(layout.gap.get().raw())
                }
            })
            .ok_or(StagingFloatLayoutError::InvalidGeometry)?;
    }
    let expected_right = master
        .body
        .x()
        .raw()
        .checked_add(master.body.width().get().raw())
        .ok_or(StagingFloatLayoutError::InvalidGeometry)?;
    if x != expected_right {
        return Err(StagingFloatLayoutError::InvalidGeometry);
    }
    Ok(columns)
}

#[allow(clippy::too_many_arguments)]
fn allocate_flow(
    flows: &mut Vec<StagingFloatFlowRecord>,
    owner_node_id: Option<NodeId>,
    owner_kind: StagingFloatFlowOwnerKind,
    parent_flow_id: FlowId,
    depth: u32,
    terminal: u32,
    master_id: Option<MasterId>,
    column_index: Option<u32>,
    limits: &ValidatedResourceLimits,
) -> Result<FlowId, StagingFloatLayoutError> {
    if depth > limits.get().max_ast_nesting_depth {
        return Err(StagingFloatLayoutError::FlowDepthLimit);
    }
    let flow_id =
        FlowId::new(u32::try_from(flows.len()).map_err(|_| StagingFloatLayoutError::AstNodeLimit)?);
    flows
        .try_reserve(1)
        .map_err(|_| StagingFloatLayoutError::AllocationFailure)?;
    flows.push(StagingFloatFlowRecord {
        flow_id,
        owner_node_id,
        owner_kind,
        parent_flow_id: Some(parent_flow_id),
        source_flow_id: flow_id,
        depth,
        terminal,
        master_id,
        column_index,
    });
    if owner_kind == StagingFloatFlowOwnerKind::ColumnTemplate {
        let last = flows
            .last_mut()
            .ok_or(StagingFloatLayoutError::AllocationFailure)?;
        last.source_flow_id = FlowId::DOCUMENT_BODY;
    }
    Ok(flow_id)
}

fn measure_float(
    package: &ValidatedStagingAdvancedPackage,
    block: &Block,
    auto_width: PositiveLength,
) -> Result<(PositiveLength, PositiveLength), StagingFloatLayoutError> {
    let node_id = block_node_id(block);
    let computed = package
        .package()
        .package()
        .style_sheet
        .cascade_basic_document(block_style_name(block), block.classes())
        .map_err(|_| StagingFloatLayoutError::InvalidStyle(node_id))?;
    let image_width = match computed
        .basic_figure_width()
        .map_err(|_| StagingFloatLayoutError::InvalidStyle(node_id))?
    {
        MachineFigureWidth::Auto => auto_width,
        MachineFigureWidth::Length(value) => value,
    };
    let (extent, _) = measure_block(package, block, auto_width)?;
    let extent =
        PositiveLength::new(extent.get()).ok_or(StagingFloatLayoutError::InvalidStyle(node_id))?;
    Ok((image_width, extent))
}

fn measure_block(
    package: &ValidatedStagingAdvancedPackage,
    block: &Block,
    auto_figure_width: PositiveLength,
) -> Result<(NonNegativeLength, bool), StagingFloatLayoutError> {
    if matches!(block, Block::PageBreak { .. }) {
        return Ok((NonNegativeLength::ZERO, false));
    }
    let node_id = block_node_id(block);
    let computed = package
        .package()
        .package()
        .style_sheet
        .cascade_basic_document(block_style_name(block), block.classes())
        .map_err(|_| StagingFloatLayoutError::InvalidStyle(node_id))?;
    let nonnegative = |name: &str| match computed.properties().get(name) {
        Some(StyleValue::Length(value)) => NonNegativeLength::new(*value),
        None => Some(NonNegativeLength::ZERO),
        Some(_) => None,
    };
    let before = nonnegative("space_before")
        .ok_or(StagingFloatLayoutError::InvalidStyle(node_id))?
        .get()
        .raw();
    let after = nonnegative("space_after")
        .ok_or(StagingFloatLayoutError::InvalidStyle(node_id))?
        .get()
        .raw();
    let core = match block {
        Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
            let line_height = computed
                .properties()
                .get("line_height")
                .and_then(|value| match value {
                    StyleValue::Length(value) => PositiveLength::new(*value),
                    _ => None,
                })
                .ok_or(StagingFloatLayoutError::InvalidStyle(node_id))?;
            line_height
                .get()
                .raw()
                .checked_mul(inline_line_count(children)?)
                .ok_or(StagingFloatLayoutError::ArithmeticOverflow)?
        }
        Block::List { items, .. } => {
            let mut extent = 0i64;
            for nested in items.iter().flat_map(|item| &item.blocks) {
                extent = extent
                    .checked_add(
                        measure_block(package, nested, auto_figure_width)?
                            .0
                            .get()
                            .raw(),
                    )
                    .ok_or(StagingFloatLayoutError::ArithmeticOverflow)?;
            }
            if extent == 0 {
                computed
                    .properties()
                    .get("line_height")
                    .and_then(|value| match value {
                        StyleValue::Length(value) => PositiveLength::new(*value),
                        _ => None,
                    })
                    .ok_or(StagingFloatLayoutError::InvalidStyle(node_id))?
                    .get()
                    .raw()
            } else {
                extent
            }
        }
        Block::Figure { caption, .. } => {
            let image_extent = match computed
                .basic_figure_width()
                .map_err(|_| StagingFloatLayoutError::InvalidStyle(node_id))?
            {
                MachineFigureWidth::Auto => auto_figure_width.get().raw(),
                MachineFigureWidth::Length(value) => value.get().raw(),
            };
            caption.iter().try_fold(image_extent, |extent, nested| {
                extent
                    .checked_add(
                        measure_block(package, nested, auto_figure_width)?
                            .0
                            .get()
                            .raw(),
                    )
                    .ok_or(StagingFloatLayoutError::ArithmeticOverflow)
            })?
        }
        Block::Table { .. } => return Err(StagingFloatLayoutError::UnsupportedContent(node_id)),
        Block::PageBreak { .. } => unreachable!("page break returned above"),
    };
    let raw = before
        .checked_add(core)
        .and_then(|value| value.checked_add(after))
        .ok_or(StagingFloatLayoutError::ArithmeticOverflow)?;
    let extent = Length::from_raw(raw)
        .and_then(NonNegativeLength::new)
        .ok_or(StagingFloatLayoutError::ArithmeticOverflow)?;
    let keep_with_next = computed
        .basic_keep_with_next()
        .map_err(|_| StagingFloatLayoutError::InvalidStyle(node_id))?;
    Ok((extent, keep_with_next))
}

fn inline_line_count(inlines: &[Inline]) -> Result<i64, StagingFloatLayoutError> {
    let mut count = 1i64;
    let mut stack: Vec<&Inline> = inlines.iter().rev().collect();
    while let Some(inline) = stack.pop() {
        match inline {
            Inline::HardBreak { .. } => {
                count = count
                    .checked_add(1)
                    .ok_or(StagingFloatLayoutError::ArithmeticOverflow)?;
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => stack.extend(children.iter().rev()),
            Inline::Text { .. }
            | Inline::Anchor { .. }
            | Inline::Reference { .. }
            | Inline::FootnoteReference { .. }
            | Inline::SoftBreak { .. } => {}
        }
    }
    Ok(count)
}

fn block_node_id(block: &Block) -> NodeId {
    match block {
        Block::Paragraph { node_id, .. }
        | Block::Heading { node_id, .. }
        | Block::List { node_id, .. }
        | Block::Table { node_id, .. }
        | Block::Figure { node_id, .. }
        | Block::PageBreak { node_id, .. } => *node_id,
    }
}

fn block_style_name(block: &Block) -> &'static str {
    match block {
        Block::Paragraph { .. } => "paragraph",
        Block::Heading { .. } => "heading",
        Block::List { .. } => "list",
        Block::Table { .. } => "table",
        Block::Figure { .. } => "figure",
        Block::PageBreak { .. } => "page_break",
    }
}

fn encode_registry(
    package_sha256: [u8; 32],
    profile_receipt_sha256: [u8; 32],
    flows: &[StagingFloatFlowRecord],
    columns: &[StagingFloatColumnTemplate],
    body_items: &[StagingFloatBodyItem],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, FLOAT_FLOW_REGISTRY_ALGORITHM);
    output.push_str(",\"body_items\":[");
    for (index, item) in body_items.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"after_position\":");
        output.push_str(&item.after_position.to_string());
        output.push_str(",\"before_position\":");
        output.push_str(&item.before_position.to_string());
        output.push_str(",\"block_extent\":");
        output.push_str(&item.block_extent.get().raw().to_string());
        output.push_str(",\"caption_flow_id\":");
        push_optional_flow(&mut output, item.caption_flow_id);
        output.push_str(",\"float_extent\":");
        push_optional_positive(&mut output, item.float_extent);
        output.push_str(",\"float_flow_id\":");
        push_optional_flow(&mut output, item.float_flow_id);
        output.push_str(",\"forced_page_break\":");
        output.push_str(if item.forced_page_break {
            "true"
        } else {
            "false"
        });
        output.push_str(",\"image_width\":");
        push_optional_positive(&mut output, item.image_width);
        output.push_str(",\"keep_with_next\":");
        output.push_str(if item.keep_with_next { "true" } else { "false" });
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, item.kind.as_str());
        output.push_str(",\"node_id\":");
        output.push_str(&item.node_id.get().to_string());
        output.push('}');
    }
    output.push_str("],\"columns\":[");
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"column_index\":");
        output.push_str(&column.column_index.to_string());
        output.push_str(",\"frame_flow_id\":");
        output.push_str(&column.frame_flow_id.get().to_string());
        output.push_str(",\"master_id\":");
        push_jcs_string(&mut output, column.master_id.as_str());
        output.push_str(",\"rect\":");
        push_rect(&mut output, column.rect);
        output.push_str(",\"source_flow_id\":");
        output.push_str(&column.source_flow_id.get().to_string());
        output.push('}');
    }
    output.push_str("],\"flows\":[");
    for (index, flow) in flows.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"column_index\":");
        match flow.column_index {
            Some(value) => output.push_str(&value.to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"depth\":");
        output.push_str(&flow.depth.to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&flow.flow_id.get().to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, flow.owner_kind.as_str());
        output.push_str(",\"master_id\":");
        match &flow.master_id {
            Some(value) => push_jcs_string(&mut output, value.as_str()),
            None => output.push_str("null"),
        }
        output.push_str(",\"owner_node_id\":");
        match flow.owner_node_id {
            Some(value) => output.push_str(&value.get().to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"parent_flow_id\":");
        match flow.parent_flow_id {
            Some(value) => output.push_str(&value.get().to_string()),
            None => output.push_str("null"),
        }
        output.push_str(",\"source_flow_id\":");
        output.push_str(&flow.source_flow_id.get().to_string());
        output.push_str(",\"terminal\":");
        output.push_str(&flow.terminal.to_string());
        output.push('}');
    }
    output.push_str("],\"package_sha256\":");
    push_hex(&mut output, package_sha256);
    output.push_str(",\"profile_receipt_sha256\":");
    push_hex(&mut output, profile_receipt_sha256);
    output.push('}');
    output
}

fn push_optional_flow(output: &mut String, value: Option<FlowId>) {
    match value {
        Some(value) => output.push_str(&value.get().to_string()),
        None => output.push_str("null"),
    }
}

fn push_optional_positive(output: &mut String, value: Option<PositiveLength>) {
    match value {
        Some(value) => output.push_str(&value.get().raw().to_string()),
        None => output.push_str("null"),
    }
}

fn push_rect(output: &mut String, rect: Rect) {
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

fn push_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

/// Deterministic lower-layer fixture for focused float queue/placement tests.
#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_float_layout_fixture() -> StagingFloatLayout {
    use core::num::NonZeroU16;
    use typaxis_document::{AdvancedPageMaster, ColumnBalance, ColumnFill, ColumnLayout};

    let positive = |raw| {
        Length::from_raw(raw)
            .and_then(PositiveLength::new)
            .expect("fixture length is positive")
    };
    let nonnegative = |raw| {
        Length::from_raw(raw)
            .and_then(NonNegativeLength::new)
            .expect("fixture length is nonnegative")
    };
    let rect = |x, y, width, height| {
        Rect::new(
            Length::from_raw(x).expect("fixture coordinate is exact"),
            Length::from_raw(y).expect("fixture coordinate is exact"),
            positive(width),
            positive(height),
        )
    };
    let master_id = MasterId::new("single").expect("fixture MasterId is valid");
    let page_master = PageMaster {
        master_id: master_id.clone(),
        width: positive(20),
        height: positive(20),
        body: rect(1, 1, 12, 10),
        header: None,
        footer: None,
        footnote: None,
    };
    let advanced_page_master = AdvancedPageMaster {
        master_id: master_id.clone(),
        trim: rect(0, 0, 20, 20),
        header_content: None,
        footer_content: None,
        column_layout: Some(ColumnLayout {
            count: NonZeroU16::new(2).expect("fixture count is nonzero"),
            gap: nonnegative(1),
            fill: ColumnFill::Sequential,
            balance: ColumnBalance::None,
        }),
    };
    let mut flows = vec![StagingFloatFlowRecord {
        flow_id: FlowId::DOCUMENT_BODY,
        owner_node_id: Some(NodeId::new(0)),
        owner_kind: StagingFloatFlowOwnerKind::DocumentBody,
        parent_flow_id: None,
        source_flow_id: FlowId::DOCUMENT_BODY,
        depth: 1,
        terminal: 7,
        master_id: None,
        column_index: None,
    }];
    for (flow_id, node_id, kind, parent, depth, terminal) in [
        (1, 3, StagingFloatFlowOwnerKind::Float, 0, 2, 1),
        (2, 3, StagingFloatFlowOwnerKind::FloatCaption, 1, 3, 1),
        (3, 8, StagingFloatFlowOwnerKind::Float, 0, 2, 1),
        (4, 8, StagingFloatFlowOwnerKind::FloatCaption, 3, 3, 1),
        (5, 11, StagingFloatFlowOwnerKind::Float, 0, 2, 1),
        (6, 11, StagingFloatFlowOwnerKind::FloatCaption, 5, 3, 1),
        (7, 14, StagingFloatFlowOwnerKind::Float, 0, 2, 1),
        (8, 14, StagingFloatFlowOwnerKind::FloatCaption, 7, 3, 1),
        (9, 17, StagingFloatFlowOwnerKind::Float, 0, 2, 1),
        (10, 17, StagingFloatFlowOwnerKind::FloatCaption, 9, 3, 1),
    ] {
        flows.push(StagingFloatFlowRecord {
            flow_id: FlowId::new(flow_id),
            owner_node_id: Some(NodeId::new(node_id)),
            owner_kind: kind,
            parent_flow_id: Some(FlowId::new(parent)),
            source_flow_id: FlowId::new(flow_id),
            depth,
            terminal,
            master_id: None,
            column_index: None,
        });
    }
    flows.extend([
        StagingFloatFlowRecord {
            flow_id: FlowId::new(11),
            owner_node_id: None,
            owner_kind: StagingFloatFlowOwnerKind::ColumnTemplate,
            parent_flow_id: Some(FlowId::DOCUMENT_BODY),
            source_flow_id: FlowId::DOCUMENT_BODY,
            depth: 2,
            terminal: 7,
            master_id: Some(master_id.clone()),
            column_index: Some(0),
        },
        StagingFloatFlowRecord {
            flow_id: FlowId::new(12),
            owner_node_id: None,
            owner_kind: StagingFloatFlowOwnerKind::ColumnTemplate,
            parent_flow_id: Some(FlowId::DOCUMENT_BODY),
            source_flow_id: FlowId::DOCUMENT_BODY,
            depth: 2,
            terminal: 7,
            master_id: Some(master_id.clone()),
            column_index: Some(1),
        },
    ]);
    let columns = vec![
        StagingFloatColumnTemplate {
            master_id: master_id.clone(),
            column_index: 0,
            frame_flow_id: FlowId::new(11),
            source_flow_id: FlowId::DOCUMENT_BODY,
            rect: rect(1, 1, 5, 10),
        },
        StagingFloatColumnTemplate {
            master_id: master_id.clone(),
            column_index: 1,
            frame_flow_id: FlowId::new(12),
            source_flow_id: FlowId::DOCUMENT_BODY,
            rect: rect(7, 1, 6, 10),
        },
    ];
    let block = |node_id, before, extent| StagingFloatBodyItem {
        node_id: NodeId::new(node_id),
        before_position: before,
        after_position: before + 1,
        kind: StagingFloatBodyItemKind::Block,
        block_extent: nonnegative(extent),
        keep_with_next: false,
        forced_page_break: false,
        float_flow_id: None,
        caption_flow_id: None,
        image_width: None,
        float_extent: None,
    };
    let float = |node_id, before, float_flow, caption_flow, width, extent| StagingFloatBodyItem {
        node_id: NodeId::new(node_id),
        before_position: before,
        after_position: before + 1,
        kind: StagingFloatBodyItemKind::FloatAnchor,
        block_extent: NonNegativeLength::ZERO,
        keep_with_next: false,
        forced_page_break: false,
        float_flow_id: Some(FlowId::new(float_flow)),
        caption_flow_id: Some(FlowId::new(caption_flow)),
        image_width: Some(positive(width)),
        float_extent: Some(positive(extent)),
    };
    let body_items = vec![
        block(1, 0, 1),
        float(3, 1, 1, 2, 2, 4),
        block(6, 2, 3),
        float(8, 3, 3, 4, 4, 8),
        float(11, 4, 5, 6, 4, 8),
        float(14, 5, 7, 8, 4, 8),
        float(17, 6, 9, 10, 4, 8),
    ];
    let profile_receipt_sha256 = [0x31; 32];
    let package_sha256 = [0x13; 32];
    let canonical_jcs = encode_registry(
        package_sha256,
        profile_receipt_sha256,
        &flows,
        &columns,
        &body_items,
    );
    let receipt = StagingFloatFlowRegistryReceipt {
        package: DocumentFingerprint::from_untrusted_bytes([0x41; 32]),
        style: StyleFingerprint::from_untrusted_bytes([0x42; 32]),
        profile_receipt_sha256,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        flow_count: 13,
        body_terminal: 7,
        canonical_jcs,
    };
    StagingFloatLayout {
        page_master,
        advanced_page_master,
        flows,
        columns,
        body_items,
        receipt,
    }
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_float_oversize_layout_fixture() -> StagingFloatLayout {
    let mut layout = staging_float_layout_fixture();
    layout.body_items[1].image_width = Some(
        Length::from_raw(6)
            .and_then(PositiveLength::new)
            .expect("fixture width is positive"),
    );
    reseal_float_layout_fixture(&mut layout);
    layout
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_float_forced_break_layout_fixture() -> StagingFloatLayout {
    let mut layout = staging_float_layout_fixture();
    let boundary = &mut layout.body_items[4];
    boundary.kind = StagingFloatBodyItemKind::Block;
    boundary.block_extent = NonNegativeLength::ZERO;
    boundary.keep_with_next = false;
    boundary.forced_page_break = true;
    boundary.float_flow_id = None;
    boundary.caption_flow_id = None;
    boundary.image_width = None;
    boundary.float_extent = None;
    reseal_float_layout_fixture(&mut layout);
    layout
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_float_trailing_break_layout_fixture() -> StagingFloatLayout {
    let mut layout = staging_float_layout_fixture();
    let mut boundary = layout.body_items.remove(0);
    boundary.block_extent = NonNegativeLength::ZERO;
    boundary.keep_with_next = false;
    boundary.forced_page_break = true;
    boundary.float_flow_id = None;
    boundary.caption_flow_id = None;
    boundary.image_width = None;
    boundary.float_extent = None;
    boundary.before_position = 0;
    boundary.after_position = 1;
    layout.body_items.clear();
    layout.body_items.push(boundary);

    let master_id = layout.page_master.master_id.clone();
    layout.flows = vec![
        StagingFloatFlowRecord {
            flow_id: FlowId::DOCUMENT_BODY,
            owner_node_id: Some(NodeId::new(0)),
            owner_kind: StagingFloatFlowOwnerKind::DocumentBody,
            parent_flow_id: None,
            source_flow_id: FlowId::DOCUMENT_BODY,
            depth: 1,
            terminal: 1,
            master_id: None,
            column_index: None,
        },
        StagingFloatFlowRecord {
            flow_id: FlowId::new(1),
            owner_node_id: None,
            owner_kind: StagingFloatFlowOwnerKind::ColumnTemplate,
            parent_flow_id: Some(FlowId::DOCUMENT_BODY),
            source_flow_id: FlowId::DOCUMENT_BODY,
            depth: 2,
            terminal: 1,
            master_id: Some(master_id.clone()),
            column_index: Some(0),
        },
        StagingFloatFlowRecord {
            flow_id: FlowId::new(2),
            owner_node_id: None,
            owner_kind: StagingFloatFlowOwnerKind::ColumnTemplate,
            parent_flow_id: Some(FlowId::DOCUMENT_BODY),
            source_flow_id: FlowId::DOCUMENT_BODY,
            depth: 2,
            terminal: 1,
            master_id: Some(master_id),
            column_index: Some(1),
        },
    ];
    for (index, column) in layout.columns.iter_mut().enumerate() {
        column.frame_flow_id =
            FlowId::new(u32::try_from(index + 1).expect("fixture column FlowId fits u32"));
    }
    reseal_float_layout_fixture(&mut layout);
    layout
}

#[cfg(any(test, feature = "staging-fixtures"))]
fn reseal_float_layout_fixture(layout: &mut StagingFloatLayout) {
    let canonical_jcs = encode_registry(
        [0x13; 32],
        layout.receipt.profile_receipt_sha256,
        &layout.flows,
        &layout.columns,
        &layout.body_items,
    );
    layout.receipt.fingerprint = sha256(canonical_jcs.as_bytes());
    layout.receipt.flow_count =
        u32::try_from(layout.flows.len()).expect("fixture flow count fits u32");
    layout.receipt.body_terminal =
        u32::try_from(layout.body_items.len()).expect("fixture body count fits u32");
    layout.receipt.canonical_jcs = canonical_jcs;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_owner_order_keeps_float_and_caption_before_columns() {
        assert_eq!(StagingFloatFlowOwnerKind::Float.as_str(), "float");
        assert_eq!(
            StagingFloatFlowOwnerKind::FloatCaption.as_str(),
            "float_caption"
        );
        assert_eq!(
            FLOAT_FLOW_REGISTRY_ALGORITHM,
            "typaxis.advanced-flow-registry/1"
        );
    }
}
