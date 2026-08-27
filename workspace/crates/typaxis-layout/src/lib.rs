#![forbid(unsafe_code)]

mod advanced_header_footer;

#[cfg(feature = "staging-fixtures")]
pub use advanced_header_footer::staging_header_footer_page_master_fixture;
pub use advanced_header_footer::{
    layout_staging_header_footer, StagingAdvancedFlowOwnerKind, StagingAdvancedFlowRecord,
    StagingAdvancedFlowRegistryReceipt, StagingHeaderFooterLayout, StagingHeaderFooterLayoutError,
    StagingPageRegionKind, StagingPageRegionLayout, StagingRegionBlockLayout,
    ADVANCED_FLOW_REGISTRY_ALGORITHM,
};

use core::cmp::Ordering;
use core::num::{NonZeroU16, NonZeroU32};
use std::collections::{BTreeMap, BTreeSet};
use typaxis_core::{
    push_jcs_string, sha256, AnchorId, BidiLevel, DocumentFingerprint, FootnoteId,
    GeneratedBufferKey, ImageResourceId, Length, MasterId, NodeId, NonNegativeLength, PageName,
    Point, PositiveLength, Rect, StyleFingerprint, ValidatedResourceLimits,
};
use typaxis_document::{
    Block, ColumnSizing, DocumentNodeKind, FootnoteDefinition, Inline, ListItem, ReferenceFormat,
    TableCell, TableColumn, TableRow, ValidatedDocumentNodeIndex,
};
pub use typaxis_layout_contract::{
    flow_registry_fingerprint_from_jcs, footnote_flow_registry_fingerprint_from_jcs,
    footnote_page_evaluation_fingerprint_from_jcs, footnote_profile_fingerprint_from_jcs,
    footnote_selected_layout_fingerprint_from_jcs, multi_flow_selected_state_fingerprint_from_jcs,
    table_selected_layout_fingerprint_from_jcs, FlowContentKind, FlowId, FlowOwnerKind,
    FlowRegistryFingerprint, FlowTerminal, FootnoteFlowBinding, FootnoteFlowId,
    FootnoteFlowRegistryFingerprint, FootnoteFlowTerminal, FootnotePageEvaluationFingerprint,
    FootnoteProfileFingerprint, FootnoteSelectedLayoutFingerprint, LayoutEpoch, LayoutEpochError,
    LayoutTextStyleError, MachineGlyphCoverage, MachineStyleFontPreparationError,
    MachineTextSiteSource, MultiFlowSelectedStateFingerprint, PreparedMachineStyleFonts,
    PreparedMachineTextSite, ResolvedLayoutTextStyle, ResolvedTableColumn,
    ResolvedTableColumnInput, ShapeFontSelectionError, ShapeFontSelectionReceipt,
    TableGridFingerprint, TableGridReceiptError, TableGridReceiptInput, TableSection,
    TableSelectedLayoutFingerprint, TableVerticalAlignment, ValidatedTableCellBinding,
    ValidatedTableGridReceipt, ValidatedTableRowBinding,
};
use typaxis_linebreak::ValidatedParagraphItemRegistry;
use typaxis_resource_admission::{AdmittedImageMediaKind, AdmittedResourceLedger};
use typaxis_style::{
    BasicStyleBlockKind, MachineFigureWidth, MachineTextAlign, PageMaster,
    PageMasterValidationError, PageSelectionContext, PageSelectionError, StyleValue,
    TABLE_BLOCK_STYLE_REGISTRY_VERSION,
};
use typaxis_syntax::{
    MachineBlockComputedStyleReceipt, MachineTableComputedStyleReceipt,
    PackageGeneratedTextBinding, PackagePaginationContext, PackageStyleError,
    ValidatedMachinePackage, ValidatedParsedPackage, ValidatedStagingFigureUsageReceipt,
    ValidatedStagingForcedPageBreakUsageReceipt, ValidatedStagingListMarkerUsageReceipt,
    ValidatedStagingStylePackage, STAGING_BASIC_FIGURE_POLICY_VERSION,
    STAGING_BASIC_LIST_POLICY_VERSION, STAGING_FORCED_PAGE_BREAK_POLICY_VERSION,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypedStyleConsumerError {
    ArithmeticOverflow,
    IndentsExhaustInlineSize,
    ContentExceedsInlineSize,
    FigureWidthRequired,
    BlockExceedsEmptyFrame,
}

/// Typed geometry and pagination context for one staging block. Every length
/// has crossed the fixed-point constructors before this value can be built.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypedBlockLayoutInput {
    frame_inline_size: PositiveLength,
    intrinsic_inline_size: PositiveLength,
    intrinsic_block_size: PositiveLength,
    remaining_block_size: PositiveLength,
    empty_frame_block_size: PositiveLength,
    previous_space_after: NonNegativeLength,
    at_frame_start: bool,
    at_flow_end: bool,
    base_direction: BidiLevel,
}

impl TypedBlockLayoutInput {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        frame_inline_size: PositiveLength,
        intrinsic_inline_size: PositiveLength,
        intrinsic_block_size: PositiveLength,
        remaining_block_size: PositiveLength,
        empty_frame_block_size: PositiveLength,
        previous_space_after: NonNegativeLength,
        at_frame_start: bool,
        at_flow_end: bool,
        base_direction: BidiLevel,
    ) -> Self {
        Self {
            frame_inline_size,
            intrinsic_inline_size,
            intrinsic_block_size,
            remaining_block_size,
            empty_frame_block_size,
            previous_space_after,
            at_frame_start,
            at_flow_end,
            base_direction,
        }
    }
}

/// Selected typed observation handed to Display. It records the effective
/// spacing suppression/page split plus exact logical and physical placement;
/// no downstream stage needs the originating declaration names.
#[derive(Debug, Eq, PartialEq)]
pub struct SelectedTypedBlockStyle {
    owner: NodeId,
    package_sha256: [u8; 32],
    registry_version: &'static str,
    block_kind: BasicStyleBlockKind,
    frame_inline_size: PositiveLength,
    available_inline_size: PositiveLength,
    content_inline_size: PositiveLength,
    start_indent: NonNegativeLength,
    end_indent: NonNegativeLength,
    logical_start_alignment_space: NonNegativeLength,
    logical_end_alignment_space: NonNegativeLength,
    physical_left_inset: NonNegativeLength,
    effective_space_before: NonNegativeLength,
    effective_space_after: NonNegativeLength,
    page_break_before: bool,
    keep_with_next: bool,
    keep_caption: bool,
}

impl SelectedTypedBlockStyle {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn registry_version(&self) -> &'static str {
        self.registry_version
    }
    pub const fn block_kind(&self) -> BasicStyleBlockKind {
        self.block_kind
    }
    pub const fn frame_inline_size(&self) -> PositiveLength {
        self.frame_inline_size
    }
    pub const fn available_inline_size(&self) -> PositiveLength {
        self.available_inline_size
    }
    pub const fn content_inline_size(&self) -> PositiveLength {
        self.content_inline_size
    }
    pub const fn start_indent(&self) -> NonNegativeLength {
        self.start_indent
    }
    pub const fn end_indent(&self) -> NonNegativeLength {
        self.end_indent
    }
    pub const fn logical_start_alignment_space(&self) -> NonNegativeLength {
        self.logical_start_alignment_space
    }
    pub const fn logical_end_alignment_space(&self) -> NonNegativeLength {
        self.logical_end_alignment_space
    }
    pub const fn physical_left_inset(&self) -> NonNegativeLength {
        self.physical_left_inset
    }
    pub const fn effective_space_before(&self) -> NonNegativeLength {
        self.effective_space_before
    }
    pub const fn effective_space_after(&self) -> NonNegativeLength {
        self.effective_space_after
    }
    pub const fn page_break_before(&self) -> bool {
        self.page_break_before
    }
    pub const fn keep_with_next(&self) -> bool {
        self.keep_with_next
    }
    pub const fn keep_caption(&self) -> bool {
        self.keep_caption
    }
}

