use typaxis_core::{
    push_jcs_string, sha256, DocumentFingerprint, Length, MasterId, NodeId, NonNegativeLength,
    PositiveLength, Rect, StyleFingerprint, ValidatedResourceLimits,
};
use typaxis_document::{Block, Inline, ListItem};
use typaxis_layout_contract::FlowId;
use typaxis_style::{MachineFigureWidth, PageMaster, StyleValue};
use typaxis_syntax::ValidatedStagingAdvancedPackage;

const ADVANCED_FLOW_REGISTRY_ALGORITHM: &str = "typaxis.advanced-flow-registry/1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingColumnFlowOwnerKind {
    DocumentBody,
    ListItem,
    FigureCaption,
    ColumnTemplate,
}

impl StagingColumnFlowOwnerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DocumentBody => "document_body",
            Self::ListItem => "list_item",
            Self::FigureCaption => "figure_caption",
            Self::ColumnTemplate => "column_template",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingColumnFlowRecord {
    flow_id: FlowId,
    owner_node_id: Option<NodeId>,
    owner_kind: StagingColumnFlowOwnerKind,
    parent_flow_id: Option<FlowId>,
    source_flow_id: FlowId,
    depth: u32,
    terminal: u32,
    master_id: Option<MasterId>,
    column_index: Option<u32>,
}

