use crate::{ValidatedPrecomposedVectorBindings, ValidatedPrecomposedVectorReceipt};
use typaxis_core::{
    push_jcs_string, sha256, Length, M4EffectiveResourceLimits, NodeId, NonNegativeLength,
    PositiveLength, Rect,
};
use typaxis_document::{StagingM4Block, StagingM4InlineVectorKind};
use typaxis_layout_contract::{PrecomposedVectorGeometryError, PrecomposedVectorPlacementInput};
use typaxis_linebreak::{
    break_atomic_vector_inline, AtomicVectorInlineBreak, AtomicVectorInlineError,
    AtomicVectorInlineItem, AtomicVectorInlineKind, AtomicVectorInlineLogicalUnit,
    AtomicVectorInlineParagraph, AtomicVectorTextUnit, JapaneseLineBreakMode,
};
use typaxis_resource_admission::AdmittedResourceLedger;
use typaxis_syntax::{
    machine_profile_boundary::wire::{
        WireStagingM4Block, WireStagingM4Inline, WireStagingM4TextBuffer, WireStagingTextSpan,
    },
    PrecomposedVectorKind, StagingM4PageGeometry, StagingPrecomposedVectorProfileAuthorization,
    ValidatedStagingSemanticPackage,
};

pub const PRECOMPOSED_VECTOR_SELECTED_LAYOUT_ALGORITHM: &str =
    "typaxis.precomposed-vector-layout/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingInlineVectorLogicalUnit {
    Text(AtomicVectorTextUnit),
    Vector(NodeId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingInlineVectorParagraphInput {
    paragraph_node: NodeId,
    units: Vec<StagingInlineVectorLogicalUnit>,
    computed_line_height: PositiveLength,
    japanese_mode: JapaneseLineBreakMode,
}

impl StagingInlineVectorParagraphInput {
    pub fn new(
        paragraph_node: NodeId,
        units: Vec<StagingInlineVectorLogicalUnit>,
        computed_line_height: PositiveLength,
        japanese_mode: JapaneseLineBreakMode,
    ) -> Self {
        Self {
            paragraph_node,
            units,
            computed_line_height,
            japanese_mode,
        }
    }

    pub const fn paragraph_node(&self) -> NodeId {
        self.paragraph_node
    }

    pub fn units(&self) -> &[StagingInlineVectorLogicalUnit] {
        &self.units
    }

    pub const fn computed_line_height(&self) -> PositiveLength {
        self.computed_line_height
    }

    pub const fn japanese_mode(&self) -> JapaneseLineBreakMode {
        self.japanese_mode
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingInlineVectorLine {
    line_index: u32,
    paragraph_node: NodeId,
    paragraph_line_index: u32,
    start_unit: u32,
    end_unit: u32,
    page_index: u32,
    frame_index: u32,
    fragment_ordinal: u32,
    line_top: Length,
    baseline_y: Length,
    break_demerits: i64,
    line_height: PositiveLength,
    content_ascent: NonNegativeLength,
    content_descent: NonNegativeLength,
    leading_before: NonNegativeLength,
    leading_after: NonNegativeLength,
    logical_advance: NonNegativeLength,
    visual_left: Option<Length>,
    visual_right: Option<Length>,
    itemization_fingerprint: [u8; 32],
    line_selection_fingerprint: [u8; 32],
    fingerprint: [u8; 32],
}

impl StagingInlineVectorLine {
    pub const fn line_index(&self) -> u32 {
        self.line_index
    }

    pub const fn paragraph_node(&self) -> NodeId {
        self.paragraph_node
    }

    pub const fn paragraph_line_index(&self) -> u32 {
        self.paragraph_line_index
    }

    pub const fn start_unit(&self) -> u32 {
        self.start_unit
    }

    pub const fn end_unit(&self) -> u32 {
        self.end_unit
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

    pub const fn line_top(&self) -> Length {
        self.line_top
    }

    pub const fn baseline_y(&self) -> Length {
        self.baseline_y
    }

    pub const fn break_demerits(&self) -> i64 {
        self.break_demerits
    }

    pub const fn line_height(&self) -> PositiveLength {
        self.line_height
    }

    pub const fn content_ascent(&self) -> NonNegativeLength {
        self.content_ascent
    }

    pub const fn content_descent(&self) -> NonNegativeLength {
        self.content_descent
    }

    pub const fn leading_before(&self) -> NonNegativeLength {
        self.leading_before
    }

    pub const fn leading_after(&self) -> NonNegativeLength {
        self.leading_after
    }

    pub const fn logical_advance(&self) -> NonNegativeLength {
        self.logical_advance
    }

    pub const fn visual_left(&self) -> Option<Length> {
        self.visual_left
    }

    pub const fn visual_right(&self) -> Option<Length> {
        self.visual_right
    }

    pub const fn itemization_fingerprint(&self) -> [u8; 32] {
        self.itemization_fingerprint
    }

    pub const fn line_selection_fingerprint(&self) -> [u8; 32] {
        self.line_selection_fingerprint
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingInlineVectorPlacement {
    occurrence: u32,
    node_id: NodeId,
    paragraph_node: NodeId,
    binding_fingerprint: [u8; 32],
    atomic_item_fingerprint: [u8; 32],
    line_index: u32,
    page_index: u32,
    frame_index: u32,
    fragment_ordinal: u32,
    paint_ordinal: u32,
    pen_origin_x: Length,
    baseline_y: Length,
    baseline: NonNegativeLength,
    viewport: Rect,
    scale: i32,
    spacing_before: NonNegativeLength,
    spacing_after: NonNegativeLength,
    fingerprint: [u8; 32],
}

impl StagingInlineVectorPlacement {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }

    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub const fn paragraph_node(&self) -> NodeId {
        self.paragraph_node
    }

    pub const fn binding_fingerprint(&self) -> [u8; 32] {
        self.binding_fingerprint
    }

    pub const fn atomic_item_fingerprint(&self) -> [u8; 32] {
        self.atomic_item_fingerprint
    }

    pub const fn line_index(&self) -> u32 {
        self.line_index
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

    pub const fn pen_origin_x(&self) -> Length {
        self.pen_origin_x
    }

    pub const fn baseline_y(&self) -> Length {
        self.baseline_y
    }

    pub const fn baseline(&self) -> NonNegativeLength {
        self.baseline
    }

    pub const fn viewport(&self) -> Rect {
        self.viewport
    }

    pub const fn scale_raw(&self) -> i32 {
        self.scale
    }

    pub const fn spacing_before(&self) -> NonNegativeLength {
        self.spacing_before
    }

    pub const fn spacing_after(&self) -> NonNegativeLength {
        self.spacing_after
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StagingInlineVectorParagraphFact {
    paragraph_node: NodeId,
    itemization_fingerprint: [u8; 32],
    line_selection_fingerprint: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingInlineVectorSelectedLayoutReceipt {
    package_sha256: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    binding_set_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    page_geometry_fingerprint: [u8; 32],
    line_count: u32,
    placement_count: u32,
    fragment_charge: u64,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingInlineVectorSelectedLayoutReceipt {
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

    pub const fn page_geometry_fingerprint(&self) -> [u8; 32] {
        self.page_geometry_fingerprint
    }

    pub const fn line_count(&self) -> u32 {
        self.line_count
    }

    pub const fn placement_count(&self) -> u32 {
        self.placement_count
    }

    pub const fn fragment_charge(&self) -> u64 {
        self.fragment_charge
    }

    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingInlineVectorSelectedLayout {
    paragraphs: Vec<AtomicVectorInlineParagraph>,
    breaks: Vec<AtomicVectorInlineBreak>,
    paragraph_facts: Vec<StagingInlineVectorParagraphFact>,
    lines: Vec<StagingInlineVectorLine>,
    placements: Vec<StagingInlineVectorPlacement>,
    page_geometry: StagingM4PageGeometry,
    receipt: StagingInlineVectorSelectedLayoutReceipt,
}

impl StagingInlineVectorSelectedLayout {
    pub fn lines(&self) -> &[StagingInlineVectorLine] {
        &self.lines
    }

    pub fn placements(&self) -> &[StagingInlineVectorPlacement] {
        &self.placements
    }

    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        &self.page_geometry
    }

    pub const fn receipt(&self) -> &StagingInlineVectorSelectedLayoutReceipt {
        &self.receipt
    }

    pub fn trace_json(&self) -> String {
        let mut output = String::from(
            "{\"contract\":\"typaxis.contract/1.4\",\"coordinate_unit\":\"pdf_point_1_65536\",\"precomposed_vector_layout\":",
        );
        output.push_str(self.receipt.canonical_jcs());
        output.push('}');
        output
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingPrecomposedVectorProfileAuthorization,
        limits: &M4EffectiveResourceLimits,
        admitted: &AdmittedResourceLedger,
        bindings: &ValidatedPrecomposedVectorBindings,
        input: &[StagingInlineVectorParagraphInput],
    ) -> Result<(), StagingInlineVectorLayoutError> {
        let expected = build_staging_inline_vector_layout(
            package, profile, limits, admitted, bindings, input,
        )?;
        if self != &expected {
            return Err(StagingInlineVectorLayoutError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StagingInlineVectorLayoutError {
    BindingMismatch,
    InputMismatch(NodeId),
    Atomic(AtomicVectorInlineError),
    Oversize(NodeId),
    PlacementLimit,
    PageLimit,
    ArithmeticOverflow,
    AllocationFailure,
    ReceiptMismatch,
}

impl std::fmt::Display for StagingInlineVectorLayoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BindingMismatch => {
                formatter.write_str("I9190: inline vector binding set mismatch")
            }
            Self::InputMismatch(owner) => write!(
                formatter,
                "I9190: inline vector layout input mismatch at node {}",
                owner.get()
            ),
            Self::Atomic(error) => std::fmt::Display::fmt(error, formatter),
            Self::Oversize(owner) => write!(
                formatter,
                "L5100: inline vector line at node {} exceeds an empty frame",
                owner.get()
            ),
            Self::PlacementLimit => {
                formatter.write_str("L5110: inline vector fragment limit exceeded")
            }
            Self::PageLimit => formatter.write_str("L5100: inline vector page limit exceeded"),
            Self::ArithmeticOverflow => {
                formatter.write_str("L5100: inline vector layout arithmetic overflow")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5100: inline vector layout allocation failed")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: inline vector selected layout mismatch")
            }
        }
    }
}

impl std::error::Error for StagingInlineVectorLayoutError {}

impl From<AtomicVectorInlineError> for StagingInlineVectorLayoutError {
    fn from(value: AtomicVectorInlineError) -> Self {
        Self::Atomic(value)
    }
}

pub fn layout_staging_precomposed_vector_inlines(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
    input: &[StagingInlineVectorParagraphInput],
) -> Result<StagingInlineVectorSelectedLayout, StagingInlineVectorLayoutError> {
    let selected =
        build_staging_inline_vector_layout(package, profile, limits, admitted, bindings, input)?;
    if !selected.integrity_matches(limits) {
        return Err(StagingInlineVectorLayoutError::ReceiptMismatch);
    }
    Ok(selected)
}

/// Derive the atomic inline-vector input directly from the checked 1.4 wire
/// carrier. This is the public producer-to-layout bridge: vector nodes remain
/// indivisible, while surrounding authored text participates in Unicode and
/// Japanese line breaking with deterministic fallback metrics.
pub fn prepare_staging_precomposed_vector_inline_inputs(
    package: &ValidatedStagingSemanticPackage,
) -> Result<Vec<StagingInlineVectorParagraphInput>, StagingInlineVectorLayoutError> {
    const TEXT_ADVANCE: i64 = 10 * 65_536;
    const TEXT_ASCENT: i64 = 8 * 65_536;
    const TEXT_DESCENT: i64 = 2 * 65_536;
    const LINE_HEIGHT: i64 = 20 * 65_536;

    let wire = package
        .checked_wire()
        .map_err(|_| StagingInlineVectorLayoutError::ReceiptMismatch)?;
    let text_buffers = wire.text_buffers();
    let mut output = Vec::new();
    collect_wire_inline_inputs(
        &wire.document().blocks,
        text_buffers,
        &mut output,
        TEXT_ADVANCE,
        TEXT_ASCENT,
        TEXT_DESCENT,
        LINE_HEIGHT,
    )?;
    for footnote in &wire.document().footnotes {
        collect_wire_inline_inputs(
            &footnote.blocks,
            text_buffers,
            &mut output,
            TEXT_ADVANCE,
            TEXT_ASCENT,
            TEXT_DESCENT,
            LINE_HEIGHT,
        )?;
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn collect_wire_inline_inputs(
    blocks: &[WireStagingM4Block],
    text_buffers: &[WireStagingM4TextBuffer],
    output: &mut Vec<StagingInlineVectorParagraphInput>,
    text_advance: i64,
    text_ascent: i64,
    text_descent: i64,
    line_height: i64,
) -> Result<(), StagingInlineVectorLayoutError> {
    for block in blocks {
        match block {
            WireStagingM4Block::Paragraph {
                node_id, children, ..
            }
            | WireStagingM4Block::Heading {
                node_id, children, ..
            } => {
                let mut units = Vec::new();
                collect_wire_inline_units(
                    children,
                    text_buffers,
                    &mut units,
                    text_advance,
                    text_ascent,
                    text_descent,
                )?;
                if units
                    .iter()
                    .any(|unit| matches!(unit, StagingInlineVectorLogicalUnit::Vector(_)))
                {
                    output.push(StagingInlineVectorParagraphInput::new(
                        NodeId::new(*node_id),
                        units,
                        positive_raw(line_height)?,
                        JapaneseLineBreakMode::Normal,
                    ));
                }
            }
            WireStagingM4Block::List { items, .. } => {
                for item in items {
                    collect_wire_inline_inputs(
                        &item.blocks,
                        text_buffers,
                        output,
                        text_advance,
                        text_ascent,
                        text_descent,
                        line_height,
                    )?;
                }
            }
            WireStagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_wire_inline_inputs(
                        &cell.blocks,
                        text_buffers,
                        output,
                        text_advance,
                        text_ascent,
                        text_descent,
                        line_height,
                    )?;
                }
            }
            WireStagingM4Block::Figure { caption, .. }
            | WireStagingM4Block::VectorFigure { caption, .. } => {
                collect_wire_inline_inputs(
                    caption,
                    text_buffers,
                    output,
                    text_advance,
                    text_ascent,
                    text_descent,
                    line_height,
                )?;
            }
            WireStagingM4Block::SemanticContainer { blocks, .. } => {
                collect_wire_inline_inputs(
                    blocks,
                    text_buffers,
                    output,
                    text_advance,
                    text_ascent,
                    text_descent,
                    line_height,
                )?;
            }
            WireStagingM4Block::PageBreak { .. }
            | WireStagingM4Block::DisplayMath { .. }
            | WireStagingM4Block::MathVectorBlock { .. } => {}
        }
    }
    Ok(())
}

fn collect_wire_inline_units(
    inlines: &[WireStagingM4Inline],
    text_buffers: &[WireStagingM4TextBuffer],
    output: &mut Vec<StagingInlineVectorLogicalUnit>,
    advance: i64,
    ascent: i64,
    descent: i64,
) -> Result<(), StagingInlineVectorLayoutError> {
    for inline in inlines {
        match inline {
            WireStagingM4Inline::InlineVector { node_id, .. }
            | WireStagingM4Inline::MathVector { node_id, .. } => {
                output.push(StagingInlineVectorLogicalUnit::Vector(NodeId::new(
                    *node_id,
                )));
            }
            WireStagingM4Inline::Text { text_span, .. } => {
                push_text_span_units(*text_span, text_buffers, output, advance, ascent, descent)?
            }
            WireStagingM4Inline::InlineMath { math_source, .. } => push_text_span_units(
                math_source.text_span,
                text_buffers,
                output,
                advance,
                ascent,
                descent,
            )?,
            WireStagingM4Inline::Emphasis { children, .. }
            | WireStagingM4Inline::Strong { children, .. }
            | WireStagingM4Inline::Link { children, .. } => {
                collect_wire_inline_units(children, text_buffers, output, advance, ascent, descent)?
            }
            WireStagingM4Inline::Reference { .. } => {
                push_scalar_unit('A', output, advance, ascent, descent)?;
            }
            WireStagingM4Inline::FootnoteReference { .. } => {
                push_scalar_unit('*', output, advance, ascent, descent)?;
            }
            WireStagingM4Inline::SoftBreak { .. } => {
                push_scalar_unit(' ', output, advance, ascent, descent)?;
            }
            WireStagingM4Inline::HardBreak { .. } => {
                push_scalar_unit('\n', output, advance, ascent, descent)?;
            }
            WireStagingM4Inline::Anchor { .. } => {}
        }
    }
    Ok(())
}

fn push_text_span_units(
    span: WireStagingTextSpan,
    buffers: &[WireStagingM4TextBuffer],
    output: &mut Vec<StagingInlineVectorLogicalUnit>,
    advance: i64,
    ascent: i64,
    descent: i64,
) -> Result<(), StagingInlineVectorLayoutError> {
    let buffer = buffers
        .get(span.text_id as usize)
        .filter(|buffer| buffer.text_id == span.text_id)
        .ok_or(StagingInlineVectorLayoutError::ReceiptMismatch)?;
    let start = usize::try_from(span.start_byte)
        .map_err(|_| StagingInlineVectorLayoutError::ArithmeticOverflow)?;
    let end = usize::try_from(span.end_byte)
        .map_err(|_| StagingInlineVectorLayoutError::ArithmeticOverflow)?;
    let text = buffer
        .utf8
        .get(start..end)
        .ok_or(StagingInlineVectorLayoutError::ReceiptMismatch)?;
    for scalar in text.chars() {
        push_scalar_unit(scalar, output, advance, ascent, descent)?;
    }
    Ok(())
}

fn push_scalar_unit(
    scalar: char,
    output: &mut Vec<StagingInlineVectorLogicalUnit>,
    advance: i64,
    ascent: i64,
    descent: i64,
) -> Result<(), StagingInlineVectorLayoutError> {
    output.push(StagingInlineVectorLogicalUnit::Text(
        AtomicVectorTextUnit::new(
            scalar,
            nonnegative_raw(advance)?,
            nonnegative_raw(ascent)?,
            nonnegative_raw(descent)?,
        ),
    ));
    Ok(())
}

fn nonnegative_raw(value: i64) -> Result<NonNegativeLength, StagingInlineVectorLayoutError> {
    NonNegativeLength::new(raw_length(value)?)
        .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)
}

fn positive_raw(value: i64) -> Result<PositiveLength, StagingInlineVectorLayoutError> {
    PositiveLength::new(raw_length(value)?)
        .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)
}

impl StagingInlineVectorSelectedLayout {
    fn integrity_matches(&self, limits: &M4EffectiveResourceLimits) -> bool {
        let observed_fragment_charge = u64::try_from(self.lines.len()).ok().and_then(|lines| {
            u64::try_from(self.placements.len())
                .ok()
                .and_then(|placements| lines.checked_add(placements))
        });
        let canonical = encode_selected_layout(
            self.receipt.package_sha256,
            self.receipt.profile_fingerprint,
            self.receipt.limits_fingerprint,
            self.receipt.admitted_fingerprint,
            self.receipt.binding_set_fingerprint,
            self.receipt.layout_epoch_fingerprint,
            &self.page_geometry,
            &self.paragraph_facts,
            &self.lines,
            &self.placements,
            self.receipt.fragment_charge,
        );
        self.paragraphs.len() == self.breaks.len()
            && self.paragraphs.len() == self.paragraph_facts.len()
            && self
                .paragraphs
                .iter()
                .zip(&self.breaks)
                .zip(&self.paragraph_facts)
                .all(|((paragraph, selected), fact)| {
                    paragraph.paragraph_node() == fact.paragraph_node
                        && paragraph.fingerprint() == fact.itemization_fingerprint
                        && selected.itemization_fingerprint() == paragraph.fingerprint()
                        && selected.fingerprint() == fact.line_selection_fingerprint
                })
            && usize::try_from(self.receipt.line_count) == Ok(self.lines.len())
            && usize::try_from(self.receipt.placement_count) == Ok(self.placements.len())
            && observed_fragment_charge == Some(self.receipt.fragment_charge)
            && self.receipt.fragment_charge <= limits.base().get().max_fragments
            && self.receipt.page_geometry_fingerprint == self.page_geometry.fingerprint()
            && self.receipt.canonical_jcs == canonical
            && self.receipt.fingerprint == sha256(canonical.as_bytes())
            && self.lines.iter().enumerate().all(|(index, line)| {
                usize::try_from(line.line_index) == Ok(index)
                    && line.fragment_ordinal == line.line_index
                    && line.frame_index == 0
                    && line.fingerprint == sha256(encode_line_payload(line).as_bytes())
            })
            && self
                .placements
                .iter()
                .enumerate()
                .all(|(index, placement)| {
                    usize::try_from(placement.occurrence) == Ok(index)
                        && placement.paint_ordinal == placement.occurrence
                        && placement.frame_index == 0
                        && placement.scale > 0
                        && placement.viewport.y().checked_add(placement.baseline.get())
                            == Some(placement.baseline_y)
                        && placement.fingerprint
                            == sha256(encode_placement_payload(placement).as_bytes())
                        && usize::try_from(placement.line_index)
                            .ok()
                            .and_then(|line_index| self.lines.get(line_index))
                            .is_some_and(|line| {
                                placement.page_index == line.page_index
                                    && placement.frame_index == line.frame_index
                                    && placement.fragment_ordinal == line.fragment_ordinal
                                    && placement.baseline_y == line.baseline_y
                                    && placement.paragraph_node == line.paragraph_node
                            })
                })
    }
}

fn build_staging_inline_vector_layout(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &AdmittedResourceLedger,
    bindings: &ValidatedPrecomposedVectorBindings,
    input: &[StagingInlineVectorParagraphInput],
) -> Result<StagingInlineVectorSelectedLayout, StagingInlineVectorLayoutError> {
    bindings
        .verify(package, profile, limits, admitted)
        .map_err(|_| StagingInlineVectorLayoutError::BindingMismatch)?;
    profile
        .authorizes(package, limits)
        .map_err(|_| StagingInlineVectorLayoutError::BindingMismatch)?;

    let mut expected = Vec::new();
    expected
        .try_reserve_exact(package.precomposed_vector_metrics().len())
        .map_err(|_| StagingInlineVectorLayoutError::AllocationFailure)?;
    collect_inline_vectors(&package.document().blocks, &mut expected);
    for footnote in &package.document().footnotes {
        collect_inline_vectors(&footnote.blocks, &mut expected);
    }
    let input_vector_count = input
        .iter()
        .flat_map(|paragraph| paragraph.units.iter())
        .filter(|unit| matches!(unit, StagingInlineVectorLogicalUnit::Vector(_)))
        .count();
    if input_vector_count != expected.len() {
        return Err(StagingInlineVectorLayoutError::InputMismatch(
            expected
                .get(input_vector_count.min(expected.len().saturating_sub(1)))
                .map_or(NodeId::new(0), |value| value.node_id),
        ));
    }
    if input.len() > expected.len() {
        return Err(StagingInlineVectorLayoutError::InputMismatch(
            input
                .get(expected.len())
                .map_or(NodeId::new(0), |paragraph| paragraph.paragraph_node),
        ));
    }
    let placement_charge = u64::try_from(expected.len())
        .map_err(|_| StagingInlineVectorLayoutError::ArithmeticOverflow)?;
    let mut remaining_line_fragments = limits
        .base()
        .get()
        .max_fragments
        .checked_sub(placement_charge)
        .ok_or(StagingInlineVectorLayoutError::PlacementLimit)?;
    if u64::try_from(input.len()).map_err(|_| StagingInlineVectorLayoutError::ArithmeticOverflow)?
        > remaining_line_fragments
    {
        return Err(StagingInlineVectorLayoutError::PlacementLimit);
    }

    let body = profile.page_geometry().body();
    let mut expected_index = 0usize;
    let mut paragraphs = Vec::new();
    let mut breaks = Vec::new();
    let mut paragraph_facts = Vec::new();
    paragraphs
        .try_reserve_exact(input.len())
        .map_err(|_| StagingInlineVectorLayoutError::AllocationFailure)?;
    breaks
        .try_reserve_exact(input.len())
        .map_err(|_| StagingInlineVectorLayoutError::AllocationFailure)?;
    paragraph_facts
        .try_reserve_exact(input.len())
        .map_err(|_| StagingInlineVectorLayoutError::AllocationFailure)?;

    let mut paragraph_owners = std::collections::BTreeSet::new();
    for paragraph_input in input {
        if !paragraph_owners.insert(paragraph_input.paragraph_node) {
            return Err(StagingInlineVectorLayoutError::InputMismatch(
                paragraph_input.paragraph_node,
            ));
        }
        let mut units = Vec::new();
        units
            .try_reserve_exact(paragraph_input.units.len())
            .map_err(|_| StagingInlineVectorLayoutError::AllocationFailure)?;
        for unit in &paragraph_input.units {
            match *unit {
                StagingInlineVectorLogicalUnit::Text(value) => {
                    units.push(AtomicVectorInlineLogicalUnit::Text(value));
                }
                StagingInlineVectorLogicalUnit::Vector(node_id) => {
                    let expected_vector = expected
                        .get(expected_index)
                        .ok_or(StagingInlineVectorLayoutError::InputMismatch(node_id))?;
                    if expected_vector.node_id != node_id
                        || expected_vector.paragraph_node != paragraph_input.paragraph_node
                    {
                        return Err(StagingInlineVectorLayoutError::InputMismatch(node_id));
                    }
                    let receipt = bindings
                        .receipt(node_id)
                        .ok_or(StagingInlineVectorLayoutError::InputMismatch(node_id))?;
                    units.push(AtomicVectorInlineLogicalUnit::Vector(
                        atomic_item_from_receipt(expected_vector, receipt)?,
                    ));
                    expected_index = expected_index
                        .checked_add(1)
                        .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)?;
                }
            }
        }
        let paragraph = AtomicVectorInlineParagraph::itemize(units, paragraph_input.japanese_mode)?;
        if paragraph.paragraph_node() != paragraph_input.paragraph_node {
            return Err(StagingInlineVectorLayoutError::InputMismatch(
                paragraph_input.paragraph_node,
            ));
        }
        let selected = break_atomic_vector_inline(
            &paragraph,
            body.width(),
            paragraph_input.computed_line_height,
            remaining_line_fragments,
        )
        .map_err(|error| match error {
            AtomicVectorInlineError::SelectionLimit => {
                StagingInlineVectorLayoutError::PlacementLimit
            }
            error => StagingInlineVectorLayoutError::Atomic(error),
        })?;
        remaining_line_fragments = remaining_line_fragments
            .checked_sub(
                u64::try_from(selected.lines().len())
                    .map_err(|_| StagingInlineVectorLayoutError::ArithmeticOverflow)?,
            )
            .ok_or(StagingInlineVectorLayoutError::PlacementLimit)?;
        paragraph_facts.push(StagingInlineVectorParagraphFact {
            paragraph_node: paragraph.paragraph_node(),
            itemization_fingerprint: paragraph.fingerprint(),
            line_selection_fingerprint: selected.fingerprint(),
        });
        paragraphs.push(paragraph);
        breaks.push(selected);
    }
    if expected_index != expected.len() {
        return Err(StagingInlineVectorLayoutError::InputMismatch(
            expected
                .get(expected_index)
                .map_or(NodeId::new(0), |value| value.node_id),
        ));
    }

    let line_count = breaks.iter().try_fold(0usize, |total, selected| {
        total
            .checked_add(selected.lines().len())
            .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)
    })?;
    let placement_count = breaks.iter().try_fold(0usize, |total, selected| {
        let count = selected
            .lines()
            .iter()
            .try_fold(0usize, |line_total, line| {
                line_total
                    .checked_add(line.occurrences().len())
                    .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)
            })?;
        total
            .checked_add(count)
            .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)
    })?;
    let fragment_charge = u64::try_from(line_count)
        .ok()
        .and_then(|lines| {
            u64::try_from(placement_count)
                .ok()
                .and_then(|placements| lines.checked_add(placements))
        })
        .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)?;
    if fragment_charge > limits.base().get().max_fragments {
        return Err(StagingInlineVectorLayoutError::PlacementLimit);
    }

    let mut lines = Vec::new();
    let mut placements = Vec::new();
    lines
        .try_reserve_exact(line_count)
        .map_err(|_| StagingInlineVectorLayoutError::AllocationFailure)?;
    placements
        .try_reserve_exact(placement_count)
        .map_err(|_| StagingInlineVectorLayoutError::AllocationFailure)?;
    let mut page_index = 0u32;
    let mut cursor_y = 0i64;
    for (paragraph, selected) in paragraphs.iter().zip(&breaks) {
        for line in selected.lines() {
            let metrics = line.metrics();
            let line_height = metrics.line_height().get().raw();
            let oversize_owner = line
                .occurrences()
                .first()
                .map_or(paragraph.paragraph_node(), |value| value.item().node_id());
            if line_height > body.height().get().raw() {
                return Err(StagingInlineVectorLayoutError::Oversize(oversize_owner));
            }
            if cursor_y
                .checked_add(line_height)
                .map_or(true, |bottom| bottom > body.height().get().raw())
            {
                page_index = page_index
                    .checked_add(1)
                    .ok_or(StagingInlineVectorLayoutError::PageLimit)?;
                cursor_y = 0;
            }
            if page_index >= limits.base().get().max_pages {
                return Err(StagingInlineVectorLayoutError::PageLimit);
            }
            let line_top = body
                .y()
                .checked_add(raw_length(cursor_y)?)
                .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)?;
            let baseline_y = line_top
                .checked_add(metrics.leading_before().get())
                .and_then(|value| value.checked_add(metrics.content_ascent().get()))
                .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)?;
            let global_line_index = u32::try_from(lines.len())
                .map_err(|_| StagingInlineVectorLayoutError::ArithmeticOverflow)?;
            let visual_left = line
                .visual_left()
                .map(|value| {
                    body.x()
                        .checked_add(value)
                        .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)
                })
                .transpose()?;
            let visual_right = line
                .visual_right()
                .map(|value| {
                    body.x()
                        .checked_add(value)
                        .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)
                })
                .transpose()?;
            let mut selected_line = StagingInlineVectorLine {
                line_index: global_line_index,
                paragraph_node: paragraph.paragraph_node(),
                paragraph_line_index: line.line_index(),
                start_unit: line.start_unit(),
                end_unit: line.end_unit(),
                page_index,
                frame_index: 0,
                fragment_ordinal: global_line_index,
                line_top,
                baseline_y,
                break_demerits: line.break_demerits(),
                line_height: metrics.line_height(),
                content_ascent: metrics.content_ascent(),
                content_descent: metrics.content_descent(),
                leading_before: metrics.leading_before(),
                leading_after: metrics.leading_after(),
                logical_advance: line.logical_advance(),
                visual_left,
                visual_right,
                itemization_fingerprint: paragraph.fingerprint(),
                line_selection_fingerprint: selected.fingerprint(),
                fingerprint: [0; 32],
            };
            selected_line.fingerprint = sha256(encode_line_payload(&selected_line).as_bytes());

            let line_bottom = line_top
                .checked_add(metrics.line_height().get())
                .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)?;
            for occurrence in line.occurrences() {
                let item = occurrence.item();
                let metrics = item.metrics();
                let pen_origin_x = body
                    .x()
                    .checked_add(occurrence.pen_x())
                    .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)?;
                let geometry = metrics
                    .select_inline_geometry(pen_origin_x, baseline_y)
                    .map_err(map_geometry_error)?;
                let viewport = geometry.viewport();
                let viewport_right = viewport
                    .x()
                    .checked_add(viewport.width().get())
                    .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)?;
                let viewport_bottom = viewport
                    .y()
                    .checked_add(viewport.height().get())
                    .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)?;
                let body_right = body
                    .x()
                    .checked_add(body.width().get())
                    .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)?;
                if viewport.x().raw() < body.x().raw()
                    || viewport_right.raw() > body_right.raw()
                    || viewport.y().raw() < line_top.raw()
                    || viewport_bottom.raw() > line_bottom.raw()
                {
                    return Err(StagingInlineVectorLayoutError::Oversize(item.node_id()));
                }
                let occurrence_index = u32::try_from(placements.len())
                    .map_err(|_| StagingInlineVectorLayoutError::ArithmeticOverflow)?;
                let mut placement = StagingInlineVectorPlacement {
                    occurrence: occurrence_index,
                    node_id: item.node_id(),
                    paragraph_node: item.paragraph_node(),
                    binding_fingerprint: item.binding_fingerprint().bytes(),
                    atomic_item_fingerprint: item.fingerprint(),
                    line_index: global_line_index,
                    page_index,
                    frame_index: 0,
                    fragment_ordinal: global_line_index,
                    paint_ordinal: occurrence_index,
                    pen_origin_x,
                    baseline_y,
                    baseline: metrics.baseline(),
                    viewport,
                    scale: item.placement().scale().get().raw(),
                    spacing_before: occurrence.spacing_before(),
                    spacing_after: occurrence.spacing_after(),
                    fingerprint: [0; 32],
                };
                placement.fingerprint = sha256(encode_placement_payload(&placement).as_bytes());
                placements.push(placement);
            }
            lines.push(selected_line);
            cursor_y = cursor_y
                .checked_add(line_height)
                .ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)?;
        }
    }

    let page_geometry = profile.page_geometry().clone();
    let canonical_jcs = encode_selected_layout(
        package.canonical_jcs_sha256(),
        profile.profile_fingerprint(),
        limits.fingerprint(),
        admitted.fingerprint().bytes(),
        bindings.fingerprint(),
        bindings.epoch().fingerprint(),
        &page_geometry,
        &paragraph_facts,
        &lines,
        &placements,
        fragment_charge,
    );
    Ok(StagingInlineVectorSelectedLayout {
        paragraphs,
        breaks,
        paragraph_facts,
        lines,
        placements,
        page_geometry: page_geometry.clone(),
        receipt: StagingInlineVectorSelectedLayoutReceipt {
            package_sha256: package.canonical_jcs_sha256(),
            profile_fingerprint: profile.profile_fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            admitted_fingerprint: admitted.fingerprint().bytes(),
            binding_set_fingerprint: bindings.fingerprint(),
            layout_epoch_fingerprint: bindings.epoch().fingerprint(),
            page_geometry_fingerprint: page_geometry.fingerprint(),
            line_count: u32::try_from(line_count)
                .map_err(|_| StagingInlineVectorLayoutError::ArithmeticOverflow)?,
            placement_count: u32::try_from(placement_count)
                .map_err(|_| StagingInlineVectorLayoutError::ArithmeticOverflow)?,
            fragment_charge,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
    })
}

