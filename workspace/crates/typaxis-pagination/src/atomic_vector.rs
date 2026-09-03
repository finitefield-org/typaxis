use std::collections::BTreeSet;

use typaxis_core::{
    push_jcs_string, sha256, AffineTransform, ImageResourceId, Length, M4EffectiveResourceLimits,
    NodeId, NonNegativeLength, PageName, PositiveLength, Rect, SourceSpan, Unitless16_16,
    ValidatedResourceLimits,
};
use typaxis_layout::{
    FlowId, MathVectorFlowId, MathVectorFlowTerminal, StagingMathVectorFlowRegistry,
    StagingMathVectorTerminalError, StagingMathVectorTerminalReceiptSet,
    StagingPrecomposedVectorBlockLayout, StagingPreparedVectorBlock,
    StagingPreparedVectorBlockKind, StagingSemanticContainerFlowItemKind,
};
use typaxis_style::MachineTextAlign;

use crate::StagingFigureCaptionBlockInput;

pub const PRECOMPOSED_VECTOR_SELECTED_LAYOUT_ALGORITHM: &str =
    "typaxis.precomposed-vector-layout/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorKeepSuccessorInput {
    owner: NodeId,
    successor_owner: NodeId,
    /// Full kept extent after `owner`, including the boundary spacing and the
    /// external successor's own atomic extent.
    required_extent_after_vector: PositiveLength,
}

impl StagingAtomicVectorKeepSuccessorInput {
    pub const fn new(
        owner: NodeId,
        successor_owner: NodeId,
        required_extent_after_vector: PositiveLength,
    ) -> Self {
        Self {
            owner,
            successor_owner,
            required_extent_after_vector,
        }
    }

    pub const fn owner(self) -> NodeId {
        self.owner
    }

    pub const fn successor_owner(self) -> NodeId {
        self.successor_owner
    }