impl StagingColumnFlowRecord {
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
    pub const fn owner_node_id(&self) -> Option<NodeId> {
        self.owner_node_id
    }
    pub const fn owner_kind(&self) -> StagingColumnFlowOwnerKind {
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
pub struct StagingColumnTemplate {
    master_id: MasterId,
    column_index: u32,
    frame_flow_id: FlowId,
    source_flow_id: FlowId,
    rect: Rect,
}

impl StagingColumnTemplate {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingColumnBlockLayout {
    node_id: NodeId,
    before_position: u32,
    after_position: u32,
    block_extent: NonNegativeLength,
    keep_with_next: bool,
    forced_page_break: bool,
}

impl StagingColumnBlockLayout {
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn before_position(&self) -> u32 {
        self.before_position
    }
    pub const fn after_position(&self) -> u32 {
        self.after_position
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
}

#[derive(Debug)]
pub struct StagingColumnsFlowRegistryReceipt {
    package: DocumentFingerprint,
    style: StyleFingerprint,
    profile_receipt_sha256: [u8; 32],
    fingerprint: [u8; 32],
    flow_count: u32,
    body_terminal: u32,
    canonical_jcs: String,
}

impl StagingColumnsFlowRegistryReceipt {
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
pub struct StagingColumnsLayout {
    page_master: PageMaster,
    advanced_page_master: typaxis_document::AdvancedPageMaster,
    flows: Vec<StagingColumnFlowRecord>,
    columns: Vec<StagingColumnTemplate>,
    blocks: Vec<StagingColumnBlockLayout>,
    receipt: StagingColumnsFlowRegistryReceipt,
}

impl StagingColumnsLayout {
    pub const fn page_master(&self) -> &PageMaster {
        &self.page_master
    }
    pub const fn advanced_page_master(&self) -> &typaxis_document::AdvancedPageMaster {
        &self.advanced_page_master
    }
    pub fn flows(&self) -> &[StagingColumnFlowRecord] {
        &self.flows
    }
    pub fn flow(&self, flow_id: FlowId) -> Option<&StagingColumnFlowRecord> {
        self.flows
            .get(usize::try_from(flow_id.get()).ok()?)
            .filter(|flow| flow.flow_id == flow_id)
    }
    pub fn columns(&self) -> &[StagingColumnTemplate] {
        &self.columns
    }
    pub fn blocks(&self) -> &[StagingColumnBlockLayout] {
        &self.blocks
    }
    pub const fn receipt(&self) -> &StagingColumnsFlowRegistryReceipt {
        &self.receipt
    }

    pub fn verify_receipt(
        &self,
        package: &ValidatedStagingAdvancedPackage,
        profile_receipt_sha256: [u8; 32],
        limits: &ValidatedResourceLimits,
    ) -> Result<(), StagingColumnsLayoutError> {
        let epoch = package.package().epoch_identity();
        let expected_master = package
            .package()
            .package()
            .page_masters
            .masters
            .first()
            .ok_or(StagingColumnsLayoutError::ReceiptMismatch)?;
        let expected_advanced = package
            .page_masters()
            .masters
            .first()
            .ok_or(StagingColumnsLayoutError::ReceiptMismatch)?;
        let canonical = encode_registry(
            package.raw_sha256(),
            profile_receipt_sha256,
            &self.flows,
            &self.columns,
            &self.blocks,
        );
        if self.page_master != *expected_master
            || self.advanced_page_master != *expected_advanced
            || !self.structure_is_valid(package, limits)?
            || self.receipt.package != epoch.document()
            || self.receipt.style != epoch.style()
            || self.receipt.profile_receipt_sha256 != profile_receipt_sha256
            || usize::try_from(self.receipt.flow_count) != Ok(self.flows.len())
            || usize::try_from(self.receipt.body_terminal) != Ok(self.blocks.len())
            || self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingColumnsLayoutError::ReceiptMismatch);
        }
        Ok(())
    }

    fn structure_is_valid(
        &self,
        package: &ValidatedStagingAdvancedPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<bool, StagingColumnsLayoutError> {
        let document = &package.package().package().document;
        let Some(body) = self.flows.first() else {
            return Ok(false);
        };
        if body.flow_id != FlowId::DOCUMENT_BODY
            || body.owner_node_id != Some(document.node_id)
            || body.owner_kind != StagingColumnFlowOwnerKind::DocumentBody
            || body.parent_flow_id.is_some()
            || body.source_flow_id != FlowId::DOCUMENT_BODY
            || body.depth != 1
            || usize::try_from(body.terminal) != Ok(document.blocks.len())
            || body.master_id.is_some()
            || body.column_index.is_some()
            || self
                .flows
                .iter()
                .enumerate()
                .any(|(index, flow)| usize::try_from(flow.flow_id.get()) != Ok(index))
        {
            return Ok(false);
        }

        let body_terminal = u32::try_from(document.blocks.len())
            .map_err(|_| StagingColumnsLayoutError::AstNodeLimit)?;
        let mut expected_flows = Vec::new();
        expected_flows
            .try_reserve_exact(package.document_nodes_capacity_hint())
            .map_err(|_| StagingColumnsLayoutError::AllocationFailure)?;
        expected_flows.push(StagingColumnFlowRecord {
            flow_id: FlowId::DOCUMENT_BODY,
            owner_node_id: Some(document.node_id),
            owner_kind: StagingColumnFlowOwnerKind::DocumentBody,
            parent_flow_id: None,
            source_flow_id: FlowId::DOCUMENT_BODY,
            depth: 1,
            terminal: body_terminal,
            master_id: None,
            column_index: None,
        });
        collect_descendant_flows(
            &document.blocks,
            FlowId::DOCUMENT_BODY,
            1,
            &mut expected_flows,
            limits,
        )?;
        let mut expected_columns = derive_column_templates(
            &self.page_master,
            &self.advanced_page_master,
            &mut expected_flows,
            body_terminal,
            limits,
        )?;
        expected_columns.sort_by_key(|column| column.column_index);
        if self.flows != expected_flows || self.columns != expected_columns {
            return Ok(false);
        }

        if self.columns.is_empty()
            || self.columns.iter().enumerate().any(|(index, column)| {
                u32::try_from(index) != Ok(column.column_index)
                    || column.master_id != self.page_master.master_id
                    || column.source_flow_id != FlowId::DOCUMENT_BODY
            })
            || !columns_close_exactly(&self.page_master, &self.columns)
        {
            return Ok(false);
        }

        let has_layout = self.advanced_page_master.column_layout.is_some();
        for column in &self.columns {
            if has_layout {
                let Some(flow) = self.flow(column.frame_flow_id) else {
                    return Ok(false);
                };
                if flow.owner_node_id.is_some()
                    || flow.owner_kind != StagingColumnFlowOwnerKind::ColumnTemplate
                    || flow.parent_flow_id != Some(FlowId::DOCUMENT_BODY)
                    || flow.source_flow_id != FlowId::DOCUMENT_BODY
                    || flow.depth != 2
                    || flow.terminal != body.terminal
                    || flow.master_id.as_ref() != Some(&self.page_master.master_id)
                    || flow.column_index != Some(column.column_index)
                {
                    return Ok(false);
                }
            } else if column.frame_flow_id != FlowId::DOCUMENT_BODY || column.column_index != 0 {
                return Ok(false);
            }
        }

        if self.blocks.len() != document.blocks.len() {
            return Ok(false);
        }
        for (index, (actual, source)) in self.blocks.iter().zip(&document.blocks).enumerate() {
            let before =
                u32::try_from(index).map_err(|_| StagingColumnsLayoutError::ArithmeticOverflow)?;
            let expected = measure_block(package, source, self.page_master.body.width())?;
            if actual.node_id != block_node_id(source)
                || actual.before_position != before
                || actual.after_position
                    != before
                        .checked_add(1)
                        .ok_or(StagingColumnsLayoutError::ArithmeticOverflow)?
                || actual.block_extent != expected.0
                || actual.keep_with_next != expected.1
                || actual.forced_page_break != matches!(source, Block::PageBreak { .. })
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingColumnsLayoutError {
    UnsupportedContent(NodeId),
    InvalidStyle(NodeId),
    InvalidGeometry,
    AstNodeLimit,
    FlowDepthLimit,
    ReceiptMismatch,
    ArithmeticOverflow,
    AllocationFailure,
}

impl std::fmt::Display for StagingColumnsLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContent(node) => {
                write!(
                    formatter,
                    "L5100: unsupported columns content at node {}",
                    node.get()
                )
            }
            Self::InvalidStyle(node) => {
                write!(
                    formatter,
                    "L5101: invalid columns style at node {}",
                    node.get()
                )
            }
            Self::InvalidGeometry => formatter.write_str("L5101: invalid column geometry"),
            Self::AstNodeLimit => formatter.write_str("P1120: column flow limit exceeded"),
            Self::FlowDepthLimit => formatter.write_str("P1121: column flow depth exceeded"),
            Self::ReceiptMismatch => formatter.write_str("I9190: column flow receipt mismatch"),
            Self::ArithmeticOverflow => formatter.write_str("L5101: column layout overflow"),
            Self::AllocationFailure => formatter.write_str("L5110: column allocation failure"),
        }
    }
}

impl std::error::Error for StagingColumnsLayoutError {}

/// Deterministic lower-layer fixture for focused column pagination tests.
/// Production builds do not expose it unless the explicit fixture feature is
/// selected.
#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_columns_layout_fixture() -> StagingColumnsLayout {
    use core::num::NonZeroU16;
    use typaxis_document::{AdvancedPageMaster, ColumnBalance, ColumnFill, ColumnLayout};

    let positive = |raw| {
        Length::from_raw(raw)
            .and_then(PositiveLength::new)
            .expect("fixture length is positive")
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
            count: NonZeroU16::new(2).expect("fixture column count is nonzero"),
            gap: Length::from_raw(1)
                .and_then(NonNegativeLength::new)
                .expect("fixture gap is nonnegative"),
            fill: ColumnFill::Sequential,
            balance: ColumnBalance::LastPage,
        }),
    };
    let flows = vec![
        StagingColumnFlowRecord {
            flow_id: FlowId::DOCUMENT_BODY,
            owner_node_id: Some(NodeId::new(0)),
            owner_kind: StagingColumnFlowOwnerKind::DocumentBody,
            parent_flow_id: None,
            source_flow_id: FlowId::DOCUMENT_BODY,
            depth: 1,
            terminal: 5,
            master_id: None,
            column_index: None,
        },
        StagingColumnFlowRecord {
            flow_id: FlowId::new(1),
            owner_node_id: None,
            owner_kind: StagingColumnFlowOwnerKind::ColumnTemplate,
            parent_flow_id: Some(FlowId::DOCUMENT_BODY),
            source_flow_id: FlowId::DOCUMENT_BODY,
            depth: 2,
            terminal: 5,
            master_id: Some(master_id.clone()),
            column_index: Some(0),
        },
        StagingColumnFlowRecord {
            flow_id: FlowId::new(2),
            owner_node_id: None,
            owner_kind: StagingColumnFlowOwnerKind::ColumnTemplate,
            parent_flow_id: Some(FlowId::DOCUMENT_BODY),
            source_flow_id: FlowId::DOCUMENT_BODY,
            depth: 2,
            terminal: 5,
            master_id: Some(master_id.clone()),
            column_index: Some(1),
        },
    ];
    let columns = vec![
        StagingColumnTemplate {
            master_id: master_id.clone(),
            column_index: 0,
            frame_flow_id: FlowId::new(1),
            source_flow_id: FlowId::DOCUMENT_BODY,
            rect: rect(1, 1, 5, 10),
        },
        StagingColumnTemplate {
            master_id: master_id.clone(),
            column_index: 1,
            frame_flow_id: FlowId::new(2),
            source_flow_id: FlowId::DOCUMENT_BODY,
            rect: rect(7, 1, 6, 10),
        },
    ];
    let blocks = (0..5u32)
        .map(|index| StagingColumnBlockLayout {
            node_id: NodeId::new(index + 1),
            before_position: index,
            after_position: index + 1,
            block_extent: Length::from_raw(4)
                .and_then(NonNegativeLength::new)
                .expect("fixture extent is nonnegative"),
            keep_with_next: false,
            forced_page_break: false,
        })
        .collect::<Vec<_>>();
    let profile_receipt_sha256 = [0x31; 32];
    let package_sha256 = [0x13; 32];
    let canonical_jcs = encode_registry(
        package_sha256,
        profile_receipt_sha256,
        &flows,
        &columns,
        &blocks,
    );
    let receipt = StagingColumnsFlowRegistryReceipt {
        package: DocumentFingerprint::from_untrusted_bytes([0x41; 32]),
        style: StyleFingerprint::from_untrusted_bytes([0x42; 32]),
        profile_receipt_sha256,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        flow_count: 3,
        body_terminal: 5,
        canonical_jcs,
    };
    StagingColumnsLayout {
        page_master,
        advanced_page_master,
        flows,
        columns,
        blocks,
        receipt,
    }
}

/// Lower-layer fixture whose first indivisible block exceeds a full column.
#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_columns_oversize_layout_fixture() -> StagingColumnsLayout {
    let mut layout = staging_columns_layout_fixture();
    layout.blocks[0].block_extent = Length::from_raw(11)
        .and_then(NonNegativeLength::new)
        .expect("fixture extent is nonnegative");
    reseal_staging_columns_layout_fixture(&mut layout);
    layout
}

