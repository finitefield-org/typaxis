#![forbid(unsafe_code)]

use typaxis_core::{
    BidiLevel, GlyphRunId, Length, NodeId, NonNegativeLength, PositiveLength, ReferenceFingerprint,
    TextOffset, TextSpan, ValidatedResourceLimits,
};
use typaxis_document::{Block, DocumentNodeKind, Inline};
use typaxis_layout_contract::LayoutEpoch;
use typaxis_shaping::{ShapeSourceSpan, ValidatedGlyphRun};
use typaxis_syntax::{
    PackageGeneratedTextBinding, PackageShapeTextReceipt, PackageShapeTextSource,
    ValidatedParsedPackage,
};
use typaxis_text::GeneratedProvenance;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BreakKind {
    Allowed,
    Mandatory,
    Prohibited,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemTextOffset {
    Parsed(TextOffset),
    Generated(GeneratedProvenance),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BreakOpportunity {
    pub offset: ItemTextOffset,
    pub penalty: i32,
    pub kind: BreakKind,
    pub flagged: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineShapeExhaustion {
    RepeatLast,
    Error,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineShape {
    pub inline_size: PositiveLength,
}

/// Canonical paragraph-item IR. Break legality is explicit; stretch and shrink
/// are never reconstructed from glyph coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemProvenance {
    Text(TextSpan),
    Generated(GeneratedProvenance),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShapedSlice {
    run_id: GlyphRunId,
    glyph_start: u32,
    glyph_end: u32,
    bidi_level: BidiLevel,
    derived_width: NonNegativeLength,
    source: ShapeSourceSpan,
    epoch: LayoutEpoch,
    site_owner: NodeId,
    style_owner: NodeId,
}
impl ShapedSlice {
    /// Creates one canonical paragraph box from one validated logical
    /// cluster. Arbitrary run IDs and partial glyph ranges are not accepted.
    pub fn from_cluster(run: &ValidatedGlyphRun, logical_ordinal: u32) -> Result<Self, BreakError> {
        let cluster = run
            .clusters()
            .get(logical_ordinal as usize)
            .ok_or(BreakError::UnknownShapedCluster)?;
        let start =
            usize::try_from(cluster.glyph_start).map_err(|_| BreakError::ArithmeticOverflow)?;
        let end = usize::try_from(cluster.glyph_end).map_err(|_| BreakError::ArithmeticOverflow)?;
        let width = run.glyphs()[start..end]
            .iter()
            .try_fold(Length::ZERO, |total, glyph| {
                total.checked_add(glyph.advance_x)
            })
            .ok_or(BreakError::ArithmeticOverflow)?;
        let derived_width = NonNegativeLength::new(width).ok_or(BreakError::InvalidGlyphAdvance)?;
        Ok(Self {
            run_id: run.run_id(),
            glyph_start: cluster.glyph_start,
            glyph_end: cluster.glyph_end,
            bidi_level: run.bidi_level(),
            derived_width,
            source: cluster.source_span,
            epoch: run.epoch(),
            site_owner: run.site_owner(),
            style_owner: run.style_owner(),
        })
    }
    pub const fn run_id(self) -> GlyphRunId {
        self.run_id
    }
    pub const fn glyph_start(self) -> u32 {
        self.glyph_start
    }
    pub const fn glyph_end(self) -> u32 {
        self.glyph_end
    }
    pub const fn bidi_level(self) -> BidiLevel {
        self.bidi_level
    }
    pub const fn derived_width(self) -> NonNegativeLength {
        self.derived_width
    }
    pub const fn source(self) -> ShapeSourceSpan {
        self.source
    }
    pub const fn epoch(self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn site_owner(self) -> NodeId {
        self.site_owner
    }
    pub const fn style_owner(self) -> NodeId {
        self.style_owner
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiscretionaryBranch {
    pub width: Length,
    pub shaped: Option<ShapedSlice>,
    pub provenance: ItemProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParagraphItem {
    Box {
        width: NonNegativeLength,
        shaped: ShapedSlice,
        provenance: ItemProvenance,
    },
    Glue {
        natural: NonNegativeLength,
        stretch: NonNegativeLength,
        shrink: NonNegativeLength,
        priority: u8,
        provenance: ItemProvenance,
    },
    Penalty {
        width: Length,
        cost: i32,
        kind: BreakKind,
        flagged: bool,
        provenance: ItemProvenance,
    },
    Discretionary {
        no_break: Box<DiscretionaryBranch>,
        pre_break: Box<DiscretionaryBranch>,
        post_break: Box<DiscretionaryBranch>,
        penalty: i32,
        flagged: bool,
    },
    InlineObject {
        node_id: NodeId,
        width: NonNegativeLength,
        height: NonNegativeLength,
        provenance: ItemProvenance,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParagraphInput<'a> {
    paragraph_node: NodeId,
    epoch: LayoutEpoch,
    reference_fingerprint: ReferenceFingerprint,
    items: &'a [ParagraphItem],
    line_shapes: &'a [LineShape],
    line_shape_exhaustion: LineShapeExhaustion,
}
impl<'a> ParagraphInput<'a> {
    /// Reserved for the sealed in-crate canonical itemizer. Until that
    /// implementation lands, nonempty paragraphs fail closed; downstream
    /// callers cannot promote an arbitrary `Vec<ParagraphItem>`.
    #[allow(dead_code)]
    fn new(
        paragraph_node: NodeId,
        generated_text: PackageGeneratedTextBinding<'_>,
        epoch: LayoutEpoch,
        items: &'a [ParagraphItem],
        line_shapes: &'a [LineShape],
        line_shape_exhaustion: LineShapeExhaustion,
    ) -> Result<Self, BreakError> {
        if line_shapes.is_empty() {
            return Err(BreakError::EmptyLineShapes);
        }
        let package = generated_text.package();
        if package.document_nodes().node_kind(paragraph_node) != Some(DocumentNodeKind::Paragraph)
            && package.document_nodes().node_kind(paragraph_node) != Some(DocumentNodeKind::Heading)
        {
            return Err(BreakError::InvalidParagraphOwner);
        }
        if epoch.document() != package.epoch_identity().document()
            || epoch.style() != package.epoch_identity().style()
            || epoch.references() != generated_text.generated_text().reference_fingerprint()
        {
            return Err(BreakError::ParagraphEpochMismatch);
        }
        if items.is_empty() {
            return Err(BreakError::EmptyParagraphItems);
        }
        let mut generated = std::collections::BTreeSet::new();
        for item in items {
            validate_item(item, paragraph_node, generated_text, epoch)?;
        }
        for provenance in items.iter().flat_map(item_provenances) {
            if let ItemProvenance::Generated(provenance) = provenance {
                if !generated.insert(*provenance) {
                    return Err(BreakError::DuplicateGeneratedProvenance);
                }
            }
        }
        Ok(Self {
            paragraph_node,
            epoch,
            reference_fingerprint: generated_text.generated_text().reference_fingerprint(),
            items,
            line_shapes,
            line_shape_exhaustion,
        })
    }
    pub const fn paragraph_node(&self) -> NodeId {
        self.paragraph_node
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn reference_fingerprint(&self) -> ReferenceFingerprint {
        self.reference_fingerprint
    }
    pub const fn items(&self) -> &[ParagraphItem] {
        self.items
    }
    pub const fn line_shapes(&self) -> &[LineShape] {
        self.line_shapes
    }
    pub const fn line_shape_exhaustion(&self) -> LineShapeExhaustion {
        self.line_shape_exhaustion
    }
}

fn validate_item(
    item: &ParagraphItem,
    paragraph_node: NodeId,
    generated_text: PackageGeneratedTextBinding<'_>,
    epoch: LayoutEpoch,
) -> Result<(), BreakError> {
    match item {
        ParagraphItem::Box {
            width,
            shaped,
            provenance,
        } => {
            if *width != shaped.derived_width() {
                return Err(BreakError::ShapedWidthMismatch);
            }
            validate_shaped_provenance(*shaped, provenance, paragraph_node, generated_text, epoch)
        }
        ParagraphItem::Glue { provenance, .. } | ParagraphItem::Penalty { provenance, .. } => {
            validate_provenance(provenance, paragraph_node, generated_text).map(|_| ())
        }
        ParagraphItem::Discretionary {
            no_break,
            pre_break,
            post_break,
            ..
        } => {
            for branch in [no_break.as_ref(), pre_break.as_ref(), post_break.as_ref()] {
                let receipt =
                    validate_provenance(&branch.provenance, paragraph_node, generated_text)?;
                if let Some(shaped) = branch.shaped {
                    if branch.width != shaped.derived_width().get() {
                        return Err(BreakError::ShapedWidthMismatch);
                    }
                    validate_shaped_receipt(shaped, &receipt, epoch)?;
                } else if branch.width != Length::ZERO || !receipt.utf8().is_empty() {
                    return Err(BreakError::InvalidEmptyDiscretionaryBranch);
                }
            }
            Ok(())
        }
        ParagraphItem::InlineObject {
            node_id,
            provenance,
            ..
        } => {
            validate_provenance(provenance, paragraph_node, generated_text)?;
            let nodes = generated_text.package().document_nodes();
            let paragraph_path = nodes
                .node_path(paragraph_node)
                .ok_or(BreakError::InvalidParagraphOwner)?;
            let object_path = nodes
                .node_path(*node_id)
                .ok_or(BreakError::InvalidInlineObject)?;
            if object_path.len() <= paragraph_path.len() || !object_path.starts_with(paragraph_path)
            {
                return Err(BreakError::InvalidInlineObject);
            }
            Ok(())
        }
    }
}

fn validate_shaped_provenance(
    shaped: ShapedSlice,
    provenance: &ItemProvenance,
    paragraph_node: NodeId,
    generated_text: PackageGeneratedTextBinding<'_>,
    epoch: LayoutEpoch,
) -> Result<(), BreakError> {
    let receipt = validate_provenance(provenance, paragraph_node, generated_text)?;
    validate_shaped_receipt(shaped, &receipt, epoch)
}

fn validate_shaped_receipt(
    shaped: ShapedSlice,
    receipt: &PackageShapeTextReceipt<'_>,
    epoch: LayoutEpoch,
) -> Result<(), BreakError> {
    if shaped.epoch() != epoch
        || shaped.source() != shape_source(receipt.source())
        || shaped.site_owner() != receipt.site_owner()
        || shaped.style_owner() != receipt.style_owner()
    {
        return Err(BreakError::ShapedSliceMismatch);
    }
    Ok(())
}

fn validate_provenance<'a>(
    provenance: &ItemProvenance,
    paragraph_node: NodeId,
    generated_text: PackageGeneratedTextBinding<'a>,
) -> Result<PackageShapeTextReceipt<'a>, BreakError> {
    let receipt = match provenance {
        ItemProvenance::Text(span) => generated_text
            .package()
            .bind_parsed_shape_text(*span)
            .map_err(|_| BreakError::InvalidItemProvenance)?,
        ItemProvenance::Generated(provenance) => generated_text
            .bind_generated_shape_text(*provenance)
            .map_err(|_| BreakError::InvalidItemProvenance)?,
    };
    if receipt.style_owner() != paragraph_node {
        return Err(BreakError::InvalidItemOwner);
    }
    Ok(receipt)
}

const fn shape_source(source: PackageShapeTextSource) -> ShapeSourceSpan {
    match source {
        PackageShapeTextSource::Parsed(span) => ShapeSourceSpan::Parsed(span),
        PackageShapeTextSource::Generated(provenance) => ShapeSourceSpan::Generated(provenance),
    }
}

fn item_provenances(item: &ParagraphItem) -> Vec<&ItemProvenance> {
    match item {
        ParagraphItem::Box { provenance, .. }
        | ParagraphItem::Glue { provenance, .. }
        | ParagraphItem::Penalty { provenance, .. }
        | ParagraphItem::InlineObject { provenance, .. } => vec![provenance],
        ParagraphItem::Discretionary {
            no_break,
            pre_break,
            post_break,
            ..
        } => {
            vec![
                &no_break.provenance,
                &pre_break.provenance,
                &post_break.provenance,
            ]
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineBreak {
    pub item_index: u32,
    pub offset: Option<ItemTextOffset>,
    pub demerits: i64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParagraphBreak {
    pub lines: Vec<LineBreak>,
}

/// Line-break result tied to the exact paragraph-item sequence and layout
/// epoch. Flow construction consumes this receipt, never a raw item count.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedParagraphBreak {
    paragraph_node: NodeId,
    epoch: LayoutEpoch,
    item_count: u32,
    items: Vec<ParagraphItem>,
    result: ParagraphBreak,
    reshape_passes: u16,
}
impl ValidatedParagraphBreak {
    pub const fn paragraph_node(&self) -> NodeId {
        self.paragraph_node
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn item_count(&self) -> u32 {
        self.item_count
    }
    pub fn items(&self) -> &[ParagraphItem] {
        &self.items
    }
    pub const fn result(&self) -> &ParagraphBreak {
        &self.result
    }
    pub const fn reshape_passes(&self) -> u16 {
        self.reshape_passes
    }
}

/// Canonical paragraph-item boundary registry issued by line layout. The
/// package/epoch and exact set of main-flow paragraphs are inseparable from
/// their dense item counts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedParagraphItemRegistry {
    epoch: LayoutEpoch,
    item_sequences: std::collections::BTreeMap<NodeId, ParagraphItemSequence>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
enum ParagraphItemSequence {
    EmptyContent,
    Items {
        count: u32,
        items: Vec<ParagraphItem>,
    },
}
impl ValidatedParagraphItemRegistry {
    pub fn from_breaks(
        package: &ValidatedParsedPackage,
        epoch: LayoutEpoch,
        breaks: &[ValidatedParagraphBreak],
    ) -> Result<Self, BreakError> {
        validate_package_epoch(package, epoch)?;
        if !package.package().document.footnotes.is_empty() {
            return Err(BreakError::UnsupportedFlowDomain);
        }
        let expected = main_paragraph_nodes(&package.package().document.blocks);
        let mut item_sequences = std::collections::BTreeMap::new();
        for receipt in breaks {
            if receipt.epoch != epoch
                || !expected.contains(&receipt.paragraph_node)
                || receipt.item_count == 0
            {
                return Err(BreakError::InvalidParagraphBreakReceipt);
            }
            if item_sequences
                .insert(
                    receipt.paragraph_node,
                    ParagraphItemSequence::Items {
                        count: receipt.item_count,
                        items: receipt.items.clone(),
                    },
                )
                .is_some()
            {
                return Err(BreakError::DuplicateParagraphBreak);
            }
        }
        if item_sequences.len() != expected.len()
            || expected
                .iter()
                .any(|node| !item_sequences.contains_key(node))
        {
            return Err(BreakError::MissingParagraphBreak);
        }
        Ok(Self {
            epoch,
            item_sequences,
        })
    }

    /// Deterministic reference path for paragraphs containing no text,
    /// generated site, break, or inline object. It is not a fallback for
    /// unimplemented itemization: any text-producing content fails closed.
    pub fn for_empty_content(
        package: &ValidatedParsedPackage,
        epoch: LayoutEpoch,
    ) -> Result<Self, BreakError> {
        validate_package_epoch(package, epoch)?;
        if !package.package().document.footnotes.is_empty() {
            return Err(BreakError::UnsupportedFlowDomain);
        }
        let paragraphs = main_paragraph_blocks(&package.package().document.blocks);
        if paragraphs
            .iter()
            .any(|block| !paragraph_has_empty_content(block))
        {
            return Err(BreakError::ParagraphItemsRequired);
        }
        let item_sequences = paragraphs
            .into_iter()
            .map(|block| (block_node_id(block), ParagraphItemSequence::EmptyContent))
            .collect();
        Ok(Self {
            epoch,
            item_sequences,
        })
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub fn item_count(&self, paragraph_node: NodeId) -> Option<u32> {
        match self.item_sequences.get(&paragraph_node)? {
            ParagraphItemSequence::EmptyContent => Some(1),
            ParagraphItemSequence::Items { count, .. } => Some(*count),
        }
    }
    pub fn items(&self, paragraph_node: NodeId) -> Option<&[ParagraphItem]> {
        match self.item_sequences.get(&paragraph_node)? {
            ParagraphItemSequence::EmptyContent => Some(&[]),
            ParagraphItemSequence::Items { items, .. } => Some(items),
        }
    }
    pub fn paragraphs(&self) -> impl ExactSizeIterator<Item = (NodeId, u32)> + '_ {
        self.item_sequences.iter().map(|(node, sequence)| {
            let count = match sequence {
                ParagraphItemSequence::EmptyContent => 1,
                ParagraphItemSequence::Items { count, .. } => *count,
            };
            (*node, count)
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BreakError {
    NoFeasibleBreak,
    ArithmeticOverflow,
    IterationLimit,
    ClusterBoundaryViolation,
    InvalidOpportunity,
    EmptyLineShapes,
    DuplicateGeneratedProvenance,
    InvalidGeneratedProvenance,
    BudgetAlreadyIssued,
    BreakAlreadyStarted,
    UnknownShapedCluster,
    InvalidGlyphAdvance,
    ShapedWidthMismatch,
    InvalidEmptyDiscretionaryBranch,
    InvalidParagraphOwner,
    ParagraphEpochMismatch,
    EmptyParagraphItems,
    InvalidItemProvenance,
    InvalidItemOwner,
    ShapedSliceMismatch,
    InvalidInlineObject,
    InvalidParagraphBreakReceipt,
    DuplicateParagraphBreak,
    MissingParagraphBreak,
    ParagraphItemsRequired,
    UnsupportedFlowDomain,
}
#[derive(Debug, Eq, PartialEq)]
pub struct LineLayoutBudget {
    initial_reshape_passes: u16,
    remaining_reshape_passes: u16,
    break_started: bool,
}
impl LineLayoutBudget {
    fn from_limits(limits: &ValidatedResourceLimits) -> Self {
        Self {
            initial_reshape_passes: limits.get().max_line_reshape_passes,
            remaining_reshape_passes: limits.get().max_line_reshape_passes,
            break_started: false,
        }
    }
    fn begin_break(&mut self) -> Result<(), BreakError> {
        if core::mem::replace(&mut self.break_started, true) {
            return Err(BreakError::BreakAlreadyStarted);
        }
        Ok(())
    }
    pub fn consume_reshape(&mut self) -> Result<(), BreakError> {
        self.remaining_reshape_passes = self
            .remaining_reshape_passes
            .checked_sub(1)
            .ok_or(BreakError::IterationLimit)?;
        Ok(())
    }
    pub const fn remaining_reshape_passes(&self) -> u16 {
        self.remaining_reshape_passes
    }
    fn consumed_reshape_passes(&self) -> u16 {
        self.initial_reshape_passes - self.remaining_reshape_passes
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct LineLayoutContext {
    budget: Option<LineLayoutBudget>,
}
impl LineLayoutContext {
    pub fn from_limits(limits: &ValidatedResourceLimits) -> Self {
        Self {
            budget: Some(LineLayoutBudget::from_limits(limits)),
        }
    }
    pub fn take_budget(&mut self) -> Result<LineLayoutBudget, BreakError> {
        self.budget.take().ok_or(BreakError::BudgetAlreadyIssued)
    }
}
pub trait ParagraphBreaker {
    fn break_paragraph(
        &self,
        input: &ParagraphInput<'_>,
        budget: &mut LineLayoutBudget,
    ) -> Result<ParagraphBreak, BreakError>;
}

/// Sole promotion boundary from a pluggable breaker result to a trusted
/// paragraph layout receipt. The one-shot work budget is consumed before the
/// implementation runs, and its output is checked against the exact items.
pub fn break_paragraph_validated<B: ParagraphBreaker>(
    breaker: &B,
    input: &ParagraphInput<'_>,
    budget: &mut LineLayoutBudget,
) -> Result<ValidatedParagraphBreak, BreakError> {
    budget.begin_break()?;
    let result = breaker.break_paragraph(input, budget)?;
    validate_paragraph_break(input, &result)?;
    let item_count =
        u32::try_from(input.items.len()).map_err(|_| BreakError::ArithmeticOverflow)?;
    Ok(ValidatedParagraphBreak {
        paragraph_node: input.paragraph_node,
        epoch: input.epoch,
        item_count,
        items: input.items.to_vec(),
        result,
        reshape_passes: budget.consumed_reshape_passes(),
    })
}

fn validate_paragraph_break(
    input: &ParagraphInput<'_>,
    result: &ParagraphBreak,
) -> Result<(), BreakError> {
    let item_count =
        u32::try_from(input.items.len()).map_err(|_| BreakError::ArithmeticOverflow)?;
    if item_count == 0 || result.lines.is_empty() {
        return Err(BreakError::InvalidParagraphBreakReceipt);
    }
    let mut previous = 0u32;
    let mut selected_mandatory = std::collections::BTreeSet::new();
    for line in &result.lines {
        if line.item_index <= previous || line.item_index > item_count {
            return Err(BreakError::InvalidParagraphBreakReceipt);
        }
        let break_item = &input.items[(line.item_index - 1) as usize];
        if line.item_index != item_count && !is_legal_intermediate_break(break_item) {
            return Err(BreakError::InvalidOpportunity);
        }
        if matches!(
            break_item,
            ParagraphItem::Penalty {
                kind: BreakKind::Mandatory,
                ..
            }
        ) {
            selected_mandatory.insert(line.item_index);
        }
        if let Some(offset) = line.offset {
            if !item_provenances(break_item)
                .into_iter()
                .any(|provenance| provenance_contains_offset(provenance, offset))
            {
                return Err(BreakError::InvalidOpportunity);
            }
        }
        previous = line.item_index;
    }
    if previous != item_count {
        return Err(BreakError::InvalidParagraphBreakReceipt);
    }
    for (index, item) in input.items.iter().enumerate() {
        if matches!(
            item,
            ParagraphItem::Penalty {
                kind: BreakKind::Mandatory,
                ..
            }
        ) {
            let exclusive_end =
                u32::try_from(index + 1).map_err(|_| BreakError::ArithmeticOverflow)?;
            if !selected_mandatory.contains(&exclusive_end) {
                return Err(BreakError::InvalidOpportunity);
            }
        }
    }
    Ok(())
}

fn is_legal_intermediate_break(item: &ParagraphItem) -> bool {
    matches!(
        item,
        ParagraphItem::Glue { .. }
            | ParagraphItem::Penalty {
                kind: BreakKind::Allowed | BreakKind::Mandatory,
                ..
            }
            | ParagraphItem::Discretionary { .. }
    )
}

fn provenance_contains_offset(provenance: &ItemProvenance, offset: ItemTextOffset) -> bool {
    match (provenance, offset) {
        (ItemProvenance::Text(span), ItemTextOffset::Parsed(offset)) => {
            span.text_id() == offset.text_id && offset.byte.get() == span.end_byte().get()
        }
        (ItemProvenance::Generated(expected), ItemTextOffset::Generated(actual)) => {
            expected == &actual
        }
        _ => false,
    }
}

fn validate_package_epoch(
    package: &ValidatedParsedPackage,
    epoch: LayoutEpoch,
) -> Result<(), BreakError> {
    if epoch.document() != package.epoch_identity().document()
        || epoch.style() != package.epoch_identity().style()
    {
        return Err(BreakError::ParagraphEpochMismatch);
    }
    Ok(())
}

fn main_paragraph_nodes(blocks: &[Block]) -> std::collections::BTreeSet<NodeId> {
    main_paragraph_blocks(blocks)
        .into_iter()
        .map(block_node_id)
        .collect()
}

const fn block_node_id(block: &Block) -> NodeId {
    match block {
        Block::Paragraph { node_id, .. }
        | Block::Heading { node_id, .. }
        | Block::List { node_id, .. }
        | Block::Table { node_id, .. }
        | Block::Figure { node_id, .. }
        | Block::PageBreak { node_id, .. } => *node_id,
    }
}

fn main_paragraph_blocks(blocks: &[Block]) -> Vec<&Block> {
    let mut paragraphs = Vec::new();
    let mut pending: Vec<&Block> = blocks.iter().rev().collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Paragraph { .. } | Block::Heading { .. } => paragraphs.push(block),
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Table { head, body, .. } => {
                pending.extend(
                    body.iter()
                        .rev()
                        .chain(head.iter().rev())
                        .flat_map(|row| row.cells.iter().rev())
                        .flat_map(|cell| cell.blocks.iter().rev()),
                );
            }
            Block::Figure { caption, .. } => pending.extend(caption.iter().rev()),
            Block::PageBreak { .. } => {}
        }
    }
    paragraphs
}

fn paragraph_has_empty_content(block: &Block) -> bool {
    let children = match block {
        Block::Paragraph { children, .. } | Block::Heading { children, .. } => children,
        _ => return false,
    };
    let mut pending: Vec<&Inline> = children.iter().rev().collect();
    while let Some(inline) = pending.pop() {
        match inline {
            Inline::Anchor { .. } => {}
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => pending.extend(children.iter().rev()),
            Inline::Text { .. }
            | Inline::Reference { .. }
            | Inline::FootnoteReference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{
        PortablePath, ResourceLimits, SourceId, TextBufferId, Utf8ByteOffset,
        ValidatedResourceLimits,
    };
    use typaxis_resource_admission::AdmittedResourceResolver;
    use typaxis_syntax::{
        PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
    };
    use typaxis_text::GeneratedTextStore;

    fn package_epoch(
        source_text: &str,
    ) -> (ValidatedParsedPackage, GeneratedTextStore, LayoutEpoch) {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
        let source = SourceFile {
            source_id: SourceId::new(0),
            uri: PortablePath::new("linebreak-input.tsf").unwrap(),
            text: source_text.to_owned(),
        };
        let ParseOutcome::Parsed { package, .. } = ReferenceParser::new().parse(
            &source,
            &PackageValidationPolicy::new(&limits, &schemes).unwrap(),
        ) else {
            panic!("reference package must parse");
        };
        let package = *package;
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
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let epoch = LayoutEpoch::from_validated_inputs(binding, admitted.token()).unwrap();
        (package, generated, epoch)
    }

    fn parsed_span(start: u32, end: u32) -> TextSpan {
        TextSpan::new(
            TextBufferId::new(0),
            Utf8ByteOffset::new(start),
            Utf8ByteOffset::new(end),
        )
        .unwrap()
    }

    struct FixedBreaker(ParagraphBreak);
    impl ParagraphBreaker for FixedBreaker {
        fn break_paragraph(
            &self,
            _input: &ParagraphInput<'_>,
            _budget: &mut LineLayoutBudget,
        ) -> Result<ParagraphBreak, BreakError> {
            Ok(self.0.clone())
        }
    }
    #[test]
    fn break_kind_cannot_be_mandatory_and_prohibited() {
        let opportunity = BreakOpportunity {
            offset: ItemTextOffset::Parsed(TextOffset {
                text_id: typaxis_core::TextBufferId::new(0),
                byte: typaxis_core::Utf8ByteOffset::new(0),
            }),
            penalty: 0,
            kind: BreakKind::Mandatory,
            flagged: false,
        };
        assert_eq!(opportunity.kind, BreakKind::Mandatory);
    }
    #[test]
    fn reshape_budget_rejects_max_plus_one_before_work() {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_line_reshape_passes: 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        let mut context = LineLayoutContext::from_limits(&limits);
        let mut budget = context.take_budget().unwrap();
        assert_eq!(context.take_budget(), Err(BreakError::BudgetAlreadyIssued));
        assert!(budget.consume_reshape().is_ok());
        assert_eq!(budget.consume_reshape(), Err(BreakError::IterationLimit));
    }

    #[test]
    fn empty_content_registry_is_deterministic_and_text_fails_closed() {
        let (empty_paragraph, _, empty_epoch) = package_epoch("paragraph");
        let registry =
            ValidatedParagraphItemRegistry::for_empty_content(&empty_paragraph, empty_epoch)
                .unwrap();
        assert_eq!(registry.item_count(NodeId::new(1)), Some(1));
        assert_eq!(registry.items(NodeId::new(1)), Some([].as_slice()));

        let (text_package, _, text_epoch) = package_epoch("text:x");
        assert_eq!(
            ValidatedParagraphItemRegistry::for_empty_content(&text_package, text_epoch),
            Err(BreakError::ParagraphItemsRequired)
        );
    }

    #[test]
    fn break_receipt_binds_exact_items_and_legal_breaks() {
        let (package, generated, epoch) = package_epoch("text:ab");
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let items = [
            ParagraphItem::Penalty {
                width: Length::ZERO,
                cost: 0,
                kind: BreakKind::Mandatory,
                flagged: false,
                provenance: ItemProvenance::Text(parsed_span(0, 1)),
            },
            ParagraphItem::Penalty {
                width: Length::ZERO,
                cost: 0,
                kind: BreakKind::Allowed,
                flagged: false,
                provenance: ItemProvenance::Text(parsed_span(1, 2)),
            },
        ];
        let shapes = [LineShape {
            inline_size: PositiveLength::new(Length::from_raw(10).unwrap()).unwrap(),
        }];
        let input = ParagraphInput::new(
            NodeId::new(1),
            binding,
            epoch,
            &items,
            &shapes,
            LineShapeExhaustion::RepeatLast,
        )
        .unwrap();

        let missing_mandatory = FixedBreaker(ParagraphBreak {
            lines: vec![LineBreak {
                item_index: 2,
                offset: None,
                demerits: 0,
            }],
        });
        let mut context = LineLayoutContext::from_limits(&limits);
        assert_eq!(
            break_paragraph_validated(
                &missing_mandatory,
                &input,
                &mut context.take_budget().unwrap(),
            ),
            Err(BreakError::InvalidOpportunity)
        );

        let wrong_item_offset = FixedBreaker(ParagraphBreak {
            lines: vec![
                LineBreak {
                    item_index: 1,
                    offset: Some(ItemTextOffset::Parsed(TextOffset {
                        text_id: TextBufferId::new(0),
                        byte: Utf8ByteOffset::new(2),
                    })),
                    demerits: 0,
                },
                LineBreak {
                    item_index: 2,
                    offset: None,
                    demerits: 0,
                },
            ],
        });
        let mut context = LineLayoutContext::from_limits(&limits);
        assert_eq!(
            break_paragraph_validated(
                &wrong_item_offset,
                &input,
                &mut context.take_budget().unwrap(),
            ),
            Err(BreakError::InvalidOpportunity)
        );

        let prohibited_items = [
            ParagraphItem::Penalty {
                width: Length::ZERO,
                cost: 0,
                kind: BreakKind::Prohibited,
                flagged: false,
                provenance: ItemProvenance::Text(parsed_span(0, 1)),
            },
            items[1].clone(),
        ];
        let prohibited_input = ParagraphInput::new(
            NodeId::new(1),
            binding,
            epoch,
            &prohibited_items,
            &shapes,
            LineShapeExhaustion::RepeatLast,
        )
        .unwrap();
        let prohibited_break = FixedBreaker(ParagraphBreak {
            lines: vec![
                LineBreak {
                    item_index: 1,
                    offset: None,
                    demerits: 0,
                },
                LineBreak {
                    item_index: 2,
                    offset: None,
                    demerits: 0,
                },
            ],
        });
        let mut context = LineLayoutContext::from_limits(&limits);
        assert_eq!(
            break_paragraph_validated(
                &prohibited_break,
                &prohibited_input,
                &mut context.take_budget().unwrap(),
            ),
            Err(BreakError::InvalidOpportunity)
        );

        let valid = FixedBreaker(ParagraphBreak {
            lines: vec![
                LineBreak {
                    item_index: 1,
                    offset: Some(ItemTextOffset::Parsed(TextOffset {
                        text_id: TextBufferId::new(0),
                        byte: Utf8ByteOffset::new(1),
                    })),
                    demerits: 0,
                },
                LineBreak {
                    item_index: 2,
                    offset: None,
                    demerits: 0,
                },
            ],
        });
        let mut context = LineLayoutContext::from_limits(&limits);
        let receipt =
            break_paragraph_validated(&valid, &input, &mut context.take_budget().unwrap()).unwrap();
        assert_eq!(receipt.items(), &items);
        let registry =
            ValidatedParagraphItemRegistry::from_breaks(&package, epoch, &[receipt]).unwrap();
        assert_eq!(registry.item_count(NodeId::new(1)), Some(2));
        assert_eq!(registry.items(NodeId::new(1)), Some(items.as_slice()));
    }

    #[test]
    fn parsed_break_offset_is_the_item_exclusive_end() {
        let provenance = ItemProvenance::Text(parsed_span(0, 2));
        for byte in [0, 1] {
            assert!(!provenance_contains_offset(
                &provenance,
                ItemTextOffset::Parsed(TextOffset {
                    text_id: TextBufferId::new(0),
                    byte: Utf8ByteOffset::new(byte),
                }),
            ));
        }
        assert!(provenance_contains_offset(
            &provenance,
            ItemTextOffset::Parsed(TextOffset {
                text_id: TextBufferId::new(0),
                byte: Utf8ByteOffset::new(2),
            }),
        ));
    }

    #[test]
    fn paragraph_items_cannot_override_shaped_or_empty_branch_widths() {
        let (package, generated, epoch) = package_epoch("text:a");
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let shapes = [LineShape {
            inline_size: PositiveLength::new(Length::from_raw(10).unwrap()).unwrap(),
        }];
        let shaped = ShapedSlice {
            run_id: GlyphRunId::new(0),
            glyph_start: 0,
            glyph_end: 1,
            bidi_level: BidiLevel::new(0).unwrap(),
            derived_width: NonNegativeLength::new(Length::from_raw(2).unwrap()).unwrap(),
            source: ShapeSourceSpan::Parsed(parsed_span(0, 1)),
            epoch,
            site_owner: NodeId::new(2),
            style_owner: NodeId::new(1),
        };
        let wrong_box = [ParagraphItem::Box {
            width: NonNegativeLength::new(Length::from_raw(1).unwrap()).unwrap(),
            shaped,
            provenance: ItemProvenance::Text(parsed_span(0, 1)),
        }];
        assert_eq!(
            ParagraphInput::new(
                NodeId::new(1),
                binding,
                epoch,
                &wrong_box,
                &shapes,
                LineShapeExhaustion::RepeatLast,
            ),
            Err(BreakError::ShapedWidthMismatch)
        );

        let nonempty_unshaped = DiscretionaryBranch {
            width: Length::from_raw(1).unwrap(),
            shaped: None,
            provenance: ItemProvenance::Text(parsed_span(0, 1)),
        };
        let discretionary = [ParagraphItem::Discretionary {
            no_break: Box::new(nonempty_unshaped),
            pre_break: Box::new(nonempty_unshaped),
            post_break: Box::new(nonempty_unshaped),
            penalty: 0,
            flagged: false,
        }];
        assert_eq!(
            ParagraphInput::new(
                NodeId::new(1),
                binding,
                epoch,
                &discretionary,
                &shapes,
                LineShapeExhaustion::RepeatLast,
            ),
            Err(BreakError::InvalidEmptyDiscretionaryBranch)
        );
    }
}
