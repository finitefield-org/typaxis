use typaxis_core::MachinePdfProfileId;

/// Block kinds understood by the current document domain.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineBlockKind {
    DisplayMath,
    Figure,
    Heading,
    List,
    MathVectorBlock,
    PageBreak,
    Paragraph,
    SemanticContainer,
    Table,
    VectorFigure,
}

impl MachineBlockKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DisplayMath => "display_math",
            Self::Figure => "figure",
            Self::Heading => "heading",
            Self::List => "list",
            Self::MathVectorBlock => "math_vector_block",
            Self::PageBreak => "page_break",
            Self::Paragraph => "paragraph",
            Self::SemanticContainer => "semantic_container",
            Self::Table => "table",
            Self::VectorFigure => "vector_figure",
        }
    }
}

/// Inline kinds understood by the current document domain.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineInlineKind {
    Anchor,
    Emphasis,
    FootnoteReference,
    HardBreak,
    InlineMath,
    InlineVector,
    Link,
    MathVector,
    Reference,
    SoftBreak,
    Strong,
    Text,
}

impl MachineInlineKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Anchor => "anchor",
            Self::Emphasis => "emphasis",
            Self::FootnoteReference => "footnote_reference",
            Self::HardBreak => "hard_break",
            Self::InlineMath => "inline_math",
            Self::InlineVector => "inline_vector",
            Self::Link => "link",
            Self::MathVector => "math_vector",
            Self::Reference => "reference",
            Self::SoftBreak => "soft_break",
            Self::Strong => "strong",
            Self::Text => "text",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineReferenceFormat {
    Number,
    Page,
    Text,
}

impl MachineReferenceFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Page => "page",
            Self::Text => "text",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineStyleProperty {
    EndIndent,
    FontFamily,
    FontSize,
    KeepCaption,
    KeepWithNext,
    LineHeight,
    Page,
    SpaceAfter,
    SpaceBefore,
    StartIndent,
    TextAlign,
    Width,
}

impl MachineStyleProperty {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EndIndent => "end_indent",
            Self::FontFamily => "font_family",
            Self::FontSize => "font_size",
            Self::KeepCaption => "keep_caption",
            Self::KeepWithNext => "keep_with_next",
            Self::LineHeight => "line_height",
            Self::Page => "page",
            Self::SpaceAfter => "space_after",
            Self::SpaceBefore => "space_before",
            Self::StartIndent => "start_indent",
            Self::TextAlign => "text_align",
            Self::Width => "width",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "end_indent" => Some(Self::EndIndent),
            "font_family" => Some(Self::FontFamily),
            "font_size" => Some(Self::FontSize),
            "keep_caption" => Some(Self::KeepCaption),
            "keep_with_next" => Some(Self::KeepWithNext),
            "line_height" => Some(Self::LineHeight),
            "page" => Some(Self::Page),
            "space_after" => Some(Self::SpaceAfter),
            "space_before" => Some(Self::SpaceBefore),
            "start_indent" => Some(Self::StartIndent),
            "text_align" => Some(Self::TextAlign),
            "width" => Some(Self::Width),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachinePageValue {
    Auto,
    Named,
}

impl MachinePageValue {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Named => "named",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachinePageFrame {
    Footer,
    Footnote,
    Header,
}

impl MachinePageFrame {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Footer => "footer",
            Self::Footnote => "footnote",
            Self::Header => "header",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineFontFormat {
    OpenTypeCff,
    SfntTrueTypeGlyf,
    TtcOpenTypeCff,
    TtcTrueTypeGlyf,
    Woff2,
}

impl MachineFontFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpenTypeCff => "sfnt-opentype-cff",
            Self::SfntTrueTypeGlyf => "sfnt-truetype-glyf",
            Self::TtcOpenTypeCff => "ttc-opentype-cff",
            Self::TtcTrueTypeGlyf => "ttc-truetype-glyf",
            Self::Woff2 => "woff2",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineImageFormat {
    Jpeg,
    Png,
    Svg,
    Vector,
}

impl MachineImageFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Vector => "vector",
        }
    }
}

/// Coarse image-family vocabulary for the public production profile.
/// Exact Safe-SVG media/profile names have a separate nominal type and cannot
/// accidentally leak into `image_formats`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineCoarseImageFormat {
    Jpeg,
    Png,
    Svg,
}

impl MachineCoarseImageFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Svg => "svg",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineVectorBlockKind {
    MathVectorBlock,
    VectorFigure,
}

impl MachineVectorBlockKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MathVectorBlock => "math_vector_block",
            Self::VectorFigure => "vector_figure",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineVectorInlineKind {
    InlineVector,
    MathVector,
}

impl MachineVectorInlineKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InlineVector => "inline_vector",
            Self::MathVector => "math_vector",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineVectorKind {
    Figure,
    InlineVector,
    MathVector,
    MathVectorBlock,
    VectorFigure,
}

impl MachineVectorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Figure => "figure",
            Self::InlineVector => "inline_vector",
            Self::MathVector => "math_vector",
            Self::MathVectorBlock => "math_vector_block",
            Self::VectorFigure => "vector_figure",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineVectorFormat {
    Svg,
}

impl MachineVectorFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Svg => "svg",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineVectorProfile {
    SvgSafe1,
    SvgSafe2,
}

impl MachineVectorProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SvgSafe1 => "svg-safe-1",
            Self::SvgSafe2 => "svg-safe-2",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineVectorMetric {
    Advance,
    Ascent,
    Baseline,
    Descent,
    OriginX,
    Viewport,
}