/// Lower-layer fixture for terminal empty-column behavior.
#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_columns_empty_layout_fixture() -> StagingColumnsLayout {
    let mut layout = staging_columns_layout_fixture();
    layout.blocks.clear();
    for flow in &mut layout.flows {
        if matches!(
            flow.owner_kind,
            StagingColumnFlowOwnerKind::DocumentBody | StagingColumnFlowOwnerKind::ColumnTemplate
        ) {
            flow.terminal = 0;
        }
    }
    reseal_staging_columns_layout_fixture(&mut layout);
    layout
}

#[cfg(any(test, feature = "staging-fixtures"))]
fn reseal_staging_columns_layout_fixture(layout: &mut StagingColumnsLayout) {
    let canonical_jcs = encode_registry(
        [0x13; 32],
        layout.receipt.profile_receipt_sha256,
        &layout.flows,
        &layout.columns,
        &layout.blocks,
    );
    layout.receipt.fingerprint = sha256(canonical_jcs.as_bytes());
    layout.receipt.flow_count =
        u32::try_from(layout.flows.len()).expect("fixture flow count fits u32");
    layout.receipt.body_terminal =
        u32::try_from(layout.blocks.len()).expect("fixture block count fits u32");
    layout.receipt.canonical_jcs = canonical_jcs;
}