pub fn consume_typed_block_style(
    receipt: &MachineBlockComputedStyleReceipt,
    input: TypedBlockLayoutInput,
) -> Result<SelectedTypedBlockStyle, TypedStyleConsumerError> {
    let style = receipt.computed();
    let after_start = input
        .frame_inline_size
        .get()
        .checked_sub(style.start_indent().get())
        .ok_or(TypedStyleConsumerError::ArithmeticOverflow)?;
    let available = after_start
        .checked_sub(style.end_indent().get())
        .and_then(PositiveLength::new)
        .ok_or(TypedStyleConsumerError::IndentsExhaustInlineSize)?;
    let content = if receipt.block_kind() == BasicStyleBlockKind::Figure {
        match style.width() {
            MachineFigureWidth::Auto => return Err(TypedStyleConsumerError::FigureWidthRequired),
            MachineFigureWidth::Length(width) => width,
        }
    } else {
        input.intrinsic_inline_size
    };
    let residual = available
        .get()
        .checked_sub(content.get())
        .ok_or(TypedStyleConsumerError::ContentExceedsInlineSize)?;
    let alignment = if matches!(
        receipt.block_kind(),
        BasicStyleBlockKind::Paragraph | BasicStyleBlockKind::Heading
    ) {
        style.text_align()
    } else {
        MachineTextAlign::Start
    };
    let logical_start_raw = match alignment {
        MachineTextAlign::Start => 0,
        MachineTextAlign::End => residual.raw(),
        MachineTextAlign::Center => residual.raw() / 2,
    };
    let logical_start_alignment_space = NonNegativeLength::new(
        Length::from_raw(logical_start_raw).ok_or(TypedStyleConsumerError::ArithmeticOverflow)?,
    )
    .ok_or(TypedStyleConsumerError::ArithmeticOverflow)?;
    let logical_end_alignment_space = NonNegativeLength::new(
        residual
            .checked_sub(logical_start_alignment_space.get())
            .ok_or(TypedStyleConsumerError::ArithmeticOverflow)?,
    )
    .ok_or(TypedStyleConsumerError::ArithmeticOverflow)?;
    let physical_left = if input.base_direction.is_rtl() {
        style
            .end_indent()
            .get()
            .checked_add(logical_end_alignment_space.get())
    } else {
        style
            .start_indent()
            .get()
            .checked_add(logical_start_alignment_space.get())
    }
    .and_then(NonNegativeLength::new)
    .ok_or(TypedStyleConsumerError::ArithmeticOverflow)?;

    if input.intrinsic_block_size.get().raw() > input.empty_frame_block_size.get().raw() {
        return Err(TypedStyleConsumerError::BlockExceedsEmptyFrame);
    }
    let requested_before = if input.at_frame_start {
        NonNegativeLength::ZERO
    } else {
        NonNegativeLength::new(
            input
                .previous_space_after
                .get()
                .checked_add(style.space_before().get())
                .ok_or(TypedStyleConsumerError::ArithmeticOverflow)?,
        )
        .ok_or(TypedStyleConsumerError::ArithmeticOverflow)?
    };
    let requested_height = requested_before
        .get()
        .checked_add(input.intrinsic_block_size.get())
        .ok_or(TypedStyleConsumerError::ArithmeticOverflow)?;
    let page_break_before = requested_height.raw() > input.remaining_block_size.get().raw();
    let effective_space_before = if page_break_before {
        NonNegativeLength::ZERO
    } else {
        requested_before
    };
    let effective_space_after = if input.at_flow_end {
        NonNegativeLength::ZERO
    } else {
        style.space_after()
    };

    Ok(SelectedTypedBlockStyle {
        owner: receipt.owner(),
        package_sha256: receipt.package_fingerprint().into_bytes(),
        registry_version: receipt.registry_version(),
        block_kind: receipt.block_kind(),
        frame_inline_size: input.frame_inline_size,
        available_inline_size: available,
        content_inline_size: content,
        start_indent: style.start_indent(),
        end_indent: style.end_indent(),
        logical_start_alignment_space,
        logical_end_alignment_space,
        physical_left_inset: physical_left,
        effective_space_before,
        effective_space_after,
        page_break_before,
        keep_with_next: style.keep_with_next(),
        keep_caption: style.keep_caption(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingListItemPaintInput {
    item_owner: NodeId,
    marker_key: GeneratedBufferKey,
    marker_inline_size: PositiveLength,
    first_line_inline_size: Option<PositiveLength>,
    first_line_block_size: Option<PositiveLength>,
    painted_block_size: Option<PositiveLength>,
}

impl StagingListItemPaintInput {
    pub const fn painted(
        item_owner: NodeId,
        marker_inline_size: PositiveLength,
        first_line_inline_size: PositiveLength,
        first_line_block_size: PositiveLength,
        painted_block_size: PositiveLength,
    ) -> Self {
        Self {
            item_owner,
            marker_key: GeneratedBufferKey::new(
                item_owner,
                typaxis_core::GenerationKind::ListMarker,
                0,
            ),
            marker_inline_size,
            first_line_inline_size: Some(first_line_inline_size),
            first_line_block_size: Some(first_line_block_size),
            painted_block_size: Some(painted_block_size),
        }
    }

    pub const fn empty(item_owner: NodeId, marker_inline_size: PositiveLength) -> Self {
        Self {
            item_owner,
            marker_key: GeneratedBufferKey::new(
                item_owner,
                typaxis_core::GenerationKind::ListMarker,
                0,
            ),
            marker_inline_size,
            first_line_inline_size: None,
            first_line_block_size: None,
            painted_block_size: None,
        }
    }

    pub const fn item_owner(&self) -> NodeId {
        self.item_owner
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListLayoutInput {
    frame_inline_size: PositiveLength,
    base_direction: BidiLevel,
    items: Vec<StagingListItemPaintInput>,
}

impl StagingMachineListLayoutInput {
    pub fn new(
        frame_inline_size: PositiveLength,
        base_direction: BidiLevel,
        items: Vec<StagingListItemPaintInput>,
    ) -> Self {
        Self {
            frame_inline_size,
            base_direction,
            items,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMachineListLayoutError {
    PreflightMismatch,
    GeneratedTextMismatch,
    FlowRegistryMismatch,
    MissingMeasurement(NodeId),
    ExtraMeasurement(NodeId),
    DuplicateMeasurement(NodeId),
    EmptyPaintedItem(NodeId),
    InvalidMeasurement(NodeId),
    MissingItemFlow(NodeId),
    WrongItemFlow(NodeId),
    MissingListStyle(NodeId),
    IndentsExhaustInlineSize(NodeId),
    MarkerColumnExhaustsInlineSize(NodeId),
    FirstLineExceedsInlineSize(NodeId),
    ArithmeticOverflow,
    AllocationFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListLayoutList {
    list_owner: NodeId,
    list_flow_id: FlowId,
    marker_column_width: PositiveLength,
    marker_gap: PositiveLength,
    start_indent: NonNegativeLength,
    end_indent: NonNegativeLength,
    item_frame_inline_size: PositiveLength,
}

impl StagingMachineListLayoutList {
    pub const fn list_owner(&self) -> NodeId {
        self.list_owner
    }
    pub const fn list_flow_id(&self) -> FlowId {
        self.list_flow_id
    }
    pub const fn marker_column_width(&self) -> PositiveLength {
        self.marker_column_width
    }
    pub const fn marker_gap(&self) -> PositiveLength {
        self.marker_gap
    }
    pub const fn start_indent(&self) -> NonNegativeLength {
        self.start_indent
    }
    pub const fn end_indent(&self) -> NonNegativeLength {
        self.end_indent
    }
    pub const fn item_frame_inline_size(&self) -> PositiveLength {
        self.item_frame_inline_size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingMachineListLayoutItem {
    list_owner: NodeId,
    item_owner: NodeId,
    item_index: u32,
    list_flow_id: FlowId,
    item_flow_id: FlowId,
    marker_key: GeneratedBufferKey,
    marker_utf8: String,
    marker_inline_size: PositiveLength,
    marker_column_width: PositiveLength,
    marker_logical_start: NonNegativeLength,
    marker_physical_left: NonNegativeLength,
    content_logical_start: NonNegativeLength,
    content_physical_left: NonNegativeLength,
    content_inline_size: PositiveLength,
    first_line_inline_size: PositiveLength,
    first_line_block_size: PositiveLength,
    keep_group_block_size: PositiveLength,
    painted_block_size: PositiveLength,
}

impl StagingMachineListLayoutItem {
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
    pub const fn marker_inline_size(&self) -> PositiveLength {
        self.marker_inline_size
    }
    pub const fn marker_column_width(&self) -> PositiveLength {
        self.marker_column_width
    }
    pub const fn marker_logical_start(&self) -> NonNegativeLength {
        self.marker_logical_start
    }
    pub const fn marker_physical_left(&self) -> NonNegativeLength {
        self.marker_physical_left
    }
    pub const fn content_logical_start(&self) -> NonNegativeLength {
        self.content_logical_start
    }
    pub const fn content_physical_left(&self) -> NonNegativeLength {
        self.content_physical_left
    }
    pub const fn content_inline_size(&self) -> PositiveLength {
        self.content_inline_size
    }
    pub const fn first_line_inline_size(&self) -> PositiveLength {
        self.first_line_inline_size
    }
    pub const fn first_line_block_size(&self) -> PositiveLength {
        self.first_line_block_size
    }
    pub const fn keep_group_block_size(&self) -> PositiveLength {
        self.keep_group_block_size
    }
    pub const fn painted_block_size(&self) -> PositiveLength {
        self.painted_block_size
    }
}

/// Complete list geometry bound to canonical generated markers and MI2-02's
/// independent item-flow registry.
#[derive(Debug, Eq, PartialEq)]
pub struct StagingMachineListLayoutReceipt {
    package_sha256: [u8; 32],
    epoch: LayoutEpoch,
    flow_registry: FlowRegistryFingerprint,
    marker_usage_sha256: [u8; 32],
    policy_version: &'static str,
    frame_inline_size: PositiveLength,
    base_direction: BidiLevel,
    lists: Vec<StagingMachineListLayoutList>,
    items: Vec<StagingMachineListLayoutItem>,
}

impl StagingMachineListLayoutReceipt {
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
    pub const fn frame_inline_size(&self) -> PositiveLength {
        self.frame_inline_size
    }
    pub const fn base_direction(&self) -> BidiLevel {
        self.base_direction
    }
    pub fn lists(&self) -> &[StagingMachineListLayoutList] {
        &self.lists
    }
    pub fn items(&self) -> &[StagingMachineListLayoutItem] {
        &self.items
    }
}

pub fn layout_staging_machine_lists(
    package: &ValidatedStagingStylePackage,
    preflight: &ValidatedStagingListMarkerUsageReceipt,
    generated: PackageGeneratedTextBinding<'_>,
    ir: &ProductionFlowIr,
    mut input: StagingMachineListLayoutInput,
) -> Result<StagingMachineListLayoutReceipt, StagingMachineListLayoutError> {
    if !preflight.verifies(package)
        || preflight.policy_version() != STAGING_BASIC_LIST_POLICY_VERSION
    {
        return Err(StagingMachineListLayoutError::PreflightMismatch);
    }
    if generated.package().epoch_identity() != package.package().epoch_identity()
        || !preflight.verifies_generated_text(generated)
    {
        return Err(StagingMachineListLayoutError::GeneratedTextMismatch);
    }
    let epoch = ir.registry().receipt().epoch();
    if epoch.document() != package.package().epoch_identity().document()
        || epoch.style() != package.package().epoch_identity().style()
        || epoch.references() != generated.generated_text().reference_fingerprint()
    {
        return Err(StagingMachineListLayoutError::FlowRegistryMismatch);
    }

    input
        .items
        .sort_by_key(|measurement| measurement.item_owner);
    if let Some(pair) = input
        .items
        .windows(2)
        .find(|pair| pair[0].item_owner == pair[1].item_owner)
    {
        return Err(StagingMachineListLayoutError::DuplicateMeasurement(
            pair[1].item_owner,
        ));
    }
    let expected: std::collections::BTreeSet<_> = preflight
        .markers()
        .iter()
        .map(|marker| marker.item_owner())
        .collect();
    if let Some(measurement) = input
        .items
        .iter()
        .find(|measurement| !expected.contains(&measurement.item_owner))
    {
        return Err(StagingMachineListLayoutError::ExtraMeasurement(
            measurement.item_owner,
        ));
    }
    let mut measurements: std::collections::BTreeMap<_, _> = input
        .items
        .into_iter()
        .map(|measurement| (measurement.item_owner, measurement))
        .collect();

    let mut list_markers = std::collections::BTreeMap::<NodeId, Vec<NodeId>>::new();
    let mut item_list_owners = std::collections::BTreeMap::new();
    for marker in preflight.markers() {
        list_markers
            .entry(marker.list_owner())
            .or_default()
            .push(marker.item_owner());
        item_list_owners.insert(marker.item_owner(), marker.list_owner());
    }
    let mut lists = Vec::new();
    lists
        .try_reserve_exact(list_markers.len())
        .map_err(|_| StagingMachineListLayoutError::AllocationFailure)?;
    let mut list_geometry =
        std::collections::BTreeMap::<NodeId, StagingMachineListLayoutList>::new();
    for (list_owner, item_owners) in &list_markers {
        let style = package
            .compute_list_style(*list_owner)
            .map_err(|_| StagingMachineListLayoutError::MissingListStyle(*list_owner))?;
        let marker_column_raw = item_owners
            .iter()
            .map(|owner| {
                measurements
                    .get(owner)
                    .map(|measurement| measurement.marker_inline_size.get().raw())
                    .ok_or(StagingMachineListLayoutError::MissingMeasurement(*owner))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .ok_or(StagingMachineListLayoutError::MissingMeasurement(
                *list_owner,
            ))?;
        let marker_column_width = positive_raw(marker_column_raw)?;
        let list_flow_id = item_owners
            .first()
            .and_then(|owner| list_item_flow_record(ir, *owner))
            .map(|record| record.flow_id())
            .ok_or(StagingMachineListLayoutError::MissingItemFlow(*list_owner))?;
        if item_owners.iter().any(|owner| {
            list_item_flow_record(ir, *owner).map(|record| record.flow_id()) != Some(list_flow_id)
        }) {
            return Err(StagingMachineListLayoutError::WrongItemFlow(*list_owner));
        }
        let containing_frame = if list_flow_id == FlowId::DOCUMENT_BODY {
            input.frame_inline_size
        } else {
            let parent_item = ir
                .content_registry()
                .contents()
                .iter()
                .find(|record| {
                    record.content().kind() == FlowContentKind::ListItem
                        && record.child_flow_id() == Some(list_flow_id)
                })
                .map(|record| record.content().owner())
                .ok_or(StagingMachineListLayoutError::WrongItemFlow(*list_owner))?;
            let parent_list = item_list_owners
                .get(&parent_item)
                .and_then(|owner| list_geometry.get(owner))
                .ok_or(StagingMachineListLayoutError::WrongItemFlow(*list_owner))?;
            parent_list.item_frame_inline_size
        };
        let block = style.computed().block();
        let after_indents = containing_frame
            .get()
            .checked_sub(block.start_indent().get())
            .and_then(|value| value.checked_sub(block.end_indent().get()))
            .ok_or(StagingMachineListLayoutError::IndentsExhaustInlineSize(
                *list_owner,
            ))?;
        let marker_gap = style.computed().font_size();
        let item_frame = after_indents
            .checked_sub(marker_column_width.get())
            .and_then(|value| value.checked_sub(marker_gap.get()))
            .and_then(PositiveLength::new)
            .ok_or(StagingMachineListLayoutError::MarkerColumnExhaustsInlineSize(*list_owner))?;
        let fact = StagingMachineListLayoutList {
            list_owner: *list_owner,
            list_flow_id,
            marker_column_width,
            marker_gap,
            start_indent: block.start_indent(),
            end_indent: block.end_indent(),
            item_frame_inline_size: item_frame,
        };
        list_geometry.insert(*list_owner, fact.clone());
        lists.push(fact);
    }

    let mut items = Vec::new();
    items
        .try_reserve_exact(preflight.markers().len())
        .map_err(|_| StagingMachineListLayoutError::AllocationFailure)?;
    for marker in preflight.markers() {
        let measurement = measurements.remove(&marker.item_owner()).ok_or(
            StagingMachineListLayoutError::MissingMeasurement(marker.item_owner()),
        )?;
        if measurement.marker_key != marker.key() {
            return Err(StagingMachineListLayoutError::InvalidMeasurement(
                marker.item_owner(),
            ));
        }
        let (Some(first_line_inline_size), Some(first_line_block_size), Some(painted_block_size)) = (
            measurement.first_line_inline_size,
            measurement.first_line_block_size,
            measurement.painted_block_size,
        ) else {
            return Err(StagingMachineListLayoutError::EmptyPaintedItem(
                marker.item_owner(),
            ));
        };
        if painted_block_size.get().raw() < first_line_block_size.get().raw() {
            return Err(StagingMachineListLayoutError::InvalidMeasurement(
                marker.item_owner(),
            ));
        }
        let list = list_geometry.get(&marker.list_owner()).ok_or(
            StagingMachineListLayoutError::MissingListStyle(marker.list_owner()),
        )?;
        if first_line_inline_size.get().raw() > list.item_frame_inline_size.get().raw()
            || measurement.marker_inline_size.get().raw() > list.marker_column_width.get().raw()
        {
            return Err(StagingMachineListLayoutError::FirstLineExceedsInlineSize(
                marker.item_owner(),
            ));
        }
        let marker_logical_start_raw = list
            .start_indent
            .get()
            .raw()
            .checked_add(
                list.marker_column_width
                    .get()
                    .raw()
                    .checked_sub(measurement.marker_inline_size.get().raw())
                    .ok_or(StagingMachineListLayoutError::ArithmeticOverflow)?,
            )
            .ok_or(StagingMachineListLayoutError::ArithmeticOverflow)?;
        let content_logical_start_raw = list
            .start_indent
            .get()
            .raw()
            .checked_add(list.marker_column_width.get().raw())
            .and_then(|value| value.checked_add(list.marker_gap.get().raw()))
            .ok_or(StagingMachineListLayoutError::ArithmeticOverflow)?;
        let (marker_physical_left_raw, content_physical_left_raw) = if input.base_direction.is_rtl()
        {
            (
                input
                    .frame_inline_size
                    .get()
                    .raw()
                    .checked_sub(marker_logical_start_raw)
                    .and_then(|value| value.checked_sub(measurement.marker_inline_size.get().raw()))
                    .ok_or(StagingMachineListLayoutError::ArithmeticOverflow)?,
                input
                    .frame_inline_size
                    .get()
                    .raw()
                    .checked_sub(content_logical_start_raw)
                    .and_then(|value| value.checked_sub(list.item_frame_inline_size.get().raw()))
                    .ok_or(StagingMachineListLayoutError::ArithmeticOverflow)?,
            )
        } else {
            (marker_logical_start_raw, content_logical_start_raw)
        };
        let flow_record = list_item_flow_record(ir, marker.item_owner()).ok_or(
            StagingMachineListLayoutError::MissingItemFlow(marker.item_owner()),
        )?;
        if flow_record.flow_id() != list.list_flow_id {
            return Err(StagingMachineListLayoutError::WrongItemFlow(
                marker.item_owner(),
            ));
        }
        let item_flow_id =
            flow_record
                .child_flow_id()
                .ok_or(StagingMachineListLayoutError::MissingItemFlow(
                    marker.item_owner(),
                ))?;
        let marker_line_height = package
            .compute_list_style(marker.list_owner())
            .map_err(|_| StagingMachineListLayoutError::MissingListStyle(marker.list_owner()))?
            .computed()
            .line_height();
        let keep_group_block_size = positive_raw(
            marker_line_height
                .get()
                .raw()
                .max(first_line_block_size.get().raw()),
        )?;
        let painted_block_size = positive_raw(
            painted_block_size
                .get()
                .raw()
                .max(keep_group_block_size.get().raw()),
        )?;
        items.push(StagingMachineListLayoutItem {
            list_owner: marker.list_owner(),
            item_owner: marker.item_owner(),
            item_index: marker.item_index(),
            list_flow_id: list.list_flow_id,
            item_flow_id,
            marker_key: marker.key(),
            marker_utf8: generated_marker_utf8(generated, marker.key())
                .ok_or(StagingMachineListLayoutError::GeneratedTextMismatch)?
                .to_owned(),
            marker_inline_size: measurement.marker_inline_size,
            marker_column_width: list.marker_column_width,
            marker_logical_start: nonnegative_raw(marker_logical_start_raw)?,
            marker_physical_left: nonnegative_raw(marker_physical_left_raw)?,
            content_logical_start: nonnegative_raw(content_logical_start_raw)?,
            content_physical_left: nonnegative_raw(content_physical_left_raw)?,
            content_inline_size: list.item_frame_inline_size,
            first_line_inline_size,
            first_line_block_size,
            keep_group_block_size,
            painted_block_size,
        });
    }
    if let Some((owner, _)) = measurements.first_key_value() {
        return Err(StagingMachineListLayoutError::ExtraMeasurement(*owner));
    }
    Ok(StagingMachineListLayoutReceipt {
        package_sha256: package.package_fingerprint().into_bytes(),
        epoch,
        flow_registry: ir.registry().receipt().fingerprint(),
        marker_usage_sha256: preflight.marker_usage_sha256(),
        policy_version: preflight.policy_version(),
        frame_inline_size: input.frame_inline_size,
        base_direction: input.base_direction,
        lists,
        items,
    })
}

fn list_item_flow_record(
    ir: &ProductionFlowIr,
    owner: NodeId,
) -> Option<&ValidatedFlowContentRecord> {
    ir.content_registry().contents().iter().find(|record| {
        record.content().owner() == owner && record.content().kind() == FlowContentKind::ListItem
    })
}

fn generated_marker_utf8(
    generated: PackageGeneratedTextBinding<'_>,
    key: GeneratedBufferKey,
) -> Option<&str> {
    generated
        .generated_text()
        .buffers()
        .iter()
        .find(|buffer| buffer.key() == key)
        .map(|buffer| buffer.utf8())
}

fn positive_raw(raw: i64) -> Result<PositiveLength, StagingMachineListLayoutError> {
    Length::from_raw(raw)
        .and_then(PositiveLength::new)
        .ok_or(StagingMachineListLayoutError::ArithmeticOverflow)
}

fn nonnegative_raw(raw: i64) -> Result<NonNegativeLength, StagingMachineListLayoutError> {
    Length::from_raw(raw)
        .and_then(NonNegativeLength::new)
        .ok_or(StagingMachineListLayoutError::ArithmeticOverflow)
}

pub const STAGING_FOOTNOTE_PROFILE_ID: &str = "typaxis.machine-pdf/footnote-1";
pub const FOOTNOTE_SEPARATOR_BAND_RAW: i64 = 65_536;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingFootnoteRegistryError {
    PackageEpochMismatch,
    AstNodeLimit,
    UnsupportedBodyDomain(NodeId),
    UnsupportedDefinitionContent(NodeId),
    EmptyDefinition(FootnoteId),
    UnreferencedDefinition(FootnoteId),
    MissingDefinition(FootnoteId),
    UnknownDefinition(FootnoteId),
    DuplicateDefinition(FootnoteId),
    WrongDefinitionOwner(FootnoteId),
    WrongProfileReceipt,
    InvalidFootnoteMaster,
    EmptyDefinitionFragments(FootnoteId),
    IncompleteDefinitionFragments(FootnoteId),
    FragmentLimit,
    NonDenseFootnoteFlow,
    ArithmeticOverflow,
    AllocationFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteReferenceRecord {
    reference_owner: NodeId,
    footnote_id: FootnoteId,
    logical_ordinal: u32,
}

impl StagingFootnoteReferenceRecord {
    pub const fn reference_owner(&self) -> NodeId {
        self.reference_owner
    }

    pub const fn footnote_id(&self) -> &FootnoteId {
        &self.footnote_id
    }

    pub const fn logical_ordinal(&self) -> u32 {
        self.logical_ordinal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteDefinitionRecord {
    footnote_id: FootnoteId,
    definition_owner: NodeId,
    catalog_ordinal: u32,
    block_owners: Vec<NodeId>,
}

impl StagingFootnoteDefinitionRecord {
    pub const fn footnote_id(&self) -> &FootnoteId {
        &self.footnote_id
    }

    pub const fn definition_owner(&self) -> NodeId {
        self.definition_owner
    }

    /// One-based marker/catalog ordinal. This is intentionally independent of
    /// the zero-based page assignment ordinal used by pagination.
    pub const fn catalog_ordinal(&self) -> u32 {
        self.catalog_ordinal
    }

    pub fn block_owners(&self) -> &[NodeId] {
        &self.block_owners
    }
}

/// Package/epoch-bound proof of the private footnote profile's definition and
/// reference closure plus its fixed maximum footnote frame.
#[derive(Debug)]
pub struct StagingFootnoteProfilePreflightReceipt {
    package: DocumentFingerprint,
    epoch: LayoutEpoch,
    body_flow_registry: FlowRegistryFingerprint,
    master_id: MasterId,
    body_frame: Rect,
    maximum_footnote_frame: Rect,
    definitions: Vec<StagingFootnoteDefinitionRecord>,
    references: Vec<StagingFootnoteReferenceRecord>,
    fingerprint: FootnoteProfileFingerprint,
    canonical_jcs: String,
}

impl StagingFootnoteProfilePreflightReceipt {
    pub const fn package_fingerprint(&self) -> DocumentFingerprint {
        self.package
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub const fn body_flow_registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.body_flow_registry
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

    pub fn definitions(&self) -> &[StagingFootnoteDefinitionRecord] {
        &self.definitions
    }

    pub fn references(&self) -> &[StagingFootnoteReferenceRecord] {
        &self.references
    }

    pub const fn fingerprint(&self) -> FootnoteProfileFingerprint {
        self.fingerprint
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

/// Private MI3-06 preflight. Public profile dispatch remains unchanged; this
/// consumes only the already validated 1.2 typed package and rederives every
/// footnote-specific closure fact before any FootnoteFlowId is allocated.
pub fn preflight_staging_footnote_profile(
    package: &ValidatedParsedPackage,
    epoch: LayoutEpoch,
    body_flow_registry: FlowRegistryFingerprint,
    limits: &ValidatedResourceLimits,
) -> Result<StagingFootnoteProfilePreflightReceipt, StagingFootnoteRegistryError> {
    if epoch.document() != package.epoch_identity().document()
        || epoch.style() != package.epoch_identity().style()
    {
        return Err(StagingFootnoteRegistryError::PackageEpochMismatch);
    }
    if u64::try_from(package.document_nodes().node_count())
        .map_err(|_| StagingFootnoteRegistryError::AstNodeLimit)?
        > limits.get().max_ast_nodes
    {
        return Err(StagingFootnoteRegistryError::AstNodeLimit);
    }

    let catalog: BTreeMap<_, _> = package
        .package()
        .document
        .footnotes
        .iter()
        .map(|definition| (definition.footnote_id.clone(), definition))
        .collect();
    if catalog.len() != package.package().document.footnotes.len() {
        return Err(StagingFootnoteRegistryError::AstNodeLimit);
    }

    let mut references = Vec::new();
    references
        .try_reserve_exact(package.document_nodes().footnote_reference_targets().len())
        .map_err(|_| StagingFootnoteRegistryError::AllocationFailure)?;
    for block in &package.package().document.blocks {
        collect_body_footnote_references(package, block, &catalog, &mut references)?;
    }
    for (index, reference) in references.iter_mut().enumerate() {
        reference.logical_ordinal =
            u32::try_from(index).map_err(|_| StagingFootnoteRegistryError::AstNodeLimit)?;
    }

    let referenced: BTreeSet<_> = references
        .iter()
        .map(|reference| reference.footnote_id.clone())
        .collect();
    let mut definitions = Vec::new();
    definitions
        .try_reserve_exact(package.package().document.footnotes.len())
        .map_err(|_| StagingFootnoteRegistryError::AllocationFailure)?;
    for (index, definition) in package.package().document.footnotes.iter().enumerate() {
        if !referenced.contains(&definition.footnote_id) {
            return Err(StagingFootnoteRegistryError::UnreferencedDefinition(
                definition.footnote_id.clone(),
            ));
        }
        let block_owners = validate_footnote_definition(package, definition)?;
        let catalog_ordinal = u32::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(StagingFootnoteRegistryError::AstNodeLimit)?;
        definitions.push(StagingFootnoteDefinitionRecord {
            footnote_id: definition.footnote_id.clone(),
            definition_owner: definition.node_id,
            catalog_ordinal,
            block_owners,
        });
    }

    let masters = &package.package().page_masters;
    let [master] = masters.masters.as_slice() else {
        return Err(StagingFootnoteRegistryError::InvalidFootnoteMaster);
    };
    if masters.default_master_id != master.master_id
        || !masters.selection_rules.is_empty()
        || master.header.is_some()
        || master.footer.is_some()
    {
        return Err(StagingFootnoteRegistryError::InvalidFootnoteMaster);
    }
    let maximum_footnote_frame = master
        .footnote
        .ok_or(StagingFootnoteRegistryError::InvalidFootnoteMaster)?;
    validate_footnote_master_geometry(master.body, maximum_footnote_frame)?;

    let canonical_jcs = encode_footnote_profile_preflight(
        package.epoch_identity().document(),
        epoch,
        body_flow_registry,
        &master.master_id,
        master.body,
        maximum_footnote_frame,
        &definitions,
        &references,
    );
    let fingerprint = footnote_profile_fingerprint_from_jcs(&canonical_jcs);
    Ok(StagingFootnoteProfilePreflightReceipt {
        package: package.epoch_identity().document(),
        epoch,
        body_flow_registry,
        master_id: master.master_id.clone(),
        body_frame: master.body,
        maximum_footnote_frame,
        definitions,
        references,
        fingerprint,
        canonical_jcs,
    })
}

fn collect_body_footnote_references(
    package: &ValidatedParsedPackage,
    block: &Block,
    catalog: &BTreeMap<FootnoteId, &FootnoteDefinition>,
    references: &mut Vec<StagingFootnoteReferenceRecord>,
) -> Result<(), StagingFootnoteRegistryError> {
    match block {
        Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
            collect_inline_footnote_references(package, children, catalog, references)
        }
        Block::List { items, .. } => {
            for nested in items.iter().flat_map(|item| &item.blocks) {
                collect_body_footnote_references(package, nested, catalog, references)?;
            }
            Ok(())
        }
        Block::Figure { caption, .. } => {
            for nested in caption {
                collect_body_footnote_references(package, nested, catalog, references)?;
            }
            Ok(())
        }
        Block::PageBreak { .. } => Ok(()),
        Block::Table { node_id, .. } => Err(StagingFootnoteRegistryError::UnsupportedBodyDomain(
            *node_id,
        )),
    }
}

fn collect_inline_footnote_references(
    package: &ValidatedParsedPackage,
    inlines: &[Inline],
    catalog: &BTreeMap<FootnoteId, &FootnoteDefinition>,
    references: &mut Vec<StagingFootnoteReferenceRecord>,
) -> Result<(), StagingFootnoteRegistryError> {
    for inline in inlines {
        match inline {
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => {
                collect_inline_footnote_references(package, children, catalog, references)?;
            }
            Inline::FootnoteReference {
                node_id,
                footnote_id,
                ..
            } => {
                if package.document_nodes().node_kind(*node_id)
                    != Some(DocumentNodeKind::FootnoteReference)
                {
                    return Err(StagingFootnoteRegistryError::UnsupportedBodyDomain(
                        *node_id,
                    ));
                }
                if !catalog.contains_key(footnote_id) {
                    return Err(StagingFootnoteRegistryError::MissingDefinition(
                        footnote_id.clone(),
                    ));
                }
                references
                    .try_reserve(1)
                    .map_err(|_| StagingFootnoteRegistryError::AllocationFailure)?;
                references.push(StagingFootnoteReferenceRecord {
                    reference_owner: *node_id,
                    footnote_id: footnote_id.clone(),
                    logical_ordinal: 0,
                });
            }
            Inline::Text { .. }
            | Inline::Anchor { .. }
            | Inline::Reference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {}
        }
    }
    Ok(())
}

fn validate_footnote_definition(
    package: &ValidatedParsedPackage,
    definition: &FootnoteDefinition,
) -> Result<Vec<NodeId>, StagingFootnoteRegistryError> {
    if package.document_nodes().node_kind(definition.node_id)
        != Some(DocumentNodeKind::FootnoteDefinition)
    {
        return Err(StagingFootnoteRegistryError::UnsupportedDefinitionContent(
            definition.node_id,
        ));
    }
    if definition.blocks.is_empty() {
        return Err(StagingFootnoteRegistryError::EmptyDefinition(
            definition.footnote_id.clone(),
        ));
    }
    let mut block_owners = Vec::new();
    block_owners
        .try_reserve_exact(definition.blocks.len())
        .map_err(|_| StagingFootnoteRegistryError::AllocationFailure)?;
    let mut text_producing = false;
    for block in &definition.blocks {
        let (owner, children) = match block {
            Block::Paragraph {
                node_id, children, ..
            }
            | Block::Heading {
                node_id, children, ..
            } => (*node_id, children.as_slice()),
            _ => {
                return Err(StagingFootnoteRegistryError::UnsupportedDefinitionContent(
                    definition.node_id,
                ))
            }
        };
        block_owners.push(owner);
        validate_footnote_definition_inlines(children, false, &mut text_producing)?;
    }
    if !text_producing {
        return Err(StagingFootnoteRegistryError::EmptyDefinition(
            definition.footnote_id.clone(),
        ));
    }
    Ok(block_owners)
}

fn validate_footnote_definition_inlines(
    inlines: &[Inline],
    inside_link: bool,
    text_producing: &mut bool,
) -> Result<(), StagingFootnoteRegistryError> {
    for inline in inlines {
        match inline {
            Inline::Text { text_span, .. } => {
                if text_span.start_byte() < text_span.end_byte() {
                    *text_producing = true;
                }
            }
            Inline::Reference {
                format: ReferenceFormat::Page,
                ..
            } => {
                *text_producing = true;
            }
            Inline::Link {
                node_id, children, ..
            } if !inside_link => {
                let mut link_text_producing = false;
                validate_footnote_definition_inlines(children, true, &mut link_text_producing)
                    .map_err(|_| {
                        StagingFootnoteRegistryError::UnsupportedDefinitionContent(*node_id)
                    })?;
                if !link_text_producing {
                    return Err(StagingFootnoteRegistryError::UnsupportedDefinitionContent(
                        *node_id,
                    ));
                }
                *text_producing = true;
            }
            Inline::Anchor { .. } | Inline::SoftBreak { .. } | Inline::HardBreak { .. } => {}
            Inline::Emphasis { node_id, .. }
            | Inline::Strong { node_id, .. }
            | Inline::Link { node_id, .. }
            | Inline::Reference { node_id, .. }
            | Inline::FootnoteReference { node_id, .. } => {
                return Err(StagingFootnoteRegistryError::UnsupportedDefinitionContent(
                    *node_id,
                ));
            }
        }
    }
    Ok(())
}

fn validate_footnote_master_geometry(
    body: Rect,
    footnote: Rect,
) -> Result<(), StagingFootnoteRegistryError> {
    let body_end = body
        .y()
        .checked_add(body.height().get())
        .ok_or(StagingFootnoteRegistryError::ArithmeticOverflow)?;
    let footnote_end = footnote
        .y()
        .checked_add(footnote.height().get())
        .ok_or(StagingFootnoteRegistryError::ArithmeticOverflow)?;
    if footnote.x() != body.x()
        || footnote.width() != body.width()
        || footnote_end != body_end
        || footnote.height().get().raw() >= body.height().get().raw()
    {
        return Err(StagingFootnoteRegistryError::InvalidFootnoteMaster);
    }
    Ok(())
}

/// Worker measurement for one definition in strict flow order. Every extent
/// is one positive indivisible fragment; the first extent includes the
/// definition marker, its fixed font-size glue, and the kept first line.
#[derive(Debug)]
pub struct ValidatedStagingFootnoteDefinitionLayout {
    profile: FootnoteProfileFingerprint,
    footnote_id: FootnoteId,
    definition_owner: NodeId,
    fragment_extents: Vec<PositiveLength>,
    fragment_line_counts: Vec<NonZeroU32>,
}

impl ValidatedStagingFootnoteDefinitionLayout {
    pub const fn footnote_id(&self) -> &FootnoteId {
        &self.footnote_id
    }

    pub const fn definition_owner(&self) -> NodeId {
        self.definition_owner
    }

    pub fn fragment_extents(&self) -> &[PositiveLength] {
        &self.fragment_extents
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingFootnoteFlow {
    binding: FootnoteFlowBinding,
    catalog_ordinal: u32,
    block_owners: Vec<NodeId>,
    fragment_extents: Vec<PositiveLength>,
    fragment_line_counts: Vec<NonZeroU32>,
}

impl StagingFootnoteFlow {
    pub const fn binding(&self) -> &FootnoteFlowBinding {
        &self.binding
    }

    pub const fn catalog_ordinal(&self) -> u32 {
        self.catalog_ordinal
    }

    pub fn block_owners(&self) -> &[NodeId] {
        &self.block_owners
    }

    pub fn fragment_extents(&self) -> &[PositiveLength] {
        &self.fragment_extents
    }

    /// Number of consecutive shaped definition lines sealed into each
    /// indivisible fragment. A count greater than one represents a hard
    /// `keep_with_next` boundary (or a chain of such boundaries).
    pub fn fragment_line_counts(&self) -> &[NonZeroU32] {
        &self.fragment_line_counts
    }
}

#[derive(Debug)]
pub struct StagingFootnoteFlowRegistryReceipt {
    package: DocumentFingerprint,
    epoch: LayoutEpoch,
    profile: FootnoteProfileFingerprint,
    body_flow_registry: FlowRegistryFingerprint,
    fingerprint: FootnoteFlowRegistryFingerprint,
    flow_count: u32,
}

impl StagingFootnoteFlowRegistryReceipt {
    pub const fn package_fingerprint(&self) -> DocumentFingerprint {
        self.package
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub const fn profile_fingerprint(&self) -> FootnoteProfileFingerprint {
        self.profile
    }

    pub const fn body_flow_registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.body_flow_registry
    }

    pub const fn fingerprint(&self) -> FootnoteFlowRegistryFingerprint {
        self.fingerprint
    }

    pub const fn flow_count(&self) -> u32 {
        self.flow_count
    }
}

/// Canonical definition-flow registry. Worker registration order is discarded
/// and every lookup table is projected from the validated, fingerprinted
/// vectors rather than included as authority.
#[derive(Debug)]
pub struct StagingFootnoteFlowRegistry {
    receipt: StagingFootnoteFlowRegistryReceipt,
    master_id: MasterId,
    body_frame: Rect,
    maximum_footnote_frame: Rect,
    flows: Vec<StagingFootnoteFlow>,
    references: Vec<StagingFootnoteReferenceRecord>,
    flow_by_footnote: BTreeMap<FootnoteId, usize>,
    reference_by_owner: BTreeMap<NodeId, usize>,
    canonical_jcs: String,
}

impl StagingFootnoteFlowRegistry {
    pub const fn receipt(&self) -> &StagingFootnoteFlowRegistryReceipt {
        &self.receipt
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

    pub fn flows(&self) -> &[StagingFootnoteFlow] {
        &self.flows
    }

    pub fn flow(&self, flow_id: FootnoteFlowId) -> Option<&StagingFootnoteFlow> {
        self.flows
            .get(flow_id.get() as usize)
            .filter(|flow| flow.binding.flow_id() == flow_id)
    }

    pub fn flow_by_footnote_id(&self, footnote_id: &FootnoteId) -> Option<&StagingFootnoteFlow> {
        self.flow_by_footnote
            .get(footnote_id)
            .and_then(|index| self.flows.get(*index))
    }

    pub fn references(&self) -> &[StagingFootnoteReferenceRecord] {
        &self.references
    }

    pub fn reference(&self, reference_owner: NodeId) -> Option<&StagingFootnoteReferenceRecord> {
        self.reference_by_owner
            .get(&reference_owner)
            .and_then(|index| self.references.get(*index))
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

/// Mutable worker-registration phase for measured definition fragments.
pub struct StagingFootnoteFlowRegistryBuilder<'a> {
    preflight: &'a StagingFootnoteProfilePreflightReceipt,
    registrations: Vec<(FootnoteId, ValidatedStagingFootnoteDefinitionLayout)>,
    registered_fragment_count: u64,
    max_fragments: u64,
}

impl<'a> StagingFootnoteFlowRegistryBuilder<'a> {
    pub fn new(
        preflight: &'a StagingFootnoteProfilePreflightReceipt,
        limits: &ValidatedResourceLimits,
    ) -> Self {
        Self {
            preflight,
            registrations: Vec::new(),
            registered_fragment_count: 0,
            max_fragments: limits.get().max_fragments,
        }
    }

    pub fn expected_definition_ids(&self) -> impl ExactSizeIterator<Item = &FootnoteId> {
        self.preflight
            .definitions
            .iter()
            .map(|definition| &definition.footnote_id)
    }

    pub fn issue_definition(
        &self,
        footnote_id: &FootnoteId,
        fragment_extents: Vec<PositiveLength>,
    ) -> Result<ValidatedStagingFootnoteDefinitionLayout, StagingFootnoteRegistryError> {
        let mut fragment_line_counts = Vec::new();
        fragment_line_counts
            .try_reserve_exact(fragment_extents.len())
            .map_err(|_| StagingFootnoteRegistryError::AllocationFailure)?;
        fragment_line_counts.resize(fragment_extents.len(), NonZeroU32::MIN);
        self.issue_definition_with_line_counts(footnote_id, fragment_extents, fragment_line_counts)
    }

    pub fn issue_definition_with_line_counts(
        &self,
        footnote_id: &FootnoteId,
        fragment_extents: Vec<PositiveLength>,
        fragment_line_counts: Vec<NonZeroU32>,
    ) -> Result<ValidatedStagingFootnoteDefinitionLayout, StagingFootnoteRegistryError> {
        let definition = self
            .preflight
            .definitions
            .iter()
            .find(|definition| &definition.footnote_id == footnote_id)
            .ok_or_else(|| StagingFootnoteRegistryError::UnknownDefinition(footnote_id.clone()))?;
        if fragment_extents.is_empty() {
            return Err(StagingFootnoteRegistryError::EmptyDefinitionFragments(
                footnote_id.clone(),
            ));
        }
        let line_count = fragment_line_counts.iter().try_fold(0u64, |total, count| {
            total.checked_add(u64::from(count.get()))
        });
        if fragment_line_counts.len() != fragment_extents.len()
            || line_count.map_or(true, |count| {
                count < u64::try_from(definition.block_owners.len()).unwrap_or(u64::MAX)
            })
        {
            return Err(StagingFootnoteRegistryError::IncompleteDefinitionFragments(
                footnote_id.clone(),
            ));
        }
        if u64::try_from(fragment_extents.len())
            .map_err(|_| StagingFootnoteRegistryError::FragmentLimit)?
            > self.max_fragments
        {
            return Err(StagingFootnoteRegistryError::FragmentLimit);
        }
        Ok(ValidatedStagingFootnoteDefinitionLayout {
            profile: self.preflight.fingerprint,
            footnote_id: footnote_id.clone(),
            definition_owner: definition.definition_owner,
            fragment_extents,
            fragment_line_counts,
        })
    }

    pub fn register(
        &mut self,
        definition: ValidatedStagingFootnoteDefinitionLayout,
    ) -> Result<(), StagingFootnoteRegistryError> {
        let footnote_id = definition.footnote_id.clone();
        self.register_for(footnote_id, definition)
    }

    pub fn register_for(
        &mut self,
        registered_id: FootnoteId,
        definition: ValidatedStagingFootnoteDefinitionLayout,
    ) -> Result<(), StagingFootnoteRegistryError> {
        if definition.profile != self.preflight.fingerprint {
            return Err(StagingFootnoteRegistryError::WrongProfileReceipt);
        }
        let definition_fragment_count = u64::try_from(definition.fragment_extents.len())
            .map_err(|_| StagingFootnoteRegistryError::FragmentLimit)?;
        let registered_fragment_count = self
            .registered_fragment_count
            .checked_add(definition_fragment_count)
            .filter(|count| *count <= self.max_fragments)
            .ok_or(StagingFootnoteRegistryError::FragmentLimit)?;
        self.registrations
            .try_reserve(1)
            .map_err(|_| StagingFootnoteRegistryError::AllocationFailure)?;
        self.registrations.push((registered_id, definition));
        self.registered_fragment_count = registered_fragment_count;
        Ok(())
    }

    pub fn finish(mut self) -> Result<StagingFootnoteFlowRegistry, StagingFootnoteRegistryError> {
        self.registrations
            .sort_by(|left, right| left.0.cmp(&right.0));
        if let Some(pair) = self
            .registrations
            .windows(2)
            .find(|pair| pair[0].0 == pair[1].0)
        {
            return Err(StagingFootnoteRegistryError::DuplicateDefinition(
                pair[1].0.clone(),
            ));
        }
        let expected: BTreeSet<_> = self
            .preflight
            .definitions
            .iter()
            .map(|definition| definition.footnote_id.clone())
            .collect();
        if let Some((extra, _)) = self
            .registrations
            .iter()
            .find(|(registered, _)| !expected.contains(registered))
        {
            return Err(StagingFootnoteRegistryError::UnknownDefinition(
                extra.clone(),
            ));
        }
        let mut registered: BTreeMap<_, _> = self.registrations.into_iter().collect();
        let mut flows = Vec::new();
        flows
            .try_reserve_exact(self.preflight.definitions.len())
            .map_err(|_| StagingFootnoteRegistryError::AllocationFailure)?;
        for (index, expected) in self.preflight.definitions.iter().enumerate() {
            let measured = registered.remove(&expected.footnote_id).ok_or_else(|| {
                StagingFootnoteRegistryError::MissingDefinition(expected.footnote_id.clone())
            })?;
            if measured.footnote_id != expected.footnote_id
                || measured.definition_owner != expected.definition_owner
            {
                return Err(StagingFootnoteRegistryError::WrongDefinitionOwner(
                    expected.footnote_id.clone(),
                ));
            }
            let flow_id = FootnoteFlowId::new(
                u32::try_from(index)
                    .map_err(|_| StagingFootnoteRegistryError::NonDenseFootnoteFlow)?,
            );
            let terminal = FootnoteFlowTerminal::new(
                u32::try_from(measured.fragment_extents.len())
                    .map_err(|_| StagingFootnoteRegistryError::FragmentLimit)?,
            );
            flows.push(StagingFootnoteFlow {
                binding: FootnoteFlowBinding::new(
                    expected.footnote_id.clone(),
                    flow_id,
                    expected.definition_owner,
                    terminal,
                ),
                catalog_ordinal: expected.catalog_ordinal,
                block_owners: expected.block_owners.clone(),
                fragment_extents: measured.fragment_extents,
                fragment_line_counts: measured.fragment_line_counts,
            });
        }
        if let Some((extra, _)) = registered.first_key_value() {
            return Err(StagingFootnoteRegistryError::UnknownDefinition(
                extra.clone(),
            ));
        }

        let canonical_jcs = encode_footnote_flow_registry(self.preflight, &flows);
        let fingerprint = footnote_flow_registry_fingerprint_from_jcs(&canonical_jcs);
        let flow_count = u32::try_from(flows.len())
            .map_err(|_| StagingFootnoteRegistryError::NonDenseFootnoteFlow)?;
        let flow_by_footnote = flows
            .iter()
            .enumerate()
            .map(|(index, flow)| (flow.binding.footnote_id().clone(), index))
            .collect();
        let reference_by_owner = self
            .preflight
            .references
            .iter()
            .enumerate()
            .map(|(index, reference)| (reference.reference_owner, index))
            .collect();
        Ok(StagingFootnoteFlowRegistry {
            receipt: StagingFootnoteFlowRegistryReceipt {
                package: self.preflight.package,
                epoch: self.preflight.epoch,
                profile: self.preflight.fingerprint,
                body_flow_registry: self.preflight.body_flow_registry,
                fingerprint,
                flow_count,
            },
            master_id: self.preflight.master_id.clone(),
            body_frame: self.preflight.body_frame,
            maximum_footnote_frame: self.preflight.maximum_footnote_frame,
            flows,
            references: self.preflight.references.clone(),
            flow_by_footnote,
            reference_by_owner,
            canonical_jcs,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_footnote_profile_preflight(
    package: DocumentFingerprint,
    epoch: LayoutEpoch,
    body_flow_registry: FlowRegistryFingerprint,
    master_id: &MasterId,
    body_frame: Rect,
    maximum_footnote_frame: Rect,
    definitions: &[StagingFootnoteDefinitionRecord],
    references: &[StagingFootnoteReferenceRecord],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, FootnoteProfileFingerprint::ALGORITHM_ID);
    output.push_str(",\"body_flow_registry_sha256\":");
    push_hash_hex_jcs(&mut output, body_flow_registry.bytes());
    output.push_str(",\"body_frame\":");
    encode_footnote_rect(&mut output, body_frame);
    output.push_str(",\"contract\":\"typaxis.contract/1.2\",\"definitions\":[");
    for (index, definition) in definitions.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"block_owners\":[");
        for (block_index, owner) in definition.block_owners.iter().enumerate() {
            if block_index != 0 {
                output.push(',');
            }
            output.push_str(&owner.get().to_string());
        }
        output.push_str("],\"catalog_ordinal\":");
        output.push_str(&definition.catalog_ordinal.to_string());
        output.push_str(",\"definition_owner\":");
        output.push_str(&definition.definition_owner.get().to_string());
        output.push_str(",\"footnote_id\":");
        push_jcs_string(&mut output, definition.footnote_id.as_str());
        output.push('}');
    }
    output.push_str("],\"layout_epoch\":");
    push_layout_epoch_jcs(&mut output, epoch);
    output.push_str(",\"master_id\":");
    push_jcs_string(&mut output, master_id.as_str());
    output.push_str(",\"maximum_footnote_frame\":");
    encode_footnote_rect(&mut output, maximum_footnote_frame);
    output.push_str(",\"package_sha256\":");
    push_hash_hex_jcs(&mut output, package.bytes());
    output.push_str(",\"profile\":");
    push_jcs_string(&mut output, STAGING_FOOTNOTE_PROFILE_ID);
    output.push_str(",\"references\":[");
    for (index, reference) in references.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"footnote_id\":");
        push_jcs_string(&mut output, reference.footnote_id.as_str());
        output.push_str(",\"logical_ordinal\":");
        output.push_str(&reference.logical_ordinal.to_string());
        output.push_str(",\"reference_owner\":");
        output.push_str(&reference.reference_owner.get().to_string());
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn encode_footnote_flow_registry(
    preflight: &StagingFootnoteProfilePreflightReceipt,
    flows: &[StagingFootnoteFlow],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, FootnoteFlowRegistryFingerprint::ALGORITHM_ID);
    output.push_str(",\"body_flow_registry_sha256\":");
    push_hash_hex_jcs(&mut output, preflight.body_flow_registry.bytes());
    output.push_str(",\"flows\":[");
    for (index, flow) in flows.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"block_owners\":[");
        for (block_index, owner) in flow.block_owners.iter().enumerate() {
            if block_index != 0 {
                output.push(',');
            }
            output.push_str(&owner.get().to_string());
        }
        output.push_str("],\"catalog_ordinal\":");
        output.push_str(&flow.catalog_ordinal.to_string());
        output.push_str(",\"definition_owner\":");
        output.push_str(&flow.binding.definition_owner().get().to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&flow.binding.flow_id().get().to_string());
        output.push_str(",\"footnote_id\":");
        push_jcs_string(&mut output, flow.binding.footnote_id().as_str());
        output.push_str(",\"fragment_extents\":[");
        for (fragment_index, extent) in flow.fragment_extents.iter().enumerate() {
            if fragment_index != 0 {
                output.push(',');
            }
            output.push_str(&extent.get().raw().to_string());
        }
        output.push_str("],\"fragment_line_counts\":[");
        for (fragment_index, count) in flow.fragment_line_counts.iter().enumerate() {
            if fragment_index != 0 {
                output.push(',');
            }
            output.push_str(&count.get().to_string());
        }
        output.push_str("],\"terminal\":");
        output.push_str(&flow.binding.terminal().fragment_count().to_string());
        output.push('}');
    }
    output.push_str("],\"layout_epoch\":");
    push_layout_epoch_jcs(&mut output, preflight.epoch);
    output.push_str(",\"package_sha256\":");
    push_hash_hex_jcs(&mut output, preflight.package.bytes());
    output.push_str(",\"profile_receipt_sha256\":");
    push_hash_hex_jcs(&mut output, preflight.fingerprint.bytes());
    output.push('}');
    output
}

fn encode_footnote_rect(output: &mut String, rect: Rect) {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowRegistryError {
    UnsupportedFlowDomain,
    UnknownOwner(NodeId),
    InvalidOwnerKind(NodeId),
    PackageEpochMismatch,
    ParagraphRegistryMismatch,
    MissingParagraphContent(NodeId),
    InvalidTableGrid(NodeId),
    MissingContent(NodeId),
    ExtraContent(NodeId),
    WrongContentKind {
        owner: NodeId,
        expected: FlowContentKind,
        actual: FlowContentKind,
    },
    WrongOwner {
        registered: NodeId,
        actual: NodeId,
    },
    WrongParent(FlowId),
    WrongEpoch(NodeId),
    WrongTerminal(FlowId),
    NonDenseFlowId,
    AstNodeLimit,
    FlowDepthLimit,
    ArithmeticOverflow,
    AllocationFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableGridLayoutError {
    PackageEpochMismatch,
    WrongStyleReceipt,
    FlowRegistryMismatch,
    TableNotFound(NodeId),
    UnsupportedTablePlacement(NodeId),
    UnsupportedCellContent(NodeId),
    WrongOwner(NodeId),
    EmptyColumns(NodeId),
    EmptyRows(NodeId),
    AstNodeLimit,
    ColumnArithmetic,
    GridOutOfRange(NodeId),
    GridOverlap(NodeId),
    GridHole(NodeId),
    RowspanOutOfRange(NodeId),
    MissingCellFlow(NodeId),
    WrongCellFlow(NodeId),
    Receipt(TableGridReceiptError),
    AllocationFailure,
}

#[derive(Debug)]
struct ValidatedFlowContentReceipt {
    owner: NodeId,
    epoch: LayoutEpoch,
    boundary_count: u32,
}

#[derive(Debug)]
pub struct ValidatedParagraphFlowContent(ValidatedFlowContentReceipt);

#[derive(Debug)]
pub struct ValidatedListItemFlowContent(ValidatedFlowContentReceipt);

#[derive(Debug)]
pub struct ValidatedFigureCaptionFlowContent(ValidatedFlowContentReceipt);

#[derive(Debug)]
pub struct ValidatedPageBreakFlowContent(ValidatedFlowContentReceipt);

#[derive(Debug)]
pub struct ValidatedTableRowFlowContent(ValidatedFlowContentReceipt);

/// A worker-produced content receipt. `TableRow` is the private M3 addition;
/// footnotes and later domains still cannot enter as an unrecognized string or
/// a generic block.
#[derive(Debug)]
pub enum ValidatedFlowContent {
    Paragraph(ValidatedParagraphFlowContent),
    ListItem(ValidatedListItemFlowContent),
    FigureCaption(ValidatedFigureCaptionFlowContent),
    PageBreak(ValidatedPageBreakFlowContent),
    TableRow(ValidatedTableRowFlowContent),
}

impl ValidatedFlowContent {
    pub fn for_node(
        package: &ValidatedParsedPackage,
        paragraph_items: &ValidatedParagraphItemRegistry,
        owner: NodeId,
        epoch: LayoutEpoch,
    ) -> Result<Self, FlowRegistryError> {
        validate_flow_content_epoch(package, paragraph_items, epoch)?;
        let receipt = match package.document_nodes().node_kind(owner) {
            Some(DocumentNodeKind::Paragraph | DocumentNodeKind::Heading) => {
                let boundary_count = paragraph_items
                    .item_count(owner)
                    .ok_or(FlowRegistryError::MissingParagraphContent(owner))?;
                return Ok(Self::Paragraph(ValidatedParagraphFlowContent(
                    ValidatedFlowContentReceipt {
                        owner,
                        epoch,
                        boundary_count,
                    },
                )));
            }
            Some(DocumentNodeKind::ListItem) => ValidatedFlowContentReceipt {
                owner,
                epoch,
                boundary_count: 1,
            },
            Some(DocumentNodeKind::Figure) => ValidatedFlowContentReceipt {
                owner,
                epoch,
                boundary_count: 1,
            },
            Some(DocumentNodeKind::PageBreak) => ValidatedFlowContentReceipt {
                owner,
                epoch,
                boundary_count: 1,
            },
            Some(DocumentNodeKind::TableRow) => ValidatedFlowContentReceipt {
                owner,
                epoch,
                boundary_count: 1,
            },
            Some(_) => return Err(FlowRegistryError::InvalidOwnerKind(owner)),
            None => return Err(FlowRegistryError::UnknownOwner(owner)),
        };
        Ok(match package.document_nodes().node_kind(owner) {
            Some(DocumentNodeKind::ListItem) => {
                Self::ListItem(ValidatedListItemFlowContent(receipt))
            }
            Some(DocumentNodeKind::Figure) => {
                Self::FigureCaption(ValidatedFigureCaptionFlowContent(receipt))
            }
            Some(DocumentNodeKind::PageBreak) => {
                Self::PageBreak(ValidatedPageBreakFlowContent(receipt))
            }
            Some(DocumentNodeKind::TableRow) => {
                Self::TableRow(ValidatedTableRowFlowContent(receipt))
            }
            _ => return Err(FlowRegistryError::InvalidOwnerKind(owner)),
        })
    }

    pub const fn kind(&self) -> FlowContentKind {
        match self {
            Self::Paragraph(_) => FlowContentKind::Paragraph,
            Self::ListItem(_) => FlowContentKind::ListItem,
            Self::FigureCaption(_) => FlowContentKind::FigureCaption,
            Self::PageBreak(_) => FlowContentKind::PageBreak,
            Self::TableRow(_) => FlowContentKind::TableRow,
        }
    }

    pub const fn owner(&self) -> NodeId {
        self.receipt().owner
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.receipt().epoch
    }

    pub const fn boundary_count(&self) -> u32 {
        self.receipt().boundary_count
    }

    const fn receipt(&self) -> &ValidatedFlowContentReceipt {
        match self {
            Self::Paragraph(value) => &value.0,
            Self::ListItem(value) => &value.0,
            Self::FigureCaption(value) => &value.0,
            Self::PageBreak(value) => &value.0,
            Self::TableRow(value) => &value.0,
        }
    }
}

fn validate_flow_content_epoch(
    package: &ValidatedParsedPackage,
    paragraph_items: &ValidatedParagraphItemRegistry,
    epoch: LayoutEpoch,
) -> Result<(), FlowRegistryError> {
    if paragraph_items.epoch() != epoch {
        return Err(FlowRegistryError::ParagraphRegistryMismatch);
    }
    if epoch.document() != package.epoch_identity().document()
        || epoch.style() != package.epoch_identity().style()
    {
        return Err(FlowRegistryError::PackageEpochMismatch);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedFlowContent {
    owner: NodeId,
    block_child_path: Vec<u32>,
    flow_id: FlowId,
    kind: FlowContentKind,
    boundary_count: u32,
    child_flow_ids: Vec<FlowId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedFlow {
    flow_id: FlowId,
    owner_node_id: NodeId,
    owner_kind: FlowOwnerKind,
    parent_flow_id: Option<FlowId>,
    depth: u32,
    terminal: FlowTerminal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedFlowModel {
    flows: Vec<ExpectedFlow>,
    contents: Vec<ExpectedFlowContent>,
    node_count: u64,
    max_depth: u32,
}

enum PendingFlowNode<'a> {
    Block {
        flow_id: FlowId,
        block: &'a Block,
    },
    ListItem {
        parent_flow_id: FlowId,
        item: &'a ListItem,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ValidatedTableCellShape {
    cell_owner: NodeId,
    row_owner: NodeId,
    section: TableSection,
    row_ordinal: u32,
    column_ordinal: u32,
    colspan: NonZeroU16,
    rowspan: NonZeroU16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ValidatedTableRowShape {
    row_owner: NodeId,
    section: TableSection,
    row_ordinal: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ValidatedTableShape {
    table_owner: NodeId,
    rows: Vec<ValidatedTableRowShape>,
    cells: Vec<ValidatedTableCellShape>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ValidatedTableProfileShapes {
    effective_node_count: u64,
    tables: Vec<ValidatedTableShape>,
}

impl ValidatedTableProfileShapes {
    fn table(&self, owner: NodeId) -> Option<&ValidatedTableShape> {
        self.tables.iter().find(|table| table.table_owner == owner)
    }
}

/// Validates the complete private table domain before any cell FlowId or cell
/// content work is allocated. The first pass charges every NodeId-less column;
/// only after that inclusive limit succeeds are one-dimensional grid vectors
/// constructed.
fn prevalidate_table_profile(
    package: &ValidatedParsedPackage,
    limits: &ValidatedResourceLimits,
) -> Result<ValidatedTableProfileShapes, TableGridLayoutError> {
    let mut column_count = 0u64;
    for block in &package.package().document.blocks {
        count_table_columns(block, true, &mut column_count)?;
    }
    let semantic_node_count = u64::try_from(package.document_nodes().node_count())
        .map_err(|_| TableGridLayoutError::AstNodeLimit)?;
    let effective_node_count = semantic_node_count
        .checked_add(column_count)
        .ok_or(TableGridLayoutError::AstNodeLimit)?;
    if effective_node_count > limits.get().max_ast_nodes {
        return Err(TableGridLayoutError::AstNodeLimit);
    }

    let table_count = package
        .package()
        .document
        .blocks
        .iter()
        .filter(|block| matches!(block, Block::Table { .. }))
        .count();
    let mut tables = Vec::new();
    tables
        .try_reserve_exact(table_count)
        .map_err(|_| TableGridLayoutError::AllocationFailure)?;
    for block in &package.package().document.blocks {
        let Block::Table {
            node_id,
            columns,
            head,
            body,
            ..
        } = block
        else {
            continue;
        };
        if package.document_nodes().node_kind(*node_id) != Some(DocumentNodeKind::Table) {
            return Err(TableGridLayoutError::WrongOwner(*node_id));
        }
        if columns.is_empty() {
            return Err(TableGridLayoutError::EmptyColumns(*node_id));
        }
        if head.is_empty() && body.is_empty() {
            return Err(TableGridLayoutError::EmptyRows(*node_id));
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(head.len().saturating_add(body.len()))
            .map_err(|_| TableGridLayoutError::AllocationFailure)?;
        let mut cells = Vec::new();
        let cell_count = head
            .iter()
            .chain(body)
            .try_fold(0usize, |total, row| total.checked_add(row.cells.len()))
            .ok_or(TableGridLayoutError::AstNodeLimit)?;
        cells
            .try_reserve_exact(cell_count)
            .map_err(|_| TableGridLayoutError::AllocationFailure)?;
        validate_table_section_shape(
            package,
            *node_id,
            TableSection::Head,
            head,
            columns.len(),
            &mut rows,
            &mut cells,
        )?;
        validate_table_section_shape(
            package,
            *node_id,
            TableSection::Body,
            body,
            columns.len(),
            &mut rows,
            &mut cells,
        )?;
        tables.push(ValidatedTableShape {
            table_owner: *node_id,
            rows,
            cells,
        });
    }
    Ok(ValidatedTableProfileShapes {
        effective_node_count,
        tables,
    })
}

fn count_table_columns(
    block: &Block,
    document_body: bool,
    total: &mut u64,
) -> Result<(), TableGridLayoutError> {
    match block {
        Block::Table {
            node_id,
            columns,
            head,
            body,
            ..
        } => {
            if !document_body {
                return Err(TableGridLayoutError::UnsupportedTablePlacement(*node_id));
            }
            *total = total
                .checked_add(
                    u64::try_from(columns.len()).map_err(|_| TableGridLayoutError::AstNodeLimit)?,
                )
                .ok_or(TableGridLayoutError::AstNodeLimit)?;
            for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                for nested in &cell.blocks {
                    let Block::Paragraph { children, .. } = nested else {
                        return Err(TableGridLayoutError::UnsupportedCellContent(cell.node_id));
                    };
                    if children.iter().any(|inline| {
                        !matches!(
                            inline,
                            Inline::Text { .. }
                                | Inline::SoftBreak { .. }
                                | Inline::HardBreak { .. }
                        )
                    }) {
                        return Err(TableGridLayoutError::UnsupportedCellContent(cell.node_id));
                    }
                }
            }
            Ok(())
        }
        Block::List { items, .. } => {
            for nested in items.iter().flat_map(|item| &item.blocks) {
                count_table_columns(nested, false, total)?;
            }
            Ok(())
        }
        Block::Figure { caption, .. } => {
            for nested in caption {
                count_table_columns(nested, false, total)?;
            }
            Ok(())
        }
        Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_table_section_shape(
    package: &ValidatedParsedPackage,
    table_owner: NodeId,
    section: TableSection,
    section_rows: &[TableRow],
    column_count: usize,
    rows: &mut Vec<ValidatedTableRowShape>,
    cells: &mut Vec<ValidatedTableCellShape>,
) -> Result<(), TableGridLayoutError> {
    if section_rows.is_empty() {
        return Ok(());
    }
    let section_row_count =
        u32::try_from(section_rows.len()).map_err(|_| TableGridLayoutError::AstNodeLimit)?;
    let mut remaining = Vec::new();
    remaining
        .try_reserve_exact(column_count)
        .map_err(|_| TableGridLayoutError::AllocationFailure)?;
    remaining.resize(column_count, 0u16);
    for (row_index, row) in section_rows.iter().enumerate() {
        let row_ordinal =
            u32::try_from(row_index).map_err(|_| TableGridLayoutError::AstNodeLimit)?;
        if package.document_nodes().node_kind(row.node_id) != Some(DocumentNodeKind::TableRow) {
            return Err(TableGridLayoutError::WrongOwner(row.node_id));
        }
        rows.push(ValidatedTableRowShape {
            row_owner: row.node_id,
            section,
            row_ordinal,
        });
        for cell in &row.cells {
            if package.document_nodes().node_kind(cell.node_id) != Some(DocumentNodeKind::TableCell)
            {
                return Err(TableGridLayoutError::WrongOwner(cell.node_id));
            }
            let Some(origin) = remaining.iter().position(|value| *value == 0) else {
                return Err(TableGridLayoutError::GridOutOfRange(cell.node_id));
            };
            let end = origin
                .checked_add(usize::from(cell.colspan.get()))
                .ok_or(TableGridLayoutError::ColumnArithmetic)?;
            if end > column_count {
                return Err(TableGridLayoutError::GridOutOfRange(cell.node_id));
            }
            if remaining[origin..end].iter().any(|value| *value != 0) {
                return Err(TableGridLayoutError::GridOverlap(cell.node_id));
            }
            let end_row = row_ordinal
                .checked_add(u32::from(cell.rowspan.get()))
                .ok_or(TableGridLayoutError::ColumnArithmetic)?;
            if end_row > section_row_count {
                return Err(TableGridLayoutError::RowspanOutOfRange(cell.node_id));
            }
            for value in &mut remaining[origin..end] {
                *value = cell.rowspan.get();
            }
            cells.push(ValidatedTableCellShape {
                cell_owner: cell.node_id,
                row_owner: row.node_id,
                section,
                row_ordinal,
                column_ordinal: u32::try_from(origin)
                    .map_err(|_| TableGridLayoutError::AstNodeLimit)?,
                colspan: cell.colspan,
                rowspan: cell.rowspan,
            });
        }
        if remaining.contains(&0) {
            return Err(TableGridLayoutError::GridHole(row.node_id));
        }
        for value in &mut remaining {
            *value -= 1;
        }
    }
    if remaining.iter().any(|value| *value != 0) {
        return Err(TableGridLayoutError::RowspanOutOfRange(table_owner));
    }
    Ok(())
}

fn table_profile_flow_error(error: TableGridLayoutError) -> FlowRegistryError {
    match error {
        TableGridLayoutError::AstNodeLimit => FlowRegistryError::AstNodeLimit,
        TableGridLayoutError::ColumnArithmetic => FlowRegistryError::ArithmeticOverflow,
        TableGridLayoutError::AllocationFailure => FlowRegistryError::AllocationFailure,
        TableGridLayoutError::UnsupportedTablePlacement(_)
        | TableGridLayoutError::UnsupportedCellContent(_) => {
            FlowRegistryError::UnsupportedFlowDomain
        }
        TableGridLayoutError::EmptyColumns(owner)
        | TableGridLayoutError::EmptyRows(owner)
        | TableGridLayoutError::WrongOwner(owner)
        | TableGridLayoutError::GridOutOfRange(owner)
        | TableGridLayoutError::GridOverlap(owner)
        | TableGridLayoutError::GridHole(owner)
        | TableGridLayoutError::RowspanOutOfRange(owner) => {
            FlowRegistryError::InvalidTableGrid(owner)
        }
        _ => FlowRegistryError::UnsupportedFlowDomain,
    }
}

fn derive_expected_flow_model(
    package: &ValidatedParsedPackage,
    paragraph_items: &ValidatedParagraphItemRegistry,
    epoch: LayoutEpoch,
    limits: &ValidatedResourceLimits,
    separate_footnote_flows: bool,
) -> Result<ExpectedFlowModel, FlowRegistryError> {
    validate_flow_content_epoch(package, paragraph_items, epoch)?;
    if !separate_footnote_flows && !package.package().document.footnotes.is_empty() {
        return Err(FlowRegistryError::UnsupportedFlowDomain);
    }
    let table_shapes =
        prevalidate_table_profile(package, limits).map_err(table_profile_flow_error)?;
    let node_count = table_shapes.effective_node_count;
    let root = package.package().document.node_id;
    if root != NodeId::new(0)
        || package.document_nodes().node_kind(root) != Some(DocumentNodeKind::Document)
        || package.document_nodes().node_path(root) != Some([].as_slice())
    {
        return Err(FlowRegistryError::UnknownOwner(root));
    }
    if limits.get().max_ast_nesting_depth < 1 {
        return Err(FlowRegistryError::FlowDepthLimit);
    }

    let mut flows = Vec::new();
    flows
        .try_reserve_exact(1)
        .map_err(|_| FlowRegistryError::AllocationFailure)?;
    flows.push(ExpectedFlow {
        flow_id: FlowId::DOCUMENT_BODY,
        owner_node_id: root,
        owner_kind: FlowOwnerKind::DocumentBody,
        parent_flow_id: None,
        depth: 1,
        terminal: FlowTerminal::new(0),
    });
    let mut contents = Vec::new();
    let mut pending = Vec::new();
    pending
        .try_reserve_exact(package.package().document.blocks.len())
        .map_err(|_| FlowRegistryError::AllocationFailure)?;
    pending.extend(package.package().document.blocks.iter().rev().map(|block| {
        PendingFlowNode::Block {
            flow_id: FlowId::DOCUMENT_BODY,
            block,
        }
    }));

    let mut max_depth = 1u32;
    while let Some(next) = pending.pop() {
        match next {
            PendingFlowNode::Block { flow_id, block } => match block {
                Block::Paragraph { node_id, .. } | Block::Heading { node_id, .. } => {
                    let boundary_count = paragraph_items
                        .item_count(*node_id)
                        .ok_or(FlowRegistryError::MissingParagraphContent(*node_id))?;
                    push_expected_content(
                        package,
                        &mut flows,
                        &mut contents,
                        limits,
                        *node_id,
                        flow_id,
                        FlowContentKind::Paragraph,
                        boundary_count,
                        Vec::new(),
                    )?;
                }
                Block::List { items, .. } => {
                    pending
                        .try_reserve(items.len())
                        .map_err(|_| FlowRegistryError::AllocationFailure)?;
                    pending.extend(items.iter().rev().map(|item| PendingFlowNode::ListItem {
                        parent_flow_id: flow_id,
                        item,
                    }));
                }
                Block::Figure {
                    node_id, caption, ..
                } => {
                    let child_flow_id = allocate_expected_flow(
                        &mut flows,
                        limits,
                        node_count,
                        *node_id,
                        FlowOwnerKind::FigureCaption,
                        flow_id,
                    )?;
                    max_depth = max_depth.max(flows[child_flow_id.get() as usize].depth);
                    push_expected_content(
                        package,
                        &mut flows,
                        &mut contents,
                        limits,
                        *node_id,
                        flow_id,
                        FlowContentKind::FigureCaption,
                        1,
                        vec![child_flow_id],
                    )?;
                    pending
                        .try_reserve(caption.len())
                        .map_err(|_| FlowRegistryError::AllocationFailure)?;
                    pending.extend(caption.iter().rev().map(|block| PendingFlowNode::Block {
                        flow_id: child_flow_id,
                        block,
                    }));
                }
                Block::PageBreak { node_id, .. } => {
                    push_expected_content(
                        package,
                        &mut flows,
                        &mut contents,
                        limits,
                        *node_id,
                        flow_id,
                        FlowContentKind::PageBreak,
                        1,
                        Vec::new(),
                    )?;
                }
                Block::Table {
                    node_id,
                    head,
                    body,
                    ..
                } => {
                    if flow_id != FlowId::DOCUMENT_BODY {
                        return Err(FlowRegistryError::UnsupportedFlowDomain);
                    }
                    let shape = table_shapes
                        .table(*node_id)
                        .ok_or(FlowRegistryError::InvalidTableGrid(*node_id))?;
                    let ast_cells: Vec<&TableCell> =
                        head.iter().chain(body).flat_map(|row| &row.cells).collect();
                    if ast_cells.len() != shape.cells.len()
                        || ast_cells
                            .iter()
                            .zip(&shape.cells)
                            .any(|(cell, expected)| cell.node_id != expected.cell_owner)
                    {
                        return Err(FlowRegistryError::InvalidTableGrid(*node_id));
                    }
                    let mut cell_flows = Vec::new();
                    cell_flows
                        .try_reserve_exact(ast_cells.len())
                        .map_err(|_| FlowRegistryError::AllocationFailure)?;
                    for cell in &ast_cells {
                        let child_flow_id = allocate_expected_flow(
                            &mut flows,
                            limits,
                            node_count,
                            cell.node_id,
                            FlowOwnerKind::TableCell,
                            flow_id,
                        )?;
                        max_depth = max_depth.max(flows[child_flow_id.get() as usize].depth);
                        cell_flows.push(child_flow_id);
                    }

                    let mut cell_offset = 0usize;
                    for row in head.iter().chain(body) {
                        let end = cell_offset
                            .checked_add(row.cells.len())
                            .ok_or(FlowRegistryError::ArithmeticOverflow)?;
                        let mut row_cell_flows = Vec::new();
                        row_cell_flows
                            .try_reserve_exact(row.cells.len())
                            .map_err(|_| FlowRegistryError::AllocationFailure)?;
                        row_cell_flows.extend_from_slice(&cell_flows[cell_offset..end]);
                        push_expected_content(
                            package,
                            &mut flows,
                            &mut contents,
                            limits,
                            row.node_id,
                            flow_id,
                            FlowContentKind::TableRow,
                            1,
                            row_cell_flows,
                        )?;
                        cell_offset = end;
                    }
                    if cell_offset != cell_flows.len() {
                        return Err(FlowRegistryError::InvalidTableGrid(*node_id));
                    }

                    let additional = ast_cells.iter().try_fold(0usize, |total, cell| {
                        total
                            .checked_add(cell.blocks.len())
                            .ok_or(FlowRegistryError::ArithmeticOverflow)
                    })?;
                    pending
                        .try_reserve(additional)
                        .map_err(|_| FlowRegistryError::AllocationFailure)?;
                    for (cell, child_flow_id) in ast_cells.iter().zip(&cell_flows).rev() {
                        pending.extend(cell.blocks.iter().rev().map(|block| {
                            PendingFlowNode::Block {
                                flow_id: *child_flow_id,
                                block,
                            }
                        }));
                    }
                }
            },
            PendingFlowNode::ListItem {
                parent_flow_id,
                item,
            } => {
                let child_flow_id = allocate_expected_flow(
                    &mut flows,
                    limits,
                    node_count,
                    item.node_id,
                    FlowOwnerKind::ListItem,
                    parent_flow_id,
                )?;
                max_depth = max_depth.max(flows[child_flow_id.get() as usize].depth);
                push_expected_content(
                    package,
                    &mut flows,
                    &mut contents,
                    limits,
                    item.node_id,
                    parent_flow_id,
                    FlowContentKind::ListItem,
                    1,
                    vec![child_flow_id],
                )?;
                pending
                    .try_reserve(item.blocks.len())
                    .map_err(|_| FlowRegistryError::AllocationFailure)?;
                pending.extend(
                    item.blocks
                        .iter()
                        .rev()
                        .map(|block| PendingFlowNode::Block {
                            flow_id: child_flow_id,
                            block,
                        }),
                );
            }
        }
    }
    if u64::try_from(flows.len()).map_err(|_| FlowRegistryError::AstNodeLimit)? > node_count
        || max_depth > limits.get().max_ast_nesting_depth
    {
        return Err(FlowRegistryError::FlowDepthLimit);
    }
    Ok(ExpectedFlowModel {
        flows,
        contents,
        node_count,
        max_depth,
    })
}

fn allocate_expected_flow(
    flows: &mut Vec<ExpectedFlow>,
    limits: &ValidatedResourceLimits,
    admitted_node_count: u64,
    owner_node_id: NodeId,
    owner_kind: FlowOwnerKind,
    parent_flow_id: FlowId,
) -> Result<FlowId, FlowRegistryError> {
    let next_count = u64::try_from(flows.len())
        .map_err(|_| FlowRegistryError::AstNodeLimit)?
        .checked_add(1)
        .ok_or(FlowRegistryError::AstNodeLimit)?;
    if next_count > limits.get().max_ast_nodes || next_count > admitted_node_count {
        return Err(FlowRegistryError::AstNodeLimit);
    }
    let parent = flows
        .get(parent_flow_id.get() as usize)
        .ok_or(FlowRegistryError::WrongParent(parent_flow_id))?;
    let depth = parent
        .depth
        .checked_add(1)
        .ok_or(FlowRegistryError::FlowDepthLimit)?;
    if depth > limits.get().max_ast_nesting_depth {
        return Err(FlowRegistryError::FlowDepthLimit);
    }
    let flow_id =
        FlowId::new(u32::try_from(flows.len()).map_err(|_| FlowRegistryError::AstNodeLimit)?);
    flows
        .try_reserve(1)
        .map_err(|_| FlowRegistryError::AllocationFailure)?;
    flows.push(ExpectedFlow {
        flow_id,
        owner_node_id,
        owner_kind,
        parent_flow_id: Some(parent_flow_id),
        depth,
        terminal: FlowTerminal::new(0),
    });
    Ok(flow_id)
}

#[allow(clippy::too_many_arguments)]
fn push_expected_content(
    package: &ValidatedParsedPackage,
    flows: &mut [ExpectedFlow],
    contents: &mut Vec<ExpectedFlowContent>,
    limits: &ValidatedResourceLimits,
    owner: NodeId,
    flow_id: FlowId,
    kind: FlowContentKind,
    boundary_count: u32,
    child_flow_ids: Vec<FlowId>,
) -> Result<(), FlowRegistryError> {
    let next_count = u64::try_from(contents.len())
        .map_err(|_| FlowRegistryError::AstNodeLimit)?
        .checked_add(1)
        .ok_or(FlowRegistryError::AstNodeLimit)?;
    if next_count > limits.get().max_ast_nodes {
        return Err(FlowRegistryError::AstNodeLimit);
    }
    let flow = flows
        .get_mut(flow_id.get() as usize)
        .ok_or(FlowRegistryError::WrongParent(flow_id))?;
    let next_terminal = flow
        .terminal
        .owner_local_ordinal()
        .checked_add(boundary_count)
        .ok_or(FlowRegistryError::ArithmeticOverflow)?;
    let block_child_path = package
        .document_nodes()
        .node_path(owner)
        .ok_or(FlowRegistryError::UnknownOwner(owner))?
        .to_vec();
    contents
        .try_reserve(1)
        .map_err(|_| FlowRegistryError::AllocationFailure)?;
    contents.push(ExpectedFlowContent {
        owner,
        block_child_path,
        flow_id,
        kind,
        boundary_count,
        child_flow_ids,
    });
    flow.terminal = FlowTerminal::new(next_terminal);
    Ok(())
}

/// Mutable, untrusted registration stage. `finish` discards insertion order
/// and compares every receipt with the package-derived canonical model.
pub struct ValidatedFlowContentRegistryBuilder<'a> {
    package: &'a ValidatedParsedPackage,
    paragraph_items: &'a ValidatedParagraphItemRegistry,
    epoch: LayoutEpoch,
    model: ExpectedFlowModel,
    registrations: Vec<(NodeId, ValidatedFlowContent)>,
    max_ast_nodes: u64,
    max_ast_nesting_depth: u32,
}

impl<'a> ValidatedFlowContentRegistryBuilder<'a> {
    pub fn new(
        package: &'a ValidatedParsedPackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
        epoch: LayoutEpoch,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, FlowRegistryError> {
        Self::new_internal(package, paragraph_items, epoch, limits, false)
    }

    pub fn new_for_footnote_body(
        package: &'a ValidatedParsedPackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
        epoch: LayoutEpoch,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, FlowRegistryError> {
        Self::new_internal(package, paragraph_items, epoch, limits, true)
    }

    fn new_internal(
        package: &'a ValidatedParsedPackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
        epoch: LayoutEpoch,
        limits: &ValidatedResourceLimits,
        separate_footnote_flows: bool,
    ) -> Result<Self, FlowRegistryError> {
        let model = derive_expected_flow_model(
            package,
            paragraph_items,
            epoch,
            limits,
            separate_footnote_flows,
        )?;
        Ok(Self {
            package,
            paragraph_items,
            epoch,
            model,
            registrations: Vec::new(),
            max_ast_nodes: limits.get().max_ast_nodes,
            max_ast_nesting_depth: limits.get().max_ast_nesting_depth,
        })
    }

    pub fn expected_content_owners(&self) -> impl ExactSizeIterator<Item = NodeId> + '_ {
        self.model.contents.iter().map(|content| content.owner)
    }

    pub fn issue_content(&self, owner: NodeId) -> Result<ValidatedFlowContent, FlowRegistryError> {
        ValidatedFlowContent::for_node(self.package, self.paragraph_items, owner, self.epoch)
    }

    pub fn register(&mut self, content: ValidatedFlowContent) -> Result<(), FlowRegistryError> {
        let owner = content.owner();
        self.register_for(owner, content)
    }

    pub fn register_for(
        &mut self,
        registered_owner: NodeId,
        content: ValidatedFlowContent,
    ) -> Result<(), FlowRegistryError> {
        let next_count = u64::try_from(self.registrations.len())
            .map_err(|_| FlowRegistryError::AstNodeLimit)?
            .checked_add(1)
            .ok_or(FlowRegistryError::AstNodeLimit)?;
        if next_count > self.max_ast_nodes {
            return Err(FlowRegistryError::AstNodeLimit);
        }
        self.registrations
            .try_reserve(1)
            .map_err(|_| FlowRegistryError::AllocationFailure)?;
        self.registrations.push((registered_owner, content));
        Ok(())
    }

    pub fn finish(mut self) -> Result<ValidatedFlowContentRegistry, FlowRegistryError> {
        validate_flow_content_epoch(self.package, self.paragraph_items, self.epoch)?;
        if self.model.node_count > self.max_ast_nodes
            || u64::try_from(self.model.flows.len()).map_err(|_| FlowRegistryError::AstNodeLimit)?
                > self.model.node_count
        {
            return Err(FlowRegistryError::AstNodeLimit);
        }
        if self.model.max_depth > self.max_ast_nesting_depth {
            return Err(FlowRegistryError::FlowDepthLimit);
        }
        self.registrations.sort_by_key(|(owner, _)| *owner);
        if let Some(pair) = self
            .registrations
            .windows(2)
            .find(|pair| pair[0].0 == pair[1].0)
        {
            return Err(FlowRegistryError::ExtraContent(pair[1].0));
        }
        let expected_owners: std::collections::BTreeSet<_> = self
            .model
            .contents
            .iter()
            .map(|content| content.owner)
            .collect();
        if let Some((owner, _)) = self
            .registrations
            .iter()
            .find(|(owner, _)| !expected_owners.contains(owner))
        {
            return Err(FlowRegistryError::ExtraContent(*owner));
        }
        let mut registered: std::collections::BTreeMap<_, _> =
            self.registrations.into_iter().collect();
        let mut contents = Vec::new();
        contents
            .try_reserve_exact(self.model.contents.len())
            .map_err(|_| FlowRegistryError::AllocationFailure)?;
        for expected in &self.model.contents {
            let content = registered
                .remove(&expected.owner)
                .ok_or(FlowRegistryError::MissingContent(expected.owner))?;
            if content.owner() != expected.owner {
                return Err(FlowRegistryError::WrongOwner {
                    registered: expected.owner,
                    actual: content.owner(),
                });
            }
            if content.epoch() != self.epoch {
                return Err(FlowRegistryError::WrongEpoch(expected.owner));
            }
            if content.kind() != expected.kind {
                return Err(FlowRegistryError::WrongContentKind {
                    owner: expected.owner,
                    expected: expected.kind,
                    actual: content.kind(),
                });
            }
            if content.boundary_count() != expected.boundary_count {
                return Err(FlowRegistryError::WrongTerminal(expected.flow_id));
            }
            contents.push(ValidatedFlowContentRecord {
                content,
                block_child_path: expected.block_child_path.clone(),
                flow_id: expected.flow_id,
                child_flow_ids: expected.child_flow_ids.clone(),
            });
        }
        if let Some((owner, _)) = registered.first_key_value() {
            return Err(FlowRegistryError::ExtraContent(*owner));
        }
        Ok(ValidatedFlowContentRegistry {
            package: self.package.epoch_identity().document(),
            epoch: self.epoch,
            contents,
            model: self.model,
            max_ast_nodes: self.max_ast_nodes,
            max_ast_nesting_depth: self.max_ast_nesting_depth,
        })
    }
}

#[derive(Debug)]
pub struct ValidatedFlowContentRecord {
    content: ValidatedFlowContent,
    block_child_path: Vec<u32>,
    flow_id: FlowId,
    child_flow_ids: Vec<FlowId>,
}

impl ValidatedFlowContentRecord {
    pub const fn content(&self) -> &ValidatedFlowContent {
        &self.content
    }

    pub fn block_child_path(&self) -> &[u32] {
        &self.block_child_path
    }

    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }

    pub fn child_flow_id(&self) -> Option<FlowId> {
        (self.child_flow_ids.len() == 1).then(|| self.child_flow_ids[0])
    }

    pub fn child_flow_ids(&self) -> &[FlowId] {
        &self.child_flow_ids
    }
}

/// Complete package/epoch-bound content registry. It has no raw-parts
/// constructor and deliberately is not `Clone`.
#[derive(Debug)]
pub struct ValidatedFlowContentRegistry {
    package: DocumentFingerprint,
    epoch: LayoutEpoch,
    contents: Vec<ValidatedFlowContentRecord>,
    model: ExpectedFlowModel,
    max_ast_nodes: u64,
    max_ast_nesting_depth: u32,
}

impl ValidatedFlowContentRegistry {
    pub const fn package_fingerprint(&self) -> DocumentFingerprint {
        self.package
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub fn contents(&self) -> &[ValidatedFlowContentRecord] {
        &self.contents
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedFlow {
    flow_id: FlowId,
    owner_node_id: NodeId,
    owner_kind: FlowOwnerKind,
    parent_flow_id: Option<FlowId>,
    depth: u32,
    terminal: FlowTerminal,
}

impl ValidatedFlow {
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }

    pub const fn owner_node_id(&self) -> NodeId {
        self.owner_node_id
    }

    pub const fn owner_kind(&self) -> FlowOwnerKind {
        self.owner_kind
    }

    pub const fn parent_flow_id(&self) -> Option<FlowId> {
        self.parent_flow_id
    }

    pub const fn depth(&self) -> u32 {
        self.depth
    }

    pub const fn terminal(&self) -> FlowTerminal {
        self.terminal
    }
}

/// Non-cloneable completeness proof projected from the canonical registry.
#[derive(Debug)]
pub struct ValidatedFlowRegistryReceipt {
    package: DocumentFingerprint,
    epoch: LayoutEpoch,
    fingerprint: FlowRegistryFingerprint,
    flow_count: u32,
    max_depth: u32,
}

impl ValidatedFlowRegistryReceipt {
    pub const fn package_fingerprint(&self) -> DocumentFingerprint {
        self.package
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub const fn fingerprint(&self) -> FlowRegistryFingerprint {
        self.fingerprint
    }

    pub const fn flow_count(&self) -> u32 {
        self.flow_count
    }

    pub const fn max_depth(&self) -> u32 {
        self.max_depth
    }
}

/// Dense canonical body/subflow registry. All relations are derived from the
/// typed Document traversal retained by `ValidatedFlowContentRegistry`.
#[derive(Debug)]
pub struct ValidatedFlowRegistry {
    flows: Vec<ValidatedFlow>,
    receipt: ValidatedFlowRegistryReceipt,
}

impl ValidatedFlowRegistry {
    fn from_content(content: &ValidatedFlowContentRegistry) -> Result<Self, FlowRegistryError> {
        if content.model.node_count > content.max_ast_nodes
            || u64::try_from(content.model.flows.len())
                .map_err(|_| FlowRegistryError::AstNodeLimit)?
                > content.model.node_count
        {
            return Err(FlowRegistryError::AstNodeLimit);
        }
        if content.model.max_depth > content.max_ast_nesting_depth {
            return Err(FlowRegistryError::FlowDepthLimit);
        }
        let mut flows = Vec::new();
        flows
            .try_reserve_exact(content.model.flows.len())
            .map_err(|_| FlowRegistryError::AllocationFailure)?;
        for (expected_id, expected) in content.model.flows.iter().enumerate() {
            if expected.flow_id.get() as usize != expected_id {
                return Err(FlowRegistryError::NonDenseFlowId);
            }
            let observed_terminal = content
                .contents
                .iter()
                .filter(|entry| entry.flow_id == expected.flow_id)
                .try_fold(0u32, |total, entry| {
                    total
                        .checked_add(entry.content.boundary_count())
                        .ok_or(FlowRegistryError::ArithmeticOverflow)
                })?;
            if observed_terminal != expected.terminal.owner_local_ordinal() {
                return Err(FlowRegistryError::WrongTerminal(expected.flow_id));
            }
            if let Some(parent) = expected.parent_flow_id {
                let Some(parent_record) = content.model.flows.get(parent.get() as usize) else {
                    return Err(FlowRegistryError::WrongParent(expected.flow_id));
                };
                if parent_record.depth.checked_add(1) != Some(expected.depth) {
                    return Err(FlowRegistryError::WrongParent(expected.flow_id));
                }
            } else if expected.flow_id != FlowId::DOCUMENT_BODY || expected.depth != 1 {
                return Err(FlowRegistryError::WrongParent(expected.flow_id));
            }
            flows.push(ValidatedFlow {
                flow_id: expected.flow_id,
                owner_node_id: expected.owner_node_id,
                owner_kind: expected.owner_kind,
                parent_flow_id: expected.parent_flow_id,
                depth: expected.depth,
                terminal: expected.terminal,
            });
        }
        let canonical_jcs = encode_flow_registry_jcs(content, &flows);
        let fingerprint = flow_registry_fingerprint_from_jcs(&canonical_jcs);
        let flow_count = u32::try_from(flows.len()).map_err(|_| FlowRegistryError::AstNodeLimit)?;
        Ok(Self {
            flows,
            receipt: ValidatedFlowRegistryReceipt {
                package: content.package,
                epoch: content.epoch,
                fingerprint,
                flow_count,
                max_depth: content.model.max_depth,
            },
        })
    }

    pub fn flows(&self) -> &[ValidatedFlow] {
        &self.flows
    }

    pub fn flow(&self, flow_id: FlowId) -> Option<&ValidatedFlow> {
        self.flows
            .get(flow_id.get() as usize)
            .filter(|flow| flow.flow_id == flow_id)
    }

    pub const fn receipt(&self) -> &ValidatedFlowRegistryReceipt {
        &self.receipt
    }
}

fn encode_flow_registry_jcs(
    content: &ValidatedFlowContentRegistry,
    flows: &[ValidatedFlow],
) -> String {
    let mut output = String::from("{\"algorithm\":\"");
    output.push_str(FlowRegistryFingerprint::ALGORITHM_ID);
    output.push_str("\",\"flows\":[");
    for (flow_index, flow) in flows.iter().enumerate() {
        if flow_index > 0 {
            output.push(',');
        }
        output.push_str("{\"contents\":[");
        for (content_index, entry) in content
            .contents
            .iter()
            .filter(|entry| entry.flow_id == flow.flow_id)
            .enumerate()
        {
            if content_index > 0 {
                output.push(',');
            }
            output.push_str("{\"boundary_count\":");
            output.push_str(&entry.content.boundary_count().to_string());
            if entry.content.kind() == FlowContentKind::TableRow {
                output.push_str(",\"child_flow_ids\":[");
                for (index, flow_id) in entry.child_flow_ids.iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    output.push_str(&flow_id.get().to_string());
                }
                output.push(']');
            } else {
                output.push_str(",\"child_flow_id\":");
                push_optional_flow_id(&mut output, entry.child_flow_id());
            }
            output.push_str(",\"kind\":\"");
            output.push_str(entry.content.kind().as_str());
            output.push_str("\",\"owner_node_id\":");
            output.push_str(&entry.content.owner().get().to_string());
            output.push('}');
        }
        output.push_str("],\"depth\":");
        output.push_str(&flow.depth.to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&flow.flow_id.get().to_string());
        output.push_str(",\"kind\":\"");
        output.push_str(flow.owner_kind.as_str());
        output.push_str("\",\"owner_node_id\":");
        output.push_str(&flow.owner_node_id.get().to_string());
        output.push_str(",\"parent_flow_id\":");
        push_optional_flow_id(&mut output, flow.parent_flow_id);
        output.push_str(",\"terminal\":");
        output.push_str(&flow.terminal.owner_local_ordinal().to_string());
        output.push('}');
    }
    output.push_str("],\"layout_epoch\":");
    push_layout_epoch_jcs(&mut output, content.epoch);
    output.push_str(",\"package_sha256\":");
    push_hash_hex_jcs(&mut output, content.package.bytes());
    output.push('}');
    output
}

fn push_optional_flow_id(output: &mut String, value: Option<FlowId>) {
    match value {
        Some(value) => output.push_str(&value.get().to_string()),
        None => output.push_str("null"),
    }
}

fn push_layout_epoch_jcs(output: &mut String, epoch: LayoutEpoch) {
    output.push_str("{\"admitted_resources_sha256\":");
    push_hash_hex_jcs(output, epoch.admitted_resources().bytes());
    output.push_str(",\"document_sha256\":");
    push_hash_hex_jcs(output, epoch.document().bytes());
    output.push_str(",\"resolved_input_sha256\":");
    push_hash_hex_jcs(output, epoch.references().bytes());
    output.push_str(",\"style_page_master_sha256\":");
    push_hash_hex_jcs(output, epoch.style().bytes());
    output.push('}');
}

fn push_hash_hex_jcs(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionFlowPosition {
    epoch: LayoutEpoch,
    registry: FlowRegistryFingerprint,
    flow_id: FlowId,
    flow_owner_node_id: NodeId,
    parent_flow_id: Option<FlowId>,
    flow_local_ordinal: u32,
    content_owner_node_id: Option<NodeId>,
    owner_local_boundary: u32,
    content_kind: Option<FlowContentKind>,
    child_flow_ids: Vec<FlowId>,
    terminal: bool,
    block_child_path: Vec<u32>,
}

impl ProductionFlowPosition {
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }

    pub const fn registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.registry
    }

    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }

    pub const fn flow_owner_node_id(&self) -> NodeId {
        self.flow_owner_node_id
    }

    pub const fn parent_flow_id(&self) -> Option<FlowId> {
        self.parent_flow_id
    }

    pub const fn flow_local_ordinal(&self) -> u32 {
        self.flow_local_ordinal
    }

    pub const fn content_owner_node_id(&self) -> Option<NodeId> {
        self.content_owner_node_id
    }

    pub const fn owner_local_boundary(&self) -> u32 {
        self.owner_local_boundary
    }

    pub const fn content_kind(&self) -> Option<FlowContentKind> {
        self.content_kind
    }

    pub fn child_flow_id(&self) -> Option<FlowId> {
        (self.child_flow_ids.len() == 1).then(|| self.child_flow_ids[0])
    }

    pub fn child_flow_ids(&self) -> &[FlowId] {
        &self.child_flow_ids
    }

    pub const fn is_terminal(&self) -> bool {
        self.terminal
    }

    pub fn block_child_path(&self) -> &[u32] {
        &self.block_child_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionFlow {
    descriptor: ValidatedFlow,
    positions: Vec<ProductionFlowPosition>,
}

impl ProductionFlow {
    pub const fn descriptor(&self) -> &ValidatedFlow {
        &self.descriptor
    }

    pub fn positions(&self) -> &[ProductionFlowPosition] {
        &self.positions
    }

    pub fn terminal_position(&self) -> &ProductionFlowPosition {
        self.positions
            .last()
            .expect("a validated production flow always has a terminal")
    }
}

/// Complete production IR. Body and subflow positions remain in separate
/// vectors; no API exposes a body-flattened cursor sequence.
#[derive(Debug)]
pub struct ProductionFlowIr {
    content_registry: ValidatedFlowContentRegistry,
    registry: ValidatedFlowRegistry,
    flows: Vec<ProductionFlow>,
}

impl ProductionFlowIr {
    /// Deterministic staging convenience for documents whose paragraph and
    /// heading content is empty. It is not a fallback for missing line-break
    /// work: `ValidatedParagraphItemRegistry` rejects any text-producing site.
    pub fn for_empty_paragraph_content(
        package: &ValidatedParsedPackage,
        epoch: LayoutEpoch,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, FlowRegistryError> {
        let paragraph_items = ValidatedParagraphItemRegistry::for_empty_content(package, epoch)
            .map_err(|_| FlowRegistryError::MissingParagraphContent(NodeId::new(0)))?;
        let mut builder = ProductionFlowIrBuilder::new(package, &paragraph_items, epoch, limits)?;
        let owners: Vec<_> = builder.expected_content_owners().collect();
        for owner in owners {
            let content = builder.issue_content(owner)?;
            builder.register_content(content)?;
        }
        builder.finish()
    }

    pub const fn content_registry(&self) -> &ValidatedFlowContentRegistry {
        &self.content_registry
    }

    pub const fn registry(&self) -> &ValidatedFlowRegistry {
        &self.registry
    }

    pub fn flows(&self) -> &[ProductionFlow] {
        &self.flows
    }

    pub fn flow(&self, flow_id: FlowId) -> Option<&ProductionFlow> {
        self.flows
            .get(flow_id.get() as usize)
            .filter(|flow| flow.descriptor.flow_id == flow_id)
    }
}

/// Private MI3-02 entry point. It consumes a sealed table style and the
/// canonical flow IR, then issues the complete column/grid/cell-frame receipt.
/// Public profile selection remains intentionally unchanged until MI3-04.
pub fn layout_table_grid(
    package: &ValidatedStagingStylePackage,
    table_owner: NodeId,
    style: &MachineTableComputedStyleReceipt,
    ir: &ProductionFlowIr,
    frame_inline_size: PositiveLength,
    limits: &ValidatedResourceLimits,
) -> Result<ValidatedTableGridReceipt, TableGridLayoutError> {
    let parsed = package.package();
    let epoch = ir.registry().receipt().epoch();
    if ir.registry().receipt().package_fingerprint() != parsed.epoch_identity().document()
        || epoch.document() != parsed.epoch_identity().document()
        || epoch.style() != parsed.epoch_identity().style()
    {
        return Err(TableGridLayoutError::PackageEpochMismatch);
    }
    if style.owner() != table_owner
        || style.package_fingerprint() != package.package_fingerprint()
        || style.document_fingerprint() != epoch.document()
        || style.style_fingerprint() != epoch.style()
        || style.registry_version() != TABLE_BLOCK_STYLE_REGISTRY_VERSION
    {
        return Err(TableGridLayoutError::WrongStyleReceipt);
    }
    let Block::Table {
        columns,
        head,
        body,
        ..
    } = parsed
        .package()
        .document
        .blocks
        .iter()
        .find(|block| matches!(block, Block::Table { node_id, .. } if *node_id == table_owner))
        .ok_or(TableGridLayoutError::TableNotFound(table_owner))?
    else {
        return Err(TableGridLayoutError::TableNotFound(table_owner));
    };

    let shapes = prevalidate_table_profile(parsed, limits)?;
    let shape = shapes
        .table(table_owner)
        .ok_or(TableGridLayoutError::TableNotFound(table_owner))?;
    let computed = style.computed();
    let after_start = frame_inline_size
        .get()
        .checked_sub(computed.start_indent().get())
        .ok_or(TableGridLayoutError::ColumnArithmetic)?;
    let available_inline_size = after_start
        .checked_sub(computed.end_indent().get())
        .and_then(PositiveLength::new)
        .ok_or(TableGridLayoutError::ColumnArithmetic)?;
    let (resolved_columns, rounding_residual, residual_recipient) =
        resolve_table_columns(columns, available_inline_size)?;

    let mut rows = Vec::new();
    rows.try_reserve_exact(shape.rows.len())
        .map_err(|_| TableGridLayoutError::AllocationFailure)?;
    rows.extend(
        shape
            .rows
            .iter()
            .map(|row| ValidatedTableRowBinding::new(row.row_owner, row.section, row.row_ordinal)),
    );

    let ast_cells: Vec<&TableCell> = head.iter().chain(body).flat_map(|row| &row.cells).collect();
    if ast_cells.len() != shape.cells.len() {
        return Err(TableGridLayoutError::WrongOwner(table_owner));
    }
    let mut cells = Vec::new();
    cells
        .try_reserve_exact(shape.cells.len())
        .map_err(|_| TableGridLayoutError::AllocationFailure)?;
    for (cell_shape, ast_cell) in shape.cells.iter().zip(&ast_cells) {
        if cell_shape.cell_owner != ast_cell.node_id
            || cell_shape.colspan != ast_cell.colspan
            || cell_shape.rowspan != ast_cell.rowspan
        {
            return Err(TableGridLayoutError::WrongOwner(ast_cell.node_id));
        }
        let mut matching_flows = ir
            .registry()
            .flows()
            .iter()
            .filter(|flow| flow.owner_node_id() == cell_shape.cell_owner);
        let flow = matching_flows
            .next()
            .ok_or(TableGridLayoutError::MissingCellFlow(cell_shape.cell_owner))?;
        if matching_flows.next().is_some()
            || flow.owner_kind() != FlowOwnerKind::TableCell
            || flow.parent_flow_id() != Some(FlowId::DOCUMENT_BODY)
        {
            return Err(TableGridLayoutError::WrongCellFlow(cell_shape.cell_owner));
        }
        let (frame_inline_start, cell_inline_size) = table_cell_inline_frame(
            &resolved_columns,
            cell_shape.column_ordinal,
            cell_shape.colspan,
            cell_shape.cell_owner,
        )?;
        cells.push(ValidatedTableCellBinding::new(
            cell_shape.cell_owner,
            cell_shape.row_owner,
            cell_shape.section,
            cell_shape.row_ordinal,
            cell_shape.column_ordinal,
            cell_shape.colspan,
            cell_shape.rowspan,
            flow.flow_id(),
            flow.terminal(),
            frame_inline_start,
            cell_inline_size,
        ));
    }

    for row in shape.rows.iter() {
        let expected_cell_flows: Vec<_> = cells
            .iter()
            .filter(|cell| cell.section() == row.section && cell.row_ordinal() == row.row_ordinal)
            .map(ValidatedTableCellBinding::flow_id)
            .collect();
        let Some(content) = ir.content_registry().contents().iter().find(|content| {
            content.content().owner() == row.row_owner
                && content.content().kind() == FlowContentKind::TableRow
        }) else {
            return Err(TableGridLayoutError::FlowRegistryMismatch);
        };
        if content.flow_id() != FlowId::DOCUMENT_BODY
            || content.child_flow_ids() != expected_cell_flows
        {
            return Err(TableGridLayoutError::FlowRegistryMismatch);
        }
    }

    ValidatedTableGridReceipt::new(TableGridReceiptInput {
        package_sha256: package.package_fingerprint().into_bytes(),
        epoch,
        flow_registry: ir.registry().receipt().fingerprint(),
        table_owner,
        containing_flow_id: FlowId::DOCUMENT_BODY,
        frame_inline_size,
        available_inline_size,
        start_indent: computed.start_indent(),
        end_indent: computed.end_indent(),
        space_before: computed.space_before(),
        space_after: computed.space_after(),
        keep_with_next: computed.keep_with_next(),
        columns: resolved_columns,
        rounding_residual,
        residual_recipient,
        rows,
        cells,
    })
    .map_err(TableGridLayoutError::Receipt)
}

pub const TABLE_ROW_BAND_LAYOUT_ALGORITHM: &str = "typaxis.table-row-band-receipt/1";

/// Measured indivisible block fragments for one canonical cell flow. The
/// fragment sizes are ordered in cell-flow order; an empty vector is the
/// already-terminal transparent cell required by the table profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableCellLayoutInput {
    cell_owner: NodeId,
    flow_id: FlowId,
    fragment_block_sizes: Vec<PositiveLength>,
}

impl TableCellLayoutInput {
    pub fn new(
        cell_owner: NodeId,
        flow_id: FlowId,
        fragment_block_sizes: Vec<PositiveLength>,
    ) -> Self {
        Self {
            cell_owner,
            flow_id,
            fragment_block_sizes,
        }
    }

    pub const fn empty(cell_owner: NodeId, flow_id: FlowId) -> Self {
        Self {
            cell_owner,
            flow_id,
            fragment_block_sizes: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableRowBandLayoutError {
    MissingCellMeasurement(NodeId),
    ExtraCellMeasurement(NodeId),
    WrongCellFlow(NodeId),
    MissingRow(NodeId),
    FragmentLimit,
    ArithmeticOverflow,
    AllocationFailure,
}

/// Sealed cell-flow measurement. Endpoints are cumulative legal break
/// boundaries, so pagination never has to reinterpret paragraph fragments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableCellLayoutReceipt {
    cell_owner: NodeId,
    row_owner: NodeId,
    section: TableSection,
    row_ordinal: u32,
    column_ordinal: u32,
    colspan: NonZeroU16,
    rowspan: NonZeroU16,
    flow_id: FlowId,
    frame_inline_start: NonNegativeLength,
    frame_inline_size: PositiveLength,
    fragment_block_sizes: Vec<PositiveLength>,
    fragment_endpoints: Vec<PositiveLength>,
    natural_block_size: NonNegativeLength,
}

impl TableCellLayoutReceipt {
    pub const fn cell_owner(&self) -> NodeId {
        self.cell_owner
    }
    pub const fn row_owner(&self) -> NodeId {
        self.row_owner
    }
    pub const fn section(&self) -> TableSection {
        self.section
    }
    pub const fn row_ordinal(&self) -> u32 {
        self.row_ordinal
    }
    pub const fn column_ordinal(&self) -> u32 {
        self.column_ordinal
    }
    pub const fn colspan(&self) -> NonZeroU16 {
        self.colspan
    }
    pub const fn rowspan(&self) -> NonZeroU16 {
        self.rowspan
    }
    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }
    pub const fn frame_inline_start(&self) -> NonNegativeLength {
        self.frame_inline_start
    }
    pub const fn frame_inline_size(&self) -> PositiveLength {
        self.frame_inline_size
    }
    pub fn fragment_block_sizes(&self) -> &[PositiveLength] {
        &self.fragment_block_sizes
    }
    pub fn fragment_endpoints(&self) -> &[PositiveLength] {
        &self.fragment_endpoints
    }
    pub const fn natural_block_size(&self) -> NonNegativeLength {
        self.natural_block_size
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TableRowBandReceipt {
    row_owner: NodeId,
    section: TableSection,
    row_ordinal: u32,
    block_size: NonNegativeLength,
}

impl TableRowBandReceipt {
    pub const fn row_owner(self) -> NodeId {
        self.row_owner
    }
    pub const fn section(self) -> TableSection {
        self.section
    }
    pub const fn row_ordinal(self) -> u32 {
        self.row_ordinal
    }
    pub const fn block_size(self) -> NonNegativeLength {
        self.block_size
    }
}

/// Package/epoch/grid-bound table measurement. Rowspan deficits are assigned
/// wholly to the last covered logical row, yielding one deterministic band
/// vector without retaining a recursive grid snapshot.
#[derive(Debug)]
pub struct TableRowBandLayoutReceipt {
    package_sha256: [u8; 32],
    epoch: LayoutEpoch,
    flow_registry: FlowRegistryFingerprint,
    grid: TableGridFingerprint,
    table_owner: NodeId,
    cells: Vec<TableCellLayoutReceipt>,
    rows: Vec<TableRowBandReceipt>,
    contained_fragment_count: u64,
    fingerprint: [u8; 32],
}

impl TableRowBandLayoutReceipt {
    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn flow_registry_fingerprint(&self) -> FlowRegistryFingerprint {
        self.flow_registry
    }
    pub const fn grid_fingerprint(&self) -> TableGridFingerprint {
        self.grid
    }
    pub const fn table_owner(&self) -> NodeId {
        self.table_owner
    }
    pub fn cells(&self) -> &[TableCellLayoutReceipt] {
        &self.cells
    }
    pub fn rows(&self) -> &[TableRowBandReceipt] {
        &self.rows
    }
    pub const fn contained_fragment_count(&self) -> u64 {
        self.contained_fragment_count
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn cell(&self, owner: NodeId) -> Option<&TableCellLayoutReceipt> {
        self.cells.iter().find(|cell| cell.cell_owner == owner)
    }
    pub fn row(&self, section: TableSection, ordinal: u32) -> Option<TableRowBandReceipt> {
        self.rows
            .iter()
            .copied()
            .find(|row| row.section == section && row.row_ordinal == ordinal)
    }
}

/// Private MI3-03 cell-layout and row-band issuer. Inputs must correspond
/// one-for-one with the already validated grid cells in canonical order.
pub fn layout_table_row_bands(
    grid: &ValidatedTableGridReceipt,
    inputs: Vec<TableCellLayoutInput>,
    limits: &ValidatedResourceLimits,
) -> Result<TableRowBandLayoutReceipt, TableRowBandLayoutError> {
    let contained_fragment_count = inputs.iter().try_fold(0u64, |total, input| {
        total
            .checked_add(
                u64::try_from(input.fragment_block_sizes.len())
                    .map_err(|_| TableRowBandLayoutError::ArithmeticOverflow)?,
            )
            .ok_or(TableRowBandLayoutError::ArithmeticOverflow)
    })?;
    if contained_fragment_count > limits.get().max_fragments {
        return Err(TableRowBandLayoutError::FragmentLimit);
    }
    if inputs.len() < grid.cells().len() {
        let owner = grid.cells()[inputs.len()].cell_owner();
        return Err(TableRowBandLayoutError::MissingCellMeasurement(owner));
    }
    if inputs.len() > grid.cells().len() {
        return Err(TableRowBandLayoutError::ExtraCellMeasurement(
            inputs[grid.cells().len()].cell_owner,
        ));
    }

    let mut cells = Vec::new();
    cells
        .try_reserve_exact(inputs.len())
        .map_err(|_| TableRowBandLayoutError::AllocationFailure)?;
    for (binding, input) in grid.cells().iter().zip(inputs) {
        if input.cell_owner != binding.cell_owner() {
            return Err(TableRowBandLayoutError::MissingCellMeasurement(
                binding.cell_owner(),
            ));
        }
        if input.flow_id != binding.flow_id() {
            return Err(TableRowBandLayoutError::WrongCellFlow(binding.cell_owner()));
        }
        let mut endpoints = Vec::new();
        endpoints
            .try_reserve_exact(input.fragment_block_sizes.len())
            .map_err(|_| TableRowBandLayoutError::AllocationFailure)?;
        let mut total = Length::ZERO;
        for size in &input.fragment_block_sizes {
            total = total
                .checked_add(size.get())
                .ok_or(TableRowBandLayoutError::ArithmeticOverflow)?;
            endpoints.push(
                PositiveLength::new(total).ok_or(TableRowBandLayoutError::ArithmeticOverflow)?,
            );
        }
        cells.push(TableCellLayoutReceipt {
            cell_owner: binding.cell_owner(),
            row_owner: binding.row_owner(),
            section: binding.section(),
            row_ordinal: binding.row_ordinal(),
            column_ordinal: binding.column_ordinal(),
            colspan: binding.colspan(),
            rowspan: binding.rowspan(),
            flow_id: binding.flow_id(),
            frame_inline_start: binding.frame_inline_start(),
            frame_inline_size: binding.frame_inline_size(),
            fragment_block_sizes: input.fragment_block_sizes,
            fragment_endpoints: endpoints,
            natural_block_size: NonNegativeLength::new(total)
                .ok_or(TableRowBandLayoutError::ArithmeticOverflow)?,
        });
    }

    let mut row_sizes = Vec::new();
    row_sizes
        .try_reserve_exact(grid.rows().len())
        .map_err(|_| TableRowBandLayoutError::AllocationFailure)?;
    row_sizes.resize(grid.rows().len(), 0i128);
    for cell in &cells {
        let start = grid
            .rows()
            .iter()
            .position(|row| row.section() == cell.section && row.row_ordinal() == cell.row_ordinal)
            .ok_or(TableRowBandLayoutError::MissingRow(cell.row_owner))?;
        let end = start
            .checked_add(usize::from(cell.rowspan.get()))
            .ok_or(TableRowBandLayoutError::ArithmeticOverflow)?;
        let covered = row_sizes
            .get(start..end)
            .ok_or(TableRowBandLayoutError::MissingRow(cell.row_owner))?
            .iter()
            .try_fold(0i128, |total, value| total.checked_add(*value))
            .ok_or(TableRowBandLayoutError::ArithmeticOverflow)?;
        let natural = i128::from(cell.natural_block_size.get().raw());
        if natural > covered {
            let deficit = natural
                .checked_sub(covered)
                .ok_or(TableRowBandLayoutError::ArithmeticOverflow)?;
            row_sizes[end - 1] = row_sizes[end - 1]
                .checked_add(deficit)
                .ok_or(TableRowBandLayoutError::ArithmeticOverflow)?;
        }
    }

    let mut rows = Vec::new();
    rows.try_reserve_exact(grid.rows().len())
        .map_err(|_| TableRowBandLayoutError::AllocationFailure)?;
    for (binding, raw) in grid.rows().iter().zip(row_sizes) {
        let raw = i64::try_from(raw).map_err(|_| TableRowBandLayoutError::ArithmeticOverflow)?;
        let block_size = Length::from_raw(raw)
            .and_then(NonNegativeLength::new)
            .ok_or(TableRowBandLayoutError::ArithmeticOverflow)?;
        rows.push(TableRowBandReceipt {
            row_owner: binding.row_owner(),
            section: binding.section(),
            row_ordinal: binding.row_ordinal(),
            block_size,
        });
    }
    for cell in &cells {
        let mut spanning_block_size = Length::ZERO;
        let end = cell
            .row_ordinal
            .checked_add(u32::from(cell.rowspan.get()))
            .ok_or(TableRowBandLayoutError::ArithmeticOverflow)?;
        for ordinal in cell.row_ordinal..end {
            spanning_block_size = spanning_block_size
                .checked_add(
                    rows.iter()
                        .find(|row| row.section == cell.section && row.row_ordinal == ordinal)
                        .ok_or(TableRowBandLayoutError::MissingRow(cell.row_owner))?
                        .block_size
                        .get(),
                )
                .ok_or(TableRowBandLayoutError::ArithmeticOverflow)?;
        }
        if spanning_block_size.raw() < cell.natural_block_size.get().raw() {
            return Err(TableRowBandLayoutError::ArithmeticOverflow);
        }
    }
    let canonical_jcs = encode_table_row_band_layout(grid, &cells, &rows, contained_fragment_count);
    Ok(TableRowBandLayoutReceipt {
        package_sha256: grid.package_sha256(),
        epoch: grid.epoch(),
        flow_registry: grid.flow_registry(),
        grid: grid.fingerprint(),
        table_owner: grid.table_owner(),
        cells,
        rows,
        contained_fragment_count,
        fingerprint: sha256(canonical_jcs.as_bytes()),
    })
}

fn encode_table_row_band_layout(
    grid: &ValidatedTableGridReceipt,
    cells: &[TableCellLayoutReceipt],
    rows: &[TableRowBandReceipt],
    contained_fragment_count: u64,
) -> String {
    let mut output = String::from("{\"algorithm\":\"");
    output.push_str(TABLE_ROW_BAND_LAYOUT_ALGORITHM);
    output.push_str("\",\"cells\":[");
    for (index, cell) in cells.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"cell_node_id\":");
        output.push_str(&cell.cell_owner.get().to_string());
        output.push_str(",\"flow_id\":");
        output.push_str(&cell.flow_id.get().to_string());
        output.push_str(",\"fragment_block_sizes\":[");
        for (fragment_index, size) in cell.fragment_block_sizes.iter().enumerate() {
            if fragment_index != 0 {
                output.push(',');
            }
            output.push_str(&size.get().raw().to_string());
        }
        output.push_str("],\"natural_block_size\":");
        output.push_str(&cell.natural_block_size.get().raw().to_string());
        output.push('}');
    }
    output.push_str("],\"contained_fragment_count\":");
    output.push_str(&contained_fragment_count.to_string());
    output.push_str(",\"grid_sha256\":");
    push_hash_hex_jcs(&mut output, grid.fingerprint().bytes());
    output.push_str(",\"rows\":[");
    for (index, row) in rows.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"block_size\":");
        output.push_str(&row.block_size.get().raw().to_string());
        output.push_str(",\"row_node_id\":");
        output.push_str(&row.row_owner.get().to_string());
        output.push_str(",\"row_ordinal\":");
        output.push_str(&row.row_ordinal.to_string());
        output.push_str(",\"section\":\"");
        output.push_str(row.section.as_str());
        output.push_str("\"}");
    }
    output.push_str("],\"table_node_id\":");
    output.push_str(&grid.table_owner().get().to_string());
    output.push('}');
    output
}

fn resolve_table_columns(
    columns: &[TableColumn],
    available_inline_size: PositiveLength,
) -> Result<(Vec<ResolvedTableColumn>, Length, Option<u32>), TableGridLayoutError> {
    if columns.is_empty() {
        return Err(TableGridLayoutError::ColumnArithmetic);
    }
    let available = i128::from(available_inline_size.get().raw());
    let mut fixed_sum = 0i128;
    let mut weight_sum = 0i128;
    let mut last_fraction = None;
    for (index, column) in columns.iter().enumerate() {
        match &column.sizing {
            ColumnSizing::Fixed(width) => {
                fixed_sum = fixed_sum
                    .checked_add(i128::from(width.get().raw()))
                    .ok_or(TableGridLayoutError::ColumnArithmetic)?;
            }
            ColumnSizing::Fraction(weight) => {
                weight_sum = weight_sum
                    .checked_add(i128::from(weight.get()))
                    .ok_or(TableGridLayoutError::ColumnArithmetic)?;
                last_fraction =
                    Some(u32::try_from(index).map_err(|_| TableGridLayoutError::ColumnArithmetic)?);
            }
        }
    }
    let remaining = available
        .checked_sub(fixed_sum)
        .ok_or(TableGridLayoutError::ColumnArithmetic)?;
    if remaining < 0 || (weight_sum == 0 && remaining != 0) || (weight_sum != 0 && remaining <= 0) {
        return Err(TableGridLayoutError::ColumnArithmetic);
    }

    let mut rounded = Vec::new();
    rounded
        .try_reserve_exact(columns.len())
        .map_err(|_| TableGridLayoutError::AllocationFailure)?;
    let mut rounded_sum = 0i128;
    for column in columns {
        let value = match &column.sizing {
            ColumnSizing::Fixed(_) => None,
            ColumnSizing::Fraction(weight) => {
                let numerator = remaining
                    .checked_mul(i128::from(weight.get()))
                    .ok_or(TableGridLayoutError::ColumnArithmetic)?;
                let share = round_table_ratio_ties_even(numerator, weight_sum)?;
                rounded_sum = rounded_sum
                    .checked_add(share)
                    .ok_or(TableGridLayoutError::ColumnArithmetic)?;
                Some(share)
            }
        };
        rounded.push(value);
    }
    let residual = if weight_sum == 0 {
        0
    } else {
        remaining
            .checked_sub(rounded_sum)
            .ok_or(TableGridLayoutError::ColumnArithmetic)?
    };
    let rounding_residual = table_length_from_i128(residual)?;

    let mut resolved = Vec::new();
    resolved
        .try_reserve_exact(columns.len())
        .map_err(|_| TableGridLayoutError::AllocationFailure)?;
    let mut final_sum = 0i128;
    for (index, (column, rounded)) in columns.iter().zip(rounded).enumerate() {
        let index = u32::try_from(index).map_err(|_| TableGridLayoutError::ColumnArithmetic)?;
        let resolved_column = match (&column.sizing, rounded) {
            (ColumnSizing::Fixed(width), None) => {
                final_sum = final_sum
                    .checked_add(i128::from(width.get().raw()))
                    .ok_or(TableGridLayoutError::ColumnArithmetic)?;
                ResolvedTableColumn::fixed(index, *width)
            }
            (ColumnSizing::Fraction(weight), Some(rounded)) => {
                let final_raw = if last_fraction == Some(index) {
                    rounded
                        .checked_add(residual)
                        .ok_or(TableGridLayoutError::ColumnArithmetic)?
                } else {
                    rounded
                };
                let rounded = table_nonnegative_from_i128(rounded)?;
                let final_width = table_positive_from_i128(final_raw)?;
                final_sum = final_sum
                    .checked_add(final_raw)
                    .ok_or(TableGridLayoutError::ColumnArithmetic)?;
                ResolvedTableColumn::fraction(index, *weight, rounded, final_width)
            }
            _ => return Err(TableGridLayoutError::ColumnArithmetic),
        };
        resolved.push(resolved_column);
    }
    if final_sum != available {
        return Err(TableGridLayoutError::ColumnArithmetic);
    }
    Ok((resolved, rounding_residual, last_fraction))
}

fn table_cell_inline_frame(
    columns: &[ResolvedTableColumn],
    column_ordinal: u32,
    colspan: NonZeroU16,
    owner: NodeId,
) -> Result<(NonNegativeLength, PositiveLength), TableGridLayoutError> {
    let start =
        usize::try_from(column_ordinal).map_err(|_| TableGridLayoutError::GridOutOfRange(owner))?;
    let end = start
        .checked_add(usize::from(colspan.get()))
        .ok_or(TableGridLayoutError::ColumnArithmetic)?;
    if end > columns.len() {
        return Err(TableGridLayoutError::GridOutOfRange(owner));
    }
    let inline_start = columns[..start].iter().try_fold(0i128, |sum, column| {
        sum.checked_add(i128::from(column.final_width().get().raw()))
            .ok_or(TableGridLayoutError::ColumnArithmetic)
    })?;
    let inline_size = columns[start..end].iter().try_fold(0i128, |sum, column| {
        sum.checked_add(i128::from(column.final_width().get().raw()))
            .ok_or(TableGridLayoutError::ColumnArithmetic)
    })?;
    Ok((
        table_nonnegative_from_i128(inline_start)?,
        table_positive_from_i128(inline_size)?,
    ))
}

fn round_table_ratio_ties_even(
    numerator: i128,
    denominator: i128,
) -> Result<i128, TableGridLayoutError> {
    if numerator < 0 || denominator <= 0 {
        return Err(TableGridLayoutError::ColumnArithmetic);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let doubled = remainder
        .checked_mul(2)
        .ok_or(TableGridLayoutError::ColumnArithmetic)?;
    if doubled < denominator || (doubled == denominator && quotient % 2 == 0) {
        Ok(quotient)
    } else {
        quotient
            .checked_add(1)
            .ok_or(TableGridLayoutError::ColumnArithmetic)
    }
}

fn table_length_from_i128(raw: i128) -> Result<Length, TableGridLayoutError> {
    i64::try_from(raw)
        .ok()
        .and_then(Length::from_raw)
        .ok_or(TableGridLayoutError::ColumnArithmetic)
}

fn table_nonnegative_from_i128(raw: i128) -> Result<NonNegativeLength, TableGridLayoutError> {
    NonNegativeLength::new(table_length_from_i128(raw)?)
        .ok_or(TableGridLayoutError::ColumnArithmetic)
}

fn table_positive_from_i128(raw: i128) -> Result<PositiveLength, TableGridLayoutError> {
    PositiveLength::new(table_length_from_i128(raw)?).ok_or(TableGridLayoutError::ColumnArithmetic)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingFigureKeepPolicy {
    KeepImageAndCaption,
    AllowCaptionSplit,
}

impl StagingFigureKeepPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KeepImageAndCaption => "keep_image_and_caption",
            Self::AllowCaptionSplit => "allow_caption_split",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingFigureOversizePolicy {
    TerminalOnce,
}

impl StagingFigureOversizePolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TerminalOnce => "terminal_once",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingFigureLayoutError {
    PreflightMismatch,
    PackageMismatch,
    EpochMismatch,
    ResourceLedgerMismatch,
    UnsupportedPageMasterPolicy,
    UnsupportedFitPolicy(NodeId),
    MissingFigure(NodeId),
    ExtraFigure(NodeId),
    DuplicateFigure(NodeId),
    WrongFigureBoundary(NodeId),
    CaptionFlowMismatch(NodeId),
    MissingAdmittedImage(ImageResourceId),
    ExtraAdmittedImage(ImageResourceId),
    WrongMediaKind(ImageResourceId),
    FigureWidthRequired(NodeId),
    FigureExceedsInlineSize(NodeId),
    InvalidDimensions(ImageResourceId),
    ArithmeticOverflow,
    AllocationFailure,
}

/// One PNG figure bound to its admitted bytes, caption subflow, computed
/// horizontal geometry, and the closed keep/oversize policies. Vertical page
/// coordinates are selected by pagination from this receipt only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedFigureLayoutItem {
    figure_owner: NodeId,
    document_ordinal: u32,
    figure_flow_id: FlowId,
    caption_flow_id: FlowId,
    caption_owners: Vec<NodeId>,
    image_id: ImageResourceId,
    alt: String,
    admitted_media_kind: AdmittedImageMediaKind,
    admitted_sha256: [u8; 32],
    admitted_byte_length: u64,
    pixel_width: NonZeroU32,
    pixel_height: NonZeroU32,
    decoded_bytes: u64,
    inline_size: PositiveLength,
    block_size: PositiveLength,
    physical_left: Length,
    space_before: NonNegativeLength,
    space_after: NonNegativeLength,
    keep_policy: StagingFigureKeepPolicy,
    oversize_policy: StagingFigureOversizePolicy,
}

impl ValidatedFigureLayoutItem {
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
    pub fn caption_owners(&self) -> &[NodeId] {
        &self.caption_owners
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub fn alt(&self) -> &str {
        &self.alt
    }
    pub const fn admitted_media_kind(&self) -> AdmittedImageMediaKind {
        self.admitted_media_kind
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn admitted_byte_length(&self) -> u64 {
        self.admitted_byte_length
    }
    pub const fn pixel_width(&self) -> NonZeroU32 {
        self.pixel_width
    }
    pub const fn pixel_height(&self) -> NonZeroU32 {
        self.pixel_height
    }
    pub const fn decoded_bytes(&self) -> u64 {
        self.decoded_bytes
    }
    pub const fn inline_size(&self) -> PositiveLength {
        self.inline_size
    }
    pub const fn block_size(&self) -> PositiveLength {
        self.block_size
    }
    pub const fn physical_left(&self) -> Length {
        self.physical_left
    }
    pub const fn space_before(&self) -> NonNegativeLength {
        self.space_before
    }
    pub const fn space_after(&self) -> NonNegativeLength {
        self.space_after
    }
    pub const fn keep_policy(&self) -> StagingFigureKeepPolicy {
        self.keep_policy
    }
    pub const fn oversize_policy(&self) -> StagingFigureOversizePolicy {
        self.oversize_policy
    }
}

/// Complete MI2-06 Figure layout proof. The page/master and resource facts are
/// exact and package/epoch-bound; no downstream stage reads raw declarations.
#[derive(Debug)]
pub struct ValidatedFigureLayout {
    package_sha256: [u8; 32],
    epoch: LayoutEpoch,
    flow_registry: FlowRegistryFingerprint,
    figure_usage_sha256: [u8; 32],
    policy_version: &'static str,
    master_id: MasterId,
    page_width: PositiveLength,
    page_height: PositiveLength,
    body: Rect,
    figures: Vec<ValidatedFigureLayoutItem>,
}

impl ValidatedFigureLayout {
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
    pub fn figures(&self) -> &[ValidatedFigureLayoutItem] {
        &self.figures
    }
}

pub fn layout_staging_machine_figures(
    package: &ValidatedStagingStylePackage,
    preflight: &ValidatedStagingFigureUsageReceipt,
    admitted: &AdmittedResourceLedger,
    ir: &ProductionFlowIr,
) -> Result<ValidatedFigureLayout, StagingFigureLayoutError> {
    if !preflight.verifies(package)
        || preflight.policy_version() != STAGING_BASIC_FIGURE_POLICY_VERSION
    {
        return Err(StagingFigureLayoutError::PreflightMismatch);
    }
    let epoch = ir.registry().receipt().epoch();
    if ir.content_registry().package_fingerprint() != package.package().epoch_identity().document()
        || epoch.document() != package.package().epoch_identity().document()
        || epoch.style() != package.package().epoch_identity().style()
    {
        return Err(StagingFigureLayoutError::PackageMismatch);
    }
    if epoch.admitted_resources() != admitted.fingerprint() {
        return Err(StagingFigureLayoutError::EpochMismatch);
    }
    let declarations = &package.package().package().resources;
    if !admitted.matches_declarations(declarations) {
        return Err(StagingFigureLayoutError::ResourceLedgerMismatch);
    }

    let masters = &package.package().package().page_masters;
    if masters.masters.len() != 1
        || !masters.selection_rules.is_empty()
        || masters.masters[0].master_id != masters.default_master_id
        || masters.masters[0].header.is_some()
        || masters.masters[0].footer.is_some()
        || masters.masters[0].footnote.is_some()
    {
        return Err(StagingFigureLayoutError::UnsupportedPageMasterPolicy);
    }
    let master = &masters.masters[0];

    let expected_images: std::collections::BTreeSet<_> = preflight
        .figures()
        .iter()
        .map(|figure| figure.image_id())
        .collect();
    for image_id in &expected_images {
        if admitted.image(*image_id).is_none() {
            return Err(StagingFigureLayoutError::MissingAdmittedImage(*image_id));
        }
    }
    if let Some(image) = admitted
        .images()
        .iter()
        .find(|image| !expected_images.contains(&image.image_id()))
    {
        return Err(StagingFigureLayoutError::ExtraAdmittedImage(
            image.image_id(),
        ));
    }

    let mut positions = std::collections::BTreeMap::new();
    for position in ir
        .flows()
        .iter()
        .flat_map(|flow| flow.positions())
        .filter(|position| position.content_kind() == Some(FlowContentKind::FigureCaption))
    {
        let owner = position
            .content_owner_node_id()
            .ok_or(StagingFigureLayoutError::PackageMismatch)?;
        if positions.insert(owner, position).is_some() {
            return Err(StagingFigureLayoutError::DuplicateFigure(owner));
        }
    }

    let mut figures = Vec::new();
    figures
        .try_reserve_exact(preflight.figures().len())
        .map_err(|_| StagingFigureLayoutError::AllocationFailure)?;
    for expected in preflight.figures() {
        let position = positions
            .remove(&expected.owner())
            .ok_or(StagingFigureLayoutError::MissingFigure(expected.owner()))?;
        if position.epoch() != epoch || position.flow_id() != FlowId::DOCUMENT_BODY {
            return Err(StagingFigureLayoutError::WrongFigureBoundary(
                expected.owner(),
            ));
        }
        let caption_flow_id =
            position
                .child_flow_id()
                .ok_or(StagingFigureLayoutError::CaptionFlowMismatch(
                    expected.owner(),
                ))?;
        let caption_flow =
            ir.flow(caption_flow_id)
                .ok_or(StagingFigureLayoutError::CaptionFlowMismatch(
                    expected.owner(),
                ))?;
        let observed_caption_owners: Vec<_> = caption_flow
            .positions()
            .iter()
            .filter_map(ProductionFlowPosition::content_owner_node_id)
            .collect();
        if observed_caption_owners != expected.caption_owners() {
            return Err(StagingFigureLayoutError::CaptionFlowMismatch(
                expected.owner(),
            ));
        }

        let style = package
            .compute_block_style(expected.owner(), None)
            .map_err(|_| StagingFigureLayoutError::PackageMismatch)?
            .computed();
        if style.keep_with_next() {
            return Err(StagingFigureLayoutError::UnsupportedFitPolicy(
                expected.owner(),
            ));
        }
        let inline_size = match style.width() {
            MachineFigureWidth::Length(width) => width,
            MachineFigureWidth::Auto => {
                return Err(StagingFigureLayoutError::FigureWidthRequired(
                    expected.owner(),
                ))
            }
        };
        let available = master
            .body
            .width()
            .get()
            .checked_sub(style.start_indent().get())
            .and_then(|value| value.checked_sub(style.end_indent().get()))
            .ok_or(StagingFigureLayoutError::ArithmeticOverflow)?;
        if inline_size.get().raw() > available.raw() {
            return Err(StagingFigureLayoutError::FigureExceedsInlineSize(
                expected.owner(),
            ));
        }
        let physical_left = master
            .body
            .x()
            .checked_add(style.start_indent().get())
            .ok_or(StagingFigureLayoutError::ArithmeticOverflow)?;
        let image = admitted.image(expected.image_id()).ok_or(
            StagingFigureLayoutError::MissingAdmittedImage(expected.image_id()),
        )?;
        if image.media_kind() != AdmittedImageMediaKind::Png {
            return Err(StagingFigureLayoutError::WrongMediaKind(
                expected.image_id(),
            ));
        }
        let block_size = scale_figure_height(inline_size, image.width(), image.height()).ok_or(
            StagingFigureLayoutError::InvalidDimensions(expected.image_id()),
        )?;
        figures.push(ValidatedFigureLayoutItem {
            figure_owner: expected.owner(),
            document_ordinal: expected.document_ordinal(),
            figure_flow_id: position.flow_id(),
            caption_flow_id,
            caption_owners: observed_caption_owners,
            image_id: expected.image_id(),
            alt: expected.alt().to_owned(),
            admitted_media_kind: image.media_kind(),
            admitted_sha256: image.content_hash(),
            admitted_byte_length: image.byte_length(),
            pixel_width: image.width(),
            pixel_height: image.height(),
            decoded_bytes: image.decoded_bytes(),
            inline_size,
            block_size,
            physical_left,
            space_before: style.space_before(),
            space_after: style.space_after(),
            keep_policy: if style.keep_caption() {
                StagingFigureKeepPolicy::KeepImageAndCaption
            } else {
                StagingFigureKeepPolicy::AllowCaptionSplit
            },
            oversize_policy: StagingFigureOversizePolicy::TerminalOnce,
        });
    }
    if let Some((owner, _)) = positions.first_key_value() {
        return Err(StagingFigureLayoutError::ExtraFigure(*owner));
    }
    Ok(ValidatedFigureLayout {
        package_sha256: package.package_fingerprint().into_bytes(),
        epoch,
        flow_registry: ir.registry().receipt().fingerprint(),
        figure_usage_sha256: preflight.usage_sha256(),
        policy_version: preflight.policy_version(),
        master_id: master.master_id.clone(),
        page_width: master.width,
        page_height: master.height,
        body: master.body,
        figures,
    })
}

fn scale_figure_height(
    inline_size: PositiveLength,
    pixel_width: NonZeroU32,
    pixel_height: NonZeroU32,
) -> Option<PositiveLength> {
    let numerator =
        i128::from(inline_size.get().raw()).checked_mul(i128::from(pixel_height.get()))?;
    let denominator = i128::from(pixel_width.get());
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let doubled = remainder.checked_mul(2)?;
    let rounded = if doubled < denominator {
        quotient
    } else if doubled > denominator || quotient % 2 != 0 {
        quotient.checked_add(1)?
    } else {
        quotient
    };
    let raw = i64::try_from(rounded).ok()?;
    Length::from_raw(raw).and_then(PositiveLength::new)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingForcedPageBreakLayoutError {
    PreflightMismatch,
    PackageMismatch,
    MissingBoundary(NodeId),
    ExtraBoundary(NodeId),
    DuplicateBoundary(NodeId),
    ArithmeticOverflow,
    AllocationFailure,
}

/// Layout's typed forced boundary. Unlike a fragment or a measured block it
/// has no geometry; its sole position is the exact pre-consume flow cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingForcedPageBreakBoundary {
    owner: NodeId,
    document_ordinal: u32,
    flow_id: FlowId,
    flow_local_ordinal: u32,
    epoch: LayoutEpoch,
}

impl StagingForcedPageBreakBoundary {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn document_ordinal(&self) -> u32 {
        self.document_ordinal
    }

    pub const fn flow_id(&self) -> FlowId {
        self.flow_id
    }

    pub const fn flow_local_ordinal(&self) -> u32 {
        self.flow_local_ordinal
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
}

/// Exact layout projection of syntax's forced-break usage proof into MI2-02's
/// sealed flow registry.
#[derive(Debug)]
pub struct StagingForcedPageBreakLayoutReceipt {
    package_sha256: [u8; 32],
    epoch: LayoutEpoch,
    flow_registry: FlowRegistryFingerprint,
    usage_sha256: [u8; 32],
    policy_version: &'static str,
    boundaries: Vec<StagingForcedPageBreakBoundary>,
}

impl StagingForcedPageBreakLayoutReceipt {
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

    pub fn boundaries(&self) -> &[StagingForcedPageBreakBoundary] {
        &self.boundaries
    }
}

pub fn layout_staging_forced_page_breaks(
    package: &ValidatedStagingStylePackage,
    preflight: &ValidatedStagingForcedPageBreakUsageReceipt,
    ir: &ProductionFlowIr,
) -> Result<StagingForcedPageBreakLayoutReceipt, StagingForcedPageBreakLayoutError> {
    if !preflight.verifies(package)
        || preflight.policy_version() != STAGING_FORCED_PAGE_BREAK_POLICY_VERSION
    {
        return Err(StagingForcedPageBreakLayoutError::PreflightMismatch);
    }
    if ir.content_registry().package_fingerprint() != package.package().epoch_identity().document()
    {
        return Err(StagingForcedPageBreakLayoutError::PackageMismatch);
    }

    let mut positions = std::collections::BTreeMap::new();
    for position in ir
        .flows()
        .iter()
        .flat_map(|flow| flow.positions().iter())
        .filter(|position| position.content_kind() == Some(FlowContentKind::PageBreak))
    {
        let owner = position
            .content_owner_node_id()
            .ok_or(StagingForcedPageBreakLayoutError::PackageMismatch)?;
        if positions.insert(owner, position).is_some() {
            return Err(StagingForcedPageBreakLayoutError::DuplicateBoundary(owner));
        }
    }

    let mut boundaries = Vec::new();
    boundaries
        .try_reserve_exact(preflight.breaks().len())
        .map_err(|_| StagingForcedPageBreakLayoutError::AllocationFailure)?;
    for expected in preflight.breaks() {
        let position = positions.remove(&expected.owner()).ok_or(
            StagingForcedPageBreakLayoutError::MissingBoundary(expected.owner()),
        )?;
        boundaries.push(StagingForcedPageBreakBoundary {
            owner: expected.owner(),
            document_ordinal: expected.document_ordinal(),
            flow_id: position.flow_id(),
            flow_local_ordinal: position.flow_local_ordinal(),
            epoch: position.epoch(),
        });
    }
    if let Some((owner, _)) = positions.first_key_value() {
        return Err(StagingForcedPageBreakLayoutError::ExtraBoundary(*owner));
    }
    if boundaries
        .iter()
        .enumerate()
        .any(|(index, boundary)| usize::try_from(boundary.document_ordinal) != Ok(index))
    {
        return Err(StagingForcedPageBreakLayoutError::ArithmeticOverflow);
    }
    Ok(StagingForcedPageBreakLayoutReceipt {
        package_sha256: package.package_fingerprint().into_bytes(),
        epoch: ir.registry().receipt().epoch(),
        flow_registry: ir.registry().receipt().fingerprint(),
        usage_sha256: preflight.usage_sha256(),
        policy_version: preflight.policy_version(),
        boundaries,
    })
}

/// Production entry point. Worker receipts may be registered in any order;
/// `finish` walks the package-derived registry and issues every position and
/// terminal in canonical owner order.
pub struct ProductionFlowIrBuilder<'a> {
    content: ValidatedFlowContentRegistryBuilder<'a>,
}

impl<'a> ProductionFlowIrBuilder<'a> {
    pub fn new(
        package: &'a ValidatedParsedPackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
        epoch: LayoutEpoch,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, FlowRegistryError> {
        Ok(Self {
            content: ValidatedFlowContentRegistryBuilder::new(
                package,
                paragraph_items,
                epoch,
                limits,
            )?,
        })
    }

    pub fn new_for_footnote_body(
        package: &'a ValidatedParsedPackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
        epoch: LayoutEpoch,
        limits: &ValidatedResourceLimits,
    ) -> Result<Self, FlowRegistryError> {
        Ok(Self {
            content: ValidatedFlowContentRegistryBuilder::new_for_footnote_body(
                package,
                paragraph_items,
                epoch,
                limits,
            )?,
        })
    }

    pub fn expected_content_owners(&self) -> impl ExactSizeIterator<Item = NodeId> + '_ {
        self.content.expected_content_owners()
    }

    pub fn issue_content(&self, owner: NodeId) -> Result<ValidatedFlowContent, FlowRegistryError> {
        self.content.issue_content(owner)
    }

    pub fn register_content(
        &mut self,
        content: ValidatedFlowContent,
    ) -> Result<(), FlowRegistryError> {
        self.content.register(content)
    }

    pub fn register_content_for(
        &mut self,
        owner: NodeId,
        content: ValidatedFlowContent,
    ) -> Result<(), FlowRegistryError> {
        self.content.register_for(owner, content)
    }

    pub fn finish(self) -> Result<ProductionFlowIr, FlowRegistryError> {
        let content_registry = self.content.finish()?;
        let registry = ValidatedFlowRegistry::from_content(&content_registry)?;
        let mut flows = Vec::new();
        flows
            .try_reserve_exact(registry.flows.len())
            .map_err(|_| FlowRegistryError::AllocationFailure)?;
        for descriptor in &registry.flows {
            let position_capacity = usize::try_from(descriptor.terminal.owner_local_ordinal())
                .map_err(|_| FlowRegistryError::ArithmeticOverflow)?
                .checked_add(1)
                .ok_or(FlowRegistryError::ArithmeticOverflow)?;
            let mut positions = Vec::new();
            positions
                .try_reserve_exact(position_capacity)
                .map_err(|_| FlowRegistryError::AllocationFailure)?;
            let mut flow_local_ordinal = 0u32;
            for entry in content_registry
                .contents
                .iter()
                .filter(|entry| entry.flow_id == descriptor.flow_id)
            {
                for owner_local_boundary in 0..entry.content.boundary_count() {
                    positions.push(ProductionFlowPosition {
                        epoch: content_registry.epoch,
                        registry: registry.receipt.fingerprint,
                        flow_id: descriptor.flow_id,
                        flow_owner_node_id: descriptor.owner_node_id,
                        parent_flow_id: descriptor.parent_flow_id,
                        flow_local_ordinal,
                        content_owner_node_id: Some(entry.content.owner()),
                        owner_local_boundary,
                        content_kind: Some(entry.content.kind()),
                        child_flow_ids: entry.child_flow_ids.clone(),
                        terminal: false,
                        block_child_path: entry.block_child_path.clone(),
                    });
                    flow_local_ordinal = flow_local_ordinal
                        .checked_add(1)
                        .ok_or(FlowRegistryError::ArithmeticOverflow)?;
                }
            }
            if flow_local_ordinal != descriptor.terminal.owner_local_ordinal() {
                return Err(FlowRegistryError::WrongTerminal(descriptor.flow_id));
            }
            let owner_path = if descriptor.flow_id == FlowId::DOCUMENT_BODY {
                Vec::new()
            } else {
                content_registry
                    .model
                    .contents
                    .iter()
                    .find(|entry| entry.child_flow_ids.contains(&descriptor.flow_id))
                    .map(|entry| entry.block_child_path.clone())
                    .ok_or(FlowRegistryError::WrongParent(descriptor.flow_id))?
            };
            positions.push(ProductionFlowPosition {
                epoch: content_registry.epoch,
                registry: registry.receipt.fingerprint,
                flow_id: descriptor.flow_id,
                flow_owner_node_id: descriptor.owner_node_id,
                parent_flow_id: descriptor.parent_flow_id,
                flow_local_ordinal,
                content_owner_node_id: None,
                owner_local_boundary: 0,
                content_kind: None,
                child_flow_ids: Vec::new(),
                terminal: true,
                block_child_path: owner_path,
            });
            flows.push(ProductionFlow {
                descriptor: descriptor.clone(),
                positions,
            });
        }
        Ok(ProductionFlowIr {
            content_registry,
            registry,
            flows,
        })
    }
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FlowPosition {
    epoch: LayoutEpoch,
    global_flow_ordinal: u64,
    owner: NodeId,
    block_child_path: Vec<u32>,
    owner_local_boundary: u32,
}
impl FlowPosition {
    fn new(
        epoch: LayoutEpoch,
        global_flow_ordinal: u64,
        owner: NodeId,
        block_child_path: Vec<u32>,
        owner_local_boundary: u32,
    ) -> Self {
        Self {
            epoch,
            global_flow_ordinal,
            owner,
            block_child_path,
            owner_local_boundary,
        }
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn global_flow_ordinal(&self) -> u64 {
        self.global_flow_ordinal
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub fn block_child_path(&self) -> &[u32] {
        &self.block_child_path
    }
    pub const fn owner_local_boundary(&self) -> u32 {
        self.owner_local_boundary
    }
    pub fn cmp_within_epoch(&self, other: &Self) -> Result<Ordering, FragmentError> {
        if self.epoch != other.epoch {
            return Err(FragmentError::InvalidCursorEpoch);
        }
        Ok((
            self.global_flow_ordinal,
            self.owner,
            &self.block_child_path,
            self.owner_local_boundary,
        )
            .cmp(&(
                other.global_flow_ordinal,
                other.owner,
                &other.block_child_path,
                other.owner_local_boundary,
            )))
    }
}

/// One owner-local boundary in canonical FlowTree preorder. The FlowTree,
/// rather than a caller-provided ordinal, assigns its global position.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum FlowBoundaryKind {
    DocumentStart,
    ParagraphItem,
    TableRow,
    ListItem,
    BlockItem,
    End,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct FlowBoundary {
    owner: NodeId,
    block_child_path: Vec<u32>,
    owner_local_boundary: u32,
    kind: FlowBoundaryKind,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CursorPosition {
    DocumentStart,
    ParagraphItem(u32),
    TableRow(u32),
    ListItem(u32),
    BlockItem(u32),
    End,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCursor {
    owner_node: NodeId,
    epoch: LayoutEpoch,
    position: FlowPosition,
    location: CursorPosition,
}
impl FlowCursor {
    pub fn document_start(flow: &FlowTree) -> Self {
        let position = flow.positions[0].clone();
        Self {
            owner_node: flow.root_node,
            epoch: flow.epoch,
            position,
            location: CursorPosition::DocumentStart,
        }
    }
    pub fn at(
        flow: &FlowTree,
        global_flow_ordinal: u64,
        location: CursorPosition,
    ) -> Result<Self, FragmentError> {
        let position = flow
            .positions
            .get(
                usize::try_from(global_flow_ordinal)
                    .map_err(|_| FragmentError::UnknownFlowPosition)?,
            )
            .ok_or(FragmentError::UnknownFlowPosition)?
            .clone();
        let terminal_ordinal = u64::try_from(flow.positions.len() - 1)
            .map_err(|_| FragmentError::UnknownFlowPosition)?;
        let location_matches = match location {
            CursorPosition::DocumentStart => {
                global_flow_ordinal == 0
                    && flow.boundary_kind(global_flow_ordinal)
                        == Some(FlowBoundaryKind::DocumentStart)
            }
            CursorPosition::ParagraphItem(index) => {
                index == position.owner_local_boundary()
                    && flow.boundary_kind(global_flow_ordinal)
                        == Some(FlowBoundaryKind::ParagraphItem)
            }
            CursorPosition::TableRow(index) => {
                index == position.owner_local_boundary()
                    && flow.boundary_kind(global_flow_ordinal) == Some(FlowBoundaryKind::TableRow)
            }
            CursorPosition::ListItem(index) => {
                index == position.owner_local_boundary()
                    && flow.boundary_kind(global_flow_ordinal) == Some(FlowBoundaryKind::ListItem)
            }
            CursorPosition::BlockItem(index) => {
                index == position.owner_local_boundary()
                    && flow.boundary_kind(global_flow_ordinal) == Some(FlowBoundaryKind::BlockItem)
            }
            CursorPosition::End => global_flow_ordinal == terminal_ordinal,
        };
        if !location_matches {
            return Err(FragmentError::InvalidCursorLocation);
        }
        Ok(Self {
            owner_node: position.owner(),
            epoch: flow.epoch,
            position,
            location,
        })
    }
    pub const fn owner_node(&self) -> NodeId {
        self.owner_node
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn position(&self) -> &FlowPosition {
        &self.position
    }
    pub fn location(&self) -> &CursorPosition {
        &self.location
    }
    pub fn is_end(&self) -> bool {
        matches!(self.location, CursorPosition::End)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowTree {
    root_node: NodeId,
    epoch: LayoutEpoch,
    positions: Vec<FlowPosition>,
    boundary_kinds: Vec<FlowBoundaryKind>,
    anchors: std::collections::BTreeMap<AnchorId, NodeId>,
    paragraph_items: Option<ValidatedParagraphItemRegistry>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowTreeError {
    MissingDocumentStart,
    DuplicateBoundary,
    NonDenseOwnerBoundary,
    TooManyBoundaries,
    UnknownOwner,
    InvalidOwnerKind,
    NonEmptyDocument,
    MissingOwnerBoundary,
    EpochPackageMismatch,
    InvalidOwnerBoundary,
    UnsupportedFlowDomain,
    ParagraphItemRegistryMismatch,
}

/// Sole issuer for canonical flow boundaries. Owners and typed child paths
/// come from a validated document index; owner-local ordinals are assigned by
/// this builder rather than supplied by layout workers.
pub struct CanonicalFlowIrBuilder<'a> {
    package: &'a ValidatedParsedPackage,
    paragraph_items: &'a ValidatedParagraphItemRegistry,
    boundaries: Vec<FlowBoundary>,
    inserted_boundaries: std::collections::BTreeMap<NodeId, u32>,
    separate_table_cell_flows: bool,
    separate_footnote_flows: bool,
}

impl<'a> CanonicalFlowIrBuilder<'a> {
    pub fn new(
        package: &'a ValidatedParsedPackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
    ) -> Result<Self, FlowTreeError> {
        Self::new_internal(package, paragraph_items, false)
    }

    /// Body-flow issuer for ADR-0030. Definition descendants stay exclusively
    /// in the dedicated FootnoteFlow registry and can never enter this cursor.
    pub fn new_for_footnote_body(
        package: &'a ValidatedParsedPackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
    ) -> Result<Self, FlowTreeError> {
        Self::new_internal(package, paragraph_items, true)
    }

    fn new_internal(
        package: &'a ValidatedParsedPackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
        separate_footnote_flows: bool,
    ) -> Result<Self, FlowTreeError> {
        if !separate_footnote_flows && !package.package().document.footnotes.is_empty() {
            return Err(FlowTreeError::UnsupportedFlowDomain);
        }
        if paragraph_items.epoch().document() != package.epoch_identity().document()
            || paragraph_items.epoch().style() != package.epoch_identity().style()
        {
            return Err(FlowTreeError::ParagraphItemRegistryMismatch);
        }
        let document_nodes = package.document_nodes();
        let root = NodeId::new(0);
        if document_nodes.node_kind(root) != Some(DocumentNodeKind::Document)
            || document_nodes.node_path(root) != Some([].as_slice())
        {
            return Err(FlowTreeError::MissingDocumentStart);
        }
        Ok(Self {
            package,
            paragraph_items,
            boundaries: vec![FlowBoundary {
                owner: root,
                block_child_path: Vec::new(),
                owner_local_boundary: 0,
                kind: FlowBoundaryKind::DocumentStart,
            }],
            inserted_boundaries: std::collections::BTreeMap::new(),
            separate_table_cell_flows: false,
            separate_footnote_flows,
        })
    }

    /// Keep table-cell paragraphs in their canonical child `FlowId`s instead
    /// of flattening them into the document-body cursor. The table profile is
    /// the only public caller; all other builders retain the closed M2 flow.
    pub fn use_separate_table_cell_flows(&mut self) {
        self.separate_table_cell_flows = true;
    }
    /// `item_index` is the semantic paragraph-item index, not a worker
    /// allocation ordinal. Finish canonicalizes insertion order and requires a
    /// dense 0-based owner-local sequence.
    pub fn push_paragraph_item(
        &mut self,
        owner: NodeId,
        item_index: u32,
    ) -> Result<(), FlowTreeError> {
        match self.package.document_nodes().node_kind(owner) {
            Some(DocumentNodeKind::Paragraph | DocumentNodeKind::Heading)
                if self
                    .paragraph_items
                    .item_count(owner)
                    .is_some_and(|count| item_index < count) =>
            {
                self.push(owner, item_index, FlowBoundaryKind::ParagraphItem)
            }
            Some(DocumentNodeKind::Paragraph | DocumentNodeKind::Heading) => {
                Err(FlowTreeError::InvalidOwnerBoundary)
            }
            Some(_) => Err(FlowTreeError::InvalidOwnerKind),
            None => Err(FlowTreeError::UnknownOwner),
        }
    }
    pub fn push_table_row(&mut self, owner: NodeId) -> Result<(), FlowTreeError> {
        match self.package.document_nodes().node_kind(owner) {
            Some(DocumentNodeKind::TableRow) => self.push(owner, 0, FlowBoundaryKind::TableRow),
            Some(_) => Err(FlowTreeError::InvalidOwnerKind),
            None => Err(FlowTreeError::UnknownOwner),
        }
    }
    pub fn push_list_item(&mut self, owner: NodeId) -> Result<(), FlowTreeError> {
        match self.package.document_nodes().node_kind(owner) {
            Some(DocumentNodeKind::ListItem) => self.push(owner, 0, FlowBoundaryKind::ListItem),
            Some(_) => Err(FlowTreeError::InvalidOwnerKind),
            None => Err(FlowTreeError::UnknownOwner),
        }
    }
    pub fn push_block_item(&mut self, owner: NodeId) -> Result<(), FlowTreeError> {
        match self.package.document_nodes().node_kind(owner) {
            Some(DocumentNodeKind::Figure | DocumentNodeKind::PageBreak) => {
                self.push(owner, 0, FlowBoundaryKind::BlockItem)
            }
            Some(_) => Err(FlowTreeError::InvalidOwnerKind),
            None => Err(FlowTreeError::UnknownOwner),
        }
    }
    fn push(
        &mut self,
        owner: NodeId,
        owner_local_boundary: u32,
        kind: FlowBoundaryKind,
    ) -> Result<(), FlowTreeError> {
        let path = self
            .package
            .document_nodes()
            .node_path(owner)
            .ok_or(FlowTreeError::UnknownOwner)?
            .to_vec();
        let inserted = self.inserted_boundaries.entry(owner).or_insert(0);
        *inserted = inserted
            .checked_add(1)
            .ok_or(FlowTreeError::TooManyBoundaries)?;
        self.boundaries.push(FlowBoundary {
            owner,
            block_child_path: path,
            owner_local_boundary,
            kind,
        });
        Ok(())
    }
    pub fn finish(self, epoch: LayoutEpoch) -> Result<FlowTree, FlowTreeError> {
        if epoch != self.paragraph_items.epoch()
            || epoch.document() != self.package.epoch_identity().document()
            || epoch.style() != self.package.epoch_identity().style()
        {
            return Err(FlowTreeError::EpochPackageMismatch);
        }
        let table_cell_descendants = if self.separate_table_cell_flows {
            table_cell_descendant_owners(&self.package.package().document.blocks)
        } else {
            std::collections::BTreeSet::new()
        };
        let footnote_descendants = if self.separate_footnote_flows {
            footnote_descendant_owners(
                &self.package.package().document,
                self.package.document_nodes(),
            )
        } else {
            std::collections::BTreeSet::new()
        };
        for (node_id, kind) in self.package.document_nodes().nodes() {
            let mut needs_boundary = matches!(
                kind,
                DocumentNodeKind::Paragraph
                    | DocumentNodeKind::Heading
                    | DocumentNodeKind::ListItem
                    | DocumentNodeKind::TableRow
                    | DocumentNodeKind::Figure
                    | DocumentNodeKind::PageBreak
            );
            if self.separate_table_cell_flows
                && matches!(
                    kind,
                    DocumentNodeKind::Paragraph | DocumentNodeKind::Heading
                )
                && table_cell_descendants.contains(&node_id)
            {
                needs_boundary = false;
            }
            if self.separate_footnote_flows && footnote_descendants.contains(&node_id) {
                needs_boundary = false;
            }
            if needs_boundary {
                let expected = self.paragraph_items.item_count(node_id).unwrap_or(1);
                let actual = self.inserted_boundaries.get(&node_id).copied().unwrap_or(0);
                if actual == 0 {
                    return Err(FlowTreeError::MissingOwnerBoundary);
                }
                if actual != expected {
                    return Err(FlowTreeError::InvalidOwnerBoundary);
                }
            }
        }
        let anchors = self
            .package
            .document_nodes()
            .anchors()
            .map(|(id, owner)| (id.clone(), owner))
            .collect();
        FlowTree::from_boundaries(
            NodeId::new(0),
            epoch,
            self.boundaries,
            anchors,
            Some(self.paragraph_items.clone()),
        )
    }
}

fn footnote_descendant_owners(
    document: &typaxis_document::Document,
    nodes: &ValidatedDocumentNodeIndex,
) -> std::collections::BTreeSet<NodeId> {
    let mut owners = std::collections::BTreeSet::new();
    for definition in &document.footnotes {
        let Some(path) = nodes.node_path(definition.node_id) else {
            continue;
        };
        for (owner, _) in nodes.nodes() {
            if owner == definition.node_id
                || nodes
                    .node_path(owner)
                    .is_some_and(|candidate| candidate.starts_with(path))
            {
                owners.insert(owner);
            }
        }
    }
    owners
}

fn table_cell_descendant_owners(blocks: &[Block]) -> std::collections::BTreeSet<NodeId> {
    let mut owners = std::collections::BTreeSet::new();
    let mut pending: Vec<(&Block, bool)> =
        blocks.iter().rev().map(|block| (block, false)).collect();
    while let Some((block, inside_cell)) = pending.pop() {
        match block {
            Block::Paragraph { node_id, .. } | Block::Heading { node_id, .. } => {
                if inside_cell {
                    owners.insert(*node_id);
                }
            }
            Block::List { items, .. } => pending.extend(
                items
                    .iter()
                    .rev()
                    .flat_map(|item| item.blocks.iter().rev())
                    .map(|block| (block, inside_cell)),
            ),
            Block::Table { head, body, .. } => pending.extend(
                body.iter()
                    .rev()
                    .chain(head.iter().rev())
                    .flat_map(|row| row.cells.iter().rev())
                    .flat_map(|cell| cell.blocks.iter().rev())
                    .map(|block| (block, true)),
            ),
            Block::Figure { caption, .. } => {
                pending.extend(caption.iter().rev().map(|block| (block, inside_cell)))
            }
            Block::PageBreak { .. } => {}
        }
    }
    owners
}

/// Paragraph-only flow builder reachable only after machine style/font
/// preparation has bound the same package and stable layout epoch.
pub struct MachineParagraphFlowBuilder<'a> {
    inner: CanonicalFlowIrBuilder<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineParagraphFlowError {
    PreparationMismatch,
    Flow(FlowTreeError),
}

impl<'a> MachineParagraphFlowBuilder<'a> {
    pub fn new(
        package: &'a ValidatedMachinePackage,
        paragraph_items: &'a ValidatedParagraphItemRegistry,
        preparation: &PreparedMachineStyleFonts,
    ) -> Result<Self, MachineParagraphFlowError> {
        if !preparation.matches_package_epoch(package, paragraph_items.epoch()) {
            return Err(MachineParagraphFlowError::PreparationMismatch);
        }
        let inner = CanonicalFlowIrBuilder::new(package.package(), paragraph_items)
            .map_err(MachineParagraphFlowError::Flow)?;
        Ok(Self { inner })
    }

    pub fn push_paragraph_item(
        &mut self,
        owner: NodeId,
        item_index: u32,
    ) -> Result<(), MachineParagraphFlowError> {
        self.inner
            .push_paragraph_item(owner, item_index)
            .map_err(MachineParagraphFlowError::Flow)
    }

    pub fn finish(self, epoch: LayoutEpoch) -> Result<FlowTree, MachineParagraphFlowError> {
        self.inner
            .finish(epoch)
            .map_err(MachineParagraphFlowError::Flow)
    }
}

impl FlowTree {
    fn from_boundaries(
        root_node: NodeId,
        epoch: LayoutEpoch,
        mut boundaries: Vec<FlowBoundary>,
        anchors: std::collections::BTreeMap<AnchorId, NodeId>,
        paragraph_items: Option<ValidatedParagraphItemRegistry>,
    ) -> Result<Self, FlowTreeError> {
        boundaries.sort_by(|left, right| {
            (
                left.owner,
                &left.block_child_path,
                left.owner_local_boundary,
            )
                .cmp(&(
                    right.owner,
                    &right.block_child_path,
                    right.owner_local_boundary,
                ))
        });
        if !matches!(
            boundaries.first(),
            Some(boundary)
                if boundary.owner == root_node
                    && boundary.block_child_path.is_empty()
                    && boundary.owner_local_boundary == 0
        ) {
            return Err(FlowTreeError::MissingDocumentStart);
        }
        let mut unique = std::collections::BTreeSet::new();
        let mut positions = Vec::with_capacity(boundaries.len());
        let mut boundary_kinds = Vec::with_capacity(boundaries.len());
        let mut previous_group: Option<(NodeId, Vec<u32>, u32)> = None;
        for (ordinal, boundary) in boundaries.into_iter().enumerate() {
            if !unique.insert((
                boundary.owner,
                boundary.block_child_path.clone(),
                boundary.owner_local_boundary,
            )) {
                return Err(FlowTreeError::DuplicateBoundary);
            }
            let expected_local = match &previous_group {
                Some((owner, path, local))
                    if *owner == boundary.owner && *path == boundary.block_child_path =>
                {
                    local
                        .checked_add(1)
                        .ok_or(FlowTreeError::NonDenseOwnerBoundary)?
                }
                _ => 0,
            };
            if boundary.owner_local_boundary != expected_local {
                return Err(FlowTreeError::NonDenseOwnerBoundary);
            }
            previous_group = Some((
                boundary.owner,
                boundary.block_child_path.clone(),
                boundary.owner_local_boundary,
            ));
            positions.push(FlowPosition::new(
                epoch,
                u64::try_from(ordinal).map_err(|_| FlowTreeError::TooManyBoundaries)?,
                boundary.owner,
                boundary.block_child_path,
                boundary.owner_local_boundary,
            ));
            boundary_kinds.push(boundary.kind);
        }
        if positions.len() > 1 {
            positions.push(FlowPosition::new(
                epoch,
                u64::try_from(positions.len()).map_err(|_| FlowTreeError::TooManyBoundaries)?,
                root_node,
                Vec::new(),
                1,
            ));
            boundary_kinds.push(FlowBoundaryKind::End);
        }
        Ok(Self {
            root_node,
            epoch,
            positions,
            boundary_kinds,
            anchors,
            paragraph_items,
        })
    }
    pub fn empty(
        package: &ValidatedParsedPackage,
        epoch: LayoutEpoch,
    ) -> Result<Self, FlowTreeError> {
        if package.document_nodes().node_count() != 1 {
            return Err(FlowTreeError::NonEmptyDocument);
        }
        if epoch.document() != package.epoch_identity().document()
            || epoch.style() != package.epoch_identity().style()
        {
            return Err(FlowTreeError::EpochPackageMismatch);
        }
        FlowTree::from_boundaries(
            NodeId::new(0),
            epoch,
            vec![FlowBoundary {
                owner: NodeId::new(0),
                block_child_path: Vec::new(),
                owner_local_boundary: 0,
                kind: FlowBoundaryKind::DocumentStart,
            }],
            std::collections::BTreeMap::new(),
            None,
        )
    }
    pub const fn root_node(&self) -> NodeId {
        self.root_node
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub fn positions(&self) -> &[FlowPosition] {
        &self.positions
    }
    pub fn paragraph_items(&self) -> Option<&ValidatedParagraphItemRegistry> {
        self.paragraph_items.as_ref()
    }
    pub fn contains_position(&self, position: &FlowPosition) -> bool {
        position.epoch() == self.epoch
            && usize::try_from(position.global_flow_ordinal())
                .ok()
                .and_then(|index| self.positions.get(index))
                == Some(position)
    }
    pub fn contains_owner(&self, owner: NodeId) -> bool {
        self.positions
            .iter()
            .any(|position| position.owner() == owner)
    }
    pub fn anchor_owner(&self, anchor_id: &AnchorId) -> Option<NodeId> {
        self.anchors.get(anchor_id).copied()
    }
    pub fn anchors(&self) -> impl ExactSizeIterator<Item = (&AnchorId, NodeId)> {
        self.anchors.iter().map(|(id, owner)| (id, *owner))
    }
    pub fn terminal_cursor(&self) -> FlowCursor {
        let position = self.positions[self.positions.len() - 1].clone();
        FlowCursor {
            owner_node: position.owner(),
            epoch: self.epoch,
            position,
            location: CursorPosition::End,
        }
    }
    fn boundary_kind(&self, ordinal: u64) -> Option<FlowBoundaryKind> {
        usize::try_from(ordinal)
            .ok()
            .and_then(|index| self.boundary_kinds.get(index))
            .copied()
    }
    fn is_terminal_position(&self, position: &FlowPosition) -> bool {
        self.positions.last() == Some(position)
    }
    fn is_document_bootstrap(&self, start: &FlowPosition, next: &FlowPosition) -> bool {
        self.contains_position(start)
            && self.contains_position(next)
            && start.global_flow_ordinal() == 0
            && self.boundary_kind(0) == Some(FlowBoundaryKind::DocumentStart)
            && next.global_flow_ordinal() == 1
    }
    fn is_paintable_position(&self, position: &FlowPosition) -> bool {
        self.contains_position(position)
            && matches!(
                self.boundary_kind(position.global_flow_ordinal()),
                Some(
                    FlowBoundaryKind::ParagraphItem
                        | FlowBoundaryKind::TableRow
                        | FlowBoundaryKind::ListItem
                        | FlowBoundaryKind::BlockItem
                )
            )
    }
}

/// Flow-issued site at which the next page begins. The next content owner can
/// differ from the cursor owner at document start.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPageSelection {
    page_start: FlowPosition,
    flow_owner: NodeId,
    content_owner: NodeId,
    style_owner: NodeId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    page_name: Option<PageName>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageStyleResolutionError {
    InvalidCursor,
    EpochPackageMismatch,
    InvalidPackageStyle(PackageStyleError),
}
impl ResolvedPageSelection {
    pub fn new(
        flow: &FlowTree,
        cursor: &FlowCursor,
        package: &ValidatedParsedPackage,
    ) -> Result<Self, PageStyleResolutionError> {
        if cursor.epoch() != flow.epoch()
            || !flow.contains_position(cursor.position())
            || cursor.owner_node() != cursor.position().owner()
        {
            return Err(PageStyleResolutionError::InvalidCursor);
        }
        if flow.epoch().document() != package.epoch_identity().document()
            || flow.epoch().style() != package.epoch_identity().style()
        {
            return Err(PageStyleResolutionError::EpochPackageMismatch);
        }
        let current = usize::try_from(cursor.position().global_flow_ordinal())
            .map_err(|_| PageStyleResolutionError::InvalidCursor)?;
        let blank = flow.positions.len() == 1;
        if blank && !matches!(cursor.location(), CursorPosition::DocumentStart) {
            return Err(PageStyleResolutionError::InvalidCursor);
        }
        let content_owner = if blank {
            flow.root_node
        } else if flow.boundary_kinds[current] == FlowBoundaryKind::DocumentStart {
            flow.positions
                .get(current + 1)
                .ok_or(PageStyleResolutionError::InvalidCursor)?
                .owner()
        } else if cursor.is_end() {
            return Err(PageStyleResolutionError::InvalidCursor);
        } else {
            cursor.position().owner()
        };
        let package_selection = if blank {
            package.resolve_blank_page_selection()
        } else {
            package.resolve_page_selection(content_owner)
        }
        .map_err(PageStyleResolutionError::InvalidPackageStyle)?;
        if package_selection.owner() != content_owner {
            return Err(PageStyleResolutionError::InvalidPackageStyle(
                PackageStyleError::UnknownStyleOwner,
            ));
        }
        Ok(Self {
            page_start: cursor.position().clone(),
            flow_owner: cursor.owner_node(),
            content_owner,
            style_owner: package_selection.style_owner(),
            document: package_selection.document_fingerprint(),
            style: package_selection.style_fingerprint(),
            page_name: package_selection.page_name().cloned(),
        })
    }

    /// Holds the terminal body cursor while a dedicated footnote subflow
    /// advances on a carry-only page. The footnote profile has exactly one
    /// default master and no selection rules, so no content style may select a
    /// different page here.
    pub fn for_footnote_terminal_carry(
        flow: &FlowTree,
        cursor: &FlowCursor,
        package: &ValidatedParsedPackage,
    ) -> Result<Self, PageStyleResolutionError> {
        if cursor.epoch() != flow.epoch()
            || !cursor.is_end()
            || flow.positions.last() != Some(cursor.position())
            || flow.epoch().document() != package.epoch_identity().document()
            || flow.epoch().style() != package.epoch_identity().style()
        {
            return Err(PageStyleResolutionError::InvalidCursor);
        }
        Ok(Self {
            page_start: cursor.position().clone(),
            flow_owner: cursor.owner_node(),
            content_owner: flow.root_node,
            style_owner: flow.root_node,
            document: flow.epoch().document(),
            style: flow.epoch().style(),
            page_name: None,
        })
    }
    pub const fn page_start(&self) -> &FlowPosition {
        &self.page_start
    }
    pub const fn flow_owner(&self) -> NodeId {
        self.flow_owner
    }
    pub const fn content_owner(&self) -> NodeId {
        self.content_owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
}

/// Page parity and first-page status are derived, never independently stored.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageContext {
    page_index: u32,
    physical_page_number: NonZeroU32,
    named_page: Option<PageName>,
    page_start: FlowPosition,
    flow_owner: NodeId,
    content_owner: NodeId,
    style_owner: NodeId,
    package_document: DocumentFingerprint,
    package_style: StyleFingerprint,
    selected_master: PageMaster,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageContextError {
    PageNumberOverflow,
    InvalidPageMasters(PageMasterValidationError),
    InvalidPageSelection(PageSelectionError),
    PackageStyleMismatch,
}
impl PageContext {
    pub fn select(
        page_index: u32,
        resolved_page: &ResolvedPageSelection,
        package_context: &PackagePaginationContext,
    ) -> Result<Self, PageContextError> {
        if resolved_page.document != package_context.document_fingerprint()
            || resolved_page.style != package_context.style_fingerprint()
        {
            return Err(PageContextError::PackageStyleMismatch);
        }
        let physical_page_number = page_index
            .checked_add(1)
            .and_then(NonZeroU32::new)
            .ok_or(PageContextError::PageNumberOverflow)?;
        let named_page = resolved_page.page_name.clone();
        let selection = PageSelectionContext::new(page_index, named_page.clone())
            .map_err(PageContextError::InvalidPageSelection)?;
        let selected_master = package_context
            .page_masters()
            .select(&selection)
            .map_err(PageContextError::InvalidPageMasters)?
            .clone();
        Ok(Self {
            page_index,
            physical_page_number,
            named_page,
            page_start: resolved_page.page_start.clone(),
            flow_owner: resolved_page.flow_owner,
            content_owner: resolved_page.content_owner,
            style_owner: resolved_page.style_owner,
            package_document: package_context.document_fingerprint(),
            package_style: package_context.style_fingerprint(),
            selected_master,
        })
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn physical_page_number(&self) -> NonZeroU32 {
        self.physical_page_number
    }
    pub const fn master_id(&self) -> &MasterId {
        &self.selected_master.master_id
    }
    /// Returns the exact validated page master selected for this page. Work
    /// permits bind frame geometry to this receipt before layout begins.
    pub const fn selected_master(&self) -> &PageMaster {
        &self.selected_master
    }
    pub const fn package_document_fingerprint(&self) -> DocumentFingerprint {
        self.package_document
    }
    pub const fn package_style_fingerprint(&self) -> StyleFingerprint {
        self.package_style
    }
    pub const fn named_page(&self) -> Option<&PageName> {
        self.named_page.as_ref()
    }
    pub const fn page_start(&self) -> &FlowPosition {
        &self.page_start
    }
    pub const fn flow_owner(&self) -> NodeId {
        self.flow_owner
    }
    pub const fn content_owner(&self) -> NodeId {
        self.content_owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn is_first(&self) -> bool {
        self.page_index == 0
    }
    pub const fn is_odd(&self) -> bool {
        self.physical_page_number.get() % 2 == 1
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FragmentRequest<'a> {
    flow: &'a FlowTree,
    cursor: &'a FlowCursor,
    frame: Rect,
    reserved_footnote_height: NonNegativeLength,
    page: PageContext,
}
impl<'a> FragmentRequest<'a> {
    pub fn new(
        flow: &'a FlowTree,
        cursor: &'a FlowCursor,
        frame: Rect,
        reserved_footnote_height: NonNegativeLength,
        page: PageContext,
    ) -> Result<Self, FragmentError> {
        let request = Self {
            flow,
            cursor,
            frame,
            reserved_footnote_height,
            page,
        };
        request.validate()?;
        Ok(request)
    }
    pub fn validate(&self) -> Result<(), FragmentError> {
        if self.cursor.epoch() != self.flow.epoch {
            return Err(FragmentError::InvalidCursorEpoch);
        }
        if self.page.package_document_fingerprint() != self.flow.epoch.document()
            || self.page.package_style_fingerprint() != self.flow.epoch.style()
            || self.page.page_start().epoch() != self.flow.epoch
            || !self.flow.contains_position(self.page.page_start())
            || self.page.flow_owner() != self.page.page_start().owner()
        {
            return Err(FragmentError::InvalidPageContext);
        }
        if !self.flow.contains_position(self.cursor.position()) {
            return Err(FragmentError::UnknownFlowPosition);
        }
        if self.cursor.owner_node() != self.cursor.position().owner() {
            return Err(FragmentError::InvalidCursorOwner);
        }
        Ok(())
    }
    pub const fn flow(&self) -> &FlowTree {
        self.flow
    }
    pub const fn cursor(&self) -> &FlowCursor {
        self.cursor
    }
    pub const fn frame(&self) -> Rect {
        self.frame
    }
    pub const fn reserved_footnote_height(&self) -> NonNegativeLength {
        self.reserved_footnote_height
    }
    pub const fn page(&self) -> &PageContext {
        &self.page
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FragmentDraft {
    start: FlowPosition,
    end: FlowPosition,
    bounds: Rect,
    break_after_penalty: i32,
}
impl FragmentDraft {
    pub fn new(
        start: FlowPosition,
        end: FlowPosition,
        bounds: Rect,
        break_after_penalty: i32,
    ) -> Result<Self, FragmentError> {
        if start.cmp_within_epoch(&end)? != Ordering::Less {
            return Err(FragmentError::InvalidFragmentRange);
        }
        Ok(Self {
            start,
            end,
            bounds,
            break_after_penalty,
        })
    }
    pub const fn start(&self) -> &FlowPosition {
        &self.start
    }
    pub const fn end(&self) -> &FlowPosition {
        &self.end
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
    pub const fn break_after_penalty(&self) -> i32 {
        self.break_after_penalty
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredAnchor {
    pub anchor_id: AnchorId,
    pub owner_node: NodeId,
    pub position_in_frame: Point,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Continuation {
    Exhausted(Box<FlowCursor>),
    More(Box<FlowCursor>),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FragmentResult {
    pub fragments: Vec<FragmentDraft>,
    pub continuation: Continuation,
    pub discovered_footnotes: Vec<FootnoteId>,
    pub discovered_anchors: Vec<DiscoveredAnchor>,
}
impl FragmentResult {
    pub fn validate_progress(&self, request: &FragmentRequest<'_>) -> Result<(), FragmentError> {
        request.validate()?;
        let input = request.cursor();
        let continuation = match &self.continuation {
            Continuation::Exhausted(terminal)
                if terminal.epoch() != input.epoch()
                    || !request.flow().contains_position(terminal.position()) =>
            {
                return Err(FragmentError::InvalidCursorEpoch);
            }
            Continuation::Exhausted(terminal) if !terminal.is_end() => {
                return Err(FragmentError::InvalidCursorLocation);
            }
            Continuation::Exhausted(terminal) => {
                match terminal.position().cmp_within_epoch(input.position())? {
                    Ordering::Greater | Ordering::Equal => terminal.position(),
                    Ordering::Less => return Err(FragmentError::NoProgress),
                }
            }
            Continuation::More(next) if next.epoch() != input.epoch() => {
                return Err(FragmentError::InvalidCursorEpoch);
            }
            Continuation::More(next)
                if next.is_end() || request.flow().is_terminal_position(next.position()) =>
            {
                return Err(FragmentError::InvalidCursorLocation);
            }
            Continuation::More(next) => match next.position().cmp_within_epoch(input.position())? {
                Ordering::Greater => next.position(),
                Ordering::Equal | Ordering::Less => return Err(FragmentError::NoProgress),
            },
        };
        if self.fragments.is_empty()
            && continuation.cmp_within_epoch(input.position())? == Ordering::Greater
            && !request
                .flow()
                .is_document_bootstrap(input.position(), continuation)
        {
            return Err(FragmentError::InvalidFragmentRange);
        }
        let mut previous_end: Option<&FlowPosition> = None;
        for (index, fragment) in self.fragments.iter().enumerate() {
            if !request.flow().contains_position(fragment.start())
                || !request.flow().contains_position(fragment.end())
            {
                return Err(FragmentError::UnknownFlowPosition);
            }
            if !request.flow().is_paintable_position(fragment.start())
                || fragment.start().cmp_within_epoch(fragment.end())? != Ordering::Less
                || (index == 0 && fragment.start() != input.position())
                || fragment.end().cmp_within_epoch(continuation)? == Ordering::Greater
                || previous_end.is_some_and(|end| {
                    end.cmp_within_epoch(fragment.start())
                        .is_ok_and(|ordering| ordering != Ordering::Equal)
                })
            {
                return Err(FragmentError::InvalidFragmentRange);
            }
            previous_end = Some(fragment.end());
        }
        if previous_end.is_some_and(|end| end != continuation) {
            return Err(FragmentError::InvalidFragmentRange);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FragmentError {
    InvalidCursorEpoch,
    InvalidCursorOwner,
    InvalidCursorLocation,
    UnknownFlowPosition,
    NoProgress,
    Unplaceable,
    ArithmeticOverflow,
    ResourceLimit,
    InvalidFragmentRange,
    InvalidFragmentKey,
    InvalidPageContext,
    InvalidFloatState,
    UnsupportedFlowDomain,
}
pub trait FragmentWorkBudget {
    fn consume_fragments(&mut self, count: u64) -> Result<(), FragmentError>;
    fn consume_footnote_reflow(&mut self, page_index: u32) -> Result<(), FragmentError>;
    fn consume_column_candidate(&mut self, container: NodeId) -> Result<(), FragmentError>;
    fn enqueue_float(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError>;
    fn dequeue_float(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError>;
    fn consume_float_carry(
        &mut self,
        owner: NodeId,
        owner_local_ordinal: u32,
    ) -> Result<(), FragmentError>;
}
pub trait Fragmenter {
    fn fragment(
        &self,
        request: &FragmentRequest<'_>,
        budget: &mut dyn FragmentWorkBudget,
    ) -> Result<FragmentResult, FragmentError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReferenceAnchorPlacement {
    flow_ordinal: u64,
    anchor_id: AnchorId,
    owner_node: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReferenceFootnotePlacement {
    flow_ordinal: u64,
    reference_owner: NodeId,
    footnote_id: FootnoteId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReferenceLinePlacement {
    start: usize,
    end: usize,
    height: PositiveLength,
    forced_break: bool,
    keep_with_next: bool,
}

fn reference_line_height(
    package: &ValidatedParsedPackage,
    owner: NodeId,
) -> Result<PositiveLength, FragmentError> {
    let computed = package
        .cascade_style(owner)
        .map_err(|_| FragmentError::InvalidFragmentKey)?;
    match computed.computed().properties().get("line_height") {
        Some(StyleValue::Length(value)) => {
            PositiveLength::new(*value).ok_or(FragmentError::InvalidFragmentKey)
        }
        // Empty reference paragraphs intentionally have no text style. Their
        // legacy fragment mode paints no glyphs and replaces this placeholder
        // with the complete requested frame.
        None => PositiveLength::new(Length::from_raw(1).ok_or(FragmentError::ArithmeticOverflow)?)
            .ok_or(FragmentError::ArithmeticOverflow),
        Some(_) => Err(FragmentError::InvalidFragmentKey),
    }
}

fn reference_keep_with_next(
    package: &ValidatedParsedPackage,
    owner: NodeId,
) -> Result<bool, FragmentError> {
    let computed = package
        .cascade_style(owner)
        .map_err(|_| FragmentError::InvalidFragmentKey)?;
    computed
        .computed()
        .basic_keep_with_next()
        .map_err(|_| FragmentError::InvalidFragmentKey)
}

/// Deterministic reference fragmenter for validated top-level paragraphs and
/// headings. Line ranges come from the paragraph-break receipts retained by
/// the canonical FlowTree; callers cannot substitute item counts or breaks.
#[derive(Clone, Debug)]
pub struct ReferenceFragmenter<'flow> {
    flow: &'flow FlowTree,
    anchors: Vec<ReferenceAnchorPlacement>,
    footnotes: Vec<ReferenceFootnotePlacement>,
    lines: Vec<ReferenceLinePlacement>,
    legacy_full_frame: bool,
    basic_document: bool,
    enforce_keep_with_next: bool,
}

impl<'flow> ReferenceFragmenter<'flow> {
    pub fn for_empty_paragraphs(
        package: &ValidatedParsedPackage,
        flow: &'flow FlowTree,
    ) -> Result<Self, FragmentError> {
        if !package.package().document.footnotes.is_empty()
            || !package.package().text_store.buffers().is_empty()
            || package.document_nodes().generated_sites().len() != 0
        {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        let mut fragmenter = Self::for_paragraphs(package, flow)?;
        fragmenter.legacy_full_frame = true;
        Ok(fragmenter)
    }

    pub fn for_paragraphs(
        package: &ValidatedParsedPackage,
        flow: &'flow FlowTree,
    ) -> Result<Self, FragmentError> {
        if !package.package().document.footnotes.is_empty() {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        if flow.epoch.document() != package.epoch_identity().document()
            || flow.epoch.style() != package.epoch_identity().style()
        {
            return Err(FragmentError::InvalidCursorEpoch);
        }
        let Some(root_position) = flow.positions.first() else {
            return Err(FragmentError::InvalidFragmentKey);
        };
        if flow.root_node != NodeId::new(0)
            || package.document_nodes().node_kind(flow.root_node)
                != Some(DocumentNodeKind::Document)
            || root_position.owner() != flow.root_node
            || !root_position.block_child_path().is_empty()
            || root_position.owner_local_boundary() != 0
            || flow.boundary_kinds.first() != Some(&FlowBoundaryKind::DocumentStart)
        {
            return Err(FragmentError::InvalidFragmentKey);
        }

        let blocks = &package.package().document.blocks;
        if blocks
            .iter()
            .any(|block| !matches!(block, Block::Paragraph { .. } | Block::Heading { .. }))
        {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        let registry = flow.paragraph_items();
        if !blocks.is_empty() && registry.is_none() {
            return Err(FragmentError::InvalidFragmentKey);
        }
        let paragraph_item_count = blocks.iter().try_fold(0usize, |total, block| {
            let node_id = match block {
                Block::Paragraph { node_id, .. } | Block::Heading { node_id, .. } => *node_id,
                _ => return Err(FragmentError::UnsupportedFlowDomain),
            };
            let count = registry
                .and_then(|items| items.item_count(node_id))
                .ok_or(FragmentError::InvalidFragmentKey)?;
            total
                .checked_add(usize::try_from(count).map_err(|_| FragmentError::ArithmeticOverflow)?)
                .ok_or(FragmentError::ArithmeticOverflow)
        })?;
        let expected_position_count = if blocks.is_empty() {
            1
        } else {
            paragraph_item_count
                .checked_add(2)
                .ok_or(FragmentError::ArithmeticOverflow)?
        };
        if flow.positions.len() != expected_position_count
            || flow.boundary_kinds.len() != expected_position_count
        {
            return Err(FragmentError::InvalidFragmentKey);
        }

        let mut anchors = Vec::new();
        let mut lines = Vec::new();
        let mut position_index = 1usize;
        for block in blocks {
            let (node_id, heading_anchor, children) = match block {
                Block::Paragraph {
                    node_id, children, ..
                } => (*node_id, None, children.as_slice()),
                Block::Heading {
                    node_id,
                    anchor_id,
                    children,
                    ..
                } => (*node_id, anchor_id.as_ref(), children.as_slice()),
                _ => return Err(FragmentError::UnsupportedFlowDomain),
            };
            let expected_path = package
                .document_nodes()
                .node_path(node_id)
                .ok_or(FragmentError::InvalidFragmentKey)?;
            let item_count = registry
                .and_then(|items| items.item_count(node_id))
                .ok_or(FragmentError::InvalidFragmentKey)?;
            let line_height = reference_line_height(package, node_id)?;
            for local in 0..item_count {
                let position = flow
                    .positions
                    .get(position_index)
                    .ok_or(FragmentError::InvalidFragmentKey)?;
                if position.owner() != node_id
                    || position.block_child_path() != expected_path
                    || position.owner_local_boundary() != local
                    || flow.boundary_kinds[position_index] != FlowBoundaryKind::ParagraphItem
                {
                    return Err(FragmentError::InvalidFragmentKey);
                }
                position_index = position_index
                    .checked_add(1)
                    .ok_or(FragmentError::ArithmeticOverflow)?;
            }
            let paragraph_start = position_index
                .checked_sub(
                    usize::try_from(item_count).map_err(|_| FragmentError::ArithmeticOverflow)?,
                )
                .ok_or(FragmentError::ArithmeticOverflow)?;
            let mut previous_item = 0u32;
            if let Some(result) = registry.and_then(|items| items.paragraph_break(node_id)) {
                for line in &result.lines {
                    if line.item_index <= previous_item || line.item_index > item_count {
                        return Err(FragmentError::InvalidFragmentKey);
                    }
                    lines.push(ReferenceLinePlacement {
                        start: paragraph_start
                            .checked_add(
                                usize::try_from(previous_item)
                                    .map_err(|_| FragmentError::ArithmeticOverflow)?,
                            )
                            .ok_or(FragmentError::ArithmeticOverflow)?,
                        end: paragraph_start
                            .checked_add(
                                usize::try_from(line.item_index)
                                    .map_err(|_| FragmentError::ArithmeticOverflow)?,
                            )
                            .ok_or(FragmentError::ArithmeticOverflow)?,
                        height: line_height,
                        forced_break: false,
                        keep_with_next: false,
                    });
                    previous_item = line.item_index;
                }
                if previous_item != item_count {
                    return Err(FragmentError::InvalidFragmentKey);
                }
            } else {
                lines.push(ReferenceLinePlacement {
                    start: paragraph_start,
                    end: position_index,
                    height: line_height,
                    forced_break: false,
                    keep_with_next: false,
                });
            }
            if let Some(anchor_id) = heading_anchor {
                if package.document_nodes().anchor_owner(anchor_id) != Some(node_id)
                    || flow.anchor_owner(anchor_id) != Some(node_id)
                {
                    return Err(FragmentError::InvalidFragmentKey);
                }
                anchors.push(ReferenceAnchorPlacement {
                    flow_ordinal: u64::try_from(paragraph_start)
                        .map_err(|_| FragmentError::ArithmeticOverflow)?,
                    anchor_id: anchor_id.clone(),
                    owner_node: node_id,
                });
            }
            collect_reference_anchors(
                children,
                package,
                flow,
                u64::try_from(paragraph_start).map_err(|_| FragmentError::ArithmeticOverflow)?,
                &mut anchors,
            )?;
        }

        if !blocks.is_empty() {
            let terminal_index = expected_position_count - 1;
            let terminal = &flow.positions[terminal_index];
            if terminal.owner() != flow.root_node
                || !terminal.block_child_path().is_empty()
                || terminal.owner_local_boundary() != 1
                || flow.boundary_kinds[terminal_index] != FlowBoundaryKind::End
            {
                return Err(FragmentError::InvalidFragmentKey);
            }
        }

        anchors.sort_by(|left, right| {
            (left.flow_ordinal, &left.anchor_id).cmp(&(right.flow_ordinal, &right.anchor_id))
        });
        if anchors
            .windows(2)
            .any(|pair| pair[0].anchor_id == pair[1].anchor_id)
            || anchors.len() != flow.anchors.len()
            || anchors
                .iter()
                .any(|anchor| flow.anchor_owner(&anchor.anchor_id) != Some(anchor.owner_node))
        {
            return Err(FragmentError::InvalidFragmentKey);
        }
        Ok(Self {
            flow,
            anchors,
            footnotes: Vec::new(),
            lines,
            legacy_full_frame: false,
            basic_document: false,
            enforce_keep_with_next: false,
        })
    }

    /// Fragment the immutable basic-document profile from the same canonical
    /// FlowTree used by paragraph pagination. Non-paragraph boundaries remain
    /// typed: list items and figures produce boxes, and page breaks produce a
    /// paint-free box which terminates the current page.
    pub fn for_basic_document(
        package: &ValidatedParsedPackage,
        flow: &'flow FlowTree,
    ) -> Result<Self, FragmentError> {
        Self::for_basic_document_internal(package, flow, false)
    }

    /// Footnote-profile body fragmenter. Definition descendants remain outside
    /// this cursor, while their anchors stay in the complete FlowTree so the
    /// pagination owner can place them from selected definition fragments.
    pub fn for_footnote_body(
        package: &ValidatedParsedPackage,
        flow: &'flow FlowTree,
    ) -> Result<Self, FragmentError> {
        Self::for_basic_document_internal(package, flow, true)
    }

    fn for_basic_document_internal(
        package: &ValidatedParsedPackage,
        flow: &'flow FlowTree,
        allow_footnotes: bool,
    ) -> Result<Self, FragmentError> {
        if (!allow_footnotes && !package.package().document.footnotes.is_empty())
            || flow.epoch.document() != package.epoch_identity().document()
            || flow.epoch.style() != package.epoch_identity().style()
        {
            return Err(FragmentError::UnsupportedFlowDomain);
        }
        let registry = flow
            .paragraph_items()
            .ok_or(FragmentError::InvalidFragmentKey)?;
        let terminal = flow
            .positions
            .len()
            .checked_sub(1)
            .ok_or(FragmentError::InvalidFragmentKey)?;
        if flow.boundary_kinds.first() != Some(&FlowBoundaryKind::DocumentStart)
            || flow.boundary_kinds.get(terminal) != Some(&FlowBoundaryKind::End)
        {
            return Err(FragmentError::InvalidFragmentKey);
        }

        let paragraphs = basic_paragraph_blocks(&package.package().document.blocks);
        let by_owner: std::collections::BTreeMap<_, _> = paragraphs
            .into_iter()
            .map(|block| (basic_block_node_id(block), block))
            .collect();
        let mut anchors = Vec::new();
        let mut footnotes = Vec::new();
        let mut lines = Vec::new();
        let mut index = 1usize;
        while index < terminal {
            let position = flow
                .positions
                .get(index)
                .ok_or(FragmentError::InvalidFragmentKey)?;
            match flow.boundary_kinds.get(index) {
                Some(FlowBoundaryKind::ParagraphItem) => {
                    let owner = position.owner();
                    if position.owner_local_boundary() != 0 {
                        return Err(FragmentError::InvalidFragmentKey);
                    }
                    let item_count = registry
                        .item_count(owner)
                        .ok_or(FragmentError::InvalidFragmentKey)?;
                    let paragraph_end = index
                        .checked_add(
                            usize::try_from(item_count)
                                .map_err(|_| FragmentError::ArithmeticOverflow)?,
                        )
                        .ok_or(FragmentError::ArithmeticOverflow)?;
                    if paragraph_end > terminal {
                        return Err(FragmentError::InvalidFragmentKey);
                    }
                    for (local, observed) in flow.positions[index..paragraph_end].iter().enumerate()
                    {
                        if observed.owner() != owner
                            || observed.owner_local_boundary()
                                != u32::try_from(local)
                                    .map_err(|_| FragmentError::ArithmeticOverflow)?
                            || flow.boundary_kinds[index + local] != FlowBoundaryKind::ParagraphItem
                        {
                            return Err(FragmentError::InvalidFragmentKey);
                        }
                    }
                    let height = reference_line_height(package, owner)?;
                    let mut previous = 0u32;
                    if let Some(result) = registry.paragraph_break(owner) {
                        for line in &result.lines {
                            if line.item_index <= previous || line.item_index > item_count {
                                return Err(FragmentError::InvalidFragmentKey);
                            }
                            lines.push(ReferenceLinePlacement {
                                start: index
                                    .checked_add(
                                        usize::try_from(previous)
                                            .map_err(|_| FragmentError::ArithmeticOverflow)?,
                                    )
                                    .ok_or(FragmentError::ArithmeticOverflow)?,
                                end: index
                                    .checked_add(
                                        usize::try_from(line.item_index)
                                            .map_err(|_| FragmentError::ArithmeticOverflow)?,
                                    )
                                    .ok_or(FragmentError::ArithmeticOverflow)?,
                                height,
                                forced_break: false,
                                keep_with_next: false,
                            });
                            previous = line.item_index;
                        }
                        if previous != item_count {
                            return Err(FragmentError::InvalidFragmentKey);
                        }
                    } else {
                        lines.push(ReferenceLinePlacement {
                            start: index,
                            end: paragraph_end,
                            height,
                            forced_break: false,
                            keep_with_next: false,
                        });
                    }
                    let block = by_owner
                        .get(&owner)
                        .ok_or(FragmentError::InvalidFragmentKey)?;
                    let (heading_anchor, children) = match block {
                        Block::Paragraph { children, .. } => (None, children.as_slice()),
                        Block::Heading {
                            anchor_id,
                            children,
                            ..
                        } => (anchor_id.as_ref(), children.as_slice()),
                        _ => return Err(FragmentError::InvalidFragmentKey),
                    };
                    if allow_footnotes && reference_keep_with_next(package, owner)? {
                        lines
                            .last_mut()
                            .ok_or(FragmentError::InvalidFragmentKey)?
                            .keep_with_next = true;
                    }
                    if let Some(anchor_id) = heading_anchor {
                        anchors.push(ReferenceAnchorPlacement {
                            flow_ordinal: u64::try_from(index)
                                .map_err(|_| FragmentError::ArithmeticOverflow)?,
                            anchor_id: anchor_id.clone(),
                            owner_node: owner,
                        });
                    }
                    collect_reference_anchors(
                        children,
                        package,
                        flow,
                        u64::try_from(index).map_err(|_| FragmentError::ArithmeticOverflow)?,
                        &mut anchors,
                    )?;
                    collect_reference_footnotes(children, owner, index, registry, &mut footnotes)?;
                    index = paragraph_end;
                }
                Some(FlowBoundaryKind::ListItem) => {
                    lines.push(ReferenceLinePlacement {
                        start: index,
                        end: index
                            .checked_add(1)
                            .ok_or(FragmentError::ArithmeticOverflow)?,
                        height: basic_boundary_height(package, position.owner(), false)?,
                        forced_break: false,
                        keep_with_next: false,
                    });
                    index = index
                        .checked_add(1)
                        .ok_or(FragmentError::ArithmeticOverflow)?;
                }
                Some(FlowBoundaryKind::BlockItem) => {
                    let forced_break = package.document_nodes().node_kind(position.owner())
                        == Some(DocumentNodeKind::PageBreak);
                    lines.push(ReferenceLinePlacement {
                        start: index,
                        end: index
                            .checked_add(1)
                            .ok_or(FragmentError::ArithmeticOverflow)?,
                        height: basic_boundary_height(package, position.owner(), forced_break)?,
                        forced_break,
                        keep_with_next: false,
                    });
                    index = index
                        .checked_add(1)
                        .ok_or(FragmentError::ArithmeticOverflow)?;
                }
                Some(FlowBoundaryKind::TableRow) => {
                    // Table rows are selected by the dedicated table
                    // paginator. The body-flow placeholder is a one-unit,
                    // paint-free progress record so the ordinary M2 flow can
                    // still close around direct-body tables without flattening
                    // cell subflows into the body cursor.
                    lines.push(ReferenceLinePlacement {
                        start: index,
                        end: index
                            .checked_add(1)
                            .ok_or(FragmentError::ArithmeticOverflow)?,
                        height: PositiveLength::new(
                            Length::from_raw(1).ok_or(FragmentError::ArithmeticOverflow)?,
                        )
                        .ok_or(FragmentError::ArithmeticOverflow)?,
                        forced_break: false,
                        keep_with_next: false,
                    });
                    index = index
                        .checked_add(1)
                        .ok_or(FragmentError::ArithmeticOverflow)?;
                }
                _ => return Err(FragmentError::UnsupportedFlowDomain),
            }
        }
        if allow_footnotes {
            for list in basic_list_blocks(&package.package().document.blocks) {
                let owner = basic_block_node_id(list);
                if !reference_keep_with_next(package, owner)? {
                    continue;
                }
                let path = package
                    .document_nodes()
                    .node_path(owner)
                    .ok_or(FragmentError::InvalidFragmentKey)?;
                let last_line = lines
                    .iter()
                    .rposition(|line| {
                        flow.positions
                            .get(line.start)
                            .is_some_and(|position| position.block_child_path().starts_with(path))
                    })
                    .ok_or(FragmentError::InvalidFragmentKey)?;
                lines[last_line].keep_with_next = true;
            }
        }
        anchors.sort_by(|left, right| {
            (left.flow_ordinal, &left.anchor_id).cmp(&(right.flow_ordinal, &right.anchor_id))
        });
        let footnote_descendants = if allow_footnotes {
            footnote_descendant_owners(&package.package().document, package.document_nodes())
        } else {
            std::collections::BTreeSet::new()
        };
        let expected_body_anchors: std::collections::BTreeMap<_, _> = flow
            .anchors
            .iter()
            .filter(|(_, owner)| !footnote_descendants.contains(owner))
            .map(|(anchor, owner)| (anchor.clone(), *owner))
            .collect();
        if anchors
            .windows(2)
            .any(|pair| pair[0].anchor_id == pair[1].anchor_id)
            || anchors.len() != expected_body_anchors.len()
            || anchors.iter().any(|anchor| {
                expected_body_anchors.get(&anchor.anchor_id) != Some(&anchor.owner_node)
            })
        {
            return Err(FragmentError::InvalidFragmentKey);
        }
        footnotes.sort_by_key(|placement| (placement.flow_ordinal, placement.reference_owner));
        if footnotes
            .windows(2)
            .any(|pair| pair[0].reference_owner == pair[1].reference_owner)
        {
            return Err(FragmentError::InvalidFragmentKey);
        }
        Ok(Self {
            flow,
            anchors,
            footnotes: if allow_footnotes {
                footnotes
            } else {
                Vec::new()
            },
            lines,
            legacy_full_frame: false,
            basic_document: true,
            enforce_keep_with_next: allow_footnotes,
        })
    }

    pub fn ends_with_forced_break(&self) -> bool {
        self.basic_document && matches!(self.lines.last(), Some(line) if line.forced_break)
    }

    /// Reference owners selected by one exact body range. This projects the
    /// same item-bound placements used for PagePlan footnote IDs, so MI3's
    /// page evaluator cannot substitute a package-preorder approximation.
    pub fn footnote_reference_owners_between(
        &self,
        start: &FlowPosition,
        end: &FlowPosition,
    ) -> Result<Vec<NodeId>, FragmentError> {
        if !self.flow.contains_position(start)
            || !self.flow.contains_position(end)
            || start.cmp_within_epoch(end)? == Ordering::Greater
        {
            return Err(FragmentError::InvalidFragmentRange);
        }
        Ok(self
            .footnotes
            .iter()
            .filter(|placement| {
                placement.flow_ordinal >= start.global_flow_ordinal()
                    && placement.flow_ordinal < end.global_flow_ordinal()
            })
            .map(|placement| placement.reference_owner)
            .collect())
    }

    /// Returns the greatest selected-fragment cut before one reference while
    /// preserving every hard body `keep_with_next` boundary. The input must
    /// be the contiguous candidate emitted by this fragmenter; callers cannot
    /// nominate a flow position directly.
    pub fn legal_cut_index_before_reference(
        &self,
        fragments: &[FragmentDraft],
        reference_owner: NodeId,
    ) -> Result<Option<usize>, FragmentError> {
        let Some(mut cut_index) = fragments.iter().position(|fragment| {
            self.footnote_reference_owners_between(fragment.start(), fragment.end())
                .is_ok_and(|owners| owners.contains(&reference_owner))
        }) else {
            return Ok(None);
        };
        if !self.enforce_keep_with_next {
            return Ok(Some(cut_index));
        }
        while cut_index != 0 {
            let previous = &fragments[cut_index - 1];
            let current = &fragments[cut_index];
            if previous.end() != current.start() {
                return Err(FragmentError::InvalidFragmentRange);
            }
            let line = self
                .lines
                .iter()
                .find(|line| {
                    self.flow.positions.get(line.start) == Some(previous.start())
                        && self.flow.positions.get(line.end) == Some(previous.end())
                })
                .ok_or(FragmentError::InvalidFragmentRange)?;
            if !line.keep_with_next {
                break;
            }
            cut_index -= 1;
        }
        Ok(Some(cut_index))
    }

    /// Reissues body-anchor discoveries for an exact selected range. This is
    /// used by the footnote page-local owner after a body-cut candidate has
    /// discarded trailing fragments.
    pub fn anchors_between(
        &self,
        start: &FlowPosition,
        end: &FlowPosition,
    ) -> Result<Vec<DiscoveredAnchor>, FragmentError> {
        if !self.flow.contains_position(start)
            || !self.flow.contains_position(end)
            || start.cmp_within_epoch(end)? == Ordering::Greater
        {
            return Err(FragmentError::InvalidFragmentRange);
        }
        Ok(self
            .anchors
            .iter()
            .filter(|placement| {
                placement.flow_ordinal >= start.global_flow_ordinal()
                    && placement.flow_ordinal < end.global_flow_ordinal()
            })
            .map(|placement| DiscoveredAnchor {
                anchor_id: placement.anchor_id.clone(),
                owner_node: placement.owner_node,
                position_in_frame: Point {
                    x: Length::ZERO,
                    y: Length::ZERO,
                },
            })
            .collect())
    }

    /// Issues the typed cursor for one package-derived position retained by
    /// this fragmenter. Callers cannot supply a cursor location tag.
    pub fn cursor_for_position(
        &self,
        position: &FlowPosition,
    ) -> Result<FlowCursor, FragmentError> {
        if !self.flow.contains_position(position) {
            return Err(FragmentError::UnknownFlowPosition);
        }
        let index = usize::try_from(position.global_flow_ordinal())
            .map_err(|_| FragmentError::UnknownFlowPosition)?;
        if self.flow.positions.get(index) != Some(position) {
            return Err(FragmentError::UnknownFlowPosition);
        }
        self.cursor_at(index)
    }

    fn cursor_at(&self, position_index: usize) -> Result<FlowCursor, FragmentError> {
        let position = self
            .flow
            .positions
            .get(position_index)
            .ok_or(FragmentError::UnknownFlowPosition)?;
        let terminal = self
            .flow
            .positions
            .len()
            .checked_sub(1)
            .ok_or(FragmentError::UnknownFlowPosition)?;
        let location = match self.flow.boundary_kinds.get(position_index) {
            Some(_) if position_index == terminal => CursorPosition::End,
            Some(FlowBoundaryKind::DocumentStart) => CursorPosition::DocumentStart,
            Some(FlowBoundaryKind::ParagraphItem) => {
                CursorPosition::ParagraphItem(position.owner_local_boundary())
            }
            Some(FlowBoundaryKind::ListItem) => {
                CursorPosition::ListItem(position.owner_local_boundary())
            }
            Some(FlowBoundaryKind::BlockItem) => {
                CursorPosition::BlockItem(position.owner_local_boundary())
            }
            Some(FlowBoundaryKind::TableRow) => {
                CursorPosition::TableRow(position.owner_local_boundary())
            }
            Some(FlowBoundaryKind::End) => CursorPosition::End,
            None => return Err(FragmentError::UnknownFlowPosition),
        };
        FlowCursor::at(
            self.flow,
            u64::try_from(position_index).map_err(|_| FragmentError::ArithmeticOverflow)?,
            location,
        )
    }
}

impl Fragmenter for ReferenceFragmenter<'_> {
    fn fragment(
        &self,
        request: &FragmentRequest<'_>,
        budget: &mut dyn FragmentWorkBudget,
    ) -> Result<FragmentResult, FragmentError> {
        request.validate()?;
        if request.flow().epoch() != self.flow.epoch() {
            return Err(FragmentError::InvalidCursorEpoch);
        }
        if request.flow() != self.flow {
            return Err(FragmentError::InvalidFragmentKey);
        }
        let current = usize::try_from(request.cursor().position().global_flow_ordinal())
            .map_err(|_| FragmentError::UnknownFlowPosition)?;
        let terminal = self
            .flow
            .positions
            .len()
            .checked_sub(1)
            .ok_or(FragmentError::UnknownFlowPosition)?;

        match (
            self.flow.boundary_kinds.get(current),
            request.cursor().location(),
        ) {
            (Some(FlowBoundaryKind::DocumentStart), CursorPosition::DocumentStart)
                if current == terminal =>
            {
                return Ok(FragmentResult {
                    fragments: Vec::new(),
                    continuation: Continuation::Exhausted(Box::new(self.cursor_at(terminal)?)),
                    discovered_footnotes: Vec::new(),
                    discovered_anchors: Vec::new(),
                });
            }
            (Some(FlowBoundaryKind::DocumentStart), CursorPosition::DocumentStart) => {
                return Ok(FragmentResult {
                    fragments: Vec::new(),
                    continuation: Continuation::More(Box::new(self.cursor_at(current + 1)?)),
                    discovered_footnotes: Vec::new(),
                    discovered_anchors: Vec::new(),
                });
            }
            (Some(FlowBoundaryKind::ParagraphItem), CursorPosition::ParagraphItem(local))
                if *local == request.cursor().position().owner_local_boundary() => {}
            (Some(FlowBoundaryKind::ListItem), CursorPosition::ListItem(local))
                if self.basic_document
                    && *local == request.cursor().position().owner_local_boundary() => {}
            (Some(FlowBoundaryKind::BlockItem), CursorPosition::BlockItem(local))
                if self.basic_document
                    && *local == request.cursor().position().owner_local_boundary() => {}
            (Some(FlowBoundaryKind::TableRow), CursorPosition::TableRow(local))
                if self.basic_document
                    && *local == request.cursor().position().owner_local_boundary() => {}
            (Some(FlowBoundaryKind::End), CursorPosition::End) => {
                return Err(FragmentError::InvalidCursorLocation);
            }
            (Some(_), _) => return Err(FragmentError::InvalidCursorLocation),
            (None, _) => return Err(FragmentError::UnknownFlowPosition),
        }

        let first_line = self
            .lines
            .iter()
            .position(|line| line.start == current)
            .ok_or(FragmentError::InvalidCursorLocation)?;
        let available = request
            .frame()
            .height()
            .get()
            .raw()
            .checked_sub(request.reserved_footnote_height().get().raw())
            .ok_or(FragmentError::ArithmeticOverflow)?;
        let mut capacity = if self.legacy_full_frame {
            self.lines.len()
        } else {
            let mut occupied = 0i64;
            let mut count = 0usize;
            for line in &self.lines[first_line..] {
                if line.forced_break && line.end == terminal && count != 0 {
                    break;
                }
                let next = occupied
                    .checked_add(line.height.get().raw())
                    .ok_or(FragmentError::ArithmeticOverflow)?;
                if next > available {
                    break;
                }
                occupied = next;
                count = count
                    .checked_add(1)
                    .ok_or(FragmentError::ArithmeticOverflow)?;
                if line.forced_break {
                    break;
                }
            }
            count
        };
        if self.enforce_keep_with_next {
            while capacity != 0
                && first_line + capacity < self.lines.len()
                && self.lines[first_line + capacity - 1].keep_with_next
            {
                capacity -= 1;
            }
        }
        if capacity == 0 {
            return Err(FragmentError::Unplaceable);
        }
        let fragment_count = (self.lines.len() - first_line).min(capacity);
        budget.consume_fragments(
            u64::try_from(fragment_count).map_err(|_| FragmentError::ArithmeticOverflow)?,
        )?;
        let mut fragments = Vec::with_capacity(fragment_count);
        let mut y_delta = 0i64;
        for line in &self.lines[first_line..first_line + fragment_count] {
            let y = request
                .frame()
                .y()
                .raw()
                .checked_add(y_delta)
                .and_then(Length::from_raw)
                .ok_or(FragmentError::ArithmeticOverflow)?;
            fragments.push(FragmentDraft::new(
                self.flow.positions[line.start].clone(),
                self.flow.positions[line.end].clone(),
                if self.legacy_full_frame {
                    request.frame()
                } else {
                    Rect::new(request.frame().x(), y, request.frame().width(), line.height)
                },
                0,
            )?);
            y_delta = y_delta
                .checked_add(line.height.get().raw())
                .ok_or(FragmentError::ArithmeticOverflow)?;
        }
        let current_ordinal =
            u64::try_from(current).map_err(|_| FragmentError::ArithmeticOverflow)?;
        let continuation_index = self.lines[first_line + fragment_count - 1].end;
        let continuation_ordinal =
            u64::try_from(continuation_index).map_err(|_| FragmentError::ArithmeticOverflow)?;
        let discovered_anchors = self
            .anchors
            .iter()
            .filter(|anchor| {
                anchor.flow_ordinal >= current_ordinal && anchor.flow_ordinal < continuation_ordinal
            })
            .map(|anchor| DiscoveredAnchor {
                anchor_id: anchor.anchor_id.clone(),
                owner_node: anchor.owner_node,
                position_in_frame: Point {
                    x: Length::ZERO,
                    y: Length::ZERO,
                },
            })
            .collect();
        let mut discovered_footnotes: Vec<_> = self
            .footnotes
            .iter()
            .filter(|placement| {
                placement.flow_ordinal >= current_ordinal
                    && placement.flow_ordinal < continuation_ordinal
            })
            .map(|placement| placement.footnote_id.clone())
            .collect();
        discovered_footnotes.sort();
        discovered_footnotes.dedup();
        Ok(FragmentResult {
            fragments,
            continuation: if continuation_index == terminal {
                Continuation::Exhausted(Box::new(self.cursor_at(terminal)?))
            } else {
                Continuation::More(Box::new(self.cursor_at(continuation_index)?))
            },
            discovered_footnotes,
            discovered_anchors,
        })
    }
}

fn collect_reference_footnotes(
    inlines: &[Inline],
    paragraph_owner: NodeId,
    paragraph_start: usize,
    registry: &ValidatedParagraphItemRegistry,
    output: &mut Vec<ReferenceFootnotePlacement>,
) -> Result<(), FragmentError> {
    for inline in inlines {
        match inline {
            Inline::FootnoteReference {
                node_id,
                footnote_id,
                ..
            } => {
                let local = registry
                    .generated_site_first_item_index(
                        paragraph_owner,
                        *node_id,
                        typaxis_core::GenerationKind::FootnoteMarker,
                    )
                    .ok_or(FragmentError::InvalidFragmentKey)?;
                let flow_ordinal = paragraph_start
                    .checked_add(
                        usize::try_from(local).map_err(|_| FragmentError::ArithmeticOverflow)?,
                    )
                    .and_then(|value| u64::try_from(value).ok())
                    .ok_or(FragmentError::ArithmeticOverflow)?;
                output.push(ReferenceFootnotePlacement {
                    flow_ordinal,
                    reference_owner: *node_id,
                    footnote_id: footnote_id.clone(),
                });
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => collect_reference_footnotes(
                children,
                paragraph_owner,
                paragraph_start,
                registry,
                output,
            )?,
            Inline::Text { .. }
            | Inline::Anchor { .. }
            | Inline::Reference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {}
        }
    }
    Ok(())
}

fn basic_paragraph_blocks(blocks: &[Block]) -> Vec<&Block> {
    let mut paragraphs = Vec::new();
    let mut pending: Vec<&Block> = blocks.iter().rev().collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Paragraph { .. } | Block::Heading { .. } => paragraphs.push(block),
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Figure { caption, .. } => pending.extend(caption.iter().rev()),
            Block::Table { head, body, .. } => pending.extend(
                body.iter()
                    .rev()
                    .chain(head.iter().rev())
                    .flat_map(|row| row.cells.iter().rev())
                    .flat_map(|cell| cell.blocks.iter().rev()),
            ),
            Block::PageBreak { .. } => {}
        }
    }
    paragraphs
}

fn basic_list_blocks(blocks: &[Block]) -> Vec<&Block> {
    let mut lists = Vec::new();
    let mut pending: Vec<&Block> = blocks.iter().rev().collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::List { items, .. } => {
                lists.push(block);
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Figure { caption, .. } => pending.extend(caption.iter().rev()),
            Block::Table { head, body, .. } => pending.extend(
                body.iter()
                    .rev()
                    .chain(head.iter().rev())
                    .flat_map(|row| row.cells.iter().rev())
                    .flat_map(|cell| cell.blocks.iter().rev()),
            ),
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    lists
}

const fn basic_block_node_id(block: &Block) -> NodeId {
    match block {
        Block::Paragraph { node_id, .. }
        | Block::Heading { node_id, .. }
        | Block::List { node_id, .. }
        | Block::Table { node_id, .. }
        | Block::Figure { node_id, .. }
        | Block::PageBreak { node_id, .. } => *node_id,
    }
}

fn basic_boundary_height(
    package: &ValidatedParsedPackage,
    owner: NodeId,
    forced_break: bool,
) -> Result<PositiveLength, FragmentError> {
    let one = || {
        Length::from_raw(1)
            .and_then(PositiveLength::new)
            .ok_or(FragmentError::ArithmeticOverflow)
    };
    if forced_break {
        return one();
    }
    let computed = match package.cascade_style(owner) {
        Ok(computed) => computed,
        Err(_) => return one(),
    };
    if package.document_nodes().node_kind(owner) == Some(DocumentNodeKind::Figure) {
        return match computed
            .computed()
            .basic_figure_width()
            .map_err(|_| FragmentError::InvalidFragmentKey)?
        {
            MachineFigureWidth::Length(value) => Ok(value),
            MachineFigureWidth::Auto => one(),
        };
    }
    match computed.computed().properties().get("line_height") {
        Some(StyleValue::Length(value)) => {
            PositiveLength::new(*value).ok_or(FragmentError::InvalidFragmentKey)
        }
        None => one(),
        Some(_) => Err(FragmentError::InvalidFragmentKey),
    }
}

fn collect_reference_anchors(
    inlines: &[Inline],
    package: &ValidatedParsedPackage,
    flow: &FlowTree,
    flow_ordinal: u64,
    output: &mut Vec<ReferenceAnchorPlacement>,
) -> Result<(), FragmentError> {
    for inline in inlines {
        match inline {
            Inline::Anchor {
                node_id, anchor_id, ..
            } => {
                if package.document_nodes().anchor_owner(anchor_id) != Some(*node_id)
                    || flow.anchor_owner(anchor_id) != Some(*node_id)
                {
                    return Err(FragmentError::InvalidFragmentKey);
                }
                output.push(ReferenceAnchorPlacement {
                    flow_ordinal,
                    anchor_id: anchor_id.clone(),
                    owner_node: *node_id,
                });
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => {
                collect_reference_anchors(children, package, flow, flow_ordinal, output)?;
            }
            Inline::Text { .. }
            | Inline::Reference { .. }
            | Inline::FootnoteReference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
    use typaxis_core::{
        sha256, ConfigResourceRoot, DocumentPackageContractId, EffectiveConfig,
        EffectiveDataVersions, HostAdmissionContext, HostPath, ImageResourceId,
        PdfStreamCompression, PortablePath, ResourceLimits, SourceId, ValidatedResourceLimits,
        DEFAULT_ALLOWED_URI_SCHEMES, JSON_SAFE_INTEGER_MAX, REGISTERED_JAPANESE_LINE_BREAK_VERSION,
        REGISTERED_UNICODE_VERSION,
    };
    use typaxis_resource_admission::{
        AdmittedFontInstanceTable, AdmittedResourceResolver, HostResourceAdmissionSession,
    };
    use typaxis_style::StyleValidationError;
    use typaxis_syntax::{
        machine_profile_boundary::{wire, HostMachineInputSession, MachineInputHostOptions},
        DocumentPackageParser, MachineParseOutcome, PackageGeneratedTextError,
        PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
        StagingStylePackageParser, ValidatedMachinePackage, ValidatedParsedPackage,
    };
    use typaxis_text::{GeneratedTextStore, TextStore};
    fn parsed_reference_package(seed: u8, text: &str) -> ValidatedParsedPackage {
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new(format!("input-{seed}.tsf")).unwrap(),
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
    fn validated_package(seed: u8) -> ValidatedParsedPackage {
        parsed_reference_package(seed, "")
    }
    fn paragraph_package(seed: u8) -> ValidatedParsedPackage {
        parsed_reference_package(seed, "paragraph\nparagraph")
    }

    fn staging_footnote_wire(
        reference_ids: &[&str],
        definitions: &[(&str, bool)],
        footnote_y: i64,
        footnote_height: i64,
    ) -> wire::WireDocumentPackage {
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let mut next_node_id = 2u32;
        let body_children = reference_ids
            .iter()
            .map(|footnote_id| {
                let node_id = next_node_id;
                next_node_id += 1;
                wire::WireInline::FootnoteReference {
                    node_id,
                    span,
                    footnote_id: (*footnote_id).to_owned(),
                }
            })
            .collect();
        let mut footnotes = Vec::new();
        for (footnote_id, productive) in definitions {
            let definition_node = next_node_id;
            let paragraph_node = next_node_id + 1;
            let inline_node = next_node_id + 2;
            next_node_id += 3;
            let child = if *productive {
                wire::WireInline::Reference {
                    node_id: inline_node,
                    span,
                    target: "target".to_owned(),
                    format: wire::WireReferenceFormat::Page,
                }
            } else {
                wire::WireInline::SoftBreak {
                    node_id: inline_node,
                    span,
                }
            };
            footnotes.push(wire::WireFootnote {
                footnote_id: (*footnote_id).to_owned(),
                node_id: definition_node,
                span,
                blocks: vec![wire::WireBlock::Paragraph {
                    node_id: paragraph_node,
                    span,
                    classes: Vec::new(),
                    children: vec![child],
                }],
            });
        }
        wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_2,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: "footnote-input.tsf".to_owned(),
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
                    children: body_children,
                }],
                footnotes,
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
                        y: footnote_y,
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
        }
    }

    fn parse_staging_footnote_wire(
        package: wire::WireDocumentPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<ValidatedStagingStylePackage, ()> {
        let bytes = wire::StagingStyleDocumentPackageEncoder::default()
            .to_jcs_vec(&package)
            .unwrap();
        let decoded = wire::StagingStyleDocumentPackageDecoder::new()
            .decode(&bytes, &wire::DocumentPackageDecodePolicy::new(limits))
            .unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        StagingStylePackageParser::new()
            .parse(
                decoded,
                String::new(),
                &PackageValidationPolicy::new(limits, &schemes).unwrap(),
            )
            .map_err(|_| ())
    }

    fn staging_footnote_epoch(
        package: &ValidatedParsedPackage,
        limits: &ValidatedResourceLimits,
    ) -> LayoutEpoch {
        let generated = package.materialize_initial_generated_text(limits).unwrap();
        let generated = package.bind_generated_text(&generated, limits).unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, limits)
            .unwrap()
            .finish()
            .unwrap();
        LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap()
    }

    fn staging_body_registry_fingerprint() -> FlowRegistryFingerprint {
        flow_registry_fingerprint_from_jcs(
            "{\"algorithm\":\"typaxis.basic-flow-registry/1\",\"fixture\":true}",
        )
    }

    fn footnote_extent(raw: i64) -> PositiveLength {
        PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap()
    }

    #[test]
    fn footnote_registry_is_canonical_across_worker_registration_order() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let package = parse_staging_footnote_wire(
            staging_footnote_wire(
                &["z", "z", "a"],
                &[("a", true), ("z", true)],
                100_000,
                100_000,
            ),
            &limits,
        )
        .unwrap();
        let epoch = staging_footnote_epoch(package.package(), &limits);
        let preflight = preflight_staging_footnote_profile(
            package.package(),
            epoch,
            staging_body_registry_fingerprint(),
            &limits,
        )
        .unwrap();
        assert_eq!(
            preflight
                .definitions()
                .iter()
                .map(|definition| (
                    definition.footnote_id().as_str(),
                    definition.catalog_ordinal()
                ))
                .collect::<Vec<_>>(),
            vec![("a", 1), ("z", 2)]
        );
        assert_eq!(
            preflight
                .references()
                .iter()
                .map(|reference| (
                    reference.footnote_id().as_str(),
                    reference.logical_ordinal()
                ))
                .collect::<Vec<_>>(),
            vec![("z", 0), ("z", 1), ("a", 2)]
        );

        let build = |reverse: bool| {
            let mut builder = StagingFootnoteFlowRegistryBuilder::new(&preflight, &limits);
            let mut ids: Vec<_> = builder.expected_definition_ids().cloned().collect();
            if reverse {
                ids.reverse();
            }
            for id in ids {
                let measured = if id.as_str() == "a" {
                    builder
                        .issue_definition(&id, vec![footnote_extent(10_000)])
                        .unwrap()
                } else {
                    builder
                        .issue_definition_with_line_counts(
                            &id,
                            vec![footnote_extent(30_000)],
                            vec![NonZeroU32::new(2).unwrap()],
                        )
                        .unwrap()
                };
                builder.register(measured).unwrap();
            }
            builder.finish().unwrap()
        };
        let forward = build(false);
        let reverse = build(true);
        assert_eq!(
            forward.receipt().fingerprint(),
            reverse.receipt().fingerprint()
        );
        assert_eq!(forward.canonical_jcs(), reverse.canonical_jcs());
        assert_eq!(forward.flows().len(), 2);
        assert_eq!(forward.flows()[0].binding().footnote_id().as_str(), "a");
        assert_eq!(
            forward.flows()[0].binding().flow_id(),
            FootnoteFlowId::new(0)
        );
        assert_eq!(forward.flows()[1].binding().footnote_id().as_str(), "z");
        assert_eq!(
            forward.flows()[1].binding().flow_id(),
            FootnoteFlowId::new(1)
        );
        assert_eq!(forward.flows()[1].binding().terminal().fragment_count(), 1);
        assert_eq!(forward.flows()[1].fragment_line_counts()[0].get(), 2);
        assert!(forward
            .canonical_jcs()
            .contains("\"fragment_line_counts\":[2]"));
    }

    #[test]
    fn footnote_registry_preflight_rejects_unreferenced_empty_and_missing_definitions() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let unreferenced = parse_staging_footnote_wire(
            staging_footnote_wire(&["z"], &[("a", true), ("z", true)], 100_000, 100_000),
            &limits,
        )
        .unwrap();
        let error = preflight_staging_footnote_profile(
            unreferenced.package(),
            staging_footnote_epoch(unreferenced.package(), &limits),
            staging_body_registry_fingerprint(),
            &limits,
        )
        .unwrap_err();
        assert_eq!(
            error,
            StagingFootnoteRegistryError::UnreferencedDefinition(FootnoteId::new("a").unwrap())
        );

        let empty = parse_staging_footnote_wire(
            staging_footnote_wire(&["a"], &[("a", false)], 100_000, 100_000),
            &limits,
        )
        .unwrap();
        assert_eq!(
            preflight_staging_footnote_profile(
                empty.package(),
                staging_footnote_epoch(empty.package(), &limits),
                staging_body_registry_fingerprint(),
                &limits,
            )
            .unwrap_err(),
            StagingFootnoteRegistryError::EmptyDefinition(FootnoteId::new("a").unwrap())
        );

        let missing = staging_footnote_wire(&["a"], &[], 100_000, 100_000);
        assert!(parse_staging_footnote_wire(missing, &limits).is_err());

        let package = parse_staging_footnote_wire(
            staging_footnote_wire(&["a", "z"], &[("a", true), ("z", true)], 100_000, 100_000),
            &limits,
        )
        .unwrap();
        let preflight = preflight_staging_footnote_profile(
            package.package(),
            staging_footnote_epoch(package.package(), &limits),
            staging_body_registry_fingerprint(),
            &limits,
        )
        .unwrap();
        let mut builder = StagingFootnoteFlowRegistryBuilder::new(&preflight, &limits);
        let a = FootnoteId::new("a").unwrap();
        let measured = builder
            .issue_definition(&a, vec![footnote_extent(10_000)])
            .unwrap();
        builder.register(measured).unwrap();
        assert_eq!(
            builder.finish().unwrap_err(),
            StagingFootnoteRegistryError::MissingDefinition(FootnoteId::new("z").unwrap())
        );
    }

    #[test]
    fn footnote_registry_checks_master_geometry_and_fragment_limit_before_allocation() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let invalid_master = parse_staging_footnote_wire(
            staging_footnote_wire(&["a"], &[("a", true)], 90_000, 100_000),
            &limits,
        )
        .unwrap();
        assert_eq!(
            preflight_staging_footnote_profile(
                invalid_master.package(),
                staging_footnote_epoch(invalid_master.package(), &limits),
                staging_body_registry_fingerprint(),
                &limits,
            )
            .unwrap_err(),
            StagingFootnoteRegistryError::InvalidFootnoteMaster
        );

        let package = parse_staging_footnote_wire(
            staging_footnote_wire(&["a"], &[("a", true)], 100_000, 100_000),
            &limits,
        )
        .unwrap();
        let preflight = preflight_staging_footnote_profile(
            package.package(),
            staging_footnote_epoch(package.package(), &limits),
            staging_body_registry_fingerprint(),
            &limits,
        )
        .unwrap();
        let raw_limits = ResourceLimits {
            max_fragments: 2,
            ..ResourceLimits::default()
        };
        let exact_limits = ValidatedResourceLimits::new(raw_limits).unwrap();
        let builder = StagingFootnoteFlowRegistryBuilder::new(&preflight, &exact_limits);
        let a = FootnoteId::new("a").unwrap();
        assert!(builder
            .issue_definition(&a, vec![footnote_extent(1), footnote_extent(1)])
            .is_ok());
        assert_eq!(
            builder
                .issue_definition(
                    &a,
                    vec![footnote_extent(1), footnote_extent(1), footnote_extent(1)],
                )
                .unwrap_err(),
            StagingFootnoteRegistryError::FragmentLimit
        );
    }

    fn staging_machine_list_package() -> typaxis_syntax::ValidatedStagingStylePackage {
        staging_machine_list_package_with_keep(false)
    }

    fn staging_machine_list_package_with_keep(
        keep_with_next: bool,
    ) -> typaxis_syntax::ValidatedStagingStylePackage {
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
        let mut declarations = vec![
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
        ];
        if keep_with_next {
            declarations.push(wire::WireDeclaration {
                name: wire::WireDeclarationName::KeepWithNext,
                value: wire::WireStyleValue::Boolean { value: true },
                important: false,
            });
        }
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
                                    classes: vec!["nested".to_owned()],
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
                rules: vec![
                    wire::WireStyleRule {
                        style_id: "list-base".to_owned(),
                        extends: None,
                        selector: "list".to_owned(),
                        source_order: 0,
                        declarations,
                    },
                    wire::WireStyleRule {
                        style_id: "list-nested".to_owned(),
                        extends: None,
                        selector: "list.nested".to_owned(),
                        source_order: 1,
                        declarations: vec![wire::WireDeclaration {
                            name: wire::WireDeclarationName::StartIndent,
                            value: wire::WireStyleValue::Length { value: 7 },
                            important: false,
                        }],
                    },
                ],
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
        StagingStylePackageParser::new()
            .parse(
                decoded,
                String::new(),
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            )
            .unwrap()
    }

    fn list_length(raw: i64) -> PositiveLength {
        PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap()
    }

    fn staging_machine_list_layout(
        items: Vec<StagingListItemPaintInput>,
    ) -> Result<StagingMachineListLayoutReceipt, StagingMachineListLayoutError> {
        staging_machine_list_layout_with_direction(items, BidiLevel::LTR)
    }

    fn staging_machine_list_layout_with_direction(
        items: Vec<StagingListItemPaintInput>,
        base_direction: BidiLevel,
    ) -> Result<StagingMachineListLayoutReceipt, StagingMachineListLayoutError> {
        let package = staging_machine_list_package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
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
        let package_epoch =
            LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let ir = ProductionFlowIr::for_empty_paragraph_content(
            package.package(),
            package_epoch,
            &limits,
        )
        .unwrap();
        layout_staging_machine_lists(
            &package,
            &preflight,
            generated,
            &ir,
            StagingMachineListLayoutInput::new(list_length(100), base_direction, items),
        )
    }

    fn painted_list_item(
        owner: u32,
        marker_width: i64,
        first_line_width: i64,
        first_line_height: i64,
        painted_height: i64,
    ) -> StagingListItemPaintInput {
        StagingListItemPaintInput::painted(
            NodeId::new(owner),
            list_length(marker_width),
            list_length(first_line_width),
            list_length(first_line_height),
            list_length(painted_height),
        )
    }

    #[test]
    fn machine_list_layout_end_aligns_markers_and_keeps_nested_indents_in_child_flow() {
        let layout = staging_machine_list_layout(vec![
            painted_list_item(2, 4, 20, 8, 14),
            painted_list_item(5, 6, 18, 8, 12),
            painted_list_item(7, 8, 24, 8, 16),
        ])
        .unwrap();
        assert_eq!(layout.lists().len(), 2);
        let outer = layout
            .lists()
            .iter()
            .find(|list| list.list_owner() == NodeId::new(1))
            .unwrap();
        let nested = layout
            .lists()
            .iter()
            .find(|list| list.list_owner() == NodeId::new(4))
            .unwrap();
        assert_eq!(outer.list_flow_id(), FlowId::DOCUMENT_BODY);
        assert_eq!(outer.marker_column_width().get().raw(), 8);
        assert_eq!(outer.marker_gap().get().raw(), 10);
        assert_eq!(outer.item_frame_inline_size().get().raw(), 74);
        assert_eq!(nested.list_flow_id(), FlowId::new(1));
        assert_eq!(nested.start_indent().get().raw(), 7);
        assert_eq!(nested.marker_column_width().get().raw(), 6);
        assert_eq!(nested.item_frame_inline_size().get().raw(), 48);

        let first = layout
            .items()
            .iter()
            .find(|item| item.item_owner() == NodeId::new(2))
            .unwrap();
        let second_outer = layout
            .items()
            .iter()
            .find(|item| item.item_owner() == NodeId::new(7))
            .unwrap();
        assert_eq!(first.marker_utf8(), "9.");
        assert_eq!(second_outer.marker_utf8(), "10.");
        assert_eq!(first.marker_physical_left().get().raw(), 9);
        assert_eq!(second_outer.marker_physical_left().get().raw(), 5);
        assert_eq!(first.content_physical_left().get().raw(), 23);
        assert_eq!(first.item_flow_id(), FlowId::new(1));
        assert_eq!(
            layout
                .items()
                .iter()
                .find(|item| item.item_owner() == NodeId::new(5))
                .unwrap()
                .item_flow_id(),
            FlowId::new(2)
        );
        assert_eq!(second_outer.item_flow_id(), FlowId::new(3));

        let rtl = staging_machine_list_layout_with_direction(
            vec![
                painted_list_item(2, 4, 20, 8, 14),
                painted_list_item(5, 6, 18, 8, 12),
                painted_list_item(7, 8, 24, 8, 16),
            ],
            BidiLevel::RTL,
        )
        .unwrap();
        let rtl_outer: Vec<_> = rtl
            .items()
            .iter()
            .filter(|item| item.list_owner() == NodeId::new(1))
            .collect();
        assert_eq!(rtl_outer[0].marker_physical_left().get().raw(), 87);
        assert_eq!(rtl_outer[1].marker_physical_left().get().raw(), 87);
        assert_eq!(rtl_outer[0].content_physical_left().get().raw(), 3);
    }

    #[test]
    fn machine_list_layout_rejects_empty_or_incomplete_item_paint_receipts() {
        let empty = staging_machine_list_layout(vec![
            StagingListItemPaintInput::empty(NodeId::new(2), list_length(4)),
            painted_list_item(5, 6, 18, 8, 12),
            painted_list_item(7, 8, 24, 8, 16),
        ]);
        assert_eq!(
            empty.unwrap_err(),
            StagingMachineListLayoutError::EmptyPaintedItem(NodeId::new(2))
        );

        let missing = staging_machine_list_layout(vec![
            painted_list_item(2, 4, 20, 8, 14),
            painted_list_item(7, 8, 24, 8, 16),
        ]);
        assert_eq!(
            missing.unwrap_err(),
            StagingMachineListLayoutError::MissingMeasurement(NodeId::new(5))
        );
    }

    fn staging_table_grid_package(seed: u8) -> ValidatedStagingStylePackage {
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let cell = |node_id, colspan, rowspan| wire::WireTableCell {
            node_id,
            span,
            colspan,
            rowspan,
            blocks: Vec::new(),
        };
        let table = wire::WireBlock::Table {
            node_id: 1,
            span,
            classes: vec!["matrix".to_owned()],
            columns: vec![
                wire::WireTableColumn::Fraction { weight: 1 },
                wire::WireTableColumn::Fraction { weight: 1 },
                wire::WireTableColumn::Fraction { weight: 1 },
                wire::WireTableColumn::Fraction { weight: 1 },
            ],
            head: Vec::new(),
            body: vec![
                wire::WireTableRow {
                    node_id: 2,
                    span,
                    cells: vec![cell(3, 2, 1), cell(4, 1, 2), cell(5, 1, 1)],
                },
                wire::WireTableRow {
                    node_id: 6,
                    span,
                    cells: vec![cell(7, 2, 1), cell(8, 1, 1)],
                },
            ],
        };
        let declaration = |name, value| wire::WireDeclaration {
            name,
            value,
            important: false,
        };
        let package = wire::WireDocumentPackage {
            contract: DocumentPackageContractId::V1_2,
            coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
            sources: vec![wire::WireSource {
                source_id: 0,
                uri: format!("table-{seed}.tsf"),
                utf8_byte_length: 0,
                sha256: sha256(&[]),
            }],
            text_buffers: Vec::new(),
            document: wire::WireDocument {
                node_id: 0,
                blocks: vec![table],
                footnotes: Vec::new(),
            },
            style_sheet: wire::WireStyleSheet {
                rules: vec![wire::WireStyleRule {
                    style_id: "table-matrix".to_owned(),
                    extends: None,
                    selector: "table.matrix".to_owned(),
                    source_order: 0,
                    declarations: vec![
                        declaration(
                            wire::WireDeclarationName::StartIndent,
                            wire::WireStyleValue::Length { value: 1 },
                        ),
                        declaration(
                            wire::WireDeclarationName::EndIndent,
                            wire::WireStyleValue::Length { value: 1 },
                        ),
                        declaration(
                            wire::WireDeclarationName::SpaceBefore,
                            wire::WireStyleValue::Length { value: 2 },
                        ),
                        declaration(
                            wire::WireDeclarationName::SpaceAfter,
                            wire::WireStyleValue::Length { value: 3 },
                        ),
                        declaration(
                            wire::WireDeclarationName::KeepWithNext,
                            wire::WireStyleValue::Boolean { value: true },
                        ),
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
        StagingStylePackageParser::new()
            .parse(
                decoded,
                String::new(),
                &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
            )
            .unwrap()
    }

    fn table_grid_ir(
        package: &ValidatedParsedPackage,
        package_epoch: LayoutEpoch,
        limits: &ValidatedResourceLimits,
        reverse_registration: bool,
    ) -> Result<ProductionFlowIr, FlowRegistryError> {
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(package, package_epoch).unwrap();
        let mut builder =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, limits)?;
        let mut owners: Vec<_> = builder.expected_content_owners().collect();
        if reverse_registration {
            owners.reverse();
        }
        for owner in owners {
            let content = builder.issue_content(owner)?;
            builder.register_content(content)?;
        }
        builder.finish()
    }

    #[test]
    fn table_grid_columns_cells_and_flow_registration_are_canonical() {
        let package = staging_table_grid_package(1);
        let package_epoch = epoch(package.package());
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let reversed = table_grid_ir(package.package(), package_epoch, &limits, true).unwrap();
        let canonical = table_grid_ir(package.package(), package_epoch, &limits, false).unwrap();
        assert_eq!(
            reversed.registry().receipt().fingerprint(),
            canonical.registry().receipt().fingerprint()
        );
        assert_eq!(reversed.registry().receipt().flow_count(), 6);
        assert_eq!(
            reversed
                .registry()
                .flows()
                .iter()
                .map(|flow| (
                    flow.flow_id().get(),
                    flow.owner_node_id().get(),
                    flow.owner_kind(),
                    flow.parent_flow_id().map(FlowId::get),
                ))
                .collect::<Vec<_>>(),
            vec![
                (0, 0, FlowOwnerKind::DocumentBody, None),
                (1, 3, FlowOwnerKind::TableCell, Some(0)),
                (2, 4, FlowOwnerKind::TableCell, Some(0)),
                (3, 5, FlowOwnerKind::TableCell, Some(0)),
                (4, 7, FlowOwnerKind::TableCell, Some(0)),
                (5, 8, FlowOwnerKind::TableCell, Some(0)),
            ]
        );
        assert_eq!(
            reversed.flow(FlowId::DOCUMENT_BODY).unwrap().positions()[0].child_flow_ids(),
            [FlowId::new(1), FlowId::new(2), FlowId::new(3)]
        );
        assert_eq!(
            reversed.flow(FlowId::DOCUMENT_BODY).unwrap().positions()[1].child_flow_ids(),
            [FlowId::new(4), FlowId::new(5)]
        );

        let style = package.compute_table_style(NodeId::new(1)).unwrap();
        let reversed_layout = layout_table_grid(
            &package,
            NodeId::new(1),
            &style,
            &reversed,
            list_length(12),
            &limits,
        )
        .unwrap();
        let canonical_layout = layout_table_grid(
            &package,
            NodeId::new(1),
            &style,
            &canonical,
            list_length(12),
            &limits,
        )
        .unwrap();
        assert_eq!(
            reversed_layout.fingerprint(),
            canonical_layout.fingerprint()
        );
        assert_eq!(reversed_layout.available_inline_size().get().raw(), 10);
        assert_eq!(
            reversed_layout
                .columns()
                .iter()
                .map(|column| column.final_width().get().raw())
                .collect::<Vec<_>>(),
            [2, 2, 2, 4]
        );
        assert_eq!(reversed_layout.rounding_residual().raw(), 2);
        assert_eq!(reversed_layout.residual_recipient(), Some(3));
        assert_eq!(
            reversed_layout
                .cells()
                .iter()
                .map(|cell| (
                    cell.cell_owner().get(),
                    cell.flow_id().get(),
                    cell.column_ordinal(),
                    cell.colspan().get(),
                    cell.rowspan().get(),
                    cell.frame_inline_start().get().raw(),
                    cell.frame_inline_size().get().raw(),
                ))
                .collect::<Vec<_>>(),
            vec![
                (3, 1, 0, 2, 1, 0, 4),
                (4, 2, 2, 1, 2, 4, 2),
                (5, 3, 3, 1, 1, 6, 4),
                (7, 4, 0, 2, 1, 0, 4),
                (8, 5, 3, 1, 1, 6, 4),
            ]
        );
        assert!(reversed_layout
            .cells()
            .iter()
            .all(|cell| cell.padding_start() == NonNegativeLength::ZERO
                && cell.vertical_alignment() == TableVerticalAlignment::BlockStart));
        assert_eq!(reversed_layout.space_before().get().raw(), 2);
        assert_eq!(reversed_layout.space_after().get().raw(), 3);
        assert!(reversed_layout.keep_with_next());
    }

    #[test]
    fn table_rowspan_bands_assign_the_complete_deficit_to_the_last_covered_row() {
        let package = staging_table_grid_package(11);
        let package_epoch = epoch(package.package());
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let ir = table_grid_ir(package.package(), package_epoch, &limits, false).unwrap();
        let style = package.compute_table_style(NodeId::new(1)).unwrap();
        let grid = layout_table_grid(
            &package,
            NodeId::new(1),
            &style,
            &ir,
            list_length(12),
            &limits,
        )
        .unwrap();
        let fragments = [
            vec![list_length(4)],
            vec![list_length(4), list_length(6)],
            vec![list_length(7)],
            vec![list_length(3)],
            vec![list_length(8)],
        ];
        let inputs = grid
            .cells()
            .iter()
            .zip(fragments)
            .map(|(cell, fragments)| {
                TableCellLayoutInput::new(cell.cell_owner(), cell.flow_id(), fragments)
            })
            .collect();
        let bands = layout_table_row_bands(&grid, inputs, &limits).unwrap();
        assert_eq!(
            bands
                .rows()
                .iter()
                .map(|row| (
                    row.section(),
                    row.row_ordinal(),
                    row.block_size().get().raw()
                ))
                .collect::<Vec<_>>(),
            vec![(TableSection::Body, 0, 7), (TableSection::Body, 1, 8)]
        );
        let spanning = bands.cell(NodeId::new(4)).unwrap();
        assert_eq!(
            spanning
                .fragment_endpoints()
                .iter()
                .map(|endpoint| endpoint.get().raw())
                .collect::<Vec<_>>(),
            vec![4, 10]
        );
        assert_eq!(spanning.natural_block_size().get().raw(), 10);
        assert_ne!(bands.fingerprint(), [0; 32]);
    }

    #[test]
    fn table_rowspan_measurements_reject_missing_and_wrong_cell_flows() {
        let package = staging_table_grid_package(12);
        let package_epoch = epoch(package.package());
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let ir = table_grid_ir(package.package(), package_epoch, &limits, false).unwrap();
        let style = package.compute_table_style(NodeId::new(1)).unwrap();
        let grid = layout_table_grid(
            &package,
            NodeId::new(1),
            &style,
            &ir,
            list_length(12),
            &limits,
        )
        .unwrap();
        let missing = grid.cells()[..grid.cells().len() - 1]
            .iter()
            .map(|cell| TableCellLayoutInput::empty(cell.cell_owner(), cell.flow_id()))
            .collect();
        assert_eq!(
            layout_table_row_bands(&grid, missing, &limits).unwrap_err(),
            TableRowBandLayoutError::MissingCellMeasurement(NodeId::new(8))
        );
        let wrong = grid
            .cells()
            .iter()
            .map(|cell| {
                TableCellLayoutInput::empty(
                    cell.cell_owner(),
                    if cell.cell_owner() == NodeId::new(3) {
                        FlowId::new(99)
                    } else {
                        cell.flow_id()
                    },
                )
            })
            .collect();
        assert_eq!(
            layout_table_row_bands(&grid, wrong, &limits).unwrap_err(),
            TableRowBandLayoutError::WrongCellFlow(NodeId::new(3))
        );

        let measured = || {
            grid.cells()
                .iter()
                .map(|cell| {
                    TableCellLayoutInput::new(
                        cell.cell_owner(),
                        cell.flow_id(),
                        vec![list_length(1)],
                    )
                })
                .collect()
        };
        let exact = ValidatedResourceLimits::new(ResourceLimits {
            max_fragments: 5,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(
            layout_table_row_bands(&grid, measured(), &exact)
                .unwrap()
                .contained_fragment_count(),
            5
        );
        let max_plus_one = ValidatedResourceLimits::new(ResourceLimits {
            max_fragments: 4,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(
            layout_table_row_bands(&grid, measured(), &max_plus_one).unwrap_err(),
            TableRowBandLayoutError::FragmentLimit
        );
    }

    #[test]
    fn table_grid_rejects_limit_plus_one_malformed_grids_and_wrong_receipts() {
        let package = staging_table_grid_package(2);
        let package_epoch = epoch(package.package());
        // Nine semantic nodes plus four NodeId-less column records.
        let exact = ValidatedResourceLimits::new(ResourceLimits {
            max_ast_nodes: 13,
            ..ResourceLimits::default()
        })
        .unwrap();
        table_grid_ir(package.package(), package_epoch, &exact, false).unwrap();
        let plus_one = ValidatedResourceLimits::new(ResourceLimits {
            max_ast_nodes: 12,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(
            table_grid_ir(package.package(), package_epoch, &plus_one, false).unwrap_err(),
            FlowRegistryError::AstNodeLimit
        );

        let Block::Table { columns, body, .. } = &package.package().package().document.blocks[0]
        else {
            unreachable!()
        };
        let check = |section_rows: &[TableRow]| {
            let mut rows = Vec::new();
            let mut cells = Vec::new();
            validate_table_section_shape(
                package.package(),
                NodeId::new(1),
                TableSection::Body,
                section_rows,
                columns.len(),
                &mut rows,
                &mut cells,
            )
        };

        let mut overlap = body.clone();
        overlap[1].cells[0].colspan = NonZeroU16::new(3).unwrap();
        assert_eq!(
            check(&overlap),
            Err(TableGridLayoutError::GridOverlap(NodeId::new(7)))
        );
        let mut hole = body.clone();
        hole[1].cells[0].colspan = NonZeroU16::new(1).unwrap();
        assert_eq!(
            check(&hole),
            Err(TableGridLayoutError::GridHole(NodeId::new(6)))
        );
        let mut out_of_range = body.clone();
        out_of_range[1].cells[0].colspan = NonZeroU16::new(5).unwrap();
        assert_eq!(
            check(&out_of_range),
            Err(TableGridLayoutError::GridOutOfRange(NodeId::new(7)))
        );
        let mut wrong_rowspan = body.clone();
        wrong_rowspan[1].cells[0].rowspan = NonZeroU16::new(2).unwrap();
        assert_eq!(
            check(&wrong_rowspan),
            Err(TableGridLayoutError::RowspanOutOfRange(NodeId::new(7)))
        );

        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let ir = table_grid_ir(package.package(), package_epoch, &limits, false).unwrap();
        let other = staging_table_grid_package(3);
        let wrong_style = other.compute_table_style(NodeId::new(1)).unwrap();
        assert_eq!(
            layout_table_grid(
                &package,
                NodeId::new(1),
                &wrong_style,
                &ir,
                list_length(12),
                &limits,
            )
            .unwrap_err(),
            TableGridLayoutError::WrongStyleReceipt
        );

        let style = package.compute_table_style(NodeId::new(1)).unwrap();
        let mut wrong_flow =
            table_grid_ir(package.package(), package_epoch, &limits, false).unwrap();
        wrong_flow.registry.flows[1].owner_node_id = NodeId::new(99);
        assert_eq!(
            layout_table_grid(
                &package,
                NodeId::new(1),
                &style,
                &wrong_flow,
                list_length(12),
                &limits,
            )
            .unwrap_err(),
            TableGridLayoutError::MissingCellFlow(NodeId::new(3))
        );
    }

    #[test]
    fn table_grid_fixed_columns_require_exact_width_at_safe_integer_max() {
        let maximum = list_length(JSON_SAFE_INTEGER_MAX);
        let exact = [TableColumn {
            sizing: ColumnSizing::Fixed(maximum),
        }];
        let (resolved, residual, recipient) = resolve_table_columns(&exact, maximum).unwrap();
        assert_eq!(resolved[0].final_width(), maximum);
        assert_eq!(residual, Length::ZERO);
        assert_eq!(recipient, None);

        let short = [TableColumn {
            sizing: ColumnSizing::Fixed(list_length(JSON_SAFE_INTEGER_MAX - 1)),
        }];
        assert_eq!(
            resolve_table_columns(&short, maximum),
            Err(TableGridLayoutError::ColumnArithmetic)
        );
        let over = [
            TableColumn {
                sizing: ColumnSizing::Fixed(maximum),
            },
            TableColumn {
                sizing: ColumnSizing::Fixed(list_length(1)),
            },
        ];
        assert_eq!(
            resolve_table_columns(&over, maximum),
            Err(TableGridLayoutError::ColumnArithmetic)
        );

        let last_fraction_before_fixed = [
            TableColumn {
                sizing: ColumnSizing::Fraction(NonZeroU16::new(1).unwrap()),
            },
            TableColumn {
                sizing: ColumnSizing::Fraction(NonZeroU16::new(1).unwrap()),
            },
            TableColumn {
                sizing: ColumnSizing::Fixed(list_length(7)),
            },
        ];
        let (resolved, residual, recipient) =
            resolve_table_columns(&last_fraction_before_fixed, list_length(10)).unwrap();
        assert_eq!(
            resolved
                .iter()
                .map(|column| column.final_width().get().raw())
                .collect::<Vec<_>>(),
            [2, 1, 7]
        );
        assert_eq!(residual.raw(), -1);
        assert_eq!(recipient, Some(1));
    }

    static NEXT_FLOW_REGISTRY_FIXTURE: AtomicU64 = AtomicU64::new(0);

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    fn multi_flow_machine_package() -> (Box<ValidatedMachinePackage>, LayoutEpoch) {
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
                    wire::WireBlock::Figure {
                        node_id: 8,
                        span,
                        classes: Vec::new(),
                        image_id: 0,
                        alt: "diagram".to_owned(),
                        caption: vec![paragraph(9)],
                    },
                    wire::WireBlock::PageBreak {
                        node_id: 10,
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
                images: vec![wire::WireImage {
                    image_id: 0,
                    uri: "image.png".to_owned(),
                    expected_sha256: None,
                }],
            },
        };
        let root = std::env::temp_dir().join(format!(
            "typaxis-layout-flow-registry-{}-{}",
            std::process::id(),
            NEXT_FLOW_REGISTRY_FIXTURE.fetch_add(1, AtomicOrdering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let package_path = root.join("document-package.json");
        fs::write(
            &package_path,
            wire::DocumentPackageEncoder::default()
                .to_jcs_vec(&package)
                .unwrap(),
        )
        .unwrap();
        fs::write(root.join("input.tsf"), []).unwrap();
        fs::write(
            root.join("image.png"),
            [
                137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0,
                1, 1, 3, 0, 0, 0, 206, 236, 237, 201, 0, 0, 0, 6, 80, 76, 84, 69, 255, 0, 0, 0,
                255, 0, 210, 135, 239, 113, 0, 0, 0, 2, 116, 82, 78, 83, 255, 0, 229, 183, 48, 74,
                0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 112, 0, 0, 0, 66, 0, 65, 41, 55, 244,
                239, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
            ],
        )
        .unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let (session, raw) = HostMachineInputSession::open(
            MachineInputHostOptions::new(HostPath::new(package_path.clone()).unwrap(), None),
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
        let config = EffectiveConfig::new(
            false,
            PdfStreamCompression::Flate,
            vec![ConfigResourceRoot::ProjectRoot],
            DEFAULT_ALLOWED_URI_SCHEMES
                .iter()
                .map(|scheme| (*scheme).to_owned())
                .collect(),
            EffectiveDataVersions::new(
                REGISTERED_UNICODE_VERSION,
                REGISTERED_JAPANESE_LINE_BREAK_VERSION,
            )
            .unwrap(),
            ResourceLimits::default(),
        )
        .unwrap();
        let host = HostAdmissionContext::new(
            HostPath::new(package_path).unwrap(),
            HostPath::new(root.clone()).unwrap(),
            None,
            Vec::new(),
        );
        let resource_session = HostResourceAdmissionSession::new(
            &host,
            &config,
            &parsed.package().package().resources,
        )
        .unwrap();
        let mut resolver = AdmittedResourceResolver::new_with_roots(
            &parsed.package().package().resources,
            &limits,
            resource_session.roots(),
        )
        .unwrap();
        let pending = resolver
            .read_image(
                resource_session
                    .open_image(ImageResourceId::new(0))
                    .unwrap(),
            )
            .unwrap();
        resolver.parse_and_bind_png(pending).unwrap();
        let admitted_resources = resolver.finish().unwrap();
        let generated = parsed
            .package()
            .materialize_initial_generated_text(&limits)
            .unwrap();
        let generated = parsed
            .package()
            .bind_generated_text(&generated, &limits)
            .unwrap();
        let epoch =
            LayoutEpoch::from_validated_inputs(generated, admitted_resources.token()).unwrap();
        fs::remove_dir_all(root).unwrap();
        (parsed, epoch)
    }
    fn epoch(package: &ValidatedParsedPackage) -> LayoutEpoch {
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
        let generated = package.bind_generated_text(&generated, &limits).unwrap();
        LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap()
    }
    fn frame() -> Rect {
        let size =
            typaxis_core::PositiveLength::new(typaxis_core::Length::from_raw(10).unwrap()).unwrap();
        Rect::new(
            typaxis_core::Length::ZERO,
            typaxis_core::Length::ZERO,
            size,
            size,
        )
    }
    fn empty_paragraph_flow(package: &ValidatedParsedPackage) -> FlowTree {
        let package_epoch = epoch(package);
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(package, package_epoch).unwrap();
        let mut builder = CanonicalFlowIrBuilder::new(package, &paragraph_items).unwrap();
        for block in &package.package().document.blocks {
            let Block::Paragraph { node_id, .. } = block else {
                panic!("test package must contain only paragraphs");
            };
            builder.push_paragraph_item(*node_id, 0).unwrap();
        }
        builder.finish(package_epoch).unwrap()
    }
    fn page_context(
        package: &ValidatedParsedPackage,
        flow: &FlowTree,
        cursor: &FlowCursor,
    ) -> PageContext {
        PageContext::select(
            0,
            &ResolvedPageSelection::new(flow, cursor, package).unwrap(),
            &package.pagination_context(),
        )
        .unwrap()
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct CountingBudget {
        remaining_fragments: u64,
        consumed_fragments: u64,
        fragment_calls: u64,
    }
    impl CountingBudget {
        const fn new(remaining_fragments: u64) -> Self {
            Self {
                remaining_fragments,
                consumed_fragments: 0,
                fragment_calls: 0,
            }
        }
    }
    impl FragmentWorkBudget for CountingBudget {
        fn consume_fragments(&mut self, count: u64) -> Result<(), FragmentError> {
            self.fragment_calls = self
                .fragment_calls
                .checked_add(1)
                .ok_or(FragmentError::ArithmeticOverflow)?;
            let remaining = self
                .remaining_fragments
                .checked_sub(count)
                .ok_or(FragmentError::ResourceLimit)?;
            self.remaining_fragments = remaining;
            self.consumed_fragments = self
                .consumed_fragments
                .checked_add(count)
                .ok_or(FragmentError::ArithmeticOverflow)?;
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
    #[test]
    fn page_flags_are_derived() {
        let package = validated_package(1);
        let flow = FlowTree::empty(&package, epoch(&package)).unwrap();
        let cursor = FlowCursor::document_start(&flow);
        let selection = ResolvedPageSelection::new(&flow, &cursor, &package).unwrap();
        let package_context = package.pagination_context();
        let context = PageContext::select(0, &selection, &package_context).unwrap();
        assert!(context.is_first());
        assert!(context.is_odd());
        assert_eq!(context.physical_page_number().get(), 1);
        assert_eq!(
            PageContext::select(u32::MAX, &selection, &package_context),
            Err(PageContextError::PageNumberOverflow)
        );
    }
    #[test]
    fn request_rejects_cursor_from_another_epoch() {
        let package = validated_package(1);
        let other_package = validated_package(9);
        assert_eq!(
            FlowTree::empty(&package, epoch(&other_package)),
            Err(FlowTreeError::EpochPackageMismatch)
        );
        let flow = FlowTree::empty(&package, epoch(&package)).unwrap();
        let other = FlowTree::empty(&other_package, epoch(&other_package)).unwrap();
        let cursor = FlowCursor::document_start(&other);
        let package_context = package.pagination_context();
        let selection =
            ResolvedPageSelection::new(&flow, &FlowCursor::document_start(&flow), &package)
                .unwrap();
        let page = PageContext::select(0, &selection, &package_context).unwrap();
        assert_eq!(
            FragmentRequest::new(&flow, &cursor, frame(), NonNegativeLength::ZERO, page),
            Err(FragmentError::InvalidCursorEpoch)
        );
    }
    #[test]
    fn epoch_rejects_generated_overlay_from_another_document_registry() {
        let package = validated_package(1);
        let paragraph_package = paragraph_package(2);
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated = GeneratedTextStore::new(
            vec![],
            paragraph_package.document_nodes(),
            &limits,
            &TextStore::new(vec![]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            package.bind_generated_text(&generated, &limits),
            Err(PackageGeneratedTextError::DocumentMismatch)
        );
    }
    #[test]
    fn resolved_text_style_is_bound_to_package_style_and_admission() {
        let package = paragraph_package(1);
        let other = paragraph_package(2);
        let computed = other.cascade_style(NodeId::new(1)).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!(
            ResolvedLayoutTextStyle::new(&package, &computed, admitted.token()),
            Err(LayoutTextStyleError::PackageStyleMismatch)
        );

        let computed = package.cascade_style(NodeId::new(1)).unwrap();
        assert_eq!(
            ResolvedLayoutTextStyle::new(&package, &computed, admitted.token()),
            Err(LayoutTextStyleError::InvalidStyle(
                StyleValidationError::MissingTextProperty
            ))
        );

        let instances = AdmittedFontInstanceTable::from_used_faces(&admitted, []).unwrap();
        let other_computed = other.cascade_style(NodeId::new(1)).unwrap();
        assert_eq!(
            ShapeFontSelectionReceipt::new(
                &package,
                &other_computed,
                admitted.token(),
                &instances,
                epoch(&package),
            ),
            Err(ShapeFontSelectionError::LayoutStyle(
                LayoutTextStyleError::PackageStyleMismatch
            ))
        );
        assert_eq!(
            ShapeFontSelectionReceipt::new(
                &package,
                &computed,
                admitted.token(),
                &instances,
                epoch(&package),
            ),
            Err(ShapeFontSelectionError::LayoutStyle(
                LayoutTextStyleError::InvalidStyle(StyleValidationError::MissingTextProperty)
            ))
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn canonical_flow_registry_is_dense_complete_and_insertion_order_independent() {
        let (machine, package_epoch) = multi_flow_machine_package();
        let package = machine.package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(package, package_epoch).unwrap();

        let mut reversed =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &limits)
                .unwrap();
        let mut owners: Vec<_> = reversed.expected_content_owners().collect();
        assert_eq!(owners, [1, 3, 4, 6, 7, 8, 9, 10].map(NodeId::new).to_vec());
        owners.reverse();
        for owner in owners {
            let content = reversed.issue_content(owner).unwrap();
            reversed.register_content(content).unwrap();
        }
        let reversed = reversed.finish().unwrap();

        let mut canonical =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &limits)
                .unwrap();
        let owners: Vec<_> = canonical.expected_content_owners().collect();
        for owner in owners {
            let content = canonical.issue_content(owner).unwrap();
            canonical.register_content(content).unwrap();
        }
        let canonical = canonical.finish().unwrap();

        assert_eq!(
            reversed.registry().receipt().fingerprint(),
            canonical.registry().receipt().fingerprint()
        );
        assert_eq!(reversed.registry().receipt().flow_count(), 4);
        assert_eq!(reversed.registry().receipt().max_depth(), 3);
        assert_eq!(
            reversed
                .registry()
                .flows()
                .iter()
                .map(|flow| (
                    flow.flow_id().get(),
                    flow.owner_node_id().get(),
                    flow.parent_flow_id().map(FlowId::get),
                    flow.terminal().owner_local_ordinal(),
                ))
                .collect::<Vec<_>>(),
            vec![
                (0, 0, None, 4),
                (1, 3, Some(0), 2),
                (2, 6, Some(1), 1),
                (3, 8, Some(0), 1),
            ]
        );
        assert_eq!(reversed.flows(), canonical.flows());
        assert_eq!(reversed.flow(FlowId::new(0)).unwrap().positions().len(), 5);
        assert_eq!(reversed.flow(FlowId::new(1)).unwrap().positions().len(), 3);
        assert_eq!(reversed.flow(FlowId::new(2)).unwrap().positions().len(), 2);
        assert_eq!(reversed.flow(FlowId::new(3)).unwrap().positions().len(), 2);
        for flow in reversed.flows() {
            assert!(flow.terminal_position().is_terminal());
            assert_eq!(
                flow.terminal_position().flow_local_ordinal(),
                flow.descriptor().terminal().owner_local_ordinal()
            );
            assert!(flow.positions()[..flow.positions().len() - 1]
                .iter()
                .all(|position| !position.is_terminal()));
        }
        assert_eq!(
            reversed.flow(FlowId::new(0)).unwrap().positions()[1].child_flow_id(),
            Some(FlowId::new(1))
        );
        assert_eq!(
            reversed.flow(FlowId::new(1)).unwrap().positions()[1].child_flow_id(),
            Some(FlowId::new(2))
        );
        assert_eq!(
            reversed.flow(FlowId::new(0)).unwrap().positions()[2].child_flow_id(),
            Some(FlowId::new(3))
        );
        assert_eq!(
            reversed.flow(FlowId::new(0)).unwrap().positions()[2].content_kind(),
            Some(FlowContentKind::FigureCaption)
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn canonical_flow_registry_finish_rejects_incomplete_and_tampered_content() {
        let (machine, package_epoch) = multi_flow_machine_package();
        let package = machine.package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(package, package_epoch).unwrap();

        let mut missing =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &limits)
                .unwrap();
        for owner in [1, 3, 4, 6, 7, 8, 9].map(NodeId::new) {
            let content = missing.issue_content(owner).unwrap();
            missing.register_content(content).unwrap();
        }
        assert_eq!(
            missing.finish().unwrap_err(),
            FlowRegistryError::MissingContent(NodeId::new(10))
        );

        let mut duplicate =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &limits)
                .unwrap();
        let owners: Vec<_> = duplicate.expected_content_owners().collect();
        for owner in &owners {
            let content = duplicate.issue_content(*owner).unwrap();
            duplicate.register_content(content).unwrap();
        }
        duplicate
            .register_content(duplicate.issue_content(NodeId::new(1)).unwrap())
            .unwrap();
        assert_eq!(
            duplicate.finish().unwrap_err(),
            FlowRegistryError::ExtraContent(NodeId::new(1))
        );

        let mut wrong_owner =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &limits)
                .unwrap();
        wrong_owner
            .register_content_for(
                NodeId::new(1),
                wrong_owner.issue_content(NodeId::new(3)).unwrap(),
            )
            .unwrap();
        for owner in [3, 4, 6, 7, 8, 9, 10].map(NodeId::new) {
            let content = wrong_owner.issue_content(owner).unwrap();
            wrong_owner.register_content(content).unwrap();
        }
        assert_eq!(
            wrong_owner.finish().unwrap_err(),
            FlowRegistryError::WrongOwner {
                registered: NodeId::new(1),
                actual: NodeId::new(3),
            }
        );

        let mut wrong_kind =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &limits)
                .unwrap();
        let paragraph = wrong_kind.issue_content(NodeId::new(1)).unwrap();
        let receipt = match paragraph {
            ValidatedFlowContent::Paragraph(value) => value.0,
            _ => unreachable!(),
        };
        wrong_kind
            .register_content(ValidatedFlowContent::PageBreak(
                ValidatedPageBreakFlowContent(receipt),
            ))
            .unwrap();
        for owner in [3, 4, 6, 7, 8, 9, 10].map(NodeId::new) {
            let content = wrong_kind.issue_content(owner).unwrap();
            wrong_kind.register_content(content).unwrap();
        }
        assert_eq!(
            wrong_kind.finish().unwrap_err(),
            FlowRegistryError::WrongContentKind {
                owner: NodeId::new(1),
                expected: FlowContentKind::Paragraph,
                actual: FlowContentKind::PageBreak,
            }
        );

        let mut wrong_terminal =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &limits)
                .unwrap();
        let mut paragraph = wrong_terminal.issue_content(NodeId::new(1)).unwrap();
        match &mut paragraph {
            ValidatedFlowContent::Paragraph(value) => value.0.boundary_count = 2,
            _ => unreachable!(),
        }
        wrong_terminal.register_content(paragraph).unwrap();
        for owner in [3, 4, 6, 7, 8, 9, 10].map(NodeId::new) {
            let content = wrong_terminal.issue_content(owner).unwrap();
            wrong_terminal.register_content(content).unwrap();
        }
        assert_eq!(
            wrong_terminal.finish().unwrap_err(),
            FlowRegistryError::WrongTerminal(FlowId::DOCUMENT_BODY)
        );

        let other = paragraph_package(99);
        let other_epoch = epoch(&other);
        let mut wrong_epoch =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &limits)
                .unwrap();
        let mut paragraph = wrong_epoch.issue_content(NodeId::new(1)).unwrap();
        match &mut paragraph {
            ValidatedFlowContent::Paragraph(value) => value.0.epoch = other_epoch,
            _ => unreachable!(),
        }
        wrong_epoch.register_content(paragraph).unwrap();
        for owner in [3, 4, 6, 7, 8, 9, 10].map(NodeId::new) {
            let content = wrong_epoch.issue_content(owner).unwrap();
            wrong_epoch.register_content(content).unwrap();
        }
        assert_eq!(
            wrong_epoch.finish().unwrap_err(),
            FlowRegistryError::WrongEpoch(NodeId::new(1))
        );

        let mut wrong_parent =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &limits)
                .unwrap();
        wrong_parent.content.model.flows[1].parent_flow_id = Some(FlowId::new(1));
        let owners: Vec<_> = wrong_parent.expected_content_owners().collect();
        for owner in owners {
            let content = wrong_parent.issue_content(owner).unwrap();
            wrong_parent.register_content(content).unwrap();
        }
        assert_eq!(
            wrong_parent.finish().unwrap_err(),
            FlowRegistryError::WrongParent(FlowId::new(1))
        );
    }

    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    #[test]
    fn canonical_flow_registry_rechecks_count_and_depth_limits_before_ir() {
        let (machine, package_epoch) = multi_flow_machine_package();
        let package = machine.package();
        let default_limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(package, package_epoch).unwrap();

        let exact = ValidatedResourceLimits::new(ResourceLimits {
            max_ast_nodes: package.document_nodes().node_count() as u64,
            max_ast_nesting_depth: 3,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert!(
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &exact,).is_ok()
        );

        let too_few_nodes = ValidatedResourceLimits::new(ResourceLimits {
            max_ast_nodes: package.document_nodes().node_count() as u64 - 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert!(matches!(
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &too_few_nodes,),
            Err(FlowRegistryError::AstNodeLimit)
        ));

        let too_shallow = ValidatedResourceLimits::new(ResourceLimits {
            max_ast_nesting_depth: 2,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert!(matches!(
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &too_shallow,),
            Err(FlowRegistryError::FlowDepthLimit)
        ));

        let mut finish_count =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &default_limits)
                .unwrap();
        finish_count.content.max_ast_nodes = u64::try_from(package.document_nodes().node_count())
            .unwrap()
            .checked_sub(1)
            .unwrap();
        let owners: Vec<_> = finish_count.expected_content_owners().collect();
        for owner in owners {
            let content = finish_count.issue_content(owner).unwrap();
            finish_count.register_content(content).unwrap();
        }
        assert_eq!(
            finish_count.finish().unwrap_err(),
            FlowRegistryError::AstNodeLimit
        );

        let mut finish_depth =
            ProductionFlowIrBuilder::new(package, &paragraph_items, package_epoch, &default_limits)
                .unwrap();
        finish_depth.content.max_ast_nesting_depth = 2;
        let owners: Vec<_> = finish_depth.expected_content_owners().collect();
        for owner in owners {
            let content = finish_depth.issue_content(owner).unwrap();
            finish_depth.register_content(content).unwrap();
        }
        assert_eq!(
            finish_depth.finish().unwrap_err(),
            FlowRegistryError::FlowDepthLimit
        );
    }

    #[test]
    fn continuation_requires_monotonic_structured_progress() {
        let package = paragraph_package(1);
        let package_epoch = epoch(&package);
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(&package, package_epoch).unwrap();
        let mut builder = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        builder.push_paragraph_item(NodeId::new(1), 0).unwrap();
        builder.push_paragraph_item(NodeId::new(2), 0).unwrap();
        let flow = builder.finish(package_epoch).unwrap();
        assert_eq!(flow.positions().len(), 4);
        assert_eq!(flow.positions().last().unwrap().owner(), NodeId::new(0));
        assert_eq!(flow.positions().last().unwrap().owner_local_boundary(), 1);
        let cursor = FlowCursor::document_start(&flow);
        let request = FragmentRequest::new(
            &flow,
            &cursor,
            frame(),
            NonNegativeLength::ZERO,
            PageContext::select(
                0,
                &ResolvedPageSelection::new(&flow, &cursor, &package).unwrap(),
                &package.pagination_context(),
            )
            .unwrap(),
        )
        .unwrap();
        let stalled = FlowCursor::document_start(&flow);
        let result = FragmentResult {
            fragments: vec![],
            continuation: Continuation::More(Box::new(stalled)),
            discovered_footnotes: vec![],
            discovered_anchors: vec![],
        };
        assert_eq!(
            result.validate_progress(&request),
            Err(FragmentError::NoProgress)
        );

        let advanced = FlowCursor::at(&flow, 1, CursorPosition::ParagraphItem(0)).unwrap();
        let result = FragmentResult {
            fragments: vec![],
            continuation: Continuation::More(Box::new(advanced.clone())),
            discovered_footnotes: vec![],
            discovered_anchors: vec![],
        };
        assert!(result.validate_progress(&request).is_ok());

        let root_fragment = FragmentDraft::new(
            flow.positions()[0].clone(),
            flow.positions()[1].clone(),
            frame(),
            0,
        )
        .unwrap();
        assert_eq!(
            FragmentResult {
                fragments: vec![root_fragment],
                continuation: Continuation::More(Box::new(advanced.clone())),
                discovered_footnotes: vec![],
                discovered_anchors: vec![],
            }
            .validate_progress(&request),
            Err(FragmentError::InvalidFragmentRange)
        );

        let advanced_request = FragmentRequest::new(
            &flow,
            &advanced,
            frame(),
            NonNegativeLength::ZERO,
            request.page().clone(),
        )
        .unwrap();

        let terminal = flow.terminal_cursor();
        assert_eq!(
            FragmentResult {
                fragments: vec![],
                continuation: Continuation::More(Box::new(terminal.clone())),
                discovered_footnotes: vec![],
                discovered_anchors: vec![],
            }
            .validate_progress(&request),
            Err(FragmentError::InvalidCursorLocation)
        );
        assert!(FragmentResult {
            fragments: vec![FragmentDraft::new(
                flow.positions()[1].clone(),
                flow.positions().last().unwrap().clone(),
                frame(),
                0,
            )
            .unwrap()],
            continuation: Continuation::Exhausted(Box::new(terminal)),
            discovered_footnotes: vec![],
            discovered_anchors: vec![],
        }
        .validate_progress(&advanced_request)
        .is_ok());

        assert_eq!(
            FlowCursor::at(&flow, 0, CursorPosition::End,),
            Err(FragmentError::InvalidCursorLocation)
        );

        let mut same = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        same.push_paragraph_item(NodeId::new(2), 0).unwrap();
        same.push_paragraph_item(NodeId::new(1), 0).unwrap();
        assert_eq!(
            flow.positions(),
            same.finish(package_epoch).unwrap().positions()
        );

        let mut invalid = CanonicalFlowIrBuilder::new(&package, &paragraph_items).unwrap();
        assert_eq!(
            invalid.push_table_row(NodeId::new(1)),
            Err(FlowTreeError::InvalidOwnerKind)
        );
        assert_eq!(
            invalid.push_paragraph_item(NodeId::new(99), 0),
            Err(FlowTreeError::UnknownOwner)
        );
        assert_eq!(
            invalid.push_paragraph_item(NodeId::new(1), 1),
            Err(FlowTreeError::InvalidOwnerBoundary)
        );
        assert_eq!(
            FlowTree::empty(&package, epoch(&package)),
            Err(FlowTreeError::NonEmptyDocument)
        );
        assert_eq!(
            CanonicalFlowIrBuilder::new(&package, &paragraph_items)
                .unwrap()
                .finish(package_epoch),
            Err(FlowTreeError::MissingOwnerBoundary)
        );

        let valid_fragment = FragmentDraft::new(
            flow.positions()[1].clone(),
            flow.positions().last().unwrap().clone(),
            frame(),
            0,
        )
        .unwrap();
        assert_eq!(valid_fragment.start(), &flow.positions()[1]);
        assert_eq!(valid_fragment.end(), flow.positions().last().unwrap());
        assert_eq!(
            FragmentDraft::new(
                flow.positions().last().unwrap().clone(),
                flow.positions()[1].clone(),
                frame(),
                0,
            ),
            Err(FragmentError::InvalidFragmentRange)
        );
        let terminal = flow.terminal_cursor();
        assert!(FragmentResult {
            fragments: vec![valid_fragment.clone()],
            continuation: Continuation::Exhausted(Box::new(terminal.clone())),
            discovered_footnotes: vec![],
            discovered_anchors: vec![],
        }
        .validate_progress(&advanced_request)
        .is_ok());
        assert_eq!(
            FragmentResult {
                fragments: vec![valid_fragment.clone(), valid_fragment],
                continuation: Continuation::Exhausted(Box::new(terminal)),
                discovered_footnotes: vec![],
                discovered_anchors: vec![],
            }
            .validate_progress(&advanced_request),
            Err(FragmentError::InvalidFragmentRange)
        );
        let other_package = paragraph_package(2);
        let other_epoch = epoch(&other_package);
        let other_items =
            ValidatedParagraphItemRegistry::for_empty_content(&other_package, other_epoch).unwrap();
        let mut other_builder = CanonicalFlowIrBuilder::new(&other_package, &other_items).unwrap();
        other_builder
            .push_paragraph_item(NodeId::new(1), 0)
            .unwrap();
        other_builder
            .push_paragraph_item(NodeId::new(2), 0)
            .unwrap();
        let other = other_builder.finish(other_epoch).unwrap();
        let outside = FragmentDraft::new(
            flow.positions()[1].clone(),
            other.positions()[1].clone(),
            frame(),
            0,
        );
        assert_eq!(outside, Err(FragmentError::InvalidCursorEpoch));
    }

    #[test]
    fn reference_fragmenter_is_reentrant_and_deterministic() {
        let package = parsed_reference_package(17, "anchor:z\nparagraph\nanchor:a");
        let flow = empty_paragraph_flow(&package);
        let fragmenter = ReferenceFragmenter::for_empty_paragraphs(&package, &flow).unwrap();
        let start = FlowCursor::document_start(&flow);
        let request = FragmentRequest::new(
            &flow,
            &start,
            frame(),
            NonNegativeLength::ZERO,
            page_context(&package, &flow, &start),
        )
        .unwrap();

        let mut first_budget = CountingBudget::new(u64::MAX);
        let first = fragmenter.fragment(&request, &mut first_budget).unwrap();
        let mut repeated_budget = CountingBudget::new(u64::MAX);
        let repeated = fragmenter.fragment(&request, &mut repeated_budget).unwrap();
        assert_eq!(first, repeated);
        assert_eq!(first_budget.consumed_fragments, 0);
        assert_eq!(repeated_budget.consumed_fragments, 0);
        assert!(first.validate_progress(&request).is_ok());
        let next = match &first.continuation {
            Continuation::More(next) => next.as_ref().clone(),
            Continuation::Exhausted(_) => panic!("nonblank bootstrap must continue"),
        };
        assert_eq!(next.position(), &flow.positions()[1]);

        let continuation_request = FragmentRequest::new(
            &flow,
            &next,
            frame(),
            NonNegativeLength::ZERO,
            request.page().clone(),
        )
        .unwrap();
        let mut continuation_budget = CountingBudget::new(u64::MAX);
        let laid_out = fragmenter
            .fragment(&continuation_request, &mut continuation_budget)
            .unwrap();
        let mut repeated_continuation_budget = CountingBudget::new(u64::MAX);
        let repeated_laid_out = fragmenter
            .fragment(&continuation_request, &mut repeated_continuation_budget)
            .unwrap();
        assert_eq!(laid_out, repeated_laid_out);
        assert_eq!(continuation_budget.consumed_fragments, 3);
        assert_eq!(repeated_continuation_budget.consumed_fragments, 3);
        assert_eq!(laid_out.fragments.len(), 3);
        assert!(laid_out.validate_progress(&continuation_request).is_ok());
        for (index, fragment) in laid_out.fragments.iter().enumerate() {
            assert_eq!(fragment.start(), &flow.positions()[index + 1]);
            assert_eq!(fragment.end(), &flow.positions()[index + 2]);
            assert_eq!(fragment.bounds(), frame());
            assert_eq!(fragment.break_after_penalty(), 0);
        }
        assert_eq!(
            laid_out
                .discovered_anchors
                .iter()
                .map(|anchor| anchor.anchor_id.clone())
                .collect::<Vec<_>>(),
            vec![AnchorId::new("z").unwrap(), AnchorId::new("a").unwrap()]
        );
        for anchor in &laid_out.discovered_anchors {
            assert_eq!(
                package.document_nodes().anchor_owner(&anchor.anchor_id),
                Some(anchor.owner_node)
            );
            assert_eq!(
                flow.anchor_owner(&anchor.anchor_id),
                Some(anchor.owner_node)
            );
            assert_eq!(
                anchor.position_in_frame,
                Point {
                    x: Length::ZERO,
                    y: Length::ZERO,
                }
            );
        }
        assert_eq!(
            laid_out.continuation,
            Continuation::Exhausted(Box::new(flow.terminal_cursor()))
        );
        assert!(laid_out.discovered_footnotes.is_empty());
    }

    #[test]
    fn reference_fragmenter_honors_blank_and_terminal_semantics() {
        let package = validated_package(18);
        let flow = FlowTree::empty(&package, epoch(&package)).unwrap();
        let fragmenter = ReferenceFragmenter::for_empty_paragraphs(&package, &flow).unwrap();
        let start = FlowCursor::document_start(&flow);
        let request = FragmentRequest::new(
            &flow,
            &start,
            frame(),
            NonNegativeLength::ZERO,
            page_context(&package, &flow, &start),
        )
        .unwrap();
        let mut budget = CountingBudget::new(0);
        let result = fragmenter.fragment(&request, &mut budget).unwrap();
        assert!(result.fragments.is_empty());
        assert!(result.discovered_anchors.is_empty());
        assert_eq!(budget.consumed_fragments, 0);
        assert_eq!(
            result.continuation,
            Continuation::Exhausted(Box::new(flow.terminal_cursor()))
        );
        assert!(result.validate_progress(&request).is_ok());

        let terminal = flow.terminal_cursor();
        let terminal_request = FragmentRequest::new(
            &flow,
            &terminal,
            frame(),
            NonNegativeLength::ZERO,
            request.page().clone(),
        )
        .unwrap();
        assert_eq!(
            fragmenter.fragment(&terminal_request, &mut budget),
            Err(FragmentError::InvalidCursorLocation)
        );
        assert_eq!(budget.fragment_calls, 0);
    }

    #[test]
    fn reference_fragmenter_rejects_unsupported_content_and_budget_before_output() {
        let supported = paragraph_package(19);
        let flow = empty_paragraph_flow(&supported);
        let unsupported = parsed_reference_package(20, "paragraph\ntext:actual");
        assert!(matches!(
            ReferenceFragmenter::for_empty_paragraphs(&unsupported, &flow),
            Err(FragmentError::UnsupportedFlowDomain)
        ));

        let fragmenter = ReferenceFragmenter::for_empty_paragraphs(&supported, &flow).unwrap();
        let start = FlowCursor::document_start(&flow);
        let page = page_context(&supported, &flow, &start);
        let bootstrap_request = FragmentRequest::new(
            &flow,
            &start,
            frame(),
            NonNegativeLength::ZERO,
            page.clone(),
        )
        .unwrap();
        let mut bootstrap_budget = CountingBudget::new(0);
        let bootstrap = fragmenter
            .fragment(&bootstrap_request, &mut bootstrap_budget)
            .unwrap();
        let next = match bootstrap.continuation {
            Continuation::More(next) => *next,
            Continuation::Exhausted(_) => panic!("nonblank bootstrap must continue"),
        };
        let request =
            FragmentRequest::new(&flow, &next, frame(), NonNegativeLength::ZERO, page).unwrap();
        let mut insufficient = CountingBudget::new(1);
        assert_eq!(
            fragmenter.fragment(&request, &mut insufficient),
            Err(FragmentError::ResourceLimit)
        );
        assert_eq!(insufficient.fragment_calls, 1);
        assert_eq!(insufficient.consumed_fragments, 0);
    }

    #[test]
    fn footnote_body_fragmenter_moves_a_kept_last_line_with_the_next_first_line() {
        let package = parsed_reference_package(21, "paragraph\nparagraph\nparagraph");
        let flow = empty_paragraph_flow(&package);
        assert_eq!(flow.positions().len(), 5);
        let fragmenter = ReferenceFragmenter {
            flow: &flow,
            anchors: Vec::new(),
            footnotes: vec![ReferenceFootnotePlacement {
                flow_ordinal: 3,
                reference_owner: NodeId::new(99),
                footnote_id: FootnoteId::new("kept").unwrap(),
            }],
            lines: vec![
                ReferenceLinePlacement {
                    start: 1,
                    end: 2,
                    height: positive(3),
                    forced_break: false,
                    keep_with_next: false,
                },
                ReferenceLinePlacement {
                    start: 2,
                    end: 3,
                    height: positive(4),
                    forced_break: false,
                    keep_with_next: true,
                },
                ReferenceLinePlacement {
                    start: 3,
                    end: 4,
                    height: positive(4),
                    forced_break: false,
                    keep_with_next: false,
                },
            ],
            legacy_full_frame: false,
            basic_document: true,
            enforce_keep_with_next: true,
        };
        let start = fragmenter.cursor_at(1).unwrap();
        let request = FragmentRequest::new(
            &flow,
            &start,
            frame(),
            NonNegativeLength::ZERO,
            page_context(&package, &flow, &start),
        )
        .unwrap();
        let mut budget = CountingBudget::new(3);
        let first = fragmenter.fragment(&request, &mut budget).unwrap();
        assert_eq!(first.fragments.len(), 1);
        assert_eq!(first.fragments[0].start(), &flow.positions()[1]);
        assert_eq!(first.fragments[0].end(), &flow.positions()[2]);
        let Continuation::More(next) = first.continuation else {
            panic!("the protected pair must move to the next body frame");
        };

        let next_request = FragmentRequest::new(
            &flow,
            &next,
            frame(),
            NonNegativeLength::ZERO,
            request.page().clone(),
        )
        .unwrap();
        let second = fragmenter.fragment(&next_request, &mut budget).unwrap();
        assert_eq!(second.fragments.len(), 2);
        assert_eq!(second.fragments[0].start(), &flow.positions()[2]);
        assert_eq!(second.fragments[1].end(), &flow.positions()[4]);
        assert_eq!(
            second.continuation,
            Continuation::Exhausted(Box::new(flow.terminal_cursor()))
        );

        let candidate = (1..4)
            .map(|index| {
                FragmentDraft::new(
                    flow.positions()[index].clone(),
                    flow.positions()[index + 1].clone(),
                    frame(),
                    0,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            fragmenter
                .legal_cut_index_before_reference(&candidate, NodeId::new(99))
                .unwrap(),
            Some(1)
        );
    }

    #[test]
    fn footnote_body_fragmenter_applies_list_keep_to_each_lists_last_line() {
        let staging = staging_machine_list_package_with_keep(true);
        let package = staging.package();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let generated_store = package.materialize_initial_generated_text(&limits).unwrap();
        let generated = package
            .bind_generated_text(&generated_store, &limits)
            .unwrap();
        let admitted = AdmittedResourceResolver::new(&package.package().resources, &limits)
            .unwrap()
            .finish()
            .unwrap();
        let package_epoch =
            LayoutEpoch::from_validated_inputs(generated, admitted.token()).unwrap();
        let paragraph_items =
            ValidatedParagraphItemRegistry::for_empty_content(package, package_epoch).unwrap();
        let mut builder =
            CanonicalFlowIrBuilder::new_for_footnote_body(package, &paragraph_items).unwrap();
        for (node, kind) in package.document_nodes().nodes() {
            match kind {
                DocumentNodeKind::Paragraph | DocumentNodeKind::Heading => {
                    builder.push_paragraph_item(node, 0).unwrap();
                }
                DocumentNodeKind::ListItem => builder.push_list_item(node).unwrap(),
                DocumentNodeKind::Figure | DocumentNodeKind::PageBreak => {
                    builder.push_block_item(node).unwrap();
                }
                DocumentNodeKind::TableRow => builder.push_table_row(node).unwrap(),
                _ => {}
            }
        }
        let flow = builder.finish(package_epoch).unwrap();
        let fragmenter = ReferenceFragmenter::for_footnote_body(package, &flow).unwrap();

        for list_owner in [NodeId::new(1), NodeId::new(4)] {
            let path = package.document_nodes().node_path(list_owner).unwrap();
            let last_line = fragmenter
                .lines
                .iter()
                .rposition(|line| {
                    flow.positions()[line.start]
                        .block_child_path()
                        .starts_with(path)
                })
                .unwrap();
            assert!(fragmenter.lines[last_line].keep_with_next);
        }
    }

    fn positive(raw: i64) -> PositiveLength {
        PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap()
    }

    fn typed_style_receipt(
        block: BasicStyleBlockKind,
        declarations: Vec<wire::WireDeclaration>,
    ) -> MachineBlockComputedStyleReceipt {
        let span = wire::WireSourceSpan {
            source_id: 0,
            start_byte: 0,
            end_byte: 0,
        };
        let (wire_block, images) = match block {
            BasicStyleBlockKind::Paragraph => (
                wire::WireBlock::Paragraph {
                    node_id: 1,
                    span,
                    classes: vec![],
                    children: vec![],
                },
                vec![],
            ),
            BasicStyleBlockKind::Figure => (
                wire::WireBlock::Figure {
                    node_id: 1,
                    span,
                    classes: vec![],
                    image_id: 0,
                    alt: "fixture".to_owned(),
                    caption: vec![],
                },
                vec![wire::WireImage {
                    image_id: 0,
                    uri: "fixture.png".to_owned(),
                    expected_sha256: None,
                }],
            ),
            _ => panic!("typed style fixture supports paragraph and figure"),
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
            text_buffers: vec![],
            document: wire::WireDocument {
                node_id: 0,
                blocks: vec![wire_block],
                footnotes: vec![],
            },
            style_sheet: wire::WireStyleSheet {
                rules: vec![wire::WireStyleRule {
                    style_id: "typed-style".to_owned(),
                    extends: None,
                    selector: block.as_str().to_owned(),
                    source_order: 0,
                    declarations,
                }],
            },
            page_masters: wire::WirePageMasterSet {
                default_master_id: "default".to_owned(),
                masters: vec![wire::WirePageMaster {
                    master_id: "default".to_owned(),
                    width: 1_000,
                    height: 1_000,
                    body: wire::WireRect {
                        x: 0,
                        y: 0,
                        width: 1_000,
                        height: 1_000,
                    },
                    header: None,
                    footer: None,
                    footnote: None,
                }],
                selection_rules: vec![],
            },
            resources: wire::WireResourceCatalog {
                font_faces: vec![],
                images,
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
        package.compute_block_style(NodeId::new(1), None).unwrap()
    }

    fn typed_declaration(
        name: wire::WireDeclarationName,
        value: wire::WireStyleValue,
    ) -> wire::WireDeclaration {
        wire::WireDeclaration {
            name,
            value,
            important: false,
        }
    }

    #[test]
    fn typed_style_consumers_apply_spacing_indents_center_rounding_rtl_and_page_split() {
        let receipt = typed_style_receipt(
            BasicStyleBlockKind::Paragraph,
            vec![
                typed_declaration(
                    wire::WireDeclarationName::SpaceBefore,
                    wire::WireStyleValue::Length { value: 5 },
                ),
                typed_declaration(
                    wire::WireDeclarationName::SpaceAfter,
                    wire::WireStyleValue::Length { value: 6 },
                ),
                typed_declaration(
                    wire::WireDeclarationName::StartIndent,
                    wire::WireStyleValue::Length { value: 10 },
                ),
                typed_declaration(
                    wire::WireDeclarationName::EndIndent,
                    wire::WireStyleValue::Length { value: 10 },
                ),
                typed_declaration(
                    wire::WireDeclarationName::TextAlign,
                    wire::WireStyleValue::Keyword {
                        value: "center".to_owned(),
                    },
                ),
                typed_declaration(
                    wire::WireDeclarationName::KeepWithNext,
                    wire::WireStyleValue::Boolean { value: true },
                ),
            ],
        );
        let input = TypedBlockLayoutInput::new(
            positive(101),
            positive(20),
            positive(20),
            positive(25),
            positive(100),
            NonNegativeLength::new(Length::from_raw(7).unwrap()).unwrap(),
            false,
            false,
            BidiLevel::RTL,
        );
        let selected = consume_typed_block_style(&receipt, input).unwrap();
        assert_eq!(selected.available_inline_size().get().raw(), 81);
        assert_eq!(selected.logical_start_alignment_space().get().raw(), 30);
        assert_eq!(selected.logical_end_alignment_space().get().raw(), 31);
        assert_eq!(selected.physical_left_inset().get().raw(), 41);
        assert!(selected.page_break_before());
        assert_eq!(selected.effective_space_before(), NonNegativeLength::ZERO);
        assert_eq!(selected.effective_space_after().get().raw(), 6);
        assert!(selected.keep_with_next());
    }

    #[test]
    fn typed_style_consumers_use_figure_width_and_caption_keep_without_reinterpretation() {
        let receipt = typed_style_receipt(
            BasicStyleBlockKind::Figure,
            vec![
                typed_declaration(
                    wire::WireDeclarationName::StartIndent,
                    wire::WireStyleValue::Length { value: 10 },
                ),
                typed_declaration(
                    wire::WireDeclarationName::EndIndent,
                    wire::WireStyleValue::Length { value: 20 },
                ),
                typed_declaration(
                    wire::WireDeclarationName::Width,
                    wire::WireStyleValue::Length { value: 30 },
                ),
                typed_declaration(
                    wire::WireDeclarationName::KeepCaption,
                    wire::WireStyleValue::Boolean { value: false },
                ),
            ],
        );
        let selected = consume_typed_block_style(
            &receipt,
            TypedBlockLayoutInput::new(
                positive(100),
                positive(99),
                positive(20),
                positive(100),
                positive(100),
                NonNegativeLength::ZERO,
                true,
                true,
                BidiLevel::LTR,
            ),
        )
        .unwrap();
        assert_eq!(selected.available_inline_size().get().raw(), 70);
        assert_eq!(selected.content_inline_size().get().raw(), 30);
        assert_eq!(selected.physical_left_inset().get().raw(), 10);
        assert!(!selected.keep_caption());
        assert_eq!(selected.effective_space_after(), NonNegativeLength::ZERO);

        let auto = typed_style_receipt(BasicStyleBlockKind::Figure, vec![]);
        assert_eq!(
            consume_typed_block_style(
                &auto,
                TypedBlockLayoutInput::new(
                    positive(100),
                    positive(10),
                    positive(10),
                    positive(100),
                    positive(100),
                    NonNegativeLength::ZERO,
                    true,
                    true,
                    BidiLevel::LTR,
                ),
            ),
            Err(TypedStyleConsumerError::FigureWidthRequired)
        );
    }

    #[test]
    fn machine_figure_height_uses_checked_pixel_aspect_ratio_ties_to_even() {
        let height = scale_figure_height(
            positive(5),
            NonZeroU32::new(2).unwrap(),
            NonZeroU32::new(3).unwrap(),
        )
        .unwrap();
        assert_eq!(height.get().raw(), 8);
        let half = scale_figure_height(
            positive(3),
            NonZeroU32::new(2).unwrap(),
            NonZeroU32::new(1).unwrap(),
        )
        .unwrap();
        assert_eq!(half.get().raw(), 2);
        assert!(scale_figure_height(
            positive(1),
            NonZeroU32::new(3).unwrap(),
            NonZeroU32::new(1).unwrap(),
        )
        .is_none());
    }

    #[test]
    fn typed_style_consumers_production_path_has_no_new_property_name_comparisons() {
        let production = include_str!("lib.rs").split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            "\"space_before\"",
            "\"space_after\"",
            "\"start_indent\"",
            "\"end_indent\"",
            "\"text_align\"",
            "\"width\"",
            "\"keep_with_next\"",
            "\"keep_caption\"",
        ] {
            assert!(!production.contains(forbidden), "found {forbidden}");
        }
    }
}