impl MachineVectorMetric {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Advance => "advance",
            Self::Ascent => "ascent",
            Self::Baseline => "baseline",
            Self::Descent => "descent",
            Self::OriginX => "origin_x",
            Self::Viewport => "viewport",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineVectorFeature {
    ClipPath,
    CurrentColor,
    PaintOpacity,
    SharedFormXObject,
}

impl MachineVectorFeature {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClipPath => "clip-path",
            Self::CurrentColor => "current-color",
            Self::PaintOpacity => "paint-opacity",
            Self::SharedFormXObject => "shared-form-xobject",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachineVectorFeaturesByProfile {
    profile: MachineVectorProfile,
    features: &'static [MachineVectorFeature],
}

impl MachineVectorFeaturesByProfile {
    pub const fn profile(self) -> MachineVectorProfile {
        self.profile
    }

    pub const fn features(self) -> &'static [MachineVectorFeature] {
        self.features
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachineVectorMediaByKind {
    kind: MachineVectorKind,
    media: &'static [MachineVectorProfile],
}

impl MachineVectorMediaByKind {
    pub const fn kind(self) -> MachineVectorKind {
        self.kind
    }

    pub const fn media(self) -> &'static [MachineVectorProfile] {
        self.media
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrecomposedVectorCapabilityProjection {
    block_additions: &'static [MachineVectorBlockKind],
    coarse_image_formats: &'static [MachineCoarseImageFormat],
    inline_additions: &'static [MachineVectorInlineKind],
    style_block_additions: &'static [MachineVectorBlockKind],
    vector_features: &'static [MachineVectorFeature],
    vector_features_by_profile: &'static [MachineVectorFeaturesByProfile],
    vector_formats: &'static [MachineVectorFormat],
    vector_media_by_kind: &'static [MachineVectorMediaByKind],
    vector_metrics: &'static [MachineVectorMetric],
    vector_profiles: &'static [MachineVectorProfile],
}

impl PrecomposedVectorCapabilityProjection {
    pub const fn block_additions(self) -> &'static [MachineVectorBlockKind] {
        self.block_additions
    }

    pub const fn coarse_image_formats(self) -> &'static [MachineCoarseImageFormat] {
        self.coarse_image_formats
    }

    pub const fn inline_additions(self) -> &'static [MachineVectorInlineKind] {
        self.inline_additions
    }

    pub const fn style_block_additions(self) -> &'static [MachineVectorBlockKind] {
        self.style_block_additions
    }

    pub const fn vector_features(self) -> &'static [MachineVectorFeature] {
        self.vector_features
    }

    pub const fn vector_features_by_profile(self) -> &'static [MachineVectorFeaturesByProfile] {
        self.vector_features_by_profile
    }

    pub const fn vector_formats(self) -> &'static [MachineVectorFormat] {
        self.vector_formats
    }

    pub const fn vector_media_by_kind(self) -> &'static [MachineVectorMediaByKind] {
        self.vector_media_by_kind
    }

    pub const fn vector_metrics(self) -> &'static [MachineVectorMetric] {
        self.vector_metrics
    }

    pub const fn vector_profiles(self) -> &'static [MachineVectorProfile] {
        self.vector_profiles
    }
}

const PRIVATE_VECTOR_BLOCK_ADDITIONS: &[MachineVectorBlockKind] = &[
    MachineVectorBlockKind::MathVectorBlock,
    MachineVectorBlockKind::VectorFigure,
];
const PRIVATE_VECTOR_IMAGE_FORMATS: &[MachineCoarseImageFormat] = &[
    MachineCoarseImageFormat::Jpeg,
    MachineCoarseImageFormat::Png,
    MachineCoarseImageFormat::Svg,
];
const PRIVATE_VECTOR_INLINE_ADDITIONS: &[MachineVectorInlineKind] = &[
    MachineVectorInlineKind::InlineVector,
    MachineVectorInlineKind::MathVector,
];
const PRIVATE_VECTOR_FEATURES: &[MachineVectorFeature] = &[
    MachineVectorFeature::ClipPath,
    MachineVectorFeature::CurrentColor,
    MachineVectorFeature::PaintOpacity,
    MachineVectorFeature::SharedFormXObject,
];
const PRIVATE_VECTOR_SAFE_1_FEATURES: &[MachineVectorFeature] = &[
    MachineVectorFeature::ClipPath,
    MachineVectorFeature::SharedFormXObject,
];
const PRIVATE_VECTOR_FEATURES_BY_PROFILE: &[MachineVectorFeaturesByProfile] = &[
    MachineVectorFeaturesByProfile {
        profile: MachineVectorProfile::SvgSafe1,
        features: PRIVATE_VECTOR_SAFE_1_FEATURES,
    },
    MachineVectorFeaturesByProfile {
        profile: MachineVectorProfile::SvgSafe2,
        features: PRIVATE_VECTOR_FEATURES,
    },
];
const PRIVATE_VECTOR_FORMATS: &[MachineVectorFormat] = &[MachineVectorFormat::Svg];
const PRIVATE_VECTOR_SAFE_1_MEDIA: &[MachineVectorProfile] = &[MachineVectorProfile::SvgSafe1];
const PRIVATE_VECTOR_SAFE_2_MEDIA: &[MachineVectorProfile] = &[MachineVectorProfile::SvgSafe2];
const PRIVATE_VECTOR_SAFE_1_2_MEDIA: &[MachineVectorProfile] = &[
    MachineVectorProfile::SvgSafe1,
    MachineVectorProfile::SvgSafe2,
];
const PRIVATE_VECTOR_MEDIA_BY_KIND: &[MachineVectorMediaByKind] = &[
    MachineVectorMediaByKind {
        kind: MachineVectorKind::Figure,
        media: PRIVATE_VECTOR_SAFE_1_MEDIA,
    },
    MachineVectorMediaByKind {
        kind: MachineVectorKind::InlineVector,
        media: PRIVATE_VECTOR_SAFE_1_2_MEDIA,
    },
    MachineVectorMediaByKind {
        kind: MachineVectorKind::MathVector,
        media: PRIVATE_VECTOR_SAFE_2_MEDIA,
    },
    MachineVectorMediaByKind {
        kind: MachineVectorKind::MathVectorBlock,
        media: PRIVATE_VECTOR_SAFE_2_MEDIA,
    },
    MachineVectorMediaByKind {
        kind: MachineVectorKind::VectorFigure,
        media: PRIVATE_VECTOR_SAFE_1_2_MEDIA,
    },
];
const PRIVATE_VECTOR_METRICS: &[MachineVectorMetric] = &[
    MachineVectorMetric::Advance,
    MachineVectorMetric::Ascent,
    MachineVectorMetric::Baseline,
    MachineVectorMetric::Descent,
    MachineVectorMetric::OriginX,
    MachineVectorMetric::Viewport,
];
const PRIVATE_VECTOR_PROFILES: &[MachineVectorProfile] = &[
    MachineVectorProfile::SvgSafe1,
    MachineVectorProfile::SvgSafe2,
];

