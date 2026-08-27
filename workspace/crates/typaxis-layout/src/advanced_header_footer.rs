#[cfg(any(test, feature = "staging-fixtures"))]
use typaxis_core::Rect;
use typaxis_core::{
    push_jcs_string, sha256, DocumentFingerprint, Length, MasterId, NodeId, NonNegativeLength,
    PositiveLength, StyleFingerprint, ValidatedResourceLimits,
};
use typaxis_document::{Block, ListItem, PageRegion, PageRegionBlock, PageRegionInline};
use typaxis_layout_contract::FlowId;
use typaxis_style::{PageMasterSet, StyleValue};
use typaxis_syntax::ValidatedStagingAdvancedPackage;

pub const ADVANCED_FLOW_REGISTRY_ALGORITHM: &str = "typaxis.advanced-flow-registry/1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingAdvancedFlowOwnerKind {
    DocumentBody,
    ListItem,
    FigureCaption,
    Header,
    Footer,
}

impl StagingAdvancedFlowOwnerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DocumentBody => "document_body",
            Self::ListItem => "list_item",
            Self::FigureCaption => "figure_caption",
            Self::Header => "header",
            Self::Footer => "footer",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingPageRegionKind {
    Header,
    Footer,
}

impl StagingPageRegionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Footer => "footer",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAdvancedFlowRecord {
    flow_id: FlowId,
    owner_node_id: NodeId,
    owner_kind: StagingAdvancedFlowOwnerKind,
    parent_flow_id: Option<FlowId>,
    source_flow_id: FlowId,
    depth: u32,
    terminal: u32,
    master_id: Option<MasterId>,
}

