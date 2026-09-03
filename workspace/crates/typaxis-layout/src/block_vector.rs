use typaxis_core::{
    push_jcs_string, sha256, ImageResourceId, Length, M4EffectiveResourceLimits, NodeId,
    NonNegativeLength, PageName, PositiveLength, SourceSpan,
};
use typaxis_layout_contract::{FlowId, MathVectorFlowId};
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_style::MachineTextAlign;
use typaxis_syntax::{
    PrecomposedVectorKind, StagingM4PageGeometry, StagingPrecomposedVectorProfileAuthorization,
    ValidatedStagingSemanticPackage,
};

use crate::{
    semantic_container::project_staging_precomposed_vector_parent_flows,
    PrecomposedVectorPlacementInput, StagingMathVectorFlowError, StagingMathVectorFlowRegistry,
    StagingSemanticContainerFlowItemKind, StagingSemanticContainerFlowRegistry,
    ValidatedPrecomposedVectorBindings,
};

pub const PRECOMPOSED_VECTOR_BLOCK_PREPARATION_ALGORITHM: &str =
    "typaxis.precomposed-vector-block-preparation/1";

#[cfg(any(test, feature = "staging-fixtures"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingPrecomposedVectorBlockFixtureCase {
    Default,
    AlignmentStart,
    AlignmentCenter,
    AlignmentEnd,
    NumberShort,
    NumberCollision,
    NarrowInnerFrame,
    ShortBody,
    FigureCaption,
    FigureCaptionSplit,
    KeepWithNext,
    ForcedPageBreak,
    NamedPage,
    MixedNativeMath,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingPreparedVectorBlockKind {
    VectorFigure,
    MathVectorBlock,
}

impl StagingPreparedVectorBlockKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VectorFigure => "vector_figure",
            Self::MathVectorBlock => "math_vector_block",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPreparedVectorFollowingSibling {
    owner: NodeId,
    kind: StagingSemanticContainerFlowItemKind,
    parent_position: u32,
}

impl StagingPreparedVectorFollowingSibling {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn kind(&self) -> StagingSemanticContainerFlowItemKind {
        self.kind
    }

    pub const fn parent_position(&self) -> u32 {
        self.parent_position
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPreparedMathVectorBlockFlow {
    flow_id: MathVectorFlowId,
    flow_fingerprint: [u8; 32],
}

impl StagingPreparedMathVectorBlockFlow {
    pub const fn flow_id(&self) -> MathVectorFlowId {
        self.flow_id
    }

    pub const fn flow_fingerprint(&self) -> [u8; 32] {
        self.flow_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPreparedVectorEquationNumber {
    owner: NodeId,
    source_span: SourceSpan,
    shape_fingerprint: [u8; 32],
    minimum_gap: PositiveLength,
    width: PositiveLength,
    height: PositiveLength,
    left: Length,
    top_offset: NonNegativeLength,
}

impl StagingPreparedVectorEquationNumber {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }

    pub const fn shape_fingerprint(&self) -> [u8; 32] {
        self.shape_fingerprint
    }

    pub const fn minimum_gap(&self) -> PositiveLength {
        self.minimum_gap
    }

    pub const fn width(&self) -> PositiveLength {
        self.width
    }

    pub const fn height(&self) -> PositiveLength {
        self.height
    }

    pub const fn left(&self) -> Length {
        self.left
    }

    pub const fn top_offset(&self) -> NonNegativeLength {
        self.top_offset
    }
}

/// Horizontal and intrinsic block geometry sealed before pagination.
///
/// The formula viewport remains producer-sized. Pagination may only choose a
/// page and a block top; it cannot rewrite any value in this record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPreparedVectorBlock {
    block_ordinal: u32,
    owner: NodeId,
    source_span: SourceSpan,
    kind: StagingPreparedVectorBlockKind,
    image_id: ImageResourceId,
    binding_fingerprint: [u8; 32],
    style_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    parent_flow_id: FlowId,
    parent_position: u32,
    parent_item_kind: StagingSemanticContainerFlowItemKind,
    start_indent: NonNegativeLength,
    end_indent: NonNegativeLength,
    text_align: MachineTextAlign,
    inner_frame_left: Length,
    inner_frame_width: PositiveLength,
    viewport_left: Length,
    viewport_width: PositiveLength,
    viewport_height: PositiveLength,
    viewport_top_offset: NonNegativeLength,
    content_height: PositiveLength,
    origin_x: Option<Length>,
    baseline: Option<NonNegativeLength>,
    scale: i32,
    space_before: NonNegativeLength,
    space_after: NonNegativeLength,
    page_name: Option<PageName>,
    keep_with_next: bool,
    keep_caption: bool,
    forced_page_break_before: bool,
    following_sibling: Option<StagingPreparedVectorFollowingSibling>,
    caption_flow_id: Option<FlowId>,
    caption_owners: Vec<NodeId>,
    math_flow: Option<StagingPreparedMathVectorBlockFlow>,
    equation_number: Option<StagingPreparedVectorEquationNumber>,
    fingerprint: [u8; 32],
}

impl StagingPreparedVectorBlock {
    pub const fn block_ordinal(&self) -> u32 {
        self.block_ordinal
    }

    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }

    pub const fn kind(&self) -> StagingPreparedVectorBlockKind {
        self.kind
    }

    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }

    pub const fn binding_fingerprint(&self) -> [u8; 32] {
        self.binding_fingerprint
    }

    pub const fn style_fingerprint(&self) -> [u8; 32] {
        self.style_fingerprint
    }

