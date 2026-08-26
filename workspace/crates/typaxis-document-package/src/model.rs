use typaxis_core::DocumentPackageContractId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireCoordinateUnit {
    PdfPoint1_65536,
}

impl WireCoordinateUnit {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PdfPoint1_65536 => "pdf_point_1_65536",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireDocumentPackage {
    pub contract: DocumentPackageContractId,
    pub coordinate_unit: WireCoordinateUnit,
    pub sources: Vec<WireSource>,
    pub text_buffers: Vec<WireTextBuffer>,
    pub document: WireDocument,
    pub style_sheet: WireStyleSheet,
    pub page_masters: WirePageMasterSet,
    pub resources: WireResourceCatalog,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireSource {
    pub source_id: u32,
    pub uri: String,
    pub utf8_byte_length: u32,
    pub sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireByteRange {
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireSourceSpan {
    pub source_id: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireTextSpan {
    pub text_id: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireTextMapKind {
    Identity,
    Replacement,
    Inserted,
}

impl WireTextMapKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Replacement => "replacement",
            Self::Inserted => "inserted",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireTextMapSegment {
    pub text_range: WireByteRange,
    pub kind: WireTextMapKind,
    pub source_span: Option<WireSourceSpan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireTextBuffer {
    pub text_id: u32,
    pub utf8: String,
    pub mappings: Vec<WireTextMapSegment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireLinkTarget {
    Internal { anchor_id: String },
    Uri { uri: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireReferenceFormat {
    Text,
    Page,
    Number,
}

impl WireReferenceFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Page => "page",
            Self::Number => "number",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireInline {
    Text {
        node_id: u32,
        span: WireSourceSpan,
        text_span: WireTextSpan,
    },
    Emphasis {
        node_id: u32,
        span: WireSourceSpan,
        children: Vec<WireInline>,
    },
    Strong {
        node_id: u32,
        span: WireSourceSpan,
        children: Vec<WireInline>,
    },
    Link {
        node_id: u32,
        span: WireSourceSpan,
        target: WireLinkTarget,
        children: Vec<WireInline>,
    },
    Anchor {
        node_id: u32,
        span: WireSourceSpan,
        anchor_id: String,
    },
    Reference {
        node_id: u32,
        span: WireSourceSpan,
        target: String,
        format: WireReferenceFormat,
    },
    FootnoteReference {
        node_id: u32,
        span: WireSourceSpan,
        footnote_id: String,
    },
    SoftBreak {
        node_id: u32,
        span: WireSourceSpan,
    },
    HardBreak {
        node_id: u32,
        span: WireSourceSpan,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireListItem {
    pub node_id: u32,
    pub span: WireSourceSpan,
    pub blocks: Vec<WireBlock>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireTableColumn {
    Fixed { width: i64 },
    Fraction { weight: u16 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireTableCell {
    pub node_id: u32,
    pub span: WireSourceSpan,
    pub colspan: u16,
    pub rowspan: u16,
    pub blocks: Vec<WireBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireTableRow {
    pub node_id: u32,
    pub span: WireSourceSpan,
    pub cells: Vec<WireTableCell>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireBlock {
    Paragraph {
        node_id: u32,
        span: WireSourceSpan,
        classes: Vec<String>,
        children: Vec<WireInline>,
    },
    Heading {
        node_id: u32,
        span: WireSourceSpan,
        classes: Vec<String>,
        level: u8,
        anchor_id: Option<String>,
        children: Vec<WireInline>,
    },
    List {
        node_id: u32,
        span: WireSourceSpan,
        classes: Vec<String>,
        ordered: bool,
        start: Option<u32>,
        items: Vec<WireListItem>,
    },
    Table {
        node_id: u32,
        span: WireSourceSpan,
        classes: Vec<String>,
        columns: Vec<WireTableColumn>,
        head: Vec<WireTableRow>,
        body: Vec<WireTableRow>,
    },
    Figure {
        node_id: u32,
        span: WireSourceSpan,
        classes: Vec<String>,
        image_id: u32,
        alt: String,
        caption: Vec<WireBlock>,
    },
    PageBreak {
        node_id: u32,
        span: WireSourceSpan,
        classes: Vec<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireFootnote {
    pub footnote_id: String,
    pub node_id: u32,
    pub span: WireSourceSpan,
    pub blocks: Vec<WireBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireDocument {
    pub node_id: u32,
    pub blocks: Vec<WireBlock>,
    pub footnotes: Vec<WireFootnote>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireDeclarationName {
    FontFamily,
    FontSize,
    LineHeight,
    Page,
}

impl WireDeclarationName {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FontFamily => "font_family",
            Self::FontSize => "font_size",
            Self::LineHeight => "line_height",
            Self::Page => "page",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireStyleValue {
    Keyword { value: String },
    String { value: std::string::String },
    Integer { value: i64 },
    Length { value: i64 },
    Boolean { value: bool },
    FontFamilyList { families: Vec<String> },
    Ratio { numerator: i64, denominator: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireDeclaration {
    pub name: WireDeclarationName,
    pub value: WireStyleValue,
    pub important: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireStyleRule {
    pub style_id: String,
    pub extends: Option<String>,
    pub selector: String,
    pub source_order: u32,
    pub declarations: Vec<WireDeclaration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireStyleSheet {
    pub rules: Vec<WireStyleRule>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireRect {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WirePageMaster {
    pub master_id: String,
    pub width: i64,
    pub height: i64,
    pub body: WireRect,
    pub header: Option<WireRect>,
    pub footer: Option<WireRect>,
    pub footnote: Option<WireRect>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WirePageParity {
    Any,
    Odd,
    Even,
}

impl WirePageParity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Odd => "odd",
            Self::Even => "even",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WirePageMasterRule {
    pub master_id: String,
    pub parity: WirePageParity,
    pub first: Option<bool>,
    pub named_page: Option<String>,
    pub source_order: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WirePageMasterSet {
    pub default_master_id: String,
    pub masters: Vec<WirePageMaster>,
    pub selection_rules: Vec<WirePageMasterRule>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireFontFace {
    pub font_face_id: u32,
    pub family: String,
    pub uri: String,
    pub face_index: u32,
    pub expected_sha256: Option<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireImage {
    pub image_id: u32,
    pub uri: String,
    pub expected_sha256: Option<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireResourceCatalog {
    pub font_faces: Vec<WireFontFace>,
    pub images: Vec<WireImage>,
}