struct ExpectedInlineVector {
    node_id: NodeId,
    paragraph_node: NodeId,
    kind: StagingM4InlineVectorKind,
}

fn collect_inline_vectors(blocks: &[StagingM4Block], output: &mut Vec<ExpectedInlineVector>) {
    for block in blocks {
        match block {
            StagingM4Block::Paragraph {
                common,
                inline_vectors,
                ..
            }
            | StagingM4Block::Heading {
                common,
                inline_vectors,
                ..
            } => output.extend(inline_vectors.iter().map(|value| ExpectedInlineVector {
                node_id: value.node_id,
                paragraph_node: common.node_id,
                kind: value.kind,
            })),
            StagingM4Block::List { items, .. } => {
                for item in items {
                    collect_inline_vectors(&item.blocks, output);
                }
            }
            StagingM4Block::Table { head, body, .. } => {
                for cell in head.iter().chain(body).flat_map(|row| &row.cells) {
                    collect_inline_vectors(&cell.blocks, output);
                }
            }
            StagingM4Block::Figure { caption, .. }
            | StagingM4Block::VectorFigure { caption, .. } => {
                collect_inline_vectors(caption, output);
            }
            StagingM4Block::SemanticContainer { blocks, .. } => {
                collect_inline_vectors(blocks, output);
            }
            StagingM4Block::PageBreak { .. }
            | StagingM4Block::DisplayMath { .. }
            | StagingM4Block::MathVectorBlock { .. } => {}
        }
    }
}