pub(crate) const PRECOMPOSED_VECTOR_CAPABILITY_PROJECTION: PrecomposedVectorCapabilityProjection =
    PrecomposedVectorCapabilityProjection {
        block_additions: PRIVATE_VECTOR_BLOCK_ADDITIONS,
        coarse_image_formats: PRIVATE_VECTOR_IMAGE_FORMATS,
        inline_additions: PRIVATE_VECTOR_INLINE_ADDITIONS,
        style_block_additions: PRIVATE_VECTOR_BLOCK_ADDITIONS,
        vector_features: PRIVATE_VECTOR_FEATURES,
        vector_features_by_profile: PRIVATE_VECTOR_FEATURES_BY_PROFILE,
        vector_formats: PRIVATE_VECTOR_FORMATS,
        vector_media_by_kind: PRIVATE_VECTOR_MEDIA_BY_KIND,
        vector_metrics: PRIVATE_VECTOR_METRICS,
        vector_profiles: PRIVATE_VECTOR_PROFILES,
    };

pub(crate) const PRODUCTION_BLOCKS: &[&str] = &[
    "display_math",
    "figure",
    "heading",
    "list",
    "math_vector_block",
    "page_break",
    "paragraph",
    "semantic_container",
    "table",
    "vector_figure",
];
pub(crate) const PRODUCTION_INLINES: &[&str] = &[
    "anchor",
    "emphasis",
    "footnote_reference",
    "hard_break",
    "inline_math",
    "inline_vector",
    "link",
    "math_vector",
    "reference",
    "soft_break",
    "strong",
    "text",
];
pub(crate) const PRODUCTION_STYLE_BLOCK_TYPES: &[&str] = PRODUCTION_BLOCKS;
pub(crate) const PRODUCTION_STYLE_SELECTORS: &[&str] = PRODUCTION_BLOCKS;

/// ADR-fixed order. These arrays are semantic tuples and are deliberately not
/// passed through the set sorter used by capability vocabularies.
pub(crate) const PRODUCTION_RESOURCE_COMPONENTS: &[&str] = &[
    "typaxis.resource-profile/png/1",
    "typaxis.resource-profile/safe-vector/2",
    "typaxis.resource-profile/jpeg-baseline/1",
    "typaxis.resource-profile/truetype-glyf/1",
    "typaxis.resource-profile/sfnt-cff1/1",
];
pub(crate) const PRODUCTION_IMAGE_MEDIA: &[&str] =
    &["png", "svg-safe-1", "svg-safe-2", "jpeg-baseline"];
pub(crate) const PRODUCTION_FONT_MEDIA: &[&str] =
    &["sfnt-truetype-glyf", "ttc-truetype-glyf", "sfnt-cff1"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProductionBookCapabilityDescriptor;

impl ProductionBookCapabilityDescriptor {
    pub(crate) const PROFILE_ID: &'static str = "typaxis.machine-pdf/production-book-1";
    pub(crate) const RESOURCE_SET_ID: &'static str = "typaxis.production-book-resource-set/2";
    pub(crate) const DEFAULT_PROFILE_ID: &'static str = "typaxis.machine-pdf/paragraph-1";
    pub(crate) const fn vector(self) -> PrecomposedVectorCapabilityProjection {
        PRECOMPOSED_VECTOR_CAPABILITY_PROJECTION
    }
    pub(crate) const fn blocks(self) -> &'static [&'static str] {
        PRODUCTION_BLOCKS
    }
    pub(crate) const fn inlines(self) -> &'static [&'static str] {
        PRODUCTION_INLINES
    }
    pub(crate) const fn style_block_types(self) -> &'static [&'static str] {
        PRODUCTION_STYLE_BLOCK_TYPES
    }
    pub(crate) const fn style_selectors(self) -> &'static [&'static str] {
        PRODUCTION_STYLE_SELECTORS
    }
    pub(crate) const fn resource_components(self) -> &'static [&'static str] {
        PRODUCTION_RESOURCE_COMPONENTS
    }
    pub(crate) const fn image_media(self) -> &'static [&'static str] {
        PRODUCTION_IMAGE_MEDIA
    }
    pub(crate) const fn font_media(self) -> &'static [&'static str] {
        PRODUCTION_FONT_MEDIA
    }
}

pub(crate) const PRODUCTION_BOOK_CAPABILITY_DESCRIPTOR: ProductionBookCapabilityDescriptor =
    ProductionBookCapabilityDescriptor;

const _: &str = ProductionBookCapabilityDescriptor::DEFAULT_PROFILE_ID;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachinePdfFeature {
    HeadingSemantics,
    LinkAnnotations,
    NamedDestinations,
    Outlines,
    PngXObjects,
    TaggedPdf,
    TextExtraction,
}