    pub const fn required_extent_after_vector(self) -> PositiveLength {
        self.required_extent_after_vector
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorBlockPaginationInput {
    initial_consumed_block_size: NonNegativeLength,
    prior_fragment_charge: u64,
    captions: Vec<StagingFigureCaptionBlockInput>,
    keep_successors: Vec<StagingAtomicVectorKeepSuccessorInput>,
    preparation_fingerprint: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingAtomicVectorBlockPaginationInput {
    pub fn new(
        layout: &StagingPrecomposedVectorBlockLayout,
        initial_consumed_block_size: NonNegativeLength,
        prior_fragment_charge: u64,
        mut captions: Vec<StagingFigureCaptionBlockInput>,
        mut keep_successors: Vec<StagingAtomicVectorKeepSuccessorInput>,
    ) -> Result<Self, StagingAtomicVectorBlockLayoutError> {
        if !layout.integrity_matches() {
            return Err(StagingAtomicVectorBlockLayoutError::LayoutMismatch);
        }
        if initial_consumed_block_size.get().raw()
            > layout.page_geometry().body().height().get().raw()
        {
            return Err(StagingAtomicVectorBlockLayoutError::InitialContentExceedsFrame);
        }
        captions.sort_by_key(|caption| caption.owner());
        if let Some(pair) = captions
            .windows(2)
            .find(|pair| pair[0].owner() == pair[1].owner())
        {
            return Err(StagingAtomicVectorBlockLayoutError::DuplicateCaption(
                pair[1].owner(),
            ));
        }
        let expected_caption_count = layout.blocks().iter().try_fold(0usize, |count, block| {
            count
                .checked_add(block.caption_owners().len())
                .ok_or(StagingAtomicVectorBlockLayoutError::AllocationFailure)
        })?;
        let mut expected_captions = Vec::new();
        expected_captions
            .try_reserve_exact(expected_caption_count)
            .map_err(|_| StagingAtomicVectorBlockLayoutError::AllocationFailure)?;
        for block in layout.blocks() {
            expected_captions.extend_from_slice(block.caption_owners());
        }
        expected_captions.sort_unstable();
        for owner in &expected_captions {
            if captions
                .binary_search_by_key(owner, |caption| caption.owner())
                .is_err()
            {
                return Err(StagingAtomicVectorBlockLayoutError::MissingCaption(*owner));
            }
        }
        if let Some(extra) = captions
            .iter()
            .find(|caption| expected_captions.binary_search(&caption.owner()).is_err())
        {
            return Err(StagingAtomicVectorBlockLayoutError::ExtraCaption(
                extra.owner(),
            ));
        }

        keep_successors.sort_by_key(|value| value.owner);
        if let Some(pair) = keep_successors
            .windows(2)
            .find(|pair| pair[0].owner == pair[1].owner)
        {
            return Err(StagingAtomicVectorBlockLayoutError::DuplicateKeepSuccessor(
                pair[1].owner,
            ));
        }
        let mut expected_keep = Vec::new();
        expected_keep
            .try_reserve_exact(layout.blocks().len())
            .map_err(|_| StagingAtomicVectorBlockLayoutError::AllocationFailure)?;
        for block in layout.blocks() {
            if let Some(successor) = external_keep_successor(layout, block) {
                expected_keep.push(successor);
            }
        }
        for (owner, successor) in &expected_keep {
            let supplied = keep_successors
                .binary_search_by_key(owner, |value| value.owner)
                .ok()
                .map(|index| keep_successors[index])
                .ok_or(StagingAtomicVectorBlockLayoutError::MissingKeepSuccessor(
                    *owner,
                ))?;
            if supplied.successor_owner != *successor {
                return Err(StagingAtomicVectorBlockLayoutError::WrongKeepSuccessor(
                    *owner,
                ));
            }
        }
        if let Some(extra) = keep_successors.iter().find(|value| {
            !expected_keep.iter().any(|(owner, successor)| {
                *owner == value.owner && *successor == value.successor_owner
            })
        }) {
            return Err(StagingAtomicVectorBlockLayoutError::ExtraKeepSuccessor(
                extra.owner,
            ));
        }
        let canonical_jcs = encode_pagination_input(
            initial_consumed_block_size,
            prior_fragment_charge,
            &captions,
            &keep_successors,
            layout.receipt().fingerprint(),
        );
        Ok(Self {
            initial_consumed_block_size,
            prior_fragment_charge,
            captions,
            keep_successors,
            preparation_fingerprint: layout.receipt().fingerprint(),
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        })
    }

    pub const fn initial_consumed_block_size(&self) -> NonNegativeLength {
        self.initial_consumed_block_size
    }

    pub const fn prior_fragment_charge(&self) -> u64 {
        self.prior_fragment_charge
    }

    pub fn captions(&self) -> &[StagingFigureCaptionBlockInput] {
        &self.captions
    }

    pub fn keep_successors(&self) -> &[StagingAtomicVectorKeepSuccessorInput] {
        &self.keep_successors
    }

    pub const fn preparation_fingerprint(&self) -> [u8; 32] {
        self.preparation_fingerprint
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    fn integrity_matches(&self) -> bool {
        let canonical = encode_pagination_input(
            self.initial_consumed_block_size,
            self.prior_fragment_charge,
            &self.captions,
            &self.keep_successors,
            self.preparation_fingerprint,
        );
        self.canonical_jcs == canonical && self.fingerprint == sha256(canonical.as_bytes())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingAtomicVectorStructureRole {
    Figure,
    Formula,
    EquationNumber,
    Caption,
}

impl StagingAtomicVectorStructureRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Figure => "figure",
            Self::Formula => "formula",
            Self::EquationNumber => "equation_number",
            Self::Caption => "caption",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorSelectedViewport {
    rect: Rect,
    scale: i32,
    matrix: AffineTransform,
    paint_ordinal: u32,
}

impl StagingAtomicVectorSelectedViewport {
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    pub const fn scale_raw(&self) -> i32 {
        self.scale
    }

    pub const fn matrix(&self) -> AffineTransform {
        self.matrix
    }

    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorMathBaseline {
    pen_origin_x: Length,
    baseline: NonNegativeLength,
    baseline_y: Length,
}

impl StagingAtomicVectorMathBaseline {
    pub const fn pen_origin_x(&self) -> Length {
        self.pen_origin_x
    }

    pub const fn baseline(&self) -> NonNegativeLength {
        self.baseline
    }

    pub const fn baseline_y(&self) -> Length {
        self.baseline_y
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorMathFlow {
    flow_id: MathVectorFlowId,
    flow_fingerprint: [u8; 32],
    terminal: MathVectorFlowTerminal,
    terminal_receipt_fingerprint: [u8; 32],
}

impl StagingAtomicVectorMathFlow {
    pub const fn flow_id(&self) -> MathVectorFlowId {
        self.flow_id
    }

    pub const fn flow_fingerprint(&self) -> [u8; 32] {
        self.flow_fingerprint
    }

    pub const fn terminal(&self) -> MathVectorFlowTerminal {
        self.terminal
    }

    pub const fn terminal_receipt_fingerprint(&self) -> [u8; 32] {
        self.terminal_receipt_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorSelectedEquationNumber {
    owner: NodeId,
    source_span: SourceSpan,
    shape_fingerprint: [u8; 32],
    minimum_gap: PositiveLength,
    rect: Rect,
    paint_ordinal: u32,
}

impl StagingAtomicVectorSelectedEquationNumber {
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

    pub const fn rect(&self) -> Rect {
        self.rect
    }

    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorSelectedCaption {
    owner: NodeId,
    caption_flow_id: FlowId,
    page_index: u32,
    rect: Rect,
    paint_ordinal: u32,
}

impl StagingAtomicVectorSelectedCaption {
    pub const fn owner(&self) -> NodeId {
        self.owner
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

    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorStructureChild {
    owner: NodeId,
    role: StagingAtomicVectorStructureRole,
    page_index: u32,
    rect: Rect,
    paint_ordinal: u32,
}

impl StagingAtomicVectorStructureChild {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn role(&self) -> StagingAtomicVectorStructureRole {
        self.role
    }

    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn rect(&self) -> Rect {
        self.rect
    }

    pub const fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorBlockPlacement {
    block_ordinal: u32,
    page_block_ordinal: u32,
    owner: NodeId,
    source_span: SourceSpan,
    kind: StagingPreparedVectorBlockKind,
    image_id: ImageResourceId,
    binding_fingerprint: [u8; 32],
    style_fingerprint: [u8; 32],
    parent_flow_id: FlowId,
    parent_position: u32,
    start_indent: NonNegativeLength,
    end_indent: NonNegativeLength,
    text_align: MachineTextAlign,
    requested_space_before: NonNegativeLength,
    requested_space_after: NonNegativeLength,
    requested_page: Option<PageName>,
    keep_with_next: bool,
    keep_caption: bool,
    page_index: u32,
    frame_index: u32,
    fragment_ordinal: u32,
    pagination_bounds: Rect,
    paint_bounds: Rect,
    structure_bounds: Rect,
    effective_space_before: NonNegativeLength,
    effective_space_after: NonNegativeLength,
    moved_to_fresh_page: bool,
    forced_page_break_before: bool,
    viewport: StagingAtomicVectorSelectedViewport,
    math_baseline: Option<StagingAtomicVectorMathBaseline>,
    math_flow: Option<StagingAtomicVectorMathFlow>,
    equation_number: Option<StagingAtomicVectorSelectedEquationNumber>,
    captions: Vec<StagingAtomicVectorSelectedCaption>,
    structure_children: Vec<StagingAtomicVectorStructureChild>,
    fingerprint: [u8; 32],
}

impl StagingAtomicVectorBlockPlacement {
    pub const fn block_ordinal(&self) -> u32 {
        self.block_ordinal
    }

    pub const fn page_block_ordinal(&self) -> u32 {
        self.page_block_ordinal
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

    pub const fn parent_flow_id(&self) -> FlowId {
        self.parent_flow_id
    }

    pub const fn parent_position(&self) -> u32 {
        self.parent_position
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

    pub const fn requested_space_before(&self) -> NonNegativeLength {
        self.requested_space_before
    }

    pub const fn requested_space_after(&self) -> NonNegativeLength {
        self.requested_space_after
    }

    pub const fn requested_page(&self) -> Option<&PageName> {
        self.requested_page.as_ref()
    }

    pub const fn keep_with_next(&self) -> bool {
        self.keep_with_next
    }

    pub const fn keep_caption(&self) -> bool {
        self.keep_caption
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

    pub const fn pagination_bounds(&self) -> Rect {
        self.pagination_bounds
    }

    pub const fn paint_bounds(&self) -> Rect {
        self.paint_bounds
    }

    pub const fn structure_bounds(&self) -> Rect {
        self.structure_bounds
    }

    pub const fn effective_space_before(&self) -> NonNegativeLength {
        self.effective_space_before
    }

    pub const fn effective_space_after(&self) -> NonNegativeLength {
        self.effective_space_after
    }

    pub const fn moved_to_fresh_page(&self) -> bool {
        self.moved_to_fresh_page
    }

    pub const fn forced_page_break_before(&self) -> bool {
        self.forced_page_break_before
    }

    pub const fn viewport(&self) -> &StagingAtomicVectorSelectedViewport {
        &self.viewport
    }

    pub const fn math_baseline(&self) -> Option<&StagingAtomicVectorMathBaseline> {
        self.math_baseline.as_ref()
    }

    pub const fn math_flow(&self) -> Option<&StagingAtomicVectorMathFlow> {
        self.math_flow.as_ref()
    }

    pub const fn equation_number(&self) -> Option<&StagingAtomicVectorSelectedEquationNumber> {
        self.equation_number.as_ref()
    }

    pub fn captions(&self) -> &[StagingAtomicVectorSelectedCaption] {
        &self.captions
    }

    pub fn structure_children(&self) -> &[StagingAtomicVectorStructureChild] {
        &self.structure_children
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorPage {
    page_index: u32,
    block_count: u32,
    caption_count: u32,
}

impl StagingAtomicVectorPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub const fn block_count(&self) -> u32 {
        self.block_count
    }

    pub const fn caption_count(&self) -> u32 {
        self.caption_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorBlockSelectedLayoutReceipt {
    package_sha256: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    binding_set_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    preparation_fingerprint: [u8; 32],
    pagination_input_fingerprint: [u8; 32],
    math_flow_registry_fingerprint: [u8; 32],
    math_terminal_set_fingerprint: [u8; 32],
    page_geometry_fingerprint: [u8; 32],
    block_placement_count: u32,
    fragment_charge: u64,
    cumulative_fragment_charge: u64,
    pagination_input_canonical_jcs: String,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingAtomicVectorBlockSelectedLayoutReceipt {
    pub const fn algorithm(&self) -> &'static str {
        PRECOMPOSED_VECTOR_SELECTED_LAYOUT_ALGORITHM
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

    pub const fn preparation_fingerprint(&self) -> [u8; 32] {
        self.preparation_fingerprint
    }

    pub const fn block_placement_count(&self) -> u32 {
        self.block_placement_count
    }

    pub const fn fragment_charge(&self) -> u64 {
        self.fragment_charge
    }

    pub const fn cumulative_fragment_charge(&self) -> u64 {
        self.cumulative_fragment_charge
    }

    pub const fn math_terminal_set_fingerprint(&self) -> [u8; 32] {
        self.math_terminal_set_fingerprint
    }

    pub const fn pagination_input_fingerprint(&self) -> [u8; 32] {
        self.pagination_input_fingerprint
    }

    pub const fn math_flow_registry_fingerprint(&self) -> [u8; 32] {
        self.math_flow_registry_fingerprint
    }

    pub const fn page_geometry_fingerprint(&self) -> [u8; 32] {
        self.page_geometry_fingerprint
    }

    pub fn pagination_input_canonical_jcs(&self) -> &str {
        &self.pagination_input_canonical_jcs
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingAtomicVectorBlockSelectedLayout {
    pages: Vec<StagingAtomicVectorPage>,
    placements: Vec<StagingAtomicVectorBlockPlacement>,
    math_terminals: StagingMathVectorTerminalReceiptSet,
    receipt: StagingAtomicVectorBlockSelectedLayoutReceipt,
}

impl StagingAtomicVectorBlockSelectedLayout {
    pub fn pages(&self) -> &[StagingAtomicVectorPage] {
        &self.pages
    }

    pub fn placements(&self) -> &[StagingAtomicVectorBlockPlacement] {
        &self.placements
    }

    pub const fn math_terminals(&self) -> &StagingMathVectorTerminalReceiptSet {
        &self.math_terminals
    }

    pub const fn receipt(&self) -> &StagingAtomicVectorBlockSelectedLayoutReceipt {
        &self.receipt
    }

    pub fn trace_json(&self, layout: &StagingPrecomposedVectorBlockLayout) -> String {
        let mut output = String::from(
            "{\"contract\":\"typaxis.contract/1.4\",\"coordinate_unit\":\"pdf_point_1_65536\",\"precomposed_vector_block_layout\":",
        );
        if self.receipt.page_geometry_fingerprint == layout.page_geometry().fingerprint()
            && self.receipt.preparation_fingerprint == layout.receipt().fingerprint()
        {
            output.push_str(self.receipt.canonical_jcs());
        } else {
            output.push_str("null");
        }
        output.push('}');
        output
    }

    pub fn verify(
        &self,
        layout: &StagingPrecomposedVectorBlockLayout,
        math_flows: &StagingMathVectorFlowRegistry,
        input: &StagingAtomicVectorBlockPaginationInput,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingAtomicVectorBlockLayoutError> {
        self.math_terminals
            .verify(math_flows)
            .map_err(StagingAtomicVectorBlockLayoutError::Terminal)?;
        let expected = build_selected(layout, math_flows, input, limits)?;
        if self != &expected || !self.integrity_matches(layout, math_flows, input, limits) {
            return Err(StagingAtomicVectorBlockLayoutError::ReceiptMismatch);
        }
        Ok(())
    }

    fn integrity_matches(
        &self,
        layout: &StagingPrecomposedVectorBlockLayout,
        math_flows: &StagingMathVectorFlowRegistry,
        input: &StagingAtomicVectorBlockPaginationInput,
        limits: &M4EffectiveResourceLimits,
    ) -> bool {
        let canonical = encode_selected_layout(
            &self.receipt,
            layout.page_geometry(),
            &self.pages,
            &self.placements,
        );
        layout.integrity_matches()
            && self.math_terminals.verify(math_flows).is_ok()
            && self.receipt.package_sha256 == layout.receipt().package_sha256()
            && self.receipt.profile_fingerprint == layout.receipt().profile_fingerprint()
            && self.receipt.limits_fingerprint == layout.receipt().limits_fingerprint()
            && self.receipt.admitted_fingerprint == layout.receipt().admitted_fingerprint()
            && self.receipt.binding_set_fingerprint == layout.receipt().binding_set_fingerprint()
            && self.receipt.layout_epoch_fingerprint == layout.receipt().layout_epoch_fingerprint()
            && self.receipt.preparation_fingerprint == layout.receipt().fingerprint()
            && input.preparation_fingerprint() == layout.receipt().fingerprint()
            && self.receipt.pagination_input_canonical_jcs == input.canonical_jcs()
            && self.receipt.pagination_input_fingerprint == input.fingerprint()
            && self.receipt.math_flow_registry_fingerprint == math_flows.receipt().fingerprint()
            && self.receipt.math_terminal_set_fingerprint == self.math_terminals.fingerprint()
            && self.receipt.page_geometry_fingerprint == layout.page_geometry().fingerprint()
            && self.receipt.limits_fingerprint == limits.fingerprint()
            && usize::try_from(self.receipt.block_placement_count) == Ok(self.placements.len())
            && u64::try_from(self.placements.len()) == Ok(self.receipt.fragment_charge)
            && u64::try_from(layout.blocks().len()) == Ok(self.receipt.fragment_charge)
            && input
                .prior_fragment_charge()
                .checked_add(self.receipt.fragment_charge)
                == Some(self.receipt.cumulative_fragment_charge)
            && self.receipt.cumulative_fragment_charge <= limits.base().get().max_fragments
            && input.integrity_matches()
            && self.receipt.canonical_jcs == canonical
            && self.receipt.fingerprint == sha256(canonical.as_bytes())
            && pages_are_closed(
                &self.pages,
                &self.placements,
                layout.page_geometry().body(),
                limits.base(),
            )
            && paint_ordinals_are_dense(&self.placements)
            && self.placements.iter().zip(layout.blocks()).enumerate().all(
                |(index, (placement, prepared))| {
                    usize::try_from(placement.block_ordinal) == Ok(index)
                        && placement.fragment_ordinal == placement.block_ordinal
                        && placement.owner == prepared.owner()
                        && placement.kind == prepared.kind()
                        && placement.binding_fingerprint == prepared.binding_fingerprint()
                        && placement.style_fingerprint == prepared.style_fingerprint()
                        && placement.parent_flow_id == prepared.parent_flow_id()
                        && placement.parent_position == prepared.parent_position()
                        && placement.start_indent == prepared.start_indent()
                        && placement.end_indent == prepared.end_indent()
                        && placement.text_align == prepared.text_align()
                        && placement.requested_space_before == prepared.space_before()
                        && placement.requested_space_after == prepared.space_after()
                        && placement.requested_page.as_ref() == prepared.page_name()
                        && placement.keep_with_next == prepared.keep_with_next()
                        && placement.keep_caption == prepared.keep_caption()
                        && placement.image_id == prepared.image_id()
                        && placement.source_span == prepared.source_span()
                        && placement.pagination_bounds.height() == prepared.content_height()
                        && placement.viewport.scale == prepared.scale_raw()
                        && placement.fingerprint == sha256(encode_placement(placement).as_bytes())
                        && placement_geometry_is_closed(
                            placement,
                            prepared,
                            layout.page_geometry().body(),
                            input,
                        )
                },
            )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingAtomicVectorBlockLayoutError {
    LayoutMismatch,
    FlowMismatch,
    InitialContentExceedsFrame,
    MissingCaption(NodeId),
    ExtraCaption(NodeId),
    DuplicateCaption(NodeId),
    MissingKeepSuccessor(NodeId),
    ExtraKeepSuccessor(NodeId),
    DuplicateKeepSuccessor(NodeId),
    WrongKeepSuccessor(NodeId),
    Oversize(NodeId, SourceSpan),
    FragmentLimit,
    PageLimit,
    ArithmeticOverflow,
    AllocationFailure,
    Terminal(StagingMathVectorTerminalError),
    ReceiptMismatch,
}

impl std::fmt::Display for StagingAtomicVectorBlockLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LayoutMismatch => {
                formatter.write_str("I9190: atomic vector block layout mismatch")
            }
            Self::FlowMismatch => formatter.write_str("I9190: atomic vector block flow mismatch"),
            Self::InitialContentExceedsFrame => {
                formatter.write_str("L5100: initial content exceeds the vector block frame")
            }
            Self::MissingCaption(owner) => write!(
                formatter,
                "I9190: missing vector Figure caption measurement for node {}",
                owner.get()
            ),
            Self::ExtraCaption(owner) => write!(
                formatter,
                "I9190: extra vector Figure caption measurement for node {}",
                owner.get()
            ),
            Self::DuplicateCaption(owner) => write!(
                formatter,
                "I9190: duplicate vector Figure caption measurement for node {}",
                owner.get()
            ),
            Self::MissingKeepSuccessor(owner) => write!(
                formatter,
                "I9190: missing keep-with-next successor extent for vector node {}",
                owner.get()
            ),
            Self::ExtraKeepSuccessor(owner) => write!(
                formatter,
                "I9190: extra keep-with-next successor extent for vector node {}",
                owner.get()
            ),
            Self::DuplicateKeepSuccessor(owner) => write!(
                formatter,
                "I9190: duplicate keep-with-next successor extent for vector node {}",
                owner.get()
            ),
            Self::WrongKeepSuccessor(owner) => write!(
                formatter,
                "I9190: wrong keep-with-next successor for vector node {}",
                owner.get()
            ),
            Self::Oversize(owner, span) => write!(
                formatter,
                "L5100: atomic vector block {} at source {}:{}..{} exceeds an empty frame",
                owner.get(),
                span.source_id().get(),
                span.start_byte().get(),
                span.end_byte().get()
            ),
            Self::FragmentLimit => {
                formatter.write_str("L5110: atomic vector block fragment limit exceeded")
            }
            Self::PageLimit => {
                formatter.write_str("L5100: atomic vector block page limit exceeded")
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("L5100: atomic vector block arithmetic overflow")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5111: atomic vector block allocation failed")
            }
            Self::Terminal(error) => std::fmt::Display::fmt(error, formatter),
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: atomic vector selected receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingAtomicVectorBlockLayoutError {}

pub fn paginate_staging_atomic_vector_blocks(
    layout: &StagingPrecomposedVectorBlockLayout,
    math_flows: &StagingMathVectorFlowRegistry,
    input: &StagingAtomicVectorBlockPaginationInput,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingAtomicVectorBlockSelectedLayout, StagingAtomicVectorBlockLayoutError> {
    let selected = build_selected(layout, math_flows, input, limits)?;
    if !selected.integrity_matches(layout, math_flows, input, limits) {
        return Err(StagingAtomicVectorBlockLayoutError::ReceiptMismatch);
    }
    Ok(selected)
}

fn build_selected(
    layout: &StagingPrecomposedVectorBlockLayout,
    math_flows: &StagingMathVectorFlowRegistry,
    input: &StagingAtomicVectorBlockPaginationInput,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingAtomicVectorBlockSelectedLayout, StagingAtomicVectorBlockLayoutError> {
    if !layout.integrity_matches()
        || !input.integrity_matches()
        || input.preparation_fingerprint() != layout.receipt().fingerprint()
        || layout.receipt().limits_fingerprint() != limits.fingerprint()
        || layout.receipt().math_flow_registry_fingerprint() != math_flows.receipt().fingerprint()
        || input.initial_consumed_block_size.get().raw()
            > layout.page_geometry().body().height().get().raw()
    {
        return Err(StagingAtomicVectorBlockLayoutError::LayoutMismatch);
    }
    let fragment_charge = u64::try_from(layout.blocks().len())
        .map_err(|_| StagingAtomicVectorBlockLayoutError::FragmentLimit)?;
    let cumulative_fragment_charge = input
        .prior_fragment_charge
        .checked_add(fragment_charge)
        .ok_or(StagingAtomicVectorBlockLayoutError::FragmentLimit)?;
    if cumulative_fragment_charge > limits.base().get().max_fragments {
        return Err(StagingAtomicVectorBlockLayoutError::FragmentLimit);
    }
    if limits.base().get().max_pages == 0 {
        return Err(StagingAtomicVectorBlockLayoutError::PageLimit);
    }
    let body = layout.page_geometry().body();
    let body_height = body.height().get().raw();
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(layout.blocks().len().saturating_add(1))
        .map_err(|_| StagingAtomicVectorBlockLayoutError::AllocationFailure)?;
    pages.push(StagingAtomicVectorPage {
        page_index: 0,
        block_count: 0,
        caption_count: 0,
    });
    let mut placements = Vec::new();
    placements
        .try_reserve_exact(layout.blocks().len())
        .map_err(|_| StagingAtomicVectorBlockLayoutError::AllocationFailure)?;
    let mut terminal_ledger = math_flows
        .terminal_ledger()
        .map_err(StagingAtomicVectorBlockLayoutError::Terminal)?;
    let mut used = input.initial_consumed_block_size.get().raw();
    let mut pending_space_after = 0i64;
    let mut active_page_name: Option<&str> = None;
    let mut paint_ordinal = 0u32;

    for (index, block) in layout.blocks().iter().enumerate() {
        let caption_inputs = captions_for(block, input)?;
        let caption_total = caption_total(&caption_inputs)?;
        let mut moved_to_fresh_page = false;
        let requested_page_name = block.page_name().map(|page| page.as_str());
        let policy_break =
            block.forced_page_break_before() || requested_page_name != active_page_name;
        if policy_break && used != 0 {
            defer_math_page_move(block, &terminal_ledger)?;
            add_page(&mut pages, limits.base())?;
            used = 0;
            pending_space_after = 0;
            moved_to_fresh_page = true;
        }
        active_page_name = requested_page_name;

        let atomic_height = atomic_primary_height(block, caption_total)?;
        if atomic_height > body_height {
            return Err(StagingAtomicVectorBlockLayoutError::Oversize(
                block.owner(),
                block.source_span(),
            ));
        }
        let mut effective_before = if used == 0 {
            0
        } else {
            pending_space_after
                .checked_add(block.space_before().get().raw())
                .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?
        };
        if block.keep_with_next() {
            let group_height = keep_group_height(layout, index, input)?;
            if group_height > body_height {
                return Err(StagingAtomicVectorBlockLayoutError::Oversize(
                    block.owner(),
                    block.source_span(),
                ));
            }
            let requested = effective_before
                .checked_add(group_height)
                .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
            if requested > body_height - used {
                if used == 0 {
                    return Err(StagingAtomicVectorBlockLayoutError::Oversize(
                        block.owner(),
                        block.source_span(),
                    ));
                }
                defer_math_page_move(block, &terminal_ledger)?;
                add_page(&mut pages, limits.base())?;
                used = 0;
                effective_before = 0;
                moved_to_fresh_page = true;
            }
        }
        let requested = effective_before
            .checked_add(atomic_height)
            .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
        if requested > body_height - used {
            if used == 0 {
                return Err(StagingAtomicVectorBlockLayoutError::Oversize(
                    block.owner(),
                    block.source_span(),
                ));
            }
            defer_math_page_move(block, &terminal_ledger)?;
            add_page(&mut pages, limits.base())?;
            used = 0;
            effective_before = 0;
            moved_to_fresh_page = true;
        }

        let page_index = current_page_index(&pages)?;
        let page_block_ordinal = pages
            .last()
            .ok_or(StagingAtomicVectorBlockLayoutError::PageLimit)?
            .block_count;
        let block_top = page_y(body, used, effective_before)?;
        let pagination_bounds = Rect::new(
            block.inner_frame_left(),
            block_top,
            block.inner_frame_width(),
            block.content_height(),
        );
        let viewport_top = block_top
            .checked_add(block.viewport_top_offset().get())
            .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
        let viewport_rect = Rect::new(
            block.viewport_left(),
            viewport_top,
            block.viewport_width(),
            block.viewport_height(),
        );
        let formula_paint_ordinal = paint_ordinal;
        paint_ordinal = paint_ordinal
            .checked_add(1)
            .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
        let matrix = AffineTransform {
            a: Unitless16_16::from_raw(block.scale_raw()),
            b: Unitless16_16::from_raw(0),
            c: Unitless16_16::from_raw(0),
            d: Unitless16_16::from_raw(block.scale_raw()),
            e: viewport_rect.x(),
            f: viewport_rect.y(),
        };
        let viewport = StagingAtomicVectorSelectedViewport {
            rect: viewport_rect,
            scale: block.scale_raw(),
            matrix,
            paint_ordinal: formula_paint_ordinal,
        };
        let math_baseline = match (block.origin_x(), block.baseline()) {
            (Some(origin_x), Some(baseline)) => Some(StagingAtomicVectorMathBaseline {
                pen_origin_x: viewport_rect
                    .x()
                    .checked_sub(origin_x)
                    .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?,
                baseline,
                baseline_y: viewport_rect
                    .y()
                    .checked_add(baseline.get())
                    .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?,
            }),
            (None, None) => None,
            _ => return Err(StagingAtomicVectorBlockLayoutError::LayoutMismatch),
        };
        let math_flow = block.math_flow().map(|flow| StagingAtomicVectorMathFlow {
            flow_id: flow.flow_id(),
            flow_fingerprint: flow.flow_fingerprint(),
            terminal: MathVectorFlowTerminal::ONE,
            terminal_receipt_fingerprint: [0; 32],
        });
        let equation_number = block
            .equation_number()
            .map(|number| {
                let top = block_top
                    .checked_add(number.top_offset().get())
                    .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
                let selected = StagingAtomicVectorSelectedEquationNumber {
                    owner: number.owner(),
                    source_span: number.source_span(),
                    shape_fingerprint: number.shape_fingerprint(),
                    minimum_gap: number.minimum_gap(),
                    rect: Rect::new(number.left(), top, number.width(), number.height()),
                    paint_ordinal,
                };
                paint_ordinal = paint_ordinal
                    .checked_add(1)
                    .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
                Ok(selected)
            })
            .transpose()?;
        let mut structure_children = Vec::new();
        structure_children
            .try_reserve_exact(1 + usize::from(equation_number.is_some()) + caption_inputs.len())
            .map_err(|_| StagingAtomicVectorBlockLayoutError::AllocationFailure)?;
        structure_children.push(StagingAtomicVectorStructureChild {
            owner: block.owner(),
            role: match block.kind() {
                StagingPreparedVectorBlockKind::VectorFigure => {
                    StagingAtomicVectorStructureRole::Figure
                }
                StagingPreparedVectorBlockKind::MathVectorBlock => {
                    StagingAtomicVectorStructureRole::Formula
                }
            },
            page_index,
            rect: viewport_rect,
            paint_ordinal: formula_paint_ordinal,
        });
        if let Some(number) = &equation_number {
            structure_children.push(StagingAtomicVectorStructureChild {
                owner: number.owner,
                role: StagingAtomicVectorStructureRole::EquationNumber,
                page_index,
                rect: number.rect,
                paint_ordinal: number.paint_ordinal,
            });
        }
        used = used
            .checked_add(effective_before)
            .and_then(|value| value.checked_add(block.content_height().get().raw()))
            .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
        increment_page(&mut pages, page_index, true)?;

        let mut selected_captions = Vec::new();
        selected_captions
            .try_reserve_exact(caption_inputs.len())
            .map_err(|_| StagingAtomicVectorBlockLayoutError::AllocationFailure)?;
        for caption in caption_inputs {
            let height = caption.block_size().get().raw();
            if height > body_height {
                return Err(StagingAtomicVectorBlockLayoutError::Oversize(
                    caption.owner(),
                    block.source_span(),
                ));
            }
            if height > body_height - used {
                add_page(&mut pages, limits.base())?;
                used = 0;
            }
            let caption_page = current_page_index(&pages)?;
            let top = page_y(body, used, 0)?;
            let caption_flow_id = block
                .caption_flow_id()
                .ok_or(StagingAtomicVectorBlockLayoutError::FlowMismatch)?;
            let selected = StagingAtomicVectorSelectedCaption {
                owner: caption.owner(),
                caption_flow_id,
                page_index: caption_page,
                rect: Rect::new(body.x(), top, body.width(), caption.block_size()),
                paint_ordinal,
            };
            paint_ordinal = paint_ordinal
                .checked_add(1)
                .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
            structure_children.push(StagingAtomicVectorStructureChild {
                owner: selected.owner,
                role: StagingAtomicVectorStructureRole::Caption,
                page_index: selected.page_index,
                rect: selected.rect,
                paint_ordinal: selected.paint_ordinal,
            });
            used = used
                .checked_add(height)
                .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
            increment_page(&mut pages, caption_page, false)?;
            selected_captions.push(selected);
        }

        if let Some(flow) = block.math_flow() {
            terminal_ledger
                .consume_selected(flow.flow_id(), block.owner())
                .map_err(StagingAtomicVectorBlockLayoutError::Terminal)?;
        }
        let effective_space_after = if block.following_sibling().is_some() {
            block.space_after()
        } else {
            NonNegativeLength::ZERO
        };
        pending_space_after = effective_space_after.get().raw();
        placements.push(StagingAtomicVectorBlockPlacement {
            block_ordinal: block.block_ordinal(),
            page_block_ordinal,
            owner: block.owner(),
            source_span: block.source_span(),
            kind: block.kind(),
            image_id: block.image_id(),
            binding_fingerprint: block.binding_fingerprint(),
            style_fingerprint: block.style_fingerprint(),
            parent_flow_id: block.parent_flow_id(),
            parent_position: block.parent_position(),
            start_indent: block.start_indent(),
            end_indent: block.end_indent(),
            text_align: block.text_align(),
            requested_space_before: block.space_before(),
            requested_space_after: block.space_after(),
            requested_page: block.page_name().cloned(),
            keep_with_next: block.keep_with_next(),
            keep_caption: block.keep_caption(),
            page_index,
            frame_index: 0,
            fragment_ordinal: block.block_ordinal(),
            pagination_bounds,
            paint_bounds: pagination_bounds,
            structure_bounds: pagination_bounds,
            effective_space_before: nonnegative(effective_before)?,
            effective_space_after,
            moved_to_fresh_page,
            forced_page_break_before: block.forced_page_break_before(),
            viewport,
            math_baseline,
            math_flow,
            equation_number,
            captions: selected_captions,
            structure_children,
            fingerprint: [0; 32],
        });
    }

    let math_terminals = terminal_ledger
        .finish()
        .map_err(StagingAtomicVectorBlockLayoutError::Terminal)?;
    math_terminals
        .verify(math_flows)
        .map_err(StagingAtomicVectorBlockLayoutError::Terminal)?;
    for placement in &mut placements {
        if let Some(flow) = &mut placement.math_flow {
            let terminal = math_terminals
                .receipts()
                .get(
                    usize::try_from(flow.flow_id().get())
                        .map_err(|_| StagingAtomicVectorBlockLayoutError::FlowMismatch)?,
                )
                .filter(|receipt| {
                    receipt.flow_id() == flow.flow_id && receipt.owner() == placement.owner
                })
                .ok_or(StagingAtomicVectorBlockLayoutError::FlowMismatch)?;
            flow.terminal_receipt_fingerprint = terminal.fingerprint();
        }
        placement.fingerprint = sha256(encode_placement(placement).as_bytes());
    }

    let mut receipt = StagingAtomicVectorBlockSelectedLayoutReceipt {
        package_sha256: layout.receipt().package_sha256(),
        profile_fingerprint: layout.receipt().profile_fingerprint(),
        limits_fingerprint: layout.receipt().limits_fingerprint(),
        admitted_fingerprint: layout.receipt().admitted_fingerprint(),
        binding_set_fingerprint: layout.receipt().binding_set_fingerprint(),
        layout_epoch_fingerprint: layout.receipt().layout_epoch_fingerprint(),
        preparation_fingerprint: layout.receipt().fingerprint(),
        pagination_input_fingerprint: input.fingerprint(),
        math_flow_registry_fingerprint: math_flows.receipt().fingerprint(),
        math_terminal_set_fingerprint: math_terminals.fingerprint(),
        page_geometry_fingerprint: layout.page_geometry().fingerprint(),
        block_placement_count: u32::try_from(placements.len())
            .map_err(|_| StagingAtomicVectorBlockLayoutError::FragmentLimit)?,
        fragment_charge,
        cumulative_fragment_charge,
        pagination_input_canonical_jcs: input.canonical_jcs().to_owned(),
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs =
        encode_selected_layout(&receipt, layout.page_geometry(), &pages, &placements);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    Ok(StagingAtomicVectorBlockSelectedLayout {
        pages,
        placements,
        math_terminals,
        receipt,
    })
}

fn external_keep_successor(
    layout: &StagingPrecomposedVectorBlockLayout,
    block: &StagingPreparedVectorBlock,
) -> Option<(NodeId, NodeId)> {
    if !block.keep_with_next() {
        return None;
    }
    let sibling = block.following_sibling()?;
    if sibling.kind() == StagingSemanticContainerFlowItemKind::PageBreak
        || layout.blocks().iter().any(|candidate| {
            candidate.owner() == sibling.owner()
                && candidate.parent_flow_id() == block.parent_flow_id()
                && candidate.parent_position() == sibling.parent_position()
        })
    {
        None
    } else {
        Some((block.owner(), sibling.owner()))
    }
}

fn captions_for(
    block: &StagingPreparedVectorBlock,
    input: &StagingAtomicVectorBlockPaginationInput,
) -> Result<Vec<StagingFigureCaptionBlockInput>, StagingAtomicVectorBlockLayoutError> {
    let mut captions = Vec::new();
    captions
        .try_reserve_exact(block.caption_owners().len())
        .map_err(|_| StagingAtomicVectorBlockLayoutError::AllocationFailure)?;
    for owner in block.caption_owners() {
        let caption = input
            .captions
            .binary_search_by_key(owner, |caption| caption.owner())
            .ok()
            .map(|index| input.captions[index])
            .ok_or(StagingAtomicVectorBlockLayoutError::MissingCaption(*owner))?;
        captions.push(caption);
    }
    Ok(captions)
}

fn caption_total(
    captions: &[StagingFigureCaptionBlockInput],
) -> Result<i64, StagingAtomicVectorBlockLayoutError> {
    captions.iter().try_fold(0i64, |total, caption| {
        total
            .checked_add(caption.block_size().get().raw())
            .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)
    })
}

fn atomic_primary_height(
    block: &StagingPreparedVectorBlock,
    caption_total: i64,
) -> Result<i64, StagingAtomicVectorBlockLayoutError> {
    if block.kind() == StagingPreparedVectorBlockKind::VectorFigure && block.keep_caption() {
        block
            .content_height()
            .get()
            .raw()
            .checked_add(caption_total)
            .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)
    } else {
        Ok(block.content_height().get().raw())
    }
}

fn keep_group_height(
    layout: &StagingPrecomposedVectorBlockLayout,
    start: usize,
    input: &StagingAtomicVectorBlockPaginationInput,
) -> Result<i64, StagingAtomicVectorBlockLayoutError> {
    let mut visited = BTreeSet::new();
    let mut current_index = start;
    let mut extent = 0i64;
    loop {
        let block = layout
            .blocks()
            .get(current_index)
            .ok_or(StagingAtomicVectorBlockLayoutError::LayoutMismatch)?;
        if !visited.insert(block.owner()) {
            return Err(StagingAtomicVectorBlockLayoutError::FlowMismatch);
        }
        let captions = captions_for(block, input)?;
        let captions_height = caption_total(&captions)?;
        // A caption flow lies between its Figure owner and the following
        // sibling. Include it when this boundary is kept, even if
        // `keep_caption` alone would permit the caption to move. For the last
        // block in the group, only `keep_caption` keeps its captions.
        let kept_caption_height = if block.kind() == StagingPreparedVectorBlockKind::VectorFigure
            && (block.keep_caption() || block.keep_with_next())
        {
            captions_height
        } else {
            0
        };
        extent = extent
            .checked_add(block.content_height().get().raw())
            .and_then(|value| value.checked_add(kept_caption_height))
            .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
        if !block.keep_with_next() {
            break;
        }
        let Some(sibling) = block.following_sibling() else {
            break;
        };
        if sibling.kind() == StagingSemanticContainerFlowItemKind::PageBreak {
            break;
        }
        let next = layout.blocks().iter().position(|candidate| {
            candidate.owner() == sibling.owner()
                && candidate.parent_flow_id() == block.parent_flow_id()
                && candidate.parent_position() == sibling.parent_position()
        });
        if let Some(next) = next {
            let following = &layout.blocks()[next];
            if following.forced_page_break_before()
                || following.page_name().map(|page| page.as_str())
                    != block.page_name().map(|page| page.as_str())
            {
                break;
            }
            extent = extent
                .checked_add(block.space_after().get().raw())
                .and_then(|value| value.checked_add(following.space_before().get().raw()))
                .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
            current_index = next;
            continue;
        }
        let supplied = input
            .keep_successors
            .binary_search_by_key(&block.owner(), |value| value.owner)
            .ok()
            .map(|index| input.keep_successors[index])
            .ok_or(StagingAtomicVectorBlockLayoutError::MissingKeepSuccessor(
                block.owner(),
            ))?;
        if supplied.successor_owner != sibling.owner() {
            return Err(StagingAtomicVectorBlockLayoutError::WrongKeepSuccessor(
                block.owner(),
            ));
        }
        extent = extent
            .checked_add(supplied.required_extent_after_vector.get().raw())
            .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
        break;
    }
    Ok(extent)
}

fn defer_math_page_move(
    block: &StagingPreparedVectorBlock,
    ledger: &typaxis_layout::StagingMathVectorTerminalLedger,
) -> Result<(), StagingAtomicVectorBlockLayoutError> {
    if let Some(flow) = block.math_flow() {
        ledger
            .defer_page_move(flow.flow_id(), block.owner())
            .map_err(StagingAtomicVectorBlockLayoutError::Terminal)?;
    }
    Ok(())
}

fn add_page(
    pages: &mut Vec<StagingAtomicVectorPage>,
    limits: &ValidatedResourceLimits,
) -> Result<(), StagingAtomicVectorBlockLayoutError> {
    let page_index =
        u32::try_from(pages.len()).map_err(|_| StagingAtomicVectorBlockLayoutError::PageLimit)?;
    if page_index >= limits.get().max_pages {
        return Err(StagingAtomicVectorBlockLayoutError::PageLimit);
    }
    pages
        .try_reserve(1)
        .map_err(|_| StagingAtomicVectorBlockLayoutError::AllocationFailure)?;
    pages.push(StagingAtomicVectorPage {
        page_index,
        block_count: 0,
        caption_count: 0,
    });
    Ok(())
}

fn current_page_index(
    pages: &[StagingAtomicVectorPage],
) -> Result<u32, StagingAtomicVectorBlockLayoutError> {
    pages
        .last()
        .map(|page| page.page_index)
        .ok_or(StagingAtomicVectorBlockLayoutError::PageLimit)
}

fn increment_page(
    pages: &mut [StagingAtomicVectorPage],
    page_index: u32,
    block: bool,
) -> Result<(), StagingAtomicVectorBlockLayoutError> {
    let page = pages
        .get_mut(
            usize::try_from(page_index)
                .map_err(|_| StagingAtomicVectorBlockLayoutError::PageLimit)?,
        )
        .ok_or(StagingAtomicVectorBlockLayoutError::PageLimit)?;
    let count = if block {
        &mut page.block_count
    } else {
        &mut page.caption_count
    };
    *count = count
        .checked_add(1)
        .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
    Ok(())
}

fn page_y(
    body: Rect,
    used: i64,
    before: i64,
) -> Result<Length, StagingAtomicVectorBlockLayoutError> {
    let offset = used
        .checked_add(before)
        .and_then(Length::from_raw)
        .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)?;
    body.y()
        .checked_add(offset)
        .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)
}

fn nonnegative(raw: i64) -> Result<NonNegativeLength, StagingAtomicVectorBlockLayoutError> {
    Length::from_raw(raw)
        .and_then(NonNegativeLength::new)
        .ok_or(StagingAtomicVectorBlockLayoutError::ArithmeticOverflow)
}

fn pages_are_closed(
    pages: &[StagingAtomicVectorPage],
    placements: &[StagingAtomicVectorBlockPlacement],
    body: Rect,
    limits: &ValidatedResourceLimits,
) -> bool {
    let page_block_total = pages.iter().try_fold(0u64, |total, page| {
        total.checked_add(u64::from(page.block_count))
    });
    let page_caption_total = pages.iter().try_fold(0u64, |total, page| {
        total.checked_add(u64::from(page.caption_count))
    });
    let selected_caption_total = placements.iter().try_fold(0u64, |total, placement| {
        u64::try_from(placement.captions.len())
            .ok()
            .and_then(|count| total.checked_add(count))
    });
    !pages.is_empty()
        && u32::try_from(pages.len()).is_ok_and(|count| count <= limits.get().max_pages)
        && page_block_total == u64::try_from(placements.len()).ok()
        && page_caption_total == selected_caption_total
        && placements.last().map_or(true, |_| {
            pages
                .last()
                .is_some_and(|page| page.block_count > 0 || page.caption_count > 0)
        })
        && pages.iter().enumerate().all(|(index, page)| {
            usize::try_from(page.page_index) == Ok(index)
                && usize::try_from(page.block_count).ok()
                    == Some(
                        placements
                            .iter()
                            .filter(|placement| placement.page_index == page.page_index)
                            .count(),
                    )
                && usize::try_from(page.caption_count).ok()
                    == Some(
                        placements
                            .iter()
                            .flat_map(|placement| &placement.captions)
                            .filter(|caption| caption.page_index == page.page_index)
                            .count(),
                    )
                && placements
                    .iter()
                    .filter(|placement| placement.page_index == page.page_index)
                    .enumerate()
                    .all(|(ordinal, placement)| {
                        usize::try_from(placement.page_block_ordinal) == Ok(ordinal)
                            && rect_is_inside_body(placement.pagination_bounds, body)
                    })
                && placements
                    .iter()
                    .flat_map(|placement| &placement.captions)
                    .filter(|caption| caption.page_index == page.page_index)
                    .all(|caption| rect_is_inside_body(caption.rect, body))
        })
}

fn paint_ordinals_are_dense(placements: &[StagingAtomicVectorBlockPlacement]) -> bool {
    let mut expected = 0u32;
    for placement in placements {
        if placement.viewport.paint_ordinal != expected {
            return false;
        }
        let Some(next) = expected.checked_add(1) else {
            return false;
        };
        expected = next;
        if let Some(number) = &placement.equation_number {
            if number.paint_ordinal != expected {
                return false;
            }
            let Some(next) = expected.checked_add(1) else {
                return false;
            };
            expected = next;
        }
        for caption in &placement.captions {
            if caption.paint_ordinal != expected {
                return false;
            }
            let Some(next) = expected.checked_add(1) else {
                return false;
            };
            expected = next;
        }
    }
    true
}

fn rect_is_inside_body(rect: Rect, body: Rect) -> bool {
    let Some(right) = rect.x().checked_add(rect.width().get()) else {
        return false;
    };
    let Some(bottom) = rect.y().checked_add(rect.height().get()) else {
        return false;
    };
    let Some(body_right) = body.x().checked_add(body.width().get()) else {
        return false;
    };
    let Some(body_bottom) = body.y().checked_add(body.height().get()) else {
        return false;
    };
    rect.x().raw() >= body.x().raw()
        && rect.y().raw() >= body.y().raw()
        && right.raw() <= body_right.raw()
        && bottom.raw() <= body_bottom.raw()
}

fn placement_geometry_is_closed(
    placement: &StagingAtomicVectorBlockPlacement,
    prepared: &StagingPreparedVectorBlock,
    body: Rect,
    input: &StagingAtomicVectorBlockPaginationInput,
) -> bool {
    let expected_bounds = Rect::new(
        prepared.inner_frame_left(),
        placement.pagination_bounds.y(),
        prepared.inner_frame_width(),
        prepared.content_height(),
    );
    let Some(expected_viewport_top) = placement
        .pagination_bounds
        .y()
        .checked_add(prepared.viewport_top_offset().get())
    else {
        return false;
    };
    let expected_viewport = Rect::new(
        prepared.viewport_left(),
        expected_viewport_top,
        prepared.viewport_width(),
        prepared.viewport_height(),
    );
    let matrix = placement.viewport.matrix;
    if placement.frame_index != 0
        || placement.pagination_bounds != expected_bounds
        || placement.paint_bounds != expected_bounds
        || placement.structure_bounds != expected_bounds
        || placement.viewport.rect != expected_viewport
        || !rect_is_inside_body(expected_bounds, body)
        || matrix.a.raw() != placement.viewport.scale
        || matrix.d.raw() != placement.viewport.scale
        || matrix.b.raw() != 0
        || matrix.c.raw() != 0
        || matrix.e != placement.viewport.rect.x()
        || matrix.f != placement.viewport.rect.y()
        || placement
            .structure_children
            .first()
            .map(|child| (child.owner, child.rect, child.paint_ordinal))
            != Some((
                placement.owner,
                placement.viewport.rect,
                placement.viewport.paint_ordinal,
            ))
    {
        return false;
    }
    if !rect_is_inside_body(placement.viewport.rect, body) {
        return false;
    }
    let Ok(caption_inputs) = captions_for(prepared, input) else {
        return false;
    };
    if placement.captions.len() != caption_inputs.len()
        || placement.structure_children.len()
            != 1 + usize::from(placement.equation_number.is_some()) + placement.captions.len()
    {
        return false;
    }
    let expected_caption_flow = prepared.caption_flow_id();
    let caption_structure_start = 1 + usize::from(placement.equation_number.is_some());
    if !placement
        .captions
        .iter()
        .zip(caption_inputs)
        .zip(&placement.structure_children[caption_structure_start..])
        .all(|((selected, supplied), structure)| {
            selected.owner == supplied.owner()
                && expected_caption_flow == Some(selected.caption_flow_id)
                && selected.rect.x() == body.x()
                && selected.rect.width() == body.width()
                && selected.rect.height() == supplied.block_size()
                && rect_is_inside_body(selected.rect, body)
                && structure.owner == selected.owner
                && structure.role == StagingAtomicVectorStructureRole::Caption
                && structure.page_index == selected.page_index
                && structure.rect == selected.rect
                && structure.paint_ordinal == selected.paint_ordinal
        })
    {
        return false;
    }
    match placement.kind {
        StagingPreparedVectorBlockKind::VectorFigure => {
            placement.math_baseline.is_none()
                && placement.math_flow.is_none()
                && placement.equation_number.is_none()
                && placement.structure_children.first().is_some_and(|child| {
                    child.role == StagingAtomicVectorStructureRole::Figure
                        && child.page_index == placement.page_index
                })
        }
        StagingPreparedVectorBlockKind::MathVectorBlock => {
            let Some(baseline) = &placement.math_baseline else {
                return false;
            };
            let Some(prepared_baseline) = prepared.baseline() else {
                return false;
            };
            let Some(prepared_origin_x) = prepared.origin_x() else {
                return false;
            };
            if placement
                .viewport
                .rect
                .y()
                .checked_add(baseline.baseline.get())
                != Some(baseline.baseline_y)
                || placement.viewport.rect.x().checked_sub(prepared_origin_x)
                    != Some(baseline.pen_origin_x)
                || baseline.baseline != prepared_baseline
                || placement.math_flow.as_ref().map_or(true, |flow| {
                    prepared.math_flow().map(|prepared_flow| {
                        (prepared_flow.flow_id(), prepared_flow.flow_fingerprint())
                    }) != Some((flow.flow_id, flow.flow_fingerprint))
                        || flow.terminal != MathVectorFlowTerminal::ONE
                        || flow.terminal_receipt_fingerprint == [0; 32]
                })
                || placement.structure_children.first().map_or(true, |child| {
                    child.role != StagingAtomicVectorStructureRole::Formula
                        || child.page_index != placement.page_index
                })
            {
                return false;
            }
            match (&placement.equation_number, prepared.equation_number()) {
                (None, None) => placement.structure_children.len() == 1,
                (Some(selected), Some(number)) => {
                    let Some(number_top) = placement
                        .pagination_bounds
                        .y()
                        .checked_add(number.top_offset().get())
                    else {
                        return false;
                    };
                    selected.owner == number.owner()
                        && selected.source_span == number.source_span()
                        && selected.shape_fingerprint == number.shape_fingerprint()
                        && selected.minimum_gap == number.minimum_gap()
                        && selected.rect
                            == Rect::new(number.left(), number_top, number.width(), number.height())
                        && rect_is_inside_body(selected.rect, body)
                        && placement.structure_children.get(1).is_some_and(|child| {
                            child.owner == selected.owner
                                && child.role == StagingAtomicVectorStructureRole::EquationNumber
                                && child.page_index == placement.page_index
                                && child.rect == selected.rect
                                && child.paint_ordinal == selected.paint_ordinal
                        })
                }
                _ => false,
            }
        }
    }
}

fn encode_selected_layout(
    receipt: &StagingAtomicVectorBlockSelectedLayoutReceipt,
    page_geometry: &typaxis_syntax::StagingM4PageGeometry,
    pages: &[StagingAtomicVectorPage],
    placements: &[StagingAtomicVectorBlockPlacement],
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, receipt.admitted_fingerprint);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, PRECOMPOSED_VECTOR_SELECTED_LAYOUT_ALGORITHM);
    output.push_str(",\"binding_set_fingerprint\":");
    push_hash(&mut output, receipt.binding_set_fingerprint);
    output.push_str(",\"block_placement_count\":");
    output.push_str(&placements.len().to_string());
    output.push_str(",\"block_placements\":[");
    for (index, placement) in placements.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"fingerprint\":");
        push_hash(&mut output, placement.fingerprint);
        output.push_str(",\"record\":");
        output.push_str(&encode_placement(placement));
        output.push('}');
    }
    output.push_str("],\"cumulative_fragment_charge\":");
    output.push_str(&receipt.cumulative_fragment_charge.to_string());
    output.push_str(",\"fragment_charge\":");
    output.push_str(&receipt.fragment_charge.to_string());
    output.push_str(",\"layout_epoch_fingerprint\":");
    push_hash(&mut output, receipt.layout_epoch_fingerprint);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, receipt.limits_fingerprint);
    output.push_str(",\"math_flow_registry_fingerprint\":");
    push_hash(&mut output, receipt.math_flow_registry_fingerprint);
    output.push_str(",\"math_terminal_set_fingerprint\":");
    push_hash(&mut output, receipt.math_terminal_set_fingerprint);
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, receipt.package_sha256);
    output.push_str(",\"page_geometry\":");
    output.push_str(page_geometry.canonical_jcs());
    output.push_str(",\"page_geometry_fingerprint\":");
    push_hash(&mut output, receipt.page_geometry_fingerprint);
    output.push_str(",\"pages\":[");
    for (index, page) in pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"block_count\":");
        output.push_str(&page.block_count.to_string());
        output.push_str(",\"caption_count\":");
        output.push_str(&page.caption_count.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push('}');
    }
    output.push_str("],\"pagination_input\":");
    output.push_str(&receipt.pagination_input_canonical_jcs);
    output.push_str(",\"pagination_input_fingerprint\":");
    push_hash(&mut output, receipt.pagination_input_fingerprint);
    output.push_str(",\"preparation_fingerprint\":");
    push_hash(&mut output, receipt.preparation_fingerprint);
    output.push_str(",\"profile_fingerprint\":");
    push_hash(&mut output, receipt.profile_fingerprint);
    output.push('}');
    output
}

fn encode_placement(value: &StagingAtomicVectorBlockPlacement) -> String {
    let mut output = String::from("{\"binding_fingerprint\":");
    push_hash(&mut output, value.binding_fingerprint);
    output.push_str(",\"block_ordinal\":");
    output.push_str(&value.block_ordinal.to_string());
    output.push_str(",\"captions\":[");
    for (index, caption) in value.captions.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"caption_flow_id\":");
        output.push_str(&caption.caption_flow_id.get().to_string());
        output.push_str(",\"owner\":");
        output.push_str(&caption.owner.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&caption.page_index.to_string());
        output.push_str(",\"paint_ordinal\":");
        output.push_str(&caption.paint_ordinal.to_string());
        output.push_str(",\"rect\":");
        push_rect(&mut output, caption.rect);
        output.push('}');
    }
    output.push_str("],\"effective_space_after\":");
    output.push_str(&value.effective_space_after.get().raw().to_string());
    output.push_str(",\"effective_space_before\":");
    output.push_str(&value.effective_space_before.get().raw().to_string());
    output.push_str(",\"end_indent\":");
    output.push_str(&value.end_indent.get().raw().to_string());
    output.push_str(",\"equation_number\":");
    match &value.equation_number {
        Some(number) => {
            output.push_str("{\"minimum_gap\":");
            output.push_str(&number.minimum_gap.get().raw().to_string());
            output.push_str(",\"owner\":");
            output.push_str(&number.owner.get().to_string());
            output.push_str(",\"paint_ordinal\":");
            output.push_str(&number.paint_ordinal.to_string());
            output.push_str(",\"rect\":");
            push_rect(&mut output, number.rect);
            output.push_str(",\"shape_fingerprint\":");
            push_hash(&mut output, number.shape_fingerprint);
            output.push_str(",\"source_span\":");
            push_source_span(&mut output, number.source_span);
            output.push('}');
        }
        None => output.push_str("null"),
    }
    output.push_str(",\"forced_page_break_before\":");
    output.push_str(if value.forced_page_break_before {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"fragment_ordinal\":");
    output.push_str(&value.fragment_ordinal.to_string());
    output.push_str(",\"frame_index\":");
    output.push_str(&value.frame_index.to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&value.image_id.get().to_string());
    output.push_str(",\"keep_caption\":");
    output.push_str(if value.keep_caption { "true" } else { "false" });
    output.push_str(",\"keep_with_next\":");
    output.push_str(if value.keep_with_next {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, value.kind.as_str());
    output.push_str(",\"math_baseline\":");
    match &value.math_baseline {
        Some(baseline) => {
            output.push_str("{\"baseline\":");
            output.push_str(&baseline.baseline.get().raw().to_string());
            output.push_str(",\"baseline_y\":");
            output.push_str(&baseline.baseline_y.raw().to_string());
            output.push_str(",\"pen_origin_x\":");
            output.push_str(&baseline.pen_origin_x.raw().to_string());
            output.push('}');
        }
        None => output.push_str("null"),
    }
    output.push_str(",\"math_flow\":");
    match &value.math_flow {
        Some(flow) => {
            output.push_str("{\"flow_fingerprint\":");
            push_hash(&mut output, flow.flow_fingerprint);
            output.push_str(",\"flow_id\":");
            output.push_str(&flow.flow_id.get().to_string());
            output.push_str(",\"terminal\":");
            output.push_str(&flow.terminal.get().to_string());
            output.push_str(",\"terminal_receipt_fingerprint\":");
            push_hash(&mut output, flow.terminal_receipt_fingerprint);
            output.push('}');
        }
        None => output.push_str("null"),
    }
    output.push_str(",\"moved_to_fresh_page\":");
    output.push_str(if value.moved_to_fresh_page {
        "true"
    } else {
        "false"
    });
    output.push_str(",\"node_id\":");
    output.push_str(&value.owner.get().to_string());
    output.push_str(",\"page_block_ordinal\":");
    output.push_str(&value.page_block_ordinal.to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&value.page_index.to_string());
    output.push_str(",\"pagination_bounds\":");
    push_rect(&mut output, value.pagination_bounds);
    output.push_str(",\"paint_bounds\":");
    push_rect(&mut output, value.paint_bounds);
    output.push_str(",\"parent_flow_id\":");
    output.push_str(&value.parent_flow_id.get().to_string());
    output.push_str(",\"parent_position\":");
    output.push_str(&value.parent_position.to_string());
    output.push_str(",\"requested_page\":");
    match &value.requested_page {
        Some(page) => push_jcs_string(&mut output, page.as_str()),
        None => output.push_str("null"),
    }
    output.push_str(",\"requested_space_after\":");
    output.push_str(&value.requested_space_after.get().raw().to_string());
    output.push_str(",\"requested_space_before\":");
    output.push_str(&value.requested_space_before.get().raw().to_string());
    output.push_str(",\"source_span\":");
    push_source_span(&mut output, value.source_span);
    output.push_str(",\"start_indent\":");
    output.push_str(&value.start_indent.get().raw().to_string());
    output.push_str(",\"structure_bounds\":");
    push_rect(&mut output, value.structure_bounds);
    output.push_str(",\"structure_children\":[");
    for (index, child) in value.structure_children.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"owner\":");
        output.push_str(&child.owner.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&child.page_index.to_string());
        output.push_str(",\"paint_ordinal\":");
        output.push_str(&child.paint_ordinal.to_string());
        output.push_str(",\"rect\":");
        push_rect(&mut output, child.rect);
        output.push_str(",\"role\":");
        push_jcs_string(&mut output, child.role.as_str());
        output.push('}');
    }
    output.push_str("],\"style_fingerprint\":");
    push_hash(&mut output, value.style_fingerprint);
    output.push_str(",\"text_align\":");
    push_jcs_string(&mut output, value.text_align.as_str());
    output.push_str(",\"viewport\":{\"matrix\":");
    push_matrix(&mut output, value.viewport.matrix);
    output.push_str(",\"paint_ordinal\":");
    output.push_str(&value.viewport.paint_ordinal.to_string());
    output.push_str(",\"rect\":");
    push_rect(&mut output, value.viewport.rect);
    output.push_str(",\"scale\":");
    output.push_str(&value.viewport.scale.to_string());
    output.push_str("}}");
    output
}

fn encode_pagination_input(
    initial_consumed_block_size: NonNegativeLength,
    prior_fragment_charge: u64,
    captions: &[StagingFigureCaptionBlockInput],
    keep_successors: &[StagingAtomicVectorKeepSuccessorInput],
    preparation_fingerprint: [u8; 32],
) -> String {
    let mut output = String::from("{\"captions\":[");
    for (index, caption) in captions.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"block_size\":");
        output.push_str(&caption.block_size().get().raw().to_string());
        output.push_str(",\"owner\":");
        output.push_str(&caption.owner().get().to_string());
        output.push('}');
    }
    output.push_str("],\"initial_consumed_block_size\":");
    output.push_str(&initial_consumed_block_size.get().raw().to_string());
    output.push_str(",\"keep_successors\":[");
    for (index, successor) in keep_successors.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"owner\":");
        output.push_str(&successor.owner.get().to_string());
        output.push_str(",\"required_extent_after_vector\":");
        output.push_str(
            &successor
                .required_extent_after_vector
                .get()
                .raw()
                .to_string(),
        );
        output.push_str(",\"successor_owner\":");
        output.push_str(&successor.successor_owner.get().to_string());
        output.push('}');
    }
    output.push_str("],\"preparation_fingerprint\":");
    push_hash(&mut output, preparation_fingerprint);
    output.push_str(",\"prior_fragment_charge\":");
    output.push_str(&prior_fragment_charge.to_string());
    output.push('}');
    output
}

fn push_matrix(output: &mut String, value: AffineTransform) {
    output.push_str("{\"a_16_16\":");
    output.push_str(&value.a.raw().to_string());
    output.push_str(",\"b_16_16\":");
    output.push_str(&value.b.raw().to_string());
    output.push_str(",\"c_16_16\":");
    output.push_str(&value.c.raw().to_string());
    output.push_str(",\"d_16_16\":");
    output.push_str(&value.d.raw().to_string());
    output.push_str(",\"e\":");
    output.push_str(&value.e.raw().to_string());
    output.push_str(",\"f\":");
    output.push_str(&value.f.raw().to_string());
    output.push('}');
}

fn push_rect(output: &mut String, value: Rect) {
    output.push_str("{\"height\":");
    output.push_str(&value.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&value.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&value.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&value.y().raw().to_string());
    output.push('}');
}

fn push_source_span(output: &mut String, value: SourceSpan) {
    output.push_str("{\"end_byte\":");
    output.push_str(&value.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&value.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&value.start_byte().get().to_string());
    output.push('}');
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
    use typaxis_layout::{
        staging_precomposed_vector_block_layout_fixture,
        staging_precomposed_vector_block_layout_fixture_for_case,
        StagingPrecomposedVectorBlockFixtureCase,
    };

    fn nonnegative(raw: i64) -> NonNegativeLength {
        NonNegativeLength::new(Length::from_raw(raw).unwrap()).unwrap()
    }

    fn positive(raw: i64) -> PositiveLength {
        PositiveLength::new(Length::from_raw(raw).unwrap()).unwrap()
    }

    #[test]
    fn atomic_vector_block_moves_whole_math_and_consumes_one_terminal() {
        let fixture = staging_precomposed_vector_block_layout_fixture_for_case(
            StagingPrecomposedVectorBlockFixtureCase::FigureCaption,
        )
        .unwrap();
        let input = StagingAtomicVectorBlockPaginationInput::new(
            &fixture.layout,
            nonnegative(60 * 65_536),
            0,
            vec![StagingFigureCaptionBlockInput::new(
                NodeId::new(6),
                positive(20 * 65_536),
            )],
            Vec::new(),
        )
        .unwrap();
        let selected = paginate_staging_atomic_vector_blocks(
            &fixture.layout,
            &fixture.math_flows,
            &input,
            &fixture.limits,
        )
        .unwrap();
        selected
            .verify(
                &fixture.layout,
                &fixture.math_flows,
                &input,
                &fixture.limits,
            )
            .unwrap();
        assert_eq!(
            selected.trace_json(&fixture.layout),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/block-layout-trace.json"
            ))
            .trim_end()
        );
        assert_eq!(selected.placements().len(), 2);
        assert_eq!(selected.receipt().fragment_charge(), 2);
        let math = selected
            .placements()
            .iter()
            .find(|placement| placement.kind() == StagingPreparedVectorBlockKind::MathVectorBlock)
            .unwrap();
        assert_eq!(math.page_index(), 1);
        assert!(math.moved_to_fresh_page());
        assert_eq!(math.fragment_ordinal(), math.block_ordinal());
        assert_eq!(math.math_flow().unwrap().terminal().get(), 1);
        assert_ne!(
            math.math_flow().unwrap().terminal_receipt_fingerprint(),
            [0; 32]
        );
        assert_eq!(selected.math_terminals().receipts().len(), 1);
        let baseline = math.math_baseline().unwrap();
        assert_eq!(
            math.viewport()
                .rect()
                .y()
                .checked_add(baseline.baseline().get()),
            Some(baseline.baseline_y())
        );
        let number = math.equation_number().unwrap();
        assert_eq!(
            math.structure_children()[0].role(),
            StagingAtomicVectorStructureRole::Formula
        );
        assert_eq!(math.structure_children()[0].rect(), math.viewport().rect());
        assert_eq!(
            math.structure_children()[1].role(),
            StagingAtomicVectorStructureRole::EquationNumber
        );
        assert_eq!(math.structure_children()[1].rect(), number.rect());
        assert_eq!(
            math.pagination_bounds().height().get().raw(),
            math.viewport()
                .rect()
                .height()
                .get()
                .raw()
                .max(number.rect().height().get().raw())
        );
    }

    #[test]
    fn atomic_vector_block_fragment_charge_is_not_reset() {
        let fixture = staging_precomposed_vector_block_layout_fixture().unwrap();
        let exact_prior = fixture
            .limits
            .base()
            .get()
            .max_fragments
            .checked_sub(2)
            .unwrap();
        let exact = StagingAtomicVectorBlockPaginationInput::new(
            &fixture.layout,
            NonNegativeLength::ZERO,
            exact_prior,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let selected = paginate_staging_atomic_vector_blocks(
            &fixture.layout,
            &fixture.math_flows,
            &exact,
            &fixture.limits,
        )
        .unwrap();
        assert_eq!(
            selected.receipt().cumulative_fragment_charge(),
            fixture.limits.base().get().max_fragments
        );

        let over = StagingAtomicVectorBlockPaginationInput::new(
            &fixture.layout,
            NonNegativeLength::ZERO,
            exact_prior + 1,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(
            paginate_staging_atomic_vector_blocks(
                &fixture.layout,
                &fixture.math_flows,
                &over,
                &fixture.limits,
            ),
            Err(StagingAtomicVectorBlockLayoutError::FragmentLimit)
        );
    }

    #[test]
    fn atomic_vector_block_rejects_input_from_another_preparation() {
        let source = staging_precomposed_vector_block_layout_fixture().unwrap();
        let input = StagingAtomicVectorBlockPaginationInput::new(
            &source.layout,
            NonNegativeLength::ZERO,
            0,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let target = staging_precomposed_vector_block_layout_fixture_for_case(
            StagingPrecomposedVectorBlockFixtureCase::AlignmentStart,
        )
        .unwrap();
        assert_ne!(
            input.preparation_fingerprint(),
            target.layout.receipt().fingerprint()
        );
        assert_eq!(
            paginate_staging_atomic_vector_blocks(
                &target.layout,
                &target.math_flows,
                &input,
                &target.limits,
            ),
            Err(StagingAtomicVectorBlockLayoutError::LayoutMismatch)
        );
    }

    #[test]
    fn atomic_vector_block_suppresses_page_top_space_and_consumes_pending_glue() {
        let fixture = staging_precomposed_vector_block_layout_fixture_for_case(
            StagingPrecomposedVectorBlockFixtureCase::AlignmentStart,
        )
        .unwrap();
        let input = StagingAtomicVectorBlockPaginationInput::new(
            &fixture.layout,
            NonNegativeLength::ZERO,
            0,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let selected = paginate_staging_atomic_vector_blocks(
            &fixture.layout,
            &fixture.math_flows,
            &input,
            &fixture.limits,
        )
        .unwrap();
        let figure = &selected.placements()[0];
        let math = &selected.placements()[1];
        assert_eq!(figure.effective_space_before(), NonNegativeLength::ZERO);
        assert_eq!(figure.effective_space_after().get().raw(), 196_608);
        assert_eq!(math.effective_space_before().get().raw(), 327_680);
        assert_eq!(math.effective_space_after(), NonNegativeLength::ZERO);
        assert!(math.equation_number().is_none());
        assert_eq!(math.structure_children().len(), 1);
        assert_eq!(
            math.structure_children()[0].role(),
            StagingAtomicVectorStructureRole::Formula
        );
        assert_eq!(math.pagination_bounds(), math.paint_bounds());
        assert_eq!(math.pagination_bounds(), math.structure_bounds());
    }

    #[test]
    fn atomic_vector_block_keeps_figure_caption_on_the_same_fresh_page() {
        let fixture = staging_precomposed_vector_block_layout_fixture_for_case(
            StagingPrecomposedVectorBlockFixtureCase::FigureCaption,
        )
        .unwrap();
        let input = StagingAtomicVectorBlockPaginationInput::new(
            &fixture.layout,
            nonnegative(75 * 65_536),
            0,
            vec![StagingFigureCaptionBlockInput::new(
                NodeId::new(6),
                positive(20 * 65_536),
            )],
            Vec::new(),
        )
        .unwrap();
        let selected = paginate_staging_atomic_vector_blocks(
            &fixture.layout,
            &fixture.math_flows,
            &input,
            &fixture.limits,
        )
        .unwrap();
        let figure = &selected.placements()[0];
        assert_eq!(figure.owner(), NodeId::new(5));
        assert_eq!(figure.page_index(), 1);
        assert!(figure.moved_to_fresh_page());
        assert_eq!(figure.captions().len(), 1);
        assert_eq!(figure.captions()[0].owner(), NodeId::new(6));
        assert_eq!(figure.captions()[0].page_index(), figure.page_index());
        assert_eq!(figure.structure_children().len(), 2);
        assert_eq!(
            figure.structure_children()[0].role(),
            StagingAtomicVectorStructureRole::Figure
        );
        assert_eq!(
            figure.structure_children()[1].role(),
            StagingAtomicVectorStructureRole::Caption
        );
        assert!(
            figure.structure_children()[0].paint_ordinal()
                < figure.structure_children()[1].paint_ordinal()
        );

        let mut wrong_caption_page = selected.clone();
        wrong_caption_page.placements[0].captions[0].page_index = 99;
        wrong_caption_page.placements[0].structure_children[1].page_index = 99;
        assert!(!pages_are_closed(
            &wrong_caption_page.pages,
            &wrong_caption_page.placements,
            fixture.layout.page_geometry().body(),
            fixture.limits.base(),
        ));
    }

    #[test]
    fn atomic_vector_block_allows_an_unkept_caption_on_the_next_page() {
        let fixture = staging_precomposed_vector_block_layout_fixture_for_case(
            StagingPrecomposedVectorBlockFixtureCase::FigureCaptionSplit,
        )
        .unwrap();
        let input = StagingAtomicVectorBlockPaginationInput::new(
            &fixture.layout,
            nonnegative(80 * 65_536),
            0,
            vec![StagingFigureCaptionBlockInput::new(
                NodeId::new(6),
                positive(20 * 65_536),
            )],
            Vec::new(),
        )
        .unwrap();
        let selected = paginate_staging_atomic_vector_blocks(
            &fixture.layout,
            &fixture.math_flows,
            &input,
            &fixture.limits,
        )
        .unwrap();
        let figure = &selected.placements()[0];
        assert!(!figure.keep_caption());
        assert_eq!(figure.page_index(), 0);
        assert!(!figure.moved_to_fresh_page());
        assert_eq!(figure.captions()[0].page_index(), 1);
        assert_eq!(
            figure.structure_children()[0].role(),
            StagingAtomicVectorStructureRole::Figure
        );
        assert_eq!(
            figure.structure_children()[1].role(),
            StagingAtomicVectorStructureRole::Caption
        );
        assert!(
            figure.structure_children()[0].paint_ordinal()
                < figure.structure_children()[1].paint_ordinal()
        );
    }

    #[test]
    fn atomic_vector_block_keep_with_next_moves_the_whole_group() {
        let fixture = staging_precomposed_vector_block_layout_fixture_for_case(
            StagingPrecomposedVectorBlockFixtureCase::KeepWithNext,
        )
        .unwrap();
        let input = StagingAtomicVectorBlockPaginationInput::new(
            &fixture.layout,
            nonnegative(75 * 65_536),
            0,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let selected = paginate_staging_atomic_vector_blocks(
            &fixture.layout,
            &fixture.math_flows,
            &input,
            &fixture.limits,
        )
        .unwrap();
        assert_eq!(selected.placements()[0].page_index(), 1);
        assert_eq!(selected.placements()[1].page_index(), 1);
        assert!(selected.placements()[0].moved_to_fresh_page());
        assert!(!selected.placements()[1].moved_to_fresh_page());
    }

    #[test]
    fn atomic_vector_block_honors_forced_and_named_page_boundaries() {
        for case in [
            StagingPrecomposedVectorBlockFixtureCase::ForcedPageBreak,
            StagingPrecomposedVectorBlockFixtureCase::NamedPage,
        ] {
            let fixture = staging_precomposed_vector_block_layout_fixture_for_case(case).unwrap();
            let input = StagingAtomicVectorBlockPaginationInput::new(
                &fixture.layout,
                NonNegativeLength::ZERO,
                0,
                Vec::new(),
                Vec::new(),
            )
            .unwrap();
            let selected = paginate_staging_atomic_vector_blocks(
                &fixture.layout,
                &fixture.math_flows,
                &input,
                &fixture.limits,
            )
            .unwrap();
            let math = &selected.placements()[1];
            assert_eq!(math.page_index(), 1);
            assert!(math.moved_to_fresh_page());
            assert_eq!(
                math.forced_page_break_before(),
                case == StagingPrecomposedVectorBlockFixtureCase::ForcedPageBreak
            );
        }
    }

    #[test]
    fn atomic_vector_block_rejects_svg_height_on_an_empty_page() {
        let fixture = staging_precomposed_vector_block_layout_fixture_for_case(
            StagingPrecomposedVectorBlockFixtureCase::ShortBody,
        )
        .unwrap();
        let input = StagingAtomicVectorBlockPaginationInput::new(
            &fixture.layout,
            NonNegativeLength::ZERO,
            0,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let error = paginate_staging_atomic_vector_blocks(
            &fixture.layout,
            &fixture.math_flows,
            &input,
            &fixture.limits,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            StagingAtomicVectorBlockLayoutError::Oversize(owner, _)
                if owner == NodeId::new(5)
        ));
        assert!(error.to_string().starts_with("L5100:"));
    }

    #[test]
    fn atomic_vector_block_consumes_mixed_vector_terminals_once_in_source_order() {
        let fixture = staging_precomposed_vector_block_layout_fixture_for_case(
            StagingPrecomposedVectorBlockFixtureCase::MixedNativeMath,
        )
        .unwrap();
        let input = StagingAtomicVectorBlockPaginationInput::new(
            &fixture.layout,
            NonNegativeLength::ZERO,
            0,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let selected = paginate_staging_atomic_vector_blocks(
            &fixture.layout,
            &fixture.math_flows,
            &input,
            &fixture.limits,
        )
        .unwrap();
        assert_eq!(selected.math_terminals().receipts().len(), 2);
        assert_eq!(
            selected
                .placements()
                .iter()
                .map(|placement| (
                    placement.owner().get(),
                    placement.parent_position(),
                    placement.math_flow().unwrap().flow_id().get(),
                    placement.math_flow().unwrap().terminal().get(),
                ))
                .collect::<Vec<_>>(),
            vec![(3, 1, 0, 1), (5, 3, 1, 1)]
        );
    }
}