impl StagingAdvancedFlowRecord {
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
    pub const fn owner_node_id(&self) -> NodeId {
        self.owner_node_id
    }
    pub const fn owner_kind(&self) -> StagingAdvancedFlowOwnerKind {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingRegionBlockLayout {
    node_id: NodeId,
    before_position: u32,
    after_position: u32,
    y_offset: NonNegativeLength,
    block_extent: PositiveLength,
}

impl StagingRegionBlockLayout {
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    pub const fn before_position(&self) -> u32 {
        self.before_position
    }
    pub const fn after_position(&self) -> u32 {
        self.after_position
    }
    pub const fn y_offset(&self) -> NonNegativeLength {
        self.y_offset
    }
    pub const fn block_extent(&self) -> PositiveLength {
        self.block_extent
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPageRegionLayout {
    master_id: MasterId,
    kind: StagingPageRegionKind,
    flow_id: FlowId,
    source_node_id: NodeId,
    terminal: u32,
    total_extent: NonNegativeLength,
    blocks: Vec<StagingRegionBlockLayout>,
}

impl StagingPageRegionLayout {
    pub const fn master_id(&self) -> &MasterId {
        &self.master_id
    }
    pub const fn kind(&self) -> StagingPageRegionKind {
        self.kind
    }
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
    pub const fn source_node_id(&self) -> NodeId {
        self.source_node_id
    }
    pub const fn terminal(&self) -> u32 {
        self.terminal
    }
    pub const fn total_extent(&self) -> NonNegativeLength {
        self.total_extent
    }
    pub fn blocks(&self) -> &[StagingRegionBlockLayout] {
        &self.blocks
    }
}

#[derive(Debug)]
pub struct StagingAdvancedFlowRegistryReceipt {
    package: DocumentFingerprint,
    style: StyleFingerprint,
    profile_receipt_sha256: [u8; 32],
    fingerprint: [u8; 32],
    flow_count: u32,
    body_terminal: u32,
    canonical_jcs: String,
}

impl StagingAdvancedFlowRegistryReceipt {
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
pub struct StagingHeaderFooterLayout {
    page_masters: PageMasterSet,
    advanced_page_masters: typaxis_document::AdvancedPageMasterSet,
    flows: Vec<StagingAdvancedFlowRecord>,
    regions: Vec<StagingPageRegionLayout>,
    receipt: StagingAdvancedFlowRegistryReceipt,
}

impl StagingHeaderFooterLayout {
    pub const fn page_masters(&self) -> &PageMasterSet {
        &self.page_masters
    }
    pub const fn advanced_page_masters(&self) -> &typaxis_document::AdvancedPageMasterSet {
        &self.advanced_page_masters
    }
    pub fn flows(&self) -> &[StagingAdvancedFlowRecord] {
        &self.flows
    }
    pub fn flow(&self, flow_id: FlowId) -> Option<&StagingAdvancedFlowRecord> {
        self.flows
            .get(flow_id.get() as usize)
            .filter(|flow| flow.flow_id == flow_id)
    }
    pub fn regions(&self) -> &[StagingPageRegionLayout] {
        &self.regions
    }
    pub fn region(
        &self,
        master_id: &MasterId,
        kind: StagingPageRegionKind,
    ) -> Option<&StagingPageRegionLayout> {
        self.regions
            .iter()
            .find(|region| region.master_id == *master_id && region.kind == kind)
    }
    pub const fn receipt(&self) -> &StagingAdvancedFlowRegistryReceipt {
        &self.receipt
    }

    pub fn verify_receipt(
        &self,
        package: &ValidatedStagingAdvancedPackage,
        profile_receipt_sha256: [u8; 32],
    ) -> Result<(), StagingHeaderFooterLayoutError> {
        let canonical =
            encode_registry(package, profile_receipt_sha256, &self.flows, &self.regions);
        let expected_flow_count = u32::try_from(self.flows.len())
            .map_err(|_| StagingHeaderFooterLayoutError::ReceiptMismatch)?;
        let expected_body_terminal =
            u32::try_from(package.package().package().document.blocks.len())
                .map_err(|_| StagingHeaderFooterLayoutError::ReceiptMismatch)?;
        let epoch = package.package().epoch_identity();
        if !self.registry_structure_is_valid(package)
            || self.page_masters != package.package().package().page_masters
            || self.advanced_page_masters != *package.page_masters()
            || self.receipt.package != epoch.document()
            || self.receipt.style != epoch.style()
            || self.receipt.profile_receipt_sha256 != profile_receipt_sha256
            || self.receipt.flow_count != expected_flow_count
            || self.receipt.body_terminal != expected_body_terminal
            || self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingHeaderFooterLayoutError::ReceiptMismatch);
        }
        Ok(())
    }

    fn registry_structure_is_valid(&self, package: &ValidatedStagingAdvancedPackage) -> bool {
        let document = &package.package().package().document;
        let Some(body) = self.flows.first() else {
            return false;
        };
        if body.flow_id != FlowId::DOCUMENT_BODY
            || body.owner_node_id != document.node_id
            || body.owner_kind != StagingAdvancedFlowOwnerKind::DocumentBody
            || body.parent_flow_id.is_some()
            || body.source_flow_id != FlowId::DOCUMENT_BODY
            || body.depth != 1
            || usize::try_from(body.terminal) != Ok(document.blocks.len())
            || body.master_id.is_some()
            || self
                .flows
                .iter()
                .enumerate()
                .any(|(index, flow)| usize::try_from(flow.flow_id.get()) != Ok(index))
        {
            return false;
        }

        let mut region_index = 0usize;
        for master in &package.page_masters().masters {
            for (kind, content) in [
                (
                    StagingPageRegionKind::Header,
                    master.header_content.as_ref(),
                ),
                (
                    StagingPageRegionKind::Footer,
                    master.footer_content.as_ref(),
                ),
            ] {
                let Some(content) = content else { continue };
                let Some(region) = self.regions.get(region_index) else {
                    return false;
                };
                if region.master_id != master.master_id
                    || region.kind != kind
                    || region.source_node_id != content.node_id
                    || !self.region_layout_is_valid(region)
                {
                    return false;
                }
                region_index += 1;
            }
        }
        region_index == self.regions.len()
    }

    fn region_layout_is_valid(&self, region: &StagingPageRegionLayout) -> bool {
        let Some(flow) = self.flow(region.flow_id) else {
            return false;
        };
        let expected_kind = match region.kind {
            StagingPageRegionKind::Header => StagingAdvancedFlowOwnerKind::Header,
            StagingPageRegionKind::Footer => StagingAdvancedFlowOwnerKind::Footer,
        };
        if flow.owner_node_id != region.source_node_id
            || flow.owner_kind != expected_kind
            || flow.parent_flow_id != Some(FlowId::DOCUMENT_BODY)
            || flow.source_flow_id != region.flow_id
            || flow.depth != 2
            || flow.terminal != region.terminal
            || flow.master_id.as_ref() != Some(&region.master_id)
            || usize::try_from(region.terminal) != Ok(region.blocks.len())
        {
            return false;
        }

        let mut extent = 0i64;
        for (index, block) in region.blocks.iter().enumerate() {
            let Ok(before_position) = u32::try_from(index) else {
                return false;
            };
            let Some(after_position) = before_position.checked_add(1) else {
                return false;
            };
            if block.before_position != before_position
                || block.after_position != after_position
                || block.y_offset.get().raw() != extent
            {
                return false;
            }
            let Some(next) = extent.checked_add(block.block_extent.get().raw()) else {
                return false;
            };
            extent = next;
        }
        region.total_extent.get().raw() == extent
    }
}

/// Deterministic lower-layer fixture for pagination's focused page-master
/// tests. It is feature-gated so production callers cannot use fixture facts
/// as a substitute for syntax/profile validation.
#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_header_footer_page_master_fixture() -> StagingHeaderFooterLayout {
    use typaxis_document::{
        AdvancedPageMaster, AdvancedPageMasterSet, PageProgression, PageWritingMode,
    };
    use typaxis_style::{PageMaster, PageMasterRule, PageParity};

    let positive = |raw| {
        Length::from_raw(raw)
            .and_then(PositiveLength::new)
            .expect("fixture length is positive")
    };
    let rect = |x, y, width, height| {
        Rect::new(
            Length::from_raw(x).expect("fixture x is exact"),
            Length::from_raw(y).expect("fixture y is exact"),
            positive(width),
            positive(height),
        )
    };
    let width = positive(40_000_000);
    let height = positive(50_000_000);
    let trim = rect(1_000_000, 1_000_000, 38_000_000, 48_000_000);
    let body = rect(3_000_000, 6_000_000, 34_000_000, 38_000_000);
    let header = rect(3_000_000, 2_000_000, 34_000_000, 3_000_000);
    let footer = rect(3_000_000, 45_000_000, 34_000_000, 3_000_000);
    let first_id = MasterId::new("first").expect("fixture master ID");
    let left_id = MasterId::new("left").expect("fixture master ID");
    let right_id = MasterId::new("right").expect("fixture master ID");

    let mut masters = Vec::new();
    let mut advanced_masters = Vec::new();
    let mut flows = vec![StagingAdvancedFlowRecord {
        flow_id: FlowId::DOCUMENT_BODY,
        owner_node_id: NodeId::new(0),
        owner_kind: StagingAdvancedFlowOwnerKind::DocumentBody,
        parent_flow_id: None,
        source_flow_id: FlowId::DOCUMENT_BODY,
        depth: 1,
        terminal: 7,
        master_id: None,
    }];
    let mut regions = Vec::new();
    for (master_id, header_blocks, footer_blocks) in [
        (first_id.clone(), 1u32, 0u32),
        (left_id.clone(), 0u32, 1u32),
        (right_id.clone(), 1u32, 1u32),
    ] {
        masters.push(PageMaster {
            master_id: master_id.clone(),
            width,
            height,
            body,
            header: Some(header),
            footer: Some(footer),
            footnote: None,
        });
        advanced_masters.push(AdvancedPageMaster {
            master_id: master_id.clone(),
            trim,
            header_content: None,
            footer_content: None,
            column_layout: None,
        });
        for (kind, block_count) in [
            (StagingPageRegionKind::Header, header_blocks),
            (StagingPageRegionKind::Footer, footer_blocks),
        ] {
            let flow_id = FlowId::new(u32::try_from(flows.len()).expect("fixture flow count"));
            let owner_kind = match kind {
                StagingPageRegionKind::Header => StagingAdvancedFlowOwnerKind::Header,
                StagingPageRegionKind::Footer => StagingAdvancedFlowOwnerKind::Footer,
            };
            let owner_node_id = NodeId::new(40 + flow_id.get());
            flows.push(StagingAdvancedFlowRecord {
                flow_id,
                owner_node_id,
                owner_kind,
                parent_flow_id: Some(FlowId::DOCUMENT_BODY),
                source_flow_id: flow_id,
                depth: 2,
                terminal: block_count,
                master_id: Some(master_id.clone()),
            });
            let blocks = if block_count == 0 {
                Vec::new()
            } else {
                vec![StagingRegionBlockLayout {
                    node_id: NodeId::new(80 + flow_id.get()),
                    before_position: 0,
                    after_position: 1,
                    y_offset: NonNegativeLength::ZERO,
                    block_extent: positive(1_000_000),
                }]
            };
            regions.push(StagingPageRegionLayout {
                master_id: master_id.clone(),
                kind,
                flow_id,
                source_node_id: owner_node_id,
                terminal: block_count,
                total_extent: NonNegativeLength::new(
                    Length::from_raw(i64::from(block_count) * 1_000_000).expect("fixture extent"),
                )
                .expect("fixture extent is nonnegative"),
                blocks,
            });
        }
    }

    let canonical_jcs = String::from("{\"fixture\":\"header-footer-page-masters/1\"}");
    StagingHeaderFooterLayout {
        page_masters: PageMasterSet {
            default_master_id: right_id.clone(),
            masters,
            selection_rules: vec![
                PageMasterRule {
                    master_id: first_id,
                    parity: PageParity::Any,
                    first: Some(true),
                    named_page: None,
                    source_order: 0,
                },
                PageMasterRule {
                    master_id: left_id,
                    parity: PageParity::Even,
                    first: None,
                    named_page: None,
                    source_order: 1,
                },
            ],
        },
        advanced_page_masters: AdvancedPageMasterSet {
            page_progression: PageProgression::LeftToRight,
            writing_mode: PageWritingMode::HorizontalTopToBottom,
            masters: advanced_masters,
        },
        flows,
        regions,
        receipt: StagingAdvancedFlowRegistryReceipt {
            package: DocumentFingerprint::from_untrusted_bytes([0x11; 32]),
            style: StyleFingerprint::from_untrusted_bytes([0x22; 32]),
            profile_receipt_sha256: [0x5a; 32],
            fingerprint: sha256(canonical_jcs.as_bytes()),
            flow_count: 7,
            body_terminal: 7,
            canonical_jcs,
        },
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingHeaderFooterLayoutError {
    UnsupportedContent(NodeId),
    InvalidStyle(NodeId),
    AstNodeLimit,
    FlowDepthLimit,
    ArithmeticOverflow,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingHeaderFooterLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContent(node) => {
                write!(
                    formatter,
                    "L5100: unsupported flow content at node {}",
                    node.get()
                )
            }
            Self::InvalidStyle(node) => {
                write!(
                    formatter,
                    "L5101: invalid region style at node {}",
                    node.get()
                )
            }
            Self::AstNodeLimit => formatter.write_str("P1120: advanced flow AST limit"),
            Self::FlowDepthLimit => formatter.write_str("P1121: advanced flow depth limit"),
            Self::ArithmeticOverflow => formatter.write_str("L5101: advanced layout overflow"),
            Self::AllocationFailure => formatter.write_str("L5110: advanced layout allocation"),
            Self::ReceiptMismatch => formatter.write_str("I9190: advanced registry mismatch"),
        }
    }
}

impl std::error::Error for StagingHeaderFooterLayoutError {}

/// Lower staging boundary invoked only after the CLI owner verifies the sealed
/// machine-profile receipt against `package`. The hash is retained as an
/// observation; this crate does not reimplement the profile trust decision.
pub fn layout_staging_header_footer(
    package: &ValidatedStagingAdvancedPackage,
    verified_profile_receipt_sha256: [u8; 32],
    limits: &ValidatedResourceLimits,
) -> Result<StagingHeaderFooterLayout, StagingHeaderFooterLayoutError> {
    let document = &package.package().package().document;
    let body_terminal = u32::try_from(document.blocks.len())
        .map_err(|_| StagingHeaderFooterLayoutError::AstNodeLimit)?;
    let mut flows = Vec::new();
    flows
        .try_reserve_exact(package.document_flow_capacity_hint())
        .map_err(|_| StagingHeaderFooterLayoutError::AllocationFailure)?;
    flows.push(StagingAdvancedFlowRecord {
        flow_id: FlowId::DOCUMENT_BODY,
        owner_node_id: document.node_id,
        owner_kind: StagingAdvancedFlowOwnerKind::DocumentBody,
        parent_flow_id: None,
        source_flow_id: FlowId::DOCUMENT_BODY,
        depth: 1,
        terminal: body_terminal,
        master_id: None,
    });
    collect_descendant_flows(
        &document.blocks,
        FlowId::DOCUMENT_BODY,
        1,
        &mut flows,
        limits,
    )?;

    let region_count = package
        .page_masters()
        .masters
        .iter()
        .try_fold(0usize, |count, master| {
            count
                .checked_add(usize::from(master.header_content.is_some()))
                .and_then(|value| value.checked_add(usize::from(master.footer_content.is_some())))
        })
        .ok_or(StagingHeaderFooterLayoutError::AstNodeLimit)?;
    let mut regions = Vec::new();
    regions
        .try_reserve_exact(region_count)
        .map_err(|_| StagingHeaderFooterLayoutError::AllocationFailure)?;
    for master in &package.page_masters().masters {
        for (kind, region) in [
            (
                StagingPageRegionKind::Header,
                master.header_content.as_ref(),
            ),
            (
                StagingPageRegionKind::Footer,
                master.footer_content.as_ref(),
            ),
        ] {
            let Some(region) = region else { continue };
            let owner_kind = match kind {
                StagingPageRegionKind::Header => StagingAdvancedFlowOwnerKind::Header,
                StagingPageRegionKind::Footer => StagingAdvancedFlowOwnerKind::Footer,
            };
            let terminal = u32::try_from(region.blocks.len())
                .map_err(|_| StagingHeaderFooterLayoutError::AstNodeLimit)?;
            let source_flow_id = next_flow_id(&flows)?;
            let flow_id = allocate_flow(
                &mut flows,
                region.node_id,
                owner_kind,
                FlowId::DOCUMENT_BODY,
                source_flow_id,
                2,
                terminal,
                Some(master.master_id.clone()),
                limits,
            )?;
            regions.push(layout_region(
                package,
                &master.master_id,
                kind,
                flow_id,
                region,
            )?);
        }
    }
    if flows
        .iter()
        .enumerate()
        .any(|(index, flow)| usize::try_from(flow.flow_id.get()) != Ok(index))
    {
        return Err(StagingHeaderFooterLayoutError::ReceiptMismatch);
    }
    let canonical_jcs = encode_registry(package, verified_profile_receipt_sha256, &flows, &regions);
    let fingerprint = sha256(canonical_jcs.as_bytes());
    let receipt = StagingAdvancedFlowRegistryReceipt {
        package: package.package().epoch_identity().document(),
        style: package.package().epoch_identity().style(),
        profile_receipt_sha256: verified_profile_receipt_sha256,
        fingerprint,
        flow_count: u32::try_from(flows.len())
            .map_err(|_| StagingHeaderFooterLayoutError::AstNodeLimit)?,
        body_terminal,
        canonical_jcs,
    };
    let layout = StagingHeaderFooterLayout {
        page_masters: package.package().package().page_masters.clone(),
        advanced_page_masters: package.page_masters().clone(),
        flows,
        regions,
        receipt,
    };
    layout.verify_receipt(package, verified_profile_receipt_sha256)?;
    Ok(layout)
}

trait AdvancedPackageCapacity {
    fn document_flow_capacity_hint(&self) -> usize;
}

impl AdvancedPackageCapacity for ValidatedStagingAdvancedPackage {
    fn document_flow_capacity_hint(&self) -> usize {
        self.package()
            .document_nodes()
            .node_count()
            .min(usize::MAX.saturating_sub(self.page_masters().masters.len().saturating_mul(2)))
            .saturating_add(self.page_masters().masters.len().saturating_mul(2))
    }
}

fn collect_descendant_flows(
    blocks: &[Block],
    parent: FlowId,
    parent_depth: u32,
    flows: &mut Vec<StagingAdvancedFlowRecord>,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingHeaderFooterLayoutError> {
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
                    .ok_or(StagingHeaderFooterLayoutError::FlowDepthLimit)?;
                let terminal = u32::try_from(caption.len())
                    .map_err(|_| StagingHeaderFooterLayoutError::AstNodeLimit)?;
                allocate_flow(
                    flows,
                    *node_id,
                    StagingAdvancedFlowOwnerKind::FigureCaption,
                    parent,
                    flow_id,
                    depth,
                    terminal,
                    None,
                    limits,
                )?;
                collect_descendant_flows(caption, flow_id, depth, flows, limits)?;
            }
            Block::Table { node_id, .. } => {
                return Err(StagingHeaderFooterLayoutError::UnsupportedContent(*node_id));
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
    flows: &mut Vec<StagingAdvancedFlowRecord>,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingHeaderFooterLayoutError> {
    let flow_id = next_flow_id(flows)?;
    let depth = parent_depth
        .checked_add(1)
        .ok_or(StagingHeaderFooterLayoutError::FlowDepthLimit)?;
    allocate_flow(
        flows,
        item.node_id,
        StagingAdvancedFlowOwnerKind::ListItem,
        parent,
        flow_id,
        depth,
        u32::try_from(item.blocks.len())
            .map_err(|_| StagingHeaderFooterLayoutError::AstNodeLimit)?,
        None,
        limits,
    )?;
    collect_descendant_flows(&item.blocks, flow_id, depth, flows, limits)
}

fn next_flow_id(
    flows: &[StagingAdvancedFlowRecord],
) -> Result<FlowId, StagingHeaderFooterLayoutError> {
    Ok(FlowId::new(u32::try_from(flows.len()).map_err(|_| {
        StagingHeaderFooterLayoutError::AstNodeLimit
    })?))
}

#[allow(clippy::too_many_arguments)]
fn allocate_flow(
    flows: &mut Vec<StagingAdvancedFlowRecord>,
    owner: NodeId,
    owner_kind: StagingAdvancedFlowOwnerKind,
    parent: FlowId,
    source: FlowId,
    depth: u32,
    terminal: u32,
    master_id: Option<MasterId>,
    limits: &ValidatedResourceLimits,
) -> Result<FlowId, StagingHeaderFooterLayoutError> {
    let flow_id = next_flow_id(flows)?;
    if source != flow_id
        || u64::try_from(flows.len()).map_err(|_| StagingHeaderFooterLayoutError::AstNodeLimit)?
            >= limits.get().max_ast_nodes
    {
        return Err(StagingHeaderFooterLayoutError::AstNodeLimit);
    }
    if depth > limits.get().max_ast_nesting_depth {
        return Err(StagingHeaderFooterLayoutError::FlowDepthLimit);
    }
    flows
        .try_reserve(1)
        .map_err(|_| StagingHeaderFooterLayoutError::AllocationFailure)?;
    flows.push(StagingAdvancedFlowRecord {
        flow_id,
        owner_node_id: owner,
        owner_kind,
        parent_flow_id: Some(parent),
        source_flow_id: source,
        depth,
        terminal,
        master_id,
    });
    Ok(flow_id)
}

fn layout_region(
    package: &ValidatedStagingAdvancedPackage,
    master_id: &MasterId,
    kind: StagingPageRegionKind,
    flow_id: FlowId,
    region: &PageRegion,
) -> Result<StagingPageRegionLayout, StagingHeaderFooterLayoutError> {
    let mut y = 0i64;
    let mut blocks = Vec::new();
    blocks
        .try_reserve_exact(region.blocks.len())
        .map_err(|_| StagingHeaderFooterLayoutError::AllocationFailure)?;
    for (index, block) in region.blocks.iter().enumerate() {
        let before =
            u32::try_from(index).map_err(|_| StagingHeaderFooterLayoutError::ArithmeticOverflow)?;
        let after = before
            .checked_add(1)
            .ok_or(StagingHeaderFooterLayoutError::ArithmeticOverflow)?;
        let extent = measure_region_block(package, block)?;
        let y_offset = Length::from_raw(y)
            .and_then(NonNegativeLength::new)
            .ok_or(StagingHeaderFooterLayoutError::ArithmeticOverflow)?;
        y = y
            .checked_add(extent.get().raw())
            .ok_or(StagingHeaderFooterLayoutError::ArithmeticOverflow)?;
        blocks.push(StagingRegionBlockLayout {
            node_id: block.node_id(),
            before_position: before,
            after_position: after,
            y_offset,
            block_extent: extent,
        });
    }
    let total_extent = Length::from_raw(y)
        .and_then(NonNegativeLength::new)
        .ok_or(StagingHeaderFooterLayoutError::ArithmeticOverflow)?;
    Ok(StagingPageRegionLayout {
        master_id: master_id.clone(),
        kind,
        flow_id,
        source_node_id: region.node_id,
        terminal: u32::try_from(region.blocks.len())
            .map_err(|_| StagingHeaderFooterLayoutError::ArithmeticOverflow)?,
        total_extent,
        blocks,
    })
}

fn measure_region_block(
    package: &ValidatedStagingAdvancedPackage,
    block: &PageRegionBlock,
) -> Result<PositiveLength, StagingHeaderFooterLayoutError> {
    let computed = package
        .package()
        .package()
        .style_sheet
        .cascade_basic_document(block.style_block_name(), block.classes())
        .map_err(|_| StagingHeaderFooterLayoutError::InvalidStyle(block.node_id()))?;
    let positive = |name: &str| match computed.properties().get(name) {
        Some(StyleValue::Length(value)) => PositiveLength::new(*value),
        _ => None,
    };
    let nonnegative = |name: &str| match computed.properties().get(name) {
        Some(StyleValue::Length(value)) => NonNegativeLength::new(*value),
        None => Some(NonNegativeLength::ZERO),
        Some(_) => None,
    };
    let line_height = positive("line_height").ok_or(
        StagingHeaderFooterLayoutError::InvalidStyle(block.node_id()),
    )?;
    let line_count = block
        .children()
        .iter()
        .try_fold(1i64, |count, inline| {
            if matches!(inline, PageRegionInline::HardBreak { .. }) {
                count.checked_add(1)
            } else {
                Some(count)
            }
        })
        .ok_or(StagingHeaderFooterLayoutError::ArithmeticOverflow)?;
    let lines = line_height
        .get()
        .raw()
        .checked_mul(line_count)
        .ok_or(StagingHeaderFooterLayoutError::ArithmeticOverflow)?;
    let before = nonnegative("space_before")
        .ok_or(StagingHeaderFooterLayoutError::InvalidStyle(
            block.node_id(),
        ))?
        .get()
        .raw();
    let after = nonnegative("space_after")
        .ok_or(StagingHeaderFooterLayoutError::InvalidStyle(
            block.node_id(),
        ))?
        .get()
        .raw();
    let extent = before
        .checked_add(lines)
        .and_then(|value| value.checked_add(after))
        .and_then(Length::from_raw)
        .and_then(PositiveLength::new)
        .ok_or(StagingHeaderFooterLayoutError::ArithmeticOverflow)?;
    Ok(extent)
}

fn encode_registry(
    package: &ValidatedStagingAdvancedPackage,
    profile_receipt_sha256: [u8; 32],
    flows: &[StagingAdvancedFlowRecord],
    regions: &[StagingPageRegionLayout],
) -> String {
    encode_registry_for_package(package.raw_sha256(), profile_receipt_sha256, flows, regions)
}

fn encode_registry_for_package(
    package_sha256: [u8; 32],
    profile_receipt_sha256: [u8; 32],
    flows: &[StagingAdvancedFlowRecord],
    regions: &[StagingPageRegionLayout],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, ADVANCED_FLOW_REGISTRY_ALGORITHM);
    output.push_str(",\"flows\":[");
    for (index, flow) in flows.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"depth\":");
        output.push_str(&flow.depth.to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&flow.flow_id.get().to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, flow.owner_kind.as_str());
        output.push_str(",\"master_id\":");
        match &flow.master_id {
            Some(master) => push_jcs_string(&mut output, master.as_str()),
            None => output.push_str("null"),
        }
        output.push_str(",\"owner_node_id\":");
        output.push_str(&flow.owner_node_id.get().to_string());
        output.push_str(",\"parent_flow_id\":");
        match flow.parent_flow_id {
            Some(parent) => output.push_str(&parent.get().to_string()),
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
    output.push_str(",\"regions\":[");
    for (index, region) in regions.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"blocks\":[");
        for (block_index, block) in region.blocks.iter().enumerate() {
            if block_index > 0 {
                output.push(',');
            }
            output.push_str("{\"after_position\":");
            output.push_str(&block.after_position.to_string());
            output.push_str(",\"before_position\":");
            output.push_str(&block.before_position.to_string());
            output.push_str(",\"block_extent\":");
            output.push_str(&block.block_extent.get().raw().to_string());
            output.push_str(",\"node_id\":");
            output.push_str(&block.node_id.get().to_string());
            output.push_str(",\"y_offset\":");
            output.push_str(&block.y_offset.get().raw().to_string());
            output.push('}');
        }
        output.push_str("],\"flow_id\":");
        output.push_str(&region.flow_id.get().to_string());
        output.push_str(",\"kind\":");
        push_jcs_string(&mut output, region.kind.as_str());
        output.push_str(",\"master_id\":");
        push_jcs_string(&mut output, region.master_id.as_str());
        output.push_str(",\"source_node_id\":");
        output.push_str(&region.source_node_id.get().to_string());
        output.push_str(",\"terminal\":");
        output.push_str(&region.terminal.to_string());
        output.push_str(",\"total_extent\":");
        output.push_str(&region.total_extent.get().raw().to_string());
        output.push('}');
    }
    output.push(']');
    output.push('}');
    output
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
    fn flow_registry_receipt_seals_region_measurements() {
        let mut layout = staging_header_footer_page_master_fixture();
        let package_sha256 = [0x33; 32];
        let profile_receipt_sha256 = [0x5a; 32];
        let original = encode_registry_for_package(
            package_sha256,
            profile_receipt_sha256,
            &layout.flows,
            &layout.regions,
        );

        let changed_extent = Length::from_raw(
            layout.regions[0].blocks[0]
                .block_extent
                .get()
                .raw()
                .checked_add(1)
                .unwrap(),
        )
        .and_then(PositiveLength::new)
        .unwrap();
        layout.regions[0].blocks[0].block_extent = changed_extent;
        layout.regions[0].total_extent = NonNegativeLength::new(changed_extent.get()).unwrap();
        let tampered = encode_registry_for_package(
            package_sha256,
            profile_receipt_sha256,
            &layout.flows,
            &layout.regions,
        );
        assert_ne!(original, tampered);
        assert_ne!(sha256(original.as_bytes()), sha256(tampered.as_bytes()));
    }
}