impl MachinePdfFeature {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HeadingSemantics => "heading-semantics",
            Self::LinkAnnotations => "link-annotations",
            Self::NamedDestinations => "named-destinations",
            Self::Outlines => "outlines",
            Self::PngXObjects => "png-xobjects",
            Self::TaggedPdf => "tagged-pdf",
            Self::TextExtraction => "text-extraction",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineSourceClosure {
    EntryOnly,
}

impl MachineSourceClosure {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EntryOnly => "entry_only",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceCountBounds {
    minimum: u32,
    maximum: u32,
}

impl SourceCountBounds {
    pub const fn minimum(self) -> u32 {
        self.minimum
    }

    pub const fn maximum(self) -> u32 {
        self.maximum
    }

    pub(crate) const fn permits(self, count: usize) -> bool {
        count >= self.minimum as usize && count <= self.maximum as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FootnoteCapability {
    definitions: bool,
    references: bool,
}

impl FootnoteCapability {
    pub const fn definitions(self) -> bool {
        self.definitions
    }

    pub const fn references(self) -> bool {
        self.references
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachinePageMasterCapability {
    count: u32,
    optional_frames: &'static [MachinePageFrame],
    rejected_optional_frames: &'static [MachinePageFrame],
    selection_rules: bool,
}

impl MachinePageMasterCapability {
    pub const fn count(self) -> u32 {
        self.count
    }

    pub const fn optional_frames(self) -> &'static [MachinePageFrame] {
        self.optional_frames
    }

    pub const fn rejected_optional_frames(self) -> &'static [MachinePageFrame] {
        self.rejected_optional_frames
    }

    pub const fn selection_rules(self) -> bool {
        self.selection_rules
    }
}

const ACCEPTED_BLOCKS: &[MachineBlockKind] =
    &[MachineBlockKind::Heading, MachineBlockKind::Paragraph];
const REJECTED_BLOCKS: &[MachineBlockKind] = &[
    MachineBlockKind::Figure,
    MachineBlockKind::List,
    MachineBlockKind::PageBreak,
    MachineBlockKind::Table,
];
const ACCEPTED_INLINES: &[MachineInlineKind] = &[
    MachineInlineKind::Anchor,
    MachineInlineKind::HardBreak,
    MachineInlineKind::Reference,
    MachineInlineKind::SoftBreak,
    MachineInlineKind::Text,
];
const REJECTED_INLINES: &[MachineInlineKind] = &[
    MachineInlineKind::Emphasis,
    MachineInlineKind::FootnoteReference,
    MachineInlineKind::Link,
    MachineInlineKind::Strong,
];
const ACCEPTED_REFERENCE_FORMATS: &[MachineReferenceFormat] = &[MachineReferenceFormat::Page];
const REJECTED_REFERENCE_FORMATS: &[MachineReferenceFormat] =
    &[MachineReferenceFormat::Number, MachineReferenceFormat::Text];
const STYLE_BLOCK_TYPES: &[MachineBlockKind] =
    &[MachineBlockKind::Heading, MachineBlockKind::Paragraph];
const ACCEPTED_STYLE_SELECTORS: &[MachineBlockKind] = STYLE_BLOCK_TYPES;
const REJECTED_STYLE_SELECTORS: &[MachineBlockKind] = &[
    MachineBlockKind::Figure,
    MachineBlockKind::List,
    MachineBlockKind::PageBreak,
    MachineBlockKind::Table,
];
const ACCEPTED_STYLE_PROPERTIES: &[MachineStyleProperty] = &[
    MachineStyleProperty::FontFamily,
    MachineStyleProperty::FontSize,
    MachineStyleProperty::LineHeight,
    MachineStyleProperty::Page,
];
const REJECTED_STYLE_PROPERTIES: &[MachineStyleProperty] = &[];
const ACCEPTED_PAGE_VALUES: &[MachinePageValue] = &[MachinePageValue::Auto];
const REJECTED_PAGE_VALUES: &[MachinePageValue] = &[MachinePageValue::Named];
const ACCEPTED_FONT_FORMATS: &[MachineFontFormat] = &[
    MachineFontFormat::SfntTrueTypeGlyf,
    MachineFontFormat::TtcTrueTypeGlyf,
];
const REJECTED_FONT_FORMATS: &[MachineFontFormat] = &[
    MachineFontFormat::OpenTypeCff,
    MachineFontFormat::TtcOpenTypeCff,
    MachineFontFormat::Woff2,
];
const ACCEPTED_IMAGE_FORMATS: &[MachineImageFormat] = &[];
const REJECTED_IMAGE_FORMATS: &[MachineImageFormat] = &[
    MachineImageFormat::Jpeg,
    MachineImageFormat::Png,
    MachineImageFormat::Svg,
    MachineImageFormat::Vector,
];
const PDF_FEATURES: &[MachinePdfFeature] = &[
    MachinePdfFeature::NamedDestinations,
    MachinePdfFeature::TextExtraction,
];
const UNSUPPORTED_PDF_FEATURES: &[MachinePdfFeature] = &[
    MachinePdfFeature::HeadingSemantics,
    MachinePdfFeature::LinkAnnotations,
    MachinePdfFeature::Outlines,
    MachinePdfFeature::TaggedPdf,
];
const REJECTED_OPTIONAL_FRAMES: &[MachinePageFrame] = &[
    MachinePageFrame::Footer,
    MachinePageFrame::Footnote,
    MachinePageFrame::Header,
];

const BASIC_ACCEPTED_BLOCKS: &[MachineBlockKind] = &[
    MachineBlockKind::Figure,
    MachineBlockKind::Heading,
    MachineBlockKind::List,
    MachineBlockKind::PageBreak,
    MachineBlockKind::Paragraph,
];
const BASIC_REJECTED_BLOCKS: &[MachineBlockKind] = &[MachineBlockKind::Table];
const BASIC_ACCEPTED_INLINES: &[MachineInlineKind] = &[
    MachineInlineKind::Anchor,
    MachineInlineKind::HardBreak,
    MachineInlineKind::Link,
    MachineInlineKind::Reference,
    MachineInlineKind::SoftBreak,
    MachineInlineKind::Text,
];
const BASIC_REJECTED_INLINES: &[MachineInlineKind] = &[
    MachineInlineKind::Emphasis,
    MachineInlineKind::FootnoteReference,
    MachineInlineKind::Strong,
];
const BASIC_STYLE_BLOCK_TYPES: &[MachineBlockKind] = &[
    MachineBlockKind::Figure,
    MachineBlockKind::Heading,
    MachineBlockKind::List,
    MachineBlockKind::PageBreak,
    MachineBlockKind::Paragraph,
];
const BASIC_ACCEPTED_STYLE_PROPERTIES: &[MachineStyleProperty] = &[
    MachineStyleProperty::EndIndent,
    MachineStyleProperty::FontFamily,
    MachineStyleProperty::FontSize,
    MachineStyleProperty::KeepCaption,
    MachineStyleProperty::KeepWithNext,
    MachineStyleProperty::LineHeight,
    MachineStyleProperty::Page,
    MachineStyleProperty::SpaceAfter,
    MachineStyleProperty::SpaceBefore,
    MachineStyleProperty::StartIndent,
    MachineStyleProperty::TextAlign,
    MachineStyleProperty::Width,
];
const BASIC_ACCEPTED_IMAGE_FORMATS: &[MachineImageFormat] = &[MachineImageFormat::Png];
const BASIC_REJECTED_IMAGE_FORMATS: &[MachineImageFormat] = &[
    MachineImageFormat::Jpeg,
    MachineImageFormat::Svg,
    MachineImageFormat::Vector,
];
const BASIC_PDF_FEATURES: &[MachinePdfFeature] = &[
    MachinePdfFeature::LinkAnnotations,
    MachinePdfFeature::NamedDestinations,
    MachinePdfFeature::PngXObjects,
    MachinePdfFeature::TextExtraction,
];
const BASIC_UNSUPPORTED_PDF_FEATURES: &[MachinePdfFeature] = &[
    MachinePdfFeature::HeadingSemantics,
    MachinePdfFeature::Outlines,
    MachinePdfFeature::TaggedPdf,
];

const TABLE_ACCEPTED_BLOCKS: &[MachineBlockKind] = &[
    MachineBlockKind::Figure,
    MachineBlockKind::Heading,
    MachineBlockKind::List,
    MachineBlockKind::PageBreak,
    MachineBlockKind::Paragraph,
    MachineBlockKind::Table,
];
const TABLE_STYLE_BLOCK_TYPES: &[MachineBlockKind] = TABLE_ACCEPTED_BLOCKS;

const FOOTNOTE_ACCEPTED_INLINES: &[MachineInlineKind] = &[
    MachineInlineKind::Anchor,
    MachineInlineKind::FootnoteReference,
    MachineInlineKind::HardBreak,
    MachineInlineKind::Link,
    MachineInlineKind::Reference,
    MachineInlineKind::SoftBreak,
    MachineInlineKind::Text,
];
const FOOTNOTE_REJECTED_INLINES: &[MachineInlineKind] =
    &[MachineInlineKind::Emphasis, MachineInlineKind::Strong];
const FOOTNOTE_OPTIONAL_FRAMES: &[MachinePageFrame] = &[MachinePageFrame::Footnote];
const FOOTNOTE_REJECTED_OPTIONAL_FRAMES: &[MachinePageFrame] =
    &[MachinePageFrame::Footer, MachinePageFrame::Header];

const PRODUCTION_ACCEPTED_BLOCKS: &[MachineBlockKind] = &[
    MachineBlockKind::DisplayMath,
    MachineBlockKind::Figure,
    MachineBlockKind::Heading,
    MachineBlockKind::List,
    MachineBlockKind::MathVectorBlock,
    MachineBlockKind::PageBreak,
    MachineBlockKind::Paragraph,
    MachineBlockKind::SemanticContainer,
    MachineBlockKind::Table,
    MachineBlockKind::VectorFigure,
];
const PRODUCTION_ACCEPTED_INLINES: &[MachineInlineKind] = &[
    MachineInlineKind::Anchor,
    MachineInlineKind::Emphasis,
    MachineInlineKind::FootnoteReference,
    MachineInlineKind::HardBreak,
    MachineInlineKind::InlineMath,
    MachineInlineKind::InlineVector,
    MachineInlineKind::Link,
    MachineInlineKind::MathVector,
    MachineInlineKind::Reference,
    MachineInlineKind::SoftBreak,
    MachineInlineKind::Strong,
    MachineInlineKind::Text,
];
const PRODUCTION_ACCEPTED_FONT_FORMATS: &[MachineFontFormat] = &[
    MachineFontFormat::OpenTypeCff,
    MachineFontFormat::SfntTrueTypeGlyf,
    MachineFontFormat::TtcTrueTypeGlyf,
];
const PRODUCTION_ACCEPTED_IMAGE_FORMATS: &[MachineImageFormat] = &[
    MachineImageFormat::Jpeg,
    MachineImageFormat::Png,
    MachineImageFormat::Svg,
];
const PRODUCTION_PDF_FEATURES: &[MachinePdfFeature] = &[
    MachinePdfFeature::HeadingSemantics,
    MachinePdfFeature::LinkAnnotations,
    MachinePdfFeature::NamedDestinations,
    MachinePdfFeature::Outlines,
    MachinePdfFeature::PngXObjects,
    MachinePdfFeature::TaggedPdf,
    MachinePdfFeature::TextExtraction,
];

/// Immutable, closed contract for one machine-PDF profile.
///
/// All fields are private and every slice points at static, canonically ordered
/// data. New acceptance requires a new profile ID rather than mutation of this
/// value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachineProfileDescriptor {
    id: MachinePdfProfileId,
    source_closure: MachineSourceClosure,
    source_count: SourceCountBounds,
    accepted_blocks: &'static [MachineBlockKind],
    rejected_blocks: &'static [MachineBlockKind],
    accepted_inlines: &'static [MachineInlineKind],
    rejected_inlines: &'static [MachineInlineKind],
    accepted_reference_formats: &'static [MachineReferenceFormat],
    rejected_reference_formats: &'static [MachineReferenceFormat],
    footnotes: FootnoteCapability,
    style_block_types: &'static [MachineBlockKind],
    accepted_style_selectors: &'static [MachineBlockKind],
    rejected_style_selectors: &'static [MachineBlockKind],
    accepted_style_properties: &'static [MachineStyleProperty],
    rejected_style_properties: &'static [MachineStyleProperty],
    accepted_page_values: &'static [MachinePageValue],
    rejected_page_values: &'static [MachinePageValue],
    page_master: MachinePageMasterCapability,
    accepted_font_formats: &'static [MachineFontFormat],
    rejected_font_formats: &'static [MachineFontFormat],
    minimum_fonts_for_text: u32,
    accepted_image_formats: &'static [MachineImageFormat],
    rejected_image_formats: &'static [MachineImageFormat],
    pdf_features: &'static [MachinePdfFeature],
    unsupported_pdf_features: &'static [MachinePdfFeature],
}

impl MachineProfileDescriptor {
    pub const PARAGRAPH_1: Self = Self {
        id: MachinePdfProfileId::PARAGRAPH_1,
        source_closure: MachineSourceClosure::EntryOnly,
        source_count: SourceCountBounds {
            minimum: 1,
            maximum: 1,
        },
        accepted_blocks: ACCEPTED_BLOCKS,
        rejected_blocks: REJECTED_BLOCKS,
        accepted_inlines: ACCEPTED_INLINES,
        rejected_inlines: REJECTED_INLINES,
        accepted_reference_formats: ACCEPTED_REFERENCE_FORMATS,
        rejected_reference_formats: REJECTED_REFERENCE_FORMATS,
        footnotes: FootnoteCapability {
            definitions: false,
            references: false,
        },
        style_block_types: STYLE_BLOCK_TYPES,
        accepted_style_selectors: ACCEPTED_STYLE_SELECTORS,
        rejected_style_selectors: REJECTED_STYLE_SELECTORS,
        accepted_style_properties: ACCEPTED_STYLE_PROPERTIES,
        rejected_style_properties: REJECTED_STYLE_PROPERTIES,
        accepted_page_values: ACCEPTED_PAGE_VALUES,
        rejected_page_values: REJECTED_PAGE_VALUES,
        page_master: MachinePageMasterCapability {
            count: 1,
            optional_frames: &[],
            rejected_optional_frames: REJECTED_OPTIONAL_FRAMES,
            selection_rules: false,
        },
        accepted_font_formats: ACCEPTED_FONT_FORMATS,
        rejected_font_formats: REJECTED_FONT_FORMATS,
        minimum_fonts_for_text: 1,
        accepted_image_formats: ACCEPTED_IMAGE_FORMATS,
        rejected_image_formats: REJECTED_IMAGE_FORMATS,
        pdf_features: PDF_FEATURES,
        unsupported_pdf_features: UNSUPPORTED_PDF_FEATURES,
    };