fn atomic_item_from_receipt(
    expected: &ExpectedInlineVector,
    receipt: &ValidatedPrecomposedVectorReceipt,
) -> Result<AtomicVectorInlineItem, StagingInlineVectorLayoutError> {
    let expected_kind = match expected.kind {
        StagingM4InlineVectorKind::InlineVector => AtomicVectorInlineKind::InlineVector,
        StagingM4InlineVectorKind::MathVector => AtomicVectorInlineKind::MathVector,
    };
    let receipt_kind = match receipt.kind() {
        PrecomposedVectorKind::InlineVector => AtomicVectorInlineKind::InlineVector,
        PrecomposedVectorKind::MathVector => AtomicVectorInlineKind::MathVector,
        PrecomposedVectorKind::VectorFigure | PrecomposedVectorKind::MathVectorBlock => {
            return Err(StagingInlineVectorLayoutError::InputMismatch(
                expected.node_id,
            ));
        }
    };
    let PrecomposedVectorPlacementInput::Inline(placement) = receipt.placement() else {
        return Err(StagingInlineVectorLayoutError::InputMismatch(
            expected.node_id,
        ));
    };
    if receipt.node_id() != expected.node_id || receipt_kind != expected_kind {
        return Err(StagingInlineVectorLayoutError::InputMismatch(
            expected.node_id,
        ));
    }
    AtomicVectorInlineItem::from_bound_placement(
        receipt.node_id(),
        expected.paragraph_node,
        receipt.owner_source_span(),
        expected_kind,
        receipt.binding_fingerprint(),
        *placement,
    )
    .map_err(Into::into)
}

