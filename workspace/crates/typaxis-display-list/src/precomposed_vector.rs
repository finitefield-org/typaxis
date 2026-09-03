use typaxis_core::{
    push_jcs_string, sha256, AffineTransform, ImageResourceId, Length, M4EffectiveResourceLimits,
    NodeId, NonNegativeLength, Rect, Unitless16_16,
};
use typaxis_layout::{
    BoundPrecomposedVectorMedia, FlowId, MathVectorFlowId, MathVectorFlowTerminal,
    PrecomposedVectorPlacementInput, ResolvedRgb8, StagingInlineVectorParagraphInput,
    StagingInlineVectorPlacement, StagingInlineVectorSelectedLayout, StagingMathVectorFlowRegistry,
    StagingPrecomposedVectorBlockLayout, StagingPreparedVectorBlock,
    StagingPreparedVectorBlockKind, ValidatedPrecomposedVectorBindings,
    ValidatedPrecomposedVectorReceipt,
};
use typaxis_pagination::{
    StagingAtomicVectorBlockPaginationInput, StagingAtomicVectorBlockPlacement,
    StagingAtomicVectorBlockSelectedLayout,
};
use typaxis_resource_admission::{
    AdmittedResourceLedger, VectorContentKey, VectorContentMediaType,
};
use typaxis_syntax::{
    PrecomposedVectorKind, StagingPrecomposedVectorProfileAuthorization,
    ValidatedStagingSemanticPackage,
};

pub const STAGING_DRAW_VECTOR_V2_ALGORITHM: &str = "typaxis.draw-vector-display/2";

/// Complete upstream selected state consumed by DrawVector Display `/2`.
/// Keeping both layout inputs here makes it impossible for Display to accept
/// an inline or block receipt without the data required to revalidate it.
#[derive(Clone, Copy, Debug)]
pub struct StagingPrecomposedVectorDisplayLayoutInput<'a> {
    inline_input: &'a [StagingInlineVectorParagraphInput],
    inline_selected: &'a StagingInlineVectorSelectedLayout,
    block_preparation: &'a StagingPrecomposedVectorBlockLayout,
    math_flows: &'a StagingMathVectorFlowRegistry,
    block_pagination_input: &'a StagingAtomicVectorBlockPaginationInput,
    block_selected: &'a StagingAtomicVectorBlockSelectedLayout,
}

impl<'a> StagingPrecomposedVectorDisplayLayoutInput<'a> {
    pub const fn new(
        inline_input: &'a [StagingInlineVectorParagraphInput],
        inline_selected: &'a StagingInlineVectorSelectedLayout,
        block_preparation: &'a StagingPrecomposedVectorBlockLayout,
        math_flows: &'a StagingMathVectorFlowRegistry,
        block_pagination_input: &'a StagingAtomicVectorBlockPaginationInput,
        block_selected: &'a StagingAtomicVectorBlockSelectedLayout,
    ) -> Self {
        Self {
            inline_input,
            inline_selected,
            block_preparation,
            math_flows,
            block_pagination_input,
            block_selected,
        }
    }

    pub const fn inline_selected(&self) -> &StagingInlineVectorSelectedLayout {
        self.inline_selected
    }

    pub const fn block_selected(&self) -> &StagingAtomicVectorBlockSelectedLayout {
        self.block_selected
    }

    fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingPrecomposedVectorProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        bindings: &ValidatedPrecomposedVectorBindings,
    ) -> Result<(), StagingPrecomposedVectorDisplayError> {
        bindings
            .verify(package, profile, limits, admitted)
            .map_err(|_| StagingPrecomposedVectorDisplayError::SelectedMismatch)?;
        self.inline_selected
            .verify(
                package,
                profile,
                limits,
                admitted,
                bindings,
                self.inline_input,
            )
            .map_err(|_| StagingPrecomposedVectorDisplayError::SelectedMismatch)?;
        self.block_preparation
            .verify(
                package,
                profile,
                limits,
                admitted,
                bindings,
                self.math_flows,
            )
            .map_err(|_| StagingPrecomposedVectorDisplayError::SelectedMismatch)?;
        self.block_selected
            .verify(
                self.block_preparation,
                self.math_flows,
                self.block_pagination_input,
                limits,
            )
            .map_err(|_| StagingPrecomposedVectorDisplayError::SelectedMismatch)?;
        if self.inline_selected.page_geometry() != self.block_preparation.page_geometry()
            || self.inline_selected.receipt().package_sha256()
                != self.block_selected.receipt().package_sha256()
            || self.inline_selected.receipt().profile_fingerprint()
                != self.block_selected.receipt().profile_fingerprint()
            || self.inline_selected.receipt().limits_fingerprint()
                != self.block_selected.receipt().limits_fingerprint()
            || self.inline_selected.receipt().admitted_fingerprint()
                != self.block_selected.receipt().admitted_fingerprint()
            || self.inline_selected.receipt().binding_set_fingerprint()
                != self.block_selected.receipt().binding_set_fingerprint()
            || self.inline_selected.receipt().layout_epoch_fingerprint()
                != self.block_selected.receipt().layout_epoch_fingerprint()
        {
            return Err(StagingPrecomposedVectorDisplayError::SelectedMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingDrawVectorBaselineMetrics {
    metric_receipt_fingerprint: [u8; 32],
    pen_origin_x: Length,
    baseline: NonNegativeLength,
    baseline_y: Length,
}

impl StagingDrawVectorBaselineMetrics {
    pub const fn metric_receipt_fingerprint(self) -> [u8; 32] {
        self.metric_receipt_fingerprint
    }

    pub const fn pen_origin_x(self) -> Length {
        self.pen_origin_x
    }

    pub const fn baseline(self) -> NonNegativeLength {
        self.baseline
    }

    pub const fn baseline_y(self) -> Length {
        self.baseline_y
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingDrawVectorFigureCaption {
    caption_flow_id: FlowId,
    caption_owners: Vec<NodeId>,
    keep_caption: bool,
}

impl StagingDrawVectorFigureCaption {
    pub const fn caption_flow_id(&self) -> FlowId {
        self.caption_flow_id
    }

    pub fn caption_owners(&self) -> &[NodeId] {
        &self.caption_owners
    }

    pub const fn keep_caption(&self) -> bool {
        self.keep_caption
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StagingDrawVectorMathFlow {
    flow_id: MathVectorFlowId,
    flow_fingerprint: [u8; 32],
    parent_flow_id: FlowId,
    parent_position: u32,
    terminal: MathVectorFlowTerminal,
    terminal_receipt_fingerprint: [u8; 32],
}

impl StagingDrawVectorMathFlow {
    pub const fn flow_id(self) -> MathVectorFlowId {
        self.flow_id
    }

    pub const fn flow_fingerprint(self) -> [u8; 32] {
        self.flow_fingerprint
    }

    pub const fn parent_flow_id(self) -> FlowId {
        self.parent_flow_id
    }

    pub const fn parent_position(self) -> u32 {
        self.parent_position
    }

    pub const fn terminal(self) -> MathVectorFlowTerminal {
        self.terminal
    }

    pub const fn terminal_receipt_fingerprint(self) -> [u8; 32] {
        self.terminal_receipt_fingerprint
    }
}

/// Kind-conditional state. Source TeX, alternative text, resource URI, PDF
/// names, object numbers, and MCIDs are deliberately not representable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingDrawVectorV2Relation {
    Inline {
        baseline_metrics: StagingDrawVectorBaselineMetrics,
    },
    VectorFigure {
        figure_caption: StagingDrawVectorFigureCaption,
    },
    MathVectorBlock {
        baseline_metrics: StagingDrawVectorBaselineMetrics,
        math_flow: StagingDrawVectorMathFlow,
    },
}

impl StagingDrawVectorV2Relation {
    pub const fn baseline_metrics(&self) -> Option<StagingDrawVectorBaselineMetrics> {
        match self {
            Self::Inline { baseline_metrics }
            | Self::MathVectorBlock {
                baseline_metrics, ..
            } => Some(*baseline_metrics),
            Self::VectorFigure { .. } => None,
        }
    }

    pub const fn figure_caption(&self) -> Option<&StagingDrawVectorFigureCaption> {
        match self {
            Self::VectorFigure { figure_caption } => Some(figure_caption),
            Self::Inline { .. } | Self::MathVectorBlock { .. } => None,
        }
    }

    pub const fn math_flow(&self) -> Option<StagingDrawVectorMathFlow> {
        match self {
            Self::MathVectorBlock { math_flow, .. } => Some(*math_flow),
            Self::Inline { .. } | Self::VectorFigure { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingDrawVectorV2 {
    usage_id: u32,
    owner: NodeId,
    kind: PrecomposedVectorKind,
    image_id: ImageResourceId,
    content_key: VectorContentKey,
    ir_fingerprint: [u8; 32],
    binding_fingerprint: [u8; 32],
    selected_placement_fingerprint: [u8; 32],
    page_index: u32,
    frame_index: u32,
    fragment_ordinal: u32,
    paint_ordinal: u32,
    viewport: Rect,
    scale: i32,
    matrix: AffineTransform,
    resolved_current_color: ResolvedRgb8,
    relation: StagingDrawVectorV2Relation,
    fingerprint: [u8; 32],
}

impl StagingDrawVectorV2 {
    pub const fn usage_id(&self) -> u32 {
        self.usage_id
    }

    pub const fn owner(&self) -> NodeId {
        self.owner
    }

    pub const fn kind(&self) -> PrecomposedVectorKind {
        self.kind
    }

    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }

    pub const fn content_key(&self) -> VectorContentKey {
        self.content_key
    }

    pub const fn ir_fingerprint(&self) -> [u8; 32] {
        self.ir_fingerprint
    }

    pub const fn binding_fingerprint(&self) -> [u8; 32] {
        self.binding_fingerprint
    }

    pub const fn selected_placement_fingerprint(&self) -> [u8; 32] {
        self.selected_placement_fingerprint
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

    pub const fn viewport(&self) -> Rect {
        self.viewport
    }

    pub const fn scale_raw(&self) -> i32 {
        self.scale
    }

    pub const fn matrix(&self) -> AffineTransform {
        self.matrix
    }

    pub const fn resolved_current_color(&self) -> ResolvedRgb8 {
        self.resolved_current_color
    }

    pub const fn relation(&self) -> &StagingDrawVectorV2Relation {
        &self.relation
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPrecomposedVectorDisplayPage {
    page_index: u32,
    commands: Vec<StagingDrawVectorV2>,
    fingerprint: [u8; 32],
}

impl StagingPrecomposedVectorDisplayPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }

    pub fn commands(&self) -> &[StagingDrawVectorV2] {
        &self.commands
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPrecomposedVectorDisplayReceipt {
    package_sha256: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    binding_set_fingerprint: [u8; 32],
    inline_selected_layout_fingerprint: [u8; 32],
    block_selected_layout_fingerprint: [u8; 32],
    page_geometry_fingerprint: [u8; 32],
    page_count: u32,
    command_count: u32,
    content_key_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingPrecomposedVectorDisplayReceipt {
    pub const fn algorithm(&self) -> &'static str {
        STAGING_DRAW_VECTOR_V2_ALGORITHM
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

    pub const fn inline_selected_layout_fingerprint(&self) -> [u8; 32] {
        self.inline_selected_layout_fingerprint
    }

    pub const fn block_selected_layout_fingerprint(&self) -> [u8; 32] {
        self.block_selected_layout_fingerprint
    }

    pub const fn page_geometry_fingerprint(&self) -> [u8; 32] {
        self.page_geometry_fingerprint
    }

    pub const fn page_count(&self) -> u32 {
        self.page_count
    }

    pub const fn command_count(&self) -> u32 {
        self.command_count
    }

    pub const fn content_key_count(&self) -> u32 {
        self.content_key_count
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingPrecomposedVectorDisplay {
    pages: Vec<StagingPrecomposedVectorDisplayPage>,
    receipt: StagingPrecomposedVectorDisplayReceipt,
}

impl StagingPrecomposedVectorDisplay {
    pub fn pages(&self) -> &[StagingPrecomposedVectorDisplayPage] {
        &self.pages
    }

    pub fn commands(&self) -> impl Iterator<Item = &StagingDrawVectorV2> {
        self.pages.iter().flat_map(|page| &page.commands)
    }

    pub const fn receipt(&self) -> &StagingPrecomposedVectorDisplayReceipt {
        &self.receipt
    }

    pub fn trace_json(&self) -> String {
        let mut output = String::from(
            "{\"contract\":\"typaxis.contract/1.4\",\"coordinate_unit\":\"pdf_point_1_65536\",\"precomposed_vector_display\":",
        );
        output.push_str(self.receipt.canonical_jcs());
        output.push('}');
        output
    }

    /// Dependency-inversion boundary for Form planning. Every selected usage
    /// can be recovered from this sealed Display receipt without reopening
    /// source, SVG bytes, or layout.
    pub fn verify_resource_closure(&self) -> Result<(), StagingPrecomposedVectorDisplayError> {
        let command_count = self
            .pages
            .iter()
            .try_fold(0usize, |total, page| total.checked_add(page.commands.len()));
        let content_key_count = distinct_content_key_count(&self.pages)?;
        let canonical = encode_display(&self.receipt, &self.pages);
        if self.pages.is_empty()
            || usize::try_from(self.receipt.page_count) != Ok(self.pages.len())
            || command_count.and_then(|count| u32::try_from(count).ok())
                != Some(self.receipt.command_count)
            || content_key_count != self.receipt.content_key_count
            || self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch);
        }

        let mut expected_usage_id = 0u32;
        let mut previous_order: Option<(u32, u32)> = None;
        for (page_index, page) in self.pages.iter().enumerate() {
            if usize::try_from(page.page_index) != Ok(page_index)
                || page.fingerprint != sha256(encode_page_record(page).as_bytes())
            {
                return Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch);
            }
            for command in &page.commands {
                let order = (command.page_index, command.paint_ordinal);
                if command.page_index != page.page_index
                    || command.usage_id != expected_usage_id
                    || previous_order.is_some_and(|previous| previous >= order)
                    || !command_is_closed(command)
                    || command.fingerprint != sha256(encode_command_record(command).as_bytes())
                {
                    return Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch);
                }
                expected_usage_id = expected_usage_id
                    .checked_add(1)
                    .ok_or(StagingPrecomposedVectorDisplayError::CommandLimit)?;
                previous_order = Some(order);
            }
        }
        if expected_usage_id != self.receipt.command_count
            || has_duplicate_selected_occurrence(&self.pages)?
        {
            return Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch);
        }
        Ok(())
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingPrecomposedVectorProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        bindings: &ValidatedPrecomposedVectorBindings,
        layout: &StagingPrecomposedVectorDisplayLayoutInput<'_>,
    ) -> Result<(), StagingPrecomposedVectorDisplayError> {
        layout.verify(package, profile, limits, admitted, bindings)?;
        self.verify_resource_closure()?;
        let expected = build_display(package, profile, limits, admitted, bindings, layout, true)?;
        if self != &expected {
            return Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingPrecomposedVectorDisplayError {
    SelectedMismatch,
    ResourceMismatch(ImageResourceId),
    PaintOrderCollision(u32, u32),
    PageLimit,
    CommandLimit,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingPrecomposedVectorDisplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelectedMismatch => {
                formatter.write_str("I9190: precomposed vector selected layout mismatch")
            }
            Self::ResourceMismatch(image_id) => write!(
                formatter,
                "I9190: precomposed vector resource {} does not match Display",
                image_id.get()
            ),
            Self::PaintOrderCollision(page, paint) => write!(
                formatter,
                "I9190: duplicate precomposed vector paint order {page}:{paint}"
            ),
            Self::PageLimit => {
                formatter.write_str("L5100: precomposed vector Display page limit exceeded")
            }
            Self::CommandLimit => {
                formatter.write_str("L5110: precomposed vector DrawVector command limit exceeded")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5111: precomposed vector Display allocation failed")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: precomposed DrawVector Display receipt mismatch")
            }
        }
    }
}

impl std::error::Error for StagingPrecomposedVectorDisplayError {}

pub fn build_staging_precomposed_vector_display(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
    layout: &StagingPrecomposedVectorDisplayLayoutInput<'_>,
) -> Result<StagingPrecomposedVectorDisplay, StagingPrecomposedVectorDisplayError> {
    layout.verify(package, profile, limits, admitted, bindings)?;
    let display = build_display(package, profile, limits, admitted, bindings, layout, true)?;
    display.verify_resource_closure()?;
    Ok(display)
}

fn build_display(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
    layout: &StagingPrecomposedVectorDisplayLayoutInput<'_>,
    inline_first: bool,
) -> Result<StagingPrecomposedVectorDisplay, StagingPrecomposedVectorDisplayError> {
    let command_count = layout
        .inline_selected
        .placements()
        .len()
        .checked_add(layout.block_selected.placements().len())
        .ok_or(StagingPrecomposedVectorDisplayError::CommandLimit)?;
    if command_count != bindings.receipts().len()
        || u64::try_from(command_count)
            .map_or(true, |count| count > limits.base().get().max_fragments)
    {
        return Err(StagingPrecomposedVectorDisplayError::SelectedMismatch);
    }
    let command_count_u32 = u32::try_from(command_count)
        .map_err(|_| StagingPrecomposedVectorDisplayError::CommandLimit)?;
    let mut commands = Vec::new();
    commands
        .try_reserve_exact(command_count)
        .map_err(|_| StagingPrecomposedVectorDisplayError::AllocationFailure)?;
    if inline_first {
        collect_inline_commands(&mut commands, admitted, bindings, layout.inline_selected)?;
        collect_block_commands(
            &mut commands,
            admitted,
            bindings,
            layout.block_preparation,
            layout.block_selected,
        )?;
    } else {
        collect_block_commands(
            &mut commands,
            admitted,
            bindings,
            layout.block_preparation,
            layout.block_selected,
        )?;
        collect_inline_commands(&mut commands, admitted, bindings, layout.inline_selected)?;
    }

    commands.sort_unstable_by_key(|command| command.owner);
    if commands
        .windows(2)
        .any(|pair| pair[0].owner == pair[1].owner)
        || commands
            .iter()
            .zip(bindings.receipts())
            .any(|(command, binding)| {
                command.owner != binding.node_id()
                    || command.kind != binding.kind()
                    || command.binding_fingerprint != binding.fingerprint()
            })
    {
        return Err(StagingPrecomposedVectorDisplayError::SelectedMismatch);
    }
    commands.sort_unstable_by_key(|command| command.selected_placement_fingerprint);
    if commands.windows(2).any(|pair| {
        pair[0].selected_placement_fingerprint == pair[1].selected_placement_fingerprint
    }) {
        return Err(StagingPrecomposedVectorDisplayError::SelectedMismatch);
    }
    commands.sort_unstable_by_key(|command| (command.page_index, command.paint_ordinal));
    if let Some(pair) = commands.windows(2).find(|pair| {
        (pair[0].page_index, pair[0].paint_ordinal) == (pair[1].page_index, pair[1].paint_ordinal)
    }) {
        return Err(StagingPrecomposedVectorDisplayError::PaintOrderCollision(
            pair[1].page_index,
            pair[1].paint_ordinal,
        ));
    }
    for (index, command) in commands.iter_mut().enumerate() {
        command.usage_id =
            u32::try_from(index).map_err(|_| StagingPrecomposedVectorDisplayError::CommandLimit)?;
        command.fingerprint = sha256(encode_command_record(command).as_bytes());
    }

    let page_count = commands
        .last()
        .map_or(Some(1u32), |command| command.page_index.checked_add(1))
        .ok_or(StagingPrecomposedVectorDisplayError::PageLimit)?;
    if page_count > limits.base().get().max_pages {
        return Err(StagingPrecomposedVectorDisplayError::PageLimit);
    }
    let page_capacity =
        usize::try_from(page_count).map_err(|_| StagingPrecomposedVectorDisplayError::PageLimit)?;
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(page_capacity)
        .map_err(|_| StagingPrecomposedVectorDisplayError::AllocationFailure)?;
    for page_index in 0..page_count {
        pages.push(StagingPrecomposedVectorDisplayPage {
            page_index,
            commands: Vec::new(),
            fingerprint: [0; 32],
        });
    }
    for command in commands {
        let page_index = usize::try_from(command.page_index)
            .map_err(|_| StagingPrecomposedVectorDisplayError::PageLimit)?;
        let page = pages
            .get_mut(page_index)
            .ok_or(StagingPrecomposedVectorDisplayError::PageLimit)?;
        page.commands
            .try_reserve(1)
            .map_err(|_| StagingPrecomposedVectorDisplayError::AllocationFailure)?;
        page.commands.push(command);
    }
    for page in &mut pages {
        page.fingerprint = sha256(encode_page_record(page).as_bytes());
    }
    let content_key_count = distinct_content_key_count(&pages)?;
    let mut receipt = StagingPrecomposedVectorDisplayReceipt {
        package_sha256: package.canonical_jcs_sha256(),
        profile_fingerprint: profile.profile_fingerprint(),
        limits_fingerprint: limits.fingerprint(),
        admitted_fingerprint: admitted.fingerprint().bytes(),
        binding_set_fingerprint: bindings.fingerprint(),
        inline_selected_layout_fingerprint: layout.inline_selected.receipt().fingerprint(),
        block_selected_layout_fingerprint: layout.block_selected.receipt().fingerprint(),
        page_geometry_fingerprint: layout.inline_selected.page_geometry().fingerprint(),
        page_count,
        command_count: command_count_u32,
        content_key_count,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_display(&receipt, &pages);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    Ok(StagingPrecomposedVectorDisplay { pages, receipt })
}

fn collect_inline_commands(
    commands: &mut Vec<StagingDrawVectorV2>,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
    selected: &StagingInlineVectorSelectedLayout,
) -> Result<(), StagingPrecomposedVectorDisplayError> {
    for placement in selected.placements() {
        let binding = binding_for(bindings, placement.node_id())?;
        if !matches!(
            binding.kind(),
            PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::MathVector
        ) || binding.fingerprint() != placement.binding_fingerprint()
        {
            return Err(StagingPrecomposedVectorDisplayError::SelectedMismatch);
        }
        let content_key = content_key_for(binding, admitted)?;
        let viewport = placement.viewport();
        let scale = placement.scale_raw();
        commands.push(StagingDrawVectorV2 {
            usage_id: 0,
            owner: placement.node_id(),
            kind: binding.kind(),
            image_id: binding.resource().image_id(),
            content_key,
            ir_fingerprint: binding.resource().ir_fingerprint(),
            binding_fingerprint: binding.fingerprint(),
            selected_placement_fingerprint: placement.fingerprint(),
            page_index: placement.page_index(),
            frame_index: placement.frame_index(),
            fragment_ordinal: placement.fragment_ordinal(),
            paint_ordinal: placement.paint_ordinal(),
            viewport,
            scale,
            matrix: placement_matrix(viewport, scale),
            resolved_current_color: binding_paint(binding),
            relation: StagingDrawVectorV2Relation::Inline {
                baseline_metrics: inline_baseline(binding, placement),
            },
            fingerprint: [0; 32],
        });
    }
    Ok(())
}

fn collect_block_commands(
    commands: &mut Vec<StagingDrawVectorV2>,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
    preparation: &StagingPrecomposedVectorBlockLayout,
    selected: &StagingAtomicVectorBlockSelectedLayout,
) -> Result<(), StagingPrecomposedVectorDisplayError> {
    for (placement, prepared) in selected.placements().iter().zip(preparation.blocks()) {
        let binding = binding_for(bindings, placement.owner())?;
        let expected_kind = match placement.kind() {
            StagingPreparedVectorBlockKind::VectorFigure => PrecomposedVectorKind::VectorFigure,
            StagingPreparedVectorBlockKind::MathVectorBlock => {
                PrecomposedVectorKind::MathVectorBlock
            }
        };
        if binding.kind() != expected_kind
            || binding.fingerprint() != placement.binding_fingerprint()
            || prepared.owner() != placement.owner()
        {
            return Err(StagingPrecomposedVectorDisplayError::SelectedMismatch);
        }
        let content_key = content_key_for(binding, admitted)?;
        let viewport = placement.viewport().rect();
        let relation = block_relation(binding, prepared, placement)?;
        commands.push(StagingDrawVectorV2 {
            usage_id: 0,
            owner: placement.owner(),
            kind: expected_kind,
            image_id: placement.image_id(),
            content_key,
            ir_fingerprint: binding.resource().ir_fingerprint(),
            binding_fingerprint: binding.fingerprint(),
            selected_placement_fingerprint: placement.fingerprint(),
            page_index: placement.page_index(),
            frame_index: placement.frame_index(),
            fragment_ordinal: placement.fragment_ordinal(),
            paint_ordinal: placement.viewport().paint_ordinal(),
            viewport,
            scale: placement.viewport().scale_raw(),
            matrix: placement.viewport().matrix(),
            resolved_current_color: binding_paint(binding),
            relation,
            fingerprint: [0; 32],
        });
    }
    if selected.placements().len() != preparation.blocks().len() {
        return Err(StagingPrecomposedVectorDisplayError::SelectedMismatch);
    }
    Ok(())
}

fn block_relation(
    binding: &ValidatedPrecomposedVectorReceipt,
    prepared: &StagingPreparedVectorBlock,
    placement: &StagingAtomicVectorBlockPlacement,
) -> Result<StagingDrawVectorV2Relation, StagingPrecomposedVectorDisplayError> {
    match placement.kind() {
        StagingPreparedVectorBlockKind::VectorFigure => {
            let caption_flow_id = prepared
                .caption_flow_id()
                .ok_or(StagingPrecomposedVectorDisplayError::SelectedMismatch)?;
            let mut caption_owners = Vec::new();
            caption_owners
                .try_reserve_exact(prepared.caption_owners().len())
                .map_err(|_| StagingPrecomposedVectorDisplayError::AllocationFailure)?;
            caption_owners.extend_from_slice(prepared.caption_owners());
            if placement
                .captions()
                .iter()
                .map(|caption| caption.owner())
                .ne(caption_owners.iter().copied())
            {
                return Err(StagingPrecomposedVectorDisplayError::SelectedMismatch);
            }
            Ok(StagingDrawVectorV2Relation::VectorFigure {
                figure_caption: StagingDrawVectorFigureCaption {
                    caption_flow_id,
                    caption_owners,
                    keep_caption: placement.keep_caption(),
                },
            })
        }
        StagingPreparedVectorBlockKind::MathVectorBlock => {
            let baseline = placement
                .math_baseline()
                .ok_or(StagingPrecomposedVectorDisplayError::SelectedMismatch)?;
            let flow = placement
                .math_flow()
                .ok_or(StagingPrecomposedVectorDisplayError::SelectedMismatch)?;
            Ok(StagingDrawVectorV2Relation::MathVectorBlock {
                baseline_metrics: StagingDrawVectorBaselineMetrics {
                    metric_receipt_fingerprint: binding.metrics_fingerprint(),
                    pen_origin_x: baseline.pen_origin_x(),
                    baseline: baseline.baseline(),
                    baseline_y: baseline.baseline_y(),
                },
                math_flow: StagingDrawVectorMathFlow {
                    flow_id: flow.flow_id(),
                    flow_fingerprint: flow.flow_fingerprint(),
                    parent_flow_id: placement.parent_flow_id(),
                    parent_position: placement.parent_position(),
                    terminal: flow.terminal(),
                    terminal_receipt_fingerprint: flow.terminal_receipt_fingerprint(),
                },
            })
        }
    }
}

fn inline_baseline(
    binding: &ValidatedPrecomposedVectorReceipt,
    placement: &StagingInlineVectorPlacement,
) -> StagingDrawVectorBaselineMetrics {
    StagingDrawVectorBaselineMetrics {
        metric_receipt_fingerprint: binding.metrics_fingerprint(),
        pen_origin_x: placement.pen_origin_x(),
        baseline: placement.baseline(),
        baseline_y: placement.baseline_y(),
    }
}

fn binding_for(
    bindings: &ValidatedPrecomposedVectorBindings,
    owner: NodeId,
) -> Result<&ValidatedPrecomposedVectorReceipt, StagingPrecomposedVectorDisplayError> {
    bindings
        .receipt(owner)
        .ok_or(StagingPrecomposedVectorDisplayError::SelectedMismatch)
}

fn content_key_for(
    binding: &ValidatedPrecomposedVectorReceipt,
    admitted: &AdmittedResourceLedger,
) -> Result<VectorContentKey, StagingPrecomposedVectorDisplayError> {
    let resource = binding.resource();
    let image = admitted.image(resource.image_id()).ok_or(
        StagingPrecomposedVectorDisplayError::ResourceMismatch(resource.image_id()),
    )?;
    let key = VectorContentKey::from_admitted(image)
        .map_err(|_| StagingPrecomposedVectorDisplayError::ResourceMismatch(resource.image_id()))?;
    let expected_media = match resource.admitted_media() {
        BoundPrecomposedVectorMedia::SafeSvg1 => VectorContentMediaType::SafeSvg1,
        BoundPrecomposedVectorMedia::SafeSvg2 => VectorContentMediaType::SafeSvg2,
    };
    if key.source_sha256() != resource.source_sha256()
        || key.media_type() != expected_media
        || key.parser_id() != resource.parser_id()
        || key.ir_id() != resource.ir_id()
        || key.ir_fingerprint() != resource.ir_fingerprint()
    {
        return Err(StagingPrecomposedVectorDisplayError::ResourceMismatch(
            resource.image_id(),
        ));
    }
    Ok(key)
}

fn binding_paint(binding: &ValidatedPrecomposedVectorReceipt) -> ResolvedRgb8 {
    match binding.placement() {
        PrecomposedVectorPlacementInput::Inline(value) => value.paint(),
        PrecomposedVectorPlacementInput::VectorFigure(value) => value.paint(),
        PrecomposedVectorPlacementInput::MathVectorBlock(value) => value.paint(),
    }
}

fn placement_matrix(viewport: Rect, scale: i32) -> AffineTransform {
    AffineTransform {
        a: Unitless16_16::from_raw(scale),
        b: Unitless16_16::from_raw(0),
        c: Unitless16_16::from_raw(0),
        d: Unitless16_16::from_raw(scale),
        e: viewport.x(),
        f: viewport.y(),
    }
}

fn command_is_closed(command: &StagingDrawVectorV2) -> bool {
    let matrix = command.matrix;
    if command.scale <= 0
        || command.content_key.ir_fingerprint() != command.ir_fingerprint
        || matrix.a.raw() != command.scale
        || matrix.b.raw() != 0
        || matrix.c.raw() != 0
        || matrix.d.raw() != command.scale
        || matrix.e != command.viewport.x()
        || matrix.f != command.viewport.y()
        || command.binding_fingerprint == [0; 32]
        || command.selected_placement_fingerprint == [0; 32]
    {
        return false;
    }
    match (&command.kind, &command.relation) {
        (
            PrecomposedVectorKind::InlineVector | PrecomposedVectorKind::MathVector,
            StagingDrawVectorV2Relation::Inline { baseline_metrics },
        ) => {
            baseline_metrics.metric_receipt_fingerprint != [0; 32]
                && command
                    .viewport
                    .y()
                    .checked_add(baseline_metrics.baseline.get())
                    == Some(baseline_metrics.baseline_y)
        }
        (
            PrecomposedVectorKind::VectorFigure,
            StagingDrawVectorV2Relation::VectorFigure { figure_caption },
        ) => figure_caption
            .caption_owners
            .windows(2)
            .all(|pair| pair[0] < pair[1]),
        (
            PrecomposedVectorKind::MathVectorBlock,
            StagingDrawVectorV2Relation::MathVectorBlock {
                baseline_metrics,
                math_flow,
            },
        ) => {
            baseline_metrics.metric_receipt_fingerprint != [0; 32]
                && command
                    .viewport
                    .y()
                    .checked_add(baseline_metrics.baseline.get())
                    == Some(baseline_metrics.baseline_y)
                && math_flow.terminal == MathVectorFlowTerminal::ONE
                && math_flow.flow_fingerprint != [0; 32]
                && math_flow.terminal_receipt_fingerprint != [0; 32]
        }
        _ => false,
    }
}

fn distinct_content_key_count(
    pages: &[StagingPrecomposedVectorDisplayPage],
) -> Result<u32, StagingPrecomposedVectorDisplayError> {
    let count = pages
        .iter()
        .try_fold(0usize, |total, page| total.checked_add(page.commands.len()))
        .ok_or(StagingPrecomposedVectorDisplayError::CommandLimit)?;
    let mut keys = Vec::new();
    keys.try_reserve_exact(count)
        .map_err(|_| StagingPrecomposedVectorDisplayError::AllocationFailure)?;
    keys.extend(
        pages
            .iter()
            .flat_map(|page| &page.commands)
            .map(|command| command.content_key),
    );
    keys.sort_unstable();
    keys.dedup();
    u32::try_from(keys.len()).map_err(|_| StagingPrecomposedVectorDisplayError::CommandLimit)
}

fn has_duplicate_selected_occurrence(
    pages: &[StagingPrecomposedVectorDisplayPage],
) -> Result<bool, StagingPrecomposedVectorDisplayError> {
    let count = pages
        .iter()
        .try_fold(0usize, |total, page| total.checked_add(page.commands.len()));
    let mut occurrences = Vec::new();
    occurrences
        .try_reserve_exact(count.ok_or(StagingPrecomposedVectorDisplayError::CommandLimit)?)
        .map_err(|_| StagingPrecomposedVectorDisplayError::AllocationFailure)?;
    occurrences.extend(
        pages
            .iter()
            .flat_map(|page| &page.commands)
            .map(|command| (command.owner, command.selected_placement_fingerprint)),
    );
    occurrences.sort_unstable_by_key(|(owner, _)| *owner);
    if occurrences.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Ok(true);
    }
    occurrences.sort_unstable_by_key(|(_, fingerprint)| *fingerprint);
    Ok(occurrences.windows(2).any(|pair| pair[0].1 == pair[1].1))
}

fn encode_display(
    receipt: &StagingPrecomposedVectorDisplayReceipt,
    pages: &[StagingPrecomposedVectorDisplayPage],
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, receipt.admitted_fingerprint);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, STAGING_DRAW_VECTOR_V2_ALGORITHM);
    output.push_str(",\"binding_set_fingerprint\":");
    push_hash(&mut output, receipt.binding_set_fingerprint);
    output.push_str(",\"block_selected_layout_fingerprint\":");
    push_hash(&mut output, receipt.block_selected_layout_fingerprint);
    output.push_str(",\"command_count\":");
    output.push_str(&receipt.command_count.to_string());
    output.push_str(",\"content_key_count\":");
    output.push_str(&receipt.content_key_count.to_string());
    output.push_str(",\"inline_selected_layout_fingerprint\":");
    push_hash(&mut output, receipt.inline_selected_layout_fingerprint);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, receipt.limits_fingerprint);
    output.push_str(",\"package_sha256\":");
    push_hash(&mut output, receipt.package_sha256);
    output.push_str(",\"page_count\":");
    output.push_str(&receipt.page_count.to_string());
    output.push_str(",\"page_geometry_fingerprint\":");
    push_hash(&mut output, receipt.page_geometry_fingerprint);
    output.push_str(",\"pages\":[");
    for (index, page) in pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"fingerprint\":");
        push_hash(&mut output, page.fingerprint);
        output.push_str(",\"record\":");
        output.push_str(&encode_page_record(page));
        output.push('}');
    }
    output.push_str("],\"profile_fingerprint\":");
    push_hash(&mut output, receipt.profile_fingerprint);
    output.push('}');
    output
}

fn encode_page_record(page: &StagingPrecomposedVectorDisplayPage) -> String {
    let mut output = String::from("{\"commands\":[");
    for (index, command) in page.commands.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"fingerprint\":");
        push_hash(&mut output, command.fingerprint);
        output.push_str(",\"record\":");
        output.push_str(&encode_command_record(command));
        output.push('}');
    }
    output.push_str("],\"page_index\":");
    output.push_str(&page.page_index.to_string());
    output.push('}');
    output
}

fn encode_command_record(command: &StagingDrawVectorV2) -> String {
    let mut output = String::from("{");
    if let Some(baseline) = command.relation.baseline_metrics() {
        output.push_str("\"baseline_metrics\":");
        push_baseline_metrics(&mut output, baseline);
        output.push(',');
    }
    output.push_str("\"binding_fingerprint\":");
    push_hash(&mut output, command.binding_fingerprint);
    output.push_str(",\"content_key\":");
    push_content_key(&mut output, command.content_key);
    if let Some(caption) = command.relation.figure_caption() {
        output.push_str(",\"figure_caption\":");
        push_figure_caption(&mut output, caption);
    }
    output.push_str(",\"fragment_ordinal\":");
    output.push_str(&command.fragment_ordinal.to_string());
    output.push_str(",\"frame_index\":");
    output.push_str(&command.frame_index.to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&command.image_id.get().to_string());
    output.push_str(",\"ir_fingerprint\":");
    push_hash(&mut output, command.ir_fingerprint);
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, command.kind.as_str());
    if let Some(flow) = command.relation.math_flow() {
        output.push_str(",\"math_flow\":");
        push_math_flow(&mut output, flow);
    }
    output.push_str(",\"matrix\":");
    push_matrix(&mut output, command.matrix);
    output.push_str(",\"op\":\"draw_vector\",\"owner\":");
    output.push_str(&command.owner.get().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&command.page_index.to_string());
    output.push_str(",\"paint_ordinal\":");
    output.push_str(&command.paint_ordinal.to_string());
    output.push_str(",\"resolved_current_color\":");
    push_color(&mut output, command.resolved_current_color);
    output.push_str(",\"scale\":");
    output.push_str(&command.scale.to_string());
    output.push_str(",\"selected_placement_fingerprint\":");
    push_hash(&mut output, command.selected_placement_fingerprint);
    output.push_str(",\"usage_id\":");
    output.push_str(&command.usage_id.to_string());
    output.push_str(",\"viewport\":");
    push_rect(&mut output, command.viewport);
    output.push('}');
    output
}

fn push_baseline_metrics(output: &mut String, value: StagingDrawVectorBaselineMetrics) {
    output.push_str("{\"baseline\":");
    output.push_str(&value.baseline.get().raw().to_string());
    output.push_str(",\"baseline_y\":");
    output.push_str(&value.baseline_y.raw().to_string());
    output.push_str(",\"metric_receipt_fingerprint\":");
    push_hash(output, value.metric_receipt_fingerprint);
    output.push_str(",\"pen_origin_x\":");
    output.push_str(&value.pen_origin_x.raw().to_string());
    output.push('}');
}

fn push_content_key(output: &mut String, value: VectorContentKey) {
    output.push_str("{\"ir_fingerprint\":");
    push_hash(output, value.ir_fingerprint());
    output.push_str(",\"ir_id\":");
    push_jcs_string(output, value.ir_id());
    output.push_str(",\"media_type\":");
    push_jcs_string(output, value.media_type().as_str());
    output.push_str(",\"parser_id\":");
    push_jcs_string(output, value.parser_id());
    output.push_str(",\"source_sha256\":");
    push_hash(output, value.source_sha256());
    output.push('}');
}

fn push_figure_caption(output: &mut String, value: &StagingDrawVectorFigureCaption) {
    output.push_str("{\"caption_flow_id\":");
    output.push_str(&value.caption_flow_id.get().to_string());
    output.push_str(",\"caption_owners\":[");
    for (index, owner) in value.caption_owners.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&owner.get().to_string());
    }
    output.push_str("],\"keep_caption\":");
    output.push_str(if value.keep_caption { "true" } else { "false" });
    output.push('}');
}