    /// Immutable, closed M2 profile adopted by ADR-0028.
    pub const BASIC_DOCUMENT_1: Self = Self {
        id: MachinePdfProfileId::BASIC_DOCUMENT_1,
        source_closure: MachineSourceClosure::EntryOnly,
        source_count: SourceCountBounds {
            minimum: 1,
            maximum: 1,
        },
        accepted_blocks: BASIC_ACCEPTED_BLOCKS,
        rejected_blocks: BASIC_REJECTED_BLOCKS,
        accepted_inlines: BASIC_ACCEPTED_INLINES,
        rejected_inlines: BASIC_REJECTED_INLINES,
        accepted_reference_formats: ACCEPTED_REFERENCE_FORMATS,
        rejected_reference_formats: REJECTED_REFERENCE_FORMATS,
        footnotes: FootnoteCapability {
            definitions: false,
            references: false,
        },
        style_block_types: BASIC_STYLE_BLOCK_TYPES,
        accepted_style_selectors: BASIC_STYLE_BLOCK_TYPES,
        rejected_style_selectors: &[MachineBlockKind::Table],
        accepted_style_properties: BASIC_ACCEPTED_STYLE_PROPERTIES,
        rejected_style_properties: &[],
        accepted_page_values: ACCEPTED_PAGE_VALUES,
        rejected_page_values: REJECTED_PAGE_VALUES,
        page_master: MachinePageMasterCapability {
            count: 1,
            optional_frames: &[],
            rejected_optional_frames: REJECTED_OPTIONAL_FRAMES,
            selection_rules: false,
        },
        accepted_font_formats: ACCEPTED_FONT_FORMATS,
        rejected_font_formats: REJECTED_FONT_FORMATS,
        minimum_fonts_for_text: 1,
        accepted_image_formats: BASIC_ACCEPTED_IMAGE_FORMATS,
        rejected_image_formats: BASIC_REJECTED_IMAGE_FORMATS,
        pdf_features: BASIC_PDF_FEATURES,
        unsupported_pdf_features: BASIC_UNSUPPORTED_PDF_FEATURES,
    };