fn map_geometry_error(_: PrecomposedVectorGeometryError) -> StagingInlineVectorLayoutError {
    StagingInlineVectorLayoutError::ArithmeticOverflow
}

fn raw_length(value: i64) -> Result<Length, StagingInlineVectorLayoutError> {
    Length::from_raw(value).ok_or(StagingInlineVectorLayoutError::ArithmeticOverflow)
}

#[allow(clippy::too_many_arguments)]
fn encode_selected_layout(
    package_sha256: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    admitted_fingerprint: [u8; 32],
    binding_set_fingerprint: [u8; 32],
    layout_epoch_fingerprint: [u8; 32],
    page_geometry: &StagingM4PageGeometry,
    paragraphs: &[StagingInlineVectorParagraphFact],
    lines: &[StagingInlineVectorLine],
    placements: &[StagingInlineVectorPlacement],
    fragment_charge: u64,
) -> String {
    let mut output = String::from("{\"admitted_fingerprint\":");
    push_hash(&mut output, admitted_fingerprint);
    output.push_str(",\"algorithm\":");
    push_jcs_string(&mut output, PRECOMPOSED_VECTOR_SELECTED_LAYOUT_ALGORITHM);
    output.push_str(",\"binding_set_fingerprint\":");
    push_hash(&mut output, binding_set_fingerprint);
    output.push_str(",\"fragment_charge\":");
    output.push_str(&fragment_charge.to_string());
    output.push_str(",\"itemizations\":[");
    for (index, paragraph) in paragraphs.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"itemization_fingerprint\":");
        push_hash(&mut output, paragraph.itemization_fingerprint);
        output.push_str(",\"line_selection_fingerprint\":");
        push_hash(&mut output, paragraph.line_selection_fingerprint);
        output.push_str(",\"paragraph_node\":");
        output.push_str(&paragraph.paragraph_node.get().to_string());
        output.push('}');
    }
    output.push_str("],\"layout_epoch_fingerprint\":");
    push_hash(&mut output, layout_epoch_fingerprint);
    output.push_str(",\"limits_fingerprint\":");
    push_hash(&mut output, limits_fingerprint);
    output.push_str(",\"line_count\":");
    output.push_str(&lines.len().to_string());
    output.push_str(",\"lines\":[");
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_line_with_fingerprint(&mut output, line);
    }
    output.push_str("],\"package_sha256\":");
    push_hash(&mut output, package_sha256);
    output.push_str(",\"page_geometry\":");
    output.push_str(page_geometry.canonical_jcs());
    output.push_str(",\"page_geometry_fingerprint\":");
    push_hash(&mut output, page_geometry.fingerprint());
    output.push_str(",\"placement_count\":");
    output.push_str(&placements.len().to_string());
    output.push_str(",\"placements\":[");
    for (index, placement) in placements.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_placement_with_fingerprint(&mut output, placement);
    }
    output.push_str("],\"profile_fingerprint\":");
    push_hash(&mut output, profile_fingerprint);
    output.push('}');
    output
}