pub fn layout_staging_columns(
    package: &ValidatedStagingAdvancedPackage,
    verified_profile_receipt_sha256: [u8; 32],
    limits: &ValidatedResourceLimits,
) -> Result<StagingColumnsLayout, StagingColumnsLayoutError> {
    let document = &package.package().package().document;
    let page_master = package
        .package()
        .package()
        .page_masters
        .masters
        .first()
        .ok_or(StagingColumnsLayoutError::InvalidGeometry)?
        .clone();
    let advanced_page_master = package
        .page_masters()
        .masters
        .first()
        .ok_or(StagingColumnsLayoutError::InvalidGeometry)?
        .clone();
    let body_terminal = u32::try_from(document.blocks.len())
        .map_err(|_| StagingColumnsLayoutError::AstNodeLimit)?;

    let mut flows = Vec::new();
    flows
        .try_reserve_exact(package.document_nodes_capacity_hint())
        .map_err(|_| StagingColumnsLayoutError::AllocationFailure)?;
    flows.push(StagingColumnFlowRecord {
        flow_id: FlowId::DOCUMENT_BODY,
        owner_node_id: Some(document.node_id),
        owner_kind: StagingColumnFlowOwnerKind::DocumentBody,
        parent_flow_id: None,
        source_flow_id: FlowId::DOCUMENT_BODY,
        depth: 1,
        terminal: body_terminal,
        master_id: None,
        column_index: None,
    });
    collect_descendant_flows(
        &document.blocks,
        FlowId::DOCUMENT_BODY,
        1,
        &mut flows,
        limits,
    )?;

    let mut columns = derive_column_templates(
        &page_master,
        &advanced_page_master,
        &mut flows,
        body_terminal,
        limits,
    )?;
    columns.sort_by_key(|column| column.column_index);

    let mut blocks = Vec::new();
    blocks
        .try_reserve_exact(document.blocks.len())
        .map_err(|_| StagingColumnsLayoutError::AllocationFailure)?;
    for (index, block) in document.blocks.iter().enumerate() {
        let before = u32::try_from(index).map_err(|_| StagingColumnsLayoutError::AstNodeLimit)?;
        let (extent, keep_with_next) = measure_block(package, block, page_master.body.width())?;
        blocks.push(StagingColumnBlockLayout {
            node_id: block_node_id(block),
            before_position: before,
            after_position: before
                .checked_add(1)
                .ok_or(StagingColumnsLayoutError::ArithmeticOverflow)?,
            block_extent: extent,
            keep_with_next,
            forced_page_break: matches!(block, Block::PageBreak { .. }),
        });
    }

    let canonical_jcs = encode_registry(
        package.raw_sha256(),
        verified_profile_receipt_sha256,
        &flows,
        &columns,
        &blocks,
    );
    let receipt = StagingColumnsFlowRegistryReceipt {
        package: package.package().epoch_identity().document(),
        style: package.package().epoch_identity().style(),
        profile_receipt_sha256: verified_profile_receipt_sha256,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        flow_count: u32::try_from(flows.len())
            .map_err(|_| StagingColumnsLayoutError::AstNodeLimit)?,
        body_terminal,
        canonical_jcs,
    };
    let layout = StagingColumnsLayout {
        page_master,
        advanced_page_master,
        flows,
        columns,
        blocks,
        receipt,
    };
    layout.verify_receipt(package, verified_profile_receipt_sha256, limits)?;
    Ok(layout)
}