    /// Contract-1.3 sequential-column profile adopted by ADR-0031.
    pub const COLUMNS_1: Self = Self {
        id: MachinePdfProfileId::COLUMNS_1,
        ..Self::BASIC_DOCUMENT_1
    };

    /// Contract-1.3 FIFO float profile adopted by ADR-0031.
    pub const FLOAT_1: Self = Self {
        id: MachinePdfProfileId::FLOAT_1,
        ..Self::BASIC_DOCUMENT_1
    };

    /// Contract-1.3 page-region profile adopted by ADR-0031.
    pub const HEADER_FOOTER_1: Self = Self {
        id: MachinePdfProfileId::HEADER_FOOTER_1,
        ..Self::BASIC_DOCUMENT_1
    };

    /// Immutable, closed M3 footnote profile adopted by ADR-0030. It contains
    /// the complete basic-document domain and adds only body references plus
    /// document-owned paragraph/heading definitions.
    pub const FOOTNOTE_1: Self = Self {
        id: MachinePdfProfileId::FOOTNOTE_1,
        source_closure: MachineSourceClosure::EntryOnly,
        source_count: SourceCountBounds {
            minimum: 1,
            maximum: 1,
        },
        accepted_blocks: BASIC_ACCEPTED_BLOCKS,
        rejected_blocks: BASIC_REJECTED_BLOCKS,
        accepted_inlines: FOOTNOTE_ACCEPTED_INLINES,
        rejected_inlines: FOOTNOTE_REJECTED_INLINES,
        accepted_reference_formats: ACCEPTED_REFERENCE_FORMATS,
        rejected_reference_formats: REJECTED_REFERENCE_FORMATS,
        footnotes: FootnoteCapability {
            definitions: true,
            references: true,
        },
        style_block_types: BASIC_STYLE_BLOCK_TYPES,
        accepted_style_selectors: BASIC_STYLE_BLOCK_TYPES,
        rejected_style_selectors: &[MachineBlockKind::Table],
        accepted_style_properties: BASIC_ACCEPTED_STYLE_PROPERTIES,
        rejected_style_properties: &[],
        accepted_page_values: ACCEPTED_PAGE_VALUES,
        rejected_page_values: REJECTED_PAGE_VALUES,
        page_master: MachinePageMasterCapability {
            count: 1,
            optional_frames: FOOTNOTE_OPTIONAL_FRAMES,
            rejected_optional_frames: FOOTNOTE_REJECTED_OPTIONAL_FRAMES,
            selection_rules: false,
        },
        accepted_font_formats: ACCEPTED_FONT_FORMATS,
        rejected_font_formats: REJECTED_FONT_FORMATS,
        minimum_fonts_for_text: 1,
        accepted_image_formats: BASIC_ACCEPTED_IMAGE_FORMATS,
        rejected_image_formats: BASIC_REJECTED_IMAGE_FORMATS,
        pdf_features: BASIC_PDF_FEATURES,
        unsupported_pdf_features: BASIC_UNSUPPORTED_PDF_FEATURES,
    };