fn encode_line_payload(value: &StagingInlineVectorLine) -> String {
    let mut output = String::from("{\"baseline_y\":");
    output.push_str(&value.baseline_y.raw().to_string());
    output.push_str(",\"break_demerits\":");
    output.push_str(&value.break_demerits.to_string());
    output.push_str(",\"content_ascent\":");
    output.push_str(&value.content_ascent.get().raw().to_string());
    output.push_str(",\"content_descent\":");
    output.push_str(&value.content_descent.get().raw().to_string());
    output.push_str(",\"end_unit\":");
    output.push_str(&value.end_unit.to_string());
    output.push_str(",\"fragment_ordinal\":");
    output.push_str(&value.fragment_ordinal.to_string());
    output.push_str(",\"frame_index\":");
    output.push_str(&value.frame_index.to_string());
    output.push_str(",\"itemization_fingerprint\":");
    push_hash(&mut output, value.itemization_fingerprint);
    output.push_str(",\"leading_after\":");
    output.push_str(&value.leading_after.get().raw().to_string());
    output.push_str(",\"leading_before\":");
    output.push_str(&value.leading_before.get().raw().to_string());
    output.push_str(",\"line_height\":");
    output.push_str(&value.line_height.get().raw().to_string());
    output.push_str(",\"line_index\":");
    output.push_str(&value.line_index.to_string());
    output.push_str(",\"line_selection_fingerprint\":");
    push_hash(&mut output, value.line_selection_fingerprint);
    output.push_str(",\"line_top\":");
    output.push_str(&value.line_top.raw().to_string());
    output.push_str(",\"logical_advance\":");
    output.push_str(&value.logical_advance.get().raw().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&value.page_index.to_string());
    output.push_str(",\"paragraph_line_index\":");
    output.push_str(&value.paragraph_line_index.to_string());
    output.push_str(",\"paragraph_node\":");
    output.push_str(&value.paragraph_node.get().to_string());
    output.push_str(",\"start_unit\":");
    output.push_str(&value.start_unit.to_string());
    output.push_str(",\"visual_left\":");
    push_optional_length(&mut output, value.visual_left);
    output.push_str(",\"visual_right\":");
    push_optional_length(&mut output, value.visual_right);
    output.push('}');
    output
}