trait AdvancedPackageCapacity {
    fn document_nodes_capacity_hint(&self) -> usize;
}

impl AdvancedPackageCapacity for ValidatedStagingAdvancedPackage {
    fn document_nodes_capacity_hint(&self) -> usize {
        let columns = self
            .page_masters()
            .masters
            .first()
            .and_then(|master| master.column_layout.as_ref())
            .map_or(0usize, |layout| usize::from(layout.count.get()));
        self.package()
            .document_nodes()
            .node_count()
            .saturating_add(columns)
    }
}

fn collect_descendant_flows(
    blocks: &[Block],
    parent: FlowId,
    parent_depth: u32,
    flows: &mut Vec<StagingColumnFlowRecord>,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingColumnsLayoutError> {
    for block in blocks {
        match block {
            Block::List { items, .. } => {
                for item in items {
                    collect_list_item_flow(item, parent, parent_depth, flows, limits)?;
                }
            }
            Block::Figure {
                node_id, caption, ..
            } => {
                let flow_id = next_flow_id(flows)?;
                let depth = parent_depth
                    .checked_add(1)
                    .ok_or(StagingColumnsLayoutError::FlowDepthLimit)?;
                allocate_flow(
                    flows,
                    Some(*node_id),
                    StagingColumnFlowOwnerKind::FigureCaption,
                    parent,
                    flow_id,
                    depth,
                    u32::try_from(caption.len())
                        .map_err(|_| StagingColumnsLayoutError::AstNodeLimit)?,
                    None,
                    None,
                    limits,
                )?;
                collect_descendant_flows(caption, flow_id, depth, flows, limits)?;
            }
            Block::Table { node_id, .. } => {
                return Err(StagingColumnsLayoutError::UnsupportedContent(*node_id));
            }
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    Ok(())
}

fn collect_list_item_flow(
    item: &ListItem,
    parent: FlowId,
    parent_depth: u32,
    flows: &mut Vec<StagingColumnFlowRecord>,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingColumnsLayoutError> {
    let flow_id = next_flow_id(flows)?;
    let depth = parent_depth
        .checked_add(1)
        .ok_or(StagingColumnsLayoutError::FlowDepthLimit)?;
    allocate_flow(
        flows,
        Some(item.node_id),
        StagingColumnFlowOwnerKind::ListItem,
        parent,
        flow_id,
        depth,
        u32::try_from(item.blocks.len()).map_err(|_| StagingColumnsLayoutError::AstNodeLimit)?,
        None,
        None,
        limits,
    )?;
    collect_descendant_flows(&item.blocks, flow_id, depth, flows, limits)
}

fn derive_column_templates(
    master: &PageMaster,
    advanced: &typaxis_document::AdvancedPageMaster,
    flows: &mut Vec<StagingColumnFlowRecord>,
    body_terminal: u32,
    limits: &ValidatedResourceLimits,
) -> Result<Vec<StagingColumnTemplate>, StagingColumnsLayoutError> {
    let Some(layout) = &advanced.column_layout else {
        return Ok(vec![StagingColumnTemplate {
            master_id: master.master_id.clone(),
            column_index: 0,
            frame_flow_id: FlowId::DOCUMENT_BODY,
            source_flow_id: FlowId::DOCUMENT_BODY,
            rect: master.body,
        }]);
    };
    let count = i64::from(layout.count.get());
    let total_gap = count
        .checked_sub(1)
        .and_then(|value| value.checked_mul(layout.gap.get().raw()))
        .ok_or(StagingColumnsLayoutError::InvalidGeometry)?;
    let available = master
        .body
        .width()
        .get()
        .raw()
        .checked_sub(total_gap)
        .ok_or(StagingColumnsLayoutError::InvalidGeometry)?;
    let base = available
        .checked_div(count)
        .filter(|value| *value > 0)
        .ok_or(StagingColumnsLayoutError::InvalidGeometry)?;
    let residual = available
        .checked_rem(count)
        .ok_or(StagingColumnsLayoutError::InvalidGeometry)?;
    let capacity = usize::from(layout.count.get());
    let mut columns = Vec::new();
    columns
        .try_reserve_exact(capacity)
        .map_err(|_| StagingColumnsLayoutError::AllocationFailure)?;
    let mut x = master.body.x().raw();
    for index in 0..layout.count.get() {
        let column_index = u32::from(index);
        let width = if index + 1 == layout.count.get() {
            base.checked_add(residual)
                .ok_or(StagingColumnsLayoutError::InvalidGeometry)?
        } else {
            base
        };
        let width = Length::from_raw(width)
            .and_then(PositiveLength::new)
            .ok_or(StagingColumnsLayoutError::InvalidGeometry)?;
        let frame_flow_id = next_flow_id(flows)?;
        allocate_flow(
            flows,
            None,
            StagingColumnFlowOwnerKind::ColumnTemplate,
            FlowId::DOCUMENT_BODY,
            FlowId::DOCUMENT_BODY,
            2,
            body_terminal,
            Some(master.master_id.clone()),
            Some(column_index),
            limits,
        )?;
        columns.push(StagingColumnTemplate {
            master_id: master.master_id.clone(),
            column_index,
            frame_flow_id,
            source_flow_id: FlowId::DOCUMENT_BODY,
            rect: Rect::new(
                Length::from_raw(x).ok_or(StagingColumnsLayoutError::InvalidGeometry)?,
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
            .ok_or(StagingColumnsLayoutError::InvalidGeometry)?;
    }
    let expected_right = master
        .body
        .x()
        .raw()
        .checked_add(master.body.width().get().raw())
        .ok_or(StagingColumnsLayoutError::InvalidGeometry)?;
    if x != expected_right {
        return Err(StagingColumnsLayoutError::InvalidGeometry);
    }
    Ok(columns)
}

fn columns_close_exactly(master: &PageMaster, columns: &[StagingColumnTemplate]) -> bool {
    let Some(first) = columns.first() else {
        return false;
    };
    let Some(last) = columns.last() else {
        return false;
    };
    if first.rect.x() != master.body.x()
        || columns.iter().any(|column| {
            column.rect.y() != master.body.y()
                || column.rect.height() != master.body.height()
                || column.rect.width().get().raw() <= 0
        })
    {
        return false;
    }
    let Some(right) = last
        .rect
        .x()
        .raw()
        .checked_add(last.rect.width().get().raw())
    else {
        return false;
    };
    let Some(body_right) = master
        .body
        .x()
        .raw()
        .checked_add(master.body.width().get().raw())
    else {
        return false;
    };
    if right != body_right {
        return false;
    }
    let gap = match columns.get(1) {
        Some(second) => second
            .rect
            .x()
            .raw()
            .checked_sub(first.rect.x().raw())
            .and_then(|value| value.checked_sub(first.rect.width().get().raw())),
        None => Some(0),
    };
    let Some(gap) = gap else { return false };
    columns.windows(2).all(|pair| {
        pair[0]
            .rect
            .x()
            .raw()
            .checked_add(pair[0].rect.width().get().raw())
            .and_then(|value| value.checked_add(gap))
            == Some(pair[1].rect.x().raw())
    })
}

fn next_flow_id(flows: &[StagingColumnFlowRecord]) -> Result<FlowId, StagingColumnsLayoutError> {
    Ok(FlowId::new(
        u32::try_from(flows.len()).map_err(|_| StagingColumnsLayoutError::AstNodeLimit)?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn allocate_flow(
    flows: &mut Vec<StagingColumnFlowRecord>,
    owner_node_id: Option<NodeId>,
    owner_kind: StagingColumnFlowOwnerKind,
    parent_flow_id: FlowId,
    source_flow_id: FlowId,
    depth: u32,
    terminal: u32,
    master_id: Option<MasterId>,
    column_index: Option<u32>,
    limits: &ValidatedResourceLimits,
) -> Result<FlowId, StagingColumnsLayoutError> {
    if u64::try_from(flows.len()).map_err(|_| StagingColumnsLayoutError::AstNodeLimit)?
        >= limits.get().max_ast_nodes
    {
        return Err(StagingColumnsLayoutError::AstNodeLimit);
    }
    if depth > limits.get().max_ast_nesting_depth {
        return Err(StagingColumnsLayoutError::FlowDepthLimit);
    }
    let flow_id = next_flow_id(flows)?;
    flows
        .try_reserve(1)
        .map_err(|_| StagingColumnsLayoutError::AllocationFailure)?;
    flows.push(StagingColumnFlowRecord {
        flow_id,
        owner_node_id,
        owner_kind,
        parent_flow_id: Some(parent_flow_id),
        source_flow_id,
        depth,
        terminal,
        master_id,
        column_index,
    });
    Ok(flow_id)
}

fn measure_block(
    package: &ValidatedStagingAdvancedPackage,
    block: &Block,
    auto_figure_width: PositiveLength,
) -> Result<(NonNegativeLength, bool), StagingColumnsLayoutError> {
    if matches!(block, Block::PageBreak { .. }) {
        return Ok((NonNegativeLength::ZERO, false));
    }
    let node_id = block_node_id(block);
    let computed = package
        .package()
        .package()
        .style_sheet
        .cascade_basic_document(block_style_name(block), block.classes())
        .map_err(|_| StagingColumnsLayoutError::InvalidStyle(node_id))?;
    let nonnegative = |name: &str| match computed.properties().get(name) {
        Some(StyleValue::Length(value)) => NonNegativeLength::new(*value),
        None => Some(NonNegativeLength::ZERO),
        Some(_) => None,
    };
    let before = nonnegative("space_before")
        .ok_or(StagingColumnsLayoutError::InvalidStyle(node_id))?
        .get()
        .raw();
    let after = nonnegative("space_after")
        .ok_or(StagingColumnsLayoutError::InvalidStyle(node_id))?
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
                .ok_or(StagingColumnsLayoutError::InvalidStyle(node_id))?;
            let lines = inline_line_count(children)?;
            line_height
                .get()
                .raw()
                .checked_mul(lines)
                .ok_or(StagingColumnsLayoutError::ArithmeticOverflow)?
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
                    .ok_or(StagingColumnsLayoutError::ArithmeticOverflow)?;
            }
            if extent == 0 {
                computed
                    .properties()
                    .get("line_height")
                    .and_then(|value| match value {
                        StyleValue::Length(value) => PositiveLength::new(*value),
                        _ => None,
                    })
                    .ok_or(StagingColumnsLayoutError::InvalidStyle(node_id))?
                    .get()
                    .raw()
            } else {
                extent
            }
        }
        Block::Figure { caption, .. } => {
            let image_extent = match computed
                .basic_figure_width()
                .map_err(|_| StagingColumnsLayoutError::InvalidStyle(node_id))?
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
                    .ok_or(StagingColumnsLayoutError::ArithmeticOverflow)
            })?
        }
        Block::Table { .. } => return Err(StagingColumnsLayoutError::UnsupportedContent(node_id)),
        Block::PageBreak { .. } => unreachable!("page break returned above"),
    };
    let raw = before
        .checked_add(core)
        .and_then(|value| value.checked_add(after))
        .ok_or(StagingColumnsLayoutError::ArithmeticOverflow)?;
    let extent = Length::from_raw(raw)
        .and_then(NonNegativeLength::new)
        .ok_or(StagingColumnsLayoutError::ArithmeticOverflow)?;
    let keep_with_next = computed
        .basic_keep_with_next()
        .map_err(|_| StagingColumnsLayoutError::InvalidStyle(node_id))?;
    Ok((extent, keep_with_next))
}

fn inline_line_count(inlines: &[Inline]) -> Result<i64, StagingColumnsLayoutError> {
    let mut count = 1i64;
    let mut stack: Vec<&Inline> = inlines.iter().rev().collect();
    while let Some(inline) = stack.pop() {
        match inline {
            Inline::HardBreak { .. } => {
                count = count
                    .checked_add(1)
                    .ok_or(StagingColumnsLayoutError::ArithmeticOverflow)?;
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
    flows: &[StagingColumnFlowRecord],
    columns: &[StagingColumnTemplate],
    blocks: &[StagingColumnBlockLayout],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ADVANCED_FLOW_REGISTRY_ALGORITHM);
    output.push_str(",\"blocks\":[");
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"after_position\":");
        output.push_str(&block.after_position.to_string());
        output.push_str(",\"before_position\":");
        output.push_str(&block.before_position.to_string());
        output.push_str(",\"block_extent\":");
        output.push_str(&block.block_extent.get().raw().to_string());
        output.push_str(",\"forced_page_break\":");
        output.push_str(if block.forced_page_break {
            "true"
        } else {
            "false"
        });
        output.push_str(",\"keep_with_next\":");
        output.push_str(if block.keep_with_next {
            "true"
        } else {
            "false"
        });
        output.push_str(",\"node_id\":");
        output.push_str(&block.node_id.get().to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_partition_uses_exact_gap_and_last_physical_residual() {
        let layout = staging_columns_layout_fixture();
        assert_eq!(
            layout
                .columns()
                .iter()
                .map(|column| {
                    (
                        column.column_index(),
                        column.frame_flow_id().get(),
                        column.rect().x().raw(),
                        column.rect().width().get().raw(),
                    )
                })
                .collect::<Vec<_>>(),
            [(0, 1, 1, 5), (1, 2, 7, 6)]
        );
        let occupied = layout
            .columns()
            .iter()
            .map(|column| column.rect().width().get().raw())
            .sum::<i64>()
            + 1;
        assert_eq!(occupied, layout.page_master().body.width().get().raw());
        assert_eq!(layout.receipt().flow_count(), 3);
        assert_eq!(layout.receipt().body_terminal(), 5);
    }
}