    /// Immutable, closed M3 table profile adopted by ADR-0029. It contains
    /// the complete basic-document domain and adds only direct-body tables.
    pub const TABLE_1: Self = Self {
        id: MachinePdfProfileId::TABLE_1,
        source_closure: MachineSourceClosure::EntryOnly,
        source_count: SourceCountBounds {
            minimum: 1,
            maximum: 1,
        },
        accepted_blocks: TABLE_ACCEPTED_BLOCKS,
        rejected_blocks: &[],
        accepted_inlines: BASIC_ACCEPTED_INLINES,
        rejected_inlines: BASIC_REJECTED_INLINES,
        accepted_reference_formats: ACCEPTED_REFERENCE_FORMATS,
        rejected_reference_formats: REJECTED_REFERENCE_FORMATS,
        footnotes: FootnoteCapability {
            definitions: false,
            references: false,
        },
        style_block_types: TABLE_STYLE_BLOCK_TYPES,
        accepted_style_selectors: TABLE_STYLE_BLOCK_TYPES,
        rejected_style_selectors: &[],
        accepted_style_properties: BASIC_ACCEPTED_STYLE_PROPERTIES,
        rejected_style_properties: &[],
        accepted_page_values: ACCEPTED_PAGE_VALUES,
        rejected_page_values: REJECTED_PAGE_VALUES,
        page_master: MachinePageMasterCapability {
            count: 1,
            optional_frames: &[],
            rejected_optional_frames: REJECTED_OPTIONAL_FRAMES,
            selection_rules: false,
        },
        accepted_font_formats: ACCEPTED_FONT_FORMATS,
        rejected_font_formats: REJECTED_FONT_FORMATS,
        minimum_fonts_for_text: 1,
        accepted_image_formats: BASIC_ACCEPTED_IMAGE_FORMATS,
        rejected_image_formats: BASIC_REJECTED_IMAGE_FORMATS,
        pdf_features: BASIC_PDF_FEATURES,
        unsupported_pdf_features: BASIC_UNSUPPORTED_PDF_FEATURES,
    };

    /// Complete contract-1.4 production profile. The versioned resource-set
    /// and SafeVector details are emitted from the companion nominal
    /// descriptor; all common profile fields remain owned by this value.
    pub const PRODUCTION_BOOK_1: Self = Self {
        id: MachinePdfProfileId::PRODUCTION_BOOK_1,
        source_closure: MachineSourceClosure::EntryOnly,
        source_count: SourceCountBounds {
            minimum: 1,
            maximum: 1,
        },
        accepted_blocks: PRODUCTION_ACCEPTED_BLOCKS,
        rejected_blocks: &[],
        accepted_inlines: PRODUCTION_ACCEPTED_INLINES,
        rejected_inlines: &[],
        accepted_reference_formats: ACCEPTED_REFERENCE_FORMATS,
        rejected_reference_formats: REJECTED_REFERENCE_FORMATS,
        footnotes: FootnoteCapability {
            definitions: true,
            references: true,
        },
        style_block_types: PRODUCTION_ACCEPTED_BLOCKS,
        accepted_style_selectors: PRODUCTION_ACCEPTED_BLOCKS,
        rejected_style_selectors: &[],
        accepted_style_properties: BASIC_ACCEPTED_STYLE_PROPERTIES,
        rejected_style_properties: &[],
        accepted_page_values: ACCEPTED_PAGE_VALUES,
        rejected_page_values: REJECTED_PAGE_VALUES,
        page_master: MachinePageMasterCapability {
            count: 1,
            optional_frames: FOOTNOTE_OPTIONAL_FRAMES,
            rejected_optional_frames: FOOTNOTE_REJECTED_OPTIONAL_FRAMES,
            selection_rules: false,
        },
        accepted_font_formats: PRODUCTION_ACCEPTED_FONT_FORMATS,
        rejected_font_formats: &[],
        minimum_fonts_for_text: 1,
        accepted_image_formats: PRODUCTION_ACCEPTED_IMAGE_FORMATS,
        rejected_image_formats: &[],
        pdf_features: PRODUCTION_PDF_FEATURES,
        unsupported_pdf_features: &[],
    };

