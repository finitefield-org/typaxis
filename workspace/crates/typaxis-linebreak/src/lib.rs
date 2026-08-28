#![forbid(unsafe_code)]

mod math;

pub use math::{AtomicMathInlineItem, AtomicMathPlacement, MathAtomicItemError};

mod unicode_linebreak;

pub use unicode_linebreak::{
    unicode_line_breaks, UnicodeBreak, UnicodeBreakKind, UnicodeLineBreakError,
    UNICODE_VERSION as UNICODE_LINE_BREAK_VERSION,
};

use typaxis_core::{
    push_jcs_string, sha256, BidiLevel, GeneratedBufferKey, GenerationKind, GlyphRunId, Length,
    NodeId, NonNegativeLength, PositiveLength, ReferenceFingerprint, TextOffset, TextSpan,
    Utf8ByteOffset, ValidatedResourceLimits,
};
use typaxis_document::{Block, DocumentNodeKind, Inline, ReferenceFormat};
use typaxis_layout_contract::LayoutEpoch;
use typaxis_shaping::{ItemizedShapeRequests, ShapeSourceSpan, ValidatedGlyphRun};
use typaxis_syntax::{
    PackageGeneratedTextBinding, PackageShapeTextReceipt, PackageShapeTextSource,
    ValidatedParsedPackage, ValidatedStagingLinkTarget, ValidatedStagingLinkUsageReceipt,
    ValidatedStagingStylePackage,
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum JapaneseLineBreakMode {
    Loose,
    #[default]
    Normal,
    Strict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JapanesePairPermission {
    Preserve,
    Prohibit,
}

/// Versioned horizontal Japanese pair-table output. Gap values are signed
/// 1/1024-em units so they remain independent of a caller's font size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JapanesePairRule {
    permission: JapanesePairPermission,
    penalty: i32,
    natural_gap_per_1024_em: i16,
    stretch_per_1024_em: u16,
    shrink_per_1024_em: u16,
    priority: u8,
}
impl JapanesePairRule {
    pub const fn permission(self) -> JapanesePairPermission {
        self.permission
    }
    pub const fn penalty(self) -> i32 {
        self.penalty
    }
    pub const fn natural_gap_per_1024_em(self) -> i16 {
        self.natural_gap_per_1024_em
    }
    pub const fn stretch_per_1024_em(self) -> u16 {
        self.stretch_per_1024_em
    }
    pub const fn shrink_per_1024_em(self) -> u16 {
        self.shrink_per_1024_em
    }
    pub const fn priority(self) -> u8 {
        self.priority
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineShape {
    pub inline_size: PositiveLength,
}

/// Typed spacing parameters for the canonical paragraph factory's U+0020
/// glue. Natural width always comes from the shaped cluster and cannot be
/// supplied by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReferenceSpaceGlue {
    stretch: NonNegativeLength,
    shrink: NonNegativeLength,
}
impl ReferenceSpaceGlue {
    pub const fn new(stretch: NonNegativeLength, shrink: NonNegativeLength) -> Self {
        Self { stretch, shrink }
    }
    pub const fn stretch(self) -> NonNegativeLength {
        self.stretch
    }
    pub const fn shrink(self) -> NonNegativeLength {
        self.shrink
    }
}

/// Canonical paragraph-item IR. Break legality is explicit; stretch and shrink
/// are never reconstructed from glyph coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemProvenance {
    Text(TextSpan),
    Generated(GeneratedProvenance),
}

/// Dense index into the exact validated run table owned by one canonical
/// paragraph. Original shaping run IDs may restart at each text site, so they
/// are not sufficient as paragraph-wide lookup keys.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParagraphRunIndex(u32);
impl ParagraphRunIndex {
    const fn new(value: u32) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShapedSlice {
    paragraph_run_index: ParagraphRunIndex,
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
    fn from_cluster(
        run: &ValidatedGlyphRun,
        paragraph_run_index: ParagraphRunIndex,
        logical_ordinal: u32,
    ) -> Result<Self, BreakError> {
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
            paragraph_run_index,
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
    pub const fn paragraph_run_index(self) -> ParagraphRunIndex {
        self.paragraph_run_index
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
        shaped: ShapedSlice,
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
    paragraph_level: BidiLevel,
    runs: &'a [ValidatedGlyphRun],
    items: &'a [ParagraphItem],
    line_shapes: &'a [LineShape],
    line_shape_exhaustion: LineShapeExhaustion,
}
impl<'a> ParagraphInput<'a> {
    /// Private promotion boundary used by the crate-owned paragraph factory.
    /// Downstream callers cannot promote an arbitrary item vector.
    #[allow(clippy::too_many_arguments)]
    fn new(
        paragraph_node: NodeId,
        generated_text: PackageGeneratedTextBinding<'_>,
        epoch: LayoutEpoch,
        paragraph_level: BidiLevel,
        runs: &'a [ValidatedGlyphRun],
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
        if paragraph_level.get() > 1 {
            return Err(BreakError::InvalidParagraphBidiLevel);
        }
        if !matches!(
            items.last(),
            Some(ParagraphItem::Penalty {
                kind: BreakKind::Mandatory,
                ..
            })
        ) {
            return Err(BreakError::MissingTerminalBreak);
        }
        let mut generated = std::collections::BTreeSet::new();
        for item in items {
            validate_item(item, paragraph_node, generated_text, epoch, runs)?;
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
            paragraph_level,
            runs,
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
    pub const fn paragraph_level(&self) -> BidiLevel {
        self.paragraph_level
    }
    pub const fn runs(&self) -> &[ValidatedGlyphRun] {
        self.runs
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

/// One complete package-declared text site and the validated shaping runs
/// which cover it in logical order. Nonempty construction requires the exact
/// canonical itemizer owner; [`BoundedReferenceParagraphFactory`] then
/// rebinds the package receipt and validates every request/run/cluster before
/// issuing a paragraph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParagraphShapedText<'a> {
    receipt: PackageShapeTextReceipt<'a>,
    itemized: Option<&'a ItemizedShapeRequests<'a>>,
    runs: &'a [ValidatedGlyphRun],
}
impl<'a> ParagraphShapedText<'a> {
    /// Binds shaped output to the exact canonical itemization owner which
    /// issued its run requests and paragraph embedding level.
    pub fn from_itemized(
        itemized: &'a ItemizedShapeRequests<'a>,
        runs: &'a [ValidatedGlyphRun],
    ) -> Self {
        Self {
            receipt: itemized.text_receipt(),
            itemized: Some(itemized),
            runs,
        }
    }
    /// Represents a package-issued empty generated site. The canonical
    /// itemizer rejects empty work, so no itemization receipt exists here;
    /// the paragraph factory rechecks that both text and run list are empty.
    pub const fn empty(receipt: PackageShapeTextReceipt<'a>) -> Self {
        Self {
            receipt,
            itemized: None,
            runs: &[],
        }
    }
    pub const fn receipt(self) -> PackageShapeTextReceipt<'a> {
        self.receipt
    }
    pub const fn runs(self) -> &'a [ValidatedGlyphRun] {
        self.runs
    }
    pub const fn itemized(self) -> Option<&'a ItemizedShapeRequests<'a>> {
        self.itemized
    }
}

/// Owned output of the canonical factory. `ParagraphInput` remains borrowed
/// and can only be projected from this crate-issued value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalParagraph {
    paragraph_node: NodeId,
    epoch: LayoutEpoch,
    reference_fingerprint: ReferenceFingerprint,
    paragraph_level: BidiLevel,
    runs: Vec<ValidatedGlyphRun>,
    items: Vec<ParagraphItem>,
    line_shapes: Vec<LineShape>,
    line_shape_exhaustion: LineShapeExhaustion,
}
impl CanonicalParagraph {
    pub const fn paragraph_node(&self) -> NodeId {
        self.paragraph_node
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn reference_fingerprint(&self) -> ReferenceFingerprint {
        self.reference_fingerprint
    }
    pub const fn paragraph_level(&self) -> BidiLevel {
        self.paragraph_level
    }
    pub fn runs(&self) -> &[ValidatedGlyphRun] {
        &self.runs
    }
    pub fn items(&self) -> &[ParagraphItem] {
        &self.items
    }
    pub fn line_shapes(&self) -> &[LineShape] {
        &self.line_shapes
    }
    pub const fn line_shape_exhaustion(&self) -> LineShapeExhaustion {
        self.line_shape_exhaustion
    }
    pub fn input(&self) -> ParagraphInput<'_> {
        ParagraphInput {
            paragraph_node: self.paragraph_node,
            epoch: self.epoch,
            reference_fingerprint: self.reference_fingerprint,
            paragraph_level: self.paragraph_level,
            runs: &self.runs,
            items: &self.items,
            line_shapes: &self.line_shapes,
            line_shape_exhaustion: self.line_shape_exhaustion,
        }
    }
}

const REFERENCE_SPACE_PRIORITY: u8 = 0;
/// Mandatory behavior is carried exclusively by `BreakKind::Mandatory`.
/// Keeping its numeric cost neutral avoids making the terminal edge a reward
/// that can distort otherwise equal break paths.
const MANDATORY_BREAK_COST: i32 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum JapanesePairClass {
    Opening,
    Closing,
    SmallKana,
    Nonstarter,
    Japanese,
    Latin,
    Numeric,
    Space,
    Other,
}

/// Looks up the registered `typaxis-jlreq-horizontal/1.0.0` pair rule.
/// UAX #14 remains the candidate source; `Prohibit` can only remove a
/// candidate and never invent a boundary which Unicode prohibited.
pub fn japanese_pair_rule(
    left: Option<char>,
    right: Option<char>,
    mode: JapaneseLineBreakMode,
) -> JapanesePairRule {
    const PRESERVE: JapanesePairRule = JapanesePairRule {
        permission: JapanesePairPermission::Preserve,
        penalty: 0,
        natural_gap_per_1024_em: 0,
        stretch_per_1024_em: 0,
        shrink_per_1024_em: 0,
        priority: 0,
    };
    let (Some(left), Some(right)) = (left, right) else {
        return PRESERVE;
    };
    let left = japanese_pair_class(left);
    let right = japanese_pair_class(right);

    let prohibited = left == JapanesePairClass::Opening
        || right == JapanesePairClass::Closing
        || (mode != JapaneseLineBreakMode::Loose
            && matches!(
                right,
                JapanesePairClass::SmallKana | JapanesePairClass::Nonstarter
            ));
    if prohibited {
        return JapanesePairRule {
            permission: JapanesePairPermission::Prohibit,
            penalty: 0,
            natural_gap_per_1024_em: 0,
            stretch_per_1024_em: 0,
            shrink_per_1024_em: 0,
            priority: 0,
        };
    }

    let japanese = |class| {
        matches!(
            class,
            JapanesePairClass::Opening
                | JapanesePairClass::Closing
                | JapanesePairClass::SmallKana
                | JapanesePairClass::Nonstarter
                | JapanesePairClass::Japanese
        )
    };
    if japanese(left) && japanese(right) {
        return JapanesePairRule {
            permission: JapanesePairPermission::Preserve,
            penalty: 0,
            natural_gap_per_1024_em: 0,
            stretch_per_1024_em: 256,
            shrink_per_1024_em: 0,
            priority: 1,
        };
    }
    if (japanese(left) && matches!(right, JapanesePairClass::Latin | JapanesePairClass::Numeric))
        || (matches!(left, JapanesePairClass::Latin | JapanesePairClass::Numeric)
            && japanese(right))
    {
        return JapanesePairRule {
            permission: JapanesePairPermission::Preserve,
            penalty: if mode == JapaneseLineBreakMode::Strict {
                100
            } else {
                50
            },
            natural_gap_per_1024_em: 128,
            stretch_per_1024_em: 64,
            shrink_per_1024_em: 64,
            priority: 2,
        };
    }
    if left == JapanesePairClass::Space || right == JapanesePairClass::Space {
        return JapanesePairRule {
            permission: JapanesePairPermission::Preserve,
            penalty: 0,
            natural_gap_per_1024_em: 0,
            stretch_per_1024_em: 512,
            shrink_per_1024_em: 256,
            priority: 4,
        };
    }
    PRESERVE
}

fn japanese_pair_rule_at(
    text: &str,
    byte_offset: usize,
    mode: JapaneseLineBreakMode,
) -> JapanesePairRule {
    let left = text
        .get(..byte_offset)
        .and_then(|prefix| prefix.chars().next_back());
    let right = text
        .get(byte_offset..)
        .and_then(|suffix| suffix.chars().next());
    japanese_pair_rule(left, right, mode)
}

fn japanese_pair_class(character: char) -> JapanesePairClass {
    if "（〔［｛〈《「『【〘〖〝‘“".contains(character) {
        JapanesePairClass::Opening
    } else if "）〕］｝〉》」』】〙〗〟’”、。，．・：；？！‼⁇⁈⁉".contains(character)
    {
        JapanesePairClass::Closing
    } else if "ぁぃぅぇぉっゃゅょゎゕゖァィゥェォッャュョヮヵヶㇰㇱㇲㇳㇴㇵㇶㇷㇸㇹㇺㇻㇼㇽㇾㇿ"
        .contains(character)
    {
        JapanesePairClass::SmallKana
    } else if "ー〜～…‥ヽヾゝゞ々〻".contains(character) {
        JapanesePairClass::Nonstarter
    } else if matches!(
        character as u32,
        0x2e80..=0x2fff
            | 0x3040..=0x30ff
            | 0x31f0..=0x31ff
            | 0x3400..=0x4dbf
            | 0x4e00..=0x9fff
            | 0xf900..=0xfaff
            | 0xff66..=0xff9f
            | 0x20000..=0x3ffff
    ) {
        JapanesePairClass::Japanese
    } else if character.is_ascii_alphabetic() {
        JapanesePairClass::Latin
    } else if character.is_numeric() {
        JapanesePairClass::Numeric
    } else if character.is_whitespace() {
        JapanesePairClass::Space
    } else {
        JapanesePairClass::Other
    }
}

/// Bounded canonical paragraph factory using the Unicode 16.0 default UAX #14
/// rules. Legal boundaries are computed over the complete logical paragraph,
/// including boundaries between package text sites, and then intersected with
/// validated shaping-cluster boundaries so the breaker can never split a
/// cluster. U+0020 clusters retain explicit Glue metrics; all other clusters
/// are immutable Boxes. Every cluster is followed by an explicit Penalty that
/// records whether its Unicode boundary is prohibited, allowed, or mandatory.
/// Japanese tailoring is a separate resolved-data-table layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundedReferenceParagraphFactory {
    japanese_mode: JapaneseLineBreakMode,
}
impl Default for BoundedReferenceParagraphFactory {
    fn default() -> Self {
        Self::new()
    }
}
impl BoundedReferenceParagraphFactory {
    pub const fn new() -> Self {
        Self {
            japanese_mode: JapaneseLineBreakMode::Normal,
        }
    }
    pub const fn with_japanese_mode(mode: JapaneseLineBreakMode) -> Self {
        Self {
            japanese_mode: mode,
        }
    }
    pub const fn japanese_mode(self) -> JapaneseLineBreakMode {
        self.japanese_mode
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build(
        &self,
        generated_text: PackageGeneratedTextBinding<'_>,
        paragraph_node: NodeId,
        epoch: LayoutEpoch,
        shaped_text: &[ParagraphShapedText<'_>],
        space_glue: ReferenceSpaceGlue,
        line_shapes: &[LineShape],
        line_shape_exhaustion: LineShapeExhaustion,
        limits: &ValidatedResourceLimits,
    ) -> Result<CanonicalParagraph, BreakError> {
        self.build_internal(
            generated_text,
            paragraph_node,
            epoch,
            shaped_text,
            space_glue,
            line_shapes,
            line_shape_exhaustion,
            limits,
            false,
        )
    }

    /// Footnote-profile paragraph factory. This is the only entry point that
    /// admits definition-owned paragraphs and their generated definition
    /// marker; the frozen ordinary factory continues to search body content
    /// only.
    #[allow(clippy::too_many_arguments)]
    pub fn build_with_footnotes(
        &self,
        generated_text: PackageGeneratedTextBinding<'_>,
        paragraph_node: NodeId,
        epoch: LayoutEpoch,
        shaped_text: &[ParagraphShapedText<'_>],
        space_glue: ReferenceSpaceGlue,
        line_shapes: &[LineShape],
        line_shape_exhaustion: LineShapeExhaustion,
        limits: &ValidatedResourceLimits,
    ) -> Result<CanonicalParagraph, BreakError> {
        self.build_internal(
            generated_text,
            paragraph_node,
            epoch,
            shaped_text,
            space_glue,
            line_shapes,
            line_shape_exhaustion,
            limits,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build_internal(
        &self,
        generated_text: PackageGeneratedTextBinding<'_>,
        paragraph_node: NodeId,
        epoch: LayoutEpoch,
        shaped_text: &[ParagraphShapedText<'_>],
        space_glue: ReferenceSpaceGlue,
        line_shapes: &[LineShape],
        line_shape_exhaustion: LineShapeExhaustion,
        limits: &ValidatedResourceLimits,
        allow_footnotes: bool,
    ) -> Result<CanonicalParagraph, BreakError> {
        validate_factory_owner(generated_text, paragraph_node, epoch)?;
        if line_shapes.is_empty() {
            return Err(BreakError::EmptyLineShapes);
        }

        let paragraph = find_paragraph_block(
            &generated_text.package().package().document,
            paragraph_node,
            allow_footnotes,
        )
        .ok_or(BreakError::InvalidParagraphOwner)?;
        let definition_marker = allow_footnotes
            .then(|| {
                generated_text
                    .generated_text()
                    .buffers()
                    .iter()
                    .find_map(|buffer| {
                        let key = buffer.key();
                        if key.generation_kind() != GenerationKind::FootnoteMarker
                            || generated_text
                                .package()
                                .document_nodes()
                                .node_kind(key.owner())
                                != Some(DocumentNodeKind::FootnoteDefinition)
                        {
                            return None;
                        }
                        let end = u32::try_from(buffer.utf8().len()).ok()?;
                        let provenance = generated_text
                            .generated_text()
                            .provenance(
                                key,
                                typaxis_core::Utf8ByteOffset::new(0),
                                typaxis_core::Utf8ByteOffset::new(end),
                            )
                            .ok()?;
                        generated_text
                            .bind_generated_shape_text(provenance)
                            .ok()
                            .filter(|receipt| receipt.style_owner() == paragraph_node)
                            .map(|_| key)
                    })
            })
            .flatten();
        let mut expected_elements = Vec::new();
        collect_paragraph_elements(paragraph, definition_marker, &mut expected_elements)?;
        let expected_site_count = expected_elements
            .iter()
            .filter(|element| matches!(element, ExpectedParagraphElement::Text(_)))
            .count();
        if expected_site_count != shaped_text.len() {
            return Err(BreakError::ParagraphTextSiteMismatch);
        }

        let mut cluster_count = 0u64;
        let mut run_count = 0usize;
        let mut expected_tables = None;
        let mut expected_shaper = None;
        let mut paragraph_level = None;
        for (expected, shaped) in expected_elements
            .iter()
            .filter_map(|element| match element {
                ExpectedParagraphElement::Text(site) => Some(site),
                ExpectedParagraphElement::ExplicitBreak { .. } => None,
            })
            .zip(shaped_text)
        {
            validate_complete_site(generated_text, paragraph_node, expected, shaped.receipt)?;
            validate_site_run_coverage(
                generated_text,
                paragraph_node,
                epoch,
                shaped.receipt,
                shaped.itemized,
                shaped.runs,
                &mut paragraph_level,
                &mut expected_tables,
                &mut expected_shaper,
            )?;
            let site_clusters = u64::try_from(
                shaped
                    .runs
                    .iter()
                    .try_fold(0usize, |count, run| count.checked_add(run.clusters().len()))
                    .ok_or(BreakError::ArithmeticOverflow)?,
            )
            .map_err(|_| BreakError::ArithmeticOverflow)?;
            cluster_count = cluster_count
                .checked_add(site_clusters)
                .ok_or(BreakError::ArithmeticOverflow)?;
            run_count = run_count
                .checked_add(shaped.runs.len())
                .ok_or(BreakError::ArithmeticOverflow)?;
        }

        let paragraph_text_bytes = shaped_text.iter().try_fold(0usize, |total, shaped| {
            total
                .checked_add(shaped.receipt.utf8().len())
                .ok_or(BreakError::ArithmeticOverflow)
        })?;
        let item_capacity = preflight_factory_limits(
            cluster_count,
            expected_elements
                .len()
                .checked_sub(expected_site_count)
                .ok_or(BreakError::ArithmeticOverflow)?,
            paragraph_text_bytes,
            line_shapes.len(),
            limits,
        )?;
        let mut paragraph_utf8 = String::new();
        paragraph_utf8
            .try_reserve_exact(paragraph_text_bytes)
            .map_err(|_| BreakError::AllocationFailure)?;
        for shaped in shaped_text {
            paragraph_utf8.push_str(shaped.receipt.utf8());
        }
        let unicode_breaks =
            unicode_line_breaks(&paragraph_utf8).map_err(|_| BreakError::AllocationFailure)?;
        let mut items = Vec::new();
        items
            .try_reserve_exact(item_capacity)
            .map_err(|_| BreakError::AllocationFailure)?;
        let mut owned_runs = Vec::new();
        owned_runs
            .try_reserve_exact(run_count)
            .map_err(|_| BreakError::AllocationFailure)?;
        let mut paragraph_run_index = 0u32;
        let mut paragraph_byte_cursor = 0usize;
        let mut unicode_break_index = 0usize;
        let mut shaped_sites = shaped_text.iter();
        let mut definition_prefix_needs_source = definition_marker.is_some();
        for (element_index, element) in expected_elements.iter().enumerate() {
            let ExpectedParagraphElement::Text(_) = element else {
                let ExpectedParagraphElement::ExplicitBreak { node_id, mut kind } = *element else {
                    unreachable!()
                };
                kind = protect_definition_marker_prefix_break(kind, definition_prefix_needs_source);
                if element_index
                    .checked_add(1)
                    .is_some_and(|index| index == expected_elements.len())
                {
                    kind = BreakKind::Mandatory;
                }
                if let Some(ParagraphItem::Penalty {
                    kind: previous_kind,
                    provenance,
                    ..
                }) = items.last_mut()
                {
                    if *previous_kind == BreakKind::Mandatory
                        && !matches!(
                            provenance,
                            ItemProvenance::Generated(provenance)
                                if provenance.buffer_key().generation_kind()
                                    == GenerationKind::Discretionary
                        )
                    {
                        *previous_kind = BreakKind::Prohibited;
                    }
                }
                let key = GeneratedBufferKey::new(node_id, GenerationKind::Discretionary, 0);
                let provenance = complete_generated_provenance(generated_text, key)?;
                let receipt = generated_text
                    .bind_generated_shape_text(provenance)
                    .map_err(|_| BreakError::InvalidGeneratedProvenance)?;
                if receipt.site_owner() != node_id
                    || receipt.style_owner() != paragraph_node
                    || !receipt.covers_complete_site()
                    || !receipt.utf8().is_empty()
                {
                    return Err(BreakError::InvalidGeneratedProvenance);
                }
                items.push(ParagraphItem::Penalty {
                    width: Length::ZERO,
                    cost: MANDATORY_BREAK_COST,
                    kind,
                    flagged: false,
                    provenance: ItemProvenance::Generated(provenance),
                });
                continue;
            };
            let is_definition_marker = matches!(
                element,
                ExpectedParagraphElement::Text(ExpectedParagraphTextSite::Generated {
                    key,
                    ..
                }) if Some(*key) == definition_marker
            );
            let shaped = shaped_sites
                .next()
                .ok_or(BreakError::ParagraphTextSiteMismatch)?;
            for run in shaped.runs {
                owned_runs.push(run.clone());
                for ordinal in 0..run.clusters().len() {
                    let ordinal =
                        u32::try_from(ordinal).map_err(|_| BreakError::ArithmeticOverflow)?;
                    let slice = ShapedSlice::from_cluster(
                        run,
                        ParagraphRunIndex::new(paragraph_run_index),
                        ordinal,
                    )?;
                    let cluster_receipt = bind_shape_source(generated_text, slice.source())?;
                    if cluster_receipt.site_owner() != run.site_owner()
                        || cluster_receipt.style_owner() != paragraph_node
                    {
                        return Err(BreakError::InvalidItemOwner);
                    }
                    let provenance = provenance_from_shape_source(slice.source());
                    if cluster_receipt.utf8() == " " {
                        if space_glue.shrink().get().raw() > slice.derived_width().get().raw() {
                            return Err(BreakError::SpaceShrinkExceedsNatural);
                        }
                        items.push(ParagraphItem::Glue {
                            natural: slice.derived_width(),
                            stretch: space_glue.stretch(),
                            shrink: space_glue.shrink(),
                            priority: REFERENCE_SPACE_PRIORITY,
                            shaped: slice,
                            provenance,
                        });
                    } else {
                        items.push(ParagraphItem::Box {
                            width: slice.derived_width(),
                            shaped: slice,
                            provenance,
                        });
                    }
                    paragraph_byte_cursor = paragraph_byte_cursor
                        .checked_add(cluster_receipt.utf8().len())
                        .ok_or(BreakError::ArithmeticOverflow)?;
                    let mut boundary_kind = BreakKind::Prohibited;
                    while let Some(boundary) = unicode_breaks.get(unicode_break_index) {
                        if boundary.byte_offset() > paragraph_byte_cursor {
                            break;
                        }
                        if boundary.byte_offset() < paragraph_byte_cursor {
                            if boundary.kind() == UnicodeBreakKind::Mandatory {
                                return Err(BreakError::MandatoryBreakInsideCluster);
                            }
                            unicode_break_index = unicode_break_index
                                .checked_add(1)
                                .ok_or(BreakError::ArithmeticOverflow)?;
                            continue;
                        }
                        boundary_kind = match boundary.kind() {
                            UnicodeBreakKind::Allowed => BreakKind::Allowed,
                            UnicodeBreakKind::Mandatory => BreakKind::Mandatory,
                        };
                        unicode_break_index = unicode_break_index
                            .checked_add(1)
                            .ok_or(BreakError::ArithmeticOverflow)?;
                        break;
                    }
                    let pair_rule = japanese_pair_rule_at(
                        &paragraph_utf8,
                        paragraph_byte_cursor,
                        self.japanese_mode,
                    );
                    if boundary_kind != BreakKind::Mandatory
                        && pair_rule.permission() == JapanesePairPermission::Prohibit
                    {
                        boundary_kind = BreakKind::Prohibited;
                    }
                    // ADR-0030 makes the definition marker and the first
                    // source line one indivisible unit. Unicode data for a
                    // generated decimal marker must never introduce a legal
                    // break before the marker gap and first source cluster.
                    if is_definition_marker {
                        boundary_kind = BreakKind::Prohibited;
                    }
                    items.push(ParagraphItem::Penalty {
                        width: Length::ZERO,
                        cost: if boundary_kind == BreakKind::Allowed {
                            pair_rule.penalty()
                        } else {
                            MANDATORY_BREAK_COST
                        },
                        kind: boundary_kind,
                        flagged: false,
                        provenance: boundary_provenance(
                            slice.source(),
                            paragraph_node,
                            generated_text,
                        )?,
                    });
                    if !is_definition_marker && slice.derived_width().get() > Length::ZERO {
                        definition_prefix_needs_source = false;
                    }
                }
                paragraph_run_index = paragraph_run_index
                    .checked_add(1)
                    .ok_or(BreakError::ArithmeticOverflow)?;
            }
        }
        if shaped_sites.next().is_some() {
            return Err(BreakError::ParagraphTextSiteMismatch);
        }
        if paragraph_utf8.is_empty()
            && unicode_breaks.len() == 1
            && unicode_breaks[0].byte_offset() == 0
            && unicode_breaks[0].kind() == UnicodeBreakKind::Mandatory
        {
            unicode_break_index = 1;
        }

        if !matches!(
            items.last(),
            Some(ParagraphItem::Penalty {
                kind: BreakKind::Mandatory,
                ..
            })
        ) {
            let provenance = items
                .last()
                .and_then(|item| item_provenances(item).into_iter().next())
                .copied()
                .or_else(|| {
                    shaped_text
                        .first()
                        .map(|shaped| match shaped.receipt.source() {
                            PackageShapeTextSource::Generated(provenance) => {
                                ItemProvenance::Generated(provenance)
                            }
                            PackageShapeTextSource::Parsed(span) => ItemProvenance::Text(span),
                        })
                })
                .ok_or(BreakError::EmptyParagraphItems)?;
            items.push(ParagraphItem::Penalty {
                width: Length::ZERO,
                cost: MANDATORY_BREAK_COST,
                kind: BreakKind::Mandatory,
                flagged: false,
                provenance,
            });
        }

        if paragraph_byte_cursor != paragraph_utf8.len()
            || unicode_break_index != unicode_breaks.len()
            || !matches!(
                items.last(),
                Some(ParagraphItem::Penalty {
                    kind: BreakKind::Mandatory,
                    ..
                })
            )
        {
            return Err(BreakError::MalformedRunCoverage);
        }

        let mut owned_line_shapes = Vec::new();
        owned_line_shapes
            .try_reserve_exact(line_shapes.len())
            .map_err(|_| BreakError::AllocationFailure)?;
        owned_line_shapes.extend_from_slice(line_shapes);
        {
            ParagraphInput::new(
                paragraph_node,
                generated_text,
                epoch,
                paragraph_level.unwrap_or(BidiLevel::LTR),
                &owned_runs,
                &items,
                &owned_line_shapes,
                line_shape_exhaustion,
            )?;
        }
        Ok(CanonicalParagraph {
            paragraph_node,
            epoch,
            reference_fingerprint: generated_text.generated_text().reference_fingerprint(),
            paragraph_level: paragraph_level.unwrap_or(BidiLevel::LTR),
            runs: owned_runs,
            items,
            line_shapes: owned_line_shapes,
            line_shape_exhaustion,
        })
    }
}

const fn protect_definition_marker_prefix_break(
    kind: BreakKind,
    definition_prefix_needs_source: bool,
) -> BreakKind {
    if definition_prefix_needs_source && matches!(kind, BreakKind::Allowed) {
        BreakKind::Prohibited
    } else {
        kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedParagraphTextSite {
    Parsed {
        owner: NodeId,
        span: TextSpan,
    },
    Generated {
        owner: NodeId,
        key: GeneratedBufferKey,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedParagraphElement {
    Text(ExpectedParagraphTextSite),
    ExplicitBreak { node_id: NodeId, kind: BreakKind },
}

fn validate_factory_owner(
    generated_text: PackageGeneratedTextBinding<'_>,
    paragraph_node: NodeId,
    epoch: LayoutEpoch,
) -> Result<(), BreakError> {
    let package = generated_text.package();
    if !matches!(
        package.document_nodes().node_kind(paragraph_node),
        Some(DocumentNodeKind::Paragraph | DocumentNodeKind::Heading)
    ) {
        return Err(BreakError::InvalidParagraphOwner);
    }
    if epoch.document() != package.epoch_identity().document()
        || epoch.style() != package.epoch_identity().style()
        || epoch.references() != generated_text.generated_text().reference_fingerprint()
    {
        return Err(BreakError::ParagraphEpochMismatch);
    }
    Ok(())
}

fn find_paragraph_block(
    document: &typaxis_document::Document,
    owner: NodeId,
    allow_footnotes: bool,
) -> Option<&Block> {
    let mut pending: Vec<&Block> = document.blocks.iter().rev().collect();
    if allow_footnotes {
        pending.extend(
            document
                .footnotes
                .iter()
                .rev()
                .flat_map(|footnote| footnote.blocks.iter().rev()),
        );
    }
    while let Some(block) = pending.pop() {
        if matches!(
            block,
            Block::Paragraph { node_id, .. } | Block::Heading { node_id, .. }
                if *node_id == owner
        ) {
            return Some(block);
        }
        match block {
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
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    None
}

fn collect_paragraph_elements(
    paragraph: &Block,
    definition_marker: Option<GeneratedBufferKey>,
    output: &mut Vec<ExpectedParagraphElement>,
) -> Result<(), BreakError> {
    let children = match paragraph {
        Block::Paragraph { children, .. } | Block::Heading { children, .. } => children,
        _ => return Err(BreakError::InvalidParagraphOwner),
    };
    if let Some(key) = definition_marker {
        output.push(ExpectedParagraphElement::Text(
            ExpectedParagraphTextSite::Generated {
                owner: key.owner(),
                key,
            },
        ));
    }
    let mut pending: Vec<&Inline> = children.iter().rev().collect();
    while let Some(inline) = pending.pop() {
        match inline {
            Inline::Text {
                node_id, text_span, ..
            } => output.push(ExpectedParagraphElement::Text(
                ExpectedParagraphTextSite::Parsed {
                    owner: *node_id,
                    span: *text_span,
                },
            )),
            Inline::Reference {
                node_id, format, ..
            } => {
                let kind = match format {
                    ReferenceFormat::Page => GenerationKind::PageReference,
                    ReferenceFormat::Text | ReferenceFormat::Number => GenerationKind::Counter,
                };
                output.push(ExpectedParagraphElement::Text(
                    ExpectedParagraphTextSite::Generated {
                        owner: *node_id,
                        key: GeneratedBufferKey::new(*node_id, kind, 0),
                    },
                ));
            }
            Inline::FootnoteReference { node_id, .. } => {
                output.push(ExpectedParagraphElement::Text(
                    ExpectedParagraphTextSite::Generated {
                        owner: *node_id,
                        key: GeneratedBufferKey::new(*node_id, GenerationKind::FootnoteMarker, 0),
                    },
                ));
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => pending.extend(children.iter().rev()),
            Inline::Anchor { .. } => {}
            Inline::SoftBreak { node_id, .. } | Inline::HardBreak { node_id, .. } => {
                output.push(ExpectedParagraphElement::ExplicitBreak {
                    node_id: *node_id,
                    kind: if matches!(inline, Inline::SoftBreak { .. }) {
                        BreakKind::Allowed
                    } else {
                        BreakKind::Mandatory
                    },
                });
            }
        }
    }
    Ok(())
}

fn validate_complete_site(
    generated_text: PackageGeneratedTextBinding<'_>,
    paragraph_node: NodeId,
    expected: &ExpectedParagraphTextSite,
    receipt: PackageShapeTextReceipt<'_>,
) -> Result<(), BreakError> {
    let rebound = match receipt.source() {
        PackageShapeTextSource::Parsed(span) => generated_text
            .package()
            .bind_parsed_shape_text(span)
            .map_err(|_| BreakError::InvalidItemProvenance)?,
        PackageShapeTextSource::Generated(provenance) => generated_text
            .bind_generated_shape_text(provenance)
            .map_err(|_| BreakError::InvalidItemProvenance)?,
    };
    if rebound != receipt
        || !receipt.covers_complete_site()
        || receipt.style_owner() != paragraph_node
    {
        return Err(BreakError::ParagraphTextSiteMismatch);
    }
    match (expected, receipt.source()) {
        (
            ExpectedParagraphTextSite::Parsed { owner, span },
            PackageShapeTextSource::Parsed(actual),
        ) if *owner == receipt.site_owner() && *span == actual => Ok(()),
        (
            ExpectedParagraphTextSite::Generated { owner, key },
            PackageShapeTextSource::Generated(actual),
        ) if *owner == receipt.site_owner() && *key == actual.buffer_key() => Ok(()),
        _ => Err(BreakError::ParagraphTextSiteMismatch),
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_site_run_coverage<'a>(
    generated_text: PackageGeneratedTextBinding<'_>,
    paragraph_node: NodeId,
    epoch: LayoutEpoch,
    receipt: PackageShapeTextReceipt<'_>,
    itemized: Option<&ItemizedShapeRequests<'_>>,
    runs: &'a [ValidatedGlyphRun],
    expected_paragraph_level: &mut Option<BidiLevel>,
    expected_tables: &mut Option<&'a typaxis_core::ResolvedDataTables>,
    expected_shaper: &mut Option<typaxis_core::ShaperIdentity>,
) -> Result<(), BreakError> {
    if receipt.utf8().is_empty() {
        return if itemized.is_none() && runs.is_empty() {
            Ok(())
        } else {
            Err(BreakError::MalformedRunCoverage)
        };
    }
    let itemized = itemized.ok_or(BreakError::MissingCanonicalItemization)?;
    let paragraph_level = itemized.paragraph_level();
    if itemized.text_receipt() != receipt
        || paragraph_level.get() > 1
        || itemized.requests().len() != runs.len()
        || runs.is_empty()
    {
        return Err(BreakError::MalformedRunCoverage);
    }
    match expected_paragraph_level {
        Some(expected) if *expected != paragraph_level => {
            return Err(BreakError::ParagraphBidiLevelMismatch)
        }
        None => *expected_paragraph_level = Some(paragraph_level),
        _ => {}
    }
    let expected_source = shape_source(receipt.source());
    let (mut cursor, expected_end) = shape_source_bounds(expected_source);
    for (index, (request, run)) in itemized.requests().iter().zip(runs).enumerate() {
        let expected_run_id = u32::try_from(index).map_err(|_| BreakError::ArithmeticOverflow)?;
        if run.run_id().get() != expected_run_id
            || request.run_id() != run.run_id()
            || request.text().source() != run.source_span()
            || request.bidi_level() != run.bidi_level()
            || request.font() != run.font()
            || request.layout_epoch() != epoch
            || run.epoch() != epoch
            || run.site_owner() != receipt.site_owner()
            || run.style_owner() != paragraph_node
            || !same_shape_source_namespace(run.source_span(), expected_source)
        {
            return Err(BreakError::MalformedRunCoverage);
        }
        let (run_start, run_end) = shape_source_bounds(run.source_span());
        if run_start != cursor || run_start >= run_end || run_end > expected_end {
            return Err(BreakError::MalformedRunCoverage);
        }
        let run_receipt = bind_shape_source(generated_text, run.source_span())?;
        if run_receipt.site_owner() != receipt.site_owner()
            || run_receipt.style_owner() != paragraph_node
        {
            return Err(BreakError::MalformedRunCoverage);
        }
        match expected_tables {
            Some(tables) if *tables != run.data_tables() => {
                return Err(BreakError::ShapingDataTablesMismatch)
            }
            None => *expected_tables = Some(run.data_tables()),
            _ => {}
        }
        match expected_shaper {
            Some(shaper) if *shaper != run.shaper_identity() => {
                return Err(BreakError::ShaperIdentityMismatch)
            }
            None => *expected_shaper = Some(run.shaper_identity()),
            _ => {}
        }
        validate_cluster_source_coverage(
            run.source_span(),
            run.clusters().iter().map(|cluster| cluster.source_span),
        )?;
        cursor = run_end;
    }
    if cursor != expected_end {
        return Err(BreakError::MalformedRunCoverage);
    }
    Ok(())
}

fn validate_cluster_source_coverage(
    run_source: ShapeSourceSpan,
    cluster_sources: impl IntoIterator<Item = ShapeSourceSpan>,
) -> Result<(), BreakError> {
    let (mut cursor, run_end) = shape_source_bounds(run_source);
    let mut saw_cluster = false;
    for source in cluster_sources {
        saw_cluster = true;
        if !same_shape_source_namespace(run_source, source) {
            return Err(BreakError::MalformedRunCoverage);
        }
        let (start, end) = shape_source_bounds(source);
        if start != cursor || start >= end || end > run_end {
            return Err(BreakError::MalformedRunCoverage);
        }
        cursor = end;
    }
    if !saw_cluster || cursor != run_end {
        return Err(BreakError::MalformedRunCoverage);
    }
    Ok(())
}

fn preflight_factory_limits(
    cluster_count: u64,
    explicit_break_count: usize,
    paragraph_text_bytes: usize,
    line_shape_count: usize,
    limits: &ValidatedResourceLimits,
) -> Result<usize, BreakError> {
    let paragraph_text_bytes =
        u64::try_from(paragraph_text_bytes).map_err(|_| BreakError::ArithmeticOverflow)?;
    if paragraph_text_bytes > limits.get().max_text_bytes {
        return Err(BreakError::ParagraphTextLimit);
    }
    let line_shape_count =
        u64::try_from(line_shape_count).map_err(|_| BreakError::ArithmeticOverflow)?;
    if line_shape_count > u64::from(limits.get().max_pages) {
        return Err(BreakError::LineShapeLimit);
    }
    let explicit_break_count =
        u64::try_from(explicit_break_count).map_err(|_| BreakError::ArithmeticOverflow)?;
    let item_capacity = cluster_count
        .checked_mul(2)
        .and_then(|capacity| capacity.checked_add(explicit_break_count))
        .and_then(|capacity| capacity.checked_add(1))
        .ok_or(BreakError::ArithmeticOverflow)?;
    usize::try_from(item_capacity).map_err(|_| BreakError::ArithmeticOverflow)
}

fn bind_shape_source(
    generated_text: PackageGeneratedTextBinding<'_>,
    source: ShapeSourceSpan,
) -> Result<PackageShapeTextReceipt<'_>, BreakError> {
    match source {
        ShapeSourceSpan::Parsed(span) => generated_text
            .package()
            .bind_parsed_shape_text(span)
            .map_err(|_| BreakError::InvalidItemProvenance),
        ShapeSourceSpan::Generated(provenance) => generated_text
            .bind_generated_shape_text(provenance)
            .map_err(|_| BreakError::InvalidItemProvenance),
    }
}

fn complete_generated_provenance(
    generated_text: PackageGeneratedTextBinding<'_>,
    key: GeneratedBufferKey,
) -> Result<GeneratedProvenance, BreakError> {
    let buffer = generated_text
        .generated_text()
        .buffers()
        .iter()
        .find(|buffer| buffer.key() == key)
        .ok_or(BreakError::InvalidGeneratedProvenance)?;
    let end = u32::try_from(buffer.utf8().len()).map_err(|_| BreakError::ArithmeticOverflow)?;
    generated_text
        .generated_text()
        .provenance(key, Utf8ByteOffset::new(0), Utf8ByteOffset::new(end))
        .map_err(|_| BreakError::InvalidGeneratedProvenance)
}

const fn provenance_from_shape_source(source: ShapeSourceSpan) -> ItemProvenance {
    match source {
        ShapeSourceSpan::Parsed(span) => ItemProvenance::Text(span),
        ShapeSourceSpan::Generated(provenance) => ItemProvenance::Generated(provenance),
    }
}

fn boundary_provenance(
    source: ShapeSourceSpan,
    paragraph_node: NodeId,
    generated_text: PackageGeneratedTextBinding<'_>,
) -> Result<ItemProvenance, BreakError> {
    let end = Utf8ByteOffset::new(shape_source_bounds(source).1);
    let point = match source {
        ShapeSourceSpan::Parsed(span) => {
            TextSpan::new(span.text_id(), end, end).map(ItemProvenance::Text)
        }
        ShapeSourceSpan::Generated(provenance) => {
            provenance.subspan(end, end).map(ItemProvenance::Generated)
        }
    };
    if let Some(point) = point {
        if validate_provenance(&point, paragraph_node, generated_text).is_ok() {
            return Ok(point);
        }
    }
    let complete = provenance_from_shape_source(source);
    validate_provenance(&complete, paragraph_node, generated_text)?;
    Ok(complete)
}

fn shape_source_bounds(source: ShapeSourceSpan) -> (u32, u32) {
    match source {
        ShapeSourceSpan::Parsed(span) => (span.start_byte().get(), span.end_byte().get()),
        ShapeSourceSpan::Generated(provenance) => {
            let range = provenance.text_span().range();
            (range.start_byte().get(), range.end_byte().get())
        }
    }
}

fn same_shape_source_namespace(left: ShapeSourceSpan, right: ShapeSourceSpan) -> bool {
    match (left, right) {
        (ShapeSourceSpan::Parsed(left), ShapeSourceSpan::Parsed(right)) => {
            left.text_id() == right.text_id()
        }
        (ShapeSourceSpan::Generated(left), ShapeSourceSpan::Generated(right)) => {
            left.buffer_key() == right.buffer_key()
                && left.text_span().text_id() == right.text_span().text_id()
        }
        _ => false,
    }
}

fn validate_item(
    item: &ParagraphItem,
    paragraph_node: NodeId,
    generated_text: PackageGeneratedTextBinding<'_>,
    epoch: LayoutEpoch,
    runs: &[ValidatedGlyphRun],
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
            validate_shaped_provenance(
                *shaped,
                provenance,
                paragraph_node,
                generated_text,
                epoch,
                runs,
            )
        }
        ParagraphItem::Glue {
            natural,
            shaped,
            provenance,
            ..
        } => {
            if *natural != shaped.derived_width() {
                return Err(BreakError::ShapedWidthMismatch);
            }
            validate_shaped_provenance(
                *shaped,
                provenance,
                paragraph_node,
                generated_text,
                epoch,
                runs,
            )
        }
        ParagraphItem::Penalty { provenance, .. } => {
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
                    validate_shaped_receipt(shaped, &receipt, epoch, runs)?;
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
    runs: &[ValidatedGlyphRun],
) -> Result<(), BreakError> {
    let receipt = validate_provenance(provenance, paragraph_node, generated_text)?;
    validate_shaped_receipt(shaped, &receipt, epoch, runs)
}

fn validate_shaped_receipt(
    shaped: ShapedSlice,
    receipt: &PackageShapeTextReceipt<'_>,
    epoch: LayoutEpoch,
    runs: &[ValidatedGlyphRun],
) -> Result<(), BreakError> {
    if shaped.epoch() != epoch
        || shaped.source() != shape_source(receipt.source())
        || shaped.site_owner() != receipt.site_owner()
        || shaped.style_owner() != receipt.style_owner()
    {
        return Err(BreakError::ShapedSliceMismatch);
    }
    let run = runs
        .get(shaped.paragraph_run_index().get() as usize)
        .ok_or(BreakError::UnknownParagraphRun)?;
    if run.run_id() != shaped.run_id()
        || run.epoch() != shaped.epoch()
        || run.bidi_level() != shaped.bidi_level()
        || run.site_owner() != shaped.site_owner()
        || run.style_owner() != shaped.style_owner()
    {
        return Err(BreakError::ShapedSliceMismatch);
    }
    let cluster = run
        .clusters()
        .iter()
        .find(|cluster| {
            cluster.source_span == shaped.source()
                && cluster.glyph_start == shaped.glyph_start()
                && cluster.glyph_end == shaped.glyph_end()
        })
        .ok_or(BreakError::ShapedSliceMismatch)?;
    let start = usize::try_from(cluster.glyph_start).map_err(|_| BreakError::ArithmeticOverflow)?;
    let end = usize::try_from(cluster.glyph_end).map_err(|_| BreakError::ArithmeticOverflow)?;
    let width = run.glyphs()[start..end]
        .iter()
        .try_fold(Length::ZERO, |total, glyph| {
            total.checked_add(glyph.advance_x)
        })
        .ok_or(BreakError::ArithmeticOverflow)?;
    if NonNegativeLength::new(width) != Some(shaped.derived_width()) {
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

/// UAX #9 classes which participate in the line-level L1 reset. The caller
/// must derive these classes from the same resolved Unicode data used during
/// canonical itemization; this type deliberately does not infer them from a
/// glyph ID or visual order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineBidiClass {
    Other,
    Whitespace,
    SegmentSeparator,
    ParagraphSeparator,
    IsolateFormatting,
    BoundaryNeutral,
}

/// Result of UAX #9 L1 followed by L2. `visual_to_logical[i]` names the
/// logical cluster occupying visual position `i`; the levels remain in
/// logical order so source mapping is never rewritten into visual order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedLineBidiOrder {
    paragraph_level: BidiLevel,
    logical_levels_after_l1: Vec<BidiLevel>,
    visual_to_logical: Vec<u32>,
}

/// Logical cluster levels after UAX #9 L1 and before final shaping,
/// justification, or visual reordering. Keeping this as a distinct sealed
/// value makes it possible for the final-line pipeline to enforce the stage
/// order rather than calling a combined L1/L2 helper too early.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineLevelsAfterL1 {
    paragraph_level: BidiLevel,
    logical_levels: Vec<BidiLevel>,
}
impl LineLevelsAfterL1 {
    pub const fn paragraph_level(&self) -> BidiLevel {
        self.paragraph_level
    }
    pub fn logical_levels(&self) -> &[BidiLevel] {
        &self.logical_levels
    }
}
impl ResolvedLineBidiOrder {
    pub const fn paragraph_level(&self) -> BidiLevel {
        self.paragraph_level
    }
    pub fn logical_levels_after_l1(&self) -> &[BidiLevel] {
        &self.logical_levels_after_l1
    }
    pub fn visual_to_logical(&self) -> &[u32] {
        &self.visual_to_logical
    }
}

/// Applies UAX #9 rule L1 to one already-broken logical line and then derives
/// its L2 visual permutation. This function performs no shaping or
/// justification; those stages must run between this L1 result and use of the
/// returned L2 permutation by the final-line pipeline.
pub fn resolve_line_bidi_order(
    paragraph_level: BidiLevel,
    resolved_levels: &[BidiLevel],
    classes: &[LineBidiClass],
) -> Result<ResolvedLineBidiOrder, BreakError> {
    let levels = reset_line_bidi_levels(paragraph_level, resolved_levels, classes)?;
    reorder_line_l2(&levels)
}

pub fn reset_line_bidi_levels(
    paragraph_level: BidiLevel,
    resolved_levels: &[BidiLevel],
    classes: &[LineBidiClass],
) -> Result<LineLevelsAfterL1, BreakError> {
    if paragraph_level.get() > 1 {
        return Err(BreakError::InvalidParagraphBidiLevel);
    }
    if resolved_levels.len() != classes.len()
        || resolved_levels
            .iter()
            .any(|level| level.get() < paragraph_level.get())
    {
        return Err(BreakError::InvalidLineBidiInput);
    }
    let mut levels = Vec::new();
    levels
        .try_reserve_exact(resolved_levels.len())
        .map_err(|_| BreakError::AllocationFailure)?;
    levels.extend_from_slice(resolved_levels);

    // L1.1 and L1.2: reset each segment/paragraph separator and the
    // immediately preceding whitespace sequence to the paragraph level.
    for (index, class) in classes.iter().enumerate() {
        if matches!(
            class,
            LineBidiClass::SegmentSeparator | LineBidiClass::ParagraphSeparator
        ) {
            levels[index] = paragraph_level;
            let mut preceding = index;
            while preceding > 0 && classes[preceding - 1] == LineBidiClass::Whitespace {
                preceding -= 1;
                levels[preceding] = paragraph_level;
            }
        }
    }
    // L1.3: reset the terminal sequence of whitespace, isolate formatting
    // controls, and boundary neutrals.
    for index in (0..classes.len()).rev() {
        if matches!(
            classes[index],
            LineBidiClass::Whitespace
                | LineBidiClass::IsolateFormatting
                | LineBidiClass::BoundaryNeutral
        ) {
            levels[index] = paragraph_level;
        } else {
            break;
        }
    }

    Ok(LineLevelsAfterL1 {
        paragraph_level,
        logical_levels: levels,
    })
}

pub fn reorder_line_l2(after_l1: &LineLevelsAfterL1) -> Result<ResolvedLineBidiOrder, BreakError> {
    let mut visual_to_logical = Vec::new();
    visual_to_logical
        .try_reserve_exact(after_l1.logical_levels.len())
        .map_err(|_| BreakError::AllocationFailure)?;
    for index in 0..after_l1.logical_levels.len() {
        visual_to_logical.push(u32::try_from(index).map_err(|_| BreakError::ArithmeticOverflow)?);
    }
    apply_l2_permutation(&after_l1.logical_levels, &mut visual_to_logical);
    Ok(ResolvedLineBidiOrder {
        paragraph_level: after_l1.paragraph_level,
        logical_levels_after_l1: after_l1.logical_levels.clone(),
        visual_to_logical,
    })
}

fn apply_l2_permutation(levels: &[BidiLevel], visual_to_logical: &mut [u32]) {
    let Some(lowest_odd) = levels
        .iter()
        .map(|level| level.get())
        .filter(|level| level % 2 == 1)
        .min()
    else {
        return;
    };
    let highest = levels
        .iter()
        .map(|level| level.get())
        .max()
        .unwrap_or(lowest_odd);
    for threshold in (lowest_odd..=highest).rev() {
        let mut start = 0usize;
        while start < visual_to_logical.len() {
            while start < visual_to_logical.len()
                && levels[visual_to_logical[start] as usize].get() < threshold
            {
                start += 1;
            }
            let mut end = start;
            while end < visual_to_logical.len()
                && levels[visual_to_logical[end] as usize].get() >= threshold
            {
                end += 1;
            }
            visual_to_logical[start..end].reverse();
            start = end;
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
    paragraph_level: BidiLevel,
    item_count: u32,
    runs: Vec<ValidatedGlyphRun>,
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
    pub const fn paragraph_level(&self) -> BidiLevel {
        self.paragraph_level
    }
    pub const fn item_count(&self) -> u32 {
        self.item_count
    }
    pub fn runs(&self) -> &[ValidatedGlyphRun] {
        &self.runs
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
        paragraph_level: BidiLevel,
        runs: Vec<ValidatedGlyphRun>,
        items: Vec<ParagraphItem>,
        result: ParagraphBreak,
    },
}
impl ValidatedParagraphItemRegistry {
    pub fn from_breaks(
        package: &ValidatedParsedPackage,
        epoch: LayoutEpoch,
        breaks: &[ValidatedParagraphBreak],
    ) -> Result<Self, BreakError> {
        Self::from_breaks_internal(package, epoch, breaks, false, false)
    }

    /// Combines validated nonempty paragraph breaks with canonical empty
    /// anchor-only paragraphs. This is the production mixed-document path;
    /// a missing text-producing paragraph remains an error.
    pub fn from_breaks_allowing_empty(
        package: &ValidatedParsedPackage,
        epoch: LayoutEpoch,
        breaks: &[ValidatedParagraphBreak],
    ) -> Result<Self, BreakError> {
        Self::from_breaks_internal(package, epoch, breaks, true, false)
    }

    /// Complete paragraph registry for the footnote profile. Definition
    /// paragraphs remain available to the dedicated FootnoteFlow registry,
    /// while the body FlowTree explicitly excludes them.
    pub fn from_breaks_with_footnotes_allowing_empty(
        package: &ValidatedParsedPackage,
        epoch: LayoutEpoch,
        breaks: &[ValidatedParagraphBreak],
    ) -> Result<Self, BreakError> {
        Self::from_breaks_internal(package, epoch, breaks, true, true)
    }

    fn from_breaks_internal(
        package: &ValidatedParsedPackage,
        epoch: LayoutEpoch,
        breaks: &[ValidatedParagraphBreak],
        allow_empty: bool,
        include_footnotes: bool,
    ) -> Result<Self, BreakError> {
        validate_package_epoch(package, epoch)?;
        if !include_footnotes && !package.package().document.footnotes.is_empty() {
            return Err(BreakError::UnsupportedFlowDomain);
        }
        let document = &package.package().document;
        let expected = if include_footnotes {
            all_paragraph_blocks(document)
                .into_iter()
                .map(block_node_id)
                .collect()
        } else {
            main_paragraph_nodes(&document.blocks)
        };
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
                        paragraph_level: receipt.paragraph_level,
                        runs: receipt.runs.clone(),
                        items: receipt.items.clone(),
                        result: receipt.result.clone(),
                    },
                )
                .is_some()
            {
                return Err(BreakError::DuplicateParagraphBreak);
            }
        }
        if allow_empty {
            let empty_candidates = if include_footnotes {
                all_paragraph_blocks(document)
            } else {
                main_paragraph_blocks(&document.blocks)
            };
            for block in empty_candidates {
                let node = block_node_id(block);
                if !item_sequences.contains_key(&node) && paragraph_has_empty_content(block) {
                    item_sequences.insert(node, ParagraphItemSequence::EmptyContent);
                }
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
    pub fn runs(&self, paragraph_node: NodeId) -> Option<&[ValidatedGlyphRun]> {
        match self.item_sequences.get(&paragraph_node)? {
            ParagraphItemSequence::EmptyContent => Some(&[]),
            ParagraphItemSequence::Items { runs, .. } => Some(runs),
        }
    }
    pub fn paragraph_break(&self, paragraph_node: NodeId) -> Option<&ParagraphBreak> {
        match self.item_sequences.get(&paragraph_node)? {
            ParagraphItemSequence::EmptyContent => None,
            ParagraphItemSequence::Items { result, .. } => Some(result),
        }
    }
    pub fn paragraph_level(&self, paragraph_node: NodeId) -> Option<BidiLevel> {
        match self.item_sequences.get(&paragraph_node)? {
            ParagraphItemSequence::EmptyContent => Some(BidiLevel::LTR),
            ParagraphItemSequence::Items {
                paragraph_level, ..
            } => Some(*paragraph_level),
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

    /// Finds the first canonical item carrying one generated site. This is
    /// used to bind inline footnote discovery to exact selected line ranges;
    /// the returned index is not a caller-supplied layout coordinate.
    pub fn generated_site_first_item_index(
        &self,
        paragraph_node: NodeId,
        site_owner: NodeId,
        generation_kind: GenerationKind,
    ) -> Option<u32> {
        self.items(paragraph_node)?
            .iter()
            .enumerate()
            .find(|(_, item)| {
                item_provenances(item).into_iter().any(|provenance| {
                    matches!(
                        provenance,
                        ItemProvenance::Generated(value)
                            if value.buffer_key().owner() == site_owner
                                && value.buffer_key().generation_kind() == generation_kind
                    )
                })
            })
            .and_then(|(index, _)| u32::try_from(index).ok())
    }

    /// Proves that a generated marker's first selected line also contains a
    /// distinct positive-width shaped cluster. Footnote definitions use this
    /// to reject an authored leading hard break that would strand the marker
    /// on a marker-only line, contrary to the marker/first-source-line keep.
    pub fn generated_site_first_line_has_other_shaped_content(
        &self,
        paragraph_node: NodeId,
        site_owner: NodeId,
        generation_kind: GenerationKind,
    ) -> bool {
        let Some(ParagraphItemSequence::Items { items, result, .. }) =
            self.item_sequences.get(&paragraph_node)
        else {
            return false;
        };
        let Some(first_line) = result.lines.first() else {
            return false;
        };
        let first_line_end = first_line.item_index as usize;
        if first_line_end == 0 || first_line_end > items.len() {
            return false;
        }
        let marker_is_on_first_line = items[..first_line_end].iter().any(|item| {
            item_provenances(item).into_iter().any(|provenance| {
                matches!(
                    provenance,
                    ItemProvenance::Generated(value)
                        if value.buffer_key().owner() == site_owner
                            && value.buffer_key().generation_kind() == generation_kind
                )
            })
        });
        marker_is_on_first_line
            && items[..first_line_end].iter().any(|item| {
                let shaped = match item {
                    ParagraphItem::Box { shaped, .. } | ParagraphItem::Glue { shaped, .. } => {
                        Some(*shaped)
                    }
                    _ => None,
                };
                shaped.is_some_and(|shaped| {
                    shaped.derived_width().get() > Length::ZERO
                        && !matches!(
                            shaped.source(),
                            ShapeSourceSpan::Generated(value)
                                if value.buffer_key().owner() == site_owner
                                    && value.buffer_key().generation_kind() == generation_kind
                        )
                })
            })
    }
}

pub const STAGING_MACHINE_LINK_CLUSTER_ALGORITHM: &str = "typaxis.machine-link-cluster-ranges/1";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StagingMachineLinkClusterKey {
    paragraph_node: NodeId,
    logical_ordinal: u32,
    item_index: u32,
    paragraph_run_index: u32,
    glyph_start: u32,
    glyph_end: u32,
    site_owner: NodeId,
}

impl StagingMachineLinkClusterKey {
    pub const fn paragraph_node(self) -> NodeId {
        self.paragraph_node
    }
    pub const fn logical_ordinal(self) -> u32 {
        self.logical_ordinal
    }
    pub const fn item_index(self) -> u32 {
        self.item_index
    }
    pub const fn paragraph_run_index(self) -> u32 {
        self.paragraph_run_index
    }
    pub const fn glyph_start(self) -> u32 {
        self.glyph_start
    }
    pub const fn glyph_end(self) -> u32 {
        self.glyph_end
    }
    pub const fn site_owner(self) -> NodeId {
        self.site_owner
    }

    pub fn matches_shaped(self, paragraph_node: NodeId, shaped: ShapedSlice) -> bool {
        self.paragraph_node == paragraph_node
            && self.paragraph_run_index == shaped.paragraph_run_index().get()
            && self.glyph_start == shaped.glyph_start()
            && self.glyph_end == shaped.glyph_end()
            && self.site_owner == shaped.site_owner()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingMachineLinkClusterRange {
    link_node: NodeId,
    paragraph_node: NodeId,
    target: ValidatedStagingLinkTarget,
    logical_start: u32,
    logical_end: u32,
    clusters: Vec<StagingMachineLinkClusterKey>,
}

impl ValidatedStagingMachineLinkClusterRange {
    pub const fn link_node(&self) -> NodeId {
        self.link_node
    }
    pub const fn paragraph_node(&self) -> NodeId {
        self.paragraph_node
    }
    pub const fn target(&self) -> &ValidatedStagingLinkTarget {
        &self.target
    }
    pub const fn logical_start(&self) -> u32 {
        self.logical_start
    }
    pub const fn logical_end(&self) -> u32 {
        self.logical_end
    }
    pub fn clusters(&self) -> &[StagingMachineLinkClusterKey] {
        &self.clusters
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingMachineLinkClusterError {
    EmptyLinkSet,
    PackageMismatch,
    EpochMismatch,
    MissingParagraph,
    MissingPaintedCluster(NodeId),
    ZeroWidthPaintedCluster(NodeId),
    NonContiguousClusterRange(NodeId),
    OverlappingCluster(NodeId),
    ArithmeticOverflow,
    AllocationFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedStagingMachineLinkClusters {
    package_sha256: [u8; 32],
    usage_sha256: [u8; 32],
    epoch: LayoutEpoch,
    ranges: Vec<ValidatedStagingMachineLinkClusterRange>,
    canonical_jcs: String,
}

impl ValidatedStagingMachineLinkClusters {
    /// Binds each syntax-owned link child set to the exact logical shaping
    /// clusters retained by the selected paragraph-item registry.
    pub fn from_registry(
        package: &ValidatedStagingStylePackage,
        usage: &ValidatedStagingLinkUsageReceipt,
        registry: &ValidatedParagraphItemRegistry,
    ) -> Result<Self, StagingMachineLinkClusterError> {
        if !usage.verifies(package) {
            return Err(StagingMachineLinkClusterError::PackageMismatch);
        }
        if registry.epoch().document() != package.package().epoch_identity().document()
            || registry.epoch().style() != package.package().epoch_identity().style()
        {
            return Err(StagingMachineLinkClusterError::EpochMismatch);
        }
        if usage.links().is_empty() {
            return Err(StagingMachineLinkClusterError::EmptyLinkSet);
        }

        let mut used_clusters = std::collections::BTreeSet::new();
        let mut ranges = Vec::new();
        ranges
            .try_reserve_exact(usage.links().len())
            .map_err(|_| StagingMachineLinkClusterError::AllocationFailure)?;
        for link in usage.links() {
            let items = registry
                .items(link.paragraph_owner())
                .ok_or(StagingMachineLinkClusterError::MissingParagraph)?;
            let expected_sites: std::collections::BTreeSet<_> =
                link.painted_site_owners().iter().copied().collect();
            let mut observed_sites = std::collections::BTreeSet::new();
            let mut clusters = Vec::new();
            let mut logical_ordinal = 0u32;
            for (item_index, item) in items.iter().enumerate() {
                let shaped = match item {
                    ParagraphItem::Box { shaped, .. } | ParagraphItem::Glue { shaped, .. } => {
                        Some(*shaped)
                    }
                    ParagraphItem::Penalty { .. }
                    | ParagraphItem::Discretionary { .. }
                    | ParagraphItem::InlineObject { .. } => None,
                };
                let Some(shaped) = shaped else {
                    continue;
                };
                let current_ordinal = logical_ordinal;
                logical_ordinal = logical_ordinal
                    .checked_add(1)
                    .ok_or(StagingMachineLinkClusterError::ArithmeticOverflow)?;
                if !expected_sites.contains(&shaped.site_owner()) {
                    continue;
                }
                if shaped.derived_width().get() == Length::ZERO {
                    return Err(StagingMachineLinkClusterError::ZeroWidthPaintedCluster(
                        link.owner(),
                    ));
                }
                observed_sites.insert(shaped.site_owner());
                let key = StagingMachineLinkClusterKey {
                    paragraph_node: link.paragraph_owner(),
                    logical_ordinal: current_ordinal,
                    item_index: u32::try_from(item_index)
                        .map_err(|_| StagingMachineLinkClusterError::ArithmeticOverflow)?,
                    paragraph_run_index: shaped.paragraph_run_index().get(),
                    glyph_start: shaped.glyph_start(),
                    glyph_end: shaped.glyph_end(),
                    site_owner: shaped.site_owner(),
                };
                if !used_clusters.insert(key) {
                    return Err(StagingMachineLinkClusterError::OverlappingCluster(
                        link.owner(),
                    ));
                }
                clusters
                    .try_reserve(1)
                    .map_err(|_| StagingMachineLinkClusterError::AllocationFailure)?;
                clusters.push(key);
            }
            if observed_sites != expected_sites {
                return Err(StagingMachineLinkClusterError::MissingPaintedCluster(
                    link.owner(),
                ));
            }
            let (logical_start, logical_end) =
                staging_machine_link_logical_bounds(link.owner(), &clusters)?;
            ranges.push(ValidatedStagingMachineLinkClusterRange {
                link_node: link.owner(),
                paragraph_node: link.paragraph_owner(),
                target: link.target().clone(),
                logical_start,
                logical_end,
                clusters,
            });
        }
        ranges.sort_by_key(ValidatedStagingMachineLinkClusterRange::link_node);
        let mut value = Self {
            package_sha256: package.package_fingerprint().into_bytes(),
            usage_sha256: usage.usage_sha256(),
            epoch: registry.epoch(),
            ranges,
            canonical_jcs: String::new(),
        };
        value.canonical_jcs = encode_staging_machine_link_clusters(&value);
        Ok(value)
    }

    pub const fn package_sha256(&self) -> [u8; 32] {
        self.package_sha256
    }
    pub const fn usage_sha256(&self) -> [u8; 32] {
        self.usage_sha256
    }
    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub fn ranges(&self) -> &[ValidatedStagingMachineLinkClusterRange] {
        &self.ranges
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }

    pub fn verifies(
        &self,
        package: &ValidatedStagingStylePackage,
        registry: &ValidatedParagraphItemRegistry,
    ) -> bool {
        self.package_sha256 == package.package_fingerprint().into_bytes()
            && self.epoch == registry.epoch()
            && self.epoch.document() == package.package().epoch_identity().document()
            && self.epoch.style() == package.package().epoch_identity().style()
    }

    pub fn range_for_shaped(
        &self,
        paragraph_node: NodeId,
        shaped: ShapedSlice,
    ) -> Option<(
        &ValidatedStagingMachineLinkClusterRange,
        StagingMachineLinkClusterKey,
    )> {
        self.ranges.iter().find_map(|range| {
            range
                .clusters
                .iter()
                .copied()
                .find(|key| key.matches_shaped(paragraph_node, shaped))
                .map(|key| (range, key))
        })
    }
}

fn staging_machine_link_logical_bounds(
    link_node: NodeId,
    clusters: &[StagingMachineLinkClusterKey],
) -> Result<(u32, u32), StagingMachineLinkClusterError> {
    let Some(first) = clusters.first() else {
        return Err(StagingMachineLinkClusterError::MissingPaintedCluster(
            link_node,
        ));
    };
    if clusters
        .windows(2)
        .any(|pair| match pair[0].logical_ordinal.checked_add(1) {
            Some(next) => next != pair[1].logical_ordinal,
            None => true,
        })
    {
        return Err(StagingMachineLinkClusterError::NonContiguousClusterRange(
            link_node,
        ));
    }
    let logical_end = clusters
        .last()
        .and_then(|cluster| cluster.logical_ordinal.checked_add(1))
        .ok_or(StagingMachineLinkClusterError::ArithmeticOverflow)?;
    Ok((first.logical_ordinal, logical_end))
}

fn encode_staging_machine_link_clusters(value: &ValidatedStagingMachineLinkClusters) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_MACHINE_LINK_CLUSTER_ALGORITHM);
    output.push_str(",\"layout_epoch\":{");
    output.push_str("\"admitted_resources_sha256\":");
    push_linebreak_json_hex(&mut output, value.epoch.admitted_resources().bytes());
    output.push_str(",\"document_sha256\":");
    push_linebreak_json_hex(&mut output, value.epoch.document().bytes());
    output.push_str(",\"resolved_input_sha256\":");
    push_linebreak_json_hex(&mut output, value.epoch.references().bytes());
    output.push_str(",\"style_page_master_sha256\":");
    push_linebreak_json_hex(&mut output, value.epoch.style().bytes());
    output.push_str("},\"links\":[");
    for (index, range) in value.ranges.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"clusters\":[");
        for (cluster_index, cluster) in range.clusters.iter().enumerate() {
            if cluster_index > 0 {
                output.push(',');
            }
            output.push_str("{\"glyph_end\":");
            output.push_str(&cluster.glyph_end.to_string());
            output.push_str(",\"glyph_start\":");
            output.push_str(&cluster.glyph_start.to_string());
            output.push_str(",\"item_index\":");
            output.push_str(&cluster.item_index.to_string());
            output.push_str(",\"logical_ordinal\":");
            output.push_str(&cluster.logical_ordinal.to_string());
            output.push_str(",\"paragraph_run_index\":");
            output.push_str(&cluster.paragraph_run_index.to_string());
            output.push_str(",\"site_owner_node_id\":");
            output.push_str(&cluster.site_owner.get().to_string());
            output.push('}');
        }
        output.push_str("],\"link_node_id\":");
        output.push_str(&range.link_node.get().to_string());
        output.push_str(",\"logical_cluster_count\":");
        output.push_str(&range.clusters.len().to_string());
        output.push_str(",\"logical_cluster_end\":");
        output.push_str(&range.logical_end.to_string());
        output.push_str(",\"logical_cluster_start\":");
        output.push_str(&range.logical_start.to_string());
        output.push_str(",\"paragraph_node_id\":");
        output.push_str(&range.paragraph_node.get().to_string());
        output.push_str(",\"target\":");
        encode_staging_machine_link_target(&mut output, &range.target);
        output.push('}');
    }
    output.push_str("],\"package_sha256\":");
    push_linebreak_json_hex(&mut output, value.package_sha256);
    output.push_str(",\"usage_sha256\":");
    push_linebreak_json_hex(&mut output, value.usage_sha256);
    output.push('}');
    output
}

fn encode_staging_machine_link_target(output: &mut String, target: &ValidatedStagingLinkTarget) {
    match target {
        ValidatedStagingLinkTarget::Internal {
            anchor_id,
            anchor_owner,
        } => {
            output.push_str("{\"anchor_id\":");
            push_jcs_string(output, anchor_id.as_str());
            output.push_str(",\"anchor_owner_node_id\":");
            output.push_str(&anchor_owner.get().to_string());
            output.push_str(",\"kind\":\"internal\"}");
        }
        ValidatedStagingLinkTarget::External(uri) => {
            output.push_str("{\"kind\":\"external\",\"uri\":");
            push_jcs_string(output, uri.as_str());
            output.push('}');
        }
    }
}

fn push_linebreak_json_hex(output: &mut String, bytes: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BreakError {
    NoFeasibleBreak,
    ArithmeticOverflow,
    AllocationFailure,
    IterationLimit,
    ClusterBoundaryViolation,
    InvalidOpportunity,
    EmptyLineShapes,
    LineShapeLimit,
    ParagraphTextLimit,
    DuplicateGeneratedProvenance,
    InvalidGeneratedProvenance,
    BudgetAlreadyIssued,
    BreakAlreadyStarted,
    ReshapePassInFlight,
    ReshapeTerminal,
    UnknownShapedCluster,
    UnknownParagraphRun,
    InvalidGlyphAdvance,
    ShapedWidthMismatch,
    InvalidEmptyDiscretionaryBranch,
    InvalidParagraphOwner,
    ParagraphEpochMismatch,
    EmptyParagraphItems,
    MissingTerminalBreak,
    ParagraphTextSiteMismatch,
    MalformedRunCoverage,
    MissingCanonicalItemization,
    InvalidParagraphBidiLevel,
    ParagraphBidiLevelMismatch,
    InvalidLineBidiInput,
    ShapingDataTablesMismatch,
    ShaperIdentityMismatch,
    SpaceShrinkExceedsNatural,
    MandatoryBreakInsideCluster,
    UnsupportedExplicitLineBreak,
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

/// Domain-separated fingerprint of one complete canonical line-layout state
/// at the reshape feedback boundary. The caller supplies canonical bytes, not
/// a precomputed digest, so every pass uses the same algorithm domain.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LineLayoutStateFingerprint([u8; 32]);
impl LineLayoutStateFingerprint {
    pub const ALGORITHM_ID: &'static str = "typaxis.line-layout-state.sha256/1";

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, BreakError> {
        let capacity = Self::ALGORITHM_ID
            .len()
            .checked_add(1)
            .and_then(|length| length.checked_add(bytes.len()))
            .ok_or(BreakError::ArithmeticOverflow)?;
        let mut domain = Vec::new();
        domain
            .try_reserve_exact(capacity)
            .map_err(|_| BreakError::AllocationFailure)?;
        domain.extend_from_slice(Self::ALGORITHM_ID.as_bytes());
        domain.push(0);
        domain.extend_from_slice(bytes);
        Ok(Self(sha256(&domain)))
    }
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineReshapePassRecord {
    pass_index: u16,
    input: LineLayoutStateFingerprint,
    output: LineLayoutStateFingerprint,
}
impl LineReshapePassRecord {
    pub const fn pass_index(self) -> u16 {
        self.pass_index
    }
    pub const fn input(self) -> LineLayoutStateFingerprint {
        self.input
    }
    pub const fn output(self) -> LineLayoutStateFingerprint {
        self.output
    }
    pub fn is_stable(self) -> bool {
        self.input.0 == self.output.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineReshapeObservation {
    Stable,
    RebreakRequired,
}

/// Owns the post-break `final reshape -> compare -> rebreak` loop. Creating
/// this owner does not count the initial shaping/break. Each `begin_pass`
/// consumes exactly one allowed reshape pass before work starts; completing
/// the final allowed pass with a changed fingerprint returns
/// `IterationLimit` only after recording that output.
#[derive(Debug, Eq, PartialEq)]
pub struct LineReshapeFeedback {
    current: LineLayoutStateFingerprint,
    next_pass_index: u16,
    records: Vec<LineReshapePassRecord>,
    pass_in_flight: bool,
    terminal: bool,
}
impl LineReshapeFeedback {
    pub const fn new(initial: LineLayoutStateFingerprint) -> Self {
        Self {
            current: initial,
            next_pass_index: 1,
            records: Vec::new(),
            pass_in_flight: false,
            terminal: false,
        }
    }
    pub const fn current(&self) -> LineLayoutStateFingerprint {
        self.current
    }
    pub fn records(&self) -> &[LineReshapePassRecord] {
        &self.records
    }
    pub fn begin_pass<'a>(
        &'a mut self,
        budget: &'a mut LineLayoutBudget,
    ) -> Result<LineReshapePassPermit<'a>, BreakError> {
        if self.terminal {
            return Err(BreakError::ReshapeTerminal);
        }
        if self.pass_in_flight {
            return Err(BreakError::ReshapePassInFlight);
        }
        budget.consume_reshape()?;
        let pass_index = self.next_pass_index;
        if budget.remaining_reshape_passes() > 0 {
            self.next_pass_index = self
                .next_pass_index
                .checked_add(1)
                .ok_or(BreakError::ArithmeticOverflow)?;
        }
        self.pass_in_flight = true;
        Ok(LineReshapePassPermit {
            feedback: self,
            budget,
            pass_index,
        })
    }
}

/// One-shot permission for exactly one final reshape pass. Dropping it leaves
/// the owner fail-closed with an in-flight pass; only `complete` can publish a
/// comparison record and permit another pass.
#[derive(Debug)]
pub struct LineReshapePassPermit<'a> {
    feedback: &'a mut LineReshapeFeedback,
    budget: &'a mut LineLayoutBudget,
    pass_index: u16,
}
impl LineReshapePassPermit<'_> {
    pub const fn pass_index(&self) -> u16 {
        self.pass_index
    }
    pub const fn input(&self) -> LineLayoutStateFingerprint {
        self.feedback.current
    }
    pub fn complete(
        self,
        output: LineLayoutStateFingerprint,
    ) -> Result<LineReshapeObservation, BreakError> {
        let input = self.feedback.current;
        self.feedback.records.push(LineReshapePassRecord {
            pass_index: self.pass_index,
            input,
            output,
        });
        self.feedback.pass_in_flight = false;
        self.feedback.current = output;
        if input == output {
            self.feedback.terminal = true;
            return Ok(LineReshapeObservation::Stable);
        }
        if self.budget.remaining_reshape_passes() == 0 {
            self.feedback.terminal = true;
            return Err(BreakError::IterationLimit);
        }
        Ok(LineReshapeObservation::RebreakRequired)
    }
}

pub trait ParagraphBreaker {
    fn break_paragraph(
        &self,
        input: &ParagraphInput<'_>,
        budget: &mut LineLayoutBudget,
    ) -> Result<ParagraphBreak, BreakError>;
}

/// Deterministic first-fit paragraph breaker. It chooses the furthest legal
/// opportunity that fits the current line, never crosses a mandatory break,
/// and emits an overfull line only when the first reachable opportunity
/// itself cannot fit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GreedyParagraphBreaker;

impl ParagraphBreaker for GreedyParagraphBreaker {
    fn break_paragraph(
        &self,
        input: &ParagraphInput<'_>,
        _budget: &mut LineLayoutBudget,
    ) -> Result<ParagraphBreak, BreakError> {
        let candidates = break_candidates(input.items())?;
        let mut lines = Vec::new();
        let mut start = 0usize;
        let mut line_index = 0usize;
        let mut cumulative_demerits = 0i64;
        while start < input.items().len() {
            let target = line_inline_size(input, line_index)?;
            let mut chosen = None;
            for candidate in candidates.iter().filter(|candidate| candidate.end > start) {
                if crosses_mandatory(input.items(), start, candidate.end) {
                    continue;
                }
                let metrics = measure_line(input.items(), start, *candidate)?;
                let fits = metrics.natural <= i128::from(target.raw());
                if fits || chosen.is_none() {
                    chosen = Some((*candidate, metrics));
                }
                if candidate.mandatory || (!fits && chosen.is_some()) {
                    break;
                }
            }
            let (candidate, metrics) = chosen.ok_or(BreakError::NoFeasibleBreak)?;
            cumulative_demerits = cumulative_demerits
                .checked_add(line_demerits(target, metrics, None)?)
                .ok_or(BreakError::ArithmeticOverflow)?;
            lines.push(LineBreak {
                item_index: u32::try_from(candidate.end)
                    .map_err(|_| BreakError::ArithmeticOverflow)?,
                offset: break_offset(&input.items()[candidate.end - 1]),
                demerits: cumulative_demerits,
            });
            start = candidate.end;
            line_index = line_index
                .checked_add(1)
                .ok_or(BreakError::ArithmeticOverflow)?;
        }
        Ok(ParagraphBreak { lines })
    }
}

/// Bounded dynamic-programming breaker using the same canonical opportunity
/// set as the greedy implementation. It minimizes integer badness, penalty,
/// fitness-transition, consecutive-flag, and overfull costs. Equal-cost paths
/// are resolved by the lexicographically earliest item-boundary sequence.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OptimalParagraphBreaker;

#[derive(Clone, Debug, Eq, PartialEq)]
struct OptimalState {
    end: usize,
    line_count: usize,
    fitness: u8,
    flagged: bool,
    demerits: i64,
    path: Vec<usize>,
    line_costs: Vec<i64>,
}

impl ParagraphBreaker for OptimalParagraphBreaker {
    fn break_paragraph(
        &self,
        input: &ParagraphInput<'_>,
        _budget: &mut LineLayoutBudget,
    ) -> Result<ParagraphBreak, BreakError> {
        let candidates = break_candidates(input.items())?;
        let mut frontiers: Vec<Vec<OptimalState>> = vec![Vec::new(); candidates.len()];
        for (candidate_index, candidate) in candidates.iter().copied().enumerate() {
            let mut best: std::collections::BTreeMap<(usize, u8, bool), OptimalState> =
                std::collections::BTreeMap::new();
            let roots = core::iter::once(OptimalState {
                end: 0,
                line_count: 0,
                fitness: 1,
                flagged: false,
                demerits: 0,
                path: Vec::new(),
                line_costs: Vec::new(),
            });
            let predecessors = roots.chain(
                frontiers[..candidate_index]
                    .iter()
                    .flat_map(|states| states.iter().cloned()),
            );
            for predecessor in predecessors {
                if predecessor.end >= candidate.end
                    || crosses_mandatory(input.items(), predecessor.end, candidate.end)
                {
                    continue;
                }
                let target = match line_inline_size(input, predecessor.line_count) {
                    Ok(target) => target,
                    Err(BreakError::NoFeasibleBreak) => continue,
                    Err(error) => return Err(error),
                };
                let metrics = measure_line(input.items(), predecessor.end, candidate)?;
                let fitness = line_fitness(target, metrics);
                let edge = line_demerits(
                    target,
                    metrics,
                    (predecessor.line_count > 0)
                        .then_some((predecessor.fitness, predecessor.flagged)),
                )?;
                let demerits = predecessor
                    .demerits
                    .checked_add(edge)
                    .ok_or(BreakError::ArithmeticOverflow)?;
                let mut path = predecessor.path;
                path.push(candidate.end);
                let mut line_costs = predecessor.line_costs;
                line_costs.push(demerits);
                let state = OptimalState {
                    end: candidate.end,
                    line_count: predecessor
                        .line_count
                        .checked_add(1)
                        .ok_or(BreakError::ArithmeticOverflow)?,
                    fitness,
                    flagged: candidate.flagged,
                    demerits,
                    path,
                    line_costs,
                };
                let key = (state.line_count, state.fitness, state.flagged);
                match best.get(&key) {
                    Some(current)
                        if (current.demerits, &current.path) <= (state.demerits, &state.path) => {}
                    _ => {
                        best.insert(key, state);
                    }
                }
            }
            frontiers[candidate_index] = best.into_values().collect();
        }
        let terminal_index = candidates
            .iter()
            .rposition(|candidate| candidate.end == input.items().len())
            .ok_or(BreakError::NoFeasibleBreak)?;
        let selected = frontiers[terminal_index]
            .iter()
            .min_by(|left, right| (left.demerits, &left.path).cmp(&(right.demerits, &right.path)))
            .ok_or(BreakError::NoFeasibleBreak)?;
        let lines = selected
            .path
            .iter()
            .zip(&selected.line_costs)
            .map(|(end, demerits)| {
                Ok(LineBreak {
                    item_index: u32::try_from(*end).map_err(|_| BreakError::ArithmeticOverflow)?,
                    offset: break_offset(&input.items()[*end - 1]),
                    demerits: *demerits,
                })
            })
            .collect::<Result<_, BreakError>>()?;
        Ok(ParagraphBreak { lines })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BreakCandidate {
    end: usize,
    penalty: i32,
    flagged: bool,
    mandatory: bool,
}

fn break_candidates(items: &[ParagraphItem]) -> Result<Vec<BreakCandidate>, BreakError> {
    if !matches!(
        items.last(),
        Some(ParagraphItem::Penalty {
            kind: BreakKind::Mandatory,
            ..
        })
    ) {
        return Err(BreakError::MissingTerminalBreak);
    }
    let mut candidates = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let attributes = match item {
            ParagraphItem::Glue { .. }
                if !glue_has_explicit_boundary(items, index)
                    && !glue_precedes_mandatory(items, index) =>
            {
                Some((0, false, false))
            }
            ParagraphItem::Glue { .. } => None,
            ParagraphItem::Penalty {
                cost,
                kind: BreakKind::Allowed,
                flagged,
                ..
            } => Some((*cost, *flagged, false)),
            ParagraphItem::Penalty {
                cost,
                kind: BreakKind::Mandatory,
                flagged,
                ..
            } => Some((*cost, *flagged, true)),
            ParagraphItem::Discretionary {
                penalty, flagged, ..
            } => Some((*penalty, *flagged, false)),
            ParagraphItem::Box { .. }
            | ParagraphItem::Penalty {
                kind: BreakKind::Prohibited,
                ..
            }
            | ParagraphItem::InlineObject { .. } => None,
        };
        if let Some((penalty, flagged, mandatory)) = attributes {
            candidates.push(BreakCandidate {
                end: index.checked_add(1).ok_or(BreakError::ArithmeticOverflow)?,
                penalty,
                flagged,
                mandatory,
            });
        }
    }
    Ok(candidates)
}

/// Canonical factories emit an explicit Penalty after every Glue so Unicode
/// rules such as `SP × WJ` can override the usual break-after-space behavior.
/// Pluggable item streams without that explicit boundary retain the
/// conventional implicit Glue opportunity.
fn glue_has_explicit_boundary(items: &[ParagraphItem], glue_index: usize) -> bool {
    matches!(
        items.get(glue_index + 1),
        Some(ParagraphItem::Penalty { .. })
    )
}

/// A discardable Glue immediately before a mandatory boundary is not a
/// distinct break. Otherwise selecting both it and the mandatory Penalty can
/// manufacture an empty line, most visibly for trailing U+0020.
fn glue_precedes_mandatory(items: &[ParagraphItem], glue_index: usize) -> bool {
    items[glue_index + 1..]
        .iter()
        .find(|item| {
            !matches!(
                item,
                ParagraphItem::Glue { .. }
                    | ParagraphItem::Penalty {
                        kind: BreakKind::Prohibited,
                        ..
                    }
            )
        })
        .is_some_and(|item| {
            matches!(
                item,
                ParagraphItem::Penalty {
                    kind: BreakKind::Mandatory,
                    ..
                }
            )
        })
}

fn crosses_mandatory(items: &[ParagraphItem], start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    items[start..end - 1].iter().any(|item| {
        matches!(
            item,
            ParagraphItem::Penalty {
                kind: BreakKind::Mandatory,
                ..
            }
        )
    })
}

fn line_inline_size(input: &ParagraphInput<'_>, line_index: usize) -> Result<Length, BreakError> {
    match input.line_shapes().get(line_index) {
        Some(shape) => Ok(shape.inline_size.get()),
        None if input.line_shape_exhaustion() == LineShapeExhaustion::RepeatLast => input
            .line_shapes()
            .last()
            .map(|shape| shape.inline_size.get())
            .ok_or(BreakError::EmptyLineShapes),
        None => Err(BreakError::NoFeasibleBreak),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LineMetrics {
    natural: i128,
    stretch: i128,
    shrink: i128,
    penalty: i32,
    flagged: bool,
}

fn measure_line(
    items: &[ParagraphItem],
    start: usize,
    candidate: BreakCandidate,
) -> Result<LineMetrics, BreakError> {
    let mut natural = 0i128;
    let mut stretch = 0i128;
    let mut shrink = 0i128;
    let trailing_glue_start = if matches!(
        items.get(candidate.end - 1),
        Some(ParagraphItem::Penalty {
            kind: BreakKind::Mandatory,
            ..
        })
    ) {
        let mut index = candidate.end - 1;
        while index > start && matches!(items[index - 1], ParagraphItem::Glue { .. }) {
            index -= 1;
        }
        Some(index)
    } else {
        None
    };
    if start > 0 {
        if let ParagraphItem::Discretionary { post_break, .. } = &items[start - 1] {
            natural = natural
                .checked_add(i128::from(post_break.width.raw()))
                .ok_or(BreakError::ArithmeticOverflow)?;
        }
    }
    for (index, item) in items[start..candidate.end].iter().enumerate() {
        let absolute = start
            .checked_add(index)
            .ok_or(BreakError::ArithmeticOverflow)?;
        let at_break = absolute + 1 == candidate.end;
        match item {
            ParagraphItem::Box { width, .. } | ParagraphItem::InlineObject { width, .. } => {
                natural = natural
                    .checked_add(i128::from(width.get().raw()))
                    .ok_or(BreakError::ArithmeticOverflow)?;
            }
            ParagraphItem::Glue {
                natural: width,
                stretch: item_stretch,
                shrink: item_shrink,
                ..
            } if trailing_glue_start.map_or(true, |trailing| absolute < trailing) => {
                natural = natural
                    .checked_add(i128::from(width.get().raw()))
                    .ok_or(BreakError::ArithmeticOverflow)?;
                stretch = stretch
                    .checked_add(i128::from(item_stretch.get().raw()))
                    .ok_or(BreakError::ArithmeticOverflow)?;
                shrink = shrink
                    .checked_add(i128::from(item_shrink.get().raw()))
                    .ok_or(BreakError::ArithmeticOverflow)?;
            }
            ParagraphItem::Glue { .. } => {}
            ParagraphItem::Penalty { width, .. } if at_break => {
                natural = natural
                    .checked_add(i128::from(width.raw()))
                    .ok_or(BreakError::ArithmeticOverflow)?;
            }
            ParagraphItem::Penalty { .. } => {}
            ParagraphItem::Discretionary {
                no_break,
                pre_break,
                ..
            } => {
                let width = if at_break {
                    pre_break.width
                } else {
                    no_break.width
                };
                natural = natural
                    .checked_add(i128::from(width.raw()))
                    .ok_or(BreakError::ArithmeticOverflow)?;
            }
        }
    }
    Ok(LineMetrics {
        natural,
        stretch,
        shrink,
        penalty: candidate.penalty,
        flagged: candidate.flagged,
    })
}

fn line_fitness(target: Length, metrics: LineMetrics) -> u8 {
    let delta = i128::from(target.raw()) - metrics.natural;
    if delta < 0 {
        0
    } else if delta == 0 {
        1
    } else if metrics.stretch > 0 && delta <= metrics.stretch {
        2
    } else {
        3
    }
}

fn line_demerits(
    target: Length,
    metrics: LineMetrics,
    previous: Option<(u8, bool)>,
) -> Result<i64, BreakError> {
    let target = i128::from(target.raw());
    let delta = target
        .checked_sub(metrics.natural)
        .ok_or(BreakError::ArithmeticOverflow)?;
    let capacity = if delta >= 0 {
        metrics.stretch
    } else {
        metrics.shrink
    };
    let magnitude = delta.checked_abs().ok_or(BreakError::ArithmeticOverflow)?;
    let ratio_milli = if magnitude == 0 {
        0
    } else if capacity <= 0 {
        10_000
    } else {
        magnitude
            .checked_mul(1_000)
            .ok_or(BreakError::ArithmeticOverflow)?
            .checked_div(capacity)
            .ok_or(BreakError::ArithmeticOverflow)?
            .min(10_000)
    };
    let badness = ratio_milli
        .checked_mul(ratio_milli)
        .and_then(|value| value.checked_mul(ratio_milli))
        .and_then(|value| value.checked_mul(100))
        .and_then(|value| value.checked_div(1_000_000_000))
        .ok_or(BreakError::ArithmeticOverflow)?
        .min(10_000);
    let base = badness
        .checked_add(1)
        .and_then(|value| value.checked_mul(value))
        .ok_or(BreakError::ArithmeticOverflow)?;
    let penalty = i128::from(metrics.penalty);
    let penalty_cost = penalty
        .checked_mul(penalty)
        .ok_or(BreakError::ArithmeticOverflow)?;
    let mut total = if penalty >= 0 {
        base.checked_add(penalty_cost)
    } else {
        base.checked_sub(penalty_cost)
    }
    .ok_or(BreakError::ArithmeticOverflow)?
    .max(0);
    if delta < 0 && magnitude > metrics.shrink {
        total = total
            .checked_add(1_000_000_000)
            .ok_or(BreakError::ArithmeticOverflow)?;
    }
    if let Some((previous_fitness, previous_flagged)) = previous {
        let fitness = line_fitness(
            Length::from_raw(target as i64).ok_or(BreakError::ArithmeticOverflow)?,
            metrics,
        );
        if previous_fitness.abs_diff(fitness) > 1 {
            total = total
                .checked_add(3_000)
                .ok_or(BreakError::ArithmeticOverflow)?;
        }
        if previous_flagged && metrics.flagged {
            total = total
                .checked_add(100)
                .ok_or(BreakError::ArithmeticOverflow)?;
        }
    }
    i64::try_from(total).map_err(|_| BreakError::ArithmeticOverflow)
}

fn break_offset(item: &ParagraphItem) -> Option<ItemTextOffset> {
    let provenance = match item {
        ParagraphItem::Box { provenance, .. }
        | ParagraphItem::Glue { provenance, .. }
        | ParagraphItem::Penalty { provenance, .. }
        | ParagraphItem::InlineObject { provenance, .. } => provenance,
        ParagraphItem::Discretionary { pre_break, .. } => &pre_break.provenance,
    };
    Some(match provenance {
        ItemProvenance::Text(span) => ItemTextOffset::Parsed(TextOffset {
            text_id: span.text_id(),
            byte: span.end_byte(),
        }),
        ItemProvenance::Generated(provenance) => ItemTextOffset::Generated(*provenance),
    })
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
        paragraph_level: input.paragraph_level,
        item_count,
        runs: input.runs.to_vec(),
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
        let Some(offset) = line.offset else {
            return Err(BreakError::ClusterBoundaryViolation);
        };
        if !item_provenances(break_item)
            .into_iter()
            .any(|provenance| provenance_contains_offset(provenance, offset))
        {
            return Err(BreakError::ClusterBoundaryViolation);
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

fn all_paragraph_blocks(document: &typaxis_document::Document) -> Vec<&Block> {
    document
        .footnotes
        .iter()
        .flat_map(|definition| main_paragraph_blocks(&definition.blocks))
        .chain(main_paragraph_blocks(&document.blocks))
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
        PortablePath, ResourceLimits, SourceId, SourceSpan, TextBufferId, Utf8ByteOffset,
        ValidatedResourceLimits,
    };
    use typaxis_resource_admission::AdmittedResourceResolver;
    use typaxis_syntax::{
        PackageValidationPolicy, ParseOutcome, Parser, ReferenceParser, SourceFile,
    };
    use typaxis_text::{GeneratedBufferDraft, GeneratedTextStore};

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
            package
                .document_nodes()
                .generated_sites()
                .map(|site| {
                    GeneratedBufferDraft::new(package.document_nodes(), site.key(), String::new())
                        .unwrap()
                })
                .collect(),
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
    fn reshape_feedback_counts_only_post_break_passes_and_records_stability() {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_line_reshape_passes: 2,
            ..ResourceLimits::default()
        })
        .unwrap();
        let mut context = LineLayoutContext::from_limits(&limits);
        let mut budget = context.take_budget().unwrap();
        let initial = LineLayoutStateFingerprint::from_canonical_bytes(b"initial").unwrap();
        let changed = LineLayoutStateFingerprint::from_canonical_bytes(b"changed").unwrap();
        let mut feedback = LineReshapeFeedback::new(initial);

        let first = feedback.begin_pass(&mut budget).unwrap();
        assert_eq!(first.pass_index(), 1);
        assert_eq!(first.input(), initial);
        assert_eq!(
            first.complete(changed),
            Ok(LineReshapeObservation::RebreakRequired)
        );
        let second = feedback.begin_pass(&mut budget).unwrap();
        assert_eq!(second.pass_index(), 2);
        assert_eq!(second.input(), changed);
        assert_eq!(second.complete(changed), Ok(LineReshapeObservation::Stable));
        assert_eq!(budget.remaining_reshape_passes(), 0);
        assert_eq!(feedback.records().len(), 2);
        assert!(!feedback.records()[0].is_stable());
        assert!(feedback.records()[1].is_stable());
    }

    #[test]
    fn final_allowed_reshape_failure_is_reported_after_recording_that_pass() {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_line_reshape_passes: 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        let mut context = LineLayoutContext::from_limits(&limits);
        let mut budget = context.take_budget().unwrap();
        let initial = LineLayoutStateFingerprint::from_canonical_bytes(b"initial").unwrap();
        let changed = LineLayoutStateFingerprint::from_canonical_bytes(b"still-unstable").unwrap();
        let mut feedback = LineReshapeFeedback::new(initial);

        let final_allowed = feedback.begin_pass(&mut budget).unwrap();
        assert_eq!(
            final_allowed.complete(changed),
            Err(BreakError::IterationLimit)
        );
        assert_eq!(feedback.current(), changed);
        assert_eq!(feedback.records().len(), 1);
        assert_eq!(feedback.records()[0].output(), changed);
        assert!(matches!(
            feedback.begin_pass(&mut budget),
            Err(BreakError::ReshapeTerminal)
        ));
    }

    #[test]
    fn abandoned_reshape_pass_fails_closed() {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_line_reshape_passes: 2,
            ..ResourceLimits::default()
        })
        .unwrap();
        let mut context = LineLayoutContext::from_limits(&limits);
        let mut budget = context.take_budget().unwrap();
        let initial = LineLayoutStateFingerprint::from_canonical_bytes(b"initial").unwrap();
        let mut feedback = LineReshapeFeedback::new(initial);
        {
            let _abandoned = feedback.begin_pass(&mut budget).unwrap();
        }
        assert!(matches!(
            feedback.begin_pass(&mut budget),
            Err(BreakError::ReshapePassInFlight)
        ));
        assert!(feedback.records().is_empty());
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
                cost: MANDATORY_BREAK_COST,
                kind: BreakKind::Mandatory,
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
            BidiLevel::LTR,
            &[],
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
            Err(BreakError::ClusterBoundaryViolation)
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
                    offset: Some(ItemTextOffset::Parsed(TextOffset {
                        text_id: TextBufferId::new(0),
                        byte: Utf8ByteOffset::new(2),
                    })),
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
            Err(BreakError::ClusterBoundaryViolation)
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
            BidiLevel::LTR,
            &[],
            &prohibited_items,
            &shapes,
            LineShapeExhaustion::RepeatLast,
        )
        .unwrap();
        let prohibited_break = FixedBreaker(ParagraphBreak {
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
                    offset: Some(ItemTextOffset::Parsed(TextOffset {
                        text_id: TextBufferId::new(0),
                        byte: Utf8ByteOffset::new(2),
                    })),
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
                    offset: Some(ItemTextOffset::Parsed(TextOffset {
                        text_id: TextBufferId::new(0),
                        byte: Utf8ByteOffset::new(2),
                    })),
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
            paragraph_run_index: ParagraphRunIndex::new(0),
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
        let wrong_box = [
            ParagraphItem::Box {
                width: NonNegativeLength::new(Length::from_raw(1).unwrap()).unwrap(),
                shaped,
                provenance: ItemProvenance::Text(parsed_span(0, 1)),
            },
            terminal_penalty(1),
        ];
        assert_eq!(
            ParagraphInput::new(
                NodeId::new(1),
                binding,
                epoch,
                BidiLevel::LTR,
                &[],
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
        let discretionary = [
            ParagraphItem::Discretionary {
                no_break: Box::new(nonempty_unshaped),
                pre_break: Box::new(nonempty_unshaped),
                post_break: Box::new(nonempty_unshaped),
                penalty: 0,
                flagged: false,
            },
            terminal_penalty(1),
        ];
        assert_eq!(
            ParagraphInput::new(
                NodeId::new(1),
                binding,
                epoch,
                BidiLevel::LTR,
                &[],
                &discretionary,
                &shapes,
                LineShapeExhaustion::RepeatLast,
            ),
            Err(BreakError::InvalidEmptyDiscretionaryBranch)
        );
    }

    fn shaped_slice(width: i64, start: u32, end: u32, epoch: LayoutEpoch) -> ShapedSlice {
        ShapedSlice {
            paragraph_run_index: ParagraphRunIndex::new(start),
            run_id: GlyphRunId::new(start),
            glyph_start: 0,
            glyph_end: 1,
            bidi_level: BidiLevel::new(0).unwrap(),
            derived_width: NonNegativeLength::new(Length::from_raw(width).unwrap()).unwrap(),
            source: ShapeSourceSpan::Parsed(parsed_span(start, end)),
            epoch,
            site_owner: NodeId::new(2),
            style_owner: NodeId::new(1),
        }
    }

    #[test]
    fn link_clusters_match_the_exact_selected_shaping_slice() {
        let (_, _, epoch) = package_epoch("text:a");
        let shaped = shaped_slice(10, 0, 1, epoch);
        let key = StagingMachineLinkClusterKey {
            paragraph_node: NodeId::new(1),
            logical_ordinal: 0,
            item_index: 0,
            paragraph_run_index: shaped.paragraph_run_index().get(),
            glyph_start: shaped.glyph_start(),
            glyph_end: shaped.glyph_end(),
            site_owner: shaped.site_owner(),
        };
        assert!(key.matches_shaped(NodeId::new(1), shaped));
        assert!(!key.matches_shaped(NodeId::new(9), shaped));

        let mut wrong_run = key;
        wrong_run.paragraph_run_index += 1;
        assert!(!wrong_run.matches_shaped(NodeId::new(1), shaped));
        let mut wrong_site = key;
        wrong_site.site_owner = NodeId::new(9);
        assert!(!wrong_site.matches_shaped(NodeId::new(1), shaped));
    }

    #[test]
    fn link_clusters_require_nonempty_contiguous_logical_ranges() {
        let key = |logical_ordinal| StagingMachineLinkClusterKey {
            paragraph_node: NodeId::new(1),
            logical_ordinal,
            item_index: logical_ordinal,
            paragraph_run_index: logical_ordinal,
            glyph_start: 0,
            glyph_end: 1,
            site_owner: NodeId::new(2),
        };
        let owner = NodeId::new(3);
        assert_eq!(
            staging_machine_link_logical_bounds(owner, &[key(4), key(5)]),
            Ok((4, 6))
        );
        assert_eq!(
            staging_machine_link_logical_bounds(owner, &[]),
            Err(StagingMachineLinkClusterError::MissingPaintedCluster(owner))
        );
        assert_eq!(
            staging_machine_link_logical_bounds(owner, &[key(4), key(6)]),
            Err(StagingMachineLinkClusterError::NonContiguousClusterRange(
                owner
            ))
        );
        assert_eq!(
            staging_machine_link_logical_bounds(owner, &[key(u32::MAX)]),
            Err(StagingMachineLinkClusterError::ArithmeticOverflow)
        );
    }

    fn glue(width: i64, start: u32, end: u32, epoch: LayoutEpoch) -> ParagraphItem {
        ParagraphItem::Glue {
            natural: NonNegativeLength::new(Length::from_raw(width).unwrap()).unwrap(),
            stretch: NonNegativeLength::ZERO,
            shrink: NonNegativeLength::ZERO,
            priority: 0,
            shaped: shaped_slice(width, start, end, epoch),
            provenance: ItemProvenance::Text(parsed_span(start, end)),
        }
    }

    fn shaped_box(width: i64, start: u32, end: u32, epoch: LayoutEpoch) -> ParagraphItem {
        ParagraphItem::Box {
            width: NonNegativeLength::new(Length::from_raw(width).unwrap()).unwrap(),
            shaped: shaped_slice(width, start, end, epoch),
            provenance: ItemProvenance::Text(parsed_span(start, end)),
        }
    }

    fn prohibited_penalty(byte: u32) -> ParagraphItem {
        ParagraphItem::Penalty {
            width: Length::ZERO,
            cost: 0,
            kind: BreakKind::Prohibited,
            flagged: false,
            provenance: ItemProvenance::Text(parsed_span(byte, byte)),
        }
    }

    fn terminal_penalty(byte: u32) -> ParagraphItem {
        ParagraphItem::Penalty {
            width: Length::ZERO,
            cost: MANDATORY_BREAK_COST,
            kind: BreakKind::Mandatory,
            flagged: false,
            provenance: ItemProvenance::Text(parsed_span(byte, byte)),
        }
    }

    #[test]
    fn production_breakers_are_deterministic_and_use_distinct_policies() {
        let (package, generated, epoch) = package_epoch("text:a b c");
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let items = [
            shaped_box(6, 0, 1, epoch),
            prohibited_penalty(1),
            glue(0, 1, 2, epoch),
            shaped_box(4, 2, 3, epoch),
            prohibited_penalty(3),
            glue(0, 3, 4, epoch),
            shaped_box(6, 4, 5, epoch),
            terminal_penalty(5),
        ];
        let shapes = [LineShape {
            inline_size: PositiveLength::new(Length::from_raw(10).unwrap()).unwrap(),
        }];
        let input = ParagraphInput {
            paragraph_node: NodeId::new(1),
            epoch,
            reference_fingerprint: binding.generated_text().reference_fingerprint(),
            paragraph_level: BidiLevel::LTR,
            runs: &[],
            items: &items,
            line_shapes: &shapes,
            line_shape_exhaustion: LineShapeExhaustion::RepeatLast,
        };

        let run = |breaker: &dyn ParagraphBreaker| {
            let mut context = LineLayoutContext::from_limits(&limits);
            let mut budget = context.take_budget().unwrap();
            breaker.break_paragraph(&input, &mut budget).unwrap()
        };
        let greedy = run(&GreedyParagraphBreaker);
        assert_eq!(
            greedy
                .lines
                .iter()
                .map(|line| line.item_index)
                .collect::<Vec<_>>(),
            [6, 8]
        );
        let optimal = run(&OptimalParagraphBreaker);
        assert_eq!(
            optimal
                .lines
                .iter()
                .map(|line| line.item_index)
                .collect::<Vec<_>>(),
            [3, 8]
        );
        assert_eq!(optimal, run(&OptimalParagraphBreaker));
    }

    #[test]
    fn explicit_penalty_owns_canonical_glue_break_legality() {
        let (_, _, epoch) = package_epoch("text:a ");
        let prohibited = [
            glue(1, 0, 1, epoch),
            prohibited_penalty(1),
            terminal_penalty(1),
        ];
        assert!(glue_has_explicit_boundary(&prohibited, 0));
        assert_eq!(
            break_candidates(&prohibited).unwrap(),
            [BreakCandidate {
                end: 3,
                penalty: MANDATORY_BREAK_COST,
                flagged: false,
                mandatory: true,
            }]
        );

        let allowed = [
            glue(1, 0, 1, epoch),
            ParagraphItem::Penalty {
                width: Length::ZERO,
                cost: 0,
                kind: BreakKind::Allowed,
                flagged: false,
                provenance: ItemProvenance::Text(parsed_span(1, 1)),
            },
            terminal_penalty(1),
        ];
        assert_eq!(break_candidates(&allowed).unwrap()[0].end, 2);
    }

    #[test]
    fn production_breakers_never_cross_mandatory_opportunities() {
        let (package, generated, epoch) = package_epoch("text:abc");
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let items = [
            glue(3, 0, 1, epoch),
            ParagraphItem::Penalty {
                width: Length::ZERO,
                cost: -10_000,
                kind: BreakKind::Mandatory,
                flagged: false,
                provenance: ItemProvenance::Text(parsed_span(1, 2)),
            },
            glue(3, 2, 3, epoch),
            terminal_penalty(3),
        ];
        let shapes = [LineShape {
            inline_size: PositiveLength::new(Length::from_raw(10).unwrap()).unwrap(),
        }];
        let input = ParagraphInput {
            paragraph_node: NodeId::new(1),
            epoch,
            reference_fingerprint: binding.generated_text().reference_fingerprint(),
            paragraph_level: BidiLevel::LTR,
            runs: &[],
            items: &items,
            line_shapes: &shapes,
            line_shape_exhaustion: LineShapeExhaustion::RepeatLast,
        };
        for result in [
            GreedyParagraphBreaker.break_paragraph(
                &input,
                &mut LineLayoutContext::from_limits(&limits)
                    .take_budget()
                    .unwrap(),
            ),
            OptimalParagraphBreaker.break_paragraph(
                &input,
                &mut LineLayoutContext::from_limits(&limits)
                    .take_budget()
                    .unwrap(),
            ),
        ] {
            let result = result.unwrap();
            assert!(result.lines.iter().any(|line| line.item_index == 2));
            assert_eq!(result.lines.last().unwrap().item_index, 4);
        }
    }

    #[test]
    fn trailing_space_glue_cannot_create_an_empty_terminal_line() {
        let (package, generated, epoch) = package_epoch("text:  ");
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let items = [
            glue(3, 0, 1, epoch),
            glue(3, 1, 2, epoch),
            terminal_penalty(2),
        ];
        let shapes = [LineShape {
            inline_size: PositiveLength::new(Length::from_raw(1).unwrap()).unwrap(),
        }];
        let input = ParagraphInput {
            paragraph_node: NodeId::new(1),
            epoch,
            reference_fingerprint: binding.generated_text().reference_fingerprint(),
            paragraph_level: BidiLevel::LTR,
            runs: &[],
            items: &items,
            line_shapes: &shapes,
            line_shape_exhaustion: LineShapeExhaustion::RepeatLast,
        };

        assert!(glue_precedes_mandatory(&items, 0));
        assert!(glue_precedes_mandatory(&items, 1));
        for result in [
            GreedyParagraphBreaker.break_paragraph(
                &input,
                &mut LineLayoutContext::from_limits(&limits)
                    .take_budget()
                    .unwrap(),
            ),
            OptimalParagraphBreaker.break_paragraph(
                &input,
                &mut LineLayoutContext::from_limits(&limits)
                    .take_budget()
                    .unwrap(),
            ),
        ] {
            let result = result.unwrap();
            assert_eq!(result.lines.len(), 1);
            assert_eq!(result.lines[0].item_index, 3);
        }
    }

    #[test]
    fn multibyte_and_rtl_clusters_retain_shaped_source_and_level() {
        let (_, _, epoch) = package_epoch("text:א");
        let rtl = ShapedSlice {
            paragraph_run_index: ParagraphRunIndex::new(0),
            run_id: GlyphRunId::new(0),
            glyph_start: 0,
            glyph_end: 1,
            bidi_level: BidiLevel::new(1).unwrap(),
            derived_width: NonNegativeLength::new(Length::from_raw(7).unwrap()).unwrap(),
            source: ShapeSourceSpan::Parsed(parsed_span(0, 2)),
            epoch,
            site_owner: NodeId::new(2),
            style_owner: NodeId::new(1),
        };
        let item = ParagraphItem::Box {
            width: rtl.derived_width(),
            shaped: rtl,
            provenance: ItemProvenance::Text(parsed_span(0, 2)),
        };
        let ParagraphItem::Box { shaped, .. } = item else {
            panic!("box fixture must remain a box");
        };
        assert!(shaped.bidi_level().is_rtl());
        assert_eq!(shape_source_bounds(shaped.source()), (0, 2));
    }

    #[test]
    fn logical_cluster_coverage_accepts_multibyte_boundaries_and_rejects_gaps() {
        let run = ShapeSourceSpan::Parsed(parsed_span(0, 4));
        assert_eq!(
            validate_cluster_source_coverage(
                run,
                [
                    ShapeSourceSpan::Parsed(parsed_span(0, 2)),
                    ShapeSourceSpan::Parsed(parsed_span(2, 4)),
                ],
            ),
            Ok(())
        );
        assert_eq!(
            validate_cluster_source_coverage(
                run,
                [
                    ShapeSourceSpan::Parsed(parsed_span(0, 2)),
                    ShapeSourceSpan::Parsed(parsed_span(3, 4)),
                ],
            ),
            Err(BreakError::MalformedRunCoverage)
        );
        assert_eq!(
            validate_cluster_source_coverage(
                run,
                [
                    ShapeSourceSpan::Parsed(parsed_span(0, 3)),
                    ShapeSourceSpan::Parsed(parsed_span(2, 4)),
                ],
            ),
            Err(BreakError::MalformedRunCoverage)
        );
    }

    #[test]
    fn paragraph_run_indices_disambiguate_per_site_run_ids() {
        let (_, _, epoch) = package_epoch("text:ab");
        let mut first = shaped_slice(1, 0, 1, epoch);
        first.paragraph_run_index = ParagraphRunIndex::new(0);
        first.run_id = GlyphRunId::new(0);
        let mut second = shaped_slice(1, 1, 2, epoch);
        second.paragraph_run_index = ParagraphRunIndex::new(1);
        second.run_id = GlyphRunId::new(0);

        assert_eq!(first.run_id(), second.run_id());
        assert_ne!(first.paragraph_run_index(), second.paragraph_run_index());
        assert_ne!(first, second);
    }

    #[test]
    fn factory_preflight_accepts_exact_limits_and_rejects_max_plus_one() {
        let limits = ValidatedResourceLimits::new(ResourceLimits {
            max_text_bytes: 2,
            max_text_buffer_bytes: 2,
            max_shaping_context_bytes: 2,
            max_pages: 1,
            ..ResourceLimits::default()
        })
        .unwrap();
        assert_eq!(preflight_factory_limits(2, 0, 2, 1, &limits), Ok(5));
        assert_eq!(
            preflight_factory_limits(2, 0, 3, 1, &limits),
            Err(BreakError::ParagraphTextLimit)
        );
        assert_eq!(
            preflight_factory_limits(2, 0, 2, 2, &limits),
            Err(BreakError::LineShapeLimit)
        );
    }

    #[test]
    fn explicit_breaks_are_allowed_and_mandatory_elements() {
        let source_span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let paragraph = Block::Paragraph {
            node_id: NodeId::new(1),
            span: source_span,
            classes: vec![],
            children: vec![
                Inline::SoftBreak {
                    node_id: NodeId::new(2),
                    span: source_span,
                },
                Inline::HardBreak {
                    node_id: NodeId::new(3),
                    span: source_span,
                },
            ],
        };
        let mut elements = Vec::new();
        collect_paragraph_elements(&paragraph, None, &mut elements).unwrap();
        assert_eq!(
            elements,
            vec![
                ExpectedParagraphElement::ExplicitBreak {
                    node_id: NodeId::new(2),
                    kind: BreakKind::Allowed,
                },
                ExpectedParagraphElement::ExplicitBreak {
                    node_id: NodeId::new(3),
                    kind: BreakKind::Mandatory,
                },
            ]
        );
    }

    #[test]
    fn definition_marker_prefix_prohibits_only_optional_breaks_before_source() {
        assert_eq!(
            protect_definition_marker_prefix_break(BreakKind::Allowed, true),
            BreakKind::Prohibited
        );
        assert_eq!(
            protect_definition_marker_prefix_break(BreakKind::Mandatory, true),
            BreakKind::Mandatory
        );
        assert_eq!(
            protect_definition_marker_prefix_break(BreakKind::Allowed, false),
            BreakKind::Allowed
        );
    }

    #[test]
    fn japanese_pair_table_applies_mode_specific_kinsoku_and_spacing() {
        let opening = japanese_pair_rule(Some('「'), Some('日'), JapaneseLineBreakMode::Loose);
        assert_eq!(opening.permission(), JapanesePairPermission::Prohibit);
        let normal_small =
            japanese_pair_rule(Some('日'), Some('ゃ'), JapaneseLineBreakMode::Normal);
        assert_eq!(normal_small.permission(), JapanesePairPermission::Prohibit);
        let loose_small = japanese_pair_rule(Some('日'), Some('ゃ'), JapaneseLineBreakMode::Loose);
        assert_eq!(loose_small.permission(), JapanesePairPermission::Preserve);
        let mixed = japanese_pair_rule(Some('日'), Some('A'), JapaneseLineBreakMode::Strict);
        assert_eq!(mixed.natural_gap_per_1024_em(), 128);
        assert_eq!(mixed.stretch_per_1024_em(), 64);
        assert_eq!(mixed.shrink_per_1024_em(), 64);
        assert_eq!(mixed.penalty(), 100);
        assert_eq!(mixed.priority(), 2);
    }

    #[test]
    fn paragraph_factory_accepts_break_only_paragraphs() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let shape = [LineShape {
            inline_size: PositiveLength::new(Length::from_raw(10).unwrap()).unwrap(),
        }];
        for (source, expected_kinds) in [
            ("soft_break", vec![BreakKind::Mandatory]),
            ("hard_break", vec![BreakKind::Mandatory]),
        ] {
            let (package, generated, epoch) = package_epoch(source);
            let binding = package.bind_generated_text(&generated, &limits).unwrap();
            let paragraph = BoundedReferenceParagraphFactory::new()
                .build(
                    binding,
                    NodeId::new(1),
                    epoch,
                    &[],
                    ReferenceSpaceGlue::new(NonNegativeLength::ZERO, NonNegativeLength::ZERO),
                    &shape,
                    LineShapeExhaustion::RepeatLast,
                    &limits,
                )
                .unwrap();
            let kinds: Vec<_> = paragraph
                .items()
                .iter()
                .map(|item| match item {
                    ParagraphItem::Penalty { kind, .. } => *kind,
                    _ => panic!("break-only paragraph emitted drawing content"),
                })
                .collect();
            assert_eq!(kinds, expected_kinds);
        }
    }

    #[test]
    fn paragraph_input_requires_an_explicit_terminal_penalty() {
        let (package, generated, epoch) = package_epoch("text:a");
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let binding = package.bind_generated_text(&generated, &limits).unwrap();
        let items = [ParagraphItem::Penalty {
            width: Length::ZERO,
            cost: 0,
            kind: BreakKind::Allowed,
            flagged: false,
            provenance: ItemProvenance::Text(parsed_span(0, 1)),
        }];
        let shapes = [LineShape {
            inline_size: PositiveLength::new(Length::from_raw(1).unwrap()).unwrap(),
        }];
        assert_eq!(
            ParagraphInput::new(
                NodeId::new(1),
                binding,
                epoch,
                BidiLevel::LTR,
                &[],
                &items,
                &shapes,
                LineShapeExhaustion::RepeatLast,
            ),
            Err(BreakError::MissingTerminalBreak)
        );
    }

    #[test]
    fn line_bidi_applies_l1_before_l2_and_preserves_logical_levels() {
        let level = |value| BidiLevel::new(value).unwrap();
        let order = resolve_line_bidi_order(
            BidiLevel::LTR,
            &[level(0), level(1), level(1), level(1), level(1)],
            &[
                LineBidiClass::Other,
                LineBidiClass::Other,
                LineBidiClass::Whitespace,
                LineBidiClass::IsolateFormatting,
                LineBidiClass::BoundaryNeutral,
            ],
        )
        .unwrap();
        assert_eq!(
            order.logical_levels_after_l1(),
            &[level(0), level(1), level(0), level(0), level(0)]
        );
        assert_eq!(order.visual_to_logical(), &[0, 1, 2, 3, 4]);

        let separator = resolve_line_bidi_order(
            BidiLevel::LTR,
            &[level(1), level(1), level(1), level(1)],
            &[
                LineBidiClass::Other,
                LineBidiClass::Whitespace,
                LineBidiClass::Whitespace,
                LineBidiClass::SegmentSeparator,
            ],
        )
        .unwrap();
        assert_eq!(
            separator.logical_levels_after_l1(),
            &[level(1), level(0), level(0), level(0)]
        );
    }

    #[test]
    fn line_bidi_l2_reorders_nested_levels_by_cluster_not_glyph() {
        let level = |value| BidiLevel::new(value).unwrap();
        let ltr = resolve_line_bidi_order(
            BidiLevel::LTR,
            &[level(0), level(1), level(1), level(0)],
            &[LineBidiClass::Other; 4],
        )
        .unwrap();
        assert_eq!(ltr.visual_to_logical(), &[0, 2, 1, 3]);

        let rtl = resolve_line_bidi_order(
            BidiLevel::RTL,
            &[level(1), level(2), level(2), level(1)],
            &[LineBidiClass::Other; 4],
        )
        .unwrap();
        assert_eq!(rtl.visual_to_logical(), &[3, 1, 2, 0]);
        assert_eq!(rtl.paragraph_level(), BidiLevel::RTL);
    }

    #[test]
    fn line_bidi_rejects_unbound_or_impossible_level_inputs() {
        let level = |value| BidiLevel::new(value).unwrap();
        assert_eq!(
            resolve_line_bidi_order(
                BidiLevel::LTR,
                &[level(0)],
                &[LineBidiClass::Other, LineBidiClass::Other],
            ),
            Err(BreakError::InvalidLineBidiInput)
        );
        assert_eq!(
            resolve_line_bidi_order(BidiLevel::RTL, &[level(0)], &[LineBidiClass::Other],),
            Err(BreakError::InvalidLineBidiInput)
        );
        assert_eq!(
            resolve_line_bidi_order(level(2), &[level(2)], &[LineBidiClass::Other]),
            Err(BreakError::InvalidParagraphBidiLevel)
        );
    }
}