    pub const fn layout_epoch_fingerprint(&self) -> [u8; 32] {
        self.layout_epoch_fingerprint
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

    pub const fn start_indent(&self) -> NonNegativeLength {
        self.start_indent
    }

    pub const fn end_indent(&self) -> NonNegativeLength {
        self.end_indent
    }

    pub const fn text_align(&self) -> MachineTextAlign {
        self.text_align
    }

    pub const fn inner_frame_left(&self) -> Length {
        self.inner_frame_left
    }

    pub const fn inner_frame_width(&self) -> PositiveLength {
        self.inner_frame_width
    }

    pub const fn viewport_left(&self) -> Length {
        self.viewport_left
    }

    pub const fn viewport_width(&self) -> PositiveLength {
        self.viewport_width
    }

    pub const fn viewport_height(&self) -> PositiveLength {
        self.viewport_height
    }

    pub const fn viewport_top_offset(&self) -> NonNegativeLength {
        self.viewport_top_offset
    }

    pub const fn content_height(&self) -> PositiveLength {
        self.content_height
    }

    pub const fn origin_x(&self) -> Option<Length> {
        self.origin_x
    }

    pub const fn baseline(&self) -> Option<NonNegativeLength> {
        self.baseline
    }

    pub const fn scale_raw(&self) -> i32 {
        self.scale
    }

    pub const fn space_before(&self) -> NonNegativeLength {
        self.space_before
    }

    pub const fn space_after(&self) -> NonNegativeLength {
        self.space_after
    }

    pub const fn page_name(&self) -> Option<&PageName> {
        self.page_name.as_ref()
    }

    pub const fn keep_with_next(&self) -> bool {
        self.keep_with_next
    }

    pub const fn keep_caption(&self) -> bool {
        self.keep_caption
    }

    pub const fn forced_page_break_before(&self) -> bool {
        self.forced_page_break_before
    }

    pub const fn following_sibling(&self) -> Option<&StagingPreparedVectorFollowingSibling> {
        self.following_sibling.as_ref()
    }

    pub const fn caption_flow_id(&self) -> Option<FlowId> {
        self.caption_flow_id
    }

    pub fn caption_owners(&self) -> &[NodeId] {
        &self.caption_owners
    }

    pub const fn math_flow(&self) -> Option<&StagingPreparedMathVectorBlockFlow> {
        self.math_flow.as_ref()
    }

    pub const fn equation_number(&self) -> Option<&StagingPreparedVectorEquationNumber> {
        self.equation_number.as_ref()
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPrecomposedVectorBlockLayoutReceipt {
    package_sha256: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    binding_set_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    parent_flow_registry_fingerprint: [u8; 32],
    math_flow_registry_fingerprint: [u8; 32],
    page_geometry_fingerprint: [u8; 32],
    block_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingPrecomposedVectorBlockLayoutReceipt {
    pub const fn algorithm(&self) -> &'static str {
        PRECOMPOSED_VECTOR_BLOCK_PREPARATION_ALGORITHM
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }

    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }

    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }

    pub const fn admitted_fingerprint(&self) -> [u8; 32] {
        self.admitted_fingerprint
    }

    pub const fn binding_set_fingerprint(&self) -> [u8; 32] {
        self.binding_set_fingerprint
    }

    pub const fn layout_epoch_fingerprint(&self) -> [u8; 32] {
        self.layout_epoch_fingerprint
    }

    pub const fn parent_flow_registry_fingerprint(&self) -> [u8; 32] {
        self.parent_flow_registry_fingerprint
    }

    pub const fn math_flow_registry_fingerprint(&self) -> [u8; 32] {
        self.math_flow_registry_fingerprint
    }

    pub const fn page_geometry_fingerprint(&self) -> [u8; 32] {
        self.page_geometry_fingerprint
    }

    pub const fn block_count(&self) -> u32 {
        self.block_count
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPrecomposedVectorBlockLayout {
    blocks: Vec<StagingPreparedVectorBlock>,
    page_geometry: StagingM4PageGeometry,
    receipt: StagingPrecomposedVectorBlockLayoutReceipt,
}

impl StagingPrecomposedVectorBlockLayout {
    pub fn blocks(&self) -> &[StagingPreparedVectorBlock] {
        &self.blocks
    }

    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        &self.page_geometry
    }

    pub const fn receipt(&self) -> &StagingPrecomposedVectorBlockLayoutReceipt {
        &self.receipt
    }

    #[doc(hidden)]
    pub fn integrity_matches(&self) -> bool {
        let canonical = encode_layout_receipt(
            self.receipt.package_sha256,
            self.receipt.profile_fingerprint,
            self.receipt.limits_fingerprint,
            self.receipt.admitted_fingerprint,
            self.receipt.binding_set_fingerprint,
            self.receipt.layout_epoch_fingerprint,
            self.receipt.parent_flow_registry_fingerprint,
            self.receipt.math_flow_registry_fingerprint,
            &self.page_geometry,
            &self.blocks,
        );
        usize::try_from(self.receipt.block_count) == Ok(self.blocks.len())
            && self.receipt.page_geometry_fingerprint == self.page_geometry.fingerprint()
            && self.receipt.canonical_jcs == canonical
            && self.receipt.fingerprint == sha256(canonical.as_bytes())
            && self.blocks.iter().enumerate().all(|(index, block)| {
                usize::try_from(block.block_ordinal) == Ok(index)
                    && block.layout_epoch_fingerprint == self.receipt.layout_epoch_fingerprint
                    && block.scale > 0
                    && block.fingerprint == sha256(encode_block(block).as_bytes())
                    && block_geometry_is_closed(block, self.page_geometry.body())
            })
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingPrecomposedVectorProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        bindings: &ValidatedPrecomposedVectorBindings,
        math_flows: &StagingMathVectorFlowRegistry,
    ) -> Result<(), StagingPrecomposedVectorBlockLayoutError> {
        let expected = build_staging_precomposed_vector_blocks(
            package, profile, limits, admitted, bindings, math_flows,
        )?;
        if self != &expected || !self.integrity_matches() {
            return Err(StagingPrecomposedVectorBlockLayoutError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingPrecomposedVectorBlockLayoutError {
    BindingMismatch,
    FlowMismatch(NodeId),
    InvalidGeometry(NodeId, SourceSpan),
    UnsupportedPage(NodeId, SourceSpan),
    BlockLimit,
    ArithmeticOverflow,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingPrecomposedVectorBlockLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BindingMismatch => {
                formatter.write_str("I9190: vector block binding set mismatch")
            }
            Self::FlowMismatch(owner) => write!(
                formatter,
                "I9190: vector block parent flow mismatch at node {}",
                owner.get()
            ),
            Self::InvalidGeometry(owner, span) => write!(
                formatter,
                "L5100: vector block {} at source {}:{}..{} does not fit its inner frame",
                owner.get(),
                span.source_id().get(),
                span.start_byte().get(),
                span.end_byte().get()
            ),
            Self::UnsupportedPage(owner, span) => write!(
                formatter,
                "L5100: vector block {} at source {}:{}..{} selects an unavailable page",
                owner.get(),
                span.source_id().get(),
                span.start_byte().get(),
                span.end_byte().get()
            ),
            Self::BlockLimit => formatter.write_str("L5110: vector block count limit exceeded"),
            Self::ArithmeticOverflow => {
                formatter.write_str("L5100: vector block geometry arithmetic overflow")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5111: vector block preparation allocation failed")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: vector block preparation receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingPrecomposedVectorBlockLayoutError {}

pub fn prepare_staging_precomposed_vector_blocks(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
    math_flows: &StagingMathVectorFlowRegistry,
) -> Result<StagingPrecomposedVectorBlockLayout, StagingPrecomposedVectorBlockLayoutError> {
    let layout = build_staging_precomposed_vector_blocks(
        package, profile, limits, admitted, bindings, math_flows,
    )?;
    if !layout.integrity_matches() {
        return Err(StagingPrecomposedVectorBlockLayoutError::ReceiptMismatch);
    }
    Ok(layout)
}

fn build_staging_precomposed_vector_blocks(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
    math_flows: &StagingMathVectorFlowRegistry,
) -> Result<StagingPrecomposedVectorBlockLayout, StagingPrecomposedVectorBlockLayoutError> {
    bindings
        .verify(package, profile, limits, admitted)
        .map_err(|_| StagingPrecomposedVectorBlockLayoutError::BindingMismatch)?;
    math_flows
        .verify(package, profile, limits, admitted, bindings)
        .map_err(map_math_flow_error)?;
    let parent_registry = project_staging_precomposed_vector_parent_flows(package, profile, limits)
        .map_err(|_| StagingPrecomposedVectorBlockLayoutError::ReceiptMismatch)?;
    if math_flows.receipt().parent_flow_registry_fingerprint()
        != parent_registry.receipt().fingerprint()
    {
        return Err(StagingPrecomposedVectorBlockLayoutError::ReceiptMismatch);
    }

    let block_count = bindings
        .receipts()
        .iter()
        .filter(|receipt| {
            matches!(
                receipt.kind(),
                PrecomposedVectorKind::VectorFigure | PrecomposedVectorKind::MathVectorBlock
            )
        })
        .count();
    if u64::try_from(block_count).map_or(true, |count| count > limits.base().get().max_ast_nodes) {
        return Err(StagingPrecomposedVectorBlockLayoutError::BlockLimit);
    }
    let block_count_u32 = u32::try_from(block_count)
        .map_err(|_| StagingPrecomposedVectorBlockLayoutError::BlockLimit)?;
    let mut blocks = Vec::new();
    blocks
        .try_reserve_exact(block_count)
        .map_err(|_| StagingPrecomposedVectorBlockLayoutError::AllocationFailure)?;
    for receipt in bindings.receipts().iter().filter(|receipt| {
        matches!(
            receipt.kind(),
            PrecomposedVectorKind::VectorFigure | PrecomposedVectorKind::MathVectorBlock
        )
    }) {
        let owner = receipt.node_id();
        let source_span = receipt.owner_source_span();
        let metrics = package
            .precomposed_vector_metrics_for(owner)
            .ok_or(StagingPrecomposedVectorBlockLayoutError::BindingMismatch)?;
        let style = package
            .precomposed_vector_style(owner)
            .ok_or(StagingPrecomposedVectorBlockLayoutError::BindingMismatch)?;
        package
            .verify_precomposed_vector_style(style)
            .map_err(|_| StagingPrecomposedVectorBlockLayoutError::BindingMismatch)?;
        if metrics.fingerprint() != receipt.metrics_fingerprint()
            || style.fingerprint()
                != match receipt.placement() {
                    PrecomposedVectorPlacementInput::VectorFigure(value) => {
                        value.style().fingerprint()
                    }
                    PrecomposedVectorPlacementInput::MathVectorBlock(value) => {
                        value.style().fingerprint()
                    }
                    PrecomposedVectorPlacementInput::Inline(_) => {
                        return Err(StagingPrecomposedVectorBlockLayoutError::BindingMismatch)
                    }
                }
        {
            return Err(StagingPrecomposedVectorBlockLayoutError::BindingMismatch);
        }
        let parent = parent_item(owner, receipt.kind(), &parent_registry)?;
        let body = profile.page_geometry().body();
        let (style_input, viewport_width, viewport_height, scale, origin_x, baseline) =
            match receipt.placement() {
                PrecomposedVectorPlacementInput::VectorFigure(value) => (
                    BlockStyleRef::Figure(value.style()),
                    value.viewport_width(),
                    value.viewport_height(),
                    value.scale().get().raw(),
                    None,
                    None,
                ),
                PrecomposedVectorPlacementInput::MathVectorBlock(value) => (
                    BlockStyleRef::Math(value.style()),
                    value.metrics().viewport_width(),
                    value.metrics().viewport_height(),
                    value.scale().get().raw(),
                    Some(value.metrics().origin_x()),
                    Some(value.metrics().baseline()),
                ),
                PrecomposedVectorPlacementInput::Inline(_) => {
                    return Err(StagingPrecomposedVectorBlockLayoutError::BindingMismatch)
                }
            };
        if style_input
            .page_name()
            .is_some_and(|page| page.as_str() != profile.page_geometry().master_id().as_str())
        {
            return Err(StagingPrecomposedVectorBlockLayoutError::UnsupportedPage(
                owner,
                source_span,
            ));
        }
        let inner_frame_width = body
            .width()
            .get()
            .checked_sub(style_input.start_indent().get())
            .and_then(|value| value.checked_sub(style_input.end_indent().get()))
            .and_then(PositiveLength::new)
            .ok_or(StagingPrecomposedVectorBlockLayoutError::InvalidGeometry(
                owner,
                source_span,
            ))?;
        if viewport_width.get().raw() > inner_frame_width.get().raw() {
            return Err(StagingPrecomposedVectorBlockLayoutError::InvalidGeometry(
                owner,
                source_span,
            ));
        }
        let inner_frame_left = body
            .x()
            .checked_add(style_input.start_indent().get())
            .ok_or(StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow)?;
        let slack = inner_frame_width
            .get()
            .checked_sub(viewport_width.get())
            .ok_or(StagingPrecomposedVectorBlockLayoutError::InvalidGeometry(
                owner,
                source_span,
            ))?;
        let align_offset = match style_input.text_align() {
            MachineTextAlign::Start => Length::ZERO,
            MachineTextAlign::Center => length(slack.raw() / 2)?,
            MachineTextAlign::End => slack,
        };
        let viewport_left = inner_frame_left
            .checked_add(align_offset)
            .ok_or(StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow)?;

        let (content_height, viewport_top_offset, equation_number, math_flow) = if receipt.kind()
            == PrecomposedVectorKind::MathVectorBlock
        {
            let flow = math_flows
                .flows()
                .iter()
                .find(|flow| flow.owner() == owner)
                .ok_or(StagingPrecomposedVectorBlockLayoutError::FlowMismatch(
                    owner,
                ))?;
            if flow.parent_flow_id() != parent.flow_id
                || flow.parent_position() != parent.position
                || flow.parent_item_kind() != StagingSemanticContainerFlowItemKind::DisplayMath
            {
                return Err(StagingPrecomposedVectorBlockLayoutError::FlowMismatch(
                    owner,
                ));
            }
            let (content_height, viewport_top_offset, equation_number) = prepare_equation_number(
                owner,
                source_span,
                metrics,
                math_flows,
                inner_frame_left,
                inner_frame_width,
                viewport_left,
                viewport_width,
                viewport_height,
            )?;
            (
                content_height,
                viewport_top_offset,
                equation_number,
                Some(StagingPreparedMathVectorBlockFlow {
                    flow_id: flow.flow_id(),
                    flow_fingerprint: flow.fingerprint(),
                }),
            )
        } else {
            (viewport_height, NonNegativeLength::ZERO, None, None)
        };

        let following_sibling =
            parent
                .following_sibling
                .map(|item| StagingPreparedVectorFollowingSibling {
                    owner: item.owner(),
                    kind: item.kind(),
                    parent_position: item.position(),
                });
        let block_ordinal = u32::try_from(blocks.len())
            .map_err(|_| StagingPrecomposedVectorBlockLayoutError::BlockLimit)?;
        let mut block = StagingPreparedVectorBlock {
            block_ordinal,
            owner,
            source_span,
            kind: match receipt.kind() {
                PrecomposedVectorKind::VectorFigure => StagingPreparedVectorBlockKind::VectorFigure,
                PrecomposedVectorKind::MathVectorBlock => {
                    StagingPreparedVectorBlockKind::MathVectorBlock
                }
                PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::MathVector => {
                    return Err(StagingPrecomposedVectorBlockLayoutError::BindingMismatch)
                }
            },
            image_id: receipt.resource().image_id(),
            binding_fingerprint: receipt.fingerprint(),
            style_fingerprint: style.fingerprint(),
            layout_epoch_fingerprint: bindings.epoch().fingerprint(),
            parent_flow_id: parent.flow_id,
            parent_position: parent.position,
            parent_item_kind: parent.item_kind,
            start_indent: style_input.start_indent(),
            end_indent: style_input.end_indent(),
            text_align: style_input.text_align(),
            inner_frame_left,
            inner_frame_width,
            viewport_left,
            viewport_width,
            viewport_height,
            viewport_top_offset,
            content_height,
            origin_x,
            baseline,
            scale,
            space_before: style_input.space_before(),
            space_after: style_input.space_after(),
            page_name: style_input.page_name().cloned(),
            keep_with_next: style_input.keep_with_next(),
            keep_caption: style_input.keep_caption(),
            forced_page_break_before: parent.forced_page_break_before,
            following_sibling,
            caption_flow_id: parent.caption_flow_id,
            caption_owners: parent.caption_owners,
            math_flow,
            equation_number,
            fingerprint: [0; 32],
        };
        block.fingerprint = sha256(encode_block(&block).as_bytes());
        blocks.push(block);
    }
    if blocks.len() != block_count
        || math_flows.flows().len()
            != blocks
                .iter()
                .filter(|block| block.kind == StagingPreparedVectorBlockKind::MathVectorBlock)
                .count()
    {
        return Err(StagingPrecomposedVectorBlockLayoutError::ReceiptMismatch);
    }

    let page_geometry = profile.page_geometry().clone();
    let canonical_jcs = encode_layout_receipt(
        package.canonical_jcs_sha256(),
        profile.profile_fingerprint(),
        limits.fingerprint(),
        admitted.fingerprint().bytes(),
        bindings.fingerprint(),
        bindings.epoch().fingerprint(),
        parent_registry.receipt().fingerprint(),
        math_flows.receipt().fingerprint(),
        &page_geometry,
        &blocks,
    );
    Ok(StagingPrecomposedVectorBlockLayout {
        blocks,
        page_geometry: page_geometry.clone(),
        receipt: StagingPrecomposedVectorBlockLayoutReceipt {
            package_sha256: package.canonical_jcs_sha256(),
            profile_fingerprint: profile.profile_fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            admitted_fingerprint: admitted.fingerprint().bytes(),
            binding_set_fingerprint: bindings.fingerprint(),
            layout_epoch_fingerprint: bindings.epoch().fingerprint(),
            parent_flow_registry_fingerprint: parent_registry.receipt().fingerprint(),
            math_flow_registry_fingerprint: math_flows.receipt().fingerprint(),
            page_geometry_fingerprint: page_geometry.fingerprint(),
            block_count: block_count_u32,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
    })
}

enum BlockStyleRef<'a> {
    Figure(&'a typaxis_layout_contract::VectorFigureStyleInput),
    Math(&'a typaxis_layout_contract::MathVectorBlockStyleInput),
}

impl BlockStyleRef<'_> {
    fn space_before(&self) -> NonNegativeLength {
        match self {
            Self::Figure(value) => value.space_before(),
            Self::Math(value) => value.space_before(),
        }
    }

    fn space_after(&self) -> NonNegativeLength {
        match self {
            Self::Figure(value) => value.space_after(),
            Self::Math(value) => value.space_after(),
        }
    }

    fn start_indent(&self) -> NonNegativeLength {
        match self {
            Self::Figure(value) => value.start_indent(),
            Self::Math(value) => value.start_indent(),
        }
    }

    fn end_indent(&self) -> NonNegativeLength {
        match self {
            Self::Figure(value) => value.end_indent(),
            Self::Math(value) => value.end_indent(),
        }
    }

    fn text_align(&self) -> MachineTextAlign {
        match self {
            Self::Figure(value) => value.text_align(),
            Self::Math(value) => value.text_align(),
        }
    }

    fn page_name(&self) -> Option<&PageName> {
        match self {
            Self::Figure(value) => value.page_name(),
            Self::Math(value) => value.page_name(),
        }
    }

    fn keep_with_next(&self) -> bool {
        match self {
            Self::Figure(value) => value.keep_with_next(),
            Self::Math(value) => value.keep_with_next(),
        }
    }

    fn keep_caption(&self) -> bool {
        match self {
            Self::Figure(value) => value.keep_caption(),
            Self::Math(_) => false,
        }
    }
}

struct ParentItem<'a> {
    flow_id: FlowId,
    position: u32,
    item_kind: StagingSemanticContainerFlowItemKind,
    forced_page_break_before: bool,
    following_sibling: Option<&'a crate::StagingSemanticContainerFlowItem>,
    caption_flow_id: Option<FlowId>,
    caption_owners: Vec<NodeId>,
}

fn parent_item<'a>(
    owner: NodeId,
    kind: PrecomposedVectorKind,
    registry: &'a StagingSemanticContainerFlowRegistry,
) -> Result<ParentItem<'a>, StagingPrecomposedVectorBlockLayoutError> {
    let expected_kind = match kind {
        PrecomposedVectorKind::VectorFigure => StagingSemanticContainerFlowItemKind::Figure,
        PrecomposedVectorKind::MathVectorBlock => StagingSemanticContainerFlowItemKind::DisplayMath,
        PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::MathVector => {
            return Err(StagingPrecomposedVectorBlockLayoutError::FlowMismatch(
                owner,
            ))
        }
    };
    let mut found = None;
    for flow in registry.flows() {
        for (index, item) in flow.items().iter().enumerate() {
            if item.owner() != owner {
                continue;
            }
            if item.kind() != expected_kind
                || usize::try_from(item.position()) != Ok(index)
                || found.is_some()
            {
                return Err(StagingPrecomposedVectorBlockLayoutError::FlowMismatch(
                    owner,
                ));
            }
            let previous = index
                .checked_sub(1)
                .and_then(|value| flow.items().get(value));
            let following_sibling = index.checked_add(1).and_then(|next| flow.items().get(next));
            let (caption_flow_id, caption_owners) = if kind == PrecomposedVectorKind::VectorFigure {
                let [caption_flow_id] = item.child_flow_ids() else {
                    return Err(StagingPrecomposedVectorBlockLayoutError::FlowMismatch(
                        owner,
                    ));
                };
                let caption_flow = registry.flow(*caption_flow_id).ok_or(
                    StagingPrecomposedVectorBlockLayoutError::FlowMismatch(owner),
                )?;
                if caption_flow.parent_flow_id() != Some(flow.flow_id())
                    || caption_flow.parent_position() != Some(item.position())
                {
                    return Err(StagingPrecomposedVectorBlockLayoutError::FlowMismatch(
                        owner,
                    ));
                }
                let mut caption_owners = Vec::new();
                caption_owners
                    .try_reserve_exact(caption_flow.items().len())
                    .map_err(|_| StagingPrecomposedVectorBlockLayoutError::AllocationFailure)?;
                caption_owners.extend(caption_flow.items().iter().map(|caption| caption.owner()));
                (Some(*caption_flow_id), caption_owners)
            } else {
                if !item.child_flow_ids().is_empty() {
                    return Err(StagingPrecomposedVectorBlockLayoutError::FlowMismatch(
                        owner,
                    ));
                }
                (None, Vec::new())
            };
            found = Some(ParentItem {
                flow_id: flow.flow_id(),
                position: item.position(),
                item_kind: item.kind(),
                forced_page_break_before: previous.is_some_and(|value| {
                    value.kind() == StagingSemanticContainerFlowItemKind::PageBreak
                }),
                following_sibling,
                caption_flow_id,
                caption_owners,
            });
        }
    }
    found.ok_or(StagingPrecomposedVectorBlockLayoutError::FlowMismatch(
        owner,
    ))
}

#[allow(clippy::too_many_arguments)]
fn prepare_equation_number(
    owner: NodeId,
    source_span: SourceSpan,
    metrics: &typaxis_syntax::ValidatedPrecomposedVectorMetrics,
    math_flows: &StagingMathVectorFlowRegistry,
    inner_frame_left: Length,
    inner_frame_width: PositiveLength,
    viewport_left: Length,
    viewport_width: PositiveLength,
    viewport_height: PositiveLength,
) -> Result<
    (
        PositiveLength,
        NonNegativeLength,
        Option<StagingPreparedVectorEquationNumber>,
    ),
    StagingPrecomposedVectorBlockLayoutError,
> {
    let Some(number) = metrics.equation_number() else {
        if math_flows.equation_number_shape(owner).is_some() {
            return Err(StagingPrecomposedVectorBlockLayoutError::FlowMismatch(
                owner,
            ));
        }
        return Ok((viewport_height, NonNegativeLength::ZERO, None));
    };
    let shape = math_flows
        .equation_number_shape(owner)
        .filter(|shape| {
            shape.node_id() == number.node_id()
                && shape.source_span() == number.span()
                && shape.owner() == owner
        })
        .ok_or(StagingPrecomposedVectorBlockLayoutError::FlowMismatch(
            owner,
        ))?;
    let width = shape.width();
    let height = shape.height();
    if width.get().raw() > inner_frame_width.get().raw() {
        return Err(StagingPrecomposedVectorBlockLayoutError::InvalidGeometry(
            owner,
            source_span,
        ));
    }
    let inner_right = inner_frame_left
        .checked_add(inner_frame_width.get())
        .ok_or(StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow)?;
    let number_left = inner_right
        .checked_sub(width.get())
        .ok_or(StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow)?;
    let formula_right = viewport_left
        .checked_add(viewport_width.get())
        .ok_or(StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow)?;
    let required_number_left = formula_right
        .checked_add(number.minimum_gap().get())
        .ok_or(StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow)?;
    if required_number_left.raw() > number_left.raw() {
        return Err(StagingPrecomposedVectorBlockLayoutError::InvalidGeometry(
            owner,
            source_span,
        ));
    }
    let content_height = if viewport_height.get().raw() >= height.get().raw() {
        viewport_height
    } else {
        height
    };
    let viewport_top_offset = centered_top_offset(content_height, viewport_height)?;
    let number_top_offset = centered_top_offset(content_height, height)?;
    Ok((
        content_height,
        viewport_top_offset,
        Some(StagingPreparedVectorEquationNumber {
            owner: number.node_id(),
            source_span: number.span(),
            shape_fingerprint: shape.fingerprint(),
            minimum_gap: number.minimum_gap(),
            width,
            height,
            left: number_left,
            top_offset: number_top_offset,
        }),
    ))
}

fn centered_top_offset(
    block: PositiveLength,
    child: PositiveLength,
) -> Result<NonNegativeLength, StagingPrecomposedVectorBlockLayoutError> {
    let residual = block
        .get()
        .checked_sub(child.get())
        .ok_or(StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow)?;
    let quotient = residual.raw() / 2;
    let rounded = if residual.raw() % 2 != 0 && quotient % 2 != 0 {
        quotient
            .checked_add(1)
            .ok_or(StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow)?
    } else {
        quotient
    };
    nonnegative(rounded)
}

fn block_geometry_is_closed(block: &StagingPreparedVectorBlock, body: typaxis_core::Rect) -> bool {
    let Some(expected_inner_left) = body.x().checked_add(block.start_indent.get()) else {
        return false;
    };
    let Some(expected_inner_width) = body
        .width()
        .get()
        .checked_sub(block.start_indent.get())
        .and_then(|value| value.checked_sub(block.end_indent.get()))
        .and_then(PositiveLength::new)
    else {
        return false;
    };
    let Some(inner_right) = block
        .inner_frame_left
        .checked_add(block.inner_frame_width.get())
    else {
        return false;
    };
    let Some(viewport_right) = block.viewport_left.checked_add(block.viewport_width.get()) else {
        return false;
    };
    let Some(viewport_bottom) = block
        .viewport_top_offset
        .get()
        .checked_add(block.viewport_height.get())
    else {
        return false;
    };
    let Some(slack) = block
        .inner_frame_width
        .get()
        .checked_sub(block.viewport_width.get())
    else {
        return false;
    };
    let align_offset = match block.text_align {
        MachineTextAlign::Start => Length::ZERO,
        MachineTextAlign::Center => {
            let Some(value) = Length::from_raw(slack.raw() / 2) else {
                return false;
            };
            value
        }
        MachineTextAlign::End => slack,
    };
    if block.inner_frame_left != expected_inner_left
        || block.inner_frame_width != expected_inner_width
        || block.inner_frame_left.checked_add(align_offset) != Some(block.viewport_left)
        || block.viewport_left.raw() < block.inner_frame_left.raw()
        || viewport_right.raw() > inner_right.raw()
        || viewport_bottom.raw() > block.content_height.get().raw()
    {
        return false;
    }
    match block.kind {
        StagingPreparedVectorBlockKind::VectorFigure => {
            block.origin_x.is_none()
                && block.baseline.is_none()
                && block.math_flow.is_none()
                && block.equation_number.is_none()
                && block.caption_flow_id.is_some()
                && block.content_height == block.viewport_height
                && block.viewport_top_offset == NonNegativeLength::ZERO
        }
        StagingPreparedVectorBlockKind::MathVectorBlock => {
            if block.origin_x.is_none()
                || block.baseline.is_none()
                || block.math_flow.is_none()
                || block.caption_flow_id.is_some()
                || !block.caption_owners.is_empty()
            {
                return false;
            }
            match &block.equation_number {
                None => {
                    block.content_height == block.viewport_height
                        && block.viewport_top_offset == NonNegativeLength::ZERO
                }
                Some(number) => {
                    let Some(number_right) = number.left.checked_add(number.width.get()) else {
                        return false;
                    };
                    let Some(number_bottom) =
                        number.top_offset.get().checked_add(number.height.get())
                    else {
                        return false;
                    };
                    let Some(required_left) = viewport_right.checked_add(number.minimum_gap.get())
                    else {
                        return false;
                    };
                    let Ok(expected_viewport_top) =
                        centered_top_offset(block.content_height, block.viewport_height)
                    else {
                        return false;
                    };
                    let Ok(expected_number_top) =
                        centered_top_offset(block.content_height, number.height)
                    else {
                        return false;
                    };
                    number.left.raw() >= required_left.raw()
                        && number_right == inner_right
                        && number_bottom.raw() <= block.content_height.get().raw()
                        && block.viewport_top_offset == expected_viewport_top
                        && number.top_offset == expected_number_top
                        && block.content_height.get().raw()
                            == block
                                .viewport_height
                                .get()
                                .raw()
                                .max(number.height.get().raw())
                }
            }
        }
    }
}

fn map_math_flow_error(
    error: StagingMathVectorFlowError,
) -> StagingPrecomposedVectorBlockLayoutError {
    match error {
        StagingMathVectorFlowError::AllocationFailure => {
            StagingPrecomposedVectorBlockLayoutError::AllocationFailure
        }
        _ => StagingPrecomposedVectorBlockLayoutError::BindingMismatch,
    }
}

fn length(raw: i64) -> Result<Length, StagingPrecomposedVectorBlockLayoutError> {
    Length::from_raw(raw).ok_or(StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow)
}

fn nonnegative(raw: i64) -> Result<NonNegativeLength, StagingPrecomposedVectorBlockLayoutError> {
    NonNegativeLength::new(length(raw)?)
        .ok_or(StagingPrecomposedVectorBlockLayoutError::ArithmeticOverflow)
}

fn encode_block(block: &StagingPreparedVectorBlock) -> String {
    let mut output = String::from("{\"baseline\":");
    push_optional_nonnegative(&mut output, block.baseline);
    output.push_str(",\"binding_fingerprint\":");
    push_hash(&mut output, block.binding_fingerprint);
    output.push_str(",\"block_ordinal\":");
    output.push_str(&block.block_ordinal.to_string());
    output.push_str(",\"caption_flow_id\":");
    push_optional_u32(&mut output, block.caption_flow_id.map(FlowId::get));
    output.push_str(",\"caption_owners\":[");
    for (index, owner) in block.caption_owners.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&owner.get().to_string());
    }
    output.push_str("],\"content_height\":");
    output.push_str(&block.content_height.get().raw().to_string());
    output.push_str(",\"end_indent\":");
    output.push_str(&block.end_indent.get().raw().to_string());
    output.push_str(",\"equation_number\":");
    match &block.equation_number {
        Some(number) => {
            output.push_str("{\"height\":");
            output.push_str(&number.height.get().raw().to_string());
            output.push_str(",\"left\":");
            output.push_str(&number.left.raw().to_string());
            output.push_str(",\"minimum_gap\":");
            output.push_str(&number.minimum_gap.get().raw().to_string());
            output.push_str(",\"owner\":");
            output.push_str(&number.owner.get().to_string());
            output.push_str(",\"shape_fingerprint\":");
            push_hash(&mut output, number.shape_fingerprint);
            output.push_str(",\"source_span\":");
            push_source_span(&mut output, number.source_span);
            output.push_str(",\"top_offset\":");
            output.push_str(&number.top_offset.get().raw().to_string());
            output.push_str(",\"width\":");
            output.push_str(&number.width.get().raw().to_string());
            output.push('}');
        }
        None => output.push_str("null"),
    }
    output.push_str(",\"following_sibling\":");
    match &block.following_sibling {
        Some(sibling) => {
            output.push_str("{\"kind\":");
            push_jcs_string(&mut output, sibling.kind.as_str());
            output.push_str(",\"owner\":");
            output.push_str(&sibling.owner.get().to_string());
            output.push_str(",\"parent_position\":");
            output.push_str(&sibling.parent_position.to_string());
            output.push('}');
        }
        None => output.push_str("null"),
    }
    output.push_str(",\"forced_page_break_before\":");
    output.push_str(if block.forced_page_break_before {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"image_id\":");
    output.push_str(&block.image_id.get().to_string());
    output.push_str(",\"inner_frame_left\":");
    output.push_str(&block.inner_frame_left.raw().to_string());
    output.push_str(",\"inner_frame_width\":");
    output.push_str(&block.inner_frame_width.get().raw().to_string());
    output.push_str(",\"keep_caption\":");
    output.push_str(if block.keep_caption { "true" } else { "false" });
    output.push_str(",\"keep_with_next\":");
    output.push_str(if block.keep_with_next {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, block.kind.as_str());
    output.push_str(",\"layout_epoch_fingerprint\":");
    push_hash(&mut output, block.layout_epoch_fingerprint);
    output.push_str(",\"math_flow\":");
    match &block.math_flow {
        Some(flow) => {
            output.push_str("{\"fingerprint\":");
            push_hash(&mut output, flow.flow_fingerprint);
            output.push_str(",\"flow_id\":");
            output.push_str(&flow.flow_id.get().to_string());
            output.push('}');
        }
        None => output.push_str("null"),
    }
    output.push_str(",\"node_id\":");
    output.push_str(&block.owner.get().to_string());
    output.push_str(",\"origin_x\":");
    push_optional_length(&mut output, block.origin_x);
    output.push_str(",\"page\":");
    match &block.page_name {
        Some(page) => push_jcs_string(&mut output, page.as_str()),
        None => output.push_str("null"),
    }
    output.push_str(",\"parent_flow_id\":");
    output.push_str(&block.parent_flow_id.get().to_string());
    output.push_str(",\"parent_item_kind\":");
    push_jcs_string(&mut output, block.parent_item_kind.as_str());
    output.push_str(",\"parent_position\":");
    output.push_str(&block.parent_position.to_string());
    output.push_str(",\"scale\":");
    output.push_str(&block.scale.to_string());
    output.push_str(",\"source_span\":");
    push_source_span(&mut output, block.source_span);
    output.push_str(",\"space_after\":");
    output.push_str(&block.space_after.get().raw().to_string());
    output.push_str(",\"space_before\":");
    output.push_str(&block.space_before.get().raw().to_string());
    output.push_str(",\"start_indent\":");
    output.push_str(&block.start_indent.get().raw().to_string());
    output.push_str(",\"style_fingerprint\":");
    push_hash(&mut output, block.style_fingerprint);
    output.push_str(",\"text_align\":");
    push_jcs_string(&mut output, block.text_align.as_str());
    output.push_str(",\"viewport_height\":");
    output.push_str(&block.viewport_height.get().raw().to_string());
    output.push_str(",\"viewport_left\":");
    output.push_str(&block.viewport_left.raw().to_string());
    output.push_str(",\"viewport_top_offset\":");
    output.push_str(&block.viewport_top_offset.get().raw().to_string());
    output.push_str(",\"viewport_width\":");
    output.push_str(&block.viewport_width.get().raw().to_string());
    output.push('}');
    output
}

#[allow(clippy::too_many_arguments)]
fn encode_layout_receipt(
    package_sha256: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    binding_set_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    parent_flow_registry_fingerprint: [u8; 32],
    math_flow_registry_fingerprint: [u8; 32],
    page_geometry: &StagingM4PageGeometry,
    blocks: &[StagingPreparedVectorBlock],
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted_fingerprint);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, PRECOMPOSED_VECTOR_BLOCK_PREPARATION_ALGORITHM);
    output.push_str(",\"binding_set_fingerprint\":");
    push_hash(&mut output, binding_set_fingerprint);
    output.push_str(",\"blocks\":[");
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&encode_block(block));
    }
    output.push_str("],\"layout_epoch_fingerprint\":");
    push_hash(&mut output, layout_epoch_fingerprint);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    output.push_str(",\"math_flow_registry_fingerprint\":");
    push_hash(&mut output, math_flow_registry_fingerprint);
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, package_sha256);
    output.push_str(",\"page_geometry_fingerprint\":");
    push_hash(&mut output, page_geometry.fingerprint());
    output.push_str(",\"parent_flow_registry_fingerprint\":");
    push_hash(&mut output, parent_flow_registry_fingerprint);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, profile_fingerprint);
    output.push('}');
    output
}