fn push_line_with_fingerprint(output: &mut String, value: &StagingInlineVectorLine) {
    let payload = encode_line_payload(value);
    output.push_str("{\"fingerprint\":");
    push_hash(output, value.fingerprint);
    output.push_str(",\"record\":");
    output.push_str(&payload);
    output.push('}');
}

fn encode_placement_payload(value: &StagingInlineVectorPlacement) -> String {
    let mut output = String::from("{\"atomic_item_fingerprint\":");
    push_hash(&mut output, value.atomic_item_fingerprint);
    output.push_str(",\"baseline\":");
    output.push_str(&value.baseline.get().raw().to_string());
    output.push_str(",\"baseline_y\":");
    output.push_str(&value.baseline_y.raw().to_string());
    output.push_str(",\"binding_fingerprint\":");
    push_hash(&mut output, value.binding_fingerprint);
    output.push_str(",\"fragment_ordinal\":");
    output.push_str(&value.fragment_ordinal.to_string());
    output.push_str(",\"frame_index\":");
    output.push_str(&value.frame_index.to_string());
    output.push_str(",\"line_index\":");
    output.push_str(&value.line_index.to_string());
    output.push_str(",\"node_id\":");
    output.push_str(&value.node_id.get().to_string());
    output.push_str(",\"occurrence\":");
    output.push_str(&value.occurrence.to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&value.page_index.to_string());
    output.push_str(",\"paint_ordinal\":");
    output.push_str(&value.paint_ordinal.to_string());
    output.push_str(",\"paragraph_node\":");
    output.push_str(&value.paragraph_node.get().to_string());
    output.push_str(",\"pen_origin_x\":");
    output.push_str(&value.pen_origin_x.raw().to_string());
    output.push_str(",\"scale\":");
    output.push_str(&value.scale.to_string());
    output.push_str(",\"spacing\":{\"after\":");
    output.push_str(&value.spacing_after.get().raw().to_string());
    output.push_str(",\"before\":");
    output.push_str(&value.spacing_before.get().raw().to_string());
    output.push_str("},\"viewport\":");
    push_rect(&mut output, value.viewport);
    output.push('}');
    output
}