fn push_math_flow(output: &mut String, value: StagingDrawVectorMathFlow) {
    output.push_str("{\"flow_fingerprint\":");
    push_hash(output, value.flow_fingerprint);
    output.push_str(",\"flow_id\":");
    output.push_str(&value.flow_id.get().to_string());
    output.push_str(",\"parent_flow_id\":");
    output.push_str(&value.parent_flow_id.get().to_string());
    output.push_str(",\"parent_position\":");
    output.push_str(&value.parent_position.to_string());
    output.push_str(",\"terminal\":");
    output.push_str(&value.terminal.get().to_string());
    output.push_str(",\"terminal_receipt_fingerprint\":");
    push_hash(output, value.terminal_receipt_fingerprint);
    output.push('}');
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

fn push_color(output: &mut String, value: ResolvedRgb8) {
    output.push_str("{\"blue\":");
    output.push_str(&value.blue().to_string());
    output.push_str(",\"green\":");
    output.push_str(&value.green().to_string());
    output.push_str(",\"red\":");
    output.push_str(&value.red().to_string());
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
pub struct StagingPrecomposedVectorDisplayFixture {
    pub layout: typaxis_layout::StagingPrecomposedVectorBlockLayoutFixture,
    pub inline_input: Vec<StagingInlineVectorParagraphInput>,
    pub inline_selected: StagingInlineVectorSelectedLayout,
    pub block_input: StagingAtomicVectorBlockPaginationInput,
    pub block_selected: StagingAtomicVectorBlockSelectedLayout,
    pub display: StagingPrecomposedVectorDisplay,
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_precomposed_vector_display_fixture(
) -> Result<StagingPrecomposedVectorDisplayFixture, Box<dyn std::error::Error>> {
    use typaxis_core::{NonNegativeLength, PositiveLength};
    use typaxis_layout::{
        layout_staging_precomposed_vector_inlines, StagingInlineVectorLogicalUnit,
        StagingPrecomposedVectorBlockFixtureCase,
    };
    use typaxis_linebreak::{AtomicVectorTextUnit, JapaneseLineBreakMode};
    use typaxis_pagination::{
        paginate_staging_atomic_vector_blocks, StagingFigureCaptionBlockInput,
    };

    fn length(raw: i64) -> Length {
        Length::from_raw(raw).expect("positive fixture length")
    }
    fn positive(raw: i64) -> PositiveLength {
        PositiveLength::new(length(raw)).expect("positive fixture length")
    }
    fn nonnegative(raw: i64) -> NonNegativeLength {
        NonNegativeLength::new(length(raw)).expect("nonnegative fixture length")
    }
    fn text(scalar: char, advance: i64) -> StagingInlineVectorLogicalUnit {
        StagingInlineVectorLogicalUnit::Text(AtomicVectorTextUnit::new(
            scalar,
            nonnegative(advance),
            nonnegative(655_360),
            nonnegative(196_608),
        ))
    }

    let layout = typaxis_layout::staging_precomposed_vector_block_layout_fixture_for_case(
        StagingPrecomposedVectorBlockFixtureCase::DisplayV2,
    )?;
    let inline_input = vec![StagingInlineVectorParagraphInput::new(
        NodeId::new(2),
        vec![
            text('日', 1_000_000),
            StagingInlineVectorLogicalUnit::Vector(NodeId::new(3)),
            text('、', 500_000),
            StagingInlineVectorLogicalUnit::Vector(NodeId::new(4)),
            text('。', 500_000),
        ],
        positive(1_310_720),
        JapaneseLineBreakMode::Normal,
    )];
    let inline_selected = layout_staging_precomposed_vector_inlines(
        &layout.package,
        &layout.profile,
        &layout.limits,
        &layout.admitted,
        &layout.bindings,
        &inline_input,
    )?;
    let block_input = StagingAtomicVectorBlockPaginationInput::new(
        &layout.layout,
        nonnegative(80 * 65_536),
        inline_selected.receipt().fragment_charge(),
        vec![StagingFigureCaptionBlockInput::new(
            NodeId::new(6),
            positive(20 * 65_536),
        )],
        Vec::new(),
    )?;
    let block_selected = paginate_staging_atomic_vector_blocks(
        &layout.layout,
        &layout.math_flows,
        &block_input,
        &layout.limits,
    )?;
    let source = StagingPrecomposedVectorDisplayLayoutInput::new(
        &inline_input,
        &inline_selected,
        &layout.layout,
        &layout.math_flows,
        &block_input,
        &block_selected,
    );
    let display = build_staging_precomposed_vector_display(
        &layout.package,
        &layout.profile,
        &layout.limits,
        &layout.admitted,
        &layout.bindings,
        &source,
    )?;
    Ok(StagingPrecomposedVectorDisplayFixture {
        layout,
        inline_input,
        inline_selected,
        block_input,
        block_selected,
        display,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(
        fixture: &StagingPrecomposedVectorDisplayFixture,
    ) -> StagingPrecomposedVectorDisplayLayoutInput<'_> {
        StagingPrecomposedVectorDisplayLayoutInput::new(
            &fixture.inline_input,
            &fixture.inline_selected,
            &fixture.layout.layout,
            &fixture.layout.math_flows,
            &fixture.block_input,
            &fixture.block_selected,
        )
    }

    fn reseal(display: &mut StagingPrecomposedVectorDisplay) {
        for page in &mut display.pages {
            for command in &mut page.commands {
                command.fingerprint = sha256(encode_command_record(command).as_bytes());
            }
            page.fingerprint = sha256(encode_page_record(page).as_bytes());
        }
        display.receipt.command_count = display
            .pages
            .iter()
            .map(|page| u32::try_from(page.commands.len()).unwrap())
            .sum();
        display.receipt.content_key_count = distinct_content_key_count(&display.pages).unwrap();
        display.receipt.canonical_jcs = encode_display(&display.receipt, &display.pages);
        display.receipt.fingerprint = sha256(display.receipt.canonical_jcs.as_bytes());
    }

    #[test]
    fn draw_vector_v2_closes_all_four_kinds_and_one_content_key() {
        let fixture = staging_precomposed_vector_display_fixture().unwrap();
        assert_eq!(
            fixture.display.trace_json(),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../samples/machine-package/staging/production-book-1/",
                "precomposed-vector/display-v2.json"
            ))
            .trim_end()
        );
        let commands = fixture.display.commands().collect::<Vec<_>>();
        assert_eq!(
            fixture.display.receipt().algorithm(),
            STAGING_DRAW_VECTOR_V2_ALGORITHM
        );
        assert_eq!(fixture.display.receipt().command_count(), 4);
        assert_eq!(fixture.display.receipt().content_key_count(), 1);
        assert_eq!(
            commands
                .iter()
                .map(|command| command.kind())
                .collect::<Vec<_>>(),
            [
                PrecomposedVectorKind::InlineVector,
                PrecomposedVectorKind::MathVector,
                PrecomposedVectorKind::VectorFigure,
                PrecomposedVectorKind::MathVectorBlock,
            ]
        );
        assert_eq!(
            commands
                .iter()
                .map(|command| (command.page_index(), command.paint_ordinal()))
                .collect::<Vec<_>>(),
            [(0, 0), (0, 1), (1, 0), (1, 2)]
        );
        assert!(commands
            .iter()
            .all(|command| command.content_key() == commands[0].content_key()));
        assert!(fixture
            .layout
            .admitted
            .image(ImageResourceId::new(1))
            .is_some());
        assert!(commands
            .iter()
            .all(|command| command.image_id() != ImageResourceId::new(1)));
        assert_eq!(
            commands[2]
                .relation()
                .figure_caption()
                .unwrap()
                .caption_owners(),
            [NodeId::new(6)]
        );
        let math_flow = commands[3].relation().math_flow().unwrap();
        assert_eq!(math_flow.parent_flow_id(), FlowId::new(1));
        assert_eq!(math_flow.parent_position(), 2);
        assert_eq!(math_flow.terminal().get(), 1);
        fixture
            .display
            .verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.bindings,
                &source(&fixture),
            )
            .unwrap();
    }

    #[test]
    fn draw_vector_v2_is_independent_of_selected_collection_order() {
        let fixture = staging_precomposed_vector_display_fixture().unwrap();
        let reversed = build_display(
            &fixture.layout.package,
            &fixture.layout.profile,
            &fixture.layout.limits,
            &fixture.layout.admitted,
            &fixture.layout.bindings,
            &source(&fixture),
            false,
        )
        .unwrap();
        assert_eq!(fixture.display, reversed);
    }

    #[test]
    fn precomposed_vector_display_tamper_rejects_geometry_kind_and_order() {
        let fixture = staging_precomposed_vector_display_fixture().unwrap();
        let mut baseline = fixture.display.clone();
        let StagingDrawVectorV2Relation::Inline { baseline_metrics } =
            &mut baseline.pages[0].commands[0].relation
        else {
            panic!("first fixture command must be inline");
        };
        baseline_metrics.baseline_y = baseline_metrics
            .baseline_y
            .checked_add(Length::from_raw(1).unwrap())
            .unwrap();
        reseal(&mut baseline);
        assert_eq!(
            baseline.verify_resource_closure(),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut viewport = fixture.display.clone();
        let original_viewport = viewport.pages[0].commands[0].viewport;
        viewport.pages[0].commands[0].viewport = Rect::new(
            original_viewport
                .x()
                .checked_add(Length::from_raw(1).unwrap())
                .unwrap(),
            original_viewport.y(),
            original_viewport.width(),
            original_viewport.height(),
        );
        reseal(&mut viewport);
        assert_eq!(
            viewport.verify_resource_closure(),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut matrix = fixture.display.clone();
        matrix.pages[0].commands[0].matrix.e = matrix.pages[0].commands[0]
            .matrix
            .e
            .checked_add(Length::from_raw(1).unwrap())
            .unwrap();
        reseal(&mut matrix);
        assert_eq!(
            matrix.verify_resource_closure(),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut wrong_page = fixture.display.clone();
        wrong_page.pages[0].commands[0].page_index = 1;
        reseal(&mut wrong_page);
        assert_eq!(
            wrong_page.verify_resource_closure(),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut wrong_kind = fixture.display.clone();
        wrong_kind.pages[0].commands[0].kind = PrecomposedVectorKind::VectorFigure;
        reseal(&mut wrong_kind);
        assert_eq!(
            wrong_kind.verify_resource_closure(),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut reordered = fixture.display.clone();
        reordered.pages[0].commands.swap(0, 1);
        reseal(&mut reordered);
        assert_eq!(
            reordered.verify_resource_closure(),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut duplicate = fixture.display.clone();
        let mut repeated = duplicate.pages[1].commands[1].clone();
        repeated.usage_id = 4;
        repeated.paint_ordinal = 4;
        duplicate.pages[1].commands.push(repeated);
        reseal(&mut duplicate);
        assert_eq!(
            duplicate.verify_resource_closure(),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut missing = fixture.display.clone();
        missing.pages[0].commands.remove(0);
        reseal(&mut missing);
        assert_eq!(
            missing.verify_resource_closure(),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );
    }

    #[test]
    fn precomposed_vector_display_tamper_rejects_resealed_selected_and_resource_state() {
        let fixture = staging_precomposed_vector_display_fixture().unwrap();
        let mut selected = fixture.display.clone();
        selected.pages[0].commands[0].selected_placement_fingerprint[0] ^= 1;
        reseal(&mut selected);
        assert!(selected.verify_resource_closure().is_ok());
        assert_eq!(
            selected.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.bindings,
                &source(&fixture),
            ),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut wrong_owner = fixture.display.clone();
        wrong_owner.pages[0].commands[0].owner = NodeId::new(99);
        reseal(&mut wrong_owner);
        assert!(wrong_owner.verify_resource_closure().is_ok());
        assert_eq!(
            wrong_owner.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.bindings,
                &source(&fixture),
            ),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut wrong_image = fixture.display.clone();
        wrong_image.pages[0].commands[0].image_id = ImageResourceId::new(1);
        reseal(&mut wrong_image);
        assert!(wrong_image.verify_resource_closure().is_ok());
        assert_eq!(
            wrong_image.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.bindings,
                &source(&fixture),
            ),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let replacement_key = VectorContentKey::from_admitted(
            fixture
                .layout
                .admitted
                .image(ImageResourceId::new(1))
                .unwrap(),
        )
        .unwrap();
        assert_ne!(
            replacement_key,
            fixture.display.pages[0].commands[0].content_key
        );
        let mut wrong_key = fixture.display.clone();
        wrong_key.pages[0].commands[0].content_key = replacement_key;
        wrong_key.pages[0].commands[0].ir_fingerprint = replacement_key.ir_fingerprint();
        reseal(&mut wrong_key);
        assert!(wrong_key.verify_resource_closure().is_ok());
        assert_eq!(
            wrong_key.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.bindings,
                &source(&fixture),
            ),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut viewport_extent = fixture.display.clone();
        let original_viewport = viewport_extent.pages[0].commands[0].viewport;
        viewport_extent.pages[0].commands[0].viewport = Rect::new(
            original_viewport.x(),
            original_viewport.y(),
            original_viewport
                .width()
                .get()
                .checked_add(Length::from_raw(1).unwrap())
                .and_then(typaxis_core::PositiveLength::new)
                .unwrap(),
            original_viewport.height(),
        );
        reseal(&mut viewport_extent);
        assert!(viewport_extent.verify_resource_closure().is_ok());
        assert_eq!(
            viewport_extent.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.bindings,
                &source(&fixture),
            ),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut parent_position = fixture.display.clone();
        let math_flow = match &mut parent_position.pages[1].commands[1].relation {
            StagingDrawVectorV2Relation::MathVectorBlock { math_flow, .. } => math_flow,
            _ => panic!("last fixture command must be block math"),
        };
        math_flow.parent_position += 1;
        reseal(&mut parent_position);
        assert!(parent_position.verify_resource_closure().is_ok());
        assert_eq!(
            parent_position.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.bindings,
                &source(&fixture),
            ),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );

        let mut ir = fixture.display.clone();
        ir.pages[0].commands[0].ir_fingerprint[0] ^= 1;
        reseal(&mut ir);
        assert_eq!(
            ir.verify_resource_closure(),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );
    }

    #[test]
    fn draw_vector_v2_keeps_current_color_out_of_the_content_key() {
        let fixture = staging_precomposed_vector_display_fixture().unwrap();
        let mut changed = fixture.display.clone();
        let key = changed.pages[0].commands[0].content_key;
        let original_fingerprint = changed.receipt.fingerprint;
        changed.pages[0].commands[0].resolved_current_color = ResolvedRgb8::new(12, 34, 56);
        assert_eq!(changed.pages[0].commands[0].content_key, key);
        reseal(&mut changed);
        assert_ne!(changed.receipt.fingerprint, original_fingerprint);
        assert_eq!(changed.receipt.content_key_count, 1);
        assert!(changed.commands().all(|command| command.content_key == key));
        assert!(changed
            .commands()
            .any(|command| command.resolved_current_color != ResolvedRgb8::BLACK));
        assert!(changed
            .commands()
            .any(|command| command.resolved_current_color == ResolvedRgb8::BLACK));
        assert!(changed.commands().any(|command| command.page_index == 1));
        assert!(changed
            .commands()
            .any(|command| command.kind == PrecomposedVectorKind::MathVectorBlock));
        assert!(changed.verify_resource_closure().is_ok());
        assert_eq!(
            changed.verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.admitted,
                &fixture.layout.bindings,
                &source(&fixture),
            ),
            Err(StagingPrecomposedVectorDisplayError::ReceiptMismatch)
        );
    }
}