fn push_source_span(output: &mut String, span: SourceSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&span.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&span.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&span.start_byte().get().to_string());
    output.push('}');
}

fn push_optional_u32(output: &mut String, value: Option<u32>) {
    match value {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push_str("null"),
    }
}

fn push_optional_length(output: &mut String, value: Option<Length>) {
    match value {
        Some(value) => output.push_str(&value.raw().to_string()),
        None => output.push_str("null"),
    }
}

fn push_optional_nonnegative(output: &mut String, value: Option<NonNegativeLength>) {
    match value {
        Some(value) => output.push_str(&value.get().raw().to_string()),
        None => output.push_str("null"),
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

#[cfg(any(test, feature = "staging-fixtures"))]
pub struct StagingPrecomposedVectorBlockLayoutFixture {
    pub package: ValidatedStagingSemanticPackage,
    pub profile: StagingPrecomposedVectorProfileAuthorization,
    pub limits: M4EffectiveResourceLimits,
    pub admitted: AdmittedResourceLedger,
    pub bindings: ValidatedPrecomposedVectorBindings,
    pub math_flows: StagingMathVectorFlowRegistry,
    pub layout: StagingPrecomposedVectorBlockLayout,
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_precomposed_vector_block_layout_fixture(
) -> Result<StagingPrecomposedVectorBlockLayoutFixture, Box<dyn std::error::Error>> {
    staging_precomposed_vector_block_layout_fixture_for_case(
        StagingPrecomposedVectorBlockFixtureCase::Default,
    )
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_precomposed_vector_block_layout_fixture_for_case(
    case: StagingPrecomposedVectorBlockFixtureCase,
) -> Result<StagingPrecomposedVectorBlockLayoutFixture, Box<dyn std::error::Error>> {
    let fixture =
        crate::safe_vector::staging_precomposed_vector_binding_fixture_for_block_case(case)?;
    let math_flows = crate::prepare_staging_math_vector_flows(
        &fixture.package,
        &fixture.profile,
        &fixture.limits,
        &fixture.admitted,
        &fixture.bindings,
    )?;
    let layout = prepare_staging_precomposed_vector_blocks(
        &fixture.package,
        &fixture.profile,
        &fixture.limits,
        &fixture.admitted,
        &fixture.bindings,
        &math_flows,
    )?;
    Ok(StagingPrecomposedVectorBlockLayoutFixture {
        package: fixture.package,
        profile: fixture.profile,
        limits: fixture.limits,
        admitted: fixture.admitted,
        bindings: fixture.bindings,
        math_flows,
        layout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepare_case(
        case: StagingPrecomposedVectorBlockFixtureCase,
    ) -> Result<StagingPrecomposedVectorBlockLayout, StagingPrecomposedVectorBlockLayoutError> {
        let fixture =
            crate::safe_vector::staging_precomposed_vector_binding_fixture_for_block_case(case)
                .unwrap();
        let math_flows = crate::prepare_staging_math_vector_flows(
            &fixture.package,
            &fixture.profile,
            &fixture.limits,
            &fixture.admitted,
            &fixture.bindings,
        )
        .unwrap();
        prepare_staging_precomposed_vector_blocks(
            &fixture.package,
            &fixture.profile,
            &fixture.limits,
            &fixture.admitted,
            &fixture.bindings,
            &math_flows,
        )
    }

    #[test]
    fn math_vector_block_layout_keeps_formula_metrics_and_separate_number() {
        let fixture = staging_precomposed_vector_block_layout_fixture().unwrap();
        fixture
            .layout
            .verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
                &fixture.bindings,
                &fixture.math_flows,
            )
            .unwrap();
        let block = fixture
            .layout
            .blocks()
            .iter()
            .find(|block| block.kind() == StagingPreparedVectorBlockKind::MathVectorBlock)
            .unwrap();
        let number = block.equation_number().unwrap();
        assert_eq!(block.owner(), NodeId::new(6));
        assert_eq!(number.owner(), NodeId::new(7));
        assert_eq!(
            block.content_height().get().raw(),
            block
                .viewport_height()
                .get()
                .raw()
                .max(number.height().get().raw())
        );
        assert!(
            block
                .viewport_left()
                .checked_add(block.viewport_width().get())
                .unwrap()
                .checked_add(number.minimum_gap().get())
                .unwrap()
                .raw()
                <= number.left().raw()
        );
        assert!(block.math_flow().is_some());
        assert!(block.caption_flow_id().is_none());
        assert!(block.caption_owners().is_empty());
    }

    #[test]
    fn vector_figure_layout_reuses_caption_flow_without_raster_state() {
        let fixture = staging_precomposed_vector_block_layout_fixture().unwrap();
        let figure = fixture
            .layout
            .blocks()
            .iter()
            .find(|block| block.kind() == StagingPreparedVectorBlockKind::VectorFigure)
            .unwrap();
        assert_eq!(figure.owner(), NodeId::new(5));
        assert_eq!(
            figure.parent_item_kind(),
            StagingSemanticContainerFlowItemKind::Figure
        );
        assert!(figure.caption_flow_id().is_some());
        assert!(figure.caption_owners().is_empty());
        assert!(figure.keep_caption());
        assert!(figure.math_flow().is_none());
        assert!(figure.equation_number().is_none());
        assert_eq!(figure.content_height(), figure.viewport_height());
    }

    #[test]
    fn math_vector_block_layout_half_even_centering_is_checked() {
        assert_eq!(
            centered_top_offset(
                PositiveLength::new(Length::from_raw(5).unwrap()).unwrap(),
                PositiveLength::new(Length::from_raw(4).unwrap()).unwrap(),
            )
            .unwrap()
            .get()
            .raw(),
            0
        );
        assert_eq!(
            centered_top_offset(
                PositiveLength::new(Length::from_raw(7).unwrap()).unwrap(),
                PositiveLength::new(Length::from_raw(4).unwrap()).unwrap(),
            )
            .unwrap()
            .get()
            .raw(),
            2
        );
    }

    #[test]
    fn math_vector_block_layout_maps_ltr_alignment_over_the_full_inner_frame() {
        let cases = [
            (StagingPrecomposedVectorBlockFixtureCase::AlignmentStart, 0),
            (
                StagingPrecomposedVectorBlockFixtureCase::AlignmentCenter,
                4_587_520,
            ),
            (
                StagingPrecomposedVectorBlockFixtureCase::AlignmentEnd,
                9_175_040,
            ),
        ];
        for (case, expected_offset) in cases {
            let fixture = staging_precomposed_vector_block_layout_fixture_for_case(case).unwrap();
            assert_eq!(fixture.layout.blocks().len(), 2);
            for block in fixture.layout.blocks() {
                assert_eq!(block.inner_frame_left().raw(), 1_966_080);
                assert_eq!(block.inner_frame_width().get().raw(), 11_141_120);
                assert_eq!(
                    block.viewport_left().raw(),
                    block.inner_frame_left().raw() + expected_offset
                );
                assert_eq!(block.space_before().get().raw(), 131_072);
                assert_eq!(block.space_after().get().raw(), 196_608);
            }
            let math = &fixture.layout.blocks()[1];
            assert!(math.equation_number().is_none());
            assert_eq!(math.content_height(), math.viewport_height());
            assert_eq!(math.viewport_top_offset(), NonNegativeLength::ZERO);
        }
    }

    #[test]
    fn math_vector_block_layout_centers_short_and_tall_equation_numbers() {
        let tall = staging_precomposed_vector_block_layout_fixture().unwrap();
        let tall_math = &tall.layout.blocks()[1];
        let tall_number = tall_math.equation_number().unwrap();
        assert_eq!(tall_math.viewport_height().get().raw(), 786_432);
        assert_eq!(tall_number.height().get().raw(), 917_504);
        assert_eq!(tall_math.content_height().get().raw(), 917_504);
        assert_eq!(tall_math.viewport_top_offset().get().raw(), 65_536);
        assert_eq!(tall_number.top_offset(), NonNegativeLength::ZERO);

        let short = staging_precomposed_vector_block_layout_fixture_for_case(
            StagingPrecomposedVectorBlockFixtureCase::NumberShort,
        )
        .unwrap();
        let short_math = &short.layout.blocks()[1];
        let short_number = short_math.equation_number().unwrap();
        assert_eq!(short_number.height().get().raw(), 524_288);
        assert_eq!(short_math.content_height(), short_math.viewport_height());
        assert_eq!(short_math.viewport_top_offset(), NonNegativeLength::ZERO);
        assert_eq!(short_number.top_offset().get().raw(), 131_072);
    }

    #[test]
    fn math_vector_block_layout_rejects_number_collision_without_realignment() {
        let error =
            prepare_case(StagingPrecomposedVectorBlockFixtureCase::NumberCollision).unwrap_err();
        assert!(matches!(
            error,
            StagingPrecomposedVectorBlockLayoutError::InvalidGeometry(owner, _)
                if owner == NodeId::new(6)
        ));
        assert!(error.to_string().starts_with("L5100:"));
    }

    #[test]
    fn math_vector_block_layout_rejects_viewport_wider_than_inner_frame() {
        let error =
            prepare_case(StagingPrecomposedVectorBlockFixtureCase::NarrowInnerFrame).unwrap_err();
        assert!(matches!(
            error,
            StagingPrecomposedVectorBlockLayoutError::InvalidGeometry(owner, _)
                if owner == NodeId::new(6)
        ));
    }

    #[test]
    fn math_vector_block_layout_preserves_mixed_native_parent_positions() {
        let fixture = staging_precomposed_vector_block_layout_fixture_for_case(
            StagingPrecomposedVectorBlockFixtureCase::MixedNativeMath,
        )
        .unwrap();
        assert_eq!(fixture.layout.blocks().len(), 2);
        assert_eq!(fixture.layout.blocks()[0].owner(), NodeId::new(3));
        assert_eq!(fixture.layout.blocks()[0].parent_position(), 1);
        assert!(fixture.layout.blocks()[0].equation_number().is_none());
        assert_eq!(
            fixture.layout.blocks()[0]
                .math_flow()
                .unwrap()
                .flow_id()
                .get(),
            0
        );
        assert_eq!(fixture.layout.blocks()[1].owner(), NodeId::new(5));
        assert_eq!(fixture.layout.blocks()[1].parent_position(), 3);
        assert_eq!(
            fixture.layout.blocks()[1]
                .math_flow()
                .unwrap()
                .flow_id()
                .get(),
            1
        );
        assert!(fixture.layout.blocks()[1].equation_number().is_some());
    }
}