fn push_placement_with_fingerprint(output: &mut String, value: &StagingInlineVectorPlacement) {
    let payload = encode_placement_payload(value);
    output.push_str("{\"fingerprint\":");
    push_hash(output, value.fingerprint);
    output.push_str(",\"record\":");
    output.push_str(&payload);
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

fn push_optional_length(output: &mut String, value: Option<Length>) {
    match value {
        Some(value) => output.push_str(&value.raw().to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safe_vector::{
        staging_precomposed_vector_binding_fixture,
        staging_precomposed_vector_binding_fixture_with_fragment_limit,
    };
    use typaxis_core::NonNegativeLength;

    fn raw_length(value: i64) -> Length {
        Length::from_raw(value).unwrap()
    }

    fn positive(value: i64) -> PositiveLength {
        PositiveLength::new(raw_length(value)).unwrap()
    }

    fn nonnegative(value: i64) -> NonNegativeLength {
        NonNegativeLength::new(raw_length(value)).unwrap()
    }

    fn text(scalar: char, advance: i64) -> StagingInlineVectorLogicalUnit {
        StagingInlineVectorLogicalUnit::Text(AtomicVectorTextUnit::new(
            scalar,
            nonnegative(advance),
            nonnegative(655_360),
            nonnegative(196_608),
        ))
    }

    fn fixture_input() -> Vec<StagingInlineVectorParagraphInput> {
        vec![StagingInlineVectorParagraphInput::new(
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
        )]
    }

    #[test]
    fn inline_vector_layout_binds_baseline_spacing_and_dynamic_line_fragments() {
        let fixture = staging_precomposed_vector_binding_fixture().unwrap();
        let input = fixture_input();
        let selected = layout_staging_precomposed_vector_inlines(
            &fixture.package,
            &fixture.profile,
            &fixture.limits,
            &fixture.admitted,
            &fixture.bindings,
            &input,
        )
        .unwrap();
        selected
            .verify(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
                &fixture.bindings,
                &input,
            )
            .unwrap();

        assert_eq!(selected.lines().len(), 1);
        assert_eq!(selected.placements().len(), 2);
        assert_eq!(selected.receipt().fragment_charge(), 3);
        let line = &selected.lines()[0];
        assert_eq!(line.content_ascent().get().raw(), 655_360);
        assert_eq!(line.content_descent().get().raw(), 196_608);
        assert_eq!(line.line_height().get().raw(), 1_310_720);
        assert_eq!(line.leading_before().get().raw(), 229_376);
        assert_eq!(line.leading_after().get().raw(), 229_376);
        assert_eq!(line.break_demerits(), 159);
        assert!(
            line.logical_advance().get().raw()
                < fixture.profile.page_geometry().body().width().get().raw()
        );
        for placement in selected.placements() {
            assert_eq!(
                placement
                    .viewport()
                    .y()
                    .checked_add(placement.baseline().get()),
                Some(placement.baseline_y())
            );
            assert_eq!(placement.page_index(), line.page_index());
            assert_eq!(placement.fragment_ordinal(), line.fragment_ordinal());
            assert_eq!(placement.paint_ordinal(), placement.occurrence());
        }
        assert_eq!(
            selected.placements()[0].spacing_before().get().raw(),
            16_384
        );
        assert_eq!(selected.placements()[0].spacing_after().get().raw(), 16_384);
        assert_eq!(
            selected.placements()[1].spacing_before().get().raw(),
            16_384
        );
        assert_eq!(selected.placements()[1].spacing_after().get().raw(), 16_384);
        let trace_json = selected.trace_json();
        assert_eq!(
            trace_json,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../samples/machine-package/staging/production-book-1/precomposed-vector/inline-layout-trace.json"
            ))
            .trim_end()
        );
    }

    #[test]
    fn inline_vector_layout_moves_dynamic_line_to_next_page_without_overlap() {
        let fixture = staging_precomposed_vector_binding_fixture().unwrap();
        let input = [StagingInlineVectorParagraphInput::new(
            NodeId::new(2),
            vec![
                text('日', 10_000_000),
                StagingInlineVectorLogicalUnit::Vector(NodeId::new(3)),
                text('語', 10_000_000),
                StagingInlineVectorLogicalUnit::Vector(NodeId::new(4)),
            ],
            positive(4_000_000),
            JapaneseLineBreakMode::Normal,
        )];
        let selected = layout_staging_precomposed_vector_inlines(
            &fixture.package,
            &fixture.profile,
            &fixture.limits,
            &fixture.admitted,
            &fixture.bindings,
            &input,
        )
        .unwrap();
        assert!(selected.lines().len() >= 2);
        for pair in selected.lines().windows(2) {
            if pair[0].page_index() == pair[1].page_index() {
                let previous_bottom = pair[0]
                    .line_top()
                    .checked_add(pair[0].line_height().get())
                    .unwrap();
                assert!(previous_bottom.raw() <= pair[1].line_top().raw());
            } else {
                assert!(pair[0].page_index() < pair[1].page_index());
            }
        }
        for placement in selected.placements() {
            let line = &selected.lines()[placement.line_index() as usize];
            let viewport_bottom = placement
                .viewport()
                .y()
                .checked_add(placement.viewport().height().get())
                .unwrap();
            let line_bottom = line
                .line_top()
                .checked_add(line.line_height().get())
                .unwrap();
            assert!(placement.viewport().y().raw() >= line.line_top().raw());
            assert!(viewport_bottom.raw() <= line_bottom.raw());
        }
    }

    #[test]
    fn inline_vector_layout_rejects_dynamic_line_taller_than_empty_frame() {
        let fixture = staging_precomposed_vector_binding_fixture().unwrap();
        let input = [StagingInlineVectorParagraphInput::new(
            NodeId::new(2),
            vec![
                StagingInlineVectorLogicalUnit::Vector(NodeId::new(3)),
                StagingInlineVectorLogicalUnit::Vector(NodeId::new(4)),
            ],
            positive(fixture.profile.page_geometry().body().height().get().raw() + 1),
            JapaneseLineBreakMode::Normal,
        )];

        assert_eq!(
            layout_staging_precomposed_vector_inlines(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
                &fixture.bindings,
                &input,
            ),
            Err(StagingInlineVectorLayoutError::Oversize(NodeId::new(3)))
        );
    }

    #[test]
    fn inline_vector_layout_charges_line_and_occurrences_at_exact_limit() {
        let exact = staging_precomposed_vector_binding_fixture_with_fragment_limit(3).unwrap();
        let input = fixture_input();
        let selected = layout_staging_precomposed_vector_inlines(
            &exact.package,
            &exact.profile,
            &exact.limits,
            &exact.admitted,
            &exact.bindings,
            &input,
        )
        .unwrap();
        assert_eq!(selected.receipt().fragment_charge(), 3);

        let over = staging_precomposed_vector_binding_fixture_with_fragment_limit(2).unwrap();
        assert_eq!(
            layout_staging_precomposed_vector_inlines(
                &over.package,
                &over.profile,
                &over.limits,
                &over.admitted,
                &over.bindings,
                &input,
            ),
            Err(StagingInlineVectorLayoutError::PlacementLimit)
        );
    }

    #[test]
    fn inline_vector_layout_rejects_wrong_owner_order_and_native_namespace() {
        let fixture = staging_precomposed_vector_binding_fixture().unwrap();
        let wrong_order = [StagingInlineVectorParagraphInput::new(
            NodeId::new(2),
            vec![
                StagingInlineVectorLogicalUnit::Vector(NodeId::new(4)),
                StagingInlineVectorLogicalUnit::Vector(NodeId::new(3)),
            ],
            positive(1_000_000),
            JapaneseLineBreakMode::Normal,
        )];
        assert!(matches!(
            layout_staging_precomposed_vector_inlines(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
                &fixture.bindings,
                &wrong_order,
            ),
            Err(StagingInlineVectorLayoutError::InputMismatch(_))
        ));

        let native_namespace = [StagingInlineVectorParagraphInput::new(
            NodeId::new(2),
            vec![StagingInlineVectorLogicalUnit::Vector(NodeId::new(999))],
            positive(1_000_000),
            JapaneseLineBreakMode::Normal,
        )];
        assert!(matches!(
            layout_staging_precomposed_vector_inlines(
                &fixture.package,
                &fixture.profile,
                &fixture.limits,
                &fixture.admitted,
                &fixture.bindings,
                &native_namespace,
            ),
            Err(StagingInlineVectorLayoutError::InputMismatch(_))
        ));
    }
}
