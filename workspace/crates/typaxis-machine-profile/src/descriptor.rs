use typaxis_core::MachinePdfProfileId;

/// Block kinds understood by the current document domain.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachineBlockKind {
    Figure,
    Heading,
    List,
    PageBreak,
    Paragraph,
    Table,
}

impl MachineBlockKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Figure => "figure",
            Self::Heading => "heading",
            Self::List => "list",
            Self::PageBreak => "page_break",
            Self::Paragraph => "paragraph",
            Self::Table => "table",
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
    Link,
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
            Self::Link => "link",
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
    FontFamily,
    FontSize,
    LineHeight,
    Page,
}

impl MachineStyleProperty {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FontFamily => "font_family",
            Self::FontSize => "font_size",
            Self::LineHeight => "line_height",
            Self::Page => "page",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "font_family" => Some(Self::FontFamily),
            "font_size" => Some(Self::FontSize),
            "line_height" => Some(Self::LineHeight),
            "page" => Some(Self::Page),
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MachinePdfFeature {
    HeadingSemantics,
    LinkAnnotations,
    NamedDestinations,
    Outlines,
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
        !selector.contains('.')
            && self
                .accepted_style_selectors
                .iter()
                .any(|kind| kind.as_str() == selector)
    }

    pub(crate) fn accepts_style_property(self, name: &str) -> bool {
        MachineStyleProperty::from_str(name)
            .is_some_and(|property| self.accepted_style_properties.contains(&property))
    }

    pub(crate) fn accepts_page_value(self, value: MachinePageValue) -> bool {
        self.accepted_page_values.contains(&value)
    }
}