    pub const fn for_id(id: MachinePdfProfileId) -> Self {
        match id {
            MachinePdfProfileId::BasicDocument1 => Self::BASIC_DOCUMENT_1,
            MachinePdfProfileId::Columns1 => Self::COLUMNS_1,
            MachinePdfProfileId::Float1 => Self::FLOAT_1,
            MachinePdfProfileId::Footnote1 => Self::FOOTNOTE_1,
            MachinePdfProfileId::HeaderFooter1 => Self::HEADER_FOOTER_1,
            MachinePdfProfileId::Paragraph1 => Self::PARAGRAPH_1,
            MachinePdfProfileId::ProductionBook1 => Self::PRODUCTION_BOOK_1,
            MachinePdfProfileId::Table1 => Self::TABLE_1,
        }
    }

    pub const fn id(self) -> MachinePdfProfileId {
        self.id
    }

    pub const fn source_closure(self) -> MachineSourceClosure {
        self.source_closure
    }

    pub const fn source_count(self) -> SourceCountBounds {
        self.source_count
    }

    pub const fn accepted_blocks(self) -> &'static [MachineBlockKind] {
        self.accepted_blocks
    }

    pub const fn rejected_blocks(self) -> &'static [MachineBlockKind] {
        self.rejected_blocks
    }

    pub const fn accepted_inlines(self) -> &'static [MachineInlineKind] {
        self.accepted_inlines
    }

    pub const fn rejected_inlines(self) -> &'static [MachineInlineKind] {
        self.rejected_inlines
    }

    pub const fn accepted_reference_formats(self) -> &'static [MachineReferenceFormat] {
        self.accepted_reference_formats
    }

    pub const fn rejected_reference_formats(self) -> &'static [MachineReferenceFormat] {
        self.rejected_reference_formats
    }

    pub const fn footnotes(self) -> FootnoteCapability {
        self.footnotes
    }

    pub const fn style_block_types(self) -> &'static [MachineBlockKind] {
        self.style_block_types
    }

    pub const fn accepted_style_selectors(self) -> &'static [MachineBlockKind] {
        self.accepted_style_selectors
    }

    pub const fn rejected_style_selectors(self) -> &'static [MachineBlockKind] {
        self.rejected_style_selectors
    }

    pub const fn accepted_style_properties(self) -> &'static [MachineStyleProperty] {
        self.accepted_style_properties
    }

    pub const fn rejected_style_properties(self) -> &'static [MachineStyleProperty] {
        self.rejected_style_properties
    }

    pub const fn accepted_page_values(self) -> &'static [MachinePageValue] {
        self.accepted_page_values
    }

    pub const fn rejected_page_values(self) -> &'static [MachinePageValue] {
        self.rejected_page_values
    }

    pub const fn page_master(self) -> MachinePageMasterCapability {
        self.page_master
    }

    pub const fn accepted_font_formats(self) -> &'static [MachineFontFormat] {
        self.accepted_font_formats
    }

    pub const fn rejected_font_formats(self) -> &'static [MachineFontFormat] {
        self.rejected_font_formats
    }

    /// Minimum declared font faces when the document contains a text-producing
    /// site. A document without such a site may declare no fonts.
    pub const fn minimum_fonts_for_text(self) -> u32 {
        self.minimum_fonts_for_text
    }

    pub const fn accepted_image_formats(self) -> &'static [MachineImageFormat] {
        self.accepted_image_formats
    }

    pub const fn rejected_image_formats(self) -> &'static [MachineImageFormat] {
        self.rejected_image_formats
    }

    pub const fn pdf_features(self) -> &'static [MachinePdfFeature] {
        self.pdf_features
    }

    pub const fn unsupported_pdf_features(self) -> &'static [MachinePdfFeature] {
        self.unsupported_pdf_features
    }

    pub(crate) fn accepts_block(self, kind: MachineBlockKind) -> bool {
        self.accepted_blocks.contains(&kind)
    }

    pub(crate) fn accepts_inline(self, kind: MachineInlineKind) -> bool {
        self.accepted_inlines.contains(&kind)
    }

    pub(crate) fn accepts_reference_format(self, format: MachineReferenceFormat) -> bool {
        self.accepted_reference_formats.contains(&format)
    }

    pub(crate) fn accepts_style_selector(self, selector: &str) -> bool {
        let extended = matches!(
            self.id,
            MachinePdfProfileId::BasicDocument1
                | MachinePdfProfileId::Columns1
                | MachinePdfProfileId::Float1
                | MachinePdfProfileId::Footnote1
                | MachinePdfProfileId::HeaderFooter1
                | MachinePdfProfileId::ProductionBook1
                | MachinePdfProfileId::Table1
        );
        let block = if extended {
            selector.split('.').next().unwrap_or_default()
        } else {
            selector
        };
        (extended || !selector.contains('.'))
            && self
                .accepted_style_selectors
                .iter()
                .any(|kind| kind.as_str() == block)
    }

    pub(crate) fn accepts_style_property(self, name: &str) -> bool {
        MachineStyleProperty::from_str(name)
            .is_some_and(|property| self.accepted_style_properties.contains(&property))
    }

    pub(crate) fn accepts_page_value(self, value: MachinePageValue) -> bool {
        self.accepted_page_values.contains(&value)
    }
}
